//! `xirang plan`: print the actions `xirang build` would run, without running or writing anything.
//! `xirang plan`：打印 `xirang build` 会跑的动作，但不执行、不写文件。

use std::io::Write;
use std::path::Path;

/// `xirang plan [--json] [path]`.
/// `xirang plan [--json] [path]`。
pub(crate) fn plan(
    arguments: &mut dyn Iterator<Item = String>,
    out: &mut dyn Write,
) -> Result<(), String> {
    let mut json = false;
    let mut path = None;
    for argument in arguments {
        match argument.as_str() {
            "--json" => json = true,
            other if other.starts_with('-') => {
                return Err(format!(
                    "unknown argument `{other}`; `xirang plan` takes `--json` and an optional path"
                ));
            }
            other => path = Some(other.to_owned()),
        }
    }
    let root = Path::new(path.as_deref().unwrap_or("."));
    let text = if json {
        crate::multilang::plan_json(root)?
    } else {
        crate::multilang::plan(root)?
    };
    writeln!(out, "{text}").map_err(|error| format!("cannot write: {error}"))
}
