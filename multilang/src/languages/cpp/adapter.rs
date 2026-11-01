//! The C++ language implementation: generate, read, and precisely rewrite one face declaration.
//! C++ 语言实现：生成、读取并精确改写一条面声明。
//!
//! Two spellings are accepted. The current one is a marked declaration:
//! 两种拼法都接受。当前这种是带标记的声明：
//!
//! ```cpp
//! XIRANG_SOIL
//! constexpr auto SOIL = ::xirang::soil<Gauge>()
//!     .under(::xirang::faces::shop::panel::NODE_ID)
//!     .named("Gauge")
//!     .needs_registry(false);
//! ```
//!
//! The version-1 spelling (`XIRANG_FACE(record, (expression))`) is still read and still modified in
//! place, so an existing tree keeps working; a modify preserves whichever spelling the file uses,
//! because rewriting a declaration's spelling is a migration, not a field edit.
//! 版本 1 拼法（`XIRANG_FACE(record, (expression))`）仍然读取、仍然就地修改，因此既有树继续可用；
//! 修改保留文件所用的那种拼法，因为改写声明的拼法是一次迁移，不是一次字段编辑。

use std::sync::OnceLock;

use xirang_kernel::authoring::{
    AuthoringOperation, DiagnosticSeverity, FaceAuthoringPlan, FaceAuthoringRequest, FieldUpdate,
    LanguageCapabilities, ParentReference, PlanDiagnostic, PlannedFile, RegistrationField,
    RegistrationUpdate, ValidationObligation,
};

use crate::languages::kit::adapter::{
    EditHooks, Editable, destination_module, modify as kit_modify,
};
use crate::languages::kit::grammar::{
    FaceGrammar, FieldSpec, Role, ValueStyle, obligations as stated_obligations, quote, text_field,
};
use crate::registration::{EditorContext, LanguageAdapter};

pub use crate::languages::cpp::syntax::Spelling;

/// The C++ language implementation.
/// C++ 语言实现。
#[derive(Clone, Copy, Debug, Default)]
pub struct CppLanguage;

/// The registration fields a C++ declaration carries.
/// C++ 声明携带的注册字段。
///
/// The record is a fixed set of text fields (see `multilang/sdk/cpp/xirang/soil.hpp`). Fields whose
/// value would be a Rust expression — a registry rule, an admission, runtime checks, flow providers,
/// trait lists — are **not** in this list: the shared validator refuses them by name, so an author
/// never gets a declaration that looks like it declares one.
/// 记录是一组固定的文本字段（见 `multilang/sdk/cpp/xirang/soil.hpp`）。取值会是 Rust 表达式的字段——
/// 注册规则、准入、运行期校验、flow 提供方、trait 列表——**不**在这张表里：共享校验器按名拒绝它们，
/// 因此作者永远不会得到一条看起来声明了它们的声明。
const FIELDS: &[RegistrationField] = &[
    RegistrationField::Module,
    RegistrationField::Kind,
    RegistrationField::StableName,
    RegistrationField::Exports,
    RegistrationField::Provides,
    RegistrationField::Requires,
    RegistrationField::Preset,
    RegistrationField::Parts,
    RegistrationField::NameZh,
    RegistrationField::NameEn,
    RegistrationField::SummaryZh,
    RegistrationField::SummaryEn,
    RegistrationField::GettingFromOtherRegistry,
    RegistrationField::NeedsRegistry,
];

/// The fields a C++ builder chain carries, in the order the renderer writes them.
/// C++ 构造器链携带的字段，按渲染器写出的顺序。
///
/// The renderer and the reader both go through this table, so a method name cannot be renamed on
/// one side only: the same entry decides how a call is written and which field it names.
/// 渲染器与读取器都走这张表，因此一个方法名不可能只在一侧改名：同一条目既决定一个调用怎么写，也决定它
/// 命名哪个字段。
const GRAMMAR: FaceGrammar = FaceGrammar {
    fields: &[
        FieldSpec {
            role: Role::Parent,
            spelling: "under",
            style: ValueStyle::Expr,
        },
        FieldSpec {
            role: Role::Kind,
            spelling: "named",
            style: ValueStyle::Quoted,
        },
        FieldSpec {
            role: Role::Field(RegistrationField::StableName),
            spelling: "stable_name",
            style: ValueStyle::Quoted,
        },
        FieldSpec {
            role: Role::Field(RegistrationField::Exports),
            spelling: "exports",
            style: ValueStyle::QuotedList,
        },
        FieldSpec {
            role: Role::Field(RegistrationField::Provides),
            spelling: "provides",
            style: ValueStyle::QuotedList,
        },
        FieldSpec {
            role: Role::Field(RegistrationField::Requires),
            spelling: "requires_capabilities",
            style: ValueStyle::RequiresComma,
        },
        FieldSpec {
            role: Role::Field(RegistrationField::Preset),
            spelling: "preset",
            style: ValueStyle::Quoted,
        },
        FieldSpec {
            role: Role::Field(RegistrationField::Parts),
            spelling: "parts",
            style: ValueStyle::Quoted,
        },
        FieldSpec {
            role: Role::Field(RegistrationField::NameZh),
            spelling: "name_zh",
            style: ValueStyle::Quoted,
        },
        FieldSpec {
            role: Role::Field(RegistrationField::NameEn),
            spelling: "name_en",
            style: ValueStyle::Quoted,
        },
        FieldSpec {
            role: Role::Field(RegistrationField::SummaryZh),
            spelling: "summary_zh",
            style: ValueStyle::Quoted,
        },
        FieldSpec {
            role: Role::Field(RegistrationField::SummaryEn),
            spelling: "summary_en",
            style: ValueStyle::Quoted,
        },
        FieldSpec {
            role: Role::Field(RegistrationField::GettingFromOtherRegistry),
            spelling: "getting_from_other_registry",
            style: ValueStyle::Quoted,
        },
        FieldSpec {
            role: Role::Field(RegistrationField::NeedsRegistry),
            spelling: "needs_registry",
            style: ValueStyle::Bool {
                when_true: "true",
                when_false: "false",
            },
        },
    ],
};

/// What the C++ adapter tells the shared orchestration about itself.
/// C++ 适配器告诉共享编排的东西。
const SPEC: crate::languages::kit::adapter::AdapterSpec =
    crate::languages::kit::adapter::AdapterSpec {
        language: "cpp",
        extensions: &["cpp", "cc", "cxx", "c++"],
        fields: FIELDS,
        operations: &[AuthoringOperation::Create, AuthoringOperation::Modify],
        carrying: "a C++ declaration is a record of text fields",
    };

static CAPABILITIES: OnceLock<LanguageCapabilities> = OnceLock::new();

/// The capability table for C++.
/// C++ 的能力表。
fn capabilities() -> &'static LanguageCapabilities {
    crate::languages::kit::adapter::capabilities(&CAPABILITIES, &SPEC)
}

impl LanguageAdapter for CppLanguage {
    fn capabilities(&self) -> &'static LanguageCapabilities {
        capabilities()
    }

    fn plan_move(
        &self,
        request: &crate::registration::MoveRequest<'_>,
    ) -> Result<Vec<crate::registration::SourceEdit>, String> {
        let Some(parent) = request.new_parent else {
            return Ok(Vec::new());
        };
        let declaration = crate::languages::cpp::syntax::read_one(request.source)?;
        let range = declaration.under_argument.ok_or_else(|| {
            "the declaration has no `.under(…)` call, so its parent cannot be moved".to_owned()
        })?;
        Ok(vec![crate::registration::SourceEdit {
            start: range.start,
            end: range.end,
            replacement: format!("::xirang::faces::{parent}::NODE_ID"),
        }])
    }

    fn plan_move_with_catalog(
        &self,
        request: &crate::registration::MoveRequest<'_>,
        parent: Option<&crate::registration::CatalogParent<'_>>,
    ) -> Result<Vec<crate::registration::SourceEdit>, String> {
        let Some(parent) = parent else {
            return self.plan_move(request);
        };
        let declaration = crate::languages::cpp::syntax::read_one(request.source)?;
        let range = declaration.under_argument.ok_or_else(|| {
            "the declaration has no `.under(…)` call, so its parent cannot be moved".to_owned()
        })?;
        Ok(vec![crate::registration::SourceEdit {
            start: range.start,
            end: range.end,
            replacement: format!(
                "crate::catalog::{}::NODE_ID",
                crate::languages::cpp::names::catalog_path(parent.path)
            ),
        }])
    }

    fn read(&self, source: &str) -> Result<crate::registration::FaceReadBack, String> {
        crate::languages::kit::adapter::read_back(super::syntax::declared, source)
    }

    fn editor_material(&self, context: &EditorContext) -> Vec<PlannedFile> {
        let directories = vec![
            context.package_root.display().to_string(),
            context.out_dir.display().to_string(),
        ];
        vec![PlannedFile {
            path: "compile_flags.txt".to_owned(),
            original: None,
            updated: crate::sdk::cpp_compile_flags(&directories),
            edits: Vec::new(),
            remove: false,
        }]
    }

    fn plan(&self, request: &FaceAuthoringRequest) -> Result<FaceAuthoringPlan, String> {
        crate::languages::kit::adapter::plan(&SPEC, request, create, modify)
    }
}

/// One C++ declaration, as the text tells it.
/// 一条 C++ 声明，按文本读到的样子。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CppDeclaration {
    /// The language-neutral payload: parent, registration fields, and unmapped calls.
    /// 语言中立载荷：父级、注册字段与未映射的调用。
    pub face: crate::languages::kit::grammar::DeclaredFace,
    /// Which spelling the file uses.
    /// 文件所用的拼法。
    pub spelling: crate::languages::cpp::syntax::Spelling,
    /// The record variable a version-1 macro declares; empty for the marked spelling.
    /// 版本 1 宏声明的记录变量名；带标记的拼法下为空。
    pub variable: String,
    /// The business type inside `soil<T>()`, as written.
    /// `soil<T>()` 里的业务类型，按书写原样。
    pub business_type: String,
    /// Byte range of the text a precise edit replaces: the expression for the marked spelling, and
    /// the parenthesised expression argument for the version-1 spelling.
    /// 精确编辑所替换的文本字节区间：带标记的拼法是表达式，版本 1 拼法是括号包住的表达式参数。
    pub expression: std::ops::Range<usize>,
}

impl CppDeclaration {
    /// The face kind (the `named` value).
    /// 面种类名（`named` 的取值）。
    pub fn kind(&self) -> String {
        self.face.kind()
    }

    /// The parent as the request vocabulary spells it.
    /// 父面按请求词汇的拼法。
    pub fn parent_reference(&self) -> ParentReference {
        ParentReference::Named(self.face.parent.clone())
    }
}

/// Read one declaration out of a source file.
/// 从源码里读出一条声明。
///
/// The lexical work lives in [`crate::languages::cpp::syntax`]: it tokenises the file so a marker in a comment
/// or a string is not a declaration, accepts legal whitespace/comments, and reports every actual
/// declaration. This function only maps the recorded builder calls onto the shared field vocabulary.
/// 词法工作住在 [`crate::languages::cpp::syntax`]：它切分源码，使注释或字符串里的标记不是声明，接受合法空白/注释，
/// 并报告每一条真正的声明。本函数只把记录下来的构造器调用映射到共享字段词汇。
/// Compatibility alias for the published `languages::cpp::read` spelling.
/// 已发布拼法 `languages::cpp::read` 的兼容别名。
#[deprecated(note = "use languages::cpp::read_declaration; this alias keeps 0.2 callers compiling")]
pub fn read(source: &str) -> Result<CppDeclaration, String> {
    read_declaration(source)
}

/// Reads one C++ declaration from source text.
/// 从源码文本读出一条 C++ 声明。
pub(super) fn read_declaration(source: &str) -> Result<CppDeclaration, String> {
    let declaration = crate::languages::cpp::syntax::read_one(source)?;
    let mut registration = RegistrationUpdate::new();
    let mut extras = Vec::new();
    let mut seen: Vec<&str> = Vec::new();
    for (method, argument) in &declaration.calls {
        if seen.contains(&method.as_str()) {
            return Err(format!("the declaration calls `.{method}(…)` twice"));
        }
        seen.push(method);
        match method.as_str() {
            "under" => {}
            // The version-1 spelling of the same fact: an argument-less call that means true.
            // 同一事实的版本 1 拼法：无参调用、意为真。
            "owns_registry" => {
                if !argument.trim().is_empty() {
                    return Err("`.owns_registry()` takes no argument".to_owned());
                }
                registration.set_normalized(
                    RegistrationField::NeedsRegistry,
                    FieldUpdate::Set("true".to_owned()),
                );
            }
            other => match GRAMMAR.lookup(other).map(|spec| spec.role) {
                // The display name is the kind the registration keys on; the grammar names the role.
                // 显示名就是注册机据以索引的 kind；语法表点名的是角色。
                Some(Role::Kind) => {
                    registration.set_normalized(
                        RegistrationField::Kind,
                        FieldUpdate::Set(crate::languages::cpp::syntax::string_literal(argument)?),
                    );
                }
                Some(Role::Field(RegistrationField::NeedsRegistry)) => {
                    let value = match argument.trim() {
                        "true" => "true",
                        "false" => "false",
                        other => {
                            return Err(format!(
                                "`.needs_registry({other})` is not a boolean; write `true` or `false`"
                            ));
                        }
                    };
                    registration.set_normalized(
                        RegistrationField::NeedsRegistry,
                        FieldUpdate::Set(value.to_owned()),
                    );
                }
                Some(Role::Field(field)) => {
                    let value = crate::languages::cpp::syntax::string_literal(argument)?;
                    if field == RegistrationField::Requires {
                        for entry in split_list(&value) {
                            let (capability, provider) = entry.split_once("=>").ok_or_else(|| {
                                format!(
                                    "`.requires_capabilities(\"{entry}\")` has no `=>`; a requirement \
                                     names both the capability and its provider"
                                )
                            })?;
                            if capability.trim().is_empty() || provider.trim().is_empty() {
                                return Err(format!(
                                    "`.requires_capabilities(\"{entry}\")` has an empty half"
                                ));
                            }
                        }
                    }
                    registration.set_normalized(field, FieldUpdate::Set(value));
                }
                Some(Role::Parent) => {}
                None => {
                    // A field the standard vocabulary does not carry: preserve it verbatim rather
                    // than dropping it from the rewrite.
                    // 标准词汇不携带的字段：逐字保留，而不是在改写里丢掉它。
                    extras.push((
                        other.to_owned(),
                        crate::languages::cpp::syntax::string_literal(argument)?,
                    ));
                }
            },
        }
    }
    declaration.name.as_ref().ok_or_else(|| {
        "the declaration never calls `.named(\"…\")`; the name is part of the face's identity"
            .to_owned()
    })?;
    Ok(CppDeclaration {
        face: crate::languages::kit::grammar::DeclaredFace {
            parent: declaration.parent.unwrap_or_default(),
            registration,
            extras,
        },
        spelling: declaration.spelling,
        variable: declaration.variable,
        business_type: declaration.business_type,
        expression: declaration.expression,
    })
}

/// Create one declaration from a request.
/// 按请求创建一条声明。
fn create(request: &FaceAuthoringRequest) -> Result<FaceAuthoringPlan, String> {
    if request.source_context.source_text().is_some() {
        return Err(format!(
            "{} already exists; a create request must not overwrite it (use modify)",
            request.destination
        ));
    }
    let module = destination_module(&request.destination)?;
    if let FieldUpdate::Set(named) = request.registration.find_field(RegistrationField::Module)
        && named != module
    {
        return Err(format!(
            "the request names module `{named}` but the destination file is `{}`; the file name is \
             the module",
            request.destination
        ));
    }
    let kind = match request.registration.find_field(RegistrationField::Kind) {
        FieldUpdate::Set(kind) => kind.to_owned(),
        FieldUpdate::Clear => {
            return Err("`kind` cannot be empty; it names the face".to_owned());
        }
        FieldUpdate::Keep => xirang_kernel::authoring::pascal_case(&module),
    };
    let parent = parent_constant(request)?;
    // The declaration's fields begin as what the request named, with `kind` filled in: a create may
    // apply defaults, and the kind has one.
    let mut registration = request.registration.clone();
    if matches!(
        request.registration.find_field(RegistrationField::Kind),
        FieldUpdate::Keep
    ) {
        registration.set_normalized(RegistrationField::Kind, FieldUpdate::Set(kind.clone()));
    }
    let text = render_face(&kind, &parent, &registration, &[]);
    let mut diagnostics = vec![PlanDiagnostic {
        severity: DiagnosticSeverity::Note,
        message: format!(
            "`{kind}` is declared as an empty type; fill in its implementation beside the declaration"
        ),
    }];
    diagnostics.push(PlanDiagnostic {
        severity: DiagnosticSeverity::Note,
        message: format!(
            "the SDK include is `{}`; the build adds it to the compiler's search path",
            crate::sdk::cpp_include_dir().display()
        ),
    });
    Ok(FaceAuthoringPlan {
        language: capabilities().language.clone(),
        operation: AuthoringOperation::Create,
        files: vec![PlannedFile {
            path: request.destination.clone(),
            original: None,
            updated: text,
            edits: Vec::new(),
            remove: false,
        }],
        diagnostics,
        obligations: obligations(&kind, &registration),
    })
}
impl Editable for CppDeclaration {
    fn face(&self) -> &crate::languages::kit::grammar::DeclaredFace {
        &self.face
    }

    fn arguments(&self) -> std::ops::Range<usize> {
        self.expression.clone()
    }

    fn rendered(&self, parent: &str, registration: &RegistrationUpdate) -> String {
        let expression =
            render_expression(&self.business_type, parent, registration, &self.face.extras);
        match self.spelling {
            Spelling::Marked => expression,
            Spelling::Compat => format!("({expression})"),
        }
    }
}

const EDIT: EditHooks<CppDeclaration> = EditHooks {
    read: read_declaration,
    parent: parent_constant,
    obligations,
    carrier: Some("business type"),
};

/// Modify one existing declaration from a request, preserving everything the request does not name.
/// 按请求修改一条已有声明，保留请求没有点名的一切。
fn modify(request: &FaceAuthoringRequest) -> Result<FaceAuthoringPlan, String> {
    kit_modify(&capabilities().language, request, &EDIT)
}
/// The parent constant a C++ declaration can name.
/// C++ 声明可以点名的父常量。
fn parent_constant(request: &FaceAuthoringRequest) -> Result<String, String> {
    match &request.parent {
        // The adapter renders its own spelling from the build's facts; the surface never passes a
        // pre-rendered C++ expression.
        // 适配器从构建事实渲染自己的拼法；执行面从不传入预先渲染的 C++ 表达式。
        ParentReference::Object { module, .. } if !module.trim().is_empty() => {
            Ok(crate::languages::cpp::names::node_id(module))
        }
        ParentReference::Object { .. } => Err(
            "the request names an empty parent module; a C++ declaration names a generated \
             constant, which needs a module path"
                .to_owned(),
        ),
        ParentReference::Catalog { path, .. } if !path.trim().is_empty() => Ok(format!(
            "crate::catalog::{}::NODE_ID",
            crate::languages::cpp::names::catalog_path(path)
        )),
        ParentReference::Catalog { .. } => Err(
            "the parent reference carries an empty logical catalog path; a C++ declaration names \
             a generated constant, which needs one"
                .to_owned(),
        ),
        ParentReference::Named(text) if !text.trim().is_empty() => {
            let trimmed = text.trim();
            let path = trimmed.strip_prefix("::").unwrap_or(trimmed);
            let valid = path
                .strip_prefix("xirang::faces::")
                .or_else(|| path.strip_prefix("crate::catalog::"))
                .is_some_and(|rest| {
                    !rest.is_empty()
                        && rest.chars().all(|character| {
                            character.is_ascii_alphanumeric()
                                || character == ':'
                                || character == '_'
                        })
                        && !rest.ends_with(':')
                });
            if !valid {
                return Err(format!(
                    "`{trimmed}` is not a generated C++ face reference; a declaration names a \
                     constant the build wrote as `crate::catalog::<path>::NODE_ID` (or the \
                     compatibility `::xirang::faces::<path>::NODE_ID`), not an expression"
                ));
            }
            Ok(trimmed.to_owned())
        }
        ParentReference::Named(_) => Err(
            "the parent reference is empty; a C++ face must name a generated constant".to_owned(),
        ),
        // One spelling for the root, emitted by the build from the package namespace: the same
        // identity Rust's `root_node_id` computes, so a C++ face needs no placeholder Rust parent.
        // 根只有一种拼写，由构建按包命名空间发射：与 Rust `root_node_id` 算出的同一个身份，因此
        // C++ 面不需要占位 Rust 父面。
        ParentReference::Root => Ok(crate::languages::cpp::names::root_node_id().to_owned()),
        ParentReference::Identity(_) => Err(
            "the request carries a parent identity, but a C++ declaration names a compile-time \
             constant from `xirang/faces.hpp`"
                .to_owned(),
        ),
    }
}

fn split_list(text: &str) -> Vec<String> {
    text.split(',')
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
        .map(str::to_owned)
        .collect()
}

#[cfg(test)]
#[path = "adapter_tests.rs"]
mod adapter_tests;

// The rendering half, kept in the adapter module: a language owns one renderer, and splitting it
// out for size alone costs a file without buying a boundary.
// 渲染那一半留在适配器模块里：一门语言只有一个渲染器，仅为尺寸把它拆出去只多一个文件、换不来边界。
/// Render a whole C++ source file for one created face.
/// 为一个新建面渲染整份 C++ 源文件。
fn render_face(
    kind: &str,
    parent: &str,
    registration: &RegistrationUpdate,
    extras: &[(String, String)],
) -> String {
    format!(
        "// Generated by `xirang face`; the declaration below is the whole registration.\n\
         // 由 `xirang face` 生成；下面这条声明就是完整的注册。\n\
         #include \"xirang/soil.hpp\"\n\
         #include \"xirang/faces.hpp\"\n\n\
         struct {kind} {{}};\n\n\
         XIRANG_SOIL\n\
         constexpr auto SOIL = {};\n",
        render_expression(kind, parent, registration, extras)
    )
}

/// Render one builder chain, without a trailing semicolon.
/// 渲染一条构造器链，不带结尾分号。
///
/// The method order is fixed, so the same fields always render the same text and a build's
/// generated file is stable under a second run.
/// 方法顺序固定，因此同样的字段总是渲染出同样的文本，第二次运行生成的源码逐字不变。
fn render_expression(
    kind: &str,
    parent: &str,
    registration: &RegistrationUpdate,
    extras: &[(String, String)],
) -> String {
    let spelling = |role| {
        GRAMMAR
            .spec(role)
            .map(|spec| spec.spelling)
            .expect("the C++ grammar carries every role the renderer writes")
    };
    let mut text = format!("::xirang::soil<{kind}>()");
    text.push_str(&format!(
        "\n    .{}({})",
        spelling(Role::Parent),
        GRAMMAR.declared(Role::Parent, parent)
    ));
    text.push_str(&format!(
        "\n    .{}({})",
        spelling(Role::Kind),
        GRAMMAR.declared(
            Role::Kind,
            text_field(registration, RegistrationField::Kind)
        )
    ));
    for spec in GRAMMAR.fields {
        if matches!(
            spec.role,
            Role::Parent | Role::Kind | Role::Field(RegistrationField::NeedsRegistry)
        ) {
            continue;
        }
        if let FieldUpdate::Set(value) = registration.find_field(match spec.role {
            Role::Field(field) => field,
            Role::Parent | Role::Kind => continue,
        }) {
            text.push_str(&format!(
                "\n    .{}({})",
                spec.spelling,
                GRAMMAR.declared(spec.role, value)
            ));
        }
    }
    for (method, value) in extras {
        text.push_str(&format!("\n    .{method}({})", quote(value)));
    }
    let needs_registry = Role::Field(RegistrationField::NeedsRegistry);
    let owns = matches!(
        registration.find_field(RegistrationField::NeedsRegistry),
        FieldUpdate::Set("true")
    );
    text.push_str(&format!(
        "\n    .{}({})",
        spelling(needs_registry),
        GRAMMAR.declared(needs_registry, if owns { "true" } else { "false" })
    ));
    text
}

/// The validation obligations a plan states for one face.
/// 计划为一个面声明的验证义务。
fn obligations(kind: &str, registration: &RegistrationUpdate) -> Vec<ValidationObligation> {
    stated_obligations(
        kind,
        registration,
        "compile the package so the record lands in its `xirang.faces` section",
    )
}
