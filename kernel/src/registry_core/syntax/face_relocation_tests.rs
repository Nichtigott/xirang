//! Relocation edits must leave comments, literal contents and other invocations intact.
//! 搬迁编辑必须保留注释、字面量内容和其他宏调用。

use super::*;

#[test]
fn relocation_changes_only_the_declaration_tokens_with_unicode_and_comments() {
    let source = "const NOTE: &str = \"中文 panel_object!\";\n#[allow(dead_code)]\ncrate::/* alias */panel_object! { kind: Leaf, name: { zh: \"中文\", en: \"parent: crate::panel::NODE_ID\" }, parent: /* before */ crate::panel::NODE_ID /* after */, }\npub fn message() -> &'static str { \"panel_object!\" }\n";
    let expected = source
        .replace("/* alias */panel_object", "/* alias */shelf_object")
        .replace(
            "/* before */ crate::panel::NODE_ID",
            "/* before */ crate::shelf::NODE_ID",
        );
    assert_eq!(
        relocate_face_declaration(
            source,
            "crate::shelf::NODE_ID",
            Some(("panel_object", "shelf_object"))
        )
        .unwrap(),
        expected
    );
}

#[test]
fn relocation_refuses_an_alias_that_occurs_only_in_a_literal() {
    let source = "crate::actual_object! { kind: Leaf, parent: crate::panel::NODE_ID, }\npub const NOTE: &str = \"panel_object!\";\n";
    assert!(
        relocate_face_declaration(
            source,
            "crate::shelf::NODE_ID",
            Some(("panel_object", "shelf_object"))
        )
        .is_err()
    );
}
