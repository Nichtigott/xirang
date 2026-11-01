//! Widget leaf face: a child of Frame, inside the `frames` crate.
//! Widget 叶子面：Frame 的子对象，属于 `frames` 这个 crate。

#[xirang_toolchain::soil]
pub const SOIL: xirang_toolchain::Soil = xirang_toolchain::Soil::of::<Widget>()
    .under(crate::panel::frame::NODE_ID);

pub struct Widget;
