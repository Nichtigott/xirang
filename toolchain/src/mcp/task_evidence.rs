//! The five evidence readers a maintenance task's obligations are computed from.
//! 维护任务的义务所依据的五个证据读取器。
//!
//! Split out of `task.rs` to keep both files inside the repository's size ratchet, and because the
//! subject is different: `task.rs` owns the record and the verbs, while this file owns what the tree
//! says about an object, a plan, the rule book and the knowledge book. Every reader returns an
//! obligation — the undecidable ones included — so no caller can turn a missing fact into a pass.
//! 从 `task.rs` 拆出，既让两个文件留在仓库的尺寸棘轮之内，也因为题材不同：`task.rs` 拥有记录与动词，
//! 而这个文件拥有树对对象、计划、规则簿与知识簿的说法。每个读取器都返回一条义务——包括判不了的那些——
//! 因此没有调用方能把手边缺一个事实变成一次通过。

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde_json::Value;
use xirang_kernel::lexicon::XIRANG_DIR;
use xirang_kernel::maintenance::{RuleOutcome, TaskCompletion, TaskObligation, TaskObligationKind};

use crate::mcp::adopted::{AnchorStanding, anchor_standing, entries};
use crate::mcp::rules::{all as rule_book, check_subjects, resolve_subject};

/// What the tree says about one object right now.
/// 树此刻对某个对象的说法。
pub(super) struct ObjectFacts {
    pub(super) path: String,
    pub(super) stable: String,
    /// The record spelling the view carries; an identity input, never rewritten.
    /// 视图携带的记录拼写；它是身份输入，绝不改写。
    pub(super) source: String,
    /// The same source in the ledger's coordinate: package-relative.
    /// 同一个源码在台账坐标里的拼写：包内相对。
    pub(super) package_source: String,
    pub(super) digest: String,
}

/// Read the object's own declaration file, its persistent lifetime, and its bytes.
/// 读对象自己的声明文件、它的持续身份与它的字节。
pub(super) fn object_facts(root: &Path, selector: &str) -> Result<Option<ObjectFacts>, String> {
    let Some(subject) = resolve_subject(root, selector)? else {
        return Ok(None);
    };
    let namespace = crate::mcp::registry::namespace(root)?;
    let views = crate::build_method::face_views(root, &namespace)?;
    let Some(view) = views.iter().find(|view| view.path == subject.path) else {
        return Ok(None);
    };
    let layout = crate::build_method::source_layout(root)?;
    let file = view.locate_source(&layout)?;
    let bytes = std::fs::read(&file)
        .map_err(|error| format!("cannot read `{}`: {error}", file.display()))?;
    Ok(Some(ObjectFacts {
        path: subject.path,
        stable: subject.stable.to_string(),
        source: view.source.clone(),
        package_source: view.observed(&layout, &namespace).source,
        digest: xirang_kernel::sha256_hex(&bytes),
    }))
}

/// The rules that apply to one object, as `(rule id, revision)`.
/// 作用于某个对象的规则，按 `(规则 id, 修订)`。
pub(super) fn applicable_rules(root: &Path, object: &str) -> Result<Vec<(String, u64)>, String> {
    let checks = check_subjects(root, root, &[object.to_owned()])?;
    Ok(checks
        .into_iter()
        .map(|check| (check.rule_id, check.revision))
        .collect())
}

/// The adoption lease each applicable rule rests on, as `(anchor, fingerprint now)`.
/// 每条适用规则所依托的采信租约，按 `(anchor, 此刻的指纹)`。
///
/// A rule with no `authority_ref` rests on no lease; a rule whose anchor has no entry records the
/// absence rather than dropping the fact, so the status can say the lease disappeared later.
/// 没有 `authority_ref` 的规则不依托任何租约；锚点在台账里没有条目的规则记录下这个"没有"，而不是把这件事
/// 丢掉，因此 status 以后能说出租约消失了。
pub(super) fn applicable_leases(
    root: &Path,
    object: &str,
) -> Result<BTreeMap<String, Option<String>>, String> {
    let book = rule_book(&root.join(XIRANG_DIR).join("rules"))?;
    let applicable = applicable_rules(root, object)?
        .into_iter()
        .map(|(id, _)| id)
        .collect::<Vec<_>>();
    let ledger = entries(root)?;
    let mut leases = BTreeMap::new();
    for record in &book {
        if !applicable.contains(&record.rule_id) {
            continue;
        }
        let Some(anchor) = &record.authority_ref else {
            continue;
        };
        let fingerprint = ledger
            .iter()
            .rev()
            .find(|entry| &entry.anchor == anchor)
            .map(|entry| entry.fingerprint.clone());
        leases.insert(anchor.clone(), fingerprint);
    }
    Ok(leases)
}

/// Where a check plan's records live.
/// 检查计划的记录住在哪里。
pub(super) fn plan_directory(root: &Path, target_dir: Option<&str>) -> PathBuf {
    target_dir
        .map(|path| root.join(path))
        .unwrap_or_else(|| root.join("target"))
        .join("xirang/verification")
}

/// One short fingerprint, as a reader compares two of them.
/// 一个短指纹，读者正是这样比较两个的。
pub(super) fn short(value: &str) -> &str {
    value.get(..8).unwrap_or(value)
}

/// One sentence describing an anchor's standing.
/// 描述一个锚点状态的一句话。
fn standing_line(standing: &AnchorStanding) -> String {
    match standing {
        AnchorStanding::InForce => "in force".to_owned(),
        AnchorStanding::Missing => "absent from the ledger".to_owned(),
        AnchorStanding::Lapsed(reason) => format!("lapsed ({reason})"),
        AnchorStanding::Unreadable(reason) => format!("unreadable ({reason})"),
    }
}

/// The five obligations, recomputed from the tree.
/// 五条义务，从树上重新计算。
pub(super) fn obligations(root: &Path, record: &Value) -> Result<TaskCompletion, String> {
    let mut list = vec![
        applied(root, record)?,
        verified(root, record)?,
        rules_obligation(root, record)?,
        knowledge(root, record)?,
        acceptance(root, record)?,
    ];
    list.sort_by_key(|obligation| obligation.kind());
    TaskCompletion::new(list)
}

/// The record's recorded object spelling.
/// 记录里的对象拼写。
fn recorded_object(record: &Value) -> Result<&str, String> {
    record["object"]["path"]
        .as_str()
        .ok_or_else(|| "the task record names no object path".to_owned())
}

/// `applied`: the object exists with the bytes the task recorded.
/// `applied`：对象存在，字节就是任务记录下的那些。
fn applied(root: &Path, record: &Value) -> Result<TaskObligation, String> {
    let kind = TaskObligationKind::Applied;
    let path = recorded_object(record)?;
    let recorded = &record["object"];
    let stable = recorded["stable"].as_str().unwrap_or("");
    let digest = recorded["digest"].as_str().unwrap_or("");
    let Some(facts) = object_facts(root, path)? else {
        // The spelling may have moved; resolve the recorded identity before calling the object gone.
        // 拼写可能被搬走了；在断言对象消失之前，先用记录下的身份解析一次。
        let moved = object_facts(root, stable)?;
        return Ok(match moved {
            Some(facts) => TaskObligation::outstanding(
                kind,
                format!(
                    "the object is now at `{}` (the task recorded `{path}`); reopen the task against \
                     its current location",
                    facts.path
                ),
            ),
            None => TaskObligation::outstanding(
                kind,
                format!("no object resolves at `{path}` or at the identity this task recorded"),
            ),
        });
    };
    if facts.stable != stable {
        return Ok(TaskObligation::outstanding(
            kind,
            format!(
                "a different object now holds `{path}` (the task recorded {stable}, the tree has {})",
                facts.stable
            ),
        ));
    }
    if facts.digest != digest {
        return Ok(TaskObligation::outstanding(
            kind,
            format!(
                "the declaration changed after this task recorded it (was {}, now {}); reopen the \
                 task against the applied bytes",
                short(digest),
                short(&facts.digest)
            ),
        ));
    }
    // One object can carry several live source instances; all of them are the object.
    // 一个对象可以带多个存活的源码实例；它们都是这个对象。
    if let Some(finding) = super::task_sources::applied_instances(root, record, stable)? {
        return Ok(finding);
    }
    Ok(TaskObligation::satisfied(
        kind,
        format!(
            "`{path}` is the bytes this task recorded ({})",
            short(digest)
        ),
    ))
}

/// `verified`: a passing plan whose input closure still holds.
/// `verified`：一份通过的计划，且它的输入闭包仍然成立。
fn verified(root: &Path, record: &Value) -> Result<TaskObligation, String> {
    let kind = TaskObligationKind::Verified;
    let plan_id = record["verification"]["plan"]
        .as_str()
        .ok_or_else(|| "the task record names no verification plan".to_owned())?;
    let target_dir = record["verification"]["target_dir"].as_str();
    Ok(match plan_holding(root, plan_id, target_dir)? {
        PlanHolding::Holds { inputs } => TaskObligation::satisfied(
            kind,
            format!(
                "plan `{plan_id}` passed and none of the {inputs} input(s) it bound has changed"
            ),
        ),
        PlanHolding::NotPassed { state } => TaskObligation::outstanding(
            kind,
            format!("the verification plan `{plan_id}` is `{state}`, not `passed`"),
        ),
        PlanHolding::Gone => TaskObligation::unknown(
            kind,
            format!(
                "the verification plan `{plan_id}` is not on disk; the evidence this task rests on \
                 is gone"
            ),
        ),
        PlanHolding::Unreadable(why) => TaskObligation::unknown(kind, why),
        PlanHolding::RowsDisagree(rows) => TaskObligation::unknown(
            kind,
            format!(
                "the plan `{plan_id}` says `passed` but these obligation(s) are not: {}; a ledger \
                 that disagrees with itself is not evidence",
                rows.join(", ")
            ),
        ),
        PlanHolding::ClosureMoved(changes) => TaskObligation::unknown(kind, {
            let named = changes
                .iter()
                .take(3)
                .cloned()
                .collect::<Vec<_>>()
                .join("; ");
            let more = changes.len().saturating_sub(3);
            format!(
                "the verification's inputs changed after it ran: {named}{}",
                if more > 0 {
                    format!(" (+{more} more)")
                } else {
                    String::new()
                }
            )
        }),
    })
}

/// What a check plan's record says about whether it still holds, now.
/// 一份检查计划的记录此刻是否仍然成立。
pub(super) enum PlanHolding {
    /// The plan passed, its rows agree, and every input it bound still has the recorded bytes.
    /// 计划通过、行与标题一致，且它绑定的每项输入仍是记录下的字节。
    Holds {
        /// How many inputs the plan bound.
        /// 计划绑定了多少项输入。
        inputs: usize,
    },
    /// The plan has no passing verdict (sealed, interrupted, failed or re-opened).
    /// 计划没有通过的判定（只封存、被中断、失败或重新打开）。
    NotPassed {
        /// The state the record carries, as printed.
        /// 记录携带的状态，按打印的样子。
        state: String,
    },
    /// No record is on disk.
    /// 磁盘上没有记录。
    Gone,
    /// The record, its manifest or the tree could not be read.
    /// 记录、它的清单或这棵树读不出来。
    Unreadable(String),
    /// The headline says passed while a row does not.
    /// 标题说通过，而某行不是。
    RowsDisagree(Vec<String>),
    /// Inputs the plan bound changed since it ran (the knowledge ledger excluded).
    /// 计划绑定的输入在它跑完之后变了（知识台账已排除）。
    ClosureMoved(Vec<String>),
}

/// One sentence for a plan that does **not** hold, for a refusal that names why.
/// 一句话描述一份**不**成立的计划，供点名原因的拒绝使用。
pub(super) fn describe_holding(holding: &PlanHolding) -> String {
    match holding {
        PlanHolding::Holds { inputs } => format!("it still holds over {inputs} input(s)"),
        PlanHolding::NotPassed { state } => format!("the plan is `{state}`, not `passed`"),
        PlanHolding::Gone => "the plan record is not on disk".to_owned(),
        PlanHolding::Unreadable(why) => why.clone(),
        PlanHolding::RowsDisagree(rows) => format!(
            "the headline says passed while these rows are not: {}",
            rows.join(", ")
        ),
        PlanHolding::ClosureMoved(changes) => format!(
            "{} input(s) changed since it ran (first: {})",
            changes.len(),
            changes.first().map(String::as_str).unwrap_or("?")
        ),
    }
}

/// Decide whether one plan still holds, from the plan's own record and the tree now.
/// 按计划自己的记录与此刻的树，判定一份计划是否仍然成立。
///
/// One implementation for the two consumers that ask the same question — the task's `verified`
/// obligation and opening a task that inherits an earlier task's plan. They map the answer to
/// different replies, but they must not disagree about whether the evidence holds.
/// 两个问同一个问题的消费者共用一份实现——任务的 `verified` 义务，以及"继承上一条任务计划"的开启路径。
/// 它们把答案映射成不同的回复，但不得在"证据是否成立"上各说一套。
pub(super) fn plan_holding(
    root: &Path,
    plan_id: &str,
    target_dir: Option<&str>,
) -> Result<PlanHolding, String> {
    let directory = plan_directory(root, target_dir);
    let plan_path = directory.join(format!("{plan_id}.json"));
    let Ok(text) = std::fs::read_to_string(&plan_path) else {
        return Ok(PlanHolding::Gone);
    };
    let plan: Value = match serde_json::from_str(&text) {
        Ok(plan) => plan,
        Err(error) => {
            return Ok(PlanHolding::Unreadable(format!(
                "{}: {error}",
                plan_path.display()
            )));
        }
    };
    let state = plan["state"].as_str().unwrap_or("(no verdict)");
    if state != "passed" {
        return Ok(PlanHolding::NotPassed {
            state: state.to_owned(),
        });
    }
    // The plan's own claim and its rows must agree. Reading `completion.verification_closes` here was
    // wrong for a plan that only passed **on resume**: that block is written when the plan is created,
    // so a sealed plan that later ran to a pass kept `verification_closes: false` beside
    // `state: passed` — one of them a lie, and the consumer could not tell which. The rows are a fact
    // about now, and `state == passed` is derived from the same predicate, so their agreement is
    // checkable without trusting a snapshot.
    // 计划自己的声明与它的行必须一致。这里读 `completion.verification_closes` 对"**在恢复时**才通过"的计划
    // 是错的：那个块在计划创建时写下，因此只封存过、后来跑成通过的计划会保留 `verification_closes: false`
    // 与 `state: passed` 并排——其中一个是假话，而消费者分不出是哪个。行是关于此刻的事实，而
    // `state == passed` 由同一谓词推导，因此它们的一致可以在不信任快照的前提下被检查。
    let unpassed = plan["obligations"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|row| row["state"].as_str() != Some("passed"))
        .map(|row| row["name"].as_str().unwrap_or("?").to_owned())
        .collect::<Vec<_>>();
    if !unpassed.is_empty() {
        return Ok(PlanHolding::RowsDisagree(unpassed));
    }
    let Some(inputs_file) = plan["inputs_file"].as_str() else {
        return Ok(PlanHolding::Unreadable(format!(
            "the plan `{plan_id}` records no input manifest"
        )));
    };
    let recorded = match crate::mcp::check_task::Inputs::read(&directory.join(inputs_file)) {
        Ok(inputs) => inputs,
        Err(error) => {
            return Ok(PlanHolding::Unreadable(format!(
                "the plan's input manifest cannot be read: {error}"
            )));
        }
    };
    if let Some(reason) = recorded.incomplete() {
        return Ok(PlanHolding::Unreadable(format!(
            "the plan's closure was already incomplete when it ran: {reason}"
        )));
    }
    let excluded = target_dir.map(|path| root.join(path));
    let current = match crate::mcp::check_task::Inputs::capture(root, excluded.as_deref()) {
        Ok(inputs) => inputs,
        Err(error) => {
            return Ok(PlanHolding::Unreadable(format!(
                "the tree's inputs cannot be re-read, so the closure cannot be compared: {error}"
            )));
        }
    };
    if let Some(reason) = current.incomplete() {
        return Ok(PlanHolding::Unreadable(format!(
            "the closure cannot be bound completely now: {reason}"
        )));
    }
    // The knowledge ledger is not a verification input: the rules/knowledge obligations snapshot it.
    // 知识台账不是验证输入：规则与知识义务各自快照它。
    let changes = recorded.changes_excluding(&current, &[XIRANG_DIR]);
    if !changes.is_empty() {
        return Ok(PlanHolding::ClosureMoved(changes));
    }
    Ok(PlanHolding::Holds {
        inputs: recorded.count(),
    })
}

/// `rules`: every applicable rule is met at the revision the task recorded.
/// `rules`：每条适用规则都在任务记录的修订上成立。
fn rules_obligation(root: &Path, record: &Value) -> Result<TaskObligation, String> {
    let kind = TaskObligationKind::Rules;
    let object = recorded_object(record)?;
    let recorded: BTreeMap<String, u64> = record["rules"]
        .as_object()
        .map(|map| {
            map.iter()
                .filter_map(|(id, revision)| Some((id.clone(), revision.as_u64()?)))
                .collect()
        })
        .unwrap_or_default();
    let checks = match check_subjects(root, root, &[object.to_owned()]) {
        Ok(checks) => checks,
        Err(error) => {
            return Ok(TaskObligation::unknown(
                kind,
                format!("the rule book cannot be read, so no rule can be decided: {error}"),
            ));
        }
    };
    let now: BTreeMap<String, u64> = checks
        .iter()
        .map(|check| (check.rule_id.clone(), check.revision))
        .collect();
    if now != recorded {
        return Ok(TaskObligation::unknown(kind, book_changed(&recorded, &now)));
    }
    let mut met = 0_usize;
    let mut exempt = Vec::new();
    let mut outside = 0_usize;
    let mut violated = Vec::new();
    let mut undecided = Vec::new();
    for check in &checks {
        match &check.outcome {
            RuleOutcome::Satisfied => met += 1,
            RuleOutcome::Exempt { .. } => exempt.push(check.rule_id.clone()),
            RuleOutcome::NotApplicable { .. } => outside += 1,
            RuleOutcome::Violated => violated.push(format!(
                "`{}` [revision {}] {}",
                check.rule_id, check.revision, check.requirement
            )),
            RuleOutcome::Unknown { reason } => {
                undecided.push(format!("`{}` cannot be decided: {reason}", check.rule_id))
            }
            RuleOutcome::NeedsReview { reason } => {
                undecided.push(format!("`{}` needs a person: {reason}", check.rule_id))
            }
            RuleOutcome::NotConstraining { reason } => {
                undecided.push(format!("`{}` cannot constrain: {reason}", check.rule_id))
            }
        }
    }
    if !violated.is_empty() {
        return Ok(TaskObligation::outstanding(
            kind,
            format!(
                "{} rule(s) are violated at the revisions this task recorded: {}; fix the tree, not \
                 the book",
                violated.len(),
                violated.join("; ")
            ),
        ));
    }
    if !undecided.is_empty() {
        return Ok(TaskObligation::unknown(
            kind,
            format!(
                "{} rule(s) cannot be decided at the revisions this task recorded: {}",
                undecided.len(),
                undecided.join("; ")
            ),
        ));
    }
    Ok(TaskObligation::satisfied(
        kind,
        format!(
            "{met} rule(s) met at the revisions this task recorded{}",
            if exempt.is_empty() && outside == 0 {
                String::new()
            } else {
                format!(
                    "; {} exempt by a recorded decision, {outside} outside this subject",
                    exempt.len()
                )
            }
        ),
    ))
}

/// The sentence for a rule book that moved under the task.
/// 规则簿在任务底下移动时的说法。
fn book_changed(recorded: &BTreeMap<String, u64>, now: &BTreeMap<String, u64>) -> String {
    let mut differences = Vec::new();
    for (id, revision) in recorded {
        match now.get(id) {
            None => differences.push(format!(
                "rule `{id}` [revision {revision}] is no longer in the book"
            )),
            Some(current) if current != revision => differences.push(format!(
                "rule `{id}` is now revision {current}; this task recorded {revision}"
            )),
            Some(_) => {}
        }
    }
    for (id, revision) in now {
        if !recorded.contains_key(id) {
            differences.push(format!(
                "rule `{id}` [revision {revision}] was accepted after this task recorded its rules"
            ));
        }
    }
    format!(
        "the rule book changed after this task opened: {}; re-plan against the current book instead \
         of closing on the old one",
        differences.join("; ")
    )
}

/// `knowledge`: every adoption lease the rules rest on is in force at the recorded version.
/// `knowledge`：规则所依托的每份采信租约都按记录的版本生效。
fn knowledge(root: &Path, record: &Value) -> Result<TaskObligation, String> {
    let kind = TaskObligationKind::Knowledge;
    let recorded: BTreeMap<String, Option<String>> = record["leases"]
        .as_object()
        .map(|map| {
            map.iter()
                .map(|(anchor, fingerprint)| {
                    (anchor.clone(), fingerprint.as_str().map(str::to_owned))
                })
                .collect()
        })
        .unwrap_or_default();
    if recorded.is_empty() {
        return Ok(TaskObligation::satisfied(
            kind,
            "the rules this task recorded rest on no adoption lease".to_owned(),
        ));
    }
    let ledger = entries(root)?;
    let mut not_in_force = Vec::new();
    let mut moved = Vec::new();
    for (anchor, fingerprint) in &recorded {
        let standing = anchor_standing(root, anchor).unwrap_or_else(|error| {
            AnchorStanding::Unreadable(format!("the ledger could not be read: {error}"))
        });
        let newest = ledger
            .iter()
            .rev()
            .find(|entry| &entry.anchor == anchor)
            .map(|entry| entry.fingerprint.clone());
        if !matches!(standing, AnchorStanding::InForce) {
            not_in_force.push(format!("`{anchor}` is {}", standing_line(&standing)));
            continue;
        }
        match (fingerprint, newest) {
            (Some(was), Some(now)) if &now != was => moved.push(format!(
                "`{anchor}` was renewed on new bytes (was {}, now {})",
                short(was),
                short(&now)
            )),
            (Some(_), None) => {
                not_in_force.push(format!("`{anchor}` has no ledger entry any more"))
            }
            (None, Some(now)) => moved.push(format!(
                "`{anchor}` now exists ({}); the task recorded that it did not",
                short(&now)
            )),
            _ => {}
        }
    }
    if !not_in_force.is_empty() {
        return Ok(TaskObligation::outstanding(
            kind,
            format!(
                "the knowledge this task rests on is no longer in force: {}; the rules that rest on \
                 it cannot be satisfied",
                not_in_force.join("; ")
            ),
        ));
    }
    if !moved.is_empty() {
        return Ok(TaskObligation::unknown(
            kind,
            format!(
                "the knowledge this task rests on changed after it opened: {}; re-plan against the \
                 renewed decision",
                moved.join("; ")
            ),
        ));
    }
    Ok(TaskObligation::satisfied(
        kind,
        format!(
            "{} adoption lease(s) are in force at the versions this task recorded",
            recorded.len()
        ),
    ))
}

/// `acceptance`: only when this task's policy requires one.
/// `acceptance`：仅当这条任务的政策要求时。
fn acceptance(root: &Path, record: &Value) -> Result<TaskObligation, String> {
    let kind = TaskObligationKind::Acceptance;
    let required = record["acceptance"]["required"].as_bool().unwrap_or(false);
    if !required {
        return Ok(TaskObligation::satisfied(
            kind,
            "this task's policy requires no acceptance (a maintenance task does not acquire an \
             approval gate by default)"
                .to_owned(),
        ));
    }
    let anchor = record["acceptance"]["anchor"]
        .as_str()
        .ok_or_else(|| "the task says it requires an acceptance but names no anchor".to_owned())?;
    let recorded = record["acceptance"]["entries"].as_u64().unwrap_or(0) as usize;
    let standing = anchor_standing(root, anchor).unwrap_or_else(|error| {
        AnchorStanding::Unreadable(format!("the ledger could not be read: {error}"))
    });
    if !matches!(standing, AnchorStanding::InForce) {
        return Ok(TaskObligation::outstanding(
            kind,
            format!(
                "acceptance is required and the lease `{anchor}` is {}; confirm it after this task \
                 opened",
                standing_line(&standing)
            ),
        ));
    }
    let now = entries(root)?
        .iter()
        .filter(|entry| entry.anchor == anchor)
        .count();
    if now > recorded {
        return Ok(TaskObligation::satisfied(
            kind,
            format!(
                "the lease `{anchor}` was confirmed again after this task opened ({recorded} → {now} \
                 lines); this is the same provisional lease the rule book uses — it records a \
                 declared decision, not who made it"
            ),
        ));
    }
    Ok(TaskObligation::outstanding(
        kind,
        format!(
            "acceptance is required by this task's policy and the lease `{anchor}` has not been \
             confirmed since the task opened (still {now} line(s)); confirm it, then close again"
        ),
    ))
}
