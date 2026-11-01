//! The Zig SDK writes the same record the native reader walks.
//! Zig SDK 写出的记录与原生读取器所走的是同一种记录。
//!
//! The check needs a Zig toolchain, so it is `#[ignore]`d: `tools/xirang-zig-record-check` runs it
//! with `--ignored`. It is not part of the ten faces for the same reason the C++ build path is not:
//! a machine without the compiler should report "not run", not "failed".
//! 这项检查需要 Zig 工具链，因此标了 `#[ignore]`：`tools/xirang-zig-record-check` 会带 `--ignored` 跑它。
//! 它不进十面，理由与 C++ 构建路径相同：没有编译器的机器应报"未跑"，而不是"失败"。

#![cfg(feature = "native-record")]

use std::path::PathBuf;
use std::process::Command;

fn fixture_root() -> PathBuf {
    std::env::temp_dir().join(format!(
        "xirang-zig-record-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ))
}

/// One Zig declaration compiles into `.xirang.faces` and reads back field for field.
/// 一条 Zig 声明编译进 `.xirang.faces`，并逐字段被读回。
#[test]
#[ignore = "needs a Zig toolchain; set XIRANG_ZIG or put zig on PATH, then run --ignored"]
fn a_zig_declaration_is_read_back_by_the_record_decoder() {
    let zig = std::env::var_os("XIRANG_ZIG").unwrap_or_else(|| "zig".into());
    let sdk = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("sdk/zig/xirang/soil.zig");
    assert!(sdk.is_file(), "the Zig SDK ships at {}", sdk.display());

    let root = fixture_root();
    std::fs::create_dir_all(root.join("xirang")).expect("fixture directory");
    std::fs::copy(&sdk, root.join("xirang/soil.zig")).expect("SDK copy");
    std::fs::write(
        root.join("gauge.zig"),
        r#"const xirang = @import("xirang/soil.zig");

pub export var SOIL linksection(xirang.SECTION) = xirang.record(.{
    .kind = "Gauge",
    .name_zh = "仪表",
    .name_en = "Gauge",
    .summary_zh = "一个 Zig 声明的面",
    .summary_en = "a face declared in Zig",
    .exports = "shop.gauge",
    .params = "GaugeParams",
    .handle = "Gauge",
});
"#,
    )
    .expect("fixture source");

    // Zig keeps a global cache under `$HOME`; a test host may have it read-only, so both caches
    // live inside the fixture. A test that cannot write its own cache reports a build failure that
    // has nothing to do with the record.
    // Zig 在 `$HOME` 下留全局缓存；测试宿主可能把它设成只读，因此两个缓存都放进夹具目录。一个写不了
    // 自己缓存的测试会报一次与记录无关的构建失败。
    let output = Command::new(&zig)
        .env("ZIG_GLOBAL_CACHE_DIR", root.join("zig-global-cache"))
        .env("ZIG_LOCAL_CACHE_DIR", root.join("zig-local-cache"))
        .arg("build-obj")
        .arg("gauge.zig")
        .arg("-femit-bin=gauge.o")
        .current_dir(&root)
        .output()
        .unwrap_or_else(|error| panic!("cannot run `{}`: {error}", zig.to_string_lossy()));
    assert!(
        output.status.success(),
        "zig build-obj failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let faces = xirang_multilang::languages::kit::record::read_faces(&root.join("gauge.o"))
        .expect("the record the Zig SDK wrote is one this reader accepts");
    assert_eq!(faces.len(), 1, "one declaration, one record: {faces:#?}");
    let face = &faces[0];
    assert_eq!(face.name, "Gauge");
    assert_eq!(face.name_zh, "仪表");
    assert_eq!(face.name_en, "Gauge");
    assert_eq!(face.summary_en, "a face declared in Zig");
    assert_eq!(face.exports, vec!["shop.gauge".to_owned()]);
    assert_eq!(face.params, "GaugeParams");
    assert_eq!(face.handle, "Gauge");
    assert!(!face.owns_registry);
    let _ = std::fs::remove_dir_all(&root);
}
