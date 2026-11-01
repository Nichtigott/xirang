//! The Go language implementation: generate, read, and precisely rewrite one `xirang.Soil{…}`
//! declaration.
//! Go 语言实现：生成、读取并精确改写一条 `xirang.Soil{…}` 声明。
//!
//! Uses the language kit: the capability table, the field table (`FaceGrammar`) and the adapter
//! orchestration come from `crate::languages::kit`; this module owns Go's spelling and its two
//! carriers (the package-scope declaration and the build-time `xirang-faces` generator).
//! 使用语言 kit：能力表、字段表（`FaceGrammar`）与适配器编排来自 `crate::languages::kit`；本模块只拥有
//! Go 的拼法与它的两种载体（包作用域声明与构建期 `xirang-faces` 生成器）。
//!
//! The carrier is a composite literal, not a decorator or an attribute: a face is one
//! `var _ = xirang.Soil{…}` whose fields name the parent and the registration fields. The build's
//! generator parses the same literal statically and never runs the package.
//! 载体是一个复合字面量，而不是装饰器或特性：一个面就是一条 `var _ = xirang.Soil{…}`，字段点名父级与
//! 注册字段。构建的生成器静态解析同一个字面量，从不运行包。

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
#[path = "go_syntax.rs"]
mod syntax;
use self::syntax::{entries, matching_brace, slice, string};

/// The declaration markers this adapter reads.
/// 本适配器读取的声明标记。
pub const MARKERS: &[&str] = &["xirang.Soil{", "Soil{"];

/// The Go language implementation.
/// Go 语言实现。
#[derive(Clone, Copy, Debug, Default)]
pub struct GoLanguage;

/// The registration fields a Go declaration carries.
/// Go 声明携带的注册字段。
///
/// `GettingFromOtherRegistry` is deliberately absent: the Go `Soil` struct and the record writer
/// this SDK ships do not carry it, and a field the carrier cannot write is refused by name rather
/// than silently dropped.
/// 有意不含 `GettingFromOtherRegistry`：本 SDK 的 Go `Soil` 结构与记录写入器都不携带它，而载体写不出的
/// 字段会按名拒绝，而不是静默丢掉。
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

/// The fields a Go declaration carries, in the order the renderer writes them.
/// Go 声明携带的字段，按渲染器写出的顺序。
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
            style: ValueStyle::SliceList,
        },
        FieldSpec {
            role: Role::Field(RegistrationField::Provides),
            spelling: "Provides",
            style: ValueStyle::SliceList,
        },
        FieldSpec {
            role: Role::Field(RegistrationField::Requires),
            spelling: "Requires",
            style: ValueStyle::SliceList,
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

/// What the Go adapter tells the shared orchestration about itself.
/// Go 适配器告诉共享编排的东西。
const SPEC: AdapterSpec = AdapterSpec {
    language: "go",
    extensions: &["go"],
    fields: FIELDS,
    operations: &[AuthoringOperation::Create, AuthoringOperation::Modify],
    carrying: "the composite literal carries text, typed slices and a boolean",
};

static CAPABILITIES: OnceLock<LanguageCapabilities> = OnceLock::new();

/// The capability table for Go.
/// Go 的能力表。
fn capabilities() -> &'static LanguageCapabilities {
    kit_capabilities(&CAPABILITIES, &SPEC)
}

impl LanguageAdapter for GoLanguage {
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

/// One `xirang.Soil{…}` declaration, as the text tells it.
/// 一条 `xirang.Soil{…}` 声明，按文本读到的样子。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GoDeclaration {
    /// The language-neutral payload: parent, registration fields, and unmapped fields.
    /// 语言中立载荷：父级、注册字段与未映射的字段。
    pub face: DeclaredFace,
    /// Byte range of the composite literal's body (the text between the braces).
    /// 复合字面量体的字节区间（花括号之间的文本）。
    pub block: std::ops::Range<usize>,
}

impl GoDeclaration {
    /// The face kind (the `Named` value).
    /// 面种类名（`Named` 的取值）。
    pub fn kind(&self) -> String {
        self.face.kind()
    }
}

/// Reads one Go declaration from source text.
/// 从源码文本读出一条 Go 声明。
pub fn read_declaration(source: &str) -> Result<GoDeclaration, String> {
    let (start, marker) = MARKERS
        .iter()
        .filter_map(|marker| source.find(marker).map(|at| (at, *marker)))
        .min_by_key(|(at, _)| *at)
        .ok_or_else(|| {
            "this file has no `xirang.Soil{…}` declaration; the Go spelling is \
             `var _ = xirang.Soil{ Parent: rootNodeID, Named: \"…\" }`"
                .to_owned()
        })?;
    let open = start + marker.len() - 1;
    let close = matching_brace(source, open)?;
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
        "the declaration never names `Parent`; the parent is part of the face's position, so a \
         declaration without one cannot be registered"
            .to_owned()
    })?;
    if registration
        .find_field(RegistrationField::Kind)
        .value()
        .is_none()
    {
        return Err("a Go declaration must name `Named`".to_owned());
    }
    Ok(GoDeclaration {
        face: DeclaredFace {
            parent,
            registration,
            extras,
        },
        block: open + 1..close,
    })
}

/// The field's canonical text from one Go value.
/// 一个 Go 取值对应的字段规范文本。
fn normalize(field: RegistrationField, raw: &str) -> Result<String, String> {
    match field {
        RegistrationField::NeedsRegistry => match raw.trim() {
            "true" => Ok("true".to_owned()),
            "false" => Ok("false".to_owned()),
            other => Err(format!(
                "`NeedsRegistry: {other}` is not a boolean; write `true` or `false`"
            )),
        },
        RegistrationField::Requires => {
            let entries = slice(raw)?;
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
            let mut entries = slice(raw)?;
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
    let package = package_of(&request.destination)?;
    let parent = parent_reference(request)?;
    let mut registration = request.registration.clone();
    registration.set_normalized(RegistrationField::Kind, FieldUpdate::Set(kind.clone()));
    let text = render_file(&package, &kind, &parent, &registration, &[]);
    let mut diagnostics = vec![
        PlanDiagnostic {
            severity: DiagnosticSeverity::Note,
            message: format!(
                "the declaration is parsed by `xirang-faces`; the SDK module is `{}` (import \
                 `xirang.dev/sdk/xirang`)",
                crate::sdk::go_include_dir().display()
            ),
        },
        PlanDiagnostic {
            severity: DiagnosticSeverity::Note,
            message:
                "the build writes the generated `faces.go` with `rootNodeID` and one constant \
                      per face module; the file does not compile before that generated file exists"
                    .to_owned(),
        },
    ];
    if matches!(request.parent, ParentReference::Object { .. }) {
        diagnostics.push(PlanDiagnostic {
            severity: DiagnosticSeverity::Note,
            message: "the parent is named by a generated constant; the package needs \
                      `[record] language = \"go\"` and `constants` in `xirang.toml` so the build \
                      writes it"
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
                "changing `kind` from `{current_kind}` to `{kind}` renames the business type and \
                 the face's identity; that is a rename, not a field edit"
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
    let replacement = render_block(&parent, &registration, &declaration.face.extras);
    let mut updated = String::with_capacity(source.len() + replacement.len());
    updated.push_str(&source[..declaration.block.start]);
    updated.push_str(&replacement);
    updated.push_str(&source[declaration.block.end..]);
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
            start: declaration.block.start,
            end: declaration.block.end,
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

/// The Go package a destination file belongs to: its directory's last segment, folded.
/// 目的地文件所属的 Go 包：目录最后一段，折成合法标识符。
fn package_of(destination: &str) -> Result<String, String> {
    let directory = std::path::Path::new(destination)
        .parent()
        .and_then(|parent| parent.file_name())
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .ok_or_else(|| {
            format!(
                "`{destination}` has no directory this adapter can name a Go package after; a Go \
                 face file lives in a package directory"
            )
        })?;
    Ok(crate::languages::naming::go_segment(directory))
}

/// The parent expression a Go declaration can name.
/// Go 声明可以点名的父表达式。
fn parent_reference(request: &FaceAuthoringRequest) -> Result<String, String> {
    match &request.parent {
        ParentReference::Root => Ok("rootNodeID".to_owned()),
        ParentReference::Named(text) if !text.trim().is_empty() => {
            let trimmed = text.trim();
            if !go_expression(trimmed) {
                return Err(format!(
                    "`{trimmed}` is not a Go constant this declaration can name; point at an \
                     identifier or a dotted selector such as `rootNodeID`"
                ));
            }
            Ok(trimmed.to_owned())
        }
        ParentReference::Named(_) => {
            Err("the parent reference is empty; a Go face must name a constant".to_owned())
        }
        ParentReference::Object { module, .. } if !module.trim().is_empty() => {
            Ok(names::face_constant(module))
        }
        ParentReference::Object { .. } => Err(
            "the request names an empty parent module; a Go parent is named by the generated \
             constant of a module, which needs a module path"
                .to_owned(),
        ),
        ParentReference::Catalog { path, .. } => Err(format!(
            "the catalog path `{path}` is not a Go constant; the build does not generate one for it \
             yet"
        )),
        ParentReference::Identity(_) => Err(
            "the request carries a parent identity, but a Go declaration names a constant the build \
             generated or the author wrote"
                .to_owned(),
        ),
    }
}

/// Whether `text` is a Go expression this adapter can write: identifiers joined by dots, or a
/// 32-character hex string literal (the generated constants are strings).
/// `text` 是否是一个本适配器能写出的 Go 表达式：点号连接的标识符，或 32 字符十六进制字符串字面量
/// （生成的常量是字符串）。
fn go_expression(text: &str) -> bool {
    if text.starts_with('"') && text.ends_with('"') && text.len() == 34 {
        return text[1..33]
            .chars()
            .all(|character| character.is_ascii_hexdigit());
    }
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
    package: &str,
    kind: &str,
    parent: &str,
    registration: &RegistrationUpdate,
    extras: &[(String, String)],
) -> String {
    let block = render_block(parent, registration, extras);
    format!(
        "package {package}\n\n\
         import \"xirang.dev/sdk/xirang\"\n\n\
         // {kind} is the business type this registration face is named after.\n\
         type {kind} struct{{}}\n\n\
         // The face declares itself once, at package scope.\n\
         var _ = xirang.Soil{{{block}}}\n"
    )
}

/// The composite literal's body, one `Key: value,` per line, indented with a tab.
/// 复合字面量体，每行一个 `Key: value,`，用一个制表符缩进。
fn render_block(
    parent: &str,
    registration: &RegistrationUpdate,
    extras: &[(String, String)],
) -> String {
    let spelling = |role| {
        GRAMMAR
            .spec(role)
            .map(|spec| spec.spelling)
            .expect("the Go grammar carries every role the renderer writes")
    };
    let mut out = String::from("\n");
    out.push_str(&format!(
        "\t{}: {},\n",
        spelling(Role::Parent),
        GRAMMAR.declared(Role::Parent, parent)
    ));
    out.push_str(&format!(
        "\t{}: {},\n",
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
            out.push_str(&format!(
                "\t{}: {},\n",
                spec.spelling,
                GRAMMAR.declared(spec.role, value)
            ));
        }
    }
    for (key, value) in extras {
        out.push_str(&format!("\t{key}: {value},\n"));
    }
    let needs_registry = Role::Field(RegistrationField::NeedsRegistry);
    let owns = matches!(
        registration.find_field(RegistrationField::NeedsRegistry),
        FieldUpdate::Set("true")
    );
    if owns {
        out.push_str(&format!(
            "\t{}: {},\n",
            spelling(needs_registry),
            GRAMMAR.declared(needs_registry, "true")
        ));
    }
    out
}

fn obligations(kind: &str, registration: &RegistrationUpdate) -> Vec<ValidationObligation> {
    stated_obligations(
        kind,
        registration,
        "compile the package so the Go generator reads the declaration and the build registers it",
    )
}

#[cfg(test)]
#[path = "go_tests.rs"]
mod go_tests;
