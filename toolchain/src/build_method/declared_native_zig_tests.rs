//! Pins for the Zig native reader: a package that declares `language = "zig"` is read by the Zig
//! scanner, and its root face resolves to the registry root identity.
//! Zig 原生读取器的钉子：声明 `language = "zig"` 的包由 Zig 扫描器读取，它的根面解析到注册树根身份。

use std::path::PathBuf;

fn fixture(label: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "xirang-zig-native-{label}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    std::fs::create_dir_all(root.join("src/native/gauge")).expect("fixture directory");
    root
}

fn write_fixture(path: &std::path::Path, text: &str) {
    std::fs::create_dir_all(path.parent().expect("parent")).expect("directory");
    std::fs::write(path, text).expect("fixture file");
}

/// The Zig reader finds the declaration and its parent spelling.
/// Zig 读取器找到声明与它的父级拼法。
#[test]
fn a_zig_package_is_read_by_the_zig_scanner() {
    let root = fixture("read");
    write_fixture(
        &root.join("xirang.toml"),
        "[native]\nlanguage = \"zig\"\nsources = [\"src/native/gauge/gauge.zig\"]\n",
    );
    write_fixture(
        &root.join("src/native/gauge/gauge.zig"),
        "const xirang = @import(\"xirang\");\nconst faces = @import(\"xirang_faces\");\n\n\
         pub export var SOIL linksection(xirang.SECTION) = xirang.record(.{\n    .kind = \"Gauge\",\n    \
         .parent = faces.ROOT_NODE_ID,\n});\n",
    );
    let (_, faces) = super::read(&root)
        .expect("the Zig reader accepts the package")
        .expect("the package declares a native face");
    assert_eq!(faces.len(), 1);
    assert_eq!(faces[0].name, "Gauge");
    assert_eq!(faces[0].parent, "ROOT_NODE_ID");
    assert!(!faces[0].owns_registry);
    let _ = std::fs::remove_dir_all(root);
}

/// A root Zig face resolves to the root identity, and its own identity is derived from the source.
/// 根 Zig 面解析到根身份，它自己的身份由源码推导。
#[test]
fn a_root_zig_face_resolves_to_the_root_identity() {
    let root = fixture("resolve");
    write_fixture(
        &root.join("xirang.toml"),
        "[native]\nlanguage = \"zig\"\nsources = [\"src/native/gauge/gauge.zig\"]\n",
    );
    write_fixture(
        &root.join("src/native/gauge/gauge.zig"),
        "const xirang = @import(\"xirang\");\nconst faces = @import(\"xirang_faces\");\n\n\
         pub export var SOIL linksection(xirang.SECTION) = xirang.record(.{\n    .kind = \"Gauge\",\n    \
         .parent = faces.ROOT_NODE_ID,\n});\n",
    );
    let (_, faces) = super::read(&root).expect("read").expect("faces");
    let resolved = super::resolve("zig-namespace", &faces, &std::collections::BTreeMap::new())
        .expect("the root parent resolves");
    assert_eq!(resolved.len(), 1);
    assert_eq!(
        resolved[0].parent,
        xirang_kernel::identity::root_node_id("zig-namespace")
    );
    assert_eq!(resolved[0].name, "Gauge");
    let _ = std::fs::remove_dir_all(root);
}

/// A Zig face under a Rust parent travels with that parent when its subtree is partitioned.
/// Zig 面挂在 Rust 父面下时，父的子树被分区后它跟着走。
///
/// The parent spelling is the generated **module constant** (`panel`), which is exactly the claim's
/// flat spelling — before this pin, ownership judged only prefixes and handed the Zig face to the host,
/// so the fragment never compiled it.
/// 父拼法是生成的**模块常量**（`panel`），正是认领的扁平拼法本身——在本钉子之前，归属只看前缀，会把
/// Zig 面判给宿主，于是碎片永远编不到它。
#[test]
fn a_zig_face_follows_the_parent_it_hangs_under() {
    let face = super::DeclaredFace {
        source: "native/gauge/gauge.zig".to_owned(),
        parent: "panel".to_owned(),
        name: "Gauge".to_owned(),
        owns_registry: true,
    };
    let fragment = super::owned(vec![face.clone()], Some("panel"), false, &[]);
    assert_eq!(
        fragment.len(),
        1,
        "the fragment compiles the face under its claim"
    );
    let host = super::owned(vec![face.clone()], None, false, &["panel".to_owned()]);
    assert!(host.is_empty(), "the host handed the claim away");
    let outside = super::owned(vec![face], Some("other"), false, &[]);
    assert!(outside.is_empty(), "a different claim does not own it");
}
