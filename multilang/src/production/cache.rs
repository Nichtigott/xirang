//! Moving a build cache between checkouts: export the content store together with the stamps that
//! point into it, then import them into another checkout **only after every byte matches**.
//! 在检出之间移动构建缓存：把内容库与指向它的动作印记一起导出，然后在另一个检出里**逐字节核对之后**
//! 才导入。
//!
//! The limits are part of the design: an import never overwrites an existing file, the export is
//! refused when there is nothing to export, and the manifest is verified in full before the first
//! write so a tampered export imports nothing rather than half of it.
//! 边界是设计的一部分：导入不覆盖已有文件；没有可导出的东西时拒绝导出；清单在第一次写入前**整份**
//! 核对，因此被篡改的导出什么都不会导入，而不是导入一半。

use std::path::{Component, Path, PathBuf};

use serde_json::{Value, json};

/// The manifest version this module writes and reads.
/// 本模块写出与读取的清单版本。
const PROTOCOL: u64 = 1;

/// Export `<root>/target/xirang/{cas, build/*/stamps}` into `destination`.
/// 把 `<root>/target/xirang/{cas, build/*/stamps}` 导出到 `destination`。
pub(super) fn export(root: &Path, destination: &Path) -> Result<String, String> {
    let xirang = root.join("target").join("xirang");
    let cas = xirang.join("cas");
    if !cas.is_dir() {
        return Err(format!(
            "there is no content store at {}; build once before exporting a cache",
            cas.display()
        ));
    }
    if destination.join("manifest.json").exists() {
        return Err(format!(
            "{} already holds an exported cache; choose an empty directory",
            destination.display()
        ));
    }
    let mut files = Vec::new();
    visit_files(&cas, &xirang, &mut files)?;
    let build = xirang.join("build");
    if let Ok(entries) = std::fs::read_dir(&build) {
        for entry in entries.flatten() {
            let stamps = entry.path().join("stamps");
            if stamps.is_dir() {
                visit_files(&stamps, &xirang, &mut files)?;
            }
        }
    }
    files.sort();
    if files.is_empty() {
        return Err(format!(
            "{} holds no cached bytes or stamps; build once before exporting a cache",
            xirang.display()
        ));
    }
    let mut recorded = Vec::new();
    for relative in &files {
        let source = xirang.join(relative);
        let bytes = std::fs::read(&source)
            .map_err(|error| format!("cannot read {}: {error}", source.display()))?;
        recorded.push(json!({
            "path": relative.to_string_lossy(),
            "sha256": xirang_kernel::sha256_hex(&bytes),
            "bytes": bytes.len(),
        }));
    }
    std::fs::create_dir_all(destination)
        .map_err(|error| format!("cannot create {}: {error}", destination.display()))?;
    for relative in &files {
        let target = destination.join(relative);
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|error| format!("cannot create {}: {error}", parent.display()))?;
        }
        let bytes = std::fs::read(xirang.join(relative))
            .map_err(|error| format!("cannot read {}: {error}", xirang.join(relative).display()))?;
        std::fs::write(&target, bytes)
            .map_err(|error| format!("cannot write {}: {error}", target.display()))?;
    }
    let manifest = json!({"protocol": PROTOCOL, "files": recorded});
    let text = serde_json::to_string_pretty(&manifest)
        .map_err(|error| format!("cannot render the manifest: {error}"))?;
    std::fs::write(destination.join("manifest.json"), format!("{text}\n")).map_err(|error| {
        format!(
            "cannot write {}: {error}",
            destination.join("manifest.json").display()
        )
    })?;
    Ok(format!(
        "exported {} file(s) from {} into {}",
        files.len(),
        xirang.display(),
        destination.display()
    ))
}

/// Import an exported cache into `<root>/target/xirang`, verifying every entry first.
/// 把导出的缓存导入 `<root>/target/xirang`，先核对每个条目。
pub(super) fn import(root: &Path, source: &Path, dry_run: bool) -> Result<String, String> {
    let manifest_path = source.join("manifest.json");
    let text = std::fs::read_to_string(&manifest_path).map_err(|error| {
        format!(
            "{} does not hold an exported cache: {error}",
            source.display()
        )
    })?;
    let manifest: Value = serde_json::from_str(&text)
        .map_err(|error| format!("{} is not valid JSON: {error}", manifest_path.display()))?;
    if manifest["protocol"].as_u64() != Some(PROTOCOL) {
        return Err(format!(
            "{} was written by another cache protocol; this build reads {PROTOCOL}",
            manifest_path.display()
        ));
    }
    let entries = manifest["files"]
        .as_array()
        .ok_or_else(|| format!("{} has no `files` array", manifest_path.display()))?;
    let mut verified: Vec<(PathBuf, Vec<u8>)> = Vec::new();
    for entry in entries {
        let relative = entry["path"].as_str().ok_or_else(|| {
            format!(
                "{} has a file entry without a path",
                manifest_path.display()
            )
        })?;
        let expected = entry["sha256"].as_str().ok_or_else(|| {
            format!(
                "{} has an entry for `{relative}` without a sha256",
                manifest_path.display()
            )
        })?;
        let path = Path::new(relative);
        if path.is_absolute()
            || path
                .components()
                .any(|component| matches!(component, Component::ParentDir))
        {
            return Err(format!(
                "`{relative}` is not a path this cache may write; nothing was imported"
            ));
        }
        let bytes = std::fs::read(source.join(path)).map_err(|error| {
            format!("`{relative}` is missing from the export: {error}; nothing was imported")
        })?;
        let actual = xirang_kernel::sha256_hex(&bytes);
        if actual != expected {
            return Err(format!(
                "`{relative}` does not match the export manifest (expected {expected}, found {actual}); nothing was imported"
            ));
        }
        verified.push((path.to_path_buf(), bytes));
    }
    let xirang = root.join("target").join("xirang");
    let mut imported = 0;
    let mut skipped = 0;
    for (relative, bytes) in verified {
        let target = xirang.join(&relative);
        if target.exists() {
            skipped += 1;
            continue;
        }
        if !dry_run {
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|error| format!("cannot create {}: {error}", parent.display()))?;
            }
            std::fs::write(&target, bytes)
                .map_err(|error| format!("cannot write {}: {error}", target.display()))?;
        }
        imported += 1;
    }
    Ok(if dry_run {
        format!("would import {imported} file(s) ({skipped} already present)")
    } else {
        format!(
            "imported {imported} file(s) ({skipped} already present); the next build can reuse what these stamps describe"
        )
    })
}

/// Collect every regular file under `directory`, relative to `base`.
/// 收集 `directory` 下的每个普通文件，路径相对 `base`。
fn visit_files(directory: &Path, base: &Path, out: &mut Vec<PathBuf>) -> Result<(), String> {
    let entries = std::fs::read_dir(directory)
        .map_err(|error| format!("cannot read {}: {error}", directory.display()))?;
    for entry in entries {
        let entry =
            entry.map_err(|error| format!("cannot read {}: {error}", directory.display()))?;
        let path = entry.path();
        if path.is_dir() {
            visit_files(&path, base, out)?;
        } else if path.is_file() {
            out.push(path.strip_prefix(base).unwrap_or(&path).to_path_buf());
        }
    }
    Ok(())
}
