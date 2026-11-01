//! Whole-region replacement for a registration face.
//! 注册面的整段替换。
//!
//! The two spellings differ in where a face's generated items live: the macro spelling keeps all of them
//! inside one item, while the literal spelling splits them into a baked identity constant, the declaration,
//! and the type-class half (`impl FaceContract`, `assert_contracts!`) that sits outside it. Rewriting a face
//! therefore means owning that whole region — not splicing one expression.
//! 两种拼法的差别在于"面的生成条目住在哪"：宏拼法把它们全放在同一条目里，而字面量拼法把它们拆成烘焙的身份
//! 常量、那份声明，以及住在声明之外的类型类那一半（`impl FaceContract`、`assert_contracts!`）。因此重写一个面
//! 意味着拥有**整段区域**——而不是拼接一个表达式。

use super::face::{FaceSyntax, FaceSyntaxError, parse_face, replace_face_macro, source_offset};

/// Replace a whole registration face: everything it generates — the baked identity constant, the declaration
/// **and** (in the literal spelling) the type-class items that live outside it (`impl FaceContract` and the
/// `assert_contracts!` line).
/// 替换**整个**注册面：它生成的一切——烘焙的身份常量、那份声明**以及**（字面量拼法下）住在声明之外的类型类
/// 条目（`impl FaceContract` 与 `assert_contracts!` 那一行）。
///
/// Splitting them apart is not an option: rustc rejects a second `impl FaceContract` for one type with E0119,
/// and a `preset`/`parts` edit that rewrote only the declaration would leave the old associated types in
/// place. The macro spelling keeps all of it inside one item, so there it is a straight splice.
/// 把它们分开处理不是选项：同一类型两个 `impl FaceContract` 会被 rustc 以 E0119 拒绝，而只改写声明的
/// `preset`/`parts` 编辑会把旧关联类型留在原处。宏拼法把这一切都放在同一条条目里，因此那边就是一次直接拼接。
pub fn replace_face_region(source: &str, replacement: &str) -> Result<String, FaceSyntaxError> {
    let current = parse_face(source)?.ok_or_else(|| FaceSyntaxError {
        message: "source has no registration face".to_owned(),
        location: None,
    })?;
    if !current.is_struct_literal() {
        return replace_face_macro(source, replacement);
    }
    // Both sides are read the same way: a face file's **generated block** runs from its first generated item
    // to its last, and everything between them (the declaration included) belongs to it. The author's own
    // items — module docs, imports, the marker type — are outside and are never touched.
    // 两侧用同一种读法：面文件的**生成块**从它第一件生成条目到最后一件事，中间的一切（含声明）都属于它。
    // 作者自己的条目——模块文档、导入、标记类型——在块外，绝不被动。
    let Some((source_start, source_end)) = generated_block(source, &current) else {
        return replace_face_macro(source, replacement);
    };
    let next = parse_face(replacement)?.ok_or_else(|| FaceSyntaxError {
        message: "replacement has no registration face".to_owned(),
        location: None,
    })?;
    let Some((next_start, next_end)) = generated_block(replacement, &next) else {
        return replace_face_macro(source, replacement);
    };
    let mut output = String::with_capacity(source.len() + (next_end - next_start));
    output.push_str(&source[..source_start]);
    output.push_str(replacement[next_start..next_end].trim_end());
    output.push('\n');
    output.push_str(source[source_end..].trim_start_matches('\n'));
    Ok(output)
}

/// The span of a face's generated items: from the first to the last, declaration included.
/// 一个面生成条目的区间：从第一件到最后一件事，含声明。
fn generated_block(source: &str, face: &FaceSyntax) -> Option<(usize, usize)> {
    let (_, spans) = strip_generated_face_items(source, face);
    let mut edges: Vec<(usize, usize)> = spans;
    let declaration = source_offset(source, &face.declaration_start).ok()?;
    let end = source_offset(source, &face.end).ok()?;
    edges.push((declaration, end));
    let start = edges.iter().map(|&(start, _)| start).min()?;
    let end = edges.iter().map(|&(_, end)| end).max()?;
    Some((start, end))
}

/// Remove the generated items that are **not** the declaration, from the end of the file backwards.
/// 移除**不是**声明的那几件生成条目，从文件末尾往前处理。
///
/// Each is found by its header and closed by brace/paren/semicolon matching. The declaration itself is left
/// where it is: its span is what the caller splices over.
/// 每一件按头部找到、按花括号/圆括号/分号配对收尾。声明本身留在原处：调用方正是要在它的位置上拼接。
pub(super) fn strip_generated_face_items(
    source: &str,
    face: &FaceSyntax,
) -> (String, Vec<(usize, usize)>) {
    let mut spans: Vec<(usize, usize)> = Vec::new();
    for (needle, closing) in [
        ("pub const NODE_ID", ';'),
        ("impl FaceContract for", '}'),
        ("assert_contracts!", ')'),
    ] {
        let mut from = 0;
        while let Some(at) = source[from..].find(needle).map(|offset| from + offset) {
            let start = item_start(source, at);
            let body = match closing {
                '}' | ')' => source[at..]
                    .find(if closing == '}' { '{' } else { '(' })
                    .map(|offset| at + offset),
                _ => None,
            };
            let end = match closing {
                ';' => source[at..].find(';').map(|offset| at + offset + 1),
                _ => body.and_then(|open| {
                    let mut depth = 0i32;
                    for (offset, character) in source[open..].char_indices() {
                        if character == if closing == '}' { '{' } else { '(' } {
                            depth += 1;
                        } else if character == closing {
                            depth -= 1;
                            if depth == 0 {
                                return Some(open + offset + character.len_utf8());
                            }
                        }
                    }
                    None
                }),
            };
            let Some(end) = end else {
                from = at + needle.len();
                continue;
            };
            let end = if closing == ')' {
                source[end..]
                    .find(';')
                    .map_or(end, |offset| end + offset + 1)
            } else {
                end
            };
            let tail = skip_blank_lines(source, end);
            // the declaration is excluded by the caller; every other match is removed
            if !source[..start].trim_end().ends_with("pub struct")
                && !is_declaration_start(source, at, face)
            {
                spans.push((start, tail));
            }
            from = tail.max(at + needle.len());
        }
    }
    spans.sort_unstable();
    spans.dedup();
    let mut out = source.to_owned();
    for &(start, end) in spans.iter().rev() {
        out.replace_range(start..end, "");
    }
    (out, spans)
}

/// The first byte of the item (or of its attached doc/attribute block) that carries `at`.
/// 承载 `at` 的那个条目（或它附带的文档/属性块）的第一个字节。
pub(super) fn item_start(source: &str, at: usize) -> usize {
    let mut start = source[..at].rfind('\n').map_or(0, |line| line + 1);
    while start > 0 {
        let previous_line_end = start - 1;
        let previous_line_start = source[..previous_line_end]
            .rfind('\n')
            .map_or(0, |line| line + 1);
        let previous = source[previous_line_start..previous_line_end].trim_start();
        if previous.starts_with("///") || previous.starts_with("//!") || previous.starts_with("#[")
        {
            start = previous_line_start;
        } else {
            break;
        }
    }
    start
}

/// Whether the item starting at `start` is the face's declaration (not a generated item to strip).
/// 从 `start` 开始的条目是不是这个面的**声明**（而不是要摘掉的生成条目）。
pub(super) fn is_declaration_start(source: &str, at: usize, face: &FaceSyntax) -> bool {
    let Some(declaration) = source_offset(source, &face.declaration_start).ok() else {
        return false;
    };
    let Some(end) = source_offset(source, &face.end).ok() else {
        return false;
    };
    at >= declaration && at < end
}

/// Step past the blank lines that follow an item.
/// 跨过条目之后的空行。
pub(super) fn skip_blank_lines(source: &str, from: usize) -> usize {
    let mut at = from;
    while source[at..].starts_with('\n') {
        at += 1;
    }
    at
}

/// The first byte of a field's **value**: the parser records the field's own start, so skip the name and the
/// `:` that opens the value.
/// 字段**值**的第一个字节：解析器记的是字段自己的起点，因此要跳过名字与打开值的 `:`。
pub(crate) fn value_start(source: &str, at: usize) -> usize {
    let rest = &source[at..];
    match rest.find(':') {
        Some(colon) => {
            let after = &rest[colon + 1..];
            at + colon + 1 + (after.len() - after.trim_start().len())
        }
        None => at,
    }
}

/// The exclusive end of the expression that starts at `start`: scanning forward, a `,` or the field list's
/// own closing delimiter at depth zero ends it. Text-based on purpose — the alternative is a synthesized span.
/// 从 `start` 开始的表达式的排他终点：向前扫描，深度为零处的 `,` 或字段列表自身的收尾分隔符即其终点。有意
/// 按文本处理——另一条路是合成的 span。
pub(crate) fn expression_end(source: &str, start: usize) -> usize {
    let mut depth = 0i32;
    let mut end = start;
    for character in source[start..].chars() {
        match character {
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => {
                if depth == 0 {
                    break;
                }
                depth -= 1;
            }
            ',' if depth == 0 => break,
            _ => {}
        }
        end += character.len_utf8();
    }
    // Comments and whitespace around the expression belong to the reader, not to the expression: a rewrite
    // that swallowed `/* before */ … /* after */` deleted an author's comments (measured by
    // `relocation_changes_only_the_declaration_tokens_with_unicode_and_comments`).
    // 表达式周围的注释与空白属于读者、不属于表达式：把 `/* before */ … /* after */` 一起吞掉的改写会删掉作者
    // 的注释（`relocation_changes_only_the_declaration_tokens_with_unicode_and_comments` 实测到了这一点）。
    let mut start = start;
    loop {
        let text = &source[start..end];
        let trimmed = text.trim_start();
        start += text.len() - trimmed.len();
        let rest = &source[start..end];
        if let Some(after) = rest.strip_prefix("//") {
            start += after.find('\n').map_or(rest.len(), |newline| newline + 3);
        } else if let Some(after) = rest.strip_prefix("/*") {
            start += after.find("*/").map_or(rest.len(), |close| close + 4);
        } else {
            break;
        }
    }
    loop {
        let text = &source[start..end];
        let trimmed = text.trim_end();
        let removed = text.len() - trimmed.len();
        if removed > 0 {
            end -= removed;
            continue;
        }
        let rest = &source[start..end];
        if let Some(before) = rest.strip_suffix("*/") {
            end -= before
                .rfind("/*")
                .map_or(rest.len(), |open| rest.len() - open);
        } else if rest
            .lines()
            .last()
            .is_some_and(|line| line.trim_start().starts_with("//"))
        {
            end -= rest.lines().last().map_or(0, str::len);
        } else {
            break;
        }
    }
    end
}
