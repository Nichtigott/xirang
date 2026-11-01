//! Reload failures recorded by the session.
//! 会话记录的重载失败。

/// Structured error retained after a failed hot reload.
/// 热重载失败后保留的结构化错误。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReloadError {
    /// Reload stage that failed, such as `"reload"` or `"hot_reload"`.
    /// 失败的重载阶段，例如 `"reload"` 或 `"hot_reload"`。
    pub phase: &'static str,
    /// Human-readable failure text from the loader.
    /// 加载器给出的可读失败信息。
    pub message: String,
}
