//! Pins for the boundaries a candidate is judged against beyond its path scope — the inputs the
//! verdict was bound to (R1), the consumer obligations the change raises (R3), the content inside a
//! writable file that a person protected (R4), and the tests that were already the criteria (EX11).
//! 候选在路径范围之外还要对照的几条边界的钉子——判定绑定在哪些输入上（R1）、这次改动抬起的消费者义务
//! （R3）、可写文件内被人保护住的内容（R4），以及本来就充当判据的那些测试（EX11）。
//!
//! All of them start from the refusal side, because the 2026-10-19 review's five reproductions were all
//! "the tool could say it, the delivery was not bound by it". R4 and EX11 are additionally pinned as
//! **host** boundaries: a candidate's own record may add protection, never remove it.
//! 它们都从拒绝那一侧写起，因为 2026-10-19 复核的五条复现全都是"工具说得出、交付却不受它约束"。R4 与 EX11
//! 另外还被钉成**宿主**边界：候选自己的记录只能增加保护，不能减少。

use super::candidate_tests::{approve, confirm, fixture, opened};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};

/// Open a candidate with extra `open` arguments, then stand in for the person who confirms its scope.
/// 带上额外的 `open` 参数开一份候选，然后代表确认其范围的那个人。
fn opened_with(root: &Path, extra: Value, scope: &[&str]) -> (String, PathBuf) {
    let reply = super::candidate(root, &open_with(extra)).unwrap();
    let id = reply
        .lines()
        .next()
        .and_then(|line| line.strip_prefix("candidate "))
        .and_then(|rest| rest.strip_suffix(" open"))
        .expect("the opening line names the candidate")
        .split_whitespace()
        .next()
        .expect("the id is the first word")
        .to_owned();
    let tree = reply
        .lines()
        .find_map(|line| line.strip_prefix("tree "))
        .expect("the opening line names the tree");
    let scope: Vec<String> = scope.iter().map(|entry| (*entry).to_owned()).collect();
    approve(root, &id, &scope);
    (id, PathBuf::from(tree))
}

/// The `open` request with `extra` folded in.
/// 把 `extra` 折进去的 `open` 请求。
fn open_with(extra: Value) -> Value {
    let mut request = json!({"action":"open","authority":"control.object.button"});
    if let (Some(target), Some(extra)) = (request.as_object_mut(), extra.as_object()) {
        for (key, value) in extra {
            target.insert(key.clone(), value.clone());
        }
    }
    request
}

/// R1: a test that reads an input, passes, then rewrites it. The verdict of such a run must not be
/// bound to the bytes the run **ended** on — that is how the review's candidate landed a passing
/// verification whose own test fails in the base tree.
/// R1：一条先读输入、通过、然后把它改写的测试。这种运行的判定不能绑定到它**结束**时的字节上——复核里的候选
/// 正是这样把一份"通过"落了地，而它的测试在基树里立刻失败。
#[test]
fn a_verification_cannot_certify_inputs_that_changed_while_it_ran() {
    let root = fixture();
    let base_library = std::fs::read_to_string(root.join("src/lib.rs")).unwrap();
    std::fs::write(root.join("data.txt"), "good\n").unwrap();
    std::fs::write(
        root.join("src/lib.rs"),
        "pub fn answer() -> i64 {\n    41\n}\n\n#[test]\nfn reads_the_input_and_rewrites_it() {\n    let text = std::fs::read_to_string(\"data.txt\").unwrap();\n    assert_eq!(text.trim(), \"good\");\n    std::fs::write(\"data.txt\", \"bad\\n\").unwrap();\n}\n",
    )
    .unwrap();
    let baseline = std::fs::read_to_string(root.join("src/lib.rs")).unwrap();
    let (id, _tree) = opened(&root, json!(["src"]));
    let error = super::candidate(&root, &json!({"action":"verify","id":id,"face":"default"}))
        .expect_err("a run whose inputs moved under it certifies nothing");
    assert!(
        error.contains("inputs changed while its verification was running")
            && error.contains("data.txt"),
        "the refusal names the input that moved: {error}"
    );
    let error = super::candidate(&root, &json!({"action":"land","id":id}))
        .expect_err("the unbound verdict cannot land");
    assert!(
        error.contains("is not verified") || error.contains("verification"),
        "the landing refuses on the verdict it recorded: {error}"
    );
    assert_eq!(
        std::fs::read_to_string(root.join("src/lib.rs")).unwrap(),
        baseline,
        "the base is untouched (and differs from the pre-candidate file only by the test the test wrote)"
    );
    assert_ne!(baseline, base_library);
}

/// R3: the consumer obligations `check {files}` reports must bind the candidate's verdict and the
/// landing. A provider unit test alone is exactly the shape that passed `verify` and landed while
/// `check` still said the consumer protection was open.
/// R3：`check {files}` 报出的消费者义务必须约束候选的判定与落地。只加一个提供者单测，正是那种 `verify` 通过并
/// 落地、而 `check` 仍说消费者保护还开着的形状。
#[test]
fn a_candidate_with_an_open_consumer_obligation_is_not_verified_or_landed() {
    let root = fixture();
    std::fs::write(
        root.join("src/lib.rs"),
        "pub mod arithmetic;\npub mod first;\npub mod second;\n",
    )
    .unwrap();
    for (name, body) in [
        (
            "src/arithmetic.rs",
            "pub fn settle(n: i64) -> i64 {\n    n.max(0)\n}\n",
        ),
        (
            "src/first.rs",
            "pub fn first_round(n: i64) -> i64 {\n    crate::arithmetic::settle(n)\n}\n",
        ),
        (
            "src/second.rs",
            "pub fn second_round(n: i64) -> i64 {\n    crate::arithmetic::settle(n)\n}\n",
        ),
    ] {
        std::fs::write(root.join(name), body).unwrap();
    }
    let (id, tree) = opened(&root, json!(["src", "tests"]));
    // Only the provider gains a test: the two consumers remain unproven.
    // 只有提供者多了一条测试：两个消费者仍然没有被证明。
    std::fs::write(
        tree.join("src/arithmetic.rs"),
        "pub fn settle(n: i64) -> i64 {\n    n.max(0)\n}\n\n#[cfg(test)]\nmod tests {\n    #[test]\n    fn settle_is_non_negative() {\n        assert_eq!(super::settle(-1), 0);\n    }\n}\n",
    )
    .unwrap();
    let reply =
        super::candidate(&root, &json!({"action":"verify","id":id,"face":"default"})).unwrap();
    assert!(
        reply.contains("verification unmet") && reply.contains("2 consumer obligation(s) open"),
        "the verdict reports the obligations it leaves open: {reply}"
    );
    let error = super::candidate(&root, &json!({"action":"land","id":id}))
        .expect_err("an unmet verification cannot land");
    assert!(
        error.contains("unmet") || error.contains("obligation"),
        "{error}"
    );
    // The legal continuation: a test per consumer closes both duties, and the same candidate lands.
    // 合法的续作：每个消费者一条测试结清两项义务，同一份候选落地。
    std::fs::create_dir_all(tree.join("tests")).unwrap();
    std::fs::write(
        tree.join("tests/first.rs"),
        "#[test]\nfn first_round_is_protected() {\n    assert_eq!(candidate_probe::first::first_round(-4), 0);\n}\n",
    )
    .unwrap();
    std::fs::write(
        tree.join("tests/second.rs"),
        "#[test]\nfn second_round_is_protected() {\n    assert_eq!(candidate_probe::second::second_round(-2), 0);\n}\n",
    )
    .unwrap();
    let reply =
        super::candidate(&root, &json!({"action":"verify","id":id,"face":"default"})).unwrap();
    assert!(reply.contains("verification passed"), "{reply}");
    super::candidate(&root, &json!({"action":"land","id":id})).unwrap();
}

/// R4: the thing a person protects can be smaller than a file, and the person says it at the host's
/// landing entry. A candidate that never mentioned `protect` cannot land a change to the function the
/// host protected; a candidate's own claim at `open` is still checked (it may add, never remove).
/// R4：人真正要保的东西可以比一个文件更小，而这句话由人在宿主的落地入口说。从未提过 `protect` 的候选，不能把
/// 宿主保护的那个函数的改动落下去；候选自己在 `open` 时写下的声明仍会被检查（它只能增加，不能减少）。
#[test]
fn protected_content_is_a_host_boundary_not_a_candidate_claim() {
    let root = fixture();
    let base = "pub fn writable_helper() -> i64 {\n    1\n}\n\npub fn protected_base() -> i64 {\n    41\n}\n\n#[test]\nfn the_base_is_stable() {\n    assert_eq!(protected_base(), 41);\n}\n";
    std::fs::write(root.join("src/lib.rs"), base).unwrap();
    let (id, tree) = opened_with(&root, json!({"scope":["src/lib.rs"]}), &["src/lib.rs"]);
    // A change that keeps the tree's own tests green, because it moves the assertion with the function.
    // 一次让树自己的测试保持绿的改动：它把断言跟着函数一起改了。
    std::fs::write(
        tree.join("src/lib.rs"),
        "pub fn writable_helper() -> i64 {\n    1\n}\n\npub fn protected_base() -> i64 {\n    42\n}\n\n#[test]\nfn the_base_is_stable() {\n    assert_eq!(protected_base(), 42);\n}\n",
    )
    .unwrap();
    let reply =
        super::candidate(&root, &json!({"action":"verify","id":id,"face":"default"})).unwrap();
    assert!(
        reply.contains("verification passed"),
        "the candidate was told nothing about protection: {reply}"
    );
    // The person protects the function at the landing entry; the same bytes now cannot land.
    // 人在落地入口保护住这个函数；同样这棵树现在不能落地。
    assert_eq!(
        confirm(
            &root,
            &id,
            &["--land", "--protect", "src/lib.rs#protected_base"],
        ),
        0
    );
    let error = super::candidate(&root, &json!({"action":"land","id":id}))
        .expect_err("the host's boundary is what decides");
    assert!(
        error.contains("protected content") && error.contains("protected_base"),
        "{error}"
    );
    assert_eq!(
        std::fs::read_to_string(root.join("src/lib.rs")).unwrap(),
        base
    );
    // A claim the author writes at `open` is still checked, so protection cannot be skipped by omission.
    // 作者在 `open` 时写下的声明仍会被检查，因此保护不会因遗漏而不生效。
    let (claimed, claimed_tree) = opened_with(
        &root,
        json!({"protect":json!([{"path":"src/lib.rs","entity":"protected_base"}]),"scope":["src/lib.rs"]}),
        &["src/lib.rs"],
    );
    std::fs::write(
        claimed_tree.join("src/lib.rs"),
        "pub fn writable_helper() -> i64 {\n    1\n}\n\npub fn protected_base() -> i64 {\n    43\n}\n\n#[test]\nfn the_base_is_stable() {\n    assert_eq!(protected_base(), 43);\n}\n",
    )
    .unwrap();
    let error = super::candidate(
        &root,
        &json!({"action":"verify","id":claimed,"face":"default"}),
    )
    .expect_err("the author's own claim protects it too");
    assert!(
        error.contains("protected content") && error.contains("protected_base"),
        "{error}"
    );
    // The legal half: the writable helper changes, the protected function does not, and the host says so.
    // 合法的另一半：可写的那一半改了、受保护的函数没动，并且这一点由宿主说出来。
    let (second, second_tree) =
        opened_with(&root, json!({"scope":["src/lib.rs"]}), &["src/lib.rs"]);
    std::fs::write(
        second_tree.join("src/lib.rs"),
        "pub fn writable_helper() -> i64 {\n    2\n}\n\npub fn protected_base() -> i64 {\n    41\n}\n\n#[test]\nfn the_base_is_stable() {\n    assert_eq!(protected_base(), 41);\n}\n\n#[test]\nfn the_helper_moves() {\n    assert_eq!(writable_helper(), 2);\n}\n",
    )
    .unwrap();
    let reply = super::candidate(
        &root,
        &json!({"action":"verify","id":second,"face":"default"}),
    )
    .unwrap();
    assert!(reply.contains("verification passed"), "{reply}");
    assert_eq!(
        confirm(
            &root,
            &second,
            &["--land", "--protect", "src/lib.rs#protected_base"],
        ),
        0
    );
    super::candidate(&root, &json!({"action":"land","id":second})).unwrap();
    assert!(
        std::fs::read_to_string(root.join("src/lib.rs"))
            .unwrap()
            .contains("2")
    );
}

/// EX11: deleting a test that was already a criterion is a different act from adding a regression
/// test, and it needs the person who asked for the work to name it — otherwise a change can become
/// "green" by removing what would have failed. The candidate cannot name it for itself.
/// EX11：删掉一条本来就充当判据的测试，与新增一条回归测试是两种行为，需要提出这项工作的人点名——否则一次改动
/// 可以靠删掉本该失败的东西变"绿"。候选不能替自己点名。
#[test]
fn removing_an_existing_test_is_refused_unless_the_host_named_it() {
    let root = fixture();
    let kept = std::fs::read_to_string(root.join("src/lib.rs")).unwrap();
    let refused = super::candidate(
        &root,
        &open_with(
            json!({"allow_removed_tests":["src/lib.rs::the_answer_is_stable"],"scope":["src"]}),
        ),
    )
    .expect_err("a candidate that can allow itself can weaken its own criteria");
    assert!(
        refused.contains("belongs to the host's landing entry"),
        "{refused}"
    );
    let (id, tree) = opened(&root, json!(["src"]));
    std::fs::write(
        tree.join("src/lib.rs"),
        "pub fn answer() -> i64 {\n    41\n}\n\n#[test]\nfn the_answer_is_still_checked() {\n    assert_eq!(answer(), 41);\n}\n",
    )
    .unwrap();
    let error = super::candidate(&root, &json!({"action":"verify","id":id,"face":"default"}))
        .expect_err("a criterion that disappeared is refused");
    assert!(
        error.contains("the_answer_is_stable") && error.contains("removed"),
        "{error}"
    );
    assert_eq!(
        std::fs::read_to_string(root.join("src/lib.rs")).unwrap(),
        kept
    );
    // The person who asked for the work names the removal at the landing entry; then the same change
    // is judged on its merits, and it still carries a test of its own (zero executed tests is never a pass).
    // 提出这项工作的人在落地入口点名这次删除；此后同一项改动按它本身的价值判定，而它本身仍带着一条测试
    // （零测试的运行为远不是通过）。
    assert_eq!(
        confirm(
            &root,
            &id,
            &[
                "--land",
                "--allow-removed-test",
                "src/lib.rs::the_answer_is_stable",
            ],
        ),
        0
    );
    let reply =
        super::candidate(&root, &json!({"action":"verify","id":id,"face":"default"})).unwrap();
    assert!(reply.contains("verification passed"), "{reply}");
    super::candidate(&root, &json!({"action":"land","id":id})).unwrap();
}

/// EX11, the half a removed name does not catch: a test that keeps its name and loses an assertion.
/// "Green" is not a property of a test's name, so a body with fewer things that can fail is weakening
/// even though the name survived.
/// EX11 的另一半，只查名字抓不到：测试名保留、断言被删。绿不是测试名字的属性，因此少了几处会失败东西的函数体
/// 就是削弱，即便名字还在。
#[test]
fn an_existing_test_that_loses_an_assertion_is_refused() {
    let root = fixture();
    let base = "pub fn answer() -> i64 {\n    41\n}\n\n#[test]\nfn the_answer_is_exact() {\n    assert_eq!(answer(), 41);\n    assert!(answer() > 0);\n}\n";
    std::fs::write(root.join("src/lib.rs"), base).unwrap();
    let (id, tree) = opened(&root, json!(["src"]));
    std::fs::write(
        tree.join("src/lib.rs"),
        "pub fn answer() -> i64 {\n    41\n}\n\n#[test]\nfn the_answer_is_exact() {\n    assert_eq!(answer(), 41);\n}\n",
    )
    .unwrap();
    let error = super::candidate(&root, &json!({"action":"verify","id":id,"face":"default"}))
        .expect_err("an assertion that disappeared is a criterion that disappeared");
    assert!(
        error.contains("the_answer_is_exact") && error.contains("lost assertion"),
        "{error}"
    );
    assert_eq!(
        std::fs::read_to_string(root.join("src/lib.rs")).unwrap(),
        base
    );
}

/// The boundary is a property of **one landing**, not of the root. Otherwise a protection recorded
/// for an earlier candidate would silently constrain a later one that the person never protected,
/// which is over-rejection: the legal work of the second candidate would be refused for a decision
/// about the first.
/// 边界属于**一次落地**，不属于整个根。否则为早先候选记下的保护会静默约束后来那个从未被保护过的候选，那就是
/// 过度拒绝：第二份候选的合法工作会因为一个关于第一份的决定而被拒。
#[test]
fn a_boundary_recorded_for_one_candidate_does_not_constrain_another() {
    let root = fixture();
    let base = "pub fn writable_helper() -> i64 {\n    1\n}\n\npub fn protected_base() -> i64 {\n    41\n}\n\n#[test]\nfn the_base_is_stable() {\n    assert_eq!(protected_base(), 41);\n}\n";
    std::fs::write(root.join("src/lib.rs"), base).unwrap();
    let (first, _first_tree) = opened_with(&root, json!({"scope":["src/lib.rs"]}), &["src/lib.rs"]);
    assert_eq!(
        confirm(
            &root,
            &first,
            &["--land", "--protect", "src/lib.rs#protected_base"],
        ),
        0
    );
    // A second candidate, opened after that decision, changes the same function. Its own landing
    // entry says nothing about protection, so the work is legal.
    // 第二份候选在那次决定之后打开，改的是同一个函数。它自己的落地入口什么都没说，所以这项工作是合法的。
    let (second, second_tree) =
        opened_with(&root, json!({"scope":["src/lib.rs"]}), &["src/lib.rs"]);
    std::fs::write(
        second_tree.join("src/lib.rs"),
        "pub fn writable_helper() -> i64 {\n    1\n}\n\npub fn protected_base() -> i64 {\n    42\n}\n\n#[test]\nfn the_base_is_stable() {\n    assert_eq!(protected_base(), 42);\n}\n",
    )
    .unwrap();
    let reply = super::candidate(
        &root,
        &json!({"action":"verify","id":second,"face":"default"}),
    )
    .unwrap();
    assert!(
        reply.contains("verification passed"),
        "another candidate's boundary is not this candidate's: {reply}"
    );
    assert_eq!(confirm(&root, &second, &["--land"]), 0);
    super::candidate(&root, &json!({"action":"land","id":second})).unwrap();
    assert!(
        std::fs::read_to_string(root.join("src/lib.rs"))
            .unwrap()
            .contains("42")
    );
}

/// The build-shaped switches are narrowing too. `no_default_features: true` runs the suite without
/// features, and `workspace: false` runs only the root package; both would let a change be certified
/// by tests that never compiled the module that changed.
/// 构建形状的开关同样是收窄。`no_default_features: true` 跑的是不带特性的那套，`workspace: false`
/// 只跑根包；两者都会让一次改动被"从未编译过被改模块"的测试认证。
#[test]
fn a_verification_cannot_narrow_the_suite_with_build_switches() {
    let root = fixture();
    let (id, _tree) = opened(&root, json!(["src/lib.rs"]));
    let without_features = super::candidate(
        &root,
        &json!({"action":"verify","id":id,"no_default_features":true}),
    );
    assert!(
        without_features
            .unwrap_err()
            .contains("`no_default_features: true`"),
        "no_default_features is a narrower run, not a cheaper one"
    );
    let root_only = super::candidate(&root, &json!({"action":"verify","id":id,"workspace":false}));
    assert!(
        root_only.unwrap_err().contains("`workspace: false`"),
        "workspace:false would certify a member change with the root package's tests"
    );
}

/// R3's other half: a delta whose own test never ran is not covered by a suite that passed
/// somewhere else. The changed file declares a test that the run skips (`#[ignore]`), so the
/// executed set cannot name it — and the verdict must say so instead of `passed`.
/// R3 的另一半：差异自己的测试从未运行，不能被别处的通过覆盖。被改文件声明了一条运行会跳过
/// （`#[ignore]`）的测试，因此已执行集合里没有它——判定必须说出来，而不是 `passed`。
#[test]
fn a_delta_whose_own_test_did_not_run_is_not_passed() {
    let root = fixture();
    let (id, tree) = opened(&root, json!(["src/extra.rs"]));
    std::fs::write(
        tree.join("src/extra.rs"),
        "pub fn extra() -> i64 {\n    7\n}\n\n#[test]\n#[ignore]\nfn extra_is_ignored() {\n    assert_eq!(extra(), 7);\n}\n",
    )
    .unwrap();
    let lib = std::fs::read_to_string(tree.join("src/lib.rs")).unwrap();
    std::fs::write(tree.join("src/lib.rs"), format!("{lib}\npub mod extra;\n")).unwrap();
    let refused =
        super::candidate(&root, &json!({"action":"verify","id":id,"face":"default"})).unwrap_err();
    assert!(
        refused.contains("none of the tests those files declare ran"),
        "the delta's own test was skipped: {refused}"
    );
    let record = super::Record::read_at(&super::candidate_tests::records(&root), &id).unwrap();
    assert_eq!(record.body["verification"]["verdict"], "unknown");
}
