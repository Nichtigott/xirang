//! The shim re-export ratchet: the historical paths a host still writes.
//! shim 重导出棘轮：宿主仍在书写的那些历史路径。
//!
//! Split out of `mounting` when that file reached the size ceiling; the rule is a
//! sibling of module mounting (both are about which paths the public surface
//! keeps), not a part of it.
//! 在 `mounting` 触到尺寸上限时拆出；这条规则与模块挂载是姊妹关系（两者都关乎公开面保留哪些
//! 路径），而不是它的一部分。

use std::path::Path;

/// Public re-exports the execution surfaces promise to keep.
/// 执行面承诺保留的公开重导出。
///
/// `AGENTS.md` change rule 2 keeps historical paths alive through shim
/// re-exports. The compiler checks half of it — a shim that names something core
/// no longer exports fails the build — and cannot check the other half, because
/// *deleting* a shim compiles perfectly while quietly removing the path a
/// downstream host still writes. This list is that half, pinned as a ratchet: a
/// listed re-export must still be there, and the list may grow but not shrink by
/// accident. Statements are stored whitespace-normalised; see
/// [`xirang_reexports`].
/// `AGENTS.md` 改动规则 2 用 shim 重导出保住历史路径。编译器只检查了其中一半——指名内核不再导出的
/// 东西的 shim 会让构建失败——另一半它检查不了：**删掉**一个 shim 完全能编译，却悄悄移除了下游
/// 宿主仍在书写的路径。这份清单就是那一半，以棘轮形式钉住：清单上的重导出必须还在，清单可以增长，
/// 但不会因疏忽而缩短。条目按空白规范化后存放，见 [`xirang_reexports`]。
pub const SHIMS: &[(&str, &str)] = &[
    (
        "toolchain/src/run_method/lib.rs",
        "pub use xirang_kernel::registry_core;",
    ),
    (
        "toolchain/src/run_method/lib.rs",
        "pub use xirang_kernel::registry_core::*;",
    ),
    (
        "toolchain/src/run_method/registry.rs",
        "pub use xirang_kernel::tree;",
    ),
    (
        "toolchain/src/run_method/registry.rs",
        "pub use xirang_kernel::tree::*;",
    ),
    (
        "toolchain/src/run_method/authoring/face_file.rs",
        "pub use xirang_kernel::authoring::{FACE_FIELD_COUNT, face_field};",
    ),
    (
        "toolchain/src/run_method/authoring/parse/parse.rs",
        "pub use xirang_kernel::authoring::parse::*;",
    ),
    (
        "toolchain/src/run_method/authoring/context.rs",
        "pub use xirang_kernel::authoring::validation::*;",
    ),
    (
        "toolchain/src/run_method/runtime/runtime.rs",
        "pub use xirang_kernel::{ COORDINATES_IN_VIEWPORT, Coordinates, FINITE_NUMBER, NON_EMPTY_TEXT, \
         Provenance, ProvenanceStep, RuntimeCheckFailure, RuntimeCheckSpec, RuntimeValue, };",
    ),
    (
        "toolchain/src/run_method/runtime/trace/trace.rs",
        "pub use xirang_kernel::CallSite;",
    ),
    (
        "toolchain/src/run_method/runtime/trace/trace.rs",
        "pub use xirang_kernel::declaration::source_file_matches;",
    ),
    (
        "toolchain/src/run_method/runtime/trace/trace.rs",
        "pub use xirang_kernel::TraceMode;",
    ),
    (
        "toolchain/src/run_method/runtime/evidence.rs",
        "pub use xirang_kernel::{CallEdge, EvidenceKind, LogicalCallEdge};",
    ),
    (
        "toolchain/src/run_method/plugin.rs",
        "pub use xirang_kernel::plugin;",
    ),
    (
        "toolchain/src/run_method/plugin.rs",
        "pub use xirang_kernel::plugin::*;",
    ),
    // Keep kernel type aliases; the private parser adapter now owns execution-side reuse.
    // 保留内核类型别名；私有解析适配器现持有执行层复用。
    (
        "toolchain/src/build_method/syntax.rs",
        "pub use xirang_kernel::registry_core::syntax::{ FaceSyntax, GraftSyntax, ParentSyntax, \
         graft_entries, source_references, };",
    ),
    (
        "toolchain/src/build_method/identity.rs",
        "pub use xirang_kernel::registry_core::identity::NodeId;",
    ),
    (
        "toolchain/src/build_method/identity.rs",
        "pub use xirang_kernel::registry_core::identity::IDENTITY_SCHEMA;",
    ),
];

/// Every `pub use xirang_kernel::…;` in `text`, whitespace-normalised.
/// `text` 中每个 `pub use xirang_kernel::…;`，已按空白规范化。
///
/// Normalising is what makes the pinned statements comparable: a braced list may
/// be re-wrapped by `rustfmt`, and a gate that failed on the wrapping would be
/// removed rather than obeyed.
/// 规范化让钉住的那几条可比：花括号列表可能被 `rustfmt` 重新折行，而一个因折行而失败的门禁会被
/// 删掉，而不是被遵守。
pub fn xirang_reexports(text: &str) -> Vec<String> {
    let mut found = Vec::new();
    let mut from = 0usize;
    while let Some(offset) = text[from..].find("pub use xirang_kernel::") {
        let at = from + offset;
        let rest = &text[at..];
        let end = rest.find(';').map_or(rest.len(), |end| end + 1);
        found.push(rest[..end].split_whitespace().collect::<Vec<_>>().join(" "));
        from = at + end;
    }
    found
}

/// Pinned re-exports that are no longer in their file.
/// 已不再存在于其文件中的钉住重导出。
pub fn missing_shims(root: &Path) -> Vec<String> {
    let mut missing = Vec::new();
    for (file, statement) in SHIMS {
        let path = root.join(file);
        let text = std::fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()));
        let masked = xirang_kernel::source::mask_non_code(&text);
        // A pinned re-export behind `#[cfg(any())]` (never compiled) or `#[cfg(test)]` (not
        // part of the public surface) exists in the file but not in the crate a host
        // compiles against, so the ratchet reads the text that is actually built. The
        // attribute and the statement it governs sit at the same indentation, which is what
        // the shared helper handles.
        // 藏在 `#[cfg(any())]`（永不编译）或 `#[cfg(test)]`（不属于公开面）后面的钉住重导出，
        // 存在于文件里却不在宿主编译所对的那个 crate 里，因此棘轮读的是真正会被构建的文本。属性与
        // 它管辖的语句同缩进，这正是那个共享助手处理的情形。
        let live = crate::drop_governed_lines(&masked, |trimmed| {
            trimmed == "#[cfg(any())]" || trimmed == "#[cfg(test)]"
        });
        if !xirang_reexports(&live)
            .iter()
            .any(|found| found == statement)
        {
            missing.push(format!("{file}: {statement}"));
        }
    }
    missing
}
