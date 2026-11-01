//! The Java annotation scanner: parenthesis matching, `name = value` entries, strings and arrays.
//! Java 注解扫描器：括号配对、`name = value` 条目、字符串与数组。
//!
//! It is the text half of the Java adapter, kept apart from the language's field table and renderer
//! so each file stays inside the workspace's size budget. The scanner reads one annotation the
//! adapter's own renderer wrote (or a hand-written one of the same shape) and refuses what it cannot
//! read back.
//! 这是 Java 适配器的文本那一半，与语言的字段表和渲染器分开，使每个文件都留在工作区的尺寸预算内。
//! 扫描器只读适配器自己的渲染器写出的一个注解（或同形状的手写注解），读不回的形态按名拒绝。

use crate::languages::kit::grammar::{split_commas, split_entry};

/// The byte offset of the `)` matching the `(` at `open`.
/// `open` 处 `(` 的配对 `)` 的字节偏移。
pub(super) fn matching_paren(source: &str, open: usize) -> Result<usize, String> {
    let mut depth = 0usize;
    let mut quote: Option<char> = None;
    let mut escaped = false;
    for (at, character) in source.char_indices().skip_while(|(at, _)| *at < open) {
        if let Some(active) = quote {
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == active {
                quote = None;
            }
            continue;
        }
        match character {
            '"' | '\'' => quote = Some(character),
            '(' => depth += 1,
            ')' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return Ok(at);
                }
            }
            _ => {}
        }
    }
    Err("an `@Soil(…)` annotation is not closed".to_owned())
}

/// The `name = value` entries of one annotation argument list.
/// 一个注解参数表里的 `name = value` 条目。
pub(super) fn entries(body: &str) -> Result<Vec<(String, String)>, String> {
    let mut found = Vec::new();
    let mut current = String::new();
    let mut depth = 0usize;
    let mut quote: Option<char> = None;
    let mut escaped = false;
    let mut index = 0usize;
    while index < body.len() {
        let character = body[index..].chars().next().expect("a character");
        let width = character.len_utf8();
        if let Some(active) = quote {
            current.push(character);
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == active {
                quote = None;
            }
            index += width;
            continue;
        }
        match character {
            '"' | '\'' => {
                quote = Some(character);
                current.push(character);
            }
            '{' | '(' | '[' => {
                depth += 1;
                current.push(character);
            }
            '}' | ')' | ']' => {
                depth = depth.saturating_sub(1);
                current.push(character);
            }
            ',' if depth == 0 => {
                found.push(current.trim().to_owned());
                current.clear();
            }
            '/' if body[index..].starts_with("//") => {
                let rest = &body[index..];
                index += rest.find('\n').unwrap_or(rest.len());
                continue;
            }
            _ => current.push(character),
        }
        index += width;
    }
    found.push(current.trim().to_owned());
    let mut parsed = Vec::new();
    for entry in found {
        if entry.is_empty() {
            continue;
        }
        let Some((key, value)) = split_entry(&entry) else {
            return Err(format!(
                "`{entry}` is not a `name = value` element in an `@Soil(…)` declaration"
            ));
        };
        let key = key.trim();
        if key.is_empty() {
            return Err("a declaration element has an empty name".to_owned());
        }
        parsed.push((key.to_owned(), value.trim().to_owned()));
    }
    Ok(parsed)
}

/// One Java string literal, refusing anything else.
/// 一个 Java 字符串字面量，其它一律拒绝。
pub(super) fn string(raw: &str) -> Result<String, String> {
    let trimmed = raw.trim();
    let Some(inner) = trimmed
        .strip_prefix('"')
        .and_then(|rest| rest.strip_suffix('"'))
    else {
        return Err(format!("`{trimmed}` is not a Java string literal"));
    };
    let mut result = String::with_capacity(inner.len());
    let mut characters = inner.chars();
    while let Some(character) = characters.next() {
        if character != '\\' {
            result.push(character);
            continue;
        }
        match characters.next() {
            Some('n') => result.push('\n'),
            Some('t') => result.push('\t'),
            Some('"') => result.push('"'),
            Some('\\') => result.push('\\'),
            Some(other) => {
                result.push('\\');
                result.push(other);
            }
            None => result.push('\\'),
        }
    }
    Ok(result)
}

/// One `{"…", …}` annotation array literal.
/// 一个 `{"…", …}` 注解数组字面量。
pub(super) fn array(raw: &str) -> Result<Vec<String>, String> {
    let trimmed = raw.trim();
    let inner = trimmed
        .strip_prefix('{')
        .and_then(|rest| rest.strip_suffix('}'))
        .ok_or_else(|| format!("`{trimmed}` is not a `{{…}}` annotation array"))?;
    let mut values = Vec::new();
    for entry in split_commas(inner) {
        if entry.trim().is_empty() {
            continue;
        }
        values.push(string(entry)?);
    }
    Ok(values)
}
