//! Write guards: the context split between reading a project and writing to it,
//! and the refusal that keeps a write inside the selected project.
//! 写入守卫：读取项目与写入项目之间的上下文分野，以及那道把写入限制在已选中项目内的拒绝。
//!
//! The writes themselves are not here — they live in `super::mutations`
//! (`create_project`, `add_module_from_face`, `edit_module_face`, `submit_plugin`).
//! This module owns only the guard those writes run inside.
//! 真正的写入不在这里——它们在 `super::mutations`（`create_project`、
//! `add_module_from_face`、`edit_module_face`、`submit_plugin`）。本模块只拥有那些写入
//! 运行其内的那道守卫。
//!
//! The distinction this module exists for is read versus write. `package_root`
//! resolves a project from the session, then the environment, then the working
//! directory — a *guess*, and a reasonable one for a read, because reading the
//! wrong tree is visible. A write cannot afford it: a delete that resolved its
//! root that way moved a module out of whichever project the environment happened
//! to name, which is the bug the delete overlay still carries the comment about.
//! So writers go through [`with_selected_project`], which has no fallback, and a
//! future call site cannot repeat the bug by forgetting to establish a context.
//! 本模块存在的意义就是"读"与"写"的区分。`package_root` 依次从会话、环境、工作目录解析项目
//! ——一个**猜测**，对读取来说还算合理，因为读错树看得见。写入承担不起：一次这样解析根的删除把
//! 模块从环境变量恰好指到的那个项目里搬走了，也就是删除浮层至今在注释里记着的那个 bug。因此写入
//! 方都走 [`with_selected_project`]：它没有回落，将来的调用点也不会因为忘记建立上下文而重演。
//!
//! A launched session always has a project — `resolve_project` adopts it before the
//! terminal is taken over — so the refusal below is reachable only from a library
//! caller, a test, or a session whose selection was cleared.
//! 已启动的会话总有项目——`resolve_project` 在接管终端之前就采纳了它——因此下面的拒绝只可能来自
//! 库调用方、测试，或被清空选择的会话。

use std::path::PathBuf;

use super::project_context::{package_namespace, selected_root};

/// The selected project's root, or a named refusal.
/// 已选中项目的根目录，或一条具名拒绝。
pub(super) fn selected_package_root() -> Result<PathBuf, String> {
    selected_root()
        .ok_or_else(|| "no project is selected; open a project before writing to it".to_owned())
}

/// The selected project's root for a read, or a named refusal.
/// 读取用的已选中项目根目录，或一条具名拒绝。
pub(super) fn selected_read_root() -> Result<PathBuf, String> {
    selected_root()
        .ok_or_else(|| "no project is selected; open a project before reading it".to_owned())
}

/// Run one write inside the selected project's authoring context.
/// 在已选中项目的创作上下文里执行一次写入。
pub(super) fn with_selected_project<T>(
    operation: impl FnOnce() -> Result<T, String>,
) -> Result<T, String> {
    let root = selected_package_root()?;
    crate::run_method::AuthoringContext::new(root, package_namespace()).scope(operation)
}

/// Run one read of the authoring records inside the selected project's context.
/// 在已选中项目的上下文里读取创作记录。
///
/// Since audit `STU-S-29` a read that the screen presents as "the state of the project the
/// reader opened" goes through the same guard a write does. `package_root` deliberately
/// falls back (session, then environment, then working directory), so a read through it can
/// describe a tree the reader never opened — which is what the graft screen used to do when
/// no project was selected.
/// 自审计 `STU-S-29` 起，界面作为“读者打开的那个项目的状态”呈现的读取，与写入走同一道守卫。
/// `package_root` 有意回落（会话、环境、工作目录），因此经它的读取可能描述的是一棵读者从未打开
/// 的树——这正是没有选中项目时 graft 界面过去的行为。
pub(super) fn with_selected_project_read<T>(
    operation: impl FnOnce() -> Result<T, String>,
) -> Result<T, String> {
    let root = selected_read_root()?;
    crate::run_method::AuthoringContext::new(root, package_namespace()).scope(operation)
}
