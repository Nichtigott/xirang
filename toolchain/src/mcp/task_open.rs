//! `open`: record the task and the evidence it rests on.
//! `open`：记录这条任务以及它所依托的证据。
//!
//! Split out of `task.rs` for the size ratchet. The snapshot written here is what makes a later
//! status meaningful: the object's bytes and identity, the rule revisions, the lease fingerprints and
//! the acceptance policy are read **now**, so a change afterwards is visible as a change rather
//! than silently re-read as if the task had always meant it.
//! 从 `task.rs` 拆出以守尺寸棘轮。这里写下的快照是之后 status 有意义的原因：对象的字节与身份、规则修订、
//! 租约指纹与接受政策都是**此刻**读的，因此其后的变化会作为变化被看见，而不是被静默重读成"任务本来
//! 就是这个意思"。

use std::path::Path;

use serde_json::{Value, json};

use crate::mcp::adopted::entries;

use super::task_evidence::{
    PlanHolding, applicable_leases, applicable_rules, describe_holding, object_facts,
    plan_directory, plan_holding, short,
};
use super::{read_record, record_path, text, valid_id, write_record};

/// `open`: record the task and the evidence it rests on.
/// `open`：记录这条任务以及它所依托的证据。
pub(super) fn open_task(root: &Path, arguments: &Value) -> Result<String, String> {
    let id = text(arguments, "task")?;
    if !valid_id(&id) {
        return Err(format!(
            "`{id}` is not a task id: letters, digits, `-`, `_`, `.` (at most 64), and it is also \
             the record's file name; nothing was written"
        ));
    }
    let path = record_path(root, &id);
    if path.exists() {
        let state = read_record(root, &id)
            .ok()
            .and_then(|record| record["state"].as_str().map(str::to_owned))
            .unwrap_or_else(|| "unreadable".to_owned());
        return Err(format!(
            "task `{id}` already exists ({}, state: {state}); a task id names one record — retire it \
             or open a new id",
            path.display()
        ));
    }
    let object = text(arguments, "object")?;
    let Some(facts) = object_facts(root, &object)? else {
        return Err(format!(
            "no face in this tree is `{object}`; `xirang.registry` lists them, and `xirang.explain \
             <path>` answers for one. Nothing was written"
        ));
    };
    // A closed task is the only source of established knowledge: an open one has not concluded
    // anything yet, and inheriting from it would launder an unfinished conclusion into a second
    // record.
    // 只有已闭合的任务才是既有知识的来源：未闭合的任务还没有得出任何结论，从它继承会把一个未完成的结论洗成
    // 第二份记录。
    let from = arguments
        .get("from")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|id| !id.is_empty());
    let source = match from {
        Some(source_id) => {
            let record = read_record(root, source_id)?;
            if record["state"].as_str() != Some("closed") {
                return Err(format!(
                    "task `{source_id}` has not closed (state: {}); a task that has not closed has \
                     established nothing another task may inherit. Nothing was written",
                    record["state"].as_str().unwrap_or("open")
                ));
            }
            Some((source_id.to_owned(), record))
        }
        None => None,
    };
    // The verification is named, or inherited when the closed source's plan still holds. "Still
    // holds" is the same question the task's own `verified` obligation asks, and it is answered by
    // the same function — a plan that stopped holding may not be inherited through a different door.
    // 验证由调用方点名，或在源任务已闭合且它的计划仍然成立时被继承。"仍然成立"与任务自己的 `verified`
    // 义务是同一个问题，由同一个函数回答——不再成立的计划不能从另一道门被继承。
    let named = arguments
        .get("verification")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|verification| !verification.is_empty());
    let (verification, target_dir, verification_inherited, verification_note) = match named {
        Some(verification) => {
            if !crate::mcp::check_task::looks_like_plan_id(verification) {
                return Err(format!(
                    "`{verification}` is not a check plan id; `xirang.check` prints its id on a \
                     `plan <id>` line. Nothing was written"
                ));
            }
            let target_dir = arguments.get("target_dir").and_then(Value::as_str);
            let plan = plan_directory(root, target_dir).join(format!("{verification}.json"));
            if !plan.is_file() {
                return Err(format!(
                    "no check plan `{verification}` at {}; run `xirang.check {{files:[…]}}` and pass \
                     the `plan <id>` it printed (and the same `target_dir`, when it was given one). \
                     Nothing was written",
                    plan.display()
                ));
            }
            (verification.to_owned(), target_dir, false, String::new())
        }
        None => match &source {
            Some((source_id, record)) => {
                let plan_id = record["verification"]["plan"]
                    .as_str()
                    .unwrap_or_default()
                    .to_owned();
                let plan_target = record["verification"]["target_dir"].as_str();
                let given = arguments.get("target_dir").and_then(Value::as_str);
                if given.is_some_and(|given| Some(given) != plan_target) {
                    return Err(format!(
                        "`target_dir` was given, but the plan inherited from task `{source_id}` \
                         lives under `{}`; do not pass `target_dir` with `from` unless it is the \
                         same one. Nothing was written",
                        plan_target.unwrap_or("<package>/target")
                    ));
                }
                match plan_holding(root, &plan_id, plan_target)? {
                    PlanHolding::Holds { inputs } => (
                        plan_id.clone(),
                        plan_target,
                        true,
                        format!(
                            "inherited from task `{source_id}` (its closure still holds over \
                             {inputs} input(s))"
                        ),
                    ),
                    other => {
                        return Err(format!(
                            "task `{source_id}`'s plan `{plan_id}` no longer holds: {}; name a new \
                             `verification` plan (or run a `check` and open the task against it). \
                             Nothing was written",
                            describe_holding(&other)
                        ));
                    }
                }
            }
            None => {
                return Err(
                    "`verification` is required unless `from` names a closed task whose plan still \
                     holds. Nothing was written"
                        .to_owned(),
                );
            }
        },
    };
    let rules = applicable_rules(root, &facts.path)?;
    let leases = applicable_leases(root, &facts.path)?;
    // The object's live source instances, in the ledger's package-relative coordinate. A tree with no
    // ledger entry still records the one source the face view resolved, so the record is always
    // explicit about what "this object" covered.
    // 对象的存活源码实例，按台账的包内相对坐标。没有台账条目的树仍记录面视图解析出的那一个源码，使记录
    // 始终对"这个对象"覆盖了什么保持明确。
    let mut instances = crate::mcp::task::task_sources::object_instances(root, &facts.stable)?;
    if instances.is_empty() {
        instances.push((facts.package_source.clone(), Some(facts.digest.clone())));
    }
    let live = instances
        .iter()
        .filter(|(_, digest)| digest.is_some())
        .count();
    // What the closed source established is compared with what the tree says **now**: equal values
    // are inherited in the only sense that is checkable ("the answer has not moved"), and a moved book
    // or lease is re-read and named rather than copied. A different object has its own rule set, so
    // its rules are always re-read.
    // 源任务确立的东西与**此刻**的树比较：相等才谈得上继承（"答案没有移动"），而移动过的规则簿或租约被
    // 重新读出并点名，而不是被复制。另一个对象有自己的规则集，因此它的规则总是重读。
    let mut from_note = String::new();
    let inherited = match &source {
        Some((source_id, record)) => {
            let same_object = record["object"]["path"].as_str() == Some(facts.path.as_str());
            let source_rules = record["rules"]
                .as_object()
                .map(|map| {
                    map.iter()
                        .filter_map(|(id, revision)| Some((id.clone(), revision.as_u64()?)))
                        .collect::<std::collections::BTreeMap<_, _>>()
                })
                .unwrap_or_default();
            let now_rules = rules
                .iter()
                .cloned()
                .collect::<std::collections::BTreeMap<_, _>>();
            let rules_match = same_object && source_rules == now_rules;
            let source_leases = record["leases"]
                .as_object()
                .map(|map| {
                    map.iter()
                        .map(|(anchor, fingerprint)| {
                            (anchor.clone(), fingerprint.as_str().map(str::to_owned))
                        })
                        .collect::<std::collections::BTreeMap<_, _>>()
                })
                .unwrap_or_default();
            let leases_match = source_leases == leases;
            from_note.push_str(&format!(
                "  from       task `{source_id}` (closed {}): {}\n",
                record["closed"]["at"]
                    .as_str()
                    .unwrap_or("(no time recorded)"),
                if verification_inherited {
                    format!("verification {verification_note}")
                } else {
                    "verification named separately".to_owned()
                }
            ));
            // A different object is the reason for **both** re-reads: its rule set is its own, and a
            // rule set that is its own rests on no lease of the other object's. Saying "the ledger
            // moved" there would name the wrong change.
            // 另一个对象是**两处**重读的共同原因：它的规则集是它自己的，而它自己的规则集不依托另一个对象的
            // 租约。在那里说"台账移动过"会点错变化。
            let (rules_note, leases_note) = if !same_object {
                (
                    "re-read (the closed task is about another object)",
                    "re-read (the closed task is about another object)",
                )
            } else if rules_match && leases_match {
                (
                    "inherited (same revisions as the closed task)",
                    "inherited (same fingerprints as the closed task)",
                )
            } else {
                (
                    if rules_match {
                        "inherited (same revisions as the closed task)"
                    } else {
                        "re-read (the rule book moved since the closed task)"
                    },
                    if leases_match {
                        "inherited (same fingerprints as the closed task)"
                    } else {
                        "re-read (the adoption ledger moved since the closed task)"
                    },
                )
            };
            from_note.push_str(&format!(
                "             rules {rules_note}; leases {leases_note}\n"
            ));
            Some(json!({
                "from": source_id,
                "same_object": same_object,
                "rules": rules_match,
                "leases": leases_match,
                "verification": verification_inherited,
            }))
        }
        None => None,
    };
    let acceptance = arguments
        .get("acceptance")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let required = acceptance
        .get("required")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let anchor = acceptance
        .get("anchor")
        .and_then(Value::as_str)
        .map(str::to_owned);
    if required && anchor.as_deref().is_none_or(str::is_empty) {
        return Err(
            "`acceptance.required` needs `acceptance.anchor`: a required acceptance has to name the \
             adoption lease that will carry the confirmation. Nothing was written"
                .to_owned(),
        );
    }
    let anchor_entries = match &anchor {
        Some(anchor) => entries(root)?
            .iter()
            .filter(|entry| &entry.anchor == anchor)
            .count(),
        None => 0,
    };
    let record = json!({
        "version": 1,
        "task": id,
        "state": "open",
        "opened_at": crate::mcp::freshness::wall_clock(),
        "object": {
            "path": facts.path,
            "stable": facts.stable,
            "source": facts.source,
            "digest": facts.digest,
            "instances": instances
                .iter()
                .map(|(source, digest)| json!({"source": source, "digest": digest}))
                .collect::<Vec<_>>(),
        },
        "verification": {
            "plan": verification,
            "target_dir": target_dir,
        },
        "rules": rules
            .iter()
            .map(|(id, revision)| (id.clone(), json!(revision)))
            .collect::<serde_json::Map<_, _>>(),
        "leases": leases
            .iter()
            .map(|(anchor, fingerprint)| {
                (anchor.clone(), fingerprint.clone().map_or(Value::Null, Value::String))
            })
            .collect::<serde_json::Map<_, _>>(),
        "acceptance": {
            "required": required,
            "anchor": anchor,
            "entries": anchor_entries,
        },
        "inherited_from": from,
        "inherited": inherited,
        "closed": null,
    });
    let summary = format!(
        "task `{id}` for `{}`\n  applied    {} ({})\n  instances  {} ({} on disk, {} historical)\n  verification plan {verification}{}{from_note}  rules      {} at the revisions recorded now\n  leases     {}\n  acceptance {}\n",
        // `from_note` is inserted after the verification line, so its own indentation carries the
        // column; the empty string is the ordinary (non-inheriting) case.
        // `from_note` 插在验证那一行之后，缩进由它自己带；没有继承时是空串。
        record["object"]["path"].as_str().unwrap_or(""),
        record["object"]["source"].as_str().unwrap_or(""),
        short(record["object"]["digest"].as_str().unwrap_or("")),
        instances.len(),
        live,
        instances.len() - live,
        target_dir.map_or_else(String::new, |dir| format!(" (target_dir `{dir}`)")),
        record["rules"].as_object().map_or(0, serde_json::Map::len),
        if record["leases"]
            .as_object()
            .is_some_and(|map| map.is_empty())
        {
            "none (the applicable rules rest on no adoption lease)".to_owned()
        } else {
            record["leases"]
                .as_object()
                .map(|map| {
                    map.iter()
                        .map(|(anchor, fingerprint)| {
                            format!(
                                "`{anchor}` at {}",
                                fingerprint.as_str().map_or("no entry", short)
                            )
                        })
                        .collect::<Vec<_>>()
                        .join(", ")
                })
                .unwrap_or_default()
        },
        if required {
            format!(
                "required — lease `{}` must be confirmed again after this line",
                record["acceptance"]["anchor"].as_str().unwrap_or("")
            )
        } else {
            "not required by this task's policy".to_owned()
        }
    );
    if arguments.get("apply").and_then(Value::as_bool) != Some(true) {
        return Ok(format!(
            "{summary}nothing was written (this is the preview; pass `apply: true` to record the task)\nnext   `task {{action:\"status\",task:\"{id}\"}}` recomputes the five obligations and writes nothing\n"
        ));
    }
    let path = write_record(root, &id, &record)?;
    Ok(format!(
        "{summary}record {}\nnext   `task {{action:\"status\",task:\"{id}\"}}` recomputes the five obligations; `close` writes only when all five are satisfied\n",
        path.display()
    ))
}
