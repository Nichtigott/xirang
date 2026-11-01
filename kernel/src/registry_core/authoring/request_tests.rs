//! Pins for the language-neutral authoring vocabulary.
//! 与语言无关的创作词汇的钉子。

use super::*;

fn rust() -> LanguageId {
    LanguageId::parse_language("rust").expect("a language id")
}

fn cpp() -> LanguageId {
    LanguageId::parse_language("cpp").expect("a language id")
}

fn rust_capabilities() -> LanguageCapabilities {
    LanguageCapabilities {
        language: rust(),
        file_extensions: &["rs"],
        operations: &[AuthoringOperation::Create, AuthoringOperation::Modify],
        fields: RegistrationField::ALL,
    }
}

fn cpp_capabilities() -> LanguageCapabilities {
    LanguageCapabilities {
        language: cpp(),
        file_extensions: &["cpp", "cc"],
        operations: &[AuthoringOperation::Create, AuthoringOperation::Modify],
        fields: &[
            RegistrationField::Kind,
            RegistrationField::Exports,
            RegistrationField::NeedsRegistry,
        ],
    }
}

fn request(language: LanguageId) -> FaceAuthoringRequest {
    FaceAuthoringRequest {
        language,
        operation: AuthoringOperation::Create,
        destination: "src/shop/panel/object/area/area.cpp".to_owned(),
        parent: ParentReference::Named("::xirang::faces::shop__panel_Panel".to_owned()),
        registration: RegistrationUpdate::new(),
        source_context: SourceContext::new(),
    }
}

/// The three-way distinction is the point of the type: an absent field and an explicitly empty one
/// must not collapse into one state, because on modify they mean opposite things.
/// 三态区分正是这个类型存在的意义：未提供字段与显式空值不能塌成同一个状态，因为在修改时它们含义相反。
#[test]
fn absent_and_explicitly_empty_are_different_states() {
    let mut update = RegistrationUpdate::new();
    update
        .set_text(RegistrationField::SummaryEn, FieldUpdate::Clear)
        .unwrap();
    assert_eq!(
        update.find_field(RegistrationField::SummaryEn),
        FieldUpdate::Clear
    );
    assert_eq!(
        update.find_field(RegistrationField::SummaryZh),
        FieldUpdate::Keep
    );
    assert_ne!(
        update.find_field(RegistrationField::SummaryEn),
        update.find_field(RegistrationField::SummaryZh)
    );
}

/// List fields normalize to one canonical form, so two requests that say the same thing compare
/// equal regardless of order or spacing — the round-trip comparison depends on this.
/// 列表字段归一成一种规范形式，因此说同一件事的两个请求无论顺序或空格都比较相等——往返比较依赖这一点。
#[test]
fn list_fields_normalize_order_and_duplicates() {
    let mut first = RegistrationUpdate::new();
    first
        .set_text(RegistrationField::Exports, FieldUpdate::Set("b, a"))
        .unwrap();
    let mut second = RegistrationUpdate::new();
    second
        .set_text(RegistrationField::Exports, FieldUpdate::Set("a,b,a"))
        .unwrap();
    assert_eq!(first, second);
    assert_eq!(
        first.find_field(RegistrationField::Exports),
        FieldUpdate::Set("a,b")
    );
}

/// A create request may leave a field alone; a modify request that leaves it alone must not be read
/// as "clear it". The type states both without a mode flag on the comparison.
/// 新建请求可以不动某个字段；修改请求不动它时不能被读成"清空它"。类型本身说明这两件事，不需要比较函数
/// 再带一个模式开关。
#[test]
fn keeping_a_field_is_not_clearing_it() {
    let mut kept = RegistrationUpdate::new();
    kept.set_text(RegistrationField::NameEn, FieldUpdate::Keep)
        .unwrap();
    let mut cleared = RegistrationUpdate::new();
    cleared
        .set_text(RegistrationField::NameEn, FieldUpdate::Clear)
        .unwrap();
    assert_ne!(kept, cleared);
    assert!(kept.is_empty(), "Keep names nothing");
    assert_eq!(cleared.named().count(), 1, "Clear names one field");
}

/// The round-trip comparison is semantic: a request that named a field agrees with a read-back
/// that carries the same normalized value, and disagrees when the value changed.
/// 往返比较是语义的：点名某字段的请求与携带同一规范化取值的读回一致，取值变了就不一致。
#[test]
fn round_trip_comparison_is_semantic() {
    let mut written = RegistrationUpdate::new();
    written
        .set_text(RegistrationField::Exports, FieldUpdate::Set("area, gauge"))
        .unwrap();
    let mut read_back = RegistrationUpdate::new();
    read_back.set_normalized(
        RegistrationField::Exports,
        FieldUpdate::Set("area,gauge".to_owned()),
    );
    assert!(written.agrees_with(&read_back));
    let mut changed = RegistrationUpdate::new();
    changed.set_normalized(
        RegistrationField::Exports,
        FieldUpdate::Set("area".to_owned()),
    );
    assert!(!written.agrees_with(&changed));
}

/// An unsupported field is refused by name, and a Rust-expression field is refused for the reason
/// that matters: the other language cannot consume an expression, not merely "unknown key".
/// 不支持的字段按名拒绝，而 Rust 表达式字段的拒绝理由才是关键：另一种语言消费不了表达式，而不是
/// 含混的"未知键"。
#[test]
fn unsupported_fields_are_refused_by_name_with_a_reason() {
    let mut update = RegistrationUpdate::new();
    update
        .set_text(RegistrationField::Flow, FieldUpdate::Set("a => b"))
        .unwrap();
    let mut request = request(cpp());
    request.registration = update;
    let error = validate_request(&request, &cpp_capabilities()).unwrap_err();
    assert!(error.contains("cpp"), "{error}");
    assert!(error.contains("flow"), "{error}");
    assert!(error.contains("Rust expression"), "{error}");
    assert!(
        !error.contains("ANY"),
        "a refusal must not suggest a default"
    );
}

/// The same request against a language that carries the field passes: the refusal is about the
/// capability table, not about the request being wrong.
/// 同一请求对着携带该字段的语言就通过：拒绝是关于能力表的，不是请求本身错了。
#[test]
fn the_same_field_passes_where_the_capability_table_carries_it() {
    let mut update = RegistrationUpdate::new();
    update
        .set_text(RegistrationField::Flow, FieldUpdate::Set("a => b"))
        .unwrap();
    let mut request = request(rust());
    request.registration = update;
    assert!(validate_request(&request, &rust_capabilities()).is_ok());
}

/// A language without an operation refuses it before rendering, so no surface can believe a
/// modify was carried out by a create-only adapter.
/// 没有某种操作的语言会在渲染前拒绝它，因此没有执行面会以为一个只支持新建的适配器完成了修改。
#[test]
fn an_unimplemented_operation_is_refused_before_rendering() {
    let capabilities = LanguageCapabilities {
        operations: &[AuthoringOperation::Create],
        ..cpp_capabilities()
    };
    let mut request = request(cpp());
    request.operation = AuthoringOperation::Modify;
    let error = validate_request(&request, &capabilities).unwrap_err();
    assert!(error.contains("modify"), "{error}");
    assert!(error.contains("no adapter"), "{error}");
}

/// A capability table for another language is not silently applied: comparing the ids is what keeps
/// a caller from validating a request against the wrong adapter's table.
/// 另一门语言的能力表不会被静默套用：比较标识正是防止调用方用错适配器的表来校验请求的东西。
#[test]
fn a_table_for_another_language_is_refused() {
    let error = validate_request(&request(rust()), &cpp_capabilities()).unwrap_err();
    assert!(error.contains("describes `cpp`"), "{error}");
    assert!(error.contains("targets `rust`"), "{error}");
}

/// Invalid names are refused when the field is set, not when the file is rendered: a bad module
/// name must not reach a path join.
/// 非法名字在设置字段时就被拒绝，而不是等到渲染文件：坏的模块名不得进入路径拼接。
#[test]
fn names_are_validated_at_the_field_boundary() {
    let mut update = RegistrationUpdate::new();
    assert!(
        update
            .set_text(RegistrationField::Module, FieldUpdate::Set("Bad-Name"))
            .is_err()
    );
    assert!(
        update
            .set_text(RegistrationField::Kind, FieldUpdate::Set("lower"))
            .is_err()
    );
    assert!(
        update
            .set_text(RegistrationField::NeedsRegistry, FieldUpdate::Set("yes"))
            .is_err()
    );
    assert!(
        update
            .set_text(RegistrationField::SummaryEn, FieldUpdate::Set("two\nlines"))
            .is_err()
    );
}

/// A plan that changes no bytes says so, which is what lets a surface answer "nothing to do"
/// instead of rewriting an identical file.
/// 不改变字节的计划会说出来，执行面因此能回答"无事可做"，而不是重写一份相同的文件。
#[test]
fn a_plan_with_no_changes_is_not_a_write() {
    let same = PlannedFile {
        path: "src/a.rs".to_owned(),
        original: Some("same".to_owned()),
        updated: "same".to_owned(),
        edits: Vec::new(),
        remove: false,
    };
    let created = PlannedFile {
        path: "src/b.cpp".to_owned(),
        original: None,
        updated: String::new(),
        edits: Vec::new(),
        remove: false,
    };
    let plan = FaceAuthoringPlan {
        language: cpp(),
        operation: AuthoringOperation::Create,
        files: vec![same.clone(), created.clone()],
        diagnostics: Vec::new(),
        obligations: Vec::new(),
    };
    assert!(!same.changes());
    assert!(created.creates());
    assert_eq!(plan.changed_files().count(), 1);
}
