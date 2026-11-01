#![cfg(feature = "cli")]
//! End-to-end: every install and rollback appends one chained line to a prefix's `log.jsonl`, and
//! `xirang log verify` recomputes that chain and compares it with the generations still on disk.
//! The tests tamper with one thing at a time - an edited field, a removed line, a removed tail, a
//! removed record, a removed log - and expect each to be named.
//! 端到端：每次安装与回退都向该前缀的 `log.jsonl` 追加一行链式记录，而 `xirang log verify` 会重算
//! 这条链并与磁盘上仍在的世代对照。测试每次只动一处——改一个字段、删一行、删末尾、删记录、删日志——
//! 并要求每一处都被点名。

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
        "xirang-multilang-{label}-{}-{:?}",
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

/// Both streams, for a readable failure message.
/// 两路输出，失败时容易读。
fn both(output: &Output) -> String {
    format!(
        "status {}\nstdout:\n{}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

/// Write a C++ library project.
/// 写一个 C++ 库项目。
fn library(directory: &Path, value: u32) {
    std::fs::create_dir_all(directory.join("src")).unwrap();
    std::fs::write(
        directory.join("xirang.toml"),
        "schema = 1\n\n[package]\nname = \"widget\"\nversion = \"0.1.0\"\nkind = \"library\"\n\n\
         [language.cpp]\nstandard = \"17\"\n\n[dependencies]\n",
    )
    .unwrap();
    std::fs::write(
        directory.join("src/widget.cpp"),
        format!("int answer() {{ return {value}; }}\n"),
    )
    .unwrap();
}

/// Build and package a library whose `answer()` returns `value`; return the packaged directory.
/// 构建并打包一个 `answer()` 返回 `value` 的库；返回打包目录。
fn packaged(root: &Path, value: u32) -> PathBuf {
    let project = root.join(format!("widget-{value}"));
    library(&project, value);
    let built = invoke(&project, &["build"]);
    assert!(built.status.success(), "{}", both(&built));
    let packaged = invoke(&project, &["package", "--output", "delivery"]);
    assert!(packaged.status.success(), "{}", both(&packaged));
    project.join("delivery")
}

/// Install `package` under `prefix`.
/// 把 `package` 安装到 `prefix` 下。
fn install(root: &Path, package: &Path, prefix: &Path) -> Output {
    invoke(
        root,
        &[
            "install",
            package.to_str().unwrap(),
            "--prefix",
            prefix.to_str().unwrap(),
        ],
    )
}

/// Build, package and install one generation; return the install output.
/// 构建、打包并安装一个世代；返回安装输出。
fn installed(root: &Path, prefix: &Path, value: u32) -> Output {
    let package = packaged(root, value);
    let output = install(root, &package, prefix);
    assert!(output.status.success(), "{}", both(&output));
    output
}

/// Run `xirang log verify --prefix <prefix>` with the given extra arguments.
/// 用给定额外参数运行 `xirang log verify --prefix <prefix>`。
fn verify(root: &Path, prefix: &Path, extra: &[&str]) -> Output {
    let mut arguments = vec!["log", "verify", "--prefix", prefix.to_str().unwrap()];
    arguments.extend_from_slice(extra);
    invoke(root, &arguments)
}

/// The current generation named by the prefix's history file.
/// 前缀历史文件点名的当前世代。
fn current(prefix: &Path) -> String {
    let text = std::fs::read_to_string(prefix.join("history.json")).unwrap();
    let value: serde_json::Value = serde_json::from_str(&text).unwrap();
    value["current"].as_str().unwrap().to_owned()
}

/// The generation a command made current, read from its own report.
/// 某条命令设为当前的世代，从它自己的报告里读。
fn generation_of(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .find_map(|line| line.strip_prefix("current -> ").map(str::to_owned))
        .unwrap_or_else(|| format!("no `current -> ` line in: {}", both(output)))
}

/// The lines of the prefix's log.
/// 前缀日志的各行。
fn lines(prefix: &Path) -> Vec<String> {
    std::fs::read_to_string(prefix.join("log.jsonl"))
        .unwrap()
        .lines()
        .map(str::to_owned)
        .collect()
}

/// Rewrite the log from the given lines.
/// 用给定的行重写日志。
fn rewrite(prefix: &Path, lines: &[String]) {
    let mut text = lines.join("\n");
    if !lines.is_empty() {
        text.push('\n');
    }
    std::fs::write(prefix.join("log.jsonl"), text).unwrap();
}

/// Install two generations and roll back to the first, so the log holds three state changes.
/// 装两个世代再回退到第一个，于是日志里有三次状态变化。
fn three_changes(root: &Path) -> (PathBuf, String, String) {
    let prefix = root.join("prefix");
    let first = installed(root, &prefix, 42);
    let second = installed(root, &prefix, 7);
    assert!(
        String::from_utf8_lossy(&first.stdout).contains("log -> entry 1"),
        "{}",
        both(&first)
    );
    assert!(
        String::from_utf8_lossy(&second.stdout).contains("log -> entry 2"),
        "{}",
        both(&second)
    );
    let rolled = invoke(root, &["rollback", "--prefix", prefix.to_str().unwrap()]);
    assert!(rolled.status.success(), "{}", both(&rolled));
    assert!(
        String::from_utf8_lossy(&rolled.stdout).contains("log -> entry 3"),
        "{}",
        both(&rolled)
    );
    assert_eq!(lines(&prefix).len(), 3, "{}", lines(&prefix).join("\n"));
    let first = generation_of(&first);
    let second = generation_of(&second);
    assert_ne!(first, second, "two payloads must be two generations");
    assert_eq!(
        current(&prefix),
        first,
        "the rollback made the first current"
    );
    (
        prefix,
        first,
        String::from_utf8_lossy(&rolled.stdout)
            .lines()
            .find_map(|line| line.strip_prefix("current -> ").map(str::to_owned))
            .unwrap_or_default(),
    )
}

#[test]
fn every_state_change_is_chained_and_verifies() {
    let root = scratch("log-verify");
    let (prefix, installed_first, _) = three_changes(&root);
    let verified = verify(&root, &prefix, &[]);
    assert!(verified.status.success(), "{}", both(&verified));
    let text = String::from_utf8_lossy(&verified.stdout);
    assert!(
        text.contains("3 recorded change(s), chain intact"),
        "{text}"
    );
    assert!(text.contains("every one recorded"), "{text}");
    assert!(
        text.contains(&format!(
            "current: {installed_first} (the last recorded state change)"
        )),
        "{text}"
    );
    assert_eq!(
        current(&prefix),
        installed_first,
        "the rollback made the first generation current again"
    );
    let data = verify(&root, &prefix, &["--json"]);
    assert!(data.status.success(), "{}", both(&data));
    let value: serde_json::Value =
        serde_json::from_str(&String::from_utf8_lossy(&data.stdout)).unwrap();
    assert_eq!(value["ok"], true, "{value}");
    assert_eq!(value["entries"], 3, "{value}");
    assert_eq!(value["current"], installed_first.as_str(), "{value}");
    assert_eq!(value["problems"].as_array().unwrap().len(), 0, "{value}");
    let seqs: Vec<u64> = lines(&prefix)
        .iter()
        .map(|line| {
            serde_json::from_str::<serde_json::Value>(line).unwrap()["seq"]
                .as_u64()
                .unwrap()
        })
        .collect();
    assert_eq!(seqs, vec![1, 2, 3]);
}

#[test]
fn an_edited_field_is_reported_against_its_own_digest() {
    let root = scratch("log-edited");
    let (prefix, _, _) = three_changes(&root);
    let mut lines = lines(&prefix);
    lines[0] = lines[0].replacen("\"action\":\"install\"", "\"action\":\"rollback\"", 1);
    rewrite(&prefix, &lines);
    let verified = verify(&root, &prefix, &[]);
    assert!(!verified.status.success(), "{}", both(&verified));
    let text = both(&verified);
    assert!(text.contains("line 1 says its entry is"), "{text}");
    assert!(text.contains("fields hash to"), "{text}");
    assert!(text.contains("edited after it was written"), "{text}");
    assert!(text.contains("next:"), "{text}");
    let data = verify(&root, &prefix, &["--json"]);
    assert!(!data.status.success(), "{}", both(&data));
    let value: serde_json::Value =
        serde_json::from_str(&String::from_utf8_lossy(&data.stdout)).unwrap();
    assert_eq!(value["ok"], false, "{value}");
    assert!(!value["problems"].as_array().unwrap().is_empty(), "{value}");
}

#[test]
fn a_removed_line_is_reported_by_the_line_after_it() {
    let root = scratch("log-gap");
    let (prefix, _, _) = three_changes(&root);
    let mut lines = lines(&prefix);
    lines.remove(1);
    rewrite(&prefix, &lines);
    let verified = verify(&root, &prefix, &[]);
    assert!(!verified.status.success(), "{}", both(&verified));
    let text = both(&verified);
    assert!(
        text.contains("records seq 3, but the log's lines are numbered from 1 without gaps"),
        "{text}"
    );
    assert!(
        text.contains("entries cannot be inserted or removed"),
        "{text}"
    );
}

#[test]
fn a_removed_tail_is_reported_by_the_current_link() {
    let root = scratch("log-tail");
    let (prefix, _, _) = three_changes(&root);
    let mut lines = lines(&prefix);
    lines.pop();
    rewrite(&prefix, &lines);
    let verified = verify(&root, &prefix, &[]);
    assert!(!verified.status.success(), "{}", both(&verified));
    let text = both(&verified);
    assert!(
        text.contains("the `current` link points at")
            && text.contains("but the last recorded state change made"),
        "{text}"
    );
}

#[test]
fn a_generation_that_lost_its_record_is_reported() {
    let root = scratch("log-record");
    let (prefix, first, _) = three_changes(&root);
    std::fs::remove_file(prefix.join(&first).join("provenance.json")).unwrap();
    let verified = verify(&root, &prefix, &[]);
    assert!(!verified.status.success(), "{}", both(&verified));
    let text = both(&verified);
    assert!(
        text.contains("carries no readable `provenance.json`"),
        "{text}"
    );
    assert!(text.contains("which log entry made it current"), "{text}");
}

#[test]
fn a_prefix_whose_log_was_deleted_is_reported() {
    let root = scratch("log-deleted");
    let (prefix, _, _) = three_changes(&root);
    std::fs::remove_file(prefix.join("log.jsonl")).unwrap();
    let verified = verify(&root, &prefix, &[]);
    assert!(!verified.status.success(), "{}", both(&verified));
    let text = both(&verified);
    assert!(
        text.contains("no line of the log records it becoming current"),
        "{text}"
    );
}

#[test]
fn an_install_refuses_to_append_to_a_log_that_does_not_verify() {
    let root = scratch("log-refuse");
    let (prefix, _, _) = three_changes(&root);
    let mut lines = lines(&prefix);
    lines[0] = lines[0].replacen("\"action\":\"install\"", "\"action\":\"rollback\"", 1);
    rewrite(&prefix, &lines);
    let before = std::fs::read(prefix.join("log.jsonl")).unwrap();
    let was_current = current(&prefix);
    let package = root.join("widget-42").join("delivery");
    let refused = install(&root, &package, &prefix);
    assert!(!refused.status.success(), "{}", both(&refused));
    let text = both(&refused);
    assert!(text.contains("fields hash to"), "{text}");
    assert_eq!(
        std::fs::read(prefix.join("log.jsonl")).unwrap(),
        before,
        "a refused install must not write to the log"
    );
    assert_eq!(current(&prefix), was_current, "nor change what is current");
}
