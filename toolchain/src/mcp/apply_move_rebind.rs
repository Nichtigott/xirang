//! Typed graft endpoints rewritten from the source form the host actually wrote.
//! 按宿主实际写下的源码形式改写类型化 graft 端点。
//!
//! The parser normalizes both accepted spellings — the declaration `crate::<module>::NODE_ID` and
//! the catalog `crate::catalog::<parent>::<KIND>` — to one declaration form. A rebind therefore
//! cannot find the author's text by that normalized string: it must use the **span** the parser
//! reported, classify the form from those bytes, and write the new identity back in that same form.
//! After editing it re-parses the file and checks the endpoints resolve to the expected identities,
//! so a rewrite that produced a path the build cannot resolve refuses before anything is written.
//! 解析器把两种可接受拼写——声明形 `crate::<模块>::NODE_ID` 与目录形
//! `crate::catalog::<父>::<KIND>`——归一成同一种声明形式。因此重绑不能按那条归一化字符串去找作者的
//! 文字：必须用解析器报告的**跨度**，从那几个字节判断形式，再把新身份按同一形式写回。改完重新解析
//! 文件并核对端点解析到预期身份，因此写出构建解析不了的路径会在落盘前被拒绝。

use super::endpoint::normalized;
use std::path::Path;

use xirang_kernel::syntax::{GraftEndpointSpan, GraftSpans, SyntaxLocation};

use crate::build_method::{DeclaredGraft, FaceView};

use super::Moved;
use super::grafts::DeclarationEdit;

/// The source form one endpoint was written in.
/// 一个端点写成的源码形式。
enum Form {
    /// `crate::<module>::NODE_ID`.
    /// `crate::<模块>::NODE_ID`。
    Declaration,
    /// `crate::catalog::<parent>::<KIND>`.
    /// `crate::catalog::<父>::<KIND>`。
    Catalog,
}

/// Plan the rewrites of typed endpoints that name a face moving with this operation.
/// 为点名了本次被搬面的类型化端点规划改写。
pub(super) fn plan_typed_edits(
    entry: &Path,
    text: &str,
    cuts: &[DeclaredGraft],
    affected: &[&FaceView],
    face: &FaceView,
    moved: &Moved,
    layout: &crate::source_layout::SourceLayout,
) -> Result<Vec<DeclarationEdit>, String> {
    let mut edits = Vec::new();
    for cut in cuts {
        let Some(expressions) = &cut.expressions else {
            continue;
        };
        let matched_cut = affected.iter().find(|view| names(view, &expressions.cut));
        let matched_end = expressions
            .cut_end
            .as_deref()
            .and_then(|end| affected.iter().find(|view| names(view, end)));
        let matched_graft = affected.iter().find(|view| names(view, &expressions.graft));
        if matched_cut.is_none() && matched_end.is_none() && matched_graft.is_none() {
            continue;
        }
        // A range names two endpoints; changing one of them is a decision about which side should
        // follow, so it refuses before the write instead of picking a side.
        // 区间点名两个端点；只改其中一个等于替作者决定哪一侧跟随，因此它在写入前拒绝，而不是选一侧。
        if expressions.cut_end.is_some() {
            return Err(format!(
                "{}:{}: this graft range names a face this move relocates; a range's two ends are \
                 a decision about which one follows, so rebind it by hand. Nothing changed.",
                entry.display(),
                cut.line
            ));
        }
        let Some(spans) = spans_for(text, cut) else {
            // The parser could not report a span: rewriting by line text would address the
            // normalized form, not the author's — refuse rather than guess.
            // 解析器没有报告跨度：按行文本改写只会点到归一化形式而不是作者的原文——拒绝而不是猜。
            return Err(format!(
                "{}:{}: this typed graft entry has no source span; rebind it by hand. Nothing changed.",
                entry.display(),
                cut.line
            ));
        };
        for (endpoint, span, target) in [
            (Some(&expressions.cut), Some(&spans.cut), matched_cut),
            (
                expressions.cut_end.as_ref(),
                spans.cut_end.as_ref(),
                matched_end,
            ),
            (Some(&expressions.graft), Some(&spans.graft), matched_graft),
        ] {
            let (Some(endpoint), Some(span), Some(target)) = (endpoint, span, target) else {
                continue;
            };
            let (start, end) = endpoint_bytes(text, span, entry, cut.line)?;
            let written = text.get(start..end).ok_or_else(|| {
                format!(
                    "{}:{}: the span is outside the file",
                    entry.display(),
                    cut.line
                )
            })?;
            let form = classify(written, entry, cut.line)?;
            let package = new_package(target, face, moved, layout);
            let module = crate::run_method::authoring::operations::parents::module_chain_of(
                layout, &package,
            )?
            .ok_or_else(|| {
                format!(
                    "{}:{}: cannot resolve the module chain of the new location `{package}`; rebind \
                     this entry by hand. Nothing changed.",
                    entry.display(),
                    cut.line
                )
            })?;
            let after = match form {
                Form::Declaration => format!("crate::{module}::NODE_ID"),
                Form::Catalog => catalog_reference(&module),
            };
            if after == written {
                continue;
            }
            edits.push(DeclarationEdit {
                file: entry.to_path_buf(),
                line: cut.line,
                before: written.to_owned(),
                after,
                span: Some((start, end)),
                verify: Some(format!("{module}::NODE_ID")),
                verify_absent: Some(normalized(endpoint)),
            });
        }
    }
    Ok(edits)
}

/// The catalog spelling of a face mounted at `module`.
/// 挂在 `module` 的面的目录形拼写。
fn catalog_reference(module: &str) -> String {
    let (parent, leaf) = module
        .rsplit_once("::")
        .map_or(("", module), |(parent, leaf)| (parent, leaf));
    let leaf = leaf.to_uppercase();
    if parent.is_empty() {
        format!("crate::catalog::{leaf}")
    } else {
        format!("crate::catalog::{parent}::{leaf}")
    }
}

/// The package-relative source a face occupies after this move.
/// 一次搬动之后一个面所在的包内相对源码。
fn new_package(
    view: &FaceView,
    face: &FaceView,
    moved: &Moved,
    layout: &crate::source_layout::SourceLayout,
) -> String {
    if view.id == face.id {
        return moved.to_physical.clone();
    }
    let old = xirang_multilang::source_ref::package_relative(
        view.source_base,
        &layout.package_root,
        &layout.identity_base,
        &view.source,
    );
    let directory = super::destination::directory_of(&old);
    let prefix = format!("{}/", moved.from_package);
    let Some(rest) = directory.strip_prefix(&prefix) else {
        return old;
    };
    let name = Path::new(&old)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();
    format!("{}/{rest}/{name}", moved.to_package)
}

/// The parser's span for one declaration, or `None` when it reported none.
/// 解析器为一条声明报告的跨度；没有报告时为 `None`。
fn spans_for(text: &str, cut: &DeclaredGraft) -> Option<GraftSpans> {
    let entries = xirang_kernel::syntax::graft_entries(text).ok()?;
    entries
        .into_iter()
        .find(|entry| entry.location.line == cut.line && entry.expressions.is_some())
        .and_then(|entry| entry.spans)
}

/// Classify the written endpoint from its own bytes.
/// 从端点自己的字节判断它的形式。
fn classify(written: &str, entry: &Path, line: usize) -> Result<Form, String> {
    let trimmed = written.trim();
    let trimmed = trimmed
        .strip_prefix('(')
        .and_then(|rest| rest.strip_suffix(')'))
        .map(str::trim)
        .unwrap_or(trimmed);
    if trimmed.starts_with("crate::catalog::") {
        return Ok(Form::Catalog);
    }
    if trimmed.starts_with("crate::") && trimmed.ends_with("::NODE_ID") {
        return Ok(Form::Declaration);
    }
    Err(format!(
        "{}:{line}: this typed graft endpoint is spelled `{trimmed}`, a form this rebind does not \
         know how to rewrite; change it by hand. Nothing changed.",
        entry.display()
    ))
}

/// The bytes of the endpoint inside its reported span, skipping comments and whitespace.
/// 报告跨度内端点本身的字节，跳过注释与空白。
fn endpoint_bytes(
    text: &str,
    span: &GraftEndpointSpan,
    entry: &Path,
    line: usize,
) -> Result<(usize, usize), String> {
    let (start, end) = byte_range(text, span, entry, line)?;
    let slice = &text[start..end];
    let mut begin = 0;
    loop {
        let rest = &slice[begin..];
        let trimmed = rest.trim_start();
        begin += rest.len() - trimmed.len();
        if rest.trim_start().starts_with("//") {
            let end_of_line = slice[begin..]
                .find('\n')
                .map_or(slice.len(), |index| begin + index + 1);
            begin = end_of_line;
            continue;
        }
        if let Some(rest) = slice[begin..].strip_prefix("/*")
            && let Some(close) = rest.find("*/")
        {
            begin += 2 + close + 2;
            continue;
        }
        break;
    }
    let rest = &slice[begin..];
    let mut finish = begin
        + rest
            .find(|character: char| character.is_whitespace() || character == '/')
            .unwrap_or(rest.len());
    // A trailing `)` still belongs to the grammar's parentheses, not to the endpoint.
    while finish > begin && matches!(slice.as_bytes()[finish - 1], b')' | b',') {
        finish -= 1;
    }
    if begin >= finish {
        return Err(format!(
            "{}:{line}: the endpoint's span holds no expression",
            entry.display()
        ));
    }
    Ok((start + begin, start + finish))
}

/// The byte range of one reported endpoint.
/// 一个已报告端点的字节范围。
fn byte_range(
    text: &str,
    span: &GraftEndpointSpan,
    entry: &Path,
    line: usize,
) -> Result<(usize, usize), String> {
    let start = offset(text, &span.start).ok_or_else(|| {
        format!(
            "{}:{line}: the endpoint's start is outside the file",
            entry.display()
        )
    })?;
    let end = offset(text, &span.end).ok_or_else(|| {
        format!(
            "{}:{line}: the endpoint's end is outside the file",
            entry.display()
        )
    })?;
    if start >= end {
        return Err(format!(
            "{}:{line}: the endpoint's span is empty",
            entry.display()
        ));
    }
    // The macro grammar reports a parenthesized group (`( … )`); address the expression inside and
    // keep the delimiters where the author wrote them.
    // 宏语法报告的是括号组（`( … )`）；定位内部表达式，并让分隔符留在作者写的位置。
    let slice = &text[start..end];
    let trimmed = slice.trim();
    if trimmed.starts_with('(') && trimmed.ends_with(')') {
        let inner_start = start + slice.find('(').expect("a found paren has a position") + 1;
        let inner_end = end - 1;
        return Ok((inner_start, inner_end));
    }
    Ok((start, end))
}

/// The byte offset of a one-based character position.
/// 1 起始字符位置的字节偏移。
fn offset(text: &str, location: &SyntaxLocation) -> Option<usize> {
    let mut line_start = 0;
    for (index, line) in text.split('\n').enumerate() {
        if index + 1 == location.line {
            let mut column = 1;
            for (byte, character) in line.char_indices() {
                if column == location.column {
                    return Some(line_start + byte);
                }
                column += 1;
                let _ = character;
            }
            return (column == location.column).then_some(line_start + line.len());
        }
        line_start += line.len() + 1;
    }
    None
}

/// Whether one endpoint names this face, in either accepted spelling.
/// 一个端点是否点名这个面——两种可接受拼写都算。
fn names(view: &FaceView, endpoint: &str) -> bool {
    let wanted = format!("{}::NODE_ID", view.module);
    normalized(endpoint) == wanted
}
