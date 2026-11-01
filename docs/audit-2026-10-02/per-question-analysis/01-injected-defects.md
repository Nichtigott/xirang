# 第九轮 · 逐题对话层对比 · 四道注入缺陷（`r1`–`r4`）

> 口径：本文件只做**对话层**分析。代价与质量判定直接引 `README.md` §三/§四（不重算）；
> 逐题"仪器调用数"以 **`target/round9/logs/<id>.jsonl`（我们）** 与 **`target/probe-cg26/logs/<id>.txt`（cg）** 的日志条数为准，自报不作数（README §六.3）。
> 引用图例：`（brief/ours/r1.md ①2）` = 精简版对话第 2 条；`（ours/r1.md step 14 推理）` = 完整版里的推理逐字；
> `（cg/_apparatus.md:5273）` = 完整版装置桶的行号；`（probe-cg26/logs/r1.txt:43-74）` = 逐题日志行号。

## 零、四题共有的两个先决事实（后面每题不再重复）

1. **症状不是仪器给的**：两侧都是自己跑 `cargo test --offline` 拿到"恰 1 条红 + 红名 + 断言值"（我们：ours/r1.md step 15 起逐树跑；cg：cg/_apparatus.md:4731 那次 `grep -a -A3 'panicked at'` 扫四份 `/tmp/cg26_r*.out`），随后把断言消息或断言值当作 `locate --symptom` 的输入。四道题里没有一次仪器调用产出过"哪条测试红了"。
2. **病灶行也不是仪器先给的**：两侧 agent 事先都用**非仪器**手段把四棵树读/比过了——
   我们：step 11 `grep -n '' Cargo.toml src/lib.rs src/model/*.rs … tests/ledger.rs` 整树带行号读（ours/r1.md step 11），随后一次跨树对比，结论逐字写在 step 14 推理里：`Now I have the diffs. Note: r1 itself has the injected defect (entry.amount > min) — r2/r3/r4 have the fixed filter plus their own injected defect`（ours/r1.md step 14 推理）；
   cg：同一条 step 5 命令里跑 `diff -r -x .codegraph r1 r2` / `r1 r3` / `r1 r4`（cg/_apparatus.md:4731 的命令、:4757-4788 `######## pairwise diffs (copies)` 的结果），把四处注入行直接列了出来。
   同一个 agent 在 step 11 的推理里把这件事说得很直白：`The MCP tool is the "被测工具" — the point is to use the bridge to answer. But reading source is also fine for cross-checking hops`（ours/r1.md step 11 推理）。
   ⇒ **本族四题的仪器/工具调用基本是"确认 + 可引用证据"，不是"发现"**。下面每题仍按"哪一次调用让结论可以被引用"来判定决定性的一步，并在"引导含义"里给出把这一步提前到工具侧的具体改法。

---

### `r1` · 注入缺陷 1（inverted-guard）：`Filter::matches` 的 min 守卫写反

**我们臂的路径**：5 次仪器调用（日志 5 条，jsonl 顺序）=
`locate {symptom:"the zero floor keeps the two non-negative entries"}` → `read {path:src/query/filter.rs, line:31, context:10}` → `why {at:src/query/filter.rs:30}` → `callgraph {function:matches}` → `check {face:default, census:true}`（brief/ours/r1.md ①1–①5）。

- `locate` 拿到了什么：`symptom "…" — 22 candidate(s), ranked by word overlap with the names and the docs of this tree's functions`，**可见的 5 行全是 `tests/ledger.rs`**：
  `tests/ledger.rs:58 the_filter_keeps_only_entries_at_or_above_the_floor (the symptom's own words appear in this file; name matches floor, keeps, entries; its doc overlaps entries; test file)` … 第 5 行 `tests/ledger.rs:98 the_summary_counts_the_entries_it_accepts (…)`，末行 `… truncated: 17 of 22 candidate locations withheld at the limit of 5; narrow the symptom's words, or read the ranked few first`（①1）。真因文件 `src/query/filter.rs` **一个位置都没露**。
- `read` 拿到了什么：`src/query/filter.rs:21-40 (40 lines)`，第 31 行逐字 `31 |             && entry.amount > min`（①2）——病灶行在，**契约行不在**：答案 ② 引的是字段 doc `src/query/filter.rs:13-14`（`/// Only entries at or above this amount.` / `/// 只保留不小于该金额的分录（若设置）。`），落在窗口外。
- `why` 拿到了什么：`at src/query/filter.rs:30 — the definition matches (lines 24-39)` + `contract (from the doc above it, which is the promise this tree writes)` 两行（`src/query/filter.rs:22 /// Whether this entry passes.` / `:23 /// 本分录是否通过。`）+ `callers tests/ledger.rs::a_receipted_filter_excludes_receiptless_entries …, the_filter_keeps_only_entries_at_or_above_the_floor …, the_filter_keeps_only_the_named_account …` + `pins a test names matches; check {face} is the run that shows it still holds`（①3）。即：契约块给的是**定义自己的 doc（22-23）**，不是它读的字段 doc（13-14）。
- `callgraph` 拿到了什么：`callers (3): …`、`callees: Some`，以及带 `contract/impl` 前缀的源码——`impl && entry.amount > min`（①4，**改前**原文）。
- `check` 拿到了什么：`verdict passed (cargo exit 0)`、`result test result: ok. 9 passed; 0 failed`，census 另报 `test-reachable: 1 of 14 production function(s) no test can reach` 与 `fn no test reaches verify (src/model/account.rs:28)`（①5）。这一条是**改后**的复验：同一份日志里 `read`/`callgraph` 打的是改前源码，而 step 20 的 edit 已把该行改成 `<`（ours/r1.md step 20）⇒ 那两条取数在改前、`check` 取数在改后（本文件一律按日志内容判定取数时刻，不按 renderer 的 step 标签；口径见"没能判定" §1/§2）。

- **决定性的一步**：不是任何一次仪器调用。agent 在 step 17 的推理里**先写出答案再排调用清单**：
  `MCP calls per r-tree: - locate --symptom "<failing assertion message>" (ranked places) - read --path src/query/filter.rs --line 31 --context 12 (window) - why --at src/query/filter.rs:30 - callgraph --function Filter::matches … - check --face default --census true`（①1 推理逐字）——`path` 与 `line 31` 都是它自己读源码得出的。本桥在这题上的作用是**把已知行号转成带契约/调用者的可引用证据**。

**codegraph 的路径**：5 次仪器调用（日志 5 条，`probe-cg26/logs/r1.txt` 的 `=== CMD` 顺序）=
`init .` → `files` → `init .` → `node src/query/filter.rs`（改前） → `node src/query/filter.rs`（改后）。

- `init`：`◆ Indexed 13 files` / `● 55 nodes, 104 edges in 520ms`（brief/cg/r1.md ①1）；`files` 给 13 个文件与每文件的符号数（`src/query/filter.rs (rust, 4 symbols)`、`tests/ledger.rs (rust, 18 symbols)`）（①2）。这两条对定位无贡献。
- **决定性的一步**：step 12 的一次批量 `./cg.sh r1 node src/query/filter.rs`（cg/_apparatus.md:5273），**整文件带行号**，把 doc 与病灶放在同一屏：`13 /// Only entries at or above this amount.` / `14 /// 只保留不小于该金额的分录（若设置）。` / `31 && entry.amount > min`（brief/cg/r1.md ①4 逐字）。紧接着 step 14 的推理直接下结论并把机制算了一遍：`**r1** (inverted-guard): src/query/filter.rs:31 && entry.amount > min — inverted (should be <). Test the_filter_keeps_only_entries_at_or_above_the_floor (tests/ledger.rs:69) fails: left 1, right 2 — the zero floor keeps only the *negative* entry (inverted: keeps amounts > min → drops 500 and 5000 → wait: > min with min=0 drops entries with amount > 0 → keeps -500 → kept=1 ✓ matches)`（cg/r1.md step 14 推理）。
- 另有一条**装置**读数值得记一笔：cg 的第一次 `cargo test` 复用了共享 `CARGO_TARGET_DIR`，r1 报 `test result: ok. 9 passed; 0 failed`（假绿），换 per-tree target dir 后才 `FAILED. 8 passed; 1 failed`（probe-cg26/logs/r1.txt:43-74 vs :92-132；方法坑写在 cg/_apparatus.md:7184 的自述里）。这是装置成本，不是工具差；我们臂一开始就用 `.cargo-target/<id>`（ours/r1.md step 15 推理）。

**两侧差异与归因**：
- **信息差（可见窗口的分配）**：我们把"词 → 地方"的入口 `locate` 的 5 个可见名额**全给了同一个测试文件**（22 个候选只露 5 个，而真因文件零出现）；codegraph 没有排名，`node <file>` 是整文件转储，**契约行与病灶行必然同屏**，不需要"排得准"。这一题里"用症状的词找到病灶"这条链在我们侧是断的——真因是 agent 自己读出来的，不是 `locate` 给的。
- **信息差（契约的来源）**：答案要引的契约是**字段 doc**（`min_amount` 的 13-14 行），而本桥的 `read` 窗口是"行 ± context"、`why` 只打定义自己的 doc、`callgraph` 只打函数体前两行 doc ⇒ **四道题里我们没有任何一次调用打印过 13-14 行**；cg 的整文件转储一次到位。
- **流程差**：codegraph 每题先付 `init`（本题日志 2 次 init + 1 次 files = 3/5 条与诊断无关）；我们不建索引，但每次 `read` 都要先有 `path:line`（三种形状 `line/lines/whole` 都以 path 为前提，toolchain/src/mcp/tools.rs:197-211）。

**代价**（README §三）：仪器 **5 vs 5** · 输出 1,613 vs 2,527 · 推理 3,324 vs 4,776 · 上下文 182,016 vs 89,344。

**质量**（README §四）：我们 26/26 命中，`r1` 在 4/4 注入缺陷里（复核者在 `/tmp` 重跑过红名与最小修）；codegraph 24/26 完全命中，`r1` 在其中（两处"部分"是 `s3`/`g1`）。

**引导含义**（依据都指到原文）：
1. **`locate` 的可见名额要按文件配额，并带出失败测试的调用面。** 依据：①1 的 5 行全是 `tests/ledger.rs`、`17 of 22 … withheld`，而第 58 行那条失败测试的正文里就点名了 `filter.matches(entry)`（同一份 `tests/ledger.rs`，见 brief/cg/r2.md ①6 的逐字转储 `52 .filter(|entry| filter.matches(entry))`）。本桥的 census 已经在做同样口径的文本走查（`test-reachable: … a static walk from the test files along the same name-in-call-list rule the orphan view uses`，①5）：把它接到 `locate` 上——**当"整句原样出现"的文件是测试文件时，把该测试体内点名的 production 函数单列成行**（r1 就会多出 `src/query/filter.rs:24 matches`）。
2. **`why` 的 `contract` 块要带上定义体引用到的字段/类型 doc。** 依据：答案 ② 的契约在 `:13-14`，而 `why --at src/query/filter.rs:30` 只打了 `:22-23`（①3）。实现面：`why` 已经知道定义的行区间（`the definition matches (lines 24-39)`）与它的体，把体内引用的 `self.min_amount` 一类字段的 doc 行并进 `contract` 块即可，不必新增查询。
3. **`locate` 的 `next` 行要补 `search {literal}` 与整文件读。** 依据：`next read {path, line} for the body (the reply prints the contract lines too), callgraph {function} for who calls it`（①1）把下一步全锁在"你已经知道 path 上"；而 `read` 支持 `whole: true`、`xirang.search` 本来就在分派表里（tools.rs:197-211、tools.rs:693）。这两句进 `next`，agent 就不必靠"自己想起来"。

---

### `r2` · 注入缺陷 2（dropped-call）：`render` 不再调用 `write_totals`

**我们臂的路径**：4 次仪器调用 =
`locate {symptom:"the totals block is written LEDGER REPORT"}` → `why {at:src/report/render.rs:8}` → `callgraph {function:render}` → `check {face:default}`（brief/ours/r2.md ①1–①4）。

- `locate` 拿到了什么：17 候选，**首行就是被丢掉的被调者**：`src/report/render.rs:25 write_totals (name matches totals; its doc overlaps totals, block, report)`，第 2 行失败测试 `tests/ledger.rs:106 the_report_names_its_totals_and_its_buckets (…)`，第 3 行 `src/report/render.rs:8 render (its doc overlaps totals, block, report)`（①1）。两个正确面都在，但**没有任何调用关系**。
- `why` 拿到了什么：`contract src/report/render.rs:6 /// Render one summary: a header, the totals, then one line per bucket.` / `:7 /// 渲染一份汇总：表头、合计，然后每个桶一行。` + `callers tests/ledger.rs::the_report_names_its_totals_and_its_buckets` + `pins a test names render; …`（①2）——**契约与实现的不一致**是这题的判据。
- `callgraph` 拿到了什么：`callees: from, push_str, write_totals -> src/report/render.rs:25` 与 `also here: write_totals -> src/report/render.rs:25`（①3）——**这是改后读数**（打印的源码里已有 `write_totals(&mut text, summary);`）。agent 自己在写 r2 答案时纠正了这个毛病：`Wait — one thing to be careful about for r2's counter-proof claim: … I actually never ran callgraph before the fix for r2. I ran it after the fix. I should not claim a measurement I didn't make.`（ours/r3.md step 28 推理）；最终答案 ⑤ 也如实写 `callgraph {function:render} — 修复后 callees 含 write_totals -> src/report/render.rs:25`。
- `check` 拿到了什么：`verdict passed (cargo exit 0)` + `9 passed`；census 里与本题最近的一行是 `decl   no test names write_totals (src/report/render.rs:25)`（①4）——**"没有测试点名它"**，而不是**"没有任何调用者"**。

**codegraph 的路径**：11 次仪器调用（日志 11 条）：`init`、`files`、`init`、`node src/query/filter.rs`、`node src/store/memory.rs`、`node tests/ledger.rs`、`node src/report/render.rs`（改前）、`callers write_totals`、`callees render`、`node src/query/summary.rs`、`node src/report/render.rs`（改后）。

- 改前的 `node src/report/render.rs` 给出 8-14 行的体，里面**确实没有**那一次调用（brief/cg/r2.md ①7 逐字）。
- **决定性的一步**：step 12 批量里那两条图查询，逐字为
  `Callers of "write_totals" (0):` / `function write_totals (rust) — src/report/render.rs:24` / `ℹ No callers found for "write_totals"`（①8）
  与 `Callees of "render" (1):` / `function render (rust) — src/report/render.rs:8` / `struct Summary [references]` / `src/query/summary.rs:10`（①9）。
  step 14 的推理逐字引用了它们：`机制：callers write_totals = 0、callees render 只有 Summary（引用）⇒ 合计块从未写入`（cg/r2.md step 14 推理）。

**两侧差异与归因**：这是四题里**最干净的一处信息差**。
- cg 有一条**"某个符号的入边数"**的读数（`Callers of "write_totals" (0)`），而且取在**改前**；我们的 `callgraph {function}` 只答"它调用谁 / 谁调用它"，本题取在改后，于是**我们的仪器台账里根本没有"这次调用缺失"的证据**——这条证据只存在于非仪器的反证实验里（`补回 1 行 → 全绿；再删掉该行（sed 删除）→ 只跑该条仍红`，ours/r2.md step 23 结果）。
- 归因不是"cg 更会问"：我们的桥在 dropped-call 形状上给出的最接近的一行是 census 的 `decl no test names write_totals`（①4），它答的是**测试是否点名**；而缺陷的事实是**没有任何代码调用**。两件事在本题上恰好方向相反（不被测试点名 ≠ 没人调用）。

**代价**（§三）：仪器 **4 vs 11** · 输出 5,218 vs 1,127 · 推理 5,009 vs 0 · 上下文 526,592 vs 230,272。（cg 那 11 条里 **7 条**是装置/共享阅读：`init`×2、`files`、`node src/query/filter.rs`（r1 的文件）、`node src/store/memory.rs`、`node tests/ledger.rs`、`node src/query/summary.rs`；与本题病灶直接相关的只有 4 条：`node src/report/render.rs` 改前/改后 + `callers write_totals` + `callees render`。）

**质量**（§四）：我们 26/26；codegraph 24/26 完全命中，`r2` 在其中。**附带核到一处自报不符**（不影响判定，按 §六.3 以日志为准）：cg 的 `r4` 答案 ⑤ 列了 `node src/store/memory.rs`、`node tests/ledger.rs`，而这两条记在 **r2** 的日志里，r4 日志只有 `init`×2 + `files` + `node src/model/entry.rs`×2（probe-cg26/logs/r4.txt）。

**引导含义**：
1. **`check --census true` 的声明栏应加"树内入边数"，或对 0 入边的 production 函数单列一行。** 依据：①4 的 `decl no test names write_totals (src/report/render.rs:25)` 是我们在这题上唯一的近邻行，而它答错了问题；census 本来就逐个函数走了一遍（`24 function(s) indexed in this tree`），顺手把 `callgraph {orphans:true}` 那次走查的入边数打出来（workspace.rs:322 已提到该模式），`check` 一次就能给出 `write_totals … callers 0`。
2. **`callgraph` 的 `also here:` 行带上兄弟函数的入边数。** 依据：改后那次输出逐字有 `also here: write_totals -> src/report/render.rs:25`（①3）——它已经把同文件的另一个函数摆在眼前，却没有一个数字说明有没有人调它。若写成 `also here: write_totals -> src/report/render.rs:25 (callers 0)`，那么**改前**一次 `callgraph {function:render}` 就同时给出"render 的 callees 里没有它"和"它 0 入边"，答案 ④ 的反证不必等到改后才成立。
3. **流程（可由答案模板承担）**：本桥的图证据没有"取数时刻"的标记，导致 agent 在改后才取数（自省原文见上）。依据：`callgraph` 第一行已经是 `evidence: static-heuristic`（①3/①4），再补一句"`node`/`callgraph` 的读数描述的是**取数那一刻**的树"即可；BRIEF 侧的答案模板同时要求"图证据标注改前/改后"。

---

### `r3` · 注入缺陷 3（boundary-off-by-one）：`Bucket::contains` 的上界写成闭区间

**我们臂的路径**：4 次仪器调用 =
`locate {symptom:"assertion left debit right small"}` → `why {at:src/store/index.rs:19}` → `callgraph {function:contains}` → `check {face:default, census:true}`（brief/ours/r3.md ①1–①4）。

- `locate` **打空**：`symptom "assertion left debit right small" — 2 candidate(s)`，只有
  `src/model/entry.rs:22 new (its doc overlaps debit)`
  `src/model/entry.rs:38 postable (its doc overlaps debit)`
  （①1）。真因 `src/store/index.rs:24` 不在里面；答案 ⑤ 自己写：`2 个候选，都把 debit 词排到 src/model/entry.rs（未命中真因）`。
  原因能从源码读出来：`locate` 的语料 = **函数名 + 其上方 doc**（toolchain/src/mcp/locate.rs:15-19、62-72 的 `doc_text`），而 `debit`/`small` 只作为**函数体里的字符串字面量**存在——`0 => "debit"`、`1 => "small"`（cg/r3.md ①4 的第 45-48 行），以及测试断言的值里。
- `why` 拿到了什么（本题唯一可下结论的一行）：`at src/store/index.rs:19 — the definition contains (lines 19-27)` + `contract src/store/index.rs:17 /// Whether this bucket holds an amount: at or above low, below high when it has one.` / `:18 /// 本桶是否容纳某个金额：不小于 low，有上界时严格小于它。` + `callers src/store/index.rs::bucket_of, tests/ledger.rs::the_report_names_its_totals_and_its_buckets …`（①2）。契约说"严格小于上界"，实现是 `<=`：**契约与实现相反**。
- `callgraph` 拿到了什么：`callers (2): src/store/index.rs::bucket_of, tests/ledger.rs::…`、`also here: buckets -> src/store/index.rs:32, bucket_of -> src/store/index.rs:42`、源码里 `impl Some(high) => amount < high,`（①3）——**改后**读数；改前的 `<=` 只出现在 agent 自己的 `read` 窗口里（`24: Some(high) => amount <= high,`，ours/r3.md step 21）。
- `check`：`verdict passed … 9 passed`（①4，改后）。

**codegraph 的路径**：5 次仪器调用（日志 5 条）：`init`、`files`、`init`、`node src/store/index.rs`（改前）、`node src/store/index.rs`（改后）。

- **决定性的一步**：cg step 13 的批量 `./cg.sh r3 node src/store/index.rs`（cg/_apparatus.md:5432 的命令、:5437 起的结果；brief/cg/r3.md ①4 的逐字转储）——一次**整文件**转储把四样东西放进同一屏：
  `4 /// One bucket: a half-open range of amounts.`、`11 /// Exclusive upper bound; None means unbounded.`、`17 /// … at or above low, below high when it has one.`、`24 Some(high) => amount <= high,`，以及同一文件后半的 `34 Bucket { low: i64::MIN, high: Some(0) }, 35 Bucket { low: 0, high: Some(1000) }, 36 Bucket { low: 1000, high: None }` 与 `bucket_of`（42-52）。
  答案 ② 的机制推导（`bucket_of(0)` 先命中第一个桶 ⇒ 返回 `"debit"`）用的正是同一屏里的这两个符号——**不需要任何排名**。

**两侧差异与归因**：纯**信息差**，且落在"哪些词进了排序语料"这一处。`locate` 的模块注释与输出都写明它只读名字与 doc（locate.rs:15-19：`与函数名、与它上方文档注释的词重叠`），于是**只作为函数体字面量存在的症状词换不来任何地方**；同一时刻 codegraph 把整文件摊开，病灶行离它的契约行只有 7 行。我们这一侧真正可下结论的 `contract` 行（17-18）是在 agent **已经站在 `index.rs:19`** 之后才拿到的（①2）——顺序上它是确认，不是发现。

**代价**（§三）：仪器 **4 vs 5** · 输出 1,237 vs 0 · 推理 889 vs 0 · 上下文 88,704 vs 0。
**口径提醒**：cg 的 `r3` 四栏为 0 是状态机归属问题（它的 `node` 读数发在 step 12 的共享批量里，逐题 token 不可归因），**不代表它没有读数**；本题两侧可比的只有仪器调用数 4 vs 5。

**质量**（§四）：我们 26/26（`r3` 在 4/4 里）；codegraph 24/26 完全命中，`r3` 在其中。

**引导含义**：
1. **`locate` 的语料要把函数体（至少字符串字面量）算进去。** 依据：①1 的两行候选全错，而病灶正躺在 `bucket_of` 的函数体 `0 => "debit"` / `1 => "small"` 里（cg/r3.md ①4 第 45-48 行）。它已经在读整份文件文本（locate.rs:112 `let whole_phrase = source.contains(phrase.as_str());`），把同一次扫描的"体词频"用于打分是同一份数据、不新增扫描成本。
2. **更便宜的一半：`locate` 的 `next` 行必须写 `search {literal}`。** 依据：`locate` 的模块注释自己说了这条出路（locate.rs:6-9：`它只能猜一个符号名去问 callgraph，或猜一句话去问 search {literal}`），但**回给 agent 的 `next` 行**只有 `read`/`callgraph`（①1，locate.rs:214）。而 `xirang.search` 就在分派表里（tools.rs:693）。改成 `next … , search {literal} for a phrase this ranking did not read (it scores names and docs, not bodies)`，r3 型症状就有明文出口。
3. **`why` 应把它已经知道的定义体一并打印。** 依据：`why` 的回复第一行就写着 `the definition contains (lines 19-27)`（①2）——行区间已知，却只打 doc 与 callers，不打体；本题要看到 `amount <= high` 还得再发一次 `callgraph`（而那次是改后）。把体（或 `--body`）并进 `why`，r1/r3/r4 三题各能省 1 次调用，且"契约 vs 实现"的对照会在同一条回复里。

---

### `r4` · 注入缺陷 4（validation-trap）：`Entry::postable` 的合取写成析取

**我们臂的路径**：4 次仪器调用 =
`locate {symptom:"a zero entry is not postable left Some(0) right None"}` → `why {at:src/model/entry.rs:38}` → `callgraph {function:postable}` → `check {face:default}`（brief/ours/r4.md ①1–①4）。

- `locate` 拿到了什么：11 候选，**首名就是病灶定义**：`src/model/entry.rs:38 postable (name matches postable; its doc overlaps zero, entry, postable)`，第 2 行是失败测试 `tests/ledger.rs:36 a_zero_entry_is_not_postable (name matches zero, entry, postable; test file)`（①1）。对照 r3：症状词里带了**测试名 + `postable`** 就一次到位，只给断言值就全错。
- `why` 拿到了什么：`contract` 六行，逐字含
  `src/model/entry.rs:33 /// The rule the whole service leans on: an entry is postable when it has a`
  `:34 /// receipt, and a zero entry never is. Callers rely on the refusal, so the`
  `:35 /// check is a contract rather than a convenience.`
  以及 `callers src/store/memory.rs::post (outside this file's directory (src/store))`、`pins no test names postable — nothing pins this, so a change here is unverified until something does`（①2）——**契约原文 + 唯一调用者 + "没有测试点名它"** 三样同屏。
- `callgraph` 拿到了什么：`callers (1): src/store/memory.rs::post …`、`callees: -`、`also here: new -> src/model/entry.rs:22` 与改后源码（①3）。
- **缺口（agent 自己写进答案）**：`读源码核跳（非工具）：src/store/memory.rs:22-27 是唯一入账路径。← 该跳由 read 未覆盖，用文件读数补。`（ours/r4.md 答案 ⑤）——"调用者 `post` 的体内有没有第二道拒绝"这一跳，桥没给。

**codegraph 的路径**：5 次仪器调用（日志 5 条）：`init`、`files`、`init`、`node src/model/entry.rs`（改前）、`node src/model/entry.rs`（改后）。

- **决定性的一步**：同一个 step 13 批量里的 `./cg.sh r4 node src/model/entry.rs`（cg/_apparatus.md:5432 的命令、:5495 起的结果）一次整文件转储把契约与病灶同屏：`33 /// The rule the whole service leans on: an entry is postable when it has a` / `34 /// receipt, and a zero entry never is. …` / `39 self.has_receipt || self.amount == 0`（brief/cg/r4.md ①4 逐字），与 r1/r3 同形。
- 本题它没用图查询：日志里没有 `callers`/`callees`，答案 ④ 引的 `src/store/memory.rs:22-27` 同样是**非仪器读**得来的（且答案 ⑤ 的调用清单把 r2 日志里的两条 `node` 记到了自己名下，见 r2 小节的附带核对）。

**两侧差异与归因**：这是四题里**唯一一次我们的信息形态比"整文件转储"更贴诊断**：
- `why` 把"契约原文（含 `a zero entry never is`）+ 唯一调用者 + `pins` 行"配成一组（①2），直接支撑了答案 ④ 的两条反证（`①若系统里还有第二道拒绝（例如 Store::post 自己再判一次），这一行写错也不会红—— callgraph {postable} 给出唯一调用者 Store::post，其 :23 只看 postable()，且 :26-27 直接 push ⇒ 无第二道`，ours/r4.md 答案 ④）；cg 侧要自己从整文件里挑出契约句。
- 但两侧**都**缺同一跳：调用者 `post` 的体。我们靠非仪器读 `memory.rs:22-27` 补上（答案 ⑤ 明说"该跳由 read 未覆盖"），cg 同样靠非仪器读（答案 ④ 引同一段行号）⇒ 这是工具的同一处空白，而不是某一侧的问法问题。

**代价**（§三）：仪器 **4 vs 5** · 输出 2,856 vs 2,677 · 推理 4,537 vs 7,546 · 上下文 292,480 vs 459,264。

**质量**（§四）：我们 26/26（`r4` 在 4/4 里）；codegraph 24/26 完全命中，`r4` 在其中。

**引导含义**：
1. **`callgraph {function}` 命中"调用者"时，应附调用点所在的体（±N 行，或至少调用点上下 3 行）。** 依据：四题里两侧**都**得靠非仪器读源码才能排除"第二道拒绝"（ours/r4.md 答案 ⑤ 的"该跳由 read 未覆盖"，cg 答案 ④ 引 `src/store/memory.rs:22-27` 而日志里没有该调用）。`callgraph` 已经给出 caller 的文件与符号名（`src/store/memory.rs::post`），缺的只是行与体——把调用点那几行打印出来是一次局部改动，r4 型反证就能在同一题的一次调用内闭合。
2. **把 `locate` 的输入口径写进流程：symptom 用"失败测试名 + 断言消息"，不要只给断言值。** 依据：r4 的症状 `a zero entry is not postable left Some(0) right None` 让 `locate` 首名命中病灶（①1），r3 的症状 `assertion left debit right small` 只给断言值就两行全错（brief/ours/r3.md ①1）。实现面：`check` 在红态下若能给出"失败测试名 + 断言消息"的一行（**本族我们的 `check` 全在改后跑，红态版面无法判定**，见没能判定 §1），就能顺手附一条可粘贴的 `locate {symptom: …}`。
3. **保留 `why` 的 `pins` 行并把它提到显眼处。** 依据：①2 的 `pins no test names postable — nothing pins this, so a change here is unverified until something does` 是本题"为什么敢改这一行、以及改完谁来看住它"的另一半；这一栏在四题里只有 `why` 给（r1 是 `a test names matches; check {face} is the run that shows it still holds`，①3），值得在答案模板里被要求引用。

---

## 四题合看（每题最关键的一处工具差异）

| 题 | 决定性的信息 | 我们给了什么 | 它给了什么 | 差异落在 |
| --- | --- | --- | --- | --- |
| `r1` | 契约行 + 病灶行同屏 | `locate` 5 行**全是测试文件**，真因文件零出现；契约（字段 doc `:13-14`）**无任何调用打印过** | `node <file>` 整文件：doc `:13-14` 与 `:31` 同屏 | `locate` 的语料与名额分配 |
| `r2` | "该符号 0 入边" | census 只给 `no test names write_totals`；`callgraph` 取在改后 | `Callers of "write_totals" (0)` + `Callees of "render" (1)`，改前 | 缺一列入边数 |
| `r3` | 病灶行与它的契约行相隔 7 行 | `locate` 2 行候选**全错**（症状词只活在函数体字面量里） | `node src/store/index.rs` 整文件 | `locate` 不读函数体 |
| `r4` | 契约原文 + 唯一调用者 + pins | `why` 三样同屏，支撑两条反证 | 整文件转储，契约要自己挑 | 两侧**都**缺"调用者的体"这一跳 |

## 没能判定

1. **我们臂 `check` 的红态版面**：`r1`–`r4` 的 `check` 全部在修复之后运行（`r1` 的日志里 `read`/`callgraph` 打的是改前源码、`check` 打 `9 passed`），因此"`check` 会不会打印失败测试名/断言值、会不会给出可直接粘贴的 symptom"**无法判定**；第 r3/r4 小节里依赖它的建议标了这一点。要定它需要另取一次红态读数。
2. **我们臂仪器调用的取数时刻**：对话渲染给的 step 标签与日志内容相互矛盾——`brief/ours/r1.md ①2`（renderer 标 step 25）打印的是 `&& entry.amount > min`，而 step 20 的 `edit` 已把该行改成 `<`。我按**日志内容**判定每条读数的改前/改后（`r1` 的 read/why/callgraph 与 `r3`/`r4` 的 callgraph 见其打印的源码行），不采信 renderer 的 step 标签。
3. **cg 的 `r3`/`r4` token 级对比不成立**：§三 给这两题的 `步/输出/推理/上下文` 四栏为 0，读数是共享步归属（`cg/_shared.md` 的 `### step 12（turn 2）` 批量）。这两题只有"仪器调用数"可比。
4. **cg 的 `r4` 调用清单与日志不符**（答案 ⑤ 列了 `node src/store/memory.rs`/`node tests/ledger.rs`，日志里那两条属于 `r2`）——按 §六.3 以日志为准；因此"cg 在 r4 上是否用过图查询"的答案是**没有**。
5. **本族四题难以回答"仪器能否独立把症状变成地方"**：四棵树的注入行与失败测试名都指向同一批小文件，而两侧 agent 事先都已整树读过/diff 过（零、②）。若队长要的是这条能力轴，需要看 `s1`–`s8`/`h*` 那些"只给症状、不给树"的题。
6. **cg 的一次假绿是装置口径**：共享 `CARGO_TARGET_DIR` 让同名包复用了别的树的测试二进制，`r1` 第一次跑出 `9 passed`（probe-cg26/logs/r1.txt:43-74）。它与工具能力无关，但会让"题目态自检"这一步多花读数；我们臂用 per-tree target dir 未受影响。
