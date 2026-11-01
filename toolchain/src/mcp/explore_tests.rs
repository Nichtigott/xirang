//! Contract tests through the same dispatch used by CLI and MCP.
//! 从 CLI/MCP 共用分派入口验证读取契约。

use serde_json::{Value, json};
use std::path::PathBuf;

struct Tree(PathBuf);
impl Tree {
    fn new() -> Self {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let id = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!("xirang-explore-{}-{id}", std::process::id()));
        std::fs::create_dir_all(&path).unwrap();
        Self(path)
    }
    fn write_source(&self, path: &str, source: &str) {
        let path = self.0.join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, source).unwrap();
    }
    fn ask(&self, args: Value) -> String {
        crate::mcp::tools::run_tool(&self.0, "xirang.explore", &args).unwrap()
    }
}
impl Drop for Tree {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[path = "explore_context_tests.rs"]
mod context_tests;

#[path = "explore_structure_tests.rs"]
mod structure_tests;

#[test]
fn source_coverage_distinguishes_whole_files_and_selected_spans() {
    let tree = Tree::new();
    tree.write_source("src/whole.rs", "pub fn chosen() -> bool { true }\n");
    let whole = tree.ask(json!({"query":"src/whole.rs"}));
    assert!(
        whole.contains("source: whole file; 1 of 1 file lines shown"),
        "{whole}"
    );
    assert!(!whole.contains("read {path, whole:true}"), "{whole}");
    let mut long = String::from("pub fn chosen() -> bool { true }\n");
    long.push_str(&"// unrelated context\n".repeat(100));
    long.push_str("pub fn unrelated() {}\n");
    tree.write_source("src/whole.rs", &long);
    let partial = tree.ask(json!({"query":"chosen", "max_bytes":2000}));
    assert!(partial.contains("source: selected spans;"), "{partial}");
    assert!(!partial.contains("source: whole file;"), "{partial}");
    assert!(partial.len() <= 2000, "{}", partial.len());
}

#[test]
fn tests_through_a_direct_caller_arrive_without_recursing_into_production() {
    let tree = Tree::new();
    tree.write_source(
        "src/predicate.rs",
        "fn permitted(n: i32) -> bool { n != 0 }\n",
    );
    tree.write_source("src/store.rs", "fn post(n: i32) -> bool { permitted(n) }\n");
    tree.write_source("src/app.rs", "fn app(n: i32) -> bool { post(n) }\n");
    tree.write_source(
        "tests/ledger.rs",
        "#[test]\nfn rejects_zero() { assert!(!post(0)); }\n",
    );
    let reply = tree.ask(json!({"query":"permitted"}));
    assert!(reply.contains("assert!(!post(0))"), "{reply}");
    assert!(reply.contains("via post -> permitted"), "{reply}");
    assert!(
        !reply.contains("fn app"),
        "production expansion stays one hop: {reply}"
    );
}

#[test]
fn a_symptoms_incidental_symbol_does_not_hide_the_deciding_guard() {
    let tree = Tree::new();
    tree.write_source(
        "src/store.rs",
        "fn entries(&self) -> &[Entry] { &self.entries }\n",
    );
    tree.write_source("src/filter.rs", "/// Filter entries by their minimum amount.\nfn matches(entry: &Entry, min: i32) -> bool { entry.amount > min }\n");
    let reply = tree
        .ask(json!({"query":"the minimum-amount filter keeps the wrong entries", "max_files":1}));
    assert!(reply.contains("entry.amount > min"), "{reply}");
}

#[test]
fn qualified_method_queries_keep_ambiguous_implementations_explicit() {
    let tree = Tree::new();
    tree.write_source(
        "src/left.rs",
        "struct Left;\nimpl Left { fn verify(&self) -> bool { true } }\n",
    );
    tree.write_source(
        "src/right.rs",
        "struct Right;\nimpl Right { fn verify(&self) -> bool { false } }\n",
    );
    let reply = tree.ask(json!({"query":"Left::verify"}));
    assert!(
        reply.contains("fn verify(&self) -> bool { true }"),
        "{reply}"
    );
    assert!(
        reply.contains("fn verify(&self) -> bool { false }"),
        "{reply}"
    );
    assert!(
        reply.contains("module/type scope is not resolved"),
        "{reply}"
    );
}

#[test]
fn symptoms_return_producer_callees_and_test_assertions_together() {
    let tree = Tree::new();
    tree.write_source("src/lib.rs", "/// Reject negative quantities.\npub fn normalize(x: i32) -> i32 { clamp(x) }\nfn unrelated() {}\n");
    tree.write_source("src/math.rs", "fn clamp(x: i32) -> i32 { x.max(0) }\n");
    tree.write_source(
        "tests/edge.rs",
        "#[test]\nfn rejects() { assert_eq!(normalize(-1), 0); }\n",
    );
    let reply = tree.ask(json!({"query":"negative quantity"}));
    for expected in [
        "symptom candidate",
        "fn normalize",
        "fn clamp",
        "assert_eq!(normalize(-1), 0)",
    ] {
        assert!(reply.contains(expected), "{expected}: {reply}");
    }
    assert!(
        !reply.contains("fn unrelated"),
        "another function's docs are not its evidence: {reply}"
    );
}

#[test]
fn formatted_error_messages_find_the_producer_without_a_symbol_guess() {
    let tree = Tree::new();
    tree.write_source(
        "src/lib.rs",
        "fn enforce(n: u32) { panic!(\"flurb quota {} exceeded\", n); }\n",
    );
    let reply = tree.ask(json!({"query":"flurb quota exceeded"}));
    assert!(
        reply.contains("fn enforce") && reply.contains("body/message matches"),
        "{reply}"
    );
}

#[test]
fn a_type_name_returns_its_fields_and_implementation_context() {
    let tree = Tree::new();
    tree.write_source("src/model.rs", "pub struct Invoice { pub quantity: i32 }\nimpl Invoice { pub fn create(quantity: i32) -> Self { Self { quantity } } }\n");
    let reply = tree.ask(json!({"query":"Invoice"}));
    assert!(
        reply.contains("declaration src/model.rs:1 Invoice"),
        "{reply}"
    );
    assert!(
        reply.contains("pub quantity: i32") && reply.contains("pub fn create"),
        "{reply}"
    );
}

#[test]
fn an_exact_symbol_does_not_expand_unrelated_documentation_hits() {
    let tree = Tree::new();
    tree.write_source("src/target.rs", "fn normalize() {}\n");
    tree.write_source(
        "src/noise.rs",
        "/// normalize something else\nfn other() {}\n",
    );
    let reply = tree.ask(json!({"query":"normalize"}));
    assert!(reply.contains("fn normalize"), "{reply}");
    assert!(
        !reply.contains("file src/noise.rs"),
        "exact identity wins over prose: {reply}"
    );
}

#[test]
fn parsing_a_named_file_still_discovers_callers_of_its_unnamed_functions() {
    let tree = Tree::new();
    tree.write_source("src/entry.rs", "fn hidden_name() { dep(); }\n");
    tree.write_source("src/caller.rs", "fn caller() { hidden_name(); }\n");
    tree.write_source("src/dep.rs", "fn dep() {}\n");
    let reply = tree.ask(json!({"query":"src/entry.rs"}));
    assert!(
        reply.contains("fn caller") && reply.contains("fn dep"),
        "{reply}"
    );
}

#[test]
fn one_call_returns_contract_body_callers_callees_and_test_locations() {
    let tree = Tree::new();
    tree.write_source("src/lib.rs", "/// Preserve negative inputs.\npub fn normalize(x: i32) -> i32 { clamp(x) }\nfn clamp(x: i32) -> i32 { x.max(0) }\n");
    tree.write_source("src/client.rs", "pub fn run() { normalize(-1); }\n");
    tree.write_source(
        "tests/edge.rs",
        "#[test]\nfn negative() { assert_eq!(normalize(-1), 0); }\n",
    );
    let reply = tree.ask(json!({"query":"normalize"}));
    for expected in [
        "1 | /// Preserve negative inputs.",
        "2 | pub fn normalize",
        "3 | fn clamp",
        "src/client.rs:1 run -> normalize",
        "tests/edge.rs:2 negative -> normalize",
        "callee candidate",
        "test-file caller",
        "NOT resolved dispatch",
        "files omitted: 0",
    ] {
        assert!(reply.contains(expected), "missing {expected}: {reply}");
    }
    assert_eq!(reply.matches("2 | pub fn normalize").count(), 1);
}

#[test]
fn named_file_keeps_non_function_source_and_overlapping_ranges_are_merged() {
    let tree = Tree::new();
    tree.write_source("src/model.rs", "pub struct Model { pub value: i32 }\nimpl Model {\n    pub fn new() -> Self { Self { value: 7 } }\n}\n");
    let reply = tree.ask(json!({"query":"src/model.rs new"}));
    assert!(reply.contains("1 | pub struct Model"), "{reply}");
    assert_eq!(reply.matches("3 |     pub fn new").count(), 1, "{reply}");
    assert_eq!(reply.matches("file src/model.rs\n").count(), 1, "{reply}");
}

#[test]
fn named_targets_precede_neighbours_and_ambiguity_is_not_resolved() {
    let tree = Tree::new();
    tree.write_source("src/a.rs", "fn new() {}\n");
    tree.write_source("src/b.rs", "fn new() {}\n");
    tree.write_source("src/z.rs", "fn target() { new(); }\n");
    let short = tree.ask(json!({"query":"target", "max_files":1}));
    assert!(short.contains("1 | fn target()"), "{short}");
    assert!(short.contains("files omitted: 2 of 3"), "{short}");
    let full = tree.ask(json!({"query":"target"}));
    assert!(full.contains("new (2 definitions)"), "{full}");
    assert!(
        full.contains("file src/a.rs") && full.contains("file src/b.rs"),
        "{full}"
    );
}

#[test]
fn budgets_preserve_whole_utf8_lines_and_offer_a_continuation() {
    let tree = Tree::new();
    let long = "界".repeat(3000);
    tree.write_source(
        "src/z.rs",
        &format!("fn target() {{\n    let text = \"{long}\";\n    helper();\n}}\n"),
    );
    for i in 0..30 {
        tree.write_source(&format!("src/n{i}.rs"), "fn helper() {}\n");
    }
    let reply = tree.ask(json!({"query":"src/z.rs", "max_bytes":2000, "max_files":2}));
    assert!(reply.len() <= 2000, "{}: {reply}", reply.len());
    assert!(reply.contains("1 | fn target()"), "{reply}");
    assert!(!reply.contains("界"), "must not print partial source lines");
    assert!(reply.contains("first omitted line: 2"), "{reply}");
    assert!(reply.contains("read only missing context"), "{reply}");
}

#[test]
fn every_request_sees_edits_additions_and_deletions() {
    let tree = Tree::new();
    tree.write_source("src/lib.rs", "fn target() { old(); }\n");
    assert!(tree.ask(json!({"query":"target"})).contains("old();"));
    tree.write_source("src/lib.rs", "fn target() { fresh(); }\n");
    tree.write_source("src/new.rs", "fn fresh() {}\n");
    let reply = tree.ask(json!({"query":"target"}));
    assert!(
        reply.contains("fresh();") && reply.contains("file src/new.rs"),
        "{reply}"
    );
    assert!(!reply.contains("old();"));
    std::fs::remove_file(tree.0.join("src/new.rs")).unwrap();
    assert!(
        !tree
            .ask(json!({"query":"target"}))
            .contains("file src/new.rs")
    );
}

#[test]
fn no_manifest_needed_and_workspace_relative_paths_stay_unambiguous() {
    let tree = Tree::new();
    tree.write_source("a/src/lib.rs", "fn first() {}\n");
    tree.write_source("b/src/lib.rs", "fn second() { first(); }\n");
    let reply = tree.ask(json!({"query":"a/src/lib.rs"}));
    assert!(
        reply.contains("file a/src/lib.rs") && reply.contains("file b/src/lib.rs"),
        "{reply}"
    );
    assert!(
        tree.ask(json!({"query":"../src/lib.rs"}))
            .contains("no matches")
    );
    assert!(
        tree.ask(json!({"query":"unknown_symbol"}))
            .contains("no matches")
    );
}

#[test]
fn malformed_queries_and_bounds_are_refused() {
    let tree = Tree::new();
    for args in [
        json!({}),
        json!({"query":" "}),
        json!({"query":[]}),
        json!({"query":"x", "max_files":0}),
        json!({"query":"x", "max_files":13}),
        json!({"query":"x", "max_bytes":1999}),
        json!({"query":"x", "max_bytes":24001}),
        json!({"query":"x", "max_bytes":"2000"}),
        json!({"query":"x".repeat(1001)}),
    ] {
        assert!(crate::mcp::tools::run_tool(&tree.0, "xirang.explore", &args).is_err());
    }
}

#[cfg(unix)]
#[test]
fn outside_symlinks_never_contribute_source() {
    let inside = Tree::new();
    let outside = Tree::new();
    outside.write_source("secret.rs", "fn secret() {}\n");
    std::os::unix::fs::symlink(outside.0.join("secret.rs"), inside.0.join("secret.rs")).unwrap();
    assert!(
        inside
            .ask(json!({"query":"secret.rs secret"}))
            .contains("no matches")
    );
}

#[test]
fn an_early_large_neighbour_cannot_hide_the_named_function_in_the_same_file() {
    let tree = Tree::new();
    let filler = "    let _ = 42;\n".repeat(200);
    tree.write_source(
        "src/lib.rs",
        &format!("fn caller() {{\n{filler}    target();\n}}\nfn target() {{}}\n"),
    );
    let reply = tree.ask(json!({"query":"target", "max_bytes":2000}));
    assert!(reply.contains("204 | fn target() {}"), "{reply}");
    assert!(reply.len() <= 2000);
}

#[test]
fn a_small_budget_still_reserves_useful_source_instead_of_twelve_empty_fragments() {
    let tree = Tree::new();
    tree.write_source("src/target.rs", "fn target() {}\n");
    for i in 0..20 {
        tree.write_source(&format!("src/caller{i}.rs"), "fn caller() { target(); }\n");
    }
    let reply = tree.ask(json!({"query":"target", "max_files":12, "max_bytes":2000}));
    assert!(reply.contains("1 | fn target() {}"), "{reply}");
    assert!(reply.contains("withheld file src/caller"), "{reply}");
    assert!(reply.len() <= 2000, "{}", reply.len());
}

#[test]
fn unicode_evidence_and_omitted_file_pointers_share_the_total_budget() {
    let tree = Tree::new();
    for i in 0..20 {
        tree.write_source(&format!("src/m{i}.rs"), "fn target() {}\n");
    }
    let query = format!("{} target", "界".repeat(250));
    let reply = tree.ask(json!({"query":query, "max_bytes":2000}));
    assert!(reply.len() <= 2000, "{}: {reply}", reply.len());
    assert!(reply.contains("1 | fn target() {}"), "{reply}");
}

#[test]
fn equal_length_edits_with_restored_mtime_refresh_bodies_and_relations() {
    let tree = Tree::new();
    tree.write_source("src/lib.rs", "fn target() { old(); }\n");
    tree.write_source("src/deps.rs", "fn old() {}\nfn new() {}\n");
    let file = tree.0.join("src/lib.rs");
    let modified = std::fs::metadata(&file).unwrap().modified().unwrap();
    assert!(
        tree.ask(json!({"query":"target"}))
            .contains("target -> old")
    );
    tree.write_source("src/lib.rs", "fn target() { new(); }\n");
    std::fs::File::options()
        .write(true)
        .open(&file)
        .unwrap()
        .set_times(std::fs::FileTimes::new().set_modified(modified))
        .unwrap();
    let reply = tree.ask(json!({"query":"target"}));
    assert!(reply.contains("target -> new"), "{reply}");
    assert!(!reply.contains("target -> old"), "{reply}");
    assert!(reply.contains("fn target() { new(); }"), "{reply}");
}

#[test]
fn repeated_queries_deliver_source_again_and_root_switches_do_not_reuse_other_content() {
    let first = Tree::new();
    let second = Tree::new();
    first.write_source("src/lib.rs", "fn target() { first(); }\n");
    second.write_source("src/lib.rs", "fn target() { other(); }\n");
    let query = json!({"query":"target"});
    let answer = first.ask(query.clone());
    assert_eq!(answer, first.ask(query.clone()));
    let answer = second.ask(query.clone());
    assert!(answer.contains("other();") && !answer.contains("first();"));
    assert!(first.ask(query).contains("first();"));
}

#[test]
fn an_unmatched_query_skips_unrelated_syntax_but_sees_a_later_definition() {
    let tree = Tree::new();
    tree.write_source(
        "src/unrelated.rs",
        "fn unused_negative_probe() { helper(); }\n",
    );
    let query = json!({"query":"freshly_discovered_needle"});
    {
        let _snapshot = crate::source_cache::scope(&tree.0);
        let reply = tree.ask(query.clone());
        assert!(!reply.contains("fn unused_negative_probe"), "{reply}");
    }
    assert!(
        !tree
            .0
            .join("target/xirang/cache/syntax-facts.json")
            .exists()
    );
    tree.write_source("src/new.rs", "fn freshly_discovered_needle() {}\n");
    assert!(tree.ask(query).contains("fn freshly_discovered_needle()"));
}

#[test]
fn exploration_and_affected_share_long_chain_and_current_inventory() {
    let tree = Tree::new();
    tree.write_source("src/provider.rs", "fn provider() -> i32 { 7 }\n");
    for i in 0..8 {
        let call = if i == 0 {
            "provider".to_owned()
        } else {
            format!("layer{}", i - 1)
        };
        tree.write_source(
            &format!("src/layer{i}.rs"),
            &format!("fn layer{i}() -> i32 {{ {call}() }}\n"),
        );
    }
    tree.write_source(
        "tests/consumer.rs",
        "#[test]\nfn contract() { assert_eq!(layer7(), 7); }\n",
    );
    let before = tree.ask(json!({"query":"provider"}));
    assert!(
        before.contains("tests/consumer.rs") && before.contains("9 hops"),
        "{before}"
    );
    tree.write_source(
        "tests/new_consumer.rs",
        "#[test]\nfn another() { assert_eq!(layer7(), 7); }\n",
    );
    std::fs::rename(
        tree.0.join("tests/consumer.rs"),
        tree.0.join("tests/moved.rs"),
    )
    .unwrap();
    let after = tree.ask(json!({"query":"provider","max_files":8}));
    let affected =
        crate::mcp::affected::affected(&tree.0, &json!({"files":["src/provider.rs"]})).unwrap();
    for path in ["tests/new_consumer.rs", "tests/moved.rs"] {
        assert!(after.contains(path), "{after}");
        assert!(affected.contains(path), "{affected}");
    }
    assert!(!after.contains("tests/consumer.rs"));
}
