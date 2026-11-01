//! The source stamp: how recently anything a Studio session authors under changed,
//! as one number.
//! 源码戳：一个 Studio 会话撰写其下的东西最近变了什么——一个数字。
//!
//! One recursive walk per poll, hashed in path/mtime/size order; the hot-reload path
//! and the call-tree cache both key on it. This module also owns the answer to "which
//! files count": `xirang-dev`'s watcher keeps the same name list, by mirror rather
//! than by import — the supervisor is a binary, so a `pub(crate)` predicate is not
//! visible to it (audit `STU-S-09`, `STU-S-26`).
//! 每次轮询走一趟递归，按路径/修改时间/大小哈希；热重载路径与调用树缓存都以它为键。本模块也
//! 拥有“哪些文件算数”的答案：`xirang-dev` 的 watcher 保有同一份名单，靠镜像而不是 import
//! ——监督器是二进制，看不到 `pub(crate)` 谓词（审计 `STU-S-09`、`STU-S-26`）。

use std::path::Path;
use std::time::UNIX_EPOCH;

use super::project_context::package_root;

/// The file names that make a changed file relevant to the stamp, next to the `.rs`
/// sources: a manifest or lockfile can change what the host admits without touching
/// a source file, and `build.rs` can change what gets registered.
/// 除 `.rs` 源码之外，让一个变更文件与戳相关的文件名：清单或锁文件可以不碰源码就改变宿主准入
/// 什么，`build.rs` 则可以改变被注册的东西。
pub(super) const RELEVANT_FILE_NAMES: [&str; 5] = [
    "Cargo.toml",
    "Cargo.lock",
    "build.rs",
    "official.lock",
    "user.lock",
];

/// Whether one file the walk met belongs in the stamp.
/// 递归遇到的一个文件是否属于这个戳。
pub(super) fn is_relevant_source_path(path: &Path) -> bool {
    path.extension().and_then(|extension| extension.to_str()) == Some("rs")
        || path.file_name().is_some_and(|name| {
            // Both plugin locks count, not just the official one: writing `user.lock`
            // changes what the host admits exactly as much as writing `official.lock`.
            // The same five names are mirrored in `studio/src/bin/xirang-dev.rs`.
            // 两个插件锁都算数，不只是官方那个：写 `user.lock` 与写 `official.lock` 一样改变
            // 宿主准入什么。同一份五个名字镜像在 `studio/src/bin/xirang-dev.rs`。
            RELEVANT_FILE_NAMES
                .iter()
                .any(|known| name.to_str() == Some(known))
        })
}

pub(super) fn source_stamp() -> u128 {
    let package_root = package_root();
    let mut files = Vec::new();
    for root in [
        package_root.join("src"),
        package_root.join("toolchain/src/studio"),
        package_root.join(".xirang/plugins"),
        // A plan is an authoring record: editing or deleting one must refresh
        // the graft screen, even though the registration tree does not change.
        // 计划是创作记录：编辑或删除它必须刷新 graft 界面，尽管注册树本身没变。
        package_root
            .join(crate::run_method::lexicon::XIRANG_DIR)
            .join(crate::run_method::lexicon::EXTERNAL_GRAFT_DIR),
    ] {
        stamp_directory(&root, &mut files);
    }
    // The walk above never sees the package root itself, so these two are stamped
    // by name — the same names the predicate above accepts.
    // 上面的递归看不到包根本身，因此这两个按名字入戳——正是上面谓词接受的那些名字。
    for file in [
        package_root.join(RELEVANT_FILE_NAMES[0]),
        package_root.join(RELEVANT_FILE_NAMES[2]),
    ] {
        stamp_file(&file, &mut files);
    }
    files.sort_by(|left, right| left.0.cmp(&right.0));
    let mut stamp = 0u128;
    for (path, modified, size) in files {
        for byte in path.to_string_lossy().bytes() {
            stamp = stamp.wrapping_mul(1_000_003).wrapping_add(u128::from(byte));
        }
        stamp = stamp.wrapping_mul(1_000_003).wrapping_add(modified);
        stamp = stamp.wrapping_mul(1_000_003).wrapping_add(u128::from(size));
    }
    stamp
}

fn stamp_file(path: &std::path::Path, files: &mut Vec<(std::path::PathBuf, u128, u64)>) {
    let Ok(metadata) = std::fs::metadata(path) else {
        return;
    };
    let modified = metadata
        .modified()
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map_or(0, |duration| duration.as_nanos());
    files.push((path.to_owned(), modified, metadata.len()));
}

pub(super) fn stamp_directory(
    path: &std::path::Path,
    files: &mut Vec<(std::path::PathBuf, u128, u64)>,
) {
    let Ok(entries) = std::fs::read_dir(path) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(metadata) = entry.metadata() else {
            continue;
        };
        if metadata.is_dir() {
            if path
                .file_name()
                .is_some_and(|name| name == "target" || name == ".git")
            {
                continue;
            }
            stamp_directory(&path, files);
            continue;
        }
        if !metadata.is_file() {
            continue;
        }
        if !is_relevant_source_path(&path) {
            continue;
        }
        let modified = metadata
            .modified()
            .ok()
            .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
            .map_or(0, |duration| duration.as_nanos());
        files.push((path, modified, metadata.len()));
    }
}
