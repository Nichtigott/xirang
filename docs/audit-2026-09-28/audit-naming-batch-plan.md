# B7+B8 命名批次开工前普查（50 条：B7 30 + B8 20）

- 普查者：**boundary-architect（t92，只读）**。本文件是本轮**唯一**写入物：源码、既有文档、他人产物一律未动。
- 数据来源：`docs/audit-2026-09-28/audit-findings.json` 按 `batch` 以 `B7`/`B8` 开头过滤 = **50 条**（`NAM-*` 30 条 = B7，`AMB-*` 20 条 = B8；10 MAJOR + 40 MINOR）。每条站点的行号由**本文件给出的命令**在**当前树**上现算，不采信报告里的旧计数——树在报告之后已经变过（见 §8）。
- 复核本文件是否只读：`git status --porcelain | grep -v '^?? docs/audit-2026-09-28/audit-naming-batch-plan.md$' | wc -l` ⇒ 必须 **0**。
- 本批的执行形态：**跨 crate 改名会撞两个共享面**（`conventions/src/shims.rs` 的 SHIMS 棘轮、活文档锚点），因此**必须在静置窗口里整批一次提交**（§4 的尾段 G4b 是这条约束的落点）。

## 0. 结论摘要（先说执行者最需要的三件事）

1. **50 条里只有一部分是"真·文件改名"**：B7 的 A①②③④ 族（NAM-01..22）是文件/模块层，B⑦+B 族（NAM-30..37）是符号层，C 族（NAM-40/41/43）与 NAM-33/19/12/23 是**规则与文书**（不改名、只写共识/首行声明）。B8 的 20 条**全部是"裁定 + 文档"**（保留或限定），其中 8 条与 **9→3 发布面合并**耦合（§6）。
2. **已经在树上被前几轮做掉的**（开工前必须先扣掉，否则会重复劳动/撞车）：`NAM-02`（`state/misc.rs` 已拆成 `call_tree_view.rs`/`reload.rs`/`overlays.rs`）、`NAM-06`（`app/writers.rs` → `write_guard.rs`，且有钉子 `b5_studio.rs:114` 断言旧模块名已消失）、`NAM-31` 的**命名半**（`apply_trait_label` 已改名，只剩 LG-21 的行为半）、`NAM-23`（`ValidationChannel` 别名已在文档里写明"定义在 kernel，本别名保留历史路径"）。`NAM-01` **部分已做**（新文件 `app/project_context.rs` 已建，旧 `app/support.rs` 已缩成 2 行历史路径 shim）。
3. **切分**：4 个可派发的应用任务（G1 studio / G2 mcp / G3 build_method / G4a run_method）+ **1 个不可并行的共享尾段 G4b**（core 符号、跨 crate 收平、四条规则与 SHIMS/AGENTS.md），细则与 inScope 见 §4。

## 1. 横切面（一次性说明，逐条只标注"是否命中"）

### 1.1 SHIMS 棘轮（`conventions/src/shims.rs`，17 条）

`SHIMS: &[(&str, &str)]`（`conventions/src/shims.rs:27`）钉住 17 条历史重导出，分布于 10 个文件：

| # | 文件（SHIMS 条目） | 与哪条改名相关 |
| --- | --- | --- |
| 1–2 | `run_method/lib.rs` | —（crate 根） |
| 3–4 | `run_method/registry/registry.rs` | NAM-10（shim 族）、NAM-11（`registry` 保留词） |
| 5 | `run_method/authoring/face_manifest.rs` | **NAM-09** |
| 6 | `run_method/authoring/parse/parse.rs` | **NAM-10** |
| 7 | `run_method/authoring/validation/validation.rs` | **NAM-03** |
| 8 | `run_method/runtime/runtime.rs` | — |
| 9–11 | `run_method/runtime/trace/trace.rs` | NAM-08/NAM-11（`trace` 面） |
| 12 | `run_method/runtime/evidence.rs` | NAM-11（`evidence` 保留词） |
| 13–14 | `run_method/plugin/plugin.rs` | AMB-20（shim 家族） |
| 15 | `build_method/syntax.rs` | **NAM-40/43**（挂载名 `registry_syntax`） |
| 16–17 | `build_method/identity.rs` | **NAM-40/43**（挂载名 `registry_identity`） |

**硬事实（本仓行为，不是推测）**：`conventions/src/shims.rs:120` 的 `missing_shims` 对列表里的文件做 `fs::read_to_string(...).expect()`——**文件被改名/搬走时门禁不是报一条 finding，而是 panic（`cannot read <path>`）**。所以：
- 凡改这些**文件路径**，SHIMS 条目必须在**同一次提交**里改掉（这是一次被记录的决策：棘轮"可以增长，但不因疏忽而缩短"）；
- 凡改**模块名**而保留文件（NAM-10、NAM-40/43 的建议），SHIMS 完全不受影响 —— 这是本批优先选"首行声明/挂载名统一"而不是"动 shim 文件名"的原因。

### 1.2 活文档锚点（`doc_anchors`）与文档代码块（`doc_blocks`）

两者共用同一份文档集：`crate::doc_blocks::markdown_files(root)`（`conventions/src/doc_anchors.rs:41`）= 根级 `*.md` + `docs/**` 递归 + 各 crate 的 `README.md`/`README.zh-CN.md`，**排除记录**（任一**路径分量**以 `audit`/`design` 开头，`conventions/src/doc_blocks.rs:69` 的 `RECORD_PREFIXES`）。
- 现算活文档集 = **33 份**（命令见 §7 的 R-1）。
- 其中与 B7 改名相关的锚点，**实测只有一处**：`docs/roadmap-1.0.md:157` 引用 `authoring/validation/validation.rs:31-45`（NAM-03）——改 NAM-03 时这行必须同改，否则 `doc_anchors` 红。
- `plugin-host/README*.md`、`mcp/README*.md` 等 31 份对其余改名**零命中**（§7 的 R-2 命令）。
- 记录目录（`docs/audit-2026-09-28/**` 等）被两个门禁豁免：**本文件与所有审计产物里的 `file:line` 不受漂移影响**（这也是它们能自由引用行号的原因）。

### 1.3 其它横切面（实测）

| 面 | 与 B7 改名的关系 | 命令 |
| --- | --- | --- |
| `CHANGELOG.md` | 13 个目标路径**零命中** ⇒ 不需要改 CHANGELOG | §7 R-3 |
| `examples/**` | 对 13 个目标路径**零命中**；但 NAM-07 的收平**必须豁免** `examples/**`（身份承载） | §7 R-3 |
| `tools/**` | 零命中（`tools/xirang-publish` 只认 crate 名与版本，不认这些模块路径） | §7 R-3 |
| `.github/workflows/**` | 零命中（CI 用 `-p <crate>`，不用模块路径） | `grep -rnF '<old>' .github/` |
| 测试叶子名 | **模块改名的连带效应**：`cargo test -p <crate> -- --list` 里的模块前缀会变（如 `studio::app::support::…` → `studio::app::project_context::…`），**叶子名（函数名）不变**。执行时按 G4b 的口径逐名单对比（t82 的做法：`--list` diff 为空） | §7 R-4 |
| 挂载名与文件 stem 不一致（NAM-40/43） | 现算四族：① 历史 shim 2 处（`build_method/{identity,syntax}.rs`）② CLI 命令 5 处（`cli/commands/*.rs`）③ `explain` 子模块 3 处 ④ 测试模块 4 处（`cli/lib_tests.rs`、`core/.../{lexicon/lexicon_tests,source/source_tests,syntax/face_tests}.rs`） | §7 R-5 |
| 身份红线（`file!()` → `NodeId`） | NAM-07 的收平会让被移动的文件改 `file!()` ⇒ **只许动发布面 26–28 处，`examples/**` 与 `studio/tests/fixtures/**` 的 6–9 处一律不动**（§4 G4b 的硬约束） | §7 R-6 |

## 2. B7（30 条）逐条

**读法**：站点一律给"命令 + 我在当前树上的改前计数"。「判别性」＝该模式在改名完成后必须为 **0**（改前 > 0，可当牙齿）；「计数用」＝只用来估工作量，改名后不一定为 0（例如散文里提到旧词的句子）。
命令前缀统一为工作区根；`<scope>` 取该条自己的 crate 目录。

### A① 类别名（NAM-01/02/06）

| id | 现状（file:line） | 目标名（fix_hint） | 站点（命令 + 改前计数） | 显式/注入 | 横切面 | 状态 |
| --- | --- | --- | --- | --- | --- | --- |
| **NAM-01** | `studio/studio/app/support.rs`（挂载 `studio/studio/app/app.rs:47 mod support;`） | `app/project_context.rs`；几何→`app/hot_zones.rs`；编辑器入口→`app/editor_launch.rs` | 判别性：`grep -rnE '::support(::|[^a-zA-Z_])|mod support;' studio/src` = **15**（mod 1 + refs 14，散在 **12** 个文件） | 全部**显式**（`use super::support…`/`use support::host_manifest`）。无宏注入 | 无 SHIMS；活文档 0；测试叶子名前缀变；`app.rs` 挂载行 | **部分已应用**：`app/project_context.rs` 已在、`app/support.rs` 已缩成"Historical path"2 行 shim。剩余＝几何/编辑器归位 + shim 去留（建议随 G1 一次收口并保留 shim 一个批次） |
| **NAM-02** | ~~`studio/studio/app/state/misc.rs`~~（已不存在） | 拆 `state/call_tree_view.rs` + `state/reload.rs` + `state/overlays.rs` | 判别性：`grep -rnE 'misc' studio/src` 只余 **0** 处模块形态（`::misc`/`mod misc;` 均 0） | — | — | **已应用**（三个新文件都在；`source_path_for` 已归 `project_context.rs:275`）。本批只需在 §8 记账，不再动 |
| **NAM-06** | ~~`studio/studio/app/writers.rs`~~（已不存在） | `app/write_guard.rs` | 判别性：`grep -rnE 'mod writers;' studio/src` = **0**；负钉子 `studio/studio/app/tests/b5_studio.rs:114` 断言旧名已消失 | — | — | **已应用**（现挂载 `app/app.rs` 的 `#[path = "write_guard.rs"] mod write_guard;`）。不再动 |

### A② 名字与父目录重复（NAM-07）

| id | 现状 | 目标 | 站点 | 显式/注入 | 横切面 | 状态 |
| --- | --- | --- | --- | --- | --- | --- |
| **NAM-07** | 全仓 `<x>/<x>.rs` 且该目录**递归**只有这一个 `.rs` | 按谓词 **A1** 收平（把唯一那份提升为裸 `mod`），**只做发布面** | 我现算（命令见 §7 R-6）：**A1 总 35 处 / 发布面 26 处 / 豁免 9 处**（审计值是 34/28/6——**差 3，树已变动**，执行前必须现算） | **混合，且这是本批最重要的一条**：`#[path]`/`mod` 挂载行是**显式**的（可 grep、可改）；但 `file!()` → `NodeId` 是**宏注入**的——移动文件＝改身份，写入落盘的 graft 记录与按路径字面引用的测试/README 会随之失效。**默认注入的站点 grep 不到**（它们只在编译期生成物里） | 身份红线：`examples/**` + `studio/tests/fixtures/**` 的 9 处**必须豁免**；`conventions/src/mounting.rs`（内核模块挂载门禁）不覆盖其它 crate，但 `size` 棘轮看全仓 | 待做·**结构与身份敏感**，归 G4b，且必须单批一次做完 |

### A③ 名与内容不符（NAM-03/04/05/17/18/21/22）

| id | 现状（file:line） | 目标名 | 站点（命令 + 改前计数） | 显式/注入 | 横切面 | 状态 |
| --- | --- | --- | --- | --- | --- | --- |
| **NAM-03** | `run_method/authoring/validation/validation.rs`；挂载 `run_method/authoring/authoring.rs:32-33`（`#[path = "validation/validation.rs"] pub mod validation;`） | `authoring/context.rs`（模块名 `validation`→`context`） | 判别性：`grep -rnE '::validation(::|[^a-zA-Z_])|mod validation;\|path = "[^"]*validation\.rs"' run_method/src` = **17**（mod 1 + path 1 + refs 15，**11 个文件**） | 站内全部**显式**；但**公开路径是注入面**：下游宿主写的 `xirang_run_method::authoring::validation::*` 不在本仓文本里（SHIMS 表就是它的代表） | **SHIMS `:50`（同 commit 必改）**；`docs/roadmap-1.0.md:157` 锚点；`run_method/README*.md` | 待做·**非纯改名**（公开面 ⇒ 保旧路径 + 加新名） |
| **NAM-04** | `build_method/cache.rs`；挂载 `build_method/lib.rs:27 mod cache;` | `discovery_cache.rs`（只留 3 个缓存函数）；`write_if_changed` → 新建 `run_method/runtime/trace/artifact/io.rs` 或并入 `build_method/manifests.rs`；作用域函数 → `build_method/scope.rs` | 判别性：`grep -rnE 'mod cache;\|::cache(::|[^a-zA-Z_])' build_method/src` = **1（mod）+ refs**；`write_if_changed` 调用方 `grep -rnF 'write_if_changed' build_method/src cli/src mcp/src` = **7**（`pipeline.rs` 3、`manifests.rs` 5 处的子集，现算为准） | 显式 | `write_if_changed` 的文档已按 t64/SUR-C8 补齐"原子写"不变量（改名前先读那段，别把它拆丢）；无 SHIMS | 待做·**拆分型**（一拆三 + 跨 crate 搬函数）⇒ 若把 `write_if_changed` 搬进 `run_method`，会与 G4a 的文件重叠：**本批建议只留在 `build_method/manifests.rs`**，跨 crate 搬迁另开条 |
| **NAM-05** | `mcp/evidence.rs`；挂载 `mcp/lib.rs:82 mod evidence;` | `mcp/build_evidence.rs` | 判别性：`grep -rnE 'mod evidence;\|::evidence(::|[^a-zA-Z_])' mcp/src` = **7**（mod 1 + refs 6，**7 个文件**） | 显式 | 无 SHIMS；与 NAM-11③ 同一条改名（**不要两处各改一次**）；`mcp/README*.md` 0 命中 | 待做·纯改名 |
| **NAM-17** | `build_method/faces.rs`；挂载 `build_method/lib.rs:41 mod faces;` | `scope_faces.rs` | 判别性：`grep -rnE 'mod faces;\|::faces(::|[^a-zA-Z_])' build_method/src` = **3**（mod 1 + refs 2，**3 个文件**） | 显式 | 无 SHIMS；与 NAM-21 同族（一次收敛更省） | 待做·纯改名 |
| **NAM-18** | `build_method/node.rs`（`lib.rs:51 mod node;`）与 `node_id.rs`（`:53 mod node_id;`） | `discovery_node.rs` / `node_identity.rs` | 判别性：`mod node;` 1 + `::node` refs **6**（**7 文件**）；`mod node_id;` 1 + `::node_id` refs **11**（**5 文件**） | 显式 | 无 SHIMS；`build_method/README*.md` 0 命中 | 待做·纯改名（两条同时做，避免中间态） |
| **NAM-21** | `discovery.rs`（`lib.rs:33`）、`registration_check.rs`（`:59`）、`validation.rs`（`:75`） | `source_walk.rs` / `registration_phase.rs` / `face_syntax_check.rs` | 判别性：`discovery` **15 refs / 11 文件** + mod 1；`registration_check` **1 ref / 2 文件** + mod 1；`validation` **5 refs / 6 文件** + mod 1 | 显式 | 无 SHIMS；与 NAM-17/18 同批做（同一文件 `lib.rs` 的挂载区） | 待做·纯改名（三条一起，改 `lib.rs` 一次） |
| **NAM-22** | `mcp/nodes.rs`（`mcp/lib.rs:73 mod nodes;`） | `mcp/resolve.rs` | 判别性：`grep -rnE 'mod nodes;\|::nodes(::|[^a-zA-Z_])' mcp/src` = **11**（mod 1 + refs 10，**9 文件**） | 显式 | 无 SHIMS；与 `studio/studio/ui/graph/nodes.rs`（绘图）**同名不同义，别一起改** | 待做·纯改名 |

### A④ 同名不同职责 / 保留词（NAM-08/09/10/11/12）

| id | 现状 | 目标 | 站点 | 显式/注入 | 横切面 | 状态 |
| --- | --- | --- | --- | --- | --- | --- |
| **NAM-08** | `run_method/runtime/trace/locals/call_trace.rs` | `locals/recording.rs` | 判别性：`grep -rnE 'mod call_trace;\|path = "[^"]*call_trace\.rs"' run_method/src` = **4**（两个挂载点：`runtime/trace/trace.rs:6-7` 与 `runtime/trace/locals/locals.rs:4-5`）——**注意同一个文件被挂两次**，只改 locals 那一处 | 显式 | 无 SHIMS | 待做·纯改名（只动 locals 挂载，`trace.rs` 那处保持） |
| **NAM-09** | `run_method/authoring/face_manifest.rs` | `authoring/face_file.rs`（或 `face_store.rs`） | 判别性：`grep -rnE 'mod face_manifest;\|path = "[^"]*face_manifest\.rs"' run_method/src` = **4**（`authoring/authoring.rs:20-21` 与 `authoring/manifest/manifest.rs:6-7`），refs **2** | 显式 | **SHIMS `:42`（该文件正是 shim）** ⇒ 改文件名必须同 commit 改 SHIMS；活文档 0 | 待做·**非纯改名** |
| **NAM-10** | `authoring/parse/parse.rs`、`registry/registry.rs`、`plugin/plugin.rs`、`runtime/evidence.rs`（7–12 行 shim 族） | fix_hint **推荐不改名**，改为"模块文档**第一行**写明 Shim/历史路径" | 计数用：`grep -rnF 'this shim' run_method/src` / 逐文件读首行 | 显式 | 这些文件的公开路径被 **SHIMS** 钉住（`:45/:46/:13-14/:12`）⇒ 改名＝动公开面 | 待做·**文书型**（写首行声明，不改名）⇒ 无身份/路径风险 |
| **NAM-11** | ① `mcp/index.rs` ② `run_method/runtime/trace/artifact/` ③ `mcp/evidence.rs` ④ `mcp/registry.rs`（保留） | ① → `source_index.rs` 命名（照 studio）；② 目录 → `snapshot/`、文件 → `snapshot.rs`；③ 按 NAM-05；④ 保留 | ① 判别性：`grep -rnE 'mod index;\|::index(::|[^a-zA-Z_])' mcp/src` = **7**；② 计数用：`grep -rnF 'trace::artifact' run_method/src` = **1** + `pub use runtime::trace::artifact::{…}` 在 `run_method/lib.rs:63`（公开面！）；③ 同 NAM-05 | ② 是**公开面**：`run_method/lib.rs:63-65` 重导出 `TraceArtifact` 等（不在 SHIMS 表里，但属于已发布路径） | ② 改目录名 ⇒ 公开路径 `xirang_run_method::runtime::trace::artifact` 变化 ⇒ **需保旧路径别名**；SHIMS 目前**没有**这一条（可加一条进棘轮，正是"可以增长"的用法） | 待做·①③纯改名、②**非纯改名** |
| **NAM-12** | `studio/studio/ui/search.rs`（入口 1 fn）与 `ui/search/`（3 子模块） | 只加模块文档首行"搜索页入口（见 search/ 子模块）" | 计数用：`head -2 studio/studio/ui/search.rs` | — | 无 | 待做·文书型 |

### A⑤/A⑥ + C 族（NAM-19/20/23/40/41/43）

| id | 现状 | 目标 | 站点 | 横切面 | 状态 |
| --- | --- | --- | --- | --- | --- |
| **NAM-19** | `studio/studio/app/tests/*.rs`（**已漂移**：现 18 个文件，报告点名的 `project.rs` 等已重组） | 每个测试文件首行加"covers `app/<module>.rs`（+ …）"，**不重命名** | 计数用：`for f in studio/studio/app/tests/*.rs; do sed -n 1p $f; done`（现 18 行，其中 `b4/b5/b6_studio.rs` 已带批次说明、`misc` 已拆） | 无 | 待做·文书型（**执行前先按现清单重算**，报告里的 11 个文件名已不成立） |
| **NAM-20** | `mcp/src` 平铺 39 文件 | 拆 `tools/` + `source/`（结构） | 计数用：`ls mcp/*.rs \| wc -l` | 与结构报告 `audit-boundary-refactor-plan.md` 重叠 ⇒ **不要在本批做**（归结构批次，本批只落 NAM-11 的保留词表） | 挂账·结构批次 |
| **NAM-23** | `plugin-host/src/lazy_wasm.rs:24` 别名 `ValidationChannel` | 文档首行写明"等价于 `PluginChannel`（历史路径）" | 计数用：`sed -n '18,26p' plugin-host/src/lazy_wasm.rs` | — | **已应用**（`lazy_wasm.rs:20-23` 已写"定义本体在 kernel 的 `plugin` 模块；本别名保留历史路径"）⇒ 只记账 |
| **NAM-40** | 挂载名 ≠ 文件 stem 的 14 处（四族） | 把四族写进挂载约定；不再引用"394/394 一致" | 判别性（现算四族，§7 R-5）：历史 shim 2、CLI 命令 5、`explain` 子模块 3、测试模块 4 | `AGENTS.md`（挂载约定）；测试模块那 4 处改了会变**测试叶子名的模块前缀** | 待做·文书+4 处小改（G4b） |
| **NAM-41** | 全仓 4 处 `pub use … as …` | 别名纪律写进共识（同名同义或首行写明历史理由） | 计数用：`grep -rn 'pub use .* as ' */src \| wc -l` | 3 处有理由、1 处（NAM-23）已补文档 | 待做·文书型 |
| **NAM-43** | 同 NAM-40 的第 4 族 | 统一"`mod tests` vs `mod <name>_tests`"二选一 | 现算（§7 R-5）：`*_tests.rs` 39 个文件，其中挂 `mod tests;` 的 4 处（`cli/lib_tests.rs`、`core/.../lexicon_tests.rs`、`source_tests.rs`、`face_tests.rs`）——**审计值 44/40/4 与现算 39/35/4 有差（树已变动）** | 改挂载名会变测试叶子名前缀；`conventions/src/size.rs` 的 `is_mounted_as_test` 认 `#[cfg(test)]` 挂载，不认名字 ⇒ 安全 | 待做·文书+4 处小改（G4b） |

### B 族：函数/方法名（NAM-30/31/32/33/34/35/36/37）

| id | 现状（file:line） | 目标名 | 站点（命令 + 改前计数） | 可否纯改名 | 状态 |
| --- | --- | --- | --- | --- | --- |
| **NAM-30** | `build_method/manifests.rs:169 collect_pruning_symbols`、`:242 parse_pruning_item` | `scan_pruning_probe_markers`（或让符号列真从注册面读） | 判别性：`grep -rnF 'collect_pruning_symbols' build_method/src` = 现算 2（定义 + 调用）；`parse_pruning_item` 同 | **否**：名字改对≠行为可接受（LG-15 假实现）⇒ 与行为修复同批 | 待做·**非纯改名** |
| **NAM-31** | `run_method/authoring/operations/face_write.rs:207 apply_trait_label`（**已改名**） | — | 判别性：`grep -rnF 'apply_trait_contract' run_method/src cli/src mcp/src` = **0**（旧名已 0 ⇒ 该 row 的改名半已完成，只剩 LG-21 行为半） | — | **命名半已应用**；行为半归 LG-21 |
| **NAM-32** | 24 行裸动词清单（`run`/`load`/`find`/`build`/…；WL 4 行 + 违规 20 行） | 见报告"判定"列（入口类保留） | 判别性（逐名）：`grep -rnE '^\s*pub(\(crate\))? (const )?fn (load\|find)\b' */src` | **否（多条）**：`RuntimeCheckSpec::run`（core 公开）、`Registry::find`（core 公开）等是公开面 ⇒ 保旧名 + 加新名 | 待做·混合（多数纯改名，公开面那几条按"保旧+加新"） |
| **NAM-33** | 动词收敛表（`collect` 39 / `parse` 38 / `render` 38 / `resolve` 20 / `read` 17 / `load` 16 …；`query`/`lookup`/`fetch` 为 0） | 写成**规则**（D-4 词表） | 计数用：`grep -rhoE '^\s*pub(\(crate\))? (const )?fn [a-z_]+' */src \| awk '{print $NF}' \| cut -d_ -f1 \| sort \| uniq -c \| sort -rn` | 主体是**规则**（不改名）；只有 `get` vs `find` 那对触发 NAM-37/36 | 待做·文书型（写进共识） |
| **NAM-34** | 8 个 `_owned`/`_strings` 名（如 `core/.../parse/parse.rs:258 split_csv_owned`、`parse_admission_owned`、`parse_requirements_owned`、`parse_registration_rule_owned`、`quoted_strings`、`build_method/contracts.rs:193 rule_method_strings`、`cli/lib.rs:107 argv_strings`、`run_method/.../paths.rs:41 is_xirang_owned_source`） | 见报告"建议"列（`parse_field_list`、`read_string_literals`、`rule_method_calls`…） | 判别性（逐名）：`grep -rnF '<old>' <crate>/src` —— 其中 `parse_requirements_owned`/`parse_admission_owned` 是 **core 公开 API**（`pub fn`） | **否（4 条公开）**：保旧名 + 加新名；其余纯改名 | 待做·混合 |
| **NAM-35** | `core/.../release/release.rs:191 pub const fn full`、`core/.../source/walk.rs:54 fn skips`、`core/.../plugin/contracts/contracts.rs:460 pub fn targets` | `is_full` / `skips_path` / `targets_framework` | 判别性：`grep -rnE '\.full\(\)\|fn full\(' core/src` 等（现算 191 行存在 ✓） | `full` 是 **pub const fn**、`targets` 是 **pub** ⇒ 公开面 | 待做·**非纯改名**（公开面三条） |
| **NAM-36** | `core/.../tree/query/query.rs:80 pub fn find`；内部 `:105 collect_node_path`、`:164 collect_kind`、`:183 collect_depth_first` | `find_by_id`；内部 helper → `visit_*`/`walk_*` | 判别性：`grep -rnE '\.find\(\|fn find\(' core/src cli/src mcp/src studio/src`（现算 `find` 仍是 80 行 ✓） | **否**：`Registry::find` 是**公开 API**（AMB-20 也用）⇒ 保旧 + 加新 | 待做·**非纯改名** |
| **NAM-37** | `core/.../entry_pages/entry_pages.rs:39 pub fn get`、`:74 pub fn remove` | `entry` / `remove_entry`（`get_mut` 保留） | 判别性：`grep -rnE 'fn get\(&self\|fn remove\(' core/src/registry_core/tree/entry_pages/`（现算 `get`/`remove` 仍在 ✓） | **否**：两者都是 **pub** | 待做·**非纯改名** |

## 3. B8（20 条）逐条

**口径先说清（重要）**：`audit-findings.json` 里 **20 条 AMB 的 `fix_hint` 字段是同一个模板残句**（"建议保留 `plugin-host`（crate/目录名）**"），**不是**逐条建议——真正的裁定在 `docs/audit-2026-09-28/audit-naming-ambiguity-verify.md`（§3 的逐条复核表）与 `docs/audit-2026-09-28/audit-publish-surface-merge-plan.md`（9→3 决策表）。下表据此给"裁定 + 动作"。

| id | 现状名（file:line） | 歧义点 | 裁定（来源行） | 本批动作 | 与 9→3 合并耦合 |
| --- | --- | --- | --- | --- | --- |
| **AMB-01** | `core/` + lib `xirang`（`core/Cargo.toml:32`） | `core` = std crate 名；`xirang` 一名三角色（项目/lib/bin） | 证实，MAJOR 维持（verify.md:62） | 保留 + README 首段"kernel here = 纯协议词汇与纯方法" | **是**（合并方案 `:38/:53` 已定 `kernel/`）⇒ 留给合并批次 |
| **AMB-02** | `build_method/`（`build_method/pipeline.rs:55`） | `method` 读作"类型上的函数"；`build` 与 Cargo 同词 | 证实，MAJOR 维持（:63） | 保留 + 一行说明；**模块 `build` → `build_time`** 是合并方案的改名 | **是**（合并方案 `:17`） |
| **AMB-03** | `run_method/`（`run_method/lib.rs:27`） | `run` 与 `cargo run`/入口 `run()` 同词 | 证实，MAJOR 维持（:64） | **留给合并批次**（crate 改名 + `lexicon.rs:28` 的 `RUN_METHOD_CRATE` 四处引用） | **是**（任务书点名的例子） |
| **AMB-04** | `plugin-host/`（`plugin-host/src/verifier.rs:10`） | 方案模块名 `plugins` 与 `.xirang/plugins`（`admission.rs:38`）同词反义 | 证实，MAJOR 维持（:65）；**队长"应叫 plugin-runtime"的猜测被证伪** | crate 保留；方案模块名改 `plugin_host` | **是**（合并方案 `:19`） |
| **AMB-05** | `core/.../tree/registry.rs:18 pub struct Registry` | `registry` 在 Cargo 生态是包索引 | 证实，MAJOR 维持（:66）：修法是**限定/词表登记**，不必强改公开类型 | 写进保留词表 + 文档限定 | 部分（类型名不改） |
| **AMB-06** | 顶层 `examples/`（`Cargo.toml:2`） | 读者以为 `cargo run --example` | 部分证伪、**降 MINOR**（:67）：根清单是虚拟清单、成员显式列出 | 保留 + README 一行"这里的 `examples/` 是两个示例宿主包，不是 cargo example target" | 否（但 `examples/**` 是 NAM-07 的身份豁免区） |
| **AMB-07** | `debug_method/`（`debug_method/src/lib.rs:26`） | 与 `debug_assertions` 同词头 | 证实（:77）；合并后 crate 名消失（并入模块 `call_evidence`） | **无需额外动作** | **是**（合并方案 `:42`） |
| **AMB-08** | bin `xirang-dev` + feature `dev-supervisor`（`studio/bin/xirang-dev.rs:1`） | 读者以为它是 studio 的开发版二进制 | 证实（:78）；**已拍板保留**（合并方案 `:24`） | 保留（README 已解释 `required-features`） | **是**（拍板已落文书） |
| **AMB-09** | 顶层 `picture/`（`README.md:3`） | 读者按 `assets/`/`images/` 找图 | 证实但**判为优雅问题**（:79） | 保留 + 一句"品牌与界面截图资源"（或移出发现清单） | 否 |
| **AMB-10** | `docs/ROADMAP.md` 与 `docs/roadmap-1.0.md` | 只差大小写 | 证实，**跨平台 FS 风险**（:80） | 保留 + 加一句"大小写不敏感 FS 上会互覆"；**改名成本高（活文档锚点全覆盖）**，本批只文书 | 否 |
| **AMB-11** | 内核模块 `syntax` + feature `syntax` | 读者以为解析 `.rs` 文法 | 证实（:81）：同名同物不构成歧义 | 写进词表/共识（不改名） | 否 |
| **AMB-12** | 内核模块 `json`（`core/.../json/json.rs:44`） | 读者按格式实现理解 | 证实的同类（verify.md 表内） | 文书：首行已写"本仓工件文本的 JSON 编码器"⇒ 只登记 | 否 |
| **AMB-13** | 内核模块 `release`（`release/release.rs:9`） | 与 release 构建配置同词 | 同上 | 文书登记 | 否（NAM-07 的 A1 会收平该目录 ⇒ 与 G4b 有文件重叠，**顺序上 G4b 在后**） |
| **AMB-14** | 内核模块 `requirements`（`requirements/requirements.rs:17`） | 读者按 pip 清单理解 | 同上 | 文书登记 | 否（同上，A1 涉及） |
| **AMB-15** | 内核模块 `source`（`source/source.rs:23`） | `source` 一名四义（源码/依赖来源/…） | 证实（:82） | 文书登记 + 保留词表 | 否 |
| **AMB-16** | 类型层 `artifact` 一名四义（`mcp/mir.rs:20`） | 插件工件 vs trace 快照 | 证实（verify.md 表内） | 与 NAM-11② 同批：`trace/artifact/` → `snapshot/` | 否 |
| **AMB-17** | `index` 一名三义（`core/.../tree/index/index.rs:13`、`mcp/lib.rs:76`、`Registry::index()`） | 注册树索引 / 源码索引 / crates.io index | 证实（:83） | NAM-11① + 文书 | 否（A1 涉及 `tree/index/`） |
| **AMB-18** | `host` 一名两义（`plugin-host/src/lib.rs:1`） | 宿主应用 vs 插件宿主 | 同上 | 文书登记（README 已分写） | 部分 |
| **AMB-19** | feature 名与模块/包名重名（`macro/Cargo.toml:20` 等） | 8 个 feature 里 4 个与模块同名 | 证实（:84），并明确 `AGENTS.md` 说"feature 名是公开 API、不要为对齐改名" | **只加 README 对照表 + 一段"同名是有意的"** | **是**（合并方案 `:23`） |
| **AMB-20** | 13 个公开名"整名就是别处既有词"（`cli/lib.rs:94 pub fn main`、`build_method/lib.rs:137 pub fn run`、`mcp/protocol.rs:71`、`RuntimeCheckSpec::run`（`declaration/runtime_checks.rs:345`）…） | 与 NAM-32/33 同规则（D-5：裸动词只许入口位） | 证实（:85，抽核 4 处行号全中） | 与 NAM-32/33 合并成**一张**登记表，按对象去重（避免同一名字被数两次） | 部分 |

## 4. 批次切分与派发建议

**切分原则**：① 组内文件不重叠；② 共享文件（`conventions/src/shims.rs`、`AGENTS.md`、`docs/roadmap-1.0.md`）只在尾段动；③ 每组自带 inScope 与验证命令；④ 每组的"红侧"= 该组自己的 `grep` 判据由 >0 变 0。

### G1 · studio（可并行，纯 studio 文件）
- **inScope**：`studio/studio/app/**`（含 `tests/**`）
- **条目**：`NAM-01`（剩余：几何→`hot_zones.rs`、编辑器入口→`editor_launch.rs`、`support.rs` shim 去留）、`NAM-12`（`ui/search.rs` 首行）、`NAM-19`（18 个测试文件首行加 covers，**先按现清单重算**）
- **不做**：`NAM-02`/`NAM-06`（已应用）
- **顺序**：先 `NAM-01` 的搬函数，再文档两条（避免文档指向移动中的文件）
- **验证**：`cargo test -p xirang-studio --offline`、`cargo test -p xirang-studio --offline --features prototype-fixtures`、`cargo test -p xirang-conventions --offline`

### G2 · mcp（可并行，纯 mcp 文件）
- **inScope**：`mcp/**`
- **条目**：`NAM-05`（`evidence.rs`→`build_evidence.rs`）、`NAM-22`（`nodes.rs`→`resolve.rs`）、`NAM-11①③`（`index.rs` 命名 + 保留词表）
- **不做**：`NAM-20`（结构批次）
- **验证**：`cargo test -p xirang-mcp --offline`

### G3 · build_method（可并行，纯 build_method 文件）
- **inScope**：`build_method/**`
- **条目**：`NAM-17`、`NAM-18`、`NAM-21`（三条一起，只改一次 `lib.rs` 挂载区）、`NAM-04`（**只在本 crate 内**：`cache.rs`→`discovery_cache.rs` + 作用域函数→`scope.rs`；`write_if_changed` 暂留本 crate，跨 crate 搬迁另开条）
- **不做**：`NAM-30`（与 LG-15 行为同批）
- **验证**：`cargo test -p xirang-build-method --offline`、`cargo test -p xirang-example-control-button --offline --test registry`（身份红线）

### G4a · run_method（可并行，但与 G4b 串行——它动 SHIMS 条目）
- **inScope**：`run_method/**` + `docs/roadmap-1.0.md`（NAM-03 的锚点）+ `conventions/src/shims.rs`（**只改 NAM-03/09 两行条目**）
- **条目**：`NAM-03`（保旧路径 + 新名）、`NAM-08`、`NAM-09`（保旧路径 + 新名）、`NAM-10`（首行声明，不改名）、`NAM-11②`（`artifact/`→`snapshot/`，**含新别名**）、`NAM-23`/`NAM-31`（只记账，无改动）
- **顺序**：先改文件与挂载 → 再改 SHIMS 条目 → 再改活文档锚点（一次提交内）
- **验证**：`cargo test -p xirang-run-method --offline --features authoring`、`cargo test --workspace --offline`、`cargo test -p xirang-conventions --offline`

### G4b · 共享尾段（**不可并行**，必须与本批其它组在同一个静置窗口收口）
- **inScope**：`core/src/registry_core/**`、`conventions/src/shims.rs`、`AGENTS.md`、各 crate `README*.md`、`CHANGELOG.md`（若需记一笔）、发布面 26 处 A1 收平涉及的目录
- **条目**：`NAM-07`（A1 收平，**豁免 9 处**）、`NAM-32/33/34/35/36/37`（符号层，公开面按"保旧+加新"）、`NAM-40/41/43`（四族挂载规则 + 4 处测试模块统一）、`NAM-33` 的动词表、B8 的**文书部分**（AMB-05/06/09/10/11..20 的"保留 + 文档"）
- **顺序**（硬约束）：① 先在副本里跑 A1 收平并核对 `examples/studio-tests` 身份不受影响 → ② 符号改名（保旧+加新）→ ③ SHIMS/AGENTS/README 文书 → ④ 全量门禁
- **验证**：五条门禁 + `cargo test -p xirang-example-control-button` + 两个 example 宿主的 registry 测试 + `tools/xirang-publish --check-table`

**派发建议**：G1/G2/G3 可同时开（文件互不重叠）；G4a 与它们**不重叠但会动 `conventions/src/shims.rs`** ⇒ 与 G4b 串行；G4b 单独一个静置窗口收口，本批**一次提交**。

## 5. 不能当纯改名做的条目（须走"保旧名 + 加新名"或与行为同批）

| 条目 | 为什么不是纯改名 |
| --- | --- |
| `NAM-03` | 公开路径（SHIMS `:50`）+ 下游 `use xirang_run_method::authoring::validation::*` |
| `NAM-04` | 一拆三 + 可能跨 crate 搬 `write_if_changed`（7 个调用方） |
| `NAM-07` | 移动文件＝改 `file!()`＝改 `NodeId`（宏注入，grep 不到）；豁免区一动就伤落盘 graft 记录 |
| `NAM-09` | 该文件本身是 SHIMS 钉住的 shim（`:42`） |
| `NAM-10` | 改文件名＝改公开路径 ⇒ fix_hint 本身就建议**只写首行声明** |
| `NAM-11②` | 公开面 `run_method/lib.rs:63` 的重导出（建议新增一条 SHIMS 条目保旧路径） |
| `NAM-30` | 名字改对≠行为可接受（LG-15 假实现） |
| `NAM-31` | 命名半已完成；余下是 LG-21 行为 |
| `NAM-32/34/35/36/37` | 其中多条是 **pub** API（`Registry::find`、`EntryPages::get/remove`、`Release::full`、`parse_*_owned`） |
| `AMB-01..08` | crate/lib/bin 名 ⇒ 与 9→3 合并耦合（§6） |
| `AMB-10` | 改 `docs/ROADMAP.md`／`roadmap-1.0.md` 会牵动活文档锚点与外部链接 |

## 6. 留给 9→3 发布面合并批次（本批不做）

**AMB-01**（`core/`→`kernel/` + lib 退役 `xirang`）、**AMB-02**（`build_method/`→`build_time` 模块）、**AMB-03**（`run_method/` 改名 + `lexicon.rs:28 RUN_METHOD_CRATE` 四处）、**AMB-04**（方案模块名 `plugins`→`plugin_host`）、**AMB-07**（`debug_method/` 并入模块 `call_evidence`）、**AMB-08**（bin 名**已拍板保留**，只需文书）、**AMB-19**（feature 名保留 + README 对照表）。
依据：`docs/audit-2026-09-28/audit-publish-surface-merge-plan.md:17/:19/:21/:23/:24/:42-43` 与 `audit-naming-ambiguity-verify.md:62-67/:77-85/:104-115`。
**注意**：本批若先做 `NAM-*` 的文件改名，这些文件路径会出现在合并方案的替换表里（§4.2 `git mv`、§5.1 `shims` 路径）⇒ **合并批次开工前必须以本批后的树重算那张表**。

## 7. 复核命令（判别性 vs 计数用）

- **R-1 活文档集**（计数用）：`python3 - <<'P'`（照 `doc_blocks::markdown_files` 的口径：根 `*.md` + `docs/**` 递归 + `*/README*.md`，排除任一以 `audit`/`design` 开头的分量）⇒ 现算 **33 份**。
- **R-2 活文档是否提到某个旧名**（判别性：改名后必须 0）：`grep -rlF '<old-path>' <R-1 的 33 份>`。现算：13 个目标路径里只有 `authoring/validation` 命中 1 处（`docs/roadmap-1.0.md:157`）。
- **R-3 CHANGELOG/examples/tools**（判别性）：`grep -rlF '<old-path>' CHANGELOG.md examples tools .github` ⇒ 现算 **0**。
- **R-4 测试叶子名不变**（判别性）：改名前后各存一次 `cargo test -p <crate> --offline -- --list | sort`，`diff` 必须只差**模块前缀**；叶子名逐名不变（t82 已验证过同一手法）。
- **R-5 挂载四族与测试模块**（计数用）：`grep -rnE '^\s*(pub(\(crate\))? )?mod [a-z_]+;' <crate>/src | grep -v 'mod tests;'`；测试族：`grep -rn 'mod tests;' */src --include='*.rs'`（现算 `*_tests.rs` **39** 个文件、挂 `mod tests;` **4** 处，与审计的 44/4 有差，树已变动）。
- **R-6 A1 收平谓词**（判别性：发布面收平后为 0）：`find . -name '*.rs' -not -path './target/*' | python3 -c "…（目录内递归 .rs 集合 == {<dir>.rs}）"` ⇒ 现算 **35 / 发布面 26 / 豁免 9**（审计值 34/28/6）。
- **R-7 身份红线**（判别性）：`cargo test -p xirang-example-control-button --offline --test registry` 与 `cargo test -p xirang-example-broken-button --offline` 必须保持绿；`grep -rnF 'from_namespaced_path' build_method/src node_id 相关测试` 作为对照。

## 8. 状态漂移（本批开工前必须知道）

| 漂移 | 实证 |
| --- | --- |
| `NAM-02` / `NAM-06` **已应用** | `studio/studio/app/state/misc.rs`、`studio/studio/app/writers.rs` 均已不存在；替代文件 `state/{call_tree_view,reload,overlays}.rs`、`app/write_guard.rs` 在；`b5_studio.rs:114` 有负钉子 |
| `NAM-01` **部分应用** | `app/project_context.rs` 在、`app/support.rs` 只剩 2 行历史 shim、`app.rs:42/:47` 两个挂载并存 |
| `NAM-31` **命名半已应用** | `apply_trait_label`（`face_write.rs:207`）在，`apply_trait_contract` **0 命中** |
| `NAM-23` **已应用** | `plugin-host/src/lazy_wasm.rs:20-23` 已写明"定义本体在 kernel，本别名保留历史路径" |
| `NAM-19` **站点清单已漂移** | 报告点名的 11 个 `app/tests/*.rs` 文件现为 18 个（新增 `b4/b5/b6_studio.rs`、`project_root.rs`、`lock_writes.rs`…），`project.rs` 已不存在 |
| `NAM-07`/`NAM-43` **计数已漂移** | A1：审计 34/28/6 vs 现算 35/26/9；`*_tests.rs`：审计 44 vs 现算 39 |

## 9. 未覆盖 / 风险（如实）

- **未逐条跑红**：本单是"只出计划"，我没有对 50 条执行"改前 → 改后"的变异；§2/§3 的计数是**计划时读数**，执行者必须以 §7 的命令在动手指前重算（尤其漂移过的 4 条）。
- **未核 UGC 站点**：`NAM-32/33/34` 的"全量清单"来自审计的机器统计（24 行 / 直方图 / 25 行），我只抽核了其中 4 条的行号（`full`/`find`/`get`/`collect_pruning_symbols` 均命中）；执行前请按命令重算全量。
- **B8 的 `fix_hint` 是模板残句**（§3 开头）：任何"按 fix_hint 执行"的自动化都会做错——必须以 `audit-naming-ambiguity-verify.md` + `audit-publish-surface-merge-plan.md` 为准。
- **静置窗口的现实风险**：G4a/G4b 会碰 `conventions/src/shims.rs`，而 `missing_shims` 对缺失文件是 **panic**（不是 finding）——若有人在同一窗口改这些文件，门禁会以 `cannot read …` 的形态炸掉，与真实的改名失败难以区分。建议 G4b 开工前先在团队里声明"命名批次窗口"。
- **本文件自身**：位于记录目录内（`docs/audit-2026-09-28/`），被 `doc_blocks`/`doc_anchors` 豁免，因此它引用的 `file:line` 不会因树变动而变成门禁红——但**不保证**它们在未来仍然正确（记录的性质如此）。
