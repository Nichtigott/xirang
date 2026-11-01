//! The NAM-33 verb table, checked where a name is **declared** rather than where the
//! text happens to contain it.
//! NAM-33 动词表：检查落在名字被**声明**的位置，而不是文本恰好包含它的位置。
//!
//! `docs/audit-2026-09-28/audit-naming-review.md` settled the vocabulary: `read_*`
//! reads text or bytes, `load_*` brings something into a process-wide or cached
//! place, `resolve_*` maps a name to something that already exists, `find_*` looks
//! something up (and must not be spelled `get_*` when it returns a value rather
//! than an `Option`), `collect_*` accumulates into a caller-provided collection and
//! belongs on the *public* function only (its recursive helper is `visit_*` /
//! `walk_*`), `parse_*` turns text into structure, `render_*` turns structure into
//! text, `write_*` writes through a `Write`, `validate_*` answers whether a whole
//! value is conforming, `check_*` is a runtime assertion, and a **bare verb** is
//! allowed only at an **entry position** (a `main`, a `build.rs` `run`, a bin).
//! `docs/audit-2026-09-28/audit-naming-review.md` 定下了这套词汇：`read_*` 读文本或字节、
//! `load_*` 把东西带进进程级或缓存处、`resolve_*` 把名字映射到已存在之物、`find_*` 查出一个
//! 值（返回值而非 `Option` 时不得写成 `get_*`）、`collect_*` 累积进调用方给的集合且**只**用于
//! 公开函数（它背后的递归 helper 叫 `visit_*` / `walk_*`）、`parse_*` 把文本变成结构、
//! `render_*` 把结构变成文本、`write_*` 经 `Write` 写出、`validate_*` 回答整个值是否合规、
//! `check_*` 是运行期断言，而**裸动词**只许出现在**入口位**（`main`、`build.rs` 的 `run`、bin）。
//!
//! Three rules are mechanical from the *declaration* and are checked here; the rest
//! of the table is vocabulary, so it is listed rather than guessed at.
//! 其中三条能从**声明**机械地判定，就在这里检查；表中其余是词汇共识，因此只列出、不做猜测：
//!
//! 1. a function whose name starts with `get_` must return an `Option` (otherwise the
//!    table says `find_*`); 2. a **bare** verb (a function name with no `_`) is only
//!    allowed at an entry position; 3. `collect_*` is only allowed on a **public**
//!    function — a private one is the recursive helper the table calls `visit_*`.
//! 1. 以 `get_` 开头的函数必须返回 `Option`（否则表里那个动词是 `find_*`）；2. **裸**动词
//!    （函数名不含 `_`）只许出现在入口位；3. `collect_*` 只许用在**公开**函数上——私有那个
//!    是表里叫作 `visit_*` 的递归 helper。
//!
//! A violation is reported only from a real declaration (`fn <name>`), so a doc
//! comment that *mentions* `get_x` is not one — that is the difference between this
//! gate and a substring search.
//! 只有真实的声明（`fn <name>`）才会报违规，因此**提到** `get_x` 的文档注释不算——这就是本门禁
//! 与子串搜索的区别。

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use syn::visit::Visit;

/// The documented verbs, with the meaning the review fixed for each.
/// 词汇共识里的动词，以及复核为每一个定下的含义。
pub const VERBS: &[(&str, &str)] = &[
    ("read_", "reads text or bytes 读文本或字节"),
    (
        "load_",
        "brings something into a process-wide or cached place 带进进程级/缓存处",
    ),
    (
        "resolve_",
        "maps a name to something that already exists 把名字映射到已存在之物",
    ),
    (
        "find_",
        "looks a value up (not `get_` when it is not an Option) 查出一个值",
    ),
    (
        "collect_",
        "accumulates into a caller-provided collection 累积进调用方给的集合",
    ),
    ("visit_", "recursive helper that visits 递归访问 helper"),
    ("walk_", "recursive helper that walks 递归遍历 helper"),
    ("parse_", "turns text into structure 把文本变成结构"),
    ("render_", "turns structure into text 把结构变成文本"),
    ("write_", "writes through a `Write` 经 `Write` 写出"),
    (
        "validate_",
        "answers whether a whole value is conforming 整个值是否合规",
    ),
    ("check_", "runtime assertion 运行期断言"),
];

/// Bare verbs that are allowed, and only where an entry position frames them.
/// 允许出现的裸动词——且仅在入口位为其提供语境时。
pub const BARE_VERBS_AT_ENTRY: &[&str] = &["main", "run", "build", "check", "clean"];

/// The stems whose bare (single-word) spelling is a verb, and therefore entry-only.
/// Nouns are not in this list: `fn tree()` or `fn face()` is a noun, not a bare verb.
/// 裸（单词）拼法属动词、因而只能在入口位出现的词干。名词不在清单里：`fn tree()`、`fn face()`
/// 是名词而不是裸动词。
pub const BARE_VERB_STEMS: &[&str] = &[
    "read", "load", "resolve", "find", "collect", "visit", "walk", "parse", "render", "write",
    "validate", "check", "get", "handle", "process", "make", "do", "spawn", "start", "stop",
    "open", "close", "send", "recv", "build", "run", "clean",
];

/// One violation: where it was declared, and which rule it broke.
/// 一条违规：声明在哪里，以及违反的是哪条规则。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Violation {
    /// Workspace-relative path of the file the declaration sits in.
    /// 声明所在文件的工作区相对路径。
    pub file: String,
    /// 1-based line of the declaration.
    /// 声明所在行（从 1 开始）。
    pub line: usize,
    /// The declared function name.
    /// 被声明的函数名。
    pub name: String,
    /// Which rule it broke, in the table's wording.
    /// 违反的规则，用表的措辞表述。
    pub why: String,
}

/// Whether a file is an entry position: `build.rs`, a `src/bin/` target, or a cargo
/// example (an example is a process entry too).
/// 该文件是否属入口位：`build.rs`、`src/bin/` 下的 target，或 cargo example（example 也是
/// 进程入口）。
pub fn is_entry_file(path: &Path) -> bool {
    let text = path.to_string_lossy().replace('\\', "/");
    // Three more positions are entries by nature rather than by path: the build-script entry the
    // host's `build.rs` calls (`build_method::run`), the CLI's argv dispatch (`cli::run`), and the
    // bridge's stdio loop (`mcp::protocol::run`). Rule ④ allows a bare verb at an entry position,
    // and these *are* the entries of their surfaces — everything else in those files still has to
    // carry its object.
    // 还有三个位置是"按性质"的入口而不是按路径：宿主 `build.rs` 调用的构建期入口
    // （`build_method::run`）、CLI 的 argv 分发（`cli::run`）、桥的 stdio 循环
    // （`mcp::protocol::run`）。规则 ④ 允许裸动词出现在入口位，而它们**就是**各自执行面的入口
    // ——这三个文件里的其它东西仍然必须带宾语。
    const ENTRY_FILES: &[&str] = &[
        "toolchain/src/build_method/lib.rs",
        "toolchain/src/cli/lib.rs",
        "toolchain/src/mcp/protocol.rs",
    ];
    if ENTRY_FILES.contains(&text.as_str()) {
        return true;
    }
    text.ends_with("/build.rs")
        || text.ends_with("/build")
        || text.contains("/src/bin/")
        || text.contains("/examples/")
}

/// The declarations of `source`: `(line, name, is_public, signature_tail)`.
/// `source` 里的函数声明：`(行号, 名字, 是否公开, 签名尾部)`。
fn declarations(source: &str) -> Vec<(usize, String, bool, String)> {
    // Scan the workspace's **one** lexical rule rather than the raw text: a string literal that
    // happens to spell `fn main` is data, not a declaration — a generated `build.rs` is written from
    // such a literal (audit `M7`, P3.2), and blanket-matching it would forbid a generator from
    // emitting the very file it exists to emit.
    // 用本工作区**唯一**的词法规则来扫描，而不是原文：恰好拼出 `fn main` 的字符串字面量是数据、不是声明
    // ——生成的 `build.rs` 正是从这样的字面量里写出来的（审计 `M7`，P3.2），一刀切地匹配它，等于禁止一个
    // 生成器发射它存在就是为了发射的那份文件。
    let syntax_owned = syntax_exempt_lines(source);
    let source = &xirang_kernel::source::mask_non_code(source);
    let mut out = Vec::new();
    let lines: Vec<&str> = source.lines().collect();
    // A deprecated declaration is a **compatibility entry**: it exists so a published caller keeps
    // compiling, and the table cannot ask it to disappear. The suffix is explicit, so the exemption
    // is precise rather than a blanket allow-list.
    // 被 `#[deprecated]` 标记的声明是**兼容入口**：它存在是为了让已发布的调用方继续编译，词汇表不能要求它消失。
    // 标记是显式的，因此豁免是精确的，而不是一张范围宽泛的 allow-list。
    let is_deprecated = |upto: usize| -> bool {
        let mut i = upto;
        while i > 0 {
            i -= 1;
            let line = lines[i].trim();
            if line.is_empty() {
                return false;
            }
            if line.starts_with("#[deprecated") {
                return true;
            }
            // A multi-line attribute contributes `note = ...` / `)]` lines between the opening
            // `#[deprecated(` and the declaration; they are part of the same marker.
            // 多行属性在 `#[deprecated(` 与声明之间贡献 `note = ...` / `)]` 行；它们属于同一个标记。
            if line.starts_with("///")
                || line.starts_with("//!")
                || line.starts_with("//")
                || line.starts_with("#[")
                || line.starts_with("note =")
                || line.starts_with("since =")
                || line == ")]"
                || line == ")"
            {
                continue;
            }
            return false;
        }
        false
    };
    for (index, line) in lines.iter().enumerate() {
        let trimmed = line.trim_start();
        let is_public = trimmed.starts_with("pub ");
        let body = trimmed.strip_prefix("pub ").unwrap_or(trimmed);
        let Some(rest) = body.strip_prefix("fn ") else {
            continue; // only item declarations count; `// fn get_x` is not one
        };
        let name: String = rest
            .chars()
            .take_while(|character| character.is_alphanumeric() || *character == '_')
            .collect();
        if name.is_empty() {
            continue;
        }
        if syntax_owned.contains(&(index + 1)) || is_deprecated(index) {
            continue; // the syntax owns the name, or this is a compatibility entry
        }
        // The signature may wrap, so the tail is taken up to the opening brace.
        let mut signature = String::new();
        for candidate in lines.iter().skip(index) {
            signature.push_str(candidate);
            if candidate.contains('{') || candidate.trim_end().ends_with(';') {
                break;
            }
        }
        out.push((index + 1, name, is_public, signature));
    }
    out
}

/// The lines of function declarations whose names are owned by **syntax**, not by this crate:
/// direct functions of a trait declaration, direct methods of a trait implementation, and foreign
/// functions in an `extern` block.
/// 名字由**语法**（而非本 crate）拥有的函数声明行：trait 声明里的直接函数、trait 实现的直接方法，
/// 以及 `extern` 块里的外部函数。
///
/// Parsing with `syn` is what makes the ownership exact: a backward search for the nearest `trait`
/// or `impl` header keeps answering "inside the trait" after its closing brace, while syn tells us
/// which item directly contains each function. A file that does not parse contributes no exemption
/// (conservative: the name is checked rather than silently skipped).
/// 用 `syn` 解析才能得到精确归属：往回找最近的 `trait`/`impl` 头会在闭合括号之后仍回答"在 trait 里"，
/// 而 syn 直接告诉我们每个函数属于哪个 item。解析失败的文件不贡献豁免（保守：检查名字而不是静默跳过）。
fn syntax_exempt_lines(source: &str) -> BTreeSet<usize> {
    let Ok(file) = syn::parse_file(source) else {
        return BTreeSet::new();
    };
    let mut owners = SyntaxOwners::default();
    owners.visit_file(&file);
    owners.0
}

#[derive(Default)]
struct SyntaxOwners(BTreeSet<usize>);

impl<'ast> Visit<'ast> for SyntaxOwners {
    fn visit_item_trait(&mut self, node: &'ast syn::ItemTrait) {
        for item in &node.items {
            if let syn::TraitItem::Fn(function) = item {
                self.0.insert(function.sig.fn_token.span.start().line);
            }
        }
        syn::visit::visit_item_trait(self, node);
    }

    fn visit_item_impl(&mut self, node: &'ast syn::ItemImpl) {
        if node.trait_.is_some() {
            for item in &node.items {
                if let syn::ImplItem::Fn(function) = item {
                    self.0.insert(function.sig.fn_token.span.start().line);
                }
            }
        }
        syn::visit::visit_item_impl(self, node);
    }

    fn visit_item_foreign_mod(&mut self, node: &'ast syn::ItemForeignMod) {
        for item in &node.items {
            if let syn::ForeignItem::Fn(function) = item {
                self.0.insert(function.sig.fn_token.span.start().line);
            }
        }
        syn::visit::visit_item_foreign_mod(self, node);
    }
}

/// Every verb-table violation in `source`, given the file it came from.
/// 给定来源文件，列出 `source` 里所有动词表违规。
pub fn violations_in(file: &str, source: &str) -> Vec<Violation> {
    let entry = is_entry_file(Path::new(file));
    let mut out = Vec::new();
    for (line, name, is_public, signature) in declarations(source) {
        if name.starts_with("get_") && !signature.contains("-> Option<") {
            out.push(Violation {
                file: file.to_owned(),
                line,
                name: name.clone(),
                why: "`get_` must return an `Option`; otherwise the table's verb is `find_` \
                      / `get_` 必须返回 `Option`，否则表里的动词是 `find_`"
                    .to_owned(),
            });
        }
        if !name.contains('_')
            && BARE_VERB_STEMS.contains(&name.as_str())
            && !BARE_VERBS_AT_ENTRY.contains(&name.as_str())
        {
            out.push(Violation {
                file: file.to_owned(),
                line,
                name: name.clone(),
                why: "this bare verb is not in the entry-position list \
                      / 这个裸动词不在入口位清单里"
                    .to_owned(),
            });
        }
        if !name.contains('_') && BARE_VERBS_AT_ENTRY.contains(&name.as_str()) && !entry {
            out.push(Violation {
                file: file.to_owned(),
                line,
                name: name.clone(),
                why: "a bare verb is allowed only at an entry position (`build.rs`, `src/bin/`) \
                      / 裸动词只许出现在入口位（`build.rs`、`src/bin/`）"
                    .to_owned(),
            });
        }
        if name.starts_with("collect_") && !is_public {
            out.push(Violation {
                file: file.to_owned(),
                line,
                name: name.clone(),
                why: "`collect_` belongs on the public function; the recursive helper is \
                      `visit_`/`walk_` / `collect_` 只用于公开函数，递归 helper 叫 `visit_`/`walk_`"
                    .to_owned(),
            });
        }
    }
    out
}

/// Every `.rs` file under `root` (skipping `target/` and the agent-team state).
/// `root` 下的每个 `.rs` 文件（跳过 `target/` 与 agent-team 状态）。
pub fn rust_files(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(directory) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&directory) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            if path.is_dir() {
                if name == "target" || name.starts_with('.') {
                    continue;
                }
                stack.push(path);
            } else if name.ends_with(".rs") {
                out.push(path);
            }
        }
    }
    out.sort();
    out
}

/// Every verb-table violation in the workspace rooted at `root`.
/// 以 `root` 为根的工作区里所有动词表违规。
pub fn violations(root: &Path) -> Vec<Violation> {
    let mut out = Vec::new();
    for path in rust_files(root) {
        let relative = path
            .strip_prefix(root)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");
        if crate::archived_evaluation(&relative) {
            continue;
        }
        let Ok(source) = std::fs::read_to_string(&path) else {
            continue;
        };
        out.extend(violations_in(&relative, &source));
    }
    out
}
