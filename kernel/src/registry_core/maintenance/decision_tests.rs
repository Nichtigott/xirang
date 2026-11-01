//! Pins for the one rule decision: polarity, exemptions, authority, scope and unknown.
//! 唯一规则判定的钉子：极性、豁免、授权、作用域与未知。

use super::super::{
    Polarity, RuleCategory, RuleException, RuleOrigin, RuleRecord, RuleScope, RuleState,
};
use super::{AuthorityStanding, PredicateAnswer, RuleOutcome, RuleSubject, conflict_reason};

fn subject() -> RuleSubject {
    RuleSubject {
        roles: vec!["LocalPosition".to_owned()],
        selector: "control.object.button".to_owned(),
        configuration: "base".to_owned(),
        projection: "base".to_owned(),
        object: None,
    }
}

fn rule(id: &str, polarity: Polarity) -> RuleRecord {
    RuleRecord {
        rule_id: id.to_owned(),
        revision: 2,
        category: RuleCategory::Protocol,
        origin: RuleOrigin::AcceptedPolicy,
        authority_ref: Some("policy:control".to_owned()),
        scope: RuleScope {
            roles: vec!["LocalPosition".to_owned()],
            selector: "control.object.button".to_owned(),
            configurations: Vec::new(),
            projection: "base".to_owned(),
        },
        object: None,
        requirement: "provides cap.render".to_owned(),
        polarity,
        expects: Some(("provides".to_owned(), "cap.render".to_owned())),
        exceptions: Vec::new(),
        sources: vec![("policy.txt".to_owned(), "accepted".to_owned())],
        state: RuleState::Accepted,
    }
}

fn current() -> Vec<(String, String)> {
    vec![("policy.txt".to_owned(), "accepted".to_owned())]
}

fn authority() -> AuthorityStanding {
    AuthorityStanding::in_force("policy:control")
}

/// **A `forbid` rule is satisfied exactly when its predicate does not hold.**
/// **`forbid` 规则恰在谓词不成立时被满足。**
#[test]
fn a_forbidden_predicate_is_satisfied_when_it_does_not_hold() {
    let forbid = rule("no-render", Polarity::Forbid);
    assert_eq!(
        forbid.decide(
            &subject(),
            &current(),
            Some(&authority()),
            None,
            Some(&PredicateAnswer::NotSatisfied)
        ),
        RuleOutcome::Satisfied,
        "the object no longer provides the capability, so the prohibition holds"
    );
    assert_eq!(
        forbid.decide(
            &subject(),
            &current(),
            Some(&authority()),
            None,
            Some(&PredicateAnswer::Satisfied)
        ),
        RuleOutcome::Violated,
        "the object provides what the rule forbids, so the rule is violated"
    );
    let require = rule("keep-render", Polarity::Require);
    assert_eq!(
        require.decide(
            &subject(),
            &current(),
            Some(&authority()),
            None,
            Some(&PredicateAnswer::Satisfied)
        ),
        RuleOutcome::Satisfied
    );
    assert_eq!(
        require.decide(
            &subject(),
            &current(),
            Some(&authority()),
            None,
            Some(&PredicateAnswer::NotSatisfied)
        ),
        RuleOutcome::Violated
    );
}

/// **An unknown predicate is never turned into a pass by negation** — for either polarity.
/// **未知谓词绝不因取反变成通过**——两种极性都一样。
#[test]
fn unknown_is_unknown_for_both_polarities() {
    for polarity in [Polarity::Require, Polarity::Forbid] {
        let decided = rule("r", polarity).decide(
            &subject(),
            &current(),
            Some(&authority()),
            None,
            Some(&PredicateAnswer::Unknown(
                "the source did not parse".to_owned(),
            )),
        );
        assert_eq!(
            decided,
            RuleOutcome::Unknown {
                reason: "the source did not parse".to_owned()
            },
            "{polarity:?} must not decide an unknown predicate"
        );
        assert!(!decided.is_satisfied());
    }
    let blind = rule("blind", Polarity::Require).decide(
        &subject(),
        &current(),
        Some(&authority()),
        None,
        None,
    );
    assert!(matches!(blind, RuleOutcome::Unknown { .. }), "{blind:?}");
}

/// **An exception only covers its own scope, and only with an authority.**
/// **例外只覆盖它自己的作用域，且必须带授权。**
#[test]
fn an_exception_exempts_its_scope_and_nothing_else() {
    let mut exempt = rule("keep-render", Polarity::Require);
    exempt.exceptions = vec![
        RuleException {
            scope: "control.object.button".to_owned(),
            reason: "predates the provider".to_owned(),
            authority: "policy:control".to_owned(),
        },
        RuleException {
            scope: "control.object.slider".to_owned(),
            reason: "no authority, so it covers nothing".to_owned(),
            authority: "   ".to_owned(),
        },
    ];
    let decided = exempt.decide(
        &subject(),
        &current(),
        Some(&authority()),
        None,
        Some(&PredicateAnswer::NotSatisfied),
    );
    assert_eq!(
        decided,
        RuleOutcome::Exempt {
            scope: "control.object.button".to_owned(),
            authority: "policy:control".to_owned(),
            reason: "predates the provider".to_owned(),
        }
    );
    assert!(
        !decided.is_satisfied(),
        "an exemption is not proof that the requirement holds"
    );
    let mut unattributed_only = rule("keep-render", Polarity::Require);
    unattributed_only.exceptions = vec![RuleException {
        scope: "control.object.button".to_owned(),
        reason: "nobody attributed this".to_owned(),
        authority: "   ".to_owned(),
    }];
    assert_eq!(
        unattributed_only.decide(
            &subject(),
            &current(),
            Some(&authority()),
            None,
            Some(&PredicateAnswer::NotSatisfied)
        ),
        RuleOutcome::Violated,
        "an unattributed exception covers nothing, even inside its own scope"
    );
}

/// **A rule whose authority lapsed does not constrain**, and says which anchor and why.
/// **授权失效的规则不能约束**，并点名是哪个 anchor、为什么。
#[test]
fn a_lapsed_authority_stops_the_rule_from_constraining() {
    let decided = rule("keep-render", Polarity::Require).decide(
        &subject(),
        &current(),
        Some(&AuthorityStanding::lapsed(
            "policy:control",
            "the bytes moved after the confirmation",
        )),
        None,
        Some(&PredicateAnswer::Satisfied),
    );
    match decided {
        RuleOutcome::NotConstraining { reason } => {
            assert!(reason.contains("policy:control"), "{reason}");
            assert!(reason.contains("no longer in force"), "{reason}");
        }
        other => panic!("a lapsed authority must not constrain: {other:?}"),
    }
    let unresolved = rule("keep-render", Polarity::Require).decide(
        &subject(),
        &current(),
        None,
        None,
        Some(&PredicateAnswer::Satisfied),
    );
    assert!(
        matches!(unresolved, RuleOutcome::NotConstraining { .. }),
        "an unresolved authority cannot be read as constraining: {unresolved:?}"
    );
}

/// **A dimension the caller did not supply is unknown, not "not applicable".**
/// **调用方没有提供的维度是未知，不是"不适用"。**
#[test]
fn missing_context_is_unknown_and_a_mismatch_is_not_applicable() {
    let mut scoped = rule("keep-render", Polarity::Require);
    scoped.scope.configurations = vec!["release".to_owned()];
    let no_configuration = RuleSubject {
        configuration: String::new(),
        ..subject()
    };
    assert!(
        matches!(
            scoped.decide(
                &no_configuration,
                &current(),
                Some(&authority()),
                None,
                Some(&PredicateAnswer::Satisfied)
            ),
            RuleOutcome::Unknown { .. }
        ),
        "a rule accepted for `release` cannot be declared inapplicable when nobody named a configuration"
    );
    let debug = RuleSubject {
        configuration: "debug".to_owned(),
        ..subject()
    };
    assert_eq!(
        scoped.decide(
            &debug,
            &current(),
            Some(&authority()),
            None,
            Some(&PredicateAnswer::Satisfied)
        ),
        RuleOutcome::NotApplicable {
            reason: "the subject is not under a configuration this rule was accepted for",
        }
    );
    let mut other_object = subject();
    other_object.selector = "control.object.slider".to_owned();
    assert!(matches!(
        rule("keep-render", Polarity::Require).decide(
            &other_object,
            &current(),
            Some(&authority()),
            None,
            Some(&PredicateAnswer::Satisfied)
        ),
        RuleOutcome::NotApplicable { .. }
    ));
    let mut no_roles = subject();
    no_roles.roles.clear();
    assert!(matches!(
        rule("keep-render", Polarity::Require).decide(
            &no_roles,
            &current(),
            Some(&authority()),
            None,
            Some(&PredicateAnswer::Satisfied)
        ),
        RuleOutcome::Unknown { .. }
    ));
}

/// **A stale source stops the rule before any predicate is interpreted.**
/// **源变旧时，规则在任何谓词解释之前就停止约束。**
#[test]
fn a_changed_source_stops_the_rule_before_the_predicate() {
    let moved = vec![("policy.txt".to_owned(), "changed".to_owned())];
    let decided = rule("no-render", Polarity::Forbid).decide(
        &subject(),
        &moved,
        Some(&authority()),
        None,
        Some(&PredicateAnswer::NotSatisfied),
    );
    match decided {
        RuleOutcome::NotConstraining { reason } => {
            assert!(reason.contains("changed"), "{reason}");
        }
        other => panic!("a stale rule must not answer met: {other:?}"),
    }
}

/// **A typed contradiction is found even when the two rules word it differently.**
/// **类型化矛盾在两条规则措辞不同时也能被发现。**
#[test]
fn a_typed_contradiction_is_found_despite_different_wording() {
    let require = rule("keep-render", Polarity::Require);
    let mut forbid = rule("no-render", Polarity::Forbid);
    forbid.requirement = "must not provide cap.render".to_owned();
    let reason = conflict_reason(&require, &forbid).expect("the same typed expectation conflicts");
    assert!(
        reason.contains("keep-render") && reason.contains("no-render"),
        "{reason}"
    );
    assert!(require.contradicts(&forbid));

    let mut unrelated = rule("other", Polarity::Forbid);
    unrelated.expects = Some(("provides".to_owned(), "cap.paint".to_owned()));
    unrelated.requirement = "must not provide cap.paint".to_owned();
    assert!(
        !require.contradicts(&unrelated),
        "different typed expectations are not a conflict"
    );
}

/// **A binding is the identity half of applicability: it wins over the reused spelling.**
/// **绑定是适用性的身份那一半：它胜过被复用的拼写。**
#[test]
fn a_bound_rule_asks_the_identity_not_the_spelling() {
    use super::super::{RuleBinding, RuleSubject as Subject};
    use crate::identity::StableFaceId;

    let mut bound = rule("keep-render", Polarity::Require);
    bound.object = Some(RuleBinding {
        stable: StableFaceId::from_name("fixture-provider"),
        path: "root/panel/provider".to_owned(),
        source: "src/panel/provider/provider.rs".to_owned(),
    });
    // A different object now spells the old selector: not applicable, not violated.
    let impostor = Subject {
        selector: "root/panel/provider".to_owned(),
        object: Some(StableFaceId::from_name("somebody-else")),
        ..subject()
    };
    assert_eq!(
        bound.decide(
            &impostor,
            &current(),
            Some(&authority()),
            None,
            Some(&PredicateAnswer::Satisfied)
        ),
        RuleOutcome::NotApplicable {
            reason: "the subject is a different object than the one this rule is bound to",
        }
    );
    // The same object at a new path still applies even if the selector is stale.
    let moved = Subject {
        selector: "root/elsewhere/provider".to_owned(),
        object: Some(StableFaceId::from_name("fixture-provider")),
        ..subject()
    };
    assert_eq!(
        bound.decide(
            &moved,
            &current(),
            Some(&authority()),
            None,
            Some(&PredicateAnswer::Satisfied)
        ),
        RuleOutcome::Satisfied
    );
    // An unresolved identity cannot be decided either way.
    let unknown = Subject {
        selector: "root/panel/provider".to_owned(),
        object: None,
        ..subject()
    };
    assert!(matches!(
        bound.decide(
            &unknown,
            &current(),
            Some(&authority()),
            None,
            Some(&PredicateAnswer::Satisfied)
        ),
        RuleOutcome::Unknown { .. }
    ));
}

/// A rule in a conflict needs a person, and does not answer met even when its predicate holds.
/// 处在冲突中的规则要由人解决，即使谓词成立也不回答 met。
#[test]
fn a_conflicted_rule_needs_review() {
    let decided = rule("keep-render", Polarity::Require).decide(
        &subject(),
        &current(),
        Some(&authority()),
        Some("`keep-render` and `no-render` contradict each other"),
        Some(&PredicateAnswer::Satisfied),
    );
    assert!(
        matches!(decided, RuleOutcome::NeedsReview { .. }),
        "{decided:?}"
    );
}
