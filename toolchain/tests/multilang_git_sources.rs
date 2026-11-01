#![cfg(feature = "cli")]
//! End-to-end: a `[sources]` entry may name a git repository at one revision. The first build
//! clones it once into a cache, the resolved commit lands in the lock, a different commit is a
//! different plan, and a git source without `rev` is refused by name.
//! 端到端：`[sources]` 条目可以点名一个 git 仓库的某个版本。首次构建把它克隆进缓存，解析出的提交进入
//! 锁；另一个提交是另一份计划；不带 `rev` 的 git 源按名拒绝。

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
        "xirang-git-{label}-{}-{:?}",
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

/// Run git in `directory` and return stdout, panicking with stderr on failure.
/// 在 `directory` 里跑 git 并返回 stdout；失败时带 stderr 崩溃。
fn git(directory: &Path, arguments: &[&str]) -> String {
    let output = Command::new("git")
        .arg("-C")
        .arg(directory)
        .args(arguments)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git {arguments:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).into_owned()
}

/// Create a repository whose `lib` package exports `value()` returning `value`, and return the
/// commit id.
/// 建一个仓库：其中 `lib` 包导出返回 `value` 的 `value()`，并返回提交 id。
fn repository(root: &Path, value: u32) -> String {
    let repo = root.join("team");
    std::fs::create_dir_all(repo.join("lib/src")).unwrap();
    std::fs::write(
        repo.join("lib/xirang.toml"),
        "schema = 1\n\n[package]\nname = \"lib\"\nversion = \"0.1.0\"\nkind = \"library\"\n\n\
         [language.cpp]\nstandard = \"17\"\n\n[dependencies]\n\n[[export]]\nname = \"value\"\n\
         params = []\nreturns = \"i64\"\n",
    )
    .unwrap();
    std::fs::write(
        repo.join("lib/src/lib.cpp"),
        format!("#include <cstdint>\n\nstd::int64_t value() {{ return {value}; }}\n"),
    )
    .unwrap();
    if !repo.join(".git").is_dir() {
        git(&repo, &["init", "-q"]);
    }
    git(&repo, &["add", "-A"]);
    git(
        &repo,
        &[
            "-c",
            "user.name=Test",
            "-c",
            "user.email=test@example",
            "commit",
            "-q",
            "-m",
            &format!("value {value}"),
        ],
    );
    git(&repo, &["rev-parse", "HEAD"]).trim().to_owned()
}

/// Write the host project that depends on `lib` from the git source named `team`.
/// 写依赖具名 git 源 `team` 里 `lib` 的宿主项目。
fn host(root: &Path, repo: &Path, commit: Option<&str>) {
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::create_dir_all(root.join("tests")).unwrap();
    let source = match commit {
        Some(commit) => format!(
            "team = {{ git = \"file://{}\", rev = \"{commit}\" }}\n",
            repo.display()
        ),
        None => format!("team = {{ git = \"file://{}\" }}\n", repo.display()),
    };
    std::fs::write(
        root.join("xirang.toml"),
        format!(
            "schema = 1\n\n[package]\nname = \"consumer\"\nversion = \"0.1.0\"\nkind = \"library\"\n\n\
             [language.cpp]\nstandard = \"17\"\n\n[sources]\n{source}\n[dependencies]\n\
             lib = {{ source = \"team\", package = \"lib\" }}\n"
        ),
    )
    .unwrap();
    std::fs::write(
        root.join("src/consumer.cpp"),
        "int marker() { return 1; }\n",
    )
    .unwrap();
    std::fs::write(
        root.join("tests/dep_test.cpp"),
        "#include <cassert>\n\n#include \"lib/exports.h\"\n\nint main() {\n    assert(value() == 1);\n    return 0;\n}\n",
    )
    .unwrap();
}

#[test]
fn a_git_source_is_cloned_once_and_its_commit_lands_in_the_lock() {
    let root = scratch("clone");
    let repo = root.join("team");
    let commit = repository(&root, 1);
    let host_root = root.join("host");
    host(&host_root, &repo, Some(&commit));
    let tested = invoke(&host_root, &["test"]);
    assert!(tested.status.success(), "{}", both(&tested));
    let lock = std::fs::read_to_string(host_root.join("xirang.lock")).unwrap();
    assert!(lock.contains("source = \"team@"), "{lock}");
    let caches = std::fs::read_dir(host_root.join("target/xirang/sources"))
        .unwrap()
        .count();
    assert_eq!(caches, 1);
    let again = invoke(&host_root, &["test"]);
    assert!(again.status.success(), "{}", both(&again));
    assert_eq!(
        std::fs::read_dir(host_root.join("target/xirang/sources"))
            .unwrap()
            .count(),
        1
    );
}

#[test]
fn a_new_commit_is_a_new_plan_and_locked_refuses_before_the_update() {
    let root = scratch("commit");
    let repo = root.join("team");
    let first = repository(&root, 1);
    let host_root = root.join("host");
    host(&host_root, &repo, Some(&first));
    assert!(invoke(&host_root, &["update"]).status.success());
    let second = repository(&root, 2);
    host(&host_root, &repo, Some(&second));
    let locked = invoke(&host_root, &["build", "--locked"]);
    assert!(!locked.status.success(), "{}", both(&locked));
    assert!(invoke(&host_root, &["update"]).status.success());
    let rebuilt = invoke(&host_root, &["build", "--locked"]);
    assert!(rebuilt.status.success(), "{}", both(&rebuilt));
    let lock = std::fs::read_to_string(host_root.join("xirang.lock")).unwrap();
    assert!(lock.contains(&second[..12]), "{lock}");
    assert!(!lock.contains(&first[..12]), "{lock}");
}

#[test]
fn a_git_source_without_rev_is_refused_by_name() {
    let root = scratch("unversioned");
    let repo = root.join("team");
    repository(&root, 1);
    let host_root = root.join("host");
    host(&host_root, &repo, None);
    let built = invoke(&host_root, &["build"]);
    assert!(!built.status.success(), "{}", both(&built));
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&built.stdout),
        String::from_utf8_lossy(&built.stderr)
    );
    assert!(text.contains("must name both `git` and `rev`"), "{text}");
}

#[test]
fn a_branch_reference_follows_new_commits() {
    let root = scratch("branch");
    let repo = root.join("team");
    let first = repository(&root, 1);
    // The branch the fresh repository actually has, so the test pins what it means to name one.
    let branch = git(&repo, &["rev-parse", "--abbrev-ref", "HEAD"])
        .trim()
        .to_owned();
    let host_root = root.join("host");
    host(&host_root, &repo, Some(&branch));
    assert!(invoke(&host_root, &["update"]).status.success());
    let lock = std::fs::read_to_string(host_root.join("xirang.lock")).unwrap();
    assert!(lock.contains(&first[..12]), "{lock}");
    let second = repository(&root, 2);
    let checked = invoke(&host_root, &["update", "--check"]);
    assert!(checked.status.success(), "{}", both(&checked));
    let printed = String::from_utf8_lossy(&checked.stdout).into_owned();
    assert!(!printed.contains("nothing to update"), "{printed}");
    assert!(printed.contains(&second[..12]), "{printed}");
    assert!(invoke(&host_root, &["update"]).status.success());
    let lock = std::fs::read_to_string(host_root.join("xirang.lock")).unwrap();
    assert!(lock.contains(&second[..12]), "{lock}");
    assert!(!lock.contains(&first[..12]), "{lock}");
}

#[test]
fn a_commit_hash_stays_pinned_when_the_branch_moves_on() {
    let root = scratch("pinned");
    let repo = root.join("team");
    let first = repository(&root, 1);
    let host_root = root.join("host");
    host(&host_root, &repo, Some(&first));
    assert!(invoke(&host_root, &["update"]).status.success());
    let second = repository(&root, 2);
    let checked = invoke(&host_root, &["update", "--check"]);
    assert!(checked.status.success(), "{}", both(&checked));
    let printed = String::from_utf8_lossy(&checked.stdout).into_owned();
    assert!(printed.contains("nothing to update"), "{printed}");
    let lock = std::fs::read_to_string(host_root.join("xirang.lock")).unwrap();
    assert!(lock.contains(&first[..12]), "{lock}");
    assert!(!lock.contains(&second[..12]), "{lock}");
}

#[test]
fn a_tag_reference_follows_a_forced_tag() {
    let root = scratch("tag");
    let repo = root.join("team");
    let first = repository(&root, 1);
    git(&repo, &["tag", "v1"]);
    let host_root = root.join("host");
    host(&host_root, &repo, Some("v1"));
    assert!(invoke(&host_root, &["update"]).status.success());
    let lock = std::fs::read_to_string(host_root.join("xirang.lock")).unwrap();
    assert!(lock.contains(&first[..12]), "{lock}");
    let second = repository(&root, 2);
    git(&repo, &["tag", "-f", "v1", &second]);
    assert!(invoke(&host_root, &["update"]).status.success());
    let lock = std::fs::read_to_string(host_root.join("xirang.lock")).unwrap();
    assert!(lock.contains(&second[..12]), "{lock}");
    assert!(!lock.contains(&first[..12]), "{lock}");
}
