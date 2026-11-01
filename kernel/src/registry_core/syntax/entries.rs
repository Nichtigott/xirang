//! Host-entry grammars: `application!`, `graft_plan!` and `static_graft_plan!`.
//! 宿主入口语法：`application!`、`graft_plan!` 与 `static_graft_plan!`。
//!
//! The face parser lives in [`super::face`]; this file only mounts the two
//! entry-level grammars and re-exports their items, so every entry path stays
//! put: `application!` discovery in `application`, the graft grammars in
//! `graft`. Neither grammar executes anything.
//! 注册面解析器在 [`super::face`]；本文件只挂载两种入口级语法并重导出其条目，
//! 因此每个入口路径都保持不变：`application!` 发现在 `application`，graft 语法在
//! `graft`。两种语法都不执行任何东西。

#[path = "entries/application.rs"]
mod application;
#[path = "entries/graft.rs"]
mod graft;

pub use application::application_entries;
pub use graft::{
    GraftEndpointSpan, GraftExpressions, GraftSpans, GraftSyntax, graft_entries,
    render_graft_expression,
};

#[path = "entries/native_graft.rs"]
mod native_graft;

pub use native_graft::{append_native_graft, render_native_graft_expression, repoint_native_graft};
