//! Disk cache for a package's face read.
//! 一个包的注册面读取的磁盘缓存。
//!
//! The read walks every source file the layout names, and a fresh process paid that walk again on
//! every answer: measured on this checkout, the workspace census spent 8.6 s deriving seven members'
//! faces, while a build record would have cost a few files. This module caches the **answer** the
//! walk produced, keyed by the **content** of every file it read plus the producing parser's
//! fingerprint — the same rule `source_cache` follows, so no timestamp ever decides a hit.
//! 这次读取会走布局点名的每个源文件，而每个新进程都为每份答案重付这趟遍历：本检出实测，工作区普查为推导
//! 七个成员的面花了 8.6 s，而读一份构建记录只要几个文件。本模块缓存遍历产出的**答案**，键是它读过的每个
//! 文件的**内容**加上产生它的解析器指纹——与 `source_cache` 同一条规则，因此没有任何时间戳决定命中。
//!
//! Not a proof of freshness but of identity: the key names the exact bytes and the exact parser, so
//! a hit returns what those bytes would have produced. `XIRANG_NO_FACE_CACHE=1` turns it off for
//! anyone who wants the walk every time (tests and measurement do).
//! 这不是新鲜性证明，而是同一性证明：键点名的就是那些字节与那个解析器，因此命中返回的就是那些字节会产出的
//! 东西。`XIRANG_NO_FACE_CACHE=1` 为想每次都走一遍的人（测试与测量就是）关掉它。

use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use super::FaceView;
use super::face_view::FaceRead;
use xirang_multilang::source_ref::SourceBase;

/// The record shape this module writes; bump it when the layout changes.
/// 本模块写出的记录形状；布局变化时抬它。
const SCHEMA: &str = "faces-1";

/// A 128-bit FNV-1a over two lanes; deterministic and version-stable, which a `DefaultHasher` is not.
/// 两条泳道的 128 位 FNV-1a；确定且跨版本稳定，而 `DefaultHasher` 不是。
struct Fingerprint {
    low: u64,
    high: u64,
}

impl Fingerprint {
    fn new() -> Self {
        Fingerprint {
            low: 0xcbf2_9ce4_8422_2325,
            high: 0x8422_2325_cbf2_9ce4,
        }
    }

    fn feed(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.low ^= u64::from(*byte);
            self.low = self.low.wrapping_mul(0x0000_0100_0000_01b3);
            self.high ^= u64::from(*byte).rotate_left(32);
            self.high = self.high.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }

    fn hex(&self) -> String {
        format!("{:016x}{:016x}", self.low, self.high)
    }
}

/// The cache key for one package, or `None` when the cache is off or the tree is unreadable.
/// 一个包的缓存键；缓存关闭或树读不了时为 `None`。
pub(crate) fn key(root: &Path, package: &str) -> Option<String> {
    if std::env::var_os("XIRANG_NO_FACE_CACHE").is_some() {
        return None;
    }
    let layout = super::source_layout(root).ok()?;
    let mut files: Vec<(String, u64)> = Vec::new();
    gather(&layout.package_root, &mut files)?;
    files.sort();
    let mut hasher = Fingerprint::new();
    hasher.feed(layout.scan_root.to_string_lossy().as_bytes());
    hasher.feed(package.as_bytes());
    hasher.feed(xirang_kernel::source::parser_fingerprint().as_bytes());
    hasher.feed(SCHEMA.as_bytes());
    for (relative, digest) in files {
        hasher.feed(relative.as_bytes());
        hasher.feed(&digest.to_le_bytes());
    }
    Some(format!("{SCHEMA}-{}", hasher.hex()))
}

/// Every `.rs` file the walk would read, as (relative spelling, content digest).
/// 遍历会读到的每个 `.rs` 文件，按（相对拼写，内容摘要）。
///
/// Directories the walk never enters are skipped by the same names the discovery walk uses; a
/// directory that cannot be read makes the whole key `None`, because a key computed from half the
/// tree would let a later walk answer for bytes it never saw.
/// 遍历从不进入的目录按发现遍历用的同一批名字跳过；读不了的目录会让整个键变成 `None`，因为用半棵树算出的键
/// 会让后来的某次遍历为它没见过的字节作答。
fn gather(root: &Path, out: &mut Vec<(String, u64)>) -> Option<()> {
    gather_in(root, root, out)
}

/// Walk one directory, keeping every file under the package.
/// 走一个目录，保留包内的每个文件。
///
/// The rule is "the whole package, minus build outputs": narrowing it to `.rs` files served a stale
/// answer for a tree whose `.cpp` had moved, and narrowing it to the scan root served one for an
/// island outside `src/`; both were measured red before this widened. A package is a cargo package,
/// not a data archive, so the read cost stays small, and `.xirang/cache` is excluded because a
/// cache's own bytes must not decide a hit.
/// 规则是"整个包减去构建产物"：收窄到 `.rs` 时，`.cpp` 搬走的树返回过期答案；收窄到扫描根时，`src/` 之外
/// 的孤岛也返回过期答案——两次都是先量到红才放宽的。包是 cargo 包、不是数据归档，读的成本仍然小；排除
/// `.xirang/cache`，因为缓存自己的字节绝不能决定命中。
fn gather_in(root: &Path, directory: &Path, out: &mut Vec<(String, u64)>) -> Option<()> {
    let entries = std::fs::read_dir(directory).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        if path.is_dir() {
            // `.xirang/` is read (its move counter and island mappings change the tree's shape),
            // but its cache child is not: a cache's own bytes must not decide a hit.
            // `.xirang/` 要读（它的搬迁计数与孤岛映射改变树的形状），但它的 cache 子目录不读：
            // 缓存自己的字节绝不能决定命中。
            if matches!(
                name.as_str(),
                "target" | ".git" | ".codegraph" | ".agent-teams"
            ) || (name == "cache" && directory.ends_with(".xirang"))
            {
                continue;
            }
            gather_in(root, &path, out)?;
        } else {
            let relative = path
                .strip_prefix(root)
                .unwrap_or(&path)
                .to_string_lossy()
                .to_string();
            let text = std::fs::read(&path).ok()?;
            let mut hasher = Fingerprint::new();
            hasher.feed(&text);
            out.push((relative, hasher.low));
        }
    }
    Some(())
}

/// Where one key's record lives.
/// 一个键的记录住的地方。
fn record(key: &str) -> PathBuf {
    std::env::temp_dir()
        .join("xirang-faces-cache")
        .join(format!("{key}.json"))
}

/// The cached answer, when a complete record for this key exists.
/// 当这个键有一条完整记录时，缓存的答案。
pub(crate) fn read(key: &str) -> Option<FaceRead> {
    let text = std::fs::read_to_string(record(key)).ok()?;
    let value: Value = serde_json::from_str(&text).ok()?;
    decode(&value)
}

/// Store one answer; a failure to write is silence, because a cache miss never changes an answer.
/// 存下一份答案；写失败保持沉默，因为缓存未命中绝不改变答案。
pub(crate) fn write(key: &str, answer: &FaceRead) {
    let path = record(key);
    let Some(parent) = path.parent() else {
        return;
    };
    if std::fs::create_dir_all(parent).is_err() {
        return;
    }
    let text = encode(answer).to_string();
    let temporary = path.with_extension(format!("tmp-{}", std::process::id()));
    if std::fs::write(&temporary, text).is_ok() {
        let _ = std::fs::rename(&temporary, &path);
    }
}

fn encode(answer: &FaceRead) -> Value {
    json!({
        "schema": SCHEMA,
        "faces": answer.0.iter().map(encode_face).collect::<Vec<_>>(),
        "unreadable": answer.1,
        "external": answer.2,
    })
}

fn encode_face(face: &FaceView) -> Value {
    json!({
        "id": face.id.to_string(),
        "stable_name": face.stable_name,
        "parent": face.parent.to_string(),
        "kind": face.kind,
        "registry_name": face.registry_name,
        "module": face.module,
        "source": face.source,
        "source_base": match face.source_base {
            SourceBase::Identity => "identity",
            SourceBase::Package => "package",
        },
        "owns_registry": face.owns_registry,
        "path": face.path,
        "parent_resolved": face.parent_resolved,
    })
}

fn decode(value: &Value) -> Option<FaceRead> {
    if value.get("schema").and_then(Value::as_str) != Some(SCHEMA) {
        return None;
    }
    let faces = value
        .get("faces")?
        .as_array()?
        .iter()
        .map(decode_face)
        .collect::<Option<Vec<_>>>()?;
    Some((
        faces,
        strings(value.get("unreadable")),
        strings(value.get("external")),
    ))
}

fn decode_face(value: &Value) -> Option<FaceView> {
    let text = |name: &str| value.get(name).and_then(Value::as_str).map(str::to_owned);
    Some(FaceView {
        id: node(value.get("id")?)?,
        stable_name: value
            .get("stable_name")
            .and_then(Value::as_str)
            .map(str::to_owned),
        parent: node(value.get("parent")?)?,
        kind: text("kind")?,
        registry_name: text("registry_name")?,
        module: text("module")?,
        source: text("source")?,
        source_base: match value.get("source_base").and_then(Value::as_str)? {
            "identity" => SourceBase::Identity,
            "package" => SourceBase::Package,
            _ => return None,
        },
        owns_registry: value.get("owns_registry").and_then(Value::as_bool)?,
        path: text("path")?,
        parent_resolved: value.get("parent_resolved").and_then(Value::as_bool)?,
    })
}

fn node(value: &Value) -> Option<xirang_kernel::identity::NodeId> {
    let bytes = xirang_kernel::identity::hex_decode(value.as_str()?)?;
    let array: [u8; 16] = bytes.try_into().ok()?;
    Some(xirang_kernel::identity::NodeId::from_raw(array))
}

fn strings(value: Option<&Value>) -> Vec<String> {
    value
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
#[path = "face_cache_tests.rs"]
mod face_cache_tests;
