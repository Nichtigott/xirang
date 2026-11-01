//! Tests for the tree diff: a match, an addition, a removal, and a face that
//! changed identity under a file that did not move.
//! 树 diff 的测试：匹配、新增、移除，以及文件没动而身份变了的面。

use std::path::{Path, PathBuf};

use crate::build_method::face_views;
use serde_json::json;

use super::diff;

/// A throwaway package with one hand-written root face.
/// 一个含一个手写根面的一次性包。
fn package(label: &str) -> (PathBuf, String) {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let name = format!("mcp-diff-{label}");
    let root = std::env::temp_dir().join(format!("{name}-{}-{sequence}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("src/button")).expect("package directory");
    std::fs::write(
        root.join("Cargo.toml"),
        format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n"),
    )
    .expect("manifest");
    std::fs::write(root.join("src/lib.rs"), "// host entry\n").expect("library target");
    std::fs::write(
        root.join("src/button/button.rs"),
        "pub struct Button;\n\ncrate::root_object! {\n    kind: Button,\n    parent: crate::root_node_id(env!(\"CARGO_PKG_NAME\")),\n}\n",
    )
    .expect("face");
    (root, name)
}

/// Publish a build manifest with exactly the rows given.
/// 发布一份只含给定行的构建清单。
fn publish(root: &Path, rows: &[String]) {
    let out = root.join("target/xirang/out");
    std::fs::create_dir_all(&out).expect("build output");
    let mut text = String::from("# node\tsource\tsymbol\n");
    for row in rows {
        text.push_str(row);
        text.push('\n');
    }
    std::fs::write(out.join("pruning_manifest.tsv"), text).expect("pruning manifest");
}

/// A manifest matching the sources is reported as a match, not as an empty diff.
/// 与源码一致的清单被报成一致，而不是一份空 diff。
#[test]
fn a_matching_build_reports_no_delta() {
    let (root, name) = package("match");
    let face = face_views(&root, &name).expect("faces derive")[0].clone();
    publish(&root, &[format!("{}\t{}\t-", face.id, face.source)]);
    let reply = diff(&root, &json!({})).expect("the diff renders");
    assert!(
        reply.contains("added since build 0  gone 0  re-identified 0"),
        "{reply}"
    );
    assert!(reply.contains("face for face"), "{reply}");
    let _ = std::fs::remove_dir_all(&root);
}

/// A face the build never saw is added; a face it saw and the sources no longer
/// declare is gone.
/// 构建从未见过的面是新增；构建见过而源码不再声明的是移除。
#[test]
fn an_addition_and_a_removal_are_both_named() {
    let (root, name) = package("delta");
    let old = xirang_kernel::identity::NodeId::from_namespaced_path(&name, "old/old.rs", "Old");
    publish(&root, &[format!("{old}\told/old.rs\t-")]);
    let reply = diff(&root, &json!({})).expect("the diff renders");
    assert!(reply.contains("added since build 1  gone 1"), "{reply}");
    assert!(reply.contains("+ root/button"), "{reply}");
    assert!(reply.contains("- old/old.rs"), "{reply}");
    let _ = std::fs::remove_dir_all(&root);
}

/// The same source with a different identity is the one case a text diff would
/// miss entirely.
/// 同一份源码、不同的身份，正是文本 diff 完全看不见的那种情况。
#[test]
fn a_changed_identity_under_an_unmoved_file_is_reported() {
    let (root, name) = package("reidentified");
    let face = face_views(&root, &name).expect("faces derive")[0].clone();
    let renamed =
        xirang_kernel::identity::NodeId::from_namespaced_path(&name, &face.source, "Renamed");
    publish(&root, &[format!("{renamed}\t{}\t-", face.source)]);
    let reply = diff(&root, &json!({})).expect("the diff renders");
    assert!(reply.contains("re-identified 1"), "{reply}");
    assert!(reply.contains("~ root/button"), "{reply}");
    let _ = std::fs::remove_dir_all(&root);
}

/// Without a build the answer names the command that produces one.
/// 没有构建时，答案点名产出它的命令。
#[test]
fn a_missing_build_asks_for_one_instead_of_diffing_nothing() {
    let (root, _) = package("no-build");
    let reply = diff(&root, &json!({})).expect("the diff renders");
    assert!(reply.contains("no build evidence"), "{reply}");
    assert!(reply.contains("xirang check"), "{reply}");
    let _ = std::fs::remove_dir_all(&root);
}

/// Write one external graft record under `.xirang/external-grafts/<selector>/graft.plan`.
/// 在 `.xirang/external-grafts/<selector>/graft.plan` 下写一条外部 graft 记录。
fn record(root: &Path, selector: &str, target: xirang_kernel::identity::NodeId, path: &str) {
    let directory = root.join(".xirang/external-grafts").join(selector);
    std::fs::create_dir_all(&directory).expect("record directory");
    std::fs::write(
        directory.join("graft.plan"),
        format!("version=1\ntarget={target}\ntarget_path={path}\ngraft={selector}\nfull=false\n"),
    )
    .expect("record file");
}

/// The record side, both questions asked in both branches: a record whose identity the tree still
/// has is `ok` when some cut names its slot; one whose slot moved identity is `re-identified` when
/// a cut still names the slot; and a record whose slot **no** cut names is `undeclared` whether or
/// not its stored identity is still in the tree.
/// 记录那一侧，两个分支里都问两个问题：身份仍在树里的记录，有切口点名它的槽位时是 `ok`；槽位换了
/// 身份而仍有切口点名该槽位时是 `re-identified`；而**没有**任何切口点名其槽位的记录是 `undeclared`
/// ——无论它存下来的身份还在不在树里。
///
/// That last clause is the fix for `LGC-LG-12`: the declaration question used to be asked only in
/// the "identity is in the tree" branch, so a record whose identity was absent (the typical
/// "slot unmoved, identity changed" case) came back `re-identified` or `stale` with `undeclared 0`
/// — while `xirang.grafts` reported the same row `[NOT declared by the host entry]` and counted
/// it under `unkept plans N`, and the build refused it. One record, two opposite recommendations,
/// and the optimistic one was the wrong one.
/// 最后一句就是 `LGC-LG-12` 的修复：那个"是否被声明"的问题过去只在"身份在树里"那一支被问到，因此
/// 身份缺席的记录（典型的"槽位没动、身份换了"）会带着 `undeclared 0` 回成 `re-identified` 或
/// `stale`——而 `xirang.grafts` 把同一行报成 `[NOT declared by the host entry]` 并计入
/// `unkept plans N`，构建也拒绝它。一条记录两个相反的建议，而乐观的那个是错的。
///
/// There is no `unmatched` bucket: it existed for "the identity is absent but the plan's path looks
/// like a Rust expression (`::`)", and no plan writer produces that — both write `registry.path_for`
/// — while a typed *declaration* cannot be matched at all when the identity is absent (nothing
/// resolves its module, so `names_face` admits only string cuts there). The bucket described an
/// input that cannot occur and its heading claimed a real case; a record naming something this tree
/// has not got is judged by the same declaration rule as any other (audit `m1`).
/// 没有 `unmatched` 桶：它是为"身份缺席、而计划的路径看起来像 Rust 表达式（`::`）"而设的，而没有
/// 任何计划写入方会产出那种东西——两处都写 `registry.path_for`——而类型化的**声明**在身份缺席时
/// 根本匹配不上（没有任何东西能解析它的模块，因此 `names_face` 只接受字符串切口）。那个桶描述的
/// 是一个不可能出现的输入，标题却声称描述真实情形；点名了本树没有的东西的记录，与其它记录一样按同
/// 一条声明规则判定（审计 `m1`）。
#[test]
fn the_record_side_tells_a_stale_record_from_a_re_identified_one() {
    let (root, name) = package("records");
    let face = face_views(&root, &name).expect("faces derive")[0].clone();
    let stale_identity =
        xirang_kernel::identity::NodeId::from_namespaced_path(&name, &face.source, "Renamed");
    // `kept_fast`'s slot is named by the host entry, which is what makes it `ok` rather than
    // `undeclared`: a record no cut names is pruned by the release.
    // `kept_fast` 的槽位由宿主入口点名，这正是它成为 `ok` 而不是 `undeclared` 的原因：没有任何切口
    // 点名的记录会被发布剪掉。
    std::fs::write(
        root.join("src/lib.rs"),
        "// host entry\ncrate::static_graft_plan!(FRAMEWORK, cut \"root/button\" graft \"kept_fast\");\n",
    )
    .expect("host entry");
    record(&root, "kept_fast", face.id, "root/button");
    record(&root, "moved_fast", stale_identity, "root/button");
    record(&root, "ghost_fast", stale_identity, "root/ghost");
    record(
        &root,
        "typed_fast",
        stale_identity,
        "crate::control::NODE_ID",
    );
    let reply = diff(&root, &json!({"records": true})).expect("the diff renders");
    assert!(
        reply.contains("records 4 (external graft plans)"),
        "{reply}"
    );
    // `ghost_fast` and `typed_fast` name slots no cut names: unkept beats drifted, because the
    // first verdict is the one that decides whether the record can ever take effect.
    // `ghost_fast` 与 `typed_fast` 点名的槽位没有任何切口点名：**没被保住**压过"漂移了"，因为
    // 前一个结论才决定这条记录能否生效。
    assert!(
        reply.contains("ok 1  undeclared 2  stale 0  re-identified 1  unreadable 0"),
        "{reply}"
    );
    assert!(
        !reply.contains("unmatched"),
        "the bucket that described an impossible input is gone: {reply}"
    );
    assert!(reply.contains("kept_fast -> root/button"), "{reply}");
    assert!(
        reply.contains("! ghost_fast -> root/ghost  (no cut in the host entry names it)"),
        "an undeclared record whose identity is absent is unkept, not merely stale: {reply}"
    );
    assert!(
        reply.contains(
            "! typed_fast -> crate::control::NODE_ID  (no cut in the host entry names it)"
        ),
        "a typed path with no identity resolves to no cut either, and that is the verdict: {reply}"
    );
    assert!(
        reply.contains(&format!("~ moved_fast {stale_identity} -> {}", face.id)),
        "the record that moved identity into a declared slot names the identity the tree has now: {reply}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// A record whose identity the tree has is not `ok` unless a `static_graft_plan!` cut names its
/// slot. `diff {"records":true}` used to read only the identity and report `ok`, while
/// `xirang.grafts` reported the same row as `[NOT declared by the host entry]`, counted it under
/// `unkept plans N`, and the build refused it outright: one record, two health conclusions, and the
/// agent that read the first one would ship it (audit `M1`). The row already carried
/// `declared`, so the split costs nothing.
/// 身份在树里的记录，只有在某条 `static_graft_plan!` 切口点名它的槽位时才是 `ok`。
/// `diff {"records":true}` 过去只读身份就报 `ok`，而 `xirang.grafts` 把同一行报成
/// `[NOT declared by the host entry]`、计入 `unkept plans N`，构建还直接拒绝它：同一条记录、两个健康
/// 结论，而读到前者的代理会把它发出去（审计 `M1`）。该行本就携带 `declared`，分流不需额外代价。
#[test]
fn an_undeclared_record_is_not_reported_as_ok() {
    let (root, name) = package("undeclared");
    let face = face_views(&root, &name).expect("faces derive")[0].clone();
    // The identity is present in the tree; no host entry names this slot.
    // 身份在树里；没有任何宿主入口点名这个槽位。
    record(&root, "orphan_fast", face.id, "root/button");
    let reply = diff(&root, &json!({"records": true})).expect("the diff renders");
    assert!(
        reply.contains("ok 0  undeclared 1  stale 0  re-identified 0  unreadable 0"),
        "an undeclared record is not ok: {reply}"
    );
    assert!(
        !reply.contains("  orphan_fast -> root/button"),
        "it must not be listed under `ok:` either: {reply}"
    );
    assert!(
        reply.contains("! orphan_fast -> root/button  (no cut in the host entry names it)"),
        "and the reply says which case it is: {reply}"
    );
    assert!(
        reply.contains("the release prunes these slots"),
        "with the consequence, the same one `xirang.grafts` reports: {reply}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// An unreadable record is counted rather than dropped, and a package with no records says
/// `records 0` instead of claiming a clean bill of health on nothing.
/// 读不了的记录会被计数而不是被丢掉；没有记录的包会说 `records 0`，而不是在没有东西时声称一切正常。
#[test]
fn an_unreadable_record_is_counted_and_no_records_is_not_a_verdict() {
    let (root, _) = package("records-unreadable");
    let directory = root.join(".xirang/external-grafts/broken_fast");
    std::fs::create_dir_all(&directory).expect("record directory");
    std::fs::write(directory.join("graft.plan"), "version=9\n").expect("record file");
    let reply = diff(&root, &json!({"records": true})).expect("the diff renders");
    assert!(reply.contains("unreadable 1"), "{reply}");
    assert!(
        reply.contains("ok 0  undeclared 0  stale 0  re-identified 0  unreadable 1"),
        "{reply}"
    );
    let _ = std::fs::remove_dir_all(&root);

    let (bare, _) = package("records-none");
    let empty = diff(&bare, &json!({"records": true})).expect("the diff renders");
    assert!(
        empty.contains("records 0 (external graft plans)"),
        "{empty}"
    );
    let _ = std::fs::remove_dir_all(&bare);
}

/// Every list this report prints is bounded **and says so**: the four record buckets used
/// to be cut at `limit` with only their headline counts left to imply it, which is a
/// reader's inference rather than the answer's own declaration.
/// 本报告打印的每个清单都有上限**而且自己说出来**：记录侧的四个桶过去在 `limit` 处被切短，只留下
/// 头条计数让人去推断——那是读者的推理，而不是答案自己的声明。
#[test]
fn a_record_list_cut_by_the_limit_says_how_many_it_withheld() {
    let (root, name) = package("record-bound");
    let face = face_views(&root, &name).expect("faces derive")[0].clone();
    for selector in ["orphan_fast", "orphan_slow"] {
        record(&root, selector, face.id, "root/button");
    }
    let reply = diff(&root, &json!({"records": true, "limit": 1})).expect("the diff renders");
    assert!(
        reply.contains("ok 0  undeclared 2  stale 0  re-identified 0  unreadable 0"),
        "{reply}"
    );
    assert!(
        reply.contains("… truncated: 1 of 2 undeclared records withheld at the limit of 1"),
        "{reply}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// A registration file the derivation cannot parse is counted in the reply instead of vanishing:
/// a read-only tree query used to report a smaller tree as if it were the whole one, which is what
/// `LGC-LG-11` recorded on the producer side and what the consumer half now says out loud.
/// 推导解析不了的注册面文件被计入回复而不是消失：只读的树查询过去把一棵更小的树当成完整的树报出去
/// ——这正是 `LGC-LG-11` 在生产端记录的事，而消费端现在把它说出来。
fn broken_face(root: &std::path::Path, label: &str) {
    let directory = root.join("src").join(label);
    std::fs::create_dir_all(&directory).expect("module directory");
    std::fs::write(
        directory.join(format!("{label}.rs")),
        "crate::root_object! {\n    kind: Broken,\n",
    )
    .expect("truncated face");
}

#[test]
fn the_tree_diff_counts_unparsable_registration_files() {
    let (root, name) = package("unparsable");
    publish(&root, &[]);
    let _ = name;
    broken_face(&root, "broken");
    let reply = diff(&root, &json!({})).expect("the diff renders");
    assert!(reply.contains("unparsable faces 1"), "{reply}");
    let _ = std::fs::remove_dir_all(&root);
}

/// One directory of published records with exactly the given scope and pruning rows.
/// 一个只含给定作用域与剪枝行的已发布记录目录。
fn write_record_set(out: &Path, token: &str, id: &str, source: &str, symbol: &str) {
    std::fs::create_dir_all(out).expect("record directory");
    std::fs::write(out.join("discovery.fingerprint"), format!("{token}\n")).expect("token");
    std::fs::write(
        out.join("source_scope.tsv"),
        format!("# mode\tauto\n# selected\t1\n# node\tsource\tmodule\n{id}\t{source}\tcontrol\n"),
    )
    .expect("scope record");
    std::fs::write(
        out.join("pruning_manifest.tsv"),
        format!("# node\tsource\tsymbol\n{id}\t{source}\t{symbol}\n"),
    )
    .expect("pruning record");
}

#[test]
fn two_record_sets_are_compared_as_data() {
    let (root, _name) = package("records-against");
    let id = "bdb4427ce81c9bc51e56bee7667fd2be";
    write_record_set(
        &root.join("target/xirang/out"),
        "aaaa",
        id,
        "control/control.rs",
        "-",
    );
    write_record_set(
        &root.join("baseline"),
        "bbbb",
        id,
        "control/control.rs",
        "paint",
    );
    let report = diff(&root, &json!({"against": "baseline"})).expect("a comparison");
    assert!(report.contains("records there"), "{report}");
    assert!(
        report.contains("fingerprint here aaaa  there bbbb  -> different"),
        "{report}"
    );
    assert!(
        report.contains("pruning rows added 0  gone 0  changed 1"),
        "{report}"
    );
    assert!(report.contains("`-` -> `paint`"), "{report}");
    assert!(
        report.contains("not compared"),
        "the sections without a reader are named: {report}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_record_directory_outside_the_root_is_refused() {
    let (root, _name) = package("records-outside");
    let error = diff(&root, &json!({"against": "../elsewhere"})).expect_err("a refusal");
    assert!(error.contains("must stay inside"), "{error}");
    let _ = std::fs::remove_dir_all(&root);
}

/// Records published under **another namespace** are refused, not compared: every face would look
/// like it moved, and that is the wrong answer that looks like a right one (§M7.18).
/// 发布在**另一个命名空间**下的记录会被**拒绝**，而不是被比较：每个面都会看起来搬了家，而那正是"看起来
/// 正确的错误答案"（§M7.18）。
///
/// The two ids differ *only* by the namespace (`hash(namespace, path, name)`), so this pins both
/// halves: the refusal names both domains and the way forward, and **no** comparison is emitted.
/// 两个 id **只**差命名空间（`hash(命名空间, 路径, 名字)`），因此这条钉子钉住两半：拒绝点名两个身份域与
/// 出路，而且**不**输出任何比较。
///
/// The read happens in a child process because the namespace is process-global — the same reason
/// `verify`'s namespace pin uses one.
/// 读取发生在子进程里，因为命名空间是进程级的——与 `verify` 那条命名空间钉子同一个理由。
#[test]
fn records_published_under_another_namespace_are_refused() {
    let (root, name) = package("other-domain");
    crate::build_method::check_for(&root, &crate::mcp::build_evidence::out_dir(&root), &name)
        .expect("the tree publishes cleanly");
    let child = std::process::Command::new(std::env::current_exe().expect("the test binary"))
        .args([
            "--ignored",
            "--exact",
            "mcp::diff::diff_tests::other_domain_child",
            "--nocapture",
        ])
        .env("XIRANG_NAMESPACE", "n52-elsewhere")
        .env("N52_FIXTURE", &root)
        .output()
        .expect("the child runs");
    let stdout = String::from_utf8_lossy(&child.stdout);
    assert!(stdout.contains("namespace mismatch"), "{stdout}");
    assert!(
        stdout.contains(&name),
        "the refusal names the domain these records belong to: {stdout}"
    );
    assert!(
        stdout.contains("way forward"),
        "the refusal carries the one way forward: {stdout}"
    );
    assert!(
        !stdout.contains("re-identified"),
        "no identity comparison is emitted from another domain: {stdout}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// The child half of the pin above; it prints whatever `diff` answers.
/// 上面那条钉子的子进程那一半；它打印 `diff` 给出的任何答复。
#[test]
#[ignore = "child of records_published_under_another_namespace_are_refused"]
fn other_domain_child() {
    let root = PathBuf::from(std::env::var("N52_FIXTURE").expect("the fixture path"));
    match diff(&root, &json!({})) {
        Ok(reply) | Err(reply) => println!("{reply}"),
    }
}
