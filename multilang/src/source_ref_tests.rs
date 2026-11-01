//! Pins for the explicit source fact: the base is data, and location has no fallback.
//! 显式源码事实的钉子：基准是数据，定位不回退。

use super::*;
use xirang_kernel::identity::NodeId;

fn fact(source: &str) -> SourceRef {
    SourceRef {
        object: NodeId::from_raw([1; 16]),
        source: source.to_owned(),
        snapshot: SourceSnapshot::Workspace,
    }
}

fn root(name: &str) -> std::path::PathBuf {
    let root = std::env::temp_dir().join(format!("xirang-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    root
}

/// A record's own base decides the standard spelling; nothing is tried on disk.
/// 记录自身的基准决定标准拼写；磁盘上不试任何东西。
#[test]
fn the_declared_base_decides_the_package_relative_spelling() {
    let package = std::path::Path::new("/package");
    let src = std::path::Path::new("/package/src");
    assert_eq!(
        package_relative(SourceBase::Identity, package, src, "shop/panel/panel.rs"),
        "src/shop/panel/panel.rs"
    );
    assert_eq!(
        package_relative(SourceBase::Package, package, src, "./native/area/area.cpp"),
        "native/area/area.cpp"
    );
    // A layout whose scan root is outside `src` sets the identity base to the package root, so the
    // identity spelling is already package-relative.
    // 扫描根在 `src` 之外的布局把身份基准设为包根，因此身份拼写已经是包内相对。
    assert_eq!(
        package_relative(SourceBase::Identity, package, package, "support/lib/lib.rs"),
        "support/lib/lib.rs"
    );
}

/// The named location is the one read, even when another file of the same name exists.
/// 点名的那一处才是被读取的那一处，即使别处存在同名文件。
#[test]
fn only_the_named_source_is_read() {
    let root = root("source-ref");
    std::fs::create_dir_all(root.join("src/native/area")).unwrap();
    std::fs::create_dir_all(root.join("native/area")).unwrap();
    std::fs::write(root.join("src/native/area/area.cpp"), "wrong\n").unwrap();
    std::fs::write(root.join("native/area/area.cpp"), "right\n").unwrap();
    let located = locate(&root, &fact("native/area/area.cpp")).unwrap();
    assert_eq!(std::fs::read_to_string(located).unwrap(), "right\n");
}

/// The named location missing is an error, even when a same-named file exists elsewhere.
/// 点名位置缺失是错误，即使别处存在同名文件。
#[test]
fn a_missing_named_source_never_falls_back() {
    let root = root("source-miss");
    std::fs::create_dir_all(root.join("native/area")).unwrap();
    std::fs::write(root.join("native/area/area.cpp"), "elsewhere\n").unwrap();
    let error = locate(&root, &fact("src/native/area/area.cpp")).unwrap_err();
    assert!(error.contains("no other location is tried"), "{error}");
}

/// A record cannot name a file outside the package it claims to come from.
/// 记录不能点名它所声称的包之外的文件。
#[test]
fn a_source_cannot_leave_the_snapshot() {
    let root = root("source-escape");
    std::fs::write(root.join("inside.rs"), "face\n").unwrap();
    for source in ["../outside.rs", "/etc/passwd"] {
        let error = locate(&root, &fact(source)).unwrap_err();
        assert!(
            error.contains("not a package-relative source path"),
            "`{source}` was joined instead of refused: {error}"
        );
    }
}

/// The reproduced defect, as a pin: `External { root }` is read from **that** root even when the
/// caller's root has a file of the same name.
/// 被复现的缺陷，作为钉子：`External { root }` 从**那个**根读取，即使调用方的根里有同名文件。
#[test]
fn an_external_snapshot_is_read_from_its_own_root() {
    let local = root("snapshot-local");
    let fixed = root("snapshot-fixed");
    std::fs::write(local.join("face.cpp"), "workspace\n").unwrap();
    std::fs::write(fixed.join("face.cpp"), "external fixed snapshot\n").unwrap();
    let external = SourceRef {
        object: NodeId::from_raw([2; 16]),
        source: "face.cpp".to_owned(),
        snapshot: SourceSnapshot::External {
            root: fixed.clone(),
        },
    };
    let located = locate(&local, &external).expect("the snapshot root is read");
    assert_eq!(
        std::fs::read_to_string(located).unwrap(),
        "external fixed snapshot\n",
        "the caller's root must not answer for a fixed snapshot"
    );
    let _ = std::fs::remove_dir_all(local);
    let _ = std::fs::remove_dir_all(fixed);
}

/// A snapshot whose file is missing is an error that names the snapshot, and it does not fall back to
/// the caller's root.
/// 快照里的文件缺失时，错误点名该快照，且不回退到调用方的根。
#[test]
fn a_missing_external_snapshot_names_the_snapshot_and_does_not_fall_back() {
    let local = root("snapshot-local-miss");
    let fixed = root("snapshot-fixed-miss");
    std::fs::write(local.join("face.cpp"), "workspace\n").unwrap();
    let external = SourceRef {
        object: NodeId::from_raw([3; 16]),
        source: "face.cpp".to_owned(),
        snapshot: SourceSnapshot::External {
            root: fixed.clone(),
        },
    };
    let error = locate(&local, &external).expect_err("the snapshot has no such file");
    assert!(error.contains("snapshot at"), "{error}");
    assert!(error.contains("no other location is tried"), "{error}");
    let _ = std::fs::remove_dir_all(local);
    let _ = std::fs::remove_dir_all(fixed);
}

/// A link inside the root that points outside it is refused, because the resolved location is not in
/// the snapshot; a link that stays inside is followed.
/// 根内指向根外的链接被拒绝，因为解析后的位置不在快照里；留在根内的链接会被跟随。
#[test]
fn a_link_out_of_the_snapshot_is_refused_and_one_inside_is_followed() {
    let package = root("snapshot-link");
    let outside = root("snapshot-link-outside");
    std::fs::write(outside.join("face.cpp"), "outside\n").unwrap();
    std::fs::write(package.join("inside.cpp"), "inside\n").unwrap();
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(outside.join("face.cpp"), package.join("escapes.cpp")).unwrap();
        std::os::unix::fs::symlink(package.join("inside.cpp"), package.join("stays.cpp")).unwrap();
        let error =
            locate(&package, &fact("escapes.cpp")).expect_err("an escaping link is refused");
        assert!(error.contains("outside"), "{error}");
        assert!(error.contains("not declared"), "{error}");
        let inside =
            locate(&package, &fact("stays.cpp")).expect("a link inside the root is followed");
        assert_eq!(std::fs::read_to_string(inside).unwrap(), "inside\n");
    }
    let _ = std::fs::remove_dir_all(package);
    let _ = std::fs::remove_dir_all(outside);
}
