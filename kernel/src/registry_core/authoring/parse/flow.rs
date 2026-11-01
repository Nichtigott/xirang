//! Flow-contract field rendering and parsing for authored faces.
//! 注册面 flow 合同字段的渲染与解析。
//!
//! The editor writes one compact `id|version|input|output` line, while a face
//! holds a real `FlowContract::new(…)` expression. This page owns both
//! directions of that translation.
//! 编辑器写下一行紧凑的 `id|version|input|output`，而注册面里放的是真实的
//! `FlowContract::new(…)` 表达式。本页拥有这一转换的两个方向。

use crate::registry_core::authoring::validation::rust_string;
use crate::registry_core::declaration::OwnedFlowContract;
use crate::registry_core::syntax::nesting::guard_nesting;

use super::FaceParseError;

/// One compact flow value split into its four fields.
/// 一个紧凑 flow 值切分成的四个字段。
///
/// `render_flow_expression` and `parse_flow_value` each used to trim, split on
/// `|`, count the fields, and parse the version on their own. A rule tightened
/// in one of them — the over-count check is the easiest to forget — would let
/// the editor write a value the reload path then refused, or the reverse. This
/// is the family's only split-and-validate; the renderer and the snapshot
/// parser both consume it. `flow_render_and_parse_read_one_parser` pins the two
/// entry points to the same text and the same four fields.
/// `render_flow_expression` 与 `parse_flow_value` 过去各自去空白、按 `|` 切分、数
/// 字段、解析版本号。只在一侧收紧规则（最容易漏掉的是字段过多检查）会让编辑器写下的
/// 值随后被热重载路径拒绝，反之亦然。这是该字段族唯一的切分与校验点；渲染器与快照
/// 解析器都消费它。`flow_render_and_parse_read_one_parser` 把两个入口钉到同一文本与
/// 同样四个字段。
struct CompactFlow<'a> {
    id: &'a str,
    version: u32,
    input: &'a str,
    output: &'a str,
}

/// Split a compact flow value, or `None` when it declares no contract.
/// 切分紧凑 flow 值；空值或 `none` 表示未声明合同时返回 `None`。
fn parse_compact_flow(value: &str) -> Result<Option<CompactFlow<'_>>, FaceParseError> {
    let value = value.trim();
    if value.is_empty() || value.eq_ignore_ascii_case("none") {
        return Ok(None);
    }
    let mut fields = value.split('|');
    let id = fields.next().unwrap_or_default().trim();
    let version = fields.next().unwrap_or_default().trim();
    let input = fields.next().unwrap_or_default().trim();
    let output = fields.next().unwrap_or_default().trim();
    if fields.next().is_some() || id.is_empty() || input.is_empty() || output.is_empty() {
        return Err("flow must use id|version|input|output syntax"
            .to_owned()
            .into());
    }
    let version = version
        .parse::<u32>()
        .map_err(|_| "flow version must be an unsigned integer".to_owned())?;
    Ok(Some(CompactFlow {
        id,
        version,
        input,
        output,
    }))
}

/// Render the editor's `id|version|input|output` form as a Rust expression.
/// 将编辑器中的 `id|version|input|output` 形式渲染为 Rust 表达式。
pub fn render_flow_expression(value: &str) -> Result<String, FaceParseError> {
    let expression = flow_contract_expression(value)?;
    if expression.is_empty() {
        return Ok(String::new());
    }
    Ok(format!("    flow: {expression},\n"))
}

/// Render the compact form as just the `FlowContract::new(…)` expression, which is what a field written as
/// `flow: Some(…)` needs. One implementation, two call sites: a field line and a wrapped value.
/// 把紧凑形式渲染成 `FlowContract::new(…)` 这一个表达式本身——写成 `flow: Some(…)` 的字段要的就是它。
/// 一份实现、两个调用点：整行字段与包在 `Some(…)` 里的值。
pub fn flow_contract_expression(value: &str) -> Result<String, FaceParseError> {
    let Some(flow) = parse_compact_flow(value)? else {
        return Ok(String::new());
    };
    Ok(format!(
        "crate::FlowContract::new(crate::ContractId::new(\"{}\"), {}, \"{}\", \"{}\")",
        rust_string(flow.id),
        flow.version,
        rust_string(flow.input),
        rust_string(flow.output),
    ))
}

/// Parse the compact flow value into an owned contract for a reload snapshot.
/// 将紧凑 flow 值解析为热重载快照使用的拥有型合同。
pub fn parse_flow_value(value: &str) -> Result<Option<OwnedFlowContract>, FaceParseError> {
    Ok(parse_compact_flow(value)?.map(|flow| OwnedFlowContract {
        id: flow.id.to_owned(),
        version: flow.version,
        input: flow.input.to_owned(),
        output: flow.output.to_owned(),
    }))
}

/// Render an explicitly selected flow provider type.
/// 渲染显式选择的数据流合同提供者类型。
pub fn render_flow_provider(value: &str) -> Result<String, FaceParseError> {
    let value = value.trim();
    if value.is_empty() {
        return Ok(String::new());
    }
    // The value arrives from a manifest or the editor, which makes it untrusted input
    // to a recursive-descent parser: without this guard a 542-byte type path aborts the
    // process with a stack overflow instead of returning an error. `guard_nesting` is
    // the kernel's one nesting measurement, and this module is why it is public rather
    // than private to the syntax module.
    // 这个值来自清单或编辑器，因此对递归下降解析器而言是不可信输入：没有这道守卫时，一条 542
    // 字节的类型路径会以栈溢出 abort 进程，而不是返回错误。`guard_nesting` 是内核唯一的嵌套
    // 度量，而本模块正是它公开而非限于 syntax 模块私有的原因。
    guard_nesting(value).map_err(|error| FaceParseError::new(error.to_string()))?;
    syn::parse_str::<syn::Path>(value)
        .map_err(|_| "flow_provider must be a Rust type path".to_owned())?;
    Ok(format!("    flow_provider: {value},\n"))
}

/// Read the `flow` field of a declaration, accepting **either** spelling it is written in: the editor's
/// compact `id|version|input|output` form, or a Rust expression. Both normalize to the compact form, which is
/// what the manifest side consumes.
/// 读声明里的 `flow` 字段，**两种**它可能被写成的拼法都收：编辑器用的紧凑 `id|version|input|output` 形式，
/// 或一条 Rust 表达式。两者都归一成紧凑形式，也就是清单那一侧消费的东西。
pub fn parse_flow_field(value: &str) -> Result<String, FaceParseError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(String::new());
    }
    // A `Registration` literal writes a Rust expression; a manifest or the editor writes the compact form.
    // Try the expression first, then fall back to the compact form — the same order as the `path`-then-
    // `string` reader next door, and for the same reason: one field, two spellings.
    // `Registration` 字面量写的是 Rust 表达式；清单或编辑器写的是紧凑形式。先试表达式、再退紧凑——与隔壁
    // `path` 再退 `string` 的读法同序，理由也一样：一个字段、两种拼法。
    if let Ok(compact) = parse_flow_expression(trimmed) {
        return Ok(compact);
    }
    if parse_compact_flow(trimmed)?.is_some() {
        return Ok(trimmed.to_owned());
    }
    Ok(String::new())
}

/// Parse a flow expression back into the editor's compact form.
/// 将 flow 表达式解析回编辑器使用的紧凑形式。
pub fn parse_flow_expression(value: &str) -> Result<String, FaceParseError> {
    let value = value.trim();
    if value.is_empty() || value.ends_with("FlowContract::NONE") {
        return Ok(String::new());
    }
    // Same untrusted value, same guard: a generated face's flow expression is read back
    // from a file that an editor or a previous version wrote.
    // 同一个不可信值，同一道守卫：生成面的 flow 表达式是从编辑器或旧版本写下的文件里读回来的。
    guard_nesting(value).map_err(|error| FaceParseError::new(error.to_string()))?;
    let expression = syn::parse_str::<syn::Expr>(value)
        .map_err(|_| "generated face has an invalid flow expression".to_owned())?;
    let syn::Expr::Call(call) = expression else {
        return Err("generated face has an invalid flow expression"
            .to_owned()
            .into());
    };
    if !path_ends_with(&call.func, "FlowContract::new") || call.args.len() != 4 {
        return Err("generated face has an invalid flow expression"
            .to_owned()
            .into());
    }
    let mut args = call.args.iter();
    let id_call = args
        .next()
        .ok_or_else(|| "generated face has an invalid flow id".to_owned())?;
    let syn::Expr::Call(id_call) = id_call else {
        return Err("generated face has an invalid flow id".to_owned().into());
    };
    if !path_ends_with(&id_call.func, "ContractId::new") || id_call.args.len() != 1 {
        return Err("generated face has an invalid flow id".to_owned().into());
    }
    let id = literal_string_expr(
        id_call
            .args
            .first()
            .ok_or_else(|| "generated face has an invalid flow id".to_owned())?,
    )?;
    let version = match args
        .next()
        .ok_or_else(|| "generated face has an invalid flow version".to_owned())?
    {
        syn::Expr::Lit(literal) => match &literal.lit {
            syn::Lit::Int(value) => value
                .base10_parse::<u32>()
                .map_err(|_| "generated face has an invalid flow version".to_owned())?,
            _ => {
                return Err("generated face has an invalid flow version"
                    .to_owned()
                    .into());
            }
        },
        _ => {
            return Err("generated face has an invalid flow version"
                .to_owned()
                .into());
        }
    };
    let input = literal_string_expr(
        args.next()
            .ok_or_else(|| "generated face has an invalid flow input".to_owned())?,
    )?;
    let output = literal_string_expr(
        args.next()
            .ok_or_else(|| "generated face has an invalid flow output".to_owned())?,
    )?;
    Ok(format!("{id}|{version}|{input}|{output}"))
}

/// Whether a Rust path expression ends with the given suffix.
/// Rust 路径表达式是否以给定后缀结尾。
pub fn path_ends_with(expression: &syn::Expr, suffix: &str) -> bool {
    let syn::Expr::Path(path) = expression else {
        return false;
    };
    let actual = path
        .path
        .segments
        .iter()
        .map(|segment| segment.ident.to_string())
        .collect::<Vec<_>>()
        .join("::");
    actual.ends_with(suffix)
}

/// Extract the string literal of an expression, rejecting anything else.
/// 取出表达式的字符串字面量，其余写法一律拒绝。
pub fn literal_string_expr(expression: &syn::Expr) -> Result<String, FaceParseError> {
    let syn::Expr::Lit(literal) = expression else {
        return Err("generated face expects a string literal".to_owned().into());
    };
    let syn::Lit::Str(value) = &literal.lit else {
        return Err("generated face expects a string literal".to_owned().into());
    };
    Ok(value.value())
}

#[cfg(test)]
mod tests {
    use super::{parse_flow_value, render_flow_expression};
    use crate::registry_core::declaration::OwnedFlowContract;

    /// Both entry points now read the one parser, so a malformed value must be
    /// refused with identical text and a well-formed one must describe the same
    /// four fields. Before the extraction each function split and validated the
    /// line on its own; a rule changed on one side would let the editor write a
    /// value the reload path rejected.
    /// 两个入口现在读同一个解析器，因此畸形值必须给出完全相同的拒绝文本，合法值必须
    /// 描述同样四个字段。抽取之前两者各自切分并校验该行；只改一侧的规则会让编辑器写下
    /// 的值被热重载路径拒绝。
    #[test]
    fn flow_render_and_parse_read_one_parser() {
        for malformed in ["id|1|in", "id|1|in|out|extra", "|1|in|out", "id|x|in|out"] {
            let rendered = render_flow_expression(malformed).unwrap_err();
            let parsed = parse_flow_value(malformed).unwrap_err();
            assert_eq!(rendered, parsed, "value `{malformed}`");
        }
        assert_eq!(
            render_flow_expression(" render.v1 | 2 | LocalCoordinates | CanvasFrame ").unwrap(),
            "    flow: crate::FlowContract::new(crate::ContractId::new(\"render.v1\"), 2, \"LocalCoordinates\", \"CanvasFrame\"),\n"
        );
        assert_eq!(
            parse_flow_value(" render.v1 | 2 | LocalCoordinates | CanvasFrame ").unwrap(),
            Some(OwnedFlowContract {
                id: "render.v1".to_owned(),
                version: 2,
                input: "LocalCoordinates".to_owned(),
                output: "CanvasFrame".to_owned(),
            })
        );
        assert!(render_flow_expression("none").unwrap().is_empty());
        assert!(render_flow_expression("  ").unwrap().is_empty());
        assert_eq!(parse_flow_value("NONE").unwrap(), None);
        assert_eq!(parse_flow_value("").unwrap(), None);
    }
}

#[cfg(test)]
#[path = "flow_tests.rs"]
mod flow_tests;

/// A manifest or the editor writes the compact `id|version|input|output` value into the same field a
/// `Registration` literal fills with a Rust expression, so the field reader has to accept both spellings.
/// 清单或编辑器会把紧凑的 `id|version|input|output` 写进那个字段，而 `Registration` 字面量往同一个字段里写的是
/// Rust 表达式——因此读那个字段的人必须两种拼法都收。
#[test]
fn a_compact_flow_field_reads_back_as_the_same_compact_value() {
    let compact = "panel.render.v1|1|PanelInput|PanelFrame";
    assert_eq!(
        parse_flow_field(compact).expect("the editor's own form is accepted"),
        compact,
        "the compact form comes back unchanged"
    );
    let expression = "crate::FlowContract::new(crate::ContractId::new(\"panel.render.v1\"), 1, \"PanelInput\", \"PanelFrame\")";
    assert_eq!(
        parse_flow_field(expression).expect("a literal face's expression is accepted"),
        compact,
        "the Rust expression normalizes to the same compact value"
    );
    assert_eq!(parse_flow_field("").expect("empty is legal"), "");
}
