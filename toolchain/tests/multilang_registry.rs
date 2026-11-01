#![cfg(feature = "cli")]
//! `xirang registry check`: a registry mirror's shape (`<package>/<version>/xirang.toml`) is its
//! index, so a checker can prove before sharing that every version directory declares the package
//! and version it sits under, and name the highest version a `registry` source would pick.
//! `xirang registry check`：注册表镜像的形状就是它的索引；检查器在分享之前证明每个版本目录声明的
//! 包名与版本同它所在的位置一致，并点名 `registry` 源会选的最高版本。

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn binary() -> &'static str {
    env!("CARGO_BIN_EXE_xirang")
}

fn scratch(label: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "xirang-registry-{label}-{}-{}",
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
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

/// Write `<registry>/<package>/<version>/xirang.toml` declaring `declared_name`/`declared_version`.
fn version(
    registry: &Path,
    package: &str,
    version: &str,
    declared_name: &str,
    declared_version: &str,
) {
    let directory = registry.join(package).join(version);
    std::fs::create_dir_all(directory.join("src")).unwrap();
    std::fs::write(
        directory.join("xirang.toml"),
        format!(
            "schema = 1\n\n[package]\nname = \"{declared_name}\"\nversion = \"{declared_version}\"\n\
             kind = \"library\"\n\n[language.cpp]\nstandard = \"17\"\n\n[dependencies]\n"
        ),
    )
    .unwrap();
    std::fs::write(directory.join("src/lib.cpp"), "int value() { return 1; }\n").unwrap();
}

#[test]
fn a_shipped_shaped_registry_reports_its_packages_and_highest_versions() {
    let root = scratch("valid");
    let registry = root.join("registry");
    version(&registry, "greeter", "0.1.0", "greeter", "0.1.0");
    version(&registry, "greeter", "0.10.0", "greeter", "0.10.0");
    version(&registry, "widget", "1.0.0", "widget", "1.0.0");
    let checked = invoke(&root, &["registry", "check", "registry"]);
    assert!(checked.status.success(), "{}", both(&checked));
    let text = String::from_utf8_lossy(&checked.stdout);
    assert!(text.contains("2 package(s), 3 version(s)"), "{text}");
    assert!(
        text.contains("greeter: 2 version(s); highest 0.10.0"),
        "{text}"
    );
    assert!(
        text.contains("widget: 1 version(s); highest 1.0.0"),
        "{text}"
    );
}

#[test]
fn a_version_directory_that_declares_another_name_or_version_is_refused() {
    let root = scratch("mismatch");
    let registry = root.join("registry");
    version(&registry, "greeter", "0.1.0", "greeter", "0.1.0");
    version(&registry, "greeter", "0.2.0", "greeter", "0.3.0");
    let checked = invoke(&root, &["registry", "check", "registry"]);
    assert!(!checked.status.success());
    let message = both(&checked);
    assert!(
        message.contains("0.2.0") && message.contains("0.3.0"),
        "{message}"
    );
}

#[test]
fn a_package_with_no_version_directory_is_refused_by_name() {
    let root = scratch("empty");
    std::fs::create_dir_all(root.join("registry/empty-package")).unwrap();
    let checked = invoke(&root, &["registry", "check", "registry"]);
    assert!(!checked.status.success());
    let message = both(&checked);
    assert!(message.contains("empty-package"), "{message}");
    assert!(message.contains("no version"), "{message}");
}

#[test]
fn a_path_that_is_not_a_directory_is_refused() {
    let root = scratch("missing");
    std::fs::write(root.join("registry"), "not a directory\n").unwrap();
    let checked = invoke(&root, &["registry", "check", "registry"]);
    assert!(!checked.status.success());
    assert!(both(&checked).contains("is not a directory"));
}

#[test]
fn an_unknown_registry_action_is_refused() {
    let root = scratch("action");
    let refused = invoke(&root, &["registry", "publish", "registry"]);
    assert!(!refused.status.success());
    assert!(both(&refused).contains("check"));
}
