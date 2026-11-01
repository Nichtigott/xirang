//! Default exploration must carry responsibility evidence without an extra tool call.
//! 默认探索应携带责任证据，无需再调用别的工具。

use super::{Tree, json};
use crate::mcp::apply::apply;
use crate::run_method::{
    CallTrace, LocalKind, SourceLocation, trace_artifact_path, write_trace_artifact,
};

fn package() -> Tree {
    let tree = Tree::new();
    tree.write_source(
        "Cargo.toml",
        "[package]\nname=\"explore-chain\"\nversion=\"0.1.0\"\nedition=\"2024\"\n",
    );
    tree.write_source("src/lib.rs", "// host\n");
    apply(&tree.0, &json!({"action":"add","apply":true,"fields":{"module":"provider","kind":"Provider","needs_registry":true,"provides":"cap.render"}})).unwrap();
    for module in ["consumer", "sibling"] {
        apply(&tree.0, &json!({"action":"add","parent":"root/provider","apply":true,"fields":{"module":module,"kind":if module == "consumer" {"Consumer"} else {"Sibling"},"requires":"cap.render=>Provider"}})).unwrap();
    }
    for (path, function) in [
        (
            "src/provider/provider.rs",
            "pub fn produce(n: i32) -> i32 { n * 2 }\n",
        ),
        (
            "src/provider/object/consumer/consumer.rs",
            "/// Flurb quantity becomes negative.\npub fn symptom() -> i32 { produce(-1) }\n",
        ),
        (
            "src/provider/object/sibling/sibling.rs",
            "pub fn other() -> i32 { produce(2) }\n",
        ),
    ] {
        let original = std::fs::read_to_string(tree.0.join(path)).unwrap();
        tree.write_source(path, &format!("{original}\n{function}"));
    }
    assert!(crate::mcp::index::wait_until_idle(
        &tree.0,
        std::time::Duration::from_secs(30)
    ));
    let faces = crate::build_method::face_views(&tree.0, "explore-chain").unwrap();
    let id = |kind| faces.iter().find(|face| face.kind == kind).unwrap().id;
    let mut trace = CallTrace::full();
    trace.with(id("Consumer"), "symptom", |trace| {
        let input = trace.local("request", "i32", -1, LocalKind::Binding);
        trace.with_at(
            id("Provider"),
            "produce",
            SourceLocation {
                file: "src/provider/object/consumer/consumer.rs",
                line: 40,
                column: 1,
                function: "symptom",
            },
            |trace| {
                trace.transform(input, "answer", "i32", -2);
            },
        );
    });
    write_trace_artifact(&trace, &trace_artifact_path(&tree.0), "explore-chain").unwrap();
    tree
}

#[test]
fn default_symbol_query_returns_source_contracts_runtime_and_other_consumers() {
    let tree = package();
    let answer = tree.ask(json!({"query":"symptom"}));
    for expected in [
        "pub fn symptom",
        "responsibility chain",
        "cap.render => Provider",
        "verify root/provider/sibling",
        "root/provider::answer",
    ] {
        assert!(answer.contains(expected), "missing {expected}: {answer}");
    }
    assert!(
        answer.contains("not observed; absence is not proof of bypass"),
        "{answer}"
    );
    assert!(
        answer.contains("pub fn other"),
        "related consumer source must outrank unrelated lexical hits: {answer}"
    );
}

#[test]
fn logical_object_query_reads_only_related_sources_even_without_build_records() {
    let tree = package();
    let out = tree.0.join("target/xirang/out");
    if out.exists() {
        std::fs::remove_dir_all(out).unwrap();
    }
    // Non-UTF8 unrelated source makes accidental whole-tree source reads observable.
    // 无关的非 UTF8 源码使意外整树读取成为可观察失败。
    std::fs::write(tree.0.join("src/noise.rs"), [255]).unwrap();
    let answer = tree.ask(json!({"query":"root/provider/consumer"}));
    assert!(answer.contains("structural source scope"), "{answer}");
    assert!(answer.contains("3 Rust files scanned"), "{answer}");
    assert!(
        answer.contains("pub fn symptom") && answer.contains("pub fn other"),
        "{answer}"
    );
    assert!(
        answer.contains("outside") && answer.contains("not examined"),
        "{answer}"
    );
}

#[test]
fn scoped_requests_do_not_silently_expand_to_package_evidence() {
    let tree = package();
    let answer = tree.ask(json!({"query":"symptom","scope":["src/provider/object/consumer"]}));
    assert!(answer.contains("1 Rust files scanned"), "{answer}");
    assert!(
        answer.contains("responsibility join not evaluated outside explicit scope"),
        "{answer}"
    );
    assert!(!answer.contains("root/provider::answer"), "{answer}");
}

#[test]
fn source_receipts_cannot_hide_changed_consumer_obligations_or_runtime() {
    let tree = package();
    let first = tree.ask(json!({"query":"symptom", "known_context":[]}));
    let known: Vec<_> = first
        .lines()
        .filter_map(|line| line.strip_prefix("context receipt: "))
        .map(str::to_owned)
        .collect();
    apply(&tree.0, &json!({"action":"edit","node":"root/provider/sibling","apply":true,"fields":{"requires":""}})).unwrap();
    std::fs::remove_file(trace_artifact_path(&tree.0)).unwrap();
    let changed = tree.ask(json!({"query":"symptom","known_context":known}));
    assert!(changed.contains("trace absent"), "{changed}");
    assert!(
        !changed.contains("verify root/provider/sibling"),
        "{changed}"
    );
    assert!(!changed.contains("root/provider::answer"), "{changed}");
    assert!(changed.contains("cap.render => Provider"), "{changed}");
}

#[test]
fn structural_evidence_and_source_share_the_existing_byte_budget() {
    let tree = package();
    for budget in [2000, 4000, 14000] {
        let answer = tree.ask(json!({"query":"symptom","max_bytes":budget,"known_context":[]}));
        assert!(
            answer.len() <= budget,
            "{} > {budget}: {answer}",
            answer.len()
        );
        assert!(
            answer.contains("pub fn symptom"),
            "source must retain a budget: {answer}"
        );
        assert!(answer.contains("responsibility"), "{answer}");
    }
}

#[test]
fn helper_functions_in_a_faces_parts_inherit_declared_responsibility() {
    let tree = package();
    tree.write_source(
        "src/provider/part/math.rs",
        "pub fn coordinate_math(n: i32) -> i32 { n + 1 }\n",
    );
    let answer = tree.ask(json!({"query":"coordinate_math"}));
    for expected in [
        "pub fn coordinate_math",
        "responsibility focus: root/provider",
        "verify root/provider/consumer",
        "verify root/provider/sibling",
    ] {
        assert!(answer.contains(expected), "missing {expected}: {answer}");
    }
}

#[test]
fn ambiguous_function_owners_are_joined_together_without_choosing_a_dispatch_target() {
    let tree = package();
    apply(
        &tree.0,
        &json!({"action":"add","apply":true,"fields":{"module":"other","kind":"Other"}}),
    )
    .unwrap();
    let path = tree.0.join("src/other/other.rs");
    let source = std::fs::read_to_string(&path).unwrap();
    std::fs::write(path, format!("{source}\nfn symptom() -> i32 {{ 99 }}\n")).unwrap();
    let answer = tree.ask(json!({"query":"symptom"}));
    assert!(answer.contains("2 declared owners"), "{answer}");
    assert!(
        answer.contains("root/other") && answer.contains("root/provider/consumer"),
        "{answer}"
    );
    assert!(answer.contains("not resolved dispatch"), "{answer}");
}

#[test]
fn handwritten_native_faces_keep_their_contracts_in_the_default_entry() {
    let tree = Tree::new();
    tree.write_source(
        "Cargo.toml",
        "[package]\nname=\"hand-chain\"\nversion=\"0.1.0\"\nedition=\"2024\"\n",
    );
    tree.write_source("src/lib.rs", "// host\n");
    tree.write_source("src/provider/provider.rs", "pub struct Provider;\ncrate::root_object! { kind: Provider, parent: crate::root_node_id(env!(\"CARGO_PKG_NAME\")), needs_registry: true, provides: [\"cap.render\"], }\npub fn supply() {}\n");
    tree.write_source("src/provider/object/child/child.rs", "pub struct Child;\ncrate::provider_object! { kind: Child, parent: crate::provider::NODE_ID, requires: [\"cap.render\" => \"Provider\"], }\npub fn consume() { supply(); }\n");
    let answer = tree.ask(json!({"query":"supply"}));
    assert!(answer.contains("provides: cap.render"), "{answer}");
    assert!(answer.contains("verify root/provider/child"), "{answer}");
    assert!(!answer.contains("contract fields unreadable"), "{answer}");
}

#[test]
fn comments_and_string_literals_cannot_trigger_a_registration_join() {
    let tree = Tree::new();
    tree.write_source("Cargo.toml", "deliberately invalid manifest");
    tree.write_source("src/lib.rs", "// crate::root_object! { kind: Bogus }\nfn query() -> &'static str { \"root_object! { kind: Pretend }\" }\n");
    let answer = tree.ask(json!({"query":"query"}));
    assert!(answer.contains("fn query"), "{answer}");
    assert!(
        !answer.contains("responsibility"),
        "no native declaration means no Cargo/tree analysis: {answer}"
    );
}

#[test]
fn warm_ordinary_index_cannot_hide_added_moved_or_deleted_consumers() {
    let tree = package();
    let relative = "src/provider/object/newcomer/newcomer.rs";
    let declaration = "pub struct Newcomer;\ncrate::provider_object! { kind: Newcomer, parent: crate::provider::NODE_ID, requires: [\"cap.render\" => \"Provider\"], }\npub fn newly_consumes() { produce(1); }\n";
    let ordinary = format!("//{}\n", " ".repeat(declaration.len() - 3));
    tree.write_source(relative, &ordinary);
    let path = tree.0.join(relative);
    let modified = path.metadata().unwrap().modified().unwrap();
    let before = tree.ask(json!({"query":"root/provider/consumer"}));
    assert!(
        !before.contains("verify root/provider/newcomer"),
        "{before}"
    );

    // No mtime or size signal: the current bytes must determine membership.
    // 不提供时间戳或大小变化信号：是否成为消费者必须由当前字节决定。
    tree.write_source(relative, declaration);
    std::fs::File::options()
        .write(true)
        .open(&path)
        .unwrap()
        .set_times(std::fs::FileTimes::new().set_modified(modified))
        .unwrap();
    assert_eq!(ordinary.len(), path.metadata().unwrap().len() as usize);
    assert_eq!(modified, path.metadata().unwrap().modified().unwrap());
    let added = tree.ask(json!({"query":"root/provider/consumer"}));
    assert!(added.contains("verify root/provider/newcomer"), "{added}");
    assert!(added.contains("pub fn newly_consumes"), "{added}");

    let old = path.parent().unwrap();
    let moved = old.with_file_name("relocated");
    std::fs::rename(old, &moved).unwrap();
    std::fs::rename(moved.join("newcomer.rs"), moved.join("relocated.rs")).unwrap();
    let relocated = tree.ask(json!({"query":"root/provider/consumer"}));
    assert!(
        relocated.contains("verify root/provider/relocated"),
        "{relocated}"
    );
    assert!(
        !relocated.contains("verify root/provider/newcomer"),
        "{relocated}"
    );
    std::fs::remove_dir_all(moved).unwrap();
    let deleted = tree.ask(json!({"query":"root/provider/consumer"}));
    assert!(
        !deleted.contains("verify root/provider/relocated"),
        "{deleted}"
    );
    assert!(
        deleted.contains("verify root/provider/sibling"),
        "{deleted}"
    );

    // A previously unseen directory must also appear after the cache is warm.
    // 缓存热后，之前不存在的目录也必须被发现。
    tree.write_source(relative, declaration);
    let recreated = tree.ask(json!({"query":"root/provider/consumer"}));
    assert!(
        recreated.contains("verify root/provider/newcomer"),
        "{recreated}"
    );
}
