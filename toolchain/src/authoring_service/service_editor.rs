//! Editor material and native placeholders for the shared authoring service.
//! 共享创作服务的编辑器资料与本地面占位。

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use xirang_kernel::authoring::{
    DiagnosticSeverity, FaceAuthoringPlan, PlanDiagnostic, RegistrationUpdate,
};
use xirang_kernel::declaration::{
    Admission, OwnedLocalizedText, OwnedObjectContract, OwnedRegistrationRule, OwnedSourceLocation,
    RegistrationSnapshot,
};
use xirang_kernel::identity::NodeId;
use xirang_kernel::tree::Registry;
use xirang_kernel::{FrameworkId, OwnedFlowContract};

use crate::build_method::{declared_native, face_header, source_module_path};
use crate::run_method::{AuthoredFace, AuthoringContext};
use crate::source_layout::SourceLayout;

use crate::run_method::authoring::write_scope::WriteScope;

use super::{resolve, rust_adapter};
use xirang_multilang::registration;

use super::AuthoringService;

impl AuthoringService {
    /// Read one file back into the shared field vocabulary, for a round-trip check.
    /// 把一个文件读回共享字段词汇，供往返检查。
    pub fn read_registration(&self, file: &str) -> Result<RegistrationUpdate, String> {
        let path = self.absolute(file)?;
        if file.ends_with(".rs") {
            let face = AuthoredFace::read_source(&path)?;
            return Ok(rust_adapter::read_update(&face));
        }
        let registry = registration::registry();
        let language = registry.language_of_path(file).ok_or_else(|| {
            let enabled = registry
                .languages()
                .iter()
                .map(|language| format!("`{}`", language.as_str()))
                .collect::<Vec<_>>();
            if enabled.is_empty() {
                format!("`{file}` is not a Rust face and no language adapter is enabled")
            } else {
                format!(
                    "`{file}` is not a Rust face and no enabled adapter claims its extension; \
                     enabled languages are {}",
                    enabled.join(", ")
                )
            }
        })?;
        let adapter = registry
            .adapter(language)
            .ok_or_else(|| format!("`{}` is not enabled in this build", language.as_str()))?;
        let text = std::fs::read_to_string(&path)
            .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
        Ok(adapter.read(&text)?.registration)
    }

    /// Apply the editor material the language adapter plans.
    /// 应用语言适配器规划的编辑器资料。
    ///
    /// The surface does not decide *which* file an editor needs or what goes in it; it asks the
    /// adapter, then applies the answer through the same plan. A file that already exists is left
    /// untouched — it may be a user's edit — and the skip is reported instead of hidden.
    /// 执行面不决定编辑器需要**哪个**文件、里面写什么；它问适配器，再把答案并进同一份计划。已存在的文件
    /// 保持不动（可能是用户改过的），并把这次跳过说出来，而不是藏起来。
    pub(super) fn with_editor_material(
        &self,
        mut plan: FaceAuthoringPlan,
        adapter: &dyn xirang_multilang::registration::LanguageAdapter,
        layout: &SourceLayout,
    ) -> FaceAuthoringPlan {
        let context = xirang_multilang::registration::EditorContext {
            package_root: layout.package_root.clone(),
            out_dir: self.root.join("target/xirang/out"),
        };
        for file in adapter.editor_material(&context) {
            if self.root.join(&file.path).exists() {
                plan.diagnostics.push(PlanDiagnostic {
                    severity: DiagnosticSeverity::Note,
                    message: format!(
                        "editor material `{}` already exists and was left unchanged; delete it to \
                         regenerate it",
                        file.path
                    ),
                });
                continue;
            }
            plan.files.push(file);
        }
        plan
    }

    /// Placeholder snapshots for the faces the package declares in native sources.
    /// 包在本地源码里声明的面的占位快照。
    ///
    /// They carry exactly what the declaration text carries — identity, parent, name, ownership —
    /// and nothing the compiler record would add. The plan never writes them; they exist so the
    /// staged registry can answer "does this parent exist" the way the build will.
    /// 它们只携带声明文本携带的东西——身份、父、名字、归属——以及编译记录才会加的东西一概不携带。计划
    /// 从不写它们；它们的存在只是让暂存注册树能像构建那样回答"这个父存在吗"。
    /// Native placeholders, built from the Rust snapshots **before** those snapshots are registered.
    /// 本地占位快照，在 Rust 快照注册**之前**就从它们建出来。
    ///
    /// The order is load-bearing: a Rust child of a native parent names it through `xirang_external`,
    /// so the native placeholder must be in the staged registry before the child is validated — and
    /// the placeholder itself needs the Rust faces' module/identity facts to resolve a *native* child
    /// of a *Rust* parent. Passing the snapshot facts in (rather than a registry) is what makes both
    /// directions possible without one failing on the other.
    /// 顺序是关键：挂在本地父级下的 Rust 子面经 `xirang_external` 点名它，因此本地占位必须在子面被校验
    /// **之前**进入暂存注册树——而占位本身又需要 Rust 面的模块/身份事实，才能解析"Rust 父下的本地子面"。
    /// 把快照事实传进来（而不是传一个注册树），正是让两个方向都成立、不会互相把对方挡在门外的原因。
    pub(super) fn synthetic_native_snapshots(
        &self,
        snapshots: &[RegistrationSnapshot],
        native: &[declared_native::DeclaredFace],
    ) -> Result<Vec<RegistrationSnapshot>, String> {
        let mut known: BTreeMap<String, (String, NodeId)> = BTreeMap::new();
        for face in snapshots {
            let module = source_module_path(&face.source.file);
            known.insert(
                face_header::identifier(&module, &face.kind),
                (module, face.id),
            );
        }
        // A C++ declaration generated by the shared authoring service names its parent through
        // `crate::catalog::…`, which is the registry-name chain rather than the module chain. Seed
        // those paths too, or the next edit of that declaration cannot read itself back.
        // 共享创作服务生成的 C++ 声明经 `crate::catalog::…` 点名父，而那是注册名链、不是模块链。把
        // 这些路径也预置进去，否则下一次编辑读不回自己写下的声明。
        let names: Vec<(NodeId, NodeId, String)> = snapshots
            .iter()
            .map(|face| (face.id, face.parent, face.registry_name.clone()))
            .collect();
        let paths = crate::build_method::face_view::logical_paths(
            xirang_kernel::identity::root_node_id(&self.namespace),
            &names,
        );
        for face in snapshots {
            if let Some(path) = paths
                .get(&face.id)
                .and_then(|path| path.strip_prefix("root/"))
            {
                let path = path.replace('/', "::");
                known.insert(path.clone(), (path, face.id));
            }
        }
        {
            let mut live = snapshots
                .iter()
                .map(|face| {
                    (
                        face.source.file.clone(),
                        source_module_path(&face.source.file),
                        face.id,
                    )
                })
                .collect::<Vec<_>>();
            live.extend(native.iter().map(|face| {
                (
                    face.source.clone(),
                    declared_native::module_of(&face.source),
                    resolve::native_identity(&self.namespace, face),
                )
            }));
            crate::build_method::compatibility::extend_known(&mut known, &self.root, &live);
        }
        // The view is the build's own module/catalog relation. Seed both spellings from it: a
        // snapshot list can be empty before the first build, while the sources already name faces.
        // 视图是构建自己的模块/目录关系。从它预置两种拼法：首次构建前快照可能为空，而源码已经声明了面。
        if let Ok(views) = crate::build_method::face_views(&self.root, &self.namespace) {
            for view in views {
                known.insert(view.module.clone(), (view.module.clone(), view.id));
                if let Some(path) = view.path.strip_prefix("root/") {
                    let path = path.replace('/', "::");
                    known.insert(path.clone(), (path, view.id));
                }
            }
        }
        // Last-resort alias for a face whose module chain is known but whose catalog path is not
        // (a first edit before any build has no view). Only a **unique** leaf becomes an alias: a
        // name shared by two modules stays unresolved rather than guessing which object it means.
        // 对已知模块链、但还没有构建视图（首次编辑）的面做最后兜底别名。只有**唯一**的叶子才成为别名：
        // 两个模块共用的名字保持未解析，而不是替它猜是哪个对象。
        let mut leaves: std::collections::BTreeMap<String, std::collections::BTreeSet<NodeId>> =
            std::collections::BTreeMap::new();
        for (module, id) in known.values() {
            if let Some(leaf) = module.rsplit("::").next() {
                leaves.entry(leaf.to_owned()).or_default().insert(*id);
            }
        }
        for (leaf, ids) in leaves {
            if ids.len() == 1 {
                let id = *ids.iter().next().expect("one id");
                known.entry(leaf.clone()).or_insert((leaf.clone(), id));
            }
        }
        if std::env::var_os("XIRANG_DEBUG_KNOWN").is_some() {
            eprintln!("KNOWN={known:?}");
        }
        let resolved = declared_native::resolve(&self.namespace, native, &known)?;
        Ok(resolved
            .into_iter()
            .map(|face| {
                let registry_name = Path::new(&face.source)
                    .file_stem()
                    .and_then(|stem| stem.to_str())
                    .unwrap_or(face.name.as_str())
                    .to_owned();
                RegistrationSnapshot {
                    namespace: self.namespace.clone(),
                    id: face.id,
                    parent: face.parent,
                    kind: face.name.clone(),
                    preset: "NoPreset".to_owned(),
                    parts: "NoParts".to_owned(),
                    params: String::new(),
                    handle: face.name.clone(),
                    stable_name: None,
                    name: OwnedLocalizedText {
                        zh: face.name.clone(),
                        en: face.name.clone(),
                    },
                    summary: OwnedLocalizedText {
                        zh: String::new(),
                        en: String::new(),
                    },
                    exports: Vec::new(),
                    needs_registry: face.owns_registry,
                    registry_name,
                    getting_from_other_registry: None,
                    registry_rule_path: String::new(),
                    registry_rule: OwnedRegistrationRule {
                        required_preset: None,
                        required_parts: Vec::new(),
                        required_exports: Vec::new(),
                        required_handle_traits: Vec::new(),
                        required_part_traits: Vec::new(),
                    },
                    admission: Admission::ANY.into_owned(),
                    requires: Vec::new(),
                    provides: Vec::new(),
                    contract: OwnedObjectContract {
                        required_parts: Vec::new(),
                        provided_parts: Vec::new(),
                    },
                    flow: OwnedFlowContract::none(),
                    flow_provider: None,
                    handle_traits: Vec::new(),
                    part_traits: Vec::new(),
                    runtime_checks: Vec::new(),
                    plugin: None,
                    source: OwnedSourceLocation {
                        file: face.source.clone(),
                        line: 0,
                        column: 0,
                        function: face.name.clone(),
                    },
                }
            })
            .collect())
    }

    pub(super) fn absolute(&self, relative: &str) -> Result<PathBuf, String> {
        let path = Path::new(relative);
        if path.is_absolute()
            || path
                .components()
                .any(|component| component.as_os_str() == "..")
        {
            return Err(format!(
                "`{relative}` is not a package-relative path; a plan may only write inside {}",
                self.root.display()
            ));
        }
        let absolute = self.root.join(path);
        // The **real** location decides, not the spelling — through the one write-scope rule every
        // writer in this crate uses (`authoring::write_scope`). A declared source is a place a read
        // may enter (a legitimate materialization can be a link), but a declaration is not a write
        // permission: writing through the link would put bytes outside the package.
        // 由**真实**位置决定，而不是拼写——经本 crate 每个写入方都用的那一条写入作用域规则
        // （`authoring::write_scope`）。已声明来源是一次读取可以进入的地方，但声明不是写权限：
        // 穿过链接写会把字节放到包外。
        let scope = WriteScope::new(&self.root)?;
        if !scope.inside(&absolute)? {
            return Err(format!(
                "`{relative}` resolves outside {}; a declared source is a place a read may enter, \
                 not a place a plan may write",
                self.root.display()
            ));
        }
        Ok(absolute)
    }

    pub(super) fn load_registry(
        &self,
        layout: &SourceLayout,
    ) -> Result<(Registry, Vec<RegistrationSnapshot>), String> {
        let snapshots = AuthoringContext::new(self.root.clone(), self.namespace.clone())
            .scope(|| crate::run_method::generated_snapshots_in(layout))?;
        let mut registry =
            Registry::root_for_namespace(FrameworkId::new("xirang.authoring"), &self.namespace);
        // The build registers foreign faces from their compiled records, which authoring does not
        // have. It does have the declarations, so it registers a placeholder with the same identity
        // and ownership for the staged validation only — and it does so **first**, because a Rust
        // child of a native parent names that parent through `xirang_external` and would otherwise be
        // validated against a registry that does not have it yet (the reproduced follow-up failure:
        // one Rust-under-C++ add, then every later authoring call on that package failed).
        // 构建从编译记录登记外语面，而创作侧没有记录。它有声明，因此只为暂存校验登记一个身份与归属相同的
        // 占位——而且**先**登记，因为挂在本地父级下的 Rust 子面经 `xirang_external` 点名那个父级，否则
        // 它会被拿去跟一棵还没有那个父的注册树校验（已复现的后续失败：一次 Rust-under-C++ 的 add 之后，
        // 对该包的每次后续创作调用都失败）。
        let native = resolve::native_faces(&self.root)?;
        let mut batch = Vec::new();
        if !native.is_empty() {
            batch = self.synthetic_native_snapshots(&snapshots, &native)?;
        }
        // **One batch, not two.** The two kinds of face name each other: a Rust child of a native
        // parent (`xirang_external`) and a native child of a Rust parent (`faces::<module>`). Two
        // sequential batches can only satisfy one direction — the other is validated before its
        // parent exists — while a single batch lets the kernel register parents before children
        // (it sorts by parent and stages) whichever way the edges go.
        // **一个批次，不是两个。** 两类面互相点名：本地父下的 Rust 子面（`xirang_external`）与 Rust 父下的
        // 本地子面（`faces::<module>`）。两个先后批次只能满足一个方向——另一个会在父存在之前被校验——而单个
        // 批次让内核按父先于子的次序登记（它按父排序并暂存），无论边指向哪一边。
        batch.extend(snapshots.clone());
        registry.register_snapshot_batch(batch).map_err(|error| {
            format!("the package's declarations were rejected by the staged registry: {error}")
        })?;
        Ok((registry, snapshots))
    }
}
