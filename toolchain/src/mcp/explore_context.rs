//! Explicit context ownership and source scope, without connection-level answer caching.
//! 显式上下文持有声明与源码范围；不把连接当会话、不缓存 MCP 答案。
//!
//! online: receipts are checked against current source and the exact rendered evidence.
//! 回执必须和当前源码、实际渲染的证据逐字核对。

use std::collections::BTreeSet;
use std::path::Path;

use serde_json::Value;

pub(super) struct Context {
    pub(super) scopes: Vec<String>,
    known: Option<BTreeSet<String>>,
    root: String,
}

impl Context {
    pub(super) fn from_request(root: &Path, arguments: &Value) -> Result<Self, String> {
        let scopes = strings(arguments, "scope", 16)?.unwrap_or_default();
        let known = strings(arguments, "known_context", 128)?;
        if known.as_ref().is_some_and(|values| {
            values.iter().any(|value| {
                value.len() != 64
                    || !value
                        .bytes()
                        .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
            })
        }) {
            return Err(
                "known_context must contain SHA-256 receipts from retained source context".into(),
            );
        }
        Ok(Self {
            scopes,
            known: known.map(|values| values.into_iter().collect()),
            root: root
                .canonicalize()
                .map_err(|error| format!("source root: {error}"))?
                .to_string_lossy()
                .into_owned(),
        })
    }

    pub(super) fn scope_notice(&self) -> String {
        if self.scopes.is_empty() {
            String::new()
        } else {
            format!(
                "requested scope: {} path(s); outside-scope definitions/callers/tests were not examined; omit scope to widen\n",
                self.scopes.len()
            )
        }
    }

    pub(super) fn receipt_overhead(&self) -> usize {
        if self.known.is_some() { 90 } else { 0 }
    }

    /// Reuse only a byte-identical evidence block for a byte-identical file in the same root.
    /// 仅复用同根、同文件字节、同证据块；局部片段回执不能证明持有全文件或新增调用者。
    pub(super) fn source_block(&self, file: &super::SourceFile, block: String) -> String {
        let Some(known) = &self.known else {
            return block;
        };
        let identity = serde_json::json!([
            "xirang-context-v1",
            self.root,
            file.relative,
            xirang_kernel::sha256_hex(file.source.as_bytes()),
            block
        ])
        .to_string();
        let receipt = xirang_kernel::sha256_hex(identity.as_bytes());
        let reference = format!(
            "\nfile {}\ncontext unchanged: {receipt}; caller retained this exact source/relations; omit known_context to resend\n",
            file.relative
        );
        if known.contains(&receipt) && reference.len() + receipt.len() + 3 < block.len() {
            reference
        } else {
            format!("{block}context receipt: {receipt}\n")
        }
    }
}

fn strings(arguments: &Value, key: &str, max: usize) -> Result<Option<Vec<String>>, String> {
    let Some(value) = arguments.get(key) else {
        return Ok(None);
    };
    let values = value
        .as_array()
        .filter(|values| values.len() <= max)
        .ok_or_else(|| format!("{key} must be an array with at most {max} strings"))?;
    values
        .iter()
        .map(|value| {
            value
                .as_str()
                .filter(|value| !value.is_empty() && value.len() <= 1024)
                .map(str::to_owned)
                .ok_or_else(|| format!("{key} entries must contain 1..1024 UTF-8 bytes"))
        })
        .collect::<Result<Vec<_>, _>>()
        .map(Some)
}
