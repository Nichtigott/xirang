//! The plan phase: what a plan says about itself before anything runs.
//! 计划阶段：一份计划在**什么都还没跑**之前对自己说了什么。
//!
//! Split out because it is a different question from the plan's own construction: `check_plan.rs`
//! decides *what would be run*, and this module decides *what a caller is told about it without
//! running it*. Keeping the two apart is what makes the promise checkable — a plan phase that could
//! reach the executor would be a plan phase in name only, and the promise it makes ("no cargo test
//! was started") is the line at the bottom of `render_plan`.
//! 拆出来是因为它与计划的构造本身是两个问题：`check_plan.rs` 决定**将要跑什么**，本模块决定**在没跑的情况
//! 下告诉调用方什么**。把两者分开，那句承诺才是可检查的——一个够得着执行器的计划阶段只是名字叫计划阶段，而
//! 它做出的承诺（"没有启动任何 cargo test"）就是 `render_plan` 最后那一行。

use std::path::Path;

use serde_json::Value;

use super::check_plan::Obligation;

/// What a plan phase answers: the obligations it sealed, their execution identities, and what it
/// deliberately did **not** do.
/// 计划阶段回答什么：它封存了哪些义务、各自的执行身份，以及它**有意**没做什么。
///
/// The last line is the point of the phase. A plan that returned a table without saying that nothing
/// ran would be read as a result, and a reader who took an unrun plan for a green one has lost the
/// very thing the plan exists to protect.
/// 最后一行才是这个阶段的意义。一份只给出一张表、却不说"什么都没跑"的计划会被读成结果，而把没跑过的计划
/// 当成绿的读者，丢掉的正是计划存在所保护的那件东西。
pub(super) fn render_plan(
    id: &str,
    path: &Path,
    ledger: &Value,
    executions: &[super::check_execution::ExecutionIdentity],
    requests: &[Obligation],
) -> String {
    let mut output = format!("plan {id} sealed (not run)\nledger {}\n", path.display());
    output.push_str(&format!("obligations {}\n", requests.len()));
    for (position, obligation) in requests.iter().enumerate() {
        let execution = executions
            .get(position)
            .map(|identity| identity.digest()[..16].to_owned())
            .unwrap_or_else(|| "unknown".to_owned());
        output.push_str(&format!(
            "  {} [expected {}] execution {}\n",
            obligation.name, obligation.expected, execution
        ));
    }
    if let Some(rows) = ledger["obligations"].as_array() {
        output.push_str(&format!("duties recorded {}\n", rows.len()));
    }
    if let Some(protection) = ledger.get("protection").filter(|value| !value.is_null()) {
        output.push_str(&format!("protection {protection}\n"));
    }
    output.push_str(
        "no cargo test was started and no verdict exists yet: this is what a run would do. `resume` with this plan id runs it, and only obligations whose inputs and execution configuration still match are reused\n",
    );
    output
}

/// Whether this call asked for the plan and not for the run.
/// 这次调用要的是计划，还是那次运行。
pub(super) fn planning(arguments: &Value) -> bool {
    arguments.get("action").and_then(Value::as_str) == Some("plan")
}

/// The precondition a plan phase reports instead of satisfying it.
/// 计划阶段**报出**而不是**代为满足**的那个前提。
///
/// Resolving the dependency graph is Cargo's job and it starts a process, so a plan that needs it
/// says which command would have run rather than running it. The command is named so the caller is
/// not left to guess it.
/// 解析依赖图是 Cargo 的活，而它会启动一个进程，因此需要它的计划会说出本该跑哪条命令，而不是去跑它。命令被
/// 点名，这样调用方就不必去猜。
pub(super) fn unresolved_lockfile(root: &Path) -> String {
    format!(
        "plan needs a resolved dependency graph first: this plan would have run `cargo generate-lockfile` in {}, and a plan phase does not start Cargo. Run that yourself, then ask for the plan again",
        root.display()
    )
}

#[cfg(test)]
#[path = "check_plan_phase_tests.rs"]
mod check_plan_phase_tests;
