//! Trace recording macros.
//! 追踪记录宏。

/// Record one logical function invocation with its source location.
/// 用一条宏调用记录一次逻辑函数调用，并保留源码位置。
///
/// The closure is deliberately the only required wrapper. It keeps the
/// tracing boundary explicit, works on stable Rust, and does not introduce a
/// trait object or a global callback into the hot path.
/// 闭包是唯一必须写出的边界，兼容 stable Rust；不会引入 trait object，
/// 也不会在热路径上增加全局回调。
#[macro_export]
macro_rules! trace_call {
    ($trace:expr, $node:expr, $function:literal, $operation:expr) => {{ $trace.with($node, $function, $operation) }};
}

/// Record a fallible invocation according to the trace policy.
/// 按追踪策略记录一个可失败的函数调用。
///
/// In `errors-only` mode, successful scopes are discarded and `Err`/panic
/// scopes remain available for diagnostics. Other modes keep their normal
/// behavior.
/// 在 `errors-only` 模式下成功作用域会回滚，`Err` 或 panic 作用域会保留供诊断；
/// 其它模式保持正常收集行为。
#[macro_export]
macro_rules! trace_call_result {
    ($trace:expr, $operation:expr) => {{ $trace.with_result($operation) }};
}

/// Capture a function input or local value at the macro call site.
/// 在宏调用位置记录函数输入或局部值。
#[macro_export]
macro_rules! trace_value {
    ($trace:expr, $name:literal, $type_name:literal, $value:expr, $kind:expr) => {{ $trace.local($name, $type_name, $value, $kind) }};
}

/// Capture a transformed value and connect it to its producer.
/// 记录转换后的值，并把它连接到生产者。
#[macro_export]
macro_rules! trace_transform {
    ($trace:expr, $input:expr, $name:literal, $type_name:literal, $value:expr) => {{ $trace.transform($input, $name, $type_name, $value) }};
}

/// Capture a value consumed by another function parameter.
/// 记录值被另一个函数参数消费的位置。
#[macro_export]
macro_rules! trace_consume {
    ($trace:expr, $input:expr, $function:literal, $parameter:literal) => {{ $trace.consume($input, $function, $parameter) }};
}

/// Capture a function return value and connect it to its input.
/// 记录函数返回值，并把它连接到输入值。
#[macro_export]
macro_rules! trace_return {
    ($trace:expr, $input:expr, $name:literal, $type_name:literal, $value:expr) => {{ $trace.return_value($input, $name, $type_name, $value) }};
}
