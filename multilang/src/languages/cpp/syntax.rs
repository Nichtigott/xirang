//! Lexical facts for the two C++ declaration spellings, shared by authoring and the build.
//! 两种 C++ 声明拼法的词法事实，由创作侧与构建侧共用。
//!
//! A `contains("XIRANG_SOIL")` check is not a declaration check: the word can sit in a comment, a
//! string, or a longer identifier. This module tokenizes C++ far enough to tell code from comments
//! and literals, then recognises the two spellings by their tokens. It is deliberately not a C++
//! front end — it does not evaluate templates or preprocess — but it is the one place both the
//! authoring adapter (`languages/cpp`) and the build scanner (`declared_native_scan`) ask "what does
//! this file declare?" so the two can never disagree about which bytes are a declaration.
//! `contains("XIRANG_SOIL")` 不是声明检查：这个词可以出现在注释、字符串或更长的标识符里。本模块把
//! C++ 切到足以区分代码、注释与字面量的程度，再按 token 识别两种拼法。它刻意不是 C++ 前端——不推导
//! 模板、不做预处理——但它是创作适配器与构建扫描器共同询问"这个文件声明了什么"的唯一地方，因此两者
//! 不会对"哪些字节是声明"产生分歧。

use std::ops::Range;

/// Which spelling one declaration uses.
/// 一条声明所用的拼法。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Spelling {
    /// `XIRANG_SOIL` + `constexpr auto SOIL = …`; the author-facing spelling.
    /// `XIRANG_SOIL` + `constexpr auto SOIL = …`；面向作者的拼法。
    Marked,
    /// `XIRANG_FACE(record, (expression))`; the version-1 spelling.
    /// `XIRANG_FACE(record, (expression))`；版本 1 拼法。
    Compat,
}

/// One declaration, as the tokens tell it.
/// 一条声明，按 token 读到的样子。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Declaration {
    /// Which spelling the declaration uses.
    /// 声明所用的拼法。
    pub spelling: Spelling,
    /// The record variable a version-1 macro declares; empty for the marked spelling.
    /// 版本 1 宏声明的记录变量名；带标记的拼法下为空。
    pub variable: String,
    /// The business type inside `soil<T>()`, as written.
    /// `soil<T>()` 里的业务类型，按书写原样。
    pub business_type: String,
    /// The parent expression from `.under(…)`, as written; `None` when absent.
    /// `.under(…)` 里的父表达式，按书写原样；没有时为空。
    pub parent: Option<String>,
    /// The face name from `.named("…")`, as written; `None` when absent.
    /// `.named("…")` 里的面名，按书写原样；没有时为空。
    pub name: Option<String>,
    /// Every builder call in order, as `(method, raw argument text)`.
    /// 按顺序记录的每个构造器调用，形如 `(方法, 原样参数文本)`。
    pub calls: Vec<(String, String)>,
    /// The byte range of the `.under(…)` argument, when the declaration has one.
    /// `.under(…)` 参数的字节区间（声明有此调用时）。
    pub under_argument: Option<Range<usize>>,
    /// The byte range a precise edit replaces: the expression for the marked spelling, and the
    /// parenthesised expression argument for version 1.
    /// 精确编辑替换的字节区间：带标记的拼法是表达式，版本 1 是括号包住的表达式参数。
    pub expression: Range<usize>,
    /// The whole declaration's byte range.
    /// 整条声明的字节区间。
    pub statement: Range<usize>,
}

impl Declaration {
    /// Whether this declaration says it owns a registry, in either accepted spelling.
    /// 这条声明是否说自己自持注册机——两种可接受拼法都算。
    pub fn owns_registry(&self) -> bool {
        self.calls
            .iter()
            .any(|(method, argument)| match method.as_str() {
                "owns_registry" => true,
                "needs_registry" => argument.trim() == "true",
                _ => false,
            })
    }
}

/// The declarations a source writes, in source order.
/// 一份源码写下的声明，按源码顺序。
///
/// Comments, string literals and character literals are skipped by the tokenizer, so a marker word
/// inside any of them is not a declaration. Two declarations in one file are returned as two; the
/// caller decides whether that is supported (authoring edits one; the build names one face per
/// translation unit).
/// 注释、字符串字面量与字符字面量都被分词器跳过，因此其中出现的标记词不是声明。一个文件里的两条声明
/// 会作为两条返回；由调用方决定是否支持（创作侧一次编辑一条；构建侧一个翻译单元一个面）。
pub fn declarations(source: &str) -> Result<Vec<Declaration>, String> {
    let tokens = tokenize(source)?;
    let mut declarations = Vec::new();
    let mut index = 0usize;
    while index < tokens.len() {
        match &tokens[index].kind {
            TokenKind::Ident("XIRANG_SOIL") => {
                let (declaration, next) = marked(source, &tokens, index)?;
                declarations.push(declaration);
                index = next;
            }
            TokenKind::Ident("XIRANG_FACE") => {
                let (declaration, next) = compat(source, &tokens, index)?;
                declarations.push(declaration);
                index = next;
            }
            _ => index += 1,
        }
    }
    Ok(declarations)
}

/// Whether the source mentions either declaration marker as code.
/// 源码是否以代码形式提到任一声明标记。
pub fn mentions_declaration(source: &str) -> bool {
    tokenize(source)
        .map(|tokens| {
            tokens.iter().any(|token| {
                matches!(
                    token.kind,
                    TokenKind::Ident("XIRANG_SOIL") | TokenKind::Ident("XIRANG_FACE")
                )
            })
        })
        .unwrap_or(false)
}

/// Read exactly one declaration; a file that declares none or several is refused by name.
/// 只读一条声明；没有声明或声明了多条的文件按名拒绝。
pub fn read_one(source: &str) -> Result<Declaration, String> {
    let mut declarations = declarations(source)?;
    match declarations.len() {
        1 => Ok(declarations.remove(0)),
        0 => Err(
            "this file has no face declaration; expected `XIRANG_SOIL` followed by \
             `constexpr auto SOIL = …` or a version-1 `XIRANG_FACE(…)`"
                .to_owned(),
        ),
        count => Err(format!(
            "this file declares {count} faces; this adapter edits one declaration at a time, so \
             name the file that holds the face you mean (one translation unit is one face)"
        )),
    }
}

/// One token the parser reads.
/// 解析器读取的一个 token。
#[derive(Clone, Debug, PartialEq, Eq)]
struct Token {
    kind: TokenKind,
    range: Range<usize>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TokenKind {
    /// An identifier, with its text.
    /// 标识符及其文本。
    Ident(&'static str),
    /// An arbitrary identifier this parser does not specialise.
    /// 本解析器不专门处理的任意标识符。
    Word,
    /// A string literal, including its quotes.
    /// 字符串字面量，含引号。
    String,
    /// A raw string literal, including its delimiters.
    /// 原始字符串字面量，含分隔符。
    RawString,
    /// A character literal, including its quotes.
    /// 字符字面量，含引号。
    Char,
    /// A number.
    /// 数字。
    Number,
    /// One punctuation byte.
    /// 一个标点字节。
    Punct(u8),
}

/// The identifiers this parser specialises; matching by text keeps one token kind for the rest.
/// 本解析器专门处理的标识符；其余按同一 token 类型处理。
const KEYWORDS: &[&str] = &[
    "XIRANG_SOIL",
    "XIRANG_FACE",
    "constexpr",
    "auto",
    "SOIL",
    "xirang",
    "soil",
    "true",
    "false",
    "under",
    "named",
];

/// Tokenize enough C++ to tell code from comments and literals.
/// 把 C++ 切分到足以区分代码、注释与字面量。
fn tokenize(source: &str) -> Result<Vec<Token>, String> {
    let bytes = source.as_bytes();
    let mut tokens = Vec::new();
    let mut index = 0usize;
    while index < bytes.len() {
        let byte = bytes[index];
        if byte.is_ascii_whitespace() {
            index += 1;
            continue;
        }
        if byte == b'/' && bytes.get(index + 1) == Some(&b'/') {
            index = source[index..]
                .find('\n')
                .map_or(bytes.len(), |offset| index + offset);
            continue;
        }
        if byte == b'/' && bytes.get(index + 1) == Some(&b'*') {
            let end = source[index + 2..]
                .find("*/")
                .map(|offset| index + 2 + offset + 2)
                .ok_or_else(|| "an unterminated block comment".to_owned())?;
            index = end;
            continue;
        }
        if byte == b'"' {
            let end = string_end(source, index, b'"')
                .ok_or_else(|| "an unterminated string literal".to_owned())?;
            tokens.push(Token {
                kind: TokenKind::String,
                range: index..end,
            });
            index = end;
            continue;
        }
        if byte == b'\'' {
            let end = string_end(source, index, b'\'')
                .ok_or_else(|| "an unterminated character literal".to_owned())?;
            tokens.push(Token {
                kind: TokenKind::Char,
                range: index..end,
            });
            index = end;
            continue;
        }
        if byte == b'R' && bytes.get(index + 1) == Some(&b'"') {
            let end = raw_string_end(source, index)
                .ok_or_else(|| "an unterminated raw string literal".to_owned())?;
            tokens.push(Token {
                kind: TokenKind::RawString,
                range: index..end,
            });
            index = end;
            continue;
        }
        if byte.is_ascii_alphabetic() || byte == b'_' {
            let start = index;
            index += 1;
            while index < bytes.len()
                && (bytes[index].is_ascii_alphanumeric() || bytes[index] == b'_')
            {
                index += 1;
            }
            let text = &source[start..index];
            let kind = KEYWORDS
                .iter()
                .find(|keyword| **keyword == text)
                .map(|keyword| TokenKind::Ident(keyword))
                .unwrap_or(TokenKind::Word);
            tokens.push(Token {
                kind,
                range: start..index,
            });
            continue;
        }
        if byte.is_ascii_digit() {
            let start = index;
            index += 1;
            while index < bytes.len()
                && (bytes[index].is_ascii_alphanumeric()
                    || bytes[index] == b'_'
                    || bytes[index] == b'.')
            {
                index += 1;
            }
            tokens.push(Token {
                kind: TokenKind::Number,
                range: start..index,
            });
            continue;
        }
        tokens.push(Token {
            kind: TokenKind::Punct(byte),
            range: index..index + 1,
        });
        index += 1;
    }
    Ok(tokens)
}

/// The end (exclusive) of a quoted literal starting at `start`.
/// 从 `start` 开始的带引号字面量的结束位置（不含）。
fn string_end(source: &str, start: usize, quote: u8) -> Option<usize> {
    let bytes = source.as_bytes();
    let mut index = start + 1;
    while index < bytes.len() {
        match bytes[index] {
            b'\\' => index += 2,
            byte if byte == quote => return Some(index + 1),
            _ => index += 1,
        }
    }
    None
}

/// The end (exclusive) of a raw string literal starting at `R"`.
/// 从 `R"` 开始的原始字符串字面量的结束位置（不含）。
fn raw_string_end(source: &str, start: usize) -> Option<usize> {
    let bytes = source.as_bytes();
    let open = source[start + 2..].find('(')? + start + 2;
    let delimiter = &source[start + 2..open];
    let mut index = open + 1;
    while index < bytes.len() {
        if bytes[index] == b')' && source[index + 1..].starts_with(delimiter) {
            let after = index + 1 + delimiter.len();
            if bytes.get(after) == Some(&b'"') {
                return Some(after + 1);
            }
        }
        index += 1;
    }
    None
}

/// Parse one marked declaration starting at the `XIRANG_SOIL` token.
/// 从 `XIRANG_SOIL` token 开始解析一条带标记声明。
fn marked(source: &str, tokens: &[Token], marker: usize) -> Result<(Declaration, usize), String> {
    let mut index = marker + 1;
    expect_ident(tokens, &mut index, "constexpr")?;
    expect_ident(tokens, &mut index, "auto")?;
    expect_ident(tokens, &mut index, "SOIL")?;
    expect_punct(tokens, &mut index, b'=')?;
    let expression_start = tokens
        .get(index)
        .ok_or_else(|| "the marked declaration has no expression after `=`".to_owned())?
        .range
        .start;
    let semicolon = find_punct(tokens, index, b';')?;
    let expression_end = tokens[semicolon].range.start;
    let chain = parse_chain(source, tokens, index, semicolon)?;
    let declaration = Declaration {
        spelling: Spelling::Marked,
        variable: String::new(),
        business_type: chain.business_type,
        parent: chain.parent,
        name: chain.name,
        calls: chain.calls,
        under_argument: chain.under_argument,
        expression: expression_start..expression_end,
        statement: tokens[marker].range.start..tokens[semicolon].range.end,
    };
    Ok((declaration, semicolon + 1))
}

/// Parse one version-1 declaration starting at the `XIRANG_FACE` token.
/// 从 `XIRANG_FACE` token 开始解析一条版本 1 声明。
fn compat(source: &str, tokens: &[Token], marker: usize) -> Result<(Declaration, usize), String> {
    let mut index = marker + 1;
    let open = index;
    expect_punct(tokens, &mut index, b'(')?;
    let close = matching(tokens, open, b'(', b')')
        .ok_or_else(|| "the `XIRANG_FACE(` call is not closed".to_owned())?;
    let comma = find_top_level(tokens, open + 1, close, b',')
        .ok_or_else(|| "`XIRANG_FACE` takes a record variable and an expression".to_owned())?;
    let variable = source[tokens[open].range.end..tokens[comma].range.start]
        .trim()
        .to_owned();
    if variable.is_empty() {
        return Err("`XIRANG_FACE` names no record variable".to_owned());
    }
    let expression_open = comma + 1;
    if punct(tokens, expression_open) != Some(b'(') {
        return Err(
            "`XIRANG_FACE`'s second argument is not one parenthesised expression; a precise edit \
             cannot tell which part is the declaration"
                .to_owned(),
        );
    }
    let expression_close = matching(tokens, expression_open, b'(', b')')
        .ok_or_else(|| "`XIRANG_FACE`'s expression is not closed".to_owned())?;
    let chain = parse_chain(source, tokens, expression_open + 1, expression_close)?;
    let declaration = Declaration {
        spelling: Spelling::Compat,
        variable,
        business_type: chain.business_type,
        parent: chain.parent,
        name: chain.name,
        calls: chain.calls,
        under_argument: chain.under_argument,
        expression: tokens[expression_open].range.start..tokens[expression_close].range.end,
        statement: tokens[marker].range.start..tokens[close].range.end,
    };
    Ok((declaration, close + 1))
}

/// One parsed builder chain.
/// 解析后的构造器链。
struct Chain {
    business_type: String,
    parent: Option<String>,
    name: Option<String>,
    calls: Vec<(String, String)>,
    under_argument: Option<Range<usize>>,
}

/// Parse `soil<T>()` followed by `.method(…)` calls, from token `start` to `end` (exclusive).
/// 从 token `start` 到 `end`（不含）解析 `soil<T>()` 与其后的 `.method(…)` 调用。
fn parse_chain(source: &str, tokens: &[Token], start: usize, end: usize) -> Result<Chain, String> {
    let mut index = start;
    if punct(tokens, index) == Some(b':') && punct(tokens, index + 1) == Some(b':') {
        index += 2;
    }
    expect_ident(tokens, &mut index, "xirang")?;
    expect_punct(tokens, &mut index, b':')?;
    expect_punct(tokens, &mut index, b':')?;
    expect_ident(tokens, &mut index, "soil")?;
    let type_open = index;
    expect_punct(tokens, &mut index, b'<')?;
    let type_close = matching(tokens, type_open, b'<', b'>')
        .ok_or_else(|| "the `soil<…>` type argument is not closed".to_owned())?;
    let type_start = tokens[type_open].range.end;
    let business_type = source[type_start..tokens[type_close].range.start]
        .trim()
        .to_owned();
    if business_type.is_empty() {
        return Err("`soil<…>` names no business type".to_owned());
    }
    index = type_close + 1;
    expect_punct(tokens, &mut index, b'(')?;
    expect_punct(tokens, &mut index, b')')?;
    let mut calls = Vec::new();
    let mut parent = None;
    let mut name = None;
    let mut under_argument = None;
    while index < end {
        if punct(tokens, index) != Some(b'.') {
            return Err(format!(
                "the declaration has trailing text `{}`",
                source[tokens[index].range.start..tokens[end - 1].range.end].trim()
            ));
        }
        index += 1;
        let method = word(source, tokens, index).ok_or_else(|| {
            format!(
                "`.{}` is not a builder method name",
                source[tokens[index].range.start..tokens[end - 1].range.end].trim()
            )
        })?;
        index += 1;
        let arguments_open = index;
        expect_punct(tokens, &mut index, b'(')?;
        let arguments_close = matching(tokens, arguments_open, b'(', b')')
            .ok_or_else(|| format!("`.{method}(` is not closed"))?;
        let inner_open = tokens[arguments_open].range.end;
        let inner_close = tokens[arguments_close].range.start;
        let inner = source[inner_open..inner_close].trim();
        match method {
            "under" => {
                if inner.is_empty() {
                    return Err("`.under(…)` names no parent".to_owned());
                }
                if parent.replace(inner.to_owned()).is_some() {
                    return Err("the declaration calls `.under(…)` twice".to_owned());
                }
                under_argument = Some(inner_open..inner_close);
            }
            "named" => match name {
                Some(_) => return Err("the declaration calls `.named(…)` twice".to_owned()),
                None => name = Some(string_literal(inner)?),
            },
            _ => {}
        }
        calls.push((method.to_owned(), inner.to_owned()));
        index = arguments_close + 1;
    }
    Ok(Chain {
        business_type,
        parent,
        name,
        calls,
        under_argument,
    })
}

/// One C++ string literal, without quotes.
/// 一个 C++ 字符串字面量，去掉引号。
pub fn string_literal(text: &str) -> Result<String, String> {
    let text = text.trim();
    let body = text
        .strip_prefix('"')
        .and_then(|rest| rest.strip_suffix('"'))
        .ok_or_else(|| format!("`{text}` is not a string literal"))?;
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
            '0' => '\0',
            other => other,
        });
    }
    Ok(value)
}

fn ident(tokens: &[Token], index: usize) -> Option<&'static str> {
    match tokens.get(index)?.kind {
        TokenKind::Ident(word) => Some(word),
        _ => None,
    }
}

/// Any identifier's text, keyword or not.
/// 任意标识符的文本，无论是否是本解析器的关键字。
fn word<'a>(source: &'a str, tokens: &[Token], index: usize) -> Option<&'a str> {
    match tokens.get(index)?.kind {
        TokenKind::Ident(word) => Some(word),
        TokenKind::Word => {
            let range = &tokens[index].range;
            Some(&source[range.start..range.end])
        }
        _ => None,
    }
}

fn punct(tokens: &[Token], index: usize) -> Option<u8> {
    match tokens.get(index)?.kind {
        TokenKind::Punct(byte) => Some(byte),
        _ => None,
    }
}

fn expect_ident(tokens: &[Token], index: &mut usize, expected: &'static str) -> Result<(), String> {
    if ident(tokens, *index) == Some(expected) {
        *index += 1;
        Ok(())
    } else {
        Err(format!(
            "expected `{expected}` at byte {}, found `{}`",
            tokens.get(*index).map_or(0, |token| token.range.start),
            tokens
                .get(*index)
                .map(|token| format!("{:?}", token.kind))
                .unwrap_or_else(|| "end of file".to_owned())
        ))
    }
}

fn expect_punct(tokens: &[Token], index: &mut usize, expected: u8) -> Result<(), String> {
    if punct(tokens, *index) == Some(expected) {
        *index += 1;
        Ok(())
    } else {
        Err(format!(
            "expected `{}` at byte {}, found `{}`",
            expected as char,
            tokens.get(*index).map_or(0, |token| token.range.start),
            tokens
                .get(*index)
                .map(|token| format!("{:?}", token.kind))
                .unwrap_or_else(|| "end of file".to_owned())
        ))
    }
}

/// The index of the matching close token for the opener at `open`.
/// `open` 处开括号的配对闭括号下标。
fn matching(tokens: &[Token], open: usize, opener: u8, closer: u8) -> Option<usize> {
    if punct(tokens, open) != Some(opener) {
        return None;
    }
    let mut depth = 1usize;
    let mut index = open + 1;
    while index < tokens.len() {
        match tokens[index].kind {
            TokenKind::Punct(byte) if byte == opener => depth += 1,
            TokenKind::Punct(byte) if byte == closer => {
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

/// The first byte `expected` at bracket depth zero between `start` and `end`.
/// `start` 与 `end` 之间、括号深度为零的第一个 `expected`。
fn find_top_level(tokens: &[Token], start: usize, end: usize, expected: u8) -> Option<usize> {
    let mut depth = 0usize;
    for (offset, token) in tokens[start..end].iter().enumerate() {
        match token.kind {
            TokenKind::Punct(b'(' | b'[' | b'{' | b'<') => depth += 1,
            TokenKind::Punct(b')' | b']' | b'}' | b'>') => {
                depth = depth.saturating_sub(1);
            }
            TokenKind::Punct(byte) if byte == expected && depth == 0 => {
                return Some(start + offset);
            }
            _ => {}
        }
    }
    None
}

/// The first `expected` token after `start`, ignoring nested brackets.
/// `start` 之后、忽略嵌套括号的第一个 `expected` token。
fn find_punct(tokens: &[Token], start: usize, expected: u8) -> Result<usize, String> {
    let mut depth = 0usize;
    let mut index = start;
    while index < tokens.len() {
        match tokens[index].kind {
            TokenKind::Punct(b'(' | b'[' | b'{') => depth += 1,
            TokenKind::Punct(b')' | b']' | b'}') => {
                depth = depth.saturating_sub(1);
            }
            TokenKind::Punct(byte) if byte == expected && depth == 0 => return Ok(index),
            _ => {}
        }
        index += 1;
    }
    Err(format!(
        "the declaration has no closing `{}`",
        expected as char
    ))
}

#[cfg(test)]
#[path = "syntax_tests.rs"]
mod cpp_syntax_tests;

/// One declaration, as far as the shared C++ lexer can tell.
/// 一条声明——共享 C++ 词法器能告诉我们的那部分。
#[derive(Clone, Debug)]
pub struct Declared {
    /// The parent as the declaration spells it, after the generated-header prefix is removed.
    /// 父面按声明的拼法，去掉生成头前缀之后。
    pub parent: String,
    /// The face name from `.named("…")`.
    /// `.named("…")` 里的面名。
    pub name: String,
    /// Whether the declaration says it owns a registry.
    /// 声明是否说自己自持注册机。
    pub owns_registry: bool,
}

/// The declarations a source writes, in the order they appear.
/// 一份源码写下的声明，按出现顺序。
pub fn faces(text: &str) -> Result<Vec<Declared>, String> {
    crate::languages::cpp::syntax::declarations(text)?
        .into_iter()
        .map(|declaration| {
            let owns_registry = declaration.owns_registry();
            let parent = declaration.parent.unwrap_or_default();
            let parent = parent
                .trim()
                .trim_start_matches("::")
                .strip_prefix("xirang::faces::")
                .or_else(|| {
                    parent
                        .trim()
                        .trim_start_matches("::")
                        .strip_prefix("crate::catalog::")
                })
                .ok_or_else(|| {
                    format!(
                        "a declaration's `.under(…)` argument is `{}`, not a generated face \
                         reference from `xirang/faces.hpp` or `crate::catalog`; the build cannot \
                         resolve a hand-written expression",
                        parent.trim()
                    )
                })?
                .to_owned();
            Ok(Declared {
                parent,
                name: declaration.name.unwrap_or_default(),
                owns_registry,
            })
        })
        .collect()
}

/// The language-neutral payload of one declaration, for the shared adapter.
/// 一条声明的语言中立载荷，供共享适配器使用。
pub(super) fn declared(
    source: &str,
) -> Result<crate::languages::kit::grammar::DeclaredFace, String> {
    crate::languages::cpp::adapter::read_declaration(source).map(|declaration| declaration.face)
}
