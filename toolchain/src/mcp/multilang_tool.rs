//! A read-only view of an `xirang.toml` project: its units (`inventory`), the actions a build
//! would run (`plan`), or the economics one finished build recorded (`report`). No action executes
//! anything; `plan` does not write the fact cache, so this tool can describe a tree the bridge does
//! not own.
//! `xirang.toml` 项目的只读视图：单元清单（`inventory`）、一次构建会跑的动作（`plan`），或某次
//! 已完成构建记下的经济性（`report`）。三个动作都不执行任何东西；`plan` 连事实缓存都不写，`report`
//! 只读已写下的证据，因此这个工具可以被用来读一棵不归桥所有的树。

use serde_json::Value;
/// What this tool answers, as the catalogue advertises it: a read-only view of one
/// `xirang.toml` project.
/// 这个工具答什么，按目录所广告的那样：一个 `xirang.toml` 项目的只读视图。
pub(super) const DESCRIPTION: &str = "Read an `xirang.toml` project as data: its units (`inventory`, the default), the \
actions a build would run (`plan`), or the economics one finished build recorded (`report`). No \
action builds, tests, or writes: `inventory` reads the manifests, `plan` computes without \
writing the fact cache, and `report` reads evidence a finished build already wrote, so this tool \
can describe a tree the bridge does not own. A directory with no `xirang.toml` is refused by \
name — this is the multilingual project face, not a Rust host's registry \
(`xirang.registry`/`xirang.explain` answer those). The `plan` reply prints the plan's identity \
first, so two answers that differ are comparable.";
use std::path::Path;

/// `xirang.multilang`: `inventory` (the default) or `plan`.
/// `xirang.multilang`：`inventory`（默认）或 `plan`。
pub(super) fn multilang(root: &Path, arguments: &Value) -> Result<String, String> {
    match arguments
        .get("action")
        .and_then(Value::as_str)
        .unwrap_or("inventory")
    {
        "inventory" => crate::multilang::inventory(root),
        "plan" => crate::multilang::plan(root),
        "report" => crate::multilang::report(root, arguments.get("plan").and_then(Value::as_str)),
        other => Err(format!(
            "`{other}` is not an action of this tool; the actions are `inventory` (the units of an \
             `xirang.toml` project), `plan` (the actions a build would run, without running them) \
             and `report` (the economics one finished build recorded). This tool never builds, \
             tests, or writes."
        )),
    }
}

#[cfg(test)]
#[path = "multilang_tool_tests.rs"]
mod multilang_tool_tests;
