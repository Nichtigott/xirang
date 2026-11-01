//! The nesting guard must refuse nothing this repository actually contains.
//! 嵌套守卫不得拒绝本仓库里真实存在的任何东西。
//!
//! The guard's limits are deliberately far below what overflows a stack, so the
//! risk it carries is the opposite one: a threshold that refuses real source. That
//! risk cannot be settled by reading the heuristic — the only way to know is to
//! run it over every source file in the workspace and look. This test does that,
//! and it is why the third measured shape could be added at all: the linear-run
//! limit is a thousand tokens, which sounds tight until the longest run in this
//! repository turns out to be a small fraction of it.
//! 守卫的上限刻意远低于会撑爆栈的量级，因此它自带的风险是相反的那一个：某个阈值拒绝了真实
//! 源码。这个风险无法靠阅读启发式来定案——唯一的办法是把它跑遍工作区里每个源文件再看结果。
//! 本测试做的正是这件事，也正是第三种形状能被加进来的原因：线性串上限是一千个 token，听起来
//! 很紧，直到本仓库里最长的串被发现只占它的一小部分。
//!
//! **余量是实测的，不是推断的**（2026-09-28）：在全部源文件上量一个**保守上界**——剥掉注释与
//! 字面量后，只数 `(`/`[`/`{` 的配对嵌套（再加一份把 `<`/`>` 也算进去的更粗版本）——最深的
//! 括号嵌套是 **28**（`mcp/src/mir_tests.rs`），含尖括号也只有 **27**，而 [`LIMIT`] 是 **128**。
//! 真实的加权限深度只会 ≤ 这个上界（`fn`/`dyn`/`impl` 是**加倍**，不会凭空造出括号），因此守卫
//! 对今天的树有 4 倍以上余量；这条记录的意义是：`LIMIT` 不是"刚好装下今天的树"，它离树的实际
//! 形状还很远。
//! **The headroom is measured, not inferred** (2026-09-28): a *conservative upper bound* over every
//! source file — comments and literals stripped, only paired `(`/`[`/`{` nesting counted, plus a
//! cruder variant that also counts `<`/`>` — tops out at **28** (`mcp/src/mir_tests.rs`), and **27**
//! with angle brackets, against a `LIMIT` of **128**. The real weighted depth can only be ≤ that
//! bound (`fn`/`dyn`/`impl` double a level, they do not invent brackets), so the guard has more than
//! four times the room it needs: `LIMIT` is not "just big enough for today's tree".
//!
//! It lives in `core/tests/` rather than beside the unit tests because it walks the
//! filesystem, and the kernel-purity gate covers all of `core/src` — the gate's
//! own documentation names this directory as the place for a test that genuinely
//! needs one.
//! 它在 `core/tests/` 而不是单元测试旁边，因为它要遍历文件系统，而内核纯净性门禁覆盖
//! `core/src` 全部——那道门禁自己的文档就把本目录点名为"确实需要这类东西的测试"该去的地方。

// The guard only exists with the parser, so the whole file is gated with it. A
// whole-workspace build enables `syntax` through the other members; a bare
// `cargo test -p xirang-core` does not, and then this file has nothing to say.
// 守卫只随解析器存在，因此整个文件与之同门控。整工作区构建会经其他成员打开 `syntax`；
// 单跑 `cargo test -p xirang-core` 不会，那时本文件无话可说。
#![cfg(feature = "syntax")]

use std::path::{Path, PathBuf};

/// The phrase a nesting refusal carries, from `TooDeep`'s `Display`.
/// 嵌套拒绝所带的措辞，来自 `TooDeep` 的 `Display`。
const REFUSAL: &str = "above the limit of";

/// Set to `0` by the ten-face gate for every face except `default`.
/// 十面门禁对除 `default` 之外的每个面都设成 `0`。
///
/// The census is a property of the **sources**, not of a feature combination, so running it once is
/// the same promise as running it per face. Unset (a plain `cargo test`) keeps the old behavior and
/// runs it: a developer should not have to know an environment variable to get the check.
/// 普查是**源码**的性质，不是某个特性组合的性质，因此跑一次与每个面各跑一次是同一个承诺。未设置时
/// （普通 `cargo test`）保持原行为照跑：开发者不该为了获得检查而必须先知道一个环境变量。
const CENSUS_ENV: &str = "XIRANG_NESTING_CENSUS";

/// Bumped when the census's own logic changes, so an old cache cannot survive a change to the rule.
/// 普查自身逻辑改变时递增，使旧缓存不可能跨过一次规则变更继续生效。
const CENSUS_CACHE_VERSION: &str = "v1";

/// Where a successful census records its digest, under the workspace's own `target/`.
/// 成功的普查把摘要记在工作区自己的 `target/` 下。
const CENSUS_CACHE: &str = "target/nesting-census.tsv";

/// Whether this face should run the census.
/// 这个面是否应当跑普查。
fn census_enabled() -> bool {
    match std::env::var(CENSUS_ENV) {
        Ok(value) => value != "0" && value != "skip",
        Err(_) => true,
    }
}

/// The workspace members named by the root manifest, or `None` when the manifest cannot be read.
/// 根清单点名的 workspace 成员；读不到清单时为 `None`。
///
/// A minimal read of the `members = [...]` list rather than a TOML dependency: the test is about a
/// repository convention, and adding a parser to it would be a dependency for a single array.
/// 只做 `members = [...]` 的最小读取，不引入 TOML 依赖：这条测试管的是一个仓库约定，为单个数组加一个
/// 解析器不值得。
fn members_from_manifest(root: &Path) -> Option<Vec<String>> {
    let text = std::fs::read_to_string(root.join("Cargo.toml")).ok()?;
    let start = text.find("members")?;
    let open = text[start..].find('[')? + start;
    let close = text[open..].find(']')? + open;
    let mut rest = &text[open + 1..close];
    let mut members = Vec::new();
    while let Some(quote) = rest.find('"') {
        rest = &rest[quote + 1..];
        let end = rest.find('"')?;
        members.push(rest[..end].to_owned());
        rest = &rest[end + 1..];
    }
    (!members.is_empty()).then_some(members)
}

/// The directories the census walks: the workspace members, or the whole root as a fallback.
/// 普查遍历的目录：workspace 成员；读不到成员清单时退回整棵根。
///
/// The fallback is deliberate. A narrower walk must never be "the manifest changed and the census
/// silently checked less": an unreadable or wildcard member list goes back to the old, wider walk
/// and says so, where the old behavior still holds the promise.
/// 退回是有意的。更窄的遍历绝不能变成"清单变了，普查就悄悄少查"：成员清单读不到或含通配符时退回旧的
/// 更宽遍历，并说出来——那里旧行为仍然守住承诺。
fn census_roots(root: &Path) -> (Vec<PathBuf>, bool) {
    match members_from_manifest(root) {
        Some(members) if !members.iter().any(|member| member.contains('*')) => (
            members.iter().map(|member| root.join(member)).collect(),
            false,
        ),
        _ => (vec![root.to_path_buf()], true),
    }
}

/// One digest over every file the census will read: its relative path and its bytes.
/// 对普查将读的每个文件算一个摘要：相对路径与字节。
///
/// Path and content both go in: a file moved to another member with identical bytes is still a
/// different input, and a content change is what the cache must notice. A file that cannot be read
/// contributes its path but no bytes, so it cannot match a cache written when it was readable.
/// 路径与内容都进摘要：同样的字节搬到另一个成员仍是不同的输入，而内容变化正是缓存必须察觉的。读不到的
/// 文件只贡献路径、不贡献字节，因此它不可能命中"它还可读时写下的"那份缓存。
fn census_digest(files: &[PathBuf], root: &Path) -> String {
    let mut input = format!("{CENSUS_CACHE_VERSION}\n").into_bytes();
    for path in files {
        let relative = path.strip_prefix(root).unwrap_or(path);
        input.extend_from_slice(relative.to_string_lossy().as_bytes());
        input.push(0);
        if let Ok(text) = std::fs::read_to_string(path) {
            input.extend_from_slice(text.as_bytes());
        }
        input.push(0);
    }
    xirang_kernel::identity::sha256_hex(&input)
}

/// The digest a previous green census recorded, when the cache file is well-formed.
/// 上一次绿色普查记下的摘要（缓存文件格式正确时）。
fn cached_digest(root: &Path) -> Option<String> {
    let text = std::fs::read_to_string(root.join(CENSUS_CACHE)).ok()?;
    let mut fields = text.split('\t');
    let digest = fields.next()?.trim().to_owned();
    let version = fields.next()?.trim();
    (version == CENSUS_CACHE_VERSION && digest.len() == 64).then_some(digest)
}

/// Record a green census so the next face (and the next gate run) can reuse it.
/// 记录一次绿色普查，供下一个面（以及下一次门禁）复用。
fn record_census(root: &Path, digest: &str) -> std::io::Result<()> {
    let path = root.join(CENSUS_CACHE);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, format!("{digest}\t{CENSUS_CACHE_VERSION}\n"))
}

/// Every `.rs` file under `directory`, skipping build output and version control.
/// `directory` 下每个 `.rs` 文件，跳过构建输出与版本控制。
fn rust_files(directory: &Path, found: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if path.is_dir() {
            if name == "target" || name == ".git" || name == ".codegraph" {
                continue;
            }
            rust_files(&path, found);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            found.push(path);
        }
    }
}

/// What the guard says about a set of files: the ones it refused, and the ones it
/// could not read at all.
/// 守卫对一组文件的结论：它拒绝的那些，以及它根本读不到的那些。
fn inspect(files: &[PathBuf]) -> (Vec<String>, Vec<String>) {
    let mut refused = Vec::new();
    let mut unreadable = Vec::new();
    for path in files {
        let Ok(text) = std::fs::read_to_string(path) else {
            // A file the walk found but cannot read is not "clean". The guard never
            // saw it, and this test's whole point is that the guard sees everything
            // the repository ships; skipping it silently is how a budget test decays
            // into a test of whatever it happened to be able to open.
            // 遍历找到却读不到的文件不算"干净"。守卫从未见过它，而本测试的全部意义正是守卫见过
            // 仓库出厂的每一样东西；静默跳过它，就是一道预算测试退化成"只测它碰巧能打开的那些
            // 文件"的方式。
            unreadable.push(path.display().to_string());
            continue;
        };
        // A file that is not a face source is allowed to fail to parse: what must
        // never happen is a *nesting* refusal, because that means the guard would
        // reject a file this repository ships.
        // 不是注册面源码的文件允许解析失败：绝不允许发生的是**嵌套**拒绝，因为那意味着守卫会
        // 拒绝一个本仓库出厂的文件的。
        if let Err(error) = xirang_kernel::registry_core::syntax::parse_faces(&text) {
            let message = error.to_string();
            if message.contains(REFUSAL) {
                refused.push(format!("{}: {message}", path.display()));
            }
        }
    }
    (refused, unreadable)
}

#[test]
fn no_source_in_this_workspace_is_refused_by_the_nesting_guard() {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let root = manifest
        .parent()
        .expect("the kernel is unpacked somewhere")
        .to_path_buf();
    // This test ships inside the published crate, and a consumer has no workspace around
    // it: from a registry unpack the parent is the registry's `src/` directory, so the
    // walk used to "pass" by scanning whichever unrelated crates happened to be unpacked
    // next to it. It says so and stands down instead — the promise it keeps ("the guard
    // refuses nothing this repository ships") is about this repository.
    // 本测试随已发布 crate 出厂，而消费者周围没有工作区：从 registry 解包后父目录是 registry
    // 的 `src/`，于是遍历过去靠"恰好解包在旁边的无关 crate"来"通过"。现在它说明情况并退出——
    // 它守的承诺（"守卫不拒绝本仓库出厂的任何东西"）是关于本仓库的。
    // The two directory names are the ones a checkout has *today*: `kernel/` and
    // `toolchain/`. They used to be `core/` and `build_method/`, and after batches 1 and
    // 2 renamed them this gate did not stand down for a consumer — it stood down for
    // this repository too, so the promise it keeps went unchecked for two batches while
    // the test reported success.
    // 这两个目录名是**今天**的检出才有的：`kernel/` 与 `toolchain/`。它们过去是 `core/` 与
    // `build_method/`，而批 1 与批 2 改名之后，这道门禁不只是对消费者退出——它对**本仓库**也
    // 退出了，于是它守的承诺有两个批次无人检查，而测试一直报成功。
    if !root.join("kernel").is_dir() || !root.join("toolchain").is_dir() {
        eprintln!(
            "skipping: {} is not a XiRang checkout; this gate is about that repository",
            root.display()
        );
        return;
    }
    if !census_enabled() {
        // A skip that says so: this face is not the census face of the ten-face gate. The census is
        // about the sources, so one face running it is the same promise; silence here would make a
        // reader wonder whether the check ran at all.
        // 明说的跳过：这个面不是十面门禁的普查面。普查是关于源码的，一个面跑它就是同一个承诺；在这里
        // 沉默会让读者怀疑这项检查到底跑没跑。
        eprintln!(
            "skipping the nesting census: {CENSUS_ENV}=0 says another face already ran it over these sources"
        );
        return;
    }
    let (roots, fell_back) = census_roots(&root);
    if fell_back {
        eprintln!(
            "note: the workspace member list could not be read, so the census walks the whole root"
        );
    }
    let mut files = Vec::new();
    for directory in &roots {
        rust_files(directory, &mut files);
    }
    files.sort();
    // A floor, so a walk that silently finds nothing cannot pass. The workspace
    // has hundreds of Rust files; the number is not asserted exactly because a
    // new module should not have to update this test.
    // 一个下限，使"什么都没找到"的遍历无法通过。工作区有数百个 Rust 文件；这里不断言精确数字，
    // 因为新增模块不该被迫来改这条测试。
    assert!(
        files.len() > 100,
        "the walk found {} Rust files under {}; it is supposed to cover the workspace",
        files.len(),
        root.display()
    );

    // The digest covers exactly the inputs below, so a cache hit means "these same bytes were
    // already inspected". A hit is reported, not silent: the reader must know the run reused an
    // earlier green census rather than parsed the tree again.
    // 摘要恰好覆盖下面的输入，因此命中缓存意味着"同一批字节已经被检查过"。命中要说出来，不能沉默：
    // 读者必须知道这次是复用了早先的绿色普查，还是又解析了一遍树。
    let digest = census_digest(&files, &root);
    if cached_digest(&root).as_deref() == Some(digest.as_str()) {
        println!(
            "the nesting census is unchanged: digest {digest} over {} Rust files (reused the cached green result)",
            files.len()
        );
        return;
    }
    let (refused, unreadable) = inspect(&files);
    assert!(
        unreadable.is_empty(),
        "the walk found {} file(s) it could not read, so the budget does not cover \
         them and cannot claim the workspace is clean:\n{}",
        unreadable.len(),
        unreadable.join("\n")
    );
    assert!(
        refused.is_empty(),
        "the nesting guard refuses {} of this workspace's own files:\n{}",
        refused.len(),
        refused.join("\n")
    );
    if let Err(error) = record_census(&root, &digest) {
        // The census itself is green; failing to write the cache only costs the next run its
        // speedup, so it is reported rather than turned into a failure.
        // 普查本身是绿的；写缓存失败只会让下一次跑失去加速，因此报告它而不是把它变成失败。
        eprintln!(
            "note: the census result could not be cached ({error}); the next run will re-check"
        );
    }
    println!("the nesting guard accepted all {} Rust files", files.len());
}

/// A file that cannot be read is reported rather than skipped, because a skipped
/// file is one the guard never checked.
/// 读不到的文件会被报出而不是被跳过，因为被跳过的文件就是守卫从未检查过的文件。
#[test]
fn a_file_that_cannot_be_read_is_reported() {
    let path = std::env::temp_dir().join(format!(
        "xirang-nesting-unreadable-{}-{}.rs",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    // Invalid UTF-8 in a file that exists: exactly the shape of a silently skipped
    // source, and `read_to_string` refuses it.
    // 一个存在但含非法 UTF-8 的文件：正是被静默跳过的源码的形状，而 `read_to_string` 会拒绝它。
    std::fs::write(&path, [0x66, 0x6e, 0x20, 0xff, 0xfe, 0x0a]).expect("fixture file");
    let (refused, unreadable) = inspect(std::slice::from_ref(&path));
    let _ = std::fs::remove_file(&path);
    assert!(refused.is_empty(), "{refused:#?}");
    assert_eq!(unreadable.len(), 1, "{unreadable:#?}");
}

/// **The census reads its roots from the workspace manifest, not from the whole checkout.**
/// **普查从 workspace 清单读取要走的根，而不是整棵检出。**
///
/// The old walk crossed `docs/` (8+ GB, hundreds of thousands of files) to find a few fixtures. The
/// promise did not need that: the sources this repository ships are the workspace members. This pin
/// also spells the fallback: a manifest the test cannot read goes back to the whole root, because a
/// narrower walk must never silently mean "checked less".
/// 旧遍历为了几十个夹具穿过 `docs/`（8+ GB、几十万个文件）。那个承诺并不需要它：本仓出厂的源码就是
/// workspace 成员。这条钉子也写明退回：读不到的清单退回整棵根，因为更窄的遍历绝不能悄悄变成"查得更少"。
#[test]
fn the_census_roots_are_the_workspace_members() {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let root = manifest.parent().expect("the kernel is unpacked somewhere");
    let (roots, fell_back) = census_roots(root);
    assert!(!fell_back, "this checkout has a readable member list");
    let names = roots
        .iter()
        .map(|path| {
            path.strip_prefix(root)
                .expect("a member is under the root")
                .to_string_lossy()
                .replace('\\', "/")
        })
        .collect::<Vec<_>>();
    for expected in ["kernel", "toolchain", "multilang", "conventions"] {
        assert!(
            names.iter().any(|name| name == expected),
            "the member `{expected}` is missing from {names:?}"
        );
    }
    assert!(
        !names.iter().any(|name| name.starts_with("docs")),
        "docs is not a workspace member and must not be walked: {names:?}"
    );
}

/// **A content change invalidates the cache digest, and an unchanged tree keeps it.**
/// **内容变化使缓存摘要失效，未变的树保持同一摘要。**
///
/// The digest is the whole invalidation rule, so it is pinned directly: path and bytes in, a
/// version salt, and the same bytes must produce the same digest across calls.
/// 摘要就是整条失效规则，因此直接钉住它：路径与字节进摘要、带一个版本盐，同一批字节跨调用给出同一摘要。
#[test]
fn the_census_digest_tracks_content_and_path() {
    let root = std::env::temp_dir().join(format!(
        "xirang-nesting-digest-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).expect("fixture root");
    let first = root.join("a.rs");
    let second = root.join("nested").join("b.rs");
    std::fs::create_dir_all(second.parent().expect("nested")).expect("nested");
    std::fs::write(&first, "fn a() {}\n").expect("a");
    std::fs::write(&second, "fn b() {}\n").expect("b");
    let files = vec![first.clone(), second.clone()];
    let before = census_digest(&files, &root);
    assert_eq!(
        before,
        census_digest(&files, &root),
        "same bytes, same digest"
    );
    std::fs::write(&second, "fn b() { let _ = 1; }\n").expect("b changed");
    let changed = census_digest(&files, &root);
    assert_ne!(
        before, changed,
        "a content change must invalidate the cache"
    );
    std::fs::write(&second, "fn b() {}\n").expect("b restored");
    assert_eq!(
        before,
        census_digest(&files, &root),
        "restoring the bytes restores the digest"
    );
    // Moving a file to another member is another input even with identical bytes.
    // 把文件搬到另一个成员是另一个输入，即使字节相同。
    let moved = root.join("nested").join("c.rs");
    std::fs::rename(&second, &moved).expect("moved");
    assert_ne!(
        before,
        census_digest(&[first, moved], &root),
        "a path change must invalidate the cache"
    );
    let _ = std::fs::remove_dir_all(root);
}
