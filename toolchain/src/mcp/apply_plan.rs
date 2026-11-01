//! The rule-plan half of the write path: the record a preview leaves, and the rule check that
//! refuses a change an accepted rule forbids.
//! 写入路径的规则计划那一半：预览留下的记录，以及拒绝"已接受规则禁止的改动"的那道规则检查。
//!
//! Split out of `apply` because the write path is about the verbs and this is about the premise: a
//! preview is a promise about a rule book, and an apply has to keep it. The split is also what keeps
//! both files inside the repository's size budget.
//! 从 `apply` 拆出来，因为写入路径讲的是那几个动词，而这里讲的是前提：预览是对一本规则簿的承诺，而 apply
//! 必须兑现它。这次拆分同时让两个文件留在仓库的尺寸预算内。

use std::path::Path;

use serde_json::{Value, json};

use super::{Action, Outcome, action_name, refused_with_a_way_forward, run_add, run_edit};
use crate::build_method::{face_views, source_layout};
use crate::mcp::preview::{copy_package_with_state, remove_copy};

/// The key a preview and the later apply share: the action, the project and the request.
/// 预览与随后那次 apply 共用的键：动作、项目与请求。
pub(super) fn request_key(root: &Path, action: Action, arguments: &Value) -> String {
    let mut request = serde_json::Map::new();
    for (key, value) in arguments.as_object().cloned().unwrap_or_default() {
        if matches!(key.as_str(), "apply" | "confirm" | "full" | "root") {
            continue;
        }
        request.insert(key, value);
    }
    let canonical = canonical_json(&Value::Object(request));
    let project = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    xirang_kernel::sha256_hex(
        format!("{}|{}|{canonical}", action_name(action), project.display()).as_bytes(),
    )
}

/// One JSON value with object keys sorted, so two spellings of one request agree.
/// 一个把对象键排序过的 JSON 值，使同一请求的两种拼法得到同一个字符串。
fn canonical_json(value: &Value) -> String {
    match value {
        Value::Object(map) => {
            let mut entries = map.iter().collect::<Vec<_>>();
            entries.sort_by(|left, right| left.0.cmp(right.0));
            let body = entries
                .into_iter()
                .map(|(key, value)| {
                    format!(
                        "{}:{}",
                        serde_json::to_string(key).unwrap_or_default(),
                        canonical_json(value)
                    )
                })
                .collect::<Vec<_>>()
                .join(",");
            format!("{{{body}}}")
        }
        Value::Array(items) => format!(
            "[{}]",
            items
                .iter()
                .map(canonical_json)
                .collect::<Vec<_>>()
                .join(",")
        ),
        other => serde_json::to_string(other).unwrap_or_default(),
    }
}

/// Where one preview's plan record lives.
/// 一次预览的计划记录住在哪里。
///
/// Outside the project: a preview must leave the project alone (that promise is pinned by
/// `a_preview_reports_the_resulting_tree_and_leaves_the_project_alone`), so the record is a local
/// session aid, not project state. If it is gone, apply re-decides the rules from the current tree
/// anyway — the safety property does not depend on the record, only the "which premise changed"
/// message does.
/// 在项目之外：预览必须不碰项目（那条承诺由
/// `a_preview_reports_the_resulting_tree_and_leaves_the_project_alone` 钉住），因此记录是本机会话的辅助，
/// 不是项目状态。记录不在了，apply 仍会按当前的树重新判定规则——安全性不依赖记录，依赖它的是"哪个前提变了"
/// 这句话。
fn plan_record_path(key: &str) -> std::path::PathBuf {
    std::env::temp_dir()
        .join("xirang-plan-records")
        .join(format!("{key}.json"))
}

/// Remember the rule facts a preview rested on.
/// 记住一次预览所依托的规则事实。
pub(super) fn remember_plan(
    root: &Path,
    key: &str,
    action: Action,
    subjects: &[String],
    template: Option<&str>,
) {
    let Ok((digest, facts)) = crate::mcp::rules::plan_facts(root, subjects) else {
        return;
    };
    let record = json!({
        "action": action_name(action),
        "subjects": subjects,
        "digest": digest,
        "facts": facts,
        "template": template,
    });
    let path = plan_record_path(key);
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = crate::mcp::check_plan::persist(&path, &record);
}

/// Read the plan record a preview left for this request, when there is one.
/// 读这次请求的预览留下的计划记录——如果有。
pub(super) fn read_plan_record(key: &str) -> Option<Value> {
    let text = std::fs::read_to_string(plan_record_path(key)).ok()?;
    serde_json::from_str(&text).ok()
}

/// Drop a plan record once it has been honoured.
/// 计划被兑现之后丢掉它的记录。
pub(super) fn forget_plan(key: &str) {
    let _ = std::fs::remove_file(plan_record_path(key));
}

/// Refuse an apply whose preview rested on rule facts that have since changed.
/// 拒绝一次 apply：它的预览所依托的规则事实已经变了。
pub(super) fn refuse_if_plan_is_stale(root: &Path, record: &Value) -> Result<(), String> {
    let Some(subjects) = record.get("subjects").and_then(Value::as_array) else {
        return Ok(());
    };
    let subjects = subjects
        .iter()
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect::<Vec<_>>();
    let (digest, facts) = crate::mcp::rules::plan_facts(root, &subjects)?;
    let recorded = record
        .get("digest")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if digest == recorded {
        return Ok(());
    }
    let before = record
        .get("facts")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect::<std::collections::BTreeSet<_>>();
    let now = facts
        .iter()
        .cloned()
        .collect::<std::collections::BTreeSet<_>>();
    let changed = before
        .difference(&now)
        .map(|fact| format!("  was: {fact}"))
        .chain(now.difference(&before).map(|fact| format!("  now: {fact}")))
        .collect::<Vec<_>>()
        .join("\n");
    Err(format!(
        "the preview this apply follows is stale: the project's rules changed after it, and applying it would write under a different premise — nothing was written\n{changed}\nnext   run the preview again so the plan is decided against the current rule book, or revise the rule deliberately (`xirang.rules`) and re-preview\n"
    ))
}

/// The rules a change touched, decided before the project is written.
/// 这次改动触及的规则，在项目被写入之前就已判定。
pub(super) struct Prechecked {
    pub(super) checks: Vec<crate::mcp::rules::RuleCheck>,
}

/// Run the change on a copy, decide the project's rules against the copy's post-change state, and
/// refuse before the project is touched when a rule that constrains is explicitly violated.
/// 在副本上跑这次改动，按副本改动后的状态判定项目规则，并在一条确实约束的规则被明确违反时、在碰项目
/// 之前拒绝。
///
/// Only an explicit violation stops the write. A rule that is unknown, conflicted, stale or whose
/// authority lapsed is reported as an unclosed obligation in the reply — it must never be rendered
/// as satisfied, but it also does not lock a project that is being repaired.
/// 只有明确违规会阻止写入。未知、冲突、过期或授权失效的规则在回复里作为未闭合义务报出——它绝不能被渲染
/// 成满足，但也不该锁死一个正在被修复的项目。
pub(super) fn precheck_rules(
    root: &Path,
    namespace: &str,
    arguments: &Value,
    action: Action,
) -> Result<Prechecked, String> {
    // The pre-check copy carries the package's machine-local state files (bindings): the apply it
    // stands in for sees them, so a copy without them would refuse for the wrong reason.
    // 预检副本带上包的机器本地状态文件（绑定）：它所代表的落盘能看到它们，缺了它们的副本会因错误的原因拒绝。
    let copy = copy_package_with_state(root)?;
    let outcome = match action {
        Action::Add => run_add(&copy, namespace, arguments),
        _ => run_edit(&copy, namespace, arguments, action),
    };
    let outcome = match outcome {
        Ok(outcome) => outcome,
        Err(error) => {
            remove_copy(root, &copy);
            return Err(refused_with_a_way_forward(error));
        }
    };
    let subjects = touched_subjects(&copy, namespace, &outcome, arguments);
    let checks = crate::mcp::rules::check_subjects(root, &copy, &subjects);
    remove_copy(root, &copy);
    let checks = checks?;
    let violations = checks
        .iter()
        .filter(|check| check.blocks())
        .map(crate::mcp::rules::RuleCheck::line)
        .collect::<Vec<_>>();
    if !violations.is_empty() {
        return Err(format!(
            "the change is refused by the project's own rules: nothing was written\n{}\nnext   change the request so the post-change tree satisfies the rule, or revise the rule (`xirang.rules` accept or retire) — applying directly does not bypass a rule\n",
            violations.join("\n")
        ));
    }
    Ok(Prechecked { checks })
}

/// The subjects a change touched, spelled every way a rule selector may name them.
/// 一次改动碰到的那些主体，按规则选择器可能使用的每一种拼法列出。
pub(super) fn touched_subjects(
    work: &Path,
    namespace: &str,
    outcome: &Outcome,
    arguments: &Value,
) -> Vec<String> {
    let mut subjects = Vec::new();
    if let Some(node) = arguments.get("node").and_then(Value::as_str) {
        subjects.push(node.to_owned());
    }
    let Ok(views) = face_views(work, namespace) else {
        subjects.sort();
        subjects.dedup();
        return subjects;
    };
    let Ok(layout) = source_layout(work) else {
        subjects.sort();
        subjects.dedup();
        return subjects;
    };
    for view in views {
        let Ok(file) = view.locate_source(&layout) else {
            continue;
        };
        if file == outcome.source {
            subjects.push(view.path.clone());
            subjects.push(view.module.clone());
            subjects.push(view.id.to_string());
        }
    }
    subjects.sort();
    subjects.dedup();
    subjects
}

/// One reply section: what the rules said about this change.
/// 回复里的一节：规则对这次改动说了什么。
pub(super) fn render_rules(checks: &[crate::mcp::rules::RuleCheck], preview: bool) -> String {
    let mut section = String::from("rules (decided against the post-change tree):\n");
    for check in checks {
        section.push_str(&check.line());
        section.push('\n');
    }
    if preview {
        let refused = checks.iter().filter(|check| check.blocks()).count();
        if refused > 0 {
            section.push_str(&format!(
                "would refuse: {refused} rule(s) explicitly violated — `apply` will not write this change\n"
            ));
        }
    }
    section
}

/// The approved template a preview record promised, when it recorded one.
/// 预览记录承诺的已批准模板（记录过时）。
pub(super) fn plan_template(record: &Value) -> Option<String> {
    record
        .get("template")
        .and_then(Value::as_str)
        .map(str::to_owned)
}

/// Append the template line and the delete-stranded lines to a reply.
/// 把模板行与删除留下的规则行追加到回复里。
pub(super) fn append_reply_lines(text: &mut String, template: &str, stranded: &[String]) {
    text.push_str(template);
    for line in stranded {
        text.push_str(&format!("\n{line}"));
    }
}
