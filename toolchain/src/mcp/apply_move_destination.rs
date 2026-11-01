//! Where a move puts the face: the directory to rename, the new source path and the new module.
//! 一次搬动把面放到哪里：要改名的目录、新的源码路径与新的模块。
//!
//! Split out of `apply_move.rs` to keep that file inside the repository's size ratchet. The rule is
//! one rule: a face's module is its **directory** and its source is `<dir>/<leaf>.<ext>`, and the
//! build refuses a stem that disagrees with its directory — so the directory name, the file stem and
//! the logical leaf all move together.
//! 从 `apply_move.rs` 拆出，使那个文件留在仓库的尺寸棘轮之内。规则只有一条：一个面的模块是它的
//! **目录**，源码是 `<目录>/<末段>.<扩展名>`，而构建会拒绝与目录不一致的 stem——因此目录名、文件 stem
//! 与逻辑末段三者一起搬。
//!
//! **The plan carries both coordinate systems, because a move can cross them.** A destination under the
//! scan root is written in record (identity) spelling; a destination outside it — under another island,
//! or wherever an explicit placement puts it — is written in package spelling. The old struct carried one
//! boolean for the whole move, so an island moved to the logical root was joined to the package root
//! (file at `<package>/reading/reading.rs`) while the reply printed the identity-base join
//! (`src/reading/reading.rs`), and the ledger cleared the mapping the file still needed. Execution,
//! ledger, reply and discovery now all read this one struct.
//! **这份计划同时带两套坐标，因为一次搬动可以跨越它们。** 落在扫描根之下的目的地在记录（身份）拼写里
//! 书写；根之外的目的地——另一个孤岛之下，或显式落点放它的地方——用包内拼写。旧结构整次搬动只带一个布尔，
//! 于是"孤岛搬到逻辑根"被拼到包根（文件落在 `<包>/reading/reading.rs`），而回复打印的是身份基准拼接
//! （`src/reading/reading.rs`），台账还清掉了文件仍然需要的映射。执行、台账、回复与发现现在都读这一个结构。

use std::path::Path;

use crate::build_method::FaceView;

/// Where the face's module subtree goes.
/// 这个面的模块子树搬到哪里。
pub(crate) struct Moved {
    /// The source directory in **record spelling** (the space `FaceView::source` uses).
    /// 源目录，按**记录拼写**（`FaceView::source` 所用的空间）。
    pub(crate) from_dir: String,
    /// The destination directory in record spelling.
    /// 目的地目录，按记录拼写。
    pub(crate) to_dir: String,
    /// The face file's new path in **record spelling**, which is the identity input.
    /// 面文件的新路径，按**记录拼写**——也就是身份输入。
    pub(crate) source: String,
    /// The original face file, before moving or changing its stem (record spelling).
    /// 搬迁或改名之前的面文件（记录拼写）。
    pub(crate) original_source: String,
    /// The moved face's new **module path**, which is what its children must now spell.
    /// 被搬面的新**模块路径**，也就是它的子面现在必须拼出的东西。
    pub(crate) module: String,
    /// The source directory in **package spelling** — what a filesystem operation uses.
    /// 源目录，按**包内拼写**——文件系统操作所用的那一个。
    pub(crate) from_package: String,
    /// The destination directory in package spelling.
    /// 目的地目录，按包内拼写。
    pub(crate) to_package: String,
    /// The face file's new path in package spelling.
    /// 面文件的新路径，按包内拼写。
    pub(crate) to_physical: String,
    /// Whether the destination lies under the walk root, and therefore needs no ledger mapping.
    /// 目的地是否落在遍历根之下，因而不需要台账映射。
    pub(crate) covered_by_default_walk: bool,
}

/// The destination of a move, computed from the parent, the leaf name and the layout.
/// 一次搬动的目的地，由父级、末段名字与布局算出。
///
/// A face's module is its **directory** and its source is `<dir>/<leaf>.rs`, and the build refuses a
/// stem that disagrees with its directory — so the directory name, the file stem and the logical leaf
/// all move together. The parent decides the directory in **its own** coordinate system (an island
/// parent keeps children in the island), and the root's default location is below the walk root.
/// 一个面的模块是它的**目录**，源码是 `<目录>/<末段>.rs`，而构建会拒绝与目录不一致的 stem——因此目录名、
/// 文件 stem 与逻辑末段三者一起搬。父级在**它自己的**坐标系里决定目录（孤岛父级把子面留在孤岛里），而根的
/// 默认位置在遍历根之下。
pub(crate) fn moved_source(
    face: &FaceView,
    parent: Option<&FaceView>,
    name: &str,
    layout: &crate::source_layout::SourceLayout,
) -> Moved {
    let extension = Path::new(&face.source)
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or("rs")
        .to_owned();
    let from_dir = directory_of(&face.source);
    let from_package = package_directory(layout, face.source_base, &face.source);
    let scan_prefix = layout
        .scan_root
        .strip_prefix(&layout.package_root)
        .map(|prefix| xirang_kernel::declaration::portable_path(&prefix.to_string_lossy()))
        .unwrap_or_default();
    // The destination directory in **package** coordinates. A parent contributes its own directory,
    // so an island's children stay in the island; the root contributes the walk root, which is the
    // one location default discovery covers.
    // 目的地目录，按**包内**坐标。父级贡献它自己的目录，因此孤岛的子面留在孤岛里；根贡献遍历根，也就是
    // 默认发现唯一覆盖的位置。
    let to_package = match parent {
        Some(parent) => {
            let parent_package = package_directory(layout, parent.source_base, &parent.source);
            join(&parent_package, name)
        }
        None => join(&scan_prefix, name),
    };
    let to_physical = format!("{to_package}/{name}.{extension}");
    let record_directory = xirang_multilang::source_ref::identity_spelling(
        &layout.package_root,
        &layout.identity_base,
        &to_package,
    );
    let source = xirang_multilang::source_ref::identity_spelling(
        &layout.package_root,
        &layout.identity_base,
        &to_physical,
    );
    let module = match parent {
        Some(parent) if !parent.module.is_empty() => format!("{}::{name}", parent.module),
        _ => name.to_owned(),
    };
    let covered_by_default_walk = scan_prefix.is_empty()
        || to_package == scan_prefix
        || to_package.starts_with(&format!("{scan_prefix}/"));
    Moved {
        source,
        original_source: face.source.clone(),
        module,
        from_dir,
        to_dir: record_directory,
        from_package,
        to_package,
        to_physical,
        covered_by_default_walk,
    }
}

/// The directory part of a relative source path.
/// 相对源码路径的目录部分。
pub(crate) fn directory_of(source: &str) -> String {
    match source.rsplit_once('/') {
        Some((directory, _)) => directory.to_owned(),
        None => String::new(),
    }
}

/// A face's source directory in package spelling.
/// 一个面的源码目录，按包内拼写。
fn package_directory(
    layout: &crate::source_layout::SourceLayout,
    base: xirang_multilang::source_ref::SourceBase,
    source: &str,
) -> String {
    let package = xirang_multilang::source_ref::package_relative(
        base,
        &layout.package_root,
        &layout.identity_base,
        source,
    );
    directory_of(&package)
}

/// Join a directory and a leaf without a doubled separator at the package root.
/// 拼合目录与末段，包根处不产生双斜杠。
fn join(directory: &str, leaf: &str) -> String {
    if directory.is_empty() {
        leaf.to_owned()
    } else {
        format!("{directory}/{leaf}")
    }
}
