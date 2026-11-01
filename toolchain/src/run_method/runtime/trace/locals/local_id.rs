//! The stable identity of one captured local value.
//! 一个已捕获局部值的稳定身份。

/// The trace-local identity of one captured value.
/// 一个已捕获局部值在其所属追踪内的身份。
///
/// Ids are minted once per `CallTrace` and are never reused; an id that escapes
/// a discarded scope therefore resolves to nothing rather than aliasing new
/// evidence. Only equality and ordering within one trace are meaningful.
/// id 在一条 `CallTrace` 内只发放一次、永不复用；逃出被丢弃作用域的 id 只会解析不到
/// 任何东西，而不会别名到新证据。只有同一条追踪内的相等与次序才有意义。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct LocalId(pub u64);
