//! `xirang move --source <name> <destination>`: move a local `[sources]` directory as one
//! transaction, rewriting the root manifest's declared path and every `[overrides]` entry that
//! pointed inside it. A git or registry source is refused by name: its location is not a directory
//! this command owns. The packages keep their names, so dependencies that select them by source
//! name need no rewrite.
//! `xirang move --source <name> <destination>`：把一个本地 `[sources]` 目录作为一次事务搬动，重写
//! 根清单里声明的路径，以及每个指向它内部的 `[overrides]` 条目。git 或 registry 源按名拒绝：它们的
//! 位置不是本命令拥有的目录。包名不变，因此按源名选择它们的依赖无需改写。

use std::path::{Path, PathBuf};

/// Move the local source `name` to `destination` under `root`, rewriting the root manifest.
/// 把本地源 `name` 搬到 `root` 下的 `destination`，并重写根清单。
pub fn move_source(root: &Path, name: &str, destination: &str) -> Result<String, String> {
    let root = std::fs::canonicalize(root)
        .map_err(|error| format!("cannot resolve {}: {error}", root.display()))?;
    let manifest = root.join("xirang.toml");
    let text = std::fs::read_to_string(&manifest)
        .map_err(|error| format!("cannot read {}: {error}", manifest.display()))?;
    let (start, end, declared) = source_path(&text, name)?;
    let old = if Path::new(&declared).is_absolute() {
        PathBuf::from(&declared)
    } else {
        root.join(&declared)
    };
    let old = std::fs::canonicalize(&old).map_err(|error| {
        format!(
            "cannot resolve the source directory {}: {error}",
            old.display()
        )
    })?;
    if !old.is_dir() {
        return Err(format!(
            "the source `{name}` resolves to {} which is not a directory; nothing moved",
            old.display()
        ));
    }
    if old == root {
        return Err("the source resolves to the project root itself; nothing moved".to_owned());
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
    if destination.starts_with(&old) {
        return Err("the destination is inside the source being moved; nothing moved".to_owned());
    }
    // Every edit is computed against the original text; apply them from the back so earlier spans
    // stay valid.
    // 所有编辑都按原文计算，从后往前应用，先前的区间才不会失效。
    let mut edits: Vec<(usize, usize, String)> = vec![(
        start,
        end,
        super::move_unit::relative(&root, &destination)
            .display()
            .to_string(),
    )];
    let mut followed = 0;
    for (start, end, value) in override_paths(&text) {
        let resolved = if Path::new(&value).is_absolute() {
            PathBuf::from(&value)
        } else {
            root.join(&value)
        };
        let Ok(resolved) = std::fs::canonicalize(&resolved) else {
            continue;
        };
        let Ok(inside) = resolved.strip_prefix(&old) else {
            continue;
        };
        let moved = destination.join(inside);
        edits.push((
            start,
            end,
            super::move_unit::relative(&root, &moved)
                .display()
                .to_string(),
        ));
        followed += 1;
    }
    edits.sort_by_key(|(start, _, _)| *start);
    let mut rewritten = text.clone();
    for (start, end, replacement) in edits.iter().rev() {
        rewritten.replace_range(*start..*end, replacement);
    }
    std::fs::rename(&old, &destination)
        .map_err(|error| format!("cannot move the source: {error}"))?;
    if let Err(error) = std::fs::write(&manifest, &rewritten) {
        let _ = std::fs::rename(&destination, &old);
        return Err(format!(
            "cannot rewrite {}: {error}; the source directory was moved back",
            manifest.display()
        ));
    }
    Ok(format!(
        "moved source `{name}` from {} to {}\nrewrote the root manifest ({followed} override path(s) followed)\nthe lock still records the old path; run `xirang update` to record the move",
        old.display(),
        destination.display()
    ))
}

/// The quoted path of `name` inside the root manifest's `[sources]` table, with its byte span.
/// 根清单 `[sources]` 表里 `name` 的带引号路径，连同它的字节区间。
fn source_path(text: &str, name: &str) -> Result<(usize, usize, String), String> {
    let lines = lines_of(text, "sources")?;
    for (offset, line) in &lines {
        let trimmed = line.trim_start();
        let Some(rest) = trimmed.strip_prefix(name) else {
            continue;
        };
        let rest = rest.trim_start();
        let Some(rest) = rest.strip_prefix('=') else {
            continue;
        };
        let value = rest.trim_start();
        if value.starts_with('{') {
            if value.contains("git") {
                return Err(format!(
                    "`{name}` is a git source; its location is not a directory this command owns; nothing moved"
                ));
            }
            if value.contains("registry") {
                return Err(format!(
                    "`{name}` is a registry source; its location is not a directory this command owns; nothing moved"
                ));
            }
        }
        let base = offset + (line.len() - rest.len()) + rest.len() - value.len();
        return quoted(base, value);
    }
    Err(format!(
        "the root manifest declares no source named `{name}`; `[sources]` has: {}",
        lines
            .iter()
            .map(|(_, line)| line.split('=').next().unwrap_or(line).trim())
            .filter(|key| !key.is_empty())
            .collect::<Vec<_>>()
            .join(", ")
    ))
}

/// The quoted paths of every entry in the root manifest's `[overrides]` table.
/// 根清单 `[overrides]` 表里每个条目的带引号路径。
fn override_paths(text: &str) -> Vec<(usize, usize, String)> {
    let Ok(lines) = lines_of(text, "overrides") else {
        return Vec::new();
    };
    let mut paths = Vec::new();
    for (offset, line) in &lines {
        let trimmed = line.trim_start();
        let Some((_, rest)) = trimmed.split_once('=') else {
            continue;
        };
        let value = rest.trim_start();
        let base = offset + (line.len() - rest.len()) + rest.len() - value.len();
        if let Ok(span) = quoted(base, value) {
            paths.push(span);
        }
    }
    paths
}

/// The non-empty lines of one table, each with its byte offset in the whole text.
/// 一个表里的非空行，每行带它在整段文本里的字节偏移。
fn lines_of<'a>(text: &'a str, section: &str) -> Result<Vec<(usize, &'a str)>, String> {
    let header = format!("[{section}]");
    let mut inside = false;
    let mut found = false;
    let mut lines = Vec::new();
    let mut offset = 0;
    for line in text.split_inclusive('\n') {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            inside = trimmed == header;
            found |= inside;
        } else if inside && !trimmed.is_empty() {
            lines.push((offset, line));
        }
        offset += line.len();
    }
    if found {
        Ok(lines)
    } else {
        Err(format!("the root manifest has no `{header}` table"))
    }
}

/// The contents of the first quoted string in `value`, with its byte span in the whole text.
/// `value` 里第一个带引号字符串的内容，连同它在整段文本里的字节区间。
fn quoted(base: usize, value: &str) -> Result<(usize, usize, String), String> {
    let Some(open) = value.find('"') else {
        return Err(format!("`{value}` is not a quoted path"));
    };
    let Some(close) = value[open + 1..].find('"') else {
        return Err(format!("`{value}` has no closing quote"));
    };
    Ok((
        base + open + 1,
        base + open + 1 + close,
        value[open + 1..open + 1 + close].to_owned(),
    ))
}
