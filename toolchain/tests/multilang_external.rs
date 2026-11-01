#![cfg(feature = "cli")]
//! End-to-end: a project written in a language this checkout does not implement is a project like
//! any other, as long as a recorded backend owns it — `plan`, `build`, `test` and `package` all go
//! through the same `Project`/`Plan` shape, and the plan identity binds the input closure and the
//! backend program itself.
//! 端到端：本检出没有实现的语言，只要记录里的后端拥有它，就是一个普通项目——`plan`、`build`、
//! `test` 与 `package` 都走同一套 `Project`/`Plan` 形状，而计划身份绑住输入闭包与后端程序本身。

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn binary() -> &'static str {
    env!("CARGO_BIN_EXE_xirang")
}

fn scratch(label: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "xirang-external-{label}-{}-{:?}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&path).unwrap();
    path
}

fn invoke(directory: &Path, arguments: &[&str]) -> Output {
    Command::new(binary())
        .args(arguments)
        .current_dir(directory)
        .output()
        .unwrap()
}

fn both(output: &Output) -> String {
    format!(
        "status {}\nstdout:\n{}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

/// The `plan <id>` a build reports.
fn plan_id(output: &Output) -> String {
    let text = String::from_utf8_lossy(&output.stdout);
    let at = text.find("plan ").expect("no plan id in the summary");
    text[at + 5..]
        .chars()
        .take_while(|character| character.is_ascii_hexdigit())
        .collect()
}

const OPTIONS_PROJECT: &str = "schema = 1\n\n[package]\nname = \"widget\"\nversion = \"0.1.0\"\n\
kind = \"library\"\n\n[language.widget]\nverbosity = \"2\"\n\n[dependencies]\n";

const PLAIN_PROJECT: &str = "schema = 1\n\n[package]\nname = \"widget\"\nversion = \"0.1.0\"\n\
kind = \"library\"\n\n[language.widget]\n\n[dependencies]\n";

const RECORD: &str = "protocol = 1\n\n[[backend]]\nlanguage = \"widget\"\nbackend = \"widget-backend\"\n\
capabilities = [\"discovery\", \"build-test\"]\ncommand = [\"python3\", \"backend.py\"]\n";

const ANSWER: &str = r#"import json, os, sys
request = json.load(sys.stdin)
if request.get("request") == "handshake":
    print(json.dumps({"protocol": 2, "language": "widget", "backend": "widget-backend",
                      "capabilities": ["discovery", "build-test"],
                      "source_extensions": ["txt"], "test_suffix": "_test"}))
    sys.exit(0)
verbosity = request.get("options", {}).get("verbosity")
if verbosity not in (None, "2"):
    sys.exit("unexpected options: " + json.dumps(request.get("options")))
directory = request["directory"]
if request["action"] == "build":
    actions = [{"kind": "compile", "program": "sh",
                "args": ["-c", "mkdir -p target && printf built > target/widget.out"],
                "outputs": ["target/widget.out"]}]
else:
    actions = [{"kind": "test", "name": "sample", "program": "sh", "args": ["-c", "exit 0"]}]
print(json.dumps({"protocol": 2, "language": "widget", "backend": "widget-backend",
                  "capabilities": ["discovery", "build-test"],
                  "inputs": request["inputs"], "actions": actions}))
"#;

fn project(label: &str, manifest: &str, answer: &str) -> PathBuf {
    let root = scratch(label);
    std::fs::write(root.join("xirang.toml"), manifest).unwrap();
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::create_dir_all(root.join("tests")).unwrap();
    std::fs::write(root.join("src/sample.txt"), "answer\n").unwrap();
    std::fs::write(root.join("tests/sample_test.txt"), "check\n").unwrap();
    std::fs::write(root.join("xirang.backends.toml"), RECORD).unwrap();
    std::fs::write(root.join("backend.py"), answer).unwrap();
    root
}

#[test]
fn an_external_language_is_a_project_like_any_other() {
    let root = project("runs", PLAIN_PROJECT, ANSWER);
    let planned = invoke(&root, &["plan"]);
    assert!(planned.status.success(), "{}", both(&planned));
    let text = String::from_utf8_lossy(&planned.stdout);
    assert!(
        text.contains("backend widget-backend"),
        "the plan did not name its backend: {text}"
    );
    assert!(
        text.contains("action(s)"),
        "the plan did not list its actions: {text}"
    );
    assert!(
        !root.join("target").exists() || !root.join("target/widget.out").exists(),
        "the read-only plan wrote an artifact"
    );
    let built = invoke(&root, &["build"]);
    assert!(built.status.success(), "{}", both(&built));
    assert!(
        String::from_utf8_lossy(&built.stdout)
            .contains("through the recorded backend `widget-backend`"),
        "{}",
        both(&built)
    );
    assert!(
        root.join("target/widget.out").is_file(),
        "the action did not run"
    );
    let packaged = invoke(&root, &["package"]);
    assert!(packaged.status.success(), "{}", both(&packaged));
    let tested = invoke(&root, &["test"]);
    assert!(tested.status.success(), "{}", both(&tested));
    assert!(
        String::from_utf8_lossy(&tested.stdout).contains("test sample ... ok"),
        "{}",
        both(&tested)
    );
    let added = invoke(&root, &["add", "greeter", "--path", "../greeter"]);
    assert!(!added.status.success(), "{}", both(&added));
    assert!(
        both(&added).contains("owns the dependency graph"),
        "{}",
        both(&added)
    );
}

#[test]
fn an_external_plan_identity_binds_its_inputs_and_the_backend_program() {
    let root = project("identity", PLAIN_PROJECT, ANSWER);
    let first = invoke(&root, &["build"]);
    assert!(first.status.success(), "{}", both(&first));
    let id = plan_id(&first);
    let again = invoke(&root, &["build"]);
    assert!(again.status.success(), "{}", both(&again));
    assert_eq!(
        plan_id(&again),
        id,
        "an unchanged project got a new plan identity"
    );
    std::fs::write(root.join("src/sample.txt"), "a different answer\n").unwrap();
    let changed = invoke(&root, &["build"]);
    assert!(changed.status.success(), "{}", both(&changed));
    assert_ne!(
        plan_id(&changed),
        id,
        "a changed input kept the old plan identity"
    );
    std::fs::write(root.join("src/sample.txt"), "answer\n").unwrap();
    std::fs::write(
        root.join("backend.py"),
        format!("{ANSWER}\n# a new backend\n"),
    )
    .unwrap();
    let upgraded = invoke(&root, &["build"]);
    assert!(upgraded.status.success(), "{}", both(&upgraded));
    assert_ne!(
        plan_id(&upgraded),
        id,
        "a changed backend program kept the old plan identity"
    );
}

#[test]
fn options_declared_for_an_external_language_reach_its_backend() {
    let root = project("options", OPTIONS_PROJECT, ANSWER);
    let built = invoke(&root, &["build"]);
    assert!(built.status.success(), "{}", both(&built));
    let mismatched = OPTIONS_PROJECT.replace("verbosity = \"2\"", "verbosity = \"3\"");
    std::fs::write(root.join("xirang.toml"), mismatched).unwrap();
    let refused = invoke(&root, &["build"]);
    assert!(!refused.status.success(), "{}", both(&refused));
    assert!(
        both(&refused).contains("unexpected options"),
        "the options did not reach the backend: {}",
        both(&refused)
    );
}
