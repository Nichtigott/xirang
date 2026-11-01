# 第九轮 · 四道 hardbug 类逐题对话层分析

范围：`h1-supply-chain` · `h2-claim-unkept` · `h3-cross-file-chain` · `h4-one-file-many-algorithms`。
**只做对话层**：质量判定与代价数字全部引 `README.md` §三/§四，未重做判定、未重算代价、未改任何题树/日志/答案。

**引用体例**：`brief/<arm>/<id>.md ①N` = 该题精简版里第 N 次仪器调用的段落（含工具返回逐字与"当时推理"）；
`full/<arm>/<id>.md L###` = 完整版（本文件写作时读取的是 **2026-10-02 23:52:30–31 重生成**的那一版，行号以该版为准）；
`logs/…` = `target/round9/logs/<id>.jsonl`（我们臂）与 `target/probe-cg26/logs/<id>.txt`（cg 臂）；
源码行号引本检出（`toolchain/src/mcp/*.rs`、`kernel/src/…`）与规格 `docs/design-hardbug-bench-spec.md`。

**三条口径（先说清，后面不再重复）**：

1. **调用数一律以逐题日志条数核对**（README §六纪律 3）。核实结果与表一致：`h1` 我们 4 / cg 10（另 cg 有 2 次非桥 `cargo test`）、`h2` 5 / 7（另 cg 3 次非桥）、`h3` 3 / 13（另 cg 1 次非桥）、`h4` 2 / 9（另 cg 2 次非桥）。
2. **`步/输出/推理/上下文` 四栏不是该题的净代价**（README §三）：本批是一会话跑 26 题、且我们臂**一道题一条 bash 里连发多次桥调用**（`full/ours/h4-one-file-many-algorithms.md` 里 `h4` 的第 2 条直接标"会话里未定位（共享/装置步）"）。同一题的"当时推理"经常写的是**别的题**（例：`brief/ours/h1-supply-chain.md ①1` 挂的推理整段在规划 `h3`/`h4`，`brief/ours/h4-one-file-many-algorithms.md ①1` 挂的推理整段在总结 `h3`）。凡本文引"推理"，都注明它实际在讲哪一题；讲本题的推理取证自 `full/` 的具体步（多为 step 145–153 那几步）。
3. **两侧的"读文件"记在不同口袋**：我们臂的 `grep -n ''`/`sed -n`/`for f in …` 逐文件读**不进桥日志**（在 `full/ours/<id>.md` 的"其它调用"里），cg 臂的 `node <file>` **在日志里**、算仪器调用。因此"4 vs 10"这类差额里有一部分只是**记账口径**，逐题会点出来。

**出题台给的理想路径**（`python3 tools/xirang-mcp-hardbug plan` 逐字 + `docs/design-hardbug-bench-spec.md` 对应节）：

| 题 | 理想路径（plan 逐字） | 规格补充（spec 行号） | 判据落在哪 |
| --- | --- | --- | --- |
| `h1-supply-chain` | `consistency --parent root/control --by api` | "**一次**点名离群 ⇒ 再 `read` 那几行"（spec:203） | `consistency` 的 `outlier` 行 |
| `h2-claim-unkept` | `conformance --anchor <specimen>` then `consistency --specimen <specimen>` | "⇒ 两次调用内给全"（spec:212-213） | 台账判定 + `outlier … lacks …` |
| `h3-cross-file-chain` | `why --at <symptom line>` | 实测 `why --at src/control/object/dial/dial.rs:16` 一次给出 `scope not-selected` + `wiring …`（spec:36-39） | `why` 的 `wiring` 行 |
| `h4-one-file-many-algorithms` | `digest --file <path>` then `read {path, line}` | 红在 `check --face default --census` 的**分支栏**（spec:181） | census 的 `branch-level` 明细行 |

---

### `h1-supply-chain` · 同族一个兄弟调用了另一族名字（偏移总和 160≠136）

题面（`target/round9/trees/h1-supply-chain/BRIEF.md` 逐字）："The rendered layout is off. `cargo test --offline` fails in the offsets test with a total that no single widget's own code explains, and every widget file reads as correct on its own."；交付要五项，其中 `evidence` 要求"the command you ran, its raw output, and its exit code"。

**我们臂的路径**：4 次仪器调用（`logs/h1-supply-chain.jsonl` 4 条，全 exit 0）＝
`consistency {parent:root/control}` → `why {at:src/control/object/toggle/toggle.rs:15}` → `callgraph {function:offset}` → `check {face:default}`。

- **①1 `consistency --parent root/control`**（日志第 1 条，step 149）——**理想路径的那一次**，只是省了 `--by api`。逐字给了整个同族与离群：
  `family root/control · member xirang-example-control-button · 9 member(s)`、
  `  toggle                   1 call(s): to_world`、
  `  outlier     toggle: does not call \`to_local\`, which the other siblings call; calls \`to_world\`, which no sibling calls`、
  `outliers: 1 of 9`；并自曝边界：``and `api` compares the names called, not the units or the arithmetic — an outlier is a place to look, not a defect``；
  `next   \`read {path, line}\` for the outlier's body, \`explain {node}\` for its declared fields`（`brief/ours/h1-supply-chain.md ①1`）。
- **①2 `why --at toggle.rs:15`**：给契约行与接线——
  `  contract   (from the doc above it, which is the promise this tree writes)` / `src/control/object/toggle/toggle.rs:14 /// This widget's offset, in the coordinate family its siblings use.`、
  `  callers    tests/offsets.rs::the_rendered_offsets_add_up (outside this file's directory (tests))`、
  `  wiring     declared cut \`crate::control::object::toggle::NODE_ID\` → \`control_button_graft::toggle_fast::NODE_ID\` (full=false) at …/src/lib.rs:48`（`brief/ours/h1-supply-chain.md ①2`）。
- **①3 `callgraph --function offset`**：`matches 9`，前 5 条各 `callees: to_local -> src/control/control.rs:5`；末行把**离群自己截掉了**：
  `… truncated: 4 of 9 definitions withheld at the limit of 5; pass \`path\` for one of: \`src/control/object/slider/slider.rs\`, \`src/control/object/spinner/spinner.rs\`, \`src/control/object/timeline/timeline.rs\`, \`src/control/object/toggle/toggle.rs\`; \`limit\` raises this cap`（`brief/ours/h1-supply-chain.md ①3`）。
- **①4 `check --face default`**：`verdict  failed (cargo exit 101)`、`failed the_rendered_offsets_add_up`、
  `why    the_rendered_offsets_add_up: thread 'the_rendered_offsets_add_up' (514984) panicked at tests/offsets.rs:18:5: assertion \`left == right\` failed: the rendered offsets add up to 160, not 136`（`brief/ours/h1-supply-chain.md ①4`）。
- **决定性的一步＝①1**，而且它在**调用之前**就被规划好了，推理逐字（讲本题）：
  "For h1-supply-chain: `callgraph`/`consistency --parent root/control` (should name toggle as the outlier? \"compares the siblings' own calls\" — the API column compares called names: toggle calls `to_world`, siblings call `to_local` → the tool should name toggle as the outlier!)"（`full/ours/h1-supply-chain.md L61`）。
- **但"再 `read` 那几行"这半边走的是 shell，且发生在理想调用之前**：step 146 里并行发了两次 shell，其一先 `for f in src/control/object/*/*.rs; do … grep -n "fn offset\|offset(" -A 6 $f …` 把**九个兄弟全读了**，其二再 `grep -n '' src/control/object/toggle/toggle.rs; … grep -rn "fn to_local\|fn to_world" src/; … grep -n '' src/control/control.rs` 把两族算术拿到（`full/ours/h1-supply-chain.md` 的"调用 `bash`（step 146）"段及其"同一步并行发出的其它调用"；算术逐字 `src/control/control.rs:5:pub fn to_local(x: i32) -> i32 { x + 7 }` / `:8:… { x + 31 }`，同文件 L458-459）。随后 step 149 的推理直接引用它：**"I have it from the earlier grep: to_local = x+7 at :5, to_world = x+31 at :8）✓"**（`full/ours/h1-supply-chain.md L108`）。也就是说，`consistency` 这一次是**确认**而不是发现——本类"必拒捷径"的"逐个读完再猜"没被触发（它没有猜，证据在手上），但工具想省掉的那段读已经先付掉了。

**codegraph 的路径**：10 次仪器调用（`logs/h1-supply-chain.txt` 10 条 `=== CMD`）+ 2 次非桥 `cargo test`（第 1 次管道里写死 `exit 101`、第 2 次 `rc=$?` 真取）＝
`init .` → `files` → `node host/tests/offsets.rs` → `node host/src/lib.rs` → `node host/src/control/control.rs` → `callers to_world` → `callers to_local` → `callees offset` → `node …/toggle/toggle.rs` → `node …/button/button.rs`。

- 前 4 次（含 `init`）是定向：`Indexed 24 files · 95 nodes, 121 edges in 578ms`（①1）；`files` 列出 24 个文件（①2）；`node host/tests/offsets.rs` 给断言源文 `const EXPECTED_TOTAL: i32 = 136;` 与八个被加总的部件（①3）；`node host/src/lib.rs` 给 8 条切口的 graft 计划（①4，本题用不上，服务 `h3`）。
- 第一个**能下结论**的调用是第 5–6 次：`node host/src/control/control.rs` 给出两个族与算术
  `pub fn to_local(x: i32) -> i32 { x + 7 }` / `pub fn to_world(x: i32) -> i32 { x + 31 }`（①5），紧接着
  `callers to_world` 逐字：`Callers of "to_world" (1):` … `function    offset` / `  host/src/control/object/toggle/toggle.rs:15`（①6）。
  第 7 次 `callers to_local` 给 8 个（①7），第 8 次 `callees offset` 一次列全九个定义、最后一条是
  `function offset (rust) — host/src/control/object/toggle/toggle.rs:15 (1):` / `function    to_world` / `  host/src/control/control.rs:8`（①8）——等价于我们 ①1 的那一行，但要从"名字"绕。
- 确认在 `node …/toggle/toggle.rs` + `node …/button/button.rs`：推理逐字（讲本题）
  **"Perfect: `toggle.rs:3` imports `to_world` (vs siblings importing `to_local`), and `toggle.rs:16` calls `to_world(x)`. Same doc comment on both (\"in the coordinate family its siblings use\") — the toggle's doc is identical to the siblings', so each file \"reads as correct on its own\" ✓ (the class statement)."**（`full/cg/h1-supply-chain.md L665`）。
- 非桥 `cargo test` 之前它还权衡过成本，逐字：**"The cargo run adds \"the test actually fails with 160\" — nice-to-have but expensive."**（`full/cg/h1-supply-chain.md L677`）。

**两侧差异与归因**：

1. **信息差（主因）**：`consistency --parent` 是"同族彼此比"这一个问题的一次调用；codegraph 的图里**没有"同族"这个概念**，只有 `callers <名字>` / `callees <名字>`，所以它必须先知道要比哪两个名字——于是它花 4 次定向（`init`/`files`/测试/入口）+ 1 次读 `control.rs` 才拿到两个族名，再用 2 次 `callers` + 1 次 `callees` 做"多数派 vs 离群"的人肉比较。离群是**推**出来的（1 个 vs 8 个），不是被点名的（我们 ①1 逐字 `outlier toggle: …`）。
2. **流程差（我们侧，可改）**：一次 `callgraph {function:offset}` 本可以替代那 2–3 次比较，但它的默认 `limit=5` 恰好把 `toggle.rs` 截在 withheld 里（①3 逐字）。agent 于是回去用 shell 逐文件读——这也是本题我们臂"读九个兄弟"的来路。
3. **记账口径**：cg 的"读文件"全在日志里（4 次 `node`），我们的等价读（step 146 的两次 shell：`for f … grep`、`grep -n ''`）不在桥日志里。按仪器调用数比是 4 vs 10；按"为了拿到同一条结论实际发起的调用"比，我们这边还要加 2 次 shell（它们不计入 4）。
4. **`--by api` 是默认值**：本检出 `toolchain/src/mcp/consistency.rs:519-523` 逐字 `.get("by").and_then(Value::as_str).unwrap_or("api")`，取值只接受 `api|kind|source`。理想路径把它写成必需项，是**写法**而不是行为差异。

**代价**（README §三）：我们 4/0/0/0/0 · codegraph 10/0/0/0/0。
两栏的"步/输出/推理/上下文"都是 0——这一题的两侧调用都落在共享/装置步里（`full/ours/h1-supply-chain.md L9` 逐字："⚠️ 日志 4 条 vs 会话定位 2 条：差的那些调用发生在**共享步**（一条命令里跑了多道题）或**装置步**里 —— 这正是'逐题 token 不可归因'的具体形状。"），因此本题**没有可比的净 token 读数**，只有仪器调用数可逐题归因。

**质量**（README §四）：我们 `h1-supply-chain` 属"四道 hardbug 4/4 与登记真值逐条吻合"✓；codegraph 侧 24/26 完全命中，`h1` 不在两处"部分"（`s3`/`g1`）之列 ✓，也没有把非缺陷报成缺陷。

**引导含义**（每题要具体到可实施）：

1. **`consistency` 的 `outlier` 行应当直接带上离群对象的文件（或把 `next` 行填好）**。现在它只说对象名（`outlier     toggle: …`），`next` 是通用文案 `` `read {path, line}` for the outlier's body ``，`{path, line}` 没给；agent 于是回 shell 取 `toggle.rs:16`（`full/ours/h1-supply-chain.md` step 146 + 答案 root cause 行 `src/control/object/toggle/toggle.rs:16`）。同一条返回里工具已经能把对象解析成面（`registry` 就印 `root/control/toggle Toggle control/object/toggle/toggle.rs`），把文件补进 `outlier` 行是零成本的。
2. **`callgraph` 的 `limit` 应该对"同族里不一致的那些定义"网开一面**。依据：①3 逐字 `… truncated: 4 of 9 definitions withheld at the limit of 5; … \`src/control/object/toggle/toggle.rs\``——被截掉的正是唯一要看的那个。最低成本的改法是让 `withheld` 那行不只列路径，还标出"其中 N 个的 callee 与多数派不同（需先比一次）"。
3. **`check` 在断言失败时的 `next` 行没有指向本题最短的路**。它逐字指向文本搜索（"a short, stable phrase from one is a string literal in this tree, so `search {literal: \"the rendered offsets add up to\"}` finds the line that produced it"，`brief/ours/h1-supply-chain.md ①4`）。当失败的断言把**一族兄弟**加总时（测试源码 `full/ours/h1-supply-chain.md` 里 `offset(10) + …` 八项），`next` 应加一句 `consistency --parent <该断言的父面>`——否则 agent 的自然动作就是 step 145 那种"逐文件 grep"。
4. **流程表/理想路径里把 `--by api` 写成可选**（依据：`consistency.rs:519-523` 的默认值），免得读者以为还有别的选择要先定。

---

### `h2-claim-unkept` · 真标本台账 + 一个同族只缺一条声明 + 一条指纹失效

题面（`…/h2-claim-unkept/BRIEF.md` 逐字）："A constraint this design adopted is not holding on a later sibling, and one ledger entry no longer describes the bytes it names. The tree builds and its tests pass: the question is what the ledger and the sibling shapes say, not what a test run says."

**我们臂的路径**：5 次仪器调用（`logs/h2-claim-unkept.jsonl` 5 条，全 exit 0）＝
`adopted` → `conformance {anchor:root/control/button}` → `conformance {anchor:root/control/panel}` → `consistency {specimen:root/control/button}` → `check {face:default}`。

- **①1 `adopted`**（日志第 1 条）——一次给全"谁是标本、哪条失效、失效在哪个文件"，逐字：
  `adopted since 2026-10-01T10:00:00+08:00 (unchanged since the confirmation) (provisional) — root/control/button: the reference sibling shape [evidence: traced once; confirmed by nich; why: the shape every sibling follows]`、
  `adoption lapsed at src/control/object/panel/panel.rs (changed since the confirmation); the recorded print is \`root/control/button\`'s current print, not this file's — a copy of another entry's bytes; needs confirmation — root/control/panel: the panel's original contract [adopted at 2026-09-20T09:00:00+08:00 by nich; why: recorded before the panel was rewritten]`、
  `provisional 1  lapsed 1`，以及规则句 `note: … a lapsed one needs a person to confirm the new state, and a confirmation is one more line.`（`brief/ours/h2-claim-unkept.md ①1`）。
- **①2 `conformance --anchor root/control/button`**（理想路径的第一次调用）：`in force   provisional — certifies: the reference sibling shape (adopted 2026-10-01T10:00:00+08:00 by nich)`、`bytes      unchanged since the confirmation`、`covers     src/control/object/button/button.rs`；它的 `next` 行正好点名理想路径的第二次调用。
- **①3 `conformance --anchor root/control/panel`**：`in force   lapsed at src/control/object/panel/panel.rs: the bytes moved after the confirmation, so this needs a **person**, not an edit (adopted 2026-09-20T09:00:00+08:00 by nich)`、`bytes      changed since the confirmation (src/control/object/panel/panel.rs)`（`brief/ours/h2-claim-unkept.md ①3`）。
- **①4 `consistency --specimen root/control/button`**（理想路径的第二次调用）：`shape      exports \`control.render\` · handle_traits \`ControlHandle\``、
  `  outlier     spinner: lacks \`handle_traits\``、`conformance: 1 of 8 sibling(s) do not carry the specimen's shape`（`brief/ours/h2-claim-unkept.md ①4`）。
- **①5 `check --face default`**：`verdict  passed (cargo exit 0)`（`brief/ours/h2-claim-unkept.md ①5`）——本题"树是绿的"这条题面要求的证据。
- **决定性的一步＝①1＋①4**：这两次正好是两条真值（"谁缺哪条声明"＋"哪条采信在哪个文件失效"）。①2 与 ①3 是 ①1 的 `next` 行引出来的（①1 的 next 逐字：`` `conformance {anchor: "root/control/panel"}` says whether that lease still holds and where it lapsed ``），而 ①1 自己的正文**已经答过**同一件事（`adoption lapsed at src/control/object/panel/panel.rs …`）。①2 的 next 引出的 ①4 才是没答过的那半。

**codegraph 的路径**：7 次仪器调用（`logs/h2-claim-unkept.txt` 7 条 `=== CMD`）+ 3 次非桥（`cat .xirang/adopted/entries` 455 B；`cargo test` 1013 B；`sha256sum` + 逐兄弟 `diff` 2225 B）＝
`init .` → `files` → **非桥** `cat …/.xirang/adopted/entries` → **非桥** `cargo test` → `node host/src/control/registry_rule/registry_rule.rs` → `node host/src/control/object/spinner/spinner.rs` → `node host/src/control/control.rs` → `query REGISTRATION_RULE` → `callers REGISTRATION_RULE` → **非桥** `sha256sum button/button.rs panel/panel.rs && for o in …; do diff button/button.rs $o/$o.rs; done`。

- 台账那半来自**读文件 + 自己比**：`cat` 逐字两行，两行指纹**逐字相同**
  `root/control/button|the reference sibling shape|traced once|nich|2026-10-01T10:00:00+08:00|src/control/object/button/button.rs|edc72845…|the shape every sibling follows` /
  `root/control/panel|the panel's original contract|traced once|nich|2026-09-20T09:00:00+08:00|src/control/object/panel/panel.rs|edc72845…|recorded before the panel was rewritten`（`logs/h2-claim-unkept.txt` 第 65-68 行）；`sha256sum` 给两个文件的**字节** sha256（`06c45576…` / `d886e9b0…`，都不等于 `edc72845…`）。判定是**推**的，推理逐字："两条记录写了同一个指纹，而两个被指名的文件字节不同（06c45576… / d886e9b0…）⇒ 至少一条不描述它指名的字节；panel 那条自己写着'在 panel 被重写之前记录的'（时间 2026-09-20，早于 button 的 2026-10-01）⇒ 失效的是 panel 那条。"（`brief/cg/h2-claim-unkept.md ①1`）。
- 声明那半同样是读＋比：`node …/spinner/spinner.rs` 给 `19	crate::control_object! {` … `23	}`（缺 `handle_contracts`），逐兄弟 `diff` 只有 spinner 多一个 hunk：`23d22` / `<     handle_contracts: [crate::control::ControlHandle],`（`logs/h2-claim-unkept.txt`）。
- **两个明确的图缺口**（cg 自己写下并绕开）：`callers REGISTRATION_RULE` 逐字 `Callers of "REGISTRATION_RULE" (0):` + `ℹ No callers found for "REGISTRATION_RULE"`（宏引用的边没进图），答案 ④ 自陈"所以我没有依赖'谁引用了规则'来下判断，而是回源码读 `registry_rule.rs`"（`brief/cg/h2-claim-unkept.md` 交付答案 ④）。
- **指纹方案的那段弯路**（本题推理量的主要来源）：字节 sha256 对不上之后，推理逐字："If neither matches, my assumption about the fingerprint scheme is wrong, and I'd need to find the scheme in the toolchain source…"（`full/cg/h2-claim-unkept.md L710`）、"The scheme might be documented in the workspace's toolchain source (call_evidence / adoption), which I'm not supposed to grep…"（同文件 L768），中间枚举了"是不是 shape 的 hash""是不是 node id""是不是 macro block"等假设（L764-795）。**在本轮答题时它没有重算出那枚指纹**：会话记录里第一次成功的复算发生在**改正轮（turn 4, step 4）**，且方案是复核者给的（该次复算的完整命令与结果在 `full/cg/h2-claim-unkept.md` 的"**调用** `bash`（step 4）"段，L803-811；`target/probe-cg26/CORRECTIONS.md` §1 逐字"依据：复核者…给出的 `adoption_fingerprint` 拼法"）。即：交付版答案的那句"可重算"是**事后**补的。

**两侧差异与归因**：

1. **信息差 #1（判定 vs 文本）**：`lapsed` 不是台账文件里的一行字，而是桥**算**出来的（读被指名的文件、与记录指纹比对：`state_of`/`adoption_fingerprint`）。逐字的差别就是这一句：我们 ①1 `adoption lapsed at src/control/object/panel/panel.rs (changed since the confirmation); the recorded print is \`root/control/button\`'s current print, not this file's — a copy of another entry's bytes`；cg 侧拿到的是同一份**没有结论的**两行文本，于是它要 `cat` + `sha256sum` + `diff`，还要判"指纹方案是什么"。
2. **信息差 #2（面字段 vs 源文本）**：`consistency --specimen` 用内核的面解析器读声明字段并比（①4 逐字 `shape exports … handle_traits …` + `outlier spinner: lacks handle_traits`）；codegraph 图里没有"注册面字段"，cg 只能把 8 个兄弟文件逐个 `diff` 出那一个缺行 hunk。
3. **流程差（我们侧，可改）**：①1 的 `next` 行把调用方送回它自己刚答过的问题。若按"未答的那半"给 next（`consistency --specimen root/control/button`），5 次可降到 4 次；①3 与 ①1 的信息重叠度极高（①3 只多给了 `needs a **person**, not an edit` 的措辞与 `bytes changed` 的判据）。
4. **记账口径**：cg 的 3 次非桥里有 2 次是**必须**的（台账与哈希都不在图里）；那 2 次也解释了它"仪器 7 但 log 10 条"的形状。

**代价**（README §三）：我们 5/7/9,127/13,020/2,076,544 · codegraph 7/5/10,418/28,162/1,041,024。
**仪器调用我们少 2 次，输出我们少 1,291 tok**；**推理 cg 是我们的 2.16 倍**（28,162 vs 13,020），而多出来的那部分能指到具体段落：指纹方案的假设枚举与"能不能去读工具链源码"的自我约束（`full/cg/h2-claim-unkept.md L710`、L764-795），以及 `callers REGISTRATION_RULE` 为 0 之后回源码的决定。**上下文**那一栏 cg 反而是我们的一半（1,041,024 vs 2,076,544），属状态机归属，不作本题结论。

**质量**（README §四）：我们 4/4 hardbug 命中 ✓；同一节记"`h2` 的 fix 含违禁项'删掉该失效条目' ✗"（判定不受影响）。**注意口径**：交付现状（`target/round9/answers/h2-claim-unkept.md` 的 `fix` 行）写的是"台账**不能**靠编辑修：按工具规则，失效的条目需要**人**再追加一行确认（…`apply/confirm`…），**旧行保留为历史**"——不含违禁项。cg 侧同样：README §四记 cg 的 `h2` fix 违禁，而 `target/probe-cg26/answers/h2-claim-unkept.md` ③ 现为"**台账既有条目一字不改、不删**……'**需要人，不是改代码**'"，且 `target/probe-cg26/CORRECTIONS.md` §1 记了这次改正（"原'…（**或删掉该失效条目**）' → …"）。⇒ 两份**记分版**含违禁项、两份**当前交付版**都已改；这一点差异写在文末"没能判定"。

**引导含义**：

1. **`adopted` 的 `next` 行不要指向它自己刚答过的问题**。依据：①1 正文逐字 `adoption lapsed at src/control/object/panel/panel.rs`，而同一返回的 next 逐字 `` `conformance {anchor: "root/control/panel"}` says whether that lease still holds and where it lapsed ``，随后 ①3 又答了一遍。改法二选一：(a) next 只列"本条没有判过的锚点"（本题一个都没有 ⇒ next 直接给 `consistency --specimen root/control/button`）；(b) 让 `adopted` 与 `conformance` 合并成一个"每条一行"的视图——`conformance` **已经**支持不带 `--anchor` 的"每个锚点一行"（`toolchain/src/mcp/adopted.rs:98-146`，源码注释逐字 "Measured (T-21): asking about two entries cost two calls, because the argument was required … 不带 anchor 就是**每个**锚点一行"），只是 `adopted` 的 next 文案还停在旧能力上。
2. **把 `lapsed` 判据做成可核对的字面量**：现在写的是"the recorded print is `root/control/button`'s current print, not this file's"——**没有给两枚指纹**。补齐成 `recorded edc72845… (= src/control/object/button/button.rs 的当前指纹) · src/control/object/panel/panel.rs 的当前指纹 57afd3b6…`，则任何一侧都不必去猜哈希方案（cg 侧为此付出的推理见 `full/cg/h2-claim-unkept.md L710/L768`，本题 28k 推理字符里的一大块）。
3. **`conformance` 的 `next` 文案是好的范例**，可作模板：①2 逐字 ``next   `adopted` for every entry's verdict, `consistency --specimen root/control/button` for whether the siblings follow this shape`` ——它点名的是**下一步能拿到新事实**的那一次调用，而我们臂确实照做了（①4）。同一工具家族里 `adopted` 的 next 应改成同一风格。
4. **（对 cg 侧的口径提醒）**：本题 cg 的"must-refuse"是"把 `lapsed` 说成改代码"——它没有犯（最终答案写"需要人，不是改代码"），但它的**依据链**与我们不同：我们是工具给的判定句，它是 note＋时间戳＋两枚不同字节哈希的推断。这说明"拒绝文案"应当把**判据**一并印出（见第 2 点），否则同一条结论在一侧是事实、在另一侧是推测。

---

### `h3-cross-file-chain` · 入口计划没点名新面，构建期作用域把它剪掉

题面（`…/h3-cross-file-chain/BRIEF.md` 逐字）："A widget that exists in the sources does not appear in the shipped registry tree. `cargo test --offline` fails on the shipped-tree assertion."

**我们臂的路径**：3 次仪器调用（`logs/h3-cross-file-chain.jsonl` 3 条，全 exit 0）＝
`check {face:default}` → `registry` → `why {at:src/control/object/dial/dial.rs:15}`。

- **①1 `check --face default`**：`verdict  failed (cargo exit 101)`、`failed every_declared_widget_ships`、
  `why    every_declared_widget_ships: thread '…' panicked at tests/shipped.rs:19:9: \`dial\` is declared and does not ship; the tree is:`，census 里 `entry plan: 11 \`cut(\` site(s) and 9 \`graft(\` site(s) across this tree's sources (a static count; which of them this application ships is the plan's own business)`；其 `next` 行指向文本搜索（`search {literal: "\`dial\` is declared and does not"}`）（`brief/ours/h3-cross-file-chain.md ①1`）。
- **①2 `registry`**：`faces 10`，其中 `root/control/dial                        Dial           control/object/dial/dial.rs          e1070833a032e106f9cdd0eff2149bc2`——证明**源码侧有它**（派生树第 10 个面）。
- **①3 `why --at src/control/object/dial/dial.rs:15`（理想路径的那一次，理想行号是 spec 实测的 `:16`）**：唯一能下结论的那一行逐字——
  `  wiring     no declared cut in /home/nich/…/h3-cross-file-chain/host/src/lib.rs names \`root/control/dial\` — the entry declares 8 cut(s), and a face no cut names is one the build replaces with nothing (\`grafts\` lists them)`；
  同一返回还给 `  siblings   8 file(s) under src/control/object define \`offset\` too: …`、`  callers    0 in this root`、`  pins       no test names \`offset\``、以及两条"不知道"：`  scope      scope unknown (no source_scope.tsv; run \`xirang check\`)` / `  pruning    pruning unknown (no pruning_manifest.tsv; run \`xirang check\`)`（`brief/ours/h3-cross-file-chain.md ①3`）。
- **决定性的一步＝①3**。它的上线事实（"没有切口点名 dial"）就是答案的机制句；答案 ④ 逐字引它（`brief/ours/h3-cross-file-chain.md` 交付答案 ④）。
- **①1 为什么必须存在**：题面**没有点名是哪个部件**（只说"a widget"），所以必须先跑一次拿到 `` `dial` is declared and does not ship ``。**①2 的存在理由**是"源码侧没问题、差的是发布那一步"（答案 ③ 逐字引 `faces 10`）。
- **行号来自 shell**：答案的 `root cause` 要给 `文件:行号`，而 ①3 的 `wiring` 行只给入口文件路径（不给人 8 条切口的行）。agent 于是先自问 **"Let me see the full cut list to name a precise line … Let me print the file's plan block to give exact lines and to identify the insertion point."**、**"Root cause file:line: `host/src/lib.rs:48-…` — I need the line where the dial cut is missing … Let me get the exact text."**（`full/ours/h3-cross-file-chain.md L362/L364`），再在 ①1 那一步并行发的 shell 里读了 `host/tests/shipped.rs`、`graft/src/lib.rs`、`ls graft/src/`（`full/ours/h3-cross-file-chain.md` 的"调用 `bash`（step 151）"），最终落成 `:48-66` 与插入点 `:65/:66`。

**codegraph 的路径**：13 次仪器调用（`logs/h3-cross-file-chain.txt` 13 条 `=== CMD`）+ 1 次非桥 `cargo test`（exit 101）＝
`init` → `files` → `node host/src/lib.rs` → **`files`（逐字重复）** → **`node host/src/lib.rs`（逐字重复）** → `node host/tests/shipped.rs` → `node host/build.rs` → `node host/src/control/object/dial/dial.rs` → `callers dial::NODE_ID` → `callers button::NODE_ID` → `node host/src/control/registry_rule/registry_rule.rs` → `query dial` → `query dial_fast`。

- **图里没有"计划"这回事**：`node host/src/lib.rs` 头部逐字 `**host/src/lib.rs** — 98 lines, 3 symbols`——三个符号是 `FRAMEWORK`/`base_registry`/`outline`，`static_graft_plan!` 里的 8 组 `cut(…)/graft(…)` **不是节点**；两次想从图里抓它都以软失败收场：`callers dial::NODE_ID` → `ℹ Symbol "dial::NODE_ID" not found — did you mean: paint, Dial, DialFast?`（**exit 0**，`logs/h3-cross-file-chain.txt` 第 382-384 行），换成 `button::NODE_ID` 同样（第 385-387 行）。
- **结论靠读文件＋读宿主自己的注释**：`node host/src/lib.rs` 给整段计划（8 条切口、无 dial），`node host/tests/shipped.rs` 给被断言的 9 个名字（逐字 `        "dial",`），非桥 `cargo test` 给症状（`` `dial` is declared and does not ship; the tree is: `` 后 9 行无 dial）。答案的机制句因此引用的是**宿主文件里的说明文字**而不是任何工具判定：逐字 "`mechanism`：构建期作用域收窄到入口点名的子树（同文件 `:36-43` 的说明：没人声明的面不会被这个应用发布）"（`brief/cg/h3-cross-file-chain.md` 交付答案 ②）。
- **重复与软失败**（都可从日志单独判定）：`files` 与 `node host/src/lib.rs` 各出现**两次且参数逐字相同**——按出题台自己的白跑定义（`plan --scoring` 逐字："`repeat` — the same tool with the same arguments already ran earlier in this log"）这 2 次是 `repeat`；来路可见：同会话里 `h1` 的那一步就用 `./cg.sh h3-cross-file-chain node host/src/lib.rs … ; ./cg.sh h3-cross-file-chain files …` 先跑过一遍（`full/cg/h1-supply-chain.md L694`），到 `h3` 自己再跑一遍。两次 `callers …::NODE_ID` 的回复文本是信息，但**exit 0** ⇒ 任何按退出码读的指标看不见这次落空。

**两侧差异与归因**：

1. **信息差（结构性，解释大部分 13 vs 3）**：`why` 的 `wiring` 行是桥**从入口计划＋构建期作用域推出来的**一句判定；codegraph 里这条因果链**没有任何载体**——计划文本是宏调用、`NODE_ID` 是宏生成的、作用域不是 Rust 符号。所以 cg 侧必须"读计划文本 → 读测试 → 读宿主注释 → 自己推构建期行为"，本类"必拒捷径"里的"去看计划产物"在它那里**没有计划产物可看**。
2. **流程差（我们侧，可省 1 次）**：①2 `registry` 只为证"源码侧有 dial"；①3 的同一返回里已经有 `siblings 8 file(s) … define offset too`、`callers 0 in this root`，且 `wiring` 行本身就说明"它在派生树里、只是没有切口"。把 `wiring` 行写全（"this face is derived but no cut names it"）即可省掉 `registry`（3 → 2）。
3. **规格与装置的落差（两侧都受影响）**：规格说理想一次会给出 `scope not-selected (mode=auto)` + `wiring …`（`docs/design-hardbug-bench-spec.md`:36-39），但我们 ①3 拿到的是 `scope unknown (no source_scope.tsv; run \`xirang check\`)`——**而 ①1 的 `check` 就在同一步里先跑过**。按时间戳核对：本题 `logs/h3-cross-file-chain.jsonl` 的 mtime 是 21:35:50、答案 `answers/h3-cross-file-chain.md` 是 21:38:37，而该树 `host/target/xirang/out/` 里现存的那批产物（`source_scope.tsv`/`pruning_manifest.tsv`/`graft_plan.tsv`/`generated_lib.rs`）时间戳是 **22:47:45**、连 `check-default.log` 都是 22:46:41（都不是我们那两次调用的产物）⇒ 答题当时该目录里没有这些证据可读（`h1`/`h2` 两棵树至今也只有 `check-default.log`）。也就是说：本轮的树多数**没有被构建发布过证据**，理想路径里"scope"那半在装置上不成立，真正决定答案的是"wiring"那半。
4. **记账口径**：cg 的 13 里有 `init` 1 次、`files` 2 次（1 次重复）、`node` 6 次（其中 `host/src/lib.rs` 重复一次）、`callers` 2 次（软失败）、`query` 2 次；我们 3 次全是零副作用的只读判定（外加 shell 读了 `tests/shipped.rs`、`graft/`、`src/lib.rs`，不进日志）。

**代价**（README §三）：我们 3/4/6,521/13,740/1,193,088 · codegraph 13/1/1,981/2,305/164,352。
**仪器调用 3 vs 13（4.3 倍）是四题里差距最大的一题**；但注意 cg 那三栏（1,981/2,305/164,352）**低于**我们，因为它的大部分步发生在共享/装置步里、被记到别处——按 README §三的口径，本题只有"仪器调用数"可逐题归因，token 三栏不可比。

**质量**（README §四）：我们 4/4 命中 ✓；cg 24/26 完全命中，`h3` 有一处**措辞不准**（"要做减法但不改判定"）——判定仍是命中；没有把非缺陷报成缺陷。

**引导含义**：

1. **`why` 的 `wiring` 行要给行号（或给插入点）**——这是本题唯一可省下一次 shell 读的地方，而交付格式明确要 `文件:行号`。建议在该行尾补"8 cut(s) at src/lib.rs:50,52,54,56,58,60,62,64；no cut names `root/control/dial`（追加点在 :64 之后）"。依据：①3 逐字（只给文件路径）+ `full/ours/h3-cross-file-chain.md L362/L364`（"I need the line where the dial cut is missing … Let me get the exact text."）+ 答案 root cause `src/lib.rs:48-66`。
2. **`scope`/`pruning` 两行与 `check` 的联动要修**：现状是"`scope unknown (no source_scope.tsv; run \`xirang check\`)`"，而 `check` 跑完（①1）仍不产出该文件 ⇒ 这句指引是空转，且规格里承诺的 `scope not-selected` 永远看不到。可实施：`why` 读不到构建证据时，写"this tree has no published build evidence（`target/xirang/out/` 里没有 `source_scope.tsv`）；the `wiring` line is the answer"；或让 `check` 在写 `check-<face>.log` 的同时把这次运行的作用域证据落在同一目录（`toolchain/src/mcp/check.rs:269` 已经用同一个 `out_dir(root)`）。依据：①3 逐字；①1 的 `log …/target/xirang/out/check-default.log`；本树该目录当时的实际内容（只有 `check-default.log`）。
3. **流程表把 `registry` 那一步写成"可选"**：①3 的信息已足够回答"源码在、发布不在"，`registry` 只在需要"派生树 vs 发布树"字面对照时才要（本题答案 ③ 引了它）。这能直接省掉我们侧 1 次（3 → 2，正好等于"1 次症状 + 1 次理想调用"的下界）。
4. **（对 cg 类的能力缺口，解释差距用）**：`callers <face>::NODE_ID` 这种"宏生成名字"的查询以 **exit 0** 返回 "not found"（逐字），于是"图里没有宏展开"这件事在退出码上不可见；若要让这类 agent 少走弯路，schema 应把"宏生成的注册面名/计划文本"作为一等公民，或在 `node <file>` 的头部标注"此文件的宏调用内容未进图"。

---

### `h4-one-file-many-algorithms` · 一个文件三套算法，恰一支写反、恰一支没被覆盖

题面（`…/h4-one-file-many-algorithms/BRIEF.md` 逐字）："One algorithm in one file does the opposite of what its own documentation promises, and no test covers the branch that does it. `cargo test --offline` is green."

**我们臂的路径**：2 次仪器调用（`logs/h4-one-file-many-algorithms.jsonl` 2 条，全 exit 0）＝
`check {face:default, census:true}` → `digest {file:src/model/entry.rs}`。

- **①1 `check --face default --census true`**：`verdict  passed (cargo exit 0)`、`result test result: ok. 2 passed; 0 failed; …`（题面说的"绿"），而 census 的分支栏给出**本题真正的那条红**，逐字：
  `  branch-level: 1 constructively unreachable arm(s) in this tree (0 \`false\` guard(s), 1 never-constructed variant(s); a static read of the source text, not a coverage measurement)` +
  ``  no construction of `ZeroArm::Refuse` is spelled in this tree, so the arm matching it in `postable` at src/model/entry.rs:44 can never be entered (the enum is private, so a constructor outside this tree cannot spell the variant either); the contract above it says 34: Whether this entry may be posted at all. / 36: The rule the service leans on: an entry is postable when it carries a receipt, and a / 37: zero entry never is. Callers rely on the refusal, so this is a contract.``
  同一返回还给 `test-reachable: 2 of 5 …`、`fn     no test reaches \`signed\` (src/model/entry.rs:53)`、`fn     no test reaches \`normalized_account\` (src/model/entry.rs:60)`（`brief/ours/h4-one-file-many-algorithms.md ①1`）。
- **①2 `digest --file src/model/entry.rs`**：`file src/model/entry.rs — 5 function(s), 63 line(s)`，逐行给 5 个函数的行号、调用数、是否被测试点名、契约首句（例：`` `src/model/entry.rs:30-32 `zero_arm` — 0 call(s) out, 1 caller(s); contract: Which arm decides a zero amount.``），并自陈边界：``not covered here: which branches are dead and which are covered (ask \`check {face}\` for the whole-tree census), the contract in full (ask \`read {path, line}\`)…``（`brief/ours/h4-one-file-many-algorithms.md ①2`）。
- **决定性的一步＝①1**：它一次给全"哪一支（`ZeroArm::Refuse`）、哪一行（`src/model/entry.rs:44`）、与哪句契约相反（`:34/:36/:37` 原文）"，并给出**结构性理由**（私有枚举 + 本树无构造点 ⇒ 不是"测试恰好漏了"）。答案 ② 逐字引这一段（`brief/ours/h4-one-file-many-algorithms.md` 交付答案 ②）。
- **缺陷行 `:31` 来自 shell**：census 指的是**不可达那一臂**（`:44`），而写反的是 `:31` 的返回值；逐行的 `zero_arm` 函数体来自同一题的非桥调用（`full/ours/h4-one-file-many-algorithms.md`"调用 `bash`（step 148）"：`cargo test` + `grep -n '' tests/postable.rs src/model/entry.rs`）。理想路径的第二半 `read {path, line}` 在这里同样被 shell 读替代。

**codegraph 的路径**：9 次仪器调用（`logs/h4-one-file-many-algorithms.txt` 9 条 `=== CMD`）+ 2 次非桥（`scratch-h4` 跑一个独立调用方 632 B；树内 `cargo test` 419 B）＝
`init` → `files` → `node src/lib.rs` → `node src/model/model.rs` → `node src/model/entry.rs` → `node tests/postable.rs` → `callers zero_arm` → `callees postable` → `callees a_receipted_entry_posts`。

- **`digest` 的那张表被手工重建**：`node src/model/entry.rs` 给整份文件（头部逐字 `**src/model/entry.rs** — 64 lines, 9 symbols`，正文含 `:30-32 fn zero_arm(&self) -> ZeroArm { ZeroArm::Post }` 与 `:34-46` 的契约＋ `postable` 体）；`node tests/postable.rs` 给两个用例（都只用 `Entry::new("a", 5, …)`）；`callers zero_arm` → `Callers of "zero_arm" (1):` / `method Entry::zero_arm (rust) — src/model/entry.rs:30` / `method      postable` / `  src/model/entry.rs:38`；`callees postable` → 只有 `zero_arm`。
- **图回答不了"这条分支被覆盖了吗"**：`callees a_receipted_entry_posts` 逐字 ``Callees of "a_receipted_entry_posts" (0):`` + `ℹ No callees found for "a_receipted_entry_posts"`——测试里那句 `Entry::new("a", 5, true).postable()` 的**链式调用边没进图**（cg 自己在答案 ⑤ 写明："`callees a_receipted_entry_posts`（返回 0——工具漏了 `Entry::new(...).postable()` 这条链式调用边，所以我回源码读测试覆盖）"）。
- **"没被覆盖的那一支"来自编译器**：非桥 `scratch-h4`（一个只依赖该树的独立 crate）跑出逐字
  `warning: variant \`Refuse\` is never constructed` / ` --> /home/nich/…/src/model/entry.rs:9:5` / `  = note: \`#[warn(dead_code)]\` (part of \`#[warn(unused)]\`) on by default`，并给出运行值表
  `zero+receipt  postable = true`（契约要求 false）、`zero-no-receipt postable = false`、`five+receipt postable = true`、`five-no-receipt postable = false`、`signed(-5) = 5`、`normalized(" A ") = "a"`（`logs/h4-one-file-many-algorithms.txt` 第 156-176 行）。cg 的推理逐字："The compiler itself warns: `variant 'Refuse' is never constructed` at entry.rs:9 — that's the \"one branch uncovered/unreachable\" confirmed by rustc ✓✓."（`brief/cg/h4-one-file-many-algorithms.md ①1`）。

**两侧差异与归因**：

1. **信息差 #1（分支级事实的来源）**：我们 census 的 `branch-level` 栏是**一次调用给出的结构性判定**，还带"为什么按构造不可达"（私有枚举 ⇒ 树外也拼不出）；codegraph 没有分支/不可达概念，`callers`/`callees` 一次只答一个符号（本题 3 次图查询只覆盖 `zero_arm`/`postable` 两个名字），cg 只能用 **rustc 的 dead-code 警告**当替身——而那条警告需要**另建一个调用方 crate 去编译**（非桥 `scratch-h4`，答案 ④ 的 counter-proof 就是它）。
2. **信息差 #2（测试是否覆盖该分支）**：这是数据流问题（`amount == 0` 从未被构造），图完全不表达；cg 的 `callees a_receipted_entry_posts` = 0 正好暴露图的这条缺口，它只能回读 `tests/postable.rs` 的人肉判断。
3. **流程差（我们侧）**：本题的"红"在 `check --census` 的分支栏（spec 表格 H4 行逐字："默认面**绿**；红在 `check --face default --census` 的分支栏点名 `ZeroArm::Refuse`"），而**理想路径行只写 `digest --file` → `read`**。我们臂的两次调用正好是 `check --census` + `digest`（顺序与理想行相反），也就是说：理想行少了"红在哪"的那一次，而实际拿到答案靠的正是漏掉的那一次。
4. **`census: true` 是隐式必需参数**：`toolchain/src/mcp/check.rs:310` 逐字 `const CENSUS_SAMPLE: usize = 5;`，抽样规则（同文件 325-372 的注释与实现）＝**每栏头 + 前 5 行 + 每栏的 `not covered` 行**；h4 那句 branch 明细行是 `branch-level:` 栏下的数据行、在表中位置靠后、不是栏头，默认会被 `… withheld …` 顶掉。我们臂这次显式带了它（`logs/h4-one-file-many-algorithms.jsonl` 第 1 条的 `request` 数组逐字含 `"--face","default","--census","true"`），才拿到那句。
5. **记账口径**：cg 的 9 次里有 4 次整文件 `node`（`lib.rs`/`model.rs`/`entry.rs`/`postable.rs`）——其中 `src/lib.rs`（5 行）与 `src/model/model.rs`（3 行）只是模块挂载，属定向开销；我们有等价的 shell 读（step 148 的 `grep -n ''`），不进日志。

**代价**（README §三）：我们 2/4/3,873/10,554/1,175,936 · codegraph 9/2/3,695/8,429/339,328。
**仪器调用 2 vs 9**；输出 3,873 vs 3,695（几乎持平）、推理 10,554 vs 8,429（我们略高）——本题 cg 侧的花费主要在**调用次数与建索引**（`Indexed 4 files · 16 nodes, 19 edges`），不在 token 上；上下文那栏同样不可比（状态机归属）。

**质量**（README §四）：我们 4/4 hardbug 命中 ✓，`h4` 无误；cg 24/26 完全命中，`h4` 不在两处"部分"（`s3`/`g1`）之列 ✓。另注出题台自己写明：`h4` 在本轮**不具鉴别力**（spec:161-172 逐字："**本轮的处理**：frozen 树不动…`h4` 在本轮按**两个读法并列**报分，并标为'该题在本轮不具鉴别力'"）——两侧同受影响，不是任一臂的问题。

**引导含义**：

1. **把 H4 的理想路径行改对**：它是 `digest --file <path>` + `read {path, line}`，但真值的一半（哪一支没被覆盖）只有 `check --census` 的分支栏给；`digest` 自己就把它推出去了（①2 逐字 `not covered here: which branches are dead and which are covered (ask \`check {face}\` for the whole-tree census)`）。建议写成 `check --face default --census true` → `digest --file <path>`（+ `read`），与 spec 表格的"红在哪"一致。
2. **让 branch 栏的明细行默认可见**（或让 `digest` 的函数行尾带上它）：现状是 `CENSUS_SAMPLE = 5` 的抽样会把本题唯一那句判据换成 `… withheld …`，而"这一栏的句子与计数本来就该一起读"。依据：`check.rs:310`/`325-372` 逐字注释（"It is **sampled** by default and whole on request…"、"A flat take of the first five lines hid a whole column once: on the `g4` tree…"）+ 我们臂必须显式点名 `--census true` 才拿到那句。
3. **`digest` 的 `0 call(s) out, 1 caller(s)` 一行可以再进一步**：`zero_arm` 在本树里是"恒返回一个常量、另一臂无构造点"，这正是 census 已经算过的判据；把它并进 `digest` 的函数行（例如 `zero_arm — always Post; the Refuse arm has no construction in this tree`），H4 就真的能"`digest` 一次看清结构"。依据：①2 的 5 行函数表（无分支列）＋①1 的 branch 判据句。
4. **`check` 在"面绿"时也应把 branch 明细带出来**：本题 `verdict passed`，而缺陷就在同一份返回的后半段——若读者的默认动作是"绿了就走"，这条红只在 census 抽样窗口里被看见一次（本题靠显式 `census: true`）。

---

## 附：四题共同指向的五处工具改动（都可由上文某一条逐字依据复现）

1. **`consistency --parent` 的 `outlier` 行补上离群对象的文件**（`h1` 引导 1）——本题三个后续事实（`toggle.rs:16`、契约行 `:14`、两族算术）里有两处最后是 shell 读出来的。
2. **`callgraph` 的 `limit` 不要把"族里不一致的那些定义"截掉**（`h1` 引导 2；逐字 `4 of 9 definitions withheld … toggle.rs`）。
3. **`adopted`/`conformance` 的 `next` 行只指向"本条还没判过"的问题，并把 `lapsed` 的两枚指纹印出来**（`h2` 引导 1/2；`conformance` 无 `--anchor` 的"每个锚点一行"能力已在源码里，next 文案没跟上）。
4. **`why` 的 `wiring` 行给行号；`scope`/`pruning` 的"run `xirang check`"文案与 `check` 实际产出对齐**（`h3` 引导 1/2）。
5. **`check --census` 的分支明细行默认保留（或写进流程表/理想路径行）**（`h4` 引导 1/2；`CENSUS_SAMPLE = 5` 的抽样会把唯一那句判据顶掉）。

四题的差距形态并不一样，落到归因上是三类：
`h1`/`h4` 是**同一条信息换了个载体**（族比较 / 分支级判据：我们一次返回，cg 要 N 次图查询＋读文件，`h4` 还要借 rustc）；
`h2` 是**判定 vs 文本**（`lapsed` 是算出来的，不是台账里的一行字）；
`h3` 是**图里根本没有这个对象**（宏生成的计划与 `NODE_ID`），因此 cg 侧不是"多走几步"而是"换一条路"——13 次里 2 次重复、2 次软失败，最终的机制句来自宿主源码里的注释。

## 没能判定 / 口径存疑

1. **`h2` 的"违禁 fix"在记分版与交付版之间不一致**（已在上文 `h2` 的"质量"里点明）。README §四的判定按记分版；我核对的两份当前交付文件（`target/round9/answers/h2-claim-unkept.md`、`target/probe-cg26/answers/h2-claim-unkept.md`）都已改成"需要人、不是改代码"。**我没有能力从现有材料判定记分发生在哪一版**——`target/probe-cg26/CORRECTIONS.md` §1 只说"原 … 或删掉该失效条目"，未给记分快照的时间戳。这一条不影响 README 的判定（它明说"不改判定"），但读者若只看交付文件会以为那句违禁不存在。
2. **`h2` 理想路径的两步是否覆盖全部真值，我判不了**。规格写"`conformance --anchor <标本>`（判在不在、在哪失效）+ `consistency --specimen <标本>` … 两次调用内给全"（spec:212-213），但本轮实测：`conformance --anchor root/control/button` 的返回**没有提到 panel 的失效**（`brief/ours/h2-claim-unkept.md ①2` 全文），失效那半只在 `adopted`（①1）或 `conformance --anchor root/control/panel`（①3）里。若严格按标注的 `<specimen>` 执行这两次调用，答案是**拿不到**"哪条采信在哪个文件失效"的。我倾向认为 plan 那行是 `adopted`（或 `conformance` 无 anchor 视图）的简写——因为出题台自证用的两条命令正是 `consistency --specimen <specimen>` 与 `conformance --anchor <lapsed anchor>`（`tools/xirang-mcp-hardbug` 的 `prove()` 里逐字 `truth["lapsed"]["anchor"]`）——但这是**推断**，标在这里。
3. **`h4` 那句 branch 判据在默认 `check` 下会不会被抽掉，我是按实现推的**：`check.rs:310` 的 `CENSUS_SAMPLE = 5` 与 325-372 的抽样规则（保留栏头 + 前 5 行 + 每栏 `not covered` 行）判定它会被 `… withheld …` 替代；我**没有**对 h4 树重跑默认 `check` 去实证（那会在题树里写 `check-default.log`，违反"只读"）。结论方向我有把握（该行是数据行、不是栏头、也不在前 5 行内），但"恰好被抽掉"这一步是推断。
4. **`h3` 的 `scope not-selected` 在当时的树上是否可能出现，我判不了**。规格说它实测到了（spec:36-39），而我们 ①3 拿到的是 `scope unknown (no source_scope.tsv)`；本树 `target/xirang/out/` 的 `source_scope.tsv` 时间戳是 22:47，而本题日志（三条调用）的 mtime 是 21:35:50 ⇒ 我们那次 `check` 不晚于 21:35:50，读不到 22:47 才出现的证据。中间是谁在 22:46-22:47 把构建产物补出来的（复核者？维护者？），我没有证据，只按"答题当时读不到"来写。
5. **"白跑"（`repeat`）我只对 `h3` 的 cg 侧给了结论**（`files` 与 `node host/src/lib.rs` 各重复一次，逐字同参），因为这三条可从日志单独判定；其余三题两侧我逐条比过，没有同工具同参数的重复。但 README §三表里的"仪器调用数"不含 `init` 的分类，cg 的 10/7/13/9 都**含**各自 1 次 `init`（cg 自报的"含 1 次 init ⇒ N−1 步"与此一致）——引用时不要跨口径混算。
