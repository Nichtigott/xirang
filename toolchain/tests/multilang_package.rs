#![cfg(feature = "cli")]
//! End-to-end: `xirang package` describes a finished build as one immutable directory with a
//! manifest, refuses to build anything itself, and does not silently overwrite its own output.
//! 端到端：`xirang package` 把已完成的构建描述成一个带清单的不可变目录，自己不构建，也不静默覆盖
//! 自己的产物。

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
        "xirang-package-{label}-{}-{:?}",
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
fn library(directory: &Path) {
    std::fs::create_dir_all(directory.join("src")).unwrap();
    std::fs::write(
        directory.join("xirang.toml"),
        "schema = 1\n\n[package]\nname = \"widget\"\nversion = \"0.1.0\"\nkind = \"library\"\n\n\
         [language.cpp]\nstandard = \"17\"\n\n[dependencies]\n",
    )
    .unwrap();
    std::fs::write(
        directory.join("src/widget.cpp"),
        "int answer() { return 42; }\n",
    )
    .unwrap();
}

/// The manifest of the package written into `directory`.
/// 写进 `directory` 的包的清单。
fn manifest(directory: &Path) -> serde_json::Value {
    let text = std::fs::read_to_string(directory.join("manifest.json")).unwrap();
    serde_json::from_str(&text).unwrap()
}

#[test]
fn packaging_after_a_build_writes_a_manifest_and_every_artifact() {
    let root = scratch("built");
    library(&root);
    let built = invoke(&root, &["build"]);
    assert!(built.status.success(), "{}", both(&built));
    let packaged = invoke(&root, &["package"]);
    assert!(packaged.status.success(), "{}", both(&packaged));
    let text = String::from_utf8_lossy(&packaged.stdout).into_owned();
    assert!(text.contains("digest "), "{text}");
    let plan = std::fs::read_dir(root.join("target/xirang/build"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let package = root
        .join("target/xirang/package")
        .join(plan.file_name().unwrap());
    let manifest = manifest(&package);
    assert_eq!(manifest["protocol"], 1);
    let files = manifest["files"].as_array().unwrap();
    assert!(!files.is_empty(), "{manifest}");
    for file in files {
        let path = package.join(file["path"].as_str().unwrap());
        assert!(path.is_file(), "{}", path.display());
        assert_eq!(file["sha256"].as_str().unwrap().len(), 64);
        assert!(file["bytes"].as_u64().unwrap() > 0);
    }
}

#[test]
fn packaging_before_a_build_is_refused_by_name() {
    let root = scratch("unbuilt");
    library(&root);
    let packaged = invoke(&root, &["package"]);
    assert!(!packaged.status.success(), "{}", both(&packaged));
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&packaged.stdout),
        String::from_utf8_lossy(&packaged.stderr)
    );
    assert!(text.contains("run `xirang build` first"), "{text}");
}

#[test]
fn packaging_twice_into_the_same_directory_is_refused() {
    let root = scratch("twice");
    library(&root);
    assert!(invoke(&root, &["build"]).status.success());
    assert!(invoke(&root, &["package"]).status.success());
    let second = invoke(&root, &["package"]);
    assert!(!second.status.success(), "{}", both(&second));
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&second.stdout),
        String::from_utf8_lossy(&second.stderr)
    );
    assert!(text.contains("already holds a package"), "{text}");
}

#[test]
fn an_explicit_output_directory_is_used() {
    let root = scratch("explicit");
    library(&root);
    assert!(invoke(&root, &["build"]).status.success());
    let output = root.join("delivery");
    let packaged = invoke(&root, &["package", "--output", output.to_str().unwrap()]);
    assert!(packaged.status.success(), "{}", both(&packaged));
    assert!(!manifest(&output)["files"].as_array().unwrap().is_empty());
}

#[test]
fn a_delivery_carries_the_generated_python_module_beside_its_shared_library() {
    let root = scratch("runtime");
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::write(
        root.join("xirang.toml"),
        "schema = 1\n\n[package]\nname = \"mathlib\"\nversion = \"0.1.0\"\nkind = \"library\"\n\n\
         [language.cpp]\nstandard = \"17\"\n\n[[export]]\nname = \"add\"\nparams = [\"i64\", \"i64\"]\n\
         returns = \"i64\"\n\n[dependencies]\n",
    )
    .unwrap();
    std::fs::write(
        root.join("src/mathlib.cpp"),
        "#include <cstdint>\nstd::int64_t add(std::int64_t a, std::int64_t b) { return a + b; }\n",
    )
    .unwrap();
    let built = invoke(&root, &["build"]);
    assert!(built.status.success(), "{}", both(&built));
    let packaged = invoke(&root, &["package", "--output", "delivery"]);
    assert!(packaged.status.success(), "{}", both(&packaged));
    let delivery = root.join("delivery");
    assert!(delivery.join("export/mathlib/exports.py").is_file());
    assert!(delivery.join("export/mathlib/libmathlib.so").is_file());
    let manifest: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(delivery.join("manifest.json")).unwrap())
            .unwrap();
    assert_eq!(
        manifest["runtime"]["python_modules"][0],
        "export/mathlib/exports.py"
    );
    assert_eq!(
        manifest["runtime"]["shared_libraries"][0],
        "export/mathlib/libmathlib.so"
    );
    let script = format!(
        "import sys\nsys.path.insert(0, {:?})\nimport exports\nassert exports.add(2, 3) == 5\nprint('runtime ok')\n",
        delivery.join("export/mathlib").to_str().unwrap()
    );
    let ran = Command::new("python3")
        .arg("-c")
        .arg(script)
        .output()
        .unwrap();
    assert!(ran.status.success(), "{}", both(&ran));
}

#[test]
fn verify_checks_every_file_a_package_manifest_names() {
    let root = scratch("verify-ok");
    library(&root);
    let built = invoke(&root, &["build"]);
    assert!(built.status.success(), "{}", both(&built));
    let packaged = invoke(&root, &["package", "--output", "delivery"]);
    assert!(packaged.status.success(), "{}", both(&packaged));
    let verified = invoke(&root, &["verify", "delivery"]);
    assert!(verified.status.success(), "{}", both(&verified));
    let text = String::from_utf8_lossy(&verified.stdout).into_owned();
    assert!(text.contains("verified "), "{text}");
    assert!(text.contains("file(s)"), "{text}");
}

#[test]
fn a_tampered_file_is_named_and_the_package_is_refused() {
    let root = scratch("verify-tampered");
    library(&root);
    assert!(invoke(&root, &["build"]).status.success());
    assert!(
        invoke(&root, &["package", "--output", "delivery"])
            .status
            .success()
    );
    let delivery = root.join("delivery");
    let manifest = manifest(&delivery);
    let named = manifest["files"][0]["path"].as_str().unwrap().to_owned();
    let mut bytes = std::fs::read(delivery.join(&named)).unwrap();
    bytes.push(b'\n');
    std::fs::write(delivery.join(&named), &bytes).unwrap();
    let verified = invoke(&root, &["verify", "delivery"]);
    assert!(!verified.status.success(), "{}", both(&verified));
    let text = both(&verified);
    assert!(text.contains(&named), "{text}");
    assert!(text.contains("does not match the manifest"), "{text}");
}

#[test]
fn a_file_no_manifest_entry_names_is_refused_by_name() {
    let root = scratch("verify-extra");
    library(&root);
    assert!(invoke(&root, &["build"]).status.success());
    assert!(
        invoke(&root, &["package", "--output", "delivery"])
            .status
            .success()
    );
    std::fs::write(root.join("delivery/extra.txt"), "not part of the package\n").unwrap();
    let verified = invoke(&root, &["verify", "delivery"]);
    assert!(!verified.status.success(), "{}", both(&verified));
    let text = both(&verified);
    assert!(text.contains("extra.txt"), "{text}");
    assert!(text.contains("no manifest entry names it"), "{text}");
}

#[test]
fn a_directory_without_a_manifest_is_refused_by_name() {
    let root = scratch("verify-empty");
    std::fs::create_dir_all(root.join("empty")).unwrap();
    let verified = invoke(&root, &["verify", "empty"]);
    assert!(!verified.status.success(), "{}", both(&verified));
    assert!(
        both(&verified).contains("no package manifest"),
        "{:?}",
        both(&verified)
    );
}

#[test]
fn a_package_that_carries_a_symlink_is_not_verified() {
    let root = scratch("symlink");
    library(&root);
    assert!(invoke(&root, &["build"]).status.success());
    let packaged = invoke(&root, &["package"]);
    assert!(packaged.status.success(), "{}", both(&packaged));
    let plan = std::fs::read_dir(root.join("target/xirang/build"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let package = root
        .join("target/xirang/package")
        .join(plan.file_name().unwrap());
    std::os::unix::fs::symlink("/etc/passwd", package.join("evil")).unwrap();
    let verified = invoke(&root, &["verify", package.to_str().unwrap()]);
    assert!(!verified.status.success(), "{}", both(&verified));
    let text = both(&verified);
    assert!(text.contains("symbolic link"), "{text}");
}
