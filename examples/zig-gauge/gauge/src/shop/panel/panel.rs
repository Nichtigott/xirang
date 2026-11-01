//! Panel：注册树根下的 Rust 面，拥有注册机；Zig 声明的 Gauge 挂在它下面。
//! Panel: a Rust face under the registry root that owns a registry; the Zig-declared Gauge hangs
//! under it.

#[xirang_toolchain::soil]
pub const SOIL: xirang_toolchain::Soil = xirang_toolchain::Soil::of::<Panel>()
    .needs_registry(true)
    .under(crate::shop::NODE_ID);

pub struct Panel;
