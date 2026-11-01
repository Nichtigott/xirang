# t27 独立复核：命名抽象专项（`audit-naming-review.md`，t25）

> 复核者 boundary-architect；本轮只出报告，源码一行不改。工作树 `HEAD=cf0c378`（未提交改动与 t25 同一批）。
> 复核对象：`docs/audit-2026-09-28/audit-naming-review.md`（374 行，作者 logic-adversary）。
> 本文只写复核结论与证据，**不重写 t25 的建议清单**（它已写得很细），只标出哪些成立、哪些要改。

## 0. 复核口径（我用了什么手段，与作者有何不同）

| 项 | 作者（t25） | 我（t27） |
| --- | --- | --- |
| 文件/目录/函数全量 | 自己写的两个 python 脚本读 `audit-inventory.json`（G-1/G-2） | **不重跑它的脚本**：直接按自己的谓词查 `audit-inventory.json` 的 `files[]`/`functions[]`（`module_path`、`is_test`、`name`），另用 `find`/`wc`/`grep` 直接在文件系统上复算 |
| 目录载荷 | 谓词写在散文里："目录里只有与目录同名的那个 `.rs`" | 用**三种谓词**分别算：递归（载荷为空）、直接子项、以及"文件 stem == 父目录名"，看哪一个给出 39 |
| 名字与行为不符 | 读被引行 | 读被引行 **+ 读整个函数/两份清单/调用方**；能 grep 反证的给出反证（如 `CallRefRing`、`optional_pruning_probe`） |
| 抽样率 | 文件内容 ~28/394、函数名 ~160/2462 | **MAJOR 7/7（100%）**、MINOR **17/21（81%）**、`§D` 11 条里 9 条可机械复算的逐条重算、`§F` 8 条干净项抽查 6 条 |
| 判定用词 | — | **证实 / 部分证实 / 证伪 / 无法判定**（"部分证实"= 现象成立但证据或计数有错；"部分证伪"= 论证前提错但结论方向可保留） |

**总体判定：`needs_revision`。** 7 条 MAJOR 的**现象**全部成立（没有一条是"我不喜欢这个名字"），但其中 **2 条的论证前提是错的**、**1 条 MAJOR 建议的方向与它自己的 D-9 冲突**、**3 处写在"干净/结论"里的数字不可复算或已被证伪**（其中 NAM-40 是明确证伪）。报告可用，但作为"命名权威"必须先修这几处。

---

## 1. MAJOR 逐条（7/7 全复核）

| id | 复核结论 | 证据（命令/行号，均已实测） | 严重度 | 备注 |
| --- | --- | --- | --- | --- |
| **NAM-01** `studio/app/support.rs` | **证实** | `wc -l` = 509 ✓；23 个 item ✓；它给的 12 个锚点**全部命中**（`:23 select_project`、`:49 package_namespace`、`:70 with_authoring_context`、`:82 selected_root`、`:100 clear_project_context`、`:111 package_root`、`:157 resolve_project`、`:251 host_manifest`、`:277 cargo_rustc_mir`、`:379 near_divider`、`:389 resize_graph_split`、`:459 open_editor_file`）；文件里另有 `struct ProjectContext`（`:10`）、私有 `usable`（`:229`）、`bin_target_names`（`:332`）、`impl App`（`:378`/`:468`）、`node_editor_fixture`（`:498`） | MAJOR 不变 | 三类主语（项目/环境上下文、cargo 子进程、TUI 几何）确实挂在一个无语义的名字下。它与 t25 自己的 D-1 一致；与 `t14 R-06` 同一动作 |
| **NAM-03** `run_method/authoring/validation/validation.rs` | **证实** | 文件 158 行 9 个 `fn`；`AuthoringContext` `:20`、`new` `:47`、`scope` `:56`、`authoring_namespace` `:71`、`legacy_rule_path_for_source` `:91`、`package_root` `:99`、`source_root` `:120` —— 行号逐一命中 | MAJOR 不变 | **作者漏了一点**：该文件被 `conventions/src/shims.rs:50` 作为**公开路径钉子**守着（`pub use xirang::authoring::validation::*;`），所以"模块名 `validation` → `context`"是**公开路径变更**，必须同时改棘轮或补别名。t25 只对 NAM-10 提了棘轮，这里漏了 |
| **NAM-04** `build_method/cache.rs` | **证实** | 254 行；8 个函数行号**全部命中**（`write_if_changed:36`、`collect_active_ids:44`、`update_discovery_cache:77`、`cached_parent_id:184`、`collect_discovery_rows:205`、`source_is_active:216`、`face_source_is_active:235`、`module_feature:246`）；调用方实测：`write_if_changed` 被 `pipeline.rs:156`/`:179` 与 `manifests.rs:41`/`:76`/`:95`/`:108` 用（通用 IO ✓）；`source_is_active`/`face_source_is_active`/`module_feature` 被 `build_method/renderer/tree.rs:65`、`build_method/static_plan.rs:84`、`build_method/validation.rs:24` 消费（发布作用域判定 ✓） | MAJOR 不变 | 判据（"作用域判定住在 cache 里"）由调用方实测坐实，比纯命名论证强 |
| **NAM-05** `mcp/evidence.rs` | **部分证伪**（论证前提错，结论方向保留） | 它说"17 个工具里 16 个各自住同名列，只有 `explain` 例外"。同一张 dispatch 表（`mcp/tools.rs:294-317`）实测：`inspect` `tools.rs:331`、`read_source` `tools.rs:356`、`status` `tools.rs:393`（**三个 handler 就在 tools.rs 里**）、`unified` `mcp/mir.rs:125`、`overlay` `mcp/overlay.rs:46` → 同名文件只有 **12–13/17**，例外是 **4–5 个**而不是 1 个。另外 `mcp/evidence.rs` 的模块文档首行是 "The build's own evidence for one face, or for the tree it scoped."（`:1`）——**名字与内容相符** | **MAJOR → MINOR** | 成立的只有两点：① 工具名 `xirang.explain` 与文件名 `evidence.rs` 不一致；② `evidence` 一词在本仓一名多义（与 `run_method/runtime/evidence.rs` 的"已观测调用证据"不同义）→ 属 D-9 的实例，不属"名与内容不符"。**并且它给的改名 `explain.rs` 与它自己的 D-9（保留词要加限定词，示例正是 `build_evidence`）互相矛盾**，二选一必须先定 |
| **NAM-06** `studio/app/writers.rs` | **部分证实，降级** | 42 行、2 个函数 ✓（`selected_package_root:30`、`with_selected_project:37`）；但模块文档首行是 "**Write guards**: every operation that creates, rewrites, moves, or deletes something in the project the reader opened."，第二段更进一步解释"本模块存在的意义就是读/写的区分：写入方都走 `with_selected_project`，它没有回落"。文件内容 = 一个 guard + 一个上下文包装，**与"guard"这个自述相符** | **MAJOR → MINOR** | t25（与 lane-studio C-02）把"文档说它是写入的**守卫**"读成了"文档自称**执行**全部写入"，这是把名字的褒义读进了文档。**同时否决它的修法**：把这两个函数并入 `mutations.rs` 会拆掉"写入必须经过无回落根"那道缝——文档第二段写明这正是本模块的存在理由。建议**改名**（如 `write_guard.rs`）而不是合并 |
| **NAM-30** `collect_pruning_symbols` / `parse_pruning_item` | **证实（强）** | `build_method/manifests.rs:173` 定义；`parse_pruning_item` 第三支 `:208-210` **无条件返回字面量** `"Button::optional_pruning_probe"`；`grep -rn optional_pruning_probe --include='*.rs' .` → **全仓只有 `:208`、`:210` 两处**，没有任何源码声明这个符号（它是被写进解析器的名字）；符号列由 `format!("{module}::{item}")` 组装后写进 `pruning_manifest.tsv` | MAJOR 不变 | 这是"名与行为不符"的最强形态（名说"收集"，行为"发明"），t25 与 lane-surfaces S3 独立命中同一处。行为修复属 t15 的 LG-15 |
| **NAM-31** `apply_trait_contract` | **证实** | 定义 `run_method/authoring/operations/face_write.rs:192-206`，函数体只调 `face.edit(label_field, &labels)`（labels 来自 `trait_names_from_paths(contract_paths)` 或作者写的 labels）；`CREATE_FIELD_ORDER`（`:33` 起 23 项）**含** `handle_contracts`/`part_contracts`，`EDIT_FIELD_ORDER`（`:60` 起 20 项）**不含**（两份清单逐行读出） | MAJOR 不变 | 注意它的**自带文档是对的**（"Write the label field from the paths when there are paths…"），错的是**函数名**。行为缺口属 t15 的 LG-21 |

---

## 2. MINOR 抽样（17/21，含所有可机械复算者）

| id | 复核结论 | 证据 | 严重度 | 备注 |
| --- | --- | --- | --- | --- |
| NAM-02 `state/misc.rs` | **部分证实** | 181 行 ✓、`push_call_ref:129` ✓、`source_path_for:161` ✓、`pub enum Overlay:28` ✓；但 **`CallRefRing` 这个类型在仓库里不存在**（`grep -rn CallRefRing --include='*.rs' studio/src` → 0 命中）；实际是 `push_call_ref(...)` 去重后推入 `Vec<CallRef>` | MINOR 不变 | `misc` 这个问题成立；但证据里的**标识符是错的**——独立复核时按这个名字去找会一无所获 |
| NAM-07 无载荷目录 | **部分证实（现象成立，数与谓词不符）** | 三种谓词复算：① 递归"目录内 `.rs` 集合 == {`<dir>.rs`}" → **34**（其中**十个发布 crate 内 28**）；② 只算直接子项 → **39**；③ 文件 stem == 父目录名 → **67**。①−② 的差是 **5 个含子模块的目录**（`core/src/registry_core/plugin`、`plugin/contracts`、`examples/control-button/src/control`、夹具 `.../control`、夹具 `.../node_editor`）——正是它自己写"**不要**动那些目录里还有子模块的"那类 | MINOR 不变 | 见 §3.1：**39 这个数配错了谓词**，而 D-3 的验收标准（"收平后应为 0"）必须用 34/28 的口径，否则实现出来永远不为 0 |
| NAM-08 两个 `call_trace.rs` | **证实** | 348 / 228 行 ✓；`locals/locals.rs` 18 行为挂载根 ✓ | MINOR 不变 | — |
| NAM-09 两个 `face_manifest.rs` | **证实** | 89 / 22 行 ✓ | MINOR 不变 | — |
| NAM-10 `parse/parse.rs` shim | **证实** | 54 / 264 行 ✓；`conventions/src/shims.rs:50` 与 `:71` 确实钉住了两处 shim 路径 → 作者"建议首行声明而不是改名"的取舍**与棘轮一致** | MINOR 不变 | 这是全篇最保守也最正确的一条建议 |
| NAM-11 保留词一名多义 | **证实** | `evidence` 3 处、`index` 2 处（+`studio/app/source_index.rs`）、`artifact` 2 处、`registry` 4 处；逐条与它的表一致 | MINOR 不变 | 与 D-9 的 4 处违规对应 ✓ |
| NAM-12 `search.rs`/`forms.rs` 多处 | **证实** | `find studio/src -name 'search*.rs' -o -name 'forms*.rs'` → 7 个文件，分布与它的描述一致 | MINOR 不变 | — |
| NAM-17 `faces.rs` vs `face_view.rs` | **证实** | 96 / 444 行 ✓ | MINOR 不变 | — |
| NAM-18 `node.rs` vs `node_id.rs` | **证实** | 26 / 117 行 ✓ | MINOR 不变 | — |
| NAM-19 `covers` 行 | **证实** | studio `app/tests/` 11 个文件；除 `fixtures.rs`（夹具不是测试）外 **10 个缺 `covers` 行** = 作者给的 10 ✓ | MINOR 不变 | — |
| NAM-22 `mcp/nodes.rs` | **证实** | 76 行 ✓；`parent_id:27` ✓、`resolve_node:58` ✓ | MINOR 不变 | — |
| NAM-23 `ValidationChannel` 别名 | **证实** | `plugin-host/src/lazy_wasm.rs:24` ✓；全仓真 `pub use … as` **4 处**（第 5 个 `grep` 命中是 `build_method/renderer/pass.rs:346` 的**断言字符串**） | MINOR 不变 | 4 这个数对，但用 grep 复算时会看到 5 行，需注意 |
| NAM-32 裸动词 | **部分证实（清单不全）** | 它列的 8 处**全部成立**；我独立复算另有 ≥6 处同类：`core/src/registry_core/plugin/plugin_policy/plugin_policy.rs:55 fn load(`、`plugin-host/src/process.rs:146 pub fn load(`、`plugin-host/src/wasm.rs:149 pub fn load(`、`studio/studio/app/lifecycle.rs:79 pub fn load(`、`studio/studio/ui/graph/node_graph.rs:36 pub(super) fn build(`；另 `entry_pages.rs:43 get_mut` | MINOR 不变 | D-5 的"现状 8 处"**偏低**（我的口径 ≥14，含 `run`×4、`load`×6、`build`×2…，白名单边界仍需一次裁定） |
| NAM-33 动词收敛表 | **证实（数值全对）** | 我按 `audit-inventory.json` 的 `functions[]` 独立算首词：`collect 39`、`parse 38`、`render 38`、`resolve 20`、`validate 19`、`read 17`、`load 16`、`find 7`、`get 2` —— 与它的表**逐个相等**；`query/lookup/fetch` 确为 0 | MINOR 不变 | 一处小出入：`get 2` 实为 `get`（`entry_pages.rs:39`）+ `get_mut`（`:43`），后者是标准容器名，不该算违规 |
| NAM-34 `_owned` 家族 | **部分证实（数偏小）** | 它点的名字都成立；我复算 `_owned/_raw/_strings` 一类另有：`quoted_strings`、`is_xirang_owned_source`、`rule_method_strings`、`argv_strings`（`into_owned`/`from_owned`/`from_raw` 属惯用语，已排除） | MINOR 不变 | D-7 的"5 违规"应为 **~8** |
| NAM-35 三处裸形容词谓词 | **证实** | `release.rs:191 pub const fn full(self) -> bool` ✓、`source/walk.rs:54 fn skips(&self, path: &Path) -> bool` ✓、`plugin/contracts/contracts.rs:460 pub fn targets(self, framework: FrameworkId) -> bool` ✓ —— 行号与签名逐一命中 | MINOR 不变 | D-6 的机械谓词里"或带宾语"这一条不可机械判定，落地时要么改成纯前缀白名单（违规数会上升），要么人工维护清单 |
| NAM-36 `Registry::find` 家族 | **证实（部分）** | `core/src/registry_core/tree/query/query.rs:80 pub fn find(&self, id)` 确无宾语 ✓；同文件 `find_kind:146`、`find_where:154` ✓；三个内部递归 helper 同文件 `collect_node_path:105`、`collect_kind:164`、`collect_depth_first:183` 都是私有 `fn` ✓（借了公开动词） | MINOR 不变 | D-4 的"内部 `collect_*` 8 处"**没有给清单**，我只能证实 `core/src/registry_core/tree/query/query.rs` 的这 3 处 → 该数标"无法判定（缺清单）" |
| NAM-37 `EntryPages::get`/`remove` | **证实（+一处遗漏）** | `entry_pages.rs:39 pub(super) fn get`、`:74 pub(super) fn remove` ✓ | MINOR 不变 | 遗漏 `:43 get_mut`（与 NAM-33 同一处） |
| NAM-40 模块路径 ↔ 文件名 | **证伪** | 它说"394/394 一致、0 违规、唯二有意例外"。按**它自己的输入** `audit-inventory.json` 复算：**14 个文件的 `module_path` 末段 ≠ 文件 stem** —— `build_method/{identity,syntax}.rs` → `registry_identity`/`registry_syntax`；`cli/commands/*.rs` → `*_command`（5 个）；`cli/explain_{json,overlay,report}.rs` → `explain::{json,overlay,report}`（3 个）；`cli/lib_tests.rs` 与 core 的 3 个 `*_tests.rs` → `::tests`（4 个）。它举的两个"有意例外"（`core/src/registry_core/plugin/plugin.rs:35` 的 `graft_document`、`run_method/lib.rs:49-50` 的宏别名）其实是**重导出别名**，根本不进 `module_path` | **结论推翻**（本身是"干净项"，无严重度） | 见 §3.2：这是一条写在"干净"里的错误结论，而 §F 第 2 条与 D-10/D-11 都引它 |
| NAM-41 别名纪律 | **证实** | 4 处，3 处有理由 ✓ | MINOR 不变 | — |
| NAM-42 宏生成模块名 | **部分（未复核生成物）** | 我只确认了机制存在（`__xirang_ra_` 前缀在 `build_method` 的渲染与断言里）；**没有编译产物可查**（本轮不跑 cargo，且 `examples/*/target/xirang/out/generated_lib.rs` 是旧产物） | 维持"干净"（未证伪） | 作者引的是 `audit-inventory.json` 的 `generated_module_names`，那是构建产物文本，属可信来源 |

---

## 3. 三处必须记录的方法学问题

### 3.1 NAM-07 / D-3 的"39"配错了谓词（部分证伪）

t25 的散文谓词是"目录里只有与目录同名的那个 `.rs`（**无子模块、无测试**）"，它自己建议的门禁判定是"目录内文件集合恰为 `{<dir>.rs}`"，而 G-1 的脚本谓词是"文件 stem == 父目录名"。三者给出**三个不同的数**：

| 谓词 | 数 | 说明 |
| --- | ---: | --- |
| 递归：目录内 `.rs` 集合 == {`<dir>.rs`}（= 散文与 D-3 的门禁） | **34** | 其中**十个发布 crate 内 28**；另 6 个在两个 example 宿主与 Studio 夹具里 |
| 只算直接子项 == {`<dir>.rs`} | **39** | **多出的 5 个含子模块**：`core/src/registry_core/plugin`、`plugin/contracts`、`examples/control-button/src/control`、夹具 `.../control`、夹具 `.../node_editor` |
| 文件 stem == 父目录名（= t25 G-1 脚本） | **67** | 与"无载荷"无关，只是名字重复 |

**后果**：D-3 若按原文落地，"收平后违规应为 0"这条验收**永远不可能达成**（那 5 个含子模块的目录按约定必须保留，而松散谓词会把它们继续报成违规）；反之若按 34/28 落地，则与它自己的散文一致。→ 建议：把 NAM-07/D-3 的数改成 **34（仓库）/ 28（发布面）**，并把谓词写成"递归"。

### 3.2 NAM-40"394/394 一致、0 违规"被证伪（结论级）

按 t25 的输入文件复算得 **14 处不一致**，且分四类（见 §2 的 NAM-40 行）。它举的两个"例外"是重导出别名而非模块名，说明它把"重导出语句的形状"当成了"模块名的形状"。这不是抽样误差，是**谓词选错**：`module_path` 的末段由**挂载点的 `mod <name>;` 名字**决定，而本仓大量使用重命名挂载（`#[path="…/validation.rs"] mod context;` 是它建议的改法，`mod tests;`、`mod build_command;` 是现状）。

**影响面**：§F 第 2 条（"干净项"）、D-10（"测试文件镜像生产文件"）、D-11（跨 crate 同名）都以"模块名 == 文件 stem"为前提。修正后：**文件名**镜像成立（43/44 ✓，我复算一致），**模块名**不成立（4 处 `*_tests.rs` 挂成 `mod tests`）。

### 3.3 NAM-05 的"16/17 同名"前提被证伪（见 §1 的 NAM-05 行）

它用"17 个工具 16 个同名、explain 是唯一例外"支撑 MAJOR。实测同名者 12–13/17，`tools.rs` 自己装 3 个 handler、`mir.rs` 装 2 个。**结论方向（explain 与 evidence 的名字错位、`evidence` 一名多义）仍成立**，但严重度应按它的判据降到 MINOR，且改名方向要在 `explain.rs`（NAM-05）与 `build_evidence.rs`（D-9）之间二选一。

---

## 4. §D 的 11 条规则：谓词 vs 现状数

| 规则 | 我复算的现状 | 结论 |
| --- | --- | --- |
| D-1 类别名黑名单 | **2**（`studio/app/{misc,support}.rs`）；`utils/helpers/common/shared/types/base/general/stuff` 全仓 0 命中 | ✅ 与作者一致 |
| D-2 一文件一主语 | 不可机械判定（它自己标"人工"）；抽样 3 例（NAM-01/03/04）我复核成立 | ✅ 一致（口径诚实） |
| D-3 无载荷目录收平 | **34 / 28**，不是 39 | ❌ 数错（见 §3.1） |
| D-4 动词白名单 | `get` 2（实为 `get`+`get_mut`）；"内部 `collect_*` 8 处"**没给清单** | ⚠️ 部分（get 一方；8 无法判定） |
| D-5 裸动词只许入口位 | 它列 8 处成立，我另数到 ≥6 处未列 | ❌ 数偏低 |
| D-6 布尔谓词 | 3 处成立；但"或带宾语"不可机械判定 | ⚠️ 部分 |
| D-7 意图不写实现 | `_owned` 一族 3 + `_strings` 一类 4 ≈ **8** | ❌ 数偏低 |
| D-8 别名同名同义 | 4 处别名，1 处（`ValidationChannel`）需补文档 | ✅ 一致 |
| D-9 保留词表 | 4 个词各自一名多义：`evidence`（`mcp/evidence.rs` × `run_method/runtime/evidence.rs` × `studio/.../tests/evidence.rs`）、`index`（`core/.../tree/index/index.rs` × `mcp/index.rs`）、`artifact`（`core/.../plugin/artifact/` × `run_method/runtime/trace/artifact/`）、`registry`（4 处：`core/.../tree/registry.rs`、`run_method/registry/registry.rs`、`mcp/registry.rs`、`examples/control-button/tests/registry.rs`） | ✅ 一致（实例逐条命中） |
| D-10 类型单数/模块复数、测试镜像、`covers` | 43/44 成对 ✅（我复算相同）；`covers` 缺 10 ✅ | ✅ 一致；但"模块名镜像"被 NAM-40 证伪，规则文本要收敛到"文件名镜像" |
| D-11 跨 crate 同名词同义 | "抽样 12 组里 6 组语义不同"——**没有给 12 组清单** | ⚠️ 无法判定（缺清单） |

---

## 5. 作者漏掉的同类问题（独立发现）

1. **`module_path` 的 14 处不一致**（NAM-40 的盲区，§3.2 已列）。其中 `build_method/{identity,syntax}.rs` 的实际模块名是 `registry_identity`/`registry_syntax` —— 与 t14 的 R-15（"从文件列表看不出谁是 shim"）**同一处**，两份报告都该指向它，但 t25 把它算成了"干净"。
2. **`cli/commands/*.rs` → `mod *_command`（5 个）与 `cli/explain_*.rs` → `explain::{json,overlay,report}`（3 个）**：同属"文件 stem 与模块名不一致"的家族，完全没有被提到。
3. **`*_tests.rs` 挂成 `mod tests`**（4 处 + 全仓约定）：与它自己的 D-10"测试文件镜像生产文件"直接相邻，却没写下"镜像的是文件名不是模块名"。
4. **NAM-03 的改名会撞 `conventions/src/shims.rs:50` 的公开路径钉子**（作者只对 NAM-10 提了棘轮）。凡涉及 `run_method/{authoring/validation,runtime/evidence,registry,plugin}` 的改名，都要先看那张表。
5. **`CallRefRing` 不存在**（NAM-02 的证据引了一个仓库里没有的标识符）。
6. **`entry_pages::get_mut`**（NAM-33/NAM-37 都漏了它，导致 `get 2 处` 的口径含混）。
7. **额外裸动词**：`plugin_policy.rs:55`、`plugin-host/src/{process,wasm}.rs` 与 `studio/app/lifecycle.rs:79` 的 `load`，`studio/studio/ui/graph/node_graph.rs:36` 的 `build`（D-5 的清单因此不全）。

---

## 6. §F「查了、干净」的诚实性

**过程披露：诚实。** §0 的表格把"机器枚举 vs 人读"分成两列，§F 明写"**没有做的事**（不要当成已查）"：394 个文件只读了 ~28 个内容、2462 个函数名只人读了 ~160 个、没有做历史考古。我在复核中没有发现任何"把机器统计冒充逐个人读"的表述——这条比本仓其他几份报告做得好，应当保留这个体例。

**结论层口径：偏松，且恰好都落在"干净/一致"那一侧。** 三处不可复算或已证伪的数字都在"结论"里，而不是在"抽样"里：

| 位置 | 声称 | 实测 |
| --- | --- | --- |
| §0 覆盖表 | "全部 **68 个目录**" | 我按"394 个文件所在的**不同目录**"算得 **105**；68 的谓词未给出 → **无法复算** |
| §F 第 2 条 | "module_path ↔ 文件名 **394/394** 一致，0 违规" | **14 处不一致** → 证伪（§3.2） |
| NAM-07 / D-3 | "**39** 个无载荷目录" | 34（递归）/ 28（发布面）→ 谓词配错（§3.1） |
| NAM-32 / D-5 | "非入口位 **8 处**" | ≥14（我数到的） → 偏低 |
| NAM-34 / D-7 | "**5 违规**" | ~8 → 偏低 |

也就是说：**"哪些没查"披露得很清楚，"查完的结论"里却有 5 处数字需要修正**。建议下一轮把"干净项"也写成"谓词 + 命令 + 输出"，与 §0 的披露标准对齐。

---

## 7. 我这一轮没有覆盖的（不要当成已复核）

- 没有重跑作者的 G-2/G-3/G-4 脚本（按任务要求用不同手段），因此对**函数语义**的判断只做了：MAJOR 两条（NAM-30/31）读完整函数 + 两份清单；NAM-32/33/34/35/36/37 用清单/直方图复算 + 逐条看签名。
- 没有复核 `audit-inventory.json` 本身的正确性（那是 t1 的交付；我把它当输入，另用 `find`/`wc`/`grep` 在文件系统上交叉验证了其中被引用的**行数与存在性**，全部一致）。
- 没有评估 §D 规则本身的**好坏**（那是维护者的表决），只复核"谓词 → 数"的可复算性与陈述准确性。
- NAM-42（宏生成模块名）与 NAM-20（平铺目录）未做独立验证：前者需要编译产物，后者是结构问题（已由 t7/t14 覆盖）。

---

## 8. 对下游的接口影响（与 t26/t14 联动）

1. **t26 §10** 引用了 t25 的 `D-3` 与 `D-9`：`D-9` 的实例成立（`evidence` 一名多义），但 `D-3` 的数要按 §3.1 改成 34/28，且"收平"与"七合一"的先后顺序建议不变。
2. **`NAM-05` 的改名方向必须与 `D-9` 统一**：`explain.rs`（工具名）与 `build_evidence.rs`（保留词加限定）二者选一；我倾向后者，因为该文件的内容（`out_dir`/`build_evidence`/`node_report`/`scope_line`/`pruning_line`/`tree_report`）绝大多数是**构建证据读取器**，`explain` 只是其中一个 handler。
3. **NAM-40 的"0 违规"不能用于支撑 D-10/D-11**；若维护者要落 D-10，规则文本应写成"**文件名**镜像生产文件；模块名统一 `mod tests`（现状已如此）"。
4. **凡涉及 `run_method/{authoring/validation,runtime/evidence,registry,plugin}` 的改名**，都要先读 `conventions/src/shims.rs` 的 `SHIMS` 表（`:27` 起，16 条）——那是公开路径的棘轮。
