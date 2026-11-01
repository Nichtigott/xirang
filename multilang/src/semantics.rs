//! Language-neutral semantic facts consumed by the execution surfaces.
//! 执行面消费的语言无关语义事实。
//!
//! A language provider proves what it can and leaves the unknown in `unresolved`; the query layer
//! must not turn a missing proof into a guessed edge.
//! 语言提供者证明它能证明的，把未知留在 `unresolved`；查询层不得把缺失的证明变成猜出来的边。

/// One function or method a semantic provider can name.
/// 语义提供者能命名的一个函数或方法。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SemanticFunction {
    /// Bare name (`new`).
    /// 裸名（`new`）。
    pub name: String,
    /// Owning type/trait/class, when the language names one.
    /// 语言点名时的方法所属类型/trait/类。
    pub owner: Option<String>,
    /// `Owner::name` for methods, bare name for free functions.
    /// 方法的 `Owner::name`，自由函数的裸名。
    pub qualified: String,
    /// One-based declaration line.
    /// 从 1 开始的声明行。
    pub line: usize,
    /// One-based closing line.
    /// 从 1 开始的结束行。
    pub end_line: usize,
    /// Definite callees, qualified where the provider can prove the owner.
    /// 确定的被调用者；提供者能证明所属者时已限定。
    pub calls: Vec<String>,
    /// Calls whose target the provider cannot prove, with a short reason.
    /// 提供者无法证明目标的调用，附带简短原因。
    pub unresolved: Vec<String>,
    /// Facts that qualify a call which **is** definite: a decorator that may replace the binding, a
    /// wrapper the caller cannot see. A caveat is not an unknown — putting it in `unresolved` would
    /// report a definite edge as unproven — and it is not silence either: a reader acting on the
    /// edge needs to know what stands behind it.
    /// 限定一条**确实**确定的调用的事实：可能替换绑定的装饰器、调用者看不见的包装。caveat 不是未知——
    /// 放进 `unresolved` 会把一条确定边说成未证明——也不是沉默：依据这条边行动的读者需要知道它背后站着什么。
    pub caveats: Vec<String>,
}
