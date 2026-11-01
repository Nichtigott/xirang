//! The file facts a plan identity is computed from, cached between builds.
//! 计划身份所依据的文件事实，在两次构建之间缓存。
//!
//! The cache never decides that a file is unchanged from its modification time alone. A hit needs
//! the same inode, size, modification time **and** change time: writing a file updates its change
//! time even when a program restores the modification time afterwards, so a same-size, same-mtime
//! rewrite is still a miss and is still hashed. A cache this build cannot read is treated as empty
//! — the next build reads and hashes every file rather than trusting a damaged record — and a
//! platform without the Unix metadata key keeps the cache off entirely.
//! 缓存绝不只凭修改时间判定文件没变。命中要求同一个 inode、大小、修改时间**与**变更时间：写文件会更新
//! 变更时间，哪怕之后把修改时间改回去，因此“大小与 mtime 都相同”的重写仍然不命中、仍然重新哈希。
//! 本构建读不懂的缓存按空处理——下一次构建重新读取并哈希每个文件，而不是相信一份坏记录；没有 Unix
//! 元数据键的平台直接不启用缓存。

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

/// What the filesystem said about one file when its digest was computed.
/// 计算摘要时文件系统对某个文件的说法。
#[derive(Clone, Copy, PartialEq, Eq)]
struct Key {
    inode: u64,
    size: u64,
    mtime: i64,
    mtime_ns: i64,
    ctime: i64,
    ctime_ns: i64,
}

/// One cached file.
/// 一个被缓存的文件。
struct Entry {
    key: Key,
    digest: String,
}

/// The facts of one `target/xirang` tree: a cache best-effort by design.
/// 一个 `target/xirang` 树的事实：按设计是尽力而为的缓存。
pub struct Facts {
    path: PathBuf,
    entries: BTreeMap<PathBuf, Entry>,
    seen: BTreeSet<PathBuf>,
    enabled: bool,
}

#[cfg(unix)]
fn key(metadata: &std::fs::Metadata) -> Option<Key> {
    use std::os::unix::fs::MetadataExt;
    Some(Key {
        inode: metadata.ino(),
        size: metadata.len(),
        mtime: metadata.mtime(),
        mtime_ns: metadata.mtime_nsec(),
        ctime: metadata.ctime(),
        ctime_ns: metadata.ctime_nsec(),
    })
}

#[cfg(not(unix))]
fn key(_metadata: &std::fs::Metadata) -> Option<Key> {
    None
}

impl Facts {
    /// Load the cache under one `target/xirang`; a missing or unreadable cache is empty.
    ///
    /// `may_create` says whether this caller owns the tree: a build may create `target/xirang` and
    /// write the cache, while a verification pass must stay read-only and only reuse a cache that is
    /// already there (a candidate slot has no `target/` at all).
    /// 读取一个 `target/xirang` 下的缓存；缺失或读不懂的缓存就是空的。
    ///
    /// `may_create` 说明调用方是否拥有这棵树：构建可以创建 `target/xirang` 并写缓存，而验证过程必须
    /// 只读，只复用已经存在的缓存（候选槽根本没有 `target/`）。
    /// Compatibility alias for the published `Facts::load` spelling.
    /// 已发布拼法 `Facts::load` 的兼容别名。
    #[deprecated(note = "use Facts::load_facts; this alias keeps 0.2 callers compiling")]
    pub fn load(xirang: &Path, may_create: bool) -> Facts {
        Self::load_facts(xirang, may_create)
    }

    /// Loads the cached facts of one `target/xirang` tree.
    /// 读取一个 `target/xirang` 树的缓存事实。
    pub fn load_facts(xirang: &Path, may_create: bool) -> Facts {
        let path = xirang.join("facts.json");
        let mut facts = Facts {
            path,
            entries: BTreeMap::new(),
            seen: BTreeSet::new(),
            enabled: cfg!(unix) && (may_create || xirang.is_dir()),
        };
        if !facts.enabled {
            return facts;
        }
        let Ok(text) = std::fs::read_to_string(&facts.path) else {
            return facts;
        };
        let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) else {
            return facts;
        };
        if value.get("version").and_then(|version| version.as_u64()) != Some(1) {
            return facts;
        }
        let Some(files) = value.get("files").and_then(|files| files.as_object()) else {
            return facts;
        };
        for (name, entry) in files {
            let (
                Some(inode),
                Some(size),
                Some(mtime),
                Some(mtime_ns),
                Some(ctime),
                Some(ctime_ns),
                Some(digest),
            ) = (
                entry.get("inode").and_then(|value| value.as_u64()),
                entry.get("size").and_then(|value| value.as_u64()),
                entry.get("mtime").and_then(|value| value.as_i64()),
                entry.get("mtime_ns").and_then(|value| value.as_i64()),
                entry.get("ctime").and_then(|value| value.as_i64()),
                entry.get("ctime_ns").and_then(|value| value.as_i64()),
                entry.get("sha256").and_then(|value| value.as_str()),
            )
            else {
                continue;
            };
            facts.entries.insert(
                PathBuf::from(name),
                Entry {
                    key: Key {
                        inode,
                        size,
                        mtime,
                        mtime_ns,
                        ctime,
                        ctime_ns,
                    },
                    digest: digest.to_owned(),
                },
            );
        }
        facts
    }

    /// The digest of one file, computed from its bytes when the cache does not hold these facts.
    /// 一个文件的摘要；缓存里没有这些事实时从字节重新计算。
    pub fn digest(&mut self, file: &Path) -> Result<String, String> {
        self.seen.insert(file.to_path_buf());
        let metadata = std::fs::metadata(file).ok();
        let key = metadata.as_ref().and_then(key);
        if let (Some(key), Some(entry)) = (key, self.entries.get(file))
            && entry.key == key
        {
            return Ok(entry.digest.clone());
        }
        let bytes = std::fs::read(file)
            .map_err(|error| format!("cannot read {}: {error}", file.display()))?;
        let digest = xirang_kernel::sha256_hex(&bytes);
        if let Some(key) = key
            && self.enabled
        {
            self.entries.insert(
                file.to_path_buf(),
                Entry {
                    key,
                    digest: digest.clone(),
                },
            );
        }
        Ok(digest)
    }

    /// Write the facts this build actually used back; entries not seen this run are dropped.
    /// 把这次构建真正用到的事实写回；本次没有见到的条目会被丢弃。
    pub fn save(&self) {
        if !self.enabled || self.seen.is_empty() {
            return;
        }
        let mut files = serde_json::Map::new();
        for path in &self.seen {
            let Some(entry) = self.entries.get(path) else {
                continue;
            };
            files.insert(
                path.display().to_string(),
                serde_json::json!({
                    "inode": entry.key.inode,
                    "size": entry.key.size,
                    "mtime": entry.key.mtime,
                    "mtime_ns": entry.key.mtime_ns,
                    "ctime": entry.key.ctime,
                    "ctime_ns": entry.key.ctime_ns,
                    "sha256": entry.digest,
                }),
            );
        }
        let document = serde_json::json!({ "version": 1, "files": files });
        let Ok(text) = serde_json::to_string(&document) else {
            return;
        };
        let Some(parent) = self.path.parent() else {
            return;
        };
        if std::fs::create_dir_all(parent).is_err() {
            return;
        }
        let temporary = self.path.with_extension("json.tmp");
        if std::fs::write(&temporary, text).is_err() {
            return;
        }
        let _ = std::fs::rename(&temporary, &self.path);
    }
}
