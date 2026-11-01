#![cfg(feature = "cli")]
//! End-to-end: `xirang update` re-resolves the graph and replaces `xirang.lock` in one transaction;
//! `--check` prints the diff and writes nothing; a broken graph leaves the recorded file untouched.
//! 端到端：`xirang update` 重新解析依赖图并在一次事务里替换 `xirang.lock`；`--check` 打印差异且不写
//! 文件；解析失败时已记录的锁文件一个字节都不动。

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// The binary under test.
/// 被测二进制。
fn binary() -> &'static str {
    env!("CARGO_BIN_EXE_xirang")
}

/// A fresh scratch directory.
/// 一个全新的临时目录。
fn scratch(label: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "xirang-update-{label}-{}-{:?}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&path).unwrap();
    path
}

/// Run the binary in `directory`.
/// 在 `directory` 里运行该二进制。
fn invoke(directory: &Path, arguments: &[&str]) -> Output {
    Command::new(binary())
        .args(arguments)
        .current_dir(directory)
        .output()
        .unwrap()
}

/// Both streams, for a failure message.
/// 两条输出流，供失败信息使用。
fn both(output: &Output) -> String {
    format!(
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

/// Write a library package with the given version.
/// 写一个给定版本的库包。
fn library(directory: &Path, name: &str, version: &str) {
    std::fs::create_dir_all(directory.join("src")).unwrap();
    std::fs::write(
        directory.join("xirang.toml"),
        format!(
            "schema = 1\n\n[package]\nname = \"{name}\"\nversion = \"{version}\"\nkind = \"library\"\n\n\
             [language.cpp]\nstandard = \"17\"\n\n[dependencies]\n"
        ),
    )
    .unwrap();
    std::fs::write(
        directory.join("src").join(format!("{name}.cpp")),
        "int value() { return 1; }\n",
    )
    .unwrap();
}

/// A project whose dependency is at `../dep`.
/// 一个依赖位于 `../dep` 的项目。
fn project(scratch: &Path) -> PathBuf {
    library(&scratch.join("dep"), "dep", "0.1.0");
    let consumer = scratch.join("consumer");
    library(&consumer, "consumer", "0.1.0");
    std::fs::write(
        consumer.join("xirang.toml"),
        "schema = 1\n\n[package]\nname = \"consumer\"\nversion = \"0.1.0\"\nkind = \"library\"\n\n\
         [language.cpp]\nstandard = \"17\"\n\n[dependencies]\ndep = { path = \"../dep\" }\n",
    )
    .unwrap();
    consumer
}

/// `--check` prints the diff and leaves the recorded file byte-identical.
/// `--check` 打印差异，并让已记录的锁文件逐字节不变。
#[test]
fn a_check_prints_the_diff_and_writes_nothing() {
    let scratch = scratch("check");
    let consumer = project(&scratch);
    let built = invoke(&consumer, &["build"]);
    assert!(built.status.success(), "{}", both(&built));
    let before = std::fs::read(consumer.join("xirang.lock")).unwrap();
    library(&scratch.join("dep"), "dep", "0.2.0");
    let checked = invoke(&consumer, &["update", "--check"]);
    assert!(checked.status.success(), "{}", both(&checked));
    let text = both(&checked);
    assert!(text.contains("would be updated"), "{text}");
    assert!(text.contains("~ dep 0.1.0"), "{text}");
    assert!(text.contains("dep 0.2.0"), "{text}");
    let after = std::fs::read(consumer.join("xirang.lock")).unwrap();
    assert_eq!(before, after, "`--check` must not write the lock");
}

/// `update` records the new graph once, and a later `--locked` build agrees with it.
/// `update` 记录一次新图，之后的 `--locked` 构建与它一致。
#[test]
fn an_update_records_the_new_graph_and_locked_builds_agree() {
    let scratch = scratch("record");
    let consumer = project(&scratch);
    let built = invoke(&consumer, &["build"]);
    assert!(built.status.success(), "{}", both(&built));
    library(&scratch.join("dep"), "dep", "0.2.0");
    let refused = invoke(&consumer, &["build", "--locked"]);
    assert!(!refused.status.success(), "{}", both(&refused));
    let updated = invoke(&consumer, &["update"]);
    assert!(updated.status.success(), "{}", both(&updated));
    let lock = std::fs::read_to_string(consumer.join("xirang.lock")).unwrap();
    assert!(lock.contains("version = \"0.2.0\""), "{lock}");
    assert!(!consumer.join("xirang.lock.tmp").exists());
    let built = invoke(&consumer, &["build", "--locked"]);
    assert!(built.status.success(), "{}", both(&built));
}

/// A graph that does not resolve leaves the recorded file untouched and no temporary behind.
/// 解析不成的图让已记录的锁文件原样保留，也不留下临时文件。
#[test]
fn a_broken_graph_leaves_the_lock_untouched() {
    let scratch = scratch("broken");
    let consumer = project(&scratch);
    let built = invoke(&consumer, &["build"]);
    assert!(built.status.success(), "{}", both(&built));
    let before = std::fs::read(consumer.join("xirang.lock")).unwrap();
    std::fs::write(
        consumer.join("xirang.toml"),
        "schema = 1\n\n[package]\nname = \"consumer\"\nversion = \"0.1.0\"\nkind = \"library\"\n\n\
         [language.cpp]\nstandard = \"17\"\n\n[dependencies]\ndep = { path = \"../missing\" }\n",
    )
    .unwrap();
    let refused = invoke(&consumer, &["update"]);
    assert!(!refused.status.success(), "{}", both(&refused));
    let after = std::fs::read(consumer.join("xirang.lock")).unwrap();
    assert_eq!(before, after);
    assert!(!consumer.join("xirang.lock.tmp").exists());
}

/// `update` refuses a directory that is not an `xirang.toml` project.
/// `update` 拒绝一个不是 `xirang.toml` 项目的目录。
#[test]
fn an_update_needs_a_project_manifest() {
    let scratch = scratch("nomanifest");
    let refused = invoke(&scratch, &["update"]);
    assert!(!refused.status.success(), "{}", both(&refused));
    assert!(
        both(&refused).contains("needs a project described by"),
        "{}",
        both(&refused)
    );
}
