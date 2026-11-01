//! File-backed add, edit, and delete operations.
//! 文件化的新增、编辑和删除操作。

use super::*;
use crate::std_source_tree::StdSourceTree;
#[path = "authored.rs"]
mod authored;
pub use authored::{AuthoredFace, authored_face};
#[path = "create.rs"]
mod create;
pub(crate) use create::{NewModulePlan, plan_new_face_from_spec};
use create::{create_module, plan_create_module};
#[path = "delete.rs"]
mod delete;
pub use delete::delete_module;
#[path = "trash.rs"]
mod trash;
use trash::stash_face_source;
#[path = "face_values.rs"]
mod face_values;
use face_values::ModuleFaceValues;
#[path = "face_write.rs"]
mod face_write;
use face_write::{FaceWrite, apply_module_face_values};
#[path = "migration.rs"]
mod migration;
use migration::{migrate_kind_subtree, migrate_module_subtree};
#[path = "paths.rs"]
mod paths;
use paths::generated_paths;
#[path = "parents.rs"]
pub(crate) mod parents;

/// The outcome of one file-backed authoring operation.
/// 一次文件化创作操作的结果。
pub struct AuthoringChange {
    /// A human-readable summary of what changed.
    /// 描述改动内容的人类可读摘要。
    pub message: String,
    /// The source file the change was written to.
    /// 改动写入的源码文件。
    pub source: PathBuf,
}

/// The complete field set for creating one module registration face.
/// 创建一个模块注册面所需的完整字段集合。
#[derive(Clone, Copy, Debug)]
pub struct NewModuleFace<'a> {
    /// New module directory and file name, and the face's identity path segment.
    /// 新模块的目录与文件名，同时是该注册面身份路径的一段。
    pub module: &'a str,
    /// Face kind name, for example `Button`.
    /// 注册面种类名，例如 `Button`。
    pub kind: &'a str,
    /// Preset type path that constructs this face.
    /// 构造本注册面的 preset 类型路径。
    pub preset: &'a str,
    /// Parts type path that supplies this face's construction parts.
    /// 提供本注册面构造 parts 的 parts 类型路径。
    pub parts: &'a str,
    /// Localized display name, Chinese half.
    /// 本地化显示名称的中文部分。
    pub name_zh: &'a str,
    /// Localized display name, English half.
    /// 本地化显示名称的英文部分。
    pub name_en: &'a str,
    /// Localized one-line description, Chinese half.
    /// 本地化单行描述的中文部分。
    pub summary_zh: &'a str,
    /// Localized one-line description, English half.
    /// 本地化单行描述的英文部分。
    pub summary_en: &'a str,
    /// Export names this face declares, as manifest text.
    /// 本注册面声明的导出名称，以清单文本给出。
    pub exports: &'a str,
    /// Optional author-owned identity that survives source moves; empty means none.
    /// 可选的作者逻辑身份，可跨源码移动保持不变；为空表示没有。
    pub stable_name: &'a str,
    /// Node identity of the face this one registers under.
    /// 本注册面所挂载到的父节点身份。
    pub parent: NodeId,
    /// Whether this face owns a child Registry.
    /// 本注册面是否拥有一个子注册机。
    pub needs_registry: bool,
    /// External registry this face is provisioned from; empty means none.
    /// 本注册面从其获取内容的外部注册机；为空表示没有。
    pub getting_from_other_registry: &'a str,
    /// Rule for faces entering the Registry this face owns.
    /// 进入本注册面所拥有 Registry 的注册规范。
    pub registration_rule: &'a str,
    /// External dependency gate for this face's registry.
    /// 本注册面所属注册机对外部依赖的门禁。
    pub admission: &'a str,
    /// Interface names declared by the handle type.
    /// handle 类型声明实现的接口名称。
    pub handle_traits: &'a str,
    /// Contract types the handle type must implement.
    /// handle 类型必须实现的合同类型。
    pub handle_contracts: &'a str,
    /// Interface names declared by the parts type.
    /// parts 类型声明实现的接口名称。
    pub part_traits: &'a str,
    /// Contract types the parts type must implement.
    /// parts 类型必须实现的合同类型。
    pub part_contracts: &'a str,
    /// Capability requirements this face declares.
    /// 本注册面声明的能力需求。
    pub requires: &'a str,
    /// Capability names this face makes available to other faces.
    /// 本注册面向其他注册面提供的能力名称。
    pub provides: &'a str,
    /// Runtime value checks the host applies; empty accepts any value.
    /// 宿主执行的运行期取值校验；为空时接受任何取值。
    pub runtime_checks: &'a str,
    /// Explicit flow contract for grafting; empty selects the default.
    /// 供嫁接使用的显式数据流合同；为空时选用默认值。
    pub flow: &'a str,
    /// Type that supplies the compile-time flow contract, when explicit.
    /// 显式提供编译期数据流合同的类型路径（如果显式给出）。
    pub flow_provider: &'a str,
}

/// The complete field set for editing one module registration face in place.
/// 原地编辑一个模块注册面所需的完整字段集合。
#[derive(Clone, Copy, Debug)]
pub struct ModuleFacePatch<'a> {
    /// New module directory/file name. Changing it migrates the whole subtree.
    /// 新模块目录和文件名；修改它会迁移整棵子树。
    pub module: &'a str,
    /// Face kind name, for example `Button`.
    /// 注册面种类名，例如 `Button`。
    pub kind: &'a str,
    /// Preset type path that constructs this face.
    /// 构造本注册面的 preset 类型路径。
    pub preset: &'a str,
    /// Parts type path that supplies this face's construction parts.
    /// 提供本注册面构造 parts 的 parts 类型路径。
    pub parts: &'a str,
    /// Localized display name, Chinese half.
    /// 本地化显示名称的中文部分。
    pub name_zh: &'a str,
    /// Localized display name, English half.
    /// 本地化显示名称的英文部分。
    pub name_en: &'a str,
    /// Localized one-line description, Chinese half.
    /// 本地化单行描述的中文部分。
    pub summary_zh: &'a str,
    /// Localized one-line description, English half.
    /// 本地化单行描述的英文部分。
    pub summary_en: &'a str,
    /// Export names this face declares, as manifest text.
    /// 本注册面声明的导出名称，以清单文本给出。
    pub exports: &'a str,
    /// Optional author-owned identity that survives source moves; empty means none.
    /// 可选的作者逻辑身份，可跨源码移动保持不变；为空表示没有。
    pub stable_name: &'a str,
    /// Whether this face owns a child Registry.
    /// 本注册面是否拥有一个子注册机。
    pub needs_registry: bool,
    /// External registry this face is provisioned from; empty means none.
    /// 本注册面从其获取内容的外部注册机；为空表示没有。
    pub getting_from_other_registry: &'a str,
    /// Rule for faces entering the Registry this face owns.
    /// 进入本注册面所拥有 Registry 的注册规范。
    pub registration_rule: &'a str,
    /// External dependency gate for this face's registry.
    /// 本注册面所属注册机对外部依赖的门禁。
    pub admission: &'a str,
    /// Interface names declared by the handle type.
    /// handle 类型声明实现的接口名称。
    pub handle_traits: &'a str,
    /// Contract types the handle type must implement.
    /// handle 类型必须实现的合同类型。
    pub handle_contracts: &'a str,
    /// Interface names declared by the parts type.
    /// parts 类型声明实现的接口名称。
    pub part_traits: &'a str,
    /// Contract types the parts type must implement.
    /// parts 类型必须实现的合同类型。
    pub part_contracts: &'a str,
    /// Capability requirements this face declares.
    /// 本注册面声明的能力需求。
    pub requires: &'a str,
    /// Capability names this face makes available to other faces.
    /// 本注册面向其他注册面提供的能力名称。
    pub provides: &'a str,
    /// Runtime value checks the host applies; empty accepts any value.
    /// 宿主执行的运行期取值校验；为空时接受任何取值。
    pub runtime_checks: &'a str,
    /// Explicit flow contract for grafting; empty selects the default.
    /// 供嫁接使用的显式数据流合同；为空时选用默认值。
    pub flow: &'a str,
    /// Type that supplies the compile-time flow contract, when explicit.
    /// 显式提供编译期数据流合同的类型路径（如果显式给出）。
    pub flow_provider: &'a str,
}

/// Create the standard `<parent>/object/<name>/<name>.rs` registration face.
/// 创建标准的 `<parent>/object/<name>/<name>.rs` 注册面。
pub fn add_module(registry: &Registry, spec: &str) -> Result<AuthoringChange, String> {
    add_module_with_registration(registry, spec).map(|(change, _)| change)
}

/// Create a module and return the same registration face for immediate commit.
/// 创建模块并返回同一注册面，以便立即提交到当前 Registry。
pub fn add_module_with_registration(
    registry: &Registry,
    spec: &str,
) -> Result<(AuthoringChange, RegistrationSnapshot), String> {
    let mut fields = spec.split_whitespace();
    let name = fields
        .next()
        .ok_or_else(|| "usage: add <module-name> [parent-node]".to_owned())?;
    validate_name(name)?;
    let parent = fields
        .next()
        .map(str::parse::<NodeId>)
        .transpose()
        .map_err(|_| "parent must be a 32-digit node identity".to_owned())?
        .unwrap_or(ROOT_NODE_ID);
    if fields.next().is_some() {
        return Err("usage: add <module-name> [parent-node]".to_owned());
    }

    create_module(registry, name, parent, None)
}

/// Validate and create a fully configured face in one filesystem transaction.
/// 一次校验并创建完整注册面，避免逐字段编辑留下半成品。
pub fn add_module_from_face(
    registry: &Registry,
    spec: &NewModuleFace<'_>,
) -> Result<(AuthoringChange, RegistrationSnapshot), String> {
    let values = ModuleFaceValues::from_new(spec);
    validate_name(values.module)?;
    create_module(registry, values.module, spec.parent, Some(&values))
}

/// Plan a fully configured create without writing it, for the shared authoring service.
/// 为共享创作服务计划一次完整配置的新建，不写下它。
pub(crate) fn plan_add_module_from_face(
    registry: &Registry,
    spec: &NewModuleFace<'_>,
    parent_module: Option<&str>,
    parent_base: xirang_multilang::source_ref::SourceBase,
) -> Result<NewModulePlan, String> {
    let values = ModuleFaceValues::from_new(spec);
    validate_name(values.module)?;
    plan_create_module(
        registry,
        values.module,
        spec.parent,
        parent_module,
        parent_base,
        Some(&values),
    )
}

/// Plan a field rewrite without writing it, refusing a structural rename by name.
/// 计划一次字段重写而不写下它；结构改名的请求按名拒绝。
///
/// A module or kind change rewrites paths and identities, which is the existing rename path's job
/// (`apply rename`), not a field edit. Refusing here is what keeps the plan an exact file edit
/// rather than a silent structural operation.
/// 改模块或 kind 会改写路径与身份，那是现有改名路径（`apply rename`）的职责，不是字段编辑。在这里
/// 拒绝，才能让计划保持为精确文件编辑，而不是静默的结构操作。
pub(crate) fn plan_rewrite_module_face(
    registry: &Registry,
    id: NodeId,
    patch: &ModuleFacePatch<'_>,
) -> Result<RewriteModulePlan, String> {
    let prepared = prepare_edit(registry, id, patch)?;
    if prepared.module_changed || prepared.kind_changed {
        return Err(
            "changing `module` or `kind` is a structural rename, not a field edit; use the              existing rename action, which migrates the subtree and its references"
                .to_owned(),
        );
    }
    plan_rewrite(prepared, registry, id)
}

/// Load authored faces into an owned snapshot that can be dropped after a reload.
/// 将创作注册面加载为可随热刷新释放的拥有所有权快照。
pub fn generated_snapshots() -> Result<Vec<RegistrationSnapshot>, String> {
    // The layout is the package's, not the scan root: resolving it from the scan root would make
    // this function answer about a different package (the one whose root is that directory).
    // 布局是包的那一份，不是扫描根的：从扫描根解析会让本函数回答另一个包（以那个目录为根的那个）。
    generated_snapshots_in(&crate::source_layout(&package_root())?)
}

/// Load generated registration faces from the layout the caller already resolved.
/// 从调用方已解析的布局加载生成的注册面。
///
/// The reader takes the **layout**, not a bare root: the discovery root and the identity base are
/// two different facts (`[lib] path = "source/lib.rs"` separates them), and the record spelling the
/// reader produces is relative to the identity base. A caller that only passes the scan root makes
/// this function re-resolve a layout whose package root is that scan root — a second, wrong answer.
/// 读取方收的是**布局**，不是一个裸根：发现根与身份基准是两个事实（`[lib] path = "source/lib.rs"`
/// 会把它们分开），而读取方产出的记录拼写相对身份基准。只传扫描根的调用方会让本函数把一个"包根就是
/// 扫描根"的布局重新解析出来——第二个、错误的答案。
pub fn generated_snapshots_in(
    layout: &crate::source_layout::SourceLayout,
) -> Result<Vec<RegistrationSnapshot>, String> {
    let identity_prefix = layout
        .scan_root
        .strip_prefix(&layout.identity_base)
        .map(|prefix| prefix.to_string_lossy().replace('\\', "/"))
        .unwrap_or_default();
    let identity_prefix = if identity_prefix.is_empty() {
        String::new()
    } else {
        format!("{identity_prefix}/")
    };
    let mut sources = Vec::new();
    visit_face_sources(&layout.scan_root, &mut sources)?;
    // The read-back walks the same discovery root the build walks; a face an explicit placement put
    // outside it is appended from the ledger, because otherwise a later authoring call cannot find
    // the face the previous one just wrote.
    // 读回走的是构建走的同一个发现根；显式落点放在它之外的面从台账补进来，否则下一次创作调用就找不到
    // 上一次刚写下的面。
    // Only declarations are read back as faces: a mapped directory also carries ordinary sources
    // (its registry rule, helper modules), and parsing those as faces is how a generated rule file
    // became "this module was not generated by XiRang".
    // 只有声明被当注册面读回：映射目录里还住着普通源码（它的注册规则、辅助模块），而把它们当注册面解析，
    // 正是生成的规则文件变成"this module was not generated by XiRang"的来路。
    sources.extend(
        crate::build_method::explicit_placement_files(&layout.package_root, &layout.scan_root)
            .into_iter()
            .filter(|source| {
                fs::read_to_string(source).is_ok_and(|text| {
                    crate::run_method::syntax::is_face_source(&text, GENERATED_MARKER)
                })
            }),
    );
    sources.sort();
    sources.dedup();
    // The parent facts are the package's discovered tree, built once: a child's parent is resolved
    // from **this** table instead of from the child's module spelling.
    // 父级事实是本包已发现的树，只建一次：子面的父级从**这张表**解析，而不是从子面的模块拼写回推。
    let parents = parents::facts_of(layout, &sources)?;
    sources
        .into_iter()
        .map(|source| {
            FaceManifest::parse_source(&source, &identity_prefix, Some(&parents))
                .and_then(|face| face.to_snapshot())
                .map_err(|error| format!("{}: {error}", source.display()))
        })
        .collect()
}

/// Load generated registration faces from a project root whose layout is resolved here.
/// 从一个在此解析布局的项目根加载生成的注册面。
pub fn generated_snapshots_from(root: &Path) -> Result<Vec<RegistrationSnapshot>, String> {
    generated_snapshots_in(&crate::source_layout(root)?)
}

pub(super) fn visit_face_sources(
    directory: &Path,
    sources: &mut Vec<PathBuf>,
) -> Result<(), String> {
    xirang_kernel::source::collect_rust_sources(
        &StdSourceTree,
        directory,
        xirang_kernel::source::SourceWalk {
            skip_target: false,
            skip_registry_core: true,
            skip_compile_error_demo: true,
        },
        |_, source| match source {
            None => xirang_kernel::source::Keep::NeedSource,
            Some(text) if crate::run_method::syntax::is_face_source(text, GENERATED_MARKER) => {
                xirang_kernel::source::Keep::Yes
            }
            Some(_) => xirang_kernel::source::Keep::No,
        },
        sources,
    )
}

/// Edit all Studio-owned face fields in one filesystem transaction.
/// 在一次文件事务中编辑 Studio 管理的全部注册面字段。
///
/// This is the only authored-face edit entry point. The string-parsing
/// `edit_module` was deleted in B3a: it edited the manifest but never wrote or
/// removed the registry-rule source, so `edit <id> needs_registry true` left a
/// face referencing a `super::registry_rule` module it did not own. The doctest
/// below pins the deletion at compile time.
/// 这是唯一的注册面编辑入口。字符串解析式的 `edit_module` 已在 B3a 删除：它只改清单，
/// 从不写入或删除注册规则源，于是 `edit <id> needs_registry true` 会留下一个引用着
/// 自己并不拥有的 `super::registry_rule` 模块的注册面。下面的 doctest 在编译期钉住
/// 这次删除。
///
/// ```compile_fail,E0433
/// let _ = crate::run_method::edit_module;
/// ```
pub fn edit_module_face(
    registry: &Registry,
    id: NodeId,
    patch: &ModuleFacePatch<'_>,
) -> Result<AuthoringChange, String> {
    let prepared = prepare_edit(registry, id, patch)?;
    if prepared.module_changed {
        let requested = prepared.requested_module.clone();
        return migrate_module_subtree(registry, id, prepared.source, prepared.face, &requested);
    }
    if prepared.kind_changed {
        return migrate_kind_subtree(registry, id, prepared.source, prepared.face);
    }
    apply_rewrite(plan_rewrite(prepared, registry, id)?)
}

/// One edit after its fields were applied and before anything was rendered or written.
/// 一个编辑在字段已应用、尚未渲染或写入时的状态。
pub(crate) struct PreparedEdit {
    pub(crate) face: FaceManifest,
    pub(crate) source: PathBuf,
    pub(crate) module_changed: bool,
    pub(crate) kind_changed: bool,
    pub(crate) requested_module: String,
}

/// Apply a patch to the parsed face and decide whether the edit is structural.
/// 把补丁应用到已解析的注册面，并判断这次编辑是否属于结构操作。
pub(crate) fn prepare_edit(
    registry: &Registry,
    id: NodeId,
    patch: &ModuleFacePatch<'_>,
) -> Result<PreparedEdit, String> {
    let (_, source) = generated_paths(registry, id)?;
    let identity_prefix = super::context::identity_prefix()?;
    let layout = super::context::layout()?;
    let parents = parents::facts_in(&layout)?;
    let mut face = FaceManifest::parse_source(&source, &identity_prefix, Some(&parents))?;
    let values = ModuleFaceValues::from_patch(patch);
    let old_module = face.values.get("module").cloned().unwrap_or_default();
    let requested_module = values.module.trim().to_owned();
    validate_name(&requested_module)?;
    let module_changed = requested_module != old_module;
    let original_kind = face.values.get("kind").cloned().unwrap_or_default();
    let kind_changed = normalize_kind_name(values.kind.trim()) != original_kind;
    apply_module_face_values(&mut face, &values, FaceWrite::Edit)?;
    if !values.needs_registry
        && registry
            .registry(id)
            .is_some_and(|owned_registry| !owned_registry.is_empty())
    {
        return Err("cannot disable a Registry while it still owns child entries".to_owned());
    }
    Ok(PreparedEdit {
        face,
        source,
        module_changed,
        kind_changed,
        requested_module,
    })
}

/// One file change a rewrite would make, before the transaction writes it.
/// 一次重写会做的一处文件变更，在事务写入之前。
pub(crate) enum RuleChange {
    /// The face owns no rule source and had none.
    /// 该面不拥有规则源，过去也没有。
    Keep,
    /// Write this rule source.
    /// 写入这份规则源。
    Write(PathBuf, String),
    /// Remove this rule source.
    /// 移除这份规则源。
    Remove(PathBuf),
}

/// Everything a rewrite would write, before anything is written.
/// 一次重写会写下的全部内容，在写下任何东西之前。
pub(crate) struct RewriteModulePlan {
    pub(crate) id: NodeId,
    pub(crate) source: PathBuf,
    pub(crate) old_source: String,
    pub(crate) rendered: String,
    pub(crate) rule: RuleChange,
}

/// Render the rewrite without touching the source tree.
/// 渲染这次重写，不触碰源码树。
pub(crate) fn plan_rewrite(
    prepared: PreparedEdit,
    registry: &Registry,
    id: NodeId,
) -> Result<RewriteModulePlan, String> {
    let PreparedEdit {
        mut face, source, ..
    } = prepared;
    // Parse the complete edited face before touching either file.
    // 在修改任一文件前先解析完整注册面，保证错误编辑不会落盘。
    let authored = face.to_snapshot()?;
    let existing = registry
        .find_registry(id)
        .ok_or_else(|| format!("node `{id}` is not registered"))?;
    let merged = existing.clone().merge_authored(authored);
    registry
        .validate_snapshot_replacement(id, merged)
        .map_err(|error| format!("registration rejected:\n{error}"))?;
    let old_source = fs::read_to_string(&source)
        .map_err(|error| format!("cannot read {}: {error}", source.display()))?;
    let rule = face.rule_source_path()?;
    let old_rule = fs::read_to_string(&rule).ok();
    face.set_identity(id);
    let rendered = xirang_kernel::syntax::replace_face_region(&old_source, &face.render_source()?)
        .map_err(|error| error.to_string())?;
    let rendered = if old_source.lines().any(|line| line == GENERATED_MARKER) {
        rendered
    } else {
        format!("{GENERATED_MARKER}\n{rendered}")
    };
    let rule = if face.owns_rule_source() {
        let rule_source = face
            .render_rule_source()?
            .ok_or_else(|| "registry rule source was unexpectedly omitted".to_owned())?;
        RuleChange::Write(rule, rule_source)
    } else if old_rule.is_some() {
        RuleChange::Remove(rule)
    } else {
        RuleChange::Keep
    };
    Ok(RewriteModulePlan {
        id,
        source,
        old_source,
        rendered,
        rule,
    })
}

/// Write a planned rewrite, restoring the previous bytes on failure.
/// 落盘一次已计划的重写；失败时恢复先前的字节。
pub(crate) fn apply_rewrite(plan: RewriteModulePlan) -> Result<AuthoringChange, String> {
    let RewriteModulePlan {
        id,
        source,
        old_source,
        rendered,
        rule,
    } = plan;
    // Preserve the complete previous declaration before changing it, even though surrounding
    // implementation stays in place. No-op saves create no backup.
    // 修改前备份完整旧声明，同时保留周围实现；没有实质变化的保存不创建备份。
    // **The write scope is the same rule for every writer**, not only for the shared service: this
    // executor is still called by Studio's edit form and by the CLI, and a face whose source is a
    // declared (read-only) source resolves inside the package while its **real** location is outside.
    // 写入作用域对**每个**写入方都是同一条规则，不只对共享服务：这个执行器仍被 Studio 的编辑表单与 CLI
    // 调用，而源码住在已声明（只读）来源里的面，包内拼写成立、**真实**位置却在包外。
    {
        use crate::run_method::authoring::write_scope::{
            Change, ChangeKind, WriteScope, describe_outside,
        };
        let root = super::super::context::package_root();
        let mut changes = vec![Change {
            path: source.clone(),
            kind: ChangeKind::Write,
            subject: format!("the declaration `{}`", source.display()),
        }];
        match &rule {
            RuleChange::Write(path, _) => changes.push(Change {
                path: path.clone(),
                kind: ChangeKind::Write,
                subject: format!("the registry rule `{}`", path.display()),
            }),
            RuleChange::Remove(path) => changes.push(Change {
                path: path.clone(),
                kind: ChangeKind::Remove,
                subject: format!("the registry rule `{}`", path.display()),
            }),
            RuleChange::Keep => {}
        }
        if let Err(outside) = WriteScope::new(&root)?.authorize(&changes) {
            return Err(format!(
                "this edit would write outside the package `{}`, so nothing was written.\n{}",
                root.display(),
                describe_outside(&outside, &root)
            ));
        }
    }
    let backup = if rendered == old_source {
        None
    } else {
        Some(stash_face_source(&source, id, &old_source)?)
    };
    // One transaction owns the declaration and the registry rule it owns: the rule write used to
    // restore the declaration with `let _ = atomic_write(...)`, so a failed restore was reported as
    // a clean tree.
    // 同一个事务拥有声明与该声明拥有的注册规则：规则写入过去用 `let _ = atomic_write(...)` 恢复声明，
    // 于是一次失败了的恢复会被报成干净的树。
    let mut transaction = super::super::write_transaction::WriteTransaction::new();
    let outcome = (|| -> Result<(), String> {
        transaction.write_atomic(&source, rendered.as_bytes())?;
        match &rule {
            RuleChange::Keep => {}
            RuleChange::Write(path, text) => {
                transaction.write_atomic(path, text.as_bytes())?;
            }
            RuleChange::Remove(path) => {
                transaction.remove(path)?;
            }
        }
        Ok(())
    })();
    if let Err(error) = outcome {
        let recovery = transaction.rollback();
        return Err(format!("{error}; {}", recovery.describe()));
    }
    Ok(AuthoringChange {
        message: match &backup {
            Some(backup) => format!(
                "updated registration face {} (previous text kept at {})",
                source.display(),
                backup.display()
            ),
            None => format!("updated registration face {}", source.display()),
        },
        source,
    })
}
