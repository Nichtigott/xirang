//! §5's second book: what a **task** has spent, and why the number can only go up.
//! §5 的第二本账：一个**任务**花掉了什么，以及为什么这个数只能增不能减。
//!
//! The first book is semantic (what the final candidate differs from the baseline, which protected
//! entities were touched, how far scope widened) and lives in the scope ledger beside the candidates.
//! This one is cost: compiles, executions, verdicts. They are kept apart because they decay in
//! opposite directions — a rolled-back candidate **lowered** the semantic account's final figure, and
//! that is the point of rolling back, while the compiles it burned are not refunded. Merging them
//! would let a retry look free.
//! 第一本账是语义的（最终候选与基线的差异、碰过哪些受保护实体、范围扩大了多少），住在候选旁边的范围台账里。
//! 这一本是成本：编译、执行、判定。两者分开，是因为它们**朝相反方向**衰减——被回退的候选**降低**了语义账的最终
//! 数字，而回退的意义正在于此；它烧掉的编译却不退。把两本并成一本，会让一次重试看起来免费。

use std::path::Path;

use serde_json::{Value, json};

/// The name of the cost book, beside the plan ledgers it summarises.
/// 成本账的文件名，与它汇总的那些计划台账放在一起。
const COST_FILE: &str = "cost.jsonl";

/// The cost book: what this **task** has spent, across every plan that ever ran under it.
/// 成本账：这个**任务**花掉了什么，跨在它名下跑过的每一份计划。
///
/// §5's second book is append-only for one reason: a failed attempt may be rolled back to an earlier
/// candidate, but the compiles it burned and the seconds it took are not refunded. Erasing them on a
/// retry would make a task that needed five attempts look like one that needed one — and a cost
/// number is only worth reporting if it can only go up.
/// §5 的第二本账只追加，理由只有一个：一次失败的尝试可以被回退到更早的候选，但它烧掉的编译与耗掉的秒数不退。
/// 重试时把它们抹掉，会让一个需要五次尝试的任务看起来只要一次——而一个成本数字只有在**只增不减**时才值得报。
pub(super) fn note_cost(path: &Path, ledger: &Value, obligation: &str, state: &str) {
    let Some(directory) = path.parent() else {
        return;
    };
    let line = json!({
        "task": ledger["task"],
        "plan": ledger["id"],
        "obligation": obligation,
        "state": state,
    });
    // A cost line that cannot be written must not fail the check it describes: the verdict is the
    // answer, the account is a record about it. It is still visible — `status` prints the file it
    // reads, so an absent book reads as an absent book rather than as a zero.
    // 写不下去的成本行不能让它所描述的那次检查失败：判定是答案，账是关于答案的记录。它仍然可见——`status`
    // 会印出它读的是哪个文件，因此"没有账"读作"没有账"，而不是读作 0。
    let _ = crate::mcp::check_task::note(&directory.join(COST_FILE), line);
}

/// What one task has spent so far, read from the append-only book.
/// 一个任务到目前为止花掉了什么，从只追加的账上读。
///
/// The count is **executions**, not verdicts: a retry is a second compile and a second set of
/// seconds, and the whole point of this line is that the reader can tell a task that passed first
/// try from one that passed on the fifth.
/// 计的是**执行次数**而不是判定：重试就是第二次编译、第二段时间，而这一行的全部意义，是让读者分得出"一次就过"
/// 与"第五次才过"。
pub(super) fn cost_line(directory: &Path, task: &str) -> String {
    let path = directory.join(COST_FILE);
    let Ok(text) = std::fs::read_to_string(&path) else {
        return format!(
            "cost unknown: no book at {} (nothing has recorded an execution under this target yet)\n",
            path.display()
        );
    };
    let mut executions = 0;
    let mut not_passed = 0;
    let mut plans: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    for line in text.lines() {
        let Ok(row) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        if row["task"].as_str() != Some(task) {
            continue;
        }
        executions += 1;
        if row["state"].as_str() != Some("passed") {
            not_passed += 1;
        }
        if let Some(plan) = row["plan"].as_str() {
            plans.insert(plan.to_owned());
        }
    }
    format!(
        "cost {executions} execution(s) across {} plan(s) under this task, {not_passed} of them not passed\nbook {}\nthe book is append-only: a retry after a rollback adds a line rather than replacing one\n",
        plans.len(),
        path.display()
    )
}
