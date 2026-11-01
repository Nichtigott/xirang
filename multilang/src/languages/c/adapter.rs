//! The C authoring adapter: generate, read, and precisely rewrite one `XIRANG_SOIL` declaration.
//! C 创作适配器：生成、读取并精确改写一条 `XIRANG_SOIL` 声明。
//!
//! The carrier is a pinned struct initializer placed in the object's `xirang.faces` section. The
//! author writes designated initializers; the build compiles the source and reads the section. The
//! parent is named by a generated constant (`XIRANG_ROOT_NODE_ID` or `XIRANG_FACES_<flat>`), never
//! by sixteen baked bytes.
//! 载体是放进目标文件 `xirang.faces` 段的定长结构体初始化。作者写指定初始化器；构建编译源码并读段。父级由
//! 生成常量点名（`XIRANG_ROOT_NODE_ID` 或 `XIRANG_FACES_<flat>`），绝不是烘焙的十六字节。

use std::sync::OnceLock;

use xirang_kernel::authoring::{
    AuthoringOperation, DiagnosticSeverity, FaceAuthoringPlan, FaceAuthoringRequest, FieldUpdate,
    LanguageCapabilities, ParentReference, PlanDiagnostic, PlannedEdit, PlannedFile,
    RegistrationField, RegistrationUpdate, ValidationObligation,
};

use super::CLanguage;
use super::names;
use crate::languages::kit::adapter::{AdapterSpec, capabilities as kit_capabilities};
use crate::languages::kit::grammar::{
    DeclaredFace, FaceGrammar, FieldSpec, Role, ValueStyle, obligations as stated_obligations,
};
use crate::registration::LanguageAdapter;

#[path = "c_syntax.rs"]
mod syntax;
use self::syntax::{entries, matching_brace, string};

/// The declaration marker this adapter reads.
/// 本适配器读取的声明标记。
pub const MARKER: &str = "XIRANG_SOIL";

/// The registration fields a C declaration carries.
/// C 声明携带的注册字段。
///
/// Unlike the Java/Go/C# carriers, the C record has a `from_registry` member, so the field travels
/// here rather than being refused.
/// 与 Java/Go/C# 载体不同，C 记录有 `from_registry` 成员，因此该字段在这里可以携带，而不是被拒绝。
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
    RegistrationField::GettingFromOtherRegistry,
];

/// The fields a C declaration carries, in the order the renderer writes them.
/// C 声明携带的字段，按渲染器写出的顺序。
pub const GRAMMAR: FaceGrammar = FaceGrammar {
    fields: &[
        FieldSpec {
            role: Role::Parent,
            spelling: ".parent",
            style: ValueStyle::Expr,
        },
        FieldSpec {
            role: Role::Kind,
            spelling: ".kind",
            style: ValueStyle::Quoted,
        },
        FieldSpec {
            role: Role::Field(RegistrationField::StableName),
            spelling: ".stable_name",
            style: ValueStyle::Quoted,
        },
        FieldSpec {
            role: Role::Field(RegistrationField::Exports),
            spelling: ".exports",
            style: ValueStyle::Quoted,
        },
        FieldSpec {
            role: Role::Field(RegistrationField::Provides),
            spelling: ".provides",
            style: ValueStyle::Quoted,
        },
        FieldSpec {
            role: Role::Field(RegistrationField::Requires),
            spelling: ".requires",
            style: ValueStyle::Quoted,
        },
        FieldSpec {
            role: Role::Field(RegistrationField::Preset),
            spelling: ".preset",
            style: ValueStyle::Quoted,
        },
        FieldSpec {
            role: Role::Field(RegistrationField::Parts),
            spelling: ".parts",
            style: ValueStyle::Quoted,
        },
        FieldSpec {
            role: Role::Field(RegistrationField::NameZh),
            spelling: ".name_zh",
            style: ValueStyle::Quoted,
        },
        FieldSpec {
            role: Role::Field(RegistrationField::NameEn),
            spelling: ".name_en",
            style: ValueStyle::Quoted,
        },
        FieldSpec {
            role: Role::Field(RegistrationField::SummaryZh),
            spelling: ".summary_zh",
            style: ValueStyle::Quoted,
        },
        FieldSpec {
            role: Role::Field(RegistrationField::SummaryEn),
            spelling: ".summary_en",
            style: ValueStyle::Quoted,
        },
        FieldSpec {
            role: Role::Field(RegistrationField::NeedsRegistry),
            spelling: ".flags",
            style: ValueStyle::Bool {
                when_true: "XIRANG_OWNS_REGISTRY",
                when_false: "0",
            },
        },
        FieldSpec {
            role: Role::Field(RegistrationField::GettingFromOtherRegistry),
            spelling: ".from_registry",
            style: ValueStyle::Quoted,
        },
    ],
};

/// What the C adapter tells the shared orchestration about itself.
/// C 适配器告诉共享编排的东西。
const SPEC: AdapterSpec = AdapterSpec {
    language: "c",
    extensions: &["c"],
    fields: FIELDS,
    operations: &[AuthoringOperation::Create, AuthoringOperation::Modify],
    carrying: "the designated initializer carries strings and the registry flag",
};

static CAPABILITIES: OnceLock<LanguageCapabilities> = OnceLock::new();

/// The capability table for C.
/// C 的能力表。
fn capabilities() -> &'static LanguageCapabilities {
    kit_capabilities(&CAPABILITIES, &SPEC)
}

impl LanguageAdapter for CLanguage {
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

/// One `XIRANG_SOIL` declaration, as the text tells it.
/// 一条 `XIRANG_SOIL` 声明，按文本读到的样子。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CDeclaration {
    /// The language-neutral payload: parent, registration fields, and unmapped members.
    /// 语言中立载荷：父级、注册字段与未映射的成员。
    pub face: DeclaredFace,
    /// Byte range of the initializer's body (the text between the braces).
    /// 初始化器体的字节区间（花括号之间的文本）。
    pub block: std::ops::Range<usize>,
}

impl CDeclaration {
    /// The face kind (the `.kind` value).
    /// 面种类名（`.kind` 的取值）。
    pub fn kind(&self) -> String {
        self.face.kind()
    }
}

/// Reads one C declaration from source text.
/// 从源码文本读出一条 C 声明。
pub fn read_declaration(source: &str) -> Result<CDeclaration, String> {
    let start = source.find(MARKER).ok_or_else(|| {
        "this file has no `XIRANG_SOIL` declaration; the C spelling is \
         `XIRANG_SOIL gauge = { XIRANG_RECORD_HEADER, .parent = XIRANG_ROOT_NODE_ID, .kind = \"…\" };`"
            .to_owned()
    })?;
    let open = source[start..]
        .find('{')
        .map(|offset| start + offset)
        .ok_or_else(|| "a `XIRANG_SOIL` declaration has no initializer".to_owned())?;
    let close = matching_brace(source, open)?;
    let mut registration = RegistrationUpdate::new();
    let mut extras = Vec::new();
    let mut parent = None;
    let mut seen: Vec<String> = Vec::new();
    for (key, value) in entries(&source[open + 1..close])? {
        if key.is_empty() {
            // `XIRANG_RECORD_HEADER` (and any other bare macro) is the fixed header, not a member.
            // `XIRANG_RECORD_HEADER`（以及任何其它裸宏）是固定头，不是成员。
            continue;
        }
        if seen.iter().any(|name| name == &key) {
            return Err(format!("the declaration names `{key}` twice"));
        }
        seen.push(key.clone());
        match GRAMMAR.lookup(&key).map(|spec| spec.role) {
            Some(Role::Parent) => {
                if value.is_empty() {
                    return Err("`.parent` names no parent".to_owned());
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
        "the declaration never names `.parent`; the parent is part of the face's position, and a \
         record without one carries sixteen zero bytes, which name no face"
            .to_owned()
    })?;
    if registration
        .find_field(RegistrationField::Kind)
        .value()
        .is_none()
    {
        return Err("a C declaration must name `.kind`".to_owned());
    }
    Ok(CDeclaration {
        face: DeclaredFace {
            parent,
            registration,
            extras,
        },
        block: open + 1..close,
    })
}

/// The field's canonical text from one C value.
/// 一个 C 取值对应的字段规范文本。
fn normalize(field: RegistrationField, raw: &str) -> Result<String, String> {
    match field {
        RegistrationField::NeedsRegistry => match raw.trim() {
            "XIRANG_OWNS_REGISTRY" => Ok("true".to_owned()),
            "0" => Ok("false".to_owned()),
            other => match other.parse::<u32>() {
                Ok(flags) if flags & 1 == 0 => Ok("false".to_owned()),
                Ok(_) => Ok("true".to_owned()),
                Err(_) => Err(format!(
                    "`.flags = {other}` is not `XIRANG_OWNS_REGISTRY` or a number"
                )),
            },
        },
        RegistrationField::Requires => {
            let mut pairs = Vec::new();
            for entry in string(raw)?.split(',') {
                let entry = entry.trim();
                if entry.is_empty() {
                    continue;
                }
                let Some((capability, provider)) = entry.split_once("=>") else {
                    return Err(format!(
                        "`.requires` entry `{entry}` does not name a capability and a provider; \
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
            let mut entries: Vec<String> = string(raw)?
                .split(',')
                .map(str::trim)
                .filter(|entry| !entry.is_empty())
                .map(str::to_owned)
                .collect();
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
    let variable = crate::languages::naming::c_segment(&module);
    let parent = parent_reference(request)?;
    let mut registration = request.registration.clone();
    registration.set_normalized(RegistrationField::Kind, FieldUpdate::Set(kind.clone()));
    let text = render_file(&variable, &kind, &parent, &registration, &[]);
    let diagnostics = vec![
        PlanDiagnostic {
            severity: DiagnosticSeverity::Note,
            message: format!(
                "the declaration is compiled into the `xirang.faces` section; the SDK is `{}` \
                 (include `xirang/soil.h`)",
                crate::sdk::c_include_dir().display()
            ),
        },
        PlanDiagnostic {
            severity: DiagnosticSeverity::Note,
            message: "the build writes `xirang/faces.h` with `XIRANG_ROOT_NODE_ID` and one \
                      `XIRANG_FACES_<flat>` per face module; a non-root parent needs that header"
                .to_owned(),
        },
    ];
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
                "changing `kind` from `{current_kind}` to `{kind}` renames the face's identity; \
                 that is a rename, not a field edit"
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

/// The parent expression a C declaration can name.
/// C 声明可以点名的父表达式。
fn parent_reference(request: &FaceAuthoringRequest) -> Result<String, String> {
    match &request.parent {
        ParentReference::Root => Ok(names::ROOT_CONSTANT.to_owned()),
        ParentReference::Object { module, .. } if !module.trim().is_empty() => {
            Ok(names::face_constant(module))
        }
        ParentReference::Object { .. } => Err(
            "the request names an empty parent module; a C parent is named by the generated \
             `XIRANG_FACES_<flat>` constant of a module, which needs a module path"
                .to_owned(),
        ),
        ParentReference::Named(text) if !text.trim().is_empty() => {
            let trimmed = text.trim();
            if trimmed == names::ROOT_CONSTANT
                || trimmed
                    .strip_prefix("XIRANG_FACES_")
                    .is_some_and(|suffix| !suffix.is_empty())
            {
                return Ok(trimmed.to_owned());
            }
            Err(format!(
                "`{trimmed}` is not a generated C parent constant; use `XIRANG_ROOT_NODE_ID` or \
                 `XIRANG_FACES_<flat>`"
            ))
        }
        ParentReference::Named(_) => {
            Err("the parent reference is empty; a C face must name a constant".to_owned())
        }
        ParentReference::Catalog { path, .. } => Err(format!(
            "the catalog path `{path}` is not a C constant; the build generates per-module \
             `XIRANG_FACES_<flat>` constants, not catalog paths"
        )),
        ParentReference::Identity(_) => Err(
            "the request carries a parent identity, but a C declaration names a compile-time \
             constant"
                .to_owned(),
        ),
    }
}

/// The whole file one create writes.
/// 新建写出的整份文件。
fn render_file(
    variable: &str,
    kind: &str,
    parent: &str,
    registration: &RegistrationUpdate,
    extras: &[(String, String)],
) -> String {
    let block = render_block(parent, registration, extras);
    format!(
        "#include \"xirang/faces.h\"\n#include \"xirang/soil.h\"\n\n// {kind} is the business type this registration face is named after.\nXIRANG_SOIL {variable} = {{{block}}};\n"
    )
}

/// The initializer's body, one member per line, indented four spaces.
/// 初始化器体，每行一个成员，缩进四个空格。
fn render_block(
    parent: &str,
    registration: &RegistrationUpdate,
    extras: &[(String, String)],
) -> String {
    let spelling = |role| {
        GRAMMAR
            .spec(role)
            .map(|spec| spec.spelling)
            .expect("the C grammar carries every role the renderer writes")
    };
    let mut out = String::from("\n    XIRANG_RECORD_HEADER,\n");
    out.push_str(&format!(
        "    {} = {},\n",
        spelling(Role::Parent),
        GRAMMAR.declared(Role::Parent, parent)
    ));
    out.push_str(&format!(
        "    {} = {},\n",
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
                "    {} = {},\n",
                spec.spelling,
                GRAMMAR.declared(spec.role, value)
            ));
        }
    }
    for (key, value) in extras {
        out.push_str(&format!("    {key} = {value},\n"));
    }
    let needs_registry = Role::Field(RegistrationField::NeedsRegistry);
    let owns = matches!(
        registration.find_field(RegistrationField::NeedsRegistry),
        FieldUpdate::Set("true")
    );
    if owns {
        out.push_str(&format!(
            "    {} = {},\n",
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
        "compile the source so the record lands in the `xirang.faces` section and the build registers it",
    )
}

#[cfg(test)]
#[path = "adapter_tests.rs"]
mod adapter_tests;
