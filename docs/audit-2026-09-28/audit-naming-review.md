# 命名抽象专项：文件名 / 目录名 / 模块名 / 函数名（audit-naming-review）

- 任务：t25。本轮**只出报告，源码一行未改**；写权限只落在本文件。
- 输入：`docs/audit-2026-09-28/audit-inventory.json`（394 个 `.rs`：387 工作区成员 + 7 夹具；2462 个 fn：1433 生产 + 1029 测试）、`audit-structure-base.md` 的目录职责表、五路 lane 报告里的命名条目。
- 对象工作树：`HEAD=cf0c378`（120 项未提交改动）。
- 严重度口径（沿用任务书）：**MAJOR = 名字误导读者或与实际行为不符**；**MINOR = 名字可读性不足但不误导**。
- 每条格式：`id / 严重度 / 类别 / file:line / 现状 / 判据 / 最小改法（旧→新，具体到名字） / 复核手段`。

## 修订记录（t30：按 t27 的 9 条 findings 逐条关闭）

> t27（`audit-naming-verify.md`）对第一版判 `needs_revision`。下表是**逐条处置**；凡我不同意的，给出反证或改写理由。

| t27 finding | 处置 | 本报告位置 |
| --- | --- | --- |
| ① NAM-40「394/394 一致、0 违规」被证伪 | **照改**：谓词写清为"`module_path` 非空 ∧ 末段 ≠ 文件 stem"，输出 **14 处**（四族）；承认原两个"有意例外"其实是**重导出别名**、不进 `module_path`；D-10/D-11 与 §F 已去掉该前提 | NAM-40、D-10、D-11、§F F-2 |
| ② D-3 的「39」配错谓词 | **照改**：统一为**递归谓词 P2d** → **34（= 发布面 28 + 6 处豁免，见 ⑩）**；把 39（inventory 直接 `.rs` 口径，含 5 个有子模块目录）与 67（文件名重复口径）两者标为**不再使用**并解释来源 | NAM-07、D-3、§G 的"三处口径" |
| ③ NAM-05 降 MINOR ＋ 改写 ＋ 与 D-9 二选一 | **照改**：降为 MINOR；补上正确计数 **12/17 同名**（5 个例外：`tools.rs` 装 3 个 handler、`unified` 在 `mir.rs`、`explain` 在 `evidence.rs`）；承认 `evidence.rs` 首行文档与内容相符；**采纳 t27 的倾向**，改名采用 `build_evidence.rs`（与 D-9 一致），并注明"若按一工具一文件则拆两半"是另一条路 | NAM-05、NAM-11③、D-9 |
| ④ NAM-06 降 MINOR ＋ 改修法 | **照改**：降为 MINOR；承认原文"自称拥有全部写入"是把名字的褒义读进文档（首行是 `Write guards`，第二段说明它没有回落根）；**撤回**"并入 `mutations.rs`"，改为**改名 `write_guard.rs`** | NAM-06、§E 的 STU-S-14 行 |
| ⑤ 计数偏低/证据错误/口径缺口 | **照改并给全量清单**：D-5 → 全量 24 行清单、**非入口位 15 处**；D-7 → 全量 25 行、计入 **8 行**（并给 17 行的排除理由）；NAM-02 的 `CallRefRing` 改为真实类型 `CallRef`/`CallTreeView`；§0 的 68 → **105**（给谓词） | NAM-32、NAM-34、NAM-02、§0 |
| ⑥ 漏项 | **照补**：NAM-03 加 ⚠️"公开路径变更：`conventions/src/shims.rs:50`"；新增 **NAM-43**（模块挂载改名四家族 + 全仓 `*_tests.rs` 挂载不统一）；NAM-37/NAM-33 补 `core/src/registry_core/tree/entry_pages/entry_pages.rs:43 get_mut` | NAM-03、NAM-43、NAM-37 |
| ⑦ 口径纪律（"干净"也要写谓词+命令+输出） | **照改**：§F 全部改为"谓词（P 编号）+ 命令 + 输出"的表；§G 重写为 P1–P8 可复算脚本并附实测输出 | §F、§G |
| ⑧ D-4 的「内部 `collect_*` 8 处」缺清单；D-6「或带宾语」不可机械判定；D-11「12 组里 6 组」缺清单 | **照改**：D-4 只保留已确证的 3 处（`core/src/registry_core/tree/query/query.rs:105`、`:164`、`:183`）并标"待清单"；D-6 改成**纯前缀白名单**口径并注明落地后违规数会上升；D-11 改为"已确证 4 组（`call_trace.rs`、`face_manifest.rs`、保留词、`faces/face_view`），其余待清单" | §D 的 D-4/D-6/D-11 |
| ⑨ §F/F-2 与 D-10/D-11 不得再引用"394/394" | **照改**：§F F-2 拆成"文件名镜像（43/44）"与"模块名不镜像（14 处有意）"两句；D-10 的判定改成"文件 stem 比对（不是 `module_path`）" | §F F-2、D-10 |

| ⑩ t31 §C.1：D-3 需范围豁免（身份风险） | **照补**：34 = 发布面 28 + `examples/`/`studio/tests/fixtures/` 6（豁免）；D-3 的谓词与验收加上"∧ 路径不以 `examples/`/`studio/tests/` 开头"，三个口径（仓库 34 / 发布面 28 / 豁免后可达 0）写进 D-3、NAM-07 与 §G 的"三处口径"；**落地顺序改**：D-3 不能"一次机械完成"，要先按前缀分流只做发布面。同时把机制说准：类型化 `static_graft_plan!` 与面同编译**不会失效**，失效的是按身份落盘/按路径字面引用的东西（`registry_rule_path` 字符串、用户 checkout 的 `graft.plan`、逐字引用路径的测试与 README） | NAM-07 的豁免表、D-3、§G 的"三处口径"、§D 落地顺序 |
| ⑪ t31 §C.2：谓词标号撞名 | **照改**：NAM-07 的三档改名为 **A1/A2/A3**，并在表内与 §G 交叉注明"§G 的 `P2b`（`os.listdir` 单条目 → 34）**不是** A2（inventory 直接 `.rs` → 39）"；§G 的 P 编号说明行也加了同样一句 | NAM-07 三档表、§G-1 的 P2b 行、§G 的"三处口径" |
| ⑫ t31：D-9 落地名裁定 | **照改**：D-9 行的备注、NAM-05 与 NAM-11③ 都写明 `debug_method` 模块名 = **`call_evidence`**（特性名仍 `evidence`），**`build_evidence` 只指 mcp 侧文件**，两者不混用 | D-9、NAM-05、NAM-11③ |

**我明确不同意的**：无。9 条全部照改（其中③的 `explain.rs`/`build_evidence.rs` 二选一，我选 t27 的倾向并保留另一条作为备选；④的"合并"我撤回，因为 t27 给出的"守卫缝"反证成立）。

## 0. 覆盖口径（先把"查了什么"说清）

| 层次 | 范围 | 方法 |
| --- | --- | --- |
| 文件/目录名 | **全量 394 个 `.rs` + 全部 105 个目录**（谓词：394 个文件所在的**不同父目录**，命令见 §G-1 P3） | 机器枚举（`audit-inventory.json` 的 `path`/`lines`/`module_path`/`fn_count`/`mounted_by`）＋ 模式匹配（类别名词表、`<dir>/<dir>.rs`、跨 crate 同名、单文件目录） |
| 文件内容判断 | **抽样约 28 个文件**（下文每条点名） | 人读模块文档 + 前 10–15 行 + 全部 item 列表；`support.rs`/`cache.rs`/`query.rs`/`misc.rs` 等读了 15–25 行或全部签名 |
| 函数名 | **全量 2462 个** | 机器枚举（`name`/`kind`/`container`/`file`/`line`/`visibility`/`is_test`）＋ 五组过滤器（裸动词、首词动词直方图、实现细节词根、`-> bool` 谓词、长/短名） |
| 函数语义判断 | **约 160 个被过滤器命中的名字逐个人读**；其余 ~1270 个生产名**只过了机器过滤**，未逐个人读 | 读签名与（必要时）函数体；`grep` 取全量 `-> bool` 与 `pub use … as …` |
| 模块名 | **全量** | 机器比对 `module_path` 末段 vs 文件 stem；`mounted_by`；全仓 `pub use … as` |

**没有做的事**（不要当成已查）：没有逐个人读 394 个文件的内容（只读了上表那 ~28 个）；没有对每个函数名判断语义（只判断了过滤器命中者）；没有跨平台/跨版本的历史名字考古（只按当前工作树判断）。

---

## A. 文件名与目录名的抽象

### A① 类别名而不是东西名

**NAM-01 / MAJOR / A①③ / `studio/studio/app/support.rs`**
- **现状**：文件 509 行、23 个函数。模块文档第 1 行写「Shared interaction geometry and editor helpers」（共享交互几何与编辑器辅助），但前 12 个 item 是**项目上下文与 cargo 子进程**：`select_project`（:23）、`package_namespace`（:49）、`with_authoring_context`（:70）、`selected_root`（:82）、`clear_project_context`（:100）、`package_root`（:111）、`resolve_project`（:157）、`host_manifest`（:251）、`cargo_rustc_mir`（:277）；后面才是 TUI 几何与编辑器（`near_divider`:379、`resize_graph_split`:389、`open_editor_file`:459）。
- **判据**：三类互不相关的主语（项目/命名空间环境、cargo 调用、TUI 几何拖动）挂在一个叫 `support` 的名字下；`support` 不表达任何一类。读者要找「谁在解析项目根」不会打开 `support.rs`（我在 t8 分析 `package_root` 时就必须先 grep）。名字与内容不符 → MAJOR。
- **最小改法**：`studio/studio/app/support.rs` → `studio/studio/app/project_context.rs`；把几何相关（`near_divider`/`near_graph_divider`/`resize_graph_split`/`resize_split`/`move_selection`/`toggle_selected`/`selected_parent`）并入已有的 `studio/studio/app/hot_zones.rs`（该文件本身就叫热区）；`open_editor_at`/`open_editor_file`/`graph_locals` 归入 `ui/` 的编辑器入口或新建 `studio/studio/app/editor_launch.rs`。
- **复核手段**：`grep -n "fn " studio/studio/app/support.rs` 数三类 item 的比例；改后 `support.rs` 应不存在。

**NAM-02 / MINOR / A① / `studio/studio/app/state/misc.rs`**
- **现状**：181 行，模块文档自陈「Ungrouped Studio state: reload errors, overlays, and call references」（**未归类**的 Studio 状态）。内容是 `CallRef`（:55）与 `CallTreeView`（:76，`refs: Vec<Option<CallRef>>`，另有 `len`/`is_empty`/`item`）、`push_call_ref`（:129）、`pub enum Overlay`（:28）、`source_path_for`（:161）+ 重载错误状态。
- **判据**：`misc` 是典型类别名，读者无法据此找到任何东西；文件里有一个**清楚的主语**（`CallTreeView`/`CallRef` 一族）却没写进名字。不误导（文档承认它未归类），故 MINOR。
  > **t27 复核后的更正**：本节原写的类型名 `CallRefRing` **在本仓不存在**（`grep -rn CallRefRing --include='*.rs' .` → 0 命中）；实际类型是 `CallRef`（`studio/studio/app/state/misc.rs:55`）与 `CallTreeView`（`:76`）。原名证据引了一个不存在的标识符，已改正。
- **最小改法**：`state/misc.rs` → 按主语拆：`state/call_tree_view.rs`（`CallTreeView` + `CallRef` + `push_call_ref`）、`state/reload.rs`（重载错误）、`state/overlays.rs`（`Overlay` 状态）；`source_path_for` 归 `support.rs`（将来的 `project_context.rs`）。
- **复核手段**：`grep -n "struct CallRef\b\|struct CallTreeView\|fn push_call_ref\|enum Overlay\|reload_error" studio/studio/app/state/misc.rs`（输出：55 / 76 / 129 / 28）。

**A① 全量结果（机器 + 人读）**：类别名候选只有 5 个（都在 studio）：上两条 + `studio/studio/ui/forms.rs`（32 行，**合法**：它是 `ui/forms/` 的挂载根，只做重导出，名字与目录一致）、`studio/studio/app/state/forms.rs`（合法：表单状态，263 行，主语明确）、`studio/studio/app/tests/forms.rs`（合法：覆盖 add/edit 表单状态与字段词典，文档首行已写明）。`utils/common/helpers/shared/types/base` 在本仓**零命中**（干净，见 F）。

### A② 名字与父目录重复

**NAM-07 / MINOR / A② / 全仓 34 处 `<x>/<x>.rs` 且该目录**递归**只有这一个 `.rs`**

> **t27 复核后的修订（谓词口径）**：本节原写"39 处"，用的是更松的谓词。三档谓词实测如下（命令与输出见 §G-1）：
>
> | 谓词（精确写法） | 结果 |
> | --- | --- |
> | **A1（本报告采用，= §G-1 的 `P2d`）**：目录下**递归**所有 `.rs` 集合恰为 `{<dir>.rs}` | **34** = 发布面 **28** + `examples/` 与 `studio/tests/fixtures/` 里 **6** |
> | **A2（与 §G-1 的 `P2b` 不是同一谓词）**：目录内**直接 `.rs`（取自 inventory 记录）**恰为 `{<dir>.rs}` | 39 —— 多出的 5 个目录**含子模块**（`core/src/registry_core/plugin`、`plugin/contracts`、`examples/control-button/src/control`、夹具 `.../control`、夹具 `.../node_editor`），正是下文写"**不要动**"的那类。§G-1 的 `P2b` 用 `os.listdir`（全部条目，不只是 `.rs`），给 **34**；两个名字相近但**结果不同**，互不引用 |
> | **A3**：文件 stem == 父目录名（与"无载荷"无关） | 67 个文件 |
>
> 下文与 D-3 一律用谓词 **A1**；验收标准是"**发布面内**按 A1 收平后为 0"，不再使用 39。
>
> **范围豁免（t32 补；t31 §C.1 提出）**：34 = 发布面 28 + 下表 6 处，后者**不参与收平**——它们不是"无载荷目录"，而是**身份与路径都被别处按字面引用**的宿主源码：
>
> | 豁免路径 | 为什么不能移动（收平 = 移动文件 = 改相对源码路径 = 改 `NodeId`） |
> | --- | --- |
> | `examples/control-button/src/control/object/button/` | `button.rs` 是注册面；`examples/control-button/src/lib.rs:50` 的 `cut(crate::control::object::button::NODE_ID)` 与它同源。本仓库当前无落盘 `graft.plan`（`find . -name '*.plan'` 无命中），但 example 是给宿主复制的模板：用户侧按旧身份落盘的记录会不再解析 |
> | `examples/control-button/src/control/object/slider/` | 同上（`examples/control-button/src/lib.rs:52` 的 `cut(crate::control::object::slider::NODE_ID)`） |
> | `examples/control-button/src/control/registry_rule/` | 被**路径字符串**引用：`examples/control-button/src/control/control.rs:28` 的 `registry_rule_path: "src/control/registry_rule/registry_rule.rs"`；移动后该字符串与 `source_layout` 解析都要同改 |
> | `studio/tests/fixtures/node-editor/src/control/registry_rule/` | 同上（Studio 夹具的 `registry_rule_path` 字符串，见 `studio/studio/app/tests/edit.rs:20`、`studio/studio/app/tests/trace_ingest.rs:29`） |
> | `studio/tests/fixtures/node-editor/src/control/object/node_editor/registry_rule/` | 同上 |
> | `studio/tests/fixtures/node-editor/src/control/object/node_editor/object/` | 夹具面文件；被 `prototype-fixtures` 门控的测试按路径选中（`studio/tests/fixtures/` 不在 workspace members 里，见 `AGENTS.md` 的 `prototype-fixtures` 段） |
>
> **精确说法（不要把机制说过头）**：类型化 `static_graft_plan!`（`examples/control-button/src/lib.rs:50`、`:52`）与面**一起编译**，取值会跟着新身份走——**它本身不会"失效"**。真正会失效/需要连带改的是**以身份字符串落盘或按路径字面引用**的东西：用户 checkout 里的 `.xirang/external-grafts/*/graft.plan`、`registry_rule_path` 字符串、逐字引用 `control/object/button/button.rs` 的测试与 README（`mcp/apply_tests.rs:168`、`mcp/README.md:131`、`mcp/README.zh-CN.md:89`）。所以豁免理由是"移动会改身份 + 要连带改一串字面路径"。
下文与 D-3 一律用谓词 **A**（34 / 发布面 28）；验收标准是"按 A 收平后为 0"，不再使用 39。
**保留结论**：`tree/connector/connector.rs`、`plugin/contracts/contracts.rs` 这类**名字重复本身不是缺陷**——它们是约定的一部分，且 `tree/connector/` 等目录里另有测试或子模块时正是该约定的用途。要修的是「目录无载荷」这一半（NAM-07），不是名字本身。

### A③ 名与内容不符（误导 → MAJOR）

**NAM-03 / MAJOR / A③ / `run_method/authoring/validation/validation.rs`**
- **现状**：文件叫 `validation`，内容是 `AuthoringContext`（:47 的 `new(package_root, namespace)`、:56 的 `scope(...)`）与**进程级环境访问器** `authoring_namespace()`（:71）、`package_root()`（:99）、`source_root()`（:120）、`legacy_rule_path_for_source()`（:91）。
- **判据**：这是本仓 authoring 的**执行上下文与全局状态**实现，不是校验逻辑（真正的校验在 `core/src/registry_core/authoring/validation/validation.rs` 与 `run_method/authoring/manifest/parse`）。读者按名字找校验规则会打开错的文件；反过来，想知道「`package_root()` 是谁设的、作用域何时恢复」的人在 `validation` 下也想不到。名字与内容不符 → MAJOR。
- **最小改法**：`run_method/authoring/validation/validation.rs` → `run_method/authoring/context.rs`（或 `authoring_context.rs`），模块名 `validation` → `context`。
- **⚠️ 这是公开路径变更（t27 补的漏项）**：`conventions/src/shims.rs:50` 把 `run_method/authoring/validation/validation.rs` 的重导出 `pub use xirang::authoring::validation::*;` 钉在 `SHIMS` 棘轮里——也就是说 `xirang_run_method::authoring::validation` 是**下游可写的公开路径**。同一张表（`:27-80`）还钉着 `authoring/parse/parse.rs`、`authoring/face_manifest.rs`、`runtime/evidence.rs`、`runtime/runtime.rs`、`runtime/trace/trace.rs`、`plugin/plugin.rs`、`registry/registry.rs`、`run_method/lib.rs`。**凡改这些文件/模块名，必须同时改 `SHIMS` 条目并保留历史路径别名**，否则要么门禁红，要么悄悄移除下游仍在用的路径。
- **复核手段**：`grep -n "authoring/validation/validation.rs" conventions/src/shims.rs`（→ :50）；`grep -n "pub use xirang::authoring::validation" run_method/authoring/validation/validation.rs`；改名后 `cargo test -p xirang-conventions --offline`（`shims` 门禁）必须仍绿。

**NAM-04 / MAJOR / A③ / `build_method/cache.rs`**
- **现状**：文件叫 `cache`，8 个函数里只有一半是缓存：`update_discovery_cache`（:77）、`cached_parent_id`（:184）、`collect_discovery_rows`（:205）；另一半是**作用域判定**与**通用写盘**：`write_if_changed`（:36，全库唯一的通用「变了才写」）、`collect_active_ids`（:44）、`source_is_active`（:216）、`face_source_is_active`（:235）、`module_feature`（:246）。
- **判据**：`source_is_active`/`face_source_is_active` 决定**哪些面进入发布作用域**，是构建语义的核心判定，却住在一个叫 cache 的模块里；`write_if_changed` 是通用 IO（还被 `pipeline.rs` 的清单写入复用，见 `audit-lane-surfaces.md` 的 S2）。读者按「作用域」找会漏掉这里，按「cache」找到的却是一半作用域逻辑 → MAJOR。
- **最小改法**：`cache.rs` → `discovery_cache.rs`（只留三个缓存函数）；`write_if_changed` → 新建 `build_method/io.rs`（或并入已有 `manifests.rs`，它才是主要调用方）；`collect_active_ids`/`source_is_active`/`face_source_is_active` → `build_method/scope.rs`（该文件已经在管作用域）。
- **复核手段**：`grep -n "^pub(crate) fn\|^fn " build_method/cache.rs`；`grep -rn "cache::write_if_changed\|cache::source_is_active" build_method/src`（编译器点名）。

**NAM-05 / MINOR（原 MAJOR，t27 复核后降级） / A③④ / `mcp/evidence.rs`**

> **t27 复核后的修订**：原文用"17 个工具里 16 个各自住同名列，只有 `explain` 例外"支撑 MAJOR。按 dispatch 表复算（§G-2 的 P4）：**同名者 12/17**，例外 **5 个** —— `inspect`（`mcp/tools.rs:331`）、`read_source`（`mcp/tools.rs:356`）、`status`（`mcp/tools.rs:393`）三个 handler 就在 `tools.rs` 里，`unified` 在 `mcp/mir.rs:125`（与 `mir` 同文件），只有 `explain` 在 `mcp/evidence.rs:65`。另外 `mcp/evidence.rs` 的模块文档首行就是 "The build's own evidence for one face, or for the tree it scoped."（`:1`）——**名字与内容相符**。所以原文的 MAJOR 前提不成立。

- **现状**：261 行，7 个函数：`out_dir`（:36）、`build_evidence`（:46）、`explain`（:65，即 `xirang.explain` 工具的 handler）、`node_report`（:104）、`scope_line`（:141）、`pruning_line`（:169）、`tree_report`（:190）。内容以**构建证据读取器**为主，`explain` 只是其中一个 handler。
- **判据（降级后的两条，都是 MINOR）**：
  1. 工具名 `xirang.explain` 与文件名 `evidence.rs` 不一致（尽管这在本桥里**不是唯一**：5/17 个工具与文件不同名）；
  2. `evidence` 一词在本仓**一名多义**：`mcp/evidence.rs`（构建产物证据）vs `run_method/runtime/evidence.rs`（已观测调用证据）vs `studio/studio/app/tests/evidence.rs` —— 属 D-9 的实例。
- **最小改法（与 D-9 统一，采用 t27 倾向）**：`mcp/evidence.rs` → **`mcp/build_evidence.rs`**（保留词加限定）。**注意落地名分工（t31 裁定）**：`build_evidence` 只用于这个 **mcp 侧文件**；`debug_method` 的模块名是 **`call_evidence`**（特性名仍 `evidence`）。若维护者更看重"一工具一文件"，则拆成两半：读取器 → `build_evidence.rs`，`explain` handler → `explain.rs`；**二选一，不要同时**。
- **复核手段**：`grep -rn "^pub(crate) fn \(explain\|inspect\|read_source\|status\|unified\)\b" mcp/src` 应给出上表五行；`grep -n "^//! " mcp/evidence.rs | head -1`。

**NAM-17 / MINOR / A③ / `build_method/faces.rs` vs `face_view.rs`**
- **现状**：`faces.rs`（96 行，`FaceSource`/`collect_faces`/`has_selected_face`，做**首轮作用域的面发现**）与 `face_view.rs`（444 行，`FaceView`/`face_views`，做**操作命令的只读视图**）两个文件名字只差一个下划线，主语都是 face。
- **判据**：读者必须打开才知道哪个是"发现"、哪个是"视图"；在 `build_method/src` 里还有 `node.rs`/`node_id.rs`、`discovery.rs` 同族（见 NAM-18/21）。不误导但读不懂 → MINOR。
- **最小改法**：`faces.rs` → `scope_faces.rs`（或 `face_discovery.rs`）；`face_view.rs` 保持（它是公开概念 `FaceView` 的家）。
- **复核手段**：`grep -rn "use crate::faces\|crate::face_view" build_method/src` 逐处更新。

**NAM-18 / MINOR / A③ / `build_method/node.rs` vs `node_id.rs`**
- **现状**：`node.rs`（26 行，`Node` = 已发现的注册模块树节点 + `relative_display`）与 `node_id.rs`（117 行，`node_id()`/`collect_node_ids`/`select_module_subtree`）。
- **判据**：两个名字都短到只能靠下划线区分；`node.rs` 里的 `Node` 是**发现树的节点**，不是身份。读 `use crate::{node, node_id}` 时无法猜出区别 → MINOR。
- **最小改法**：`node.rs` → `discovery_node.rs`（结构体 `Node` 可保持，文件名表达它属于发现树）；`node_id.rs` → `node_identity.rs`。
- **复核手段**：`grep -rn "crate::node\b\|crate::node_id\b" build_method/src`。

**NAM-21 / MINOR / A③ / `build_method/src` 的"发现"族命名收敛**
- **现状**：`discovery.rs`（272 行，遍历与指纹）、`node.rs`、`node_id.rs`、`faces.rs`、`face_view.rs`、`registration_check.rs`（259 行）、`validation.rs`（352 行）——六个名字都在说"我负责发现/检查"。
- **判据**：没有一个名字能回答"我要找的是遍历、身份、面还是校验"；`discovery.rs` 与 `faces.rs` 的边界尤其模糊。
- **最小改法**：建议一轮收敛：`discovery.rs`→`source_walk.rs`（它做的是遍历+指纹）、`registration_check.rs`→`registration_phase.rs`（它是管线的一个阶段）、`validation.rs`→`face_syntax_check.rs`（它做的是面语法与父宏的诊断聚合，见我 t15 对 `parsed_face` 的引用）。
- **复核手段**：`grep -n "^//!" build_method/{discovery,faces,validation,registration_check}.rs` 对照各文件自述。

**NAM-22 / MINOR / A③ / `mcp/nodes.rs`**
- **现状**：76 行，两个函数 `parent_id`（:27）、`resolve_node`（:58）；模块文档说"代理所命名的东西，解析成执行器需要的身份"。
- **判据**：文件名 `nodes` 是名词复数、且与 `studio/studio/ui/graph/nodes.rs`（画节点）同名不同义（A④）；它的实际职责是**名字→身份解析**。
- **最小改法**：`mcp/nodes.rs` → `mcp/resolve.rs`（或 `node_refs.rs`）；`studio` 那个保持（它是绘图）。
- **复核手段**：`grep -rn "nodes::" mcp/src`。

### A④ 跨 crate / 跨目录同名不同职责

**NAM-08 / MINOR / A④ / `run_method/runtime/trace/call_trace.rs` 与 `locals/call_trace.rs`**
- **现状**：`trace/call_trace.rs`（348 行，模块文档「调用追踪收集器及其记录的协议表面」）与 `trace/locals/call_trace.rs`（228 行，模块文档「`CallTrace` 上的局部值记录与查询方法」）。
- **判据**：同一 crate 内两个同名文件、内容不同；第二个是第一个那个**类型的另一半**（locals 侧），名字却重复类型名，不表达"局部值"。`locals/locals.rs`（18 行）只是挂载根，`locals/call_trace.rs` 被它用 `#[path = "call_trace.rs"]` 挂进来。
- **最小改法**：`locals/call_trace.rs` → `locals/recording.rs`（或 `locals/local_recording.rs`，与模块文档同词）。
- **复核手段**：`grep -rn "locals::call_trace\|locals/call_trace" run_method/src`。

**NAM-09 / MINOR / A④ / `run_method/authoring/face_manifest.rs` 与 `manifest/face_manifest.rs`**
- **现状**：`authoring/face_manifest.rs`（89 行，自述「File-backed authoring for XiRang registration faces」，做文件化创作）与 `authoring/manifest/face_manifest.rs`（22 行，自述「Stable registration-face metadata model」，是数据模型）。
- **判据**：同一个名字下一个是 I/O、一个是模型；`manifest/` 里的那个才配得上"manifest"这个词，`authoring/` 里那个其实在做**文件读写**。
- **最小改法**：`authoring/face_manifest.rs` → `authoring/face_file.rs`（或 `face_store.rs`）；`manifest/face_manifest.rs` 保持。
- **复核手段**：`grep -rn "face_manifest::" run_method/src`。

**NAM-10 / MINOR / A④③ / `run_method/authoring/parse/parse.rs`（shim）与 `manifest/parse/parse.rs`**
- **现状**：`authoring/parse/parse.rs` 54 行，模块文档第 3–4 行自陈「pure source-text transformations live in the kernel… **this shim** keeps the historical [paths]」；`authoring/manifest/parse/parse.rs` 264 行（真解析边界）。
- **判据**：一个 shim 文件与真实现同名，且名字里没有"shim/历史/兼容"字样；`AGENTS.md` 改动规则 2 要求执行面用 shim 保留历史路径，所以 shim 会长期存在——名字应当说出来。`run_method/registry/registry.rs`、`run_method/plugin/plugin.rs`、`run_method/runtime/evidence.rs` 是同一族（都是 7–12 行的重导出）。
- **最小改法**：全部 shim 文件在模块名后加后缀，如 `authoring/parse/historical.rs`（模块 `historical`）；或至少让**模块文档第一行**（不是第 3 行）以「Shim / 历史路径」开头（doc_anchors/bilingual 门禁可查首行）。**注意**：这些文件的公开路径被 `conventions/src/shims.rs` 的棘轮守着，改模块名会动公开路径，所以推荐第二种（首行声明）而不是改名。
- **复核手段**：`grep -rn "pub use" run_method/{registry,plugin}/`；`cargo test --workspace --offline` 中 `path_compat` 一类钉子保持绿。

**NAM-11 / MINOR / A④ / 保留词表：`evidence` / `index` / `artifact` / `registry` 一名多义**
- **现状**（机器统计 + 逐个读模块文档）：

  | 词 | 出现处 | 各自含义 |
  | --- | --- | --- |
  | `evidence` | `mcp/evidence.rs`、`run_method/runtime/evidence.rs`、`studio/studio/app/tests/evidence.rs` | 构建产物证据 / 已观测调用证据 / 前者与后者的测试 |
  | `index` | `core/src/registry_core/tree/index/index.rs`、`mcp/index.rs`、`studio/studio/app/source_index.rs` | 注册树索引（NodeId→entry）/ 源码文件与函数索引 / 源码索引（Studio 侧，命名更好） |
  | `artifact` | `core/src/registry_core/plugin/artifact/artifact.rs`、`run_method/runtime/trace/artifact/artifact.rs` | 插件二进制+manifest / trace 快照 |
  | `registry` | `core/src/registry_core/tree/registry.rs`、`run_method/registry/registry.rs`（shim）、`mcp/registry.rs`（工具）、`examples/control-button/tests/registry.rs` | 注册机本体 / 历史路径 / `xirang.registry` 工具 / 示例测试 |

- **判据**：这四个词在本仓都**已被载义**（`EvidenceKind`/`CallEvidence`、`NodeId→entry` 索引、插件工件、`Registry` 类型）。第二个含义借用它们时，读者必须打开文件才知道是哪一套。
- **最小改法**：① `mcp/index.rs` → `mcp/source_index.rs`（照抄 studio `app/source_index.rs` 的命名，那里已经是对的）；② `run_method/runtime/trace/artifact/` → `…/snapshot/`，文件 → `snapshot.rs`（它与插件 artifact 毫无关系）；③ `mcp/evidence.rs` 按 NAM-05 改 **`build_evidence.rs`**（**mcp 侧文件名**；`debug_method` 的模块名是 `call_evidence`，见 D-9 的 t31 裁定；不采用 `explain.rs` 单改）；④ `mcp/registry.rs` 保留（工具名就是 `xirang.registry`）。把这四个词列成「保留词表」写进命名共识（见 D）。
- **复核手段**：`grep -rln "artifact" core/src run_method/src mcp/src studio/src` 与上表对照。

**NAM-12 / MINOR / A④ / `search.rs` 在同一 crate 内四处**
- **现状**：studio 有 `app/overlay/search.rs`（1 fn）、`app/state/search.rs`（0 fn，状态）、`ui/search.rs`（1 fn，入口）、`ui/search/{detail,results,query}.rs`；mcp 有 `search.rs`（工具）。`forms.rs` 同理三处（`app/state/forms.rs`、`ui/forms.rs`、`app/tests/forms.rs`）。
- **判据**：同名文件靠父目录区分，本身不算错；但 `ui/search.rs`（入口，1 fn）与 `ui/search/`（三个子模块）同时存在，是 A② 的变体——**入口文件与同目录同名**。这类"目录+同名入口"在本仓是约定，但 `search` 这个词在 studio 里同时指"状态/入口/详情/结果/查询"五件事。
- **最小改法**：不动结构，只建议 `ui/search.rs` 的模块文档第一行写明「搜索页入口（见 search/ 子模块）」（部分已有）；`app/state/search.rs`（0 fn）若只是 re-export，按 NAM-10 标注 shim。
- **复核手段**：`find studio/src -name "search*.rs"`。

**NAM-23 / MINOR / A④ / `pub use xirang_run_method::PluginChannel as ValidationChannel`**
- **现状**：`plugin-host/src/lazy_wasm.rs:24` 把内核的 `PluginChannel` 重导出为 `ValidationChannel`，文档（:20-23）说明是"保留历史路径"。
- **判据**：别名 `ValidationChannel` 与本体 `PluginChannel` 不同名不同义（一个像"校验通道"，一个就是"插件通道"）；`plugin-host/tests/fault_matrix.rs` 用的是别名。全仓 `pub use … as` 只有 4 处：`plugin::graft::document as graft_document`（避免与 `graft` 模块冲突，有理由）、`run_method/lib.rs:49-50` 的两个宏内部名（有理由）、本条（历史路径）。
- **最小改法**：保留别名（历史路径是契约），但在契约里补一句「等价于 `PluginChannel`」——最好直接写在文档首行；或在新代码里统一用 `PluginChannel`，别名只留兼容。
- **复核手段**：`grep -rn "ValidationChannel" plugin-host/src plugin-host/tests`。

### A⑤ 测试文件名与它测的东西

**结论（全量机器核对 + 抽样人读）**：`*_tests.rs` 一族共 **44 个**文件，其中 **43 个**与同目录的同名生产文件成对（`apply_tests.rs` ↔ `apply.rs` 等）；唯一例外是 `core/src/registry_core/syntax/deep_input_tests.rs`——它没有 `deep_input.rs`，因为它测的是**整个语法解析入口的深输入边界**（`AGENTS.md` 把"给守卫加一种形状"指向这个文件），名字描述的是被测的输入形状，属**有意的例外**；`tests/` 目录一族按**特性**命名（`run_method/tests/face_preset_parts.rs`、`plugin-host/tests/fault_matrix.rs`、`studio/studio/app/tests/*.rs`），名字描述的是被测行为而不是被测模块——这是好命名，不需要改成模块镜像。

**NAM-19 / MINOR / A⑤ / studio 的 `app/tests/*.rs` 需要写明"覆盖哪个生产模块"**
- **现状**：`studio/studio/app/tests/` 11 个文件按特性命名：`call_tree.rs`（空间调用树回归，文件文档已写清）、`trace_ingest.rs`（端到端 trace 摄入到 DATA 面板）、`source.rs`、`project.rs`、`graph.rs`、`forms.rs`、`graft.rs`、`edit.rs`、`navigation.rs`、`evidence.rs`、`fixtures.rs`。生产侧是 `app/trace.rs`、`app/source_index.rs`、`app/mutations.rs`、`app/graft.rs`……
- **判据**：名字描述行为（好），但**没有一条**能推断出它测哪个生产模块（`trace_ingest.rs` 测 `trace.rs`+`lifecycle.rs`；`project.rs` 测 `mutations.rs`+`support.rs`）。任务书点名问"`app/tests/call_tree.rs` 到底测什么"——**已核：它测空间调用树布局**（文件文档 1–3 行自述），名字与内容相符；问题在其余文件缺这条信息。
- **最小改法**：在这 11 个文件的模块文档第一行加「covers `app/<module>.rs`（+ …）」一句（不重命名）。`fixtures.rs` 例外（它是夹具，不是测试）。
- **复核手段**：`head -3 studio/studio/app/tests/*.rs` 应每个都有 covers 行。

### A⑥ 目录名是否表达层级

**结论（机器全量）**：`studio/studio/app/{state,overlay,tests}`、`ui/{forms,graph,search}`、`core/src/registry_core/tree/*`（10 个子目录）、`run_method/authoring/*` 的目录名都能表达"这一层放什么"；三个**平铺巨目录**才是层级问题，但那是结构问题、已在 `audit-structure-base.md` 与 `audit-boundary-refactor-plan.md` 里：`mcp/src` **39 个文件零子目录**、`build_method/src` **32 个零子目录**、`studio/studio/app` **20 个零子目录**。本报告只记一条命名相关的推论：

**NAM-20 / MINOR / A⑥ / 平铺目录让 A④ 的同名问题加倍**
- **判据**：`mcp/src` 平铺 39 个文件时，`index.rs`/`evidence.rs`/`registry.rs`/`search.rs` 与各自语义靠文件名硬区分；一旦按职责分成 `tools/`、`source/`、`evidence_build/`，A④ 的碰撞会自动消解。反之，平铺 + 宽义名 = 每个文件名都要承担全部消歧责任。
- **最小改法**：与结构报告一致（`mcp/src` 拆 `tools/`+`source/`）；命名上配合 NAM-11 的保留词表。
- **复核手段**：指路 `docs/audit-2026-09-28/audit-boundary-refactor-plan.md`（不重复结构方案）。

---

## B. 函数名与方法的抽象

### B① 空壳名（无宾语）

**NAM-32 / MINOR / B① / 裸动词作函数名：全量清单与规则**

> **t27 复核后的修订**：本节原列"非入口位 8 处"，清单不全（漏了 `load`×3、`build`、`get`）。下面按**全量谓词**重列，并把白名单逐行标出，使数目可审计。

- **谓词（精确写法，§G-2 P5）**：生产函数（`is_test == false`）∧ 名字**整名等于**裸动词集合中一个 ∧ `kind ∈ {inherent_method, free_fn, nested_fn}`（另单列 `trait_method`）。
- **输出：全量 24 行**（`名字`（container）`file:line`；`WL` = 白名单，不计违规）；另有 `trait_method` 2 行（#23 `load`、#24 `call`）单列在表尾：

| # | 名字 | container | file:line | 判定 |
| --- | --- | --- | --- | --- |
| 1 | `run` | — | `build_method/lib.rs:137` | WL（构建脚本入口） |
| 2 | `run` | — | `build_method/pipeline.rs:11` | **违规** |
| 3 | `run` | — | `cli/lib.rs:124` | **违规**（argv 分派） |
| 4 | `run` | `RuntimeCheckSpec` | `core/src/registry_core/declaration/runtime_checks.rs:345` | **违规**（建议 `evaluate`） |
| 5 | `run` | — | `mcp/protocol.rs:71` | WL（桥的 crate 入口，`main.rs` 调它） |
| 6 | `run` | — | `studio/main.rs:16` | WL（`main` 的实现） |
| 7 | `load` | `HotDeployment<I>` | `plugin-host/src/deployment.rs:49` | **违规**（返回快照，建议 `snapshot`） |
| 8 | `load` | `ProcessBackend` | `plugin-host/src/process.rs:146` | **违规** |
| 9 | `load` | `WasmBackend` | `plugin-host/src/wasm.rs:149` | **违规** |
| 10 | `load` | `App` | `studio/studio/app/lifecycle.rs:79` | **违规** |
| 11 | `load` | — | `mcp/mir.rs:217` | **违规**（建议 `load_artifact`） |
| 12 | `find` | `StaticPlan` | `core/src/registry_core/release/release.rs:243` | **违规**（D-4：`find_*` 要带宾语/限定） |
| 13 | `find` | `Registry` | `core/src/registry_core/tree/query/query.rs:80` | **违规**（建议 `find_by_id`，见 NAM-36） |
| 14 | `build` | — | `cli/commands/build.rs:17` | WL（CLI 命令同名实现） |
| 15 | `build` | — | `studio/studio/ui/graph/node_graph.rs:36` | **违规** |
| 16 | `check` | — | `cli/commands/check.rs:25` | WL（CLI 命令同名实现） |
| 17 | `apply` | — | `mcp/apply.rs:60` | **违规**（模块名已叫 `apply`） |
| 18 | `read` | `TreeDelta` | `mcp/tree_delta.rs:71` | **违规** |
| 19 | `handle` | `App` | `studio/studio/app/interaction.rs:12` | **违规**（建议 `handle_event`） |
| 20 | `get` | `EntryPages` | `core/src/registry_core/tree/entry_pages/entry_pages.rs:39` | **违规**（建议 `entry`） |
| 21 | `get_mut` | `EntryPages` | `core/src/registry_core/tree/entry_pages/entry_pages.rs:43` | WL（容器惯用语） |
| 22 | `call` | `WasmPluginTable` | `plugin-host/src/lazy_wasm.rs:137` | WL（转发到 `PluginInstance::call`） |
| 23 | `load` | `trait PluginRuntime` | `core/src/registry_core/plugin/plugin_policy/plugin_policy.rs:55` | WL（trait 声明，参数已表达宾语） |
| 24 | `call` | `trait PluginInstance` | `plugin-host/src/lib.rs:48` | WL（trait 声明） |

- **计数**：全量 24 行 − 白名单 9 行（#1、#5、#6、#14、#16、#21、#22、#23、#24）= **非入口位 15 处**。t27 独立数到 ≥14，与 15 不矛盾（其清单含 trait 声明那一行）。
- **其它裸动词（有宾语、不计）**：`studio/bin/xirang-dev.rs:169 fn stop(child: &mut Child)`（进程控制助手，宾语在参数里）。
- **判据**：`pipeline::run`、`command::run` 在模块语境下勉强可读，但 `TreeDelta::read`（读什么？）、`HotDeployment::load`（返回的其实是快照）、`App::handle`（处理什么？）需要读者回到签名才能懂。不误导，只缺信息 → MINOR。
- **最小改法**：见上表"判定"列；入口类保留。
- **复核手段**：§G-2 P5 的命令与输出（本表的 23 行就是它的原样输出 + 白名单标注）。

### B② 同一动作的同义词蔓延（谁在用哪个词）

**NAM-33 / MINOR / B② / 动词收敛表**
- **现状**（全量机器统计：生产函数名的首词直方图，只列 ≥5 的读类动词）：

  | 动词 | 计数 | 主要出现处 | 现在的实际语义 |
  | --- | --- | --- | --- |
  | `collect` | 39 | `tree/query/query.rs`(6)、`registration_check.rs`(3)、`validation.rs`(3) | 遍历累积进 `Vec`/`BTreeSet`（有时是公开返回，有时是内部递归helper，见 NAM-36） |
  | `parse` | 38 | `authoring/parse/parse.rs`(7)、`admission.rs`(3)、`flow.rs`(3) | 文本 → 结构 |
  | `render` | 38 | `authoring/parse/parse.rs`(5)、`trace/edges`(4)、`call_report.rs`(3) | 结构 → 文本 |
  | `resolve` | 20 | `graft_ops/resolution.rs`(4)、`lexicon.rs`(2)、`support.rs`(2) | id/路径 → 实体（也用于"解析成路径"，见下） |
  | `read` | 17 | `scope_view.rs`(2)、`process/child.rs`(2) | 从 reader/文件读字节 |
  | `load` | 16 | `mcp/index.rs`(3)、`studio/lifecycle.rs`(3) | 从磁盘/工件装载成结构 |
  | `validate` | 19 | `graft/document.rs`(3)、`graft_ops.rs`(3) | 检查并可能报错 |
  | `check` | 10 | `runtime_checks.rs`(5)、`contracts.rs`(1) | 检查（多为运行期断言） |
  | `find` | 7 | `tree/query/query.rs`(3) | 在集合里找（返回引用/集合） |
  | `get` | 2 | `tree/entry_pages/entry_pages.rs`(2) | **与 `find` 同义**（取一个 entry） |
  | `query` / `lookup` / `fetch` / `scan` / `extract` | 0 / 0 / 0 / 0 / 2 | `front_end.rs` 等 | 基本没蔓延（好） |
- **判据**：真正的蔓延集中在 **`get` vs `find` vs `read` vs `load`**（`EntryPages::get` 与 `Registry::find` 做同一件事）与 **`resolve` 的两义**（`Registry::resolve_path` = 路径→NodeId；`lexicon::resolve_package_root` = 从环境推出根；`studio::resolve_project` = 把参数解析成项目根——三者都是"从输入推出实体"，语义其实一致，可接受）。`check` 与 `validate` 的分工没有规则可循（`runtime_checks` 用 `check`，`graft/document` 用 `validate`）。
- **最小改法**（命名共识里定成规则，见 D-4）：`read_*` = 从 reader/文件读文本或字节；`load_*` = 从磁盘/工件装载成结构（可失败）；`resolve_*` = 输入（id/路径/名字）→ 实体；`find_*` = 在集合里查找并返回引用或集合（**`get` 一词在非 `Option` 返回时不再使用**）；`collect_*` = 累积进集合（公开函数用，内部递归 helper 应叫 `visit_*`/`walk_*`，见 NAM-36）；`parse_*` = 文本→结构；`render_*` = 结构→文本；`write_*` = 写盘；`validate_*` = 检查并把原因作为错误返回；`check_*` = 只用于运行期断言（`RuntimeCheckSpec`）。
- **复核手段**：重跑首词直方图（见 G 节脚本），改后 `get` 应为 0（`into_owned`/`Default` 除外）。

### B③ 名与行为不符（MAJOR）

**NAM-30 / MAJOR / B③ / `build_method/manifests.rs::parse_pruning_item` / `collect_pruning_symbols`**
- **现状**：`collect_pruning_symbols`（:173）说要收集"剪枝符号"，实际对每一行做魔法标识符匹配；`parse_pruning_item`（:200-213）的第三个分支**无条件返回字面量** `"Button::optional_pruning_probe"`（:210），不管当前是哪个面。
- **判据**：名字承诺"收集源码声明的符号"，行为会**发明**一个与当前面无关的符号写进 `pruning_manifest.tsv`（发布剪枝前读的证据文件）。这是"名与行为不符"的最强形态（名字说收集、行为说伪造）。我的 t15 已单独记为 LG-15（假实现），此处从命名角度补记：`collect_*` 这个名字本身就是错的。
- **最小改法**：要么让符号列真的从已解析的注册面/静态计划读（推荐），要么在修之前把函数名改成它实际做的事（`scan_pruning_probe_markers`）——但**名字改对不等于行为可接受**，行为要按 LG-15 修。
- **复核手段**：`timeout 30 grep -rn optional_pruning_probe --exclude-dir=target --exclude-dir=.git .`（当前只命中 `build_method/manifests.rs:208`、`:210`）。

**NAM-31 / MAJOR / B③ / `run_method/authoring/operations/face_write.rs::apply_trait_contract`**
- **现状**：函数名说"应用 trait 契约"；实现（:190-205）在 `labels` 与 `contract_paths` 都空时直接 `return Ok(())`，否则只写**trait 标签**（`face.edit(label_field, …)`）；真正的 `handle_contracts:` 行由**创建**路径的 `CREATE_FIELD_ORDER` 写，编辑路径的 `EDIT_FIELD_ORDER`（:59-79）**不含** `handle_contracts`/`part_contracts`。
- **判据**：名字承诺"契约"，行为只落"标签"；配合工具自述把这两个键列为可编辑（`mcp/apply.rs:172-196`），于是 `apply edit` 回"成功"而契约不写（我 t15 记为 LG-21 MAJOR）。名字在这里不是唯一缺陷，但它让维护者以为编辑路径会写契约。
- **最小改法**：`apply_trait_contract` → `apply_trait_label`（并把契约行的归属写进注释）；行为按 LG-21 修（把两个字段加进 edit 序，或在 edit 路径明确拒绝）。
- **复核手段**：`sed -n '190,205p' run_method/authoring/operations/face_write.rs`；`sed -n '56,79p' …face_write.rs` 看两张字段序。

### B④ 名字描述实现而不是意图

**NAM-34 / MINOR / B④ / `_owned` 家族与 `_strings` 一类：全量 25 行，8 行计入**

> **t27 复核后的修订**：原写"5 违规"，偏小且未给清单。下面按全量谓词重列并逐类给理由。

- **谓词（§G-2 P6）**：生产函数名含 `_owned/_leak/_boxed/_vec/_raw/_strings/_string/_opt/_impl` 之一，排除 Rust 惯用语 `into_owned`/`from_owned`/`from_raw`。**输出 25 行**。
- **计入 D-7 的 8 行**（名字的主要区分轴是实现，不是意图）：

| 名字 | file:line | 建议 |
| --- | --- | --- |
| `split_csv_owned` | `core/src/registry_core/authoring/parse/parse.rs:258` | `parse_field_list` |
| `parse_admission_owned` | `core/src/registry_core/authoring/parse/admission.rs:128` | `parse_admission_snapshot` |
| `parse_requirements_owned` | `core/src/registry_core/authoring/parse/rules.rs:108` | `parse_requirements_snapshot` |
| `parse_registration_rule_owned` | `core/src/registry_core/authoring/parse/rules.rs:140` | `parse_registration_rule_snapshot` |
| `quoted_strings` | `core/src/registry_core/authoring/parse/admission.rs:53` | `read_string_literals` |
| `rule_method_strings` | `build_method/contracts.rs:193` | `rule_method_calls`（并见 S11 的行为缺陷） |
| `argv_strings` | `cli/lib.rs:107` | `argv_arguments`（或 `collect_argv`） |
| `is_xirang_owned_source` | `run_method/authoring/operations/paths.rs:41` | `is_generated_source`（谓词里嵌了实现词 `owned`） |

- **不计入的 17 行及理由**（同一次输出的其余部分）：
  - 领域用词（"字符串字面量"是源码里的东西，不是表示法）：`rust_string`、`json_string`、`push_json_string`、`literal_string`、`raw_string_end`、`path_to_string`、`option_string`、`written_string_list`、`literal_string_expr`、`rule_source_path_string`（返回的是"把路径渲染成文本"，`_string` 在这里说取值形态而不是实现轴）、`render_optional_source`（`optional` 是语法形态）、`parse_optional_source`×2、`parse_optional_id`（同上）。
  - 惯用语/内部实现分界：`edit_optional`、`assert_impl`、`parse_face_macro_impl`（`_impl` = "某物的实现"，本仓用于区分包装层）。
- **判据**：`_owned`/`_strings` 说的是返回值或中间表示（实现细节），真正该写的是"这是什么产物"（快照 / 字面量集合 / 参数表）。不误导行为，只浪费一次阅读 → MINOR。
- **最小改法**：上表"建议"列。
- **复核手段**：§G-2 P6 的命令与输出（25 行）。

### B⑤ 布尔谓词规范

**NAM-35 / MINOR / B⑤ / 三处裸形容词谓词**
- **现状**（全量 `grep "fn … -> bool"` + 前缀过滤）：绝大多数 `-> bool` 函数都带谓词前缀（`is_`/`has_`/`can_`/`needs_`/`supports_`/`accepts_`/`matches_`/`keeps_`，共约 120 处）。三处例外：`core/src/registry_core/release/release.rs:191 pub const fn full(self) -> bool`、`core/src/registry_core/source/walk.rs:54 fn skips(&self, path: &Path) -> bool`、`core/src/registry_core/plugin/contracts/contracts.rs:460 pub fn targets(self, framework: FrameworkId) -> bool`（另 `core/src/registry_core/plugin/catalog/catalog.rs:281 accounts_for`、`core/src/registry_core/declaration/call_evidence.rs:91 confirmed` 已带宾语，可接受）。
- **判据**：`full()` 作为谓词读起来像名词/动词，Rust 惯例是 `is_full()`；`skips(path)`/`targets(framework)` 带宾语，语义清楚，只是缺少"是不是"的语感。
- **最小改法**：`full` → `is_full`；`skips` → `skips_path`（保留宾语）；`targets` → `targets_framework`。
- **复核手段**：重跑 B⑤ 过滤器（见 G 节脚本）后该清单应为 0。

### B⑥ pub API 与内部函数风格不一致

**NAM-36 / MINOR / B⑥ / `Registry::find` 与它的两个兄弟不同精细度**
- **现状**：`core/src/registry_core/tree/query/query.rs:80 pub fn find(&self, id: NodeId) -> Option<&RegistrationSnapshot>`；紧邻 `:146 pub fn find_kind(&self, kind: &str)`、`:154 pub fn find_where(...)`。另有内部递归 helper 一族：`:105 collect_node_path`、`:164 collect_kind`、`:183 collect_depth_first`（与公开的 `:100 node_path`、`:135 collect_node_ids`、`:177 depth_first` 成对）。
- **判据**：公开族里 `find`（按 id）没有宾语而两个兄弟有；内部 helper 借用了公开动词 `collect_*`（`collect_kind`/`collect_depth_first` 其实是 `find_kind`/`depth_first` 的递归实现，不是"收集者"）。读者按 `find_by_id` 猜会找不到。
- **最小改法**：`find` → `find_by_id`；内部 helper 改 `visit_*`/`walk_*`（`collect_kind` → `visit_kinds`、`collect_depth_first` → `walk_depth_first`），让 `collect_*` 只出现在公开的"返回集合"函数上。
- **复核手段**：`grep -n "pub fn \|fn collect_\|fn visit_\|fn walk_" core/src/registry_core/tree/query/query.rs`。

### B⑦ 单复数 / 单位 / 时态

**NAM-37 / MINOR / B⑦ / 两处**
- **现状**：① `core/src/registry_core/tree/entry_pages/entry_pages.rs:39 get` / `:74 remove`（单数动词，操作单个 entry；同文件 `:43 get_mut` 是容器惯用语，**不计**）；同族的公共查询却叫 `Registry::registry`/`entry_at`（无动词，位置介词）——同一件事三种风格。② 单复数：`run_method/runtime/trace/locals/` 里 `local_id.rs`/`local_kind.rs`/`local_value.rs`/`observation.rs` 是**单数类型名**，而同目录的挂载根与目录名 `locals`（复数）；`edges`/`frames` 目录也是复数装单数类型（`DataEdge`/`TraceFrame`）——这是 Rust 类型单数+模块复数的常见写法，**不建议改**，但值得在共识里写死以免下一轮反复争论。
- **最小改法**：`EntryPages::get` → `EntryPages::entry`（与 `entry_at` 同族，且避免与 NAM-33 的 `find` 撞）；`EntryPages::remove` → `remove_entry`；`get_mut`（`:43`）保留容器名。类型单数/模块复数保持（写进共识）。
- **复核手段**：`grep -rn "fn get\b" --include=*.rs core/src` 应为 0（除 trait impl/`BTreeMap` 转发）。

---

## C. 模块名

**NAM-40 / 结论已修订（原"干净"被证伪） / C / 模块路径与文件名的对应**

> **t27 复核后的修订（结论级更正）**：本报告原写"394/394 `module_path` 末段与文件 stem 一致、0 违规、唯二有意例外"。**这条是错的**。
>
> **谓词（精确写法）**：`module_path` **非空** ∧ `module_path.split("::").last() != <file stem>`。
> **命令/输出（§G-1 的 P1）**：**14 处**不一致，分四族：
>
> | 族 | 文件 → `module_path` 末段 | 数 |
> | --- | --- | --- |
> | 重命名挂载（历史 shim） | `build_method/identity.rs` → `registry_identity`、`build_method/syntax.rs` → `registry_syntax` | 2 |
> | CLI 命令 | `cli/commands/{build,check,new,snippets,studio}.rs` → `{build,check,new,snippets,studio}_command` | 5 |
> | CLI explain 子模块 | `cli/explain_{json,overlay,report}.rs` → `explain::{json,overlay,report}` | 3 |
> | 测试模块 | `cli/lib_tests.rs`、`core/src/registry_core/{lexicon/lexicon_tests,source/source_tests,syntax/face_tests}.rs` → `::tests` | 4 |
>
> 另：再有 **9 个**文件的 `module_path` 为空（两个 example 宿主的 4 个面 + Studio 夹具的 5 个面），若把空值也算"末段 != stem"则为 23 处；本节与 D-10/D-11 一律用"非空"口径 = 14。
>
> **我原来的两处"有意例外"错了**：`core/src/registry_core/plugin/plugin.rs:35` 的 `graft_document` 与 `run_method/lib.rs:49-50` 的宏别名都是**重导出别名**（`pub use … as …`），根本不进 `module_path`——我把"重导出语句的形状"当成了"模块名的形状"。
>
> **正确表述**：**文件名**镜像生产文件成立（`x.rs` ↔ `x_tests.rs`，43/44 成对）；**模块名**不镜像：4 处 `*_tests.rs` 统一挂成 `mod tests`，另有 10 处重命名挂载（上表前三族）。这些重命名挂载**都是有意的**（历史 shim 保路径、CLI 命令名与模块名解耦、测试统一 `mod tests`），因此本节不判缺陷，只把事实写清；**D-10/D-11 不得再以"394/394 一致"为前提**。
**NAM-43 / MINOR / C / 模块挂载改名的四个家族（文件 stem ≠ 模块名，见 NAM-40）**

- **现状**（谓词 P1，§G-1）：14 处不同名，分四族：
  1. **历史 shim 保路径**（2）：`build_method/identity.rs` 挂成 `pub mod registry_identity;`、`build_method/syntax.rs` 挂成 `pub mod registry_syntax;`（`build_method/lib.rs` 里）——目的是让 `build_method` 的公开名与内核的 `registry_core::identity`/`syntax` 对齐，属 `AGENTS.md` 规则 2 的同族做法。
  2. **CLI 命令**（5）：`cli/commands/{build,check,new,snippets,studio}.rs` → `mod {build,check,new,snippets,studio}_command;`——避免与命令函数名 `fn build`/`fn check` 相撞。
  3. **explain 子模块**（3）：`cli/explain_{json,overlay,report}.rs` → `mod explain::json/json`——把三个同族文件收在一个 `explain` 模块下。
  4. **测试模块**（4）：`cli/lib_tests.rs` 与 `core/.../{lexicon/lexicon_tests,source/source_tests,syntax/face_tests}.rs` → `mod tests;`——与全仓多数 `*_tests.rs` 直接以文件名挂载的做法**不一致**。
- **判据**：四族各自都有理由，但**没有一条成文规则**说"什么时候允许挂载名 ≠ 文件名"，所以第 4 族两种写法（`mod tests` vs `mod <name>_tests`）在同一仓库并存，读者无法从文件名反推模块路径。不误导（都能编译、路径都在），只让"模块路径 ↔ 文件名"这一直觉失效 → MINOR。
- **最小改法**：① 把四族写进 `AGENTS.md` 的挂载约定（"重命名挂载只许用于：历史 shim、命令名冲突、同族收拢；测试统一 `mod tests` 或统一 `mod <name>_tests`，选一个"）；② 统一第 4 族（现全仓 44 个 `*_tests.rs` 里 40 个直接挂载、4 个挂 `mod tests`，建议把这 4 个也改成直接挂载）。**不要**为对齐而改动第 1 族（那是 shim 棘轮的一部分，见 NAM-03 的注）。
- **复核手段**：§G-1 P1 的命令；`grep -rn "mod tests;" cli/lib.rs core/src/registry_core/{lexicon,source,syntax}/*.rs`。

**NAM-41 / MINOR / C / `pub use … as …` 的别名纪律**
- **现状**：全仓 4 处（见 NAM-23）。三处有明确理由，`ValidationChannel` 一处是语义漂移。
- **判据**：别名让"同一物两套名字"成为长期状态，而本仓的 `lexicon` 与 `shims` 门禁已经在管别的"两套名字"问题；别名应当比普通重导出有更高的说明要求。
- **最小改法**：命名共识 D-8（别名必须同名同义或在文档首行写明历史理由）。
- **复核手段**：`timeout 60 grep -rn "pub use .* as " --include=*.rs core/src run_method/src build_method/src mcp/src studio/src cli/src plugin-host/src debug_method/src macro/src | grep -v _tests`。

**NAM-42 / 结论（干净） / C / 宏生成的模块名**
- **现状**：`examples/control-button` 的四个面在编译产物里有 `__xirang_ra_<path>` 与短名两个模块（`control`/`button`/`slider`/`registry_rule`）——这是 IDE 镜像（`cfg(rust_analyzer)` 门控），命名由路径推导，一致。
- **判据**：见 `audit-inventory.json` 的 `generated_module_names`；与手写模块名不冲突（前缀 `__xirang_ra_`）。
- **最小改法**：无需改。

---

## D. 命名共识（可执行规则，供 `conventions` 落地）

下面每条都写成「规则 → 能机械判定的谓词 → 现状违规数」。现状违规数是我在本轮机器统计下得到的（复核脚本见 G 节）。**建议按棘轮落地**（先钉住现状清单，只许变短），与 `conventions/src/size.rs` 的做法一致。

| # | 规则 | 机械判定 | 现状 |
| --- | --- | --- | --- |
| D-1 | **文件名不得是类别名**：`support|misc|helpers|helper|common|utils|util|shared|types|base|general|stuff` 不得作为 `.rs` 文件名（测试与挂载根除外） | 文件 stem ∉ 黑名单 | 2 违规（NAM-01/02，都在 studio） |
| D-2 | **一个文件一个主语**：模块文档第一行必须出现该文件的主语（类型名/动作名），且主语与文件名同义 | 人工（doc 首行与文件名对照）→ 可作为文档门禁的一条检查 | 抽样 28 个文件中 3 个不符（NAM-01/03/04） |
| D-3 | **无载荷目录收平**：若目录下**递归**所有 `.rs` 恰为 `{<dir>.rs}`，则该目录应不存在（用裸 `mod x;`）——**豁免 `examples/**` 与 `studio/tests/fixtures/**`**（身份与字面路径，见 NAM-07 的豁免表） | 递归 `.rs` 集合比对（§G-1 的 **P2d**，即 NAM-07 的 **A1**）**∧ 路径不以 `examples/` 或 `studio/tests/` 开头** | 三个口径：**仓库 34 / 发布面 28 / 豁免后可达 0**；不再用 39（NAM-07 的 A2）或 67（A3） |
| D-4 | **同一动作全仓一个动词**：读文件/reader=`read_*`；装成结构=`load_*`；输入→实体=`resolve_*`；集合查找=`find_*`（禁 `get_*`，容器惯用语 `get_mut` 除外）；累积返回集合=`collect_*`（内部递归用 `visit_*`/`walk_*`）；文本→结构=`parse_*`；结构→文本=`render_*`；写盘=`write_*`；检查并返回原因=`validate_*`；运行期断言=`check_*` | 首词直方图 + 白名单 | `get` **1**（`core/src/registry_core/tree/entry_pages/entry_pages.rs:39`；同文件 `:43 get_mut` 排除）；内部 helper 借 `collect_*` **3 处已确证**（`core/src/registry_core/tree/query/query.rs:105`、`:164`、`:183`），全仓清单未列 → 标「待清单」（t27 亦无法判定） |
| D-5 | **裸动词只许出现在入口位**：整名是一个裸动词（`run|apply|check|build|load|read|handle|call|do|process|update|stop`）的函数，只允许是 crate 根入口、CLI 命令同名实现、trait 声明/实现、或 `main` | 名字 ∈ 裸动词集合 ∧ 位置 ∉ 白名单 | **非入口位 15 处**（全量 24 行清单见 NAM-32；t27 复核数到 ≥14） |
| D-6 | **布尔谓词要读成一个问句**：`-> bool` 的公开函数名以 `is_/has_/can_/should_/needs_/supports_/accepts_/matches_/keeps_` 开头（**门禁只认前缀**；「或带宾语」不可机械判定，落地时要么改成纯前缀白名单（违规数会升到含 `skips`/`targets` 一级），要么人工维护豁免清单） | `grep "-> bool"` + 前缀白名单 | 前缀口径 **3** 违规（NAM-35） |
| D-7 | **名字描述意图不描述实现**：主要区分轴不得是 `_owned/_strings`（`into_*`/`from_*`/`from_raw` 的 Rust 惯用语除外；`*_string` 一类指「源码里的字符串字面量」属领域用词，不在本规则内） | 名字含实现词根 ∧ 不在惯用语/领域词表 | **8** 违规（全量 25 行与分类见 NAM-34） |
| D-8 | **别名同名同义**：`pub use X as Y` 仅当 Y 是该概念的（历史）同名，且 Y 的文档首行写明"历史路径/别名" | 全量 grep + doc 首行检查 | 1 需补文档（NAM-23），3 合规 |
| D-9 | **保留词表**：`evidence`/`index`/`artifact`/`registry`/`face`/`graft`/`trace` 在本仓已被载义；同一词出现第二个含义时，新文件必须加限定词（`source_index`/`trace_snapshot`/`build_evidence`） | 保留词 × 文件 stem 的映射表 | 4 个词各一名多义：`evidence`（NAM-05/11）、`index`、`artifact`、`registry`（NAM-11）；`mcp/evidence.rs` 的改名按 NAM-05 采用 `build_evidence.rs`。**落地名已由 t31 裁定（本报告与之对齐）**：`debug_method` 的模块名取 **`call_evidence`**（特性名仍是 `evidence`）；**`build_evidence` 是 mcp 侧文件的名字**，不是该模块的名字——两者不冲突，别混用 |
| D-10 | **类型单数、模块复数**（`local_value.rs` 装 `LocalValue`，目录 `locals/`）；**文件名**镜像生产文件（`x.rs` ↔ `x_tests.rs`；**模块名自由**——现状 `*_tests.rs` 统一挂 `mod tests`，`registry_identity`/`*_command`/`explain::*` 共 14 处不同名，见 NAM-40）；按特性组织的测试目录必须在首行写 `covers` | 文件 stem 比对（不是 `module_path`）+ 首行检查 | 文件名 43/44 成对（1 个有意例外）；模块名 14 处不同名（有意，不计违规）；`covers` 行缺 10 处（NAM-19） |
| D-11 | **跨 crate 同名词必须同义**：`search.rs`/`forms.rs`/`registry.rs`/`parse.rs`/`validation.rs`/`contracts.rs` 等在两个 crate 出现时，若语义不同，其中一个必须加限定 | 同名文件 × 模块文档首行的主语比对 | **无法判定（缺清单）**：t25 原写「抽样 12 组里 6 组」，未给 12 组清单；t27 亦无法复算。已确证的只有 NAM-08（`call_trace.rs`×2）、NAM-09（`face_manifest.rs`×2）、NAM-11（4 个保留词）、NAM-17（`faces.rs`/`face_view.rs`）→ 该数改为「**已确证 4 组**（其余待清单或本规则不落地）」 |

**落地顺序建议**：D-3（纯结构；**先做发布面那 28 处**，`examples/` 与 `studio/tests/fixtures/` 的 6 处按豁免跳过——见 NAM-07 的豁免表；**因此它不能「一次机械完成」**：要先按前缀分流，只对发布面执行）→ D-1/D-5/D-6/D-7（4 条纯文本门禁，违规共 18 处）→ D-4/D-9/D-11（需要一次命名表决，涉及公开路径的按 `shims.rs` 棘轮补别名）→ D-2/D-8/D-10（文档首行门禁）。

---

## E. 与既有 lane 命名条目的归并（不重复计数）

| 既有条目 | 内容 | 本报告的处理 |
| --- | --- | --- |
| `KRN-K-19`（MINOR） | 公开 API 的失败模式与命名不符（`is_face_source` 在非注册面返回 true；`*_matches` 需要未强制的小写前提） | **并入 D-6 的判据**：`is_face_source` 的问题是"名字承诺判定、实际是启发式"，建议改名 `looks_like_face_source`（或在文档写明是启发式）。归入 NAM-35 备注。 |
| `KRN-K-24`（MINOR） | 测试名与它断言的行为相反 | 归入 **D-10 的测试命名**：`x_tests.rs` 内部函数名同样要读成一个断言（本仓多数是 `a_x_is_b` 风格，那处是例外）。列为本轮复核项而非新条目。 |
| `KRN-C-10`（MINOR） | `mir_line` 与 `resolve_*` 家族"名字承担不了信息" | `resolve_*` 部分见 **NAM-33/D-4**（`resolve` 在本仓语义一致，保留）；`mir_line` 属**字段名**，本轮范围是文件/模块/函数名，记录指针不重复。 |
| `SUR-C8`（MINOR） | `write_if_changed` 的文档没说它不原子 | 归入 **NAM-04**（该函数应迁出 `cache.rs`）；原子性本身是 t13/S2 的行为条目，不在此重复。 |
| `STU-S-14`（MINOR） | `app/writers.rs`（42 行）与 `app/mutations.rs`（395 行）职责不清 | **并入 NAM-06（维持 MINOR）**：t27 复核证明 `writers.rs` 自述的是"写入守卫"（`Write guards`，且第二段写明它没有回落根），与内容相符；t25 原写的"自称拥有全部写入"是把名字的褒义读进了文档。修法改为**改名 `write_guard.rs`**，不合并。 |
| `STU-S-17` / `STU-S-19` / `STU-S-26`（MINOR） | `studio/studio/app/navigation.rs` 的死分支与命名、`studio/studio/app/keyboard.rs:160-164`（见 `audit-lane-studio.md` 的 STU-S-19）、`studio/studio/app/navigation.rs:1-96`（见 STU-S-26） 的位置 | 属**函数内/文件职责**问题（结构轴），本轮不重复；`studio/studio/app/navigation.rs` 的文件名本身清晰，不需改。 |
| `BRG-BR-C6`（MINOR） | 无名三元组 `build_evidence`、半真的 `is_loaded` | **并入 NAM-05**（该文件改名 `build_evidence.rs`；三元组具名属类型问题、不在本轮命名范围）与 D-6（`is_loaded` 是实现名，建议 `is_serving`/`active_generation`，见 t12 对 `lazy_wasm` 的复核）。 |
| `LGC-LG-01` / `LG-13` / `LG-16` | 身份缓存名不含"命名空间"维度；`XIRANG_NAMESPACE` 在两处被解释成两套语义；`compile_error_demo` 魔法目录名当开关 | **LG-16 → D-9 的实例**（用目录名当开关 = 名字承担了协议）；**LG-13 → D-4/D-8 的实例**（同名环境变量两种语义）；**LG-01 → D-2 的实例**（`CACHED_NODE_IDS`/`node_id` 这个名字不表达"按命名空间分桶"）。三条的行为修复在 t15 总账，此处只做命名归因。 |

**NAM-06 / MINOR（原 MAJOR，t27 复核后降级并改修法） / A③ / `studio/studio/app/writers.rs`（归并 STU-S-14）**

> **t27 复核后的修订**：我原写"模块文档自称负责全部写入、实际零写入"。按原文复核：模块文档首行是 "**Write guards**: every operation that creates, rewrites, moves, or deletes something in the project the reader opened."，第二段进一步解释"本模块存在的意义是读/写的区分：写入方都走 `with_selected_project`，它没有回落"。**文档自述的是"守卫"，不是"执行全部写入"**——我（以及 lane-studio 的 C-02）把名字的褒义读进了文档。内容（`selected_package_root`（:30）、`with_selected_project`（:37））与"守卫"自述相符。
>
> **同时撤回我原来的修法**："并入 `mutations.rs` 后删除"会拆掉文档第二段写明的那道缝（写入必须走无回落根）。**改为改名**。

- **现状**：42 行、2 个函数；名字 `writers` 读起来像"所有写入都在这儿"，而它其实是一个**写入守卫/上下文包装**的窄缝。
- **判据**：名字暗示的实现范围比文件实际承担的更宽（读者会以为写盘逻辑在此，实际写盘在 `app/mutations.rs` + `run_method` 的 authoring 执行器）；不误导行为，只误导"去哪找" → MINOR。
- **最小改法**：`studio/studio/app/writers.rs` → **`studio/studio/app/write_guard.rs`**（保留两个函数与那道缝，不动 `mutations.rs`）。若还要更明确，可在模块文档首行保留 "Write guards" 的同时把文件名对齐。
- **复核手段**：`head -3 studio/studio/app/writers.rs`（应仍是 "Write guards" 自述）；`grep -rn "writers::" studio/src` 逐处更新（编译器会点名）。

---

## F. 查了、干净（覆盖口径；每条写成 谓词 + 命令 + 输出）

> **t27 复核后的修订（口径纪律）**：本节原来把结论写成"X/N 一致"而不给谓词，t27 在其中 5 处发现数不可复算（其中 NAM-40 被证伪）。现改为与 §G 的谓词编号一一对应；**每条都给谓词、命令与输出**。

| # | 结论 | 谓词（§G 编号） | 命令 | 输出 |
| --- | --- | --- | --- | --- |
| F-1 | 类别名黑名单 8 词**零命中**；只有 `support`/`misc` 各 1 | 文件 stem ∈ 黑名单（P2a） | §G-1 P2a | 8 词 `[]`；命中 2：`studio/studio/app/support.rs`、`studio/studio/app/state/misc.rs` |
| F-2 | **文件名**镜像成立（43/44 成对）；**模块名不镜像**（14 处不同名，均有意） | 文件 stem 比对（P2c）；`module_path` 非空 ∧ 末段 ≠ stem（P1） | §G-1 P2c / P1 | 文件名：`*_tests.rs` 44 个中 43 个成对（唯一例外 `core/src/registry_core/syntax/deep_input_tests.rs`）；模块名：**14 处**（4 族，见 NAM-40） |
| F-3 | `*_tests.rs` 与生产文件成对 | 同目录存在 `<stem>.rs`（P2c） | §G-1 P2c | 44 − 1 = **43** |
| F-4 | `pub use … as …` 全仓 4 处（3 处有理由） | 正则 `pub use .* as `，排除 `_tests.rs`（P7） | §G-3 | 4 行；第 5 个 grep 命中是 `build_method/renderer/pass.rs:346` 的**断言字符串**，非语句 |
| F-5 | `-> bool` 谓词例外 3 处（前缀口径） | `-> bool` ∧ 名字不以谓词前缀开头（P4） | §G-4 | 3 行：`core/src/registry_core/release/release.rs:191 full`、`core/src/registry_core/source/walk.rs:54 skips`、`core/src/registry_core/plugin/contracts/contracts.rs:460 targets` |
| F-6 | 惯用语不计缺陷 | 名字 ∈ {`main`,`new`,`fmt`,`from`,`default`,`into`…} ∧ kind ∈ {`trait_impl_method`,`trait_method`}（P5/P6 的排除项） | §G-2 P8 | `is` 41、`new` 40、`from` 31、`fmt` 26、`default` 6、`into` 9 |
| F-7 | 目录层级可读；三个平铺巨目录属结构问题 | 目录内文件数 ≥ 20（P3） | §G-1 P3 | `mcp/src` 39、`build_method/src` 32、`studio/studio/app` 20 |
| F-8 | `studio/studio/app/tests/call_tree.rs` 测空间调用树布局，名字与内容相符（**不是缺陷**） | 人读文件文档 1–3 行 | `head -3 studio/studio/app/tests/call_tree.rs` | "Spatial call-tree regression tests… 焦点独占第零层、调用者与被调用者填满周围层、游标沿轴移动" |

**只做了机器统计、没有逐个人读的**（不要当成已查）
- 394 个文件里，只有约 **28 个**的**内容**被打开（`support.rs`、`state/misc.rs`、`ui/forms.rs`、`state/forms.rs`、`tests/{call_tree,trace_ingest,fixtures}.rs`、`runtime/trace/call_trace.rs`、`locals/{call_trace,locals}.rs`、`authoring/{face_manifest,parse/parse,validation/validation}.rs`、`manifest/{face_manifest,parse/parse}.rs`、`build_method/{node,faces,cache}.rs`、`mcp/{nodes,evidence,tools,mir}.rs`、`plugin-host/{deployment,lazy_wasm}.rs`、`run_method/runtime/evidence.rs`、`core/src/registry_core/tree/query/query.rs`、`conventions/{naming,mounting,shims}.rs`，加上 t8/t12/t15 已读的若干）；其余 ~366 个只参与机器统计。
- 2462 个函数名里，**约 160 个**被过滤器命中并逐个人读；其余 ~1270 个生产名只过了机器过滤（首词、长度、词根、kind、visibility），**没有逐个人读语义**。B③ 只保证"过滤器命中者已核"。
- 测试函数名（1029 个）未判断语义，只沿用 `KRN-K-24` 的既有结论。
- **t30 本轮新增"只读了一部分"的**：`studio/studio/app/state/misc.rs` 只读 `:55-100`（够取 `CallRef`/`CallTreeView`）；`mcp/tools.rs` 只 `grep` 了 handler 行号；`conventions/src/shims.rs` 只读 `:20-80`（SHIMS 表头部）；`plugin-host/src/lazy_wasm.rs:14-28`（别名）。

---

## G. 附录：复核命令（谓词 P1–P8 + 命令 + 输出）

> 所有输出都是本报告作者在 `HEAD=cf0c378` 工作树上跑出来的；脚本只读 `audit-inventory.json` 与文件系统，不写任何文件。

```text
# G-1 文件/目录全量（P1 module_path、P2a 类别名、P2b listdir 单条目、P2c 文件名镜像、P2d 递归载荷、P3 目录数）
# 注意：P2b 用 os.listdir（全部条目）给 34；NAM-07 表的 A2 用 inventory 的直接 .rs 给 39——名字相近、谓词不同。
import json, os, collections
d = json.load(open('docs/audit-2026-09-28/audit-inventory.json'))
files = [dict(f, crate=c['name']) for c in d['crates'] for f in c['files']]

# P1: module_path 非空 ∧ 末段 != 文件 stem
mm = [f['path'] for f in files
      if f.get('module_path') and f['module_path'].split('::')[-1] != os.path.basename(f['path'])[:-3]]
print("P1 非空口径:", len(mm))                                   # -> 14
print("P1' 含空 module_path:", sum(1 for f in files
      if (f.get('module_path') or '').split('::')[-1] != os.path.basename(f['path'])[:-3]))  # -> 23

# P2a 类别名
CAT = {'support','misc','helpers','helper','common','utils','util','shared','types','base','general','stuff'}
print("P2a:", [(f['crate'], f['path']) for f in files if os.path.basename(f['path'])[:-3] in CAT])

# P2b/P2c/P2d
dirs = collections.defaultdict(list)
for f in files: dirs[os.path.dirname(f['path'])].append(f['path'])
def recursive_rs(dp):
    out = []
    for root, _, fs in os.walk(dp):
        out += [os.path.join(root, x) for x in fs if x.endswith('.rs')]
    return out
rec, direct = [], []
for dp in dirs:
    stem = os.path.basename(dp)
    if not stem: continue
    if sorted(os.path.basename(x) for x in os.listdir(dp)) == [stem + '.rs']: direct.append(dp)
    if sorted(recursive_rs(dp)) == [os.path.join(dp, stem + '.rs')]: rec.append(dp)
print("P2b listdir 单条目:", len(direct))                          # -> 34（与 P2d 同值；A2 的 39 来自 inventory 的直接 .rs，不是这一行）
print("P2d 递归:", len(rec))                                     # -> 34
print("P2d 发布面:", len([x for x in rec if not x.startswith(('examples/', 'studio/tests/'))]))  # -> 28
print("P2b-P2d:", sorted(set(direct) - set(rec)))                # -> []
print("P2c *_tests.rs 无同名生产文件:", [os.path.basename(p) for p in
      (f['path'] for f in files if os.path.basename(f['path']).endswith('_tests.rs'))
      if not os.path.exists(os.path.join(os.path.dirname(p), os.path.basename(p)[:-len('_tests.rs')] + '.rs'))])
print("P3 目录数:", len({os.path.dirname(f['path']) for f in files}))   # -> 105
```

```text
# G-2 函数全量（P5 裸动词、P6 实现词根、P8 首词）
import json, collections, re
d = json.load(open('docs/audit-2026-09-28/audit-inventory.json'))
fns = [dict(f, crate=c['name']) for c in d['crates'] for f in (c.get('functions') or [])]
prod = [f for f in fns if not f.get('is_test')]

BARE = {'handle','process','do','run','apply','update','check','build','load','call','read','write','get','set','find','stop'}
rows = [(f['name'], f.get('kind'), f.get('container'), f['file'], f['line']) for f in prod
        if f['name'] in BARE and f.get('kind') in ('inherent_method', 'free_fn', 'nested_fn')]
print("P5 行数:", len(rows))                                     # -> 22
print("P5 trait_method:", [(f['name'], f['file'], f['line']) for f in prod
      if f['name'] in BARE and f.get('kind') == 'trait_method'])  # -> load@plugin_policy.rs:55, call@plugin-host/src/lib.rs:48
print("P8 首词:", collections.Counter(re.split(r'[^a-z0-9]+', f['name'].lower())[0] for f in prod).most_common(10))

MARK = ('_owned','_leak','_boxed','_vec','_raw','_strings','_string','_opt','_impl')
idiom = ('into_owned','from_owned','from_raw')
print("P6 全量:", len([f for f in prod if any(m in f['name'] for m in MARK) and f['name'] not in idiom]))  # -> 25
```

```text
# G-3 别名（P7）
grep -rn "pub use .* as " --include=*.rs core/src run_method/src build_method/src mcp/src studio/src cli/src plugin-host/src debug_method/src macro/src | grep -v _tests
# -> 4 行（plugin.rs:35、run_method/lib.rs:49-50、lazy_wasm.rs:24）；pass.rs:346 的命中是断言字符串
```

```text
# G-4 -> bool 谓词（P4）
grep -rn "fn [a-z_0-9]*[^ ]*(.*) *-> *bool" --include=*.rs core/src build_method/src run_method/src macro/src debug_method/src plugin-host/src studio/src mcp/src cli/src conventions/src | grep -v _tests | grep -vE "fn (is|has|can|should|must|needs|allows|supports|contains|requires|owns|includes|accepts|matches|keeps|wants)_"
# -> 3 行（release.rs:191、source/walk.rs:54、plugin/contracts/contracts.rs:460）
```

**三处口径必须在引用本报告时保持一致**（t27 §3.1 的教训）：
- 无载荷目录 = **谓词 P2d（递归，= NAM-07 的 A1）** → **34 = 发布面 28 + `examples/`/`studio/tests/fixtures/` 6（豁免，不参与收平）**；验收目标是**发布面内为 0**（三个口径：仓库 34 / 发布面 28 / 豁免后可达 0）。**不要**引用 39（NAM-07 的 **A2**，inventory 直接 `.rs`，会含 5 个有子模块的目录）或 67（**A3**，文件名重复口径）。**注意 A2 与 §G 的 `P2b` 不是同一谓词**：`P2b` 是 `os.listdir` 单条目，给 34。
- 模块名与文件名 = **谓词 P1（`module_path` 非空）** → **14 处**不同名（有意）；**不要**引用"394/394 一致"。
- `*_tests.rs` 镜像 = **谓词 P2c（文件名）** → **43/44**；模块名统一挂 `mod tests`，与镜像无关。
- 裸动词 = **谓词 P5** → 全量 24 行（22 + 2 个 trait 声明），白名单 9，**非入口位 15**；不要引用第一版的"8 处"。
- 实现词根 = **谓词 P6** → 全量 25 行，计入 D-7 的 **8 行**；不要引用第一版的"5"。

**报告自检**：本文件里的 `file:line` 全部来自当前工作树的 `read`/`sed`/`grep` 输出；每条"干净"结论都在 §F 与 §G 给出谓词与命令。第 1 轮（t25）中 5 处不可复算的数（NAM-40、D-3、D-5、D-7、§0 目录数）已在 t30 修正。
