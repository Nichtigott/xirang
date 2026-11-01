//! Pins for `xirang.why`: it gathers the upstream facts in one call, and it names what it left out.
//! `xirang.why` 的钉子：一次收齐上游事实，并点名它没答的部分。
//!
//! The measured failure these exist against is the hop count: a symptom sent agents to `callgraph`,
//! then to `read` for the contract, then to the registry — 5, 6, 5 and 8 instrument calls against the
//! control's 5, 2, 2 and 2 on the four injected-defect questions. Each pin here is one of the facts
//! that used to cost its own call.
//! 这些钉子针对的失败是跳数：一个症状把代理依次送去 `callgraph`、`read`（看契约）、注册面 —— 四道注入
//! 缺陷因而花了 5、6、5、8 次仪器调用，而对照是 5、2、2、2。这里每条钉子都是过去各自要花一次调用的
//! 事实之一。

use serde_json::json;

/// The scratch package the other tool tests use.
/// 与其它工具测试同一个临时包。
fn scratch(label: &str) -> std::path::PathBuf {
    crate::mcp::tools::tools_tests::scratch_package(label)
}

/// A `path:line` inside a definition answers with its contract, its callers, and what it left out.
/// 定义内的 `路径:行号` 会答出契约、调用者，以及它没答的部分。
#[test]
fn a_line_answers_with_contract_callers_and_bounds() {
    let root = scratch("why-line");
    let answer = super::why(&root, &json!({"at": "src/lib.rs:3"})).expect("an answer");
    assert!(
        answer.contains("the definition `used`"),
        "the line is placed in its definition: {answer}"
    );
    assert!(
        answer.contains("contract") && answer.contains("What this function is for."),
        "the contract rides along: {answer}"
    );
    assert!(
        answer.contains("call_used") && answer.contains("callers"),
        "the callers ride along: {answer}"
    );
    assert!(
        answer.contains("not covered here") && answer.contains("check {face}"),
        "the bounds name the tools that answer the rest: {answer}"
    );
}

/// A file with no ledger says so rather than staying silent about adoption.
/// 没有台账的文件会说出来，而不是对采信保持沉默。
#[test]
fn adoption_is_answered_even_when_there_is_no_ledger() {
    let root = scratch("why-ledger");
    let answer = super::why(&root, &json!({"at": "src/lib.rs:3"})).expect("an answer");
    assert!(
        answer.contains("no ledger at .xirang/adopted/entries"),
        "the absence is stated: {answer}"
    );
}

/// A malformed `at` is refused **with the accepted shape**, and a node question is redirected.
/// 形状不对的 `at` 被拒绝并**带上可接受形状**；节点类问题被指到 `explain`。
#[test]
fn a_malformed_at_is_refused_with_the_shape() {
    let root = scratch("why-shape");
    let shape = super::why(&root, &json!({"at": "src/lib.rs"})).expect_err("a line is required");
    assert!(
        shape.contains("`path:line`") && shape.contains("accepted shape"),
        "the refusal carries the shape: {shape}"
    );
    let node = super::why(&root, &json!({})).expect_err("`at` is required");
    assert!(
        node.contains("explain {node}") && node.contains("accepted shape"),
        "a node question is redirected: {node}"
    );
}

/// A line no function covers lists what the file does define, instead of guessing one.
/// 没有任何函数覆盖的行会列出该文件确实定义了什么，而不是猜一个。
#[test]
fn a_line_outside_every_definition_lists_the_definitions() {
    let root = scratch("why-outside");
    let answer = super::why(&root, &json!({"at": "src/lib.rs:1"})).expect("an answer");
    assert!(
        answer.contains("no function covers") && answer.contains("used"),
        "the index is printed rather than a guess: {answer}"
    );
}

/// An entry that names the file is read through the kernel's parser, not a second scan.
/// 点名了该文件的条目经**内核的解析器**读出，而不是第二次手写扫描。
#[test]
fn a_ledger_entry_naming_the_file_is_read() {
    let root = scratch("why-named");
    std::fs::create_dir_all(root.join(".xirang/adopted")).expect("ledger directory");
    std::fs::write(
        root.join(".xirang/adopted/entries"),
        "# a ledger written for this pin\nroot/control/button|the button face renders through its \
         contract|traced once|nich|2026-10-01T00:00:00+08:00|src/lib.rs|cafe|the pin needs a \
         naming entry\n",
    )
    .expect("ledger");
    let answer = super::why(&root, &json!({"at": "src/lib.rs:3"})).expect("an answer");
    assert!(
        answer.contains("1 entry(ies) name this file") && answer.contains("root/control/button"),
        "the anchor is named: {answer}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// A package with one parent face and one child face whose function carries a `#[cfg]`.
/// 一个包：一个父面，以及一个子面——它的函数带着一条 `#[cfg]`。
///
/// The layout is the one the derivation accepts (`<name>/<name>.rs`), so the face is real to every
/// reader rather than only to this test.
/// 布局是推导接受的那一种（`<name>/<name>.rs`），因此这个面对每个读者都是真的，而不只对本测试真。
fn face_package(label: &str) -> std::path::PathBuf {
    let root = std::env::temp_dir().join(format!("xirang-mcp-why-{label}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let write = |relative: &str, text: &str| {
        let path = root.join(relative);
        std::fs::create_dir_all(path.parent().expect("parent")).expect("fixture dirs");
        std::fs::write(path, text).expect("fixture file");
    };
    write(
        "Cargo.toml",
        &format!(
            "[package]\nname = \"fixture-why-{label}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n"
        ),
    );
    write("src/lib.rs", "//! fixture\npub mod control;\n");
    write(
        "src/control/control.rs",
        "pub struct Control;\n\ncrate::root_object! {\n    kind: Control,\n    needs_registry: \
         true,\n    parent: crate::root_node_id(env!(\"CARGO_PKG_NAME\")),\n}\n",
    );
    write(
        "src/control/object/button/button.rs",
        "/// A gated paint.\n#[cfg(feature = \"fancy\")]\npub fn fancy_paint() -> i32 { 1 }\n\n\
         pub struct Button;\n\ncrate::control_object! {\n    kind: Button,\n    exports: \
         [\"control.render\"],\n    parent: crate::control::NODE_ID,\n}\n",
    );
    root
}

/// The plan half rides with the source facts: the gate, the build's scope, and the declared cut.
/// 计划那一半随源码事实一起给出：门控、构建的作用域、以及已声明的切口。
///
/// This is the half the third hard-bug class turns on — a file present on disk but absent from the
/// scope the entry declared is how a new face vanishes — and every line answers rather than staying
/// silent, including the tree that was never built.
/// 这正是第三类复杂 bug 依赖的那一半——盘上有、而入口声明的作用域里没有，正是一个新面消失的来路——而每
/// 一行都作答而不是沉默，包括那棵从未构建过的树。
#[test]
fn the_plan_facts_ride_with_the_line() {
    let root = face_package("plan");
    let answer = super::why(
        &root,
        &json!({"at": "src/control/object/button/button.rs:3"}),
    )
    .expect("an answer");
    assert!(
        answer.contains(
            "gate       #[cfg(feature = \"fancy\")] at \
                         src/control/object/button/button.rs:2"
        ),
        "the attribute and its own line are named: {answer}"
    );
    assert!(
        // The line names the CLI that writes the file (this bridge never builds) and the calls that
        // answer the same question without it: the round measured readers spending steps trying to
        // make the bridge run `xirang check`, which it cannot.
        // 这一行点名写那个文件的**是 CLI**（本桥从不构建），以及不需要它也能回答同一问题的调用：那一轮
        // 量到读者花步数想让桥去跑 `xirang check`，而桥做不到。
        // The line now separates the two truths: a tree that does not host xirang has no scope to
        // read at all, while one that does names the file and the CLI that writes it.
        // 这一行现在把两种真相分开：没有接入 xirang 的树根本没有 scope 可读，接入的树则点名文件与
        // 写它的 CLI。
        answer.contains("scope unknown")
            && answer.contains("does not host xirang")
            && answer.contains("registry")
            && answer.contains("pruning unknown (no pruning_manifest.tsv"),
        "an unbuilt tree is answered as unbuilt rather than as out-of-scope: {answer}"
    );
    assert!(
        answer.contains("declares no graft cut"),
        "an entry with no cut says so: {answer}"
    );
    assert!(
        !answer.contains("whether this definition is in the published tree"),
        "the boundary no longer disowns a fact this reply now answers: {answer}"
    );
    assert!(
        answer.contains("not covered here") && answer.contains("next"),
        "a boundary and a next call still ride along: {answer}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// A definition with no attribute above it says so, rather than leaving the reader to guess.
/// 上方没有任何属性的定义会说出来，而不是让读者去猜。
#[test]
fn an_ungated_definition_says_it_is_ungated() {
    let root = face_package("ungated");
    std::fs::write(
        root.join("src/control/object/button/button.rs"),
        "pub fn plain_paint() -> i32 { 1 }\n\npub struct Button;\n\ncrate::control_object! {\n    \
         kind: Button,\n    parent: crate::control::NODE_ID,\n}\n",
    )
    .expect("fixture write");
    let answer = super::why(
        &root,
        &json!({"at": "src/control/object/button/button.rs:1"}),
    )
    .expect("answer");
    assert!(
        answer.contains("no `#[cfg]` attribute sits directly above this definition"),
        "the absence is stated with its own bound: {answer}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// A file no face owns has no scope membership to report, and the reply says that instead of guessing.
/// 没有面拥有的文件没有可报的作用域成员资格，而回复会说清这一点，而不是猜。
#[test]
fn a_file_no_face_owns_has_no_scope_membership() {
    let root = scratch("why-no-face");
    let answer = super::why(&root, &json!({"at": "src/lib.rs:3"})).expect("an answer");
    assert!(
        answer.contains("is not any registration face's own source")
            && answer.contains("`check {face}`'s question"),
        "the line names why it cannot answer and who can: {answer}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// A line inside a registration declaration is answered as a declaration, not as a miss.
/// 落在注册面声明里的行按"声明"作答，而不是当作没命中。
///
/// Measured in the W8 round: the old reply stopped at "no function covers this line" plus a function
/// list, and the agent asked again with a different line — twice in one round, on two questions. The
/// tree already knew the line was inside the face's declaration (the kernel's parser carries the
/// macro's whole span), so the answer was withholding a fact it held.
/// W8 那轮量到的：旧答案在"no function covers this line"加一份函数清单处停住，agent 只能换个行号再问一次
/// ——一轮两次、两道题。而树本来就知道这一行在面的声明里（内核解析器带着宏的整个区间），因此那次答案是
/// 扣下了自己手里的事实。
#[test]
fn a_line_inside_a_declaration_is_answered_as_a_declaration() {
    let root = face_package("declaration");
    let answer = super::why(
        &root,
        &json!({"at": "src/control/object/button/button.rs:9"}),
    )
    .expect("answer");
    assert!(
        answer.contains("inside the `control_object!` declaration spanning lines 7-11"),
        "the declaration and its span are named: {answer}"
    );
    assert!(
        answer.contains("`explain {node}` reports what it declares"),
        "and the next call for the declared fields is named: {answer}"
    );
    assert!(
        answer.contains("the functions in this file are:") && answer.contains("fancy_paint"),
        "the function index still rides along: {answer}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// The depth checklist names the two layers a change is easiest to forget: the siblings that must
/// follow the same shape, and what pins the definition.
/// 纵深清单点名改动时最容易忘的两层：必须跟着同一形状的兄弟，以及是什么钉着这条定义。
///
/// Measured (T-13): the six layers were spread across six tools, so "change this algorithm" had no
/// single answer that said what each layer needs. The counter-proof is the whole point of the second
/// half: **a layer with nothing on it must be named as missing**, not left off the list — a sibling
/// that no test names is exactly the change nothing would catch.
/// 量到的（T-13）：六层分散在六个工具里，于是"改这个算法"没有一个答案说出每一层各要动什么。反证是后半条的
/// 全部要点：**没有任何东西的一层必须被点名为缺失**，而不是从清单上消失——没有任何测试点名的兄弟，正是那种
/// "改了也没有东西会抓住"的改动。
#[test]
fn a_definition_answers_with_its_siblings_and_what_pins_it() {
    let root = face_package("depth");
    // A family of two: the fixture's own face file, plus a sibling under the same parent.
    std::fs::create_dir_all(root.join("src/control/object/twin")).expect("sibling dir");
    std::fs::write(
        root.join("src/control/object/twin/twin.rs"),
        "/// The twin's own paint.\npub fn fancy_paint() -> i32 { 2 }\n",
    )
    .expect("sibling source");
    let answer = super::why(
        &root,
        &json!({"at": "src/control/object/button/button.rs:3"}),
    )
    .expect("the line answers");
    assert!(
        answer.contains("siblings   ") && answer.contains("src/control/object/twin/twin.rs"),
        "the checklist names the sibling under the same parent: {answer}"
    );
    assert!(
        answer.contains("pins       "),
        "and it always answers what pins the definition, even when the answer is `nothing`: {answer}"
    );
}

/// The answer carries the tree's size and says whether it left anything open.
/// 答案自带树的大小，并说出它有没有留下没做完的事。
///
/// Audit `W4-5`/`W1-2`: sessions opened with `status` only to learn how big the tree was, and an
/// answer that ended on a `next` line gave no way to tell "done" from "you should keep going". Both
/// facts are ones this tool already holds.
/// 审计 `W4-5`/`W1-2`：会话先发一次 `status`、只为知道树有多大；而一个以 `next` 行收尾的答案，分不出
/// "做完了"与"你还得继续"。两件事都是这个工具本来就持有的事实。
#[test]
fn the_answer_carries_the_census_and_a_closure_line() {
    let root = scratch("why-closure");
    let answer = super::why(&root, &json!({"at": "src/lib.rs:3"})).expect("an answer");
    assert!(
        answer.starts_with("tree: ") && answer.contains(" file(s)"),
        "the census is the first line: {answer}"
    );
    assert_eq!(
        answer.matches("closure    ").count(),
        1,
        "exactly one closure line: {answer}"
    );
    assert!(
        answer.contains("closure    closed") || answer.contains("closure    open"),
        "and it is one of the two words: {answer}"
    );
    // The `next` line names calls this answer can already fill in: no `{face}`/`{function}` braces.
    let next = answer
        .lines()
        .find(|line| line.starts_with("next"))
        .expect("a next line");
    assert!(
        !next.contains('{')
            && next.contains("used")
            && next.contains("--call read --path src/lib.rs --line 3"),
        "the next line is instantiated with this answer's own names: {next}"
    );
    // An open answer names a command; none of them is a template.
    for line in answer.lines().filter(|line| line.starts_with("remaining")) {
        assert!(
            !line.contains('{') && !line.contains('<'),
            "a remaining command is one a reader can run: {line}"
        );
    }
}
