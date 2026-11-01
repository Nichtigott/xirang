//! Scenario pins: shape drift and provisional trust must arrive together without renewing trust.
//! 场景钉子：形状漂移与暂时采信须同时到达，读取不能续期。

use super::*;

fn renew_fixture(root: &Path, anchor: &str, files: &[&str]) {
    crate::mcp::adopted::adopted(
        root,
        &json!({
            "anchor": anchor, "files": files, "certifies": "reference shape",
            "evidence": "fixture review", "verifier": "fixture maintainer",
            "reason": "explicit fixture confirmation", "apply": true, "confirm": true
        }),
    )
    .expect("append a fixture lease");
}

#[test]
fn one_comparison_joins_shape_drift_and_latest_family_leases_without_writing() {
    let root = specimen_package("joined-leases");
    let button = "src/control/object/button/button.rs";
    let slider = "src/control/object/slider/slider.rs";
    renew_fixture(&root, "root/control/button", &[button]);
    renew_fixture(&root, "root/control/slider", &[button, slider]);
    renew_fixture(&root, "root/controller/unrelated", &[slider]);
    renew_fixture(&root, "root/control/slider/nested", &[slider]);
    replace(&root, slider, "    parts: SliderParts,\n", "");
    let ledger = root.join(".xirang/adopted/entries");
    let before = std::fs::read(&ledger).expect("ledger");
    let answer = super::super::consistency(&root, &json!({"specimen":"root/control/button"}))
        .expect("comparison");
    assert!(
        answer.contains("outlier     slider: lacks `parts`"),
        "{answer}"
    );
    assert!(
        answer.contains("lease root/control/button: provisional"),
        "{answer}"
    );
    let drift = answer
        .lines()
        .find(|line| line.contains("lease root/control/slider:"))
        .unwrap();
    assert!(
        drift.contains("lapsed") && drift.contains(slider) && !drift.contains(button),
        "{drift}"
    );
    assert!(
        !answer.contains("lease root/controller/")
            && !answer.contains("lease root/control/slider/nested")
    );
    assert_eq!(
        before,
        std::fs::read(&ledger).unwrap(),
        "reading never renews"
    );

    renew_fixture(&root, "root/control/slider", &[button, slider]);
    let answer =
        super::super::consistency(&root, &json!({"specimen":"root/control/button"})).unwrap();
    assert!(
        answer.contains("lease root/control/slider: provisional"),
        "newest wins: {answer}"
    );
    assert_eq!(answer.matches("lease root/control/slider:").count(), 1);
    assert!(
        answer.contains("outlier     slider: lacks `parts`"),
        "renewing trust cannot repair shape: {answer}"
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn legacy_set_prints_do_not_claim_which_file_changed_and_missing_files_are_unknown() {
    let root = specimen_package("legacy-family");
    let files = [
        "src/control/object/button/button.rs",
        "src/control/object/slider/slider.rs",
    ];
    renew_fixture(&root, "root/control/slider", &files);
    let ledger = root.join(".xirang/adopted/entries");
    let legacy = std::fs::read_to_string(&ledger)
        .unwrap()
        .lines()
        .filter(|line| !line.starts_with('#'))
        .collect::<Vec<_>>()
        .join("\n");
    std::fs::write(&ledger, legacy).unwrap();
    replace(&root, files[1], "    parts: SliderParts,\n", "");
    let entries = crate::mcp::adopted::entries(&root).unwrap();
    let answer = crate::mcp::adopted::family_leases(&root, &entries, "root/control")
        .unwrap()
        .join("\n");
    assert!(
        answer.contains("lapsed") && answer.contains("not in the record"),
        "{answer}"
    );
    std::fs::remove_file(root.join(files[1])).unwrap();
    let answer = crate::mcp::adopted::family_leases(&root, &entries, "root/control")
        .unwrap()
        .join("\n");
    assert!(
        answer.contains("lease root/control/slider: unreadable"),
        "{answer}"
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn family_lease_budget_reports_unchecked_anchors() {
    let root = specimen_package("bounded-family");
    for index in 0..22 {
        renew_fixture(
            &root,
            &format!("root/control/child{index:02}"),
            &["src/lib.rs"],
        );
    }
    let entries = crate::mcp::adopted::entries(&root).unwrap();
    let answer = crate::mcp::adopted::family_leases(&root, &entries, "root/control")
        .unwrap()
        .join("\n");
    assert_eq!(answer.matches("  lease ").count(), 20);
    assert!(
        answer.contains("2 lease(s) omitted (not checked)"),
        "{answer}"
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[cfg(unix)]
#[test]
fn adoption_refuses_covered_files_that_escape_through_symlinks() {
    let root = specimen_package("lease-containment");
    let outside = specimen_package("lease-outside");
    std::os::unix::fs::symlink(outside.join("src/lib.rs"), root.join("escaped.rs")).unwrap();
    for apply in [false, true] {
        let result = crate::mcp::adopted::adopted(
            &root,
            &json!({
                "anchor":"root/control/button", "files":["escaped.rs"], "certifies":"fixture",
                "evidence":"fixture", "verifier":"fixture", "reason":"fixture",
                "apply":apply, "confirm":true
            }),
        );
        assert!(
            result
                .unwrap_err()
                .contains("inside the configured source root")
        );
    }
    assert!(!root.join(".xirang/adopted/entries").exists());
    assert!(crate::mcp::adopted::read_files(&root, &["escaped.rs".into()]).is_err());
    std::os::unix::fs::symlink(root.join("src/lib.rs"), root.join("inside.rs")).unwrap();
    assert!(crate::mcp::adopted::read_files(&root, &["inside.rs".into()]).is_ok());
    std::fs::remove_dir_all(root).unwrap();
    std::fs::remove_dir_all(outside).unwrap();
}

/// A01: a specimen whose covered file is gone answers with `unreadable`/`unknown` instead of
/// failing the whole comparison.
/// A01：被覆盖文件已消失的标本以 `unreadable`/`unknown` 作答，而不是让整次比对失败。
#[test]
fn a_missing_specimen_file_is_reported_not_fatal() {
    let root = specimen_package("missing-specimen");
    let button = "src/control/object/button/button.rs";
    renew_fixture(&root, "root/control/button", &[button]);
    std::fs::remove_file(root.join(button)).unwrap();
    let answer =
        crate::mcp::consistency::consistency(&root, &json!({"specimen": "root/control/button"}))
            .expect("the comparison still answers");
    assert!(
        answer.contains("unreadable") && answer.contains("unknown"),
        "{answer}"
    );
    std::fs::remove_dir_all(root).unwrap();
}
