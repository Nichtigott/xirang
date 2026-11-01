//! Pins for the freshness levels: what each policy buys, and that reuse is never spelled
//! as `current`.
//! 新鲜度等级的钉子：每种策略买到了什么，以及"复用"绝不被拼成"当前"。

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use serde_json::json;

use super::{NOT_CHECKED, Policy, REUSE_WINDOW_SECONDS, UNKNOWN, line, policy_from};

/// A directory nothing else in this process shares, so a thread-local verification from
/// one test cannot be read by another.
/// 本进程里没有别的测试共享的目录，因此一个测试的线程内核验不会被另一个读到。
fn scratch(label: &str) -> (PathBuf, PathBuf) {
    static SEQUENCE: AtomicU64 = AtomicU64::new(0);
    let unique = format!(
        "xirang-mcp-freshness-{label}-{}-{}",
        std::process::id(),
        SEQUENCE.fetch_add(1, Ordering::Relaxed)
    );
    let root = std::env::temp_dir().join(unique);
    let out = root.join("target/xirang/out");
    std::fs::create_dir_all(&out).expect("scratch output directory");
    (root, out)
}

#[test]
fn the_window_is_a_declared_constant() {
    assert_eq!(REUSE_WINDOW_SECONDS, 30, "the answer prints this number");
}

#[test]
fn the_argument_names_the_policy_and_a_typo_reads_as_the_default() {
    assert_eq!(policy_from(&json!({"freshness": "verify"})), Policy::Verify);
    assert_eq!(
        policy_from(&json!({"freshness": "reuse-only"})),
        Policy::ReuseOnly
    );
    assert_eq!(policy_from(&json!({"freshness": "reuse"})), Policy::Reuse);
    assert_eq!(policy_from(&json!({"freshness": "verfiy"})), Policy::Reuse);
    assert_eq!(policy_from(&json!({})), Policy::Reuse);
}

#[test]
fn a_verdict_this_answer_paid_for_is_not_spelled_as_a_reuse() {
    // A payment shared between readers of **one** answer is not a reuse, and this was measured as a
    // silent change in what the answer says: sharing the payment turned every roster row of a
    // 50,000-file workspace from `freshness: content-verified at 23:34:22` into
    // `freshness: reused (... 5s ago)` for exactly the same work (audit `T1`, cut 7).
    // 一份答案的读者之间共享的付费不是复用，而这是实测出来的、答案措辞的静默变化：共享付费把
    // 50,000 文件工作区的每一行普查从 `freshness: content-verified at 23:34:22` 变成了
    // `freshness: reused (... 5s ago)`，而做的是同一件工作（审计 `T1` 第七刀）。
    let (root, out) = scratch("paid-here");
    super::set_policy(Policy::Verify);
    let paid = super::verdict(&root, &out);
    assert!(!paid, "an unbuilt tree is not current");
    let same_answer = line(&root, &out);
    assert_eq!(
        same_answer, "build stale (run `xirang check`)",
        "a verdict this answer paid for is reported as what it found, not as a reuse: {same_answer}"
    );
    // A *new* answer may reuse it, and then it must say so and name the moment.
    // **新的一份**答案可以复用它，那时它必须说出来并点名那个时刻。
    super::set_policy(Policy::Reuse);
    let next_answer = line(&root, &out);
    assert!(
        next_answer.contains("reused, content-verified at"),
        "a later answer must label the reuse: {next_answer}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn an_explicit_verify_pays_without_walking_a_stamp_first() {
    // The guard stamp is only worth walking when there is a remembered verdict to validate; with
    // nothing to validate, the walk belongs **inside the payment**, where it can run beside every
    // other member's. Measured at 50,000 files: walking the guard for twenty members up front cost
    // 0.67 s of a 3.7 s answer, all of it on the calling thread (audit `T1`, cut 7).
    // 只有在**有**记忆裁决需要校验时，守卫用的戳才值得走；没有东西要校验时，这次走法属于**付费里面**，
    // 那里它能与其它每个成员的走法并行。在 50,000 文件上实测：开头为二十个成员走守卫花了 3.7 s 答案里的
    // 0.67 s，而全部落在调用线程上（审计 `T1` 第七刀）。
    let (root, out) = scratch("verify-no-walk");
    match super::consult(&root, &out, Policy::Verify) {
        super::Consultation::Pay { stamp: None } => {}
        other => panic!(
            "an explicit verify pays straight away; got {}",
            match other {
                super::Consultation::Pay { stamp: Some(_) } => "a payment that walked a stamp",
                super::Consultation::Remembered(_) => "a remembered verdict",
                super::Consultation::Pay { stamp: None } => unreachable!(),
            }
        ),
    }
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn an_output_with_no_fingerprint_is_stale_and_the_window_labels_the_reuse() {
    // `build_output_is_current` reads the published `discovery.fingerprint` first and
    // answers `false` when it is absent, which is exactly what this scratch directory is:
    // a build that never published, or one whose validation failed and removed the token.
    // `build_output_is_current` 先读已发布的 `discovery.fingerprint`，缺失即答 `false`，
    // 而这个 scratch 目录正是那种情形：从未发布过的构建，或校验失败后凭据被移除的构建。
    let (root, out) = scratch("stale");
    super::set_policy(Policy::Verify);
    let paid = line(&root, &out);
    assert!(
        paid.starts_with("build stale (run `xirang check`)"),
        "a check that runs must report what it found: {paid}"
    );
    super::set_policy(Policy::Reuse);
    let reused = line(&root, &out);
    assert!(
        reused.contains("reused, content-verified at"),
        "reuse must name itself and the moment: {reused}"
    );
    assert!(
        reused.contains(&format!("window {REUSE_WINDOW_SECONDS}s")),
        "reuse must print the window it relied on: {reused}"
    );
    assert!(
        !reused.contains("freshness: content-verified"),
        "a reused verification must never read as a fresh one: {reused}"
    );
}

#[test]
fn reuse_only_never_pays_and_says_so() {
    let (root, out) = scratch("reuse-only");
    super::set_policy(Policy::ReuseOnly);
    assert_eq!(line(&root, &out), UNKNOWN);
}

#[test]
fn a_member_nobody_answered_from_has_a_word_for_that() {
    assert!(NOT_CHECKED.contains("not checked"));
    assert!(
        !NOT_CHECKED.contains("current"),
        "the word for an unread member must not borrow `current`: {NOT_CHECKED}"
    );
}

#[test]
fn the_printed_moment_is_a_utc_clock() {
    let stamp = super::wall_clock();
    let parts: Vec<&str> = stamp.split(':').collect();
    assert_eq!(parts.len(), 3, "HH:MM:SS: {stamp}");
    assert!(parts.iter().all(|part| part.len() == 2), "{stamp}");
}
