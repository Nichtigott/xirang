//! Pins for `xirang.graph`: what it answers from the published graph, what it refuses, and that
//! every answer carries the index line (audit `M7`, P1.3).
//! `xirang.graph` 的钉子：它从已发布的图里答什么、拒绝什么，以及每份答案都带索引那一行（审计 `M7`，P1.3）。

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::json;

/// A throwaway package with two faces whose files call each other's uniquely declared function, so
/// the dependency graph has a cycle — the shape a partition must not split (audit `M7`, P1.3).
/// 一个一次性包：两个面，各自的文件调用对方唯一声明的函数，因此依赖图上有一个环——那是分区绝不能切开的
/// 形状（审计 `M7`，P1.3）。
fn package(label: &str) -> (PathBuf, String) {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let name = format!("mcp-graph-{label}");
    let root = std::env::temp_dir().join(format!("{name}-{}-{sequence}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("src")).expect("package directory");
    fs::write(
        root.join("Cargo.toml"),
        format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n"),
    )
    .expect("manifest");
    fs::write(root.join("src/lib.rs"), "// host entry\n").expect("library target");
    fs::create_dir_all(root.join("src/dial/registry_rule")).expect("dial directory");
    // The rule beside the face that owns a registry: `needs_registry: true` without a rule makes the
    // generated module read a `super::registry_rule` nothing mounts, and the build refuses that tree
    // by name. This fixture called itself healthy while leaving the file out — it was a tree no
    // `cargo build` could compile, and it only stayed invisible because nothing judged it.
    // 拥有注册机的面旁边那份规则：只有 `needs_registry: true` 而没有规则，会让生成的模块去读一个没有任何东西
    // 挂载的 `super::registry_rule`，而构建按名拒绝那棵树。本夹具自称"健康的树"却没有这个文件——它其实是一棵
    // 任何 `cargo build` 都编译不过的树，只因为没人判它才一直看不见。
    fs::write(
        root.join("src/dial/registry_rule/registry_rule.rs"),
        "use crate::RegistrationRule;\n\n\
         pub const REGISTRATION_RULE: RegistrationRule = RegistrationRule::new();\n",
    )
    .expect("dial rule");
    fs::write(
        root.join("src/dial/dial.rs"),
        "crate::root_object! {\n    kind: Dial,\n    parent: crate::root_node_id(env!(\"CARGO_PKG_NAME\")),\n    \
         needs_registry: true,\n}\npub fn dial_work() { other_work() }\n",
    )
    .expect("dial face");
    fs::create_dir_all(root.join("src/other")).expect("other directory");
    fs::write(
        root.join("src/other/other.rs"),
        "crate::control_object! {\n    kind: Other,\n    parent: crate::dial::NODE_ID,\n}\n\
         pub fn other_work() { dial_work() }\n",
    )
    .expect("other face");
    (root, name)
}

/// Publish the index the way a real run does.
/// 像真实的一次运行那样发布索引。
fn publish(root: &Path, name: &str) {
    crate::build_method::check_for(root, &crate::mcp::build_evidence::out_dir(root), name)
        .expect("a healthy tree checks clean");
}

/// The census names what the graph holds, and the answer opens with the index line (audit `M7`,
/// P1.3).
/// 普查说出图里有什么，而答案以索引那一行开头（审计 `M7`，P1.3）。
#[test]
fn the_census_opens_with_the_index_line_and_counts_the_edges() {
    let (root, name) = package("census");
    publish(&root, &name);
    let answer = super::graph(&root, &json!({})).expect("the census answers");
    let mut lines = answer.lines();
    assert!(
        lines
            .next()
            .unwrap_or_default()
            .starts_with("graph updated: generation 1, "),
        "the index line comes first: {answer}"
    );
    assert!(
        answer.contains("calls-file 2"),
        "both calls cross files: {answer}"
    );
    // Two `parent` edges: the child names the face, and the face names the package root.
    // 两条 `parent` 边：子面点名那个面，而那个面点名包根。
    assert!(
        answer.contains("parent 2"),
        "and the parent edges are there: {answer}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// A node named the way the **record** names it resolves to a face and shows its neighbourhood
/// (audit `M7`, P1.3).
/// 用**记录**的方式点名的节点会解析到一个面，并给出它的邻域（审计 `M7`，P1.3）。
#[test]
fn a_logical_path_resolves_through_the_record_and_shows_its_edges() {
    let (root, name) = package("node");
    publish(&root, &name);
    // Two hops: the face is `in` its file, and the file is what calls into the other file — the
    // dependency is a fact about files, so one hop from a face cannot see it.
    // 两跳：面 `in` 在它的文件里，而调用另一个文件的是那份文件——依赖是关于文件的事实，因此从面出发的一跳
    // 看不到它。
    let answer = super::graph(
        &root,
        &json!({"node": "Other", "direction": "out", "depth": 2}),
    )
    .expect("the neighbourhood answers");
    assert!(
        answer.contains("face:") && answer.contains("node   Other -> face:"),
        "the query is resolved to an identity: {answer}"
    );
    assert!(
        answer.contains("file:other/other.rs") && answer.contains("calls-file"),
        "its file and its dependency are both named: {answer}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// A name the record does not know is refused with the spellings it does know (audit `M7`, P1.3).
/// 记录不认识的名字会被拒绝，并带上它认识的拼写（审计 `M7`，P1.3）。
#[test]
fn a_name_the_record_does_not_know_is_refused_with_the_spellings_it_does() {
    let (root, name) = package("unknown");
    publish(&root, &name);
    let error = super::graph(&root, &json!({"node": "no/such/face"})).expect_err("the refusal");
    assert!(error.contains("logical path"), "{error}");
    assert!(error.contains("`face:`"), "{error}");
    let _ = std::fs::remove_dir_all(&root);
}

/// The two faces' files call each other, so `cycles` names them as one component that must stay
/// together (audit `M7`, P1.3).
/// 两个面的文件互相调用，因此 `cycles` 把它们点名为必须留在一起的一个分量（审计 `M7`，P1.3）。
#[test]
fn the_cycle_between_the_two_files_is_named_as_one_component() {
    let (root, name) = package("cycles");
    publish(&root, &name);
    let answer = super::graph(&root, &json!({"cycles": true})).expect("the cycles answer");
    assert!(
        answer.contains("component 1 (2 nodes)"),
        "the two files are inseparable: {answer}"
    );
    assert!(
        answer.contains("file:dial/dial.rs") && answer.contains("file:other/other.rs"),
        "and both are named: {answer}"
    );
    assert!(
        answer.contains("not dependencies and are not counted here"),
        "the answer says which edges it did not count: {answer}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// An index that is behind is still answered from, and the answer says so (audit `M7`, P2.3).
/// 落后的索引仍被据以作答，而答案会说出来（审计 `M7`，P2.3）。
#[test]
fn a_behind_index_is_answered_from_and_named() {
    let (root, name) = package("behind");
    publish(&root, &name);
    fs::write(
        root.join("src/dial/dial.rs"),
        "crate::root_object! {\n    kind: Dial,\n    parent: crate::root_node_id(env!(\"CARGO_PKG_NAME\")),\n    \
         needs_registry: true,\n}\npub fn dial_work() { other_work() }\npub fn later() -> i32 { 1 }\n",
    )
    .expect("the edit lands");
    let answer = super::graph(&root, &json!({})).expect("the census still answers");
    assert!(
        answer
            .lines()
            .next()
            .unwrap_or_default()
            .starts_with("index behind: generation 1 covers "),
        "the first line names the staleness: {answer}"
    );
    assert!(
        answer.contains("calls-file"),
        "and the edges are still read: {answer}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// A tree that never published an index says what to run rather than inventing an empty graph
/// (audit `M7`, P1.3).
/// 从未发布过索引的树会说出该跑什么，而不是编一个空图（审计 `M7`，P1.3）。
#[test]
fn an_unpublished_index_says_what_to_run() {
    let (root, _) = package("absent");
    let answer = super::graph(&root, &json!({})).expect("the answer is a sentence, not a failure");
    assert!(
        answer.contains("index not published yet: run `xirang check`"),
        "{answer}"
    );
    assert!(answer.contains("graph  unavailable"), "{answer}");
    let _ = std::fs::remove_dir_all(&root);
}
