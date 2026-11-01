//! Pins for the grant vocabulary: what a forged approval cannot do, and what a real one can.
//! 授权词汇的钉子：伪造的批准做不到什么，真实的批准能做得到什么。
//!
//! The ruling asks for **paired** acceptance (伪造批准被拒绝，合法批准能顺畅完成), and a module that
//! only pins refusals would pass while covering nothing. Every pin below therefore names both sides, or
//! is paired with one that does.
//! 裁定要求**成对**验收（伪造批准被拒绝，合法批准能顺畅完成），而一个只钉拒绝的模块会在一件事都覆盖不了的
//! 情况下通过。因此下面每条钉子都点名两侧，或与点名另一侧的那条成对。

use super::{
    Allowance, Budget, GRANT_CHANNELS, GRANT_COMPARED_DIMENSIONS, GRANT_DIMENSIONS,
    GRANT_OPERATIONS, GRANT_RECORDED_DIMENSIONS, Grant, UseRequest,
};

/// One complete, legal set of dimensions. Every test starts here and breaks one thing.
/// 一份完整、合法的维度集合。每条测试都从这里出发，只破坏一件事。
fn dimensions() -> Vec<(&'static str, String)> {
    let candidates = vec![
        ("approver", "maintainer".to_owned()),
        ("budget", "files=8,executions=8".to_owned()),
        ("candidate", "574196-1-0".to_owned()),
        ("channel", "interactive-cli".to_owned()),
        ("operation", "land".to_owned()),
        ("policy", "xirang.candidate/v1".to_owned()),
        ("project", "example-package".to_owned()),
        ("proposal", "3".to_owned()),
        ("revision", "1".to_owned()),
        ("scope", "src,README.md".to_owned()),
        ("task", "b3c1de9a28d6260f".to_owned()),
    ];
    candidates
}

fn grant() -> Grant {
    Grant::issue(dimensions()).expect("the legal control has to issue")
}

/// One proposed use, with the paths a real landing would write.
/// 一次拟议使用，带上一次真实落地会写的那些路径。
fn use_request<'a>(paths: &'a [String]) -> UseRequest<'a> {
    UseRequest {
        project: "example-package",
        task: "b3c1de9a28d6260f",
        candidate: "574196-1-0",
        proposal: "3",
        revision: "1",
        operation: "land",
        paths,
        policy: "xirang.candidate/v1",
        cost: Budget {
            files: Some(paths.len() as u64),
            executions: Some(1),
        },
    }
}

fn paths(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_owned()).collect()
}

/// The positive control the whole module rests on: a legal grant covers a use inside its scope, and
/// covering **fewer** paths than the scope is still covered — the grant is for the scope, not for one
/// edit inside it.
/// 整个模块赖以成立的正控制：合法授权覆盖它范围之内的使用，而只点名**更少**路径同样被覆盖——授权是给范围的，不是
/// 给范围里某一次编辑的。
#[test]
fn a_legal_grant_covers_a_use_inside_its_scope_and_a_subset_of_it() {
    let grant = grant();
    let whole = paths(&["src/lib.rs", "README.md"]);
    assert_eq!(
        grant.covers(&use_request(&whole)),
        Allowance::Covered,
        "the scope the grant names is covered"
    );
    let one_edit = paths(&["src/lib.rs"]);
    assert!(
        grant.covers(&use_request(&one_edit)).is_covered(),
        "an edit inside the granted scope does not need a second approval"
    );
    let no_change = paths(&[]);
    assert!(
        grant.covers(&use_request(&no_change)).is_covered(),
        "a landing that writes nothing is covered by construction"
    );
}

/// A grant with a dimension left out is **not issued**, and the refusal names every missing one.
/// 缺一个维度的授权**不会被签发**，而拒绝会点名所有缺失的那些。
#[test]
fn a_grant_missing_a_dimension_is_refused_and_the_refusal_names_it() {
    for name in GRANT_DIMENSIONS {
        let incomplete: Vec<(&'static str, String)> = dimensions()
            .into_iter()
            .filter(|(field, _)| field != name)
            .collect();
        let refusal = Grant::issue(incomplete)
            .expect_err("a grant that leaves a dimension out is not issued");
        assert!(
            refusal.contains(name),
            "the refusal has to name `{name}`: {refusal}"
        );
        assert!(
            refusal.contains("missing"),
            "and say what is wrong with it: {refusal}"
        );
    }
}

/// The three values a forged approval would reach for are refused where the grant is issued: a name
/// outside the dimension list, `unknown` as the approver or the channel, and a channel or operation
/// invented rather than approved.
/// 伪造的批准会去够的那三个取值，在签发处就被拒绝：维度清单之外的名字、`unknown` 当批准人或通道、以及被发明出来
/// 而不是被批准的通道或操作。
#[test]
fn a_forged_approval_cannot_be_issued() {
    let mut named = dimensions();
    named.push(("signed_by_somebody", "the agent".to_owned()));
    let refusal = Grant::issue(named).expect_err("an unknown field is not a dimension");
    assert!(
        refusal.contains("signed_by_somebody") && refusal.contains("not a grant dimension"),
        "{refusal}"
    );
    for (field, value) in [
        ("approver", "unknown"),
        ("channel", "unknown"),
        ("channel", "xirang-mcp"),
        ("operation", "delete-everything"),
    ] {
        let mut forged = dimensions();
        for entry in forged.iter_mut() {
            if entry.0 == field {
                entry.1 = value.to_owned();
            }
        }
        let refusal = Grant::issue(forged).expect_err("this cannot be issued");
        assert!(
            refusal.contains(field),
            "the refusal names `{field}`: {refusal}"
        );
    }
    // The two lists are the whole vocabulary, and they are fixed.
    // 这两份清单就是全部词汇，而它们是固定的。
    assert_eq!(
        GRANT_OPERATIONS,
        &["land", "publish", "widen"],
        "a new operation needs its own surface before it needs a grant"
    );
    assert_eq!(
        GRANT_CHANNELS.len(),
        3,
        "the ruling names three channels; a fourth is invented"
    );
}

/// An old approval is refused by name when the thing it was issued for moved: another candidate,
/// another proposal, another revision, another operation, another project, another task.
/// 当被批准的对象移动之后，旧批准会被按名字拒绝：另一个候选、另一份提案、另一个版本、另一个操作、另一个项目、另一个任务。
#[test]
fn an_old_approval_never_vouches_for_another_thing() {
    let grant = grant();
    let inside = paths(&["src/lib.rs"]);
    let moves: [(&str, UseRequest<'_>); 6] = [
        (
            "candidate",
            UseRequest {
                candidate: "574196-2-0",
                ..use_request(&inside)
            },
        ),
        (
            "proposal",
            UseRequest {
                proposal: "4",
                ..use_request(&inside)
            },
        ),
        (
            "revision",
            UseRequest {
                revision: "2",
                ..use_request(&inside)
            },
        ),
        (
            "operation",
            UseRequest {
                operation: "publish",
                ..use_request(&inside)
            },
        ),
        (
            "project",
            UseRequest {
                project: "another-package",
                ..use_request(&inside)
            },
        ),
        (
            "task",
            UseRequest {
                task: "ffffffffffffffff",
                ..use_request(&inside)
            },
        ),
    ];
    for (dimension, request) in moves {
        match grant.covers(&request) {
            Allowance::Refused(reason) => {
                assert!(
                    reason.contains(dimension),
                    "the refusal names the dimension `{dimension}`: {reason}"
                );
                assert!(
                    reason.contains("never vouches"),
                    "and why an approval does not travel: {reason}"
                );
            }
            Allowance::Covered => panic!("`{dimension}` moved and the grant still covered it"),
        }
    }
}

/// One approval can authorise a widening **and** the landing that follows it, because a grant's `operation` is a set
/// — and a set still cannot invent a verb the workspace does not implement.
/// 一次批准可以同时授权一次扩大**与**随后的落地，因为授权的 `operation` 是一个集合——而集合仍然不能发明本工作区没有实现的动词。
#[test]
fn one_approval_can_authorise_a_widening_and_the_landing_that_follows() {
    let mut both = dimensions();
    for entry in both.iter_mut() {
        if entry.0 == "operation" {
            entry.1 = "land,widen".to_owned();
        }
    }
    let grant = Grant::issue(both.clone()).expect("a set of implemented verbs is a value");
    let inside = paths(&["src/lib.rs"]);
    assert!(
        grant
            .covers(&UseRequest {
                operation: "widen",
                ..use_request(&inside)
            })
            .is_covered(),
        "the widening the person approved"
    );
    assert!(
        grant
            .covers(&UseRequest {
                operation: "land",
                ..use_request(&inside)
            })
            .is_covered(),
        "and the landing that follows it, from the same decision"
    );
    // A verb outside the set is still refused, by name.
    // 集合之外的动词仍被按名字拒绝。
    match grant.covers(&UseRequest {
        operation: "publish",
        ..use_request(&inside)
    }) {
        Allowance::Refused(reason) => assert!(
            reason.contains("publish") && reason.contains("land, widen"),
            "the refusal names the operation asked for and the ones granted: {reason}"
        ),
        Allowance::Covered => panic!("an approval for land+widen covered a publish"),
    }
    // And an invented member is refused where the grant is issued, not quietly kept.
    // 而发明出来的成员在签发处就被拒绝，不是被悄悄留着。
    for entry in both.iter_mut() {
        if entry.0 == "operation" {
            entry.1 = "land,delete".to_owned();
        }
    }
    let refusal = Grant::issue(both).expect_err("`delete` is not a verb here");
    assert!(
        refusal.contains("delete") && refusal.contains("not an operation"),
        "{refusal}"
    );
}

/// A **newer** approval covers an older reliance, and an older one never covers a newer reliance: that is the
/// direction a widening needs (the controller re-issues, the candidate keeps its changes) and the direction a stale
/// approval must not take.
/// **更新的**批准覆盖更旧的依赖，更旧的永远不覆盖更新的依赖：那正是扩大需要的方向（控制器重签、候选保留改动），
/// 也是过期批准不许走的方向。
#[test]
fn a_newer_approval_covers_an_older_reliance_but_not_the_reverse() {
    let inside = paths(&["src/lib.rs"]);
    let newer = {
        let mut moved = dimensions();
        for entry in moved.iter_mut() {
            if entry.0 == "revision" {
                entry.1 = "2".to_owned();
            }
        }
        Grant::issue(moved).expect("the re-issued approval")
    };
    let older_reliance = UseRequest {
        revision: "1",
        ..use_request(&inside)
    };
    assert!(
        newer.covers(&older_reliance).is_covered(),
        "a re-issued approval still covers a candidate that relied on the previous revision"
    );
    let newer_reliance = UseRequest {
        revision: "3",
        ..use_request(&inside)
    };
    match grant().covers(&newer_reliance) {
        Allowance::Refused(reason) => assert!(
            reason.contains("revision") && reason.contains("never vouches"),
            "an old approval cannot cover a reliance on a newer one: {reason}"
        ),
        Allowance::Covered => panic!("an old approval covered a newer reliance"),
    }
}

/// The scope comparison has a separator boundary: `src` covers `src/lib.rs` but not `src-other/x.rs`,
/// and a path outside it is refused **by name** so the caller knows what to ask a person for.
/// 范围比较带分隔符边界：`src` 覆盖 `src/lib.rs`，但不覆盖 `src-other/x.rs`；范围之外的路径被**按名字**拒绝，
/// 于是调用方知道该找人批什么。
#[test]
fn a_path_outside_the_scope_is_refused_by_name() {
    let grant = grant();
    let outside = paths(&["src/lib.rs", "src-other/lib.rs"]);
    match grant.covers(&use_request(&outside)) {
        Allowance::Refused(reason) => {
            assert!(
                reason.contains("src-other/lib.rs"),
                "the refusal points at the path, not just at the rule: {reason}"
            );
            assert!(
                reason.contains("outside the approved scope"),
                "and names the rule: {reason}"
            );
            assert!(
                reason.contains("src, README.md"),
                "and shows the scope that was granted: {reason}"
            );
        }
        Allowance::Covered => panic!("a string prefix is not a scope"),
    }
}

/// Paths and dimensions are sets, not orders: the same grant written in another order is the same
/// grant, so a controller restart cannot invalidate approvals by re-sorting them.
/// 路径与维度是集合而不是顺序：同样一份授权换个顺序写还是同一份授权，因此控制器重启不会因为重新排序而让批准失效。
#[test]
fn the_same_dimensions_in_another_order_are_the_same_grant() {
    let mut shuffled = dimensions();
    shuffled.reverse();
    assert_eq!(
        grant().digest(),
        Grant::issue(shuffled)
            .expect("the same dimensions")
            .digest(),
        "the digest is over the bound set, not over the order it was written in"
    );
    assert_ne!(
        grant().digest(),
        {
            let mut moved = dimensions();
            for entry in moved.iter_mut() {
                if entry.0 == "proposal" {
                    entry.1 = "4".to_owned();
                }
            }
            Grant::issue(moved).expect("still a grant").digest()
        },
        "a different proposal is a different grant"
    );
}

/// Two lists that must agree with one: the dimensions `covers` compares, the ones it only records, and
/// `GRANT_DIMENSIONS` itself. Without this pin a new dimension could be added to the contract and land in neither half,
/// which is exactly the "bound but never read" field the contract refuses.
/// 三张清单必须一致：`covers` 比较的、只记录的，以及 `GRANT_DIMENSIONS` 本身。没有这条钉子，一个新维度可以被加进
/// 契约却两边都不落——而那正是契约拒绝的"绑定了却从不读取"的字段。
#[test]
fn the_compared_and_recorded_dimensions_are_disjoint_and_complete() {
    let mut both: Vec<&str> = GRANT_COMPARED_DIMENSIONS
        .iter()
        .chain(GRANT_RECORDED_DIMENSIONS.iter())
        .copied()
        .collect();
    both.sort_unstable();
    let mut all: Vec<&str> = GRANT_DIMENSIONS.to_vec();
    all.sort_unstable();
    assert_eq!(
        both, all,
        "the two halves must cover the contract exactly once"
    );
    assert!(
        GRANT_COMPARED_DIMENSIONS
            .iter()
            .all(|name| !GRANT_RECORDED_DIMENSIONS.contains(name)),
        "a dimension is either compared or recorded, never both"
    );
}

/// 政策版本：按另一版政策签发的授权，不能为按这一版运行的使用背书——而这条以前只被印出来、没有任何东西比较它。
/// Policy version: a grant issued under another version of the policy cannot vouch for a use that runs under this
/// one — and this dimension used to be printed without anything comparing it.
#[test]
fn a_use_under_another_policy_version_is_refused_by_name() {
    let grant = grant();
    let paths = paths(&["src/lib.rs"]);
    let mut request = use_request(&paths);
    request.policy = "xirang.candidate/v2";
    match grant.covers(&request) {
        Allowance::Refused(reason) => assert!(
            reason.contains("policy") && reason.contains("xirang.candidate/v2"),
            "the refusal names the policy dimension and the value the use runs under: {reason}"
        ),
        Allowance::Covered => panic!("a grant under one policy version vouched for another"),
    }
}

/// 预算：请求说出它即将花掉多少，授权说出它批了多少，超出即拒；而未点名的限制不被执行（也就不会被假装成限制）。
/// Budget: the request says what it is about to cost and the grant says how much it approved; more is refused, and a
/// limit that is not named is not enforced — nor printed as if it were.
#[test]
fn a_use_costing_more_than_the_approved_budget_is_refused_with_both_numbers() {
    let grant = grant();
    let paths = paths(&["src/lib.rs"]);
    let mut request = use_request(&paths);
    request.cost = Budget {
        files: Some(9),
        executions: Some(1),
    };
    match grant.covers(&request) {
        Allowance::Refused(reason) => assert!(
            reason.contains("9 file(s)") && reason.contains("covers 8"),
            "the refusal puts the two numbers side by side: {reason}"
        ),
        Allowance::Covered => panic!("a landing cost more files than the approval covered"),
    }
    // The same request inside the budget is covered — a refusal that refuses everything proves nothing.
    // 同一份请求落在预算之内则被覆盖——什么都拒绝的拒绝证明不了什么。
    request.cost = Budget {
        files: Some(8),
        executions: Some(1),
    };
    assert!(grant.covers(&request).is_covered());
    // And a budget the checker cannot read is refused rather than treated as unlimited.
    // 而校验方读不懂的预算会被拒绝，而不是被当作无限。
    let unreadable = Grant::issue(dimensions().into_iter().map(|(name, value)| {
        if name == "budget" {
            ("budget", "executions=lots".to_owned())
        } else {
            (name, value)
        }
    }));
    assert!(
        unreadable.is_err_and(|reason| reason.contains("not a number of executions")),
        "an unreadable budget is refused at issue time"
    );
}
