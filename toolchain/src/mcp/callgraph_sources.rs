//! Reading the corpus a `callgraph` answer may cite: which files to list, and which of them to
//! parse.
//! 读取 `callgraph` 答案可能引用的语料：列出哪些文件，以及解析其中哪些。
//! online: the build publishes faces, not call edges, so a call graph is a property of the sources as they are now.
//!
//! Split from `callgraph.rs` so both files stay inside the repository's size budget. The two-pass
//! rule is the interesting part: the first pass parses files that mention a queried name (definition
//! or call site), and the second parses files that mention the leaves of the calls made by the
//! matching functions — that is what keeps "where is the callee defined" answerable without parsing
//! the whole tree.
//! 从 `callgraph.rs` 拆出，使两个文件都留在仓库的尺寸预算内。真正有意思的是两趟规则：第一趟解析提到被查询
//! 名字（定义或调用点）的文件，第二趟解析提到"匹配函数所发出的调用的叶名"的文件——正是它让"被调用者定义在
//! 哪"无需要解析整棵树也能回答。
//!
//! Both passes share one workspace scope and one registration cache: the scope costs a `cargo
//! metadata` run and the registration facts cost a declaration walk per member, so a query pays for
//! each once. Measured on this checkout, re-deriving them per pass and per language was 30 s of a
//! 41 s answer.
//! 两趟共享同一个工作区范围与同一份注册缓存：范围要跑一次 `cargo metadata`，注册事实要对每个成员走一遍声明
//! 遍历，因此一次查询各付一次。本检出实测：按趟、按语言重算曾占 41 s 答案里的 30 s。
//! online: 构建发布的是面而不是调用边，因此调用图是"源码此刻是什么样"的属性。

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::mcp::source_index::load_sources_matching;

use super::{RegistrationOwners, call_matches, cpp_sources, python_sources, registration_owners};

type LabelledSources = Vec<(String, crate::mcp::source_index::SourceFile)>;
type LoadedSources = (LabelledSources, Vec<String>);
type OwnersCache = BTreeMap<PathBuf, Option<RegistrationOwners>>;

/// The Rust index, parsing only the files whose text mentions one of `leaves`.
/// Rust 索引：只解析文本提到 `leaves` 之一的文件。
fn load_sources_for(
    root: &Path,
    leaves: &[&str],
) -> Result<Vec<crate::mcp::source_index::SourceFile>, String> {
    if leaves.is_empty() {
        return crate::mcp::source_index::load_sources(root);
    }
    load_sources_matching(root, |_, text| {
        leaves.iter().any(|leaf| text.contains(leaf))
    })
}

pub(super) fn labelled_sources(root: &Path, hint: Option<&str>) -> Result<LoadedSources, String> {
    labelled_sources_with(root, hint, &[], &[])
}

/// The same sources, parsing only the files whose text mentions one of `leaves`.
/// 同样的源码，只解析文本提到 `leaves` 之一的文件。
///
/// An empty `leaves` parses everything: that is the orphan view's question ("what does nothing
/// call"), which cannot be narrowed by a name.
/// 空的 `leaves` 解析全部：那是孤儿视图的问题（"什么是没人调用的"），它无法按名字收窄。
pub(super) fn labelled_sources_with(
    root: &Path,
    hint: Option<&str>,
    leaves: &[&str],
    names: &[&str],
) -> Result<LoadedSources, String> {
    // One scope and one owners cache for both passes: `cargo metadata` and the declaration walk do
    // not depend on the leaf set.
    // 两趟共用一个范围与一份注册缓存：`cargo metadata` 与声明遍历都不依赖叶名集合。
    let scope = crate::mcp::workspace::scope(root);
    let mut owners = OwnersCache::new();
    let first = load_corpus(root, &scope, hint, leaves, &mut owners)?;
    if leaves.is_empty() {
        return Ok(first);
    }
    let (mut labelled, notes) = first;
    // A callee's location is part of the answer ("where is that one defined"), and the callee may
    // live in a file that never mentions the queried name — the first pass parsed only files that
    // could contain the definition or a call site of the **asked** name, so those files came back
    // with no functions. One second pass, filtered by the leaves of the calls made by the matching
    // functions, is enough: the answer prints one hop, not the transitive closure.
    // 被调用者的位置是答案的一部分（"那一个又定义在哪"），而它可能住在一个从不提被查询名字的文件里——第一
    // 趟只解析了可能包含**被问**名字的定义或调用点的文件，因此那些文件回来时没有函数。按"匹配函数所发出的
    // 调用的叶名"再过滤一趟就够了：答案只印一跳，不是传递闭包。
    let mut extra: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    for (_, file) in &labelled {
        for function in &file.functions {
            let wanted = names.iter().any(|name| {
                function.name == *name
                    || function.qualified == *name
                    || function.qualified.split('(').next() == Some(*name)
                    || call_matches(function, name)
            });
            if !wanted {
                continue;
            }
            for call in &function.calls {
                let leaf = call.rsplit("::").next().unwrap_or(call);
                if !leaf.is_empty() && !leaves.contains(&leaf) {
                    extra.insert(leaf.to_owned());
                }
            }
        }
    }
    if extra.is_empty() {
        let mut sources = labelled;
        sources.sort_by(|left, right| left.0.cmp(&right.0));
        return Ok((sources, notes));
    }
    // The corpus already holds every file's text; the second pass only fills in the functions of
    // the files the first pass left unparsed (they did not mention a queried leaf) but that mention
    // a callee leaf. Re-reading and re-walking the tree for that was the cost measured here: the
    // common name `parse` re-parsed the whole corpus once.
    // 语料已持有每份文件的文本；第二趟只给"第一趟没解析（没提到被查询叶名）、却提到某个被调用者叶名"的文件
    // 补上函数。为此重读、重遍历整棵树就是这里量到的成本：常见名 `parse` 曾把整份语料重解析一遍。
    for (_, file) in &mut labelled {
        if !file.functions.is_empty() {
            continue;
        }
        if leaves.iter().any(|leaf| file.source.contains(leaf)) {
            continue;
        }
        if !extra.iter().any(|leaf| file.source.contains(leaf.as_str())) {
            continue;
        }
        crate::mcp::source_index::parse_in_place(file);
    }
    labelled.sort_by(|left, right| left.0.cmp(&right.0));
    Ok((labelled, notes))
}

/// One root's registration facts, computed at most once per query.
/// 一个根的注册事实，一次查询里最多计算一次。
fn owners_for<'a>(cache: &'a mut OwnersCache, root: &Path) -> &'a Option<RegistrationOwners> {
    cache
        .entry(root.to_path_buf())
        .or_insert_with(|| registration_owners(root))
}

/// Attach one root's registration facts to every language's sources.
/// 把一个根的注册事实附加到各语言的源码上。
fn attach(
    owners: &Option<RegistrationOwners>,
    sources: &mut [crate::mcp::source_index::SourceFile],
) {
    if let Some(owners) = owners {
        owners.apply(sources);
    }
}

/// One load of every source the answer may read, parsing only files whose text mentions one of
/// `leaves`.
/// 一次读取答案可能读到的全部源码，只解析文本提到 `leaves` 之一的文件。
fn load_corpus(
    root: &Path,
    scope: &Result<crate::mcp::workspace::Scope, String>,
    hint: Option<&str>,
    leaves: &[&str],
    owners: &mut OwnersCache,
) -> Result<LoadedSources, String> {
    let mut labelled: Vec<(String, crate::mcp::source_index::SourceFile)> = Vec::new();
    let mut notes: Vec<String> = Vec::new();
    match scope {
        Ok(crate::mcp::workspace::Scope::Workspace(members)) => {
            for member in members {
                let Ok(mut sources) = load_sources_for(&member.dir, leaves) else {
                    continue;
                };
                // The label is the member's directory relative to the workspace root, which is
                // the path a reader can grep for; the package name alone would name the identity
                // but not the file's place in this checkout.
                // 标签是成员目录相对工作区根的路径，也就是读者能直接 grep 的路径；只用包名会点名身份，
                // 却不说明文件在本次检出里的位置。
                let prefix = member
                    .dir
                    .strip_prefix(root)
                    .map(|relative| {
                        xirang_kernel::declaration::portable_path(&relative.to_string_lossy())
                    })
                    .unwrap_or_else(|_| member.name.clone());
                let (mut python, mut python_notes) = python_sources(&member.dir, hint);
                let (mut cpp, mut cpp_notes) = cpp_sources(&member.dir, hint);
                let has_sources = !sources.is_empty() || !python.is_empty() || !cpp.is_empty();
                if has_sources {
                    let facts = owners_for(owners, &member.dir);
                    attach(facts, &mut sources);
                    attach(facts, &mut python);
                    attach(facts, &mut cpp);
                }
                for note in &mut python_notes {
                    *note = format!("{prefix}/{}", note);
                }
                notes.append(&mut python_notes);
                for note in &mut cpp_notes {
                    *note = format!("{prefix}/{}", note);
                }
                notes.append(&mut cpp_notes);
                for file in sources {
                    labelled.push((format!("{prefix}/{}", file.relative), file));
                }
                for file in python {
                    labelled.push((format!("{prefix}/{}", file.relative), file));
                }
                for file in cpp {
                    labelled.push((format!("{prefix}/{}", file.relative), file));
                }
            }
        }
        _ => {
            let mut sources = load_sources_for(root, leaves)?;
            let (mut python, mut python_notes) = python_sources(root, hint);
            let (mut cpp, mut cpp_notes) = cpp_sources(root, hint);
            let has_sources = !sources.is_empty() || !python.is_empty() || !cpp.is_empty();
            if has_sources {
                let facts = owners_for(owners, root);
                attach(facts, &mut sources);
                attach(facts, &mut python);
                attach(facts, &mut cpp);
            }
            notes.append(&mut python_notes);
            notes.append(&mut cpp_notes);
            for file in sources {
                labelled.push((file.relative.clone(), file));
            }
            for file in python {
                labelled.push((file.relative.clone(), file));
            }
            for file in cpp {
                labelled.push((file.relative.clone(), file));
            }
        }
    }
    notes.sort();
    notes.dedup();
    labelled.sort_by(|left, right| left.0.cmp(&right.0));
    Ok((labelled, notes))
}
