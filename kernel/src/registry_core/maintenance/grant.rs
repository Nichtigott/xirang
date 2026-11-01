//! What a person approved, as a value the writing surface can only compare against.
//! 某个人批准了什么，作为一个写入面只能拿去比对的取值。
//!
//! The maintainer's ruling (2026-10-17, `docs/managed-maintenance/authorization-design-2026-10-17.md`)
//! draws the line this module exists to make checkable: **the agent may apply for a scope and look at
//! the state; it may not issue the authorization**. Everything a caller can put in a request — an
//! adoption anchor's `verifier`, a `confirm` boolean, an environment variable, a file the caller wrote
//! — is a declaration or a locator, never a grant. A [`Grant`] is what a **controller** writes after a
//! person approved through a protected channel, and the kernel's only job with it is the pure question
//! *does this grant cover this use*, with the answer naming the dimension that failed.
//! 维护者的裁定（2026-10-17，`docs/managed-maintenance/authorization-design-2026-10-17.md`）划出的那条线，正是本模块
//! 存在的意义：**agent 可以申请范围、可以看状态，不可以签发授权**。调用方能写进请求的每一样东西——采信条目的
//! `verifier`、一个 `confirm` 布尔值、一个环境变量、一个调用方自己写的文件——都是声明或定位信息，永远不是授权。
//! [`Grant`] 是**控制器**在某人经受保护通道批准之后写下的东西，而内核在这里的唯一职责是那条纯问题——*这份授权是否
//! 覆盖这次使用*——并让答案点名失败的那个维度。
//!
//! The dimension list is **fixed**, for the reason [`super::EXECUTION_DIMENSIONS`] is: a grant whose
//! digest cannot distinguish "the proposal was never recorded" from "the proposal is the same" would let
//! an approval for one proposal vouch for another. A dimension that is missing is an error rather than a
//! quietly weaker credential.
//! 维度清单是**固定的**，理由与 [`super::EXECUTION_DIMENSIONS`] 相同：一份摘要分不出"提案从没被记录"与"提案相同"的
//! 授权，会让针对一份提案的批准替另一份背书。缺一个维度是错误，而不是一张悄悄变弱的凭据。

/// Every dimension a grant binds, in one place because "which fields are an authorization" is exactly
/// the contract that must not be restated at each call site.
/// 一份授权绑定的每一个维度，集中在一处——因为"哪些字段算授权"正是那份不许在每个调用点复述的契约。
///
/// A field name outside this list is refused rather than ignored: an ignored field is a dimension the
/// issuer believes is bound and the checker never reads.
/// 不在本清单上的字段名会被拒绝而不是被忽略：被忽略的字段，是签发方以为绑定了、而校验方从不读取的维度。
pub const GRANT_DIMENSIONS: &[&str] = &[
    "approver",
    "budget",
    "candidate",
    "channel",
    "operation",
    "policy",
    "project",
    "proposal",
    "revision",
    "scope",
    "task",
];

/// The dimensions [`Grant::covers`] actually **compares** when a use is checked.
/// [`Grant::covers`] 在检查一次使用时**真的比较**的那些维度。
///
/// The split matters for honesty, not for bookkeeping: `GRANT_DIMENSIONS` is what an issuer must bind, and `approver`
/// and `channel` are **provenance** — recorded so a reader can see who approved through which channel, and never
/// compared against a request. Everything else is compared, including `policy` and `budget`: a policy version that
/// only prints is a version nothing enforces, and a budget that only prints invites the reader to believe a limit
/// that does not exist. Measured 2026-10-18: `budget` and `policy` were printed as provenance and read by nothing;
/// they are compared now, and the request states both (the policy it runs under and what it is about to cost).
/// The two lists are pinned together (`grant_tests::the_compared_and_recorded_dimensions_are_disjoint_and_complete`) so
/// a new dimension cannot land in neither half.
/// 这个划分是为了诚实，不是记账：`GRANT_DIMENSIONS` 是签发方必须绑定的东西，而 `approver` 与 `channel` 是**来源信息**——
/// 记录它们，好让读者看到是谁、经哪个通道批的；它们从不与请求比较。其余全部比较，包括 `policy` 与 `budget`：只印出来的
/// 政策版本是没人执行的政策版本，只印出来的预算则在邀请读者相信一个不存在的限制。2026-10-18 实测：`budget` 与
/// `policy` 当时只作来源信息印出、没有任何东西读；现在它们被比较，而请求必须说出自己按哪份政策、即将花掉多少。
/// 两张清单被一条钉子钉在一起（`grant_tests::the_compared_and_recorded_dimensions_are_disjoint_and_complete`），
/// 因此新增一个维度不可能两边都不落。
pub const GRANT_COMPARED_DIMENSIONS: &[&str] = &[
    "budget",
    "candidate",
    "operation",
    "policy",
    "project",
    "proposal",
    "revision",
    "scope",
    "task",
];

/// The dimensions a grant **records** as provenance, and never compares against a request.
/// 授权作为**来源信息**记录、且从不与请求比较的那些维度。
pub const GRANT_RECORDED_DIMENSIONS: &[&str] = &["approver", "channel"];

/// The channels a grant may name. A protected channel is the whole point: "approved" with no channel is
/// a declaration wearing an authorization's name.
/// 一份授权可以点名的通道。受保护的通道正是要点：没有通道的"已批准"，是一件顶着授权名字的声明。
///
/// The three are the ones the ruling names — the host's confirmation surface, an interactive CLI, and
/// Studio — and they are spelled here so a forged value cannot invent a fourth.
/// 这三个是裁定点名的三个——宿主确认界面、交互式 CLI、Studio——写在这里，是为了让伪造的取值无法发明第四个。
pub const GRANT_CHANNELS: &[&str] = &["host-confirmation", "interactive-cli", "studio"];

/// What a grant allows an operation to cost, and what an operation says it will cost.
/// 一份授权允许一次操作花掉多少，以及一次操作声称自己要花掉多少。
///
/// Both numbers are about the **writing** side: `files` bounds how many files one operation may publish, and
/// `executions` bounds how many times the grant may be used for its operation. A limit that is not named is not
/// enforced — the refusal says which one was exceeded, so a caller learns what to ask a person for rather than
/// guessing.
/// 两个数都关于**写入**那一侧：`files` 限制一次操作可以发布多少个文件，`executions` 限制这份授权可被使用多少次。没有点名的
/// 限制就不执行——拒绝会点名超的是哪一条，于是调用方知道自己该找人批什么，而不是去猜。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Budget {
    /// The most files one operation may publish, when the grant names a limit.
    /// 一次操作最多可发布多少个文件（授权点名限制时）。
    pub files: Option<u64>,
    /// The most uses of the operation the grant allows, when the grant names a limit.
    /// 这份授权允许该操作被使用多少次（授权点名限制时）。
    pub executions: Option<u64>,
}

impl Budget {
    /// Read a budget written as `files=8,executions=1`.
    /// 读取写成 `files=8,executions=1` 的预算。
    ///
    /// Unknown keys and unreadable numbers are refused rather than skipped: a budget the checker cannot read is not a
    /// limit, and treating it as unlimited would make the safest-looking grant the loosest one.
    /// 未知键与读不出的数字被拒绝而不是跳过：校验方读不懂的预算不是限制，把它当作无限会让看起来最稳妥的授权变成最松的那份。
    pub fn parse_limits(text: &str) -> Result<Self, String> {
        let mut budget = Self::default();
        for part in text.split(',') {
            let part = part.trim();
            if part.is_empty() {
                continue;
            }
            let (key, value) = part.split_once('=').ok_or_else(|| {
                format!("`{part}` is not a budget limit; write `files=<n>` or `executions=<n>`")
            })?;
            let value = value.trim();
            let count: u64 = value.parse().map_err(|_| {
                format!("`{value}` is not a number of {key}; a budget names how many, not how much it hopes")
            })?;
            match key.trim() {
                "files" => budget.files = Some(count),
                "executions" => budget.executions = Some(count),
                other => {
                    return Err(format!(
                        "`{other}` is not a budget dimension; the limits this workspace enforces are files and executions"
                    ));
                }
            }
        }
        Ok(budget)
    }

    /// The budget as it is written in a grant.
    /// 预算在授权里的书写形式。
    pub fn text(&self) -> String {
        let mut parts = Vec::new();
        if let Some(files) = self.files {
            parts.push(format!("files={files}"));
        }
        if let Some(executions) = self.executions {
            parts.push(format!("executions={executions}"));
        }
        if parts.is_empty() {
            "none".to_owned()
        } else {
            parts.join(",")
        }
    }

    /// Whether this cost fits inside the granted budget, and if not, which limit it exceeds.
    /// 这个代价是否落在被批准的预算之内；不在时，超的是哪一条限制。
    pub fn covers(&self, cost: &Self) -> Result<(), String> {
        if let (Some(allowed), Some(used)) = (self.files, cost.files)
            && used > allowed
        {
            return Err(format!(
                "this landing would publish {used} file(s), and the approval covers {allowed}"
            ));
        }
        if let (Some(allowed), Some(used)) = (self.executions, cost.executions)
            && used > allowed
        {
            return Err(format!(
                "this use would take the {used} execution(s) it asks for, and the approval covers {allowed}"
            ));
        }
        Ok(())
    }
}

/// The operations a grant may authorize.
/// 一份授权可以批准的操作。
///
/// They are verbs the writing surfaces already have, so a grant cannot authorize an action no surface
/// implements, and a surface cannot perform one no grant names.
/// 它们是写入面本来就有的动词，因此授权不能批准一个没有执行面实现的动作，执行面也不能做一件没有授权点名的操作。
pub const GRANT_OPERATIONS: &[&str] = &["land", "publish", "widen"];

/// One authorization, as the controller issued it.
/// 一条授权，按控制器签发它的那个样子。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Grant {
    /// The bound dimensions, sorted by name.
    /// 绑定的维度，按名字排序。
    fields: Vec<(String, String)>,
}

/// One proposed use of a grant: the facts a writing surface has in hand at the moment it asks.
/// 一次对授权的拟议使用：写入面在开口询问的那一刻手里已有的事实。
///
/// It is deliberately a separate type from [`Grant`]: the caller **states a use**, never a grant. A
/// surface that could construct a `Grant` from request data would be back to self-issuance.
/// 它与 [`Grant`] 有意是两个类型：调用方**陈述一次使用**，永远不构造一份授权。一个能从请求数据构造出 `Grant` 的
/// 执行面，等于回到了自行签发。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UseRequest<'a> {
    /// The project identity the grant was issued for.
    /// 这份授权是为哪个项目身份签发的。
    pub project: &'a str,
    /// The task key.
    /// 任务键。
    pub task: &'a str,
    /// The candidate whose tree would be written.
    /// 将要被写入的那棵候选树。
    pub candidate: &'a str,
    /// The proposal revision the use relies on.
    /// 这次使用所依赖的提案版本。
    pub proposal: &'a str,
    /// The grant revision the caller believes is active.
    /// 调用方认为仍然有效的那个授权版本。
    pub revision: &'a str,
    /// The operation to perform.
    /// 要执行的操作。
    pub operation: &'a str,
    /// Every path the operation would write, relative to the managed root.
    /// 这次操作会写入的每一条路径，相对被管理根。
    pub paths: &'a [String],
    /// The policy version the writing surface is running under.
    /// 写入面正在执行的政策版本。
    pub policy: &'a str,
    /// What this use would cost.
    /// 这次使用会花掉多少。
    pub cost: Budget,
}

/// What the kernel decided about one proposed use.
/// 内核就一次拟议使用作出的判定。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Allowance {
    /// The grant covers this use.
    /// 这份授权覆盖了这次使用。
    Covered,
    /// The use is refused, and the sentence names the dimension that failed with both values, because a
    /// refusal that only says "no" makes the caller guess what to ask a person for.
    /// 这次使用被拒，而那句话点名失败的维度与两端的取值——只会说"不"的拒绝，会让调用方去猜该找人批什么。
    Refused(String),
}

impl Allowance {
    /// Whether the use is covered.
    /// 这次使用是否被覆盖。
    pub fn is_covered(&self) -> bool {
        matches!(self, Self::Covered)
    }

    /// The refusal's reason, when the use is not covered.
    /// 使用未被覆盖时的拒绝原因。
    pub fn reason(&self) -> Option<&str> {
        match self {
            Self::Covered => None,
            Self::Refused(reason) => Some(reason),
        }
    }
}

impl Grant {
    /// Bind an authorization to the given dimensions, refusing one that leaves a dimension out.
    /// 把一条授权绑定到给定的维度上；缺了某个维度的授权会被拒绝。
    ///
    /// Four refusals, and each one is a way a forged approval would otherwise pass. An unknown name is a
    /// dimension the issuer thinks is bound and the checker never reads. A blank value records that
    /// something was asked and nothing was learned. `unknown` as an **approver or channel** is not a
    /// person and not a channel, so it is refused by name. An `operation` or `channel` outside their
    /// lists is invented rather than approved.
    /// 四条拒绝，每一条都是伪造批准否则能通过的路径。未知名字是"签发方以为绑定了、校验方从不读取"的维度；空值记录的
    /// 是"问了但没学到东西"；把 `unknown` 当作**批准人或通道**既不是人也不是通道，因此按名字拒绝；`operation` 或
    /// `channel` 落在各自清单之外的是被发明出来的，不是被批准出来的。
    pub fn issue<I, K, V>(fields: I) -> Result<Self, String>
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<String>,
        V: Into<String>,
    {
        let mut bound: Vec<(String, String)> = Vec::new();
        for (name, value) in fields {
            let name = name.into();
            let value = value.into();
            if !GRANT_DIMENSIONS.contains(&name.as_str()) {
                return Err(format!(
                    "`{name}` is not a grant dimension; the list is fixed so a new one cannot be forgotten silently"
                ));
            }
            let value = value.trim().to_owned();
            if value.is_empty() {
                return Err(format!(
                    "grant dimension `{name}` has no value; a grant binds facts, not places where a fact would go"
                ));
            }
            if matches!(name.as_str(), "approver" | "channel") && value == "unknown" {
                return Err(format!(
                    "`{name}` cannot be `unknown`: an approval nobody and no channel stands behind is not an approval"
                ));
            }
            if name == "budget" {
                // A budget the checker cannot read is not a limit; refusing it here is what keeps the printed
                // "budget …" line from being a number nobody enforces.
                // 校验方读不懂的预算不是限制；在这里拒绝它，正是让打印出来的 "budget …" 行不再是没人执行的一个数字。
                Budget::parse_limits(&value)?;
            }
            if name == "channel" && !GRANT_CHANNELS.contains(&value.as_str()) {
                return Err(format!(
                    "`{value}` is not an approval channel; the channels are {} — a fourth one is invented, not approved",
                    GRANT_CHANNELS.join(", ")
                ));
            }
            // An operation value may be a **set**, sorted and comma-joined, because one approval can authorise "widen
            // and then land" — the ruling's widening flow says the approval is what lets the work continue, so a grant
            // that could only name one verb would force a second approval for the same decision. Every member still
            // has to be a verb this workspace implements: a set invents no new one.
            // 操作取值可以是一个**集合**（排序、逗号连接），因为一次批准可以授权"先扩大、再落地"——裁定的扩大流程说批准就是
            // 让工作继续的东西，因此一份只能点名一个动词的授权，会为同一个决定强要第二次批准。每个成员仍必须是本工作区实现了
            // 的动词：集合不发明新的。
            if name == "operation" {
                let members: Vec<&str> = value.split(',').map(str::trim).collect();
                if members.is_empty() || members.iter().any(|member| member.is_empty()) {
                    return Err(
                        "`operation` names an empty verb; a grant authorises operations, not the absence of one"
                            .to_owned(),
                    );
                }
                if let Some(unknown) = members
                    .iter()
                    .find(|member| !GRANT_OPERATIONS.contains(member))
                {
                    return Err(format!(
                        "`{unknown}` is not an operation a grant can authorize; the operations are {}",
                        GRANT_OPERATIONS.join(", ")
                    ));
                }
            }
            bound.push((name, value));
        }
        let missing: Vec<&str> = GRANT_DIMENSIONS
            .iter()
            .copied()
            .filter(|name| !bound.iter().any(|(bound_name, _)| bound_name == name))
            .collect();
        if !missing.is_empty() {
            return Err(format!(
                "a grant is missing {} — a grant that leaves a dimension out cannot be compared, so it is not issued",
                missing.join(", ")
            ));
        }
        bound.sort();
        Ok(Self { fields: bound })
    }

    /// The bound dimensions, sorted by name.
    /// 绑定的维度，按名字排序。
    pub fn fields(&self) -> &[(String, String)] {
        &self.fields
    }

    /// One dimension's value, or `None` when it is not bound.
    /// 某个维度的取值；未绑定时为 `None`。
    pub fn field(&self, name: &str) -> Option<&str> {
        self.fields
            .iter()
            .find(|(field, _)| field == name)
            .map(|(_, value)| value.as_str())
    }

    /// The grant's identity. It is what a controller records, and what a superseded revision stops
    /// matching.
    /// 这份授权的身份。它是控制器记录的东西，也是被取代的旧版本不再匹配的东西。
    pub fn digest(&self) -> String {
        xirang_kernel_sha256(&format!("{:?}", self.fields))
    }

    /// Whether this grant covers one proposed use, and if not, which dimension failed.
    /// 这份授权是否覆盖某一次拟议使用；不覆盖时，是哪个维度失败。
    ///
    /// The six comparisons are the ruling's "an old approval cannot be used for another candidate,
    /// another scope, or a proposal that has changed", spelled one dimension at a time. Scope is the only
    /// one that is not an equality: the use may name **fewer** paths than the grant does (the grant is for
    /// the scope, not for one edit inside it — "范围内的后续编辑不必反复询问用户"), and every path it
    /// does name has to be inside the granted prefixes with a separator boundary.
    /// 这六条比较就是裁定里那句"旧批准不能用于其他候选、其他范围或已变化的提案"，一个维度一个维度地写出来。范围是
    /// 唯一不是等式的那个：使用可以只点名**更少**的路径（授权是给范围的，不是给范围里某一次编辑的——"范围内的后续
    /// 编辑不必反复询问用户"），而它点到的每一条都必须落在被批准的前缀之内，并带分隔符边界。
    pub fn covers(&self, request: &UseRequest<'_>) -> Allowance {
        // `operation` is compared just below as a **set** rather than an equality, because one approval may
        // authorise a widening and the landing that follows it.
        // `operation` 在下面按**集合**比较而不是等式，因为一次批准可以同时授权一次扩大与随后的落地。
        let comparisons = [
            ("project", request.project),
            ("task", request.task),
            ("candidate", request.candidate),
            ("proposal", request.proposal),
            ("revision", request.revision),
        ];
        for (name, used) in comparisons {
            let granted = self.field(name).unwrap_or("");
            if name == "revision" {
                // The revision is the one dimension where **newer is fine**: the controller raises it when it
                // re-issues after a widening, and requiring equality would make every already-issued approval
                // stale the moment it moved. Older is still refused — that is the direction that would let a
                // pre-widening approval vouch for a widened scope.
                // 版本是唯一一个"更新也可以"的维度：控制器在扩大之后重新签发时会抬高它，而要求相等会让每一份已签发的
                // 批准在它移动的那一刻全部失效。更旧仍然拒绝——那是会让"扩大之前的批准为扩大之后的范围背书"的方向。
                if matches!(
                    (granted.parse::<u64>(), used.parse::<u64>()),
                    (Ok(granted), Ok(used)) if granted >= used
                ) {
                    continue;
                }
            }
            if granted != used {
                return Allowance::Refused(format!(
                    "this use relies on `{name}` = `{used}`, and the grant says `{granted}`; an approval for one {name} never vouches for another"
                ));
            }
        }
        // The policy version is an equality: a grant issued under another version of the policy was issued against
        // different rules, so it cannot vouch for a use that runs under this one.
        // 政策版本是等式：按另一版政策签发的授权，是对着不同的规则签发的，因此它不能为按这一版运行的使用背书。
        let policy = self.field("policy").unwrap_or("");
        if policy != request.policy {
            return Allowance::Refused(format!(
                "this use runs under policy `{}`, and the grant was issued under `{policy}`; an approval for one version of the policy never vouches for another",
                request.policy
            ));
        }
        // The budget is a **limit**, not a record: the request says what it is about to cost, and the grant says how
        // much it approved.
        // 预算是**限制**而不是记录：请求说出它即将花掉多少，授权说出它批了多少。
        let granted = match Budget::parse_limits(self.field("budget").unwrap_or("")) {
            Ok(budget) => budget,
            Err(reason) => return Allowance::Refused(reason),
        };
        if let Err(reason) = granted.covers(&request.cost) {
            return Allowance::Refused(reason);
        }
        // The operation is a set: one approval may authorise a widening **and** the landing that follows it.
        // 操作是一个集合：一次批准可以同时授权一次扩大**与**随后的落地。
        let operations: Vec<&str> = self
            .field("operation")
            .unwrap_or("")
            .split(',')
            .map(str::trim)
            .collect();
        if !operations.contains(&request.operation) {
            return Allowance::Refused(format!(
                "this use is an `{}`, and the grant authorises `{}`; an approval for one operation never vouches for another",
                request.operation,
                operations.join(", ")
            ));
        }
        let scope: Vec<&str> = self.field("scope").unwrap_or("").split(',').collect();
        for path in request.paths {
            if !scope.iter().any(|prefix| {
                let prefix = prefix.trim();
                !prefix.is_empty() && (path == prefix || path.starts_with(&format!("{prefix}/")))
            }) {
                return Allowance::Refused(format!(
                    "`{path}` is outside the approved scope ({}); the grant covers the scope it names and nothing else",
                    scope.join(", ")
                ));
            }
        }
        Allowance::Covered
    }
}

/// The kernel's one hash implementation, reached through the module that owns it rather than a second
/// copy: a grant whose digest used a different fold than the workspace's would be comparable with
/// nothing else here.
/// 内核唯一的那份散列实现，经拥有它的模块到达而不是另写一份：一份用别的折叠算摘要的授权，与这里任何东西都不可比。
fn xirang_kernel_sha256(input: &str) -> String {
    super::super::identity::sha256_hex(input.as_bytes())
}

#[cfg(test)]
#[path = "grant_tests.rs"]
mod grant_tests;
