//! Compatibility surface for the C++ `xirang.faces` reader.
//! C++ `xirang.faces` 读取器的兼容面。
//!
//! The reader is a C++ language implementation, so it lives in `xirang-multilang` with the rest of
//! the C++ surface (`cpp_syntax`, the SDK, the adapter). This module keeps the historical path
//! `build_method::external_face` working and re-exports the implementation rather than carrying a
//! second copy.
//! 读取器是 C++ 语言实现，因此与其余 C++ 面（`cpp_syntax`、SDK、适配器）一起住在 `xirang-multilang`。
//! 本模块保住历史路径 `build_method::external_face`，转发实现而不是保留第二份。

pub use xirang_multilang::external_faces::ExternalSource;

#[cfg(feature = "native-record")]
pub(crate) use xirang_multilang::languages::kit::record::{records, records_from_file};

/// The record decoder is compiled by `native-record` (implied by `lang-cpp` and `lang-zig`); a build
/// without it can still name external objects, but cannot read them. The error names the feature
/// rather than silently returning an empty slice.
/// 记录解码器由 `native-record` 编译（`lang-cpp` 与 `lang-zig` 都隐含它）；没有它的构建仍可命名外部目标
/// 文件，但读不出面。错误点名特性，而不是静默返回空切片。
#[cfg(not(feature = "native-record"))]
pub(crate) fn records(
    objects: &[ExternalSource<'_>],
    namespace: &str,
) -> Result<Vec<ExternalFaceRecord>, String> {
    let _ = (objects, namespace);
    Err(
        "this build was handed native objects to read, but it does not carry the record decoder; \
         enable `native-record` (or `lang-cpp` / `lang-zig`, which imply it) on xirang-toolchain"
            .to_owned(),
    )
}
#[cfg(not(feature = "native-record"))]
pub(crate) fn records_from_file(
    file: &std::path::Path,
    namespace: &str,
) -> Result<Vec<ExternalFaceRecord>, String> {
    let _ = (file, namespace);
    Err(
        "this package declares a build-time record file, but this build does not carry the record \
         decoder; enable `native-record` (or `lang-cpp` / `lang-zig` / `lang-c`, which imply it) on \
         xirang-toolchain"
            .to_owned(),
    )
}
pub(crate) use xirang_multilang::external_faces::ExternalFaceRecord;
