//! Owned (heap-backed) twins of the declaration vocabulary.
//! 声明词表的 owned（堆分配）孪生类型。

use super::*;

impl fmt::Display for OwnedSourceLocation {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{}:{}:{}",
            portable_path(&self.file),
            self.line,
            self.column
        )
    }
}

impl OwnedSourceLocation {
    /// The declaration's source path with `/` separators on every platform.
    /// 该声明的源码路径，在任何平台上都以 `/` 分隔。
    pub fn portable_file(&self) -> String {
        portable_path(&self.file)
    }

    /// Render the source location with its logical function name.
    /// 渲染带逻辑函数名的源码位置。
    pub fn describe(&self) -> String {
        format!("{} function={}", self, self.function)
    }
}

/// Owned twin of [`LocalizedText`] for reloadable declarations.
/// [`LocalizedText`] 的 owned 孪生，供可热重载声明使用。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OwnedLocalizedText {
    /// Label text shown to Chinese readers.
    /// 面向中文读者的标签文本。
    pub zh: String,
    /// Label text shown to English readers.
    /// 面向英文读者的标签文本。
    pub en: String,
}

/// Owned twin of [`RequirementSpec`] for reloadable declarations.
/// [`RequirementSpec`] 的 owned 孪生，供可热重载声明使用。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OwnedRequirementSpec {
    /// Capability name this declaration requires.
    /// 本声明所要求的能力名称。
    pub capability: String,
    /// Object expected to provide that capability.
    /// 本应提供该能力的对象。
    pub provider: String,
}

/// Owned twin of [`Admission`], the external-dependency gate.
/// [`Admission`] 的 owned 孪生：外部依赖门禁。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OwnedAdmission {
    /// Path prefixes this gate admits; empty admits every path.
    /// 本门禁允许的路径前缀；为空表示允许任何路径。
    pub allowed_paths: Vec<String>,
    /// Path prefixes this gate rejects before consulting the allow list.
    /// 在查看允许列表之前即被拒绝的路径前缀。
    pub denied_paths: Vec<String>,
}

/// Owned twin of [`RegistrationRule`], the structural entry rule.
/// [`RegistrationRule`] 的 owned 孪生：注册面的结构入门规范。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OwnedRegistrationRule {
    /// Preset name the face must declare, when one is required.
    /// 注册面必须声明的 preset 名称（若有要求）。
    pub required_preset: Option<String>,
    /// Structural parts the face must provide.
    /// 注册面必须提供的结构 parts。
    pub required_parts: Vec<String>,
    /// Export names the face must declare.
    /// 注册面必须声明的导出名称。
    pub required_exports: Vec<String>,
    /// Interfaces the handle type must implement, for example `ControlHandle`.
    /// handle 类型必须实现的接口，例如 `ControlHandle`。
    pub required_handle_traits: Vec<String>,
    /// Interfaces the parts type must implement, for example `ActionParts`.
    /// parts 类型必须实现的接口，例如 `ActionParts`。
    pub required_part_traits: Vec<String>,
}

/// Owned twin of [`ObjectContract`], the construction contract.
/// [`ObjectContract`] 的 owned 孪生：构造合同。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OwnedObjectContract {
    /// Construction parts the preset requires.
    /// preset 要求的构造 parts。
    pub required_parts: Vec<String>,
    /// Construction parts the parts type supplies.
    /// parts 类型实际提供的构造 parts。
    pub provided_parts: Vec<String>,
}

impl OwnedAdmission {
    /// Check an external path without borrowing static declaration data.
    /// 不借用静态声明数据，直接检查外部依赖路径。
    ///
    /// The rule itself lives beside its static twin (`registration::admits`): one gate with two
    /// spellings must not be two gates.
    /// 规则本身住在静态孪生旁边（`registration::admits`）：同一道门禁的两种拼法不能变成两道门禁。
    pub fn accepts(&self, path: &str) -> bool {
        super::registration::admits(&self.allowed_paths, &self.denied_paths, path)
    }
}

impl OwnedRegistrationRule {
    /// Validate all structural requirements and return every failure.
    /// 校验全部结构要求，并一次返回所有失败项。
    pub fn validate_owned_registration_rule(&self, snapshot: &RegistrationSnapshot) -> Vec<String> {
        validate_registration_requirements(RegistrationRequirementCheck {
            required_preset: self.required_preset.as_deref(),
            required_parts: &self.required_parts,
            required_exports: &self.required_exports,
            required_handle_traits: &self.required_handle_traits,
            required_part_traits: &self.required_part_traits,
            preset: &snapshot.preset,
            provided_parts: &snapshot.contract.provided_parts,
            exports: &snapshot.exports,
            handle: &snapshot.handle,
            handle_traits: &snapshot.handle_traits,
            parts: &snapshot.parts,
            part_traits: &snapshot.part_traits,
        })
    }
}

impl OwnedObjectContract {
    /// Validate the relationship between required and supplied parts.
    /// 校验所需 parts、实际提供 parts 以及返回值合同。
    pub fn validate_owned_object_contract(&self, object: &str) -> Vec<String> {
        validate_object_contract(&self.required_parts, &self.provided_parts, object)
    }
}

/// Owned declaration location used by reloadable registration faces.
/// 可重载注册面使用的自有声明位置。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OwnedSourceLocation {
    /// Source path recorded at the declaration site.
    /// 声明点记录的源码路径。
    pub file: String,
    /// One-based line number of the declaration.
    /// 声明所在的行号（从 1 开始）。
    pub line: u32,
    /// One-based column number of the declaration.
    /// 声明所在的列号（从 1 开始）。
    pub column: u32,
    /// The registration-face handle or logical function associated with this declaration.
    /// 与该注册面声明关联的 handle 或逻辑函数名。
    pub function: String,
}

/// Owned registration metadata used by file-backed authoring and reloads.
/// 文件创作与热刷新使用的拥有所有权注册元数据。
///
/// Compile-time declarations keep `RegistrationInfo` as a small static value.
/// Runtime-authored values must not borrow from a process-global intern pool,
/// so this type owns every reloadable field and can be dropped with the
/// snapshot that produced it.
/// 编译期声明继续使用小型静态 `RegistrationInfo`；运行时创作值不能借用进程级
/// intern 池，因此这里拥有所有可重载字段，并随快照一起释放。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RegistrationSnapshot {
    /// Package namespace that owns this declaration.
    /// 拥有本声明的包命名空间。
    pub namespace: String,
    /// Stable node id assigned to this declaration.
    /// 分配给本声明的稳定节点 id。
    pub id: NodeId,
    /// Node id of the registry owner this declaration is mounted under.
    /// 本声明挂载到的注册机拥有者的节点 id。
    pub parent: NodeId,
    /// Face kind name the declaration was authored with.
    /// 声明书写时使用的注册面种类名。
    pub kind: String,
    /// Preset name selected for construction.
    /// 构造时所选的 preset 名称。
    pub preset: String,
    /// Parts type name supplying the construction shape.
    /// 提供构造形状的 parts 类型名。
    pub parts: String,
    /// Params type name carrying this face's parameters.
    /// 承载该注册面参数的 params 类型名。
    pub params: String,
    /// Handle type name owning this face's runtime state.
    /// 拥有该注册面运行期状态的 handle 类型名。
    pub handle: String,
    /// Optional author-owned identity that survives source moves.
    /// 可选的作者逻辑身份，可跨源码文件移动保持不变。
    pub stable_name: Option<String>,
    /// Bilingual display name of the face.
    /// 注册面的双语显示名称。
    pub name: OwnedLocalizedText,
    /// Bilingual one-line summary of the face.
    /// 注册面的双语单行摘要。
    pub summary: OwnedLocalizedText,
    /// Export names this face publishes.
    /// 本注册面公开的导出名称。
    pub exports: Vec<String>,
    /// Whether this face owns a registry others may register into.
    /// 本注册面是否拥有一个可供他人注册的注册机。
    pub needs_registry: bool,
    /// The registry name this face is registered under, defaulting to the kind. It
    /// is not an overlay slot: `slot` is the graft cut a plan targets, and the
    /// plugin host's plugin slot.
    /// 本注册面所注册到的注册面名，默认为 kind。它不是叠加层的槽位：`slot` 指计划针对的
    /// graft 切口，以及插件宿主的插件槽。
    pub registry_name: String,
    /// Name of the external registry this face takes its dependency from.
    /// 本注册面从哪个外部注册机取得依赖的名称。
    pub getting_from_other_registry: Option<String>,
    /// Source path of the declaration that supplied the registry rule.
    /// 提供注册规范的声明所在源码路径。
    pub registry_rule_path: String,
    /// Rule for faces entering the registry owned by this face.
    /// 进入本注册面所拥有的注册机时必须满足的规范。
    pub registry_rule: OwnedRegistrationRule,
    /// External dependency gate for this face's registry.
    /// 本注册面所属注册机对外部依赖的门禁。
    pub admission: OwnedAdmission,
    /// Capabilities this face requires from its dependencies.
    /// 本注册面要求其依赖提供的能力。
    pub requires: Vec<OwnedRequirementSpec>,
    /// Names of the capabilities this face provides.
    /// 本注册面所提供能力的名称。
    pub provides: Vec<String>,
    /// Construction contract between preset and parts.
    /// preset 与 parts 之间的构造合同。
    pub contract: OwnedObjectContract,
    /// Automatically comparable input/output contract for grafting.
    /// 供嫁接自动比较的输入/输出合同。
    pub flow: crate::OwnedFlowContract,
    /// Type that supplied the compile-time flow contract, when explicit.
    /// 显式提供编译期数据流合同的类型路径（如果有）。
    pub flow_provider: Option<String>,
    /// Interface names declared by the handle type.
    /// handle 类型声明实现的接口名称。
    pub handle_traits: Vec<String>,
    /// Interface names declared by the parts type.
    /// parts 类型声明实现的接口名称。
    pub part_traits: Vec<String>,
    /// Runtime checks the host must run on values from this face.
    /// 宿主必须对本注册面产出的取值执行的运行期校验。
    pub runtime_checks: Vec<RuntimeCheckSpec>,
    /// Optional provenance for a face supplied by an external plugin crate.
    /// 外部插件 crate 提供注册面时，可附带插件来源元数据。
    pub plugin: Option<crate::PluginManifest>,
    /// Declaration site captured for diagnostics.
    /// 为诊断捕获的声明位置。
    pub source: OwnedSourceLocation,
}

/// Whether an author-side value was **declared** by the source file.
/// 作者侧取值是否被源文件**声明过**。
///
/// The merge's one rule lives here so it is spelled once: a new field asks its own type rather
/// than writing a fourth spelling of "the file spoke" (audit `t84`).
/// 合并的那条规则只在这里写一次：新增字段问自己的类型，而不是把"文件说过话"再拼一遍
/// （审计 `t84`）。
trait Declared {
    /// Whether this value was declared rather than left silent.
    /// 该取值是被声明过，还是被留空。
    fn declared(&self) -> bool;
}

impl Declared for crate::OwnedFlowContract {
    fn declared(&self) -> bool {
        self.is_declared()
    }
}

impl<T> Declared for Option<T> {
    fn declared(&self) -> bool {
        self.is_some()
    }
}

impl<T> Declared for Vec<T> {
    fn declared(&self) -> bool {
        !self.is_empty()
    }
}

impl RegistrationSnapshot {
    /// Return the same logical identity used by compiled declarations.
    /// 返回与编译期声明一致的逻辑稳定身份。
    pub fn stable_face_id(&self) -> StableFaceId {
        StableFaceId::from_name(self.stable_name.as_deref().unwrap_or(&self.kind))
    }

    /// Return the explicit stable identity, if this snapshot declared one.
    /// 返回快照显式声明的稳定身份。
    pub fn explicit_stable_face_id(&self) -> Option<StableFaceId> {
        self.stable_name.as_deref().map(StableFaceId::from_name)
    }

    /// Apply file-authored fields while retaining executable compiled metadata.
    /// 应用文件中可编辑的字段，同时保留编译产物中的可执行元数据。
    ///
    /// **The one rule: an author-side value is applied only when the source file declared it;
    /// a field the file left silent keeps the compiled value.** "Declared" is each type's own
    /// spelling of "the file spoke" — `Declared` holds those spellings in one place: the flow
    /// contract answers `is_declared`, a named provider answers `Some`, an editable list answers
    /// non-empty. A field with no such marker (names, identifiers, paths, the rule and admission
    /// this face carries) is **unconditional**: the author-side value wins even when it is empty.
    /// Copy this sentence for a new field and ask `Declared`; do not write another spelling
    /// (audit `t84`).
    ///
    /// Above the split: `export`, `handle` and the parts lists used to be dropped here because a
    /// source-only reload cannot rebuild them, so the compiled evidence stayed. That is what the
    /// two folds below still say where it applies.
    /// **唯一的规则：作者侧取值仅在该源文件声明过时应用；文件没提的字段保留编译期取值。**
    /// "声明过"是各类型自己的说法——`Declared` 把这些说法收在一处：数据流合同答 `is_declared`、
    /// 具名 provider 答 `Some`、可编辑列表答非空。没有这类标记的字段（名字、标识、路径，以及本
    /// 注册面携带的规范与门禁）是**无条件**的：作者侧取值即使为空也生效。新增字段照抄这一句并去问
    /// `Declared`，不要另写一套（审计 `t84`）。
    pub fn merge_authored(mut self, authored: Self) -> Self {
        self.namespace = authored.namespace;
        self.kind = authored.kind;
        self.preset = authored.preset;
        self.parts = authored.parts;
        self.params = authored.params;
        self.handle = authored.handle;
        self.stable_name = authored.stable_name;
        self.name = authored.name;
        self.summary = authored.summary;
        self.exports = authored.exports;
        self.needs_registry = authored.needs_registry;
        self.registry_name = authored.registry_name;
        self.getting_from_other_registry = authored.getting_from_other_registry;
        self.registry_rule_path = authored.registry_rule_path;
        self.registry_rule = authored.registry_rule;
        self.admission = authored.admission;
        self.requires = authored.requires;
        self.provides = authored.provides;
        // The part lists come from the compiled PresetContract and
        // PartsContract associated constants. A source-only reload cannot
        // reconstruct them, so keep that executable evidence while applying
        // the editable output labels.
        // part 列表来自已编译 trait 的关联常量；只读源码的热刷新无法可靠重建，
        // 因此保留这份可执行证据，只更新可编辑的输出标签。
        if authored.flow.declared() {
            self.flow = authored.flow;
        }
        if authored.flow_provider.declared() {
            self.flow_provider = authored.flow_provider;
        }
        // `runtime_checks` is author-editable: the face's own field is parsed into pure
        // `RuntimeCheckSpec` data, so a non-empty author-side list is an edit and wins, the
        // way `flow`'s does. Dropping it here made a source edit to `runtime_checks` a no-op
        // while the file went on showing the new value — the silent-ignore shape this merge
        // exists to avoid. An empty author-side list means the file said nothing, and the
        // compiled checks stand. Audit `LGC-LG-27`.
        // `runtime_checks` 是作者可编辑的：注册面自己的字段会被解析成纯 `RuntimeCheckSpec` 数据，
        // 因此作者侧非空列表就是一次编辑，理应与 `flow` 一样优先。这里丢掉它，会让源码里对
        // `runtime_checks` 的改动成为空操作，而文件上依旧显示着新值——正是这次合并存在的意义所要
        // 避免的"静默忽略"形状。作者侧为空表示文件没提，编译期的校验保留。审计 `LGC-LG-27`。
        if authored.runtime_checks.declared() {
            self.runtime_checks = authored.runtime_checks;
        }
        self.handle_traits = authored.handle_traits;
        self.part_traits = authored.part_traits;
        self.source = authored.source;
        self
    }
}
