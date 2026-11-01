//! Collect content-store entries that no stamp references.
//! 收集没有任何印记引用的内容库条目。
//!
//! The content store is shared by every plan under one `target/xirang`, so it cannot be collected
//! by "belongs to this build": the question is whether any stamp still names the digest. A stamp
//! this build cannot read makes the whole collection refuse — pruning on a guess would delete bytes
//! a later restore needs, and a deletion that cannot be explained is worse than a missed one.
//! 内容库被同一个 `target/xirang` 下的每份计划共享，因此不能按“属于这次构建”来回收：问题是还有没有
//! 任何印记点名该摘要。本构建读不懂的印记会让整次回收拒绝——靠猜剪枝会删掉之后恢复要用的字节，而
//! 解释不清的删除比漏删更糟。

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// Walk `directory` recursively, collecting regular files into `files`.
/// 递归遍历 `directory`，把普通文件收进 `files`。
fn walk_files(directory: &Path, files: &mut Vec<PathBuf>) -> Result<(), String> {
    let entries = std::fs::read_dir(directory)
        .map_err(|error| format!("cannot read {}: {error}", directory.display()))?;
    for entry in entries {
        let entry =
            entry.map_err(|error| format!("cannot read {}: {error}", directory.display()))?;
        let path = entry.path();
        if path.is_dir() {
            walk_files(&path, files)?;
        } else if path.is_file() {
            files.push(path);
        }
    }
    Ok(())
}

/// Collect one content store, reporting every entry it removes.
/// 回收一个内容库，并报告它删掉的每个条目。
pub(super) fn collect(xirang: &Path, dry_run: bool) -> Result<String, String> {
    let cas = xirang.join("cas");
    if !cas.is_dir() {
        return Ok(format!(
            "no content store at {}: nothing to collect",
            cas.display()
        ));
    }
    let mut files = Vec::new();
    let builds = xirang.join("build");
    if builds.is_dir() {
        walk_files(&builds, &mut files)?;
    }
    let mut live = BTreeSet::new();
    let mut stamps = 0;
    for path in &files {
        let is_stamp = path
            .parent()
            .and_then(|parent| parent.file_name())
            .is_some_and(|name| name == "stamps")
            && path.extension().and_then(|extension| extension.to_str()) == Some("json");
        if !is_stamp {
            continue;
        }
        let text = std::fs::read_to_string(path)
            .map_err(|error| format!("cannot read the stamp {}: {error}", path.display()))?;
        let value: serde_json::Value = serde_json::from_str(&text)
            .map_err(|error| format!("cannot read the stamp {}: {error}", path.display()))?;
        let outputs = value
            .get("outputs")
            .and_then(|outputs| outputs.as_array())
            .ok_or_else(|| {
                format!(
                    "the stamp {} has no `outputs`; refusing to collect on a guess",
                    path.display()
                )
            })?;
        for output in outputs {
            let digest = output
                .get("sha256")
                .and_then(|digest| digest.as_str())
                .ok_or_else(|| {
                    format!(
                        "an output in the stamp {} has no `sha256`; refusing to collect on a guess",
                        path.display()
                    )
                })?;
            live.insert(digest.to_owned());
        }
        stamps += 1;
    }
    let entries = std::fs::read_dir(&cas)
        .map_err(|error| format!("cannot read {}: {error}", cas.display()))?;
    let mut candidates = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|error| format!("cannot read {}: {error}", cas.display()))?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        let size = entry
            .metadata()
            .map(|metadata| metadata.len())
            .unwrap_or_default();
        candidates.push((name, path, size));
    }
    candidates.sort();
    let mut removed = Vec::new();
    let mut kept = 0;
    let mut ignored = 0;
    let mut removed_bytes = 0u64;
    for (name, path, size) in candidates {
        let address =
            name.len() == 64 && name.chars().all(|character| character.is_ascii_hexdigit());
        if !address {
            ignored += 1;
            continue;
        }
        if live.contains(&name) {
            kept += 1;
            continue;
        }
        removed.push((name, size));
        removed_bytes += size;
        if !dry_run {
            std::fs::remove_file(&path)
                .map_err(|error| format!("cannot remove {}: {error}", path.display()))?;
        }
    }
    let verb = if dry_run { "would remove" } else { "removed" };
    let mut out = format!(
        "{verb} {} entries ({removed_bytes} bytes); kept {kept} referenced by {stamps} stamp(s)",
        removed.len()
    );
    for (name, size) in &removed {
        out.push_str(&format!("\n  {verb} {name} ({size} bytes)"));
    }
    if ignored > 0 {
        out.push_str(&format!(
            "\n  left {ignored} entries alone: their names are not a 64-digit content address"
        ));
    }
    Ok(out)
}
