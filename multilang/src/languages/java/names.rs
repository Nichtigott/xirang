//! Java spellings for the constants the build generates for a record-file package.
//! 构建为记录文件包生成的常量在 Java 里的拼法。
//!
//! The build writes one `public static final String <NAME> = "<hex>"` per face module and the
//! authoring adapter renders the same name for a parent it is handed. The two are one function, not
//! two implementations: an annotation element must be a compile-time constant, and a spelling that
//! disagrees with the generated class fails the user's compile at the declaration line.
//! 构建为每个面模块写一条 `public static final String <NAME> = "<hex>"`，创作适配器为收到的父级渲染
//! 同一个名字。两者是同一个函数而不是两份实现：注解元素必须是编译期常量，而与生成类不一致的拼法会在用户
//! 声明的行上编译失败。

/// The Java constant naming one face module (`shop::panel` → `SHOP_PANEL_NODE_ID`).
/// 命名一个面模块的 Java 常量（`shop::panel` → `SHOP_PANEL_NODE_ID`）。
///
/// The root is spelled `ROOT_NODE_ID` by the build; a module folding to the same constant is refused
/// by the caller, because one class cannot hold two fields with one name.
/// 根由构建拼作 `ROOT_NODE_ID`；折成同一个常量的模块会被调用方拒绝，因为一个类不能有两个同名字段。
pub fn face_constant(module: &str) -> String {
    let mut name = String::new();
    for segment in module.split("::") {
        if !name.is_empty() {
            name.push('_');
        }
        for character in segment.chars() {
            if character.is_ascii_alphanumeric() {
                name.push(character.to_ascii_uppercase());
            } else {
                name.push('_');
            }
        }
    }
    name.push_str("_NODE_ID");
    name
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The name is one deterministic spelling per module, uppercased with `_NODE_ID`.
    /// 名字对每个模块是唯一确定的拼法，全大写并加 `_NODE_ID`。
    #[test]
    fn a_face_constant_is_one_spelling_per_module() {
        assert_eq!(face_constant("panel"), "PANEL_NODE_ID");
        assert_eq!(face_constant("shop::panel"), "SHOP_PANEL_NODE_ID");
        assert_eq!(face_constant("my-panel"), "MY_PANEL_NODE_ID");
    }
}
