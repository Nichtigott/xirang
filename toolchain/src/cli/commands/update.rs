//! `xirang update`: re-resolve an `xirang.toml` project and record the graph in `xirang.lock` as
//! one transaction, printing the added/removed/changed units. `--check` prints the same diff and
//! writes nothing.
//! `xirang update`：重新解析 `xirang.toml` 项目，把依赖图作为一次事务记录进 `xirang.lock`，并打印
//! 新增/移除/变化的单元。`--check` 打印同样差异但不写文件。

use std::io::Write;

/// `xirang update [--check] [path]`.
/// `xirang update [--check] [path]`。
pub(crate) fn update(
    arguments: &mut dyn Iterator<Item = String>,
    out: &mut dyn Write,
) -> Result<(), String> {
    let mut check = false;
    let mut path = None;
    for argument in arguments {
        match argument.as_str() {
            "--check" => check = true,
            other if other.starts_with('-') => {
                return Err(format!(
                    "unknown argument `{other}`; `xirang update` takes `--check` and an optional path"
                ));
            }
            other => path = Some(other.to_owned()),
        }
    }
    let root = std::path::Path::new(path.as_deref().unwrap_or("."));
    writeln!(out, "{}", crate::multilang::update(root, check)?)
        .map_err(|error| format!("cannot write: {error}"))
}
