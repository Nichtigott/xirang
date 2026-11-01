//! Pins for the Python adapter: generation, reading, precise rewrite, and a real import.
//! Python 适配器的钉子：生成、读取、精确改写，以及一次真实导入。

use super::*;
use xirang_kernel::interchange::LanguageId;

fn python() -> LanguageId {
    LanguageId::parse_language("python").expect("a language id")
}

fn request(operation: AuthoringOperation) -> FaceAuthoringRequest {
    FaceAuthoringRequest {
        language: python(),
        operation,
        destination: "src/shop/panel/area/gauge.py".to_owned(),
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

/// A create writes the decorator spelling: an ordinary decorator with keyword arguments, and the
/// generated parent expression.
/// 新建写出装饰器拼法：一个普通装饰器加关键字参数，以及生成的父表达式。
#[test]
fn a_create_writes_the_decorator_spelling() {
    let mut registration = RegistrationUpdate::new();
    field(&mut registration, RegistrationField::Kind, "Gauge");
    field(&mut registration, RegistrationField::Exports, "gauge");
    let plan = PythonLanguage.plan(&with(registration)).expect("the plan");
    let source = &plan.files[0].updated;
    assert!(
        source.contains("from xirang import faces, soil"),
        "{source}"
    );
    assert!(source.contains("@soil.face("), "{source}");
    assert!(
        source.contains("    under=faces.shop.panel.area.NODE_ID,"),
        "{source}"
    );
    assert!(source.contains("    named=\"Gauge\","), "{source}");
    assert!(source.contains("    exports=(\"gauge\",),"), "{source}");
    assert!(source.contains("    needs_registry=False,"), "{source}");
    assert!(source.contains("class Gauge:"), "{source}");
}

/// The generated decorator reads back to the same normalized registration.
/// 生成的装饰器能读回同一份规范注册。
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
    let plan = PythonLanguage
        .plan(&with(registration.clone()))
        .expect("the plan");
    let declaration =
        read_declaration(&plan.files[0].updated).expect("the generated source reads back");
    assert_eq!(declaration.class, "Gauge");
    assert_eq!(declaration.kind(), "Gauge");
    assert!(
        registration.agrees_with(&declaration.face.registration),
        "the round trip changed a field:\n{}\n{:?}",
        plan.files[0].updated,
        declaration.face.registration
    );
}

/// A modify replaces only the argument list: the imports, the class body and the comments survive.
/// 修改只替换参数表：导入、类体与注释逐字保留。
#[test]
fn a_modify_touches_only_the_argument_list() {
    let source = "\
from xirang import faces, soil


@soil.face(
    under=faces.shop.panel.area.NODE_ID,
    named=\"Gauge\",
    exports=(\"gauge\",),
    needs_registry=False,
)
class Gauge:
    \"\"\"Business code that must survive an edit.\"\"\"

    def reading(self) -> int:
        return 42


# A trailing comment.
";
    let mut registration = RegistrationUpdate::new();
    field(
        &mut registration,
        RegistrationField::Exports,
        "gauge,reading",
    );
    let mut request = request(AuthoringOperation::Modify);
    request.registration = registration;
    request.source_context =
        xirang_kernel::authoring::SourceContext::new().with_existing_source(source.to_owned());
    let plan = PythonLanguage.plan(&request).expect("the plan");
    let file = &plan.files[0];
    assert_eq!(file.edits.len(), 1, "one precise edit");
    let updated = &file.updated;
    assert!(
        updated.contains("from xirang import faces, soil"),
        "{updated}"
    );
    assert!(updated.contains("def reading(self) -> int:"), "{updated}");
    assert!(updated.contains("# A trailing comment."), "{updated}");
    assert!(
        updated.contains("exports=(\"gauge\", \"reading\")"),
        "{updated}"
    );
    assert!(updated.contains("needs_registry=False"), "{updated}");
    let edit = &file.edits[0];
    assert_eq!(
        format!(
            "{}{}{}",
            &source[..edit.start],
            edit.replacement,
            &source[edit.end..]
        ),
        *updated
    );
}

/// A keyword the adapter does not map is preserved by a rewrite, not dropped.
/// 适配器不映射的关键字在改写时被保留而不是丢掉。
#[test]
fn an_unmapped_keyword_survives_a_modify() {
    let source = "\
@soil.face(
    under=faces.shop.panel.area.NODE_ID,
    named=\"Gauge\",
    future_option=(\"x\", 2),
)
class Gauge:
    pass
";
    let declaration = read_declaration(source).expect("the declaration reads");
    assert_eq!(declaration.face.extras.len(), 1);
    let mut request = request(AuthoringOperation::Modify);
    request.source_context =
        xirang_kernel::authoring::SourceContext::new().with_existing_source(source.to_owned());
    let plan = PythonLanguage.plan(&request).expect("the plan");
    assert!(
        plan.files[0].updated.contains("future_option=(\"x\", 2)"),
        "{}",
        plan.files[0].updated
    );
}

/// A field whose value the decorator cannot carry is refused by name.
/// 装饰器携带不了的字段按名拒绝。
#[test]
fn a_rust_expression_field_is_refused_by_name() {
    let mut registration = RegistrationUpdate::new();
    field(
        &mut registration,
        RegistrationField::RegistrationRule,
        "crate::rule::RULE",
    );
    let error = PythonLanguage.plan(&with(registration)).unwrap_err();
    assert!(error.contains("registration_rule"), "{error}");
    assert!(error.contains("cannot carry"), "{error}");
}

/// The parent must be a generated `faces.<path>.NODE_ID` expression.
/// 父必须是生成的 `faces.<path>.NODE_ID` 表达式。
#[test]
fn a_parent_that_is_not_a_generated_reference_is_refused() {
    let mut request = with(RegistrationUpdate::new());
    request.parent = ParentReference::Named("make_parent()".to_owned());
    let error = PythonLanguage.plan(&request).unwrap_err();
    assert!(
        error.contains("not a generated Python face reference"),
        "{error}"
    );
}

/// A create whose module disagrees with the file name is refused.
/// 模块与文件名不一致的新建被拒绝。
#[test]
fn a_module_that_disagrees_with_the_file_name_is_refused() {
    let mut registration = RegistrationUpdate::new();
    field(&mut registration, RegistrationField::Module, "reading");
    let error = PythonLanguage.plan(&with(registration)).unwrap_err();
    assert!(error.contains("reading"), "{error}");
}

/// The generated file is real Python: it imports the shipped SDK and executes the decorator.
/// 生成的文件是真 Python：它导入随包 SDK 并执行装饰器。
#[test]
fn a_generated_face_imports_against_the_sdk() {
    let Some(python) = interpreter() else {
        eprintln!("no python3 on PATH; the generated source was not executed");
        return;
    };
    let plan = PythonLanguage
        .plan(&with(RegistrationUpdate::new()))
        .expect("the plan");
    let directory = std::env::temp_dir().join(format!(
        "xirang-python-authoring-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("the clock")
            .as_nanos()
    ));
    std::fs::create_dir_all(&directory).expect("the fixture directory");
    let source = directory.join("gauge.py");
    std::fs::write(&source, &plan.files[0].updated).expect("the generated source");
    let output = std::process::Command::new(python)
        .env("PYTHONPATH", crate::sdk::python_include_dir())
        .arg("-c")
        .arg(format!(
            "import runpy; namespace = runpy.run_path({:?}); assert 'Gauge' in namespace",
            source.display().to_string()
        ))
        .output()
        .expect("the interpreter runs");
    let _ = std::fs::remove_dir_all(&directory);
    assert!(
        output.status.success(),
        "the generated source did not import:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// The first Python 3 interpreter on `PATH`, if any.
/// `PATH` 上的第一个 Python 3 解释器（如果有）。
fn interpreter() -> Option<String> {
    for tool in ["python3", "python"] {
        if std::process::Command::new(tool)
            .arg("--version")
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
        {
            return Some(tool.to_owned());
        }
    }
    None
}

/// A rendered declaration reads back to the fields it was rendered from.
/// 渲染出来的声明读回它被渲染时的字段。
///
/// The renderer and the reader go through one grammar table, so this is the pin that a spelling or
/// a value style cannot drift on one side: the text the tool writes is the text it reads.
/// 渲染器与读取器走同一张语法表，因此这条钉子钉住"拼法或取值风格不会只在一侧漂移"：工具写出的文本就是
/// 它读回的文本。
#[test]
fn a_rendered_declaration_reads_back_to_itself() {
    let mut registration = RegistrationUpdate::new();
    field(&mut registration, RegistrationField::Kind, "Gauge");
    field(
        &mut registration,
        RegistrationField::StableName,
        "GaugeFace",
    );
    field(
        &mut registration,
        RegistrationField::Exports,
        "gauge,volume",
    );
    field(&mut registration, RegistrationField::Provides, "cap.gauge");
    field(
        &mut registration,
        RegistrationField::Requires,
        "cap.render=>Provider",
    );
    field(
        &mut registration,
        RegistrationField::Preset,
        "crate::FastPreset",
    );
    field(
        &mut registration,
        RegistrationField::Parts,
        "crate::FastParts",
    );
    field(&mut registration, RegistrationField::NameZh, "面板");
    field(&mut registration, RegistrationField::SummaryEn, "A gauge.");
    field(&mut registration, RegistrationField::NeedsRegistry, "true");
    let plan = PythonLanguage
        .plan(&with(registration.clone()))
        .expect("the plan");
    let source = &plan.files[0].updated;
    let read = read_declaration(source).expect("the generated source reads back");
    assert!(
        registration.satisfies(&read.face.registration),
        "the read-back carries every field the request named:\n{source}\nrequest: {registration:?}\nread: {:?}",
        read.face.registration
    );
    assert_eq!(read.class, "Gauge", "the class is the face kind: {source}");
}
