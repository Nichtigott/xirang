//! The host's crate-shape declaration, read as **text** by the build (audit `M7`, P3.1).
//! 宿主的 crate 形状声明，由构建当**文本**读（审计 `M7`，P3.1）。
//!
//! The host writes `add_crates.rs` at its package root as ordinary Rust, so the compiler checks it
//! (a subtree that does not exist is an unresolved path, and the error points at that line). The
//! build cannot link the host, so it reads the same file and asks the kernel's one rule whether the
//! shape holds together — and then answers the question the declaration is *for*: which faces each
//! crate would own.
//! 宿主在包根把 `add_crates.rs` 写成普通 Rust，因此编译器会查它（不存在的子树就是解析不了的路径，而错误
//! 指到那一行）。构建无法链接宿主，于是它读同一个文件、用内核那唯一一条规则问"这份形状成立吗"，然后回答
//! 声明**真正为了**的那个问题：每个 crate 会拥有哪些面。
//!
//! The reader is **narrow and refuses**: it reads exactly the shape `Crate::named(…).at(&[…::SUBTREE])`
//! inside a `Shape { … }` literal, and anything else is a named error rather than a guess. A reader
//! that silently accepted a second spelling would decide a package's crate layout by accident.
//! 这个读取器**窄而会拒绝**：它只读 `Shape { … }` 字面量里 `Crate::named(…).at(&[…::SUBTREE])` 这一种形状，
//! 其它一律是点名错误而不是猜测。一个静默接受第二种拼写的读取器，会让一个包的 crate 布局被意外决定。

use std::fmt::Write as _;
use std::path::Path;
#[cfg(any(feature = "cli", feature = "mcp", feature = "studio"))]
use std::path::PathBuf;

use xirang_kernel::identity::NodeId;
use xirang_kernel::lexicon;
use xirang_kernel::registry_core::{DeclaredCrate, validate_shape};

use super::discovery_cache::write_if_changed;
use super::face_view::PruningRow;
use super::static_plan::source_module_path;

/// What one host declared, with the digest of the file it came from.
/// 一个宿主声明了什么，以及它来自哪个文件的摘要。
#[derive(Debug)]
pub(crate) struct ShapeDeclaration {
    /// The prefix every published crate name is built from.
    /// 每个发布包名所依据的前缀。
    pub(crate) package_prefix: String,
    /// One entry per declared crate: its name, and the `::`-separated module paths it claims.
    /// 每个已声明 crate 一项：它的名字，以及它认领的 `::` 分隔模块路径。
    pub(crate) crates: Vec<(String, Vec<String>)>,
    /// The identity derived from the declaration file's bytes, which is what the lock records.
    /// 从声明文件字节推导出的身份，也就是锁记录的东西。
    pub(crate) digest: String,
}

impl ShapeDeclaration {
    /// Every subtree the declaration hands to another crate, as `::`-separated module paths.
    /// 声明交给另一个 crate 的每一棵子树，写成 `::` 分隔的模块路径。
    ///
    /// These are what the host's own generated tree must **not** emit: a subtree that belongs to
    /// another crate and is compiled here as well would be one face compiled twice, with two
    /// registries that can disagree (audit `M7`, P3.2).
    /// 这些正是宿主自己的生成树**不得**发射的东西：属于另一个 crate、却在这里也编译一遍的子树，等于同一个面
    /// 编译两遍，还带两个可能互相分歧的注册机（审计 `M7`，P3.2）。
    pub(crate) fn cut_subtrees(&self) -> Vec<String> {
        self.crates
            .iter()
            .flat_map(|(_, subtrees)| subtrees.iter().cloned())
            .collect()
    }
}

/// The declaration text from `start` to its **balanced** closing delimiter, or why it does not balance.
/// 从 `start` 到**配平**的收尾定界符为止的声明文本，或它为何不配平。
///
/// Quoted text is skipped, because a `"…"` in a declaration may contain braces (`Crate::named("a}b")`)
/// and counting them would refuse a file that is fine. Raw strings and byte strings are not special-cased:
/// the declaration grammar this reader accepts has neither, and guessing at spellings outside it is how a
/// reader starts answering from text it did not really parse.
/// 引号里的文本被跳过，因为声明里的一处 `"…"` 可能含花括号（`Crate::named("a}b")`），把那些也数进去会拒绝
/// 一份没问题的文件。原始字符串与字节串不做特判：本读者接受的声明语法两者都没有，而靠猜语法之外的拼法，正是一个
/// 读者开始从它并没真正解析过的文本作答的方式。
fn balanced_body(text: &str, start: usize) -> Result<&str, String> {
    let bytes = text.as_bytes();
    let mut cursor = start;
    while cursor < bytes.len() && !matches!(bytes[cursor], b'{' | b'(' | b'[') {
        cursor += 1;
    }
    if bytes.get(cursor).is_none() {
        return Err("it opens a `Shape` and never opens a body".to_owned());
    }
    let close = closing_bracket(text, cursor)?;
    Ok(&text[start..=close])
}

/// The index of the delimiter that closes the one at `open`, by **depth**.
/// 与 `open` 处那个定界符相配的收尾定界符下标，按**深度**判定。
///
/// One scanner, because two writers in this module need the same answer and the first of them did not
/// ask: `declare` looked for the **first** `]` after `&[`, and the first `]` in a list of
/// `Crate::named("…").at(&[…::SUBTREE])` entries is the **first entry's own** — so the new crate was
/// inserted inside the previous one. Measured, the file it wrote was
/// `&[Crate::named("dash-board").at(&[crate::board::SUBTREE` + newline + `Crate::named("extra").at(…),` +
/// `]),`, whose brackets all balance and which no longer parses (audit `M7`, §M7.61).
/// 一个扫描器，因为这个模块里的两个写入方需要同一个答案，而第一个没有问它：`declare` 找的是 `&[` 之后
/// **第一个** `]`，而在一串 `Crate::named("…").at(&[…::SUBTREE])` 条目里，第一个 `]` 是**第一条 entry
/// 自己的**——于是新 crate 被插进了上一条里面。实测，它写出的文件是
/// `&[Crate::named("dash-board").at(&[crate::board::SUBTREE` + 换行 + `Crate::named("extra").at(…),` +
/// `]),`，每个括号都配平，而它不再能解析（审计 `M7`，§M7.61）。
///
/// Quoted text is skipped, because a `"…"` in a declaration may contain brackets (`Crate::named("a]b")`)
/// and counting them would mis-place the insertion in a file that is fine.
/// 引号里的文本被跳过，因为声明里的一处 `"…"` 可能含括号（`Crate::named("a]b")`），把那些数进去会让一次插入在
/// 一份没问题的文件里落错位置。
fn closing_bracket(text: &str, open: usize) -> Result<usize, String> {
    let bytes = text.as_bytes();
    let closers = |opening: u8| match opening {
        b'(' => b')',
        b'[' => b']',
        _ => b'}',
    };
    let Some(&opening) = bytes.get(open) else {
        return Err("there is nothing to close".to_owned());
    };
    if !matches!(opening, b'{' | b'(' | b'[') {
        return Err(format!("`{}` opens nothing", opening as char));
    }
    let mut stack: Vec<u8> = Vec::new();
    let mut quoted = false;
    let mut escaped = false;
    let mut cursor = open;
    while cursor < bytes.len() {
        let byte = bytes[cursor];
        if quoted {
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == b'"' {
                quoted = false;
            }
            cursor += 1;
            continue;
        }
        match byte {
            b'"' => quoted = true,
            b'{' | b'(' | b'[' => stack.push(byte),
            b'}' | b')' | b']' => {
                let Some(expected) = stack.pop() else {
                    return Err(format!(
                        "its brackets do not balance: `{}` closes nothing",
                        byte as char
                    ));
                };
                if closers(expected) != byte {
                    return Err(format!(
                        "its brackets do not balance: `{}` is closed by `{}`",
                        expected as char, byte as char
                    ));
                }
                if stack.is_empty() {
                    return Ok(cursor);
                }
            }
            _ => {}
        }
        cursor += 1;
    }
    let unclosed = stack.iter().map(|byte| *byte as char).collect::<String>();
    Err(format!(
        "its brackets do not balance: the declaration leaves `{unclosed}` open, so nothing here can be \
         read as a declaration"
    ))
}

/// Read the shape `package_root/add_crates.rs` declares, or `None` when the host has none.
/// 读取 `package_root/add_crates.rs` 声明的形状；宿主没有这个文件时是 `None`。
///
/// No file means "this package is one crate", which is what every host was before the declaration
/// existed — so the absence is an answer, not a missing input.
/// 没有文件意为"这个包就是一个 crate"，也就是声明存在之前每个宿主的样子——因此"没有"是一个答案，而不是
/// 缺了输入。
pub(crate) fn read_shape_declaration(
    package_root: &Path,
) -> Result<Option<ShapeDeclaration>, String> {
    let path = package_root.join(lexicon::ADD_CRATES_FILE);
    let Ok(text) = std::fs::read_to_string(&path) else {
        return Ok(None);
    };
    let digest = NodeId::from_bytes(text.as_bytes()).to_string();
    // The **last** `Shape {`, not the first: a declaration written as a function has two of them
    // (`-> Shape {` and the literal), and `split(...).nth(1)` would answer with the empty text
    // between them — measured, that is exactly how the function form read as "no package_prefix".
    // 取**最后**一个 `Shape {` 而不是第一个：写成函数的声明有两个（`-> Shape {` 与字面量），而
    // `split(...).nth(1)` 会答出两者之间的空文本——实测：函数形态就是这样被读成"没有 package_prefix"的。
    // Two spellings, one meaning: a struct literal (`Shape { … }`) or the constructor the template
    // writes (`Shape::of(…)`). Take the **last** opening of either — a declaration written as a
    // function has two `Shape {` (`-> Shape {` and the literal), and answering with the text between
    // them is how the function form once read as "no package_prefix".
    // 两种拼写、一个意思：结构体字面量（`Shape { … }`）或模板写的构造器（`Shape::of(…)`）。取两者中
    // **最后**一个开头——写成函数的声明有两个 `Shape {`（`-> Shape {` 与字面量），而答出两者之间的文本，
    // 正是函数形态曾被读成"没有 package_prefix"的原因。
    let start = ["Shape {", "Shape::of("]
        .iter()
        .filter_map(|opening| text.rfind(opening))
        .max()
        .ok_or_else(|| {
            unreadable(
                &path,
                "it has no `Shape { … }` literal and no `Shape::of(…)`",
            )
        })?;
    // The body runs to the declaration's **balanced** closing delimiter, and a body that never balances
    // is refused rather than read from. Taking the text up to the first `}` made the reader accept a
    // declaration a writer had broken in half: measured, `crates --declare --write` used to insert a new
    // `Crate::named(…)` inside the previous entry, leaving an inner `&[` that never closes, and `check`
    // then reported `ok` on a file that is not valid Rust — the file sits in the package root, no crate
    // graph contains it, and nothing compiled it (audit `M7`, §M7.61). A reader that answers from a file
    // it cannot parse is the second half of that chain; this is the first half of closing it.
    // 主体一直延伸到这条声明**配平**的收尾定界符，而永不配平的主体被拒绝而不是拿来读。取到第一个 `}` 为止
    // 让读者接受了被写入方切成两半的声明：实测，`crates --declare --write` 过去会把新的 `Crate::named(…)`
    // 插进上一条 entry 里，留下一个永不闭合的内层 `&[`，而 `check` 随后在一份不是合法 Rust 的文件上报 `ok`
    // ——那份文件住在包根、没有任何 crate 图包含它、也没有东西编译它（审计 `M7`，§M7.61）。一个从自己解析不了
    // 的文件作答的读者是那条链的后半段；这里是把它合上的前半段。
    let body = balanced_body(&text, start).map_err(|reason| unreadable(&path, &reason))?;
    // Balanced brackets are not enough, and the measured counter-example is the file a broken writer
    // leaves: `crates --declare --write` used to insert the new `Crate::named(…)` **inside** the previous
    // entry, and the result balances every bracket while holding two adjacent expressions in one array
    // (`&[crate::board::SUBTREE Crate::named("extra").at(…)…]`). A bracket stack cannot see that — it
    // counts a shape, not a grammar — and `syn` can, which is why the reader parses the body instead of
    // measuring it (audit `M7`, §M7.61).
    // 括号配平是不够的，而实测的反例正是写入方留下的那份文件：`crates --declare --write` 过去会把新的
    // `Crate::named(…)` 插进上一条 entry **里面**，结果每一个括号都配平，而一个数组里放着两个相邻的表达式
    // （`&[crate::board::SUBTREE Crate::named("extra").at(…)…]`）。括号栈看不见这件事——它数的是形状，不是
    // 语法——而 `syn` 看得见，这就是读者**解析**主体而不是**度量**它的原因（审计 `M7`，§M7.61）。
    if let Err(error) = syn::parse_str::<syn::Expr>(body) {
        return Err(unreadable(
            &path,
            &format!(
                "it does not read as one Rust expression ({error}); a declaration whose brackets balance \
                 but whose entries do not parse is a file a writer half-finished, and reading a shape out \
                 of it would describe a partition nobody wrote"
            ),
        ));
    }
    let package_prefix = body
        .split("package_prefix")
        .nth(1)
        .or_else(|| body.split("Shape::of(").nth(1))
        .and_then(|rest| rest.split('"').nth(1))
        .ok_or_else(|| {
            unreadable(
                &path,
                "it declares no `package_prefix: \"…\"`, which every published crate name is \
                 built from",
            )
        })?
        .to_owned();
    let mut crates = Vec::new();
    for chunk in body.split("Crate::named(").skip(1) {
        let name = chunk
            .split('"')
            .nth(1)
            .ok_or_else(|| unreadable(&path, "a `Crate::named(…)` carries no name string"))?
            .to_owned();
        let list = chunk
            .split("(&[")
            .nth(1)
            .and_then(|rest| rest.split(']').next())
            .ok_or_else(|| {
                unreadable(
                    &path,
                    &format!("`{name}` has no `.at(&[…::SUBTREE])`, so it claims nothing"),
                )
            })?;
        let mut subtrees = Vec::new();
        for entry in list
            .split(',')
            .map(str::trim)
            .filter(|entry| !entry.is_empty())
        {
            // The declaration names a **marker**: `<path>::SUBTREE`. Anything else is a spelling
            // this reader does not know, and guessing what it meant is how a shape gets decided by
            // accident.
            // 声明点名的是一个**标记**：`<path>::SUBTREE`。别的拼写是本读取器不认识的，而猜它的意思正是
            // 形状被意外决定的方式。
            // The marker is `SUBTREE`, and it exists exactly on the nodes that **have children** —
            // the renderer gives a node with children an inline module of the build's own, and mounts
            // a node without children straight at its file. So a leaf cannot be named at all, which is
            // the rule rather than a gap: a crate is a subtree, and one object in its own crate buys
            // nothing.
            // 标记是 `SUBTREE`，而它恰好只存在于**有子节点**的节点上——渲染器给有子节点的节点搭一个构建自己的
            // 内联模块，而没有子节点的节点直接挂到它的文件上。因此叶子根本点不了名，这是**规则**而不是缺口：
            // crate 是一棵子树，而一个对象单独成 crate 什么也换不来。
            let module = entry
                .strip_suffix("::SUBTREE")
                .ok_or_else(|| {
                    // Every subtree path ends in `::SUBTREE`, so when one does not, the fix is usually
                    // a spelling away — say it, instead of making the reader re-derive the grammar.
                    // 每一棵子树路径都以 `::SUBTREE` 收尾；不合规时通常只差几个字母——直接说出来，
                    // 不要让读者自己去推语法。
                    let head = entry.rsplit_once("::").map(|(head, _)| head).unwrap_or(entry);
                    let why = format!(
                        "`{name}` names `{entry}`, which is not a `…::SUBTREE` marker \
                         (every subtree path ends in `::SUBTREE`, so `{head}::SUBTREE` is the spelling \
                         this one is missing)"
                    );
                    unreadable(&path, &why)
                })?
                .trim()
                .trim_start_matches("crate::")
                .to_owned();
            subtrees.push(module);
        }
        crates.push((name, subtrees));
    }
    if crates.is_empty() {
        return Err(unreadable(
            &path,
            "it declares no `Crate::named(…)`, so it asks for no crate",
        ));
    }
    let modules: Vec<Vec<&str>> = crates
        .iter()
        .map(|(_, subtrees)| subtrees.iter().map(String::as_str).collect())
        .collect();
    let declared: Vec<DeclaredCrate<'_>> = crates
        .iter()
        .zip(&modules)
        .map(|((name, _), subtrees)| DeclaredCrate { name, subtrees })
        .collect();
    validate_shape(&package_prefix, &declared).map_err(|refusal| unreadable(&path, &refusal))?;
    Ok(Some(ShapeDeclaration {
        package_prefix,
        crates,
        digest,
    }))
}

/// Write the lock that records what the declaration asked for, and which faces each crate would own.
/// 写下锁：记录声明要求了什么，以及每个 crate 会拥有哪些面。
///
/// The lock is the shape's **history**: the declaration itself is what a person edits, and this file
/// is what a commit records — so "which commit was which shape" is a git question about one small
/// file rather than a re-derivation. Faces are resolved with the tree's own rule
/// (`source_module_path`), never by string surgery on file paths.
/// 锁是形状的**历史**：声明是人在编辑的东西，而这个文件是提交记录下来的东西——于是"哪个提交是哪种形状"是
/// 关于一个小文件的 git 问题，而不是一次重新推导。面的归属用树自己的规则（`source_module_path`）解析，
/// 绝不对文件路径做字符串手术。
pub(crate) fn write_shape_lock(
    out_dir: &Path,
    declaration: &ShapeDeclaration,
    rows: &[PruningRow],
) -> Result<(), String> {
    let mut owner: Vec<(String, Vec<String>)> = declaration
        .crates
        .iter()
        .map(|(name, _)| (name.clone(), Vec::new()))
        .collect();
    let mut unclaimed = 0usize;
    let mut seen: std::collections::BTreeSet<xirang_kernel::identity::NodeId> =
        std::collections::BTreeSet::new();
    for row in rows {
        if !seen.insert(row.id) {
            continue;
        }
        let module = source_module_path(&row.source);
        let mut claimed = false;
        for (position, (_, subtrees)) in declaration.crates.iter().enumerate() {
            if subtrees
                .iter()
                .any(|subtree| module == *subtree || module.starts_with(&format!("{subtree}::")))
            {
                owner[position].1.push(row.source.clone());
                claimed = true;
                break;
            }
        }
        if !claimed {
            unclaimed += 1;
        }
    }
    let mut output = format!(
        "# add-crates\t{}\n# declaration\t{}\npackage_prefix\t{}\n",
        lexicon::ADD_CRATES_MARKER,
        declaration.digest,
        declaration.package_prefix
    );
    for (position, (name, subtrees)) in declaration.crates.iter().enumerate() {
        let mut faces = owner[position].1.clone();
        faces.sort();
        let face_set = NodeId::from_bytes(faces.join("\n").as_bytes()).to_string();
        writeln!(
            output,
            "crate\t{name}\tsubtrees={}\tfaces={}\tface_set={face_set}",
            subtrees.join(","),
            faces.len()
        )
        .unwrap();
    }
    writeln!(output, "host\tfaces={unclaimed}").unwrap();
    write_if_changed(&out_dir.join(lexicon::ADD_CRATES_LOCK_FILE), &output)
}

/// The pipeline's call: read the declaration, and publish the lock when the host declared one.
/// 管线的调用：读声明，宿主声明了形状时发布锁。
///
/// A host with no declaration is not an error: it is the one-crate package every host was before
/// this file existed, and the pipeline does nothing at all for it.
/// 没有声明的宿主不是错误：它就是这份文件存在之前每个宿主的样子——一个 crate 的包，而管线对它什么都不做。
/// Validate a declaration the caller has **already read**, and record its lock.
/// 校验调用方**已经读过**的声明，并写下它的锁。
///
/// Split from the reader because a run reads the declaration once and needs it twice: the render
/// skips the subtrees it hands away, and the release checks validate the claims against the rows.
/// Reading twice would be two chances to disagree about one file.
/// 与读者拆开，是因为一次运行读一次、却要用两次：渲染要跳过它交出去的子树，发布校验要拿认领与行对账。读两遍
/// 等于给同一个文件两次分歧的机会。
pub(crate) fn check_shape(
    declaration: &ShapeDeclaration,
    out_dir: &Path,
    rows: &[PruningRow],
) -> Result<(), String> {
    let mut claims = 0usize;
    let mut satisfied = 0usize;
    for (name, subtrees) in &declaration.crates {
        for subtree in subtrees {
            claims += 1;
            // `?` rather than a `match` that returns the same error: clippy's `question_mark` denies the
            // long form under `-D warnings`, and it fires on the CI toolchain (1.99) while the local one
            // (1.96) stays quiet — which is how this red survived every local gate.
            // 用 `?` 而不是"原样返回同一个错误"的 `match`：clippy 的 `question_mark` 在 `-D warnings`
            // 下拒绝长写法，而它在 CI 的工具链（1.99）上会响、本地那条（1.96）不响——这道红就是这样
            // 熬过了每一次本地门禁。
            let resolved = claim_has_a_subtree(name, subtree, rows)?;
            satisfied += usize::from(resolved);
        }
    }
    // A declaration whose claims **all** point at nothing in this tree is refused here, and here only:
    // `claim_has_a_subtree` deliberately skips a claim whose neighbourhood this run cannot see, or every
    // fragment build would fail on the host's other claims (measured). The complement of that rule is
    // this one: when not a single claim resolves, the tree is not "a neighbourhood we cannot see" — the
    // declaration names nothing here, and a crate built from it would carry no sources and answer every
    // later question wrongly.
    // 一份声明，若它的认领**全都**在本树里点不到东西，就在这里拒绝，而且只在这里：`claim_has_a_subtree`
    // 有意跳过"本次看不见邻域"的认领，否则每个片段的构建都会在宿主的其余认领上失败（实测过）。这条规则是
    // 那条的补集：当**一条都解析不到**时，树并不是"看不见的邻域"——而是这份声明在这里什么都没点到，由它构建的
    // crate 会没有源码，并在之后每个问题上都答错。
    if claims > 0 && satisfied == 0 {
        let named: Vec<&str> = declaration
            .crates
            .iter()
            .flat_map(|(_, subtrees)| subtrees.iter().map(String::as_str))
            .collect();
        return Err(format!(
            "add_crates: `{}` names no module this tree has, so the crate would be empty: no faces \
             below it in this host. Run `xirang check` and read what it says about the tree, or fix \
             the path",
            if named.is_empty() {
                "?".to_owned()
            } else {
                named.join("`, `")
            },
        ));
    }
    write_shape_lock(out_dir, declaration, rows)
}

/// Refuse a claim that does not name a subtree, and say which node does.
/// 拒绝一条没有点名子树的认领，并说出哪个节点才是。
///
/// The compiler already refuses a leaf (there is no `SUBTREE` marker on one), so this is the second
/// reader saying the same rule in the case the compiler cannot see: a declaration the build reads as
/// text but that no crate mounts. It reads the tree, so it can name the way forward — which node
/// contains the claimed one — instead of leaving the author to guess.
/// 编译器已经会拒绝叶子（叶子上没有 `SUBTREE` 标记），因此这是第二个读者在编译器看不见的情形里说同一条规则：
/// 构建当文本读、却没有 crate 挂载的声明。它读得到树，因此能点名出路——哪个节点包含被认领的那个——而不是让
/// 作者去猜。
fn claim_has_a_subtree(name: &str, subtree: &str, rows: &[PruningRow]) -> Result<bool, String> {
    // Judge only the claims this run's tree is *about*. A partition's fragment carries one subtree
    // plus its ancestor shells, so the declaration's other claims name modules that are simply absent
    // here — measured: validating them per claim failed every multi-claim partition (`control` "has
    // no faces below it" while building the `panel::gauge` crate). A claim whose neighbourhood this
    // run cannot see at all is left to the run that owns it; the whole-declaration check upstream
    // still refuses a declaration that names nothing in this tree.
    // 只审"本次树的邻域里的"认领。划分出来的片段只带一棵子树加它的祖先壳，因此声明里其余认领点名的模块在这里
    // 根本不存在——实测：逐条校验让每一个多认领的划分都失败（构建 `panel::gauge` 那个 crate 时，`control`
    // 被报"下面没有面"）。本次运行完全看不见其邻域的认领，留给拥有它的那次运行；上游那条"整份声明"的检查
    // 仍然会拒绝一棵都点不到模块的声明。
    let below = |module: &str| -> usize {
        rows.iter()
            .filter(|row| {
                let face = source_module_path(&row.source);
                face.starts_with(&format!("{module}::"))
            })
            .count()
    };
    if below(subtree) > 0 {
        return Ok(true);
    }
    let is_a_face = rows
        .iter()
        .any(|row| source_module_path(&row.source) == subtree);
    let containing = subtree.rsplit_once("::").map(|(parent, _)| parent);
    if is_a_face {
        return Err(match containing {
            Some(parent) => format!(
                "add_crates: `{name}` names `{subtree}`, which is a face and not a subtree; a crate \
                 is a subtree — name `{parent}` (or another node that contains it) instead"
            ),
            None => format!(
                "add_crates: `{name}` names `{subtree}`, which is a face and not a subtree; a crate \
                 is a subtree, and this one is a single object at the top of the tree"
            ),
        });
    }
    // Neither below this claim nor a face in this tree: with only this run's rows there is no way to
    // tell "another crate's claim, absent here" from "a module that is really empty", and the first is
    // the common case in a fragment (measured: refusing it broke every multi-claim partition). The
    // plan, which can read the module tree, is where an empty crate belongs; this reader keeps only
    // the refusals its rows can prove — "below it" and "it is itself a face here".
    // 既不在它下面、也不是本树里的一个面：只有本次运行的行表时，无法区分"别的 crate 的认领、在这里缺席"与
    // "一个真的空模块"，而前者在片段里是常态（实测：拒绝它让每一个多认领的划分都失败）。能读模块树的规划器才是
    // "空 crate"该被判的地方；本读取器只保留行表能证明的拒绝——"它下面有面"与"它本身就是这里的一个面"。
    Ok(false)
}

/// The one refusal spelling for a declaration the build cannot read.
/// 构建读不了的声明，其唯一的拒绝拼法。
///
/// It says who checks what, because the answer is not the obvious one: **XiRang** resolves these
/// paths against the registration tree, and no compiler can. A partitioned host no longer compiles the
/// subtrees its declaration names — they went to the crates the split created — so `crate::control::…`
/// has nothing to resolve against *in the host*, while the tree still knows that node. Saying
/// "the compiler checks the paths" sent a reader looking for a check that does not exist (the file is
/// read as text, never compiled).
/// 它说清"谁检查什么"，因为答案不是显而易见的那个：**XiRang** 对着注册树解析这些路径，而没有任何编译器
/// 能做这件事。被划分的宿主不再编译声明点名的子树——它们已经去了拆分产生的 crate——因此 `crate::control::…`
/// **在宿主里**没有可解析对象，而树仍然认得那个节点。写"编译器会检查这些路径"会让读者去找一个不存在的检查
/// （本文件是按**文本**读的，从不参与编译）。
fn unreadable(path: &Path, why: &str) -> String {
    format!(
        "{}: {why}. XiRang resolves these paths against the registration tree, not a compiler: a \
         partitioned host no longer compiles the subtrees its declaration names. This reader accepts \
         `Shape {{ package_prefix: \"…\", crates: &[Crate::named(\"…\").at(&[crate::…::SUBTREE])] }}`, \
         or the same thing built through `Shape::of(…)`",
        path.display()
    )
}

/// The closest of `candidates` to `claim`, when one is close enough to be worth naming.
/// 与 `claim` 最接近的那个候选（近到值得点名为止）。
///
/// A refusal that only says "no such node" makes the reader re-read the whole tree; naming the nearest
/// one turns it into a one-character fix. The threshold is relative to the claim's length so a short
/// name does not match everything.
/// 只写"没有这个节点"的拒绝会让读者重新读整棵树；点出最近的那个就把它变成改一个字母。阈值与 `claim` 的
/// 长度相关，免得短名字什么都"像"。
pub(crate) fn closest<'a>(
    claim: &str,
    candidates: impl Iterator<Item = &'a str>,
) -> Option<String> {
    let allowed = (claim.len() / 3).max(1);
    candidates
        .map(|candidate| (distance(claim, candidate), candidate))
        .filter(|(distance, _)| *distance <= allowed)
        .min_by_key(|(distance, candidate)| (*distance, *candidate))
        .map(|(_, candidate)| candidate.to_owned())
}

/// Levenshtein distance, two rows.
/// 编辑距离，两行。
fn distance(left: &str, right: &str) -> usize {
    let right: Vec<char> = right.chars().collect();
    let mut previous: Vec<usize> = (0..=right.len()).collect();
    let mut current = vec![0; right.len() + 1];
    for (i, a) in left.chars().enumerate() {
        current[0] = i + 1;
        for (j, b) in right.iter().enumerate() {
            let substitute = previous[j] + usize::from(a != *b);
            current[j + 1] = substitute.min(previous[j + 1] + 1).min(current[j] + 1);
        }
        std::mem::swap(&mut previous, &mut current);
    }
    previous[right.len()]
}

#[cfg(any(feature = "cli", feature = "mcp", feature = "studio"))]
/// One change to the declaration: which file, and its text before and after.
/// 声明的一次改动：哪个文件，以及它改前改后的文本。
///
/// The edit is **text**, not a re-render, and that is the whole design: `add_crates.rs` is
/// hand-written source the author also reads, so a writer that re-rendered the file would reformat
/// somebody else's code — and any comment they wrote would be gone. This one touches exactly the
/// entry it was asked about, and refuses when it cannot find that entry's end.
/// 这次改动是**文本**而不是重渲染，而这正是全部设计：`add_crates.rs` 是作者也会读的手写源码，因此一个
/// 重渲染的写入方会重排别人的代码——他们写的注释也会消失。这一份只动被点名的那一条，并在找不到那条的结尾时
/// 拒绝。
#[derive(Debug)]
pub(crate) struct DeclarationEdit {
    /// The file the edit belongs to.
    /// 这次改动所属的文件。
    pub(crate) path: PathBuf,
    /// The text before, byte for byte.
    /// 改前的文本，逐字节。
    pub(crate) before: String,
    /// The text after, byte for byte.
    /// 改后的文本，逐字节。
    pub(crate) after: String,
    /// Whether this edit removes the file instead of writing it, which is what removing the **last**
    /// declared crate means: the host goes back to being one crate, and a declaration that names no
    /// crate is a shape the reader refuses on purpose.
    /// 这次改动是删掉文件而不是写文件——这正是移除**最后一个**已声明 crate 的含义：宿主回到"就是一个
    /// crate"，而一份不点名任何 crate 的声明是读取器有意拒绝的形状。
    pub(crate) removes_file: bool,
}

#[cfg(any(feature = "cli", feature = "mcp", feature = "studio"))]
impl DeclarationEdit {
    /// The lines that differ, as `-`/`+` lines, for a preview or a log.
    /// 有差异的那些行，写成 `-`/`+`，供预览或日志使用。
    pub(crate) fn diff(&self) -> String {
        let before: Vec<&str> = self.before.lines().collect();
        let after: Vec<&str> = self.after.lines().collect();
        // A line-level comparison is enough here because the edit is one entry: the diff is what the
        // reader checks before letting it be written, and a general diff algorithm would be a second
        // thing to trust.
        // 行级比较在这里足够，因为改动就是一条条目：这段差异是读者在允许写入之前核对的东西，而一个通用 diff
        // 算法只会是多一个需要信任的东西。
        let mut start = 0;
        while start < before.len() && start < after.len() && before[start] == after[start] {
            start += 1;
        }
        let mut end_before = before.len();
        let mut end_after = after.len();
        while end_before > start
            && end_after > start
            && before[end_before - 1] == after[end_after - 1]
        {
            end_before -= 1;
            end_after -= 1;
        }
        let mut text = String::new();
        if self.removes_file {
            // Every line goes, because the whole file does.
            // 每一行都走，因为整个文件都走。
            for line in &before {
                let _ = writeln!(text, "-{line}");
            }
            return text;
        }
        for line in &before[start..end_before] {
            let _ = writeln!(text, "-{line}");
        }
        for line in &after[start..end_after] {
            let _ = writeln!(text, "+{line}");
        }
        if text.is_empty() {
            text.push_str("(no change)\n");
        }
        text
    }

    /// Put the file back the way it was before this edit.
    /// 把文件恢复成这次改动之前的样子。
    ///
    /// A change that turns out not to plan is rolled back with this rather than left behind: the
    /// declaration is what the rest of the tools read, so a version of it they refuse describes a
    /// shape nobody can act on.
    /// 一次事后发现"规划不成立"的改动用它回滚，而不是留下：声明是其余工具读取的东西，因此一份它们拒绝的声明
    /// 描述的是没人能据此行动的形态。
    pub(crate) fn restore(&self) -> Result<(), String> {
        if self.before.is_empty() {
            return std::fs::remove_file(&self.path)
                .map_err(|error| format!("cannot remove {}: {error}", self.path.display()));
        }
        write_if_changed(&self.path, &self.before)
            .map_err(|error| format!("cannot restore {}: {error}", self.path.display()))
    }

    /// Write the edited text, leaving the file exactly as it was when the edit is a no-op.
    /// 把改后的文本写下去；改动是空操作时，文件保持原样。
    pub(crate) fn apply(&self) -> Result<(), String> {
        if self.removes_file {
            return std::fs::remove_file(&self.path)
                .map_err(|error| format!("cannot remove {}: {error}", self.path.display()));
        }
        if self.before == self.after {
            return Ok(());
        }
        write_if_changed(&self.path, &self.after)
            .map_err(|error| format!("cannot write {}: {error}", self.path.display()))
    }
}

#[cfg(any(feature = "cli", feature = "mcp", feature = "studio"))]
/// The canonical declaration text for a host that has none yet.
/// 一个还没有声明的宿主，其规范的声明文本。
///
/// It is the shape the reader accepts and an author can extend by hand — the package prefix is filled
/// in from the host package the same way every generated crate name is derived, so the first crate
/// declared here cannot disagree with the packages it will produce.
/// 它既是读取器接受的形状，也是作者能手工扩展的形状——包前缀与每个生成包名的推导方式相同、取自宿主包，因此
/// 这里声明的第一个 crate 不可能与它将要产出的包不一致。
fn template(package_prefix: &str, name: &str, subtrees: &[String]) -> String {
    let mut entries = String::new();
    for subtree in subtrees {
        entries.push_str("\n        ");
        entries.push_str(&entry(name, subtree));
        entries.push(',');
    }
    format!(
        "// Which subtrees of this host become crates of their own. XiRang reads this file as text\n\
         // at build time; `xirang crates --check` prints what it would write.\n\
         // 这个宿主里哪些子树各自成为一个 crate。XiRang 在构建期把本文件当**文本**读；\n\
         // `xirang crates --check` 打印它会写下什么。\n\
         use xirang_toolchain::run_method::{{Crate, Shape}};\n\
         \n\
         // A function rather than a `const`: every name on the way to the value — `Crate::named`,\n\
         // `.at`, and the paths inside `&[…]` — is ordinary Rust, so an editor completes them and a\n\
         // mistyped path is `error[E0433]` pointing at that line and column.\n\
         // 用函数而不是 `const`：通向这个值的每个名字——`Crate::named`、`.at`，以及 `&[…]` 里的路径\n\
         // ——都是普通 Rust，因此编辑器会补全，写错的路径是 `error[E0433]` 并指到那一行那一列。\n\
         pub fn add_crates() -> Shape {{\n\
         \x20   Shape::of(\"{package_prefix}\", &[{entries}\n\
         \x20   ])\n\
         }}\n\
         \n\
         // This file is read as **text** by the build. Do not `mod` it into the crate: a partitioned\n\
         // host no longer compiles the subtrees its declaration names (they went to the crates the\n\
         // split created), so the `crate::…` paths here have nothing to resolve against.\n\
         // 本文件由构建期当**文本**读。不要把它 `mod` 进 crate：被划分的宿主不再编译声明点名的那些子树\n\
         // （它们已经去了拆分产生的 crate），因此这里的 `crate::…` 路径没有东西可解析。\n"
    )
}

#[cfg(any(feature = "cli", feature = "mcp", feature = "studio"))]
/// One `Crate::named(…).at(&[…])` entry, spelled the way the reader reads it.
/// 一条 `Crate::named(…).at(&[…])` 条目，按读取器读的那种拼法。
fn entry(name: &str, subtree: &str) -> String {
    format!("Crate::named(\"{name}\").at(&[{subtree}])")
}

#[cfg(any(feature = "cli", feature = "mcp", feature = "studio"))]
/// Where one declared crate's entry starts and ends in the file's text.
/// 某个已声明 crate 的条目在文件文本里的起止位置。
///
/// The end is found by bracket depth from the entry's own `Crate::named(`, not by looking for the
/// next newline: an entry may be written on several lines and may contain a nested `&[…::SUBTREE]`
/// list, and guessing its end from the layout is how a writer eats the next entry.
/// 结尾是从条目自己的 `Crate::named(` 起按括号深度找出来的，而不是找下一个换行：一条条目可能写成好几行、也可能
/// 含一个嵌套的 `&[…::SUBTREE]` 列表，而按版式猜结尾正是写入方吃掉下一条的方式。
fn entry_span(text: &str, name: &str) -> Result<std::ops::Range<usize>, String> {
    let needle = format!("Crate::named(\"{name}\")");
    let start = text.find(&needle).ok_or_else(|| {
        let known: Vec<&str> = text
            .split("Crate::named(\"")
            .skip(1)
            .filter_map(|chunk| chunk.split('"').next())
            .collect();
        format!(
            "no `{name}` in this declaration; it declares {}",
            if known.is_empty() {
                "nothing".to_owned()
            } else {
                known.join(", ")
            }
        )
    })?;
    let mut depth = 0i32;
    let mut end = None;
    for (offset, character) in text[start..].char_indices() {
        match character {
            '(' | '[' | '{' => depth += 1,
            ')' => depth -= 1,
            // The bracket that closes the list this entry lives in ends the entry: the last entry of
            // a single-line `crates: &[Crate::named(…)]` legitimately has **no** trailing comma, and a
            // reader that demanded one would refuse to remove exactly that entry (measured — it did).
            // 关掉这条条目所在列表的那个括号就是条目的结尾：单行 `crates: &[Crate::named(…)]` 里的最后
            // 一条**合法地没有**结尾逗号，而要求它存在的读取器会恰好拒绝移除那一条（实测：确实拒了）。
            ']' | '}' => {
                depth -= 1;
                if depth < 0 {
                    end = Some(start + offset);
                    break;
                }
            }
            ',' if depth == 0 => {
                end = Some(start + offset + 1);
                break;
            }
            _ => {}
        }
    }
    let end = end.ok_or_else(|| {
        format!("the `{name}` entry never ends: its list is not closed, so this writer cannot say where it stops")
    })?;
    Ok(start..end)
}

#[cfg(any(feature = "cli", feature = "mcp", feature = "studio"))]
/// Add a crate to the host's declaration, creating the file when the host has none.
/// 往宿主的声明里加一个 crate；宿主还没有这个文件时就把它建起来。
pub(crate) fn declare(
    package_root: &Path,
    name: &str,
    subtrees: &[String],
) -> Result<DeclarationEdit, String> {
    if name.is_empty() {
        return Err("a crate needs a name".to_owned());
    }
    if subtrees.is_empty() {
        return Err(format!(
            "`{name}` claims no subtree: a crate is a subtree — name one, e.g. \
             `crate::panel::frame::SUBTREE`"
        ));
    }
    for subtree in subtrees {
        if !subtree.contains("::") || subtree.contains('"') || subtree.contains('\n') {
            return Err(format!(
                "`{subtree}` is not a subtree path: write the host's own path to a `SUBTREE` \
                 constant, e.g. `crate::panel::frame::SUBTREE`"
            ));
        }
    }
    let path = package_root.join(lexicon::ADD_CRATES_FILE);
    let Ok(before) = std::fs::read_to_string(&path) else {
        let host = super::package::package_name(&package_root.join("Cargo.toml"))?;
        // An empty prefix is the one thing a template must not carry: every generated package name
        // is built from it, so a host whose name cargo cannot read is a refusal (audit `M7`, §M7.43).
        // 空前缀是模板唯一不能带的东西：每个生成包名都由它拼出，因此 cargo 说不出名字的宿主是一句拒绝
        // （审计 `M7`，§M7.43）。
        let after = template(&host, name, subtrees);
        return Ok(DeclarationEdit {
            path,
            before: String::new(),
            after,
            removes_file: false,
        });
    };
    if before.contains(&format!("Crate::named(\"{name}\")")) {
        return Err(format!(
            "`{name}` is already declared in {}",
            path.display()
        ));
    }
    // Two spellings again: the struct literal writes `crates: &[`, the constructor writes
    // `Shape::of("prefix", &[`. Prefer the field spelling when both appear.
    // 又是两种拼写：结构体字面量写 `crates: &[`，构造器写 `Shape::of("prefix", &[`。两者都在时优先字段拼写。
    let list_open = before
        .find("crates: &[")
        .or_else(|| {
            before
                .find("Shape::of(")
                .and_then(|at| before[at..].find("&[").map(|offset| at + offset))
        })
        .ok_or_else(|| {
            unreadable(
                &path,
                "it declares no `crates: &[…]` list (nor a `Shape::of(…)` one), so this writer \
                 cannot say where a new crate goes",
            )
        })?;
    // Past the `&[` itself, whichever spelling opened it.
    let list_start = list_open
        + before[list_open..]
            .find("&[")
            .map(|at| at + "&[".len())
            .unwrap_or("crates: &[".len());
    // The list's **own** closing bracket, by depth. Searching for the first `]` answered with the first
    // entry's, which is why the new crate used to land inside the previous one (see `closing_bracket`).
    // 列表**自己的**收尾括号，按深度找。搜第一个 `]` 会答出第一条 entry 的，这正是新 crate 过去落进上一条
    // 里面的原因（见 `closing_bracket`）。
    let close =
        closing_bracket(&before, list_start - 1).map_err(|reason| unreadable(&path, &reason))?;
    // Insert **before the closing bracket**, one entry per line: an inline list gains a line break
    // first, a list that already spans lines keeps its own layout, and every byte outside this
    // insertion is untouched.
    // 插在**闭括号之前**，一条一行：单行列表先补一个换行，已经跨行的列表保持它自己的版式，而这次插入之外的
    // 每一个字节都没被动过。
    // The closing bracket usually sits on its own line, and inserting *at* it would leave that line's
    // indentation behind as a whitespace-only line — an edit no reviewer can see in a diff, and one that
    // accumulates with every `declare`. When the text between the line start and the bracket is blank, the
    // insertion point is the line start.
    // 收尾括号通常独占一行，插在**它那里**会把那一行的缩进留成一行只有空白的行——那是 diff 里看不见的改动，
    // 而且每 `declare` 一次就累积一次。当行首到括号之间只有空白时，插入点是行首。
    let line_start = before[..close].rfind('\n').map_or(0, |at| at + 1);
    let anchor = if before[line_start..close].trim().is_empty() {
        line_start
    } else {
        close
    };
    let mut inserted = String::new();
    // A comma when the entry before the insertion point does not end with one, and this is the same defect as
    // the wrong anchor one shape later: an **inline** list (`crates: &[Crate::named("a").at(…)` on a single
    // line) has no trailing comma to inherit, so inserting a whole entry gave two adjacent expressions —
    // measured, `check` then refused the file the preview had just described as a success (audit `M7`,
    // §M7.61). A list that already spans lines ends its last entry with a comma and gains nothing.
    // 插入点之前那条 entry 没有以逗号结尾时补一个，而它与"锚点找错"是同一个缺陷晚一种版式：**单行**列表
    // （`crates: &[Crate::named("a").at(…)` 在一行里）没有可继承的尾逗号，于是插入一整条 entry 会得到两个相邻
    // 的表达式——实测，`check` 随后拒绝了预览刚描述为成功的那份文件（审计 `M7`，§M7.61）。已经跨行的列表最后
    // 一条 entry 本来就带逗号，什么也不加。
    let tail = before[..anchor].trim_end();
    if !tail.ends_with(',') && !tail.ends_with('[') {
        inserted.push(',');
    }
    if !before[..anchor].ends_with('\n') {
        inserted.push('\n');
    }
    for subtree in subtrees {
        inserted.push_str("        ");
        inserted.push_str(&entry(name, subtree));
        inserted.push_str(",\n");
    }
    let mut after = String::with_capacity(before.len() + inserted.len());
    after.push_str(&before[..anchor]);
    after.push_str(&inserted);
    after.push_str(&before[anchor..]);
    Ok(DeclarationEdit {
        path,
        before,
        after,
        removes_file: false,
    })
}

#[cfg(any(feature = "cli", feature = "mcp", feature = "studio"))]
/// Remove one crate from the host's declaration, leaving every other byte alone.
/// 从宿主的声明里移除一个 crate，其余每一个字节都保持原样。
pub(crate) fn undeclare(package_root: &Path, name: &str) -> Result<DeclarationEdit, String> {
    let path = package_root.join(lexicon::ADD_CRATES_FILE);
    let before = std::fs::read_to_string(&path).map_err(|error| {
        format!(
            "{}: {error}; this host declares no crates, so there is none to remove",
            path.display()
        )
    })?;
    let span = entry_span(&before, name).map_err(|why| format!("{}: {why}", path.display()))?;
    // The last declared crate is the whole declaration: removing it is the host going back to one
    // crate, so the file goes rather than becoming a `crates: &[]` the reader refuses.
    // 最后一个已声明的 crate 就是整份声明：移除它等于宿主回到"就是一个 crate"，因此走的是文件本身，而不是
    // 变成一份读取器会拒绝的 `crates: &[]`。
    let only_entry = before[span.end..].find("Crate::named(").is_none()
        && before[..span.start].find("Crate::named(").is_none();
    // The entry's own line goes with it: leaving the indentation behind would be a whitespace-only
    // edit no reviewer can see in a diff, and the next `declare` would then insert beside a blank.
    // 条目所在的那一行连同它一起走：把缩进留下会变成 diff 里看不见的空白改动，而下一次 `declare` 会插在一行空白旁边。
    let mut start = span.start;
    while start > 0 && matches!(before.as_bytes()[start - 1], b' ' | b'\t') {
        start -= 1;
    }
    let mut end = span.end;
    while end < before.len() && matches!(before.as_bytes()[end], b' ' | b'\t') {
        end += 1;
    }
    if end < before.len() && before.as_bytes()[end] == b'\n' {
        end += 1;
    }
    if only_entry {
        return Ok(DeclarationEdit {
            path,
            before,
            after: String::new(),
            removes_file: true,
        });
    }
    let mut after = String::with_capacity(before.len());
    after.push_str(&before[..start]);
    after.push_str(&before[end..]);
    Ok(DeclarationEdit {
        path,
        before,
        after,
        removes_file: false,
    })
}

#[cfg(test)]
#[path = "shape_decl_tests.rs"]
mod shape_decl_tests;
