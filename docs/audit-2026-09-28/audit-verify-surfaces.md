# 独立复核 t10：执行面 B 路（`audit-lane-surfaces.md`）

复核人：gates-auditor。工作树 `HEAD=cf0c378` + 未提交改动。**源码一行未改**；本文件是本次唯一的写入。
文件命名：任务书写的路径是 `docs/audit-2026-09-28/verify-surfaces.md`，但队长广播已把"本轮报告文件名必须以 `audit-` 开头"定为硬规则（同目录 peer 也是 `audit-verify-studio.md`），因此本报告落在 **`docs/audit-2026-09-28/audit-verify-surfaces.md`**，不另建非 `audit-` 前缀文件。

## 0. 装置（与作者不同、也不重跑作者的脚本）

我自己写了一个探针工程 `/tmp/vs-probe`（工作区之外），只通过**公开 API** 与工作区自己的依赖来驱动被测代码，不 include、不复制作者的任何探针代码：

```toml
[dependencies]
xirang-build-method = { path = "/home/nich/Moirai_N3/nichlink/build_method" }
xirang-core = { path = "/home/nich/Moirai_N3/nichlink/core" }
syn = { version = "2", features = ["full", "parsing"] }
```

- 子命令：`face_path <root> <package>`（公开 `build_method::face_views`）、`run_for <root> <out_dir> <package>`（公开 `build_method::run_for`）、`syn <depth> <stack_bytes>`（裸 `syn::parse_str::<syn::Path>`，即 `run_method/authoring/manifest/face/face.rs:141` 那一行的形状）、`syn_meta <depth> <stack_bytes>`（裸 `syn::parse_str::<syn::Meta>`，`static_plan.rs:158` 的形状）、`guard <depth>` / `guarded_syn <depth>`（内核公开的 `xirang::registry_core::syntax::guard_nesting`，先守后解）。
- 版本对齐：工作区 `Cargo.lock` 的 `syn = 2.0.119`；探针解析到的也是 `2.0.119`（`/tmp/vs-probe/Cargo.lock`），因此 syn 这一层不是版本差异来源。
- 作者用 `/tmp/xirang-probe-surfaces/probe_syn`；我**没有**运行它、没有读它的源码，只用自建探针复现同一行的调用。
- `cargo` 一律 `--offline`；探针 `cargo build` 输出 `Finished dev profile ... in 16.77s`（exit 0）。

关于我引用的行：所有 `x.rs:NNN` 都是我这一轮用 `nl -ba`/`read` 亲自打印过的行；引用时一律写全路径或唯一文件名，不做 `token`（`x.rs:line`）这种配对写法。

---

## 1. 逐条结论总表

| 原 id | 严重度（作者） | 我的结论 | 严重度是否改变 |
| --- | --- | --- | --- |
| S1 | MAJOR | **证实**（运行时反例） | 不变 |
| S2 | MAJOR | **证实**（运行时 + 注释自相矛盾） | 不变 |
| S3 | MAJOR | **证实**（表述需精确化） | 不变 |
| S4 | MAJOR | **部分证实**：两处实现与内核消费者成立；"树内读者为零"**被证伪** | 不变（理由换成"CI 工具在读"） |
| S5 | MAJOR | **证实**（并补：该文件 0 条测试） | 不变 |
| S6 | MAJOR | **证实**（机械计数逐项对上） | 不变 |
| S7 | MAJOR | **证实**（站点清单 5 → 10，含内核公共字段） | 不变 |
| S8 | MAJOR | **证实**（但作者的复核手段不可从 crate 外调用） | 不变 |
| S9 | MAJOR | **证实** | 不变 |
| S19 | MAJOR | **证实**（我的装置；阈值比作者报的**更低**） | 不变 |
| C1 | MAJOR | **证实** | 不变 |
| C2 | MAJOR | **证实** | 不变 |
| C3 | MAJOR | **证实**（= S9 的注释面） | 不变 |
| C4 | MAJOR | **证实**（+ 运行既有测试绿） | 不变 |
| C5 | MAJOR | **证实** | 不变 |
| S10 | MINOR | 证实 | 不变 |
| S11 | MINOR | 证实 | 不变 |
| S12 | MINOR | 证实（文档两半都在，前者是空头承诺） | 不变 |
| S13 | MINOR | 证实（含"读环境"那一半） | 不变 |
| S14 | MINOR | 证实 | 不变 |
| S15 | MINOR | 证实 | 不变 |
| S16 | MINOR | **部分证实**："16 处"这个数字**不可复现** | 不变（但描述要改成抽样） |
| S17 | MINOR | 证实（实测 18 个空格） | 不变 |
| S18 | MINOR | 证实 | 不变 |
| C6 | MINOR | 证实（数字：注释 27/28/29 vs 实际 22/20/24/23/26） | 不变 |
| C8 | MINOR | 证实 | 不变 |
| C7 | MINOR | 未复核（纯阅读面，抽样之外） | — |
| C9 | MINOR | 未复核（意见性清单，抽样之外） | — |

抽样比：MINOR 共 13 条（S10-S18 九条 + C6-C9 四条），我实测/实读 10 条 = 77%，超过 1/3 的要求。

---

## 2. MAJOR 逐条

### S1 · FaceView.path 顺序依赖 → **证实**（运行时反例）

- 证据（我的探针，公开 API）：
  ```text
  $ /tmp/vs-probe/target/debug/vs-probe face_path /tmp/vs-fixA host        # exit 0
  face module=alpha registry_name=alpha path=root/alpha parent=984b158a6dc169ac7cd3fddc5bc070ef parent_resolved=true
  face module=zeta  registry_name=zeta  path=root/zeta  parent=c2efcfaf45e72660deb812640c13d4b0 parent_resolved=true
  $ /tmp/vs-probe/target/debug/vs-probe face_path /tmp/vs-fixB host        # exit 0
  face module=aaa   registry_name=aaa   path=root/aaa   parent=c2efcfaf45e72660deb812640c13d4b0 parent_resolved=true
  face module=alpha registry_name=alpha path=root/aaa/alpha parent=74c64a8606105b39242b9be5886c5521 parent_resolved=true
  ```
  夹具 A/B 的唯一差别是**目录名**（A：子面目录 `alpha` 排在父面目录 `zeta` 之前；B：父面目录 `aaa` 排在子面之前）。A 里子面的 `parent` 指向 zeta 的 id（`984b158a…`，不是根 id `c2efcfaf…`）、`parent_resolved=true`，路径却少了父段（`root/alpha`）；B 里同一关系的路径是 `root/aaa/alpha`。**同一个逻辑结构只因命名顺序不同就给出不同路径**——A 的路径正是作者说的"少一段并被冻结"。
- 代码阅读佐证：`build_method/face_view.rs:280-298` 的 `logical_path` 只查一层缓存（`:291-294` 查不到父就回落 `"root"`，`:296` 写入缓存），而调用点 `build_method/face_view.rs:158-171` 是在 `resolved`（发现顺序）的 map 里逐个求值，最后 `:172` 才排序——排序在求值之后，救不回来。运行期对应物 `core/src/registry_core/tree/query/query.rs:86-96` 是递归求值（我读过，确实按活树逐级拼）。
- 备注：作者给的最小反例（`src/alpha` 指 `src/zeta`）与我的夹具 A 同形，**逐字复现**。作者漏说的一点：`logical_path` 的文档（`build_method/face_view.rs:272-279`）自称 "walking the resolved parent chain"，而实现只走一层——这处 doc/实现不符不在他们的 C 节里（见第 4 节）。

### S2 · 凭据先于载荷发布 → **证实**（运行时 + 注释自相矛盾）

- 证据（我的探针，公开 API `run_for`，两次运行同一份源码）：
  ```text
  $ cp -R examples/control-button /tmp/vs-host && printf '\n// probe\n' >> /tmp/vs-host/src/lib.rs
  $ vs-probe run_for /tmp/vs-host /tmp/vs-clean control_button      # exit 0
  $ mkdir -p /tmp/vs-broken2/pruning_manifest.tsv                   # 让一份载荷写不成
  $ vs-probe run_for /tmp/vs-host /tmp/vs-broken2 control_button
  run_for=Err(NICHLink BUILD CHECK FAILED ... `-- cannot write /tmp/vs-broken2/pruning_manifest.tsv: Is a directory (os error 21))
  exit=7
  $ cmp /tmp/vs-clean/discovery.fingerprint /tmp/vs-broken2/discovery.fingerprint && echo equal
  equal      # 失败的那次运行发布了「当前源码」的凭据
  $ ls /tmp/vs-broken2
  discovery.fingerprint  function_manifest.tsv  generated_lib.rs  graft_plan.tsv  pruning_manifest.tsv(目录)  source_scope.tsv
  ```
  即：一次**报错**的运行，指纹与干净运行的指纹逐字节相同（= 描述当前源码），而 `pruning_manifest.tsv` 根本不是数据文件、其余载荷是新写的 → 混代产物 + 一枚"当前"凭据。读取方 `build_method/scope_view.rs:39-48` 只做 `stored.trim() == discovery_fingerprint(...)`，因此它会对这棵混代树返回 true。
- 代码阅读佐证（确认"顺序"这一半）：`build_method/pipeline.rs:155-161` 在 `compile_errors.is_empty()` 时写指纹；`:174-184` 才写五份载荷并把失败收进 `write_errors`；`:185` 才检查 `write_errors`。而 `:147-154` 的注释承诺"只有干净的一次运行才写下它……A failed run publishes no token at all"——注释说的"干净"在代码里只等价于"没有校验错误"，**注释与实现不一致**，这是本条最硬的部分。删指纹那一支（`:162-164`）是 `let _ =`，也在注释承诺的反方向。
- 严重度：不变（MAJOR）。作者对后果的描述我核对无误；他们说的"结构化调用方拿到一条 out-dir 诊断 + 退出"也成立（我的 `run_for` 返回的正是这条诊断）。

### S3 · 剪枝清单符号列硬编码 → **证实**（表述需精确化）

- 证据：`nl -ba build_method/manifests.rs | sed -n '200,215p'` → `parse_pruning_item` 三个分支；第三分支条件是 `line.contains("optional_pruning_probe") && line.contains("fn optional_pruning_probe")`（`:208`），命中后**返回字面量** `"Button::optional_pruning_probe"`（`:210`），与当前是哪个面无关。调用点 `:189` 再拼上本面 module（`format!("{module}::{item}")`），所以写进产物的是一条与本面无关、且模块前缀也不对的符号。
- 三个魔法标识符的清点（我的命令，`git grep` 全仓、排除 `docs/`）：`PRUNING_TABLE` 只命中 `build_method/manifests.rs:202,205`；`pruning_probe` 只命中 `:206,207,208,210`；`optional_pruning_probe` 只命中 `:208,210`。即**没有声明者、没有测试**，作者"全仓仅此处"成立。
- 读者：`build_method/graft_view/plan_rows.rs:118-119` 的注释与 `cli/explain_report.rs:133-147`、`mcp/evidence.rs` 确实把 `symbol` 渲染出去（我在 `cli/explain_report.rs:140` 看到 `let pruned = symbols.iter().any(|symbol| symbol != "-")`，即这列会被当成事实判断）。
- 表述修正（不是事实错误）：作者写"第三个分支**无条件**返回…"，严格说是"条件命中后返回的字符串与面无关"。建议改成后者，免得读者以为分支无条件成立。

### S4 · 第二套函数扫描器 → **部分证实**（"树内读者为零"**被证伪**）

- 成立的一半：
  - `build_method/manifests.rs:111-162` 是按行文本扫描（`:125-157` 逐行、`:154` 用 `trimmed == "}"` 结束 impl 归属），`:164-171` 的 `function_name` 用 `line.find("fn ")`，注释/字符串里的 `fn ` 同样命中 —— 我逐行读过，与作者描述一致。
  - 内核那份 `core/src/registry_core/source/source.rs:120` `pub fn function_symbols(source: &str)` 确实存在；消费者确实有：`mcp/index.rs:199`、`studio/studio/app/source_index.rs:11`（经 `run_method::source` 再导出）与 `:90`、`studio/studio/app/graph_queries.rs:56`、`studio/studio/app/search_queries.rs:55`。（我第一次 grep 只看到前 10 行、误以为这两个面不消费它，复 grep 后确认作者对。）
- **被证伪的一半**：作者写"这份产物的树内读者为零（`grep -rn function_manifest` 只命中写入侧…CLI/MCP/Studio 都不读它）"。我 grep 全仓（含 `tools/`）得到**第 5 个命中**：`tools/xirang-release-audit:38-43` 会 `find target/release/build -name function_manifest.tsv`，把每一份转成 `target/xirang-audit/release/node_functions.tsv`（`awk 'NR == 1 { next } { print package "\t" $0 }'`）。该工具由 CI 的 `release-audit` 任务运行（`.github/workflows/ci.yml:171`），所以第二套扫描器的输出**会进发布件审计产物**。
- 后果：作者两选一的修复建议里，"如果确认无人读这份产物，就在同一轮里删掉写入"**不能照做**（有人读）。正确方向只剩"改成调用内核 `function_symbols`"，并顺带核对 `node_functions.tsv` 的消费方。
- 严重度：不变（MAJOR）——理由从"随时会被树外读者信任"改成"CI 发布审计正在读一份由另一套扫描器产出的符号表"。

### S5 · 迁移用无边界文本替换 → **证实**

- 证据：`nl -ba run_method/authoring/operations/migration.rs | sed -n '165,193p'`：
  ```text
  179  let mut updated = original
  180      .replace(&rust_old, &rust_new)
  181      .replace(&old_prefix, &new_prefix);
  ```
  `rust_old = format!("crate::{}", paths.old_module_path)`（`:171`）、`old_prefix = format!("src/{}/", …)`（`:169`）——纯 `String::replace`，无 `::`/`/` 边界、不分注释与字面量 ✓ 与作者描述逐字一致。写盘条件 `:188` 只比较"是否与原文本不同" ✓。
- 补充我顺手查到的：`grep -c '#\[test\]' run_method/authoring/operations/migration.rs` → **0**，即整条迁移路径没有任何单测；`git grep -l 'migrate_module_subtree\|migrate_kind_subtree\|edit_module_face' run_method` 只命中 `face_manifest.rs`/`migration.rs`/`operations.rs`/`tests/path_compat.rs`（后者是路径兼容测试，不覆盖重命名）。这解释了为什么 S5 能一直存在：**没有夹具**。
- 严重度：不变。作者给的对立口径（`build_method/scope_view.rs:109-116` 明确写"边界是 `::` 段，不是文本前缀"）我读过，确实存在——同仓两种口径。

### S6 · `FaceManifest.values` 的集合不一致 → **证实**（逐项机械计数）

我用脚本/`nl` 数了作者点名的每一个集合：

| 集合 | 出处（我打印过） | 我的计数 | 作者说 |
| --- | --- | --- | --- |
| `FACE_FIELD_ORDER` | `core/src/registry_core/declaration/registration.rs:427-461` | **24**（22 个字面量 + `FACE_FIELD_PLUGIN` 常量 + `"runtime_checks"`） | 24 ✓ |
| `CREATE_FIELD_ORDER` | `run_method/authoring/operations/face_write.rs:32-55` | **22** | 22 ✓ |
| `EDIT_FIELD_ORDER` | `run_method/authoring/operations/face_write.rs:59-80` | **20** | 20 ✓ |
| `ModuleFaceValues` 字段 | `run_method/authoring/operations/face_values.rs:20-44` | **23** | 23 ✓ |
| `FACE_FIELD_COUNT` | `core/src/registry_core/authoring/face_field.rs:107` | **26** | 26 ✓ |

- 三个字段确实不在 `FACE_FIELD_ORDER` 里（我按数组逐项判过：`handle` / `registry_name` / `params` 全为 False），所以 `run_method/authoring/manifest/parse/parse.rs:108-115` 那个 `values.insert("handle", …)` 永远只会被写成 `kind` 的副本、且渲染模板（`run_method/authoring/manifest/face/render.rs:243-245` 的格式串）不发射它 → 死字段 ✓。
- `edit()` 白名单（`run_method/authoring/manifest/face/face.rs:88-102`）含 `handle`/`params`/`registry_name`，而两个字段顺序表都不含它们 → 从两个公开入口不可达 ✓。
- `plugin` 是唯一有"响亮拒绝"保护的字段（`run_method/authoring/manifest/face/render.rs:13-28`）✓，同模板对其余三个没有任何对应保护 ✓。
- 幂等性钉子确实是手搓 `BTreeMap`（`run_method/authoring/operations/authored.rs:186-213`），没有"解析→应用→渲染"的真实文件往返 ✓（我读到的 `from_values → as_patch` 形状一致）。
- 严重度：不变。

### S7 · `compile_error_demo` 魔法目录名 → **证实**（站点清单从 5 修正为 10）

作者列了 5 处；我 grep 全仓（`git grep -n compile_error_demo`，排除 `docs/`）得到 **10 处、跨 4 个 crate**：

| 站点 | 作用 |
| --- | --- |
| `build_method/renderer/tree.rs:68-70` | 顶层叫这个名字就套 `#[cfg(feature = "compile_error_demo")]` |
| `build_method/registration_check.rs:86-88` | 跳过**文件名**为 `compile_error_demo.rs` 的文件（`!include_demo` 时） |
| `build_method/contracts.rs:78` | `node.name == "compile_error_demo" && !include_demo` 时跳过 |
| `build_method/static_plan.rs:74` | `node.name == "compile_error_demo"` 时跳过——**没有 `include_demo` 逃逸** |
| `build_method/renderer/pass.rs:168-170` | `demo_errors` 包在 `#[cfg(feature = "compile_error_demo")]` 下 |
| **`core/src/registry_core/source/walk.rs:42,51,59`** | 内核 `SourceWalk` 的**公开字段** `pub skip_compile_error_demo: bool` + `skips()` 按名字命中 |
| **`run_method/authoring/operations/operations.rs:289`** | 创作面把 `skip_compile_error_demo: true` 写死 |
| **`run_method/authoring/parse/parse.rs:31`** | 与 S8 同一处的名字清单 |

- "没有声明者"成立：`grep -rn compile_error_demo --include=Cargo.toml .` 零命中（既没有目录也没有特性声明）。
- 我顺手发现的**加重项**：`static_plan.rs:74` 的跳过是无条件的，而 `build_method/contracts.rs:78`/`build_method/registration_check.rs:86` 是可被 `include_demo` 打开的。也就是说当那个特性**真的被打开**时，同一个子树对"注册检查/合同检查"可见、对静态计划不可见——按 `build_method/static_plan.rs:195-199` 自己那句"the plan and the compiled crate would disagree"的标准，这正是它要避免的分歧。这条作者没提（他们的描述是"默认跳过它"）。
- 严重度：不变（MAJOR）。作者的 5 处清单不完整，但方向和判据都对。

### S8 · authoring `source` 回落硬编码夹具名 → **证实**

- 证据：`nl -ba run_method/authoring/parse/parse.rs | sed -n '20,42p'`：
  `:27-29` 先按 `source_root()` 求相对路径；`:31` `let roots = ["compile_error_demo", "control", "engine", "trimmed_core"];`；`:33-40` 在绝对路径的**任意**分量里找这四个名字之一，命中就把该分量之后的部分当相对路径返回；`:41` 兜底返回整条路径 ✓ 与作者描述一致。
- 作者给出的对照物我读过：`core/src/registry_core/identity/path_text.rs:123-133` 的 `manifest_relative_source` 是"去掉 manifest_dir + 一个 `src/`"的规则，不依赖硬编码 ✓。
- 名字清点的修正：作者写"`grep -rn '"engine"' run_method/src` 只命中这里"——`"engine"` 在 `run_method/src` 里确实只此一处，但全仓还有 `core/src/registry_core/syntax/face_tests.rs` 与 `run_method/macros/face.rs`（后者的 `"engine"` 出现在别处上下文）。这不影响结论（`trimmed_core`、`compile_error_demo` 的组合确实是本仓夹具名），只是"只命中这里"的说法要限定到 `run_method/src`。
- **作者复核手段的问题**：他们建议"对 `/tmp/control/proj/src/foo/foo.rs` 这类路径调用 `source_path_from_file`，断言结果不是 `proj/src/foo/foo.rs`"——但 `source_path_from_file` 是 `pub(super)`，crate 外调不到。可行做法是在 `run_method` 内加一条单测（这也是我给 t14 的建议）。
- 严重度：不变。

### S9 · `linked` collector 什么都不提交 → **证实**

- 证据（我打印的两段）：
  - 文档承诺：`run_method/macros/face_registration.rs:9-10` "the collector ident selects which linker section receives the registration" ✓（中文同句在 `:12`）。
  - 三个分支：`:163` `(development; …) => {}`；`:164-167` `(debug; …) => #[cfg(debug_assertions)] ::xirang_debug_method::submit! { … }`；`:168-171` `(linked; …) => { /* 注释 */ }` → **只有 debug 提交**。
  - `linked` 是外部形式的默认：`run_method/macros/face_external.rs:51` `{ @tokens $($tokens:tt)* } => { $crate::face_fields! { @external collector: linked, $($tokens)* } }` ✓。
- 备注：作者说 `collector: bogus` 会落到裸 `no rules expected` —— 这属于宏展开诊断，不在我这条的复核范围，未实测。
- 严重度：不变。

### S19 · `flow_provider` 编辑入口缺 `guard_nesting` → **证实**（我的装置；阈值比作者报的更低）

装置：我自己写的 `/tmp/vs-probe`，`syn 2.0.119`（= 工作区锁文件里的版本），在**显式指定栈大小**的线程里调用与 `run_method/authoring/manifest/face/face.rs:141` 同一形状的 `syn::parse_str::<syn::Path>`，输入为 `A<A<…>>`。**没有**运行作者的探针。

```text
$ vs-probe syn 150 8388608        # 8 MiB 栈
unguarded_syn_result=true / thread_outcome=returned_normally            exit 0
$ vs-probe syn 200 8388608        # 601 字节
thread '<unknown>' has overflowed its stack / fatal runtime error: stack overflow, aborting
exit 134
$ vs-probe syn 20 262144          # 256 KiB 栈，61 字节
fatal runtime error: stack overflow, aborting                            exit 134
$ vs-probe guarded_syn 200        # 先 guard_nesting 再 syn（内核 flow.rs:107-109 的形状）
guarded_syn_result=Err(guard refused: input nests 129 levels of generic arguments, above the limit of 128 …)
exit 0
$ vs-probe guard 100 / 128 / 129 / 200
Ok / Ok / Err(nests 129 … limit of 128) / Err(…)
```

- 结论：**abort 不是 `Result`**。同一份输入经内核那道守卫后返回 `Err` 且进程存活（exit 0），裸 `syn` 直接 abort（exit 134）——S19 的要害（"不是少一条诊断，而是进程死"）在我这里成立。
- **与作者数值的差异（要点）**：作者报"8 MiB 栈：深度 200（601 字节）→ 正常返回；深度 300（901 字节）→ abort；256 KiB：深度 100（301 字节）→ abort"。我的装置在同一 syn 版本、同一字节数下 **8 MiB 就已经在深度 200 abort**，256 KiB 在 **深度 20（61 字节）** 就 abort。两处差异只能来自装置：他们没说栈是主线程（Linux 默认 8 MiB）还是显式线程、也没有给调用帧深度；我的每级帧更大（我把 `syn::parse_str` 放在 `thread::Builder::spawn` 的闭包里、且未开优化）。因此**"几百字节"这个量级一致，具体阈值随装置变化**，报告里应写成"8 MiB 栈下 200 级（601 字节）起就会 abort（我的装置）"，不要把 300 当成常数。
- 代码阅读佐证（我自己读的两侧与链）：
  - 缺守卫：`run_method/authoring/manifest/face/face.rs:140-143`（`if field == "flow_provider" && !value.trim().is_empty() { syn::parse_str::<syn::Path>(value)… }`）——同一函数里其它字段都走内核解析器（`flow` 走 `parse_flow_value`）。
  - 有守卫：`core/src/registry_core/authoring/parse/flow.rs:94` `pub fn render_flow_provider`，`:107` `guard_nesting(value)`，`:108` 才 `syn::parse_str`；动机注释在 `:99-106`。渲染侧 `run_method/authoring/manifest/face/render.rs:110` 调的正是它。
  - 可达链：`operations.rs:318` `edit_module_face`（`pub`）→ `apply_module_face_values`（`face_write.rs:87`）→ `face_write.rs:131`/`:168` → `face.edit(...)`；补丁字段 `ModuleFacePatch.flow_provider` 是公开结构体字段（`operations.rs:122` 起的结构体定义里我在打印中看到其声明区）。我没有从 crate 外驱动 `edit_module_face`（它要 `&Registry` 与 `NodeId`，构造需要宿主宏），所以这一段是**代码阅读佐证**；我实测的是它最终调用那一行。
- 严重度：不变（MAJOR）。

### C1 / C2 · 文档块与声明错位 → **证实**

- C1：`nl -ba macro/src/front_end.rs | sed -n '14,40p'` —— `:18-25` 是"在顶层 `;` 切分 token 流"的文档块，`:26-36` 是 `splice` 的文档块，`:37` 才是 `pub(crate) fn splice(`；两块之间**没有任何条目**，因此第一个块成了 `splice` 文档的前半段；真正的 `split_semicolons` 在 `:57`，上方无 `///` ✓ 逐字复现。
- C2：`nl -ba build_method/discovery.rs | sed -n '230,265p'` —— `:234-235` "Every `.rs` file under `directory`…" 与 `:236-237` "The filesystem facts…" 两段紧接 `:238 struct StdSourceTree;`，而 `:261 pub(crate) fn collect_rust_sources` 无文档 ✓。严格说第二段是 `StdSourceTree` 的（结构体自带说明），错位的是第一段——作者的描述（"两段文档 + struct 紧接其后"）略粗，结论正确。
- 严重度：不变（MAJOR，按维护者"过时/与实现不符者一律 MAJOR"的口径）。

### C3 · collector 注释与实现不符 → **证实**（= S9 的注释面）

- 证据同 S9；另外 `face_registration.rs:169-170` 写的是 `xirang-debug`（C5 的同一条）✓。
- 严重度：不变。

### C4 · `parent` 被标 Required 而实际可选 → **证实**

- 证据：`nl -ba run_method/macros/face.rs | sed -n '132,145p'` → `:139-140` "Where this face hangs. Required." / "本面挂在谁下面。必填。"，示例 `parent: crate::control::NODE_ID`。唯一的匹配臂 `run_method/macros/face_objects.rs:111` 用 `$crate::__face_expr_or!($crate::root_node_id(env!("CARGO_PKG_NAME")); $($parent)?)` → 省略时取包根 ✓。
- 运行佐证（我跑的既有测试，不是作者脚本）：`cargo test -p xirang-run-method --offline --test face_arm_defaults` → `test result: ok. 3 passed; 0 failed`（exit 0）。该测试文件正是作者指出的"已钉住默认值行为"的一处，说明省略 `parent` 的路径是被跑到且绿的。
- 严重度：不变。

### C5 · 两个不存在的 crate 名 → **证实**

- 证据（我的 grep，排除 `-method` 后缀）：
  - `xirang-build`：`run_method/macros/entry.rs:18`、`:24`、`:63`（`:67` 是其中文半边）。
  - `xirang-debug`：`run_method/macros/face_registration.rs:169`、`:170`。
  真实名分别是 `xirang-build-method` / `xirang-debug-method`（`build_method/Cargo.toml:2`、`debug_method/Cargo.toml:2`）。
- 备注：`studio/studio/app/tests/project.rs:243` 的 `assert!(manifest.contains("xirang-build"))` 是子串断言，不构成"名字写错"，不是本条的延伸。
- 严重度：不变。

---

## 3. MINOR 抽样（10/13）

- **S10 证实**：`run_method/macros/face_objects.rs:169` 的容错回退臂用 `::xirang_run_method::face_fields!`；同文件 `:10`、`face_external.rs:48` 用 `$crate::face_fields!`。作者"只有这一条臂用绝对路径"的说法我按 grep 抽查了 `$crate` 与绝对路径的出现点，一致。
- **S11 证实**：`build_method/contracts.rs:193-206` —— `source.find(&marker)`（只取第一处，`:195`）、`args.find(')')`（`:199`）、`split('"')` 取奇数下标（`:201-203`）。三方缺陷（只读第一处 / `)` 截断 / 转义引号错位）与注释/字面量不设防，代码形态完全对应。
- **S12 证实**：`run_method/runtime/trace/artifact/parse.rs:122-124` 局部值的 `function` 用 `frame_functions.get(...).unwrap_or("<local>")`；`:147` 边的 `function` 用 `local_functions.get(&to).copied().unwrap_or("<runtime>")`。文档 `:22-26` 前半句说"记录可以任意顺序出现"、后半句补了依赖关系——作者对"文档两半都在、但行为是静默降级"的表述准确。
- **S13 证实**：`run_method/call_report/call_report.rs:18-19` 文档首句"Render a complete logical call tree"，`:24` 造空 `CallTrace::new()`，真正的 trace 入口是 `:30` 的 `render_call_report_for_trace`。**并且"读环境"那一半也成立**：`CallTrace::new()` 在 `run_method/runtime/trace/call_trace.rs:94-96`→ `runtime()`（`:100-102`）→ `with_mode(trace_mode_from_env())`，一个渲染入口因此读进程环境。
- **S14 证实**：`migration.rs:218-225` 的 `rollback_migration` 全部 `let _ =`（`:219` atomic_write、`:221`/`:222` 两次 rename）✓；`operations.rs:374-393`、`create.rs:101-106` 作者点名的另两处形态相同（我按同文件同风格抽查一致）。
- **S15 证实**：`run_method/authoring/manifest/face/face.rs:144-148` 的 `let key = if field == "admission" { "admission" } else { field };` 两个分支值相同，确为恒等分支。
- **S16 部分证实**：结论方向（多数是不可失败的 `writeln!` 目标或自造常量）我认可，但 **"16 处"这个数字我复现不出来**。我用同一口径（非测试文件、排除注释行）分别数：`expect(`/`panic!`/`unreachable!` = build_method 137 + run_method 27 + macro 11 + debug_method 0 = **175**；再加 `.unwrap()` = **225**。所以这条要么写清口径（例如"我逐条判定过 16 条 *语义上* 值得记账的"），要么改成抽样清单——否则读者会以为已经清点完毕。
- **S17 证实**：`macro/src/lib.rs:283` 的字面量里，"alias" 与 "records" 之间实测 **18 个连续空格**（我用脚本测该行最长空格串 = 18，且该行缩进是 16），作者的数字精确。
- **S18 证实**：读取方 `build_method/scope_view.rs:43-48`（`root.join("src")` + `discover_root` + `discovery_fingerprint`） vs 写入方 `build_method/pipeline.rs:31-32`（`scan_root`/`identity_base`）与 `:54-56`（`discover_root_reporting(scan,…)` + `discovery_fingerprint(src,…)`）——两侧基准不同 ✓；方向保守（只会说"不是当前的"）也成立。
- **C6 证实**：三处注释数字实测存在：`face_values.rs:6` "The 28 registration-face fields"、`face_write.rs:82-85` "Write all 27 shared fields"、`macro/src/mirror.rs:286` "The 29 type arguments"。真实计数见 S6 表（22 / 20 / 24 / 23 / 26），**注释与实现、注释与注释之间都不一致** ✓。
- **C8 证实**：`build_method/cache.rs:34-35` 的文档只说"内容变化时才写"，`:36-40` 用 `fs::write` 就地写；对照面 `run_method/authoring/filesystem/filesystem.rs:18-47` 确实是"唯一临时文件 + create_new + rename + 失败清理"（我打印了全文）——同一个仓库两种写盘语义，文档没提。
- **C7 / C9 未复核**：C7 是纯阅读面的重复/残句（我扫过 `face_objects.rs:62-87` 一眼，确有断句），C9 是意见性清单（正例/反例），都属抽样之外，不下结论。

---

## 4. 作者漏掉 / 我顺手发现的同类问题

1. **`build_method/static_plan.rs:158` 是 S19 的同类缺口，作者说"缺口只有 S19 那一处"不成立**（这一条来自他们第三节"查了、干净"的自述）。
   - 数据路径：`core/src/registry_core/syntax/face.rs:58-60` 把声明的 `#[cfg]` **按原文**存成 `FaceSyntax.cfg`；`build_method/static_plan.rs:103-104` 把它交给 `face_cfg_enabled`（`:157`），后者第一件事就是 `syn::parse_str::<syn::Meta>(cfg)`（`:158`）——**没有 `guard_nesting`**。（我按 `syn::parse` 全量扫过四个执行面 crate 的每个调用点并检查其上方 30 行内有没有守卫，这是抓到的两处之一，另一处就是 S19。）
   - 实测（我的探针）：`vs-probe syn_meta <depth> 8388608`，输入 `not(not(…feature = "x"…))`：
     ```text
     depth 50 / 100 / 200 / 400 / 2000 / 10000 → returned_normally, exit 0
     depth 30000                              → fatal runtime error: stack overflow, aborting, exit 134
     ```
   - 门槛比 S19 高得多（≈120 KB 源码 vs 601 字节），所以我把它记为**抽查发现，MINOR**，而不是与 S19 同级：手写不可能，但生成式/graft 源码或代理产出的畸形 `#[cfg]` 可以让构建脚本 abort（构建脚本死掉不是一条诊断）。
   - 顺带：`evaluate_cfg`（`:163-193`）对 `not(...)` 自递归，也没有深度上限——同一输入即使过了 `syn`，也可能在这里递归。这条我只做代码阅读佐证，未实测。
2. **S4 的"树内读者为零"被证伪**：`tools/xirang-release-audit:38-43` 读 `function_manifest.tsv`（见 S4）。
3. **S7 的站点数 5 → 10**，其中 `core/src/registry_core/source/walk.rs:42` 把演示目录名写成了**内核公开字段** `pub skip_compile_error_demo: bool`（跨 crate 边界泄漏）。
4. **S16 的"16 处"不可复现**（见上）。
5. **S8 的复核手段不可用**：`source_path_from_file` 是 `pub(super)`，作者建议的"直接调用它"必须是 crate 内测试。
6. **`build_method/face_view.rs:272-279` 的文档与实现不符**（S1 同一处，作者未归入注释节）：文档说 "walking the resolved parent chain"，实现只走一层缓存。按维护者"过时注释一律 MAJOR"的口径，这条至少该进 C 节。
7. **`syn` 调用点的整体清点**（供后续判断，不当作发现）：四个执行面 crate 共 5 处 `syn::parse*`（作者抓到 1、我抓到 1、`macro/src/mirror.rs:306/315/370` 3 处解析的是宏自己生成的文本）；内核侧另有约 18 处 `syn::parse2/parse_str` 不带守卫（`syntax/entries/graft.rs`、`syntax/fields.rs`、`syntax/tokens.rs:257` 等），它们的输入是编译器给的 token 流——**我未证明任何一处可达 abort**，因此只登记为"待判定的候选"，不建议直接改。作者第三节"内核的解析入口都带守卫"这句话，对 `core/src/registry_core/authoring/parse/parse.rs:218` / `core/src/registry_core/authoring/parse/flow.rs:108` / `core/src/registry_core/authoring/parse/flow.rs:124` 这几个**解析入口**成立，对全 crate 的 syn 调用点不成立。

---

## 5. 我对作者报告本身的评价（判据：它的结论能不能被下游直接采用）

- 严重度定级我一条都没改：15 条 MAJOR 的判据都能被独立复现，其中 S1/S2/S19 我用运行时装置复现、S6/C6 用机械计数复现、其余用逐行阅读复现。
- 需要下游注意的**三处修正**（都已写进上面的条目）：S4 的"零读者"、S7 的站点数、S16 的"16 处"；以及 S19 的阈值数字（随装置变化，别当常数）。
- 作者的"锚点自检"我复核过一遍口径：他们正文确实没用配对写法，因此不会被 `conventions/src/doc_anchors.rs` 的 token 检查误伤——与我 t6 报告的结论一致。

## 6. 局限（未判定项，明说）

- 我只复核了 MAJOR 全部与 MINOR 的 10/13；C7、C9 未判定。
- 我唯一的运行时装置是 `/tmp/vs-probe`（公开 API + 裸 syn）；**没有**驱动 `edit_module_face` 的完整事务（需要构造 `&Registry`/`NodeId` 的宿主侧夹具），因此 S19 的"可达链"是代码阅读佐证 + 对末端那一行的实测，而不是端到端复现。
- 探针在 `/tmp`，不在工作区；工作区内本次只新建本报告。构建产物在 `/tmp/vs-probe/target`，未写入仓库 `target/`。
