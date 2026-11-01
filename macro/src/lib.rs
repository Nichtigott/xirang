//! Face field normalisation for the authoring macros.
//! 为作者侧宏归一化注册面字段。
//!
//! A `macro_rules!` matcher can only fail with "no rules expected `...`", and it
//! cannot reorder fields, cannot compare field names, and cannot attach a span
//! to a message of its own. This front end receives the author's tokens with
//! their spans, so it accepts `,` or `;` separators, a forgotten separator and
//! any field order, and still reports an unknown or repeated field on the exact
//! token that carries it.
//! `macro_rules!` 匹配失败只会说 "no rules expected `...`"，既不能重排字段、不能
//! 比较字段名，也无法把自己的消息挂到具体 token 上。本前端拿到的是带 span 的作者
//! token，因此接受 `,`/`;`、漏写分隔符与任意顺序，并把"未知字段/重复字段"精确报在
//! 出问题的那个 token 上。
//!
//! Splitting is shared with the build-time reader (`split_face_fields`): the
//! compiler and the build step must read one declaration the same way, so they
//! use the same tolerant splitter.
//! 切分逻辑与构建期读取器共用（`split_face_fields`）：编译器与构建步骤必须对同一份
//! 声明读出一致的字段，因此两者使用同一个宽容切分器。
//!
//! The normalised declaration goes back through `__xirang_object!`, the
//! exported entry point of the runtime crate, so the collector mode the caller
//! chose survives the round trip. This front end is only reached when
//! `__control_object!`'s single arm has already declined the declaration — a
//! field out of order, `;` separators, a misspelled name — so a well-formed face
//! expands exactly as it did before.
//! 归一化后的声明经 `__xirang_object!`（运行时 crate 的公开入口）回到宏阶梯，
//! 调用方选择的 collector 模式因此得以保留。只有当 `__control_object!` 那唯一一条 arm
//! 不接受时（字段顺序不同、用 `;` 分隔、字段名拼错）才会走到本前端，因此合法注册面的展开
//! 与从前完全一致。
//!
//! The proc-macro entry points plus `normalise` stay on this page; the mirror
//! emitter lives in `mirror` and the field/argument parsing helpers in
//! `front_end`.
//! 过程宏入口与 `normalise` 留在本页；镜像发射器位于 `mirror`，字段 / 参数解析辅助
//! 函数位于 `front_end`。

// The published surface must be readable on docs.rs without leaving the page,
// so the lint is on for the whole crate; `clippy -D warnings` makes a new
// undocumented public item a failure.
// 发布表面必须能在 docs.rs 上不跳页读懂，因此 lint 开在整个 crate 上；
// `clippy -D warnings` 会让新增的、没有文档的公开项变成失败。
#![warn(missing_docs)]

use proc_macro::TokenStream;
use proc_macro2::{Delimiter, Group, Ident, Spacing, Span, TokenStream as Tokens, TokenTree};

use xirang_kernel::lexicon;

use xirang_kernel::registry_core::declaration::FACE_FIELD_ORDER;
use xirang_kernel::registry_core::syntax::split_face_fields;

use crate::front_end::{Field, error_at, punct, render, splice, split_semicolons};

#[path = "front_end.rs"]
mod front_end;
/// Rewrite one face declaration's field list into its accepted order, with
/// tolerant separators and spanned diagnostics.
/// 把一条注册面声明的字段列表改写成它接受的顺序，容忍分隔符差异并给出带 span 的诊断。
///
/// The macro is the compile-time front end of the kernel's face grammar: it
/// parses the same tokens the authoring parser does, so the compiler and the
/// tooling reject exactly the same declarations.
/// 本宏是内核注册面语法的编译期前端：它解析与创作解析器相同的 token，因此编译器与工具
/// 拒绝的是完全相同的声明。
#[proc_macro]
pub fn face_fields(input: TokenStream) -> TokenStream {
    normalise(Tokens::from(input))
        .unwrap_or_else(|error| error)
        .into()
}

/// The `registry_rule` a face gets when it does not write one.
/// 注册面没有写 `registry_rule` 时得到什么。
///
/// The input is `<fallback> ; <needs_registry> ; <author expression?>`. The
/// author's expression always wins. A face that owns a registry
/// (`needs_registry: true`) takes the **canonical sibling rule** — the rule
/// module the authoring layout keeps beside the face folder — and every other
/// face keeps the permissive fallback the declarative layer passes in.
/// 输入是 `<fallback> ; <needs_registry> ; <作者表达式?>`。作者写下的表达式永远最优先。
/// 拥有注册机的面（`needs_registry: true`）取**同目录规范规则**——创作布局放在注册面
/// 目录旁的那个规则模块——其余注册面保留声明层传进来的宽松默认值。
///
/// The path is relative (`super::registry_rule`) on purpose, and that is why this
/// has to be a proc macro: a folder face file is loaded as the inner module of
/// its folder, so the rule module is its *sibling*, and `module_path!()` is a
/// string that cannot become a path. Tokens built here carry `Span::call_site()`,
/// so the relative path resolves in the author's own module.
/// 路径是相对的（`super::registry_rule`）而不是绝对的，这也正是它必须是过程宏的原因：
/// 文件夹注册面文件被载入为其文件夹的内层模块，因此规则模块是它的**兄弟**；而
/// `module_path!()` 是字符串，无法变成路径。这里生成的 token 带 `Span::call_site()`，
/// 因此相对路径在作者自己的模块里解析。
#[proc_macro]
pub fn face_rule_or(input: TokenStream) -> TokenStream {
    let mut parts = split_semicolons(Tokens::from(input)).into_iter();
    let fallback = parts.next().unwrap_or_default();
    let needs_registry = parts.next().unwrap_or_default();
    let authored = parts.next().unwrap_or_default();
    if !authored.is_empty() {
        return authored.into();
    }
    let owns_registry = needs_registry
        .into_iter()
        .map(|token| token.to_string())
        .collect::<String>();
    if owns_registry == "true" {
        // The IDE arm this used to carry is gone with the mirror (step ⑤ of A2e): the cfg splice existed
        // only so an editor's crate-root shadow could still compile a nested face's derived rule path.
        // `rustc` has always taken the real path, so removing the arm changes nothing it compiles.
        // 它过去带的 IDE 那一支已随镜像一并退役（A2e 第⑤步）：那条 cfg 拼接只为让编辑器看到的 crate 根
        // 影子仍能编译嵌套面的推导规则路径。`rustc` 一直走真实路径，因此去掉该支不改变它编译的任何东西。
        // A `let` rather than a `use`: the binding is a struct's associated constant, and `use` cannot
        // import one. A `let` binding in a const initializer needs no type annotation, which is what keeps
        // this template free of a type the resolver would have to name.
        // 用 `let` 而不是 `use`：绑定的是结构体的关联常量，而 `use` 导不进来。const 初始化器里的 `let`
        // 绑定不需要类型标注，这正是让模板不必说出一个解析器无从命名的类型的原因。
        let template: Tokens =
            "{ let __xirang_rule = super::registry_rule::REGISTRATION_RULE; __xirang_rule }"
                .parse()
                .expect("the derived rule template is static");
        return splice(template, "__XIRANG_IDE_FALLBACK", &fallback).into();
    }
    fallback.into()
}

/// Labels for the traits a face declares, derived from the paths it wrote.
/// 注册面声明实现的 trait 标签，从其写下的路径派生。
///
/// `face_trait_labels_or!([path, …]; [label, …])` answers with the last path
/// segment of every path when at least one path was given, and with the labels
/// verbatim when none was. That is the same rule the authoring applier
/// (`apply_trait_label`) already implements, so the file form and the compiled
/// form cannot answer "which interfaces does this face implement" differently —
/// and a face that states a compiler-checked path never has to state the label
/// twice.
/// `face_trait_labels_or!([路径, …]; [标签, …])`：只要给出至少一个路径，就用每个路径的
/// 最后一段作答；一个路径都没有时，原样交回标签。这与创作应用器
/// （`apply_trait_label`）已经实现的规则相同，因此文件形式与编译形式对"本注册面实现了
/// 哪些接口"不可能给出不同答案——写了参与编译检查的路径的注册面也不必再写一遍标签。
#[proc_macro]
pub fn face_trait_labels_or(input: TokenStream) -> TokenStream {
    let mut parts = split_semicolons(Tokens::from(input)).into_iter();
    let paths = parts.next().unwrap_or_default();
    let labels = parts.next().unwrap_or_default();
    let derived = bracket_items(&paths)
        .and_then(|items| items.iter().map(last_segment).collect::<Option<Vec<_>>>())
        .filter(|names| !names.is_empty());
    let Some(derived) = derived else {
        // No compiler-checked path: the author's labels stand as written, which
        // is what `__string_list!` produced before this macro existed.
        // 没有参与编译检查的路径：作者的标签原样成立，这正是本宏出现之前
        // `__string_list!` 产出的东西。
        let fallback = format!("&{labels}");
        return fallback
            .parse::<Tokens>()
            .map_or_else(|_| labels.into(), TokenStream::from);
    };
    let literal = format!(
        "&[{}]",
        derived
            .iter()
            .map(|name| format!("{name:?}"))
            .collect::<Vec<_>>()
            .join(", ")
    );
    literal
        .parse::<Tokens>()
        .map_or_else(|_| labels.into(), TokenStream::from)
}

/// The comma-separated items of the first bracketed group, if there is one.
/// 第一个方括号分组里以逗号分隔的条目（若有）。
fn bracket_items(tokens: &Tokens) -> Option<Vec<Tokens>> {
    let group = tokens.clone().into_iter().find_map(|token| match token {
        TokenTree::Group(group) if group.delimiter() == Delimiter::Bracket => Some(group),
        _ => None,
    })?;
    let mut items = vec![Tokens::new()];
    for token in group.stream() {
        let separator = matches!(&token, TokenTree::Punct(punct) if punct.as_char() == ',');
        if separator {
            items.push(Tokens::new());
        } else {
            items.last_mut()?.extend([token]);
        }
    }
    Some(items.into_iter().filter(|item| !item.is_empty()).collect())
}

/// The last `::`-separated segment of a path, when it is a plain identifier.
/// 路径最后一段 `::` 之后的名字，且它必须是普通标识符。
fn last_segment(path: &Tokens) -> Option<String> {
    let text = path.to_string().replace(' ', "");
    let name = text.rsplit("::").next()?.to_owned();
    let mut characters = name.chars();
    let first = characters.next()?;
    if !(first.is_alphabetic() || first == '_')
        || !characters.all(|c| c.is_alphanumeric() || c == '_')
    {
        return None;
    }
    Some(name)
}

/// Which macro the normalised declaration goes back to.
/// 归一化后的声明要回到哪个宏。
#[derive(Clone, Copy, PartialEq, Eq)]
enum Target {
    /// The generated host aliases' path.
    /// 生成的宿主别名那条路。
    Control,
    /// `external_object!`, whose matcher names the source file first.
    /// `external_object!`——它的 matcher 首要指出源文件。
    External,
}

pub(crate) fn normalise(input: Tokens) -> Result<Tokens, Tokens> {
    let mut list = input.into_iter().collect::<Vec<_>>();
    let mut target = Target::Control;
    if let (Some(TokenTree::Punct(at)), Some(TokenTree::Ident(name))) = (list.first(), list.get(1))
        && at.as_char() == '@'
    {
        target = match name.to_string().as_str() {
            "control" => Target::Control,
            "external" => Target::External,
            other => {
                return Err(error_at(
                    name.span(),
                    format!("unknown face field target `{other}`"),
                ));
            }
        };
        list.drain(0..2);
    }
    let input = list.into_iter().collect::<Tokens>();
    let original = input.to_string();
    let mut collector: Option<TokenTree> = None;
    let mut fields: Vec<Field> = Vec::new();
    for field in split_face_fields(input) {
        let tokens = field.into_iter().collect::<Vec<_>>();
        let Some(TokenTree::Ident(name)) = tokens.first().cloned() else {
            let span = tokens.first().map_or_else(Span::call_site, TokenTree::span);
            return Err(error_at(
                span,
                format!("expected a face field name, found `{}`", render(&tokens)),
            ));
        };
        match tokens.get(1) {
            Some(TokenTree::Punct(colon)) if colon.as_char() == ':' => {}
            _ => {
                return Err(error_at(
                    name.span(),
                    format!("expected `:` after face field `{name}`"),
                ));
            }
        }
        if tokens.len() == 2 {
            return Err(error_at(
                name.span(),
                format!("face field `{name}` has no value"),
            ));
        }
        if name == "source" && target == Target::Control {
            return Err(error_at(
                name.span(),
                "`source` belongs to `external_object!`; a generated host alias records \
                 the file itself"
                    .to_owned(),
            ));
        }
        if name == lexicon::FACE_FIELD_COLLECTOR {
            if collector.is_some() {
                return Err(error_at(
                    name.span(),
                    "face field `collector` is given twice".to_owned(),
                ));
            }
            collector = Some(tokens[2].clone());
            continue;
        }
        fields.push(Field {
            key: name.to_string(),
            span: name.span(),
            tokens: tokens.into_iter().collect(),
        });
    }

    let position = FACE_FIELD_ORDER
        .iter()
        .enumerate()
        .map(|(index, key)| (*key, index))
        .collect::<std::collections::BTreeMap<_, _>>();
    let mut seen: Vec<&str> = Vec::with_capacity(fields.len());
    for field in &fields {
        if !position.contains_key(field.key.as_str()) {
            return Err(error_at(
                field.span,
                format!(
                    "unknown face field `{}`; expected one of: {}",
                    field.key,
                    FACE_FIELD_ORDER.join(", ")
                ),
            ));
        }
        if seen.contains(&field.key.as_str()) {
            return Err(error_at(
                field.span,
                format!("face field `{}` is given twice", field.key),
            ));
        }
        seen.push(&field.key);
    }
    let Some(collector) = collector else {
        return Err(error_at(
            Span::call_site(),
            "a face declaration must carry `collector:`".to_owned(),
        ));
    };
    fields.sort_by_key(|field| position[field.key.as_str()]);

    let mut body = Tokens::new();
    body.extend([
        TokenTree::Ident(Ident::new(lexicon::FACE_FIELD_COLLECTOR, Span::call_site())),
        punct(':', Spacing::Alone),
        collector,
        punct(',', Spacing::Alone),
    ]);
    for field in &fields {
        body.extend(field.tokens.clone());
        body.extend([punct(',', Spacing::Alone)]);
    }
    if target == Target::Control && body.to_string() == original {
        // Reordering and re-emitting produced this very token stream, so another
        // round would only recurse: the declaration already is in the accepted
        // order and still did not match, which means a field's shape is wrong.
        // 重排与重发得到的正是这串 token，再走一轮只会递归：声明已在接受顺序上却仍未
        // 匹配，说明某个字段的写法不对。
        return Err(error_at(
            Span::call_site(),
            "these face fields are in the accepted order but the declaration still did not match; \
             check each field's shape against the field list in the macro's documentation"
                .to_owned(),
        ));
    }

    let mut output = Tokens::new();
    output.extend([
        punct(':', Spacing::Joint),
        punct(':', Spacing::Alone),
        TokenTree::Ident(Ident::new(lexicon::RUN_METHOD_CRATE, Span::call_site())),
        punct(':', Spacing::Joint),
        punct(':', Spacing::Alone),
        TokenTree::Ident(Ident::new(
            match target {
                Target::Control => "__xirang_object",
                Target::External => "__external_object",
            },
            Span::call_site(),
        )),
        punct('!', Spacing::Alone),
        TokenTree::Group(Group::new(Delimiter::Brace, body)),
    ]);
    Ok(output)
}

#[path = "native.rs"]
mod native;

/// Register a native Rust Soil const without an editor-only mirror.
/// 注册原生 Rust Soil 常量，不生成编辑器专用镜像。
#[proc_macro_attribute]
pub fn soil(args: TokenStream, input: TokenStream) -> TokenStream {
    let input = Tokens::from(input);
    match native::expand(args.into(), input.clone()) {
        Ok(output) => output.into(),
        Err(error) => {
            // Keep the real incomplete declaration visible to ordinary Rust completion.
            // 保留真实的未完成声明，让普通 Rust 补全仍能看到它。
            let diagnostic = error.to_compile_error();
            quote::quote!(#input #diagnostic).into()
        }
    }
}

/// Compatibility entry for the original `xirang_macro::face` attribute.
/// 原 `xirang_macro::face` 属性的兼容入口。
#[doc(hidden)]
#[proc_macro_attribute]
pub fn face(args: TokenStream, input: TokenStream) -> TokenStream {
    soil(args, input)
}

/// Check trait implementations and return their registration labels.
/// 检查 trait 实现并返回注册标签。
#[proc_macro]
pub fn contracts(input: TokenStream) -> TokenStream {
    match xirang_kernel::syntax::contract_paths(input.into()) {
        Ok((ty, paths)) => {
            let labels = paths
                .iter()
                .map(|p| p.segments.last().unwrap().ident.to_string())
                .collect::<Vec<_>>();
            quote::quote!({
                fn __check<T: #(#paths)+*>() {}
                let _ = __check::<#ty>;
                &[#(#labels),*]
            })
            .into()
        }
        Err(error) => error.to_compile_error().into(),
    }
}
