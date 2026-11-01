# 独立复核：内核 A 路（lane-kernel）发现 — t9

- 复核对象：`docs/audit-2026-09-28/audit-lane-kernel.md`（386 行；作者 = kernel-auditor / t2）。
- 复核人：studio-auditor（t9，attempt `0ebd6bc3-dde3-497e-9957-e52eae102b9c`）。
- 基线：工作树 HEAD=cf0c378，未做任何 checkout/restore/stash/fmt；**源码一行未改**。
- 文件命名：任务书写的是 `verify-kernel.md`，队长广播（后发、且点名本角色的复核文件名）要求 `audit-` 前缀以避开活文档门禁，本报告按队长口径写为 `docs/audit-2026-09-28/audit-verify-kernel.md`，未另建旧名文件。
- 口径：默认怀疑作者。每条结论以我亲手跑出/读到的为准；只靠读代码的显式标注「代码阅读佐证」，不写成「已实测」。

## 0. 复核手段（与作者不同的一套）

作者**没有**落任何探针（其报告第 375 行自述：本轮没有新增测试或探针文件，K 系列复核手段只以“探针命令”形式给出、未实跑）。因此我的手段是造一套真跑的探针：

- 探针工程 `/tmp/nk-probe`（`[workspace]` 独立、`path` 依赖只**读**检出：`xirang = { package = "xirang-core", path = "…/core", features = ["syntax"] }`），产物与 target 全在 `/tmp`，检出目录零写入。
- 构建：`CARGO_TARGET_DIR=/tmp/nk-probe/target cargo build --offline` → exit 0。
- 运行：`/tmp/nk-probe/target/debug/nk-probe all` → exit 0（逐条打印 `[probe] <id> = <实际值>`）；递归类发现另起进程跑（`nk-probe deep <n>`），用退出码观察 abort。
- 夹具是自写的 `RegistrationSnapshot` 构造器（与内核测试夹具同构但独立），不是作者的脚本，也不是既有测试的重跑。

命令与退出码汇总：见 §3。

---

## 1. 一览（结论 + 严重度）

| 原发现 | 我的结论 | 严重度 |
| --- | --- | --- |
| K-01 CRITICAL（admission 读回丢 deny） | **证实**（探针 + 读→写整链实测） | 维持 CRITICAL |
| K-02 MAJOR（`merge_authored` 丢 runtime_checks） | **证实** | 维持 MAJOR |
| K-03 MAJOR（validate 通过 / apply 失败 / UnknownTarget） | **证伪机制，部分证实缺陷**（两者同判 Err；真问题是同一条错误文案错指原因） | **下调 MINOR** |
| K-04 MAJOR（overlay 非 full 保留子注册机不查 `needs_registry`） | **证实**（并实测该“叶子”仍可被注册） | 维持 MAJOR |
| K-05 MAJOR（兄弟同名 registry_name 无人拒绝） | **证实** | 维持 MAJOR |
| K-06 MAJOR（畸形 `requires` 被静默吞） | **证实** | 维持 MAJOR |
| K-07 MAJOR（插件锁 schema 门禁按行序/对空锁不生效） | **证实**（三种形态逐一实测） | 维持 MAJOR |
| K-08 MAJOR（模块级 `cfg` 整个跳过） | **证实** | 维持 MAJOR |
| K-09 MAJOR（带逗号泛型实参的 trait 标签派生失败） | **证实** | 维持 MAJOR |
| C-01 MAJOR（admission 模块自述“两种拼法同一个门禁”） | **证实** | 维持 MAJOR |
| C-02 MAJOR（`CallSite` 文档写成了边） | **证实** | 维持 MAJOR |
| C-03 MAJOR（ports 自述“绝不静默解析” vs `resolve_path`） | **证实** | 维持 MAJOR |
| K-10 MINOR | 证实 | 维持 |
| K-11 MINOR | **部分证实**（缺陷成立；作者引用的测试说反了） | 维持 MINOR |
| K-12 MINOR | 证实 | 维持 |
| K-13 MINOR | 证实 | 维持 |
| K-14 MINOR | **部分证实**（行为成立，数字错：实为期望 3 / 当前 2） | 维持 MINOR |
| K-15 MINOR | 证实 | 维持 |
| K-16 MINOR | 部分证实（不变量成立 ⇒ 兜底当前不可达，与作者自述一致） | 维持 MINOR |
| K-17 MINOR | 证实 | 维持 |
| K-18 MINOR | 证实 | 维持 |
| K-19 MINOR | 证实（另半支为代码阅读佐证） | 维持 |
| K-20 MINOR / C-04 | **部分证实**（类别成立，数字错：`Split decision` 4 处、`B3a` 10 处；作者漏了 `reports.rs`） | 维持 MINOR |
| K-22 MINOR | 证实（实测 abort） | **建议上调 MAJOR**（见 §2.11） |
| K-23 MINOR | 证实（注意：作者给的探针文本本身会得到 `None`） | 维持 MINOR |
| K-24 MINOR | 证实 | 维持 |
| C-05/C-06/C-07/C-08/C-10/C-11 | 证实（C-10 我补了第三种语义） | 维持 |

未抽到的 MINOR：K-21（两份 trait 派生）、C-09（字段帮助文本讲错机制）——本轮受时间预算限制未独立复核，**不视为已证实**。

---

## 2. 逐条复核

### 2.1 K-01 CRITICAL — 证实（最强的一条）

- 复核结论：**证实**，且我把作者只描述到一半的链路跑通了：读（`parse_admission_expression`）→ 写回（`render_admission`）→ 门禁放宽，三跳全部实测。
- 证据（命令 `/tmp/nk-probe/target/debug/nk-probe K01`，exit 0；构建见 §3）：

```text
[probe] K01.parse_admission_expression = Ok("allow:ui")
[probe] K01.original.accepts(ui/experimental) = false
[probe] K01.roundtrip_allow_only.accepts(ui/experimental) = true
[probe] K01.render_admission(parsed) = Ok("crate::Admission::new(&[\"ui\"], &[])")
[probe] K01.render_admission(allow:ui) = Ok("crate::Admission::new(&[\"ui\"], &[])")
```

  即：`crate::Admission::new(&["ui"], &["ui/experimental"])` → 读成 `allow:ui` → 写回 `crate::Admission::new(&["ui"], &[])`；原始门禁拒绝 `ui/experimental`，写回后的门禁接受它。作者对位置（`core/src/registry_core/authoring/parse/admission.rs:40-44`）、判据（`Admission::accepts` 的 deny 优先）与后果（不可逆放宽）的描述全部成立。
- 严重度：**维持 CRITICAL**。
- 备注（作者漏的一点，直接解释它为何活得下来）：内核自己的往返测试 `admission_round_trips_through_the_constructor_form`（`core/src/registry_core/authoring/parse/admission.rs:174-189`）只覆盖 allow-only / deny-only / ANY 三种形态，**恰好没有两列表同时非空的那一种**；而紧凑语法本身也没有承载两张列表的写法，所以这不是单点笔误而是“一种拼法缺一个语法”。作者的修复方向（扩展紧凑语法，或两列表非空即 `Err`）是唯一两条路，我同意。

### 2.2 K-02 MAJOR — 证实

- 复核结论：**证实**。
- 证据（同一支探针，`/tmp/nk-probe/target/debug/nk-probe K02`）：`[probe] K02.merged.runtime_checks = [FiniteNumber]`——编译期 `[FiniteNumber]` + 作者侧 `[NonEmptyText]` 经 `merge_authored` 后仍是前者。
- 作者所述的注入点也核对了：`merge_authored`（`core/src/registry_core/declaration/owned.rs:280-315`）在 19 个字段逐项搬移后不碰 `runtime_checks`；唯一调用点 `run_method/authoring/operations/operations.rs:352`（`existing.clone().merge_authored(authored)`）随后把合并结果送去 `validate_snapshot_replacement`——即被丢弃的值连校验都不到（代码阅读佐证）。
- 严重度：**维持 MAJOR**。
- 备注：作者的对照（`flow` 有“作者侧非空即应用”）成立，我在 `owned.rs:300-315` 读到同一处。

### 2.3 K-03 MAJOR — 机制证伪，缺陷部分证实（严重度下调）

- 复核结论：**证伪作者的机制叙述**（“`validate_*` 说通过、`apply_*` 却失败、错误是 `UnknownTarget`”），**部分证实其残余缺陷**（存在把“不能降级非空子注册机”报成“面已不存在”的错误文案）。
- 证据 1（`nk-probe K03`，exit 0）：带一个子面的 owner 降级为 `needs_registry: false` 时——

```text
[probe] K03.debug.find(owner_id).is_some = true
[probe] K03.validate_snapshot_replacement = Err(RegistryError { … message: "edited face is no longer present in the registry" … })
[probe] K03.apply_snapshot_replacement = … +-- error: edited face is no longer present in the registry
```

  两条路径**同判**：`validate` 返回 `Err`，`apply` 返回的是 `validate` 的这个错误（因为 `apply` 第一句就是 `self.validate_snapshot_replacement(current, info.clone())?`）。没有任何“校验通过而提交失败”的窗口。
- 证据 2（控制组，同一支探针）：owner 拥有**空**子注册机时——

```text
[probe] K03.validate(owner_with_no_children) = Ok
[probe] K03.apply(owner_with_no_children) = Ok
```

  证明我观察到的 `Err` 来自“保留非空子注册机”这一条，不是身份/父级/命名空间之类的其它前置检查。
- 作者错在哪：`validate_snapshot_replacement` 内**也**调用 `replace_info`（`core/src/registry_core/tree/graft_ops/graft_ops.rs:196-205` 的 staged 分支），因此 `replace_info` 返回 `false` 时两条路径都报同一条消息；`apply` 里那个 `GraftError::UnknownTarget(current)` 兜底（`graft_ops.rs:222-224`）在 `validate` 已通过的条件下不可达（同一输入、同一 staged 状态，纯函数必然再返回 `true`）。作者把“校验闸门”与“提交”当作两套判定，实际是同一套判定被复用了同一条错误字符串。
- 真实缺陷（保留下来的那部分）：`graft_ops.rs:110` 与 `graft_ops.rs:203`（同一句“edited face is no longer present in the registry”）被两个不同原因共用——(A) 目标确实不在了、(B) 目标在、但保留非空子注册机会把叶子声明变成父级。文件创作路径正以该校验作写盘闸门（作者的这条引用成立），所以作者看到的是“面不存在”，会被引去查身份/父级，而真正要做的是先移走/清空子注册机。
- 严重度：**下调 MAJOR → MINOR**。理由：写盘被正确拦下（无静默放宽、无树损坏），剩下的是一条会误导的诊断文本 + 一条消息两个原因。
- 备注：这与作者自己的判据“校验与提交不同判等于校验结果不可信”不符——它们同判。建议把这条改写为“错误文案错指原因”，最小修复方向也随之收敛为：给 `replace_info` 的 `false` 加一个区分原因（或让 `replace_info` 返回 `Result`），两条路径各自报准确原因。

### 2.4 K-04 MAJOR — 证实

- 复核结论：**证实**，而且我把作者只推理到的后果实跑了：非 full 嫁接确实能留下“声明为叶子、实际仍拥有子注册机”的节点，且该子注册机**仍可被注册**。
- 证据（`nk-probe K04`，exit 0）：基座 owner（`needs_registry: true`）+ 一个子面；外部替换件（`needs_registry: false`、flow 已声明且兼容）——

```text
[probe] K04.overlay = Ok
[probe] K04.effective.registry(owner).is_some = true
[probe] K04.register_into_leaf_owner = Ok
```

  第三行是关键：向这个“叶子”注册一个新的孙面**成功**，即 `needs_registry: false` 的节点仍能在树上长出子级。
- 代码侧对照（代码阅读佐证）：`core/src/registry_core/tree/graft_ops/overlay.rs:258-309` 的非 full 分支只做 `registry.reconfigure(&candidate)` 与 `child_violating(...)`，从不读 `candidate.needs_registry`；就地替换路径对同一状态明确拒绝（`core/src/registry_core/tree/graft_ops/graft_ops.rs:261-264`）。
- 作者所指的测试缺口也复核了：`a_non_full_graft_does_not_install_a_rule_its_children_violate`（`overlay.rs:329-362`）的替换件确实是 `needs_registry: true`（`core/src/registry_core/tree/graft_ops/overlay.rs:343`），因此没覆盖这条边界。
- 严重度：**维持 MAJOR**。

### 2.5 K-05 MAJOR — 证实

- 复核结论：**证实**。
- 证据（`nk-probe K05`，exit 0）：同一父级下两个 `id` 不同、`registry_name` 都是 `a` 的快照——

```text
[probe] K05.register_two_siblings_same_slot = Ok
[probe] K05.path_for(first) == path_for(second) = Some("root/a") == Some("root/a") -> true
[probe] K05.node_count = 2
```

  注册通过、两条路径逐字节相同（`root/a`），`path_for` 无法区分。作者的“三处静默选择”里 `resolve_path` 取第一个匹配也已核对（`core/src/registry_core/tree/graft_ops/resolution.rs:22-28` 用 `find_map`），而同一模块 `resolve_node`（`resolution.rs:146-174`）对多匹配返回 `Resolution::Ambiguous`——同一文件两种严格度。
- 严重度：**维持 MAJOR**。
- 备注：`RegistryIndex::by_path` 是 `pub(super)`，我无法从外部探针直接数它的长度（作者的探针若断言了 `index().by_path.len()` 需要 crate 内可见性）；这不影响结论——`path_for` 相等本身就足以证明事实键碰撞。

### 2.6 K-06 MAJOR — 证实

- 复核结论：**证实**。
- 证据（`nk-probe K06`，exit 0）：

```text
[probe] K06.parse_requirements_owned(cap).len = 0
[probe] K06.parse_requirements(cap) = Err: requires entries must use capability=>provider syntax
```

  同一输入：静默版给空列表，严格版给 `Err`。接线也核对了：`core/src/registry_core/authoring/snapshot/snapshot.rs:116` 的 `requires: parse_requirements_owned(value("requires"))` 正是静默版（代码阅读佐证）。
- 严重度：**维持 MAJOR**。

### 2.7 K-07 MAJOR — 证实（三种形态逐一实测）

- 复核结论：**证实**，作者列出的三条全部命中。
- 证据（`nk-probe K07`，exit 0）：

```text
[probe] K07.parse(header_only) = Ok(0 records)
[probe] K07.parse(record_before_header) = Ok(1 records)
[probe] K07.parse(duplicate_headers) = Ok(0 records)
```

  ①只有表头的锁不报错；②表头在记录之后时记录照样解析（新/旧 schema 被当成本版本可读）；③重复表头静默覆盖（不报错）。三条都是**放行**方向，与 `PluginCatalog::parse` 自己的文档“refusing unknown schemas”（`core/src/registry_core/plugin/catalog/catalog.rs:120`）相反。
- 严重度：**维持 MAJOR**。
- 备注：作者没提的第四条同类（我另记为 §4 X-2）：**表头拼错就是注释**——`# xirang-schema v2`（缺 `=`）被 `catalog.rs:135` 的“`#` 开头即注释”吞掉，闸门完全不生效。

### 2.8 K-08 MAJOR — 证实

- 复核结论：**证实**。
- 证据（`nk-probe K08`，exit 0）：

```text
[probe] K08.graft_entries(cfg_mod) = Ok(0 entries)
[probe] K08.graft_entries(no_cfg_mod) = Ok(1 entries)
```

  同一个模块，只加一条 `#[cfg(feature = "on")]`，静态 graft 表从 1 条变 0 条。作者所述的对照（条目级 cfg 仍被收集）也复核过：`graft_parser_ignores_declarations_that_are_not_items`（`core/src/registry_core/syntax/entries/graft.rs:408-435`）里 `#[cfg(feature = "optional-graft")]` 写在条目上时 `entries.len() == 1` 且 `cfg` 被记录。
- 严重度：**维持 MAJOR**。

### 2.9 K-09 MAJOR — 证实

- 复核结论：**证实**。
- 证据（`nk-probe K09`，exit 0）：面源码 `crate::control_object! { kind: Tool, handle_contracts: [crate::ControlHandle<u8, u16>] }`——

```text
[probe] K09.path_list(handle_contracts) = None
[probe] K09.string_list(handle_traits) = None
```

  带逗号的泛型实参让 `path_list` 直接 `None`（`syn::parse2::<syn::Path>` 在两段碎片上都失败 → `collect::<Result<_>>().ok()` 吞掉），`string_list("handle_traits")` 因没有显式标签而同样 `None`；下游 `unwrap_or_default()` 会把它们变成空列表（`build_method/contracts.rs` 与 `run_method/authoring/manifest/parse/parse.rs:141-146`，代码阅读佐证）。
- 严重度：**维持 MAJOR**。
- 备注（探针写法提醒）：注册宏必须是**条目位置**（`syn::ItemMacro`）才会被访问；写在 `fn f() { … }` 里会得到 `Ok(None)` 而掩盖结论。作者报告第 127 行的探针文本与 K-23 的探针文本都写成函数内嵌，**幸好结论方向对**。

### 2.10 C-01 / C-02 / C-03（注释轴的 3 条 MAJOR） — 全部证实

- **C-01 证实**：`core/src/registry_core/authoring/parse/admission.rs:5-8` 逐字写着“Both spellings describe the same gate, so both are read here.”，而 §2.1 的实测显示两种拼法描述的不是同一道门禁（一张列表消失）。
- **C-02 证实**：`core/src/registry_core/declaration/call_evidence.rs:6-7`（`CallSite`）与 `:31-32`（`CallEdge`）是**逐字相同**的一句“One call edge that was observed while a `CallTrace` frame was active.”；而 `CallSite` 的字段是 `node`/`function`/`frame_id`/`source`，是一处“调用点”。
- **C-03 证实**：`core/src/registry_core/tree/ports/ports.rs:29-33` 写“a handle or a port that matches more than one face is **reported, never resolved silently**”，同仓 `resolution.rs:22-28` 的 `resolve_path` 用 `find_map` 取第一个匹配（静默解析），而 `resolve_node`（`resolution.rs:146-174`）返回 `Ambiguous`。同一份文档在两条解析入口上只有一条兑现。
- 严重度：三条均**维持 MAJOR**（C-01/C-03 的严重度完全跟随其实现缺陷；C-02 是公开文档面的错误陈述）。

### 2.11 K-22 MINOR — 证实（建议上调）

- 复核结论：**证实**（不是理论上的递归风险，是实测的进程级 abort）。
- 证据 1：`/tmp/nk-probe/target/debug/nk-probe deep 20000` → **exit 134**，stderr：

```text
thread '<unknown>' (396289) has overflowed its stack
fatal runtime error: stack overflow, aborting
```

  该进程调用的是公开 API `xirang::mir::call_tree("f0", &chain_of(20_000), 20_001, 20_001)`（链上符号互不相同），跑在一个 256 KiB 栈的线程上；`count=200000` 同样以 abort 结束。作者预测的 20 万层不需要，**2 万层就够**。
- 证据 2（实现侧，代码阅读佐证）：`core/src/registry_core/mir/call_tree.rs:373-405` 的 `assign_lane` 沿父子链递归；`:152-153` 只对 `limit` 做 `max(1)`，`depth` 无上界。
- 严重度：**建议上调 MINOR → MAJOR**。理由：`call_tree` 是公开 API，`depth` 是调用方可控参数且文档未给上界；abort 不可捕获，与内核自己“栈溢出不是 `Result`”的立场（`core/src/registry_core/syntax/nesting.rs`）冲突。若维护者认定 `depth` 只作内部约定（本仓调用方 Studio 传 2），则应补一句文档上界并保持 MINOR——两条路都要求一个显式决定，现状是两者皆无。
- 备注：作者写的“在小栈线程上期望不 abort”是可执行的复核手段，我按它跑通了。

### 2.12 MINOR 抽样（≥1/3：23 条抽 14 条）

| id | 我的探针输出（`nk-probe all`，exit 0） | 结论 |
| --- | --- | --- |
| K-10 | `K10.rule_syntax_from_text = exports:wrong` | 证实（注释里的伪子句被当真） |
| K-11 | `K11.verify(official, host_fingerprint=None) = Ok` | **部分证实**：缺陷成立（指纹可由工件自述）；作者引用说反了（见 §4） |
| K-12 | `K12.open.decision(no_signature_official) = Accepted` | 证实（测试 `plugin_policy.rs:206-210` 确实这么断言） |
| K-13 | `K13.contains_manifest(prefixed_lock, bare_manifest) = false` | 证实（同一摘要两种拼法，锁匹配只认字面相等） |
| K-14 | `K14.split_face_fields.count = 2`；`K14.baseline_without_comparison.count = 3` | **部分证实**：吞字段成立，数字错（见 §4） |
| K-15 | `K15.from_jsonl(glued_records) = Ok(functions=["a"], calls=0)` | 证实（第二条记录 `b` 消失且无错） |
| K-16 | `K16.mask_length_and_utf8_invariant = true` | 部分证实：不变量在我的样本上恒成立 ⇒ 兜底当前不可达，与作者自述一致；这条是“未来可能变坏”的防线 |
| K-17 | `K17.registration_kinds(let kind: String) = ["String"]` | 证实（唯一调用方 `mcp/tools.rs:344` 已核对） |
| K-18 | `K18.duplicate_id_diagnostics = 0` | 证实（重复身份在静态拓扑检查里零诊断） |
| K-19 | `K19.source_file_matches(mixed_case_needle) = false` | 证实（另半支 `is_face_source` 为代码阅读佐证：`syntax/face.rs:169-171` 的标记行短路 + 调用点 `run_method/authoring/operations/operations.rs:293`） |
| K-20 / C-04 | `grep "Split decision" core/src = 4`；`grep "B3a" core/src = 10` | **部分证实**：类别成立（4 处同义说明、10 处历史叙事），数字错（见 §4） |
| K-23 | `K23.parse_face(widget_object) = Ok(Some(macro=widget_object, kind=None))` | 证实（非 XiRang 的 `*_object` 宏被当成注册面） |
| K-24 | `grep -c leak_code = 2`，测试体四条断言全为否定（`None` / `is_empty`） | 证实（名字与断言相反） |
| C-05..C-11 | 见下 | 证实 |

- **C-05**：`core/src/registry_core/tree/registry.rs:1` 与 `core/src/registry_core/diagnostic/topology.rs:4` 确实用 `//` 开头（同层别的模块用 `//!`）。
- **C-06**：`grep -c '# Panics' core/src/registry_core/release/release.rs = 2`，且第一段（`release.rs:261-277`）讲的是“const 孪生”而不是 panic 契约。**但要修正作者的判据**：承载这两段的 `assert_static_registration` 带 `#[doc(hidden)]`（`core/src/registry_core/release/release.rs:290`），所以“docs.rs 主读者会按标题检索”这半句不成立；受影响的读者是**源码读者**。结论仍是 MINOR（同一标题下两件事）。
- **C-07**：`core/src/registry_core/declaration/runtime_checks_tests.rs` 的 `//!` 段在 `:1-19` 与 `:24-44` 重复，后者多出 `health_check` 与测试名补充。证实。
- **C-08**：`core/src/registry_core/plugin/trust/trust.rs:86-93` 的 `verify` 文档只有一句，三个参数与 `key_fingerprint` 为 `None` 的回退（`:117-119`）都没有文档。证实。
- **C-10**：证实**并且比作者说的更强**——`mir_line` 有三种语义：JSONL 解析器用工件给的字段值（`core/src/registry_core/mir/jsonl.rs:64`，缺省值是该 JSONL 记录所在行）、文本解析器填合成序号（`core/src/registry_core/mir/text.rs:43` 的 `graph.calls.len() + 1`）、字段名却读作“MIR 行号”。
- **C-11**：`grep -rn 'branch handle\|tree slot\|logical slot\|slot name' core/src = 12`，术语一名多指成立。

---

## 3. 命令与退出码（可复现）

| # | 命令 | exit | 关键输出 |
| --- | --- | --- | --- |
| 1 | `CARGO_TARGET_DIR=/tmp/nk-probe/target cargo build --offline`（in `/tmp/nk-probe`） | 0 | `Finished dev profile` |
| 2 | `/tmp/nk-probe/target/debug/nk-probe all` | 0 | §2 引用的全部 `[probe] …` 行 |
| 3 | `/tmp/nk-probe/target/debug/nk-probe K01` | 0 | `K01.render_admission(parsed) = Ok("crate::Admission::new(&[\"ui\"], &[])")` |
| 4 | `/tmp/nk-probe/target/debug/nk-probe K03` | 0 | `validate … Err(… "edited face is no longer present in the registry")`；`validate(owner_with_no_children) = Ok`；`apply(owner_with_no_children) = Ok` |
| 5 | `/tmp/nk-probe/target/debug/nk-probe deep 20000` | **134** | `has overflowed its stack` / `fatal runtime error: stack overflow, aborting` |
| 6 | `grep -rn "Split decision" core/src \| wc -l` / `grep -rn "B3a" core/src \| wc -l` | 0 / 0 | `4` / `10` |
| 7 | `grep -c '# Panics' core/src/registry_core/release/release.rs` | 0 | `2` |
| 8 | `grep -rn 'branch handle\|tree slot\|logical slot\|slot name' core/src \| wc -l` | 0 | `12` |
| 9 | `grep -rn 'std::fs\|std::env\|process::Command\|SystemTime\|Instant::now\|std::io' core/src/` | 0 | **2 行，均为文档注释**（见 §4），作者“无命中”的措辞需修正 |

检出目录零写入：`git status --porcelain` 中 `core/`、`run_method/`、`build_method/` 的条目与本轮开始时逐项一致；探针与其 target 全在 `/tmp/nk-probe`。

---

## 4. 作者报告本身的准确性（数字、引用、探针文本）

按“报告与代码不符时以我读到/跑到的为准”逐项列出——这些不改变发现的类别，但下游（t15 逻辑总账、修复清单）若照抄数字会出错：

1. **K-03 的机制叙述错**（最要紧的一条）：不存在“`validate` 通过 / `apply` 失败 / `UnknownTarget`”。实际是两条路径同判 `Err`，共用同一条错误文案。严重度应下调到 MINOR。（§2.3）
2. **K-11 引用的测试说反了**：作者写“`trust.rs` 的测试正是这么用的：`policy.verify(manifest, b"abc", None)` 配一个自带指纹的 manifest 得到 `Ok`”。实际 `core/src/registry_core/plugin/trust/trust.rs:277-286` 的用例传 `None` 时把 `public_key_fingerprint` 也清成 `None`，断言的是 **`Err(MissingOfficialKey)`**。缺陷本身我用探针证实了（自述指纹 + 宿主 `None` → `Ok`），但“既有测试覆盖了它 / 它被当成正常”这句是错的：**现有测试恰好没覆盖自述指纹这条路**。
3. **K-14 的数字错**：作者写“期望 4 个字段，当前得到 3 个”。实测：带比较运算符 `<` 时是 **2** 个（`needs_registry` 被并进 `flow`），去掉 `<` 的基线是 **3** 个。即“期望 3 / 当前 2”。
4. **K-20 / C-04 的数字错，且漏了一处**：作者写“`grep "Split decision"` 3 处、`grep "B3a"` 3 处”。实测 `core/src` 是 **4 处 / 10 处**；漏掉的那处是 `core/src/registry_core/tree/graft_ops/reports.rs:15`。性质不变（甚至更严重），但计数需以本报告为准。
5. **F 铁律的措辞错**：作者写“`grep -rn "std::fs\|std::env\|process::Command\|SystemTime\|Instant::now\|std::io" core/src/` 无命中”。实测 **2 处命中**，都在 `core/src/registry_core/source/source.rs:260/265` 的文档注释里（讲“`include!`/`std::fs` 不被读成违规”）。**结论（`core/src` 无真实 I/O）成立**，但“无命中”应写成“命中 2 处，均为文档注释”。
6. **K-23 的探针文本会自证错**：`parse_face("fn f() { widget_object! { … } }")` 里宏在函数体内，`FaceVisit` 只访问条目级宏（`syntax/face.rs:246-258` 的 `visit_item_macro`），因此无论宏名如何都会返回 `Ok(None)`——作者的“期望 `Ok(None)`，当前 `Ok(Some(…))`”若照原文本跑会得到 `Ok(None)`。我把宏提到条目位置后复现了作者的真实结论（`Ok(Some(widget_object))`）。K-09 的探针文本同理。
7. **K-22 的量级偏大**：不需要 20 万层；256 KiB 栈上 2 万层即 abort。结论方向不变。

作者写对且我逐字复核的地方：K-01 的位置/链路/后果、K-02 的注入点与对照、K-04 的测试缺口引用、K-05 的三处静默选择与 `resolve_node` 对照、K-06 的接线、K-07 的三条形态、K-08 的条目级对照、K-09 的 `syn::Path` 失败链、C-01/C-02/C-03 的三处注释原文、K-15/K-17/K-18/K-19/K-24 的结论。

---

## 5. 跨片提示复核

- **`flow_provider` 缺 `guard_nesting`（作者列为最要紧的跨片提示）——证实。** `run_method/authoring/manifest/face/face.rs:140-143` 直接 `syn::parse_str::<syn::Path>(value)` 且没有守卫；内核同名渲染入口 `core/src/registry_core/authoring/parse/flow.rs:107` 之前有 `guard_nesting(value)`，其注释明确写着“没有这道守卫时，一条 542 字节的类型路径会以栈溢出 abort 进程”。同一份不可信值（清单/编辑器）在两条入口上一条有守卫一条没有——这不是风格问题，是把内核已经付过代价的教训漏在了执行面。**建议在最终报告里保留并升级为独立条目**（我这一路没有执行面片区的复核权限，只做交叉确认）。
- `build_method/contracts.rs` 与 `run_method/authoring/manifest/parse/parse.rs:141-146` 用 `unwrap_or_default()` 消费 `FaceSyntax::string_list`：确认（代码阅读佐证），这正是 K-09 的 `None` 会退化成“空标签列表”的链路。

---

## 6. 我顺手发现的同类问题（作者未列）

### X-1 · MINOR · 逻辑/静默等价 · `core/src/registry_core/plugin/catalog/catalog.rs:179-190`

- 现象：10 字段记录里的三个来源字段可以为空串，`fields.get(7..9).filter(|value| !value.is_empty())` 把空串变成 `None`：一条写坏的 10 字段记录被**当成 7 字段**接受。
- 实测（`nk-probe X1`，exit 0）：输入 `user|fw|p|1|c|sha256:b|extension|||`（10 字段）→ `record accepted; signature=None fingerprint=None revocation=None`。
- 判据：与 K-07 同一类——解析器的立场是“读不懂就拒绝”，但“字段在、值为空”被读成“字段没写”。对 `accounts_for`（`catalog.rs:281-301`）而言，这等于把记录侧的“期望”静默降低，方向是放行。
- 最小修复方向：10 字段形态下三个来源字段各要求非空（或与 7 字段形态用不同的构造入口区分），拒绝“空字段”而不是折叠。
- 复核手段：`PluginCatalog::parse(<10-field line with empty provenance>)` 期望 `Err`。

### X-2 · MINOR · 逻辑/静默放过 · `core/src/registry_core/plugin/catalog/catalog.rs:131-137`

- 现象：schema 表头只认逐字 `# xirang-schema=`；拼错（`# xirang-schema v2`、`# xirang_schema=v2`）落进“`#` 开头即注释”的分支被丢掉，于是**整道 schema 闸门静默失效**（正是 K-07 的同一后果，入口不同）。
- 实测（`nk-probe X2`，exit 0）：`# xirang-schema v2\nuser|fw|p|1|c|sha256:b|extension\n` → `Ok(1 records, no schema error)`。
- 判据：一个版本闸门的失效方式应当是拒绝而不是沉默；本仓对“不认识的键”在同一 crate 的其它解析器里都是拒绝（如 graft 文档解析）。
- 最小修复方向：把 `#` 注释收窄为已知键白名单（其余 `#` 行若形似表头则报错），或要求表头必须出现（缺表头即 `Err`）。
- 复核手段：`PluginCatalog::parse("<typo'd header line>\n<record>")` 期望 `Err`。

### X-3 · 补充（不是新发现，是 K-10 的加强）· `mir_line` 的第三种语义

见 §2.12 的 C-10：JSONL 缺省值使 `mir_line` 还能等于“该 JSONL 记录所在行号”。修复时若只在文本解析器里改语义，JSONL 侧的名字仍然在说谎。

---

## 7. 复核口径与未覆盖项

- 覆盖：CRITICAL 1/1、MAJOR 11/11（K-02…K-09 + C-01…C-03）全部独立复核；MINOR 14/23（≥1/3 要求）。
- 未独立复核的 MINOR：K-21（两份 trait 派生 `derived_trait_names` vs `trait_names_from_paths`）、C-09（`RUNTIME_CHECKS` 帮助文本 vs `health_check`），以及作者「查了、干净」里除 F 铁律与空目录之外的逐项清单（那部分是作者的正面确认，我按抽样风险原则只做了 F 铁律与两条结构观察的抽查）。
- 我未复核作者的结构观察 5 条与「查了、干净」的覆盖面声明（属结构架构师与队长对账范围）。
- 本报告中凡标「代码阅读佐证」的结论都只靠 `read`/`grep`，没有运行证据；其余均带命令与退出码。
