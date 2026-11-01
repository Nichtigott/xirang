//! Pins for the Zig adapter: rendered text reads back, parents are module constants, and the plan
//! round-trips through the shared orchestration.
//! Zig 适配器的钉子：渲染出的文本能读回、父级是模块常量、计划经共享编排往返。

use super::{GRAMMAR, ZigLanguage, faces_module, parent_literal, read_declaration, render_file};
use crate::languages::kit::grammar::Role;
use crate::registration::LanguageAdapter;
use xirang_kernel::authoring::{
    AuthoringOperation, FaceAuthoringRequest, ParentReference, RegistrationField,
    RegistrationUpdate, SourceContext,
};
use xirang_kernel::interchange::LanguageId;

fn registration(kind: &str) -> RegistrationUpdate {
    let mut registration = RegistrationUpdate::new();
    registration
        .set_text(
            RegistrationField::Kind,
            xirang_kernel::authoring::FieldUpdate::Set(kind),
        )
        .expect("kind");
    registration
        .set_text(
            RegistrationField::NameZh,
            xirang_kernel::authoring::FieldUpdate::Set("仪表"),
        )
        .expect("name_zh");
    registration
        .set_text(
            RegistrationField::Exports,
            xirang_kernel::authoring::FieldUpdate::Set("shop.gauge,shop.dial"),
        )
        .expect("exports");
    registration
}

/// A rendered file parses back with the same fields, and the SDK spelling stays in place.
/// 渲染出的文件能读回同样的字段，SDK 拼法留在原处。
#[test]
fn a_rendered_declaration_reads_back_with_the_same_fields() {
    let text = render_file("", &registration("Gauge"), &[]);
    assert!(text.contains("pub export var SOIL linksection(xirang.SECTION) = xirang.record(.{"));
    assert!(text.contains("const xirang = @import(\"xirang\");"));
    let declaration = read_declaration(&text).expect("the rendered text parses");
    assert_eq!(declaration.face.kind(), "Gauge");
    assert_eq!(
        declaration
            .face
            .registration
            .find_field(RegistrationField::NameZh)
            .value()
            .copied(),
        Some("仪表")
    );
    assert_eq!(
        declaration
            .face
            .registration
            .find_field(RegistrationField::Exports)
            .value()
            .copied(),
        Some("shop.dial,shop.gauge")
    );
    assert!(declaration.face.extras.is_empty());
}

/// Fields the standard vocabulary does not name survive a read and are rendered back.
/// 标准词汇没有点名的字段能被读回，并在重渲染时保留。
#[test]
fn unmapped_fields_survive_a_rewrite() {
    let text = render_file(
        "",
        &registration("Gauge"),
        &[("params".to_owned(), "\"GaugeParams\"".to_owned())],
    );
    let declaration = read_declaration(&text).expect("parses");
    assert_eq!(
        declaration.face.extras,
        vec![("params".to_owned(), "\"GaugeParams\"".to_owned())]
    );
    let again = render_file("", &declaration.face.registration, &declaration.face.extras);
    assert!(again.contains(".params = \"GaugeParams\","));
    assert_eq!(
        GRAMMAR.lookup("kind").map(|spec| spec.role),
        Some(Role::Kind)
    );
}

/// Parents are the generated module constants; a bare identity is refused by name.
/// 父级是生成的模块常量；没有模块的身份按名拒绝。
#[test]
fn parents_are_generated_module_constants() {
    assert_eq!(
        parent_literal(&ParentReference::Root).expect("root"),
        "xirang_faces.ROOT_NODE_ID"
    );
    let object = parent_literal(&ParentReference::Object {
        id: xirang_kernel::identity::NodeId::from_raw([7u8; 16]),
        module: "shop::panel".to_owned(),
    })
    .expect("object");
    assert_eq!(object, "xirang_faces.shop__panel");
    let error = parent_literal(&ParentReference::Identity(
        xirang_kernel::identity::NodeId::from_raw([7u8; 16]),
    ))
    .expect_err("an identity without a module cannot be spelled");
    assert!(error.contains("module"), "{error}");
}

/// The generated identity module names every module and spells the root as sixteen bytes.
/// 生成的恒等模块点名每个模块，并把根写成十六字节。
#[test]
fn the_identity_module_spells_every_module_as_bytes() {
    let root = xirang_kernel::identity::root_node_id("shop").into_bytes();
    let text = faces_module(
        root,
        &[
            ("shop::panel".to_owned(), [7u8; 16]),
            ("3d::panel".to_owned(), [8u8; 16]),
        ],
    );
    assert!(text.contains("pub const ROOT_NODE_ID: [16]u8 = .{"));
    for byte in root {
        assert!(text.contains(&format!("0x{byte:02x}")), "{text}");
    }
    assert!(
        text.contains("pub const shop__panel: [16]u8 = .{"),
        "{text}"
    );
    assert!(text.contains("pub const _3d__panel: [16]u8 = .{"), "{text}");
}

/// The adapter plans a create, and its own reader accepts what the plan writes.
/// 适配器规划一次新建，它自己的读取器接受计划写出的内容。
#[test]
fn a_create_plan_reads_back_through_the_adapter() {
    let request = FaceAuthoringRequest {
        language: LanguageId::parse_language("zig").expect("a language id"),
        operation: AuthoringOperation::Create,
        destination: "src/gauge/gauge.zig".to_owned(),
        parent: ParentReference::Root,
        registration: registration("Gauge"),
        source_context: SourceContext::new(),
    };
    let plan = ZigLanguage
        .plan(&request)
        .expect("the shared orchestration accepts a Zig create");
    assert_eq!(plan.files.len(), 1);
    let updated = plan.files[0].updated.clone();
    let read_back = ZigLanguage
        .read(&updated)
        .expect("the file the plan wrote is one this adapter reads");
    assert!(
        request.registration.satisfies(&read_back.registration),
        "every field the create named survived: {read_back:#?}"
    );
}

/// A modify plan changes the named field in place and keeps everything it did not name.
/// 修改计划就地改名点名字段，保留它没有点名的一切。
#[test]
fn a_modify_plan_keeps_fields_it_does_not_name() {
    let created = render_file("", &registration("Gauge"), &[]);
    let mut update = RegistrationUpdate::new();
    update
        .set_text(
            RegistrationField::NameEn,
            xirang_kernel::authoring::FieldUpdate::Set("Dial"),
        )
        .expect("name_en");
    let request = FaceAuthoringRequest {
        language: LanguageId::parse_language("zig").expect("a language id"),
        operation: AuthoringOperation::Modify,
        destination: "src/gauge/gauge.zig".to_owned(),
        parent: ParentReference::Root,
        registration: update,
        source_context: SourceContext::new().with_existing_source(created),
    };
    let plan = ZigLanguage
        .plan(&request)
        .expect("the shared orchestration accepts a Zig modify");
    let updated = plan.files[0].updated.clone();
    assert!(updated.contains(".name_en = \"Dial\""), "{updated}");
    assert!(
        updated.contains("pub export var SOIL"),
        "the carrier is untouched: {updated}"
    );
    assert_eq!(
        updated.matches("xirang.record(.{").count(),
        1,
        "one record block after the rewrite: {updated}"
    );
    assert!(
        updated.contains(".parent = xirang_faces.ROOT_NODE_ID"),
        "a root parent is spelled as the generated constant: {updated}"
    );
    let read_back = ZigLanguage
        .read(&updated)
        .expect("the rewritten file parses");
    assert_eq!(
        read_back
            .registration
            .find_field(RegistrationField::NameZh)
            .value()
            .copied(),
        Some("仪表"),
        "the field the modify did not name survives"
    );
}

/// The build's scanner reads the kind and the parent constant; a byte literal is refused.
/// 构建的扫描器读出 kind 与父级常量；字节字面量按名拒绝。
#[test]
fn the_scanner_reads_a_parent_constant_and_refuses_bytes() {
    let source = "const xirang = @import(\"xirang\");\nconst faces = @import(\"xirang_faces\");\n\n                  pub export var SOIL linksection(xirang.SECTION) = xirang.record(.{\n    .kind = \"Gauge\",\n                      .parent = faces.shop__panel,\n});\n";
    let face = super::scan("src/gauge/gauge.zig", source)
        .expect("the scanner accepts the declaration")
        .expect("a face");
    assert_eq!(face.name, "Gauge");
    assert_eq!(face.parent, "shop__panel");
    assert!(!face.owns_registry);

    let bytes = source.replace("faces.shop__panel", "[_]u8{ 0x00 }");
    let error = super::scan("src/gauge/gauge.zig", &bytes)
        .expect_err("a byte literal is not a spelling the build resolves");
    assert!(error.contains("module constant"), "{error}");
}

/// A comment inside the record builder is prose, not a field, and the reader skips it.
/// 记录构造器里的注释是散文而不是字段，读取器跳过它。
#[test]
fn a_comment_inside_the_record_builder_is_not_a_field() {
    let text = "const xirang = @import(\"xirang\");\n\n\
                pub export var SOIL linksection(xirang.SECTION) = xirang.record(.{\n\
                    .kind = \"Gauge\",\n    // the face owns children\n    .owns_registry = true,\n});\n";
    let declaration = read_declaration(text).expect("comments are skipped");
    assert_eq!(declaration.face.kind(), "Gauge");
    assert!(declaration.face.extras.is_empty());
}
