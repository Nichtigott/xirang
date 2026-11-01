# 片区审计 A：内核 `core/`（lane-kernel）

- 审计对象：工作树 HEAD=cf0c378（120 项未提交改动），`core/` 全部 93 文件 21787 行（`core/src/test/registry_rule/` 是空目录，无文件）。
- 口径：本轮只出报告，源码一行不改。所有 `file:line` 均为本次亲自 read 到的当前内容；附带的复核命令/探针只写命令，不落盘。
- 严重度：CRITICAL＝静默错答、数据/身份损坏、安全或不可逆；MAJOR＝明确行为错误或维护性硬伤；MINOR＝局部可读性/一致性。
- 类别：A 命名、B 责任与碎片化、C 阅读顺序、D 大逻辑、E 函数内谬误、F 铁律（core 不得 I/O / 读 `std::env` / 依赖进程生命周期）。
- 格式：id / 严重度 / 类别 / file:line / 现象 / 判据 / 最小修复方向 / 复核手段。

## 发现

### K-01 CRITICAL — D 大逻辑（作者侧写回静默丢弃 deny 列表，安全门禁被放宽）

- 位置：`core/src/registry_core/authoring/parse/admission.rs:40-44`（读入口 `run_method/authoring/manifest/parse/parse.rs:134`，写回 `run_method/authoring/manifest/face/render.rs:79`）
- 现象：`parse_admission_expression` 读构造形式时，若 allow 与 deny **同时**非空，只返回 allow：

  ```text
  return match (allow.is_empty(), deny.is_empty()) {
      (true, true) => Ok("ANY".to_owned()),
      (false, _) => Ok(format!("allow:{}", allow.join(","))),   // deny 被丢弃
      (true, false) => Ok(format!("deny:{}", deny.join(","))),
  };
  ```

  该函数正是创作面读注册面 `admission` 字段的入口（`manifest/parse/parse.rs:134`），写回走 `render_admission`（`face/render.rs:79`）。于是 `Admission::new(&["ui"], &["ui/experimental"])` 读成 `allow:ui`，写回成 `crate::Admission::new(&["ui"], &[])`——**deny 列表在“读一次再存一次”后消失**。
- 判据：`Admission::accepts` 的实现 `pub fn accepts`（`core/src/registry_core/declaration/registration.rs:64-77`）规定 deny 优先，deny 列表是这道门禁的否决权；两列表同时使用是被支持、且被本仓自带数据采用的写法（`declaration/declaration.rs:344`：`Admission::new(&["ui", "ui/controls"], &["ui/experimental"])`）。丢弃 deny 不是格式化，而是把门禁放宽，且在源码被改写后不可逆。文件头注释自称“Both spellings describe the same gate”（`core/src/registry_core/authoring/parse/admission.rs:5-8`），与该行为直接矛盾（另见 C-02）。
- 最小修复方向：紧凑形式必须能承载两张列表（如 `allow:a,b;deny:c`），`parse_admission_expression` 与 `parse_admission_owned` 双向支持；若坚持不扩展语法，则**两列表非空时返回 Err**（不允许静默丢字段），`render_admission` 同步拒绝。
- 复核手段：单测探针 `parse_admission_expression(r#"crate::Admission::new(&["ui"], &["ui/experimental"])"#)` 期望 `Err`，当前返回 `Ok("allow:ui")`；端到端：取一个 `admission: crate::Admission::new(&["ui"], &["ui/experimental"])` 的注册面，经 Studio/MCP 任一字段编辑后写回，检查文件里 deny 是否还在。

### K-02 MAJOR — D 大逻辑（热重载静默忽略作者改过的 `runtime_checks`）

- 位置：`core/src/registry_core/declaration/owned.rs:280-315`（`RegistrationSnapshot::merge_authored`）
- 现象：`merge_authored(mut self, authored: Self)` 逐字段应用作者侧值，唯独不碰 `runtime_checks`、`contract`、`plugin`。而作者侧快照**确实**解析了 `runtime_checks`（`core/src/registry_core/authoring/snapshot/snapshot.rs:127`；run_method 的字段读入见 `run_method/authoring/manifest/parse/parse.rs:179-180`）。唯一调用点是 `run_method/authoring/operations/operations.rs:352`（`existing.clone().merge_authored(authored)`，即“编译期快照 + 文件快照”）。
- 判据：`flow` 给出同文件内的对照：作者侧已声明 flow 时会 `self.flow = authored.flow`（`owned.rs:305-310`），并有注释解释为何 part 列表必须保留编译期证据。`runtime_checks` 与 `flow` 一样已被解析成纯数据（`RuntimeCheckSpec` 枚举），并非只能来自编译期常量，却被整条丢弃且无注释说明这是有意的。结果是：作者改 `runtime_checks` 后热重载行为不变——编辑被静默忽略，而文件里写着新值。
- 最小修复方向：对 `runtime_checks` 采用与 `flow` 相同的“作者侧非空即应用”规则（`if !authored.runtime_checks.is_empty() { self.runtime_checks = authored.runtime_checks; }`），或在函数文档里明确“运行期校验以编译期为准，文件编辑只在新编译后生效”，并让编辑入口据此拒绝该字段，避免“看起来能改、其实不改”。
- 复核手段：单测探针：`self.runtime_checks = vec![FINITE_NUMBER]`、`authored.runtime_checks = vec![NON_EMPTY_TEXT]`，`merge_authored` 后断言结果（当前仍是 `[FINITE_NUMBER]`）。

### K-03 MAJOR — D 大逻辑（`validate_*` 说通过、`apply_*` 却失败，且错误指向错误原因）

- 位置：`core/src/registry_core/tree/graft_ops/graft_ops.rs:198-224`（apply）、`:229-273`（`replace_info`）、`:100-210`（validate）
- 现象：`validate_snapshot_replacement` 不检查“把拥有非空子注册机的面改成叶子（`needs_registry: false`）”，于是校验通过；随后 `apply_snapshot_replacement` 里 `replace_info` 走到

  ```text
  (Some(child_registry), false) if !child_registry.entries.is_empty() => {
      entry.child = Some(child_registry.clone());
      return false;
  }
  ```

  返回 `false`，调用方把它一律映射成 `GraftError::UnknownTarget(current)`（`graft_ops.rs:222-224`），报“目标不存在”。实际原因恰恰相反：目标存在，是**不能把非空子注册机降级为叶子**。
- 判据：`validate_snapshot_replacement` 文档承诺“Validate an authored replacement without committing it”，`apply_snapshot_replacement` 注释说“Commit one already-authored replacement after validating it against the same structural and connector rules used by registration”。校验与提交不同判等于校验结果不可信；文件创作路径正是以该校验作为写盘闸门（同页测试 `an_edit_that_invalidates_an_existing_child_is_refused` 的注释：*the file-authoring path gates its write on this very check*）。错误文本错指原因，会让作者去改身份/父级而不是子注册机。
- 最小修复方向：在 `validate_snapshot_replacement` 补上与 `replace_info` 相同的降级检查（`!info.needs_registry && 现存子注册机非空` → 专门错误，文案说明“先移走/清空子注册机”）；`replace_info` 改为返回 `Result`，取消 `UnknownTarget` 兜底映射。
- 复核手段：单测探针：注册 owner（`needs_registry: true`）+ 一个子面，`validate_snapshot_replacement(owner_id, owner_with_needs_registry_false)` 期望 `Err`，当前 `Ok`。

### K-04 MAJOR — D 大逻辑（“不能留下不自洽的子注册机”这条规则在 overlay 里缺失，两份实现分叉）

- 位置：`core/src/registry_core/tree/graft_ops/overlay.rs:258-309` 对比 `graft_ops.rs:261-264`
- 现象：非 full 嫁接分支无条件保留基座子注册机：

  ```text
  let mut kept = entry.child.clone();
  if let Some(registry) = &mut kept { ... registry.reconfigure(&candidate); ... }
  ```

  它只检查“保留的子级是否满足新规则”，**不检查** `candidate.needs_registry`。而就地替换路径对同一状态明确拒绝（`graft_ops.rs:261-264`：`(Some(child_registry), false) if !child_registry.entries.is_empty() => return false`）。结果：一次非 full 嫁接可以让一个 `needs_registry: false` 的面继续拥有非空子注册机，并且 `register_snapshot_batch` 仍能通过 `registry(parent_id)` 找到它并向里注册（`tree/transaction/transaction.rs:47`）。
- 判据：两条路径回答同一个问题（“这个面还能不能拥有它现在拥有的子注册机”）却给出相反答案；`needs_registry` 是声明的权威（注册路径只按它创建子注册机，`transaction.rs:246-256`）。留下“声明为叶子、实际是父级”的节点会让 `port_index`/`dump`/连接器遍历与声明不一致，且该类不一致只能靠重读源码发现。现有测试 `record_tests::full_discards_base_children_and_non_full_keeps_them` 的替换件也是 `needs_registry: true`，因此没有覆盖这条边界。
- 最小修复方向：`apply_overlay_face` 的非 full 分支在 `!candidate.needs_registry` 且 `kept` 非空时返回专门的 `GraftError`；把判据抽成一个共享函数（如 `fn child_registry_can_survive(needs_registry: bool, child: &Option<Arc<Registry>>) -> bool`）供两条路径调用。
- 复核手段：单测探针：基座 owner（`needs_registry: true`）+ 子面；外部替换件 `needs_registry: false`、flow 已声明且兼容；`overlay` 期望 `Err`，当前 `Ok` 且 `effective.registry(owner_id).is_some()`。

### K-05 MAJOR — D 大逻辑（兄弟同名 `registry_name` 无人拒绝，路径这一“事实键”会静默碰撞）

- 位置：`core/src/registry_core/tree/transaction/transaction.rs:98-179`（唯一查重处）、`tree/index/index.rs:14-18,85-110`、`tree/query/query.rs:86-96`、`tree/graft_ops/resolution.rs:22-28`
- 现象：`plan_batch` 只检查三类冲突：`NodeId` 与基树/批内重复（:116-128）、显式 `stable_name` 重复（:129-147）、父链环与缺父（:149-177）。**没有任何地方检查同一父级下 `registry_name` 是否重复**。而 `registry_name` 是可被作者编辑的字段（`run_method/authoring/manifest/face/face.rs:100,116-118`，只按 `validate_name` 要求 snake_case），默认取模块名。于是两个 `id` 不同、路径相同的兄弟可以同时注册。后果是三处“谁赢”的静默选择：
  - `RegistryIndex` 的 `by_path` 索引（`core/src/registry_core/tree/index/index.rs:88-93`）：`insert` 覆盖，后插入者赢，查询结果取决于 `BTreeMap<NodeId>` 遍历顺序；
  - `path_for`（`core/src/registry_core/tree/query/query.rs:86-96`）：两个节点的路径字符串完全相同，下游任何按路径记账的判断都无法区分；
  - `resolve_path`（`resolution.rs:22-28`）：`find_map` 返回**第一个**匹配，嫁接切口选择器静默落在其中一个上——而同模块的 `resolve_node`（`:146-175`）对同名/同 kind/同路径明确返回 `Resolution::Ambiguous`。
- 判据：本工作区既定立场是“匹配到多个面一律报出，绝不静默解析”（`tree/ports/ports.rs:29-33` 明写）。路径在本仓不只是显示串：连接器的准入门禁就用 `path_is_strictly_under(provider_path, owner_path)` 判定内外（`tree/connector/connector.rs:174-179,284-286`），路径碰撞会让“内/外”分类失去单值含义。重复 `stable_name` 都被拒绝，重复“槽位名”（路径段）却放行，标准不一致。
- 最小修复方向：在 `plan_batch` 增加兄弟级 `registry_name` 查重（按 `snapshot.parent` 收集现存子项名与批内名字，冲突即 `RegistryError`，文案与 `duplicate stable face name` 同构）；`resolve_path` 改为返回 `Resolution`（`One/Ambiguous/Missing`），与 `resolve_node` 合并成一条解析规则。
- 复核手段：单测探针：同一 `parent` 下注册两个 `registry_name: "a"`、`id` 不同的快照，期望 `Err`；当前 `Ok`，且 `registry.index().by_path.len()` 比节点数少一、`registry.path_for(id2) == registry.path_for(id1)`。

### K-06 MAJOR — E 函数内谬误（`requires` 的畸形条目被静默吞掉，校验器与快照解析器不一致）

- 位置：`core/src/registry_core/authoring/parse/rules.rs:108-119`（静默版）对比 `:123-136`（严格版），调用点 `core/src/registry_core/authoring/snapshot/snapshot.rs:116`
- 现象：`parse_requirements_owned` 用 `filter_map(|item| { let (capability, provider) = item.split_once("=>")?; ... })`：缺 `=>` 的条目被直接丢弃且无报告；而 `parse_requirements` 对同一文本返回 `Err("requires entries must use capability=>provider syntax")`。快照构建 `snapshot_from_values` 调用的是**静默版**，没有先跑严格版。
- 判据：`requires` 是连接器准入的输入（`connector.rs:230-360` 逐条检查提供者与 admission）。丢一条 `requires` 等于该输入不存在：连接器不再检查它，注册静默通过，而作者以为已经声明了依赖。严格校验器只挂在编辑入口（`run_method/authoring/manifest/face/face.rs:131-133`），手写或旧版本写下的文件不经过它。
- 最小修复方向：删掉 `parse_requirements_owned` 的 `filter_map`，改为 `collect::<Result<Vec<_>, _>>()` 复用 `parse_requirements` 的判据（或先调用 `parse_requirements(value)?`）；`render_requirements`（`rules.rs:91-104`）同样应拒绝而不是丢弃。
- 复核手段：单测探针：`snapshot_from_values` 传入 `requires = "cap"`（无 `=>`）期望 `Err`，当前得到空 `requires`；等价探针 `parse_requirements_owned("cap") == []`。

### K-07 MAJOR — D 大逻辑（插件锁的 schema 门禁按行序生效，且对无记录的锁完全不生效）

- 位置：`core/src/registry_core/plugin/catalog/catalog.rs:126-145`（判定在记录循环内部）、`:115-117`（`schema_matches`）
- 现象：schema 校验写在“处理一条记录”的分支里：

  ```text
  if let Some(value) = line.strip_prefix("# xirang-schema=") { schema = Some(value.trim()); continue; }
  if line.starts_with('#') { continue; }
  if let Some(version) = schema && !schema_matches(version) { return Err(...); }
  ```

  因此：①只有注释与表头、没有任何记录的锁 → 循环体永不走到校验 → `Ok`（空目录），与函数文档“refusing unknown schemas”（`:120`）不符；②表头写在记录**之后**时，前面的记录在 `schema == None` 下解析 → 旧/新 schema 的记录被当作本版本可读；③同一文件里第二行 `# xirang-schema=` 会静默覆盖第一行（此处没有重复键拒绝，而同一模块的 `PluginRecord` 与 `graft.plan` 都拒绝重复）。
- 判据：schema 是身份/命名空间的版本闸门，它的全部意义是“读不懂就拒绝”（`catalog.rs:103-114` 注释）。按行序生效意味着一个乱序或被裁剪过的锁文件能绕过它，且方向是**放行**（读到本不兼容的语义），不是拒绝。
- 最小修复方向：把 schema 校验移到循环之后（`if let Some(version) = schema && !schema_matches(version) { return Err(...) }`），并拒绝重复的 schema 表头（第二行出现即 `Err`）；记录出现在表头之前也应报错。
- 复核手段：单测探针：`PluginCatalog::parse("# xirang-schema=v4\n")` 期望 `Err`，当前 `Ok(空)`；`PluginCatalog::parse("user|fw|p|1|c|sha256:b|extension\n# xirang-schema=v4\n")` 期望 `Err`，当前 `Ok(1 条记录)`。

### K-08 MAJOR — D 大逻辑（`graft_entries` 跳过**所有**带 `cfg` 的模块，被开启的特性也照跳）

- 位置：`core/src/registry_core/syntax/entries/graft.rs:105-116`
- 现象：

  ```text
  fn visit_item_mod(&mut self, item: &'ast syn::ItemMod) {
      if item.attrs.iter().any(|attribute| attribute.path().is_ident("cfg")) { return; }
      syn::visit::visit_item_mod(self, item);
  }
  ```

  判定是“有任意 `cfg` 属性就整棵模块不进入”，理由是注释写的“A module the compiler may drop cannot contribute to the plan”。但条目级的 `cfg` 是**保留**的（同文件 `:121-132` 收集 `cfg`，测试 `graft_parser_ignores_declarations_that_are_not_items` 钉住 `#[cfg(feature = "optional-graft")]` 的条目仍被收集）。于是同一个 `#[cfg(feature = "fast")]`：写在条目上会进入计划，写在 `mod` 上则整模块的 `static_graft_plan!` 被悄悄丢掉——包括 `#[cfg(not(test))]` 这种必然生效的门控。
- 判据：构建捕获的静态 graft 表是宿主启用 overlay 的唯一来源（`tree/graft_ops/overlay.rs:111-121`、`release.rs:196-217`）。少一条切口 = 该槽位静默地跑基座实现，而源码里明明写着要嫁接；没有任何诊断报出这件事。跳过 `#[cfg(test)] mod tests` 是合理目标，但用“任意 cfg”近似它，代价是把真实门控一起丢掉。
- 最小修复方向：把跳过条件收窄为“编译器可能丢弃”的判定，即只跳 `#[cfg(test)]`（以及 `#[cfg(not(...))]`/特性未开启的情况交给既有的 cfg 求值路径），或在跳过时记录一条报告（该模块里有 N 条切口未被纳入）。
- 复核手段：单测探针：`graft_entries("#[cfg(feature = \"on\")] mod g { xirang::static_graft_plan!(FRAMEWORK, cut \"root/a\" graft \"x\"); }")` 期望收集到 1 条，当前得到 0 条。

### K-09 MAJOR — D 大逻辑（trait 标签派生遇到带逗号的泛型实参就失败，构建期会误报“缺 trait”）

- 位置：`core/src/registry_core/syntax/fields.rs:104-131`（`string_list`/`trait_labels`）、`:147-155`（`path_list` 经 `split_top_level`）、`syntax/tokens.rs:139-155`
- 现象：`string_list("handle_traits")` 会先取 `path_list("handle_contracts")`，再把这些路径 `join(",")` 交给 `authoring::parse::trait_names_from_paths` 派生标签。两处都按**裸逗号**切分：`split_top_level`（`core/src/registry_core/syntax/tokens.rs:139-155`）不看尖括号深度，`trait_names_from_paths`（`core/src/registry_core/authoring/parse/parse.rs:205-227`）也按 `,` 切。于是 `handle_contracts: [crate::ControlHandle<u8, u16>]` 被切成 `crate::ControlHandle<u8` 与 `u16>` 两段，`syn::parse2::<syn::Path>` 失败 → `path_list` 返回 `None` → `unwrap_or_default()` 变空 → `trait_labels` 退回 `written_string_list("handle_traits")`（未写标签时为空）。
- 判据：`trait_names_from_paths` 的注释明确设想“A trait path is a *type path*, so it can carry generic arguments — `Trait<A<B<…>>>`”（`core/src/registry_core/authoring/parse/parse.rs:211-217`），只覆盖了无逗号的嵌套形态；两个 `split(',')` 是这段注释与实现之间的裂缝。下游 `build_method/contracts.rs:146-165` 用 `unwrap_or_default()` 消费它，因此标签为空时构建会报“required handle trait is missing”——对一个合法声明报结构错误（false reject），而 `run_method/authoring/manifest/parse/parse.rs:141-146` 会把该可编辑字段读成空、写回时把标签列表抹掉。
- 最小修复方向：`split_top_level` 增加尖括号深度（`<` 增、`>` 减）后再按顶层逗号切；`trait_names_from_paths` 改为接收 `&[String]`（或先按同一规则切分）而不是吃一个 `join(",")` 的字符串。
- 复核手段：单测探针：`FaceSyntax::string_list("handle_traits")`（面里写 `handle_contracts: [crate::ControlHandle<u8, u16>]`）期望 `["ControlHandle"]`，当前 `None`；`path_list("handle_contracts")` 期望 1 条，当前 `None`。

### K-10 MINOR — D 大逻辑（注册规则用“整文件文本启发式”解析，可读到注释里的伪子句）

- 位置：`core/src/registry_core/authoring/parse/rules.rs:27-55`（`rule_syntax_from_text`）、`:14`（`quoted_list_field`）、`:18-23`（`quoted_value_field`）
- 现象：对**整个文件文本**做 `split_once('=')`，再用 `find(".require_exports(")` 之类的字面量搜索取第一条方括号列表。初始器之后、注释之内的同名标记会被当成真子句：

  ```text
  pub const REGISTRATION_RULE: RegistrationRule = RegistrationRule::new() // 旧写法: .require_exports(&["wrong"])
      .require_exports(&["control.render"]);
  ```

  读出 `exports:wrong`。若第一个 `=` 落在真正初始器之后的位置，`quoted_value_field` 取不到 `preset` 时会静默降级。
- 判据：该函数是规则文件读入的唯一入口（`run_method/authoring/parse/parse.rs:44-53` 直接把文件文本喂进来），输出又会被 `render_registration_rule` 写回文件：读错即写错，把结构规则换成另一条（或降级成 `ANY`＝无结构要求），是静默弱化。工具自己写出的形状不受影响，但手写/旧版本文件不设防——这类输入正是解析器存在的理由。
- 最小修复方向：用 `syn::parse_file` 定位 `REGISTRATION_RULE` 这个 `const` 项的初始化表达式，再在其 AST 上取 `.require_*` 调用；返回 `Result<_, FaceParseError>`，无法识别时拒绝而不是当成 `ANY`。
- 复核手段：探针：把上面那段带注释的规则文本喂给 `rule_syntax_from_text`，期望 `Err` 或 `exports:control.render`，当前得到 `exports:wrong`。

### K-11 MINOR — D 大逻辑（官方通道的密钥信任判定可以取工件自述的指纹）

- 位置：`core/src/registry_core/plugin/trust/trust.rs:107-129`（`verify` 内 `let fingerprint = key_fingerprint.or(manifest.public_key_fingerprint)`）、`:163-165`（`verify_with` 同样回退）
- 现象：官方来源 + 策略信任列表非空时，被拿去与信任列表比对的指纹可由调用方传入，**也可由 manifest 自述**（`PluginManifest.public_key_fingerprint` 是工件字段）。`PluginArtifact.key_fingerprint` 也是公开字段（`plugin/artifact/artifact.rs:22-24`），调用方可以传 `None`。
- 判据：“这个密钥是否被信任”这一判定，其输入取值不应由被判定方自己提供。当前 API 允许 adapter 只传 `None`（`trust.rs` 的测试正是这么用的：`policy.verify(manifest, b"abc", None)` 配一个自带指纹的 manifest 得到 `Ok`），于是信任列表比对的对象变成工件声明值；`PluginTrustError::MissingOfficialKey` 的语义也从“没有外部来源的密钥身份”变成“工件没自称”。真正的签名校验仍由宿主验证器完成（`:143-171` 委托），所以危害限于“信任列表过滤”这一层，但这一层正是 policy 的全部职责。
- 最小修复方向：官方通道要求外部来源的指纹（把 `key_fingerprint: &str` 作为必需参数，或在 `Some` 与 manifest 回退之间区分错误码并在文档里写明）；若保留回退，至少在 `verify` 的文档里写出“回退值来自工件”。
- 复核手段：探针：`PluginTrustPolicy::official(&[KEY], &[]).verify(manifest_without_host_fingerprint, bytes, None)` 当前 `Ok`（manifest 自述 KEY）；期望 `Err(MissingOfficialKey)` 或文档明确该回退。

### K-12 MINOR — D 大逻辑（“官方插件必须已签名”这条拒绝被 `require_digest` 顺带关掉）

- 位置：`core/src/registry_core/plugin/plugin_policy/plugin_policy.rs:145-155`
- 现象：

  ```text
  if self.require_digest && plugin.source == PluginSource::Official
      && plugin.signature.is_none_or(|signature| signature.trim().is_empty())
  { return PluginDecision::Rejected(PluginRejectReason::MissingSignature); }
  ```

  签名要求挂在 `require_digest` 之下。`PluginPolicy::open`（`:90-97`，`require_digest: false`）因此接受一个**无签名**的官方 manifest——测试 `plugin_policy_keeps_frameworks_and_sources_separate`（`:206`）正是这么断言的。
- 判据：`require_digest` 的文档是“Reject manifests whose checksum is not a cryptographic digest”（`:71-73`），一个格式要求；`PluginRejectReason::MissingSignature` 的文档是“An official plugin carried no usable signature”（`plugin/artifact/artifact.rs:171-173`），一个信任要求。把后者实现为前者的从属条件，等于让宿主为了放宽校验和格式而静默放弃官方签名要求。`open` 是开发策略，但它同时是文档化的“admit both official and user sources”，没有一处说明它会连签名一起放弃。
- 最小修复方向：把签名检查独立出来（例如按 `allow_official` 或新增 `require_signature` 门控），或在 `open` 的文档里显式写出“官方来源也不再要求签名”。
- 复核手段：单测探针：`PluginPolicy::open(fw).decision(official_manifest_with_signature_none, None)` 期望 `Rejected(MissingSignature)`（或文档变更），当前 `Accepted`。

### K-13 MINOR — D 大逻辑（锁记录与 manifest 的 checksum 用字面比较，摘要有两种拼法却只认一种）

- 位置：`core/src/registry_core/plugin/catalog/catalog.rs:281-301`（`accounts_for` 的 `record.checksum == candidate.checksum`）、`:243-256`（`contains_manifest`）
- 现象：`PluginManifest.checksum` 的文档是“Digest of the plugin bytes, optionally carrying a `sha256:` prefix”（`plugin/contracts/contracts.rs:443-445`），`is_sha256`（`core/src/registry_core/plugin/artifact/artifact.rs:191-194`）与 `PluginManifest` 的 `verify_bytes`（`core/src/registry_core/plugin/contracts/contracts.rs:466-472`）都接受带前缀与裸十六进制两种拼法且大小写不敏感；但锁记录匹配是 `String` 字面相等。于是描述同一段字节的 `sha256:abc…` 与 `abc…` 会得出 `LockMismatch`。
- 判据：同一个“摘要”概念在三个读取点被两种规则解释（前两处归一化，第三处不归一化），这是“同一规则两处实现”的典型形态；而 `LockMismatch` 是拒绝方向的 hard failure，作者看到的是“锁里没有这个插件”，而不是“拼法不同”。
- 最小修复方向：`accounts_for` 里对 checksum 做与 `verify_bytes` 相同的归一化（strip `sha256:` + `eq_ignore_ascii_case`），或让 `PluginRecord::parse` 在解析时归一化存储。
- 复核手段：单测探针：锁写 `...|sha256:ba7816bf…|...`、manifest 写 `ba7816bf…`（同一摘要），`contains_manifest` 期望 `true`，当前 `false`。

### K-14 MINOR — D 大逻辑（`split_face_fields` 的尖括号计数会把后续字段吞进前一个值）

- 位置：`core/src/registry_core/syntax/tokens.rs:64-85`
- 现象：`<` 一律使 `angles += 1`，`>` 才减，且 `,`/`;` 只在 `angles == 0` 时才算分隔符；`starts_face_field`（`:97-101`）也要求 `angles == 0`。因此值里出现一个比较用的 `<` 之后，该字段之后的所有字段会被并入同一个值：

  ```text
  crate::control_object! { kind: Tool, flow: |a: u32| a < b, needs_registry: true }
  ```

  `<` 使 `angles` 停在 1，`,` 不再是分隔符，`needs_registry` 也不再是字段起点；`parse_fields` 于是只有一个字段 `flow`，`needs_registry` 静默取默认 `false`。
- 判据：同一文件的正上方注释（`:59-63`）与测试 `face_field_splitting_survives_generics_and_closures`（`:255-287`）声明这条跟踪的目的是“泛型实参里的逗号不算字段分隔符”，而闭包正是被显式支持的值形状之一；一个比较运算符就把它变成“吞掉后面所有字段”。容忍手写语法是这个解析器的设计目标，因此静默丢字段比报错更坏。
- 最小修复方向：区分“泛型实参的 `<`”与“比较的 `<`”（例如只在 `<` 紧邻标识符/路径片段且当前值未以表达式片段结束时计数，或对 `,`/`;` 恢复分隔符角色并把尖括号深度限定在 `path`/`preset`/`parts` 等已知类型字段内）。
- 复核手段：探针：`split_face_fields(syn::parse_str("kind: Tool, flow: |a: u32| a < b, needs_registry: true").unwrap())` 期望 4 个字段，当前得到 3 个（最后一个并入 `flow`）。

### K-15 MINOR — D 大逻辑（`from_jsonl` 忽略 `}` 之后的残留文本，整条记录可被静默丢弃）

- 位置：`core/src/registry_core/mir/jsonl.rs:111-157`（`parse_object` 在 `:123` 与 `:153` 直接返回，不检查 `cursor` 之后的内容）
- 现象：一条记录解析到 `}` 即成功返回，行内剩余字节不再检查。两行记录粘成一行（丢失换行）时，第二条会被静默丢掉：

  ```text
  {"kind":"function","name":"a"}{"kind":"call","caller":"a","callee":"b","mir_line":1"}
  ```

  只留下 `a`。
- 判据：函数文档承诺“A malformed line fails the whole parse with its 1-based line number”（`:19-20`）。这是 MIR 证据（调用图）的读入口，静默少一条记录会直接改变 delta/调用图结论，且没有行号可查。
- 最小修复方向：`parse_object` 返回前跳过空白并要求已到行尾，否则报 `"trailing text after record"`（带行号）。
- 复核手段：单测探针：`MirGraph::from_jsonl("{\"kind\":\"function\",\"name\":\"a\"}{\"kind\":\"function\",\"name\":\"b\"}\n")` 期望 `Err`，当前 `Ok`（只含 `a`）。

### K-16 MINOR — E 函数内谬误（掩码失败时回退到**未掩码**源码，方向最坏）

- 位置：`core/src/registry_core/source/source.rs:429`（`String::from_utf8(masked).unwrap_or_else(|_| source.to_owned())`）
- 现象：掩码过程如果产生非法 UTF-8，`mask` 会返回**原始源码**。当前实现不可能触发（掩码只把字节换成 ASCII 空格，且起点都在字符边界上：能进入掩码分支的字节都不是 UTF-8 续字节），但这条兜底一旦生效，方向是最坏的：注释、字符串、raw 字符串会全部被 `function_symbols`/`direct_calls`/`function_source_range` 当成代码，凭空造出函数与调用，而不是少认一些。
- 判据：这是纯文本变换的核心不变量（掩码必须保留偏移且保持 UTF-8 合法）。静默回退等于把“检查失效”变成“输出错误答案”，而调用方无法区分。同类原则在本仓另有先例：`tokens.rs:239-243` 的 `only_group` 遇到形状不符返回 `None` 而不是猜。
- 最小修复方向：把兜底改成 `String::from_utf8_lossy(&masked).into_owned()`（保留掩码结果），或让 `mask` 返回 `Result`；无论如何不要回退到未掩码文本。
- 复核手段：单测探针：构造一个 `masked` 非 UTF-8 的输入（可临时在测试内构造，或断言 `mask_non_code` 的输出长度恒等于输入长度——该不变量当前成立，能间接固化这条路径）。

### K-17 MINOR — D 大逻辑（`registration_kinds` 用“整行含 `kind:`”启发式，既过收也漏收）

- 位置：`core/src/registry_core/source/source.rs:434-466`
- 现象：对掩码后的文本逐行 `trimmed.split_once("kind:")`，再取随后的第一个 token 作为 kind。它无法区分注册声明与普通代码：`let kind: String = value;` 会贡献一个 kind `String`（`String` 全是字母，通过过滤器）；反之 `kind:` 与其值分行书写时该行取到空值而被跳过。函数文档写的是“Collect the deduplicated `kind:` values used in registration declarations”（`:432-433`）。
- 判据：调用方是 MCP 的源码查询（`mcp/tools.rs:344`），产物被当作“这个文件里的注册 kind”呈现；过收会让不存在的 kind 出现在结果里，漏收会让存在的 kind 消失，两者都无法从输出看出来。
- 最小修复方向：复用已有的注册面解析（`syntax::parse_face` → `FaceSyntax::path("kind")`）来取 kind，或至少要求该行同时出现注册宏名；把“只扫文本”的收益（不依赖 `syntax` 特性）用文档写明。
- 复核手段：单测探针：`registration_kinds("fn f() { let kind: String = x; }")` 期望 `[]`，当前 `["String"]`。

### K-18 MINOR — D 大逻辑（静态拓扑检查不查重复身份，运行时注册路径却查）

- 位置：`core/src/registry_core/diagnostic/topology.rs:39-46`（`ids` 是 `BTreeSet`、`owners`/`parents` 是 `BTreeMap`，重复 id 后者覆盖前者）
- 现象：两条 `TopologyRecord` 携带同一 `id` 时，缺失父级检查、父级类型检查、环检查都照常通过（后一条覆盖前一条的表项），不会报任何诊断；而运行时的 `plan_batch` 对同一情况明确报 `duplicate registration node identity`（`tree/transaction/transaction.rs:116-128`）。
- 判据：两处都在回答“这棵收集到的树是否自洽”，一条规则两种严格度；构建期这份是静态计划的输入（`validate_face_topology`），漏掉重复身份意味着计划里可能出现两个不同声明共用一个身份，而下游的 `StaticPlan` 的 `find`（`core/src/registry_core/release/release.rs:243-245`）按身份线性查找，只会命中第一个。
- 最小修复方向：在 `validate_face_topology` 里加一项 `ids.insert` 返回值检查，冲突即诊断（文案与运行时同构）。
- 复核手段：单测探针：两条同 id、父级合法的记录，`validate_face_topology` 期望至少 1 条诊断，当前 0 条。

### K-19 MINOR — A 命名（公开 API 的失败模式与命名不符：`is_face_source` 会在非注册面上返回 true；`*_matches` 需要未强制的小写前提）

- 位置：`core/src/registry_core/syntax/face.rs:169-171`（`is_face_source`）、`core/src/registry_core/declaration/source_location.rs:61-67`（`source_file_matches`）
- 现象：①`is_face_source(source, marker)` 对“任意一行等于 marker 的文件”返回 `true`，即使文件里没有任何注册宏；调用方 `run_method/authoring/operations/operations.rs:293` 据此把文件当注册面处理。②`source_file_matches(file, needle)` 的文档要求 `needle` 已小写（`:56-60`），但没有任何断言或类型承担它：调用方传混合大小写时会静默搜不到（`file` 被小写化，`needle` 没有）。
- 判据：①名字读作“这是不是一个注册面源码”，实现的语义是“有标记行，或解析出了一个面”，前者的肯定分支会在解析失败时仍然成立（标记行本身不保证有面）。②用一个 `bool` 返回表达“可能匹配”而把前提留在调用方脑中是命名/契约问题（交底在注释里）。
- 最小修复方向：①`is_face_source` 改名 `has_generated_marker_or_parses_as_face`（或让标记行只作短路、解析失败时返回 false）；②`source_file_matches` 在 `debug_assert!(needle == needle.to_ascii_lowercase())`，或直接改为在函数内对 `needle` 做一次性小写（当前对每个候选分配两次字符串，顺手也少了这个前提）。
- 复核手段：探针：`source_file_matches("Control/Button.rs", "Control/Button")` 期望 `true`，当前 `false`（`needle` 未小写）。

### K-20 MINOR — B 责任与碎片化（三页共用同一段“为什么拆”的说明；同一历史叙事散落在生产文件里）

- 位置：`core/src/registry_core/tree/graft_ops/record.rs:24-38`、`graft_ops/reconcile.rs:11-16`、`graft_ops/apply.rs:12-17`；`tree/metadata/metadata.rs:30-36`、`tree/index/index.rs:36-45`、`diagnostic/error.rs:118-132`
- 现象：record/reconcile/apply 三页各写一段意思相同的“Split decision”说明（为什么把记录管线拆成三页），共约 20 行；另外三处注释叙述的是过去的删除动作与审计阶段（“were zero-caller readers and were removed in B3a”“Principle: a zero-caller deletion is reversed when a real caller appears”）。
- 判据：读者要理解一次拆分需要跳三页读三遍同一段话（信噪比），而历史叙事是提交信息的内容：它描述的不是代码契约，下次重构后必然过时（本仓自我描述必须与行为一致的标准）。这两类注释都不会拦住任何错误，却占据生产文件的行数预算。
- 最小修复方向：拆分说明只保留在 `record.rs` 一处，另两页改为一行指针（`// Split rationale: see record.rs.`）；B3a 叙事删掉或移进 `docs/`，只留“零调用者项已删除”的现状说明。
- 复核手段：`grep -rn "Split decision" core/src` 与 `grep -rn "B3a" core/src` 计数即可核对（当前前者 3 处、后者 3 处）。

### K-21 MINOR — D 大逻辑（同一“trait 标签”概念有两份派生实现，泛型路径下结果不同）

- 位置：`core/src/registry_core/syntax/fields.rs:104-131`（数据侧，经 `core/src/registry_core/authoring/parse/parse.rs:205-227` 的 syn 路径解析）与 `core/src/registry_core/authoring/field_presentation.rs:341-353`（展示侧 `derived_trait_names`，`rsplit("::")`）
- 现象：对 `crate::ControlHandle<u8, u16>`，展示侧给出 `ControlHandle<u8, u16>`（`rsplit("::").next()` 保留泛型实参），数据侧给出 `ControlHandle`（`syn::Path` 最后一段的 ident）。字段帮助文本承诺“Searchable trait labels derived from the handle contract paths”（`field_presentation.rs:217-218`），因此两边应当同值。
- 判据：同一概念的两种派生实现，差异只在带泛型实参时显现；作者在 Studio 里看到 `<derived: ControlHandle<u8, u16>>`，而写盘/校验用的是 `ControlHandle`，这在“会对渲染文本做差异比较”的创作写入方（`core/src/registry_core/authoring/parse/parse.rs:153-157` 说明过这一点）里会产生虚假改动。
- 最小修复方向：`derived_trait_names` 复用 `authoring::parse::trait_names_from_paths`（并修好 K-09 的切分），删掉自己的 `rsplit`。
- 复核手段：探针：对同一输入分别调用 `derived_trait_names("crate::ControlHandle<u8, u16>")` 与 `trait_names_from_paths("crate::ControlHandle<u8,u16>")`，断言两者相等（当前不等）。

### K-22 MINOR — E 函数内谬误（公开 API 里按调用方给的预算递归，深度不受本模块约束）

- 位置：`core/src/registry_core/mir/call_tree.rs:373-405`（`assign_lane` 递归）、`:152-153`（只钳 `limit`，不钳 `depth`）
- 现象：`call_tree(focus, relations, depth, limit)` 的树高由 `depth` 决定，`assign_lane` 沿父子链递归；`limit` 被 `max(1)` 钳住，`depth` 没有上限。调用方（Studio 传的是个位/十位跳数）若传 `depth = 10^6` 且关系链上有同样多的不同符号，递归深度与树高同阶。
- 判据：本仓对“栈溢出不是 `Result`”有明确立场（`syntax/nesting.rs:1-43` 的守卫），`call_tree` 是暴露给执行面的公开 API，输入来自调用方的关系列表；这条递归没有任何守卫或文档说明。
- 最小修复方向：把 `assign_lane` 改成显式栈的迭代版本，或在文档里写明 `depth` 必须远小于调用栈预算，并在 `call_tree` 里对 `depth` 取 `min(limit, …)`。
- 复核手段：探针：`call_tree("a", &chain_of(200_000), 200_000, 200_000)`（链上符号各不相同）在小栈线程上期望不 abort；当前会递归 20 万层。

### K-23 MINOR — D 大逻辑（任何以 `_object` 结尾的用户宏都被当作注册面，非注册宏会得到错误的构建诊断）

- 位置：`core/src/registry_core/syntax/face.rs:250-258`（`is_face_macro = … || macro_name.ends_with("_object")`）
- 现象：`widget_object! { name: "x", size: 3 }` 这类与 XiRang 无关的宏，只要 token 形状是 `ident: value`，`parse_faces` 就会把它当作注册面（`name` 是词表字段，`size` 不是词表字段但 `parse_fields` 不校验字段名，只要求 `ident :`）。下游 `build_method/validation.rs:52-70` 把 `*_object` 面按父级宏处理，要求显式 `parent:`，于是会为这个无关宏报 `parent-macro` 构建错误。
- 判据：`_object` 后缀规则是有意的（`core/src/registry_core/syntax/face.rs:254-255` 与生成别名有关），但没有任何一处把“宏名是否属于本次构建的注册机制”作为准入条件；代价是误报构建错误与误收注册面（`FaceSyntax` 会被当作面数据使用，`run_method/authoring/manifest/parse/parse.rs:39` 直接读 `macro_name`）。
- 最小修复方向：要求宏路径的段数与来源符合注册机制（例如末段必须是 `control_object`/`external_object`/`<crate>_object` 且该 crate 等于本次构建的宿主 crate），或在 `parse_fields` 拒绝不在 `FACE_FIELD_ORDER` 中的字段名。
- 复核手段：单测探针：`parse_face("fn f() { widget_object! { name: \"x\", size: 3 } }")` 期望 `Ok(None)`，当前 `Ok(Some(…))`。

### K-24 MINOR — A 命名（测试名与它断言的行为相反）

- 位置：`core/src/registry_core/source/source_tests.rs:207-236`（`fn multiline_literals_and_block_comments_leak_code()`）
- 现象：名字说“跨行字面量与块注释会泄漏代码”，而四条断言全部断言**不泄漏**（`assert_eq!(…, None)`、`assert!(registration_kinds(…).is_empty())`）。同类里 `function_symbols` 的 `mask` 行为是对的，错的是名字。
- 判据：测试名是维护者定位回归的第一入口；写反的名字会让下一个读它的人以为这是“已知泄漏”的现状钉，从而放过真正的泄漏。
- 最小修复方向：改名 `multiline_literals_and_block_comments_do_not_leak_code`（或 `…_are_masked`）。
- 复核手段：`grep -rn "leak_code" core/src`。

## 注释清晰度与可读性

（本节按队长追加的审计轴单列。上面 K 系列里凡是“注释与行为不符”的判断在此复述并给出最小改法，正文处保留指针。）

### C-01 MAJOR — 注释描述的行为与实现不符（模块自述承诺“两种拼法同一个门禁”，实际丢字段）

- 位置：`core/src/registry_core/authoring/parse/admission.rs:5-8`
- 现象：模块文档写“Admission is written either as the compact `allow:…`/`deny:…`/`ANY` form or as the `Admission::new(&[…], &[…])` constructor an editor emits. Both spellings describe the same gate, so both are read here.”，而 `:40-44` 在两张列表同时非空时只保留 allow。
- 判据：见 K-01。这条注释让读者相信“读一遍写一遍是无损的”，正是最需要被推翻的那句话；过时注释比没有注释更坏（维护者判据 1）。
- 最小改法：修好 K-01 后此注释才成立；若暂不修，必须改成明确的边界声明（“两列表同时出现时 deny 会被丢弃，见 K-01；勿据此写回”）。

### C-02 MAJOR — 注释描述的类型与实现不符（`CallSite` 的文档写的是“一条调用边”）

- 位置：`core/src/registry_core/declaration/call_evidence.rs:6-7`（`CallSite` 的 doc）与 `:31-32`（`CallEdge` 的 doc，逐字相同）
- 现象：`CallSite` 的字段是 `node`/`function`/`frame_id`/`source`（一个调用点），但它的文档是“One call edge that was observed while a `CallTrace` frame was active.”——那句话正是 `CallEdge`（caller+callee 对）的定义，两句逐字重复。
- 判据：文档字符串是公开面的一部分（crate 开了 `#![warn(missing_docs)]`，docs.rs 是主读者），把“点”写成“边”会让按该名索引的人找错类型；重复文本会让下一次修改只改一处。维护者判据 1。
- 最小改法：`CallSite` 改为“One observed call site: which node/function was active in which trace frame, and where it was written.”，`CallEdge` 保留现句并注明两端都是 `CallSite`。

### C-03 MAJOR — 模块自述的解析规则与同模块的另一函数相反（“绝不静默解析”vs `resolve_path` 取第一个）

- 位置：`core/src/registry_core/tree/ports/ports.rs:29-33` 与 `tree/graft_ops/resolution.rs:22-28`、`:146-175`
- 现象：ports 的模块文档写“a handle or a port that matches more than one face is **reported, never resolved silently** — an implicit "nearest wins" rule is exactly the kind of hidden behaviour this workspace refuses.”；`resolution.rs` 的 `resolve_node` 遵守它（返回 `Resolution::Ambiguous`），而同一文件上方的 `resolve_path` 用 `find_map` 取第一个匹配。
- 判据：见 K-05。注释陈述的是全仓立场，实现只在其中一条入口上兑现，读者无法从注释知道哪一条被豁免。
- 最小改法：修好 K-05（`resolve_path` 返回 `Ambiguous`），或在该注释里点明 `resolve_path` 是例外并说明为什么。

### C-04 MINOR — 同一段说明重复三页、历史叙事进入生产文件

- 位置：`tree/graft_ops/record.rs:24-38`、`tree/graft_ops/reconcile.rs:11-16`、`tree/graft_ops/apply.rs:12-17`；`tree/metadata/metadata.rs:30-36`、`tree/index/index.rs:36-45`、`diagnostic/error.rs:118-132`
- 现象/判据/最小改法：见 K-20（`// Split decision:` 三段同义；`B3a` 叙事三处）。补充：`diagnostic/error.rs:126` 的“Principle: a zero-caller deletion is reversed when a real caller appears.”是一条**承诺式**注释，它描述流程而不是代码契约，属于维护者判据 6 点名的“以后再改”式说明。
- 复核手段：`grep -rn "Split decision" core/src`（3 处）、`grep -rn "B3a" core/src`（3 处）。

### C-05 MINOR — 文档头用 `//` 而非 `//!`，与同层模块不一致

- 位置：`core/src/registry_core/tree/registry.rs:1-2`（`// The one registry type…`）、`diagnostic/topology.rs:3-7`（`// ---` 横幅 + `//` 说明）
- 现象：同层与同模块的其它文件用 `//!`（如 `tree/tree.rs:1-7`、`syntax/nesting.rs:1-43`）。写成 `//` 的说明不会进入 rustdoc，也不会被 `missing_docs` 相关工具看见。
- 判据：维护者判据 3/7：同一份公开面里两种约定并存，读者无法判断哪一段是模块文档；这也不是风格口味问题——`tree/registry.rs` 是 `Registry` 这个最重要类型的所在页，它的说明在 docs.rs 上是缺失的。
- 最小改法：把这两处改成 `//!`（或反过来把 `//!` 全改 `//`，但那样会丢掉公开文档）。
- 复核手段：`grep -rn "^// [A-Z]" core/src/registry_core/tree/registry.rs core/src/registry_core/diagnostic/topology.rs`。

### C-06 MINOR — 同一个文档注释里两个 `# Panics` 段，第一段讲的是别的事

- 位置：`core/src/registry_core/release/release.rs:259-289`
- 现象：`:261` 与 `:279` 各有一个 `# Panics` 标题。第一段（`:263-277`）写的是“本函数是 `RegistrationRule::validate` 的 const 孪生、代价不同、必须一起改”，与 panic 无关；第二段（`:281-289`）才是真正的 panic 契约。
- 判据：`# Panics` 是读者按标题检索契约的锚点（rustdoc 会把它排成小节）；放在该标题下的内容会被当作 panic 契约阅读。维护者判据 1/7。
- 最小改法：第一段移出 `# Panics`（放到正文或 `# Why this is a const twin` 之类标题下），只保留一段 panic 契约。
- 复核手段：`grep -n "# Panics" core/src/registry_core/release/release.rs`（应只剩 1 处）。

### C-07 MINOR — 同一模块文档整段重复（第二份只多一句）

- 位置：`core/src/registry_core/declaration/runtime_checks_tests.rs:1-22` 与 `:24-44`
- 现象：`:1-22` 的 `//!` 段与 `:24-44` 的 `//!` 段逐字重复（中文与英文各一遍），后者只多出 `:36-44` 关于 `health_check` 与五个测试名的补充。同时 `core/src/registry_core/source/source.rs:252-253` 与 `:254-256` 是叠在同一个函数上的两段文档，前一段是一句残留碎片；`mir/text.rs` 与 `plugin/trust/trust.rs` 也有“先说结论再说边界”的分段，但内容不重复。
- 判据：重复段落会各自漂移（本仓对“同一规则两处副本”的立场见 K-05 的判据）；文件行数预算被重复文本占用。
- 最小改法：`runtime_checks_tests.rs` 合并为一段（保留 `:36-44` 的补充）；`core/src/registry_core/source/source.rs:252` 的碎片句删除或并入下一段。
- 复核手段：`sed -n '1,44p' core/src/registry_core/declaration/runtime_checks_tests.rs`。

### C-08 MINOR — 安全相关的参数没有文档，回退语义只存在于代码里

- 位置：`core/src/registry_core/plugin/trust/trust.rs:86-93`
- 现象：`PluginTrustPolicy::verify` 的文档是“Run the checksum, revocation, and official-key checks; does no signature work.”，三个参数都没有说明；其中 `key_fingerprint: Option<&str>` 决定“拿哪个指纹去和信任列表比对”，并且在 `None` 时回退到 `manifest.public_key_fingerprint`（`:117-119`）——这条回退没有任何文档。
- 判据：维护者判据 2/5：这里最需要注释的正是“输入从哪来、为 `None` 时信任判定读谁”，而现在读者只能从实现推。与 K-11 是同一处，建议一并处理。
- 最小改法：为三个参数各写一行（尤其是 `key_fingerprint` 的来源与 `None` 的回退），并把这一步的判据写清（“指纹必须由验证宿主从密钥材料得出”）。

### C-09 MINOR — 字段帮助文本讲的是另一个机制（`runtime_checks`）

- 位置：`core/src/registry_core/authoring/field_presentation.rs:248-254`
- 现象：`RUNTIME_CHECKS` 的帮助是“Value checks retained according to the selected trace mode.”；而这个字段的实际作用是“宿主在取值跨边界时执行的检查”（`tree/inspection/inspection.rs:23-116` 的 `health_check`），保留策略属于 `TraceMode`（`declaration/call_evidence.rs:117-137`）。叠加 K-02（文件改这个字段在热重载里被忽略），帮助文本会让作者以为改它会立刻生效。
- 判据：该文件开头自己写下了判据——“The help text states what the field does, not what it sounds like it should do: a row whose sentence contradicts the code is worse than no row.”（`:6-8`）。
- 最小改法：改成“宿主在生产值时执行的具名检查；改动只在重新编译后生效”（并在 K-02 修好后删掉后半句）。
- 复核手段：与 `health_check` 的行为对照即可（`examples/control-button/tests/health_check.rs` 有端到端用例）。

### C-10 MINOR — 名字承担不了信息，只能靠注释补（`mir_line` 与 `resolve_*` 家族）

- 位置：`core/src/registry_core/mir/model.rs:28-30`、`:46-48`（`mir_line` 的文档是“1-based position of this record within the parsed MIR, not a source line.”）；`mir/text.rs:36-44`（文本解析器填的是 `graph.calls.len() + 1`，即序号而不是行号）
- 现象：字段名 `mir_line` 说的是“MIR 行号”，文档用一整句纠正为“记录序号”；两处解析器（JSONL 与文本）填进去的语义还不同（真实行号 vs 合成序号）。
- 判据：维护者判据 5：需要一句注释才能解释清楚的名字就是命名问题。`CallRelation` 的 `mir_line`（`core/src/registry_core/mir/model.rs:71-73`）继承同一名字，读者必须跳回 `MirCall` 才知道它的含义。
- 最小改法：改名 `mir_record` / `mir_record_index`（或在文本解析器里填真实行号，让名字成立）；文档改为一句。
- 复核手段：`grep -rn "mir_line" core/src | wc -l`（改动面）＋ 用文本 MIR 输入打印 `calls[0].mir_line` 观察它是否为源行号。

### C-11 MINOR — 术语一名多指（同一个“槽位名”概念四种叫法）

- 位置：`core/src/registry_core/plugin/contracts/contracts.rs:35-39`（`ContractId` 的 doc 叫它 “logical replacement slot”）、`tree/ports/ports.rs:21-27`（叫它 branch handle）、`authoring/face_field.rs:35-37`（`TREE_SLOT`，中文“树槽位”）、`tree/entry_pages/entry_pages.rs`/`tree/graft_ops/reports.rs`（叫它 slot/selector），字段本体名是 `registry_name`（`declaration/registration.rs:268-270`）。
- 现象：同一段字符串（路径的分量）在四个模块里被命名为 slot / branch handle / tree slot / selector，落盘字段又叫 `registry_name`；另有 “face / object / node / entry / 注册面 / 节点 / 条目” 一组近义名词混用（例如 `tree/query/query.rs` 的 `find_kind` 返回 “face”，而 `tree/metadata` 统计的是 “node”）。
- 判据：维护者判据 4。术语分裂让搜索失效：想知道“谁用了 registry_name 构成路径”的人，必须在四个近义词里各搜一遍。
- 最小改法：在 `declaration` 或 `lexicon` 里给这个概念定一个名字（建议 `slot_name`，与已有 `registry_name` 字段对齐并加一行别名说明），文档里统一；`face`/`node` 二选一定为“树上的项”，另一词只用于声明期。
- 复核手段：`grep -rn "branch handle\|tree slot\|logical slot\|slot name" core/src | wc -l`。

## 跨片提示（非本片区源码，但会改变本片区结论的正确性）

- `run_method/authoring/manifest/face/face.rs:140-143`：`flow_provider` 编辑路径直接用 `syn::parse_str::<syn::Path>(value)`，**没有** `guard_nesting`；而内核侧同名渲染入口（`authoring/parse/flow.rs:107`）是有的。同一份不可信值（清单/编辑器）在两条入口上一条有守卫一条没有——`flow_tests.rs:19-26` 记的“542 字节类型路径爆栈 abort”在该入口仍然成立。建议归入执行面片区或作为一条交叉发现处理。
- `build_method/contracts.rs:146-165`、`run_method/authoring/manifest/parse/parse.rs:141-146` 用 `unwrap_or_default()` 消费 `FaceSyntax::string_list`，因此 K-09 的 `None` 会退化为“空标签列表”；修 K-09 时这两处无需改动，但值得在提交信息里点出这条链路。
- `mcp/tools.rs:344` 是 K-17（`registration_kinds` 启发式）的唯一调用方。

## 查了、干净

**口径与数字**

- 覆盖：`find core -name '*.rs'` 的**全部 93 个文件、21787 行**都逐文件读过（`read` 工具，全文件而非片段）；`core/src/test/registry_rule/` 是空目录，无内容可读。
- 五条深读块（identity 974 + declaration 2468 + tree 4746 + syntax 2575 + authoring 1836 ≈ 10.6k 行）逐文件过；覆盖块（plugin 3557 + mir 1855 + source 1275 + diagnostic 940 ≈ 7.6k 行）逐文件过并额外追了调用方；全读块（lexicon 479、release 409、requirements 255、json 98）逐行过。
- 深度的诚实说明：生产文件是逐行读并逐条对照调用方；**内联测试与独立测试页（约 2.1k 行：`face_tests`、`source_location_tests`、`runtime_checks_tests`、`call_tree_tests`、`source_tests`、`artifact_tests`、`ports_tests`、`record_tests`、`fixtures`、`flow_tests`、`deep_input_tests`、`nesting_budget`、`ungated_authoring_data`）我读的是“它钉住了什么行为”**，用于反向验证我的发现是否已被既有测试覆盖，而不是逐行审查测试质量。K-04、K-07、K-18 正是这样确认“现有测试没覆盖”的。
- 运行过的东西：`cargo test -p xirang-conventions --offline documented_rust_blocks_parse`（报告改名与围栏改 `text` 后 1 passed）与 `cargo test -p xirang-core --offline --lib`（基线，结果见交付消息）。本轮**没有**新增测试或探针文件（写权限只覆盖本报告），因此 K 系列的复核手段都以“探针命令”形式给出，未逐条实跑；每条探针都写明了期望值与当前值。
- 未发现问题的轴：**F 铁律干净**——`grep -rn "std::fs\|std::env\|process::Command\|SystemTime\|Instant::now\|std::io" core/src/` 无命中；唯一带 I/O 的 `core/tests/nesting_budget.rs` 在 `core/tests/`（`core/src` 之外，且该测试自己的注释说明它为何必须在那里）。`core/src` 里没有 `mod.rs`，没有第二个 `include!`（`include!` 仅出现在 `host!()` 的 `OUT_DIR/generated_lib.rs`，由约定门禁把守）。
- 干净的实现（我逐个对照了注释与代码，确认自述成立）：`identity/node_id.rs` 与 `identity/sha256.rs`（含 0..=200 全长度差分测试与 `sha2` 预言机、双片段/分隔符按值钉住、`from_str` 的 32 位限制）；`identity/path_text.rs`（`strip_path_prefix` 的组件边界与 separator 折叠）；`lexicon/lexicon.rs`（`path_is_under`/`path_is_strictly_under` 两种含义并存且各有钉子，文本契约 17 条全被 `lexicon_tests.rs:13-31` 钉住，`resolve_package_root` 的三步规则有边界测试）；`json/json.rs`（RFC 8259 转义集完整，控制字符走 `\u00XX`，是工作区唯一编码器并在 `mir/render.rs:6` 与 `diagnostic/build.rs:7` 被复用）；`syntax/nesting.rs`（三种溢出形状 + 加权限深 + 箭头/比较的边界，`deep_input_tests.rs` 与 `nesting_budget.rs` 两侧夹住）；`syntax/face.rs` 的 `parse_faces`/`source_references`（词法扫描器共用一份掩码、KN6 的路径段边界已修）；`source/*` 的掩码与调用扫描（生命周期/字符字面量/raw string/嵌套块注释/多行字面量/非 ASCII 标识符全有钉子，且 `body_calls` 与 `direct_calls` 的一致性有对照测试）；`mir/merge.rs`（`same_symbol` 的 `::` 边界 + live 优先 + 未观测候选保留为 `Mir`）；`mir/call_tree.rs`（布局不变量：列顺序、每列车道唯一、共享被调用者单节点、环终止、两种不完整都有报告）；`plugin/artifact/artifact.rs`（`VerifiedPluginArtifact` 字段私有、`Signature` 只有一个构造入口、撤销先于签名）；`plugin/contracts/signing/signing.rs`（长度前缀编码、29 个字段逐一覆盖、签名自身排除）；`plugin/graft/document.rs`（重复键/未知键/未知版本/`.` 与 `..` 选择器全拒，写读共用一条选择器规则）；`tree/entry_pages`（页级 COW 与 `len` 维护）；`tree/graft_ops/reconcile.rs` 的四种身份/路径组合（干净/漂移/矛盾/未保留）；`tree/transaction::plan_batch` 的环检测与稳定名查重；`requirements::missing_capabilities` 的环安全；`release::assert_static_registration` 与运行期 `validate_registration_requirements` 的五项检查逐项同序（我对着两处各数了一遍）。
- 明确没查的：`core` 之外的 crate 只按“为了判断 core 结论”的范围读（`run_method` 的创作读写路径、`build_method` 的合同检查与面发现、`mcp` 的一处消费），未做片区级审计；`docs/` 里的历史审计文档只在需要对照时扫过标题与结论段，未逐行复核。

## 结构观察（供结构架构师汇总）

1. **单文件已按职责拆到近饱和，`tree/` 的拆分粒度参差。** `tree/` 20 个文件里 `graft_ops/` 一页一责（record/reconcile/apply/reports/overlay/resolution + 两个测试页）非常清楚，但 `registry.rs`（98 行，只放类型与构造）与 `index.rs`/`query.rs`/`metadata.rs` 三页共用一个 `Registry` 的只读面（合计约 400 行），彼此都以 `impl Registry` 开头，读者要读完四页才知道这个类型的公开面有多大。可以考虑按“读/写/构造/统计”四组归并成两页（`registry.rs` = 类型 + 读，`tree_ops.rs` = 写），代价是 `#[path]` 约定下文件数变化，收益是 `Registry` 的公开面集中。
2. **`declaration` 与 `plugin` 的“孪生类型”是最大的结构性重复来源。** `RegistrationInfo`/`RegistrationSnapshot`、`ObjectContract`/`OwnedObjectContract`、`Admission`/`OwnedAdmission`、`RegistrationRule`/`OwnedRegistrationRule`、`FlowContract`/`OwnedFlowContract`、`LocalizedText`/`OwnedLocalizedText`、`SourceLocation`/`OwnedSourceLocation` 共 7 对；本仓已经用“共享一个核心函数 + 两侧只做存储适配”的手段压住了校验逻辑（`declaration.rs:50-171`），但**数据搬移**仍然是一份手写的 40 行列表（`core/src/registry_core/declaration/registration.rs:343-408` 的 `into_snapshot`，与 `core/src/registry_core/declaration/owned.rs:280-315` 的 `merge_authored`），K-02 正是漏在这个列表上。若要继续加固，方向是让“字段清单”本身只有一个来源（宏或 `into_owned`/`merge` 的成对生成），而不是再加注释提醒。
3. **`core/src/test/registry_rule/` 是空目录**（无文件、无引用）；`cargo package --list` 会把空目录丢掉，因此它在包内不存在。若不是有意保留的占位（例如给 `rule_path_for_source` 的路径形状做样例），建议删除；若有意保留，需要一个文件与一行说明，否则下一位维护者会当成遗漏。
4. **execution-surface shim 的边界目前只靠命名约定。** `core` 的模块层级被 `run_method`/`debug_method` 以 shim 再导出（AGENTS.md 规则 2），因此 `core` 里任何一个公开项的重命名都会连带改动执行面；本轮 K-05/K-09/K-10 的修复都落在 `pub` 面上（`resolve_path` 是 `pub(super)`，影响可控；`string_list`/`rule_syntax_from_text` 是 `pub`），建议结构轮把“哪些 core 公开项被 shim 承诺”列成一张表放进 `conventions` 门禁，而不是散在 shim 文件里。
5. **注释与文件尺寸的张力已经影响到结构。** 本片区至少 6 个文件把“为什么拆”写成 15-30 行说明（K-20/C-04），`record.rs`/`ports.rs`/`runtime_checks.rs`/`call_tree.rs`/`release.rs` 都有“测试/fixture 因行数预算另立一页”的记录。这说明 600 行棘轮在生效，但也说明**文件预算正在决定注释与测试的摆放**——结构轮若调整棘轮或拆页规则，应同时给“拆分理由”这类元注释一个公认的落点（如 `docs/` 或统一的一行指针），否则每拆一次都会新增一段平行的历史叙事。
