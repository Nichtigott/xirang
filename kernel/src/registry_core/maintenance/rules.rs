//! Accepted rules, and the memory that is allowed to go stale.
//! 已接受的规则，以及**允许变旧**的记忆。
//!
//! A project's memory is not one list. §2 of the memory specification separates a rule's **origin**
//! (an accepted policy, an adopted specimen, a derived candidate), its **authority** (a controller's
//! acceptance event — never a model-written author name), and its **state**, and it separates the
//! three because they decay differently: a derived candidate is not a rule yet, a proposal is not a
//! constraint, and an accepted rule whose source bytes moved is not evidence about the tree in front
//! of you. Everything here is the pure half of that: given a rule and the current file digests, decide
//! whether it may constrain at all, and refuse to let a rule that may not constrain come back as
//! `Met` — a stale rule answering "satisfied" is the same false green as an expired credential.
//! 项目的记忆不是一张清单。记忆规格 §2 把一条规则的**来源**（已接受的政策、采信的标本、派生的候选）、
//! 它的**授权**（控制器的一次接受事件——绝不是模型写的作者姓名）与它的**状态**分开，而分开的理由是它们
//! 变旧的方式不同：派生候选还不是规则，提议不是约束，而源字节动过的已接受规则，不是关于你眼前这棵树的
//! 证据。这里全是这件事的纯那一半：给定一条规则与当前的文件摘要，判定它**到底能不能约束**，并且不让一条
//! 不能约束的规则以 `Met` 回来——一条过期的规则回答"满足"，与一张过期的凭据是同一种假绿。
//!
//! Two rules that are worth stating outright, because both are places where the convenient answer is
//! the wrong one. **A newer rule never retires an accepted older one** (§2.2 step 6): the two are
//! reported `Conflicted` and a person resolves them, because "the newer one wins" silently withdraws a
//! commitment somebody made. **An exception only exists inside its own scope and only with an
//! authority**: an exception without one is a rule that quietly stopped applying.
//! 两条值得直接写明的规矩，因为这两处的省事答案都是错的。**更新的规则从不撤销一条已接受的旧规则**
//! （§2.2 第 6 条）：两条被判为 `Conflicted` 并由人来解决，因为"新的赢"会静默撤回某个人做出过的承诺。
//! **例外只存在于它自己的作用域里，且必须带授权**：没有授权的例外，就是一条悄悄不再适用的规则。

use std::collections::BTreeSet;

use crate::identity::StableFaceId;

/// What a rule is about.
/// 一条规则是关于什么的。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuleCategory {
    /// Which object owns which responsibility.
    /// 哪个对象拥有哪份职责。
    Responsibility,
    /// Required declarations and lawful variation in shape.
    /// 必需声明与形状上的合法变体。
    Shape,
    /// Contracts, units and compatibility duties.
    /// 合同、单位与兼容义务。
    Protocol,
    /// Which route a call must (or must not) take.
    /// 一次调用必须（或不得）走哪条路线。
    Route,
    /// House style that is checkable.
    /// 可机械判定的书写风格。
    Style,
    /// What a delivery must contain.
    /// 一次交付必须包含什么。
    Delivery,
}

impl RuleCategory {
    /// One stable word, for records and replies.
    /// 一个稳定的词，供记录与回复使用。
    pub fn label(&self) -> &'static str {
        match self {
            Self::Responsibility => "responsibility",
            Self::Shape => "shape",
            Self::Protocol => "protocol",
            Self::Route => "route",
            Self::Style => "style",
            Self::Delivery => "delivery",
        }
    }

    /// The category a reply spelled, or `None` for a spelling this vocabulary does not have.
    /// 回复拼出的类别；拼法不在本词汇里时为 `None`。
    pub fn parse_label(text: &str) -> Option<Self> {
        [
            Self::Responsibility,
            Self::Shape,
            Self::Protocol,
            Self::Route,
            Self::Style,
            Self::Delivery,
        ]
        .into_iter()
        .find(|category| category.label() == text)
    }
}

/// Where a rule came from.
/// 一条规则从哪来。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuleOrigin {
    /// A policy the project already accepted.
    /// 项目已接受的政策。
    AcceptedPolicy,
    /// A specimen somebody adopted.
    /// 有人采信过的标本。
    AdoptedSpecimen,
    /// A candidate derived from a repair — **not** a rule yet.
    /// 从一次修复里派生的候选——**还不是**规则。
    DerivedCandidate,
}

impl RuleOrigin {
    /// One stable word.
    /// 一个稳定的词。
    pub fn label(&self) -> &'static str {
        match self {
            Self::AcceptedPolicy => "accepted_policy",
            Self::AdoptedSpecimen => "adopted_specimen",
            Self::DerivedCandidate => "derived_candidate",
        }
    }

    /// The origin a reply spelled.
    /// 回复拼出的来源。
    pub fn parse_label(text: &str) -> Option<Self> {
        [
            Self::AcceptedPolicy,
            Self::AdoptedSpecimen,
            Self::DerivedCandidate,
        ]
        .into_iter()
        .find(|origin| origin.label() == text)
    }
}

/// Whether a rule requires something or forbids it.
/// 一条规则是要求某件事，还是禁止某件事。
///
/// The polarity is what makes a conflict decidable: "this route must be taken" and "this route must
/// not be taken" over the same scope are a contradiction, while two requirements of the same polarity
/// are merely two obligations.
/// 极性正是让冲突可判定的东西：同一个作用域上的"必须走这条路线"与"不得走这条路线"是矛盾，而同极性的两条
/// 要求只是两条义务。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Polarity {
    /// The subject must satisfy the requirement.
    /// 主体必须满足这条要求。
    Require,
    /// The subject must not satisfy it.
    /// 主体不得满足它。
    Forbid,
}

impl Polarity {
    /// One stable word.
    /// 一个稳定的词。
    pub fn label(&self) -> &'static str {
        match self {
            Self::Require => "require",
            Self::Forbid => "forbid",
        }
    }

    /// The polarity a reply spelled.
    /// 回复拼出的极性。
    pub fn parse_label(text: &str) -> Option<Self> {
        [Self::Require, Self::Forbid]
            .into_iter()
            .find(|polarity| polarity.label() == text)
    }
}

/// Where a rule's state is.
/// 一条规则处在哪个状态。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuleState {
    /// Proposed by a model or derived from a repair; **constrains nothing**.
    /// 由模型提议或从修复派生；**什么都不约束**。
    Proposed,
    /// Accepted by an authority; may constrain while its sources still hold.
    /// 被授权接受；在源仍然成立期间可以约束。
    Accepted,
    /// The bytes it was accepted against moved.
    /// 它当初据以被接受的字节动过了。
    Stale,
    /// Another accepted rule contradicts it.
    /// 另一条已接受的规则与它矛盾。
    Conflicted,
    /// Withdrawn on purpose, with a reason on the record.
    /// 有意撤回，理由留在记录上。
    Retired,
}

impl RuleState {
    /// One stable word.
    /// 一个稳定的词。
    pub fn label(&self) -> &'static str {
        match self {
            Self::Proposed => "proposed",
            Self::Accepted => "accepted",
            Self::Stale => "stale",
            Self::Conflicted => "conflicted",
            Self::Retired => "retired",
        }
    }

    /// The state a ledger spelled.
    /// 台账拼出的状态。
    pub fn parse_label(text: &str) -> Option<Self> {
        [
            Self::Proposed,
            Self::Accepted,
            Self::Stale,
            Self::Conflicted,
            Self::Retired,
        ]
        .into_iter()
        .find(|state| state.label() == text)
    }
}

/// Where a rule applies: roles, a selector, configurations and a projection.
/// 一条规则适用于哪里：角色、选择器、配置与投影。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuleScope {
    /// The roles the rule speaks about; empty means "any role".
    /// 规则谈论的角色；空表示"任何角色"。
    pub roles: Vec<String>,
    /// The entity selector, spelled the way this tree names objects.
    /// 实体选择器，按这棵树命名对象的拼法。
    pub selector: String,
    /// The configurations it was accepted for; empty means "any configuration".
    /// 它被接受时的配置；空表示"任何配置"。
    pub configurations: Vec<String>,
    /// The projection it speaks about: `base`, or `graft(<id>)`.
    /// 它谈论的投影：`base`，或 `graft(<id>)`。
    pub projection: String,
}

impl RuleScope {
    /// Why this scope does not cover one subject, or `None` when it does.
    /// 这个作用域为什么不覆盖某个主体；覆盖时为 `None`。
    ///
    /// Every dimension is checked, and the reason names the dimension: "roles do not match" is
    /// actionable, "not applicable" is not.
    /// 每个维度都检查，而理由点名维度："角色不匹配"可行动，"不适用"不可行动。
    pub fn refuses(
        &self,
        roles: &[String],
        selector: &str,
        configuration: &str,
        projection: &str,
    ) -> Option<&'static str> {
        if !self.roles.is_empty() && !self.roles.iter().any(|role| roles.contains(role)) {
            return Some("the subject's role is not one this rule speaks about");
        }
        if self.selector != selector {
            return Some("the subject is not the entity this rule selects");
        }
        if !self.configurations.is_empty()
            && !self
                .configurations
                .iter()
                .any(|known| known == configuration)
        {
            return Some("the subject is not under a configuration this rule was accepted for");
        }
        if self.projection != projection {
            return Some(
                "the subject is under a different projection than the one this rule speaks about",
            );
        }
        None
    }
}

/// An explicit exception, which only exists inside its own scope and only with an authority.
/// 一条显式例外，只存在于它自己的作用域里，且必须带授权。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuleException {
    /// The narrow scope the exception covers.
    /// 例外覆盖的那个窄作用域。
    pub scope: String,
    /// Why it was granted, in the granter's own words.
    /// 为什么批准，用批准者自己的话。
    pub reason: String,
    /// Who granted it. Empty is refused: an exception with no authority is a rule that quietly
    /// stopped applying.
    /// 谁批准的。空值被拒绝：没有授权的例外，就是一条悄悄不再适用的规则。
    pub authority: String,
}

/// The persistent object a rule is about, when the rule was accepted against one.
/// 规则所针对的持续对象——当它是对照一个对象被接受的时候。
///
/// A selector is a **spelling** (a logical path or a module name), and a spelling is reused: after a
/// move, the old path can be occupied by a different object. The binding is the identity half: a
/// rule bound to object `X` applies to `X` wherever it now sits, and **never** to a different object
/// that happens to reuse its old path. `path` and `source` are the coordinates at the moment the
/// binding was recorded; the move migration updates them so a reader can print where the object is.
/// 选择器是一种**拼写**（逻辑路径或模块名），而拼写会被复用：一次搬动之后，旧路径可能被另一个对象占据。
/// 绑定是身份那一半：绑定到对象 `X` 的规则适用于 `X` 现在所在之处，**绝不**适用于碰巧复用旧路径的另一个
/// 对象。`path` 与 `source` 是记录绑定时那一刻的坐标；搬动迁移会更新它们，使读者能印出对象现在在哪。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuleBinding {
    /// The ledger's persistent object identity.
    /// 台账里的持续对象身份。
    pub stable: StableFaceId,
    /// The logical registration path at the moment the binding was recorded.
    /// 记录绑定时那一刻的逻辑注册路径。
    pub path: String,
    /// The package-relative source at the moment the binding was recorded.
    /// 记录绑定时那一刻的包内相对源码。
    pub source: String,
}

/// One rule, as the ledger stores it and as the decider reads it.
/// 一条规则，按台账存它的样子、按判定器读它的样子。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuleRecord {
    /// The rule's stable id.
    /// 规则的稳定 id。
    pub rule_id: String,
    /// A monotonic revision: a policy change is a new revision, not a new rule.
    /// 单调修订号：政策变更是一次新修订，而不是一条新规则。
    pub revision: u64,
    /// What the rule is about.
    /// 规则是关于什么的。
    pub category: RuleCategory,
    /// Where it came from.
    /// 它从哪来。
    pub origin: RuleOrigin,
    /// The controller's acceptance event, when it has one.
    /// 控制器的接受事件（有时）。
    pub authority_ref: Option<String>,
    /// Where it applies.
    /// 它适用于哪里。
    pub scope: RuleScope,
    /// The persistent object it is about, when it was accepted against one.
    /// 它针对的持续对象（当它是照着一个对象被接受的时候）。
    ///
    /// Additive and optional on purpose: a rule recorded before this field existed keeps its
    /// selector-only meaning, and a selector that resolves to no object is still a lawful rule (the
    /// object may be created later). Once bound, identity wins over the spelling.
    /// 有意做成可加的、可选的：这个字段存在之前记录的规则保持"只有选择器"的含义，而解析不到对象的选择器
    /// 仍是一条合法规则（对象可能以后才创建）。一旦绑定，身份胜过拼写。
    pub object: Option<RuleBinding>,
    /// The requirement, as a typed predicate, an executable check reference, or a semantic review.
    /// 要求：类型化谓词、可执行检查的引用，或一次语义审查。
    pub requirement: String,
    /// Whether it requires or forbids.
    /// 它要求还是禁止。
    pub polarity: Polarity,
    /// The expectation this rule can be **decided** by, as one `(field, value)` pair.
    /// 这条规则可被**判定**所依据的那条期望，写成一对 `(字段, 值)`。
    ///
    /// One pair and not a JSON value: this is the pure kernel, so the type is std, and one pair is
    /// exactly what the decider consumes (§2.1 — a rule references an existing check rather than
    /// growing a second one). A rule that names none stays `unknown` and says what it would need.
    /// 一对而不是一个 JSON 值：这里是纯净内核，因此类型只能是 std，而一对正好是判定器消费的形状（§2.1——
    /// 规则引用既有检查，不长出第二份）。没有点名的规则照旧 `unknown`，并说清它需要什么。
    pub expects: Option<(String, String)>,
    /// Its explicit exceptions.
    /// 它的显式例外。
    pub exceptions: Vec<RuleException>,
    /// The files it was accepted against, with the digests accepted at that moment.
    /// 它据以被接受的文件，以及那一刻被接受的摘要。
    pub sources: Vec<(String, String)>,
    /// Where its state is.
    /// 它处在哪个状态。
    pub state: RuleState,
}

/// Why a rule may not constrain, when it may not.
/// 一条规则不能约束时的原因。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Standing {
    /// The rule may constrain subjects its scope covers.
    /// 规则可以约束它作用域覆盖的主体。
    Constrains,
    /// It may not, and this is why. A rule that may not constrain never answers `Met`.
    /// 它不能，而原因在此。不能约束的规则永不回答 `Met`。
    NotConstraining(&'static str),
}

impl RuleRecord {
    /// Accept this rule, if there is an authority to accept it with.
    /// 接受这条规则——前提是有一个可以据以接受的授权。
    ///
    /// A model writing `accepted` is not an authority: the reference must name a controller's
    /// acceptance event, and an empty one is refused here rather than discovered later as a rule
    /// nobody ever agreed to.
    /// 模型写下 `accepted` 不是授权：引用必须点名一次控制器的接受事件，而空引用在这里就被拒绝，而不是日后
    /// 被发现成一条谁也没同意过的规则。
    pub fn accept(&mut self, authority_ref: &str, revision: u64) -> Result<(), String> {
        if authority_ref.trim().is_empty() {
            return Err(
                "accepting a rule needs an authority: a controller's acceptance event, not a model's own word"
                    .to_owned(),
            );
        }
        if self.origin == RuleOrigin::DerivedCandidate {
            return Err(
                "a candidate derived from one repair is not a project rule; it must be reviewed as an adopted specimen or an accepted policy first"
                    .to_owned(),
            );
        }
        self.authority_ref = Some(authority_ref.to_owned());
        self.revision = revision;
        self.state = RuleState::Accepted;
        Ok(())
    }

    /// Withdraw this rule on purpose, with a reason.
    /// 有意撤回这条规则，并给出理由。
    pub fn retire(&mut self, reason: &str, revision: u64) -> Result<(), String> {
        if reason.trim().is_empty() {
            return Err(
                "retiring a rule needs a reason: the record is what the next reader has".to_owned(),
            );
        }
        self.revision = revision;
        self.state = RuleState::Retired;
        Ok(())
    }

    /// Whether this rule may constrain at all, given the current bytes.
    /// 给定当前字节，这条规则到底能不能约束。
    ///
    /// `current` is `(path, digest)` for the files as they stand now. A source this rule was accepted
    /// against that is **missing** counts as changed: the bytes it was judged on are no longer there
    /// to judge it against, and treating "gone" as "same" is how a stale rule keeps constraining.
    /// `current` 是这些文件**现在**的 `(路径, 摘要)`。它当初据以被接受、而**现在不在**的源算作已变：它被
    /// 判定的那批字节已经不在了，而把"没了"当成"一样"，正是一条过期规则继续约束的方式。
    pub fn standing(&self, current: &[(String, String)]) -> Standing {
        match self.state {
            RuleState::Proposed => {
                return Standing::NotConstraining("a proposal is not a constraint");
            }
            RuleState::Retired => {
                return Standing::NotConstraining("this rule was withdrawn on purpose");
            }
            RuleState::Conflicted => {
                return Standing::NotConstraining(
                    "two accepted rules contradict each other and a person has to resolve them",
                );
            }
            RuleState::Accepted | RuleState::Stale => {}
        }
        for (path, accepted) in &self.sources {
            match current.iter().find(|(current, _)| current == path) {
                None => {
                    return Standing::NotConstraining(
                        "a source this rule was accepted against is no longer here",
                    );
                }
                Some((_, digest)) if digest != accepted => {
                    return Standing::NotConstraining(
                        "a source this rule was accepted against changed",
                    );
                }
                Some(_) => {}
            }
        }
        if self.state == RuleState::Stale {
            // The bytes are back; the acceptance still stands, but it is the ledger's job to say so.
            // 字节回来了；接受仍然有效，但说这件事是台账的责任。
            return Standing::Constrains;
        }
        Standing::Constrains
    }

    /// Whether one of this rule's exceptions covers a subject, and why it does not when it does not.
    /// 这条规则的某条例外是否覆盖某个主体；不覆盖时说明原因。
    pub fn exception_for(&self, scope: &str) -> Option<&RuleException> {
        self.exceptions
            .iter()
            .find(|exception| exception.scope == scope && !exception.authority.trim().is_empty())
    }

    /// Whether two accepted rules contradict each other.
    /// 两条已接受的规则是否相互矛盾。
    ///
    /// Same scope, same requirement, opposite polarity. The comparison is deliberately narrow: two
    /// rules that merely disagree in wording are a review question, and inventing a semantic
    /// comparison would make this decider guess.
    /// 同作用域、同要求、反极性。比较是**有意收窄**的：仅仅措辞不同的两条规则是审查问题，而发明一种语义比较
    /// 会让这个判定器去猜。
    pub fn contradicts(&self, other: &Self) -> bool {
        let same_commitment = self.requirement == other.requirement
            || (self.expects.is_some() && self.expects == other.expects);
        self.rule_id != other.rule_id
            && self.scope == other.scope
            && same_commitment
            && self.polarity != other.polarity
    }
}

/// Which accepted rules contradict each other, so both can be reported rather than one silently winning.
/// 哪些已接受的规则相互矛盾，使两条都被报出，而不是一条静默胜出。
///
/// Returns the ids in conflict, sorted and deduplicated. A rule in this list does not constrain: the
/// convenient answer — let the newer revision win — is the one §2.2 step 6 forbids.
/// 返回冲突中的 id，已排序去重。在这张清单里的规则不约束：那个省事的答案——让更新的修订胜出——正是
/// §2.2 第 6 条禁止的那一个。
pub fn conflicted(rules: &[RuleRecord]) -> Vec<String> {
    let mut ids: BTreeSet<String> = BTreeSet::new();
    for (index, rule) in rules.iter().enumerate() {
        if rule.state != RuleState::Accepted {
            continue;
        }
        for other in rules.iter().skip(index + 1) {
            if other.state == RuleState::Accepted && rule.contradicts(other) {
                ids.insert(rule.rule_id.clone());
                ids.insert(other.rule_id.clone());
            }
        }
    }
    ids.into_iter().collect()
}

#[cfg(test)]
#[path = "rules_tests.rs"]
mod rules_tests;
