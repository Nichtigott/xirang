//! Pins for the pure reference-rewrite planners.
//! 纯引用改写规划器的钉子。
//!
//! Every test plans against literal source and then applies the plan to that same
//! text, so the assertion is about the rewritten Rust a reader would get — never
//! about a byte range by itself.
//! 每条测试都针对字面源码规划，再把计划应用到同一份文本，因此断言针对的是读者会得到的改写后 Rust，
//! 而不是孤立的字节区间。

use super::{ModuleMove, plan_reference_rewrites};
use crate::syntax::apply_text_edits;

fn moved(from: &str, to: &str) -> ModuleMove {
    ModuleMove {
        from: from.to_owned(),
        to: to.to_owned(),
    }
}

/// The real prior failure: a crate-root `pub use … as button_api` must follow the
/// moved module and keep its alias.
/// 真实的历史失败：crate 根上的 `pub use … as button_api` 必须跟随被搬模块并保住别名。
#[test]
fn a_crate_root_reexport_follows_the_move_and_keeps_its_alias() {
    let source = "pub use crate::control::object::button as button_api;\n";
    let plan = plan_reference_rewrites(
        source,
        &[moved(
            "control::object::button",
            "control::object::panel::button",
        )],
        false,
    )
    .expect("a use tree parses");
    assert!(plan.unresolved.is_empty(), "{:?}", plan.unresolved);
    let rewritten = apply_text_edits(source, &plan.edits);
    assert_eq!(
        rewritten, "pub use crate::control::object::panel::button as button_api;\n",
        "the alias name is preserved and only the module path moved"
    );
}

/// A bare import is only rewritten at a crate root, where it resolves inside the
/// crate; elsewhere the first segment is an external crate and must be left alone.
/// 裸导入只在 crate 根改写（在那里它解析到 crate 内部）；其它位置首段是外部 crate，必须保持不动。
#[test]
fn a_bare_import_is_only_rewritten_at_a_crate_root() {
    let source = "pub use control::object::button as button_api;\n";
    let moves = [moved(
        "control::object::button",
        "control::object::panel::button",
    )];

    let at_root = plan_reference_rewrites(source, &moves, true).expect("parses");
    assert_eq!(
        apply_text_edits(source, &at_root.edits),
        "pub use control::object::panel::button as button_api;\n"
    );

    let elsewhere = plan_reference_rewrites(source, &moves, false).expect("parses");
    assert!(
        elsewhere.edits.is_empty(),
        "a bare path away from the crate root could be an external crate: {:?}",
        elsewhere.edits
    );
}

/// A grouped import rewrites only the group base or the item's own tail, never the
/// braces or the sibling items.
/// 分组导入只改写分组基准或条目自己的尾部，绝不改花括号或兄弟条目。
#[test]
fn a_grouped_import_rewrites_only_the_changed_tail() {
    let base_move = "use crate::control::object::{button, slider};\n";
    let item = plan_reference_rewrites(
        base_move,
        &[moved(
            "control::object::button",
            "control::object::panel::button",
        )],
        false,
    )
    .expect("parses");
    assert_eq!(
        apply_text_edits(base_move, &item.edits),
        "use crate::control::object::{panel::button, slider};\n"
    );

    let group_base = plan_reference_rewrites(
        base_move,
        &[moved("control::object", "control::moved")],
        false,
    )
    .expect("parses");
    assert_eq!(
        apply_text_edits(base_move, &group_base.edits),
        "use crate::control::moved::{button, slider};\n",
        "a match entirely in the brace base replaces that base once"
    );
}

/// Typed and expression paths are explicit references too, and they follow the move.
/// 类型路径与表达式路径同样是显式引用，也跟随搬动。
#[test]
fn typed_and_expression_paths_follow_the_move() {
    let source = "pub fn build() -> crate::control::object::button::Button {\n    \
                  crate::control::object::button::Button::new()\n}\n";
    let plan = plan_reference_rewrites(
        source,
        &[moved(
            "control::object::button",
            "control::object::panel::button",
        )],
        false,
    )
    .expect("parses");
    let rewritten = apply_text_edits(source, &plan.edits);
    assert_eq!(
        rewritten,
        "pub fn build() -> crate::control::object::panel::button::Button {\n    \
         crate::control::object::panel::button::Button::new()\n}\n"
    );
}

/// Identical text in a comment or an ordinary string is not a reference and is not
/// rewritten; only the parsed use item changes.
/// 注释或普通字符串里相同的文本不是引用，不改写；只有已解析的 use 条目改变。
#[test]
fn ordinary_strings_and_comments_are_not_touched() {
    let source = "// pub use crate::control::object::button as button_api;\n\
                  pub const NOTE: &str = \"crate::control::object::button\";\n\
                  pub use crate::control::object::button as button_api;\n";
    let plan = plan_reference_rewrites(
        source,
        &[moved(
            "control::object::button",
            "control::object::panel::button",
        )],
        false,
    )
    .expect("parses");
    let rewritten = apply_text_edits(source, &plan.edits);
    assert_eq!(
        rewritten,
        "// pub use crate::control::object::button as button_api;\n\
         pub const NOTE: &str = \"crate::control::object::button\";\n\
         pub use crate::control::object::panel::button as button_api;\n"
    );
}

/// A relative `super::`/`self::` import that could name the moved module cannot be
/// resolved lexically, so it is reported rather than guessed.
/// 可能命名了被搬模块的相对 `super::`/`self::` 导入无法在词法层解析，因此报告而不是猜测。
#[test]
fn relative_imports_that_could_name_the_move_are_reported() {
    let source = "use super::button::run;\n";
    let plan = plan_reference_rewrites(
        source,
        &[moved(
            "control::object::button",
            "control::object::panel::button",
        )],
        false,
    )
    .expect("parses");
    assert!(plan.edits.is_empty(), "{:?}", plan.edits);
    assert_eq!(plan.unresolved.len(), 1, "{:?}", plan.unresolved);
    assert!(
        plan.unresolved[0].contains("super::") && plan.unresolved[0].contains("button"),
        "the obligation names the spelling and the module: {:?}",
        plan.unresolved
    );
}

/// The supervisor's third boundary: a path nested in a generic argument is a
/// reference too. Skipping nested positions left `Option<crate::…>` silently stale
/// while the top-level paths in the same file were rewritten.
/// 监督者的第三条边界：嵌在泛型实参里的路径同样是引用。跳过嵌套位置会让 `Option<crate::…>` 静默过时，
/// 而同一文件里顶层的路径却改了。
#[test]
fn a_path_nested_in_a_generic_argument_is_rewritten() {
    let source = "type X = Option<crate::control::button::Button>;\n";
    let plan = plan_reference_rewrites(
        source,
        &[moved("control::button", "control::panel::button")],
        false,
    )
    .expect("parses");
    assert!(plan.unresolved.is_empty(), "{:?}", plan.unresolved);
    assert_eq!(
        apply_text_edits(source, &plan.edits),
        "type X = Option<crate::control::panel::button::Button>;\n"
    );
}

/// Nested positions of every kind: turbofish arguments, `QSelf`, trait bounds and an
/// `impl Trait` return type. Each is a path the move invalidates.
/// 各种嵌套位置：turbofish 实参、`QSelf`、trait 约束，以及 `impl Trait` 返回类型。每一处都是搬动会让它
/// 失效的路径。
#[test]
fn nested_generic_qself_and_trait_positions_are_all_rewritten() {
    let source = "fn f() -> impl Iterator<Item = <crate::control::button::B as crate::control::button::T>::Out> {\n    \
                  let _ = crate::control::button::make::<crate::control::button::B>();\n    \
                  std::iter::empty()\n}\n";
    let plan = plan_reference_rewrites(
        source,
        &[moved("control::button", "control::panel::button")],
        false,
    )
    .expect("parses");
    let rewritten = apply_text_edits(source, &plan.edits);
    assert!(
        !rewritten.contains("crate::control::button"),
        "every nested position follows the move: {rewritten}"
    );
    assert_eq!(
        rewritten.matches("crate::control::panel::button").count(),
        4,
        "the turbofish, the QSelf halves and the impl-Trait bound all moved: {rewritten}"
    );
}

/// The supervisor's fourth boundary: a nested inline module that declares its own
/// `control` makes the bare spelling point inside *that* module. Rewriting it as the
/// crate-root module compiles something the author never wrote.
/// 监督者的第四条边界：内联模块自己声明了 `control` 时，裸拼法指向**那个**模块内部。把它当成 crate 根
/// 模块改写，会编译出作者从未写过的代码。
#[test]
fn a_nested_module_that_declares_the_head_keeps_its_own_bare_path() {
    let source = "mod local { mod control { pub mod button { pub const N: u32 = 1; } } \
                  fn f() -> u32 { control::button::N } }\n";
    let plan = plan_reference_rewrites(
        source,
        &[moved("control::button", "control::panel::button")],
        true,
    )
    .expect("parses");
    assert!(plan.edits.is_empty(), "{:?}", plan.edits);
    assert_eq!(apply_text_edits(source, &plan.edits), source);
}

/// The supervisor's fifth boundary: a function-local `use … as control` rebinds the
/// head for that block, so the call below it is not a reference to the crate module.
/// 监督者的第五条边界：函数内的 `use … as control` 为那个块重新绑定了首段，因此它下面的调用不是对
/// crate 模块的引用。
#[test]
fn a_function_local_alias_hides_the_bare_head_for_its_block() {
    let source = "fn f() { use crate::other as control; control::button::call(); }\n\
                  fn g() { crate::control::button::call(); }\n";
    let plan = plan_reference_rewrites(
        source,
        &[moved("control::button", "control::panel::button")],
        true,
    )
    .expect("parses");
    let rewritten = apply_text_edits(source, &plan.edits);
    assert_eq!(
        rewritten,
        "fn f() { use crate::other as control; control::button::call(); }\n\
         fn g() { crate::control::panel::button::call(); }\n",
        "the shadowed call keeps its bytes; the crate-qualified one follows the move"
    );
}

/// A `use crate::control;` at the crate root binds the head to the moved module
/// itself, so bare uses below it are still ours.
/// crate 根上的 `use crate::control;` 把首段绑定到被搬模块本身，因此它下面的裸用法仍然是我们的。
#[test]
fn a_root_import_of_the_ancestor_keeps_bare_uses_in_scope() {
    let source = "use crate::control;\nfn f() -> control::button::B { control::button::B }\n";
    let plan = plan_reference_rewrites(
        source,
        &[moved("control::button", "control::panel::button")],
        true,
    )
    .expect("parses");
    let rewritten = apply_text_edits(source, &plan.edits);
    assert!(rewritten.contains("use crate::control;"), "{rewritten}");
    assert_eq!(rewritten.matches("control::panel::button").count(), 2);
}

/// The supervisor's sixth boundary: `::control::…` is an external crate in 2018 and
/// the crate root in 2015. The planner reports the ambiguity instead of picking one.
/// 监督者的第六条边界：`::control::…` 在 2018 是外部 crate、在 2015 是 crate 根。规划器报出这个两可，
/// 而不是挑一种。
#[test]
fn an_absolute_expression_path_is_reported_not_rewritten() {
    let source = "fn f() { ::control::button::call(); }\n";
    let plan = plan_reference_rewrites(
        source,
        &[moved("control::button", "control::panel::button")],
        true,
    )
    .expect("parses");
    assert!(plan.edits.is_empty(), "{:?}", plan.edits);
    assert_eq!(plan.unresolved.len(), 1, "{:?}", plan.unresolved);
    assert!(
        plan.unresolved[0].contains("external crate"),
        "{:?}",
        plan.unresolved
    );
}

/// An absolute path that cannot name the moved module is not noise in the obligation
/// list — only a real candidate is reported.
/// 不可能命名被搬模块的绝对路径不会成为义务清单里的噪音——只有真正的候选才报告。
#[test]
fn an_unrelated_absolute_path_is_not_reported() {
    let source = "fn f() { ::std::mem::size_of::<u8>(); }\n";
    let plan = plan_reference_rewrites(
        source,
        &[moved("control::button", "control::panel::button")],
        true,
    )
    .expect("parses");
    assert!(plan.is_empty(), "{:?}", plan);
}

/// A macro body that spells the moved module is an obligation; an unrelated macro,
/// and a string literal that merely contains the spelling, are not.
/// 拼出被搬模块的宏体是义务；无关的宏、以及只是包含该拼法的字符串字面量，都不是。
#[test]
fn a_macro_that_spells_the_moved_module_is_reported() {
    let moves = [moved("control::button", "control::panel::button")];
    let reported = plan_reference_rewrites(
        "macro_rules! m { () => { control::button::call() } }\n",
        &moves,
        true,
    )
    .expect("parses");
    assert_eq!(reported.unresolved.len(), 1, "{:?}", reported.unresolved);

    let quiet = plan_reference_rewrites("fn f() { format!(\"control::button\"); }\n", &moves, true)
        .expect("parses");
    assert!(quiet.is_empty(), "{:?}", quiet);

    let unrelated =
        plan_reference_rewrites("fn f() { println!(\"hi\"); }\n", &moves, true).expect("parses");
    assert!(unrelated.is_empty(), "{:?}", unrelated);
}

/// A `#[cfg]`-gated reference is still parsed, so both branches of a configuration are
/// rewritten: leaving one behind would break exactly the feature nobody builds.
/// 门控在 `#[cfg]` 之后的引用仍会被解析，因此两种配置分支都被改写：漏掉一个，恰好会弄坏没人构建的那个特性。
#[test]
fn cfg_gated_branches_are_both_rewritten() {
    let source = "#[cfg(feature = \"a\")]\nfn a() -> crate::control::button::B { todo!() }\n\
                  #[cfg(not(feature = \"a\"))]\nfn b() -> crate::control::button::B { todo!() }\n";
    let plan = plan_reference_rewrites(
        source,
        &[moved("control::button", "control::panel::button")],
        false,
    )
    .expect("parses");
    let rewritten = apply_text_edits(source, &plan.edits);
    assert_eq!(
        rewritten.matches("crate::control::panel::button").count(),
        2
    );
}
