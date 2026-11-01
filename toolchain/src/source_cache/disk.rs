//! Disk-backed syntax facts for the MCP surfaces.
//! MCP 执行面的磁盘语法事实。
//!
//! `facts` keeps a parsed unit in memory, keyed by the source text; a **fresh process** paid the
//! kernel parse again for every file a query mentions, which is what made a common-name `callgraph`
//! query on this checkout cost 22 s after the face read had been cached. This module gives the same
//! unit a content-addressed record: the key names the exact bytes and the producing parser, so a hit
//! returns what those bytes would have produced — no timestamp decides it, the same rule
//! `face_cache` and `source_cache` follow.
//! `facts` 把解析单元按源码文本缓存在内存里；**新进程**会为查询提到的每个文件重付内核解析，这正是本检出上
//! 常见名 `callgraph` 在面读取缓存之后仍要 22 s 的原因。本模块给同一个单元一份内容寻址记录：键点名确切字节与
//! 产生它的解析器，因此命中返回的就是那些字节会产出的东西——没有时间戳决定命中，与 `face_cache`、
//! `source_cache` 同一条规则。
//!
//! `XIRANG_NO_SYNTAX_CACHE=1` turns it off (measurement and A/B use it).
//! `XIRANG_NO_SYNTAX_CACHE=1` 关闭它（测量与 A/B 用）。

use std::path::PathBuf;
use std::sync::{Arc, OnceLock};

use serde_json::{Value, json};

use xirang_kernel::source::SourceFunction;

use super::facts::Unit;

/// The record shape; bump it when `Unit`'s meaning changes.
/// 记录形状；`Unit` 的含义变化时抬它。
const SCHEMA: &str = "syntax-facts-1";

/// The directory one parser fingerprint writes into.
/// 一个解析器指纹写进的目录。
fn directory() -> PathBuf {
    std::env::temp_dir()
        .join("xirang-syntax-cache")
        .join(format!(
            "{SCHEMA}-{}",
            xirang_kernel::source::parser_fingerprint()
        ))
}

/// A 128-bit FNV-1a over the source; deterministic and version-stable.
/// 对源码的 128 位 FNV-1a；确定且跨版本稳定。
fn fingerprint(source: &str) -> String {
    let (mut low, mut high) = (0xcbf2_9ce4_8422_2325u64, 0x8422_2325_cbf2_9ce4u64);
    for byte in source.as_bytes() {
        low ^= u64::from(*byte);
        low = low.wrapping_mul(0x0000_0100_0000_01b3);
        high ^= u64::from(*byte).rotate_left(32);
        high = high.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{low:016x}{high:016x}-{:x}", source.len())
}

fn record(source: &str) -> Option<PathBuf> {
    if std::env::var_os("XIRANG_NO_SYNTAX_CACHE").is_some() {
        return None;
    }
    Some(directory().join(format!("{}.json", fingerprint(source))))
}

/// The cached unit for this exact source, when a complete record exists.
/// 这份确切源码的缓存单元，当完整记录存在时。
pub(crate) fn load(source: &str) -> Option<Arc<Unit>> {
    let text = std::fs::read_to_string(record(source)?).ok()?;
    let value: Value = serde_json::from_str(&text).ok()?;
    if value.get("schema").and_then(Value::as_str) != Some(SCHEMA) {
        return None;
    }
    let functions = value
        .get("functions")?
        .as_array()?
        .iter()
        .map(function)
        .collect::<Option<Vec<_>>>()?;
    let calls = value
        .get("calls")?
        .as_array()?
        .iter()
        .map(|row| {
            row.as_array().map(|items| {
                items
                    .iter()
                    .filter_map(Value::as_str)
                    .map(str::to_owned)
                    .collect::<Vec<_>>()
            })
        })
        .collect::<Option<Vec<_>>>()?;
    if calls.len() != functions.len() {
        return None;
    }
    Some(Arc::new(Unit {
        functions,
        calls,
        branches: OnceLock::new(),
    }))
}

/// Store one unit; a write failure is silence, because a cache miss never changes an answer.
/// 存下一个单元；写失败保持沉默，因为缓存未命中绝不改变答案。
pub(crate) fn store(source: &str, unit: &Unit) {
    let Some(path) = record(source) else {
        return;
    };
    let Some(parent) = path.parent() else {
        return;
    };
    if std::fs::create_dir_all(parent).is_err() {
        return;
    }
    let value = json!({
        "schema": SCHEMA,
        "functions": unit.functions.iter().map(function_value).collect::<Vec<_>>(),
        "calls": unit.calls,
    });
    let temporary = path.with_extension(format!("tmp-{}", std::process::id()));
    if std::fs::write(&temporary, value.to_string()).is_ok() {
        let _ = std::fs::rename(&temporary, &path);
    }
}

fn function_value(function: &SourceFunction) -> Value {
    json!({
        "name": function.name,
        "signature": function.signature,
        "body": function.body,
        "line": function.line,
        "end_line": function.end_line,
    })
}

fn function(value: &Value) -> Option<SourceFunction> {
    let text = |name: &str| value.get(name).and_then(Value::as_str).map(str::to_owned);
    Some(SourceFunction {
        name: text("name")?,
        signature: text("signature")?,
        body: text("body")?,
        line: u32::try_from(value.get("line")?.as_u64()?).ok()?,
        end_line: u32::try_from(value.get("end_line")?.as_u64()?).ok()?,
    })
}

#[cfg(test)]
#[path = "disk_tests.rs"]
mod disk_tests;
