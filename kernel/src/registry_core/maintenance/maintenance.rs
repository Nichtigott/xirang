//! Minimal delivery-state vocabulary shared by verification surfaces.
//! 验证执行面共用的最小交付状态词汇。
//!
//! Pure nouns and one pure classification: no filesystem, no environment, no process lifetime.
//! The execution surface owns the process handle, the log read and the reply text; this module owns
//! only the typed result those facts are classified into.
//! 纯名词加一条纯分类：不碰文件系统、不读环境、不依赖进程生存期。执行面拥有进程句柄、日志读取与回复
//! 文本；本模块只拥有那些事实被分类成的类型化结果。

/// What kind of run produced a result.
/// 产出结果的是哪一种运行。
///
/// A compile-only run has no `test result:` line and no executed tests, so it must not be forced
/// through the test classifier's "nothing ran" rules; a test run that executed zero tests is not a
/// pass. The kind is carried explicitly instead of fabricating counts.
/// 仅编译的运行没有 `test result:` 行、也没有执行的测试，因此不能硬套测试分类器的"什么都没跑"规则；
/// 执行了零个测试的测试运行不是通过。种类被显式携带，而不是伪造计数。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VerificationKind {
    /// `cargo test`: a pass requires tests that actually executed.
    /// `cargo test`：通过需要确有测试执行。
    Tests,
    /// `cargo check` / `build` / `clippy`: exit status is the whole verdict.
    /// `cargo check` / `build` / `clippy`：退出码就是全部判定。
    Compilation,
}

/// Why an observed verification cannot be read as either a pass or a failure.
/// 为什么一次被观测的验证既不能读成通过、也不能读成失败。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnknownReason {
    /// The configured timeout fired and the run was killed.
    /// 配置的超时触发，运行被杀。
    Timeout,
    /// A caller or session cancelled the run and it was killed.
    /// 调用方或会话取消了运行，运行被杀。
    Cancelled,
    /// The process was signalled.
    /// 进程被信号终止。
    Signalled,
    /// The process exited zero but the log carried no `test result:` line.
    /// 进程以 0 退出，但日志里没有 `test result:` 行。
    NoResultLine,
    /// Result lines exist and the process exited zero, but zero tests executed.
    /// 存在结果行且进程以 0 退出，但实际执行了零个测试。
    ZeroExecuted,
}

/// The typed outcome of one verification run.
/// 一次验证运行的类型化结果。
///
/// `Failed` covers every nonzero exit, including compiler errors and test failures; the surface's
/// observation, not this noun, says which named test failed and on what assertion.
/// `Failed` 覆盖所有非零退出，含编译错误与测试失败；哪些点名的测试因什么断言失败，由执行面的观测而不是
/// 本名词说明。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VerificationOutcome {
    /// The run's success condition held.
    /// 运行的通过条件成立。
    Passed,
    /// The process exited nonzero.
    /// 进程以非零退出。
    Failed,
    /// No verdict is supported, with the reason.
    /// 证据不支持任何判定，并给出原因。
    Unknown(UnknownReason),
}

impl VerificationOutcome {
    /// Whether this outcome is a verified pass.
    /// 这个结果是否是一次被验证的通过。
    pub fn is_passed(&self) -> bool {
        matches!(self, Self::Passed)
    }

    /// The reason a verdict could not be given.
    /// 给不出判定时的原因。
    pub fn unknown_reason(&self) -> Option<UnknownReason> {
        match self {
            Self::Unknown(reason) => Some(*reason),
            Self::Passed | Self::Failed => None,
        }
    }

    /// One stable word for this outcome, for structured records.
    /// 这个结果的一个稳定词，供结构化记录使用。
    pub fn label(&self) -> &'static str {
        match self {
            Self::Passed => "passed",
            Self::Failed => "failed",
            Self::Unknown(_) => "unknown",
        }
    }
}

impl UnknownReason {
    /// One stable word for this reason, for structured records.
    /// 这个原因的一个稳定词，供结构化记录使用。
    pub fn label(&self) -> &'static str {
        match self {
            Self::Timeout => "timeout",
            Self::Cancelled => "cancelled",
            Self::Signalled => "signalled",
            Self::NoResultLine => "no_result_line",
            Self::ZeroExecuted => "zero_executed",
        }
    }
}

impl VerificationKind {
    /// One stable word for the kind, for structured records.
    /// 这个种类的一个稳定词，供结构化记录使用。
    pub fn label(&self) -> &'static str {
        match self {
            Self::Tests => "tests",
            Self::Compilation => "compilation",
        }
    }
}

/// Turn one finished run into the typed outcome, with the actual counts it observed.
/// 把一次结束的运行变成类型化结果，并带上它实际观测到的计数。
///
/// Timeout and cancellation are separate reasons on purpose: a timeout says the work outran its
/// budget, a cancellation says a person or session stopped it. A compilation run is judged by its
/// exit status alone; a test run is judged by its result lines and executed tests.
/// 超时与取消有意分开：超时说工作超出预算，取消说人或会话中止了它。编译运行只看退出码；测试运行看结果行与
/// 实际执行的测试。
pub fn classify(
    kind: VerificationKind,
    timed_out: bool,
    cancelled: bool,
    code: Option<i32>,
    results: usize,
    executed: usize,
) -> VerificationOutcome {
    if timed_out {
        return VerificationOutcome::Unknown(UnknownReason::Timeout);
    }
    if cancelled {
        return VerificationOutcome::Unknown(UnknownReason::Cancelled);
    }
    match code {
        None => VerificationOutcome::Unknown(UnknownReason::Signalled),
        Some(0) if kind == VerificationKind::Tests && results == 0 => {
            VerificationOutcome::Unknown(UnknownReason::NoResultLine)
        }
        Some(0) if kind == VerificationKind::Tests && executed == 0 => {
            VerificationOutcome::Unknown(UnknownReason::ZeroExecuted)
        }
        Some(0) => VerificationOutcome::Passed,
        Some(_) => VerificationOutcome::Failed,
    }
}

/// What one delivery step has actually established, with the four claims kept distinct.
/// 一步交付实际确立了哪件事，四种声明各自分开。
///
/// A query being answered is not a mutation, a mutation is not verification, and neither is a
/// person's acceptance. [`Completion::verification_closes`] is the rule for a verification
/// obligation. Whether **acceptance** is required is the caller's applicable policy, passed to
/// [`Completion::is_complete`] — this noun imposes no blanket human-approval gate, and it carries no
/// task/input binding (that is the delivery plan's, scheduled for W4).
/// 查询作答不是改动，改动不是验证，两者也都不是人的接受。[`Completion::verification_closes`] 是验证
/// 义务的规则。**接受**是否为必需由调用方适用的政策决定，作为参数传给 [`Completion::is_complete`]
/// ——本名词不强加一揽子人工批准门，也不携带任务/输入绑定（那属于交付计划，排在 W4）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Completion {
    /// A query or read answered; it says nothing about the tree.
    /// 一次查询或读取作了答；它对树本身什么都没说。
    pub answered: bool,
    /// A mutation landed on disk.
    /// 一次改动落盘。
    pub applied: bool,
    /// The verification this step ran, when one ran.
    /// 这一步运行的验证（若有）。
    pub verified: Option<VerificationOutcome>,
    /// A person accepted the result.
    /// 有人接受了结果。
    pub accepted: bool,
}

impl Completion {
    /// A step that only answered a query.
    /// 只回答了一次查询的一步。
    pub fn answered() -> Self {
        Self {
            answered: true,
            applied: false,
            verified: None,
            accepted: false,
        }
    }

    /// A step whose mutation landed.
    /// 改动已落盘的一步。
    pub fn applied() -> Self {
        Self {
            answered: true,
            applied: true,
            verified: None,
            accepted: false,
        }
    }

    /// A step that ran a verification.
    /// 运行过一次验证的一步。
    pub fn verified(outcome: VerificationOutcome) -> Self {
        Self {
            answered: true,
            applied: false,
            verified: Some(outcome),
            accepted: false,
        }
    }

    /// A step a person accepted.
    /// 被一个人接受的一步。
    pub fn accepted() -> Self {
        Self {
            answered: true,
            applied: false,
            verified: None,
            accepted: true,
        }
    }

    /// Whether a **verification** obligation is closed by this step's evidence.
    /// 这一步的证据是否结清了一条**验证**义务。
    pub fn verification_closes(&self) -> bool {
        self.verified.is_some_and(|outcome| outcome.is_passed())
    }

    /// Whether the delivery claim holds under the caller's applicable policy.
    /// 在调用方适用的政策下，交付声明是否成立。
    ///
    /// `acceptance_required` is that policy and nothing else decides it: adoption and new semantic
    /// contracts keep their existing explicit confirmation, while an ordinary verified maintenance
    /// step does not acquire a new approval gate. The operation must still have been applied and the
    /// verification must still pass, so a historical pass cannot settle an unperformed operation.
    /// `acceptance_required` 就是那条政策，别的都不决定它：采信与新语义合同保留既有的显式确认，而一次
    /// 普通的已验证维护步骤不会因此多出一道批准门。操作仍必须已落盘、验证仍必须通过，因此历史通过不能结清
    /// 一个从未执行的操作。
    pub fn is_complete(&self, acceptance_required: bool) -> bool {
        self.applied && self.verification_closes() && (!acceptance_required || self.accepted)
    }
}

/// The dimensions a verification run's evidence is bound to.
/// 一次验证运行的证据所绑定的维度。
///
/// A field name that is not in this list is not a dimension; a dimension that is missing from an
/// identity is a **loud** error rather than a silently weaker credential. That asymmetry is the
/// whole point: "profile was never recorded" and "profile was the same" are indistinguishable from
/// a digest alone, and it is exactly the second reading that would reuse evidence across two
/// different builds (BR05).
/// 不在这份清单上的字段名不是维度；身份里缺一个维度是**响亮**的错误，而不是一张悄悄变弱的凭据。这个
/// 不对称正是要点：只看摘要，"profile 从没被记录"与"profile 相同"无法区分，而**后者**正是会让证据在两次
/// 不同构建之间被复用的读法（BR05）。
pub const EXECUTION_DIMENSIONS: &[&str] = &[
    "candidate",
    "toolchain",
    "target",
    "profile",
    "features",
    "no_default_features",
    "selection",
    "mode",
    "checker",
    "isolation",
];

/// Everything one run's evidence is bound to, as named fields.
/// 一次运行的证据所绑定的全部内容，按具名字段。
///
/// Two obligations may share one execution **only** when their identities are equal in every
/// dimension. Nothing here decides that two *different* configurations are equivalent: proving
/// `default`, `all-features` and `no-default-features` agree needs the fully resolved package,
/// feature, target and build-environment sets, and a rule that guessed it from "this package
/// declares no features" would be the false green the specification names outright.
/// 两条义务**只有**在每个维度都相等时才能共用一次执行。这里没有任何东西判定两种**不同**配置等价：证明
/// `default`、`all-features` 与 `no-default-features` 一致，需要完整解析过的包、特性、目标与构建环境集合，
/// 而一条从"这个包没声明特性"猜出来的规则，正是规格点名的那种假绿。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExecutionIdentity {
    /// The bound dimensions, sorted by name.
    /// 绑定的维度，按名字排序。
    fields: Vec<(String, String)>,
}

impl ExecutionIdentity {
    /// Bind one run to the given dimensions, refusing an identity that leaves one out.
    /// 把一次运行绑定到给定的维度上；缺了某个维度的身份会被拒绝。
    ///
    /// An empty or whitespace-only value is refused the same way a missing name is: a dimension whose
    /// value is `""` records that something was asked and nothing was learned, and letting it compare
    /// equal to another empty value would merge two runs on a dimension neither of them established.
    /// 空值或纯空白值与缺名一样被拒绝：取值是 `""` 的维度记录的是"问了但没学到东西"，而让它与另一个空值
    /// 比较相等，等于在一个两边都没确立的维度上合并两次运行。
    pub fn of<I, K, V>(fields: I) -> Result<Self, String>
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<String>,
        V: Into<String>,
    {
        let mut bound: Vec<(String, String)> = Vec::new();
        for (name, value) in fields {
            let name = name.into();
            let value = value.into();
            if !EXECUTION_DIMENSIONS.contains(&name.as_str()) {
                return Err(format!(
                    "`{name}` is not an execution dimension; the list is fixed so a new one cannot be forgotten silently"
                ));
            }
            if value.trim().is_empty() {
                return Err(format!(
                    "execution dimension `{name}` has no value; record `unknown` explicitly instead of binding nothing"
                ));
            }
            bound.push((name, value));
        }
        bound.sort();
        bound.dedup();
        // Membership is checked by name rather than by position: the list above is grouped by what
        // each dimension means, and sorting the bound fields for a stable digest would make a
        // positional check compare a name against whichever dimension happened to sort next.
        // 成员资格按名字而不是按位置检查：上面的清单是按每个维度的含义分组的，而为稳定摘要把绑定字段排序之后，
        // 按位置检查就会拿一个名字去和恰好排在后面的那个维度比。
        // Missing first, duplicates second: a duplicate displaces some other dimension, and the
        // error that names *which* one is missing is the one a caller can act on.
        // 先查缺失、再查重复：重复会挤掉另一个维度，而点出**缺的是哪一个**的错误才是调用方能据以行动的那个。
        for dimension in EXECUTION_DIMENSIONS {
            if !bound.iter().any(|(name, _)| name == dimension) {
                return Err(format!(
                    "execution identity is missing the `{dimension}` dimension; binding fewer dimensions than the list would reuse evidence across a difference it never recorded"
                ));
            }
        }
        if bound.len() != EXECUTION_DIMENSIONS.len() {
            return Err(
                "execution identity binds the same dimension twice; a duplicated dimension would hide the one it displaced"
                    .to_owned(),
            );
        }
        Ok(Self { fields: bound })
    }

    /// The bound dimensions, sorted by name.
    /// 绑定的维度，按名字排序。
    pub fn fields(&self) -> &[(String, String)] {
        &self.fields
    }

    /// The identity's digest: what a ledger records and what two obligations compare.
    /// 身份的摘要：台账记录它，两条义务也比较它。
    pub fn digest(&self) -> String {
        crate::sha256_hex(self.render_summary().as_bytes())
    }

    /// The dimensions as one readable line, so a reader can see *which* difference separated two runs.
    /// 把维度渲染成一行可读文本，使读者看得出是**哪一个**差异分开了两次运行。
    pub fn render_summary(&self) -> String {
        self.fields
            .iter()
            .map(|(name, value)| format!("{name}={value}"))
            .collect::<Vec<_>>()
            .join("; ")
    }

    /// Whether one execution's evidence may be referenced by another obligation's duty.
    /// 一次执行的证据是否可以由另一条义务引用。
    pub fn is_the_same_run_as(&self, other: &Self) -> bool {
        self.fields == other.fields
    }
}

/// What one obligation's run actually established, as the reuse decision sees it.
/// 一条义务的运行实际确立了什么，按复用判定所看到的样子。
///
/// This is the `VerificationReceipt` of the execution specification, reduced to the fields the
/// decision reads. The reduction is the point: a receipt carrying every diagnostic would tempt a
/// reader into re-deriving the verdict from prose, and the conditions below are decidable from
/// exactly these fields.
/// 这就是执行规格里的 `VerificationReceipt`，收敛到判定实际读取的那些字段。收敛正是要点：一份带着全部诊断的
/// 凭据会诱使读者从散文里重新推导判定，而下面那些条件恰好只由这些字段决定。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Receipt {
    /// The obligation this run settled.
    /// 这次运行结清的那条义务。
    pub obligation: String,
    /// The execution identity, in every dimension.
    /// 执行身份，逐维度。
    pub execution: ExecutionIdentity,
    /// The digest of the exact input closure the run was made against.
    /// 运行所依据的确切输入闭包的摘要。
    pub input_manifest: String,
    /// The accepted oracle and policy this verdict was judged under.
    /// 这次判定所依据的、已接受的 oracle 与政策。
    pub oracle_and_policy: String,
    /// How many tests the run executed.
    /// 运行执行了多少个测试。
    pub executed: usize,
    /// The typed outcome.
    /// 类型化结果。
    pub outcome: VerificationOutcome,
    /// Whether the run's coverage was confirmed rather than assumed.
    /// 这次运行的覆盖是被确认的，还是被假定的。
    pub coverage_confirmed: bool,
    /// Whether a flaky failure or an undisposed anomaly is attached to this obligation.
    /// 这条义务上是否挂着一次 flaky 失败或未处置的异常。
    pub anomaly_undisposed: Witness,
    /// Whether the scope this obligation must now cover has grown since it ran.
    /// 这条义务现在必须覆盖的范围，是否在它运行之后扩大了。
    pub scope_expanded: Witness,
    /// Whether the published form gained semantics this run did not verify.
    /// 发布形态是否新增了这次运行未验证的语义。
    pub published_form_changed: Witness,
}

/// What one reuse dimension was observed to be — and the difference between "no" and "nobody looked".
/// 一个复用维度被观测到什么——以及"不是"与"没人看过"的区别。
///
/// The three dimensions below used to be `bool` filled with `false` at every construction site, which read as "no
/// anomaly", "the scope did not grow" and "the published form did not change" while nobody had observed any of them.
/// The ruling calls that出 explicitly (`authorization-design-2026-10-17.md` §1.6): a missing observation is `unknown`,
/// and reuse that depends on it is refused — but the unknown is not turned into a failure, and caching is not
/// disabled wholesale: a run that **does** record the dimension can still be reused.
/// 下面三个维度过去是每个构造点都填 `false` 的 `bool`，读起来是"没有异常""范围没有扩大""发布形态没有变"，
/// 而没有人观测过其中任何一个。裁定点名了这件事（`authorization-design-2026-10-17.md` §1.6）：缺观测就是
/// `unknown`，依赖它的复用被拒——但未知不被改成失败，缓存也不被整体关掉：**确实记录了**该维度的运行仍然可复用。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Witness {
    /// Somebody observed this dimension for this run, and this is what they saw.
    /// 有人为这次运行观测过这个维度，这就是所见的。
    Known(bool),
    /// Nobody observed it. Not the same as `Known(false)`: "no anomaly was observed" and "no anomaly was looked for"
    /// are different facts, and only the first supports reuse. The string says what would make it observable.
    /// 没人观测过它。与 `Known(false)` 不同："没有观测到异常"与"没有去找异常"是两个事实，而只有前者支持复用。
    /// 那句字符串说明**怎样才能让它可观测**。
    Unknown(&'static str),
}

impl Witness {
    /// The dimension as a refusal, when it cannot support reuse.
    /// 该维度不能支持复用时，作为一次拒绝。
    ///
    /// `true` on either side refuses (the fact it records happened), and `Unknown` refuses too but with a
    /// different sentence: one says "this is why", the other says "nobody looked, and here is how to make them look".
    /// 任意一侧为 `true` 都拒绝（它记录的事实发生了），`Unknown` 也拒绝，但句子不同：一句说"原因在此"，另一句说
    /// "没人看过，而这是让他们去看的办法"。
    fn refuses(&self, other: &Self) -> Option<String> {
        for witness in [self, other] {
            match witness {
                Witness::Known(true) => {
                    return Some("the run recorded this dimension as changed".to_owned());
                }
                Witness::Unknown(why) => {
                    return Some(format!(
                        "nobody observed this dimension for the run ({why}), so reusing it would assume what was never looked for"
                    ));
                }
                Witness::Known(false) => {}
            }
        }
        None
    }
}

/// Whether a previous receipt may be referenced instead of running the obligation again.
/// 上一份凭据是否可以代替重跑这条义务被引用。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Reuse {
    /// The receipt stands for this obligation under the current facts.
    /// 在当前事实下，这份凭据代表这条义务。
    Allowed,
    /// Reuse is refused, with the condition that refused it and — for a dimension nobody observed — what would make
    /// it observable. It owns its sentence because the witness cases compose one from the dimension's `Unknown`
    /// reason, and a `&'static str` would have forced those reasons to be invented at every call site instead.
    /// 拒绝复用，并给出拒绝它的那条条件；维度没人观测时，还给出**怎样才能让它可观测**。它拥有自己的句子，因为见证分支要
    /// 用维度自带的 `Unknown` 原因拼出来，而 `&'static str` 会逼着在每个调用点现编那些原因。
    Refused(String),
}

impl Receipt {
    /// The five conditions of the specification, each decidable from the receipt and the current facts.
    /// 规格里的五条条件，每条都只由凭据与当前事实决定。
    ///
    /// **The order is the order a reader would check them in**, and every refusal names a condition
    /// rather than a symptom: "the run executed no tests" tells the caller to run it, while "not
    /// reusable" tells them nothing they can act on. A `passed` verdict is necessary and not
    /// sufficient — an obligation can have really run and really passed and still not be reusable.
    /// **顺序就是读者会按着查的顺序**，而每条拒绝都点名一条条件而不是症状："这次运行执行了零个测试"告诉调用方
    /// 去跑它，而"不可复用"什么可行动的都没说。`passed` 是必要条件而不是充分条件——一条义务可以真的跑过、真的
    /// 通过，而仍然不可复用。
    pub fn reuse_against(&self, current: &Self) -> Reuse {
        if self.obligation != current.obligation {
            return Reuse::Refused(
                "a different obligation's receipt cannot settle this one".to_owned(),
            );
        }
        if self.execution != current.execution {
            return Reuse::Refused(
                "the execution identity differs: checker, toolchain, target, profile, features, mode, selection or isolation changed".to_owned(),
            );
        }
        if self.input_manifest != current.input_manifest {
            return Reuse::Refused(
                "the input closure is not the one the run was made against".to_owned(),
            );
        }
        if self.oracle_and_policy != current.oracle_and_policy {
            return Reuse::Refused("the accepted oracle or policy changed".to_owned());
        }
        if self.executed == 0 {
            return Reuse::Refused(
                "the run executed no tests, so it establishes no behaviour".to_owned(),
            );
        }
        if !matches!(self.outcome, VerificationOutcome::Passed) {
            return Reuse::Refused("only a passed run is a receipt for this obligation".to_owned());
        }
        if !self.coverage_confirmed {
            return Reuse::Refused(
                "the run's coverage was assumed rather than confirmed".to_owned(),
            );
        }
        for (name, stored, now) in [
            (
                "anomaly",
                self.anomaly_undisposed,
                current.anomaly_undisposed,
            ),
            ("scope", self.scope_expanded, current.scope_expanded),
            (
                "published form",
                self.published_form_changed,
                current.published_form_changed,
            ),
        ] {
            if let Some(why) = stored.refuses(&now) {
                return Reuse::Refused(match (name, why.as_str()) {
                    ("anomaly", why) => format!(
                        "the anomaly dimension of this obligation is `{why}` — a flaky failure or an undisposed anomaly is attached to it"
                    ),
                    _ => format!("{name} is not reusable: {why}"),
                });
            }
        }
        Reuse::Allowed
    }
}

#[path = "rules.rs"]
mod rules;
pub use rules::*;

#[path = "decision.rs"]
mod decision;
pub use decision::*;

#[path = "grant.rs"]
mod grant;
pub use grant::*;

#[path = "task.rs"]
mod task;
pub use task::*;

#[cfg(test)]
#[path = "maintenance_tests.rs"]
mod maintenance_tests;
