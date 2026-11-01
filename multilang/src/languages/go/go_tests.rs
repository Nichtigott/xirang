//! Pins for the Go adapter: generation, reading, precise rewrite, and a real `go build`.
//! Go 适配器的钉子：生成、读取、精确改写，以及一次真实 `go build`。

use super::*;
use xirang_kernel::authoring::{
    FaceAuthoringRequest, ParentReference, RegistrationUpdate, SourceContext,
};
use xirang_kernel::interchange::LanguageId;

fn go() -> LanguageId {
    LanguageId::parse_language("go").expect("a language id")
}

fn request(operation: AuthoringOperation) -> FaceAuthoringRequest {
    FaceAuthoringRequest {
        language: go(),
        operation,
        destination: "go/src/gauge/gauge.go".to_owned(),
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

/// A create writes the Go spelling: package clause, SDK import, business type, and the literal.
/// 新建写出 Go 拼法：包子句、SDK 导入、业务类型，以及字面量。
#[test]
fn a_create_writes_the_composite_literal_spelling() {
    let mut registration = RegistrationUpdate::new();
    field(&mut registration, RegistrationField::Kind, "Gauge");
    field(&mut registration, RegistrationField::Exports, "gauge");
    let plan = GoLanguage.plan(&with(registration)).expect("the plan");
    let source = &plan.files[0].updated;
    assert!(source.contains("package gauge\n"), "{source}");
    assert!(
        source.contains("import \"xirang.dev/sdk/xirang\"\n"),
        "{source}"
    );
    assert!(source.contains("type Gauge struct{}\n"), "{source}");
    assert!(source.contains("var _ = xirang.Soil{"), "{source}");
    assert!(source.contains("\tParent: rootNodeID,\n"), "{source}");
    assert!(source.contains("\tNamed: \"Gauge\",\n"), "{source}");
    assert!(
        source.contains("\tExports: []string{\"gauge\"},\n"),
        "{source}"
    );
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
    ] {
        field(&mut registration, field_, text);
    }
    let plan = GoLanguage
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

/// A modify replaces only the literal's body: package, import, type and trailing code survive.
/// 修改只替换字面量体：包、导入、类型与后面的代码逐字保留。
#[test]
fn a_modify_touches_only_the_literal_body() {
    let source = "\
package gauge

import \"xirang.dev/sdk/xirang\"

// Gauge is the business type this face is named after.
type Gauge struct{}

var _ = xirang.Soil{
\tParent: rootNodeID,
\tNamed: \"Gauge\",
\tExports: []string{\"gauge\"},
}

// Reading is an ordinary business method.
func (Gauge) Reading() int {
\treturn 42
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
    let plan = GoLanguage.plan(&update).expect("the plan");
    let updated = &plan.files[0].updated;
    assert!(updated.starts_with("package gauge\n\n"), "{updated}");
    assert!(
        updated.contains("func (Gauge) Reading() int {\n\treturn 42\n}\n"),
        "the business method survives:\n{updated}"
    );
    assert!(
        updated.contains("\tExports: []string{\"gauge\", \"reading\"},\n"),
        "the named field is rewritten normalized:\n{updated}"
    );
    assert_eq!(plan.files[0].edits.len(), 1, "one precise edit");
}

/// A parent is named by the generated constant for its module, the same rule the build writes.
/// 父级用其模块的生成常量点名，与构建写出的规则相同。
#[test]
fn a_parent_is_named_by_the_generated_face_constant() {
    let mut request = with(RegistrationUpdate::new());
    request.parent = ParentReference::Object {
        id: xirang_kernel::identity::NodeId::from_raw([1; 16]),
        module: "shop::panel".to_owned(),
    };
    let plan = GoLanguage.plan(&request).expect("the parent is nameable");
    let source = &plan.files[0].updated;
    assert!(
        source.contains("\tParent: xirangFaceShopPanel,\n"),
        "{source}"
    );
    assert!(
        plan.diagnostics
            .iter()
            .any(|diagnostic| diagnostic.message.contains("`[record] language = \"go\"`")),
        "the plan says which record table key makes the constant exist: {:?}",
        plan.diagnostics
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
    let error = GoLanguage
        .plan(&request)
        .expect_err("an empty module names no constant");
    assert!(error.contains("empty parent module"), "{error}");
}

/// A field the carrier cannot write is refused, naming the field.
/// 载体写不出的字段被拒绝，并点名字段。
#[test]
fn a_field_the_carrier_cannot_write_is_refused_by_name() {
    let mut registration = RegistrationUpdate::new();
    field(
        &mut registration,
        RegistrationField::GettingFromOtherRegistry,
        "other",
    );
    let error = GoLanguage
        .plan(&with(registration))
        .expect_err("the Go carrier does not carry from_registry");
    assert!(error.contains("getting_from_other_registry"), "{error}");
}

/// Changing the kind in place is a rename, not a field edit, so it is refused.
/// 原地改 kind 是重命名而不是字段编辑，因此拒绝。
#[test]
fn changing_the_kind_in_place_is_refused() {
    let source = "var _ = xirang.Soil{\n\tParent: rootNodeID,\n\tNamed: \"Gauge\",\n}\n";
    let mut registration = RegistrationUpdate::new();
    field(&mut registration, RegistrationField::Kind, "Dial");
    let mut request = request(AuthoringOperation::Modify);
    request.registration = registration;
    request.source_context = SourceContext::new().with_existing_source(source);
    let error = GoLanguage
        .plan(&request)
        .expect_err("a kind change is a rename");
    assert!(error.contains("renames the business type"), "{error}");
}

/// The committed example reads back to the declaration its runtime test asserts.
/// 入库的示例读回成它运行期测试所断言的同一条声明。
#[test]
fn the_committed_example_reads_back_to_the_same_declaration() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../examples/go-gauge/gauge");
    let source = std::fs::read_to_string(root.join("go/src/gauge/gauge.go")).expect("the example");
    let declaration = read_declaration(&source).expect("the example's declaration");
    assert_eq!(declaration.kind(), "Gauge");
    assert_eq!(declaration.face.parent, "xirangFacePanel");
    assert_eq!(
        declaration
            .face
            .registration
            .find_field(RegistrationField::Exports)
            .value()
            .copied(),
        Some("gauge")
    );
}

/// The Go toolchain, when this machine has one.
/// 本机有 Go 工具链时返回它。
fn go_tool() -> Option<String> {
    if let Ok(from_env) = std::env::var("XIRANG_GO")
        && std::process::Command::new(&from_env)
            .arg("version")
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    {
        return Some(from_env);
    }
    std::process::Command::new("go")
        .arg("version")
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|_| "go".to_owned())
}

/// The generated source really compiles against the SDK (skipped without a Go toolchain).
/// 生成的源码真的能在 SDK 下编译（没有 Go 工具链时跳过）。
#[test]
fn a_generated_declaration_compiles_against_the_sdk() {
    let Some(go) = go_tool() else {
        eprintln!("no Go toolchain (XIRANG_GO or `go`); the generated source was not compiled");
        return;
    };
    let mut registration = RegistrationUpdate::new();
    field(&mut registration, RegistrationField::Kind, "Gauge");
    field(&mut registration, RegistrationField::Exports, "gauge");
    let plan = GoLanguage.plan(&with(registration)).expect("the plan");
    let directory = std::env::temp_dir().join(format!(
        "xirang-go-authoring-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("the clock")
            .as_nanos()
    ));
    std::fs::create_dir_all(directory.join("src/gauge")).expect("the fixture directory");
    std::fs::write(directory.join("src/gauge/gauge.go"), &plan.files[0].updated)
        .expect("the generated source");
    let hex = xirang_kernel::identity::root_node_id("gauge")
        .into_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    std::fs::write(
        directory.join("src/gauge/faces.go"),
        format!("package gauge\n\nconst rootNodeID = \"{hex}\"\n"),
    )
    .expect("the generated constants");
    std::fs::write(
        directory.join("go.mod"),
        format!(
            "module example.com/generated\n\ngo 1.21\n\nrequire xirang.dev/sdk v0.0.0\n\nreplace \
             xirang.dev/sdk => {}\n",
            crate::sdk::go_include_dir().display()
        ),
    )
    .expect("the module file");
    let output = std::process::Command::new(go)
        .arg("build")
        .arg("./...")
        .current_dir(&directory)
        .env("GOCACHE", directory.join(".cache"))
        .env("GOPATH", directory.join(".path"))
        .output()
        .expect("go runs");
    let _ = std::fs::remove_dir_all(&directory);
    assert!(
        output.status.success(),
        "go refused the generated source:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
