//! Build-time syntax parsing. The parsers live in the kernel `syntax`
//! feature; this module keeps the historical `crate::build_method::syntax` import paths.
//! 构建期语法解析。解析器本体在 kernel 的 `syntax` feature 中；
//! 本模块保留 `crate::build_method::syntax` 历史导入路径。

pub use xirang_kernel::registry_core::syntax::{
    FaceSyntax, GraftSyntax, ParentSyntax, graft_entries, source_references,
};

pub(crate) use crate::source_cache::{application_entries, parse_face};
