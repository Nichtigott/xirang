//! JSON rendering shared by both `explain` output shapes.
//! `explain` 两种输出形状共用的 JSON 渲染。
//!
//! Split decision: the node report and the overlay projection each build their
//! own `serde_json::Value`, but both must serialize identically and both must
//! turn a write failure into the same command error. Keeping that one rule in
//! one page stops the two shapes from drifting apart.
//! 拆分决定：逐节点报告与覆盖投影各自构建 `serde_json::Value`，但两者的序列化必须
//! 一致，且都必须把写出失败变成同一种命令错误。把这条规则放进单独一页，两种形状就
//! 不会各自漂移。

use serde_json::Value;

/// Serialize one report, reporting a serializer failure instead of panicking.
/// 序列化一个报告；序列化失败时返回错误而不是 panic。
pub(crate) fn render_json(report: &Value) -> String {
    serde_json::to_string(report).unwrap_or_else(|error| format!("{{\"error\":\"{error}\"}}"))
}

/// Map a write failure onto the command's error type.
/// 把写出失败映射为命令的错误类型。
pub(crate) fn write_error(error: std::io::Error) -> String {
    format!("cannot write output: {error}")
}
