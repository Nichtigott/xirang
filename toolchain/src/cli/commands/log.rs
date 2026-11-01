//! `xirang log verify`: recompute a prefix's append-only install log, compare it with the
//! generations still under that prefix, and name the first thing that disagrees. Read-only.
//! `xirang log verify`：重算一个前缀的只追加安装日志，把它与该前缀下仍在的世代对照，并点名第一处
//! 不一致。只读。

use std::io::Write;

/// `xirang log verify --prefix <dir> [--json]`.
/// `xirang log verify --prefix <dir> [--json]`。
pub(crate) fn log(
    arguments: &mut dyn Iterator<Item = String>,
    out: &mut dyn Write,
) -> Result<(), String> {
    let action = arguments
        .next()
        .ok_or_else(|| "`xirang log` needs an action: `verify --prefix <dir>`".to_owned())?;
    if action != "verify" {
        return Err(format!(
            "unknown action `{action}`; `xirang log` takes `verify --prefix <dir> [--json]`"
        ));
    }
    let mut prefix = None;
    let mut json = false;
    let mut arguments = arguments.peekable();
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--prefix" => {
                prefix = Some(
                    arguments
                        .next()
                        .ok_or_else(|| "`--prefix` needs a directory".to_owned())?,
                );
            }
            "--json" => json = true,
            other => {
                return Err(format!(
                    "unknown argument `{other}`; `xirang log` takes `verify --prefix <dir> [--json]`"
                ));
            }
        }
    }
    let prefix = prefix.ok_or_else(|| "`xirang log` needs `verify --prefix <dir>`".to_owned())?;
    let (report, verified) = crate::multilang::log_verify(std::path::Path::new(&prefix), json)?;
    writeln!(out, "{report}").map_err(|error| format!("cannot write: {error}"))?;
    if verified {
        Ok(())
    } else {
        Err(format!(
            "the log under {prefix} does not verify; the report above names each disagreement"
        ))
    }
}
