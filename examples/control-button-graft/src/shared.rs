//! The drawing contract shared by the host and its out-of-project replacement.
//! 宿主与项目外替换件共享的绘制契约。
//!
//! It lives on the replacement crate because the host already depends on that crate; the host
//! re-exports these names at their original public paths.
//! 它住在替换件 crate 里，因为宿主已经依赖那个 crate；宿主在原公开路径上重导出这些名字。

/// The frame a parent face hands to its children for painting.
/// 父注册面交给子对象的绘制结果。
pub struct ControlFrame;

/// The interface every direct child must implement.
/// 每个直接子对象必须实现的接口。
pub trait ControlHandle {
    /// Produce one frame.
    /// 产生一帧。
    fn paint(&self) -> ControlFrame;
}
