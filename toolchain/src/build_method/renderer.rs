//! Generated module tree rendering.
//! 生成模块树渲染。
//!
//! This file only mounts the renderer's concerns and re-exports the single
//! entry point the pipeline calls. The top-level pass lives in `pass`, the
//! recursive tree walk in `tree`, the object-alias emitter in `aliases`, and
//! the IDE shadow declarations in `ide`.
//! 本文件只挂载渲染器的各个关注点，并重新导出管线调用的唯一入口。顶层流程位于
//! `pass`，递归的模块树遍历位于 `tree`，对象别名发射器位于 `aliases`，
//! IDE 影子声明位于 `ide`。
//!
//! The file name `renderer/renderer.rs` was not used for the top-level pass
//! because a child module may not share its parent's name without tripping
//! `clippy::module_inception`, which this crate denies.
//! 顶层流程没有使用 `renderer/renderer.rs` 这个文件名：子模块与父模块同名会触发
//! `clippy::module_inception`，而本 crate 拒绝该警告。

#[path = "renderer/catalog.rs"]
mod catalog;
pub(crate) use catalog::render_catalog;

#[path = "renderer/aliases.rs"]
mod aliases;
#[path = "renderer/modules.rs"]
mod modules;
pub(crate) use modules::write_modules;
#[path = "renderer/pass.rs"]
mod pass;
#[path = "renderer/tree.rs"]
mod tree;

pub(crate) use owners::emitted_grafts;
#[cfg(test)]
pub(crate) use pass::render_lib;
pub(crate) use pass::render_lib_with_external;
pub(crate) use tree::ShapeRender;

#[path = "renderer/owners.rs"]
pub(super) mod owners;
