//! Panel：Shop 的子面，自己也拥有 Registry；Button 与 Area 都挂在它下面。
//! Panel: a child of Shop that owns its own Registry; Button and Area hang under it.

#[xirang_toolchain::soil]
pub const SOIL: xirang_toolchain::Soil = xirang_toolchain::Soil::of::<Panel>()
    .needs_registry(true)
    .under(crate::shop::NODE_ID);

pub struct Panel;
