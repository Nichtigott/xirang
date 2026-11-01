//! Pure Rust-source lexer shared by search, callgraph, and indexing tools.
//! 供搜索、调用图与索引工具共用的纯 Rust 源码词法器。
//!
//! Every function here is a text transformation only: callers own file I/O.
//! 这里的所有函数只做文本变换，文件 I/O 由调用方负责。
//!
//! Function discovery lives here; `calls` scans call sites inside an extracted
//! body, `branches` reads the arms a static branch read can decide, `items` owns the
//! declaration vocabulary (`item_symbols`), and `walk` owns the recursive source
//! traversal.
//! 函数发现位于本页；`calls` 扫描已提取函数体内的调用点，`branches` 读取静态分支读取
//! 能判定的臂，`items` 拥有声明词表（`item_symbols`），`walk` 拥有递归源码遍历。
#[path = "lex.rs"]
pub(crate) mod lex;

#[path = "calls.rs"]
mod calls;
pub use calls::*;
#[path = "branches.rs"]
mod branches;
pub use branches::*;
#[path = "walk.rs"]
mod walk;
pub use walk::*;
#[path = "items.rs"]
mod items;
pub use items::*;

/// One Rust function discovered in a source file.
/// 在源码文件中发现的一个 Rust 函数。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceFunction {
    /// Function name as written after the `fn` token.
    /// `fn` token 之后书写的函数名。
    pub name: String,
    /// Source text from the start of the line through the opening brace, trimmed.
    /// 从行首到左花括号的源码文本，已去除首尾空白。
    pub signature: String,
    /// Source text between the opening and closing braces.
    /// 左、右花括号之间的源码文本。
    pub body: String,
    /// 1-based line of the opening `fn` token.
    /// 以 1 起始的 `fn` 起始行行号。
    pub line: u32,
    /// 1-based line of the closing brace.
    /// 以 1 起始的右花括号所在行行号。
    pub end_line: u32,
}

/// Find the 0-based inclusive line range of a function by name.
/// 按名称查找函数的 0 起始闭区间行范围。
pub fn function_source_range(lines: &[&str], name: &str) -> Option<(usize, usize)> {
    // The start test runs on masked text and requires the name to be a whole
    // identifier. On the raw line, `// fn ghost() {` started a range for `ghost` that
    // closed on the next real function, and `fn renew(` matched the name `new`.
    // 起点判断跑在屏蔽后的文本上，并要求名字是完整标识符。按原始行时，`// fn ghost() {`
    // 会为 `ghost` 开出一个到下一个真实函数才闭合的范围，而 `fn renew(` 会匹配名字 `new`。
    // Mask the whole source once rather than one line at a time: a `fn` or a brace inside
    // a multi-line string, a raw string or a block comment is prose, and a per-line mask
    // cannot see where such a construct starts — it reported `fn ghost()` inside a string
    // and `kind: Ghost` inside a `/* … */` fence. `mask_non_code` keeps every line break,
    // so the masked text splits into exactly the lines the caller handed over.
    // 对整份源码掩码一次，而不是逐行：多行字符串、raw 字符串或块注释里的 `fn` 或花括号是散文，
    // 而逐行掩码看不见这类构造从哪里开始——它把字符串里的 `fn ghost()` 和 `/* … */` 里的
    // `kind: Ghost` 都报了出来。`mask_non_code` 保留每一个换行，因此掩码后的文本切出来的正是
    // 调用方交进来的那些行。
    let source = lines.join("\n");
    let masked = mask_non_code(&source);
    let masked_lines: Vec<&str> = masked.split('\n').collect();
    let start = masked_lines.iter().position(|line| {
        let mut from = 0usize;
        while let Some(offset) = line[from..].find("fn ") {
            let at = from + offset;
            let after = line[at + "fn ".len()..].trim_start();
            if let Some(rest) = after.strip_prefix(name)
                && rest.trim_start().starts_with('(')
            {
                return true;
            }
            from = at + "fn ".len();
        }
        false
    })?;
    let mut depth = 0usize;
    let mut opened = false;
    for (index, line) in masked_lines.iter().enumerate().skip(start) {
        // Braces inside a string or a comment are not code. Counting them on the
        // raw line let `let s = "{";` open a range that closed on a later `}` —
        // and with no closing brace at all the old fallback claimed a one-line
        // function. The masking below is the same rule the function index uses.
        // 字符串或注释里的花括号不是代码。按原始行计数会让 `let s = "{";` 打开一个在更后面的
        // `}` 处闭合的范围——而完全没有闭合花括号时，旧的兜底会声称这是个单行函数。下面的
        // 屏蔽与函数索引用的是同一条规则。
        for character in line.chars() {
            match character {
                '{' => {
                    depth += 1;
                    opened = true;
                }
                '}' if opened => depth = depth.saturating_sub(1),
                _ => {}
            }
        }
        if opened && depth == 0 {
            return Some((start, index));
        }
    }
    // Unbalanced at the end of `lines`: the function does not close inside what
    // the caller handed over. `Some((start, start))` reported that as a one-line
    // function, which is the opposite of what the caller needs to know.
    // 在 `lines` 末尾仍未配平：该函数没有在调用方给出的范围内闭合。过去用
    // `Some((start, start))` 把它报成单行函数，而这与调用方需要知道的事实相反。
    None
}

/// Advance `cursor` past one UTF-8 character, when there is one.
/// 若存在，把 `cursor` 推进一个 UTF-8 字符。
fn skip_one_character(bytes: &[u8], cursor: &mut usize) {
    if *cursor < bytes.len() {
        *cursor += 1;
        while bytes.get(*cursor).is_some_and(|byte| byte & 0xC0 == 0x80) {
            *cursor += 1;
        }
    }
}

/// Index function bodies without treating comments, strings, or macro text as Rust.
/// 扫描函数体时屏蔽注释、字符串和宏文本，避免把它们误认成 Rust 函数。
pub fn function_symbols(source: &str) -> Vec<SourceFunction> {
    let masked = mask_non_code(source);
    functions_from_masked(source, &masked)
}

/// Version content-addressed syntax caches by the exact pure parser implementation.
/// 用纯解析器的精确实现为内容寻址语法缓存版本化。
pub fn parser_fingerprint() -> String {
    crate::sha256_hex(
        concat!(
            include_str!("source.rs"),
            include_str!("calls.rs"),
            include_str!("lex.rs")
        )
        .as_bytes(),
    )
}

/// Functions and branch facts produced by one shared lexical reading.
/// 一次共用词法读取产出的函数与分支事实。
pub struct SourceFacts {
    /// Function definitions in source order.
    /// 按源码顺序排列的函数定义。
    pub functions: Vec<SourceFunction>,
    /// Static branch facts over those same bytes.
    /// 基于同一批字节的静态分支事实。
    pub branches: BranchFacts,
}

/// Share masking and function discovery when both functions and branches are needed.
/// 同时需要函数和分支时，共用掩码与函数发现。
pub fn source_facts(source: &str) -> SourceFacts {
    let masked = mask_non_code(source);
    let functions = functions_from_masked(source, &masked);
    let branches = branches::from_symbols(&masked, &functions);
    SourceFacts {
        functions,
        branches,
    }
}

fn functions_from_masked(source: &str, masked: &str) -> Vec<SourceFunction> {
    let bytes = masked.as_bytes();
    let mut result = Vec::new();
    // Line numbers come from one forward scan. Both offsets this reports are
    // non-decreasing — the loop resumes at the end of the function it just took —
    // so counting from the last offset instead of from zero makes the whole pass
    // linear; counting from zero per function made it quadratic in the number of
    // functions, which is the shape MCP's index walks.
    // 行号来自一次前向扫描。它报告的两个偏移都是非递减的——循环从刚取下的函数末尾继续——
    // 因此从上次的偏移继续数、而不是每次从 0 数，使整趟是线性的；过去每个函数都从 0 数，
    // 于是复杂度与函数个数相乘，而 MCP 的索引正是按那种形状遍历的。
    //
    // The `target < counted_to` arm is a correctness fallback, not the path any
    // caller takes today: a future caller that asks about an earlier offset gets
    // the right answer at the old cost instead of a wrong one.
    // `target < counted_to` 那一支是正确性兜底，不是今天任何调用方会走的路径：将来若有
    // 调用方问一个更早的偏移，它会以旧代价拿到正确答案，而不是拿到一个错答案。
    let mut counted_to = 0usize;
    let mut counted_lines = 1u32;
    let line_at = |target: usize, counted_to: &mut usize, counted_lines: &mut u32| -> u32 {
        if target < *counted_to {
            return source[..target]
                .bytes()
                .filter(|byte| *byte == b'\n')
                .count() as u32
                + 1;
        }
        *counted_lines += source[*counted_to..target]
            .bytes()
            .filter(|byte| *byte == b'\n')
            .count() as u32;
        *counted_to = target;
        *counted_lines
    };
    let mut index = 0usize;
    while index < bytes.len() {
        // Identifiers are Unicode. Testing one byte at a time indexed `héllo` as `h`, and
        // that truncation reached the MCP index, Studio and the build's function manifest.
        // 标识符是 Unicode 的。一次只看一个字节会把 `héllo` 索引成 `h`，而这个截断会一路传到
        // MCP 索引、Studio 与构建产出的函数清单。
        let Some(character) = masked[index..].chars().next() else {
            break;
        };
        if !is_ident_start(character) {
            index += character.len_utf8();
            continue;
        }
        let token_start = index;
        index += character.len_utf8();
        while let Some(next) = masked[index..].chars().next() {
            if !is_ident_continue(next) {
                break;
            }
            index += next.len_utf8();
        }
        if &masked[token_start..index] != "fn" {
            continue;
        }
        let mut name_start = index;
        while let Some(next) = masked[name_start..].chars().next() {
            if !next.is_whitespace() {
                break;
            }
            name_start += next.len_utf8();
        }
        let Some(first) = masked[name_start..].chars().next() else {
            continue;
        };
        if !is_ident_start(first) {
            continue;
        }
        let mut name_end = name_start + first.len_utf8();
        while let Some(next) = masked[name_end..].chars().next() {
            if !is_ident_continue(next) {
                break;
            }
            name_end += next.len_utf8();
        }
        let name = masked[name_start..name_end].to_owned();
        let mut open = name_end;
        let mut angle_depth = 0usize;
        while open < bytes.len() {
            match bytes[open] {
                b'<' => angle_depth += 1,
                b'>' if angle_depth > 0 => angle_depth -= 1,
                b'{' if angle_depth == 0 => break,
                b';' if angle_depth == 0 => break,
                _ => {}
            }
            open += 1;
        }
        if open >= bytes.len() || bytes[open] != b'{' {
            continue;
        }
        let mut depth = 1usize;
        let mut close = open + 1;
        while close < bytes.len() && depth > 0 {
            match bytes[close] {
                b'{' => depth += 1,
                b'}' => depth = depth.saturating_sub(1),
                _ => {}
            }
            close += 1;
        }
        if depth != 0 {
            continue;
        }
        let line_start = source[..token_start].rfind('\n').map_or(0, |line| line + 1);
        let line = line_at(token_start, &mut counted_to, &mut counted_lines);
        let end_line = line_at(close, &mut counted_to, &mut counted_lines);
        let signature = source[line_start..open].trim().to_owned();
        result.push(SourceFunction {
            name,
            signature,
            body: source[open + 1..close - 1].to_owned(),
            line,
            end_line,
        });
        // Resume **inside** the body rather than after it: a declaration nested in a function
        // is still a declaration, and stopping at the closing brace made every nested helper
        // invisible — a chain that ran through one lost that node, and the MCP answered
        // `no static function match` for a name the call site carried. Descending costs a
        // second pass over the nested text, which is why the line counter keeps its fallback
        // arm for offsets that go backwards.
        // 从**体内**继续，而不是从体后继续：嵌套在函数里的声明仍是声明，而在闭合花括号处停下会让每个
        // 嵌套辅助函数不可见——经过它的调用链会丢掉那个节点，MCP 也就对调用点携带的名字答出
        // `no static function match`。向下走会对嵌套文本多扫一遍，这也是行计数器为回退的偏移保留兜底
        // 那一支的原因。
        index = open + 1;
    }
    result
}
/// The leading identifier of `text` and the text after it.
/// `text` 开头的标识符与它之后的文本。
///
/// `items` and `branches` both read one identifier off a masked line before deciding what it means;
/// the lexing rule belongs beside the character tests it uses, so the two cannot disagree about what
/// counts as an identifier.
/// `items` 与 `branches` 都要在判定含义之前先从掩码行上读出一个标识符；词法规则住在它所用的字符判定旁边，
/// 这样两者对"什么算标识符"不会给出不同答案。
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

fn is_ident_start(character: char) -> bool {
    character.is_alphabetic() || character == '_'
}

fn is_ident_continue(character: char) -> bool {
    character.is_alphanumeric() || character == '_'
}

/// Replace the contents of comments, string literals and character literals with spaces,
/// preserving the byte offsets of everything else.
/// 把注释、字符串字面量与字符字面量的内容替换为空格，其余部分的字节偏移保持不变。
///
/// The workspace's one text rule for "what is code": the source scanners use it so
/// a `fn` inside a comment or a brace inside a string is not read as Rust, and the
/// `conventions` gates use it so a workspace test's *fixture string* mentioning
/// `include!`/`std::fs` is not read as a violation. Macro bodies are deliberately
/// left alone — the macro name and its delimiter are code, and a gate that looks
/// for a macro invocation has to see them.
/// 本工作区关于"什么算代码"的唯一文本规则：源码扫描器用它，使注释里的 `fn` 或字符串里的花括号不被
/// 读成 Rust；`conventions` 的门禁也用它，使工作区测试里**夹具字符串**中提到的
/// `include!`/`std::fs` 不被读成违规。宏内容有意保留——宏名与它的定界符是代码，而查找宏调用的
/// 门禁必须看见它们。
pub fn mask_non_code(source: &str) -> String {
    mask(source, true)
}

/// Replace quoted literals with spaces while preserving offsets, keeping comments.
/// 用空格替换引号字面量，同时保留原始偏移量，并保留注释。
///
/// The same scan as [`mask_non_code`], with the comment writes disabled: what
/// survives is code *plus comment text*, which is what a rule about what a
/// comment says needs. A gate that read the raw lines instead was fooled by a
/// test fixture whose string literal carried a `///` line — the fixture's text
/// looked like a doc comment and was reported as one.
/// 与 [`mask_non_code`] 同一次扫描，只是关掉写注释的空白：留下的是代码**加上注释文本**，
/// 而一条关于注释说了什么的规则正需要这个。直接读原始行的门禁曾被一个测试夹具骗过：那个字符串
/// 字面量里带着一行 `///`，夹具文本看起来像文档注释，于是被当成文档注释报了出来。
pub fn mask_literals(source: &str) -> String {
    mask(source, false)
}

/// The shared scan behind [`mask_non_code`] and [`mask_literals`].
/// [`mask_non_code`] 与 [`mask_literals`] 共用的扫描。
///
/// `blank_comments` decides whether comment text is replaced. Literals are always
/// replaced, because a quote inside a comment must not be read as the start of a
/// string and a quote inside a string must not be read as code.
/// `blank_comments` 决定注释文本是否被替换。字面量总会被替换，因为注释里的引号不能被读成字符串的
/// 开始，字符串里的引号也不能被读成代码。
fn mask(source: &str, blank_comments: bool) -> String {
    let bytes = source.as_bytes();
    let mut masked = bytes.to_vec();
    let mut index = 0usize;
    while index < bytes.len() {
        if bytes[index] == b'/' && bytes.get(index + 1) == Some(&b'/') {
            index += 2;
            while index < bytes.len() && bytes[index] != b'\n' {
                if blank_comments {
                    masked[index] = b' ';
                }
                index += 1;
            }
            continue;
        }
        if bytes[index] == b'/' && bytes.get(index + 1) == Some(&b'*') {
            if blank_comments {
                masked[index] = b' ';
                if index + 1 < bytes.len() {
                    masked[index + 1] = b' ';
                }
            }
            index += 2;
            let mut depth = 1usize;
            while index < bytes.len() && depth > 0 {
                if bytes[index] == b'/' && bytes.get(index + 1) == Some(&b'*') {
                    depth += 1;
                    if blank_comments {
                        masked[index] = b' ';
                        masked[index + 1] = b' ';
                    }
                    index += 2;
                } else if bytes[index] == b'*' && bytes.get(index + 1) == Some(&b'/') {
                    depth = depth.saturating_sub(1);
                    if blank_comments {
                        masked[index] = b' ';
                        masked[index + 1] = b' ';
                    }
                    index += 2;
                } else {
                    if blank_comments && bytes[index] != b'\n' {
                        masked[index] = b' ';
                    }
                    index += 1;
                }
            }
            continue;
        }
        // A raw string is not closed by the quote that follows its opening `#`s, so the
        // ordinary branch below stopped at the first interior `"` and the rest of the
        // literal — braces, `fn`, call sites — was read as code. `r#"…"#` is ordinary
        // Rust, and JSON payloads inside it are full of interior quotes.
        // 原始字符串不由其开头 `#` 之后的那个引号闭合，因此下面的普通分支会在第一个内部 `"`
        // 处停下，而字面量剩下的部分——花括号、`fn`、调用点——会被读成代码。`r#"…"#` 是普通
        // Rust，而它里面的 JSON 载荷满是内部引号。
        if let Some(end) = lex::raw_string_end(bytes, index) {
            for byte in &mut masked[index..end] {
                if *byte != b'\n' {
                    *byte = b' ';
                }
            }
            index = end;
            continue;
        }
        if bytes[index] == b'"' {
            let quote = bytes[index];
            masked[index] = b' ';
            index += 1;
            while index < bytes.len() {
                let escaped = bytes[index] == b'\\';
                if bytes[index] != b'\n' {
                    masked[index] = b' ';
                }
                index += 1;
                if escaped && index < bytes.len() {
                    if bytes[index] != b'\n' {
                        masked[index] = b' ';
                    }
                    index += 1;
                } else if bytes[index - 1] == quote {
                    break;
                }
            }
            continue;
        }
        // A `'` opens a character literal only when that literal closes. A
        // lifetime (`'a`), a label (`'outer`) or the apostrophe of `&'static` has
        // no closing quote, and treating it as one masked everything up to the
        // next apostrophe — a function's `{` included — so every function with a
        // lifetime parameter vanished from the index, and every call after a
        // `&'static` was missed. The rule here is the lexer's: `'\…'`, or one
        // character followed by `'`.
        // `'` 只有在字符字面量闭合时才是它的起始。生命周期（`'a`）、标签（`'outer`）或
        // `&'static` 的撇号没有闭合引号，把它当成引号会一路遮到下一个撇号——包括函数的 `{`
        // ——于是每个带生命周期参数的函数都从索引里消失，`&'static` 之后的调用也全部漏掉。
        // 这里的规则与词法器相同：`'\…'`，或一个字符后紧跟 `'`。
        if bytes[index] == b'\'' {
            let mut cursor = index + 1;
            if bytes.get(cursor) == Some(&b'\\') {
                cursor += 1;
                match bytes.get(cursor) {
                    // `\u{…}`: the escape is delimited by braces.
                    Some(&b'u') if bytes.get(cursor + 1) == Some(&b'{') => {
                        cursor += 2;
                        while cursor < bytes.len() && bytes[cursor] != b'}' {
                            cursor += 1;
                        }
                        cursor = (cursor + 1).min(bytes.len());
                    }
                    // `\xNN`: exactly two hex digits.
                    Some(&b'x') => cursor = (cursor + 3).min(bytes.len()),
                    // `\n`, `\'`, `\\`, an escaped multi-byte character: one.
                    _ => skip_one_character(bytes, &mut cursor),
                }
            } else {
                skip_one_character(bytes, &mut cursor);
            }
            if bytes.get(cursor) == Some(&b'\'') {
                for byte in &mut masked[index..=cursor] {
                    if *byte != b'\n' {
                        *byte = b' ';
                    }
                }
                index = cursor + 1;
            } else {
                // A lifetime or a label: an apostrophe is not a token either
                // scanner looks for, so masking it alone is enough.
                // 生命周期或标签：撇号不是两个扫描器要找的 token，只遮掉它本身即可。
                masked[index] = b' ';
                index += 1;
            }
            continue;
        }
        index += 1;
    }
    // Keep the mask. The scan only writes ASCII spaces, and only at character
    // boundaries, so this text is valid UTF-8 and the `Err` arm is unreachable today —
    // but returning the *unmasked* source on failure is the worst possible direction:
    // comments, strings and raw strings would all be read as code and the caller would
    // get invented functions and calls instead of a visible failure (audit `KRN-K-16`).
    // 保留掩码。这次扫描只写 ASCII 空格、且只在字符边界上写，因此这段文本是合法 UTF-8、
    // `Err` 支今天不可达——但失败时回退到**未掩码**源码是最坏的方向：注释、字符串与 raw string
    // 都会被当成代码，调用方拿到凭空造出的函数与调用，而不是一次可见的失败（审计 `KRN-K-16`）。
    masked_text(masked)
}

/// The masked bytes as text, keeping the mask even when the bytes are not valid UTF-8.
/// 掩码后的字节转成文本；即使字节不是合法 UTF-8 也保留掩码。
///
/// The caller above cannot produce invalid UTF-8 (it only writes ASCII spaces at character
/// boundaries), which is exactly why the failure direction had to be decided rather than
/// inherited: this function never returns the unmasked source, so the unreachable arm
/// cannot turn into invented code. It takes only the bytes, so a test can feed it a
/// sequence the scanner cannot produce and pin that direction (audit `KRN-K-16`).
/// 上面的调用方不可能产出非法 UTF-8（它只在字符边界写 ASCII 空格），这正是失败方向必须被**决定**
/// 而不是被继承的原因：本函数绝不返回未掩码的源码，因此不可达的那一支不会变成凭空造出的代码。它只
/// 接收字节，所以测试可以喂给它扫描器造不出的字节序列，把方向钉住（审计 `KRN-K-16`）。
fn masked_text(masked: Vec<u8>) -> String {
    match String::from_utf8(masked) {
        Ok(text) => text,
        Err(error) => String::from_utf8_lossy(error.as_bytes()).into_owned(),
    }
}

/// Collect the deduplicated `kind:` values used in registration declarations.
/// 收集注册声明中出现过的 `kind:` 取值，去重并排序。
///
/// Only a `kind:` inside a registration macro body counts. The rule used to be "this line
/// contains `kind:`", which read ordinary code as a declaration — `let kind: String = value;`
/// contributed a kind named `String` — and missed a declaration whose `kind:` and value sat
/// on different lines. Both directions are invisible in the output the source queries print
/// (audit `KRN-K-17`), so the macro body is what bounds the scan; the price is stated here:
/// this stays a text scan with no dependency on the `syntax` feature, so it recognizes the
/// macro by name (any `…object!` invocation, the same rule the face parser uses) rather than
/// by parsing the declaration.
/// 只有注册宏体内的 `kind:` 才算。规则过去是"本行含 `kind:`"，于是普通代码被读成声明——
/// `let kind: String = value;` 贡献了一个叫 `String` 的 kind——而 `kind:` 与取值分行的声明被漏掉。
/// 两个方向在源码查询打印的输出里都看不出来（审计 `KRN-K-17`），因此以宏体为界；代价写在这里：
/// 这仍是纯文本扫描、不依赖 `syntax` 特性，因此它按名字认宏（任何 `…object!` 调用，与注册面解析器
/// 同一条规则），而不是去解析声明。
pub fn registration_kinds(source: &str) -> Vec<String> {
    // Mask once for the same reason the function range does: a `kind:` inside a
    // multi-line string or a block comment is prose, and a per-line mask reported the name
    // it mentioned.
    // 与函数范围同理，掩码一次：多行字符串或块注释里的 `kind:` 是散文，而逐行掩码会把其中提到的
    // 那个名字报出来。
    let masked_source = mask_non_code(source);
    let lines = masked_source.split('\n').collect::<Vec<_>>();
    let mut kinds = Vec::new();
    let mut depth = 0i32;
    let mut in_declaration = false;
    let mut body_opened = false;
    for (index, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if !in_declaration && trimmed.contains("object!") {
            in_declaration = true;
        }
        if in_declaration {
            // The value may sit on the next line (`kind:` alone on its own line, which the
            // per-line rule skipped): take the next non-empty line that does not itself
            // open a field. A field start is an identifier the vocabulary knows followed by
            // a colon, which is the same test the token reader uses.
            // 取值可能在下一行（`kind:` 单独占一行，旧的逐行规则会跳过）：取下一非空、且自身不开启
            // 字段的行。字段起点是词表认识的标识符后跟冒号，与 token 读取器用的是同一条判据。
            let remainder = match trimmed.split_once("kind:") {
                Some((_, remainder)) if !remainder.trim().is_empty() => Some(remainder.trim()),
                Some(_) => lines[index + 1..]
                    .iter()
                    .map(|line| line.trim())
                    .find(|line| !line.is_empty() && !starts_a_known_field(line)),
                None => None,
            };
            if let Some(kind) = remainder {
                let kind = kind
                    .trim()
                    .split(|character: char| {
                        character == ',' || character == '}' || character.is_whitespace()
                    })
                    .next()
                    .unwrap_or_default();
                // A kind is a Rust type name, so it may be non-ASCII like any identifier.
                // kind 是 Rust 类型名，因此与任何标识符一样可以是非 ASCII 的。
                if !kind.is_empty()
                    && kind
                        .chars()
                        .all(|character| character.is_alphanumeric() || character == '_')
                {
                    kinds.push(kind.to_owned());
                }
            }
        }
        let opens = line.matches('{').count();
        let closes = line.matches('}').count();
        depth += opens as i32 - closes as i32;
        if depth > 0 {
            body_opened = true;
        }
        // A body that never opened (the `{` is on a later line) keeps the declaration
        // open; a body that opened and closed on this line ends it here.
        // 尚未打开体的声明（`{` 在后面的行）保持打开；在本行打开又闭合的体在此结束。
        if depth <= 0 && (body_opened || opens > 0) {
            in_declaration = false;
            body_opened = false;
        }
    }
    kinds.sort();
    kinds.dedup();
    kinds
}

/// Whether `line` opens a `name:` field the face vocabulary knows.
/// `line` 是否开启一个词表认识的 `name:` 字段。
///
/// Used to decide whether the line after a `kind:` that carries no value *is* that value
/// or the next field: `preset:` opens a field, `Widget,` does not.
/// 用来判断"不带取值的 `kind:` 之后那一行"是它的取值还是下一个字段：`preset:` 开启字段，
/// `Widget,` 不是。
fn starts_a_known_field(line: &str) -> bool {
    let Some((name, _)) = line.split_once(':') else {
        return false;
    };
    let name = name.trim();
    !name.is_empty()
        && name
            .chars()
            .all(|character| character.is_alphanumeric() || character == '_')
        && crate::registry_core::declaration::FACE_FIELD_ORDER.contains(&name)
}
#[cfg(test)]
#[path = "source_tests.rs"]
mod source_tests;
