//! The local reuse record for compile actions.
//! 编译动作的本地复用记录。
//!
//! A stamp names the plan identity and the bytes each action wrote. An action is reused only when
//! the plan identity matches the stamp **and** every output still hashes to the recorded digest;
//! a missing or edited output is restored from the content store when those exact bytes are still
//! there, and otherwise makes the action run again. Identity is the same one the plan already has
//! (sources, manifests, toolchain versions, language standards, exports), so a source or compiler
//! change lands in a different build directory and never sees these stamps.
//! 印记记下计划身份与该动作写出的字节。只有计划身份与印记相符、**并且**每个产物仍与记录摘要相符时
//! 才复用；产物缺失或被改动时，若内容库仍存有那批字节就从内容库恢复，否则动作重新运行。身份就是计划
//! 已有的那一个（源码、清单、工具链版本、语言标准、导出），因此源码或编译器变化会落在另一个构建目录，
//! 永远看不到这些印记。
//!
//! Tests are never reused: a test verdict is a credential, and this record is about compilation.
//! 测试永不复用：测试判定是凭据，而这份记录只关于编译。

use std::path::PathBuf;

use super::project::Plan;

/// What the reuse record says about one action.
/// 复用记录对一个动作的判定。
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Decision {
    /// No usable stamp: the action must run.
    /// 没有可用的印记：动作必须运行。
    Run,
    /// Every output still holds; nothing was written.
    /// 每个产物都仍在原处；没有写任何东西。
    Reused,
    /// Every output holds, and at least one was restored from the content store.
    /// 每个产物都就位，且至少一个是从内容库恢复的。
    Restored,
}

/// The stamp of one action in one plan's directory.
/// 一份计划目录里某个动作的印记。
fn stamp_path(plan: &Plan, index: usize) -> PathBuf {
    plan.directory
        .join("stamps")
        .join(format!("{index:04}.json"))
}

/// Where the bytes of one output live: addressed by their own digest, shared by every plan under
/// the same `target/xirang` directory.
/// 一个产物的字节住在哪里：按它自己的摘要寻址，被同一个 `target/xirang` 下的每份计划共享。
fn content_path(plan: &Plan, digest: &str) -> Option<PathBuf> {
    let xirang = plan.directory.parent()?.parent()?;
    Some(xirang.join("cas").join(digest))
}

/// Store the bytes of one output in the content store, and return their digest.
/// 把一个产物的字节存进内容库，并返回它的摘要。
fn store(plan: &Plan, path: &PathBuf) -> Result<String, String> {
    let bytes =
        std::fs::read(path).map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    let digest = xirang_kernel::sha256_hex(&bytes);
    let Some(destination) = content_path(plan, &digest) else {
        return Ok(digest);
    };
    if destination.is_file() {
        return Ok(digest);
    }
    if let Some(parent) = destination.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("cannot create {}: {error}", parent.display()))?;
    }
    std::fs::write(&destination, &bytes)
        .map_err(|error| format!("cannot write {}: {error}", destination.display()))?;
    Ok(digest)
}

/// Record what an action wrote, keyed to the plan identity, and put a copy of each output in the
/// content store so a later run can restore it without recompiling.
/// 记录一个动作写出了什么（以计划身份为键），并把每个产物复制进内容库，使之后的运行不必重新编译就能恢复。
pub(super) fn record(plan: &Plan, index: usize, outputs: &[PathBuf]) -> Result<(), String> {
    let mut entries = Vec::new();
    for path in outputs {
        let digest = store(plan, path)?;
        // The stamp records the output **relative to the build directory**, so a stamp can cross
        // checkouts (`xirang cache export`/`import`) while still naming the same file.
        // 印记以**构建目录相对**的形式记录产物，因此印记可以跨检出（`xirang cache export`/`import`）
        // 而仍然指名同一个文件。
        let relative = path
            .strip_prefix(&plan.directory)
            .map(|path| path.display().to_string())
            .unwrap_or_else(|_| path.display().to_string());
        entries.push(serde_json::json!({
            "path": relative,
            "sha256": digest,
        }));
    }
    let path = stamp_path(plan, index);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("cannot create {}: {error}", parent.display()))?;
    }
    // A stamp names the plan, the action, and the bytes: `attempt` refuses a stamp that does not
    // agree on all three, so an edited or misplaced stamp makes the action run instead of being
    // believed. Reuse therefore depends only on the plan identity, this record, and bytes whose
    // own digest names them — never on an index elsewhere that could drift.
    // 印记同时点名计划、动作与字节：三者不一致时 `attempt` 一律当作不可用，改过的或放错位置的印记
    // 只会让动作重跑，不会被相信。于是复用只依赖计划身份、这份记录、以及以自身摘要命名的字节，
    // 不依赖别处可能漂移的索引。
    let document =
        serde_json::json!({ "version": 1, "plan": plan.id, "index": index, "outputs": entries });
    std::fs::write(&path, format!("{}\n", document))
        .map_err(|error| format!("cannot write {}: {error}", path.display()))
}

/// Whether an action can be skipped, and whether skipping it meant restoring bytes.
/// 一个动作能否跳过，以及跳过它是否意味着恢复了字节。
pub(super) fn attempt(plan: &Plan, index: usize, outputs: &[PathBuf]) -> Decision {
    if outputs.is_empty() {
        return Decision::Run;
    }
    let Ok(text) = std::fs::read_to_string(stamp_path(plan, index)) else {
        return Decision::Run;
    };
    let Ok(document) = serde_json::from_str::<serde_json::Value>(&text) else {
        return Decision::Run;
    };
    if document["version"].as_u64() != Some(1) {
        return Decision::Run;
    }
    if document["plan"] != serde_json::Value::String(plan.id.clone()) {
        return Decision::Run;
    }
    if document["index"].as_u64() != Some(index as u64) {
        return Decision::Run;
    }
    let Some(entries) = document["outputs"].as_array() else {
        return Decision::Run;
    };
    if entries.len() != outputs.len() {
        return Decision::Run;
    }
    let mut restored = false;
    for (entry, path) in entries.iter().zip(outputs) {
        let relative = path
            .strip_prefix(&plan.directory)
            .map(|path| path.display().to_string())
            .unwrap_or_else(|_| path.display().to_string());
        if entry["path"] != serde_json::Value::String(relative) {
            return Decision::Run;
        }
        let Some(digest) = entry["sha256"].as_str() else {
            return Decision::Run;
        };
        let holds =
            std::fs::read(path).is_ok_and(|bytes| xirang_kernel::sha256_hex(&bytes) == digest);
        if holds {
            continue;
        }
        let Some(source) = content_path(plan, digest) else {
            return Decision::Run;
        };
        let Ok(bytes) = std::fs::read(&source) else {
            return Decision::Run;
        };
        if xirang_kernel::sha256_hex(&bytes) != digest {
            return Decision::Run;
        }
        if let Some(parent) = path.parent()
            && std::fs::create_dir_all(parent).is_err()
        {
            return Decision::Run;
        }
        if std::fs::write(path, &bytes).is_err() {
            return Decision::Run;
        }
        restored = true;
    }
    if restored {
        Decision::Restored
    } else {
        Decision::Reused
    }
}
