//! Build-phase diagnostic model. The types live in the kernel; this module
//! keeps the historical `crate::build_method::diagnostics` import paths.
//! 构建期诊断模型。类型本体在 kernel；本模块保留 `crate::build_method::diagnostics` 历史导入路径。

pub(crate) use xirang_kernel::{BuildDiagnostic, BuildDiagnostics};

/// Extend one diagnostics set with another.
/// 把一组诊断并入另一组。
pub(super) fn append_error(target: &mut BuildDiagnostics, addition: BuildDiagnostics) {
    target.extend(addition);
}
