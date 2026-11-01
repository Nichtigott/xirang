//! Branch-level facts: the arms a static read can decide, and the ones it must not judge.
//! 分支级事实：静态读取能判定的臂，以及它绝不能判的臂。
//!
//! Two rules live here, and nothing else does. **A**: an `if`/`else if` whose condition is the
//! literal `false` — no input, no runtime state and no expansion can enter its `then` block.
//! **B**: a `match` arm whose pattern is the qualified path `Enum::Variant`, where the enum is
//! declared in this tree without `pub` and without any attribute that could build a value, and no
//! construction of `Enum::Variant` is spelled anywhere in this tree. Nothing data-dependent is
//! decided here: `if` over a field, a comparison against a parameter or a `match` over a value are
//! runtime facts, and a static read that answered them would be guessing.
//! 这里只有两条规则。**A**：条件是字面量 `false` 的 `if`/`else if`——没有任何输入、运行期状态或展开
//! 能进入它的 `then` 块。**B**：模式是限定路径 `Enum::Variant` 的 `match` 臂，而该枚举在本树里声明、
//! 不带 `pub`、也不带任何能造出值的属性，且本树任何地方都没有拼出 `Enum::Variant` 的构造。这里不判定
//! 任何数据相关的东西：对字段的 `if`、对参数的比较、对值的 `match` 都是运行期事实，静态读取去回答它们
//! 就是在猜。
//!
//! Every judgement runs on [`super::mask_non_code`] output, the workspace's one rule for what counts as
//! code. A `Band::Frozen` written inside a `//` comment is prose, and counting it as a construction
//! would silently erase rule B's row — the same reason `calls.rs` scans masked text instead of
//! re-lexing.
//! 所有判断都跑在 [`super::mask_non_code`] 的输出上——本工作区关于"什么算代码"的唯一规则。写在 `//` 注释里的
//! `Band::Frozen` 是散文，把它数成构造会静默抹掉规则 B 的那一行——与 `calls.rs` 扫掩码文本而不是自己
//! 再词法一遍的理由相同。
//!
//! This module reports **facts**, not verdicts: it says which arms are decided and which paths the
//! tree spells, and the caller joins those facts across files. That join (and the boundary it
//! states) is the execution surface's business, exactly as it is for the reachability column.
//! 本模块报的是**事实**而不是裁定：它说哪些臂被判定、本树拼出了哪些路径，而跨文件的连接由调用方做。
//! 那份连接（以及它陈述的边界）是执行面的事，与可达性那一栏完全相同。

use super::{SourceFunction, is_ident_continue, is_ident_start};

/// The derives that cannot build a value out of nothing.
/// 无法凭空造出值的派生。
///
/// A `Clone` needs a value of that variant to exist already, and `Debug`/`PartialEq` and friends
/// only read one. `Default`, a serialization derive or anything this list does not know *can*
/// build a value, so an enum carrying one is never judged — a miss, never a false row.
/// `Clone` 需要该变体的值已经存在，而 `Debug`/`PartialEq` 这类只读取它。`Default`、序列化派生或本
/// 清单不认识的任何东西**可以**造出值，因此带着它们的枚举一律不判——只会漏报，绝不误报。
const VALUE_FREE_DERIVES: [&str; 8] = [
    "Clone",
    "Copy",
    "Debug",
    "PartialEq",
    "Eq",
    "PartialOrd",
    "Ord",
    "Hash",
];

/// One `if`/`else if` whose condition is the literal `false`.
/// 一个条件是字面量 `false` 的 `if`/`else if`。
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ConstantFalseGuard {
    /// The innermost function the guard sits in.
    /// 该守卫所在的最内层函数。
    pub function: String,
    /// 1-based line of the `if` token, which is the line the row reports.
    /// 以 1 起始的 `if` token 行号——也就是那一行报告的行号。
    pub line: u32,
}

/// One `match` arm whose pattern names an enum variant by its qualified path.
/// 一个模式以限定路径点名枚举变体的 `match` 臂。
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct MatchedVariant {
    /// The innermost function the arm sits in.
    /// 该臂所在的最内层函数。
    pub function: String,
    /// The enum named before `::`.
    /// `::` 之前点名的枚举。
    pub enum_name: String,
    /// The variant named after `::`.
    /// `::` 之后点名的变体。
    pub variant: String,
    /// 1-based line of the arm's pattern, which is the line the row reports.
    /// 以 1 起始的臂模式行号——也就是那一行报告的行号。
    pub line: u32,
}

/// One `enum` declaration, with the two facts that decide whether its arms are judged.
/// 一个 `enum` 声明，以及决定它的臂是否被判定的两个事实。
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct EnumDeclaration {
    /// The enum's name.
    /// 枚举的名字。
    pub name: String,
    /// 1-based line of the declaration.
    /// 以 1 起始的声明行号。
    pub line: u32,
    /// Whether an arm matching this enum's variants may be reported at all: the declaration must
    /// not begin with `pub`, and it may carry only value-free derives.
    /// 匹配该枚举变体的臂是否**可以**被报告：声明不得以 `pub` 开头，且只能带值无关派生。
    pub judged: bool,
}

/// One place the source writes the path `Enum::Variant` in expression position.
/// 源码在表达式位置写出路径 `Enum::Variant` 的一处。
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct VariantPath {
    /// The enum named before `::`.
    /// `::` 之前点名的枚举。
    pub enum_name: String,
    /// The variant named after `::`.
    /// `::` 之后点名的变体。
    pub variant: String,
    /// 1-based line of the path.
    /// 以 1 起始的该路径行号。
    pub line: u32,
}

/// Everything a branch-level read learns from one file.
/// 一次分支级读取从单个文件学到的一切。
#[derive(Clone, Debug, Default)]
pub struct BranchFacts {
    /// The literal-`false` guards, in source order.
    /// 字面量 `false` 守卫，按源码顺序。
    pub false_guards: Vec<ConstantFalseGuard>,
    /// The `match` arms whose pattern is a qualified variant path, in source order.
    /// 模式是限定变体路径的 `match` 臂，按源码顺序。
    pub matched_variants: Vec<MatchedVariant>,
    /// The `enum` declarations, in source order.
    /// `enum` 声明，按源码顺序。
    pub enums: Vec<EnumDeclaration>,
    /// The `Enum::Variant` paths written in expression position, in source order.
    /// 写在表达式位置的 `Enum::Variant` 路径，按源码顺序。
    pub variant_paths: Vec<VariantPath>,
    /// The names this file imports from **outside its own crate** — every identifier of a `use`
    /// declaration whose first segment is not `crate`, `self` or `super`.
    /// 本文件从**自己 crate 之外**引入的名字——首段不是 `crate`、`self` 或 `super` 的 `use` 声明里
    /// 的每个标识符。
    ///
    /// A two-segment pattern `Band::v` names whatever `Band` means *in this file*, and a `use`
    /// whose root is another crate is the one spelling that says so. The join skips an arm whose
    /// enum name appears here, because a miss is the cheap direction and reporting a row about
    /// this tree's `Band` while the arm actually named someone else's would be the other one.
    /// 两段模式 `Band::v` 点名的是 `Band` **在这个文件里**的含义，而根是另一个 crate 的 `use` 正是
    /// 说出这件事的那种拼法。连接会跳过枚举名出现在这里的臂，因为漏报是便宜的方向，而在一臂其实点名的是
    /// 别人的 `Band` 时报一条关于本树的 `Band` 的行，是另一个方向。
    pub foreign_imports: Vec<String>,
}

/// Read one file's branch-level facts.
/// 读取一个文件的分支级事实。
///
/// The arms come from one linear pass over the masked text, and the enclosing function of each arm
/// is looked up in [`super::function_symbols`] — the same discovery the call graph uses, so no second
/// notion of "what is a function" is introduced here.
/// 臂来自对掩码文本的一次线性扫描，而每个臂所属的函数在 [`super::function_symbols`] 里查——也就是调用图用的
/// 那份函数发现，因此这里没有引入第二套"什么算函数"。
pub fn branch_facts(source: &str) -> BranchFacts {
    super::source_facts(source).branches
}

pub(super) fn from_symbols(masked: &str, functions: &[SourceFunction]) -> BranchFacts {
    let starts = line_starts(masked);
    let mut facts = BranchFacts::default();
    let mut patterns: Vec<(usize, usize)> = Vec::new();
    scan_arms(masked, &starts, functions, &mut facts, &mut patterns);
    scan_paths(masked, &starts, &patterns, &mut facts);
    scan_enums(masked, &mut facts);
    scan_foreign_imports(masked, &starts, &mut facts);
    facts
}

/// The byte offset every 1-based line starts at.
/// 每一条以 1 起始的行开始的字节偏移。
fn line_starts(masked: &str) -> Vec<usize> {
    let mut starts = vec![0usize];
    for (at, byte) in masked.bytes().enumerate() {
        if byte == b'\n' {
            starts.push(at + 1);
        }
    }
    starts
}

/// The 1-based line `offset` falls on.
/// `offset` 落在哪一条以 1 起始的行上。
fn line_at(starts: &[usize], offset: usize) -> u32 {
    match starts.binary_search(&offset) {
        Ok(index) => index as u32 + 1,
        Err(index) => index as u32,
    }
}

/// The text of one 1-based line, without its terminator.
/// 某一条以 1 起始的行的文本（不含行终止符）。
fn line_text<'a>(masked: &'a str, starts: &[usize], line: u32) -> Option<&'a str> {
    let index = (line as usize).checked_sub(1)?;
    let from = *starts.get(index)?;
    let to = starts.get(index + 1).copied().unwrap_or(masked.len());
    Some(masked[from..to].trim_end_matches('\n'))
}

/// `text` after the whole word `word`, when `text` starts with it.
/// 当 `text` 以完整单词 `word` 开头时，返回 `word` 之后的文本。
fn strip_word<'a>(text: &'a str, word: &str) -> Option<&'a str> {
    let after = text.strip_prefix(word)?;
    if after.starts_with(is_ident_continue) {
        return None;
    }
    Some(after)
}

/// The leading identifier of `text` and the text after it.
/// `text` 开头的标识符与它之后的文本。
fn leading_identifier(text: &str) -> Option<(String, &str)> {
    let mut characters = text.char_indices();
    let (_, first) = characters.next()?;
    if !is_ident_start(first) {
        return None;
    }
    let mut end = first.len_utf8();
    for (index, character) in characters {
        if !is_ident_continue(character) {
            break;
        }
        end = index + character.len_utf8();
    }
    Some((text[..end].to_owned(), &text[end..]))
}

/// What one open brace region is.
/// 一个已打开的花括号区域是什么。
enum Region {
    /// An ordinary block.
    /// 普通块。
    Block,
    /// A `match` body, in one of its two phases.
    /// `match` 体，处于它的两个阶段之一。
    Match(MatchPhase),
    /// The block body of one `match` arm: closing it ends that arm.
    /// 某个 `match` 臂的块体：闭合它就结束该臂。
    Arm,
}

/// Which half of a `match` body the scan is in.
/// 扫描处于 `match` 体的哪一半。
#[derive(Clone, Copy, PartialEq, Eq)]
enum MatchPhase {
    /// Reading a pattern, up to its `=>`.
    /// 正在读模式，直到它的 `=>`。
    Pattern,
    /// Reading an arm's body, up to its `,` or its closing brace.
    /// 正在读某个臂的体，直到它的 `,` 或闭合花括号。
    Body,
}

/// Walk `masked` once, recording every arm a rule can decide and every pattern span.
/// 在 `masked` 上走一趟，记录每一条可判定的臂与每一个模式区间。
///
/// The scan never skips a region, so a `match` nested inside an arm's body is read exactly like a
/// top-level one; `Region::Arm` is what tells an arm's own block apart from a block inside it.
/// 这次扫描不跳过任何区域，因此嵌在臂体里的 `match` 与顶层的 `match` 被同样对待；`Region::Arm` 正是
/// 用来把"臂自己的块"与"它内部的块"区分开的东西。
fn scan_arms(
    masked: &str,
    starts: &[usize],
    functions: &[SourceFunction],
    facts: &mut BranchFacts,
    patterns: &mut Vec<(usize, usize)>,
) {
    let bytes = masked.as_bytes();
    let mut regions: Vec<Region> = Vec::new();
    let mut pending_match = false;
    let mut pending_arm = false;
    let mut pattern_start = 0usize;
    let mut index = 0usize;
    while index < bytes.len() {
        let Some(character) = masked[index..].chars().next() else {
            break;
        };
        match character {
            '{' => {
                if pending_match {
                    pending_match = false;
                    regions.push(Region::Match(MatchPhase::Pattern));
                    pattern_start = index + 1;
                } else if pending_arm {
                    pending_arm = false;
                    regions.push(Region::Arm);
                } else {
                    regions.push(Region::Block);
                }
                index += 1;
            }
            '}' => {
                match regions.pop() {
                    // A `match` closed while a pattern was open: the pattern ends here (an empty
                    // match, or a malformed one). Recording the span only widens the exclusion.
                    // 在模式还开着的时候 `match` 闭合了：模式到此为止（空的或畸形的 match）。记下这个
                    // 区间只会让排除范围更宽。
                    Some(Region::Match(MatchPhase::Pattern)) => {
                        patterns.push((pattern_start, index))
                    }
                    Some(Region::Arm) => {
                        if let Some(Region::Match(phase)) = regions.last_mut() {
                            *phase = MatchPhase::Pattern;
                        }
                        pattern_start = index + 1;
                    }
                    _ => {}
                }
                index += 1;
            }
            ',' => {
                if let Some(Region::Match(phase)) = regions.last_mut()
                    && *phase == MatchPhase::Body
                {
                    *phase = MatchPhase::Pattern;
                    pattern_start = index + 1;
                }
                index += 1;
            }
            '=' if bytes.get(index + 1) == Some(&b'>') => {
                if let Some(Region::Match(phase)) = regions.last_mut()
                    && *phase == MatchPhase::Pattern
                {
                    record_arm(masked, starts, functions, pattern_start, index, facts);
                    patterns.push((pattern_start, index));
                    *phase = MatchPhase::Body;
                    pending_arm = true;
                }
                index += 2;
            }
            _ => {
                if pending_arm && !character.is_whitespace() {
                    pending_arm = false;
                }
                if !is_ident_start(character) {
                    index += character.len_utf8();
                    continue;
                }
                let start = index;
                index += character.len_utf8();
                while let Some(next) = masked[index..].chars().next() {
                    if !is_ident_continue(next) {
                        break;
                    }
                    index += next.len_utf8();
                }
                match &masked[start..index] {
                    "match" => pending_match = true,
                    "if" => record_false_guard(masked, starts, functions, start, index, facts),
                    _ => {}
                }
            }
        }
    }
}

/// Record the arm whose pattern spans `[from, to)`, one entry per qualified variant path it names.
/// 记录模式区间为 `[from, to)` 的那个臂，它点名的每一条限定变体路径各一条记录。
///
/// One arm can name several variants (an or-pattern, or a struct pattern with a nested one), and
/// they all carry the **arm's** line: the caller deduplicates rows by (file, line), so an arm is
/// reported at most once no matter how many of its variants are decided.
/// 一个臂可以点名多个变体（或模式，或带嵌套模式的结构体模式），它们全都带**该臂**的行号：调用方按
/// (文件, 行) 去重，因此一个臂无论有多少个变体被判定，最多只报一次。
fn record_arm(
    masked: &str,
    starts: &[usize],
    functions: &[SourceFunction],
    from: usize,
    to: usize,
    facts: &mut BranchFacts,
) {
    let paths = pattern_paths(masked, from, to);
    // The arm's line is the line of its **first variant path**, not of the first byte in the span:
    // a comment above the pattern is masked to spaces but its `//` is still a byte, and reporting
    // the comment's line would send a reader to prose instead of to the arm.
    // 该臂的行号是它**第一条变体路径**的行号，而不是区间里第一个字节的行号：模式上方的注释被掩码成
    // 空格，但它的 `//` 仍是字节，而报注释那一行会把读者送到散文而不是那个臂上。
    let Some((first, _, _)) = paths.first() else {
        return;
    };
    let line = line_at(starts, *first);
    let function = enclosing_function(functions, line).unwrap_or_default();
    for (_, enum_name, variant) in paths {
        facts.matched_variants.push(MatchedVariant {
            function: function.clone(),
            enum_name,
            variant,
            line,
        });
    }
}

/// The qualified `Enum::Variant` paths one arm's pattern names, up to its ` if ` guard.
/// 某个臂的模式点名的限定 `Enum::Variant` 路径，到它的 ` if ` 守卫为止。
///
/// Wildcards, bindings, literals and ranges name no path, so they contribute nothing; a pair whose
/// first segment is a module (`crate::Band`) contributes the pair the rule reads — the last two
/// segments — because that is the spelling the join is keyed on.
/// 通配符、绑定、字面量与区间不点名任何路径，因此什么也不贡献；首段是模块的路径对
/// （`crate::Band`）贡献规则所读的那一对——最后两段——因为连接正是按那种拼法建键的。
fn pattern_paths(masked: &str, from: usize, to: usize) -> Vec<(usize, String, String)> {
    let mut found = Vec::new();
    let mut cursor = from;
    let mut depth = 0usize;
    while cursor < to {
        let Some(character) = masked[cursor..].chars().next() else {
            break;
        };
        match character {
            '(' | '[' | '{' => {
                depth += 1;
                cursor += 1;
            }
            ')' | ']' | '}' => {
                depth = depth.saturating_sub(1);
                cursor += 1;
            }
            _ if is_ident_start(character) => {
                let start = cursor;
                cursor += character.len_utf8();
                while let Some(next) = masked[cursor..].chars().next() {
                    if !is_ident_continue(next) {
                        break;
                    }
                    cursor += next.len_utf8();
                }
                let word = &masked[start..cursor];
                // The guard is not part of the pattern, and a binding inside it is not a variant.
                // 守卫不是模式的一部分，而守卫里的绑定不是变体。
                if depth == 0 && word == "if" {
                    break;
                }
                let Some(name_at) = after_double_colon(masked, cursor, to) else {
                    continue;
                };
                if let Some((variant, rest)) = leading_identifier(&masked[name_at..to]) {
                    found.push((start, word.to_owned(), variant));
                    // `rest` is what is left of the pattern, so this is the byte just after the
                    // variant — not `rest.len()`, which is a length and not an offset.
                    // `rest` 是模式剩下的部分，因此这里得到的是变体之后那个字节——不是
                    // `rest.len()`，那是长度而不是偏移。
                    cursor = to - rest.len();
                }
            }
            _ => cursor += character.len_utf8(),
        }
    }
    found
}

/// Where the identifier after a `::` starts, when `text` carries that `::` before `end`.
/// 当 `text` 在 `end` 之前带着那个 `::` 时，`::` 之后那个标识符的起始位置。
fn after_double_colon(masked: &str, from: usize, end: usize) -> Option<usize> {
    let mut cursor = from;
    while cursor < end && masked[cursor..].chars().next()?.is_whitespace() {
        cursor += 1;
    }
    if !masked[cursor..].starts_with("::") {
        return None;
    }
    let mut at = cursor + 2;
    while at < end && masked[at..].chars().next()?.is_whitespace() {
        at += 1;
    }
    Some(at)
}

/// Record the `if` at `start` when its condition is exactly the literal `false`.
/// 当 `start` 处的 `if` 的条件恰好是字面量 `false` 时记录它。
fn record_false_guard(
    masked: &str,
    starts: &[usize],
    functions: &[SourceFunction],
    start: usize,
    after_if: usize,
    facts: &mut BranchFacts,
) {
    let Some((from, to)) = condition_before_block(masked, after_if) else {
        return;
    };
    // Whitespace is not meaning here: `if false {` and `if  false  {` are the same guard. Anything
    // richer (`!true`, `1 == 2`, a `const` bool, `cfg!(…)`) needs a constant evaluator and is
    // therefore not in the subset at all.
    // 空白在这里没有含义：`if false {` 与 `if  false  {` 是同一个守卫。更复杂的东西（`!true`、
    // `1 == 2`、`const` 布尔、`cfg!(…)`）需要常量求值，因此根本不在子集里。
    let condition = masked[from..to].split_whitespace().collect::<String>();
    if condition != "false" {
        return;
    }
    let line = line_at(starts, start);
    facts.false_guards.push(ConstantFalseGuard {
        function: enclosing_function(functions, line).unwrap_or_default(),
        line,
    });
}

/// The byte range between `from` and the `{` that opens the block, when there is one.
/// `from` 与打开块的 `{` 之间的字节区间（存在时）。
fn condition_before_block(masked: &str, from: usize) -> Option<(usize, usize)> {
    let bytes = masked.as_bytes();
    let mut cursor = from;
    let mut depth = 0usize;
    while cursor < bytes.len() {
        match bytes[cursor] {
            b'(' | b'[' => depth += 1,
            b')' | b']' => depth = depth.saturating_sub(1),
            b'{' if depth == 0 => return Some((from, cursor)),
            b';' if depth == 0 => return None,
            _ => {}
        }
        cursor += 1;
    }
    None
}

/// The innermost function whose range contains `line`.
/// 范围包含 `line` 的最内层函数。
fn enclosing_function(functions: &[SourceFunction], line: u32) -> Option<String> {
    functions
        .iter()
        .filter(|function| function.line <= line && line <= function.end_line)
        .max_by_key(|function| function.line)
        .map(|function| function.name.clone())
}

/// Record every `Enum::Variant` path written outside a match-arm pattern and outside a `use` line.
/// 记录每一个写在 match 臂模式之外、且不在 `use` 行上的 `Enum::Variant` 路径。
fn scan_paths(
    masked: &str,
    starts: &[usize],
    patterns: &[(usize, usize)],
    facts: &mut BranchFacts,
) {
    let bytes = masked.as_bytes();
    let mut index = 0usize;
    while index < bytes.len() {
        let Some(character) = masked[index..].chars().next() else {
            break;
        };
        if !is_ident_start(character) {
            index += character.len_utf8();
            continue;
        }
        let mut segments: Vec<(usize, usize)> = Vec::new();
        let mut cursor = index;
        let mut consumed = false;
        // One path segment per turn: an identifier, then `::` and the next identifier while the
        // spelling continues. The `filter` is what makes the identifier the loop's own condition
        // instead of a `break` inside it.
        // 每一轮读一段路径：一个标识符，然后在拼写继续时读 `::` 与下一个标识符。那个 `filter` 正是让
        // 标识符成为循环自身条件、而不是循环里一个 `break` 的东西。
        while let Some(next) = masked[cursor..]
            .chars()
            .next()
            .filter(|character| is_ident_start(*character))
        {
            let start = cursor;
            cursor += next.len_utf8();
            while let Some(inner) = masked[cursor..].chars().next() {
                if !is_ident_continue(inner) {
                    break;
                }
                cursor += inner.len_utf8();
            }
            segments.push((start, cursor));
            consumed = true;
            let mut after = cursor;
            while after < bytes.len() && bytes[after].is_ascii_whitespace() {
                after += 1;
            }
            // `A::<T>::B` is a turbofish, not a path pair this read understands, and `A::b` is the
            // shape it does. Stopping here leaves the turbofish out entirely.
            // `A::<T>::B` 是 turbofish，不是这次读取理解的路径对；`A::b` 才是。在这里停下会让
            // turbofish 整个不被读到。
            if bytes.get(after) != Some(&b':') || bytes.get(after + 1) != Some(&b':') {
                break;
            }
            let mut next_at = after + 2;
            while next_at < bytes.len() && bytes[next_at].is_ascii_whitespace() {
                next_at += 1;
            }
            cursor = next_at;
        }
        index = if consumed {
            cursor
        } else {
            index + character.len_utf8()
        };
        for pair in segments.windows(2) {
            let (enum_start, enum_end) = pair[0];
            let (_, variant_end) = pair[1];
            if patterns
                .iter()
                .any(|(from, to)| enum_start >= *from && variant_end <= *to)
            {
                continue;
            }
            let line = line_at(starts, enum_start);
            if is_use_line(masked, starts, line) {
                continue;
            }
            facts.variant_paths.push(VariantPath {
                enum_name: masked[enum_start..enum_end].to_owned(),
                variant: masked[pair[1].0..variant_end].to_owned(),
                line,
            });
        }
    }
}

/// The path text of line `line`, when that line is a `use` declaration.
/// 第 `line` 行是 `use` 声明时，它的路径文本。
///
/// The visibility qualifier is consumed first, because `pub use other::Band;` imports `Band` just
/// as plainly as the private spelling does — and a reader that stopped at `pub` would miss it.
/// 先吃掉可见性限定，因为 `pub use other::Band;` 与私有拼法一样实打实地引入 `Band`——而在 `pub` 处
/// 停下的读取会漏掉它。
fn use_path_line<'a>(masked: &'a str, starts: &[usize], line: u32) -> Option<&'a str> {
    let text = line_text(masked, starts, line)?;
    let mut rest = text.trim_start();
    if let Some(after) = strip_word(rest, "pub") {
        rest = after.trim_start();
        if let Some(inside) = rest.strip_prefix('(') {
            rest = inside.split_once(')')?.1.trim_start();
        }
    }
    let after = strip_word(rest, "use")?;
    Some(after.trim_start())
}

/// Whether line `line` is a `use` declaration.
/// 第 `line` 行是不是一条 `use` 声明。
fn is_use_line(masked: &str, starts: &[usize], line: u32) -> bool {
    use_path_line(masked, starts, line).is_some()
}

/// Record the names this file imports from outside its own crate.
/// 记录本文件从自己 crate 之外引入的名字。
fn scan_foreign_imports(masked: &str, starts: &[usize], facts: &mut BranchFacts) {
    let mut names: Vec<String> = Vec::new();
    for (index, _) in masked.split('\n').enumerate() {
        let Some(rest) = use_path_line(masked, starts, index as u32 + 1) else {
            continue;
        };
        for (position, (start, end)) in identifiers(rest).into_iter().enumerate() {
            let name = &rest[start..end];
            // `crate`/`self`/`super` roots stay inside this crate, so a name they carry is this
            // tree's own spelling and the join may judge it.
            // `crate`/`self`/`super` 作根仍在本 crate 内，因此它们带的这个名字是本树自己的拼法，
            // 连接可以判它。
            if position == 0 && matches!(name, "crate" | "self" | "super") {
                break;
            }
            if !names.iter().any(|seen| seen == name) {
                names.push(name.to_owned());
            }
        }
    }
    facts.foreign_imports = names;
}

/// Every identifier in `text`, as byte ranges.
/// `text` 里的每个标识符，按字节区间给出。
fn identifiers(text: &str) -> Vec<(usize, usize)> {
    let mut found = Vec::new();
    let mut cursor = 0usize;
    while cursor < text.len() {
        let Some(character) = text[cursor..].chars().next() else {
            break;
        };
        if !is_ident_start(character) {
            cursor += character.len_utf8();
            continue;
        }
        let start = cursor;
        cursor += character.len_utf8();
        while let Some(next) = text[cursor..].chars().next() {
            if !is_ident_continue(next) {
                break;
            }
            cursor += next.len_utf8();
        }
        found.push((start, cursor));
    }
    found
}

/// Record every `enum` declaration and whether its arms may be judged.
/// 记录每一个 `enum` 声明以及它的臂是否可以判定。
fn scan_enums(masked: &str, facts: &mut BranchFacts) {
    let lines = masked.split('\n').collect::<Vec<_>>();
    for (index, line) in lines.iter().enumerate() {
        let Some((name, is_public)) = enum_declaration_on_line(line) else {
            continue;
        };
        facts.enums.push(EnumDeclaration {
            name,
            line: index as u32 + 1,
            judged: !is_public && attributes_are_value_free(&lines, index),
        });
    }
}

/// The enum one masked line declares, as `(name, begins_with_pub)`.
/// 一条已屏蔽的行声明的枚举，形如 `(名字, 是否以 pub 开头)`。
fn enum_declaration_on_line(line: &str) -> Option<(String, bool)> {
    let mut rest = line.trim_start();
    let is_public = if let Some(after) = strip_word(rest, "pub") {
        rest = after.trim_start();
        if let Some(inside) = rest.strip_prefix('(') {
            rest = inside.split_once(')')?.1.trim_start();
        }
        true
    } else {
        false
    };
    let (keyword, after) = leading_identifier(rest)?;
    if keyword != "enum" {
        return None;
    }
    let (name, _) = leading_identifier(after.trim_start())?;
    Some((name, is_public))
}

/// Whether every attribute directly above line `enum_line` is a value-free derive.
/// `enum_line` 正上方的每一条属性是否都是值无关派生。
///
/// Attributes are walked upwards until an ordinary line ends the run, and anything this function
/// cannot read — a multi-line attribute, an unknown derive, `#[non_exhaustive]` — makes the
/// declaration unjudged rather than guessed at.
/// 属性向上走，直到一条普通行结束这一段；而任何本函数读不出来的东西——多行属性、不认识的派生、
/// `#[non_exhaustive]`——都会让这个声明不被判定，而不是被猜。
fn attributes_are_value_free(lines: &[&str], enum_line: usize) -> bool {
    let mut index = enum_line;
    while index > 0 {
        index -= 1;
        let trimmed = lines[index].trim();
        if trimmed.is_empty() {
            continue;
        }
        let Some(rest) = trimmed.strip_prefix("#[") else {
            return true;
        };
        let Some(inner) = rest.strip_suffix(']') else {
            return false;
        };
        if !is_value_free_derive(inner.trim()) {
            return false;
        }
    }
    true
}

/// Whether one attribute body is a `derive(…)` whose every name cannot build a value.
/// 一条属性体是否是 `derive(…)`，且其中每个名字都无法造出值。
fn is_value_free_derive(attribute: &str) -> bool {
    let Some(rest) = strip_word(attribute, "derive") else {
        return false;
    };
    let Some(inner) = rest
        .trim_start()
        .strip_prefix('(')
        .and_then(|rest| rest.strip_suffix(')'))
    else {
        return false;
    };
    let names = inner.split(',').collect::<Vec<_>>();
    !names.is_empty()
        && names
            .iter()
            .all(|name| VALUE_FREE_DERIVES.contains(&name.trim()))
}

#[cfg(test)]
#[path = "branches_tests.rs"]
mod branches_tests;
