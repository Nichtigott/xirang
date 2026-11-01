//! C spellings for the generated parent constants and the declaration variable.
//! 生成父常量与声明变量的 C 拼法。
//!
//! The build writes `#define XIRANG_FACES_<flat>` for every face module; the authoring adapter
//! renders the same suffix for a parent it is handed, and `naming::c_flat_module` is the one
//! implementation both sides call.
//! 构建为每个面模块写 `#define XIRANG_FACES_<flat>`；创作适配器为收到的父级渲染同一个后缀，而
//! `naming::c_flat_module` 是两边共用的唯一实现。

use crate::languages::naming::c_flat_module;

/// The generated constant naming one face module (`shop::panel` → `XIRANG_FACES_SHOP__PANEL`).
/// 命名一个面模块的生成常量（`shop::panel` → `XIRANG_FACES_SHOP__PANEL`）。
pub fn face_constant(module: &str) -> String {
    format!("XIRANG_FACES_{}", c_flat_module(module))
}

/// The generated constant naming the registry root.
/// 命名注册树根的生成常量。
pub const ROOT_CONSTANT: &str = "XIRANG_ROOT_NODE_ID";

#[cfg(test)]
mod tests {
    use super::*;

    /// The parent constant is the header's own spelling, including the `XIRANG_FACES_` prefix.
    /// 父常量是头文件自己的拼法，含 `XIRANG_FACES_` 前缀。
    #[test]
    fn a_face_constant_is_the_headers_spelling() {
        assert_eq!(face_constant("panel"), "XIRANG_FACES_PANEL");
        assert_eq!(face_constant("shop::panel"), "XIRANG_FACES_SHOP__PANEL");
        assert_eq!(ROOT_CONSTANT, "XIRANG_ROOT_NODE_ID");
    }
}
