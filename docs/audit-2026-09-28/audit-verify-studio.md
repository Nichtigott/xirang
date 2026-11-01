# 独立复核：Studio C 路（lane-studio）发现

- 复核对象：`docs/audit-2026-09-28/audit-lane-studio.md`（作者 lane-studio，355 行，39 条发现）。
- 复核人：kernel-auditor（内核片区审计员，与作者不同一条路、不同一份代码记忆）。
- 复核基线：同一工作树 `HEAD=cf0c378`。本轮只出报告，源码一行未改；唯一写入是本文件（按队长广播命名 `audit-verify-studio.md`，任务书里的 `verify-studio.md` 是旧名）。
- 手段（与作者不同）：我不使用作者的脚本，也不复述其 `codegraph_explore` 路径。我的手段是 ① 亲自 `read` 被引用的 studio 源文件全文/区段；② 用**可数探针**（`sed`/`grep -c`/`python3` 计数、`grep -rn` 调用点枚举）把作者的“数量类主张”变成命令输出；③ 用**既有内核单测**证明被依赖的内核不变量（不是作者的脚本）；④ 重跑标准测试基线确认树是绿的。
- 逐条判定用四值：**证实 / 部分证实 / 证伪 / 无法判定**。默认怀疑作者，凡与作者结论不符处我点名作者错在哪。
- 结论统计（复核的 12 条 CRITICAL/MAJOR + 22 条 MINOR 抽样）：

| 判定 | 条数 | 明细 |
| --- | --- | --- |
| 证实 | 30 | S-01、S-02、S-03、S-05、S-08、C-01…C-04；S-10…S-19、S-22…S-29（抽样）、C-05…C-08 |
| 部分证实 | 4 | S-04（事实成立、严重度偏高）、S-06（“多收”成立、“漏”的例子被反证）、S-07（行为成立、严重度偏高）、S-09（结论成立、两处枚举不准） |
| 证伪 | 0 | — |
| 无法判定 | 0 | 未复核项另列（S-20、S-21、C-09） |

- 严重度调整建议：**1 条应下调**（S-07 MAJOR→MINOR）、**1 条建议下调**（S-04 MAJOR→MINOR）、**1 条需改写判据但保留 MAJOR**（S-06）。其余 CRITICAL/MAJOR 维持原级。
- 作者漏掉而我顺手发现的同类问题：3 条（V-01…V-03，见 §3）。

---

## 1. CRITICAL / MAJOR 逐条复核

### S-01 · CRITICAL · 插件锁写盘路径能产出解析器拒绝的锁 → **证实**（严重度不变）

- 证据（命令 + exit code + 关键输出）：
  - `sed -n '255,270p;296,335p' studio/studio/app/mutations.rs`（exit 0）关键行：
    - `mutations.rs:258`：`format!("{source}|{framework}|{package}|{version}|{crate_name}|{checksum}|{mode}")` —— 7 字段，全部取自表单；
    - `mutations.rs:259`：`let existing = std::fs::read_to_string(&lock).unwrap_or_default();`
    - `mutations.rs:260`：`if existing.lines().any(|line| line.trim() == record)` —— 只挡**整行完全相同**；
    - `mutations.rs:265`：`let catalog = match PluginCatalog::parse(&existing)` —— 解析的是**既有内容**，不是 `existing + record`；
    - `mutations.rs:299`：`if source == "official" && !catalog.contains_record(&candidate)` —— 身份检查只在 official 分支；
    - `mutations.rs:332-333`：`let line = format!("{record}\n");` + `append_line(&lock, &line)`。
  - `grep -rn "PluginCatalog::parse" studio/src plugin-host/src`（exit 0）：studio 侧只有 `mutations.rs:265`（读既有锁）；宿主的准入读在 `plugin-host/src/admission.rs:76`。**写入方没有任何一处把候选记录交给内核校验。**
  - `sed -n '30,45p' studio/studio/app/state/plugin_field.rs`（exit 0）：`LABELS = ["source","framework","package","version","crate","checksum","mode"]` —— 触发所需的 `checksum` 与 `mode` 都是界面上可编辑的行；`overlay/plugin.rs:30` 的 `plugin.field == 0 || plugin.field == 6` 正是 source/mode 的 Enter 切换。
  - 独立内核锚点（既有测试，非作者脚本）：`cargo test -p xirang-core --offline --lib catalog::tests` → **exit 0**，`6 passed; 0 failed`，其中
    `test registry_core::plugin::catalog::tests::plugin_lock_parser_rejects_ambiguous_duplicate_identity ... ok`。
  - 身份五元组不含 checksum：`core/src/registry_core/plugin/catalog/catalog.rs:192-198` 的 `identity = (source, framework, package, version, crate_name)`。
- 我的因果链复核（三个独立事实拼起来）：① 同一身份、不同 checksum/mode 的第二条记录能通过 `260` 与 `299`（user 来源连 `299` 都不进）；② 内核把同一身份的第二条记录判为硬错误（既有测试证明）；③ 于是 Studio 报告 “Plugin selected”（`mutations.rs:347`）而宿主准入随后 `plugin-host/src/admission.rs:57-82` 解析失败（两份锁在 `:60-66` 拼接、解析在 `:76`、错误 `HostError::Policy` 在 `:77-80`）。作者这条**成立**，且它是本轮 studio 一路唯一真正破坏工件可用性的路径（失败方向是 fail-closed：插件面不可用、需手改文件，而不是放行未经检查的插件）。
- 严重度是否改变：**不变（CRITICAL）**。
- 备注：作者的两道闸描述准确；我额外确认了 `mutations.rs:265` 的解析结果只用于 official 检查，user 分支连这个已解析的目录都不使用——这加强了“写入方自己决定什么是合法锁”的判据。

### S-02 · MAJOR · 检视器行数两处真值（14 vs 12）→ **证实**（严重度不变）

- 证据：
  - `sed -n '118,153p' studio/studio/ui/panels.rs | grep -c '^            ('` → **12**（exit 0）；`sed -n '154,159p' … | grep -c '^            ('` → **2**（exit 0）。即 `Some` 分支 12 行、`None` 分支 2 行。
  - `sed -n '150,158p' studio/studio/app/lifecycle.rs` → `pub fn detail_field_count` 返回 `14`（Some）/ `2`（None）。
  - `grep -rn "detail_field_count" studio/src`（exit 0）四个消费点：`lifecycle.rs:130`、`keyboard.rs:151`、`pointer.rs:88`、`pointer.rs:118` —— 与作者列举一致（他把 lifecycle 的 clamp 与三个交互点分开描述）。
  - `panels.rs:161`：`let selected = app.details_selected.min(values.len().saturating_sub(1));` —— 渲染侧确实只在那一处自钳。
- 严重度是否改变：**不变（MAJOR）**，但需在报告里写清影响边界：错位只影响光标/高亮（↓ 两次后画面无高亮），不产生数据损坏；作者把它记 MAJOR 的理由是“两份行清单必须同源”，可接受。
- 备注：作者说“渲染器只在 `ui/panels.rs:161` 自钳一次”准确。我的可数探针纠正的是我自己的第一版 grep（多行元组会被漏数），最终两个数字与作者一致。

### S-03 · MAJOR · reload / hot reload 不重校验已装入的 trace 与 MIR → **证实**（严重度不变）

- 证据（代码阅读佐证 + 调用点枚举）：
  - `grep -rn "install_trace\|mir_graph\|trace_mismatch\|unresolved_nodes" studio/src`（exit 0）：
    - `install_trace` 只有 `app/lifecycle.rs:90` 一个调用点（`App::load` 内）；
    - `trace_mismatch`/`unresolved_nodes` 只出现在 `app/trace.rs:106/121/184`（即 `install_trace` 那条链上）；
    - `mir_graph` 的写入只有 `app/lifecycle.rs:213`（`load_mir_snapshot`），读取在 `graph_queries.rs:132/172`、`app.rs:153`。
  - `sed -n '108,142p;162,183p' …/lifecycle.rs`（exit 0）：`poll_hot_reload` 与 `reload` 的函数体里只有 `self.registry = registry`、`reload_error`、`selected`、`details_selected`、`event`、`refresh_open_graft()`，**没有** trace 判定、也没有 `mir_graph = None`。
  - `app/trace.rs:97-105` 的文档自己写明“每个已记录节点是否仍存在于本快照……是唯一真正的‘不同构建’检测器”；`trace_is_live`（`studio/studio/app/trace.rs:154`）只等值比较 `TraceStatus::Loaded`。
  - `state/misc.rs:145-149`：`push_call_ref` 的 `file` 用 `registry.find(node).map(…).unwrap_or("")` —— 节点消失后确实产出空 `file`（作者该句成立）。
- 严重度是否改变：**不变（MAJOR）**（证据诚实性问题：界面继续宣称 LIVE 与 `?`，而有效性前提已被替换掉）。
- 备注：作者的“侧门”表述准确。补一条我从 `grep` 得到的加强证据：`reload()` 也没有把 `mir_graph` 清掉，而 MIR 是**编译器快照**——`rebuild` 后旧 MIR 被继续当作同一次构建的候选，比作者写的“旧 MIR 继续标 ?”更具体。

### S-04 · MAJOR · 渲染期改写 `App` → **部分证实**（事实全部成立；严重度建议下调 MINOR）

- 证据：
  - `sed -n '60,70p' studio/studio/ui/ui.rs` → `pub fn draw(frame: &mut Frame<'_>, app: &mut App)`。
  - `grep -n "fn draw_workspace\|fn draw_tree\|fn draw_details" studio/studio/ui/panels.rs` → `51: …app: &mut App`、`66: …app: &mut App`、`118: …app: &App`（签名不统一成立）；`panels.rs:115` `app.tree_offset = state.offset();`。
  - `grep -n "graph_flow" …/ui/graph/nodes.rs …/app/app.rs …/app/pointer.rs …/app/lifecycle.rs` → 绘制期写入点 `studio/studio/ui/graph/nodes.rs:134/143/160`，读取 `studio/studio/ui/graph/nodes.rs:130/165`、`pointer.rs:151`，初始化 `lifecycle.rs:62`，**没有任何清空点**。
  - `sed -n '6,24p;47,54p' studio/studio/ui/overlay.rs` → `let Some(overlay) = app.overlay.clone()`，随后只写回 `current.offset = offset`（`studio/studio/ui/overlay.rs:52`）。
- 我方判据：**渲染期写状态本身今天没有产生错误结果**——`draw_search` 只返回一个 `offset`，写回的是同一个字段；热区本就是 immediate-mode 设计（作者也承认）。真正还站得住的是“签名不统一 + clone 后只写回一个字段”的**未来脆性**，属维护性风险而非现行缺陷。
- 严重度是否改变：**建议下调为 MINOR**（作者标 MAJOR 的理由是职责边界，但没有给出今天会错的行为；判据要求 MAJOR 需“明确行为错误或维护性硬伤”，这一条我判在边界上）。若保留 MAJOR，请把判据改成“`draw` 的 `&mut App` 阻止了任何 `&App` 渲染测试/缓存”，避免与“热区设计”混在一起。

### S-05 · MAJOR · `search_rows` 无缓存，列表模式一帧至少三次全量重扫 → **证实**（严重度不变）

- 证据（可数）：
  - `sed -n '58,86p' studio/studio/ui/search.rs`（exit 0）：列表分支一帧内依次调用 `draw_search_lane`（`columns[1]`）、`draw_search_list`（`columns[0]`）、`draw_search_preview`（`columns[2]`）——**恰好三次**；三者的首行（`studio/studio/ui/search/detail.rs:163`、`studio/studio/ui/search/results.rs:29`、`studio/studio/ui/search/detail.rs:15`）各自调用 `app.search_rows(...)`。
  - `grep -rn "search_rows(" studio/src`（exit 0）除上述三处外还有 `ui/graph.rs:102`、`pointer.rs:283`、`overlay/search.rs:146/151`（作者列举一致）。
  - `sed -n '18,136p' studio/studio/app/search_queries.rs`：`search_rows` → `source_symbol_rows` 对 `self.registry.depth_first()` 每个节点 `std::fs::read_to_string(source)`（`:41`）+ `function_symbols`（`:55`）+ 逐行扫描（`:74-133`）；文件 142 行内没有任何备忘字段（与 `call_tree_queries.rs` 的 `CallTreeMemo` 对照）。
- 严重度是否改变：**不变（MAJOR）**（同步 I/O 与帧率绑定，且同仓已有 `CallTreeMemo` 先例）。
- 备注：作者“至少三次”其实是“列表模式**恰好**三次”，比其表述更强，我按更强者记。

### S-06 · MAJOR · 手写第二套符号词法器 → **部分证实**（“多收”证实；“漏”的例子被作者自己引用的测试反证）

- 证实部分：
  - `sed -n '74,133p' …/search_queries.rs`（exit 0）：前缀表恰为 13 个，末端含 `let `（`:95`）；`functions.iter().any(|f| f.line == line_index+1)` 只跳过“函数首行”（`:75-79`）。
  - `sed -n '28,50p' studio/studio/ui/search/results.rs`：`let marker = if row.function.is_empty() { "󰈙" } else { "ƒ" };` —— `let` 绑定行会带 `ƒ`。
  - `sed -n '140,180p' …/app/overlay/search.rs`：`KeyCode::Enter` 把 `row.function` 放进 `search.center_function` 并 `graph_mode = true` —— `let` 行可按 Enter 推进调用图。
  - `sed -n '9,23p' search_queries.rs`：文档写 “Flat, **deduplicated** rows”，实现是 `dedup_by`（只消相邻）——与 C-03 一致。
- **反证部分（作者错处）**：作者用 `app/tests/source.rs:26-53` 的用例 `function_index_accepts_qualified_visibility_and_impl_methods` 证明“内核认 `pub(crate) async fn load` 与 impl 里的 `unsafe fn render`”，并据此说 Studio 的第二套前缀“漏”这两类。但该用例断言的正是**内核 `function_symbols` 认它们**（`app/tests/source.rs:28-49`：`pub(crate) async fn load`、`items == ["load","render"]`、`functions[0].signature.contains("pub(crate) async fn load")`）。也就是说：这两类行会由 `search_queries.rs:56-73` 的**内核函数循环**产出结果行，`signature` 又是“行首到 `{` 的整段文本”，凡是 needle 命中该行的情形大多已被覆盖——作者的“漏”举错了例子。真正的漏只发生在内核也不索引的 item 上（例如无函数体的 trait 方法声明 `pub(crate) fn f(&self);`，core 的 `function_symbols` 因找不到 `{` 而跳过），这条更窄、需要单独给输入。
- 严重度是否改变：**保留 MAJOR，但必须改写判据**——把“漏 `pub(crate)`/`unsafe fn`”换成“无 body 的 item 声明（trait 方法等）”；“多收 `let`”那半边可以原样保留（已证实且用户可见：`ƒ` 标记 + Enter 可跳）。

### S-07 · MAJOR · `submit_graft` 无条件覆盖编辑器失败消息 → **证实（行为）**，严重度**建议下调 MINOR**

- 证据：`sed -n '119,162p' …/app/graft.rs`（exit 0）：`studio/studio/app/graft.rs:127` 取 `declaration`，`:128` `self.open_editor_file(path.clone(), 1);`，`:129` `self.event = match &state.declaration { … }`（无分支检查编辑器结果）。`sed -n '457,466p' …/app/support.rs`：`open_editor_file` 在 `!path.is_file()` 时写 `self.event = "Editor failed: source file not found at …"` 并 `return`（不设 `editor_request`）。
- 我方判据：**计划文件确实已经写好**（`with_selected_project(create_external_graft)` 成功才会进入 `Ok(plan)` 分支），被覆盖的只是“编辑器没打开”这一条次要反馈；主操作成功、用户会立刻发现编辑器没开。按本轮的严重度口径（MAJOR＝明确行为错误或维护性硬伤），我判它更接近“错误反馈缺失”的 MINOR。
- 严重度是否改变：**建议下调 MINOR**（判据：主操作成功，覆盖的是次要消息；修复是一行 match）。

### S-08 · MAJOR · 插件入口行用全文子串匹配 → **证实**（严重度不变）

- 证据：`sed -n '316,325p' …/app/mutations.rs`（exit 0）：`let anchor = format!("\n#[allow(unused_imports)]\nuse {crate_name} as _;\n");`、`let entry_text = std::fs::read_to_string(&entry).unwrap_or_default();`、`if !entry_text.contains(&format!("use {crate_name} as _;")) && let Err(error) = append_line(&entry, &anchor)`。
- 我方判据：`contains` 不看行、不看是否被注释；命中即跳过 append，final `format!("Plugin selected: {package} ({source}, {mode})")`（`studio/studio/app/mutations.rs:347`）。此时锁里有记录而入口文件没有生效的 `use <crate> as _;`，宿主不会链接该 crate —— 与作者结论一致，且这正是 S-01 的“写盘成功但语义未达成”同族。
- 严重度是否改变：**不变（MAJOR）**。

### C-01 · MAJOR（注释轴） · `retreat_graph_focus` 注释与实现不符 → **证实**（严重度不变）

- 证据：`read` 全文 `studio/studio/app/navigation.rs`：`:155-163` 与 `:165-173` 两个函数体逐字相同（都是 `match graph_focus { 0 => 1, _ => 0 }` + `outline_focus = graph_focus == 0`），`:166-167` 注释写 “BackTab walks the same two panes the other way.”。与 S-17 同处两轴，作者已互相指针。
- 严重度：**不变**（维护者判据①：注释描述的行为不存在）。

### C-02 · MAJOR（注释轴） · `writers.rs` 模块文档自称拥有全部写入 → **证实**（严重度不变）

- 证据（可数）：`grep -c "std::fs\|OpenOptions\|create_project\|add_module_from_face\|edit_module_face" studio/studio/app/writers.rs` → **0**（exit 1，无命中）；同模式在 `mutations.rs` → **8**；`wc -l`：`writers.rs` 42 行、`mutations.rs` 395 行。文档在 `writers.rs:1-2`。
- 严重度：**不变**（这是 S-01 的成因之一，作者判得对）。

### C-03 · MAJOR（注释轴） · `search_rows` 文档称“已去重” → **证实**（严重度不变，但请保留作者的“今天无可复现输入”限定）

- 证据：`search_queries.rs:9`（`Flat, deduplicated rows for one search query.`）与 `:20-22`（`dedup_by(|l, r| l.node == r.node && l.function == r.function && l.line == r.line)`）——`dedup_by` 只合并**相邻**元素。
- 严重度：**不变**（作者已声明这是“文档承诺 > 代码保证”的隐性依赖，不是现存 bug；这个自我限定我复核后同意）。

### C-04 · MAJOR（注释轴） · `support.rs` 文档声明的范围小于实现 → **证实**（严重度不变）

- 证据：`support.rs:1-2` 只有 “Shared interaction geometry and editor helpers.”；`grep -n` 函数清单显示 `resolve_project`（157）、`resolve_project_from`（196）、`usable`（229）、`host_manifest`（251）、`cargo_rustc_mir`（277）、`bin_target_names`（332）等 I/O 与进程派生内容；另有 `use super::*;` 出现在**文件中部**（`:376`）与第二段 `impl App`（`:378`）。
- 严重度：**不变**（判据①的弱形式；作者对“谁 import 它就连带 `std::process`/`std::env`”的判断成立）。

---

## 2. MINOR 抽样复核（27+5 条中抽 22 条，>1/3）

判定口径同上；未逐条展开的只给一行结论与证据。

| id | 结论 | 证据（命令/读数） | 严重度 |
| --- | --- | --- | --- |
| S-09 | **部分证实** | `studio/studio/app/navigation.rs:79-85` 的目录过滤确实只认 `.rs / Cargo.toml / Cargo.lock / official.lock`（无 `user.lock`）✓；但作者对两处清单的**枚举都不准**：`xirang-dev.rs:273-276` 的集合是 `Cargo.toml / Cargo.lock / build.rs / official.lock / user.lock`（含 Cargo.toml/Cargo.lock，作者漏写），而 `source_stamp` 在顶层 `stamp_file` 里还额外钉 `build.rs`（`studio/studio/app/navigation.rs:24-29`）。结论（user.lock 不对称 → 戳不失效 → 界面不刷新）成立 | 不变 MINOR |
| S-10 | 证实 | `studio/studio/ui/graph/nodes.rs:183-194` `flow_key` = `"{focus}:{nodes}:{edges}"`；`grep -n graph_flow` 全仓无清空点（只有 `lifecycle.rs:62` 初始化） | 不变 |
| S-11 | 证实（附一处表述纠正） | `ui/overlay.rs:8-22` 与 `:30-44` 两段各 15 个赋值、顺序不同；`HotZones` 实际有 **18** 个字段（`hot_zones.rs:15-70`），另 3 个（`tree_area`/`details_area`/`workspace_area`）由 `panels.rs:52,60,61` 每帧设置——“无遗漏”结论成立，但作者写的“全部 15 个 HotZones 字段”不精确 | 不变 |
| S-12 | 证实 | `plugin_field.rs:33` `COUNT = 7`、`new_project_field.rs:27` `COUNT = 3`；裸数字确实存在：`overlay/plugin.rs:29-30`（`.min(6)`、`== 6`）、`overlay/new_project.rs:28-29`（`.min(2)`、`== 2`）、`pointer.rs:294`（`visible_row == 2`）、`pointer.rs:335`（`== 0 || == 6`） | 不变 |
| S-13 | 证实 | `state/misc.rs` 181 行含 `ReloadError`/`Overlay`/`CallRef`/`CallTreeView`/`CallTreeMemo`/`push_call_ref`/`source_path_for`；`source_path_for`（`:161-181`）调 `package_root()`（`:162`）并做 `path.is_dir()`（`:172`）/`attached.is_file()`（`:176`） | 不变 |
| S-14 | 证实 | 同 C-02：`writers.rs` 42 行、写点 0；`mutations.rs` 8 个写点 | 不变 |
| S-15 | 证实 | 同 C-04；另核到作者说的“第二个 `impl App` + 文件中部 `use super::*;`”位置（`:376`/`:378`）准确 | 不变 |
| S-16 | 证实 | `lifecycle.rs:85-88` 的 `Err` 分支里再次 `load_registry().unwrap_or_else(|_| Registry::root())` | 不变 |
| S-17 | 证实 | 同 C-01（两份函数体相同）；`step_graph_cursor` 的 `_ => TreeStep::Right` 在 `studio/studio/app/navigation.rs:141-146` | 不变 |
| S-18 | 证实 | `ui/status.rs:16-17` `starts_with("Warning:")` 或 `to_ascii_lowercase().contains("fail")`，每帧一次分配 | 不变（作者补的“同仓有 `ReloadError` 结构化先例”成立） |
| S-19 | 证实 | `keyboard.rs:160-164` `self.handle_key(KeyEvent::from(KeyCode::Char('e')))`；`:165` 的 `Enter`/`Char(' ')` 是折叠 | 不变 |
| S-22 | 证实 | `grep select_project` 在 `app/tests/`：`studio/studio/app/tests/call_tree.rs:27`、`studio/studio/app/tests/evidence.rs:62`、`studio/studio/app/tests/graph.rs:18/66/132/181` 为夹具式三行；`edit/forms/project/source/...` 的其它调用是自建临时工程（作者已排除） | 不变 |
| S-23 | 证实 | `wc -l tests/project.rs` = 562；`grep -n "cargo" project.rs` → `Command::new("cargo")`（`:342`）与 `cargo_rustc_mir(…)`（`:522`）等真编译用例 | 不变 |
| S-24 | 证实 | `mutations.rs:259`、`:319` 两处 `read_to_string(...).unwrap_or_default()` | 不变 |
| S-25 | 证实 | `support.rs:296` `contains("no library targets")`；`:362-373` 的 `bin_target_names` 只按 `target["kind"]` 过滤 `bin`，**没有** `manifest_path == manifest` 过滤 | 不变 |
| S-26 | 证实 | `navigation.rs` 1-96 行是 `source_stamp`/`stamp_file`/`stamp_directory`，导航实现从 `:103` 起 | 不变 |
| S-27 | 证实 | `submit_add` 只 `register_snapshot_batch` + “press r to reload”（`mutations.rs:141,145`）；`submit_edit` 直接 `self.reload()`（`:195`）；`grep last_source_stamp` 只有 `lifecycle.rs:68/114/117` 与 `call_tree_queries.rs:83/94` —— 写入路径都不更新戳 | 不变 |
| S-28 | 证实 | `interaction.rs:18` 显式忽略 `Paste/Resize/Focus*` | 不变 |
| S-29 | 证实 | 读路径 `studio/studio/app/graft.rs:167`、`:236` 用 `with_authoring_context`；写路径 `studio/studio/app/graft.rs:111`、`:180`、`:200` 用 `with_selected_project` | 不变 |
| C-05 | 证实 | `lifecycle.rs:185-189` 两段摘要叠在 `load_mir_snapshot_report` 上，`load_mir_snapshot`（`:197`）无文档 | 不变 |
| C-06 | 证实 | `status.rs:37-38` 的 “exactly the keys … handles” 与 `status.rs:48` 页脚串对照：页脚无 `Tab`（`keyboard.rs:132-138`）、无 `j`/`k`（`:139-153`）、无空格，且 `Enter fold` 在 Details 焦点下其实是编辑（`:160-164`） | 不变 |
| C-08 | 证实 | `hot_zones.rs:40-45` 两个字段的注释自认 “no screen sets it yet”；赋值只在 `ui/overlay.rs:21-22`、`:35-36` 的每帧清零（我按 `grep action_validate_area\|action_edit_area` 全仓确认没有真实设置点） | 不变 |

未复核（诚实列出）：**S-20**（add/edit 两个 overlay 的状态机逐支重复）、**S-21**（四个表单页按钮区四份拷贝）、**C-09**（同一条事实在注释/文案/测试名各说一遍）。这三条是结构性计数/信噪比判断，我没有独立重数，留给下一道闸或作者自证。

---

## 3. 作者漏掉、我顺手发现的同类问题（单独一节）

### V-01 · MINOR（S-01 同族）· 锁追加假定“既有文件以换行结尾”

- 位置：`studio/studio/app/mutations.rs:332-333` + `:389-395`（`append_line`）。
- 现象：`append_line` 只把给定字节写到末尾；锁路径传的是 `format!("{record}\n")`，但没有任何地方保证 `existing` 以 `\n` 结尾。若锁文件是被编辑器去掉末尾换行的（或手工 `printf` 写的），新记录会**拼到最后一行尾部**，产生一条既不是旧记录也不是新记录的畸形行 → 下一次 `PluginCatalog::parse` 报 “must contain 7 or 10 fields”，与 S-01 一样是“Studio 写出宿主读不进的锁”，只是触发条件不同。
- 对照：入口文件那条路径**想到了**这件事——`anchor` 以 `\n` 开头（`mutations.rs:317`），所以入口不会粘连；锁这条没有等价保护。
- 最小修复：`if !existing.is_empty() && !existing.ends_with('\n') { 补一个换行 }`（或在 `append_line` 内保证行终止），并把它并进 S-01 的“交给内核唯一写入器”修复。
- 复核手段：把 `user.lock` 的最后一行换成不带换行的等价内容后提交一条新插件，断言事件行不是 “Plugin selected” 且该文件仍能被 `parse` 接受。

### V-02 · MINOR（C-06/S-19 同族）· 页脚宣传的首屏键在根节点上是静默空操作

- 位置：`studio/studio/app/keyboard.rs:38-43`、`:129` 与 `studio/studio/ui/status.rs:48`。
- 现象：`d`（`:38-42`）、`e`（`:43`）、`g`（`:129`）都带 `self.selected != self.registry.id()` 守卫；选中根时按键**什么也不做、连事件行都没有**，而页脚仍写 “e edit   d delete   g graft”。这与作者 C-06 的“页脚是可验证的错误断言”同族，但比 `Tab`/`j`/`k` 更严重：那三个是漏列，这三个是列出后无效。
- 最小修复：根节点选中时给一条事件行（例如 “the root has no editable face”），或页脚按焦点/选择态变化（哪怕只把 `e/d/g` 换成 “(face)” 后缀）。
- 复核手段：`app.selected = app.registry.id()` 后分别按 `d/e/g`，断言 `app.event` 非空。

### V-03 · MINOR（S-24 同族，且后果是数据丢失）· 编辑表单读取失败被当成“字段为空”，提交即抹掉两个字段

- 位置：`studio/studio/app/keyboard.rs:52-56`。
- 现象：进入 Edit 时用 `std::fs::read_to_string(source_path_for(file)).ok().map(declaration_contract_paths).unwrap_or_default()` 填充 `handle_contracts`/`part_contracts`。读失败（权限/是目录/编码）与“这份面没有写 contract”得到同一个结果：两个字段为空。用户在表单里改任意别的字段并提交，`edit_module_face` 会按空值重渲染这两个字段 → **源文件里的 contract 路径被静默删除**（比 S-24 的锁追加更严重：那一条只是写不进去，这一条是抹掉已有内容）。
- 最小修复：读失败时禁用 Edit 表单或把事件行置为失败（与 S-24 的 `Err(e) if e.kind() == NotFound` 分支同一个模式），并在空值提交时保留原值。
- 复核手段：把某个面的源文件 `chmod 000`（或换成目录）后按 `e` 提交，断言事件行是失败且源文件的 `handle_contracts:` 行未变。

---

## 4. 复核手段与自检（我自己的，不复用作者的）

1. 锚点自检：我对本文件里的每一处 `file.rs:NNN` 与每一处 `` `token`（`file.rs:NNN`） `` 配对都跑了一条独立脚本（`python3`：按后缀解析仓库路径 → 检查文件存在、行号 ≤ 总行数 → 再断言配对 token 出现在所引行区间内），结果见交付消息。
2. 测试基线：`cargo test -p xirang-studio --offline --all-features` → **exit 0**（`launch` 4 passed / `0 failed`，doc-tests 0）；`cargo test -p xirang-studio --offline --all-features --lib` → `91 passed; 0 failed` —— 与作者“查了、干净”第 11 项的 91 一致，说明作者的发现确实都不在现有测试覆盖内（不是破损的树）。
3. 内核锚点：`cargo test -p xirang-core --offline --lib catalog::tests` → `6 passed; 0 failed`（S-01 依赖的解析器不变量）。
4. 我的复核未做任何写入（`git status` 里 studio/ 的改动与本次会话无关；本会话只写本报告）。

---

## 5. 复核总评

- 作者的这条 CRITICAL 与 7 条实现轴 MAJOR **全部有真实代码依据**，没有一条是“看起来像”的推测；4 条注释轴 MAJOR 全部成立，且作者主动标注了 C-01…C-04 与 S-17/S-14/S-06/S-15 的“两轴同处”，这种自我去重值得保留。
- 作者的主要错处有三类，都不影响结论方向：
  1. **举错例子**：S-06 的“漏”用了 `pub(crate)`/`unsafe fn`，而这两个形状恰被作者自己引用的用例证明内核认它们；
  2. **枚举不全/不准**：S-09 对两个 watcher 集合的枚举都漏项（结论仍成立）；S-11 把 `HotZones` 的 18 个字段写成 15（覆盖面结论仍成立）；
  3. **严重度略高**：S-04（无今日错误行为）、S-07（主操作成功）我建议降到 MINOR。
- 我另外找到 3 条同族问题（V-01 锁追加、V-02 根节点静默空操作、V-03 编辑读失败导致数据丢失），其中 V-03 的后果（抹掉已写字段）比作者列的多数 MINOR 更值钱，建议并入下一轮修复清单。
