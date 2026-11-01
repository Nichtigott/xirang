#![cfg(feature = "cli")]
//! End-to-end: `xirang history --prefix <dir>` reports the generations a prefix holds, verifies
//! each against the manifest it carries, and fails when the bytes drifted.
//! 端到端：`xirang history --prefix <dir>` 报告前缀里的世代、按各自清单校验，字节漂移时失败。

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

/// Build, package and install; return the prefix.
/// 构建、打包并安装；返回前缀。
fn installed(root: &Path) -> PathBuf {
    library(&root.join("widget"), 42);
    let project = root.join("widget");
    assert!(invoke(&project, &["build"]).status.success());
    let packaged = invoke(&project, &["package", "--output", "delivery"]);
    assert!(packaged.status.success(), "{}", both(&packaged));
    let prefix = root.join("prefix");
    let installed = invoke(
        &project,
        &[
            "install",
            project.join("delivery").to_str().unwrap(),
            "--prefix",
            prefix.to_str().unwrap(),
        ],
    );
    assert!(installed.status.success(), "{}", both(&installed));
    prefix
}

#[test]
fn history_reports_the_installed_generation_as_verified() {
    let root = scratch("history");
    let prefix = installed(&root);
    let reported = invoke(&root, &["history", "--prefix", prefix.to_str().unwrap()]);
    assert!(reported.status.success(), "{}", both(&reported));
    let text = String::from_utf8_lossy(&reported.stdout);
    assert!(text.contains("current generation"), "{text}");
    assert!(text.contains("verified"), "{text}");
    assert!(!text.contains("NOT verified"), "{text}");
}

#[test]
fn a_generation_whose_bytes_drifted_is_reported_and_fails() {
    let root = scratch("history-drift");
    let prefix = installed(&root);
    let generation = std::fs::read_link(prefix.join("current")).unwrap();
    let mut files = Vec::new();
    visit_files(&prefix.join(&generation), &mut files);
    let victim = files
        .iter()
        .find(|path| path.file_name().is_some_and(|name| name != "manifest.json"))
        .unwrap();
    std::fs::write(victim, b"tampered\n").unwrap();
    let reported = invoke(&root, &["history", "--prefix", prefix.to_str().unwrap()]);
    assert!(!reported.status.success(), "{}", both(&reported));
    let text = both(&reported);
    assert!(text.contains("NOT verified"), "{text}");
    assert!(text.contains("does not match its manifest"), "{text}");
}

#[test]
fn history_refuses_a_prefix_that_holds_nothing() {
    let root = scratch("history-empty");
    let empty = root.join("nowhere");
    let reported = invoke(&root, &["history", "--prefix", empty.to_str().unwrap()]);
    assert!(!reported.status.success(), "{}", both(&reported));
    assert!(
        both(&reported).contains("nothing has been installed under"),
        "{}",
        both(&reported)
    );
}

/// Collect every regular file under `directory`.
/// 收集 `directory` 下的每个普通文件。
fn visit_files(directory: &Path, files: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(directory) else {
        return;
    };
    for entry in entries {
        let path = entry.unwrap().path();
        if path.is_dir() {
            visit_files(&path, files);
        } else {
            files.push(path);
        }
    }
}

#[test]
fn a_damaged_current_generation_names_the_rollback_that_restores_a_verified_one() {
    let root = scratch("repair");
    let prefix = installed(&root);
    // A second, different generation so there is a verified previous one to go back to.
    library(&root.join("widget"), 43);
    let project = root.join("widget");
    assert!(invoke(&project, &["build"]).status.success());
    let packaged = invoke(&project, &["package", "--output", "delivery-two"]);
    assert!(packaged.status.success(), "{}", both(&packaged));
    let second = invoke(
        &project,
        &[
            "install",
            project.join("delivery-two").to_str().unwrap(),
            "--prefix",
            prefix.to_str().unwrap(),
        ],
    );
    assert!(second.status.success(), "{}", both(&second));
    let history: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(prefix.join("history.json")).unwrap())
            .unwrap();
    let current = history["current"].as_str().unwrap().to_owned();
    let mut files = Vec::new();
    visit_files(&prefix.join(&current), &mut files);
    let damaged = files
        .iter()
        .find(|file| file.file_name().is_some_and(|name| name != "manifest.json"))
        .expect("a generation holds files")
        .clone();
    std::fs::write(&damaged, b"damaged").unwrap();
    let reported = invoke(&root, &["history", "--prefix", prefix.to_str().unwrap()]);
    assert!(!reported.status.success());
    let text = both(&reported);
    assert!(text.contains("NOT verified"), "{text}");
    assert!(text.contains("next: xirang rollback --prefix"), "{text}");
    assert!(text.contains("which verifies"), "{text}");
}
