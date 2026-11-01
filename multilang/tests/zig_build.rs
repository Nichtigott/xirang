//! The shared native driver compiles a Zig declaration through the language's compiler hook.
//! 共享原生驱动通过语言的编译器钩子，把一条 Zig 声明编译成目标文件。
//!
//! The driver used to spell `-c/-o/-I` itself, which is a cc shape; this check drives the same
//! driver with a Zig command shape and reads the record back with the shared reader. It needs a Zig
//! toolchain, so it is `#[ignore]`d and run by `tools/xirang-zig-record-check`.
//! 驱动过去自己拼 `-c/-o/-I`（cc 形状）；本检查用 Zig 的命令形状驱动同一个驱动，并用共享读取器读回记录。
//! 它需要 Zig 工具链，因此标了 `#[ignore]`，由 `tools/xirang-zig-record-check` 运行。

#![cfg(feature = "native-record")]

use std::path::{Path, PathBuf};

use xirang_multilang::external_faces::Recipe;
use xirang_multilang::languages::kit::native::compile;
use xirang_multilang::languages::zig::NATIVE;

fn fixture_root() -> PathBuf {
    std::env::temp_dir().join(format!(
        "xirang-zig-build-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ))
}

fn sdk_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("sdk/zig")
}

/// One Zig source becomes one object, with the record the native reader walks.
/// 一份 Zig 源码变成一份目标文件，里面是原生读取器所走的记录。
#[test]
#[ignore = "needs a Zig toolchain; set XIRANG_ZIG or put zig on PATH, then run --ignored"]
fn the_native_driver_compiles_a_zig_declaration_through_the_language_hook() {
    let root = fixture_root();
    std::fs::create_dir_all(&root).expect("fixture directory");
    std::fs::write(
        root.join("main.zig"),
        r#"const xirang = @import("xirang");

pub export var SOIL linksection(xirang.SECTION) = xirang.record(.{
    .kind = "Gauge",
    .name_en = "Gauge",
    .exports = "shop.gauge",
    .handle = "Gauge",
});
"#,
    )
    .expect("fixture source");
    let out = root.join("out");
    std::fs::create_dir_all(&out).expect("out directory");
    let recipe = Recipe {
        program: std::env::var("XIRANG_ZIG").unwrap_or_else(|_| "zig".to_owned()),
        arguments: Vec::new(),
        includes: Vec::new(),
        archive: "ar".to_owned(),
    };
    let objects = compile(
        &root,
        &out,
        &recipe,
        &["main.zig"],
        &[sdk_root()],
        &NATIVE,
        false,
    )
    .unwrap_or_else(|error| panic!("the shared native driver refused the Zig source: {error}"));
    assert_eq!(objects.len(), 1);
    let faces = xirang_multilang::languages::kit::record::read_faces(&objects[0])
        .expect("the object the driver produced carries a record the reader accepts");
    assert_eq!(faces.len(), 1, "one declaration, one record: {faces:#?}");
    assert_eq!(faces[0].name, "Gauge");
    assert_eq!(faces[0].exports, vec!["shop.gauge".to_owned()]);
    assert_eq!(faces[0].handle, "Gauge");
    let _ = std::fs::remove_dir_all(&root);
}

#[allow(dead_code)]
fn _quiet(_object: &Path) {}

/// A hand-written declaration names its parent through the generated identity module.
/// 手写声明经生成的恒等模块点名父级。
///
/// This is the path a hand-written Zig face takes: the build knows the root identity before compiling,
/// so the author writes `xirang_faces.ROOT_NODE_ID` instead of sixteen bytes. The record the compiler
/// lands must carry that same identity, or the face hangs under nothing at runtime.
/// 这是手写 Zig 面走的路：构建在编译前就知道根身份，因此作者写 `xirang_faces.ROOT_NODE_ID` 而不是十六字节。
/// 编译器落下的记录必须携带同一个身份，否则运行期这个面挂不到任何父级下。
#[test]
#[ignore = "needs a Zig toolchain; set XIRANG_ZIG or put zig on PATH, then run --ignored"]
fn a_generated_identity_module_carries_the_parent_into_the_record() {
    let namespace = "zig-build-test";
    let root_id = xirang_kernel::identity::root_node_id(namespace).into_bytes();
    let root = fixture_root();
    std::fs::create_dir_all(&root).expect("fixture directory");
    let out = root.join("out");
    std::fs::create_dir_all(&out).expect("out directory");
    std::fs::write(
        out.join("xirang_faces.zig"),
        xirang_multilang::languages::zig::faces_module(root_id, &[]),
    )
    .expect("generated module");
    std::fs::write(
        root.join("main.zig"),
        r#"const xirang = @import("xirang");
const faces = @import("xirang_faces");

pub export var SOIL linksection(xirang.SECTION) = xirang.record(.{
    .kind = "Gauge",
    .parent = faces.ROOT_NODE_ID,
    .name_en = "Gauge",
});
"#,
    )
    .expect("fixture source");
    let recipe = Recipe {
        program: std::env::var("XIRANG_ZIG").unwrap_or_else(|_| "zig".to_owned()),
        arguments: Vec::new(),
        includes: Vec::new(),
        archive: "ar".to_owned(),
    };
    let objects = compile(
        &root,
        &out,
        &recipe,
        &["main.zig"],
        &[sdk_root()],
        &NATIVE,
        false,
    )
    .unwrap_or_else(|error| panic!("the shared native driver refused the Zig source: {error}"));
    let faces = xirang_multilang::languages::kit::record::read_faces(&objects[0])
        .expect("the object carries a record the reader accepts");
    assert_eq!(faces.len(), 1);
    assert_eq!(
        faces[0].parent,
        xirang_kernel::identity::root_node_id(namespace),
        "the record carries the identity the generated module spelled"
    );
    let _ = std::fs::remove_dir_all(&root);
}
