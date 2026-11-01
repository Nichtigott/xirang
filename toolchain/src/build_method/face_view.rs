//! Read-only registration-face view for operator commands.
//! 面向操作命令的只读注册面视图。
//!
//! `xirang explain` has to answer "what is this node, and why is it (not)
//! shipped" for a host it cannot link. It therefore reads the same discovery
//! tree, the same face parser, and the same parent/identity rules the build
//! uses, and never re-implements them: this module is the one place a command
//! turns a package root into per-face rows. The scope/pruning manifest readers
//! live in `scope_view`, mounted below and re-exported so the public
//! `face_view::…` paths are unchanged.
//! `xirang explain` 必须为它无法链接的宿主回答"这个节点是什么、为什么（没）被发布"。
//! 因此它读的是构建使用的同一棵发现树、同一个注册面解析器与同一套父级/身份规则，
//! 绝不重新实现：本模块是命令把包根变成逐面行数据的唯一位置。作用域/修剪清单读取方
//! 位于 `scope_view`，在下方挂载并重新导出，因此公开的 `face_view::…` 路径保持不变。
//!
//! Read-only: nothing here writes, and nothing here consults process-wide build
//! state such as a pinned namespace — `package` is passed in, so two fixtures in
//! one process cannot contaminate each other's identities.
//! 只读：这里不写任何东西，也不查询进程级的构建状态（例如被固定的命名空间）——
//! `package` 由调用方传入，因此同一进程里的两个夹具不会污染彼此的身份。

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use xirang_kernel::lexicon;

use super::registry_identity::NodeId;
use super::registry_syntax::{FaceSyntax, ParentSyntax};
use super::source_ref::{SourceBase, SourceRef, SourceSnapshot, locate, package_relative};
use super::{Node, SourceLayout, parsed_face};

// Split decision: the discovery walk (this page) and the published-manifest
// readers (`scope_view`) change for different reasons — one follows source and
// parent rules, the other a frozen TSV layout — so they are separate files. The
// re-export keeps `crate::build_method::face_view::{BuildScopeView, PruningRow,
// read_build_scope, read_pruning_manifest}` and the root whitelist in `lib.rs`
// byte-compatible for every caller.
// 拆分决定：发现遍历（本页）与已发布清单读取方（`scope_view`）因不同原因变化——
// 一个跟随源码与父级规则，另一个跟随冻结的 TSV 版式——因此分成两个文件。重新导出
// 使 `crate::build_method::face_view::{BuildScopeView, PruningRow, read_build_scope,
// read_pruning_manifest}` 与 `lib.rs` 的根部白名单对所有调用方逐字节兼容。
#[path = "scope_view.rs"]
mod scope_view;

pub use self::scope_view::{
    BuildScopeView, FileRow, PruningRow, ShapeRow, build_output_is_current, read_build_scope,
    read_file_manifest, read_pruning_manifest, read_shape_manifest,
};

/// One registration face as the build's own discovery sees it.
/// 构建自身的发现过程看到的一个注册面。
///
/// `path` is the logical registry path a runtime tree would report, derived from
/// the `registry_name` chain; `source` is relative to the package's `src/`, so a
/// caller can line these rows up with `source_scope.tsv` and
/// `pruning_manifest.tsv`, which use the same spelling.
/// `path` 是运行期树会报告的逻辑注册路径，由 `registry_name` 链推导；`source` 相对包
/// 的 `src/`，因此调用方可以与 `source_scope.tsv`、`pruning_manifest.tsv` 对齐——
/// 它们用的是同一种写法。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FaceView {
    /// The face's source-instance identity.
    /// 该面的源码实例身份。
    pub id: NodeId,
    /// The `stable_name` the declaration opted into, when it declared one. It is the author-owned
    /// half of the persistent object identity; a face without it derives one from its logical
    /// registration path. See [`Self::object_id`].
    /// 声明显式选择的 `stable_name`（当它声明了时）。它是持续对象身份里作者拥有的那一半；没有它的面
    /// 从逻辑注册路径推导一个。见 [`Self::object_id`]。
    pub stable_name: Option<String>,
    /// The resolved parent registry's identity.
    /// 已解析的父注册机身份。
    pub parent: NodeId,
    /// The declared `kind`, which is one of the identity hash inputs.
    /// 声明的 `kind`，它是身份散列的输入之一。
    pub kind: String,
    /// The registry name under the parent, from the declaration or the macro
    /// default. It is not an overlay slot: `slot` is the graft cut a
    /// `static_graft_plan!` targets, and the plugin host's plugin slot.
    /// 父级下的注册面名，来自声明或宏默认值。它不是叠加层的槽位：`slot` 指
    /// `static_graft_plan!` 针对的 graft 切口，以及插件宿主的插件槽。
    pub registry_name: String,
    /// The Rust module path the source declares.
    /// 源码声明的 Rust 模块路径。
    pub module: String,
    /// The source spelling the record carries. Its base is [`Self::source_base`]; this spelling is
    /// also an identity input, so normalization never rewrites it.
    /// 记录携带的源码拼写。基准见 [`Self::source_base`]；它同时是身份输入，因此规范化绝不改写它。
    pub source: String,
    /// The base [`Self::source`] is relative to. Data from the record's own format, never guessed.
    /// [`Self::source`] 所相对的基准。来自记录自身格式的数据，绝不靠猜。
    pub source_base: SourceBase,
    /// Whether the face owns a child registry.
    /// 该面是否拥有子注册机。
    pub owns_registry: bool,
    /// The logical registry path a runtime tree would report.
    /// 运行期树会报告的逻辑注册路径。
    pub path: String,
    /// Whether the face resolved a parent the current tree knows. A face whose
    /// parent cannot be resolved is still returned, because an operator asking
    /// "what is this node" must get an answer that names the broken parent
    /// instead of a command that refuses to speak.
    /// 该面是否解析到了一个当前树认识的父级。父级解析不出来的面仍会返回，因为操作者
    /// 问"这个节点是什么"时必须得到一个指明坏父级的回答，而不是一条拒绝说话的命令。
    pub parent_resolved: bool,
}

/// A face before its logical path is known, collected so parent lookup can see
/// every face at once.
/// 逻辑路径尚不可知时的注册面；先全部收集，父级查找才能一次看到所有面。
struct RawFace {
    id: NodeId,
    stable_name: Option<String>,
    parent: ParentSpec,
    kind: String,
    registry_name: String,
    module: String,
    source: String,
    /// The base `source` is written against (data from the discovery fact).
    /// `source` 所相对的基准（来自发现事实的数据）。
    source_base: SourceBase,
    owns_registry: bool,
}

/// The parent a declaration names, kept unresolved until every face's module is
/// known.
/// 声明命名的父级；在知道每个面的模块之前保持未解析。
///
/// `pub(crate)` because the pruning record publishes the **resolved** parent identity (audit
/// `W3-1b`), and a second spelling of "what does this declaration name" written in the manifest
/// writer is exactly the copy that drifts.
/// `pub(crate)` 是因为剪枝记录要发布**解析后**的父级身份（审计 `W3-1b`），而在清单写入方里另写一份
/// "这条声明命名了什么"正是那种会漂移的拷贝。
pub(crate) enum ParentSpec {
    Root,
    FromPath {
        source: String,
        kind: String,
    },
    NodePath(String),
    /// A `parent` field exists but does not parse; the build reports this as a
    /// `static-plan` diagnostic instead of guessing.
    /// `parent` 字段存在但解析不了；构建对此报 `static-plan` 诊断，而不是猜。
    Unparsed,
}

impl FaceView {
    /// This face's source as the shared fact: object, package-relative source, snapshot.
    /// 该面的源码共享事实：对象、包内相对源码、快照。
    ///
    /// The base is data on the view, never discovered by trying which file exists. A caller that
    /// needs the file asks [`Self::locate_source`], and a missing file is an error there.
    /// 基准是视图上的数据，绝不靠"哪个文件存在"去发现。需要文件的调用方问 [`Self::locate_source`]，缺失在
    /// 那里是错误。
    pub fn source_ref(&self, layout: &SourceLayout) -> SourceRef {
        SourceRef {
            object: self.id,
            source: package_relative(
                self.source_base,
                &layout.package_root,
                &layout.identity_base,
                &self.source,
            ),
            snapshot: SourceSnapshot::Workspace,
        }
    }

    /// The persistent object identity of this face, independent of where its file sits.
    /// 该面的持续对象身份，与它的文件放在哪里无关。
    ///
    /// Two derivations, and they never disagree about one declaration:
    ///
    /// * a declaration that opted into `stable_name` keeps the value the tree already computes
    ///   ([`xirang_kernel::identity::StableFaceId::from_name`]) — this is the same identity
    ///   references use, not a second one;
    /// * every other declaration derives it from its **logical registration path** inside the
    ///   package namespace. A physical move changes the source path and the `NodeId`; it does not
    ///   change the logical path, so the object keeps this identity.
    ///
    /// **This value is the seed, not the authority.** It is derived from facts the view holds, so a
    /// consumer that must keep identity across a move asks the project's object ledger
    /// (`xirang.objects.tsv`): `ObjectMap::own` returns the entry a declaration belongs to, and its
    /// stable id wins over this derivation. Using `object_id` alone on a tree whose ledger is absent
    /// re-derives a new object for a renamed face — which is why the ledger is a project file and not
    /// a cache.
    /// **这个值是种子，不是权威。** 它由视图持有的事实推导，因此需要跨搬动保持身份的消费者要去问项目的
    /// 对象台账（`xirang.objects.tsv`）：`ObjectMap::own` 返回一条声明所属的条目，条目的稳定 id 胜过这次
    /// 推导。在一棵没有台账的树上单用 `object_id`，会给一个改过名的面重新推导出一个新对象——这正是台账是
    /// 项目文件而不是缓存的原因。
    ///
    /// 两种推导，而它们对同一条声明不会给出不同答案：
    ///
    /// * 显式选择 `stable_name` 的声明沿用树本来就算出的值
    ///   （[`xirang_kernel::identity::StableFaceId::from_name`]）——这是引用所用的同一个身份，不是第二个；
    /// * 其余声明在包命名空间内从它的**逻辑注册路径**推导。物理搬动改变源码路径与 `NodeId`，不改变逻辑
    ///   路径，因此对象保持这个身份。
    pub fn object_id(&self, namespace: &str) -> xirang_kernel::identity::StableFaceId {
        use xirang_kernel::identity::StableFaceId;
        match &self.stable_name {
            Some(name) => StableFaceId::from_name(name),
            None => StableFaceId::from_namespaced_name(namespace, &self.path),
        }
    }

    /// This face as the shared ledger sees it: object id, logical path, package-relative source.
    /// 该面在共享台账眼里的样子：对象 id、逻辑路径、包内相对源码。
    pub fn observed(
        &self,
        layout: &SourceLayout,
        namespace: &str,
    ) -> xirang_multilang::object_map::Observed {
        xirang_multilang::object_map::Observed::new(
            self.object_id(namespace),
            self.path.clone(),
            package_relative(
                self.source_base,
                &layout.package_root,
                &layout.identity_base,
                &self.source,
            ),
        )
    }

    /// The one path this face's source fact names, or an error naming the missing file.
    /// 该面源码事实点名的那一条路径；缺失时返回点名该文件的错误。
    pub fn locate_source(&self, layout: &SourceLayout) -> Result<std::path::PathBuf, String> {
        locate(&layout.package_root, &self.source_ref(layout))
    }
}

thread_local! {
    /// Faces already derived in the read-only answer this thread is serving, keyed by root and
    /// package. A workspace query reads the same member twice — once for the census header, once to
    /// annotate call-graph rows — and the walk is the whole cost of a face read.
    /// 本线程正在服务的只读答案里已经推导过的面，按根与包索引。工作区查询会把同一个成员读两次——一次给
    /// 普查抬头、一次给调用图行加注——而遍历是一次面读取的全部成本。
    static READ_ONLY_FACES: std::cell::RefCell<Option<std::collections::HashMap<(std::path::PathBuf, String), FaceRead>>> =
        const { std::cell::RefCell::new(None) };
}

/// Marks the read-only answer whose face reads may be shared. Dropping it ends the sharing.
/// 标记"面读取可以共享"的那次只读答案；drop 时结束共享。
pub struct ReadOnlyAnswer;

impl ReadOnlyAnswer {
    /// Start sharing face reads on this thread until the guard drops.
    /// 在本线程上开始共享面读取，直到 guard 被 drop。
    pub fn begin() -> Self {
        READ_ONLY_FACES.with(|cell| *cell.borrow_mut() = Some(std::collections::HashMap::new()));
        ReadOnlyAnswer
    }
}

impl Drop for ReadOnlyAnswer {
    fn drop(&mut self) {
        READ_ONLY_FACES.with(|cell| *cell.borrow_mut() = None);
    }
}

/// Every registration face under `root`'s `src/`, with the identities a build
/// with namespace `package` would compute.
/// `root` 的 `src/` 下每个注册面，身份使用命名空间 `package` 的构建会算出的值。
///
/// This is [`face_views_and_unreadable`] with the second half dropped. A caller
/// that has to say "there is a registration file I could not read" — rather than
/// answer with fewer faces and no reason — calls that one.
/// 这是 [`face_views_and_unreadable`] 丢掉后半段的结果。必须说出"有一个注册面文件我读不了"、
/// 而不是少报几个面却不说原因的调用方，应当调用那一个。
pub fn face_views(root: &Path, package: &str) -> Result<Vec<FaceView>, String> {
    Ok(face_views_and_unreadable(root, package)?.0)
}

/// Every registration face under `root`'s `src/`, plus every registration file
/// the walk could not place.
/// `root` 的 `src/` 下每个注册面，外加遍历无法安放的每个注册面文件。
///
/// The second half is why this exists. A registration file that does not parse
/// (two macro invocations, a truncated field list) is not a face, so it cannot
/// appear in the first half — and until this entry existed, dropping it was
/// silent: the caller answered "this package declares what I could list", with no
/// count and no reason, while the build reported the same file as a `face-syntax`
/// diagnostic. A file that *is* a face but sits outside the `<name>/<name>.rs`
/// layout is reported the same way, because the view drops it for the same
/// reason. Each string is `"<path relative to the package root>:<line> <reason>"`;
/// the line is `0` when the failure is about the file's place rather than its
/// text.
/// 后半段正是它存在的原因。解析不了的注册面文件（两处宏调用、字段列表被截断）不是面，因此不可能
/// 出现在前半段——而在这个入口存在之前，丢弃它是静默的：调用方回答"这个包声明的就是我能列出的
/// 这些"，没有计数也没有原因，而构建把同一个文件报成 `face-syntax` 诊断。确实是面、却长在
/// `<name>/<name>.rs` 布局之外的文件同样被报出，因为视图丢弃它的理由是同一个。每条字符串形如
/// `"<相对包根的路径>:<行号> <原因>"`；当失败关乎文件的位置而不是它的文本时，行号为 `0`。
///
/// A child whose parent file is in the second half is reported with
/// `parent_resolved == false` in the first half, which is the honest answer: the
/// chain really is broken. Its `path` still falls back to `root/<name>`, because
/// that is the only name the view has for it; a caller that must not print a
/// path the runtime never had has to check `parent_resolved` (and the second half)
/// before using it.
/// 父面文件落在后半段的子面，在前半段里以 `parent_resolved == false` 出现，这是诚实的答案：
/// 那条链确实是断的。它的 `path` 仍回退成 `root/<name>`，因为那是视图手里唯一的名字；绝不能
/// 打印运行期从未有过的路径的调用方，必须先用 `parent_resolved`（以及后半段）核对再使用它。
pub fn face_views_and_unreadable(
    root: &Path,
    package: &str,
) -> Result<(Vec<FaceView>, Vec<String>), String> {
    let (views, unreadable, _external) = face_views_with_external(root, package)?;
    Ok((views, unreadable))
}

/// What one read of the face tree found: the faces the generated tree holds, the registration
/// files it could not read, and the external faces that are deliberately not part of it.
/// 一次注册面树读取发现的东西：生成树持有的面、它读不了的注册面文件，以及有意不属于它的外部面。
pub type FaceRead = (Vec<FaceView>, Vec<String>, Vec<String>);

/// The faces a package's build-time record file declares, as `FaceView`s, when it declares one.
/// 一个包的构建期记录文件声明的面（以 `FaceView` 表示），当它声明了时。
///
/// The record file is read **without running its producer**: this is the read-only view, and a
/// missing file is reported as a sentence rather than as a failure (the build has not run yet). The
/// faces are appended to the derived tree so `check`/`registry` can list what the package declares
/// through a carrier that has no source file to walk.
/// 读取记录文件**不运行它的生产者**：这是只读视图，缺文件报成一句话而不是失败（构建还没跑）。这些面被
/// 追加进推导出的树，使 `check`/`registry` 能列出这个包经一种"没有可遍历源码文件"的载体声明的东西。
#[cfg(all(feature = "mcp", feature = "native-record"))]
pub(crate) fn record_face_views(
    root: &Path,
    package: &str,
    known: &[FaceView],
) -> Result<Option<(Vec<FaceView>, String)>, String> {
    let Some((declaration, _)) = super::record_file::context(root)? else {
        return Ok(None);
    };
    if !root.join(&declaration.file).is_file() {
        return Ok(Some((
            Vec::new(),
            format!(
                "record faces unavailable ({} is missing; run the build)",
                declaration.file
            ),
        )));
    }
    let records = super::record_file::read(root, &declaration, package)?;
    let count = records.len();
    let paths: BTreeMap<NodeId, String> = known
        .iter()
        .map(|face| (face.id, face.path.clone()))
        .collect();
    let root_id = xirang_kernel::identity::root_node_id(package);
    let mut views = Vec::with_capacity(records.len());
    for record in records {
        let registry_name = if record.registry_name.is_empty() {
            record.name.clone()
        } else {
            record.registry_name.clone()
        };
        let (path, parent_resolved) = match paths.get(&record.parent) {
            Some(parent) => (format!("{parent}/{registry_name}"), true),
            None if record.parent == root_id => (format!("root/{registry_name}"), true),
            None => (format!("root/{registry_name}"), false),
        };
        views.push(FaceView {
            id: record.id,
            stable_name: (!record.stable_name.is_empty()).then(|| record.stable_name.clone()),
            parent: record.parent,
            kind: record.name.clone(),
            registry_name,
            module: super::declared_native::module_of(&record.source),
            source: record.source.clone(),
            source_base: SourceBase::Package,
            owns_registry: record.owns_registry,
            path,
            parent_resolved,
        });
    }
    Ok(Some((
        views,
        format!("record faces {} (from {})", count, declaration.file),
    )))
}

/// The refusal a build without the record decoder gives: the record faces are simply not listed.
/// 没有记录解码器的构建给出的答复：记录面就不列出来。
#[cfg(all(feature = "mcp", not(feature = "native-record")))]
pub(crate) fn record_face_views(
    _root: &Path,
    _package: &str,
    _known: &[FaceView],
) -> Result<Option<(Vec<FaceView>, String)>, String> {
    Ok(None)
}

/// The same answer, with the external faces separated from the unreadable ones.
/// 同一个答案，只是把**外部面**与**读不了的面**分开。
///
/// They are different facts and used to share one list: a file that does not parse cannot be a
/// face, while a file written with `external_object!` is a face of its own registry that this
/// package's generated tree deliberately does not contain. A reader deciding whether a crate is
/// broken needs the difference, and a count alone made a working example look like one.
/// 它们是两个不同的事实，过去共用一张清单：解析不了的文件当不成面，而用 `external_object!` 写的文件是
/// 它自己注册机的面、本包的生成树有意不含它。要判断一个 crate 是否坏掉的读者需要这个区别，而光有计数
/// 会让一个能工作的示例看起来是坏的。
pub fn face_views_with_external(root: &Path, package: &str) -> Result<FaceRead, String> {
    let key = (root.to_path_buf(), package.to_owned());
    if let Some(hit) = READ_ONLY_FACES.with(|cell| {
        cell.borrow()
            .as_ref()
            .and_then(|cache| cache.get(&key).cloned())
    }) {
        return Ok(hit);
    }
    // The same answer, served from a record keyed by the exact bytes the walk would read: a fresh
    // process paid the whole walk again, which the workspace census measured at 8.6 s.
    // 同一个答案，从一份以遍历会读到的确切字节为键的记录里取得：新进程曾重付整趟遍历，工作区普查实测 8.6 s。
    let disk = super::face_cache::key(root, package);
    if let Some(disk) = &disk
        && let Some(hit) = super::face_cache::read(disk)
    {
        READ_ONLY_FACES.with(|cell| {
            if let Some(cache) = cell.borrow_mut().as_mut() {
                cache.insert(key, hit.clone());
            }
        });
        return Ok(hit);
    }
    let answer = face_views_uncached(root, package)?;
    READ_ONLY_FACES.with(|cell| {
        if let Some(cache) = cell.borrow_mut().as_mut() {
            cache.insert(key, answer.clone());
        }
    });
    if let Some(disk) = &disk {
        super::face_cache::write(disk, &answer);
    }
    Ok(answer)
}

/// The walk itself; `face_views_with_external` is the entry that may share its result.
/// 遍历本身；`face_views_with_external` 是可能共享它的入口。
fn face_views_uncached(root: &Path, package: &str) -> Result<FaceRead, String> {
    // The build's own layout resolution, so this read-only view reports the paths
    // and identities the build computes — including for a library target outside
    // `src/`, where the identity path keeps its leading directory component.
    // 用构建自己的布局解析，因此这个只读视图报告的路径与身份就是构建计算出的那些——包括 `src/`
    // 之外的库目标，那种情况下身份路径保留它开头的目录分量。
    let layout = super::source_layout(root)?;
    let src = &layout.scan_root;
    if !src.is_dir() {
        return Err(format!("no source tree at {}", src.display()));
    }
    // `discover_root_reporting` is the same walk the build runs, and it already
    // collects the files it cannot place; `discover_root` used to throw that list
    // away here, which is what made the drop silent.
    // `discover_root_reporting` 就是构建走的那趟遍历，它本来就收集安放不了的文件；
    // 这里过去用 `discover_root` 把那张清单扔掉了，这正是丢弃变静默的原因。
    let mut unplaced = Vec::new();
    let nodes = super::source_walk::discover_package(&layout, &mut unplaced);
    // A file that *is* the attached `<name>/<name>.rs` of a node is not "unplaced":
    // the walk placed it, and the build reports it through the same syntax rule it
    // applies to every node file. Both classes have to be in the answer, or a
    // broken face file at the expected path would still vanish without a word.
    // 确实是某个节点所附的 `<name>/<name>.rs` 的文件不属于"安放不了"：遍历已给它安排了位置，而
    // 构建经它对每个节点文件都用的同一条语法规则报告它。两类都必须进这个答案，否则长在预期位置上的
    // 坏面文件仍会一声不响地消失。
    let mut external = unplaced
        .iter()
        .filter(|face| face.phase == "face-external")
        .map(|face| format!("{}:{} {}", face.relative, face.line, face.message))
        .collect::<Vec<_>>();
    let mut unreadable = unplaced
        .iter()
        .filter(|face| face.phase != "face-external")
        .map(|face| format!("{}:{} {}", face.relative, face.line, face.message))
        .collect::<Vec<_>>();
    let syntax_errors = super::face_syntax_check::face_syntax_errors(src, &nodes);
    unreadable.extend(
        syntax_errors
            .iter()
            .map(|error| format!("{}:{} {}", error.source, error.line, error.message)),
    );
    external.extend(
        syntax_errors
            .iter()
            .filter(|error| error.phase == "face-external")
            .map(|error| format!("{}:{} {}", error.source, error.line, error.message)),
    );
    let mut raw = Vec::new();
    visit_face_view(&layout, &nodes, package, "", &mut raw);
    let mut modules = raw
        .iter()
        .map(|face| (face.module.clone(), face.id))
        .collect::<BTreeMap<_, _>>();
    // Foreign declarations are part of the same tree. Without this, a C++ face is invisible to
    // every structural operation: the move planner only sees `FaceView`s, so it cannot refuse (or
    // plan) a subtree that contains one. The identity/module rules are the build's own, reused
    // instead of copied.
    // 外语声明属于同一棵树。没有这一步，C++ 面对每个结构操作都是不可见的：move 规划器只看到
    // `FaceView`，因此既不能拒绝也不能规划包含它的子树。身份/模块规则复用构建自己的，而不是复制。
    //
    // Resolved **before** the Rust parents, because a Rust face under a foreign one names its parent
    // through the generated `xirang_external::<ident>` alias: that alias is the foreign face's
    // flattened identifier, and it only exists once the foreign faces are resolved. A Rust child
    // whose parent is foreign used to come back `parent-unresolved` with the fallback path
    // `root/<name>` — the same tree the runtime has, spelled differently, which is how one project
    // came to answer with two logical paths.
    // 在 Rust 父级**之前**解析，因为挂在外部面下的 Rust 面用生成的 `xirang_external::<ident>` 别名
    // 点名父级：那个别名就是外部面的扁平标识符，只有在外部面解析之后才存在。父级是外部面的 Rust 子面
    // 过去以 `parent-unresolved` 加回落路径 `root/<名字>` 返回——与运行期同一棵树、拼写不同，同一个工程
    // 因此给出了两条逻辑路径。
    #[cfg(feature = "lang-cpp")]
    let native = {
        let mut known = raw
            .iter()
            .map(|face| {
                (
                    super::face_header::identifier(&face.module, &face.kind),
                    (face.module.clone(), face.id),
                )
            })
            .collect::<std::collections::BTreeMap<_, _>>();
        // A declaration may name a parent that has **moved**: the generated header keeps the retired
        // module path, so the derivation has to resolve the same spelling or this view would report a
        // tree the build accepts as broken.
        // 一条声明可能点名一个**搬走过**的父：生成的头部保留退役的模块路径，因此推导必须解析同一种拼法，
        // 否则这个视图会把构建接受的树报成断链。
        {
            let mut live = raw
                .iter()
                .map(|face| (face.source.clone(), face.module.clone(), face.id))
                .collect::<Vec<_>>();
            if let Some((_recipe, declared)) = super::declared_native::read(root)? {
                live.extend(declared.iter().map(|face| {
                    (
                        face.source.clone(),
                        super::declared_native::module_of(&face.source),
                        super::registry_identity::NodeId::from_namespaced_path(
                            package,
                            &face.source,
                            &face.name,
                        ),
                    )
                }));
            }
            super::compatibility::extend_known(&mut known, root, &live);
        }
        // The ledger stores the current logical path of every object it observed. That is the
        // exact alias a generated C++ declaration uses; seed it before falling back to a leaf guess.
        // 台账存储每个对象的当前逻辑路径。那正是生成的 C++ 声明使用的别名；先预置它，再退到叶子猜测。
        if let Ok(map) = xirang_multilang::object_map::ObjectMap::read_from(root) {
            for entry in map.entries() {
                if let Some(id) = entry.current_id()
                    && let Some(path) = entry.path.strip_prefix("root/")
                {
                    let path = path.replace('/', "::");
                    known.insert(path.clone(), (path, id));
                }
            }
        }
        // A generated C++ declaration may name a Rust parent through the shared catalog spelling
        // (`crate::catalog::panel`). When the view is built without a published record the logical
        // path is not available yet; a **unique module leaf** is the only safe alias. A shared leaf
        // stays unresolved instead of naming one of several objects.
        // 生成的 C++ 声明可能经共享目录拼法（`crate::catalog::panel`）点名 Rust 父。没有发布记录时视图
        // 还没有逻辑路径；**唯一模块叶子**是唯一安全的别名。共用叶子保持未解析，而不是替它猜其中一个。
        let mut leaves: std::collections::BTreeMap<String, Vec<NodeId>> =
            std::collections::BTreeMap::new();
        for (module, id) in known.values() {
            if let Some(leaf) = module.rsplit("::").next() {
                leaves.entry(leaf.to_owned()).or_default().push(*id);
            }
        }
        for (leaf, mut ids) in leaves {
            ids.sort();
            ids.dedup();
            if ids.len() == 1 {
                known.insert(leaf.clone(), (leaf, ids[0]));
            }
        }
        match super::declared_native::read(root)? {
            Some((_recipe, declared)) => {
                super::declared_native::resolve(package, &declared, &known)?
            }
            None => Vec::new(),
        }
    };
    #[cfg(not(feature = "lang-cpp"))]
    let native: Vec<super::declared_native::Resolved> = Vec::new();
    for face in &native {
        // The spelling a Rust declaration uses for a foreign parent, with `crate::` stripped by
        // `resolve_parent` exactly as it does for a Rust module path.
        // Rust 声明点名外语父级所用的拼写；`resolve_parent` 会像对 Rust 模块路径那样剥掉 `crate::`。
        modules.insert(format!("xirang_external::{}", face.ident), face.id);
    }
    let resolved = raw
        .into_iter()
        .map(|face| {
            let (parent, parent_resolved) = resolve_parent(&face.parent, package, &modules);
            (face, parent, parent_resolved)
        })
        .collect::<Vec<_>>();
    let mut names = resolved
        .iter()
        .map(|(face, parent, _)| (face.id, *parent, face.registry_name.clone()))
        .collect::<Vec<_>>();
    names.extend(
        native
            .iter()
            .map(|face| (face.id, face.parent, face.name.clone())),
    );
    let mut paths = logical_paths(root_node_id(package), &names);
    let mut walking = BTreeSet::new();
    let mut views = resolved
        .into_iter()
        .map(|(face, parent, parent_resolved)| FaceView {
            id: face.id,
            stable_name: face.stable_name,
            parent,
            kind: face.kind,
            registry_name: face.registry_name,
            module: face.module,
            source: face.source,
            source_base: face.source_base,
            owns_registry: face.owns_registry,
            path: logical_path(face.id, &names, &mut paths, &mut walking),
            parent_resolved,
        })
        .collect::<Vec<_>>();
    views.extend(native.into_iter().map(|face| FaceView {
        id: face.id,
        // The C++ declaration spelling has no `stable_name` field; its object id is derived from
        // the logical path, which is what the ledger records for it.
        // C++ 声明拼法没有 `stable_name` 字段；它的对象 id 由逻辑路径推导，台账记录的正是那个。
        stable_name: None,
        parent: face.parent,
        kind: face.name.clone(),
        registry_name: face.name,
        module: super::declared_native::module_of(&face.source),
        path: logical_path(face.id, &names, &mut paths, &mut walking),
        source: face.source,
        source_base: SourceBase::Package,
        owns_registry: face.owns_registry,
        parent_resolved: true,
    }));
    // Python declarations are extracted (see `languages::python::faces`) but the build does not
    // register them yet. Saying so here is the difference between a view that is partial and one that
    // silently drops a declared face: the row cannot be a `FaceView` because nothing registers it.
    // Python 声明已被提取（见 `languages::python::faces`），但构建尚未登记它们。在这里说出来，是"部分
    // 视图"与"静默丢面"的差别：这行不能成为 `FaceView`，因为没有任何东西登记它。
    #[cfg(feature = "lang-python")]
    {
        for face in xirang_multilang::languages::python::faces::read_declarations(root)? {
            // The origin is named when it is a declared source, so a reader can tell a package
            // file from one materialized out of `xirang.sources.tsv`; the workspace is implicit
            // and stays unspoken, the way the package root does everywhere else.
            // 来源是已声明来源时点名，读者因此能区分包内文件与从 `xirang.sources.tsv` 物化来的文件；
            // 工作区是隐式的，像包根在别处一样不额外说出来。
            let origin = if face.origin == xirang_multilang::source_map::WORKSPACE_SOURCE {
                String::new()
            } else {
                format!(" (from source `{}`)", face.origin)
            };
            external.push(format!(
                "{}: python face `{}` is declared{origin} but this build does not register Python \
                 faces yet; it is not part of the generated tree",
                face.source, face.name
            ));
        }
    }
    // JavaScript declarations are extracted the same way and with the same honesty: the call runs
    // when a module is imported, but this build does not import JavaScript, so the face is reported
    // as declared-and-not-registered instead of being dropped or silently claimed.
    // JavaScript 声明用同一种方式、同样诚实地提取：模块被导入时调用会执行，但本构建不导入 JavaScript，
    // 因此这个面被报成"已声明但未登记"，既不丢弃也不悄悄算作已注册。
    #[cfg(feature = "lang-js")]
    {
        for face in xirang_multilang::languages::js::faces::read_declarations(root)? {
            let origin = if face.origin == xirang_multilang::source_map::WORKSPACE_SOURCE {
                String::new()
            } else {
                format!(" (from source `{}`)", face.origin)
            };
            external.push(format!(
                "{}: javascript face `{}` is declared{origin} but this build does not register \
                 JavaScript faces yet; it is not part of the generated tree",
                face.source, face.name
            ));
        }
    }
    // TypeScript declarations share the JavaScript carrier and are reported the same way: the
    // typed call is read, and the build says it does not register it rather than dropping it.
    // TypeScript 声明共用 JavaScript 载体，也以同样方式报告：带类型的调用被读取，构建说出它没有登记它
    // 而不是把它丢掉。
    #[cfg(feature = "lang-ts")]
    {
        for face in xirang_multilang::languages::ts::faces::read_declarations(root)? {
            let origin = if face.origin == xirang_multilang::source_map::WORKSPACE_SOURCE {
                String::new()
            } else {
                format!(" (from source `{}`)", face.origin)
            };
            external.push(format!(
                "{}: typescript face `{}` is declared{origin} but this build does not register \
                 TypeScript faces yet; it is not part of the generated tree",
                face.source, face.name
            ));
        }
    }
    views.sort_by(|left, right| (&left.path, left.id).cmp(&(&right.path, right.id)));
    Ok((views, unreadable, external))
}

/// The faces a published pruning record describes, without walking the sources (audit `T3`).
/// 一份已发布的剪枝记录所描述的面，不必走一遍源码（审计 `T3`）。
///
/// This is the reader `parent_node`/`owns_registry`/`logical_path` were published for: a family
/// question ("who is under this parent, and what does each declare") needed the **derived** tree,
/// which parses every file in the member — measured on a 2,501-face member as 11 s for `consistency`
/// against 0.97 s for `registry`, which takes its faces from this same record. The record is the
/// build's own statement about the faces it published, so a reader that trusts it pays nothing for
/// the work the build already did.
/// 这就是发布 `parent_node`/`owns_registry`/`logical_path` 所服务的读者：一个家族问题（"这个父级下有哪些面、
/// 各自声明了什么"）过去需要**推导**树，那要解析成员里的每个文件——在一个 2,501 面的成员上实测：`consistency`
/// 11 s，而**从这份记录取面**的 `registry` 0.97 s。记录是构建对自己发布过的面的陈述，因此信它的读者不必为
/// 构建已经付过的钱再付一次。
///
/// Rows that do not carry what a `FaceView` needs are **counted, not guessed**: the caller compares
/// that count against zero and derives instead, because a partially-built family would compare a face
/// against a smaller set than the one it belongs to — the silent wrong answer this whole area keeps
/// producing.
/// 不带 `FaceView` 所需各列的行会被**计数、而不是被猜**：调用方拿这个数与零比较，非零就改为推导——因为一个
/// 只建了一半的家族会拿一个更小的集合去比，而"格式正常但错"的答案正是这一带反复生产的东西。
pub fn face_views_from_pruning(rows: &[PruningRow]) -> (Vec<FaceView>, usize) {
    let mut views = Vec::new();
    let mut skipped = 0usize;
    for row in rows {
        // `registry_name` is deliberately **not** required: a face that leaves it to the macro
        // default publishes `-`, and filling that in here would be a second implementation of the
        // macro's own rule. The comparison this serves never reads that column — it indexes faces by
        // logical path and compares declared fields and calls — so an absent one is not a reason to
        // give up the record.
        // 有意**不**要求 `registry_name`：把该列留给宏默认值的面发布的是 `-`，在这里补上它就是宏那条规则
        // 的第二份实现。这个读者所服务的比对从不读那一列——它按逻辑路径索引面、比的是声明字段与调用——因此
        // 缺那一列不是放弃记录的理由。
        let (Some(kind), Some(path), Some(parent_node)) = (
            row.kind.as_deref(),
            row.logical_path.as_deref(),
            row.parent_node.as_deref(),
        ) else {
            skipped += 1;
            continue;
        };
        let Ok(parent) = parent_node.parse::<NodeId>() else {
            skipped += 1;
            continue;
        };
        views.push(FaceView {
            id: row.id,
            // A pruning record does not publish `stable_name`, so this reader cannot answer the
            // author-owned half. The derived half (`object_id` without an explicit name) is still
            // right, and the ledger is the authority for a declaration that named one.
            // 剪枝记录不发布 `stable_name`，因此这个读取方答不出作者拥有的那一半。推导出的那一半
            // （没有显式名字时的 `object_id`）仍然正确，而台账才是"声明了名字"的权威。
            stable_name: None,
            parent,
            kind: kind.to_owned(),
            registry_name: row.registry_name.clone().unwrap_or_default(),
            module: row.symbol.clone(),
            source: row.source.clone(),
            // The pruning record is the build's own statement about the faces it derived from
            // sources, so its `source` column uses the identity spelling.
            // 剪枝记录是构建对"它从源码推导出的面"的自述，因此 `source` 列用身份拼写。
            source_base: SourceBase::Identity,
            owns_registry: row.owns_registry.as_deref() == Some("true"),
            path: path.to_owned(),
            parent_resolved: true,
        });
    }
    views.sort_by(|left, right| (&left.path, left.id).cmp(&(&right.path, right.id)));
    (views, skipped)
}

/// Resolve one collected parent against the module-to-face map, matching the
/// build's three parent forms.
/// 用"模块 → 面"映射解析一个已收集的父级，对应构建的三种父级形式。
pub(crate) fn resolve_parent(
    parent: &ParentSpec,
    package: &str,
    modules: &BTreeMap<String, NodeId>,
) -> (NodeId, bool) {
    match parent {
        ParentSpec::Root => (root_node_id(package), true),
        ParentSpec::FromPath { source, kind } => {
            (NodeId::from_namespaced_path(package, source, kind), true)
        }
        ParentSpec::NodePath(module) => {
            // The build strips the `crate::build_method::` prefix and maps the module back to
            // the face that owns it (`cached_parent_id`). A module the tree does
            // not own stays unresolved so the operator sees the broken chain.
            // 构建剥掉 `crate::build_method::` 前缀，把模块映射回拥有它的面（`cached_parent_id`）。
            // 树不拥有的模块保持未解析，操作者因此看到断链。
            let key = module.strip_prefix("crate::").unwrap_or(module);
            match modules.get(key) {
                Some(id) => (*id, true),
                None => (root_node_id(package), false),
            }
        }
        ParentSpec::Unparsed => (root_node_id(package), false),
    }
}

/// The root identity for one package, matching the build's own helper.
/// 某个包的根身份，与构建自己的辅助函数一致。
fn root_node_id(package: &str) -> NodeId {
    NodeId::from_namespaced_path(package, "<root>", "root")
}

/// The registry name a declaration resolves to.
/// 一条声明解析到的注册名。
///
/// The fallback is the macro's default, not a new rule: a face that omits `registry_name` takes its
/// module's last segment (`__face_string_or!(last_path_segment(module_path!()))` in
/// `run_method/src/macros/face_objects.rs`). A command that guessed anything else would print a
/// logical path the runtime never had — and since the pruning record now publishes the logical path
/// (audit `W3-2`), a second spelling of this rule in the manifest writer would be a second answer to
/// the same question. Both callers call this one.
/// 回退值是**宏的默认值**，不是新规则：省略 `registry_name` 的面取模块名末段
/// （`run_method/src/macros/face_objects.rs` 中的 `__face_string_or!(last_path_segment(module_path!()))`）。
/// 命令若猜成别的，就会打印出运行期从未有过的逻辑路径——而由于剪枝记录现在会发布逻辑路径（审计 `W3-2`），
/// 在清单写入方里另写一份这条规则就是对同一个问题的第二个答案。两个调用方调的都是这一个。
pub(crate) fn resolved_registry_name(face: &FaceSyntax, module: &str) -> String {
    face.path("registry_name")
        .unwrap_or_else(|| module.rsplit("::").next().unwrap_or(module).to_owned())
}

/// The logical path of every face, from the `(id, parent id, registry name)` triples that name them.
/// 每个面的逻辑路径，取自命名它们的 `(id, 父级 id, 注册名)` 三元组。
///
/// `pub(crate)` since audit `W3-2`: the pruning record publishes this path, so the writer and the
/// derived view have to walk it with the same function — and the walk is what the two used to
/// disagree about when the record carried no path at all.
/// 自审计 `W3-2` 起为 `pub(crate)`：剪枝记录发布这条路径，因此写入方与推导视图必须用同一个函数走它——
/// 而当记录根本不携带路径时，两者分歧的正是这次行走。
pub(crate) fn logical_paths(
    root: NodeId,
    names: &[(NodeId, NodeId, String)],
) -> BTreeMap<NodeId, String> {
    let mut paths = BTreeMap::new();
    paths.insert(root, "root".to_owned());
    let mut walking = BTreeSet::new();
    for (id, _, _) in names {
        logical_path(*id, names, &mut paths, &mut walking);
    }
    paths
}

fn visit_face_view(
    layout: &SourceLayout,
    nodes: &[Node],
    package: &str,
    module_prefix: &str,
    faces: &mut Vec<RawFace>,
) {
    for node in nodes {
        // The Rust module address is the **generated module chain**: this node's name under the
        // nodes that mount it. It is not re-derived from the physical directory (an island mounts at
        // its stem, `reading`, not at `components::reading`).
        // Rust 模块地址是**生成的模块链**：本节点的名字，加上挂载它的那些节点。它不从物理目录重新推导
        // （孤岛挂在 stem `reading` 下，不是 `components::reading`）。
        let module = if module_prefix.is_empty() {
            node.name.clone()
        } else {
            format!("{module_prefix}::{}", node.name)
        };
        if let Some(file) = &node.file {
            let relative = super::discovery_node::record_spelling(layout, file);
            let source_base = xirang_multilang::source_ref::base_for(
                &layout.package_root,
                &layout.identity_base,
                &super::discovery_node::package_source(layout, file),
            );
            if !lexicon::is_registration_path(&relative)
                && let Ok(source) = fs::read_to_string(file)
                && let Some(face) = parsed_face(&source, &relative)
                && let Some(kind) = face.path("kind")
            {
                let registry_name = resolved_registry_name(&face, &module);
                faces.push(RawFace {
                    id: NodeId::from_namespaced_path(package, &relative, &kind),
                    // The author-owned half of the persistent object identity, when declared.
                    // It is a **string literal** (`.stable_name("ui.button")`), not a path: reading
                    // it with `path` parsed a literal as a type path, failed, and silently dropped
                    // the field — so every view derived a path-based identity for a face whose
                    // author had chosen one, and the build and this reader disagreed about which
                    // object a declaration is.
                    // 持续对象身份里作者拥有的那一半（当声明了时）。它是**字符串字面量**
                    // （`.stable_name("ui.button")`），不是路径：用 `path` 去读会把字面量当类型路径解析、
                    // 失败，然后静默丢掉这个字段——于是作者已经选好身份的每个面，视图仍按路径推导身份，
                    // 而构建与本读取方对“这条声明是哪个对象”给出不同答案。
                    stable_name: stable_name_of(&face),
                    parent: parent_of(&face),
                    kind,
                    registry_name,
                    module: module.clone(),
                    source: relative,
                    source_base,
                    owns_registry: face.boolean("needs_registry").unwrap_or(false),
                });
            }
        }
        visit_face_view(layout, &node.children, package, &module, faces);
    }
}

/// The author-owned stable name, in either spelling this tree accepts.
/// 作者拥有的稳定名，按这棵树接受的两种拼法。
///
/// The native declaration writes `.stable_name("ui.button")` (a bare string), and the
/// `Registration` literal writes `stable_name: Some("ui.button")`. Reading only one of them is how
/// this field was silently dropped: a `path` reader parsed the literal as a type path and failed,
/// so every view derived a path-based identity for a face whose author had chosen one.
/// 原生声明写 `.stable_name("ui.button")`（裸字符串），而 `Registration` 字面量写
/// `stable_name: Some("ui.button")`。只读其中一种，正是这个字段被静默丢掉的方式：`path` 读取器把
/// 字面量当类型路径解析并失败，于是作者已经选好身份的面，视图仍按路径推导身份。
fn stable_name_of(face: &FaceSyntax) -> Option<String> {
    face.string("stable_name")
        .or_else(|| face.option_string("stable_name").flatten())
}

/// Read one face's parent declaration without resolving it, exactly the way the
/// build decides "root or declared".
/// 读取一个面的父级声明而不解析它，判断方式与构建"根还是显式声明"完全一致。
///
/// No `parent` field means the package root. A `parent` field that parses to a
/// module or a source/kind pair is kept as data; one that does not parse is
/// `Unparsed` and becomes a reported unresolved parent rather than a guess. The
/// build turns the same case into a `static-plan` diagnostic.
/// 没有 `parent` 字段即包根。解析成模块或 source/kind 对的 `parent` 字段按数据保留；
/// 解析不了的记为 `Unparsed`，成为被上报的未解析父级而不是猜测。构建对同一情形报的
/// 是 `static-plan` 诊断。
pub(crate) fn parent_of(face: &FaceSyntax) -> ParentSpec {
    if face.field("parent").is_none() {
        return ParentSpec::Root;
    }
    match face.parent() {
        Some(ParentSyntax::Root) => ParentSpec::Root,
        Some(ParentSyntax::FromPath { source, kind }) => ParentSpec::FromPath { source, kind },
        Some(ParentSyntax::NodePath(module)) => ParentSpec::NodePath(module),
        None => ParentSpec::Unparsed,
    }
}

/// The logical registry path of one face, walking the resolved parent chain.
/// 一个面的逻辑注册路径，沿已解析的父级链行走。
///
/// The parent's own path is resolved before this one is formed, so the order the
/// discovery walk happened to visit the nodes in is not an input to the answer.
/// The map still caches a finished chain, because a tree with many faces shares
/// chains and each one is then walked once.
/// 父面自己的路径先生成，再拼出本面，因此发现遍历恰好按什么顺序访问节点不是这个答案的输入。
/// 映射仍缓存已完成的链：面很多的树会共用链，而每条链只走一次。
///
/// Two walks stop at `root` instead of inventing a prefix: a parent that the face
/// set does not contain (the unresolved-parent case `parent_resolved` reports),
/// and a chain that comes back to a face already being resolved. The second is a
/// cycle, which no runtime tree can have — the registry rejects it — but a
/// read-only view of a tree the build would refuse must terminate rather than
/// recurse until the stack runs out.
/// 两种走法会停在 `root` 而不是编造前缀：面集合里不存在的父级（也就是 `parent_resolved`
/// 报告的那种未解析父级），以及回到一个正在解析中的面的链。后者是环，运行期的树不可能有——
/// 注册机会拒绝它——但"构建会拒绝的树"的只读视图必须能终止，而不是递归到栈耗尽。
fn logical_path(
    id: NodeId,
    names: &[(NodeId, NodeId, String)],
    paths: &mut BTreeMap<NodeId, String>,
    walking: &mut BTreeSet<NodeId>,
) -> String {
    if let Some(path) = paths.get(&id) {
        return path.clone();
    }
    let Some((_, parent, name)) = names.iter().find(|(candidate, _, _)| *candidate == id) else {
        return "root".to_owned();
    };
    let parent_path = if !names.iter().any(|(candidate, _, _)| candidate == parent) {
        "root".to_owned()
    } else if walking.insert(id) {
        let path = logical_path(*parent, names, paths, walking);
        walking.remove(&id);
        path
    } else {
        "root".to_owned()
    };
    let path = format!("{parent_path}/{name}");
    paths.insert(id, path.clone());
    path
}

#[cfg(test)]
#[path = "face_view_tests.rs"]
mod face_view_tests;
