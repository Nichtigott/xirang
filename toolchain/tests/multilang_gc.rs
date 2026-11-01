#![cfg(feature = "cli")]
//! End-to-end: `xirang gc` collects content-store entries no stamp references, keeps every entry
//! a stamp names, prints each removal, and refuses rather than guessing when a stamp cannot be read.
//! 端到端：`xirang gc` 回收没有印记引用的内容库条目、保留被印记点名的条目、打印每次删除，并在读不懂
//! 印记时拒绝而不是靠猜。

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn binary() -> &'static str {
    env!("CARGO_BIN_EXE_xirang")
}

fn scratch(label: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "xirang-gc-{label}-{}-{:?}",
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
        "status {}\nstdout:\n{}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

/// Scaffold and build one C++ library, so the content store holds real compile outputs.
/// 生成并构建一个 C++ 库，使内容库里是真实的编译产物。
fn built(label: &str) -> PathBuf {
    let parent = scratch(label);
    let created = invoke(&parent, &["new", "widget", "--lib", "--lang", "cpp"]);
    assert!(created.status.success(), "{}", both(&created));
    let root = parent.join("widget");
    let built = invoke(&root, &["build"]);
    assert!(built.status.success(), "{}", both(&built));
    root
}

fn entries(root: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    if let Ok(read) = std::fs::read_dir(root.join("target/xirang/cas")) {
        for entry in read.flatten() {
            if entry.path().is_file() {
                found.push(entry.path());
            }
        }
    }
    found.sort();
    found
}

fn stamp_files(root: &Path) -> Vec<PathBuf> {
    fn walk_files(directory: &Path, found: &mut Vec<PathBuf>) {
        let Ok(read) = std::fs::read_dir(directory) else {
            return;
        };
        for entry in read.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk_files(&path, found);
            } else if path
                .parent()
                .and_then(|parent| parent.file_name())
                .is_some_and(|name| name == "stamps")
                && path.extension().and_then(|extension| extension.to_str()) == Some("json")
            {
                found.push(path);
            }
        }
    }
    let mut found = Vec::new();
    walk_files(&root.join("target/xirang/build"), &mut found);
    found.sort();
    found
}

#[test]
fn a_content_entry_no_stamp_references_is_collected_and_reported() {
    let root = built("orphan");
    let before = entries(&root);
    assert!(
        !before.is_empty(),
        "the build stores its outputs in the content store"
    );
    let orphan = "ab".repeat(32);
    std::fs::write(root.join("target/xirang/cas").join(&orphan), b"orphan").unwrap();
    let dry = invoke(&root, &["gc", "--dry-run"]);
    assert!(dry.status.success(), "{}", both(&dry));
    let text = String::from_utf8_lossy(&dry.stdout).into_owned();
    assert!(text.contains("would remove 1 entries"), "{text}");
    assert!(text.contains(&orphan), "{text}");
    assert!(
        root.join("target/xirang/cas").join(&orphan).exists(),
        "a dry run removes nothing"
    );
    let collected = invoke(&root, &["gc"]);
    assert!(collected.status.success(), "{}", both(&collected));
    let text = String::from_utf8_lossy(&collected.stdout).into_owned();
    assert!(text.contains("removed 1 entries"), "{text}");
    assert!(text.contains(&orphan), "the removal is named: {text}");
    assert!(!root.join("target/xirang/cas").join(&orphan).exists());
    for path in before {
        assert!(
            path.exists(),
            "a referenced entry survives: {}",
            path.display()
        );
    }
}

#[test]
fn gc_keeps_every_entry_a_stamp_references() {
    let root = built("keeps");
    let before = entries(&root);
    assert!(!before.is_empty());
    let collected = invoke(&root, &["gc"]);
    assert!(collected.status.success(), "{}", both(&collected));
    let text = String::from_utf8_lossy(&collected.stdout).into_owned();
    assert!(text.contains("removed 0 entries"), "{text}");
    for path in before {
        assert!(path.exists(), "{}", path.display());
    }
}

#[test]
fn an_unreadable_stamp_makes_gc_refuse_instead_of_guessing() {
    let root = built("refuse");
    let stamps = stamp_files(&root);
    assert!(!stamps.is_empty(), "the build writes stamps");
    std::fs::write(&stamps[0], "{not a stamp").unwrap();
    let before = entries(&root);
    let refused = invoke(&root, &["gc"]);
    assert!(!refused.status.success(), "{}", both(&refused));
    let text = both(&refused);
    assert!(text.contains("cannot read the stamp"), "{text}");
    for path in before {
        assert!(
            path.exists(),
            "refusing must not delete anything: {}",
            path.display()
        );
    }
}

#[test]
fn gc_without_a_content_store_says_so_and_succeeds() {
    let empty = scratch("empty");
    let collected = invoke(&empty, &["gc"]);
    assert!(collected.status.success(), "{}", both(&collected));
    let text = String::from_utf8_lossy(&collected.stdout).into_owned();
    assert!(text.contains("no content store"), "{text}");
}
