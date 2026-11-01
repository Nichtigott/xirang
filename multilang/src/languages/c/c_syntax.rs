//! The C declaration scanner: brace matching, designated-initializer entries and strings.
//! C 声明扫描器：花括号配对、指定初始化器条目与字符串。
//!
//! It is the text half of the C adapter, kept apart from the language's field table and renderer so
//! each file stays inside the workspace's size budget. The scanner reads one initializer the
//! adapter's own renderer wrote (or a hand-written one of the same shape) and refuses what it cannot
//! read back.
//! 这是 C 适配器的文本那一半，与语言的字段表和渲染器分开，使每个文件都留在工作区的尺寸预算内。
//! 扫描器只读适配器自己的渲染器写出的一个初始化器（或同形状的手写初始化器），读不回的形态按名拒绝。

/// The byte offset of the `}` matching the `{` at `open`.
/// `open` 处 `{` 的配对 `}` 的字节偏移。
pub(super) fn matching_brace(source: &str, open: usize) -> Result<usize, String> {
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
            '{' => depth += 1,
            '}' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return Ok(at);
                }
            }
            _ => {}
        }
    }
    Err("a `XIRANG_SOIL` initializer is not closed".to_owned())
}

/// The entries of one initializer body: `.name = value` members and bare header macros.
/// 一个初始化器体里的条目：`.name = value` 成员与裸头宏。
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
        let Some(rest) = entry.strip_prefix('.') else {
            // A bare macro (`XIRANG_RECORD_HEADER`) is the fixed header, not a member.
            // 裸宏（`XIRANG_RECORD_HEADER`）是固定头，不是成员。
            parsed.push((String::new(), entry));
            continue;
        };
        let Some((key, value)) = rest.split_once('=') else {
            return Err(format!(
                "`{entry}` is not a `.name = value` member in a `XIRANG_SOIL` declaration"
            ));
        };
        let key = format!(".{}", key.trim());
        if key.len() <= 1 {
            return Err("a declaration member has an empty name".to_owned());
        }
        parsed.push((key, value.trim().to_owned()));
    }
    Ok(parsed)
}

/// One C string literal, refusing anything else.
/// 一个 C 字符串字面量，其它一律拒绝。
pub(super) fn string(raw: &str) -> Result<String, String> {
    let trimmed = raw.trim();
    let Some(inner) = trimmed
        .strip_prefix('"')
        .and_then(|rest| rest.strip_suffix('"'))
    else {
        return Err(format!("`{trimmed}` is not a C string literal"));
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
