//! Source index and path safety for the MCP bridge's read side.
//! MCP 桥读取一侧的源码索引与路径安全。
//!
//! online: the index **is** the source reading the other answers are built on; the build publishes faces and nothing at this granularity.

use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

/// One indexed Rust function with its static direct-call set.
/// 一个已索引的 Rust 函数及其静态直接调用集合。
#[derive(Clone, Debug)]
pub(crate) struct Function {
    pub(crate) name: String,
    /// Owning type/trait for a method, when the syntax names one.
    /// 方法所属类型/trait（语法点名时）。
    pub(crate) owner: Option<String>,
    /// `Owner::name` for methods, bare name for free functions.
    /// 方法的 `Owner::name`，自由函数的裸名。
    pub(crate) qualified: String,
    pub(crate) line: usize,
    pub(crate) end_line: usize,
    /// Definite calls; qualified where the syntax names the owner.
    /// 确定的调用；语法点名所属者时已限定。
    pub(crate) calls: Vec<String>,
    /// Calls whose target the semantic reader cannot prove.
    /// 语义读取器无法证明目标的调用。
    pub(crate) unresolved: Vec<String>,
    /// Facts that qualify a call which is still definite (a decorator that may wrap the binding).
    /// 限定一条仍然确定的调用的事实（可能包装绑定的装饰器）。
    pub(crate) caveats: Vec<String>,
    /// Whether these facts came from `syn` parsing rather than the lexical fallback.
    /// 这些事实来自 `syn` 解析还是词法回退。
    pub(crate) semantic: bool,
    /// The registration object whose source declares/contains this function, when known.
    /// 声明/包含此函数的注册对象（已知时）。
    pub(crate) registration: Option<String>,
    /// The declared graft replacement for this function's registration slot, when one is declared.
    /// 该函数所属注册槽位声明的 graft 替换件（若声明了）。
    pub(crate) binding: Option<String>,
}

/// One indexed Rust source file.
/// 一个已索引的 Rust 源文件。
#[derive(Clone, Debug)]
pub(crate) struct SourceFile {
    pub(crate) relative: String,
    pub(crate) source: String,
    pub(crate) functions: Vec<Function>,
    /// The file's branch-level facts, as the kernel's own read reports them.
    /// 该文件的分支级事实，按内核自己的读取报告的样子。
    ///
    /// This module **carries** them; it does not decide them. Which arms are unreachable is a
    /// lexical judgement over masked text, and the workspace has one implementation of that
    /// judgement (`kernel::source`), so a surface that lexed the file again would be the second
    /// rule that drifts — the same reason `functions` comes from `function_symbols`.
    /// 本模块只**搬运**它们，不判定它们。哪些臂不可达是对掩码文本的词法判断，而本工作区只有一份那份
    /// 判断的实现（`kernel::source`），因此再词法一遍的执行面就是第二条会漂移的规则——`functions`
    /// 来自 `function_symbols` 是同一条理由。
    pub(crate) branches: xirang_kernel::source::BranchFacts,
}

pub(crate) fn required_path(arguments: &Value) -> Result<String, String> {
    arguments
        .get("path")
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| "path is required".to_owned())
}

pub(crate) fn load_sources(root: &Path) -> Result<Vec<SourceFile>, String> {
    load_sources_matching(root, |_, _| true)
}

/// The same index, with the **parsing** decided per file (audit `W6-2`, step two).
/// 同一份索引，但**解析**与否按文件决定（审计 `W6-2` 第②步）。
///
/// `keep` sees each file's relative path and its text, and answers whether that file is worth
/// lexing. A caller whose question is "does this text appear anywhere" passes `false` for everything
/// and still gets every path and every line — only the symbol scan is skipped; a caller whose question
/// is "where is this **name** defined" passes "the text mentions it", because a name that is not in
/// the bytes cannot be a declaration in them. Both callers still see **every file**, which is the
/// property the index owes its readers: a pre-filter decides what to *lex*, never what to *list*.
/// `keep` 拿到每个文件的相对路径与文本，回答"这个文件值不值得词法一遍"。问题是"这段文本出现在哪儿"的
/// 调用方对一切回 `false`，照样拿到每条路径与每一行——只是跳过符号扫描；问题是"这个**名字**定义在哪"的
/// 调用方回"文本里提到它"，因为不在字节里的名字不可能是字节里的声明。两类调用方仍然看到**每个文件**，而
/// 这正是索引欠读者的性质：预筛决定**词法什么**，绝不决定**列出什么**。
pub(crate) fn load_sources_matching(
    root: &Path,
    keep: impl FnMut(&str, &str) -> bool,
) -> Result<Vec<SourceFile>, String> {
    load_sources_in(root, &[], keep)
}

/// Read only caller-selected subtrees/files, preserving package-relative evidence paths.
/// 只读调用方选择的子树或文件，证据路径仍相对项目根，不暗示检查过范围外源码。
pub(crate) fn load_sources_in(
    root: &Path,
    scopes: &[String],
    mut keep: impl FnMut(&str, &str) -> bool,
) -> Result<Vec<SourceFile>, String> {
    if !super::source_request::active() {
        return load_uncached(root, scopes, keep);
    }
    let mut sources = match super::source_request::get(root, scopes) {
        Some(sources) => sources,
        None => {
            let sources = load_uncached(root, scopes, |_, _| false)?;
            super::source_request::put(root, scopes, &sources);
            sources
        }
    };
    for file in &mut sources {
        if keep(&file.relative, &file.source) {
            parse_in_place(file);
        }
    }
    Ok(sources)
}

fn load_uncached(
    root: &Path,
    scopes: &[String],
    mut keep: impl FnMut(&str, &str) -> bool,
) -> Result<Vec<SourceFile>, String> {
    if !root.is_dir() {
        return Err(format!("source root does not exist: {}", root.display()));
    }
    let canonical_root =
        fs::canonicalize(root).map_err(|error| format!("source root is not readable: {error}"))?;
    let mut paths = Vec::new();
    if scopes.is_empty() {
        visit_rs(root, &mut paths)?;
    } else {
        for scope in scopes {
            let relative = Path::new(scope);
            if relative.components().any(|part| {
                !matches!(
                    part,
                    std::path::Component::Normal(_) | std::path::Component::CurDir
                )
            }) || relative.components().any(|part| {
                matches!(
                    part.as_os_str().to_str(),
                    Some("target" | ".git" | ".codegraph" | ".xirang")
                )
            }) {
                return Err(format!(
                    "scope `{scope}` must name source inside the project"
                ));
            }
            let path = root.join(relative);
            canonical_child(&canonical_root, &path)
                .ok_or_else(|| format!("scope `{scope}` is missing or escapes the project"))?;
            if path.is_dir() {
                visit_rs(&path, &mut paths)?;
            } else if path.extension().is_some_and(|extension| extension == "rs") {
                paths.push(path);
            } else {
                return Err(format!(
                    "scope `{scope}` must be a source directory or Rust file"
                ));
            }
        }
    }
    paths.sort();
    paths.dedup();
    // A link that resolves outside the root is not part of the index: the root is
    // the declared scope of every answer this bridge gives, and a file outside it
    // would make those answers larger than the package. The walk already refuses
    // to descend into such a directory; this drops a linked *file*. It is a skip
    // rather than an error because the entry is outside the question, while a
    // direct `inspect`/`read` of the same path is refused by name.
    // 解析到根外的链接不属于本索引：根是这个桥给出的每个回答所声明的范围，而根外的文件会让
    // 回答比包更大。遍历已经拒绝进入这样的目录；这里丢弃的是被链接的**文件**。之所以跳过而
    // 不是报错，是因为该条目在问题范围之外，而对同一路径的直接 `inspect`/`read` 会按名字被拒。
    paths
        .into_iter()
        .filter_map(|path| {
            canonical_child(&canonical_root, &path)
                .map(|resolved| load_filtered_file(root, &path, &resolved, &mut keep))
        })
        .collect()
}

/// Read one indexed file, lexing it only when the caller's filter says so.
/// 读一个被索引的文件，只在调用方的筛选说"值得"时才词法一遍。
fn load_filtered_file(
    root: &Path,
    path: &Path,
    resolved: &Path,
    keep: &mut impl FnMut(&str, &str) -> bool,
) -> Result<SourceFile, String> {
    // Read the checked destination, not the original link; root canonicalization is shared per request.
    // 读取已检查的目标而非原链接；根的规范化在请求内共用。
    let text = fs::read_to_string(resolved)
        .map_err(|error| format!("read {}: {error}", path.display()))?;
    let relative = portable_path(
        path.strip_prefix(root)
            .map_err(|_| "source path escaped root".to_owned())?,
    );
    if !keep(&relative, &text) {
        return Ok(SourceFile {
            relative,
            source: text,
            functions: Vec::new(),
            branches: xirang_kernel::source::BranchFacts::default(),
        });
    }
    Ok(index_file(relative, text))
}

/// The indexed text of exactly these directories, without walking the tree (audit `W6-2`).
/// 只要这些目录的已索引文本，**不遍历整棵树**（审计 `W6-2`）。
///
/// `consistency` knows which family it is about to compare before it needs any text — the derived tree
/// told it — so reading the whole root to then use a few hundred files is work the question never
/// asked for. Measured on a 50,000-file workspace: the whole-tree read is ~24 s of a ~167 s answer,
/// and it is the part that scales with the **tree** rather than with the **family**.
/// `consistency` 在需要任何文本之前就知道自己要比较哪一家——推导树已经告诉它了——因此先读整个根、再从中
/// 用上几百个文件，是问题从没要求过的工作。在五万文件的工作区上实测：整棵树那次读取约占 ~167 s 答案里的
/// ~24 s，而且它正是**跟树走**、不跟家族走的那一部分。
///
/// The directories are relative to `root` and are each read on their own; a directory that is not
/// there contributes nothing rather than failing the answer, because a sibling whose file the index
/// cannot read is a case every caller already reports.
/// 目录相对 `root`，各自单独读；不存在的目录不贡献任何东西、也不让答案失败，因为"索引读不到某个兄弟的文件"
/// 是每个调用方本来就会报出来的情形。
/// The `src/…` directory each of these faces lives in, deduplicated (audit `W6-2`).
/// 这些面各自所在的 `src/…` 目录，已去重（审计 `W6-2`）。
///
/// The spelling is this module's own: `FaceView.source` is relative to `src/` while an index path is
/// relative to the package root, and putting the `src/` on in one place is what keeps those two
/// coordinates from being mixed up somewhere else — the mix-up that made a workspace member's answers
/// come back with an empty sibling set.
/// 这个拼写属于本模块：`FaceView.source` 相对 `src/`，而索引路径相对包根；把那个 `src/` 只加在一处，才能
/// 让这两套坐标不在别处被混起来——而混起来正是"工作区成员的答案拿到空兄弟集合"的原因。
/// The one spelling an index path is written in: `src/…`, from the package root (audit `T3`).
/// 索引路径唯一的那种拼写：`src/…`，从包根算起（审计 `T3`）。
///
/// `FaceView.source` is relative to `src/` while a record's `source` column and every `SourceFile`
/// are relative to the package root — two coordinates for the same file, and mixing them up makes a
/// lookup miss **silently**. It did: `shape_names_from_record` keyed its map by the record's
/// spelling and the caller looked faces up with `src/` in front, so every sibling missed the record
/// and fell back to reading its file — 7 s on a 2,500-face member, while the reply said it had read
/// the record. One function, used by both sides, is what keeps that from happening again.
/// `FaceView.source` 相对 `src/`，而记录的 `source` 列与每个 `SourceFile` 都相对包根——同一个文件的两套
/// 坐标，混起来会让查询**静默**落空。它真的落空了：`shape_names_from_record` 用记录的拼写做键，调用方却
/// 带着 `src/` 去查，于是每个兄弟都没命中记录、退回读自己的文件——在一个 2,500 面的成员上花了 7 s，而回复
/// 还说着"读的是记录"。一个函数、两侧共用，才能让它不再发生。
pub(crate) fn indexed_path(face_source: &str) -> String {
    format!("src/{}", face_source.trim_start_matches("./"))
}

pub(crate) fn face_directories<'a>(
    faces: impl IntoIterator<Item = &'a crate::build_method::FaceView>,
) -> Vec<String> {
    let mut directories: Vec<String> = faces
        .into_iter()
        .map(|face| {
            let file = format!("src/{}", face.source.trim_start_matches("./"));
            file.rsplit_once('/')
                .map(|(directory, _)| directory.to_owned())
                .unwrap_or(file)
        })
        .collect();
    directories.sort();
    directories.dedup();
    directories
}

pub(crate) fn load_directories(
    root: &Path,
    directories: &[String],
) -> Result<Vec<SourceFile>, String> {
    let mut paths = Vec::new();
    for directory in directories {
        let joined = root.join(directory);
        let Ok(entries) = fs::read_dir(&joined) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|extension| extension == "rs") {
                paths.push(path);
            }
        }
    }
    paths.sort();
    paths.dedup();
    paths
        .into_iter()
        .filter(|path| is_safe_child(root, path))
        .map(|path| load_file(root, &path))
        .collect()
}

/// The filesystem facts the kernel's source walk asks this surface for.
/// 内核源码遍历向本执行面索取的文件系统事实。
///
/// The root is resolved once and carried, because the walk's own facts are not
/// enough to keep it inside the tree: `is_dir` follows a symbolic link, so a
/// link under the root can point at an ancestor or at a tree outside the package
/// entirely. Every fact this surface reports therefore goes through the
/// canonical form of the path.
/// 根只解析一次并随行携带，因为遍历自身的事实不足以把它留在树内：`is_dir` 会跟随符号链接，
/// 因此根下的一个链接可以指向祖先，或指向包外的整棵树。因此本执行面报告的每个事实都经路径
/// 的规范形式。
struct StdSourceTree {
    /// The canonical source root.
    /// 规范化的源码根。
    root: PathBuf,
}

impl xirang_kernel::source::SourceTree for StdSourceTree {
    fn is_directory(&self, path: &Path) -> bool {
        path.is_dir() && canonical_child(&self.root, path).is_some()
    }

    fn entries(&self, path: &Path) -> Result<Vec<PathBuf>, String> {
        fs::read_dir(path)
            .map_err(|error| format!("read {}: {error}", path.display()))?
            .map(|entry| {
                entry
                    .map(|entry| entry.path())
                    .map_err(|error| format!("read directory entry: {error}"))
            })
            .collect()
    }

    fn read_text(&self, path: &Path) -> Result<String, String> {
        if !is_safe_child(&self.root, path) {
            return Err(format!(
                "{} cannot be resolved inside the configured source root",
                path.display()
            ));
        }
        fs::read_to_string(path).map_err(|error| format!("read {}: {error}", path.display()))
    }
}

fn visit_rs(directory: &Path, paths: &mut Vec<PathBuf>) -> Result<(), String> {
    let tree = StdSourceTree {
        root: fs::canonicalize(directory).unwrap_or_else(|_| directory.to_path_buf()),
    };
    xirang_kernel::source::collect_rust_sources(
        &tree,
        directory,
        xirang_kernel::source::SourceWalk {
            skip_target: true,
            ..xirang_kernel::source::SourceWalk::EVERYTHING
        },
        |_, _| xirang_kernel::source::Keep::Yes,
        paths,
    )?;
    // The write path's own recoverable trash lives under `.xirang/` and holds
    // `.rs` files, so without this filter a deleted fact keeps answering `status`
    // and `search` from its backup — the bridge indexing its own private state.
    // 写入路径自己的可恢复回收目录在 `.xirang/` 下，而里面就是 `.rs` 文件；没有这道过滤，一个被
    // 删掉的东西会一直从它的备份里回答 `status` 与 `search`——桥在索引自己的私有状态。
    paths.retain(|path| {
        !path
            .components()
            .any(|component| component.as_os_str() == xirang_kernel::lexicon::XIRANG_DIR)
    });
    Ok(())
}

/// One contract line's text, without its indentation and without its `///` markers.
/// 一条契约行的文本：去掉缩进、去掉 `///` 标记。
///
/// The trim order is the whole point, and it was measured twice. A doc line inside an `impl` is
/// indented, so `trim_start_matches('/')` alone does nothing and the rendered text keeps its `///` —
/// the W6 round hit this once in `digest`, and the branch column's new contract note hit it again the
/// first time it ran on a real fixture. It lives here so a third reader cannot re-learn it.
/// 顺序就是要点，而且量到过两次。`impl` 里的文档行是**缩进**的，因此单用 `trim_start_matches('/')`
/// 什么也去不掉，渲染出来的文本会留着 `///`——W6 那轮在 `digest` 上撞过一次，分支栏新加的契约注第一次
/// 在真夹具上跑又撞了一次。它住在这里，好让第三个读者不必再学一遍。
pub(crate) fn contract_text(line: &str) -> String {
    line.trim_start().trim_start_matches('/').trim().to_owned()
}

/// The `///` lines directly above a definition, with the line each sits on.
/// 定义正上方的 `///` 行，以及每一行自己的行号。
///
/// One implementation for every reader that shows a contract: `digest` prints the first line in a
/// row, `why` prints a window of them, and the census's branch column prints them beside a dead arm.
/// Three readers used to mean two copies plus a third rendering; the copies had already drifted
/// (`#![` versus `#[` in the stop condition).
/// 每个展示契约的读者共用这一份：`digest` 在行里印首行，`why` 印一窗，总账的分支栏在死臂旁边印它们。
/// 三个读者过去意味着两份副本外加第三种渲染，而两份副本**已经开始漂移**（停止条件一个写 `#![`、
/// 另一个写 `#[`）。
pub(crate) fn contract_lines(source: &str, definition: usize) -> Vec<(usize, String)> {
    let lines = source.lines().collect::<Vec<_>>();
    contract_lines_from(&lines, definition)
}

/// The same contract extraction over an already split file, shared by ranked source readers.
/// 对已经切行的文件做同一份契约提取，供源码排序读取共用。
pub(crate) fn contract_lines_from(lines: &[&str], definition: usize) -> Vec<(usize, String)> {
    let mut first = definition.saturating_sub(1).min(lines.len());
    while first > 0 {
        let above = lines[first - 1].trim_start();
        if above.starts_with("///") || above.starts_with("#[") {
            first -= 1;
        } else {
            break;
        }
    }
    lines[first..definition.saturating_sub(1).min(lines.len())]
        .iter()
        .enumerate()
        .filter(|(_, line)| line.trim_start().starts_with("///"))
        .map(|(offset, line)| (first + offset + 1, (*line).to_owned()))
        .collect()
}

pub(crate) fn load_one(root: &Path, relative: &str) -> Result<SourceFile, String> {
    let path = root.join(relative);
    if !is_safe_child(root, &path) {
        // The refusal names the base it checked and the rule that makes the base matter. Measured
        // in the second simulation round: a member-relative path and a tree-relative path look
        // alike, and the old sentence ("inside the configured source root") did not say *which*
        // root `root` had just changed the paths to be relative to, so the caller's next guess was
        // another tree-relative path.
        // 拒绝话术点名它检查的基准，以及让基准变得要紧的那条规则。第 2 轮模拟评测实测：成员相对路径与
        // 树相对路径长得一样，而旧句子（"inside the configured source root"）没说 `root` 刚把路径改成
        // 相对**哪个**根，于是调用方的下一次尝试还是树相对路径。
        return Err(format!(
            "path must stay inside the configured source root ({}); with `root` set, a path is \
             relative to that root, so write it as that root sees it (`src/…`)",
            root.display()
        ));
    }
    if path.extension().and_then(|ext| ext.to_str()) != Some("rs") {
        return Err("only Rust source files can be read".to_owned());
    }
    load_file(root, &path)
}

/// Read a text file the tree carries: the ledger, a manifest, a build log.
/// 读取这棵树携带的一份文本文件：台账、清单、构建日志。
///
/// `read` used to refuse anything that was not Rust source, and the one file the second question was
/// entirely about — `.xirang/adopted/entries` — is not Rust. The arm went to `cat`, which is
/// outside this tool's `--log`, so the ledger work left the account altogether (W8, h2 step 42: "The
/// `read` tool refuses non-Rust files (exit 1). Fine — I used `cat` for the ledger").
/// The rule now: **any UTF-8 text file inside the root is readable; a `.rs` file additionally gets
/// the symbol index**. The refusal that remains is the one that is about the file rather than about
/// its extension, and it names what `read` can print so the caller's next move is not a guess.
/// `read` 过去拒绝一切非 Rust 源码的东西，而第二个问题整个围绕的那份文件——`.xirang/adopted/entries`
/// ——恰恰不是 Rust。那一臂于是去用 `cat`，而 `cat` 在本工具的 `--log` 之外，于是台账那段工作整个离开了
/// 账本（W8 h2 第 42 步原文：「The `read` tool refuses non-Rust files (exit 1). Fine — I used `cat` for
/// the ledger」）。现在的规则是：**根内的任何 UTF-8 文本文件都可读；`.rs` 文件额外给出符号索引**。
/// 保留下来的拒绝是针对**文件本身**而不是扩展名的，并且它会说出 `read` 能打印什么，好让调用方的下一步
/// 不是猜。
pub(crate) fn load_text(root: &Path, relative: &str) -> Result<(String, String), String> {
    let path = root.join(relative);
    if !is_safe_child(root, &path) {
        return Err(format!(
            "path must stay inside the configured source root ({}); with `root` set, a path is \
             relative to that root, so write it as that root sees it (`src/…`)",
            root.display()
        ));
    }
    let bytes =
        std::fs::read(&path).map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    let source = String::from_utf8(bytes).map_err(|_| {
        format!(
            "{relative} is not UTF-8 text, and `read` prints numbered text lines — accepted shapes \
             are a window around `line`, an explicit `lines` range, or `whole: true`; for a binary \
             artifact ask the tool that produced it"
        )
    })?;
    let shown = path
        .strip_prefix(root)
        .unwrap_or(&path)
        .to_string_lossy()
        .replace('\\', "/");
    Ok((shown, source))
}

fn load_file(root: &Path, path: &Path) -> Result<SourceFile, String> {
    let source = read_source(root, path)?;
    let relative = portable_path(
        path.strip_prefix(root)
            .map_err(|_| "source path escaped root".to_owned())?,
    );
    Ok(index_file(relative, source))
}

/// The text of one indexed file, after the path-safety check.
/// 一个被索引文件的文本，已过路径安全检查。
///
/// The `strip_prefix` at the call sites only names the file; this is the check that decides whether it
/// may be read at all, and it resolves symbolic links, which a prefix comparison cannot: a link inside
/// the root that points out of it passes the prefix test and fails this one.
/// 调用点的 `strip_prefix` 只用来给文件命名；决定它是否可读的是这道检查，而它解析符号链接——前缀比较
/// 做不到：根内指向根外的链接能通过前缀检查，但过不了这一道。
fn read_source(root: &Path, path: &Path) -> Result<String, String> {
    if !is_safe_child(root, path) {
        return Err(format!(
            "{} cannot be resolved inside the configured source root",
            path.display()
        ));
    }
    fs::read_to_string(path).map_err(|error| format!("read {}: {error}", path.display()))
}

/// One indexed entry: the text plus the two lexical readings this index carries.
/// 一个被索引的条目：文本，加上这份索引搬运的两种词法读数。
fn index_file(relative: String, source: String) -> SourceFile {
    let facts = crate::source_cache::facts(&source);
    SourceFile {
        branches: facts.branches(&source),
        functions: functions_from(&facts, &source),
        relative,
        source,
    }
}

/// Render a tree-relative path with `/` separators on every platform.
/// 在任何平台上都以 `/` 分隔符渲染树内相对路径。
///
/// The rule is not re-derived here: this forwards to the kernel's one
/// implementation, so a file whose name contains a backslash cannot be spelled
/// one way by this module and another way by a caller that reports the same file
/// (`preview`'s declaration anchor, `converge_trace`'s recorded paths). The
/// kernel folds for the same reason it does at display: identity uses the raw
/// bytes, so only what is *shown* may be normalized.
/// 这条规则不在这里重新推导：它转发到内核里唯一的实现，因此名字里含反斜杠的文件不会被本模块
/// 和报告同一个文件的调用方（`preview` 的声明锚点、`converge_trace` 的记录路径）拼成两种样子。
/// 内核做折叠的理由与显示时相同：身份用的是原始字节，只有**展示**出来的东西才可以归一化。
pub(crate) fn portable_path(path: &Path) -> String {
    xirang_kernel::declaration::portable_path(&path.to_string_lossy())
}

/// Resolve a child against an already canonical root. Every read-side containment check shares this rule.
/// 对已经规范化的根解析子路径，所有读取侧包含检查共用此规则。
fn canonical_child(root: &Path, path: &Path) -> Option<PathBuf> {
    let path = fs::canonicalize(path).ok()?;
    path.starts_with(root).then_some(path)
}

pub(crate) fn is_safe_child(root: &Path, path: &Path) -> bool {
    fs::canonicalize(root)
        .ok()
        .and_then(|root| canonical_child(&root, path))
        .is_some()
}

pub(crate) fn resolve_root(base: &Path, requested: Option<&str>) -> Result<PathBuf, String> {
    let base = fs::canonicalize(base)
        .map_err(|error| format!("source root does not exist: {} ({error})", base.display()))?;
    let candidate = requested.map_or_else(|| base.clone(), |value| base.join(value));
    let candidate = fs::canonicalize(&candidate).map_err(|error| {
        format!(
            "requested root is not readable: {} ({error})",
            candidate.display()
        )
    })?;
    if !candidate.starts_with(&base) {
        return Err("requested root must stay inside XIRANG_PACKAGE_ROOT".to_owned());
    }
    if !candidate.is_dir() {
        return Err(format!(
            "requested root is not a directory: {}",
            candidate.display()
        ));
    }
    Ok(candidate)
}

pub(crate) fn parse_functions(source: &str) -> Vec<Function> {
    let facts = crate::source_cache::facts(source);
    functions_from(&facts, source)
}

/// Parse one already-read source's functions and branches in place.
/// 原地解析一份已读源码的函数与分支。
///
/// Shared by the corpus loader and the callgraph's second pass: the second pass has the text in
/// hand, so re-reading and re-walking the tree to parse it again is the cost this exists to avoid.
/// 语料加载器与调用图的第二趟共用：第二趟手上已经有文本，再读一遍、再遍历一遍树去解析它就是这里要避免的
/// 成本。
pub(crate) fn parse_in_place(file: &mut SourceFile) {
    let facts = crate::source_cache::facts(&file.source);
    file.functions = functions_from(&facts, &file.source);
    file.branches = facts.branches(&file.source);
}

fn functions_from(facts: &crate::source_cache::Unit, source: &str) -> Vec<Function> {
    // `syn` is the expensive half and is content-addressed: a record keyed by the exact bytes and
    // this bridge's version is what a fresh process reads instead of re-parsing the file.
    // `syn` 是贵的那一半，且按内容寻址：以确切字节与本桥版本为键的记录，正是新进程用来替代重解析的东西。
    let semantic = match crate::mcp::semantic_cache::cached(source) {
        Some(crate::mcp::semantic_cache::Cached::Parsed(functions)) => Some(functions),
        Some(crate::mcp::semantic_cache::Cached::Unparsable) => None,
        None => {
            let parsed = crate::mcp::semantic_sources::rust(source);
            crate::mcp::semantic_cache::remember(source, parsed.as_deref());
            parsed
        }
    };
    if let Some(semantic) = semantic {
        return semantic
            .into_iter()
            .map(|function| Function {
                name: function.name,
                owner: function.owner,
                qualified: function.qualified,
                line: function.line,
                end_line: function.end_line,
                calls: function.calls,
                unresolved: function.unresolved,
                caveats: function.caveats,
                semantic: true,
                registration: None,
                binding: None,
            })
            .collect();
    }
    facts
        .functions
        .iter()
        .zip(&facts.calls)
        .map(|(function, calls)| Function {
            name: function.name.clone(),
            owner: None,
            qualified: function.name.clone(),
            calls: calls.clone(),
            unresolved: Vec::new(),
            caveats: Vec::new(),
            semantic: false,
            registration: None,
            binding: None,
            line: function.line as usize,
            end_line: function.end_line as usize,
        })
        .collect()
}

pub(crate) fn display_list(items: &[String]) -> String {
    if items.is_empty() {
        "-".to_owned()
    } else {
        items.join(", ")
    }
}

#[cfg(test)]
#[path = "source_index_tests.rs"]
mod source_index_tests;
