//! Button：纯 Rust 叶子面。
//! Button: a plain Rust leaf face.

#[xirang_toolchain::soil]
pub const SOIL: xirang_toolchain::Soil = xirang_toolchain::Soil::of::<Button>()
    .under(crate::shop::panel::NODE_ID);

pub struct Button;
