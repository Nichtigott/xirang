# xirang MCP 优化执行记录（按《xirang-mcp 完整执行清单（2026-10-04）》）

本文件是那份执行清单在本检出里的**执行记录**：每一项的证据、改法、验收怎么测、实测结果与提交。
执行清单本身（用户的附件）是入口；这里只记"做没做、怎么自证的"。

写法定：交付给维护者的 markdown **中文为主**，代码标识符、路径、命令、测试名保留英文原文。

## 状态总览

| 阶段 | 项 | 状态 | 提交 | 验收怎么测 |
| --- | --- | --- | --- | --- |
| 0-1 | bench 进仓 | **完成**（规模 bench 新写；hardbug 与 S1–S7 已确认可用） | 本地脚本（`.py` 不入库） | 见 §阶段 0 |
| 0-2 | 四轴回放固化 | 待做 | — | 见 §阶段 0 |
| 0-3 | 基线落盘 | **完成**（五档曲线，实测到 t4） | `1156794` | 见 §阶段 0 的曲线表 |
| M1 W4-1 | F1 显示名漂移 | **完成** | `e21d6f3` | `consistency_tests::a_displayed_field_name_is_the_spelling_the_declaration_wrote` |
| M1 W4-2 | F6 路径类校验 | **完成** | `aadf372` | `apply_tests::a_logical_path_is_refused_by_a_typed_plan_with_the_spelling_to_use` |
| M1 W4-3 | F8 离线脚手架 | **完成** | `5281de5` | `scaffold::project::tests::a_request_names_the_source_and_git_is_never_guessed` + `tools/xirang-external-rehearsal` |
| M1 W4-4 | 拒绝消息实例化 | **完成**（adopt 指针待 W5-1） | `23aa8c3` | `apply_tests::a_refusal_carries_a_request_built_from_this_trees_names` |
| M1 W4-5 | 闭合标志 | **完成** | `d9510f6` | `consistency_tests::the_answer_opens_with_the_census_and_closes_conservatively` |
| M1 W4-6 | 离群行原文片段 | **完成** | `25ab03b` | `consistency_tests::an_outlier_carries_the_lines_it_deviates_on` |
| M1 W4-7 | 答案形状路由 | **完成** | `b53439c` | `client_tests::the_answer_shape_routing_is_on_both_pages` |
| M1 W1-2 | status 并入首行 + next 实例化 | **完成** | `d9510f6` | `consistency_tests::the_answer_opens_with_the_census_and_closes_conservatively`、`why_tests::the_answer_carries_the_census_and_a_closure_line` |
| 门禁 M1 | 整仓门禁 + hardbug 电池 | 见 §门禁 M1 | — | — |
| M2 W1-1 | 广告收敛 2+1 | **完成** | `03e427f` | `protocol_tests::the_advertised_frame_and_the_handshake_fit_their_budgets` |
| M2 W1-4 | 描述节食 | **完成** | `03e427f` | 同上（三条描述 + 三张 schema 的字节预算） |
| M2 W2-4 | apply 面清单有界化（D1） | **完成** | `6a605b8` | `apply_tests::the_write_reply_is_bounded_and_full_buys_the_tree_back` |
| M2 W5-7 | 写后响应节食（D4） | **完成** | `0d8fcd6` | `a_deepen_adds_a_layer_inside_the_face_and_prices_the_other_reading` |
| M2 W5-8 | new_project 自带初始普查 | **完成** | `0d8fcd6` | `new_project_tests::the_creation_reply_carries_the_initial_census` |
| M2 W1-3 | cargo 输出瘦身 | **完成**（边界索引合并已是现状，本次做分组折叠） | `15ab15b` | `check_tests::the_passing_groups_fold_and_verbose_undoes_it` |
| M2 W2-1 | registry 默认档普查化 | **完成** | `40cf6ca` | `registry_tests::the_default_is_a_census_whose_size_follows_the_levels` |
| M2 W2-2 | consistency 免责文本会话化 | **完成** | `0785094` | `consistency_tests::the_bounds_are_said_once_per_session_and_root` |
| M2 W2-5 | apply 成功回复节食 | **完成**（D2+D3） | `6a605b8` `0785094` | 见 `apply_refusals_tests::the_write_disclaimer_is_said_once_per_session_and_project` |
| M2 W2-6 | `--list` 瘦身 schema | **完成**（口径已裁：默认有界、按需无界） | `9f06025` | `client_tests::every_discovery_page_is_bounded_with_a_way_to_the_whole_text` |
| M2 W2-7 | `--list --all-slim` | **完成** | `9f06025` | `client_tests::the_thirteen_tool_view_is_one_small_call` |
| M3 W3-1 | 记录增肥三列 + 读写成对 | **完成** | `63ab9ec` | `manifests_tests::the_record_carries_the_source_hash_fields_and_calls_a_derivation_agrees_with`、`every_published_column_reads_back` |
| M3 W3-2 | 发布路径记录优先（普查/分页 + 显示声明事实 + `logical_path`） | **完成** | `8869a69` `cfef385` `a751612` | `published_tests::{a_current_record_is_shown_and_its_levels_are_counted, a_published_answer_and_a_derived_answer_carry_the_same_facts}` |
| M3 W3-1b | 记录补 `parent_node`/`owns_registry` | **完成** | `6693bda` | `manifests_tests::a_module_named_parent_resolves_to_its_identity` |
| M3 W3-3 | 在线分析评审规则（门禁 `online_analysis`） | **完成**（21 个桥文件各写了理由） | 本轮 | `conventions::online_analysis_tests::every_online_answer_says_why_the_build_cannot` |
| M4 W5-2 | affected 补计划层 | **完成** | 本轮 | `affected_tests::a_changed_face_names_the_plan_entries_that_target_it` |
| M4 W5-4 | consistency 升级修复器入口 | **完成** | 本轮 | `consistency_tests::the_repair_the_comparison_hands_back_clears_the_outlier` |
| M4 W5-5 | check 失败按类型路由 | **完成** | 本轮 | `check_tests::a_red_routes_by_its_kind_rather_than_to_the_default_hint` |
| M4 W5-3 | deepen 预览附消费方 diff | **完成** | 本轮 | `apply_consumers_tests::the_consumer_story_is_the_same_in_the_preview_and_after_the_write` |
| M4 W5-6 | apply 树驱动一次成型 | **大半**（① 从树推家族契约 + ③ 写后家族判定已落地；②"一次生成完整文件"本已如此） | 上轮+本轮 | `apply_consumers_tests::{a_write_reports_whether_the_new_face_landed_like_its_siblings, an_omitted_family_field_is_inherited_and_reported}` |
| M4 W5-1 | adopt（单独设计评审后动工） | 待做 | — | — |
| M5 W6-1 | 编译墙归因 | **完成**（报告 + 10k<5min 实测达标） | `22c882f` | 见 §阶段 M5 |
| M5 W6-2 | 中大型项目参考结构 | **进行中**（验收已实测；四步已落，下一步＝兄弟集合走记录，家族分层仍开） | `c41bac3` `6fb4e42` `460eafe` `36a107b` | 见 §阶段 M5 |

## 阶段 0：测量地基

### 0-1 bench 进仓（规模 bench 已新写；两套既有 bench 确认可用）

执行清单说"`scale-logs/gen_scale_tree.py`、`scale-logs/scale_bench.py` 移入 `tools/`"。**这两个文件
在本检出不存**（`scale-logs/` 目录不存在，全盘也没有这两个名字）⇒ **新写**，不是搬家：
`tools/gen_scale_tree.py`（按面数生成一棵规模题树：直接写 `src/`，不走 `new_project`/`apply`——生成器
造规模，被测的是**读**那条路；生成的清单带空的 `[workspace]` 表，否则 Cargo 拒绝给包命名）与
`tools/scale_bench.py`（五档 `t1:100 / t2:1000 / t3:5000 / t4:20000 / t5:50000`，每个读数 = 一次
`--call` 的墙钟毫秒与回复字节，取 `--repeat` 次中位数，落 `scale-logs/scale-bench.json`）。

**注意**：按维护者 2026-10-04 的指示 `*.py` 不入库，因此这两个脚本**只活在本地**；它们的**口径**写在
本文件与脚本头部（可据此重建），而不是靠"仓库里有那个文件"。

另外两套既有 bench 已确认可用：`tools/xirang-mcp-hardbug`（四类题配方 + 旁路 tokens 账 + `report`；
本轮 `build` 四类各建一棵并**自证**成功）与 S1–S7 夹具（`build-s*.sh` / `write-briefs.py` /
`score-s*.py` / `axes2.py`）。

### 0-3 基线落盘：规模曲线（2026-10-04 实测）

`tools/scale_bench.py --binary target/debug/xirang-mcp --all --tiers t1:100,t2:1000,t3:5000
--repeat 2`（本机，debug 二进制；中位数）：

| 档 | 面数 | `registry` 默认 | `registry --full --limit 200` | `status` | `search` | `read` | `why` |
| --- | --- | --- | --- | --- | --- | --- | --- |
| t1 | 101 | 317ms / **863 B** | 339ms / 13.8 KB | 142ms | 334ms | 71ms | 390ms |
| t2 | 1001 | 2923ms / **866 B** | 3099ms / 26.9 KB | 874ms | 2966ms | 51ms | 2927ms |
| t3 | 5001 | 14102ms / **866 B** | 14365ms / 26.9 KB | 3773ms | 14233ms | 51ms | 14517ms |
| t4 | 20001 | **55706ms** / **869 B** | 56389ms / 27.0 KB | 14907ms | 57892ms | 74ms | 58829ms |

**两条读数**：

1. **W2-1 的"拍平"成立** ✓：默认 `registry` 的**字节数**在 101 / 1001 / 5001 面上是 863 / 866 / 866
   ——它是普查（计数 + 逐层），不是清单；`--full` 也在 26.9 KB 封顶（200 行一页）。清单要求"50k 档
   registry 默认 ≤2 KB"，按这条曲线它由**层级数**决定而与面数无关。
2. **读路径是 O(面数)/次调用** ✗✗：`registry`/`search`/`why`/`status` 每次都重走一遍 `src/` 的推导，
   317ms → 2.9s → 14.1s → **55.7s**（≈2.8ms/面，线性）⇒ **t4 实测 55.7s**、外推 50k ≈ 140s，
   与清单里"t4 读延迟 211s"是同一个现象（本机更快，量级一致）。只有 `read` 是平的（51ms，它只读一个文件）。**这就是 M3（记录路径）要消灭的东西**，而它现在
   有了一台可复现的量器与一条基线。

四轴预算基线（写入 175,959 / 未命中 17,683 / 命中 36,226 / 输出 9,824 / 成本 291,967）来自四轴估算
§3，那份文档不在检出里，因此要在两臂重跑时**从会话日志重算**（方法已有：`tools/xirang-mcp-hardbug
tokens <session>` 或 `target/bench-scenarios/axes2.py`，语料是那 12 个会话）。

## 阶段 M1：信任清零

### W4-1 F1 显示名漂移

**证据**：`consistency` 把两个 trait 字段固定报成 `handle_traits`/`part_traits`，而内核接受
`handle_contracts`（参与编译检查的路径）与 `handle_traits`（纯标签）**两种**拼写 ⇒ 一个写
`handle_contracts: [crate::control::ControlHandle]` 的面，读者拿到的是另一个名字，而两者的取值住在
各自的位置上。

**改法**：`ShapeField` 同时带**规范键**与**拼写清单**；比较按键（同族选另一种拼写不算漂移），
`shape` 行与 `lacks` 行点名**文件真正写着的**那个拼写；`shape` 信号的存在性集合按键归并；
`shape_field_names()` 把两种拼写都列出来。

**全仓同类普查**：逐个字段 grep 展示层（`handle_traits`/`handle_contracts`/`part_*`/
`registry_rule`/`stable_name`/`needs_registry`/`runtime_checks`/`flow_provider`/
`getting_from_other_registry`），命中处只有 `apply` 的 `EDITABLE_FIELDS`、`usages` 的 21 栏回读、
`explain`/`digest` 的字段行——它们点的是**可编辑字段名**（`name_zh`/`registry_rule` 等，能原样回写
`apply`），只有 `consistency` 这一处点名了源码里不存在的拼写。⇒ 改一处即闭合。

**验收**：H2 题树重跑，`consistency` 每个可引用名可原样用于 `apply edit` 且编译绿。**本仓自证**：
新钉子两个方向都走（标本写 contracts ⇒ 显示 contracts、且兄弟写 traits 不算缺；兄弟真缺 ⇒ 行里给的
仍是标本的拼写），并对**每一种拼写**查内核词表 `FACE_FIELD_ORDER`。

### W4-2 F6 路径类校验

**证据**：`apply {action:"cut"}` 接受读者手上的逻辑路径 `root/control/dial`，并把它渲染成
`cut(root/control/dial)` 拼进一份其余每条都是 `crate::…::NODE_ID` 的计划。两种拼写在这套语法里
含义不同（逻辑路径由**构建期选择器**解析，类型化那一形由**编译器**解析）⇒ 读回来的条目不是调用方
要的那条，而写进去的是宿主的源码。

**改法**：`apply_cut::path_class_refusal` 在渲染之前、写盘之前检查输入；只在计划**通篇**类型化时拒绝
（反方向会拒绝语法本就接受的请求）。`typed_spelling` 从面自己的 `source` 经**构建自己的**
`build_method::source_module_path` 推导拼写——不从逻辑路径拼装，因为 `object` 那一层只有源码布局知道。
推导不出时说"这棵树里没有这个面"并指向 `registry`。

**验收**：喂 `root/control/dial` ⇒ 拒绝且消息含 `crate::control::object::dial::NODE_ID`（并给出
来源文件）；喂正确拼写 ⇒ 一次成功；被拒那条路上什么都没写。三条都在钉子里走了一遍。

### W4-3 F8 离线脚手架

**证据**：不在检出内运行的工具会往生成清单里写 `git = "https://github.com/…"`——一个需要联网才能解析
的来源，而调用方从没要求过。

**改法**：`DependencySource::Registry`（只写版本要求，`cargo add` 会写下的那一形）；
`detected_source` 的回落从 `Git` 改成 `Registry`，从检出内跑仍是 `path`（唯一完全离线可解析的一形）。
词表收进 `scaffold::requested_source(dependency, git, …)`：`path`/`registry`/`git`；只给 URL 而没说
`dependency: "git"` ⇒ 拒绝；`path` 而没有检出 ⇒ 拒绝并指向 `registry`，不静默换源。桥的
`new_project` 接受 `dependency`/`git` 两个键（schema 同步）；CLI 的 `--git` 照旧显式。

**验收**（清单口径：断网沙箱 `new_project` → `cargo build` 绿）：走 `tools/xirang-external-rehearsal`
的第三条腿——它在检出之外用本检出的 CLI 生成二进制与库两个项目并各跑 `cargo test --offline`。

### W4-4 拒绝消息实例化 + 边界文档化

**证据**：写入工具的拒绝都点了形状，但用的是占位符（`<node>`/`<snake_case>`/`<Kind>`）⇒ 读者还得自己
找合法父面、想一个不撞的模块名、把切口的类型化拼写推出来，而这三件都是**请求所在那棵树**的事实。
S1/S6/S7 上的用法型拒绝正是这么来的。

**改法**：`apply::write_example(root, namespace, action)` 按动作搭完整请求（`parent` 取第一个拥有
注册机的真实面、`module` 取树里没人用的名字、`node` 取真实面路径）；`cut` 示例的 `cut` 半取自
`typed_spelling`、`graft` 半取自计划**已经在用**的拼写（`apply_cut::plan_graft_example`，经内核
解析器读回）；`promote` 的 `selector` 取本包真实的 `external-grafts` 目录。接进 `apply` 的
缺/错 action、`add`/`edit`/`rename`/`delete`/`deepen`/`cut` 的缺键支、`promote` 的缺
selector/confirm 支、`new_project` 的三个必填键支。`apply` 的**描述**（客户端真正拿到的那份）写明
分界：add/deepen/cut/promote 是**追加**类（改声明，能落到手写面上）；edit/rename/delete 是**改写**类
（改写文件，只落生成过的面）；执行器的 `this module was not generated by XiRang` 拒绝也附同一句。

**仍开的一半**：清单要的那句"指向 adopt（W5-1）"**还没写**——`adopt` 这个动作不存在，现在写就是承诺面
超过能力面。等 W5-1 落地后补，并同时补一条钉子。

**验收**：S1/S6/S7 重跑用法拒绝 = 0（待合并重跑）；agent 仅读描述即可预知放行/拒绝（本仓自证：钉子直接
读 `tools()` 交出去的 `description`）。

### W4-5 闭合标志

**证据**：debug 电池上两个方向都量到——一个停在一处偏离、对其余只字不提的答案让代理猜要不要继续；
一个总带 `next` 行的答案让每个答案看起来都没做完。

**改法**：`tools::closure(closed, remaining)`——只说两种之一，判定**从保守**：closed 只在"这个答案
自己的形状会提出的每个问题都已被回答"时写；任何读不了的文件、任何被点名的偏离都是 open，并附上解决它
的命令（命令是**可粘贴的 `--call` 拼法**，已用真二进制验过旗标与数字解释）。`consistency` 两支
（specimen/parent）与 `why`（主答案 + "这一行不在任何函数里"那条早退）都接上。

### W4-6 离群行原文片段

**证据**：只点名字段 ⇒ 读者还要再开一次文件（多一次仪器调用）；对照工具对同一个问题回整份文件
6,781 字符的片段。

**改法**：离群者**自己有**的那行先引（`lines_naming`，行首带 `路径:行号`）；缺字段那种引不出行的
情况用它的**声明块**代替（`declaration_block`，从宏调用到收尾大括号）；随后引标本（多数派）的对位行。
父面 `api`/`shape` 信号同理：本兄弟自己多调/多声明的引它自己的文件，别人都有的那个名字引**多数派写它的
地方**。上限 `EXCERPT_LIMIT = 10` 行/离群者，单行 `LINE_LIMIT = 200` 字符后截断带 `…`；答案的边界句
里写明"引的是摘录不是文件"。

### W4-7 答案形状路由

**证据**：两种问题形状要两种答案（理解型＝"这个文件是干什么的"由整份文件回答；结构型＝"里面有什么、
谁调用它"由有界答案回答），而这条规则只活在设计文档里。

**改法**：`SHAPES_SHORT`（`--list` 真正印的那页）与 `INSTRUCTIONS`（`initialize` 交的那页）各加一条
`**Answer shape**` / `**答案的形状**` 路由句；`locate` 的 `next` 行换成**排名第一那个文件**的
`--call read --path <file> --line <n>` 与 `--call callgraph --function <name>`，并新增 `shape` 行
指向 `--call read --path <file> --whole`。`--list` 总长仍在 5000 字符预算内（实测 4,991）。

### W1-2 status 并入首行 + next 实例化

**改法**：`consistency::tree_census` 给出 `tree: N face(s), M file(s)`，作为 `consistency`（两支）
与 `why` 的**首行**（`check` 本来就有 `tree   N rust file(s), M function(s)`）⇒ "先发一次 `status`
只为知道树有多大"这件事不必再发生。`why` 的 `next` 换成这个答案已经知道的名字（`--call check --face
<面>`、`--call callgraph --function <函数>`、`--call read --path <路径> --line <行>`），没有归属面时
退到 `--call registry` 而不是印 `{face}`。

## 门禁 M1

| 门禁 | 怎么跑 | 结果 |
| --- | --- | --- |
| 格式 | `cargo fmt --all -- --check` | 绿 |
| 工作区测试 | `cargo test --workspace --offline` | 绿（默认面 151 项 conventions 全绿；`size` 棘轮先红后修，见下） |
| clippy 默认面 | `cargo clippy --workspace --all-targets --offline -- -D warnings` | 绿 |
| clippy all-features 面 | `cargo clippy --workspace --all-targets --offline --all-features -- -D warnings` | 绿 |
| 发布表 | `tools/xirang-publish --check-table` | 绿（`dependency table matches the manifests (3 crates)`） |
| 十面测试 | `tools/xirang-test` | **先红后绿**：四个面各一项失败，全是 `studio::…::installed_studio_never_exports_cargo_git_cache_paths`——它还在断言 F8 之前的行为（见下），已在 `cd1ff1b` 跟上 |
| 生成物编译 | `tools/xirang-external-rehearsal` | 绿：`external rehearsal passed: control-button built and tested outside the checkout (CLI half: ran; scaffold half: ran)` ⇒ **W4-3 的验收**（检出之外生成的两个项目 `cargo test --offline` 都过） |
| 棘轮 | `cargo test -p xirang-conventions` | 绿（M1 把 `apply.rs`/`consistency.rs`/`apply_tests.rs` 顶过 600/800 ⇒ 按本仓规矩拆出 `apply_refusals.rs` / `consistency_support.rs` / `apply_cut_tests.rs`，见 `5db85cc`） |
| hardbug 电池可用 | `tools/xirang-mcp-hardbug build <dir> <class>` ×4 | 四类各建一棵并**自证**：h1/h3 红→绿（`cargo test` 101→0）、h2 由 `consistency --specimen` 点名、h4 普查点名 `ZeroArm::Refuse` 第 44 行 |
| hardbug 电池命中 10/10 | 两臂作答 | **合并到 M2 门禁**（同下） |
| S1–S7 零用法拒绝 | 两臂重跑 | **合并到 M2 门禁**（理由见下） |

**两处"先红后绿"如实记**（都不许拿"两臂都完成了"掩盖）：

1. `size` 棘轮被 M1 自己的改动顶红 ⇒ 拆模块（`5db85cc`），不是加基线。
2. `installed_studio_never_exports_cargo_git_cache_paths` 是**被 W4-3 正确的行为变化打红的旧断言**：
   它断言装出来的副本写 `git = "…"`，而 F8 的修法正是让那一支写已发布版本要求。断言跟着行为走并更强
   （两条都不含 `git =`、不含本机路径），提交 `cd1ff1b`。

**关于把两臂重跑合并进 M2 门禁**：维护者的规矩是"一轮轮测试很贵，做到当前设计的最好再去测试"，而 M2 的
门禁本来就要"四轴重算 12 会话语料"。同一套夹具连着跑两遍，第二遍只是把第一遍的数字复述一遍。因此 M1 的
机械门禁照跑，**两臂的实机重跑只在 M2 门禁做一次**，两阶段的用量从那一次里各取所需（M1 取"用法型拒绝
= 0"，M2 取四轴）。

## 阶段 M2：经济性主刀（进展）

### W1-1 广告收敛 2+1（完成，`03e427f`）

**证据**：`tools/list` 一帧交出全部二十七个工具（约 36,000 字符散文）；`initialize` 交出的长指引页约
7,000 字符。那一轮量到这两个数字正是"第一个决定很贵"的来路（25 步 vs 对照工具的 14 步）。

**改法**（**不删任何能力**）：新增 `tools::advertised()`，`tools/list` 只广告 `xirang.check` /
`xirang.apply` / `xirang_tools`；`tools()` 仍是完整目录（28 项），`--list`、`--list <tool>`、
分派、归属分类都读它。`xirang_tools` 不给参 ⇒ 与 `--list` **同一份实现**的一行式清单
（`client::list_tool_lines`）；`tool: "<名字>"` ⇒ 与 `--list <tool>` **同一份实现**的整页
（`client::describe_tool`）——广告与手册各只有一份实现，因此不可能漂移。长指引页改名 `GUIDANCE`
（`--shapes` 仍印它，内容一字未删）；`INSTRUCTIONS` 变成握手用的短文本。

**实测**（线上回复的形状）：广告帧 ≤ **4,200** 字节、三张广告 schema 合计 ≤ **4,300**、
`INSTRUCTIONS` ≤ **550** 字节；目录仍有 **28** 项（≥27，一个能力没少）。

**命名注**：`xirang_tools` 按清单原文。它不是 `xirang.*` 命名空间里的能力，而是"关于目录本身"的
那一个，因此一行式清单的钉子放行这一个例外（`first.starts_with("xirang.") || first.starts_with("xirang_tools — ")`）。

### W1-4 描述节食（完成，`03e427f`）

**改法**：广告位工具各带一份**只留决策信息**的描述（check ~470 / apply ~430 字节）与一张**只留决定
这次调用的键**的 schema。细则没有被删：完整描述与完整 schema 仍在 `tools()`（`--list <tool>` 印）、
拒绝文案仍带实例化形状（W4-4）。`apply` 的广告 schema 里也补上了 `full`（W2-4 新键）与
`cut`/`graft`/`to`/`full`（此前完整 schema 里**根本没有**这四个键——一处承诺面落后于能力面的旧缺陷）。

**口径冲突（记下来，等维护者或下一轮裁）**：W1-4 说"细则全文迁入 `--list`"，而 W2-6 说"`--list` 返回
瘦身 schema"。两者指向同一个出口的两种形态。当前实现站在 W1-4 那一边（`--list <tool>` 仍是完整描述），
W2-6/W2-7 因此标"待定"而不是"完成"——不把没做的说成做了。

### W2-4 apply 面清单有界化（完成，`6a605b8`）

**证据**：每次写入印出**整棵树的每个面**。九面夹具上是几行，五万面的树上就是整份载荷。

**改法**：默认回复 = 普查行 `faces N` + 发生变动的那个面（`changed  <path>  <kind>  <source>`）
+ `truncation::withheld` 扣下行（出路 `full: true`）；`full: true` 买回全量。`full` 同时进广告 schema
与完整目录 schema。**实测**：预览 ≤4.4K、落盘 ≤2.2K 字节（钉子直接量 `reply.len()`）。

### W2-5 apply 成功回复节食（D2 完成，D3 仍开，`6a605b8`）

**D2**：`editable_fields_line` 是**拒绝**的辅助，却搭在每条成功回复上。成功回复删掉，拒绝路径照旧带。
那条"每次回复都要带形状"的旧钉子按新意图改写，并且**两半都断言**（成功里没有、拒绝里有）——否则
"搬走了"与"没了"看起来一样。

**D3（仍开）**：`consequences` 的固定块（entry plan 行 + "not covered" 免责）**会话化**——同会话第二次
`apply` 不再含全文。它需要进程级会话状态，而桥同时有长期 stdio 服务与 `--call` 一次性两条入口；
"什么算同一个会话"要一个设计决定，因此留到 M2 收尾时定，而不是先写一个可能错的实现。

### W5-7 写后响应节食（完成，`0d8fcd6`）

`deepen` 的 `alternative` 与 `note` 两块是**决策辅助**，属于"决定还没做"的那一刻：预览给全文，落盘改印
一行 `alternative was stated in preview; …`。验收两半（预览含全文、落盘不含）都钉在同一条测试里。

### W5-8 new_project 自带初始普查（完成，`0d8fcd6`）

落盘/预览回复新增 `tree: 1 package, N rust file(s); … M of them own a registry — \`registry\` would
say the same thing`，用的是写入路径**刚刚**读过的那批源码（`load_sources` + `face_views`）⇒ 零额外
推导，消灭了"写后立刻再发一次 `registry`"那一步。

### 尚未开始（M2 余项）

`W1-3` cargo 输出瘦身 · `W2-1` registry 分页/普查化 · `W2-2` consistency 免责文本会话化 ·
`W2-6`/`W2-7`（口径待裁） · 决策点 `DP-1`/`DP-2`/`DP-3`（要门禁实测才裁，不许预设）。

### 修 W1-1 自己带出来的两处（`4f5cf85`）

真跑一遍才发现的，**两条钉子当时都放行**——记下来是因为这正是"钉子钉的是那句话，不是那个能力"的老毛病：

1. **`--call xirang_tools` 解析不到**：命令行对每个裸名补 `xirang.` 前缀，而目录工具没有点号
   ⇒ 解析成 `xirang.xirang_tools` ⇒ 广告让会话"用它去够其余一切"的那个工具在命令行上回
   `unknown tool`。修法：`client::resolve_name`——**精确命中目录名者胜**，只在需要时补前缀；三处解析
   都改走它。钉子两半都钉（新名字可达、`callgraph` 仍解析成 `xirang.callgraph`）。
2. **预算量错了对象**：原钉子量 tools 数组的 serde 序列化，而客户端付的是**完整回复行**（含
   JSON-RPC 信封，实测 4,245）。现在钉子走真 stdio 入口量那一行；据此把两条广告描述各削一句，
   实测线上那一行 = **4,194 ≤ 4,200**。

**可复用两条**：**(a)** "预算量在哪个形状上"必须与"客户端付费的形状"一致——量数组不等于量回复；
**(b)** 新广告出去的名字必须**自己可达**，钉子要钉"那个名字真的能调到"，不是"那句话真的写了"。

## 下一轮从这里接着做（本轮的收尾状态）

**已闭合**：阶段 0 的可测部分 + 阶段 M1 全部 + M2 的 W1-1 / W1-4 / W2-4 / W2-5(D2) / W5-7 / W5-8。
**当前门禁**：`cargo fmt --check`、`cargo test --workspace --offline`、两面 `clippy -D warnings`、
`tools/xirang-publish --check-table`、`cargo test -p xirang-conventions`（含 600/800 棘轮）
全绿；`tools/xirang-test` 十面在修掉 studio 那条旧断言后应全绿（下次整跑复核）；`tools/xirang-
external-rehearsal` 绿（W4-3 的验收）。

**明确仍开**（不许说成做完了）：

| 项 | 为什么还开着 |
| --- | --- |
| 门禁 M1 的两臂项（hardbug 命中 10/10、S1–S7 零用法拒绝） | 需要 LLM 臂；与 M2 的四轴重算**合并成一次**（理由见 §门禁 M1） |
| W1-3 cargo 输出瘦身 | 未动 |
| W2-1 registry 默认档普查化 | 未动；动手前要先决定"包根默认是否一律普查"——它会改动 `registry_tests` 的全部期望形状，值得单独一轮 |
| W2-2 consistency 免责文本会话化 | 未动；与 W2-5(D3) 同一个"什么算同一个会话"的设计决定 |
| W2-5 D3 | 见上 |
| W2-6 / W2-7 | 与 W1-4 的口径冲突（"细则迁入 `--list`" vs "`--list` 返回瘦身 schema"），先裁口径再动手 |
| DP-1 / DP-2 / DP-3 | 要门禁实测才裁，不许预设 |
| M3 / M4 / M5 | 未开始（W5-1 `adopt` 按清单要求"单独设计评审后动工"） |

## 阶段 M2 收尾（本轮补完）

### W1-3 cargo 输出瘦身（`15ab15b`）

**第一半已是现状**：三条边界索引早已并成一条 `SHORT_BOUNDARY`（`not covered: the reachability
column (…) · the branch column (…) · this census (…)`），默认只带它，逐栏全文只在 `census: true`
时出现——如实记，不重复做一遍。

**第二半动手**：一次 `cargo test` 每个测试二进制品一组，每组印自己的 `test result:` 行。现在**带失败的**
逐行印（上限 12，超出走 `truncation::withheld`），**通过的**折成
`result N group(s) passed — pass \`verbose: true\` for each line; first: …`；`verbose: true` 全印。
schema 两处同步。

### W2-1 registry 默认档普查化（`40cf6ca`）

默认 = 普查（`faces N` + 逐层计数，最多 20 层，超出 withheld）+ 一行"`full: true` prints the rows,
200 per page and `offset: <n>` for the next"；`full: true` = 行，一次一页（`rows 1-200 of N` +
游标，`limit` 1–200）。**拍平是承重的**：普查大小跟**层级数**走、不跟面数走（实测 4 面 vs 120 面，
答案长度只差 <40 字节，两者都 ≤2 KB）。工作区的 `full` 仍走合并视图（跨成员的页索引会是同一棵树的
第二套坐标）。原"整份清单"渲染器被分页路径取代并删除。

### W2-2 + W2-5(D3) 常量块会话化（`0785094`）

**"什么算同一个会话"这个设计决定先定下来**：单位是**服务一个客户端的那一个进程**——stdio 服务活一条
连接，"已经说过"= "已经对这个客户端说过"；一次性 `--call` 是它自己的进程，因此永远是第一次（它没有更早
的答案可回指）。新增 `mcp/session.rs`（进程级 `static`，键里带根 ⇒ 两棵树各有自己的第一次，测试的
临时目录不会互相污染）。接上两处常量块：`consistency` 的两支共用 `consistency-bounds:<root>`；
写入路径的 `consequences` 用 `apply-consequences:<project>`。

### W2-6 + W2-7 发现载荷（`9f06025`，口径冲突已裁）

**裁法**：`W1-4`（"细则迁入 `--list`"）与 `W2-6`（"`--list` 返回瘦身 schema"）同时成立的读法是
**默认有界、按需无界**——长文仍住在 `--list`（没搬走），而默认那次调用只为"要不要调这个工具"付费。
实现：`describe_tool` 把描述压在 250 字节，截断句**必须**走 `truncation::withheld`（本仓有一条钉子钉
这句话的唯一拼写——我第一版手写了一句，当场被它抓住）；外加一条**自适应规则**：只有省下 ≥44% 才收窄
这一页（"短十分之一、却多花一次调用"比长一点但能作答更糟）。全文：`--list <tool> --full` /
`xirang_tools {tool, full: true}`。实测默认/全文（字节）：why 429/759 · digest 330/578 ·
check 343/4847 · apply 433/4579 · search 466/2282 · consistency 435/1329 ⇒ 每次截断都 ≤60%。
`W2-7`：`COMMON_TOOLS`（七种形状入口的并集，13 件）+ `slim_tools_page()`，由**同一份** `short_lines`
渲染（不可能与 `--list` 各说各话），末尾一行说出其余 15 件在哪；CLI `--list --all-slim`、桥
`xirang_tools {slim: true}`。

### M2 余下

`DP-1`/`DP-2`/`DP-3` 与门禁 M2 的四轴重算都要**两臂实机**数据，仍按 §门禁 M1 的决定合并成一次跑。

## 阶段 M3：记录路径（**已完成**；下表保留逐项证据）

### W3-1 记录 schema 增肥（完成，`63ab9ec`）

**先核对现状再动手**：`pruning_manifest.tsv` 的表头已经是
`# node  source  symbol  path  kind  registry_name  parent` —— 前四项事实 2026-09-29 就有了
（只是磁盘上 `examples/control-button/target/…` 那份是旧构建留下的三列表，一度让我以为没做）。
缺的是 **变化探测与调用面** 那三样。

**改法**（只落构建期**已经算出**的东西）：`FaceColumns` 新增
- `source_hash` = 该面自己那份源码字节的 `sha256_hex`；
- `fields` = 声明字段指纹（按内核 `FACE_FIELD_ORDER` 枚举、`name=raw` 排序去重后散列，**并把 `kind`
  折进去**——它是身份输入且不总被写下来，只散列"写下来的东西"会把字段相同、kind 不同的两个面判成相等）；
- `calls` = 该文件直接调用的名字（**内核自己的 `direct_calls` 规则**，与桥里每个读者同一条；无调用写
  `-`）。

**读写成对**：`PruningRow` 同步新增三列并在 `read_pruning_manifest` 里读回——只写不读等于发布了没人
够得着的事实（"承诺面落后于能力面"的另一个方向）。旧的三列表照旧解析、缺列读作 `None` 而不是报错。

**钉子**：表头即契约；哈希与调用名用读者会用的**同一批内核函数重算对账**；指纹查长度与两次构建间稳定；
写入→读回逐列相等。

### W3-2 全读工具记录优先（发布路径已完成；余项见 §阶段 M5）

**对着代码核到的现状**：`Member::tree()` 已经有 `Tree::Published` 分支，`PublishedTree` 也已经带着
`faces: Vec<PruningRow>`（含 `path`/`kind`/`registry_name`/`parent`，本轮又加了 `source_hash`/
`fields`/`calls`）——**发布路径本来就在**，缺的是"据它构造出读者要的东西"。

**顺手修掉一处自述落后**：`workspace.rs::derived_tree` 的文档说"记录不携带 `path` 与 `kind`"——
那句从 2026-09-29 起就不成立，本轮再假一层。已改成事实：记录仍**不**携带的是把 `FaceView`
**构造出来**所需的两样——解析后的父级 **`NodeId`**（记录发布的 `parent` 是声明拼出的 Rust 路径）与
`owns_registry`——以及"构建之后新增的面只在源码里"。

**W3-2 的下一步（按依赖排序，动手前先看这一段）**：
1. **W3-1b**：记录再补两列——`parent_node`（解析后的父级 `NodeId`；构建期拓扑已经算出）与
   `owns_registry`。有了它们，`FaceView` 才能**从记录构造**，而不是只能填空。
2. **`registry` 的普查/分页先走记录**（它是规模 bench 里最慢的一个，t4 55.7s）：普查只需要 `path` 逐层
   计数——连 `FaceView` 都不必构造。新鲜度用 `source_hash` 对现读字节比（比整棵树的解析便宜一个量级），
   记录不新鲜就回落并声明。答案要带 `tree published from <记录路径>`。
3. `status` / `search` / `why` 随后；注意它们慢在 **`load_sources` 遍历源码**（函数与调用面），不是慢在
   面的推导——记录里没有函数级数据，那半边要另想办法（`function_manifest.tsv` 只有「面 → 符号」）。
4. `check`/`verify` **永远 derive**（清单红线），不接记录。

### W3-2 第一刀：发布路径也普查，并**显示记录本来就携带的**声明事实（本轮）

**对着代码与真构建设施核到的三件事**（都写进注释与本节，免得下一轮再猜）：

1. **记录真的会被写出来**：把 `examples/control-button` 的 build script 逼着重跑（`touch build.rs`）后，
   新写下的 `pruning_manifest.tsv` 表头就是十列，`source_hash`/`fields` 是 64 位十六进制真值，
   `calls` 是内核规则算出来的。**它落在 cargo 的 `OUT_DIR`**（`target/debug/build/<pkg>-<hash>/out/`），
   而**桥读的是 `<包根>/target/xirang/out/`**（`check`/CLI 刷新的位置）——同一棵树的两份记录住两个地方，
   这一点以前没写下来过。本轮把新鲜的那份拷到桥读的位置，验到端到端。
2. **发布路径过去既无界又少用**：`registry` 在已构建的树上每个面印一行、**没有分页**（W2-1 的普查只做在
   推导那支），而且它的注里写着 `path`/`kind`/`registry_name`/`parent`"are derived facts and are not in
   the record"——**这句从 2026-09-29 起就不成立**（剪枝行一直在发布它们）。这是一句**落后于行为**的自述。
3. **`path` 常常是 `-`**：示例的三个面都没声明 `path`（逻辑路径是宏在运行期派生的），因此普查的层级只能
   按**源码目录**数，答案里明写 `this record predates the \`path\` column` 之外还多一句
   `level (source directory)`——两条坐标不混在一张计数表里。

**改法**：`render_published` 接上 `full`/`offset`/`limit`（与推导那支同一套分页常量与
`truncation::withheld` 出路）；默认给**普查**（层级计数，按已发布 `path`，缺失时按源码目录并明说）；
`full: true` 给**分页的行**，每行附记录携带的 `path=`/`kind=`/`registry_name=`/`parent=`/`calls=`；
`source_hash` 与 `fields` **不逐行重印**（64 位十六进制 × 每行 = 一屏噪声），注里说清它们发布在
`pruning_manifest.tsv` 里、以及新鲜度是 `check` 的问题。注重写成两半：记录**携带**什么、读者**还要**推导
什么（构建之后新增的面）。

**端到端证据**（拷入本轮 build script 真写出的记录后，`--call registry --full` 对示例树的输出）：
`kind=Slider/Control/Button`、`parent=crate::control::NODE_ID` **直接来自记录**——这些事实过去被那句话
否认，读者只能自己推导。

**钉子**：`published_tests::a_current_record_is_shown_and_its_levels_are_counted`（默认是普查且按**已发布
的**逻辑路径计数、默认不印行；`full` 页显示五个声明列与 `calls`，`calls=-` 的情形也钉住；注里说出两个
哈希住哪）、`records_that_do_not_describe_these_sources_are_reported_stale`（注的两半都断言）。

### W3-1b 记录补 `parent_node` / `owns_registry`（完成，本轮）

**为什么需要这两列**：`FaceView` 的 `parent` 是 `NodeId`，而记录发布的 `parent` 是**声明拼出的 Rust
路径**（`crate::control::NODE_ID`）——把那段文字变成身份是读者本要自己做的工作，也是"能不能从记录构造
`FaceView`"的分界。`owns_registry` 同理（它决定"这个面是不是文件夹面"）。

**改法**：`write_pruning_manifest` 变成**两遍**——第一遍索引「每个面拥有的模块路径 → 该面的 `NodeId`」
（子面可能先于它点名的父面被访问到，因此解析不能在遍历途中做），第二遍逐行解析父级身份。解析器用的是
**构建自己的** `face_view::resolve_parent` / `parent_of`（本轮把这两个从私有提到 `pub(crate)`，而不是
在清单写入方里另写一份"这条声明命名了什么"——那种拷贝正是会漂移的东西）。解析不出时写 `-`（树不拥有的
模块、解析不了的 `parent`），并明说 `-` 的读法是"推导"。

**端到端证据**（真 build script 写出的十二列表头 + 前两行）：

```
# node	source	symbol	path	kind	registry_name	parent	source_hash	fields	calls	parent_node	owns_registry
bdb4427c…	control/object/slider/slider.rs	-	-	Slider	-	crate::control::NODE_ID	981ae775…	c760ca90…	-	fb97ddd5f2b803d1b7f40a776d8a22d9	false
fb97ddd5…	control/control.rs	-	-	Control	-	-	620a2d06…	57a3a780…	-	b6a6bea94077152dbb7dd780a2708acf	true
```
slider 的 `parent_node` **正是 control.rs 自己的 `node` 列**（子面的解析父级＝父面的身份 ✓），而 control.rs
的 `owns_registry=true` 与它声明的 `needs_registry: true` 对得上。桥里 `registry --full` 现在直接印
`owns_registry=true|false`（读记录，不推导）。

**钉子**：`manifests_tests::the_record_carries_the_source_hash_fields_and_calls_a_derivation_agrees_with`
（表头十二列即契约；`parent_node` 查形状）、`a_module_named_parent_resolves_to_its_identity`（**两种形状**
都走：按模块点名的父级 ⇒ 与父面自己的 `id` **逐字相等**；点名树里没有的模块 ⇒ `None` 而不是猜）、
`every_published_column_reads_back`（读写成对）、`published_tests::a_current_record_is_shown_and_its_levels_are_counted`
（发布行显示 `owns_registry`，注里点名 `parent_node` 发布在文件里）。

**一处如实记**：`parent_node` 的期望值我第一版硬写了 `env!("CARGO_PKG_NAME")`，而写入方用的是
`package_namespace()`（运行期读环境）⇒ 两个不同函数，测试先红。改成"与另一行自己的 `id` 比"（不需要
测试侧做任何命名空间算术）——与第十轮那条"两边必须用同一个函数"同族，本轮又踩一次。

### 本轮：把"已构建的树"造出来并实测（W3-2 的验收形状）

**为什么要造**：M3 的验收是"**built 树**全部读工具 p95 < 3s"，而此前的五档曲线都是**未构建**的树（走推导）。
`tools/gen_scale_tree.py` 因此新增 `--host`：额外写 `build.rs`、`lib.rs` 里的 `host!()`、指向本检出的
`path` 依赖 ⇒ 这棵树真的能被 `cargo build`。

**造的过程本身抓到两个坑**（第一版模板只满足**读者**、不满足**编译器**，`cargo build` 报 203 个错）：
① 面文件的模板必须照 `new_project` 写出的形状（用桥脚手架一个真两面上机、把字节读回来抄），**不能**从
"推导解析得过"出发自己编；② `ROOT_FACE` 是普通字符串不是 f-string，我按 f-string 写了 `{{`，`rustc`
读成字面双括号 ⇒ 改成单括号。

**实测（101 个面，同一棵规模树，构建 vs 未构建，中位数）**：

| 工具 | 已构建 | 未构建 |
| --- | --- | --- |
| `registry` 默认 | **70.4ms / 1,966 B** | 278.8ms / 867 B |
| `registry --full` | 70.2ms / 21.2 KB | 245.3ms / 13.9 KB |
| `status` | 86.8ms | 96.6ms |
| `search` | 70.4ms | 258.9ms |
| `read` | 44.5ms | 41.1ms |
| `why` | 203.9ms | 263.5ms |

⇒ **记录路径在 101 面上已经把 `registry`/`search` 拉到 70ms 量级**（3–4×）；`read` 两支都平（它只读一个
文件）。**但发布普查比推导普查大**：2,824 → 1,966 B（修剪后），因为记录里 `path` 列是 `-`（宏派生的逻辑
路径不写进声明）⇒ 发布普查只能用**源码目录**做层级，而本夹具每个面各占一个目录 ⇒ "一面一层"。为此把
`CENSUS_LEVELS` 20 → 10 并把发布注从 734 B 收到 ~340 B，最坏情形回到 **1,966 B ≤ 2 KB**（W2-1 的预算）。

**由此得到 W3-2 的下一条具体活**：记录应当发布**逻辑路径**（构建期已经算出——`face_view::logical_path`
沿父链走），而不是只发布声明里写的 `path`；否则"按层级计数"这件事在记录侧永远比推导侧粗一档。

**顺带修掉一处并发缺陷（本轮真被它咬到）**：会话账本原先是**进程级** `Mutex`，而 `cargo test` 在一个进程里
**并行跑线程** ⇒ `the_bounds_are_said_once_per_session_and_root` 只在全量跑时红（另一个测试的
`session::forget()` 把它的键擦了）。改成 **thread-local**：stdio 服务本来就在调用它的那个线程上读并作答，
因此生产语义不变，而测试各自一个线程、互不干扰。连跑三遍 565 项全绿。

### W3-2 第二刀：记录发布**逻辑路径**（完成，本轮）

**上一轮实测指出的正是这一条**：记录里 `path` 是声明拼出的东西，对每个宏派生的面都是 `-`，于是发布
普查只能退到**源码目录**计数（一面一层）。而逻辑路径**构建期早就算出来了**——`face_view::logical_path`
沿解析后的父链走。

**改法（复用，不复制）**：
- `face_view::resolved_registry_name(face, module)` 与 `face_view::logical_paths(root, names)` 提为
  `pub(crate)`，**推导视图与清单写入方调同一对函数**——否则"注册名的回退规则"（省略时取模块名末段）
  会有两份拼写，而对同一个问题给出两个答案正是本仓最不许的事。
- `write_pruning_manifest` 现在**三遍**：① 索引「模块 → 面 id」；② 用**完整**的索引算
  `(id, 父级 id, 注册名)` 三元组；③ 逐行解析并发布。**第二遍必须独立**——子面可能先于它点名的模块被
  访问到；我第一版把解析塞在索引那趟里，钉子当场抓到：`parent:` 点名稍后访问的模块的子面拿到
  `root/child` 而不是 `root/dial/child`（注释里把这件事记下来了）。
- 新增第十三列 `logical_path`；`PruningRow` 同步读回；发布行也印 `logical_path=`；普查的层级回退链是
  **逻辑路径 → 声明路径 → 源码目录**，并且**说出用的是哪一个**。

**端到端（真 build script + `verify`，101 面）**：

```
… owns_registry  logical_path
… false          root/control/child000088
```
```
level (logical path prefix) and how many recorded faces sit under it:
  root                                         1 face(s)
  root/control                                 100 face(s)
```
⇒ 与推导侧**形状完全一致**（同一棵树两支都给出 `root 1 / root/control 100`），发布普查从 2,824 B 降到
**1,386 B**（此前那次"压到 1,966 B"是靠修剪行数与注长度换来的；现在形状本身对了，预算不是靠削出来的）。
已构建 101 面的默认 `registry` **88.7ms**（未构建 278.8ms）。

### W3-2 第三刀：把 M3 的"published vs derive 一致"做成钉子（完成，本轮）

**先把口径读清楚**：清单的验收写"t2 级对照答案与 derive **逐字节一致**"，而同一份清单又要求发布答案声明
`tree published from …`、推导答案说出它推导了——两条路**不可能**逐字节相同，**而且这是要求而不是缺陷**。
因此本仓把 M3 门禁的"答案一致"读作**事实**一致：总数行与逐层计数行。这个口径读法已写进钉子文档。

**改法**：新增 `published_tests::a_published_answer_and_a_derived_answer_carry_the_same_facts`——
同一棵树先按已发布作答、再把两份记录移走按推导作答，比对**事实行**（`faces N` 与层级行），并
`assert_ne!` 两份整答复（逼"必须说出自己来自哪里"这件事留在答案里）。

**它当场抓到一件事**（这正是这枚钉子的价值）：我手写的夹具记录里 `logical_path` 写的是
`root/control/slider`，而同一份源码推导出来是 `root/slider`（那两个面都声明父级为**根**）⇒ 发布普查说
`root 1 / root/control 1`、推导普查说 `root 2`。夹具的历史值错了，已改。**手写的记录可以编码一个推导
会反驳的事实——只有两边对账才发现。**

**101 面规模上的同一次对账**（已构建的 `h1` 树，记录的发布普查 vs 把记录移开后的推导普查，取事实行）：

```
diff <(registry 已构建 | grep -E '^faces |^  ') <(registry 推导 | grep -E '^faces |^  ')
→ 无差异（AGREE）
```
两条路都给出 `faces 101` + `root 1 face(s)` + `root/control 100 face(s)` + 同一句汇总行 ⇒ **事实逐行相同**，
而整答复不同（差在 `tree published from …` / `tree derived now …`、`freshness`、`scope` 与发布注）。

**门禁 M3 的另一半**（t4 读延迟 211s → <3s）目前只有"未构建 t4 = 55.7s、已构建 101 面 = 88.7ms"两个点；
要把五档都变成已构建的树需要每档都编译（t4 = 两万面，属 M5 的编译墙），因此这一半与 M5 一起量。

### W3-3 在线分析评审规则（完成，本轮）

清单的原文是"任何 PR 在 MCP 层新增在线分析必须回答『为什么构建期不能算』"——它是一条**评审**规则。
本仓的规矩是"新增全仓规则请加到 `conventions`，而不是加到散文文档里"，因此落成门禁
`conventions::online_analysis`：

- **触发条件**是"摸到源码的那几个调用"（`derived_faces(` / `derived_tree(` / `load_sources(` /
  `face_views_with_external(`），而不是"分析"这个看不见的词；清单住在一处，扩它只需改一行。
- **回答**写在模块文档里：`//! online: <为什么构建算不了>`，且散文至少 30 字符（那是**下限**而不是
  裁判——门禁判不了理由是否成立，它做的是让这个问题在唯一能回答它的地方无法回避，好让评审有一条可以反驳的
  句子而不是一片沉默。`W3-3` 要的正是这个）。
- **边界明说**：只判桥的生产那一半（文件里第一个 `#[cfg(test)]` 之前，且从不判 `*_tests.rs` 兄弟文件）
  ——推导的测试是**关于**推导的测试。
- **21 个文件各写了自己那句**：读源码文本的（`source_index`/`callgraph`/`impact`/`usages`/`why`/
  `search`/`digest`/`claims`/`diff`/`affected`/`locate`/`consistency`/`consistency_support`/`check`）
  说"构建发布的是面，不是调用图/不是源码文本，而这些答案是关于源码**此刻**的样子"；需要整份面的
  （`resolve`/`workspace`/`grafts`/`apply_refusals`/`new_project`/`tools`/`ownership`）说"构建之后
  新增的面根本不在记录里"。**不是 21 份同一句话**。

钉子：三种合成形状（写了理由 ⇒ 放过；什么都没写 ⇒ 报出并带 `file:line` 位置；敷衍一句 ⇒ 仍报）+ 测试那一半
不判 + 遍历覆盖桥（>40 文件）+ **对全检出断言 0 findings**。

**顺带记两处先红**：① 我把 `pub mod online_analysis;` 插在了 `#[path = "purity.rs"]` 与
`pub mod purity;` **之间** ⇒ purity 门禁的源码被当成 `online_analysis` 编译了一遍（`--list` 里出现
`online_analysis::purity_tests::*` 才暴露），修法是把属性与它的条目还在一起；② 我的 `//! online:` 行
紧跟在一条 markdown 列表项后面，`clippy::doc_lazy_continuation` 判定它是列表延续 ⇒ 在标记前补一行空
`//!`（更可读，也顺手解掉这一类）。

## 阶段 M4：反补丁感（进行中）

### W5-2 affected 补计划层（完成，本轮）

**改动**：`affected` 的每个改动文件行下面多一段 `plan:`——点名"该文件所声明的那个面"的 **graft 计划条目**
（按**选择器 + 它针对的逻辑路径**报告，不报它选中的实现：`affected` 回答"这次改动波及什么"，替换件是
`grafts` 的问题）。

**两处次序是承重的**：
1. **没有计划目录的成员一分钱不付**——先做 `.xirang/external-grafts` 的存在性判断，因此从未有过外部 graft
   的树既不多一次推导、也不多一行字（用它把"计划层"塞进 `affected` 而不让它变成全树遍历）。
2. **两套坐标要认全**——`FaceView.source` 相对 `src/`，而 `affected` 的改动路径带 `src/` 前缀；原样比较
   会让每个面都像"没有名字"，而那是**静默漏报**（对一个确实被计划点名的面会说"没有计划点名它"）。

**钉子**：有计划且点名 ⇒ 报出选择器与逻辑路径；有计划但没点名 ⇒ **明说**"`none of the N entry(ies) here
names this face`"（沉默会被读成"根本没有计划"）；**没有计划目录的树一个 `plan:` 字都不多**。

**如实记：这个夹具我写错了两次，两次都是布局/身份写错、看起来像工具坏了。**
① 面放在 `src/shared.rs` ⇒ 构建的遍历**一个面都找不到**（面必须住在 `<dir>/<name>.rs`）；② 计划的
`target` 我按叶名写成 `shared.rs`，而身份路径是**文件自己的相对路径** `shared/shared.rs`。两次都是
"夹具错了、被断言抓到"，不是工具错——这条留给以后写夹具的人。

### W5-4 consistency 升级修复器入口（完成，本轮）

**验收的读法**：清单要"一次调用给出可执行修复，执行后 outliers: 0"。因此这条不是"答复里多一句建议"，
而是**答复里直接给出可执行的请求**：多了一行 `majority   N of M sibling(s) carry this shape`（多数派就是
本比对认的基线），以及每个离群者下的 `fix` 行——**写入路径自己的 JSON 请求**（`{"action":"edit", …}`）。

**钉子把"可执行"当真**：测试不靠读一遍来判 `fix` 行，而是把工具自己打印的 JSON **取出来、交给写入路径
执行**，再问一次比对，断言 `outliers` 归零。一条点名错字段/错面/错值的修复行会让离群继续站着。

**三处只有做了才知道的事**：
1. **`edit` 按逻辑路径指认面**：键是 `node` 而不是 `face`。第一版给 `face`，写入路径当场拒绝并交回可接受
   形状（`{"action":"edit","apply":true,"fields":{…},"node":"root/control/button"}`）。修好之后那行还是
   "粘贴即用"的。
2. **`edit` 只重写生成的面**（`// generated-by=XiRang` 是硬边界，不是疏漏）：对手写的面交回 `edit` 请求＝
   交回一条**粘贴即失败**的行。因此分两种拼法——生成的给 JSON 请求，手写的给"在那个文件里加
   `parts: SliderParts`，或先 adopt"。两个方向都有钉子。
3. **`parts` 只比存在性、比值属于对象自己**：把标本的 `parts: ButtonParts` 抄进 slider 是一次**弄坏被修
   文件**的修复。判断走 `SHAPE_FIELDS` 的 `ShapeComparison::Presence`，取值用兄弟自己的 `<Kind>Parts`。
4. `GENERATED_MARKER` 从创作模块的私有常量**搬进 `kernel::lexicon`**（文本契约的家）：桥要问同一个问题，
   而第二个读取方只抄得到一个字面量——那是两份答案。搬的时候我把它插在了 `XIRANG_DIR` 的**文档与条目
   之间**（`missing_docs` 当场报出），与上一轮 `#[path]` 那次是同一族错误：**插入必须落在被插入条目的文档
   *之上***。

**顺带满足 600 行棘轮**：新代码把 `consistency.rs` 顶到 649 行，把三个修复辅助函数（`repair_value` /
`repair_request` / `manual_repair`）移进 `consistency_support.rs`（形状词汇本来就住那里）。

### W5-5 debug 入口路由（完成，本轮）

清单要"check 失败按**失败类型**路由（坐标族→`consistency --parent`；产物缺席→`why`）"，验收是"dbg 电池
步数方差收敛到 1-2"。因此 `next_step` 从"一条固定提示"变成**按种类路由**，三种：

| 种类 | 判据（**观察到的**，不是猜的） | 路由 |
| --- | --- | --- |
| 产物缺席 / 构建的红 | 日志里**没有** `test result:` 行（`results == 0`） | `why {at: "<编译器点名的 path:line>"}` + `check {verbose: true}` |
| 坐标族 | 失败测试名或回复自己的 `why` 行里出现**这棵树自己的**族名词（内核的 `RuntimeCheckSpec::CoordinatesInViewport.name()`，不分大小写，另收 "coordinate"/"viewport" 两种散文写法） | `consistency {parent: "<该面的父级>"}` |
| 默认（断言的红） | 其余 | 那条实测会被采纳的字面检索（T-22 起由本次答复自己实例化） |

**为此 `Observation` 多带两个事实**：`failed`（失败测试名清单）与 `first_location`（日志里第一个
`--> path:line`）——两者都在**读日志那一次**顺便取到，因此路由不额外读盘。

**钉子**（`a_red_routes_by_its_kind_rather_than_to_the_default_hint`）三种都走：构建的红路由到编译器点名的
位置、且**不**落到断言检索（那里根本没有断言可搜）；坐标族路由到该面的**父级**比对；默认那一种保留原提示。
**一处刻意的负断言**：名字里凑巧提到 offsets、但回复的 `why` 行不含族名词的失败测试**不得**被当成坐标的红
——路由的信号是这棵树自己的词汇，而不是测试名的联想。

### W5-3 deepen 预览附消费方 diff（完成，本轮）

**改动**：每次 `deepen` 的回复（**预览与落盘都带**）多两行消费方说法——
`slots      the layer adds N filling option(s) inside \`Kind\`: …`（这一层提供的槽位填充选项）与
`consumers  K of N graft plan entry(ies) target this face or its subtree (…) — this layer leaves every
target and path alone, so those plans keep pointing where they point and gain the options above`。
消费方＝**从面自己的文件之外读它的东西**：针对它/它子树的 graft 计划条目，以及填充它零件层槽位的那些。
计划目录不存在的树只印 `slots` 一行（"没有消费方"不需要一句噪声）。

**验收的实质**是"预览不是广告"，钉子就照这个写：`preview` 与 `applied` 两条回复的
`slots`/`consumers` 行**逐字相等**。

**它当场抓到一处真缺陷（这枚钉子的价值所在）**：预览在**副本**里跑，而副本按设计**跳过 `.xirang`**
（`preview::skipped_directory` 与 `target`/`.git` 同列）⇒ 我第一版在**工作目录**里读计划，于是预览说
"没有计划针对这个面"、落盘说"1 of 1 条针对它"——**正是"预览是广告"那个缺陷**。修法：两半各读各的树——
面取自工作树（被做深的那个面在那里），**计划取自项目**（入口在那里）。`deepen` 因此与 `promote` 一样
同时拿到工作目录与项目根，签名与注释都写明理由。

**顺带**：新代码把 `apply_tests.rs` 顶到 820 行代码行（超 800 的测试上限），把这条钉子拆进
`apply_consumers_tests.rs`（`package` 提为 `pub(crate)` 复用）。

### W5-6 apply 树驱动一次成型（本轮完成第 ③ 半：写后家族判定）

清单这条有三半：① 从树自行推导家族契约与模板；② 一次生成完整文件；③ 响应附 diff + 计划后果 +
**写后家族判定（`outliers: x of N`）**。本轮落的是 **③**（验收"零写后回查"的那一半）。

**改动**：`report()` 现在把**家族判定**搭在每条写入回复上（预览与落盘都有）——新增
`mcp/apply_family.rs`，它在**变更之后的树**上（预览＝副本）取"该面所在父级的孩子们"，按**形状信号**的
字段名集合跑 `consistency` 自己的多数派算术（`deviations`），给出：

```
family     4 sibling(s) under root/control; outliers: 1 of 4
  outlier     lever: declares `exports`, which no sibling declares; declares `handle_traits`, which
              no sibling declares — `consistency {parent: "root/control"}` prints the fix
```

**一条规则不是两条**：多数派算术用 `consistency::deviations`，形状词汇用 `consistency_support` 的
`declared_shape`/`one_face`/`Wording`（本轮把它们提到 `pub(crate)` 并重导出）——在这里抄第二份，就是
"outlier"在关于同一棵树的两份答案里开始有两个意思的来路。只有自己一家时不给判定（对空无一物的判定不是判定）。

**真机实测**（`/tmp/lever-drift` 上 `apply add`）：新面 `knob` 与三个兄弟一致，而前几轮实验留下的
`lever` 被点名为唯一离群者——判定跟着**父级**走，不是一条固定路径。

**钉子**：一致的面报 `outliers: 0 of 3`；多声明一个字段的面被点名 `outliers: 1 of 4` 并带上"哪个调用
交回修复"；判定跟着面落地的**父级**走（第二个根子面按根自己的孩子们判）。

**两处先红，都是我自己前几轮立下的门禁抓到的**（这正是它们的价值）：
- `conventions::online_analysis`（W3-3 立的）当场报 `apply_family.rs` "computes online without saying why"
  ——补上 `//! online: 判定说的是**写入刚留下的**那棵树：刚创建的面根本不在任何记录里，而兄弟可能自构建以来
  已变"；
- `conventions::size`（600 行棘轮）报 `apply.rs` 602→603 行 ⇒ 把打印搬进 `apply_family::verdict_lines`、
  把 `work` 绑定提到 `consequences` 之前。

**仍开**：① 从树推家族契约与模板（`add` 自动继承兄弟的形状，调用方不必逐字段拼）；② 一次生成完整文件；
验收里的 S4 ≤2 次仪器调用与 S6 = 预览→落盘 2 调用属两臂实机。

### W5-6 第①半：从树自行推导家族契约（完成，本轮）

**改动**：`apply add` 时调用方**没写**的家族字段，由**它正在加入的那一家**补上，并且回复**说出来**。
新增 `apply_family::complete_add_fields` / `inherited_fields`：取父级的孩子，按形状信号解析每个兄弟声明的
`exports` / `handle_traits` / `part_traits` 标签，取**多数派**（出现次数 > 半数）合成声明里那种
**逗号分隔**的拼写（渲染器按逗号切分并逐个加引号），填进请求并给出一行

```
inherited  exports = control.render from the family under root/control; pass it explicitly to override
```

**三条边界（都是有意为之）**：① **`parts` 从不继承**——它点名对象自己的零件类型，抄兄弟的值按构造就是错的
（与形状比对同一条规则）；② **显式给的值永远赢**，树是**补全**请求、从不推翻它；③ 父级的两种拼法都读
（顶层 `parent` 或 `fields` 里的），否则写了第二种的调用方会**静默地**完全得不到继承。

**同一处的两个先红**：`conventions::online_analysis`（W3-3）报新调用点缺 `//! online:`（补："判定的是写入
**刚留下**的那棵树"）；`conventions::size` 报 `apply.rs` 614 行 ⇒ 把 `W5-3`/`W5-6`/`W5-7` 三组注记整体搬进
新模块 `apply_notes.rs`（报告的职责是渲染执行器的事实；这三组是"关于调用方那个决定"的事实）。

**真机实测**（两个兄弟都声明了 `exports: ["control.render"]` 与 `handle_traits: ["ControlHandle"]`，再加一个
不写这两项的 `gamma`）：回复给出两行 `inherited …`，**写下的文件里带着它们**，而家族判定同时说
`outliers: 0 of 3`——W5-6 的两半合起来正好是"继承契约、然后确认家族干净"。

**钉子**：省略 ⇒ 继承**并报告**且文件里落成声明的拼写；显式 ⇒ 不覆盖、也不为该字段出继承行，而未写的另一个
字段仍然被继承。**W5-6 由此只剩验收里的步数（S4 ≤2 次仪器调用、S6 = 预览→落盘 2 调用）属两臂实机；"一次生成
完整文件"本就是这个动作的做法。**

## 阶段 M5：框架规模墙（W6-1 归因已启动）

### W6-1 编译墙归因（本轮交出报告；验收的 10k<5min 已实测达标）

**量法**（可复现，脚本是本地 `/tmp/wall.sh`，数据落 `scale-logs/compile-wall.json`）：用
`gen_scale_tree.py --host` 造一棵可构建的单 crate 宿主 ⇒ `cargo build` 预热 ⇒ `touch src/lib.rs` ⇒ 计时
`cargo build`（＝构建脚本 + rustc 编这个 crate）⇒ 再**直接把构建脚本二进制按 cargo 自己的
`CARGO_MANIFEST_DIR`/`OUT_DIR`/`CARGO_PKG_NAME` 跑一遍**并计时 ⇒ rustc 那一份 = 总 − 构建脚本；
`generated_lib.rs` 的字节数由构建产物读。

**实测（2026-10-04，debug 档，暖依赖缓存）**：

| 面数 | 总计 | 构建脚本 | rustc | 生成的计划 |
| --- | --- | --- | --- | --- |
| 103 | 1.56 s | 1.15 s（74%） | 0.41 s | 128 KB |
| 1,003 | 11.66 s | 9.69 s（83%） | 1.96 s | 1.21 MB |
| 4,644 | 59.78 s | 45.53 s（**76%**） | 14.25 s | 5.61 MB |
| 10,003 | 141.64 s | 103.89 s（**73%**） | 37.75 s | 12.05 MB |
| 1,003（**冷**，先 `rm -rf target`） | 91.12 s | — | — | 依赖编译的固定成本 ≈ 80 s |

**归因结论（与清单预设的方向相反）**：**墙不在宏展开/代码生成，在构建脚本自己**——发现 + 解析 + 渲染计划
+ 写 12 MB，10k 面时占 **73%**，且近乎线性；rustc 编这个单 crate 只占 27%，但**超线性**（0.41 → 1.96 →
14.25 → 37.75 s，末端 ≈ 每 4.6 倍面数涨 3.3 倍）。因此"对症"的杠杆是**构建脚本那一段**（发现/解析/写盘），
而不是让编译器少干活。

**验收**："10k 面单 crate 编译 <5min 的路径" ⇒ **实测 141.6 s（暖）**；把冷依赖那 ~80 s 加上约 222 s，
**两条都在 5 min 内** ✓。因此这一条**不需要**论证分包也不给分包模板——但有一个前提要写明：以上用的是本生成器
的**最小面**（`NoParts`、每面一个文件）。**清单里"4,641 面 15–18 min"用这棵树复现不出来**（4,644 面暖编译
59.8 s）——能把那个数字做出来的多半是更重的面形（每面带零件/契约体）或另一种调用方式（每个家族一次
`cargo test`）。**这是本轮如实记下的分歧，不是把它抹平**：按上面的归因，更重的面形会把成本压到 rustc 那一侧
（它是超线性的那一半），而本报告的杠杆结论只对"最小面"这一形状成立。

**仍开**：W6-2（中大型项目参考结构：workspace 分成员 + 家族分层规范 + 规模生成器验证树，验收 50k 文件
workspace 树 `registry`/`consistency`/`search` p95 <3s 且响应拍平）。

### W6-2 中大型项目参考结构（本轮：验收在 50k 面上实测，并修掉一处"响应不拍平"）

**造树**（生成器新增 `--workspace <成员数> <每个成员的面数>`）：`gen_scale_tree.py target/scale/w50
--workspace 20 2500` ⇒ **20 个成员 × 2,500 个面 = 50,080 个 `.rs` 文件**。成员是**普通包**（无 `build.rs`、
无 `[workspace]` 表——后者会让 cargo 看到多个工作区根并拒绝这棵树），因此发布记录**不需要编译器**：对每个成员
各跑一次 `verify`（只走管线）即可。

**实测（三跑，第三次的字节数；数据 `scale-logs/workspace-50k.json`）**：

| 工具 | 运行时间 | 响应字节 | 判定 |
| --- | --- | --- | --- |
| `registry`（默认普查） | 0.52 / 0.46 / 0.48 s | **1,961 B** | **达标且拍平**（五万面 1,961 B，而已构建的 101 面是 1,966 B） |
| `search --query child000001` | 23.5 / 24.4 / 24.0 s | 10,802 B | ✗ 约 24 s |
| `consistency --parent root/control --by shape` | 137.9 / 161.3 / 167.6 s | **2,808,408 B** | ✗ 约 2.5 min 且**完全没有拍平** |

**结论分两半**：
1. **`registry` 达标**——M3 的记录路径 + W2-1 的普查，在五万面上就是"平"的（这条验收最想看到的东西成立）。
2. **`search`/`consistency` 未达标，而瓶颈不是面的推导**：它们每次调用都要 `load_sources` 读一遍整棵树的
   源码（记录里没有函数级/调用级数据）——正是清单 W3-2 自己点名的那个余项。

**本轮顺带修掉一处真的"不拍平"**：`consistency` 的 `--parent` 模式**每个成员印一行**，五万面实测 **2,808,408
字节**；而本工具自己的承诺是"一次调用点名离群者"，默认印出每个成员与它自相矛盾。改为默认保留**家族行 + 离群者
+ 它们的原文片段**，其余扣下并附 `full: true`（与 W2-1 同一套 `truncation::withheld` 出路）⇒ 同一棵树同一
调用降到 **10,688 字节**（时间不变：成本在**读源码**，不在渲染）。钉子
`consistency_tests::the_member_rows_are_bought_not_given` 两个方向都钉（默认扣下且说出去哪儿买；`full: true`
全印）；既有那条 `the_api_signal_reads_each_siblings_own_calls…` 改成用 `full: true` 买下它要读的行。

**如实记一处正确性缺口**（不藏）：在这棵 50k 工作区上 `consistency` 答 `outliers: 0 of 0`——源码索引有
**每棵树的预算**、超了就跳过文件，于是兄弟集合是空的。它该单独立项，本轮只把它记下来。

### W6-2 续：`search`/`consistency` 那 24 s 与 183 s 花在哪儿（本轮量出归因；数据 `scale-logs/read-path-attribution.json`）

上一轮量到"`registry` 达标而 `search`/`consistency` 不达标"，本轮把**不达标那两个的时间归属**量清楚：

| 工具 | 103 文件 | 2,501 文件 | 50,020 文件 |
| --- | --- | --- | --- |
| `search --query child000001` | 0.104 s / 0.107 s（两棵树同规模） | — | **24.0 s** |
| `consistency --parent … --by shape` | 0.631 s | ~9 s（由 20 成员 183 s 折出） | **167.6 s** |

**归因**：两者都**与树里的文件数成线性**——`search` 0.104 s/103 文件 → 24 s/50,020 文件（约 480×，文件数 485×）；
`consistency` 0.63 s/103 → 9 s/2,501（约 14×，文件数 24×）。**原因不是面的推导，也不是响应渲染**（上一轮把成员行封顶
后字节数降了 266×、时间一点没动，已经把渲染这一侧排除），而是 `source_index::load_sources(root)` **每次调用都
遍历并解析根下每一个 `.rs`**，而这两个工具在看清"自己的答案要点名哪几个文件"之前就已经把它调用了。

**下一步（已定，下一轮动手）**：让成本跟着**问题的大小**走，而不是跟着树的大小走——
① `consistency` 的家族模式**只加载该家族的文件**（它从树上就知道兄弟是谁，不必先读整棵树）；
② `search` 改为**先按字面量预筛、只解析命中的文件**。
两条都不需要新记录格式，也不需要改渲染；它们直接打掉上表那条线性。

**但要如实说清它们够不到哪里**：本夹具的家族是**一个父级下 2,500 个兄弟**——那种家族里"点名/摘录每个兄弟"这件事
本身就是秒级，上面两条只省掉"读整棵树"那一份（本夹具约占 24 s），省不掉"读这个家族"。要让 `consistency` 在
**几千成员的家族**上也回到 3 s 以内，还需要第三件事：**把"每个面声明了哪些字段"放进记录**（记的是**声明级**事实，
不是现在记录里只有指纹的 `fields` 列），这样多数派与离群者的判定就能在记录上做，源码只用来给**离群者**取原文片段。
这三条（家族内定向读取 · search 预筛 · 声明级记录）是同一件事的三个梯度，按"先便宜后贵"推进。

### W6-2 三步走之第①步：`consistency` 的家族模式只读该家族（本轮，并**修掉一处真实缺陷**）

**改动**：`source_index::load_directories(root, dirs)`（只读列出的目录，不走整棵树）+ `consistency` 的
`--parent` 模式改为**在成员循环内**、按**该成员自己的根**读**这一家自己的目录**。原先那次整棵树读取是在
**工作区根**上做一次、再被所有成员复用。

**它同时修掉了一处真实缺陷（这才是本轮最重要的产出）**：用工作区根索引去匹配成员内 `src/…` 的相对路径，
永远匹配不上 ⇒ 在工作区模式下**兄弟集合是空的**，于是判定是 `outliers: 0 of 0`——**工作区上的答案是空的**。
现在同一棵树同一调用答 **`outliers: 0 of 2500`**（真的比了 2,500 个兄弟）。

**实测（50k 面工作区，同一调用）**：

| | 修前 | 修后 |
| --- | --- | --- |
| 时间 | 167.6 s | **151.5 s** |
| 字节 | 10,688 B | 10,775 B（仍拍平） |
| 判定 | `outliers: 0 of 0`（**空集合**） | `outliers: 0 of 2500` |

**并如实说清这一步够不到哪儿**：本夹具的家族是**一个父级下 2,500 个兄弟**，单独量一个成员（2,501 文件）仍要
**12.6 s**——省掉的是"读整棵树"那份，省不掉"读这个家族"。要让几千成员的家族也回到 3 s 以内，必须走第③步
（**声明级记录**：多数派与离群者在记录上判定，源码只给离群者取原文片段）。第②步（`search` 预筛）仍未动。

### W6-2 三步走之第②步：`search` 预筛（完成）

**改动**：`source_index::load_sources_matching(root, keep)` 把索引拆成两半——**读**（每个文件的文本，照旧全读）
与**词法**（函数、调用、分支事实，按 `keep(relative, text)` 决定）。`load_sources` 就是 `keep = |_, _| true`
的那一种，因此旧行为没变、也没有第二份实现。

`search` 两处各取所需：
- `--query`（匹配**名字**）：只在文本提到该名字的文件上词法——不在字节里的名字不可能是字节里的声明；**路径那一半
  仍然看到每个文件**（这是预筛绝不能碰的性质）；
- `--literal`（匹配**文本**）：每个文件的文本照读，**一个文件都不词法**——这一模式从不查看符号扫描，省下的正是它。

**实测（50k 面工作区，`search --query child000001`）**：**24.0 s → 14.6 s**，输出**字节完全相同**（10,802 B）；
单成员（2,501 文件）`--query` 0.587 s、`--literal` 0.422 s。

**安全性的钉子（这枚比"变快了"重要）**：`a_file_is_listed_by_path_even_when_its_text_never_mentions_the_query`
——夹具把查询词拼在一个文件的**路径**里、从不拼在**文本**里，于是预筛跳过它的词法，而**路径那一半仍必须列出它**。
一个悄悄缩小了文件清单的预筛能骗过"函数找到了"那类钉子，但骗不过这一条。

**仍开第③步**：**声明级记录**——本夹具一个父级下 2,500 个兄弟，家族自己的文件仍要读（单成员 12.6 s 量于
第①步那一轮），只有把"每个面声明了哪些字段"落盘、让多数派与离群者在记录上判定、源码只给离群者取原文片段，
几千成员的家族才可能回到 3 s 以内。它同时是 W3-2 余项的正解。

### W6-2 三步走之第③步（上半）：构建期发布**声明形状**记录（本轮）

**改动**：`write_shape_manifest` 落 `shape_manifest.tsv`——每个面、它按内核 `FACE_FIELD_ORDER` 拼出的每个字段
各一行（`source`、`field`、**声明拼出的原文**），一个什么都没声明的面写一行 `-`（要计数的读者必须看得见整个
家族）。读取方 `read_shape_manifest` → `ShapeRow`，与其它记录一样走 `face_view` 的重导出。

**关键设计决定（写进注释与钉子）**：这份记录携带的是**声明**而不是**结论**——按**源码路径**做键、原样保留
声明文本（例如 `["a" , "b"]` 而不是规范化成 `["a", "b"]`）。"它意味着什么"（哪些字段参与比较、怎么比）是桥的
`SHAPE_FIELDS` 词汇；在这里规范化就是那第二份会漂移的实现，也正是本仓最不许的事。**这一层分工也让内核保持纯净**：
记录里是构建期看见的字节，判据留在执行面。

**实测**（真 `verify`，一个 50 面的成员）：记录 104 行，表头 `# source	field	value`，例如
`control/control.rs  kind  Control`。

**钉子**：表头即契约；`exports`/`handle_traits`/`kind` 的**原文**都读得回来（并断言原文的原始间距，把"不规范化"
这条决定钉住）；非注册面文件根本不在记录里。

**这一步的下半（仍开）**：让家族比对**读**这条记录（并在读之前判新鲜度——陈旧的记录会给出错的多数派），
源码只用于给**离群者**取原文片段。那才是"几千成员的家族回到 3 s 内"的那一半。

### W6-2 三步走之第③步（下半）：家族形状读声明记录 + 源码惰性化（完成，**但如实说：这一形状上它不是那堵墙**）

**改动**：`consistency` 的 `--by shape` 在构建的**声明记录新鲜时**从 `shape_manifest.tsv` 读每个兄弟声明了哪些
字段（新鲜度走构建自己的 `build_output_is_current`，不另写一条规则）；兄弟源码改为**惰性**（`LazySources`：只有
`api` 信号或离群者取原文片段时才读）。本节明确说出用的是哪棵树：
`shapes from  member <m>: the build's declaration record (shape_manifest.tsv), current` ／
`the sources, derived now (no current declaration record)`。

**端到端验证**（真 `verify` 过的 50 面成员）：两条路都答 `outliers: 0 of 50`，而 `shapes from` 行不同 ✓。
钉子 `consistency_tests::the_declaration_record_answers_which_fields_a_face_declares`（记录按 `src/…` 做键、
"什么都没声明"映射到**空集合**而不是缺失条目、空记录回 `None` 让调用方回退）。

**但实测说这条在这一形状上不是那堵墙（如实记）**：同一个 **2,501 面成员**——记录路径 **11.07 s** vs 推导路径
**10.37 s**（惰性化之前是 12.4 vs 12.6）。而**同一个成员**上：`registry` **0.97 s**（它从剪枝记录取面）、
`search` **0.68 s**。⇒ **墙是 `member_faces` 的推导本身**（每个成员解析 2,500 个面文件），不是读源码、也不是渲染
（后者已在上一轮被"字节降 266× 而时间不动"排除）。

**顺带修掉一处二次复杂度**：`deviations`（多数派算术）过去对每个名字、每个兄弟都重扫一遍集合，是
O(sets² × names)；现在只数一遍（`name → 计数`）。它在这个形状上不是主项，但它在几千成员的家族上是数千万次
`contains`。

**下一步（已由上述对比定死）**：让**兄弟集合**也从记录取——即 `registry` 已经在用的那条 W3-2 规则（剪枝清单
新鲜时按它的行构面），推导只在记录答不了时付费。那才是让 `consistency` 在几千成员家族上回到 3 s 内的那一步。

### 追问"还有 schema 呢？"：广告面 vs 能力面（本轮，**当场抓到 3 处 + 1 条钉子**）

**方法（机械，不靠记忆）**：把 `tools.rs` 的 28 个目录条目逐块切开取"声明了哪些键"（**任意深度**的
`properties`，因为 `parts` 在 `inside` 之下、`cut`/`graft` 在某个 `anyOf` 分支里都算广告），再把
每个处理函数模块里**实际读的键**取出来（读的拼写只有两种：`arguments.get("X")` 与 `text/optional_text(arguments, "X")`），
两边对差。

**抓到的三处（都是"能力跑到广告前面"，也就是清单 W2-4 那一族）**：
1. `consistency.by` 的 `enum` 只写 `["api","kind","source"]`——**缺 `"shape"`** ✗，而 `shape` 甚至不是可选的，它是**默认**跑的两个信号之一；
2. `consistency` 的 properties 里**没有 `full`** ✗——而它正是那个被有界化的答案**唯一的出路**（读者根本发现不了怎么买回逐成员行）；
3. `mir` 的处理函数读 `against_trace`，schema 里没有它 ✗。

**修法**：三处补齐（`by` 的 enum 加 `shape`、补 `full` 与 `against_trace` 的描述），并把 `by` 的描述从
"`api` (default)" 改成事实（**省略即同时比 `api` 与 `shape`**）。

**钉子（有牙，已验）**：`tools_tests::every_key_a_handler_reads_is_advertised_and_every_entry_is_accounted_for`
——① 每个处理函数读的键都必须被 schema 广告（**任意深度**）；② `READ_KEYS` 表必须点名**每一个**目录条目，
新工具没人加进去就红；③ `by` 的 enum 不得比处理函数接受的信号更窄。**反向验证**：把 `full` 从 schema 删掉 ⇒
红并点名 `consistency reads \`full\` but its schema does not declare it`；把 `shape` 从 enum 去掉 ⇒ 红并点名
`by must offer \`shape\``；恢复 ⇒ 绿。

**如实记的两处过程事实**：① 我第一版 `READ_KEYS` 是**手写猜的**，跑出 8 条"缺失"里有一半是**表的错**
（`closure` 根本不是入参、`inspect`/`locate` 的键不在我以为的模块里）⇒ 改成**从代码机械导出**每模块真正读的键，
只剩 1 条待判（`conformance` 的 `files` 实为 `adopted` 的续期路径读的，已按事实归位）；② 全仓 `verb_table` 门禁
当场拦下我给它起的名 `collect_properties`（私有遍历动词必须是 `visit_`/`walk_`）⇒ 改名 `walk_properties`。
**仍开**：这条对差现在是"一手维护的表 + 机械比对"，下一步可把**工具→处理模块**的映射也从 `DISPATCH` 表机械导出，
让整件事零维护。

## 阶段 M6（终局收口）：按《xirang 终局优化方向》再落一轮

### §3 离线出生证明（本轮）

**缺陷（如实复现）**：把二进制拷到检出外跑 `new_project`，清单写的是注册局依赖
`xirang-toolchain = { version = "0.2.0" }`，而 0.2.0 **没发布** ⇒ 刚生成的宿主第一条命令就失败：
`cargo build --offline` → `error: no matching package named xirang-toolchain found`。
**第一印象场景在最该演示约束系统的地方演示不了**（复核记录 `target/hardbug-runs/offline-birth.txt`）。

**关键实测事实（它决定了这条检查该问哪儿）**：本机 `~/.cargo` 里**有** 0.2.0 的 `.crate` 存档、**没有**
索引条目（cargo 的 `.cache/ni/ch/xirang-toolchain`），而 `cargo build --offline` **依然失败**
⇒ **没有索引条目的存档解析不了**。因此探针必须查**索引缓存**（并要求其中出现 `"vers":"<版本>"`），
不能查 `.crate` —— 我第一版查的就是 `.crate`，实测被这条事实推翻（并把它钉进了钉子）。

**改动**：`build_method::scaffold::{registry_release_present, offline_source_warning}`（纯本地、确定性、
无网络、不会在气隙机器上挂住），由 `new_project` 在**预览与落盘两种回复**里附加一行 `offline …`：
点名版本、它查过的位置、以及三条出路（先联网构建一次 / 改用 `dependency: "git"` / 从检出里跑以获得
`path` 依赖）。`path`/`git` 来源自带字节，因此从不警告。

**实测（两个方向）**：真实 HOME（索引无该版本）⇒ 警告出现 ✓；伪造一份已预热的索引缓存 ⇒
`grep -c '^offline'` = **0** ✓；只放 `.crate` 不放索引 ⇒ 仍警告 ✓（钉子
`new_project_tests::a_release_the_cache_does_not_have_is_reported_rather_than_left_to_fail_later`）。

### §3 续（本轮）：把修复做成"本地确实能跑起来"

上一轮只做到**说出来**（警告），本轮做到**跑得起来**：`new_project` 新增 `path` 键——**点名一个 XiRang 检出**
（`kernel/` 与 `toolchain/` 相邻，校验过），生成的清单就写指向它的 `path` 依赖。这条拼写正是给**装出来的**
工具用的：它既不在检出内、又拿不到注册表缓存，以前只能生出一份 `cargo build --offline` 必败的清单。

**边界**：① `path` 单独给就是来源（本地拼写，读它到不了网络，拒绝只会让调用方把同一件事写两遍）；② 目录不像
检出 ⇒ **按名拒绝**并点名它找过的两个目录；③ `dependency: "registry"` 与 `path` 同时给 ⇒ **按名拒绝冲突**，
不静默偏向一个；④ 只有注册局来源才谈离线警告，`path`/`git` 自带字节、从不警告。

**出生证明实测（创建 → 构建 → 普查，全部离线）**：从检出外 `/tmp/nlmcp4` 生成 → `cargo build --offline`
**Finished**（21.37 s）→ `registry` 答 `namespace probe / faces 0`（library 模板未声明面，符合预期）。
`--list` 的字节预算被这次新增顶到 5001 ⇒ 削 `SHAPES_SHORT` 一句散文到 **4,976**（预算是预算，钉子逼着做取舍）。
复核记录 `target/hardbug-runs/offline-birth.txt`。

**仍开**：① 回复**无条件**说出"选了哪条来源"（现在只在有问题时说；无条件那句会动到回复形状的既有钉子，
留下一轮）；② 装出来的二进制若既不在检出内、又拿不到缓存，仍只能靠这条警告指路——真正的解除要么发布
0.2.0，要么调用方自己给 `git`/`path`。

### M6 §4 写路径边界：把不对称写成明示 + 给拒绝一条路（本轮）

**§4 第①条（不对称写进描述）已在位**：`apply` 的广告描述本来就写着两类动作——`add`/`deepen`/`cut`/`promote`
是**追加**类（改声明，手写面是合法主体），`edit`/`rename`/`delete` 是**改写**类（只改写自己生成过的文件），
拒绝文案也重复了这条界。钉子 `apply_tests::the_write_description_states_which_actions_reach_hand_written_faces`。

**§4 第②条（s7 写链的拒绝要给出路）本轮补上**：手写面被拒时，原来只划界（"这是作者身份，收编它是另一个
决定"），没有下一步。现在多一段 `way forward:`，给出**两条可走的路**：① 只要**声明**要改（加层、加条目、
换父级）⇒ 用追加类动作，这份文件就是合法主体；② 要给**文件**改名/改写 ⇒ 在编辑器里做，并点名"模块名就是
目录名与文件名，所以改名＝那一对名字 + 每一条点名旧路径的计划条目"，且给出**列出它们的命令**
（`affected {files: ["<该面的源码>"]}`）。钉子
`apply_refusals_tests::a_handwritten_face_refusal_names_the_routes_that_do_work`。同时保留了
"taking one over is a separate, explicit adoption" 那句（既有钉子要求它在拒绝里出现）。

**§4 第③条（D5：裁确认步）本轮落，但按证据裁在准确的位置**：先把那一问的归属查清——
`consistency --specimen` 的定义（GUIDANCE 自己的话）是"把兄弟与那批文件携带的已声明形状比对，点名谁缺哪条
声明"，也就是说它问的**就是同族形状那一问**；而 W5-6③ 之后，`apply` 的**写回执自己**就报
`family N sibling(s) under <parent>; outliers: x of N` ✓ ⇒ **"新面与同族一致吗"这一问不必再花一次调用**
（D5 的前提成立）。因此改动是：**add 形状里去掉那次 `consistency --specimen`**，把停止条件改成由回执回答
（`stop when that line names no outlier and the gates are green`），并在 GUIDANCE 里保留"台账那一问"
（`conformance {anchor}` + `consistency --specimen <anchor>`）——它问的是**与台账认证过的形状是否漂移**，
与写回执的同族多数派**不是同一个问题**，不许一起裁掉。
钉子 `client_tests::the_flow_table_packages_the_seven_shapes_with_their_stop_conditions` 当场因停止条件
措辞变化而红（行为变化打红旧断言 ⇒ 跟着行为改断言且保持同样强），改后绿。`--list` 仍 **4,976 ≤ 5,000**。

### M6 §5.2 T3：家族读走记录（本轮，**部分达成，验收未过**）

**改动**：① `build_method::face_views_from_pruning(rows)` —— 由剪枝记录构造 `FaceView`（W3-1b 发布
`parent_node`/`owns_registry`/`logical_path` 的目的就是它）；缺列的行**计数**，非零则调用方回退推导，
**绝不拿更小的家族去比**。② `consistency::member_faces` 记录优先：成员已发布且 `build_output_is_current`
⇒ 面取自记录，证据行说 `tree published from … (this comparison reads the build's own record)`。
③ **顺带修掉一处真缺陷**：`shape_names_from_record` 用记录的 `source` 拼写做键（**无** `src/` 前缀），而调用方
带 `src/` 去查 ⇒ **每次查询都落空**、静默退回读每个文件，而回复还声称"读的是记录"✗✗。新增
`source_index::indexed_path`（索引坐标只在一处定义，两侧共用）后，2,500 面独立包 **7.67 → 5.03 s**。
**这是同一族缺陷的第三次**（"两套坐标混用"已在 W6-2 第①步栽过一次）。

**实测（同一台机、同一棵树）**：`--by shape` 2,500 面独立包 **5.03 s**（验收 ≤1.5 s，**未过**）· 50 面独立包
**0.32 s**（走记录 ✓）· `--by kind` 同一棵树 **10.44 s** · 对照 `registry` **0.87 s**。

**已定位的下一处（O(n²)）**：`--by kind`/`--by source` 的 `values` 多数派分支对**每个兄弟**都做一次
`counts.values().max()` + 一次全表过滤 ⇒ 2,500² ≈ 6.25M 次比较 ⇒ 10.4 s，**反而比 shape 慢**。下一刀与
`deviations` 一样改成一遍计数、每兄弟 O(1) 查询。shape 那 5 s 还需再切一段量（记录读取本身应 << 1 s）。
复核记录 `target/hardbug-runs/t3-family-record.txt`（工作区数据，不进库）。

### M6 §5.2 T3 第二刀 + 第三刀前的归因（本轮）

**第二刀（O(n²) 修掉）**：`--by kind`/`--by source` 的 `values` 多数派分支对**每个兄弟**都做一次
`counts.values().max()` + 一次全量过滤 ⇒ 2,500² ≈ 6.25M 次比较。改成"一遍计数 + 预先算好 `top` 与
`tied_at_top`"，每兄弟 O(1)（语义等价：原条件＝该取值计数等于最高计数且不止一个取值拿到它）。
实测：`--by kind` **10.44 → 5.01 s** · `--by source` **4.69 s** · `--by shape` 5.07 s（本就走 `deviations`
的一遍计数，未受影响）。

**第三刀前的决定性归因**（同一棵 2,500 面独立包）：

| 探针 | 时间 |
| --- | --- |
| `consistency --parent root/nope --by kind`（**空家族**） | **4.53 s** |
| 同上、但把 `shape_manifest.tsv` 拿走 | 4.82 s（**不是它**） |
| `consistency --by kind`（**无记录 ⇒ 推导**） | 8.27 s |
| `registry --full --limit 200`（同树对照） | 0.83 s |
| `status` / `affected` | 1.25 s / 0.80 s |

⇒ **逐兄弟的循环几乎不花钱**：空家族就付掉了大部分。那 4.5 s 是**固定路径**的成本、随成员面数线性
（50 面的 `sr` 是 0.22 s ⇒ ≈1.7 ms/面），而且**不是** shape 清单的读取。`registry` 在同一棵树上 0.83 s，
证明这条固定路径可以低一个量级。**下一探针（已定）**：在固定路径三段各打一个计时点
（`build_evidence` 新鲜性 / 记录读取 + `face_views_from_pruning` / `tree_census` 与头部装配），量出 4.5 s
落在哪一段；在那之前不再猜。复核记录 `target/hardbug-runs/t3-family-record.txt`。

### M6 §5.2 T3 第三刀：`tree_census` 每答一次就整树推导（**成员级验收已过**）

**怎么找到的**：在固定路径上打计时点（临时插桩，量完即撤）⇒
`scope 83ms · build_evidence 198ms · read_pruning 15ms · build_views 7ms · member_faces 220ms ·
**tree_census 4.755s**` —— 就是它。`tree_census` 的实现是 `load_sources(root)`（读并解析整份源码）
**加上** `derived_faces(root, &namespace)`（**再推导一整棵树**），只为了印一行
`tree: N face(s), M file(s)`，而它印在**每一条** `consistency` 答复上（包括整个答案就是一个家族的那些）。

**修法**：**记录优先**——`published_census` 读剪枝清单得到"面数 + 承载它们的源码数"（与推导报告的是同一批
**事实**），记录新鲜时就是一次清单读取；工作区根没有自己的记录 ⇒ **把成员的加起来**（否则一个 20 成员的
工作区根为了印一行要付 50,000 个面的推导）。这一行还**说出自己是哪一种读法**
（`(read from the published record)`），因为要对比两棵树的读者不该去猜计数来自哪一侧。

**实测（2,500 面独立包）**：`--by kind` 5.01 → **0.51 s** · `--by shape` 5.07 → **0.79 s**（**验收 ≤1.5 s
过 ✓**）· `--by source` 4.69 → **0.51 s** · 空家族 4.53 → **0.54 s**。
**50k 面工作区**：151.5–174 → **50.0 s**（普查行证明走记录：`50020 face(s) … (read from the published record)`）。

**仍开**：工作区根那 50 s ＝ **20 × 每成员固定成本**（单成员 0.73 s，即每成员多约 1.8 s，未逐段量）。
下一探针同法：工作区根与一个成员分别打点，找出每成员多出来的那 ~1.8 s。
复核记录 `target/hardbug-runs/t3-family-record.txt`（工作区数据，不进库）。

### M6 §5.2 T1（第一刀）：`search --query` 的源码那一半走记录

**落地**：① 新增构建期记录 **`file_manifest.tsv`**（每份 `.rs` 一行：`source` + 它声明的函数名；一个函数都没
声明的文件写 `-`，因为按**路径**匹配的读者必须看得见每份文件）——写入方 `manifests::write_file_manifest`
（用内核自己的 `function_symbols`），读取方 `build_method::read_file_manifest -> Vec<FileRow>`。
② `search` 的**源码那一半**改由它作答（`record_source_lines`）：路径与函数名都取自清单，**零文件读取**；
记录答不了才回退原扫描，预筛规则不变。③ **工作区根**读**成员**的清单（根没有自己的记录），否则对着工作区根的
查询会退回扫描整棵树。
**实测**：分段计时把这一半从 **~4 s 打到 0.17 s**（2,500 文件的包）；t3 `--query child000001` 0.47 → 0.51 s。

**本轮回退的一刀（重要，如实记）**：我一度把"记录的空答案在记录新鲜时即为最终答案"做成规则（它让 50k 的
`--query` 从 **96.5 s 降到 12.8 s**、t3 的 4.5 s 降到 0.50 s）——**被既有钉子当场判红**：
`search_tests::a_published_face_is_ok_and_a_new_one_is_added_since_build` 在一份**新鲜**记录旁边写下一个新面并
期待 `[added since build]`，而那个面不在任何记录里 ⇒ **空答案不可能是最终答案**。根因是 **新鲜度不会注意到
"新增的文件"**。⇒ 次序改成：**先让 `build_output_is_current` 覆盖"被发现到的文件集合"**，再让记录的空答案成为
最终答案——那一刀约值 90 s（50k）/4 s（2,500 面），是 T1 真正的大头，但建立在前一刀之上。
复核记录 `target/hardbug-runs/t1-search-record.txt`（工作区数据，不进库）。

### M6 §5.2 T1（第二刀未落地）：空答案要成为最终答案，前提是新鲜度覆盖"新增文件"

**前提诊断（有证据）**：`discovery_fingerprint` 只哈希**被模块树发现到的**文件（`collect_source_files(nodes)`）
⇒ 一份 `src/` 下新增、**没有任何 `mod` 声明**的文件**不改指纹**，`build_output_is_current` 仍答 `current`
⇒ 这正是 `search` 的 `[added since build]` 钉子抓住的东西，也是"记录的空答案＝最终答案"不成立的原因。

**尝试**：把指纹补上"**原始文件集合**"（`visit_rust_paths(scan)`：只走 `src/**/*.rs` 的**路径**、不读内容
——编辑已由原循环覆盖，读内容会让每次新鲜性检查多读一遍整棵树），两个调用点同步传 `scan`。

**结果：那条钉子仍然红**，而且诊断打印留下一个**没解释清楚的现象**：
`stored=2acca1e0 recomputed=2acca1e0 raw=3`（**新增文件之后**，三个 `.rs` 都在，而 stored 与 recomputed 相等
——按测试顺序 stored 应在新增之前写下）。⇒ **改动全部回退**（`git checkout -- toolchain/`，回到 `e0ae9d5`），
空答案规则**保持关闭**。
**下一轮第一步**：查清上表里的 `stored` 是谁、何时写的（在 `check_for` 与 `TreeDelta::read` 两处打点，或比对
两次 search 前后指纹文件的 mtime/内容），再决定"原始文件集合进指纹"是否可行、以及空答案规则的次序。

### M6 §5.2 T1（第三轮追查）：仍未落地 —— **这一轮的错误在我自己身上**

三次插桩得出三个互相矛盾的结论，根因是我**把两次测量分开跑、再跨运行推理**：
- 运行 A（只打印指纹前 8 位）：新增文件后 `stored == recomputed`、`raw=3` ⇒ 判"两侧仍一致"；
- 运行 B（只打印原始路径清单）：新增后清单**确实**由 3 变 4（多出 `dial/dial.rs`）⇒ 判"覆盖生效"；
- 运行 C（两处都改好、规则打开）：那条钉子**仍然红**，回复依旧是"记录作答 + no matches"。
⇒ 三者不在同一尺度上，跨运行推理不成立。**教训**：一次实验必须把"输入、两侧的值、判定"**打印在同一行**。
**已回退**（`git checkout -- toolchain/`，回到 `e0ae9d5`），工作树干净、`search_tests` 18 项全绿。
**下一轮的一次性实验（写死）**：在 `build_output_is_current` 里把 `stored / recomputed / raw 清单 / scan 路径`
打在**同一行**，跑一次那条钉子，直接读出"谁写的指纹、两侧差在哪、raw 是否把新文件算进去"。

### M6 §5.2 T1（第四轮）：一次性实验读出了真正的前置缺陷

按上一轮写死的方式做**同一行打印**（`stored / recomputed / raw / scan`），一次跑完得出三件事：
① 全仓只有一个 `build_output_is_current`（`scope_view.rs` 里那个，`tree_delta.rs:106` 调的就是它）；
② 两行 ONELINE 都在新增 `dial` **之前**，也就是说第二、三次搜索**根本没再查新鲜度**——因为第一次搜索时
`record_face_lines` 就已返回空答案，而我的规则把它当成了最终答案，后面的代码路径根本没走到；
③ **判错的位置是第一次断言**（`search_tests.rs:89`，查询 `root/button` 期待 `[ok]`）。
⇒ 真正的事实：**`record_face_lines` 对 `root/button` 这个逻辑路径没有匹配到任何一行**，于是返回空。规则关闭时
这没关系（空答案让位给推导，推导按逻辑路径找到了那个面 ⇒ `[ok]` ✓）；规则一开，这份空答案就变成
`no matches` ⇒ 断言红 ✗✗。
⇒ 因此这一刀的前置缺陷不是"新鲜度不覆盖新增文件"，而是：**记录路径的匹配规则比推导窄**。今天不出错，只因为
空答案会让位。修好匹配，空答案才真的意味着"没有这个面"，"空答案即最终"才既对又省。
**已回退**（回到 `e0ae9d5` + 文档），`search_tests` 18 项全绿。
**下一轮写死的一次性实验**：在 `record_face_lines` 里打印**一行**：`query / 行数 / 前 3 行的
(logical_path, kind, registry_name, source) 拼写 / 是否命中`，看 `root/button` 为什么没被匹配上。

### M6 §5.2 T1：落地（匹配补全 · 陈旧记录逐面重认 · 指纹覆盖原始文件集合 · 空答案即最终）

**怎么定的位**：按上一轮写死的方式做**一行打印**：`parsed_kind=None / row_kind=Some("Button") / judged==row_id`
⇒ 我用来重认身份的读取器没读出 `kind`；再一行打印 ⇒ `one_face` 解析**成功**，但 `string("kind")=None` 而
`field("kind")=Some("RenamedButton")` ⇒ 改用 `field` ✓。**两次一行打印就定位**（对比前两轮的三次跨运行推理）。

**四件改动**：① **匹配补全**——`record_face_lines` 的匹配拼写加上 **`logical_path`**（`path` 是**声明**写的，
宏派生的面常是 `-`；`logical_path` 才是别的工具报告、调用方手里有的那个），这正是前四轮追到的"记录路径比推导窄"；
② **陈旧记录逐面重认**——记录陈旧时，被匹配的行用**它自己那份文件此刻的 `kind`** 重算身份（只解析那一个文件），
就地改过的 `kind` 因此报 `[re-identified (旧 -> 新)]`，回复照旧说 `build stale`；两条既有钉子（"陈旧记录照样答
名字"与"改过的 kind 要报重认"）由此同时满足；③ **指纹覆盖原始文件集合**——`discovery_fingerprint` 增加
`src/**/*.rs` 的**路径**列表（不读内容），于是新增一个没有 `mod` 声明的文件也会让记录陈旧；④ **空答案即最终**
（仅当 `known && current`）。
**实测**：t3（2,500 文件）`--query zzz-nothing` **4.54 → 0.75 s**、`child000001` 0.47 → 0.80 s；50k 工作区
`zzz-nothing` **96.5 → 17.2 s**、`child000001` 12.7 → 16.2 s。
**如实说明 50k 变慢的那一格**：`build_output_is_current` 现在每次多走一遍 `src/**`，而一次搜索**最多问它三次**
（delta / `record_source_lines` / `TreeDelta::read`）⇒ 20 成员 × 3 × ~0.25 s ≈ 15 s。**下一刀：一次调用内每个
成员只问一次新鲜度**（把判定往下传；**不新增缓存**——缓存正是这一带反复出错的形状）。

### M6 §5.2 T1 第二刀：新鲜度"一次调用只问一次"

**改动**：把判定往下传（`TreeDelta::read_with(root, current)`；`roots_with_freshness(root)` 一次算好，面半与
源码半共用）⇒ 每个成员从"问两次"变成"问一次"。**不新增缓存**——记住一个判定正是这一带反复出错的形状。
配套两处：① 成员清单为空（cargo 解析不了的根、没有清单的目录）时 `record_source_lines` 回退 `None`，让扫描
照旧回答源码那一半（既有钉子 `a_root_without_a_package_still_answers_the_source_half` 守着）；② 两个新读者按
600 行棘轮搬进 `consistency::support`（`roots_with_freshness`、`current_identity`），由 `consistency` 重导出。

| 调用 | 上一轮 | 本轮 |
| --- | --- | --- |
| t3（2,500 文件）`--query child000001` | 0.80 s | **0.34 s** |
| t3 `--query zzz-nothing` | 0.75 s | **0.32 s** |
| 50k 工作区 `--query child000001` | 16.2 s | **10.2 s** |
| 50k 工作区 `--query zzz-nothing` | 17.2 s | **11.0 s** |

**仍未达 T1 验收（50k p95 ≤1 s）**：50k 上剩下的 ~10 s 是 20 个成员各自的**记录读取 + 新鲜度**（每成员 ~0.5 s），
即"每成员固定成本"，与 `consistency` 那边同一形状 ⇒ 下一刀：**把每成员的新鲜度做成一次目录级遍历**
（现在每成员都要走一遍 `src/**` 列路径 + 读回记录过的文件内容），或按 K1 的方向把"未变即复用"做进去。

### M6 §5.2 T1 第三刀（方向 ①，按维护者裁决"走到底"）：先量天花板，结果把方向改了

**实测（bash，不是估算）**：50k 工作区 = **50,060 个 `.rs`**、合计 **19 MB**；全读一遍 **0.62 s**、读 + sha256（单核）
**0.68 s** ⇒ **一次调用做逐字节新鲜度的数据量下限是 ≤0.7 s**。而 `search --query` 现在是 8.5 s ⇒ **多出来的
7.8 s 是每成员一遍的固定开销，不是 I/O、不是哈希**。

**开销在哪（已定位）**：`build_output_is_current` 为了知道"哪些文件算被构建过"，调用
`source_walk::discover_root(scan)`——那是**整树解析**（2,503 文件/成员），20 个成员各来一遍 ⇒ 每成员 ~0.3 s，
与实测的"单成员 0.31 s"吻合。

**所以 ① 走到底是两件事**：
1. **一次调用只走一遍**：整棵工作区的 `.rs` 路径走一次、成员共用（去掉 20 次遍历与重复读）；
2. **新鲜度不再解析源码**：给 `file_manifest.tsv`（本会话已发布、列全 `src/**/*.rs`）**补一列 sha256**，
判据变成"**走出来的原始路径集合 == 清单集合** 且 **每个文件的 sha256 == 清单值**"⇒ 不解析、**仍然逐字节**；清单缺该列
（旧记录）时**回退到今天那条 digest 规则**（旧格式降级解析的纪律）。

**预期**：50k `search --query` **8.5 → ~1–2 s**，且**判据一个字不改**（仍逐字节、不碰时间戳）——这正是维护者裁决的
方向 ①，且比先前估的"1.5× 天花板"大得多：**原先的天花板估计错在把"解析开销"当成了"I/O 下限"**。

### M6 §5.2 T1 第四刀：K2 并行哈希落地（摘要逐字节守恒）+ 50k 的三段账

**诊断（一行打印、一次运行）**：`build_output_is_current` 每个成员**只被调用一次**（上一轮"每成员问一次"有效 ✓），
每成员 = **`discover_root` 整树解析 ≈40 ms + hash ≈125 ms**（2,500 文件 ⇒ 每文件 ~50 µs）⇒ 20 成员 ≈3.3 s。
**K2 做法**：只并行"每文件读字节 + 拼路径"这一半（`parallel_map`，**标准库线程**；本检出离线且发布 crate ⇒
**不加依赖**），**分块按排序后的文件顺序拼接** ⇒ 摘要与串行**逐字节相同**。**守恒证据**：改动后 `registry` 仍报
`tree published from …`（记录是**改动前**签的，仍判 current ✓）。钉子
`source_walk::parallel_tests::fingerprints_do_not_depend_on_the_worker_count`。
**实测**：t3 `--query` 0.29 → **0.27 s**；50k `child000001` 8.8 → **7.8 s**、`zzz-nothing` 8.5 → **7.9 s**。
**只快了 ~8%**（预估的"省 2.5 s"没兑现）⇒ 说明 hash 之外还有更大的一块。
**50k 三段账（同一次运行）**：`pre_freshness 0.4 µs` · `freshness 3.73 s` · `faces_half 4.37 s`（含 freshness ⇒
faces **0.64 s**）· `source_half 0.06 s` ⇒ 合计 4.4 s，而墙钟 **7.9 s** ⇒ **约 3.5 s 不在 `search()` 里**（在
`--call` 的分派层）。**目标 <1 s 仍未达；下一步写死**：① 在 `main`/dispatch 打一行查那 3.5 s；② `freshness` 里
`discover` 仍占 0.8 s（递归解析，可并行但要先确认顺序与共享状态）。

### M6 §5.2 T1 第五刀：同一裁决被付两次（40 次哈希 / 20 个成员）——拿到 4.8 s，但未能同时守住钉子，已回退

**证据（同一次运行）**：`--call search --root w50` 里 `build_output_is_current` 被调用 **40 次**（20 成员 × 2）；
`cargo metadata` 只 2 次 / 65 ms（**证伪**了"分派层在跑 cargo"的猜测）。三段账与墙钟来自同一次运行：
`freshness 3.55 s` + `up_to_source 4.28 s`（含 freshness ⇒ faces 0.73 s）+ `source_half 0.06 s` = **4.34 s**，
而 **WALL 7.90 s** ⇒ 差额就是第二次付费（3.5 s）。
**试过的修法**：把"是否新鲜"收敛成唯一付费入口 `freshness::verdict`，`search` 两半与 `TreeDelta::read` 都走它，
边界用 `begin_call()`（一次调用内共享）。**效果**：50k `--query` **7.9 → 4.81 s**、584 项绿 ✓ —— 但
① 边界同时放在 `run_tool` 与 `search` 时，`search` 的 `begin_call()` 抹掉了分派层刚付的 20 次 ⇒ 回到 8.5 s；
② 边界只放 `search` 时，跨调用复用把钉子 `a_published_face_is_ok_and_a_new_one_is_added_since_build` 打红
（第二次搜索复用了第一次的裁决，看不见新增的面）✗。
**下一刀（已设计好写法）**：**带廉价戳的记忆**——`VERIFIED` 条目加"树戳"（`src/**/*.rs` 的**文件数 + 最新 mtime**，
只走名字与时间戳、不读内容），`verdict()` 命中记忆时先验戳：戳未变 ⇒ 复用（**分派层付过的那一次直接给 `search` 用**，
于是 20 次而不是 40 次）；戳变了 ⇒ 重新付费（新增/改动都会改戳 ⇒ 钉子照旧绿）。**规则仍逐字节**，戳只用来决定
"要不要信这份进程内记忆"——这正是"元数据只做缓存守卫、绝不授权新鲜"的形状，影响圈限定在进程内。
**已回退**（回到 `c29ffa2`，7.9 s、584 项绿）。

---

## M7 crate 分区 · 图产物 · 异步信号（计划，2026-10-05）

**起因（维护者原话要点）**：① 想要"变形金刚"——在**入口 `.rs`** 里声明"哪棵树封装成哪个 crate"，本目录即可重组/打散，**且不破坏注册树**；② 写文件本身是**调度**问题（保存即开始建索引），索引构建可当**异步**，"4 s 甚至十几秒也没有体感"，但**必须做信号对齐**：下一次 `search` 要等新文件**全部注册好**，**不是靠重试**，而是**看到终端明确输出更新成功**；③ crate 工具要**自动判断大小并可视化给 Studio**、进 MCP 工具与 Studio 工作流、是**独立且路径明确、不常调整**的工作流、并能回答"是否满足打包给 crates.io 的条件"；④ 顺带**别忘了性能优化**（重复付费那笔）。

### §M7.1 能力核查：那句"软解析 → 100~500 个物理平衡块"到底做到了没有

> 原话："在编译阶段，xirang 通过你设计的'软解析'算法，直接在内存中吞掉这张由 50,000 个节点组成的巨大依赖图，将其剪枝、归并，吐出最适合编译器吃满多核、且最不容易发生缓存失效的 100~500 个物理平衡块喂给底层编译器。"

| 成分 | 判定 | 证据 |
| --- | --- | --- |
| "在编译阶段" | **成立** ✓ | `build.rs` → `build_method::run()`（`pipeline.rs`）在编译期跑 |
| "**软解析**算法" | **成立** ✓ | `source_walk::discover_root_reporting`：读目录、**静默跳过**不是面的普通模块、只对"是面却解析不了"的文件报错 ⇒ 自有容忍式读取器，**不经 rustc**（内核 `syntax`/`source` 同族） |
| "在内存中吞掉 **50,000 节点**的依赖图" | **半** ⚠ | 图的信息有：`Node` 模块树 + `pruning_manifest`（`parent`/`calls`/`logical_path`）+ 调用图 + `source_scope`；但**没有"图"作为一等产物**，也**没有 50k 节点的实测**——真跑过的是 **10,003 面单 crate = 141.64 s**；50k 是**文件数**夹具（20 成员 × 2,500） |
| "**剪枝**" | **成立（注册意义）** ✓ | `cut`/`graft` 计划 + `source_scope.tsv` 决定哪些面进构建范围 |
| "**归并**" | **不存在** ✗ | 全仓无 partition/balance/block 逻辑（唯二命中无关：括号配平、K2 的哈希分块） |
| "吐出 **100~500 个物理平衡块**" | **不存在** ✗✗ | 今天**一个包 = 一个编译单元**；构建脚本只产出**一份** `generated_lib.rs`（10k 面时 12 MB） |
| "喂给底层编译器" | **不存在** ✗ | 同上：一份计划喂一次 rustc 调用 |
| "吃满多核" | **不是我们的功劳** ✗ | 构建脚本**单线程**（实测占 10k 面构建 **73% ≈ 103 s**）；多核只来自 cargo 的**包级**并行 |
| "最不容易发生缓存失效" | **半**：一处做了、整体反着 | 做了：`discovery_cache::write_if_changed`（内容没变不重写 + temp+rename）✓；**反的**：`emit_rerun_paths` 把 `src/` 与**每个文件**都声明为 `rerun-if-changed` ⇒ **任何一次源码改动都重跑整个构建脚本**（＝最大化失效）✗ |

⇒ **结论：那句话是目标，不是现状**，其中"软解析/剪枝"有真实原料，"归并/平衡块/多编译单元/多核/缓存失效感知"四半要新建；**"100~500 块"没有任何实测依据**（唯一锚点：每面 ~14 ms、10k 面 141.64 s、103/1003/4644/10003 面 = 1.56/11.66/59.78/141.64 s）。文档中一律标 **目标（未实现）**，以免违反"自我描述必须与行为一致"。

### §M7.2 计划（四段；执行清单住在 todo 工具，此处是依据与判据）

**总预算锚点（都是实测，不是估算）**：单 crate 构建 `103/1003/4644/10003 面 = 1.56/11.66/59.78/141.64 s`（构建脚本占 **73%** ⇒ **~14 ms/面**）· 50k 工作区 = **50,060 文件 / 19 MB**，全读 **0.62 s**、读+sha256 **0.68 s** · 50k `search --query` 现状 **7.9 s**（`c29ffa2`）· 同一裁决曾付 **40 次**哈希（20 成员 × 2），去掉后实测到过 **4.81 s** · T1 验收线 **p95 ≤1 s**（清单原线 ≤3 s）· 生态 crate 文件数分位：**p50 11 · p90 82 · p99 310 · max 1726** · 真宿主面密度 **0.31 面/文件**（`examples/control-button` 13 文件 / 4 面）。

#### P0 性能收尾（先把"同一件事付两次"清掉）

| 项 | 目标 | 现状 | 具体改动 | 验收（可判定） |
| --- | --- | --- | --- | --- |
| **P0.1** | 去掉重复付费 | **完成**（见 §M7.8）：`freshness::verdict(root, out)` 是唯一付费入口；`Verified` 增 `stamp: (usize, u64)`＝`src/**/*.rs` 的**文件数 + 最新 mtime**（只读目录项）；`remembered()` 要求戳匹配；`begin_answer()` 让三个读取方**每成员共享一遍戳**（计数：40→20 次哈希、60→20 次戳）；`tree_delta::read`、`consistency_support::{roots_with_freshness, published_census}` 全改走它 | 50k `search --query`：**P0.1 单独 8.21→5.10 s / 7.82→5.28 s（同会话交错，约 1.6×）**；与 P0.2 合起来 **8.21→4.76 s（1.72×）** | `a_published_face_is_ok_and_a_new_one_is_added_since_build` **绿**、`--features mcp` 584 项全绿、两面 clippy + 全工作区测试 + `--check-table` 全绿；盲区写进文档：戳每答案只走一遍（同答案内的并发编辑不被发现，与窗口本来的暴露相同）、窗口内 `cp -p`（保留 mtime）不被发现，窗口 `REUSE_WINDOW_SECONDS` 且仅进程内 |
| **P0.2** | 并行 `discover_root` | **完成**（见 §M7.9）：每一层都用 `parallel_map_with_workers` 映射（宽层不在根层）、条目先读出来再映射（顺序＝目录顺序）、`MAPPING` 标记拒绝嵌套、`workers<=1` 或 <64 条目走串行（作为钉子的串行参照） | 同一棵树上**计划与记录逐字节相同**（t3 上改前/改后两份 `out/` `diff -r` 为空、八份产物 sha256 全等；w50 旧记录仍 `content-verified`、指纹不变） | 50k 省 0.24–0.52 s（5.095→4.859 / 5.279→4.759）；钉子 `source_walk::discovery_tests::the_discovered_tree_does_not_depend_on_the_worker_count`（200 面宽夹具，workers 1/2/3/8/64 树与发现顺序都一致） |
| **P0.3′** | 成员级并行（**本轮按维护者意见新增**） | 20 个成员的整树核验一个接一个 | `parallel_map_with_threshold(&due, worker_budget(), 2, …)`：条目是**成员**不是文件，两个就够付线程交接；预算＝**一半的核、最多 8 个、永远留一个核**，`XIRANG_JOBS` 可覆盖（原话要求：「每个电脑的核心数也不一样，而且也不能把人家核心全占满的」）；戳的走法**折进付费**（只在"有记忆裁决要校验"时才先走）；剪枝清单每成员**只读一次** | 同会话交错：**10.161 → 3.260 s（3.12×）**、**10.095 → 3.220 s（3.14×）**；**语义与答案形状一个字不改**（除修回 §M7.10 那处措辞回归） | 钉子：`parallel_tests::the_worker_budget_leaves_the_machine_alone_and_honours_an_explicit_request`；`freshness_tests::{a_verdict_this_answer_paid_for_is_not_spelled_as_a_reuse, an_explicit_verify_pays_without_walking_a_stamp_first}`；`--features mcp` 588 项全绿 |
| **P0.3**（**待裁**） | 按答案验鲜 | 现在按"整棵树"验 | `record_face_lines` 只对**它点到的行**读文件比 `source_hash`；答案加一行 `verified N named row(s)`；"整棵树新鲜吗"不再有任何一次调用回答（写进文档） | 50k `search --query` **≤1 s**；命中查询只读命中文件；钉子：答案必须自报验了几行。**§M7.10 的实测与建议：先动预算（不损语义），再考虑缩窄语义** |
| **P0.4** | 数据落地 | 数字散在会话里 | `scale-logs/t1-freshness.json`（三段账 + 两次付费 + 并行前后） | 文档表格与 git 对账；**估算与实测分列** |

#### P1 图成一等产物（"索引"的正确形状：产物 + 异步刷新，不是搜索引擎）

| 项 | 目标 | 现状 | 具体改动 | 验收 |
| --- | --- | --- | --- | --- |
| **P1.1** | 图有产物 | **完成**（`0417572`，见 §M7.12）：`graph.rs` 发布 **`graph_edges.tsv`**（头部 `# graph`/`# nodes`/`# edges`/`# digest` + `from<TAB>to<TAB>kind`）；五类边 `parent`/`in`/`calls-file`/`calls`/`graft`；**记录与图是同一次计算的两个视图**（写入方把行交回来，不重读清单） | 与记录一致（钉子 `graph::graph_tests::{the_graph_is_the_records_own_rows_seen_as_edges, a_name_two_files_declare_stays_a_name, the_header_counts_and_digest_vouch_for_the_body}`）；**原有八份产物逐字节不变** + 新增第九份（t3 实测 `diff` 为空、sha256 全等） |
| **P1.2** | 写路径异步刷新 | **完成**（`b1069f8`，见 §M7.13）：`apply` 落盘后 `index::start` 在**后台**驱动 CLI `check` 与桥 `verify` 的**同一个入口**（一个根一次只跑一次）；一次性客户端 `settle(60s)` 等它做完；外部编辑**不靠监听**，靠**廉价戳差分** | 端到端实测：`--call apply …` 回复尾 `index behind … a refresh is running (0s)`，下一次 `--call graph` 已是 `graph updated: generation 2, 6 file(s), 4 face(s), 723f2f71…`；外部 `>>` 改一个源码文件后**不触发刷新**也报 `index behind: generation 1 covers 92ef0bd3…` ✓ |
| **P1.3** | MCP `graph` 读工具 | **完成**（`b1069f8`，见 §M7.13）：`xirang.graph`＝普查（节点/边/种类/最忙节点）· `node`+`direction`(out/in/both)+`depth`(1–3) 邻域 · `cycles: true`（**只在依赖边 `calls-file`/`graft` 上**跑的迭代 Kosaraju，答案明说结构边与未解析 `calls` 不算依赖）；节点按**记录自己的词汇**解析；输出有界（默认 20、上限 200，超出写 `N more withheld`） | 承诺面与能力面对上（`tools_tests` 的目录＝分派钉子 + `READ_KEYS` 点名它）；每份答案第一行就是索引行；**不做倒排索引**（§7 红线）✓ |

#### P2 异步的信号对齐（维护者点名的机制）

| 项 | 目标 | 具体改动 | 验收 |
| --- | --- | --- | --- |
| **P2.1** | generation + 就绪 | **完成**（`b1069f8`，见 §M7.13）：`graph.generation` 由管线在干净运行里**最后**原子写（`write_if_changed` 本就是 temp+rename），字段 `generation`/`digest`/`faces`/`files`/`graph_nodes`/`graph_edges`/`stamp`/`finished_at`；**它在＝这次运行完成，它不在＝磁盘上是上一次（或没有）** | 钉子：`index_tests::{a_record_that_does_not_match_its_graph_is_refused, a_finished_run_publishes_a_record_that_says_ready, a_run_that_published_nothing_reads_as_absent}`（切掉一条边 ⇒ `Damaged` 并点名计数/摘要不符） |
| **P2.2** | 终端明确成功行 | **完成**（`b1069f8`）：`index::line` **只有一种拼法**，CLI `check` 的**第二行**（第一行历史契约逐字节不变）/ 写入回复 / 图工具共用 ⇒ 同一个目录不会被说出两种说法 | 钉子 `cli::lib_tests::check_without_json_keeps_the_human_line`（第一行不变 + 第二行是索引行）；端到端 `xirang check /tmp/cb` ⇒ `graph updated: generation 1, 5 file(s), 3 face(s), 92ef0bd3…` |
| **P2.3** | 落后即报，不重试 | **完成**（`b1069f8`）：`graph` 与写入回复比 generation 的 `stamp` 与当前 `source_stamp`，不一致即首行 `index behind: generation N covers <digest>; the sources have changed since — run \`xirang check\``（或 `a refresh is running (Ns)`）；**绝不重试、绝不猜**，边照旧读出来但在首行说清楚 | 钉子 `index_tests::an_edit_makes_the_index_behind_and_the_line_says_so`、`graph_tests::a_behind_index_is_answered_from_and_named`；端到端见 P1.2 行。**与计划的偏差已写进 §M7.13**：`search`/`registry`/`consistency` **不**加这句——它们的记录新鲜度是**逐字节**的（比戳差分更强）且已给出同一出路 |
| **边界（必须写进文档）** | 新鲜度规则**不变** | generation 只服务**图这种加速器**；记录的**逐字节裁决**仍走 `freshness::verdict`，落后时按既有规则"回退推导"——两件事分开说 | 两条规则都各有钉子 |

#### P3 crate 分区（"变形金刚"）

| 项 | 目标 | 具体改动 | 验收 |
| --- | --- | --- | --- |
| **P3.1** | 声明 + 锁 | 入口（宿主）手写：`partitions! { namespace = "…"; host = [a, c]; A = [A]; B = [B]; c1 = [c1]; c2 = [c2]; }`（点名的是一棵子树的逻辑路径）；锁 `.xirang/partitions.lock`：声明哈希 + 每分区（名字/面集合哈希/成员数）+ **形状指纹** | 形状历史 = 锁的 git 历史（`git log -- .xirang/partitions.lock`） |
| **P3.2** | 生成器 | 每分区产 `crates/<名>/Cargo.toml`（包名·版本·`[lib] path`·**由 parent 树 + 调用 SCC 导出的 path 依赖**·`[package.metadata.xirang] shape`）+ 挂载根 `src/__partitions/<名>.rs`（`#[path="…"] pub mod …;` + `pub use host::XIRANG_NAMESPACE;`）+ `.cargo/config.toml` 的 remap；**三类拒绝**：分区重叠 · 陈旧挂载（面没被任何分区挂载 / 挂载点不存在）· 成环（SCC 报"这几处必须同 crate"） | 三类拒绝各有一枚钉子（红并点名）；生成物**不进 git** |
| **P3.3**（**待裁**） | 身份不变 | 宿主定义 `pub const XIRANG_NAMESPACE: &str = env!("CARGO_PKG_NAME");`；宏烤 `crate::XIRANG_NAMESPACE` 而不是 `env!`；门禁：**面文件不得读 `CARGO_PKG_NAME`**；生成器把该常量 re-export 进每个挂载根 | **NodeId 集合分区前后逐字节相同**（总闸门钉子） |
| **P3.4** | `re` 命令 | `xirang partition --check`（声明↔锁↔树自洽）/ `--write`（物化）/ `--revert`（打散）/ `--at <commit>`（换回某个形状） | 删生成物后 `git status` 无差异；`--at` 后形状指纹与那个提交的锁一致 |
| **P3.5** | 发布物化 | `--release` 物化真实包 + **身份/记录/租约迁移**（显式列出哪些记录失效） + `[package.metadata.xirang] shape`；**CI 两个形状都构建都测**；`cargo publish --workspace`（cargo 自己算顺序） | crates.io 约束：自包含（包外文件被拒，已实测）· ≤10 MB · 包名唯一 · `license-file` 在包内 |
| **P3.6** | 可视化与会判断 | 阈值取**实测**（>1,000 面 / >310 文件 ⇒ 建议切）；Studio 视图给：建议切口 + 代价（新增 N 项 `pub`、M 处身份迁移）+ **编译时长回填**（分 2/4/8 各量一次） | "100~500 块"这个数**由测量得出**，不照抄；无测量不下结论 |

#### P4 MCP / Studio 接入

| 项 | 目标 | 具体改动 | 验收 |
| --- | --- | --- | --- |
| **P4** | 工具化 | MCP：`partitions`（读：当前形状/锁/建议/代价）、`partition`（写：**默认预览**，`apply: true` 才落盘）；Studio：形状视图 + 一键重组 + 时长回填；**是独立工作流，路径明确、不常调整** | `READ_KEYS` 与 schema 对齐（既有钉子）；预览与落盘**说同一件事** |

#### 顺序与依赖

**P0 → P1 → P2 → P3 → P4**。理由：P0 是"同一件事付两次"的止血（立刻见效、且不改语义）；P1/P2 是**图 + 异步 + 信号**，它们是 P3.2 分区器的输入（没有图就没有"按依赖切"）；P3.3 是 P3.2 的前置（不改命名空间就会改身份）；P3.5/P4 依赖 P3.1–P3.4。**P0.3 与 P3.3 两处等维护者裁**，其余可并行推进。

#### 风险登记（每条都有缓解）

| 风险 | 症状 | 缓解 |
| --- | --- | --- |
| 身份变化 | 分区后 `NodeId` 变 ⇒ 记录/租约失效 | P3.3（根常量）+ **逐字节相同**的钉子（总闸门） |
| 陈旧挂载 | 新面没被任何分区挂载 ⇒ 静默不编译 | 构建脚本集合对差 `compile_error!`；`partition --check` 进 CI |
| 并发写 | MCP/CLI 写生成物时有人在构建 | 生成物一律 temp + rename；构建期只读 |
| 两个形状 | 测的不是发的 | CI 两个形状都构建都测（P3.5） |
| 缓存失效 | 改一个面重跑整个构建脚本 | K5 增量（按面/子树指纹重渲染），把这句从"目标"变"事实" |
| 假绿 | generation 落后却给出旧答案 | P2.3 落后即报 + 钉子；`cargo package --list` 那类"看起来成功"的步骤一律不当作绿灯 |


### §M7.3 异步与信号对齐（维护者点名的机制）

1. **generation 文件**（`graph.generation`：单调计数 + 内容哈希 + 时间戳）+ **就绪标记**：写一半 ⇒ 无标记 ⇒ **被拒**；
2. 写路径结束打印**明确成功行**（终端可见）——这就是"看到更新成功再查"；
3. 读工具启动**校验 generation 与当前源码**：落后 ⇒ **明确回答**并给出该跑的命令，**不重试、不猜**（本仓既有纪律：宁可明确拒绝，不产出"看似合理却错误"的答案）；
4. 钉子：人为截断图文件 ⇒ 红；generation 落后 ⇒ 报而不静默。

### §M7.4 维护面 ↔ 发布面（crates.io 的硬约束，已实测）

- **crates.io 只接受自包含包**：包外文件既**不进 tarball**，也**过不了隔离校验** —— 本机实测原话：`warning: ignoring library … as … is not included in the published package` → `error: failed to prepare local package for uploading` → `no targets specified in the manifest`（⚠️ 而 `cargo package --list` 那一步**不报错**，只看它会以为能发）；
- **发布面 ≠ 维护面是常态**，cargo 内建三件转换：**扁平化**（tarball 根＝包目录内容）、**清单重写**（原始清单留在 `Cargo.toml.orig`）、**git 溯源**（`.cargo_vcs_info.json`：`{"git":{"sha1":…},"path_in_vcs":"tokio"}` ← 实测 `tokio-1.53.1`）；
- 我们补它缺的那半：**形状指纹**写进 `[package.metadata.xirang] shape`（cargo 会原样保留在 tarball 的清单里）⇒ 审计链 = `commit → 声明 → 形状指纹 → tarball`；
- **权威进 git（声明 + 锁），派生的 gitignore（壳 + 挂载根 + remap）**；CI 五步：`partition --check` → `--write` → 分区形状构建+测试 → 撤销后（发布形状）构建+测试 → `package-audit`。

### §M7.5 两处待维护者裁

1. **P0.3**：新鲜度语义从"整棵树"改为"**这份答案所依赖的那几行**"（50k 才能 ≤1 s）——代价是有些调用不再回答"整棵树新鲜吗"，答案里必须写明验了哪几行；
2. **P3.3**：命名空间从 `env!("CARGO_PKG_NAME")` 改成**根常量**（分区后包名会变）——这是唯一的内核级身份规则改动；不改它，幽灵 crate 会**改身份**（`studio/app/namespace.rs` 记着 Studio 曾在"另一个身份域"重建注册树出事）。

### §M7.6 明确不做（重申 §7）

不在 MCP 层做缓存/索引 · 不做倒排索引/常驻进程 · 不改 2+1 广告位 · 不为数字删校验 · 编排合并不许静默省掉预览-确认门 · 不用"两臂同判"当"不具鉴别力"。

### §M7.7 P0.1 现状（本轮）

**代码在树里**（未提交）：`freshness::verdict(root, out)` 成为**唯一付费入口**，`Verified` 增加 `stamp: (usize, u64)`（文件数 + 最新 mtime，只读目录项），`remembered()` 要求戳仍匹配；`tree_delta::read`、`consistency_support::{roots_with_freshness, published_census}` 全部改走它；`search::record_source_lines` 本来就从 `roots_with_freshness` **收判定**、不自己付费 ✓。**测试 584 项全绿**（含"改动看得见"那条钉子）⇒ 上一轮"共享裁决打红钉子"的问题由**戳**解决。**实测数待补**（下一段）。

### §M7.8 P0.1 落地与实测（2026-10-05，本轮补完）

**一句话**：同一份裁决**每个成员一次**（重复付费清掉），而守卫它的**树戳也每个成员只走一遍**（三个读取方共享一次遍历）；`a_published_face_is_ok_and_a_new_one_is_added_since_build` 仍绿。

**改法（在 §M7.7 那段之上加的一层）**：`begin_answer()` —— 一份答案开始时**只清空戳记忆**（`STAMPS`），**不清核验本身**（复用窗口照旧）。它在两处被调用：`set_policy()`（每次派发调用都经过它，`run_tool` 与 `tool_call` 各一处）与 `roots_with_freshness()`（进程内直接调用的答案从这里开始——测试里的 `search`、一个读取方调另一个，只有这一个边界）。**为什么要多这一层**：戳是**给记忆用的守卫**，而一份答案要问它三遍（`roots_with_freshness` 付费、普查印等级、`TreeDelta::read` 取裁决）⇒ 实测 8.5 s 里有 **2.0 s** 是在为同一个问题走三遍树。共享之后**每成员 1 次哈希 + 1 次戳**。

**机制计数（同一棵树、一次运行，桩打在两处）**：

| 版本 | `build_output_is_current` 调用 | `tree_stamp` 遍历 | 墙钟 |
| --- | --- | --- | --- |
| 第五刀（基线，未改） | **40**（20 成员 × 2 读者） | —（当时还没有戳） | 11.28 s |
| 第六刀，但三个读取方各走一遍戳 | 20 ✓ | **60** | 8.34 s |
| 第六刀 + 每答案共享一次戳（本轮） | 20 ✓ | **20** ✓ | **7.53 s** |

**实测（同一会话内两臂交错跑，三次取中位；`scale-logs/t1-freshness.json` 是原始记录）**：

| 树 / 查询 | 基线（`c29ffa2`） | 本轮（第六刀） | 比值 |
| --- | --- | --- | --- |
| w50 `--query child000001` | 11.276 s | **7.528 s** | **1.50×** |
| w50 `--query zzz-nothing` | 11.109 s | **7.581 s** | **1.47×** |
| t3 `--query child000001` | 0.325–0.548 s | 0.344–0.555 s | 不可分（单成员，这一刀省下的约 0.2 s 在噪声之下） |

**必须一起读的两条**：① **绝对秒数不跨会话可比** —— 同一个**基线**二进制在上一次会话里是 **7.90 s**、在本会话里是 **11.28 s**（本机此刻约慢 1.4×），因此本轮只报**同一会话内交错测出的比值**（1.50× / 1.47×），并按这个比例把上轮的数字换算着读（≈5.3 s）✓；② **T1 的 p95 ≤1 s 仍未达**，这一刀只是把"同一件事付两次"（3.5 s）清掉，剩下的 ~4.5 s（20 次内容哈希本身）与 ~2.3 s（哈希之外）都是**下一步**的对象。

**仍未闭合（不许说成做完了）**：P0.2 并行 `discover_root`（上表里"哈希之外"的一块）· P0.3【待裁】按答案验鲜（≤1 s 的唯一路径）· P0.4 数据落地（本节与 `scale-logs/t1-freshness.json` 已落 P0.1 的一半，P0.2 后补全）· P1–P4 全部未动。

### §M7.9 P0.2 落地：每一层各自并行、顺序不动（2026-10-05）

**改法**：发现遍历的**每一层**都用 `parallel_map_with_workers` 映射（根层与 `discover_children` 各一次），而不是只并行根层——真实树的宽层很少是顶层：50,000 文件夹具把它的 2,500 个面放在 `src/control/object/` 下，只并行根层等于没并行。三处配套：① **条目先读出来再映射**，因此合并顺序就是目录交出条目的顺序（`unplaced` 的顺序也被保住）；② **拒绝嵌套**——线程本地的 `MAPPING` 标记让"工作线程里的另一个宽层"在自己线程上做完，线程数由机器决定而不是由树的深度决定；③ `workers <= 1` 或条目数 <64 时走串行，那条路就是钉子的**串行参照**。

**验收（逐字节，实测不是声称）**：
- **发布物逐字节相同**：把 `target/scale/t3`（2,504 文件）复制一份，先由**改前**的二进制 `verify` 发布、存下 `out/`，清掉 `target/` 再由**改后**的二进制发布同一个路径 ⇒ `diff -r` 为空、八份产物 sha256 全等（`discovery.fingerprint`、`file_manifest.tsv`、`function_manifest.tsv`、`generated_lib.rs`、`graft_plan.tsv`、`pruning_manifest.tsv`、`shape_manifest.tsv`、`source_scope.tsv`）✓
- **旧记录不失效**：`target/scale/w50` 的 20 个成员是在这次改动**之前**发布的；改前/改后两个二进制都印 `freshness: content-verified` 且指纹同为 `ca0d05b9e102b8aa0300a7d40ee703d2` ✓（若顺序变了，指纹会变、全部记录立刻陈旧——这正是要防的）
- **钉子**：`build_method::source_walk::discovery_tests::the_discovered_tree_does_not_depend_on_the_worker_count`——200 个面的宽夹具（宽层不在根层），workers 取 **1/2/3/8/64**，树与**发现的顺序**都必须一致 ✓

**实测（同一会话内与 P0.1 交错跑、三次取中位）**：`cut6`（只有 P0.1）5.095 s → `p02`（P0.1+P0.2）**4.859 s**（`child000001`）；5.279 → **4.759 s**（`zzz-nothing`）⇒ 这一刀值 0.24–0.52 s（1.05–1.11×）。**与基线合起来**（同会话交错）：**8.205 → 4.762 s（1.72×）**、**7.823 → 4.689 s（1.67×）** ✓ —— 这达到 P0.1 表里那句"50k `search --query` ~4.8 s"。

**门禁**：`cargo fmt --check` ✓ · `cargo test --workspace --offline` ✓ · `--features mcp` **585 项全绿** ✓ · 两面 `clippy -D warnings` ✓ · `xirang-conventions` 158 项（含 600 行棘轮与 `test_shape`）✓ · `tools/xirang-publish --check-table` ✓。

**仍未闭合**：**T1 的 p95 ≤1 s 仍未达**（本机 4.76 s）；剩下的三块是 20 次内容哈希（安静时每次约 0.16 s）、20 次戳遍历、以及每成员的清单读取 ⇒ 下一步是 **P0.3**（待裁）。

### §M7.10 P0.3′：成员级并行 + 有界预算 + 修回一处措辞回归（2026-10-05）

**维护者的两条要求（原话）**：「自动并行因为每个电脑的核心数也不一样，而且也不能把人家核心全占满的」；以及他对我给的例子的纠正：「我本来的想法是在顶层的某个 rs 文件指定某树为某 crate，然后 xirang 自动划分，然后也可以在 studio 中操作……我想要的代价是在最顶层声明一次拆分 crate 行为，然后就提交时生成，然后日常开发的时候是无感的，除了不同的分 crate 会导致编译速度不同外」。**前一条本轮落地成规则；后一条改变了 P3 的形态**（见 §M7.11）。

**一、有界且随机器变化的预算（新增，且是唯一一处并行规则的实现）**：`worker_budget()` ＝ **一半的核、最多 8 个、至少留一个核**，`XIRANG_JOBS`（已进内核 `lexicon`，与 `XIRANG_ENTRY` 同族）可显式覆盖。四条性质：① 机器核数不同 ⇒ 规则算出来的线程数不同，但**结果绝不不同**（两张保持顺序的钉子守着）；② 绝不用满整台机器（`budget < cores`，核数为 1 时取 1）；③ 条目数封顶（三个成员不会开八个线程）；④ 在**外层映射的工作线程之内**串行运行 ⇒ 线程数由机器决定、不由树的深度决定。钉子 `parallel_tests::the_worker_budget_leaves_the_machine_alone_and_honours_an_explicit_request`（把核数当**参数**逐个断言 1/2/3/4/8/16/20/64/128，因此不写进程环境、也不需要"本机恰好是几核"）。

**二、成员级并行**：读路径核验工作区的成员那一处改用 `parallel_map_with_threshold(&due, worker_budget(), 2, …)`——**"小"是条目的性质**：一个文件是几 KB，而一个成员是一次整树内容哈希，**两个成员就够付线程交接的钱**（文件级阈值仍是 64）。依据是量出来的，不是假定的：**同样八个成员，一个接一个答 4.392 s，`-P8` 下 0.864 s（5.1×）**。三处配套：① **戳的走法折进付费**——只有在"有记忆裁决需要校验"时才先走戳，没有东西要校验时那次走法发生在付费里面，于是它也能并行；② **剪枝清单每成员只读一次**（`TreeDelta::from_rows`）——记录那一半与差异读的是同一份清单，过去解析两遍五万行；③ 记录回来的戳存进本份答案的戳记忆，随后 `TreeDelta`/普查行不再走第二遍。

**三、修回一处措辞回归（P0.1 带出来的，维护者问"会不会影响输入质量"时查到的）**：付费在几个读者之间共享之后，50,000 文件工作区的每一行普查从 `freshness: content-verified at 23:34:22` 静默变成了 `freshness: reused (content-verified at …, 5s ago; window 30s)`——**同一件工作，两种不同的声称**，而且是更差的那种。现在规则是：在**本份答案**里付过费 ⇒ `content-verified`；在更早的答案里付费且仍在窗口内 ⇒ `reused`（点名时刻与窗口）。同一批还补了两条钉子（`a_verdict_this_answer_paid_for_is_not_spelled_as_a_reuse`、`an_explicit_verify_pays_without_walking_a_stamp_first`）。

**四、阶段账（同一次运行，桩打在三处；同一棵树同一个查询，三行是同一件工作的先后状态）**：

| 状态 | consult（调用线程：戳＋记忆查找） | parallel_pay（20 次哈希，线程池上） | 树那一半（每成员记录） | 墙钟 |
| --- | --- | --- | --- | --- |
| 成员并行，另两处未改 | 0.780 s | 1.561 s | 1.502 s | 4.505 s |
| ＋剪枝清单只读一次 | 0.673 s | 1.391 s | 1.056 s | 3.708 s |
| ＋戳折进付费 | **0.009 s** | 1.671 s | 0.922 s | **3.159 s** |

**五、总账（同一会话内与基线交错、三次取中位）**：`w50 --query child000001` **10.161 → 3.260 s（3.12×）**、`--query zzz-nothing` **10.095 → 3.220 s（3.14×）**；t3 0.416 → 0.351 s。**本会话的基线偏慢**（同一天量到的基线：上一会话 7.90 s、会话 C 8.20 s、本会话 10.16 s），所以按会话 C 的比例折算，这一批在安静时段约是 **8.2 → 2.6 s**。**语义一个字没改**：仍然整棵树、仍逐字节；改的只是"谁来付这笔钱、在几颗核上付"。

**六、T1 的 p95 ≤1 s 仍未达，以及到那里有两条路（请裁）**：本机 3.16–3.26 s 的构成是——20 次整树哈希（线程池上 1.4–1.7 s，**这正是预算封顶的地方**：20 个成员 / 8 个工作线程）· 每成员一遍的清单读取（0.9 s，仍是串行）· 进程启动与工作区解析（≈0.5 s）。两条路：**① 动预算**（`XIRANG_JOBS=20` 或把规则的上限提高 ⇒ 预计 2.0–2.3 s，语义零风险，但违背"不占满"的默认）；**② P0.3 缩窄语义**（≤1 s 可达，代价是否定答案失去整树保证——"文件里新增一个函数"这类会让读者照"没有"去做决定）。**我的建议：先不做 ②；如果你要 ≤1 s，我建议先让你定预算上限（例如默认一半但允许 `XIRANG_JOBS` 拉满），而不是拿答案的保证去换秒数。**

### §M7.11 P3 的形态：按维护者的原话修正（2026-10-05）

**维护者原话（逐字）**：「我本来的想法是在顶层的某个rs文件指定某树为某crate,然后xirang自动划分，然后也可以在studio中操作，partitions! { … } 这个感觉你要在每一个crate边界上写，我想要的代价是在最顶层声明一次拆分crate行为，然后就提交时生成，然后日常开发的时候是无感的，除了不同的分crate会导致编译速度不同外」。

**⇒ 三处要按这句话改我原先的说法（§M7.2 的 P3 行）**：

1. **就是"最顶层声明一次"**，我先前举例时把它讲成了"每个边界写一次"（那是我的表述问题，不是设计）：`partitions!` 只在**宿主入口那一个文件**里出现一次，块内每一行只是"哪棵子树 → 哪个 crate"的**一项**；宿主 crate 里一行都不写，幽灵 crate 里一行都不写（它们是生成的）。**并且应当支持最简拼法**：只写要切出去的那几棵子树，**没被点名的都留在宿主**（`host = […]` 是可选的白名单，不是必填）。
2. **日常开发无感、且分裂在开发期就生效**（这正是他"编译速度不同"的意思）：幽灵 crate 在本地生成（派生、gitignore），宿主的生成树**把已切出的子树排除**（否则同一批面会被编译两遍）；写代码的人只是在跑 `cargo build`，不管理任何 `Cargo.toml`。**提交/发布时才物化"发布形状"**（自包含包 + `[package.metadata.xirang] shape`），维护形状（注册树全在一个包里）仍是 git 里的那一个。
3. **Studio 是一等入口**（P3.6/P4 原本就有，这里确认它是"操作面"而不是只读视图：看当前形状、看建议切口与代价、一键重组）。

**仍然待裁的是 P3.3（身份规则）**：分区后幽灵 crate 的包名不同，而声明宏现在烤 `env!("CARGO_PKG_NAME")` ⇒ 同一个面的 `NodeId` 会变（记录/租约失效）。选项 A＝宏改读 `crate::XIRANG_NAMESPACE`（`host!()` 与生成器各提供一次；只有"不走 `host!()` 直接用宏"的调用方要在 crate 根加一行）；选项 B＝先按计划顺序做 P1/P2，P3.3 与 P3.2 一起裁。**未裁之前 P3 不动工。**

### §M7.12 P1.1 落地：图成一等产物 `graph_edges.tsv`（2026-10-05）

**一句话**：把散落在 `pruning_manifest.tsv` / `file_manifest.tsv` 与读者各自的重新推导里的图事实，变成**一个带节点数、边数与内容摘要的产物**——而且它和记录是**同一次计算的两个视图**，因此两份文件不可能对一个面产生分歧。

**改法**：① 新增 `toolchain/src/build_method/graph.rs`：`write_graph_manifest(out_dir, rows, file_rows, grafts)`；② 记录写入方**把行交回来**（`write_pruning_manifest` 与 `write_file_manifest` 现在返回它们发布的行）：图是同一批行的第二个视图，而把清单读回来建图要付同一批五万行的第二次解析——这正是本批一直在去掉的"算出来了却不交出来"，只是方向相反；③ 管线在记录之后、指纹之前写图（载荷全部落地才写指纹，因此**半份图不会被当成完整的**）。

**产物形状**（`# graph\txirang-build-graph` / `# nodes` / `# edges` / `# digest` / `# from\tto\tkind`）：

| 边 | 从 → 到 | 事实 |
| --- | --- | --- |
| `parent` | `face:<id>` → `face:<父 id>` | 记录里**解析过**的父链；解析不出的不猜 |
| `in` | `face:<id>` → `file:<源码路径>` | 这个面长在哪份文件里 |
| `calls-file` | `face:<id>` → `file:<声明它的文件>` | 调用的名字**恰好被一份文件声明**时，就是对那份文件的依赖 |
| `calls` | `face:<id>` → `name:<拼写>` | 名字被两份文件声明、或没有文件声明 ⇒ **按歧义发布**，不挑一个 |
| `graft` | `cut:<切口>` → `graft:<嫁接路径>` | 来自 graft 计划；两端是计划携带的 Rust 路径（解析属于手里有声明的读者） |

**如实说明它不做的**：不把被调用的名字解析成**面**（那需要读路径调用图手里的声明；在这里再订一条更弱的规则，会让两个答案对"这条边在不在"产生分歧）。图的节点因此有四类（face/file/name/cut），而"面依赖面"要由能解析的读者走两跳。

**验收（实测）**：
- **发布物逐字节不变**：t3（2,504 文件）用改前/改后二进制各 `verify` 一次 ⇒ 原有八份产物 `diff` 全同、sha256 全等，**新增第九份** `graph_edges.tsv` 一个 ✓
- **图上真实规模**：t3 = **5,003 节点 / 5,002 边**（2,501 `in` + 2,501 `parent`，摘要 `8290a465…`）；`examples/control-button` = 11 节点 / 8 边，含两条 `graft` 边 ✓
- **钉子**（`build_method::graph::graph_tests`，15 项）：`the_graph_is_the_records_own_rows_seen_as_edges`（记录里每个面都是节点、`face:` 节点里记录没有对应行的**只有包根**、`parent` 边等于记录的 `parent_node`、解析不出的父级**不成边**、`calls-file` 指向唯一声明的文件）· `a_name_two_files_declare_stays_a_name`（歧义按歧义发布）· `the_header_counts_and_digest_vouch_for_the_body`（计数与摘要对得上正文，截断/手改可被发现）

**门禁**：fmt ✓ · workspace test ✓ · `--features mcp` **591 项** ✓ · conventions 158 项（含 600 行棘轮）✓ · 两面 clippy ✓ · `--check-table` ✓。

**下一步**：P1.2（写路径刷新 + 外部编辑靠 stamp 差分发现落后）与 P1.3（MCP `graph` 读工具：扇入扇出/强连通分量，输出有界）。
### §M7.13 P1.2 · P1.3 · P2 落地：写即调度、索引就绪信号、图读工具（2026-10-05）

**一句话**：写入落盘后**索引在后台开始构建**；一条**就绪记录**说明它什么时候完成；而**读图的那个工具**每份答案第一行就说"这些边描述的正是此刻的源码 / 不是"。**"写一个文件是个调度问题"这句话，成了代码。**

#### 一、就绪记录 `graph.generation`（P2.1）

管线在一次**干净**运行里**最后**写它（排在 `discovery.fingerprint` 之后），原子写（temp + rename，`write_if_changed` 本就是这个形状）。字段：`generation`（数**运行次数**，从上一份 +1）、`digest`（图的边正文摘要）、`faces`/`files`（记录里的面数、源码 `.rs` 文件数）、`graph_nodes`/`graph_edges`、`stamp`（`文件数:最新 mtime`，用的是与核验记忆**同一个** `source_stamp`——这条规则在本轮从桥里搬进 `build_method`，只剩一份实现）、`finished_at`（UTC `HH:MM:SS`）。**它在＝这次运行完成了，它不在＝磁盘上是上一次（或没有）**。

#### 二、写入路径：落盘即启动刷新（P1.2）

`xirang.apply` 在**落盘成功**（不是预览）之后调 `index::start(root)`：一个后台线程驱动的是 **CLI 的 `check` 与桥的 `verify` 所驱动的那同一个入口**（`check_for`），因此刷新不可能发布一棵那两者会拒绝的树；同一个根**一次只跑一次**（两次运行发布同一个目录会互相抢载荷）；判断结果不是丢掉而是**发布**（失败的一次移除指纹、不写就绪记录）。回复末尾带上索引那一行。

#### 三、信号（P2.2）与落后即报（P2.3）

- **终端**：`xirang check` 的人类输出**第一行不变**（历史契约），其后新增一行 `graph updated: generation N, X file(s), Y face(s), <digest>`。既有钉子 `check_without_json_keeps_the_human_line` 随之更新为"第一行逐字节不变 + 第二行就是索引行"——这是**有意的契约变更**，记录在此。
- **桥**：写入回复与 `graph` 的每份答案都带这一行（同一份实现 `index::line`，因此同一个目录不会被说出两种说法）。
- **落后**：外部编辑（编辑器 / `cp` / `git`）**不靠监听**，靠**廉价戳差分**发现 ⇒ 那一行变成 `index behind: generation N covers <digest>; the sources have changed since — run xirang check`（或 `a refresh is running (Ns)`）。端到端实测：`/tmp/cb` 上 `graph` 先答 `graph updated: generation 1, …`，`>> src/control/control.rs` 之后再答 `index behind: generation 1 covers 92ef0bd3…`，而边**照旧被读出来**（说清楚，而不是静默走旧图）。
- **`Damaged`**：图与它旁边的记录对不上（截断、手改、半写）⇒ 拒绝读，并把原因说出来。这是"就绪"必须可检查的那一半。

#### 四、`xirang.graph` 读工具（P1.3）

三个问题一个工具：**不带参数**＝普查（节点/边数、边的种类、被边触及最多的节点）；`node`＋`direction`(out/in/both)＋`depth`(1–3)＝邻域；`cycles: true`＝**强连通分量**。要点三条：① 节点按**记录自己的词汇**点名（逻辑路径 / 声明路径 / kind / `registry_name` / 源码路径，或裸键 `face:`/`file:`/`name:`/`cut:`/`graft:`）——解析是记录的职责，这里不发明第二套词汇；② `cycles` 只在**依赖边**（`calls-file`、`graft`）上算（Kosaraju，**迭代**不递归，深图不会爆栈），结构边 `in`/`parent` 与未解析的 `calls` **不算依赖**，答案**明说**这一点（在错误的边集上说"没有环"是一句让人安心的假话）；③ 输出有界（默认 20 行、上限 200，超出写 `N more withheld`），并点名下一问该问什么。

**为什么 `search`/`registry`/`consistency` 不额外加这句（与计划的偏差，必须说出来）**：这三者的记录新鲜度是**逐字节**的（`freshness::verdict`），源码一变它们立刻判 `stale` 并**退回推导**——比戳差分**更强**，而且它们已经给出同一个出路（`run \`xirang check\``）。在图工具上加这句是因为**图没有可回退的推导**。计划表里的"读工具落后即报"因此**只落在图工具与写入回复上**，其余三者的行为一个字节都没改。

#### 五、验收与门禁

| 项 | 证据 |
| --- | --- |
| 就绪记录与图互相对不上 ⇒ 拒绝 | 钉子 `mcp::index::index_tests::a_record_that_does_not_match_its_graph_is_refused`（切掉最后一条边 ⇒ `Damaged`，理由点名计数/摘要不符） |
| 没发布过 ⇒ 说该跑什么 | `a_run_that_published_nothing_reads_as_absent`、`mcp::graph::graph_tests::an_unpublished_index_says_what_to_run` |
| 就绪 / 落后 两态 | `a_finished_run_publishes_a_record_that_says_ready`、`an_edit_makes_the_index_behind_and_the_line_says_so`、`a_behind_index_is_answered_from_and_named` |
| 图工具三问 | `mcp::graph::graph_tests::{the_census_opens_with_the_index_line_and_counts_the_edges, a_logical_path_resolves_through_the_record_and_shows_its_edges, the_cycle_between_the_two_files_is_named_as_one_component, a_name_the_record_does_not_know_is_refused_with_the_spellings_it_does}` |
| 跨文件调用是文件之间的边（环的原料） | `build_method::graph::graph_tests::a_call_that_crosses_files_is_an_edge_between_them` |
| CLI 那一行 | `cli::lib_tests::check_without_json_keeps_the_human_line`（第一行不变 + 第二行是索引行） |
| 工具注册 | `tools_tests::the_dispatch_table_follows_the_catalog`、`READ_KEYS` 的表点名 `xirang.graph`；`--list` 预算从 5000 提到 **5200**（第 29 个工具，约 90 字符/条目——**动的是界，不是把条目削到能塞进去**，理由写在钉子旁） |

端到端（真二进制）：`--call graph --root /tmp/cb` ⇒ `graph updated: generation 1, 5 file(s), 3 face(s), 92ef0bd3…` + 11 节点 / 8 边（含两条 `graft`）；`--cycles true` ⇒ 0 个分量并写明只数依赖边；`xirang check /tmp/cb` ⇒ `xirang check: ok (…)` + `graph updated: generation 1, …`。

#### 六、这一批补记的四处（都是"自我保护"而不是功能）

1. **一次性客户端会等**：`--call` 在回复写出之后、退出之前调 `index::settle(60s)`——进程退出会杀掉它刚启动的后台刷新，而"启动了"在那条路径上本来是句空话。等一段（有界）让常见的 `--call apply` 真把它开始的事做完；超时就往 stderr 说清楚并**留下可被报告的状态**（载荷逐个原子写、就绪记录最后写 ⇒ 半途停下会被报出来，不会被相信）。stdio 服务路径不经过这里（它没有理由退出）。
2. **"这个桥从不构建"这句话现在不成立了**，`build_evidence.rs` 的两处文案已改成事实：`apply` 在后台启动一次、一次性客户端会等它、`verify` 按需构建。**自我描述与行为必须一致**，这是本仓的红线之一。
3. **一处钉子因批次的真实行为而失效并被修好**（不是放宽）：`every_report_spells_the_freshness_word_that_one_place_produces` 里的 "stale" 夹具原本是"经写入路径写下、还没有证据"——而写入现在会启动索引刷新 ⇒ 夹具一瞬之后自己就发布了证据，报告叫它 `current` 是**对的**。夹具改成**真实的过期态**：经写入路径写下 → `wait_until_idle` 等到刷新结束 → 改一个**指纹连内容一起覆盖**的文件（`src/label/label.rs`；`src/lib.rs` 不行——原始文件集合只按路径取哈希，发现过程也跳过它）。
4. **两处门禁被新代码顶到上限，按规则处置**：`tools.rs` 越过 600 行 ⇒ 把图工具的**描述与 schema 搬到它实现旁边**（`graph.rs` 的 `DESCRIPTION` / `schema()`，两者因此不会漂开）、把 `READ_KEYS` 表搬进 `tools_tests.rs`（测试预算 800）；`mcp::graph` 里的裸动词 `resolve` 违反动词表 ⇒ 改名 `resolve_nodes`。`--list` 的字节预算从 5000 提到 5200（第 29 个工具；理由写在钉子旁）。

### §M7.14 P3.1 落地：`add_crates!` 的**函数/常量**形态（2026-10-06）

维护者两句指示定了这一版的形状：命名用 **Add crates**（`partitions` 有歧义），而且**声明要是函数、不是宏**——"不然连自动补全都没有"。第二句是对的，而且理由比补全更深：宏里那点补全靠"镜像宏"补丁，而函数/常量形态下，**字段名、方法名、子树路径都是真 Rust**，于是编辑器原生补全、写错是编译错误、类型不对是类型错误。

**声明的成品样子**（包根 `add_crates.rs`，包级文件，与 `Cargo.toml`/`build.rs` 同级——放 `src/` 下会被发现遍历读到）：

```rust
use xirang_toolchain::run_method::{Crate, Shape};

pub const SHAPE: Shape = Shape {
    package_prefix: "xirang-example-control-button",
    crates: &[
        Crate::named("widgets").at(&[crate::control::object::SUBTREE]),
        Crate::named("rules").at(&[crate::control::registry_rule::SUBTREE]),
    ],
};
```

**落地清单**（每项一个提交，门禁在批末跑）：

| 件 | 位置 | 钉了什么 |
| --- | --- | --- |
| 规则（唯一一份） | `kernel/src/registry_core/shape/shape.rs` | 边界是**整个 `::` 段**（`control` 含 `control::object`、不含 `control_extra`）；重叠/嵌套/重名/空名/空前缀各**点名**拒绝。5 条钉子 |
| 函数面 | `toolchain/src/run_method/shape.rs` | `Subtree` / `Crate` / `Shape` / `add_crates(&Shape)`；宿主自己的 crate 装载时校验。2 条钉子 |
| 构建期读者 | `toolchain/src/build_method/shape_decl.rs` | 读包根那份文件（**文本**，不链接宿主），只认 `Crate::named(…).at(&[…::SUBTREE])`，别的拼写**点名拒绝**；写 `add-crates.lock`（声明身份 + 每 crate 的子树与面集合身份 + 留在宿主的面数）。6 条钉子 |
| 渲染器 | `build_method/renderer/tree.rs` | 每个**生成的内联模块**发一个 `pub const SUBTREE: Subtree = Subtree::new(module_path!())` |
| 词汇表 | `kernel/src/registry_core/lexicon/lexicon.rs` | `ADD_CRATES_FILE` / `ADD_CRATES_LOCK_FILE` / `ADD_CRATES_MARKER` |
| 管线 | `build_method/pipeline.rs` | 树干净时校验声明；不成立 ⇒ `add-crates` 诊断，且**发布任何产物之前**失败。没有声明 = 一个 crate 的包（不是错误） |

**端到端实测**（真的在 `examples/control-button` 上加了声明 + 一行 `#[path]` 挂载，跑完已还原）：`cargo build` 过 ✓；CLI `check` 写出

```
# add-crates	xirang-crate-shape
# declaration	28815a30eab2b8f39b7daedebba59911
package_prefix	xirang-example-control-button
crate	widgets	subtrees=control::object	faces=2	face_set=25404c233d3361da27fb78d24e167410
crate	rules	subtrees=control::registry_rule	faces=0	face_set=e3b0c44298fc1c149afbf4c8996fb924
host	faces=1
```

顺带两条**免费的证据**：加 `add_crates.rs` **没有**改变 `discovery.fingerprint` 与 generation 的摘要（`92ef0bd3…` 不变）——因为发现只走 `src/**`，这正是"声明必须放包根"的另一个理由；以及 `rules` 那个子树 `faces=0` 是对的（`registry_rule.rs` 是**规则**不是注册面）。

**测出来的一个真缺口（未解决，需要裁决）**：**文件叶子节点没有可点名的标记**。生成树对"有自己文件的叶子"是直接挂载 `#[path=…] pub mod <name>;`（模块**就是用户那份文件**），而 `SUBTREE` 只能发在**生成器拥有的内联模块**里 ⇒ 于是

```
error[E0425]: cannot find value `SUBTREE` in module `crate::control::registry_rule`
 --> examples/control-button/src/../add_crates.rs:9:67
```

目录节点（`control`、`control::object`）今天就能点名 ✓；叶子（`…::button`、`…::registry_rule`）不能 ✗。三条出路：

| | 做法 | 好处 | 代价 |
| --- | --- | --- | --- |
| **A** | 叶子也套一层生成的内联模块（`pub mod button { #[path] pub mod button; pub use button::*; pub const SUBTREE = … }`），与容器面今天的形态一致 | 统一、拼法最好看（`…::button::SUBTREE`） | 改动核心渲染器、影响每个宿主；必须用"改动前后记录逐字节相同"的闸门实测（身份来自 `file!()` + `module_path!()` 的**最后一段** + 父链，理论上不变） |
| **B** | 父模块为每个子节点发一个标记常量（`SUBTREE_BUTTON`） | 零结构变化 | 声明拼法难看，且名字是拼出来的 |
| **C** | 叶子面用**今天就有**的 `::NODE_ID`；非面的文件叶子（规则）暂时不能单独成 crate，点名拒绝 | 零风险、马上可用 | 两种标记；规则文件不能单独切 |

我倾向 **A**（配上逐字节闸门实测），**C** 作为过渡。这一条定下来之前，`add_crates.rs` 只能点名**目录子树**。

**仍待补（本轮有意没做）**：① 生成的挂载树自动引入 `add_crates.rs` 并调用 `add_crates(&SHAPE)`（今天宿主自己写一行 `#[path = "../add_crates.rs"] pub mod add_crates;` 即可）；② `.xirang/add-crates.lock` 的进 git 副本（现在只写进构建产物目录）；③ CLI/Studio 的 `--add/--remove` 写这份文件；④ P3.2 的生成器、facade 与 graft 渲染。

**裁决：撤掉 C —— 规则是"有子树的节点才成得了 crate"（维护者 2026-10-06，原话「**而且就一个对象我包裹成crate我不是有毛病吗？？我的理解是就是这个节点必须有子树才能打包crate呗**」）**

他是对的，而且 C 是在解一个**不该存在的问题**：把一个对象包成 crate 什么也换不来。于是 `faces(&[…::NODE_ID])` 撤掉（内核的 `faces` 计数、runtime 的第二个构造器、读者的第二种标记一并撤），只剩一条拼法 `.at(&[…::SUBTREE])`。

更值得记的是：**这条规则已经由编译器守着，不需要任何新机制**。`SUBTREE` 恰好只发在**有子节点**的节点上——渲染器给有子节点的节点搭一个构建自己的内联模块，而没有子节点的节点是直接 `#[path]` 挂到它的文件上（模块**就是**那份文件，而文件写不进去）。所以：

| 节点 | 有 `SUBTREE` 吗 | 能成 crate 吗 |
| --- | --- | --- |
| `control`（有文件 + 有子节点） | ✓ | ✓（它带着自己的子树） |
| `control::object`（只有子节点） | ✓ | ✓ |
| `button` / `slider` / `registry_rule`（文件叶子） | ✗ | ✗ —— 编译器直接拒 |

真宿主实测（`examples/control-button`，跑完已还原）：写 `Crate::named("button").at(&[crate::control::object::button::SUBTREE])` ⇒

```
error[E0425]: cannot find value `SUBTREE` in module `crate::control::object::button`
 --> examples/control-button/src/../add_crates.rs:8:69
```

**所以先前记的"叶子缺口"与出路 A 一起作废**（A 也不必做了：叶子本就不该成 crate，不是"缺失的能力"）。

构建期读者说同一条规则，覆盖编译器看不见的那一半（声明没有被任何 crate 挂载时），而且它读得到树，所以能**点名出路**：① 认领的是一个面 ⇒ `` `X` is a face and not a subtree; name `parent` instead``；② 认领的子树下面没有面 ⇒ `` `X` has no faces below it; that crate would be empty``。

**合法形状的真宿主实测**（已还原）：`widgets = [crate::control::object::SUBTREE]` ⇒ `cargo build` 过 ✓，锁里 `crate widgets subtrees=control::object faces=2` / `host faces=1` ✓，generation 摘要仍是 `92ef0bd3…` ✓（声明不碰记录）。

钉子：内核 5 条 · runtime 2 条 · 构建期读者 8 条（其中两条就是上面那两条点名拒绝）。

### §M7.15 身份漂移：从静默变成编译错误（2026-10-06）

**动机**是 P3 的失效形态：分区 crate 用 `#[path]` 挂载同一个面文件，若拼写（或 `--remap-path-prefix`）不能把 `file!()` 还原成树记录的那条路径，面的 `NODE_ID` 就会与构建烤进 `BUILTIN_STATIC_FACES` 的那个不一致 —— 而在此之前**没有任何东西会报错**。

**实测（改动前）**：`cargo rustc -p xirang-example-control-button --lib -- --remap-path-prefix=<宿主 src>=/tmp/ghost` ⇒ **exit 0** ✗ —— 构建成功，而每个面的身份都已经变了（`manifest_relative_source` 在前缀不匹配时**回退成原始 `file!()`**，于是相对路径变了）。graft 切口随后点名的是一个没有任何面拥有的身份。

**修法**：内核新增 `release::assert_static_identity(face: NodeId, planned: NodeId)`（`const fn`，逐字节比较，不符即 panic 并点名"这个面编译出的 id 不是构建烤进去的那个"）；渲染器为**每个静态面**在数组之外发一条 `#[cfg(not(rust_analyzer))] const _: () = assert_static_identity(crate::<模块>::NODE_ID, NodeId::from_raw([…]));`。

**实测（改动后）**：同一条 remap 命令 ⇒ **exit 101**，`error[E0080]: evaluation panicked: static identity failed: this face compiles an id the build did not bake, so its namespace or source path differs from the tree this plan came from (see the `#[path]` spelling a partitioned crate mounts it through, and `XIRANG_NAMESPACE`)` ✓；正常构建仍 **exit 0** ✓（真宿主上零误报）。

**它管什么、不管什么（边界要说清）**：
- **管**：构建的算法与编译器的算法**不一致**（分区挂载拼写错、漏 remap、命名空间来源不同）⇒ 当场红；
- **不管**：两边**一起变**的情形（例如把面文件搬走 ⇒ 构建与编译器都算出新 id ⇒ 断言照过），那属于"记录/租约漂移"，仍由既有的漂移检测（`freshness`/台账）负责。⇒ 两条规则各管一半，不互相替代。

钉子：内核 doctest 两条（同 id 通过 / 异 id `compile_fail`）· 渲染器 1 条（烤进去的每个面身份都要与面对账）。

### §M7.16 `graft_plan.tsv` 的位置列：从"宏那一行"改成"每条切口自己的行"（2026-10-06）

**症状**（查账时实测）：`graft_plan.tsv` 两条切口的 `line`/`column` 都是 `48 1`，而它们在入口里是**第 50 与第 52 行**；48 是 `static_graft_plan!(` 那一行。于是 `apply_promote` 的消息（"the declaration at …:48"）与 `grafts.rs` 的 "declared at entry line 48" 都把读者指到宏上。

**根因**（`kernel/src/registry_core/syntax/entries/graft.rs`）：每条条目发布的是**宏的**位置 —— `location(mac.span())`；而且类型化分支的 `cut_span` 本身也被赋成了 `mac.span()`（字符串分支用的是字面量/组的 span，本来就是对的）。

**修法**：类型化切口的 `cut_span` 用那个**括号组**的 span（`group.span()`，随解析出的一对一起传出来）；条目发布 `location(cut_span)`；同一处三条"条目内部的拒绝"（`graft cut expects graft …`、`a graft cut must name both sides …`、`graft expects a string literal or (…)`）也改成指向该条目的切口。

**实测（真宿主）**：

```
# 修前                      # 修后
… button  false  48  1      … button  false  50  8
… slider  false  48  1      … slider  false  52  8
```
（入口里 `grep -n "cut("` 给的就是第 50/52 行 ✓；列 8 是那个 `(…::NODE_ID)` 组的起始列 ✓。）

**钉子**：`graft_tests` 新增两条 —— 一条断言"一个宏里的两条条目各报自己的行，且位置不同"，一条断言"条目内部的拒绝指到那条条目所在的行"（该模块 15 条全绿）。写入路径不受影响：`repoint_graft` 一直按**字节**定位，从不读这个列（它的注释就写着为什么）。

### §M7.17 外部计划：实测它的读取与强制（2026-10-06）

**背景**：仓库里**一条外部计划都没有**，所以这条链从没端到端跑过。我在检出外的宿主副本（`/tmp/ext/control-button`，`path` 依赖重指到本检出）上手写了两份计划跑了一遍。

**读取与判定（都通）**：

```
button_fast: target=root/control/button graft=button_fast full=false [declared]
    declared at line 50 as cut `crate::control::object::button::NODE_ID` graft `control_button_graft::button_fast::NODE_ID`
unknown_slot: target=root/control graft=control_ext full=false [NOT declared by the host entry]
```
—— 第一行里的 **line 50 正是今天 `ac96d69` 修好的那个列**（修前会印 48）✓。

**强制**：未声明槽位的计划**让构建失败**（`cargo build` **exit 101**），错误是生成代码里的一条 `compile_error!`，消息点名计划文件并给出**可粘贴的补救行**：

```
source=.xirang/external-grafts/unknown_slot/graft.plan
`-- external graft plan `unknown_slot` targets `root/control`, which no declaration in the host entry
    names; the release-time plan keeps no such slot alive, so the record could never take effect.
    Declare it in static_graft_plan!: cut "root/control" graft "control_ext",
compile_error!("NICHLink BUILD CHECK FAILED …")
```

⚠️ **更正一处自述**：`grafts.rs` 的模块文档原文说这里"构建会**警告**，而 `cargo` 日志正是那条警告湮没的地方"——实测是**硬失败**。已把那段中英文都改成事实（"build **fails** rather than warning …exits 101"）。

**`promote` 的三次点名拒绝（都是设计要的窄而会拒绝）**：① 计划里的 `graft` 与实现 crate 实际声明的面不符 ⇒ `this action will not land a face the record did not select`；② **计划目录名必须等于 `graft` 名** ⇒ `graft record directory `button_ext` and its plan's `graft=button_fast` disagree; rename the directory or the file so the selector and the implementation name agree`；③ **目标面必须是"生成的"**（`// generated-by=XiRang`）⇒ `this module was not generated by XiRang` + 两条 `way forward:`（手写面只能在编辑器里落地，或一开始就用 `apply add`/`new_project` 生成）+ 一条 `boundary:`（`add`/`deepen`/`cut`/`promote` 是**追加**类、能手写面；`edit`/`rename`/`delete` 重写文件，因此拒绝手写面）。

**拒绝是干净的**：三次被拒之后逐字节比对——入口 `src/lib.rs` 未变 ✓、目标面文件未变 ✓、计划仍留在 `.xirang/external-grafts/`（没进 trash）✓ ⇒ **没有半成品写入**。

**还没跑通的那半**：**promote 的成功路径**需要一个**由 `apply add` 生成**的目标面（外加一个真的声明了对应实现面的实现 crate），本轮没搭；另外 `create_external_graft` 目前**只从 Studio 调用**（`studio/app/graft.rs`），CLI/桥都没有创建计划的入口 ⇒ 无可视界面的用户只能手写计划文件。这两条都记进待办。

**promote 的成功路径（本轮也跑通了）**：需要一个**由 `apply add` 生成**的目标面（外加一个真的声明了对应实现面的实现 crate），因为 promote 会同时改写**目标面的声明**（经创作执行器，只改它生成的文件）与**入口里那条切口**。步骤：`apply add`（要带 `handle_traits`/`handle_contracts`，否则父注册机的规则会以 `must implement interface ControlHandle` 点名拒绝）→ 给实现 crate 加一个名字匹配的实现面 → 计划写进 `.xirang/external-grafts/<实现名>/graft.plan` → 入口里把该槽位声明成**类型化**写法 → `apply promote`。成功时的回复要点：

```
landed `toggle_fast` into `root/control/toggle`: the declaration … now carries the external
implementation's fields, the entry in src/lib.rs now points that slot at **this face itself**,
and the record moved to `.xirang/trash/external-grafts/`
the entry at src/lib.rs:54 now reads `cut(crate::control::object::toggle::NODE_ID) graft(crate::control::object::toggle::NODE_ID)` (3 → 3 entries, unchanged)
note   the kind moved: `Toggle` → `ToggleFast`. `kind` is an identity input …, so this face is no longer `4dbbd3f9…`
consequences (static, text-level): 2 in-tree test line(s) name this face
  tests/registry.rs:181 / tests/registry.rs:187 …
```

重建后生成表里第三条切口是**自嫁接**：`from_ids(crate::control::object::toggle::NODE_ID, crate::control::object::toggle::NODE_ID, false)` ✓。

**顺带修掉一个真 bug**：字符串写法的拒绝说「…or pass `implementation` with the external crate's path」，可那段代码**在那句话之前就返回了** ⇒ `implementation` 永远救不了字符串写法 ✗（`tools.rs` 的工具自述与 `implementation` 键的描述也照着这么写）。已改成事实：**字符串说不出实现在哪，`implementation` 是给类型化声明定位 crate 用的** ⇒ 出路是先把该槽位改写成 `graft(<crate>::<module>::NODE_ID)` 再来一次；两处自述同步。

⚠️ **门禁教训（本轮踩到）**：既有钉子 `repointing_one_entry_keeps_the_others_as_they_were` 里有一句 **"前提"** 断言——"解析器给每条条目都报同一行"——它住的模块属于 **`mcp` 面**，而 `cargo test --workspace`（默认特性）**不跑那个面** ⇒ `ac96d69` 把这个 bug 修好之后，那条钉子**在暗处变红了**，而我当时跑的门禁（workspace + clippy 两面 + conventions + 发布表）都没碰到它。已把它的前提改成"各条目报各自的行（`[3, 4, 6]`）"，并把 **`tools/xirang-test`（十面跑测器）** 作为本批门禁之一重跑。

### §M7.18 跨进程的命名空间竞态：读者不再把「别人的记录」读成「身份搬家」（2026-10-06，**已修**）

**可复现的演示（确定性，不靠时序）**：把一个包的记录发布在一个身份域下，再**用另一个身份域去读**——记录一个字节都没动，答案却变成"这个面换了身份"：

```bash
mkdir -p /tmp/nsrace/src && printf '[package]\nname = "nsrace"\nversion = "0.1.0"\nedition = "2021"\n' > /tmp/nsrace/Cargo.toml
printf '// host entry\n' > /tmp/nsrace/src/lib.rs
MCP="cargo run --offline -q -p xirang-toolchain --features mcp --bin xirang-mcp --"

# 1) 按【包名命名空间】发布记录
$MCP --call apply --root /tmp/nsrace --json '{"action":"add","apply":true,"fields":{"module":"label","kind":"Label"}}'

# 2) 同一个身份域读 ⇒ 健康
$MCP --call diff --root /tmp/nsrace
#   faces 1 (source) vs 1 (build)
#   added since build 0  gone 0  re-identified 0

# 3) 换一个身份域读（记录没动）
XIRANG_NAMESPACE=t48-override $MCP --call diff --root /tmp/nsrace
#   namespace t48-override
#   faces 1 (source) vs 1 (build)
#   added since build 0  gone 0  re-identified 1      ← ✗ 错但看起来对的答案
#   re-identified:
#     ~ root/label 3424be7f7d83f46afb39b00a08565f5e -> 30926530a158f3572ba61e3bd387b42a
```

两个 id 的差别**只来自命名空间**（记录里的 `3424be7f…` 是在包名下算的，`30926530…` 是按 `t48-override` 算的），而答案把它说成"这个面被 re-identified"，还附一条 `~` 的**旧→新映射** ⇒ 读起来更像"身份真的搬家了" ✗✗。这正是"看起来正确的错误答案"：源码没改、记录完好，而读者会去查一个不存在的问题。

**而它在测试里怎么现形的**（本节的另一半）：那条钉子构造"两个进程 + 两个身份域"，父进程的 `apply` 还会在后台再刷一次索引 ⇒ 那次刷新可能落在子进程 `verify` 之后、`diff` 之前，于是子进程读到的记录属于父进程的身份域 ⇒ 同一族的错答案以**间歇**的形式出现（单独跑必过、整面并行跑才红）。


**症状**：`toolchain+mcp` 面里 `mcp::verify::verify_tests::the_override_is_the_namespace_the_run_publishes_under` **间歇红**；子进程输出

```
faces 1 (source) vs 1 (build)
added since build 0  gone 0  re-identified 1
```

（断言要 `re-identified 0`）；**单独跑必过、整面并行跑才红**（我先跑单条得绿，再跑模块/整面得红，最后给测试临时加一行 eprintln 打印子进程 stderr 才看清——stderr 是空的，不是崩溃）。

**机制**：这条钉子先在**本进程** `apply add`，再起一个**子进程**（`XIRANG_NAMESPACE=t48-override`）跑 `verify` + `diff` ✓。而 P1.2 之后 `apply` 落盘会**在后台**再刷一次索引（与 CLI `check`/桥 `verify` 同一个入口）⇒ 父进程那次刷新可能落在**子进程 `verify` 之后、`diff` 之前**，把记录改回**父进程命名空间**的版本 ⇒ 子进程的 `diff` 读到的记录属于另一个身份域 ⇒ 它如实地说"每个面都 re-identified" ✗。

**为什么算缺陷而不是测试毛病**：两个进程共用一棵树是**正常**用法（CLI 与桥同时看着一棵树）；而"每个面都搬家了"正是"**这份记录属于另一个身份域**"的签名——今天没有任何一处把这个原因说出来（`graph.generation` 的字段里**没有命名空间**）⇒ 读者拿到的就是 `LGC-LG-13` 那条钉子当年防的"看起来正确的错误答案"。

**候选修法**：① **在记录里盖命名空间**（`graph.generation` 增一列/一行）⇒ 读者能直说"这些记录是在命名空间 X 下发布的，你按 Y 读 ⇒ 先跑 `check`"，而不是"所有面都搬家了" ✓（我推荐）；② 只在"全部 re-identified"时加一行提示（记录里没有依据 ⇒ 只能猜 ✗）；③ 测试侧在 `apply` 后等刷新结束（**只掩盖产品问题** ✗）。

**现状**：本批不修，记入待办；`tools/xirang-test` 因此**恰好在这一条上红**（其余九面全绿，`toolchain+mcp` 615 通过 / 1 失败）。

**修法（已落地）**：① 记录里盖命名空间 —— `graph.generation` 多一行 `namespace\t<本次运行的命名空间>`（写方与身份用同一个来源：`identity::package_namespace()`）；② 读取方在**任何比较之前**先过一道闸（`TreeDelta::other_domain_refusal`，一条规则一处实现）：记录里的域与本次运行的域不同 ⇒ **拒绝**，点名两个域与唯一出路：

```
namespace mismatch: the records in /tmp/nsrace/target/xirang/out were published under `nsrace`, and
this run reads identities as `t48-override`. Identity is the namespace plus the source path plus the
name, so the same face has two different ids here — comparing them would report every face as moved,
which is what this refuses to do.
way forward: run `xirang check` in this tree (it republishes under `t48-override`), or set
XIRANG_NAMESPACE=nsrace and ask again
```

③ **旧记录降级**：没有这一行（更早的二进制写的）⇒ 读出 `None` ⇒ 对命名空间**什么都不说**（退回旧措辞），不许猜 ✓。这道闸放在 `diff_body` 的三个分支**之前**，因此 `against` 与 `records: true` 那两条「记录对记录」的比较也走同一道闸 ✓。

**代价与效果（实测）**：记录多一行（约 20 字节）；两端本来就**按键解析**（`header_value(&text, "key\t")`）⇒ 新写×旧读忽略未知行 ✓、旧写×新读降级 ✓，不是格式迁移。**域匹配时输出一个字节都不变**（实测 `re-identified 0` 与修前逐字相同）；**域不匹配时**从「每个面都搬家 + 一串 `~` 旧→新映射」变成「一句原因 + 两条出路」⇒ 出错时更短更准，且不再让读者去追一个不存在的问题。**仍未解决**：并发**写**同一棵树仍是后写者赢（这道闸只让读者看懂「这不是我的域」）；要彻底避免得加跨进程互斥，单独立项。

**钉子**：① `index_tests::the_record_stamps_the_namespace_and_old_records_read_as_none`（盖戳 + 旧记录降级）；② `diff_tests::records_published_under_another_namespace_are_refused`（**就是上面那个演示**：父进程按 A 发布、子进程按 B 读 ⇒ 断言拒绝里点名两个域、带出路、且**不**输出 `re-identified`）；③ 那条「间歇红」的 verify 钉子改为**先等它自己启动的那次刷新**（`index::wait_until_idle`）再起读者 —— 产品对那个状态的答复现在是**点名拒绝**，测试侧的等待因此不再掩盖任何东西 ✓。

### §M7.19 P3.3 落地：身份命名空间从"每处 `env!`"改成"crate 根常量"（2026-10-06）

**改什么**：声明宏过去在**每个声明处**读 `env!("CARGO_PKG_NAME")`；现在读 crate 根的 `crate::XIRANG_NAMESPACE`，而 `host!()` 用包名定义这个常量。四处宏内落点：`face_objects.rs`（默认父级）、`face_external.rs`（同上）、`face_registration.rs`（`NODE_ID` 与 `REGISTRATION.namespace`）。**为什么是分区的前置**：分区后的 crate 会用另一个 `#[path]` 挂载同一个面文件，在那里读 `env!` 得到的是**幽灵 crate 的**包名，而身份是 `hash(命名空间, 源码路径, 名字)` ⇒ 每个面都会被**静默重命名**；根上一个常量，正是分区能指向宿主那个值的唯一位置。

**代价（写清楚，因为它出现在用户面上）**：不走 `host!()` 而直接用声明宏的 crate，要自己在 crate 根写一行 `pub const XIRANG_NAMESPACE: &str = env!("CARGO_PKG_NAME");`。本仓已按此处理：外部实现 crate（`examples/control-button-graft`）· 八个 `toolchain/tests/*.rs` 集成测试 · `toolchain/src/lib.rs` 里给 crate 内探针的 `#[cfg(test)]` 常量（值与它们过去烤进去的**完全相同**）· 宏文档里那道 doctest 也把这个常量写进示例（并显式给一个 `fn main`，否则整块会被包进函数、常量就落不到 crate 根）。

**验收：身份逐字节不变（这是 P3.3 唯一真正的判据）**。先复制改前的 `examples/control-button/target/xirang/out`，再用新代码重跑同一棵树：

| 产物 | 结果 |
| --- | --- |
| `pruning_manifest.tsv` 的 **(id, source)** 列（4 个面） | **逐字节相同** ✓ |
| `graph_edges.tsv` · `source_scope.tsv` · `function_manifest.tsv` · `file_manifest.tsv` · `graft_plan.tsv` | **整份逐字节相同** ✓ |
| `shape_manifest.tsv` | 只有那一行如预期地变：`parent crate::root_node_id(env!("CARGO_PKG_NAME"))` → `crate::root_node_id(crate::XIRANG_NAMESPACE)` ✓ |
| `discovery.fingerprint` · `graph.generation` | 变了（源码字节变了 ⇒ 指纹变；新一轮 ⇒ generation +1，且多了 §M7.18 的 `namespace` 行）✓ |
| 生成树里的 `assert_static_identity`（§M7.15 那道闸） | **3 条都在，且编译通过** ⇒ 编译器算出的 id 与构建烤进去的仍然一致 ✓✓ |

**钉子**：新增集成测试 `toolchain/tests/namespace_constant.rs` —— 它的常量**故意不等于包名**（`"p33-probe-domain"`），两个宏族各声明一个面，断言 `NODE_ID == from_namespaced_path(XIRANG_NAMESPACE, 相对源码, kind)` 且 **≠** 用包名算的那个 ⇒ 宏若改回读包名，这条立刻红 ✓。该 target 不声明 `required-features`：`features` 门禁拒绝"点名默认开着的特性"（我第一版写了 `["run"]`，被它按规矩拒了 ✓）。

**自述同步**：`package.rs` · `identity.rs` · `mcp/workspace.rs` · `mcp/registry.rs` · `studio/app/namespace.rs` · `studio/app/project_context.rs` · `runtime/trace/snapshot/io.rs` · `kernel/syntax/face.rs` 以及宏自己的 `NODE_ID` 文档——凡是说"宏把 `env!(CARGO_PKG_NAME)` 烤进去"的地方都改成事实（中英各一遍）✓。

**一处 clippy 取舍**：`clippy::crate_in_macro_def` 认为宏里写 `crate::` 是想写 `$crate`，而这里**必须**是调用点的 crate 根（`$crate` 会指工具链自己、什么都解析不到）。处理方式是 `macros.rs` 里**一处**模块级 `#![allow(clippy::crate_in_macro_def)]` + 中英理由 ✓（不是逐处 allow）。

**已知未做**：`conventions` 里还**没有**"面文件不得拼 `env!("CARGO_PKG_NAME")`"的门禁——它是这条规则不回退的保障，下一步就补（新规则约百行：只判**声明了注册面**的文件，因为测试 crate 给自己的常量、以及 `host!()` 自己的定义都必须保留这个拼法）。

### §M7.20 跨进程互斥：一棵树一个写者（2026-10-06）

**为什么**：CLI 与桥、两个 agent 会话、或构建脚本与编辑器的 `check` 可能**同时**看着一棵树。每次发布写的是一**组**文件（清单/图/graft/指纹/就绪记录），每份各自原子（`write_if_changed` 写临时文件再改名），但**这一组不是** ⇒ 没有锁时后写者逐份取胜：读者可能读到"新指纹 + 旧清单"，一次运行也可能静默覆盖另一次刚发布的代。

**是什么**：`<output 目录的父目录>/.publishing.lock`（住在输出目录**旁边**，因此 `target/xirang/out/**` 的产物集合一个文件都不多），用 `create_new` 原子创建，内容两行 `pid\t…` / `started\t…`（与本工作区其它记录同形），发布结束时由 Drop 删除。**陈旧锁**：持有者活着就等；pid 已死（Linux 查 `/proc/<pid>`，其它平台"假定活着"）或没有 pid 且年龄 ≥ 10 分钟 ⇒ 接管，并在真宿主构建时印一行 `cargo:warning=removed a stale publish lock left by pid N`。**等待有界**：`XIRANG_LOCK_WAIT_MS`（默认 30 s）⇒ 超时不是挂死，而是一句点名锁文件、持有者与两条出路的拒绝。

**锁在哪一段**：只包住**载荷写入**这一段（`pipeline.rs` 的写阶段），而不是整次运行——上面的发现/解析/渲染是计算、幂等、而且昂贵；不许交错的只有写入。拿不到锁的那次运行**什么都不发布**（不会在别人的文件旁留下半代产物），并以 `BuildDiagnostic`（phase `publish-lock`）失败。

**边界（写明）**：它不是通用跨进程互斥，也**不是给读者的保证**——想要一致"一组"的读者最后读就绪记录（`graph.generation`，管线一向最后写它）；两个进程的**写**仍然独立，只是不再交错。

**钉子**（`build_method::publish_lock`，7 条 + 1 子进程）：取到即存在、点名持有者、Drop 后消失 · 第二个写者**等**而不是并肩写 · 活持有者被**点名拒绝**（含 `way forward` 与预算变量名）· 已死 pid 的锁被**接管**且报告 pid · 无法辨认但新鲜的锁被**尊重** · 预算从环境读 · **端到端**：占住锁 ⇒ 子进程那次发布被拒且**一个载荷都没落盘**，清掉锁后**同一个夹具**能发布（正对照）。实测真宿主：`check` 后**无锁残留**、五份产物仍逐字节未变、`(id, source)` 列不变、digest 仍 `92ef0bd3…`。

### §M7.21 门禁：面文件不得拼 `env!("CARGO_PKG_NAME")`（2026-10-06）

P3.3 把身份来源改成 crate 根常量，这道门禁是它不回退的保障（`conventions::namespace_source`）。**只判**：某个 crate `src/` 树里**调用声明宏**的**代码行**。**不判**（各有理由）：`tests/` 下的路径与 `<name>_tests.rs`（集成测试与仅检出可用的夹具是给自己常量下定义的测试 crate；解析器的字符串夹具根本不编译）· 注释与文档行（文档可以、也必须谈论旧拼法）· 不是调用的拼法（字符串里或数组里的名字）。**唯一允许的例外**：**定义常量本身**那一行。

**它当场抓到一处我 sweep 漏掉的真命中**：`examples/control-button-graft/src/lib.rs` 的 `external_registry()` 仍写 `Registry::root_for_namespace(FRAMEWORK, env!("CARGO_PKG_NAME"))`（我先前只替换了 `root_node_id(env!(…))` 那种形状）⇒ 已改成读常量 ✓。

### §M7.22 P3.2 起点（已勘察，下一步就动这几处）

**第一刀（自包含、可判定）：宿主生成树必须**跳过**已被切出的子树** —— 否则同一批面在两个 crate 里各编译一遍，正是分区要避免的事。已勘察的落点：
- `shape_decl::check_shape_declaration(package_root, out_dir, rows) -> Result<(), String>` 已经**解析出**声明（`declaration.crates: Vec<(name, subtrees)>`，子树的拼法是 `control::object` 这样的模块路径），但只用于校验与写 `add-crates.lock` ⇒ 让它把声明**交出去**（或由管线读一次传下去）。
- `renderer::pass::render(…)` 里 `let ide_shadows = render_nodes(&mut output, src, scope, nodes);` ⇒ `render_nodes` 增加一个"被切出的模块路径前缀集合"，命中即**不发射**该内联模块（及其整棵子树）。
- 语义边界（重要）：**记录不变、只有生成树变** —— `pruning_manifest.tsv` 仍列出全部面（它们是这棵树的面），变的是**这个 crate 编译哪些**；因此"切口两端身份逐字节相同"的验收成立。
**第二刀**：生成幽灵 crate（`<package_prefix>-<name>/`，`src/lib.rs` 用 `#[path = "../../<host>/src/…"]` 挂载宿主面文件，且**显式定义** `pub const XIRANG_NAMESPACE: &str = "<宿主的命名空间>";` ⇒ P3.3 正是为它做的）＋路径 remap（`--remap-path-prefix`）保证 `file!()` 逐字节回到宿主相对路径 ⇒ 身份不变（`b261fe0` 那道闸会在拼错时当场红）。
**第三刀**：facade crate —— 它同时依赖宿主、所有幽灵 crate 与实现 crate，**切口表与两条 `assert_contract` 渲染进它**（分区后被切的面在宿主里解析不到 ⇒ 只有 facade 两端都可见）。
**验收（维护者定的四条）**：同一份含 graft 的宿主，在不分区与分区两形状下：入口 `src/lib.rs` **逐字节相同**（唯一新增是 `add_crates.rs`）· `graft_plan.tsv` 的 `(cut, graft, full)` **逐字节相同** · 切口两端 id **逐字节相同** · `promote` 在分区形状下**仍能落地**。

### §M7.23 P3.2 第一刀落地：宿主生成树跳过被切出的子树（2026-10-06）

**改动**：`shape_decl` 拆成"**读一次**（`read_shape_declaration`，本来就是）+ **校验已读的**（`check_shape`）"，并新增 `ShapeDeclaration::cut_subtrees()`；管线在**渲染之前**读一次形状，把 `cut_out` 交给 `render_lib` ⇒ `render_nodes` 拿到这一组 `::` 分隔模块路径，`render_node` 命中即**直接返回**（连整棵子树一起跳过）。七个既有渲染钉子调用点补了那个新参数；新钉子 `a_subtree_handed_to_another_crate_is_not_rendered_here`（对照：不切时 `control`/`object`/`button` 三个模块都在；切 `control::object` 后父模块在、子模块与其下都不在；点名不存在的路径渲染结果**逐字节相同** ⇒ 精确匹配）。

**端到端实证（检出外副本 `/tmp/cut`，声明 `Crate::named("widgets").at(&[crate::control::object::SUBTREE])`）**：

| 观察 | 结果 |
| --- | --- |
| 生成树 `generated_lib.rs` | 只剩 `pub mod control` ✓ **没有** `pub mod object`/`pub mod button` |
| `pruning_manifest.tsv` | 仍是**全部面**（control + button + slider）✓ |
| generation 摘要 | 仍 `92ef0bd3…` ✓（**树没变**，变的只是"哪个 crate 编译它"） |
| `add-crates.lock` | `package_prefix cut` · `crate widgets subtrees=control::object faces=2 face_set=25404c23…` · `host faces=1` ✓ |
| 直接 `cargo build` 那棵树 | **编译失败**，`cannot find 'object' in 'control'` ✓ —— 正是"第三刀"的理由：切口表与 `assert_static_registration` 仍渲染在宿主里（生成树第 120/151 行仍引用被切出的面）|

⇒ 这一刀只做"不重复编译"这一半；**把切口表搬到 facade** 是下一刀（否则宿主自己编译不过——这是一次**编译期拒绝**，不是静默的双注册 ✓，方向是对的）。

### §M7.24 P3.2 第二刀（一半）：规划器 —— 幽灵 crate 会变成什么，先算清楚再写（2026-10-06）

`build_method::crate_plan`：**纯规划器，什么都不写**。对声明里的每个 crate 给出：包名 `<package_prefix>-<name>` · 目录（宿主包的**同级**）· **宿主自己的命名空间**（幽灵必须定义它，P3.3 正是为它引入的常量）· 每条子树之下**每份面文件**一项挂载（`module_path` / 宿主相对 `source` / `#[path]` 拼写）· 以及 `--remap-path-prefix` 要用的前缀对。

**拼写规则（可推导，不靠猜）**：`#[path]` 相对**内联模块所在目录**解析，而内联模块把自己的名字加进那个目录 ⇒ 深度 `d` 的挂载需要 `../`×`(d+1)` 走出内联目录、`src` 与幽灵包，再 `<host>/src/` 走进去 ⇒ 拼写恰好是 `<前缀><宿主相对源码>` ✓ ⇒ 把前缀映射为**空**，`file!()` 读起来就是宿主记录点名的那个拼写，身份因此逐字节不变（`b261fe0` 那道闸守着它）。

**挂载修不了的那一件事**（诚实边界）：幽灵只有内联容器模块与被挂载的叶子，**它上面没有 `control.rs`** ⇒ 够到碎片之外的 `crate::<模块>` 引用解析不到 ⇒ **点名拒绝**（文件:行 + 那条路径 + 两条出路：把定义搬进碎片，或**分区一个更大的子树**——"crate 是一棵子树"，更大的子树是合法答案 ✓）。**宏路径不算**（`crate::control_object!` 是宏调用，别名会发射进每个 crate 自己的生成树 ⇒ 与宿主一样拥有 ✓）。

**这一刀是载重的**：管线**先校验、再规划**（都在拿到 `pruning_rows` 之后）⇒ 一份本次构建变不成可用 crate 的声明**当场**得到可操作的拒绝。实测真宿主副本：

```
phase=add-crates
add_crates: `widgets` would not compile: control/object/button/button.rs:4 reaches `crate::control::ControlHandle`,
which lives outside the subtree(s) this crate claims (control::object) …
way forward: partition a node that contains the definition (a crate is a subtree, so a larger subtree is a
legal answer), or make the fragment self-contained by moving the item into it
```
（exit 1 ✓）——对比第一刀时"留下一棵没人编译的子树、由 rustc 报 `cannot find 'object' in 'control'`"，这是把失败提前到**能说清原因**的位置 ✓。

**钉子 3 条**：自足碎片 ⇒ 一个 crate、一次挂载、一对映射（并断言拼写＝前缀＋源码这条**不变量**，以及精确拼写 `../../../../host/src/…`）· 向外引用 ⇒ 点名拒绝（含"不打印以 `::` 结尾的路径"）· **同一个 crate 的两棵子树之间可以互相引用** ⇒ 两次挂载、两个不同前缀 ✓。

**顺带发现（本次不修，记录在案）**：`cli` 而**不开** `studio` 的组合下有构建告警（`studio/ui/graph/nodes.rs` 的 `unused variable: cache`）——没有门禁对这个组合跑 `-D warnings`（clippy 只跑默认与 all-features 两面，十面跑测器会构建 `toolchain+cli` 但**不因告警失败**）⇒ 一个小的门禁缺口。

**还差**：幽灵的三份文件内容（`lib.rs` 定义常量 + `build.rs` 跑管线 + `Cargo.toml` 依赖 + 承载 remap 的 `.cargo/config.toml`）· 渲染侧的"**只**编译这个 crate 的子树"模式（第一刀的反面）· 第三刀 facade · 验收四条。

**第二刀的另一半：三个已确认的设计点（动手前写清，免得半途改口径）**

1. **幽灵的 `src/lib.rs` 只有两项**：`pub const XIRANG_NAMESPACE: &str = "<宿主命名空间>";` + `include!(concat!(env!("OUT_DIR"), "/generated_lib.rs"));` —— 它自己**没有**面文件（面在宿主那边），生成树由它自己的构建脚本产出。
2. **幽灵的 `build.rs` 为宿主的清单跑管线**：`build_method::run_for(<宿主>/Cargo.toml, OUT_DIR, "<宿主命名空间>")`（`run_for` 已是公开入口 ✓），并打印 `cargo:rerun-if-changed=<宿主>/src`。也就是说**幽灵的生成树 = 按 `crate_plan` 的挂载清单渲染出来的树**：祖先节点只发**空的容器模块**（绝不挂载祖先的面文件——挂了就是同一个面在两个注册机里 ✗），子树下的叶子按 `PlannedMount.spelling` 挂载宿主的文件。
3. **remap 只能落在工作区根的 `.cargo/config.toml`**：rustflags 是**每次调用**的，不是按包生效的（cargo 的 config 发现基于当前目录而不是被构建的包 ⇒ 幽灵目录里的 `.cargo/config.toml` 只在从那个目录跑 cargo 时才生效 ✗）。因此规划器只**报告**需要的 `(from, to)` 对，写入方必须**合并**工作区根的配置而不是覆盖它 ⚠ —— 这条同时是 P3.5"发布形状的身份"边界的近亲：依赖方不会继承我们的 `config.toml`，所以**跨发布形状的身份今天仍无解**。

### §M7.25 幽灵渲染模式：只编译本 crate 的子树（祖先只发壳）（2026-10-06）

**输入**：`XIRANG_SHAPE_ONLY=<子树>[,<子树>]`（lexicon `SHAPE_ONLY_ENV`）。**渲染规则**（`renderer::tree::ShapeRender`，把"切出"与"只渲染"合成一个概念，`render_lib` 因此仍只多一个参数）：`only` 存在时，只保留**认领的子树**与它们**之上**的节点；之上的节点是**壳**——保留模块路径（好让认领的面经它解析）但**不挂载自己的文件**（挂载祖先的面文件会把那个面第二次注册进本 crate 的注册机），其余节点一律不发射。`only` 为空时退回第一刀的"整棵树减去交出去的子树"。

**端到端实测**（`XIRANG_SHAPE_ONLY=control::object` 跑真宿主副本）：

```
pub mod control {                 ← 壳（control/control.rs 的挂载数 = 0 ✓ 不会二次注册）
    pub mod object {              ← 认领的碎片
        #[path = "…/control/object/button/button.rs"] pub mod button;
        #[path = "…/control/object/slider/slider.rs"] pub mod slider;
```

**端到端抓到一个钉子看不见的真 bug**（值得记住的形状）：幽灵那次运行仍按**宿主**的声明算 `cut_out` ⇒ 它把恰好要编译的碎片跳过了 ✗✗ —— 钉子看不到，因为钉子自己传的是 `cut_out: &[] , only: Some(..)`（正是幽灵的意图 ✓），**错的在管线怎么构造这个组合**。修法：`crate_plan::cut_out_for(declaration, ghost)` —— 宿主交出去、**幽灵什么都不交** —— 并给它一条钉子（两种模式读同一份形状、读法不同）。

**还差**：幽灵的**挂载拼写**必须改用 `PlannedMount.spelling`（今天渲染器按 `relative_display` 发的是**绝对路径** ✓ 宿主可以，幽灵不行——它需要"走出去再走进宿主 src"的那种拼写才能保住 `file!()`）+ 三份文件内容（`lib.rs`/`build.rs`/`Cargo.toml`）+ facade。

### §M7.26 幽灵的挂载拼写：渲染器改用规划值（2026-10-06）

**改动**：`ShapeRender` 多一个字段 `mounts: &[(module path, #[path] spelling)]`；`render_node` 取"可移植绝对路径"之前先查表——**命中就用规划算出的拼写**（幽灵必须有它：`file!()` 报告的是拼写，而身份是 `hash(命名空间, 源码路径, 名字)` ⇒ **拼写就是身份**），未命中才回落可移植路径（宿主的情形 ✓；而身份错了不可能悄悄过——构建烤进一个 id、编译器算出另一个，§M7.15 那道闸当场红 ✓）。为此规划器改吃"**面清单**"（`(宿主相对源码, 模块路径)` 对）而不是 `PruningRow`：管线用 `static_faces`（**渲染之前**就算好 ✓）喂它，于是"规划 → 渲染用其产物 → 发布块只用行做声明校验"三段各就各位 ✓。钉子：渲染侧的新断言（幽灵模式**必须**发出规划拼写、且**不得**发出夹具自己的绝对路径）· 规划器测试改成从行推导同一份面清单 ✓。

**这一刀顺带暴露的下一个设计点（真问题，未做）**：一个被切出的面，`parent:` 往往指向**碎片之外**的祖先（`crate::panel::NODE_ID`）——而幽灵没有 `panel` 模块 ⇒ 按今天的可达性检查它会被**拒绝** ✓。出路有两条，都要选：① 幽灵的**祖先壳**里补发祖先的 `NODE_ID` **常量**（不注册 ✓——壳已经有那个模块，加一个从宿主计划里烤进去的 id 即可，于是 `crate::panel::NODE_ID` 在幽灵里解析得到 ✓）；② 让可达性检查把"祖先节点的身份常量"也算作可达，并据此实现 ①。**在 ① 落地之前，"可分区"的碎片只有那些连父级都只用 `root_node_id(XIRANG_NAMESPACE)` 写的树**——而真实宿主的写法几乎都不是这样，因此 ① 是让 P3.2 真正可用的关键一步。

### §M7.27 祖先的身份进壳 + 认领根面归碎片（2026-10-06）

**① 祖先的 `NODE_ID` 常量进壳**（§M7.26 记下的那个设计点）：被切出的面常写 `parent: crate::panel::NODE_ID`，而幽灵没有 `panel` 模块 ⇒ 规划器现在把**认领的真前缀里"是注册面"的那些**（连同它们的 id）算出来（`PlannedCrate.ancestors`），渲染器在壳模块里补发那一枚常量（`NodeId::from_raw([…16 字节…])`，字节取自宿主构建的计划 ⇒ 与宿主看见的是**同一个** id）。**可达性规则同时放宽一格**：`crate::<祖先>`（作为模块）与 `crate::<祖先>::NODE_ID` 可达，**别的仍拒绝**（定义在碎片之上的 trait 仍不可达——那是诚实的边界，钉子两半都钉住 ✓）。

**② 认领根面归碎片**（**这一刀自己抓到的第三个真 bug**）：过滤条件只认 `module.starts_with("<claim>::")` ⇒ **认领节点自己的面文件既没被挂载、也没被扫描** ✗✗ ⇒ 碎片的**根面没人编译**，而它对外引用**逃过了可达性检查** ✗。修法：`module == claim || module.starts_with("<claim>::")` ✓（宿主把整个模块交出去——它的渲染正是跳过该模块及其下的一切 ✓ ⇒ 根面必须归幽灵 ✓）。两个既有钉子因此要从 2 个挂载/2 个前缀改成 3 个（每个认领**含它自己的根面**、每个内联深度一个前缀 ✓），断言也跟着写清理由 ✓。

⇒ 三个真 bug 全部由"把功能一路做到端到端"暴露（`cut_out` 与 `only` 冲突 · 祖先身份缺失 · 认领根面漏掉），**没有一个是钉子先看见的**——钉子证明的是"我实现的这条规则在我想到的输入上成立"。

### §M7.28 幽灵的三份文件 + 工作区根的 remap 补丁；动词门禁改成读掩码（2026-10-06）

**四份产物**（规划器产出字节，仍然什么都不写）：`lib_rs` = 宿主的 `XIRANG_NAMESPACE` 常量 + `include!(OUT_DIR/generated_lib.rs)` · `build_rs` = `run_for(宿主 Cargo.toml, OUT_DIR, 宿主命名空间)` + 两条 `rerun-if-changed` + `XIRANG_SHAPE_ONLY=<认领>`（`unsafe { set_var }`，构建脚本此刻单线程）· `cargo_toml` = 宿主的 `[dependencies]`/`[build-dependencies]` **逐字照抄** + `publish = false` · `config_patch` = 需要**合并**进**工作区根** `.cargo/config.toml` 的 `--remap-path-prefix=<前缀>=` 条目（rustflag 是每次调用的、cargo 从当前目录找 config ⇒ 放包里只在从那个目录跑 cargo 时生效）。钉子 1 条覆盖四份产物（含"逐字照抄的依赖"与"remap 条目逐条拼出"）。

**门禁纠了我一处真违规**，而且纠得对：动词门禁把我生成串里的 `fn main` 当成裸动词声明 ✗ —— 根因是它扫**原文**而没屏蔽字符串字面量 ✗。仓库早就有唯一词法规则 `xirang_kernel::source::mask_non_code`（`size` 门禁就用它 ✓）⇒ 修法是**让动词门禁也读掩码**（一处改动 + 新钉子：字面量里的声明不算、同一句作为代码仍算 ✓）。这是"门禁必须建模自己的文本"那条教训的第二次现身（第一次是新写的 `namespace_source` 门禁抓到自己 ✗）。

### §M7.29 P3.4 的写盘一半：`xirang crates --check|--write`（2026-10-06）

**命令**：`xirang crates [<包目录>] [--check|--write]`（默认 `--check`）。`--check` 读声明与**已发布的记录**（缺记录时按名拒绝并给 `run xirang check first` 那条出路 ✓）算出计划并打印"会创建哪个包、在哪、几次挂载、几条 remap"；`--write` 真写下幽灵的三份文件，并把缺少的 `--remap-path-prefix` 条目**合并**进**工作区根**的 `.cargo/config.toml`（`workspace_root_of` 从包目录向上找最外层带 `[workspace]` 的 `Cargo.toml`，没有就退回包自己 ✓）。

**写盘的两条规则**（`build_method::crate_write`，`#[cfg(feature = "cli")]` 门控——唯一调用方是 CLI 那一面）：① **幂等且先比内容**（`write_if_changed`）：第二遍不改一个字节，因为拆分是要提交的东西；② **配置是合并、从不整体写下**：没有文件就建（含目录 ✓）、有 `[build]` 无 `rustflags` 就追加、有**单行**数组就插进缺少的条目并**原样保留其余每行每键**；**多行数组/别的语法一律点名拒绝**并给出该手写上去的条目 —— 盲目往用户维护的文件里合并，是一件工具吃掉别人配置的方式 ✓。

**钉子 3 条又是它们先抓到了两个真问题**：① 创建 `.cargo/config.toml` 时没先建 `.cargo/` 目录 ✗；② **文件先写、配置后合并** ⇒ 配置不可合并时幽灵包已经落盘 ✗（钉子断言了相反的顺序 ✓）。修法：配置**先**合并（拒绝必须让树保持原样——留下一次没有 remap 的拆分，等于留下一次身份已经悄悄搬家的拆分）。

**端到端实测**（自足夹具宿主）：`crates --check` ⇒ `crate fix-widgets at /tmp/fix-widgets (2 mounted file(s), 2 remap entr(ies))` + `preview only…` ✓；`--write` ⇒ `wrote 3 file(s) under /tmp/fix; workspace config updated` ✓，产物是 `/tmp/fix-widgets/{Cargo.toml,build.rs,src/lib.rs}` ✓，配置里两条 remap 各自对应一个内联深度 ✓。⚠️ 顺带如实记：那个夹具的 `check` 仍报一条诊断（`parent declaration cannot be resolved`，与写盘无关）——夹具的 `src/lib.rs` 是手写入口、没有 `host!()`，值得下一轮查清它是不是产品侧的第二个口径。

**未做**：`--revert`（删幽灵包 + 撤销配置条目）与 `--at`（指定输出位置）；CLI 的 `--help` 里已列出 `crates` 一行 ✓。

### §M7.30 §M7.29 里那条诊断的结案（2026-10-06）

上一节如实记下"夹具的 `check` 报 `parent declaration cannot be resolved`，下一轮查清"。**查明：那是夹具欠规格，不是产品侧的第二个口径** ✓。三条真实诊断按顺序是：① `parent-specific registration macro requires an explicit parent`（嵌套面必须显式写 `parent:` ✓）· ② `parent does not own a registry`（子面只能挂在**拥有注册机**的父级下 ⇒ 中间层要 `needs_registry: true` ✓）。补齐后同一棵树 **exit 0 + `xirang check: ok (fix)` + 3 face(s)** ✓。⇒ 记录的价值在于：**当时没有把"夹具没报绿"当成产品缺陷去改产品** ✓，而是先记下、再查；查的过程只动夹具、一处产品代码都没碰 ✓。

### §M7.31 P3.4 收尾：`--revert`（撤出时守两条线）（2026-10-06）

`xirang crates <包> --revert`：删掉本动作创建的幽灵包，并把它的 `--remap-path-prefix` 条目从工作区根 config 里减掉。**撤出时守的两条线比进入时更要紧**：① **本动作没有创建的目录永不删除** —— 生成的三份文件都带 `Generated by XiRang`（`crate_plan::GENERATED_MARKER`，一处住址：写下它的写入方与检查它的读取方共用 ✓），没有这一行的目录**点名拒绝**并原样留下（另一种做法是工具删掉别人的包，只因为它躺在幽灵会在的位置 ✗）；② **配置只减去自己加的那些条目** —— 单行数组里摘掉我们的条目，其余旗标/键/节**原样保留**；摘完只剩本动作那层壳的文件**删掉**而不是留空壳；多行数组再次**点名拒绝**并给出该手删的条目 ✓。

**钉子 3 条**（撤回恰好只撤自己的东西 + 第二次撤回什么都不改 · 没有标记的包被拒绝且原封不动 · 配置里不属于我们的东西全部保留 ✓）——其中一条**当场纠正了我的夹具**：占位字节里没有那枚标记 ⇒ 撤回把"自己写的包"认成了别人的 ✗（夹具补上标记后才对 ✓），顺带证明这条检查真的在按内容工作 ✓。

**端到端写-撤一轮**：`--write` ⇒ 幽灵包存在 ✓；`--revert` ⇒ `removed 1 generated package(s) under /tmp/fix; workspace config updated` ✓、幽灵包已删 ✓、配置里再无 `remap-path-prefix` ✓。

**`--at` 仍不做（有理由）**：挂载拼写现在是"从幽灵的内联目录走回宿主 `src`"的前缀 ⇒ 它**假定了幽灵是宿主的同级目录** ✓。要让 `--at <目录>` 成立，得先把 `spelling_for` 从"同级 + 深度"推广成"两个绝对路径之间的相对走法"——那是下一刀的第一件事，在那之前 CLI 不广告 `--at`（传它会得到 `unexpected argument` ✓，而不是一个悄悄换了身份的输出 ✗）。

### §M7.32 挂载拼写推广成"两个路径之间的相对走法"（2026-10-06）

`spelling_for` 过去按"**同级 + 内联深度**"算前缀（`../`×`(深度+1)` + `<宿主包名>/src/`）——它把"幽灵必须是宿主的同级目录"写进了公式 ✗。现在改成**真正的相对走法**：起点 = 幽灵的 `src` 加上该面之上每个**容器**模块一段（`inline_directory`），终点 = `<宿主>/src/<宿主相对源码>`，用 `relative_walk`（数共同前缀 ⇒ 每个剩余目录段一个 `..` ⇒ 再接剩余段）算出拼写；remap 前缀直接由"拼写减去源码"推导 ✓。

**为什么值得改**：① 它是 `--at <目录>` 的前置（§M7.31 记的下一刀第一件事 ✓）；② **facade 的位置计算需要同一套算术**（facade 也在别处 ✓）；③ 公式从此不再隐含"同级"这个未经声明的假设 ✓ —— 而唯一必须成立的东西仍是那条不变量：`file!()` 最终读起来就是宿主相对源码 ✓。

**钉子**：既有的"精确拼写 `../../../../host/src/…`"与"两棵子树 ⇒ 两个不同前缀"两条**原样通过** ✓（推广没有改变同级时的结果 ✓）；新增一条**非同级**用例：幽灵嵌深两层 ⇒ 相对走法的 `../` 正好多两个、且终点仍是宿主自己的源码 ✓。**写这条钉子时我又错了一次**：我最初断言"走法会点名它经过的目录（`nested/deep/`）" ✗ —— **向上走不会点名经过的目录** ✓，改成断言 `../` 的个数差 ✓。

### §M7.33 第三刀 facade：设计冻结（照它实现，不再改口径）（2026-10-06）

**为什么必须有它**（实测，不是推理）：切出 `control::object` 之后，宿主自己的生成树仍要写 `assert_static_registration(control::REGISTRATION.registry_rule, control::object::button::REGISTRATION)` 与 `StaticGraftCut::from_ids(crate::control::object::button::NODE_ID, …)`（生成树第 120/151 行），而 `control::object` 已经不由它编译 ⇒ `cargo build` 报 `cannot find 'object' in 'control'` ✓。facade 就是那个**两端都看得见**的 crate。

**它必须携带**：① `pub use <registry>::*;` ✓；② **`BUILTIN_STATIC_FACES` = 整棵树的并集**（宿主的面 + 每个幽灵的面 —— facade 依赖全部，因此它是唯一能给出完整表的那一个；宿主仍保留自己那部分，两者是同一棵树的两个视图，不矛盾 ✓）；③ **切口表**（类型化：`StaticGraftCut::from_ids(cut, graft, full)`；字符串写法 `new("path","name",full)` **不需要改写** ✓）；④ **两条 `assert_contract::<cut::__Preset, impl::__Parts>`** ✓。

**它必须不携带**：任何 `#[path]` 挂载（facade 一个面都不编译 ✓）与任何 `assert_static_identity`（那条断言属于**编译该面的 crate**——幽灵自己有 ✓，在 facade 里它只会指向不存在的模块 ✗）。

**改写规则（一条，覆盖一切）**：facade 的生成树里，**每一个** `crate::<模块路径>` 都要改写成 `<属主>::<模块路径>`——**包括宿主自己的面**（`crate::` 在 facade 里指的是 facade 自己 ✗）。属主来自规划器：每条认领（含其**根面**）与其祖先壳都属于那个幽灵的包 ✓，其余属于宿主 ✓。这正是维护者那句"**生成器只改写生成代码里的路径拼写**"的落地 ✓。

**三份文件**：`Cargo.toml`（依赖 = 宿主 + 每个幽灵 + **切口表达式里出现的每个实现 crate** + `xirang-toolchain`；`publish = false`）· `src/lib.rs`（宿主命名空间常量 + `include!(OUT_DIR/generated_lib.rs)`）· `build.rs`（`run_for(宿主清单, OUT_DIR, 宿主命名空间)` + `XIRANG_SHAPE_FACADE=1` + 两条 `rerun-if-changed`）✓。命名：`<package_prefix>-facade` ✓（发布形状是 P3.5 的问题 ✓）。

**渲染侧**：`ShapeRender` 多一个 `facade: bool` ⇒ `render_nodes` 一个模块都不发 ✓；`render_lib` 跳过别名/IDE 影子/身份断言 ✓，保留注册机再导出、静态面表、切口表与契约断言 ✓（后者按上面的规则改写 ✓）。

**验收（维护者四条 + 新增第五条）**：入口 `src/lib.rs` 逐字节相同 · `graft_plan.tsv` 的 `(cut, graft, full)` 逐字节相同 · 切口两端 id 逐字节相同 · `promote` 在分区形状下仍能落地 · **⑤ 分区后的树真的编译得过**（今天不能，实测见本段开头 ✓：这正是 facade 存在的理由）。

**实现顺序**（下一轮照此做，不重排）：① 规划器加"属主映射 + facade 三份产物"（并给钉子）→ ② 渲染侧加 `facade` 模式与改写（钉子：一条切口两端被改成两个包、字符串写法不动、没有 `#[path]`、没有身份断言）→ ③ `write_partition` 连 facade 一起写（CLI 的 `--check/--write/--revert` 一并覆盖）→ ④ 端到端：真宿主两形状各 `cargo build` 一次 + 四条逐字节比对。

### §M7.34 第三刀 facade 的渲染侧落地（2026-10-06）

按 §M7.33 的规格做了**第②步**（也是这一刀最难的一半）：`XIRANG_SHAPE_FACADE=1` 驱动一个新渲染模式。**它做什么**：`render_nodes` 一个模块都不发（facade 不编译任何面 ✓）、别名与 IDE 影子一并跳过 ✓、`assert_static_identity` 全部跳过（那条断言属于**编译该面**的 crate ✓）；保留注册机再导出、`BUILTIN_STATIC_FACES`、切口表与契约断言 ✓。**改写**：新增 `renderer::owners`（`owned()` 一条规则）——facade 里**每个** `crate::<模块>` 与裸模块路径都改写成 `<属主>::<模块>`，属主来自规划（认领、它的根面、它的祖先壳都归那个幽灵；其余归宿主，**包括宿主自己的面**，因为 facade 里的 `crate::` 指的是 facade 自己 ✓）；外部实现 crate 原样通过 ✓；非 facade 模式**逐字节不变** ✓。属主表由管线从 `planned` + 面清单构出（`BTreeMap` 让幽灵覆盖宿主的同名项 ✓）。

**端到端（真夹具 `/tmp/fix`）**：facade 模式下 `pub mod`/`#[path]` **0** 处 ✓、`assert_static_identity` **0** 处 ✓、跨 crate 部分保留 ✓、三行注册断言全部改写成 `fix_widgets::…` ✓（回退分支 `::xirang_toolchain::…` 未被动 ✓）。

**这一刀又抓到一个真 bug（第 7 个）**：`assert_static_registration` 有**两个**参数都点名模块（父级注册规则 + 被断言的面），我第一版只改写了第二个 ✗ ⇒ 端到端跑出一行裸的 `panel::REGISTRATION.registry_rule` 站在一个不编译 `panel` 的 crate 里 ✓。修完三行全对 ✓。

**顺带踩到尺寸棘轮**：`pass.rs` 加到 **656** 行（上限 600）✗ ⇒ 按仓库规矩**按职责拆同级模块**（不加基线）：`renderer/owners.rs` + `renderer/owners_tests.rs` 收走 `owned`/`face_module` 与那条钉子 ✓，`pass.rs` 回到 600 以内 ✓。

**十面的一次环境性红（如实记）**：第一次跑，`toolchain+dev-supervisor,studio` 面里 `new_project_and_explicit_root_face_compile` 红，原因是 **`rustc` 编译第三方 `syn` 时 SIGSEGV**（不是 XiRang 的错误）✗；单跑该条**转绿** ✓，重跑十面**全绿** ✓ ⇒ 判**环境性失败**（并行资源压力下的 rustc 崩溃），非本批回归 ✓。

### §M7.35 第三刀 facade：规划与写盘落地（§M7.33 的第①③步）（2026-10-06）

**规划器**（新模块 `build_method::crate_facade`，随 `cli` 特性门控；`crate_plan.rs` 已 540 行，按职责另立同级模块 ✓）：`plan_facade(host_root, package_prefix, namespace, host_package, &planned) -> Result<Option<PlannedFacade>, String>` 给出 facade 的三份产物 —— `Cargo.toml`（**宿主的依赖表逐字照抄** + 同级 `path` 依赖 `host`/每个幽灵 + `publish = false`）· `src/lib.rs`（宿主命名空间常量 + `include!(OUT_DIR/generated_lib.rs)`）· `build.rs`（`run_for(宿主清单, OUT_DIR, 宿主命名空间)` + **`XIRANG_SHAPE_FACADE=1`** + 宿主 `src` 与声明各一条 `rerun-if-changed`）✓。

**两个边界（都钉住了）**：① 声明没交出任何东西 ⇒ **返回 `None`**（没有跨 crate 的东西要承载的 facade 是为说空话而存在的 crate ✓）；② facade 包名与宿主包名相同 ⇒ **点名拒绝**（`package_prefix` 是那个旋钮 ✓，一个 crate 不能依赖它自己 ✓）。

**一处简化（wiring 时发现）**：**实现 crate 不需要参数** ✓ —— 类型化切口要求实现 crate 是宿主的依赖，而宿主的依赖表是逐字照抄的 ⇒ 可能出现在切口表达式里的 crate 正好就是照抄带来的那些 ✓（原设计要多传一份 `implementation_crates`，删掉后少一个会漂的输入 ✓）。

**写盘**：`write_partition`/`revert_partition` 多收一个 `Option<&PlannedFacade>` ⇒ **两个包走同一个写入方**（三份文件同一套逻辑 ✓），撤回的标记检查同样覆盖 facade ✓。属主表提取成 `crate_plan::owners`（**一处实现、两个调用方**：渲染 pass 与 facade 规划器 ✓）。

**端到端（夹具 `/tmp/fix`）**：`--check` ⇒ `crate fix-widgets at /tmp/fix-widgets (2 mounted file(s), 2 remap entr(ies))` + `facade fix-facade at /tmp/fix-facade (dependencies: fix, fix-widgets)` ✓；`--write` ⇒ **`wrote 6 file(s)`**（2 包 × 3 文件 ✓），facade 的清单确实是"宿主依赖逐字 + `fix`/`fix-widgets` 两条 path"✓，`build.rs` 带 `XIRANG_SHAPE_FACADE` ✓ 而幽灵带 `SHAPE_ONLY` ✓；`--revert` ⇒ `removed 2 generated package(s)` ✓、两个目录都没了、配置里再无 remap ✓。

**两处自纠（都由钉子/告警当场抓出）**：① 我用**行区间**删旧 `revert_partition` 主体时，把新主体的尾部一起切掉了 ✗ ⇒ 三条写盘钉子立刻红并点名"什么都没删"✓（按行删除的锚点不可靠，改为显式重写整段 ✓）；② `PlannedFacade::namespace` **没有读者**（已经烤进那两份文件）⇒ 按仓库规矩**删字段**而不是留着当占位 ✓。

**还差（下一轮做）**：**能编译的端到端** —— 要一棵真正的**工作区形状**夹具（成员＝宿主＋幽灵＋facade），因为 `rustflags` 落在工作区根、而 cargo 从当前目录找 config：夹具没有工作区时配置会落在包目录里，而幽灵是从**它自己的目录**被构建的 ⇒ remap 不生效、身份断言会响 ✓（这正是"发布形状的身份"那条边界的近亲 ✓）。随后才是验收四条（入口逐字节相同 · `graft_plan.tsv` 三列逐字节相同 · 切口两端 id 逐字节相同 · `promote` 仍能落地）。

### §M7.36 第一次"分区后的树真能 cargo build"：三个真缺陷，两个已修（2026-10-06）

**夹具**：`/tmp/ws2` —— 真**工作区形状**（`[workspace] members = ["app","app-widgets","app-facade"]`），宿主 `app` 是自足碎片（`panel` 根面 → `frame`（有注册机）→ `widget` 叶子，**没有任何跨碎片引用** ✓），声明 `Crate::named("widgets").at(&[crate::panel::frame::SUBTREE])` ✓。**注意**：真宿主 `examples/control-button` 的 `control::object` 会被规划器**点名拒绝**（`button.rs` 用 `crate::control::ControlHandle`，碎片不自足 ✓）——这是设计里的规则 ✓，因此"能编译"的验证必须用自足碎片 ✓。

**第 8 个真缺陷（已修）**：生成的 `build.rs`（幽灵与 facade **都是**）把**清单路径**传给了 `run_for`，而它收的是**包根** ✗ ⇒ 构建报 `<root>/Cargo.toml/src is not a source directory` ✓。修：两处生成器改传包根，两条钉子同时**反向断言**清单路径不再出现 ✓。

**第 9 个真缺陷（已修）**：分区后**每个 crate 只该为自己编译的面发表** ✗ —— 宿主仍为被切走的面发注册断言、幽灵仍为宿主的面发断言 ⇒ 实测 `error[E0433]: cannot find 'frame' in 'panel'`（宿主）+ 幽灵侧同理 ✓。修：管线按模式过滤面表 —— **facade 全留**（并集 ✓）· **幽灵只留自己认领之下的**（`under_a_claim(module, only)` ✓）· **宿主只留不在任何认领之下的** ✓。`whole()`（无声明）时是恒等 ✓，因此既有钉子不受影响 ✓。

**第 10 个真缺陷（未修，下一轮第一件事）**：**容器面的挂载拼写少算一层** ✗ —— 有子节点的面被渲染成"内联模块 + 在它**自己**的目录里挂载自己" ⇒ 该挂载的 `#[path]` 是相对 `<ghost>/src/<容器路径>/<面名>/` 的，而 `inline_directory` 只按**容器**段算（少一层 ✓）⇒ 实测 `couldn't read …/out/panel/frame/../../../app/src/panel/frame/frame.rs` ✓（差一个 `../` ✓）。修法：`plan` 需要知道这个面**是不是容器**（可由面清单推出：存在别的面的模块以 `它::` 开头 ✓）⇒ `spelling_for` 多一个 `container: bool`，容器情形多走一层 ✓ + 一条钉子（容器面的拼写比叶子多一个 `../` ✓）。

**第 11 个真缺陷（未修，紧随其后）**：宿主侧 `error[E0433]: cannot find 'registry_rule' in 'super'` ✗ —— 过滤掉 `panel::frame` 之后，注册断言仍在某个内联模块里找 `super` 的注册规则 ✓。下一轮先读那一段生成代码（`…/out/generated_lib.rs` 第 ~104 行）再定修法；候选是"断言只在该面**自己的容器**作用域里发"或"父规则取不到时退回 `ANY` 并明写" ✓。

**实测状态**：`check`/`--write`/`--revert` 在真工作区里全部正常 ✓（`wrote 6 file(s)`、`removed 2 generated package(s)` ✓），宿主的**非分区**构建绿（25.48 s ✓），分区构建卡在上面两条 ✓。门禁（fmt · workspace 36 · clippy 两面 · conventions）在后一条修复后全绿 ✓。

### §M7.37 挂载改成**绝对路径**：相对走法这条路本身不成立（2026-10-06）

**第一次真正编译分区树时量到的**（§M7.36 的 ⑩）：`couldn't read <out>/panel/frame/../../../app/src/panel/frame/frame.rs` ✓。我原先判成"容器面少算一层" ✗ —— 算一下就知道不对：`#[path]` 实际相对**生成树所在的 `OUT_DIR`** 解析（cargo 决定的、规划时看不见的目录 ✓），而那个文件离它有**七层** ✓。

⇒ **根因**：拼写的起点用的是**幽灵的包目录** ✗，而 `#[path]` 的解析基准是 `OUT_DIR` ✓ ⇒ **相对走法在原理上不成立** ✓（不是差一层，是差一个不可知的位置 ✓）。前几轮夹具没暴露它，是因为它们只跑 `crates --write`（写文件 ✓），**从没编译过** ✓。

**修法**：挂载用宿主的**绝对路径** ✓（`spelling_for(host_dir, source) = <宿主>/src/<宿主相对源码>` ✓），remap 用**一条** `<宿主>/src/` → 空 ✓。**不变量仍然成立**（拼写＝前缀＋源码 ✓）⇒ `file!()` 读起来仍是记录点名的那个相对源码、身份逐字节不变 ✓。这不是权宜：**宿主自己的生成树早就这么发**（它给 `registry_rule` 的是 `#[path = "/home/nich/…/src/control/registry_rule/registry_rule.rs"]` ✓）。

**顺带简化**：remap 从"每个内联深度一条"变成"**每个宿主一条**" ✓（夹具实测 `1 remap entr(ies)`，此前是 2 ✓）；`relative_walk` 只剩创作面 CLI 用（facade 写同级 path 依赖 ✓）⇒ 移到 `#[cfg(feature = "cli")]` 之下 ✓；`inline_directory` 删除 ✓。**`--at` 因此自动成立**（拼写不再依赖幽灵的位置 ✓）。钉子相应改成**性质断言**：拼写是绝对路径且以宿主相对源码结尾 ✓ · remap 只有一条 ✓ · **幽灵放在哪里都拼出同一个字符串** ✓（原"位置影响拼写"那条的前提已作废 ✓）；配置补丁那条改成形状断言（宿主根是临时目录，精确串不可预知 ✓）。

**实测**：真工作区夹具里 `--revert` ⇒ `--write` ⇒ `wrote 6 file(s); workspace config updated`、`1 remap entr(ies)` ✓；随后 `cargo build` 的分区树里 **宿主 `app` 与幽灵 `app-widgets` 都不再报原来的错** ✓，剩下**全在 facade**：① 注册断言把父级规则指向一个**祖先壳**的 `REGISTRATION` ✗（壳只携带 `NODE_ID` ✓ ⇒ 应当回退成 `ANY` ✓）；② 一行裸的 `panel::frame::REGISTRATION` 没被改写成幽灵 crate ✗（§M7.34 的 `owned()` 没覆盖这一处 ✓）。这两条是下一轮的第一件事 ✓。

### §M7.38 **分区后的树真的能 `cargo build` 了**（2026-10-06，第 ⑫ 个真缺陷收尾）

**里程碑**：真工作区形状夹具（`[workspace] members = ["app","app-widgets","app-facade"]`，声明认领 `panel::frame`）里，**三个 crate 全部编译通过** ✓ —— 宿主 `app` · 幽灵 `app-widgets` · facade `app-facade`，强制重编（`touch` 三个 `src/lib.rs`）后明确看到三条 `Compiling` + `Finished` ✓✓。这是 P3.2 从第一刀以来一直缺的那块：**"分区"不再只是"文件写对了"，而是"真的编得过"** ✓。

**收尾的两个真缺陷（都在 facade，同族：跨 crate 引用漏改写）**：① 属主表把**祖先壳**算成了幽灵的 ✗ ⇒ facade 发 `app_widgets::panel::REGISTRATION`，而**壳只携带 `NODE_ID`** ⇒ `cannot find value REGISTRATION in module app_widgets::panel` ✓。修法：**祖先壳不进属主表**（壳是幽灵搭的，但那个模块上的**面**是宿主编译的 ⇒ 面级别的项必须用宿主的 crate 拼 ✓）；两者其实各答对一半 —— `NODE_ID` 幽灵有、`REGISTRATION` 宿主有 ⇒ 现在按"面级别项归宿主"统一 ✓。② **`owned()` 的模块提取对 `<模块>::REGISTRATION.registry_rule` 失效** ✗（按最后一个 `::` 切得到 `panel::frame::REGISTRATION`，一个谁都不是的属主 ⇒ 原样发出 ⇒ `cannot find module or crate panel` ✓）。修法：拿**整条**表达式去匹配属主表、取最长的 `::` 边界前缀 ✓。③ 顺带：`registrations()` 那段快照也点名模块 ⇒ 同样走 `owned()` ✓；父级规则抽成 `owners::parent_rule`（"父级是壳 ⇒ `ANY`" ✓），它同时把 `pass.rs` 从 605 行压回 600 以内（尺寸棘轮 ✓）。

**为什么这几条一直躲着**：它们只在**facade**（依赖三个 crate 的那一个）里现形 ✓，而此前每一轮都停在"宿主与幽灵过了"就以为是拼写问题 ✓。⇒ 教训已记：**凡是"跨 crate 引用"，要问"这个名字在**哪个** crate 里存在"** ✓。

**状态**：`check`/`crates --write/--revert` 在真工作区里全绿 ✓；分区树构建绿 ✓；非分区宿主构建绿 ✓。**下一步**：验收四条（入口逐字节相同 · `graft_plan.tsv` 三列逐字节相同 · 切口两端 id 逐字节相同 · `promote` 在分区形状下仍能落地）→ P3.5 → P3.6 → P4。

### §M7.39 P3.2 第三刀的**验收四条**：全部成立；路上又逼出三个真缺陷（2026-10-07）

§M7.38 说"下一步：验收四条"。真去做的时候，验收本身把 **⑬⑭⑮** 三个真缺陷逼了出来 —— 前两个卡在"含 graft 的宿主"这一半（§M7.36 那个夹具**根本没有 graft**，所以它们一直躲着），第三个卡在 `promote` 自己身上。

**装置（两棵树，都在 `/tmp`，都可按本节步骤复现）**：
- `/tmp/p32acc` —— **手写**宿主：面 `panel` / `panel::frame` / `panel::frame::widget`，声明认领 `panel::frame`，类型化切口 `cut(crate::panel::frame::widget::NODE_ID) graft(fast_widget::fast::NODE_ID)`（实现 crate 的面在 `src/fast/fast.rs` ⇒ 模块 `fast`）。
- `/tmp/p32promote` —— **脚手架**宿主：`new_project {faces:[panel, frame(parent root/panel), widget(parent root/panel/frame)]}` 生成（面因此带 `// generated-by=XiRang`，这是 `promote` 能落地的前提），声明认领 `panel::object::frame`，同一型切口 + 一条 `.xirang/external-grafts/fast/graft.plan` 记录。
两棵树都先 `check` → 加声明 → 再 `check` → `crates --write` → 手工把两个新包加进工作区 `members`（**成员物化仍是 P3.5 的事** ✓）→ `cargo build -p app-facade`。

**① 入口逐字节相同**（`sha256`，`app/src/**` 6 份 + `Cargo.toml` + `build.rs`，`--write` 前后各一遍）：**逐字节相同** ✓；`app/` 里**唯一新增的文件是 `add_crates.rs`**（声明本体）✓ —— 分区没有碰宿主一个字节。

**② `graft_plan.tsv` 的 `(cut, graft, full)` 逐字节相同**：**四个形状**（不分区宿主 / 分区宿主 / 幽灵 / facade）的 `graft_plan.tsv` **整份文件逐字节相同**（连 `line`/`column` 都相同，因为声明住在同一个文件里）✓：

```
# cut	graft	full	line	column
crate::panel::frame::widget::NODE_ID	fast_widget::fast::NODE_ID	false	7	8
```

**③ 切口两端 id 逐字节相同**：以 `BUILTIN_STATIC_FACES` 的 `(id, parent, owns_registry)` 三元组为准 —— **facade 的 3 条与不分区宿主的 3 条逐字节相等**（facade 携带并集 ✓），而切割端 `panel::frame::widget` 的烤入身份在 **幽灵树里与基线树里逐字节相同**：

```
assert_static_identity(crate::panel::frame::widget::NODE_ID, NodeId::from_raw([237, 5, 168, 59, 255, 18, 166, 220, 215, 79, 157, 26, 106, 221, 20, 231]))
```

替换端 `fast_widget::fast::NODE_ID` 住在一个分区分不到的 crate 里（`fast-widget` 不是被切的树），两侧表达式逐字相同 ✓。幽灵与宿主各自只为自己编译的面发身份断言（facade 一条都不发 —— 那是 §M7.33 的设计 ✓）。

**④ `promote` 在分区形状下仍能落地**（脚手架宿主）：`apply {action:"promote", selector:"fast", confirm:true}` 先给出预览、`apply:true` 后**落地** ✓ —— 入口第 7 行改成自嫁接 `cut(<同一 id>) graft(<同一 id>)`（条目数 1 → 1 ✓）、面文件改写成外部实现的字段（`kind: WidgetFast` + `flow: FlowContract::new(…, "panel.render.v1", …)`）✓、记录移到 `.xirang/trash/external-grafts/fast-<stamp>` ✓；**落地后分区树仍 `cargo build` 通过** ✓。

**⑤ 分区后的树真的编译得过**（§M7.33 的第五条）：**两棵树各一次**，宿主 + 幽灵 + facade 三个 crate 全绿 ✓（`touch` 三个 `src/lib.rs` 强制重编，看到三条 `Compiling` + `Finished` ✓）。

#### 逼出来的三个真缺陷（⑬⑭⑮）

**⑬ 宿主为"已经交出去的切口面"发 graft 表** ✗ —— 上一轮那个夹具的宿主**没有 graft**（`fast-widget` 是后来才加的），所以这一条一直躲着。含 graft 的宿主一编译就是：

```
error[E0433]: cannot find `frame` in `panel`
  --> build/app-.../out/generated_lib.rs:157
  StaticGraftCut::from_ids(crate::panel::frame::widget::NODE_ID, fast_widget::fast::NODE_ID, false),
```

§M7.36 的 ⑨ 只过滤了**面表**，没过滤**切口表**。修法：条目归**编译该切口面的 crate** —— 新增 `renderer::owners::names_a_compiled_module`，管线里每个 graft 条目先问"这个切口面在本 crate 编译的 `static_faces` 里吗"，不在就整条不发（宿主因此一条不发、幽灵发它认领的那条、facade 因为并集全发 ✓）。**这条同时是"窄而会拒绝"的正面例子**：宿主不是"少发一条数据"，而是**根本不该点名一个它没有的模块** ✓。

**⑭ 可达性检查把前言里的名字当成"够到碎片之外"** ✗ —— 这条更狠：**脚手架建出来的宿主完全无法分区**。两个错叠在一起：① `use crate::{NoParts, NoPreset};` 的**分组导入**在扫描里得到一个**空头**（`{` 不在标识符字符集里）⇒ 每个生成的注册面都被拒；② 就算过了分组那一关，`crate::RegistrationRule` 也不在允许清单里。而**幽灵其实解析得到它们**：生成树的前言是 `pub use <registry_core>::*;`，宿主与幽灵**一字不差**。修法两条：分组导入**逐个名字**受检；判定改成**可判定**的那条 —— **首段不是本包顶层模块的名字就是 crate 根上的条目**（顶层模块＝`src/` 下的目录或与 `lib.rs` 同级的 `.rs`，就是 Rust 的模块↔路径映射 ✓），crate 根条目一律可达（前提是前言相同）。剩下**诚实的残留**：宿主在**自己 crate 根**上定义（且不是模块）的条目，幽灵确实没有 —— 那件事用文本判等于猜"什么算一个 Rust 条目"，因此**交给 rustc 说一次**，并在代码注释里写明，不假装判得了 ✓。这条改动的直接证据：`promote` 落地的面写的是 `crate::FlowContract` / `crate::ContractId` —— 一份"允许名字清单"会被我们自己产物的下一个字段再打破一次 ✗。

**⑮ `promote` 找不到本生成器写出的面文件版式** ✗ —— `external_file` 只找 `src/<模块>.rs` 与 `src/<模块>/mod.rs`，而**脚手架与 `apply add` 把面放在 `src/<模块目录>/<叶名>.rs`**（`panel::object::frame` ⇒ `src/panel/object/frame/frame.rs`，生成树也正是从那里挂载它）⇒ `promote` 对**本工具自己创建的每一个面**都拒绝，理由是"既没有……也没有……"，而那个文件就在那里、只是深了一层 ✓。修法：候选表加中间那一形（`src/<relative>/<leaf>.rs`），平坦形仍排第一（同时带两种拼写的 crate 答案不变），拒绝文案把三个候选都点名 ✓。

**尺寸棘轮（顺带）**：`pass.rs` 因为 ⑬ 的守卫到了 **605** code lines ✗ ⇒ 按职责把 `StaticGraftCut` 构造式的拼写移到 `renderer/owners.rs`（`graft_constructor`）——它与 `owned()` 是同一个关注点（"这个 crate 怎么拼这条表达式"）✓，`pass.rs` 回到 600 以内 ✓。

**新钉子（4 条，都会红）**：`a_crate_emits_only_the_graft_entries_it_compiles`（**反证做过**：把守卫改成 `if false` ⇒ 当场红、恢复 ⇒ 绿 ✓）· `the_scaffolds_own_imports_do_not_block_a_partition`（夹具就是脚手架的产物形状，含 `promote` 落地的 `FlowContract`/`ContractId` 两行）· `a_one_segment_module_path_is_still_checked`（规则的另一半：一段 ≠ 不是模块，`crate::panel` 照样拒 ✓）· `the_generators_own_face_layout_is_found`（三种版式 + 平坦优先）✓。

**门禁**：`fmt --check` ✓ · `cargo test --workspace`（36 target 全绿）✓ · clippy **两面** `-D warnings` exit 0 ✓ · conventions（含 size）✓ · `tools/xirang-publish --check-table` ✓ · **十面 `tools/xirang-test` 补跑**（§M7.38 那次漏跑的正是它）：见提交说明。

**状态**：P3.2 第三刀的**验收四条全部成立** ✓，分区形状在**手写宿主**与**脚手架宿主**上都真的编译 ✓，`promote` 在分区形状上真的落地 ✓。**下一步**：P3.5 发布物化 + CI 两形状（本节这套"两棵树 + 四条"就是那条 CI 的雏形；**成员清单物化**仍是它的活）。

### §M7.40 P3.5 第一件：**成员清单物化** + 配置落在"真正会被读到的那个目录"（2026-10-07）

§M7.39 记了两次"手工把生成包加进 `members`"。这一节把那一步交给工具，顺带逼出 **⑯**。

**新模块 `build_method::crate_members`**（随 `cli` 门控）：把生成的包物化进**工作区清单**的 `members`，撤出时按**逐字节的带引号拼写**收回。四条规矩与工作区配置同一套：**合并、从不整体重写**（其余每一行、每一个键原样保留——注释、`[workspace.package]`、别人的成员条目、`members = ["crates/*"]` 这样的 glob 都不动）· **幂等**（第二遍发现每一条都在就什么都不写）· **只收回自己写下的条目**（按精确拼写匹配，别人的条目与 glob 留着）· **读不懂的形状点名拒绝**（`members` 不是一串带引号字符串、或数组不闭合 ⇒ 拒绝并点名 `Cargo.toml:<行>`，一个字节都不写）。单行与**一行一条目**两种写法都支持（后者是 cargo/rustfmt 写长清单的方式，只重建条目行、保留键行与结束括号的原文与缩进）。

**写入顺序是有讲究的**：成员清单**先算、后写**——拒绝必须让树保持原样，而"清单点名了不存在的包"正是 cargo 拒绝加载的状态（`failed to load manifest for workspace member`）。因此：配置合并（拒绝点）→ 算出清单新文本（拒绝点）→ 删旧包 → 写三份文件 → **最后**写清单。撤出时反过来：清单**先在内存里剥**（拒绝点），再删包，最后写清单 ✓。

**⑯（真缺陷，实测）配置落在了 cargo 永远不会读的地方**：宿主**不在**任何工作区里时（`new_project` 生成的宿主就是这一种——它自带 `[workspace]` 表），`workspace_root_of` 返回宿主包自己 ⇒ `.cargo/config.toml` 写进 `<宿主>/.cargo/` ⇒ 而生成的包是宿主的**同级**，cargo 从**被调用的目录**及其祖先找配置，宿主里面的那份在构建同级包时永远不会被读到 ⇒ 实测分区树 **exit 101**：每个被挂载的面都在 `assert_static_identity` 上炸（`the failure occurred here` + `static identity failed`）。**修法**：配置的位置改成"**这次拆分的构建区**"——有外层工作区就是那个工作区根，没有就是**宿主与它的同级包共同拥有的父目录** ✓。写在父目录不是漏进无关目录：拆分把它的包**放在**那里，而每条 rustflags 都是作用域限于一个宿主绝对路径的 `--remap-path-prefix`，对别的东西是空操作；那里已有的配置照旧只合并不替换 ✓。

**实测（两组夹具）**：
- **工作区形状** `/tmp/p32ws`（用户手写的工作区清单：注释 + 多行 `members` + `[workspace.package]`）：`crates --write` ⇒ 回复尾 `; workspace members updated`，清单变成 `members = ["app","fast-widget","app-facade","app-widgets"]`（**缩进、注释、别的表一字未动** ✓），随后 `cargo build -p app-facade` 从**工作区根**一次通过——**没有手工编辑** ✓；再跑一次 `--write`：清单与配置**逐字节相同**（幂等 ✓）；`--revert` ⇒ 清单**逐字节回到原样**、两个包目录与 `.cargo/config.toml` 都消失、`cargo metadata` 仍能加载 ✓。
- **独立形状** `/tmp/p32sol2`（宿主自带 `[workspace]` 表、上面没有任何工作区）：`--write` ⇒ 配置落在 `/tmp/p32sol2/.cargo/config.toml`（父目录 ✓）、回复尾 `; no enclosing workspace, so no member list` ✓；`cargo build` 在 facade 自己的目录里 ⇒ 三个 crate 全绿 ✓（**⑯ 之前这里是 exit 101** ✗✓）。

**钉子 8 条**：`crate_members` 7 条（单行合并 · 多行保形 · 没有清单时在表头下加一行 · 收回后逐字节还原/键本身是本动作加的就整键去掉 · 别人的条目与 glob 不动 · 三种读不懂的形状都拒绝且不写一个字节 · 拼写是相对路径、排序去重）+ `crate_write` 1 条（**有工作区才编辑清单**：写入 `members_changed` 为真、撤回后清单逐字节还原；`workspace: None` 时清单一个字都不动）✓。

**仍差（P3.5 余下）**：`--release` 物化真实包（自包含副本 + `[package.metadata.xirang] shape` + 身份/记录/租约迁移并显式列出哪些记录失效）· **CI 两个形状都构建都测**。

### §M7.41 P3.5 第二件：**CI 两个形状都构建都测** = 一道可复跑的门禁（2026-10-07）

新工具 `tools/xirang-partition-rehearsal`（POSIX sh，与 `tools/xirang-external-rehearsal` 同一套写法：`XIRANG_REHEARSAL_CARGO_FLAGS` 控制 cargo 标志、默认 `--offline`，`mktemp -d` + `trap` 清理），接进 `.github/workflows/ci.yml` 的 `verify` 任务（Linux + stable 那一个单元，与外部路径演练并列）。**它存在的理由写在脚本头上**：这个特性被手工验证了三个批次，而它每一个缺陷都是靠**把整件事完整做一遍**才发现的——不可能相对的挂载拼写、facade 必须改写的两处跨 crate 引用、宿主为交出去的切口面发 graft 表、可达性检查拒绝脚手架自己的导入、`promote` 找不到的实现文件版式、写在 cargo 永远不会读的地方的 remap（§M7.37–§M7.40）。手工跑什么都不留下。

**三条腿**（三种形状是三条代码路径）：
1. **工作区形状**——宿主是某个工作区的成员：先 `check` 取不分区的基线，再取**分区后的三个形状**（宿主 / `XIRANG_SHAPE_ONLY=panel::frame` / `XIRANG_SHAPE_FACADE=1`），然后 `crates --write`、从**工作区根** `cargo build -p app-facade`、跑一次幂等写、再 `--revert`；四条验收事实逐字节断言（① 宿主 `src/**`+清单+build.rs 的 sha256 前后相同 ② `graft_plan.tsv` 在四个形状里逐字节相同 ③ facade 的 `StaticFace::new` 三元组与基线逐行相同、切割端的身份断言与幽灵树相同、**且各自只为自己的面断言**——宿主不为交出去的面断言、幽灵不为留下的面断言 ④ 见第 3 条腿），另加 `members` 物化、注释/别的表未被吃掉、revert 后清单**逐字节**还原、`cargo metadata` 仍能加载。
2. **独立形状**——宿主之上没有任何工作区：`--write` 后 remap 必须在**宿主与其同级包共享的父目录**（⑯ 的回归），`cargo build` 在 facade 自己的目录里必须通过。
3. **在分区树上 `promote`**——用 `new_project` 建一个脚手架宿主（它写的面归创作执行器所有，这正是 `promote` 能改写的前提），补上 graft 计划、实现依赖与声明，分区并构建，然后写一条记录（**目标身份用 `xirang explain` 读出，脚本不硬编码哈希**），先预览、再落盘，断言：入口改成自嫁接、记录进 `trash/`、**落地后仍能构建**。

**它第一次跑就抓到 ⑰（真缺陷）**：leg 3 的**预览**被拒——`promote` 读命名空间时问的是**工作目录**，而预览时工作目录是临时目录里的一次性副本，宿主清单里相对的 `path` 依赖（`../fast-widget`，也就是分区后宿主**必然**有的那条实现依赖）在那个副本旁边什么都不存在 ⇒ `cargo metadata` 说不出包名（`cannot learn the identity namespace of /tmp/xirang-toolchain-preview-…: failed to read /tmp/fast-widget/Cargo.toml`）。**落盘那一半是好的**（它在项目里跑），因此这个缺陷**只在预览里现形**——手工那一次之所以"看起来能预览"，是因为 `/tmp/fast-widget` 恰好是先前实验的残留 ✗（假绿，正是本门禁存在的理由）。修法：与其它动作一致，命名空间取自**项目根**（`namespace(root)`），副本只当工作目录用 ✓。钉子：`a_preview_learns_the_namespace_from_the_project_not_the_copy`（夹具里兄弟 crate 在项目旁边解析得到、在副本旁边解析不到；断言拒绝**不是**"学不到命名空间"，而是后面那一步关于记录的拒绝）✓。

**无哈希猜测**：三棵树不是从 cargo 带哈希的构建目录里刮出来的，而是用**生成构建脚本所设的同一套环境变量**经 `check` 取到——那是构建跑的同一套管线；而 `cargo build` 那一步正是证明这些树编译得过的东西 ✓。

**实测**：`tools/xirang-partition-rehearsal` 三条腿全绿（`== OK — both shapes build, the four facts hold, and the promote lands`）✓。

**P3.5 仍差**：`--release` 物化真实包（自包含副本 + `[package.metadata.xirang] shape` + 身份/记录/租约迁移并显式列出哪些记录失效）。

### §M7.42 P3.5 第三件：**发布形状** —— 每个包携带自己构建要读的源码（2026-10-07）

开发形状把碎片的文件从宿主包里**挂载**出来（一次编辑两个 crate 同时可见），而它正是发布不了的原因：crates.io 拒绝 `#[path]` 伸到包外的包（已实测），而让身份在开发树里一致的 `--remap-path-prefix` 住在工作区配置里、没有依赖方会继承（那正是 §M7.33/§M7.35 标注的未解边界）。这一节做的是同一笔交易的另一半：**每个生成的包都携带自己构建要读的源码**，因此形状里没有任何东西依赖包外的文件，**也不需要任何 remap** ✓。

**两个实测事实**（先量后写，两次都是可编译的实验）：
- **复制到包自己的 `src/` 之下 ⇒ 身份逐字节不变、remap 不需要**。声明宏的身份输入来自 `file!()`，做法是去掉 `CARGO_MANIFEST_DIR` 与其后的一个 `src/`（`kernel/src/registry_core/identity/path_text.rs::manifest_relative_source`）——文件一旦在包内，这条推导自己就给出宿主烤进去的那个 `<模块>/<文件>.rs`。实测（手搭的 `/tmp/p32rel/app-widgets`）：不分区基线与发布幽灵共享的两个面 `StaticFace` 行逐字节相同、切割面自己的 `assert_static_identity` 也逐字节相同 ✓，而且**没有写任何 `.cargo/config.toml`** ✓。
- **facade 在构建期经 `cargo metadata` 找宿主**，这是唯一活得过发布的拼写：行内 `path` 在打包后的清单里会被 cargo 改写成注册局要求。实测：这样解析宿主的 facade 构建全绿、生成的并集与宿主一致 ✓（`cargo metadata` 从构建脚本里跑不会死锁 ✓）。

**形状**（新模块 `build_method::crate_release` + `crate_write::write_release`，随 `cli` 门控）：
- **幽灵**：`Cargo.toml`（宿主依赖表逐字照抄 + `[package.metadata.xirang] shape = "release"` 与 `host = <宿主命名空间>`，**没有** `publish = false`）· `src/lib.rs`（命名空间常量 + `include!(OUT_DIR/generated_lib.rs)`）· `build.rs`（`env!("CARGO_MANIFEST_DIR")` + `XIRANG_SHAPE_ONLY=<认领>` + `run_for(本包根, OUT_DIR, 宿主命名空间)`）· **复制进来的源码**：认领目录**整棵**（注册面、注册规范、手写宿主放在旁边的任何文件）+ **每个祖先面的文件与其 `registry_rule/` 目录** + 宿主声明 `add_crates.rs`。
  - 祖先与规范文件不是可选装饰：**第一次发布构建就是被它们挡下的**——没有祖先面，推导报 `parent declaration cannot be resolved` / `parent node is missing`（碎片里 `parent:` 指向的祖先面必须在树里）；而生成树会挂载注册规范文件（`pub mod registry_rule;`），漏了同样编译不过 ✓。
  - 声明里带**自定义 `registry_rule_path`** 的祖先会被**点名拒绝**（本形状复制目录、不追声明），并给出两条出路 ✓（窄而会拒绝胜过宽而会猜）。
- **facade**：不携带源码 ✓；`build.rs` 跑 `cargo metadata` 找到宿主包根（`"name":"<宿主>"` 之后紧邻的 `manifest_path`——脚本没有自己的依赖，因此是扫描而不是解析 JSON），再以 `XIRANG_SHAPE_FACADE=1` 跑同一条 facade 管线 ✓；清单同样是"宿主依赖照抄 + 同级依赖"并带形状指纹 ✓。
- **两个形状共用同一批目录** ⇒ 写一个之前必须先撤回另一个：`crate_write::guard_shape` 按目录读 `Cargo.toml`，发现磁盘上记着的形状与本次要求不符就**点名拒绝**并给出 `--revert` ✓（钉子钉住两个方向：开发盖发布被拒、它本来就是的形状照常覆盖自己、目录还不存在时不算任何形状）。`--revert` 对两个形状是同一个（目录相同、标记检查相同、成员清单同样收回）✓。

**实测（`/tmp/p32rel3`，真 CLI）**：`crates --release --write` ⇒ `wrote 12 file(s)`（幽灵 3 份生成文件 + 4 份复制（2 面 + 自身规范 + 祖先面与祖先规范）+ facade 3 份 = 12 ✓）、成员清单物化 ✓、**没有任何 `.cargo/config.toml`** ✓；`cargo build -p app-facade` 从工作区根一次通过（宿主+幽灵+facade 三条 `Compiling` + `Finished`）✓；发布幽灵生成的 `#[path]` 全部指向**它自己**（宿主 `src/` 出现 0 次）✓；`cargo package --list` 两个包都自包含（幽灵带上 `add_crates.rs` 与全部源码、facade 只有三份文件）✓；`--revert` 把清单逐字节还原、两个包都消失 ✓。

**CI**：分区演练加了**第四条腿**（发布形状：断言携带的源码齐全、没有 remap 配置、`cargo build` 通过、`cargo package --list` 成功、`--revert` 干净），四条腿在 `tools/xirang-partition-rehearsal` 上全绿 ✓。

**仍差（P3.5 收尾）**：同级依赖的**版本号**（`path` + `version`，发布时 cargo 只保留 version）与 `license`/`description` 等发布元数据是宿主作者的字段，本形状不代填；`cargo publish --workspace` 那一步要等一次真正的发布（需要 index 与维护者一句"发布"）。

### §M7.43 P3.6：**Studio 可视化分区**（2026-10-07）

维护者愿景里的最后半句——「**然后也可以在studio中操作**」——落地：`c` 打开 **CRATE PARTITION** 屏，`w`（开发形状）/ `R`（发布形状）/ `x`（撤回）各要按一次 `y` 确认。

**一份读取器，两个执行面**：新模块 `build_method::partition_view`（`pub(crate) fn plan` / `pub fn view`）是 CLI `xirang crates` 与 Studio 本屏共用的**唯一**读取器，读的是**已发布的记录**（`target/xirang/out`）而不是现推导一遍——理由与写入路径相同：计划就是构建发布出来的东西，自己推导的画面会描述一棵写入方并不打算写的树。它给出：声明的前缀、宿主自己那棵树有多少面、每个生成包**服务哪个 crate/哪棵子树**、**编译几个面**、开发形状**挂载几个文件**、发布形状**复制几个文件**、磁盘上**是哪种形状**（`OnDisk::{Absent,Development,Release,Foreign}`）、**文件数与字节数**（维护者要的"自动判断大小"）、facade 的**分区内依赖**（发布顺序），以及逐条的**发布说明**（缺 `description`/`license` · `publish = false` · **只有 path 没有 version 的依赖** · 开发形状不可发布）。`OnDisk::Foreign` 与"包名读不出来"都是**点名拒绝**而不是兜底（原先 `package_name` 失败会静默回退成 `"host"` —— 而包名**就是**身份命名空间，猜一个会描述出没人烤过的 id ✗）。

**三个动作调用的是 CLI 同一批写入方**（`write_partition` / `write_release` / `revert_partition` + `guard_shape`），因此两个执行面不可能写下不同的树。为此把这几族模块的**特性门从 `cli` 放宽到 `any(cli, studio)`**（`cli = [..., "studio", ...]` 包含 studio、但 studio 不包含 cli，所以此前 Studio 根本够不到它们），并把 `partition_roots`（配置与成员清单该落在哪）与 `build_out_dir`（记录在哪）从 CLI 移进 `build_method` —— **一条规则只有一份实现**，CLI 现在调用它们。UI 层：`ui/forms/partition.rs` 画屏（分区块 + 每包一行 + 页脚 `w development · R release · x revert`），`app/overlay/partition.rs` 收键（**任何键都消费掉已装备的动作**，与删除/graft 同一套"两次按键"纪律），`state/partition.rs` 存状态，页脚加入 `c crates`。

**实测**：`target/visual/partition.txt`（真 tmux 面板，300×60）里 `CRATE PARTITION` 屏显示出 `declared package_prefix … · 2 generated package(s)`、幽灵行 `role crate widgets (panel::frame)` + `carries 1 face(s) compiled · 1 mounted (development) · 3 copied (release)`、facade 行 `depends on partition-demo, partition-demo-widgets` ✓。**顺带修一个 CI 一直红着的东西**：`tools/xirang-visual` 的构建行仍写着合并前的 `-p xirang-studio`（那个包在九→三合并后已不存在）⇒ 整条视觉步骤只可能失败 ✗；改成 `-p xirang-toolchain --features studio --bin xirang-studio` ✓，并把 `partition` 场景接进 `ci.yml` 的视觉步骤（场景自带一个声明了拆分的宿主，因为它**不能**往真实示例宿主里加 `add_crates.rs`——那会改变示例构建出来的东西）。

**钉子 7 条**（4 条视图 + 3 条 Studio）：无声明 ⇒ 视图是 `None` · 视图描述两种形状与磁盘状态（含 `Foreign` 与"开发形状不可发布"）· 未列入工作区成员清单的包被点名 · **屏幕打开时看到声明出来的两个包**（含 facade 的依赖顺序）· 无声明的宿主被说出来而不是给空清单 · **三个动作真的写下/收回**（`R` + `y` ⇒ 幽灵带 `shape = "release"` 且携带碎片源码、facade 就位、屏幕随即读到 release；`x` + `y` ⇒ 两个包都消失）✓。夹具两处地雷同时修掉：**宿主必须放在自己的一层目录里**（幽灵是同级，直接坐在 `/tmp` 的宿主会把包规划进 `/tmp`——实测真读到了别人的目录 ✗）、以及夹具的 `../toolchain` 依赖要指向本检出（否则 `cargo metadata` 读不出包名）。

**门禁**：fmt ✓ · workspace 全绿 ✓ · clippy 两面 exit 0 ✓ · conventions（含 size）✓ · 发布表 ✓ · 视觉场景 ✓ · 十面见提交说明。**仍差**：P4（MCP 工具面接 `add_crates`）——`partition_view` 已经是为它准备的公开读取器。

### §M7.44 P4：MCP 工具面接 `add_crates` —— `xirang.crates`（2026-10-07）

维护者的话「进 MCP 工具与 Studio 工作流」两半里的后半：**桥**是第三个执行面，而它用**同一个**读取器（`build_method::partition_view`）与**同一批**写入方，因此三个面不可能对"这次拆分要写什么"给出不同答案。`action` 是 `plan`（默认，也是唯一不写入的）/ `write`（开发形状）/ `release`（可发布形状）/ `revert`（收回来）；**不带 `apply: true` 一律只是预览**——回复是分区加上一句"这个动作会改什么"，带上才真的写，并且回复携带**从磁盘读回**的那棵树（下一次调用可以据此瞄准，而不是据调用方希望发生的事）。答案逐包给出：它服务哪个 crate/哪棵子树 · 编译几个面 · 开发形状挂载几个 · 发布形状复制几个 · **磁盘上是哪种形状** · **文件数与字节数** · facade 的**分区内依赖**（＝可发布顺序）· 逐条**发布说明**（缺 `description`/`license` · `publish = false` · 只有 path 没有 version 的依赖 · 开发形状不可发布）；没有声明的宿主被告知声明该写在哪、长什么样，而不是拿到一张空表。

**登记要成套**（四条表缺一不可，钉子会当场点名）：目录 `tools()` 的一句话 + `advertised_schema` 的 schema 臂 + `DISPATCH` 的顺序（与目录同序）+ `effect`（`Effect::Rewrite` ⇒ 自动 `destructiveHint: true`，因为 `revert` 真的会把包拿走）+ `ownership::subject` 归到 `Subject::Write` + `READ_KEYS` 行（`action`/`apply`）+ 写者名单与 destructive 名单各一行 + `next_hint` 一句话。**顺带把特性门放宽到 mcp**：`partition_view`/`crate_write`/`crate_release`/`crate_facade`/`crate_members` 与 `crate_plan::relative_walk` 的门从 `any(cli, studio)` 变成 `any(cli, studio, mcp)`——桥要用同一批写入方，而它此前根本够不到它们。

**实测（真二进制、真调用）**：`xirang-mcp --list crates` 列出 `keys: action, apply, root` ✓；`--call xirang.crates --root <fixture> --action plan` 打出分区 + "no action taken" + 双语说明 + `next` 提示 + 可引用的 `evidence` 行 ✓；`--action release`（不带 apply）**磁盘上什么都没变** ✓；`--action release --apply true` 写下 **9 个文件**，`Cargo.toml` 里 `shape = "release"`，读回的行变成 `on disk release · … 6 file(s), 2.0 KiB` ✓；`--action revert --apply true` 把包收回去 ✓。**钉子 5 条**（`mcp/crates_tests.rs`）：无声明的宿主被告知声明在哪 · `plan` 不写任何东西 · 不带 `apply` 的动作不改变磁盘 · `apply` 写下发布形状后 `revert` 收回 · 未知动作按名字被拒并列出存在的四个 ✓。**门禁**：fmt ✓ · clippy 两面 exit 0 ✓ · 发布表 ✓ · 十面见提交说明。

### §M7.45 把 CI 推绿（2026-10-07）：1.99 的两条新 lint + 六个只在 macOS/Windows 上成立的路径假设

首次推送（64 个提交）后 CI 七个作业红。**归因先行**：与上一次 main 上的运行（`37242219783`，2026-10-04）逐作业对比——`release-audit`、`features`、`ubuntu-latest stable`、`windows-latest` 两个 `Tests` 在两次里都红（继承），而两个 **Documentation** 失败这次**变绿**（§M7.44 那批 rustdoc 修复生效 ✓），macOS 两个作业则从"更早的步骤就红"变成"Tests 红"（先前被 Clippy/Documentation 挡住，**新可见**而不是新引入）。⇒ 四条真实缺陷：

① **clippy 1.99 的两条新 lint**（本机 1.96 根本看不到，所以本地门禁全绿而 CI 红）：
`clippy::single_element_loop` @ `toolchain/src/run_method/authoring/manifest/parse.rs:59`（`for key in ["registry_rule_path"]` ⇒ 展平成一个 `if let`）；`Some(x).filter(|_| predicate)` @ `toolchain/src/mcp/check.rs:776`（谓词不看值 ⇒ `(!path.is_empty()).then(|| …)`）。这两条也是 `release-audit` 作业失败的原因（它同样跑 lint）。
② **macOS：`/var` 是 `/private/var` 的符号链接** ⇒ 我的 P3 测试把 `canonicalize()` 过的路径与原始 `temp_dir()` 路径逐字比较，左边 `/private/var/…` 右边 `/var/…` ✗。
③ **Windows：分隔符与 `\\?\` 前缀** ⇒ `starts_with('/')`、`ends_with("host/src/")`、`spec.contains("/checkout/toolchain")`、以及"报告里含 `lock.display()`"这四条断言在 `\` 拼写的路径上全假 ✗。
④ **`a_gone_holders_lock_is_taken_over` 在 macOS/Windows 上本来就不该成立**：`process_is_alive` 只在 Linux 经 `/proc` 回答，其它平台**有意**回答"假定活着"（宁可让陈旧锁把下一次发布推迟到 `STALE_AGE`，也不从活写者手里夺锁）——这是产品里写明并带理由的设计，错的是**测试**：它在没有 `/proc` 的平台上断言了一次该平台做不到的接管 ✗。

**修法**：路径一律经 `Path` 判断（`is_absolute`、`ends_with("src/…")`——Windows 的解析器同时接受 `\` 与 `/`），或在比较前把两侧的分隔符拍平；"幽灵位置不影响拼写"那条钉子改成比较**同一个根**拼出的两个字符串（平台无关 ✓），规范化那一半只比 `file_name()`；发布锁那条按平台分支：有 `/proc` 走接管（并断言锁文件随后点名本次运行），否则走拒绝（并断言拒绝点名文件与 pid、且锁还在）✓。同时为 1.99 的同类形状做了全仓扫描（单元素 `for … in [..]` 归零、`.filter(|_|` 只剩迭代器上的一处与两行注释）。

**验证**：本机 fmt ✓ · workspace 全绿 ✓ · clippy 两面 0 ✗error ✓ · 发布表 ✓ · 十面见提交说明 ✓；平台相关的那几条本机跑的是 Linux 分支，macOS/Windows 分支由 CI 矩阵判定 ✓。

### §M7.46 第二轮推 CI：两条归一化、两处合并遗留的 crate 名、以及视觉步骤的等待与诊断（2026-10-07）

第二轮结果：**macOS 两个 + ubuntu stable + ubuntu 1.96 全绿** ✓（1.99 的两条 lint 修好了）。剩三个作业红，逐个定性：

① **macOS/Windows 上还剩两条**（`publish_lock_tests.rs:249`、`scaffold/project.rs:641`）——我上一轮的两处"归一化"各自错了一层：
- 锁那条断言比较的是子进程报告，而报告是 **`{:?}` 打印**的，因此 Windows 路径在**文本里就是双反斜杠**；只把 `\` 换成 `/` 会把 `C:\\Users` 变成 `C://Users` ✗。正确顺序是**先**把 `\\` 折成 `/`、**再**处理单个 `\`。
- 脚手架那条更微妙：`toml_path` **已经**做了正确的转义（`\` → `\\`，因为拼出来的是一段 TOML **基本字符串**）✓，产品是对的；错的是我的断言——它把转义过的 `\\` 又各换成一个 `/`，于是 `/checkout\\toolchain` 变成 `/checkout//toolchain` ✗。修法是**按 TOML 的规则先还原转义、再统一分隔符**（顺序反过来就判错一条拼写正确的路径）。
② **`release-audit` 的真实原因与 lint 无关**：`tools/xirang-release-audit:150` 与 `tools/xirang-scale-audit:10,15` 仍写着 **`-p xirang-run-method`** ——九→三合并之后那个包不存在了，于是整个作业在 `cargo run` 处报 `package(s) xirang-run-method not found in workspace` ✗（与 `tools/xirang-visual` 的 `-p xirang-studio` 同一族：**合并遗留的 crate 名**）。改成 `-p xirang-toolchain --example scale_audit` ✓；本机跑 `tools/xirang-release-audit` 通过：`symbol_audit=…/artifacts.tsv (126 artifacts, each with defined symbols)` ✓。注意这一族里 `scale-audit` 那一半此前从未跑到过（上一步先红），因此它是**顺带修掉的第二个潜伏失败** ✓。
③ **视觉步骤 `Visual check (tmux)` 第一次真的跑到 tmux 那一步，就在第一次等待上红了**，面板**完全空白**：分不清"会话没起来"与"程序还没画第一帧"。这一族以前从没跑过（构建行是坏的 ⇒ 更早的步骤就红 ✗）。工具因此收三处：等待预算从 150×0.2s（30 s）提到 **600×0.2s（120 s）**并可用 `XIRANG_VISUAL_TRIES` 覆盖（冷的 CI runner 在 Studio 画出第一帧之前要解析工程并跑 `cargo metadata`）；每轮用 `tmux has-session` 判"会话是否已经死掉"，死了立刻失败并**打印该场景的 stderr**（启动失败时那就是答案）；超时时也把 stderr 一并打出。本地无法复现 tmux 这一步（本机沙箱不允许 tmux 在 `/tmp` 建 socket，会话直接消失），因此这一半**由 CI 判定** ✓。

**门禁**：fmt ✓ · workspace 全绿 ✓ · clippy 两面 0 error ✓ · 发布表 ✓ · `tools/xirang-release-audit` 本机通过 ✓ · 十面见提交说明。

### §M7.47 视觉步骤真正的病根：夹具路径也是合并遗留（2026-10-07）

上一轮加的诊断（会话死掉就打印该场景 stderr）在 CI 上立刻给出了"会话死了、stderr 为空"，**还是没说出为什么**——因为 `cd` 的失败不在 `2>` 的覆盖范围内，而会话死得太快、`capture-pane` 已经抓不到面板。于是要一个真实 pty 才能继续：本机沙箱不允许 tmux 在 `/tmp` 建 socket（会话会直接消失，症状与 CI 一模一样 ✗），放行一次后**当场复现**，两条命令就定位了：

`FIXTURE="$ROOT/studio/tests/fixtures/node-editor"` —— 这个目录在九→三合并后**不存在**（夹具在 `toolchain/tests/fixtures/node-editor`）✗；会话命令是 `cd '$FIXTURE' && …`，`cd` 失败 ⇒ shell 立刻退出 ⇒ **会话在第一帧之前就死了**，面板因此完全空白 ✓。这是同一族"合并遗留"的第三处（前两处：`-p xirang-studio`、`-p xirang-run-method`）——**改动 crate 布局时，工具里的路径与 `-p` 名字都是引用，而没有任何门禁读它们**。

改成 `toolchain/tests/fixtures/node-editor` 后，本机真实 tmux 里四个场景（`home graph tree-demo partition`）**全部通过** ✓，抓屏非空且标记都在 ✓。教训写进 lesson：**"会话死了但 stderr 为空"时，先怀疑会话命令自己的第一步（`cd`/`exec`）失败——重定向只覆盖最后一条命令**；而"合并/搬目录之后，工具里的路径与包名要当成引用逐个核对"，这一族在这个仓库已经出现三次。

### §M7.48 声明层可操作：`declare` / `undeclare`（2026-10-07，维护者问出来的洞）

维护者一句「**mcp 难道不能 delete crate 布局吗？**」点到的是真洞：P3.6/P4 只覆盖了**生成物**那一层 —— `write`/`release` 把声明的拆分写出来、`revert` 把生成的包收回去（三个面都有 ✓）—— 而**声明本身**（`add_crates.rs`：哪棵子树成为哪个 crate）在 CLI/MCP/Studio 里**都只能人手编辑** ✗。他原来那句「也可以在 studio 中操作」指的正是这一层。补上：

**一份实现，三个面共用**（`build_method::shape_decl`）：`declare(root, name, subtrees)` 与 `undeclare(root, name)` 返回 `DeclarationEdit { path, before, after, removes_file }`，编辑是**文本**而不是重渲染 —— `add_crates.rs` 是作者也会读的手写源码，重渲染会重排版、还会丢掉他们的注释 ✗。`undeclare` 从 `Crate::named("<name>")` 起**按括号深度**找条目结尾（不按版式猜；条目可能跨行、还可能带嵌套 `&[…::SUBTREE]`），连同它那一行的缩进一起移除，**其余每个字节保持原样**（钉子逐行比对 ✓）。**移除最后一个 crate 就是移除文件**：宿主回到"就是一个 crate"，而一份 `crates: &[]` 是读取器有意拒绝的形状（`declare` 在无文件时会**创建**它，前缀取自宿主包名 —— 与每个生成包名的推导同一个值）。

**三条守住语义的规则**：① **`undeclare` 在该 crate 的包还在磁盘上时拒绝**（"先 `revert`"）—— 条目一旦消失，就没有动作还知道那个目录了，留着就是本工具再也够不到的孤儿 ✓（这条还当场纠正了我自己写错的路径算法：生成包在宿主**旁边**，第二份"包在前缀里"的字符串运算第一次就找错了地方 ⇒ 改成问计划本身 ✓）。② **改动后会验证规划**：`apply_declaration_edit` 落盘 → 读回计划 → 不成立就**回滚原样**并说明（"声明已恢复原样"），而不是留下一份其余工具都拒绝的声明 ✓。③ **预览就是那段确切的文本差异**（`-`/`+` 行），`apply: true`／`--write` 才写 ✓。

**三个面**：MCP `crates {action:"declare"|"undeclare", crate, subtree}`（schema/目录/READ_KEYS/next 提示成套登记 ✓）· CLI `xirang crates --declare <name> --subtree <crate::…::SUBTREE> [--write]` / `--undeclare <name>` ✓ · **Studio 的删除键还没做**（列在余项里，没有假装完成 ✗）。

**实测**：CLI 端到端（一次性宿主）：`--declare` 预览只打印 diff、文件未动 ✓ → `--write` 创建声明（`package_prefix: "smoke"` 取自宿主包名 ✓、读回 1 个 crate ✓）→ `--undeclare` 预览打印整份文件的 `-` 行 ✓ → `--write` 连文件一起收走（"declares no crates again"）✓。MCP 侧同路径由钉子覆盖（预览不动文件、`apply` 落盘、包在磁盘上时拒绝、缺参数按名字拒绝）✓。**钉子 7 条**（`shape_decl_tests`：只动被点名那条、最后一条连文件走、未知名字列出已知名、无声明时拒绝、首次声明创建文件、同名拒绝、非路径子树拒绝）+ 3 条 MCP 钉子 ✓。

### §M7.49 Studio 的删除键（维护者：「0.2.1 是包含这个删除键的」）

§M7.48 里我标为"尚未做"的那一件补上了：分区屏按 **`D`** 把**选中的已声明 crate** 的条目从 `add_crates.rs` 里移除，按 `y` 确认（与那三个针对生成包的动作同一套"两次按键"纪律 ✓）。三个面至此都改得动声明层：**MCP `declare`/`undeclare` · CLI `--declare/--undeclare` · Studio `D`** ✓，三者走**同一批写入方**（`shape_decl::{declare,undeclare}` + `partition_view::apply_declaration_edit` + `package_directory_of` 守卫 ✓）。

**两个语义点**：① 选中的是 **facade** 时不静默（它不是已声明的 crate，按键会**说出来**——静默会被读成坏键，审计 `STU-V-02` ✓）；② 该 crate 的包**还在磁盘上**时拒绝并给出那个键（"先按 `x` 收回"）——条目一旦消失，就没有动作还知道那个目录 ✓。

**这条键当场抓出一个真 bug（值得记）**：它第一次跑就报 `the widgets entry has no ending comma, so this writer cannot say where it ends` ✗ —— 我的 `entry_span` 只把**结尾逗号**当条目终点，于是**单行** `crates: &[Crate::named("widgets").at(&[…])]` 里的最后一条（**合法地没有**逗号）永远移除不掉 ✗。修法是让"关掉这条条目所在列表的那个括号"也成为终点（`]`/`}` 使深度 < 0 时收尾 ✓），并补一条**单行声明**的钉子 —— 原来的夹具都是多行写法，所以这个洞在 CLI/MCP 两级钉子那里都没露出来 ✓✓（**同一份实现，两套夹具的形状不同，就能漏掉一种版式**）。

**实测**：真 tmux 面板里页脚已宣传该键（`… · D remove the selected crate from the declaration (each asks once more) · …` ✓）；钉子 3 条（`D`+`y` 移除且屏上随即读到"宿主又只有一个 crate" · facade 上按键会被说明 · 包在磁盘上时被拒并提示先 `x`）+ 1 条（单行声明可移除）✓。门禁：fmt ✓ · clippy 两面 0 error ✓ · workspace 全绿 ✓ · studio 分区钉子 6/6 ✓。

### §M7.50 graft × 多 crate：两个 guest 同时在场（维护者点的测试轴）

维护者：「**你不仅仅要测试你说的那些，你还要测试比如 graft 的部分有 crateA 也同时有 crateB 等等，就是它是不是全代码域都可以随意 graft**」。先测他点的这一轴（**两个 guest 同时在场**），方式是**先写红钉子**：新钉子 `two_guests_each_emit_their_own_cut_and_the_facade_carries_both`（renderer 级，故意用两个 guest：`panel::frame` → `app_widgets`、`panel::gauge` → `app_gauges`，计划里两条切口分别落在两者之内）。

**第一次跑红了** ✓（这就是它的价值）：facade 发出的属主是 `app-widgets::…`（**连字符**）。查下去发现是**我夹具的错**——真实管线里 `crate_plan::owners` 会 `planned.package.replace('-', "_")` ✓（Rust 路径段不能含连字符），我手工传的属主表没做这一步 ✗。产品是对的 ✓，钉子随即改成按管线的拼法并**加了一条反向断言**（生成的文件里不许出现 `app-widgets` ✓）。

**结论：这一轴是通的** ✓，且现在被钉住了。实测（真渲染输出）：
```
pub static BUILTIN_GRAFT_CUTS: &[…StaticGraftCut] = &[
    …StaticGraftCut::from_ids(app_widgets::panel::frame::widget::NODE_ID, fast_widget::fast::NODE_ID, false),
    …StaticGraftCut::from_ids(app_gauges::panel::gauge::dial::NODE_ID, fast_gauge::fast::NODE_ID, false),
];
```
⇒ ①**宿主**（两条切口都落在交出去的子树里）**两条都不发**（表为空 ✓）；②**幽灵 A** 只发自己那条、**幽灵 B** 只发自己那条（各自都不点名对方的模块 ✓）；③**facade 携带并集**，两条各自改写成**编译它的那个 crate**（`app_widgets` / `app_gauges` ✓），契约断言同属主 ✓。**这正是"crateA 与 crateB 同时在场"的答案**：切口按**面**归属到编译它的 crate，跨 crate 的那一半归 facade ✓。

**仍未测的（"全代码域随意 graft"的其余位置，下一批）**：切口落在**根面** / **容器面**（含 `full` 跨两个 crate 的子树：`panel` 自己有面、其子 `frame`(A) 与 `gauge`(B) 已交出去 ⇒ 谁携带这条切口？）/ **项目外面**（现有钉子只断言 facade 原样通过 ✓）/ **插件宣告的面** / **链式 graft**（切口面本身是别人 graft 来的）。这些要靠**真 cargo 的端到端**（现成载体：`tools/xirang-partition-rehearsal` 加一条腿），不是渲染级钉子能收口的 ✓。

### §M7.51 第一个**真的**被划分过的示例：`examples/partitioned-button`（2026-10-07）

维护者问「我们现在声明的例子在哪里？」—— 查真树后的答案是：**仓库里没有真宿主带声明** ✗（`ls examples/*/add_crates.rs` 为空）；声明只存在于夹具、测试常量、`crates --declare` 的模板与文档片段里。他要「补真示例来测试一下」，于是补了这一个。

**形状**（独立虚拟工作区，**不进根工作区** ✓ 实测根成员仍是 6 个、不含它 ✓）：
```
examples/partitioned-button/
  Cargo.toml                 ← 虚拟工作区：members = host, graft, + 两个生成包
  host/                      ← 真宿主：src/**（5 文件/3 面）+ add_crates.rs + 一条类型化 graft 计划
  graft/                     ← 第三方 graft 实现（独立包，`[lib] name = control_button_graft`）
  partitioned-button-objects/ ← 生成物：幽灵（3 mounted files、1 remap）
  partitioned-button-facade/  ← 生成物：facade（依赖宿主 + 幽灵）
  .cargo/config.toml          ← 生成物：partition 根的 remap（**入库**，否则换台机器构建不过）
```
**实测（原始退出码）**：`check exit=0` → `crates --write`（wrote 8 files；config 与 members 均已就位）→ **宿主 / 幽灵 / facade 三个 `cargo build` 全部 exit=0** ✓；各 crate 发出的切口 = **宿主 0 · 幽灵 2 · facade 2（属主改写为 `partitioned_button_objects::control::object::button::NODE_ID`）** ✓ —— 这就是"划分把 `control` 整棵交给新 crate 之后，graft 计划一个字没改、切口跟着面走"的可运行证据 ✓。

**门禁**：`tools/xirang-partition-rehearsal` 加 **leg 5**（构建这个入库示例，断言三个 build 0 且切口 0/2/2 + facade 点名属主 ✓）。**它当场抓出我自己脚本里的一个真 bug** ✗：`cuts_in()` 写成 `grep -c … || printf '0'` —— `grep -c` 无匹配时**既打印 `0` 又退出 1**，于是 `||` 又打一个 `0`，比较读到 `"00"` ⇒ 腿误报红；另外 `find … | head -1` 会挑到**声明存在之前那份旧产物**（构建目录会活过形状改变）⇒ 改为取最新一份 ✓。修后整条演练 **OK** ✓。

**一处已知小瑕疵（未修，记账）**：生成包构建时会报 `unexpected_cfgs: rust_analyzer` 警告（宿主若 `deny(warnings)` 会硬失败 ✗）—— 已在本项目 todo 里，处置方式是在生成的清单里补 `[lints.rust] unexpected_cfgs` ✓。

### §M7.53 提交进仓库的示例改成**发布（复制）形状**：开发形状的 remap 会毒死 rust-analyzer（2026-10-07）

**起因**：维护者报「我所有的补全都失效了，连 cargo.toml 都不高亮」，截图里 nvim 的横幅是
`LSP client log is large (2782 MB): ~/.local/state/nvim/lsp.log`。看日志尾部，刷屏的是：

```
rust-analyzer stderr ERROR flycheck 0: File with cargo diagnostic not found in VFS:
  file not found: …/examples/partitioned-button/control/control.rs
```

**链条**：我提交的示例当时是**开发形状**（幽灵 `#[path]` 挂载宿主源码 + `.cargo/config.toml` 里一条
`--remap-path-prefix`，后者是**身份**所必需）⇒ rustc 报出的诊断路径是被 remap 过的**宿主相对路径**
（`control/control.rs`）⇒ rust-analyzer 拿它去工作区根找、找不到 ⇒ **每次 flycheck、每个文件一条 ERROR**，
`lsp.log` 涨到 2.6 GB ⇒ nvim 的 LSP 客户端被拖死 ⇒ **所有补全/高亮全灭**。⇒ **开发形状与 rust-analyzer
天生冲突**，这不是示例写错了，而是那个形状的性质。

**处置**：把提交进仓库的示例按 **`crates --release --write`** 重写成**自包含复制形状** —— 每个生成包携带
自己编译的源码副本，**没有** `.cargo/config.toml`、**没有** remap ⇒ 编辑器不再报错、不再刷日志 ✓；同时它
天然满足 crates.io 的自包含要求 ✓。代价＝包里是复制品（改动要重生成 ✓），这与维护者早先的
「**为啥不全按正常的目录放置呢**」是同一个方向。

**顺带修掉的两处真缺陷**（都由这次切换当场抓到）：
1. **CLI 打印的 facade 路径是自己拼的**（`package_root.parent().join("<prefix>-facade")`），比实际写下处
   **高一层**（打印 `…/partitioned-button-facade`，实写 `…/crates/partitioned-button-facade`）——正是那种会把
   读者送去追一个不存在的缺陷的自我描述 ✗；改成**问**共用规则 `crate_plan::crates_dir` ✓（排查时它把我误导了三轮）。
2. **发布形状的 facade 清单漏了重拼 `[build-dependencies]`** ⇒ 同一个 crate 在 `[dependencies]`（4 层）与
   `[build-dependencies]`（3 层）里指向不同路径 ⇒ cargo 直接拒绝（`Dependency 'xirang-toolchain' has different
   source paths depending on the build target`）✓；已与 `[dependencies]` 走同一条重拼 ✓。

**实测**：`host` / `crates/partitioned-button-objects` / `crates/partitioned-button-facade` 三个 `cargo build`
**exit=0** ✓；示例里 `.cargo/config.toml` 数量 **0** ✓、`--remap-path-prefix` **0** 处 ✓；
`tools/xirang-partition-rehearsal` OK ✓（它自己的四条腿仍在 `/tmp` 里写开发形状夹具 —— 那些不进编辑器，
因此无害 ✓，但**形状之争（挂载 vs 复制）的总账仍开着** ✓）。

### §M7.54 交接四项落地：缺规则的面、重新划分不变性、A/B 两轴钉子、认领被剪的真缺陷（2026-10-08）

**① `check` 与 `build` 的差集（真缺陷，最该修的那件）**。拥有注册机（`needs_registry: true`）、又不点名
`registry_rule:` 的面，宏展开成 `super::registry_rule::REGISTRATION_RULE`（`macro/src/lib.rs` 的
`face_rule_or`），而生成树只在面旁边存在 `<面目录>/registry_rule/registry_rule.rs` 时才挂载该模块 ⇒ 那个文件
被删掉、改名或搬走时，**`check` 报 ok（rc=0）而 `cargo build` 报 101**（`error[E0433]: cannot find
registry_rule in super`）。实测：`examples/control-suite` 删掉 `panel/registry_rule/registry_rule.rs` ⇒
`check rc=0` / `build rc=101`。

**修法（窄修）**：新增 `phase=face-rule` 诊断（`face_syntax_check::aggregate_registry_rule_errors`，接进
`pipeline.rs` 的声明层检查）。判据是**被发射的那条引用**，不是文件：显式写了 `registry_rule:` 的面走作者给的
路径、不需要兄弟模块；不拥有注册机的面保留宽松兜底 —— 两者都不被拒绝。拒绝给三条出路：补回规范规则
文件、用 `registry_rule: <path>::REGISTRATION_RULE` 点名、或去掉 `needs_registry: true`。解析之前有一道
`source.contains("needs_registry")` 的便宜闸门（字段要存在就必须字面出现 ⇒ 只可能放进更多文件，绝不藏起
拒绝）：1,000 个面上实测，无闸门 6.6–6.8 s，有闸门回到基线 6.4–6.5 s（对照 6.48–6.54 s）。

**真项目验证（无假拒绝）8/8**：基线 ✓ · 新面带规则 ✓ · 新面**无**规则 ⇒ 拒绝且 build 也失败 ✓ · 新面点名规则
且无兄弟模块 ✓ · 不拥有注册机的面无规则 ✓ · 删/改名/搬走既有规则 ⇒ 拒绝 ✓。同一批把
`tools/xirang-daily-behaviors` 变成**自检门禁**（每行带期望 + 不变量「build 失败必须是 `check` 已经拒绝过
的」），修掉它自己的两个装置缺陷（`crate::__xirang_face!` 从来不存在 ⇒ 那两行报的是装置错；行 8 删的其实
是规则文件），并接进 CI：**两阶段 39 行全过**。

**③ 重新划分不变性做成门禁**：`tools/xirang-partition-rehearsal` 新增 **leg 6** —— 作者的 graft 计划只写
一次，crate 边界搬三次（`panel::frame::widget` → `panel::frame` → **一个 crate 认领两棵子树**），每一次都
检查三种形状、`--write`、构建，并断言切口逐字不变、facade 点名胜任属主、宿主的计划文件未被改写。

**④A 三条钉子**：**根面 + `full`** 早有两条钉子（`scope_tests::a_full_cut_on_the_root_face_keeps_every_face_live`
+ graft matrix leg 1，本次重跑通过）✓；**记录 `version` 不支持 ⇒ 构建按名拒绝**（内核早有该错误，但
`planned_slots` 对任何解析失败都静默 `continue` ⇒ 等于发布一个嫁接静默地从未发生的二进制）新增
`graft_plan_check::unsupported_version_errors`；**区间 + `full` ⇒ 命名拒绝**（实测此前**被接受**并发出
`from_id_range(…, true)`，含义哪里都没写）在**解析器与渲染器两处**按名拒绝，共用一份文案。

**④B 两处缺口**：**`crates --check` 现在比对形状指纹**（`guard_shape` 原先只在 `--write` 调用 ⇒ 手跑开发
形状落在入库发布形状上无声；受检集合与对应 `--write` 完全一致）；**一个 crate 认领多棵子树**的端到端（leg 6
边界 C）当场逼出一个**真缺陷**：认领的子树若不含任何声明槽位，会被嫁接槽位造成的自动收窄剪掉 —— 宿主交出去
了、幽灵又被剪 ⇒ **没有任何 crate 编译那些面**。修法：认领成为强制存活根（`SourceScope::from_environment`
多收 `claims`，pipeline 在作用域之前读 `input.only`），钉子
`scope_tests::a_crate_claim_keeps_its_subtree_live_next_to_a_graft_slot`（含反证）。

**CI 的最后一道红**：`gh` 读不到日志的真因是本地 `~/.cache/gh` 写不进去（`XDG_CACHE_HOME=/tmp/gh-cache`
即可读）；唯一的红是 clippy **1.99** 的 `question_mark`（`shape_decl.rs` 的 `match` ⇒ `?`），本地 stable 是
**1.96** ⇒ 它熬过了每一次本地门禁。藏在它后面的还有两处只在 CI/全新检出才现形的装置缺陷：真实规模夹具的容器
面声明了 `handle_contracts` 却只定义 trait 不实现（用改动前的 CLI 复现 ⇒ 判明不是回归）；graft matrix leg 3
在任何 `check` 之前读复制过来的 `host/target/` 里的陈旧计划（全新检出里那文件不存在 ⇒ 每次都判红）。**六个
设备步骤在全新副本（`git archive HEAD`）里复跑全部通过**。

**落地门禁**：`cargo fmt --all -- --check` ✓ · `cargo test --workspace`（36 个二进制全绿）✓ · 两面
`clippy --all-targets -D warnings` ✓ · `tools/xirang-test` **十面全绿、无灰测试** ✓ ·
`RUSTDOCFLAGS='-D warnings' cargo doc --workspace --no-deps` ✓ · `tools/xirang-publish --check-table` ✓ ·
`tools/xirang-package-audit`（contents failed: none · verified 3 · failed: none）✓ · 六个设备 ✓。
CI：**10 个作业里 9 个 ✓**，唯一没绿的是 `verify (macos-latest, 1.96.0)` —— 它**根本没跑**（GitHub 侧
"The job was not acquired by Runner of type hosted"，macOS arm64 runner 容量），已单独重跑该格。
