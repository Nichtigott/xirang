//! Pins for `xirang.affected`: what a changed file reaches, and what it says when nothing
//! reaches it.
//! `xirang.affected` 的钉子：一个改动过的文件能触到什么，以及什么都没有时它说什么。

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use serde_json::json;
use xirang_kernel::identity::NodeId;

use super::affected;

/// A throwaway single package: one source file with a definition, and one test file calling it.
/// 一个一次性单包：一个带定义的文件，和一个调用它的测试文件。
fn package(label: &str) -> PathBuf {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "xirang-mcp-affected-{label}-{}-{sequence}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    write_fixture(
        &root.join("Cargo.toml"),
        "[package]\nname = \"affected-fixture\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    write_fixture(&root.join("src/lib.rs"), "// entry\n");
    write_fixture(&root.join("src/shared.rs"), "pub fn shared_helper() {}\n");
    write_fixture(
        &root.join("tests/uses_shared.rs"),
        "#[test]\nfn it_calls_shared() {\n    shared_helper();\n}\n",
    );
    root
}

#[test]
fn a_file_with_an_unresolved_call_says_so() {
    let root = package("unknown");
    write_fixture(
        &root.join("src/dynamic.rs"),
        "pub fn caller(value: &dyn crate::Trait) { value.run(); }\n",
    );
    let text = affected(&root, &json!({"files": ["src/dynamic.rs"]})).expect("an answer");
    assert!(
        text.contains("unresolved: 1 call site(s) kept as unknown"),
        "{text}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

fn write_fixture(path: &Path, text: &str) {
    std::fs::create_dir_all(path.parent().expect("fixture parent")).expect("fixture dirs");
    std::fs::write(path, text).expect("fixture file");
}

#[test]
fn a_changed_file_names_the_tests_that_call_its_definitions() {
    let root = package("named");
    let text = affected(&root, &json!({"files": ["src/shared.rs"]})).expect("an answer");
    assert!(text.contains("src/shared.rs: 1 definition(s)"), "{text}");
    assert!(
        text.contains("tests/uses_shared.rs"),
        "the test that calls the definition is the answer: {text}"
    );
}

#[test]
fn a_changed_file_no_test_reaches_says_so() {
    let root = package("unreached");
    write_fixture(&root.join("src/quiet.rs"), "pub fn quiet_helper() {}\n");
    let text = affected(&root, &json!({"files": ["src/quiet.rs"]})).expect("an answer");
    assert!(
        text.contains("none reference these definitions"),
        "an empty list would read as `nothing to run` rather than `run the suite`: {text}"
    );
}

#[test]
fn a_path_outside_the_index_is_named_rather_than_dropped() {
    let root = package("missing");
    let text = affected(&root, &json!({"files": ["src/nowhere.rs"]})).expect("an answer");
    assert!(text.contains("not in the index"), "{text}");
}

#[test]
fn an_empty_file_list_is_a_usage_error() {
    let root = package("empty");
    let error = affected(&root, &json!({"files": []})).expect_err("no paths is a usage error");
    assert!(error.contains("at least one path"), "{error}");
}

/// Known defect T-27, pinned as it behaves today: a file whose only definition is `Store::new`
/// "affects" a test that calls `Entry::new`, because the index records the bare `new` on both
/// sides and `is_call_to` cannot tell the two owners apart.
/// 已登记缺陷 T-27，按今天的样子钉住：只定义了 `Store::new` 的文件会"影响"一个只调用 `Entry::new`
/// 的测试，因为索引两侧记的都是裸名 `new`，`is_call_to` 分不出两个属主。
#[test]
fn a_same_named_method_on_another_type_is_not_reported_as_affected() {
    let root = package("t27");
    write_fixture(
        &root.join("src/entry.rs"),
        "pub struct Entry;\nimpl Entry {\n    pub fn new() -> Entry { Entry }\n}\n",
    );
    write_fixture(
        &root.join("src/store.rs"),
        "pub struct Store;\nimpl Store {\n    pub fn new() -> Store { Store }\n}\n",
    );
    write_fixture(
        &root.join("tests/uses_entry.rs"),
        "#[test]\nfn it_builds_an_entry() {\n    let _ = Entry::new();\n}\n",
    );
    let text = affected(&root, &json!({"files": ["src/store.rs"]})).expect("an answer");
    // The old false positive is gone: nothing in `tests/uses_entry.rs` touches `Store`, and the
    // semantic index now carries the owning type into the match.
    // 旧的误报消失：`tests/uses_entry.rs` 里没有任何东西碰 `Store`，而语义索引现在把属主类型带进匹配。
    assert!(
        !text.contains("tests/uses_entry.rs"),
        "T-27 false positive: a change to Store::new listed Entry::new's test: {text}"
    );
    assert!(
        !text.contains("uses_shared"),
        "the unrelated test is not listed, so the answer is not just every test: {text}"
    );
    // The scope line still admits that unresolved names exist; what changed is that this pair is
    // now resolved rather than matched by bare suffix.
    // 作用域行仍承认存在未解析名字；改变的是这一对现在按语义解析，而不是裸后缀匹配。
    assert!(text.contains("unresolved"), "{text}");
    // The true positive still works, so the pin is about the extra row, not the whole list.
    // 真阳性仍然成立，因此这枚钉子钉的是多出来的那一行，而不是整张清单。
    let entry = affected(&root, &json!({"files": ["src/entry.rs"]})).expect("an answer");
    assert!(entry.contains("tests/uses_entry.rs"), "{entry}");
}

/// One path, a comma list, and a JSON array are the same request.
/// 一个路径、逗号列表、JSON 数组是同一个请求。
///
/// The round measured the comma list failing: `--files "a.rs,b.rs"` reached the tool as **one** path
/// named `a.rs,b.rs` and answered `not in the index`, while the same two paths in a JSON array
/// worked. Nothing about the argument's type says "one path" — its spelling does.
/// 那一轮量到逗号列表失败：`--files "a.rs,b.rs"` 以**一个**名为 `a.rs,b.rs` 的路径到达并答
/// `not in the index`，而同样两个路径写成 JSON 数组就成功。参数的**类型**并没有说"这是一个路径"——
/// 是它的**拼法**在说。
#[test]
fn the_three_spellings_of_several_files_agree() {
    let root = package("spellings");
    let one = affected(&root, &json!({"files": ["src/shared.rs"]})).expect("an answer");
    let comma = affected(&root, &json!({"files": "src/shared.rs"})).expect("an answer");
    assert_eq!(
        one, comma,
        "a one-element string is the same request as a one-element array"
    );
    // And a real comma list is read as two paths, not one.
    // 而真正的逗号列表被读成两个路径，不是一个。
    write_fixture(&root.join("src/quiet.rs"), "pub fn quiet_helper() {}\n");
    let two = affected(&root, &json!({"files": "src/shared.rs, src/quiet.rs"})).expect("an answer");
    assert!(
        !two.contains("not in the index"),
        "neither path may be reported as unknown: {two}"
    );
    assert!(
        two.contains("src/shared.rs: 1 definition(s)")
            && two.contains("src/quiet.rs: 1 definition(s)"),
        "both paths are answered: {two}"
    );
}

/// A package whose changed file **is** a registration face, plus one graft plan naming it.
/// 一个"改动文件本身就是注册面"的包，外加一条点名它的 graft 计划。
///
/// `plan_layer` answers from records that are not Rust code, so the fixture has to have both halves:
/// a face the derivation can find (the plan's target is an identity) and a plan on disk under
/// `.xirang/external-grafts`.
/// `plan_layer` 依据的是"不是 Rust 代码"的那些记录，因此夹具两半都要有：推导找得到的注册面（计划的目标
/// 是一个身份）与 `.xirang/external-grafts` 下的一份计划。
fn package_with_plan(label: &str) -> PathBuf {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "xirang-mcp-affected-{label}-{}-{sequence}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    write_fixture(
        &root.join("Cargo.toml"),
        "[package]\nname = \"affected-plan-fixture\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    write_fixture(&root.join("src/lib.rs"), "// entry\n");
    // The face lives at `<dir>/<name>.rs`, which is the shape the build's walk recognises — the
    // first version of this fixture put it at `src/shared.rs` and the derivation found **no faces at
    // all**, so the plan layer had nothing to match and said "none of the 1 entry(ies) names this
    // face". A fixture that is wrong about layout makes a correct tool look broken.
    // 面住在 `<dir>/<name>.rs`——构建遍历认得的那种形状；这个夹具的第一版把它放在 `src/shared.rs`，于是推导
    // **一个面都没找到**，计划层无从匹配、只能报"没有条目点名这个面"。一个布局写错的夹具，会让正确的工具
    // 看起来是坏的。
    write_fixture(
        &root.join("src/shared/shared.rs"),
        "pub struct Shared;\n\npub fn shared_helper() {}\n\ncrate::root_object! {\n    kind: \
         Shared,\n    parent: crate::root_node_id(env!(\"CARGO_PKG_NAME\")),\n}\n",
    );
    write_fixture(
        &root.join("tests/uses_shared.rs"),
        "#[test]\nfn it_calls_shared() {\n    shared_helper();\n}\n",
    );
    // The plan's target is the **identity** the build computes for this face, which is why the
    // fixture builds it with the same kernel function the derivation uses.
    // 计划的目标是构建为这个面算出的**身份**，因此夹具用推导自己用的那个内核函数把它造出来。
    // The identity path is the face file's **own relative path** (`shared/shared.rs`), not its leaf
    // name: writing `shared.rs` here was this fixture's second mistake, and the plan then targeted an
    // identity no face has.
    // 身份路径是面文件**自己的相对路径**（`shared/shared.rs`），不是它的叶名：这里写 `shared.rs` 是这个
    // 夹具的第二个错，于是计划针对了一个没有任何面拥有的身份。
    let target =
        NodeId::from_namespaced_path("affected-plan-fixture", "shared/shared.rs", "Shared");
    write_fixture(
        &root.join(".xirang/external-grafts/swapped/graft.plan"),
        &format!(
            "version = 1\ntarget = {target}\ntarget_path = root/shared\ngraft = swapped\nfull = \
             false\n"
        ),
    );
    root
}

/// A changed file's reach names the graft plan entries that touch its face (audit `W5-2`).
/// 一个改动文件的波及面会点名触到它那个面的 graft 计划条目（审计 `W5-2`）。
#[test]
fn a_changed_face_names_the_plan_entries_that_target_it() {
    let root = package_with_plan("plan");
    let text = affected(&root, &json!({"files": ["src/shared/shared.rs"]})).expect("an answer");
    assert!(
        text.contains("plan: 1 entry(ies) name this face"),
        "the plan layer names the entry: {text}"
    );
    assert!(
        text.contains("swapped targets root/shared"),
        "and it says which selector targets which logical path: {text}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// A tree with plans but none for this face says so, rather than staying silent.
/// 有计划、但没有一条针对这个面的树会把这件事说出来，而不是保持沉默。
#[test]
fn a_face_no_plan_targets_is_reported_as_such() {
    let root = package_with_plan("plan-miss");
    let text = affected(&root, &json!({"files": ["src/lib.rs"]})).expect("an answer");
    assert!(
        text.contains("plan: none of the 1 entry(ies) here names this face"),
        "silence would read as 'no plans exist': {text}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// A tree with no plan directory grows no plan line at all.
/// 没有计划目录的树完全不会多出一行计划。
#[test]
fn a_tree_without_plans_pays_nothing() {
    let root = package("no-plans");
    let text = affected(&root, &json!({"files": ["src/shared.rs"]})).expect("an answer");
    assert!(
        !text.contains("plan:"),
        "a tree that has never had a graft says nothing about plans: {text}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// Long consumer chains and cycles must not hide downstream regression obligations.
/// 长消费者链与环不能隐藏下游回归义务。
#[test]
fn transitive_workspace_consumers_cycles_and_limits_are_reported() {
    let root = package("long-chain");
    write_fixture(
        &root.join("Cargo.toml"),
        "[workspace]\nmembers=['provider','consumer']\nresolver='2'\n",
    );
    for name in ["provider", "consumer"] {
        write_fixture(
            &root.join(name).join("Cargo.toml"),
            &format!("[package]\nname='{name}'\nversion='0.0.0'\nedition='2024'\n"),
        );
    }
    write_fixture(&root.join("provider/src/lib.rs"), "pub fn rate() {}\n");
    let mut chain = String::new();
    for step in 0..8 {
        chain.push_str(&format!("fn step{step}() {{ step{}(); }}\n", step + 1));
    }
    chain.push_str("fn step8() { provider::rate(); step0(); }\n");
    write_fixture(&root.join("consumer/src/lib.rs"), &chain);
    for name in ["first", "second"] {
        write_fixture(
            &root.join(format!("consumer/tests/{name}.rs")),
            &format!("#[test]\nfn {name}() {{ step0(); }}\n"),
        );
    }
    let reply = affected(&root, &json!({"files":["provider/src/lib.rs"], "limit":1})).unwrap();
    assert!(reply.contains("consumer/tests/first.rs"), "{reply}");
    assert!(reply.contains("withheld"), "{reply}");
    let all = affected(
        &root,
        &json!({"files":["provider/src/lib.rs"], "limit":200}),
    )
    .unwrap();
    assert!(all.contains("consumer/tests/second.rs"), "{all}");
    assert!(
        !all.contains("tests/uses_shared.rs"),
        "unrelated tests: {all}"
    );
    assert!(all.contains("not coverage"), "{all}");
    // A fresh inventory discovers a newly added consumer without an explicit rebuild.
    // 新清单无需显式重建便能发现新增消费者。
    write_fixture(
        &root.join("consumer/tests/new.rs"),
        "#[test]\nfn added() { step3(); }\n",
    );
    let changed = affected(&root, &json!({"files":["provider/src/lib.rs"]})).unwrap();
    assert!(changed.contains("consumer/tests/new.rs"), "{changed}");
    std::fs::remove_file(root.join("consumer/tests/new.rs")).unwrap();
    let deleted = affected(&root, &json!({"files":["provider/src/lib.rs"]})).unwrap();
    assert!(!deleted.contains("consumer/tests/new.rs"), "{deleted}");
}

#[test]
fn changed_tests_and_invalid_path_lists_do_not_disappear() {
    let root = package("test-input");
    let reply = affected(&root, &json!({"files":["tests/uses_shared.rs"]})).unwrap();
    assert!(reply.contains("tests: tests/uses_shared.rs"), "{reply}");
    for request in [
        json!({"files":["src/shared.rs",42]}),
        json!({"files":["src/shared.rs"],"limit":0}),
    ] {
        assert!(affected(&root, &request).is_err());
    }
}
