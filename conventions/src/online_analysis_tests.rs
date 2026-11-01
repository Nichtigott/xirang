//! The online-analysis gate's tests: the synthetic shapes it must catch and the one it must not.
//! 在线分析门禁的测试：它必须抓到的合成形状，以及它必须放过的那一种。

use super::*;
use crate::workspace_root;
use std::fs;
use std::path::PathBuf;

/// A throwaway checkout with the given files under `toolchain/src/mcp`.
/// 一个只含给定文件的一次性检出，文件放在 `toolchain/src/mcp` 下。
fn synthetic(files: &[(&str, &str)]) -> PathBuf {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let root =
        std::env::temp_dir().join(format!("xirang-online-{}-{sequence}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    for (name, contents) in files {
        let path = root.join("toolchain/src/mcp").join(name);
        fs::create_dir_all(path.parent().expect("parent")).expect("fixture dir");
        fs::write(&path, contents).expect("fixture file");
    }
    crate::fixture_manifest(&root);
    root
}

/// A file that computes online and says why is not reported.
/// 一个在线计算并说出了原因的文件不会被报出。
#[test]
fn a_stated_reason_passes() {
    let root = synthetic(&[(
        "quiet.rs",
        "//! A reader that derives.\n//! online: the build publishes faces, not the call graph, so \
         this answer is about the sources as they are now.\n\npub fn go() { let _ = \
         load_sources(root); }\n",
    )]);
    assert!(
        findings(&root).is_empty(),
        "a stated reason passes: {:#?}",
        findings(&root)
    );
    let _ = fs::remove_dir_all(&root);
}

/// A file that computes online and says nothing is reported, with its sites.
/// 一个在线计算却什么也没说的文件会被报出，并带着它的位置。
#[test]
fn a_silent_derivation_is_reported_with_its_sites() {
    let root = synthetic(&[(
        "loud.rs",
        "//! A reader that derives silently.\n\npub fn go() { let _ = load_sources(root); }\n",
    )]);
    let found = findings(&root);
    assert_eq!(found.len(), 1, "one file, one finding: {found:#?}");
    assert!(
        found[0].file.ends_with("loud.rs") && found[0].sites.len() == 1,
        "the finding names the file and the call site: {found:#?}"
    );
    let _ = fs::remove_dir_all(&root);
}

/// A stub of an answer is not an answer.
/// 一句敷衍不算答案。
#[test]
fn a_stub_reason_is_still_reported() {
    let root = synthetic(&[(
        "stub.rs",
        "//! online: no\n\npub fn go() { let _ = load_sources(root); }\n",
    )]);
    assert_eq!(
        findings(&root).len(),
        1,
        "the floor on prose is what keeps this from passing"
    );
    let _ = fs::remove_dir_all(&root);
}

/// A test that derives is a test *about* deriving, and is out of scope.
/// 推导的测试是**关于**推导的测试，不在范围内。
#[test]
fn the_test_half_is_not_judged() {
    let root = synthetic(&[
        (
            "inline.rs",
            "//! Only its test derives.\n\n#[cfg(test)]\nmod inline_tests {\n    #[test]\n    fn t() \
             { let _ = load_sources(root); }\n}\n",
        ),
        (
            "beside_tests.rs",
            "//! A sibling test file.\n\n#[test]\nfn t() { let _ = load_sources(root); }\n",
        ),
    ]);
    assert!(
        findings(&root).is_empty(),
        "neither half of the test surface is judged: {:#?}",
        findings(&root)
    );
    let _ = fs::remove_dir_all(&root);
}

/// The walk covers the bridge it claims to cover.
/// 遍历覆盖它声称覆盖的那座桥。
#[test]
fn the_walk_covers_the_bridge() {
    let sources = rust_sources(&workspace_root().join(BRIDGE));
    assert!(
        sources.len() > 40,
        "the online-analysis walk found only {} files; it is supposed to cover {BRIDGE}",
        sources.len()
    );
}

/// Every bridge file that computes online answers the question (audit `W3-3`).
/// 每个在线计算的桥文件都回答了那个问题（审计 `W3-3`）。
#[test]
fn every_online_answer_says_why_the_build_cannot() {
    let found = findings(&workspace_root());
    assert!(
        found.is_empty(),
        "these files compute online without saying why the build cannot:\n{found:#?}"
    );
}
