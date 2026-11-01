# 审计：t1 的 MCP 工作流是真提升还是表演，兼四设想排序（2026-09-29）

审计对象：`docs/audit-2026-09-29/audit-mcp-probe.md`（t1，256 行，终态）与其逐步调用记录。
本文件是本次唯一写入的仓库文件；不改代码、不提交。

## 0 方法与边界

- **台账三源交叉**：`/tmp/spec*.json`（28 个会话、**138 次 `tools/call`** 的逐字参数，未入库）＋ t1 报告正文的引用 ＋ 我的重跑。
- **诚实限制**：t1 的**返回正文没有落盘**（驱动脚本只打印到 stdout，`/tmp` 里没有批量输出文件）。
  因此台账的"是否改变结论"栏只能判**"报告写下的结论是否依赖它"**，不能判"它有没有顺带给出别的东西"。
  标"未被引用"≠"无产出"，只等于"报告没有用它"。
- **我的重跑装置**：`/tmp/probe/control-button`（t1 的沙箱副本，**已被 t1 改过**：4 个面，`widget` 有一条无人满足的 `requires`），
  桥二进制 `target/debug/xirang-mcp`（9-29 17:11，源码未改），驱动 `/tmp/t2_drive.py`（脚本在 /tmp，未入库）。
  复算命令与结果都用 `tail`/`head` 截断。
- **收尾**：`git status --porcelain` = `?? docs/audit-2026-09-29/`（t1 报告 + 本文件），无其它改动。

## ① 调用台账（28 个会话 / 138 次调用 + 1 次 `tools/list`）

| # | 会话 | 次数 | 工具 | 目的 | 返回要点 | 改变结论？ |
|---|---|---|---|---|---|---|
| 0 | `tools/list` | 1 | — | 对账 | 17 个工具，名字与顺序 | 是（①整节） |
| 1 | spec1 | 3 | status/registry/search | 首轮可用性 | 3 面 + `search Button [ok]` | 是 |
| 2 | spec2 | 21 | 全表 | 真实宿主全工具走查 | 2.1 全部正向输出；X1 语境；M4/M5/M6 前置 | 是（3 条未被引用） |
| 3 | spec3 | 7 | explain/diff/impact/usages/converge | 换节点复核 | X1 对照 | 是 |
| 4 | spec4 | 5 | grafts/diff/mir/unified | 沙箱 graft + MIR | **X2 发现处** | 是 |
| 5 | spec5 | 4 | apply/usages | 写入试错 | **M1**（`invalid module name`） | 是 |
| 6 | spec6 | 3 | apply | 试错 | M1/M2 | 部分是（delete 未被引用） |
| 7 | spec7 | 1 | apply | `requires` 形状 | M2（缺 `=>`） | 是 |
| 8 | spec8 | 2 | apply | 同上 | M2 | 是 |
| 9 | spec9 | 1 | apply | `handle_traits` | **M3（预览成功）** | 是 |
| 10 | spec10 | 2 | apply | 正确写法 | M3（裸标签 / `handle_contracts`） | 是 |
| 11 | spec11 | 4 | apply/usages/converge/verify | 闭环 | **X1**（verify ok vs 其余 rejected） | 是 |
| 12 | spec12 | 3 | apply/usages/converge | 自指需求 | 供 X1 状态 | 部分是 |
| 13 | spec13 | 5 | apply/usages/converge/rename/registry | 写入环 | add→读回→converge→rename 全通 | 是 |
| 14 | spec14 | 5 | trace/unified/converge | artifact | **M5**（两种帧口径） | 是 |
| 15 | spec15 | 17 | 全表空参 | 参数名 | 7 个无参可答 / 10 个点名缺参 | 是（17 次换一句） |
| 16 | spec16 | 10 | search/explain/impact/apply/converge/read | 边界 | M4、G9、overlay 互斥、路径逃逸、M6 | 是 |
| 17 | spec17 | 5 | apply/converge | 提供者 | `answered by root/control/theme` | 是 |
| 18 | spec18 | 3 | usages/converge/registry | theme 侧 | 辅助 | 部分是 |
| 19 | spec19 | 4 | apply/converge/usages | 干跑 | M4 又一刻 | 是 |
| 20 | spec20 | 6 | apply ×3/converge/usages | 真写 | X1 第二次 | 是 |
| 21 | spec21 | 4 | apply/usages/converge/verify | 换需求 | X1 第三次 | 是 |
| 22 | spec22 | 2 | read | 越界 | 静默夹紧（无提示） | 是 |
| 23 | spec23 | 3 | registry/search/usages | 收尾 | 辅助 | 部分是 |
| 24 | spec24 | 4 | mir | JSONL 回环 | X2 复核 | 部分是（2 条未被引用） |
| 25 | specQ | 7 | callgraph/search/status | ③ 符号题 | C1/C2/W1/W2/I1/I2 的 xirang 侧 | 是 |
| 26 | specQ2 | 6 | callgraph/inspect/search | ③ 追问 | 同上 | 是 |
| 27 | specQ3 | 1 | callgraph | C2 闭链第二步 | `entry_rows` → `parse_graft_plan_document` | 是 |

### 装饰性 / 低产调用（报告写下的结论不依赖它）

规格里逐条比对报告引用后，**10 次**调用未被任何结论引用（7%）：spec2 的 `status{root:"src/../src"}`、
`read button.rs line=12 context=6`、`callgraph base_registry`、`diff records=true`（无 orphan 计划时的正路，报告引的是 spec4 沙箱那份）；
spec6 的 `apply delete node=root/control/button`；spec16 的 `search paint limit=2`；spec22 的 `read line=1 context=0`；
spec24 的 `mir button2.mir against`、`mir foreign.jsonl against`、`mir button.mir against + jsonl`（X2 已由 spec4 覆盖）。

另有 **1 个低产会话**：spec15 用 17 次调用换来报告里的一句"7 个无参可答 / 10 个点名缺参"（`audit-mcp-probe.md:164`）。
它不是装饰——它确实产出了那条结论——但投入/产出比是全场最差的一次。

### 台账的两个结论

1. **注册面上 MCP 承重**：`registry/explain/apply/usages/converge/grafts/diff/verify/mir/trace/impact` 的结论**只有 MCP 能给出**
   （17/17 有实现、写入环真跑内核准入、graft/记录/MIR/trace 都读的真实产物）。这一层没有"表演"。
2. **符号面上 MCP ≈ grep**：`search`/`callgraph` 的名字匹配 + 1 跳，与 `grep -rn` 同类；
   我复算的三大条（R1/R2/R3 见下）**每一条都能用 grep 独立得到同样的数字**。
   所以第③节里 t1 判"xirang 更稳"的 W1/W2/I1 三行，本质上是在夸"grep 排好版的输出"——
   这是台账里最该写明的一句：**那三行不能当作"比 grep 更好"的证据**。

## ② 三条手工复算（另加 3 条稳定性复现）

| # | t1 的断言 | 我的算法 | 结论 |
|---|---|---|---|
| R1 | W1：`names_face` 定义在 `declared.rs:138`，callers 7（含测试） | `grep -rn names_face --include=*.rs`（排除 `target/`） | **一致** |
| R2 | W2：`MirGraph::from_mir_text` 在 `text.rs:23`，callers 4 | 同上 grep + `cargo test --lib -- text::tests` | **一致** |
| R3 | I2/M6：`DISPATCH` 是常数，xirang 搜不到；测试存在且通过 | grep + `cargo test --features mcp --lib -- the_dispatch_table_follows_the_catalog` | **一致** |
| R4 | ①对账：注册了没实现 = 0 | grep 全部 `^pub(crate) fn` 逐个归类 | **一致** |
| R5 | M4：`UNANSWERED` 字面"不可达" | 读 `converge.rs:143-160` 的守卫 | **部分一致** |

**R1 原文证据**：定义 `toolchain/src/build_time/src/graft_view/declared.rs:138`；生产调用点恰 6 处
（`impact.rs:95`、`studio/app/graft.rs:324`、`graft_plan_check.rs:143`、`plan_rows.rs:160`、`overlay_rows.rs:137`、
`cli/explain_report.rs:183`），测试 1 个函数（`graft_plan_check.rs:307-316`，同一条 `#[test]` 的两组断言）。
`cargo test -p xirang-toolchain --offline --lib -- a_string_range_names_both_endpoints_as_data` → `1 passed`。
t1 说的"6 生产 + 1 测试 = 7"，与 grep 逐条一致；报告里"6/7 是口径差"的提醒**成立**。

**R2 原文证据**：定义 `kernel/src/registry_core/mir/text.rs:23`；调用点 `text.rs:100`、`text.rs:107`（两条测试）、
`toolchain/src/mcp/mir.rs:291`、`toolchain/src/studio/studio/app/lifecycle.rs:403`。
`cargo test -p xirang-kernel --offline --all-features --lib -- text::tests` → `2 passed`（`parses_native_textual_mir`、
`text_that_is_not_mir_yields_an_empty_graph`）⇒ codegraph 的 `⚠️ no covering tests found` 是**假阴性**，且 W2 那一次
它返回的源码里就有这两条测试，属**同一份返回自相矛盾**。t1 的断言一致。

**R3 原文证据**：`toolchain/src/mcp/tools.rs:316` 定义，唯一使用点同文件 `:374`（`tool_call`），测试
`tools_tests.rs:100`；`cargo test -p xirang-toolchain --offline --features mcp --lib -- the_dispatch_table_follows_the_catalog`
→ `1 passed` ⇒ 第三条假阴性也被我亲手证伪。xirang 侧"搜不到常数"由 `callgraph.rs:23-60`（纯 `load_sources` 名字匹配，
无 MIR、无跳数）与 `search` 只索引 fn/文件/面共同支持，**一致**。

**R4**：`toolchain/src/mcp/*.rs` 里的 `^pub(crate) fn` 逐条归类，只有 12 个是工具入口
（apply/callgraph/converge/diff/grafts/impact/mir/unified/search/registry/usages/verify），其余全是助手
（`load_registry`/`out_dir`/`build_evidence`/`scope_line`/`pruning_line`/`namespace`/`resolve_node`/`derived_faces`/
`load_sources`/`load_one`/`required_path`/`resolve_root`…）或已被 `explain_tool` 路由的 `overlay`；
`inspect`/`read_source`/`status_tool`/`registry_tool` 是 `tools.rs` 里的私有函数，正是 DISPATCH 里的函数指针。
**没有游离的 handler**，t1 的"0 处"一致。

**R5 部分一致（我给 t1 的唯一一处降级）**：`converge.rs:143-160` 的 `UNANSWERED` 支守卫是
`(None, Ok(authored))`，即"内核没拒绝这棵树" **且**"读回了字段"。源码只证明它**有条件**，没有证明它**死**。
t1 用"实测里需求没人满足时 `load_registry` 先失败"推出"字面不可达"，那是**观察**而不是**证明**；
外部 overlay 提供者（`connector_error_with_external`）这一支完全没有被测过。
正确写法应是"实测不可达 / 未证伪"。

## ③ 稳定性发现：MCP 给过错或会误导的答案吗

维护者的判据是"一定稳"。按**误报 / 漏报 / 含糊**三级给实例（★ = 我独立重跑过）。

**误报（说了与事实不符的话）**
- **F1 ★ 同一棵树两个"内核结论"**（t1 的 X1）。同一会话、同一棵树，我实测：
  `verify` → `verdict ok (the kernel accepted the tree)`；同刻 `apply`（预览）与 `usages` → `the package's own faces were rejected` +
  `data-flow attachment failed: input 'control.theme' has no provider`；`converge` → `kernel verdict: this package's own faces are rejected`；
  CLI `./target/debug/xirang check /tmp/probe/control-button` → `ok`（exit 0）。
  机制在源码里：`verify.rs:57` 走的是 `check_for`（静态构建面），`usages/converge/apply` 走 `load_registry` → `register_snapshot_batch`（连接器面）。
  **两边都不是 bug，但"内核接受了这棵树"这句话在错误的那一面上说出了口。** 这是本次最重的一条。
- **F2 ★ `mir jsonl` 自坏回环**（t1 的 X2）。我用同一份 `button.mir` 重跑：`jsonl: true` 返回 `chars=45496 lines=401`，
  最后一行是 `… truncated: 501 lines total, 400 shown. pass what this prints to a JSONL-suffixed file and xirang.mir reads it back.`
  （模板在 `mir.rs:467`，`MAX_ROWS = 400` 在 `mir.rs:42`）；把它当文件读回 → `MirParseError { line: 401, message: "record must start with '{'" }`。
  小图（`small.jsonl`）回环正常 ⇒ **只有 >400 行坏**。工具用一句错误的承诺把读者送进自己的解析错误里。
- **F3 `grafts` 把写坏的计划报成 `declared`**（t1 的 X3）。源码支持：匹配是
  `cut.names_face(&slot.target_path, module_of(slot.target))`（`graft_plan_check.rs:140-144`），`module` 由**能解析的 NodeId** 反推，
  于是 `target` 能解析、`target_path` 自相矛盾的手工计划照样过判；`target_path` 只被打印（`grafts.rs:104`）。
  （t1 的 orphan 计划在其沙箱里已被清理，我只能做**源码级**复核，未能现场重放——这点如实记下。）
- **F4 假阴性覆盖告警（codegraph 侧，不是 xirang）**：`⚠️ no covering tests found` 至少 3 处，其中 W2/I2 **同一份返回自相矛盾**。
  三条测试我全部亲手跑绿（见 R1/R2/R3）⇒ 这是"可信度标记"要处理的标准样本：**在能看见测试的情况下说没有**。

**漏报（该说而没说）**
- **L1 ★ `search` 的"查不到"没有一句明确的否定**：我在仓库根重跑（`/tmp/t2_drive.py /home/nich/Moirai_N3/nichlink /tmp/t2x3.json`）：
  `query=DISPATCH` 回的是 `fn dispatch -> protocol.rs:246` 加 4 条名字里含 dispatch 的测试 fn——**常数 `DISPATCH`（`tools.rs:316`）找不到，回了一个同名函数**；
  `query=control.render` 那一整条回复只输出一行 `tree unavailable (… cargo metadata listed 6 workspace member(s); set XIRANG_NAMESPACE …)`，
  **连 `no matches` 都没有**（而 `grep -rn control\.render --include=*.rs` 有 **69** 处）。
  描述确实写明了它找三类东西（`tools.rs:62`），所以这不是谎报；但读者拿到的是一个同名人或一行无关警告，
  而它该得到的结论是"这三类维度里没有"。**静默缺席比报错危险。**
- **L2 `callgraph` 的按名匹配同时会多报**：t1 在 C1 里列出的 callee 集含 `Err/Ok/Some/clone/into/new` 这类名字噪声。
  工具自报 `evidence: static-heuristic`（`callgraph.rs:61`）算部分免责，但它和"直接调用集"用的是同一个词（callees），读者会照抄。

**含糊（话是真的，读者会误取）**
- **A1 ★ M5 两种帧口径**：`trace` 用帧的 **node 身份**（`declared-at`）归面，`converge trace` 用
  `matches_file(source.file, &face.source)`（`converge_trace.rs:75-77`，即**调用点文件**）。我读源码确认了机制；
  同一份 artifact 于是同时得到"button 面跑了"与 `0 of 4 declared, matched by source file`。两处都各自写明规则，但**没有一处说另一边存在**。
- **A2 ★ 同一事实两种错误约定**：我实测同一次拒绝，`usages` 回 `isError=true`，而 `converge` 把同样十行诊断塞进正文且 `isError=false`。
  agent 按 `isError` 分流时会把 converge 的拒绝当成功读。
- **A3 `read` 越界静默夹紧**：`line=999999` 得到 `src/lib.rs:1-85`，既不报错也不说"已夹紧"（`tools.rs:422-429`，`line.max(1)` + `context.min(120)`）。
- **A4 `impact depth: 0` 静默夹成 1**：`impact.rs:278-283` 的 `clamp(1, ceiling)`，schema 写 `minimum 1`，越界不报。
- **A5 `apply.fields` 的形状全靠试错**（M1/M2/M3）：`tools/list` 把 `fields` 声明成无类型 object；
  `handle_traits` 传 Rust 路径时工具**预览成功**，到父注册规则才炸（`handle Widget must implement interface ControlHandle`），
  直接把错误推迟到构建期。写入路径是 17 个工具里唯一改仓库的，试错成本最高。

**分级小结**：误报 4 条（2 条我独立复现，1 条源码级，1 条属对照工具）；漏报 2 条；含糊 5 条。
**真正会被 agent 当结论抄走的，是 F1、F2、L1、A2 四条。**

## ④ 对照协议公平性

t1 已自曝的装置差异（两个客户端、一次调用、`maxFiles=3`、问题措辞同义非逐字），我补四条：

1. **追问臂证明了"不低估"，但数字不可复现。** 队长对 C1 追问一次（`/tmp/codegraph-baseline.md:184-222`）后信息量**净减少**（符号 24→9、文件 3→1，源码给的是路径之外的 `diagnostic/error.rs`），
   且仍无 hop 序列。我用自己的 codegraph 客户端跑了**逐字同一句**，得到 **20 symbols / 2 files**，源码含 `diagnostic/error.rs` 与 `connector.rs`（终点文件），仍**没有任何 hop 序列**。
   读数：**"C1 答不出链"是工具侧能力边界——这一点成立**；但同一问题同一索引两次返回 9 与 20 个符号、文件集合也不同，
   ⇒ 单次转录**不是可复现的测量**，t1 表里 codegraph 那半列只能当"那一次的样子"。
2. **提问面与执行面不同构**：xirang 侧是"工具名 + JSON 参数"，codegraph 侧是自然语言；一边允许换参数重问（C2 用了两次调用闭链），
   另一边被钉死在一次、不追问。这不是错，但**不能让"codegraph 少给"直接等于"codegraph 更差"**。
3. **混了证据源**：W1/W2/I2 三行里，"xirang 更稳"的部分依据是 codegraph 的假阴性告警被 **cargo 测试**证伪（R1/R2/R3）——
   方法是对的，但结论是"codegraph 的**报告**不可信"，不是"xirang 的**查找**更强"（见 ① 结论 2：那三题 grep 就够）。
4. **单裁判、无预注册量表**：题目由 prober 选，"谁更稳"由 prober 判，没有盲评。t1 的自我限定（"不下排名"）是诚实的；
   本审计也**不重排胜负**，只指出：**C2 与 I2 两行结论最硬**（codegraph 一次给整文件源码 / xirang 根本没有常数地址），
   **C1/W1/W2/I1 四行是装置相关的方向性信号**。

口径提醒（免得被算错）：`… +21 more` 是 xirang 被扣掉的**调用点数**（限 40 时列 20 条）；codegraph 的 `+4 more` 从返回里**读不出单位**。
两边的 `names_face` "6/7"是口径差（6 生产 + 1 测试），不是谁漏报。

## ⑤ 四个设想逐条判（保持原话精神，不美化）

### ① 可信度链路标记——把已稳定的面标成"标本"供学习
- **现状**：信任词是**散装**的：`unparsable faces N`（`resolve.rs:98`）、`unreadable faces N`（`converge.rs:181`）、
  `evidence: static-heuristic`（`callgraph.rs:61`，并在 `protocol.rs:290` 的 instructions 里重复一次）、`verdict ok`、`kernel verdict: … rejected`、`build current/stale`。
  没有逐面的信任戳，没有"稳"的判据，**没有任何地方报告测试覆盖**，也没有"标本"这种东西。
- **缺口**：(a) 一棵树两个判断面（F1）；(b) 同一事实两种错误约定（A2）——"信任"目前连**载体**都不统一；
  (c) 没有消费方：一个没有被任何门禁/回归消费的标签不会让仓库更稳，只会让文档更长。
- **值不值得做**：**精神值得，字面不值得现在就做。**"标本"要先有"稳"的可计算判据（调用链闭合 + 连接器过 + 有测试钉住），
  而这三样里前两样今天互相矛盾（F1）、第三样 MCP 根本不报。先做它的前置条件，再做汇编。
- **最小可验证切片**：见 ⑥ 的 S1——把"同一棵树只有一个可读的信任判断"做成一行，而不是先做标本库。
- **风险**：把"我没测到"标成"稳"（codegraph 的三处假阴性就是现成失败样本，F4）；逐面信任字段一旦落盘进 `.xirang/` 就是需要版本迁移的格式。

### ② diff 链 / diff 文件
- **现状**：`xirang.diff` 已有两种**面级**对照：默认是"源码树 vs 构建清单"（`diff.rs:1-30`、`tree_delta.rs`），
  `records: true` 是"外部 graft 记录 vs 源码"。单位是 **face**，没有 hop 的概念。
- **缺口**：**它的前置不存在**——"diff 链"要先有链，而 `callgraph` 是名字匹配 + 1 跳（`callgraph.rs:23-60`）。
- **值不值得做**：**现在不值得。**没有稳定面集合，"diff 链"只能退化成文本 diff，那是 git 的活，还会更贵。
  它是 ① 和 ③ 的**消费端**，顺序上排最后。
- **最小可验证切片**（若一定要）：给 `diff` 加 `node=` 参数，只报"该面直接调用者里，源码与构建两侧都存在的那部分行号区间变化"——但先要 ③ 有链。
- **风险**：重新实现一遍 `git diff`，token 更贵、语义更弱、还容易被当成"注册机语义"。

### ③ 提高深度链方向上的稳定性（"省不省 token 不知道，但一定稳"）
- **现状**：`callgraph` 是**词法候选清单**、名字匹配、1 跳、自报 static-heuristic（`callgraph.rs:61`，`trace.rs:4-6` 也自陈这一点）；
  `impact` 有 `depth` 上限且按名匹配（`clamp(1,…)`，A4）；
  `trace` 那侧反而**守着一条硬规矩**：artifact 描述的是另一棵树时**拒绝**出报告（`trace.rs` 模块文档："a report is refused when the artifact describes a different tree"，因为帧是节点身份）；
  `converge` 组合面级判断。跨工具口径不一致：F1（两个内核结论）、A1（两种帧口径）、A2（两种错误约定）。
- **缺口**：不是"跳数浅"，而是**同一问题多答**。深度链现在最贵的地方恰恰是维护者最在意的那点：**不稳**。
- **值不值得做**：**值得，但先修口径再谈深度。**加跳数会**放大**按名匹配的误差（一跳错，整条链错），
  这正是"口述参考不一定有我省 token"的反面案例。
- **最小可验证切片**：把"哪个面跑了 / 这棵树被接受吗"收敛到**唯一判定入口**（S1 + S3），再谈多跳。
- **风险**：用"看起来更深"换掉"当下可核对"，是本仓历史上最贵的一类返工（`callgraph.rs` 模块文档里那次 142 个同名定义 / 4.5 MB 回复的实测记录）。

### ④ 质量类 MCP（找问题、写伪代码、减冗余）
- **现状**：**零**。17 个工具全是"读注册面 / 查符号 / 写注册面"，没有一个产出"这段实现有问题"的判断；
  质量判断目前在 `cargo test`/`clippy`/审计文档这边，MCP 侧没有对应物。
- **缺口**：连"什么算问题"的定义域都没有（clippy 已覆盖一部分）；伪代码与冗余发现需要跨函数语义，
  而现有符号面是名字匹配 1 跳 + fn/文件/面三维索引（L1、R3）。它是 t1 的 G6（能力扩张）的下游。
- **值不值得做**：**现在不值得。**它最贵的失败模式是误报：agent 会把它当结论，而信任预算一旦花掉不可回收（F4 就是学费）。
  要做的第一版也不该"给结论"，只该"给证据"。
- **最小可验证切片**（若做）：不改 MCP，先把 `usages` 已有原料（声明字段 vs 生成实现里的实际用法）重排成
  "面 × 声明 vs 用法不一致"的**证据清单**，人判、不自动报错。
- **风险**：误报即噪音；以及"质量"这个词会诱使它越过内核词汇去下判断，等于给 MCP 加了第二套判定面——正是 F1 的成因。

## ⑥ 结论与排序

**一句话结论**：t1 的工作流**不是表演**——138 次 `tools/call`（+1 次 `tools/list`）里只有 10 次无结论引用，注册面上 MCP 是承重的；
但它在符号面上把"grep 排好版"记成了"更稳"（① 结论 2），而它自己挖出来的四条真问题（F1/F2/L1/A2）都没被它放到第一位。
按维护者的判据"一定稳"，**MCP 今天最不稳的不是浅，是同一件事有两种说法**。

排序（按 **误答风险 ÷ 改动成本**）：

| 序 | 切片 | 对应 | 成本 | 风险收益 |
|---|---|---|---|---|
| **S1** | `verify` 的回复在 `verdict ok` 之外**再加一行连接器判定**（复用 `load_registry`，保留静态面原词，绝不改动 CLI `check` 的同一入口不变量） | ①的前置 + ③的"稳" | **≤15 行 + 1 条钉子**（`verify.rs` + `verify_tests.rs`） | 消掉最重的 F1：agent 不再"拿了绿灯再处处被拒" |
| S2 | `mir jsonl` 的截断说明**移出 JSONL 正文**（写成合法的 `{"kind":"truncated",…}` 记录或走 stderr），并钉住 >400 行回环 | "稳"（承诺与行为一致） | ≤20 行 + 1 条测试（`mir.rs:42/467`） | 消掉 F2 的自坏通道 |
| S3 | `converge trace` 的帧归属改用 **node 身份**（`converge_trace.rs:75-77`），或两处都印两种口径 | ③ | 小 | 消掉 A1；深度链的第一块地基 |
| S4 | 把 `apply.fields` 的取值形状写进 `tools/list` 的 schema/描述（module 裸 snake_case、值必须字符串、`handle_traits` 是标签） | ①（信任载体） | 小、纯文档/JSON | 省掉 M1–M3 的试错，且把 M3 的错提前到预览期 |
| S5 | ① 的后半：可信度**标记**（需先定义"稳"的判据 + 一个消费方） | ① | 大（新格式 + 门禁） | 判据没定之前不做 |
| S6 | ③ 的多跳链/路径（需要先修口径） | ③ | 大（能力扩张） | 先 S1/S3 |
| — | ② diff 链、④ 质量类 MCP | ②④ | 很大 | **本轮不做**：②依赖 S5/S6，④连"问题"的定义域都没有 |

**排名第一且最便宜 = S1。** 理由只有一条，且是维护者自己的判据：
它是唯一一条**今天就会让 agent 信错**、改动却不超过 15 行、并且能用一个已有 fixture 直接钉住的切片
（在同一棵树上同时断言 `verdict ok` 与 `connector verdict: rejected` 必须并存）。
它也是 ① 的**最小可落地前置**：先让"信任"只有一种读法，再谈"标本"。

**给 t3 的落地口径（S1）**：改 `toolchain/src/mcp/verify.rs`，在 `check_for` 之后调用已有的
`crate::mcp::apply::load_registry(root, &package)`（`apply.rs:510`，错误串已经就是
`the package's own faces were rejected: …`），把它的成败作为**独立的第二行**印出（用同一个 `MAX_DIAGNOSTIC_LINES` 截断），
`verdict ok` 的字面**保持不变**——因为"静态面通过"仍然为真，`verify.rs:5-11` 与 CLI `check` 的不漂移不变量必须守住。
验收命令就是我复现 X1 的那一对调用：一个会话里先 `apply`（预览，被拒）再 `verify`，`verify` 的回复必须同时含两面。

## ⑦ 队长补记（2026-09-29）：§④ 第 1 条的"不可复现"是读错了参数

§④ 第 1 条据"同一句两次得到 9 与 20 个符号"判"单次转录不是可复现的测量"。**这句不成立**，更正如下。维护者提示 codegraph 的 CLI 装在 `~/.local/bin`（默认不在 PATH），加进 PATH 后实测：

- `codegraph explore` 对**同一句查询 + 同一个 `--max-files`** 是**逐字节可复现**的：同一句连跑两次，21913 字节、sha256 相同；换 `--max-files 1` 再连跑两次，同样逐字节相同。
- 那个 `Found N symbols across M files` **是被 `--max-files` 卡住的**，不是"索引里有多少"：同一句查询，`--max-files 1` → `9 symbols across 1 file`（**正是队长那次 maxFiles=1 的读数**），`--max-files 2/3/5` → `20 symbols across 2 files`（**正是本审计复核者那次的读数**）。
- ⇒ 9 与 20 的差别来自**参数**，不是随机性；原判据里的"文件集合也不同"同理（队长那次只许 1 个文件）。

**结论保留，但理由要换**：这份输出仍然**不能当完整性依据**，因为那个计数由显示旋钮决定，而输出里没有任何一处说明这件事——它长得像"我找到了 N 个符号"，实际是"我在这 M 个文件里放了 N 个"。拿它做影响面判断前必须自己指定 `--max-files`，并知道它截掉了什么。

复现命令（本机实测 2026-09-29）：

```sh
export PATH="$HOME/.local/bin:$PATH"   # codegraph 1.5.0，装在 ~/.local/bin，默认不在 PATH
Q='List the intermediate functions on the path from Registry::register_snapshot_batch in transaction.rs to connector_error in connector.rs, as a call path with each hop named'
codegraph explore --max-files 1 "$Q" | head -3
codegraph explore --max-files 3 "$Q" | head -3
```

另：措辞确实会改变**拿回哪些文件**（带 "in transaction.rs" 的那句给 `diagnostic/error.rs` + `connector.rs`；第一臂那句给 5 个源块，含真正的起点 `transaction.rs` 与终点 `connector.rs`）⇒ §④ 第 1 条"写死反而更差"的观察成立，但它与"不可复现"是两件事。
