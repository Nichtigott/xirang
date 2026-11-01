# 第九轮 · 关卡题 `g1`–`g4` 逐题对话层分析

范围：`g1`（≡`s3` 树）· `g2`（≡`s5` 树）· `g3` · `g4`。口径与判定全部引 README §三/§四，未重做判定、未重算代价。

**引用体例**：`brief/ours/g4.md ①1` = 该题精简版里第 1 次仪器调用的段落；`full/...`、`_shared.md`、日志文件路径按需给全。
**调用数一律以逐题日志条数核对**（`target/round9/logs/<id>.jsonl`、`target/probe-cg26/logs/<id>.txt`），不采信答案末行自报。

**先记一条装置口径（影响 g1/g2 的"推理"栏）**：我们臂的 `g1` 三条与 `g2` 三条调用各自**只在一条 bash 步里**发出（`_shared.md` step 44 / step 45），
且每条 shell 都对自己的 stdout 做了切片（`sed -n '1,40p'`、`sed -n '5,14p'`、`sed -n '1,20p'`、`tail -22`）。
于是：表里 `g1`/`g2` 的「步 0 · 推理 0 字符」是**归属假象**——规划那两题答案的推理被记到了相邻步/共享桶
（`brief/ours/g2.md ①1` 挂的"当时推理"其实是 g1 的答案规划；`brief/ours/g1.md ①1` 挂的其实是 g4 的），
而**日志里记的是未切片的完整返回**，agent 当时看到的只是切片窗口。下文凡"拿到了什么"都按切片窗口判定，并标出窗口外但进了日志的行。

---

### `g1` · （只读 s3 的树）这个仓库里还有哪些地方是没有任何测试能到达的——给可核对的具体函数 文件:行，并说出你看不见什么

**我们臂的路径**：3 次仪器调用（日志 3 条，全 exit 0）＝ `check {root:trees/s3, face:default, census:true}` → `callgraph {root:trees/s3, orphans:true}`（shell 只回第 5–14 行） → `search {root:trees/s3, literal:"audit_unused"}`，三步同一条 bash。

- ①1 `check --census true` 一次给全三样：机械事实（7 条结果行里 `1 passed`/`3 passed` 之外全是 `0 passed`，即 audit 面那个二进制**没跑**）、函数级可达性（`test-reachable: 1 of 12 production function(s) no test can reach (18 function(s) indexed in this tree; a static walk from the test files along the same name-in-call-list rule the orphan view uses)`、`by directory: crates/core 1 of 7`、明细 `fn     no test reaches audit_unused (crates/core/src/audit.rs:14)`）、以及**必须降级使用的文本层计数**（`declarations: 4 production pub fn name(s) appear in no test file (a text-level count: a test that reaches one without writing its name does not count here)`；`unreferenced SMALL_LIMIT (crates/core/src/limits.rs:10) is not read anywhere outside tests`）。
- ①2 `callgraph --orphans`：`orphans 1 (defined here, no static caller in this tree)` → `fn    audit_unused -> crates/core/src/audit.rs:14`；同一段还给 F1 口径要的那半边：`note   5 function(s) in test files have no static caller either; the test harness calls them, so they are counted here rather than listed above`。
- ①3 `search --literal audit_unused`：`literal "audit_unused" (raw bytes, case-sensitive; comments and string literals included)` → 唯一命中 `src/audit.rs:14: pub fn audit_unused(entries: &[Entry]) -> usize {`、`member ledger-core (1 lines)`。
- **决定性的一步**：①1。`test-reachable: 1 of 12` + `fn no test reaches audit_unused (…)` 同一条结论两种写法，①2 的 `orphans 1` 再钉一次同址；答案②正是这么写的：判据三件套 `orphans 1` / "全树**只有定义那一行**命中，没有任何调用点" / census 的 `fn` 行。
- **答案是否用了 census 栏**：用了，而且是**分档用**——`test-reachable`/`by directory`/`fn`/`unreferenced` 直接进答案；`declarations: 4` 被显式降级（答案③："是**文本层**计数：后三者由调用链间接到达（`Store::post` → `postable`；`bucket_lines` → `bucket_name`；`render` → `write_count`）"）；`SMALL_LIMIT` 被判成"「生产代码从不读」而不是「测试到不了」，两件事不能混"。这一分档的推理原文在共享桶：`Non-function places: SMALL_LIMIT … Hmm, is that "没有任何测试能到达"? No: tests do reach it. So I should list it as a related-but-different finding … clearly labeled as not a "test-unreachable" item.`（`_shared.md` step 45 段内）。
- 答案④的盲区清单是 census 的 `not covered by the test-reachability column:` 那段的压缩复述（按名匹配会误判可达、以 `tests/`/`_tests.rs`/`#[test]` 播种、`main` 从不列出、"静态走查，不是覆盖率测量"）。
- 一个**归属限制**：答案②里 `#![cfg(feature = "audit")]`（`tests/audit.rs:10`）与 `lib.rs:9` 这两条行号，g1 自己的 3 条调用**没有**提供（census 不打印它）；同内容的 s1 树读过（`full/ours/s1.md` 里 `crates/core/src/lib.rs:8: #[cfg(feature = "audit")]` / `:9: pub mod audit;`），s3 那一题也写了同样的行号（`brief/ours/s3.md` 答案②）。即：g1 的答案与 cg 一样有一半来自**同会话的同内容树**。

**codegraph 的路径**：8 次仪器调用（`logs/g1.txt` 8 条 `=== CMD`）＝ `init` → `files` → `init`（第一次副本多套一层 `s3/`，`files` 路径前缀暴露后重建；推理原文 `g1 currently = trees/g1/s3/{...}. That's an extra nesting level…`，`brief/cg/g1.md ①2`） → `callers audit_unused` → `callers audit_count` → `callees the_audit_counts_the_non_zero_entries` → `explore audit_unused unreachable functions test coverage` → `node crates/core/tests/audit.rs`。

- ①4 `Callers of "audit_unused" (0):` + `ℹ No callers found for "audit_unused"`；①5 `Callers of "audit_count" (1):` → `function    audit_unused`。
- ①6 `Callees of "the_audit_counts_the_non_zero_entries" (1):` → 只有 `method      new / crates/core/src/model.rs:21`——测试里那句限定路径 `ledger_core::audit::audit_count(&entries)` **没成边**。
- ①7 `explore` 的 blast radius 于是写了 `audit_count (crates/core/src/audit.rs:8) — 1 caller in crates/core/src/audit.rs; no tests found within 3 caller hops`（**假警报**），同一次返回里附了 `crates/core/src/audit.rs` 的逐字源码（答案末行自报的 `node crates/core/src/audit.rs` 其实来自**同树的 s3 日志**，本题没发过这次调用——`logs/s3.txt` 有 `=== CMD: … codegraph node crates/core/src/audit.rs`）。
- ①8 `node crates/core/tests/audit.rs` → `10	#![cfg(feature = "audit")]` … `25	        ledger_core::audit::audit_count(&entries),`——回源码把 ①7 推翻。答案④原文："工具漏边——`ledger_core::audit::audit_count` 这条限定路径调用**没有**变成 caller 边…所以 explore 的 "no tests found within 3 caller hops" 在 `audit_count` 上是假警报；我回源码核到 `tests/audit.rs:25` 才没把它报成死代码。"
- **它为什么要"回读源码"**：本题日志里真正只有 **1 次 `node`**（上面那次反证）。原因在图的两处缺口：(a) 限定路径调用丢边（`callees` 只回 `Entry::new`）；(b) **分支级问题图完全不表达**——答案③的三条分支（`buckets.rs:8-9` 负金额支、`store.rs:22-24` 拒绝支、`query.rs:21-25` account 过滤支）没有任何一条能由 callers/callees 读出，答案自己写"分支级的判断同样来自读源码（`node`）而不是图——图不表达"哪个分支被执行过""；这些 `node` 读在同会话的 s1 日志里（s1≡s3，`logs/s1.txt` 有 `node crates/core/src/store.rs`、`node crates/report/src/buckets.rs`、`node crates/report/src/query.rs`），答案把它们一并算进了本题的"调用清单"（自报 `callers` × 全树 12 个符号也是 s3 的 12 次，本题日志只有 2 次 `callers`）——按纪律 3 以 8 条为准。

**两侧差异与归因**：差距不在"谁更会读源码"（两边都吃了同内容树的同会话红利），而在**工具把哪些判断变成一次返回**：
(1) 我们的 `check --census` 是**一次调用给一整张表**（`test-reachable` 计数 + `by directory` + 逐条 `fn` 明细 + `decl` 文本层计数 + 盲区声明），cg 的 `callers` 一次只答**一个符号**（日志逐条 `callers Entry::new`、`callers Store::new`…），覆盖 12 个符号在 g1 本题要 12 次、在 s3 就是 12 次；
(2) 同样的"名字在调用表里出现即算可达"规则，我们的走查认得测试文件里那句限定路径 `ledger_core::audit::audit_count(&entries)`（所以只报 1/12 不可达），cg 的边解析不认（`callees` 丢边 → `explore` 出"3 跳内没有测试"假警报）——一侧因此**多花 1 次 `node` 反证**；
(3) cg 的仪器要先有可写树与索引：本题 8 次里有 2 次 `init`，其中 1 次纯粹是副本嵌套的返工；我们的 3 次都是零副作用的只读调用。
(4) 分支级那一半（答案③）在 cg 侧**没有任何工具表达**，只能读源码；我们的 census 有 `branch-level` 列（本题恰好是 `0 constructively unreachable arm(s)`）。

**代价**（README §三）：`g1` 我们 3/0/0/0/0 · codegraph 8/1/2,302/7,618/56,832。

**质量**（README §四）：我们 = 26/26 命中，`g1` 属"只读情景 `s1`–`s8`+`g1`+`g2`+`g4` 10/10"✓；codegraph = `s3`/`g1` 两题判"部分"✗，两题都答对死函数 `audit_unused`，但"都没声明"5 个 `#[test]` 函数由框架运行期调用、不计入"（`F1` 口径要求的那半边）"，g1 答案里那条是复核者的补注（`brief/cg/g1.md` 答案末"补注（复核者口径）"），不是臂自己写的。

**引导含义**：
1. `check` 的 census 应把 `callgraph --orphans` 里那句 `note   5 function(s) in test files have no static caller either; the test harness calls them, so they are counted here rather than listed above` **搬进 `test-reachable` 行**（或紧随其后加一条 `harness-calls 5 …`）。依据：本桥两份返回各自只有一半——census 有 `test-reachable: 1 of 12 …`，F1 要的那半边只在 `callgraph` 的 note 里（`brief/ours/g1.md ①2`），而本题答案④只复述了 census 的"播种"规则、没复述那半句；cg 的 s3/g1 正是因为缺这半句被判"部分"。
2. `declarations: 4 production pub fn name(s) appear in no test file (a text-level count…)` 这一行**会误导**（4 条里 3 条是误报）。同一工具已经有"名字在调用表里"的走查，应当在每个 `decl` 行尾直接给裁决，例如 `decl no test names postable (crates/core/src/model.rs:37) — reachable via Store::post`。依据：我们臂在 g1（答案③）与 s3（答案③）**两处都手工降级了这一行**，推理原文 `Hmm, is that "没有任何测试能到达"? No: tests do reach it.`（`_shared.md`）。
3. `search --literal` 那条"全树只命中定义行"之所以值一次调用，是因为 census 的 `decl` 行不可信；若第 2 条落地，本题可降到 2 次调用（省掉 ①3）。

---

### `g2` · （只读 s5 的树）用一段伪代码说明这段渲染逻辑想做什么，再指出实现与意图的差

**我们臂的路径**：3 次仪器调用（日志 3 条，全 exit 0）＝ `check {root:trees/s5, face:default}`（stdout 切到第 20 行） → `read {path:crates/report/src/render.rs, whole}`（`tail -22`） → `callgraph {root:trees/s5, function:render}`（`sed -n '5,26p'`）。

- ①1 `check`：`verdict  failed (cargo exit 101)`，三条 `failed …`（`the_report_names_its_count_and_its_entries` / `the_report_prints_its_count_before_the_entries` / `the_minimum_amount_filter_keeps_the_entries_at_or_above_it`），外加 7 条结果行里只有 `1 passed` 非零。**切片窗口到第一条 `why` 为止**（`why    the_report_names_its_count_and_its_entries: … panicked at crates/report/tests/report.rs:48:5: the count block is written: LEDGER REPORT`）；日志里还有第 2、3 条 `why`（`:38:41`、`:27:5`）与一条 `next` 行，但那两行在窗口外。
- ①2 `read --whole`：`symbols 7: `render` lines 7-13 — Render the entries: a header, the count, then one line per entry. / 17: `write_count` lines 17-19 — The count block.` + 1–19 行全文——**意图在这里**（`:5-6` 的契约注释），且文件里同时给出 `write_count` 的定义（`:17-19`，体内 `:18` 是 `entries: {}\n`）。
- ①3 `callgraph {function:render}`：`contract /// Render the entries: a header, the count, then one line per entry.`（工具把文档契约单列成 `contract` 行）、`callees: from, push_str`（**只列调用**，不含 `write_count`）、`also here: write_count -> crates/report/src/render.rs:17`、`tests: crates/report/tests/report.rs`。
- **决定性的一步**：①3。答案③的判据原文："`callgraph {function:render}` 的 `callees: from, push_str`（**不含** `write_count`），`also here: write_count -> render.rs:17`（定义在，可达性不在）"——**缺件**由"callees 是调用清单"这一条语义直接得出；意图由 ①2 的契约行 + ①3 的 `contract` 行双重给定。
- 答案③后半"同树另有 1 条与渲染无关的红（… `report.rs:27`，由 `query.rs:27` 的 `entry.amount > min` 反转引起）；两条红独立…互不掩饰"——`report.rs:27`/`query.rs:27` 不在本题的切片窗口内，它与同树 s5 那一题的答案几乎逐字相同（`brief/ours/s5.md` 答案③有同一句），属同会话同内容树的复用；日志里那条 `next   … If two red things may be independent, two green runs are not the evidence: fix one and re-run, and say which red survived.`（`brief/ours/g2.md ①1` 末）正好是这句的模板，但它在 20 行窗口之外，装置上无法证明 agent 当时看到了它。

**codegraph 的路径**：8 次仪器调用（`logs/g2.txt` 8 条 `=== CMD`，**无任何 nonbridge**，即本题**没有跑过一次测试**）＝ `init` → `files` → `init` → `node crates/report/src/render.rs` → `callers write_count` → `callees render` → `node crates/report/tests/report.rs` → `node crates/report/src/query.rs`。

- ①4 `node crates/report/src/render.rs`：20 行逐字源码，含 `5	/// Render the entries: a header, the count, then one line per entry.` 与 `17	pub fn write_count(text: &mut String, entries: &[Entry]) {`、`18	    text.push_str(&format!("entries: {}\n", entries.len()));`。
- ①5 `Callers of "write_count" (0):` + `ℹ No callers found for "write_count"`；①6 `Callees of "render" (1):` → 只有 `struct      Entry [references]`（**不是调用，是类型引用**）。
- ①7 `node crates/report/tests/report.rs`：拿到两条会红的断言原文——`:38	let count = text.find("entries: 3").expect("the count line is printed");`、`:27	assert_eq!(kept, 2, "the zero floor keeps the two non-negative entries");`；①8 `node crates/report/src/query.rs` 拿到 `:27	            if entry.amount > min {`。
- **决定性的一步**：①4 + ①5。意图来自源码里的契约注释（不是工具字段），缺件来自"定义在、callers=0"；判断"测试会红"则是**读出来的预测**：答案④"若它其实被调用，`callees` 会带它、输出里会有 `entries: N`、`:48` 的 `contains("entries: 3")` 不会红——三处都相反"。

**两侧差异与归因**：
(1) **意图的来源不同**：我们把文档契约变成了工具返回里的 `contract` 行（`brief/ours/g2.md ①3`），cg 必须自己读源码注释（`brief/cg/g2.md ①4`）——同一条信息，一个在结构化字段里、一个在全文里。
(2) **"缺件"的判据强度不同**：`callees: from, push_str` 是**调用清单**，缺席即可作反证；cg 的 `Callees of "render" (1):` 只回 `struct Entry [references]`——它列的是引用不是调用，**缺席不能作反证**，所以 cg 又读了 `render.rs` 全文才敢下结论（`brief/cg/g2.md ①5/①6` 与答案③）。
(3) **红的来源不同**：我们跑了 `check`（`cargo test`，`verdict failed (cargo exit 101)` + 三条 `failed` + 至少一条 `why` 原文）；cg **一次测试都没跑**（日志无 nonbridge），它把 3 条红**预测**出来并回头读断言源码核对（①7/①8）——两侧最终结论一致，但一侧是"读到的红"，一侧是"推出来的红"。

**代价**（README §三）：`g2` 我们 3/0/0/0/0 · codegraph 8/1/836/2,487/126,464。

**质量**（README §四）：我们 = 26/26 命中（`g2` 在只读情景 10/10 内）✓；codegraph = 24/26 完全命中，`g2` 未被列入任何瑕疵项 ✓（它被点名的问题在 `s3`/`g1`、`h2`、`fc`、`h3`、`s4`）。

**引导含义**：
1. `callgraph {function}` 的 `also here: write_count -> crates/report/src/render.rs:17` 已经点出"同文件里这个函数没在被调用的清单里"，但**不下裁决**。把它改成带结论的行（`also here: write_count -> crates/report/src/render.rs:17 (0 callers in this tree)`）就等于把本题答案的②③一步给出。依据：答案③引的正是 `callees: from, push_str`（不含它）+ `also here` 两条**推断**；cg 侧对照是 `Callers of "write_count" (0)`（`brief/cg/g2.md ①5`）——同一事实，我们这边要多一次推理。
2. `read --whole` 的 `symbols … — Render the entries: a header, the count, then one line per entry.` 已经把契约与符号并排给出，方向是对的；`callgraph` 再单列 `contract` 行属于重复。可实施的是：`read` 的符号行只留定位，**契约原文只在 `callgraph`/`explain` 里出现一次**，避免同一意图在同一题内出现三份（`read` 符号行、`read` 正文、`callgraph` 的 `contract` 行）。
3. 针对 (3)：`check` 的 `why`/`next` 行是"两条红是否独立"的唯一权威话术，但它在第 21–22 行、被 agent 自己的 `sed -n '1,20p'` 切掉。可实施：把这类**流程指导**放到 `verdict` 之后（前 10 行内），或单独一行 `coach  …`；不要让它排在 3 条 `failed` + 3 条 `why` 之后。依据：`_shared.md` step 45 的命令原文（`| sed -n '1,20p'`）与 `brief/ours/g2.md ①1` 里 `next` 行的位置。

---

### `g3` · 给这个对象加一层内部结构，但不要动它的注册树和公开路径；说明凭什么相信没动，门必须绿

**我们臂的路径**：9 次仪器调用（日志 9 条，全 exit 0）＝ `registry` → `explain` → `registry`（改前，存 `/tmp/g3.reg.before`） → `usages {node:root/control/button}`（改前，存 before） → `apply {deepen, inside:{parts:{label:String}}}`（**preview**，未写盘） → `apply {…, apply:true}` → `registry`（改后） → `usages`（改后） → `check {face:default, timeout_ms:900000}`；另有非仪器 bash：`find … | xargs sha256sum` 前后快照 + `diff`（`full/ours/g3.md` 二、）。

- ①1/①3/①7 `registry` 三次返回**逐字节相同**：`faces 3` + 三行（`root/control Control control/control.rs fb97ddd5…`、`root/control/button Button control/object/button/button.rs ff1c57d9…`、`root/control/slider Slider … bdb4427c…`）；注意首行自陈没有基线：`tree derived now (no published records at …/target/xirang/out; cannot read …/source_scope.tsv: No such file or directory (os error 2))`。
- ①4/①8 `usages {node:root/control/button}` 前后也逐字节相同（`node ff1c57d9…`、`parent …root/control`、`children (0)`）。
- ①5 preview 是本题信息量最大的一次返回：`action preview` / `would write …` / **`preview effect: deepened root/control/button: `Button` now holds `ButtonParts` with 1 part(s) ("label"); its declaration, its public path, its tree row and the factory-shape pins were not touched — `check {face}` is what shows those pins still hold`** / `consequences (static, text-level): 113 in-tree test line(s) name this face`（随即 `… truncated: 108 of 113 pin lines withheld at the limit of 5; grep the face's name in test files`）/ `alternative (the other reading of `make it deeper`): hang another face **under** this one …`（把"再加一个面"这条路逐项定价）/ `note   this face stops being a unit struct …`，并给出完整 diff。
- ①9 `check`：`verdict  passed (cargo exit 0)`、`elapsed 19409 ms`、`25 passed`（0 failed）。
- **决定性的一步**：①5 的 `preview effect` 一句（工具自己声明"声明/公开路径/树行/工厂形状钉子都没被碰"）定了做法与证法；随后 ①1↔①7、①4↔①8 的前后对账把它变成证据。这个手工对账协议是 agent 自己在 ①2 重推理里定的原文："1. Save a pre-copy of the button file and the full file list + hashes (to prove only one file changed). 2. `apply` preview (no apply) … 4. `registry` + `explain` after → compare rows with before. 5. `check {face:default}` → green. 6. Diff of the button file."

**codegraph 的路径**：5 次仪器调用（`logs/g3.txt` 5 条 `=== CMD`）＝ `init` → `files` → `init` → `files`（改动后，`button.rs (rust, 9 symbols)` → `(rust, 5 symbols)`） → `node src/control/object/button/button.rs`（改动后 68 行全文）——**codegraph 对"证明没动"几乎没有贡献**；真正的证据全在 18 次 nonbridge（`logs/g3.txt` 逐条 `=== CMD(nonbridge)` 的 `cargo` 调用）：

- `cargo run --example tree` 前后逐字节相同（`sha 571b70bc642f6aa1`，"三行路径/`kind`/`source` 一字不差"）；
- `cargo run --example health_check` 前后逐字节相同（17 行，含声明点 `[control/object/button/button.rs:15:1]`——"我把新层放在声明**之后**、导入保持单行，所以连声明所在行号都没动（先放在前面时这里是 `:42:1`，我实测过并因此改了排布）"）；
- `cargo run --example graft_record` PID 归一化后逐字节相同，含两个身份哈希 `ff1c57d9b110ef57eac134a3cb4e46b6`（slot）与 `15dc596acbee32774e92a9afe0f52e54`（typed declaration）；
- `cargo test --offline` 与 `--all-features` 均 `rc=0`，6 个 target 全 ok、0 failed；
- 依据是读工具链源码得出的（`kernel/src/registry_core/declaration/contract.rs:26-33`、`toolchain/src/runtime/src/macros/face_objects.rs:38-39`、`assert_contract`），落点判据是 `tests/registry.rs:72` 原本钉着 `REGISTRATION.parts == "NoParts"`——**它改了这个测试的期望**（答案④："`tests/registry.rs:64-91`（期望更新：…）"），并踩过一次编译坑（`Output = ButtonParts` → `expected ButtonParts, found ()`）。
- ①3 的重推理原文："Now g3 is perfect: - Gate green (default: 6 ok, 0 failed) ✓ - All three observable artifacts byte-identical before/after ✓ (tree, health_check literally; graft_record modulo the temp-dir PID) — including the NodeId hashes."

**两侧差异与归因**（这题是**写题**，差异全在"怎么证明没动"）：
(1) **我们桥没有一次"专门证明没动"的调用**。有的相邻工具是 `consistency`（工具目录原文：`whether the siblings follow the same shape (that is `consistency --specimen <anchor>`)`，`toolchain/src/mcp/tools.rs`）与 `diff`（`tree_delta.rs` 头：`xirang.diff` states the whole delta … a face the build published is `ok`, a face it never saw is `added since build`）——前者比的是**兄弟形状**（本题恰好要把 button 变得与 slider 不同，方向相反），后者比的是**相对构建清单**（本题 `registry` 三次都报 `no published records …`，没有基线）。所以 agent 只能自己造协议：before/after 各存一份 `registry`/`usages` + `diff` 无输出 + `sha256sum` 全树快照 + `check` 绿。`registry --full` 也不是这个用途（它只买虚拟根下逐成员的树）。
(2) **cg 的证法不依赖任何图工具**，而是**跑这棵树自己的产物**（`tree`/`health_check`/`graft_record` 三个 example + 两套 test 面）：证明来自"同一命令前后逐字节相同"，且它连**声明所在行列号**都保持不变（为此把新层挪到声明之后）。它的 5 次 codegraph 调用只用于"文件与符号数变了没有"（`files` 前后）+ 看一眼改后的源码。
(3) **一处实打实的流程差**：我们的 `apply` 直接产出可编译的层（`check` 一次绿，`25 passed`），cg 手写宏字段并因 `type Output` 与嫁接侧不一致**编译失败过一次**、再改回 `()`；一侧还有 preview 把"另一种读法"（在其下再挂一个面）的代价逐项列出，另一侧是 agent 自己判断"这条路会让 `built_in_tree_…` 红"。

**代价**（README §三）：`g3` 我们 9/7/6,183/10,892/1,047,296 · codegraph 5/6/6,823/9,384/1,845,376；**但 cg 的仪器外还有 `nonbridge: 18 次 / 30,744 B（基线/改动后各三次 example 运行 + 四次 test；全在 logs/g3.txt）`**（`brief/cg/g3.md` 答案末行），真正的验证工作在那个桶里。

**质量**（README §四）：我们 = 26/26 命中，`h1`/`g3`/`fa`/`fb`/`fc` 5/5 ✓；codegraph = 24/26 完全命中，`g3` 未被列入瑕疵项 ✓。

**引导含义**：
1. **把"没动"做成一次调用**：`apply` 在给出 `preview effect: … its declaration, its public path, its tree row and the factory-shape pins were not touched — `check {face}` is what shows those pins still hold`（`brief/ours/g3.md ①5`）的同时，应当吐出**可执行的 invariance 清单**：要比对的 `registry` 行/身份哈希、要重跑的 pin 目标（本树是 `tests/registry.rs:99-100` 的 type_name 断言与两个形状钉子）、以及一句现成调用 `registry {expect:"<before rows>"}` 或 `verify {…}`。现在这条对账由 agent 用 `cp`+`sha256sum`+`diff` 手搓（`full/ours/g3.md` 二、的命令原文），成本与可漏性都在 agent 一侧。
2. **`apply` 的 `consequences` 截断挡住的正是要保全的钉子**：`… truncated: 108 of 113 pin lines withheld at the limit of 5; grep the face's name in test files`（`brief/ours/g3.md ①5`）。可实施：截断时给"按测试文件分组的 pin 计数 + 每组一行示例"，或直接把 `grep -n '<face>' tests/ examples/` 这条填好面名的命令写进 `next`；cg 是直接读 `tests/registry.rs` 的期望并改它的（答案④）。
3. **`diff` 在评测树上是死的，值得修**：`registry` 三次都报 `no published records at …/target/xirang/out; cannot read …/source_scope.tsv`，而 `check` 是带共享 `CARGO_TARGET_DIR` 跑的——"相对构建清单的 delta"这类工具在题树上永远没有基线。具体：`check` 把 `source_scope.tsv`/`pruning_manifest.tsv` 落到树内（或 `.xirang` 下），或在 `registry`/`check` 的返回里点名它写到了哪里（现在只在 `log` 行里给 `check-default.log`）。
4. 本题也是**流程表**的用武之地：`client.rs` 的流程段落已经写了"对象有兄弟时先问 `consistency --parent`"，但 g3 这类"加深自己、别动树"的场景没有对应条目。可实施：给 `apply` 的目录描述补一句"When the change must not move the tree, the preview's `preview effect` line is the contract; re-read `registry`/`usages` before and after and run `check`"——即把本题 agent 自己发明的 6 步协议升格为工具文案。

---

### `g4` · 这棵树里有哪些臂是没有任何执行能进入的；给出每条判据，并说清哪些你判不了、为什么

**我们臂的路径**：3 次仪器调用（日志 3 条，全 exit 0）＝ `check {root:trees/g4, face:default, census:true}` → `callgraph {root:trees/g4, orphans:true}` → `inspect {root:trees/g4, path:crates/core/src/bands.rs}`；另有非仪器 bash 一次（step 42：`find … | sort` + `grep -n '' crates/core/src/bands.rs crates/core/src/bands_tests.rs crates/core/src/lib.rs`，把三个文件带行号打印）。

- ①1 `check --census true` 的 **branch-level 两行就是答案**（逐字）：
  - `branch-level: 2 constructively unreachable arm(s) in this tree (1 `false` guard(s), 1 never-constructed variant(s); a static read of the source text, not a coverage measurement)`；
  - `` `if false` guards an arm in `band_word` at crates/core/src/bands.rs:36 that no run can enter; the contract above it says 33: The sweep's word for one band, behind a data-dependent condition. / 34: 清扫给某一档的词，前面挡着一个数据相关条件。``；
  - `` no construction of `Band::Frozen` is spelled in this tree, so the arm matching it in `band_word` at crates/core/src/bands.rs:54 can never be entered (the enum is private, so a constructor outside this tree cannot spell the variant either); the contract above it says 33: …``。
  - 同一段还给了两条"判不了"的**通用**理由：`a condition whose value depends on data — a field, a parameter, a comparison, a `match` over a value — is not judged at all`、`a `pub` enum is never judged`。
- ①3 `inspect`：`fn band_word lines 35-56 calls=[]` / `fn default_band lines 60-62 calls=[]` / `fn state_word lines 88-94 calls=[]`（三个函数的行号区间）；①2 `callgraph --orphans`：`orphans 1` + `note   7 function(s) in test files have no static caller either; the test harness calls them`，用于给出该树函数级视图（`audit_unused`）。
- **决定性的一步**：①1。agent 在**调用之前**就用 step 42 的源码 grep 把四种形状排好（推理原文：`① `if false { … }` guard at bands.rs:36-41 — constructively unreachable … ③ `amount > limit` … CANNOT judge statically … ④ `State::Dormant` … the enum is `pub` … **cannot judge**`），并预言 `Now the census column: the check earlier (r3/r4) said "branch-level: 0 constructively unreachable arm(s)". For g4's tree, I expect the tool to report 2 (① and ②). Let me run `check {face:default, census:true}` on g4 and see the branch-level column. That's the核心 evidence.`（`brief/ours/g4.md ①1` 当时推理）。census 两行逐字确认了①与②，并给出②的**判据措辞**（"the enum is private, so a constructor outside this tree cannot spell the variant either"）——答案②即为这两行的转写。
- 答案④（判不了的两条 + 为什么）与⑤（看不见什么）是把 census 的通用散文**按站点重排**：③ "现有测试确实一次没走（`bands_tests.rs:14` 只传 `default_band(), 1, 1000`，`1 > 1000` 为假），但「本套件没走」≠「没有任何执行能进入」"；④ "`pub enum State`（:68-78）是**公开表面**——外包使用者可以构造它 ⇒ 把它列为不可达等于对「该 crate 的每一个使用方」下结论"。

**codegraph 的路径**：13 次仪器调用（`logs/g4.txt` 13 条 `=== CMD`）＝ `init` → `files` → `files`（重复） → `node Cargo.toml`（**失败**：`Symbol "Cargo.toml" not found in the codebase`） → `node crates/core/src/lib.rs` → `node crates/core/src/bands.rs` → `node crates/core/src/bands_tests.rs` → `node crates/core/src/limits.rs` → `node crates/core/src/audit.rs` → `callers band_word` → `callees band_word` → `callers state_word` → `callees the_small_band_is_small`；另 1 次 nonbridge（`diff -r -x .codegraph trees/s1 trees/g4`）。

- ①6 `node crates/core/src/bands.rs`：**夹具自述**先到手上——`//! Three shapes live here on purpose, because a branch column is only worth having if it separates them. ① is a `false` guard: constructively unreachable. ② is a `match` arm on a variant no construction in this tree spells, and the enum is private, so no construction outside this tree can spell it either: constructively unreachable. ③ is `amount > limit`: whether it is taken is a runtime fact, so no static read may claim anything about it.`，以及 `:23` 的派生表、`:24-31` 私有 `enum Band`、`:60-62` `fn default_band() -> Band { Band::Small }`、`:68` `pub enum State`、`:83-87` 关于可见性边界的注释。
- ①7 `node crates/core/src/bands_tests.rs`：`:14	    assert_eq!(band_word(default_band(), 1, 1000), "small");`、`:19	    assert_eq!(state_word(State::Open), "open");`——**这就是"回源码核"的对象**。
- ①10/①12/①13 三个假警报：`Callers of "band_word" (0):` + `ℹ No callers found for "band_word"`、`Callers of "state_word" (0):` + `ℹ No callers found for "state_word"`、`Callees of "the_small_band_is_small" (0):` + `ℹ No callees found for "the_small_band_is_small"`。
- **决定性的一步**：①10/①12 + ①7。推理原文（①2 段）：`2. `callers band_word` = 0 and `callers state_word` = 0 — even though bands_tests.rs:14 calls `band_word(default_band(), 1, 1000)`. So codegraph MISSES the `use super::{band_word, default_band, state_word, State};` + call — because the test module is mounted via `#[path]` and the calls are bare names resolved via `use super::`. Similarly `callees the_small_band_is_small` = 0 … That's a big fidelity note for the g4 answer: the tool reports band_word and state_word as having NO callers, which would falsely suggest they're dead code. The truth (source read) is that bands_tests.rs calls them. So for g4 I must not report them as unreachable functions.` 答案⑤把它写进交付："所以我没把这两个函数报成死代码；凡是"0 callers"的结论都回源码核过。"
- **它凭什么回源码核**：①7 的 `#[cfg(test)] #[path = "bands_tests.rs"] mod bands_tests;` 挂在 `bands.rs:96-98`（①6 的 68–99 行给了），而 `files` 列表里 `bands_tests.rs (rust, 4 symbols)` 证明文件**已被索引**；"文件被索引却 0 caller"本身就是边没解析的信号——即**两个工具返回之间的矛盾**触发的复核，不是猜测。

**两侧差异与归因**：
(1) **答案的判据来源**：我们这边是**工具算出来的裁决**（census 的 branch-level 两行，含"私有枚举 ⇒ 树外拼不出该变体"这条判据），agent 只需在调用前用源码 grep 预排、调用后转写；cg 这边是**夹具自己的注释**（`bands.rs:1-12` 就把三种形状与判据说全了）+ 逐条源码核对，图的贡献只有两条**假警报**与一个 `callees band_word (1): enum Band [references]`。
(2) **"回源码"的动机相反**：我们臂是**确认**（census 已下结论，源码用于写"反证"与判不了的两条）；cg 是**否证**（图说 0 caller，源码说有人在调）。两者都回源码，但一处是补证、一处是纠错。
(3) **判不了那两条的产出方式不同**：我们的 census 只给**通用**散文（数据相关条件不判、`pub` 枚举不判，另有 `1 == 2`/`!true`/`const bool`/`cfg!()` 等与本题无关的条款），agent 要把它**落到站点**（`bands.rs:42` 的数据相关参数比较、`bands.rs:92` 的 `pub` 消费者边界）；cg 直接引夹具注释里的站点句（`:83-87`）。cg 的答案还多一条我们没写的：`#[cfg(feature = "audit")]` 的 `audit` 模块与 `audit_unused`"是否'没有执行能进入'取决于使用方的构建面"（答案④）。

**代价**（README §三）：`g4` 我们 3/3/5,406/12,600/595,328 · codegraph 13/0/0/0/0（13 次里含 2 次重复 `files`、1 次失败的 `node Cargo.toml`）。

**质量**（README §四）：我们 = 26/26 命中，`g4` 在只读情景 10/10 内 ✓；codegraph = 命中，且被记为正面："`g4` 上它还**主动拒绝了 codegraph 自己的 0-caller 假警报**（`band_word`/`state_word` 其实被 `bands_tests.rs:14/19` 调用）" ✓。

**引导含义**：
1. **branch-level 的两行已经是完整答案，但只在 `census: true` 下出现**：`g1`/`g2` 的 `check` 不传该参数时只有 `… truncated: 5 of 14 census rows withheld at the limit of 5; pass `census: true` for the whole table`（`brief/ours/g2.md ①1`）。可实施：无论是否 `census: true`，都在 `verdict` 之后固定给一行 `branch-level: N constructively unreachable arm(s)`（一行，不含全文与判据），让"这棵树有没有按构造不可达的臂"不需要额外的 flag 与猜测。依据：g4 的 agent 是靠推理先猜"我 expect the tool to report 2"，而 g4 的题面并没有提示要开 census。
2. **把 `not covered by the branch-level column:` 那段 ~1,900 字符的通用散文拆成站点行**：`not-judged crates/core/src/bands.rs:42 (data-dependent condition)` / `not-judged crates/core/src/bands.rs:92 (pub enum State — a consumer outside this root may construct it)`。依据：答案③④要的正是逐站点裁决，agent 是从通用散文里"摘+落站点"的；而散文里对本题无用的条款（`macro_rules!` 体、`cfg!(…)`、通配符臂）反而更长。cg 侧对照：它靠夹具注释 `bands.rs:83-87` 直接拿到站点句。
3. **`0 callers` 这类负结论要给"边没解析"的旁证**：cg 是靠自己比对"文件已索引 vs 0 caller"才发现边缺失的。可实施的对照是**保持并写清我们已有的行为**——本桥 `callgraph --orphans` 在本题只列 `orphans 1`（`audit_unused`）并附 `note   7 function(s) in test files have no static caller either; the test harness calls them`（`brief/ours/g4.md ①2`），**没有**把 `band_word`/`state_word` 误报（`#[cfg(test)]`/`#[path]` 播种生效）。这一条应写进工具 schema/描述作为契约（"孤儿视图按 `tests/`/`_tests.rs`/`#[test]` 播种，测试文件里的被调函数不计入孤儿"），因为 F1 口径要的正是这个**可数集合**；顺带把 note 里的 `7 function(s)` 展开成清单（现在只有计数，agent 要自己回数）。

---

## 没能判定 / 口径含糊

1. **`g1`、`g2` 的 agent 推理不可归因**。表里两题都是「步 0 · 推理 0 字符」，brief 挂的"当时推理"分别属于 g4（`brief/ours/g1.md ①1`）与 g1（`brief/ours/g2.md ①1`）；真正的规划文本落在共享桶（`_shared.md` step 45 段内的 `Now g1's answer: …`）。因此这两题"agent 为何此刻这么决定"只能由调用 + 答案反推，无法引到一段**属于本题**的推理原文。
2. **切片窗口 vs 日志全文**。我们臂每条 shell 自带 `sed`/`tail` 切片（`_shared.md` step 44/45 命令原文），而 brief 的"工具返回"是**日志全文**。`g2` 的第 2/3 条 `why` 与 `next` 行在第 20 行窗口之外；答案里 `:38`/`:27`/`query.rs:27` 的出处因此无法与同树 s5 的复用区分开（s5 的 `check` 返回里有同一批行）。本报告按"窗口内为直接证据、窗口外标为不可判定"处理。
3. **F1 那半边在题与题之间的口径**。README §四判 codegraph 的 `s3`/`g1` 为"部分"，理由是没写"5 个 `#[test]` 函数由框架运行期调用、不计入"，并注明"我们臂的 `s3.md:11` 写了这条"。但**我们臂的 `g1` 答案自己也没写这半句**（`brief/ours/g1.md` 答案④只复述了 census 的播种规则），而 `g1`≡`s3` 同树同会话、该 note 就在 g1 自己的 `callgraph` 返回里（①2）。同一口径是否允许"同树的另一题写了就算"在题书里没有明说 ⇒ 标为口径含糊，不重做判定（README 已核定我们 26/26 命中）。
4. **cg 的 g4 判据有多少是"读出夹具自述"**。`bands.rs:1-12` 的模块注释把三种形状、两种判据（常量假守卫、私有枚举）连同"静态读取必须沉默"都写明了，`brief/cg/g4.md ①1` 的推理也确实以 `The three shapes (documented in bands.rs:1-12)` 开篇。本报告只记信息路径（"判据来自夹具注释 + 源码核对"），不对"发现的独立性"下结论——需要一次**去掉该注释的对照夹具**才能分离。
5. **`g1`/`g4` 答案里的措辞与日志不符处（不影响结论）**。`g1` 答案的"调用清单"把 s3 日志里的 `callers`×12 与 `node crates/core/src/audit.rs` 算作本题调用（本题日志 8 条、其中 `callers` 只有 2 次）；`g4` 答案写"`read` 到的是字面 `false`""`grep` 到的构造只有 :61"，而本题没有 `read` 调用，step 42 那次是 `grep -n ''` 打印整文件。按纪律 3 以日志为准，两处只是措辞不精确。
6. **`nonbridge` 桶在 cg 侧的完整性**。cg 自陈有若干 `diff`/`sha256sum` 是用普通 bash（非 `nb.sh`）跑的、事后才补记（`brief/cg/g4.md ①3` 段内："I ran `sha256sum ... ; diff ...` in a plain bash call (not via nb.sh) — one wasn't logged into h2's log!"）。因此 `g3` 的 `nonbridge: 18 次 / 30,744 B` 是**补记后**的读数，`diff` 那几次不在 18 之内；本报告只用它做量级说明（"真正的验证在仪器外"），不当作精确代价。
