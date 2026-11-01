//! Pins for the shared adapter orchestration: the capability table, the refusal, and the dispatch.
//! 共享适配器编排的钉子：能力表、拒绝与派发。

use std::sync::OnceLock;

use xirang_kernel::authoring::{
    AuthoringOperation, FaceAuthoringPlan, FaceAuthoringRequest, FieldUpdate, LanguageCapabilities,
    RegistrationField, RegistrationUpdate,
};

use super::{AdapterSpec, capabilities, refuse_unmapped};

const SPEC: AdapterSpec = AdapterSpec {
    language: "toy",
    extensions: &["toy"],
    fields: &[RegistrationField::Kind, RegistrationField::Exports],
    operations: &[AuthoringOperation::Create],
    carrying: "a toy declaration is one marked line",
};

static LOCK: OnceLock<LanguageCapabilities> = OnceLock::new();

/// The capability table is the spec, field by field — one source for the language id, extensions,
/// operations and fields.
/// 能力表就是 spec 逐字段的样子——语言 id、扩展名、操作与字段只有一份来源。
#[test]
fn capabilities_are_the_spec_field_by_field() {
    let table = capabilities(&LOCK, &SPEC);
    assert_eq!(table.language.as_str(), "toy");
    assert_eq!(table.file_extensions, &["toy"]);
    assert_eq!(table.operations, &[AuthoringOperation::Create]);
    assert_eq!(
        table.fields,
        &[RegistrationField::Kind, RegistrationField::Exports]
    );
}

/// A field the carrier cannot write is refused by name and by carrier; a field it can write passes.
/// 载体写不出的字段被按字段名与载体拒绝；能写出的字段通过。
#[test]
fn the_refusal_names_the_field_and_the_carrier() {
    let mut unmapped = RegistrationUpdate::new();
    unmapped.set_normalized(
        RegistrationField::RegistrationRule,
        FieldUpdate::Set("crate::rule::RULE".to_owned()),
    );
    let error = refuse_unmapped(&SPEC, &unmapped).expect_err("the rule field is not carried");
    assert!(
        error.contains("`toy` cannot carry `registration_rule`"),
        "{error}"
    );
    assert!(
        error.contains("a toy declaration is one marked line"),
        "{error}"
    );

    let mut carried = RegistrationUpdate::new();
    carried.set_normalized(
        RegistrationField::Kind,
        FieldUpdate::Set("Gauge".to_owned()),
    );
    assert!(refuse_unmapped(&SPEC, &carried).is_ok());
}

/// The dispatch calls create for a create and modify for a modify, and never the other one.
/// 派发对 create 调 create、对 modify 调 modify，绝不调错。
#[test]
fn the_dispatch_follows_the_operation() {
    fn create(_request: &FaceAuthoringRequest) -> Result<FaceAuthoringPlan, String> {
        Err("create".to_owned())
    }
    fn modify(_request: &FaceAuthoringRequest) -> Result<FaceAuthoringPlan, String> {
        Err("modify".to_owned())
    }
    let mut request = FaceAuthoringRequest {
        language: xirang_kernel::interchange::LanguageId::parse_language("toy")
            .expect("a language id"),
        operation: AuthoringOperation::Create,
        destination: "src/a.toy".to_owned(),
        parent: xirang_kernel::authoring::ParentReference::Named("x".to_owned()),
        registration: RegistrationUpdate::new(),
        source_context: xirang_kernel::authoring::SourceContext::new(),
    };
    assert_eq!(
        super::plan(&SPEC, &request, create, modify).unwrap_err(),
        "create"
    );
    request.operation = AuthoringOperation::Modify;
    assert_eq!(
        super::plan(&SPEC, &request, create, modify).unwrap_err(),
        "modify"
    );
}
