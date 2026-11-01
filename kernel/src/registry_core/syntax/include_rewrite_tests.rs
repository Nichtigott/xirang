//! Pins for the pure `include_*!` path planner.
//! 纯 `include_*!` 路径规划器的钉子。
//!
//! Every case plans against literal source and applies the plan to that same text, so
//! the assertion is about the rewritten Rust a reader would get. The three shapes that
//! matter are: the asset moves with the file, the asset stays while the file moves, and
//! neither moves.
//! 每个用例都针对字面源码规划、再把计划应用到同一份文本，因此断言针对的是读者会得到的改写后 Rust。
//! 要紧的三种形状是：资产随文件一起搬、资产不动而文件搬、两者都不动。

use super::plan_include_rewrites;
use crate::syntax::apply_text_edits;

/// A file-relative `include_str!` into the moved directory is rewritten with path
/// semantics, while the surrounding assertion is preserved.
/// 指向被搬目录、相对文件的 `include_str!` 按路径语义改写，周围的断言得到保留。
#[test]
fn include_str_into_the_moved_directory_is_rewritten() {
    let source =
        "const DATA: &str = include_str!(\"../button/data.txt\");\nassert!(DATA == \"x\");\n";
    let plan = plan_include_rewrites(
        source,
        "control/object/slider",
        "control/object/button",
        "control/object/panel/button",
    )
    .expect("parses");
    let rewritten = apply_text_edits(source, &plan.edits);
    assert_eq!(
        rewritten,
        "const DATA: &str = include_str!(\"../panel/button/data.txt\");\nassert!(DATA == \"x\");\n"
    );
}

/// An include path that points outside the moved directory is left untouched.
/// 指向被搬目录之外的 include 路径保持不动。
#[test]
fn include_str_outside_the_move_is_untouched() {
    let source = "const DATA: &str = include_str!(\"../slider/data.txt\");\n";
    let plan = plan_include_rewrites(
        source,
        "control/object/slider",
        "control/object/button",
        "control/object/panel/button",
    )
    .expect("parses");
    assert!(plan.edits.is_empty(), "{:?}", plan.edits);
}

/// The supervisor's first file-reference boundary: when the including file moves and
/// the asset moves with it, the relative spelling does not change — and in particular
/// the planner must not resolve the old literal against the file's *new* directory.
/// 监督者的第一条文件引用边界：包含文件搬动、资产随之搬动时，相对拼法不变——尤其是规划器不得拿旧字面量
/// 去对着文件的**新**目录解析。
#[test]
fn an_asset_that_moves_with_its_file_keeps_the_relative_spelling() {
    let source = "const DATA: &str = include_str!(\"data.txt\");\n";
    let plan = plan_include_rewrites(
        source,
        "control/button",
        "control/button",
        "control/panel/button",
    )
    .expect("parses");
    assert!(plan.edits.is_empty(), "{:?}", plan.edits);
}

/// The supervisor's second file-reference boundary: an asset that does **not** move
/// needs one more `..` once its owner gains a directory level. Leaving the bytes
/// alone here is the silent breakage the tool exists to prevent.
/// 监督者的第二条文件引用边界：不搬动的资产，在属主多出一层目录之后需要一个额外的 `..`。在这里保持原
/// 字节不动，正是这个工具存在要防的静默损坏。
#[test]
fn an_unmoved_asset_gains_a_level_when_its_owner_moves_deeper() {
    let source = "const DATA: &str = include_str!(\"../../assets.txt\");\n";
    let plan = plan_include_rewrites(
        source,
        "control/button",
        "control/button",
        "control/panel/button",
    )
    .expect("parses");
    let rewritten = apply_text_edits(source, &plan.edits);
    assert_eq!(
        rewritten,
        "const DATA: &str = include_str!(\"../../../assets.txt\");\n"
    );
}

/// An absolute include names a place outside the identity base: moving the file does
/// not change what it names, and re-spelling it relative would change it.
/// 绝对 include 命名的是身份基准之外的位置：搬动文件不改变它命名的东西，而改写成相对路径会改变它。
#[test]
fn an_absolute_include_is_never_re_spelled_relative() {
    let source = "const DATA: &str = include_str!(\"/control/button/data.txt\");\n";
    let plan = plan_include_rewrites(source, "", "control/button", "control/panel/button")
        .expect("parses");
    assert!(plan.edits.is_empty(), "{:?}", plan.edits);
}

/// A file deeper inside the moved subtree keeps pointing at its sibling asset.
/// 住在被搬子树更深处的一个文件，继续指向它的兄弟资产。
#[test]
fn a_file_inside_the_moved_subtree_keeps_its_sibling_include() {
    let source = "const DATA: &str = include_str!(\"../data.txt\");\n";
    let plan = plan_include_rewrites(
        source,
        "control/button/inner",
        "control/button",
        "control/panel/button",
    )
    .expect("parses");
    assert!(plan.edits.is_empty(), "{:?}", plan.edits);
    // The same file reaching one level further out has to follow the new depth.
    // 同一个文件若指向再往外一层，就必须跟随新的深度。
    let outward = "const DATA: &str = include_str!(\"../../assets.txt\");\n";
    let plan = plan_include_rewrites(
        outward,
        "control/button/inner",
        "control/button",
        "control/panel/button",
    )
    .expect("parses");
    assert_eq!(
        apply_text_edits(outward, &plan.edits),
        "const DATA: &str = include_str!(\"../../../assets.txt\");\n"
    );
}

/// An `include_str!` in a comment or an ordinary string is not an include.
/// 注释或普通字符串里的 `include_str!` 不是 include。
#[test]
fn an_include_inside_a_comment_or_string_is_not_a_reference() {
    let source =
        "// include_str!(\"data.txt\")\nconst S: &str = \"include_str!(\\\"data.txt\\\")\";\n";
    let plan = plan_include_rewrites(
        source,
        "control/button",
        "control/button",
        "control/panel/button",
    )
    .expect("parses");
    assert!(plan.edits.is_empty(), "{:?}", plan.edits);
}

/// A computed `include!(concat!(…))` cannot be checked and becomes an obligation.
/// 计算式的 `include!(concat!(…))` 无法检查，成为未解决义务。
#[test]
fn a_computed_include_is_an_unresolved_obligation() {
    let source = "const DATA: &str = include!(concat!(env!(\"OUT_DIR\"), \"/gen.rs\"));\n";
    let plan = plan_include_rewrites(source, "", "gen", "moved/gen").expect("parses");
    assert!(plan.edits.is_empty(), "{:?}", plan.edits);
    assert_eq!(plan.unresolved.len(), 1, "{:?}", plan.unresolved);
}

/// An `include_str!` naming a path above the identity base has no package-relative
/// resolution, so it is left alone rather than guessed.
/// 命名了身份基准之上路径的 `include_str!` 没有包内相对解析，因此保持不动而不是猜测。
#[test]
fn an_include_above_the_identity_base_is_left_alone() {
    let source = "const DATA: &str = include_str!(\"../../../outside.txt\");\n";
    let plan = plan_include_rewrites(source, "", "control/button", "control/panel/button")
        .expect("parses");
    assert!(plan.edits.is_empty(), "{:?}", plan.edits);
}
