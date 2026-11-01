//! The Java language implementation: generate, read, and precisely rewrite one `@Soil(…)`
//! annotation.
//! Java 语言实现：生成、读取并精确改写一条 `@Soil(…)` 注解。
//!
//! Uses the language kit: the capability table, the field table (`FaceGrammar`) and the adapter
//! orchestration come from `crate::languages::kit`; this module owns Java's spelling and its two
//! carriers (the annotation and the build-time `SoilProcessor`).
//! 使用语言 kit：能力表、字段表（`FaceGrammar`）与适配器编排来自 `crate::languages::kit`；本模块只拥有 Java
//! 的拼法与它的两种载体（注解与构建期 `SoilProcessor`）。
//!
//! The carrier is an annotation, and an annotation element must be a **compile-time constant**: a
//! parent is named by a generated `Faces.<MODULE>_NODE_ID` field, never by a runtime expression. The
//! build writes those fields, and this adapter renders the same spelling through
//! `languages::java::names::face_constant`.
//! 载体是注解，而注解元素必须是**编译期常量**：父级由生成的 `Faces.<MODULE>_NODE_ID` 字段点名，绝不是运行期
//! 表达式。构建写出那些字段，本适配器经 `languages::java::names::face_constant` 渲染同一拼法。

use std::sync::OnceLock;

use xirang_kernel::authoring::{
    AuthoringOperation, DiagnosticSeverity, FaceAuthoringPlan, FaceAuthoringRequest, FieldUpdate,
    LanguageCapabilities, ParentReference, PlanDiagnostic, PlannedFile, RegistrationField,
    RegistrationUpdate, ValidationObligation,
};

use crate::languages::kit::adapter::{
    AdapterSpec, EditHooks, Editable, capabilities as kit_capabilities, destination_module,
    modify as kit_modify,
};
use crate::languages::kit::grammar::{
    DeclaredFace, FaceGrammar, FieldSpec, Role, ValueStyle, dotted_identifier,
    obligations as stated_obligations,
};
use crate::registration::LanguageAdapter;

#[path = "java_syntax.rs"]
mod syntax;
use self::syntax::{array, entries, matching_paren, string};

pub mod names;

/// The annotation spellings this adapter reads.
/// 本适配器读取的注解拼法。
pub const MARKERS: &[&str] = &["@Soil(", "@xirang.Soil("];

/// The Java language implementation.
/// Java 语言实现。
#[derive(Clone, Copy, Debug, Default)]
pub struct JavaLanguage;

/// The registration fields a Java annotation carries.
/// Java 注解携带的注册字段。
///
/// `GettingFromOtherRegistry` is deliberately absent: the annotation and the record writer this SDK
/// ships do not carry it, and a field the carrier cannot write is refused by name rather than
/// silently dropped.
/// 有意不含 `GettingFromOtherRegistry`：本 SDK 的注解与记录写入器都不携带它，而载体写不出的字段会按名拒绝，
/// 而不是静默丢掉。
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
    RegistrationField::NeedsRegistry,
];

/// The fields a Java annotation carries, in the order the renderer writes them.
/// Java 注解携带的字段，按渲染器写出的顺序。
pub const GRAMMAR: FaceGrammar = FaceGrammar {
    fields: &[
        FieldSpec {
            role: Role::Parent,
            spelling: "parent",
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
            style: ValueStyle::JavaArray,
        },
        FieldSpec {
            role: Role::Field(RegistrationField::Provides),
            spelling: "provides",
            style: ValueStyle::JavaArray,
        },
        FieldSpec {
            role: Role::Field(RegistrationField::Requires),
            spelling: "requires",
            style: ValueStyle::JavaArray,
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
            role: Role::Field(RegistrationField::NeedsRegistry),
            spelling: "needs_registry",
            style: ValueStyle::Bool {
                when_true: "true",
                when_false: "false",
            },
        },
    ],
};

/// What the Java adapter tells the shared orchestration about itself.
/// Java 适配器告诉共享编排的东西。
const SPEC: AdapterSpec = AdapterSpec {
    language: "java",
    extensions: &["java"],
    fields: FIELDS,
    operations: &[AuthoringOperation::Create, AuthoringOperation::Modify],
    carrying: "the annotation carries text, array and boolean elements",
};

static CAPABILITIES: OnceLock<LanguageCapabilities> = OnceLock::new();

/// The capability table for Java.
/// Java 的能力表。
fn capabilities() -> &'static LanguageCapabilities {
    kit_capabilities(&CAPABILITIES, &SPEC)
}

impl LanguageAdapter for JavaLanguage {
    fn capabilities(&self) -> &'static LanguageCapabilities {
        capabilities()
    }

    fn read(&self, source: &str) -> Result<crate::registration::FaceReadBack, String> {
        crate::languages::kit::adapter::read_back(declared, source)
    }

    fn plan(&self, request: &FaceAuthoringRequest) -> Result<FaceAuthoringPlan, String> {
        crate::languages::kit::adapter::plan(&SPEC, request, create, modify)
    }
}

/// The language-neutral payload of one declaration, for the shared adapter.
/// 一条声明的语言中立载荷，供共享适配器使用。
fn declared(source: &str) -> Result<DeclaredFace, String> {
    read_declaration(source).map(|declaration| declaration.face)
}

/// One `@Soil(…)` declaration, as the text tells it.
/// 一条 `@Soil(…)` 声明，按文本读到的样子。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JavaDeclaration {
    /// The language-neutral payload: parent, registration fields, and unmapped elements.
    /// 语言中立载荷：父级、注册字段与未映射的元素。
    pub face: DeclaredFace,
    /// Byte range of the annotation's argument list (the text between the parentheses).
    /// 注解参数表的字节区间（括号之间的文本）。
    pub arguments: std::ops::Range<usize>,
}

impl JavaDeclaration {
    /// The face kind (the `named` value).
    /// 面种类名（`named` 的取值）。
    pub fn kind(&self) -> String {
        self.face.kind()
    }
}

/// Reads one Java declaration from source text.
/// 从源码文本读出一条 Java 声明。
pub fn read_declaration(source: &str) -> Result<JavaDeclaration, String> {
    let (start, marker) = MARKERS
        .iter()
        .filter_map(|marker| source.find(marker).map(|at| (at, *marker)))
        .min_by_key(|(at, _)| *at)
        .ok_or_else(|| {
            "this file has no `@Soil(…)` declaration; the Java spelling is \
             `@Soil(parent = Faces.ROOT_NODE_ID, named = \"…\")` above the class"
                .to_owned()
        })?;
    let open = start + marker.len() - 1;
    let close = matching_paren(source, open)?;
    let mut registration = RegistrationUpdate::new();
    let mut extras = Vec::new();
    let mut parent = None;
    let mut seen: Vec<String> = Vec::new();
    for (key, value) in entries(&source[open + 1..close])? {
        if seen.iter().any(|name| name == &key) {
            return Err(format!("the declaration names `{key}` twice"));
        }
        seen.push(key.clone());
        match GRAMMAR.lookup(&key).map(|spec| spec.role) {
            Some(Role::Parent) => {
                if value.is_empty() {
                    return Err("`parent` names no parent".to_owned());
                }
                parent = Some(value);
            }
            Some(Role::Kind) => {
                let text = string(&value)?;
                registration.set_normalized(RegistrationField::Kind, FieldUpdate::Set(text));
            }
            Some(Role::Field(field)) => {
                let normalized = normalize(field, &value)?;
                registration.set_normalized(field, FieldUpdate::Set(normalized));
            }
            None => extras.push((key, value)),
        }
    }
    let parent = parent.ok_or_else(|| {
        "the annotation never names `parent`; the parent is part of the face's position, so a \
         declaration without one cannot be registered"
            .to_owned()
    })?;
    if registration
        .find_field(RegistrationField::Kind)
        .value()
        .is_none()
    {
        return Err("a Java declaration must name `named`".to_owned());
    }
    Ok(JavaDeclaration {
        face: DeclaredFace {
            parent,
            registration,
            extras,
        },
        arguments: open + 1..close,
    })
}

/// The field's canonical text from one Java value.
/// 一个 Java 取值对应的字段规范文本。
fn normalize(field: RegistrationField, raw: &str) -> Result<String, String> {
    match field {
        RegistrationField::NeedsRegistry => match raw.trim() {
            "true" => Ok("true".to_owned()),
            "false" => Ok("false".to_owned()),
            other => Err(format!(
                "`needs_registry = {other}` is not a boolean; write `true` or `false`"
            )),
        },
        RegistrationField::Requires => {
            let entries = array(raw)?;
            let mut pairs = Vec::new();
            for entry in entries {
                let Some((capability, provider)) = entry.split_once("=>") else {
                    return Err(format!(
                        "`requires` entry `{entry}` does not name a capability and a provider; \
                         write `\"capability=>provider\"`"
                    ));
                };
                pairs.push(format!("{}=>{}", capability.trim(), provider.trim()));
            }
            pairs.sort();
            pairs.dedup();
            Ok(pairs.join(","))
        }
        field if field.is_list() => {
            let mut entries = array(raw)?;
            entries.sort();
            entries.dedup();
            Ok(entries.join(","))
        }
        _ => string(raw),
    }
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
        FieldUpdate::Clear => return Err("`kind` cannot be empty; it names the face".to_owned()),
        FieldUpdate::Keep => xirang_kernel::authoring::pascal_case(&module),
    };
    let package = package_of(&request.destination)?;
    let parent = parent_reference(request)?;
    let mut registration = request.registration.clone();
    registration.set_normalized(RegistrationField::Kind, FieldUpdate::Set(kind.clone()));
    let public = std::path::Path::new(&request.destination)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .is_some_and(|stem| stem == kind);
    let text = render_file(&package, &kind, &parent, &registration, &[], public);
    let mut diagnostics = vec![
        PlanDiagnostic {
            severity: DiagnosticSeverity::Note,
            message: format!(
                "the declaration is read by `SoilProcessor`; the SDK is `{}` (import \
                 `xirang.Soil`)",
                crate::sdk::java_include_dir().display()
            ),
        },
        PlanDiagnostic {
            severity: DiagnosticSeverity::Note,
            message:
                "the build writes the generated `xirang.Faces` class (`ROOT_NODE_ID` plus one \
                      constant per face module) when `xirang.toml` declares `[record] language = \
                      \"java\"` and `constants`; an annotation element must be a compile-time \
                      constant, so a parent that is not the root needs that class"
                    .to_owned(),
        },
    ];
    if !public {
        diagnostics.push(PlanDiagnostic {
            severity: DiagnosticSeverity::Note,
            message: format!(
                "the class is package-private because the file is not `{kind}.java`; a public type \
                 needs a file named after it"
            ),
        });
    }
    if matches!(request.parent, ParentReference::Object { .. }) {
        diagnostics.push(PlanDiagnostic {
            severity: DiagnosticSeverity::Note,
            message: "the parent is named by a generated `Faces` constant; the package needs \
                      `[record] language = \"java\"` and `constants` so the build writes it"
                .to_owned(),
        });
    }
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
impl Editable for JavaDeclaration {
    fn face(&self) -> &DeclaredFace {
        &self.face
    }

    fn arguments(&self) -> std::ops::Range<usize> {
        self.arguments.clone()
    }

    fn rendered(&self, parent: &str, registration: &RegistrationUpdate) -> String {
        render_arguments(parent, registration, &self.face.extras)
    }
}

const EDIT: EditHooks<JavaDeclaration> = EditHooks {
    read: read_declaration,
    parent: parent_reference,
    obligations,
    carrier: Some("class"),
};

/// Modify one existing declaration, preserving everything the request does not name.
/// 按请求修改一条已有声明，保留请求没有点名的一切。
fn modify(request: &FaceAuthoringRequest) -> Result<FaceAuthoringPlan, String> {
    kit_modify(&capabilities().language, request, &EDIT)
}
/// The Java package a destination file belongs to: its directory's last segment, folded.
/// 目的地文件所属的 Java 包：目录最后一段，折成合法标识符。
fn package_of(destination: &str) -> Result<String, String> {
    let directory = std::path::Path::new(destination)
        .parent()
        .and_then(|parent| parent.file_name())
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .ok_or_else(|| {
            format!(
                "`{destination}` has no directory this adapter can name a Java package after; a \
                 Java face file lives in a package directory"
            )
        })?;
    Ok(crate::languages::naming::java_segment(directory))
}

/// The parent expression a Java annotation can name.
/// Java 注解可以点名的父表达式。
fn parent_reference(request: &FaceAuthoringRequest) -> Result<String, String> {
    match &request.parent {
        ParentReference::Root => Ok("Faces.ROOT_NODE_ID".to_owned()),
        ParentReference::Object { module, .. } if !module.trim().is_empty() => {
            Ok(format!("Faces.{}", names::face_constant(module)))
        }
        ParentReference::Object { .. } => Err(
            "the request names an empty parent module; a Java parent is named by the generated \
             `Faces` constant of a module, which needs a module path"
                .to_owned(),
        ),
        ParentReference::Named(text) if !text.trim().is_empty() => {
            let trimmed = text.trim();
            if !dotted_identifier(trimmed) {
                return Err(format!(
                    "`{trimmed}` is not a Java constant expression this annotation can name; point \
                     at `Faces.<MODULE>_NODE_ID` or another compile-time constant"
                ));
            }
            Ok(trimmed.to_owned())
        }
        ParentReference::Named(_) => {
            Err("the parent reference is empty; a Java face must name a constant".to_owned())
        }
        ParentReference::Catalog { path, .. } => Err(format!(
            "the catalog path `{path}` is not a Java constant; the build generates per-module \
             `Faces` constants, not catalog paths"
        )),
        ParentReference::Identity(_) => Err(
            "the request carries a parent identity, but a Java annotation names a compile-time \
             constant"
                .to_owned(),
        ),
    }
}

/// The whole file one create writes.
/// 新建写出的整份文件。
fn render_file(
    package: &str,
    kind: &str,
    parent: &str,
    registration: &RegistrationUpdate,
    extras: &[(String, String)],
    public: bool,
) -> String {
    let arguments = render_arguments(parent, registration, extras);
    // A public Java class must live in a file named after it. When the destination's stem is the
    // module (the CLI's default `<module>.java`), the class is package-private so the file name can
    // stay the module; a file named after the kind gets the public type.
    // 公开的 Java 类必须住在与它同名的文件里。当目的地主干是模块名（CLI 默认的 `<module>.java`）时，
    // 类写成包私有，文件名得以保持模块名；以种类名命名的文件得到公开类型。
    let visibility = if public {
        "public final class"
    } else {
        "final class"
    };
    format!(
        "package {package};\n\nimport xirang.Faces;\nimport xirang.Soil;\n\n/**\n * {kind} is the business type this registration face is named after.\n */\n@Soil({arguments})\n{visibility} {kind} {{\n    private {kind}() {{}}\n}}\n"
    )
}

fn render_arguments(
    parent: &str,
    registration: &RegistrationUpdate,
    extras: &[(String, String)],
) -> String {
    let spelling = |role| {
        GRAMMAR
            .spec(role)
            .map(|spec| spec.spelling)
            .expect("the Java grammar carries every role the renderer writes")
    };
    // A Java annotation's element list has **no trailing comma** (unlike a Go slice or a JS array),
    // so the lines are joined rather than each one being terminated.
    // Java 注解的元素表**不允许尾逗号**（与 Go 切片或 JS 数组不同），因此这些行是 join 的，而不是逐行以逗号结尾。
    let mut lines: Vec<String> = Vec::new();
    lines.push(format!(
        "        {} = {}",
        spelling(Role::Parent),
        GRAMMAR.declared(Role::Parent, parent)
    ));
    lines.push(format!(
        "        {} = {}",
        spelling(Role::Kind),
        GRAMMAR.declared(
            Role::Kind,
            registration
                .find_field(RegistrationField::Kind)
                .value()
                .copied()
                .unwrap_or("")
        )
    ));
    for spec in GRAMMAR.fields {
        if matches!(
            spec.role,
            Role::Parent | Role::Kind | Role::Field(RegistrationField::NeedsRegistry)
        ) {
            continue;
        }
        let field = match spec.role {
            Role::Field(field) => field,
            Role::Parent | Role::Kind => continue,
        };
        if let FieldUpdate::Set(value) = registration.find_field(field)
            && !value.is_empty()
        {
            lines.push(format!(
                "        {} = {}",
                spec.spelling,
                GRAMMAR.declared(spec.role, value)
            ));
        }
    }
    for (key, value) in extras {
        lines.push(format!("        {key} = {value}"));
    }
    let needs_registry = Role::Field(RegistrationField::NeedsRegistry);
    let owns = matches!(
        registration.find_field(RegistrationField::NeedsRegistry),
        FieldUpdate::Set("true")
    );
    if owns {
        lines.push(format!(
            "        {} = {}",
            spelling(needs_registry),
            GRAMMAR.declared(needs_registry, "true")
        ));
    }
    format!("\n{}\n    ", lines.join(",\n"))
}

fn obligations(kind: &str, registration: &RegistrationUpdate) -> Vec<ValidationObligation> {
    stated_obligations(
        kind,
        registration,
        "compile the package so `SoilProcessor` writes the declaration and the build registers it",
    )
}

#[cfg(test)]
#[path = "java_tests.rs"]
mod java_tests;
