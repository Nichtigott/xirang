//! Durable plan actions: read a plan back, decide what is still valid, and continue it.
//! 持久计划动作：把计划读回来、裁定哪些仍然有效、继续执行它。
//!
//! Splitting this out of `check_plan.rs` is not cosmetic: that module *creates* a plan, this
//! one **continues** one. The executor they share lives here because the resuming half is what
//! needs indices, phases and a writer verdict, while creation is a single ordered pass.
//! 从 `check_plan.rs` 拆出来不是装饰：那个模块**创建**计划，本模块**继续**计划。两者共用的执行器住在这里，
//! 因为需要行号、阶段与写者判决的是"继续"这一半，而创建只是一趟有序的通过。
//!
//! Two rules decide everything below. **A pass is evidence only while the inputs it was bound to
//! still hold**, so changed bytes make earlier passes stale rather than keeping them. **An
//! obligation that already landed an effect is never replayed automatically**, because a retry
//! and a first attempt are indistinguishable from the ledger alone.
//! 两条规则决定下面的一切。**只有它被绑定到的输入仍然成立时，一次通过才是证据**，因此字节一变，早先的
//! 通过就成为 stale，而不是被保留。**已经落过副作用的义务绝不自动重放**，因为只看台账分不出"重试"与
//! "第一次"。

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};
use xirang_kernel::maintenance::{ExecutionIdentity, Receipt, Reuse, VerificationOutcome};

use super::check::CheckOutcome;
use super::check_plan::{ExpectedFailure, Obligation, fingerprint, judge, persist};

/// Run the named obligations and record each phase, sharing one loop with `resume`.
/// 执行点名的义务并记录每个阶段；与 `resume` 共用同一个循环。
///
/// `indices[i]` is the ledger row `obligations[i]` writes into, because a resumed plan already
/// carries the rows an earlier run appended and can no longer assume "the i-th request is the
/// i-th row".
/// `indices[i]` 是 `obligations[i]` 写入的台账行号——被恢复的计划已经带着上一轮追加的行，不能再假定
/// "第 i 个请求就是第 i 行"。
///
/// `executions[i]` is that obligation's execution identity. Two obligations sharing one execution is
/// decided by **exact equality of every dimension**, never by a rule that guesses two configurations
/// agree: the first one runs, the later ones reference its result and say so, and every obligation
/// keeps its own row and its own verdict — an execution can be shared, a duty cannot be deleted.
/// `executions[i]` 是那条义务的执行身份。两条义务是否共用一次执行，由**每个维度都完全相等**决定，绝不
/// 由一条猜测两种配置一致的规则决定：第一条真的跑，后面的引用它的结果并写明这一点，而每条义务保留自己的
/// 行与自己的判定——执行可以共用，义务不能删除。
#[allow(clippy::too_many_arguments)]
pub(super) fn execute(
    ledger: &mut Value,
    path: &Path,
    journal: &Path,
    root: &Path,
    identity: &str,
    run: &str,
    excluded: Option<&Path>,
    indices: &[usize],
    obligations: &[Obligation],
    executions: &[ExecutionIdentity],
    targets: &[PathBuf],
) -> Result<bool, String> {
    let mut all_passed = true;
    // One settled **run** per identity, with the obligation that produced it. The run is cached, not
    // the verdict: two obligations sharing an execution still judge it against their own expectation,
    // so a negative control that reuses a regression's run is judged as the negative control it is
    // rather than inheriting "passed".
    // 每个身份一次已结算的**运行**，以及产出它的那条义务。缓存的是**运行**而不是判定：共用一次执行的两条
    // 义务仍各自按自己的预期判定它，因此复用了回归运行的负控制会按它自己的身份被判，而不是继承"passed"。
    let mut settled: BTreeMap<String, (CheckOutcome, String)> = BTreeMap::new();
    for (position, obligation) in obligations.iter().enumerate() {
        let row = indices[position];
        if super::check_jobs::cancelled() {
            all_passed = false;
            break;
        }
        if fingerprint(root, excluded)? != identity {
            all_passed = false;
            break;
        }
        let execution = executions
            .get(position)
            .ok_or("every obligation needs its own execution identity")?;
        let digest = execution.digest();
        let mut stable = true;
        let (state, detail, reused) = match settled.get(&digest) {
            Some((outcome, first)) => (
                judge(&obligation.expected, obligation.expect.as_ref(), outcome).to_owned(),
                super::check::render(&obligation.request, outcome),
                Some(first.clone()),
            ),
            None => {
                ledger["obligations"][row]["state"] = json!("running");
                persist(path, ledger)?;
                note_phase(journal, run, &obligation.name, "running")?;
                // The obligation runs in its own tree when it is a variant: the fault lives in a
                // private copy, and the base tree's own stability is still checked around this call.
                // 义务是变体时，它在自己的树里运行：故障住在私有副本里，而基树自身的稳定性仍在这个调用前后
                // 被检查。
                let target = targets
                    .get(position)
                    .ok_or("every obligation needs the tree it runs in")?;
                let answer = super::check::outcome(target, &obligation.request);
                stable = fingerprint(root, excluded)? == identity;
                match answer {
                    Ok(outcome) => {
                        let state = if stable {
                            judge(&obligation.expected, obligation.expect.as_ref(), &outcome)
                        } else {
                            "unknown"
                        };
                        let failed: Vec<Value> = outcome
                            .observation
                            .failed
                            .iter()
                            .map(|test| json!({"name": test.name, "binary": test.binary}))
                            .collect();
                        ledger["obligations"][row]["outcome"] = json!({
                            "kind": outcome.kind.label(),
                            "verdict": outcome.verdict.label(),
                            "unknown_reason": outcome.verdict.unknown_reason().map(|reason| reason.label()),
                            "results": outcome.observation.results,
                            "executed": outcome.observation.executed,
                            "skipped": outcome.observation.skipped,
                            "failed_tests": failed,
                        });
                        let detail = super::check::render(&obligation.request, &outcome);
                        if stable {
                            settled.insert(digest.clone(), (outcome, obligation.name.clone()));
                        }
                        (state.to_owned(), detail, None)
                    }
                    Err(error) => ("unknown".to_owned(), error, None),
                }
            }
        };
        all_passed &= state == "passed";
        ledger["obligations"][row]["state"] = json!(state);
        super::check_cost::note_cost(path, ledger, &obligation.name, &state);
        // A pass is bound to the exact input set it ran against, so a later resume can tell a
        // current pass from one a crash left behind under older bytes.
        // 一次通过绑定在它实际跑的那一组输入上，因此之后的恢复分得出"当前的通过"与"崩溃留在旧字节下的通过"。
        ledger["obligations"][row]["evidence_identity"] = json!(identity);
        ledger["obligations"][row]["execution"] = json!(execution.render_summary());
        ledger["obligations"][row]["execution_digest"] = json!(digest);
        match &reused {
            Some(first) => {
                ledger["obligations"][row]["satisfied_by"] = json!(first);
                ledger["obligations"][row]["evidence"] = json!(format!(
                    "no run: this obligation's execution identity is identical to `{first}`'s in every dimension, so one execution settles both, and this obligation was judged against its own expectation. {detail}"
                ));
            }
            None => {
                ledger["obligations"][row]["evidence"] = json!(detail);
            }
        }
        persist(path, ledger)?;
        note_phase(
            journal,
            run,
            &obligation.name,
            if reused.is_some() { "reused" } else { &state },
        )?;
        if !stable {
            break;
        }
    }
    Ok(all_passed)
}

/// Record one obligation's transition in the journal.
/// 在 journal 里记录一条义务的状态迁移。
fn note_phase(journal: &Path, run: &str, obligation: &str, state: &str) -> Result<(), String> {
    let mut record = super::check_task::phase(run, "obligation");
    record["obligation"] = json!(obligation);
    record["state"] = json!(state);
    super::check_task::note(journal, record)
}

/// Rebuild one planned obligation from the ledger row it was persisted as.
/// 从一条被持久下来的台账行重建它所记录的计划义务。
pub(super) fn obligation_from_row(row: &Value) -> Option<Obligation> {
    Some(Obligation {
        name: row["name"].as_str()?.to_owned(),
        expected: row["expected"].as_str()?.to_owned(),
        expect: row.get("expect").and_then(|expect| {
            Some(ExpectedFailure {
                test: expect["test"].as_str()?.to_owned(),
                target: expect
                    .get("target")
                    .and_then(Value::as_str)
                    .map(str::to_owned),
                violation: expect["violation"].as_str()?.to_owned(),
            })
        }),
        request: row["request"].clone(),
    })
}

/// What the journal says about the run that wrote it.
/// journal 对写下它的那次运行的判断。
enum Writer {
    /// The last phase is a finished one, so nothing owns the plan.
    /// 最后一个阶段已经结束，因此没有东西拥有这份计划。
    Idle,
    /// Another live process is inside the plan.
    /// 另一个活着的进程正在这份计划里。
    Busy(String),
    /// The last phase says running and the writer is gone (or is this same process).
    /// 最后一个阶段说"正在运行"，而写者已经走了（或者就是本进程）。
    Interrupted(String),
}

/// Read the journal's verdict on its own writer.
/// 从 journal 读出它对自己那位写者的判决。
fn writer_verdict(journal: &Path) -> Writer {
    let Some(tail) = super::check_task::tail(journal) else {
        return Writer::Idle;
    };
    let running = matches!(tail["phase"].as_str(), Some("running" | "obligation"))
        && tail["state"]
            .as_str()
            .is_none_or(|state| state == "running");
    if !running {
        return Writer::Idle;
    }
    let Some((writer, pid)) = super::check_task::writer_of(&tail) else {
        return Writer::Interrupted("the journal names no writer".to_owned());
    };
    // A tail this same process wrote is a run that did not finish (calls are sequential here),
    // never a live peer: an interrupted run, not a busy plan.
    // 本进程自己写下的尾记录是一次没有跑完的运行（这里的调用是串行的），绝不是活着的同伴：它是被打断的
    // 运行，不是忙碌的计划。
    if pid == std::process::id() as u64 {
        return Writer::Interrupted(format!("{writer} (pid {pid}) did not finish"));
    }
    match super::check_task::writer_alive(pid) {
        Some(true) => Writer::Busy(format!("{writer} (pid {pid})")),
        Some(false) => Writer::Interrupted(format!("{writer} (pid {pid}) is gone")),
        None => Writer::Interrupted(format!(
            "{writer} (pid {pid}); this platform cannot tell whether it is alive"
        )),
    }
}

/// The durable plan a `plan_id` names: resolve it to the ledger on disk.
/// `plan_id` 命名的那份持久计划：解析成磁盘上的台账。
fn resolve_plan(directory: &Path, plan_id: &str) -> Result<PathBuf, String> {
    let direct = directory.join(format!("{plan_id}.json"));
    if direct.is_file() {
        return Ok(direct);
    }
    if super::check_task::looks_like_plan_id(plan_id) {
        return Err(format!(
            "no plan named `{plan_id}` under {}",
            directory.display()
        ));
    }
    super::check_task::newest_for_task(directory, plan_id).ok_or_else(|| {
        format!(
            "no plan under {} carries the task key `{plan_id}`; run `check {{files:[...]}}` first",
            directory.display()
        )
    })
}

/// A recent passing plan whose executions cover exactly the ones this call would run.
/// 一份较近的通过计划，它的执行恰好覆盖这次调用本来会跑的那些。
pub(super) struct Reusable {
    /// The plan id the reply names.
    /// 回复点名的计划 id。
    pub(super) id: String,
    /// The ledger it was read from.
    /// 读出它的那份台账。
    pub(super) record: PathBuf,
    /// How long ago it was written, in whole seconds.
    /// 它是多少秒之前写下的。
    pub(super) age_seconds: u64,
    /// Per requested execution, the obligation in that plan which ran it.
    /// 每个被请求的执行，在该计划里跑它的那条义务。
    pub(super) covered_by: Vec<String>,
}

/// Find a plan that may be referenced instead of running Cargo again, or answer that none may.
/// 找出一份可以引用、而不必再跑一次 Cargo 的计划；没有可引用的就如实回答。
///
/// Reuse is decided by **equality of the recorded facts**, never by "nothing looks like it changed":
/// the plan's own `source_configuration` must equal this call's input closure, and every requested
/// execution identity must appear as a **passed** row in it, dimension for dimension. A plan whose
/// pass came from a different toolchain, target, profile, feature face, mode, selection or output
/// directory is not a candidate at all (BR05). A caller that does not ask for reuse never reaches
/// this function, so the `files` branch keeps executing by default and no existing promise narrows.
/// 复用由**记录事实的相等**决定，绝不由"看起来什么都没变"决定：计划自己的 `source_configuration` 必须等于
/// 这次调用的输入闭包，且每个被请求的执行身份都必须在它里面作为一条**已通过**的行、逐维度地出现。通过来自
/// 另一条工具链、目标、profile、特性面、模式、选择或输出目录的计划根本不是候选（BR05）。不请求复用的调用
/// 根本到不了这个函数，因此 `files` 分支默认照旧执行，既有承诺一点没窄。
pub(super) fn reusable_plan(
    root: &Path,
    directory: &Path,
    task_key: &str,
    candidate: &str,
    executions: &[ExecutionIdentity],
    environment: &super::check_execution::Environment,
) -> Result<Option<Reusable>, String> {
    let Some(record) = super::check_task::newest_for_task(directory, task_key) else {
        return Ok(None);
    };
    let Ok(text) = std::fs::read_to_string(&record) else {
        return Ok(None);
    };
    let Ok(ledger) = serde_json::from_str::<Value>(&text) else {
        return Ok(None);
    };
    if ledger["state"].as_str() != Some("passed") {
        return Ok(None);
    }
    if ledger["source_configuration"].as_str() != Some(candidate) {
        return Ok(None);
    }
    let rows = ledger["obligations"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let policy = ledger["policy"].as_str().unwrap_or("unknown-policy");
    let mut covered_by = Vec::new();
    for execution in executions {
        let rendered = execution.render_summary();
        let Some(row) = rows
            .iter()
            .find(|row| row["execution"].as_str() == Some(rendered.as_str()))
        else {
            return Ok(None);
        };
        // The stored receipt is **re-derived** from the row, not read back as a verdict: its
        // execution identity comes from the request this machine would run now, so a toolchain,
        // profile or target that changed since the plan was written refuses the reuse by itself.
        // 存量凭据是从那一行**重新推导**出来的，而不是当作判定读回：它的执行身份来自这台机器**现在**会跑的那个
        // 请求，因此计划写下之后变过的工具链、profile 或目标会自己否掉这次复用。
        let Some(obligation) = obligation_from_row(row) else {
            return Ok(None);
        };
        let target = row["variant_root"]
            .as_str()
            .map_or_else(|| root_for(&ledger).unwrap_or_default(), PathBuf::from);
        let stored = Receipt {
            obligation: obligation.name.clone(),
            execution: super::check_execution::execution_identity(
                &obligation.request,
                row["candidate"].as_str().unwrap_or(candidate),
                environment,
                &target,
            )?,
            input_manifest: row["candidate"].as_str().unwrap_or(candidate).to_owned(),
            oracle_and_policy: policy.to_owned(),
            executed: settled_count(&rows, row),
            outcome: settled_outcome(&rows, row),
            coverage_confirmed: settled_results(&rows, row) > 0,
            // The anomaly dimension is read from what the run **recorded**, not assumed: a row whose observation carries
            // skip counts that are all zero and a complete attribution is an observation that the run left nothing
            // undisposed. A row written before those counts were recorded cannot say that, and says so.
            // 异常维度读自这次运行**记录下的东西**，不是假定：某一行的观测带着全为零的跳过计数、归属也完整，那才是一次
            // "运行没有留下未处置之物"的观测。这些计数被记录之前写下的行说不出这句话，于是它就说自己说不出。
            anomaly_undisposed: super::check_witness::anomaly(&rows, row),
            scope_expanded: super::check_witness::coverage(&ledger, root),
            published_form_changed: super::check_witness::published(&ledger, root),
        };
        let current = Receipt {
            obligation: obligation.name,
            execution: execution.clone(),
            input_manifest: candidate.to_owned(),
            oracle_and_policy: super::check_execution::VERIFICATION_POLICY.to_owned(),
            // The current side states what this call would establish if it ran: one execution that
            // must produce at least one result line.
            // 当前这一侧陈述的是：若这次调用真的跑了，它要确立什么——一次必须产出至少一条结果行的执行。
            executed: 1,
            outcome: VerificationOutcome::Passed,
            coverage_confirmed: true,
            // The current side states what a **fresh** run would establish: nothing is attached to it yet, and it would
            // be run against the tree as it is now.
            // 当前这一侧陈述的是**一次新运行**会确立什么：还没有东西挂到它上面，而它会在眼前的这棵树上跑。
            anomaly_undisposed: xirang_kernel::maintenance::Witness::Known(false),
            scope_expanded: xirang_kernel::maintenance::Witness::Known(false),
            published_form_changed: xirang_kernel::maintenance::Witness::Known(false),
        };
        match stored.reuse_against(&current) {
            Reuse::Allowed => covered_by.push(stored.obligation.clone()),
            Reuse::Refused(_) => return Ok(None),
        }
    }
    let age_seconds = std::fs::metadata(&record)
        .and_then(|metadata| metadata.modified())
        .ok()
        .and_then(|modified| modified.elapsed().ok())
        .map(|elapsed| elapsed.as_secs())
        .unwrap_or(0);
    Ok(Some(Reusable {
        id: ledger["id"].as_str().unwrap_or_default().to_owned(),
        record,
        age_seconds,
        covered_by,
    }))
}

/// `check {plan_id, action}`: read a durable plan back, or continue one that was interrupted.
/// `check {plan_id, action}`：把一份持久计划读回来，或继续一份被打断的计划。
///
/// `status` writes nothing at all. `resume` re-runs only what is unfinished or invalidated,
/// and refuses while another live process owns the plan — that is the two-writer rule.
/// `status` 什么都不写。`resume` 只重跑未完成或已失效的部分，而在另一个活进程拥有这份计划时拒绝——
/// 这就是两写者规则。
pub(super) fn plan_action(root: &Path, arguments: &Value) -> Result<String, String> {
    let plan_id = arguments
        .get("plan_id")
        .and_then(Value::as_str)
        .ok_or("plan_id is required")?;
    let action = arguments
        .get("action")
        .and_then(Value::as_str)
        .unwrap_or("status");
    if !matches!(action, "status" | "resume") {
        return Err(format!(
            "a plan action is `status` or `resume`, not `{action}`; nothing was run"
        ));
    }
    let excluded = arguments
        .get("target_dir")
        .and_then(Value::as_str)
        .map(|path| root.join(path));
    let directory = excluded
        .clone()
        .unwrap_or_else(|| root.join("target"))
        .join("xirang/verification");
    let ledger_path = resolve_plan(&directory, plan_id)?;
    let mut ledger: Value =
        serde_json::from_str(&std::fs::read_to_string(&ledger_path).map_err(|e| e.to_string())?)
            .map_err(|error| format!("{}: {error}", ledger_path.display()))?;
    let id = ledger["id"].as_str().unwrap_or_default().to_owned();
    let files = super::check_task::files(&directory, &id);
    let recorded = ledger["source_configuration"]
        .as_str()
        .unwrap_or_default()
        .to_owned();
    let current = super::check_task::Inputs::capture(root, excluded.as_deref())?;
    // The verification identity, environment included: the verdict being reused was computed under a
    // whole environment, and reusing it under another one is the false green the environment dimension
    // exists to prevent.
    // 验证身份，含环境：被复用的判定是在一整个环境下算出来的，而在另一个环境下复用它，正是环境这一维存在的目的
    // 所要防的那次假绿。
    let identity = current.verification_digest();
    // An incomplete closure cannot support reuse: unknown inputs are reported, never dropped (BR-A). This is the
    // second half of the fix — binding more inputs makes a *changed* input visible, and refusing an unbound one
    // keeps a closure that is still incomplete from being presented as if it were whole.
    // 不完整的闭包支撑不了复用：未知输入要被报出来，绝不丢掉（BR-A）。这是修复的另一半——绑更多输入让**变化过的**
    // 输入可见，而拒绝一份仍然不完整的闭包，则不让它被当作完整来呈现。
    let incomplete = current.incomplete_reason().map(str::to_owned);
    let writer = writer_verdict(&files.journal);
    let inputs_match = identity == recorded && incomplete.is_none();
    let environment = super::check_execution::Environment::observe();
    let mut report = format!(
        "plan {id}\ntask {}\nrecord {}\nstate {}\nsource_configuration {recorded}\n{}",
        ledger["task"].as_str().unwrap_or("unknown"),
        ledger_path.display(),
        ledger["state"].as_str().unwrap_or("unknown"),
        super::check_cost::cost_line(&directory, ledger["task"].as_str().unwrap_or("unknown")),
    );
    if let Some(reason) = &incomplete {
        report.push_str(&format!(
            "inputs not fully bound, so no verdict is reusable: {reason}\n"
        ));
    }
    if inputs_match {
        report.push_str(&format!(
            "inputs unchanged: {identity} ({} file(s))\n",
            current.count()
        ));
    } else {
        report.push_str(&format!("inputs changed: {recorded} -> {identity}\n"));
        let changes = super::check_task::Inputs::read(&files.inputs)
            .map(|before| before.changes(&current))
            .unwrap_or_else(|error| vec![format!("manifest unreadable: {error}")]);
        for change in changes.iter().take(40) {
            report.push_str(&format!("  {change}\n"));
        }
        if changes.len() > 40 {
            report.push_str(&format!("  {} more changed inputs\n", changes.len() - 40));
        }
        // A closure that moved with **no** changed path named is the environment: every variable is bound
        // into the verification identity, while the manifest spells out only the declared prefixes. Saying
        // so is the difference between "the inputs changed" and a reader concluding the diff is broken.
        // 闭包移动了、却**没有**点名任何改动路径，那就是环境：每个变量都被绑进验证身份，而清单只写出声明过的
        // 前缀。把这件事说出来，是"输入变了"与读者以为差异算坏了两者之间的区别。
        if changes.is_empty() {
            report.push_str(
                "  the manifest names no changed path: an environment variable moved, and every variable is \n  \
                 part of the verification identity (only the declared prefixes are spelled out above)\n",
            );
        }
    }
    report.push_str(&match &writer {
        // The journal line below says where it stopped, so this one only needs to say that nobody
        // holds the plan — claiming "finished" here was wrong for a record that stops at `planned`.
        // 下面那一行会说出它停在哪，因此这里只需说明没有持有者——对一份停在 `planned` 的记录说"已结束"
        // 是错的。
        Writer::Idle => "writer none\n".to_owned(),
        Writer::Busy(who) => format!("writer busy: {who} is still inside this plan\n"),
        Writer::Interrupted(why) => format!("writer interrupted: {why}\n"),
    });
    // A19: the record locates the phase it stopped at, whatever that phase is.
    // A19：记录说出它停在哪个阶段，无论那是哪个阶段。
    if let Some(tail) = super::check_task::tail(&files.journal) {
        match tail["state"].as_str() {
            Some(state) => report.push_str(&format!(
                "journal stops at {}/{state}\n",
                tail["phase"].as_str().unwrap_or("unknown")
            )),
            None => report.push_str(&format!(
                "journal stops at {}\n",
                tail["phase"].as_str().unwrap_or("unknown")
            )),
        }
    }
    if action == "status" {
        report.push_str(&obligation_table(&ledger, &identity, &environment, root));
        report.push_str(
            "\nstatus only reads: nothing was run and nothing was written. \
             `check {plan_id, action:\"resume\"}` runs what is unfinished or invalidated\n",
        );
        return Ok(report);
    }
    if let Writer::Busy(who) = &writer {
        return Err(format!(
            "{who} is already running this plan; a second writer would interleave obligations. \
             Wait for it, or read the plan with action:\"status\""
        ));
    }
    let summary = resume(
        root,
        &mut ledger,
        &files,
        excluded.as_deref(),
        &identity,
        &id,
        inputs_match,
    )?;
    // `report` was rendered **before** the run, so its `state` line is the state this call started
    // from. Beside a summary that says every obligation passed, that reads as a contradiction
    // ("unknown" next to three "passed"), and a reply whose two halves disagree is exactly the kind
    // of false statement this surface exists to refuse. One line of it can no longer be true, so it
    // is re-rendered; every other line in `report` is still a fact about the tree as read.
    // `report` 是在这次运行**之前**渲染的，因此它那行 `state` 是这次调用的**起点**。放在一份"每条义务都通过"
    // 的摘要旁边，它读起来就是自相矛盾（"unknown" 挨着三个 "passed"），而回复的两半互相打架，正是这个执行面
    // 存在就是为了拒绝的假话。它有一行已经不可能为真，于是重渲染那一行；`report` 的其余各行仍然是对那棵树的事实。
    let now_state = ledger["state"].as_str().unwrap_or("unknown");
    let started = report
        .lines()
        .find(|line| line.starts_with("state "))
        .unwrap_or("state unknown")
        .to_owned();
    let report = if started != format!("state {now_state}") && report.contains(&started) {
        report.replacen(&started, &format!("state {now_state}"), 1)
    } else {
        report
    };
    // The reply leads with what this call did, the same way a check reply leads with its
    // verdict: a reader that stops at line one still learns whether anything ran.
    // 回复以这次调用做了什么开头，与 check 回复以判定开头同一个道理：只读第一行的读者也知道有没有跑过。
    Ok(format!("{summary}{report}"))
}

/// The per-obligation lines both `status` and `resume` print.
/// `status` 与 `resume` 都会打印的逐义务行。
///
/// A pass goes stale for one of two different reasons, and the line says **which**: the bytes it was
/// bound to changed, or the execution configuration (toolchain, target, profile, features, mode,
/// selection, isolation) changed while the bytes did not. Reporting only "stale" would send the
/// reader looking for an edit that never happened; reporting only the closure comparison is how a
/// green produced under another profile reads as a green about this one (BR05).
/// 一次通过会因为两种不同原因之一失效，而这一行说出**是哪一种**：它绑定的字节变了，或者字节没变而执行配置
/// （工具链、目标、profile、特性、模式、选择、隔离）变了。只报"stale"会让读者去找一次根本没发生过的编辑；
/// 而只比闭包，正是"在另一个 profile 下产出的绿被读成关于这一个的绿"的来路（BR05）。
fn obligation_table(
    ledger: &Value,
    identity: &str,
    environment: &super::check_execution::Environment,
    root: &Path,
) -> String {
    let mut text = String::from("obligations");
    for row in ledger["obligations"].as_array().into_iter().flatten() {
        let state = row["state"].as_str().unwrap_or("unknown");
        let bound = row["evidence_identity"].as_str();
        let reason = if state != "passed" {
            ""
        } else if bound != Some(identity) {
            " — stale: the inputs changed since this passed"
        } else {
            match super::check_execution::execution_identity(
                &row["request"],
                identity,
                environment,
                root,
            ) {
                Ok(now) if row["execution_digest"].as_str() == Some(now.digest().as_str()) => "",
                Ok(_) => " — stale: the execution configuration changed since this passed",
                // A row whose request cannot be read back cannot be shown to still hold, and an
                // unprovable pass is not a pass.
                Err(_) => " — stale: this row's execution configuration cannot be re-derived",
            }
        };
        text.push_str(&format!(
            "\n  {}: {} (expected {}){execution}{reason}{settled}",
            row["name"].as_str().unwrap_or("?"),
            state,
            row["expected"].as_str().unwrap_or("passed"),
            // The execution that produced this row, so two rows are told apart by *which build*
            // settled them rather than only by their names.
            // 产出这一行的执行，使两行按**哪一次构建**而不是只按名字分辨。
            execution = match row["execution_digest"].as_str() {
                Some(digest) if digest.len() >= 16 => format!(" [execution {}]", &digest[..16]),
                _ => String::new(),
            },
            // Named `settled` and not `reason`: a named argument shadows the local of the same name,
            // and calling this one `reason` would silently print the reuse marker where the staleness
            // verdict belongs.
            // 命名成 `settled` 而不是 `reason`：同名具名参数会遮蔽同名的局部变量，把它叫 `reason` 就会
            // 在失效判定的位置上静默印出复用标记。
            settled = match row["satisfied_by"].as_str() {
                Some(first) => format!(" [no run: settled by `{first}`'s execution]"),
                None => String::new(),
            }
        ));
    }
    text.push('\n');
    text
}

/// Continue a durable plan, re-running only what is unfinished or invalidated.
/// 继续一份持久计划，只重跑未完成或已失效的部分。
fn resume(
    root: &Path,
    ledger: &mut Value,
    files: &super::check_task::PlanFiles,
    excluded: Option<&Path>,
    identity: &str,
    run: &str,
    inputs_match: bool,
) -> Result<String, String> {
    let rows = ledger["obligations"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let environment = super::check_execution::Environment::observe();
    let mut indices = Vec::new();
    let mut obligations = Vec::new();
    let mut executions = Vec::new();
    let mut targets = Vec::new();
    let mut skipped = Vec::new();
    for (index, row) in rows.iter().enumerate() {
        let state = row["state"].as_str().unwrap_or("unknown");
        // An obligation that already landed an effect is never re-run automatically: replaying
        // it could duplicate a mutation, and this plan has no way to tell a retry from a first
        // attempt (W4).
        // 已经落过副作用的义务绝不自动重跑：重放它可能重复一次改动，而这份计划分不出"重试"与"第一次"（W4）。
        if row["effect"].as_str() == Some("side_effecting") {
            skipped.push(format!(
                "{}: {} — not re-run automatically",
                row["name"].as_str().unwrap_or("?"),
                state
            ));
            continue;
        }
        // A pass is evidence only while the bytes **and** the configuration it was produced under
        // still hold. The closure comparison alone would keep a green that another toolchain, target,
        // profile, feature face, mode or output directory produced — the merge BR05 forbids — so the
        // recorded execution digest has to match the one this resume would run under.
        // 只有**它自己**被绑定到的字节**与配置**都仍然成立时，一次通过才是证据。只比闭包会留下一个由另一个
        // 工具链、目标、profile、特性面、模式或输出目录产出的绿——正是 BR05 禁止的合并——因此记录下来的执行
        // 摘要必须与这次恢复将要使用的那个相同。
        let record = obligation_from_row(row);
        // A variant row's candidate is the copy's digest and its tree is the copy, not the base: a
        // resume that recomputed either one from the plan's own identity would call every variant
        // stale and re-run it, and — worse — would judge it against the base tree's bytes.
        // 变体行的 candidate 是副本的摘要、它的树是副本而不是基树：若恢复时从计划自身的身份重算这两样，
        // 就会把每个变体都判成失效并重跑它，更糟的是会拿基树的字节去判定它。
        let candidate = row["candidate"].as_str().unwrap_or(identity);
        let target = row["variant_root"]
            .as_str()
            .map_or_else(|| root.to_path_buf(), PathBuf::from);
        if state == "passed"
            && row["evidence_identity"].as_str() == Some(identity)
            && match &record {
                Some(obligation) => {
                    let now = super::check_execution::execution_identity(
                        &obligation.request,
                        candidate,
                        &environment,
                        &target,
                    )?;
                    row["execution_digest"].as_str() == Some(now.digest().as_str())
                }
                // A row the plan did not write a replayable request for (the registration
                // inventory) is kept exactly as the previous behaviour kept it.
                // 计划没有为它写下可重放请求的行（注册清单）按既有行为原样保留。
                None => true,
            }
        {
            continue;
        }
        let Some(obligation) = record else {
            skipped.push(format!(
                "{}: the ledger row cannot be replayed",
                row["name"].as_str().unwrap_or("?")
            ));
            continue;
        };
        let now = super::check_execution::execution_identity(
            &obligation.request,
            candidate,
            &environment,
            &target,
        )?;
        let because = if !inputs_match {
            "inputs changed"
        } else if state == "passed" {
            "configuration changed"
        } else {
            "unfinished"
        };
        let mut phase = super::check_task::phase(run, "resume");
        phase["obligation"] = json!(rows[index]["name"]);
        phase["because"] = json!(because);
        super::check_task::note(&files.journal, phase)?;
        indices.push(index);
        obligations.push(obligation);
        executions.push(now);
        targets.push(target);
    }
    let refreshed = super::check_task::Inputs::capture(root, excluded)?;
    refreshed.write(&files.inputs)?;
    ledger["source_configuration"] = json!(identity);
    ledger["inputs"] = json!(refreshed.count());
    // The count travels with the identity. A plan re-recorded against a different closure must not keep
    // reporting the old "N inputs" — that field is what a consumer reads to size the closure, and leaving it
    // stale is the same class of defect as a stale digest (measured: a plan re-recorded at 10 inputs still
    // reported `"inputs": 6`).
    // 计数与身份同行。按不同闭包重记的计划不得继续报旧的"N inputs"——消费者正是读这个字段来判断闭包大小，
    // 而让它陈旧与"摘要陈旧"是同一类缺陷（实测：重记到 10 项输入的计划仍报 `"inputs": 6`）。
    persist(&files.ledger, ledger)?;
    let all_passed = execute(
        ledger,
        &files.ledger,
        &files.journal,
        root,
        identity,
        run,
        excluded,
        &indices,
        &obligations,
        &executions,
        &targets,
    )?;
    let open = ledger["obligations"]
        .as_array()
        .into_iter()
        .flatten()
        .any(|row| row["state"].as_str() != Some("passed"));
    let complete = all_passed && !open && skipped.is_empty();
    ledger["state"] = json!(if complete { "passed" } else { "open" });
    super::check_task::note(
        &files.journal,
        super::check_task::phase(run, if complete { "completed" } else { "open" }),
    )?;
    persist(&files.ledger, ledger)?;
    let mut report = format!(
        "resumed {run}: {} obligation(s) re-run, {} kept, {} not re-run\n",
        obligations.len(),
        rows.len() - obligations.len() - skipped.len(),
        skipped.len()
    );
    for line in &skipped {
        report.push_str(&format!("  {line}\n"));
    }
    if !inputs_match {
        report.push_str(
            "  inputs changed, so every earlier pass was stale: evidence is bound to the \
             bytes it was produced from\n",
        );
    }
    report.push_str(&obligation_table(ledger, identity, &environment, root));
    Ok(report)
}

/// The root a stored plan was written against, recovered from its own rows.
/// 一份存量计划所依据的根，从它自己的行里恢复。
///
/// The `target_dir` a row carries is absolute, so the tree is its nearest ancestor that is not a
/// build directory. A row written without one is answered as the current root by the caller.
/// 一行携带的 `target_dir` 是绝对路径，因此那棵树就是它最近的一个非构建目录祖先。没有它的行由调用方按当前
/// 根作答。
fn root_for(ledger: &Value) -> Option<PathBuf> {
    let row = ledger["obligations"].as_array()?.first()?;
    let directory = Path::new(row["request"]["target_dir"].as_str()?);
    directory
        .ancestors()
        .find(|path| path.join("Cargo.toml").is_file())
        .map(Path::to_path_buf)
}

/// The row that actually ran an obligation, following `satisfied_by` when the row was settled by
/// another obligation's execution.
/// 真正跑过某条义务的那一行；该行由别的义务的执行结清时，沿 `satisfied_by` 找到它。
pub(super) fn settled_row<'a>(rows: &'a [Value], row: &'a Value) -> &'a Value {
    let Some(first) = row["satisfied_by"].as_str() else {
        return row;
    };
    rows.iter()
        .find(|candidate| candidate["name"].as_str() == Some(first))
        .unwrap_or(row)
}

/// How many tests the execution behind one row actually ran.
/// 某一行背后的执行实际跑了多少个测试。
fn settled_count(rows: &[Value], row: &Value) -> usize {
    settled_row(rows, row)["outcome"]["executed"]
        .as_u64()
        .unwrap_or(0) as usize
}

/// How many `test result:` lines that execution produced; zero means nothing ran.
/// 那次执行产出了几条 `test result:` 行；零意味着什么都没跑。
fn settled_results(rows: &[Value], row: &Value) -> usize {
    settled_row(rows, row)["outcome"]["results"]
        .as_u64()
        .unwrap_or(0) as usize
}

/// The typed outcome that execution reached.
/// 那次执行达到的类型化结果。
fn settled_outcome(rows: &[Value], row: &Value) -> VerificationOutcome {
    match settled_row(rows, row)["outcome"]["verdict"].as_str() {
        Some("passed") => VerificationOutcome::Passed,
        Some("failed") => VerificationOutcome::Failed,
        _ => VerificationOutcome::Unknown(xirang_kernel::maintenance::UnknownReason::NoResultLine),
    }
}

#[cfg(test)]
#[path = "check_resume_tests.rs"]
mod check_resume_tests;
