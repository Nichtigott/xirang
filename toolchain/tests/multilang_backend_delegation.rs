#![cfg(feature = "cli")]
//! End-to-end: a recorded backend with a `command` answers with a plan, and `xirang build` /
//! `xirang test` really run that plan and record which backend answered.
//! 端到端：带 `command` 的记录后端给出一份计划，`xirang build` / `xirang test` 真的运行它，并记录
//! 是哪个后端回答的。
//!
//! The backend here is a small script, not a compiler: what is under test is the delegation
//! boundary — the request, the checked answer, the execution and the evidence — and every refusal
//! gets its own run so a passing test never stands for a check that did not happen.
//! 这里后端是一小段脚本而不是编译器：被测的是委派边界——请求、检查过的应答、执行与证据——而每一种
//! 拒绝都有自己的运行，避免一次通过替一次没发生的检查背书。

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// The binary under test.
/// 被测二进制。
fn binary() -> &'static str {
    env!("CARGO_BIN_EXE_xirang")
}

/// A fresh scratch directory, so two runs never share a plan or an evidence file.
/// 一个全新的临时目录，使两次运行不共享计划或证据文件。
fn scratch(label: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "xirang-delegate-{label}-{}-{:?}",
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

/// Write a project whose language is `perl` (no built-in backend), its record, and one script.
/// 写一个语言为 `perl`（没有内置后端）的项目、它的记录与一段脚本。
fn project(label: &str, answer: &str) -> PathBuf {
    let root = scratch(label);
    std::fs::write(
        root.join("xirang.toml"),
        "schema = 1\n\n[package]\nname = \"widget\"\nversion = \"0.1.0\"\nkind = \"library\"\n\n\
         [language.perl]\nstandard = \"3\"\n\n[dependencies]\n",
    )
    .unwrap();
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::create_dir_all(root.join("tests")).unwrap();
    std::fs::write(root.join("src/lib.rb"), "def answer\n  42\nend\n").unwrap();
    std::fs::write(
        root.join("tests/lib_test.rb"),
        "raise unless answer == 42\n",
    )
    .unwrap();
    std::fs::write(
        root.join("xirang.backends.toml"),
        "protocol = 1\n\n[[backend]]\nlanguage = \"perl\"\nbackend = \"fixture-perl\"\n\
         capabilities = [\"discovery\", \"build-test\"]\ncommand = [\"python3\", \"backend.py\"]\n",
    )
    .unwrap();
    std::fs::write(root.join("backend.py"), answer).unwrap();
    root
}

/// A backend that answers a plan for either action.
/// 为两种动作都给出计划的后端。
const ANSWER: &str = r#"import json, sys
request = json.load(sys.stdin)
if request.get("request") == "handshake":
    print(json.dumps({"protocol": 2, "source_extensions": ["rb"], "test_suffix": "_test", "language": "perl", "backend": "fixture-perl",
                      "capabilities": ["discovery", "build-test"]}))
    sys.exit(0)
if request["action"] == "test":
    actions = [{"kind": "test", "program": sys.executable, "args": ["-c", "print('delegated-ok')"], "name": "lib_test"}]
else:
    actions = [{"kind": "compile", "program": sys.executable, "args": ["-c", "print('delegated-build')"]}]
print(json.dumps({"protocol": 2, "inputs": request["inputs"], "language": "perl", "backend": "fixture-perl",
                  "capabilities": ["build-test"], "actions": actions}))
"#;

/// The evidence document of the newest build directory.
/// 最新构建目录里的证据文档。
fn evidence(root: &Path) -> serde_json::Value {
    let builds = root.join("target/xirang/build");
    let mut newest: Option<(std::time::SystemTime, PathBuf)> = None;
    for entry in std::fs::read_dir(&builds).unwrap() {
        let path = entry.unwrap().path().join("evidence.json");
        let Ok(metadata) = std::fs::metadata(&path) else {
            continue;
        };
        let modified = metadata.modified().unwrap();
        if newest.as_ref().is_none_or(|(at, _)| modified > *at) {
            newest = Some((modified, path));
        }
    }
    let (_, path) = newest.expect("no evidence.json was written");
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

#[test]
fn a_recorded_backend_with_a_command_builds_and_tests_through_the_delegation_boundary() {
    let root = project("runs", ANSWER);
    let built = invoke(&root, &["build"]);
    assert!(built.status.success(), "{}", both(&built));
    let text = String::from_utf8_lossy(&built.stdout);
    assert!(
        text.contains("through the recorded backend `fixture-perl`"),
        "the summary does not name the backend that answered: {text}"
    );
    let listed = invoke(&root, &["languages"]);
    assert!(listed.status.success(), "{}", both(&listed));
    let report = String::from_utf8_lossy(&listed.stdout);
    assert!(
        report.contains("perl  fixture-perl  discovery, build-test  (executable)"),
        "the report did not mark the recorded backend executable: {report}"
    );
    let document = evidence(&root);
    assert_eq!(document["result"], "passed");
    assert_eq!(document["backend"]["name"], "fixture-perl");
    assert_eq!(document["backend"]["language"], "perl");
    assert!(
        document.get("compiler").is_none(),
        "a delegated plan was recorded as if a built-in compiler produced it: {document}"
    );
    let locked = invoke(&root, &["build", "--locked"]);
    assert!(locked.status.success(), "{}", both(&locked));

    let tested = invoke(&root, &["test"]);
    assert!(tested.status.success(), "{}", both(&tested));
    let text = String::from_utf8_lossy(&tested.stdout);
    assert!(text.contains("test lib_test ... ok"), "{text}");
    assert_eq!(evidence(&root)["result"], "passed");
}

#[test]
fn a_failing_delegated_test_is_reported_as_a_failed_test() {
    let answer = r#"import json, sys
request = json.load(sys.stdin)
print(json.dumps({"protocol": 2, "inputs": request["inputs"], "language": "perl", "backend": "fixture-perl",
                  "capabilities": ["build-test"],
                  "actions": [{"kind": "test", "program": sys.executable,
                               "args": ["-c", "import sys; sys.exit(1)"], "name": "lib_test"}]}))
"#;
    let root = project("fails", answer);
    let tested = invoke(&root, &["test"]);
    assert!(!tested.status.success(), "{}", both(&tested));
    let text = both(&tested);
    assert!(text.contains("test lib_test ... FAILED"), "{text}");
    assert!(text.contains("test `lib_test` failed"), "{text}");
    assert_eq!(evidence(&root)["result"], "failed");
}

#[test]
fn a_declared_backend_without_a_command_is_not_an_executor() {
    let root = project("declared", ANSWER);
    std::fs::write(
        root.join("xirang.backends.toml"),
        "protocol = 1\n\n[[backend]]\nlanguage = \"perl\"\nbackend = \"fixture-perl\"\n\
         capabilities = [\"discovery\", \"build-test\"]\n",
    )
    .unwrap();
    let built = invoke(&root, &["build"]);
    assert!(!built.status.success(), "{}", both(&built));
    let text = both(&built);
    assert!(text.contains("names no `command`"), "{text}");
    assert!(text.contains("declaration is not an executor"), "{text}");
    assert!(
        !root.join("target/xirang/build").exists(),
        "a refused delegation still wrote a build directory"
    );
}

#[test]
fn an_answer_with_another_protocol_or_language_is_refused() {
    let wrong_protocol = r#"import json, sys
json.load(sys.stdin)
print(json.dumps({"protocol": 2, "language": "perl", "backend": "fixture-perl",
                  "capabilities": ["build-test"],
                  "actions": [{"kind": "compile", "program": "python3", "args": ["-c", "pass"]}]}))
"#;
    let root = project("protocol", wrong_protocol);
    let refused = invoke(&root, &["build"]);
    assert!(!refused.status.success(), "{}", both(&refused));
    assert!(both(&refused).contains("protocol 2"), "{}", both(&refused));

    let wrong_language = r#"import json, sys
request = json.load(sys.stdin)
print(json.dumps({"protocol": 2, "inputs": request["inputs"], "language": "python", "backend": "fixture-perl",
                  "capabilities": ["build-test"],
                  "actions": [{"kind": "compile", "program": "python3", "args": ["-c", "pass"]}]}))
"#;
    let root = project("language", wrong_language);
    let refused = invoke(&root, &["build"]);
    assert!(!refused.status.success(), "{}", both(&refused));
    assert!(
        both(&refused).contains("answered for `python`"),
        "{}",
        both(&refused)
    );
}

#[test]
fn an_answer_that_claims_no_build_test_is_refused_by_name() {
    let answer = r#"import json, sys
request = json.load(sys.stdin)
print(json.dumps({"protocol": 2, "inputs": request["inputs"], "language": "perl", "backend": "fixture-perl",
                  "capabilities": ["discovery"],
                  "actions": [{"kind": "compile", "program": "python3", "args": ["-c", "pass"]}]}))
"#;
    let root = project("capability", answer);
    let refused = invoke(&root, &["build"]);
    assert!(!refused.status.success(), "{}", both(&refused));
    let text = both(&refused);
    assert!(text.contains("does not claim `build-test`"), "{text}");
}

/// A backend is asked for its handshake, and the report prints what it answered against the
/// record: matching, fewer capabilities, or another name are all facts, not silence.
/// 后端会被问一次握手，报告把它答的与记录印在一起：一致、能力更少或换了名字，都是事实而不是沉默。
#[test]
fn the_languages_report_asks_the_recorded_backend_and_prints_both_sides() {
    let root = project("handshake", ANSWER);
    let listed = invoke(&root, &["languages"]);
    assert!(listed.status.success(), "{}", both(&listed));
    let report = String::from_utf8_lossy(&listed.stdout);
    assert!(
        report.contains(
            "the backend reports: discovery, build-test (asked as a process; matches the record)"
        ),
        "the report did not print the handshake it asked for: {report}"
    );
    let differing = r#"import json, sys
json.load(sys.stdin)
print(json.dumps({"protocol": 2, "source_extensions": ["rb"], "test_suffix": "_test", "language": "perl", "backend": "fixture-perl-v2",
                  "capabilities": ["discovery"]}))
"#;
    std::fs::write(root.join("backend.py"), differing).unwrap();
    let listed = invoke(&root, &["languages"]);
    assert!(listed.status.success(), "{}", both(&listed));
    let report = String::from_utf8_lossy(&listed.stdout);
    assert!(
        report.contains("the backend reports: discovery; the record claims discovery, build-test"),
        "a backend that reports fewer capabilities was not printed against the record: {report}"
    );
    assert!(
        report.contains(
            "the backend names itself `fixture-perl-v2`; the record names `fixture-perl`"
        ),
        "a backend that names itself differently was not reported by name: {report}"
    );
}

/// A handshake that is not this protocol, or claims a word this build does not know, is a refusal
/// with a name — a report never turns an unreadable answer into a silent line.
/// 不是本协议、或声称本构建不认识的词的握手，是点名的拒绝——报告绝不会把读不懂的应答变成沉默的一行。
#[test]
fn a_broken_handshake_is_refused_by_name() {
    let unknown = r#"import json, sys
json.load(sys.stdin)
print(json.dumps({"protocol": 2, "source_extensions": ["rb"], "test_suffix": "_test", "language": "perl", "backend": "fixture-perl",
                  "capabilities": ["discovery", "telepathy"]}))
"#;
    let root = project("unknown-capability", unknown);
    let listed = invoke(&root, &["languages"]);
    assert!(!listed.status.success(), "{}", both(&listed));
    let text = both(&listed);
    assert!(text.contains("`telepathy`"), "{text}");
    assert!(text.contains("does not know"), "{text}");

    let wrong_protocol = r#"import json, sys
json.load(sys.stdin)
print(json.dumps({"protocol": 3, "language": "perl", "backend": "fixture-perl",
                  "capabilities": ["discovery"]}))
"#;
    std::fs::write(root.join("backend.py"), wrong_protocol).unwrap();
    let listed = invoke(&root, &["languages"]);
    assert!(!listed.status.success(), "{}", both(&listed));
    assert!(both(&listed).contains("protocol 2"), "{}", both(&listed));
}

/// A backend whose plan tries to write outside the build directory.
/// 一个计划会往构建目录之外写入的后端。
const ESCAPE: &str = r#"import json, sys
request = json.load(sys.stdin)
if request.get("request") == "handshake":
    print(json.dumps({"protocol": 2, "source_extensions": ["rb"], "test_suffix": "_test", "language": "perl", "backend": "escape-perl",
                      "capabilities": ["discovery", "build-test"]}))
    sys.exit(0)
print(json.dumps({"protocol": 2, "inputs": request["inputs"], "language": "perl", "backend": "escape-perl",
                  "capabilities": ["discovery", "build-test"],
                  "actions": [{"kind": "compile", "program": "sh",
                               "args": ["-c", "echo escaped > ./pwned"]}]}))
"#;

#[test]
fn a_sandboxed_plan_cannot_write_outside_its_domain_and_an_unsandboxed_one_can() {
    // Same backend, same plan; the only difference is the entry's `sandbox` key.
    // 同一个后端、同一份计划，唯一区别是条目的 `sandbox` 键。
    for (label, sandbox) in [("escape-sandboxed", true), ("escape-open", false)] {
        let root = project(label, ESCAPE);
        let record = std::fs::read_to_string(root.join("xirang.backends.toml")).unwrap();
        let record = if sandbox {
            record.replace(
                "capabilities = [\"discovery\", \"build-test\"]",
                "sandbox = \"true\"\ncapabilities = [\"discovery\", \"build-test\"]",
            )
        } else {
            record
        };
        std::fs::write(root.join("xirang.backends.toml"), record).unwrap();
        let output = invoke(&root, &["build"]);
        if sandbox {
            assert!(!output.status.success(), "{}", both(&output));
            assert!(
                !root.join("pwned").exists(),
                "a write outside the domain left a file: {}",
                both(&output)
            );
            let text = both(&output);
            assert!(
                text.contains("Permission denied") || text.contains("EPERM"),
                "the failure must come from the kernel, not from the plan: {text}"
            );
        } else {
            assert!(output.status.success(), "{}", both(&output));
            assert_eq!(
                std::fs::read_to_string(root.join("pwned")).unwrap().trim(),
                "escaped"
            );
        }
    }
}

#[test]
fn the_confinement_entry_writes_inside_its_domain_and_refuses_outside() {
    let root = scratch("confine-entry");
    let domain = root.join("domain");
    std::fs::create_dir_all(&domain).unwrap();
    let inside = invoke(
        &root,
        &[
            "__confine",
            domain.to_str().unwrap(),
            "--",
            "sh",
            "-c",
            &format!("echo ok > {}/inside.txt", domain.display()),
        ],
    );
    assert!(inside.status.success(), "{}", both(&inside));
    assert_eq!(
        std::fs::read_to_string(domain.join("inside.txt"))
            .unwrap()
            .trim(),
        "ok"
    );
    let outside = invoke(
        &root,
        &[
            "__confine",
            domain.to_str().unwrap(),
            "--",
            "sh",
            "-c",
            &format!("echo bad > {}/outside.txt", root.display()),
        ],
    );
    assert!(!outside.status.success(), "{}", both(&outside));
    assert!(!root.join("outside.txt").exists(), "{}", both(&outside));
}

#[test]
fn the_conformance_kit_checks_handshake_and_plan_and_names_what_breaks() {
    let root = project("check-ok", ANSWER);
    let checked = invoke(&root, &["backends", "check", "--language", "perl"]);
    assert!(checked.status.success(), "{}", both(&checked));
    let report = String::from_utf8_lossy(&checked.stdout);
    assert!(report.contains("handshake: ok"), "{report}");
    assert!(report.contains("plan: ok"), "{report}");
    assert!(report.contains("protocol 2"), "{report}");

    // A backend that answers a plan but drops the input digest is refused by the same check the
    // build uses, and the refusal names the missing echo.
    // 给出计划却不回显输入摘要的后端被构建所用的同一道检查拒绝，且拒绝点名缺的回显。
    let no_echo = project(
        "check-no-echo",
        r#"import json, sys
request = json.load(sys.stdin)
if request.get("request") == "handshake":
    print(json.dumps({"protocol": 2, "source_extensions": ["rb"], "test_suffix": "_test", "language": "perl",
                      "backend": "no-echo", "capabilities": ["discovery", "build-test"]}))
    sys.exit(0)
print(json.dumps({"protocol": 2, "language": "perl", "backend": "no-echo", "capabilities": ["build-test"],
                  "actions": [{"kind": "compile", "program": sys.executable, "args": ["-c", "print(1)"]}]}))
"#,
    );
    let refused = invoke(&no_echo, &["backends", "check", "--language", "perl"]);
    assert!(!refused.status.success(), "{}", both(&refused));
    assert!(both(&refused).contains("inputs"), "{}", both(&refused));

    // A backend that plans nothing owns nothing.
    // 计划为空的 backend 什么也不拥有。
    let empty = project(
        "check-empty",
        r#"import json, sys
request = json.load(sys.stdin)
if request.get("request") == "handshake":
    print(json.dumps({"protocol": 2, "source_extensions": ["rb"], "test_suffix": "_test", "language": "perl",
                      "backend": "empty", "capabilities": ["discovery", "build-test"]}))
    sys.exit(0)
print(json.dumps({"protocol": 2, "inputs": request["inputs"], "language": "perl", "backend": "empty",
                  "capabilities": ["build-test"], "actions": []}))
"#,
    );
    let refused = invoke(&empty, &["backends", "check", "--language", "perl"]);
    assert!(!refused.status.success(), "{}", both(&refused));
    assert!(
        both(&refused).contains("nothing to run"),
        "{}",
        both(&refused)
    );
}

#[test]
fn a_planner_sandbox_keeps_the_backend_from_writing_outside_target() {
    let root = project("planner-sandbox", "");
    std::fs::write(
        root.join("xirang.backends.toml"),
        "protocol = 1\n\n[[backend]]\nlanguage = \"perl\"\nbackend = \"fixture-perl\"\n\
         capabilities = [\"discovery\", \"build-test\"]\ncommand = [\"python3\", \"backend.py\"]\n\
         planner_sandbox = \"true\"\n",
    )
    .unwrap();
    let script = r#"import json, sys, pathlib
request = json.load(sys.stdin)
if request.get("request") == "handshake":
    print(json.dumps({"protocol": 2, "language": "perl", "backend": "fixture-perl",
                      "capabilities": ["discovery", "build-test"],
                      "source_extensions": ["rb"], "test_suffix": "_test"}))
    sys.exit(0)
try:
    pathlib.Path("escaped.txt").write_text("outside the domain")
except OSError as error:
    print(f"write outside refused: {error}", file=sys.stderr)
    sys.exit(2)
print(json.dumps({"protocol": 2, "language": "perl", "backend": "fixture-perl",
                  "capabilities": ["discovery", "build-test"],
                  "inputs": request["inputs"],
                  "actions": [{"kind": "compile", "program": "sh", "args": ["-c", "true"]}]}))
"#;
    std::fs::write(root.join("backend.py"), script).unwrap();
    let sandboxed = invoke(&root, &["build"]);
    assert!(!sandboxed.status.success(), "{}", both(&sandboxed));
    assert!(
        both(&sandboxed).contains("Permission denied") || both(&sandboxed).contains("denied"),
        "{}",
        both(&sandboxed)
    );
    assert!(!root.join("escaped.txt").exists());

    // Control: the same backend without the flag writes the file and the build succeeds.
    let record = std::fs::read_to_string(root.join("xirang.backends.toml")).unwrap();
    std::fs::write(
        root.join("xirang.backends.toml"),
        record.replace("planner_sandbox = \"true\"\n", ""),
    )
    .unwrap();
    let plain = invoke(&root, &["build"]);
    assert!(plain.status.success(), "{}", both(&plain));
    assert!(root.join("escaped.txt").is_file());
}
