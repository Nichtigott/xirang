# lane-studio — 审计 C：Studio（ratatui 创作与检视面）

范围：`studio/src`（71 文件 / 12209 行，`wc -l` 实测）、`studio/tests/`。
基线：工作树 `HEAD=cf0c378`（120 项未提交改动，未做任何 checkout/restore/stash）。
本轮只出报告，源码一行未改；**唯一写入**是本文件 `docs/audit-2026-09-28/lane-studio.md`。

## 0. 方法与证据基线

- 全程 `read` 逐个文件（`file:line` 均为本人读过的行号），结构性问题用 `codegraph_explore` 交叉核对内核侧规则：`core/src/registry_core/source/source.rs`、`core/src/registry_core/plugin/catalog/catalog.rs`、`core/src/registry_core/authoring/face_field.rs`、`plugin-host/src/admission.rs`。
- 亲自通读的 Studio 文件：`lib.rs`、`main.rs`、`bin/xirang-dev.rs`、`studio.rs`、`terminal.rs`；`app/{app,lifecycle,keyboard,keyboard_overlay,interaction,pointer,mutations,writers,support,namespace,hot_zones,trace,graft,navigation,call_tree_queries,search_queries,graph_queries,source_index,tests}.rs` + `app/overlay/*`（7）+ `app/state/*`（8）；`ui/{ui,panels,overlay,status,mark,forms}.rs` + `ui/forms/*`（6）+ `ui/graph{,/nodes,/data,/node_graph}.rs` + `ui/search{,/query,/results,/detail}.rs`；`tests/launch.rs`、`tests/project.rs`（全文）、`tests/{call_tree,evidence,forms,graph,fixtures,source,edit,trace_ingest,navigation}.rs`（各组头部/全部用例名，逐条判读的用例另在文内点名）。
- 验证命令（`--offline`）：
  - `cargo test -p xirang-studio --offline --all-features` → 通过（tail：lib 91 passed / `xirang-dev` 7 passed / `launch` 4 passed，0 failed）。**默认特性下夹具测试被跳过**，因此用 `--all-features` 才覆盖 `prototype-fixtures` 那一半。
  - 四条门禁基线（fmt / `cargo test --workspace` 740 passed / clippy / `--check-table`）由 executor 实测全绿并已转达，本报告不再自证，也不据此推断任何结论。
  - captain 转达 `studio/tests/fixtures/node-editor` 的 5 个源文件在工作树里没有 `mod` 声明属**设计如此**（夹具从不编译）：我独立核实了该事实，并且**没有**把它记为挂载缺陷；它作为“存在方式”的设计问题另记为 S-30。

结论：**39 条 —— 1 CRITICAL / 11 MAJOR / 27 MINOR**（§2 注释轴另计 4 条 MAJOR + 5 条 MINOR，其中 C-01…C-04 与 S-17/S-14/S-06/S-15 同处两轴）。CRITICAL 是插件锁的写盘路径——Studio 能静默写出它自己的解析器（以及宿主准入）拒绝读的锁文件。S-30 是 captain 转达的夹具存在方式，按“设计问题（非挂载缺陷）”记。

---

## 1. 发现

### S-01 · CRITICAL · 可靠性/数据损坏 · `studio/studio/app/mutations.rs:257-346`

- 现象：`submit_plugin` 手写锁文件格式并直接 append：记录串在 `mutations.rs:258` 拼成 7 字段 `{source}|{framework}|{package}|{version}|{crate_name}|{checksum}|{mode}`，`mutations.rs:332-333` 用 `append_line` 追加。写入前只有两道闸：整行字符串相等（`mutations.rs:260`）与「official 必须已在受信锁里」（`mutations.rs:299`）。两者都拦不住**同一身份、不同 checksum/mode** 的新行：
  - 整行比较要求七个字段的序列化字节完全相同，checksum 或 mode 任一不同即放行；
  - `contains_record` 只对 `source == "official"` 调用（`mutations.rs:299` 的 `if source == "official" && ...`），`user` 来源完全绕过。
  - 而 `PluginCatalog::parse` 把「身份五元组 `(source, framework, package, version, crate_name)` 重复」判为**硬错误**：`core/src/registry_core/plugin/catalog/catalog.rs:192-207`（checksum/mode 不在身份元组里），内核自己的用例 `catalog.rs:325-332` 正是用 `sha256:a` / `sha256:b` 这一对证明它会失败（`duplicates package identity`）。
  - 触发路径是界面上两个可编辑字段（`plugin_field::CHECKSUM`、`plugin_field::MODE`）：同一个 user 插件改 checksum、或只按一次 Enter 把 mode 从 `extension` 切到 `replacement`，再按两次 `s`，`mutations.rs:333` 成功、`mutations.rs:347` 报 “Plugin selected”。
- 判据：`.xirang/plugins/{official,user}.lock` 是**宿主工件**。宿主的准入读它：`plugin-host/src/admission.rs:57-82` 把两份锁拼起来解析，失败即 `HostError::Policy("invalid plugin lock under …")`——插件面此后完全不可用，Studio 自己的下一次 `submit_plugin` 也会在 `mutations.rs:265-271` 报 “invalid lock”，而界面没有任何手段修回来（只能手改文件）。写入方在**写的那一刻**没有把解析器会拒绝的输入挡掉，且报告为成功。
- 最小修复方向：把「追加一条记录」交回内核唯一的解析/校验入口——例如新增 `PluginCatalog::push_record`/`append_record`，内部先跑 `parse` 的同一套身份检查（复用 `accounts_for` 的身份五元组），或写入前先 `PluginCatalog::parse(format!("{existing}{record}\n"))` 并要求 `Ok`；`submit_plugin` 只组合候选串、不再自己决定什么是合法锁。
- 复核手段：新增一条 Studio 级测试（结构与 `app/tests/forms.rs:154-216` 同形）：先写入一条 user 记录，再以「同 framework/package/version/crate、不同 checksum」提交 → 断言第二次被拒绝且锁文件字节未变；再加一个 `mode: extension → replacement` 的用例。内核侧已有 `plugin_lock_parser_rejects_ambiguous_duplicate_identity` 作为不变量锚点。

### S-02 · MAJOR · 逻辑/一致性 · `app/lifecycle.rs:152-158` × `ui/panels.rs:119-159`

- 现象：检视器行数有两个真值。`detail_field_count()` 返回 **14**（有选中面）/ 2（根），而渲染器 `draw_details` 的 `Some` 分支实际画出 **12** 行：name、kind、node、path、parent、preset / parts、handle interfaces、parts interfaces、declared、registration rule、dependency admission、exports（`ui/panels.rs:119-153`；`None` 分支 2 行，与返回值一致）。三个消费点都用 14 当上界：`keyboard.rs:150-151`（↓）、`pointer.rs:87-88`（滚轮）、`pointer.rs:117-118`（点击），渲染侧只在 `ui/panels.rs:161` 自钳一次。
- 判据：选中一个注册面后连按两次 ↓（或滚两次、或点第 13 行）会把 `details_selected` 推到 12/13，画面上不再有任何高亮——按键有效、界面无反应，且真实行数一变（加一行）就立刻扩大错位。两份行清单必须是同一个来源。
- 最小修复方向：抽出 `fn detail_rows(app: &App) -> Vec<(&'static str, String)>`，`draw_details` 与 `detail_field_count()` 都从它取 `len()`；顺带把 `keyboard.rs`/`pointer.rs` 的 `detail_field_count() - 1` 统一成 `saturating_sub(1)`（现已如此）。
- 复核手段：新增测试断言 `app.detail_field_count() == detail_rows(&app).len()`（选中面与根两种状态各一次）；或在 `ui.rs` 的 `TestBackend` 用例里比较渲染出的 `›` 标记行数与 `detail_field_count()`。

### S-03 · MAJOR · 逻辑/证据诚实 · `app/lifecycle.rs:108-183`（配合 `app/trace.rs:56-89`）

- 现象：`install_trace()` 只在 `App::load()` 调一次（`lifecycle.rs:90`），身份检查（namespace、root、每个记录节点能否在本快照解析）只在那时跑。此后 `poll_hot_reload`（`lifecycle.rs:108-133`）与 `reload`（`lifecycle.rs:162-183`）**整体替换 registry，但不重跑 `trace_mismatch`/`unresolved_nodes`，也不清 `mir_graph`**。`mir_graph` 只在 `lifecycle.rs:213` 被写、在 `graph_queries.rs:132/172` 与 `app.rs:153` 被读。
- 判据：这两条替换路径会改变「本快照有哪些节点」，而 trace 与 MIR 的有效性前提正是「记录指向的节点仍在本快照」——`trace.rs:97-105` 自己把这条称为唯一真正的“不同构建”检测器，`trace.rs:152-170` 又把 `Loaded` 渲染成 `LIVE`。于是一次外部编辑删掉一个被记录的注册面、或一次 `rebuild` 之后的 hot reload，会话会继续宣称 `LIVE`、继续用旧 MIR 给边标 `? compiler-only`，而 `push_call_ref` 会产出 `file: ""` 的引用。这正是身份闸门要防的“宣称并不持有的证据”，只是从 reload 这个侧门重新打开。
- 最小修复方向：把「registry 被替换」收敛成一个函数（`reload` 与 `poll_hot_reload` 都走它），在其中重跑 `read_trace` 的判定（或至少 `unresolved_nodes` 非空即降级为 `Mismatch`）并按需清 `mir_graph`。
- 复核手段：测试——加载带 artifact 的临时工程（`tests/trace_ingest.rs:88-130` 已有现成夹具），删掉被记录的那个 face，调 `app.reload()`，断言 `trace_status != Loaded` 或 `runtime_trace.call_edges()` 不再把该边算作 `Live`。

### S-04 · MAJOR · 职责边界 · `ui/ui.rs:66`、`ui/panels.rs:52/60-61/115`、`ui/overlay.rs:7/52`、`ui/graph/nodes.rs:130-165`

- 现象：渲染不是只读的。`draw(frame, app: &mut App)`，并在绘制期写 `App`：`panels.rs:52` 与 `60-61` 写 `hot.workspace_area/tree_area/details_area`，`panels.rs:115` 把 `ListState::offset()` 写回 `app.tree_offset`，`overlay.rs:7` 先把整个 overlay `clone()` 出来、`overlay.rs:52` 再把 `current.offset` 写回 overlay，`ui/graph/nodes.rs:130-165` 直接改 `app.graph_flow`（缓存控件的重建/重盖/视口保存）。同一批函数签名还不统一：`draw_details(&App)`、`draw_tree(&mut App)`、`draw_search(_, &mut App, _)`。
- 判据：热区是 immediate-mode 的既有设计（`app.rs:112-114` 注释、`studio/studio/app/hot_zones.rs:6-13` 说明“Every draw recomputes these”），这条可以留；但「渲染期写回状态」把渲染与事件处理绑在一个可变引用上，`overlay.rs` 的 clone→写回只写回**一个**字段这一点尤其脆：任何第二个字段在绘制期被改都会静默丢失（clone 的副本被丢弃），而这件事没有任何编译期约束。
- 最小修复方向：`draw` 收 `&App` + `&mut HotZones`（或一个 `RenderCache`），把 `offset` 与 `tree_offset` 作为**返回值**交给事件循环写回；`draw_search` 已经从“返回值 + 调用方写回”的形态退化成内部写回，退回返回值即可。
- 复核手段：把 `draw` 的 `app` 参数改成 `&App` 后能编译即证明写入点已收敛；再加一条“同一帧两次 draw 只改变 RenderCache”的断言测试。

### S-05 · MAJOR · 性能/架构 · `app/search_queries.rs:18-136`，调用点 `ui/search/results.rs:29`、`ui/search/detail.rs:15/163`、`ui/graph.rs:102`、`app/pointer.rs:283`、`app/overlay/search.rs:146`

- 现象：`search_rows(query)` 每次调用都遍历整棵注册树、对每个节点的源文件 `read_to_string`（`search_queries.rs:41`）并对全文做 `function_symbols` 词法扫描（`search_queries.rs:55`）与逐行前缀扫描（`search_queries.rs:74-133`）。它没有备忘：`call_tree_view` 有 `CallTreeMemo`（按 `last_source_stamp` 失效），搜索侧什么都没有。列表模式下**一帧至少三次**调用（结果列表、RELATION 栏、SELECTED SYMBOL 预览），数据页在 `ui/graph.rs:102` 还有一次，按键路径 `overlay/search.rs:146`（↓）与点击路径 `pointer.rs:283` 各再一次。
- 判据：这是 TUI 事件循环里的同步 I/O + 词法重扫，随工程规模线性增长且与帧率绑定；“一帧问同一件事三次”在代码里是可见的事实，而不是推测。规模上它和 `CallTreeMemo`（`call_tree_queries.rs:74-101` 注释明确写了“一帧会问它好几次”）是同一类问题，只是搜索侧没解决。
- 最小修复方向：按 `(query, last_source_stamp)` 备忘（复用 `CallTreeMemo` 的形状），或每次 reload 建立符号索引（`NodeId → Vec<SearchRow>`），一帧只查一次；`ui` 三个面板共享同一份 `rows` 而不是各自调 `search_rows`。
- 复核手段：加一个探针测试——在 `search_rows` 里计数（测试期 `static AtomicUsize`）或直接在测试里渲染一帧并断言该计数器 ≤ 1。

### S-06 · MAJOR · 逻辑/重复真值 · `app/search_queries.rs:74-133` vs `core/src/registry_core/source/source.rs:120`

- 现象：`source_symbol_rows` 第二段循环自己实现了一套“什么是符号”的识别：13 个 `strip_prefix`（`pub fn `/`fn `/`pub async fn `/…/`const `/`let `，`search_queries.rs:82-95`），然后按 `( { : = whitespace` 截名。同一函数上面已经用内核 `function_symbols` 拿到了函数清单（`search_queries.rs:55`），并只把「已是函数声明行」的行跳过（`search_queries.rs:75-79`）。
- 判据：仓库自己立的规则就在隔壁——`state/state.rs:32-37` 写了“内核拥有唯一的 `::` 边界匹配规则，Studio 曾带一份单向副本……这会让它画出的图与内核归并的证据不一致”。这里是同一错误的另一种形态：两份“符号清单”会漂移，而且两个方向都实测存在——
  - **多**：`let` 绑定被当成符号（`search_queries.rs:95`），并由 `ui/search/results.rs:38-42` 用与函数相同的 `ƒ` 标记渲染、按 `Enter` 可直接推进调用图（`app/overlay/search.rs:149-176`）——语义上它不是可跳转的符号，`call_relations` 也找不到同名函数。
  - **漏**：13 个前缀不含 `pub(crate)`/`pub(super)`/`unsafe fn`（`search_queries.rs:82-95`），而内核认这些（`studio/studio/app/tests/source.rs:26-53` 用例 `function_index_accepts_qualified_visibility_and_impl_methods` 钉了 `pub(crate) async fn load` 与 impl 内的 `unsafe fn render`）；这类行只有在 needle 命中内核已识别的 name/signature 时才由上面的循环补上，否则整行不进结果，而符号循环里真正按“整行文本”匹配的分支（`search_queries.rs:113` 的 `trimmed.contains(&needle)`）本来会命中它——同一行在两段代码里得到不同答案。
  - 去重比文档弱（注释轴另记 C-03）：`search_rows` 交给 `dedup_by`（`search_queries.rs:20-23`），而它只比较**相邻**元素；文件行（`depth: 0`、`function: ""`、`line: Some(1)`）与符号行之间有函数行插入，所以“已去重的扁平列表”（同处文档措辞）只对相邻重复成立。**今天没有可复现输入**（每行最多进一次符号循环、函数行已按行号跳过），因此这是一条“文档承诺 > 代码保证”的隐性依赖，不是现存 bug——但它意味着以后任何一处调整行序都可能悄悄放出重复行。
- 最小修复方向：让内核 `source` 提供一份“声明/符号”清单（或复用它已有的 `function_symbols` 加上独立的 `item_symbols`），Studio 只做渲染与去重。
- 复核手段：测试——对同一个源文件，断言 `search_rows("...")` 中来自该文件的符号行集合等于内核扫描产出的集合；把 `pub(crate) fn`、impl 内 `unsafe fn`、`let` 三类各放一行即可区分两份实现。

### S-07 · MAJOR · 错误吞掉 · `app/graft.rs:127-158`

- 现象：`submit_graft` 先 `self.open_editor_file(path.clone(), 1)`（`graft.rs:128`），紧接着**无条件**`self.event = match &state.declaration { … }`（`graft.rs:129-158`）。而 `open_editor_file`（`support.rs:459-465`）在路径不是文件时会写 `self.event = "Editor failed: source file not found at …"`。
- 判据：错误消息被成功横幅覆盖，且 `editor_request` 保持 `None`，用户既看不到失败也不知道没打开编辑器；`event` 是本界面唯一的反馈通道（`status.rs:6-33`）。
- 最小修复方向：把编辑器结果纳入 match（或在 `open_editor_file` 返回 `Result`），失败时不做覆盖。
- 复核手段：测试——构造一个 plan 路径不可用（例如把 `.xirang/external-grafts/<sel>/` 变成同名文件）后调 `submit_graft`，断言 `app.event` 里出现编辑器失败而不是 “External graft plan created”。

### S-08 · MAJOR · 逻辑/写盘闸门 · `app/mutations.rs:320`

- 现象：入口行是否已存在用**全文字串**判断：`if !entry_text.contains(&format!("use {crate_name} as _;"))`（`mutations.rs:320`）。
- 判据：注释、块注释或字符串里的同一行会让判断成立，于是跳过 append，随后只写锁——`mutations.rs:347` 报 “Plugin selected”，而宿主入口并没有 `use <crate> as _;`，该 crate 不进宿主构建。写盘成功 + 语义没达成的静默不一致，正是这个写入方存在的理由（`mutations.rs:326-331` 为“半写”设计了回滚，而这一条根本不会触发回滚）。
- 最小修复方向：按行判断（`entry_text.lines().any(|line| line.trim() == format!("use {crate_name} as _;"))`），或干脆把入口写入也交给内核/执行面里唯一的写入器。
- 复核手段：测试——入口文件里放 `// use demo_plugin as _;`，提交后断言：要么报“入口已声明但未生效”，要么真的补写一行；两者都不可以是“Plugin selected”且入口不含未注释的 use。

### S-09 · MINOR · 一致性 · `app/navigation.rs:79-85` vs `bin/xirang-dev.rs:273-276`

- 现象：`source_stamp()` 的“相关文件”集合只认 `Cargo.toml | Cargo.lock | official.lock`（`navigation.rs:83`），而 `xirang-dev` 的 watcher 认 `build.rs | official.lock | user.lock`（`xirang-dev.rs:275`，并且有用例 `xirang-dev.rs:382-403` 专门钉 `user.lock`）。
- 判据：两个执行面对“插件目录里什么算源码变更”给出不同答案。用户来源的插件若 crate 锚点已存在（`mutations.rs:320` 命中），`submit_plugin` 只追加 `user.lock`——此时 `poll_hot_reload`（`lifecycle.rs:108-117`）看不到戳变化，界面不刷新，而 `xirang-dev` 会重启 Studio。两边应同源。
- 最小修复方向：把“哪些文件是 Studio 的相关源码”提到 `lexicon`（或一个共享谓词），`source_stamp` 与 `relevant_event` 都调它。
- 复核手段：在 `navigation.rs` 的戳函数上写一条单测（只改 `user.lock` 必须改变戳）；或对齐两处清单后把 `xirang-dev.rs:388-403` 的期望扩到两端。

### S-10 · MINOR · 缓存失效 · `ui/graph/nodes.rs:183-188`、`app/app.rs:123-124`

- 现象：`rataflow` 控件的缓存键是 `flow_key` = `"{focus}:{node_count}:{edge_count}"`（`nodes.rs:183-188`），只在键变化时重建（`nodes.rs:130-165`）；`app.graph_flow` 是 `Option<(String, usize, rataflow::Flow)>`（`app.rs:123-124`），全仓库没有任何地方在 reload / `select_project` / registry 变更时清它（`grep graph_flow` 只有 `lifecycle.rs:62` 初始化、`pointer.rs:151` 与 `nodes.rs` 的读写）。
- 判据：键不含源码戳也不含节点文本，因此“焦点符号相同、节点数与边数相同、但树的内容/车道不同”的两次构建会复用旧绘制：画面（节点标题、车道位置）来自旧树，而状态行与 DATA 面板（`nodes.rs:201-342`、`graph.rs:69`）来自新模型——同一屏两套事实。`tree_cache` 靠戳自失效（`call_tree_queries.rs:82-88`），这个缓存不会。
- 最小修复方向：键里带上 `last_source_stamp`（并保留 cursor 的“只重盖标记”快路径），或在 reload/切项目处显式 `graph_flow = None`。
- 复核手段：测试——构造两个节点数与边数相同、但节点符号不同的 `CallTreeView`，依次 draw，断言第二次绘制出的文本来自第二棵树。

### S-11 · MINOR · 重复 · `ui/overlay.rs:8-23` vs `ui/overlay.rs:31-44`

- 现象：同一段 15 行热区复位写了两遍，两遍的字段顺序还不一样（第二遍把 `action_validate_area/action_edit_area` 提前），必须逐行比对才能确认覆盖面相同。
- 判据：手抄两遍的清单没有编译期保护：新增一个热区字段时只在其中一遍写 `Rect::default()` 就留下跨帧残留的坐标（这类残留会让点击落到已经不存在的位置）。**我逐字段核对过：目前两遍都覆盖全部 15 个字段，没有遗漏——是维护性风险，不是现存 bug。**
- 最小修复方向：`fn reset_hot_zones(app: &mut App, overlay_area: Rect)`，两处调用；或让 `HotZones::default()` 一处分派。
- 复核手段：把两个分支都改成调用同一函数（编译器保证一致）；补一条断言：关闭 overlay 后再 draw，所有热区为 `Rect::default()`。

### S-12 · MINOR · 魔法数字 · `app/overlay/plugin.rs:29-30`、`app/pointer.rs:294/335`、`app/overlay/new_project.rs:28-29`

- 现象：字段上界与“这一行是不是开关行”用裸数字：`(plugin.field + 1).min(6)`、`plugin.field == 0 || plugin.field == 6`（`overlay/plugin.rs:29-30`）、`visible_row == 0 || visible_row == 6`（`pointer.rs:335`）、`(project.field + 1).min(2)`、`project.field == 2`（`overlay/new_project.rs:28-29`）、`visible_row == 2`（`pointer.rs:294`）；而同一批代码里又混用了 `plugin_field::SOURCE`/`plugin_field::MODE`（`pointer.rs:337/343`）。
- 判据：`plugin_field::COUNT = 7`、`new_project_field::COUNT = 3`、`new_project_field::KIND = 2` 已经存在（`state/plugin_field.rs:33`、`state/new_project_field.rs:27/23`），且这两个模块的文档明确说具名下标就是为了“让重新排序变成编译错误”。裸数字把这条保护只留了一半：加一行后 `min(6)` 依然编译，但第 7 行变成不可达、点击路由错行。
- 最小修复方向：`plugin_field::COUNT - 1`、`plugin_field::MODE`、`new_project_field::COUNT - 1`、`new_project_field::KIND`。
- 复核手段：`state/plugin_field.rs:54-68` 与 `state/new_project_field.rs:41-47` 的稠密性用例已存在；再加一条断言表单上界 == `COUNT`。

### S-13 · MINOR · 职责边界 · `app/state/misc.rs:161-181`（整个文件 1-181）

- 现象：被要求核对“`state/` 八个文件是纯数据还是偷偷做 I/O”的答案是：`forms.rs`、`graft.rs`、`pages.rs`、`search.rs`、`plugin_field.rs`、`new_project_field.rs`、`state.rs` 是纯数据/纯常量，**`misc.rs` 不是**——`source_path_for`（`misc.rs:161-181`）调 `package_root()`（`support.rs:111-141`，读环境变量与 `current_dir()`）并做 `path.is_dir()`/`is_file()` 文件系统探测。文件自己的标题是 “Ungrouped Studio state / 未归类的 Studio 状态”（`misc.rs:1-2`），里面装着 `ReloadError`、`Overlay`（模态枚举）、`CallRef`、`CallTreeView`、`CallTreeMemo`、`push_call_ref`、`source_path_for`——状态、查询结果、路径解析、I/O 各一。
- 判据：`state` 是身份目录名，读它的人有权假设“只有数据”。`source_path_for` 的探测行为还会在渲染期被调用（`ui/search/detail.rs:33`、`ui/graph/nodes.rs:265-273`），即“纯状态目录”里的函数参与每帧 I/O。
- 最小修复方向：`source_path_for` → `support`（或 `writers` 旁的路径模块）；`CallRef`/`CallTreeView`/`CallTreeMemo`/`push_call_ref` → `call_tree_queries`；`Overlay`/`ReloadError` 留 state（或各自的屏幕模块）；删掉 `misc.rs` 这个名字。
- 复核手段：把 `misc.rs` 移空后编译；`grep -n 'std::fs\|std::env' studio/studio/app/state/` 应为空（可加进 `conventions`）。

### S-14 · MINOR · 命名/职责 · `app/writers.rs`（全 42 行）与 `app/mutations.rs:1-395`

- 现象：`writers.rs` 名字承诺“写入方”，实际只有两个**读取上下文**的守卫：`selected_package_root()`（`writers.rs:30-33`）与 `with_selected_project`（`writers.rs:37-42`）；真正的写入全在 `mutations.rs`：`create_project`（`mutations.rs:51-56`）、`add_module_from_face`（`mutations.rs:137-149`）、`edit_module_face`（`mutations.rs:185-210`），加上手写的读取/追加/删除调用（`mutations.rs:259`、`mutations.rs:319`、`mutations.rs:333-338`）与 `append_line`（`mutations.rs:389-395`）。文件顶注（`writers.rs:1-22`）还自称“每一个……创建、重写、移动或删除东西的操作”都在这里。
- 判据：维护者要改一次写盘必须去 `mutations.rs`，而 `writers.rs` 的文档会把他引到错误的地方；`mutations.rs` 同时承担 new-project 脚手架、add/edit 表单提交、plugin 锁写入、编辑器交接四种职责（`mutations.rs:10/74/78/152/213`）。这一条与 S-01 同根：写盘责任没有一个明确的落点，于是 plugin 那条路绕开了内核的写入器，自己拼格式。
- 注释轴：模块文档自述与实现不符，见 C-02。
- 最小修复方向：`writers.rs` 要么接纳真正的写盘辅助（`append_line` 等）并改名 `writes.rs`/`guard.rs` 与实际一致，要么把四个 `submit_*` 各自下沉到一个模块（`forms/`、`plugin.rs`、`project.rs`），`mutations.rs` 只留编辑器交接。
- 复核手段：改名/拆模块后 `grep -rn 'std::fs::' studio/studio/app/` 的命中应集中在被命名为写入方的模块里。

### S-15 · MINOR · 碎片化/阅读顺序 · `app/support.rs:1-509`

- 现象：509 行的 `support.rs` 装四类互不相干的关注点：项目解析与清单（`251-261`、`157-247`）、`cargo` 子进程探测（`277-374`）、纯几何/选择移动/编辑器交接（`378-466`）、测试夹具助手（`484-509`）。文件里出现**第二个** `impl App` 块与一句文件中间的 `use super::*;`（`support.rs:376`），第一个 `impl App` 在 `378`。
- 判据：模块顶注只声明了两件事（“Shared interaction geometry and editor helpers”，`support.rs:1-2`），实际内容的三分之二是 I/O 与进程派生；“支持文件”这个语义下任何东西都能塞。`AGENTS.md` 的 600 行棘轮也说明 509 行已接近上限，下一次添加会撞线。
- 注释轴：模块文档声明的范围小于实际内容，见 C-04。
- 最小修复方向：拆 `project.rs`（resolve_project/`resolve_project_from`/`usable`/`manifest_for`）、`cargo_probe.rs`（`cargo_rustc_mir`/`bin_target_names`）、`geometry.rs`（`near_divider`/`resize_*`/`move_selection`/`open_editor_at`），夹具助手移入 `tests.rs` 前导。
- 复核手段：拆完 `support.rs` 只剩几何；`cargo test -p xirang-studio --offline` 全绿。

### S-16 · MINOR · 逻辑/重复工作 · `app/lifecycle.rs:80-88`

- 现象：失败分支再调一次同样的 `load_registry()`：`Err(error) => Self::new(load_registry().unwrap_or_else(|_| Registry::root()), format!("Registration startup failed:\n{error}"))`。
- 判据：第二次调用重复整趟 I/O；若它恰好成功（外部进程正好写完文件），会话会带着一份**成功加载的树**和一条“失败”的事件行——状态与消息互相矛盾。另外 `unwrap_or_else` 把第二次失败也吞掉。
- 最小修复方向：`let loaded = load_registry();` 只调一次，`Err(error) => Self::new(Registry::root(), format!("Registration startup failed:\n{error}"))`。
- 复核手段：测试难以稳定复现第二次成功；改为代码审查 + 一条断言“`load()` 失败时事件行含 phase=reload/启动失败且 `registry.depth_first()` 为空”。

### S-17 · MINOR · 命名/死分支 · `app/navigation.rs:141-146/155-173`

- 现象：`advance_graph_focus` 与 `retreat_graph_focus` 函数体逐字节相同（都是 `graph_focus = {0=>1, _=>0}` + `outline_focus` 同步），注释却分别写“Two panes: the tree, then the values…”与“BackTab walks the same two panes the other way”（`navigation.rs:155-173`）；`step_graph_cursor` 的键映射以 `_ => TreeStep::Right` 收尾（`navigation.rs:141-146`）。
- 判据：两块面板下互切确实等价，但两份实现 + 一句不成立的注释会让下一个人以为 BackTab 有独立语义；`_ => Right` 把任何未来新增的键静默解释为右移（当前调用点只传四个方向键，`overlay/search.rs:57-59`，所以今天不可达）。
- 最小修复方向：合并为 `toggle_graph_focus`（保留两个名字做 thin alias 或直接改名并更新注释）；键映射改成穷尽 `match`（`KeyCode::Right` 显式）。注释那一半见 C-01。
- 复核手段：`grep -c retreat_graph_focus`；或把 `_` 分支改成 `KeyCode::Right` 后看是否出现不可达警告。

### S-18 · MINOR · 耦合/性能 · `ui/status.rs:16-17`

- 现象：事件严重度由文本子串推断：`app.event.starts_with("Warning:") || app.event.to_ascii_lowercase().contains("fail")`，并对整条消息做一次 `to_ascii_lowercase()` 分配，每帧一次。
- 判据：颜色是行为，不该由人类可读措辞决定：任何含 “fail” 的成功消息（例如路径里带 `fail`、或“0 failures”）都会变红，而 `Graft failed:` 之类的真实失败恰好也能被覆盖——这是巧合而不是契约。仓库已经有一个更好的先例：`ReloadError { phase, message }`（`state/misc.rs:16-23`）就是结构化的阶段/消息对。
- 最小修复方向：给事件加一个 `EventKind/Warning` 枚举（或在写 `self.event` 时同时设 `self.event_warning: bool`），`draw_event` 读它。
- 复核手段：测试——`app.event = "0 failures".into()` 后断言颜色不是 `LightRed`；`"Warning: …".into()` 时是红。

### S-19 · MINOR · 命名/交互 · `app/keyboard.rs:160-164`

- 现象：Details 焦点下的 Enter 通过**递归伪造按键**复用 `e` 分支：`self.handle_key(KeyEvent::from(KeyCode::Char('e')))`；而同一 match 的下一支 `KeyCode::Enter | KeyCode::Char(' ') => self.toggle_selected()` 又让 Tree 焦点下的 Enter 是折叠（页脚 `status.rs:48` 写 “Enter fold”）。
- 判据：一个键在两个焦点下语义不同却只宣传一个；递归调用绕过 `handle_key` 顶部的 overlay 守卫（今天成立，但下一个人若在该分支前加状态就失效）。关键字处理函数里出现“合成按键”通常意味着真正该抽的是一个具名动作。
- 最小修复方向：抽 `fn open_edit_form(&mut self)`，`Char('e')` 分支与 `Enter`(Details) 分支都调它；页脚写 “Enter fold / edit”。
- 复核手段：测试——两种焦点下按 Enter，断言 overlay 分别是 None/Edit；页脚断言补 “edit”。

### S-20 · MINOR · 碎片化/重复状态机 · `app/overlay/add.rs:12-36` vs `app/overlay/edit.rs:17-41`

- 现象：add 与 edit 的两个 overlay 各写一份同样的编辑状态机：editing 分支的 `Enter/Backspace/Char` 三支、非编辑分支的 `Up/Down|Tab`、`Enter|Space if field == NEEDS_REGISTRY` 开关、`Enter if is_editable` 进入编辑、`s` 提交——只有提交目标不同（`submit_add` vs `submit_edit`）。
- 判据：两文件是同一件事的两份拷贝（各 20 行），差异只在一处；任何按键语义修正都必须记得改两遍，漏一处就是“Edit 能用、Add 不能”这类静默不一致。
- 最小修复方向：`fn handle_face_form_key(&mut self, key: KeyEvent, form: &mut AddState) -> Option<Action>`，两个 overlay 各自只做提交分派。
- 复核手段：`tests/forms.rs:25-64` 的导航用例同时跑 Add/Edit 两条路径即可覆盖；或比对两文件的 match 骨架。

### S-21 · MINOR · 重复 · `ui/forms/face.rs:105-128`、`ui/forms/project.rs:47-70`、`ui/forms/plugin.rs:46-69`、`ui/forms/graft.rs:214-239`

- 现象：四个表单页各自复制 4 按钮布局（`Constraint::Percentage(25)` ×4）、各自的 `button(label, color)` 闭包、各自的 `marker`（`●/›/ `）与「选中行上色」逻辑。
- 判据：按钮区是界面的公共契约（点击热区在 `overlay.rs:56-95` 依赖返回的 `(Rect, Rect, Rect, Rect)` 顺序），四份拷贝意味着一次调整要改四遍，而 `overlay.rs` 的 `let (list, cancel, confirm, exit) = draw_*` 解构顺序同样是四份隐式约定。
- 最小修复方向：`fn draw_form_frame(frame, area, rows, title, footer) -> FormAreas { list, cancel, confirm, exit }`，四个页面只提供行与页脚。
- 复核手段：`ui.rs:171-206` 与 `graft_tests.rs:195-198` 的渲染断言保持通过；点按钮用例（`tests/forms.rs:9-24`）保持通过。

### S-22 · MINOR · 测试夹具重复 · `tests/call_tree.rs:25-35`、`tests/evidence.rs:60-67`、`tests/graph.rs:15-24/66-72/132-138/181-187`

- 现象：“取 node-editor 夹具 → `select_project(...)` → `App::load()`”三行，被写成 `fixture_app()`（call_tree）、`load_fixture()`（evidence）与 graph.rs 里**四处内联**，命名还不一致；命名空间字符串 `"xirang.fixture.node-editor"` 出现在至少 5 处。
- 判据：夹具路径与命名空间是同一个契约（`support.rs:484-509` 的 `node_editor_fixture`），散落后改一处命名空间会只改到一部分；这也是 `tests.rs` 前导已经存在的理由。
- 最小修复方向：把 `fixture_app() -> Option<App>` 放进 `tests.rs` 前导（`#[cfg(feature = "prototype-fixtures")]`），各文件删本地副本。
- 复核手段：`grep -c 'select_project' src/studio/app/tests/` 应只剩 1 处（trace_ingest/edit 等自建工程除外）。

### S-23 · MINOR · 测试结构/成本 · `tests/project.rs:1-562`

- 现象：562 行一个文件装四件事：项目根/清单/命名空间（`9-212`）、新项目向导（`215-411`）、`resolve_project_from` 纯函数（`419-486`）、MIR target 解析（`488-562`）。其中 `new_project_and_explicit_root_face_compile`（`215-354`）在单测进程里跑 `cargo check --offline`，`mir_target_resolves`（`492-529`）每个用例跑一次 `cargo rustc`，共 4 次（`536-562`）。
- 判据：单元测试二进制因此依赖外部 `cargo`、真实编译与磁盘 cache；文件本身越过 500 行，`tests/call_tree.rs`（702 行）同理混了 memo/箭头/Enter/鼠标/绘制/无特性行为。另：`app/graft_tests.rs` 是唯一一个不在 `tests/` 下的测试文件（由 `app/graft.rs:352-354` 挂载），位置约定不统一。
- 最小修复方向：`project.rs` 拆 `project_root.rs` / `new_project.rs` / `mir_target.rs`；把要真编译的用例挪到 `tests/`（集成测试）或 `#[ignore]`/feature 门控；`graft_tests.rs` 迁入 `tests/`。
- 复核手段：拆分后 `cargo test -p xirang-studio --offline` 的分组输出；`wc -l` 检查各文件尺寸。

### S-24 · MINOR · 错误吞掉 · `app/mutations.rs:259`、`319`

- 现象：`std::fs::read_to_string(&lock).unwrap_or_default()`（`259`）与 `read_to_string(&entry).unwrap_or_default()`（`319`）把**任何**读错误都当成“文件为空”。
- 判据：当锁存在但读不动（权限/编码/是目录）时，“Plugin already selected”检查与 `PluginCatalog::parse` 都在空串上做，随后仍会 append（`append(true)` 打开成功即可），等于在无法完整读取的工件上追加；而入口文件读不动时 `entry_text` 为空，回滚时 `mutations.rs:334-338` 会用空内容或 `remove_file` 覆盖，可能抹掉原文件。反过来，把 `NotFound` 与其它错误分开是有意义的：前者是正常初态。
- 最小修复方向：`match read_to_string { Ok(t) => t, Err(e) if e.kind() == NotFound => String::new(), Err(e) => { self.event = format!("Plugin failed: cannot read {lock_name}: {e}"); return; } }`；入口同理。
- 复核手段：测试——把 `user.lock` 做成一个目录（`tests/forms.rs:197` 已有这个手法），断言事件行是“cannot read”而不是“Plugin already selected”/成功。

### S-25 · MINOR · 逻辑/脆弱判据 · `app/support.rs:296/311-322/362-373`

- 现象：MIR target 降级链（`--lib` → 逐个 `--bin` → `--bins`）用 cargo 的英文 stderr 文案判定“这个包没有库目标”：`String::from_utf8_lossy(&output.stderr).contains("no library targets")`（`296`）；`bin_target_names`（`332-374`）从 `cargo metadata --no-deps` 的 `packages[*].targets[*]` 收集所有 `kind` 含 `bin` 的 target 名（`362-373`），**没有过滤出 manifest 指名的那个 package**。
- 判据：判据（1）文案是 cargo 的实现细节，随版本/措辞变化即静默走错分支（表现为“没有库目标时说 MIR 不可用”）；（2）工作区/多成员下 `--no-deps` 仍会列出各成员，于是 `--bin <别的成员的 bin>` 会被逐个试错，`last` 可能是来自另一个 package 的失败输出（`314-322` 返回 `last`）——错误消息会指错宿主。
- 最小修复方向：优先用 `cargo metadata` 判断有无 lib（`kind` 含 `lib`），把 stderr 匹配降级为兜底；`bin_target_names` 过滤 `package.manifest_path == manifest`。
- 复核手段：`tests/project.rs:536-562` 的四个用例保持通过，并新增一个“工作区根 + 两个成员各一个 bin”的用例，断言解析出的 target 名属于被指名的 manifest。

### S-26 · MINOR · 命名/位置 · `app/navigation.rs:1-96`

- 现象：模块名是 `navigation`，文件里 96 行（`1-96`）是文件系统戳扫描（`source_stamp`/`stamp_file`/`stamp_directory`，含递归目录遍历与 mtime/size 收集），只有 `103-173` 才是导航（`shift_graph_split`/`step_graph_cursor`/`advance_*`/`retreat_*`）。文件标题自己承认了这件事：“App source stamps and graph focus navigation.”（`navigation.rs:1`）。
- 判据：`source_stamp` 是性能与失效语义的所在地（整棵树每 500ms 一次 `read_dir` 递归，`lifecycle.rs:108-117`），把它放在“导航”下会让找缓存失效的人找不到它；S-09 的清单不一致也正发生在“没人知道它该和 watcher 同源”的情况下。
- 最小修复方向：`source_stamp`/两个 `stamp_*` → 独立 `source_stamp.rs`（或 `support`），并作为 S-09 的共享谓词落点。
- 复核手段：`grep -rn source_stamp studio/src` 的调用点只剩 `lifecycle`；`navigation.rs` 变成纯导航。

### S-27 · MINOR · 职责/一致性 · `app/mutations.rs:137-149` vs `185-210`；`lifecycle.rs:108-133`

- 现象：Add 与 Edit 对“写完之后界面怎么办”给出两种政策：`submit_add` 只 `register_snapshot_batch([info])`（`mutations.rs:141`）并让事件行说 “press r to reload”（`145`）；`submit_edit` 直接 `self.reload()`（`195`）并顺带选中改动后的 face。两条路径都不更新 `last_source_stamp`（`lifecycle.rs:117` 只在 `poll_hot_reload` 里更新），因此写盘后到下一次 poll（≤500ms）之间，`call_tree_view` 可能命中按旧戳备忘的树（`call_tree_queries.rs:82-88`）。
- 判据：同一类“改了项目文件”的动作有两种刷新语义与两种戳状态，用户看到的是“Add 后按 r、Edit 后自动”；缓存键与真实变更不同步的窗口是备忘机制的已知前提（`call_tree_queries.rs:74-80` 声称“源码改动会让它失效”）。
- 最小修复方向：写入后统一走一个 `after_project_write()`：重算 `last_source_stamp`、按政策 reload/增量注册、清 `graph_flow`（同时解决 S-10）。
- 复核手段：测试——`submit_add` 后立刻 `call_tree_view` 同一个 focus，断言树的节点数已反映新 face。

### S-28 · MINOR · 功能缺口 · `app/interaction.rs:18`

- 现象：`Event::Paste(_)`（以及 `Resize/FocusGained/FocusLost`）被显式忽略；所有表单输入只走 `KeyCode::Char`。
- 判据：终端里粘贴一个路径/包名/checksum 到表单是常见动作（`NewProjectState` 的 directory 字段尤其），丢弃 paste 事件意味着用户拿到的是“按键丢失”而没有解释。存储与刷新本身没问题：`run_loop`（`studio.rs:99-108`）在下一轮会重绘。
- 最小修复方向：把 `Event::Paste(text)` 分派给当前 overlay 的编辑字段（现有 `Char` 分支即插即用），或在表单页脚明确“不支持粘贴”。
- 复核手段：在 `tests/forms.rs` 里对某个 editing 表单 `app.handle(Event::Paste("abc".into()))` 并断言字段值。

### S-29 · MINOR · 读写根不一致 · `app/graft.rs:166-175/235-265` vs `111-118/180/200`

- 现象：同一个 graft 界面里，**读**走 `with_authoring_context`（回落 `package_root()`，`graft.rs:167/236`），**写**走 `with_selected_project`（拒绝回落，`graft.rs:111/180/200`）。前者解析出的项目可能来自环境变量或当前目录（`support.rs:111-141`）。
- 判据：`writers.rs:5-16` 把这条区分讲得很清楚（读错树看得见、写错树不可接受），但界面把两者放进同一屏：列表面板显示的是 `package_root()` 下的计划，`d`/`f` 作用的是同一个 selector 却写在 selected root 下。已启动会话里两者相等，所以今天不炸；库调用方/测试/被清空选择的会话下会出现“列表里看得到、删的是另一个项目里的同名计划”。
- 最小修复方向：屏幕状态的来源与写作用域同源——graft 读也走 `selected_package_root()`（或统一成一个 `with_authoring_project`），不可解析时在屏幕上说明。
- 复核手段：测试——`clear_project_context()` 后设 `XIRANG_PACKAGE_ROOT` 指到工程 A，注册一个 B 上的 graft 界面，断言 `d` 不会移动 A 的记录（或读列表为空/报错）。

### S-30 · MINOR（设计/契约，非挂载缺陷）· `studio/tests/fixtures/node-editor/**`、`app/support.rs:497-509`

- 现象：我核实过（`grep -n '^\(pub \)\?\(mod\|#\[path\)'` 逐文件）该夹具的 7 个 `.rs`（含 `build.rs`、`src/lib.rs`）里**没有任何 `mod`/`#[path]` 声明**，5 个源码面文件没有任何东西指名；`Cargo.toml:2` 的 `members` 也不含它（它不是 workspace 成员）。因此它从不被任何门禁编译——它只在 `App::load` → `load_registry`（`source_index.rs:18-42`）的词法扫描里作为**语料**被读取，供调用图形状/span 类测试使用（`support.rs:497-509` 的 `node_editor_fixture()`）。
- 判据（明确不是“漏挂”）：这是既有设计（`AGENTS.md` 说 `--all-features` 那个 CI 任务是唯一“exercise”它的门禁，而不是“compile”它），所以不按挂载缺陷记。作为**设计**它有一条可检验的缺口：这套文件是“真实宿主工程”的样本，而没有任何检查保证它们仍是**能被 rustc 接受的 Rust**——内核词法器（`function_symbols`/`mask_non_code`）比 rustc 宽松，一次把花括号/字符串写坏的编辑不会被任何门禁拦住，测试照旧全绿，而“样本代表真实宿主”的前提已经破了。
- 最小修复方向（三选一，由维护者定）：①把夹具并入 workspace，在 `--all-features` CI 任务里 `cargo check` 它（代价：多一份宿主构建，与 `prototype-fixtures` 只做词法语料的定位有冲突）；②在 `conventions` 加一条轻量门禁——夹具源码必须能被 rustc 解析（`rustc --edition 2021 --crate-type lib --emit=metadata`），不要求解析依赖；③明确写下“夹具是词法语料、不承诺可编译”，并把测试注释里 “a real project's” 这类措辞对齐到该定位。
- 复核手段：在 `cargo test --workspace --all-features` 全绿的状态下，把夹具里某个 `fn` 体改成花括号不配平，确认没有任何门禁失败（即当前无保护）；再验证所选方案后同一改动会失败。

---

## 2. 注释清晰度与可读性（维护者追加的一等审计轴）

判据按重要性取自 captain 的转达：① 注释描述的行为与实现不符 → MAJOR（过时注释比没有更坏）；② 讲「为什么/不变量/陷阱/边界」还是复述代码；③ 双语两侧是否说同一件事；④ 术语是否一致；⑤ 该由名字/类型承担的信息被塞进注释；⑥ 注释掉的死代码、悬挂 TODO/FIXME；⑦ 位置与密度（信噪比）。
说明：C-01/C-02/C-03/C-04 与 S-17/S-14/S-06/S-15 是**同一处的两个轴**（实现轴 + 注释轴），各自计一条并互相指针；其余为注释轴独有。

### C-01 · MAJOR · 注释与实现不符 · `studio/studio/app/navigation.rs:165-171`

- 现象：`retreat_graph_focus` 的注释写 “BackTab walks the same two panes the other way. / BackTab 以相反方向走同样这两块面板。”（`navigation.rs:166-167`），而函数体与 `advance_graph_focus`（`navigation.rs:155-163`）逐字节相同——方向与函数名无关。另见 `navigation.rs:155-171` 两个函数上方各自的 “Two panes…” 注释，都把两面板当三段式在描述。
- 判据：判据 ①。注释描述的行为在实现里不存在：读者据此会以为 BackTab 有独立语义，改动时只改一处，另一处静默漂移（`overlay/search.rs:41-42` 确实两条键都绑在这对函数上）。
- 最小改法：合并为 `toggle_graph_focus` 并同步注释；若要保留两个入口，就把注释改成“两块面板下前进与后退等价，保留两个名字只为键盘映射可读”。
- 复核手段：把 `_ => 0` 改成显式 `1 => 0` 之类不改变行为的改写后，注释与实现是否仍一致；实现重复那一半见 S-17。

### C-02 · MAJOR · 注释与实现不符（模块自述） · `studio/studio/app/writers.rs:1-22`

- 现象：模块文档承诺 “Write guards: every operation that creates, rewrites, moves, or deletes something in the project the reader opened. / 写入守卫：每一个在读者打开的项目里创建、重写、移动或删除东西的操作。”，实现里只有两个不写盘的守卫（`writers.rs:30-33`、`writers.rs:37-42`）；实际写入在 `mutations.rs` 中（`mutations.rs:51-56`、`mutations.rs:137-149`、`mutations.rs:185-210` 与手写的 `mutations.rs:259`、`mutations.rs:319`、`mutations.rs:333-338`）。
- 判据：判据 ①。按文档去 `writers.rs` 找写盘点会空手而归；写盘责任没有落点正是 S-01（CRITICAL）的成因。
- 最小改法：文档先改成事实（“读取上下文与写作用域守卫；写入见 `mutations.rs`”），或按 S-14 的最小修复方向把写入搬进来，文档与之合一。
- 复核手段：`grep -n 'std::fs::\|OpenOptions\|create_project\|add_module_from_face\|edit_module_face' studio/studio/app/writers.rs` 应与其文档所述一致。

### C-03 · MAJOR · 注释与实现不符 · `studio/studio/app/search_queries.rs:9-17`

- 现象：`search_rows` 的文档写 “Flat, deduplicated rows for one search query. / 一次搜索查询的扁平、已去重结果行。”而实现用 `dedup_by`（`search_queries.rs:20-23`），只消**相邻**重复。
- 判据：判据 ①。今天没有可复现输入（在 S-06 里已说明：每行最多进一次符号循环），但文档承诺的保证强于代码——“不会有跨行/跨深度重复”是后来者会据此建立的假设。
- 最小改法：文档改成 “adjacent duplicates collapsed”，或把实现改成真正的全量去重（`BTreeSet` 键）。
- 复核手段：把符号循环的行序调换一次（不改逻辑），看 `search_rows` 是否出现相邻之外的重复——出现即文档不成立。实现轴见 S-06。

### C-04 · MAJOR · 注释范围与实现不符 · `studio/studio/app/support.rs:1-2`

- 现象：模块文档只声明 “Shared interaction geometry and editor helpers. / 交互几何与编辑器辅助。”，而 509 行里有约 220 行是别的东西：项目解析（`support.rs:157-247`）与 `cargo` 子进程（`support.rs:277-374`）。
- 判据：判据 ①的弱形式——不是行为写错，而是**范围声明不全**，同样让读者误判这个模块的依赖面（谁 import 它就会连带引入 `std::process` 与 `std::env`）。
- 最小改法：按 S-15 拆成 `project`/`cargo_probe`/`geometry`，或把文档标题改成实际的三块内容。
- 复核手段：拆后 `support.rs` 只剩几何；或按文档逐项列出内容并核对。

### C-05 · MINOR · 注释位置 · `studio/studio/app/lifecycle.rs:185-190`

- 现象：两段 doc 摘要叠在 `load_mir_snapshot_report` 上（英文 `lifecycle.rs:185` + 中文 `lifecycle.rs:186` 是一份摘要，`lifecycle.rs:187-189` 是另一份），而真正做那件事的 `load_mir_snapshot`（`lifecycle.rs:197`）**没有任何文档**。
- 判据：判据 ⑦（位置）与 ⑤（该由名字承担的信息被塞进注释）：读者会以为 `load_mir_snapshot_report` 背两份契约，而 `load_mir_snapshot` 无契约可依。
- 最小改法：把第一段摘要移到 `load_mir_snapshot` 上方（`lifecycle.rs:196` 前），`load_mir_snapshot_report` 只留“按键绑定一行”那段。
- 复核手段：`sed -n '185,197p' studio/studio/app/lifecycle.rs` 应看到两份摘要各自贴着对应的函数。

### C-06 · MINOR · 注释与实现不符（页脚合同） · `studio/studio/ui/status.rs:37-48`

- 现象：注释说本页脚 “lists exactly the keys that `App::handle_key` handles on the Inspect/workspace page”，而页脚串（`status.rs:48`）不含 `Tab`（切焦点，`keyboard.rs:132-138`）、`j`/`k`（`Up`/`Down` 的别名，`keyboard.rs:139-153`）与空格（折叠），却含 `Enter fold`——`Enter` 在 Details 焦点下是**编辑**（`keyboard.rs:160-164`，见 S-19）。
- 判据：判据 ①。页脚是读者唯一可见的按键契约，而 “exactly” 让它成为可验证的错误断言（`m MIR` 那段注释则是对的，值得保留写法）。
- 最小改法：注释里的 “exactly” 改成 “the subset a reader needs”；页脚补 `Tab/j/k`，并把 `Enter fold` 写成 `Enter fold/edit`。
- 复核手段：逐条比对 `handle_key`（`keyboard.rs:16-167`）的分支与 `status.rs:48` 的页脚串。

### C-07 · MINOR · 术语一致性 · `studio/studio/app/graft.rs:69`、`studio/studio/app/graft.rs:78`、`studio/studio/app/graft.rs:235`

- 现象：同一动作三个名字——`refresh_graft`（刷新界面状态）、`refresh_open_graft`（仅当界面打开时刷新）、`graft_facts`（真正做事的名词短语）；同一“宿主入口里的静态计划声明”在 `graft.rs:15` 叫 declaration、在 `graft.rs:144` 的警告文案里叫 “Add to static_graft_plan!”、在 `mutations.rs:317` 又被叫 `anchor`（那里指的是 `use <crate> as _;` 这一行）——declaration / anchor / entry line 三个词在注释里指三种不同工件。
- 判据：判据 ④。这三个词恰好落在 S-01/S-08 涉及的写入路径上（谁写什么、写到哪里），术语不统一会放大那两处的误读。
- 最小改法：`graft_facts` → `read_graft_state`；在 `graft.rs:1-21` 的三段 bullet 上补一行术语表，把 declaration（`static_graft_plan!`）/ record（`graft.plan`）/ entry line（`use … as _`）三者钉住。
- 复核手段：`grep -rn 'graft_facts\|refresh_graft\|refresh_open_graft' studio/src` 的调用点全部可读成“刷新状态”。

### C-08 · MINOR · 悬挂预留 · `studio/studio/app/hot_zones.rs:40`、`studio/studio/app/hot_zones.rs:43`

- 现象：`action_validate_area` 与 `action_edit_area` 的注释自认 “no screen sets it yet / 目前没有界面设置它”，没有任何绘制方赋值，只有 `ui/overlay.rs:34-37` 每帧把它们清空——读起来像“有东西在用它”。
- 判据：判据 ⑥（悬挂的“以后再改”式承诺）。它比 TODO 安静，但同样是**存在的理由不在代码里**的字段，而且多一对每帧复位的矩形就多一处可能与真实按钮错位。
- 最小改法：删掉这两个字段，等真实按钮出现再加——`state/search.rs:51-55` 已经写下同一条政策：“零调用者的字段会在真实调用者出现时回来”。
- 复核手段：`grep -rn 'action_validate_area\|action_edit_area' studio/src` 只剩 `hot_zones.rs` 与 `ui/overlay.rs`。

### C-09 · MINOR · 注释密度/信噪比 · `studio/studio/app/graft.rs:129-158`、`studio/studio/ui/graph/node_graph.rs:41-48`、`studio/studio/app/call_tree_queries.rs:74-80`

- 现象：注释密度很高（71 文件 12209 行、注释与代码接近 1:2），且同一条“为什么”常在第三处再出现一次：`graft.rs:129-142` 的注释与 `graft.rs:144` 的警告文案说的是同一后果（“发布态剪掉没有声明命名的槽位，运行期 UnkeptSlot 跳过”），`node_graph.rs:41-48` 与测试 `node_graph.rs:245-249` 说的是同一条重叠规则，`call_tree_queries.rs:74-80` 与 `call_tree_queries.rs:3-11` 说的是同一次备忘动机。
- 判据：判据 ⑦ 与 ②。这些注释**是**“为什么/不变量/陷阱”型（我抽查的绝大多数属于此类，见 C-10），所以问题不在“空话”，而在**同一事实重复三遍**：改一处对不上另两处时，读者无法判断哪份是权威（`graft.rs:144` 的文案已在 S-01/S-08 相邻处出现过一次）。
- 最小改法：不必删注释；把“同一后果”只留在事件文案或注释其中一处，另一处用一行指引（如 “see the banner text at `graft.rs:143`”）代替整段复述。
- 复核手段：对同一条事实 `grep` 出所有陈述位置，确认只有一处是完整版。

### C-10 · 注释轴上的「查了、干净」

- **无 TODO/FIXME/XXX/HACK/`todo!()`/`unimplemented!()`**：`grep -rn 'TODO\|FIXME\|XXX\|HACK\|unimplemented!\|todo!' studio/src` 零命中（内核侧有，例如 `core/src/registry_core/plugin/catalog/catalog.rs:113-114` 的两条 TODO，属另一路）。
- **无注释掉的死代码**：被删掉的东西都以“曾有一份副本/示例”的叙述留在注释里（`state/state.rs:32-37`、`ui/graph.rs:5-11`、`ui/search/results.rs:34-37`、`app/overlay/search.rs:110-118`、`ui/forms/graft.rs:193-199`），是记录而不是死代码。
- **双语成色**：抽查的每一对中英都讲同一件事——`support.rs:267-276` 连 “Cargo decides which targets exist” 都译了出来；没有出现“只为过门禁而凑的第二语言”（`conventions` 只保证两种语言都存在，质量靠作者，这一批合格）。
- **模板级样本**：`graft.rs:1-21` 用三段 bullet 把 declaration / application / record 分开，是本仓注释质量的上限，可直接作为其他模块的模板；`support.rs:300-310`（“为什么 `--bins` 不够”）与 `overlay/search.rs:110-118`（“这里为什么**没有**输入分支”）同属这一类。

---

## 3. 重点问题专项小结

- **MIR target 降级链**：链本身正确（`--lib` → 逐个 `--bin` → `--bins`，`support.rs:282-322`），且 `S14` 的两 bin 场景已修；遗留的是 S-25 的两处：cargo stderr 文案判据、`bin_target_names` 未按 package 过滤（`support.rs:296/362-373`）。`tests/project.rs:536-562` 已钉四个形状，缺“工作区根”形状。
- **查询缓存与树变更后的失效**：三套缓存三种状态——`tree_cache` 按源码戳自失效（`call_tree_queries.rs:82-88`，正确）；`graph_flow` 只按 `flow_key` 失效且从不被清（S-10）；搜索侧根本没有缓存（S-05）。再加上 reload 不重校验 trace/MIR（S-03）与写盘不同步戳（S-27），失效语义分散在四个地方，没有一个“项目变了”的单一入口。
- **官方记录闸门 `contains_record` vs `contains`**：内核侧规则是一致的、也是对的——身份七字段严格相等 + 三个来源字段作为**记录侧的期望**（`core/src/registry_core/plugin/catalog/catalog.rs:271-301`，`PH-7` 已修）。**问题不在闸门，而在闸门只装在 official 分支上**：user 来源的写入没有任何身份重复检查，于是 S-01 直接从锁的解析不变量旁边走过去。修法应是“写入前先让解析器判定候选锁”，而不是再加一个分支判断。
- **graft 编辑的写回路径**：`create_external_graft` → `open_editor_file` → 三条事件文案（`graft.rs:111-161`），写回只有一条路径且经 `with_selected_project`，没有“写坏宿主源码”的风险（计划写在 `.xirang/external-grafts/`，`graft.rs:346-350`）。缺陷是消息覆盖（S-07）与读写域不一致（S-29）。

---

## 4. 查了、干净

以下是我按“可能出事”逐点核对后**没有发现问题**的地方，列出以免复核者重复劳动：

1. **TUI 里的索引/panic**：非测试代码没有 `unwrap()`/`expect()` 落在事件循环或渲染路径上。全部数组下标都有上界（`overlay/*.rs` 的 `values[field]` 由 `move_face_field`/`min(COUNT-1)`/`visible_row < values.len()` 保证；`call_tree_queries.rs:238` 的 `nodes[target]` 来自同一 `enumerate`；`pointer.rs:308/322` 用 `fields.get(..)`）。唯一的 `expect` 是 `keyboard_overlay.rs:43` 的 `self.overlay.take().expect("overlay exists")`，两个调用点（`keyboard.rs:12-15` 的 `is_some` 判定、`pointer.rs:8-50` 的 overlay 分支）都先确认了 `overlay.is_some()`——今天不可达，但它是全仓唯一一处“界面会整体带走”的 hard panic，建议改成 `let Some(overlay) = ... else { return }`（不改也可）。
2. **`ui/graph/nodes.rs:209` 的 `&view.tree.nodes[cursor]`**：上一行 `view.item(cursor)`（`nodes.rs:202`）是同一个下标的界内检查，且 `nodes.rs:83` 已把 cursor 钳到 `len-1`；`ui/graph/data.rs:105` 的 `cursor.min(rows.len()-1)` 同理。
3. **`ui/search/detail.rs:94` 的 `source_lines[start..=end]`**：`function_source_range`（`core/src/registry_core/source/source.rs:43-105`）保证返回 0 起始闭区间且 `end < masked_lines.len()`，而掩码保留换行、`split('\n')` 与 `lines()` 行数一致；`search/detail.rs:105` 的行号加法只在 `u32` 极限溢出，现实中不可达。
4. **热区跨帧残留**：`ui/overlay.rs` 两个分支各覆盖全部 15 个 `HotZones` 字段（逐字段核对：无遗漏——残留风险只是 S-11 的重复写法）。无 overlay 时 `graph_area`/`graph_*_area` 被复位，因此 `near_graph_divider`（`support.rs:384-387`）在非图浮层下不可能误命中；`Resize` 被忽略但 `run_loop`（`studio.rs:99-108`）在读取下一个事件前一定重绘，热区不会用到旧尺寸。
5. **`--bins` / MIR 链**：`S14`（两 bin 宿主）确实已修，`support.rs:300-313` 的注释与实现一致；降级链不会把“本来能用”变成错误（metadata 失败仍走 `--bins`）。
6. **官方插件闸门本身**：`contains_record` 的规则与内核用例（`catalog.rs:353+` 的十字段/七字段互认）一致，不要求 candidate 的三个来源字段反向匹配 record——这是 PI `PH-7` 的正确修法，不是漏洞。
7. **graft 写的粒度**：`submit_graft` 只在选择器合法且不冲突时写；已存在且完全相同的计划走“打开而不是覆盖”（`graft.rs:92-110`），删除是 `remove_external_graft`（进回收目录，`graft.rs:199-209`），不改宿主源码。
8. **`ui/graph/node_graph.rs`**：纯构建函数（`build` 无 I/O、无 App），`lane_width[&lane]`/`band_y[band]` 的下标与键集合同源（`node_graph.rs:81-103`），空树时 map 为空、闭包不执行；`offsets` 与测试 `every_box_gets_its_own_room`（`node_graph.rs:250-279`）覆盖了重叠回归。
9. **`ui/forms/graft.rs:193-206`**：`then_some` 提前求值导致 `plans.len()-1` 下溢的 panic 已被修成 `checked_sub`，注释与代码一致（这是我核对的“曾经的 panic”，现已干净）。
10. **配置与契约**：`state/plugin_field.rs`、`state/new_project_field.rs`、`core/src/registry_core/authoring/face_field.rs` 的稠密性/标签用例都在；`state/state.rs:24-31` 的重导出保持历史路径可解析（`AGENTS.md` 的 shim 规则）。
11. **测试基线**：`cargo test -p xirang-studio --offline --all-features` 全绿（lib 91 / dev-bin 7 / launch 4，0 failed），说明本报告里的问题都是**未被测试覆盖**的潜在缺陷，不是破损的树。

---

## 5. 结构观察

- **目录与包边界（studio 内部）**：`studio/studio/{app,ui}` 与 `app/{state,overlay,tests}` 的父文件只做 `#[path]` 挂载 + 重导出（`state/state.rs` 45 行、`ui/forms.rs` 32 行、`keyboard_overlay.rs` 24 行），与 `AGENTS.md` 的模块挂载规则一致，也确实是本轮审计里最健康的一层。真正的边界问题在**文件级职责**而不是目录级：`mutations.rs`（写盘）、`writers.rs`（无写盘）、`support.rs`（四类关注点）、`state/misc.rs`（杂物抽屉）、`navigation.rs`（FS 戳 + 导航）。这五处都不需要动目录，只需要把内容搬到与名字相符的文件，正好符合“本轮只出报告、下一轮逐条批准”的节奏。
- **依赖方向**：`ui → app`单向，`app` 不引用 `ui`（除 `ui/search.rs:96-98` 把 app 的文本函数改名重导出，方向仍是从 app 取）。这是好性质，值得在重构中保住：上面 S-04 的修法（`draw(&App)`）会让这条边更硬。
- **重复实现的三处“真值分裂”**（按危险度）：S-01（锁的合法性：解析器 vs 写入方）、S-02（检视器行数：渲染 vs 键盘）、S-06（符号清单：内核词法器 vs Studio 前缀扫描）。三处都是同一种病：**同一规则在写方与读方各有一份**，三处都建议“让一份成为唯一来源”，而不是两边对齐。
- **命名**：整体很好——`submit_*`（表单提交）、`handle_*`（按键）、`*_queries`（查询）、`draw_*`（渲染）四组前缀确实让维护者知道“这是什么”；`is_editable`/`owns_registry`/`visible_nodes`/`node_label` 等谓词/取值名也清楚。掉信息或名实不符的只有六个：`App::handle`（应为 `handle_event`，`interaction.rs:12`）、`graft_facts`（应为 `refresh_graft_state`，`graft.rs:235`）、`usable`（`support.rs:229`，实为“接受或按名拒绝一个候选根”）、`writers`（S-14）、`misc`（S-13）、`support`（S-15）。没有任何 `process`/`update`/`data`/`info` 这类纯噪音函数名。
- **碎片化**：`app/overlay/`（7 文件）与 `ui/forms/`（6 文件）按 overlay 变体切分是**合理的**——每个文件是一台独立的状态机，父文件是路由器；真正把同一件事切碎的是 add/edit 两文件（S-20）与四个表单页的按钮区（S-21）。`support.rs` 属于“什么都往里塞”的反面案例。测试侧 `call_tree.rs`（702）/`project.rs`（562）与三份夹具助手是主要碎片化点（S-22/S-23）。
- **测试语料的存在方式**：`studio/tests/fixtures/node-editor/` 是一个**不被编译**的嵌套包（S-30），它的价值是“有真实形状的调用图语料”。这与 `tests/project.rs` 内嵌的 `cargo check`/`cargo rustc`（S-23）正好相反：一边完全没有编译校验，一边在单测里做真编译。两种极端建议在下一次重构里各自向中间靠一步——语料至少保证可被 rustc 解析，编译型验证移出单测二进制。
- **信息密度**：Studio 的注释密度极高（多为“这里曾有一个 bug + 审计编号”的形态，例如 `graft.rs:132-142`、`node_graph.rs:41-48`、`overlay/search.rs:110-118`），这在评审时非常有用，代价是同一个事实在注释、测试名、事件文案里各说一遍；本报告里 S-18 的“严重度靠文本推断”就是这种重复的一个副作用。
