//! Pins for the JavaScript adapter: generation, reading, precise rewrite, and a real import.
//! JavaScript 适配器的钉子：生成、读取、精确改写，以及一次真实导入。

use super::*;
use xirang_kernel::interchange::LanguageId;

fn javascript() -> LanguageId {
    LanguageId::parse_language("javascript").expect("a language id")
}

fn request(operation: AuthoringOperation) -> FaceAuthoringRequest {
    FaceAuthoringRequest {
        language: javascript(),
        operation,
        destination: "src/shop/panel/area/gauge.js".to_owned(),
        parent: ParentReference::Named("faces.shop.panel.area.NODE_ID".to_owned()),
        registration: RegistrationUpdate::new(),
        source_context: xirang_kernel::authoring::SourceContext::new(),
    }
}

fn with(registration: RegistrationUpdate) -> FaceAuthoringRequest {
    let mut request = request(AuthoringOperation::Create);
    request.registration = registration;
    request
}

fn field(registration: &mut RegistrationUpdate, field: RegistrationField, text: &str) {
    registration
        .set_text(field, FieldUpdate::Set(text))
        .expect("the fixture value is valid");
}

/// A create writes the call spelling: an import, the business type, and the field object.
/// 新建写出调用拼法：一条导入、业务类型，以及字段对象。
#[test]
fn a_create_writes_the_call_spelling() {
    let mut registration = RegistrationUpdate::new();
    field(&mut registration, RegistrationField::Kind, "Gauge");
    field(&mut registration, RegistrationField::Exports, "gauge");
    let plan = JsLanguage.plan(&with(registration)).expect("the plan");
    let source = &plan.files[0].updated;
    assert!(
        source.contains("import { faces, soil } from \"xirang\";"),
        "{source}"
    );
    assert!(
        source.contains("under: faces.shop.panel.area.NODE_ID,"),
        "{source}"
    );
    assert!(source.contains("named: \"Gauge\","), "{source}");
    assert!(source.contains("exports: [\"gauge\"],"), "{source}");
    assert!(source.contains("needs_registry: false,"), "{source}");
    assert!(source.contains("export class Gauge {"), "{source}");
    assert!(source.contains("soil.face({"), "{source}");
}

/// The generated declaration reads back to the same normalized registration.
/// 生成的声明能读回同一份规范注册。
#[test]
fn a_generated_declaration_reads_back_to_the_same_fields() {
    let mut registration = RegistrationUpdate::new();
    for (field_, text) in [
        (RegistrationField::Kind, "Gauge"),
        (RegistrationField::StableName, "stable_gauge"),
        (RegistrationField::Exports, "gauge,reading"),
        (RegistrationField::Provides, "geometry"),
        (RegistrationField::Requires, "clock=>SystemClock"),
        (RegistrationField::Preset, "gauge.preset"),
        (RegistrationField::Parts, "gauge.parts"),
        (RegistrationField::NameZh, "仪表"),
        (RegistrationField::NameEn, "Gauge"),
        (RegistrationField::SummaryZh, "一个仪表"),
        (RegistrationField::SummaryEn, "one gauge"),
        (RegistrationField::GettingFromOtherRegistry, "other"),
        (RegistrationField::NeedsRegistry, "true"),
    ] {
        field(&mut registration, field_, text);
    }
    let plan = JsLanguage
        .plan(&with(registration.clone()))
        .expect("the plan");
    let declaration =
        read_declaration(&plan.files[0].updated).expect("the generated source reads back");
    assert_eq!(declaration.kind(), "Gauge");
    assert!(
        registration.agrees_with(&declaration.face.registration),
        "the round trip changed a field:\n{}\n{:?}",
        plan.files[0].updated,
        declaration.face.registration
    );
}

/// A modify replaces only the object body: the import, the class and everything after survive.
/// 修改只替换对象体：导入、类与后面的一切逐字保留。
#[test]
fn a_modify_touches_only_the_object_body() {
    let source = "\
import { faces, soil } from \"xirang\";

/** Business code that must survive an edit. */
export class Gauge {
  reading() {
    return 42;
  }
}

soil.face({
  under: faces.shop.panel.area.NODE_ID,
  named: \"Gauge\",
  exports: [\"gauge\"],
  needs_registry: false,
});

// A trailing comment.
";
    let mut registration = RegistrationUpdate::new();
    field(
        &mut registration,
        RegistrationField::Exports,
        "gauge,reading",
    );
    let mut update = request(AuthoringOperation::Modify);
    update.registration = registration;
    update.source_context =
        xirang_kernel::authoring::SourceContext::new().with_existing_source(source);
    let plan = JsLanguage.plan(&update).expect("the plan");
    let updated = &plan.files[0].updated;
    assert!(
        updated.starts_with("import { faces, soil } from \"xirang\";\n\n/** Business code"),
        "the import and class survive:\n{updated}"
    );
    assert!(
        updated.contains("    return 42;\n  }\n}"),
        "the class body survives:\n{updated}"
    );
    assert!(
        updated.contains("exports: [\"gauge\", \"reading\"],"),
        "the named field is rewritten normalized:\n{updated}"
    );
    assert!(
        updated.ends_with("// A trailing comment.\n"),
        "everything after the declaration survives:\n{updated}"
    );
    assert_eq!(
        plan.files[0].edits.len(),
        1,
        "one precise edit, not a whole-file rewrite"
    );
}

/// A parent that is not a generated `faces.…NODE_ID` reference is refused by name.
/// 不是生成 `faces.…NODE_ID` 引用的父级按名拒绝。
#[test]
fn a_parent_outside_the_generated_directory_is_refused() {
    let mut request = with(RegistrationUpdate::new());
    request.parent = ParentReference::Named("panel.NODE_ID".to_owned());
    let error = JsLanguage
        .plan(&request)
        .expect_err("a bare expression is not a generated reference");
    assert!(error.contains("faces.<path>.NODE_ID"), "{error}");
}

/// Changing the kind in place is a rename, not a field edit, so it is refused.
/// 原地改 kind 是重命名而不是字段编辑，因此拒绝。
#[test]
fn changing_the_kind_in_place_is_refused() {
    let source = "soil.face({\n  under: faces.root.NODE_ID,\n  named: \"Gauge\",\n});\n";
    let mut registration = RegistrationUpdate::new();
    field(&mut registration, RegistrationField::Kind, "Dial");
    let mut request = request(AuthoringOperation::Modify);
    request.registration = registration;
    request.source_context =
        xirang_kernel::authoring::SourceContext::new().with_existing_source(source);
    let error = JsLanguage
        .plan(&request)
        .expect_err("a kind change is a rename");
    assert!(error.contains("renames the class"), "{error}");
}

/// A field the call cannot carry is refused, naming the field and the carrier.
/// 调用承载不了的字段被拒绝，并点名字段与载体。
#[test]
fn a_field_the_call_cannot_carry_is_refused_by_name() {
    let mut registration = RegistrationUpdate::new();
    field(
        &mut registration,
        RegistrationField::RegistrationRule,
        "crate::rule",
    );
    let error = JsLanguage
        .plan(&with(registration))
        .expect_err("the call has no place for a rule");
    assert!(error.contains("registration_rule"), "{error}");
    assert!(error.contains("the call carries"), "{error}");
}

/// A declaration without `under:` cannot be registered: the parent is its position.
/// 没有 `under:` 的声明无法注册：父级就是它的位置。
#[test]
fn a_declaration_without_a_parent_is_refused() {
    let error = read_declaration("soil.face({\n  named: \"Gauge\",\n});\n")
        .expect_err("a face without a parent has no position");
    assert!(error.contains("under:"), "{error}");
}

/// Node is the interpreter these declarations run under; its absence skips the execution pin.
/// Node 是这些声明运行的解释器；它不存在时跳过执行钉子。
fn node() -> Option<String> {
    let output = std::process::Command::new("node")
        .arg("--version")
        .output()
        .ok()?;
    output.status.success().then(|| "node".to_owned())
}

/// The generated source really imports and registers against the SDK.
/// 生成的源码真的能在 SDK 下导入并登记声明。
#[test]
fn a_generated_face_imports_against_the_sdk() {
    let Some(node) = node() else {
        eprintln!("no node on PATH; the generated source was not executed");
        return;
    };
    let mut registration = RegistrationUpdate::new();
    field(&mut registration, RegistrationField::Kind, "Gauge");
    field(&mut registration, RegistrationField::Exports, "gauge");
    let plan = JsLanguage.plan(&with(registration)).expect("the plan");
    let directory = std::env::temp_dir().join(format!(
        "xirang-javascript-authoring-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("the clock")
            .as_nanos()
    ));
    std::fs::create_dir_all(directory.join("node_modules")).expect("the fixture directory");
    std::fs::write(directory.join("gauge.js"), &plan.files[0].updated)
        .expect("the generated source");
    std::os::unix::fs::symlink(
        crate::sdk::js_include_dir().join("xirang"),
        directory.join("node_modules/xirang"),
    )
    .expect("the SDK link");
    std::fs::write(
        directory.join("check.mjs"),
        "import \"./gauge.js\";\n\
         import { declarations } from \"xirang\";\n\
         const faces = declarations();\n\
         if (faces.length !== 1) throw new Error(`one declaration, got ${faces.length}`);\n\
         if (faces[0].named !== \"Gauge\") throw new Error(`named is ${faces[0].named}`);\n\
         console.log(\"registered\");\n",
    )
    .expect("the check script");
    let output = std::process::Command::new(node)
        .arg("check.mjs")
        .current_dir(&directory)
        .output()
        .expect("node runs");
    let _ = std::fs::remove_dir_all(&directory);
    assert!(
        output.status.success(),
        "node refused the generated source:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("registered"),
        "the declaration did not register: {}",
        String::from_utf8_lossy(&output.stdout)
    );
}

/// The committed example reads back to the declaration its test asserts at import time.
/// 入库的示例读回成它导入期测试所断言的同一条声明。
#[test]
fn the_committed_example_reads_back_to_the_same_declaration() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../examples/js-gauge");
    let faces = faces::read_declarations(&root).expect("the example's declarations");
    assert_eq!(faces.len(), 1, "one declaration in the example: {faces:?}");
    assert_eq!(faces[0].source, "src/gauge.js");
    assert_eq!(faces[0].name, "Gauge");
    assert_eq!(faces[0].parent, "root");
    assert_eq!(faces[0].exports, vec!["gauge".to_owned()]);
    assert!(!faces[0].owns_registry);
}
