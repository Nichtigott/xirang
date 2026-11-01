# 第九轮逐题对话分析 · 五族 c/d/e：`fc` · `fd` · `fe`

> 口径：代价（README §三）与质量判定（README §四）**直接引用，不重做**；本文件只做对话层。
> 引用写法：`（brief/ours/fc.md ①1）`= 精简版第 ①1 条配对；`（full/cg/fc.md …）`= 完整版；`（logs/<id>.jsonl 第 N 条）`、`（logs/fd.txt:60）`= 逐题日志原文；`（answers/<arm>/<id>.md:N）`= 交付答案原文。
> 逐题日志条数 = 仪器调用数（唯一可逐题归因的那一栏，README §三）；步/输出/推理/上下文是状态机归属。

---

### `fc` · 「已经采信了，现在再横向加一个。」（可改；交付后台账必须还在、门必须绿）

**我们臂的路径**：11 次仪器调用，序列（前 4 条落在 step 129/130/131/138，其余在共享/装置步）= `adopted` → `conformance {anchor:root/control/button}` → `registry` → `consistency {specimen:root/control/button}` → `apply {action:add,…}`（预览）→ `apply {…,apply:true}` → `adopted` → `conformance` → `consistency` → `registry` → `check {face:default}`（brief/ours/fc.md ①1–⑪）

- ①1 `adopted` 一条就把本题的三个前提给全了：(a) 现状「adopted since 12:32:28 (unchanged since the confirmation) (provisional) — root/control/button: …」；(b) 计数「provisional 1  lapsed 0」；(c) **新锚的写入契约**，逐字：「a route this ledger does not name is a **new anchor** — a first confirmation, not a renewal: pass `anchor`, `certifies`, `evidence`, `verifier`, `reason` and `files` together with `apply: true` and `confirm: true`, and this tool appends one line whose fingerprint it computes from those files」。
- ①2 `conformance` 给既有条目的裁定：`bytes      unchanged since the confirmation`，并列出 `covers     src/control/object/button/button.rs`。
- ①4 / ①9 `consistency --specimen` 给"横向加的那个跟没跟已采信的形状"：改前 `family root/control · member xirang-example-control-button · 1 sibling(s)`、`conformance: 0 of 1 sibling(s) do not carry the specimen's shape`；改后 `2 sibling(s)`、`conformance: 0 of 2 sibling(s) do not carry the specimen's shape`。
- ①3 / ①10 `registry` 给树的变化：`faces 3` → `faces 4`，新增行 `root/control/dial   Dial   control/object/dial/dial.rs   e1070833a032e106f9cdd0eff2149bc2`。
- ①5 / ①6 `apply` 预览+落盘：`preview effect: created \`control/object/dial/dial.rs\` under parent fb97ddd5f2b803d1b7f40a776d8a22d9`、`faces 4`、`consequences (static, text-level): 0 in-tree test line(s) name this face`。值得一提：**这段输出一个字都没提台账**。
- ⑪ `check`：`verdict  passed (cargo exit 0)`。

**决定性的一步 = ①1+①2 这一对。** `adopted` 把"要不要给新兄弟补一条台账"变成工具写死的契约（新锚＝人给 6 个字段 + `apply/confirm`，指纹由工具自己算），`conformance` 把"既有条目还算不算数"变成一句裁定。①1 时 agent 本来是困惑的——推理原文：「Hold on: fc/button.rs = e842be74 ≠ ledger's 22b54a4c ⇒ **the ledger entry's fingerprint does NOT match the current bytes**!」并列出「maybe sha256 of the file with normalization, or blake3 truncated」几种猜测，靠「The `adopted` tool will tell us. Let me run `adopted`」收场（brief/ours/fc.md ①1 推理）；到 ①2 它自己收束：「So the ledger's fingerprint (22b54a4c...) must be a normalized fingerprint matching e842be74's file bytes … Whatever — the tool says the bytes are unchanged. Good. … the ledger fingerprint may be a different hash (e.g. blake3). **Not important.**」（brief/ours/fc.md ①2 推理）。落笔时答案③直接引工具原话：「新对象 `root/control/dial` 是一条**新锚**——按工具自己的话，它是「first confirmation, not a renewal」」（answers/round9/fc.md:14）。它 ①2 的推理（`conformance` 那一步）还直接把 `consistency` 认成题面的专用工具：「`consistency --specimen` … This is the perfect tool for "横向加一个" + ledger.」（full/ours/fc.md ①2；brief 版此处被截断）

**我们臂怎么证明台账没动**（四条，三条来自桥、一条来自 shell）：(a) 改后 `adopted` 仍 `provisional 1  lapsed 0`（①7）；(b) 改后 `conformance` 仍 `bytes unchanged since the confirmation`（①8）；(c) 台账文件 sha256 开工前/全部改动后同为一枚 `sha256 697a5380e448c1e52baa3d0a60423b0c8a1a7556a7231490d567b6b9223e9052`（answers/round9/fc.md:5，shell `sha256sum` ×3）；(d) 改后 `consistency` 报 `2 sibling(s)` 且 0 异类（①9）。

**codegraph 的路径**：7 次仪器调用，序列 = `init .` → `files` → `init .` → `node src/control/object/button/button.rs` → `sync .` → `node src/control/object/toggle/toggle.rs` → `files`（brief/cg/fc.md ①1–①7；步 73/77 在 turn 2，其余在 turn 3 的步 2/4/8/9/10）。

**那 21 步在干什么**（按对话可指认 14 步，其余是无调用的答复/簿记步）：
1. **turn 2 的 3 步（73 / 74 / 77）在打一份错的索引 + 做装置自述簿记。** ①1 `init` 打在**上一臂的交付态副本**上：「Indexed 14 files / ● 108 nodes, 167 edges in 530ms」；①2 `files` 清单里赫然有 `src/control/object/dial/dial.rs`——正是我们臂交付的 `dial` 面。发现后重拷，①3 再 `init`：「Indexed 13 files / ● 102 nodes, 158 edges」——**第一次 init 白做**。step 74 是一条跨 26 题的答案格式批检（`answers/fc.md: No such file or directory`）。
2. **步 2–5：前提自检**。读台账（非桥 `cat`）、读 `tests/registry.rs` 的钉子、①4 `node button.rs`（拿到要照抄的 41 行形状），然后把推理花在**成批试指纹假设**上：「sha256 of the button.rs file *without* the doc comments? / sha256 of the `control_object!` block? / Maybe it's the NodeId hash?」（full/cg/fc.md ①4），结论写在 step 5：「None of my fingerprint hypotheses match `22b54a4c…`. So the ledger's fingerprint scheme isn't derivable from the tree … I'll report that honestly.」（full/cg/fc.md step 5）
3. **步 5 还有一大段"要不要手写第二条台账"的权衡**：「Should I add a new ledger entry? … Writing a hand-made row would be fabricating evidence — I'd have to invent a fingerprint whose scheme I can't compute.」（full/cg/fc.md step 5）——这段推理我们臂的 ①1 一条 `next` 行就免掉了。
4. **步 6–7：实现**。heredoc 写 `toggle.rs` 因目录不存在失败（`bash: line 1: trees/fc/src/control/object/toggle/toggle.rs: No such file or directory` → `error[E0433]: cannot find \`toggle\` in \`object\``，非桥日志 `=== EXIT: 101`），`mkdir -p` 重来才 `rc=0`。
5. **步 8–10：索引收尾**。①5 `sync`（`Synced 3 changed files`）、①6 `node toggle.rs`（读回自己刚写的文件）、①7 `files`。这 3 条占了 7 条仪器的 43%，产出的信息是"新文件已入索引"。
6. **步 16/17：改答案格式与台账号**（`nonbridge:` 必须做末行、SETUP.md 的字节数），不是题目本身（full/cg/fc.md step 16/17）。

**题面要的东西全靠非仪器通道**：`cat .xirang/adopted/entries`、`sha256sum` ×3、`cargo test` ×3、`cargo run --example tree` ×2、`diff`。这 7 条仪器没有一条能判定题面的两个问题（这条采信是否仍然有效／新对象是否跟随已采信形状）：①1/①2 的价值是**反证**（暴露副本是错的、第一次 init 作废），①4 是照抄形状的素材，①5/①6/①7 是索引维护与留档。它自己在 ①4 就把边界写明了：「I cannot recompute the ledger's fingerprint scheme **from the tree** (no code in the tree reads/writes it)」（full/cg/fc.md ①4）。

**它怎么证明台账没动**：(a) `cat` 台账原文（非桥 1）；(b) 台账文件 sha256 基线 `697a5380e448c1e5`、`button.rs` 基线 `e842be74cce534bf`（step 5 基线捕获）；(c) 答案声明"前后都是"这两枚，并用 `--example tree` 3 行→4 行证明扩展真的发生了（brief/cg/fc.md 非桥 2/6）。它用的是**间接**判据（"被覆盖的文件字节没变"），因为它的工具说不出"这条采信现在是否仍然成立"。改后那一次 sha256 复核没进 `logs/fc.txt`（日志里只有基线那一次），复核者另用 mtime 独立核过台账一字未动（`cg26-review-2.md` §107 表行「台账文件 mtime 仍是拷贝时刻 23:24:56（它的改动都在 23:26:25 之后）」。

**两侧差异与归因**（信息差，不是"谁更聪明"）：
- **归因 1：桥有一句"这条采信是否仍描述它描述的那些字节"的裁定，codegraph 没有。** 我们 `conformance` 的 `bytes      unchanged since the confirmation` 一行直接终结了讨论；codegraph 侧因为没有这条信息通道，被迫自己造判据（文件字节 sha256），发现对不上（`e842be74…` vs `22b54a4c…`）后只能推断"指纹方案从树里推不出来"——**这句推断后来成了写进答案的错话**（见下节）。
- **归因 2：`adopted` 的 `next` 行把"新锚怎么写"说全了（6 个字段 + `apply/confirm` + 指纹由工具算），codegraph 侧没有任何一行说这件事**，于是它花了一整段推理在"要不要手写第二条"上（step 5），并最终选择"不写"——结论对，但代价与风险（差点手写）不同。
- **归因 3：`consistency --specimen` 给出"0 of N siblings do not carry the specimen's shape"这种可直接引用的裁定**，codegraph 只能用 `node button.rs` 读回形状 + 散文声称"与 Button/Slider 同形"。
- **归因 4：`apply` 一条命令完成"建面 + 报声明位置 + 报静态后果"**，codegraph 手写文件一次失败（目录）+ 一次编译失败。
- **归因 5（对 codegraph 不利、但不是它的工具差）：前 3 步在装置故障处理**（错的副本 + 错的索引），按 README §三口径已在整轮层面声明"含装置故障处理 ⇒ 对它不利、且无法从会话里扣出"。

**代价**（README §三）：仪器 **11 vs 7** · 步 **12 vs 21** · 输出 **5,860 vs 29,401 tok** · 推理 **9,527 vs 54,898 字符** · 累计上下文 **3,076,608 vs 7,804,544**。逐题净代价不可归因（多题一会话，两臂都如此）；能说的只有：它那一侧的 21 步里可指认的内容包含 2 次索引维护、1 次答案格式修正、1 次命令失败重试、成批指纹假设与"要不要手写台账"的权衡——这些在我们侧分别由 `conformance` 一行、`adopted` 的 `next` 行、`apply` 一条命令吸收掉了。

**质量**（README §四）：我们 `fc` 5/5 命中 ✓，且 0 把非缺陷报成缺陷；它 `fc` 命中 ✓ 但"理由有一处错" ✗（下表）。

**引导含义**（三条，具体到可实施）：
1. **`conformance` 的 `bytes` 行应连同指纹口径与重算值一起给**。建议加一行，例如 `fingerprint 22b54a4c… = adoption_fingerprint(path ++ 0x00 ++ len ++ 0x00 ++ content) — recomputed here ✓`（内核公开函数：`kernel/src/registry_core/adoption/adoption.rs:200`；写入点 `toolchain/src/mcp/adopted.rs:329`，两条路径见 `cg26-review-2.md` §4.1）。依据：codegraph 的推理原文「I cannot recompute the ledger's fingerprint scheme from the tree (no code in the tree reads/writes it)」（full/cg/fc.md ①4）与「None of my fingerprint hypotheses match」（step 5）；我们臂也没有这条信息，只是被 `conformance` 的裁定挡过去了，推理里留下「maybe … blake3 truncated … Not important.」（brief/ours/fc.md ①2）。**这一行不改，下一轮还会有人把"我算不出"写成"无法重算"。**
2. **`apply` 的 `consequences` 块应显式声明它对 `.xirang/adopted/entries` 没有写入**（例如 `ledger: untouched — no adopted entry names this path`）。依据：`apply` 现在的输出只列 test 行与 `cut(`/`graft(` 站点，台账一字未提（brief/ours/fc.md ①5/①6），而两侧为回答"台账还在"各掏了 3 次 sha256（answers/round9/fc.md:26；brief/cg/fc.md 非桥 1 + step 5）。
3. **把"新锚 vs 续期"提前到判定行**：`adopted` 的输出现在把这条契约放在最后一行 `next`；建议在 `provisional 1  lapsed 0` 同一行补一个计数（如 `new anchors: 0`），让"新对象要不要补一行"不必读到末尾才知道。依据：codegraph 在 step 5 的整段权衡（「Should I add a new ledger entry? … would be fabricating evidence」）与我们臂恰好读到 `next` 行就照办（answers/round9/fc.md:14）。

**关于"指纹无法重算"这句错话：README §四把它记在我们臂名下，但一手材料指向 codegraph 侧。**
- 我们臂的答案从未有这句：`target/round9/answers/fc.md` 全文 28 行（mtime 21:38，之后未被改动），无"重算/无法重算/fingerprint scheme"字样；`grep -rn "无法重算\|不可由树内重算" target/round9/` 无命中；round9 的独立复核（`target/round9/review.md:135`）对 `fc` 的判词是「**有效交付**（cg 同纪律）… **没有替人签字**，新锚按规则留给"人"」，未记任何指纹错误。
- 写这句的是 codegraph 侧：`cg26-review-2.md` §4.1 逐字引「cg26 的 `fc.md` ②③ 写：该记录的指纹"**不是文件字节的 sha256**……**树内无法重算** ⇒ 手写一条等于伪造证据"」；`target/probe-cg26/CORRECTIONS.md` §2 记录了 t5 的改正范围正是「`answers/fc.md` ③：指纹理由从"不可重算"改成流程性」，并附自检「`! grep -n "无法重算" target/probe-cg26/answers/fc.md` → 无匹配」；`target/probe-cg26/answers/fc.md` 的 mtime 是 23:38（改正时刻），round9 那份是 21:38（本轮运行时刻）。
- 它当时**试了什么**：成批候选哈希（去 doc comment、`control_object!` 块、NodeId 串、路径+source+kind 的序列化；full/cg/fc.md ①4 逐条列出），全部不中（step 5「None of my fingerprint hypotheses match」）；**缺什么信息**：一是树的 Cargo.toml 里的 `path = "/home/nich/Moirai_N3/nichlink/toolchain"` 指向的检出**不在题树内**，它没有去读 `kernel/src/registry_core/adoption/adoption.rs`（它甚至猜到"the toolchain's `call_evidence` module has API to load adopted entries and check conformance"，但随即按臂的纪律放弃，full/cg/fc.md step 3）。**会话记录可核**：整个 `fc` 作答期间（到 23:26:59 落笔为止）会话文本里 `registry_core/adoption`／`adoption_fingerprint` 出现 **0 次**，这两个词第一次出现是 23:36:56 的 t5 改正任务（cg26 会话 `225f5295-20fb-4d73-8bb7-375b9ee71a5b/session.v4.jsonl.zstd`，记录 seq 1023 / 1135）。二是桥的 `adopted`/`conformance` 本来会给一句裁定，它拿不到。于是"树内推不出"被写成了"无法重算"，并升级为"手写＝伪造证据"。
- **残留问题（可实施）**：t5 只删了"无法重算"这个词面，同一段落的最后一句还写着「"指纹方案不可由树内重算"这一条也如实写明。」（target/probe-cg26/answers/fc.md:11），与它上一行"我按这个拼法复算得 `22b54a4c…` == 台账值 ✓"（:10）**自相矛盾**。建议把 :11 那半句也一并删掉，并把自检从 grep `无法重算` 扩到 `不可由树内重算|算不出来`。

---

### `fd` · 「这个对象有问题。」（可改；交付后门必须绿）

**题面结构（已核定）**：同一 target `crates/report/tests/report.rs` 内 3 条断言红 = 2 处缺陷（filter 方向 + render 缺 `write_count`）。

**我们臂的路径**：4 次仪器调用 = `check {face:default, census:true}`（改前，failed）→ `check {face:audit}`（改前，failed）→ `check {face:default}`（改后 passed）→ `check {face:audit}`（改后 passed）（brief/ours/fd.md ①1–④；前两条落在 step 56/64）

- ①1 改前那条是本题唯一一次"工具给出形状"的调用，逐字：`verdict  failed (cargo exit 101)` + `result test result: FAILED. 0 passed; 3 failed; …` + 三个红名（`the_minimum_amount_filter_keeps_the_entries_at_or_above_it` / `the_report_prints_its_count_before_the_entries` / `the_report_names_its_count_and_its_entries`）+ 三条 `why` 行。
- **`why` 行是截断的**：第二条长行停在被折断处——`assertion \`left == right\` failed: the zero floor keeps the two non-negative…`（`…` 是工具自己打的），第三条只留 panic 消息首行 `the count block is written: LEDGER REPORT`；`left: 1 / right: 2` 与渲染转储（`a: -500` 等）**都不在 `why` 列里**（logs/fd.jsonl 第 1 条，逐字）。
- `census` 栏这一题**没有帮忙**：它报的是 `decl no test names audit_unused (…)`、`unreferenced SMALL_LIMIT`、`respelled 1000` 这类与本题两处缺陷无关的行（brief/ours/fd.md ①1）。"还有别处"在我们侧不是 census 给的。
- ①2 的 `check {face:audit}` 改前也 failed 且是同样 3 条（该面把 report 套件一起带上），主要作用是排除"另有一个只在特性面才现形的缺陷"。
- 改后两条 `check` 都 `verdict  passed (cargo exit 0)`；①3 输出里 `test-reachable: 1 of 12 …`（由 2 变 1）是修好后的静态痕迹。

**决定性的一步不是某一条返回，而是返回里那行 `next`**，逐字：「If two red things may be independent, two green runs are not the evidence: **fix one and re-run, and say which red survived**.」（brief/ours/fd.md ①1）。agent 的推理明确记了这笔账：「Let me apply all these fixes now, run the gates, and record counter-proofs (fix one, show the other red survives — **per the tool's own advice** and BRIEF's "独立性是构造出来的证据" …)」（full/ours/fd.md step 57）。答案④就是照它做的：「先只修 render（`:9`），重跑：`the_minimum_amount_filter…` **仍红**（2 passed / 1 failed）⇒ 两处独立」（answers/round9/fd.md:12）。
另外必须写明的先验：agent 在同一会话里已经做过 `s5`/`g2`（同一棵 state-5 树，其答案逐字点出 `render.rs` 缺 `write_count` 与 `query.rs:27` 反转）与 `s1`（干净参照树），它这一步的推理是「**Since the diffs are identical to s1's known-correct versions**, the fixes are: …」（full/ours/fd.md step 57）。也就是说我们侧的"两处"是 **工具的 3 红同 target + 非桥 `diff -ru s1 fd`（batching.md §2.6）+ 同族前题既有知识** 三者的合成，不是桥单独给出的。

**codegraph 的路径**：6 次仪器调用 = `init .` → `files` → `init .` → `files` → `node crates/report/src/render.rs` → `node crates/report/src/query.rs`（brief/cg/fd.md ①1–①6；逐题步数 0）。

- 逐题日志里的两次 `node` **读的是修好之后的文件**：render.rs 的 `:9` 已经是 `write_count(&mut text, entries);`、query.rs 的 `:27` 已经是 `if entry.amount < min`（brief/cg/fd.md ①5/①6）。它们不是发现，是留档。
- 真正让它相信"还有别处"的是**非仪器的 raw `cargo test` 输出**（logs/fd.txt:60，逐字）：
  ```
  running 3 tests
  test the_minimum_amount_filter_keeps_the_entries_at_or_above_it ... FAILED
  test the_report_prints_its_count_before_the_entries ... FAILED
  test the_report_names_its_count_and_its_entries ... FAILED
  …
  assertion `left == right` failed: the zero floor keeps the two non-negative entries
    left: 1
   right: 2
  …
  the count block is written: LEDGER REPORT
  a: -500
  a: 500
  b: 5000
  ```
  **同一段文本同时暴露两处**：`a: -500` 被留下（≥0 的反面）＝ filter 反了；`LEDGER REPORT` 后直接是分录、没有 `entries: N` ＝ render 缺条数块。它的答案④正是"两次失败文本互证"（brief/cg/fd.md 交付答案④）。
- 另外它用了 `diff -r trees/s1 trees/fd`（"定位 2 处差异"，target/probe-cg26/answers/fd.md:8）。这条 diff 的输出**没有进逐题日志**（只在 `BATCHING.md` 第 7 条登记、`logs/fd.txt` 里查不到），所以"diff 逐字显示了哪两处"无法核。

**两侧差异与归因**：
- 两侧**都没有"报一处就收工"**：我们侧 3 红同 target + 独立性构造；它侧 raw 文本两处互证。质量判定也都是命中（README §四）。
- 信息差在**证据的粒度**：codegraph 侧那一段 raw cargo 文本把"过滤留下了负数"与"条数行没写出来"摊在同一屏（转储是测试自己打印的渲染产物）；我们桥的 `why` 列**看不到这两条关键证据**——第一条断在 `…the two non-negative…`（连 `left/right` 都没有），第三条只留首行、转储被吃掉。我们臂要补上机制，只能去读源码（它确实走的是 `read` + s1 差分 + 同族先验）。**这不是"它更聪明"：同一份证据，一侧在工具输出里，另一侧不在。**
- 流程差在**独立性反证**：我们桥的 `next` 行把"先修一个再跑、说清哪条红活下来"写成指令，我们侧照做了；codegraph 侧的工具里没有对应的话，它的反证停在"两段失败文本互证"，没有构造"修一处→另一条仍红"的实验（brief/cg/fd.md 交付答案④）。

**代价**（README §三）：仪器 **4 vs 6** · 步 **2 vs 0** · 输出 **2,162 vs 0** · 推理 **4,671 vs 0** · 上下文 **343,168 vs 0**（`cg` 的 6 条全部落在共享/装置步，故四栏记 0，不是"没花钱"）。

**质量**（README §四）：我们 `fd` 2/2 命中 ✓（复核口径：同一 target 3 条断言 = 2 处缺陷）；它 `fd` 命中 ✓。附带记录：我们臂在答案里如实写了装置差异「`TREES.json` 记 fd 为「2 条失败」；实测是同一 target 内 3 条失败、2 处缺陷（filter + render），按缺陷数一致，此处记录差异而非视为装置作废」（answers/round9/fd.md:219）——本文件核对：`TREES.json` 现在的 fd 行已被复核者改写成「同一 target 内 3 条断言失败 = 2 处缺陷（第九轮复核实跑确认）」，臂当时读到的是旧串（full/ours/fd.md step 57 逐字引用了旧串）。

**引导含义**：
1. **`why` 列应保留 panic 载荷**（至少 `left/right` 与消息的后续行，或加一行 `payload`）。依据：fd 的 `why` 现在断在 `…the two non-negative…`（logs/fd.jsonl 第 1 条），而决定性证据「`a: -500` 与没有 `entries: N`」只存在于 raw 文本（logs/fd.txt:60）。**这一条直接对应"一题多缺陷"里第二处缺陷的机制**。
2. **verdict 块应给一句红分布计数**，例如 `red 3 assertion(s) in 1 target(s)`。依据：我们侧"2 处缺陷"是 3 红同 target 加上 `s5`/`g2` 的既有知识推出来的（full/ours/fd.md step 57「fd: same as h1 minus audit_count」）；给一行计数能让"多处缺陷"不依赖先验。
3. **保留并前置 `next` 里的独立性指令**——它已经被证明会改变行为（full/ours/fd.md step 57「per the tool's own advice」）。建议在 `verdict failed` 后紧跟一行 `independent? fix one and re-run, say which red survived`。

---

### `fe` · 「这几个对象都有问题。」（可改；交付后门必须绿）

**题面结构（已核定）**：两个面各一处、互相独立——`crates/report/src/query.rs:27`（默认面红）与 `crates/core/src/audit.rs:9`（只在 `--features audit` 面红）。

**我们臂的路径**：4 次仪器调用 = `check {face:default}`（改后 passed）→ `check {face:audit}`（改后 passed）→ `read {path:crates/core/src/audit.rs, whole}` → `read {path:crates/report/src/query.rs, lines:20-31}`（brief/ours/fe.md ①1–④；前两条落 step 64/140）。**四条全在修复之后**（logs/fe.jsonl 四条逐字可核）。

- ①1 的输出是"全绿"：`verdict  passed (cargo exit 0)`，7 个测试二进制全 ok，其中两行 `result test result: ok. **0 passed**; 0 failed; …`——正是"什么都没跑"的那两行。同一条输出里的 census 行 `decl   no test names \`audit_unused\` (crates/core/src/audit.rs:14)` 是**唯一**提到 audit 模块的地方（brief/ours/fe.md ①1）。
- ①2 `check {face:audit}`：`verdict  passed (cargo exit 0)`，其中一行 `result test result: ok. 1 passed; …` 就是那条特性面测试。
- ③/④ 两条 `read` 返回的是修好后的 `:9`（`!= 0`）与 `:27`（`< min`），属留档。

**我们臂"还有别处"的那一行不在桥里**：改前没有任何桥调用，检测走的是非桥通道——`cargo test` 两遍（默认面 1 红 / `--features audit` 1 红）、`diff -ru --exclude=.git s1 {h1,fd,fe}`（batching.md §2.6）、以及**同族前题 s2 的既有知识**（s2 的题面就是"全绿但有一个面是红的"，其答案逐字给出 `crates/core/src/audit.rs:9` 与 `#![cfg(feature = "audit")]` 两道门控，answers/round9/s2.md:7、:9-10）。agent 的推理也承认这笔先验：「Since the diffs are identical to s1's known-correct versions, the fixes are: … fe/core/audit.rs:9 `== 0` → `!= 0`」（full/ours/fe.md step 57）。
改写一下父任务的假设：让我们侧相信"还有别处"的**不是 `check` 的 census 栏**（那一栏只有 `audit_unused` 这种无关行），而是"默认面绿"这个事实 + 非桥的第二个面 + `s2` 的既有知识。桥本可以给这一行：`status` 的输出里就有——`faces  ledger-core: default=[] all=[audit]` 与 `note   a file compiled only under a non-default feature cannot fail on the default face: run \`cargo test --all-features\` (or \`--features <name>\`) before believing a green default run`（logs/s1.jsonl 第 1 条、logs/s3.jsonl 第 1 条逐字）——但本臂**没有在 fe 上调用 `status`**（全轮只有 s1/s3 两次）。

**决定性的一步（我们侧）**：答案④的独立性构造——「只修 audit（`!= 0`）后重跑：默认面**仍红**且红的就是 filter 那条（2 passed / 1 failed），而 `cargo test -p ledger-core --features audit` 已转绿（1 passed）⇒ 两处互不掩饰、各自充分」（answers/round9/fe.md:10）；驱动它的仍是同一句工具指令（full/ours/fe.md step 57「per the tool's own advice」）。

**codegraph 的路径**：6 次仪器调用 = `init .` → `files` → `init .` → `files` → `node crates/report/src/query.rs` → `node crates/core/src/audit.rs`（brief/cg/fe.md ①1–①6；逐题步数 0）。两次 `node` 同样读的是修好后的 `:27`（`< min`）与 `:9`（`!= 0`）。

**它这一侧的"还有别处"来自 raw 测试输出**（logs/fe.txt:60，逐字）：
```
test the_minimum_amount_filter_keeps_the_entries_at_or_above_it ... FAILED
test the_report_prints_its_count_before_the_entries ... ok
test the_report_names_its_count_and_its_entries ... ok
…
test result: FAILED. 2 passed; 1 failed
```
以及同一段里那行 `Running tests/audit.rs (…) / running 0 tests / test result: ok. 0 passed`——一个跑了 0 个测试的二进制。它的答案把发现过程记成：「B 在默认面看不见——若我只修 A，`--all-features` 面会红（实测两条命令对比：默认面绿但 all-features 才暴露 B）」（full/cg/fe.md ①3 里的答案草稿，逐字；交付答案④ 的对应措辞见 target/probe-cg26/answers/fe.md:7「B 单靠默认面**证伪不了**（它绿）——所以我跑了 `--all-features` 面」）。也就是说它是靠"把每个面都跑一遍 + 对比"撞上第二处的，工具没有告诉它"这棵树还有别的面"。

**两侧差异与归因**：
- 两侧都找到了两处、都没有收工；差别在**第二处的定位依据**：我们侧靠 `s2` 的既有知识 + `diff` 对 s1；它侧靠 `cargo test --all-features` 的对比 + `diff` 对 s1。**两侧的桥/工具都没有在改前给出"这棵树有第二个面且它没被这个 check 跑到"这一行。**
- 我们桥**具备**这条信息（`status` 的 `faces` 表 + `note`），只是这一题没被调用；codegraph 侧**根本没有** face 概念（它的 `files` 只给文件+符号计数，`node` 只给单个文件源码）。所以这不是"agent 忘了一步"，而是**流程表没有把 `check` 和 `status`/face 绑在一起**。

**代价**（README §三）：仪器 **4 vs 6** · 步 **4 vs 0** · 输出 **5,420 vs 0** · 推理 **7,405 vs 0** · 上下文 **784,768 vs 0**（同上，`cg` 六条都在共享/装置步）。

**质量**（README §四）：我们 `fe` 2/2 命中 ✓；它 `fe` 命中 ✓。

**引导含义**：
1. **`check` 在默认面绿、而该树还有没跑过的 face 时，必须自己报出来**，例如在 verdict 块加一行 `faces  ledger-core: all=[audit] — not run here`，或直接照抄 `status` 的 `note`（「a file compiled only under a non-default feature cannot fail on the default face: run `cargo test --all-features` … before believing a green default run」，logs/s1.jsonl 第 1 条逐字）。依据：我们 `fe` 的 4 条桥调用全在改后（logs/fe.jsonl），改前的"还有别处"完全由非桥通道与 `s2` 的既有知识供给；`check {face:default}` 自己的输出里只有两行 `0 passed`，没有任何一句解释它意味着什么（brief/ours/fe.md ①1）。
2. **`check` 头部补成员包数**（现在只有 `tree   12 rust file(s), 18 function(s)`）。依据：题面「**这几个**对象」的答案要靠"两个 crate"支撑，而这句话是 agent 自己断言的（answers/round9/fe.md:4「这几个对象」＝两个 crate）；`read` 的输出里有 `members 2`，`check` 里没有。
3. **对"一题多缺陷"，把 `check --face all` 写进流程表**（`status` 的 `note` 已经这么建议，`check` 的 `next` 行没有）。依据：codegraph 侧是"跑遍所有面"才撞见第二处（target/probe-cg26/answers/fe.md:7 + full/cg/fe.md ①3）；我们侧是"同族前题 + diff"（full/ours/fe.md step 57）——两条路都不是流程表给的。

---

## 没能判定 / 口径存疑（如实）

1. **README §四把 `fc` 的"指纹无法重算"记在我们臂名下，与一手材料不符**（`cg26-review-2.md` §4.1 明写"cg26 的 `fc.md`"；`target/probe-cg26/CORRECTIONS.md` §2 的改正范围就是那份文件；我们那份 `answers/fc.md` 全文 28 行无此句、mtime 21:38 未被改动）。同理 README 里那句"我们 `h2` 的 fix 含违禁项'删掉该失效条目'"也指向 codegraph 侧（我们 `answers/h2-claim-unkept.md:10` 写的是"台账**不能**靠编辑修…需要**人**再追加一行确认"）。**我据此只把错话归到 codegraph 侧**；若队长有意把两臂的"写下来的理由"都记成错，需要另给依据。
2. **codegraph 侧 `fc`/`fd`/`fe` 的答案自报调用数与逐题日志不符**：`fd`/`fe` 的答案 ⑤ 列了 `callers write_count`、`callers audit_count`、`node …/tests/audit.rs`、`diff -r trees/s1 trees/…`，但逐题日志只有 6 条（`init`×2、`files`×2、`node`×2），会话定位 5 条，`logs/*.txt` 里也查不到这些命令。按 README §六.3 以日志为准 ⇒ 这些调用的存在与内容**无法判定**（可能发生在未入日志的共享/装置步）。
3. **`diff -r trees/s1 trees/fd|fe` 的逐字输出无从核对**：`BATCHING.md` 只登记了它的存在（第 7/8 条），`logs/fd.txt`、`logs/fe.txt` 里没有。因此"diff 到底显示了哪两处差异、agent 是不是靠它定案"只有它答案里的一句自述（target/probe-cg26/answers/fd.md:8）。
4. **`fc` 改后的 sha256 复核没有进 codegraph 的逐题日志**（`logs/fc.txt` 只有 step 5 的基线捕获）；"前后同为一枚"的后半段依赖答案自述 + 复核者独立做的 mtime 检查（`cg26-review-2.md` §107）。
5. **`fc` 的"21 步"只能按对话里的 14 个可指认步描述**（步 73/74/77 · 2–10 · 16/17），其余步没有调用记录、也没有推理留存（被渲染脚本截断处标了"全文见 outputs/"），那部分**无法判定**其内容。
6. **`fd`/`fe` 的两臂改前状态都不在桥日志里**：我们侧 `fe` 完全没有改前桥调用、`fd` 有两条；codegraph 侧的改前状态全在非桥 raw cargo。因此"工具的哪一行让它相信还有别处"在 `fe`（我们侧）上**无法从工具输出回答**，只能从 `batching.md` 与推理复原。
7. **方法说明**：为核验 `fc` 那句错话的成因，本文件除对话渲染版外还查了 cg26 会话原文（`~/.dsh/sessions/--home-nich-Moirai_N3-xirang--/225f5295-…/session.v4.jsonl.zstd`，`zstd -dc` 后按 seq 定位）；时间线结论（作答期间 0 次 `registry_core/adoption`、首次出现在 t5 任务里）只有这条通道能给出，对话渲染版本身没有它。
8. **装置（不属我的判定对象，只登记）**：codegraph 的 `fc` 前 3 步打在上一臂交付态副本上（①2 `files` 里能看到 `dial.rs`），第一次 `init` 因此作废；README §三已声明整轮口径"含装置故障处理 ⇒ 对它不利、无法扣除"。另：`TREES.json` 的 `fd` 行在复核后被改写，臂当时读到的是旧串（见 `fd` 节）。
