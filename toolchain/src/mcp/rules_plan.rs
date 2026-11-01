//! The plan-facing half of the rule book: object identity, the checks a change has to pass, and the
//! rewrites a move owes.
//! 规则簿面向计划的那一半：对象身份、一次改动必须通过的检查，以及一次搬动欠下的改写。
//!
//! The write surface (`rules`) answers "what is recorded and who may record it"; this answers "what
//! does the rule book say about **this** change". Kept apart because both files then fit the
//! repository's size budget, and because the split is the real seam: a preview, an apply and a move
//! all consume this half without going through the propose/accept verbs.
//! 写入面（`rules`）回答"记录了什么、谁能记"；这里回答"规则簿对**这一次**改动怎么说"。分开既因为两个文件
//! 由此都落在仓库的尺寸预算内，也因为这条分界是真的：预览、apply 与搬动都消费这一半，而不经过
//! propose/accept 那几个动词。

use std::path::{Path, PathBuf};

use serde_json::{Value, json};
use xirang_kernel::identity::StableFaceId;
use xirang_kernel::maintenance::{
    AuthorityStanding, PredicateAnswer, RuleBinding, RuleOutcome, RuleRecord, RuleSubject,
    Standing, conflict_reason,
};

use super::{XIRANG_DIR, all, current_digests, encode, history_path, path_of};

/// One subject a selector names, with the persistent identity the decision needs.
/// 选择器点名的一个主体，带上判定所需的持续身份。
pub(crate) struct ResolvedSubject {
    pub(crate) path: String,
    pub(crate) stable: StableFaceId,
    pub(crate) source: String,
}

/// Resolve one selector to this tree's persistent object identity.
/// 把选择器解析成这棵树的持续对象身份。
///
/// A selector is a spelling — a logical path, a module name, or a source instance id — and a
/// spelling can be reused by another object after a move. The ledger's entry wins over the derived
/// seed (`FaceView::object_id`), exactly as `ObjectView::observed` documents: the derivation is a
/// seed, the project's `xirang.objects.tsv` is the authority. `None` means "the selector names no
/// single face", which is a lawful state for a rule about a future object; it is the binding that
/// distinguishes "not resolved yet" from "a different object is here now".
/// 选择器是一种拼写——逻辑路径、模块名或源码实例 id——而拼写可能在搬动后被另一个对象复用。台账条目胜过
/// 推导出的种子（`FaceView::object_id`），正如 `ObjectView::observed` 所写：推导是种子，项目的
/// `xirang.objects.tsv` 才是权威。`None` 表示"选择器没有点名唯一的面"，对于一条关于未来对象的规则而言
/// 是合法状态；区分"还没解析出来"与"现在这里是另一个对象"的是绑定。
pub(crate) fn resolve_subject(
    root: &Path,
    selector: &str,
) -> Result<Option<ResolvedSubject>, String> {
    if selector.trim().is_empty() {
        return Ok(None);
    }
    let namespace = crate::mcp::registry::namespace(root)?;
    let views = crate::build_method::face_views(root, &namespace)?;
    let matched = views
        .iter()
        .filter(|view| {
            view.path == selector || view.module == selector || view.id.to_string() == selector
        })
        .collect::<Vec<_>>();
    if matched.len() != 1 {
        return Ok(None);
    }
    let view = matched[0];
    let layout = crate::build_method::source_layout(root)?;
    let observed = view.observed(&layout, &namespace);
    // `own_current`, not `own`: the live declaration must not be claimed by an entry whose only
    // link to these coordinates is history — that is how a moved object's rule would follow a new
    // object created at its old path.
    // 用 `own_current` 而不是 `own`：活着的声明不能被“只靠历史与这些坐标相连”的条目认领——否则
    // 被搬对象的规则会跟着在它旧路径上新建的另一个对象走。
    let map = xirang_multilang::object_map::ObjectMap::read_from(root)?;
    let owned = map.own_current(&observed)?;
    let stable = owned.map_or(observed.stable, |entry| entry.stable);
    Ok(Some(ResolvedSubject {
        path: observed.path,
        stable,
        source: observed.source,
    }))
}

/// The binding this selector resolves to right now, when it resolves to one face.
/// 这个选择器此刻解析出的绑定（当它解析到唯一的面时）。
pub(crate) fn binding_for(root: &Path, selector: &str) -> Result<Option<RuleBinding>, String> {
    Ok(resolve_subject(root, selector)?.map(|subject| RuleBinding {
        stable: subject.stable,
        path: subject.path,
        source: subject.source,
    }))
}

/// The `expected_revision` a request named, when it named one.
/// 请求点名的 `expected_revision`（当它点名了时）。
pub(crate) fn expected_revision(arguments: &Value) -> Result<Option<u64>, String> {
    match arguments.get("expected_revision") {
        None => Ok(None),
        Some(value) => value
            .as_u64()
            .map(Some)
            .ok_or_else(|| "`expected_revision` must be a non-negative integer".to_owned()),
    }
}

/// One recorded rule decided for one subject in a tree, with the identity a reader can act on.
/// 一条已记录规则在某一棵树里对一个主体做出的判定，并带上读者可据以行动的标识。
pub(crate) struct RuleCheck {
    pub(crate) rule_id: String,
    pub(crate) revision: u64,
    pub(crate) requirement: String,
    pub(crate) selector: String,
    pub(crate) outcome: RuleOutcome,
}

impl RuleCheck {
    /// Whether this check is an explicit violation of a rule that does constrain.
    /// 这条检查是否是一条确实约束的规则被明确违反。
    pub(crate) fn blocks(&self) -> bool {
        matches!(self.outcome, RuleOutcome::Violated)
    }

    /// One line for a reply: what the rule says and what the decision is.
    /// 供回复的一行：规则说了什么、判定是什么。
    pub(crate) fn line(&self) -> String {
        match &self.outcome {
            RuleOutcome::Satisfied => format!(
                "  {} [revision {}] {}: met — {}",
                self.rule_id, self.revision, self.selector, self.requirement
            ),
            RuleOutcome::Violated => format!(
                "  {} [revision {}] {}: VIOLATED — {}",
                self.rule_id, self.revision, self.selector, self.requirement
            ),
            RuleOutcome::Exempt {
                scope,
                authority,
                reason,
            } => format!(
                "  {} [revision {}] {}: exempt ({scope}; {reason}; authority {authority})",
                self.rule_id, self.revision, self.selector
            ),
            RuleOutcome::Unknown { reason } => format!(
                "  {} [revision {}] {}: unknown — {reason}",
                self.rule_id, self.revision, self.selector
            ),
            RuleOutcome::NeedsReview { reason } => format!(
                "  {} [revision {}] {}: needs review — {reason}",
                self.rule_id, self.revision, self.selector
            ),
            RuleOutcome::NotConstraining { reason } => format!(
                "  {} [revision {}] {}: does not constrain — {reason}",
                self.rule_id, self.revision, self.selector
            ),
            RuleOutcome::NotApplicable { reason } => format!(
                "  {} [revision {}] {}: not applicable — {reason}",
                self.rule_id, self.revision, self.selector
            ),
        }
    }
}

/// Resolve and decide every recorded rule that speaks about one of these subjects.
/// 解析并判定每一条谈到这些主体之一的已记录规则。
///
/// The facts come from two trees on purpose: the rule book, its authority ledger and the bytes the
/// rules were accepted against are the **project's**, while the typed predicate is decided against
/// the **work** tree's post-change state (the project for a direct apply, the preview copy for a
/// preview). A rule whose recorded sources moved in the work tree stops constraining — that is the
/// documented decay, not an error.
/// 事实有意来自两棵树：规则簿、其授权台账与规则据以被接受的字节是**项目**的，而类型化谓词针对**工作**
/// 树改动后的状态判定（直接 apply 时是项目，预览时是副本）。记录下来的源在工作树里动过的规则停止约束
/// ——那是规格写明的衰减，不是错误。
pub(crate) fn check_subjects(
    project: &Path,
    work: &Path,
    subjects: &[String],
) -> Result<Vec<RuleCheck>, String> {
    if subjects.is_empty() {
        return Ok(Vec::new());
    }
    let records = all(&project.join(XIRANG_DIR).join("rules"))?;
    if records.is_empty() {
        return Ok(Vec::new());
    }
    let mut checks = Vec::new();
    for record in &records {
        let Some(selector) = subjects
            .iter()
            .find(|subject| **subject == record.scope.selector)
            .cloned()
        else {
            continue;
        };
        let current = current_digests(work, &record.sources)?;
        let authority = authority_for(project, record);
        let conflict = records
            .iter()
            .find_map(|other| conflict_reason(record, other));
        let (predicate, _) = predicate_for(work, record);
        let object = resolve_subject(work, &selector)
            .ok()
            .flatten()
            .map(|subject| subject.stable);
        let subject = RuleSubject {
            roles: Vec::new(),
            selector,
            configuration: String::new(),
            projection: "base".to_owned(),
            object,
        };
        let outcome = record.decide(
            &subject,
            &current,
            authority.as_ref(),
            conflict.as_deref(),
            predicate.as_ref(),
        );
        checks.push(RuleCheck {
            rule_id: record.rule_id.clone(),
            revision: record.revision,
            requirement: record.requirement.clone(),
            selector: record.scope.selector.clone(),
            outcome,
        });
    }
    checks.sort_by(|left, right| left.rule_id.cmp(&right.rule_id));
    Ok(checks)
}

/// The rule facts a preview rests on, and a digest over them.
/// 一次预览所依托的规则事实，以及它们的摘要。
///
/// A preview's plan is only good for the rule book that produced it: a rule added, revised, retired,
/// whose source moved, or whose authority lapsed after the preview is a different premise, and
/// applying the old plan against it would be "previewed one thing, wrote another". The digest covers
/// the facts that can change the decision — identity, revision, state, scope, requirement, source
/// freshness, authority standing — and deliberately not the predicate's answer, because the answer
/// is recomputed from the work tree at apply time.
/// 一份预览的计划只对它产生时的那本规则簿成立：预览之后新增、修订、撤回、源动过或授权失效的规则，是另一个
/// 前提，而对着旧前提应用旧计划就是"预览一件事、写入另一件"。摘要覆盖会改变判定的那些事实——身份、修订、
/// 状态、作用域、要求、源新鲜度、授权状态——有意**不**覆盖谓词的答案，因为答案在 apply 时从工作树重算。
pub(crate) fn plan_facts(
    project: &Path,
    subjects: &[String],
) -> Result<(String, Vec<String>), String> {
    let records = all(&project.join(XIRANG_DIR).join("rules"))?;
    let mut facts = Vec::new();
    for record in &records {
        if !subjects
            .iter()
            .any(|subject| subject == &record.scope.selector)
        {
            continue;
        }
        let current = current_digests(project, &record.sources)?;
        let standing = match record.standing(&current) {
            Standing::Constrains => "constrains".to_owned(),
            Standing::NotConstraining(why) => format!("does not constrain: {why}"),
        };
        let authority = match authority_for(project, record) {
            None => "authority none".to_owned(),
            Some(standing) => format!(
                "authority {} in_force={} {}",
                standing.anchor,
                standing.in_force,
                standing.reason.unwrap_or_default()
            ),
        };
        facts.push(format!(
            "{}|revision {}|{}|{}|{}|{}|{}|{}",
            record.rule_id,
            record.revision,
            record.state.label(),
            record.polarity.label(),
            record.scope.selector,
            record.requirement,
            standing,
            authority
        ));
    }
    facts.sort();
    let digest = xirang_kernel::sha256_hex(facts.join("\n").as_bytes());
    Ok((digest, facts))
}

/// The rule id a request named, validated because it is also the file name.
/// 请求点名的规则 id，因为同时是文件名所以要校验。
/// One rule a move carries with it: the new bytes and the history line that explains them.
/// 一次搬动带走的规则：新字节，以及解释它的历史行。
pub(crate) struct RuleWrite {
    pub(crate) path: PathBuf,
    pub(crate) updated: String,
    pub(crate) history_path: PathBuf,
    pub(crate) history_updated: String,
    pub(crate) description: String,
}

/// One object a move relocates, as the rule book needs it.
/// 一次搬动移动的一个对象，按规则簿需要的形状。
pub(crate) struct MovedObject<'a> {
    pub(crate) old_path: &'a str,
    pub(crate) new_path: &'a str,
    pub(crate) new_source: &'a str,
    /// The identity the object **kept** across the move.
    /// 对象跨这次搬动**保持**的身份。
    pub(crate) stable: StableFaceId,
    /// The identity the object had **before** the move, as a rule would have bound it.
    /// 对象在搬动**之前**的身份，也就是一条规则会绑定的那个。
    ///
    /// When this differs from `stable`, the move re-derives the object's identity (a descendant whose
    /// declaration did not opt into an author-owned name and has no ledger entry). A rule bound to
    /// the old id must then be reported as **left behind**, not silently rebound: the object at the
    /// new path is a different object, and quietly transferring the commitment is exactly the stale
    /// knowledge the rule book exists to prevent.
    /// 它与 `stable` 不同时，这次搬动重新推导了对象的身份（一个没有作者稳定名、也没有台账条目的后代）。
    /// 绑定到旧 id 的规则必须被报成**留在原处**，而不是静默换绑：新路径上的对象是另一个对象，悄悄把承诺
    /// 转移过去，正是规则簿存在要防止的陈旧知识。
    pub(crate) old_stable: StableFaceId,
}

/// What a move owes the rule book: rewrites for the objects that kept their identity, and the rules
/// that could not follow because theirs changed.
/// 一次搬动欠规则簿的东西：为保持身份的对象改写，以及那些因身份改变而跟不过去的规则。
pub(crate) struct MovePlan {
    pub(crate) writes: Vec<RuleWrite>,
    pub(crate) stranded: Vec<String>,
}

/// Plan the rule rewrites a move owes: every rule bound to a moved object follows it.
/// 规划一次搬动欠下的规则改写：每条绑定到被搬对象的规则跟着它走。
///
/// A bound rule is rewritten because the commitment is about the **object**, and the object kept its
/// identity across the move. An unbound rule whose selector spells the old path is bound and rewritten
/// only when the move names exactly that path — the move is the determination ("this path became that
/// path"), so this is the mapping half a tool may do; a selector spelled as a module name is left
/// alone because the move does not carry a module mapping.
/// 绑定规则被改写，因为承诺是关于**对象**的，而对象跨搬动保持了身份。选择器写着旧路径的未绑定规则，只有在
/// 搬动恰好点名那个路径时才被绑定并改写——搬动本身就是那个判定（"这个路径变成了那个路径"），因此这是工具
/// 可以代做的那一半映射；写成模块名的选择器不动，因为搬动不带模块映射。
pub(crate) fn plan_move(root: &Path, moved: &[MovedObject<'_>]) -> Result<MovePlan, String> {
    if moved.is_empty() {
        return Ok(MovePlan {
            writes: Vec::new(),
            stranded: Vec::new(),
        });
    }
    let directory = root.join(XIRANG_DIR).join("rules");
    let mut writes = Vec::new();
    let mut stranded = Vec::new();
    for mut record in all(&directory)? {
        // Identity first: a rule follows the object whose id survived. Only a rule that is still
        // unbound may be attached by the move's spelling, and only when the spelling's object kept
        // its identity — otherwise the move would transfer a commitment to a different object.
        // 身份优先：规则跟随 id 存活下来的那个对象。只有仍未绑定的规则才可能按搬动的拼写挂上去，而且只有
        // 那个拼写指向的对象保持了身份才行——否则这次搬动就是把承诺转给了另一个对象。
        let target = moved.iter().find(|target| match &record.object {
            Some(binding) => binding.stable == target.stable,
            None => record.scope.selector == target.old_path,
        });
        let Some(target) = target else {
            if let Some(binding) = &record.object
                && let Some(changed) = moved
                    .iter()
                    .find(|candidate| candidate.old_stable == binding.stable)
            {
                stranded.push(format!(
                    "rule `{}` was not carried: `{}` → `{}` re-derives that object's identity, so the rule's object no longer exists at either path — retire it, or propose a rule about the object at the new path (its selector `{}` now names no face)",
                    record.rule_id, changed.old_path, changed.new_path, record.scope.selector
                ));
            }
            continue;
        };
        let old_selector = record.scope.selector.clone();
        record.object = Some(RuleBinding {
            stable: target.stable,
            path: target.new_path.to_owned(),
            source: target.new_source.to_owned(),
        });
        if record.scope.selector == target.old_path {
            record.scope.selector = target.new_path.to_owned();
        }
        record.revision += 1;
        let path = path_of(&directory, &record.rule_id);
        let updated =
            serde_json::to_string_pretty(&encode(&record)).map_err(|error| error.to_string())?;
        let history = history_path(&directory, &record.rule_id);
        let mut history_updated = std::fs::read_to_string(&history).unwrap_or_default();
        if !history_updated.is_empty() && !history_updated.ends_with('\n') {
            history_updated.push('\n');
        }
        let event = json!({
            "revision": record.revision,
            "action": "moved",
            "at": crate::mcp::freshness::wall_clock(),
            "state": record.state.label(),
            "detail": format!(
                "the object moved from `{old_selector}` to `{}`; the binding follows it (stable {})",
                record.scope.selector, target.stable
            ),
        });
        history_updated
            .push_str(&serde_json::to_string(&event).map_err(|error| error.to_string())?);
        history_updated.push('\n');
        writes.push(RuleWrite {
            path,
            updated,
            history_path: history,
            history_updated,
            description: format!(
                "rule `{}` follows the object (revision {}; `{old_selector}` → `{}`)",
                record.rule_id, record.revision, record.scope.selector
            ),
        });
    }
    writes.sort_by(|left, right| left.description.cmp(&right.description));
    stranded.sort();
    Ok(MovePlan { writes, stranded })
}

/// The subject context a listing was given, and the two assumptions it states instead of guessing.
/// 一次列举得到的主体上下文，以及它**明说**而不是去猜的两个假设。
///
/// Only `projection` has a default (`base`, the tree's own default), because the rule book is
/// written about this tree's base projection. `roles` and `configuration` have **no** default: a
/// rule accepted for `release` cannot be declared applicable or inapplicable by a listing that was
/// never told which configuration it is looking at.
/// 只有 `projection` 有默认值（`base`，这棵树自己的默认），因为规则簿写的就是本树的 base 投影。
/// `roles` 与 `configuration` **没有**默认值：一条为 `release` 接受的规则，不能被一次从没被告知在看哪个
/// 配置的列举判成适用或不适用。
pub(crate) struct DecisionContext {
    pub(crate) roles: Vec<String>,
    pub(crate) configuration: String,
    pub(crate) projection: String,
    pub(crate) selector: Option<String>,
}

impl DecisionContext {
    pub(crate) fn from_request(arguments: &Value) -> Result<Self, String> {
        let Some(value) = arguments.get("subject") else {
            return Ok(Self {
                roles: Vec::new(),
                configuration: String::new(),
                projection: "base".to_owned(),
                selector: None,
            });
        };
        let subject = value.as_object().ok_or(
            "`subject` must be an object with `roles`, `configuration`, `projection` and/or `selector`",
        )?;
        let roles = match subject.get("roles") {
            None => Vec::new(),
            Some(value) => value
                .as_array()
                .filter(|rows| rows.len() <= 16)
                .ok_or("`subject.roles` must be an array of at most 16 strings")?
                .iter()
                .map(|row| {
                    row.as_str()
                        .filter(|text| !text.trim().is_empty() && text.len() <= 200)
                        .map(|text| text.trim().to_owned())
                        .ok_or_else(|| {
                            "`subject.roles` entries must be 1..200 UTF-8 bytes".to_owned()
                        })
                })
                .collect::<Result<Vec<_>, _>>()?,
        };
        let text = |key: &str| -> Result<Option<String>, String> {
            match subject.get(key) {
                None => Ok(None),
                Some(value) => value
                    .as_str()
                    .filter(|text| !text.trim().is_empty() && text.len() <= 1024)
                    .map(|text| Some(text.trim().to_owned()))
                    .ok_or_else(|| {
                        format!(
                            "`subject.{key}` must be a non-empty string of at most 1024 UTF-8 bytes"
                        )
                    }),
            }
        };
        Ok(Self {
            roles,
            configuration: text("configuration")?.unwrap_or_default(),
            projection: text("projection")?.unwrap_or_else(|| "base".to_owned()),
            selector: text("selector")?,
        })
    }
}

/// The authority fact for one rule: its newest ledger record, or nothing when it names no anchor.
/// 一条规则的授权事实：它最新的台账记录；没有点名 anchor 时为空。
fn authority_for(root: &Path, record: &RuleRecord) -> Option<AuthorityStanding> {
    let anchor = record.authority_ref.as_deref()?;
    Some(match crate::mcp::adopted::anchor_standing(root, anchor) {
        Ok(crate::mcp::adopted::AnchorStanding::InForce) => AuthorityStanding::in_force(anchor),
        Ok(crate::mcp::adopted::AnchorStanding::Lapsed(reason)) => {
            AuthorityStanding::lapsed(anchor, &reason)
        }
        Ok(crate::mcp::adopted::AnchorStanding::Unreadable(reason)) => {
            AuthorityStanding::lapsed(anchor, &format!("its lease cannot be read: {reason}"))
        }
        Ok(crate::mcp::adopted::AnchorStanding::Missing) => {
            AuthorityStanding::lapsed(anchor, "no ledger line names this anchor any more")
        }
        Err(cause) => AuthorityStanding::lapsed(
            anchor,
            &format!("the adoption ledger could not be read: {cause}"),
        ),
    })
}

/// The typed predicate's answer, and the field it was about.
/// 类型化谓词的答案，以及它针对的字段。
fn predicate_for(root: &Path, record: &RuleRecord) -> (Option<PredicateAnswer>, Option<String>) {
    let Some((field, wanted)) = &record.expects else {
        return (None, None);
    };
    let mut expect = serde_json::Map::new();
    expect.insert(field.clone(), json!(wanted));
    let answer = match crate::mcp::constraints::decide_expectation_detail(
        root,
        &record.scope.selector,
        &expect,
    ) {
        Ok(crate::mcp::constraints::ExpectationAnswer::Satisfied) => PredicateAnswer::Satisfied,
        Ok(crate::mcp::constraints::ExpectationAnswer::NotSatisfied { .. }) => {
            PredicateAnswer::NotSatisfied
        }
        Ok(crate::mcp::constraints::ExpectationAnswer::Unknown(reason)) => {
            PredicateAnswer::Unknown(reason)
        }
        Err(cause) => PredicateAnswer::Unknown(cause),
    };
    (Some(answer), Some(field.clone()))
}

/// One line per rule, with the complete decision and the dimension that stopped it when one did.
/// 每条规则一行，带上完整判定；被拦下时带上拦住它的那个维度。
pub(crate) fn render_book(
    root: &Path,
    records: &[RuleRecord],
    context: &DecisionContext,
) -> String {
    let mut output = format!("rules {}\n", records.len());
    for record in records {
        let mut line = format!(
            "  {} [{}] {} {} {}\n",
            record.rule_id,
            record.state.label(),
            record.category.label(),
            record.polarity.label(),
            record.scope.selector
        );
        line.push_str(&format!(
            "    requirement {}\n    sources {}\n",
            record.requirement,
            if record.sources.is_empty() {
                "none recorded — nothing can make this rule stale, which is itself worth knowing"
                    .to_owned()
            } else {
                record
                    .sources
                    .iter()
                    .map(|(path, _)| path.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            }
        ));
        if let Some(binding) = &record.object {
            line.push_str(&format!(
                "    object {} (`{}`; bound by identity, so a move carries it)\n",
                binding.stable, binding.path
            ));
        }
        let current = current_digests(root, &record.sources).unwrap_or_default();
        let authority = authority_for(root, record);
        let conflict = records
            .iter()
            .find_map(|other| conflict_reason(record, other));
        let (predicate, field) = predicate_for(root, record);
        let selector = context
            .selector
            .clone()
            .unwrap_or_else(|| record.scope.selector.clone());
        let resolved = resolve_subject(root, &selector).ok().flatten();
        let object = resolved.as_ref().map(|subject| subject.stable);
        let subject = RuleSubject {
            roles: context.roles.clone(),
            selector,
            configuration: context.configuration.clone(),
            projection: context.projection.clone(),
            object,
        };
        let outcome = record.decide(
            &subject,
            &current,
            authority.as_ref(),
            conflict.as_deref(),
            predicate.as_ref(),
        );
        match &outcome {
            RuleOutcome::NotApplicable { reason } => {
                line.push_str(&format!("    not applicable: {reason}\n"));
            }
            RuleOutcome::NotConstraining { reason } => {
                line.push_str(&format!("    does not constrain: {reason}\n"));
            }
            RuleOutcome::Exempt {
                scope,
                authority,
                reason,
            } => {
                line.push_str(&format!(
                    "    exempt: {reason} (scope {scope}, authority {authority})\n"
                ));
            }
            RuleOutcome::Satisfied => line.push_str(&format!(
                "    met (structural): `{}` decided against this tree\n",
                field.as_deref().unwrap_or("the expectation")
            )),
            RuleOutcome::Violated => line.push_str(&format!(
                "    violated (structural): `{}` decided against this tree\n",
                field.as_deref().unwrap_or("the expectation")
            )),
            RuleOutcome::Unknown { reason } => {
                line.push_str(&format!("    unknown: {reason}\n"));
            }
            RuleOutcome::NeedsReview { reason } => {
                line.push_str(&format!("    needs review: {reason}\n"));
            }
        }
        // A conflict is printed even when the decision stopped for another reason: a stale rule does
        // not stop being conflicted, and the reader resolving it needs both sides.
        // 即使判定因另一个原因停下，冲突也要印出来：一条过期规则不会因此不再冲突，而解决它的读者需要两边。
        if let Some(reason) = conflict
            && !matches!(outcome, RuleOutcome::NeedsReview { .. })
        {
            line.push_str(&format!("    {reason}\n"));
        }
        output.push_str(&line);
    }
    output
}

/// The refusal this surface owes when the lease it was pointed at is not current.
/// 当被点名的租约并不当前时，这个执行面欠下的拒绝。
///
/// "The anchor exists somewhere in the ledger" was the old check, and it let a rule be accepted
/// against a confirmation whose bytes had moved — the probe did exactly that after `conformance`
/// had already said `lapsed`. An adoption is a lease, so what licenses a rule is the **newest**
/// record's state, not the presence of any record.
/// "台账里某处存在这个 anchor"是旧的检查，它让规则可以对照一份字节已经动过的确认被接受——探针在
/// `conformance` 已经说出 `lapsed` 之后正是这么做的。采信是一份租约，因此给规则作依据的是**最新**那条
/// 记录的状态，而不是"存在过某条记录"。
pub(crate) fn authority_in_force(root: &Path, anchor: &str, purpose: &str) -> Result<(), String> {
    let entries = crate::mcp::adopted::entries(root)?;
    match crate::mcp::adopted::anchor_standing(root, anchor) {
        Ok(crate::mcp::adopted::AnchorStanding::InForce) => Ok(()),
        Ok(crate::mcp::adopted::AnchorStanding::Missing) => Err(format!(
            "no adoption in {} names the anchor `{anchor}`; {purpose} a lease somebody confirmed (`xirang.adopted` with apply+confirm), not against a model's own word{}",
            root.join(XIRANG_DIR).join("adopted").display(),
            if entries.is_empty() {
                " — this tree has no adoption entries at all"
            } else {
                ""
            }
        )),
        Ok(crate::mcp::adopted::AnchorStanding::Lapsed(reason)) => Err(format!(
            "the adoption lease `{anchor}` is no longer in force: {reason}; {purpose} a **current** decision, not a historical one — re-confirm the policy (`xirang.adopted` with apply+confirm) or name another anchor. A lapsed lease is not a licence to keep accepting rules: it is exactly the state a person has to renew"
        )),
        Ok(crate::mcp::adopted::AnchorStanding::Unreadable(reason)) => Err(format!(
            "the adoption lease `{anchor}` cannot be judged: {reason}; {purpose} a lease nobody can read, and this surface does not record decisions it cannot attribute"
        )),
        Err(cause) => Err(format!(
            "the adoption ledger could not be read, so no authority can be judged: {cause}"
        )),
    }
}

/// The rules a **delete** leaves behind: commitments about objects the delete removed.
/// **删除**留下不管的规则：对被删对象的承诺。
///
/// A delete has no "new path" to carry a rule to, so the honest act is to name every commitment the
/// delete stranded. The check runs against the tree **after** the remove (the copy, in a preview) and
/// reads the rule book from the project, because a preview promises not to write it.
/// 删除没有"新路径"可以把规则带过去，因此诚实的动作是把这次删除留下的每一条承诺点出来。判定跑在删除
/// **之后**的树上（预览时是副本），而规则簿从项目读，因为预览承诺不写它。
pub(crate) fn plan_removal(
    root: &Path,
    work: &Path,
    removed_prefix: &str,
) -> Result<Vec<String>, String> {
    if removed_prefix.trim().is_empty() {
        return Ok(Vec::new());
    }
    let directory = root.join(XIRANG_DIR).join("rules");
    let mut stranded = Vec::new();
    for record in all(&directory)? {
        let path = record
            .object
            .as_ref()
            .map_or(record.scope.selector.as_str(), |binding| {
                binding.path.as_str()
            });
        let under = path == removed_prefix || path.starts_with(&format!("{removed_prefix}/"));
        if !under {
            continue;
        }
        // Still resolvable means the delete did not actually remove this object (or removed a
        // sibling); a rule there is not stranded by this act.
        // 仍可解析意味着删除没有真的移除这个对象（或移走的是兄弟）；那里的规则不是被这次动作留下的。
        if resolve_subject(work, path).ok().flatten().is_some() {
            continue;
        }
        stranded.push(format!(
            "rule `{}` was not carried: this delete removed the object it is about (`{path}`); retire it, or propose a rule about what replaces it",
            record.rule_id
        ));
    }
    stranded.sort();
    Ok(stranded)
}
