//! Tests for the workspace entrance: a virtual manifest names no package, so its tree
//! tools enumerate the members and answer grouped by package.
//! 工作区入口的测试：虚拟清单不命名任何包，因此它的树级工具枚举成员并按包分组作答。
//!
//! Two nails live here, and both are about an answer rather than a value. The first is
//! that a tree tool called on a virtual root names **every** member with its status and
//! really carries the host member's faces — a member dropped from the census, or a
//! `no faces` member silently skipped, turns it red and says which member is missing.
//! The second is that a degradation is stated in the body: a member whose tree cannot be
//! derived carries `tree unavailable (reason)` instead of contributing nothing, and a
//! root whose manifest Cargo cannot resolve says why rather than erroring out.
//! 这里有两枚钉子，两枚都是关于"一份答案"而不是一个取值。第一枚：在虚拟根上调用树级工具时，答案点名
//! **每一个**成员及其状态，并且真的带着宿主成员的面——把某个成员从普查里丢掉、或把 `no faces` 成员
//! 静默跳过，都会让它变红并说出缺的是哪个成员。第二枚：降级写在正文里——推导不出树的成员带着
//! `tree unavailable (原因)`，而不是什么都不贡献；Cargo 解析不了的根的清单则说出原因，而不是整条报错。

use std::path::{Path, PathBuf};

use serde_json::json;
use xirang_kernel::NodeId;

use super::{Scope, scope};

/// One fixture workspace: the root, and the three member package names.
/// 一个夹具工作区：根，以及三个成员的包名。
struct Workspace {
    root: PathBuf,
    /// A host member: it declares one face and published nothing, so its tree was
    /// derived now and is reported as `not built`.
    /// 宿主成员：声明一个面且什么都没发布，因此它的树是现推的，报成 `not built`。
    host: String,
    /// A framework member: a library with no registration face at all.
    /// 框架成员：一个完全没有注册面的库。
    framework: String,
    /// A binary-only member with no `src/`: its tree cannot be derived.
    /// 一个没有 `src/`、只有二进制的成员：它的树推导不出来。
    tool: String,
}

impl Workspace {
    /// The census line every merged tree answer carries.
    /// 每份合并树级答案都带的那条普查行。
    ///
    /// `queried` is the face axis this census has always counted; `published` and
    /// `not built` are the evidence axis, and this fixture publishes nothing, so
    /// the one member with faces is `not built`.
    /// `queried` 是这个普查一直在数的面那一轴；`published` 与 `not built` 是证据轴，而本夹具什么
    /// 都没发布，因此那个有面的成员是 `not built`。
    fn census(&self) -> String {
        "members 3  queried 1  no faces 1  unresolvable 1  published 0  not built 1".to_owned()
    }
}

impl Drop for Workspace {
    /// Remove the fixture tree once the test that built it is done.
    /// 构建它的测试结束后删除夹具树。
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

/// A throwaway virtual workspace whose three members cover the three statuses.
/// 一个一次性虚拟工作区，它的三个成员覆盖三种状态。
fn workspace(label: &str) -> Workspace {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "xirang-mcp-workspace-{label}-{}-{sequence}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    let name = |member: &str| format!("mcp-ws-{label}-{member}");
    let manifest = |member: &str| {
        format!(
            "[package]\nname = \"{}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
            name(member)
        )
    };
    write_file(
        &root.join("Cargo.toml"),
        "[workspace]\nmembers = [\"host\", \"framework\", \"tool\"]\nresolver = \"2\"\n",
    );
    write_file(&root.join("host/Cargo.toml"), &manifest("host"));
    write_file(&root.join("host/src/lib.rs"), "// host entry\n");
    write_file(
        &root.join("host/src/button/button.rs"),
        "pub struct Button;\n\ncrate::root_object! {\n    kind: Button,\n    parent: \
         crate::root_node_id(env!(\"CARGO_PKG_NAME\")),\n}\n",
    );
    write_file(&root.join("framework/Cargo.toml"), &manifest("framework"));
    write_file(&root.join("framework/src/lib.rs"), "// framework entry\n");
    // A package whose only target is a binary at the package root: Cargo resolves it as a
    // member, and the face derivation has no `src/` to read — the reachable
    // `unresolvable` member.
    // 唯一目标是包根下一个二进制的包：Cargo 把它解析为成员，而面的推导没有 `src/` 可读——这正是
    // 可达的那个 `unresolvable` 成员。
    write_file(
        &root.join("tool/Cargo.toml"),
        &format!(
            "{}autobins = false\n\n[[bin]]\nname = \"{}\"\npath = \"main.rs\"\n",
            manifest("tool"),
            name("tool")
        ),
    );
    write_file(&root.join("tool/main.rs"), "fn main() {}\n");
    Workspace {
        root,
        host: name("host"),
        framework: name("framework"),
        tool: name("tool"),
    }
}

/// Write one fixture file, creating its directory.
/// 写一个夹具文件并建好它的目录。
fn write_file(path: &Path, text: &str) {
    std::fs::create_dir_all(path.parent().expect("fixture parent")).expect("fixture directories");
    std::fs::write(path, text).expect("fixture file");
}

/// The first nail: `xirang.registry` on a virtual root names every member with its
/// status, and the host member's faces are really in the answer, under the namespace its
/// own `CARGO_PKG_NAME` compiled with.
/// 第一枚钉子：虚拟根上的 `xirang.registry` 点名每个成员及其状态，而宿主成员的面真的在答案里，
/// 就在它自己 `CARGO_PKG_NAME` 编译所用的命名空间之下。
/// A member root is told which workspace it belongs to, so an answer can say who is invisible:
/// the scenario round measured two functions reported as orphans at a member root whose callers
/// simply live in the other member.
/// 成员根会被告知它属于哪个工作区，于是答案能说清谁不可见：情景轮量到两个函数的调用者只是住在另一个
/// 成员里，却在成员根上被报成孤儿。
#[test]
fn a_member_root_knows_the_workspace_above_it() {
    let base = std::env::temp_dir().join(format!("xirang-member-{}", std::process::id()));
    let member = base.join("core");
    std::fs::create_dir_all(member.join("src")).expect("the fixture directory");
    std::fs::write(
        base.join("Cargo.toml"),
        "[workspace]\nmembers = [\"core\", \"report\"]\n",
    )
    .expect("the manifest");
    assert_eq!(
        crate::mcp::workspace::enclosing_workspace(&member),
        Some(base.clone()),
        "the member names its workspace"
    );
    assert_eq!(
        crate::mcp::workspace::enclosing_workspace(&base),
        None,
        "the workspace root itself has nothing above it"
    );
    let _ = std::fs::remove_dir_all(&base);
}

/// The third nail: the preamble is spent **once per root**. The answer that opens a root
/// carries the census and a row per member; the next answer for the same root — whatever tool
/// asks — carries the one-line index instead, census and all; and a *change* of root buys the
/// rows back.
/// 第三枚钉子：前言**每个根只花一次**。打开一个根的那份答案带着普查与逐成员一行；同一个根的下一条
/// 答案——无论哪个工具问——带的是那一行索引，普查也在里面；而**换根**会把那些行买回来。
///
/// The index repeats the census verbatim rather than abbreviating it, so this pin can assert the
/// same counts on both shapes: a compressed answer must not be able to say less about the tree it
/// read than the full one did.
/// 索引逐字重复普查而不是缩写它，因此这枚钉子可以在两种形态上断言同一份计数：一份压缩过的答案，不许
/// 对"它读过的那棵树"说得比完整形态更少。
#[test]
fn the_preamble_is_spent_once_per_root_and_a_change_of_root_brings_it_back() {
    let fixture = workspace("preamble");
    super::forget_announced();
    let opened = crate::mcp::registry::registry(&fixture.root).expect("the root opens");
    assert!(opened.contains(&fixture.census()), "{opened}");
    for member in [&fixture.host, &fixture.framework, &fixture.tool] {
        assert!(
            ["not built", "no faces", "unresolvable"]
                .iter()
                .any(|word| opened.contains(&format!("  {word:<13} {member:<30} "))),
            "every member gets a row when the root opens, `{member}` did not: {opened}"
        );
    }

    // A **different tool** on the same root pays nothing: what is remembered is the root, not
    // the tool, so the second answer cannot be shorter merely because the first one was registry.
    // 同一个根上的**另一个工具**什么都不同付：记住的是根而不是工具，因此第二条答案不会仅仅因为第一条
    // 是 registry 就短一截。
    let again = crate::mcp::search::search(&fixture.root, &json!({"query": "button"}))
        .expect("the same root answers again");
    let index = format!(
        "workspace {} · members 3 · built 0 · census {}",
        fixture.root.display(),
        fixture.census()
    );
    assert!(
        again.contains(&index),
        "the second answer is the index line: {again}"
    );
    assert!(
        !again.contains(&format!("  {:<13} {:<30} ", "not built", fixture.host)),
        "the rows are not repeated: {again}"
    );

    let other = workspace("preamble-other");
    let elsewhere = crate::mcp::registry::registry(&other.root).expect("another root opens");
    assert!(elsewhere.contains(&other.census()), "{elsewhere}");
    let back = crate::mcp::registry::registry(&fixture.root).expect("the first root opens again");
    assert!(
        back.contains(&fixture.census()) && back.contains("== "),
        "a change of root buys the rows back: {back}"
    );
    assert!(
        back.contains(&format!("  {:<13} {:<30} ", "not built", fixture.host)),
        "the rows are back: {back}"
    );
}

/// The fourth nail: the degradation reason is expanded by the face and build tools and folded
/// everywhere else — and the two facts a row must never lose stay in **both** shapes: the status
/// word (`not built`, `unresolvable`), and why a member has no faces.
/// 第四枚钉子：降级原因由面与构建类工具展开、其余地方折起来——而一行绝不能丢的两件事在**两种**形态里
/// 都在：状态词（`not built`、`unresolvable`），以及一个成员为什么没有面。
#[test]
fn the_degradation_reason_is_expanded_by_the_face_and_build_tools_only() {
    let fixture = workspace("reasons");
    super::forget_announced();
    let registry = crate::mcp::registry::registry(&fixture.root).expect("registry answers");
    assert!(
        registry.contains(&format!(
            "  {:<13} {:<30} tree unavailable (no source tree at",
            "unresolvable", fixture.tool
        )),
        "a face tool expands the reason in its row: {registry}"
    );
    assert!(
        registry.contains("1 faces; not built (cannot read"),
        "a face tool expands the not-built reason too: {registry}"
    );

    super::forget_announced();
    let search = crate::mcp::search::search(&fixture.root, &json!({"query": "button"}))
        .expect("search answers");
    assert!(
        search.contains(&format!(
            "  {:<13} {:<30} tree unavailable\n",
            "unresolvable", fixture.tool
        )),
        "a lookup tool folds the unresolvable reason away: {search}"
    );
    assert!(
        search.contains("1 faces; not built\n"),
        "a lookup tool folds the not-built reason away: {search}"
    );
    assert!(
        !search.contains("not built (cannot read"),
        "the folded row carries no reason: {search}"
    );
    assert!(
        search.contains("no registration face under src/"),
        "why a member has no faces is not the reason that folds: {search}"
    );
}

#[test]
fn the_registry_tool_answers_a_virtual_root_with_every_member() {
    let fixture = workspace("registry");
    let reply = crate::mcp::registry::registry(&fixture.root).expect("a workspace root answers");
    assert!(reply.contains("workspace "), "{reply}");
    assert!(
        reply.contains(&fixture.census()),
        "the census must count every member with its status ({}): {reply}",
        fixture.census()
    );
    for member in [&fixture.host, &fixture.framework, &fixture.tool] {
        assert!(
            reply.contains(member.as_str()),
            "every member must appear in the answer, `{member}` did not: {reply}"
        );
    }
    assert!(
        reply.contains(&format!("namespace {}\n", fixture.host)),
        "the host member `{}` must be answered under its own namespace: {reply}",
        fixture.host
    );
    assert!(reply.contains("root/button"), "{reply}");
    assert!(
        reply.contains("tree derived now (no published records at"),
        "a member with no records must say so rather than read as an unpublished tree: {reply}"
    );
    let expected = NodeId::from_namespaced_path(&fixture.host, "button/button.rs", "Button");
    assert!(
        reply.contains(&expected.to_string()),
        "the host member's face must carry its compiled identity {expected}: {reply}"
    );
    assert!(
        reply.contains(&format!("== {} (not built)\n", fixture.host)),
        "the host member `{}` has no section: {reply}",
        fixture.host
    );
    assert!(
        reply.contains(&format!("== {} (no faces)\n", fixture.framework)),
        "the framework member `{}` must have a `no faces` section, not a silence: {reply}",
        fixture.framework
    );
    assert!(
        reply.contains(&format!("== {} (unresolvable)\n", fixture.tool)),
        "the unresolvable member `{}` has no section: {reply}",
        fixture.tool
    );
    assert!(
        reply.contains("tree unavailable (no source tree at"),
        "the unresolvable member's reason must be in the body: {reply}"
    );
}

/// The second nail: `xirang.search` on a virtual root states the census, groups each
/// member's face hits under it, and names the degradations instead of hiding them behind
/// a longer list of file hits.
/// 第二枚钉子：虚拟根上的 `xirang.search` 说出普查，把每个成员的面命中分组在其下，并点名降级，
/// 而不是把它们藏在一份更长的文件命中之后。
#[test]
fn a_workspace_search_states_every_member_and_its_degradation() {
    let fixture = workspace("search");
    let reply = crate::mcp::search::search(&fixture.root, &json!({"query": "button"}))
        .expect("the search answers");
    assert!(
        reply.contains(&fixture.census()),
        "the census must count every member with its status ({}): {reply}",
        fixture.census()
    );
    for member in [&fixture.host, &fixture.framework, &fixture.tool] {
        assert!(
            reply.contains(member.as_str()),
            "`{member}` is missing from the answer: {reply}"
        );
    }
    assert!(reply.contains("face  root/button"), "{reply}");
    assert!(
        reply.contains(&format!("member {} (not built)", fixture.host)),
        "the host member's hits are grouped under it: {reply}"
    );
    assert!(
        reply.contains("no registration face under src/"),
        "the framework member's status says why it has none: {reply}"
    );
    assert!(
        reply.contains(&format!(
            "member {} (unresolvable)  tree unavailable (",
            fixture.tool
        )),
        "the unresolvable member says so where the reader is looking: {reply}"
    );
    let nothing =
        crate::mcp::search::search(&fixture.root, &json!({"query": "nothing-matches-this"}))
            .expect("the search answers");
    assert!(
        nothing.contains(&fixture.census()) && nothing.contains("no matches"),
        "a workspace query with no hits still states the tree it read, and says nothing matched: \
         {nothing}"
    );
}

/// The other two tree tools answer the same virtual root, and their bodies say the same
/// two things: a status per member, and a reason for the member that has no tree.
/// 另外两个树级工具对同一个虚拟根作答，而它们的正文说同样两件事：逐成员一个状态，以及没有树的那个
/// 成员的原因。
#[test]
fn grafts_and_diff_answer_a_virtual_root_with_the_same_census() {
    let fixture = workspace("tools");
    let report = crate::mcp::grafts::grafts(&fixture.root, &json!({})).expect("grafts answers");
    assert!(
        report.contains(&fixture.census()),
        "grafts must carry the same census ({}): {report}",
        fixture.census()
    );
    assert!(
        report.contains(&format!("namespace {}\n", fixture.host)),
        "grafts must answer the host member `{}` under its own namespace: {report}",
        fixture.host
    );
    assert!(
        report.contains("tree unavailable (no source tree at"),
        "grafts must explain the unresolvable member `{}`: {report}",
        fixture.tool
    );
    let delta = crate::mcp::diff::diff(&fixture.root, &json!({"records": true}))
        .expect("the records diff answers");
    assert!(
        delta.contains(&fixture.census()),
        "diff must carry the same census ({}): {delta}",
        fixture.census()
    );
    assert!(
        delta.contains(&format!("== {} (no faces)\n", fixture.framework)),
        "diff must give the framework member `{}` a section, not a silence: {delta}",
        fixture.framework
    );
    assert!(
        delta.contains("tree unavailable (no source tree at"),
        "{delta}"
    );
}

/// A member root is still one package: pointing `root` at a member keeps the
/// single-package answer byte for byte, with no census above it.
/// 成员根本来就还是一个包：把 `root` 指向成员时，单包答案逐字节不变，上面没有普查。
#[test]
fn a_member_root_still_answers_as_one_package() {
    let fixture = workspace("member");
    let reply = crate::mcp::registry::registry(&fixture.root.join("host"))
        .expect("the member answers about itself");
    assert!(
        reply.starts_with(&format!("namespace {}\n", fixture.host)),
        "{reply}"
    );
    assert!(!reply.contains("workspace "), "{reply}");
    assert!(!reply.contains("members "), "{reply}");
    let resolved = match scope(&fixture.root.join("host")).expect("the member resolves") {
        Scope::Package(namespace) => namespace,
        _ => panic!("a member directory is a package"),
    };
    assert_eq!(resolved, fixture.host);
}

/// A root whose manifest Cargo cannot resolve is a real root whose tree cannot be
/// named — the nested fixture package's shape — so the answer carries Cargo's reason and
/// the way out instead of a bare error, and `search` still answers the source half.
/// Cargo 解析不了清单的根是一个真实存在、却无法被命名的根——正是嵌套夹具包的形状——因此答案带上
/// Cargo 的原因与出路，而不是一个光秃秃的错误；而 `search` 仍然回答源码那一半。
#[test]
fn a_root_cargo_cannot_resolve_says_why_in_the_body() {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    // The dangling dependency points *outside* the package, and the inner `[workspace]`
    // table is what makes Cargo load it eagerly — the exact shape the nested fixture
    // package has (its own `[workspace]` plus path dependencies that no longer exist).
    // A missing path inside the package, with no `[workspace]`, is silently deferred.
    // 悬空的依赖指向包**之外**，而内部的 `[workspace]` 表正是让 Cargo 急切加载它的东西——这正是嵌套
    // 夹具包的形状（它自己的 `[workspace]` 加上已不存在的路径依赖）。包内缺失的路径、且没有
    // `[workspace]` 时，会被静默推迟。
    let base = std::env::temp_dir().join(format!(
        "xirang-mcp-unresolvable-{}-{sequence}",
        std::process::id()
    ));
    let root = base.join("pkg");
    let _ = std::fs::remove_dir_all(&base);
    write_file(
        &root.join("Cargo.toml"),
        "[workspace]\n\n[package]\nname = \"mcp-unresolvable\"\nversion = \"0.1.0\"\n\
         edition = \"2021\"\n\n[dependencies]\ngone = { path = \"../gone\" }\n",
    );
    write_file(&root.join("src/lib.rs"), "// host entry\n");
    write_file(&root.join("src/widget.rs"), "pub fn widget_spin() {}\n");

    let reply = crate::mcp::registry::registry(&root).expect("a reason is an answer");
    assert!(reply.starts_with("unresolvable "), "{reply}");
    assert!(
        reply.contains("failed to load manifest for dependency"),
        "the reason Cargo gave must be the reason reported: {reply}"
    );
    assert!(reply.contains("XIRANG_NAMESPACE"), "{reply}");

    let search = crate::mcp::search::search(&root, &json!({"query": "widget"}))
        .expect("the source half still answers");
    assert!(
        search.contains("tree  unavailable ("),
        "the tree half is named as unavailable, not silently empty: {search}"
    );
    assert!(
        search.contains("failed to load manifest for dependency"),
        "{search}"
    );
    assert!(search.contains("file  src/widget.rs"), "{search}");
    let _ = std::fs::remove_dir_all(&base);
}

/// A tree that declares its own `[workspace]` is a workspace root, so nothing encloses it; and a
/// member is recognised by a whole path segment, never by a substring.
/// 自己声明 `[workspace]` 的树就是 workspace 根，没有东西包着它；而成员是按**整段路径**认的，
/// 绝不按子串。
#[test]
fn a_tree_that_is_its_own_workspace_has_no_enclosing_one() {
    let base = std::env::temp_dir().join(format!("ws-note-{}", std::process::id()));
    let member = base.join("fe");
    let nested = member.join("crates").join("core");
    std::fs::create_dir_all(&nested).expect("tree");
    // The member is its own workspace, exactly like `target/round10/ours/trees/fe`.
    // 这个成员就是它自己的 workspace，与 `target/round10/ours/trees/fe` 一样。
    std::fs::write(
        member.join("Cargo.toml"),
        "[workspace]\nmembers = [\"crates/core\"]\n\n[workspace.package]\nversion = \"0.1.0\"\n",
    )
    .expect("manifest");
    // The unrelated parent mentions `features`, which used to make the member look like its child
    // by substring: `fe` matched inside `features`.
    // 不相干的父工程含 `features` 一词，从前正是它让这个成员看起来像它的孩子：`fe` 命中了 `features`。
    std::fs::write(
        base.join("Cargo.toml"),
        "[workspace]\nmembers = [\"crates/app\"]\nresolver = \"2\"\n\n[workspace.lints]\nfeatures = []\n",
    )
    .expect("parent manifest");
    assert_eq!(
        super::enclosing_workspace(&member),
        None,
        "a root that declares [workspace] itself has no enclosing workspace"
    );
    // A real member *of that workspace* is still recognised, by its whole segment.
    // 那个 workspace 的**真成员**仍被认出——按整段。
    std::fs::write(
        base.join("Cargo.toml"),
        "[workspace]\nmembers = [\"fe\", \"crates/app\", \"plain\"]\n",
    )
    .expect("parent manifest");
    assert_eq!(
        super::enclosing_workspace(&member),
        None,
        "still its own workspace"
    );
    let plain = base.join("plain");
    std::fs::create_dir_all(&plain).expect("plain");
    std::fs::write(plain.join("Cargo.toml"), "[package]\nname = \"plain\"\n")
        .expect("plain manifest");
    assert_eq!(
        super::enclosing_workspace(&plain),
        Some(base.clone()),
        "a member without its own [workspace] is named by its parent"
    );
    let _ = std::fs::remove_dir_all(&base);
}

/// A tree hosts xirang only when a `build.rs` **and** the dependency are both there.
/// 一棵树只有在 `build.rs` **与**那条依赖同时存在时才算宿主 xirang。
///
/// The round measured the corpus: the trees under test have neither, so on them `0 faces` is the
/// answer rather than a defect — which is why `registry` says so before its per-member rows.
/// 那一轮量遍了语料：被测的树两者都没有，因此在它们身上 `0 faces` 是答案而不是缺陷 —— 这也是
/// `registry` 在逐成员行**之前**先说明这一点的原因。
#[test]
fn hosts_xirang_reads_the_manifest_and_the_build_script_together() {
    let base = std::env::temp_dir().join(format!("ws-host-{}", std::process::id()));
    let crate_dir = base.join("crates").join("app");
    std::fs::create_dir_all(&crate_dir).expect("tree");
    // Neither half: not a host.
    // 两半都没有：不是宿主。
    assert!(!super::hosts_xirang(&base));
    // Only the manifest half: still not a host.
    // 只有清单那一半：仍不是宿主。
    std::fs::write(
        crate_dir.join("Cargo.toml"),
        "[package]\nname = \"app\"\n\n[dependencies]\nxirang-toolchain = \"0.2.0\"\n",
    )
    .expect("manifest");
    assert!(!super::hosts_xirang(&base));
    // Both halves: a host, found two levels down from the root.
    // 两半都有：是宿主，且从根往下两层就能找到。
    std::fs::write(crate_dir.join("build.rs"), "fn main() {}\n").expect("build script");
    assert!(super::hosts_xirang(&base));
    let _ = std::fs::remove_dir_all(&base);
}

/// A non-host member's `*_object!` macros are application macros, not unparsable faces.
/// 非宿主成员的 `*_object!` 宏是应用宏，不是"读不了的面"。
///
/// Measured on a real repository: a workspace of two ordinary packages contains 19 `handle:` fields
/// its own macro understands, and the census used to print one "unparsable face" row per field while
/// `registry` answered "this tree does not host xirang". The row now says what is actually true: the
/// declarations are not registration declarations.
/// 在一个真实仓库上量到：两个普通包组成的工作区里有 19 个它自己的宏才懂的 `handle:` 字段，普查过去为每个
/// 字段印一条"读不了的面"，而 `registry` 回答"这棵树不宿主 xirang"。那一行现在说真话：这些声明不是注册声明。
#[test]
fn a_non_host_members_object_macros_are_not_unparsable_faces() {
    let root = std::env::temp_dir().join(format!(
        "xirang-mcp-nonhost-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    let _ = std::fs::remove_dir_all(&root);
    write_file(
        &root.join("Cargo.toml"),
        "[workspace]\nmembers = [\"plain\", \"host\"]\nresolver = \"2\"\n",
    );
    write_file(
        &root.join("plain/Cargo.toml"),
        "[package]\nname = \"plain-app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    write_file(&root.join("plain/src/lib.rs"), "mod scroll;\n");
    write_file(&root.join("plain/src/scroll/mod.rs"), "pub mod scroll;\n");
    write_file(
        &root.join("plain/src/scroll/scroll.rs"),
        "control_object! {\n    kind: ScrollBar,\n    handle: ScrollBar,\n}\n",
    );
    write_file(
        &root.join("host/Cargo.toml"),
        "[package]\nname = \"host-app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n         [build-dependencies]\nxirang-toolchain = \"0.1.0\"\n",
    );
    write_file(&root.join("host/build.rs"), "fn main() {}\n");
    write_file(&root.join("host/src/lib.rs"), "mod scroll;\n");
    write_file(&root.join("host/src/scroll/mod.rs"), "pub mod scroll;\n");
    write_file(
        &root.join("host/src/scroll/scroll.rs"),
        "control_object! {\n    kind: ScrollBar,\n    handle: ScrollBar,\n}\n",
    );
    let members = match super::scope(&root).expect("a workspace scope") {
        super::Scope::Workspace(members) => members,
        other => panic!(
            "expected a workspace, got {:?}",
            std::mem::discriminant(&other)
        ),
    };
    let text = super::roster_expanded(&root, &members);
    let plain = text
        .split("\n")
        .find(|line| {
            line.contains("plain-app") || line.contains("mcp-ws") || line.contains("plain")
        })
        .unwrap_or_default();
    assert!(
        text.contains("does not host xirang"),
        "the non-host member is named as such: {text}"
    );
    assert!(
        text.matches("is not a registration face field").count() == 1,
        "only the host member reports an unparsable face: {text}"
    );
    let _ = plain;
    let _ = std::fs::remove_dir_all(root);
}
