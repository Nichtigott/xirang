//! The language-neutral authoring request, plan, and pure validation.
//! 与语言无关的创作请求、计划与纯校验。
//!
//! Every surface that creates or edits a registration face speaks this vocabulary: the CLI, the
//! Studio form, the MCP bridge, and any scaffold that writes a face. The types are pure data —
//! no filesystem, no process state — so the kernel owns them and every execution surface shares
//! one spelling of "what was asked" and "what would be written".
//! 每个创建或编辑注册面的执行面都说这套词汇：CLI、Studio 表单、MCP 桥，以及任何写出注册面的脚手架。
//! 这些类型是纯数据——不碰文件系统、不读进程状态——因此由内核拥有，"请求了什么"与"会写出什么"在所有
//! 执行面只有一种拼法。
//!
//! The request is **resolved** before an adapter sees it: defaults are applied, the destination is a
//! concrete package-relative path, and the parent is a concrete identity plus the language spelling
//! that names it. An adapter therefore renders and reads; it never guesses at process context.
//! 适配器看到请求之前，请求已经**解析**：默认值已补齐、目的地是具体的包内路径、父是具体身份外加点名
//! 它的那种语言拼法。因此适配器只负责渲染与读取，从不猜进程上下文。

use std::collections::BTreeMap;

use crate::identity::NodeId;
use crate::interchange::LanguageId;

/// Whether a request creates a face or modifies one that exists.
/// 请求是新建一个注册面，还是修改一个已存在的注册面。
///
/// The two operations are not one overwrite path: create may apply defaults, while modify must
/// preserve every field the request did not name. Keeping them distinct in the type is what makes
/// "the request omitted a field" mean different things without a caller remembering which.
/// 两种操作不是同一条覆盖路径：新建可以套默认值，而修改必须保留请求没有点名的每个字段。在类型上分开，
/// 正是让"请求没带某个字段"拥有不同含义而不必由调用方记住的原因。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AuthoringOperation {
    /// Create a face that does not exist yet.
    /// 新建一个尚不存在的注册面。
    Create,
    /// Modify an existing face, keeping everything the request does not name.
    /// 修改一个已存在的注册面，保留请求没有点名的一切。
    Modify,
}

impl AuthoringOperation {
    /// The word the surfaces print for this operation.
    /// 各执行面为这种操作打印的词。
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Create => "create",
            Self::Modify => "modify",
        }
    }
}

/// One field's requested state: absent, explicitly empty, or a value.
/// 一个字段的请求状态：未提供、显式空值，或一个取值。
///
/// This is the three-way distinction the acceptance contract requires. `Keep` is "the request did
/// not name it"; on create that means the default applies, and on modify it means the current value
/// stays. `Clear` is an explicit empty value, which is different from `Keep` on modify — it erases
/// the field instead of preserving it.
/// 这是验收契约要求的三态区分。`Keep` 是"请求没有点名它"：新建时套默认值，修改时保留当前值。
/// `Clear` 是显式空值，在修改时与 `Keep` 不同——它清空字段而不是保留。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FieldUpdate<T> {
    /// The request did not name this field.
    /// 请求没有点名这个字段。
    Keep,
    /// The request named this field with an empty value.
    /// 请求以空值点名了这个字段。
    Clear,
    /// The request named this field with this value.
    /// 请求以这个取值点名了该字段。
    Set(T),
}

impl<T> FieldUpdate<T> {
    /// Whether the request left this field alone.
    /// 请求是否没有动这个字段。
    pub fn is_keep(&self) -> bool {
        matches!(self, Self::Keep)
    }

    /// Whether the request explicitly cleared this field.
    /// 请求是否显式清空了这个字段。
    pub fn is_clear(&self) -> bool {
        matches!(self, Self::Clear)
    }

    /// The value, when the request named one.
    /// 请求点名了取值时返回它。
    pub fn value(&self) -> Option<&T> {
        match self {
            Self::Set(value) => Some(value),
            Self::Keep | Self::Clear => None,
        }
    }
}

/// A reference to the face a new face hangs under.
/// 新注册面所挂父面的引用。
///
/// What a request says about the parent, in facts a language can consume.
/// 请求对父面所说的事实，按语言能消费的形式。
///
/// `Object` is the shape an execution surface produces: the parent's identity plus the module path
/// the build already assigned it. A language adapter turns those facts into its own spelling
/// (`::xirang::faces::shop::panel::NODE_ID`, `faces.shop.panel.NODE_ID`, …); the surface never
/// renders another language's expression. `Named` stays for a caller that already holds a spelling
/// (a modify reads it back out of the source), and `Identity`/`Root` for languages that pass those.
/// `Object` 是执行面产出的形状：父面的身份，加上构建已经分配给它的模块路径。语言适配器把这些事实
/// 变成自己的拼法；执行面从不渲染别的语言的表达式。`Named` 留给已经持有拼法的调用方（修改时从源码
/// 读回），`Identity`/`Root` 留给直接传这些的语言。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ParentReference {
    /// The registry root, which some languages cannot name.
    /// 注册机根；有些语言无法点名它。
    Root,
    /// The identity value, for languages that pass identities.
    /// 身份值，供直接传递身份的语言使用。
    Identity(NodeId),
    /// The parent's identity and the module path the build assigned it.
    /// 父面的身份，以及构建分配给它的模块路径。
    Object {
        /// The identity the build computes.
        /// 构建计算出的身份。
        id: NodeId,
        /// The module path, as the build spells it (`shop::panel`).
        /// 模块路径，按构建的拼法（`shop::panel`）。
        module: String,
    },
    /// The parent's identity and its **logical** path (`shop/panel`), for the shared catalog.
    /// 父面的身份与**逻辑**路径（`shop/panel`），供共享对象目录使用。
    Catalog {
        /// The identity the build computes.
        /// 构建计算出的身份。
        id: NodeId,
        /// The logical path below the registry root, slash-separated.
        /// 注册树根之下的逻辑路径，用斜杠分隔。
        path: String,
    },
    /// A fully spelled language reference, for a caller that already holds one.
    /// 完整拼写的语言引用，供已经持有它的调用方使用。
    Named(String),
}

impl ParentReference {
    /// The reference as text, when the caller already spelled it.
    /// 调用方已经拼好时返回文本形式。
    pub fn named(&self) -> Option<&str> {
        match self {
            Self::Named(text) => Some(text),
            Self::Root | Self::Identity(_) | Self::Object { .. } | Self::Catalog { .. } => None,
        }
    }

    /// The parent identity, when the reference carries one.
    /// 引用携带身份时返回它。
    pub fn id(&self) -> Option<NodeId> {
        match self {
            Self::Identity(id) | Self::Object { id, .. } | Self::Catalog { id, .. } => Some(*id),
            Self::Root | Self::Named(_) => None,
        }
    }

    /// The module path, when the reference carries the build's spelling.
    /// 引用携带构建拼法的模块路径时返回它。
    pub fn module(&self) -> Option<&str> {
        match self {
            Self::Object { module, .. } => Some(module),
            Self::Root | Self::Identity(_) | Self::Named(_) | Self::Catalog { .. } => None,
        }
    }
}

/// One registration field an author can set.
/// 作者可以设置的一个注册字段。
///
/// The set is the on-disk set the Rust renderer writes back (`render.rs`'s `ON_DISK_FIELDS`),
/// minus the fields derived from the file's own location. Naming them once here is what lets the
/// capability table say, field by field, which languages can carry them.
/// 这份集合就是 Rust 渲染器会写回的落盘集合（`render.rs` 的 `ON_DISK_FIELDS`），去掉由文件自身位置
/// 派生的那些。在这里只列一次，能力表才能逐字段说明哪些语言携带得了它们。
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RegistrationField {
    /// The module directory and file name.
    /// 模块目录与文件名。
    Module,
    /// The face kind, which is the face's identity name.
    /// 注册面种类名，也就是它的身份名。
    Kind,
    /// The preset type path.
    /// preset 类型路径。
    Preset,
    /// The parts type path.
    /// parts 类型路径。
    Parts,
    /// Localized display name, Chinese half.
    /// 本地化显示名，中文部分。
    NameZh,
    /// Localized display name, English half.
    /// 本地化显示名，英文部分。
    NameEn,
    /// Localized summary, Chinese half.
    /// 本地化摘要，中文部分。
    SummaryZh,
    /// Localized summary, English half.
    /// 本地化摘要，英文部分。
    SummaryEn,
    /// Export names.
    /// 导出名称。
    Exports,
    /// Author-owned identity that survives source moves.
    /// 作者拥有的、可跨源码移动保持的身份名。
    StableName,
    /// Whether the face owns a child registry.
    /// 该面是否拥有子注册机。
    NeedsRegistry,
    /// External registry this face is provisioned from.
    /// 该面从哪个外部注册机取得内容。
    GettingFromOtherRegistry,
    /// Rule for faces entering the registry this face owns.
    /// 进入该面所拥有注册机的规范。
    RegistrationRule,
    /// External dependency gate for this face's registry.
    /// 该面注册机的外部依赖门禁。
    Admission,
    /// Interface names declared by the handle type.
    /// handle 类型声明的接口名。
    HandleTraits,
    /// Contract paths the handle type must implement.
    /// handle 类型必须实现的合同路径。
    HandleContracts,
    /// Interface names declared by the parts type.
    /// parts 类型声明的接口名。
    PartTraits,
    /// Contract paths the parts type must implement.
    /// parts 类型必须实现的合同路径。
    PartContracts,
    /// Capability requirements, as `capability => provider` entries.
    /// 能力需求，形如 `capability => provider`。
    Requires,
    /// Capability names this face provides.
    /// 该面提供的能力名。
    Provides,
    /// Runtime value checks, as expressions the host evaluates.
    /// 宿主求值的运行期取值校验表达式。
    RuntimeChecks,
    /// Explicit flow contract expression.
    /// 显式数据流合同表达式。
    Flow,
    /// Type path supplying the flow contract.
    /// 提供数据流合同的类型路径。
    FlowProvider,
}

impl RegistrationField {
    /// Every field an author can name.
    /// 作者可以点名的全部字段。
    pub const ALL: &'static [Self] = &[
        Self::Module,
        Self::Kind,
        Self::Preset,
        Self::Parts,
        Self::NameZh,
        Self::NameEn,
        Self::SummaryZh,
        Self::SummaryEn,
        Self::Exports,
        Self::StableName,
        Self::NeedsRegistry,
        Self::GettingFromOtherRegistry,
        Self::RegistrationRule,
        Self::Admission,
        Self::HandleTraits,
        Self::HandleContracts,
        Self::PartTraits,
        Self::PartContracts,
        Self::Requires,
        Self::Provides,
        Self::RuntimeChecks,
        Self::Flow,
        Self::FlowProvider,
    ];

    /// The spelling a request or a refusal prints.
    /// 请求或拒绝打印的拼法。
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Module => "module",
            Self::Kind => "kind",
            Self::Preset => "preset",
            Self::Parts => "parts",
            Self::NameZh => "name_zh",
            Self::NameEn => "name_en",
            Self::SummaryZh => "summary_zh",
            Self::SummaryEn => "summary_en",
            Self::Exports => "exports",
            Self::StableName => "stable_name",
            Self::NeedsRegistry => "needs_registry",
            Self::GettingFromOtherRegistry => "getting_from_other_registry",
            Self::RegistrationRule => "registration_rule",
            Self::Admission => "admission",
            Self::HandleTraits => "handle_traits",
            Self::HandleContracts => "handle_contracts",
            Self::PartTraits => "part_traits",
            Self::PartContracts => "part_contracts",
            Self::Requires => "requires",
            Self::Provides => "provides",
            Self::RuntimeChecks => "runtime_checks",
            Self::Flow => "flow",
            Self::FlowProvider => "flow_provider",
        }
    }

    /// Parse one field spelling, refusing unknown names by name.
    /// 解析一个字段拼法，未知名字按名拒绝。
    /// Compatibility alias for the published `RegistrationField::parse` spelling.
    /// 已发布拼法 `RegistrationField::parse` 的兼容别名。
    #[deprecated(
        note = "use RegistrationField::parse_field; this alias keeps 0.2 callers compiling"
    )]
    pub fn parse(text: &str) -> Result<Self, String> {
        Self::parse_field(text)
    }

    /// Parses one registration-field spelling.
    /// 解析一个注册字段拼法。
    pub fn parse_field(text: &str) -> Result<Self, String> {
        Self::ALL
            .iter()
            .copied()
            .find(|field| field.as_str() == text)
            .ok_or_else(|| {
                format!(
                    "`{text}` is not a registration field; the fields are {}",
                    Self::ALL
                        .iter()
                        .map(|field| field.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            })
    }

    /// Whether the field's value is a Rust expression, not data another language can consume.
    /// 该字段的值是否是 Rust 表达式，而不是别的语言能消费的数据。
    ///
    /// The distinction is the acceptance contract's "do not treat an arbitrary Rust expression as
    /// data every language can consume". A language whose capability table omits one of these is
    /// refusing by name rather than downgrading it to a string and claiming equivalence.
    /// 这条区分就是验收契约里的"不把任意 Rust 表达式当成所有语言都能直接消费的数据"。能力表里没有某个
    /// 字段的语言，是按名拒绝，而不是把它降级成字符串再声称等价。
    pub fn is_rust_expression(self) -> bool {
        matches!(
            self,
            Self::Preset
                | Self::Parts
                | Self::GettingFromOtherRegistry
                | Self::RegistrationRule
                | Self::Admission
                | Self::HandleTraits
                | Self::HandleContracts
                | Self::PartTraits
                | Self::PartContracts
                | Self::Requires
                | Self::RuntimeChecks
                | Self::Flow
                | Self::FlowProvider
        )
    }

    /// Whether the field's value is a comma-separated list.
    /// 该字段的值是否是逗号分隔的列表。
    pub fn is_list(self) -> bool {
        matches!(
            self,
            Self::Exports
                | Self::HandleTraits
                | Self::HandleContracts
                | Self::PartTraits
                | Self::PartContracts
                | Self::Requires
                | Self::Provides
                | Self::RuntimeChecks
        )
    }
}

/// The sparse set of field values a request names.
/// 请求点名的稀疏字段取值集合。
///
/// Only fields the request named are present. `Clear` is stored as an empty value, which is why the
/// map can answer "was this field named at all" separately from "what does it say".
/// 只有请求点名的字段在这里。`Clear` 存成空值，因此这张表能分别回答"这个字段被点名了吗"与"它说了什么"。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RegistrationUpdate {
    values: BTreeMap<RegistrationField, String>,
}

impl RegistrationUpdate {
    /// An update that names nothing.
    /// 没有点名任何字段的更新。
    pub fn new() -> Self {
        Self::default()
    }

    /// Whether the request names no field.
    /// 请求是否没有点名任何字段。
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    /// The state of one field.
    /// 一个字段的状态。
    /// Compatibility alias for the published `RegistrationUpdate::get` spelling.
    /// 已发布拼法 `RegistrationUpdate::get` 的兼容别名。
    #[deprecated(
        note = "use RegistrationUpdate::find_field; this alias keeps 0.2 callers compiling"
    )]
    pub fn get(&self, field: RegistrationField) -> FieldUpdate<&str> {
        self.find_field(field)
    }

    /// Returns the update state of one registration field.
    /// 返回一个注册字段的更新状态。
    pub fn find_field(&self, field: RegistrationField) -> FieldUpdate<&str> {
        match self.values.get(&field) {
            None => FieldUpdate::Keep,
            Some(value) if value.is_empty() => FieldUpdate::Clear,
            Some(value) => FieldUpdate::Set(value.as_str()),
        }
    }

    /// The fields this request names, with their normalized text.
    /// 本请求点名的字段及其规范化文本。
    pub fn named(&self) -> impl Iterator<Item = (RegistrationField, &str)> {
        self.values
            .iter()
            .map(|(field, value)| (*field, value.as_str()))
    }

    /// Name one field, normalizing and validating its text.
    /// 点名一个字段，规范化并校验其文本。
    ///
    /// `Clear` is an explicit empty value; every list field becomes a sorted, de-duplicated,
    /// comma-joined list, so two requests that say the same thing compare equal.
    /// `Clear` 是显式空值；每个列表字段变成排序、去重、逗号连接的列表，因此说同一件事的两个请求比较相等。
    pub fn set_text(
        &mut self,
        field: RegistrationField,
        update: FieldUpdate<&str>,
    ) -> Result<(), String> {
        let normalized = match update {
            FieldUpdate::Keep => {
                self.values.remove(&field);
                return Ok(());
            }
            FieldUpdate::Clear => String::new(),
            FieldUpdate::Set(text) => normalize_field(field, text)?,
        };
        self.values.insert(field, normalized);
        Ok(())
    }

    /// Name one field with a value the caller already normalized.
    /// 用一个调用方已规范化的取值点名一个字段。
    ///
    /// The caller is a language adapter reading a source file back: its text is already in the
    /// canonical form this type compares, and re-normalizing it would validate a spelling the
    /// source, not the author, chose.
    /// 调用方是把源码读回来的语言适配器：它的文本已经是本类型比较用的规范形式，再规范化一次等于校验
    /// 源码而不是作者选择的拼法。
    pub fn set_normalized(&mut self, field: RegistrationField, update: FieldUpdate<String>) {
        match update {
            FieldUpdate::Keep => {
                self.values.remove(&field);
            }
            FieldUpdate::Clear => {
                self.values.insert(field, String::new());
            }
            FieldUpdate::Set(value) => {
                self.values.insert(field, value);
            }
        }
    }

    /// Whether this update's named fields all match what a read-back carries.
    /// 本更新点名的字段是否都与读回一致。
    ///
    /// A create request names only the fields it wants to differ from the defaults, while a
    /// read-back carries every field its language can read. This asks the round-trip question that
    /// matters — did every value I asked for survive — without demanding that the read-back invent
    /// the same set of omissions.
    /// 新建请求只点名它想偏离默认值的字段，而读回携带该语言能读的每个字段。这个方法问的是真正要紧的
    /// 往返问题——我要求的每个取值都活下来了吗——而不要求读回凭空复制同样的省略集合。
    pub fn satisfies(&self, read_back: &Self) -> bool {
        self.values.iter().all(|(field, value)| {
            let theirs = read_back.find_field(*field);
            match (value.is_empty(), theirs) {
                (true, FieldUpdate::Clear) => true,
                (false, FieldUpdate::Set(theirs)) => value == theirs,
                _ => false,
            }
        })
    }

    /// Whether two updates say the same thing about every field either one names.
    /// 两个更新是否对它们任一点名的每个字段说的是同一件事。
    ///
    /// This is the round-trip comparison: it compares normalized semantics, not source appearance.
    /// A request that set three fields and a read-back that carries those three compares equal even
    /// if one side wrote a list in another order.
    /// 这是往返比较：比的是规范化后的语义，不是源码外观。设置了三个字段的请求与携带这三个字段的读回
    /// 比较相等，即使某一侧用另一种顺序写列表。
    pub fn agrees_with(&self, other: &Self) -> bool {
        let fields: std::collections::BTreeSet<RegistrationField> = self
            .values
            .keys()
            .chain(other.values.keys())
            .copied()
            .collect();
        fields.into_iter().all(|field| {
            let mine = self.find_field(field);
            let theirs = other.find_field(field);
            mine == theirs
        })
    }
}

/// Normalize and validate one field's text.
/// 规范化并校验一个字段的文本。
fn normalize_field(field: RegistrationField, text: &str) -> Result<String, String> {
    if text.contains(['\n', '\r']) {
        return Err(format!("`{}` cannot contain a newline", field.as_str()));
    }
    match field {
        RegistrationField::Module => {
            crate::authoring::validation::validate_name(text)?;
            Ok(text.to_owned())
        }
        RegistrationField::Kind => {
            crate::authoring::validation::validate_kind_name(text)?;
            Ok(text.to_owned())
        }
        RegistrationField::StableName => {
            let valid = !text.is_empty()
                && text
                    .chars()
                    .all(|character| character == '_' || character.is_ascii_alphanumeric());
            if !valid {
                return Err(format!(
                    "`{text}` is not a stable name; use letters, digits and `_`"
                ));
            }
            Ok(text.to_owned())
        }
        RegistrationField::NeedsRegistry => match text {
            "true" | "false" => Ok(text.to_owned()),
            _ => Err("`needs_registry` must be `true` or `false`".to_owned()),
        },
        field if field.is_list() => {
            let mut entries: Vec<&str> = text
                .split(',')
                .map(str::trim)
                .filter(|entry| !entry.is_empty())
                .collect();
            entries.sort_unstable();
            entries.dedup();
            Ok(entries.join(","))
        }
        _ => Ok(text.trim().to_owned()),
    }
}

/// What one language implementation can carry.
/// 一个语言实现能携带什么。
///
/// The table is data, so a surface can print it (Studio's language picker, the CLI's `--list`) and
/// the validator can refuse a field by name without knowing which language it is looking at.
/// 这张表是数据，因此执行面可以打印它（Studio 的语言选择、CLI 的 `--list`），校验器也能按名拒绝字段
/// 而不必知道自己在看哪种语言。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LanguageCapabilities {
    /// The language this table describes.
    /// 本表描述的语言。
    pub language: LanguageId,
    /// File extensions the adapter reads, without the leading dot.
    /// 适配器读取的文件扩展名，不含前导点。
    pub file_extensions: &'static [&'static str],
    /// Operations the adapter implements.
    /// 适配器实现的操作。
    pub operations: &'static [AuthoringOperation],
    /// Registration fields the adapter can carry.
    /// 适配器能携带的注册字段。
    pub fields: &'static [RegistrationField],
}

impl LanguageCapabilities {
    /// Whether this language can carry one field.
    /// 该语言能否携带某个字段。
    pub fn supports(&self, field: RegistrationField) -> bool {
        self.fields.contains(&field)
    }

    /// Whether this language implements one operation.
    /// 该语言是否实现了某种操作。
    pub fn supports_operation(&self, operation: AuthoringOperation) -> bool {
        self.operations.contains(&operation)
    }
}

/// A fully resolved authoring request, ready for a language adapter.
/// 一份完全解析、可直接交给语言适配器的创作请求。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FaceAuthoringRequest {
    /// The target language.
    /// 目标语言。
    pub language: LanguageId,
    /// Whether this creates or modifies a face.
    /// 本请求是新建还是修改。
    pub operation: AuthoringOperation,
    /// The package-relative file the plan writes.
    /// 计划写入的包内相对文件。
    pub destination: String,
    /// The parent's reference, in the spelling the target language consumes.
    /// 父面的引用，按目标语言消费的拼法。
    pub parent: ParentReference,
    /// The fields the request names.
    /// 请求点名的字段。
    pub registration: RegistrationUpdate,
    /// The existing source and the derived context the adapter may read.
    /// 现有源码，以及适配器可以读取的派生上下文。
    pub source_context: SourceContext,
}

/// What an adapter may read besides the request's fields.
/// 适配器除字段之外可以读取的内容。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SourceContext {
    /// The existing file's text, for a modify request.
    /// 修改请求里现有文件的文本。
    pub existing_source: Option<String>,
    /// The module path the generated declaration lives in, when the surface derived one.
    /// 生成声明所在模块路径（执行面推导出来时）。
    pub module_path: Option<String>,
    /// The package namespace identities are hashed under.
    /// 身份散列所用的包命名空间。
    pub namespace: Option<String>,
}

impl SourceContext {
    /// An empty context.
    /// 空上下文。
    pub fn new() -> Self {
        Self::default()
    }

    /// The existing source text, for a modify request.
    /// 修改请求的现有源码文本。
    pub fn source_text(&self) -> Option<&str> {
        self.existing_source.as_deref()
    }

    /// Record the existing source text.
    /// 记录现有源码文本。
    pub fn with_existing_source(mut self, source: impl Into<String>) -> Self {
        self.existing_source = Some(source.into());
        self
    }
}

/// One precise edit inside a file.
/// 文件内的一处精确编辑。
///
/// Byte offsets into the original text, so a reviewer can see that a modify touches the declaration
/// and not the business code around it. A create plan has no edits and carries the whole file.
/// 原文本的字节偏移，因此审阅者能看出修改只动声明、不动周围业务代码。新建计划没有编辑项，携带整份文件。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlannedEdit {
    /// First byte replaced.
    /// 被替换的首字节。
    pub start: usize,
    /// One past the last byte replaced.
    /// 被替换区间的结束字节（不含）。
    pub end: usize,
    /// The text that replaces the range.
    /// 替换该区间的文本。
    pub replacement: String,
}

/// One file a plan would write.
/// 计划会写入的一个文件。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlannedFile {
    /// Package-relative path.
    /// 包内相对路径。
    pub path: String,
    /// The file's text before the plan; `None` means the plan creates it.
    /// 计划之前的文件文本；`None` 表示计划创建它。
    pub original: Option<String>,
    /// The file's text after the plan.
    /// 计划之后的文件文本。
    pub updated: String,
    /// The precise edits into `original`, empty for a created file.
    /// 相对 `original` 的精确编辑；新建文件为空。
    pub edits: Vec<PlannedEdit>,
    /// Whether the plan removes this file rather than writing `updated`.
    /// 计划是移除该文件，而不是写下 `updated`。
    pub remove: bool,
}

impl PlannedFile {
    /// Whether this file already exists.
    /// 该文件是否已存在。
    pub fn creates(&self) -> bool {
        self.original.is_none()
    }

    /// Whether the plan changes the file's bytes or removes it.
    /// 计划是否改变该文件的字节或移除它。
    pub fn changes(&self) -> bool {
        self.remove || self.original.as_deref() != Some(self.updated.as_str())
    }
}

/// How serious a plan diagnostic is.
/// 计划诊断的严重程度。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiagnosticSeverity {
    /// Informational, shown but not a refusal.
    /// 提示信息，展示但不拒绝。
    Note,
    /// The plan is possible but a reader should look before applying it.
    /// 计划可行，但审阅者落盘前应当看一眼。
    Warning,
}

/// One thing the plan wants a reader to know.
/// 计划想让审阅者知道的一件事。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlanDiagnostic {
    /// How serious it is.
    /// 严重程度。
    pub severity: DiagnosticSeverity,
    /// The message.
    /// 消息正文。
    pub message: String,
}

/// A check the plan owes before it can be called done.
/// 计划在可称为完成之前必须履行的检查。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ObligationKind {
    /// Re-read the generated declaration and compare normalized fields.
    /// 重新读取生成的声明并比较规范化字段。
    RoundTrip,
    /// Build the package so the declaration is compiled and registered.
    /// 构建包，使声明被编译并登记。
    Build,
    /// Confirm the face appears in the built tree with the expected parent.
    /// 确认该面以预期父出现在构建后的树里。
    Registration,
    /// Call the exported behaviour through the built tree.
    /// 经构建后的树调用导出行为。
    Call,
    /// Confirm the face's identity is the one the other language names.
    /// 确认该面的身份就是另一种语言点名的那个。
    Identity,
}

impl ObligationKind {
    /// The word printed for this obligation.
    /// 该义务打印的词。
    pub fn as_str(self) -> &'static str {
        match self {
            Self::RoundTrip => "round-trip",
            Self::Build => "build",
            Self::Registration => "registration",
            Self::Call => "call",
            Self::Identity => "identity",
        }
    }
}

/// One validation obligation the plan names.
/// 计划点名的一条验证义务。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ValidationObligation {
    /// What kind of check this is.
    /// 检查的种类。
    pub kind: ObligationKind,
    /// The concrete thing to check.
    /// 要检查的具体内容。
    pub detail: String,
}

/// The reviewable result of a shared authoring request.
/// 共享创作请求的可审阅结果。
///
/// The plan is what a surface shows before writing: files with their exact bytes, precise edits,
/// diagnostics, and the checks that still have to pass. Applying it goes through the execution
/// surface's transaction, not through the language adapter — a template never writes a file itself.
/// 计划是执行面在写入前展示的东西：带精确字节的文件、精确编辑、诊断，以及仍须通过的检查。落盘经执行面的
/// 事务完成，而不是由语言适配器完成——模板从不自己写文件。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FaceAuthoringPlan {
    /// The language that produced the plan.
    /// 生成该计划的语言。
    pub language: LanguageId,
    /// The operation the plan implements.
    /// 计划实现的操作。
    pub operation: AuthoringOperation,
    /// The files the plan writes.
    /// 计划写入的文件。
    pub files: Vec<PlannedFile>,
    /// Diagnostics for a reviewer.
    /// 给审阅者的诊断。
    pub diagnostics: Vec<PlanDiagnostic>,
    /// Checks that still have to pass.
    /// 仍须通过的检查。
    pub obligations: Vec<ValidationObligation>,
}

impl FaceAuthoringPlan {
    /// The files whose bytes the plan changes.
    /// 计划改变字节的文件。
    pub fn changed_files(&self) -> impl Iterator<Item = &PlannedFile> {
        self.files.iter().filter(|file| file.changes())
    }

    /// Whether applying the plan would write nothing.
    /// 落盘该计划是否不会写任何东西。
    pub fn writes_nothing(&self) -> bool {
        self.changed_files().next().is_none()
    }
}

/// Validate one resolved request against a language's capability table.
/// 按一门语言的能力表校验一份已解析的请求。
///
/// The refusals name the field and the language, because the caller's next move is either to drop
/// the field or to pick a language that carries it. A silent drop, a `ANY` default, or a string
/// downgrade are all forbidden by the acceptance contract; this function is where that rule is
/// enforced rather than remembered.
/// 拒绝会点名字段与语言，因为调用方的下一步要么去掉字段，要么换一门携带它的语言。静默丢弃、填 `ANY`
/// 默认值、或降级成字符串，都是验收契约禁止的；这条规则在这里执行，而不是靠人记住。
pub fn validate_request(
    request: &FaceAuthoringRequest,
    capabilities: &LanguageCapabilities,
) -> Result<(), String> {
    if capabilities.language != request.language {
        return Err(format!(
            "the capability table describes `{}` but the request targets `{}`",
            capabilities.language.as_str(),
            request.language.as_str()
        ));
    }
    if !capabilities.supports_operation(request.operation) {
        return Err(format!(
            "`{}` does not implement `{}`; this build has no adapter for that operation",
            request.language.as_str(),
            request.operation.as_str()
        ));
    }
    for (field, _) in request.registration.named() {
        if !capabilities.supports(field) {
            let reason = if field.is_rust_expression() {
                "its value is a Rust expression, which this language cannot consume"
            } else {
                "this language's registration record does not carry it"
            };
            return Err(format!(
                "`{}` cannot carry `{}`: {reason}; drop the field or use a language that supports it",
                request.language.as_str(),
                field.as_str()
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "request_tests.rs"]
mod request_tests;
