//! Pins for the shared declaration walk: the marker filter, the source spelling in errors, and the
//! fact that the walk goes through the shared scope rather than a private one.
//! 共享声明遍历的钉子：标记过滤、错误里的源码拼写，以及"遍历走共享范围而不是私有遍历"这一点。

use std::path::PathBuf;

use super::{DeclarationSpec, Unreadable, read_package};

const SPEC: DeclarationSpec = DeclarationSpec {
    extensions: &[".toy"],
    skip_directories: &["target", ".git"],
    marker: mentions,
    unreadable: Unreadable::Refuse,
};

fn mentions(text: &str) -> bool {
    text.contains("TOY_SOIL")
}

fn package(label: &str, entries: &[(&str, &str)]) -> PathBuf {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "xirang-kit-declarations-{label}-{}-{sequence}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    for (relative, text) in entries {
        let path = root.join(relative);
        std::fs::create_dir_all(path.parent().unwrap()).expect("fixture parent");
        std::fs::write(path, text).expect("fixture file");
    }
    std::fs::create_dir_all(&root).expect("fixture root");
    root
}

/// Only files whose text carries the marker are handed to the language, and the language sees the
/// package-relative spelling, not the physical path.
/// 只有文本带标记的文件才会交给语言，而且语言看到的是包内相对拼写、不是物理路径。
#[test]
fn the_marker_selects_the_files_and_the_spelling_is_package_relative() {
    let root = package(
        "selects",
        &[
            ("native/a.toy", "TOY_SOIL\n"),
            ("native/helper.toy", "// no marker\n"),
            ("notes.txt", "TOY_SOIL\n"),
            ("target/skipped.toy", "TOY_SOIL\n"),
        ],
    );
    let seen = read_package(&root, &SPEC, |source, text| {
        Ok(vec![format!("{source}|{}", text.trim())])
    })
    .expect("the walk runs");
    assert_eq!(
        seen,
        vec!["native/a.toy|TOY_SOIL".to_owned()],
        "one marked source, spelled from the package root"
    );
    let _ = std::fs::remove_dir_all(root);
}

/// A refusal from the language is prefixed with the source spelling, so the reader knows the file.
/// 语言给出的拒绝会带上源码拼写前缀，读者因此知道是哪个文件。
#[test]
fn a_language_error_names_the_source() {
    let root = package("error", &[("native/bad.toy", "TOY_SOIL broken\n")]);
    let refused = read_package(&root, &SPEC, |_source, _text| {
        Err::<Vec<String>, String>("a declaration is missing its parent".to_owned())
    })
    .expect_err("the language refused the file");
    assert!(
        refused.starts_with("native/bad.toy: "),
        "the error names the source it came from: {refused}"
    );
    assert!(
        refused.contains("missing its parent"),
        "the language's own sentence survives: {refused}"
    );
    let _ = std::fs::remove_dir_all(root);
}
