//! The one-shot client: call one tool without writing an MCP client first.
//! 一次性客户端：不必先写一个 MCP 客户端就能调一个工具。
//!
//! Measured need (2026-09-29 chain-fit evaluation): the bridge had **no directly callable
//! command**, so an agent had to write and debug its own JSON-RPC client before it could ask the
//! first question. In round 1 of that evaluation the arm spent **blocks 0–10 of its 25 reasoning
//! blocks** on that plumbing — building the binary, writing a Python driver, debugging a hang on
//! `notifications/initialized`, a `pkill` that matched its own shell, and a tool list its own
//! script truncated at 6,000 characters — and only **block 11** named the defect. Codegraph, with
//! a CLI, named it at **block 3**. Two commands here are the whole fix:
//! 量出来的需求（2026-09-29 思维链拟合评测）：桥**没有可直接调用的命令**，因此 agent 必须先写、再调试一个
//! 自己的 JSON-RPC 客户端，才问得出第一个问题。那次评测第 1 题里，该组把 25 个推理块中的**块 0–10** 花在
//! 这层管道上——构建二进制、写 Python 驱动、排查 `notifications/initialized` 的挂起、一次把自己 shell 也
//! 匹配上的 `pkill`、以及被自己脚本截断在 6,000 字符的清单——**块 11** 才点名病灶。而有 CLI 的 codegraph
//! 在**块 3** 就点名了。这里的两条命令就是全部修法：
//!
//! - `xirang-mcp --list` prints the workflow table and one line per tool, so the surface arrives
//!   ordered and short instead of as ~36,000 characters of prose in one truncated payload.
//!   `--list` 打印流程表与每个工具一行，于是描述面以"有组织且短"的形式到达，而不是一帧约 36,000 字符、
//!   还被截断的散文。
//! - `xirang-mcp --call <tool> --json '{…}'` runs exactly one tool; **its exit code is the
//!   call's, never the tool's**: `0` the tool answered (its text goes to stdout), `1` it refused (its
//!   text goes to stderr), `2` the request itself was malformed (a usage error). The tool's own
//!   **verdict is in the reply body** — `check`, the one tool that judges a run, puts it on the
//!   reply's **first line**. Round 7 measured the cost of the old promise that a call's exit code was
//!   the tool's verdict: `check` on the failing fixture `target/round7/s5` answered with `exit   101`
//!   on line six while this client exited `0`, so anything that read the exit code as the verdict read
//!   a red face as green. Plain `--key value` arguments work too, so the common call needs no JSON.
//!   `--call` 只跑一个工具；**它的退出码是这次调用的，绝不是工具的**：`0` 工具作答（文本走 stdout）、
//!   `1` 工具拒绝（文本走 stderr）、`2` 请求本身畸形（用法错）。工具自己的**判定在回复正文里**——
//!   `check` 这个唯一判定一次运行的工具把它放在回复的**第一行**。第七轮量到了旧承诺的代价——那句说
//!   "一次调用的退出码就是工具的判定"的话：`check` 在失败夹具 `target/round7/s5` 上把 `exit   101`
//!   写在第六行，而这个客户端以 `0` 退出，于是任何"把退出码当判定"的读法都把红面读成了绿面。也接受
//!   普通的 `--key value` 参数，因此常见的调用不必写 JSON。
//!
//! Neither shape replaces the stdio bridge: with no arguments this binary still serves it.
//! 两种形状都不取代 stdio 桥：不带参数时这个二进制仍然做服务。

use std::io::Write;
use std::path::Path;

use serde_json::{Map, Value};

/// What the agent is told once, at `initialize`, and again by `--list`.
/// agent 在 `initialize` 时被告知一次、`--list` 时再看到的东西。
///
/// It is a **workflow table**, because that is what the evaluation's failure was about: the old
/// text was one 202-character sentence against ~28,000 characters of per-tool prose, and an agent
/// that reads the first clause of each description had nothing that said which tool answers which
/// symptom. The table names the symptom, the call, and the one thing the call cannot answer.
/// 它是一张**流程表**，因为评测失败的地方正在这里：旧文本是一条 202 字符的句子，对面是约 28,000 字符的
/// 逐工具散文，而只读每条描述第一小句的 agent 手上没有任何东西说"哪种症状用哪个工具"。表里点名症状、
/// 该发的调用、以及这次调用答不了的那一件事。
/// The short form of the guidance page: seven shapes, the symptom rule, and the verdict rule.
/// 指引页的短形态：七种形状、症状规则、判定规则。
///
/// The round measured the long page as read by every session and referenced by none: 17 KB, the
/// largest single reply of a session, and the sessions that piped it through `head -60` lost the
/// `keys:` line marking `face` required — so seven of the seven that truncated sent a bare
/// `check` and burned a refusal. What is kept here changes a decision: the entry call per shape,
/// and the two rules a reader acts on. The long page is still there, behind `--shapes`.
/// 那一轮量到长页「每场都读、无一引用」：17 KB、会话里最大的一条回复，而接进 `head -60` 的会话
/// 丢掉了标注 `face` 必填的 `keys:` 行——截断的七题里七题随后发了裸 `check`、白吃一次拒绝。
/// 留在这里的是能改变决定的部分：每种形状的入口调用，与读者会照做的两条规则。长页仍在 `--shapes` 后面。
pub const SHAPES_SHORT: &str = "\
**Put the request into one of seven shapes first** — by what it names and which way it moves:\n\
  empty tree          -> `new_project` -> `registry` -> `check`\n\
  inspect one object  -> `converge {node}` -> `why --at` -> `check`\n\
  a family            -> `consistency --parent <parent>` — **one** call names the outlier\n\
  inspect several     -> `consistency --parent` once, then one object at a time\n\
  a range question    -> `check` and dispose of every census column\n\
  add an object       -> `registry` -> `apply {action: \"add\"}` -> `consistency --specimen` -> `check`\n\
  deepen an object    -> `explain {node}` -> `apply {action: \"deepen\"}` -> `check`\n\
  move or merge       -> `affected` -> `why` (plan half) -> `grafts` -> `apply`\n\
**Symptom first**: run the suite you already have (`cargo test`) — the failing assertion's own words are the clue. On a tree with no registered face, \"the object\" can only mean a *symbol*: go to `search` / `locate` / `read` rather than the manuals.\n\
**`check`'s verdict is the first line of its reply** (`verdict  passed (cargo exit 0)`), not this client's exit code (`0` answered, `1` refused, `2` a usage error).\n\
**Answer shape**: understanding -> `read --whole`; structural -> bounded.\n\
Tool page: `--list <tool>`; full guide: `--shapes`.\n\
**先把请求放进七种形状之一**：空树 ⇒ `new_project`；查看单对象 ⇒ `converge` → `why --at`/`check {face}`；有兄弟 ⇒ `consistency --parent` 一次点名异类；查看多个 ⇒ 同上再逐个；范围型 ⇒ `check` 逐栏处置；新增 ⇒ `registry`/`apply`/`consistency --specimen`/`check`；加深 ⇒ `explain`/`apply`/`check`；迁移 ⇒ `affected`/`why`/`grafts`/`apply`。**先读症状**：跑 `cargo test`，失败断言的原话就是下一条线索；**无注册面的树上「对象」只能指符号**，直接 `search`/`locate`/`read`。答案形状：理解型⇒read --whole，结构型⇒有界。**`check` 的判定在它回复第一行**。完整页是 `--shapes`。\n";

/// The handshake text: the capability map, the routing rule, and the one gate a writer must know.
/// 握手文本：能力地图、路由规则、以及写者必须知道的那一道门。
///
/// Audit `W1-1`: this is what `initialize` hands the client, and the round measured the old text as
/// seven kilobytes read by every session and referenced by none. Its budget is 550 characters — the
/// map says **where each capability lives**, not what it answers, because the answers are the tools'
/// own replies and their refusals. The long page is `--shapes`; one tool's whole description is
/// `--list <tool>`; the whole surface on demand is the `xirang_tools` call.
/// 审计 `W1-1`：这是 `initialize` 交给客户端的东西，而那一轮量到旧文本是"每场都读、无一引用"的七千字节。
/// 它的预算是 550 字符——地图说的是**每项能力住在哪**，不是它答什么，因为答案在各工具自己的回复与拒绝里。
/// 长页是 `--shapes`；一个工具的完整描述是 `--list <tool>`；整面按需取是 `xirang_tools`。
pub const INSTRUCTIONS: &str = "\
Start with xirang.explore {query}: current source and evidence. Reuse shown lines; read {path,whole:true} for missing context. \
xirang_tools {tool} returns documentation ONLY; include arguments to execute. \
Test edits: {\"tool\":\"check\",\"arguments\":{\"workspace\":true,\"census\":false}}. \
verify validates registration, not Rust behavior. For registered responsibility chains: converge {node} using a returned path. \
Families: consistency; leases: conformance; empty tree: new_project. apply previews; writes need apply:true; adoption also confirm:true.";

/// The long guidance page: the prose behind every branch above. Read it by name with `--shapes`.
/// 长指引页：上面每一条背后的散文。用 `--shapes` 按名字取它。
///
/// It **used to be** what `initialize` handed the client, and audit `W1-1` is why it is not any more:
/// seven kilobytes arriving unasked, in every session, before the first question. It is still one
/// call away, and the tools' own refusals and `next` lines carry the parts a reader acts on.
/// 它**过去**是 `initialize` 交给客户端的东西，而审计 `W1-1` 正是它不再是的原因：七千字节不请自来，
/// 每个会话、在每个问题之前。它仍离一次调用，而各工具自己的拒绝与 `next` 行带着读者会照做的那些部分。
pub const GUIDANCE: &str = "\
**Place the request in one of seven shapes first — by what it names and which way it moves, not by \
its wording.** The signals are structural (how many objects it names, whether it inspects, adds, \
deepens or moves, and whether the tree is empty yet), and each shape carries an entry call and a \
**stop condition**:\n\
  empty tree                -> `new_project`, then `registry`, then `check`; stop when the skeleton \
compiles and its faces are in `registry`\n\
  inspect, one object named -> `converge {node}` for a registered face; `explore {query}` for source, then `why --at`, \
`check {face}`; stop at a root cause with a file and a line, its smallest fix, and a counter-proof. \
**A claim about a path (X reaches Y through Z) is read at each hop's own definition** — a name that \
looks like the hop is not the hop, and a route guessed from naming is the one an evaluation caught \
twice (a private helper that is never called, and a type whose fields arrive through its constructor) \
每条路径断言（X 经 Y 到 Z）都要在**每一跳自己的定义处**读出来\n\
  (an object in a family)   when the object has siblings, `consistency --parent <its parent>` \
answers `does this one differ from the family?` in **one** call — it compares the siblings' own calls \
and names the outlier, which beats reading the siblings one at a time\n\
  (independent lookups)     lookups whose arguments do not depend on each other may share **one** \
shell step (`a; b`), each writing its own `--log`; what that saves is a **turn**, and a turn re-sends \
the whole context — measured on a five-question round: 25 of 73 calls were adjacent lookups of the \
same kind, so about a third of the turns. Do it only when the next argument does not depend on the \
last answer\n\
  inspect, several named    -> `consistency --parent` **once** for the family, then one object at a \
time; stop when every named object has a verdict, including the ones with nothing wrong\n\
  inspect, a range question -> `check` and dispose of every census column; stop when each column is \
either covered or named as one it does not cover\n\
  add an object             -> `registry` for the family, `apply {action: \"add\", apply: true}`, `check`; \
the write's own reply already reports `family … outliers: x of N` for the siblings it landed among, \
so its shape question costs no second call: stop when that line names no outlier and the gates \
are green\n\
  (the parent's own rules)  a parent declares what its children must carry in \
`<parent>/registry_rule/registry_rule.rs` — `search {literal: \"REGISTRATION_RULE\"}` finds it, and a child \
that violates it **fails the build naming the missing requirement**; read it before adding or changing a \
child, because that failure arrives after the edit\n\
  deepen an object          -> `explain {node}`, `apply {action: \"deepen\", apply: true}`, then `check`; stop \
when the tree, the public paths and the factory pins are untouched\n\
  (a part is not a slot)    a cut replaces the **face**, and the parts layer lives in that face's own \
file — so a part written by `deepen` goes with it and is **not** a graft target of its own; deepening \
is how a face is changed in place, grafting is how a whole face is replaced\n\
  move or merge (refactor)  -> `affected`, the plan half of `why`, `grafts`, `apply`; stop when every \
layer's impact is disposed of, not when the move compiles\n\
**先把这个请求放进七种形状之一——按它点名什么、朝哪个方向动，而不是按它的措辞。** 信号是结构性的\
（点几个对象、是查看/新增/加深/迁移、树还空不空），每种形状带一个入口调用与一个**停止条件**：空树 ⇒ \
`new_project` 到 `registry`、`check`，停在「骨架编译过且面在 `registry` 里」；（**彼此独立的查询**）参数互不依赖的几次查询可以放在**同一条 shell 命令**里（`a; b`），每次调用照旧各写自己的 `--log`；省下的是一**轮**，而一轮要把整个上下文再发一遍——五题实测：73 次调用里有 25 次是相邻的同类查询，约三分之一轮数。**只在`下一个参数不依赖上一个答案`时这么做**；（**对象有兄弟时**）先问 `consistency --parent <它的父面>`——它一次比完同族各自的调用并点名**异类**，比逐个读兄弟便宜；查看单对象 ⇒ 注册面用 `converge {node}`，源码用 `explore {query}`，再到 `why --at`、`check {face}`，停在「根因带文件与行号、最小修、反证」；查看多个 ⇒ \
`consistency --parent` **一次**比完同族再逐个，停在「每个被点名的对象都有裁定（没问题的也要有依据）」；\
范围型 ⇒ `check` 并逐栏处置，停在「每栏要么被覆盖、要么被点名为它不覆盖」；新增 ⇒ `registry`、\
`apply {action: \"add\", apply: true}`、`check`——写回执自己就报 `family … outliers: x of N`，同族形状那一问不必再花一次调用，停在「那一行没有离群者且门禁绿」；\
**父面自己的规范**写在 `<父>/registry_rule/registry_rule.rs` 里——`search {literal: \"REGISTRATION_RULE\"}` 找得到它，\
违反的子面会**构建失败并点名缺哪条**，因此在新增或改动子面之前先读它（那次失败是在改动之后才到的）；加深 ⇒ \
`explain`、`apply {action: \"deepen\", apply: true}`、`check`，停在「树、公开路径与出厂形状钉子都没动」；\
**零件不是槽位**：切口替换的是**整个面**，而零件层住在那份面自己的文件里——因此 `deepen` 写下的零件随面一起被替换，\
**它本身不是 graft 的目标**；加深是「就地改一个面」，替换是「换掉一整个面」；迁移/合并 ⇒ \
`affected`、`why` 的计划半边、`grafts`、`apply`，停在「每一层的影响面都已处置」，而不是「能编译过」。\n\
Read the symptom first, then take the shortest route it names. A failing test? Run the suite you \
already have (`cargo test`) and keep its output: the failing assertion's own words are the next \
clue. `check {face}` is the same run aimed at **one face** — reach for it when the default face is \
green and you suspect another one (`face: \"all\"`, or a feature name), because it names the face \
it ran. **Its head carries the tree's size** (`tree   N rust file(s), M function(s)`), so a question does not have to open with `status`. **Its verdict is the first line of the reply, not this client's exit code**: the exit code \
says only how the call went (`0` answered, `1` refused, `2` a usage error), and `check` answers \
even about a failing run — so read the `verdict  passed (cargo exit 0)` line or the \
`verdict  failed (cargo exit 101)` line before anything else. **A range-type question** \
（「这里还有别的问题吗」）is the other workflow and it starts from the same reply: `check` ends with a \
whole-tree census of static facts (pass `census: true` for the whole table), and every column says \
what it does not cover — dispose of each column before opening files one by one. \
**范围型问题**（「这里还有别的问题吗」）是另一条工作流，也从同一次回复开始：`check` 末尾带一张整树\
静态事实普查（`census: true` 给整表），每栏都写明它不覆盖什么——先把每栏处置掉，再逐个打开文件。 \
**答案的形状**：理解型问题（这个文件是干什么的、为什么这样写）⇒ `read --whole` 整份倒出（界＝文件\
大小）；结构型问题（里面有什么、谁调用它）⇒ 有界的那份答案；`locate` 的回复已经指向 `read --whole`。\
**Answer shape**: an understanding question (what does this file do, why is it like this) is answered \
by the whole file (`read --whole`); a structural one (what is in it, who calls it) by the bounded \
answer above; `locate` already points at `read --whole`. \
**If the symptom is in something the \
code produces** (a rendered report, a generated record), `search {literal}` on that product's own \
words finds the line that produces it, usually in one call. **If the symptom is the assertion's \
message**, search a short, stable phrase from it: the test framework appends `left:`/`right:` and \
the numbers, so the whole line matches nothing. **A symbol name you can already read** (from a test \
or a trace) goes straight to `callgraph {function}` — one call gives its callers, its callees, its \
own lines and its doc, and `search {query}` first would only cost a second call. A name you do not \
have yet: `callgraph` hop by hop from the caller you do have. `callgraph {orphans: true}` lists what \
this package defines and nothing here calls. One function's own lines: `read {path, line}` — the \
window is ±8 lines by default, so pass `context` for a wider one and `whole` for the whole file. \
Which \
tests a change reaches: `affected`. Registration faces: `registry`, `explain`, `diff`, then `apply` \
(a preview unless `apply: true`). \\
             If the tree carries an adoption ledger (`.xirang/adopted/entries`), `adopted` reads it \
             and says which entries still hold; a byte that moved makes an entry need a **person**, \
             not an edit, and a confirmation is one more line appended. 若这棵树带采信台账 \
             （`.xirang/adopted/entries`），`adopted` 读它并说出哪些条目仍然成立；字节一动，条目就需要 \
             **人来确认**而不是改代码，而确认就是**再追加一行**。 A route the ledger never named is a **new anchor** — a first confirmation, not a \
             renewal; the read names that action. 台账从未点名的路线是**新锚**——首次确认、不是续期，\
             读取时会把该动作点出来。 **Did the design get carried through?** `conformance {anchor}` \
             says whether the ledger's claim still holds and where it lapsed; \
             `consistency --specimen <anchor>` compares the siblings against the declared shape those \
             files carry, and names who lacks which declaration. **设计落实下去了吗？** \
             `conformance {anchor}` 判台账的声明是否仍成立、在哪个文件失效；\
             `consistency --specimen <anchor>` 把兄弟与那批文件携带的已声明形状比对，点名谁缺哪条声明。 \
             **An object that should be deeper inside is not a new face**: \
             `apply {action: \"deepen\", node, inside, apply: true}` writes a parts layer into that \
             face's own file and leaves the tree, the public path and the factory pins alone, and \
             its reply prices the other reading (another face under it). 把一个对象做深**不是**加面：\
             `apply {action: \"deepen\", node, inside, apply: true}` 把零件层写进那个面自己的文件，\
             不动树、公开路径与出厂形状钉子，并在回复里给出另一种读法的代价。 \
             **Independence is evidence you construct**: two green runs do \
             not show that two defects are independent — fix or revoke one and show the other's \
             symptom is still there. **独立性是构造出来的证据**：两次全绿不能说明两处缺陷互相独立\
             ——只修其一（或撤销其一），再证明另一处的症状仍在。 With `root`, every path argument is relative to that root (root \
\"kernel\" means path \"src/…\").";

/// One line per tool: its name, then the first sentence of its description.
/// 每个工具一行：名字，然后是它描述的第一句。
///
/// The first sentence is the part that gets read — the evaluation's digests quote it — so the list
/// is the ordered, cheap view of the same catalogue `tools/list` advertises in full.
/// 第一句才是被读到的部分——评测里那些摘要引的就是它——因此这张表是同一个目录的"有序、便宜"的视图，
/// 而 `tools/list` 给的仍是完整形态。
/// The property names a tool accepts, following `anyOf`/`oneOf` one level down, with the names it
/// insists on.
/// 一个工具接受的属性名（向下跟一层 `anyOf`/`oneOf`），以及它坚持要的那些名字。
fn visit_keys(schema: Option<&Value>, names: &mut Vec<String>, required: &mut Vec<String>) {
    let Some(schema) = schema else { return };
    if let Some(items) = schema.get("required").and_then(Value::as_array) {
        for item in items.iter().filter_map(Value::as_str) {
            if !required.iter().any(|seen| seen == item) {
                required.push(item.to_owned());
            }
        }
    }
    if let Some(map) = schema.get("properties").and_then(Value::as_object) {
        for key in map.keys() {
            if !names.iter().any(|seen| seen == key) {
                names.push(key.clone());
            }
        }
    }
    // Branched names are collected **without** the required mark: a `anyOf` branch stating that it
    // needs one of two keys does not mean both are required, and starring both would be a louder
    // lie than the missing star. The refusal still names the pair.
    // 分支里的名字**不**带必填标记：`anyOf` 的一支说"两个键要其一"，不等于两个都必填，把两个都打星
    // 是比"没星"更响的谎。拒绝文案仍会把那一对点出来。
    for branch in ["anyOf", "oneOf"] {
        if let Some(items) = schema.get(branch).and_then(Value::as_array) {
            for item in items {
                let mut ignored = Vec::new();
                visit_keys(Some(item), names, &mut ignored);
            }
        }
    }
}

/// How much of one tool's description the **default** discovery page prints, in bytes.
/// **默认**发现页印一个工具描述多少字节。
///
/// Audit `W2-6` resolved the two entries that looked contradictory: `W1-4` says the long text moves
/// *into* `--list`, `W2-6` says the discovery payload halves. Both hold if the page is **bounded by
/// default and unbounded on request** — the text still lives here (nothing moved out), and the
/// default call pays for the part that decides whether to call the tool. The number is set so the
/// measured page for `why` lands near the ratio `W2-6` asked for.
/// 审计 `W2-6` 把两条看起来矛盾的条目解开了：`W1-4` 说长文**迁入** `--list`，`W2-6` 说发现载荷减半。
/// 两者同时成立，只要这一页**默认有界、按需无界**——长文仍住在这里（没有搬走），而默认那次调用只为
/// "要不要调这个工具"付费。这个数字是按 `why` 的实测页落在 `W2-6` 要的比例附近定的。
const DISCOVERY_LIMIT: usize = 250;

/// The thirteen tools the seven shapes enter through, in the catalogue's own order.
/// 七种形状进入时用到的十三个工具，按目录自己的顺序。
///
/// Audit `W2-7`: a session that needs a second instrument should not have to read the whole
/// catalogue again. This is that set — the union of the shapes' entry calls in
/// `SHAPES_SHORT` — and it is a **view**, not a second catalogue: `--list` still lists every tool.
/// 审计 `W2-7`：一个会话需要第二件仪器时，不该把整份目录再读一遍。这就是那个集合——`SHAPES_SHORT`
/// 里七种形状入口调用的并集——而它是一**视图**，不是第二份目录：`--list` 仍列出每个工具。
pub const COMMON_TOOLS: &[&str] = &[
    "xirang.explore",
    "xirang.check",
    "xirang.apply",
    "xirang.new_project",
    "xirang.registry",
    "xirang.search",
    "xirang.converge",
    "xirang.read",
    "xirang.conformance",
    "xirang.why",
    "xirang.consistency",
    "xirang.explain",
    "xirang.affected",
    "xirang.grafts",
];

/// One tool's discovery page: its keys, its decision text, and the way to the rest.
/// 一个工具的发现页：它的键、它的决策文本，以及拿到其余部分的出路。
///
/// The round measured what unbounded discovery costs (`--list` at 17 KB, read by every session and
/// referenced by none) **and** what truncation costs (an arm spent four refusals on `apply`, whose
/// description already spelled the shape out). The resolution is the same one every other bounded
/// answer here uses: print what decides the call, say that more exists, and name the call that
/// prints it — `--list <tool> --full`.
/// 那一轮既量到了无界发现的代价（`--list` 17 KB，每场都读、无一引用），也量到了截断的代价（一个臂在
/// `apply` 上白吃四次被拒，而它的描述本就写明了形状）。解法与这里每一条有界答案相同：印出决定这次调用的
/// 东西、说明还有更多、并点名印出它的那次调用——`--list <tool> --full`。
pub fn describe_tool(name: &str) -> Option<String> {
    describe_tool_within(name, DISCOVERY_LIMIT)
}

/// One tool's **whole** description, for the caller that asked for it by name.
/// 一个工具的**完整**描述，供按名字要它的调用方使用。
pub fn describe_tool_fully(name: &str) -> Option<String> {
    describe_tool_within(name, usize::MAX)
}

/// The thirteen-tool discovery view, one line each, plus where the other fifteen are.
/// 十三件工具的发现视图，每个一行，外加其余十五个在哪。
pub fn slim_tools_page() -> String {
    let mut lines = vec![format!(
        "{} tools the seven shapes enter through (one line each; `*` = required):",
        COMMON_TOOLS.len()
    )];
    lines.extend(short_lines(|name| COMMON_TOOLS.contains(&name)));
    lines.push(format!(
        "the other {} tool(s) — `--list` lists them all, `--list <tool>` prints one page\
",
        crate::mcp::tools::tools().len() - COMMON_TOOLS.len()
    ));
    lines.join("\n")
}

/// The page itself, with the description kept inside `limit` bytes.
/// 页面本身，描述保持在 `limit` 字节之内。
fn describe_tool_within(name: &str, limit: usize) -> Option<String> {
    let wanted = resolve_name(name);
    crate::mcp::tools::tools().into_iter().find_map(|tool| {
        let tool_name = tool.get("name")?.as_str()?;
        if tool_name != wanted {
            return None;
        }
        let description = tool
            .get("description")
            .and_then(Value::as_str)
            .unwrap_or("");
        let schema = tool.get("inputSchema");
        let mut properties: Vec<String> = Vec::new();
        let mut required: Vec<String> = Vec::new();
        visit_keys(schema, &mut properties, &mut required);
        // The legend only appears when something is actually marked: a line explaining an asterisk it
        // does not use is a line the reader has to undo (found in the round-8 review, once
        // `conformance` stopped requiring `anchor`).
        // 只有真的画了星号才印图例：解释一个自己没有使用的星号，是读者还得自己撤销的一行（第八轮复核后
        // `conformance` 不再要求 `anchor` 时暴露）。
        let legend = if required.is_empty() {
            ""
        } else {
            " (* = required)"
        };
        let keys = properties
            .iter()
            .map(|key| {
                if required.contains(key) {
                    format!("{key}*")
                } else {
                    key.clone()
                }
            })
            .collect::<Vec<_>>()
            .join(", ");
        // The bound is stated with its way out, like every other bounded answer here: a reader is
        // told how much was withheld and which call prints it. The cut prefers a sentence end so the
        // page reads as a page rather than as a broken line — but it never goes below half the
        // budget looking for one.
        // 上限连同出路一起说出来，与本处每条有界答案一致：读者被告知扣下多少、以及哪次调用印出它。切点
        // 优先落在句末，好让这一页读起来像一页而不是断掉的一行——但为了找句末最多只退到预算的一半。
        // A truncation costs a call, so it has to be worth one: the page is bounded only when that
        // saves at least 44% of the text (the withheld line is a fixed cost, and on a description
        // that only just exceeds the budget it would eat the saving). Otherwise the whole text is
        // printed — a page that is shorter by a tenth and costs a second call is worse than a longer
        // one that answers.
        // 一次截断要花掉一次调用，因此它得值那一次：只有当省下至少 44% 文本时才收窄这一页（那句截断说明是
        // 固定成本，在刚过预算的描述上会把省下的吃掉）。否则整段印出来——短十分之一、却要多花一次调用的页面，
        // 比长一点但能作答的页面更糟。
        let shown = if description.len() > limit {
            let mut cut = limit;
            while cut > 0 && !description.is_char_boundary(cut) {
                cut -= 1;
            }
            if let Some((at, _)) = description[..cut].rmatch_indices(['.', ';', '\n']).next()
                && at >= limit / 2
            {
                cut = at + 1;
            }
            let withheld = crate::mcp::truncation::withheld(
                description.len() - cut,
                description.len(),
                limit,
                "byte(s) of this description",
                &format!("`--list {tool_name} --full` prints all of it"),
            );
            let candidate = format!("{}\n{withheld}", &description[..cut]);
            if candidate.len() * 100 <= description.len() * 56 {
                candidate
            } else {
                description.to_owned()
            }
        } else {
            description.to_owned()
        };
        Some(format!("{tool_name}\n    keys: {keys}{legend}\n{shown}"))
    })
}

/// The catalogue name a command line spelled, with the redundant prefix added only when needed.
/// 命令行写出的目录名；只在必要时补上那个冗余前缀。
///
/// The `xirang.` prefix is redundant on a command line whose only tool source is this bridge, so a
/// bare `callgraph` resolves to `xirang.callgraph`. What it is **not** is universal: the catalogue
/// tool is called `xirang_tools` (audit `W1-1`), and blindly prefixing made the one tool the
/// advertisement tells a session to reach everything else with unreachable by name from the CLI.
/// `xirang.` 前缀在"唯一工具来源就是这个桥"的命令行上是冗余的，因此裸写 `callgraph` 解析成
/// `xirang.callgraph`。但它**不是**普遍的：目录工具叫 `xirang_tools`（审计 `W1-1`），而盲目补前缀
/// 让那个"广告让会话用它去够其余一切"的工具，在命令行上按名字够不着。
pub fn resolve_name(spelled: &str) -> String {
    let exact = crate::mcp::tools::tools()
        .into_iter()
        .any(|tool| tool.get("name").and_then(|name| name.as_str()) == Some(spelled));
    if exact || spelled.contains('.') {
        return spelled.to_owned();
    }
    format!("xirang.{spelled}")
}

/// One line per tool: its name, the first sentence of its description, and the keys it takes
/// (`*` = required), so a caller does not have to guess a key name and spend a refusal learning it.
/// 每个工具一行：名字、描述第一小句，以及它接受的键（`*` = 必填），这样调用方不必靠猜键名、再花一次
/// 被拒来学会它。
pub fn list_tool_lines() -> Vec<String> {
    short_lines(|_| true)
}

/// Complete routing labels keep the cold catalogue small without cutting sentences mid-phrase.
/// 完整路由标签让冷目录保持紧凑，不把句子从中间截断。
fn tool_summary(name: &str) -> &'static str {
    match name {
        "xirang.explore" => "Source and context",
        "xirang.search" => "Names or literal text",
        "xirang.digest" => "File contracts",
        "xirang.conformance" => "Claim versus evidence",
        "xirang.consistency" => "Sibling differences",
        "xirang.why" => "Upstream facts at a line",
        "xirang.locate" => "Rank symptom locations",
        "xirang.inspect" => "File symbols",
        "xirang.callgraph" => "Static callers and callees",
        "xirang.read" => "Files and log pages",
        "xirang.status" => "Project and test faces",
        "xirang.apply" => "Preview or apply tree edits",
        "xirang.new_project" => "Scaffold a project",
        "xirang.plugin" => "Preview or write plugin lock",
        "xirang.registry" => "Registration tree",
        "xirang.explain" => "Build selection and contracts",
        "xirang.diff" => "Registration changes",
        "xirang.trace" => "Recorded execution",
        "xirang.mir" => "Compiler MIR evidence",
        "xirang.unified" => "MIR and trace",
        "xirang.grafts" => "External graft plans",
        "xirang.impact" => "Structural change radius",
        "xirang.usages" => "Object neighbours",
        "xirang.converge" => "Repair context",
        "xirang.adopted" => "Read or record adoption",
        "xirang.affected" => "Downstream tests",
        "xirang.check" => "Verification plans or Cargo",
        "xirang.verify" => "Validate registration",
        "xirang.graph" => "Published graph edges",
        "xirang.crates" => "Preview or apply partitions",
        "xirang.multilang" => "Multilingual project inventory",
        "xirang.candidate" => "Isolated candidate",
        "xirang.rules" => "Rule book",
        "xirang.refactor" => "Template carry",
        "xirang.task" => "Maintenance closure",
        "xirang_tools" => "Tool manual or invocation",
        _ => "Tool manual",
    }
}

/// The short catalogue lines for the tools `wanted` selects.
/// `wanted` 选中的那些工具的短目录行。
///
/// One implementation for `--list` and the common-tool view (`--all-slim`), because two renderings
/// of "name, what it answers, what it takes" is how the two views would start disagreeing about the
/// same tool.
/// `--list` 与常用工具视图（`--all-slim`）共用一份实现，因为"名字、它答什么、它收什么"的两份渲染
/// 正是两个视图开始对同一个工具各说各话的方式。
fn short_lines(wanted: impl Fn(&str) -> bool) -> Vec<String> {
    crate::mcp::tools::tools()
        .into_iter()
        .filter_map(|tool| {
            let name = tool.get("name")?.as_str()?.to_owned();
            if !wanted(&name) {
                return None;
            }
            let first = tool_summary(&name);
            // The shape of the call, on the same line as its name: the round measured an arm
            // guessing a key (`--face` where the tool wanted `node`) and burning a refusal on it,
            // because this list named the tool without naming what it takes.
            // 调用的形状与名字同一行：那一轮量到一个臂猜键名（工具要 `node`、它写了 `--face`）并为此
            // 白吃一次拒绝——因为这张表只点了工具名，没说它收什么。
            let schema = tool.get("inputSchema");
            let mut properties: Vec<String> = Vec::new();
            let mut required: Vec<String> = Vec::new();
            visit_keys(schema, &mut properties, &mut required);
            // Same rule as the full page: the legend exists only when something is starred.
            // 与完整页同一条规则：只有真画了星号才印图例。
            let legend = if required.is_empty() {
                ""
            } else {
                " (* = required)"
            };
            let complete = properties.len();
            if complete > 12 {
                let primary = [
                    "action",
                    "mode",
                    "files",
                    "face",
                    "workspace",
                    "package",
                    "job",
                    "capture",
                    "path",
                    "bin",
                    "query",
                ];
                properties.retain(|key| required.contains(key) || primary.contains(&key.as_str()));
            }
            let omitted = complete.saturating_sub(properties.len());
            let keys = properties
                .iter()
                .map(|key| {
                    if required.contains(key) {
                        format!("{key}*")
                    } else {
                        key.clone()
                    }
                })
                .collect::<Vec<_>>()
                .join(", ");
            let more = if omitted > 0 {
                format!("; +{omitted} optional: --list {name}")
            } else {
                String::new()
            };
            Some(format!("{name} — {first}\n    keys: {keys}{legend}{more}"))
        })
        .collect()
}

/// Run one tool and return its text, or the text of its refusal.
/// 跑一个工具并回它的文本，或它拒绝时的文本。
pub fn call_tool(root: &Path, name: &str, arguments: &Value) -> Result<String, String> {
    // The base is chosen by the caller (`--root`) or by the process's own directory; the tools then
    // resolve their `path`/`file` arguments against it.
    // 基底由调用方（`--root`）或进程自身所在目录决定；各工具随后相对它解析自己的 `path`/`file` 参数。
    crate::mcp::tools::run_tool(root, name, arguments)
}

/// What a run of the binary should exit with, when it is not serving stdio.
/// 不是在做 stdio 服务时，这个二进制应当以什么状态退出。
pub enum Client {
    /// Serve the stdio bridge as before.
    /// 照旧做 stdio 桥服务。
    Serve,
    /// Call one tool; the number is the exit code of the **call** — answered (`0`), refused (`1`), or
    /// a usage error (`2`) — never the tool's own verdict, which is in the reply's text.
    /// 调用一个工具；那个数字是这次**调用**的退出码——作答（`0`）、拒绝（`1`）、用法错（`2`）——绝不是
    /// 工具自己的判定，后者在回复的文本里。
    Called(i32),
}

/// Print one block, treating a reader that closed the pipe as a finished reader.
/// 打印一段文本，并把"读者关掉了管道"当作读者读完了。
///
/// Measured the hard way: `--list | head -6` made `println!` panic with `Broken pipe`, which is the
/// one thing an agent piping our output through `head` or `grep` must never see. A closed pipe is
/// not a failure of ours; it is the reader saying it has enough.
/// 硬撞出来的：`--list | head -6` 让 `println!` 以 `Broken pipe` 崩溃，而这正是把我们的输出管给 `head`
/// 或 `grep` 的 agent 绝不能看到的东西。管道被关掉不是我们的失败，而是读者说它够了。
fn emit(text: &str) -> Result<(), std::io::Error> {
    let stdout = std::io::stdout();
    let mut lock = stdout.lock();
    writeln!(lock, "{text}")
}

/// Emit, or say what stopped us: `Some(code)` when the caller should exit with it.
/// 输出，或者说清是什么让我们停下：需要以它退出时回 `Some(code)`。
fn emit_or_stop(text: &str) -> Option<i32> {
    match emit(text) {
        Ok(()) => None,
        Err(error) if error.kind() == std::io::ErrorKind::BrokenPipe => Some(0),
        Err(error) => {
            eprintln!("xirang-toolchain: cannot write stdout: {error}");
            Some(2)
        }
    }
}

/// Decide what this invocation is, and do it.
/// 判断这次调用是什么，并执行它。
///
/// Arguments are the ones after the program name. `--call` takes the tool name, then any of
/// `--json <object>`, `--root <path>`, or `--<key> <value>` pairs; a bare `true`/`false` becomes a
/// boolean and digits become a number, so the common call reads like a command rather than like a
/// hand-written protocol frame.
/// 参数是程序名之后的那些。`--call` 接受工具名，然后是 `--json <对象>`、`--root <路径>`，或任意
/// `--<键> <值>` 对；裸的 `true`/`false` 变成布尔、数字串变成数字，因此常见的调用读起来像一条命令，
/// 而不像手写的协议帧。
pub fn run_client(arguments: &[String]) -> Client {
    if arguments.is_empty() {
        return Client::Serve;
    }
    // W1.7: `--log <file>` appends one JSON line per call — `{"request":[…],"response":"…","exit":N}`
    // — so a measured run needs no wrapper script around every call. The round paid one extra step
    // per instrument call for exactly that wrapper.
    // W1.7：`--log <文件>` 每次调用追一行 JSON —— `{"request":[…],"response":"…","exit":N}` —— 因此
    // 一次被测量的运行不必为每次调用套一个包装脚本。那一轮正是为这个包装每次仪器调用多花一步。
    let (log, arguments) = split_log(arguments);
    let arguments = arguments.as_slice();
    let request: Vec<&str> = arguments.iter().map(String::as_str).collect();
    match arguments[0].as_str() {
        // `--list <tool>` prints one tool's **whole** description: the one-line list is for choosing
        // a tool, and this form is for calling it without guessing the shape the list truncates.
        // `--list <tool>` 打印**一个**工具的完整描述：一行式清单用来挑工具，这个形态用来"不必猜清单
        // 截掉的形状"就能调用。
        "--list" | "-l" if arguments.get(1).is_some_and(|name| !name.starts_with('-')) => {
            let name = arguments.get(1).map(String::as_str).unwrap_or_default();
            // `--full` is the way back to the whole description (audit `W2-6`): the default page is
            // bounded, so the reader that needs the manual asks for it by name instead of paying for
            // it in every session.
            // `--full` 是回到完整描述的路（审计 `W2-6`）：默认页有界，因此需要手册的读者按名字要它，
            // 而不是每个会话都为它付费。
            let full = arguments.iter().any(|flag| flag == "--full");
            let page = if full {
                describe_tool_fully(name)
            } else {
                describe_tool(name)
            };
            match page {
                Some(text) => Client::Called(emit_or_stop(&text).unwrap_or(0)),
                None => {
                    eprintln!("unknown tool `{name}`; --list names them all");
                    Client::Called(1)
                }
            }
        }
        "--list" | "-l" if arguments.iter().any(|flag| flag == "--all-slim") => {
            Client::Called(emit_or_stop(&slim_tools_page()).unwrap_or(0))
        }
        "--list" | "-l" => {
            for block in std::iter::once(SHAPES_SHORT.to_owned())
                .chain(std::iter::once(String::new()))
                .chain(list_tool_lines())
            {
                if let Some(code) = emit_or_stop(&block) {
                    return Client::Called(code);
                }
            }
            Client::Called(0)
        }
        // The long page, by name. The one-line list carries the shapes; this carries the prose,
        // for a reader who asks for it instead of getting it every session.
        // 长页，按名字取。一行式清单带形状；这一支带散文，给主动要它的读者，而不是每场都塞给它。
        "--shapes" => Client::Called(emit_or_stop(GUIDANCE).unwrap_or(0)),
        // The **only** way a decision about a boundary enters the system. It is a front end, not a tool: the
        // bridge's catalogue has no confirmation entry, so an agent holding this binary can call tools but
        // cannot confirm its own request. The maintainer's correction of 2026-10-18 removed the terminal check
        // and the typed name: neither was evidence of a person, and what the requirement serves is that an
        // overrun is explained and the user decides whether the boundary grows.
        // **关于边界的决定进入系统的唯一入口。** 它是一个前端，不是工具：桥的目录里没有确认条目，因此拿着这个二进制的
        // agent 能调用工具，却不能确认自己的请求。维护者 2026-10-18 的纠正去掉了终端检查与手输姓名：两者都不是人的证据，
        // 而这条要求服务的是"越权要说明原因、由使用方决定边界是否扩大"。
        "--confirm" => Client::Called(crate::mcp::approval::confirm(&arguments[1..])),
        // The review entry is the host's, for the same reason the confirmation entry is: a tool that
        // can write a verdict can write its own pass, so the matrix has no review action and this
        // entry records only against the packet digest it printed.
        // 评审入口归宿主，理由与确认入口相同：能写判定的工具就能写自己的"通过"，因此工具面没有 review 动作，
        // 本入口只对照它打印过的那份材料包摘要记录。
        "--review" => Client::Called(crate::mcp::approval::review(&arguments[1..])),
        // Managed execution: the command runs inside a write domain the kernel enforces. Not a tool, and not a
        // claim about the caller — the guarantee is that a write outside the domain returns EPERM.
        // 托管执行：命令在内核执行的写域里运行。不是工具，也不是关于调用方的声明——保证是写域之外的写入返回 EPERM。
        "--managed" | "--managed-exec" => Client::Called(crate::mcp::managed::main(arguments)),
        // The name this entry used to have. Kept as a tombstone rather than silently accepted: a caller that
        // still passes `--approve` gets told what replaced it and why, instead of a usage error that reads like
        // a typo.
        // 这个入口过去的名字。留作墓碑而不是静默接受：仍然传 `--approve` 的调用方会被告知它被什么取代、以及为什么，
        // 而不是收到一个读起来像打错字的用法错误。
        "--approve" => Client::Called({
            eprintln!(
                "`--approve` is gone: it demanded a terminal and a typed name, and neither is evidence that a person approved. Use `--confirm --root <path> --id <candidate> --accept --version <version>` (or `--reject`, or `--land`) — it records a decision on one exact request"
            );
            2
        }),
        "--help" | "-h" => Client::Called(emit_or_stop(USAGE).unwrap_or(0)),
        // A bare tool name is a call: `xirang-mcp callgraph --function x` reads the way a command
        // line reads, and requiring `--call` first cost the round a refused call.
        // 裸工具名就是一次调用：`xirang-mcp callgraph --function x` 是命令行的读法，而此前必须先写
        // `--call` 让那一轮白吃了一次拒绝。
        other if other.starts_with("xirang.") || !other.starts_with('-') => {
            answered(&log, &request, call_from_arguments(arguments))
        }
        "--call" => answered(&log, &request, call_from_arguments(&arguments[1..])),
        other => {
            eprintln!("unknown argument `{other}`\n\n{USAGE}");
            Client::Called(2)
        }
    }
}

/// Emit one call's answer, and write the log line when the caller asked for one.
/// 输出一次调用的答案；调用方要了日志就写一行。
/// A refusal says what it wanted; this says what it got.
/// 拒绝文案说它要什么，这个说它**收到了什么**。
///
/// The round measured 41 refusals and classified what followed each one: 20 corrected the shape,
/// **13 left the tool for a shell** (6 of the 12 `check` refusals went to `cargo test`), 3 re-sent
/// the same shape and 4 gave up. A refusal that names the missing key without showing the supplied
/// ones leaves the reader to re-read its own command line from the transcript.
/// 那一轮量了 41 次拒绝并分类了每次之后发生了什么：20 次改对了形状，**13 次离开工具去 shell**
/// （12 次 `check` 的拒绝里有 6 次转去 `cargo test`），3 次原样重发，4 次放弃。一条只说"缺哪个键"、
/// 不显示"给了哪些"的拒绝，会让读者回去翻自己的命令行。
fn with_echo(text: String, request: &[&str]) -> String {
    let shown = request
        .iter()
        .take(8)
        .cloned()
        .collect::<Vec<_>>()
        .join(" ");
    let more = if request.len() > 8 { " …" } else { "" };
    format!("{text}\nyou passed: {shown}{more}\n")
}

fn answered(
    log: &Option<std::path::PathBuf>,
    request: &[&str],
    outcome: Result<String, Refusal>,
) -> Client {
    let (text, code) = match outcome {
        Ok(text) => (text, 0),
        Err(Refusal::Tool(text)) => (with_echo(text, request), 1),
        Err(Refusal::Usage(text)) => (with_echo(text, request), 2),
    };
    if let Some(path) = log {
        let line = serde_json::json!({
            "request": request,
            "response": text,
            "exit": code,
        });
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
        {
            use std::io::Write;
            let _ = writeln!(file, "{line}");
        }
    }
    // The reply is out; if the call started the index refresh, this process is about to exit and an
    // exit kills it. Wait (bounded) here, in the one-shot client, so `--call apply` finishes what it
    // began; giving up is safe because a run that stopped halfway is reported rather than trusted
    // (audit `M7`, P1.2). The serve path never reaches this — it has no reason to exit.
    // 回复已经写出；若这次调用启动了索引刷新，本进程即将退出，而退出会杀掉它。就在这里、在一次性客户端
    // 里等一段（有界），好让 `--call apply` 把它开始的事做完；放弃是安全的，因为半途停下的一次运行会被
    // **报告出来**而不是被相信（审计 `M7`，P1.2）。stdio 服务路径到不了这里——它没有理由退出。
    crate::mcp::index::settle(std::time::Duration::from_secs(60));
    match code {
        0 => Client::Called(emit_or_stop(&text).unwrap_or(0)),
        _ => {
            eprintln!("{text}");
            Client::Called(code)
        }
    }
}

/// Pull `--log <file>` out of the arguments; the rest is parsed as before.
/// 把 `--log <文件>` 从实参里摘出来；其余照旧解析。
fn split_log(arguments: &[String]) -> (Option<std::path::PathBuf>, Vec<String>) {
    let mut log = None;
    let mut rest = Vec::with_capacity(arguments.len());
    let mut at = 0;
    while at < arguments.len() {
        if arguments[at] == "--log" && at + 1 < arguments.len() {
            log = Some(std::path::PathBuf::from(&arguments[at + 1]));
            at += 2;
            continue;
        }
        rest.push(arguments[at].clone());
        at += 1;
    }
    (log, rest)
}

/// The first argument that names a catalogue tool, and where it sat.
/// 第一个点名目录里某个工具的实参，以及它出现的位置。
fn find_catalogue_name(arguments: &[String]) -> Option<(usize, &String)> {
    arguments.iter().enumerate().find(|(_, token)| {
        let wanted = resolve_name(token);
        crate::mcp::tools::tools()
            .into_iter()
            .any(|tool| tool.get("name").and_then(|name| name.as_str()) == Some(wanted.as_str()))
    })
}

/// How a `--call` can fail.
/// `--call` 可能怎么失败。
enum Refusal {
    /// The tool answered with a refusal; its text is the answer.
    /// 工具以拒绝作答；它的文本就是答案。
    Tool(String),
    /// The request itself was malformed.
    /// 请求本身畸形。
    Usage(String),
}

/// The usage text.
/// 用法文本。
const USAGE: &str = "\
xirang-mcp                 serve the stdio bridge (JSON-RPC 2.0)
xirang-mcp --list          the workflow table and one line per tool
xirang-mcp --call <tool>   run one tool; exit 0 answered, 1 refused, 2 usage error
                             (the tool's own verdict is in its output, e.g. `check`'s first line)
    [--json '<object>'] [--root <path>] [--<key> <value> …]
xirang-mcp --confirm       the host's confirmation entry: decide one pending widening request
                             (--root <path> --id <candidate> --accept|--reject --version <version>)
                             or authorise landing what is already inside the boundary (--land)
xirang-mcp --review        the host's review entry: print the packet an independent reviewer works from
                             (--root <path> --id <candidate>) or record a verdict against that packet
                             (--packet <digest> --verdict pass|fail [--reviewer <name>] [--finding <text>])
                             [--correction <the correction as stated>] appends a correction to the packet the reviewer reads
xirang-mcp --managed       run a command inside a write domain the kernel enforces
                             (--root <path> [--writable <path>]… -- <command> [args…])";

/// Assemble and run one `--call`.
/// 组装并执行一次 `--call`。
/// A command-line value that spells a JSON array or object, read as one.
/// 命令行上拼成 JSON 数组或对象的值，按它读。
///
/// Only containers qualify: a scalar that happens to parse as JSON (`"7"`, `"true"`) keeps the
/// scalar path, so a key's own declared type still decides how a plain number or word is read.
/// 只有容器算数：恰好能解析成 JSON 的标量（`"7"`、`"true"`）仍走标量那条路，因此一个键自己声明的
/// 类型仍然决定一个普通数字或词该怎么读。
fn json_container(text: &str) -> Option<Value> {
    if !text.starts_with('[') && !text.starts_with('{') {
        return None;
    }
    match serde_json::from_str::<Value>(text) {
        Ok(value @ (Value::Array(_) | Value::Object(_))) => Some(value),
        _ => None,
    }
}

fn call_from_arguments(arguments: &[String]) -> Result<String, Refusal> {
    // The tool name may sit **anywhere** among the flags. A round measured the cost of requiring it
    // first: `--call --json … --root … s3` was refused with "needs a tool name, not `--json`" and
    // that call bought nothing.
    // 工具名可以出现在开关之间的**任意位置**。有一轮量到"必须放最前"的代价：`--call --json … --root
    // … s3` 被拒成 "needs a tool name, not `--json`"，那次调用一无所获。
    let (raw, remaining): (&String, Vec<&String>) = match arguments.first() {
        Some(first) if !first.starts_with('-') => (first, arguments[1..].iter().collect()),
        _ => match find_catalogue_name(arguments) {
            Some((index, name)) => (
                name,
                arguments
                    .iter()
                    .enumerate()
                    .filter(|(at, _)| *at != index)
                    .map(|(_, token)| token)
                    .collect(),
            ),
            None => {
                let spelled = arguments.first().map(String::as_str).unwrap_or("");
                return Err(Refusal::Usage(format!(
                    "`--call` needs a tool name (one of the names `--list` prints), not `{spelled}`\n\n{USAGE}"
                )));
            }
        },
    };
    // The `xirang.` prefix is the catalogue's spelling; on a command line whose only tool source is
    // this bridge it is redundant, so a bare `callgraph` resolves to `xirang.callgraph`.
    // `xirang.` 前缀是目录里的拼法；在命令行上，唯一的工具来源就是这个桥，因此它是冗余的 ——
    // 裸写 `callgraph` 就解析成 `xirang.callgraph`。
    let resolved = resolve_name(raw);
    let name = resolved.as_str();
    let mut object = Map::new();
    let mut at = 0usize;
    // `--json` sets the base and the explicit pairs win, so a caller can start from a pasted object
    // and still override one key.
    // `--json` 设基底，显式键值对胜出，因此调用方可以从粘贴的对象出发、仍然覆盖某一项。
    let mut overrides: Vec<(String, Value)> = Vec::new();
    let mut base: Option<Map<String, Value>> = None;
    while at < remaining.len() {
        let flag = remaining[at];
        let Some((key, inline)) = flag_pair(flag) else {
            return Err(Refusal::Usage(format!(
                "expected `--<key>`, got `{flag}`\n\n{USAGE}"
            )));
        };
        at += 1;
        if key == "json" {
            let text = match &inline {
                Some(text) => text.as_str(),
                None => {
                    let Some(text) = remaining.get(at) else {
                        return Err(Refusal::Usage("`--json` needs an object".to_owned()));
                    };
                    at += 1;
                    text.as_str()
                }
            };
            let parsed: Value = serde_json::from_str(text)
                .map_err(|error| Refusal::Usage(format!("`--json` is not valid JSON: {error}")))?;
            match parsed {
                Value::Object(map) => base = Some(map),
                other => {
                    return Err(Refusal::Usage(format!(
                        "`--json` must be an object, got {}",
                        type_of(&other)
                    )));
                }
            }
            continue;
        }
        // A boolean is a flag: `--orphans` alone means true, and only a token that is not another
        // `--flag` is taken as a value. The round measured the cost of requiring a value here —
        // `--orphans` answered "needs a value" while the flow table recommended exactly that shape.
        // 布尔就是开关：`--orphans` 单独出现意为 true，只有不是另一个 `--flag` 的记号才被当作取值。
        // 那轮量出了"这里必须给值"的代价——流程表推荐的形状恰恰是 `--orphans`，而它回的是 "needs a
        // value"。
        let value = match inline {
            Some(text) => match json_container(&text) {
                Some(value) => value,
                None => scalar_for(&key, &text),
            },
            None => match remaining.get(at) {
                Some(next) if !next.starts_with("--") => {
                    at += 1;
                    // A value that spells a JSON array or object **is** one: `--function '["a","b"]'`
                    // is how "several symbols in one call" is written on a command line host, and the
                    // round measured what happens without this — the text arrived as one string, the
                    // tool looked for a symbol literally named `["a","b"]`, and the caller concluded
                    // the array form was unsupported. A value that does not parse, or that parses to a
                    // scalar, still goes through `scalar_for`: a string may legitimately begin with `[`.
                    // 拼成 JSON 数组或对象的值**就是**它：`--function '["a","b"]'` 是命令行宿主上写"一次问
                    // 几个符号"的方式，而那一轮量到了没有它的后果——文本作为一个字符串到达，工具去找一个字面
                    // 名叫 `["a","b"]` 的符号，调用方于是以为不支持数组形式。解析不了、或解析出标量的值仍走
                    // `scalar_for`：字符串本来就可能以 `[` 开头。
                    match json_container(next) {
                        Some(value) => value,
                        None => scalar_for(&key, next),
                    }
                }
                _ => Value::Bool(true),
            },
        };
        overrides.push((key, value));
    }
    for (key, value) in base.unwrap_or_default() {
        object.insert(key, value);
    }
    for (key, value) in overrides {
        // A repeated flag is how a list is spelled on a command line: `--files a --files b` becomes
        // the array the tool asks for. Without it the only spelling was a pasted `--json` object,
        // which the round measured costing three refused calls.
        // 重复的开关就是命令行上写列表的方式：`--files a --files b` 变成工具要的那个数组。没有它，
        // 唯一的写法是粘贴一个 `--json` 对象——那轮量到它花了三次被拒的调用。
        match object.get_mut(&key) {
            Some(Value::Array(items)) => items.push(value),
            Some(existing) => {
                let first = std::mem::take(existing);
                object.insert(key, Value::Array(vec![first, value]));
            }
            None => {
                object.insert(key, value);
            }
        }
    }
    // A dotted key is the command line's way of writing one field of an object-valued
    // argument: `--fields.module button` means `--fields '{"module":"button"}'`. Leaving it
    // as a key literally named `fields.module` makes the tool see no `fields` object at all
    // and refuse the request — the round-13 benchmark measured exactly that: a fresh agent's
    // first `apply add` on a new project spent three refused calls, then a fourth on the
    // `=` spelling, before it found the object form.
    // 点号键是命令行写"对象取值里的一个字段"的方式：`--fields.module button` 就是
    // `--fields '{"module":"button"}'`。把它留成一个字面名叫 `fields.module` 的键，会让工具根本
    // 看不到 `fields` 对象、直接拒绝——第十三轮量到的正是这件事：一个新代理在新项目上的第一次
    // `apply add` 连吃三次拒绝，再用 `=` 的写法又吃一次，才摸到对象形式。
    fold_dotted_keys(&mut object);
    // `--root` names the tree the question is about, so it decides the base every other argument is
    // resolved against. Forwarding it as an argument instead made it a path *inside* the caller's own
    // root, and a scenario round measured what that costs: a call made from the wrong directory got a
    // 475-file answer about the checkout, and only the root line on the answer caught it.
    // `--root` 命名的是"问题问的是哪棵树"，因此它决定其它每个参数相对什么解析。把它当作参数转发，会让它
    // 变成调用方自己根**内部**的一个路径，而一个情景轮量出了代价：在错的目录里发出的一次调用拿回的是关于本
    // 检出的 475 个文件，只有答案上的那行 root 抓住了它。
    let base = match object
        .remove("root")
        .and_then(|value| value.as_str().map(str::to_owned))
    {
        Some(path) => {
            let candidate = Path::new(&path);
            let absolute = if candidate.is_absolute() {
                candidate.to_path_buf()
            } else {
                std::env::current_dir()
                    .unwrap_or_else(|_| std::path::PathBuf::from("."))
                    .join(candidate)
            };
            match std::fs::canonicalize(&absolute) {
                Ok(found) => found,
                Err(error) => {
                    return Err(Refusal::Usage(format!("--root {path}: {error}")));
                }
            }
        }
        None => crate::mcp::protocol::package_root(),
    };
    call_tool(&base, name, &Value::Object(object)).map_err(Refusal::Tool)
}

/// Fold every `a.b` key into a nested object under `a`.
/// 把每个 `a.b` 键折进 `a` 底下的嵌套对象。
///
/// The fold is on the *spelling*, which is what a command line is: `--fields.module button`
/// and `--fields '{"module":"button"}'` are the same request, and only one of them reached
/// the tool before this existed. It is recursive, so `--inside.parts.width u32` nests two
/// levels, and a repeated dotted flag merges into an array exactly as a repeated plain flag
/// does.
/// 折的是**拼法**，而命令行本来就是拼法：`--fields.module button` 与
/// `--fields '{"module":"button"}'` 是同一个请求，而这个函数存在之前只有后者能到达工具。它是递归的，
/// 因此 `--inside.parts.width u32` 会嵌两层；重复的点号开关与重复的普通开关一样合并成数组。
fn fold_dotted_keys(object: &mut serde_json::Map<String, Value>) {
    let dotted: Vec<String> = object
        .keys()
        .filter(|key| key.contains('.'))
        .cloned()
        .collect();
    for key in dotted {
        // A key that must stay as written: the removal below has to know that before it
        // takes the value out, so the two ends of a leading/trailing dot are checked here.
        // 必须按原样保留的键：下面的移除要先知道这一点，因此开头或结尾的点在这里判掉。
        let segments: Vec<&str> = key.split('.').collect();
        if segments.iter().any(|segment| segment.is_empty()) {
            continue;
        }
        // A head that is already a scalar cannot hold the dotted key's tail: the caller
        // wrote two things that contradict each other, and the honest move is to leave both
        // as written so one of them is reported rather than silently dropped.
        // 头已经是个标量时装不下点号键的尾：调用方写下了互相矛盾的两样东西，诚实的做法是两者都按原样留着，
        // 让其中一个被报出来，而不是静默丢掉一个。
        let holds_object = object
            .get(segments[0])
            .is_none_or(|existing| existing.is_object());
        if !holds_object {
            continue;
        }
        let Some(value) = object.remove(&key) else {
            continue;
        };
        let mut cursor = &mut *object;
        for segment in &segments[..segments.len() - 1] {
            cursor = cursor
                .entry((*segment).to_owned())
                .or_insert_with(|| Value::Object(serde_json::Map::new()))
                .as_object_mut()
                .expect("the fold only descends into objects it created or checked");
        }
        let leaf = segments[segments.len() - 1].to_owned();
        match cursor.get_mut(&leaf) {
            Some(Value::Array(items)) => items.push(value),
            Some(existing) => {
                let first = std::mem::take(existing);
                cursor.insert(leaf, Value::Array(vec![first, value]));
            }
            None => {
                cursor.insert(leaf, value);
            }
        }
    }
}

/// The key and optional inline value of one `--flag` token.
/// 一个 `--flag` 记号里的键与可选的行内取值。
///
/// `--key value` and `--key=value` are the two standard spellings of the same pair, and the
/// round-13 benchmark measured a fresh agent trying the second one right after the dotted
/// spelling on `apply add`: untreated the key arrives as `fields.module=button`, the fold turns
/// it into a field literally named `module=button`, and the refusal names a field nobody wrote.
/// Only the key side is split, so a value that contains `=` (`--query a=b`) survives.
/// `--key value` 与 `--key=value` 是同一个键值对的两条标准拼法，而第十三轮量到一个新代理在
/// `apply add` 上紧跟着点号拼法就试了第二条：不处理时键到达为 `fields.module=button`，折叠把它变成
/// 一个名叫 `module=button` 的字段，拒绝文案于是点名一个没人写过的字段。只拆键那一侧，因此取值里含
/// `=`（`--query a=b`）能活下来。
fn flag_pair(flag: &str) -> Option<(String, Option<String>)> {
    let key = flag.strip_prefix("--")?;
    // A token with nothing before its `=` (`--=x`) has no key either way, so it is refused as
    // a flag rather than handed on as a key literally named `=x`.
    // `=` 之前什么都没有的记号（`--=x`）两边都没有键，因此按"不是开关"拒绝，而不是当成一个名叫
    // `=x` 的键交给下游。
    if key.is_empty() || key.starts_with('=') {
        return None;
    }
    Some(match key.split_once('=') {
        Some((key, value)) if !key.is_empty() => (key.to_owned(), Some(value.to_owned())),
        _ => (key.to_owned(), None),
    })
}

/// A command-line value with the type its **key** implies.
/// 命令行取值按**键**所暗示的类型解释。
///
/// Text is the default, because a command line is text: turning every digit string into a number is
/// what made `--literal 1000` unexpressible in the round (the search wanted text and got a number).
/// Only the keys that are numbers by name get a number, and `true`/`false` are booleans anywhere.
/// 默认是文本，因为命令行本来就是文本：把每个数字串都变成数字，正是那轮 `--literal 1000` 无法表达的
/// 原因（检索要文本、拿到的是数字）。只有**名字上就是数字**的键才转数字，而 `true`/`false` 在任何键上
/// 都是布尔。
fn scalar_for(key: &str, value: &str) -> Value {
    match value {
        "true" => Value::Bool(true),
        "false" => Value::Bool(false),
        other if numeric_key(key) => match other.parse::<i64>() {
            Ok(number) if number.to_string() == other => Value::from(number),
            _ => Value::from(other),
        },
        other => Value::from(other),
    }
}

/// Whether a key names a number, so its value may become one.
/// 某个键是否点名一个数字，从而它的取值可以变成数字。
fn numeric_key(key: &str) -> bool {
    matches!(key, "limit" | "line" | "context" | "count" | "depth")
        || key.ends_with("_ms")
        || key.ends_with("_bytes")
}

/// The name of a JSON value's type, for a refusal that says what it got.
/// JSON 取值的类型名，供"它收到了什么"的拒绝使用。
fn type_of(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "a boolean",
        Value::Number(_) => "a number",
        Value::String(_) => "a string",
        Value::Array(_) => "an array",
        Value::Object(_) => "an object",
    }
}

#[cfg(test)]
#[path = "client_tests.rs"]
mod client_tests;
