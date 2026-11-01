//! Frame folder face: a child of Panel that owns its own Registry, so it can be a crate of its own.
//! Frame 文件夹面：Panel 的子面，拥有自己的 Registry，因此可以自成一个 crate。
//!
//! It imports nothing from the parent face: once this subtree is handed to a crate of its own, the
//! parent exists only as a generated ancestor shell, so a fragment that reached into it would not
//! compile. Self-containment is what keeps the split a build-shape change rather than a rewrite.
//! 它不从父面导入任何东西：这棵子树一旦交给自己的 crate，父面就只剩下生成的祖先壳，引用它的碎片编译不过。
//! 自包含正是"划分只是构建形状的改变、不是重写"的原因。

#[xirang_toolchain::soil]
pub const SOIL: xirang_toolchain::Soil = xirang_toolchain::Soil::of::<Frame>()
    .needs_registry(true)
    .under(crate::panel::NODE_ID);

pub struct Frame;
