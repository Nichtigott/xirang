//! Tests for the C++ spelling rules.
//! C++ 拼法规则的测试。

use super::*;

/// A normal module path reads like the logical tree.
/// 普通模块路径读起来就是逻辑树。
#[test]
fn a_module_path_becomes_nested_namespaces() {
    assert_eq!(nested("shop::panel"), "shop::panel");
    assert_eq!(
        node_id("shop::panel"),
        "::xirang::faces::shop::panel::NODE_ID"
    );
}

/// A keyword segment stays a legal identifier, and both consumers get the same escape.
/// 关键字段仍是合法标识符，且两个消费者拿到同一个转义。
#[test]
fn a_keyword_segment_is_escaped_deterministically() {
    assert_eq!(segment("new"), "new_");
    assert_eq!(segment("class"), "class_");
    assert_eq!(nested("shop::new"), "shop::new_");
    assert_eq!(flat("shop::new", "New"), "shop__new_New");
    // `type` is not a C++ keyword, so it keeps its spelling.
    // `type` 不是 C++ 关键字，因此保持原拼写。
    assert_eq!(segment("type"), "type");
}

/// The flat identifier keeps the version-1 spelling for an ordinary module.
/// 扁平标识符对普通模块保持版本 1 的拼法。
#[test]
fn the_flat_identifier_matches_the_version_one_spelling() {
    assert_eq!(flat("shop::panel", "Panel"), "shop__panel_Panel");
    assert_eq!(flat("native::area", "Area"), "native__area_Area");
}

/// A module segment that starts with a digit cannot start an identifier.
/// 以数字开头的模块段不能开始一个标识符。
#[test]
fn a_leading_digit_is_prefixed() {
    assert_eq!(segment("2d"), "_2d");
}

/// The keyword table is about identifiers, not about the words that merely look reserved.
/// 关键字表针对标识符，而不是仅仅看起来像保留字的词。
#[test]
fn a_contextual_identifier_is_left_alone() {
    assert_eq!(segment("override"), "override");
    assert_eq!(segment("final"), "final");
}
