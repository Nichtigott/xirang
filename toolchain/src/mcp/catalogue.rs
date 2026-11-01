//! On-demand tool documentation, distinct from project execution.
//! 按需提供工具文档，与项目执行明确区分。

use super::tools;
use serde_json::Value;
use std::path::Path;

/// The catalogue tool: every tool on one line, or one tool's whole page.
/// 目录工具：所有工具一行式列出，或某一个工具的整页。
///
/// It reads the **same** catalogue and prints the **same** one-liner `--list` does
/// (`client::list_tool_lines`) and the **same** page `--list <tool>` does (`client::describe_tool`),
/// so the advertisement and the manual cannot drift: there is one implementation of each.
/// 它读的是**同一份**目录、印的是 `--list` 印的那张**同一份**一行式清单（`client::list_tool_lines`）与
/// `--list <tool>` 印的**同一页**（`client::describe_tool`），因此广告与手册不可能漂移：各自只有一份实现。
pub(super) fn describe(_root: &Path, arguments: &Value) -> Result<String, String> {
    let flag = |key: &str| arguments.get(key).and_then(Value::as_bool) == Some(true);
    let Some(name) = arguments.get("tool").and_then(Value::as_str) else {
        return Ok(if flag("slim") {
            crate::mcp::client::slim_tools_page()
        } else {
            crate::mcp::client::list_tool_lines().join("\n") + "\n"
        });
    };
    // `full: true` prints the whole description (audit `W2-6`); the default page is bounded and says
    // so, so the manual is requested rather than paid for by every session.
    // `full: true` 印完整描述（审计 `W2-6`）；默认页有界并且自己说出来，因此手册是被**索取**的，而不是
    // 每个会话都为它付费。
    let page = if flag("full") {
        crate::mcp::client::describe_tool_fully(name)
    } else {
        crate::mcp::client::describe_tool(name)
    };
    let page = page.ok_or_else(|| {
        format!(
            "`{name}` is not a tool this bridge has; call `xirang_tools` with no `tool` to list \
             them, or `--list` for the same list (plus the workflow table)"
        )
    })?;
    let name = crate::mcp::client::resolve_name(name);
    let schema = if flag("full") {
        tools()
            .into_iter()
            .find(|entry| entry["name"] == name)
            .expect("the manual names a catalogue tool")["inputSchema"]
            .clone()
    } else {
        advertised_schema(&name)
    };
    let execution = format!(
        "execute via xirang_tools: tool={name}, arguments=<object matching inputSchema>; include arguments even when empty"
    );
    Ok(format!("{page}\ninputSchema {schema}\n{execution}\n"))
}

#[cfg(test)]
#[path = "catalogue_tests.rs"]
mod catalogue_tests;

/// 一个广告工具的瘦 schema：决定这次调用的那几个键，每个一行。
///
/// Audit `W1-4`: the apply parameter block alone was 2,140 characters of prose repeated in every
/// frame. What moved out is not deleted — `--list <tool>` and `xirang_tools` still carry it — and
/// the refusals carry the *instantiated* form, which is the one a caller can actually run.
/// 审计 `W1-4`：仅 apply 的参数块就是 2,140 字符的散文，每一帧都重发一遍。搬走的不是删掉的——
/// `--list <tool>` 与 `xirang_tools` 仍然带着它——而拒绝文案带的是**实例化**的那一份，也就是调用方
/// 真能跑的那一份。
pub(super) fn advertised_schema(name: &str) -> Value {
    let mut schema = tools()
        .into_iter()
        .find(|entry| entry["name"] == name)
        .expect("advertised tool belongs to the catalogue")["inputSchema"]
        .clone();
    compact_schema(&mut schema);
    schema
}

// Keep validation identical; only long explanatory prose belongs in the manual.
// 校验规则保持一致；只有较长的说明文字留在手册里。
pub(super) fn compact_schema(value: &mut Value) {
    match value {
        Value::Object(map) => {
            if map
                .get("description")
                .and_then(Value::as_str)
                .is_some_and(|text| text.len() > 110)
            {
                map.remove("description");
            }
            for value in map.values_mut() {
                compact_schema(value);
            }
        }
        Value::Array(items) => {
            for value in items {
                compact_schema(value);
            }
        }
        _ => {}
    }
}
