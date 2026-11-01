//! `xirang move <unit> <destination>`: move one unit's directory and rewrite exactly the path
//! dependencies that point at it, as one transaction. A unit reached through `[sources]` or an
//! override, the root unit, or a destination that already exists is refused by name before
//! anything moves; a rewrite failure restores every manifest and the directory.
//! `xirang move <unit> <destination>`：搬动一个单元的目录，并把**恰好**指向它的那些路径依赖
//! 重写，作为一次事务。经 `[sources]` 或 override 到达的单元、根单元、已存在的目标目录都会在
//! 动手之前按名拒绝；任何一次重写失败都会把清单与目录恢复原状。

use std::path::{Component, Path, PathBuf};

/// Move `unit` to `destination` under `root`, rewriting the dependents that point at it.
/// 把 `unit` 搬到 `root` 下的 `destination`，并重写指向它的依赖者。
pub fn move_unit(root: &Path, unit: &str, destination: &str) -> Result<String, String> {
    let root = std::fs::canonicalize(root)
        .map_err(|error| format!("cannot resolve {}: {error}", root.display()))?;
    let project = super::project::load(&root)?;
    let index = locate(&project, unit)?;
    let target = &project.units[index];
    let target_directory = std::fs::canonicalize(&target.directory)
        .map_err(|error| format!("cannot resolve {}: {error}", target.directory.display()))?;
    if target_directory == root {
        return Err("the root unit cannot be moved; only a dependency can".to_owned());
    }
    if let Some(source) = &target.source {
        return Err(format!(
            "`{}` is reached through `{source}`; a unit that a source or an override resolves is moved by moving its source (`xirang move --source <name> <destination>`), not by this command. Nothing moved",
            target.manifest.package.name
        ));
    }
    let destination = if Path::new(destination).is_absolute() {
        PathBuf::from(destination)
    } else {
        root.join(destination)
    };
    let parent = destination
        .parent()
        .ok_or_else(|| format!("`{}` has no parent directory", destination.display()))?;
    if !parent.is_dir() {
        return Err(format!(
            "the destination's parent `{}` does not exist; create it first, nothing moved",
            parent.display()
        ));
    }
    let destination = std::fs::canonicalize(parent)
        .map_err(|error| format!("cannot resolve {}: {error}", parent.display()))?
        .join(
            destination
                .file_name()
                .ok_or_else(|| format!("`{}` names no directory", destination.display()))?,
        );
    if destination.exists() {
        return Err(format!(
            "`{}` already exists; nothing moved",
            destination.display()
        ));
    }
    if destination.starts_with(&target_directory) {
        return Err("the destination is inside the unit being moved; nothing moved".to_owned());
    }
    // Plan every rewrite before touching the tree, so a dependent this command cannot spell is a
    // refusal rather than a half-moved tree.
    // 动手之前先把每处重写算好：写不出来的依赖者应导致拒绝，而不是半搬的树。
    let mut rewrites: Vec<(PathBuf, String)> = Vec::new();
    for (position, dependent) in project.units.iter().enumerate() {
        if position == index {
            continue;
        }
        let dependent_directory = std::fs::canonicalize(&dependent.directory).map_err(|error| {
            format!("cannot resolve {}: {error}", dependent.directory.display())
        })?;
        let old = relative(&dependent_directory, &target_directory);
        let new = relative(&dependent_directory, &destination);
        let manifest = dependent.directory.join("xirang.toml");
        let text = std::fs::read_to_string(&manifest)
            .map_err(|error| format!("cannot read {}: {error}", manifest.display()))?;
        match rewrite(&text, &old, &new) {
            Some(rewritten) => rewrites.push((manifest, rewritten)),
            None => {
                if project.edges[position].contains(&index) {
                    return Err(format!(
                        "`{}` depends on `{}` through a spelling this command does not rewrite (only `path = \"…\"` inside `[dependencies]`); nothing moved",
                        dependent.manifest.package.name, target.manifest.package.name
                    ));
                }
            }
        }
    }
    let backups: Vec<(PathBuf, String)> = rewrites
        .iter()
        .map(|(manifest, _)| {
            std::fs::read_to_string(manifest)
                .map(|text| (manifest.clone(), text))
                .map_err(|error| format!("cannot read {}: {error}", manifest.display()))
        })
        .collect::<Result<_, _>>()?;
    std::fs::rename(&target_directory, &destination)
        .map_err(|error| format!("cannot move the unit: {error}"))?;
    for (manifest, text) in &rewrites {
        if let Err(error) = std::fs::write(manifest, text) {
            for (path, backup) in &backups {
                let _ = std::fs::write(path, backup);
            }
            let _ = std::fs::rename(&destination, &target_directory);
            return Err(format!(
                "cannot rewrite {}: {error}; the unit and every manifest were restored",
                manifest.display()
            ));
        }
    }
    let listed = rewrites
        .iter()
        .map(|(manifest, _)| manifest.display().to_string())
        .collect::<Vec<_>>()
        .join(", ");
    Ok(format!(
        "moved `{}` from {} to {}\nrewrote {} manifest(s): {}\nthe lock still records the old path; run `xirang update` to record the move",
        target.manifest.package.name,
        target_directory.display(),
        destination.display(),
        rewrites.len(),
        if listed.is_empty() {
            "none".to_owned()
        } else {
            listed
        }
    ))
}

/// The unit `unit` names: its package name or its key, refused by name when ambiguous or absent.
/// `unit` 指名的单元：包名或键；有歧义或不存在时按名拒绝。
fn locate(project: &super::project::Project, unit: &str) -> Result<usize, String> {
    let matches: Vec<usize> = project
        .units
        .iter()
        .enumerate()
        .filter(|(_, candidate)| candidate.manifest.package.name == unit || candidate.key == unit)
        .map(|(index, _)| index)
        .collect();
    match matches.as_slice() {
        [index] => Ok(*index),
        [] => Err(format!(
            "no unit is named `{unit}`; the units are: {}",
            project
                .units
                .iter()
                .map(|unit| unit.key.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        )),
        _ => Err(format!(
            "`{unit}` names {} units; use the unique key instead: {}",
            matches.len(),
            matches
                .iter()
                .map(|index| project.units[*index].key.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        )),
    }
}

/// The path from `from` to `to`, without consulting the filesystem.
/// 从 `from` 到 `to` 的相对路径，不查文件系统。
pub(super) fn relative(from: &Path, to: &Path) -> PathBuf {
    let from: Vec<Component<'_>> = from.components().collect();
    let to: Vec<Component<'_>> = to.components().collect();
    let mut common = 0;
    while common < from.len() && common < to.len() && from[common] == to[common] {
        common += 1;
    }
    let mut out = PathBuf::new();
    for _ in common..from.len() {
        out.push("..");
    }
    for component in &to[common..] {
        out.push(component.as_os_str());
    }
    if out.as_os_str().is_empty() {
        out.push(".");
    }
    out
}

/// Rewrite every `path = "<old>"` inside the `[dependencies]` table; `None` when the text holds
/// no such binding.
/// 重写 `[dependencies]` 表里每一处 `path = "<old>"`；文本没有这样的绑定就返回 `None`。
fn rewrite(text: &str, old: &Path, new: &Path) -> Option<String> {
    let old = old.display().to_string();
    let new = new.display().to_string();
    let mut out = String::new();
    let mut in_dependencies = false;
    let mut replaced = 0;
    for line in text.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with('[') {
            in_dependencies = trimmed == "[dependencies]";
            out.push_str(line);
            out.push('\n');
            continue;
        }
        let mut line = line.to_owned();
        if in_dependencies
            && let Some(value) = path_value(&line)
            && value.1 == old
        {
            line = format!("{}\"{}\"{}", &line[..value.0], new, &line[value.2..]);
            replaced += 1;
        }
        out.push_str(&line);
        out.push('\n');
    }
    (replaced > 0).then_some(out)
}

/// The byte range of the quoted value of a `path = "…"` binding on `line`, plus the value.
/// `line` 上 `path = "…"` 绑定里被引号值的字节区间与值。
fn path_value(line: &str) -> Option<(usize, String, usize)> {
    let key = line.find("path")?;
    let rest = &line[key + 4..];
    let spaces = rest.len() - rest.trim_start().len();
    let rest = rest.trim_start();
    let rest = rest.strip_prefix('=')?;
    let spaces2 = rest.len() - rest.trim_start().len();
    let rest = rest.trim_start();
    let quote = key + 4 + spaces + 1 + spaces2;
    let rest = rest.strip_prefix('"')?;
    let end = rest.find('"')?;
    let value = rest[..end].to_owned();
    Some((quote, value, quote + 1 + end + 1))
}
