//! Pins for `xirang.refactor`: what a template change carries, and the boundary it does not cross.
//! `xirang.refactor` 的钉子：一次模板变化搬运什么，以及它不越过的边界。

use serde_json::json;
use std::path::PathBuf;

/// A package with a policy file, like the template fixtures around it.
/// 一个带政策文件的包，与它周围的模板夹具相同。
fn package(label: &str) -> PathBuf {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let name = format!("mcp-refactor-{label}");
    let root = std::env::temp_dir().join(format!("{name}-{}-{sequence}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("src")).expect("package directory");
    std::fs::write(
        root.join("Cargo.toml"),
        format!(
            "[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n[workspace]\n"
        ),
    )
    .expect("manifest");
    std::fs::write(root.join("src/lib.rs"), "// host entry\n").expect("library target");
    std::fs::write(root.join("policy.txt"), "fixture policy version 1\n").expect("policy");
    root
}

fn add(root: &std::path::Path, request: serde_json::Value) -> String {
    crate::mcp::apply::apply(root, &request).expect("the add applies")
}

fn edit(root: &std::path::Path, node: &str, fields: serde_json::Value) -> String {
    crate::mcp::apply::apply(
        root,
        &json!({"action":"edit","node":node,"fields":fields,"apply":true}),
    )
    .expect("the edit applies")
}

/// Confirm one anchor against exactly the files given, so a later change to one of them moves the
/// version the lineage records.
/// 对着恰好给定的文件确认一个锚点，使之后其中一个的改动会移动谱系记录的版本。
fn adopt(root: &std::path::Path, anchor: &str, files: &[&str]) -> String {
    crate::mcp::adopted::adopted(
        root,
        &json!({"anchor":anchor,"certifies":"the specimen shape","evidence":"desk check",
                "verifier":"refactor test","reason":"this face is the specimen for its family",
                "files":files,"apply":true,"confirm":true}),
    )
    .expect("the adoption applies")
}

fn refactor(root: &std::path::Path, arguments: serde_json::Value) -> String {
    super::refactor(root, &arguments).expect("the refactor plans")
}

fn source(root: &std::path::Path, path: &str) -> String {
    std::fs::read_to_string(root.join(path)).expect("the face file")
}

fn lineage(root: &std::path::Path) -> String {
    std::fs::read_to_string(root.join(".xirang/templates/lineage.tsv")).expect("the lineage")
}

/// **A template's shape change is carried to its followers, and only then does the version move.**
/// **模板的形状变化会被搬到跟随者身上，只有到那时版本才移动。**
///
/// The copy is the same one `add` performs: the follower still leaves the field at its default, so
/// nothing an author chose is overwritten. A plan writes nothing, and applying it writes the byte the
/// request named and advances the record only after the file agrees.
/// 这次复制与 `add` 做的完全相同：跟随者仍把该字段留在默认值上，因此作者选定的东西没有被覆盖。计划不写任何
/// 东西，而执行它写下请求点名的那个字节，并在文件一致之后才推进记录。
#[test]
fn a_shape_change_is_carried_and_the_version_advances() {
    let root = package("shape");
    add(
        &root,
        json!({"action":"add","fields":{"module":"widget","kind":"Widget"},"apply":true}),
    );
    adopt(&root, "root/widget", &["src/widget/widget.rs"]);
    let follower = add(
        &root,
        json!({"action":"add","fields":{"module":"widget2","kind":"Widget"},"apply":true}),
    );
    assert!(
        follower.contains("follows the approved specimen `root/widget`"),
        "the fixture follower is created from the template: {follower}"
    );
    let before_file = source(&root, "src/widget2/widget2.rs");
    assert!(
        !before_file.contains("preset::<"),
        "the follower was created before the template had a shape: {before_file}"
    );
    let before_lineage = lineage(&root);
    // The template gains a shape, and the decision is re-confirmed against the new bytes.
    // 模板获得一个形状，而决定被对着新字节重新确认。
    edit(&root, "root/widget", json!({"preset":"FastPreset"}));
    let adoption = adopt(&root, "root/widget", &["src/widget/widget.rs"]);
    assert!(
        adoption.contains("followers on an earlier template version: root/widget2"),
        "the re-confirmed decision names the follower it left behind: {adoption}"
    );
    assert!(
        adoption.contains("`xirang.refactor {anchor: \"root/widget\"}` plans the carry"),
        "and names the call that turns that fact into a plan: {adoption}"
    );

    let plan = refactor(&root, json!({"anchor":"root/widget"}));
    assert!(
        plan.contains("mechanical: `preset` `FastPreset` (a field `add` copies from a template)"),
        "the plan classifies the copy as the one `add` performs: {plan}"
    );
    assert!(
        plan.contains("\"preset\":\"FastPreset\"") && plan.contains("\"node\":\"root/widget2\""),
        "the plan carries a paste-ready request: {plan}"
    );
    assert!(
        plan.contains("nothing was written"),
        "a plan writes nothing and says so: {plan}"
    );
    assert_eq!(lineage(&root), before_lineage, "the plan wrote no lineage");
    assert_eq!(
        source(&root, "src/widget2/widget2.rs"),
        before_file,
        "the plan wrote no follower byte"
    );

    let applied = refactor(&root, json!({"anchor":"root/widget","apply":true}));
    assert!(
        applied.contains("applied: one edit landed (`preset` <- `FastPreset`)"),
        "the mechanical copy landed through the edit path: {applied}"
    );
    assert!(
        applied.contains("advanced the lineage of `root/widget2`"),
        "the record moved only after the byte did: {applied}"
    );
    let after_file = source(&root, "src/widget2/widget2.rs");
    assert!(
        after_file.contains("preset::<FastPreset>()"),
        "the follower carries the template's shape: {after_file}"
    );
    assert_ne!(lineage(&root), before_lineage, "the lineage version moved");

    let again = refactor(&root, json!({"anchor":"root/widget"}));
    assert!(
        again.contains("1 already current") && !again.contains("mechanical:"),
        "a carried follower is not listed again: {again}"
    );
    let _ = std::fs::remove_dir_all(root);
}

/// **A lease that changed without changing the shape advances the record and touches no byte.**
/// **租约变了而形状没变的，只推进记录、不碰任何字节。**
///
/// This is the common case — a decision re-confirmed over new policy bytes — and it must not look
/// like a refactor of the file, nor must it stay listed forever.
/// 这是常见情形——一个决定在新政策字节上被重新确认——它既不该看起来像对文件的改写，也不该永远留在清单上。
#[test]
fn a_version_only_change_advances_the_record_without_touching_bytes() {
    let root = package("neutral");
    add(
        &root,
        json!({"action":"add","fields":{"module":"widget","kind":"Widget"},"apply":true}),
    );
    adopt(&root, "root/widget", &["policy.txt"]);
    add(
        &root,
        json!({"action":"add","fields":{"module":"widget2","kind":"Widget"},"apply":true}),
    );
    let before = source(&root, "src/widget2/widget2.rs");
    std::fs::write(root.join("policy.txt"), "fixture policy version 2\n").expect("policy moved");
    let adoption = adopt(&root, "root/widget", &["policy.txt"]);
    assert!(
        adoption.contains("followers on an earlier template version: root/widget2"),
        "the fixture has a follower on an earlier version: {adoption}"
    );

    let plan = refactor(&root, json!({"anchor":"root/widget"}));
    assert!(
        plan.contains("shape: already the template's; only the version differs"),
        "the plan says why there is nothing to rewrite: {plan}"
    );
    assert!(
        !plan.contains("mechanical:") && !plan.contains("conflict:"),
        "nothing was classified as a change: {plan}"
    );
    let applied = refactor(&root, json!({"anchor":"root/widget","apply":true}));
    assert!(
        applied.contains("advanced the lineage of `root/widget2`"),
        "the record is what moved: {applied}"
    );
    assert_eq!(
        source(&root, "src/widget2/widget2.rs"),
        before,
        "no byte of the follower moved"
    );
    let again = refactor(&root, json!({"anchor":"root/widget"}));
    assert!(
        again.contains("1 already current"),
        "the follower is settled afterwards: {again}"
    );
    let _ = std::fs::remove_dir_all(root);
}

/// **A value the follower chose is reported, requested, and never overwritten.**
/// **跟随者自己选定的值只被报告、以请求给出，永不被覆盖。**
///
/// The tool cannot tell "copied from an older template" from "chosen deliberately", so treating the
/// difference as semantic is the only honest classification — and it must keep the record behind, or
/// the next plan would stop listing a difference nobody resolved.
/// 工具分不出"抄自旧模板"与"刻意选定"，因此把这个差异当作语义是唯一诚实的分类——而它必须把记录留在后面，
/// 否则下一份计划就不再列出这个没人解决的差异。
#[test]
fn a_value_the_follower_chose_is_reported_and_not_overwritten() {
    let root = package("conflict");
    add(
        &root,
        json!({"action":"add","fields":{"module":"widget","kind":"Widget"},"apply":true}),
    );
    adopt(&root, "root/widget", &["src/widget/widget.rs"]);
    add(
        &root,
        json!({"action":"add","fields":{"module":"widget2","kind":"Widget","preset":"SlowPreset"},
                "apply":true}),
    );
    edit(&root, "root/widget", json!({"preset":"FastPreset"}));
    adopt(&root, "root/widget", &["src/widget/widget.rs"]);
    let before_file = source(&root, "src/widget2/widget2.rs");
    let before_lineage = lineage(&root);

    let plan = refactor(&root, json!({"anchor":"root/widget"}));
    assert!(
        plan.contains(
            "conflict: declares `preset` `SlowPreset` while the template now declares `FastPreset`"
        ),
        "the plan states both values: {plan}"
    );
    assert!(
        plan.contains("not applied: a template does not overwrite a value an object chose"),
        "it says why it will not write it: {plan}"
    );
    assert!(
        plan.contains("\"preset\":\"FastPreset\""),
        "and leaves a request that would resolve it: {plan}"
    );
    assert!(
        plan.contains("1 behind"),
        "the follower stays counted as behind in a plan: {plan}"
    );

    let applied = refactor(&root, json!({"anchor":"root/widget","apply":true}));
    assert!(
        applied.contains("conflict: declares `preset` `SlowPreset`"),
        "the write says the same thing as the plan: {applied}"
    );
    assert!(
        applied.contains("its lineage stays at"),
        "the record does not advance over an unresolved difference: {applied}"
    );
    assert_eq!(
        source(&root, "src/widget2/widget2.rs"),
        before_file,
        "the follower's own value is still there"
    );
    assert_eq!(lineage(&root), before_lineage, "the record did not move");
    let again = refactor(&root, json!({"anchor":"root/widget"}));
    assert!(
        again.contains("conflict: declares `preset` `SlowPreset`") && again.contains("1 behind"),
        "the next plan lists the unresolved difference again: {again}"
    );
    let _ = std::fs::remove_dir_all(root);
}

/// **A file `edit` may not write is reported, not attempted.**
/// **`edit` 不能写的文件只被报告，不被尝试。**
///
/// The generated marker is the one text contract `edit` checks; without it the mechanical copy would
/// be refused mid-plan, so the plan says what to paste instead and leaves the record where it is.
/// 生成标记是 `edit` 检查的唯一文本契约；没有它，机械复制会在计划中途被拒，因此计划说出该粘贴什么，并把记录
/// 留在原处。
#[test]
fn a_hand_written_follower_is_reported_and_not_written() {
    let root = package("handwritten");
    add(
        &root,
        json!({"action":"add","fields":{"module":"widget","kind":"Widget"},"apply":true}),
    );
    adopt(&root, "root/widget", &["src/widget/widget.rs"]);
    add(
        &root,
        json!({"action":"add","fields":{"module":"widget2","kind":"Widget"},"apply":true}),
    );
    // The marker is what `edit` treats as "this file is mine"; removing it is the hand-written shape.
    // 标记是 `edit` 判定"这个文件是我的"的依据；去掉它就是手写形状。
    let path = root.join("src/widget2/widget2.rs");
    let stripped = source(&root, "src/widget2/widget2.rs")
        .lines()
        .filter(|line| line.trim() != xirang_kernel::lexicon::GENERATED_MARKER)
        .collect::<Vec<_>>()
        .join("\n")
        + "\n";
    std::fs::write(&path, &stripped).expect("the marker is gone");
    edit(&root, "root/widget", json!({"preset":"FastPreset"}));
    adopt(&root, "root/widget", &["src/widget/widget.rs"]);

    let plan = refactor(&root, json!({"anchor":"root/widget"}));
    assert!(
        plan.contains("mechanical but not writable here"),
        "the plan names the copy it will not write: {plan}"
    );
    assert!(
        plan.contains("paste into the declaration: .preset::<FastPreset>()"),
        "and gives the exact line to paste: {plan}"
    );
    let applied = refactor(&root, json!({"anchor":"root/widget","apply":true}));
    assert!(
        applied.contains("its lineage stays at"),
        "nothing was written, so the record does not move: {applied}"
    );
    assert_eq!(
        source(&root, "src/widget2/widget2.rs"),
        stripped,
        "the hand-written file is byte for byte unchanged"
    );
    let _ = std::fs::remove_dir_all(root);
}

/// **A lease that is not in force is not a target.**
/// **不在生效中的租约不是目标。**
#[test]
fn a_lapsed_lease_plans_nothing() {
    let root = package("lapsed");
    add(
        &root,
        json!({"action":"add","fields":{"module":"widget","kind":"Widget"},"apply":true}),
    );
    adopt(&root, "root/widget", &["policy.txt"]);
    add(
        &root,
        json!({"action":"add","fields":{"module":"widget2","kind":"Widget"},"apply":true}),
    );
    // The bytes the decision was taken on moved, and nobody re-confirmed it.
    // 决定所依据的字节动了，而没有人重新确认它。
    std::fs::write(root.join("policy.txt"), "fixture policy version 2\n").expect("policy moved");
    let before_lineage = lineage(&root);
    let plan = refactor(&root, json!({"anchor":"root/widget"}));
    assert!(
        plan.contains("nothing is planned") && plan.contains("its lease lapsed"),
        "an unconfirmed version is not a refactor target: {plan}"
    );
    let applied = refactor(&root, json!({"anchor":"root/widget","apply":true}));
    assert!(
        applied.contains("nothing is planned"),
        "the same answer in both modes: {applied}"
    );
    assert_eq!(lineage(&root), before_lineage, "no record moved");
    let _ = std::fs::remove_dir_all(root);
}

/// **The advertised tool answers a real call.**
/// **被广告的工具对一次真实调用作答。**
///
/// The catalogue, the dispatch table and the surface's own reply are three places, and this pin holds
/// them together by calling the tool the way a client does.
/// 目录、分派表与执行面自己的回复是三处，本钉子按客户端的方式调用工具，把它们钉在一起。
#[test]
fn the_advertised_tool_answers_a_real_call() {
    let root = package("advertised");
    add(
        &root,
        json!({"action":"add","fields":{"module":"widget","kind":"Widget"},"apply":true}),
    );
    adopt(&root, "root/widget", &["policy.txt"]);
    add(
        &root,
        json!({"action":"add","fields":{"module":"widget2","kind":"Widget"},"apply":true}),
    );
    // A lease nobody re-confirmed: the version it records is not a target.
    // 一份没人重新确认的租约：它记录的版本不是目标。
    std::fs::write(root.join("policy.txt"), "fixture policy version 2\n").expect("policy moved");
    let reply = crate::mcp::tools::tool_call(
        &root,
        json!(1),
        &json!({"name":"xirang.refactor","arguments":{"anchor":"root/widget"}}),
    );
    let text = reply["result"]["content"][0]["text"]
        .as_str()
        .unwrap_or_else(|| panic!("the refactor returned no text: {reply}"));
    let failed = reply["result"]["isError"].as_bool().unwrap_or(false);
    assert!(
        !failed && text.contains("refactor plan for the template lineage"),
        "the advertised tool answers: {text}"
    );
    assert!(
        text.contains("nothing is planned") && text.contains("lease lapsed"),
        "a template with no confirmed lease is refused by name: {text}"
    );
    let _ = std::fs::remove_dir_all(root);
}
