//! The declaration interchange a non-Rust program can emit: `xirang declaration new` writes a
//! starter that prints the identity subset of one registration face (name/kind/contracts plus
//! preset, parts, requires, provides, exports, stable_name, graft and flow), and `xirang declaration
//! check` normalises it. Two languages saying the same thing must compare equal.
//! 非 Rust 程序可以发出的声明交换：`xirang declaration new` 写出一段打印某个注册面身份子集（含
//! preset/parts/requires/provides/exports/stable_name 与 graft/flow）的起点程序，
//! `xirang declaration check` 把它规范化。两种语言说同一件事时必须比较相等。

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn scratch(label: &str) -> PathBuf {
    let path =
        std::env::temp_dir().join(format!("xirang-declaration-{label}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).unwrap();
    path
}

fn xirang() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_xirang"))
}

fn invoke(directory: &Path, arguments: &[&str]) -> Output {
    Command::new(xirang())
        .args(arguments)
        .current_dir(directory)
        .output()
        .unwrap()
}

fn tool_available(program: &str, flag: &str) -> bool {
    Command::new(program)
        .arg(flag)
        .output()
        .is_ok_and(|output| output.status.success())
}

fn check_document(directory: &Path, document: &Path) -> Output {
    invoke(
        directory,
        &["declaration", "check", document.to_str().unwrap()],
    )
}

#[test]
fn a_python_starter_emits_the_declaration_the_check_normalises() {
    if !tool_available("python3", "--version") {
        eprintln!("skipping: no python3");
        return;
    }
    let root = scratch("python");
    let emitted = invoke(
        &root,
        &[
            "declaration",
            "new",
            "--language",
            "python",
            "--name",
            "Button",
            "--kind",
            "Button",
            "--contract",
            "control::ControlHandle",
            "--directory",
            "emitters",
        ],
    );
    assert!(
        emitted.status.success(),
        "{}",
        String::from_utf8_lossy(&emitted.stderr)
    );
    let script = root.join("emitters/button.py");
    let output = Command::new("python3").arg(&script).output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let document = root.join("python.json");
    std::fs::write(&document, &output.stdout).unwrap();
    let checked = check_document(&root, &document);
    assert!(
        checked.status.success(),
        "{}",
        String::from_utf8_lossy(&checked.stderr)
    );
    let canonical = String::from_utf8_lossy(&checked.stdout);
    assert!(canonical.contains("\"name\":\"Button\""), "{canonical}");
    assert!(canonical.contains("control::ControlHandle"), "{canonical}");
}

#[test]
fn a_cpp_starter_emits_the_same_canonical_document() {
    if !tool_available("g++", "--version") || !tool_available("python3", "--version") {
        eprintln!("skipping: no g++ or python3");
        return;
    }
    let root = scratch("cpp");
    for language in ["python", "cpp"] {
        let emitted = invoke(
            &root,
            &[
                "declaration",
                "new",
                "--language",
                language,
                "--name",
                "Button",
                "--kind",
                "Button",
                "--contract",
                "control::ControlHandle",
                "--directory",
                "emitters",
            ],
        );
        assert!(
            emitted.status.success(),
            "{}",
            String::from_utf8_lossy(&emitted.stderr)
        );
    }
    let compile = Command::new("g++")
        .args(["-std=c++17", "-o"])
        .arg(root.join("emitters/button"))
        .arg(root.join("emitters/button.cpp"))
        .output()
        .unwrap();
    assert!(
        compile.status.success(),
        "{}",
        String::from_utf8_lossy(&compile.stderr)
    );
    let running = Command::new(root.join("emitters/button")).output().unwrap();
    assert!(running.status.success());
    let cpp = root.join("cpp.json");
    std::fs::write(&cpp, &running.stdout).unwrap();

    let python = Command::new("python3")
        .arg(root.join("emitters/button.py"))
        .output()
        .unwrap();
    let python_document = root.join("python.json");
    std::fs::write(&python_document, &python.stdout).unwrap();

    let cpp_checked = check_document(&root, &cpp);
    let python_checked = check_document(&root, &python_document);
    assert!(
        cpp_checked.status.success(),
        "{}",
        String::from_utf8_lossy(&cpp_checked.stderr)
    );
    assert!(python_checked.status.success());
    assert_eq!(
        String::from_utf8_lossy(&cpp_checked.stdout),
        String::from_utf8_lossy(&python_checked.stdout),
        "two languages said the same thing and the normal forms differ"
    );
}

#[test]
fn a_document_outside_the_identity_subset_is_refused_by_name() {
    let root = scratch("refusals");
    let cases: Vec<(&str, &str, &str)> = vec![
        (
            "extra.json",
            "{\"protocol\":1,\"faces\":[{\"name\":\"Button\",\"kind\":\"Button\",\"contracts\":[\"a::B\"],\"colour\":\"red\"}]}",
            "unknown key",
        ),
        (
            "protocol.json",
            "{\"protocol\":2,\"faces\":[{\"name\":\"Button\",\"kind\":\"Button\",\"contracts\":[\"a::B\"]}]}",
            "protocol",
        ),
        (
            "duplicate.json",
            "{\"protocol\":1,\"faces\":[{\"name\":\"Button\",\"kind\":\"Button\",\"contracts\":[\"a::B\"]},{\"name\":\"Button\",\"kind\":\"Slider\",\"contracts\":[\"a::B\"]}]}",
            "Button",
        ),
        (
            "contracts.json",
            "{\"protocol\":1,\"faces\":[{\"name\":\"Button\",\"kind\":\"Button\",\"contracts\":[]}]}",
            "contract",
        ),
        (
            "kind.json",
            "{\"protocol\":1,\"faces\":[{\"name\":\"Button\",\"kind\":\"not a kind\",\"contracts\":[\"a::B\"]}]}",
            "kind",
        ),
    ];
    for (file, body, expected) in cases {
        let document = root.join(file);
        std::fs::write(&document, body).unwrap();
        let checked = check_document(&root, &document);
        assert!(!checked.status.success(), "{file} was accepted");
        let message = String::from_utf8_lossy(&checked.stderr).into_owned();
        assert!(
            message.contains(expected),
            "{file} was refused without naming `{expected}`: {message}"
        );
    }
}

#[test]
fn a_starter_for_an_unknown_language_is_refused() {
    let root = scratch("language");
    let refused = invoke(
        &root,
        &[
            "declaration",
            "new",
            "--language",
            "ruby",
            "--name",
            "Button",
            "--directory",
            "emitters",
        ],
    );
    assert!(!refused.status.success());
    let message = String::from_utf8_lossy(&refused.stderr).into_owned();
    assert!(
        message.contains("python") && message.contains("cpp"),
        "{message}"
    );
}

#[test]
fn the_richer_identity_subset_round_trips_through_a_python_starter() {
    if !tool_available("python3", "--version") {
        eprintln!("skipping: no python3");
        return;
    }
    let root = scratch("richer");
    let created = invoke(
        &root,
        &[
            "declaration",
            "new",
            "--language",
            "python",
            "--name",
            "Button",
            "--kind",
            "Button",
            "--contract",
            "control::ControlHandle",
            "--preset",
            "crate::preset::Button",
            "--part",
            "ScaleParts",
            "--requires",
            "control.render.v1=engine",
            "--provides",
            "render",
            "--exports",
            "size",
            "--stable-name",
            "button",
            "--directory",
            ".",
        ],
    );
    assert!(
        created.status.success(),
        "{}",
        String::from_utf8_lossy(&created.stderr)
    );
    let emitted = Command::new("python3")
        .arg("button.py")
        .current_dir(&root)
        .output()
        .unwrap();
    assert!(
        emitted.status.success(),
        "{}",
        String::from_utf8_lossy(&emitted.stderr)
    );
    let output = String::from_utf8_lossy(&emitted.stdout).trim().to_owned();
    for expected in [
        "\"preset\":\"crate::preset::Button\"",
        "\"parts\":[\"ScaleParts\"]",
        "\"requires\":{\"control.render.v1\":\"engine\"}",
        "\"provides\":[\"render\"]",
        "\"exports\":[\"size\"]",
        "\"stable_name\":\"button\"",
    ] {
        assert!(output.contains(expected), "{output}");
    }
    let document = root.join("emitted.json");
    std::fs::write(&document, format!("{output}\n")).unwrap();
    let checked = check_document(&root, &document);
    assert!(
        checked.status.success(),
        "{}",
        String::from_utf8_lossy(&checked.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&checked.stdout).trim(),
        output,
        "the canonical form must equal what the starter emitted"
    );
}

#[test]
fn the_richer_subset_refuses_fields_it_cannot_read() {
    let root = scratch("richer-refuse");
    let write = |name: &str, body: &str| {
        let path = root.join(name);
        std::fs::write(&path, body).unwrap();
        path
    };
    let cases = [
        (
            "bad-preset.json",
            "{\"protocol\":1,\"faces\":[{\"name\":\"Button\",\"kind\":\"Button\",\"contracts\":[\"control::ControlHandle\"],\"preset\":\"Foo<Bar>\"}]}",
            "not a path",
        ),
        (
            "bad-requires.json",
            "{\"protocol\":1,\"faces\":[{\"name\":\"Button\",\"kind\":\"Button\",\"contracts\":[\"control::ControlHandle\"],\"requires\":{\"cap\":3}}]}",
            "not a string",
        ),
        (
            "duplicate-exports.json",
            "{\"protocol\":1,\"faces\":[{\"name\":\"Button\",\"kind\":\"Button\",\"contracts\":[\"control::ControlHandle\"],\"exports\":[\"size\",\"size\"]}]}",
            "twice",
        ),
    ];
    for (file, body, expected) in cases {
        let document = write(file, body);
        let refused = check_document(&root, &document);
        assert!(!refused.status.success(), "{file} was accepted");
        let message = String::from_utf8_lossy(&refused.stderr);
        assert!(message.contains(expected), "{file}: {message}");
    }
}

// ---- The declaration documents a build consumes -------------------------------------------
// ---- 构建消费的声明文档 ----------------------------------------------------------------

/// One document that fills the whole identity subset, so the tree has something to publish.
/// 一份填满整个身份子集的文档，让树有东西可发布。
const BUTTON: &str = r#"{"protocol":1,"faces":[{"name":"Button","kind":"Button","contracts":["control::ControlHandle"],"preset":"crate::preset::Button","parts":["ScaleParts"],"requires":{"control.render.v1":"engine"},"provides":["render"],"exports":["size"],"stable_name":"button"}]}"#;

/// The id this tree gives that document. Identities are written into `registry.json`, so this is an
/// on-disk contract: a changed hashing rule renames every declared face, exactly like the Rust tree.
/// 这份树给该文档的身份。身份会被写进 `registry.json`，因此这是落盘契约：散列规则一变，每个声明面
/// 都会改名，和 Rust 树完全一样。
const BUTTON_ID: &str = "a72cec1fba1328a71937a8daf413df01";

/// A fresh C++ library project, so a plan and a build need nothing but a C++ compiler.
/// 一个全新的 C++ 库项目，于是计划与构建除了 C++ 编译器什么都不需要。
fn project(label: &str) -> PathBuf {
    let root = scratch(label);
    let created = invoke(&root, &["new", "--lang", "cpp", "--lib", "widget"]);
    assert!(
        created.status.success(),
        "{}",
        String::from_utf8_lossy(&created.stderr)
    );
    root.join("widget")
}

/// Write one declaration document under the project's `declarations/` directory.
/// 在项目的 `declarations/` 下写一份声明文档。
fn declared(root: &Path, file: &str, contents: &str) -> PathBuf {
    let path = root.join("declarations").join(file);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, contents).unwrap();
    path
}

/// Run an invocation that must succeed, and return its standard output.
/// 跑一条必须成功的调用，返回它的标准输出。
fn succeeded(directory: &Path, arguments: &[&str]) -> String {
    let output = invoke(directory, arguments);
    assert!(
        output.status.success(),
        "{arguments:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).into_owned()
}

/// The plan id `xirang plan --json` reports; the plan directory is named after it.
/// `xirang plan --json` 报出的计划身份；计划目录以它命名。
fn plan_id(directory: &Path) -> String {
    let text = succeeded(directory, &["plan", "--json"]);
    let marker = "\"plan\": \"";
    let start = text.find(marker).expect("the plan is named") + marker.len();
    text[start..start + 16].to_owned()
}

#[test]
fn a_declared_face_enters_the_plan_identity_and_the_published_tree() {
    let root = project("declared-tree");
    let without = plan_id(&root);
    declared(&root, "button.json", BUTTON);
    let with = plan_id(&root);
    assert_ne!(
        without, with,
        "a declaration document is an input of the plan, so the plan must change"
    );

    let printed = succeeded(&root, &["declaration", "tree"]);
    assert!(printed.starts_with("namespace widget\n"), "{printed}");
    let row = printed
        .lines()
        .find(|line| line.starts_with(BUTTON_ID))
        .unwrap_or_else(|| panic!("no row for the declared face:\n{printed}"));
    assert_eq!(
        row,
        format!("{BUTTON_ID}\tButton\tButton\tcontrol::ControlHandle\tdeclarations/button.json"),
        "the read surface and the artifact must describe one tree"
    );

    let listed = succeeded(&root, &["plan", "--json"]);
    assert!(
        listed.contains(&format!("{}/registry.json", with)),
        "the plan announces the tree it will publish:\n{listed}"
    );
    let built = succeeded(&root, &["build"]);
    assert!(built.contains(&format!("plan {with}")), "{built}");

    let artifact = std::fs::read_to_string(
        root.join("target/xirang/build")
            .join(&with)
            .join("registry.json"),
    )
    .expect("the build publishes registry.json beside the generated files");
    assert_eq!(
        artifact,
        succeeded(&root, &["declaration", "tree", "--json"]),
        "printing and building must read one tree"
    );
    for expected in [
        "\"document\":\"declarations/button.json\"",
        "\"kind\":\"Button\"",
        "\"name\":\"Button\"",
        "\"preset\":\"crate::preset::Button\"",
        "\"parts\":[\"ScaleParts\"]",
        "\"provides\":[\"render\"]",
        "\"exports\":[\"size\"]",
        "\"stable_name\":\"button\"",
        "\"requires\":{\"control.render.v1\":\"engine\"}",
        "\"namespace\":\"widget\"",
    ] {
        assert!(
            artifact.contains(expected),
            "{expected} missing from {artifact}"
        );
    }
}

#[test]
fn editing_one_declared_field_is_another_plan() {
    let root = project("declared-edit");
    declared(&root, "button.json", BUTTON);
    let first = plan_id(&root);
    declared(
        &root,
        "button.json",
        &BUTTON.replace(
            "control::ControlHandle",
            "control::ControlHandle\",\"control::Focusable",
        ),
    );
    let second = plan_id(&root);
    assert_ne!(first, second, "a changed contract must change the plan");
    let printed = succeeded(&root, &["declaration", "tree"]);
    assert!(
        printed.contains("control::ControlHandle,control::Focusable"),
        "{printed}"
    );
}

#[test]
fn two_documents_that_declare_one_name_are_refused() {
    let root = project("declared-duplicate");
    declared(&root, "first.json", BUTTON);
    declared(&root, "second.json", BUTTON);
    let refused = invoke(&root, &["plan"]);
    assert!(
        !refused.status.success(),
        "two `Button` faces were accepted"
    );
    let message = String::from_utf8_lossy(&refused.stderr);
    assert!(
        message.contains("`declarations/first.json` and `declarations/second.json` both declare a face named `Button`"),
        "{message}"
    );
    assert!(
        !root.join("target").exists(),
        "a refused plan must not create the build directory"
    );
}

#[test]
fn a_document_the_tree_cannot_read_names_its_file() {
    let root = project("declared-broken");
    let broken = declared(
        &root,
        "broken.json",
        r#"{"protocol":1,"faces":[{"name":"Button","kind":"Button","contracts":[],"colour":"red"}]}"#,
    );
    let refused = invoke(&root, &["plan"]);
    assert!(!refused.status.success(), "an unknown key was accepted");
    let message = String::from_utf8_lossy(&refused.stderr);
    assert!(message.contains(broken.to_str().unwrap()), "{message}");
    assert!(message.contains("unknown key `colour`"), "{message}");

    std::fs::remove_file(&broken).unwrap();
    std::fs::write(root.join("declarations/notes.txt"), "not a document").unwrap();
    let refused = invoke(&root, &["plan"]);
    assert!(!refused.status.success(), "a non-document was accepted");
    let message = String::from_utf8_lossy(&refused.stderr);
    assert!(message.contains("is not a `.json` document"), "{message}");

    std::fs::remove_file(root.join("declarations/notes.txt")).unwrap();
    std::fs::create_dir_all(root.join("declarations/nested")).unwrap();
    let refused = invoke(&root, &["plan"]);
    assert!(!refused.status.success(), "a nested directory was accepted");
    let message = String::from_utf8_lossy(&refused.stderr);
    assert!(
        message.contains("is a directory; a declaration document is a `.json` file"),
        "{message}"
    );
}

#[test]
fn a_project_that_declares_no_face_has_an_empty_tree() {
    let root = project("declared-empty");
    let printed = succeeded(&root, &["declaration", "tree"]);
    assert_eq!(printed.lines().count(), 3, "{printed}");
    assert!(printed.starts_with("namespace widget\n"), "{printed}");
    let as_json = succeeded(&root, &["declaration", "tree", "--json"]);
    assert!(as_json.contains("\"faces\":[]"), "{as_json}");
    assert!(as_json.contains("\"namespace\":\"widget\""), "{as_json}");
}
