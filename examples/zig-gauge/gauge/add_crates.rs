//! 这个宿主声明的 crate 形状：`panel` 子树（连同它下面的 Zig 面）自成一个 crate。
//! The crate shape this host declares: the `panel` subtree (with the Zig face under it) becomes a
//! crate of its own.

use xirang_toolchain::run_method::{Crate, Shape};

pub fn add_crates() -> Shape {
    Shape::of("gauge", &[Crate::named("panel").at(&[crate::shop::panel::SUBTREE])])
}
