//! 这个宿主声明的 crate 形状：`panel` 子树自成一个 crate，宿主留下 Shop 与 C++ 的 Area。
//! The crate shape this host declares: the `panel` subtree becomes a crate of its own, while the
//! host keeps Shop and the C++ Area.

use xirang_toolchain::run_method::{Crate, Shape};

pub fn add_crates() -> Shape {
    Shape::of("shop", &[Crate::named("panel").at(&[crate::shop::panel::SUBTREE])])
}
