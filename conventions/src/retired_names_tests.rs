//! Tests for the retired-name gate (audit round B4 follow-up, t56).
//! 退役名门禁的测试（B4 轮后续，t56）。

use std::fs;
use std::path::PathBuf;

use super::{findings, reason};

/// A throwaway workspace whose members are derived from the directories written.
/// 一个一次性工作区，成员由写出的目录推导。
fn synthetic(name: &str, files: &[(&str, &str)]) -> PathBuf {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "xirang-retired-{name}-{}-{sequence}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).expect("fixture root");
    for (path, contents) in files {
        let full = root.join(path);
        fs::create_dir_all(full.parent().expect("parent")).expect("fixture dir");
        fs::write(&full, contents).expect("fixture file");
    }
    crate::fixture_manifest(&root);
    root
}

/// A retired symbol written back into a comment is reported, with the file and
/// the line, and the report carries the replacement.
/// 把已退役的符号写回注释里会被报出，带文件与行号，并且报告里带着替代者。
#[test]
fn a_retired_symbol_written_back_is_reported() {
    let root = synthetic(
        "written-back",
        &[(
            "probe/src/lib.rs",
            "//! probe\n// the applier (`apply_trait_contract`) still reads the old name\npub fn probe() {}\n",
        )],
    );
    let found = findings(&root);
    assert_eq!(found.len(), 1, "{found:#?}");
    assert_eq!(found[0].file, "probe/src/lib.rs");
    assert_eq!(found[0].line, 2);
    assert_eq!(found[0].name, "apply_trait_contract");
    assert_eq!(found[0].replacement, "apply_trait_label");
    assert!(reason(&found[0].name).is_some_and(|why| why.contains("apply_trait_label")));
    let _ = fs::remove_dir_all(&root);
}

/// A retired phrase written back is reported too, and the word boundary keeps a
/// longer word (`slot names`) out of the report.
/// 把已退役的短语写回同样会被报出，而词边界把更长的词（`slot names`）挡在报告之外。
#[test]
fn a_retired_phrase_is_reported_and_a_longer_word_is_not() {
    let root = synthetic(
        "phrase",
        &[
            (
                "probe/src/lib.rs",
                "//! probe\n// it matched logical path, `kind`, module and slot name first\npub fn probe() {}\n",
            ),
            (
                "probe/src/other.rs",
                "//! other\n// the authoring layout's slot names index the same constants\npub fn other() {}\n",
            ),
        ],
    );
    let found = findings(&root);
    assert_eq!(found.len(), 1, "{found:#?}");
    assert_eq!(found[0].file, "probe/src/lib.rs");
    assert_eq!(found[0].name, "module and slot name");
    let _ = fs::remove_dir_all(&root);
}

/// The three live concepts, the converged spelling, a record and the changelog
/// are all silent: that is the "does not cry wolf" half.
/// 三个活概念、被统一后的拼法、记录文档与 changelog 全部静默：这是"不乱叫"的那一半。
#[test]
fn the_other_concepts_records_and_the_changelog_are_not_reported() {
    let root = synthetic(
        "quiet",
        &[
            (
                "toolchain/src/plugin_host/lib.rs",
                "//! host\n/// Wrap a slot name; an empty string means no slot was declared.\npub fn host() {}\n",
            ),
            (
                "toolchain/src/studio/ui.rs",
                "//! studio\n// The authoring layout's slot names: the form, the appliers and the tests index\npub fn studio() {}\n",
            ),
            (
                "toolchain/src/runtime/face_manifest.rs",
                "//! run\n// Field dictionary and slot names shared by the Studio form and the file authoring API\n// Studio 表单与文件创作 API 共用的字段词典与槽位名\npub fn run() {}\n",
            ),
            (
                "kernel/src/lib.rs",
                "//! kernel\n/// The replacement check does not consult the slot name.\npub fn kernel() {}\n",
            ),
            (
                "README.md",
                "the tool matches logical path, kind, module, or `registry_name`, and annotates each hit\n",
            ),
            (
                "CHANGELOG.md",
                "the applier `apply_trait_contract` and the enumeration `module and slot name`\n",
            ),
            (
                "docs/audit-2026-09-28/audit-something.md",
                "`apply_trait_contract` and `module and slot name` were the old spellings\n",
            ),
        ],
    );
    let found = findings(&root);
    assert_eq!(found, Vec::new(), "{found:#?}");
    let _ = fs::remove_dir_all(&root);
}

/// The shipped tree writes none of the retired names — the no-over-fire pin that
/// makes the list trustworthy.
/// 出厂树不写任何退役名——让这份清单可信的"不误报"钉子。
#[test]
fn the_shipped_tree_writes_no_retired_name() {
    let root = crate::workspace_root();
    let found = findings(&root);
    assert!(
        found.is_empty(),
        "a retired name is still written in the live tree; use its replacement: {found:#?}"
    );
}
