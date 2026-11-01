//! `xirang move <unit> <destination>`: move one unit's directory and rewrite the path
//! dependencies that point at it as one transaction.
//! `xirang move <unit> <destination>`：搬动一个单元的目录，并把指向它的路径依赖作为一次事务重写。

use std::io::Write;

/// `xirang move <unit> <destination>`.
/// `xirang move <unit> <destination>`。
pub(crate) fn move_unit(
    arguments: &mut dyn Iterator<Item = String>,
    out: &mut dyn Write,
) -> Result<(), String> {
    let mut source = false;
    let mut positional = Vec::new();
    for argument in arguments {
        match argument.as_str() {
            "--source" => source = true,
            other if other.starts_with('-') => {
                return Err(format!(
                    "unknown argument `{other}`; `xirang move` takes `--source` optionally and a name and a destination"
                ));
            }
            _ => positional.push(argument),
        }
    }
    let [unit, destination] = positional.as_slice() else {
        return Err(
            "`xirang move` takes a name and a destination: `xirang move [--source] <name> <destination>`"
                .to_owned(),
        );
    };
    let message = if source {
        crate::multilang::move_source(std::path::Path::new("."), unit, destination)?
    } else {
        crate::multilang::move_unit(std::path::Path::new("."), unit, destination)?
    };
    writeln!(out, "{message}").map_err(|error| format!("cannot write: {error}"))
}
