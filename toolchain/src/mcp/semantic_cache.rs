//! Disk cache for the `syn`-derived semantic functions of one Rust source.
//! 一份 Rust 源码的 `syn` 语义函数的磁盘缓存。
//!
//! The expensive half of indexing a file is not the kernel's lexical facts (cheap, and already
//! cached by `source_cache`): `functions_from` first tries `semantic_sources::rust`, which parses the
//! file with `syn`. A fresh process paid that parse again for every file a query mentions — measured
//! on this checkout, a common-name `callgraph` query spent **14 ms per file** in `parse_in_place`,
//! 8 s of it after the lexical facts were already cached.
//! 索引一个文件的贵的一半不是内核的词法事实（便宜，且已被 `source_cache` 缓存）：`functions_from` 先试
//! `semantic_sources::rust`，而它用 `syn` 解析文件。新进程会为查询提到的每个文件重付这次解析——本检出
//! 实测：常见名 `callgraph` 查询在 `parse_in_place` 里每个文件 **14 ms**，在词法事实已缓存之后仍占 8 s。
//!
//! The key carries the exact bytes and this bridge's own version, so a hit returns what those bytes
//! would have produced; `XIRANG_NO_SEMANTIC_CACHE=1` turns it off for measurement.
//! 键带着确切字节与本桥自己的版本，因此命中返回的就是那些字节会产出的东西；`XIRANG_NO_SEMANTIC_CACHE=1`
//! 为测量关闭它。

use std::path::PathBuf;

use serde_json::{Value, json};

use crate::mcp::source_index::Function;

/// The record shape; bump it when the mapping from `syn` facts to [`Function`] changes.
/// 记录形状；从 `syn` 事实到 [`Function`] 的映射变化时抬它。
const SCHEMA: &str = "semantic-functions-1";

/// What one source's record says.
/// 一份源码的记录说什么。
pub(crate) enum Cached {
    /// The file parses, with these functions.
    /// 文件解析成功，函数如下。
    Parsed(Vec<Function>),
    /// The file does not parse as Rust: recorded so the next process does not try again.
    /// 文件不能作为 Rust 解析：记录下来，下一个进程不必再试。
    Unparsable,
}

fn directory() -> PathBuf {
    std::env::temp_dir()
        .join("xirang-semantic-cache")
        .join(format!("{SCHEMA}-{}", env!("CARGO_PKG_VERSION")))
}

fn record(source: &str) -> Option<PathBuf> {
    if std::env::var_os("XIRANG_NO_SEMANTIC_CACHE").is_some() {
        return None;
    }
    let (mut low, mut high) = (0xcbf2_9ce4_8422_2325u64, 0x8422_2325_cbf2_9ce4u64);
    for byte in source.as_bytes() {
        low ^= u64::from(*byte);
        low = low.wrapping_mul(0x0000_0100_0000_01b3);
        high ^= u64::from(*byte).rotate_left(32);
        high = high.wrapping_mul(0x0000_0100_0000_01b3);
    }
    Some(directory().join(format!("{low:016x}{high:016x}-{:x}.json", source.len())))
}

/// The record for this exact source, when one exists.
/// 这份确切源码的记录，当存在时。
pub(crate) fn cached(source: &str) -> Option<Cached> {
    let text = std::fs::read_to_string(record(source)?).ok()?;
    let value: Value = serde_json::from_str(&text).ok()?;
    if value.get("schema").and_then(Value::as_str) != Some(SCHEMA) {
        return None;
    }
    if value.get("parsed").and_then(Value::as_bool) != Some(true) {
        return Some(Cached::Unparsable);
    }
    let functions = value
        .get("functions")?
        .as_array()?
        .iter()
        .map(function)
        .collect::<Option<Vec<_>>>()?;
    Some(Cached::Parsed(functions))
}

/// Remember what `syn` said about this source; a write failure is silence.
/// 记住 `syn` 对这份源码说了什么；写失败保持沉默。
pub(crate) fn remember(source: &str, parsed: Option<&[Function]>) {
    let Some(path) = record(source) else {
        return;
    };
    let Some(parent) = path.parent() else {
        return;
    };
    if std::fs::create_dir_all(parent).is_err() {
        return;
    }
    let value = match parsed {
        Some(functions) => json!({
            "schema": SCHEMA,
            "parsed": true,
            "functions": functions.iter().map(function_value).collect::<Vec<_>>(),
        }),
        None => json!({"schema": SCHEMA, "parsed": false}),
    };
    let temporary = path.with_extension(format!("tmp-{}", std::process::id()));
    if std::fs::write(&temporary, value.to_string()).is_ok() {
        let _ = std::fs::rename(&temporary, &path);
    }
}

fn function_value(function: &Function) -> Value {
    json!({
        "name": function.name,
        "owner": function.owner,
        "qualified": function.qualified,
        "line": function.line,
        "end_line": function.end_line,
        "calls": function.calls,
        "unresolved": function.unresolved,
        "caveats": function.caveats,
    })
}

fn function(value: &Value) -> Option<Function> {
    let text = |name: &str| value.get(name).and_then(Value::as_str).map(str::to_owned);
    let list = |name: &str| {
        value.get(name)?.as_array().map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect::<Vec<_>>()
        })
    };
    Some(Function {
        name: text("name")?,
        owner: value
            .get("owner")
            .and_then(Value::as_str)
            .map(str::to_owned),
        qualified: text("qualified")?,
        line: usize::try_from(value.get("line")?.as_u64()?).ok()?,
        end_line: usize::try_from(value.get("end_line")?.as_u64()?).ok()?,
        calls: list("calls")?,
        unresolved: list("unresolved")?,
        caveats: list("caveats")?,
        semantic: true,
        registration: None,
        binding: None,
    })
}

#[cfg(test)]
#[path = "semantic_cache_tests.rs"]
mod semantic_cache_tests;
