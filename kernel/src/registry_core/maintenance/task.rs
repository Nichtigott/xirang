//! What a maintenance task has established, and the one rule that decides whether it may close.
//! 一条维护任务已经确立了哪些事，以及决定它能否闭合的那一条规则。
//!
//! The review's P4 asks for one layer that connects the authoring plan, the check ledger, the rule
//! book, the object map and the knowledge updates — and it asks that layer to refuse closure until
//! every one of them is disposed. This module owns only that composition: the five obligation kinds,
//! the state each kind's evidence can be in, and the verdict. Producing the evidence (reading a plan,
//! deciding a rule, fingerprinting a lease) belongs to the execution surface.
//! 复核的 P4 要一层把创作计划、检查台账、规则簿、对象映射与知识更新连起来的东西，而且要它**在每一件都被
//! 处置之前拒绝闭合**。本模块只拥有这个合成：五种义务、每种义务的证据可能处的状态，以及判定。产生证据
//! （读计划、判规则、取租约指纹）属于执行面。
//!
//! Two boundaries are deliberately part of the type, not of a caller's memory:
//! **a missing obligation is an error, not a satisfied one** — a task that silently drops "verified"
//! would read as done while nothing ran; and **an undecidable obligation is never complete** — the
//! difference between "satisfied" and "could not tell" is the difference between closing and
//! pretending.
//! 两条边界被刻意放进类型而不是留给调用方记着：**缺一条义务是错误，不是"满足"**——一条静默丢掉"已验证"的
//! 任务会在什么都没跑的情况下读成做完了；而**判不了的义务永远不算完成**——"满足"与"判不了"的差别，就是
//! 闭合与假装之间的差别。

/// The obligations a maintenance task carries. Exactly these, exactly one each.
/// 一条维护任务携带的义务。恰好这些，各恰好一条。
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TaskObligationKind {
    /// The required change is on disk: the object exists and its bytes are the ones the task recorded.
    /// 要求的改动在磁盘上：对象存在，字节就是任务记录下的那些。
    Applied,
    /// A verification that covers those bytes passed, and its input closure still holds.
    /// 覆盖那些字节的验证通过了，而且它的输入闭包仍然成立。
    Verified,
    /// Every rule that applies is met at the revision the task recorded.
    /// 适用的每条规则都在任务记录的修订上成立。
    Rules,
    /// Every adoption lease the knowledge rests on is still in force at the recorded version.
    /// 知识所依托的每份采信租约仍按记录的版本生效。
    Knowledge,
    /// The acceptance the applicable policy requires, when it requires one.
    /// 适用政策要求的那次接受（当它要求时）。
    Acceptance,
}

impl TaskObligationKind {
    /// Every kind, in the order a reply prints them.
    /// 每一种，按回复打印的顺序。
    pub const ALL: [Self; 5] = [
        Self::Applied,
        Self::Verified,
        Self::Rules,
        Self::Knowledge,
        Self::Acceptance,
    ];

    /// One stable word for the kind, used in records and replies.
    /// 这个种类的一个稳定词，供记录与回复使用。
    pub fn label(&self) -> &'static str {
        match self {
            Self::Applied => "applied",
            Self::Verified => "verified",
            Self::Rules => "rules",
            Self::Knowledge => "knowledge",
            Self::Acceptance => "acceptance",
        }
    }
}

/// What one obligation's evidence says right now.
/// 一条义务的证据此刻说的是什么。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TaskObligationState {
    /// The evidence establishes the obligation.
    /// 证据确立了这条义务。
    Satisfied,
    /// The evidence shows the obligation is not met.
    /// 证据表明这条义务没有被满足。
    Outstanding,
    /// The evidence cannot decide it: missing, unreadable, changed underneath, or incomplete.
    /// 证据判不了它：缺失、读不出、底下已经变了，或不完整。
    Unknown,
}

impl TaskObligationState {
    /// One stable word for the state, used in records and replies.
    /// 状态的一个稳定词，供记录与回复使用。
    pub fn label(&self) -> &'static str {
        match self {
            Self::Satisfied => "satisfied",
            Self::Outstanding => "outstanding",
            Self::Unknown => "unknown",
        }
    }
}

/// One obligation and the sentence its evidence supports.
/// 一条义务，以及它的证据所支撑的那句话。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TaskObligation {
    kind: TaskObligationKind,
    state: TaskObligationState,
    detail: String,
}

impl TaskObligation {
    /// An obligation in the given state, with the sentence a reader gets.
    /// 处于给定状态的义务，并带上读者拿到的那句话。
    pub fn new(
        kind: TaskObligationKind,
        state: TaskObligationState,
        detail: impl Into<String>,
    ) -> Self {
        Self {
            kind,
            state,
            detail: detail.into(),
        }
    }

    /// An obligation the evidence establishes.
    /// 被证据确立的义务。
    pub fn satisfied(kind: TaskObligationKind, detail: impl Into<String>) -> Self {
        Self::new(kind, TaskObligationState::Satisfied, detail)
    }

    /// An obligation the evidence shows is not met.
    /// 证据表明没有被满足的义务。
    pub fn outstanding(kind: TaskObligationKind, detail: impl Into<String>) -> Self {
        Self::new(kind, TaskObligationState::Outstanding, detail)
    }

    /// An obligation the evidence cannot decide.
    /// 证据判不了的义务。
    pub fn unknown(kind: TaskObligationKind, detail: impl Into<String>) -> Self {
        Self::new(kind, TaskObligationState::Unknown, detail)
    }

    /// Which obligation this is.
    /// 这是哪一条义务。
    pub fn kind(&self) -> TaskObligationKind {
        self.kind
    }

    /// What the evidence says about it.
    /// 证据对它说了什么。
    pub fn state(&self) -> TaskObligationState {
        self.state
    }

    /// The sentence the evidence supports.
    /// 证据所支撑的那句话。
    pub fn detail(&self) -> &str {
        &self.detail
    }
}

/// What a task's obligation set adds up to.
/// 一组义务加起来是什么结论。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TaskVerdict {
    /// Every obligation is satisfied.
    /// 每条义务都被满足。
    Complete,
    /// At least one obligation is known not to be met.
    /// 至少有一条义务已知没有被满足。
    Incomplete,
    /// Nothing is known to be missing, but at least one obligation cannot be decided.
    /// 没有已知缺失，但至少有一条义务判不了。
    Unknown,
}

impl TaskVerdict {
    /// One stable word for the verdict, used in records and replies.
    /// 结论的一个稳定词，供记录与回复使用。
    pub fn label(&self) -> &'static str {
        match self {
            Self::Complete => "complete",
            Self::Incomplete => "incomplete",
            Self::Unknown => "unknown",
        }
    }
}

/// Exactly one obligation per kind, and the rule that reads them.
/// 每种义务恰好一条，以及读取它们的规则。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TaskCompletion {
    obligations: Vec<TaskObligation>,
}

impl TaskCompletion {
    /// Build the completion, refusing a set that leaves a kind out or names one twice.
    /// 构造完成状态；缺一种或重复一种的集合会被拒绝。
    ///
    /// The check is per kind and reports the count: "there is no `verified` obligation" is the
    /// failure this type exists to prevent, and a set that merely *happens* to have four entries is
    /// exactly how it happens.
    /// 检查按种类进行并报出条数："没有 `verified` 义务"正是本类型要防的失败，而一个**恰好**只有四条的目的
    /// 集合正是它发生的方式。
    pub fn new(obligations: Vec<TaskObligation>) -> Result<Self, String> {
        for kind in TaskObligationKind::ALL {
            let count = obligations
                .iter()
                .filter(|obligation| obligation.kind == kind)
                .count();
            if count != 1 {
                return Err(format!(
                    "a maintenance task carries exactly one `{}` obligation and this set has {count}; \
                     a missing obligation is not the same as a satisfied one",
                    kind.label()
                ));
            }
        }
        Ok(Self { obligations })
    }

    /// The obligations, in the fixed kind order rather than the order they were supplied in.
    /// 这些义务，按固定的种类顺序，而不是按传进来的顺序。
    pub fn obligations(&self) -> Vec<&TaskObligation> {
        let mut ordered = TaskObligationKind::ALL
            .iter()
            .filter_map(|kind| {
                self.obligations
                    .iter()
                    .find(|obligation| obligation.kind == *kind)
            })
            .collect::<Vec<_>>();
        ordered.sort_by_key(|obligation| obligation.kind);
        ordered
    }

    /// The verdict: complete only when every obligation is satisfied.
    /// 结论：只有每条义务都被满足时才是完成。
    ///
    /// The two non-complete answers are kept apart on purpose. An `Outstanding` obligation is a fact
    /// about the tree ("the rule is violated"), an `Unknown` one is a fact about the evidence ("the
    /// closure changed"), and a caller acting on them does different things: fix the tree, or obtain
    /// the evidence. Outstanding outranks Unknown because "something is known to be missing" is a
    /// stronger statement than "one thing cannot be judged".
    /// 两种"未完成"刻意分开：`Outstanding` 是关于树的事实（"规则被违反"），`Unknown` 是关于证据的事实
    /// （"闭包变了"），而据此行动的人要做不同的事：修树，或去取证据。`Outstanding` 排在 `Unknown` 前面，
    /// 因为"已知有东西缺着"比"有一件判不了"是更强的陈述。
    pub fn verdict(&self) -> TaskVerdict {
        if self
            .obligations
            .iter()
            .all(|obligation| obligation.state == TaskObligationState::Satisfied)
        {
            return TaskVerdict::Complete;
        }
        if self
            .obligations
            .iter()
            .any(|obligation| obligation.state == TaskObligationState::Outstanding)
        {
            return TaskVerdict::Incomplete;
        }
        TaskVerdict::Unknown
    }

    /// Whether this task may close.
    /// 这条任务是否可以闭合。
    pub fn is_complete(&self) -> bool {
        self.verdict() == TaskVerdict::Complete
    }

    /// The obligations that are not satisfied, in the fixed kind order.
    /// 未被满足的义务，按固定的种类顺序。
    pub fn open_obligations(&self) -> Vec<&TaskObligation> {
        self.obligations()
            .into_iter()
            .filter(|obligation| obligation.state != TaskObligationState::Satisfied)
            .collect()
    }

    /// One line per obligation, in the fixed kind order, satisfied ones included.
    /// 每条义务一行，按固定的种类顺序，满足的也打印。
    ///
    /// The satisfied lines are not noise: a reader deciding whether to trust a closure needs to see
    /// which five questions were asked, not just the one that failed.
    /// 满足的行不是噪声：判断这次闭合能不能信的读者需要看到问了哪五个问题，而不只是失败的那一个。
    pub fn summary(&self) -> String {
        let mut text = String::new();
        for obligation in self.obligations() {
            text.push_str(&format!(
                "  {:<12} {:<12} {}\n",
                obligation.kind.label(),
                obligation.state.label(),
                obligation.detail
            ));
        }
        text
    }
}

#[cfg(test)]
#[path = "task_tests.rs"]
mod task_tests;
