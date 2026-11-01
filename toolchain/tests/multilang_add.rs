//! `xirang add --source`: a dependency reached through a named `[sources]` entry, with the source
//! declared by the same call when it is new.
//! `xirang add --source`：经具名 `[sources]` 条目到达的依赖；源是新的就由同一次调用声明。
//!
//! The tests drive the real binary: a source that resolves but is never used by a build would still
//! satisfy a text assertion, so each success case ends with `xirang test` compiling the new graph.

use std::path::{Path, PathBuf};
use std::process::Command;

/// A fresh scratch directory for one test.
/// 为一条测试开一个干净的临时目录。
fn scratch(label: &str) -> PathBuf {
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let directory = std::env::temp_dir().join(format!(
        "xirang-add-{label}-{}-{unique}",
        std::process::id()
    ));
    std::fs::create_dir_all(&directory).unwrap();
    directory
}

/// The binary under test.
/// 被测二进制。
fn xirang() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_xirang"))
}

/// Run one command in a directory.
/// 在目录里跑一条命令。
fn invoke(directory: &Path, args: &[&str]) -> std::process::Output {
    Command::new(xirang())
        .args(args)
        .current_dir(directory)
        .output()
        .expect("the xirang binary runs")
}

/// Both streams, for a failure message.
/// 两条流，供失败信息使用。
fn both(output: &std::process::Output) -> String {
    format!(
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

/// Write a file, creating its parents.
/// 写一个文件，必要时建父目录。
fn write_file(path: &Path, text: &str) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}

/// A C++ library that exports `greet() -> i64` returning 7.
/// 一个导出 `greet() -> i64`（返回 7）的 C++ 库。
fn greeter(directory: &Path) {
    write_file(
        &directory.join("xirang.toml"),
        "schema = 1\n\n[package]\nname = \"greeter\"\nversion = \"0.1.0\"\nkind = \"library\"\n\n\
         [language.cpp]\nstandard = \"17\"\n\n[[export]]\nname = \"greet\"\nparams = []\nreturns = \
         \"i64\"\n\n[dependencies]\n",
    );
    write_file(
        &directory.join("src/greeter.cpp"),
        "#include <cstdint>\n\nstd::int64_t greet() { return 7; }\n",
    );
}

/// The root project: a library whose own test calls the dependency's export.
/// 根项目：一个库，它自己的测试调用依赖的导出。
fn app(directory: &Path, sources: &str) {
    write_file(
        &directory.join("xirang.toml"),
        &format!(
            "schema = 1\n\n[package]\nname = \"app\"\nversion = \"0.1.0\"\nkind = \"library\"\n\n\
             [language.cpp]\nstandard = \"17\"\n\n{sources}[dependencies]\n"
        ),
    );
    write_file(
        &directory.join("src/app.cpp"),
        "#include <cstdint>\n\nstd::int64_t marker() { return 1; }\n",
    );
    write_file(
        &directory.join("tests/greet_test.cpp"),
        "#include <cassert>\n#include \"greeter/exports.h\"\n\nint main() {\n    assert(greet() == \
         7);\n    return 0;\n}\n",
    );
}

/// The manifest text of a project.
/// 项目清单的文本。
fn manifest(directory: &Path) -> String {
    std::fs::read_to_string(directory.join("xirang.toml")).unwrap()
}

#[test]
fn an_existing_local_source_becomes_a_dependency() {
    let root = scratch("existing");
    greeter(&root.join("team/greeter"));
    app(&root, "[sources]\nteam = { path = \"team\" }\n\n");
    let added = invoke(&root, &["add", "greeter", "--source", "team"]);
    assert!(added.status.success(), "{}", both(&added));
    let text = manifest(&root);
    assert!(
        text.contains("greeter = { source = \"team\", package = \"greeter\" }"),
        "the dependency was not recorded through the source:\n{text}"
    );
    let tested = invoke(&root, &["test"]);
    assert!(tested.status.success(), "{}", both(&tested));
}

#[test]
fn a_new_local_source_is_declared_by_the_same_call() {
    let root = scratch("declare");
    greeter(&root.join("team/greeter"));
    app(&root, "");
    let added = invoke(
        &root,
        &["add", "greeter", "--source", "team", "--path", "team"],
    );
    assert!(added.status.success(), "{}", both(&added));
    let text = manifest(&root);
    assert!(
        text.contains("[sources]") && text.contains("team = { path = \"team\" }"),
        "the source was not declared:\n{text}"
    );
    assert!(
        text.contains("greeter = { source = \"team\", package = \"greeter\" }"),
        "the dependency was not recorded through the source:\n{text}"
    );
    let tested = invoke(&root, &["test"]);
    assert!(tested.status.success(), "{}", both(&tested));
}

#[test]
fn a_git_source_is_declared_at_one_exact_revision() {
    let root = scratch("git");
    let repository = root.join("origin");
    // A git source resolves `<checkout>/<package>`, so the repository holds the package below its root.
    greeter(&repository.join("greeter"));
    for command in [
        vec!["init", "-q"],
        vec!["add", "-A"],
        vec![
            "-c",
            "user.name=xirang",
            "-c",
            "user.email=xirang@example.invalid",
            "commit",
            "-q",
            "-m",
            "greeter",
        ],
    ] {
        let status = Command::new("git")
            .args(&command)
            .current_dir(&repository)
            .status()
            .expect("git runs");
        assert!(status.success(), "git {command:?} failed");
    }
    let rev = String::from_utf8(
        Command::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(&repository)
            .output()
            .unwrap()
            .stdout,
    )
    .unwrap()
    .trim()
    .to_owned();
    let url = format!("file://{}", repository.display());
    app(&root, "");
    let added = invoke(
        &root,
        &[
            "add", "greeter", "--source", "team", "--git", &url, "--rev", &rev,
        ],
    );
    assert!(added.status.success(), "{}", both(&added));
    let tested = invoke(&root, &["test"]);
    assert!(tested.status.success(), "{}", both(&tested));
}

#[test]
fn the_refusals_leave_the_manifest_untouched() {
    let root = scratch("refusals");
    greeter(&root.join("team/greeter"));
    app(&root, "[sources]\nteam = { path = \"team\" }\n\n");
    let before = manifest(&root);
    for (args, wanted) in [
        (
            vec!["add", "greeter", "--source", "missing"],
            "is not a declared source",
        ),
        (
            vec!["add", "greeter", "--source", "team", "--path", "other"],
            "already declared",
        ),
        (
            vec![
                "add",
                "greeter",
                "--source",
                "next",
                "--git",
                "file:///nowhere",
            ],
            "--git needs --rev",
        ),
    ] {
        let refused = invoke(&root, &args);
        assert!(!refused.status.success(), "{args:?} was accepted");
        let message = both(&refused);
        assert!(
            message.contains(wanted),
            "{args:?} did not say `{wanted}`:\n{message}"
        );
        assert_eq!(manifest(&root), before, "{args:?} wrote the manifest");
    }
}
