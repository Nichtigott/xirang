//! Pins for the shared grammar half: the literal spelling and the obligations every plan states.
//! 共享语法那一半的钉子：字面量拼法，以及每个计划都会声明的义务。

use xirang_kernel::authoring::{
    FieldUpdate, ObligationKind, RegistrationField, RegistrationUpdate,
};

use super::{bracket_list, csharp_array, java_array, obligations, quote, slice_list, tuple};

/// The escapes are the ones both carriers compile or import.
/// 转义是两种载体都能编译或 import 的那些。
#[test]
fn a_literal_escapes_what_both_carriers_need() {
    assert_eq!(quote(r#"a"b"#), r#""a\"b""#);
    assert_eq!(quote("a\\b"), r#""a\\b""#);
    assert_eq!(quote("a\nb\tc"), r#""a\nb\tc""#);
}

/// A list literal keeps the decorator spelling, including the one-entry trailing comma.
/// 列表字面量保留装饰器拼法，包括单元素时的尾逗号。
#[test]
fn a_list_literal_keeps_the_decorator_spelling() {
    assert_eq!(tuple(""), "()");
    assert_eq!(tuple("a"), r#"("a",)"#);
    assert_eq!(tuple("a, b"), r#"("a", "b")"#);
    assert_eq!(tuple(" a ,, b "), r#"("a", "b")"#);
}

/// A bracket list is the JavaScript/TypeScript spelling: no trailing comma, empty is `[]`.
/// 方括号列表是 JavaScript/TypeScript 的拼法：没有尾逗号，空列表是 `[]`。
#[test]
fn a_bracket_list_is_the_script_spelling() {
    assert_eq!(bracket_list(""), "[]");
    assert_eq!(bracket_list("a"), r#"["a"]"#);
    assert_eq!(bracket_list("a, b"), r#"["a", "b"]"#);
    assert_eq!(bracket_list(" a ,, b "), r#"["a", "b"]"#);
}

/// A C# attribute array is `new[] { … }`, and an empty one needs its element type.
/// C# 特性数组是 `new[] { … }`，空数组要写出元素类型。
#[test]
fn a_csharp_attribute_array_is_new_brackets() {
    assert_eq!(csharp_array(""), "new string[0]");
    assert_eq!(csharp_array("a"), r#"new[] { "a" }"#);
    assert_eq!(csharp_array("a, b"), r#"new[] { "a", "b" }"#);
}

/// A Java annotation array is braces without `new`: empty is `{}`.
/// Java 注解数组是不带 `new` 的花括号：空列表是 `{}`。
#[test]
fn a_java_annotation_array_is_braces() {
    assert_eq!(java_array(""), "{}");
    assert_eq!(java_array("a"), r#"{"a"}"#);
    assert_eq!(java_array("a, b"), r#"{"a", "b"}"#);
    assert_eq!(java_array(" a ,, b "), r#"{"a", "b"}"#);
}

/// A typed slice is Go's spelling: the element type is written, and empty is `[]string{}`.
/// 带类型切片是 Go 的拼法：写出元素类型，空列表是 `[]string{}`。
#[test]
fn a_typed_slice_is_the_go_spelling() {
    assert_eq!(slice_list(""), "[]string{}");
    assert_eq!(slice_list("a"), r#"[]string{"a"}"#);
    assert_eq!(slice_list("a, b"), r#"[]string{"a", "b"}"#);
    assert_eq!(slice_list(" a ,, b "), r#"[]string{"a", "b"}"#);
}

/// The exports decide whether the Call obligation is required; the build sentence is the
/// language's own and is passed through verbatim.
/// 导出决定是否需要 Call 义务；构建那一句属于语言，原样传入。
#[test]
fn obligations_share_one_contract_and_take_the_build_sentence() {
    let mut registration = RegistrationUpdate::new();
    registration.set_normalized(
        RegistrationField::Exports,
        FieldUpdate::Set("gauge".to_owned()),
    );
    registration.set_normalized(
        RegistrationField::Kind,
        FieldUpdate::Set("Gauge".to_owned()),
    );
    let stated = obligations("Gauge", &registration, "compile it");
    assert!(
        stated
            .iter()
            .any(|obligation| obligation.kind == ObligationKind::Call),
        "an export adds the Call obligation: {stated:?}"
    );
    assert!(
        stated
            .iter()
            .any(|obligation| obligation.kind == ObligationKind::Build
                && obligation.detail == "compile it"),
        "the build sentence is the language's own: {stated:?}"
    );
    let plain = RegistrationUpdate::new();
    assert!(
        !obligations("Gauge", &plain, "import it")
            .iter()
            .any(|obligation| obligation.kind == ObligationKind::Call),
        "no exports means nothing to call"
    );
}
