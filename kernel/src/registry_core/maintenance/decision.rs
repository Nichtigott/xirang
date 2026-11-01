//! The one place where a rule's facts become a verdict.
//! 一条规则的事实变成判决的**唯一**地方。
//!
//! Before this module the rule book had two readings of the same rule: `standing` asked "may this
//! rule constrain at all", and the surface asked "does its predicate hold" — and the second one
//! ignored polarity, exceptions, roles, configurations and projections. A `forbid` rule over a
//! capability the object *did* provide was therefore reported `met`, and the same rule over a tree
//! that stopped providing it was reported `violated`: the answer was exactly inverted, and it was
//! inverted in the direction a reader acts on.
//! 在本模块之前，规则簿对同一条规则有两套读法：`standing` 问"这条规则到底能不能约束"，而执行面问
//! "它的谓词成不成立"——后者忽略极性、例外、角色、配置与投影。于是"禁止某对象提供 cap.render"这条规则在
//! 对象**确实**提供它时报 `met`，在树不再提供它时报 `violated`：答案正好反了，而反的方向正是读者会照做的方向。
//!
//! The decision here is pure: the execution surface supplies the subject, the current source prints,
//! the authority's state and the typed predicate's answer; this module supplies applicability,
//! standing, exemptions, conflict and the polarity rule. Two rules are load-bearing:
//! **an unknown predicate is never turned into a pass by negation**, and **a rule that may not
//! constrain never answers `Satisfied`** — not even when its subject would have satisfied it.
//! 这里的判定是纯的：执行面提供主体、当前源码摘要、授权状态与类型化谓词的答案；本模块提供适用性、能否
//! 约束、豁免、冲突与极性规则。两条规矩承重：**未知谓词绝不因取反而变成通过**，以及**不能约束的规则
//! 永不回答 `Satisfied`**——即使它的主体本会满足它。

use super::rules::{Polarity, RuleRecord, RuleState, Standing};

/// The subject a decision is about: its roles, the selector this tree names it by, and the context
/// the rule may be scoped to.
/// 一次判定所针对的主体：它的角色、这棵树命名它的选择器，以及规则可能限定到的上下文。
///
/// An empty `roles` or `configuration` means **the caller did not supply that dimension**, not "the
/// subject has none". The difference decides between "this rule does not apply" and "this decision
/// cannot say" — and the second must never be rendered as the first.
/// 空的 `roles` 或 `configuration` 表示**调用方没有提供那一维**，而不是"主体没有它"。这个区别决定答案是
/// "这条规则不适用"还是"这次判定说不出来"——而后者绝不能被渲染成前者。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RuleSubject {
    /// Roles the caller states for this subject; empty means the caller gave none.
    /// 调用方陈述的主体角色；空表示没有给出。
    pub roles: Vec<String>,
    /// The selector the subject is named by; empty means no subject was named.
    /// 主体的选择器；空表示没有点名主体。
    pub selector: String,
    /// The configuration the caller states; empty means the caller gave none.
    /// 调用方陈述的配置；空表示没有给出。
    pub configuration: String,
    /// The projection the caller states; empty means the caller gave none.
    /// 调用方陈述的投影；空表示没有给出。
    pub projection: String,
    /// The subject's persistent object identity, when the caller could resolve it.
    /// 主体的持续对象身份（当调用方能解析出来时）。
    ///
    /// `None` means "this decision could not resolve the identity", not "the subject has none": a
    /// rule bound to an object must never be applied to a subject whose identity is unknown, because
    /// the selector alone is a spelling another object can reuse.
    /// `None` 表示"这次判定解析不出身份"，不是"主体没有身份"：绑定到对象的规则绝不能适用于身份未知的
    /// 主体，因为选择器本身是一种别的对象可以复用的拼写。
    pub object: Option<crate::identity::StableFaceId>,
}

impl RuleSubject {
    /// A subject named only by its selector, with no context dimensions supplied.
    /// 只用选择器点名的主体，未提供任何上下文维度。
    pub fn named(selector: &str) -> Self {
        Self {
            selector: selector.to_owned(),
            ..Self::default()
        }
    }
}

/// What the execution surface knows about the authority a rule was accepted against.
/// 执行面知道的、关于一条规则据以被接受的授权的事实。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuthorityStanding {
    /// The anchor the rule names.
    /// 规则点名的 anchor。
    pub anchor: String,
    /// Whether the newest ledger record for that anchor is in force.
    /// 该 anchor 在台账里的最新一条是否仍然生效。
    pub in_force: bool,
    /// Why it is not in force, when it is not.
    /// 不生效时的原因。
    pub reason: Option<String>,
}

impl AuthorityStanding {
    /// An anchor whose newest record still holds.
    /// 最新记录仍然成立的 anchor。
    pub fn in_force(anchor: &str) -> Self {
        Self {
            anchor: anchor.to_owned(),
            in_force: true,
            reason: None,
        }
    }

    /// An anchor whose newest record no longer holds, and why.
    /// 最新记录已不成立，并给出原因。
    pub fn lapsed(anchor: &str, reason: &str) -> Self {
        Self {
            anchor: anchor.to_owned(),
            in_force: false,
            reason: Some(reason.to_owned()),
        }
    }
}

/// What the typed predicate said, or that it could not say anything.
/// 类型化谓词给出的答案，或它给不出答案。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PredicateAnswer {
    /// The predicate holds for this tree.
    /// 谓词对这棵树成立。
    Satisfied,
    /// The predicate does not hold for this tree.
    /// 谓词对这棵树不成立。
    NotSatisfied,
    /// The predicate could not be decided, and why.
    /// 谓词判不了，以及原因。
    Unknown(String),
}

/// One rule's complete outcome for one subject.
/// 一条规则对一个主体的完整结果。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RuleOutcome {
    /// The rule does not speak about this subject, and this is the dimension that differs.
    /// 规则不谈这个主体；这里是不同的那一维。
    NotApplicable {
        /// The dimension, spelled so a caller can act on it.
        /// 被点名的维度，调用方可据以行动。
        reason: &'static str,
    },
    /// The rule may not constrain at all, and this is why.
    /// 规则不能约束，原因在此。
    NotConstraining {
        /// One sentence naming what stopped it.
        /// 一句点名拦住它的原因的话。
        reason: String,
    },
    /// The rule applies but this subject is inside an explicit, attributed exception.
    /// 规则适用，但主体落在一条显式、有归属的例外里。
    Exempt {
        /// The scope the exception covers.
        /// 例外覆盖的作用域。
        scope: String,
        /// Who granted it — never empty, because an unattributed exception cannot cover anything.
        /// 谁批准的——永不为空，因为没有归属的例外覆盖不了任何东西。
        authority: String,
        /// Why it was granted.
        /// 为什么批准。
        reason: String,
    },
    /// The rule constrains this subject and it is satisfied.
    /// 规则约束这个主体，而它被满足。
    Satisfied,
    /// The rule constrains this subject and it is violated.
    /// 规则约束这个主体，而它被违反。
    Violated,
    /// The rule constrains this subject, but the tree cannot decide it yet.
    /// 规则约束这个主体，但这棵树还判不了。
    Unknown {
        /// What would be needed to decide it.
        /// 要判定它还缺什么。
        reason: String,
    },
    /// The rule constrains this subject, but it is in a conflict a person has to resolve.
    /// 规则约束这个主体，但它处在一处必须由人解决的冲突里。
    NeedsReview {
        /// The conflict, named.
        /// 被点名的冲突。
        reason: String,
    },
}

impl RuleOutcome {
    /// One stable word, for replies and records.
    /// 一个稳定的词，供回复与记录使用。
    pub fn label(&self) -> &'static str {
        match self {
            Self::NotApplicable { .. } => "not applicable",
            Self::NotConstraining { .. } => "does not constrain",
            Self::Exempt { .. } => "exempt",
            Self::Satisfied => "met",
            Self::Violated => "violated",
            Self::Unknown { .. } => "unknown",
            Self::NeedsReview { .. } => "needs review",
        }
    }

    /// Whether this outcome is evidence that the rule holds for this subject.
    /// 这个结果是否能作为"规则对该主体成立"的证据。
    ///
    /// Only `Satisfied` is. An exemption is a decision **not** to apply the rule, not a proof that
    /// the rule's subject meets it; a non-constraining rule proves nothing about the tree.
    /// 只有 `Satisfied` 是。豁免是"决定不适用这条规则"，不是"主体满足它"的证明；不能约束的规则对树
    /// 不证明任何东西。
    pub fn is_satisfied(&self) -> bool {
        matches!(self, Self::Satisfied)
    }
}

impl RuleRecord {
    /// Decide this rule for one subject, from the facts the execution surface supplies.
    /// 按执行面提供的事实，对一条规则、一个主体做出判定。
    ///
    /// The order is the specification's: applicability first (a rule about another object says
    /// nothing here), then whether it may constrain at all, then whether its authority still holds,
    /// then conflict (a person resolves those), then an explicit exception, and only then the typed
    /// predicate — interpreted through the rule's polarity.
    /// 顺序按规格：先适用性（关于别的对象的规则在这里什么都不说），再是它到底能不能约束，再是它的授权
    /// 是否仍然成立，然后是冲突（由人解决），再是显式例外，最后才是类型化谓词——并按规则的极性解释。
    pub fn decide(
        &self,
        subject: &RuleSubject,
        current: &[(String, String)],
        authority: Option<&AuthorityStanding>,
        conflict: Option<&str>,
        predicate: Option<&PredicateAnswer>,
    ) -> RuleOutcome {
        if let Some(reason) = self.applicability(subject) {
            return reason;
        }
        if let Standing::NotConstraining(reason) = self.standing(current) {
            return RuleOutcome::NotConstraining {
                reason: reason.to_owned(),
            };
        }
        if let Some(anchor) = &self.authority_ref {
            match authority {
                None => {
                    return RuleOutcome::NotConstraining {
                        reason: format!(
                            "its authority `{anchor}` was not resolved against this tree's adoption ledger, so this rule cannot be read as constraining"
                        ),
                    };
                }
                Some(standing) if !standing.in_force => {
                    return RuleOutcome::NotConstraining {
                        reason: format!(
                            "its authority `{}` is no longer in force: {}",
                            standing.anchor,
                            standing
                                .reason
                                .as_deref()
                                .unwrap_or("the newest record lapsed")
                        ),
                    };
                }
                Some(standing) if standing.anchor != *anchor => {
                    return RuleOutcome::NotConstraining {
                        reason: format!(
                            "the authority resolved for this decision is `{}`, but the rule was accepted against `{anchor}`",
                            standing.anchor
                        ),
                    };
                }
                Some(_) => {}
            }
        }
        if let Some(reason) = conflict {
            return RuleOutcome::NeedsReview {
                reason: reason.to_owned(),
            };
        }
        if let Some(exception) = self.exception_for(&subject.selector) {
            return RuleOutcome::Exempt {
                scope: exception.scope.clone(),
                authority: exception.authority.clone(),
                reason: exception.reason.clone(),
            };
        }
        match predicate {
            None => RuleOutcome::Unknown {
                reason: format!(
                    "this decider does not evaluate `{}`; it needs an executable check reference or a semantic review before it can answer met or violated",
                    self.requirement
                ),
            },
            Some(PredicateAnswer::Unknown(reason)) => RuleOutcome::Unknown {
                reason: reason.clone(),
            },
            Some(PredicateAnswer::Satisfied) => match self.polarity {
                Polarity::Require => RuleOutcome::Satisfied,
                Polarity::Forbid => RuleOutcome::Violated,
            },
            Some(PredicateAnswer::NotSatisfied) => match self.polarity {
                Polarity::Require => RuleOutcome::Violated,
                Polarity::Forbid => RuleOutcome::Satisfied,
            },
        }
    }

    /// Whether the rule speaks about this subject, and why not when it does not.
    /// 规则是否谈这个主体；不谈时说明为什么。
    ///
    /// A dimension the **caller did not supply** is `Unknown`, not `NotApplicable`: this decision
    /// cannot tell whether the rule speaks about the subject, and answering "not applicable" would
    /// quietly drop a constraint that may well apply.
    /// **调用方没有提供**的维度是 `Unknown`，不是 `NotApplicable`：这次判定说不出规则是否谈这个主体，而
    /// 回答"不适用"会悄悄丢掉一条很可能适用的约束。
    fn applicability(&self, subject: &RuleSubject) -> Option<RuleOutcome> {
        if subject.selector.trim().is_empty() {
            return Some(RuleOutcome::Unknown {
                reason: "no subject selector was supplied, so no rule can be matched to an object"
                    .to_owned(),
            });
        }
        // The binding is the identity half of applicability, and it is checked **before** the
        // selector: after a move the selector may still spell the old location (until the migration
        // updates it), while the object is the same one. A bound rule therefore asks the identity,
        // never the spelling — and a different object that reuses the old path is not the subject.
        // 绑定是适用性的身份那一半，先于选择器检查：搬动之后选择器可能还写着旧位置（直到迁移更新它），
        // 而对象仍是同一个。因此绑定规则问身份、不问拼写——而复用旧路径的另一个对象不是它的主体。
        if let Some(binding) = &self.object {
            match &subject.object {
                None => {
                    return Some(RuleOutcome::Unknown {
                        reason: format!(
                            "this rule is bound to object {} (`{}`); the decision could not resolve the subject's persistent identity",
                            binding.stable, binding.path
                        ),
                    });
                }
                Some(identity) if identity != &binding.stable => {
                    return Some(RuleOutcome::NotApplicable {
                        reason: "the subject is a different object than the one this rule is bound to",
                    });
                }
                Some(_) => {}
            }
        }
        if !self.scope.roles.is_empty() {
            if subject.roles.is_empty() {
                return Some(RuleOutcome::Unknown {
                    reason: format!(
                        "this rule speaks about the role(s) {}; the decision was given no roles for the subject",
                        self.scope.roles.join(", ")
                    ),
                });
            }
            if !self
                .scope
                .roles
                .iter()
                .any(|role| subject.roles.contains(role))
            {
                return Some(RuleOutcome::NotApplicable {
                    reason: "the subject's role is not one this rule speaks about",
                });
            }
        }
        if !self.scope.configurations.is_empty() {
            if subject.configuration.is_empty() {
                return Some(RuleOutcome::Unknown {
                    reason: format!(
                        "this rule was accepted for the configuration(s) {}; the decision names no configuration for the subject",
                        self.scope.configurations.join(", ")
                    ),
                });
            }
            if !self
                .scope
                .configurations
                .iter()
                .any(|known| known == &subject.configuration)
            {
                return Some(RuleOutcome::NotApplicable {
                    reason: "the subject is not under a configuration this rule was accepted for",
                });
            }
        }
        if subject.projection.is_empty() {
            return Some(RuleOutcome::Unknown {
                reason: "the decision names no projection for the subject".to_owned(),
            });
        }
        if self.scope.projection != subject.projection {
            return Some(RuleOutcome::NotApplicable {
                reason: "the subject is under a different projection than the one this rule speaks about",
            });
        }
        // Only an unbound rule is matched by spelling. A bound one already answered the identity
        // question above; its selector is a coordinate the migration keeps fresh, not the key.
        // 只有未绑定的规则按拼写匹配。绑定规则已在上面的身份问题里作答；它的选择器是迁移会保持新鲜的
        // 坐标，而不是键。
        if self.object.is_none() && self.scope.selector != subject.selector {
            return Some(RuleOutcome::NotApplicable {
                reason: "the subject is not the entity this rule selects",
            });
        }
        None
    }
}

/// The conflict reason a rule is in, when a typed or worded contradiction names it.
/// 当一条类型化或措辞上的矛盾点名某条规则时，它处在什么冲突里。
///
/// Same scope, opposite polarity, and **either** the same requirement text **or** the same typed
/// expectation. The typed half is what makes the contradiction reliable: two rules that name the
/// same `(field, value)` in different words are still the same commitment, and a decider that only
/// compared prose would miss the one conflict it can actually prove.
/// 同作用域、反极性，且**要么**要求文本相同**要么**类型化期望相同。类型化那一半才是矛盾可靠的原因：
/// 两条用不同措辞点名同一个 `(字段, 值)` 的规则仍是同一个承诺，而只比较散文的判定器会漏掉它唯一能证明的冲突。
pub fn conflict_reason(rule: &RuleRecord, other: &RuleRecord) -> Option<String> {
    // `Conflicted` is included on purpose: `check` records that state once it has seen the
    // contradiction, and a conflicted rule must keep being reported as conflicted — otherwise the
    // second read of the same book would silently drop the conflict it just recorded.
    // 有意包含 `Conflicted`：`check` 在看到矛盾后写下这个状态，而一条已冲突的规则必须继续被报成冲突——
    // 否则同一本册子的第二次读取会静默丢掉它刚刚记录下的冲突。
    if !matches!(rule.state, RuleState::Accepted | RuleState::Conflicted)
        || !matches!(other.state, RuleState::Accepted | RuleState::Conflicted)
    {
        return None;
    }
    if !rule.contradicts(other) {
        return None;
    }
    Some(format!(
        "`{}` and `{}` are accepted rules over the same scope with opposite polarity; a person resolves this, and a newer revision does not win by itself",
        rule.rule_id, other.rule_id
    ))
}

#[cfg(test)]
#[path = "decision_tests.rs"]
mod decision_tests;
