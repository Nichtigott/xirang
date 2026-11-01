//! Pins for materializing generated packages into a workspace's member list (audit `M7`, P3.5).
//! 把生成的包物化进工作区成员清单的钉子（审计 `M7`，P3.5）。

use std::fs;
use std::path::PathBuf;

use super::{entries, merged, stripped};

/// A throwaway directory with one file in it.
/// 一个一次性目录，里面放一个文件。
fn scratch(label: &str, manifest: &str) -> PathBuf {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let root = std::env::temp_dir()
        .join("xirang-scratch")
        .join(module_path!().replace("::", "-"))
        .join(format!(
            "xirang-members-{label}-{}-{sequence}",
            std::process::id()
        ));
    let _ = std::fs::remove_dir_all(&root);
    fs::create_dir_all(&root).expect("the fixture directory");
    fs::write(root.join("Cargo.toml"), manifest).expect("the workspace manifest");
    root
}

/// Read the manifest, merge these entries into it, and write it back — what the write path does.
/// 读清单、把这些条目合并进去、再写回来——也就是写入路径所做的事。
fn add(root: &std::path::Path, entries: &[String]) -> Result<bool, String> {
    let path = root.join("Cargo.toml");
    let text = fs::read_to_string(&path).expect("the workspace manifest");
    match merged(&text, entries)? {
        Some(merged) => {
            fs::write(&path, merged).expect("the merged manifest");
            Ok(true)
        }
        None => Ok(false),
    }
}

/// Read the manifest, take these entries out of it, and write it back.
/// 读清单、把这些条目从中拿掉、再写回来。
fn remove(root: &std::path::Path, entries: &[String]) -> Result<bool, String> {
    let path = root.join("Cargo.toml");
    let text = fs::read_to_string(&path).expect("the workspace manifest");
    match stripped(&text, entries)? {
        Some(stripped) => {
            fs::write(&path, stripped).expect("the stripped manifest");
            Ok(true)
        }
        None => Ok(false),
    }
}

/// Read the manifest back.
/// 把清单读回来。
fn manifest(root: &std::path::Path) -> String {
    fs::read_to_string(root.join("Cargo.toml")).expect("the workspace manifest")
}

/// The entries a host at `<root>/app` with two generated siblings would write.
/// 一个位于 `<root>/app` 的宿主、带两个生成兄弟包时会写下的那些条目。
fn siblings() -> Vec<String> {
    vec![
        "\"../app-facade\"".to_owned(),
        "\"../app-widgets\"".to_owned(),
    ]
}

/// A single-line list gains the packages, and every other line is left alone.
/// 单行清单加上这些包，其余每一行原样保留。
#[test]
fn a_single_line_list_gains_the_packages() {
    let root = scratch(
        "single-line",
        "[workspace]\nresolver = \"3\"\nmembers = [\"app\", \"fast-widget\"]\n",
    );
    assert!(add(&root, &siblings()).expect("merged"));
    assert_eq!(
        manifest(&root),
        "[workspace]\nresolver = \"3\"\nmembers = [\"app\", \"fast-widget\", \"../app-facade\", \"../app-widgets\"]\n"
    );
    // Idempotent: the second run finds every entry and writes nothing.
    // 幂等：第二遍发现每一条都在，什么都不写。
    assert!(!add(&root, &siblings()).expect("nothing missing"));
    let _ = std::fs::remove_dir_all(&root);
}

/// A list written one item per line keeps that shape, and the closing bracket stays where it was.
/// 一行一个条目的清单保持原有形状，结束括号留在原处。
#[test]
fn a_multi_line_list_keeps_its_shape() {
    let root = scratch(
        "multi-line",
        "[workspace]\nmembers = [\n    \"app\",\n    \"fast-widget\",\n]\n\n[workspace.package]\nversion = \"0.1.0\"\n",
    );
    assert!(add(&root, &siblings()).expect("merged"));
    assert_eq!(
        manifest(&root),
        "[workspace]\nmembers = [\n    \"app\",\n    \"fast-widget\",\n    \"../app-facade\",\n    \"../app-widgets\",\n]\n\n[workspace.package]\nversion = \"0.1.0\"\n"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// A workspace whose root is a package has no list, and gets one — right under its header.
/// 根就是一个包的工作区没有清单，于是得到一份——就在它的表头下面。
#[test]
fn a_workspace_without_a_list_gets_one() {
    let root = scratch(
        "no-list",
        "[workspace]\n\n[package]\nname = \"app\"\n\n[dependencies]\n",
    );
    assert!(add(&root, &siblings()).expect("merged"));
    assert_eq!(
        manifest(&root),
        "[workspace]\nmembers = [\"../app-facade\", \"../app-widgets\"]\n\n[package]\nname = \"app\"\n\n[dependencies]\n"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// Taking the packages back restores the list this action found — and takes the key with it when the
/// key itself was what it added.
/// 把包收回来会恢复本动作看到的清单——而当那份清单本身就是它加的时候，键也一起消失。
#[test]
fn taking_the_packages_back_restores_what_was_there() {
    let single = scratch("restore-single", "[workspace]\nmembers = [\"app\"]\n");
    assert!(add(&single, &siblings()).expect("merged"));
    assert!(remove(&single, &siblings()).expect("removed"));
    assert_eq!(manifest(&single), "[workspace]\nmembers = [\"app\"]\n");
    assert!(!remove(&single, &siblings()).expect("nothing left to remove"));
    let _ = std::fs::remove_dir_all(&single);

    let multi = scratch(
        "restore-multi",
        "[workspace]\nmembers = [\n    \"app\",\n]\n",
    );
    assert!(add(&multi, &siblings()).expect("merged"));
    assert!(remove(&multi, &siblings()).expect("removed"));
    assert_eq!(
        manifest(&multi),
        "[workspace]\nmembers = [\n    \"app\",\n]\n"
    );
    let _ = std::fs::remove_dir_all(&multi);

    let added = scratch(
        "restore-added",
        "[workspace]\n\n[package]\nname = \"app\"\n",
    );
    assert!(add(&added, &siblings()).expect("merged"));
    assert!(remove(&added, &siblings()).expect("removed"));
    assert_eq!(
        manifest(&added),
        "[workspace]\n\n[package]\nname = \"app\"\n"
    );
    let _ = std::fs::remove_dir_all(&added);
}

/// Another crate's entries are not this action's to remove, and a glob stays a glob.
/// 别的 crate 的条目不是本动作该移除的，而 glob 仍然是 glob。
#[test]
fn only_this_actions_entries_are_taken_back() {
    let root = scratch(
        "not-ours",
        "[workspace]\nmembers = [\"crates/*\", \"app\"]\n",
    );
    assert!(add(&root, &siblings()).expect("merged"));
    assert!(remove(&root, &siblings()).expect("removed"));
    assert_eq!(
        manifest(&root),
        "[workspace]\nmembers = [\"crates/*\", \"app\"]\n"
    );
    // A spelling the list does not carry changes nothing either: matching is on the exact token.
    // 清单里没有的拼写同样什么都不改：匹配是按逐字节的条目做的。
    assert!(!remove(&root, &["\"../app-facade\"".to_owned()]).expect("not listed"));
    let _ = std::fs::remove_dir_all(&root);
}

/// A `members` that is not an array of quoted strings is refused, naming the line to fix.
/// 不是一串带引号字符串的 `members` 会被拒绝，并点名该修的那一行。
#[test]
fn a_members_this_action_cannot_read_is_refused() {
    for (label, text) in [
        ("string", "[workspace]\nmembers = \"app\"\n"),
        ("unclosed", "[workspace]\nmembers = [\n    \"app\",\n"),
        ("bare", "[workspace]\nmembers = [app]\n"),
    ] {
        let root = scratch(label, text);
        let refused = add(&root, &siblings()).expect_err("refused");
        assert!(
            refused.contains("not an array of quoted strings") && refused.contains("Cargo.toml:2"),
            "the refusal names the shape and the line: {refused}"
        );
        assert_eq!(manifest(&root), text, "a refusal writes nothing");
        let _ = std::fs::remove_dir_all(&root);
    }
}

/// The spellings are workspace-relative with `/` separators, sorted and deduplicated.
/// 拼写相对工作区根、用 `/` 分隔，并且排好序、去过重。
#[test]
fn entries_are_relative_sorted_and_deduplicated() {
    let root = std::path::Path::new("/tmp/ws");
    let written = entries(
        root,
        &[
            &PathBuf::from("/tmp/ws/app-widgets"),
            &PathBuf::from("/tmp/ws/app-facade"),
            &PathBuf::from("/tmp/ws/app-widgets"),
        ],
    );
    assert_eq!(
        written,
        vec!["\"app-facade\"".to_owned(), "\"app-widgets\"".to_owned()]
    );
}
