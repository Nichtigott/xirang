//! Go spellings for the constants the build generates for a record-file package.
//! 构建为记录文件包生成的常量在 Go 里的拼法。
//!
//! The build writes one `const <name> = "<hex>"` per face it knows, and the authoring adapter
//! renders the same name for a parent it is handed. The two must be one function, not two
//! implementations: a declaration that names a constant the build spells differently does not
//! compile, and the error would point at the author's line rather than at the disagreement.
//! 构建为它知道的每个面写一条 `const <name> = "<hex>"`，创作适配器为收到的父级渲染同一个名字。两者必须是
//! 同一个函数而不是两份实现：声明点名一个构建拼法不同的常量会编译不过，而错误会指向作者那一行而不是
//! 那个分歧。

use crate::languages::naming::go_segment;

/// The Go constant naming one face module (`shop::panel` → `xirangFaceShopPanel`).
/// 命名一个面模块的 Go 常量（`shop::panel` → `xirangFaceShopPanel`）。
///
/// The prefix keeps the generated names from colliding with an author's own identifiers, and each
/// module segment is folded and capitalized so the name is a valid Go identifier whatever the
/// module path contains. An empty module has no face to name and is refused by the caller.
/// 前缀让生成的名字不会撞上作者自己的标识符，每个模块段都被折成并首字母大写，因此无论模块路径包含什么，
/// 名字都是合法的 Go 标识符。空模块没有面可命名，由调用方拒绝。
pub fn face_constant(module: &str) -> String {
    let mut name = String::from("xirangFace");
    for segment in module.split("::") {
        let folded = go_segment(segment);
        let mut characters = folded.chars();
        if let Some(first) = characters.next() {
            name.extend(first.to_uppercase());
            name.push_str(characters.as_str());
        }
    }
    name
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The name is one deterministic spelling for a module and its segments.
    /// 名字对一个模块及其各段是唯一确定的拼法。
    #[test]
    fn a_face_constant_is_one_spelling_per_module() {
        assert_eq!(face_constant("panel"), "xirangFacePanel");
        assert_eq!(face_constant("shop::panel"), "xirangFaceShopPanel");
        assert_eq!(face_constant("my-shop::panel"), "xirangFaceMy_shopPanel");
        assert_eq!(face_constant("type"), "xirangFaceType_");
    }
}
