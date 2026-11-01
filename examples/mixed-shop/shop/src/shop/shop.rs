//! Shop：这个示例的根文件夹面，拥有一个 Registry。
//! Shop: the example's root folder face; it owns a Registry.

#[xirang_toolchain::soil]
pub const SOIL: xirang_toolchain::Soil = xirang_toolchain::Soil::of::<Shop>()
    .needs_registry(true)
    .under(crate::root_node_id(crate::XIRANG_NAMESPACE));

pub struct Shop;
