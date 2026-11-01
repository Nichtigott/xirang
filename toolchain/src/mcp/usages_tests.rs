//! Tests for the neighbourhood report: tree edges, the fields the write path
//! accepts read back, and capability references in both directions.
//! 邻域报告的测试：树的边、写入路径接受的那些字段被读回，以及双向的能力引用。

use std::path::{Path, PathBuf};

use serde_json::json;

use super::usages;
use crate::mcp::apply::apply;

/// A throwaway package with no faces: every face in it is one the test adds.
/// 一个没有注册面的一次性包：它里面的每个面都是测试加进去的。
fn package(label: &str) -> (PathBuf, String) {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let name = format!("mcp-usages-{label}");
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

/// Add one generated face through the write path, so the executor owns it.
/// 经写入路径添加一个生成的面，让它归执行器所有。
fn add(root: &Path, fields: serde_json::Value) {
    let mut request = json!({"action": "add", "apply": true, "fields": fields});
    if let Some(object) = request
        .get_mut("fields")
        .and_then(|value| value.as_object_mut())
    {
        object.entry("kind").or_insert(json!("Face"));
    }
    apply(root, &request).expect("the face is added");
}

/// The fields `xirang.apply` accepts as input come back out: before this, the
/// write path could set a contract and no tool could report it.
/// `xirang.apply` 作为输入接受的字段能读回来：在这之前，写入路径能设置契约，而没有任何工具能
/// 报告它。
#[test]
fn the_fields_the_write_path_accepts_are_read_back() {
    let (root, _) = package("fields");
    add(
        &root,
        json!({"module": "label", "kind": "Label", "provides": "cap.render", "name_en": "Label"}),
    );
    let reply = usages(&root, &json!({"node": "root/label"})).expect("the report renders");
    assert!(reply.contains("fields (read back"), "{reply}");
    assert!(reply.contains("provides cap.render"), "{reply}");
    assert!(reply.contains("name_en Label"), "{reply}");
    assert!(reply.contains("requires -"), "{reply}");
    let _ = std::fs::remove_dir_all(&root);
}

/// A child face is reported under its parent, with the logical path the registry
/// reports rather than the file's path. The parent has to *own* a registry for the
/// kernel to admit a child at all — the first version of this fixture was refused
/// for exactly that reason.
/// 子面被报在它的父级之下，用的是注册机报告的逻辑路径而不是文件路径。父级必须**拥有**一个注册机，
/// 内核才会准入一个子面——本测试的第一版 fixture 正是因为这一点被拒绝的。
#[test]
fn children_are_listed_under_their_parent() {
    let (root, _) = package("children");
    add(
        &root,
        json!({"module": "label", "kind": "Label", "needs_registry": true}),
    );
    apply(
        &root,
        &json!({"action": "add", "parent": "root/label", "apply": true,
                "fields": {"module": "knob", "kind": "Knob"}}),
    )
    .expect("the child is added");
    let reply = usages(&root, &json!({"node": "root/label"})).expect("the report renders");
    assert!(reply.contains("children (1)"), "{reply}");
    assert!(reply.contains("root/label/knob"), "{reply}");
    let _ = std::fs::remove_dir_all(&root);
}

/// Capability references are reported in both directions, and the reply says the
/// match is on declared tokens rather than a resolved graph. The requirement is
/// written the way the kernel reads it — `capability=>ProviderKind` — and the
/// consumer sits under the provider, because a requirement has to be answerable by
/// an ancestor. The first version of this fixture wrote a bare capability name and
/// the kernel refused it by name: "requires entries must use capability=>provider
/// syntax".
/// 能力引用双向报告，并且回复说明匹配发生在声明的记号上，而不是一棵已解析的图。需求按内核读取的
/// 写法给出——`capability=>ProviderKind`——而消费者位于提供者之下，因为需求必须能被某个祖先满足。
/// 本测试的第一版把需求写成裸能力名，内核按名拒绝了它："requires entries must use
/// capability=>provider syntax"。
#[test]
fn capability_references_are_reported_in_both_directions() {
    let (root, _) = package("capabilities");
    add(
        &root,
        json!({"module": "label", "kind": "Label", "needs_registry": true, "provides": "cap.render"}),
    );
    apply(
        &root,
        &json!({"action": "add", "parent": "root/label", "apply": true,
                "fields": {"module": "slider", "kind": "Slider", "requires": "cap.render=>Label"}}),
    )
    .expect("the consumer is admitted under its provider");
    let provider = usages(&root, &json!({"node": "root/label"})).expect("the report renders");
    assert!(
        provider.contains("capability refs (matched on declared tokens, not resolved)"),
        "{provider}"
    );
    assert!(provider.contains("consumers (1)"), "{provider}");
    assert!(
        provider.contains("root/label/slider (cap.render)"),
        "{provider}"
    );
    let consumer =
        usages(&root, &json!({"node": "root/label/slider"})).expect("the report renders");
    assert!(consumer.contains("providers (1)"), "{consumer}");
    assert!(consumer.contains("root/label (cap.render)"), "{consumer}");
    let _ = std::fs::remove_dir_all(&root);
}

/// A hand-written module has no generated field list, and the executor refuses to
/// invent one — so the report says the fields are unreadable and counts how many
/// faces that affected, rather than pretending they are empty.
/// 手写模块没有生成的字段清单，执行器也拒绝凭空造一个——因此报告说字段读不回来，并数出影响了多少个面，
/// 而不是假装它们是空的。
#[test]
fn a_hand_written_face_is_reported_as_unreadable() {
    let (root, _) = package("hand-written");
    std::fs::create_dir_all(root.join("src/manual")).expect("module directory");
    std::fs::write(
        root.join("src/manual/manual.rs"),
        "pub struct Manual;\n\ncrate::root_object! {\n    kind: Manual,\n    parent: crate::root_node_id(env!(\"CARGO_PKG_NAME\")),\n}\n",
    )
    .expect("hand-written face");
    let reply = usages(&root, &json!({"node": "root/manual"})).expect("the report renders");
    assert!(reply.contains("fields unreadable"), "{reply}");
    assert!(reply.contains("not generated by XiRang"), "{reply}");
    assert!(reply.contains("unreadable faces 1"), "{reply}");
    let _ = std::fs::remove_dir_all(&root);
}

/// Every field the write path accepts is read back, not most of them. `module`, the two
/// summaries and `flow_provider` were missing while the header claimed the fields were read
/// back, so an agent asking "what can I set" got a shorter answer than the tool's contract.
/// 写入路径接受的每个字段都会被读回，而不是大部分。`module`、两条摘要与 `flow_provider` 过去缺失，
/// 而表头声称字段已被读回——一个问"我能设什么"的代理因此拿到比工具契约更短的答案。
#[test]
fn every_field_the_write_path_accepts_is_read_back() {
    let (root, _) = package("all-fields");
    add(
        &root,
        json!({
            "module": "label",
            "kind": "Label",
            "summary_zh": "标签",
            "summary_en": "A label",
            "flow_provider": "crate::ControlHandle",
        }),
    );
    let reply = usages(&root, &json!({"node": "root/label"})).expect("the report renders");
    for expected in [
        "module label",
        "summary_zh 标签",
        "summary_en A label",
        "flow_provider crate::ControlHandle",
    ] {
        assert!(
            reply.contains(expected),
            "`{expected}` is one of the fields the write path accepts: {reply}"
        );
    }
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
fn the_neighbourhood_counts_unparsable_registration_files() {
    let (root, _) = package("unparsable");
    add(&root, json!({"module": "label", "kind": "Label"}));
    broken_face(&root, "broken");
    let reply = usages(&root, &json!({"node": "root/label"})).expect("the report renders");
    assert!(reply.contains("unparsable faces 1"), "{reply}");
    let _ = std::fs::remove_dir_all(&root);
}
