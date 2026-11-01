//! Pins for the C++ lexical reader: real declarations, not words in comments or strings.
//! C++ 词法读取器的钉子：认真正的声明，不认注释或字符串里的词。

use super::*;

const COMPAT: &str = "\
#include \"xirang/soil.hpp\"
struct Area {};
XIRANG_FACE(r_area, (::xirang::soil<Area>()
                         .under(::xirang::faces::shop__panel_Panel)
                         .named(\"Area\")
                         .exports(\"area\")
                         .owns_registry()));
";

const MARKED: &str = "\
#include \"xirang/soil.hpp\"
struct Gauge {};
XIRANG_SOIL
constexpr auto SOIL = ::xirang::soil<Gauge>()
    .under(::xirang::faces::shop::panel::NODE_ID)
    .named(\"Gauge\")
    .needs_registry(false);
";

/// The baseline spellings read as one declaration each.
/// 两种基线拼法各读出一条声明。
#[test]
fn both_spellings_read_as_one_declaration() {
    let compat = read_one(COMPAT).expect("the version-1 spelling reads");
    assert_eq!(compat.spelling, Spelling::Compat);
    assert_eq!(compat.variable, "r_area");
    assert_eq!(compat.business_type, "Area");
    assert_eq!(compat.name.as_deref(), Some("Area"));
    assert!(compat.owns_registry());

    let marked = read_one(MARKED).expect("the marked spelling reads");
    assert_eq!(marked.spelling, Spelling::Marked);
    assert_eq!(marked.business_type, "Gauge");
    assert_eq!(marked.name.as_deref(), Some("Gauge"));
    assert!(!marked.owns_registry());
}

/// A comment that mentions `XIRANG_SOIL` before an old declaration does not change which spelling
/// the file uses: comments are not code.
/// 旧声明前提到 `XIRANG_SOIL` 的注释不改变文件所用的拼法：注释不是代码。
#[test]
fn a_comment_mentioning_the_other_marker_is_not_a_declaration() {
    let source = format!("// Future migration: XIRANG_SOIL\n{COMPAT}");
    let declaration = read_one(&source).expect("the old declaration is the only one");
    assert_eq!(declaration.spelling, Spelling::Compat);
    assert_eq!(declaration.name.as_deref(), Some("Area"));
}

/// Legal whitespace between `constexpr`, `auto` and `SOIL` is not part of the syntax.
/// `constexpr`、`auto`、`SOIL` 之间的合法空白不是语法的一部分。
#[test]
fn whitespace_between_the_keywords_is_accepted() {
    let source = MARKED.replace("constexpr auto SOIL", "constexpr  auto\nSOIL");
    let declaration = read_one(&source).expect("the marked spelling reads with extra whitespace");
    assert_eq!(declaration.spelling, Spelling::Marked);
    assert_eq!(declaration.name.as_deref(), Some("Gauge"));
}

/// The marker words inside strings or raw strings are not declarations.
/// 字符串或原始字符串里的标记词不是声明。
#[test]
fn markers_inside_strings_are_not_declarations() {
    let source = format!(
        "{MARKED}\nstatic const char* text = \"XIRANG_FACE(r, (x)) XIRANG_SOIL\";\nstatic const char* raw = R\"tag(XIRANG_SOIL constexpr auto SOIL = fake;)tag\";\n"
    );
    let declarations = declarations(&source).expect("the source tokenizes");
    assert_eq!(declarations.len(), 1, "{declarations:?}");
    assert_eq!(declarations[0].spelling, Spelling::Marked);
}

/// Comments inside the declaration are skipped when finding its delimiters and its end.
/// 声明内部的注释在找分隔符与结尾时被跳过。
#[test]
fn comments_inside_the_declaration_do_not_break_it() {
    let source = MARKED.replace(
        ".named(\"Gauge\")",
        "/* the face name */ .named(\"Gauge\") // and the rest",
    );
    let declaration = read_one(&source).expect("the declaration reads with comments inside");
    assert_eq!(declaration.name.as_deref(), Some("Gauge"));
}

/// Two declarations in one file are returned as two, so the caller can refuse or choose.
/// 一个文件里的两条声明作为两条返回，由调用方决定拒绝还是选择。
#[test]
fn two_declarations_are_both_seen() {
    let source = format!("{COMPAT}\n{MARKED}");
    let declarations = declarations(&source).expect("both tokenize");
    assert_eq!(declarations.len(), 2);
    assert_eq!(declarations[0].spelling, Spelling::Compat);
    assert_eq!(declarations[1].spelling, Spelling::Marked);
    let error = read_one(&source).unwrap_err();
    assert!(error.contains("declares 2 faces"), "{error}");
}

/// A marker in a longer identifier is not the marker.
/// 更长标识符里的标记词不是标记。
#[test]
fn a_longer_identifier_is_not_the_marker() {
    let source = format!("const int MY_XIRANG_SOIL = 1;\n{COMPAT}");
    let declaration = read_one(&source).expect("only the real declaration is seen");
    assert_eq!(declaration.spelling, Spelling::Compat);
}

/// Calls the authoring vocabulary does not map are still recorded, so an edit can preserve them.
/// 创作词汇不映射的调用仍被记录，使编辑能保留它们。
#[test]
fn unmapped_calls_are_kept_in_order() {
    let source = MARKED.replace(
        ".needs_registry(false)",
        ".params(\"crate::panel::Params\")\n    .needs_registry(false)",
    );
    let declaration = read_one(&source).expect("the declaration reads");
    let methods: Vec<&str> = declaration
        .calls
        .iter()
        .map(|(method, _)| method.as_str())
        .collect();
    assert_eq!(methods, vec!["under", "named", "params", "needs_registry"]);
}
