//! Where one edit request runs, and how its paths come back.
//! 一次编辑请求在哪里运行，以及它的路径怎么回来。
//!
//! A preview runs the executor in a throwaway copy, so every path it reports
//! belongs to a directory that is deleted a moment later. That rule used to live in
//! a comment plus an `applied: bool` inside `apply`, so "a copy's paths are rewritten
//! before anyone reads them" held only by convention — and both defects this write
//! path has had (`would write /tmp/...`) lived in exactly that rewriting. The type
//! carries it now: a copy can only be named through [`Target::report_path`], the
//! message is rewritten by [`Target::report_message`], and the diff is computed
//! only for a copy. It is a module of its own because that invariant is what a
//! reader of the write path has to settle first, while `apply`'s own subject is the
//! four verbs.
//! 预览在一份一次性副本里运行执行器，因此它报告的每条路径都属于一个随后就被删掉的目录。这条规则
//! 过去住在 `apply` 里的注释、外加一个 `applied: bool`，因此"副本的路径在任何人读之前都已改写"
//! 只靠约定成立——而这条写入路径出过的两个缺陷（`would write /tmp/...`）恰恰都出在那层改写上。
//! 现在由类型携带它：副本只能经 [`Target::report_path`] 被命名，消息由
//! [`Target::report_message`] 改写，而 diff 只为副本计算。它单独成模块，是因为这个不变量正是读
//! 写入路径的人必须先定下来的东西，而 `apply` 自己的题材是那四个动词。

use std::path::{Path, PathBuf};

use crate::mcp::preview::remove_copy;

/// Where one edit request runs, and how its paths come back.
/// 一次编辑请求在哪里运行，以及它的路径怎么回来。
pub(crate) enum Target {
    /// The project itself: `apply: true`.
    /// 项目本身：`apply: true`。
    Project,
    /// A throwaway copy of the project: the default preview.
    /// 项目的一次性副本：默认的预览。
    Copy(PathBuf),
}

impl Target {
    /// The directory the executor runs in.
    /// 执行器运行所在的目录。
    pub(crate) fn work_dir<'a>(&'a self, root: &'a Path) -> &'a Path {
        match self {
            Self::Project => root,
            Self::Copy(work) => work,
        }
    }

    /// Whether this request writes to the project.
    /// 这次请求是否写入项目。
    pub(crate) fn applied(&self) -> bool {
        matches!(self, Self::Project)
    }

    /// The path to report for a file the executor touched: the project's path, so a
    /// preview cannot name a directory that is deleted a moment later.
    /// 执行器碰过的文件应当报告的路径：项目里的那个，因此预览说不出一个随后就被删掉的目录。
    pub(crate) fn report_path(&self, path: &Path, root: &Path) -> PathBuf {
        match path.strip_prefix(self.work_dir(root)) {
            Ok(relative) => root.join(relative),
            Err(_) => path.to_path_buf(),
        }
    }

    /// The tree-relative path a reported declaration anchor is written with.
    /// 报告的声明锚点所用的树内相对路径。
    pub(crate) fn report_relative(&self, path: &Path, root: &Path) -> PathBuf {
        path.strip_prefix(self.work_dir(root))
            .or_else(|_| path.strip_prefix(root))
            .map_or_else(|_| path.to_path_buf(), Path::to_path_buf)
    }

    /// Rewrite the executor's own message, which names paths too: a preview would
    /// otherwise print a directory that is deleted a moment later.
    /// 改写执行器自己的消息——它也会点名路径，否则预览会打印出一个随后就被删掉的目录。
    pub(crate) fn report_message(&self, message: String, root: &Path) -> String {
        match self {
            Self::Project => message,
            Self::Copy(work) => {
                message.replace(&work.display().to_string(), &root.display().to_string())
            }
        }
    }

    /// Drop the copy, if this request used one. The project is never removed.
    /// 收掉副本（如果这次请求用了副本）。项目永远不会被删除。
    pub(crate) fn discard(&self, root: &Path) {
        if let Self::Copy(work) = self {
            remove_copy(root, work);
        }
    }
}
