//! Static extraction of `@soil.face(...)` declarations from a package's Python sources.
//! 从包的 Python 源码里静态提取 `@soil.face(...)` 声明。
//!
//! This is the build-time reader, not the authoring adapter: it walks the package once, reads every
//! declaration with the same parser the adapter uses, and returns the facts the tree needs — source,
//! display name, parent module and the fields that describe the face. It does not load Python and it
//! does not write anything; wiring these facts into the generated tree is a separate step.
//! 这是构建期读取器，不是创作适配器：它遍历包一次，用适配器同一个解析器读每条声明，返回树需要的事实
//! ——源码、显示名、父模块与描述该面的字段。它不加载 Python、不写任何东西；把这些事实接进生成树是另一步。
//!
//! The walk never leaves the package root and only accepts one parent spelling family, so an
//! extraction cannot quietly pick up a file (or a parent) from outside the tree it was given.
//! 遍历不离开包根，且只接受一族父拼写，因此一次提取不会悄悄读到给定树之外的文件（或父）。

use std::path::Path;

use xirang_kernel::authoring::{FieldUpdate, RegistrationField};

use crate::source_scope::SourceScope;

/// One `@soil.face(...)` declaration, as the build reads it.
/// 构建读到的一条 `@soil.face(...)` 声明。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PythonFace {
    /// The package-relative source path, part of the face's identity.
    /// 包内相对源码路径，面身份的一部分。
    pub source: String,
    /// The declared source this file came from: its id, or `workspace` for the package itself.
    /// 这个文件来自哪个已声明来源：它的 id，包自身则是 `workspace`。
    pub origin: String,
    /// The class the decorator is attached to.
    /// 装饰器所附着的类。
    pub class: String,
    /// The display name from `named=`.
    /// `named=` 里的显示名。
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
const SKIPPED: &[&str] = &[
    "target",
    ".git",
    ".xirang",
    "__pycache__",
    ".venv",
    "node_modules",
    "crates",
];

/// What the shared declaration walk needs to know about Python.
/// 共享声明遍历需要知道的 Python 那部分。
const DECLARATIONS: crate::languages::kit::declarations::DeclarationSpec =
    crate::languages::kit::declarations::DeclarationSpec {
        extensions: &["py"],
        skip_directories: SKIPPED,
        marker: mentions,
        // A fact reader must not report "no declaration" for a file it never opened.
        // 事实读取器绝不能把自己从没打开过的文件报成"没有声明"。
        unreadable: crate::languages::kit::declarations::Unreadable::Refuse,
    };

/// Whether a text names the decorator this reader parses.
/// 一份文本是否点名本读取器解析的装饰器。
fn mentions(text: &str) -> bool {
    super::DECORATORS.iter().any(|marker| text.contains(marker))
}

/// Every Python declaration in the package, sorted by source.
/// 包里的每条 Python 声明，按源码排序。
///
/// The files are enumerated by [`xirang_multilang::source_scope`], not by a walk of this language's
/// own: which sources a read may enter is one rule, and a second copy of it is how the Python reader
/// came to follow an undeclared link out of the package. The scope is the one that also carries the
/// project's **declared** sources, so an explicitly declared external source is read and an
/// undeclared link is refused — the same two answers for every language.
/// 文件由 [`xirang_multilang::source_scope`] 枚举，而不是由这门语言自己的遍历决定：一次读取可以进入
/// 哪些来源是一条规则，而第二份拷贝正是 Python 读取器跟随未声明链接走出包外的原因。这个范围同时携带项目
/// **声明**的来源，因此显式声明的外部来源会被读取、未声明的链接被拒绝——对每种语言是同两个答案。
/// 文件由 [`xirang_multilang::source_scope`] 枚举，而不是由这门语言自己的遍历决定：一次读取可以进入哪些
/// 来源是一条规则，而第二份拷贝正是 Python 读取器跟随未声明链接走出包外的原因。
pub fn read_declarations(package_root: &Path) -> Result<Vec<PythonFace>, String> {
    read_declarations_in(&SourceScope::with_sources(package_root)?)
}

/// Every Python declaration under one already-decided source scope.
/// 在一个已经决定好的来源范围下的每条 Python 声明。
pub fn read_declarations_in(scope: &SourceScope) -> Result<Vec<PythonFace>, String> {
    // The declarations decide which files belong to a declared source; a file under none of them
    // is the package's own. Reading them once is a lookup, not a second enumeration.
    // 声明决定哪些文件属于某个已声明来源；不属于任何声明的文件是包自身的。读一次是查表，不是第二次枚举。
    let map = match scope.package_root() {
        Some(root) => crate::source_map::SourceMap::read_from(root)?,
        None => crate::source_map::SourceMap::empty(),
    };
    let mut faces =
        crate::languages::kit::declarations::read_in(scope, &DECLARATIONS, |relative, text| {
            let declaration =
                super::read_declaration(text).map_err(|error| format!("{relative}: {error}"))?;
            let name = declaration.kind();
            if name.is_empty() {
                return Err(
                    "the declaration has no `named=`, so the face has no display name".to_owned(),
                );
            }
            let parent = parent_module(&declaration.face.parent)?;
            Ok(vec![PythonFace {
                origin: map.origin_of(relative).to_owned(),
                source: relative.to_owned(),
                class: declaration.class,
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
fn parent_module(spelling: &str) -> Result<String, String> {
    let path = spelling
        .trim()
        .strip_suffix(".NODE_ID")
        .ok_or_else(|| format!("`{spelling}` does not end in `.NODE_ID`"))?;
    let path = path
        .strip_prefix("faces.")
        .or_else(|| path.strip_prefix("xirang.faces."))
        .ok_or_else(|| {
            format!(
                "`{spelling}` is not a generated `faces.…` reference; a Python parent is named the \
                 way the SDK documents it (`faces.<module>.NODE_ID`)"
            )
        })?;
    let segments: Vec<&str> = path.split('.').collect();
    if segments.iter().any(|segment| segment.is_empty()) {
        return Err(format!("`{spelling}` has an empty module segment"));
    }
    Ok(segments.join("::"))
}

/// One boolean field, refusing text that is not a Python boolean.
/// 一个布尔字段，拒绝不是 Python 布尔的文本。
fn boolean(
    registration: &xirang_kernel::authoring::RegistrationUpdate,
    field: RegistrationField,
) -> Result<bool, String> {
    match registration.find_field(field) {
        FieldUpdate::Keep | FieldUpdate::Clear => Ok(false),
        FieldUpdate::Set(value) => match value {
            "True" | "true" => Ok(true),
            "False" | "false" => Ok(false),
            other => Err(format!(
                "`{}` is `{other}`; it must be `True` or `False`",
                field.as_str()
            )),
        },
    }
}

/// One list field, split the way the request vocabulary stores it.
/// 一个列表字段，按请求词汇存储它的方式拆分。
fn list(
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

#[cfg(test)]
#[path = "python_faces_tests.rs"]
mod python_faces_tests;

/// Where one keyword argument's value ends: the next top-level comma.
/// 一个关键字参数取值的结束处：下一个顶层逗号。
pub(super) fn value_end(text: &str, from: usize) -> usize {
    let bytes = text.as_bytes();
    let mut depth = 0usize;
    let mut index = from;
    while index < bytes.len() {
        match bytes[index] {
            b'"' | b'\'' => {
                let quote = bytes[index];
                index += 1;
                while index < bytes.len() && bytes[index] != quote {
                    if bytes[index] == b'\\' {
                        index += 1;
                    }
                    index += 1;
                }
            }
            b'(' | b'[' | b'{' => depth += 1,
            b')' | b']' | b'}' => depth = depth.saturating_sub(1),
            b',' if depth == 0 => return index,
            _ => {}
        }
        index += 1;
    }
    text.len()
}

/// The index of the `)` matching the `(` whose body starts at `open`.
/// 与 `open` 处开始的 `(` 配对的 `)` 的下标。
pub(super) fn matching_paren(text: &str, open: usize) -> Option<usize> {
    let bytes = text.as_bytes();
    let mut depth = 1usize;
    let mut index = open;
    while index < bytes.len() {
        match bytes[index] {
            b'"' | b'\'' => {
                let quote = bytes[index];
                index += 1;
                while index < bytes.len() && bytes[index] != quote {
                    if bytes[index] == b'\\' {
                        index += 1;
                    }
                    index += 1;
                }
            }
            b'#' => {
                index = text[index..]
                    .find('\n')
                    .map_or(bytes.len(), |at| index + at);
            }
            b'(' | b'[' | b'{' => depth += 1,
            b')' | b']' | b'}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(index);
                }
            }
            _ => {}
        }
        index += 1;
    }
    None
}

/// The elements of a Python tuple or list literal.
/// Python 元组或列表字面量的元素。
pub(super) fn sequence(raw: &str) -> Result<Vec<String>, String> {
    let raw = raw.trim();
    let body = raw
        .strip_prefix('(')
        .and_then(|rest| rest.strip_suffix(')'))
        .or_else(|| {
            raw.strip_prefix('[')
                .and_then(|rest| rest.strip_suffix(']'))
        })
        .ok_or_else(|| format!("`{raw}` is not a tuple or list"))?;
    let mut entries = Vec::new();
    let mut rest = body;
    loop {
        rest = rest
            .trim_start_matches(|character: char| character.is_whitespace() || character == ',');
        if rest.is_empty() {
            break;
        }
        let end = value_end(rest, 0);
        entries.push(rest[..end].trim().to_owned());
        rest = &rest[end..];
    }
    Ok(entries)
}

/// One Python string literal, without quotes.
/// 一个 Python 字符串字面量，去掉引号。
pub(super) fn string(raw: &str) -> Result<String, String> {
    let raw = raw.trim();
    let quote = raw
        .chars()
        .next()
        .filter(|character| *character == '"' || *character == '\'')
        .ok_or_else(|| format!("`{raw}` is not a string literal"))?;
    let body = raw
        .strip_prefix(quote)
        .and_then(|rest| rest.strip_suffix(quote))
        .ok_or_else(|| format!("`{raw}` is not one quoted string"))?;
    let mut value = String::with_capacity(body.len());
    let mut characters = body.chars();
    while let Some(character) = characters.next() {
        if character != '\\' {
            value.push(character);
            continue;
        }
        let escaped = characters
            .next()
            .ok_or_else(|| "a string literal ends with a lone backslash".to_owned())?;
        value.push(match escaped {
            'n' => '\n',
            't' => '\t',
            'r' => '\r',
            other => other,
        });
    }
    Ok(value)
}
