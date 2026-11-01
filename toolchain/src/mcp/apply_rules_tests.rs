//! Pins for P1: the project's own rules are a **premise of the plan**, not a report afterwards.
//! P1 的钉子：项目自己的规则是计划的**前提**，而不是事后的报告。

use serde_json::json;
use std::path::PathBuf;

use super::apply;

/// A throwaway package with a real registration face, a policy file and a lease in force.
/// 一个带有真实注册面、政策文件与生效租约的一次性包。
fn package(label: &str) -> PathBuf {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let name = format!("mcp-apply-rules-{label}");
    let root = std::env::temp_dir().join(format!("{name}-{}-{sequence}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("src")).expect("package directory");
    std::fs::write(
        root.join("Cargo.toml"),
        format!(
            "[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n[workspace]\n"
        ),
    )
    .expect("manifest");
    std::fs::write(root.join("src/lib.rs"), "// host entry\n").expect("library target");
    std::fs::write(root.join("policy.txt"), "fixture policy version 1\n").expect("policy");
    crate::mcp::apply::apply(
        &root,
        &json!({"action":"add","fields":{"module":"provider","kind":"Provider","provides":"cap.render"},"apply":true}),
    )
    .expect("the provider face is created");
    crate::mcp::adopted::adopted(
        &root,
        &json!({"anchor":"policy","certifies":"fixture policy","evidence":"desk check","verifier":"apply test","reason":"the fixture policy is the authority","files":["policy.txt"],"apply":true,"confirm":true}),
    )
    .expect("the lease is in force");
    root
}

/// Record one accepted rule through the real rule-book entry.
/// 经真实规则簿入口记录一条已接受规则。
fn accept_rule(root: &std::path::Path, id: &str, payload: serde_json::Value) {
    let mut propose = json!({
        "action":"propose","rule_id":id,"category":"protocol","selector":"root/provider",
        "requirement":"retain cap.render","origin":"accepted_policy","sources":["policy.txt"],
        "expects":{"provides":"cap.render"}
    });
    for (key, value) in payload.as_object().cloned().unwrap_or_default() {
        propose[key] = value;
    }
    crate::mcp::rules::rules(root, &propose).expect("the rule is proposed");
    crate::mcp::rules::rules(
        root,
        &json!({"action":"accept","rule_id":id,"authority":"policy","confirm":true}),
    )
    .expect("the rule is accepted");
}

/// **A change that violates an accepted rule is refused, and the project keeps its bytes.**
/// **违反已接受规则的改动被拒绝，项目保留原字节。**
///
/// This is F2 from the review: `apply edit` changed `provides` while an accepted `require` said to
/// keep it, succeeded, and only a later `rules check` reported the violation.
/// 这是审查里的 F2：已接受的 `require` 要求保留某个能力时，`apply edit` 把它改掉仍成功，只有事后
/// `rules check` 才报违规。
#[test]
fn a_change_that_violates_an_accepted_rule_is_refused_and_nothing_is_written() {
    let root = package("refuse");
    accept_rule(&root, "keep-render", json!({}));
    let source = root.join("src/provider/provider.rs");
    let before = std::fs::read_to_string(&source).expect("the face exists");
    let refused = apply(
        &root,
        &json!({"action":"edit","node":"root/provider","fields":{"provides":"cap.changed"},"apply":true}),
    )
    .expect_err("the rule refuses this change");
    assert!(
        refused.contains("refused by the project's own rules"),
        "{refused}"
    );
    assert!(
        refused.contains("keep-render") && refused.contains("VIOLATED"),
        "the refusal names the rule and its verdict: {refused}"
    );
    assert!(
        refused.contains("nothing was written"),
        "the refusal states the project is untouched: {refused}"
    );
    assert_eq!(
        std::fs::read_to_string(&source).expect("the face still exists"),
        before,
        "a refused apply must not rewrite the face"
    );
    // Cleanup is not the assertion: a background index refresh may still be writing into the
    // fixture when the test ends, and `remove_dir_all` can lose that race.
    // 清理不是断言：后台索引刷新可能还在写夹具，`remove_dir_all` 会输掉这个竞态。
    let _ = std::fs::remove_dir_all(root);
}

/// **The preview states the violation before anyone applies, and says apply would refuse.**
/// **预览在任何人应用之前就说出违规，并说明 apply 会拒绝。**
#[test]
fn a_preview_reports_the_violation_and_says_apply_would_refuse() {
    let root = package("preview");
    accept_rule(&root, "keep-render", json!({}));
    let preview = apply(
        &root,
        &json!({"action":"edit","node":"root/provider","fields":{"provides":"cap.changed"}}),
    )
    .expect("the preview runs");
    assert!(preview.contains("action preview"), "{preview}");
    assert!(
        preview.contains("rules (decided against the post-change tree)"),
        "the preview carries the rule decision: {preview}"
    );
    assert!(
        preview.contains("VIOLATED") && preview.contains("would refuse"),
        "the preview must not read as a green light: {preview}"
    );
    // Cleanup is not the assertion: a background index refresh may still be writing into the
    // fixture when the test ends, and `remove_dir_all` can lose that race.
    // 清理不是断言：后台索引刷新可能还在写夹具，`remove_dir_all` 会输掉这个竞态。
    let _ = std::fs::remove_dir_all(root);
}

/// **A change that repairs an existing violation is allowed.**
/// **修复既有违规的改动是允许的。**
///
/// The rule is judged on the **post-change** tree, so a project that is already non-compliant can be
/// fixed through the same gate instead of being locked by it.
/// 规则是对**改动后**的树判定的，因此已经不合规的项目可以经同一道门修复，而不是被这道门锁死。
#[test]
fn a_change_that_repairs_an_existing_violation_is_allowed() {
    let root = package("repair");
    accept_rule(&root, "keep-render", json!({}));
    let source = root.join("src/provider/provider.rs");
    let text = std::fs::read_to_string(&source).expect("the face exists");
    std::fs::write(&source, text.replace("cap.render", "cap.changed"))
        .expect("the fixture is made non-compliant by hand");
    let repaired = apply(
        &root,
        &json!({"action":"edit","node":"root/provider","fields":{"provides":"cap.render"},"apply":true}),
    )
    .expect("repairing an existing violation is allowed");
    assert!(
        repaired.contains("keep-render [revision 2] root/provider: met"),
        "the landed reply states the rule is now met: {repaired}"
    );
    assert!(
        std::fs::read_to_string(&source)
            .expect("the face exists")
            .contains("cap.render"),
        "the repair actually landed"
    );
    // Cleanup is not the assertion: a background index refresh may still be writing into the
    // fixture when the test ends, and `remove_dir_all` can lose that race.
    // 清理不是断言：后台索引刷新可能还在写夹具，`remove_dir_all` 会输掉这个竞态。
    let _ = std::fs::remove_dir_all(root);
}

/// **A preview whose rule premise changed after it is stale, and applying it is refused.**
/// **预览之后规则前提变了的计划已经过期，应用它会被拒绝。**
#[test]
fn a_preview_is_stale_when_the_rules_change_after_it() {
    let root = package("stale");
    accept_rule(&root, "keep-render", json!({}));
    let request =
        json!({"action":"edit","node":"root/provider","fields":{"provides":"cap.render"}});
    apply(&root, &request).expect("the preview runs");
    // The premise moves: the rule is retired after the preview.
    crate::mcp::rules::rules(
        &root,
        &json!({"action":"retire","rule_id":"keep-render","reason":"the fixture changes its premise","confirm":true}),
    )
    .expect("the rule is retired");
    let mut applied = request.clone();
    applied["apply"] = json!(true);
    let stale = apply(&root, &applied).expect_err("the stale preview must not be applied");
    assert!(
        stale.contains("preview this apply follows is stale"),
        "{stale}"
    );
    assert!(
        stale.contains("keep-render"),
        "the refusal names the rule whose premise moved: {stale}"
    );
    // A fresh preview against the current book is applied.
    apply(&root, &request).expect("a fresh preview runs");
    apply(&root, &applied).expect("the fresh preview is applied");
    // Cleanup is not the assertion: a background index refresh may still be writing into the
    // fixture when the test ends, and `remove_dir_all` can lose that race.
    // 清理不是断言：后台索引刷新可能还在写夹具，`remove_dir_all` 会输掉这个竞态。
    let _ = std::fs::remove_dir_all(root);
}

/// **A change the rules say nothing about still reports that the rules were consulted.**
/// **规则没有谈到的改动仍然报告"规则被查过"。**
#[test]
fn a_clean_change_reports_the_rule_verdict_it_passed() {
    let root = package("clean");
    accept_rule(&root, "keep-render", json!({}));
    let reply = apply(
        &root,
        &json!({"action":"edit","node":"root/provider","fields":{"provides":"cap.render"},"apply":true}),
    )
    .expect("the change is allowed");
    assert!(
        reply.contains("rules (decided against the post-change tree)")
            && reply.contains("keep-render [revision 2] root/provider: met"),
        "the reply states the rule that was consulted and its verdict: {reply}"
    );
    // Cleanup is not the assertion: a background index refresh may still be writing into the
    // fixture when the test ends, and `remove_dir_all` can lose that race.
    // 清理不是断言：后台索引刷新可能还在写夹具，`remove_dir_all` 会输掉这个竞态。
    let _ = std::fs::remove_dir_all(root);
}
