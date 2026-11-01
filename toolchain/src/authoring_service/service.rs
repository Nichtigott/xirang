//! The shared authoring service every surface calls.
//! 每个执行面都调用的共享创作服务。
//!
//! The CLI, the Studio form and the MCP bridge used to each hold their own idea of "add a face":
//! the Rust generator was shared, but there was no place where a language could be chosen, a
//! destination derived, or a foreign-language plan produced. This module is that place. It resolves
//! the request (language, parent, destination, defaults), hands it to a language adapter, and
//! applies the resulting plan through one transaction.
//! CLI、Studio 表单与 MCP 桥过去各自持有"加一个面"的想法：Rust 生成器是共用的，但没有一个地方能
//! 选择语言、推导目的地或产出外语计划。本模块就是那个地方。它解析请求（语言、父、目的地、默认值），
//! 交给语言适配器，并通过一个事务落盘产出的计划。

use std::path::{Path, PathBuf};

use xirang_kernel::authoring::{
    AuthoringOperation, FaceAuthoringPlan, FaceAuthoringRequest, FieldUpdate, LanguageCapabilities,
    RegistrationField, RegistrationUpdate, SourceContext,
};
use xirang_kernel::identity::NodeId;
use xirang_kernel::interchange::LanguageId;
use xirang_kernel::tree::Registry;

use xirang_kernel::ROOT_NODE_ID;
use xirang_kernel::declaration::RegistrationSnapshot;

use xirang_multilang::registration;

use crate::build_method::declared_native;
use crate::run_method::AuthoredFace;
use crate::run_method::AuthoringContext;
use crate::run_method::authoring::write_transaction::WriteTransaction;
use crate::source_layout::{SourceLayout, source_layout};

use self::rust_adapter::FaceFields;

#[path = "service_editor.rs"]
mod editor;
#[path = "service_languages.rs"]
mod languages;

pub use self::languages::{available_languages, language_capabilities};

use self::languages::rust_capabilities;
#[path = "placement.rs"]
pub(crate) mod placement;
#[path = "resolve.rs"]
mod resolve;
#[path = "rust_adapter.rs"]
mod rust_adapter;

/// What a surface asks for, before the service resolves it.
/// 执行面请求的内容，在服务解析它之前。
///
/// `language` is optional on purpose: an explicit choice wins, an existing file's extension is a
/// reliable hint, a project default is a decision the project made, and anything else is an
/// ambiguity the caller must settle. Deriving the language from the parent is deliberately not one
/// of the steps — that would force a child's implementation language to match its parent's.
/// `language` 有意可选：显式选择优先，已有文件的扩展名是可靠线索，项目默认值是项目做过的决定，其余
/// 属于调用方必须解决的歧义。从父推导语言**不在**这些步骤里——那会强迫子面的实现语言与父一致。
#[derive(Clone, Debug)]
pub struct AuthoringRequest {
    /// The language the caller chose, when it chose one.
    /// 调用方选择的语言（当它选择时）。
    pub language: Option<LanguageId>,
    /// Create or modify.
    /// 新建或修改。
    pub operation: AuthoringOperation,
    /// The existing face's identity, for a modify.
    /// 修改时已有面的身份。
    pub node: Option<NodeId>,
    /// The module name, for a create.
    /// 新建时的模块名。
    pub module: Option<String>,
    /// The parent identity, for a create.
    /// 新建时的父身份。
    pub parent: Option<NodeId>,
    /// The package-relative file, when the caller names it.
    /// 调用方点名时的包内相对文件。
    pub destination: Option<String>,
    /// The fields the caller names.
    /// 调用方点名的字段。
    pub registration: RegistrationUpdate,
}

impl AuthoringRequest {
    /// A create request that names only its operation.
    /// 只点名操作的创建请求。
    pub fn create() -> Self {
        Self {
            language: None,
            operation: AuthoringOperation::Create,
            node: None,
            module: None,
            parent: None,
            destination: None,
            registration: RegistrationUpdate::new(),
        }
    }

    /// A modify request for one face.
    /// 针对一个面的修改请求。
    pub fn modify(node: NodeId) -> Self {
        Self {
            language: None,
            operation: AuthoringOperation::Modify,
            node: Some(node),
            module: None,
            parent: None,
            destination: None,
            registration: RegistrationUpdate::new(),
        }
    }
}

/// What applying a plan wrote.
/// 落盘计划写了什么。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AppliedAuthoring {
    /// The package-relative files written, removed, or created.
    /// 写入、移除或创建的包内相对文件。
    pub written: Vec<String>,
    /// A one-line summary.
    /// 一行摘要。
    pub message: String,
}

/// One package's authoring service: a package root and the namespace its faces are authored under.
/// 一个包的创作服务：包根，以及其注册面创作所用的命名空间。
#[derive(Clone, Debug)]
pub struct AuthoringService {
    root: PathBuf,
    namespace: String,
}

impl AuthoringService {
    /// Bind the service to a package root and namespace.
    /// 把服务绑定到包根与命名空间。
    pub fn new(root: impl Into<PathBuf>, namespace: impl Into<String>) -> Self {
        Self {
            root: root.into(),
            namespace: namespace.into(),
        }
    }

    /// The package root this service writes into.
    /// 本服务写入的包根。
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// The languages this build can author: Rust always, plus every compiled adapter.
    /// 本构建可以创作的语言：始终有 Rust，加上每个已编译适配器。
    pub fn languages(&self) -> Vec<LanguageId> {
        available_languages()
    }

    /// The capability tables this build carries, for a surface's language picker.
    /// 本构建携带的能力表，供执行面的语言选择。
    pub fn capabilities(&self) -> Vec<LanguageCapabilities> {
        let mut tables = vec![rust_capabilities()];
        tables.extend(registration::registry().capabilities().into_iter().cloned());
        tables
    }

    /// Resolve one request and produce a reviewable plan. Nothing is written.
    /// 解析一份请求并产出可审阅的计划。不写任何东西。
    pub fn plan(&self, request: &AuthoringRequest) -> Result<FaceAuthoringPlan, String> {
        // The existing Rust generator reads its namespace and package root from the authoring
        // context, so the whole plan runs inside it. A plan made outside the context would bake
        // `xirang.default` into the declaration and the staged registry would reject it.
        // 现有 Rust 生成器从创作上下文读取命名空间与包根，因此整个计划都在该上下文中运行。在上下文
        // 之外做的计划会把 `xirang.default` 烘进声明，暂存注册树会拒绝它。
        AuthoringContext::new(self.root.clone(), self.namespace.clone())
            .scope(|| self.plan_in_context(request))
    }

    fn plan_in_context(&self, request: &AuthoringRequest) -> Result<FaceAuthoringPlan, String> {
        let layout = source_layout(&self.root)?;
        let (registry, snapshots) = self.load_registry(&layout)?;
        let native = resolve::native_faces(&self.root)?;
        match request.operation {
            AuthoringOperation::Create => {
                self.plan_create(request, &layout, &registry, &snapshots, &native)
            }
            AuthoringOperation::Modify => {
                self.plan_modify(request, &layout, &registry, &snapshots, &native)
            }
        }
    }

    /// Apply a plan through one transaction: all files or none.
    /// 经一个事务落盘计划：要么全部写入，要么全都不写。
    ///
    /// Two properties, in this order and both load-bearing:
    /// 两条性质，次序如此、缺一不可：
    ///
    /// 1. **The complete write set is resolved and authorized before the first byte.** A plan names
    ///    its files up front, so a refusal on the second one must not arrive as a rollback of the
    ///    first — and the set, not "the main file", is what the write-scope rule judges.
    /// 2. **One transaction owns every byte.** It captures each path before changing it (so a write
    ///    that fails after truncating is still restorable) and removes what it created, and its
    ///    rollback reports the paths it could **not** restore instead of claiming a clean tree.
    /// 1. **在写出第一个字节之前，完整写入集合已被解析并授权。** 计划一开始就点明它的文件，因此第二个文件的
    ///    拒绝绝不该以"回滚第一个"的形式到达——而写入作用域规则判定的是这个**集合**，不是"那个主文件"。
    /// 2. **每一个字节由同一个事务拥有。** 它在改动之前保存每个路径（因此"截断之后才失败"的写入仍可恢复）、
    ///    删除自己创建的东西，而且它的回滚会点名**没能**恢复的路径，而不是声称树是干净的。
    pub fn apply(&self, plan: &FaceAuthoringPlan) -> Result<AppliedAuthoring, String> {
        let changed = plan
            .files
            .iter()
            .filter(|file| file.changes())
            .collect::<Vec<_>>();
        let mut write_set = Vec::new();
        let mut problems = Vec::new();
        for file in &changed {
            match self.absolute(&file.path) {
                Ok(path) => write_set.push((*file, path)),
                Err(error) => problems.push(error),
            }
        }
        if !problems.is_empty() {
            return Err(format!("{}; nothing was written", problems.join("; ")));
        }
        let mut written = Vec::new();
        let mut transaction = WriteTransaction::new();
        let outcome = (|| -> Result<(), String> {
            for (file, path) in &write_set {
                match &file.original {
                    // A create never overwrites: `create_new` refuses an existing target before a
                    // byte is written, and forgets the capture again so a rollback cannot rewrite a
                    // file this plan never owned.
                    // 新建从不覆盖：`create_new` 在写出字节之前就拒绝已存在的目标，并撤掉那份保存，
                    // 使回滚不会去重写一个本计划从未拥有的文件。
                    None => transaction.create_new(path, file.updated.as_bytes())?,
                    Some(original) => {
                        let current = std::fs::read_to_string(path)
                            .map_err(|error| format!("cannot read {}: {error}", file.path))?;
                        if &current != original {
                            return Err(format!(
                                "{} changed since the plan was made; re-run the plan before \
                                 applying it",
                                file.path
                            ));
                        }
                        if file.remove {
                            transaction.remove(path)?;
                        } else {
                            transaction.write_atomic(path, file.updated.as_bytes())?;
                        }
                    }
                }
                written.push(file.path.clone());
            }
            Ok(())
        })();
        if let Err(error) = outcome {
            let recovery = transaction.rollback();
            return Err(format!("{error}; {}", recovery.describe()));
        }
        let count = written.len();
        Ok(AppliedAuthoring {
            written,
            message: format!(
                "{} `{}` plan wrote {count} file(s)",
                plan.operation.as_str(),
                plan.language.as_str()
            ),
        })
    }

    fn plan_create(
        &self,
        request: &AuthoringRequest,
        layout: &SourceLayout,
        registry: &Registry,
        _snapshots: &[RegistrationSnapshot],
        native: &[declared_native::DeclaredFace],
    ) -> Result<FaceAuthoringPlan, String> {
        let module = match &request.module {
            Some(module) => module.clone(),
            None => request
                .destination
                .as_deref()
                .and_then(|path| {
                    Path::new(path)
                        .file_stem()
                        .and_then(|stem| stem.to_str())
                        .map(str::to_owned)
                })
                .ok_or_else(|| {
                    "create needs a module name (or a destination file whose stem names one)"
                        .to_owned()
                })?,
        };
        let language = languages::select_language(
            &self.root,
            &self.languages(),
            request.language.as_ref(),
            request.destination.as_deref(),
        )?;
        // A Rust face **can** be placed in an explicit directory; what it cannot be is placed
        // somewhere its module path would disagree with. That rule is spelled once, in
        // `placement::rust_explicit_directory`, and it refuses by naming the spelling that works.
        // Rust 面**可以**放进显式目录；它不能的只是"放成模块路径与文件不一致"。这条规则只在
        // `placement::rust_explicit_directory` 里写一次，而它拒绝时会点名那种能成立的拼写。
        let rust_directory = if language.as_str() == "rust" {
            match &request.destination {
                Some(destination) => Some(placement::rust_explicit_directory(
                    layout,
                    destination,
                    &module,
                )?),
                None => None,
            }
        } else {
            None
        };
        let parent_id = match request.parent {
            Some(id) => id,
            // No parent named: an island directory decides it when exactly one object owns it, and
            // the ledger refuses to guess when several do.
            // 没有点名父级时：若恰好一个对象拥有这个孤岛目录，就由它决定；有多个时台账拒绝猜测。
            None => placement::island_parent(
                &self.root,
                &self.namespace,
                request.destination.as_deref(),
            )?
            .unwrap_or_else(|| registry.id()),
        };
        let parent_id = if parent_id == ROOT_NODE_ID {
            registry.id()
        } else {
            parent_id
        };
        let parent =
            resolve::parent_face(&self.root, registry, native, &self.namespace, parent_id)?;
        if language.as_str() == "rust" {
            let kind = match request.registration.find_field(RegistrationField::Kind) {
                FieldUpdate::Set(kind) => kind.to_owned(),
                _ => xirang_kernel::authoring::pascal_case(&module),
            };
            let fields = FaceFields::for_create(&request.registration, &module, &kind);
            let mut plan = rust_adapter::plan_create(
                &self.root,
                layout,
                registry,
                &parent,
                &fields,
                rust_directory
                    .as_ref()
                    .map(|(directory, base)| (directory.as_path(), *base)),
            )?;
            // The default layout is what `add` derives when nothing is named; an explicit
            // destination that differs from it is a **deviation**, and the ledger is the list of
            // deviations. A descendant inherits it because a create derives its directory from the
            // parent's recorded source, exactly as the foreign half does.
            // 默认布局是没点名任何东西时 `add` 推导出的落点；与它不同的显式目的地是一次**偏离**，而台账就是
            // 偏离项的清单。后代继承它，是因为一次新建从父级记录的源码推导自己的目录——与外语那一半完全相同。
            if let Some(explicit) = &request.destination {
                let default = resolve::destination(
                    layout,
                    &parent,
                    &module,
                    xirang_multilang::registration::SourcePlacement::UnderScanRoot,
                    "rs",
                )?
                .to_string_lossy()
                .replace('\\', "/");
                if explicit.replace('\\', "/") != default
                    && let Some(ledger) = placement::record_deviation(
                        &self.root,
                        &self.namespace,
                        parent_id,
                        &module,
                        explicit,
                        &request.registration,
                    )?
                {
                    plan.files.push(ledger.planned());
                }
            }
            return Ok(plan);
        }
        // The destination's extension is the adapter's own claim, not a list here: a language that
        // cannot read a file must not be handed one, and a list in this function would be a second
        // copy of the capability table.
        // 目的地的扩展名由适配器自己声明，而不是由这里的一张表决定：读不了某种文件的语言不该被交给那种
        // 文件，而在这里列一张表等于复制第二份能力表。
        let adapter = registration::registry().adapter(&language).ok_or_else(|| {
            format!(
                "`{}` support is not compiled into this build",
                language.as_str()
            )
        })?;
        let extensions = adapter.capabilities().file_extensions;
        let extension = extensions.first().ok_or_else(|| {
            format!(
                "`{}` claims no file extension; an adapter that reads no source cannot author one",
                language.as_str()
            )
        })?;
        if let Some(destination) = &request.destination
            && !extensions
                .iter()
                .any(|extension| destination.ends_with(&format!(".{extension}")))
        {
            return Err(format!(
                "`{destination}` is not a `{}` source file; `{}` reads {}",
                extension,
                language.as_str(),
                extensions
                    .iter()
                    .map(|extension| format!("`.{extension}`"))
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        let explicit_destination = request.destination.clone();
        let destination = match &explicit_destination {
            Some(destination) => destination.clone(),
            None => resolve::destination(layout, &parent, &module, adapter.placement(), extension)?
                .to_string_lossy()
                .replace('\\', "/"),
        };
        let mut registration = request.registration.clone();
        registration.set_text(RegistrationField::Module, FieldUpdate::Set(&module))?;
        let resolved = FaceAuthoringRequest {
            language: language.clone(),
            operation: AuthoringOperation::Create,
            destination: destination.clone(),
            parent: if language.as_str() == "cpp" {
                parent.catalog_reference()?
            } else {
                parent.request_reference()?
            },
            registration,
            source_context: SourceContext::new(),
        };
        let plan = registration::registry().plan(&resolved)?;
        let mut plan = self.with_editor_material(plan, adapter, layout);
        // The placement is recorded only when it deviates from the default layout: the ledger is the
        // list of deviations, and "only the nodes that leave the default need a mapping" is exactly
        // this condition. The write joins the plan, so it is previewed, applied and rolled back with
        // the source it describes.
        // 只有偏离默认布局时才记录落点：台账是偏离项的清单，而"只有离开默认布局的节点需要映射"正是这个
        // 条件。这次写入进入计划，因此它与它所描述的源码一起被预览、落盘、回滚。
        if let Some(explicit) = explicit_destination {
            let default =
                resolve::destination(layout, &parent, &module, adapter.placement(), extension)?
                    .to_string_lossy()
                    .replace('\\', "/");
            if explicit != default {
                let leaf = match request.registration.find_field(RegistrationField::Kind) {
                    FieldUpdate::Set(kind) => kind.to_owned(),
                    _ => xirang_kernel::authoring::pascal_case(&module),
                };
                if let Some(ledger) = placement::record_deviation(
                    &self.root,
                    &self.namespace,
                    parent_id,
                    &leaf,
                    &explicit,
                    &request.registration,
                )? {
                    plan.files.push(ledger.planned());
                }
            }
        }
        Ok(plan)
    }

    fn plan_modify(
        &self,
        request: &AuthoringRequest,
        layout: &SourceLayout,
        registry: &Registry,
        snapshots: &[RegistrationSnapshot],
        native: &[declared_native::DeclaredFace],
    ) -> Result<FaceAuthoringPlan, String> {
        let target = self.target(request, layout, snapshots, native)?;
        let language = match &request.language {
            Some(language) => {
                languages::ensure_language(&self.languages(), language)?;
                language.clone()
            }
            None => target.language.clone().ok_or_else(|| {
                format!(
                    "{} is not claimed by an enabled language adapter; pass `--lang` with a \
                     language this build carries",
                    target.path
                )
            })?,
        };
        if language.as_str() == "rust" && !target.is_rust() {
            return Err(format!(
                "{} is a {} face; the request asked for `rust`",
                target.path,
                target
                    .language
                    .clone()
                    .map(|language| language.as_str().to_owned())
                    .unwrap_or_else(|| "non-Rust".to_owned())
            ));
        }
        if language.as_str() != "rust" && target.is_rust() {
            return Err(format!(
                "{} is a Rust face; the request asked for `{}`",
                target.path,
                language.as_str()
            ));
        }
        if language.as_str() == "rust" {
            let face = AuthoredFace::read_source(&self.root.join(&target.path))?;
            let id = target
                .node
                .ok_or_else(|| "a Rust edit needs the face's identity".to_owned())?;
            return rust_adapter::plan_edit(
                &self.root,
                layout,
                registry,
                id,
                &face,
                &request.registration,
            );
        }
        // The declaration is read back by the adapter, not by this service: every language has its
        // own syntax, and a service that knew one of them would be a second implementation of it.
        // 声明由适配器读回，而不是由本服务读回：每种语言有自己的语法，而懂其中一种的服务就是它的第二份实现。
        let adapter = registration::registry().adapter(&language).ok_or_else(|| {
            format!(
                "`{}` support is not compiled into this build",
                language.as_str()
            )
        })?;
        let text = std::fs::read_to_string(self.root.join(&target.path))
            .map_err(|error| format!("cannot read {}: {error}", target.path))?;
        let read_back = adapter.read(&text)?;
        if let Some(parent) = request.parent
            && Some(parent) != target.node
        {
            return Err(
                "changing a face's parent is a move, not a field edit; this service rewrites \
                 the declaration in place"
                    .to_owned(),
            );
        }
        #[rustfmt::skip]
        let parent = resolve::cpp_catalog_reference(&self.root, &self.namespace, &language, &read_back.parent, registry, snapshots, native)?;
        let resolved = FaceAuthoringRequest {
            language: language.clone(),
            operation: AuthoringOperation::Modify,
            destination: target.path.clone(),
            parent,
            registration: request.registration.clone(),
            source_context: SourceContext::new().with_existing_source(text),
        };
        let plan = registration::registry().plan(&resolved)?;
        Ok(self.with_editor_material(plan, adapter, layout))
    }

    /// Resolve which face a modify request targets.
    /// 解析修改请求指向哪个面。
    fn target(
        &self,
        request: &AuthoringRequest,
        layout: &SourceLayout,
        snapshots: &[RegistrationSnapshot],
        native: &[declared_native::DeclaredFace],
    ) -> Result<Target, String> {
        if let Some(node) = request.node {
            if let Some(snapshot) = resolve::snapshot_of(snapshots, node) {
                // A record spelling is relative to the identity base for an ordinary face, but
                // package-relative for an explicit island. Try those two bases and accept only the
                // file whose **identity** is the requested node; a same-named file elsewhere cannot
                // be edited by mistake.
                // 普通面的记录拼写相对身份基准，显式孤岛则相对包根。按这两个基准取候选，并且只接受
                // **身份**正好等于请求节点的文件；同名但身份不同的文件不会被误编辑。
                let candidates = [
                    layout.identity_base.join(&snapshot.source.file),
                    self.root.join(&snapshot.source.file),
                ];
                for candidate in candidates {
                    if !candidate.is_file() {
                        continue;
                    }
                    let face = AuthoredFace::read_source(&candidate)?;
                    let source = resolve::rust_identity_source(layout, &candidate)?;
                    if NodeId::from_namespaced_path(&self.namespace, &source, &face.kind) != node {
                        continue;
                    }
                    return Ok(Target {
                        path: resolve::package_relative(&self.root, &candidate)?,
                        node: Some(node),
                        rust: true,
                        language: None,
                    });
                }
                return Err(format!(
                    "`{node}` is recorded at {} but no Rust source under this package carries that \
                     identity; the record and the tree disagree",
                    snapshot.source.file
                ));
            }
            if let Some(face) = native
                .iter()
                .find(|face| resolve::native_identity(&self.namespace, face) == node)
            {
                return Ok(Target {
                    path: face.source.clone(),
                    node: Some(node),
                    rust: false,
                    language: registration::registry()
                        .language_of_path(&face.source)
                        .cloned(),
                });
            }
            return Err(format!(
                "`{node}` is neither a face this package's registry carries nor a declared native \
                 face"
            ));
        }
        let destination = request.destination.as_deref().ok_or_else(|| {
            "modify needs `node` or `destination` to name the face it edits".to_owned()
        })?;
        let path = self.absolute(destination)?;
        if !path.is_file() {
            return Err(format!("{destination} does not exist"));
        }
        if destination.ends_with(".rs") {
            let face = AuthoredFace::read_source(&path)?;
            let source = resolve::rust_identity_source(layout, &path)?;
            let id = NodeId::from_namespaced_path(&self.namespace, &source, &face.kind);
            if resolve::snapshot_of(snapshots, id).is_none() {
                return Err(format!(
                    "{} parses as a face but `{id}` is not in this package's registry; a file the \
                     build does not register cannot be edited by identity",
                    destination
                ));
            }
            return Ok(Target {
                path: destination.to_owned(),
                node: Some(id),
                rust: true,
                language: None,
            });
        }
        if let Some(face) = native.iter().find(|face| face.source == destination) {
            return Ok(Target {
                path: destination.to_owned(),
                node: Some(resolve::native_identity(&self.namespace, face)),
                rust: false,
                language: registration::registry()
                    .language_of_path(destination)
                    .cloned(),
            });
        }
        // A file an enabled adapter claims is editable by path: the adapter reads its own
        // declaration back, and a path is the only handle a language whose declarations are not
        // registered in the build tree has.
        // 启用的适配器认领的文件可以按路径编辑：适配器读回自己的声明，而一个声明尚未进构建树的语言，
        // 路径是它唯一的把手。
        if registration::registry()
            .language_of_path(destination)
            .is_some()
        {
            return Ok(Target {
                path: destination.to_owned(),
                node: None,
                rust: false,
                language: registration::registry()
                    .language_of_path(destination)
                    .cloned(),
            });
        }
        Err(format!(
            "{destination} is not a Rust face, no native declaration in this package names it, and \
             no enabled language adapter claims its extension"
        ))
    }
}

/// One resolved edit target: a package-relative path, the node it names when it has one, and the
/// language its extension selects.
/// 一个已解析的编辑目标：包内相对路径、它点名的节点（有时），以及扩展名选出的语言。
struct Target {
    /// The package-relative source path.
    /// 包内相对源码路径。
    path: String,
    /// The node, when the tree knows one for this file.
    /// 树为这个文件知道的节点（有时）。
    node: Option<NodeId>,
    /// Whether the target is a Rust face of the package's own registry.
    /// 目标是否是包自己注册树里的 Rust 面。
    rust: bool,
    /// The language the extension selects, when an adapter claims it.
    /// 扩展名选出的语言（当某个适配器认领它时）。
    language: Option<LanguageId>,
}

impl Target {
    fn is_rust(&self) -> bool {
        self.rust
    }
}

/// Turn one JSON object of face fields into the shared update vocabulary.
/// 把一个 JSON 字段对象转换成共享更新词汇。
///
/// Every surface that takes fields as JSON (the CLI and the MCP bridge) calls this, so a field
/// spelling, an array, or a boolean means the same thing on both. An unknown key is refused by name
/// rather than dropped — the direction the bridge's `add` used to get wrong.
/// 每个以 JSON 接收字段的执行面（CLI 与 MCP 桥）都调用它，因此字段拼法、数组或布尔值在两边含义相同。
/// 未知键按名拒绝而不是丢弃——正是桥的 `add` 过去弄错的方向。
#[cfg(any(feature = "cli", feature = "mcp", feature = "studio"))]
pub fn update_from_json(fields: &serde_json::Value) -> Result<RegistrationUpdate, String> {
    let object = fields
        .as_object()
        .ok_or_else(|| "`fields` must be a JSON object of registration-face fields".to_owned())?;
    let mut update = RegistrationUpdate::new();
    for (key, value) in object {
        let field = RegistrationField::parse_field(key)?;
        let text = match value {
            serde_json::Value::Null => FieldUpdate::Clear,
            serde_json::Value::String(text) => FieldUpdate::Set(text.as_str()),
            serde_json::Value::Bool(flag) => {
                if field != RegistrationField::NeedsRegistry {
                    return Err(format!(
                        "`{key}` is a text field; only `needs_registry` takes true or false"
                    ));
                }
                FieldUpdate::Set(if *flag { "true" } else { "false" })
            }
            serde_json::Value::Array(entries) => {
                let mut parts = Vec::new();
                for entry in entries {
                    let text = entry.as_str().ok_or_else(|| {
                        format!("`{key}` is a list of strings; one entry is not a string")
                    })?;
                    parts.push(text.to_owned());
                }
                FieldUpdate::Set(&*Box::leak(parts.join(",").into_boxed_str()))
            }
            serde_json::Value::Object(map) => {
                if field != RegistrationField::Requires {
                    return Err(format!(
                        "`{key}` is not a map; only `requires` maps a capability to its provider"
                    ));
                }
                let mut parts = Vec::new();
                for (capability, provider) in map {
                    let provider = provider.as_str().ok_or_else(|| {
                        format!("`requires.{capability}` must be a provider name")
                    })?;
                    parts.push(format!("{capability} => {provider}"));
                }
                FieldUpdate::Set(&*Box::leak(parts.join(",").into_boxed_str()))
            }
            serde_json::Value::Number(number) => {
                FieldUpdate::Set(&*Box::leak(number.to_string().into_boxed_str()))
            }
        };
        update.set_text(field, text)?;
    }
    Ok(update)
}

/// Plan and apply a create in one call, for a surface that does not need a preview.
/// 一次调用计划并落盘新建，供不需要预览的执行面使用。
pub fn create(
    service: &AuthoringService,
    request: &AuthoringRequest,
) -> Result<(FaceAuthoringPlan, AppliedAuthoring), String> {
    let plan = service.plan(request)?;
    let applied = service.apply(&plan)?;
    Ok((plan, applied))
}

#[cfg(test)]
#[path = "service_tests.rs"]
mod tests;
