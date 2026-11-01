//! Tests for the identity every later pass asks for, and for the per-pass facts it may answer from.
//! 每个后续 pass 都会问的身份，以及它可以据此作答的"按趟"事实。

use std::path::PathBuf;

use super::{Node, node_id};
use crate::build_method::discovery_cache::{CACHE_SCHEMA, FaceFactsScope, facts_of, remember_unit};
use crate::build_method::registry_identity::{NodeId, package_node_id};

/// The section's face, in the shipped shape.
/// 本节的面，按出厂形状。
const FACE: &str = "pub struct Button;\n\ncrate::root_object! {\n    kind: Button,\n    parent: crate::root_node_id(crate::XIRANG_NAMESPACE),\n}\n";

const RELATIVE: &str = "control/button/button.rs";

/// A package with exactly one face, and the node the walk would produce for it.
/// 只有一个面的包，以及遍历会为它产出的那个节点。
fn fixture(label: &str) -> (PathBuf, PathBuf, Node) {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "xirang-identity-{label}-{}-{sequence}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    let src = root.join("src");
    let file = src.join(RELATIVE);
    std::fs::create_dir_all(file.parent().expect("face directory")).expect("directories");
    std::fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"identity-probe\"\n",
    )
    .expect("manifest");
    std::fs::write(&file, FACE).expect("face");
    let node = Node {
        name: "button".to_owned(),
        file: Some(file),
        children: Vec::new(),
    };
    (root, src, node)
}

/// A unit row for one of the fixture's other faces.
/// 夹具里另一个面的单元行。
fn other_unit(kind: &str) -> String {
    format!(
        "# schema\t{CACHE_SCHEMA}\n# fingerprint\tprobe\npath\tcontrol/shelf/shelf.rs\nnode\t{}\nkind\t{kind}\n",
        package_node_id("control/shelf/shelf.rs", kind)
    )
}

/// A unit row for one face, with the kind the caller names.
/// 一个面的单元行，kind 由调用方点名。
fn unit(kind: &str) -> String {
    format!(
        "# schema\t{CACHE_SCHEMA}\n# fingerprint\tprobe\npath\t{RELATIVE}\nnode\t{}\nkind\t{kind}\n",
        package_node_id(RELATIVE, kind)
    )
}

/// The identity of a face is recomputed from the source when the cached row contradicts itself.
/// 当缓存行自相矛盾时，面的身份要从源码重新算出。
///
/// `valid_cached_unit` checks the schema, the path and the node id — it does **not** recheck the kind
/// against the node, because that is each reader's job (`prime_node_id_cache` and
/// `registration_phase::cached_face` both do it). This pin is the third reader's: a row whose kind was
/// altered while its node row was left alone is accepted by the cache and rejected by the reader, and
/// the answer is the source's. Without the recheck the name a consumer sees would come from bytes
/// nobody verified.
/// `valid_cached_unit` 检查 schema、路径与 node id——它**不**回头查 kind 与 node 是否相符，因为那是每个读取方
/// 自己的活（`prime_node_id_cache` 与 `registration_phase::cached_face` 都查）。这条钉子属于第三个读取方：一行
/// 被改过 kind、node 行却原封不动的记录会被缓存接受、被读取方拒绝，而答案是源码给的。没有那次互查，消费者看到的
/// 名字就来自没人验证过的字节。
#[test]
fn a_unit_whose_kind_contradicts_its_node_is_not_trusted() {
    let (root, src, node) = fixture("tampered-kind");
    let tampered = unit("Button").replace("kind\tButton", "kind\tSomethingElse");
    let _facts = FaceFactsScope::open();
    remember_unit(RELATIVE, &tampered);
    assert!(
        facts_of(RELATIVE).is_some(),
        "the store holds the row the pass just read"
    );
    assert_eq!(
        node_id(&src, &node),
        Some(package_node_id(RELATIVE, "Button")),
        "the identity comes from the source, not from the row nobody checked"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// The facts store is a **stack**: a layer dies with its pass, every layer hears what is read while it is
/// open, and a nested pass still starts empty.
/// 事实存储是一个**栈**：一层随它那一趟结束而死，每一层都听得见它开着的时候读到的东西，而嵌套的那一趟仍然从空开始。
///
/// Why "every layer": the build opens one store around the **whole run** and the discovery pass — a nested
/// one — is what learns the faces, while the passes that ask are the ones after it. Measured failure this
/// pins against (`stem 2`): with a single slot, the nested `open` *replaced* the outer map, every fact
/// landed in the inner one, and the outer the consumers could see stayed empty — the counter said
/// `facts=0` on a 1,000-face build ✗, and the wall clock could not see it at all.
/// 为什么"每一层"：构建在**整次运行**外面开一个存储，而学到面的是嵌套的那一趟（发现），问它们的是它之后的几趟。
/// 这条钉子针对的实测失败（`stem 2`）：单槽位时，内层的 `open` **换掉**了外层那张表，所有事实落进内层，而消费者能
/// 看见的外层始终是空的——计数器在 1,000 面的构建上报 `facts=0` ✗，而墙钟完全看不见。
#[test]
fn the_facts_store_dies_with_its_pass_and_every_layer_hears_what_is_read() {
    let (root, _src, _node) = fixture("per-pass");
    let other = "control/shelf/shelf.rs";
    assert!(
        facts_of(RELATIVE).is_none(),
        "with no pass open there are no facts"
    );
    {
        let _outer = FaceFactsScope::open();
        remember_unit(RELATIVE, &unit("Button"));
        assert_eq!(facts_of(RELATIVE).expect("the row").kind, "Button");
        {
            let _inner = FaceFactsScope::open();
            assert!(
                facts_of(RELATIVE).is_none(),
                "a nested pass starts empty: it does not inherit what the outer read before it"
            );
            // What the inner reads is heard by every open layer — that is what lets a run-level store
            // serve the passes that come after the one that did the reading.
            // 内层读到的东西，每一层开着的都会听见——这正是运行级存储能服务"读它的那一趟之后"的几趟的原因。
            remember_unit(other, &other_unit("Shelf"));
        }
        assert_eq!(
            facts_of(other).expect("the row the inner read").kind,
            "Shelf",
            "the outer layer heard what the inner one read"
        );
        assert_eq!(
            facts_of(RELATIVE).expect("the row the outer read").kind,
            "Button",
            "and what the outer itself read is not lost when a nested pass closes"
        );
    }
    assert!(
        facts_of(RELATIVE).is_none() && facts_of(other).is_none(),
        "the store dies with the outermost pass, taking every layer with it"
    );
    let _scope = FaceFactsScope::open();
    remember_unit(RELATIVE, "not a unit at all\n");
    assert!(
        facts_of(RELATIVE).is_none(),
        "a malformed cache is not facts, and the caller parses instead"
    );
    let _: NodeId = package_node_id(RELATIVE, "Button");
    let _ = std::fs::remove_dir_all(&root);
}
