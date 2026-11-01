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
    AuthoringOperation, FaceAuthoringPlan, FaceAuthoringRequest, LanguageCapabilities,
    RegistrationField, RegistrationUpdate,
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
