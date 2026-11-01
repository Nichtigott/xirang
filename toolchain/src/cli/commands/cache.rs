//! `xirang cache`: move a build cache between checkouts. `export` copies the content store and the
//! stamps that point into it; `import` verifies every byte first and never overwrites.
//! `xirang cache`：在检出之间移动构建缓存。`export` 复制内容库与指向它的印记；`import` 先逐字节
//! 核对，并且不覆盖已有文件。

use std::io::Write;
use std::path::Path;

/// `xirang cache export <dir>` / `xirang cache import <dir> [--dry-run]`.
/// `xirang cache export <dir>` / `xirang cache import <dir> [--dry-run]`。
pub(crate) fn cache(
    arguments: &mut dyn Iterator<Item = String>,
    out: &mut dyn Write,
) -> Result<(), String> {
    let mut dry_run = false;
    let mut positional = Vec::new();
    for argument in arguments {
        match argument.as_str() {
            "--dry-run" => dry_run = true,
            other if other.starts_with('-') => {
                return Err(format!(
                    "unknown argument `{other}`; `xirang cache` takes `export <dir>` or `import <dir> [--dry-run]`"
                ));
            }
            other => positional.push(other.to_owned()),
        }
    }
    let action = positional.first().ok_or_else(|| {
        "`xirang cache` takes `export <dir>` or `import <dir> [--dry-run]`".to_owned()
    })?;
    let directory = positional.get(1).ok_or_else(|| {
        format!(
            "`xirang cache {action}` needs a directory to {}",
            match action.as_str() {
                "export" => "write into",
                _ => "read from",
            }
        )
    })?;
    let directory = Path::new(directory);
    let text = match action.as_str() {
        "export" => crate::multilang::cache_export(Path::new("."), directory)?,
        "import" => crate::multilang::cache_import(Path::new("."), directory, dry_run)?,
        other => {
            return Err(format!(
                "`{other}` is not a cache action; the actions are `export` and `import`"
            ));
        }
    };
    writeln!(out, "{text}").map_err(|error| format!("cannot write: {error}"))
}
