//! Pins for constraint admission: what a declaration decides, and what it never can.
//! 约束准入的钉子：声明能判定什么，以及它永远判不了什么。

use serde_json::json;

use super::constraints;

/// A host with three siblings under one parent, one of them declaring a capability.
/// 一个宿主：同一父级下三个兄弟，其中一个声明了能力。
fn host(label: &str) -> std::path::PathBuf {
    let (root, _) = crate::mcp::apply::apply_tests::package(label);
    for request in [
        json!({"action": "add", "fields": {"module": "control", "kind": "Control", "needs_registry": true}, "apply": true}),
        json!({"action": "add", "parent": "root/control", "fields": {"module": "button", "kind": "Button", "needs_registry": true, "provides": "cap.render=>Button"}, "apply": true}),
        json!({"action": "add", "parent": "root/control", "fields": {"module": "slider", "kind": "Slider", "needs_registry": true, "provides": "cap.render=>Slider"}, "apply": true}),
        json!({"action": "add", "parent": "root/control", "fields": {"module": "routing", "kind": "Routing", "needs_registry": true}, "apply": true}),
    ] {
        crate::mcp::apply::apply(&root, &request)
            .unwrap_or_else(|error| panic!("{request}: {error}"));
    }
    root
}

/// A05: an expectation the tree does not support comes back `unmet` **with the declarations it was
/// decided against** — and the reply says the conflict is between the expectation and the tree,
/// not a defect report about the business.
/// A05：树不支持的期望回 `unmet`，**并带上判定所依据的声明**——而且回复说这是期望与树之间的冲突，不是
/// 关于业务的缺陷报告。
#[test]
fn an_expectation_the_tree_does_not_support_is_unmet_with_its_evidence() {
    let root = host("unmet");
    let reply = constraints(
        &root,
        &json!({"node": "root/control/slider", "expect": {"provides": "cap.zoom"}}),
    )
    .expect("the tool answers");
    assert!(
        reply.starts_with("constraint unmet (structural)"),
        "{reply}"
    );
    assert!(reply.contains("node  root/control/slider"), "{reply}");
    assert!(
        reply.contains("provides") && reply.contains("cap.render=>Slider"),
        "the reply prints what the node does declare: {reply}"
    );
    assert!(
        reply.contains("only a person decides which of the two moves"),
        "an unmet expectation is not a licence to edit the business: {reply}"
    );
    // Best-effort, for the reason the W4 pins record: `apply` starts a background index refresh
    // of this tree, so a cleanup that walks it can meet a file the refresh just created.
    // 尽力而为，理由与 W4 的钉子相同：`apply` 会为这棵树启动后台索引刷新，遍历它的清理可能撞上刷新刚
    // 创建的文件。
    let _ = std::fs::remove_dir_all(root);
}

/// The same expectation against a declaration that does carry it is `met`, quoted with its file.
/// 同一条期望落在确实载有它的声明上就是 `met`，并带出它所在的文件。
#[test]
fn a_declaration_that_carries_the_expectation_is_met() {
    let root = host("met");
    let reply = constraints(
        &root,
        &json!({"node": "root/control/slider", "expect": {"provides": "cap.render=>Slider"}}),
    )
    .expect("the tool answers");
    assert!(reply.starts_with("constraint met (structural)"), "{reply}");
    assert!(
        reply.contains("file  control/object/slider/slider.rs"),
        "the identity-relative path is what a reader opens: {reply}"
    );
    assert!(
        reply.contains("kind") && reply.contains("Slider"),
        "the declaration lines are quoted so the gap is visible: {reply}"
    );
    // Best-effort, for the reason the W4 pins record: `apply` starts a background index refresh
    // of this tree, so a cleanup that walks it can meet a file the refresh just created.
    // 尽力而为，理由与 W4 的钉子相同：`apply` 会为这棵树启动后台索引刷新，遍历它的清理可能撞上刷新刚
    // 创建的文件。
    let _ = std::fs::remove_dir_all(root);
}

/// A12: a behavioural claim is `unknown` **however plausible it is**, because no declaration states
/// that every method must call the provider — the same rule the acceptance matrix writes down for
/// `.requires`.
/// A12：行为性主张一律 `unknown`，**无论它多像真的**——没有任何声明说"每个方法都必须调用该提供者"，
/// 这正是验收矩阵为 `.requires` 写下的规则。
#[test]
fn a_behavioural_claim_is_never_met_by_a_declaration() {
    let root = host("behavioural");
    for expect in [
        json!({"must_call": "root/control/button"}),
        json!({"via": "root/control/button"}),
        json!({"route": "root/control/slider -> root/control/button -> cap.render"}),
    ] {
        let reply = constraints(
            &root,
            &json!({"node": "root/control/slider", "expect": expect}),
        )
        .unwrap_or_else(|error| panic!("{expect}: {error}"));
        assert!(
            reply.starts_with("constraint unknown (undecided)"),
            "{expect}: a behavioural claim has no oracle here: {reply}"
        );
        assert!(
            reply.contains("majority of siblings is a candidate rather than a verdict"),
            "{reply}"
        );
    }
    // Best-effort, for the reason the W4 pins record: `apply` starts a background index refresh
    // of this tree, so a cleanup that walks it can meet a file the refresh just created.
    // 尽力而为，理由与 W4 的钉子相同：`apply` 会为这棵树启动后台索引刷新，遍历它的清理可能撞上刷新刚
    // 创建的文件。
    let _ = std::fs::remove_dir_all(root);
}

/// A11: three siblings that each declare their own provider are not a chain, and asking for a
/// declaration that none of them writes comes back `unmet` — the majority never becomes a verdict.
/// A11：三个各自声明自己提供者的兄弟不构成一条链；问一条谁都没写的声明只会得到 `unmet`——多数永远
/// 不会变成判决。
#[test]
fn a_sibling_majority_is_not_a_verdict() {
    let root = host("majority");
    let reply = constraints(
        &root,
        &json!({"node": "root/control/button", "expect": {"declares": "requires", "value": "cap.render=>Routing"}}),
    )
    .expect("the tool answers");
    assert!(
        reply.starts_with("constraint unmet (structural)"),
        "{reply}"
    );
    assert!(
        reply.contains("`requires` is absent here"),
        "the field is absent, and that is printed rather than guessed: {reply}"
    );
    // Best-effort, for the reason the W4 pins record: `apply` starts a background index refresh
    // of this tree, so a cleanup that walks it can meet a file the refresh just created.
    // 尽力而为，理由与 W4 的钉子相同：`apply` 会为这棵树启动后台索引刷新，遍历它的清理可能撞上刷新刚
    // 创建的文件。
    let _ = std::fs::remove_dir_all(root);
}

/// A malformed expectation is a caller error: it is `unknown` and says which key is wrong, rather
/// than being decided against the tree.
/// 畸形的期望是调用方错误：它回 `unknown` 并点名哪个键不对，而不是拿去对树判定。
#[test]
fn a_malformed_expectation_is_never_decided_against_the_tree() {
    let root = host("malformed");
    let missing = constraints(
        &root,
        &json!({"node": "root/control/button", "expect": {"declares": "provides"}}),
    )
    .expect("the tool answers");
    assert!(missing.starts_with("constraint unknown"), "{missing}");
    assert!(missing.contains("needs a sibling `value`"), "{missing}");

    let empty = constraints(
        &root,
        &json!({"node": "root/control/button", "expect": {"provides": "  "}}),
    )
    .expect("the tool answers");
    assert!(empty.starts_with("constraint unknown"), "{empty}");
    assert!(empty.contains("non-empty string"), "{empty}");

    let named = constraints(
        &root,
        &json!({"node": "root/nothing", "expect": {"provides": "x"}}),
    )
    .expect_err("a node that does not exist is refused by name");
    assert!(named.contains("no face in this tree"), "{named}");
    // Best-effort, for the reason the W4 pins record: `apply` starts a background index refresh
    // of this tree, so a cleanup that walks it can meet a file the refresh just created.
    // 尽力而为，理由与 W4 的钉子相同：`apply` 会为这棵树启动后台索引刷新，遍历它的清理可能撞上刷新刚
    // 创建的文件。
    let _ = std::fs::remove_dir_all(root);
}

/// The tool decides expectations and changes nothing: the tree's bytes are identical afterwards.
/// 这个工具只判定期望，什么都不改：之后树的字节逐字不变。
#[test]
fn deciding_an_expectation_writes_nothing() {
    let root = host("readonly");
    let before = crate::mcp::check_plan::fingerprint(&root, None).unwrap();
    for expect in [
        json!({"provides": "cap.render=>Slider"}),
        json!({"declares": "kind", "value": "Slider"}),
        json!({"must_call": "root/control/button"}),
    ] {
        let _ = constraints(
            &root,
            &json!({"node": "root/control/slider", "expect": expect}),
        )
        .expect("the tool answers");
    }
    assert_eq!(
        crate::mcp::check_plan::fingerprint(&root, None).unwrap(),
        before,
        "`constraints` decides; it never writes"
    );
    // Best-effort, for the reason the W4 pins record: `apply` starts a background index refresh
    // of this tree, so a cleanup that walks it can meet a file the refresh just created.
    // 尽力而为，理由与 W4 的钉子相同：`apply` 会为这棵树启动后台索引刷新，遍历它的清理可能撞上刷新刚
    // 创建的文件。
    let _ = std::fs::remove_dir_all(root);
}

/// W3: a **negative** control states something the tree must not carry, so the same fact means the
/// opposite thing — and running one is the only way to show a check can fail at all.
/// W3：**反控制**声明的是这棵树不该载有的东西，因此同一个事实意味着相反的结论——而只有真的跑一次反控制，
/// 才能证明一道检查能红。
#[test]
fn a_negative_control_passes_on_what_the_tree_does_not_carry() {
    let root = host("control");
    let passing = constraints(
        &root,
        &json!({"node": "root/control/slider", "expect": {"provides": "cap.zoom"}, "control": "negative"}),
    )
    .expect("the tool answers");
    assert!(
        passing.starts_with("constraint unmet (structural)"),
        "{passing}"
    );
    assert!(
        passing.contains("control negative unmet — control passed"),
        "an absent declaration is what a negative control is for: {passing}"
    );

    let failing = constraints(
        &root,
        &json!({"node": "root/control/slider", "expect": {"provides": "cap.render=>Slider"}, "control": "negative"}),
    )
    .expect("the tool answers");
    assert!(
        failing.starts_with("constraint met (structural)"),
        "{failing}"
    );
    assert!(
        failing.contains("control negative met — control failed"),
        "a negative control that the tree satisfies has failed, and says so: {failing}"
    );

    // The positive direction is the same fact read the other way.
    // 正方向就是把同一个事实反过来读。
    let positive = constraints(
        &root,
        &json!({"node": "root/control/slider", "expect": {"provides": "cap.render=>Slider"}, "control": "positive"}),
    )
    .expect("the tool answers");
    assert!(
        positive.contains("control positive met — control passed"),
        "{positive}"
    );
    let typo = constraints(
        &root,
        &json!({"node": "root/control/slider", "expect": {"provides": "cap.render=>Slider"}, "control": "either"}),
    )
    .expect_err("a direction that is neither positive nor negative is refused");
    assert!(typo.contains("`positive` or `negative`"), "{typo}");

    // An undecidable expectation is not a passing control in **either** direction.
    // 判不了的期望在两个方向下都不是通过的控制。
    for control in ["positive", "negative"] {
        let undecided = constraints(
            &root,
            &json!({"node": "root/control/slider", "expect": {"must_call": "root/control/button"}, "control": control}),
        )
        .expect("the tool answers");
        assert!(
            undecided.starts_with("constraint unknown"),
            "{control}: {undecided}"
        );
        assert!(
            undecided.contains("not a control"),
            "{control}: an undecidable expectation cannot pass: {undecided}"
        );
    }
    let _ = std::fs::remove_dir_all(root);
}

/// A08: the tool never decides which side is out of date — the line that says so is part of every
/// answer, and it does not change with who asks or which file the expectation came from.
/// A08：这个工具绝不判定哪一边过时——说这句话的那一行是每条答复的一部分，而且不随"谁问"或"期望出自哪个
/// 文件"而变。
#[test]
fn the_answer_never_assigns_blame() {
    let root = host("blame");
    let unmet = constraints(
        &root,
        &json!({"node": "root/control/slider", "expect": {"provides": "cap.zoom"}}),
    )
    .expect("the tool answers");
    assert!(
        unmet.contains("blame  undecided here"),
        "an unmet expectation says out loud that it is not a verdict on the business: {unmet}"
    );
    assert!(unmet.contains("not something this tool decides"), "{unmet}");
    // The line is a property of the tool, not of the verdict: `met`, `unmet` and `unknown` all
    // carry the same one, so a reader can never read blame out of a word.
    // 这一行是工具的属性、不是判决的属性：`met`、`unmet`、`unknown` 都带着同一行，因此读者不可能从一个
    // 词里读出罪责。
    let met = constraints(
        &root,
        &json!({"node": "root/control/slider", "expect": {"provides": "cap.render=>Slider"}}),
    )
    .expect("the tool answers");
    let by_module = constraints(
        &root,
        &json!({"node": "root/control/slider", "expect": {"must_call": "root/control/button"}}),
    )
    .expect("the tool answers an undecidable expectation too");
    assert!(met.starts_with("constraint met"), "{met}");
    assert!(by_module.starts_with("constraint unknown"), "{by_module}");
    let blame = |text: &str| {
        text.lines()
            .find(|line| line.starts_with("blame"))
            .unwrap_or_default()
            .to_owned()
    };
    assert_eq!(blame(&unmet), blame(&met));
    assert_eq!(blame(&unmet), blame(&by_module));
    let _ = std::fs::remove_dir_all(root);
}

/// A08: the two causes are reported **separately** and each is marked a candidate — the tool reads
/// whether the tree's declarations carry the expectation, and never which side is out of date.
/// A08：两类原因**分别**报出，且各自标明是候选——工具读的是"这棵树的声明是否载有该期望"，而不是"哪一边过时"。
///
/// The four situations the acceptance names are covered here: a legitimate implementation with an
/// unsupported expectation (`constraint_invalid`), a supported expectation (`implementation_violation`
/// territory), and a claim nothing here can decide (`unknown`) — under both control directions, so
/// the label never depends on which way the expectation was aimed.
/// 这里覆盖验收点名的四种情形：合法实现配了树不支持的期望（`constraint_invalid`）、期望被声明载有
/// （`implementation_violation` 那一侧）、以及这里根本判不了的主张（`unknown`）——两个控制方向都跑，
/// 因此标签绝不取决于期望瞄准的是哪一边。
#[test]
fn the_two_causes_are_reported_separately_and_marked_as_candidates() {
    let root = host("cause");
    let label = |expect: serde_json::Value, control: &str| {
        let reply = constraints(
            &root,
            &json!({"node": "root/control/slider", "expect": expect, "control": control}),
        )
        .expect("the tool answers");
        let line = reply
            .lines()
            .find(|line| line.starts_with("cause"))
            .unwrap_or_else(|| panic!("the answer names a cause: {reply}"))
            .to_owned();
        assert!(
            reply.contains("blame  undecided here"),
            "and it still refuses to decide blame: {reply}"
        );
        line
    };
    for control in ["positive", "negative"] {
        assert_eq!(
            label(json!({"provides": "cap.zoom"}), control),
            "cause  constraint_invalid (candidate)",
            "an expectation the declarations do not carry points at the constraint side"
        );
        assert_eq!(
            label(json!({"provides": "cap.render=>Slider"}), control),
            "cause  implementation_violation (candidate)",
            "one they do carry points at the implementation side"
        );
        assert_eq!(
            label(json!({"must_call": "root/control/button"}), control),
            "cause  unknown",
            "and nothing here decides a behavioural claim"
        );
    }
    let _ = std::fs::remove_dir_all(root);
}
