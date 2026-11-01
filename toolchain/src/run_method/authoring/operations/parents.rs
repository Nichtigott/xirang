//! The parent facts a file-backed reader resolves child declarations against.
//! 文件读取方解析子面声明父级所依据的事实表。
//!
//! A generated child names its parent as a **module** (`.under(crate::panel::NODE_ID)`), and turning
//! that module into the parent's record spelling and kind is a fact of the package's discovered tree,
//! not an arithmetic on the spelling. The old arithmetic (`module → a/b.rs`, prefix the scan-root
//! offset) is correct only while every face lives under the walk root; an island's module is its stem,
//! so the arithmetic produced a path that does not exist and the read-back refused the parent as
//! "invalid parent path".
//! 生成的子面把父级写成**模块**（`.under(crate::panel::NODE_ID)`），而把那个模块变成父级的记录拼写与
//! kind 是包内已发现树的事实，不是对拼写做算术。旧的算术（`模块 → a/b.rs`，再加扫描根偏移）只在每个面都
//! 住在遍历根之下时成立；孤岛的模块是它的 stem，于是算术产出一个不存在的路径，读回便把父级拒成
//! "invalid parent path"。
//!
//! **Missing facts refuse.** When the reader handed a table in, a module the table does not carry is an
//! error that says so; it is never repaired by falling back to the historical reconstruction.
//! **缺事实就拒绝。** 读取方交进了表时，表里没有的模块就是一条明说的错误；绝不用历史重建把它补上。

use std::collections::btree_map::Entry;
use std::path::Path;

use super::*;
use crate::run_method::authoring::parse::{ParentFacts, ParentMap};

/// The parent facts of this package, from the walk the read-back is about to parse.
/// 本包的父级事实，来自读回即将解析的那趟遍历。
pub(super) fn facts_in(layout: &crate::source_layout::SourceLayout) -> Result<ParentMap, String> {
    let mut sources = Vec::new();
    super::visit_face_sources(&layout.scan_root, &mut sources)?;
    sources.extend(crate::build_method::explicit_placement_files(
        &layout.package_root,
        &layout.scan_root,
    ));
    sources.sort();
    sources.dedup();
    facts_of(layout, &sources)
}

/// The module chain one package-relative file is mounted at, from the layout and the ledger.
/// 一个包内相对文件所挂载的模块链，来自布局与台账。
///
/// One implementation, shared by the read-back's fact table and by the writer that has to name a new
/// face's rule module: guessing from a single coordinate (the parent's module, or the placement base)
/// is how an inherited child came to reference `crate::gauge::registry_rule` while it mounts at
/// `reading::object::gauge`.
/// 只有一份实现，读回的事实表与"必须点名新面的规则模块"的写入方共用：从单一坐标（父模块、或落点基
/// 准）猜测，正是继承子面挂载在 `reading::object::gauge`、却引用 `crate::gauge::registry_rule` 的来路。
pub(crate) fn module_chain_of(
    layout: &crate::source_layout::SourceLayout,
    package_source: &str,
) -> Result<Option<String>, String> {
    let prefix = scan_prefix(layout);
    let mappings = mappings(layout, &prefix);
    Ok(module_chain(package_source, &prefix, &mappings))
}

/// The parent facts of a known source set.
/// 一组已知源码的父级事实。
pub(super) fn facts_of(
    layout: &crate::source_layout::SourceLayout,
    sources: &[PathBuf],
) -> Result<ParentMap, String> {
    let prefix = scan_prefix(layout);
    let mappings = mappings(layout, &prefix);
    let mut facts = ParentMap::new();
    for source in sources {
        let package = xirang_multilang::source_ref::package_source(&layout.package_root, source);
        let record = xirang_multilang::source_ref::identity_spelling(
            &layout.package_root,
            &layout.identity_base,
            &package,
        );
        let Some(module) = module_chain(&package, &prefix, &mappings) else {
            continue;
        };
        let Ok(text) = fs::read_to_string(source) else {
            continue;
        };
        // A file with no declaration is a source input, not a face: it cannot be a parent.
        // 没有声明的文件是源码输入，不是面：它当不了父级。
        let Some(kind) = xirang_kernel::authoring::parse::kind_from_source_text(&text) else {
            continue;
        };
        let fact = ParentFacts {
            source: record,
            kind,
        };
        match facts.entry(module.clone()) {
            Entry::Vacant(slot) => {
                slot.insert(fact);
            }
            Entry::Occupied(slot) => {
                if slot.get() != &fact {
                    return Err(format!(
                        "two files claim the module `{module}`: `{}` and `{}`; one module path is one \
                         face, so the tree this read-back sees is ambiguous",
                        slot.get().source,
                        fact.source
                    ));
                }
            }
        }
    }
    Ok(facts)
}

/// One explicit placement that is a **discovery root**: package directory plus the module its face
/// mounts at (the file stem).
/// 一次作为**发现根**的显式落点：包内目录，加上它的面所挂载的模块（文件 stem）。
fn mappings(layout: &crate::source_layout::SourceLayout, prefix: &str) -> Vec<(String, String)> {
    use xirang_multilang::object_map::ObjectMap;
    let Ok(ledger) = ObjectMap::read_from(&layout.package_root) else {
        return Vec::new();
    };
    let mut mappings = Vec::new();
    for entry in ledger.entries() {
        let Some(directory) = entry.directory.clone() else {
            continue;
        };
        if directory.is_empty() {
            continue;
        }
        // A mapping inside the walk root is the main walk's own tree: its module chain is the
        // directory chain, not the stem.
        // 遍历根之内的映射归主遍历自己的树管：它的模块链是目录链，而不是 stem。
        if !prefix.is_empty()
            && (directory == prefix || directory.starts_with(&format!("{prefix}/")))
        {
            continue;
        }
        let file = layout.package_root.join(&entry.source);
        let Some(stem) = file.file_stem().and_then(|stem| stem.to_str()) else {
            continue;
        };
        mappings.push((directory, stem.to_owned()));
    }
    mappings
}

/// The scan root's package-relative prefix, without a trailing slash (`""`, `src`).
/// 扫描根的包内相对前缀，不带尾部斜杠（`""`、`src`）。
fn scan_prefix(layout: &crate::source_layout::SourceLayout) -> String {
    layout
        .scan_root
        .strip_prefix(&layout.package_root)
        .map(|prefix| xirang_kernel::declaration::portable_path(&prefix.to_string_lossy()))
        .unwrap_or_default()
}

/// The module chain one package-relative source is mounted at.
/// 一个包内相对源码所挂载的模块链。
fn module_chain(package: &str, prefix: &str, mappings: &[(String, String)]) -> Option<String> {
    // A mapped directory outside the walk root mounts at its stem and grows one segment per
    // directory below it. Deepest mapping wins, so a nested override owns its own subtree.
    // 遍历根之外的映射目录挂在它的 stem 上，每下一层目录长一段。最深的映射胜出，因此嵌套覆盖拥有自己的子树。
    let deepest = mappings
        .iter()
        // A mapping inside the walk root does not own a stem-rooted chain, even when the caller
        // passed the raw ledger list (the shared interpreter must not depend on pre-filtering).
        // 遍历根之内的映射不拥有"以 stem 为根"的链，即使调用方传来的是原始台账清单（共用解释器不能
        // 依赖调用方先过滤）。
        .filter(|(directory, _)| {
            !(!prefix.is_empty()
                && (directory == prefix || directory.starts_with(&format!("{prefix}/"))))
        })
        .filter(|(directory, _)| {
            package.len() > directory.len()
                && package.starts_with(directory.as_str())
                && package.as_bytes().get(directory.len()) == Some(&b'/')
        })
        .max_by_key(|(directory, _)| directory.len());
    if let Some((directory, root)) = deepest {
        let rest = &package[directory.len() + 1..];
        let mut chain = root.clone();
        for segment in Path::new(rest).parent()?.components() {
            let segment = segment.as_os_str().to_str()?;
            chain.push_str("::");
            chain.push_str(segment);
        }
        return Some(chain);
    }
    let relative = if prefix.is_empty() {
        package
    } else {
        package.strip_prefix(prefix)?.strip_prefix('/')?
    };
    let directory = Path::new(relative).parent()?;
    let mut segments = Vec::new();
    for segment in directory.components() {
        segments.push(segment.as_os_str().to_str()?);
    }
    if segments.is_empty() {
        return None;
    }
    Some(segments.join("::"))
}

#[cfg(test)]
#[path = "parents_tests.rs"]
mod parents_tests;
