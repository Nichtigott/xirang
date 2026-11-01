//! Tests for the build-evidence report: it must name what the build decided, and
//! it must say "unknown" rather than guess when the build never ran.
//! 构建证据报告的测试：它必须说出构建决定了什么，而在构建从未跑过时必须说"未知"而不是猜。

use std::path::{Path, PathBuf};

use crate::build_method::face_views;
use serde_json::json;

use super::explain;

/// A throwaway package holding one hand-written root face, so `face_views`
/// derives exactly one node to report on.
/// 一个含一个手写根面的一次性包，因此 `face_views` 恰好推导出一个可报告节点。
fn package(label: &str) -> (PathBuf, String) {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let name = format!("mcp-evidence-{label}");
    let root = std::env::temp_dir().join(format!("{name}-{}-{sequence}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("src/button")).expect("package directory");
    std::fs::write(
        root.join("Cargo.toml"),
        format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n"),
    )
    .expect("manifest");
    std::fs::write(root.join("src/lib.rs"), "// host entry\n").expect("library target");
    std::fs::write(
        root.join("src/button/button.rs"),
        "pub struct Button;\n\ncrate::root_object! {\n    kind: Button,\n    parent: crate::root_node_id(env!(\"CARGO_PKG_NAME\")),\n}\n",
    )
    .expect("face");
    (root, name)
}

/// Publish the two files the build writes, so the report has something to read.
/// 发布构建写下的那两个文件，让报告有东西可读。
fn publish(root: &Path, name: &str, all: bool) {
    let out = root.join("target/xirang/out");
    std::fs::create_dir_all(&out).expect("build output");
    let faces = face_views(root, name).expect("faces derive");
    let id = faces[0].id;
    std::fs::write(
        out.join("source_scope.tsv"),
        if all {
            "# mode\tauto\n# result\tall\n# selected\tall\n".to_owned()
        } else {
            format!("# mode\tauto\n# result\tselected\n# selected\t-\n# id\t{id}\n")
        },
    )
    .expect("scope");
    std::fs::write(
        out.join("pruning_manifest.tsv"),
        format!(
            "# node\tsource\tsymbol\n{id}\t{}\t{}\n",
            faces[0].source, "button_symbol"
        ),
    )
    .expect("pruning");
}

/// The one question no other tool can answer: is this face in the shipped scope,
/// and does release pruning strip it. The answer comes from the build's own files.
/// 没有别的工具能回答的那个问题：这个面在发布作用域里吗，发布剪枝会不会剥掉它。答案来自构建
/// 自己的文件。
#[test]
fn the_report_answers_scope_and_pruning_from_the_builds_own_files() {
    let (root, name) = package("scope");
    publish(&root, &name, true);
    let reply = explain(&root, &json!({"node": "root/button"})).expect("the report renders");
    assert!(reply.contains("path root/button"), "{reply}");
    assert!(reply.contains("kind Button"), "{reply}");
    assert!(reply.contains("registry_name button"), "{reply}");
    assert!(reply.contains("scope selected (all=true"), "{reply}");
    assert!(reply.contains("pruning strips button_symbol"), "{reply}");
    // No `discovery.fingerprint` was published, so the report must admit the
    // build output is not known to be current.
    // 没有发布 `discovery.fingerprint`，因此报告必须承认无法认为构建产物是新鲜的。
    assert!(reply.contains("build stale"), "{reply}");
    let _ = std::fs::remove_dir_all(&root);
}

/// A face the scope does not select is a pruning candidate, and the report says so
/// by name instead of reporting a generic "not in the tree".
/// 作用域没选中的面是剪枝候选，报告要指名道姓地说出来，而不是笼统地报"不在树里"。
#[test]
fn a_face_outside_the_scope_is_reported_as_not_selected() {
    let (root, name) = package("unselected");
    publish(&root, &name, false);
    let reply = explain(&root, &json!({"node": "root/button"})).expect("the report renders");
    assert!(reply.contains("scope not-selected"), "{reply}");
    let _ = std::fs::remove_dir_all(&root);
}

/// Without a build there is no evidence, and the report says which command
/// produces it rather than inventing a verdict.
/// 没有构建就没有证据，报告要说出哪个命令会产出它，而不是编一个结论。
#[test]
fn a_missing_build_is_unknown_rather_than_guessed() {
    let (root, _) = package("no-build");
    let reply = explain(&root, &json!({"node": "root/button"})).expect("the report renders");
    assert!(reply.contains("scope unknown"), "{reply}");
    assert!(reply.contains("pruning unknown"), "{reply}");
    assert!(reply.contains("xirang check"), "{reply}");
    let _ = std::fs::remove_dir_all(&root);
}

/// A face whose manifest symbol is `-` has nothing tracked, and saying "strips -"
/// would read as a symbol named `-`. The end-to-end run against a real host is
/// what caught that wording.
/// 清单符号为 `-` 的面没有可跟踪的东西，而说"strips -"会被读成有个叫 `-` 的符号。真实宿主上的
/// 端到端运行抓到了这个措辞。
#[test]
fn a_face_with_no_tracked_symbol_is_not_reported_as_stripped() {
    let (root, name) = package("no-symbol");
    let out = root.join("target/xirang/out");
    std::fs::create_dir_all(&out).expect("build output");
    let faces = face_views(&root, &name).expect("faces derive");
    std::fs::write(
        out.join("pruning_manifest.tsv"),
        format!(
            "# node\tsource\tsymbol\n{}\t{}\t-\n",
            faces[0].id, faces[0].source
        ),
    )
    .expect("pruning");
    let reply = explain(&root, &json!({"node": "root/button"})).expect("the report renders");
    assert!(reply.contains("pruning nothing to strip"), "{reply}");
    assert!(!reply.contains("pruning strips -"), "{reply}");
    let _ = std::fs::remove_dir_all(&root);
}

/// A package with one face written through the write path and no build evidence yet:
/// every report has a node to answer about, and the freshness word starts out the
/// "stale" one.
/// 一个经写入路径写下一个面、尚无构建证据的包：每份报告都有节点可答，而新鲜度词从"stale"那一种开始。
fn generated_package(label: &str) -> (PathBuf, String) {
    let (root, name) = package(label);
    crate::mcp::apply::apply(
        &root,
        &json!({"action": "add", "apply": true, "fields": {"module": "label", "kind": "Label"}}),
    )
    .expect("the face is written");
    (root, name)
}

/// The same package with its build evidence just published by a `verify` run, so
/// `BuildEvidence::current` is true.
/// 同一个包，但构建证据刚由一次 `verify` 运行发布，因此 `BuildEvidence::current` 为真。
fn current_build_package(label: &str) -> (PathBuf, String) {
    let (root, name) = generated_package(label);
    crate::mcp::verify::verify(&root, &json!({}))
        .expect("the tree verifies and publishes the evidence");
    (root, name)
}

/// The freshness word is spelled in one place — `BuildEvidence::freshness()`, in this
/// module — and every report that prints it reads it: `explain` (its first caller),
/// `overlay`, `converge`, the tree `diff`, and the records `diff`. The others used to
/// spell it themselves, and nothing coupled the copies: renaming the word here left an
/// independent check's mutated binary answering the new word for `explain` while
/// `overlay`/`converge` kept the old one — with every test green (that check's mutation
/// `E`).
/// 新鲜度词只在一处拼出——本模块的 `BuildEvidence::freshness()`——而**每一份**打印它的报告都读
/// 它：`explain`（最早的调用方）、`overlay`、`converge`、树 `diff` 与记录 `diff`。其余的过去
/// 各拼一份，且没有任何钉子把副本耦合起来：在这里改名后，独立复核的变异二进制里 `explain` 换成了
/// 新词，而 `overlay`/`converge` 仍是旧词——全部测试仍然全绿（那次复核的变异 `E`）。
///
/// The assertions below therefore spell the word *literally*: they are the coupling. A
/// rename of the one source must fail every report here, and the failure is collected per
/// report rather than short-circuiting, so one run names *all* the reports that drifted.
/// 因此下面的断言把词**逐字**写出来：它们就是耦合本身。那一处改名必须让这里的每一份报告都失败，
/// 而失败是逐份收集的、不是首个就短路，因此一次运行就能点名**所有**漂移的报告。
#[test]
fn every_report_spells_the_freshness_word_that_one_place_produces() {
    let (current, _) = current_build_package("freshness-current");
    // A **real** stale state: evidence was published and then a source changed, so the fingerprint no
    // longer describes the tree. It used to be "written through the write path and no evidence yet",
    // and that stopped being a stable state the moment a write began starting the index refresh in
    // the background — the fixture would publish its own evidence a moment later and the report
    // would have been right to call it `current` (audit `M7`, P1.2).
    // 一个**真实的**过期状态：证据发布过，随后源码变了，因此指纹不再描述这棵树。它过去是"经写入路径写下、
    // 还没有证据"，而写入一开始在后台启动索引刷新，那就不再是一个稳定状态——夹具会在一瞬之后发布自己的证据，
    // 而报告那时叫它 `current` 是对的（审计 `M7`，P1.2）。
    let (stale, _) = generated_package("freshness-stale");
    // The write started the index refresh in the background; wait for the signal before making the
    // tree stale, so the fixture describes a state rather than a race.
    // 写入在后台启动了索引刷新；在把树弄成过期之前等那个信号，因此夹具描述的是一种状态而不是一场竞态。
    assert!(
        crate::mcp::index::wait_until_idle(&stale, std::time::Duration::from_secs(30)),
        "the refresh a write starts must finish"
    );
    // The edit has to land in a file the fingerprint **covers with content**: the raw file set is
    // hashed by path alone, and `src/lib.rs` is skipped by discovery, so touching it would leave the
    // stored fingerprint matching (see `discovery_fingerprint`'s two halves).
    // 改动的落点必须是指纹**连内容一起**覆盖的文件：原始文件集合只按路径取哈希，而 `src/lib.rs` 被发现
    // 过程跳过，因此动它对已存指纹毫无影响（见 `discovery_fingerprint` 的两半）。
    let face = stale.join("src/label/label.rs");
    let source = std::fs::read_to_string(&face).expect("the written face");
    std::fs::write(&face, format!("{source}// edited after the build\n")).expect("the edit lands");
    let (stale_build, stale_build_name) = package("freshness-stale-build");
    publish(&stale_build, &stale_build_name, true);
    let mut drifted = Vec::new();
    for (label, reply, expected) in [
        (
            "explain/current",
            explain(&current, &json!({"node": "root/label"})).expect("explain renders"),
            "\nbuild current\n",
        ),
        (
            "overlay/current",
            crate::mcp::overlay::overlay(&current, &json!({})).expect("overlay renders"),
            "\nbuild current\n",
        ),
        (
            "converge/current",
            crate::mcp::converge::converge(&current, &json!({"node": "root/label"}))
                .expect("converge renders"),
            "\nbuild current\n",
        ),
        (
            "diff/current",
            crate::mcp::diff::diff(&current, &json!({})).expect("the tree diff renders"),
            "\nbuild current\n",
        ),
        (
            "diff records/current",
            crate::mcp::diff::diff(&current, &json!({"records": true}))
                .expect("the records diff renders"),
            "\nbuild current\n",
        ),
        (
            "explain/stale",
            explain(&stale, &json!({"node": "root/label"})).expect("explain renders"),
            "\nbuild stale (run `xirang check`)",
        ),
        (
            "overlay/stale",
            crate::mcp::overlay::overlay(&stale, &json!({})).expect("overlay renders"),
            "\nbuild stale (run `xirang check`)",
        ),
        (
            "converge/stale",
            crate::mcp::converge::converge(&stale, &json!({"node": "root/label"}))
                .expect("converge renders"),
            "\nbuild stale (run `xirang check`)",
        ),
        (
            "diff records/stale",
            crate::mcp::diff::diff(&stale, &json!({"records": true}))
                .expect("the records diff renders"),
            "\nbuild stale (run `xirang check`)",
        ),
        // The tree diff has no build line at all when no evidence was ever published:
        // it answers with the command that produces one instead. The fixture above
        // (`publish`, no fingerprint) is the "known but stale" case, which does print
        // the line.
        // 从未发布过证据时树 diff 根本没有 build 行——它改为回答该用哪条命令产出证据。上面那份
        // fixture（`publish`，没有指纹）才是"已知但过期"的情形，那种会打印这一行。
        (
            "diff/stale",
            crate::mcp::diff::diff(&stale_build, &json!({})).expect("the tree diff renders"),
            "\nbuild stale (run `xirang check`)",
        ),
    ] {
        if !reply.contains(expected) {
            let printed = reply
                .lines()
                .find(|line| line.starts_with("build "))
                .unwrap_or("(no build line)");
            drifted.push(format!(
                "{label} printed `{printed}` instead of `{expected}`"
            ));
        }
    }
    assert!(
        drifted.is_empty(),
        "these reports spell the freshness word instead of reading `BuildEvidence::freshness()`: {drifted:#?}"
    );
    let _ = std::fs::remove_dir_all(&current);
    let _ = std::fs::remove_dir_all(&stale);
    let _ = std::fs::remove_dir_all(&stale_build);
}

/// The tree projection is bounded, because it is the one answer here that grows
/// with the project.
/// 树的投影是有上限的，因为它是这里唯一随项目变大的答案。
#[test]
fn the_tree_projection_is_bounded_by_limit() {
    let (root, name) = package("projection");
    publish(&root, &name, true);
    let reply = explain(&root, &json!({"limit": 1})).expect("the projection renders");
    assert!(reply.contains("faces:"), "{reply}");
    assert!(reply.contains("root/button"), "{reply}");
    let _ = std::fs::remove_dir_all(&root);
}

/// The bridge's `explain` says **who implements the slot** — and says so when nobody does.
/// 桥的 `explain` 说出**这个槽位由谁实现**——而在没有人实现它时也会说。
///
/// The CLI's `explain --overlay` printed `<- graft=… full=false (line N)` for the same slot while the bridge
/// said nothing at all, so an agent asking "who implements this face now" got a report that never answered
/// (audit `M7`, §M7.62). The negative half is deliberate, not noise: a view that stays silent when there is
/// nothing to hand over makes "nothing" indistinguishable from "this tool did not look" — which is the shape of
/// the defect this line closes.
/// CLI 的 `explain --overlay` 为同一个槽位打印 `<- graft=… full=false (line N)`，而桥什么都不说，于是问"这个面
/// 现在由谁实现"的代理拿到一份从不回答的报告（审计 `M7`，§M7.62）。否定的那一半是有意的、不是噪音：一个在没有
/// 东西可交时保持沉默的视图，会让"没有"与"这件工具根本没看"无法区分——而那正是本行要闭合的缺陷形状。
#[test]
fn the_report_says_who_implements_the_slot() {
    let (root, _name) = package("graft-line");
    let before = explain(&root, &json!({"node": "root/button"})).expect("a report");
    assert!(
        before.contains("graft  none — no declaration"),
        "a slot nothing hands over says so: {before}"
    );

    // Now declare it the way a host does: the entry names the slot with a Rust path.
    // 然后按宿主的方式声明它：入口用一条 Rust 路径点名这个槽位。
    std::fs::write(
        root.join("src/lib.rs"),
        "pub const FRAMEWORK: FrameworkId = FrameworkId::new(\"evidence.fixture\");\n\
         xirang_toolchain::run_method::static_graft_plan!(\n    FRAMEWORK,\n    \
         cut(crate::button::NODE_ID) graft(fast_button::fast::NODE_ID),\n);\n",
    )
    .expect("the entry");
    let after = explain(&root, &json!({"node": "root/button"})).expect("a report");
    assert!(
        after.contains("graft  <- fast_button::fast::NODE_ID full=false form=typed"),
        "the implementation is named, in the declaration's own spelling: {after}"
    );
    assert!(
        after.contains("(entry line 4)"),
        "and the line it was written on, so the reader can go there: {after}"
    );
    let _ = std::fs::remove_dir_all(&root);
}
