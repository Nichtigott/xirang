//! Static extraction of `soil.face({…})` declarations from a package's JavaScript sources.
//! 从包的 JavaScript 源码里静态提取 `soil.face({…})` 声明。
//!
//! This is the build-time reader, not the authoring adapter: it walks the package once, reads every
//! declaration with the same parser the adapter uses, and returns the facts the tree needs — source,
//! name, parent module and the fields that describe the face. It does not load Node and it does not
//! write anything; wiring these facts into the generated tree is a separate step.
//! 这是构建期读取器，不是创作适配器：它遍历包一次，用适配器同一个解析器读每条声明，返回树需要的事实
//! ——源码、名字、父模块与描述该面的字段。它不加载 Node、不写任何东西；把这些事实接进生成树是另一步。
//!
//! The walk never leaves the package root and only accepts one parent spelling family, so an
//! extraction cannot quietly pick up a file (or a parent) from outside the tree it was given.
//! 遍历不离开包根，且只接受一族父拼写，因此一次提取不会悄悄读到给定树之外的文件（或父）。

use std::path::Path;

use xirang_kernel::authoring::{FieldUpdate, RegistrationField};

use crate::languages::kit::grammar::split_commas;
use crate::source_scope::SourceScope;

/// One `soil.face({…})` declaration, as the build reads it.
/// 构建读到的一条 `soil.face({…})` 声明。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JsFace {
    /// The package-relative source path, part of the face's identity.
    /// 包内相对源码路径，面身份的一部分。
    pub source: String,
    /// The declared source this file came from: its id, or `workspace` for the package itself.
    /// 这个文件来自哪个已声明来源：它的 id，包自身则是 `workspace`。
    pub origin: String,
    /// The display name from `named:`.
    /// `named:` 里的显示名。
    pub name: String,
    /// The parent module, dotted-to-`::` (`faces.shop.panel` → `shop::panel`).
    /// 父模块，点号转 `::`（`faces.shop.panel` → `shop::panel`）。
    pub parent: String,
    /// Whether the face owns a registry of its own.
    /// 该面是否自持注册机。
    pub owns_registry: bool,
    /// The exports the declaration names, in the normalized order.
    /// 声明点名的导出，按规范化后的顺序。
    pub exports: Vec<String>,
}

/// Directories the walk never enters: build products, caches and other packages.
/// 遍历不进入的目录：构建产物、缓存与其它包。
pub(crate) const SKIPPED: &[&str] = &[
    "target",
    ".git",
    ".xirang",
    "node_modules",
    "coverage",
    "crates",
];

/// What the shared declaration walk needs to know about JavaScript.
/// 共享声明遍历需要知道的 JavaScript 那部分。
const DECLARATIONS: crate::languages::kit::declarations::DeclarationSpec =
    crate::languages::kit::declarations::DeclarationSpec {
        // `.mjs` and `.cjs` both end in `.js`, so one suffix covers the three module spellings.
        // `.mjs` 与 `.cjs` 都以 `.js` 结尾，因此一个后缀覆盖三种模块拼法。
        extensions: &[".js"],
        skip_directories: SKIPPED,
        marker: mentions,
        // A fact reader must not report "no declaration" for a file it never opened.
        // 事实读取器绝不能把自己从没打开过的文件报成"没有声明"。
        unreadable: crate::languages::kit::declarations::Unreadable::Refuse,
    };

/// Whether a text names the call this reader parses.
/// 一份文本是否点名本读取器解析的那次调用。
pub(crate) fn mentions(text: &str) -> bool {
    super::MARKERS.iter().any(|marker| text.contains(marker))
}

/// Every JavaScript declaration in the package, sorted by source.
/// 包里的每条 JavaScript 声明，按源码排序。
pub fn read_declarations(package_root: &Path) -> Result<Vec<JsFace>, String> {
    read_declarations_in(&SourceScope::with_sources(package_root)?)
}

/// Every JavaScript declaration under one already-decided source scope.
/// 在一个已经决定好的来源范围下的每条 JavaScript 声明。
pub fn read_declarations_in(scope: &SourceScope) -> Result<Vec<JsFace>, String> {
    read_declarations_for(scope, &DECLARATIONS)
}

/// Every declaration under one scope, for any carrier that shares this parser.
/// 在一个范围下的每条声明，供共用本解析器的任何载体使用。
pub(crate) fn read_declarations_for(
    scope: &SourceScope,
    spec: &crate::languages::kit::declarations::DeclarationSpec,
) -> Result<Vec<JsFace>, String> {
    // The declarations decide which files belong to a declared source; a file under none of them
    // is the package's own. Reading them once is a lookup, not a second enumeration.
    // 声明决定哪些文件属于某个已声明来源；不属于任何声明的文件是包自身的。读一次是查表，不是第二次枚举。
    let map = match scope.package_root() {
        Some(root) => crate::source_map::SourceMap::read_from(root)?,
        None => crate::source_map::SourceMap::empty(),
    };
    let mut faces = crate::languages::kit::declarations::read_in(scope, spec, |relative, text| {
        let declaration =
            super::read_declaration(text).map_err(|error| format!("{relative}: {error}"))?;
        let name = declaration.kind();
        if name.is_empty() {
            return Err(
                "the declaration has no `named:`, so the face has no display name".to_owned(),
            );
        }
        let parent = parent_module(&declaration.face.parent)?;
        Ok(vec![JsFace {
            origin: map.origin_of(relative).to_owned(),
            source: relative.to_owned(),
            name,
            parent,
            owns_registry: boolean(
                &declaration.face.registration,
                RegistrationField::NeedsRegistry,
            )?,
            exports: list(&declaration.face.registration, RegistrationField::Exports),
        }])
    })?;
    faces.sort_by(|left, right| left.source.cmp(&right.source));
    Ok(faces)
}

/// Turn `faces.shop.panel.NODE_ID` into `shop::panel`, and refuse any other spelling.
/// 把 `faces.shop.panel.NODE_ID` 变成 `shop::panel`，并拒绝其它拼写。
pub(super) fn parent_module(spelling: &str) -> Result<String, String> {
    let value = spelling.trim();
    let path = value
        .strip_suffix(".NODE_ID")
        .ok_or_else(|| format!("`{spelling}` does not end in `.NODE_ID`"))?;
    let path = path
        .strip_prefix("faces.")
        .or_else(|| path.strip_prefix("xirang.faces."))
        .ok_or_else(|| {
            format!(
                "`{spelling}` is not a generated `faces.…` reference; a JavaScript parent is named \
                 the way the SDK documents it (`faces.<module>.NODE_ID`)"
            )
        })?;
    let segments: Vec<&str> = path.split('.').collect();
    if segments.iter().any(|segment| !identifier(segment)) {
        return Err(format!(
            "`{spelling}` has a module segment that is not an identifier"
        ));
    }
    Ok(segments.join("::"))
}

/// Whether `text` is one JavaScript identifier.
/// `text` 是否是一个 JavaScript 标识符。
fn identifier(text: &str) -> bool {
    let mut characters = text.chars();
    match characters.next() {
        Some(first) if first == '_' || first == '$' || first.is_ascii_alphabetic() => {}
        _ => return false,
    }
    characters
        .all(|character| character == '_' || character == '$' || character.is_ascii_alphanumeric())
}

/// One boolean field, refusing text that is not a JavaScript boolean.
/// 一个布尔字段，拒绝不是 JavaScript 布尔的文本。
pub(super) fn boolean(
    registration: &xirang_kernel::authoring::RegistrationUpdate,
    field: RegistrationField,
) -> Result<bool, String> {
    match registration.find_field(field) {
        FieldUpdate::Keep | FieldUpdate::Clear => Ok(false),
        FieldUpdate::Set(value) => match value {
            "true" => Ok(true),
            "false" => Ok(false),
            other => Err(format!(
                "`{}` is `{other}`; the value reader only stores `true` or `false`",
                field.as_str()
            )),
        },
    }
}

/// One list field, split the way the request vocabulary stores it.
/// 一个列表字段，按请求词汇存储它的方式拆分。
pub(super) fn list(
    registration: &xirang_kernel::authoring::RegistrationUpdate,
    field: RegistrationField,
) -> Vec<String> {
    match registration.find_field(field) {
        FieldUpdate::Set(value) => value
            .split(',')
            .map(str::trim)
            .filter(|item| !item.is_empty())
            .map(str::to_owned)
            .collect(),
        FieldUpdate::Keep | FieldUpdate::Clear => Vec::new(),
    }
}

/// The byte offset of the `}` matching the `{` at `open`.
/// `open` 处 `{` 的配对 `}` 的字节偏移。
pub(super) fn matching_brace(source: &str, open: usize) -> Result<usize, String> {
    let bytes = source.as_bytes();
    let mut depth = 0usize;
    for (at, byte) in bytes.iter().enumerate().skip(open) {
        match byte {
            b'{' => depth += 1,
            b'}' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return Ok(at);
                }
            }
            _ => {}
        }
    }
    Err("a `soil.face({…})` object literal is not closed".to_owned())
}

/// The `key: value` pairs of one object body, comments removed.
/// 一个对象体里的 `key: value` 对，注释已去掉。
///
/// The splitter tracks brackets, parentheses and strings, so a value that is an array of strings
/// (the only nesting the carrier has) is not cut at its own commas. A trailing comma is allowed.
/// 分隔器跟踪方括号、圆括号与字符串，因此一个字符串数组取值（载体唯一的嵌套）不会在自己的逗号处被切开。
/// 尾逗号允许。
pub(super) fn pairs(body: &str) -> Result<Vec<(String, String)>, String> {
    let mut entries = Vec::new();
    let mut current = String::new();
    let mut depth = 0usize;
    let mut quote: Option<char> = None;
    let mut escaped = false;
    let mut index = 0usize;
    let bytes = body.as_bytes();
    while index < bytes.len() {
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
            '[' | '{' | '(' => {
                depth += 1;
                current.push(character);
            }
            ']' | '}' | ')' => {
                depth = depth.saturating_sub(1);
                current.push(character);
            }
            ',' if depth == 0 => {
                entries.push(current.trim().to_owned());
                current.clear();
            }
            '/' if body[index..].starts_with("//") => {
                // A line comment is prose, not a field: skip to the newline.
                // 行注释是散文而不是字段：跳到换行。
                let rest = &body[index..];
                index += rest.find('\n').unwrap_or(rest.len());
                continue;
            }
            '/' if body[index..].starts_with("/*") => {
                // A block comment is skipped whole; it never contributes a field.
                // 块注释整体跳过；它从不贡献字段。
                let rest = &body[index + 2..];
                let end = rest
                    .find("*/")
                    .ok_or_else(|| "a block comment in the declaration is not closed".to_owned())?;
                index += 2 + end + 2;
                continue;
            }
            _ => current.push(character),
        }
        index += width;
    }
    entries.push(current.trim().to_owned());
    let mut parsed = Vec::new();
    for entry in entries {
        if entry.is_empty() {
            continue;
        }
        let Some((key, value)) = split_pair(&entry) else {
            return Err(format!(
                "`{entry}` is not a `key: value` entry in a `soil.face({{…}})` declaration"
            ));
        };
        let key = key
            .trim()
            .trim_matches(|character| character == '"' || character == '\'');
        if key.is_empty() {
            return Err("a declaration entry has an empty key".to_owned());
        }
        parsed.push((key.to_owned(), value.trim().to_owned()));
    }
    Ok(parsed)
}

/// Split one entry at its first `:` that is outside a string and outside brackets.
/// 在一个条目的第一个不在字符串里、也不在括号里的 `:` 处切开。
fn split_pair(entry: &str) -> Option<(&str, &str)> {
    let mut depth = 0usize;
    let mut quote: Option<char> = None;
    let mut escaped = false;
    for (at, character) in entry.char_indices() {
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
            '[' | '{' | '(' => depth += 1,
            ']' | '}' | ')' => depth = depth.saturating_sub(1),
            ':' if depth == 0 => return Some((&entry[..at], &entry[at + 1..])),
            _ => {}
        }
    }
    None
}

/// One quoted string, refusing anything else.
/// 一个带引号的字符串，其它一律拒绝。
pub(super) fn string(raw: &str) -> Result<String, String> {
    let trimmed = raw.trim();
    let mut characters = trimmed.chars();
    let Some(quote @ ('"' | '\'')) = characters.next() else {
        return Err(format!("`{trimmed}` is not a string literal"));
    };
    if !trimmed.ends_with(quote) || trimmed.len() < 2 {
        return Err(format!("`{trimmed}` is not a closed string literal"));
    }
    let inner = &trimmed[quote.len_utf8()..trimmed.len() - quote.len_utf8()];
    Ok(unescape(inner))
}

/// One array of quoted strings, refusing anything else.
/// 一个带引号字符串的数组，其它一律拒绝。
pub(super) fn array(raw: &str) -> Result<Vec<String>, String> {
    let trimmed = raw.trim();
    let inner = trimmed
        .strip_prefix('[')
        .and_then(|rest| rest.strip_suffix(']'))
        .ok_or_else(|| format!("`{trimmed}` is not an array literal"))?;
    let mut entries = Vec::new();
    for entry in split_commas(inner) {
        if entry.trim().is_empty() {
            continue;
        }
        entries.push(string(entry)?);
    }
    Ok(entries)
}

/// Undo the escapes the renderer writes.
/// 还原渲染器写下的转义。
fn unescape(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut characters = text.chars();
    while let Some(character) = characters.next() {
        if character != '\\' {
            result.push(character);
            continue;
        }
        match characters.next() {
            Some('n') => result.push('\n'),
            Some('t') => result.push('\t'),
            Some('\\') => result.push('\\'),
            Some('"') => result.push('"'),
            Some('\'') => result.push('\''),
            Some(other) => {
                result.push('\\');
                result.push(other);
            }
            None => result.push('\\'),
        }
    }
    result
}
