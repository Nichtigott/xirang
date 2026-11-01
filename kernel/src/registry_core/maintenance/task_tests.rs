//! Pins for the task rule: five obligations, and "unknown is never complete".
//! 任务规则的钉子：五条义务，以及"判不了永远不算完成"。

use super::{TaskCompletion, TaskObligation, TaskObligationKind, TaskObligationState, TaskVerdict};

/// A set with every kind satisfied.
/// 一个每种都被满足的集合。
fn satisfied_set() -> Vec<TaskObligation> {
    TaskObligationKind::ALL
        .iter()
        .map(|kind| TaskObligation::satisfied(*kind, "fixture evidence"))
        .collect()
}

/// **A missing obligation is refused by name, not silently treated as satisfied.**
/// **缺一条义务按名字被拒，而不是被静默当成满足。**
#[test]
fn a_missing_obligation_is_refused_by_name() {
    let mut obligations = satisfied_set();
    obligations.retain(|obligation| obligation.kind() != TaskObligationKind::Verified);
    let refused = TaskCompletion::new(obligations).expect_err("four obligations are not a task");
    assert!(
        refused.contains("`verified`") && refused.contains("has 0"),
        "the refusal names the missing kind and its count: {refused}"
    );
}

/// **A duplicated obligation is refused the same way.**
/// **重复的义务以同样的方式被拒。**
#[test]
fn a_duplicated_obligation_is_refused() {
    let mut obligations = satisfied_set();
    obligations.push(TaskObligation::satisfied(
        TaskObligationKind::Applied,
        "a second record",
    ));
    let refused = TaskCompletion::new(obligations).expect_err("two applied obligations");
    assert!(
        refused.contains("`applied`") && refused.contains("has 2"),
        "the refusal names the duplicated kind and its count: {refused}"
    );
}

/// **All five satisfied is the only complete set.**
/// **五条全满足是唯一算完成的集合。**
#[test]
fn only_all_five_satisfied_is_complete() {
    let completion = TaskCompletion::new(satisfied_set()).expect("five obligations");
    assert_eq!(completion.verdict(), TaskVerdict::Complete);
    assert!(completion.is_complete());
    assert!(completion.open_obligations().is_empty());
}

/// **An undecidable obligation is never complete, and it is not "incomplete" either.**
/// **判不了的义务永远不算完成，也不被算成"未完成"。**
///
/// The difference matters downstream: a reader who sees `incomplete` goes and fixes the tree, while
/// one who sees `unknown` has to obtain the evidence. Collapsing them would send the wrong one.
/// 这个区别在下游有意义：读到 `incomplete` 的人去修树，而读到 `unknown` 的人要去取证据。把两者合成一个
/// 会把错的那一半派出去。
#[test]
fn an_undecidable_obligation_is_never_complete() {
    let mut obligations = satisfied_set();
    obligations.retain(|obligation| obligation.kind() != TaskObligationKind::Verified);
    obligations.push(TaskObligation::unknown(
        TaskObligationKind::Verified,
        "the closure changed after the run",
    ));
    let completion = TaskCompletion::new(obligations).expect("five obligations");
    assert_eq!(completion.verdict(), TaskVerdict::Unknown);
    assert!(!completion.is_complete());
    assert_eq!(completion.open_obligations().len(), 1);
}

/// **An outstanding obligation outranks an unknown one.**
/// **"已知缺失"压过"判不了"。**
#[test]
fn an_outstanding_obligation_outranks_an_unknown_one() {
    let mut obligations = satisfied_set();
    obligations.retain(|obligation| {
        !matches!(
            obligation.kind(),
            TaskObligationKind::Rules | TaskObligationKind::Verified
        )
    });
    obligations.push(TaskObligation::unknown(
        TaskObligationKind::Verified,
        "the plan is unreadable",
    ));
    obligations.push(TaskObligation::outstanding(
        TaskObligationKind::Rules,
        "rule `keep-render` is violated",
    ));
    let completion = TaskCompletion::new(obligations).expect("five obligations");
    assert_eq!(completion.verdict(), TaskVerdict::Incomplete);
    assert!(!completion.is_complete());
}

/// **The summary prints every kind, satisfied ones included.**
/// **摘要打印每一种，包括满足的。**
#[test]
fn the_summary_prints_every_kind() {
    let completion = TaskCompletion::new(satisfied_set()).expect("five obligations");
    let summary = completion.summary();
    for kind in TaskObligationKind::ALL {
        assert!(
            summary.contains(&format!("{:<12}", kind.label())),
            "the summary names `{}`: {summary}",
            kind.label()
        );
    }
    assert_eq!(
        summary.lines().count(),
        5,
        "one line per obligation: {summary}"
    );
    assert_eq!(
        TaskObligationKind::ALL.len(),
        5,
        "the fixed set is five kinds; adding one means deciding its evidence"
    );
    assert_eq!(
        TaskObligationState::Unknown.label(),
        "unknown",
        "the stable word records survive with"
    );
}
