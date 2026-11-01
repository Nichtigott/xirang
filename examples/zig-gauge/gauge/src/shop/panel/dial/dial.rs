//! Dial：挂在 Zig 声明的 Gauge 下的 Rust 面。
//! Dial: a Rust face hanging under the Zig-declared Gauge.

#[xirang_toolchain::soil]
pub const SOIL: xirang_toolchain::Soil = xirang_toolchain::Soil::of::<Dial>()
    .under(crate::xirang_external::native__gauge_Gauge::NODE_ID);

pub struct Dial;
