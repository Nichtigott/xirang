//! What the **controller** says about one candidate, in the terms a landing would be decided on.
//! 控制器对一棵候选说了什么，用的正是落地会据以判定的那套措辞。
//!
//! Split out of `candidate.rs` when that page reached its code-line budget, and the seam is real: this page reads the
//! authorization and nothing else — it decides nothing, writes nothing, and its verdict is advisory in exactly one way
//! (the other gates are separate lines of the same reply).
//! 从 `candidate.rs` 拆出，是因为那一页到了代码行预算；这条缝也是真的：本页只读授权、别的什么都不做——不做决定、
//! 不写任何东西，而它的结论只在一个意义上成立（其它几道门是同一条回复里的另外几行）。

use std::path::Path;

/// What the controller says about **this** candidate, in the terms a `land` would be decided on.
/// 控制器对**这棵**候选说了什么，用的正是 `land` 会据以判定的那套措辞。
///
/// It answers the authorization question only, and says so: the other gates (verified bytes, base tree unmoved, graft
/// conflicts) are separate lines of the same reply, and "covered" must not be read as "this will land".
/// 它只回答授权这一问，并且写明了这一点：其它几道门（已验证字节、基树未动、graft 冲突）是同一条回复里的另外几行，
/// "覆盖"不许被读成"这下就能落地了"。
pub(super) fn state(root: &Path, record: &super::Record, id: &str) -> String {
    let directory = crate::mcp::controller::directory(root);
    match crate::mcp::controller::active(root) {
        Err(why) => format!(
            "approval unreadable: {why}\nthe controller's store is {}; a land would be refused until it is repaired",
            directory.display()
        ),
        Ok(None) => format!(
            "approval none: nothing is approved for this tree, so a land would be refused (the controller's store is {})\nthe decision is the host's, not a tool's: `xirang-mcp --confirm --root <path> --id <candidate> --land` is the host's confirmation entry, and no tool in this bridge reaches it",
            directory.display()
        ),
        Ok(Some(grant)) => {
            let line = crate::mcp::controller::line(&grant);
            let project =
                crate::mcp::registry::namespace(root).unwrap_or_else(|_| "unknown".to_owned());
            let scope = record.scope();
            let requested = xirang_kernel::maintenance::UseRequest {
                project: &project,
                task: record.body["task"].as_str().unwrap_or("candidate"),
                candidate: id,
                proposal: record.body["proposal"].as_str().unwrap_or("0"),
                revision: record.body["grant_revision"].as_str().unwrap_or("0"),
                operation: "land",
                paths: &scope,
                policy: super::POLICY,
                cost: xirang_kernel::maintenance::Budget {
                    files: super::changed_files(record),
                    executions: Some(1),
                },
            };
            // What this bridge actually compares, said out loud: every dimension the kernel compares is named here, and
            // the two that are provenance say so, because printing "budget executions=8" without this line invites the
            // reader to believe a limit is enforced when the field is only recorded.
            // 这个桥真的比较什么，明说出来：内核比较的每个维度都在这里点名；只作来源信息的那两个也说出来——少了这一行，
            // "budget executions=8" 就会邀请读者相信这里限制着八次执行，而那个字段只是被记录。
            let split = format!(
                "compared on every use: {}\nrecorded as provenance, never compared: {}",
                xirang_kernel::maintenance::GRANT_COMPARED_DIMENSIONS.join(", "),
                xirang_kernel::maintenance::GRANT_RECORDED_DIMENSIONS.join(", ")
            );
            match grant.covers(&requested) {
                xirang_kernel::maintenance::Allowance::Covered => format!(
                    "{line}\n{split}\napproval covers this candidate's declared scope: a land would pass the authorization gate (the other gates are reported separately)"
                ),
                xirang_kernel::maintenance::Allowance::Refused(why) => format!(
                    "{line}\n{split}\napproval does not cover this candidate: {why}\na land would be refused by the authorization gate"
                ),
            }
        }
    }
}
