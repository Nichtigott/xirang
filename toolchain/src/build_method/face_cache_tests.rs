//! Pins for the face disk cache: the record round-trips, and the key follows content.
//! 注册面磁盘缓存的钉子：记录可往返，且键跟着内容走。

use std::fs;

use super::{key, read, write};
use crate::build_method::FaceView;
use xirang_kernel::identity::{NodeId, hex_decode};
use xirang_multilang::source_ref::SourceBase;

fn root(label: &str) -> std::path::PathBuf {
    let root = std::env::temp_dir().join(format!(
        "xirang-faces-cache-test-{label}-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("src")).expect("fixture src");
    root
}

fn face() -> FaceView {
    FaceView {
        id: NodeId::from_raw([1; 16]),
        stable_name: Some("button".to_owned()),
        parent: NodeId::from_raw([2; 16]),
        kind: "Button".to_owned(),
        registry_name: "button".to_owned(),
        module: "root::button".to_owned(),
        source: "src/button/button.rs".to_owned(),
        source_base: SourceBase::Package,
        owns_registry: false,
        path: "root/button".to_owned(),
        parent_resolved: true,
    }
}

/// A record survives the round trip, bytes included.
/// 记录经往返后不变，连字节身份也一样。
#[test]
fn a_record_round_trips() {
    let key = key(&root("round"), "host").expect("a key");
    let answer = (vec![face()], vec!["src/bad.rs:1 broken".to_owned()], vec![]);
    write(&key, &answer);
    let read_back = read(&key).expect("the record is there");
    assert_eq!(read_back, answer);
    assert_eq!(
        read_back.0[0].id.to_string(),
        "01010101010101010101010101010101"
    );
}

/// The key changes with any file the package holds: sources, islands, native sources, the manifest.
/// 键随包内任何文件变化：源码、孤岛、原生源、清单。
#[test]
fn the_key_follows_source_content() {
    let package = root("key");
    fs::write(package.join("src/lib.rs"), "pub fn a() {}\n").expect("source");
    fs::write(package.join("README.md"), "one\n").expect("readme");
    let first = key(&package, "host").expect("a key");
    let again = key(&package, "host").expect("the same key");
    assert_eq!(first, again, "the same bytes give the same key");
    fs::write(package.join("src/lib.rs"), "pub fn b() {}\n").expect("source");
    let changed = key(&package, "host").expect("a key");
    assert_ne!(first, changed, "a changed source gives a new key");
    fs::write(package.join("README.md"), "two\n").expect("readme");
    let readme_only = key(&package, "host").expect("a key");
    assert_ne!(changed, readme_only, "the whole package enters the key");
    // An island lives outside `src/` and a foreign face lives in a native source: both change the
    // tree, so both must change the key.
    // 孤岛住在 `src/` 之外、外语面住在原生源里：两者都改变树，因此都必须改变键。
    fs::create_dir_all(package.join("components")).expect("island dir");
    fs::write(package.join("components/reading.rs"), "// island\n").expect("island");
    let island = key(&package, "host").expect("a key");
    assert_ne!(
        readme_only, island,
        "an island outside the scan root enters the key"
    );
    fs::write(package.join("area.cpp"), "// foreign face\n").expect("native");
    let native = key(&package, "host").expect("a key");
    assert_ne!(island, native, "a native source enters the key");
    assert!(
        hex_decode(&first["faces-1-".len()..]).is_some(),
        "the key is hex"
    );
}
