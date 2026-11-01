//! Pins for the typed verification vocabulary and the policy-scoped completion rule.
//! 类型化验证词汇与政策范围内完成规则的钉子。

use super::{
    Completion, ExecutionIdentity, Receipt, Reuse, UnknownReason, VerificationKind,
    VerificationOutcome, Witness, classify,
};

#[test]
fn timeout_and_cancellation_are_different_typed_reasons() {
    assert_eq!(
        classify(VerificationKind::Tests, true, false, None, 0, 0),
        VerificationOutcome::Unknown(UnknownReason::Timeout)
    );
    assert_eq!(
        classify(VerificationKind::Tests, false, true, None, 0, 0),
        VerificationOutcome::Unknown(UnknownReason::Cancelled)
    );
    assert_eq!(UnknownReason::Timeout.label(), "timeout");
    assert_eq!(UnknownReason::Cancelled.label(), "cancelled");
}

#[test]
fn a_zero_test_run_and_a_compile_error_are_not_a_pass() {
    assert_eq!(
        classify(VerificationKind::Tests, false, false, Some(0), 1, 0),
        VerificationOutcome::Unknown(UnknownReason::ZeroExecuted)
    );
    assert_eq!(
        classify(VerificationKind::Tests, false, false, Some(0), 0, 0),
        VerificationOutcome::Unknown(UnknownReason::NoResultLine)
    );
    assert_eq!(
        classify(VerificationKind::Tests, false, false, Some(101), 0, 0),
        VerificationOutcome::Failed
    );
    assert_eq!(
        classify(VerificationKind::Tests, false, false, Some(0), 1, 1),
        VerificationOutcome::Passed
    );
}

/// A compilation-only run is judged by its exit status and is never forced through the test rules,
/// and its actual (zero) counts are not fabricated.
/// 仅编译的运行只看退出码，绝不硬套测试规则，其实际（零）计数也不被伪造。
#[test]
fn compilation_success_is_typed_apart_from_executed_tests() {
    assert_eq!(
        classify(VerificationKind::Compilation, false, false, Some(0), 0, 0),
        VerificationOutcome::Passed
    );
    assert_eq!(
        classify(VerificationKind::Compilation, false, false, Some(101), 0, 0),
        VerificationOutcome::Failed
    );
    assert_eq!(
        classify(VerificationKind::Compilation, true, false, None, 0, 0),
        VerificationOutcome::Unknown(UnknownReason::Timeout)
    );
    assert_eq!(VerificationKind::Tests.label(), "tests");
    assert_eq!(VerificationKind::Compilation.label(), "compilation");
}

#[test]
fn answered_is_not_applied_is_not_verified_is_not_accepted() {
    let answered = Completion::answered();
    assert!(answered.answered && !answered.applied && !answered.accepted);
    assert!(!answered.verification_closes());

    let applied = Completion::applied();
    assert!(applied.applied && !applied.verification_closes());

    let verified = Completion::verified(VerificationOutcome::Passed);
    assert!(verified.verification_closes());
    assert!(!verified.accepted);

    let accepted = Completion::accepted();
    assert!(accepted.accepted && !accepted.verification_closes());
}

/// Acceptance is demanded only by a policy that says so; the vocabulary adds no blanket gate.
/// 只有明说需要接受的政策才要求接受；词汇本身不额外加门。
#[test]
fn acceptance_is_required_only_when_the_policy_says_so() {
    let applied_and_verified = Completion {
        answered: true,
        applied: true,
        verified: Some(VerificationOutcome::Passed),
        accepted: false,
    };
    assert!(
        applied_and_verified.is_complete(false),
        "an ordinary verified maintenance step needs no new approval gate"
    );
    assert!(
        !applied_and_verified.is_complete(true),
        "a policy that requires acceptance still rejects it"
    );
    let accepted = Completion {
        answered: true,
        applied: true,
        verified: Some(VerificationOutcome::Passed),
        accepted: true,
    };
    assert!(accepted.is_complete(true));
}

#[test]
fn a_historical_pass_never_settles_an_unperformed_operation() {
    let verified_only = Completion {
        answered: true,
        applied: false,
        verified: Some(VerificationOutcome::Passed),
        accepted: true,
    };
    assert!(!verified_only.is_complete(false) && !verified_only.is_complete(true));
    let applied_only = Completion {
        answered: true,
        applied: true,
        verified: None,
        accepted: true,
    };
    assert!(!applied_only.is_complete(false));
    assert!(
        !Completion::verified(VerificationOutcome::Unknown(UnknownReason::Timeout))
            .verification_closes(),
        "an unknown outcome never closes a verification obligation"
    );
}

/// §6.3 as a pin: every one of the five conditions is flipped **alone** and must refuse on its own
/// line, and the fully satisfied receipt must be allowed. Flipping one at a time is what makes this
/// a test of the rule rather than of the happy path: a rule that refused everything would fail the
/// last assertion, and one that allowed everything would fail the others.
/// §6.3 作为一条钉子：五条条件各自**单独**翻转，且必须由它自己那一行拒绝；而完全满足的凭据必须被允许。
/// 一次只翻一条，正是这条测试在测规则而不是在测顺利路径的原因：一条什么都拒绝的规则会在最后一条断言上失败，
/// 而一条什么都允许的会在其余各条上失败。
#[test]
fn each_reuse_condition_refuses_on_its_own_line() {
    let reuse = |receipt: &Receipt, current: &Receipt| receipt.reuse_against(current);
    let identity = ExecutionIdentity::of([
        ("candidate", "c"),
        ("toolchain", "rustc 1.96.0"),
        ("target", "x86_64-unknown-linux-gnu"),
        ("profile", "debug"),
        ("features", "default"),
        ("no_default_features", "false"),
        ("selection", "workspace=true; package=none"),
        ("mode", "test"),
        ("checker", "xirang-toolchain 0.1.0"),
        ("isolation", "in-tree (cargo default)"),
    ])
    .unwrap();
    let base = Receipt {
        obligation: "regression".to_owned(),
        execution: identity.clone(),
        input_manifest: "closure-1".to_owned(),
        oracle_and_policy: "policy-1".to_owned(),
        executed: 3,
        outcome: VerificationOutcome::Passed,
        coverage_confirmed: true,
        anomaly_undisposed: Witness::Known(false),
        scope_expanded: Witness::Known(false),
        published_form_changed: Witness::Known(false),
    };
    assert_eq!(reuse(&base, &base.clone()), Reuse::Allowed);

    let other_identity = ExecutionIdentity::of([
        ("candidate", "c"),
        ("toolchain", "rustc 1.96.0"),
        ("target", "x86_64-unknown-linux-gnu"),
        ("profile", "release"),
        ("features", "default"),
        ("no_default_features", "false"),
        ("selection", "workspace=true; package=none"),
        ("mode", "test"),
        ("checker", "xirang-toolchain 0.1.0"),
        ("isolation", "in-tree (cargo default)"),
    ])
    .unwrap();
    let cases: Vec<(&str, Receipt, &str)> = vec![
        (
            "a different obligation",
            Receipt {
                obligation: "other".to_owned(),
                ..base.clone()
            },
            "different obligation",
        ),
        (
            "a different execution identity",
            Receipt {
                execution: other_identity,
                ..base.clone()
            },
            "execution identity differs",
        ),
        (
            "a different input closure",
            Receipt {
                input_manifest: "closure-2".to_owned(),
                ..base.clone()
            },
            "input closure",
        ),
        (
            "a different oracle or policy",
            Receipt {
                oracle_and_policy: "policy-2".to_owned(),
                ..base.clone()
            },
            "oracle or policy",
        ),
        (
            "zero executed tests",
            Receipt {
                executed: 0,
                ..base.clone()
            },
            "executed no tests",
        ),
        (
            "a run that did not pass",
            Receipt {
                outcome: VerificationOutcome::Unknown(UnknownReason::ZeroExecuted),
                ..base.clone()
            },
            "only a passed run",
        ),
        (
            "assumed coverage",
            Receipt {
                coverage_confirmed: false,
                ..base.clone()
            },
            "assumed rather than confirmed",
        ),
        (
            "an undisposed anomaly",
            Receipt {
                anomaly_undisposed: Witness::Known(true),
                ..base.clone()
            },
            "undisposed anomaly",
        ),
        (
            "a grown scope",
            Receipt {
                scope_expanded: Witness::Known(true),
                ..base.clone()
            },
            "scope",
        ),
        (
            "a published form that changed",
            Receipt {
                published_form_changed: Witness::Known(true),
                ..base.clone()
            },
            "published form",
        ),
    ];
    for (label, receipt, expected) in cases {
        match reuse(&receipt, &base) {
            Reuse::Refused(reason) => assert!(
                reason.contains(expected),
                "{label}: the refusal must name the condition, got `{reason}`"
            ),
            Reuse::Allowed => panic!("{label} must refuse reuse"),
        }
    }
    // The current side matters too: a scope that grew *now* disqualifies a receipt produced before it.
    // 当前那一侧同样算数：**现在**才扩大的范围会让此前产出的凭据失去资格。
    let grown = Receipt {
        scope_expanded: Witness::Known(true),
        ..base.clone()
    };
    assert!(matches!(reuse(&base, &grown), Reuse::Refused(_)));
}

/// The third state: a dimension **nobody observed** is neither `true` nor `false`, and reuse refuses it with a sentence
/// that says what would make it observable — while a dimension somebody did observe as `false` still allows reuse.
/// 第三种状态：**没人观测过**的维度既不是 `true` 也不是 `false`，复用会拒绝它，并给出"怎样才能让它可观测"的句子——而
/// 有人确实观测为 `false` 的维度仍然允许复用。
///
/// This is the ruling's §1.6 in kernel form: don't hardcode `false`, don't turn the unknown into a failure, and don't
/// disable caching wholesale — the last clause is the positive control below.
/// 这就是裁定 §1.6 的内核形式：不要把 `false` 写死，不要把未知改成失败，也不要整体关掉缓存——最后一条由下面的正控制证明。
#[test]
fn an_unobserved_dimension_refuses_reuse_and_says_what_would_make_it_observable() {
    let reuse = |receipt: &Receipt, current: &Receipt| receipt.reuse_against(current);
    let identity = ExecutionIdentity::of([
        ("candidate", "c"),
        ("toolchain", "rustc 1.96.0"),
        ("target", "x86_64-unknown-linux-gnu"),
        ("profile", "debug"),
        ("features", "default"),
        ("no_default_features", "false"),
        ("selection", "workspace=true; package=none"),
        ("mode", "test"),
        ("checker", "xirang-toolchain 0.1.0"),
        ("isolation", "in-tree (cargo default)"),
    ])
    .unwrap();
    let base = Receipt {
        obligation: "regression".to_owned(),
        execution: identity,
        input_manifest: "closure-1".to_owned(),
        oracle_and_policy: "policy-1".to_owned(),
        executed: 3,
        outcome: VerificationOutcome::Passed,
        coverage_confirmed: true,
        anomaly_undisposed: Witness::Known(false),
        scope_expanded: Witness::Known(false),
        published_form_changed: Witness::Known(false),
    };
    assert_eq!(
        reuse(&base, &base.clone()),
        Reuse::Allowed,
        "the legal control: three observed-unchanged dimensions allow reuse"
    );
    // The legal control first: three dimensions somebody observed as unchanged still allow reuse.
    // 先看正控制：三个有人观测为"没变"的维度仍然允许复用。
    for dimension in ["anomaly", "scope", "published form"] {
        let witness = Witness::Unknown("the plan did not record it; rerun to record one");
        let mut receipt = base.clone();
        match dimension {
            "anomaly" => receipt.anomaly_undisposed = witness,
            "scope" => receipt.scope_expanded = witness,
            _ => receipt.published_form_changed = witness,
        }
        match reuse(&receipt, &base) {
            Reuse::Refused(reason) => {
                assert!(
                    reason.contains("nobody observed this dimension"),
                    "{dimension}: the refusal says nobody looked, not that the fact happened: {reason}"
                );
                assert!(
                    reason.contains("rerun to record one"),
                    "{dimension}: and says what would make it observable: {reason}"
                );
            }
            Reuse::Allowed => panic!("{dimension}: an unobserved dimension allowed reuse"),
        }
    }
    // And the unknown is **not** the same as the fact being true: its sentence differs from the `Known(true)` one.
    // 而未知**不**等于"事实成立"：它的句子与 `Known(true)` 的那句不同。
    let happened = Receipt {
        scope_expanded: Witness::Known(true),
        ..base.clone()
    };
    let unobserved = Receipt {
        scope_expanded: Witness::Unknown("nobody looked"),
        ..base.clone()
    };
    let (Reuse::Refused(fact), Reuse::Refused(looked)) =
        (reuse(&happened, &base), reuse(&unobserved, &base))
    else {
        panic!("both sides must refuse");
    };
    assert_ne!(
        fact, looked,
        "`it happened` and `nobody looked` are different sentences"
    );
}

/// **ME04: a credential judged by another checker is not a credential for this one.**
/// **ME04：由另一个检查器判定的凭据，不是这个检查器的凭据。**
///
/// `ExecutionIdentity` already carries the checker, so "the checker changed" is a refusal by
/// construction; this pin names the dimension, so a future refactor that folds it into an opaque
/// "identity" blob cannot silently stop comparing it. Only that one field differs between the two
/// receipts — everything else, including the bytes and the oracle, is identical.
/// `ExecutionIdentity` 已带检查器，"检查器变了"因此在构造上就是拒绝；这条钉子把**那一维**点名，好让将来
/// 把它折进一个不透明的 "identity" 块的重构不能悄悄停止比较它。两份凭据之间只差这一个字段——其余一切
/// （包括字节与判据）都相同。
#[test]
fn a_credential_from_another_checker_is_refused_by_name() {
    let identity = |checker: &str| {
        ExecutionIdentity::of([
            ("candidate", "c"),
            ("toolchain", "rustc 1.96.0"),
            ("target", "x86_64-unknown-linux-gnu"),
            ("profile", "debug"),
            ("features", "default"),
            ("no_default_features", "false"),
            ("selection", "workspace=true; package=none"),
            ("mode", "test"),
            ("checker", checker),
            ("isolation", "in-tree (cargo default)"),
        ])
        .unwrap()
    };
    let receipt = |checker: &str| Receipt {
        obligation: "regression".to_owned(),
        execution: identity(checker),
        input_manifest: "closure-1".to_owned(),
        oracle_and_policy: "policy-1".to_owned(),
        executed: 3,
        outcome: VerificationOutcome::Passed,
        coverage_confirmed: true,
        anomaly_undisposed: Witness::Known(false),
        scope_expanded: Witness::Known(false),
        published_form_changed: Witness::Known(false),
    };
    let stored = receipt("xirang-toolchain 0.1.0");
    let current = receipt("xirang-toolchain 0.2.0");
    match stored.reuse_against(&current) {
        Reuse::Refused(why) => assert!(
            why.contains("checker"),
            "the refusal has to name the dimension that moved: {why}"
        ),
        other => panic!("another checker's credential must be refused, got {other:?}"),
    }
    assert_eq!(
        stored.reuse_against(&receipt("xirang-toolchain 0.1.0")),
        Reuse::Allowed,
        "the same checker, bytes and oracle is still the same credential"
    );
}
