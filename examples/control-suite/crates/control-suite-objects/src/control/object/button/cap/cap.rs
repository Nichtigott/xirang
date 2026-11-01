//! Cap leaf face: a child of Button, one level deeper than the grafted faces.
//! Cap 叶子面：Button 的子对象，比被 graft 的那些面再深一层。

#[xirang_toolchain::soil]
pub const SOIL: xirang_toolchain::Soil = xirang_toolchain::Soil::of::<Cap>()
    .under(crate::control::object::button::NODE_ID);

pub struct Cap;
