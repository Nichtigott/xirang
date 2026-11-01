//! Pins for the rule vocabulary: what may constrain, what may not, and what must never win silently.
//! 规则词汇的钉子：什么可以约束、什么不可以，以及什么绝不能静默胜出。

use super::{
    Polarity, RuleCategory, RuleException, RuleOrigin, RuleRecord, RuleScope, RuleState, Standing,
    conflicted,
};

/// A base scope a subject can be checked against.
/// 一个可以拿来核对主体的 base 作用域。
fn scope() -> RuleScope {
    RuleScope {
        roles: vec!["LocalPosition".to_owned()],
        selector: "control.object.button".to_owned(),
        configurations: Vec::new(),
        projection: "base".to_owned(),
    }
}

/// A rule as it is written down before anything accepts it.
/// 一条在任何人接受之前被写下的规则。
fn rule(id: &str, polarity: Polarity, requirement: &str) -> RuleRecord {
    RuleRecord {
        rule_id: id.to_owned(),
        revision: 1,
        category: RuleCategory::Route,
        origin: RuleOrigin::DerivedCandidate,
        authority_ref: None,
        scope: scope(),
        object: None,
        requirement: requirement.to_owned(),
        polarity,
        expects: None,
        exceptions: Vec::new(),
        sources: vec![("src/a.rs".to_owned(), "digest-a".to_owned())],
        state: RuleState::Proposed,
    }
}

/// **A proposal is not a constraint, and a model's word is not an authority.**
/// **提议不是约束，模型的话不是授权。**
#[test]
fn only_an_authority_accepts_a_rule_and_a_candidate_cannot_be_one_yet() {
    let mut candidate = rule("r1", Polarity::Require, "calls the layout provider");
    assert_eq!(
        candidate.standing(&[("src/a.rs".to_owned(), "digest-a".to_owned())]),
        Standing::NotConstraining("a proposal is not a constraint"),
        "a proposal must not constrain before it is accepted"
    );
    let refused = candidate
        .accept("  ", 2)
        .expect_err("an empty authority is not an authority");
    assert!(refused.contains("needs an authority"), "{refused}");
    let too_early = candidate
        .accept("policy:control-v2", 2)
        .expect_err("a derived candidate is not a rule yet");
    assert!(too_early.contains("is not a project rule"), "{too_early}");

    candidate.origin = RuleOrigin::AdoptedSpecimen;
    candidate.accept("policy:control-v2", 2).unwrap();
    assert_eq!(candidate.state, RuleState::Accepted);
    assert_eq!(candidate.revision, 2);
    assert_eq!(
        candidate.standing(&[("src/a.rs".to_owned(), "digest-a".to_owned())]),
        Standing::Constrains
    );
}

/// The memory is allowed to go stale, and a stale rule must not come back as `Met`.
/// 记忆允许变旧，而过期的规则绝不能以 `Met` 回来。
#[test]
fn a_source_that_moved_or_vanished_stops_the_rule_from_constraining() {
    let mut accepted = rule("r1", Polarity::Require, "calls the layout provider");
    accepted.origin = RuleOrigin::AdoptedSpecimen;
    accepted.accept("policy:control-v2", 2).unwrap();
    assert_eq!(
        accepted.standing(&[("src/a.rs".to_owned(), "digest-b".to_owned())]),
        Standing::NotConstraining("a source this rule was accepted against changed")
    );
    assert_eq!(
        accepted.standing(&[]),
        Standing::NotConstraining("a source this rule was accepted against is no longer here"),
        "a missing source counts as changed: the bytes it was judged on are not there to judge it"
    );
    // The bytes coming back restores the acceptance — nobody retired it.
    // 字节回来了，接受就恢复——没有人撤回它。
    let mut marked = accepted.clone();
    marked.state = RuleState::Stale;
    assert_eq!(
        marked.standing(&[("src/a.rs".to_owned(), "digest-a".to_owned())]),
        Standing::Constrains
    );
    // A retired rule is out for good, whatever the bytes say.
    // 撤回的规则就是出局了，字节说什么都一样。
    accepted.retire("superseded by the v3 contract", 3).unwrap();
    assert_eq!(
        accepted.standing(&[("src/a.rs".to_owned(), "digest-a".to_owned())]),
        Standing::NotConstraining("this rule was withdrawn on purpose")
    );
    assert!(
        accepted.retire("", 4).is_err(),
        "a retirement needs a reason"
    );
}

/// §2.2 step 6: a newer revision never silently withdraws an older commitment — both are reported.
/// §2.2 第 6 条：更新的修订从不静默撤回旧的承诺——两条都被报出。
#[test]
fn opposites_over_one_scope_are_both_conflicted_and_neither_wins() {
    let mut requires = rule("r-old", Polarity::Require, "calls the layout provider");
    requires.origin = RuleOrigin::AdoptedSpecimen;
    requires.accept("policy:control-v1", 1).unwrap();
    let mut forbids = rule("r-new", Polarity::Forbid, "calls the layout provider");
    forbids.origin = RuleOrigin::AcceptedPolicy;
    forbids.accept("policy:control-v2", 5).unwrap();

    let found = conflicted(&[requires.clone(), forbids.clone()]);
    assert_eq!(found, vec!["r-new".to_owned(), "r-old".to_owned()]);
    assert!(requires.contradicts(&forbids) && forbids.contradicts(&requires));
    // Same polarity is two obligations, not a contradiction; a different scope is not one either.
    // 同极性是两条义务而不是矛盾；作用域不同同样不是。
    let mut also_requires = forbids.clone();
    also_requires.rule_id = "r-third".to_owned();
    also_requires.polarity = Polarity::Require;
    assert!(!requires.contradicts(&also_requires));
    let mut elsewhere = forbids.clone();
    elsewhere.rule_id = "r-elsewhere".to_owned();
    elsewhere.scope.selector = "control.object.slider".to_owned();
    assert!(!requires.contradicts(&elsewhere));
    // A rule that is not accepted cannot conflict: a proposal has nothing to withdraw.
    // 未被接受的规则无法冲突：提议没有东西可以撤回。
    let proposal = rule("r-proposal", Polarity::Forbid, "calls the layout provider");
    assert!(conflicted(&[requires, proposal]).is_empty());
}

/// An exception exists only inside its own scope and only with an authority.
/// 例外只存在于它自己的作用域里，且必须带授权。
#[test]
fn an_exception_without_an_authority_covers_nothing() {
    let mut accepted = rule("r1", Polarity::Require, "calls the layout provider");
    accepted.origin = RuleOrigin::AdoptedSpecimen;
    accepted.accept("policy:control-v2", 2).unwrap();
    accepted.exceptions = vec![
        RuleException {
            scope: "control.object.button.legacy".to_owned(),
            reason: "the legacy path predates the provider".to_owned(),
            authority: "maintainer:2026-10-16".to_owned(),
        },
        RuleException {
            scope: "control.object.button.scratch".to_owned(),
            reason: "somebody left this here".to_owned(),
            authority: "   ".to_owned(),
        },
    ];
    assert!(
        accepted
            .exception_for("control.object.button.legacy")
            .is_some()
    );
    assert!(
        accepted
            .exception_for("control.object.button.scratch")
            .is_none(),
        "an exception with no authority is a rule that quietly stopped applying"
    );
    assert!(accepted.exception_for("control.object.slider").is_none());
}

/// Applicability names the dimension that refused, so `not_applicable` cannot hide a failure.
/// 适用性点名拒绝的那个维度，因此 `not_applicable` 无法掩盖失败。
#[test]
fn a_scope_that_does_not_cover_a_subject_says_which_dimension_refused() {
    let scope = scope();
    assert_eq!(
        scope.refuses(
            &["LocalPosition".to_owned()],
            "control.object.button",
            "debug",
            "base"
        ),
        None
    );
    assert!(
        scope
            .refuses(
                &["Persisted".to_owned()],
                "control.object.button",
                "debug",
                "base"
            )
            .is_some_and(|why| why.contains("role"))
    );
    assert!(
        scope
            .refuses(
                &["LocalPosition".to_owned()],
                "control.object.slider",
                "debug",
                "base"
            )
            .is_some_and(|why| why.contains("selects"))
    );
    assert!(
        scope
            .refuses(
                &["LocalPosition".to_owned()],
                "control.object.button",
                "debug",
                "graft(button_fast)"
            )
            .is_some_and(|why| why.contains("projection"))
    );
    // An empty roles list means "any role"; an empty configurations list means "any configuration",
    // and the two are different from "the empty role" or "the empty configuration".
    // 空角色清单表示"任何角色"，空配置清单表示"任何配置"，两者与"空角色"或"空配置"不同。
    let mut any = scope.clone();
    any.roles.clear();
    any.configurations.clear();
    assert_eq!(
        any.refuses(&[], "control.object.button", "release", "base"),
        None
    );
    let mut narrowed = scope;
    narrowed.configurations = vec!["debug".to_owned()];
    assert!(
        narrowed
            .refuses(
                &["LocalPosition".to_owned()],
                "control.object.button",
                "release",
                "base"
            )
            .is_some_and(|why| why.contains("configuration"))
    );
}
