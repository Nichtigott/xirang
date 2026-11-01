//! The host-side re-export that keeps `crate::<claimed>::…` valid after a partition.
//! 分区之后让 `crate::<被认领子树>::…` 继续有效的宿主侧 re-export。
//!
//! The types a claimed subtree declares move to a generated crate, but host faces outside that subtree
//! still name them through the original path. This module emits a compatibility module in the host's
//! generated tree that re-exports the generated crate's module — the same type, trait and identity,
//! not a copy. It is emitted only when the generated crate already exists: a declaration alone must
//! not make the host reference a package nobody has written yet.
//! 被认领子树声明的类型搬进生成 crate，而该子树之外的宿主面仍用原路径点名它们。本模块在宿主生成的树里
//! 发射一个兼容模块，re-export 生成 crate 的模块——同一个类型、trait 与身份，不是副本。只有生成 crate
//! 已经存在时才发射：仅有一份声明，不能让宿主去引用一个还没人写下的包。

use std::path::Path;

use super::crate_plan::partition_roots;
use super::shape_decl::ShapeDeclaration;

/// Append the compatibility modules a host needs, when this run is the host and a package exists.
/// 在本次运行是宿主、且生成包存在时追加兼容模块。
pub(crate) fn append(
    generated: &mut String,
    shape: Option<&ShapeDeclaration>,
    host_root: &Path,
    host_mode: bool,
) {
    if host_mode && let Some(shape) = shape {
        generated.push_str(&reexports(shape, host_root));
    }
}

/// The compatibility modules a host needs after its declared split has been written.
/// 宿主在声明的拆分写下之后所需的兼容模块。
pub(crate) fn reexports(shape: &ShapeDeclaration, host_root: &Path) -> String {
    let root = partition_roots(host_root).0;
    let manifest = std::fs::read_to_string(host_root.join("Cargo.toml")).unwrap_or_default();
    let mut text = String::new();
    for (name, subtrees) in &shape.crates {
        let package = format!("{}-{name}", shape.package_prefix);
        let ident = package.replace('-', "_");
        let directory = super::crate_plan::crates_dir(&root).join(&package);
        if !directory.exists() {
            continue;
        }
        // Presence of the directory is not the condition: the host must **depend** on the generated
        // crate for a re-export to resolve. A committed partition whose host deliberately does not
        // depend on the fragments (their faces are not named from the host) must stay without the
        // module — otherwise the host names a crate it never linked.
        // 目录存在不是条件：宿主必须**依赖**生成 crate，re-export 才解析得到。入库的分区若宿主刻意不依赖
        // 碎片（宿主不点名它们的面），就不能有那个模块——否则宿主会点名一个从未链接的 crate。
        if !manifest
            .lines()
            .any(|line| line.trim_start().starts_with(&format!("{package} =")))
        {
            continue;
        }
        for subtree in subtrees {
            let module = subtree.trim_start_matches("crate::");
            if module.is_empty() {
                continue;
            }
            text.push_str(&nested(module, &ident, 0));
        }
    }
    text
}

/// One nested module chain whose innermost module re-exports the generated crate's module.
/// 一条嵌套模块链，最内层 re-export 生成 crate 的模块。
fn nested(module: &str, ident: &str, depth: usize) -> String {
    let (head, rest) = module.split_once("::").unwrap_or((module, ""));
    let indent = "    ".repeat(depth);
    if rest.is_empty() {
        return format!(
            "{indent}#[doc(hidden)]\npub mod {head} {{\n{indent}    pub use ::{ident}::{module}::*;\n{indent}}}\n"
        );
    }
    let inner = nested(rest, ident, depth + 1);
    format!("{indent}#[doc(hidden)]\npub mod {head} {{\n{inner}{indent}}}\n")
}

/// Whether a module is at or under one of the claimed subtrees.
/// 某个模块是否就在某条认领的子树之下（含它自己）。
pub(super) fn under_a_claim<'a>(module: &str, claims: impl Iterator<Item = &'a str>) -> bool {
    claims
        .filter(|claim| !claim.is_empty())
        .any(|claim| module == claim || module.starts_with(&format!("{claim}::")))
}
