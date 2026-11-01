//! `xirang gc`: collect content-store entries no stamp references.
//! `xirang gc`：回收没有任何印记引用的内容库条目。

use std::io::Write;

/// `xirang gc [--dry-run] [path]`.
/// `xirang gc [--dry-run] [path]`。
pub(crate) fn gc(
    arguments: &mut dyn Iterator<Item = String>,
    out: &mut dyn Write,
) -> Result<(), String> {
    let mut dry_run = false;
    let mut path = None;
    for argument in arguments {
        match argument.as_str() {
            "--dry-run" => dry_run = true,
            other if other.starts_with('-') => {
                return Err(format!(
                    "unknown argument `{other}`; `xirang gc` takes `--dry-run` and an optional path"
                ));
            }
            other => path = Some(other.to_owned()),
        }
    }
    let root = std::path::Path::new(path.as_deref().unwrap_or("."));
    writeln!(out, "{}", crate::multilang::collect_garbage(root, dry_run)?)
        .map_err(|error| format!("cannot write: {error}"))
}
