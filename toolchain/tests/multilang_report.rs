#![cfg(feature = "cli")]
//! End-to-end: `xirang report` reads the evidence one build wrote and summarizes its economics —
//! how many actions executed, reused, or were restored from the content store — and refuses when
//! there is no build to report on.
//! 端到端：`xirang report` 读一次构建写下的证据，汇总它的经济性——多少动作真执行、复用或从内容库恢复
//! ——没有可报告的一次构建时按名拒绝。

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

/// Scaffold a C++ library and build it once.
/// 搭一个 C++ 库并构建一次。
fn built(label: &str) -> (PathBuf, PathBuf) {
    let scratch = scratch(label);
    let created = invoke(&scratch, &["new", "--lang", "cpp", "--lib", "widget"]);
    assert!(created.status.success(), "{}", both(&created));
    let project = scratch.join("widget");
    let first = invoke(&project, &["build"]);
    assert!(first.status.success(), "{}", both(&first));
    (scratch, project)
}

/// The most recently written build directory.
/// 最近写入的构建目录。
fn newest_plan(project: &Path) -> PathBuf {
    let build = project.join("target/xirang/build");
    let mut newest: Option<(std::time::SystemTime, PathBuf)> = None;
    for entry in std::fs::read_dir(&build).unwrap() {
        let path = entry.unwrap().path();
        let Ok(metadata) = std::fs::metadata(path.join("evidence.json")) else {
            continue;
        };
        let modified = metadata.modified().unwrap();
        if newest.as_ref().is_none_or(|(at, _)| modified > *at) {
            newest = Some((modified, path));
        }
    }
    newest.expect("no build directory").1
}

/// The report text of a project, asserting the command succeeded.
/// 一个项目的报告文本，并断言命令成功。
fn reported(project: &Path) -> String {
    let output = invoke(project, &["report"]);
    assert!(output.status.success(), "{}", both(&output));
    String::from_utf8_lossy(&output.stdout).into_owned()
}

#[test]
fn a_cold_build_reports_every_action_as_executed() {
    let (_, project) = built("report-cold");
    let text = reported(&project);
    assert!(text.contains("result passed"), "{text}");
    assert!(text.contains("0 reused"), "{text}");
    assert!(text.contains("0 restored"), "{text}");
    assert!(!text.contains(" 0 executed"), "{text}");
}

#[test]
fn a_warm_build_reports_every_action_as_reused() {
    let (_, project) = built("report-warm");
    let second = invoke(&project, &["build"]);
    assert!(second.status.success(), "{}", both(&second));
    let text = reported(&project);
    assert!(text.contains("0 executed"), "{text}");
    assert!(!text.contains(" 0 reused"), "{text}");
    assert!(text.contains("result passed"), "{text}");
}

#[test]
fn a_build_that_restores_outputs_reports_them_as_restored() {
    let (_, project) = built("report-restored");
    let plan = newest_plan(&project);
    for directory in ["lib", "obj"] {
        let _ = std::fs::remove_dir_all(plan.join(directory));
    }
    let again = invoke(&project, &["build"]);
    assert!(again.status.success(), "{}", both(&again));
    let text = reported(&project);
    assert!(!text.contains(" 0 restored"), "{text}");
}

#[test]
fn report_refuses_when_there_is_no_build_or_no_such_plan() {
    let scratch = scratch("report-empty");
    let empty = invoke(&scratch, &["report"]);
    assert!(!empty.status.success(), "{}", both(&empty));
    assert!(
        String::from_utf8_lossy(&empty.stderr).contains("run `xirang build` first"),
        "{}",
        both(&empty)
    );
    let (_, project) = built("report-missing-plan");
    let missing = invoke(&project, &["report", "--plan", "nope"]);
    assert!(!missing.status.success(), "{}", both(&missing));
    assert!(
        String::from_utf8_lossy(&missing.stderr).contains("no build evidence for plan `nope`"),
        "{}",
        both(&missing)
    );
}

#[test]
fn a_compile_failure_is_counted_as_a_failed_action_and_named() {
    let (_scratch, project) = built("counter");
    std::fs::write(project.join("src/widget.cpp"), "this is not C++\n").unwrap();
    let failed = invoke(&project, &["build"]);
    assert!(!failed.status.success(), "{}", both(&failed));
    let report = invoke(&project, &["report"]);
    assert!(report.status.success(), "{}", both(&report));
    let text = String::from_utf8_lossy(&report.stdout).into_owned();
    assert!(text.contains("result failed"), "{text}");
    assert!(text.contains("failed action:"), "{text}");
    assert!(text.contains("compile"), "{text}");
}
