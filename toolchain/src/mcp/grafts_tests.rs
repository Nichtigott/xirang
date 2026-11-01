//! Tests for the graft-plan report: a declared plan names the declaration that keeps it,
//! an undeclared one is counted as unkept, an unreadable plan carries its reason, and a
//! package with no plans says so instead of showing an empty list.
//! graft 计划报告的测试：被声明的计划点名保住它的那条声明；未被声明的被计为 unkept；读不了的计划
//! 带上原因；没有计划的包如实说明，而不是给一份空清单。

use std::path::{Path, PathBuf};

use serde_json::json;
use xirang_kernel::identity::NodeId;
use xirang_kernel::plugin::graft_document::GraftPlanDocument;

use super::grafts;

/// A throwaway package with one hand-written root face, and the entry that declares a
/// cut naming it.
/// 一个含一个手写根面的一次性包，以及声明了一条点名该面的切口的入口。
fn package(label: &str, declares: bool) -> (PathBuf, String, NodeId) {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let name = format!("mcp-grafts-{label}");
    let root = std::env::temp_dir().join(format!("{name}-{}-{sequence}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("src/button")).expect("package directory");
    std::fs::write(
        root.join("Cargo.toml"),
        format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n"),
    )
    .expect("manifest");
    let declaration = if declares {
        "crate::static_graft_plan!(FRAMEWORK, cut \"root/button\" graft \"button_fast\");\n"
    } else {
        ""
    };
    std::fs::write(
        root.join("src/lib.rs"),
        format!("// host entry\n{declaration}"),
    )
    .expect("library target");
    std::fs::write(
        root.join("src/button/button.rs"),
        "pub struct Button;\n\ncrate::root_object! {\n    kind: Button,\n    parent: crate::root_node_id(env!(\"CARGO_PKG_NAME\")),\n}\n",
    )
    .expect("face");
    let id = crate::build_method::face_views(&root, &name).expect("faces derive")[0].id;
    (root, name, id)
}

/// Write one plan document under `.xirang/external-grafts/<selector>/graft.plan`.
/// 在 `.xirang/external-grafts/<selector>/graft.plan` 下写一份计划文档。
fn plan(root: &Path, selector: &str, document: &str) {
    let directory = root.join(".xirang/external-grafts").join(selector);
    std::fs::create_dir_all(&directory).expect("plan directory");
    std::fs::write(directory.join("graft.plan"), document).expect("plan file");
}

/// A plan the entry declares is reported with the declaration that keeps it; one the entry
/// does not name is counted as unkept, because the release prunes its slot and the record
/// can never take effect.
/// 入口声明了的计划会连"保住它的那条声明"一起报出；入口没点名的被计为 unkept，因为发布态会剪掉它的
/// 槽位，那条记录永远无法生效。
#[test]
fn a_declared_plan_names_its_declaration_and_an_undeclared_one_is_unkept() {
    let (root, name, id) = package("declared", true);
    plan(
        &root,
        "button_fast",
        &GraftPlanDocument::new(id, "root/button", "button_fast", false)
            .render_graft_plan_document(),
    );
    plan(
        &root,
        "orphan_fast",
        &GraftPlanDocument::new(id, "root/elsewhere", "orphan_fast", true)
            .render_graft_plan_document(),
    );
    let reply = grafts(&root, &json!({})).expect("the report renders");
    assert!(reply.contains("plans 2"), "{reply}");
    assert!(
        reply.contains("declared at entry line 2 as cut `root/button` graft `button_fast`"),
        "the keeping declaration is named: {reply}"
    );
    assert!(
        reply.contains("[NOT declared by the host entry]"),
        "the orphan is reported as such: {reply}"
    );
    assert!(
        reply.contains("unkept plans 1"),
        "and the count says what the release would drop: {reply}"
    );
    assert!(
        reply.contains("target=root/elsewhere graft=orphan_fast full=true"),
        "{reply}"
    );
    let _ = name;
    let _ = std::fs::remove_dir_all(&root);
}

/// A plan that cannot be parsed carries its reason instead of being skipped, and a package
/// with no plans directory says so — those are different answers and the reply keeps them
/// apart.
/// 解析不了的计划带上原因而不是被跳过；没有计划目录的包如实说明——这是两个不同的答案，回复把它们分开。
#[test]
fn an_unreadable_plan_carries_its_reason_and_no_plans_says_so() {
    let (root, _, id) = package("unreadable", true);
    plan(&root, "broken_graft", "version=9\n");
    let reply = grafts(&root, &json!({})).expect("the report renders");
    assert!(
        reply.contains("broken_graft: unreadable ("),
        "the reason reaches the reader: {reply}"
    );
    assert!(
        !reply.contains("unkept plans"),
        "an unreadable plan is not counted as an unkept one: {reply}"
    );
    let _ = id;

    let (bare, _, _) = package("none", false);
    let empty = grafts(&bare, &json!({})).expect("the report renders");
    assert!(
        empty.contains("no external graft plans under .xirang/external-grafts/"),
        "{empty}"
    );
    let _ = std::fs::remove_dir_all(&root);
    let _ = std::fs::remove_dir_all(&bare);
}

/// A readable entry that declares no such cut is the build's own warning case: the plan is
/// `NOT declared` and counted as unkept, because that is exactly what the release will do
/// to its slot.
/// 入口可读、但没有声明那样的切口，正是构建自己会警告的情形：该计划是 `NOT declared`，并被计为
/// unkept，因为发布态确实会这样处理它的槽位。
#[test]
fn a_readable_entry_that_declares_nothing_makes_the_plan_unkept() {
    let (root, _, id) = package("no-declaration", false);
    plan(
        &root,
        "button_fast",
        &GraftPlanDocument::new(id, "root/button", "button_fast", false)
            .render_graft_plan_document(),
    );
    let reply = grafts(&root, &json!({})).expect("the report renders");
    assert!(
        reply.contains("[NOT declared by the host entry]"),
        "{reply}"
    );
    assert!(reply.contains("unkept plans 1"), "{reply}");
    let _ = std::fs::remove_dir_all(&root);
}

/// An unreadable host entry leaves the declaration state *unknown* rather than "not
/// declared": a missing answer must not become a false one.
/// 宿主入口读不了时，声明状态是**未知**而不是"未声明"：缺失的答案不能变成一个错误的答案。
#[test]
fn an_unreadable_entry_leaves_the_declaration_state_unknown() {
    let (root, _, id) = package("entry-unreadable", true);
    // A directory where the entry belongs: the path exists and is not a readable file.
    // 入口所在的位置放一个目录：路径存在，却不是可读文件。
    std::fs::remove_file(root.join("src/lib.rs")).expect("the entry is removed");
    std::fs::create_dir_all(root.join("src/lib.rs")).expect("a directory takes its place");
    plan(
        &root,
        "button_fast",
        &GraftPlanDocument::new(id, "root/button", "button_fast", false)
            .render_graft_plan_document(),
    );
    let reply = grafts(&root, &json!({})).expect("the report renders");
    assert!(reply.contains("host entry unreadable"), "{reply}");
    assert!(reply.contains("declaration state unknown"), "{reply}");
    assert!(reply.contains("[declaration unknown]"), "{reply}");
    assert!(
        !reply.contains("unkept plans"),
        "an unknown declaration is not an unkept one: {reply}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// The bound is a bound: more plans than `limit` truncate with the total named.
/// 上限就是上限：计划多于 `limit` 时截断并说出总数。
#[test]
fn the_report_is_bounded_by_limit() {
    let (root, _, id) = package("bounded", true);
    for index in 0..4 {
        plan(
            &root,
            &format!("graft_{index}"),
            &GraftPlanDocument::new(id, "root/button", format!("graft_{index}"), false)
                .render_graft_plan_document(),
        );
    }
    let reply = grafts(&root, &json!({"limit": 2})).expect("the report renders");
    assert!(reply.contains("plans 4"), "{reply}");
    assert!(
        reply.contains("… truncated: 2 of 4 plan rows withheld at the limit of 2"),
        "{reply}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// The count is over every row, not over the rows the bound printed. Counting inside the
/// truncating loop turned `unkept plans 2` into `unkept plans 1` whenever `limit` was
/// smaller than the number of unkept plans.
/// 计数覆盖每一行，而不是上限打印出来的那些。在截断循环里计数，会让 `limit` 小于 unkept 计划数时
/// 把 `unkept plans 2` 变成 `unkept plans 1`。
#[test]
fn unkept_plans_are_counted_past_the_limit() {
    let (root, _, id) = package("unkept-count", false);
    for selector in ["button_fast", "button_slow"] {
        plan(
            &root,
            selector,
            &GraftPlanDocument::new(id, "root/button", selector, false)
                .render_graft_plan_document(),
        );
    }
    let reply = grafts(&root, &json!({"limit": 1})).expect("the report renders");
    assert!(reply.contains("plans 2"), "{reply}");
    assert!(
        reply.contains("unkept plans 2"),
        "the count is over all rows, not over the printed ones: {reply}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// The 2026-10-10 defect queue, item ②: a tree whose plan lives in its **own entry** has zero
/// external records and a plan the build applies anyway, so `plans 0` must not be the whole answer.
/// Measured on the shipped example before the fix: `plans 0` over an entry that declares two typed
/// grafts (`graft(crate::…::NODE_ID).with(…)`), with nothing in the reply saying so (report §7h).
/// 2026-10-10 缺陷队列第 ② 条：整份计划写在**自己入口**里的树，外部记录为零，而计划照旧生效，因此
/// `plans 0` 不能是全部答案。修之前在出厂示例上实测：入口声明了两条类型化 graft
/// （`graft(crate::…::NODE_ID).with(…)`），而回复只有 `plans 0`，没有一个字提到它们（报告 §7h）。
#[test]
fn an_entry_that_declares_its_own_typed_plan_is_not_reported_as_no_grafts() {
    let (root, name, _id) = package("entry-typed-plan", false);
    let entry = root.join("src/lib.rs");
    let existing = std::fs::read_to_string(&entry).expect("the entry reads");
    std::fs::write(
        &entry,
        format!(
            "{existing}\npub const GRAFTS: &[Graft] = \
             &[graft(crate::button::NODE_ID).with(crate::button_fast::NODE_ID)];\n"
        ),
    )
    .expect("the typed plan lands in the entry");
    let reply = grafts(&root, &json!({})).expect("the report renders");
    assert!(
        reply.contains("entry grafts 1"),
        "the entry's own declaration is counted: {reply}"
    );
    assert!(
        reply.contains("the build applies this one itself")
            && reply.contains("crate::button::NODE_ID"),
        "and it is named with its own typed spelling: {reply}"
    );
    assert!(
        reply.contains("plans 0"),
        "the external-record count is still zero and still said: {reply}"
    );
    assert!(
        reply.contains("the entry's own declaration(s) above are the plan the build applies"),
        "so the zero cannot be read as `no grafts`: {reply}"
    );
    let _ = name;
    let _ = std::fs::remove_dir_all(&root);
}
