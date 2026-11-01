//! Scope and context-receipt counterexamples through real dispatch.
//! 经真实分派验证范围与上下文回执，重点守住省略不能伪造完整性的边界。

use super::{Tree, json};

fn receipts(reply: &str) -> Vec<String> {
    reply
        .lines()
        .filter_map(|line| line.strip_prefix("context receipt: "))
        .map(str::to_owned)
        .collect()
}

fn long_source() -> String {
    format!(
        "pub fn target() -> usize {{\n{}    73\n}}\n",
        "    // Evidence whose original bytes the caller explicitly retains.\n".repeat(24)
    )
}

#[test]
fn explicit_receipts_reduce_repetition_but_defaults_always_return_source() {
    let tree = Tree::new();
    tree.write_source("src/lib.rs", &long_source());
    let original = tree.ask(json!({"query":"target", "known_context":[]}));
    let known = receipts(&original);
    assert_eq!(known.len(), 1);
    let reduced = tree.ask(json!({"query":"target", "known_context":known}));
    assert!(reduced.contains("context unchanged:"), "{reduced}");
    assert!(!reduced.contains("pub fn target"));
    assert!(
        reduced.len() * 2 < original.len(),
        "{} / {}",
        reduced.len(),
        original.len()
    );
    assert!(
        tree.ask(json!({"query":"target"}))
            .contains("pub fn target")
    );
    assert!(
        tree.ask(json!({"query":"target", "known_context":[]}))
            .contains("pub fn target")
    );
}

#[test]
fn same_length_same_mtime_edits_and_new_callers_invalidate_old_evidence() {
    let tree = Tree::new();
    tree.write_source("src/lib.rs", &long_source());
    let initial = tree.ask(json!({"query":"target", "known_context":[]}));
    let known = receipts(&initial);
    tree.write_source(
        "tests/new.rs",
        "#[test]\nfn new_caller() { assert_eq!(target(), 73); }\n",
    );
    let caller_added = tree.ask(json!({"query":"target", "known_context":known}));
    assert!(caller_added.contains("fn new_caller"), "{caller_added}");
    let path = tree.0.join("src/lib.rs");
    let modified = path.metadata().unwrap().modified().unwrap();
    tree.write_source("src/lib.rs", &long_source().replace("73", "74"));
    std::fs::File::options()
        .write(true)
        .open(path)
        .unwrap()
        .set_times(std::fs::FileTimes::new().set_modified(modified))
        .unwrap();
    tree.write_source(
        "tests/new.rs",
        "#[test]\nfn new_caller() { assert_eq!(target(), 74); }\n",
    );
    let changed = tree.ask(json!({"query":"target", "known_context":known}));
    assert!(
        changed.contains("74") && changed.contains("fn new_caller"),
        "{changed}"
    );
    assert!(!changed.contains("context unchanged:"), "{changed}");
}

#[test]
fn a_root_or_rendering_change_cannot_reuse_a_partial_receipt() {
    let first = Tree::new();
    let second = Tree::new();
    for tree in [&first, &second] {
        tree.write_source("src/lib.rs", &long_source());
    }
    let small = first.ask(json!({"query":"target", "max_bytes":2000, "known_context":[]}));
    let known = receipts(&small);
    assert_eq!(known.len(), 1);
    let larger = first.ask(json!({"query":"target", "known_context":known}));
    assert!(larger.contains("pub fn target") && !larger.contains("context unchanged:"));
    let other = second.ask(json!({"query":"target", "known_context":receipts(&larger)}));
    assert!(other.contains("pub fn target") && !other.contains("context unchanged:"));
}

#[test]
fn scoped_queries_read_only_selected_sources_and_declare_unexamined_callers() {
    let tree = Tree::new();
    tree.write_source("src/part/a.rs", "fn target() {}\n");
    tree.write_source("tests/external.rs", "#[test]\nfn outside() { target(); }\n");
    let scoped = tree.ask(json!({"query":"target", "scope":["src/part", "src/part/a.rs"]}));
    assert!(scoped.contains("1 Rust files scanned"), "{scoped}");
    assert!(scoped.contains("outside-scope definitions/callers/tests were not examined"));
    assert!(scoped.contains("src/part/a.rs") && !scoped.contains("fn outside"));
    assert!(tree.ask(json!({"query":"target"})).contains("fn outside"));
    // Unreadable text outside the selected scope cannot force a tree-wide read.
    // 范围外的不可读文本不能触发整树读取。
    std::fs::write(tree.0.join("src/invalid.rs"), [255]).unwrap();
    assert!(
        tree.ask(json!({"query":"target", "scope":["src/part"]}))
            .contains("fn target")
    );
}

#[test]
fn deleted_sources_and_invalid_scope_or_receipts_are_not_silently_reused() {
    let tree = Tree::new();
    tree.write_source("src/lib.rs", &long_source());
    let known = receipts(&tree.ask(json!({"query":"target", "known_context":[]})));
    std::fs::remove_file(tree.0.join("src/lib.rs")).unwrap();
    let gone = tree.ask(json!({"query":"target", "known_context":known}));
    assert!(gone.contains("no matches") && !gone.contains("context unchanged:"));
    for args in [
        json!({"query":"target", "scope":["../"]}),
        json!({"query":"target", "scope":["target"]}),
        json!({"query":"target", "scope":["missing.rs"]}),
        json!({"query":"target", "known_context":["pretend"]}),
        json!({"query":"target", "known_context":["A".repeat(64)]}),
    ] {
        assert!(crate::mcp::tools::run_tool(&tree.0, "xirang.explore", &args).is_err());
    }
}

#[cfg(unix)]
#[test]
fn scoped_symlinks_cannot_read_another_project() {
    let tree = Tree::new();
    let outside = Tree::new();
    outside.write_source("secret.rs", "fn target() {}\n");
    std::os::unix::fs::symlink(&outside.0, tree.0.join("linked")).unwrap();
    let result = crate::mcp::tools::run_tool(
        &tree.0,
        "xirang.explore",
        &json!({"query":"target", "scope":["linked"]}),
    );
    assert!(result.is_err());
}
