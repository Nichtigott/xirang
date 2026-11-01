//! The text half of the declared-native reader: what a source file says before it compiles.
//! 声明式本地源码读取器的文本那一半：一份源码在编译之前说了什么。
//!
//! Both spellings are read here — the version-1 `XIRANG_FACE(record, (expression))` and the marked
//! `XIRANG_SOIL` + `constexpr auto SOIL = …` declaration — and only the facts the build needs before
//! compiling: the parent, the name, and whether the face keeps a registry.
//! 这里读两种拼法——版本 1 的 `XIRANG_FACE(record, (expression))` 与带标记的 `XIRANG_SOIL` +
//! `constexpr auto SOIL = …` 声明——并且只读构建在编译前需要的事实：父、名字、以及该面是否自持注册机。

use crate::build_method::registry_identity::NodeId;

use super::{DeclaredFace, ident, module_of};

/// The face one parent spelling names: a flat identifier, or a nested `module::NODE_ID`.
/// 一种父拼法所点名的面：扁平标识符，或嵌套的 `module::NODE_ID`。
///
/// The nested spelling is looked up by module because it deliberately does **not** encode the kind:
/// `xirang::faces::shop::panel::NODE_ID` is the panel face there, and flattening the path by
/// concatenating a kind would be the encoding this spelling removes.
/// 嵌套拼法按**模块**查找，因为它有意不编码种类名：`xirang::faces::shop::panel::NODE_ID` 就是那里的
/// panel 面，而把路径连上种类名再拼成扁平标识符，正是这种拼法要消灭的编码。
pub(super) fn lookup<'map>(
    known: &'map std::collections::BTreeMap<String, (String, NodeId)>,
    spelling: &str,
) -> Option<&'map (String, NodeId)> {
    match spelling.strip_suffix("::NODE_ID") {
        Some(module) => known
            .values()
            .find(|(known_module, _)| known_module == module),
        None => known.get(spelling),
    }
}

/// Whether `spelling` names this declaration, in either of the two accepted forms.
/// `spelling` 是否点名这条声明——两种可接受拼法都算。
pub(super) fn names_face(declared: &DeclaredFace, spelling: &str) -> bool {
    if ident(&declared.source, &declared.name) == spelling {
        return true;
    }
    match spelling.strip_suffix("::NODE_ID") {
        Some(module) => module == module_of(&declared.source),
        None => false,
    }
}

/// Whether the parent this declaration names lives inside that subtree.
/// 这条声明点名的父是否住在那棵子树里。
pub(super) fn under(declared: &DeclaredFace, claim: &str) -> bool {
    // Two parent spellings reach this function. The version-1 flat identifier is
    // `shop__panel_<Kind>`, so a claim covers it when it starts with `shop__panel_`; the trailing
    // underscore is what keeps `shop::pa` from swallowing `shop::panel`. The nested spelling is the
    // module path itself (`shop::panel::NODE_ID`), so a claim covers it when the module path is the
    // claim or begins with it.
    // 这个函数会收到两种父拼法。版本 1 的扁平标识符是 `shop__panel_<Kind>`，因此父以 `shop__panel_`
    // 开头时被认领覆盖；末尾那道下划线让 `shop::pa` 不会吞掉 `shop::panel`。嵌套拼法是模块路径本身
    // （`shop::panel::NODE_ID`），因此模块路径等于认领或以认领开头时被覆盖。
    let spelling = claim.replace("::", "__");
    // A Zig declaration names the parent by its module constant, which is the claim's flat spelling
    // itself; a flat face identifier is that spelling plus `_<Kind>`.
    // Zig 声明按模块常量点名父级，那正是认领的扁平拼法本身；扁平的面标识符则是它加 `_<Kind>`。
    if declared.parent == spelling || declared.parent.starts_with(&format!("{spelling}_")) {
        return true;
    }
    match declared.parent.strip_suffix("::NODE_ID") {
        Some(module) => module == claim || module.starts_with(&format!("{claim}::")),
        None => false,
    }
}
