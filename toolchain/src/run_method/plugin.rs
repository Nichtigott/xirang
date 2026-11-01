//! Historical-path shim: `crate::run_method::plugin`.
//! 历史路径 shim：`crate::run_method::plugin`。
//!
//! Plugin protocol. The implementation lives in the kernel `plugin` module;
//! this shim keeps the historical path.
//! 插件协议。实现本体在 kernel 的 `plugin` 模块；本 shim 保留历史路径。

pub use xirang_kernel::plugin;
pub use xirang_kernel::plugin::*;
