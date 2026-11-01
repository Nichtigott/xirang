//! The shared adapter orchestration: capabilities from a spec, the unmapped-field refusal, and the
//! read-back shape.
//! 共享的适配器编排：由 spec 产生的 capabilities、未映射字段的拒绝、以及读回的形状。
//!
//! What stays in a language is what only it can answer: which extensions it claims, what its carrier
//! is called in a refusal, and how it renders or parses one declaration. The dispatch, the capability
//! table and the read-back construction are the same in every language, so they live here.
//! 留在语言里的是只有它能回答的东西：认领哪些扩展名、它的载体在拒绝里叫什么、以及它怎么渲染或解析一条
//! 声明。派发、能力表与读回构造在每个语言里都一样，因此住在这里。

use std::sync::OnceLock;

use xirang_kernel::authoring::{
    AuthoringOperation, DiagnosticSeverity, FaceAuthoringPlan, FaceAuthoringRequest, FieldUpdate,
    LanguageCapabilities, PlanDiagnostic, PlannedEdit, PlannedFile, RegistrationField,
    RegistrationUpdate, ValidationObligation,
};
use xirang_kernel::interchange::LanguageId;

use crate::languages::kit::grammar::DeclaredFace;
use crate::registration::FaceReadBack;

/// What one language tells the shared adapter about itself.
/// 一门语言告诉共享适配器的东西。
pub struct AdapterSpec {
    /// The language id this adapter answers for.
    /// 本适配器对应的语言 id。
    pub language: &'static str,
    /// The file-name suffixes it claims.
    /// 它认领的文件名后缀。
    pub extensions: &'static [&'static str],
    /// The registration fields its carrier can write.
    /// 它的载体能写出的注册字段。
    pub fields: &'static [RegistrationField],
    /// The structural operations it implements.
    /// 它实现的结构操作。
    pub operations: &'static [AuthoringOperation],
    /// What the carrier is, in the refusal sentence: a language that says "a C++ declaration is a
    /// record of text fields" and one that says "the decorator carries text and scalar fields" are
    /// refusing for the same reason, but a reader has to know which carrier they are looking at.
    /// 载体是什么，用在拒绝句里：说"a C++ declaration is a record of text fields"的语言与说
    /// "the decorator carries text and scalar fields"的语言是在按同一个理由拒绝，但读者需要知道
    /// 自己看的是哪种载体。
    pub carrying: &'static str,
}

/// The capability table for one language, built once.
/// 一门语言的能力表，只构建一次。
pub fn capabilities(
    lock: &'static OnceLock<LanguageCapabilities>,
    spec: &'static AdapterSpec,
) -> &'static LanguageCapabilities {
    lock.get_or_init(|| LanguageCapabilities {
        language: LanguageId::parse_language(spec.language).expect("the spec names a language id"),
        file_extensions: spec.extensions,
        operations: spec.operations,
        fields: spec.fields,
    })
}

/// Refuse any field this carrier cannot write, naming the field and the carrier.
/// 拒绝本载体写不出的任何字段，点名字段与载体。
pub fn refuse_unmapped(
    spec: &AdapterSpec,
    registration: &RegistrationUpdate,
) -> Result<(), String> {
    for (field, _) in registration.named() {
        if !spec.fields.contains(&field) {
            return Err(format!(
                "`{}` cannot carry `{}`: {}, and this one's value would be a Rust expression or a \
                 rule it has no place for; drop it or name it in a Rust face",
                spec.language,
                field.as_str(),
                spec.carrying
            ));
        }
    }
    Ok(())
}

/// The module a destination file names: its stem, folded into an identifier.
/// 目的地文件点名的模块：文件名主干，折成标识符。
///
/// Seven adapters wrote this function out verbatim. It lives here so a change to the folding rule
/// (which characters become `_`, and what a stemless spelling is refused with) has one home.
/// 七个适配器一字不差地写过这个函数。放在这里，让折叠规则（哪些字符变成 `_`，以及无主干的拼写被什么理由
/// 拒绝）只有一个家。
pub fn destination_module(destination: &str) -> Result<String, String> {
    std::path::Path::new(destination)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .map(|stem| stem.replace(['-', '.'], "_"))
        .ok_or_else(|| {
            format!("`{destination}` has no file stem this adapter can name a module after")
        })
}

/// A declaration the shared in-place `modify` can rewrite.
/// 共享的就地 `modify` 可以改写的一条声明。
///
/// The carrier is the language's own type (a Java annotation, a C++ expression, a Go block); the
/// shared edit needs only the payload it registers, the byte range it rewrites, and the text it
/// renders, so each adapter answers those three questions and the plan assembly stays in one place.
/// 载体是语言自己的类型（Java 注解、C++ 表达式、Go 块）；共享编辑只需要它注册的载荷、它改写的那段字节区间
/// 与它渲染出的文本，因此每个适配器回答这三个问题，而计划装配只留在一处。
pub trait Editable {
    /// The language-neutral payload: where it hangs and what it registers.
    /// 语言中立载荷：它挂在哪里、注册什么。
    fn face(&self) -> &DeclaredFace;

    /// The byte range of the text a modify replaces.
    /// 一次修改要替换的那段文本的字节区间。
    fn arguments(&self) -> std::ops::Range<usize>;

    /// The replacement text for this declaration's arguments, as the language spells them.
    /// 本条声明实参的替换文本，按该语言的拼法写出。
    fn rendered(&self, parent: &str, registration: &RegistrationUpdate) -> String;
}

/// What one language supplies to the shared `modify`.
/// 一门语言给共享 `modify` 的东西。
pub struct EditHooks<D> {
    /// How to read the declaration out of source text.
    /// 怎么从源码文本里读出声明的。
    pub read: fn(&str) -> Result<D, String>,
    /// How the request names the parent, in the language's spelling.
    /// 请求按该语言的拼法怎么点名父级。
    pub parent: fn(&FaceAuthoringRequest) -> Result<String, String>,
    /// The obligations the plan must state for this language.
    /// 该语言下计划必须声明的义务。
    pub obligations: fn(&str, &RegistrationUpdate) -> Vec<ValidationObligation>,
    /// What a `kind` change would rename, or `None` when the carrier has no name of its own to
    /// rename. The refusal sentence is the same either way; only that half differs.
    /// 改 `kind` 会重命名什么；载体没有自己的名字可改时为 `None`。拒绝句在两种情况下相同，只有那一半不同。
    pub carrier: Option<&'static str>,
}

/// The shared `modify`: rewrite one declaration's arguments in place, preserving everything the
/// request does not name.
/// 共享的 `modify`：就地改写一条声明的实参，保留请求没有点名的一切。
///
/// Every adapter wrote this assembly out longhand, because the carrier differs but the edit does
/// not. Three rules are why it has one home: changing the module is a move and changing `kind` is
/// a rename, so both are refused rather than performed; a rewrite that changes nothing says so
/// instead of pretending to have written something; and the edit list is empty exactly when the
/// updated text equals the source, so an empty edit never claims a change the file does not have.
/// 每个适配器都曾把这套装配逐行写开，因为载体不同而这次编辑相同。三条规则是它只有一个家的原因：改模块是
/// 搬动、改 `kind` 是重命名，因此两者被拒绝而不是被实施；没有改动任何东西的改写会说出来，而不是假装写了
/// 东西；改写后的文本与源码相同时编辑表恰好为空，因此空编辑从不声称文件里没有的变化。
pub fn modify<D: Editable>(
    language: &LanguageId,
    request: &FaceAuthoringRequest,
    hooks: &EditHooks<D>,
) -> Result<FaceAuthoringPlan, String> {
    let source = request.source_context.source_text().ok_or_else(|| {
        format!(
            "{} does not exist; modify needs the file",
            request.destination
        )
    })?;
    let declaration = (hooks.read)(source)?;
    let module = destination_module(&request.destination)?;
    if let FieldUpdate::Set(named) = request.registration.find_field(RegistrationField::Module)
        && named != module
    {
        return Err(format!(
            "changing the module from `{module}` to `{named}` is a move, not a field edit; this \
             adapter rewrites the declaration in place"
        ));
    }
    let current_kind = declaration.face().kind();
    let kind = match request.registration.find_field(RegistrationField::Kind) {
        FieldUpdate::Keep => current_kind.clone(),
        FieldUpdate::Set(kind) if kind == current_kind => current_kind.clone(),
        FieldUpdate::Set(kind) => {
            let renamed = match hooks.carrier {
                Some(carrier) => format!("renames the {carrier} and the face's identity"),
                None => "renames the face's identity".to_owned(),
            };
            return Err(format!(
                "changing `kind` from `{current_kind}` to `{kind}` {renamed}; that is a rename, \
                 not a field edit"
            ));
        }
        FieldUpdate::Clear => return Err("`kind` cannot be empty; it names the face".to_owned()),
    };
    // Start from what the source says, then apply only the fields the request names: a field it
    // omits keeps its current value, which is the difference between `Keep` and `Clear`.
    // 从源码读到的内容开始，只应用请求点名的字段：请求省略的字段保留当前值，这就是 `Keep` 与 `Clear` 的区别。
    let mut registration = declaration.face().registration.clone();
    for (field, _) in request.registration.named() {
        let update = match request.registration.find_field(field) {
            FieldUpdate::Keep => FieldUpdate::Keep,
            FieldUpdate::Clear => FieldUpdate::Clear,
            FieldUpdate::Set(value) => FieldUpdate::Set(value.to_owned()),
        };
        registration.set_normalized(field, update);
    }
    registration.set_normalized(RegistrationField::Kind, FieldUpdate::Set(kind.clone()));
    // The parent travels on the request, not in the sparse field map: a modify request that does
    // not move the face still has to name the parent for the re-rendered declaration.
    // 父随请求传递，不在稀疏字段表里：不移动面的修改请求仍须为重新渲染的声明点名父级。
    let parent = (hooks.parent)(request)?;
    let replacement = declaration.rendered(&parent, &registration);
    let range = declaration.arguments();
    let mut updated = String::with_capacity(source.len() + replacement.len());
    updated.push_str(&source[..range.start]);
    updated.push_str(&replacement);
    updated.push_str(&source[range.end..]);
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
            start: range.start,
            end: range.end,
            replacement,
        }]
    };
    Ok(FaceAuthoringPlan {
        language: language.clone(),
        operation: AuthoringOperation::Modify,
        files: vec![PlannedFile {
            path: request.destination.clone(),
            original: Some(source.to_owned()),
            updated,
            edits,
            remove: false,
        }],
        diagnostics,
        obligations: (hooks.obligations)(&kind, &registration),
    })
}

/// The shared read-back: parse one declaration and hand back the fields the authoring service
/// reads. (`read` is a bare verb, reserved for entry positions.)
/// 共享的读回：解析一条声明，交回创作服务读取的字段。（`read` 是裸动词、只留给入口位置。）
pub fn read_back(
    declaration: fn(&str) -> Result<DeclaredFace, String>,
    source: &str,
) -> Result<FaceReadBack, String> {
    let face = declaration(source)?;
    Ok(FaceReadBack {
        registration: face.registration,
        parent: xirang_kernel::authoring::ParentReference::Named(face.parent),
    })
}

/// The shared `plan`: refuse unmapped fields, then dispatch to the language's create/modify.
/// 共享的 `plan`：拒绝未映射字段，再派发给语言的 create/modify。
pub fn plan(
    spec: &AdapterSpec,
    request: &FaceAuthoringRequest,
    create: fn(&FaceAuthoringRequest) -> Result<FaceAuthoringPlan, String>,
    modify: fn(&FaceAuthoringRequest) -> Result<FaceAuthoringPlan, String>,
) -> Result<FaceAuthoringPlan, String> {
    refuse_unmapped(spec, &request.registration)?;
    match request.operation {
        AuthoringOperation::Create => create(request),
        AuthoringOperation::Modify => modify(request),
    }
}

#[cfg(test)]
#[path = "adapter_tests.rs"]
mod adapter_tests;
