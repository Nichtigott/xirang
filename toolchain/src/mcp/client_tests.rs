//! Pins for the one-shot client: the list is the catalogue one entry each (name, routing summary,
//! keys), a call's exit code says how the call went (answered, refused, or a usage error), and the
//! tool's own verdict is in its text — `check` puts it on the reply's first line.
//! 一次性客户端的钉子：清单是目录的"每条一目"形态（名字、路由摘要、键名），一次调用的退出码说的是这次调用
//! 怎么样（作答、拒绝、或用法错），而工具自己的判定在它的文本里——`check` 把它放在回复第一行。

use super::*;
use serde_json::json;

/// The list has one **entry** per tool — the name, its first sentence, and the keys it takes
/// (`*` = required) — because the round measured an arm guessing a key name (`--face` where the
/// tool wanted `node`) and spending a refusal on it: naming the tool without naming what it takes
/// is the shape that caused it. The entry's first line stays the one-line form the earlier pin
/// asserted; the keys line is the addition, and this pin now says both.
/// 清单每个工具**一条**——名字、第一句，以及它接受的键（`*` = 必填）——因为那一轮量到一个臂猜键名
/// （工具要 `node`、它写了 `--face`）并为此白吃一次拒绝：只点工具名、不说它收什么，正是造成这件事的形状。
/// 条目的第一行仍是早先那条钉子断言的"一行式"，键名行是新增的，这条钉子现在把两半都说出来。
#[test]
fn the_list_is_the_catalogue_one_entry_each() {
    let entries = list_tool_lines();
    assert_eq!(entries.len(), crate::mcp::tools::tools().len());
    for entry in &entries {
        let mut lines = entry.lines();
        let first = lines.next().unwrap_or_default();
        assert!(
            first.starts_with("xirang.") || first.starts_with("xirang_tools — "),
            "every entry is namespaced, or is the one catalogue tool: {entry}"
        );
        assert!(
            first.contains(" — "),
            "name, then the first sentence: {entry}"
        );
        let keys = lines.next().unwrap_or_default();
        assert!(
            keys.trim_start().starts_with("keys: "),
            "then its keys: {entry}"
        );
        assert_eq!(lines.count(), 0, "and nothing else: {entry}");
    }
    assert!(
        entries
            .iter()
            .any(|entry| entry.starts_with("xirang.callgraph — Static callers and callees")),
        "{entries:?}"
    );
    // The key line says which names are required, and a branched pair is not starred as if both
    // were: `search` takes `query` or `literal`, and starring both would be a louder lie.
    // 键名行说出哪些名字是必填，而"二选一"的那对不会被当成两个都必填：`search` 收 `query` 或
    // `literal`，两个都打星是更响的谎。
    let search = entries
        .iter()
        .find(|entry| entry.starts_with("xirang.search — "))
        .expect("the search entry");
    assert!(search.contains("keys: "), "{search}");
    // `--list <tool>` is the un-truncated form, because the one-line list is what cost an arm four
    // refusals on `apply`: its description already spelled the shape out.
    // `--list <tool>` 是不截断的形态，因为正是"一行式清单"让一个臂在 `apply` 上白吃四次被拒：它的描述
    // 本来就把形状写清楚了。
    let apply = describe_tool_fully("apply").expect("a described tool");
    // 用描述里**真实存在**的那半句（我第一次钉的是自己编的句子 ✗）：`apply` 的描述写明除
    // `needs_registry` 外每个 `fields` 值都是字符串。
    assert!(
        apply.contains("Every other `fields` value is a **string**"),
        "{apply}"
    );
    assert!(
        describe_tool("xirang.apply").is_some(),
        "the prefix is optional here too"
    );
    assert!(
        describe_tool("no_such_tool").is_none(),
        "unknown names are not invented"
    );

    assert!(!search.contains("query*"), "query is one of two: {search}");
    assert!(
        !search.contains("literal*"),
        "literal is one of two: {search}"
    );
}

/// The workflow table says which symptom takes which call, because the measured failure was that
/// nothing said it.
/// 流程表说出"哪种症状用哪个调用"，因为量出来的失败正是没有任何地方说这件事。
#[test]
fn the_instructions_name_the_symptom_and_the_call() {
    for expected in [
        "check {face}",
        "search {literal}",
        "search {query}",
        // The table names the tool and its key; the order it recommends is asserted below, and
        // `source` is no longer a key to remember because bodies are the default.
        // 表里点名工具与它的键；它推荐的顺序在下面另断言，而 `source` 已不再是需要记住的键——函数体是默认。
        "callgraph {function}",
        "callgraph {orphans: true}",
        "affected",
    ] {
        assert!(
            GUIDANCE.contains(expected),
            "missing `{expected}`: {GUIDANCE}"
        );
    }
    assert!(
        GUIDANCE.len() > 700,
        "the table has to carry the mapping, not one sentence: {}",
        GUIDANCE.len()
    );
}

/// The one-line list is the form every session reads, so it is the form with a bound: the round
/// measured 17 KB being read by 26 of 26 sessions, referenced by none, and truncating under
/// `head -60` — which cut off the `keys:` line that marks `face` required, so seven of seven
/// truncating sessions sent a bare `check` and burned a refusal.
/// 一行式清单是每场都读的形态，因此也是要有上限的形态：那一轮量到 17 KB 被 26/26 场读、被 0 场引用，
/// 并在 `head -60` 下被截——截掉的恰是标注 `face` 必填的 `keys:` 行，于是七个截断的会话都发了裸
/// `check`、白吃一次拒绝。
#[test]
fn the_one_line_list_stays_small_and_keeps_the_entry_calls() {
    assert!(
        SHAPES_SHORT.len() < 2600,
        "the short page has to fit a reader's first screen: {}",
        SHAPES_SHORT.len()
    );
    for expected in [
        "new_project",
        "consistency --parent",
        "apply {action: \"add\"}",
        "apply {action: \"deepen\"}",
        "affected",
        "Symptom first",
        "verdict is the first line of its reply",
        "--shapes",
    ] {
        assert!(
            SHAPES_SHORT.contains(expected),
            "missing `{expected}`: {SHAPES_SHORT}"
        );
    }
    // The long page is still reachable by name, and still carries what the short one drops.
    // 长页仍可按名字取到，并且仍带短页省掉的东西。
    assert!(GUIDANCE.len() > SHAPES_SHORT.len());
    // And the whole one-line reply — what a session actually reads — has a bound of its own.
    // 而整条一行式回复——会话真正读到的那个——有自己的上限。
    let whole = std::iter::once(SHAPES_SHORT.to_owned())
        .chain(std::iter::once(String::new()))
        .chain(list_tool_lines())
        .collect::<Vec<_>>()
        .join("\n");
    // The bound is per **catalogue size**, not a constant of nature: it was 5000 while there were
    // twenty-eight tools, `xirang.graph` is the twenty-ninth, and each entry costs about ninety
    // characters (a name, a five-word handle and its keys) — so the number moved with the catalogue
    // rather than the entry being cut to fit it. What the bound is for has not changed: `--list` is
    // read once per session and must stay a screenful. A tool that pushes past this needs the same
    // arithmetic, not a silent bump.
    // 上限是跟着**目录规模**走的，不是自然常数：二十八个工具时它是 5000，`xirang.graph` 是第二十九个，
    // 而每个条目约九十字符（名字、五个词的手柄、它的键）——因此动的是这个数，而不是把条目削到能塞进去。
    // 它的用途没有变：`--list` 每会话读一次，必须还是一屏。把它再推上去的工具需要同样的算术，而不是偷偷加一个数。
    //
    // The move to 5450 is that arithmetic: eleven tools gained one key each (`target_dir`, the
    // argument that lets a reader follow a build into an out-of-tree target directory — the
    // 2026-10-10 queue item ⑤, report §7h). A key costs about twelve characters in this listing,
    // so the eleven of them are ~130 characters, and 5200 + 130 = 5330; the measured 5341 is that
    // plus the usual slack. The same edit is what keeps the advertised **frame** (three tools)
    // under its own 4200: `explore` is the one advertised tool that reads no evidence directory,
    // so it takes the policy without the path.
    // 升到 5450 就是那套算术：十一个工具各多一个键（`target_dir`，让读取方跟着构建进树外 target
    // 目录的参数——2026-10-10 队列第 ⑤ 条，报告 §7h）。键在这份清单里约十二字符，十一个约一百三十
    // 字符，5200 + 130 = 5330，而实测 5341 就是它加上通常的余量。同一次改动也是让广告**帧**（三个
    // 工具）留在它自己 4200 之内的原因：`explore` 是唯一不读证据目录的广告工具，它只拿策略、不拿路径。
    // 5520 is the same arithmetic as the 5450 line above, moved by one tool: `candidate` adds one
    // line — its name, its label and the keys it reads — and the measured total is that plus the
    // usual slack. It is the first growth of this pin that came from a **new** entry rather than from
    // an argument, and it is the honest shape of the trade: a 32nd tool costs a line in the largest
    // reply of the session, so the label is kept short rather than the cap raised by habit.
    // 5520 是上面 5450 那行的同一套算术，被一个工具推动了：`candidate` 加一行——它的名字、标签与它读的
    // 键——而实测总数就是它加上通常的余量。这是这条钉子第一次因为**新条目**而不是新参数而长大，而这就是这笔
    // 交易诚实的形状：第 32 个工具要花掉本会话最大回复里的一行，因此标签写短，而不是习惯性地抬上限。
    //
    // 5620 is the same arithmetic once more: `multilang` adds one line (name, label) and one key
    // (`action`), and the measured 5563 is that plus the usual slack.
    // 5620 还是同一套算术：`multilang` 加了一行（名字、标签）与一个键（`action`），实测 5563 就是它
    // 加上通常的余量。
    //
    // 5660 is that arithmetic once more, and the **label** is where this one paid: `refactor` costs a
    // line (name, label) and two keys (`anchor`, `apply`), so the label is `Template carry` — nine
    // bytes shorter than the sentence it would otherwise be — and the measure is 5635. The cap moved
    // because a tool that a template change needs must be discoverable from this page; what it must
    // not do is grow by a sentence nobody measured.
    // 5660 还是同一套算术，而这一次付账的是**标签**：`refactor` 花掉一行（名字、标签）与两个键
    // （`anchor`、`apply`），因此标签写成 `Template carry`——比它本来的那句话短九个字节——而实测是
    // 5635。上限之所以移动，是因为"模板变化时需要的那件工具"必须能从这一页被发现；不该发生的是它因为
    // 一句没人量过的句子而长大。
    // 5800 is the same arithmetic with the largest single step so far: `task` adds one line whose key
    // list is long because the tool's shape needs it — an action, an id, the object, the plan that
    // verified it, and the two switches (`apply`, `confirm`) that keep a write from happening by
    // silence. The measured total is 5778; the alternative was a tool whose keys a reader cannot
    // discover, which is the cost this page exists to avoid paying.
    // 5800 还是同一套算术，只是这一次的单步最大：`task` 加的一行键很多，而那是工具形状需要的——一个动作、
    // 一个 id、对象、验证它的计划，以及两个防止"沉默即写入"的开关（`apply`、`confirm`）。实测总数 5778；
    // 另一种做法是让读者发现不了键，而那正是这一页存在所要避免的代价。
    assert!(
        whole.len() < 5800,
        "`--list` is the largest reply of a session and must stay a screenful: {}",
        whole.len()
    );
}

/// The table no longer says the exit code is the verdict: round 7 measured that promise false, and a
/// reader who believed it read a red face as green.
/// 流程表不再说"退出码就是判定"：第七轮量出那句承诺是假的，而信了它的读者把红面读成了绿。
///
/// The measurement, on the frozen fixture `target/round7/s5`: `check` answered `exit   101` on line
/// six of the reply while the one-shot client exited `0`, so the promise held for the green `s1` and
/// failed exactly where a reader needed it. What replaces it names both halves — where the verdict
/// is (`check`'s first line) and what the exit code does say (`0`/`1`/`2` = answered/refused/usage
/// error). The strings are pinned verbatim, because the failure was a *sentence* nobody checked.
/// 实测（冻结夹具 `target/round7/s5`）：`check` 把 `exit   101` 写在回复第六行，而一次性客户端以 `0`
/// 退出——那句承诺对绿的 `s1` 成立、恰好在读者最需要它的地方不成立。替换它的文本把两半都点名——判定在哪
/// （`check` 的第一行）以及退出码到底说的是什么（`0`/`1`/`2` = 作答/拒绝/用法错）。字符串逐字钉住，因为
/// 失败的正是一句没人检查的**话**。
#[test]
fn the_table_does_not_read_the_exit_code_as_the_verdict() {
    assert!(
        !GUIDANCE.contains("exit code the verdict"),
        "the promise the round measured false is gone: {GUIDANCE}"
    );
    for expected in [
        "verdict is the first line of the reply",
        "`0` answered, `1` refused, `2` a usage error",
        "verdict  passed (cargo exit 0)",
        "verdict  failed (cargo exit 101)",
        // The range-type workflow starts from the same reply's census, and the table has to say so:
        // the round measured agents opening files one by one for a question the census already
        // answered column by column.
        // 范围型工作流要从同一次回复的普查开始，表里必须写明：那一轮量到代理为一个"普查已逐栏回答"的
        // 问题去逐个打开文件。
        "A range-type question",
        "census: true",
    ] {
        assert!(
            GUIDANCE.contains(expected),
            "missing `{expected}`: {GUIDANCE}"
        );
    }
}

/// `--list check` says which line the verdict lands on and what the client's exit code means, because
/// the measured defect was an agent taking the exit code for the verdict.
/// `--list check` 说明判定落在哪一行、以及客户端的退出码是什么意思，因为量到的缺陷正是 agent 把退出码当判定。
#[test]
fn the_check_description_names_the_verdict_line_and_the_exit_code() {
    // Audit `W2-6` bounded the **default** page, so a pin about the manual asks for the manual.
    // 审计 `W2-6` 把**默认**页收成有界，因此关于手册的钉子按名字要手册。
    let check = describe_tool_fully("check").expect("the check tool is described");
    for expected in [
        "The reply's first line is the run's verdict",
        "verdict  passed (cargo exit 0)",
        "verdict  failed (cargo exit 101)",
        "not the one-shot client's exit code",
        "`0` when the tool answered",
        "`1` when it refused",
        "`2` on a usage error",
    ] {
        assert!(check.contains(expected), "missing `{expected}`: {check}");
    }
}

/// The exit code says how the **call** went — answered, refused, or a usage error — and never what
/// the tool found: `check` on a failing face still answers, so this client still exits `0` here.
/// 退出码说的是这次**调用**怎么样——作答、拒绝、或用法错——从不说工具发现了什么：`check` 在失败的面上
/// 仍是作答，因此这里客户端仍以 `0` 退出。
#[test]
fn the_client_exit_code_says_how_the_call_went() {
    let temp = std::env::temp_dir().join(format!("xirang-client-{}", std::process::id()));
    std::fs::create_dir_all(&temp).expect("scratch");
    let root = temp.display().to_string();
    let args = |parts: &[&str]| parts.iter().map(|p| (*p).to_owned()).collect::<Vec<_>>();

    match run_client(&args(&["--call", "xirang.status", "--root", &root])) {
        Client::Called(code) => assert_eq!(code, 0, "a tool that answers exits 0"),
        Client::Serve => panic!("--call is a call"),
    }
    match run_client(&args(&["--call", "xirang.no_such_tool"])) {
        Client::Called(code) => assert_eq!(code, 1, "a refusal is exit 1"),
        Client::Serve => panic!("--call is a call"),
    }
    match run_client(&args(&["--call"])) {
        Client::Called(code) => assert_eq!(code, 2, "a usage error is exit 2"),
        Client::Serve => panic!("--call is a call"),
    }
    let _ = std::fs::remove_dir_all(&temp);
}

/// Plain `--key value` arguments carry the obvious JSON type, so the common call needs no JSON.
/// 普通 `--key value` 参数带上最显然的 JSON 类型，因此常见调用不必写 JSON。
#[test]
fn plain_arguments_carry_their_json_type() {
    assert_eq!(scalar_for("source", "true"), Value::Bool(true));
    assert_eq!(scalar_for("source", "false"), Value::Bool(false));
    assert_eq!(scalar_for("timeout_ms", "900000"), Value::from(900_000));
    assert_eq!(scalar_for("face", "default"), Value::from("default"));
    assert_eq!(
        scalar_for("limit", "00"),
        Value::from("00"),
        "a leading zero is a string, not a number"
    );
}

/// Every call shape the flow table advertises is one the tool actually accepts.
/// 流程表承诺的每一种调用形状，都是工具真正接受的形状。
///
/// This is the pin the round was missing. The table advertised `callgraph {orphans: true}` while the
/// orphan view had deliberately been left out, and the old pin asserted only that the *string*
/// appeared in `GUIDANCE` — so two independent members spent two of a round's ten calls being
/// refused (`requires function`, then `needs a value`). Checking the sentence is not checking the
/// shape. The reverse direction is checked too: every `tool {keys}` shape the table names must be
/// listed here, so adding a promise without an acceptance check fails.
/// 这就是那轮缺失的钉子。流程表承诺了 `callgraph {orphans: true}`，而孤儿视图当时被我故意缓做，旧钉子只断言
/// 那个**字符串**出现在 `GUIDANCE` 里——于是两位成员各自花掉那轮 10 次调用里的 2 次被拒
/// （`requires function`、继而 `needs a value`）。**检查句子不等于检查形状。**反向也检查：表里点名的每个
/// `tool {keys}` 形状都必须在这里列名，因此"加了承诺却没有接受性检查"会失败。
#[test]
fn the_table_only_advertises_shapes_the_tool_accepts() {
    let root = crate::mcp::tools::tools_tests::scratch_package("instructions");
    // The read shapes, called for real: acceptance is a call that is not refused for its arguments.
    // 读形状真调一次：接受 = 这次调用不因为参数而被拒。
    let shapes: Vec<(&str, Value)> = vec![
        ("xirang.status", json!({})),
        ("xirang.registry", json!({})),
        ("xirang.explore", json!({"query": "used"})),
        ("xirang.search", json!({"query": "used"})),
        ("xirang.search", json!({"literal": "call_used"})),
        ("xirang.callgraph", json!({"function": "used"})),
        ("xirang.callgraph", json!({"orphans": true})),
        ("xirang.read", json!({"path": "src/lib.rs", "line": 1})),
        ("xirang.inspect", json!({"path": "src/lib.rs"})),
        ("xirang.affected", json!({"files": ["src/lib.rs"]})),
        ("xirang.explain", json!({})),
        ("xirang.diff", json!({})),
        // The two ledger shapes: a fixture with no ledger is still an answer (the absence is
        // reported), so acceptance here means the arguments were not refused.
        // 两种台账形状：没有台账的夹具仍然是一个答案（缺失会被报出），因此这里的接受指的是"参数没被拒"。
        ("xirang.conformance", json!({"anchor": "root/button"})),
        ("xirang.consistency", json!({"specimen": "root/button"})),
        ("xirang.consistency", json!({"parent": "root/control"})),
        // The shapes the seven-scenario table adds. Acceptance here means the arguments were not
        // refused — the table promises these spellings work, and the pin that reads this list is what
        // caught them missing: adding a promise without a check is exactly what it exists to refuse.
        // 七场景表新增的形状。这里的"接受"指参数没被拒——表承诺这些拼法可用，而读这份清单的那条钉子
        // 正是抓出它们缺失的那条：加了承诺却没加检查，正是它存在来拒绝的事。
        (
            "xirang.locate",
            json!({"symptom": "the offsets do not add up"}),
        ),
        ("xirang.why", json!({"at": "src/lib.rs:1"})),
        ("xirang.grafts", json!({})),
    ];
    for (tool, arguments) in &shapes {
        let answer = crate::mcp::tools::run_tool(&root, tool, arguments);
        assert!(answer.is_ok(), "{tool} {arguments} was refused: {answer:?}");
    }
    // The write shape is not called (a preview runs the real executor on a copy); its schema is what
    // has to match the table.
    // 写入形状不真调（预览会在副本上跑真执行器）；要对上那张表的是它的 schema。
    let apply = crate::mcp::tools::tools()
        .into_iter()
        .find(|tool| tool["name"] == "xirang.apply")
        .expect("apply is in the catalogue");
    assert_eq!(apply["inputSchema"]["required"][0], "action", "{apply}");
    let _ = std::fs::remove_dir_all(&root);
}

/// Each shape the table names has an acceptance check above, and each check names a shape the table
/// really has.
/// 表里点名的每种形状在上面的检查里都有，而每个检查点名的形状表里真有。
///
/// Drift runs both ways: a table entry with no check is a promise nobody tests, and a check for a
/// shape the table dropped is a test that outlived its subject.
/// 漂移是双向的：表里有、检查里没有 ⇒ 没人测的承诺；检查里有、表里已经删掉 ⇒ 活得比被检查对象还久的测试。
#[test]
fn every_shape_the_table_names_is_covered_by_an_acceptance_check() {
    let named = GUIDANCE
        .split('`')
        .filter(|part| part.contains('{') && part.contains('}'))
        .filter_map(|part| part.split_once(' '))
        .map(|(tool, keys)| (tool.to_owned(), keys.to_owned()))
        .collect::<Vec<_>>();
    assert!(named.len() >= 6, "the table names shapes: {named:?}");
    for (tool, keys) in &named {
        let checked = match tool.as_str() {
            // The table writes tool names with backticks around the call, e.g. "check {face}".
            // 表里的写法是"工具 {键}"，例如 `check {face}`。
            "check" => keys.contains("face"),
            "explore" => keys.contains("query"),
            // Registered-face acceptance is exercised against real faces in converge_tests.
            // 注册面的调用接受性由 converge_tests 使用真实注册面验证。
            "converge" => keys.contains("node"),
            "search" | "callgraph" | "read" | "inspect" | "affected" | "registry" | "explain"
            | "diff" | "locate" | "why" | "grafts" => true,
            // `new_project` writes a scaffold, so its schema is the check, the way `apply`'s is.
            // `new_project` 会写出脚手架，因此检查它的是 schema，与 `apply` 同理。
            "new_project" => keys.contains("directory"),
            "apply" => keys.contains("apply"),
            // The ledger pair: the table names `conformance {anchor}` and
            // `consistency --specimen <anchor>`, and both are called for real above.
            // 台账那一对：表里点名 `conformance {anchor}` 与 `consistency --specimen <anchor>`，两者都在
            // 上面真调过。
            "conformance" => keys.contains("anchor"),
            "consistency" => keys.contains("specimen") || keys.contains("parent"),
            other => panic!("the table names `{other}`, which no acceptance check covers"),
        };
        assert!(checked, "{tool} {keys}");
    }
}

/// `--root` decides which tree is answered, wherever the process happens to be standing.
/// `--root` 决定答的是哪棵树，无论进程此刻站在哪里。
///
/// Measured: before this, the flag was forwarded as an argument and resolved *inside* the caller's
/// own root, so one scenario round answered about a 475-file checkout instead of the workspace the
/// question named, and only the answer's own root line revealed it.
/// 量出来的：在此之前，该开关被当作参数转发、在调用方自己的根**内部**解析，于是某个情景轮答的是 475 个文件
/// 的检出而不是问题点名的工作区，而只有答案自己那行 root 暴露了这件事。
#[test]
fn the_root_flag_decides_which_tree_is_answered() {
    let alpha = crate::mcp::tools::tools_tests::scratch_package("root-alpha");
    let beta = crate::mcp::tools::tools_tests::scratch_package("root-beta");
    let args = vec![
        "xirang.status".to_owned(),
        "--root".to_owned(),
        alpha.display().to_string(),
    ];
    let answer = match call_from_arguments(&args) {
        Ok(answer) => answer,
        Err(Refusal::Tool(text)) | Err(Refusal::Usage(text)) => {
            panic!("the call must be accepted, got: {text}")
        }
    };
    assert!(
        answer.contains(&alpha.display().to_string()),
        "the answer names the tree the flag named: {answer}"
    );
    assert!(
        !answer.contains(&beta.display().to_string()),
        "and not another one: {answer}"
    );
    let _ = std::fs::remove_dir_all(&alpha);
    let _ = std::fs::remove_dir_all(&beta);
}

/// A boolean is a flag: the bare spelling is accepted and means true.
/// 布尔就是开关：裸写被接受，且意为 true。
/// A text key keeps digits as text: the round could not express `--literal 1000`.
/// 文本键把数字串留作文本：那轮无法表达 `--literal 1000`。
#[test]
fn a_text_key_keeps_digits_as_text() {
    assert_eq!(scalar_for("literal", "1000"), Value::from("1000"));
    assert_eq!(scalar_for("query", "0"), Value::from("0"));
    assert_eq!(scalar_for("context", "4"), Value::from(4));
}

/// The table names the adoption ledger, because the round measured the cost of not naming it: both
/// arms had to re-derive the ledger's policy (`no renewal of unchanged bytes, one appended line for
/// a new route`) from the kernel instead of being told where the ledger is and what it means.
/// 流程表点名采信台账，因为"不点名"的代价被量到过：两臂都得从内核里重新推出台账策略（"未变的字节不续期、
/// 新路线追加一行"），而不是被告知台账在哪、它是什么意思。
#[test]
fn the_table_names_the_adoption_ledger() {
    assert!(
        super::GUIDANCE.contains("adopted"),
        "the ledger tool is named"
    );
    assert!(
        super::GUIDANCE.contains(".xirang/adopted/entries"),
        "and so is where it lives"
    );
}

#[test]
fn a_bare_flag_means_true() {
    let temp = std::env::temp_dir().join(format!("xirang-flag-{}", std::process::id()));
    std::fs::create_dir_all(&temp).expect("scratch");
    let root = temp.display().to_string();
    for parts in [
        vec!["xirang.search", "--query", "x", "--converge"],
        vec!["xirang.search", "--query", "x", "--converge", "true"],
    ] {
        let mut args = vec!["--call".to_owned()];
        args.extend(parts.iter().map(|part| (*part).to_owned()));
        args.extend(["--root".to_owned(), root.clone()]);
        match run_client(&args) {
            Client::Called(code) => assert_eq!(code, 0, "bare flag accepted: {parts:?}"),
            Client::Serve => panic!("--call is a call"),
        }
    }
    let _ = std::fs::remove_dir_all(&temp);
}

/// `--log <file>` records one JSON line per call — `{request, response, exit}` — so a measured run
/// needs no wrapper script around every call (the round paid one extra step per instrument call for
/// exactly that wrapper).
/// `--log <文件>` 每次调用记一行 JSON —— `{request, response, exit}` —— 因此被测量的运行不必为每次
/// 调用套包装脚本（那一轮正是为这个包装，每次仪器调用多花一步）。
#[test]
fn a_log_flag_records_one_line_that_mirrors_the_call() {
    let temp = std::env::temp_dir().join(format!("mcp-log-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&temp);
    std::fs::create_dir_all(&temp).expect("temp directory");
    let log = temp.join("calls.jsonl");

    let args: Vec<String> = [
        "--call",
        "callgraph",
        "--root",
        temp.to_str().expect("path"),
        "--log",
        log.to_str().expect("path"),
    ]
    .iter()
    .map(|part| (*part).to_owned())
    .collect();
    let code = match run_client(&args) {
        Client::Called(code) => code,
        Client::Serve => panic!("--call is a call"),
    };

    let text = std::fs::read_to_string(&log).expect("the log file exists");
    assert_eq!(text.lines().count(), 1, "one line per call: {text}");
    let line: serde_json::Value =
        serde_json::from_str(text.lines().next().expect("a line")).expect("the line is JSON");
    assert_eq!(line["exit"].as_i64(), Some(code.into()), "{line}");
    assert!(
        line["request"]
            .as_array()
            .is_some_and(|request| request.len() >= 4),
        "the request is the argv: {line}"
    );
    assert!(
        line["response"]
            .as_str()
            .is_some_and(|body| !body.is_empty()),
        "the response is the answer body: {line}"
    );

    let _ = std::fs::remove_dir_all(&temp);
}

/// The flow table places a request in one of seven shapes, and every shape carries a stop condition.
/// 流程表把请求放进七种形状之一，而每种形状都带一个停止条件。
///
/// Measured need (the maintainer's own words): a flat tool list with no packaged workflow makes the
/// model rebuild the route from scratch on every question — and the numbers agree (`next` was
/// followed 28% of the time, and one question spent 14 of its 16 calls before the fact that decided
/// it). The signals are asserted to be **structural**, not words, because a table keyed on wording
/// would be the overfitting the same rules forbid.
/// 量出来的需求（维护者原话）：工具平铺、没有打包的工作流 ⇒ 模型每个问题都要把路线重建一遍——数与之一致
/// （`next` 被采纳 28%，而有一道题把 16 次调用里的 14 次花在"决定性事实"之前）。这里断言信号是**结构性**
/// 的而不是词，因为按措辞索引的表正是同一套规则禁止的那种过拟合。
#[test]
fn the_flow_table_packages_the_seven_shapes_with_their_stop_conditions() {
    // One row per shape: the signal as the table spells it, and the stop condition it must carry.
    // 每种形状一行：表里写的信号，以及它必须携带的停止条件。
    let rows = [
        ("empty tree", "stop when the skeleton"),
        ("one object named", "stop at a root cause"),
        (
            "several named",
            "stop when every named object has a verdict",
        ),
        ("a range question", "stop when each column is"),
        ("add an object", "stop when that line names no outlier"),
        (
            "deepen an object",
            "stop when the tree, the public paths and the factory pins",
        ),
        ("move or merge", "stop when every layer's impact"),
    ];
    for (signal, stop) in rows {
        assert!(
            GUIDANCE.contains(signal),
            "the table names the signal `{signal}`: {GUIDANCE}"
        );
        assert!(
            GUIDANCE.contains(stop),
            "and its stop condition `{stop}`: {GUIDANCE}"
        );
    }
    assert!(
        GUIDANCE.contains("not by its wording"),
        "the signals are structural, not words: {GUIDANCE}"
    );
    assert!(
        GUIDANCE.contains("停止条件"),
        "and the Chinese half carries the same table: {GUIDANCE}"
    );
}

/// The scenario fixture covers every shape with variants, and it cannot drift from the flow table.
/// 场景夹具覆盖每个形状且带多种措辞，并且它不会与流程表漂移。
///
/// **What this pin does not do**: it does not claim the model picks the right shape for a wording.
/// That is behaviour, measured only in the run this fixture feeds (T-21) — asserting it here would be
/// a green nothing earned. What is decidable is the fixture's own contract: seven shapes, at least
/// four wordings and one discriminating pair each, the shapes named here are exactly the shapes the
/// table names, and every row carries the three structural signals.
/// **这条钉子不做什么**：它不声称"模型会为某种措辞选对形状"——那是行为，只在这个夹具所喂的那一轮大测试里量
/// （T-21）；在这里断言它就是白拿一个绿。可判定的是夹具自己的契约：七个形状、每个至少四条措辞加一个判别对、
/// 这里的形状名与表里的**逐字相同**、且每行都带那三个结构信号。
#[test]
fn the_scenario_fixture_covers_every_shape_and_cannot_drift_from_the_table() {
    const CASES: &str = include_str!("../../../docs/design-scenario-cases.md");
    let shapes = [
        "S1 空树",
        "S2 范围型",
        "S3 查看单对象",
        "S4 新增对象",
        "S5 查看多个对象",
        "S6 加深一个对象",
        "S7 迁移/合并（重构）",
    ];
    for shape in shapes {
        assert!(
            CASES.contains(shape),
            "the fixture covers `{shape}`: it is one of the table's seven"
        );
        let section = CASES
            .split(&format!("## {shape}"))
            .nth(1)
            .unwrap_or_default()
            .split("\n## ")
            .next()
            .unwrap_or_default();
        let rows = section
            .lines()
            .filter(|line| {
                line.starts_with("| ") && !line.contains("---") && !line.contains("说法")
            })
            .count();
        assert!(
            rows >= 5,
            "`{shape}` carries four wordings and one discriminating pair, found {rows} rows"
        );
        assert!(
            section.contains("**判别对**"),
            "and one of them is the pair that separates it from its neighbour: `{shape}`"
        );
        for signal in ["| 点名 | 方向 | 树 |", "停止条件"] {
            assert!(
                section.contains(signal),
                "`{shape}` states `{signal}`: the signals are structural and the stop is a fact"
            );
        }
    }
    // Both ways: a shape the fixture dropped, and a shape the table added, are both drift.
    // 双向：夹具丢掉的形状与表里新加的形状都是漂移。
    for row in GUIDANCE.split("  ").filter(|part| part.contains(" -> ")) {
        let named = row
            .split_whitespace()
            .take_while(|word| !word.contains("->"))
            .collect::<Vec<_>>()
            .join(" ");
        if named.is_empty() {
            continue;
        }
        let covered = match named.as_str() {
            "empty tree" => "S1 空树",
            "inspect, a range question" => "S2 范围型",
            "inspect, one object named" => "S3 查看单对象",
            "add an object" => "S4 新增对象",
            "inspect, several named" => "S5 查看多个对象",
            "deepen an object" => "S6 加深一个对象",
            "move or merge (refactor)" => "S7 迁移/合并（重构）",
            other => panic!("the table names a shape the fixture does not: `{other}`"),
        };
        assert!(CASES.contains(covered), "{covered}");
    }
}

/// The table tells an agent where a parent's rules live, because the failure arrives after the edit.
/// 表里告诉代理父面的规范住在哪——因为那次失败是在改动**之后**才到的。
///
/// Measured (T-14): the mechanism was complete and host-declarable (`<parent>/registry_rule/
/// registry_rule.rs`, enforced at build time with the missing requirement named — verified by
/// declaring a rule and watching `spinner` fail with "required handle trait is missing"), and **no
/// answer ever mentioned it**. A capability whose existence is only discoverable by breaking it is
/// the same "entrance was not there" shape the whole round is about.
/// 量到的（T-14）：机制完整且宿主可声明（`<父>/registry_rule/registry_rule.rs`，建树时判定并点名缺哪条——实测：
/// 声明一条规则后 `spinner` 以 "required handle trait is missing" 失败），而**没有任何答案提过它**。一项只有
/// 破坏它才会被发现的能力，正是整轮在讲的那个"入口不在"的形状。
#[test]
fn the_table_says_where_a_parents_rules_live() {
    for expected in [
        "registry_rule/registry_rule.rs",
        "REGISTRATION_RULE",
        "fails the build naming the missing requirement",
        "父面自己的规范",
    ] {
        assert!(
            GUIDANCE.contains(expected),
            "the table carries `{expected}`: {GUIDANCE}"
        );
    }
}

/// The table points from a one-object question at the family comparison, because that is the call
/// that answers it.
/// 表从一个"单个对象"的问题**指向**族比较——因为那才是回答它的调用。
///
/// Measured (T-21, h1): the arm read four sibling files in a row to work out that eight siblings add
/// `to_local(7)` and one adds `to_world(31)`. `consistency --parent root/control` answers exactly
/// that **in one call** — it prints each sibling's own calls and names the outlier — and it was in
/// the table already, under the *several-named* row. The capability was there; the pointer from the
/// one-object row was not, which is the same shape as the parent-rule channel.
/// 量到的（T-21 的 h1）：那一臂连着读了四个兄弟文件才弄清八个兄弟加 `to_local(7)`、一个加
/// `to_world(31)`。`consistency --parent root/control` **一次**就答这个——它印出每个兄弟各自的调用并点名异类
/// ——而它本来就在表里，只是挂在"点名多个"那一行。**能力在，从"单个对象"那一行过去的指路不在**——与父面规则
/// 那条通道是同一个形状。
#[test]
fn the_table_points_a_family_question_at_the_family_comparison() {
    for expected in [
        "(an object in a family)",
        "consistency --parent <its parent>",
        "names the outlier",
        "对象有兄弟时",
    ] {
        assert!(
            GUIDANCE.contains(expected),
            "the table carries `{expected}`: {GUIDANCE}"
        );
    }
}

/// The table says that independent lookups may share one shell step — because a turn is the unit that
/// costs.
/// 表里写明"彼此独立的查询可以共用一条 shell 命令"——因为**轮**才是花钱的单位。
///
/// Measured (T-21): cache reads were 10,378,112 tokens against 142,869 of output, so a turn costs
/// roughly 87,000 cached tokens while shortening every answer by a whole line saves hundreds. The
/// same round's logs show 25 of 73 calls were adjacent lookups of the same kind — about a third of
/// the turns, and the largest lever left that is not the apparatus itself.
/// 量到的（T-21）：缓存读 10,378,112 token、输出 142,869 ⇒ 一轮约 87,000 个缓存 token，而把每条答案砍掉一整行
/// 只省几百。同一轮的日志里 73 次调用有 25 次是相邻的同类查询——约三分之一轮数，也是除装置本身之外最大的杠杆。
#[test]
fn the_table_lets_independent_lookups_share_a_turn() {
    for expected in [
        "(independent lookups)",
        "may share **one**",
        "what that saves is a **turn**",
        "彼此独立的查询",
        "只在`下一个参数不依赖上一个答案`时这么做",
    ] {
        assert!(
            GUIDANCE.contains(expected),
            "the table carries `{expected}`: {GUIDANCE}"
        );
    }
}

/// The table's pointer to a parent's rule uses `literal`, because `query` matches names.
/// 表里指向父面规则的那句用 `literal`，因为 `query` 只匹配**名字**。
///
/// Measured (T-21, h2): the table said `search {query: "REGISTRATION_RULE"}` and the arm called
/// exactly that — `query` matches faces, files and functions, so a `const` is not a name it knows and
/// the reply was `no matches`. The reader then read the file by hand. `literal` matches text anywhere
/// (comments and string literals included) and **its reply carries the way to the rest**
/// (`note   pass context: 2 to print the lines around each hit here`), which is what the requirement
/// list on the next line needed.
/// 量到的（T-21 的 h2）：表里写的是 `search {query: "REGISTRATION_RULE"}`，那一臂就照着调了——而 `query`
/// 匹配的是面、文件与函数，`const` 不是它认识的名字，回复是 `no matches`；读者随后手工 `read` 了那个文件。
/// `literal` 匹配任意文本（含注释与字符串字面量），并且**它的回复自带"剩下的在哪"**
/// （`note   pass context: 2 to print the lines around each hit here`）——而下一行那句 `require_exports` 正是
/// 读者要的东西。
#[test]
fn the_pointer_to_a_parents_rule_uses_the_spelling_that_finds_it() {
    // Only the instructions are asserted here: `pass literal` is what *search's reply* says, and it is
    // pinned where that reply is built.
    // 这里只断言指引页：`pass literal` 是 **search 的回复**里的话，钉在生成那条回复的地方。
    assert!(
        GUIDANCE.contains("search {literal: \"REGISTRATION_RULE\"}"),
        "the pointer uses the spelling that finds a const: {GUIDANCE}"
    );
    assert!(
        !GUIDANCE.contains("search {query: \"REGISTRATION_RULE\"}"),
        "the name-shaped spelling cannot find a const: {GUIDANCE}"
    );
}

/// The `keys:` legend appears only when something is actually starred.
/// `keys:` 的图例只在真的画了星号时出现。
///
/// Measured (round-8 review): once `conformance` stopped requiring `anchor`, its line read
/// `keys: anchor, root (* = required)` — a legend explaining a mark it does not use, which a reader
/// has to undo before trusting the line. Both renderers (the full page and the one-line catalogue) had
/// the literal hard-coded.
/// 量到的（第八轮复核）：`conformance` 不再要求 `anchor` 之后，它那一行成了
/// `keys: anchor, root (* = required)`——解释一个自己没有使用的星号，读者得先撤销它才敢信这一行。两处渲染
/// （完整页与单行目录）都把这个图例写死了。
#[test]
fn the_keys_legend_appears_only_when_something_is_starred() {
    let lines = list_tool_lines();
    let line = |name: &str| {
        lines
            .iter()
            .find(|line| {
                line.starts_with(&format!("{name} ")) || line.starts_with(&format!("{name} —"))
            })
            .unwrap_or_else(|| panic!("{name} is in the catalogue: {lines:?}"))
            .clone()
    };
    let conformance = line("xirang.conformance");
    assert!(
        !conformance.contains("= required"),
        "nothing is starred on this tool, so there is no legend: {conformance}"
    );
    // `check` used to be the example here; it no longer stars `face` because the call path defaults
    // it to `default` (the round measured 12 face-less calls, 6 of which left the tool for a shell).
    // The example moves to a tool whose keys really are required, so this pin still has teeth — a
    // version that lost the legend everywhere would clear the assertion above and fail here.
    // `check` 从前是这里的例子；它不再给 `face` 打星，因为调用路径把它缺省成 `default`（那一轮量到 12 次
    // 无 face 的调用，其中 6 次离开工具去了 shell）。例子换成**真的必需**键的工具，这条钉子因此仍有牙 ——
    // 一个"到处都丢了图例"的版本会通过上面那条、在这里失败。
    let apply = line("xirang.apply");
    assert!(
        apply.contains("action*") && apply.contains("= required"),
        "and a tool that does star a key keeps its legend: {apply}"
    );
}

/// The table says a path claim is read at each hop's definition.
/// 表里写明：路径断言要在每一跳的定义处读出来。
///
/// Measured (round 8): two answers claimed a route that the sources contradict — one said a
/// `buckets.rs` value arrives through `postable` when it arrives through the constructor, another
/// asserted an inconsistency the implementation does not have. Both are the same failure: a hop
/// inferred from a name instead of read at its definition.
/// 量到的（第八轮）：两条答案断言了源码并不支持的路径——一条说 `buckets.rs` 的值经 `postable` 到达，实际
/// 经构造函数；另一条断言了一处实现里并不存在的不一致。同一个失误：跳数由**名字**推断，而不是在定义处读。
#[test]
fn the_table_says_a_path_claim_is_read_at_each_hop() {
    assert!(
        GUIDANCE.contains("read at each hop's own definition"),
        "the rule is on the page every agent reads: {GUIDANCE}"
    );
    assert!(
        GUIDANCE.contains("a name that looks like the hop is not the hop"),
        "and it says why naming is not enough: {GUIDANCE}"
    );
}

/// A command-line value that spells a JSON array or object is read as one.
/// 命令行上拼成 JSON 数组或对象的值，按它读。
///
/// The round measured the cost of the alternative three times over: `--files a` arrived as a string
/// (a false "a renewal requires `files`"), a repeated `--files a --files b` reached the ledger as
/// `path,path`, and `--function '["a","b"]'` was searched for as one symbol literally named
/// `["a","b"]`. A scalar that merely parses as JSON keeps the scalar path.
/// 那一轮的代价量到过三次：`--files a` 以字符串到达（假拒绝"a renewal requires `files`"）、重复的
/// `--files a --files b` 落到台账里是 `path,path`、`--function '["a","b"]'` 被当成一个字面名叫
/// `["a","b"]` 的符号去查。恰好能解析成 JSON 的标量仍走标量那条路。
#[test]
fn a_value_that_spells_a_json_array_is_read_as_one() {
    // Test the reader itself, not someone else's output shape: the first version of this pin
    // assumed `call_from_arguments` returns JSON and asserted on a string it does not produce.
    // 测读取器本身，不测别人的输出形状：这条钉子的第一版假设 `call_from_arguments` 返回 JSON，
    // 结果断言了一个它并不产生的字符串。
    assert_eq!(
        json_container("[\"a\",\"b\"]"),
        Some(serde_json::json!(["a", "b"]))
    );
    assert_eq!(
        json_container("{\"a\":1}"),
        Some(serde_json::json!({"a": 1}))
    );
    // Scalars keep the scalar path even when they parse as JSON.
    // 标量即使能解析成 JSON 也仍走标量那条路。
    for scalar in ["7", "true", "null", "\"quoted\""] {
        assert_eq!(json_container(scalar), None, "{scalar}");
    }
    // A value that merely starts like a container is still a string.
    // 只是开头像容器的值仍然是字符串。
    for text in ["[not json", "{", "plain", ""] {
        assert_eq!(json_container(text), None, "{text}");
    }
}

/// `check` says the face may be omitted, because it runs `default` when it is.
/// `check` 说面可以省略，因为省略时它跑 `default`。
///
/// Three members of the round-11 verification independently reported the drift this pins: the tool's
/// behaviour was changed (a missing `face` runs `default`, because 6 of 12 face-less calls left the
/// tool for a shell) while two descriptions still said "there is no default face here" and the
/// schema still marked `face` required — so the `keys:` line printed `face*` to every reader.
/// 第十一轮验证里有**三位成员独立报出**这条钉子守的漂移：工具的行为改了（缺 `face` 跑 `default`，
/// 因为 12 次无 `face` 的调用里有 6 次离开工具去了 shell），而两处描述仍写着 "there is no default
/// face here"、schema 仍把 `face` 标为必需 —— 于是 `keys:` 行给每个读者印的是 `face*`。
#[test]
fn check_says_the_face_may_be_omitted_because_it_is() {
    let described = describe_tool("check").expect("a description");
    assert!(
        !described.contains("there is no default face"),
        "the refusal wording is gone from the description: {described}"
    );
    assert!(
        described.contains("may be omitted"),
        "the description has to say the face may be omitted: {described}"
    );
    // And the keys line must not mark it required.
    // 而 keys 行不许把它标成必需。
    let line = list_tool_lines()
        .into_iter()
        .find(|row| row.contains("xirang.check"))
        .expect("check is listed");
    assert!(
        !line.contains("face*"),
        "`face` is not required, so the keys line must not star it: {line}"
    );
}

/// A dotted flag is the command line's spelling of one field inside an object-valued
/// argument, so it must reach the tool as that object: `--fields.module button` and
/// `--fields '{"module":"button"}'` are the same request. Leaving it as a key literally
/// named `fields.module` made the tool see no `fields` object and refuse — the round-13
/// benchmark measured a fresh agent's first `apply add` eating three refusals on exactly
/// that shape.
/// 点号开关是命令行写"对象取值里的一个字段"的拼法，因此它必须以那个对象的形式到达工具：
/// `--fields.module button` 与 `--fields '{"module":"button"}'` 是同一个请求。把它留成一个字面
/// 名叫 `fields.module` 的键，会让工具看不到 `fields` 对象、直接拒绝——第十三轮量到的新代理第一次
/// `apply add` 正是为这个形状连吃三次拒绝。
#[test]
fn a_dotted_flag_folds_into_the_object_it_names() {
    let mut object = serde_json::Map::new();
    object.insert("fields.module".to_owned(), json!("button"));
    object.insert("fields.kind".to_owned(), json!("Object"));
    fold_dotted_keys(&mut object);
    assert_eq!(
        object.get("fields"),
        Some(&json!({"module": "button", "kind": "Object"})),
        "{object:?}"
    );
    // The reverse direction: the dotted key must not survive beside the object it folded
    // into, because a lingering one is what the tool used to be handed.
    // 反向：点号键不许与它折进的那个对象并存在一起，因为工具过去接到的正是那个残留的键。
    assert!(!object.contains_key("fields.module"), "{object:?}");

    // Two levels nest, and a repeated flag merges into an array exactly as a repeated plain
    // flag does — that merge already happens before this fold, so the array is what arrives.
    // 两层会嵌起来，而重复的开关与重复的普通开关一样合并成数组——那次合并在本折叠之前就发生了，
    // 因此到达这里的就是那个数组。
    let mut nested = serde_json::Map::new();
    nested.insert("inside.parts.width".to_owned(), json!("u32"));
    nested.insert("files.a".to_owned(), json!(["x", "y"]));
    fold_dotted_keys(&mut nested);
    assert_eq!(
        nested.get("inside"),
        Some(&json!({"parts": {"width": "u32"}})),
        "{nested:?}"
    );
    assert_eq!(
        nested.get("files"),
        Some(&json!({"a": ["x", "y"]})),
        "{nested:?}"
    );

    // A head that is already a scalar cannot hold a tail: both spellings stay as written so
    // the tool reports one of them rather than this fold silently dropping the other.
    // 头已经是个标量时装不下尾：两种拼法都按原样留着，让工具报出其中一个，而不是让本折叠静默丢掉另一个。
    let mut conflicting = serde_json::Map::new();
    conflicting.insert("fields".to_owned(), json!("scalar"));
    conflicting.insert("fields.module".to_owned(), json!("button"));
    fold_dotted_keys(&mut conflicting);
    assert_eq!(
        conflicting.get("fields"),
        Some(&json!("scalar")),
        "{conflicting:?}"
    );
    assert_eq!(
        conflicting.get("fields.module"),
        Some(&json!("button")),
        "a conflicting pair is left for the tool to refuse, not resolved here: {conflicting:?}"
    );
}

/// The two standard spellings of one pair, and the one thing that must not be split: only the
/// key side. `--fields.module=button` used to arrive as a field named `module=button` (round-13
/// measured a fresh agent trying exactly that), while `--query a=b` must keep its `=`.
/// 同一个键值对的两条标准拼法，以及唯一不许被拆的地方：只拆键那一侧。`--fields.module=button`
/// 过去会到达成一个名叫 `module=button` 的字段（第十三轮量到一个新代理正是这么试的），而
/// `--query a=b` 必须保住它的 `=`。
#[test]
fn a_flag_is_split_at_its_first_equals_only() {
    assert_eq!(
        flag_pair("--fields.module=button"),
        Some(("fields.module".to_owned(), Some("button".to_owned())))
    );
    assert_eq!(
        flag_pair("--fields.module"),
        Some(("fields.module".to_owned(), None)),
        "a spaced pair keeps the value for the next token"
    );
    assert_eq!(
        flag_pair("--query=a=b"),
        Some(("query".to_owned(), Some("a=b".to_owned()))),
        "only the first `=` splits, so a value with `=` survives"
    );
    assert_eq!(flag_pair("--"), None);
    assert_eq!(flag_pair("--=x"), None);
    assert_eq!(flag_pair("face"), None);
}

/// The answer-shape routing is in the shipped text, on both the short page and the long one.
/// 答案形状的路由写在**出厂文本**里——短页与长页都有。
///
/// Audit `W4-7`: the two question shapes need different answers (an understanding question is
/// answered by dumping the file, a structural one by the bounded answer), and a reader who does not
/// know that asks the wrong tool and pays for the round trip. A rule that only lives in a design
/// document is a rule that is not in the product.
/// 审计 `W4-7`：两种问题形状要两种答案（理解型由"把文件倒出来"回答，结构型由有界答案回答），而不知道
/// 这一点的读者会去问错的工具、并为此付一个往返。只活在设计文档里的规则，就是不在产物里的规则。
#[test]
fn the_answer_shape_routing_is_on_both_pages() {
    // The routing is on the two pages that carry prose (the `--list` short page and the long one),
    // and the MCP handshake uses JSON argument syntax for the same call.
    // 路由在两张带散文的页上；MCP 握手用同一调用的 JSON 参数语法。
    assert!(INSTRUCTIONS.contains("read {path,whole:true}"));
    for page in [SHAPES_SHORT, GUIDANCE] {
        assert!(
            page.contains("read --whole"),
            "the shipped text names the whole-file call"
        );
    }
    for page in [SHAPES_SHORT, GUIDANCE] {
        assert!(
            page.contains("Answer shape"),
            "the English routing half is on both prose pages"
        );
        assert!(
            page.contains("答案的形状") || page.contains("答案形状"),
            "and so is the Chinese half"
        );
    }
    // And the tool that raises the question routes it itself, with the file it ranked first.
    // 而提出这个问题的工具自己会路由它，并用它排第一的那个文件实例化。
    let root = crate::mcp::tools::tools_tests::scratch_package("routing-locate");
    let answer = crate::mcp::locate::locate(&root, &serde_json::json!({"symptom": "used"}))
        .expect("a ranked answer");
    assert!(
        answer.contains("understanding question") && answer.contains("--whole"),
        "locate points at the whole-file call: {answer}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// The catalogue tool is reachable by the name the advertisement uses.
/// 目录工具按广告里那个名字够得着。
///
/// Audit `W1-1` left `xirang_tools` advertised and the CLI could not call it: every bare name got
/// the `xirang.` prefix, so `--call xirang_tools` resolved to `xirang.xirang_tools` and the
/// one tool the handshake tells a session to reach everything else with answered `unknown tool`.
/// The rule is now "an exact catalogue name wins; the prefix is added only when it is needed", and
/// both halves are pinned — the new name resolves, and the old convenience still does.
/// 审计 `W1-1` 把 `xirang_tools` 广告出去了，而命令行调不到它：每个裸名都被补上 `xirang.` 前缀，
/// 于是 `--call xirang_tools` 解析成 `xirang.xirang_tools`，而"握手让会话用它去够其余一切"的那个
/// 工具回的是 `unknown tool`。现在的规则是"精确命中目录名者胜，只在需要时才补前缀"，两半都钉住——新名字
/// 解析得到，旧的便利也还在。
#[test]
fn the_catalogue_tool_resolves_by_its_advertised_name() {
    assert_eq!(super::resolve_name("xirang_tools"), "xirang_tools");
    assert_eq!(super::resolve_name("callgraph"), "xirang.callgraph");
    assert_eq!(super::resolve_name("xirang.why"), "xirang.why");
    assert_eq!(super::resolve_name("why"), "xirang.why");
    let root = crate::mcp::tools::tools_tests::scratch_package("catalogue-name");
    let listed = super::call_tool(&root, "xirang_tools", &serde_json::json!({}))
        .expect("the catalogue answers by its advertised name");
    assert!(
        listed.contains("xirang.registry — "),
        "and it lists every tool: {listed}"
    );
    let one = super::call_tool(&root, "xirang_tools", &serde_json::json!({"tool": "why"}))
        .expect("one tool's page");
    assert!(one.contains("Gather the upstream facts"), "{one}");
    let unknown = super::call_tool(
        &root,
        "xirang_tools",
        &serde_json::json!({"tool": "no_such_tool"}),
    )
    .expect_err("an unknown name is refused, not invented");
    assert!(unknown.contains("not a tool this bridge has"), "{unknown}");
    let _ = std::fs::remove_dir_all(&root);
}

/// The discovery page is bounded **and** the manual is one flag away, for every tool.
/// 发现页有界，而手册离一个开关，对每个工具都成立。
///
/// Audit `W2-6` resolved a pair of entries that read as contradictory (`W1-4`: the long text moves
/// into `--list`; `W2-6`: the discovery payload halves). The resolution is that the page is bounded
/// by default and unbounded on request, and this pin holds both halves at once over the **whole**
/// catalogue: the default never grows the text, the ratio holds wherever there was something to
/// withhold, and every truncation names the call that prints the rest.
/// 审计 `W2-6` 解开了一对读起来矛盾的条目（`W1-4`：长文迁入 `--list`；`W2-6`：发现载荷减半）。解法是
/// "默认有界、按需无界"，而这条钉子在**整份**目录上同时钉两半：默认页从不把文本变长、在真有东西可扣的
/// 地方比例成立、每次截断都点名印出其余部分的那次调用。
#[test]
fn every_discovery_page_is_bounded_with_a_way_to_the_whole_text() {
    let mut truncated = 0usize;
    for entry in crate::mcp::tools::tools() {
        let name = entry["name"].as_str().expect("a name");
        let short = describe_tool(name).expect("a default page");
        let full = describe_tool_fully(name).expect("the whole page");
        assert!(
            short.len() <= full.len() + 1,
            "`{name}`: the default page is not the longer one ({} vs {})",
            short.len(),
            full.len()
        );
        if short == full {
            // The rule is "bounded only when bounding is worth a call": a description that just
            // exceeds the budget is printed whole rather than shortened by a tenth at the price of
            // a second call. Such a tool is `continue`d here, and the count below keeps the ratio
            // from passing vacuously.
            // 规则是"只有收窄值一次调用时才收窄"：刚过预算的描述整段印出，而不是短十分之一、却要再花一次
            // 调用。这样的工具在这里跳过，而下面的计数保证比例不是空过。
            continue;
        }
        truncated += 1;
        assert!(
            (short.len() as f64) <= (full.len() as f64) * 0.6,
            "`{name}`: the discovery page is ≤60% of the manual ({} vs {})",
            short.len(),
            full.len()
        );
        assert!(
            short.contains("--full") && short.contains("withheld at"),
            "`{name}`: the truncation names its way out: {short}"
        );
        assert!(
            !full.contains("withheld at"),
            "`{name}`: the manual withholds nothing"
        );
    }
    assert!(
        truncated >= 3,
        "the ratio is exercised, not vacuous: only {truncated} tool(s) were bounded"
    );
}

/// The common-tool view is one call, is small, and says where the rest are.
/// 常用工具的视图是一次调用、很小，并且说出其余的在哪儿。
///
/// Audit `W2-7`: a session's second instrument should not cost the whole catalogue again. The view
/// is a selection over the **same** short-line implementation, so it cannot disagree with `--list`
/// about a tool they both print.
/// 审计 `W2-7`：一个会话的第二件仪器不该再花掉整份目录。这个视图是**同一份**短行实现上的选择，因此
/// 它与 `--list` 不可能对两者都印的同一个工具各说各话。
#[test]
fn the_common_tool_view_is_one_small_call() {
    let page = super::slim_tools_page();
    assert!(
        page.len() <= 3250,
        "the slim view is a screenful: {} bytes",
        page.len()
    );
    assert_eq!(
        super::COMMON_TOOLS.len(),
        14,
        "the workflow entry tools include read-first exploration"
    );
    for name in super::COMMON_TOOLS {
        assert!(
            page.contains(&format!("{name} — ")),
            "`{name}` is in the view"
        );
    }
    assert!(
        page.contains("lists them all"),
        "and the page says where the other tools are: {page}"
    );
    // The same lines `--list` prints, for the tools both show: one implementation, two selections.
    // 两者都显示的工具，行文与 `--list` 一致：一份实现，两种选择。
    let all = list_tool_lines();
    for name in super::COMMON_TOOLS {
        let from_all = all
            .iter()
            .find(|line| line.starts_with(&format!("{name} — ")))
            .expect("`--list` has every tool");
        assert!(page.contains(from_all.as_str()), "{from_all}");
    }
}
