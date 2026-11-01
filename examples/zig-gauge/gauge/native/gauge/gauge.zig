// 面在 Zig 这一侧声明自己：父面用生成模块里的常量点名，写错就是编译错误而不是挂错树。
// The face declares itself on the Zig side: the parent is named by a constant from the generated
// module (the Rust `Panel`'s module is `shop::panel`), so a wrong name is a compile error rather than a
// wrong tree.
const xirang = @import("xirang");
const faces = @import("xirang_faces");

pub export var SOIL linksection(xirang.SECTION) = xirang.record(.{
    .kind = "Gauge",
    .parent = faces.shop__panel,
    .name_en = "Gauge",
    .exports = "gauge",
    // 这个面拥有子面，因此 Rust 的 Dial 可以挂在它下面：跨语言父链的方向是 Zig 父、Rust 子。
    // The face owns children, so the Rust Dial can hang under it: the cross-language chain here is
    // Zig parent, Rust child.
    .owns_registry = true,
});
