//! The item vocabulary: what counts as a symbol in a source file.
//! 条目词表：源码文件里什么算一个符号。
//!
//! Functions come from [`function_symbols`] (the same index the call graph uses),
//! and the remaining declarations — including the bodyless `fn` a trait declares —
//! come from one line scan over masked text. This is the one implementation of that
//! judgement: a surface that lists or searches declarations calls [`item_symbols`]
//! instead of spelling a prefix list of its own — Studio carried such a list, and it
//! both missed qualified declarations (`pub(crate) struct`) and promoted `let`
//! bindings to symbols (audit `STU-S-06`).
//! 函数来自 [`function_symbols`]（调用图用的同一份索引），其余声明——包括 trait 里那条没有
//! 函数体的 `fn`——来自对**已屏蔽文本**的一次逐行扫描。这是那条判定的唯一实现：列出或搜索声明的
//! 执行面调用 [`item_symbols`]，而不是自己拼一份前缀词表——Studio 曾有这样一份词表，它既漏掉带
//! 可见性的声明（`pub(crate) struct`），又把 `let` 绑定提升成符号（审计 `STU-S-06`）。

use super::{function_symbols, is_ident_continue, leading_identifier, mask_non_code};

/// One Rust item (declaration) discovered in a source file.
/// 在源码文件中发现的一个 Rust 条目（声明）。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceItem {
    /// Item name as written after its introducer keyword.
    /// 条目名字，即其引导关键字之后书写的标识符。
    pub name: String,
    /// Declaring line, trimmed; for a function with a body, the text through its
    /// opening brace — the same text [`function_symbols`] reports as the signature.
    /// 声明所在行的文本（已去首尾空白）；有函数体的函数则是到左花括号为止的文本——与
    /// [`function_symbols`] 报告的 signature 同一份。
    pub signature: String,
    /// 1-based line of the declaration.
    /// 以 1 起始的声明行号。
    pub line: u32,
    /// Whether this item is a function: the only kind a caller can promote to a call
    /// graph, and the only kind that can have a body.
    /// 本条目是否为函数：只有函数能被调用方推进调用图，也只有函数可能有函数体。
    pub is_function: bool,
}

/// The introducer keywords that name a non-function item.
/// 命名非函数条目的引导关键字。
const INTRODUCERS: [&str; 8] = [
    "struct", "enum", "union", "trait", "type", "const", "static", "mod",
];

/// List the items declared in a source file, in source order.
/// 按源码顺序列出源码文件中声明的条目。
///
/// The scan runs on `mask_non_code` output and takes the declaration text from the
/// raw line, so a keyword inside a comment, a string or a character literal is not an
/// item, and the reported signature is what a reader sees. `let` is deliberately not
/// an introducer: a binding has no name a call graph could reach, and listing one made
/// a search result claim a symbol `call_relations` can never find.
/// 扫描跑在 `mask_non_code` 的输出上，而声明文本取自原始行，因此注释、字符串或字符字面量里的
/// 关键字不是条目，报告的 signature 也正是读者看到的那一行。`let` 刻意不是引导关键字：绑定没有
/// 调用图能到达的名字，把它列出来会让搜索结果宣称一个 `call_relations` 永远找不到的符号。
pub fn item_symbols(source: &str) -> Vec<SourceItem> {
    let functions = function_symbols(source);
    let function_lines = functions
        .iter()
        .map(|function| function.line)
        .collect::<Vec<_>>();
    let mut items = functions
        .into_iter()
        .map(|function| SourceItem {
            name: function.name,
            signature: function.signature,
            line: function.line,
            is_function: true,
        })
        .collect::<Vec<_>>();
    let masked = mask_non_code(source);
    for (index, (masked_line, raw_line)) in masked.lines().zip(source.lines()).enumerate() {
        let line = index as u32 + 1;
        if function_lines.contains(&line) {
            continue;
        }
        let Some((name, is_function)) = declaration_on_line(masked_line) else {
            continue;
        };
        items.push(SourceItem {
            name,
            signature: raw_line.trim().to_owned(),
            line,
            is_function,
        });
    }
    // Source order, so a list shows the items the way a reader meets them: a function
    // that starts after a declaration must not sort ahead of it.
    // 源码顺序，使列表按读者遇到条目的顺序展示：比某条声明更晚开始的函数不得排在它前面。
    items.sort_by_key(|item| item.line);
    items
}

/// The item one masked line declares, as `(name, is_function)`.
/// 一条已屏蔽的行所声明的条目，形如 `(名字, 是否函数)`。
///
/// Leading modifiers are consumed first — `pub`, a `pub(...)` visibility qualifier,
/// `unsafe`, `async`, `const` in `const fn` — and then the introducer keyword names
/// the item. The keyword has to be an identifier written at the start of the remaining
/// text, which is what keeps `fn(u32) -> u32` (a function-pointer type) and
/// `let s = f;` out: after `fn` there is no name, and `let` is not an introducer.
/// 先吃掉前导修饰符——`pub`、`pub(...)` 可见性限定、`unsafe`、`async`、`const fn` 里的
/// `const`——再由引导关键字命名条目。该关键字必须是剩余文本开头的一个标识符，这正是把
/// `fn(u32) -> u32`（函数指针类型）与 `let s = f;` 挡在外面的原因：`fn` 之后没有名字，而 `let`
/// 不是引导关键字。
fn declaration_on_line(masked_line: &str) -> Option<(String, bool)> {
    let mut rest = masked_line.trim_start();
    if let Some(after) = keyword_rest(rest, "pub") {
        rest = after.trim_start();
        if let Some(inside) = rest.strip_prefix('(') {
            rest = inside.split_once(')')?.1.trim_start();
        }
    }
    for modifier in ["unsafe", "async"] {
        if let Some(after) = keyword_rest(rest, modifier) {
            rest = after.trim_start();
        }
    }
    let (keyword, after) = leading_identifier(rest)?;
    let rest = after.trim_start();
    if keyword == "fn" {
        return Some((leading_identifier(rest)?.0, true));
    }
    if keyword == "const"
        && let Some((next, after_next)) = leading_identifier(rest)
        && next == "fn"
    {
        return Some((leading_identifier(after_next.trim_start())?.0, true));
    }
    if !INTRODUCERS.contains(&keyword.as_str()) {
        return None;
    }
    Some((leading_identifier(rest)?.0, false))
}

/// The text after `keyword`, when `text` starts with that whole word.
/// 当 `text` 以该完整单词开头时，返回 `keyword` 之后的文本。
fn keyword_rest<'a>(text: &'a str, keyword: &str) -> Option<&'a str> {
    let after = text.strip_prefix(keyword)?;
    if after.starts_with(is_ident_continue) {
        return None;
    }
    Some(after)
}
