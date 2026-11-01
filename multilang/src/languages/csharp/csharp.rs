//! The C# language implementation: generate, read, and precisely rewrite one `[Soil(…)]`
//! attribute.
//! C# 语言实现：生成、读取并精确改写一条 `[Soil(…)]` 特性。
//!
//! Uses the language kit: the capability table, the field table (`FaceGrammar`) and the adapter
//! orchestration come from `crate::languages::kit`; this module owns C#'s spelling and its two
//! carriers (the attribute and the build-time Roslyn generator).
//! 使用语言 kit：能力表、字段表（`FaceGrammar`）与适配器编排来自 `crate::languages::kit`；本模块只拥有 C# 的
//! 拼法与它的两种载体（特性与构建期 Roslyn 生成器）。
//!
//! The carrier is an attribute, and an attribute argument must be a **compile-time constant**: a
//! parent is named by a generated `XirangFaces.<Module>NodeId` field, never by a runtime expression.
//! The build writes those fields, and this adapter renders the same spelling through
//! `languages::csharp::names::face_constant`.
//! 载体是特性，而特性实参必须是**编译期常量**：父级由生成的 `XirangFaces.<Module>NodeId` 字段点名，绝不是
//! 运行期表达式。构建写出那些字段，本适配器经 `languages::csharp::names::face_constant` 渲染同一拼法。

use std::sync::OnceLock;

use xirang_kernel::authoring::{
    AuthoringOperation, DiagnosticSeverity, FaceAuthoringPlan, FaceAuthoringRequest, FieldUpdate,
    LanguageCapabilities, ParentReference, PlanDiagnostic, PlannedEdit, PlannedFile,
    RegistrationField, RegistrationUpdate, ValidationObligation,
};

use crate::languages::kit::adapter::{AdapterSpec, capabilities as kit_capabilities};
use crate::languages::kit::grammar::{
    DeclaredFace, FaceGrammar, FieldSpec, Role, ValueStyle, obligations as stated_obligations,
};
use crate::registration::LanguageAdapter;

pub mod names;

#[path = "csharp_syntax.rs"]
mod syntax;
use self::syntax::{array, entries, matching_paren, string};

/// The attribute spellings this adapter reads.
/// 本适配器读取的特性拼法。
pub const MARKERS: &[&str] = &["[Soil(", "[Xirang.Soil("];

/// The C# language implementation.
/// C# 语言实现。
#[derive(Clone, Copy, Debug, Default)]
pub struct CSharpLanguage;

/// The registration fields a C# attribute carries.
/// C# 特性携带的注册字段。
///
/// `GettingFromOtherRegistry` is deliberately absent: the attribute and the record writer this SDK
/// ships do not carry it, and a field the carrier cannot write is refused by name rather than
/// silently dropped.
/// 有意不含 `GettingFromOtherRegistry`：本 SDK 的特性与记录写入器都不携带它，而载体写不出的字段会按名拒绝，
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

/// The fields a C# attribute carries, in the order the renderer writes them.
/// C# 特性携带的字段，按渲染器写出的顺序。
pub const GRAMMAR: FaceGrammar = FaceGrammar {
    fields: &[
        FieldSpec {
            role: Role::Parent,
            spelling: "Parent",
            style: ValueStyle::Expr,
        },
        FieldSpec {
            role: Role::Kind,
            spelling: "Named",
            style: ValueStyle::Quoted,
        },
        FieldSpec {
            role: Role::Field(RegistrationField::StableName),
            spelling: "StableName",
            style: ValueStyle::Quoted,
        },
        FieldSpec {
            role: Role::Field(RegistrationField::Exports),
            spelling: "Exports",
            style: ValueStyle::CSharpArray,
        },
        FieldSpec {
            role: Role::Field(RegistrationField::Provides),
            spelling: "Provides",
            style: ValueStyle::CSharpArray,
        },
        FieldSpec {
            role: Role::Field(RegistrationField::Requires),
            spelling: "Requires",
            style: ValueStyle::CSharpArray,
        },
        FieldSpec {
            role: Role::Field(RegistrationField::Preset),
            spelling: "Preset",
            style: ValueStyle::Quoted,
        },
        FieldSpec {
            role: Role::Field(RegistrationField::Parts),
            spelling: "Parts",
            style: ValueStyle::Quoted,
        },
        FieldSpec {
            role: Role::Field(RegistrationField::NameZh),
            spelling: "NameZh",
            style: ValueStyle::Quoted,
        },
        FieldSpec {
            role: Role::Field(RegistrationField::NameEn),
            spelling: "NameEn",
            style: ValueStyle::Quoted,
        },
        FieldSpec {
            role: Role::Field(RegistrationField::SummaryZh),
            spelling: "SummaryZh",
            style: ValueStyle::Quoted,
        },
        FieldSpec {
            role: Role::Field(RegistrationField::SummaryEn),
            spelling: "SummaryEn",
            style: ValueStyle::Quoted,
        },
        FieldSpec {
            role: Role::Field(RegistrationField::NeedsRegistry),
            spelling: "NeedsRegistry",
            style: ValueStyle::Bool {
                when_true: "true",
                when_false: "false",
            },
        },
    ],
};

/// What the C# adapter tells the shared orchestration about itself.
/// C# 适配器告诉共享编排的东西。
const SPEC: AdapterSpec = AdapterSpec {
    language: "csharp",
    extensions: &["cs"],
    fields: FIELDS,
    operations: &[AuthoringOperation::Create, AuthoringOperation::Modify],
    carrying: "the attribute carries text, array and boolean arguments",
};

static CAPABILITIES: OnceLock<LanguageCapabilities> = OnceLock::new();

/// The capability table for C#.
/// C# 的能力表。
fn capabilities() -> &'static LanguageCapabilities {
    kit_capabilities(&CAPABILITIES, &SPEC)
}

impl LanguageAdapter for CSharpLanguage {
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

/// One `[Soil(…)]` declaration, as the text tells it.
/// 一条 `[Soil(…)]` 声明，按文本读到的样子。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CSharpDeclaration {
    /// The language-neutral payload: parent, registration fields, and unmapped arguments.
    /// 语言中立载荷：父级、注册字段与未映射的实参。
    pub face: DeclaredFace,
    /// Byte range of the attribute's argument list (the text between the parentheses).
    /// 特性实参表的字节区间（括号之间的文本）。
    pub arguments: std::ops::Range<usize>,
}

impl CSharpDeclaration {
    /// The face kind (the `Named` value).
    /// 面种类名（`Named` 的取值）。
    pub fn kind(&self) -> String {
        self.face.kind()
    }
}

/// Reads one C# declaration from source text.
/// 从源码文本读出一条 C# 声明。
pub fn read_declaration(source: &str) -> Result<CSharpDeclaration, String> {
    let (start, marker) = MARKERS
        .iter()
        .filter_map(|marker| source.find(marker).map(|at| (at, *marker)))
        .min_by_key(|(at, _)| *at)
        .ok_or_else(|| {
            "this file has no `[Soil(…)]` declaration; the C# spelling is \
             `[Soil(Parent = XirangFaces.RootNodeId, Named = \"…\")]` above the class"
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
                    return Err("`Parent` names no parent".to_owned());
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
        "the attribute never names `Parent`; the parent is part of the face's position, so a \
         declaration without one cannot be registered"
            .to_owned()
    })?;
    if registration
        .find_field(RegistrationField::Kind)
        .value()
        .is_none()
    {
        return Err("a C# declaration must name `Named`".to_owned());
    }
    Ok(CSharpDeclaration {
        face: DeclaredFace {
            parent,
            registration,
            extras,
        },
        arguments: open + 1..close,
    })
}

/// The field's canonical text from one C# value.
/// 一个 C# 取值对应的字段规范文本。
fn normalize(field: RegistrationField, raw: &str) -> Result<String, String> {
    match field {
        RegistrationField::NeedsRegistry => match raw.trim() {
            "true" => Ok("true".to_owned()),
            "false" => Ok("false".to_owned()),
            other => Err(format!(
                "`NeedsRegistry = {other}` is not a boolean; write `true` or `false`"
            )),
        },
        RegistrationField::Requires => {
            let entries = array(raw)?;
            let mut pairs = Vec::new();
            for entry in entries {
                let Some((capability, provider)) = entry.split_once("=>") else {
                    return Err(format!(
                        "`Requires` entry `{entry}` does not name a capability and a provider; \
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
    let module = module_of(&request.destination)?;
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
    let namespace = namespace_of(&request.destination)?;
    let parent = parent_reference(request)?;
    let mut registration = request.registration.clone();
    registration.set_normalized(RegistrationField::Kind, FieldUpdate::Set(kind.clone()));
    let text = render_file(&namespace, &kind, &parent, &registration, &[]);
    let mut diagnostics = vec![
        PlanDiagnostic {
            severity: DiagnosticSeverity::Note,
            message: format!(
                "the declaration is read by the Roslyn generator; the SDK is `{}` (import \
                 `Xirang.Soil`)",
                crate::sdk::csharp_include_dir().display()
            ),
        },
        PlanDiagnostic {
            severity: DiagnosticSeverity::Note,
            message: "the build writes the generated `XirangFaces` class (`RootNodeId` plus one \
                      constant per face module) when `xirang.toml` declares `[record] language = \
                      \"csharp\"` and `constants`; an attribute argument must be a compile-time \
                      constant, so a parent that is not the root needs that class"
                .to_owned(),
        },
    ];
    if matches!(request.parent, ParentReference::Object { .. }) {
        diagnostics.push(PlanDiagnostic {
            severity: DiagnosticSeverity::Note,
            message:
                "the parent is named by a generated `XirangFaces` constant; the package needs \
                      `[record] language = \"csharp\"` and `constants` so the build writes it"
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

/// Modify one existing declaration, preserving everything the request does not name.
/// 按请求修改一条已有声明，保留请求没有点名的一切。
fn modify(request: &FaceAuthoringRequest) -> Result<FaceAuthoringPlan, String> {
    let source = request.source_context.source_text().ok_or_else(|| {
        format!(
            "{} does not exist; modify needs the file",
            request.destination
        )
    })?;
    let declaration = read_declaration(source)?;
    let module = module_of(&request.destination)?;
    if let FieldUpdate::Set(named) = request.registration.find_field(RegistrationField::Module)
        && named != module
    {
        return Err(format!(
            "changing the module from `{module}` to `{named}` is a move, not a field edit; this \
             adapter rewrites the declaration in place"
        ));
    }
    let current_kind = declaration.kind();
    let kind = match request.registration.find_field(RegistrationField::Kind) {
        FieldUpdate::Keep => current_kind.clone(),
        FieldUpdate::Set(kind) if kind == current_kind => current_kind.clone(),
        FieldUpdate::Set(kind) => {
            return Err(format!(
                "changing `kind` from `{current_kind}` to `{kind}` renames the class and the face's \
                 identity; that is a rename, not a field edit"
            ));
        }
        FieldUpdate::Clear => return Err("`kind` cannot be empty; it names the face".to_owned()),
    };
    let mut registration = declaration.face.registration.clone();
    for (field, _) in request.registration.named() {
        let update = match request.registration.find_field(field) {
            FieldUpdate::Keep => FieldUpdate::Keep,
            FieldUpdate::Clear => FieldUpdate::Clear,
            FieldUpdate::Set(value) => FieldUpdate::Set(value.to_owned()),
        };
        registration.set_normalized(field, update);
    }
    registration.set_normalized(RegistrationField::Kind, FieldUpdate::Set(kind.clone()));
    let parent = parent_reference(request)?;
    let replacement = render_arguments(&parent, &registration, &declaration.face.extras);
    let mut updated = String::with_capacity(source.len() + replacement.len());
    updated.push_str(&source[..declaration.arguments.start]);
    updated.push_str(&replacement);
    updated.push_str(&source[declaration.arguments.end..]);
    let mut diagnostics = Vec::new();
    if updated == source {
        diagnostics.push(PlanDiagnostic {
            severity: DiagnosticSeverity::Note,
            message: "the request changes nothing this declaration carries".to_owned(),
        });
    }
    let edits = if updated == source {
        Vec::new()
    } else {
        vec![PlannedEdit {
            start: declaration.arguments.start,
            end: declaration.arguments.end,
            replacement,
        }]
    };
    Ok(FaceAuthoringPlan {
        language: capabilities().language.clone(),
        operation: AuthoringOperation::Modify,
        files: vec![PlannedFile {
            path: request.destination.clone(),
            original: Some(source.to_owned()),
            updated,
            edits,
            remove: false,
        }],
        diagnostics,
        obligations: obligations(&kind, &registration),
    })
}

/// The module a destination file names (its stem).
/// 目的地文件点名的模块（文件名主干）。
fn module_of(destination: &str) -> Result<String, String> {
    std::path::Path::new(destination)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .map(|stem| stem.replace(['-', '.'], "_"))
        .ok_or_else(|| {
            format!("`{destination}` has no file stem this adapter can name a module after")
        })
}

/// The C# namespace a destination file belongs to: its directory's last segment, PascalCase.
/// 目的地文件所属的 C# 命名空间：目录最后一段，首字母大写。
fn namespace_of(destination: &str) -> Result<String, String> {
    let directory = std::path::Path::new(destination)
        .parent()
        .and_then(|parent| parent.file_name())
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .ok_or_else(|| {
            format!(
                "`{destination}` has no directory this adapter can name a C# namespace after; a C# \
                 face file lives in a namespace directory"
            )
        })?;
    Ok(xirang_kernel::authoring::pascal_case(
        &crate::languages::naming::csharp_segment(directory),
    ))
}

/// The parent expression a C# attribute can name.
/// C# 特性可以点名的父表达式。
fn parent_reference(request: &FaceAuthoringRequest) -> Result<String, String> {
    match &request.parent {
        ParentReference::Root => Ok("XirangFaces.RootNodeId".to_owned()),
        ParentReference::Object { module, .. } if !module.trim().is_empty() => {
            Ok(format!("XirangFaces.{}", names::face_constant(module)))
        }
        ParentReference::Object { .. } => Err(
            "the request names an empty parent module; a C# parent is named by the generated \
             `XirangFaces` constant of a module, which needs a module path"
                .to_owned(),
        ),
        ParentReference::Named(text) if !text.trim().is_empty() => {
            let trimmed = text.trim();
            if !constant_expression(trimmed) {
                return Err(format!(
                    "`{trimmed}` is not a C# constant expression this attribute can name; point at \
                     `XirangFaces.<Module>NodeId` or another compile-time constant"
                ));
            }
            Ok(trimmed.to_owned())
        }
        ParentReference::Named(_) => {
            Err("the parent reference is empty; a C# face must name a constant".to_owned())
        }
        ParentReference::Catalog { path, .. } => Err(format!(
            "the catalog path `{path}` is not a C# constant; the build generates per-module \
             `XirangFaces` constants, not catalog paths"
        )),
        ParentReference::Identity(_) => Err(
            "the request carries a parent identity, but a C# attribute names a compile-time \
             constant"
                .to_owned(),
        ),
    }
}

/// Whether `text` is a dotted identifier chain (a constant expression this adapter can write).
/// `text` 是否是点号连接的标识符链（本适配器能写出的常量表达式）。
fn constant_expression(text: &str) -> bool {
    text.split('.').all(|segment| {
        let mut characters = segment.chars();
        characters
            .next()
            .is_some_and(|first| first == '_' || first.is_ascii_alphabetic())
            && characters.all(|character| character == '_' || character.is_ascii_alphanumeric())
    })
}

/// The whole file one create writes.
/// 新建写出的整份文件。
fn render_file(
    namespace: &str,
    kind: &str,
    parent: &str,
    registration: &RegistrationUpdate,
    extras: &[(String, String)],
) -> String {
    let arguments = render_arguments(parent, registration, extras);
    format!(
        "using Xirang;\n\nnamespace {namespace};\n\n/// <summary>\n/// {kind} is the business type this registration face is named after.\n/// </summary>\n[Soil({arguments})]\npublic sealed class {kind}\n{{\n}}\n"
    )
}

/// The attribute's arguments, one `Name = value` per line, indented four spaces.
/// 特性实参，每行一个 `Name = value`，缩进四个空格。
fn render_arguments(
    parent: &str,
    registration: &RegistrationUpdate,
    extras: &[(String, String)],
) -> String {
    let spelling = |role| {
        GRAMMAR
            .spec(role)
            .map(|spec| spec.spelling)
            .expect("the C# grammar carries every role the renderer writes")
    };
    let mut lines: Vec<String> = Vec::new();
    lines.push(format!(
        "    {} = {}",
        spelling(Role::Parent),
        GRAMMAR.declared(Role::Parent, parent)
    ));
    lines.push(format!(
        "    {} = {}",
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
                "    {} = {}",
                spec.spelling,
                GRAMMAR.declared(spec.role, value)
            ));
        }
    }
    for (key, value) in extras {
        lines.push(format!("    {key} = {value}"));
    }
    let needs_registry = Role::Field(RegistrationField::NeedsRegistry);
    let owns = matches!(
        registration.find_field(RegistrationField::NeedsRegistry),
        FieldUpdate::Set("true")
    );
    if owns {
        lines.push(format!(
            "    {} = {}",
            spelling(needs_registry),
            GRAMMAR.declared(needs_registry, "true")
        ));
    }
    format!("\n{}\n", lines.join(",\n"))
}

fn obligations(kind: &str, registration: &RegistrationUpdate) -> Vec<ValidationObligation> {
    stated_obligations(
        kind,
        registration,
        "build the project so the Roslyn generator writes the declaration and the build registers it",
    )
}

#[cfg(test)]
#[path = "csharp_tests.rs"]
mod csharp_tests;
