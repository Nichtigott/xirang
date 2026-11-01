//! The role of one local value in a function invocation.
//! 一个函数调用中局部值的角色。

/// The role of a value in one function invocation.
/// 一个函数调用中局部值的角色。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LocalKind {
    /// A value passed into the invocation.
    /// 调用传入的值。
    Input,
    /// A value bound inside the invocation body.
    /// 在调用体内绑定的值。
    Binding,
    /// A value the invocation returns.
    /// 调用返回的值。
    Output,
    /// A value handed to a function as one of its parameters.
    /// 作为某个函数参数被消费的值。
    Consumer,
}

impl LocalKind {
    /// The lowercase token this role prints as in rendered data flow.
    /// 该角色在渲染数据流中打印的小写标记。
    pub const fn label(self) -> &'static str {
        match self {
            Self::Input => "input",
            Self::Binding => "let",
            Self::Output => "return",
            Self::Consumer => "consumer",
        }
    }
}
