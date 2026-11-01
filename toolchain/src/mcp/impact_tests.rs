//! Tests for the blast radius: descendants and capability consumers with their hop
//! distance and reasons, a cycle that terminates, a depth bound that says what it did
//! not reach, and a declared graft cut standing in the radius too.
//! 爆炸半径的测试：子面与能力消费者各自的距离与理由、会终止的环、说出"没走到哪"的深度上限，以及同样
//! 落在半径内的已声明 graft 切口。

use std::path::{Path, PathBuf};

use serde_json::json;

use super::impact;
use crate::mcp::apply::apply;

/// A throwaway package with a plain host entry and no graft declaration; the test
/// that needs one writes it, so the other radii stay about the tree and capabilities.
/// 一个只有朴素宿主入口、没有 graft 声明的一次性包；需要切口的那条测试自己写它，因此其它半径只关乎
/// 树与能力。
fn package(label: &str) -> (PathBuf, String) {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let name = format!("mcp-impact-{label}");
    let root = std::env::temp_dir().join(format!("{name}-{}-{sequence}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("src")).expect("package directory");
    std::fs::write(
        root.join("Cargo.toml"),
        format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n"),
    )
    .expect("manifest");
    std::fs::write(root.join("src/lib.rs"), "// host entry\n").expect("library target");
    (root, name)
}

/// `panel` owns a registry and provides `cap.render`; `slider` lives under it, needs
/// that capability and provides `cap.paint`; `knob` lives under `slider` and needs
/// `cap.paint`. That is one capability edge and two tree edges over three faces.
/// `panel` 拥有注册机并提供 `cap.render`；`slider` 在它之下、需要该能力并提供 `cap.paint`；
/// `knob` 在 `slider` 之下、需要 `cap.paint`。三个面之上一共一条能力边、两条树边。
fn chain(root: &Path, cycle: bool) {
    apply(
        root,
        &json!({"action": "add", "apply": true,
                "fields": {"module": "panel", "kind": "Panel", "needs_registry": true,
                           "provides": "cap.render"}}),
    )
    .expect("the provider is admitted");
    apply(
        root,
        &json!({"action": "add", "parent": "root/panel", "apply": true,
                "fields": {"module": "slider", "kind": "Slider", "needs_registry": true,
                           "provides": "cap.paint", "requires": "cap.render=>Panel"}}),
    )
    .expect("the consumer is admitted under its provider");
    apply(
        root,
        &json!({"action": "add", "parent": "root/panel/slider", "apply": true,
                "fields": {"module": "knob", "kind": "Knob", "requires": "cap.paint=>Slider"}}),
    )
    .expect("the grandchild is admitted");
    if cycle {
        // The cycle is introduced by a *later* edit rather than written up front: the
        // kernel refuses a requirement whose provider does not exist yet, and that is
        // the same rule that makes an orphaned descendant impossible.
        // 环是由**后续**的编辑引入的，而不是一开始就写出来的：内核拒绝提供者尚不存在的需求，而正是
        // 同一条规则让"失去答案的后代"不可能存在。
        apply(
            root,
            &json!({"action": "edit", "node": "root/panel", "apply": true,
                    "fields": {"requires": "cap.paint=>Slider"}}),
        )
        .expect("the cycle is admitted once its provider exists");
    }
}

/// A change to the provider reaches its child and its consumer at one hop, and the
/// grandchild at two — and the node that is both says *both* reasons.
/// 对提供者的一次改动，一跳到达它的子面与消费者，两跳到达孙面——而两者兼具的那个节点会把两条理由
/// 都写出来。
#[test]
fn the_radius_walks_children_and_capability_consumers_with_their_hops() {
    let (root, _) = package("hops");
    chain(&root, false);
    let reply = impact(&root, &json!({"node": "root/panel"})).expect("the radius renders");
    assert!(reply.contains("faces 3  depth 3"), "{reply}");
    assert!(
        reply.contains("affected 2 (transitive within depth)"),
        "{reply}"
    );
    assert!(reply.contains("hop 1  root/panel/slider"), "{reply}");
    assert!(
        reply.contains("because: child of this face's registry (kind Slider)"),
        "{reply}"
    );
    assert!(
        reply.contains("because: consumer: requires `cap.render=>Panel`"),
        "a node that is both a child and a consumer must say both: {reply}"
    );
    assert!(reply.contains("hop 2  root/panel/slider/knob"), "{reply}");
    assert!(
        reply.contains("how: root/panel -> root/panel/slider [child of this face's registry (kind Slider)] -> root/panel/slider/knob"),
        "the chain names every hop: {reply}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// The bound is a bound, and what it left out is stated as such: unreached is not
/// independence.
/// 上限就是上限，而被它落下的会如实说明：未到达不等于独立。
#[test]
fn the_depth_bound_says_what_it_did_not_reach() {
    let (root, _) = package("depth");
    chain(&root, false);
    let shallow = impact(&root, &json!({"node": "root/panel", "depth": 1})).expect("the render");
    assert!(shallow.contains("depth 1"), "{shallow}");
    assert!(
        shallow.contains("affected 1 (transitive within depth)"),
        "{shallow}"
    );
    assert!(
        shallow.contains("not reached within depth 1 1 declared face(s)"),
        "{shallow}"
    );
    assert!(!shallow.contains("root/panel/slider/knob"), "{shallow}");
    let _ = std::fs::remove_dir_all(&root);
}

/// A capability cycle terminates and is counted rather than walked, and it cannot
/// move a node's distance: the shortest path still wins.
/// 能力环会终止、被计数而不是被走下去，而且不会移动任何节点的距离：最短路径仍然胜出。
#[test]
fn a_capability_cycle_is_counted_and_the_shortest_path_still_wins() {
    let (root, _) = package("cycle");
    chain(&root, true);
    let reply = impact(&root, &json!({"node": "root/panel", "depth": 8})).expect("the render");
    assert!(
        reply.contains("affected 2 (transitive within depth)"),
        "{reply}"
    );
    assert!(reply.contains("hop 1  root/panel/slider"), "{reply}");
    assert!(reply.contains("hop 2  root/panel/slider/knob"), "{reply}");
    assert!(reply.contains("cycles back to this face 1"), "{reply}");
    let _ = std::fs::remove_dir_all(&root);
}

/// A face nothing depends on has an empty radius, and the reply says how many faces
/// that is rather than showing an empty list.
/// 没有任何东西依赖的面半径为空，而回复会说清那是几个面，而不是给一份空清单。
#[test]
fn a_leaf_face_has_an_empty_radius() {
    let (root, _) = package("leaf");
    chain(&root, false);
    let reply = impact(&root, &json!({"node": "root/panel/slider/knob"})).expect("the render");
    assert!(
        reply.contains("affected 0 (transitive within depth)"),
        "{reply}"
    );
    assert!(
        reply.contains("not reached within depth 3 2 declared face(s)"),
        "{reply}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// The declared graft cut that names a face is part of its radius: a cut hands that
/// face over, so a change to it is a change to what the graft receives.
/// 点名一个面的已声明 graft 切口属于它的半径：切口会把该面交出去，因此对该面的改动的确会改变 graft
/// 收到的那个东西。
#[test]
fn a_declared_cut_that_names_the_face_is_in_the_radius() {
    let (root, _) = package("cut");
    std::fs::write(
        root.join("src/lib.rs"),
        "// host entry\ncrate::static_graft_plan!(FRAMEWORK, cut \"root/panel\" graft \"panel_fast\");\n",
    )
    .expect("the entry declares the cut");
    chain(&root, false);
    let reply = impact(&root, &json!({"node": "root/panel"})).expect("the render");
    assert!(
        reply.contains("hop 1  graft `root/panel`"),
        "the declared cut must appear as a reachable node: {reply}"
    );
    assert!(
        reply.contains("because: declared cut at entry line"),
        "{reply}"
    );
    assert!(
        reply.contains("affected 3 (transitive within depth)"),
        "the cut joins the radius next to the two faces: {reply}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// A registration file the derivation cannot parse is counted in the reply instead of vanishing:
/// a read-only tree query used to report a smaller tree as if it were the whole one, which is what
/// `LGC-LG-11` recorded on the producer side and what the consumer half now says out loud.
/// 推导解析不了的注册面文件被计入回复而不是消失：只读的树查询过去把一棵更小的树当成完整的树报出去
/// ——这正是 `LGC-LG-11` 在生产端记录的事，而消费端现在把它说出来。
fn broken_face(root: &std::path::Path, label: &str) {
    let directory = root.join("src").join(label);
    std::fs::create_dir_all(&directory).expect("module directory");
    std::fs::write(
        directory.join(format!("{label}.rs")),
        "crate::root_object! {\n    kind: Broken,\n",
    )
    .expect("truncated face");
}

#[test]
fn the_radius_counts_unparsable_registration_files() {
    let (root, _) = package("unparsable");
    chain(&root, false);
    broken_face(&root, "broken");
    let reply = impact(&root, &json!({"node": "root/panel"})).expect("the radius renders");
    assert!(reply.contains("unparsable faces 1"), "{reply}");
    let _ = std::fs::remove_dir_all(&root);
}
