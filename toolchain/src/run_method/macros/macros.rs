//! Registration-face macros.
//! 注册面宏。

// The declaration macros name `crate::XIRANG_NAMESPACE` — the **call site's** crate root — and that
// is exactly what they must name: the constant `host!()` defines lives in the *host* crate, while
// `$crate` would name this macro's own crate (the toolchain) and resolve nothing. clippy's
// `crate_in_macro_def` assumes the `$crate` reading, so it is allowed once here, for the whole module
// (audit `M7`, P3.3).
// 声明宏命名的是 `crate::XIRANG_NAMESPACE`——**调用点**的 crate 根——而这正是它们必须命名的东西：
// `host!()` 定义的那个常量住在**宿主** crate 里，而 `$crate` 会指本宏自己的 crate（工具链），什么都
// 解析不到。clippy 的 `crate_in_macro_def` 假定的是 `$crate` 那种读法，因此在这里为整个模块允许一次
// （审计 `M7`，P3.3）。
#![allow(clippy::crate_in_macro_def)]

#[path = "entry.rs"]
mod entry;
#[path = "face.rs"]
mod face;
#[path = "trace.rs"]
mod trace;
