//! The maintenance task: five obligations, one durable record, and closure only when all five hold.
//! 维护任务：五条义务、一份持久记录，以及"五条全部成立才闭合"。
//!
//! The review's P4 asks for one layer that connects the authoring plan, the check ledger, the rule
//! book, the object map and the knowledge updates, and that refuses closure until each of them is
//! disposed. This tool is that layer's execution half: it reads the evidence, records what it saw
//! when the task opened, and recomputes the evidence on every status call instead of trusting a
//! conclusion somebody wrote down. The composition rule itself is the kernel's
//! [`xirang_kernel::maintenance::TaskCompletion`], so a missing or undecidable obligation cannot be
//! read as satisfied by any caller.
//! 复核的 P4 要一层把创作计划、检查台账、规则簿、对象映射与知识更新连起来、并在每一件被处置之前拒绝闭合
//! 的东西。本工具是那一层的执行半边：它读证据、在任务开启时记录它看见的东西，并在每次 status 时**重新计算**
//! 证据，而不是相信别人写下的结论。合成规则本身在内核的 [`xirang_kernel::maintenance::TaskCompletion`] 里，
//! 因此"缺一条"或"判不了"不会被任何调用方读成满足。
//!
//! Two limits are stated rather than left for a reader to discover. **The verification closure is
//! re-captured from the tree**, so a modified input is named; the whole-environment identity cannot be
//! re-derived from a manifest, so an environment variable outside the declared prefixes is outside
//! this check. **The knowledge ledger (`.xirang/`) is not part of that closure**: its changes are the
//! rules and knowledge obligations' business, which snapshot their own revisions and lease versions —
//! including the task record itself, which must not invalidate the verification it is about.
//! 两条限制是写出来的，而不是留给读者发现。**验证闭包从树上重新捕获**，因此改动过的输入会被点名；整个环境
//! 的身份无法从清单重新推导，因此声明前缀之外的环境变量不在这次检查之内。**知识台账（`.xirang/`）不属于
//! 那个闭包**：它的变化是规则与知识义务的事，它们各自快照自己的修订与租约版本——也包括任务记录本身，它
//! 不能去作废它所谈论的那次验证。

use std::path::{Path, PathBuf};

use serde_json::{Value, json};
use xirang_kernel::lexicon::XIRANG_DIR;
use xirang_kernel::maintenance::{
    TaskCompletion, TaskObligationKind, TaskObligationState, TaskVerdict,
};

#[path = "task_evidence.rs"]
mod task_evidence;

#[path = "task_open.rs"]
mod task_open;

#[path = "task_sources.rs"]
mod task_sources;

use task_evidence::short;

/// The whole tool's description, which is also its contract.
/// 整个工具的描述，也是它的契约。
pub(crate) const DESCRIPTION: &str = "A maintenance task's five obligations, one durable record, and \
closure only when every one of them holds. **The five are fixed**: `applied` (the object exists with \
exactly the bytes recorded when the task opened), `verified` (a check plan that passed, whose input \
closure — files and the declared environment prefixes — still matches the tree; the knowledge ledger \
`.xirang/` is excluded from that comparison because the rules/knowledge obligations snapshot it \
themselves), `rules` (every rule that applies is met at the revision the task recorded), `knowledge` \
(every adoption lease those rules rest on is still in force at the recorded fingerprint), and \
`acceptance` (only when this task's policy requires one: the named adoption lease must gain a new \
confirmation line after the task opened — the same provisional lease the rule book uses, which \
records a declared decision, not who made it). `status` recomputes all five from the tree and writes \
nothing; a task that was opened and never closed stays open across sessions, which is the recovery \
story. `open` and `close` preview unless the request says `apply: true`; `close` also needs \
`confirm: true` and writes the closure only after every obligation is satisfied. A changed object, a \
changed closure, a revised or retired rule, a lapsed lease, or a plan that did not pass each keep \
closure refused by name. The whole-environment identity cannot be re-derived from a manifest, so an \
environment variable outside the declared prefixes is outside `verified`; and a task does not force \
an approval gate: `acceptance` is required only when the task says so. `open {{from: \"<closed task>\"}}` inherits what a closed task established: its verification \
plan **only while that plan still holds**, and its rule revisions and lease fingerprints only when \
the tree still agrees — a moved book or lease is re-read and named rather than copied. `resume` \
recomputes and, when the missing evidence is the verification, continues that durable \
check plan (`check {plan_id, action:\"resume\"}`) — the one recovery that is a re-run rather than a \
decision; it never closes a task and never rewrites a closed conclusion. Actions: `open`, `status`, \
`close`, `resume`, `list`.";

/// The tool's input schema.
/// 工具的输入 schema。
pub(crate) fn schema() -> Value {
    json!({"type":"object","properties":{
        "action":{"type":"string","enum":["open","status","close","resume","list"],"description":"`open` records the task and the evidence it rests on; `status` recomputes all five obligations and writes nothing; `close` writes the closure only when all five are satisfied; `list` names every task record."},
        "task":{"type":"string","description":"The task id: letters, digits, `-`, `_`, `.`; it is also the record's file name."},
        "object":{"type":"string","description":"`open`: the object the change is about, as a logical path, module, or 32-digit identity."},
        "verification":{"type":"string","description":"`open`: the `plan <id>` a `xirang.check` run printed. The plan's record must already exist. Omit it when `from` names a closed task whose plan still holds; that plan is then inherited."},
        "from":{"type":"string","description":"`open`: a **closed** task whose established knowledge this one inherits. Its plan is inherited only when it still holds (same closure, every row passed); rule revisions and lease fingerprints are compared with the tree now — equal values count as inherited, moved ones are re-read and named. A task that has not closed is refused."},
        "target_dir":{"type":"string","description":"`open`: the same `target_dir` the check ran with; the plan record is read from there. Omit for `<package>/target`."},
        "acceptance":{"type":"object","description":"`open`: `{required: true, anchor: \"<adoption anchor>\"}` when this task's policy requires an acceptance. Omitted or `required: false` means the task requires none.","properties":{"required":{"type":"boolean"},"anchor":{"type":"string"}}},
        "confirm":{"type":"boolean","description":"`close`: the request has to say so; nothing closes by silence."},
        "apply":{"type":"boolean","description":"`open`/`close`/`resume`: write the record (and, for `resume`, actually continue the check plan). Default false: the reply says what would be written or run."},
        "root":{"type":"string"}
    },"required":["action"]})
}

/// The maintenance task surface: open, status, close, list.
/// 维护任务执行面：open、status、close、list。
pub(crate) fn task(root: &Path, arguments: &Value) -> Result<String, String> {
    let action = arguments
        .get("action")
        .and_then(Value::as_str)
        .ok_or("task requires an action: open, status, close, resume or list")?;
    match action {
        "open" => task_open::open_task(root, arguments),
        "status" => status(root, arguments),
        "close" => close_task(root, arguments),
        "resume" => resume_task(root, arguments),
        "list" => list(root),
        other => Err(format!(
            "task action is open, status, close, resume or list, not `{other}`; nothing was done"
        )),
    }
}

/// The directory every task record lives in.
/// 每条任务记录所在的目录。
fn directory(root: &Path) -> PathBuf {
    root.join(XIRANG_DIR).join("tasks")
}

/// One task's record path.
/// 一条任务的记录路径。
fn record_path(root: &Path, id: &str) -> PathBuf {
    directory(root).join(format!("{id}.json"))
}

/// A task id that is safe as a file name and readable in a reply.
/// 可以作为文件名、也能在回复里读的任务 id。
fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 64
        && id != "."
        && id != ".."
        && id
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || "-_.".contains(character))
}

/// Read one task record.
/// 读一条任务记录。
fn read_record(root: &Path, id: &str) -> Result<Value, String> {
    let path = record_path(root, id);
    let text = std::fs::read_to_string(&path).map_err(|error| {
        format!(
            "no task record `{id}` at {} ({error}); `task {{action:\"list\"}}` names the ones this tree has",
            path.display()
        )
    })?;
    serde_json::from_str(&text).map_err(|error| format!("{}: {error}", path.display()))
}

/// Write one task record, through the same single-file transaction the adoption ledger uses.
/// 写一条任务记录，走与采信台账相同的单文件事务。
fn write_record(root: &Path, id: &str, value: &Value) -> Result<PathBuf, String> {
    let directory = directory(root);
    std::fs::create_dir_all(&directory)
        .map_err(|error| format!("cannot create {}: {error}", directory.display()))?;
    let path = record_path(root, id);
    let bytes = serde_json::to_vec_pretty(value).map_err(|error| error.to_string())?;
    crate::run_method::authoring::write_transaction::write_one(&path, &bytes)?;
    Ok(path)
}

/// The five obligations, recomputed from the tree.
/// 五条义务，从树上重新计算。
fn obligations(root: &Path, record: &Value) -> Result<TaskCompletion, String> {
    task_evidence::obligations(root, record)
}

/// `status`：重新计算五条义务，不写任何东西。
fn status(root: &Path, arguments: &Value) -> Result<String, String> {
    let id = text(arguments, "task")?;
    let record = read_record(root, &id)?;
    let completion = obligations(root, &record)?;
    let state = record["state"].as_str().unwrap_or("open");
    let inherited = record["inherited_from"]
        .as_str()
        .map(|source| format!("inherited from task `{source}`\n"))
        .unwrap_or_default();
    let mut text = format!(
        "task {id}  {state}\nobject {} ({})\nrecord {}\n{inherited}verdict {}\n{}\n",
        record["object"]["path"].as_str().unwrap_or(""),
        short(record["object"]["stable"].as_str().unwrap_or("")),
        record_path(root, &id).display(),
        completion.verdict().label(),
        completion.summary()
    );
    if state == "closed" {
        let closed_at = record["closed"]["at"]
            .as_str()
            .unwrap_or("(no time recorded)");
        text.push_str(&format!(
            "note: this task closed at {closed_at}; the obligations above are recomputed against the \
             tree now, so a change after closure shows up here\n"
        ));
        if completion.verdict() != TaskVerdict::Complete {
            text.push_str(
                "the tree changed after this task closed: reopen a task for the current bytes if that \
                 change needs its own closure\n",
            );
        }
    } else {
        text.push_str(&format!(
            "next   {}",
            match completion.verdict() {
                TaskVerdict::Complete => format!(
                    "all five are satisfied: `task {{action:\"close\",task:\"{id}\",confirm:true,apply:true}}` writes the closure\n"
                ),
                TaskVerdict::Incomplete | TaskVerdict::Unknown => "resolve the obligations above \
                 (fix the tree for `outstanding`, obtain the evidence for `unknown`), then close \
                 again\n"
                    .to_owned(),
            }
        ));
    }
    Ok(text)
}

/// `close`: write the closure, and only when all five obligations are satisfied.
/// `close`：写闭合记录，且只在五条义务全部满足时。
fn close_task(root: &Path, arguments: &Value) -> Result<String, String> {
    let id = text(arguments, "task")?;
    let mut record = read_record(root, &id)?;
    if arguments.get("confirm").and_then(Value::as_bool) != Some(true) {
        return Err(format!(
            "close requires `confirm: true`: a closure is a conclusion somebody stands behind, and \
             nothing closes by silence. Task `{id}` was not changed"
        ));
    }
    if record["state"].as_str() == Some("closed") {
        return Ok(format!(
            "task `{id}` is already closed at {}; a closure is not rewritten — open a new task for \
             the current bytes if a new conclusion is needed\n",
            record["closed"]["at"]
                .as_str()
                .unwrap_or("(no time recorded)")
        ));
    }
    let completion = obligations(root, &record)?;
    let verdict = completion.verdict();
    let summary = completion.summary();
    if verdict != TaskVerdict::Complete {
        return Err(format!(
            "task `{id}` does not close: verdict {}\n{summary}\nNothing was written.\n",
            verdict.label()
        ));
    }
    if arguments.get("apply").and_then(Value::as_bool) != Some(true) {
        return Ok(format!(
            "task `{id}` would close: verdict {}\n{summary}nothing was written (this is the preview; \
             pass `apply: true` with `confirm: true` to write the closure)\n",
            verdict.label()
        ));
    }
    record["state"] = json!("closed");
    record["closed"] = json!({
        "at": crate::mcp::freshness::wall_clock(),
        "verdict": verdict.label(),
        "obligations": completion
            .obligations()
            .iter()
            .map(|obligation| json!({
                "kind": obligation.kind().label(),
                "state": obligation.state().label(),
                "detail": obligation.detail(),
            }))
            .collect::<Vec<_>>(),
    });
    let path = write_record(root, &id, &record)?;
    Ok(format!(
        "task `{id}` closed: verdict {}\n{summary}record {}\n",
        verdict.label(),
        path.display()
    ))
}

/// `resume`: recompute, and continue the check plan when that is the only evidence missing.
/// `resume`：重新计算；当缺的只是验证证据时，继续那份检查计划。
///
/// One recovery in this task is a **re-run** and the rest are **decisions**. A check plan is a
/// durable artifact this repository already knows how to continue, and its own resume re-runs what
/// is unfinished or invalidated — so resuming it is the honest way back from an interrupted session.
/// A changed object, a revised rule book, a lapsed or renewed lease and a required acceptance are
/// not re-runs: nobody can decide them from the request, and a tool that "resumed" them would be
/// writing a conclusion in place of a person.
/// 这条任务里只有一种恢复是**重跑**，其余都是**决定**。检查计划是本仓已经知道如何继续的持久产物，它自己的
/// resume 会重跑未完成或已失效的部分——因此继续它就是中断之后诚实的回程。对象变了、规则簿改了、租约失效或
/// 被续签、以及被要求的接受，都不是重跑：谁也替不了那个决定，而"恢复"了它们的工具，就是在用自己写的结论
/// 顶替一个人。
fn resume_task(root: &Path, arguments: &Value) -> Result<String, String> {
    let id = text(arguments, "task")?;
    let record = read_record(root, &id)?;
    if record["state"].as_str() == Some("closed") {
        return Ok(format!(
            "task `{id}` closed at {}: a closed conclusion is never resumed. Open a new task for the \
             current bytes if they need their own closure\n",
            record["closed"]["at"]
                .as_str()
                .unwrap_or("(no time recorded)")
        ));
    }
    let apply = arguments.get("apply").and_then(Value::as_bool) == Some(true);
    let before = obligations(root, &record)?;
    let before_verdict = before.verdict().label().to_owned();
    let plan_id = record["verification"]["plan"]
        .as_str()
        .unwrap_or("")
        .to_owned();
    let target_dir = record["verification"]["target_dir"].as_str();
    let verification_open = before.obligations().iter().any(|obligation| {
        obligation.kind() == TaskObligationKind::Verified
            && obligation.state() != TaskObligationState::Satisfied
    });
    let plan_path = task_evidence::plan_directory(root, target_dir).join(format!("{plan_id}.json"));
    let acted;
    let mut resumed = false;
    if !verification_open {
        acted = "the verification is satisfied; there is nothing for a resume to re-run".to_owned();
    } else if !plan_path.is_file() {
        acted = format!(
            "the verification plan `{plan_id}` is not on disk at {}; it cannot be resumed — run a new \
             `xirang.check` and open a new task against that plan",
            plan_path.display()
        );
    } else if !apply {
        acted = format!(
            "would resume the check plan `{plan_id}` (this is the dry resume: nothing was run and \
             nothing was written; pass `apply: true` to run what is unfinished or invalidated)"
        );
    } else {
        match crate::mcp::tools::run_tool(
            root,
            "xirang.check",
            &json!({"plan_id": plan_id, "action":"resume", "target_dir": target_dir}),
        ) {
            Ok(report) => {
                resumed = true;
                acted = format!(
                    "resumed the check plan `{plan_id}`\n{}",
                    report
                        .lines()
                        .take(3)
                        .map(|line| format!("  {line}"))
                        .collect::<Vec<_>>()
                        .join("\n")
                );
            }
            Err(error) => {
                acted = format!("the check plan could not be resumed: {error}");
            }
        }
    }
    let after = if resumed {
        obligations(root, &record)?
    } else {
        before
    };
    let after_verdict = after.verdict().label().to_owned();
    if apply {
        let journal = directory(root).join(format!("{id}.resumes.jsonl"));
        crate::mcp::check_task::note(
            &journal,
            json!({
                "at": crate::mcp::freshness::wall_clock(),
                "plan": plan_id,
                "resumed": resumed,
                "verdict_before": before_verdict,
                "verdict_after": after_verdict,
                "action": acted.lines().next().unwrap_or("(nothing)"),
            }),
        )?;
    }
    let mut text = format!("task {id}  open\n");
    if before_verdict != after_verdict {
        text.push_str(&format!("verdict {before_verdict} -> {after_verdict}\n"));
    } else {
        text.push_str(&format!("verdict {after_verdict}\n"));
    }
    text.push_str(&after.summary());
    text.push_str(&format!("resume: {acted}\n"));
    let open = after
        .open_obligations()
        .into_iter()
        .map(|obligation| {
            format!(
                "  {:<12} {:<12} {}\n",
                obligation.kind().label(),
                obligation.state().label(),
                obligation.detail()
            )
        })
        .collect::<String>();
    if !open.is_empty() {
        text.push_str(&format!("still open:\n{open}"));
    }
    if after.is_complete() {
        text.push_str(&format!(
            "next   all five are satisfied: `task {{action:\"close\",task:\"{id}\",confirm:true,apply:true}}` writes the closure (a resume never closes)\n"
        ));
    } else {
        text.push_str(
            "next   resolve what is listed above; a resume re-runs the check plan only — an object's \
             bytes, a rule revision, a lease and an acceptance are decisions, not re-runs\n",
        );
    }
    Ok(text)
}

/// `list`: name every task record with its state and object.
/// `list`：点名每条任务记录及其状态与对象。
fn list(root: &Path) -> Result<String, String> {
    let directory = directory(root);
    let Ok(entries) = std::fs::read_dir(&directory) else {
        return Ok(format!(
            "no task record in {}: `task {{action:\"open\",…}}` records the first one\n",
            directory.display()
        ));
    };
    let mut rows = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|extension| extension.to_str()) != Some("json") {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        let Ok(record) = serde_json::from_str::<Value>(&text) else {
            rows.push(format!("  {} (unreadable)", path.display()));
            continue;
        };
        rows.push(format!(
            "  {:<24} {:<8} {}",
            record["task"].as_str().unwrap_or("(no id)"),
            record["state"].as_str().unwrap_or("(no state)"),
            record["object"]["path"].as_str().unwrap_or("(no object)")
        ));
    }
    rows.sort();
    if rows.is_empty() {
        return Ok(format!(
            "no task record in {}: `task {{action:\"open\",…}}` records the first one\n",
            directory.display()
        ));
    }
    Ok(format!(
        "{} task record(s) in {}\n{}\n",
        rows.len(),
        directory.display(),
        rows.join("\n")
    ))
}

/// One required string argument.
/// 一个必填的字符串参数。
fn text(arguments: &Value, key: &str) -> Result<String, String> {
    arguments
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
        .ok_or_else(|| format!("`{key}` is required and must be a non-empty string"))
}

#[cfg(test)]
#[path = "task_tests.rs"]
mod task_tests;
