//! Plan the explicit Rust references a move has to rewrite alongside the rename.
//! 规划一次搬动除改名之外还必须改写的显式 Rust 引用。
//!
//! The filesystem walk lives here rather than in the kernel: the kernel planner
//! ([`xirang_kernel::syntax`]) only says what one parsed file's reference rewrite is,
//! and this module binds that answer to the files of one package. The result is
//! deliberately a **plan**, not a write: the caller re-reads every byte before it
//! touches disk and refuses the whole move when one of them already changed.
//! 文件系统遍历住在这里而不是内核：内核规划器（[`xirang_kernel::syntax`]）只说一个已解析文件的引用
//! 改写是什么，本模块把那个答案绑定到某个包的文件上。结果有意是一份**计划**而不是写入：调用方在碰盘
//! 之前重读每一份字节，任何一份已经变化就拒绝整次搬动。

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use crate::build_method::FaceView;
use xirang_kernel::syntax::{
    ModuleMove, PathContext, TextEdit, apply_text_edits_checked, plan_include_rewrites,
    plan_reference_rewrites_in,
};

use super::Moved;

/// The byte replacements planned for one file, bound to the bytes they describe.
/// 为一个文件规划、并绑定到其所描述字节的替换。
pub(super) struct FilePlan {
    /// The file as it is now, absolute.
    /// 文件当前所在的绝对路径。
    pub file: PathBuf,
    /// The exact content the edits were computed from.
    /// 计算这些替换时所用的精确内容。
    pub before: String,
    /// Non-overlapping replacements.
    /// 互不重叠的替换。
    pub edits: Vec<TextEdit>,
}

/// What the reference pass found across the package.
/// 引用遍历在整个包上发现的东西。
pub(super) struct Plan {
    /// Files that would change, each bound to its original bytes.
    /// 会改变的文件，各自绑定其原始字节。
    pub files: Vec<FilePlan>,
    /// References that could not be decided; the move must not claim these fixed.
    /// 无法裁定的引用；搬动不得声称它们已修好。
    pub unresolved: Vec<String>,
    /// Directories deliberately outside the scan, so the reply can say what it did not read.
    /// 有意排除在扫描之外的目录，使回复能说出它没读什么。
    pub skipped: Vec<String>,
    /// `.rs` files that physically move with the subtree but carry no registration face.
    /// 随子树物理搬动、却不带注册面的 `.rs` 文件。
    ///
    /// They change path like every other byte under the renamed directory, but there is no
    /// `NodeId` to map: a record keyed by one of them never existed. Counting them is what
    /// keeps "every moved entity has an identity mapping" from reading as more than it is.
    /// 它们与改名目录下的每个其它字节一样改变路径，却没有可映射的 `NodeId`：以它们为键的记录从未存在。
    /// 把它们数出来，正是为了让"每个被搬实体都有身份映射"这句话不被读成比实际更强的东西。
    pub faceless: Vec<String>,
}

/// Which files a move may rewrite, and how each one spells the crate.
/// 一次搬动可以改写哪些文件，以及每个文件如何拼出这个 crate。
///
/// The package root is the boundary: everything under it belongs to the package, and a
/// directory that carries its own `Cargo.toml` is a different package — a vendored
/// dependency, a graft source, a nested workspace member — whose bytes this move has no
/// business touching.
/// 包根就是边界：它之下的一切属于这个包，而自带 `Cargo.toml` 的目录是另一个包——vendored 依赖、graft
/// 源码、嵌套的工作区成员——这次搬动没有理由去动它的字节。
pub(super) struct Scope<'a> {
    /// The package root, and the walk's outer boundary.
    /// 包根，也是遍历的外边界。
    pub package_root: &'a Path,
    /// The directory the build walks: the crate the move happens in.
    /// 构建遍历的目录：搬动所在的那个 crate。
    pub scan_root: &'a Path,
    /// The base a face's identity path is measured from.
    /// 面的身份路径所相对的基准。
    pub identity_base: &'a Path,
    /// The crate's own name, for consumers that spell it.
    /// crate 自己的名字，供拼出它的消费者使用。
    pub crate_name: Option<&'a str>,
}

/// Refuse a move that would leave a non-Rust declaration's references stale.
/// 拒绝一次会让非 Rust 声明的引用留在过期状态的搬动。
///
/// The generic planner rewrites Rust sources; a language adapter may plan the equivalent edit or
/// may not. Silence here is the dangerous case: the object would move, its `.under(...)`/import
/// would not, and the reply would look successful. Rust is the build's own language and is handled
/// by [`plan`]; every other extension asks its adapter.
/// 通用规划器改写 Rust 源码；语言适配器可能能规划等价改写，也可能不能。这里的沉默最危险：对象搬了、
/// 它的 `.under(...)`/import 没搬，回复却看起来成功。Rust 是构建自己的语言，由 [`plan`] 处理；其余
/// 扩展名问适配器。
pub(super) fn ensure_structural_support(views: &[FaceView], from_dir: &str) -> Result<(), String> {
    for view in views {
        if !Path::new(&view.source).starts_with(from_dir) {
            continue;
        }
        if view.source.ends_with(".rs") {
            continue;
        }
        let registry = xirang_multilang::registration::registry();
        let language = registry.language_of_path(&view.source);
        let supported = language
            .and_then(|language| registry.adapter(language))
            .map(|adapter| adapter.structural().move_source)
            .unwrap_or(false);
        if !supported {
            let language = language
                .map(|language| language.as_str())
                .unwrap_or("an unregistered language");
            return Err(format!(
                "`{}` is declared in `{}` ({}), and its adapter does not yet plan a structural \
                 move; moving it would leave its references stale, so nothing changed. Remove it \
                 from the moved subtree, or enable/implement the adapter's `move_source` \
                 capability",
                view.path, view.source, language
            ));
        }
    }
    Ok(())
}

/// Every module path the move relocates, root first.
/// 本次搬动迁移的每一条模块路径，根在前。
pub(super) fn module_moves(views: &[FaceView], face: &FaceView, moved: &Moved) -> Vec<ModuleMove> {
    let mut moves = vec![ModuleMove {
        from: face.module.clone(),
        to: moved.module.clone(),
    }];
    for view in views.iter().filter(|view| view.id != face.id) {
        let Some(suffix) = view.module.strip_prefix(&face.module) else {
            continue;
        };
        if !suffix.is_empty() && !suffix.starts_with("::") {
            continue;
        }
        // The physical directory is the identity: only a file under the moved
        // directory changes its module path, whatever parent it declares.
        if !Path::new(&view.source).starts_with(&moved.from_dir) {
            continue;
        }
        moves.push(ModuleMove {
            from: view.module.clone(),
            to: format!("{}{suffix}", moved.module),
        });
    }
    moves
}

/// Walk the package's own Rust sources and plan the reference rewrites for `moves`.
/// 遍历包自己的 Rust 源码，为 `moves` 规划引用改写。
///
/// Three things the walk decides per file, and gets wrong if it treats the package as
/// one crate: whether `crate::…` in this file names the tree the move happens in,
/// whether a bare `control::…` does, and whether the file can only reach the module
/// through the crate's own name. `src/lib.rs` answers yes/yes/yes, a module under
/// `src/` answers yes/no/yes, and `tests/`, `examples/`, `benches/`, `build.rs` and a
/// second binary target answer no/no/yes — rewriting `crate::…` there would edit a
/// different crate that merely shares a directory.
/// 遍历对每个文件裁定三件事，而把整包当成一个 crate 就会裁错：本文件里的 `crate::…` 是否命名搬动所在的
/// 那棵树、裸 `control::…` 是否命名它、以及该文件是否只能经 crate 自己的名字到达那个模块。`src/lib.rs`
/// 的回答是 是/是/是，`src/` 下的模块文件是 是/否/是，而 `tests/`、`examples/`、`benches/`、`build.rs`
/// 与第二个二进制目标是 否/否/是——在那里改写 `crate::…` 会去动一个只是共享目录的另一个 crate。
///
/// `scan_root` is the directory the build walks; `identity_base` is the base a face's
/// identity path is relative to. They differ for a library target outside `src/`, which
/// is why the walk and the identity arithmetic use two roots.
/// `scan_root` 是构建遍历的目录；`identity_base` 是面的身份路径所相对的基准。对 `src/` 之外的库目标两者
/// 不同，这正是遍历与身份运算使用两个根的原因。
pub(super) fn plan(
    scope: &Scope<'_>,
    moves: &[ModuleMove],
    from_package: &str,
    to_package: &str,
) -> Result<Plan, String> {
    let mut files = Vec::new();
    let mut unresolved = Vec::new();
    let mut skipped = Vec::new();
    let mut faceless = Vec::new();
    // The plan is computed from **package** directories; a file under the identity base gets the
    // identity spelling of the same pair, derived once here instead of from one boolean.
    // 计划按**包内**目录计算；身份基准之下的文件拿到同一对目录的身份拼写，在这里一次推出，而不是从一个
    // 布尔值去猜。
    let subtree = scope.package_root.join(from_package);
    let mut visited = BTreeSet::new();
    let owner_root = owning_crate_root(scope.scan_root);
    let identity_prefix = relative_prefix(scope.identity_base, scope.package_root);
    let identity_form = |package: &str| -> Option<String> {
        if identity_prefix.is_empty() {
            return Some(package.to_owned());
        }
        package
            .strip_prefix(&format!("{identity_prefix}/"))
            .map(str::to_owned)
    };
    let from_identity = identity_form(from_package);
    let to_identity = identity_form(to_package);
    let mut stack = vec![scope.package_root.to_path_buf()];
    while let Some(directory) = stack.pop() {
        if !visited.insert(canonical_or(&directory)) {
            continue;
        }
        let entries = std::fs::read_dir(&directory).map_err(|error| {
            format!(
                "cannot scan {}: {error}; no reference under it was checked",
                directory.display()
            )
        })?;
        for entry in entries {
            let entry = match entry {
                Ok(entry) => entry,
                Err(error) => {
                    unresolved.push(format!(
                        "{}: one directory entry could not be read ({error}), so nothing under it \
                         was checked",
                        directory.display()
                    ));
                    continue;
                }
            };
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().into_owned();
            let kind = match entry.file_type() {
                Ok(kind) => kind,
                Err(error) => {
                    unresolved.push(format!(
                        "{}: its kind could not be read ({error}), so it was not checked",
                        path.display()
                    ));
                    continue;
                }
            };
            // A symlink is followed only while it stays inside the package, and every
            // directory is entered once: a link back to an ancestor would otherwise make
            // the walk unbounded, and a link out of the package would make this move
            // rewrite somebody else's sources.
            // 符号链接只在仍留在包内时跟随，且每个目录只进入一次：指回祖先的链接会让遍历无界，而指向包外的
            // 链接会让这次搬动去改写别人的源码。
            let resolved = if kind.is_symlink() {
                match std::fs::canonicalize(&path) {
                    Ok(target) => {
                        if !target.starts_with(scope.package_root) {
                            skipped.push(format!(
                                "{} → {} leaves the package and was not scanned",
                                path.display(),
                                target.display()
                            ));
                            continue;
                        }
                        target
                    }
                    Err(error) => {
                        unresolved.push(format!(
                            "{}: a symlink that cannot be resolved ({error}) was not checked",
                            path.display()
                        ));
                        continue;
                    }
                }
            } else {
                path.clone()
            };
            if resolved.is_dir() {
                if ["target", ".git", ".xirang", "node_modules"]
                    .iter()
                    .any(|skip| name == *skip)
                {
                    continue;
                }
                // A nested manifest is a nested package: its files are not this
                // package's to rewrite, whatever directory they sit in.
                // 嵌套清单就是嵌套的包：无论它坐在哪个目录，它的文件都不归这个包改写。
                if resolved != scope.package_root && resolved.join("Cargo.toml").is_file() {
                    skipped.push(format!(
                        "{} is a separate package and was not scanned",
                        relative_to(&resolved, scope.package_root)
                    ));
                    continue;
                }
                stack.push(resolved);
                continue;
            }
            if path.extension().and_then(|extension| extension.to_str()) != Some("rs") {
                continue;
            }
            // A package's second binary target (`src/main.rs` beside a library, anything
            // under `src/bin/`) is its own crate: `crate::…` there names that binary, so
            // only the owning crate's modules and its root keep the `crate::` spelling.
            // 包的第二个二进制目标（与库并存的 `src/main.rs`、`src/bin/` 下的一切）自成一个 crate：
            // 那里的 `crate::…` 命名的是那个二进制，因此只有属主 crate 的模块与它的根保留 `crate::` 拼法。
            let second_binary = resolved != owner_root
                && (resolved == scope.scan_root.join("main.rs")
                    || resolved.starts_with(scope.scan_root.join("bin")));
            let crate_root = resolved == owner_root;
            let inside_crate = resolved.starts_with(scope.scan_root) && !second_binary;
            let context = PathContext {
                crate_root: inside_crate,
                bare_root: crate_root,
                crate_name: scope.crate_name.map(str::to_owned),
            };
            let identity = relative_to(&resolved, scope.identity_base);
            let file_dir = directory_of(&identity);
            // The include planner works in identity coordinates; a file that has none is
            // described in the package's, together with the move's directories.
            // include 规划器工作在身份坐标系里；没有身份坐标的文件用包的坐标系描述，搬动目录一起换算。
            let package_file_dir = directory_of(&relative_to(&resolved, scope.package_root));
            let (file_dir, from_dir, to_dir) = match (
                resolved.starts_with(scope.identity_base),
                &from_identity,
                &to_identity,
            ) {
                (true, Some(from), Some(to)) => (file_dir, from.clone(), to.clone()),
                _ => (
                    package_file_dir,
                    from_package.to_owned(),
                    to_package.to_owned(),
                ),
            };
            let Ok(text) = std::fs::read_to_string(&resolved) else {
                unresolved.push(format!(
                    "{}: is not readable, so its references were not checked",
                    path.display()
                ));
                continue;
            };
            let mut edits = Vec::new();
            match plan_reference_rewrites_in(&text, moves, &context) {
                Ok(plan) => {
                    edits.extend(plan.edits);
                    unresolved.extend(
                        plan.unresolved
                            .into_iter()
                            .map(|reason| format!("{identity}: {reason}")),
                    );
                }
                Err(error) => unresolved.push(format!(
                    "{identity}: could not be parsed ({error}), so its references were not checked"
                )),
            }
            if from_dir != to_dir {
                match plan_include_rewrites(&text, &file_dir, &from_dir, &to_dir) {
                    Ok(plan) => {
                        edits.extend(plan.edits);
                        unresolved.extend(
                            plan.unresolved
                                .into_iter()
                                .map(|reason| format!("{identity}: {reason}")),
                        );
                    }
                    Err(error) => unresolved.push(format!(
                        "{identity}: could not be parsed ({error}), so its includes were not checked"
                    )),
                }
            }
            if !from_dir.is_empty() && resolved.starts_with(&subtree) {
                faceless.push(identity.clone());
            }
            if edits.is_empty() {
                continue;
            }
            edits.sort_by_key(|edit| (edit.start, edit.end));
            files.push(FilePlan {
                file: resolved,
                before: text,
                edits,
            });
        }
    }
    files.sort_by(|left, right| left.file.cmp(&right.file));
    unresolved.sort();
    unresolved.dedup();
    skipped.sort();
    skipped.dedup();
    Ok(Plan {
        files,
        unresolved,
        skipped,
        faceless,
    })
}

/// The crate root of the crate a move's module tree lives in.
/// 搬动的模块树所在 crate 的根文件。
///
/// A package with both `lib.rs` and `main.rs` has two crates; the faces live in the
/// library, so `main.rs` is a consumer of it rather than part of it.
/// 同时有 `lib.rs` 与 `main.rs` 的包有两个 crate；注册面住在库里，因此 `main.rs` 是它的消费者而不是它的
/// 一部分。
fn owning_crate_root(scan_root: &Path) -> PathBuf {
    let library = scan_root.join("lib.rs");
    if library.is_file() {
        return library;
    }
    scan_root.join("main.rs")
}

/// The package-relative spelling of an identity base (`src` for the usual layout).
/// 身份基准相对包的拼法（常见布局下是 `src`）。
fn relative_prefix(identity_base: &Path, package_root: &Path) -> String {
    let relative = relative_to(identity_base, package_root);
    if relative.is_empty() {
        String::new()
    } else {
        format!("{relative}/")
    }
}

/// One path as a `/`-joined string relative to `base`, or the path itself when it is
/// not under `base`.
/// 一条路径相对 `base` 的 `/` 连接字符串；不在 `base` 之下时就是路径本身。
fn relative_to(path: &Path, base: &Path) -> String {
    path.strip_prefix(base)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

/// The directory part of a relative path.
/// 相对路径的目录部分。
fn directory_of(path: &str) -> String {
    match path.rsplit_once('/') {
        Some((directory, _)) => directory.to_owned(),
        None => String::new(),
    }
}

/// A path's canonical form, or the path itself when it cannot be resolved.
/// 路径的规范形式；解析不了时就是原路径。
fn canonical_or(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

/// The package's crate name, read narrowly from its manifest.
/// 包自己的 crate 名，从其清单里窄读。
///
/// The read is one key in one table, the same shape [`crate::build_method::source_layout`]
/// uses and for the same reason: spawning `cargo metadata` inside a preview copy would
/// contend for the build lock and could resolve a different manifest. Hyphens become
/// underscores because that is how Rust spells a crate name in a path.
/// 这次读取是一张表里的一个键，形状与 [`crate::build_method::source_layout`] 相同、理由也相同：在预览
/// 副本里 spawn `cargo metadata` 会去争构建锁，还可能解析到别的清单。连字符换成下划线，因为 Rust 在路径
/// 里就是这么拼 crate 名的。
pub(super) fn manifest_crate_name(package_root: &Path) -> Option<String> {
    let text = std::fs::read_to_string(package_root.join("Cargo.toml")).ok()?;
    let mut in_package = false;
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_package = line.trim_matches(['[', ']', ' '].as_slice()) == "package";
            continue;
        }
        if !in_package {
            continue;
        }
        let Some(rest) = line.strip_prefix("name") else {
            continue;
        };
        let Some(value) = rest.trim_start().strip_prefix('=') else {
            continue;
        };
        let value = value.trim();
        if let Some(name) = value
            .strip_prefix('"')
            .and_then(|rest| rest.split('"').next())
            && !name.is_empty()
        {
            return Some(name.replace('-', "_"));
        }
    }
    None
}

/// Apply a file plan to its recorded bytes, checking the binding first.
/// 把文件计划应用到它记录的字节上，先核对绑定。
pub(super) fn bound_text(plan: &FilePlan, current: &str) -> Result<String, String> {
    if current != plan.before {
        return Err(format!(
            "needs re-plan: {} changed after the move plan was computed; preview and apply again",
            plan.file.display()
        ));
    }
    apply_text_edits_checked(current, &plan.edits).map_err(|error| {
        format!(
            "{}: the planned rewrite does not fit its own text ({error}); nothing was written",
            plan.file.display()
        )
    })
}

/// One face's identity before and after the move, for the migration map.
/// 一个面在搬动前后的身份，供迁移映射使用。
///
/// `NodeId` hashes the source path, so every entry here is a new id: the map is what
/// the next reader (or W5's lineage) uses instead of guessing a record still resolves.
/// `NodeId` 哈希源码路径，因此这里每一项都是新 id：这张映射就是下一个读者（或 W5 的血缘）所用的东西，
/// 而不是猜某条记录仍然解析。
pub(super) struct FaceIdentity {
    /// Logical path before the move.
    /// 搬动前的逻辑路径。
    pub old_path: String,
    /// Logical path after the move.
    /// 搬动后的逻辑路径。
    pub new_path: String,
    /// Identity before the move.
    /// 搬动前的身份。
    pub old_id: xirang_kernel::identity::NodeId,
    /// Identity after the move.
    /// 搬动后的身份。
    pub new_id: xirang_kernel::identity::NodeId,
    /// The package-relative source the face came from.
    /// 面原来的包内相对源码。
    pub old_source: String,
    /// The package-relative source it goes to.
    /// 它要去往的包内相对源码。
    pub new_source: String,
}

/// Old and new identity for every face in the moved subtree, root included.
/// 被搬子树中每个面的旧、新身份，含根。
///
/// A face counts as moved when its source lives **under the moved directory**,
/// whatever parent it declares: the physical directory is what changes the module
/// path, so a file whose declaration points elsewhere is still a moved face.
/// 一个面只要源码住在**被搬目录**之下就算被搬，无论它声明什么父级：改变模块路径的是物理目录，因此
/// 声明指向别处的文件仍然是搬动过的面。
pub(super) fn moved_faces(
    namespace: &str,
    layout: &crate::source_layout::SourceLayout,
    views: &[FaceView],
    face: &FaceView,
    moved: &Moved,
    new_root_path: &str,
) -> Vec<FaceIdentity> {
    let mut identities = Vec::new();
    let subtree = format!("{}/", face.path);
    for view in views.iter() {
        // Two kinds of descendant, and both follow the object:
        // * **physical** — the file lives under the moved directory, so its source, module and
        //   identity all change;
        // * **logical** — a nested override lives elsewhere but its parent chain moved, so only its
        //   logical path changes. The ledger records logical paths, so leaving it out left an entry
        //   describing a position the tree no longer reported.
        // 两种后代，而两者都跟随对象：**物理**后代文件在搬动目录下，源码、模块与身份全变；**逻辑**后代
        // （嵌套覆盖）文件在别处，但父链搬了，因此只有逻辑路径变。台账记录逻辑路径，漏掉它就会留下一条
        // 描述着树已不再报告的位置的条目。
        let physical = view.id == face.id || Path::new(&view.source).starts_with(&moved.from_dir);
        let logical = view.path == face.path || view.path.starts_with(&subtree);
        if !physical && !logical {
            continue;
        }
        let old_package = xirang_multilang::source_ref::package_relative(
            view.source_base,
            &layout.package_root,
            &layout.identity_base,
            &view.source,
        );
        let new_package = if view.id == face.id {
            moved.to_physical.clone()
        } else if physical {
            let old_dir = xirang_multilang::source_ref::package_relative(
                view.source_base,
                &layout.package_root,
                &layout.identity_base,
                &view.source,
            );
            let directory = super::directory_of(&old_dir);
            let rest = directory
                .strip_prefix(&moved.from_package)
                .map(|rest| rest.trim_start_matches('/'))
                .unwrap_or_default();
            let file = Path::new(&view.source)
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or_default();
            if rest.is_empty() {
                format!("{}/{}", moved.to_package, file)
            } else {
                format!("{}/{}/{}", moved.to_package, rest, file)
            }
        } else {
            old_package.clone()
        };
        let new_source = xirang_multilang::source_ref::identity_spelling(
            &layout.package_root,
            &layout.identity_base,
            &new_package,
        );
        let new_path = if view.id == face.id {
            new_root_path.to_owned()
        } else {
            view.path
                .strip_prefix(&face.path)
                .filter(|rest| rest.is_empty() || rest.starts_with('/'))
                .map_or_else(
                    || view.path.clone(),
                    |rest| format!("{new_root_path}{rest}"),
                )
        };
        identities.push(FaceIdentity {
            old_path: view.path.clone(),
            new_path,
            old_id: view.id,
            new_id: xirang_kernel::identity::NodeId::from_namespaced_path(
                namespace,
                &new_source,
                &view.kind,
            ),
            old_source: old_package,
            new_source: new_package,
        });
    }
    identities
}

/// The migration-map and obligation lines a move reply appends.
/// 搬动回复追加的迁移映射与义务行。
///
/// Every moved face is listed, not only the root: a record keyed by any of them stops
/// resolving, so the map is the recoverable set. The boundary note is part of the
/// answer, because an empty rewrite list is not proof that every consumer moved.
/// 列出每一个被搬的面，而不只是根：以其中任何一个为键的记录都不再解析，因此这张映射就是可恢复集。
/// 边界说明是答案的一部分，因为空的改写清单并不能证明每个消费者都已迁移。
pub(super) fn describe(identities: &[FaceIdentity], plan: &Plan) -> String {
    let files = plan.files.len();
    let unresolved = &plan.unresolved;
    let skipped = &plan.skipped;
    let mut message = String::new();
    message.push_str(&format!(
        "       subtree ({} moved faces):\n",
        identities.len()
    ));
    for face in identities.iter().take(40) {
        message.push_str(&format!(
            "         {} → {}  {} → {}\n",
            face.old_path, face.new_path, face.old_id, face.new_id
        ));
    }
    if identities.len() > 40 {
        message.push_str(&format!(
            "         {} more moved faces omitted; the mapping above is the recoverable set\n",
            identities.len() - 40
        ));
    }
    if files > 0 {
        message.push_str(&format!("       references: {files} file(s) rewritten\n"));
    }
    if !plan.faceless.is_empty() {
        message.push_str(&format!(
            "       {} `.rs` file(s) move as bytes with no identity of their own (no record is \
             keyed by them): {}\n",
            plan.faceless.len(),
            plan.faceless.first().map_or("", String::as_str)
        ));
    }
    if !unresolved.is_empty() {
        message.push_str("       unresolved obligations:\n");
        for obligation in unresolved.iter().take(40) {
            message.push_str(&format!("         {obligation}\n"));
        }
        if unresolved.len() > 40 {
            message.push_str(&format!(
                "         {} more unresolved references omitted\n",
                unresolved.len() - 40
            ));
        }
    }
    message.push_str(
        "       reference scan boundary: this package's own `.rs` files are read — `src/`, \
         `tests/`, `examples/`, `benches/`, `build.rs` and custom target roots — while paths built \
         by macros, selected by `cfg`, or produced by `include!`/`concat!` remain outstanding \
         obligations and are not claimed fixed; `crate::…` is only rewritten inside the crate the \
         faced tree lives in, and consumers are reached through the crate's own name\n",
    );
    if !skipped.is_empty() {
        message.push_str("       not scanned (outside this move's ownership):\n");
        for entry in skipped.iter().take(20) {
            message.push_str(&format!("         {entry}\n"));
        }
        if skipped.len() > 20 {
            message.push_str(&format!(
                "         {} more not scanned\n",
                skipped.len() - 20
            ));
        }
    }
    message
}

#[cfg(test)]
#[path = "apply_move_references_tests.rs"]
mod apply_move_references_tests;

#[cfg(test)]
#[path = "apply_move_structural_tests.rs"]
mod apply_move_structural_tests;
