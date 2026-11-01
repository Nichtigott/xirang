//! Tests for `xirang.explain {"overlay": true}`: the published state after a
//! declared cut replaces its slot.
//! `xirang.explain {"overlay": true}` 的测试：已声明切口替换槽位之后的发布态。
//!
//! The traversal itself is pinned in `xirang_build_method`'s own tests, because
//! the CLI's `explain --overlay` reads it too. What this file pins is the bridge's
//! half: that the projection is what the tool renders, that a declared cut shows
//! up as the slot's replacement, that an unbuilt project is told the scope is
//! unknown instead of being shown a pruned tree, and that `node` is refused
//! rather than silently dropped.
//! 遍历本身钉在 `xirang_build_method` 自己的测试里，因为 CLI 的 `explain --overlay` 也读它。
//! 本文件钉的是桥的那一半：投影确实是本工具渲染的东西、已声明切口确实表现为该槽位的替换件、没构建过的
//! 项目会被如实告知作用域未知而不是看到一棵被剪过的树，以及 `node` 会被拒绝而不是被默默丢掉。

use std::path::{Path, PathBuf};

use serde_json::json;
use xirang_kernel::identity::NodeId;
use xirang_kernel::plugin::graft_document::GraftPlanDocument;

use super::overlay;

/// A throwaway package with one hand-written root face and an entry that declares a cut
/// naming it.
/// 一个含一个手写根面的一次性包，以及声明了一条点名该面的切口的入口。
fn package(label: &str) -> (PathBuf, String, NodeId) {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let name = format!("mcp-overlay-{label}");
    let root = std::env::temp_dir().join(format!("{name}-{}-{sequence}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("src/button")).expect("package directory");
    std::fs::write(
        root.join("Cargo.toml"),
        format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n"),
    )
    .expect("manifest");
    std::fs::write(
        root.join("src/lib.rs"),
        "// host entry\ncrate::static_graft_plan!(FRAMEWORK, cut \"root/button\" graft \"button_fast\");\n",
    )
    .expect("library target");
    std::fs::write(
        root.join("src/button/button.rs"),
        "pub struct Button;\n\ncrate::root_object! {\n    kind: Button,\n    parent: crate::root_node_id(env!(\"CARGO_PKG_NAME\")),\n}\n",
    )
    .expect("face");
    let id = crate::build_method::face_views(&root, &name).expect("faces derive")[0].id;
    (root, name, id)
}

/// Write one plan document under `.xirang/external-grafts/<selector>/graft.plan`.
/// 在 `.xirang/external-grafts/<selector>/graft.plan` 下写一份计划文档。
fn plan(root: &Path, selector: &str, document: &str) {
    let directory = root.join(".xirang/external-grafts").join(selector);
    std::fs::create_dir_all(&directory).expect("plan directory");
    std::fs::write(directory.join("graft.plan"), document).expect("plan file");
}

/// The projection is what the tool renders, and a declared cut is the slot's
/// replacement — the one thing no other tool says.
/// 投影就是本工具渲染的东西，而已声明切口就是该槽位的替换件——没有别的工具会说这件事。
#[test]
fn a_declared_cut_is_rendered_as_the_slots_replacement() {
    let (root, _, id) = package("replaced");
    plan(
        &root,
        "button_fast",
        &GraftPlanDocument::new(id, "root/button", "button_fast", false)
            .render_graft_plan_document(),
    );
    let reply = overlay(&root, &json!({})).expect("the projection renders");
    assert!(
        reply.contains("overlay (static projection of the build's scope and declared cuts)"),
        "{reply}"
    );
    assert!(reply.contains("slots 1 (replaced 1):"), "{reply}");
    assert!(
        reply.contains("root/button")
            && reply.contains("kind=Button")
            && reply.contains("<- graft=button_fast full=false form=string (entry line 2)"),
        "the declared cut is the slot's replacement: {reply}"
    );
    assert!(
        reply.contains("kept by cut `root/button` graft `button_fast` at entry line 2"),
        "{reply}"
    );
    assert!(
        reply.contains("note: static projection of the build's scope and declared cuts"),
        "the projection says what it is not: {reply}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// An unbuilt project hears that the scope is unknown and that nothing was
/// pruned, rather than being shown an empty published tree.
/// 没有构建过的项目听到的是"作用域未知、什么都没被剪掉"，而不是被展示一棵空的发布树。
#[test]
fn an_unbuilt_project_reports_an_unknown_scope_and_no_pruning() {
    let (root, _, _) = package("unbuilt");
    let reply = overlay(&root, &json!({})).expect("the projection renders");
    assert!(
        reply.contains("build stale (run `xirang check`)"),
        "{reply}"
    );
    assert!(
        reply.contains("scope unknown (no source_scope.tsv; run `xirang check`)"),
        "{reply}"
    );
    assert!(reply.contains("pruned 0:"), "{reply}");
    let _ = std::fs::remove_dir_all(&root);
}

/// A published scope that does not keep the face puts it in the pruned list,
/// which is the overlay's answer to "what does the release drop".
/// 已发布且不保留该面的作用域会把它放进剪枝清单，这正是覆盖对"发布会丢掉什么"的回答。
#[test]
fn a_published_scope_marks_the_slots_it_prunes() {
    let (root, name, _) = package("pruned");
    let out = root.join("target/xirang/out");
    crate::build_method::check_for(&root, &out, &name).expect("a valid host checks clean");
    // The fingerprint is over the sources, so replacing the scope manifest models
    // exactly what a narrowed build publishes — without setting a process-wide
    // environment variable two tests could race on.
    // 指纹是对源码取的，因此替换作用域清单模拟的正是收窄后的构建所发布的东西——不必去设置一个
    // 两条测试可能相互竞争的进程级环境变量。
    std::fs::write(
        out.join("source_scope.tsv"),
        "# mode\tauto\n# selected\t1\n# node\tsource\tmodule\n00000000000000000000000000000000\telsewhere.rs\tother\n",
    )
    .expect("narrowed scope manifest");

    let reply = overlay(&root, &json!({})).expect("the projection renders");
    assert!(reply.contains("build current"), "{reply}");
    assert!(reply.contains("slots 0 (replaced 0):"), "{reply}");
    assert!(reply.contains("pruned 1:"), "{reply}");
    assert!(reply.contains("root/button"), "{reply}");
    let _ = std::fs::remove_dir_all(&root);
}

/// An unreadable host entry is reported as such: the replacement column would
/// otherwise be a guess, and a guessed replacement is worse than none.
/// 宿主入口读不了时如实报出：否则替换那一列就是猜的，而猜出来的替换件比没有更糟。
#[test]
fn an_unreadable_entry_is_reported_instead_of_guessed() {
    let (root, _, _) = package("entry-unreadable");
    std::fs::remove_file(root.join("src/lib.rs")).expect("the entry is removed");
    std::fs::create_dir_all(root.join("src/lib.rs")).expect("a directory takes its place");
    let reply = overlay(&root, &json!({})).expect("the projection renders");
    assert!(reply.contains("entry unreadable ("), "{reply}");
    assert!(
        reply.contains("(replaced 0)"),
        "no replacement can be proven without a readable entry: {reply}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// `node` names one face; the projection walks every slot, so asking for both is
/// refused rather than answered with a silently dropped argument.
/// `node` 点名一个面；投影遍历每个槽位，因此同时要两者会被拒绝，而不是丢掉一个参数后作答。
#[test]
fn naming_one_face_is_refused_by_the_projection() {
    let (root, _, _) = package("node-conflict");
    let error = overlay(&root, &json!({"node": "root/button"})).expect_err("node is refused");
    assert!(error.contains("drop `node`"), "{error}");
    let _ = std::fs::remove_dir_all(&root);
}

/// The list bounds are bounds: more slots than `limit` truncate with the total
/// named.
/// 上限就是上限：槽位多于 `limit` 时截断并说出总数。
#[test]
fn the_projection_is_bounded_by_limit() {
    let (root, _, id) = package("bounded");
    for index in 0..3 {
        plan(
            &root,
            &format!("graft_{index}"),
            &GraftPlanDocument::new(id, "root/button", format!("graft_{index}"), false)
                .render_graft_plan_document(),
        );
    }
    let reply = overlay(&root, &json!({"limit": 1})).expect("the projection renders");
    assert!(reply.contains("plan records 3:"), "{reply}");
    assert!(
        reply.contains("… truncated: 2 of 3 plan records withheld at the limit of 1"),
        "{reply}"
    );
    let _ = std::fs::remove_dir_all(&root);
}
