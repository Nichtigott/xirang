//! Pins for immutable verification variants, driven through the real plan entry.
//! 不可变验证变体的钉子，从真实的计划入口驱动。
//!
//! The point of these is the *cost shape* of a counterproof, not only its verdict: the fault must be
//! injected somewhere the base tree is not, the named test must be the thing that catches it, and a
//! variant that changed the wrong bytes must not be able to borrow the right verdict.
//! 这些钉子要钉的是反证的**代价形状**，不只是它的判定：故障必须被注入到基树之外的地方、抓住它的必须是点名的
//! 那个测试，而改错了字节的变体不得借用正确的判定。

use crate::mcp::check_plan::run;
use serde_json::{Value, json};
use std::path::PathBuf;

/// A package with a shared implementation, one consumer, and one integration test that protects the
/// consumer's contract — so a fault in the shared file is caught by a test that lives somewhere else.
/// 一个有共享实现、一个消费者、以及一个保护消费者合同的集成测试的包——因此在共享文件里注入的故障会被一个
/// 住在别处的测试抓住。
fn fixture() -> PathBuf {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let root =
        std::env::temp_dir().join(format!("xirang-variant-{}-{sequence}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::create_dir_all(root.join("tests")).unwrap();
    std::fs::write(
        root.join("Cargo.toml"),
        "[package]\nname='variant-probe'\nversion='0.0.0'\nedition='2024'\n[workspace]\n",
    )
    .unwrap();
    std::fs::write(
        root.join("src/lib.rs"),
        "pub mod arithmetic;\npub mod consumer;\n",
    )
    .unwrap();
    std::fs::write(
        root.join("src/arithmetic.rs"),
        "/// Clamp a negative quantity to zero.\npub fn settle(n: i64) -> i64 {\n    n.max(0)\n}\n",
    )
    .unwrap();
    std::fs::write(
        root.join("src/consumer.rs"),
        "pub fn clamp(n: i64) -> i64 {\n    crate::arithmetic::settle(n)\n}\n",
    )
    .unwrap();
    std::fs::write(
        root.join("tests/protection.rs"),
        "#[test]\nfn a_negative_input_is_clamped() {\n    assert_eq!(variant_probe::consumer::clamp(-5), 0, \"CLAMP_CONTRACT_VIOLATED\");\n}\n",
    )
    .unwrap();
    let output = std::process::Command::new("cargo")
        .args(["generate-lockfile", "--offline"])
        .current_dir(&root)
        .output()
        .unwrap();
    assert!(output.status.success());
    root
}

/// The ledger one plan wrote, read back from its own reply.
/// 一份计划写下的台账，从它自己的回复读回。
fn ledger_of(reply: &str) -> Value {
    let record = reply
        .lines()
        .find_map(|line| line.strip_prefix("record "))
        .unwrap_or_else(|| panic!("no record line in:\n{reply}"));
    serde_json::from_str(&std::fs::read_to_string(record).unwrap()).unwrap()
}

/// BR09: a declared fault is injected into a private copy, the **named** test catches it on the
/// **named** violation, and the base tree is byte-identical afterwards. The alternative this replaces
/// — reverting the fix in the working tree — needs a whole-repository rebuild to undo.
/// BR09：声明的故障被注入一份私有副本，**点名的**测试在**点名的**违约上抓住它，而基树事后逐字节不变。
/// 它所替代的做法——在工作树里恢复旧实现——需要一次全仓重建才能复原。
#[test]
fn a_declared_fault_is_injected_into_a_copy_and_caught_by_the_named_test() {
    let root = fixture();
    let before = std::fs::read(root.join("src/arithmetic.rs")).unwrap();
    let reply = run(
        &root,
        &json!({"files":["src/arithmetic.rs"],"face":"default","cases":[
            {"name":"revert the clamp","expected":"failed",
             "request":{"workspace":true,"face":"default"},
             "expect":{"test":"a_negative_input_is_clamped","target":"tests/protection.rs","violation":"CLAMP_CONTRACT_VIOLATED"},
             "variant":{"edits":[{"path":"src/arithmetic.rs","content":"/// Clamp a negative quantity to zero.\npub fn settle(n: i64) -> i64 {\n    n\n}\n"}]}}
        ]}),
    )
    .unwrap();
    assert!(reply.starts_with("obligations passed"), "{reply}");
    assert!(reply.contains("revert the clamp: passed"), "{reply}");
    assert!(
        reply.contains("it runs in an immutable copy at")
            && reply.contains("a variant is never a candidate and is never published"),
        "{reply}"
    );
    assert!(
        reply.contains("base tree unchanged across every obligation"),
        "{reply}"
    );
    assert_eq!(
        std::fs::read(root.join("src/arithmetic.rs")).unwrap(),
        before,
        "the base tree is not where the fault was injected"
    );
    let ledger = ledger_of(&reply);
    let variant = &ledger["obligations"][1]["variant"];
    assert!(
        variant["digest"]
            .as_str()
            .is_some_and(|digest| digest.len() == 16),
        "{ledger}"
    );
    assert!(
        variant["edits"][0]
            .as_str()
            .is_some_and(|line| line.starts_with("wrote src/arithmetic.rs")),
        "{ledger}"
    );
    // The copy is outside the source tree the plan reports on.
    // 副本在计划所报告的源码树之外。
    let tree = ledger["obligations"][1]["variant_root"].as_str().unwrap();
    assert!(tree.contains("xirang/variants"), "{tree}");
    assert!(
        std::fs::read_to_string(std::path::Path::new(tree).join("src/arithmetic.rs"))
            .unwrap()
            .contains("    n\n"),
        "the copy really carries the declared fault"
    );
    std::fs::remove_dir_all(root).unwrap();
}

/// BR09: a variant that changed bytes the named test does not read cannot borrow the counterproof.
/// The plan stays open because the intended failure was not reproduced — which is exactly the
/// difference between "the protection caught a fault" and "something red happened".
/// BR09：改了点名测试不读的字节的变体不能借用这次反证。计划保持开启，因为预期失败没有被复现——这正是
/// "保护抓住了一个故障"与"出了点红"的区别。
#[test]
fn a_variant_that_does_not_move_the_named_test_does_not_earn_the_counterproof() {
    let root = fixture();
    let reply = run(
        &root,
        &json!({"files":["src/arithmetic.rs"],"face":"default","cases":[
            {"name":"unrelated edit","expected":"failed",
             "request":{"workspace":true,"face":"default"},
             "expect":{"test":"a_negative_input_is_clamped","target":"tests/protection.rs","violation":"CLAMP_CONTRACT_VIOLATED"},
             "variant":{"edits":[{"path":"src/consumer.rs","content":"pub fn clamp(n: i64) -> i64 {\n    // a comment is not a fault\n    crate::arithmetic::settle(n)\n}\n"}]}}
        ]}),
    )
    .unwrap();
    assert!(reply.starts_with("obligations open"), "{reply}");
    assert!(reply.contains("unrelated edit: failed"), "{reply}");
    assert!(reply.contains("remaining obligations:"), "{reply}");
    std::fs::remove_dir_all(root).unwrap();
}

/// BR09: two variants with different bodies are two executions even when nothing about the tree's
/// timestamps distinguishes them. The copy's directory and the run's `candidate` are both derived
/// from the declared edits, so no artifact of one can be read as evidence about the other.
/// BR09：正文不同的两个变体是两次执行，即使树的时间戳上没有任何东西能把它们区分开。副本目录与运行的
/// `candidate` 都派生自声明的改动，因此一个变体的产物不可能读成关于另一个的证据。
#[test]
fn two_variants_with_equal_timestamps_are_two_executions() {
    let root = fixture();
    let reply = run(
        &root,
        &json!({"files":["src/arithmetic.rs"],"face":"default","cases":[
            {"name":"fault one","expected":"failed",
             "request":{"workspace":true,"face":"default"},
             "expect":{"test":"a_negative_input_is_clamped","target":"tests/protection.rs","violation":"CLAMP_CONTRACT_VIOLATED"},
             "variant":{"edits":[{"path":"src/arithmetic.rs","content":"pub fn settle(n: i64) -> i64 { n }\n"}]}},
            {"name":"fault two","expected":"failed",
             "request":{"workspace":true,"face":"default"},
             "expect":{"test":"a_negative_input_is_clamped","target":"tests/protection.rs","violation":"CLAMP_CONTRACT_VIOLATED"},
             "variant":{"edits":[{"path":"src/arithmetic.rs","content":"pub fn settle(n: i64) -> i64 { n - 1 }\n"}]}}
        ]}),
    )
    .unwrap();
    let ledger = ledger_of(&reply);
    let rows = ledger["obligations"].as_array().unwrap();
    assert_ne!(
        rows[1]["variant"]["digest"], rows[2]["variant"]["digest"],
        "two different declared faults are two variants: {ledger}"
    );
    assert_ne!(
        rows[1]["candidate"], rows[2]["candidate"],
        "and two different copies, so neither one's build products can answer for the other: {ledger}"
    );
    assert_ne!(
        rows[1]["request"]["target_dir"], rows[2]["request"]["target_dir"],
        "nor do they share an output directory: {ledger}"
    );
    assert_ne!(
        rows[1]["execution_digest"], rows[2]["execution_digest"],
        "so their execution identities differ even though every timestamp matches: {ledger}"
    );
    std::fs::remove_dir_all(root).unwrap();
}

/// BR09/BR-D: a variant may not edit the file that declares the test supposed to catch it. Without
/// this refusal the cheapest way to make a counterproof "pass" is to weaken its own protection, and
/// the reply would report that as a detected fault.
/// BR09/BR-D：变体不得改动声明"本该抓住它"的那个测试的文件。没有这条拒绝，让一次反证"通过"的最省事写法
/// 就是削弱它自己的保护，而回复会把那报成检出故障。
#[test]
fn a_variant_may_not_edit_the_file_that_declares_its_own_protection_test() {
    let root = fixture();
    let error = run(
        &root,
        &json!({"files":["src/arithmetic.rs"],"face":"default","cases":[
            {"name":"weaken the test","expected":"failed",
             "request":{"workspace":true,"face":"default"},
             "expect":{"test":"a_negative_input_is_clamped","target":"tests/protection.rs","violation":"CLAMP_CONTRACT_VIOLATED"},
             "variant":{"edits":[{"path":"tests/protection.rs","content":"#[test]\nfn a_negative_input_is_clamped() {}\n"}]}}
        ]}),
    )
    .expect_err("a variant must not rewrite its own protection test");
    assert!(
        error.contains("may not modify its own protection test"),
        "{error}"
    );
    assert_eq!(
        std::fs::read_to_string(root.join("tests/protection.rs")).unwrap(),
        "#[test]\nfn a_negative_input_is_clamped() {\n    assert_eq!(variant_probe::consumer::clamp(-5), 0, \"CLAMP_CONTRACT_VIOLATED\");\n}\n",
        "and the refusal happens before anything is written"
    );
    std::fs::remove_dir_all(root).unwrap();
}

/// BR-D: the variant's own declaration is validated like every other parameter — a path that could
/// escape the copy, an unknown key, or a variant on the outer plan is refused before Cargo runs.
/// BR-D：变体自己的声明与其他参数一样受校验——可能逃出副本的路径、未知的键、或写在外层计划上的变体，都在
/// Cargo 运行之前被拒绝。
#[test]
fn a_variant_declaration_is_validated_before_anything_is_copied() {
    let root = fixture();
    for (case, expected) in [
        (
            json!({"name":"escape","expected":"passed","request":{"workspace":true,"face":"default"},
                   "variant":{"edits":[{"path":"../outside.rs","content":"pub fn x() {}\n"}]}}),
            "without `..`",
        ),
        (
            json!({"name":"whole tree","expected":"passed","request":{"workspace":true,"face":"default"},
                   "variant":{"root":"/tmp","edits":[{"path":"src/lib.rs","content":""}]}}),
            "accepts only `edits`",
        ),
        (
            json!({"name":"empty","expected":"passed","request":{"workspace":true,"face":"default"},
                   "variant":{"edits":[]}}),
            "must be 1..32 entries",
        ),
    ] {
        let error = run(
            &root,
            &json!({"files":["src/arithmetic.rs"],"face":"default","cases":[case]}),
        )
        .expect_err("a malformed variant is refused");
        assert!(error.contains(expected), "{expected}: {error}");
    }
    // A variant belongs to a case: on the outer plan it is an unsupported parameter, not a silently
    // ignored one.
    // 变体属于用例：写在外层计划上时它是"不受支持的参数"，而不是被静默忽略的参数。
    let outer = run(
        &root,
        &json!({"files":["src/arithmetic.rs"],"face":"default",
                "variant":{"edits":[{"path":"src/lib.rs","content":""}]}}),
    )
    .expect_err("the outer plan has no variant");
    assert!(
        outer.contains("unsupported check parameter `variant`"),
        "{outer}"
    );
    assert!(
        !root.join("target/xirang/variants").exists(),
        "and nothing was copied"
    );
    std::fs::remove_dir_all(root).unwrap();
}

/// BR08, the legal half: a **replacement** provider the consumers keep delegating to must pass. The
/// variant changes the shared body; every consumer test still runs through it.
/// BR08 的合法那一半：消费者继续委派的**替换**提供者必须通过。变体换掉共享实现的主体，而每个消费者测试
/// 仍然经过它运行。
#[test]
fn a_legal_replacement_provider_still_passes_the_consumers() {
    let root = fixture();
    let reply = run(
        &root,
        &json!({"files":["src/arithmetic.rs"],"face":"default","cases":[
            {"name":"equivalent replacement","expected":"passed",
             "request":{"workspace":true,"face":"default"},
             "variant":{"kind":"replacement",
                        "route":{"caller":"src/consumer.rs","callee":"settle"},
                        "edits":[{"path":"src/arithmetic.rs","content":"/// Clamp a negative quantity to zero.\npub fn settle(n: i64) -> i64 {\n    if n < 0 { 0 } else { n }\n}\n"}]}}
        ]}),
    )
    .unwrap();
    assert!(reply.starts_with("obligations passed"), "{reply}");
    assert!(reply.contains("equivalent replacement: passed"), "{reply}");
    assert!(
        reply.contains("variant equivalent replacement [replacement, digest"),
        "{reply}"
    );
    assert!(!reply.contains("variant route:"), "{reply}");
    std::fs::remove_dir_all(root).unwrap();
}

/// BR08, the detecting half: a consumer that **copies the formula** and stops delegating passes every
/// behavioural test — the values are identical — so only the route judgement can see it. The duty is
/// recorded separately from the behaviour verdict, and the plan stays open.
/// BR08 的检出那一半：把公式**抄过去**、不再委派的消费者会通过每一条行为测试——取值完全相同——因此只有
/// 路线判定看得见它。这条义务与行为判定分开记录，计划保持开启。
#[test]
fn a_consumer_that_copies_the_formula_is_detected_by_the_route_and_not_by_behaviour() {
    let root = fixture();
    let reply = run(
        &root,
        &json!({"files":["src/arithmetic.rs"],"face":"default","cases":[
            {"name":"formula copied downstream","expected":"passed",
             "request":{"workspace":true,"face":"default"},
             "variant":{"kind":"replacement",
                        "route":{"caller":"src/consumer.rs","callee":"settle"},
                        "edits":[{"path":"src/consumer.rs","content":"pub fn clamp(n: i64) -> i64 {\n    if n < 0 { 0 } else { n }\n}\n"}]}}
        ]}),
    )
    .unwrap();
    // The behaviour run is green — that is the whole difficulty — and the route duty is what refuses
    // to call this a verified replacement.
    // 行为运行是绿的——这正是难点所在——而拒绝把这件事称为"已验证替换"的，是那条路线义务。
    assert!(
        reply.contains("formula copied downstream: passed"),
        "{reply}"
    );
    assert!(reply.starts_with("obligations open"), "{reply}");
    assert!(
        reply.contains("variant route: formula copied downstream: unknown"),
        "{reply}"
    );
    assert!(
        reply.contains(
            "a behaviourally green run is what a locally re-implemented formula produces"
        ),
        "{reply}"
    );
    let ledger = ledger_of(&reply);
    assert_eq!(
        ledger["obligations"][1]["variant"]["route_holds"], false,
        "{ledger}"
    );
    std::fs::remove_dir_all(root).unwrap();
}

/// BR08: the declared purpose and the declared outcome are one decision, so the contradictory pairs
/// are refused before anything is copied.
/// BR08：声明的用途与声明的结果是一个决定，因此自相矛盾的那两对在复制任何东西之前就被拒绝。
#[test]
fn a_variant_kind_and_its_expected_outcome_cannot_contradict_each_other() {
    let root = fixture();
    for (case, expected) in [
        (
            json!({"name":"fault that passes","expected":"passed","request":{"workspace":true,"face":"default"},
                   "variant":{"kind":"fault","edits":[{"path":"src/arithmetic.rs","content":"pub fn settle(n: i64) -> i64 { n }\n"}]}}),
            "a fault variant is a counterproof, so its expected outcome is `failed`",
        ),
        (
            json!({"name":"replacement that fails","expected":"failed","request":{"workspace":true,"face":"default"},
                   "expect":{"test":"a_negative_input_is_clamped","target":"tests/protection.rs","violation":"CLAMP_CONTRACT_VIOLATED"},
                   "variant":{"kind":"replacement","edits":[{"path":"src/arithmetic.rs","content":"pub fn settle(n: i64) -> i64 { n }\n"}]}}),
            "a replacement variant must still pass",
        ),
        (
            json!({"name":"route with a path","expected":"passed","request":{"workspace":true,"face":"default"},
                   "variant":{"kind":"replacement","route":{"caller":"../outside.rs","callee":"settle"},
                              "edits":[{"path":"src/arithmetic.rs","content":"pub fn settle(n: i64) -> i64 { n }\n"}]}}),
            "route `caller` must be a package-relative path",
        ),
    ] {
        let error = run(
            &root,
            &json!({"files":["src/arithmetic.rs"],"face":"default","cases":[case]}),
        )
        .expect_err("a contradictory variant is refused");
        assert!(error.contains(expected), "{expected}: {error}");
    }
    assert!(
        !root.join("target/xirang/variants").exists(),
        "and nothing was copied or run"
    );
    std::fs::remove_dir_all(root).unwrap();
}
