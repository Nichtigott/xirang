//! The plan phase's own pins: it seals obligations and starts nothing.
//! 计划阶段自己的钉子：它封存义务，且什么都不启动。

use super::super::check_plan::check_fixtures::{fixture, plan_action};
use super::super::check_plan::run;
use serde_json::json;

/// BR06: a plan phase says what would run, and starting Cargo is not part of saying that.
/// BR06：计划阶段说的是"将要跑什么"，而启动 Cargo 不属于"说"这件事。
#[test]
fn a_plan_seals_the_obligations_and_runs_nothing() {
    let root = fixture();
    let before = std::fs::read_dir(root.join("target").join("xirang"))
        .map(|rows| rows.count())
        .unwrap_or(0);
    let reply = run(
        &root,
        &json!({"files":["src/lib.rs"],"face":"default","action":"plan"}),
    )
    .unwrap();
    assert!(reply.contains("sealed (not run)"), "{reply}");
    assert!(
        reply.contains("obligations 1"),
        "the plan lists what it would run: {reply}"
    );
    assert!(
        reply.contains("regression [expected passed] execution "),
        "{reply}"
    );
    assert!(
        reply.contains("no cargo test was started and no verdict exists yet"),
        "a table without that line reads as a result: {reply}"
    );
    // Nothing ran, so nothing could have passed: the ledger it just wrote holds no verdict.
    let plan_id = reply
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .expect("the reply names the plan");
    let status = plan_action(&root, &json!({"plan_id": plan_id, "action": "status"})).unwrap();
    assert!(
        status.contains("state unknown"),
        "an unrun plan reads unknown, never green: {status}"
    );
    assert!(!status.contains("state passed"), "{status}");
    // A plan writes its ledger and its obligations; it must not have started a check.
    let after = std::fs::read_dir(root.join("target").join("xirang"))
        .map(|rows| rows.count())
        .unwrap_or(0);
    assert!(
        after >= before,
        "the plan wrote its own record: {before} -> {after}"
    );
    std::fs::remove_dir_all(root).unwrap();
}

/// The action vocabulary is `plan` or absent; anything else is refused by name rather than ignored.
/// 动作词汇只有 `plan` 或省略；其它值被点名拒绝，而不是被忽略。
#[test]
fn an_action_that_is_not_plan_is_refused_by_name() {
    let root = fixture();
    let refused = run(
        &root,
        &json!({"files":["src/lib.rs"],"action":"rewrite-the-tree"}),
    )
    .expect_err("only `plan` is an action here");
    assert!(refused.contains("`plan` or absent"), "{refused}");
    assert!(
        refused.contains("rewrite-the-tree"),
        "it names what it got: {refused}"
    );
    assert!(
        refused.contains("addressed by `plan_id`"),
        "and where the other actions live: {refused}"
    );
    std::fs::remove_dir_all(root).unwrap();
}

/// A plan needs a resolved dependency graph, and it says so instead of resolving it itself.
/// 计划需要一份已解析的依赖图，而它会这么说，而不是自己去解析。
#[test]
fn a_plan_without_a_lockfile_names_the_command_instead_of_running_it() {
    // A package with no lockfile: resolving it is Cargo's job, and that is exactly what a plan
    // phase refuses to do on the caller's behalf.
    // 一个没有锁文件的包：解析它是 Cargo 的活，而计划阶段拒绝代调用方做这件事。
    let root = std::env::temp_dir().join(format!("xirang-plan-phase-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::write(
        root.join("Cargo.toml"),
        "[package]\nname=\"phase-proof\"\nversion=\"0.0.0\"\nedition=\"2024\"\n[workspace]\n",
    )
    .unwrap();
    std::fs::write(
        root.join("src/lib.rs"),
        "#[test]\nfn contract() { assert_eq!(1, 1); }\n",
    )
    .unwrap();
    let reply = run(&root, &json!({"files":["src/lib.rs"],"action":"plan"})).unwrap();
    assert!(
        reply.contains("plan needs a resolved dependency graph first"),
        "{reply}"
    );
    assert!(reply.contains("cargo generate-lockfile"), "{reply}");
    assert!(
        reply.contains("a plan phase does not start Cargo"),
        "{reply}"
    );
    assert!(
        !root.join("Cargo.lock").exists(),
        "the plan must not have resolved it: that is the whole sentence"
    );
    std::fs::remove_dir_all(root).unwrap();
}

/// §5's second book: a retry adds a line rather than replacing one, and `status` reads the task's
/// book rather than only this plan's row.
/// §5 的第二本账：重试是**加**一行而不是替换一行，而 `status` 读的是任务的那本账，不是本计划这一行。
#[test]
fn the_cost_book_counts_a_second_attempt_instead_of_replacing_the_first() {
    let root = fixture();
    let first = run(&root, &json!({"files":["src/lib.rs"],"face":"default"})).unwrap();
    assert!(first.starts_with("obligations passed"), "{first}");
    // A second plan under the **same task** is a second set of compiles; the book is a task's, not
    // a plan's, so both attempts stay on it.
    // 同一**任务**下的第二份计划就是第二批编译；账是任务的而不是计划的，因此两次尝试都留在上面。
    run(&root, &json!({"files":["src/lib.rs"],"face":"default"})).unwrap();
    let key = super::super::check_task::task_key(&json!({"files":["src/lib.rs"],"face":"default"}));
    let status = plan_action(&root, &json!({"plan_id": key, "action": "status"})).unwrap();
    assert!(
        status.contains("execution(s) across"),
        "status has to report what the task spent: {status}"
    );
    assert!(
        !status.contains("cost 0 execution"),
        "and it must not report a book with nothing on it as zero cost: {status}"
    );
    assert!(
        status.contains("append-only"),
        "the line says why a rollback cannot lower it: {status}"
    );
    std::fs::remove_dir_all(root).unwrap();
}

/// A reply whose two halves disagree is a false statement: `resume` renders its header **before**
/// the run, so the state line has to be re-rendered once the run is over.
/// 回复的两半互相打架就是假话：`resume` 的头部在运行**之前**渲染，因此跑完之后那行 `state` 必须重渲染。
#[test]
fn a_resume_reply_never_contradicts_its_own_obligation_list() {
    let root = fixture();
    let sealed = run(&root, &json!({"files":["src/lib.rs"],"action":"plan"})).unwrap();
    let plan = sealed
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .expect("the plan names itself");
    assert!(sealed.contains("no cargo test was started"), "{sealed}");
    let resumed = plan_action(&root, &json!({"plan_id": plan, "action": "resume"})).unwrap();
    assert!(
        resumed.contains(": passed (expected passed)"),
        "the run has to report what it found: {resumed}"
    );
    // The header must **agree with the rows it sits beside**: a row that did not pass means the plan
    // is open, and every row passed means it is passed. Either way it may not still read `unknown`,
    // which is the state a plan has **before** anything runs.
    // 头部必须与它旁边那些行**一致**：有一行没过 ⇒ 计划是 open；全部通过 ⇒ 是 passed。两种情形都**不许**还读作
    // `unknown`，那是**什么都还没跑**时的状态。
    assert!(
        !resumed.contains("state unknown"),
        "a reply that ran its obligations may not still report the pre-run state: {resumed}"
    );
    let all_passed = !resumed.contains(": failed (expected");
    let expected = if all_passed {
        "state passed"
    } else {
        "state open"
    };
    assert!(
        resumed.contains(expected),
        "the header has to agree with the rows beside it (expected `{expected}`): {resumed}"
    );
    std::fs::remove_dir_all(root).unwrap();
}
