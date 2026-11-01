//! Pins for the source-instance grid: the ledger's coordinate, and what a changed set means.
//! 源码实例网格的钉子：台账的坐标，以及变化的集合意味着什么。

use serde_json::{Value, json};
use std::path::{Path, PathBuf};

/// A throwaway package root with a ledger naming two instances of one object.
/// 一个一次性包根，台账点名同一个对象的两个实例。
fn package(label: &str) -> PathBuf {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "mcp-task-sources-{label}-{}-{sequence}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("src/provider")).expect("package directory");
    std::fs::write(root.join("src/provider/provider.rs"), "// one\n").expect("first instance");
    std::fs::write(root.join("src/provider/provider_alt.rs"), "// two\n").expect("second instance");
    let line = two(&format!(
        "{}@src/provider/provider.rs,{}@src/provider/provider_alt.rs",
        current_id(),
        second_id()
    ));
    ledger(&root, &line);
    root
}

fn stable() -> &'static str {
    "00112233445566778899aabbccddeeff"
}

fn current_id() -> &'static str {
    "00000000000000000000000000000001"
}

fn second_id() -> &'static str {
    "00000000000000000000000000000002"
}

fn ledger(root: &Path, body: &str) {
    std::fs::write(
        root.join("xirang.objects.tsv"),
        format!("# xirang objects v1\n{body}\n"),
    )
    .expect("ledger");
}

/// The two-instance line, with the sources and ids the fixture uses.
/// 两实例的那一行，用夹具的源码与 id。
fn two(line: &str) -> String {
    format!(
        "{}\troot/provider\tsrc/provider/provider.rs\t-\t{}",
        stable(),
        line
    )
}

fn digest(root: &Path, source: &str) -> String {
    xirang_kernel::sha256_hex(&std::fs::read(root.join(source)).expect("the file"))
}

/// A record that bound whatever the ledger said at open.
/// 一份按台账开启时所说内容绑定的记录。
fn record(instances: Value) -> Value {
    json!({"object": {"path": "root/provider", "stable": stable(), "instances": instances}})
}

#[test]
fn object_instances_reads_the_ledgers_package_relative_coordinate() {
    let root = package("coordinate");
    let instances = super::object_instances(&root, stable()).expect("the ledger reads");
    assert_eq!(
        instances
            .iter()
            .map(|(source, _)| source.clone())
            .collect::<Vec<_>>(),
        vec![
            "src/provider/provider.rs".to_owned(),
            "src/provider/provider_alt.rs".to_owned()
        ],
        "the ledger's column is package-relative, not a view's record spelling"
    );
    assert_eq!(
        instances[0].1.as_deref(),
        Some(digest(&root, "src/provider/provider.rs").as_str())
    );
    // A file that is gone keeps its place with no digest: history is not a live instance.
    // 已消失的文件保留位置但没有摘要：历史不是存活实例。
    std::fs::remove_file(root.join("src/provider/provider_alt.rs")).expect("the file goes");
    let after = super::object_instances(&root, stable()).expect("the ledger still reads");
    assert_eq!(after[1].0, "src/provider/provider_alt.rs");
    assert_eq!(after[1].1, None, "a missing file is history, not a digest");
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn a_changed_second_instance_is_the_objects_change() {
    let root = package("changed");
    let recorded = record(json!([
        {"source": "src/provider/provider.rs", "digest": digest(&root, "src/provider/provider.rs")},
        {"source": "src/provider/provider_alt.rs", "digest": digest(&root, "src/provider/provider_alt.rs")},
    ]));
    assert!(
        super::applied_instances(&root, &recorded, stable())
            .expect("the comparison runs")
            .is_none(),
        "an unchanged set has nothing to report"
    );
    std::fs::write(
        root.join("src/provider/provider_alt.rs"),
        "// two, edited\n",
    )
    .expect("edit");
    let finding = super::applied_instances(&root, &recorded, stable())
        .expect("the comparison runs")
        .expect("the change is a finding");
    let line = finding.detail();
    assert!(
        line.contains("src/provider/provider_alt.rs")
            && line.contains("changed after this task recorded it"),
        "the finding names the second instance: {line}"
    );
    assert_eq!(
        finding.kind(),
        xirang_kernel::maintenance::TaskObligationKind::Applied
    );
    // Restoring the bytes restores the obligation — the finding is about the bytes.
    // 恢复字节即恢复义务——这个发现是关于字节的。
    std::fs::write(root.join("src/provider/provider_alt.rs"), "// two\n").expect("restore");
    assert!(
        super::applied_instances(&root, &recorded, stable())
            .expect("the comparison runs")
            .is_none()
    );
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn a_new_live_instance_and_a_gone_one_are_both_findings() {
    let root = package("setchanges");
    let recorded = record(json!([
        {"source": "src/provider/provider.rs", "digest": digest(&root, "src/provider/provider.rs")},
        {"source": "src/provider/provider_alt.rs", "digest": digest(&root, "src/provider/provider_alt.rs")},
    ]));
    // A third live source appears: the object's source set grew.
    // 出现第三个存活源码：对象的源码集合变大了。
    std::fs::write(root.join("src/provider/provider_third.rs"), "// three\n").expect("third");
    ledger(
        &root,
        &two(&format!(
            "{}@src/provider/provider.rs,{}@src/provider/provider_alt.rs,{}@src/provider/provider_third.rs",
            current_id(),
            second_id(),
            "00000000000000000000000000000003"
        )),
    );
    let grown = super::applied_instances(&root, &recorded, stable())
        .expect("the comparison runs")
        .expect("the new instance is a finding");
    assert!(
        grown.detail().contains("a new source instance appeared")
            && grown.detail().contains("provider_third.rs"),
        "the new live instance is named: {}",
        grown.detail()
    );
    // A recorded instance whose file is gone is a finding too.
    // 记录过的实例文件消失，同样是一个发现。
    std::fs::remove_file(root.join("src/provider/provider_alt.rs")).expect("the file goes");
    let gone = super::applied_instances(&root, &recorded, stable())
        .expect("the comparison runs")
        .expect("the gone instance is a finding");
    assert!(
        gone.detail()
            .contains("is gone from disk since this task recorded it"),
        "the gone instance is named: {}",
        gone.detail()
    );
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn a_historical_instance_is_not_a_finding_and_an_old_record_still_means_one_source() {
    let root = package("historical");
    // One live instance only: this pin is about the null-digest row, not about a live set change.
    // 只有一个存活实例：这条钉子谈的是 null 摘要那一行，不是存活集合的变化。
    let single = two(&format!("{}@src/provider/provider.rs", current_id()));
    ledger(&root, &single);
    // A row the record carries with no digest is history: it never blocks closure.
    // 记录里没有摘要的那一行是历史：它从不阻止闭合。
    let recorded = record(json!([
        {"source": "src/provider/provider.rs", "digest": digest(&root, "src/provider/provider.rs")},
        {"source": "src/provider/provider_gone.rs", "digest": null},
    ]));
    assert!(
        super::applied_instances(&root, &recorded, stable())
            .expect("the comparison runs")
            .is_none(),
        "a historical row with no digest is not a live instance"
    );
    // A record from before instances were bound has no array at all.
    // 在实例绑定之前写下的记录根本没有数组。
    let old = json!({"object": {"path": "root/provider", "stable": stable()}});
    assert!(
        super::applied_instances(&root, &old, stable())
            .expect("the comparison runs")
            .is_none(),
        "an old record keeps its single-source meaning"
    );
    let _ = std::fs::remove_dir_all(root);
}
