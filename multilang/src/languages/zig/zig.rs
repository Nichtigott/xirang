//! The Zig language implementation: read, generate, and precisely rewrite one `xirang.record(.{…})`
//! declaration.
//! Zig 语言实现：读取、生成并精确改写一条 `xirang.record(.{…})` 声明。
//!
//! Uses the language kit: the capability table, the field table, the adapter orchestration and the
//! declaration walk come from `crate::languages::kit`; this module owns Zig's spelling, its
//! declaration syntax and the native build hook.
//! 使用语言 kit：能力表、字段表、适配器编排与声明遍历来自 `crate::languages::kit`；本模块只拥有 Zig 的
//! 拼法、它的声明语法与原生构建钩子。
//!
//! The carrier is a root declaration, not a decorator: a face is one `pub export var SOIL` whose
//! initializer builds the section record at comptime. The record layout is the same 1100-byte v2
//! record the C++ SDK writes; the SDK at `sdk/zig/xirang/soil.zig` is the other half of this file's
//! spelling, and the two must move together.
//! 载体是一条根声明而不是装饰器：一个面就是一个 `pub export var SOIL`，其初始化器在 comptime 构造段记录。
//! 记录布局与 C++ SDK 写的是同一种 1100 字节 v2 记录；`sdk/zig/xirang/soil.zig` 是本文件拼法的另一半，
//! 两者必须一起改。

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use xirang_kernel::authoring::{
    AuthoringOperation, DiagnosticSeverity, FaceAuthoringPlan, FaceAuthoringRequest, FieldUpdate,
    LanguageCapabilities, ParentReference, PlanDiagnostic, PlannedFile, RegistrationField,
    RegistrationUpdate, ValidationObligation,
};

use crate::languages::kit::adapter::{AdapterSpec, capabilities as kit_capabilities};
use crate::languages::kit::grammar::{
    DeclaredFace, FaceGrammar, FieldSpec, Role, ValueStyle, obligations as stated_obligations,
};
use crate::languages::kit::native::{NativeSpec, ObjectCommand};
use crate::registration::LanguageAdapter;

/// The Zig language implementation.
/// Zig 语言实现。
#[derive(Clone, Copy, Debug, Default)]
pub struct ZigLanguage;

/// What this language tells the shared adapter about itself.
/// 这门语言告诉共享适配器的东西。
const SPEC: AdapterSpec = AdapterSpec {
    language: "zig",
    extensions: &["zig"],
    fields: &[
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
    ],
    operations: &[AuthoringOperation::Create, AuthoringOperation::Modify],
    carrying: "a `xirang.record(.{…})` declaration",
};

/// The fields a Zig declaration carries, in the order the renderer writes them.
/// Zig 声明携带的字段，按渲染器写出的顺序。
///
/// List fields are one quoted comma-separated string because that is what the SDK's `record`
/// builder takes; capability requirements keep the `capability=>provider` pair spelling.
/// 列表字段是一个带引号的逗号分隔串，因为 SDK 的 `record` 构造器收的就是它；能力需求保留
/// `capability=>provider` 成对拼法。
pub const GRAMMAR: FaceGrammar = FaceGrammar {
    fields: &[
        FieldSpec {
            role: Role::Kind,
            spelling: "kind",
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
            spelling: "requires",
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
            spelling: "from_registry",
            style: ValueStyle::Quoted,
        },
        FieldSpec {
            role: Role::Field(RegistrationField::NeedsRegistry),
            spelling: "owns_registry",
            style: ValueStyle::Bool {
                when_true: "true",
                when_false: "false",
            },
        },
    ],
};

static CAPABILITIES: OnceLock<LanguageCapabilities> = OnceLock::new();

fn capabilities() -> &'static LanguageCapabilities {
    kit_capabilities(&CAPABILITIES, &SPEC)
}

/// The Zig SDK directory every compilation adds, for a caller that wants to say which one.
/// 每次编译都会添加的 Zig SDK 目录，供想说出具体目录的调用方使用。
pub fn include_dirs() -> Vec<PathBuf> {
    vec![crate::sdk::zig_include_dir()]
}

/// One declaration found in a Zig source, before the build assigns it an identity.
/// Zig 源码里发现的一条声明，尚未由构建分配身份。
#[derive(Clone, Debug)]
pub struct NativeFace {
    /// The package-relative spelling of the source.
    /// 源码在包内的相对拼写。
    pub source: String,
    /// The parent as the declaration spells it: a generated module constant, or `ROOT_NODE_ID`.
    /// 父级按声明的拼法：生成的模块常量，或 `ROOT_NODE_ID`。
    pub parent: String,
    /// The face name from `.kind`.
    /// `.kind` 里的面名。
    pub name: String,
    /// Whether the declaration says it owns a registry.
    /// 声明是否说自己自持注册机。
    pub owns_registry: bool,
}

/// Does this source declare a face at all? The build's discovery filter.
/// 这份源码到底有没有声明一个面？构建的发现筛子。
fn mentions(text: &str) -> bool {
    text.contains("xirang.record(")
}

/// Read the package's native recipe and the declarations it compiles.
/// 读出包的原生配方，以及它要编译的声明。
pub fn read_recipe(
    package_root: &Path,
) -> Result<Option<(crate::external_faces::Recipe, Vec<NativeFace>)>, String> {
    let Some((recipe, sources)) =
        crate::languages::kit::native::read_recipe(package_root, &NATIVE, mentions)?
    else {
        return Ok(None);
    };
    let mut faces = Vec::new();
    for source in sources {
        let path = package_root.join(&source);
        let text = std::fs::read_to_string(&path)
            .map_err(|error| format!("read {}: {error}", path.display()))?;
        if let Some(face) = scan(&source, &text)? {
            faces.push(face);
        }
    }
    Ok(Some((recipe, faces)))
}

/// The declaration one source carries, when it carries one.
/// 一份源码携带的那条声明（若携带）。
pub fn scan(source: &str, text: &str) -> Result<Option<NativeFace>, String> {
    if !mentions(text) {
        return Ok(None);
    }
    let declaration = read_declaration(text)?;
    let name = declaration.face.kind();
    let parent = declaration
        .face
        .extras
        .iter()
        .find(|(name, _)| name == "parent")
        .map(|(_, value)| constant_spelling(value))
        .transpose()?
        .unwrap_or_else(|| "ROOT_NODE_ID".to_owned());
    let owns_registry = matches!(
        declaration
            .face
            .registration
            .find_field(RegistrationField::NeedsRegistry),
        FieldUpdate::Set("true")
    );
    Ok(Some(NativeFace {
        source: source.to_owned(),
        parent,
        name,
        owns_registry,
    }))
}

/// The constant a parent expression names: the identifier after the last `.`.
/// 父表达式的常量：最后一个 `.` 之后的标识符。
fn constant_spelling(value: &str) -> Result<String, String> {
    let value = value.trim();
    let ident = value.rsplit('.').next().unwrap_or("");
    if ident.is_empty()
        || !ident
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '_')
    {
        return Err(format!(
            "`{value}` is not a module constant; a Zig declaration names its parent as              `xirang_faces.<module>` (or `xirang_faces.ROOT_NODE_ID`)"
        ));
    }
    Ok(ident.to_owned())
}

/// Compile this package's Zig sources through the shared native driver.
/// 经共享原生驱动编译本包的 Zig 源码。
pub fn compile(
    package_root: &Path,
    out_dir: &Path,
    recipe: &crate::external_faces::Recipe,
    sources: &[&str],
    emit_cargo_directives: bool,
) -> Result<Vec<PathBuf>, String> {
    crate::languages::kit::native::compile(
        package_root,
        out_dir,
        recipe,
        sources,
        &include_dirs(),
        &NATIVE,
        emit_cargo_directives,
    )
}

/// The generated `xirang_faces.zig` module: the root identity and one identity per module.
/// 生成的 `xirang_faces.zig` 模块：根身份，以及每个模块一个身份。
///
/// A declaration names its parent by the **module** constant (`xirang_faces.shop__panel`), the same
/// rule the C++ header gives through `xirang::faces::shop::panel::NODE_ID`; a module owns one face, so
/// the constant needs no kind. The spelling is the flat module path, which already prefixes `_` when a
/// segment starts with a digit.
/// 声明按**模块**常量点名父级（`xirang_faces.shop__panel`），与 C++ 头经
/// `xirang::faces::shop::panel::NODE_ID` 给出的规则相同；一个模块拥有一个面，因此常量不必带种类名。
/// 拼法是扁平的模块路径，段以数字开头时它已经补了 `_`。
pub fn faces_module(root: [u8; 16], entries: &[(String, [u8; 16])]) -> String {
    let mut text = String::from(
        "//! Generated by xirang's build; do not edit.\n\
         //! 由 xirang 构建生成；请勿手工修改。\n\
         \n",
    );
    text.push_str(&format!(
        "/// The registry root, for a declaration that names no parent.\n\
         /// 注册树根，供不点名父级的声明使用。\n\
         pub const ROOT_NODE_ID: [16]u8 = {};\n\n",
        id_literal(root)
    ));
    for (module, id) in entries {
        text.push_str(&format!(
            "/// {module}\n\
             pub const {}: [16]u8 = {};\n\n",
            module_flat(module),
            id_literal(*id)
        ));
    }
    text
}

/// The flat spelling of a module: `::` folded to `__`, and a leading `_` when a segment starts with
/// a digit (Zig identifiers cannot).
/// 模块的扁平拼法：`::` 折成 `__`；段以数字开头时补前导 `_`（Zig 标识符不允许数字开头）。
pub fn module_flat(module: &str) -> String {
    let mut name = super::super::cpp_names::flat(module, "");
    if let Some(stripped) = name.strip_suffix('_') {
        name = stripped.to_owned();
    }
    name
}

/// One identity as a Zig array literal.
/// 一个身份，按 Zig 数组字面量写。
fn id_literal(bytes: [u8; 16]) -> String {
    let items: Vec<String> = bytes.iter().map(|byte| format!("0x{byte:02x}")).collect();
    format!(".{{ {} }}", items.join(", "))
}

/// The Zig compiler invocation: `zig build-obj --dep xirang -Mroot=<src> -Mxirang=<sdk>/xirang/soil.zig`.
/// Zig 编译器的调用形状。
fn object_command(compile: &ObjectCommand<'_>) -> (String, Vec<String>) {
    let sdk = crate::sdk::zig_include_dir().join("xirang/soil.zig");
    // The generated identity module exists only when the build wrote it; a caller that compiles a
    // lone declaration (the record check does) gets no module rather than a missing-file error.
    // 生成的恒等模块只在构建写下它时存在；单独编译一条声明的调用方（记录检查就是）因此不拿这个模块，
    // 而不是收到一个缺文件的错误。
    let generated = compile.out_dir.join("xirang_faces.zig");
    let mut arguments = vec![
        "build-obj".to_owned(),
        "--dep".to_owned(),
        "xirang".to_owned(),
    ];
    if generated.is_file() {
        arguments.push("--dep".to_owned());
        arguments.push("xirang_faces".to_owned());
    }
    arguments.push(format!("-Mroot={}", compile.source.display()));
    arguments.push(format!("-Mxirang={}", sdk.display()));
    if generated.is_file() {
        arguments.push(format!("-Mxirang_faces={}", generated.display()));
    }
    arguments.push(format!("-femit-bin={}", compile.object.display()));
    arguments.push("--cache-dir".to_owned());
    arguments.push(compile.out_dir.join("zig-cache").display().to_string());
    arguments.push("--global-cache-dir".to_owned());
    arguments.push(compile.out_dir.join("zig-global").display().to_string());
    arguments.extend(compile.recipe.arguments.iter().cloned());
    (compile.recipe.program.clone(), arguments)
}

/// What the Zig build path tells the shared native driver.
/// Zig 构建路径告诉共享原生驱动的东西。
pub const NATIVE: NativeSpec = NativeSpec {
    extensions: &["zig"],
    default_recipe,
    include_dirs,
    skip_directories: &["target", ".git", "crates", "zm", ".zig-cache"],
    object_command,
};

fn default_recipe(_sources: &[String]) -> (String, Vec<String>) {
    ("zig".to_owned(), Vec::new())
}

impl LanguageAdapter for ZigLanguage {
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

/// One Zig declaration, as the text tells it.
/// 一条 Zig 声明，按文本读到的样子。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ZigDeclaration {
    /// The language-neutral payload: parent, registration fields, and unmapped fields.
    /// 语言中立载荷：父级、注册字段与未映射的字段。
    pub face: DeclaredFace,
    /// The text between `xirang.record(.{` and its closing `})`.
    /// `xirang.record(.{` 与其收尾 `})` 之间的文本。
    pub body: String,
}

/// Read one `xirang.record(.{…})` declaration out of a Zig source.
/// 从一份 Zig 源码里读出一条 `xirang.record(.{…})` 声明。
pub fn read_declaration(source: &str) -> Result<ZigDeclaration, String> {
    let marker = source
        .find("xirang.record(.{")
        .ok_or_else(|| "no `xirang.record(.{…})` declaration in this source".to_owned())?;
    let open = source[marker..]
        .find(".{")
        .map(|offset| marker + offset)
        .ok_or_else(|| "the record builder has no field block".to_owned())?;
    let close = matching_brace(source, open)?;
    let body = source[open + 2..close].to_owned();
    let mut registration = RegistrationUpdate::new();
    let mut extras = Vec::new();
    for line in body.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with("//") {
            // A comment inside the record builder is prose, not a field; the authoring read ignores
            // it. A modify rewrites the block, so a comment inside it is not preserved — everything
            // outside the block is.
            // 记录构造器里的注释是散文而不是字段，创作读取忽略它。modify 会重写这个块，因此块内注释不
            // 保留——块外的一切都保留。
            continue;
        }
        let Some(rest) = line.strip_prefix('.') else {
            return Err(format!(
                "`{line}` is not a `.field = value,` line inside the record builder"
            ));
        };
        let Some((name, value)) = rest.split_once('=') else {
            return Err(format!("`{line}` names a field without a value"));
        };
        let name = name.trim();
        let value = value.trim().trim_end_matches(',');
        // `field_of` answers for registration fields only; `kind` is a role of its own, so the
        // spelling is resolved through `lookup` and mapped here.
        // `field_of` 只回答注册字段；`kind` 是它自己的角色，因此先 `lookup` 再在这里映射。
        let field = match GRAMMAR.lookup(name) {
            Some(spec) if spec.role == Role::Kind => Some(RegistrationField::Kind),
            Some(spec) => match spec.role {
                Role::Field(field) => Some(field),
                Role::Parent | Role::Kind => None,
            },
            None => None,
        };
        match field {
            Some(field) => {
                let text = literal_text(value)?;
                registration.set_normalized(field, FieldUpdate::Set(text));
            }
            None => extras.push((name.to_owned(), value.to_owned())),
        }
    }
    if registration
        .find_field(RegistrationField::Kind)
        .value()
        .is_none()
    {
        return Err("a Zig declaration must name `.kind`".to_owned());
    }
    Ok(ZigDeclaration {
        face: DeclaredFace {
            parent: String::new(),
            registration,
            extras,
        },
        body,
    })
}

/// One literal as the text carries it: a quoted string, or a boolean word.
/// 一个按文本携带的字面量：带引号的字符串，或布尔词。
fn literal_text(value: &str) -> Result<String, String> {
    if let Some(inner) = value
        .strip_prefix('"')
        .and_then(|rest| rest.strip_suffix('"'))
    {
        return Ok(inner.replace("\\\"", "\"").replace("\\\\", "\\"));
    }
    if value == "true" || value == "false" {
        return Ok(value.to_owned());
    }
    Err(format!(
        "`{value}` is not a string literal or a boolean inside the record builder"
    ))
}

/// The byte offset just past the `}` matching the `{` at `open`.
/// `open` 处 `{` 的配对 `}` 之后的字节偏移。
fn matching_brace(source: &str, open: usize) -> Result<usize, String> {
    let bytes = source.as_bytes();
    let mut depth = 0usize;
    for (at, byte) in bytes.iter().enumerate().skip(open) {
        match byte {
            b'{' => depth += 1,
            b'}' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return Ok(at);
                }
            }
            _ => {}
        }
    }
    Err("the record builder's field block is not closed".to_owned())
}

/// Render one created declaration.
/// 渲染一条新建声明。
fn create(request: &FaceAuthoringRequest) -> Result<FaceAuthoringPlan, String> {
    let module = module_of(&request.destination);
    let kind = match request.registration.find_field(RegistrationField::Kind) {
        FieldUpdate::Set(kind) => kind.to_owned(),
        FieldUpdate::Clear => return Err("`kind` cannot be empty; it names the face".to_owned()),
        FieldUpdate::Keep => xirang_kernel::authoring::pascal_case(&module),
    };
    let mut registration = request.registration.clone();
    if matches!(
        request.registration.find_field(RegistrationField::Kind),
        FieldUpdate::Keep
    ) {
        registration.set_normalized(RegistrationField::Kind, FieldUpdate::Set(kind.clone()));
    }
    let parent = parent_literal(&request.parent)?;
    let text = render_file(&parent, &registration, &[]);
    let diagnostics = vec![
        PlanDiagnostic {
            severity: DiagnosticSeverity::Note,
            message: format!(
                "`{kind}` is declared as a section record; fill in its implementation beside the declaration"
            ),
        },
        PlanDiagnostic {
            severity: DiagnosticSeverity::Note,
            message: format!(
                "the SDK module is `{}`; the build exposes it as `xirang`",
                crate::sdk::zig_include_dir()
                    .join("xirang/soil.zig")
                    .display()
            ),
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
    let current_kind = declaration.face.kind();
    match request.registration.find_field(RegistrationField::Kind) {
        FieldUpdate::Keep => {}
        FieldUpdate::Set(kind) if kind == current_kind => {}
        FieldUpdate::Set(_) => {
            return Err(format!(
                "changing `kind` from `{current_kind}` renames the business type and the face's \
                 identity; that is a rename, not a field edit"
            ));
        }
        FieldUpdate::Clear => return Err("`kind` cannot be empty; it names the face".to_owned()),
    }
    let mut registration = declaration.face.registration.clone();
    for (field, value) in request.registration.named() {
        registration.set_normalized(field, FieldUpdate::Set(value.to_owned()));
    }
    let parent = parent_literal(&request.parent)?;
    let extras: Vec<(String, String)> = declaration
        .face
        .extras
        .iter()
        .filter(|(name, _)| name != "parent")
        .cloned()
        .collect();
    let block = render_block(&parent, &registration, &extras);
    let at = source
        .find("xirang.record(.{")
        .ok_or_else(|| "the declaration disappeared between read and write".to_owned())?;
    let open = source[at..]
        .find(".{")
        .map(|offset| at + offset)
        .expect("block");
    let close = matching_brace(source, open)?;
    // Braces are assembled piece by piece: `format!` reads a literal `.{` followed by a placeholder
    // as one `{}`, which silently drops the block's delimiters (measured by the plan pin going red).
    // 花括号逐段拼接：`format!` 会把字面 `. {` 加占位符读成一个 `{}`，静默吃掉块的定界符（被计划钉子量到）。
    let mut updated = String::new();
    updated.push_str(&source[..at]);
    updated.push_str("xirang.record(.{");
    updated.push_str(&block);
    updated.push_str("})");
    updated.push_str(&source[close + 1..]);
    Ok(FaceAuthoringPlan {
        language: capabilities().language.clone(),
        operation: AuthoringOperation::Modify,
        files: vec![PlannedFile {
            path: request.destination.clone(),
            original: Some(source.to_owned()),
            updated,
            edits: Vec::new(),
            remove: false,
        }],
        diagnostics: vec![PlanDiagnostic {
            severity: DiagnosticSeverity::Note,
            message: "the record block is rewritten in place; everything outside it is untouched"
                .to_owned(),
        }],
        obligations: obligations(&current_kind, &registration),
    })
}

/// The parent as Zig names it: the generated module constant, or the caller's own spelling.
/// 父级按 Zig 的命名：生成模块的常量，或调用方自己写好的拼法。
///
/// A declaration must name a **module constant** (`xirang_faces.shop__panel`, `xirang_faces.ROOT_NODE_ID`),
/// because the build resolves the parent's identity from that spelling before compiling — the same
/// rule the C++ header gives. An identity without a module, a logical catalog path, or a byte literal
/// is refused by name: this adapter cannot turn those into a spelling the build resolves.
/// 声明必须点名**模块常量**（`xirang_faces.shop__panel`、`xirang_faces.ROOT_NODE_ID`），因为构建在编译前
/// 就按那个拼法解析父身份——与 C++ 头的规则相同。没有模块的身份、逻辑目录路径或字节字面量都按名拒绝：
/// 本适配器不能把它们变成构建可解析的拼法。
fn parent_literal(parent: &ParentReference) -> Result<String, String> {
    Ok(match parent {
        ParentReference::Root => "xirang_faces.ROOT_NODE_ID".to_owned(),
        ParentReference::Object { module, .. } => {
            format!("xirang_faces.{}", module_flat(module))
        }
        ParentReference::Named(text) => text.clone(),
        ParentReference::Identity(_) => {
            return Err(
                "an identity without a module cannot be spelled as a Zig module constant; name the \
                 parent by its module instead"
                    .to_owned(),
            );
        }
        ParentReference::Catalog { path, .. } => {
            return Err(format!(
                "the catalog path `{path}` is not a Zig module constant; name the parent by its \
                 module instead"
            ));
        }
    })
}

/// The module path a source file takes: its directory path with `::` between segments.
/// 一份源码文件取得的模块路径：目录路径，段间用 `::`。
fn module_of(destination: &str) -> String {
    let path = std::path::Path::new(destination);
    let mut segments: Vec<String> = path
        .parent()
        .map(|parent| {
            parent
                .components()
                .filter_map(|part| part.as_os_str().to_str())
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default();
    segments.push(
        path.file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or("")
            .to_owned(),
    );
    segments.join("::")
}

/// The whole file one create writes.
/// 新建写出的整份文件。
fn render_file(
    parent: &str,
    registration: &RegistrationUpdate,
    extras: &[(String, String)],
) -> String {
    let block = render_block(parent, registration, extras);
    format!(
        "const xirang = @import(\"xirang\");\n\n\
         pub export var SOIL linksection(xirang.SECTION) = xirang.record(.{{{}}});\n",
        block
    )
}

/// The field block, one `.field = value,` per line, indented one level.
/// 字段块，每行一个 `.field = value,`，缩进一级。
fn render_block(
    parent: &str,
    registration: &RegistrationUpdate,
    extras: &[(String, String)],
) -> String {
    let mut out = String::from("\n");
    if !parent.is_empty() {
        out.push_str(&format!("    .parent = {parent},\n"));
    }
    for spec in GRAMMAR.fields {
        let field = match spec.role {
            Role::Kind => RegistrationField::Kind,
            Role::Field(field) => field,
            Role::Parent => continue,
        };
        let FieldUpdate::Set(value) = registration.find_field(field) else {
            continue;
        };
        if value.is_empty() {
            continue;
        }
        out.push_str(&format!(
            "    .{} = {},\n",
            spec.spelling,
            GRAMMAR.declared(spec.role, value)
        ));
    }
    for (name, value) in extras {
        out.push_str(&format!("    .{name} = {value},\n"));
    }
    out
}

fn obligations(kind: &str, registration: &RegistrationUpdate) -> Vec<ValidationObligation> {
    stated_obligations(
        kind,
        registration,
        "compile the package so the record lands in its `xirang.faces` section",
    )
}

#[cfg(test)]
#[path = "zig_tests.rs"]
mod zig_tests;
