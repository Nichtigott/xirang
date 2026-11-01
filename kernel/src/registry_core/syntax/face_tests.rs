//! Face parser and reference-scan tests.
//! 注册面解析与引用扫描的测试。

use super::{ParentSyntax, is_face_source, parse_face, replace_face_macro, source_references};

/// Cheap exclusion preserves marker trust and lets the parser decide every candidate.
/// 廉价排除保留标记信任，并让解析器裁决全部候选。
#[test]
fn face_detection_keeps_markers_aliases_and_syntax_boundaries() {
    let marker = "// fixture generated face";
    for source in [
        marker,
        "crate::root_object /* comment */ ! { kind: Widget }",
        "crate::nested_object! { kind: Widget }",
        "crate::__external_object! { kind: Widget }",
    ] {
        assert!(is_face_source(source, marker), "{source}");
    }
    for source in [
        "pub fn ordinary() {}",
        "fn broken(",
        "// crate::root_object! { kind: Fake }",
        "fn text() { let _ = \"nested_object! { kind: Fake }\"; }",
        "crate::root_object! { kind: }",
    ] {
        assert!(!is_face_source(source, marker), "{source}");
    }
}

#[test]
fn parses_multiline_registration_tokens_and_locations() {
    let source = r#"
crate::control_object! {
    kind: Button,
    name: { zh: "按钮", en: "Button" },
    requires: [
        "layout.viewport" => "ControlRegistry",
        "draw.basic" => "BasicDrawing",
    ],
    parent: crate::NodeId::from_path("control/control.rs", "ControlRegistry"),
    getting_from_other_registry: Some("engine"),
}
"#;
    let face = parse_face(source).unwrap().unwrap();

    assert_eq!(face.path("kind").as_deref(), Some("Button"));
    assert_eq!(face.localized("name", "zh").as_deref(), Some("按钮"));
    assert_eq!(face.field_location("requires").unwrap().line, 5);
    assert_eq!(
        face.requirements("requires").unwrap(),
        [
            ("layout.viewport".to_owned(), "ControlRegistry".to_owned()),
            ("draw.basic".to_owned(), "BasicDrawing".to_owned())
        ]
    );
    assert_eq!(
        face.parent(),
        Some(ParentSyntax::FromPath {
            source: "control/control.rs".to_owned(),
            kind: "ControlRegistry".to_owned(),
        })
    );
    assert_eq!(
        face.option_string("getting_from_other_registry"),
        Some(Some("engine".to_owned()))
    );
}

#[test]
fn parses_namespaced_package_root_parent() {
    let source = r#"
crate::control_object! {
    kind: Workspace,
    parent: crate::root_node_id(env!("CARGO_PKG_NAME")),
}
"#;
    let face = parse_face(source).unwrap().unwrap();

    assert_eq!(face.parent(), Some(ParentSyntax::Root));
}

/// A parent path is matched at a path-segment boundary, not by suffix: `crate::NOT_ROOT_NODE_ID`
/// is a different constant, `my_root_node_id` a different function, and `OwnNodeId::from_path` a
/// different type's constructor. Suffix matching classified all three as the parent the face
/// declared (audit `KN6`).
/// 父路径在**路径段边界**上匹配，而不是按后缀：`crate::NOT_ROOT_NODE_ID` 是另一个常量、
/// `my_root_node_id` 是另一个函数、`OwnNodeId::from_path` 是另一个类型的构造函数。按后缀匹配会
/// 把三者都判成该面声明的那个父级（审计 `KN6`）。
#[test]
fn a_parent_look_alike_is_not_the_parent() {
    let cases = [
        (
            "crate::NOT_ROOT_NODE_ID",
            "a different constant is not the root",
        ),
        ("my_root_node_id(1)", "a different function is not the root"),
        (
            "OwnNodeId::from_path(\"control/control.rs\", \"ControlRegistry\")",
            "another type's constructor is not this one",
        ),
    ];
    for (parent, why) in cases {
        let source = format!("crate::control_object! {{ kind: Workspace, parent: {parent} }}");
        let face = parse_face(&source).unwrap().unwrap();
        assert_eq!(face.parent(), None, "{why}: {parent}");
    }

    // The spellings the tree does use still classify, so the boundary rule narrows the match
    // rather than breaking it.
    // 树里真正使用的拼法仍然分类成功——边界规则是收紧匹配，而不是弄坏它。
    let real_root = "crate::control_object! { kind: Workspace, parent: crate::ROOT_NODE_ID }";
    assert_eq!(
        parse_face(real_root).unwrap().unwrap().parent(),
        Some(ParentSyntax::Root)
    );
    let own_node = "crate::control_object! { kind: Widget, parent: crate::NODE_ID }";
    assert_eq!(
        parse_face(own_node).unwrap().unwrap().parent(),
        Some(ParentSyntax::NodePath("crate".to_owned()))
    );
}

#[test]
fn rejects_duplicate_fields() {
    let source = "crate::control_object! { kind: First, kind: Second }";
    let error = parse_face(source).unwrap_err();
    assert!(error.message.contains("duplicate field `kind`"));
}

#[test]
fn replacing_a_face_preserves_its_rust_implementation() {
    let source = r#"pub struct Button;
impl Button { pub fn paint(&self) -> u32 { 7 } }
crate::control_object! {
    kind: Button,
    stable_name: "button",
}
#[test] fn paints() { assert_eq!(Button.paint(), 7); }
"#;
    let replacement = r#"crate::control_object! {
    kind: Button,
    stable_name: "button_graft",
}"#;

    let copied = replace_face_macro(source, replacement).unwrap();

    assert!(copied.contains("pub fn paint(&self) -> u32 { 7 }"));
    assert!(copied.contains("stable_name: \"button_graft\""));
    assert!(!copied.contains("stable_name: \"button\","));
    assert!(copied.contains("#[test] fn paints()"));
}

#[test]
fn source_references_ignore_imports_strings_comments_and_registration_data() {
    let source = r#"
use crate::unused::Thing;
fn run_button() {
    crate::control::object::button::dispatch_action("unused::fake()", true);
    // crate::comment::fake();
    crate::control_object! { kind: Fake, parent: crate::hidden::NODE_ID }
}
"#;
    let references = source_references(source).unwrap();
    assert!(
        references
            .paths
            .contains("crate::control::object::button::dispatch_action")
    );
    assert!(!references.paths.iter().any(|path| path.contains("unused")));
    assert!(!references.paths.iter().any(|path| path.contains("hidden")));
    assert!(!references.conservative);
}

/// A macro invocation's tokens are read for paths rather than ignored.
/// 宏调用的 token 会被读出路径，而不是被忽略。
///
/// The build used to walk into no macro body at all, so a face named only as
/// `vec![crate::control::object::dial::NODE_ID]` was pruned while `check` reported
/// ok and the host's `cargo check` failed with `E0433`.
/// 构建过去完全不进入宏内容，因此只被 `vec![crate::control::object::dial::NODE_ID]`
/// 命名的注册面会被剪掉，而 `check` 报 ok、宿主的 `cargo check` 以 `E0433` 失败。
#[test]
fn a_macro_invocation_is_read_for_the_paths_it_spells() {
    let referenced = "pub fn build() { let _ = vec![crate::control::object::dial::NODE_ID]; }";
    let references = source_references(referenced).unwrap();
    assert!(
        references
            .paths
            .contains("crate::control::object::dial::NODE_ID"),
        "a path inside a macro is a reference like any other: {:?}",
        references.paths
    );

    // The same file without the macro keeps the scan narrow, so this is not a
    // blanket widening.
    // 同一份文件去掉宏之后扫描仍然收窄，因此这不是无脑放宽。
    let narrow = "pub fn build() -> u8 { crate::control::dial::NODE_ID }";
    assert!(!source_references(narrow).unwrap().conservative);

    // The build's own wiring is read by its own scanners, and an invocation whose
    // tokens are only literals cannot name a node: neither may widen the scope, or
    // the automatic scope would be off for every real project (an entry always
    // contains `host!()`, and a declaration macro's contents are metadata).
    // 本构建自己的接线由各自的扫描器读取，而 token 全是字面量的调用不可能命名节点：两者都不得
    // 放宽作用域，否则每个真实工程都会失去自动作用域（入口总有 `host!()`，声明宏的内容是元数据）。
    for transparent in [
        "xirang_toolchain::run_method::host!();",
        "static_graft_plan!(crate::FRAMEWORK, cut \"root/a\");",
        "fn f() { crate::control_object! { kind: Fake } }",
        "const NAME: &str = env!(\"CARGO_PKG_NAME\");",
        "const TWO: &str = concat!(\"a\", \"b\");",
    ] {
        assert!(
            !source_references(transparent).unwrap().conservative,
            "`{transparent}` must not widen the scope"
        );
    }
}

/// The build has to follow a declaration's `cfg` gate, so the parser keeps
/// it exactly as written.
/// 构建需要跟随声明的 `cfg` 门控，因此解析器按原文保留它。
#[test]
fn a_declaration_keeps_its_cfg_gate() {
    let face = parse_face(
        "// generated-by=XiRang\n#[cfg(feature = \"optional-face\")]\n\
             crate::root_object! {\n    kind: Widget,\n}\n",
    )
    .expect("parse")
    .expect("face");
    assert_eq!(face.cfg(), Some("feature = \"optional-face\""));
}

/// A `*_object!` macro that is not a registration declaration is refused for its own
/// reason, instead of being taken for a face that is missing `parent:`.
/// 并非注册声明的 `*_object!` 宏按它自己的理由被拒，而不是被当成缺少 `parent:` 的注册面。
///
/// Red before the fix: the field reader accepted `name: "x", size: 3` as one value (the
/// splitter only starts a field at a name the vocabulary knows), so the macro became a face
/// whose `size` was silently absorbed, and the build then reported a `parent-macro` error
/// for a macro that is not part of the registration mechanism (audit `KRN-K-23`).
/// 修前为红：字段读取器把 `name: "x", size: 3` 收成一个取值（切分器只在词表认识的键处开始
/// 字段），于是该宏成了一个面、`size` 被静默吞掉，随后构建为这个根本不属于注册机制的宏报出
/// `parent-macro` 错误（审计 `KRN-K-23`）。
#[test]
fn a_non_registration_object_macro_is_refused_with_its_own_reason() {
    let error = parse_face("widget_object! { name: \"x\", size: 3 }")
        .expect_err("this body is not a registration declaration");
    let message = error.to_string();
    assert!(
        message.contains("size"),
        "the reason names the text left over: {message}"
    );
    assert!(
        !message.contains("parent"),
        "it is not a missing parent: {message}"
    );

    // The registration vocabulary itself still parses, including a value written with a
    // top-level-looking closure inside brackets.
    // 注册词表本身仍可解析，包括括号内写出、看起来像顶层的闭包取值。
    let face = parse_face(
        "crate::control_object! {\n    kind: Button,\n    needs_registry: true,\n    flow: |a: u32| a,\n}\n",
    )
    .expect("a canonical body parses")
    .expect("one face");
    assert_eq!(face.macro_name, "control_object");
}

/// The struct-literal form is the **same face** as the macro block it replaces (`docs/open-work-2026-10-08.md`
/// H, ruling A2e). Identity is what "same" means here: `NodeId = hash(namespace, relative path, kind)`, and
/// the new form does not write `kind` at all — the **type name is the kind** — so the one thing this must
/// prove is that the two forms hash to the same identity. If that ever stops holding, every face in a
/// migrated repository changes identity at once, and the on-disk graft records stop resolving.
/// 结构体字面量形与它取代的宏块是**同一个面**（`docs/open-work-2026-10-08.md` H，裁定 A2e）。这里的"同一个"
/// 指身份：`NodeId = hash(命名空间, 相对路径, kind)`，而新形**根本不写 `kind`**——**类型名就是 kind**——因此
/// 这条钉子要证的就是两形哈希到同一个身份。它一旦不成立，被迁移的仓库里**所有面同时换身份**，而落盘的嫁接
/// 记录会不再解析。
#[test]
fn the_struct_literal_form_is_the_same_face_as_the_macro_block() {
    let macro_form = "crate::root_object! {\n    kind: Card,\n    parent: crate::board::NODE_ID,\n    needs_registry: true,\n}\n";
    let literal_form = "pub struct Card;\n\nimpl Card {\n    pub const REGISTRATION: Registration = Registration {\n        parent: crate::board::NODE_ID,\n        needs_registry: true,\n    };\n}\n";
    let by_macro = parse_face(macro_form)
        .expect("the macro form parses")
        .expect("a face");
    let by_literal = parse_face(literal_form)
        .expect("the struct-literal form parses")
        .expect("a face");

    assert_eq!(
        by_literal.path("kind").as_deref(),
        Some("Card"),
        "the type name is the kind, synthesized so every reader downstream sees the same field the macro form carries"
    );
    assert_eq!(by_macro.path("kind"), by_literal.path("kind"));
    assert_eq!(by_macro.parent(), by_literal.parent());
    assert_eq!(
        by_macro.boolean("needs_registry"),
        by_literal.boolean("needs_registry")
    );
    // The kernel's own namespaced constructor: `NodeId = hash(namespace, relative path, kind)`. The
    // toolchain's `package_node_id` is the same function with this checkout's namespace filled in.
    // 内核自己的命名空间构造器：`NodeId = hash(命名空间, 相对路径, kind)`。工具链的 `package_node_id`
    // 就是同一个函数、填上本检出的命名空间。
    let identity = |kind: &str| {
        crate::registry_core::identity::NodeId::from_namespaced_path(
            "test-namespace",
            "board/card/card.rs",
            kind,
        )
    };
    assert_eq!(
        identity("Card"),
        identity(by_literal.path("kind").expect("a kind").as_str()),
        "and the two forms hash to one identity"
    );
}

/// The narrow parts of the form, each one a way a reader could mistake an ordinary constant for a face.
/// 这个形**有意写窄**的几处；每一处都是"读者可能把普通常量误读成面"的一条路。
#[test]
fn only_a_registration_literal_is_read_as_a_face() {
    for source in [
        // not `REGISTRATION`
        "pub struct Card;\nimpl Card { pub const OTHER: Registration = Registration { parent: crate::x::NODE_ID }; }\n",
        // the declared type is not `Registration`
        "pub struct Card;\nimpl Card { pub const REGISTRATION: Something = Registration { parent: crate::x::NODE_ID }; }\n",
        // the literal is not `Registration`
        "pub struct Card;\nimpl Card { pub const REGISTRATION: Registration = Something { parent: crate::x::NODE_ID }; }\n",
        // not a struct literal at all
        "pub struct Card;\nimpl Card { pub const REGISTRATION: Registration = REGISTRATION; }\n",
    ] {
        assert!(
            parse_face(source).expect("parses").is_none(),
            "not a face: {source}"
        );
    }
}

/// **Step ① of H/A2e is about *two* readers, and this is the second one.** The authoring executor reads a
/// face through the same kernel parser, so it accepts the struct-literal form for free — but its
/// **writer** splices text and does not know the new form yet, so a rewrite that would cross the two
/// spellings is refused by name rather than attempted. Measured: crossing them spliced a macro block into
/// an `impl`, which does not compile.
/// **H/A2e 的第①步说的是*两个*读者，这就是第二个。** 作者面执行器经同一个内核解析器读面，因此它**免费**
/// 拿到新形；但它的**写者**拼的是文本、还不认识新形，所以"跨两形"的重写**按名拒绝**而不是去试。实测：跨形
/// 会把宏块拼进 `impl`，编译不过。
#[test]
fn the_two_readers_take_both_forms_and_the_writer_refuses_to_cross_them() {
    let macro_form =
        "crate::root_object! {\n    kind: Card,\n    parent: crate::board::NODE_ID,\n}\n";
    let literal_form = "pub struct Card;\n\nimpl Card {\n    pub const REGISTRATION: Registration = Registration {\n        parent: crate::board::NODE_ID,\n    };\n}\n";
    // Reading: the executor's own entry point, one layer up from the parser.
    let read = |source: &str| {
        let face = parse_face(source).expect("parses").expect("a face");
        face.is_struct_literal()
    };
    assert!(!read(macro_form));
    assert!(read(literal_form), "the second reader takes the new form");

    // Writing: same form ⇒ fine (the identity case is step ③'s business, not this step's).
    assert!(
        replace_face_macro(macro_form, macro_form).is_ok(),
        "a macro file is rewritten with a macro"
    );
    assert!(
        replace_face_macro(literal_form, literal_form).is_ok(),
        "and a literal file with a literal"
    );

    // Writing across the forms: refused, by name, with the step that will lift it.
    for (source, replacement, from, into) in [
        (literal_form, macro_form, "struct-literal", "macro"),
        (macro_form, literal_form, "macro", "struct-literal"),
    ] {
        let error = replace_face_macro(source, replacement)
            .expect_err("crossing the forms is refused, not attempted");
        assert!(
            error.to_string().contains(from) && error.to_string().contains(into),
            "the refusal names both spellings: {error}"
        );
    }
}

/// A list field written in the **new spelling** carries the field's real Rust type, so `exports` is
/// written `&["…"]` there and `["…"]` in the macro spellings. Both are the same list, and the difference
/// is the borrow, not the shape — the readers absorb it in one place.
/// 用**新拼法**写的列表字段带着字段真实的 Rust 类型，因此那里的 `exports` 写成 `&["…"]`，而宏那两种拼法写
/// `["…"]`。两者是同一份列表，差别只在那个引用符号、不在形状——读取器在一处吸收它。
#[test]
fn a_list_field_reads_the_same_with_or_without_a_leading_borrow() {
    let by_macro = "crate::root_object! {\n    kind: Card,\n    parent: crate::board::NODE_ID,\n    exports: [\"control.render\"],\n}\n";
    let by_literal = "pub struct Card;\n\nimpl Card {\n    pub const REGISTRATION: Registration = Registration {\n        exports: &[\"control.render\"],\n        ..Registration::DEFAULT\n    };\n}\n";

    let macro_face = parse_face(by_macro).expect("parses").expect("a face");
    let literal_face = parse_face(by_literal).expect("parses").expect("a face");

    assert_eq!(
        macro_face.string_list("exports"),
        Some(vec!["control.render".to_owned()]),
        "the macro spelling"
    );
    assert_eq!(
        literal_face.string_list("exports"),
        macro_face.string_list("exports"),
        "and the literal spelling reads the same list"
    );
    assert!(
        literal_face.is_struct_literal(),
        "the two are still two spellings of one face"
    );
}

/// The type-like half of a face written in the new spelling lives in the `impl FaceContract` beside it,
/// and every downstream reader asks for it by the same field names the macro spellings use.
/// 用新拼法写的面，其类型类那一半住在它旁边的 `impl FaceContract` 里；下游每个读者都用与宏那两种拼法相同的
/// 字段名索取它。
#[test]
fn a_literal_form_face_reads_its_type_like_half_from_the_contract_block() {
    let source = r#"// generated-by=XiRang
pub struct Card;

impl FaceContract for Card {
    type Preset = crate::board::CardPreset;
    type Parts = crate::board::CardParts;
    const HANDLE_TRAITS: &'static [&'static str] = &["ControlHandle"];
    const PART_TRAITS: &'static [&'static str] = &["CardParts"];
}

xirang_toolchain::assert_contracts!(Card: crate::board::ControlHandle, crate::board::CardParts);

impl Card {
    pub const REGISTRATION: Registration = Registration {
        name: LocalizedText { zh: "卡片", en: "Card" },
        ..Registration::DEFAULT
    };
}
"#;
    let face = parse_face(source).expect("parses").expect("a face");
    assert!(face.is_struct_literal(), "read as the literal spelling");
    assert_eq!(
        face.string_list("handle_traits"),
        Some(vec!["ControlHandle".to_owned()]),
        "the labels the runtime reads"
    );
    // A type-like field holds a **path**, not a string literal: ask for it the way the build does.
    // 类型类字段装的是**路径**而不是字符串字面量：按构建的读法索取它。
    assert_eq!(
        face.path("preset").as_deref(),
        Some("crate::board::CardPreset"),
        "the preset type, as the declaration spells it"
    );
    assert_eq!(
        face.path("parts").as_deref(),
        Some("crate::board::CardParts"),
        "and the parts type"
    );
}

/// The labels and the `assert_contracts!` line are two statements of one fact, and the build refuses a
/// face where they disagree: a rule that requires a label would otherwise find a face that never claimed
/// that trait.
/// 标签与 `assert_contracts!` 那一行是同一件事实的两种陈述；两者不一致时构建拒绝这个面，否则某条要求某个
/// 标签的规则会找到一个从未声称过那个 trait 的面。
#[test]
fn labels_that_disagree_with_the_assertion_are_refused() {
    let source = r#"// generated-by=XiRang
pub struct Card;

impl FaceContract for Card {
    const HANDLE_TRAITS: &'static [&'static str] = &["SomethingElse"];
}

xirang_toolchain::assert_contracts!(Card: crate::board::ControlHandle);

impl Card {
    pub const REGISTRATION: Registration = Registration {
        ..Registration::DEFAULT
    };
}
"#;
    let error = parse_face(source).expect_err("the mismatch is refused");
    assert!(
        error.message.contains("handle_traits"),
        "the refusal names the field: {}",
        error.message
    );
}

/// The two spellings of one face file must mention the same modules: the struct-literal form keeps its
/// declaration in an `impl`, and reading those fields as references would widen the build's source scope
/// the moment a face is migrated — while its plan stayed byte-identical.
/// 同一份面文件的两种拼法必须提及同一批模块：字面量形把声明放在 `impl` 里，而那些字段一旦被读成引用，构建的
/// 源码作用域就会在面迁移的那一刻放宽——而它的计划是逐字不变的。
#[test]
fn both_face_spellings_mention_the_same_modules() {
    let macro_form = r#"
use crate::control::{ControlFrame, ControlHandle};
#[xirang_toolchain::soil]
pub const SOIL: xirang_toolchain::Soil = xirang_toolchain::Soil::of::<Slider>()
    .exports(&["control.render"])
    .under(crate::control::NODE_ID)
    .handle_contracts(xirang_toolchain::contracts!(Slider: crate::control::ControlHandle));

pub fn refresh() { crate::control::object::slider::dispatch(); }
"#;
    let literal_form = r#"
use crate::control::{ControlFrame, ControlHandle};
pub const NODE_ID: NodeId = NodeId::from_raw([1; 16]);
pub struct Slider;
impl FaceContract for Slider {
    type Preset = NoPreset;
    const HANDLE_TRAITS: &'static [&'static str] = &["ControlHandle"];
}
xirang_toolchain::assert_contracts!(Slider: crate::control::ControlHandle);
impl Slider {
    pub const REGISTRATION: Registration = Registration {
        parent: crate::control::NODE_ID,
        exports: &["control.render"],
        ..Registration::DEFAULT
    };
}

pub fn refresh() { crate::control::object::slider::dispatch(); }
"#;
    let described = |source: &str| {
        let references = source_references(source).unwrap();
        let mut paths = references
            .paths
            .iter()
            .filter(|path| path.contains("control"))
            .cloned()
            .collect::<Vec<_>>();
        paths.sort();
        paths
    };
    assert_eq!(
        described(literal_form),
        described(macro_form),
        "the declaration's own paths are metadata in both spellings; only real code counts"
    );
    assert!(
        described(literal_form)
            .iter()
            .any(|path| path.contains("dispatch")),
        "and a genuine call in the same file is still a reference: {:?}",
        described(literal_form)
    );
}

/// One declaration, both spellings: every field must reach the **consumers** as the same value.
/// 同一份声明的两种拼法：每个字段交到**消费者**手里的值必须相同。
///
/// This is the field-by-field half of A2e's acceptance. It compares what the consumers read — the
/// accessors on `FaceSyntax` and the two authoring parsers — rather than the raw token text, because the
/// raw text already differs harmlessly (`exports: &[…]` versus `.exports(&[…])` both answer the same
/// list) and a red list that mixes harmless spelling with real breakage buries the real one.
/// 这是 A2e 验收的逐字段那一半。它比的是**消费者读到的值**——`FaceSyntax` 上的访问器与那两个创作解析器
/// ——而不是裸 token 文本，因为裸文本本来就有无害的差别（`exports: &[…]` 与 `.exports(&[…])` 给出同一份
/// 列表），而把无害拼写与真实破坏混在一起的红名单会把真问题埋掉。
#[test]
fn both_face_spellings_read_the_same_field_values() {
    use crate::authoring::parse::parse_flow_expression;

    let macro_form = r#"
use crate::control::ControlHandle;
#[xirang_toolchain::soil]
pub const SOIL: xirang_toolchain::Soil = xirang_toolchain::Soil::of::<Button>()
    .name("Button", "按钮")
    .summary("A button", "一个按钮")
    .stable_name("ui.button")
    .exports(&["control.render"])
    .requires("render", "Renderer")
    .provides(&["control.render.provide"])
    .needs_registry(true)
    .runtime_checks(&[NON_EMPTY_TEXT])
    .flow(FlowContract::new(ContractId::new("control.render.v1"), 1, "ControlInput", "ControlFrame"))
    .handle_traits(&["ControlHandle"]);
"#;
    // Written the way the literal form writes it: the field types are the `Registration` struct's own
    // (`stable_name: Option<&str>`, `name: LocalizedText`, `requires: &[RequirementSpec]`, …), because that
    // is what an author of this spelling actually types.
    // 按字面量形的写法：字段类型就是 `Registration` 结构体自己的那些（`stable_name: Option<&str>`、
    // `name: LocalizedText`、`requires: &[RequirementSpec]`……），因为这一形的作者实际敲的就是这些。
    let literal_form = r#"
use crate::control::ControlHandle;
pub const NODE_ID: NodeId = NodeId::from_raw([1; 16]);
pub struct Button;
impl FaceContract for Button {
    type Preset = NoPreset;
    type Parts = NoParts;
    const HANDLE_TRAITS: &'static [&'static str] = &["ControlHandle"];
}
xirang_toolchain::assert_contracts!(Button: crate::control::ControlHandle);
impl Button {
    pub const REGISTRATION: Registration = Registration {
        parent: crate::control::NODE_ID,
        name: LocalizedText { zh: "按钮", en: "Button" },
        summary: LocalizedText { zh: "一个按钮", en: "A button" },
        stable_name: Some("ui.button"),
        exports: &["control.render"],
        requires: &[RequirementSpec { capability: "render", provider: "Renderer" }],
        provides: &["control.render.provide"],
        needs_registry: true,
        runtime_checks: &[NON_EMPTY_TEXT],
        flow: Some(FlowContract::new(ContractId::new("control.render.v1"), 1, "ControlInput", "ControlFrame")),
        ..Registration::DEFAULT
    };
}
"#;
    let read = |source: &str| {
        parse_face(source)
            .expect("both spellings parse")
            .expect("a face is declared")
    };
    let by_macro = read(macro_form);
    let by_literal = read(literal_form);
    assert!(
        by_literal.is_struct_literal(),
        "the literal form is read as such"
    );

    let mut differences = Vec::new();
    let mut compare = |what: &str, left: String, right: String| {
        if left != right {
            differences.push(format!("{what}: macro {left} vs literal {right}"));
        }
    };
    for language in ["zh", "en"] {
        for field in ["name", "summary"] {
            compare(
                &format!("{field}.{language}"),
                format!("{:?}", by_macro.localized(field, language)),
                format!("{:?}", by_literal.localized(field, language)),
            );
        }
    }
    compare(
        "stable_name",
        format!("{:?}", by_macro.string("stable_name")),
        format!("{:?}", by_literal.string("stable_name")),
    );
    compare(
        "needs_registry",
        format!("{:?}", by_macro.boolean("needs_registry")),
        format!("{:?}", by_literal.boolean("needs_registry")),
    );
    for field in ["exports", "provides", "runtime_checks", "handle_traits"] {
        compare(
            field,
            format!("{:?}", by_macro.string_list(field)),
            format!("{:?}", by_literal.string_list(field)),
        );
    }
    // The build reads `requires` through `FaceSyntax::requirements` (`registration_phase.rs`,
    // `discovery_cache.rs`), not through the authoring parser — so that is the consumer this pin has to
    // agree on. Picking the wrong consumer would have judged the two spellings by a shape no caller reads.
    // 构建经 `FaceSyntax::requirements` 读 `requires`（`registration_phase.rs`、`discovery_cache.rs`），
    // 而不是经创作解析器——因此这才是本钉子必须对齐的那个消费者。挑错消费者，等于用没人读的形状去判两形。
    compare(
        "requires",
        format!("{:?}", by_macro.requirements("requires")),
        format!("{:?}", by_literal.requirements("requires")),
    );
    compare(
        "flow",
        format!(
            "{:?}",
            by_macro
                .field("flow")
                .map(|text| parse_flow_expression(&text))
        ),
        format!(
            "{:?}",
            by_literal
                .field("flow")
                .map(|text| parse_flow_expression(&text))
        ),
    );
    assert!(
        differences.is_empty(),
        "the two spellings must hand the consumers the same value:\n{}\n\nraw text, for diagnosis:\n           name    macro {:?} literal {:?}\n  flow    macro {:?} literal {:?}\n  exports macro {:?} literal {:?}\n           requires macro {:?} literal {:?}\n  runtime_checks macro {:?} literal {:?}",
        differences.join("\n"),
        by_macro.field("name"),
        by_literal.field("name"),
        by_macro.field("flow"),
        by_literal.field("flow"),
        by_macro.field("exports"),
        by_literal.field("exports"),
        by_macro.field("requires"),
        by_literal.field("requires"),
        by_macro.field("runtime_checks"),
        by_literal.field("runtime_checks"),
    );
}

/// Rewriting a face written as a `Registration` literal must own its **whole region**: the declaration
/// **and** the `impl FaceContract` / `assert_contracts!` items that carry the type-class half. Splicing only
/// the declaration leaves the old contract behind — two `impl FaceContract` for one type, which rustc rejects
/// with E0119 — or keeps stale labels after a preset/parts edit.
/// 重写一个写成 `Registration` 字面量的面，必须拥有它**整段区域**：那份声明**以及**承载类型类那一半的
/// `impl FaceContract` / `assert_contracts!` 条目。只拼接声明会把旧合同留在原处——同一类型两个
/// `impl FaceContract`，rustc 以 E0119 拒绝——或者在改了 preset/parts 之后留下过期的标签。
#[test]
fn rewriting_a_literal_face_owns_its_whole_region() {
    let source = "use xirang_toolchain::run_method::run_method_core::{\n    NoParts, NoPreset, NodeId, Registration,}\n;\nuse xirang_toolchain::run_method::FaceContract;\n\npub struct Card;\n\nimpl FaceContract for Card {\n    type Preset = NoPreset;\n    type Parts = NoParts;\n    const HANDLE_TRAITS: &'static [&'static str] = &[\"OldContract\"];\n}\n\nimpl Card {\n    pub const REGISTRATION: Registration = Registration {\n        parent: crate::root_node_id(crate::XIRANG_NAMESPACE),\n        ..Registration::DEFAULT\n    };\n}\n\nxirang_toolchain::assert_contracts!(Card: OldContract);\n";
    let replacement = "pub struct Card;\n\nimpl FaceContract for Card {\n    type Preset = NoPreset;\n    type Parts = NoParts;\n    const HANDLE_TRAITS: &'static [&'static str] = &[\"NewContract\"];\n}\n\nimpl Card {\n    pub const REGISTRATION: Registration = Registration {\n        parent: crate::root_node_id(crate::XIRANG_NAMESPACE),\n        ..Registration::DEFAULT\n    };\n}\n\nxirang_toolchain::assert_contracts!(Card: NewContract);\n";
    let rewritten = crate::registry_core::syntax::replace_face_region(source, replacement)
        .expect("the rewrite succeeds");
    assert_eq!(
        rewritten.matches("impl FaceContract for Card").count(),
        1,
        "exactly one type-class item survives:\n{rewritten}"
    );
    assert_eq!(
        rewritten.matches("assert_contracts!").count(),
        1,
        "exactly one assertion line survives:\n{rewritten}"
    );
    assert!(
        rewritten.contains("NewContract") && !rewritten.contains("OldContract"),
        "the labels are the new ones:\n{rewritten}"
    );
    assert!(
        rewritten.contains("pub struct Card;"),
        "the author's own item is not the thing being replaced:\n{rewritten}"
    );
}

/// An unknown field in an `*_object!`-named macro says it may be an application macro.
/// `*_object!` 名字的宏里出现未知字段时，报错点名"它可能是应用宏"。
///
/// The measured case: a real application reuses the historical `control_object!` spelling for its
/// own control registry, whose `handle:` field this framework deleted on 2026-09-25 (audit §14).
/// The refusal is correct — the field is not in `FACE_FIELD_ORDER` — but the reader had to say why
/// a macro with a registration-sounding name is not a registration declaration, or every census row
/// looks like a broken face.
/// 实测案例：一个真实应用把历史上的 `control_object!` 拼法用于它自己的控件注册表，其中的 `handle:`
/// 字段已被本框架在 2026-09-25 删除（审计 §14）。拒绝是对的——该字段不在 `FACE_FIELD_ORDER` 里——但
/// 读取器必须说明"为什么一个听起来像注册的宏不是注册声明"，否则每条普查行都像坏面。
#[test]
fn an_unknown_field_names_the_application_macro_case() {
    let source = r#"
crate::control_object! {
    kind: ScrollBar,
    handle: ScrollBar,
}
"#;
    let error = parse_face(source).unwrap_err();
    assert!(
        error
            .message
            .contains("`handle` is not a registration face field"),
        "{}",
        error.message
    );
    assert!(
        error.message.contains("application macro"),
        "the refusal says an application macro may share the name: {}",
        error.message
    );
}
