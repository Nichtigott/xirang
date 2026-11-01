#![cfg(feature = "cli")]
//! End-to-end: `xirang install` verifies a package file by file, places it under a prefix as a
//! generation, and `xirang rollback` returns to the previous one. A tampered package installs
//! nothing.
//! 端到端：`xirang install` 逐文件校验包、把它作为世代安放到前缀下，`xirang rollback` 退回上一代。
//! 被改过的包什么都不装。

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
        "xirang-install-{label}-{}-{:?}",
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

/// Write one C++ library package at `directory`.
/// 在 `directory` 写一个 C++ 库包。
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

/// Build and package into `output`, returning the package directory.
/// 构建并打包到 `output`，返回包目录。
fn delivery(root: &Path, value: u32, output: &str) -> PathBuf {
    library(root, value);
    assert!(invoke(root, &["build"]).status.success());
    let packaged = invoke(root, &["package", "--output", output]);
    assert!(packaged.status.success(), "{}", both(&packaged));
    root.join(output)
}

/// The first file named by `package`'s manifest.
/// `package` 清单点名的第一个文件。
fn first_file(package: &Path) -> (String, String) {
    let text = std::fs::read_to_string(package.join("manifest.json")).unwrap();
    let manifest: serde_json::Value = serde_json::from_str(&text).unwrap();
    let file = &manifest["files"].as_array().unwrap()[0];
    (
        file["path"].as_str().unwrap().to_owned(),
        file["sha256"].as_str().unwrap().to_owned(),
    )
}

#[test]
fn an_installed_generation_holds_the_packages_bytes_and_its_manifest() {
    let root = scratch("installed");
    let package = delivery(&root, 42, "delivery");
    let prefix = root.join("prefix");
    let installed = invoke(
        &root,
        &[
            "install",
            package.to_str().unwrap(),
            "--prefix",
            prefix.to_str().unwrap(),
        ],
    );
    assert!(installed.status.success(), "{}", both(&installed));
    let generation = std::fs::read_link(prefix.join("current")).unwrap();
    let (relative, digest) = first_file(&package);
    let original = std::fs::read(package.join(&relative)).unwrap();
    let placed = prefix.join(&generation).join(&relative);
    assert_eq!(std::fs::read(&placed).unwrap(), original);
    assert_eq!(xirang_toolchain::sha256_hex(&original), digest);
    assert!(prefix.join(&generation).join("manifest.json").is_file());
}

#[test]
fn a_tampered_package_installs_nothing() {
    let root = scratch("tampered");
    let package = delivery(&root, 42, "delivery");
    let prefix = root.join("prefix");
    let (relative, _) = first_file(&package);
    let mut bytes = std::fs::read(package.join(&relative)).unwrap();
    bytes.push(b'!');
    std::fs::write(package.join(&relative), bytes).unwrap();
    let installed = invoke(
        &root,
        &[
            "install",
            package.to_str().unwrap(),
            "--prefix",
            prefix.to_str().unwrap(),
        ],
    );
    assert!(!installed.status.success(), "{}", both(&installed));
    let both_text = format!(
        "{}{}",
        String::from_utf8_lossy(&installed.stdout),
        String::from_utf8_lossy(&installed.stderr)
    );
    assert!(
        both_text.contains("does not match the manifest"),
        "{both_text}"
    );
    assert!(!prefix.join("current").exists());
}

#[test]
fn a_second_generation_can_be_taken_back_by_rollback() {
    let root = scratch("rollback");
    let first = delivery(&root, 42, "delivery-1");
    std::fs::remove_file(root.join("src/widget.cpp")).unwrap();
    let second = delivery(&root, 43, "delivery-2");
    assert!(invoke(&root, &["build"]).status.success());
    let prefix = root.join("prefix");
    for package in [&first, &second] {
        let installed = invoke(
            &root,
            &[
                "install",
                package.to_str().unwrap(),
                "--prefix",
                prefix.to_str().unwrap(),
            ],
        );
        assert!(installed.status.success(), "{}", both(&installed));
    }
    let first_generation = std::fs::read_link(prefix.join("current")).unwrap();
    let rolled = invoke(&root, &["rollback", "--prefix", prefix.to_str().unwrap()]);
    assert!(rolled.status.success(), "{}", both(&rolled));
    let back = std::fs::read_link(prefix.join("current")).unwrap();
    assert_ne!(first_generation, back);
    let (relative, digest) = first_file(&first);
    let original = std::fs::read(first.join(&relative)).unwrap();
    assert_eq!(xirang_toolchain::sha256_hex(&original), digest);
    assert_eq!(
        std::fs::read(prefix.join(&back).join(&relative)).unwrap(),
        original
    );
}

#[test]
fn rollback_without_a_previous_generation_is_refused_by_name() {
    let root = scratch("nothing");
    let prefix = root.join("prefix");
    std::fs::create_dir_all(&prefix).unwrap();
    let rolled = invoke(&root, &["rollback", "--prefix", prefix.to_str().unwrap()]);
    assert!(!rolled.status.success(), "{}", both(&rolled));
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&rolled.stdout),
        String::from_utf8_lossy(&rolled.stderr)
    );
    assert!(text.contains("no generation has been installed"), "{text}");
}
