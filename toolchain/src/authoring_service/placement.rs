//! Where a new face goes when it leaves the default layout: the island that owns a directory, and
//! the ledger entry the deviation needs.
//! 新注册面离开默认布局时去哪里：拥有某个目录的孤岛，以及这次偏离所需要的台账条目。
//!
//! Split out of `service.rs` to keep that file inside the repository's size ratchet, and because
//! these two questions are one subject: a placement is either the default (nothing is recorded) or a
//! deviation (the object and its directory are recorded, and an `add` that names only a place can
//! find its parent again).
//! 从 `service.rs` 拆出来，使那个文件留在仓库的尺寸棘轮之内，也因为这实际上是同一个题目：一次落点要么是
//! 默认的（什么都不记），要么是一次偏离（对象与目录被记下，而一个只点名了地点的 `add` 能再找回它的父级）。

use std::path::Path;

use xirang_kernel::authoring::{FieldUpdate, RegistrationField, RegistrationUpdate};
use xirang_kernel::identity::{NodeId, StableFaceId};
use xirang_multilang::object_map::{LEDGER_FILE, ObjectMap, Observed};

use crate::object_ledger::{self, LedgerWrite};
use crate::source_layout::SourceLayout;

/// The directory a Rust face's file must live in, from the destination a request named.
/// Rust 面文件必须所在的目录，从请求点名的目的地算出。
///
/// Rust has no "the file lives anywhere and the module is declared" form: the file **is** the module
/// body, so its path decides its module. That is a fact about the compiler, not a policy this tool
/// gets to waive — what the tool owns is the *rule*, and the rule is stated here once:
///
/// * the file is a `.rs` file under the source tree the build reads;
/// * its stem **is** the module's last segment, and its directory is the module's parent chain.
///
/// A destination that does not satisfy that is refused **by name**, with the spelling that would:
/// the old blanket "a Rust face's directory is its module path" told a reader what was impossible
/// and never what to write instead.
/// Rust 没有"文件随便放、模块另声明"的形态：文件**就是**模块体，因此它的路径决定它的模块。那是编译器的事实，
/// 不是本工具可以豁免的策略——本工具拥有的是那条**规则**，而规则在这里只写一次：
///
/// * 文件是构建所读源码树下的一个 `.rs` 文件；
/// * 它的 stem **就是**模块路径的末段，它的目录就是模块的父链。
///
/// 不满足它的目的地会被**按名**拒绝，并给出能满足它的拼写：过去那句笼统的"Rust 面的目录就是它的模块路径"
/// 只告诉读者什么不可能，从不说什么该写。
pub(crate) fn rust_explicit_directory(
    layout: &SourceLayout,
    destination: &str,
    module: &str,
) -> Result<(std::path::PathBuf, xirang_multilang::source_ref::SourceBase), String> {
    let relative = Path::new(destination);
    if relative.is_absolute()
        || relative
            .components()
            .any(|component| component.as_os_str() == "..")
    {
        return Err(format!(
            "`{destination}` is not a package-relative path; a Rust face is written inside this package"
        ));
    }
    if relative
        .extension()
        .and_then(|extension| extension.to_str())
        != Some("rs")
    {
        return Err(format!(
            "`{destination}` is not a Rust source file: a Rust face is a `.rs` file, and `{module}` would be written `{module}.rs`"
        ));
    }
    let source = layout.package_root.join(relative);
    let Some(directory) = source.parent() else {
        return Err(format!("`{destination}` has no directory"));
    };
    // The directory the face's module will be, relative to the base the build takes identity paths
    // from. This is the same space `plan_new_face` joins the file name onto, so the file the plan
    // writes is the file the request named — not a second derivation of it.
    // 这个面的模块将成为的目录，相对构建取得身份路径的那个基准。它与 `plan_new_face` 拼接文件名的空间
    // 相同，因此计划写下的文件就是请求点名的文件——不是它的第二次推导。
    // Two bases, one decision: under the identity base the historical spelling is the relative
    // path; outside it (inside the package) the path is package-relative and the generated tree
    // mounts it with `#[path]`. Which base it is, is data — never "does the file exist".
    // 两个基准、一个决定：身份基准之下用历史相对拼写；之外（仍在包内）用包内相对，生成树用
    // `#[path]` 挂载它。哪个基准是数据——绝不是"文件是否存在"。
    let (directory_relative, base) = match directory.strip_prefix(&layout.identity_base) {
        Ok(relative) => (
            relative.to_path_buf(),
            xirang_multilang::source_ref::SourceBase::Identity,
        ),
        Err(_) => match directory.strip_prefix(&layout.package_root) {
            Ok(relative) => (
                relative.to_path_buf(),
                xirang_multilang::source_ref::SourceBase::Package,
            ),
            Err(_) => {
                return Err(format!(
                    "`{destination}` is outside this package; a Rust face may live anywhere inside \
                     it, and nowhere outside without an authorized source"
                ));
            }
        },
    };
    let stem = relative
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("");
    let leaf = directory
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("");
    // The module's last segment is the file's stem; the directory chain above it is the parent
    // chain. A root-level file has no leaf directory, and its stem is the whole module.
    // 模块的末段是文件的 stem；它上面的目录链就是父链。根级文件没有叶子目录，它的 stem 就是整个模块。
    let expected = if leaf.is_empty() { stem } else { leaf };
    if stem != expected {
        return Err(format!(
            "`{destination}` cannot be the module `{module}`: a Rust file is its module's body, so its **stem** names the module — write `{expected}/{expected}.rs` (or a directory tree ending in it) instead of `{destination}`"
        ));
    }
    if module != expected {
        return Err(format!(
            "`{destination}` would declare `{expected}`, not `{module}`: for Rust the directory chain **is** the module path, so the last segment and the file stem both name the module. Either name the module `{expected}`, or name the destination `{module}/{module}.rs` under the directory you wanted"
        ));
    }
    Ok((directory_relative, base))
}

/// The object a `destination` directory belongs to, when the caller named no parent.
/// 当调用方没有点名父级时，`destination` 所在目录属于哪个对象。
///
/// This is the island half of the mapping: an `add` inside a directory that exactly one object
/// mapped may leave `parent` out, and the object becomes the parent. A directory two objects claim
/// is an error the ledger already words, and a mapped path this tree does not have is refused rather
/// than quietly falling back to the root — falling back would author a child under a parent the
/// author never named.
/// 这是映射的孤岛那一半：在一个恰好由一个对象映射的目录里 `add`，可以省略 `parent`，那个对象就成为
/// 父级。两个对象都声称的目录是台账已经写好措辞的错误，而这棵树里没有的映射路径会被拒绝，而不是悄悄回退
/// 到根——回退会创做出一个父级作者从未点名的子面。
pub(crate) fn island_parent(
    root: &Path,
    namespace: &str,
    destination: Option<&str>,
) -> Result<Option<NodeId>, String> {
    let Some(destination) = destination else {
        return Ok(None);
    };
    let Some(directory) = Path::new(destination).parent() else {
        return Ok(None);
    };
    let directory = directory.to_string_lossy().replace('\\', "/");
    if directory.is_empty() {
        return Ok(None);
    }
    let map = ObjectMap::read_from(root)?;
    let Some(owner) = map.owner_of_directory(&directory)? else {
        return Ok(None);
    };
    let owner_path = owner.path.clone();
    let found = crate::build_method::face_views(root, namespace)?
        .into_iter()
        .find(|view| view.path == owner_path)
        .map(|view| view.id);
    match found {
        Some(id) => Ok(Some(id)),
        None => Err(format!(
            "`{directory}` is mapped to `{owner_path}` in {LEDGER_FILE}, and this tree has no face \
             at that path; repair the mapping or name the parent explicitly"
        )),
    }
}

/// Plan the ledger entry one deviation from the default layout needs.
/// 规划一次偏离默认布局所需要的台账条目。
///
/// The leaf name is the declaration's `kind`: that is what a foreign adapter writes into
/// `.named(…)`, and the request vocabulary has no separate `registry_name` field (a registry name is
/// derived from the declaration, not asked for).
/// 末段名是声明的 `kind`：外语适配器写进 `.named(…)` 的就是它，而请求词汇里没有单独的 `registry_name`
/// 字段（注册名由声明推导，不由请求询问）。
pub(crate) fn record_deviation(
    root: &Path,
    namespace: &str,
    parent_id: NodeId,
    leaf: &str,
    destination: &str,
    registration: &RegistrationUpdate,
) -> Result<Option<LedgerWrite>, String> {
    // The parent's **derived** path, from the same view every consumer reads. The kernel registry's
    // own `path_for` disagrees with it for a face whose registry name is not its module's last
    // segment (a C++ `.named(…)`), and a ledger keyed by the wrong path would derive a different
    // stable id than the tree.
    // 父级的**推导**路径，来自每个消费者都读的那份视图。对注册名不是模块末段的面（C++ 的
    // `.named(…)`），内核注册机自己的 `path_for` 与它不一致，而用错路径做键的台账会推导出与树不同的
    // 稳定 id。
    // The registry root is a legitimate parent even though no **face** reports it: its logical
    // path is the one the tree starts every chain at, and a foreign face mounted there needs a
    // deviation entry keyed on it like any other parent.
    // 注册树根是合法父级，尽管没有**面**报告它：它的逻辑路径是树开始每条链的地方，而挂在那里的外语面
    // 需要一条以它为键的偏离条目，与其它父级相同。
    let root_id = xirang_kernel::identity::root_node_id(namespace);
    let parent_path = if parent_id == root_id || parent_id == xirang_kernel::identity::ROOT_NODE_ID
    {
        "root".to_owned()
    } else {
        crate::build_method::face_views(root, namespace)?
            .into_iter()
            .find(|view| view.id == parent_id)
            .map(|view| view.path)
            .ok_or_else(|| {
                format!(
                    "no face in this tree has the identity {parent_id}; the parent a placement hangs \
                     under must be one the tree reports"
                )
            })?
    };
    // The leaf is the name the **tree** will give this declaration, and it is not the same rule for
    // every language: a foreign adapter writes `.named(kind)`, while a Rust declaration's registry
    // name defaults to its module's last segment. Taking the kind for both would key a Rust
    // deviation on a path the tree never reports, and island inheritance would then fail to find its
    // parent. The caller owns that fact because the caller resolved the language.
    // 末段是**树**将给这条声明的名字，而它并非每种语言同一条规则：外语适配器写 `.named(kind)`，而 Rust
    // 声明的注册名默认取模块末段。两者都用 kind，会把一次 Rust 偏离键在树从不报告的路径上，孤岛继承
    // 于是找不到父级。这个事实由调用方拥有，因为语言是调用方解析的。
    let path = format!("{parent_path}/{leaf}");
    let stable = match registration.find_field(RegistrationField::StableName) {
        FieldUpdate::Set(name) => StableFaceId::from_name(name),
        _ => StableFaceId::from_namespaced_name(namespace, &path),
    };
    let observed = Observed::new(stable, path, destination.to_owned());
    let directory = Path::new(destination)
        .parent()
        .map(|parent| parent.to_string_lossy().replace('\\', "/"))
        .unwrap_or_default();
    object_ledger::plan_observation(root, &observed, None, Some(directory))
}
