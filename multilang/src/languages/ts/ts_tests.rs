//! Pins for the TypeScript adapter: generation, reading, precise rewrite, and a real import.
//! TypeScript 适配器的钉子：生成、读取、精确改写，以及一次真实导入。

use super::*;
use xirang_kernel::authoring::{
    FaceAuthoringRequest, FieldUpdate, ParentReference, RegistrationField, RegistrationUpdate,
};

fn typescript() -> LanguageId {
    LanguageId::parse_language("typescript").expect("a language id")
}

fn request(operation: AuthoringOperation) -> FaceAuthoringRequest {
    FaceAuthoringRequest {
        language: typescript(),
        operation,
        destination: "src/shop/panel/area/gauge.ts".to_owned(),
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

/// A create writes the same call spelling the JavaScript carrier has, for a `.ts` file.
/// 新建为 `.ts` 文件写出与 JavaScript 载体相同的调用拼法。
#[test]
fn a_create_writes_the_typed_call_spelling() {
    let mut registration = RegistrationUpdate::new();
    field(&mut registration, RegistrationField::Kind, "Gauge");
    field(&mut registration, RegistrationField::Exports, "gauge");
    let plan = TsLanguage.plan(&with(registration)).expect("the plan");
    assert_eq!(plan.language.as_str(), "typescript");
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
    assert!(source.contains("export class Gauge {"), "{source}");
    assert!(
        plan.diagnostics
            .iter()
            .any(|diagnostic| diagnostic.message.contains("sdk/ts")),
        "the plan names the TypeScript SDK: {:?}",
        plan.diagnostics
    );
}

/// The generated declaration reads back to the same normalized registration.
/// 生成的声明能读回同一份规范注册。
#[test]
fn a_generated_declaration_reads_back_to_the_same_fields() {
    let mut registration = RegistrationUpdate::new();
    for (field_, text) in [
        (RegistrationField::Kind, "Gauge"),
        (RegistrationField::Exports, "gauge,reading"),
        (RegistrationField::Provides, "geometry"),
        (RegistrationField::Requires, "clock=>SystemClock"),
        (RegistrationField::NeedsRegistry, "true"),
    ] {
        field(&mut registration, field_, text);
    }
    let plan = TsLanguage
        .plan(&with(registration.clone()))
        .expect("the plan");
    let declaration =
        js::read_declaration(&plan.files[0].updated).expect("the generated source reads back");
    assert!(
        registration.agrees_with(&declaration.face.registration),
        "the round trip changed a field:\n{}\n{:?}",
        plan.files[0].updated,
        declaration.face.registration
    );
}

/// A modify replaces only the object body: type annotations and everything else survive.
/// 修改只替换对象体：类型标注与其它一切逐字保留。
#[test]
fn a_modify_keeps_the_type_annotations_around_the_call() {
    let source = "\
import { faces, soil } from \"xirang\";

/** Business code that must survive an edit. */
export class Gauge {
    reading(): number {
        return 42;
    }
}

soil.face({
  under: faces.shop.panel.area.NODE_ID,
  named: \"Gauge\",
  exports: [\"gauge\"],
  needs_registry: false,
});

const width: number = 3;
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
    let plan = TsLanguage.plan(&update).expect("the plan");
    let updated = &plan.files[0].updated;
    assert!(
        updated.contains("reading(): number {"),
        "the return annotation survives:\n{updated}"
    );
    assert!(
        updated.ends_with("const width: number = 3;\n"),
        "the trailing typed declaration survives:\n{updated}"
    );
    assert!(
        updated.contains("exports: [\"gauge\", \"reading\"],"),
        "{updated}"
    );
    assert_eq!(plan.files[0].edits.len(), 1, "one precise edit");
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

/// The generated `.ts` source really imports and registers against the typed SDK.
/// 生成的 `.ts` 源码真的能在带类型的 SDK 下导入并登记声明。
#[test]
fn a_generated_face_imports_against_the_typed_sdk() {
    let Some(node) = node() else {
        eprintln!("no node on PATH; the generated source was not executed");
        return;
    };
    let mut registration = RegistrationUpdate::new();
    field(&mut registration, RegistrationField::Kind, "Gauge");
    field(&mut registration, RegistrationField::Exports, "gauge");
    let plan = TsLanguage.plan(&with(registration)).expect("the plan");
    let directory = std::env::temp_dir().join(format!(
        "xirang-typescript-authoring-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("the clock")
            .as_nanos()
    ));
    std::fs::create_dir_all(directory.join("node_modules")).expect("the fixture directory");
    std::fs::write(directory.join("gauge.ts"), &plan.files[0].updated)
        .expect("the generated source");
    std::os::unix::fs::symlink(
        crate::sdk::ts_include_dir().join("xirang"),
        directory.join("node_modules/xirang"),
    )
    .expect("the SDK link");
    std::fs::write(
        directory.join("check.ts"),
        "import \"./gauge.ts\";\n\
         import { declarations } from \"xirang\";\n\
         const faces = declarations();\n\
         if (faces.length !== 1) throw new Error(`one declaration, got ${faces.length}`);\n\
         console.log(\"registered\");\n",
    )
    .expect("the check script");
    let output = std::process::Command::new(node)
        .arg("check.ts")
        .current_dir(&directory)
        .output()
        .expect("node runs");
    let _ = std::fs::remove_dir_all(&directory);
    assert!(
        output.status.success(),
        "node refused the generated TypeScript source:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("registered"),
        "the declaration did not register: {}",
        String::from_utf8_lossy(&output.stdout)
    );
}

/// The committed TypeScript example reads back to the declaration its test asserts at import time.
/// 入库的 TypeScript 示例读回成它导入期测试所断言的同一条声明。
#[test]
fn the_committed_example_reads_back_to_the_same_declaration() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../examples/ts-gauge");
    let faces = faces::read_declarations(&root).expect("the example's declarations");
    let found: Vec<_> = faces
        .iter()
        .filter(|face| face.source == "src/gauge.ts")
        .collect();
    assert_eq!(found.len(), 1, "one declaration in the example: {faces:?}");
    assert_eq!(found[0].name, "Gauge");
    assert_eq!(found[0].parent, "root");
    assert_eq!(found[0].exports, vec!["gauge".to_owned()]);
    assert!(!found[0].owns_registry);
}
