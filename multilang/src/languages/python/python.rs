//! The Python language implementation: generate, read, and precisely rewrite one `@soil.face`
//! decorator.
//! Python 语言实现：生成、读取并精确改写一条 `@soil.face` 装饰器。
//!
//! Uses the language kit: the capability table, the declaration walk, the field table and the
//! orchestration come from `crate::languages::kit`; this module owns Python's spelling and its two
//! carriers (the decorator and the build-time walker).
//! 使用语言 kit：能力表、声明遍历、字段表与编排来自 `crate::languages::kit`；本模块只拥有 Python 的拼法
//! 与它的两种载体（装饰器与构建期遍历器）。

#[cfg(feature = "lang-python")]
pub mod semantics;

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
    FaceGrammar, FieldSpec, Role, ValueStyle, obligations as stated_obligations, text_field,
};
use crate::registration::LanguageAdapter;

#[path = "python_faces.rs"]
pub mod faces;
use self::faces::{matching_paren, sequence, string, value_end};

/// The Python language implementation.
/// Python 语言实现。
#[derive(Clone, Copy, Debug, Default)]
pub struct PythonLanguage;

/// The registration fields a Python decorator carries.
/// Python 装饰器携带的注册字段。
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

/// The fields a Python decorator carries, in the order the renderer writes them.
/// Python 装饰器携带的字段，按渲染器写出的顺序。
///
/// The renderer and the reader both go through this table, so a spelling cannot be renamed on one
/// side only: the same entry decides how a keyword is written and which field it names.
/// 渲染器与读取器都走这张表，因此一个拼法不可能只在一侧改名：同一条目既决定一个关键字怎么写，也决定它
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
            style: ValueStyle::Tuple,
        },
        FieldSpec {
            role: Role::Field(RegistrationField::Provides),
            spelling: "provides",
            style: ValueStyle::Tuple,
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
            role: Role::Field(RegistrationField::Requires),
            spelling: "requires",
            style: ValueStyle::RequiresPairs,
        },
        FieldSpec {
            role: Role::Field(RegistrationField::NeedsRegistry),
            spelling: "needs_registry",
            style: ValueStyle::Bool {
                when_true: "True",
                when_false: "False",
            },
        },
    ],
};

/// What the Python adapter tells the shared orchestration about itself.
/// Python 适配器告诉共享编排的东西。
const SPEC: crate::languages::kit::adapter::AdapterSpec =
    crate::languages::kit::adapter::AdapterSpec {
        language: "python",
        extensions: &["py"],
        fields: FIELDS,
        operations: &[AuthoringOperation::Create, AuthoringOperation::Modify],
        carrying: "the decorator carries text and scalar fields",
    };

static CAPABILITIES: OnceLock<LanguageCapabilities> = OnceLock::new();

/// The capability table for Python.
/// Python 的能力表。
fn capabilities() -> &'static LanguageCapabilities {
    crate::languages::kit::adapter::capabilities(&CAPABILITIES, &SPEC)
}

impl LanguageAdapter for PythonLanguage {
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
fn declared(source: &str) -> Result<crate::languages::kit::grammar::DeclaredFace, String> {
    read_declaration(source).map(|declaration| declaration.face)
}

/// One `@soil.face(...)` declaration, as the text tells it.
/// 一条 `@soil.face(...)` 声明，按文本读到的样子。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PythonDeclaration {
    /// The language-neutral payload: parent, registration fields, and unmapped keywords.
    /// 语言中立载荷：父级、注册字段与未映射的关键字参数。
    pub face: crate::languages::kit::grammar::DeclaredFace,
    /// The class the decorator is attached to.
    /// 装饰器所附着的类。
    pub class: String,
    /// Byte range of the argument list (the text between the decorator's parentheses).
    /// 参数表的字节区间（装饰器括号之间的文本）。
    pub arguments: std::ops::Range<usize>,
}

impl PythonDeclaration {
    /// The face kind (the `named` value).
    /// 面种类名（`named` 的取值）。
    pub fn kind(&self) -> String {
        self.face.kind()
    }
}

/// The decorator spellings this adapter reads.
/// 本适配器读取的装饰器拼法。
const DECORATORS: &[&str] = &["@soil.face(", "@xirang.soil(", "@soil("];

/// Read one declaration out of a Python source file.
/// 从 Python 源码里读出一条声明。
/// Compatibility alias for the published `languages::python::read` spelling.
/// 已发布拼法 `languages::python::read` 的兼容别名。
#[deprecated(
    note = "use languages::python::read_declaration; this alias keeps 0.2 callers compiling"
)]
pub fn read(source: &str) -> Result<PythonDeclaration, String> {
    read_declaration(source)
}

/// Reads one Python declaration from source text.
/// 从源码文本读出一条 Python 声明。
pub fn read_declaration(source: &str) -> Result<PythonDeclaration, String> {
    let (start, marker) = DECORATORS
        .iter()
        .filter_map(|marker| source.find(marker).map(|at| (at, *marker)))
        .min_by_key(|(at, _)| *at)
        .ok_or_else(|| {
            "this file has no `@soil.face(...)` declaration; the Python spelling is \
             `@soil.face(under=…, named=\"…\")` above the class"
                .to_owned()
        })?;
    let open = start + marker.len();
    let close = matching_paren(source, open)
        .ok_or_else(|| "the `@soil.face(` argument list is not closed".to_owned())?;
    let arguments = &source[open..close];
    let parsed = parse_arguments(arguments)?;
    // The class name: the first `class <Name>` after the decorator. It is the business type and the
    // face's identity name, so a declaration without one cannot be registered.
    let after = &source[close..];
    let class = after
        .find("class ")
        .and_then(|at| {
            after[at + "class ".len()..]
                .split(|character: char| !(character.is_ascii_alphanumeric() || character == '_'))
                .next()
        })
        .filter(|name| !name.is_empty())
        .ok_or_else(|| {
            "the `@soil.face(...)` decorator is not followed by a `class <Name>`; the class is the \
             face's business type and its identity name"
                .to_owned()
        })?;
    Ok(PythonDeclaration {
        face: crate::languages::kit::grammar::DeclaredFace {
            parent: parsed.parent.ok_or_else(|| {
                "the decorator never names `under=`; the parent is part of the face's position, so a \
                 declaration without one cannot be registered"
                    .to_owned()
            })?,
            registration: parsed.registration,
            extras: parsed.extras,
        },
        class: class.to_owned(),
        arguments: open..close,
    })
}

/// The kwargs one decorator carries.
/// 一个装饰器携带的关键字参数。
struct Arguments {
    registration: RegistrationUpdate,
    extras: Vec<(String, String)>,
    parent: Option<String>,
}

/// Parse `key=value, …` into the registration fields and the extras.
/// 把 `key=value, …` 解析成注册字段与额外项。
fn parse_arguments(text: &str) -> Result<Arguments, String> {
    let mut registration = RegistrationUpdate::new();
    let mut extras = Vec::new();
    let mut parent = None;
    let mut seen: Vec<String> = Vec::new();
    let mut rest = text;
    loop {
        rest = rest
            .trim_start_matches(|character: char| character.is_whitespace() || character == ',');
        if rest.is_empty() {
            break;
        }
        let equals = rest
            .find('=')
            .ok_or_else(|| format!("`{}` is not a `key=value` keyword argument", rest.trim()))?;
        let key = rest[..equals].trim();
        if key.is_empty() {
            return Err("a keyword argument has an empty name".to_owned());
        }
        if seen.iter().any(|name| name == key) {
            return Err(format!("the decorator names `{key}` twice"));
        }
        seen.push(key.to_owned());
        let value_start = equals + 1;
        let value_end = value_end(rest, value_start);
        let raw = rest[value_start..value_end].trim();
        rest = &rest[value_end..];
        if key == "under" {
            if raw.is_empty() {
                return Err("`under=` names no parent".to_owned());
            }
            parent = Some(raw.to_owned());
            continue;
        }
        let field = field_of(key);
        match field {
            Some(field) => {
                let normalized = normalize(field, raw)?;
                registration.set_normalized(field, FieldUpdate::Set(normalized));
            }
            None => extras.push((key.to_owned(), raw.to_owned())),
        }
    }
    Ok(Arguments {
        registration,
        extras,
        parent,
    })
}

/// The registration field one keyword names.
/// 一个关键字命名的注册字段。
fn field_of(key: &str) -> Option<RegistrationField> {
    match GRAMMAR.lookup(key).map(|spec| spec.role) {
        // The display name is the kind the registration keys on; the grammar names the role.
        // 显示名就是注册机据以索引的 kind；语法表点名的是角色。
        Some(Role::Kind) => Some(RegistrationField::Kind),
        Some(Role::Field(field)) => Some(field),
        Some(Role::Parent) | None => None,
    }
}

/// The field's canonical text from one Python value.
/// 一个 Python 取值对应的字段规范文本。
fn normalize(field: RegistrationField, raw: &str) -> Result<String, String> {
    match field {
        RegistrationField::NeedsRegistry => match raw {
            "True" | "true" => Ok("true".to_owned()),
            "False" | "false" => Ok("false".to_owned()),
            other => Err(format!(
                "`needs_registry={other}` is not a boolean; write `True` or `False`"
            )),
        },
        RegistrationField::Requires => {
            let entries = sequence(raw)?;
            let mut pairs = Vec::new();
            for entry in entries {
                let pair = sequence(&entry)?;
                if pair.len() != 2 {
                    return Err(format!(
                        "`requires` entry `{entry}` does not name a capability and a provider"
                    ));
                }
                pairs.push(format!("{}=>{}", string(&pair[0])?, string(&pair[1])?));
            }
            pairs.sort();
            pairs.dedup();
            Ok(pairs.join(","))
        }
        field if field.is_list() => {
            let mut entries = sequence(raw)?
                .iter()
                .map(|entry| string(entry))
                .collect::<Result<Vec<_>, _>>()?;
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
    let parent = parent_reference(request)?;
    let mut registration = request.registration.clone();
    registration.set_normalized(RegistrationField::Kind, FieldUpdate::Set(kind.clone()));
    let text = render_face(&kind, &parent, &registration, &[]);
    let diagnostics = vec![PlanDiagnostic {
        severity: DiagnosticSeverity::Note,
        message: format!(
            "the SDK package is `{}`; add it to the interpreter's path so `from xirang import soil` \
             resolves",
            crate::sdk::python_include_dir().display()
        ),
    }];
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
impl Editable for PythonDeclaration {
    fn face(&self) -> &crate::languages::kit::grammar::DeclaredFace {
        &self.face
    }

    fn arguments(&self) -> std::ops::Range<usize> {
        self.arguments.clone()
    }

    fn rendered(&self, parent: &str, registration: &RegistrationUpdate) -> String {
        format!(
            "\n{}",
            render_arguments(parent, registration, &self.face.extras)
        )
    }
}

const EDIT: EditHooks<PythonDeclaration> = EditHooks {
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
/// The parent expression a Python declaration can name.
/// Python 声明可以点名的父表达式。
fn parent_reference(request: &FaceAuthoringRequest) -> Result<String, String> {
    match &request.parent {
        // The adapter renders the Python object path from the build's facts.
        // 适配器从构建事实渲染 Python 对象路径。
        ParentReference::Object { module, .. } if !module.trim().is_empty() => Ok(format!(
            "faces.{}.NODE_ID",
            module
                .split("::")
                .map(crate::languages::naming::python_segment)
                .collect::<Vec<_>>()
                .join(".")
        )),
        ParentReference::Object { .. } => Err(
            "the request names an empty parent module; a Python declaration names a generated \
             object path, which needs a module path"
                .to_owned(),
        ),
        ParentReference::Named(text) if !text.trim().is_empty() => {
            let trimmed = text.trim();
            let valid = trimmed.strip_prefix("faces.").is_some_and(|rest| {
                rest.ends_with(".NODE_ID")
                    && !rest.is_empty()
                    && rest.chars().all(|character| {
                        character.is_ascii_alphanumeric() || character == '.' || character == '_'
                    })
            });
            if !valid {
                return Err(format!(
                    "`{trimmed}` is not a generated Python face reference; a declaration names \
                     `faces.<path>.NODE_ID` from the generated object directory, not an expression"
                ));
            }
            Ok(trimmed.to_owned())
        }
        ParentReference::Named(_) => Err(
            "the parent reference is empty; a Python face must name a generated constant"
                .to_owned(),
        ),
        ParentReference::Root => Err(
            "a Python face cannot hang under the registry root: the generated object directory \
             carries face identities, not the root's"
                .to_owned(),
        ),
        ParentReference::Catalog { .. } => Err(
            "a Python declaration names `faces.<path>.NODE_ID`; the shared `crate::catalog` \
             spelling is currently emitted only for C++"
                .to_owned(),
        ),
        ParentReference::Identity(_) => Err(
            "the request carries a parent identity, but a Python declaration names a generated \
             constant from `xirang.faces`"
                .to_owned(),
        ),
    }
}

#[cfg(test)]
#[path = "python_tests.rs"]
mod python_tests;

// The rendering half, kept in the same module: a scaffolded language owns one spellings table and
// one renderer, not a second file for each.
// 渲染那一半留在同一个模块里：脚手架语言拥有一张拼写表与一个渲染器，而不是每样再开一个文件。

/// 为一个新建面渲染整份 Python 文件。
fn render_face(
    kind: &str,
    parent: &str,
    registration: &RegistrationUpdate,
    extras: &[(String, String)],
) -> String {
    format!(
        "from xirang import faces, soil\n\n\n@soil.face(\n{})\nclass {kind}:\n    \"\"\"A registration face declared in Python.\"\"\"\n\n    pass\n",
        render_arguments(parent, registration, extras)
    )
}

/// Render the decorator's keyword arguments, indented four spaces.
/// 渲染装饰器的关键字参数，缩进四个空格。
///
/// The spellings and value styles come from the language's grammar table, the same one the reader
/// looks keywords up in: a field renamed on one side cannot be forgotten on the other.
/// 拼法与取值风格来自语言的语法表——读取器查关键字用的也是同一张表：一侧改名的字段不会在另一侧被忘掉。
fn render_arguments(
    parent: &str,
    registration: &RegistrationUpdate,
    extras: &[(String, String)],
) -> String {
    let spelling = |role| {
        GRAMMAR
            .spec(role)
            .map(|spec| spec.spelling)
            .expect("the Python grammar carries every role the renderer writes")
    };
    let mut text = format!(
        "    {}={},\n",
        spelling(Role::Parent),
        GRAMMAR.declared(Role::Parent, parent)
    );
    text.push_str(&format!(
        "    {}={},\n",
        spelling(Role::Kind),
        GRAMMAR.declared(
            Role::Kind,
            text_field(registration, RegistrationField::Kind)
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
        if let FieldUpdate::Set(value) = registration.find_field(match spec.role {
            Role::Field(field) => field,
            Role::Parent | Role::Kind => continue,
        }) {
            text.push_str(&format!(
                "    {}={},\n",
                spec.spelling,
                GRAMMAR.declared(spec.role, value)
            ));
        }
    }
    let requires = Role::Field(RegistrationField::Requires);
    if let FieldUpdate::Set(value) = registration.find_field(RegistrationField::Requires)
        && !value.is_empty()
    {
        text.push_str(&format!(
            "    {}={},\n",
            spelling(requires),
            GRAMMAR.declared(requires, value)
        ));
    }
    for (key, value) in extras {
        text.push_str(&format!("    {key}={value},\n"));
    }
    let needs_registry = Role::Field(RegistrationField::NeedsRegistry);
    let owns = matches!(
        registration.find_field(RegistrationField::NeedsRegistry),
        FieldUpdate::Set("true")
    );
    text.push_str(&format!(
        "    {}={},\n",
        spelling(needs_registry),
        GRAMMAR.declared(needs_registry, if owns { "true" } else { "false" })
    ));
    text
}

fn obligations(kind: &str, registration: &RegistrationUpdate) -> Vec<ValidationObligation> {
    stated_obligations(
        kind,
        registration,
        "import the package so the decorator runs and its declaration is read",
    )
}
