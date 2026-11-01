use serde_json::json;
use std::path::{Path, PathBuf};

fn scratch(label: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "xirang-mcp-multilang-{label}-{}-{:?}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&path).unwrap();
    path
}

fn project(label: &str) -> PathBuf {
    let root = scratch(label);
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::write(
        root.join("xirang.toml"),
        "schema = 1\n\n[package]\nname = \"widget\"\nversion = \"0.1.0\"\nkind = \"library\"\n\n[language.cpp]\nstandard = \"17\"\n",
    )
    .unwrap();
    std::fs::write(root.join("src/widget.cpp"), "int widget() { return 1; }\n").unwrap();
    root
}

fn call(root: &Path, arguments: serde_json::Value) -> Result<String, String> {
    crate::mcp::tools::run_tool(root, "xirang.multilang", &arguments)
}

#[test]
fn the_inventory_lists_the_units_and_writes_nothing() {
    let root = project("inventory");
    let text = call(&root, json!({"action": "inventory"})).expect("inventory answers");
    assert!(text.contains("1 unit(s)"), "{text}");
    assert!(text.contains("widget"), "{text}");
    assert!(text.contains("cpp"), "{text}");
    assert!(text.contains("library"), "{text}");
    assert!(text.contains("1 source file(s), 0 test(s)"), "{text}");
    assert!(
        !root.join("target").exists(),
        "the inventory must not write a target directory"
    );
}

#[test]
fn the_plan_shows_the_actions_without_running_or_writing() {
    let root = project("plan");
    let text = call(&root, json!({"action": "plan"})).expect("plan answers");
    assert!(text.contains("plan "), "{text}");
    assert!(text.contains("Compile"), "{text}");
    assert!(text.contains("src/widget.cpp"), "{text}");
    assert!(text.contains("nothing was executed"), "{text}");
    assert!(
        !root.join("target").exists(),
        "describing a plan must not write the fact cache or a plan directory"
    );
}

#[test]
fn an_action_this_tool_does_not_have_is_refused_by_name() {
    let root = project("refusal");
    let error = call(&root, json!({"action": "build"})).expect_err("build is not an action here");
    assert!(error.contains("`build` is not an action"), "{error}");
    assert!(
        error.contains("inventory") && error.contains("plan") && error.contains("report"),
        "{error}"
    );
    assert!(
        !root.join("target").exists(),
        "a refused action writes nothing"
    );
}

#[test]
fn a_directory_without_a_manifest_is_refused_by_name() {
    let root = scratch("no-manifest");
    let error = call(&root, json!({})).expect_err("no manifest means no project");
    assert!(error.contains("xirang.toml"), "{error}");
}

#[test]
fn the_report_action_reads_evidence_and_never_builds() {
    let root = project("report");
    let error = call(&root, json!({"action": "report"}))
        .expect_err("an unbuilt project has no evidence to report");
    assert!(error.contains("build"), "{error}");
    assert!(
        !root.join("target").exists(),
        "reporting must not build or write the fact cache"
    );
}

#[test]
fn the_report_action_names_a_plan_that_was_not_built() {
    let root = project("report-plan");
    let error = call(
        &root,
        json!({"action": "report", "plan": "0123456789abcdef"}),
    )
    .expect_err("no such plan was built");
    assert!(error.contains("0123456789abcdef"), "{error}");
    assert!(error.contains("no build evidence"), "{error}");
}
