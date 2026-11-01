//! Whether a local value came from a live trace or a static inference.
//! 局部值来自实时追踪还是静态推断。

/// Whether a local value came from a live trace or a static inference.
/// 局部值来自实时追踪还是静态推断。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Observation {
    /// The value was captured while the traced code ran.
    /// 被追踪代码运行时捕获到的值。
    Observed,
    /// The value is a static inference and was never seen at runtime.
    /// 由静态推断得出、运行时从未观测到的值。
    Unobserved,
}

impl Observation {
    /// The lowercase token this state prints as in rendered output.
    /// 该状态在渲染输出中打印的小写标记。
    pub const fn label(self) -> &'static str {
        match self {
            Self::Observed => "observed",
            Self::Unobserved => "unobserved",
        }
    }
}
