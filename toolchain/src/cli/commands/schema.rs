//! `xirang schema [--output <file>]`: print (or write) the JSON Schema for `xirang.toml`. Editors
//! use it for validation and completion; the parser stays the authority.
//! `xirang schema [--output <file>]`：打印（或写出）`xirang.toml` 的 JSON Schema。编辑器用它做校验
//! 与补全；解析器仍是权威。

use std::io::Write;

/// `xirang schema [--output <file>]`.
/// `xirang schema [--output <file>]`。
pub(crate) fn schema(
    arguments: &mut dyn Iterator<Item = String>,
    out: &mut dyn Write,
) -> Result<(), String> {
    let mut output = None;
    let mut pending = false;
    for argument in arguments {
        if pending {
            output = Some(argument);
            pending = false;
            continue;
        }
        match argument.as_str() {
            "--output" => pending = true,
            other if other.starts_with('-') => {
                return Err(format!(
                    "unknown argument `{other}`; `xirang schema` takes `--output <file>`"
                ));
            }
            other => {
                return Err(format!(
                    "unexpected argument `{other}`; `xirang schema` writes to stdout or `--output <file>`"
                ));
            }
        }
    }
    if pending {
        return Err("`--output` needs a file".to_owned());
    }
    let rendered = crate::multilang::manifest_schema()?;
    match output {
        Some(path) => {
            std::fs::write(&path, format!("{rendered}\n"))
                .map_err(|error| format!("cannot write {path}: {error}"))?;
            writeln!(out, "wrote the `xirang.toml` schema to {path}")
                .map_err(|error| format!("cannot write: {error}"))
        }
        None => writeln!(out, "{rendered}").map_err(|error| format!("cannot write: {error}")),
    }
}
