//! Throwaway package copies and the diff a preview reports.
//! 一次性包副本与预览报告的 diff。
//!
//! Copying, rather than writing and reverting, is what makes a preview safe: if
//! anything goes wrong between the two, the project was never touched. The copy
//! skips the directories that are neither an input to an edit nor small: the build
//! output `target/`, the version-control store `.git/`, and XiRang's own runtime
//! data `.xirang/` — the same three the diff walk skips, because both walks decide
//! one thing.
//! 用复制而不是"先写再回滚"，正是预览安全的原因：两者之间无论哪里出错，项目从未被碰过。副本跳过
//! 那些既不是编辑输入、又不见得小的目录：构建产物 `target/`、版本库 `.git/`，以及 XiRang 自己的
//! 运行期数据 `.xirang/`——与 diff 遍历跳过的是同三个，因为两处遍历决定的是同一件事。

use std::path::{Path, PathBuf};

/// A throwaway copy of the package, so a preview cannot touch the project.
/// 包的一次性副本，因此预览碰不到项目。
///
/// The build output, the version-control store, and XiRang's own runtime data are
/// skipped: they are not inputs to the edit, and any of the three can be large
/// (`.git/` alone runs to gigabytes in a real repository). The earlier wording named
/// only the build output, which is why every preview copied that store too and walked
/// it into the diff (audit `BR-C2`).
/// 构建产物、版本库与 XiRang 自己的运行期数据都被跳过：它们不是编辑的输入，而这三者中任何一个
/// 都可能很大（真实仓库里仅 `.git/` 就常是 GB 级）。早先的措辞只点名了构建产物，于是每一次预览都
/// 连带复制了那个库、并把它带进 diff（审计 `BR-C2`）。
pub(crate) fn copy_package(root: &Path) -> Result<PathBuf, String> {
    copy_package_with(root, &std::env::temp_dir(), false, |attempt| {
        // The destination is created *exclusively*, and a failure removes whatever this
        // call created. The earlier form deleted a same-named directory first and, when
        // the copy itself failed, left the half-made copy behind: a throwaway copy has to
        // disappear on the failure path too, and pre-existing data at a predictable path
        // must never be deleted on the way in (audit `LGC-LG-20`).
        // 目标目录是**独占**创建的，失败时清掉本次调用建出来的东西。早先的形式先删掉同名目录，而在复制
        // 自身失败时把半成品留在原地：一次性副本在失败路径上同样该消失，而可预测路径上的既有数据也绝不该
        // 在入口被删掉（审计 `LGC-LG-20`）。
        format!(
            "xirang-toolchain-preview-{}-{:016x}-{attempt}",
            std::process::id(),
            entropy()
        )
    })
}

/// Copy `root` into a freshly created directory under `temp`, whose file name `candidate`
/// produces for each attempt.
/// 把 `root` 复制进 `temp` 下一个新建的目录，其文件名由 `candidate` 按每次尝试产出。
///
/// One attempt per candidate, and an existing directory is *skipped* rather than reused or
/// deleted: the destination is created with `create_dir`, which fails on an existing path,
/// and the next candidate is tried. The candidate is injectable so the pin can pre-place a
/// directory at exactly the name the first attempt asks for and prove both halves — that
/// nothing existing is touched, and that the copy lands somewhere else (audit `LGC-LG-20`).
/// 每个候选一次尝试，而已存在的目录是被**跳过**的，既不复用也不删除：目标用 `create_dir` 创建，
/// 路径已存在时它会失败，于是试下一个候选。候选名可注入，因此钉子可以在**第一次尝试会用的那个名字**
/// 上预放一个目录，并证明两件事——既有东西一点没被碰，而副本落在了别处（审计 `LGC-LG-20`）。
///
/// **At most eight attempts, and that is an observable contract, not an implementation detail.**
/// Eight candidates are tried one by one; when every one of them already exists the call is
/// refused with `cannot create a preview copy under <temp>: <the last collision>` instead of
/// retrying forever, and none of the eight existing directories is touched on the way. A caller
/// that sees that refusal knows the temp directory is full of this process's own leftovers, and
/// a caller that sees a preview knows exactly one of the eight was created and used.
/// **最多 8 次尝试，而这是可观测的契约，不是实现细节。** 八个候选被逐个尝试；当它们**全部**已存在
/// 时，调用以 `cannot create a preview copy under <temp>: <最后一次碰撞>` 被拒绝，而不是无限重试，且
/// 这八个既有目录一路都没被碰过。看到这条拒绝的调用方就知道临时目录里堆满了本进程自己的残留；而看到
/// 一次成功预览的调用方就知道八个候选里恰好有一个被创建并使用了。
/// A copy that also carries the package's own machine-local state files (bindings, and only the
/// regular files directly under `.xirang/`).
/// 一份同时带上包自己机器本地状态文件的副本（绑定，且只带 `.xirang/` 下的普通文件）。
///
/// The pre-check that runs a change on a copy before an apply needs the same inputs the apply will
/// have: a declared source is resolved through `.xirang/source-bindings.tsv`, and a copy without it
/// refuses the change for the wrong reason ("the source does not exist") instead of the real one.
/// Subtrees under `.xirang/` (trash, candidates) are deliberately not copied: they are not inputs
/// to the edit and can be large.
/// 在落盘前把改动跑在副本上的预检需要与落盘相同的输入：已声明来源经 `.xirang/source-bindings.tsv`
/// 解析，没有它的副本会因错误的原因（"来源不存在"）而不是真正的原因拒绝改动。`.xirang/` 下的子树
/// （回收、候选）有意不复制：它们不是这次编辑的输入，而且可能很大。
pub(crate) fn copy_package_with_state(root: &Path) -> Result<PathBuf, String> {
    copy_package_with(root, &std::env::temp_dir(), true, |attempt| {
        format!(
            "xirang-toolchain-precheck-{}-{:016x}-{attempt}",
            std::process::id(),
            entropy()
        )
    })
}

fn copy_package_with(
    root: &Path,
    temp: &Path,
    include_state: bool,
    mut candidate: impl FnMut(u32) -> String,
) -> Result<PathBuf, String> {
    let mut last = None;
    for attempt in 0..8 {
        let destination = temp.join(candidate(attempt));
        match std::fs::create_dir(&destination) {
            Ok(()) => return copy_into(root, destination, include_state),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                last = Some(error);
            }
            Err(error) => {
                return Err(format!("cannot create {}: {error}", destination.display()));
            }
        }
    }
    Err(format!(
        "cannot create a preview copy under {}: {}",
        temp.display(),
        last.map_or_else(String::new, |error| error.to_string())
    ))
}

/// An OS-seeded value, so the destination name cannot be predicted — and therefore pre-placed
/// — by another process on the same machine. `RandomState` draws its per-process seed from the
/// OS and every instance differs, which is the portable stand-in for a `tempfile`-style random
/// name while this crate has no random dependency (audit `LGC-LG-20`).
/// 一个由操作系统播种的值，因此同一台机器上的另一个进程无法预测——也就无法预先放置——这个目标名。
/// `RandomState` 的进程级种子取自操作系统，且每个实例都不同；在本 crate 没有随机依赖的前提下，这就是
/// `tempfile` 式随机名的可移植替身（审计 `LGC-LG-20`）。
fn entropy() -> u64 {
    use std::hash::{BuildHasher, Hasher};
    std::collections::hash_map::RandomState::new()
        .build_hasher()
        .finish()
}

/// Copy `root` into an already-created `destination`, removing that directory again when
/// the copy fails.
/// 把 `root` 复制进已创建好的 `destination`，复制失败时再把这个目录移除。
///
/// The failure path is part of the throwaway contract: a preview that leaves a half-made
/// package behind under the temp directory is not throwaway, and the copy used to do
/// exactly that (measured: seven files of a package; audit `LGC-LG-20`). The destination
/// is created by the caller, so this is also the seam the pin drives directly instead of
/// scanning the temp directory.
/// 失败路径属于一次性契约的一部分：在临时目录里留下半成品包的预览不是一次性的，而复制过去正是
/// 这么做的（实测：一个包的七个文件；审计 `LGC-LG-20`）。目标目录由调用方创建，因此这里也是钉子
/// 直接驱动的接缝，不必去扫临时目录。
fn copy_into(root: &Path, destination: PathBuf, include_state: bool) -> Result<PathBuf, String> {
    match copy_directory(root, &destination, 0, include_state) {
        Ok(()) => Ok(destination),
        Err(error) => {
            let _ = std::fs::remove_dir_all(&destination);
            Err(error)
        }
    }
}

/// Whether a directory name is one the preview copy and its diff both skip.
/// 一个目录名是否属于预览副本与其 diff 都跳过的那一类。
///
/// One list, because the two walks answer the same question: a directory the copy
/// skipped but the diff did not would be reported as removed, and one the diff
/// skipped but the copy did not is invisible by construction — either way the report
/// would describe a change the edit never made (audit `BR-C2`, `BR-4`).
/// 只有一份清单，因为两处遍历回答的是同一个问题：副本跳过而 diff 没跳过的目录会被报成删除，反过来
/// 的目录则从一开始就看不见——两种都会让报告描述一次编辑根本没做过的改动（审计 `BR-C2`、`BR-4`）。
fn skipped_directory(name: &std::ffi::OsStr) -> bool {
    name == "target" || name == ".git" || name == xirang_kernel::lexicon::XIRANG_DIR
}

/// How deep either walk follows directories before refusing to go further.
/// 两处遍历在拒绝继续之前最多跟进多少层目录。
///
/// A depth cap, not a symlink cap: a package with a genuine 70-deep tree is absurd
/// enough that refusing it with a reason beats walking it. Symlinks are not followed
/// at all (see `copy_directory`), so this only guards a pathological real tree.
/// 这是深度上限而不是符号链接上限：真有 70 层深的包荒唐到拒绝并给出理由比遍历它更好。符号链接
/// 一律不跟进（见 `copy_directory`），因此这里守的只是病态的真实目录树。
const MAX_DEPTH: usize = 64;

fn copy_directory(from: &Path, to: &Path, depth: usize, include_state: bool) -> Result<(), String> {
    if depth > MAX_DEPTH {
        return Err(format!(
            "cannot copy {}: deeper than {MAX_DEPTH} directories",
            from.display()
        ));
    }
    std::fs::create_dir_all(to)
        .map_err(|error| format!("cannot create {}: {error}", to.display()))?;
    let entries = std::fs::read_dir(from)
        .map_err(|error| format!("cannot read {}: {error}", from.display()))?;
    for entry in entries {
        let entry = entry.map_err(|error| format!("cannot read a directory entry: {error}"))?;
        let name = entry.file_name();
        let source = entry.path();
        let destination = to.join(&name);
        if name == xirang_kernel::lexicon::XIRANG_DIR {
            // The package's own state directory: skipped by a preview (it is not an input to the
            // edit), but the pre-check copy needs its **files** — bindings decide how a declared
            // source resolves, and without them the copy refuses for the wrong reason. Subtrees
            // (trash, candidates) stay out: they are not inputs and can be large.
            // 包自己的状态目录：预览跳过它（它不是编辑的输入），但预检副本需要它的**文件**——绑定决定
            // 已声明来源如何解析，缺了它副本会因错误的原因拒绝。子树（回收、候选）不进来：不是输入，且可能很大。
            if include_state {
                std::fs::create_dir_all(&destination)
                    .map_err(|error| format!("cannot create {}: {error}", destination.display()))?;
                copy_state_files(&source, &destination)?;
            }
            continue;
        }
        if skipped_directory(&name) {
            continue;
        }
        let linked = std::fs::symlink_metadata(&source)
            .map(|meta| meta.file_type().is_symlink())
            .unwrap_or(false);
        if linked {
            // A preview copy skips a link to a directory (following it would copy an outside tree,
            // and a self-link recurses — audit `LGC-LG-19`). The pre-check copy instead **recreates
            // the link**: it stands in for a write into the project, and a declared source is
            // resolved by canonicalizing the project-relative location, so a dropped link turns
            // "this is a declared source" into "this source does not exist" — the wrong refusal.
            // Recreating the link copies no outside bytes and keeps the resolution identical.
            // 预览副本跳过指向目录的链接（跟进它会把包外树复制进来，自指链接会递归——审计 `LGC-LG-19`）。
            // 预检副本则**重建链接**：它代表一次对项目的写入，而已声明来源靠规范化项目内位置来解析，丢掉的
            // 链接会把"这是一个已声明来源"变成"来源不存在"——错误的拒绝。重建链接不复制任何包外字节，且让
            // 解析与项目一致。
            if include_state {
                #[cfg(unix)]
                {
                    let target = std::fs::read_link(&source).map_err(|error| {
                        format!("cannot read link {}: {error}", source.display())
                    })?;
                    std::os::unix::fs::symlink(&target, &destination).map_err(|error| {
                        format!("cannot recreate link {}: {error}", destination.display())
                    })?;
                }
            }
            continue;
        }
        if source.is_dir() {
            copy_directory(&source, &destination, depth + 1, include_state)?;
        } else if source.is_file() {
            std::fs::copy(&source, &destination)
                .map_err(|error| format!("cannot copy {}: {error}", source.display()))?;
        }
    }
    Ok(())
}

/// Copy the regular files directly under one state directory, without following its subtrees.
/// 复制一个状态目录下直接放着的普通文件，不跟进它的子树。
fn copy_state_files(from: &Path, to: &Path) -> Result<(), String> {
    let entries = std::fs::read_dir(from)
        .map_err(|error| format!("cannot read {}: {error}", from.display()))?;
    for entry in entries {
        let entry = entry.map_err(|error| format!("cannot read a directory entry: {error}"))?;
        let source = entry.path();
        if source.is_file() {
            let destination = to.join(entry.file_name());
            std::fs::copy(&source, &destination)
                .map_err(|error| format!("cannot copy {}: {error}", source.display()))?;
        }
    }
    Ok(())
}

/// Whether `path` is a symbolic link to a directory.
/// `path` 是否是一个指向目录的符号链接。
///
/// A preview copy must never follow one out of the package: a link to an outside tree
/// copies that tree into the scratch directory (data exposure), and a self-link
/// (`ln -s . src/self`) recurses until the path limit and reports the same faces
/// dozens of times — both measured, both silent (audit `LGC-LG-19`). Skipping the
/// link keeps the copy inside the package; a link *file* is copied as the link's
/// target only when it is a plain file, which `std::fs::copy` already resolves.
/// 预览副本绝不能顺着链接走出包外：指向外部树的链接会把那棵树复制进临时目录（数据外泄），而自指链接
/// （`ln -s . src/self`）会递归到路径上限、把同一批面报上几十次——两条都实测过，且都无声
/// （审计 `LGC-LG-19`）。跳过链接使副本留在包内；链接*文件*只有在它是普通文件时才会被
/// `std::fs::copy` 解析着复制。
fn is_symlinked_directory(path: &Path) -> bool {
    std::fs::symlink_metadata(path)
        .map(|meta| meta.file_type().is_symlink() && path.is_dir())
        .unwrap_or(false)
}

pub(crate) fn remove_copy(root: &Path, work: &Path) {
    if work != root {
        let _ = std::fs::remove_dir_all(work);
    }
}

/// A fresh throwaway directory under the temp directory, for a preview whose work is
/// not a copy of an existing tree.
/// 临时目录下一个新建的一次性目录，供"工作内容不是既有树的副本"的预览使用。
///
/// `copy_package` serves a preview of an *edit*: the work is the project, copied. A
/// scaffold has no project to copy — it creates one — so its preview needs a directory
/// of its own to run the same executor in, and gets it here. The name carries the
/// process id and an OS-seeded value for the same reason the copy's does
/// (unpredictable, so nothing can pre-place a directory there), and the directory is
/// created exclusively: an existing path is a failure rather than a directory this
/// call would reuse or delete.
/// `copy_package` 服务的是**编辑**的预览：工作内容是被复制过来的项目。脚手架没有项目可复制
/// ——它就是去创建一个——因此它的预览需要一个自己的目录来运行同一个执行器，就由这里给出。名字里带
/// 进程号与一个由操作系统播种的值，理由与副本相同（不可预测，因此没有东西能预先在那里放一个目录），
/// 而目录是**独占**创建的：已存在的路径是一次失败，而不是本次调用会复用或删掉的目录。
pub(crate) fn work_directory(label: &str) -> Result<PathBuf, String> {
    let destination = std::env::temp_dir().join(format!(
        "xirang-toolchain-{label}-{}-{:016x}",
        std::process::id(),
        entropy()
    ));
    std::fs::create_dir(&destination)
        .map_err(|error| format!("cannot create {}: {error}", destination.display()))?;
    Ok(destination)
}

/// Where a written face's declaration sits, as `<path>:<line>`.
/// 被写入的面的声明位置，写作 `<path>:<line>`。
///
/// The executor validates and reports refusals with a position (the kernel's
/// diagnostics carry `file:line:column`), so a *successful* write has to answer the
/// same question or the caller has to guess: a face's declaration is the macro
/// invocation, which is the line an editor or a follow-up edit wants.
/// 执行器在拒绝时带着位置报告（内核诊断携带 `file:line:column`），因此**成功**的写入必须回答同一个
/// 问题，否则调用方只能猜：一个面的声明就是那次宏调用，而那正是编辑器或后续编辑想要的那一行。
pub(crate) fn declaration_line(file: &Path, relative: &Path) -> Option<String> {
    let text = std::fs::read_to_string(file).ok()?;
    // The **declaration's** line, not the macro's: a face written as a `Registration` literal has no macro,
    // and `location` is then unset — reading it put every literal face's anchor on line 1 (measured).
    // **声明**所在的行，而不是宏所在的行：写成 `Registration` 字面量的面没有宏，`location` 于是没被填——
    // 读它会把每个字面量面的锚点放到第 1 行（实测）。
    let parsed = xirang_kernel::syntax::parse_face(&text).ok()??;
    let line = parsed.declaration_start.line;
    // Forward slashes, like every other tree-relative path this bridge reports
    // (`FaceView.source`, the diff headers): a caller comparing the anchor with a
    // path out of `xirang.registry` must not have to know which platform produced
    // it. The absolute path in the line above stays native, like `status`'s root.
    // 正斜杠，与本桥报告的每一条树内相对路径一致（`FaceView.source`、diff 头）：把锚点与
    // `xirang.registry` 给出的路径相比的调用方，不该需要知道它由哪个平台产生。上面那行的绝对
    // 路径保持本机写法，与 `status` 的 root 一致。
    let relative = crate::mcp::source_index::portable_path(relative);
    Some(format!("{relative}:{line}"))
}

/// A line diff between the project and the copy it was previewed in.
/// 项目与它据以预览的副本之间的逐行 diff。
///
/// Deliberately not a diff *algorithm*: the common prefix and suffix are trimmed
/// and everything between them is printed as removed then added. That cannot
/// mislabel a line as unchanged (the part it claims is common really is), and for
/// the files these edits touch — a module source and its registry rule — a
/// matching diff would mostly cost code. A caller that needs precise hunks should
/// read the written file, whose path this report gives exactly.
/// 有意不做 diff **算法**：裁掉公共前缀与公共后缀，中间部分先按删除、再按新增打印。它不会把某行
/// 误标为未变（它声明公共的部分确实是公共的），而对这些编辑会碰的文件——一个模块源与其注册规则
/// ——真正的匹配式 diff 基本只是在多写代码。需要精确 hunk 的调用方应当去读被写入的那个文件，
/// 本报告会给出它的准确路径。
pub(crate) fn diff_package(original: &Path, work: &Path) -> Result<String, String> {
    let mut files = Vec::new();
    visit_files(work, work, &mut files, 0)?;
    files.sort();
    let mut output = String::new();
    let mut printed = 0usize;
    let mut truncated = false;
    for relative in files {
        // Bytes decide "unchanged", not text: two byte-identical non-UTF-8 files used
        // to be reported as added because both `read_to_string` calls failed, so an
        // edit that never touched them listed them as files that would change
        // (audit `LGC-LG-23`). Identical bytes are identical however they decode.
        // 判断"没变"靠字节而不是文本：两个逐字节相同的非 UTF-8 文件过去因为两侧 `read_to_string`
        // 都失败而被报成新增，于是一次根本没碰过它们的编辑把它们列进"会变的文件"（审计
        // `LGC-LG-23`）。逐字节相同的文件，无论怎么解码都是相同的。
        let Ok(after) = std::fs::read(work.join(&relative)) else {
            continue;
        };
        match std::fs::read(original.join(&relative)) {
            Ok(before) if before == after => continue,
            Ok(before) => {
                let (before_len, after_len) = (before.len(), after.len());
                match (String::from_utf8(before), String::from_utf8(after)) {
                    (Ok(before), Ok(after)) => {
                        push_diff(
                            &mut output,
                            &format!("~ {relative}"),
                            &mut printed,
                            &mut truncated,
                        );
                        push_diff(
                            &mut output,
                            &line_diff(&before, &after),
                            &mut printed,
                            &mut truncated,
                        );
                    }
                    // A changed file whose bytes are not text is named, not dumped: its
                    // bytes are the change, and printing them as lines would invent lines
                    // that do not exist.
                    // 内容变了而字节不是文本的文件只被点名、不被倾倒：变化就是那些字节，把它们按行
                    // 打印会造出并不存在的行。
                    _ => push_diff(
                        &mut output,
                        &format!("~ {relative} (binary, {before_len} bytes -> {after_len} bytes)"),
                        &mut printed,
                        &mut truncated,
                    ),
                }
            }
            Err(_) => match String::from_utf8(after) {
                Ok(after) => {
                    push_diff(
                        &mut output,
                        &format!("+ {relative}"),
                        &mut printed,
                        &mut truncated,
                    );
                    let added = after
                        .lines()
                        .map(|line| format!("+{line}"))
                        .collect::<Vec<_>>()
                        .join("\n");
                    push_diff(&mut output, &added, &mut printed, &mut truncated);
                }
                Err(error) => push_diff(
                    &mut output,
                    &format!("+ {relative} (binary, {} bytes)", error.as_bytes().len()),
                    &mut printed,
                    &mut truncated,
                ),
            },
        }
        if truncated {
            break;
        }
    }
    // A file the edit *removed* is part of the answer too.
    // 被编辑**删除**的文件同样是答案的一部分。
    let mut before = Vec::new();
    visit_files(original, original, &mut before, 0)?;
    before.sort();
    for relative in before {
        if truncated {
            break;
        }
        if !original.join(&relative).exists() {
            continue;
        }
        if !work.join(&relative).exists() {
            push_diff(
                &mut output,
                &format!("- {relative}"),
                &mut printed,
                &mut truncated,
            );
        }
    }
    if truncated {
        // Every other answer this bridge gives is bounded and says so; the preview was
        // the one that could not, which is how a real checkout produced a reply an agent
        // could not read (audit `LGC-LG-23`).
        // 本桥其它每个答案都有上限并说明被截断；预览曾是唯一没有上限的那个，于是一次真实检出能产出
        // 代理读不下的回复（审计 `LGC-LG-23`）。
        //
        // The count is the one fact this site cannot give: the walk stops *at* the cap, so
        // the lines past it were never produced, and counting them would walk the whole
        // change the cap exists to avoid. The sentence says so rather than naming no
        // number at all.
        // 数量是本站点唯一给不出的事实：遍历在到达上限处停下，因此上限之外的行从未被产出，而要数出
        // 它们就得走完整份改动——那正是上限要避免的。那句话把这一点说出来，而不是一个数字都不给。
        let byte_cap = format!("a preview also stops at {MAX_DIFF_BYTES} bytes");
        output.push_str(&format!(
            "{}\n",
            crate::mcp::truncation::withheld_uncounted(
                MAX_DIFF_LINES,
                "diff lines",
                &format!("{byte_cap}; read the written files for the rest")
            )
        ));
    }
    Ok(output)
}

/// The most diff lines one preview prints before it says it truncated.
/// 一条预览在声明被截断之前最多打印多少行 diff。
///
/// A line cap alone is not a size cap: one pathologically long line is still one line, so the
/// byte cap below is the one that bounds the reply whatever the file looks like (audit
/// `LGC-LG-23`).
/// 只按行设上限不等于按大小设上限：一行病态长的行仍然只是一行，因此下面那个字节上限才是无论文件长
/// 什么样都能给回复设界的那个（审计 `LGC-LG-23`）。
const MAX_DIFF_LINES: usize = 200;

/// The most diff bytes one preview prints, for the reason above.
/// 一条预览最多打印多少字节的 diff，理由同上。
const MAX_DIFF_BYTES: usize = 64 * 1024;

/// Append one chunk of diff lines, stopping at the cap and recording that it stopped.
/// 追加一段 diff 行，到上限就停，并记下自己停过。
fn push_diff(output: &mut String, chunk: &str, printed: &mut usize, truncated: &mut bool) {
    for line in chunk.lines() {
        if *printed >= MAX_DIFF_LINES || output.len() >= MAX_DIFF_BYTES {
            *truncated = true;
            return;
        }
        let remaining = MAX_DIFF_BYTES
            .saturating_sub(output.len())
            .saturating_sub(1);
        if line.len() > remaining {
            // The line alone is past the byte cap: print as much of it as fits on a character
            // boundary and stop, rather than handing the client a single unbounded line.
            // 这一行自己就超过了字节上限：在字符边界上打印能装下的部分后停下，而不是把一行无界的
            // 内容交给客户端。
            let mut cut = remaining.min(line.len());
            while cut > 0 && !line.is_char_boundary(cut) {
                cut -= 1;
            }
            output.push_str(&line[..cut]);
            output.push('\n');
            *truncated = true;
            return;
        }
        output.push_str(line);
        output.push('\n');
        *printed += 1;
    }
}

fn visit_files(
    root: &Path,
    directory: &Path,
    files: &mut Vec<String>,
    depth: usize,
) -> Result<(), String> {
    if depth > MAX_DEPTH {
        return Err(format!(
            "cannot walk {}: deeper than {MAX_DEPTH} directories",
            directory.display()
        ));
    }
    let entries = std::fs::read_dir(directory)
        .map_err(|error| format!("cannot read {}: {error}", directory.display()))?;
    for entry in entries {
        let entry = entry.map_err(|error| format!("cannot read a directory entry: {error}"))?;
        let name = entry.file_name();
        if skipped_directory(&name) {
            continue;
        }
        let path = entry.path();
        if is_symlinked_directory(&path) {
            continue;
        }
        if path.is_dir() {
            visit_files(root, &path, files, depth + 1)?;
        } else if let Ok(relative) = path.strip_prefix(root) {
            files.push(crate::mcp::source_index::portable_path(relative));
        }
    }
    Ok(())
}

fn line_diff(before: &str, after: &str) -> String {
    let before = before.lines().collect::<Vec<_>>();
    let after = after.lines().collect::<Vec<_>>();
    let mut head = 0;
    while head < before.len() && head < after.len() && before[head] == after[head] {
        head += 1;
    }
    let mut tail = 0;
    while tail < before.len() - head
        && tail < after.len() - head
        && before[before.len() - 1 - tail] == after[after.len() - 1 - tail]
    {
        tail += 1;
    }
    let mut output = String::new();
    for line in &before[head..before.len() - tail] {
        output.push_str(&format!("-{line}\n"));
    }
    for line in &after[head..after.len() - tail] {
        output.push_str(&format!("+{line}\n"));
    }
    output
}

#[cfg(test)]
#[path = "preview_tests.rs"]
mod preview_tests;
