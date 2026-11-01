//! The module-rename rewrite: a `::` boundary for module paths, and a path
//! boundary for `src/<module>/`.
//! 模块改名的重写：模块路径以 `::` 为界，`src/<module>/` 以路径为界。

use super::{replace_module_path, replace_source_prefix};

/// A sibling whose name starts with the old one is left alone; the renamed
/// module's own references move.
/// 名字以旧名开头的兄弟模块原样保留；被改名模块自己的引用才会移动。
///
/// Red before the fix: `replace_module_path` did not exist and the migration used
/// `str::replace`, which produced `crate::run_method::widget_extra::NODE_ID` here — a module
/// pointing at a path nothing declares, written silently as "changed".
/// 修前为红：`replace_module_path` 还不存在，迁移用的是 `str::replace`，在这条输入上产出
/// `crate::run_method::widget_extra::NODE_ID`——一个指向无人声明的路径的模块，还被静默写成"已改变"。
#[test]
fn a_sibling_module_is_not_the_module_being_renamed() {
    assert_eq!(
        replace_module_path(
            "parent: crate::control_extra::NODE_ID",
            "crate::control",
            "crate::widget"
        ),
        "parent: crate::control_extra::NODE_ID",
        "a longer name that starts with the old one is a different module"
    );
    assert_eq!(
        replace_module_path(
            "parent: crate::control::NODE_ID",
            "crate::control",
            "crate::widget"
        ),
        "parent: crate::widget::NODE_ID"
    );
    // Both sides are boundaries: a name that merely *ends* with the old one is
    // also a different module.
    assert_eq!(
        replace_module_path(
            "crate::my_control::NODE_ID",
            "crate::control",
            "crate::widget"
        ),
        "crate::my_control::NODE_ID"
    );
}

/// Every occurrence moves, not only the first, and text that does not contain the
/// path is untouched.
/// 每一处都会移动，而不只是第一处；不含该路径的文本保持原样。
#[test]
fn every_segment_boundary_occurrence_moves() {
    let text = "a: crate::control::NODE_ID,\nb: crate::control::NODE_ID,\nc: crate::control_x,\n";
    assert_eq!(
        replace_module_path(text, "crate::control", "crate::widget"),
        "a: crate::widget::NODE_ID,\nb: crate::widget::NODE_ID,\nc: crate::control_x,\n"
    );
    assert_eq!(
        replace_module_path("nothing to do here", "crate::control", "crate::widget"),
        "nothing to do here"
    );
}

/// The renamed face's own generated alias moves with it; a sibling's does not.
/// 被改名面自己的生成别名随它移动；兄弟模块的别名不动。
///
/// This is the case a strict `::`-only rule gets wrong: `crate::run_method::test_object!` in a
/// child face has to become `crate::run_method::panel_object!` when the parent module is
/// renamed, or the parse refuses the file by name ("registration macro
/// `test_object!` does not match parent `panel`"). The `!` is part of the match,
/// which is what keeps the longer sibling name `control_extra_object!` untouched.
/// 这正是只认 `::` 的严格规则会弄错的那种情形：父模块改名时，子面里的 `crate::run_method::test_object!`
/// 必须变成 `crate::run_method::panel_object!`，否则解析会按名拒绝该文件（`registration macro
/// \`test_object!\` does not match parent \`panel\``）。`!` 是匹配的一部分，正是它让更长的兄弟名
/// `control_extra_object!` 保持原样。
#[test]
fn the_renamed_faces_alias_moves_and_a_siblings_does_not() {
    assert_eq!(
        replace_module_path("crate::test_object! {\n", "crate::test", "crate::panel"),
        "crate::panel_object! {\n"
    );
    assert_eq!(
        replace_module_path(
            "crate::control_extra_object! {\n",
            "crate::control",
            "crate::widget"
        ),
        "crate::control_extra_object! {\n"
    );
}

/// The source-prefix half is bounded on the left: a path that merely *contains*
/// `src/control/` belongs to a different tree and stays as it is, while a real
/// reference to the moved module follows it.
/// 源前缀那一半以左侧为界：只是**包含** `src/control/` 的路径属于另一棵树，原样保留；真正指向被
/// 移动模块的引用才跟着移动。
///
/// This is the half the `::` rule cannot cover: `old_prefix` is a path
/// (`src/<old>/`), so its far bound is the trailing `/` — which is why a sibling
/// named `control_extra` was never at risk here — and its near bound is the byte in
/// front of it. A bare `str::replace` had no near bound, so a sibling pointing into
/// another tree (`dep/src/control/x.rs`) was rewritten silently (audit `LGC-LG-10`).
/// 这正是 `::` 规则覆盖不到的那一半：`old_prefix` 是路径（`src/<旧目录>/`），因此远端界是结尾的
/// `/`——这也是名为 `control_extra` 的兄弟在这里从来不受影响的原因——而近端界是它前面的那个字节。
/// 朴素的 `str::replace` 没有近端界，于是指向另一棵树的兄弟引用（`dep/src/control/x.rs`）会被静默
/// 改写（审计 `LGC-LG-10`）。
#[test]
fn a_containing_path_is_not_the_moved_source_prefix() {
    assert_eq!(
        replace_source_prefix(
            "include_str!(\"dep/src/control/x.rs\")",
            "src/control/",
            "src/widget/"
        ),
        "include_str!(\"dep/src/control/x.rs\")",
        "a path that only contains the prefix belongs to another tree"
    );
    assert_eq!(
        replace_source_prefix(
            "registry_rule_path: \"src/control/button/button.rs\",",
            "src/control/",
            "src/widget/"
        ),
        "registry_rule_path: \"src/widget/button/button.rs\","
    );
    // At the start of the text there is no byte in front of it, which is the other
    // half of the boundary rule.
    // 文本开头没有前一个字节，这是界规则的另一半。
    assert_eq!(
        replace_source_prefix(
            "src/control/button/button.rs",
            "src/control/",
            "src/widget/"
        ),
        "src/widget/button/button.rs"
    );
    // The trailing `/` is the far bound, so a longer sibling name was never at risk
    // from this half. This row is documentation, not a discriminator: the plain
    // `str::replace` this replaced already left it alone.
    // 结尾的 `/` 是远端界，因此更长的兄弟名从不受这一半影响。这一行是文档而不是判别项：它替换掉的
    // 那个朴素 `str::replace` 本来就放过它。
    assert_eq!(
        replace_source_prefix("src/control_extra/x.rs", "src/control/", "src/widget/"),
        "src/control_extra/x.rs"
    );
}
