#![cfg(unix)]
//! The write scope: the real location decides, and the whole set is refused together.
//! 写入作用域：由真实位置决定，且整个集合一起被拒绝。
//!
//! These pins build directory links to prove that a link out of the package is refused by its real
//! location. `std::os::unix::fs::symlink` is the only spelling of that fixture here, and Windows
//! spells links differently (`symlink_dir`/`symlink_file`, with its own privilege rules) — so the
//! file is Unix-only rather than silently asserting a Windows behaviour it never built.
//! 这些钉子用目录链接证明"指向包外的链接按真实位置被拒"。这里唯一的夹具写法是
//! `std::os::unix::fs::symlink`，而 Windows 的拼法不同（`symlink_dir`/`symlink_file`，还有自己的
//! 权限规则）——因此本文件只在 Unix 上编译，而不是悄悄断言一个它从未构建过的 Windows 行为。

use super::*;

fn unique(label: &str) -> PathBuf {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!(
        "xirang-scope-{label}-{}-{sequence}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&path);
    path
}

/// A path inside the package is inside, including one whose parents do not exist yet.
/// 包内的路径算包内，包括父目录还不存在的那一种。
#[test]
fn a_path_inside_or_not_yet_created_is_inside() {
    let root = unique("inside");
    std::fs::create_dir_all(root.join("src")).expect("fixture");
    let scope = WriteScope::new(&root).expect("scope");
    assert!(scope.inside(&root.join("src/panel.rs")).expect("probe"));
    assert!(
        scope
            .inside(&root.join("xirang/objects.tsv"))
            .expect("probe"),
        "a path whose parents do not exist is judged by its nearest existing ancestor"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// A directory link out of the package makes every path under it outside, and the refusal names the
/// spelled path and the real one.
/// 一个通向包外的目录链接使它下面每个路径都在包外，而拒绝同时点名书写的路径与真实的路径。
#[test]
fn a_directory_link_out_of_the_package_makes_its_children_outside() {
    let root = unique("root");
    let outside = unique("outside");
    std::fs::create_dir_all(root.join("vendor")).expect("fixture");
    std::fs::create_dir_all(&outside).expect("outside fixture");
    let link = root.join("vendor/ui");
    std::os::unix::fs::symlink(&outside, &link).expect("link");

    let scope = WriteScope::new(&root).expect("scope");
    let changes = vec![
        Change {
            path: root.join("src/panel.rs"),
            kind: ChangeKind::Write,
            subject: "the new face".to_owned(),
        },
        Change {
            path: link.join("gauge.cpp"),
            kind: ChangeKind::Write,
            subject: "the child declaration".to_owned(),
        },
    ];
    let refused = scope.authorize(&changes).expect_err("the link leaves");
    assert_eq!(
        refused.len(),
        1,
        "only the member under the link is outside"
    );
    assert_eq!(refused[0].subject, "the child declaration");
    assert!(
        refused[0]
            .real
            .starts_with(std::fs::canonicalize(&outside).expect("outside root"))
    );
    let text = describe_outside(&refused, &root);
    assert!(text.contains("the child declaration"), "{text}");
    assert!(text.contains("gauge.cpp"), "{text}");
    let _ = std::fs::remove_dir_all(&root);
    let _ = std::fs::remove_dir_all(&outside);
}

/// A file link out of the package is outside too: writing through it would write the target.
/// 一个通向包外的文件链接同样在包外：穿过它写就是写那个目标。
#[test]
fn a_file_link_out_of_the_package_is_outside() {
    let root = unique("file-root");
    let outside = unique("file-outside");
    std::fs::create_dir_all(&root).expect("fixture");
    std::fs::create_dir_all(&outside).expect("outside fixture");
    let target = outside.join("gauge.cpp");
    std::fs::write(&target, "// theirs").expect("fixture");
    std::os::unix::fs::symlink(&target, root.join("gauge.cpp")).expect("link");

    let scope = WriteScope::new(&root).expect("scope");
    let refused = scope
        .authorize(&[Change {
            path: root.join("gauge.cpp"),
            kind: ChangeKind::Remove,
            subject: "the child declaration".to_owned(),
        }])
        .expect_err("the file link leaves");
    assert_eq!(refused.len(), 1);
    assert!(target.exists(), "the probe changed nothing");
    let _ = std::fs::remove_dir_all(&root);
    let _ = std::fs::remove_dir_all(&outside);
}

/// Two members outside are both reported: a caller fixing one must not discover the next on retry.
/// 两个越界成员都被报出：修好一个的调用方不该在重试时才发现下一个。
#[test]
fn every_outside_member_is_reported_at_once() {
    let root = unique("many-root");
    let one = unique("many-one");
    let two = unique("many-two");
    std::fs::create_dir_all(&root).expect("fixture");
    std::fs::create_dir_all(&one).expect("fixture");
    std::fs::create_dir_all(&two).expect("fixture");
    std::os::unix::fs::symlink(&one, root.join("a")).expect("link");
    std::os::unix::fs::symlink(&two, root.join("b")).expect("link");

    let scope = WriteScope::new(&root).expect("scope");
    let refused = scope
        .authorize(&[
            Change {
                path: root.join("a/x.cpp"),
                kind: ChangeKind::Write,
                subject: "a/x.cpp".to_owned(),
            },
            Change {
                path: root.join("b/y.cpp"),
                kind: ChangeKind::Write,
                subject: "b/y.cpp".to_owned(),
            },
        ])
        .expect_err("both leave");
    assert_eq!(refused.len(), 2);
    let _ = std::fs::remove_dir_all(&root);
    let _ = std::fs::remove_dir_all(&one);
    let _ = std::fs::remove_dir_all(&two);
}
