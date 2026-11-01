#[xirang_toolchain::soil]
pub const SOIL: xirang_toolchain::Soil = xirang_toolchain::Soil::of::<Reading>()
    .under(crate::xirang_external::native__area_Area::NODE_ID);

pub struct Reading;
