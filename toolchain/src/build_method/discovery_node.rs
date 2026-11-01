//! The discovered registration module tree and its path naming.
//! 已发现的注册模块树及其路径命名。

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// One discovered registration module.
/// 一个已发现的注册模块。
#[derive(Clone, Debug)]
pub(crate) struct Node {
    pub(crate) name: String,
    pub(crate) file: Option<PathBuf>,
    pub(crate) children: Vec<Node>,
}

thread_local! {
    /// The record spelling of every file this thread's discovery pass placed, by materialized path.
    /// 本线程发现那一趟安放过的每个文件的记录拼写，按物化路径索引。
    ///
    /// The discovery pass is the **only** place that holds both roots (`package_root` and
    /// `identity_base`) and the materialized file at the same time, so it is where the record
    /// spelling becomes a fact. The passes after it take identities and keys from this table instead
    /// of re-deriving a spelling from one root — re-deriving is what made an explicitly placed face
    /// outside the walk root hash its **absolute path** while the declaration macro hashed the
    /// package-relative one, so the generated plan's identity and the compiled identity disagreed
    /// about one file (and the const identity assertion refused the build).
    /// 发现那一趟是**唯一**同时握有两个基准（`package_root` 与 `identity_base`）与物化文件的地方，因此记录
    /// 拼写在那里成为事实。它之后的几趟从这张表取身份与键，而不是从一个基准重新推导拼写——重新推导正是
    /// "遍历根之外显式落点的面拿**绝对路径**去散列、而声明宏拿包内相对拼写去散列"的来路，于是生成计划的身份与
    /// 编译出来的身份对同一个文件各说各话（而常量身份断言会拒绝这次构建）。
    static SOURCE_SPELLINGS: RefCell<Option<BTreeMap<PathBuf, String>>> = const { RefCell::new(None) };

    /// The file each module chain is mounted from, by the chain the generated tree gives it.
    /// 每条模块链所挂载的文件，按生成树给它的那条链索引。
    ///
    /// The chain is a **discovery fact**, not a path: an explicitly placed face mounts at its stem
    /// (`components/reading/reading.rs` is `reading`), so a consumer that rebuilds a path from a
    /// module name by joining the identity base looks for a file that never existed. Parent
    /// resolution is that consumer: a declaration names its parent by module (`crate::panel`);
    /// this table is where that name becomes the file to read.
    /// 模块链是**发现事实**，不是路径：显式落点的面挂在它的 stem 上（`components/reading/reading.rs` 是
    /// `reading`），因此把模块名拼到身份基准上重建路径的消费者会去找一个从来不存在的文件。父级解析正是
    /// 那个消费者：声明用模块（`crate::panel`）点名父级；这张表就是那个名字变成"要读的文件"的地方。
    static MODULE_FILES: RefCell<Option<BTreeMap<String, PathBuf>>> = const { RefCell::new(None) };
}

/// Remember the record spelling of every file this discovery pass placed.
/// 记住本次发现那一趟安放过的每个文件的记录拼写。
pub(crate) fn remember_source_spellings(
    layout: &crate::source_layout::SourceLayout,
    nodes: &[Node],
) {
    fn note_placement(
        layout: &crate::source_layout::SourceLayout,
        nodes: &[Node],
        prefix: &str,
        spellings: &mut BTreeMap<PathBuf, String>,
        modules: &mut BTreeMap<String, PathBuf>,
    ) {
        for node in nodes {
            // The module chain grows one segment per mounting node, exactly as the generated tree
            // mounts it: a root-level explicit placement is one segment however deep its directory is.
            // 模块链每个挂载节点长一段，正是生成树的挂载方式：根级的显式落点无论目录多深都只有一段。
            let module = if prefix.is_empty() {
                node.name.clone()
            } else {
                format!("{prefix}::{}", node.name)
            };
            if let Some(file) = &node.file {
                spellings.insert(file.clone(), record_spelling(layout, file));
                modules.insert(module.clone(), file.clone());
            }
            note_placement(layout, &node.children, &module, spellings, modules);
        }
    }
    let mut spellings = BTreeMap::new();
    let mut modules = BTreeMap::new();
    note_placement(layout, nodes, "", &mut spellings, &mut modules);
    SOURCE_SPELLINGS.with(|slot| *slot.borrow_mut() = Some(spellings));
    MODULE_FILES.with(|slot| *slot.borrow_mut() = Some(modules));
}

/// The file a module chain is mounted from, when this thread's discovery pass placed it.
/// 一条模块链所挂载的文件——当本线程的发现那一趟安放过它时。
pub(crate) fn module_file(module: &str) -> Option<PathBuf> {
    MODULE_FILES.with(|slot| {
        slot.borrow()
            .as_ref()
            .and_then(|modules| modules.get(module).cloned())
    })
}

/// The record spelling a file this thread's discovery pass placed carries.
/// 本线程发现那一趟安放过的文件所携带的记录拼写。
pub(crate) fn remembered_spelling(file: &Path) -> Option<String> {
    SOURCE_SPELLINGS.with(|slot| {
        slot.borrow()
            .as_ref()
            .and_then(|spellings| spellings.get(file).cloned())
    })
}

/// The spelling the identity and cache layers key on.
/// 身份层与缓存层用作键的拼写。
///
/// The discovery fact when this pass placed the file; otherwise the display fold, which is what a
/// file outside the walk (a declared source, a parent read by path) has always been named by.
/// 本次遍历安放过该文件时用发现事实；否则用显示折叠——遍历之外的文件（声明式来源、按路径读到的父级）
/// 一直都是那样命名的。
pub(crate) fn source_spelling(src: &Path, file: &Path) -> String {
    remembered_spelling(file).unwrap_or_else(|| relative_display(src, file))
}

/// Display a discovered file path relative to the package source root.
/// 按相对包源码根的写法显示已发现文件的路径。
///
/// Generated manifests, caches and diagnostics all name sources the same way, and
/// so does every other surface that renders a recorded `file!()` path, so the
/// conversion is not written here: it forwards to the kernel's
/// [`xirang_kernel::declaration::portable_path`], the one fold in the workspace. A
/// second copy of the rule is how two surfaces end up spelling the same file
/// differently — and that failure is invisible on the platform whose separators
/// happen to agree.
/// 生成的清单、缓存与诊断都用同一种写法命名源码，其他渲染记录下来的 `file!()` 路径的执行面也是，
/// 因此这份转换不在这里写：它转发到内核的 [`xirang_kernel::declaration::portable_path`]，那是全仓
/// 唯一的折叠。规则的第二份副本正是两个面对同一个文件给出不同拼法的成因——而在分隔符恰好一致的
/// 平台上，这种失败看不出来。
pub(crate) fn relative_display(src: &Path, path: &Path) -> String {
    xirang_kernel::declaration::portable_path(
        &path.strip_prefix(src).unwrap_or(path).to_string_lossy(),
    )
}

/// One file's package-relative source spelling (the shared fact every consumer joins to the root).
/// 一个文件的包内相对源码拼写（每个消费者拼回包根用的共同事实）。
///
/// The fold itself lives in `xirang_multilang::source_ref`, because the authoring read-back needs the
/// same answer without the `build` feature — and a second copy is how the two sides would come to
/// disagree about one file's identity.
/// 折叠本体住在 `xirang_multilang::source_ref`，因为创作读取方在没有 `build` 特性时也需要同一个答案
/// ——而第二份副本正是两侧对同一个文件的身份各说各话的来路。
pub(crate) fn package_source(layout: &crate::source_layout::SourceLayout, file: &Path) -> String {
    xirang_multilang::source_ref::package_source(&layout.package_root, file)
}

/// The spelling the published records and identities carry (see [`package_source`]).
/// 已发布记录与身份所携带的拼写（见 [`package_source`]）。
pub(crate) fn record_spelling(layout: &crate::source_layout::SourceLayout, file: &Path) -> String {
    xirang_multilang::source_ref::record_spelling(&layout.package_root, &layout.identity_base, file)
}

/// The files an explicit placement put outside the walk's root, by materialized path.
/// 显式落点放在遍历根之外的文件，按物化路径。
///
/// Only the authoring read-back asks for these; the build takes the nodes themselves
/// ([`explicit_placement_nodes`]), so this is gated with its one caller.
/// 只有创作读取方要这些；构建直接用节点本身（[`explicit_placement_nodes`]），因此这里与唯一的调用方同一
/// 个特性门控。
#[cfg(feature = "authoring")]
pub(crate) fn explicit_placement_files(package_root: &Path, scan_root: &Path) -> Vec<PathBuf> {
    fn gather_files(nodes: Vec<Node>, into: &mut Vec<PathBuf>) {
        for node in nodes {
            if let Some(file) = node.file {
                into.push(file);
            }
            gather_files(node.children, into);
        }
    }
    let mut files = Vec::new();
    gather_files(
        explicit_placement_nodes(package_root, scan_root),
        &mut files,
    );
    files
}

/// The faces a project placed **outside the walk's root**, read from the object ledger.
/// 项目放在**遍历根之外**的面，从对象台账读出来。
///
/// The ledger mapping is a **discovery root**, not one file: a face's descendants inherit its
/// directory, and a nested override maps a directory of its own. So each mapping is walked with the
/// same rules the main walk uses — a directory is a module when it carries its attached
/// `<name>/<name>.rs`, ordinary `.rs` files are inputs rather than modules, and only Rust sources
/// enter the Rust module tree (a foreign face is compiled and registered by its own adapter, and its
/// declaration is not a Rust module body). The logical parent of every face still comes from the
/// declaration; this walk only decides what the compiler sees.
/// 台账映射是**发现根**，不是一个文件：一个面的后代继承它的目录，而嵌套覆盖映射自己的目录。因此每个
/// 映射都按主遍历的同一条规则走——目录当且仅当带有 `<name>/<name>.rs` 时才是模块，普通 `.rs` 文件是
/// 输入而不是模块，且只有 Rust 源码进入 Rust 模块树（外语面由它自己的适配器编译登记，它的声明不是
/// Rust 模块体）。每个面的逻辑父级仍来自声明；这趟遍历只决定编译器看见什么。
///
/// **Overlap is deduplicated by the shallower mapping winning.** A mapping whose directory lies under
/// the main walk root, or under a mapping already walked, contributes nothing new — walking it twice
/// would mount one file at two module paths and make the identity assertions disagree with
/// themselves.
/// **重叠按"更浅的映射胜出"去重。** 目录落在主遍历根之下、或落在已经走过的映射之下的映射不再贡献
/// 新东西——走两遍会把一个文件挂到两条模块路径上，并让身份断言自相矛盾。
pub(crate) fn explicit_placement_nodes(package_root: &Path, scan_root: &Path) -> Vec<Node> {
    use xirang_multilang::object_map::ObjectMap;
    let Ok(ledger) = ObjectMap::read_from(package_root) else {
        return Vec::new();
    };
    // Shallowest first, so a deeper mapping inside an already-walked tree is recognized as covered.
    // 最浅的在前，因此已走树内部的更深映射会被认成"已被覆盖"。
    let mut mappings: Vec<(String, String)> = ledger
        .entries()
        .iter()
        .filter_map(|entry| {
            let directory = entry.directory.clone()?;
            Some((directory, entry.source.clone()))
        })
        .collect();
    mappings.sort_by_key(|(directory, _)| directory.matches('/').count());
    let mut walked: Vec<PathBuf> = Vec::new();
    let mut nodes = Vec::new();
    for (directory, source) in mappings {
        let mapped_directory = package_root.join(&directory);
        let file = package_root.join(&source);
        // A mapping inside the walk root is the main walk's own job; only the part outside it is
        // appended. The file check keeps a stale ledger entry from inventing a node.
        // 遍历根之内的映射归主遍历自己管，只有根之外的那部分被追加。文件检查让陈旧台账条目不会凭空造出
        // 一个节点。
        if directory.is_empty()
            || mapped_directory.starts_with(scan_root)
            || file.starts_with(scan_root)
            || !file.is_file()
            || file.extension().and_then(|extension| extension.to_str()) != Some("rs")
        {
            continue;
        }
        if walked.iter().any(|root| mapped_directory.starts_with(root)) {
            continue;
        }
        let Some(stem) = file.file_stem().and_then(|stem| stem.to_str()) else {
            continue;
        };
        // The same walk the main root uses, minus the findings: a mapped directory is a source
        // input, and a `.rs` file that is not at `<name>/<name>.rs` is not a module here either.
        // 与主根同一趟遍历，但不带发现清单：映射目录是源码输入，而不在 `<name>/<name>.rs` 的 `.rs`
        // 文件在这里同样不是模块。
        let mut ignored = Vec::new();
        let children = super::source_walk::discover_root_reporting(&mapped_directory, &mut ignored);
        nodes.push(Node {
            name: stem.to_owned(),
            file: Some(file),
            children,
        });
        walked.push(mapped_directory);
    }
    nodes
}

#[cfg(test)]
mod tests {
    use super::relative_display;
    use std::path::Path;

    /// The fold this function produces *is* the kernel's: the assertion compares
    /// against [`xirang_kernel::declaration::portable_path`] instead of against a
    /// literal, so a second, divergent fold in this file fails even on a platform
    /// whose separators make the difference unobservable.
    /// 本函数产出的折叠**就是**内核的那一份：断言与
    /// [`xirang_kernel::declaration::portable_path`] 比较而不是与字面量比较，因此本文件里第二份
    /// 走向不同的折叠，会在分隔符让差异不可见的平台上同样失败。
    #[test]
    fn a_relative_display_is_the_kernel_fold() {
        let root = Path::new("pkg/src");
        let written = Path::new("pkg/src/control\\object\\button.rs");
        assert_eq!(
            relative_display(root, written),
            xirang_kernel::declaration::portable_path("control/object/button.rs")
        );
        assert_eq!(
            relative_display(root, written),
            "control/object/button.rs",
            "a path recorded outside Windows still renders the portable way"
        );
    }
}
