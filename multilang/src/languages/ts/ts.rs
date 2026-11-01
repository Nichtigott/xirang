//! The TypeScript language implementation: generate, read, and precisely rewrite one
//! `soil.face({…})` call, with the declaration object type-checked against the SDK.
//! TypeScript 语言实现：生成、读取并精确改写一条 `soil.face({…})` 调用，并让声明对象对照 SDK 做类型
//! 检查。
//!
//! Uses the language kit: the capability table, the declaration walk and the adapter orchestration
//! come from `crate::languages::kit`; this module owns the TypeScript spelling and reuses the
//! JavaScript parser, because the carrier is the same call — TypeScript adds a type to the object,
//! not a second syntax.
//! 使用语言 kit：能力表、声明遍历与适配器编排来自 `crate::languages::kit`；本模块拥有 TypeScript 的拼法
//! 并复用 JavaScript 解析器，因为载体是同一次调用——TypeScript 给对象加类型，而不是第二套语法。
//!
//! The SDK at `sdk/ts/xirang` is the other half of this file's spelling: its runtime re-exports the
//! JavaScript SDK, and its `index.d.ts` is what `tsc` uses to check the declaration object.
//! `sdk/ts/xirang` 的 SDK 是本文件拼法的另一半：它的运行期重导出 JavaScript SDK，而它的 `index.d.ts`
//! 正是 `tsc` 校验声明对象所用的东西。

use std::sync::OnceLock;

use xirang_kernel::authoring::{
    AuthoringOperation, FaceAuthoringPlan, FaceAuthoringRequest, LanguageCapabilities,
};
use xirang_kernel::interchange::LanguageId;

use crate::languages::js;
use crate::languages::kit::adapter::{AdapterSpec, capabilities as kit_capabilities};
use crate::languages::kit::grammar::DeclaredFace;
use crate::registration::LanguageAdapter;

#[path = "ts_faces.rs"]
pub mod faces;

/// The TypeScript language implementation.
/// TypeScript 语言实现。
#[derive(Clone, Copy, Debug, Default)]
pub struct TsLanguage;

/// What the TypeScript adapter tells the shared orchestration about itself.
/// TypeScript 适配器告诉共享编排的东西。
///
/// The fields are the JavaScript carrier's fields: the call is the same object, and the type
/// declaration in the SDK names exactly this set. A field the SDK's interface does not know is
/// refused by the same table.
/// 字段就是 JavaScript 载体的字段：调用是同一个对象，SDK 里的类型声明点名的正是这一组。SDK 接口不认识的
/// 字段会被同一张表拒绝。
const SPEC: AdapterSpec = AdapterSpec {
    language: "typescript",
    extensions: &["ts"],
    fields: js::FIELDS,
    operations: &[AuthoringOperation::Create, AuthoringOperation::Modify],
    carrying: "the typed call carries text, list and boolean fields",
};

static CAPABILITIES: OnceLock<LanguageCapabilities> = OnceLock::new();

/// The capability table for TypeScript.
/// TypeScript 的能力表。
fn capabilities() -> &'static LanguageCapabilities {
    kit_capabilities(&CAPABILITIES, &SPEC)
}

impl LanguageAdapter for TsLanguage {
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
    js::read_declaration(source).map(|declaration| declaration.face)
}

/// The language id the plans of this adapter carry.
/// 本适配器的计划携带的语言 id。
fn language() -> LanguageId {
    capabilities().language.clone()
}

fn create(request: &FaceAuthoringRequest) -> Result<FaceAuthoringPlan, String> {
    js::create_for(&language(), &crate::sdk::ts_include_dir(), request)
}

fn modify(request: &FaceAuthoringRequest) -> Result<FaceAuthoringPlan, String> {
    js::modify_for(&language(), request)
}

#[cfg(test)]
#[path = "ts_tests.rs"]
mod ts_tests;
