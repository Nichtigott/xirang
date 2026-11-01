//! `xirang install` and `xirang rollback`: place a packaged delivery under a prefix and take it
//! back. Both verify bytes before writing, so a swapped or edited package is refused by name.
//! `xirang install` 与 `xirang rollback`：把打包的交付安放到前缀下、并把它退回。两者都在写之前校验
//! 字节，因此被替换或改过的包会按名拒绝。

use std::io::Write;
use std::path::PathBuf;

/// `xirang install <package> --prefix <dir>`.
/// `xirang install <package> --prefix <dir>`。
pub(crate) fn install(
    arguments: &mut dyn Iterator<Item = String>,
    out: &mut dyn Write,
) -> Result<(), String> {
    let mut arguments = arguments.peekable();
    let mut prefix = None;
    let mut package = None;
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--prefix" => {
                prefix = Some(PathBuf::from(
                    arguments
                        .next()
                        .ok_or_else(|| "`--prefix` needs a directory".to_owned())?,
                ));
            }
            other if other.starts_with('-') => {
                return Err(format!(
                    "unknown argument `{other}`; `xirang install` takes a package and `--prefix <dir>`"
                ));
            }
            other => package = Some(PathBuf::from(other)),
        }
    }
    let package = package.ok_or_else(|| {
        "`xirang install` needs the package directory (run `xirang package` first)".to_owned()
    })?;
    let prefix = prefix.ok_or_else(|| "`xirang install` needs `--prefix <dir>`".to_owned())?;
    write!(out, "{}", crate::multilang::install(&prefix, &package)?)
        .map_err(|error| format!("cannot write: {error}"))
}

/// `xirang rollback --prefix <dir>`.
/// `xirang rollback --prefix <dir>`。
pub(crate) fn rollback(
    arguments: &mut dyn Iterator<Item = String>,
    out: &mut dyn Write,
) -> Result<(), String> {
    let mut arguments = arguments.peekable();
    let mut prefix = None;
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--prefix" => {
                prefix = Some(PathBuf::from(
                    arguments
                        .next()
                        .ok_or_else(|| "`--prefix` needs a directory".to_owned())?,
                ));
            }
            other if other.starts_with('-') => {
                return Err(format!(
                    "unknown argument `{other}`; `xirang rollback` takes `--prefix <dir>`"
                ));
            }
            other => prefix = Some(PathBuf::from(other)),
        }
    }
    let prefix = prefix.ok_or_else(|| "`xirang rollback` needs `--prefix <dir>`".to_owned())?;
    write!(out, "{}", crate::multilang::rollback(&prefix)?)
        .map_err(|error| format!("cannot write: {error}"))
}
