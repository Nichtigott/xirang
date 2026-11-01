#![cfg(feature = "cli")]
//! End-to-end: `xirang cache export`/`import` move a build's content store and stamps to another
//! checkout, and an import refuses a tampered export before writing anything.
//! 端到端：`xirang cache export`/`import` 把一个构建的内容库与印记搬到另一个检出；被篡改的导出在
//! 写入任何东西之前就被拒绝。

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
        "xirang-cache-{label}-{}-{:?}",
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

/// Scaffold one C++ library and build it, returning its directory.
/// 生成一个 C++ 库并构建它，返回它的目录。
fn built_project(scratch: &Path, label: &str) -> PathBuf {
    let created = invoke(scratch, &["new", "--lang", "cpp", "--lib", label]);
    assert!(created.status.success(), "{}", both(&created));
    let project = scratch.join(label);
    let built = invoke(&project, &["build"]);
    assert!(built.status.success(), "{}", both(&built));
    project
}

/// Copy a project's sources without its `target/` directory.
/// 复制一个项目的源码，不带 `target/` 目录。
fn copy_project(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let name = entry.file_name();
        if name == "target" {
            continue;
        }
        let path = entry.path();
        if path.is_dir() {
            copy_project(&path, &to.join(&name));
        } else {
            std::fs::copy(&path, to.join(&name)).unwrap();
        }
    }
}

/// The newest evidence file under `project/target/xirang/build`.
/// `project/target/xirang/build` 下最新的证据文件。
fn newest_evidence(project: &Path) -> PathBuf {
    let mut best: Option<(std::time::SystemTime, PathBuf)> = None;
    let build = project.join("target").join("xirang").join("build");
    for entry in std::fs::read_dir(&build).unwrap() {
        let candidate = entry.unwrap().path().join("evidence.json");
        if let Ok(metadata) = std::fs::metadata(&candidate) {
            let modified = metadata.modified().unwrap();
            if best.as_ref().is_none_or(|(time, _)| modified > *time) {
                best = Some((modified, candidate));
            }
        }
    }
    best.expect("a build wrote evidence").1
}

#[test]
fn an_export_then_import_lets_another_checkout_reuse_the_build() {
    let first = scratch("first");
    let project = built_project(&first, "widget");
    let export = first.join("cache-export");
    let exported = invoke(&project, &["cache", "export", export.to_str().unwrap()]);
    assert!(exported.status.success(), "{}", both(&exported));
    assert!(
        String::from_utf8_lossy(&exported.stdout).contains("exported"),
        "{}",
        both(&exported)
    );

    let second = scratch("second");
    copy_project(&project, &second.join("widget"));
    let checkout = second.join("widget");
    let imported = invoke(&checkout, &["cache", "import", export.to_str().unwrap()]);
    assert!(imported.status.success(), "{}", both(&imported));
    assert!(
        String::from_utf8_lossy(&imported.stdout).contains("imported"),
        "{}",
        both(&imported)
    );

    let rebuilt = invoke(&checkout, &["build"]);
    assert!(rebuilt.status.success(), "{}", both(&rebuilt));
    let evidence = std::fs::read_to_string(newest_evidence(&checkout)).unwrap();
    assert!(
        evidence.matches("\"reused\": true").count() > 0,
        "the imported stamps did not let the second checkout reuse anything:\n{evidence}"
    );
}

#[test]
fn a_tampered_export_is_refused_before_anything_is_written() {
    let first = scratch("tampered");
    let project = built_project(&first, "widget");
    let export = first.join("cache-export");
    let exported = invoke(&project, &["cache", "export", export.to_str().unwrap()]);
    assert!(exported.status.success(), "{}", both(&exported));
    let content = std::fs::read_dir(export.join("cas"))
        .unwrap()
        .next()
        .map(|entry| entry.unwrap().path())
        .expect("the export carries content-store entries");
    std::fs::write(&content, b"tampered").unwrap();

    let second = scratch("tampered-second");
    copy_project(&project, &second.join("widget"));
    let checkout = second.join("widget");
    let imported = invoke(&checkout, &["cache", "import", export.to_str().unwrap()]);
    assert!(!imported.status.success(), "{}", both(&imported));
    let message = both(&imported);
    assert!(
        message.contains("does not match the export manifest"),
        "{message}"
    );
    assert!(
        !checkout.join("target").join("xirang").join("cas").exists(),
        "a refused import must not write content-store bytes"
    );
}

#[test]
fn exporting_a_project_that_never_built_is_refused_by_name() {
    let root = scratch("never-built");
    let created = invoke(&root, &["new", "--lang", "cpp", "--lib", "widget"]);
    assert!(created.status.success(), "{}", both(&created));
    let project = root.join("widget");
    let exported = invoke(
        &project,
        &["cache", "export", root.join("out").to_str().unwrap()],
    );
    assert!(!exported.status.success(), "{}", both(&exported));
    let message = both(&exported);
    assert!(message.contains("no content store"), "{message}");
    assert!(message.contains("build once"), "{message}");
}
