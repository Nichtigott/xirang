//! Shop：注册树根下的 Rust 面，拥有注册机；Panel 挂在它下面，是分区要搬走的子树。
//! Shop: the Rust face under the registry root that owns a registry; Panel hangs under it and is the
//! subtree the partition moves.

#[xirang_toolchain::soil]
pub const SOIL: xirang_toolchain::Soil = xirang_toolchain::Soil::of::<Shop>()
    .needs_registry(true)
    .under(crate::root_node_id(crate::XIRANG_NAMESPACE));

pub struct Shop;
