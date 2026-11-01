//! 项目外 crate：它不参与宿主那棵生成树，靠 `external_object!` 显式声明来源，
//! 在自己的命名空间里构建一个注册机，供宿主 overlay 使用。
//! An out-of-project crate: it takes no part in the host's generated tree. It
//! declares its origin explicitly with `external_object!`, builds a registry in
//! its own namespace, and the host overlays it.

use xirang_toolchain::run_method::registry_core::{FrameworkId, Registry};

/// The identity namespace this crate's faces are compiled under.
/// 本 crate 的注册面编译时所用的身份命名空间。
///
/// This crate is an **external implementation**: it declares faces with `external_object!` and never
/// calls `host!()`, so it owns this constant itself. It is what the declaration macros read now
/// (audit `M7`, P3.3) — one place per crate, instead of an `env!` read at every declaration site.
/// 本 crate 是**外部实现**：它用 `external_object!` 声明注册面，从不调用 `host!()`，因此这个常量由它自己
/// 拥有。声明宏现在读的就是它（审计 `M7`，P3.3）——每个 crate 一处，而不是每个声明处各读一次 `env!`。
pub const XIRANG_NAMESPACE: &str = env!("CARGO_PKG_NAME");

/// 必须与宿主共享同一个 framework，overlay 才接受这棵外部树。
/// Must match the host framework; `overlay` rejects a foreign tree.
pub const FRAMEWORK: FrameworkId = FrameworkId::new("xirang.example.control-button");

mod shared;
pub use shared::{ControlFrame, ControlHandle};

pub mod button_fast;
pub mod control_fast;
pub mod slider_fast;

/// 外部实现自己的注册机（项目外注册）。
/// The external implementation's own registry (out-of-project registration).
pub fn external_registry() -> Registry {
    let mut registry = Registry::root_for_namespace(FRAMEWORK, XIRANG_NAMESPACE);
    registry
        .register_all(&[
            button_fast::REGISTRATION,
            control_fast::REGISTRATION,
            slider_fast::REGISTRATION,
        ])
        .expect("external face registers");
    registry
}
