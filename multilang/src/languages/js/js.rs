//! The JavaScript language implementation: generate, read, and precisely rewrite one
//! `soil.face({…})` call.
//! JavaScript 语言实现：生成、读取并精确改写一条 `soil.face({…})` 调用。
//!
//! Uses the language kit: the capability table, the declaration walk, the field table and the
//! orchestration come from `crate::languages::kit`; this module owns JavaScript's spelling and its
//! two carriers (the import-time call and the build-time walker).
//! 使用语言 kit：能力表、声明遍历、字段表与编排来自 `crate::languages::kit`；本模块只拥有 JavaScript 的
//! 拼法与它的两种载体（导入期调用与构建期遍历器）。
//!
//! The carrier is a call, not a decorator: a face is one `soil.face({…})` whose argument object names
//! the parent and the registration fields. Importing the module runs the call; the build reads the
//! same call statically and never executes user business code.
//! 载体是一次调用而不是装饰器：一个面就是一条 `soil.face({…})`，其参数对象点名父级与注册字段。导入模块
//! 会执行这次调用；构建静态读取同一次调用，从不执行用户业务代码。

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
    DeclaredFace, FaceGrammar, FieldSpec, Role, ValueStyle, obligations as stated_obligations,
};
use crate::registration::LanguageAdapter;

#[path = "js_faces.rs"]
pub mod faces;
use self::faces::{array, matching_brace, pairs, string};

/// The call spellings this adapter reads.
/// 本适配器读取的调用拼法。
pub const MARKERS: &[&str] = &["xirang.soil.face(", "soil.face("];

/// The JavaScript language implementation.
/// JavaScript 语言实现。
#[derive(Clone, Copy, Debug, Default)]
pub struct JsLanguage;

/// The registration fields a `soil.face({…})` call carries.
/// `soil.face({…})` 调用携带的注册字段。
pub(crate) const FIELDS: &[RegistrationField] = &[
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

/// The fields a `soil.face({…})` declaration carries, in the order the renderer writes them.
/// `soil.face({…})` 声明携带的字段，按渲染器写出的顺序。
///
/// The renderer and the reader both go through this table, so a spelling cannot be renamed on one
/// side only: the same entry decides how a key is written and which field it names.
/// 渲染器与读取器都走这张表，因此一个拼法不可能只在一侧改名：同一条目既决定一个键怎么写，也决定它命名
/// 哪个字段。
pub const GRAMMAR: FaceGrammar = FaceGrammar {
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
            style: ValueStyle::BracketList,
        },
        FieldSpec {
            role: Role::Field(RegistrationField::Provides),
            spelling: "provides",
            style: ValueStyle::BracketList,
        },
        FieldSpec {
            role: Role::Field(RegistrationField::Requires),
            spelling: "requires",
            style: ValueStyle::BracketList,
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
            spelling: "from_registry",
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

/// What the JavaScript adapter tells the shared orchestration about itself.
/// JavaScript 适配器告诉共享编排的东西。
const SPEC: AdapterSpec = AdapterSpec {
    language: "javascript",
    extensions: &["js"],
    fields: FIELDS,
    operations: &[AuthoringOperation::Create, AuthoringOperation::Modify],
    carrying: "the call carries text, list and boolean fields",
};

static CAPABILITIES: OnceLock<LanguageCapabilities> = OnceLock::new();

/// The capability table for JavaScript.
/// JavaScript 的能力表。
fn capabilities() -> &'static LanguageCapabilities {
    kit_capabilities(&CAPABILITIES, &SPEC)
}

impl LanguageAdapter for JsLanguage {
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

/// One `soil.face({…})` declaration, as the text tells it.
/// 一条 `soil.face({…})` 声明，按文本读到的样子。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JsDeclaration {
    /// The language-neutral payload: parent, registration fields, and unmapped keys.
    /// 语言中立载荷：父级、注册字段与未映射的键。
    pub face: DeclaredFace,
    /// Byte range of the argument object's body (the text between the braces).
    /// 参数对象体的字节区间（花括号之间的文本）。
    pub object: std::ops::Range<usize>,
}

impl JsDeclaration {
    /// The face kind (the `named` value).
    /// 面种类名（`named` 的取值）。
    pub fn kind(&self) -> String {
        self.face.kind()
    }
}

/// Reads one JavaScript declaration from source text.
/// 从源码文本读出一条 JavaScript 声明。
pub fn read_declaration(source: &str) -> Result<JsDeclaration, String> {
    let (start, marker) = MARKERS
        .iter()
        .filter_map(|marker| source.find(marker).map(|at| (at, *marker)))
        .min_by_key(|(at, _)| *at)
        .ok_or_else(|| {
            "this file has no `soil.face({…})` declaration; the JavaScript spelling is \
             `soil.face({ under: faces.<module>.NODE_ID, named: \"…\" })`"
                .to_owned()
        })?;
    let after = start + marker.len();
    let open = source[after..]
        .find('{')
        .map(|offset| after + offset)
        .ok_or_else(|| "the `soil.face(` call has no argument object".to_owned())?;
    // A call whose first argument is not the object literal would otherwise read a brace from
    // somewhere else in the file; the text between `(` and `{` must be empty for the same reason.
    // 首个实参不是对象字面量的调用会从文件别处读到一个花括号；`(` 与 `{` 之间必须为空的理由相同。
    if !source[after..open].trim().is_empty() {
        return Err(
            "the `soil.face(` call's first argument is not an object literal; the declaration is \
             one `{ … }` of fields, not an expression"
                .to_owned(),
        );
    }
    let close = matching_brace(source, open)?;
    let mut registration = RegistrationUpdate::new();
    let mut extras = Vec::new();
    let mut parent = None;
    let mut seen: Vec<String> = Vec::new();
    for (key, value) in pairs(&source[open + 1..close])? {
        if seen.iter().any(|name| name == &key) {
            return Err(format!("the declaration names `{key}` twice"));
        }
        seen.push(key.clone());
        match GRAMMAR.lookup(&key).map(|spec| spec.role) {
            Some(Role::Parent) => {
                if value.is_empty() {
                    return Err("`under:` names no parent".to_owned());
                }
                faces::parent_module(&value)?;
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
        "the declaration never names `under:`; the parent is part of the face's position, so a \
         declaration without one cannot be registered"
            .to_owned()
    })?;
    if registration
        .find_field(RegistrationField::Kind)
        .value()
        .is_none()
    {
        return Err("a JavaScript declaration must name `named:`".to_owned());
    }
    Ok(JsDeclaration {
        face: DeclaredFace {
            parent,
            registration,
            extras,
        },
        object: open + 1..close,
    })
}

/// The field's canonical text from one JavaScript value.
/// 一个 JavaScript 取值对应的字段规范文本。
fn normalize(field: RegistrationField, raw: &str) -> Result<String, String> {
    match field {
        RegistrationField::NeedsRegistry => match raw.trim() {
            "true" => Ok("true".to_owned()),
            "false" => Ok("false".to_owned()),
            other => Err(format!(
                "`needs_registry: {other}` is not a boolean; write `true` or `false`"
            )),
        },
        RegistrationField::Requires => {
            let entries = array(raw)?;
            let mut pairs = Vec::new();
            for entry in entries {
                let Some((capability, provider)) = entry.split_once("=>") else {
                    return Err(format!(
                        "`requires` entry `{entry}` does not name a capability and a provider; write \
                         `\"capability=>provider\"`"
                    ));
                };
                if capability.trim().is_empty() || provider.trim().is_empty() {
                    return Err(format!(
                        "`requires` entry `{entry}` has an empty capability or provider"
                    ));
                }
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
    create_for(
        &capabilities().language,
        &crate::sdk::js_include_dir(),
        request,
    )
}

/// Create one declaration for a language that shares this carrier, naming the SDK it imports.
/// 为共用本载体的语言新建一条声明，并点名它导入的 SDK。
pub(crate) fn create_for(
    language: &xirang_kernel::interchange::LanguageId,
    sdk: &std::path::Path,
    request: &FaceAuthoringRequest,
) -> Result<FaceAuthoringPlan, String> {
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
    let parent = parent_reference(request)?;
    let mut registration = request.registration.clone();
    registration.set_normalized(RegistrationField::Kind, FieldUpdate::Set(kind.clone()));
    let text = render_file(&kind, &parent, &registration, &[]);
    let diagnostics = vec![
        PlanDiagnostic {
            severity: DiagnosticSeverity::Note,
            message: format!(
                "the face is declared at import time; the SDK package is `{}`, imported as `xirang`",
                sdk.join("xirang").display()
            ),
        },
        PlanDiagnostic {
            severity: DiagnosticSeverity::Note,
            message: format!(
                "`{kind}` is the business type: fill in its implementation around the declaration"
            ),
        },
    ];
    Ok(FaceAuthoringPlan {
        language: language.clone(),
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
    modify_for(&capabilities().language, request)
}
impl Editable for JsDeclaration {
    fn face(&self) -> &DeclaredFace {
        &self.face
    }

    fn arguments(&self) -> std::ops::Range<usize> {
        self.object.clone()
    }

    fn rendered(&self, parent: &str, registration: &RegistrationUpdate) -> String {
        render_object(parent, registration, &self.face.extras)
    }
}

const EDIT: EditHooks<JsDeclaration> = EditHooks {
    read: read_declaration,
    parent: parent_reference,
    obligations,
    carrier: Some("class"),
};

/// Modify one existing declaration for `language`, preserving everything the request does not name.
/// 按 `language` 修改一条已有声明，保留请求没有点名的一切。
pub(crate) fn modify_for(
    language: &xirang_kernel::interchange::LanguageId,
    request: &FaceAuthoringRequest,
) -> Result<FaceAuthoringPlan, String> {
    kit_modify(language, request, &EDIT)
}
/// The parent expression a JavaScript declaration can name.
/// JavaScript 声明可以点名的父表达式。
fn parent_reference(request: &FaceAuthoringRequest) -> Result<String, String> {
    match &request.parent {
        // The adapter renders the generated object path from the build's facts.
        // 适配器从构建事实渲染生成的对象路径。
        ParentReference::Object { module, .. } if !module.trim().is_empty() => Ok(format!(
            "faces.{}.NODE_ID",
            module
                .split("::")
                .map(crate::languages::naming::javascript_segment)
                .collect::<Vec<_>>()
                .join(".")
        )),
        ParentReference::Object { .. } => Err(
            "the request names an empty parent module; a JavaScript declaration names a generated \
             object path, which needs a module path"
                .to_owned(),
        ),
        ParentReference::Named(text) if !text.trim().is_empty() => {
            let trimmed = text.trim();
            let valid = trimmed
                .strip_suffix(".NODE_ID")
                .and_then(|path| {
                    path.strip_prefix("faces.")
                        .or_else(|| path.strip_prefix("xirang.faces."))
                })
                .is_some_and(|path| {
                    !path.is_empty()
                        && path.split('.').all(|segment| {
                            !segment.is_empty()
                                && segment.chars().all(|character| {
                                    character == '_'
                                        || character == '$'
                                        || character.is_ascii_alphanumeric()
                                })
                                && segment
                                    .chars()
                                    .next()
                                    .is_some_and(|first| !first.is_ascii_digit())
                        })
                });
            if !valid {
                return Err(format!(
                    "`{trimmed}` is not a generated JavaScript face reference; a declaration names \
                     `faces.<path>.NODE_ID` from the generated object directory, not an expression"
                ));
            }
            Ok(trimmed.to_owned())
        }
        ParentReference::Named(_) => Err(
            "the parent reference is empty; a JavaScript face must name a generated constant"
                .to_owned(),
        ),
        ParentReference::Root => Ok("faces.root.NODE_ID".to_owned()),
        ParentReference::Catalog { path, .. } => Err(format!(
            "the catalog path `{path}` is not a JavaScript module constant; name the parent by its \
             module instead"
        )),
        ParentReference::Identity(_) => Err(
            "the request carries a parent identity, but a JavaScript declaration names a generated \
             constant from the `faces` directory"
                .to_owned(),
        ),
    }
}

/// The whole file one create writes.
/// 新建写出的整份文件。
fn render_file(
    kind: &str,
    parent: &str,
    registration: &RegistrationUpdate,
    extras: &[(String, String)],
) -> String {
    let object = render_object(parent, registration, extras);
    format!(
        "import {{ faces, soil }} from \"xirang\";\n\n\
         /** The business type this registration face is named after. */\n\
         export class {kind} {{\n}}\n\n\
         soil.face({{{object}}});\n"
    )
}

/// The field object, one `key: value,` per line, indented two spaces.
/// 字段对象，每行一个 `key: value,`，缩进两个空格。
fn render_object(
    parent: &str,
    registration: &RegistrationUpdate,
    extras: &[(String, String)],
) -> String {
    let spelling = |role| {
        GRAMMAR
            .spec(role)
            .map(|spec| spec.spelling)
            .expect("the JavaScript grammar carries every role the renderer writes")
    };
    let mut out = String::from("\n");
    out.push_str(&format!(
        "  {}: {},\n",
        spelling(Role::Parent),
        GRAMMAR.declared(Role::Parent, parent)
    ));
    out.push_str(&format!(
        "  {}: {},\n",
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
            Role::Parent
                | Role::Kind
                | Role::Field(RegistrationField::Requires)
                | Role::Field(RegistrationField::NeedsRegistry)
        ) {
            continue;
        }
        let field = match spec.role {
            Role::Field(field) => field,
            Role::Parent | Role::Kind => continue,
        };
        if let FieldUpdate::Set(value) = registration.find_field(field) {
            out.push_str(&format!(
                "  {}: {},\n",
                spec.spelling,
                GRAMMAR.declared(spec.role, value)
            ));
        }
    }
    let requires = Role::Field(RegistrationField::Requires);
    if let FieldUpdate::Set(value) = registration.find_field(RegistrationField::Requires)
        && !value.is_empty()
    {
        out.push_str(&format!(
            "  {}: {},\n",
            spelling(requires),
            GRAMMAR.declared(requires, value)
        ));
    }
    for (key, value) in extras {
        out.push_str(&format!("  {key}: {value},\n"));
    }
    let needs_registry = Role::Field(RegistrationField::NeedsRegistry);
    let owns = matches!(
        registration.find_field(RegistrationField::NeedsRegistry),
        FieldUpdate::Set("true")
    );
    out.push_str(&format!(
        "  {}: {},\n",
        spelling(needs_registry),
        GRAMMAR.declared(needs_registry, if owns { "true" } else { "false" })
    ));
    out
}

fn obligations(kind: &str, registration: &RegistrationUpdate) -> Vec<ValidationObligation> {
    stated_obligations(
        kind,
        registration,
        "import the module so the `soil.face(…)` call runs and its declaration is read",
    )
}

#[cfg(test)]
#[path = "js_tests.rs"]
mod js_tests;
