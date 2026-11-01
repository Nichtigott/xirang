//! `xirang package`: write a finished build's artifacts and a manifest into one delivery
//! directory. It refuses to build anything itself, so the refusal for a missing artifact names
//! `xirang build` as the step to run first.
//! `xirang package`：把已完成构建的产物与清单写进一个交付目录。它自己不构建，因此缺产物时的拒绝文案
//! 会点名先跑 `xirang build`。

use std::io::Write;

/// `xirang package [--output <dir>] [path]`.
/// `xirang package [--output <dir>] [path]`。
pub(crate) fn package(
    arguments: &mut dyn Iterator<Item = String>,
    out: &mut dyn Write,
) -> Result<(), String> {
    let mut arguments = arguments.peekable();
    let mut output = None;
    let mut path = None;
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--output" => {
                let value = arguments
                    .next()
                    .ok_or_else(|| "`--output` needs a directory".to_owned())?;
                output = Some(std::path::PathBuf::from(value));
            }
            other if other.starts_with('-') => {
                return Err(format!(
                    "unknown argument `{other}`; `xirang package` takes `--output <dir>` and an optional path"
                ));
            }
            other => path = Some(other.to_owned()),
        }
    }
    let root = std::path::Path::new(path.as_deref().unwrap_or("."));
    write!(
        out,
        "{}",
        crate::multilang::package(root, output.as_deref())?
    )
    .map_err(|error| format!("cannot write: {error}"))
}
