//! Gauge folder face: a sibling of Frame under Panel, owning its own Registry.
//! Gauge 文件夹面：Panel 之下与 Frame 平级，拥有自己的 Registry。

#[xirang_toolchain::soil]
pub const SOIL: xirang_toolchain::Soil = xirang_toolchain::Soil::of::<Gauge>()
    .needs_registry(true)
    .under(crate::panel::NODE_ID);

pub struct Gauge;
