//! Lexical/token layer of the registration-face parser.
//! 注册面解析器的词法 / token 层。
//!
//! These helpers read `proc_macro2` token streams without interpreting the face
//! grammar: they split fields, flatten groups, render compact source text, and
//! report a spanned syntax error. The face parser in [`super`] and the host-entry
//! reader in [`super::super::entries`] share them, so both read one declaration
//! the same way.
//! 这些辅助函数只读取 `proc_macro2` token 流，不解释注册面语法：切分字段、展平分组、
//! 渲染紧凑源码文本，并报告带 span 的语法错误。位于 [`super`] 的注册面解析器与
//! [`super::super::entries`] 的宿主入口读取器共用它们，因此两者以同一方式读取同一份
//! 声明。

use proc_macro2::{Delimiter, Group, Span, TokenStream, TokenTree};

use crate::registry_core::declaration::FACE_FIELD_ORDER;

use super::{FaceSyntaxError, SyntaxLocation};

/// Render a token stream as compact Rust source text.
/// 把 token 流渲染成紧凑的 Rust 源码文本。
///
/// `TokenStream`'s own rendering spaces out `::`, which is valid but noisy in
/// generated code. A typed graft selector is emitted verbatim, so keep it close
/// to what the author wrote.
/// `TokenStream` 自身渲染会在 `::` 周围加空格，虽然合法，但在生成代码里很吵。
/// 类型化 graft 选择器会被原样发射，因此尽量贴近作者的写法。
pub fn compact_tokens(tokens: TokenStream) -> String {
    tokens
        .to_string()
        .replace(" :: ", "::")
        .replace(" (", "(")
        .replace(") ", ")")
}

/// Split a face body into `name: value` fields.
/// 把注册面主体切成 `name: value` 字段。
///
/// Authoring is hand-written, so this reader is deliberately as tolerant as the
/// compile-time front end: `,` and `;` both separate fields, a forgotten
/// separator still ends a field at the next `name:`, a trailing separator is
/// ignored, and the fields may appear in any order. A path's colon is joint
/// with the next colon, so `crate::x` stays inside its value rather than
/// starting a field.
/// 作者是手写注册面的，因此这个读取器刻意与编译期前端一样宽容：`,` 与 `;` 都算分隔
/// 符，漏写分隔符也能在下一个 `name:` 处收尾，末尾多余的分隔符被忽略，字段顺序任意。
/// 路径的冒号与下一个冒号相连，因此 `crate::x` 留在自己的值里，不会被当成新字段。
#[doc(hidden)]
pub fn split_face_fields(tokens: TokenStream) -> Vec<TokenStream> {
    let tokens = tokens.into_iter().collect::<Vec<_>>();
    let mut fields = Vec::new();
    let mut current = TokenStream::new();
    let mut value_started = false;
    let mut angles = 0usize;
    let mut index = 0;
    while index < tokens.len() {
        if let TokenTree::Punct(punct) = &tokens[index] {
            match punct.as_char() {
                // `<` and `>` are punctuation, not delimiters, so a generic
                // argument list has to be tracked by hand: its commas separate
                // type arguments, not fields. `>>` closes two levels at once.
                // `<`/`>` 是标点而不是定界符，因此泛型实参列表必须手工跟踪深度：其中
                // 的逗号分隔的是类型实参而不是字段。`>>` 一次关闭两层。
                '<' => {
                    // Only a `<` that actually closes before the next field opens a
                    // generic argument list. Counting every `<` meant a comparison in
                    // a value (`flow: |a: u32| a < b, needs_registry: true`) left the
                    // depth at one, so the following commas stopped separating fields
                    // and every later field was swallowed into this value — silently,
                    // since the parser then applied the defaults (audit `KRN-K-14`).
                    // 只有确实在下一个字段之前闭合的 `<` 才打开泛型实参列表。把每个 `<` 都算上，
                    // 意味着取值里的一个比较（`flow: |a: u32| a < b, needs_registry: true`）会让
                    // 深度停在 1，后续逗号不再分隔字段，后面每个字段都被吞进这个值——而且是静默的，
                    // 因为解析器随后套用了默认值（审计 `KRN-K-14`）。
                    if closes_before_the_next_field(&tokens, index) {
                        angles += if punct.spacing() == proc_macro2::Spacing::Joint {
                            2
                        } else {
                            1
                        };
                    }
                    current.extend([tokens[index].clone()]);
                    value_started = true;
                    index += 1;
                    continue;
                }
                '>' => {
                    let close = if punct.spacing() == proc_macro2::Spacing::Joint {
                        2
                    } else {
                        1
                    };
                    angles = angles.saturating_sub(close);
                    current.extend([tokens[index].clone()]);
                    index += 1;
                    continue;
                }
                ',' | ';' if angles == 0 => {
                    if !current.is_empty() {
                        fields.push(std::mem::take(&mut current));
                        value_started = false;
                    }
                    index += 1;
                    continue;
                }
                _ => {}
            }
        }
        if angles == 0 && value_started && starts_face_field(&tokens, index) {
            fields.push(std::mem::take(&mut current));
            value_started = false;
            continue;
        }
        value_started = true;
        current.extend([tokens[index].clone()]);
        index += 1;
    }
    if !current.is_empty() {
        fields.push(current);
    }
    fields
}

/// Whether the `<` at `index` closes before the next field begins.
/// `index` 处的 `<` 是否在下一个字段开始之前闭合。
///
/// The `<` is a generic argument list when its `>` arrives first, and a comparison when
/// the next field (`name:`) arrives first instead — a generic argument list cannot
/// contain a `name:` at its own depth, and a comparison in a value is followed by the
/// next field. Walking groups as opaque tokens keeps a `>` inside a nested group from
/// being read as this list's closer.
/// `<` 的 `>` 先到就是泛型实参列表；下一个字段（`name:`）先到就是比较——泛型实参列表在同层不会
/// 出现 `name:`，而取值里的比较之后就是下一个字段。把分组当作不可分割的 token 走过，可以避免把嵌套
/// 分组里的 `>` 读成本列表的闭合。
fn closes_before_the_next_field(tokens: &[TokenTree], index: usize) -> bool {
    let mut depth = 1usize;
    let mut cursor = index + 1;
    while cursor < tokens.len() {
        match &tokens[cursor] {
            TokenTree::Punct(punct) if punct.as_char() == '<' => {
                depth += if punct.spacing() == proc_macro2::Spacing::Joint {
                    2
                } else {
                    1
                };
            }
            TokenTree::Punct(punct) if punct.as_char() == '>' => {
                let close = if punct.spacing() == proc_macro2::Spacing::Joint {
                    2
                } else {
                    1
                };
                match depth.checked_sub(close) {
                    Some(remaining) => depth = remaining,
                    None => return true,
                }
                if depth == 0 {
                    return true;
                }
            }
            _ => {
                if starts_face_field(tokens, cursor) {
                    return false;
                }
            }
        }
        cursor += 1;
    }
    false
}

/// Whether a `name:` begins at this token, which is how a missing separator is
/// detected. `crate::x` does not: its first colon is joint with the second.
/// 这里是否开始了 `name:`——漏写分隔符就是靠它发现的。`crate::x` 不算：它的第一个
/// 冒号与第二个相连。
fn starts_face_field(tokens: &[TokenTree], index: usize) -> bool {
    let Some(TokenTree::Ident(name)) = tokens.get(index) else {
        return false;
    };
    // Only a name the vocabulary knows starts a field. That is what keeps a
    // closure's typed parameter (`|a: u32| a`) or any other `name:` inside a
    // value from splitting the field in two.
    // 只有词表认识的键才算字段起点。闭包的类型标注参数（`|a: u32| a`）或值里其它
    // `name:` 形状因此不会把字段切成两半。
    if !FACE_FIELD_ORDER.contains(&name.to_string().as_str()) {
        return false;
    }
    match tokens.get(index + 1) {
        Some(TokenTree::Punct(colon)) if colon.as_char() == ':' => {
            colon.spacing() == proc_macro2::Spacing::Alone
        }
        _ => false,
    }
}

/// Split a token stream at its top-level commas.
/// 按顶层逗号切分 token 流。
#[doc(hidden)]
pub fn split_top_level(tokens: TokenStream) -> Vec<TokenStream> {
    let mut items = Vec::new();
    let mut current = TokenStream::new();
    let mut depth = 0usize;
    let mut after_segment = false;
    for token in tokens {
        match &token {
            // A comma inside `<…>` separates generic arguments, not list items:
            // `ControlHandle<u8, u16>` is one path. Splitting it in half made both halves
            // unparseable and the caller fell back to an empty label list, which is how a
            // correctly written face was reported as missing its trait (audit `LGC-LG-30`).
            // `<…>` 里的逗号分隔的是泛型实参，不是列表项：`ControlHandle<u8, u16>` 是一条路径。
            // 把它劈成两半后两半都解析不了，调用方退回空标签列表——于是一个写得正确的注册面被报成
            // 缺 trait（审计 `LGC-LG-30`）。
            TokenTree::Punct(punct) if punct.as_char() == ',' && depth == 0 => {
                if !current.is_empty() {
                    items.push(current);
                    current = TokenStream::new();
                }
                after_segment = false;
                continue;
            }
            // Only a `<` that follows a path segment opens generics; a comparison in a
            // value (`a < b, c`) keeps the list's own commas.
            // 只有紧跟路径段之后的 `<` 才打开泛型；取值里的比较（`a < b, c`）仍按列表的逗号切。
            TokenTree::Punct(punct) if punct.as_char() == '<' && after_segment => depth += 1,
            TokenTree::Punct(punct) if punct.as_char() == '>' && depth > 0 => depth -= 1,
            _ => {}
        }
        after_segment = matches!(&token, TokenTree::Ident(_))
            || matches!(&token, TokenTree::Punct(punct) if punct.as_char() == '>');
        current.extend([token]);
    }
    if !current.is_empty() {
        items.push(current);
    }
    items
}

/// Render a Rust path without its generic arguments, `::`-joined.
/// 把 Rust 路径渲染成以 `::` 连接的文本，不含泛型实参。
#[doc(hidden)]
pub fn path_to_string(path: &syn::Path) -> String {
    let mut output = String::new();
    if path.leading_colon.is_some() {
        output.push_str("::");
    }
    for (index, segment) in path.segments.iter().enumerate() {
        if index > 0 {
            output.push_str("::");
        }
        output.push_str(&segment.ident.to_string());
    }
    output
}

/// Whether `path` names exactly `needle`, or a `needle` inside it (`…::needle`).
/// `path` 是否正好就是 `needle`，或 `needle` 位于其中（`…::needle`）。
///
/// A suffix match is not enough: `crate::NOT_ROOT_NODE_ID` ends with `ROOT_NODE_ID`,
/// `my_root_node_id` ends with `root_node_id`, and `OwnNodeId::from_path` ends with
/// `NodeId::from_path`, so a plain `ends_with` classified each look-alike as the parent the face
/// declared (audit `KN6`).
/// 只匹配后缀不够：`crate::NOT_ROOT_NODE_ID` 以 `ROOT_NODE_ID` 结尾、`my_root_node_id` 以
/// `root_node_id` 结尾、`OwnNodeId::from_path` 以 `NodeId::from_path` 结尾，因此纯 `ends_with`
/// 会把每个相似物都判成该面声明的那个父级（审计 `KN6`）。
#[doc(hidden)]
pub fn is_path_to(path: &str, needle: &str) -> bool {
    path == needle
        || path
            .strip_suffix(needle)
            .is_some_and(|prefix| prefix.ends_with("::"))
}

/// The 1-based source location of a span's start.
/// span 起点的 1 起始源码位置。
#[doc(hidden)]
pub fn location(span: Span) -> SyntaxLocation {
    let start = span.start();
    SyntaxLocation {
        line: start.line,
        column: start.column + 1,
    }
}

/// The 1-based source location of a span's exclusive end.
/// span 排他终点的 1 起始源码位置。
pub(super) fn end_location(span: Span) -> SyntaxLocation {
    let end = span.end();
    SyntaxLocation {
        line: end.line,
        column: end.column + 1,
    }
}

/// A syntax error carrying `span` as its location.
/// 以 `span` 作为位置的语法错误。
#[doc(hidden)]
pub fn syntax_error(span: Span, message: impl Into<String>) -> FaceSyntaxError {
    FaceSyntaxError {
        message: message.into(),
        location: Some(location(span)),
    }
}

/// The string value of a literal expression, when it is one.
/// 字面量表达式的字符串值（当它确实是字面量时）。
pub(super) fn literal_string(expression: &syn::Expr) -> Option<String> {
    let syn::Expr::Lit(expression) = expression else {
        return None;
    };
    let syn::Lit::Str(value) = &expression.lit else {
        return None;
    };
    Some(value.value())
}

/// The sole group of `tokens`, when it has exactly this delimiter.
/// `tokens` 中唯一的分组（当它正好使用该定界符时）。
/// The one bracketed group a list field writes, optionally behind a leading borrow.
/// 某个列表字段写下的那一个方括号组，前面可以带一个引用符号。
///
/// The new spelling (H/A2e) writes the field's real Rust type — `&'static [&'static str]` — so `&[` is how
/// an author writes `exports`. The macro spellings write `[`. Both are the same list, and the difference is
/// the borrow, not the shape, so it is absorbed here rather than in every reader.
/// 新拼法（H/A2e）写的是字段真实的 Rust 类型——`&'static [&'static str]`——因此作者写 `exports` 时是 `&[`。
/// 宏那两种拼法写 `[`。两者是同一份列表，差别只在那个引用符号、不在形状，所以在这里吸收，而不是让每个读取器
/// 各自处理。
pub(super) fn list_group(tokens: &TokenStream, delimiter: Delimiter) -> Option<Group> {
    let mut stream = tokens.clone().into_iter();
    if matches!(stream.clone().next(), Some(TokenTree::Punct(punct)) if punct.as_char() == '&') {
        stream.next();
    }
    only_group(&stream.collect::<TokenStream>(), delimiter)
}

pub(super) fn only_group(tokens: &TokenStream, delimiter: Delimiter) -> Option<Group> {
    let mut tokens = tokens.clone().into_iter();
    let TokenTree::Group(group) = tokens.next()? else {
        return None;
    };
    (group.delimiter() == delimiter && tokens.next().is_none()).then_some(group)
}

#[cfg(test)]
mod tests {
    use super::{split_face_fields, split_top_level};
    use proc_macro2::TokenStream;

    /// A comma inside `<…>` separates generic arguments, not list items. Splitting it as a
    /// list separator cut `ControlHandle<u8, u16>` into two unparseable halves, and the
    /// caller then fell back to an empty label list — so a face that named its trait
    /// correctly was reported as missing it (audit `LGC-LG-30`).
    /// `<…>` 里的逗号分隔的是泛型实参而不是列表项。把它当列表分隔符，会把
    /// `ControlHandle<u8, u16>` 切成两半、两半都解析不了，调用方随后退回空标签列表——于是一个把
    /// trait 写对了的注册面被报成缺 trait（审计 `LGC-LG-30`）。
    #[test]
    fn a_comma_inside_generic_arguments_is_not_a_list_separator() {
        let tokens: TokenStream = "crate::ui::ControlHandle<u8, u16>, crate::parts::ActionParts"
            .parse()
            .unwrap();
        let items = split_top_level(tokens);
        assert_eq!(
            items.len(),
            2,
            "one comma is a separator, the other is not: {items:?}"
        );
        assert_eq!(
            items[0].to_string().replace(' ', ""),
            "crate::ui::ControlHandle<u8,u16>",
            "{items:?}"
        );
    }

    /// A generic argument list and a closure parameter list are values, not
    /// field lists: their commas and `name:` pairs must not split a field.
    /// 泛型实参列表与闭包参数列表是值而不是字段列表：其中的逗号与 `name:` 都不该把
    /// 字段切开。
    #[test]
    fn face_field_splitting_survives_generics_and_closures() {
        let body: TokenStream = syn::parse_str(
            "kind: Tool, preset: crate::P<u8, u16>, flow: |a: u32| a, needs_registry: true",
        )
        .expect("token stream");
        let fields = split_face_fields(body);
        let names = fields
            .iter()
            .map(|field| {
                field
                    .clone()
                    .into_iter()
                    .next()
                    .map_or_else(String::new, |token| token.to_string())
            })
            .collect::<Vec<_>>();
        assert_eq!(names, ["kind", "preset", "flow", "needs_registry"]);
        assert!(
            fields[1].to_string().contains("P < u8 , u16 >"),
            "the generic comma stays in the value: {}",
            fields[1]
        );
        assert!(
            fields[2].to_string().contains("| a : u32 | a"),
            "the closure parameter stays in the value: {}",
            fields[2]
        );
        assert_eq!(
            split_face_fields(syn::parse_str("kind: X;").expect("token stream")).len(),
            1
        );
    }

    /// A comparison in a value is not a generic argument list, so the fields after it
    /// survive.
    /// 取值里的比较不是泛型实参列表，因此它之后的字段仍然存在。
    ///
    /// Red before the fix: every `<` raised the angle depth, so `a < b` left it at one, the
    /// following commas stopped separating fields, and `needs_registry` was swallowed into
    /// the `flow` value — silently, because the parser then applied the default
    /// (audit `KRN-K-14`).
    /// 修前为红：每个 `<` 都会抬高尖括号深度，因此 `a < b` 把它留在 1，后续逗号不再分隔字段，
    /// `needs_registry` 被吞进 `flow` 的取值——而且是静默的，因为解析器随后套用了默认值
    /// （审计 `KRN-K-14`）。
    #[test]
    fn a_comparison_does_not_swallow_the_following_fields() {
        let body: TokenStream =
            syn::parse_str("kind: Tool, flow: |a: u32| a < b, needs_registry: true")
                .expect("token stream");
        let fields = split_face_fields(body);
        let names = fields
            .iter()
            .map(|field| {
                field
                    .clone()
                    .into_iter()
                    .next()
                    .map_or_else(String::new, |token| token.to_string())
            })
            .collect::<Vec<_>>();
        assert_eq!(
            names,
            ["kind", "flow", "needs_registry"],
            "the comparison keeps the following fields: {fields:?}"
        );
        assert_eq!(
            split_face_fields(
                syn::parse_str("kind: Tool, flow: |a: u32| a < b").expect("token stream")
            )
            .len(),
            2,
            "a comparison at the end of the body is still just a value"
        );
    }
}
