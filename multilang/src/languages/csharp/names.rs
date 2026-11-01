//! C# spellings for the constants the build generates for a record-file package.
//! 构建为记录文件包生成的常量在 C# 里的拼法。
//!
//! The build writes one `public const string <NAME>` per face module and the authoring adapter
//! renders the same name for a parent it is handed. The two are one function, not two
//! implementations: an attribute argument must be a compile-time constant, and a spelling that
//! disagrees with the generated class fails the user's compile at the declaration line.
//! 构建为每个面模块写一条 `public const string <NAME>`，创作适配器为收到的父级渲染同一个名字。两者是同一个
//! 函数而不是两份实现：特性实参必须是编译期常量，而与生成类不一致的拼法会在用户声明的行上编译失败。

/// The C# constant naming one face module (`shop::panel` → `ShopPanelNodeId`).
/// 命名一个面模块的 C# 常量（`shop::panel` → `ShopPanelNodeId`）。
///
/// The root is spelled `RootNodeId` by the build; a module folding to the same constant is refused
/// by the caller, because one class cannot hold two fields with one name.
/// 根由构建拼作 `RootNodeId`；折成同一个常量的模块会被调用方拒绝，因为一个类不能有两个同名字段。
pub fn face_constant(module: &str) -> String {
    let mut name = String::new();
    for segment in module.split("::") {
        let folded = crate::languages::naming::csharp_segment(segment);
        let mut characters = folded.chars();
        if let Some(first) = characters.next() {
            name.extend(first.to_uppercase());
            name.push_str(characters.as_str());
        }
    }
    name.push_str("NodeId");
    name
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The name is one deterministic spelling per module, PascalCase with `NodeId`.
    /// 名字对每个模块是唯一确定的拼法，首字母大写并加 `NodeId`。
    #[test]
    fn a_face_constant_is_one_spelling_per_module() {
        assert_eq!(face_constant("panel"), "PanelNodeId");
        assert_eq!(face_constant("shop::panel"), "ShopPanelNodeId");
        assert_eq!(face_constant("my-panel"), "My_panelNodeId");
    }
}
