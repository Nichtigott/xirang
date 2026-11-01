//! Needle leaf face: a child of Gauge, inside the `gauges` crate.
//! Needle 叶子面：Gauge 的子对象，属于 `gauges` 这个 crate。

#[xirang_toolchain::soil]
pub const SOIL: xirang_toolchain::Soil = xirang_toolchain::Soil::of::<Needle>()
    .under(crate::panel::gauge::NODE_ID);

pub struct Needle;
