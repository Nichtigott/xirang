//! The project's rule book: proposals, accepted commitments, and the memory that is allowed to decay.
//! 项目的规则簿：提议、已接受的承诺，以及**允许衰减**的记忆。
//!
//! The vocabulary and every decision about it live in `xirang_kernel::maintenance` (origin, authority,
//! state, applicability, conflict, staleness). This module is the surface: where the records live, who
//! may write one, and what a reader is told. Two rules of that surface are the point of it:
//! **a rule is accepted only against an authority that already exists in this tree's adoption
//! ledger** — so there is no second, synonymous "trusted list" to keep in step with the first — and
//! **a rule that may not constrain never answers "satisfied"**; it reports the dimension that stopped
//! it, because a stale rule answering `met` is an expired credential wearing a green badge.
//! 词汇与关于它的每个判定都住在 `xirang_kernel::maintenance`（来源、授权、状态、适用性、冲突、失效）。
//! 本模块是执行面：记录住在哪里、谁可以写一条、以及读者会被告知什么。这个执行面的两条规矩就是它的要点：
//! **一条规则只能对照这棵树采信台账里**已经存在**的授权被接受**——因此不存在第二份需要与第一份保持同步的
//! 同义"可信清单"——以及**不能约束的规则永不回答"满足"**；它报出拦住它的那个维度，因为一条过期规则回答
//! `met`，就是一张戴着绿标的过期凭据。
//!
//! online: a rule's standing is a statement about the bytes in this tree right now, and a tree that
//! changed since the rule was accepted has sources the acceptance never saw.
//! online：一条规则是否成立，是关于这棵树**此刻**字节的陈述；规则被接受之后变过的树里，有那次接受从未见过的源。

use std::path::{Path, PathBuf};

use serde_json::{Value, json};
use xirang_kernel::lexicon::XIRANG_DIR;
use xirang_kernel::maintenance::{
    Polarity, RuleBinding, RuleCategory, RuleException, RuleOrigin, RuleRecord, RuleScope,
    RuleState, Standing, conflicted,
};

#[path = "rules_plan.rs"]
mod plan;

pub(super) use plan::{
    DecisionContext, MovedObject, RuleCheck, RuleWrite, authority_in_force, binding_for,
    check_subjects, expected_revision, plan_facts, plan_move, plan_removal, render_book,
    resolve_subject,
};

/// The tool's own description, next to the surface it describes.
/// 工具自己的描述，放在它所描述的执行面旁边。
pub(super) const DESCRIPTION: &str = "The project's rule book: proposals, accepted commitments, and the memory that is allowed to decay. \
     `propose` writes a rule with no authority; it **constrains nothing** until it is accepted. \
     `accept` requires `confirm` **and** an `authority` whose **newest** record in this tree's adoption ledger (`<root>/.xirang/adopted/entries`) is still in force — a lapsed lease is a decision a person has to renew, so a rule is not accepted against one; a model writing `accepted` is not an authority, and a `derived_candidate` (the experience one repair produced) is refused until it is reviewed as an adopted specimen or an accepted policy. \
     `check` recomputes every recorded source's digest: a rule whose source changed or vanished stops constraining and is recorded `stale`, and two accepted rules that require and forbid the same thing over the same scope are both recorded `conflicted` and **neither wins** — a newer revision never withdraws an older commitment. \
     `list` reads; `retire` withdraws a rule on purpose and needs a reason on the record. \
     `list`/`check` accept an optional `subject` object (`roles`, `configuration`, `projection`, `selector`): a rule whose scope needs a dimension the request did not supply is reported `unknown`, never `met` — a listing without the subject's context has not checked that configuration. \
     This decider does not evaluate a rule's predicate: a rule that does constrain is reported `unknown` with the executable check or semantic review it still needs, never `met`.";

/// The advertised input contract.
/// 广告出去的输入契约。
pub(super) fn schema() -> Value {
    json!({"type":"object","properties":{
        "action":{"type":"string","enum":["propose","accept","exception","check","list","retire"],"description":"Required. `propose`/`accept`/`check`/`retire` write; `list` only reads."},
        "rule_id":{"type":"string","description":"Stable handle, letters/digits/`_`/`-`/`.`/`:` only; it is also the file name."},
        "category":{"type":"string","enum":["responsibility","shape","protocol","route","style","delivery"]},
        "selector":{"type":"string","description":"The entity the rule speaks about, spelled the way this tree names objects."},
        "requirement":{"type":"string","description":"A typed structural predicate, an executable check reference, or a semantic review."},
        "polarity":{"type":"string","enum":["require","forbid"],"default":"require"},
        "roles":{"type":"array","maxItems":16,"items":{"type":"string"},"description":"Roles the rule speaks about; omit for any role."},
        "configurations":{"type":"array","maxItems":16,"items":{"type":"string"},"description":"Configurations it was accepted for; omit for any."},
        "projection":{"type":"string","default":"base","description":"`base`, or `graft(<id>)`."},
        "expects":{"type":"object","description":"One `{declares|provides|requires: \"value\"}` pair. A rule that names one is decided by the same function `xirang.constraints` renders; a rule that does not stays `unknown` and says what it needs."},
        "sources":{"type":"array","maxItems":64,"items":{"type":"string"},"description":"Package-relative files this rule rests on; their digests are recorded now and re-checked by `check`."},
        "origin":{"type":"string","enum":["accepted_policy","adopted_specimen","derived_candidate"],"default":"derived_candidate"},
        "authority":{"type":"string","description":"`accept`/`exception`: an anchor whose newest record in the adoption ledger must still be in force; a lapsed lease needs a person to renew it before a rule can rest on it."},
        "scope":{"type":"string","description":"`exception`: the one narrow scope where this rule does not apply."},
        "confirm":{"type":"boolean","description":"`accept`/`retire`: the request has to say so; nothing is accepted by silence."},
        "reason":{"type":"string","description":"`retire`: why. The record is what the next reader has."},
        "subject":{"type":"object","description":"`list`/`check`: the subject's context, so applicability can be decided. A rule whose scope needs `roles` or `configurations` this object does not supply stays `unknown`; without it a listing assumes only `projection: base` and the rule's own selector.","properties":{"roles":{"type":"array","maxItems":16,"items":{"type":"string"}},"configuration":{"type":"string"},"projection":{"type":"string"},"selector":{"type":"string"}}},
        "root":{"type":"string"}
    },"required":["action"]})
}

/// `xirang.rules`: propose, accept, check, list or retire one rule.
/// `xirang.rules`：提议、接受、检查、列出或撤回一条规则。
pub(super) fn rules(root: &Path, arguments: &Value) -> Result<String, String> {
    let root = root.canonicalize().map_err(|error| error.to_string())?;
    let action = arguments
        .get("action")
        .and_then(Value::as_str)
        .ok_or("rules requires an action: propose, accept, check, list or retire")?;
    if !["propose", "accept", "exception", "check", "list", "retire"].contains(&action) {
        return Err(format!(
            "rules action is propose, accept, exception, check, list or retire, not `{action}`; nothing was done"
        ));
    }
    let directory = root.join(XIRANG_DIR).join("rules");
    match action {
        "propose" => propose(&root, arguments, &directory),
        "accept" => accept(&root, arguments, &directory),
        "exception" => exception(&root, arguments, &directory),
        "retire" => retire(arguments, &directory),
        "check" => check_book(&root, &directory, arguments),
        "list" => list(&root, &directory, arguments),
        _ => unreachable!("the action list was checked above"),
    }
}

/// Write a proposal. It carries no authority and constrains nothing.
/// 写一条提议。它不带授权，也不约束任何东西。
fn propose(root: &Path, arguments: &Value, directory: &Path) -> Result<String, String> {
    let id = rule_id(arguments)?;
    let path = directory.join(format!("{id}.json"));
    if path.exists() {
        return Err(format!(
            "a rule `{id}` is already recorded at {}; `accept`, `retire`, or propose under another id — a rule is revised, not overwritten",
            path.display()
        ));
    }
    let category = arguments
        .get("category")
        .and_then(Value::as_str)
        .and_then(RuleCategory::parse_label)
        .ok_or(
            "propose needs a `category`: responsibility, shape, protocol, route, style or delivery",
        )?;
    let selector = required(arguments, "selector")?;
    let requirement = required(arguments, "requirement")?;
    let polarity = match arguments.get("polarity").and_then(Value::as_str) {
        None => Polarity::Require,
        Some(text) => Polarity::parse_label(text).ok_or("polarity is `require` or `forbid`")?,
    };
    let origin = match arguments.get("origin").and_then(Value::as_str) {
        None => RuleOrigin::DerivedCandidate,
        Some(text) => RuleOrigin::parse_label(text)
            .ok_or("origin is accepted_policy, adopted_specimen or derived_candidate")?,
    };
    let scope = RuleScope {
        roles: strings(arguments, "roles")?,
        selector,
        configurations: strings(arguments, "configurations")?,
        projection: arguments
            .get("projection")
            .and_then(Value::as_str)
            .unwrap_or("base")
            .to_owned(),
    };
    // One `(field, value)` pair, because that is what the decider consumes: §2.1 says a rule
    // *references* an existing check, and that check decides one expectation at a time. Refusing a
    // second pair here is what keeps the rule book from silently dropping it.
    // 一对 `(字段, 值)`，因为判定器消费的就是这个形状：§2.1 说规则**引用**既有检查，而它一次判一条期望。
    // 在这里拒绝第二对，正是防止规则簿把它悄悄丢掉。
    let expects = match arguments.get("expects") {
        None => None,
        Some(value) => {
            let pairs = value
                .as_object()
                .filter(|object| object.len() == 1)
                .ok_or("`expects` is one `{field: \"value\"}` pair; a second pair would be dropped by the decider, so it is refused here")?;
            let (field, wanted) = pairs.iter().next().expect("one entry was checked");
            let wanted = wanted
                .as_str()
                .filter(|text| !text.trim().is_empty())
                .ok_or("`expects` needs a non-empty string value")?;
            Some((field.clone(), wanted.to_owned()))
        }
    };
    let sources = digest_sources(root, arguments)?;
    // Bind at proposal time when the selector resolves: the binding is the identity half of what
    // this rule is about, and recording it now means a later move can carry the rule along instead
    // of leaving it pointing at a path another object may reuse.
    // 选择器能解析时就在提议阶段绑定：绑定是这条规则针对什么对象的身份那一半，现在就记下来意味着后续一次
    // 搬动可以带着规则一起走，而不是把它留在一个可能被别的对象复用的路径上。
    // A rule whose selector does not resolve yet is still a lawful rule (the object may be created
    // later), and a tree whose namespace cannot be read cannot bind anything — both stay
    // selector-only, and the reply says which one happened instead of implying a binding exists.
    // 选择器尚未解析出来的规则仍是合法规则（对象可能以后才创建），而读不出命名空间的树也绑定不了任何东西
    // ——两者都保持"只有选择器"，且回复说清是哪一种，而不是暗示存在绑定。
    let (object, unbound_reason) = match binding_for(root, &scope.selector) {
        Ok(binding) => (binding, None),
        Err(error) => (None, Some(error)),
    };
    let record = RuleRecord {
        rule_id: id.clone(),
        revision: 1,
        category,
        origin,
        authority_ref: None,
        scope,
        object,
        requirement,
        polarity,
        expects,
        exceptions: Vec::new(),
        sources,
        state: RuleState::Proposed,
    };
    write_rule(
        directory,
        &record,
        "proposed",
        &format!(
            "selector `{}`; object {}",
            record.scope.selector,
            match &record.object {
                Some(binding) => binding.stable.to_string(),
                None => "unresolved".to_owned(),
            }
        ),
    )?;
    Ok(format!(
        "rule {id} proposed ({} {})\nrecord {}\nobject {}\nstate proposed — this constrains nothing until `accept` names an authority that exists in the adoption ledger\n",
        record.category.label(),
        record.polarity.label(),
        path.display(),
        match &record.object {
            Some(binding) => format!("{} (`{}`, `{}`; bound by identity, so a move carries it)", binding.stable, binding.path, binding.source),
            None => match &unbound_reason {
                Some(reason) => format!("selector-only (identity could not be resolved: {reason})"),
                None => "selector-only (no face in this tree matches it yet; it is not bound to an object)".to_owned(),
            },
        }
    ))
}

/// Accept a rule against an authority that already exists in this tree's adoption ledger.
/// 对照这棵树采信台账里**已经存在**的授权接受一条规则。
fn accept(root: &Path, arguments: &Value, directory: &Path) -> Result<String, String> {
    confirmed(arguments, "accept")?;
    let id = rule_id(arguments)?;
    let authority = required(arguments, "authority")?;
    let mut record = read_rule(directory, &id)?;
    // The whole point of the cross-check: a rule's authority is a lease somebody already confirmed,
    // so this surface cannot grow a second "trusted list" that drifts from the first.
    // 这次交叉核对的全部要点：一条规则的授权是**别人已经确认过**的一份租约，因此这个执行面长不出第二份会与
    // 第一份漂移的"可信清单"。
    authority_in_force(root, &authority, "a rule is accepted against")?;
    if let Some(expected) = expected_revision(arguments)?
        && expected != record.revision
    {
        return Err(format!(
            "concurrent edit: rule `{id}` is at revision {}, not {expected}; re-read the rule and retry — a later write must not overwrite a revision it never saw",
            record.revision
        ));
    }
    // The binding is re-resolved at the moment of acceptance, because that is the decision: the
    // object this rule constrains is the one the tree has now. A selector that now names a
    // different object is refused rather than silently rebound — a rule is a commitment about an
    // object, not about whichever face happens to occupy a path.
    // 绑定在接受的那一刻重新解析，因为那才是决定：这条规则约束的对象是树现在的那一个。选择器现在点名另一个
    // 对象时拒绝而不是静默换绑——规则是对一个对象的承诺，不是对恰好占据某个路径的那个面的承诺。
    let resolved = match resolve_subject(root, &record.scope.selector) {
        Ok(resolved) => resolved,
        Err(error) if record.object.is_some() => {
            return Err(format!(
                "this rule is bound to object {} and the subject's identity cannot be resolved: {error}; accepting it would leave the commitment resting on a path this surface cannot check",
                record
                    .object
                    .as_ref()
                    .map(|binding| binding.stable.to_string())
                    .unwrap_or_default()
            ));
        }
        Err(_) => None,
    };
    match (&record.object, resolved) {
        (Some(binding), Some(found)) if found.stable != binding.stable => {
            return Err(format!(
                "the object this rule is bound to is no longer at `{}`: a different object occupies it now (stable {} vs {}); a rule is not silently rebound — retire it, or propose a new rule about the object that is there",
                record.scope.selector, binding.stable, found.stable
            ));
        }
        (Some(binding), found) => {
            if let Some(found) = found {
                record.object = Some(RuleBinding {
                    stable: binding.stable,
                    path: found.path,
                    source: found.source,
                });
            } else if let Some(existing) = record.object.as_mut() {
                let _ = existing;
            }
        }
        (None, Some(found)) => {
            record.object = Some(RuleBinding {
                stable: found.stable,
                path: found.path,
                source: found.source,
            });
        }
        (None, None) => {}
    }
    let revision = record.revision + 1;
    record.accept(&authority, revision)?;
    write_rule(
        directory,
        &record,
        "accepted",
        &format!(
            "authority `{authority}`; object {}",
            record
                .object
                .as_ref()
                .map(|binding| format!("{} (`{}`)", binding.stable, binding.path))
                .unwrap_or_else(|| "selector-only".to_owned())
        ),
    )?;
    Ok(format!(
        "rule {id} accepted at revision {revision}\nauthority {authority} (an existing adoption entry)\nobject {}\nstate accepted — it constrains while every recorded source still holds; `check` re-derives that\n",
        match &record.object {
            Some(binding) => format!("{} (`{}`)", binding.stable, binding.path),
            None => "selector-only (nothing in this tree matches the selector yet)".to_owned(),
        }
    ))
}

/// Grant an exception, inside its own narrow scope and against an authority that exists.
/// 授予一条例外——在它自己的窄作用域里，且对照一个**存在**的授权。
///
/// The kernel already refuses to let an exception without an authority cover anything; this is the
/// write side of that rule, and it is deliberately the narrowest of the write actions: it cannot
/// widen a rule, only name a place where the rule is not applied, and it says which place. An
/// exception that reached beyond its own scope would be the rule quietly not applying, which is the
/// failure this whole ledger exists to make visible.
/// 内核已经拒绝让没有授权的例外覆盖任何东西；这里是那条规则的**写**侧，而且它有意是所有写动作里最窄的一个：
/// 它不能扩大一条规则，只能点名一处不适用这条规则的地方，并说出是哪一处。越出自己作用域的例外，就是那条规则
/// 在悄悄不适用——正是这本台账存在所要让之可见的失败。
fn exception(root: &Path, arguments: &Value, directory: &Path) -> Result<String, String> {
    confirmed(arguments, "exception")?;
    let id = rule_id(arguments)?;
    let scope = required(arguments, "scope")?;
    let reason = required(arguments, "reason")?;
    let authority = required(arguments, "authority")?;
    let mut record = read_rule(directory, &id)?;
    if let Some(expected) = expected_revision(arguments)?
        && expected != record.revision
    {
        return Err(format!(
            "concurrent edit: rule `{id}` is at revision {}, not {expected}; re-read the rule and retry",
            record.revision
        ));
    }
    // The same cross-check `accept` makes, for the same reason: an exception is a decision, and a
    // decision this surface cannot attribute is not one it may record.
    // 与 `accept` 同一次交叉核对、同一个理由：例外是一个决定，而这个执行面无法归属的决定，不是它可以记录的
    // 决定。
    authority_in_force(root, &authority, "an exception is granted against")?;
    if record
        .exceptions
        .iter()
        .any(|existing| existing.scope == scope)
    {
        return Err(format!(
            "rule {id} already has an exception for `{scope}`; an exception is a revision, not a pile of duplicates — `retire` the rule or pick the scope that actually differs"
        ));
    }
    let revision = record.revision + 1;
    record.exceptions.push(RuleException {
        scope: scope.clone(),
        reason: reason.clone(),
        authority: authority.clone(),
    });
    record.revision = revision;
    write_rule(
        directory,
        &record,
        "exception",
        &format!("scope `{scope}`; authority `{authority}`; reason: {reason}"),
    )?;
    Ok(format!(
        "rule {id} exception at revision {revision}\nscope {scope}\nreason {reason}\nauthority {authority} (an existing adoption entry)\nthis exception covers `{scope}` and nothing else: everywhere else the rule still applies, and `check` re-derives whether it can
"
    ))
}

/// Withdraw a rule on purpose.
/// 有意撤回一条规则。
fn retire(arguments: &Value, directory: &Path) -> Result<String, String> {
    confirmed(arguments, "retire")?;
    let id = rule_id(arguments)?;
    let reason = required(arguments, "reason")?;
    let mut record = read_rule(directory, &id)?;
    if let Some(expected) = expected_revision(arguments)?
        && expected != record.revision
    {
        return Err(format!(
            "concurrent edit: rule `{id}` is at revision {}, not {expected}; re-read the rule and retry",
            record.revision
        ));
    }
    let revision = record.revision + 1;
    record.retire(&reason, revision)?;
    write_rule(directory, &record, "retired", &format!("reason: {reason}"))?;
    Ok(format!(
        "rule {id} retired at revision {revision}\nreason {reason}\nstate retired — the record stays, so a withdrawal is never silent\n"
    ))
}

/// Re-derive every rule's standing from the bytes as they stand now, then decide each one.
/// 按当前的字节重新推导每条规则是否成立，然后逐条判定。
fn check_book(root: &Path, directory: &Path, arguments: &Value) -> Result<String, String> {
    let mut records = all(directory)?;
    if records.is_empty() {
        return Ok(format!(
            "no rules recorded under {}\na rule book with no rules is not evidence that the project has none\n",
            directory.display()
        ));
    }
    for record in &mut records {
        let current = current_digests(root, &record.sources)?;
        if let Standing::NotConstraining(reason) = record.standing(&current)
            && record.state == RuleState::Accepted
        {
            record.state = RuleState::Stale;
            record.revision += 1;
            write_rule(directory, record, "marked stale", reason)?;
        }
    }
    let conflicts = conflicted(&records);
    for record in &mut records {
        if conflicts.contains(&record.rule_id) && record.state == RuleState::Accepted {
            record.state = RuleState::Conflicted;
            record.revision += 1;
            write_rule(
                directory,
                record,
                "marked conflicted",
                "two accepted rules contradict each other over the same scope",
            )?;
        }
    }
    let records = all(directory)?;
    let context = DecisionContext::from_request(arguments)?;
    Ok(render_book(root, &records, &context))
}

/// Read the book without changing it.
/// 只读地把规则簿读出来。
fn list(root: &Path, directory: &Path, arguments: &Value) -> Result<String, String> {
    let records = all(directory)?;
    let context = DecisionContext::from_request(arguments)?;
    Ok(render_book(root, &records, &context)
        + "list only reads: nothing was run and nothing was written\n")
}

fn rule_id(arguments: &Value) -> Result<String, String> {
    let id = required(arguments, "rule_id")?;
    if id.len() > 120
        || !id
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || "_-.:".contains(character))
    {
        return Err(
            "rule_id must be at most 120 characters of letters, digits, `_`, `-`, `.` or `:` — it is also the file name"
                .to_owned(),
        );
    }
    Ok(id)
}

/// A required non-empty string argument.
/// 一个必填的非空字符串参数。
fn required(arguments: &Value, key: &str) -> Result<String, String> {
    arguments
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty() && value.len() <= 1024)
        .map(str::to_owned)
        .ok_or_else(|| format!("`{key}` is required and must be 1..1024 UTF-8 bytes"))
}

/// An optional array of short strings.
/// 一个可选的短字符串数组。
fn strings(arguments: &Value, key: &str) -> Result<Vec<String>, String> {
    let Some(value) = arguments.get(key) else {
        return Ok(Vec::new());
    };
    let rows = value
        .as_array()
        .filter(|rows| rows.len() <= 16)
        .ok_or_else(|| format!("`{key}` must be an array of at most 16 strings"))?;
    rows.iter()
        .map(|row| {
            row.as_str()
                .filter(|text| !text.trim().is_empty() && text.len() <= 200)
                .map(str::to_owned)
                .ok_or_else(|| format!("`{key}` entries must be 1..200 UTF-8 bytes"))
        })
        .collect()
}

/// Whether the request actually said `confirm: true`; nothing is accepted by silence.
/// 请求是否真的说了 `confirm: true`；没有任何东西是靠沉默被接受的。
fn confirmed(arguments: &Value, action: &str) -> Result<(), String> {
    if arguments.get("confirm").and_then(Value::as_bool) == Some(true) {
        return Ok(());
    }
    Err(format!(
        "`{action}` needs `confirm: true`: a rule that quietly accepted itself is a rule nobody agreed to"
    ))
}

fn digest_sources(root: &Path, arguments: &Value) -> Result<Vec<(String, String)>, String> {
    let mut sources = Vec::new();
    for path in strings(arguments, "sources")? {
        if path.starts_with('/') || path.split('/').any(|part| part == "..") {
            return Err(format!(
                "`sources` entries are package-relative, and `{path}` is not"
            ));
        }
        sources.push((path.clone(), digest(root, &path)?));
    }
    sources.sort();
    Ok(sources)
}

/// The recorded sources as they stand now; a source that is gone is reported as gone, not as `""`.
/// 记录下来的源**现在**的样子；不在的源报"不在"，而不是报空串。
fn current_digests(
    root: &Path,
    sources: &[(String, String)],
) -> Result<Vec<(String, String)>, String> {
    let mut current = Vec::new();
    for (path, _) in sources {
        // A source that cannot be read is simply **absent** from the current view: `standing` treats
        // a missing entry as changed, which is the answer that stops the rule from constraining.
        // 读不到的源就是**不在**当前视图里：`standing` 把缺席当作已变，而那正是让规则停止约束的答案。
        if let Ok(digest) = digest(root, path) {
            current.push((path.clone(), digest));
        }
    }
    Ok(current)
}

/// One file's digest, or the reason it could not be read.
/// 一个文件的摘要，或读不到它的原因。
fn digest(root: &Path, path: &str) -> Result<String, String> {
    let resolved = root.join(path);
    if !crate::mcp::source_index::is_safe_child(root, &resolved) {
        return Err(format!("{path} is outside the package root"));
    }
    let bytes = std::fs::read(&resolved).map_err(|error| format!("{path}: {error}"))?;
    Ok(xirang_kernel::sha256_hex(&bytes))
}

/// One rule's file.
/// 一条规则的文件。
fn path_of(directory: &Path, id: &str) -> PathBuf {
    directory.join(format!("{id}.json"))
}

/// Write one rule through the same write-then-rename every durable record here uses, and append the
/// revision to its history.
/// 用这里每份持久记录都用的"先写临时再 rename"写下一条规则，并把它这一次修订追加到历史。
///
/// The history is the answer to "why is this the rule now": a revision number that only ever appears
/// in the current file cannot say what changed or who changed it. It is append-only on purpose — a
/// rule is revised, not overwritten — and it lives beside the rule as `<id>.history.jsonl`, which the
/// rule reader ignores because it only parses `.json`.
/// 历史回答的是"为什么现在是这样一条规则"：只出现在当前文件里的修订号说不出改了什么、谁改的。它有意是
/// 只追加的——规则是被修订而不是被覆盖——并以 `<id>.history.jsonl` 住在规则旁边；规则读取方只解析
/// `.json`，因此会忽略它。
fn write_rule(
    directory: &Path,
    record: &RuleRecord,
    action: &str,
    detail: &str,
) -> Result<(), String> {
    std::fs::create_dir_all(directory).map_err(|error| error.to_string())?;
    super::check_plan::persist(&path_of(directory, &record.rule_id), &encode(record))?;
    append_history(directory, record, action, detail)
}

/// The append-only history beside one rule.
/// 一条规则旁边只追加的历史。
fn history_path(directory: &Path, id: &str) -> PathBuf {
    directory.join(format!("{id}.history.jsonl"))
}

/// Append one event to a rule's history.
/// 向一条规则的历史追加一个事件。
fn append_history(
    directory: &Path,
    record: &RuleRecord,
    action: &str,
    detail: &str,
) -> Result<(), String> {
    let path = history_path(directory, &record.rule_id);
    let mut text = std::fs::read_to_string(&path).unwrap_or_default();
    if !text.is_empty() && !text.ends_with('\n') {
        text.push('\n');
    }
    let event = json!({
        "revision": record.revision,
        "action": action,
        "at": crate::mcp::freshness::wall_clock(),
        "state": record.state.label(),
        "detail": detail,
    });
    text.push_str(&serde_json::to_string(&event).map_err(|error| error.to_string())?);
    text.push('\n');
    std::fs::create_dir_all(directory).map_err(|error| error.to_string())?;
    let temporary = path.with_extension("tmp");
    std::fs::write(&temporary, text)
        .map_err(|error| format!("cannot write {}: {error}", temporary.display()))?;
    std::fs::rename(&temporary, &path)
        .map_err(|error| format!("cannot publish {}: {error}", path.display()))
}

fn read_rule(directory: &Path, id: &str) -> Result<RuleRecord, String> {
    let path = path_of(directory, id);
    let text = std::fs::read_to_string(&path)
        .map_err(|error| format!("unknown rule `{id}` ({}): {error}", path.display()))?;
    let value: Value =
        serde_json::from_str(&text).map_err(|error| format!("{}: {error}", path.display()))?;
    decode(&value).ok_or_else(|| format!("{}: this record is not a rule", path.display()))
}

/// Every rule in the book, by id.
/// 规则簿里的每条规则，按 id。
pub(crate) fn all(directory: &Path) -> Result<Vec<RuleRecord>, String> {
    let Ok(entries) = std::fs::read_dir(directory) else {
        return Ok(Vec::new());
    };
    let mut records = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|extension| extension.to_str()) != Some("json") {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        let Ok(value) = serde_json::from_str::<Value>(&text) else {
            continue;
        };
        if let Some(record) = decode(&value) {
            records.push(record);
        }
    }
    records.sort_by(|left, right| left.rule_id.cmp(&right.rule_id));
    Ok(records)
}

/// One rule as JSON.
/// 一条规则的 JSON 形态。
fn encode(record: &RuleRecord) -> Value {
    json!({
        "rule_id": record.rule_id,
        "revision": record.revision,
        "category": record.category.label(),
        "origin": record.origin.label(),
        "authority_ref": record.authority_ref,
        "scope": {
            "roles": record.scope.roles,
            "selector": record.scope.selector,
            "configurations": record.scope.configurations,
            "projection": record.scope.projection,
        },
        "object": record.object.as_ref().map(|binding| json!({
            "stable": binding.stable.to_string(),
            "path": binding.path,
            "source": binding.source,
        })),
        "requirement": record.requirement,
        "polarity": record.polarity.label(),
        "exceptions": record.exceptions.iter().map(|exception| json!({
            "scope": exception.scope, "reason": exception.reason, "authority": exception.authority,
        })).collect::<Vec<_>>(),
        "expects": record.expects.as_ref().map(|(field, wanted)| json!({ field.clone(): wanted.clone() })),
        "sources": record.sources.iter().map(|(path, digest)| json!({"path": path, "digest": digest})).collect::<Vec<_>>(),
        "state": record.state.label(),
    })
}

/// One rule from JSON, or `None` when a field this vocabulary requires is missing or unspellable.
/// 从 JSON 读一条规则；本词汇要求的字段缺失或拼不出时为 `None`。
fn decode(value: &Value) -> Option<RuleRecord> {
    let text = |key: &str| value.get(key).and_then(Value::as_str).map(str::to_owned);
    Some(RuleRecord {
        rule_id: text("rule_id")?,
        revision: value.get("revision").and_then(Value::as_u64).unwrap_or(1),
        category: RuleCategory::parse_label(&text("category")?)?,
        origin: RuleOrigin::parse_label(&text("origin")?)?,
        authority_ref: text("authority_ref"),
        scope: RuleScope {
            roles: value["scope"]["roles"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect(),
            selector: value["scope"]["selector"].as_str()?.to_owned(),
            configurations: value["scope"]["configurations"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect(),
            projection: value["scope"]["projection"]
                .as_str()
                .unwrap_or("base")
                .to_owned(),
        },
        object: value.get("object").and_then(|binding| {
            Some(RuleBinding {
                stable: binding.get("stable")?.as_str()?.parse().ok()?,
                path: binding.get("path")?.as_str()?.to_owned(),
                source: binding.get("source")?.as_str()?.to_owned(),
            })
        }),
        requirement: text("requirement")?,
        polarity: Polarity::parse_label(&text("polarity")?)?,
        exceptions: value["exceptions"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|row| {
                Some(RuleException {
                    scope: row["scope"].as_str()?.to_owned(),
                    reason: row["reason"].as_str()?.to_owned(),
                    authority: row["authority"].as_str()?.to_owned(),
                })
            })
            .collect(),
        expects: value["expects"].as_object().and_then(|object| {
            let (field, wanted) = object.iter().next()?;
            Some((field.clone(), wanted.as_str()?.to_owned()))
        }),
        sources: value["sources"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|row| {
                Some((
                    row["path"].as_str()?.to_owned(),
                    row["digest"].as_str()?.to_owned(),
                ))
            })
            .collect(),
        state: RuleState::parse_label(&text("state")?)?,
    })
}

#[cfg(test)]
#[path = "rules_tests.rs"]
mod rules_tests;

#[cfg(test)]
#[path = "rules_identity_tests.rs"]
mod rules_identity_tests;
