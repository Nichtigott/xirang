//! Pins for the C++ adapter: generation, reading, precise rewrite, compilation and refusal.
//! C++ 适配器的钉子：生成、读取、精确改写、编译与拒绝。

use super::*;
use xirang_kernel::interchange::LanguageId;

fn cpp() -> LanguageId {
    LanguageId::parse_language("cpp").expect("a language id")
}

fn request(operation: AuthoringOperation) -> FaceAuthoringRequest {
    FaceAuthoringRequest {
        language: cpp(),
        operation,
        destination: "src/shop/panel/area/area.cpp".to_owned(),
        parent: ParentReference::Named("::xirang::faces::shop::panel::NODE_ID".to_owned()),
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

/// The generated declaration is the target spelling: a marker, an ordinary `constexpr auto SOIL`,
/// and a builder chain that names the generated parent constant.
/// 生成的声明是目标拼法：一个标记、一个普通 `constexpr auto SOIL`，以及点名生成父常量的构造器链。
/// A catalog parent is rendered as the shared `crate::catalog::…` spelling, not the old face header.
/// 目录父级渲染为共享的 `crate::catalog::…` 拼法，而不是旧 face 头。
#[test]
fn a_catalog_parent_renders_the_shared_expression() {
    let mut registration = RegistrationUpdate::new();
    field(&mut registration, RegistrationField::Kind, "Gauge");
    let mut request = with(registration);
    request.parent = ParentReference::Catalog {
        id: xirang_kernel::identity::NodeId::from_raw([0; 16]),
        path: "shop/panel/Area".to_owned(),
    };
    let plan = CppLanguage.plan(&request).expect("the plan");
    let source = &plan.files[0].updated;
    assert!(
        source.contains(".under(crate::catalog::shop::panel::Area::NODE_ID)"),
        "{source}"
    );
}

#[test]
fn a_create_writes_the_marked_spelling_and_no_record_variable() {
    let mut registration = RegistrationUpdate::new();
    field(&mut registration, RegistrationField::Kind, "Gauge");
    field(&mut registration, RegistrationField::Exports, "gauge");
    field(&mut registration, RegistrationField::NeedsRegistry, "false");
    let plan = CppLanguage.plan(&with(registration)).expect("the plan");
    let source = &plan.files[0].updated;
    assert!(
        source.contains("XIRANG_SOIL\nconstexpr auto SOIL = ::xirang::soil<Gauge>()"),
        "{source}"
    );
    assert!(
        source.contains(".under(::xirang::faces::shop::panel::NODE_ID)"),
        "{source}"
    );
    assert!(source.contains(".named(\"Gauge\")"), "{source}");
    assert!(source.contains(".exports(\"gauge\")"), "{source}");
    assert!(source.contains(".needs_registry(false)"), "{source}");
    assert!(
        !source.contains("XIRANG_FACE("),
        "a create must not write the version-1 spelling: {source}"
    );
    assert!(
        !source.contains("r_gauge"),
        "a create must not name a record variable: {source}"
    );
}

/// The generated declaration reads back to the same normalized registration, which is the round trip
/// the acceptance contract asks for.
/// 生成的声明能读回同一份规范注册——这正是验收契约要求的往返。
#[test]
fn a_generated_declaration_reads_back_to_the_same_fields() {
    let mut registration = RegistrationUpdate::new();
    for (field_, text) in [
        (RegistrationField::Kind, "Area"),
        (RegistrationField::StableName, "stable_area"),
        (RegistrationField::Exports, "area,volume"),
        (RegistrationField::Provides, "geometry"),
        (RegistrationField::Requires, "clock=>SystemClock"),
        (RegistrationField::Preset, "crate::panel::Preset"),
        (RegistrationField::Parts, "crate::panel::Parts"),
        (RegistrationField::NameZh, "区域"),
        (RegistrationField::NameEn, "Area"),
        (RegistrationField::SummaryZh, "一块区域"),
        (RegistrationField::SummaryEn, "one area"),
        (RegistrationField::GettingFromOtherRegistry, "crate::other"),
        (RegistrationField::NeedsRegistry, "true"),
    ] {
        field(&mut registration, field_, text);
    }
    let plan = CppLanguage
        .plan(&with(registration.clone()))
        .expect("the plan");
    let source = &plan.files[0].updated;
    let declaration = read_declaration(source).expect("the generated source reads back");
    assert_eq!(declaration.spelling, Spelling::Marked);
    assert_eq!(declaration.kind(), "Area");
    assert_eq!(declaration.business_type, "Area");
    assert_eq!(
        declaration.face.parent,
        "::xirang::faces::shop::panel::NODE_ID"
    );
    assert!(
        registration.agrees_with(&declaration.face.registration),
        "the round trip changed a field:\n{source}\n{:?}",
        declaration.face.registration
    );
}

/// A create plan names the file it creates, has no precise edits, and states the checks it owes.
/// 新建计划点名它创建的文件、没有精确编辑，并列出它欠下的检查。
#[test]
fn a_create_plan_creates_one_file_and_names_its_obligations() {
    let plan = CppLanguage
        .plan(&with(RegistrationUpdate::new()))
        .expect("the plan");
    assert_eq!(plan.files.len(), 1);
    assert!(plan.files[0].creates());
    assert!(plan.files[0].edits.is_empty());
    let kinds: Vec<&str> = plan
        .obligations
        .iter()
        .map(|obligation| obligation.kind.as_str())
        .collect();
    assert!(kinds.contains(&"round-trip"), "{kinds:?}");
    assert!(kinds.contains(&"build"), "{kinds:?}");
    assert!(kinds.contains(&"registration"), "{kinds:?}");
    assert!(kinds.contains(&"identity"), "{kinds:?}");
}

/// A modify of a marked declaration changes only the expression and keeps the marker, the variable
/// name and the business code byte for byte.
/// 对带标记声明的修改只动表达式，标记、变量名与业务代码逐字节保持。
#[test]
fn a_modify_touches_only_the_expression_and_keeps_the_spelling() {
    let source = "\
// The area implementation lives here and must survive an edit.\n\
struct Area { int width = 0; };\n\
\n\
XIRANG_SOIL\n\
constexpr auto SOIL = ::xirang::soil<Area>()\n\
    .under(::xirang::faces::shop::panel::NODE_ID)\n\
    .named(\"Area\")\n\
    .exports(\"area\")\n\
    .needs_registry(false);\n\
\n\
// A trailing comment that is not part of the declaration.\n\
int area(int w) { return w; }\n";
    let mut registration = RegistrationUpdate::new();
    field(&mut registration, RegistrationField::Exports, "area,volume");
    let mut request = request(AuthoringOperation::Modify);
    request.registration = registration;
    request.source_context =
        xirang_kernel::authoring::SourceContext::new().with_existing_source(source.to_owned());
    let plan = CppLanguage.plan(&request).expect("the plan");
    let file = &plan.files[0];
    assert!(file.changes());
    assert_eq!(file.edits.len(), 1, "one precise edit");
    let updated = &file.updated;
    assert!(
        updated.contains("XIRANG_SOIL\nconstexpr auto SOIL ="),
        "{updated}"
    );
    assert!(
        updated.contains("// The area implementation lives here"),
        "{updated}"
    );
    assert!(
        updated.contains("struct Area { int width = 0; };"),
        "{updated}"
    );
    assert!(updated.contains("// A trailing comment"), "{updated}");
    assert!(
        updated.contains("int area(int w) { return w; }"),
        "{updated}"
    );
    assert!(updated.contains(".exports(\"area,volume\")"), "{updated}");
    let edit = &file.edits[0];
    assert!(
        source[edit.start..edit.end].starts_with("::xirang::soil"),
        "{:?}",
        &source[edit.start..edit.end]
    );
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

/// A version-1 declaration is still read, and a modify keeps it in that spelling: turning it into the
/// marked one is a migration, not a field edit.
/// 版本 1 声明仍然读取，修改保留那种拼法：把它变成带标记的拼法是一次迁移，不是字段编辑。
#[test]
fn a_version_one_declaration_is_read_and_modified_in_place() {
    let source = "\
XIRANG_FACE(r_area, (::xirang::soil<Area>()\n\
                         .under(::xirang::faces::shop__panel_Panel)\n\
                         .named(\"Area\")\n\
                         .exports(\"area\").owns_registry()));\n";
    let declaration = read_declaration(source).expect("the version-1 spelling reads");
    assert_eq!(declaration.spelling, Spelling::Compat);
    assert_eq!(declaration.variable, "r_area");
    assert_eq!(declaration.kind(), "Area");
    let mut registration = RegistrationUpdate::new();
    field(&mut registration, RegistrationField::Exports, "area,volume");
    let mut request = request(AuthoringOperation::Modify);
    request.parent = ParentReference::Named("::xirang::faces::shop__panel_Panel".to_owned());
    request.registration = registration;
    request.source_context =
        xirang_kernel::authoring::SourceContext::new().with_existing_source(source.to_owned());
    let plan = CppLanguage.plan(&request).expect("the plan");
    let updated = &plan.files[0].updated;
    assert!(
        updated.starts_with("XIRANG_FACE(r_area, (::xirang::soil<Area>()"),
        "{updated}"
    );
    assert!(updated.contains(".exports(\"area,volume\")"), "{updated}");
    assert!(
        !updated.contains("XIRANG_SOIL"),
        "a modify must not migrate the spelling: {updated}"
    );
    assert!(
        read_declaration(updated)
            .expect("the result reads")
            .face
            .registration
            .find_field(RegistrationField::NeedsRegistry)
            == FieldUpdate::Set("true"),
        "the field the request did not name was lost: {updated}"
    );
}

/// A method the standard authoring vocabulary has no field for is preserved by a rewrite, not
/// dropped: the vocabulary's gap must not become data loss.
/// 标准创作词汇里没有对应字段的方法在改写时被保留而不是丢掉：词汇的缺口不能变成数据丢失。
#[test]
fn a_field_outside_the_authoring_vocabulary_survives_a_modify() {
    let source = "\
XIRANG_SOIL\nconstexpr auto SOIL = ::xirang::soil<Area>()\n    .under(::xirang::faces::shop::panel::NODE_ID)\n    .named(\"Area\")\n    .params(\"crate::panel::Params\")\n    .handle(\"crate::panel::Handle\")\n    .needs_registry(false);\n";
    let declaration = read_declaration(source).expect("the declaration reads");
    assert_eq!(
        declaration.face.extras.len(),
        2,
        "{:?}",
        declaration.face.extras
    );
    let mut request = request(AuthoringOperation::Modify);
    request.source_context =
        xirang_kernel::authoring::SourceContext::new().with_existing_source(source.to_owned());
    let plan = CppLanguage.plan(&request).expect("the plan");
    let updated = &plan.files[0].updated;
    assert!(
        updated.contains(".params(\"crate::panel::Params\")"),
        "{updated}"
    );
    assert!(
        updated.contains(".handle(\"crate::panel::Handle\")"),
        "{updated}"
    );
}

/// A request that changes nothing produces a plan with no edit and says so, instead of rewriting an
/// identical file.
/// 什么都不改的请求产出没有编辑的计划并说出来，而不是重写一份相同的文件。
#[test]
fn a_no_op_modify_does_not_write() {
    let created = CppLanguage
        .plan(&with(RegistrationUpdate::new()))
        .expect("the plan");
    let source = created.files[0].updated.clone();
    let mut request = request(AuthoringOperation::Modify);
    request.source_context =
        xirang_kernel::authoring::SourceContext::new().with_existing_source(source);
    let plan = CppLanguage.plan(&request).expect("the plan");
    assert!(plan.writes_nothing());
    assert!(
        plan.diagnostics
            .iter()
            .any(|diagnostic| diagnostic.message.contains("changes nothing"))
    );
}

/// Renaming the kind is refused by name: it would rename the business type and re-key the identity,
/// which is a structural operation, not a field edit.
/// 改 kind 按名拒绝：它会重命名业务类型并换掉身份，属于结构操作而不是字段编辑。
#[test]
fn changing_the_kind_is_refused_as_a_rename() {
    let source = CppLanguage
        .plan(&with(RegistrationUpdate::new()))
        .expect("the plan")
        .files[0]
        .updated
        .clone();
    let mut registration = RegistrationUpdate::new();
    field(&mut registration, RegistrationField::Kind, "Gauge");
    let mut request = request(AuthoringOperation::Modify);
    request.registration = registration;
    request.source_context =
        xirang_kernel::authoring::SourceContext::new().with_existing_source(source);
    let error = CppLanguage.plan(&request).unwrap_err();
    assert!(error.contains("rename"), "{error}");
    assert!(error.contains("Gauge"), "{error}");
}

/// A review-probe regression: a comment that mentions `XIRANG_SOIL` before an old declaration is
/// not a new declaration. The file is legal C++ and the adapter must still choose the compat
/// spelling.
/// 复核探针回归：旧声明前提到 `XIRANG_SOIL` 的注释不是新声明。文件是合法 C++，适配器必须仍选旧拼法。
#[test]
fn a_comment_mentioning_the_marked_spelling_does_not_change_the_spelling() {
    let source = "\
// Future migration: XIRANG_SOIL
XIRANG_FACE(r_area, (::xirang::soil<Area>()
                         .under(::xirang::faces::shop__panel_Panel)
                         .named(\"Area\")
                         .exports(\"area\")));
";
    let declaration = read_declaration(source).expect("the old declaration reads");
    assert_eq!(declaration.spelling, Spelling::Compat);
    assert_eq!(declaration.kind(), "Area");
    assert_eq!(
        declaration.face.parent,
        "::xirang::faces::shop__panel_Panel"
    );
}

/// A review-probe regression: legal whitespace between `constexpr`, `auto` and `SOIL` reads back.
/// 复核探针回归：`constexpr`、`auto`、`SOIL` 之间的合法空白能读回。
#[test]
fn legal_whitespace_between_the_marked_keywords_reads_back() {
    let source = "\
XIRANG_SOIL
constexpr  auto
SOIL = ::xirang::soil<Gauge>()
    .under(::xirang::faces::shop::panel::NODE_ID)
    .named(\"Gauge\")
    .needs_registry(false);
";
    let declaration = read_declaration(source).expect("the marked declaration reads");
    assert_eq!(declaration.spelling, Spelling::Marked);
    assert_eq!(declaration.kind(), "Gauge");
}

/// A method outside the authoring vocabulary with a literal argument is preserved; a precise edit
/// re-emits it instead of dropping it. A non-literal argument cannot be re-emitted safely and is
/// refused by name.
/// 创作词汇之外的、参数是字面量的方法被保留；精确编辑会重新写出它而不是丢掉。参数不是字面量时无法
/// 安全重写，按名拒绝。
#[test]
fn an_unmapped_call_with_a_literal_is_preserved_and_a_non_literal_is_refused() {
    let source = "XIRANG_SOIL\nconstexpr auto SOIL = ::xirang::soil<Area>().under(P).named(\"Area\").future(\"x\");\n";
    let declaration = read_declaration(source).expect("an unknown literal call is preserved");
    assert_eq!(
        declaration.face.extras,
        vec![("future".to_owned(), "x".to_owned())]
    );
    let mut request = request(AuthoringOperation::Modify);
    request.source_context =
        xirang_kernel::authoring::SourceContext::new().with_existing_source(source.to_owned());
    let plan = CppLanguage.plan(&request).expect("the plan");
    assert!(
        plan.files[0].updated.contains(".future(\"x\")"),
        "{}",
        plan.files[0].updated
    );

    let non_literal = "XIRANG_SOIL\nconstexpr auto SOIL = ::xirang::soil<Area>().under(P).named(\"Area\").future(make());\n";
    let error = read_declaration(non_literal).unwrap_err();
    assert!(error.contains("not a string literal"), "{error}");
}

/// A field whose value the C++ record cannot carry is refused by name, so an author never gets a plan
/// that looks like it declared a registry rule.
/// C++ 记录携带不了的字段按名拒绝，因此作者永远不会得到一份看起来声明了注册规则的计划。
#[test]
fn a_rust_expression_field_is_refused_by_name() {
    let mut registration = RegistrationUpdate::new();
    field(
        &mut registration,
        RegistrationField::RegistrationRule,
        "crate::rule::RULE",
    );
    let error = CppLanguage.plan(&with(registration)).unwrap_err();
    assert!(error.contains("registration_rule"), "{error}");
    assert!(error.contains("cannot carry"), "{error}");
}

/// The parent must be a generated constant, not an arbitrary C++ expression: a precise rewrite of the
/// chain cannot phrase anything else, and accepting one would make the declaration a code channel.
/// 父必须是生成的常量，而不是任意 C++ 表达式：整条链的精确改写写不出别的东西，接受它就等于把声明变成代码通道。
#[test]
fn a_parent_that_is_not_a_generated_reference_is_refused() {
    let mut request = with(RegistrationUpdate::new());
    request.parent = ParentReference::Named("::some::other::CONSTANT".to_owned());
    let error = CppLanguage.plan(&request).unwrap_err();
    assert!(
        error.contains("not a generated C++ face reference"),
        "{error}"
    );
}

/// The root is one generated constant, emitted from the package namespace; a C++ face hangs under
/// it exactly like any other parent and writes no bytes by hand.
/// 根就是一个生成常量，由包命名空间发射；C++ 面像挂在其它父级下一样挂在它下面，不手写任何字节。
#[test]
fn the_root_parent_uses_the_generated_root_constant() {
    let mut request = with(RegistrationUpdate::new());
    request.parent = ParentReference::Root;
    let planned = CppLanguage
        .plan(&request)
        .expect("a C++ face can hang under the root");
    let text = planned
        .files
        .iter()
        .map(|file| file.updated.clone())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        text.contains("::xirang::faces::ROOT_NODE_ID"),
        "the declaration names the generated root constant:\n{text}"
    );
}

/// A create request that names a module different from its destination is refused: the file name is
/// the module, and two answers would make the identity and the path disagree.
/// 新建请求点名的模块与目的地不同会被拒绝：文件名就是模块，两个答案会让身份与路径互相矛盾。
#[test]
fn a_module_that_disagrees_with_the_file_name_is_refused() {
    let mut registration = RegistrationUpdate::new();
    field(&mut registration, RegistrationField::Module, "gauge");
    let error = CppLanguage.plan(&with(registration)).unwrap_err();
    assert!(error.contains("gauge"), "{error}");
    assert!(error.contains("file name is"), "{error}");
}

/// The generated file is real C++: it compiles against the shipped SDK and a generated parent
/// constant. Skipped only when this machine has no C++ compiler.
/// 生成的文件是真 C++：它对着随包 SDK 与生成的父常量编译通过。仅在本机没有 C++ 编译器时跳过。
#[test]
fn a_generated_face_compiles_against_the_sdk() {
    let Some(compiler) = compiler() else {
        eprintln!("no C++ compiler on PATH; the generated source was not compiled");
        return;
    };
    let plan = CppLanguage
        .plan(&with(RegistrationUpdate::new()))
        .expect("the plan");
    let directory = scratch("compile");
    let source = directory.join("area.cpp");
    std::fs::write(&source, &plan.files[0].updated).expect("the generated source");
    std::fs::create_dir_all(directory.join("xirang")).expect("the header directory");
    std::fs::write(
        directory.join("xirang/faces.hpp"),
        "#pragma once\n#include \"xirang/soil.hpp\"\nnamespace xirang::faces::shop::panel {\ninline constexpr ::xirang::Id NODE_ID = ::xirang::Id{{0,1,2,3,4,5,6,7,8,9,10,11,12,13,14,15}};\n}\n",
    )
    .expect("the parent header");
    let output = std::process::Command::new(compiler)
        .arg("-std=c++20")
        .arg(format!("-I{}", crate::sdk::cpp_include_dir().display()))
        .arg(format!("-I{}", directory.display()))
        .arg("-c")
        .arg(&source)
        .arg("-o")
        .arg(directory.join("area.o"))
        .output()
        .expect("the compiler runs");
    assert!(
        output.status.success(),
        "the generated source did not compile:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    // The record really landed in the object, at the length the reader walks.
    // 记录真的落进了目标文件，长度就是读取器所走的那个。
    if let Some(bytes) = section(&directory.join("area.o")) {
        assert_eq!(bytes.len(), 1100, "one marked declaration is one record");
        assert_eq!(&bytes[..4], b"XFRC");
        assert_eq!(u16::from_le_bytes([bytes[8], bytes[9]]), 1100);
        assert_eq!(&bytes[28..33], b"Area\0");
    }
    let _ = std::fs::remove_dir_all(&directory);
}

/// The two review probes are legal C++ **and** read back: a comment before the old spelling, and
/// extra whitespace in the marked one. The test compiles each source first, so a source that only
/// looks readable is not enough.
/// 两条复核探针既是合法 C++ **又**能读回：旧拼法前的注释，以及带标记拼法里的额外空白。测试先编译每份
/// 源码，因此"看起来能读"不算数。
#[test]
fn the_review_probes_compile_and_read_back() {
    let Some(compiler) = compiler() else {
        eprintln!(
            "NOT_RUN: no C++ compiler on PATH; the review probes were read textually but not compiled"
        );
        return;
    };
    let probes = [
        (
            "comment",
            "\n#include <xirang/soil.hpp>\n\
             struct Probe {};\n\
             namespace xirang::faces::shop { inline constexpr xirang::Id NODE_ID{{1}}; }\n\
             // Future migration: XIRANG_SOIL\n\
             XIRANG_FACE(r_probe, (::xirang::soil<Probe>().under(::xirang::faces::shop::NODE_ID).named(\"Probe\")));\n",
            Spelling::Compat,
        ),
        (
            "whitespace",
            "\n#include <xirang/soil.hpp>\n\
             struct Probe {};\n\
             namespace xirang::faces::shop { inline constexpr xirang::Id NODE_ID{{1}}; }\n\
             XIRANG_SOIL\n\
             constexpr  auto SOIL = ::xirang::soil<Probe>().under(::xirang::faces::shop::NODE_ID).named(\"Probe\");\n",
            Spelling::Marked,
        ),
    ];
    for (label, source, spelling) in probes {
        let directory = scratch(&format!("probe-{label}"));
        let file = directory.join(format!("{label}.cpp"));
        std::fs::write(&file, source).expect("the probe source");
        let output = std::process::Command::new(&compiler)
            .arg("-std=c++20")
            .arg(format!("-I{}", crate::sdk::cpp_include_dir().display()))
            .arg("-c")
            .arg(&file)
            .arg("-o")
            .arg(directory.join(format!("{label}.o")))
            .output()
            .expect("the compiler runs");
        assert!(
            output.status.success(),
            "the {label} probe did not compile:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let declaration = read_declaration(source).expect("the probe reads back");
        assert_eq!(declaration.spelling, spelling, "{label}");
        assert_eq!(declaration.kind(), "Probe", "{label}");
        let _ = std::fs::remove_dir_all(&directory);
    }
}

/// The version-1 and the marked spelling of the **same** declaration produce the same record bytes:
/// the spelling changed, the identity, parent and fields did not.
/// 同一份声明的版本 1 拼法与带标记拼法产出同样的记录字节：拼法变了，身份、父与字段没变。
#[test]
fn the_two_spellings_produce_the_same_record() {
    let (Some(compiler), Some(objcopy)) = (compiler(), objcopy()) else {
        eprintln!("no C++ compiler or objcopy on PATH; the equivalence was not exercised");
        return;
    };
    let marked = "XIRANG_SOIL\nconstexpr auto SOIL = ::xirang::soil<Gauge>()\n    .under(::xirang::faces::shop::panel::NODE_ID)\n    .named(\"Gauge\")\n    .exports(\"gauge\")\n    .needs_registry(true);\n";
    let compat = "XIRANG_FACE(r_gauge, (::xirang::soil<Gauge>()\n    .under(::xirang::faces::shop::panel::NODE_ID)\n    .named(\"Gauge\")\n    .exports(\"gauge\")\n    .owns_registry()));\n";
    let mut records = Vec::new();
    for (label, body) in [("marked", marked), ("compat", compat)] {
        let directory = scratch(label);
        std::fs::write(
            directory.join("gauge.cpp"),
            format!(
                "#include \"xirang/soil.hpp\"\nstruct Gauge {{}};\nnamespace xirang::faces::shop::panel {{\ninline constexpr ::xirang::Id NODE_ID = ::xirang::Id{{0,1,2,3,4,5,6,7,8,9,10,11,12,13,14,15}};\n}}\n{body}"
            ),
        )
        .expect("the fixture source");
        let output = std::process::Command::new(&compiler)
            .arg("-std=c++20")
            .arg(format!("-I{}", crate::sdk::cpp_include_dir().display()))
            .arg("-c")
            .arg(directory.join("gauge.cpp"))
            .arg("-o")
            .arg(directory.join("gauge.o"))
            .output()
            .expect("the compiler runs");
        assert!(
            output.status.success(),
            "{label} did not compile:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let bytes =
            section_with(&objcopy, &directory.join("gauge.o")).expect("the section is there");
        records.push(bytes);
        let _ = std::fs::remove_dir_all(&directory);
    }
    assert_eq!(
        records[0], records[1],
        "the two spellings disagree about the record"
    );
}

fn scratch(label: &str) -> std::path::PathBuf {
    let directory = std::env::temp_dir().join(format!(
        "xirang-cpp-authoring-{}-{label}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(&directory).expect("the fixture directory");
    directory
}

/// The `xirang.faces` section of an object, when an `objcopy` is on `PATH`.
/// 一个目标文件的 `xirang.faces` 段（当 `PATH` 上有 `objcopy` 时）。
fn section(object: &std::path::Path) -> Option<Vec<u8>> {
    objcopy().and_then(|tool| section_with(&tool, object))
}

fn section_with(tool: &str, object: &std::path::Path) -> Option<Vec<u8>> {
    let dumped = object.with_extension("section");
    let output = std::process::Command::new(tool)
        .arg("--dump-section")
        .arg(format!("xirang.faces={}", dumped.display()))
        .arg(object)
        .arg(object.with_extension("scratch"))
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    std::fs::read(&dumped).ok()
}

/// The first C++ compiler on `PATH`, if any.
/// `PATH` 上的第一个 C++ 编译器（如果有）。
fn compiler() -> Option<String> {
    for tool in ["g++", "clang++"] {
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

/// The first section-copying program on `PATH`, if any.
/// `PATH` 上的第一个能拷段的程序（如果有）。
fn objcopy() -> Option<String> {
    for tool in ["objcopy", "llvm-objcopy"] {
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

/// A structural move that changes the parent asks the adapter for the language's spelling, not a
/// Rust `crate::…` path. The edit covers exactly the `.under(…)` argument.
/// 改变父级的结构搬动向适配器要该语言的拼写，而不是 Rust `crate::…`。改写恰好覆盖 `.under(…)` 参数。
#[test]
fn a_move_plan_rewrites_the_under_argument_in_cpp_spelling() {
    let source = "XIRANG_SOIL\nconstexpr auto SOIL = ::xirang::soil<Area>()\n    .under(::xirang::faces::old::NODE_ID)\n    .named(\"Area\");\n";
    let edits = CppLanguage
        .plan_move(&crate::registration::MoveRequest {
            source,
            old_module: "native::area",
            new_module: "native::panel::area",
            old_parent: Some("old"),
            new_parent: Some("shop::panel"),
        })
        .expect("the C++ adapter plans the parent rewrite");
    assert_eq!(edits.len(), 1);
    let edit = &edits[0];
    assert_eq!(
        &source[edit.start..edit.end],
        "::xirang::faces::old::NODE_ID"
    );
    assert_eq!(edit.replacement, "::xirang::faces::shop::panel::NODE_ID");
}

/// The version-1 spelling plans the same edit: the adapter locates the `.under(...)` argument in
/// either accepted form, so a move does not silently work only for the marked spelling.
/// 版本 1 拼法规划同一条改写：适配器在两种可接受拼法里都能定位 `.under(...)` 参数，因此搬动不会只对
/// 带标记拼法静默可用。
#[test]
fn a_move_plan_works_for_the_version_one_spelling() {
    let source = "XIRANG_FACE(r_area, (::xirang::soil<Area>()\n    .under(::xirang::faces::old::NODE_ID)\n    .named(\"Area\")));\n";
    let edits = CppLanguage
        .plan_move(&crate::registration::MoveRequest {
            source,
            old_module: "native::area",
            new_module: "native::panel::area",
            old_parent: Some("old"),
            new_parent: Some("shop::panel"),
        })
        .expect("the compat spelling plans too");
    assert_eq!(edits.len(), 1);
    assert_eq!(
        &source[edits[0].start..edits[0].end],
        "::xirang::faces::old::NODE_ID"
    );
    assert_eq!(
        edits[0].replacement,
        "::xirang::faces::shop::panel::NODE_ID"
    );
}

/// The adapter's plan and the generic edit gate compose: applying the planned edit yields source
/// that reads back with the new parent, and the unrelated bytes are untouched.
/// 适配器计划与通用编辑门可以组合：应用计划得到的源码能读回新父，且无关字节未被改动。
#[test]
fn a_planned_move_edit_applies_and_reads_back() {
    let source = "XIRANG_SOIL\nconstexpr auto SOIL = ::xirang::soil<Area>()\n    .under(::xirang::faces::old::NODE_ID)\n    .named(\"Area\");\n";
    let edits = CppLanguage
        .plan_move(&crate::registration::MoveRequest {
            source,
            old_module: "native::area",
            new_module: "native::shop::panel::area",
            old_parent: Some("old"),
            new_parent: Some("shop::panel"),
        })
        .expect("the adapter plans");
    let updated =
        crate::registration::apply_source_edits(source, &edits).expect("the gate accepts");
    assert!(updated.contains(".under(::xirang::faces::shop::panel::NODE_ID)"));
    assert!(updated.contains(".named(\"Area\")"));
    let declaration = read_declaration(&updated).expect("the rewritten source reads back");
    assert_eq!(
        declaration.face.parent,
        "::xirang::faces::shop::panel::NODE_ID"
    );
}

/// A rendered declaration reads back to the fields it was rendered from.
/// 渲染出来的声明读回它被渲染时的字段。
///
/// The renderer and the reader go through one grammar table, so this is the pin that a method name
/// or a value style cannot drift on one side: the text the tool writes is the text it reads.
/// 渲染器与读取器走同一张语法表，因此这条钉子钉住"方法名或取值风格不会只在一侧漂移"：工具写出的文本就是
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
    let plan = CppLanguage
        .plan(&with(registration.clone()))
        .expect("the plan");
    let source = &plan.files[0].updated;
    let read = read_declaration(source).expect("the generated source reads back");
    assert!(
        registration.satisfies(&read.face.registration),
        "the read-back carries every field the request named:\n{source}\nrequest: {registration:?}\nread: {:?}",
        read.face.registration
    );
    assert_eq!(read.kind(), "Gauge", "the kind reads back: {source}");
}
