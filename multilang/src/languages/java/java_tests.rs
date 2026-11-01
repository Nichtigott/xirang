//! Pins for the Java adapter: generation, reading, precise rewrite, and a real `javac`.
//! Java 适配器的钉子：生成、读取、精确改写，以及一次真实 `javac`。

use super::*;
use xirang_kernel::authoring::{
    FaceAuthoringRequest, ParentReference, RegistrationUpdate, SourceContext,
};
use xirang_kernel::interchange::LanguageId;

fn java() -> LanguageId {
    LanguageId::parse_language("java").expect("a language id")
}

fn request(operation: AuthoringOperation) -> FaceAuthoringRequest {
    FaceAuthoringRequest {
        language: java(),
        operation,
        destination: "java/src/gauge/Gauge.java".to_owned(),
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

/// A create writes the annotation spelling: package, imports, class, and the element list.
/// 新建写出注解拼法：包、导入、类与元素表。
#[test]
fn a_create_writes_the_annotation_spelling() {
    let mut registration = RegistrationUpdate::new();
    field(&mut registration, RegistrationField::Kind, "Gauge");
    field(&mut registration, RegistrationField::Exports, "gauge");
    let plan = JavaLanguage.plan(&with(registration)).expect("the plan");
    let source = &plan.files[0].updated;
    assert!(source.contains("package gauge;\n"), "{source}");
    assert!(source.contains("import xirang.Soil;\n"), "{source}");
    assert!(source.contains("import xirang.Faces;\n"), "{source}");
    assert!(source.contains("@Soil(\n"), "{source}");
    assert!(
        source.contains("        parent = Faces.ROOT_NODE_ID,\n"),
        "{source}"
    );
    assert!(source.contains("        named = \"Gauge\",\n"), "{source}");
    assert!(
        source.contains("        exports = {\"gauge\"}\n"),
        "{source}"
    );
    assert!(source.contains("public final class Gauge {\n"), "{source}");
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
    let plan = JavaLanguage
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

/// A modify replaces only the argument list: imports, class body and trailing code survive.
/// 修改只替换参数表：导入、类体与后面的代码逐字保留。
#[test]
fn a_modify_touches_only_the_argument_list() {
    let source = "\
package gauge;

import xirang.Faces;
import xirang.Soil;

@Soil(parent = Faces.ROOT_NODE_ID, named = \"Gauge\", exports = {\"gauge\"})
public final class Gauge {
    /** A business method that must survive an edit. */
    public static int reading() {
        return 42;
    }
}

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
    update.source_context = SourceContext::new().with_existing_source(source);
    let plan = JavaLanguage.plan(&update).expect("the plan");
    let updated = &plan.files[0].updated;
    assert!(
        updated.contains("public static int reading() {\n        return 42;\n    }\n}"),
        "the class body survives:\n{updated}"
    );
    assert!(
        updated.contains("        exports = {\"gauge\", \"reading\"}\n"),
        "the named element is rewritten normalized:\n{updated}"
    );
    assert!(
        updated.ends_with("// A trailing comment.\n"),
        "everything after the declaration survives:\n{updated}"
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
    let plan = JavaLanguage.plan(&request).expect("the parent is nameable");
    let source = &plan.files[0].updated;
    assert!(
        source.contains("        parent = Faces.SHOP_PANEL_NODE_ID,\n"),
        "{source}"
    );
    assert!(
        plan.diagnostics.iter().any(|diagnostic| diagnostic
            .message
            .contains("`[record] language = \"java\"`")),
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
    let error = JavaLanguage
        .plan(&request)
        .expect_err("an empty module names no constant");
    assert!(error.contains("empty parent module"), "{error}");
}

/// Changing the kind in place is a rename, not a field edit, so it is refused.
/// 原地改 kind 是重命名而不是字段编辑，因此拒绝。
#[test]
fn changing_the_kind_in_place_is_refused() {
    let source =
        "@Soil(parent = Faces.ROOT_NODE_ID, named = \"Gauge\")\npublic final class Gauge {}\n";
    let mut registration = RegistrationUpdate::new();
    field(&mut registration, RegistrationField::Kind, "Dial");
    let mut request = request(AuthoringOperation::Modify);
    request.registration = registration;
    request.source_context = SourceContext::new().with_existing_source(source);
    let error = JavaLanguage
        .plan(&request)
        .expect_err("a kind change is a rename");
    assert!(error.contains("renames the class"), "{error}");
}

/// The committed example reads back to the declaration its runtime test asserts.
/// 入库的示例读回成它运行期测试所断言的同一条声明。
#[test]
fn the_committed_example_reads_back_to_the_same_declaration() {
    let root =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../examples/java-gauge/gauge");
    let source =
        std::fs::read_to_string(root.join("java/src/gauge/Gauge.java")).expect("the example");
    let declaration = read_declaration(&source).expect("the example's declaration");
    assert_eq!(declaration.kind(), "Gauge");
    assert_eq!(declaration.face.parent, "Faces.PANEL_NODE_ID");
}

/// The Java toolchain, when this machine has one.
/// 本机有 Java 工具链时返回它。
fn javac() -> Option<String> {
    if let Ok(from_env) = std::env::var("XIRANG_JAVA")
        && std::process::Command::new(&from_env)
            .arg("-version")
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    {
        return Some(from_env);
    }
    std::process::Command::new("javac")
        .arg("-version")
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|_| "javac".to_owned())
}

/// The generated source really compiles against the SDK and the generated constants (skipped
/// without a JDK).
/// 生成的源码真的能在 SDK 与生成的常量下编译（没有 JDK 时跳过）。
#[test]
fn a_generated_declaration_compiles_against_the_sdk() {
    let Some(javac) = javac() else {
        eprintln!("no JDK (XIRANG_JAVA or `javac`); the generated source was not compiled");
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
    let plan = JavaLanguage.plan(&request).expect("the plan");
    let directory = std::env::temp_dir().join(format!(
        "xirang-java-authoring-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("the clock")
            .as_nanos()
    ));
    let classes = directory.join("classes");
    let generated = directory.join("gen/xirang");
    std::fs::create_dir_all(directory.join("src/gauge")).expect("the fixture directory");
    std::fs::create_dir_all(&generated).expect("the generated package");
    std::fs::write(
        directory.join("src/gauge/Gauge.java"),
        &plan.files[0].updated,
    )
    .expect("the generated source");
    let hex = "cff196f1773f4ab1f1c49314821f99ea";
    std::fs::write(
        generated.join("Faces.java"),
        format!(
            "package xirang;\n\npublic final class Faces {{\n    public static final String \
             ROOT_NODE_ID = \"d50df7e06896ebbf0b0912f3603dbc67\";\n    public static final String \
             PANEL_NODE_ID = \"{hex}\";\n    private Faces() {{}}\n}}\n"
        ),
    )
    .expect("the generated constants");
    let output = std::process::Command::new(&javac)
        .arg("-d")
        .arg(&classes)
        .args(
            std::fs::read_dir(crate::sdk::java_include_dir().join("xirang"))
                .expect("the SDK")
                .filter_map(|entry| entry.ok().map(|entry| entry.path()))
                .filter(|path| {
                    path.extension().and_then(|extension| extension.to_str()) == Some("java")
                }),
        )
        .arg(generated.join("Faces.java"))
        .arg(directory.join("src/gauge/Gauge.java"))
        .output()
        .expect("javac runs");
    let _ = std::fs::remove_dir_all(&directory);
    assert!(
        output.status.success(),
        "javac refused the generated source:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
