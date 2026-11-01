//! Pins for the C adapter: generation, reading, precise rewrite, and a real `cc` compile.
//! C 适配器的钉子：生成、读取、精确改写，以及一次真实 `cc` 编译。

use super::*;
use xirang_kernel::authoring::{
    FaceAuthoringRequest, ParentReference, RegistrationUpdate, SourceContext,
};
use xirang_kernel::interchange::LanguageId;

fn c() -> LanguageId {
    LanguageId::parse_language("c").expect("a language id")
}

fn request(operation: AuthoringOperation) -> FaceAuthoringRequest {
    FaceAuthoringRequest {
        language: c(),
        operation,
        destination: "native/gauge/gauge.c".to_owned(),
        parent: ParentReference::Root,
        registration: RegistrationUpdate::new(),
        source_context: SourceContext::new(),
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

/// A create writes the designated-initializer spelling: includes, header macro, parent, kind.
/// 新建写出指定初始化器拼法：include、头宏、父级、kind。
#[test]
fn a_create_writes_the_designated_initializer_spelling() {
    let mut registration = RegistrationUpdate::new();
    field(&mut registration, RegistrationField::Kind, "Gauge");
    field(&mut registration, RegistrationField::Exports, "gauge");
    let plan = CLanguage.plan(&with(registration)).expect("the plan");
    let source = &plan.files[0].updated;
    assert!(source.contains("#include \"xirang/faces.h\"\n"), "{source}");
    assert!(source.contains("#include \"xirang/soil.h\"\n"), "{source}");
    assert!(source.contains("XIRANG_SOIL gauge = {\n"), "{source}");
    assert!(source.contains("    XIRANG_RECORD_HEADER,\n"), "{source}");
    assert!(
        source.contains("    .parent = XIRANG_ROOT_NODE_ID,\n"),
        "{source}"
    );
    assert!(source.contains("    .kind = \"Gauge\",\n"), "{source}");
    assert!(source.contains("    .exports = \"gauge\",\n"), "{source}");
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
        (RegistrationField::NeedsRegistry, "true"),
        (RegistrationField::GettingFromOtherRegistry, "other"),
    ] {
        field(&mut registration, field_, text);
    }
    let plan = CLanguage
        .plan(&with(registration.clone()))
        .expect("the plan");
    let declaration =
        read_declaration(&plan.files[0].updated).expect("the generated source reads back");
    assert_eq!(declaration.face.kind(), "Gauge");
    assert!(
        registration.agrees_with(&declaration.face.registration),
        "the round trip changed a field:\n{}\n{:?}",
        plan.files[0].updated,
        declaration.face.registration
    );
}

/// A modify replaces only the initializer body: includes, trailing code and comments survive.
/// 修改只替换初始化器体：include、后面的代码与注释逐字保留。
#[test]
fn a_modify_touches_only_the_initializer_body() {
    let source = "\
#include \"xirang/faces.h\"
#include \"xirang/soil.h\"

XIRANG_SOIL gauge = {
    XIRANG_RECORD_HEADER,
    .parent = XIRANG_ROOT_NODE_ID,
    .kind = \"Gauge\",
    .exports = \"gauge\",
};

// A business function that must survive an edit.
int gauge_reading(void) {
    return 42;
}
";
    let mut registration = RegistrationUpdate::new();
    field(
        &mut registration,
        RegistrationField::Exports,
        "gauge,reading",
    );
    let mut update = request(AuthoringOperation::Modify);
    update.registration = registration;
    update.source_context = SourceContext::new().with_existing_source(source);
    let plan = CLanguage.plan(&update).expect("the plan");
    let updated = &plan.files[0].updated;
    assert!(
        updated.contains("int gauge_reading(void) {\n    return 42;\n}\n"),
        "the business function survives:\n{updated}"
    );
    assert!(
        updated.contains("    .exports = \"gauge,reading\",\n"),
        "the named member is rewritten normalized:\n{updated}"
    );
    assert_eq!(plan.files[0].edits.len(), 1, "one precise edit");
}

/// A parent is named by the generated constant for its module, the same spelling the header writes.
/// 父级用其模块的生成常量点名，与头文件写出的拼法相同。
#[test]
fn a_parent_is_named_by_the_generated_face_constant() {
    let mut request = with(RegistrationUpdate::new());
    request.parent = ParentReference::Object {
        id: xirang_kernel::identity::NodeId::from_raw([1; 16]),
        module: "shop::panel".to_owned(),
    };
    let plan = CLanguage.plan(&request).expect("the parent is nameable");
    let source = &plan.files[0].updated;
    assert!(
        source.contains("    .parent = XIRANG_FACES_SHOP__PANEL,\n"),
        "{source}"
    );
}

/// An empty parent module is refused by name.
/// 空的父模块按名拒绝。
#[test]
fn an_empty_parent_module_is_refused() {
    let mut request = with(RegistrationUpdate::new());
    request.parent = ParentReference::Object {
        id: xirang_kernel::identity::NodeId::from_raw([1; 16]),
        module: String::new(),
    };
    let error = CLanguage
        .plan(&request)
        .expect_err("an empty module names no constant");
    assert!(error.contains("empty parent module"), "{error}");
}

/// Changing the kind in place is a rename, not a field edit, so it is refused.
/// 原地改 kind 是重命名而不是字段编辑，因此拒绝。
#[test]
fn changing_the_kind_in_place_is_refused() {
    let source = "XIRANG_SOIL gauge = {\n    XIRANG_RECORD_HEADER,\n    .parent = \
                  XIRANG_ROOT_NODE_ID,\n    .kind = \"Gauge\",\n};\n";
    let mut registration = RegistrationUpdate::new();
    field(&mut registration, RegistrationField::Kind, "Dial");
    let mut request = request(AuthoringOperation::Modify);
    request.registration = registration;
    request.source_context = SourceContext::new().with_existing_source(source);
    let error = CLanguage
        .plan(&request)
        .expect_err("a kind change is a rename");
    assert!(error.contains("renames the face's identity"), "{error}");
}

/// The committed example reads back to the declaration its runtime test asserts.
/// 入库的示例读回成它运行期测试所断言的同一条声明。
#[test]
fn the_committed_example_reads_back_to_the_same_declaration() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../examples/c-gauge/gauge");
    let source = std::fs::read_to_string(root.join("native/gauge/gauge.c")).expect("the example");
    let declaration = read_declaration(&source).expect("the example's declaration");
    assert_eq!(declaration.face.kind(), "Gauge");
    assert_eq!(declaration.face.parent, "XIRANG_FACES_PANEL");
}

/// The C compiler, when this machine has one.
/// 本机有 C 编译器时返回它。
fn cc() -> Option<String> {
    if let Ok(from_env) = std::env::var("XIRANG_CC")
        && std::process::Command::new(&from_env)
            .arg("--version")
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    {
        return Some(from_env);
    }
    std::process::Command::new("cc")
        .arg("--version")
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|_| "cc".to_owned())
}

/// The generated source really compiles against the SDK and the generated header (skipped without
/// a C compiler).
/// 生成的源码真的能在 SDK 与生成的头部下编译（没有 C 编译器时跳过）。
#[test]
fn a_generated_declaration_compiles_against_the_sdk() {
    let Some(cc) = cc() else {
        eprintln!("no C compiler (XIRANG_CC or `cc`); the generated source was not compiled");
        return;
    };
    let mut registration = RegistrationUpdate::new();
    field(&mut registration, RegistrationField::Kind, "Gauge");
    field(&mut registration, RegistrationField::Exports, "gauge");
    let mut request = with(registration);
    request.parent = ParentReference::Object {
        id: xirang_kernel::identity::NodeId::from_raw([1; 16]),
        module: "panel".to_owned(),
    };
    let plan = CLanguage.plan(&request).expect("the plan");
    let directory = std::env::temp_dir().join(format!(
        "xirang-c-authoring-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("the clock")
            .as_nanos()
    ));
    std::fs::create_dir_all(directory.join("xirang")).expect("the fixture directory");
    std::fs::write(directory.join("gauge.c"), &plan.files[0].updated)
        .expect("the generated source");
    std::fs::write(
        directory.join("xirang/faces.h"),
        "#define XIRANG_ROOT_NODE_ID { 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00 }\n#define XIRANG_FACES_PANEL { 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11 }\n",
    )
    .expect("the generated header");
    let output = std::process::Command::new(cc)
        .arg("-c")
        .arg("-I")
        .arg(crate::sdk::c_include_dir())
        .arg("-I")
        .arg(&directory)
        .arg(directory.join("gauge.c"))
        .arg("-o")
        .arg(directory.join("gauge.o"))
        .output()
        .expect("cc runs");
    let _ = std::fs::remove_dir_all(&directory);
    assert!(
        output.status.success(),
        "cc refused the generated source:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
