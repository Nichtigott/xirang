//! Pins for the graft half of a candidate: what a landing refuses, what it reports, and what the
//! record can be read back for after a conflict.
//! 候选的 graft 那一半的钉子：一次落地拒绝什么、报告什么，以及一次冲突之后记录能读回什么。
//!
//! §8.3's completion criterion is a sentence about the **record**: after a graft conflict it must be
//! readable which two parties the conflict was between, and whether the recovery succeeded or wrote
//! nothing. Every pin here asserts one half of that sentence, against a package whose declarations
//! are real to the parser and inert to rustc — so the pins run the product path (`open`, `verify`,
//! `land`, `status`) with a real Cargo invocation inside the copy.
//! §8.3 的完成判据是一句关于**记录**的话：一次 graft 冲突之后，必须能从记录里读回冲突在哪两方之间，以及恢复
//! 是成功了还是什么都没写。这里的每条钉子断言那句话的一半，用的包对解析器是真的、对 rustc 是惰性的——因此钉子跑的是
//! 产品路径（`open`、`verify`、`land`、`status`），并在副本里真跑一次 Cargo。

use crate::mcp::candidate::candidate;
use serde_json::{Value, json};
use std::path::{Path, PathBuf};

/// A package that compiles, carries one host entry, and has a registration face the walk can see.
/// 一个能编译、带一个宿主入口、并且有遍历看得见的注册面的包。
///
/// The graft declaration is written with `static_graft_plan!` and the fixture defines a stub macro by
/// that name, so the entry **compiles** while the parser still sees a real declaration: the pins then
/// exercise the product path with a real Cargo run instead of a tree that cannot build. The faces
/// live under `src/control/object/<name>/<name>.rs` — the shipped layout, where the walk derives
/// `root/control/<name>` — and are not reachable from the entry, so rustc never compiles them.
/// graft 声明用 `static_graft_plan!` 写，而夹具定义了一个同名桩宏，因此入口**能编译**而解析器仍看到一个真声明：
/// 于是钉子用真 Cargo 跑产品路径，而不是一棵建不出来的树。面住在 `src/control/object/<name>/<name>.rs`——
/// 出厂布局，遍历据此推出 `root/control/<name>`——且从入口不可达，因此 rustc 从不编译它们。
fn fixture(label: &str) -> PathBuf {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "xirang-graft-{label}-{}-{sequence}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("src/control/object/left")).unwrap();
    std::fs::create_dir_all(root.join("src/control/object/right")).unwrap();
    // The scope has a person behind it, and landing needs that confirmation to be on a lease rather
    // than in the request.
    // 范围背后有一个人，而落地需要那份确认落在一份租约里，而不是在请求里。
    std::fs::create_dir_all(root.join(".xirang/adopted")).unwrap();
    std::fs::write(
        root.join(".xirang/adopted/entries"),
        "control.object.button|the provider is the shared route|converge answer|maintainer|2026-10-18|src/lib.rs|deadbeef|because\n",
    )
    .unwrap();
    std::fs::write(
        root.join("Cargo.toml"),
        "[package]\nname='graft-probe'\nversion='0.0.0'\nedition='2024'\n[workspace]\n",
    )
    .unwrap();
    std::fs::write(root.join("src/lib.rs"), entry("")).unwrap();
    std::fs::write(
        root.join("src/control/control.rs"),
        "#[xirang_toolchain::soil]\n\
         pub const SOIL: xirang_toolchain::Soil = xirang_toolchain::Soil::of::<Control>()\n\
         \x20\x20\x20\x20.needs_registry(true)\n\
         \x20\x20\x20\x20.under(crate::root_node_id(crate::XIRANG_NAMESPACE));\n\n\
         pub struct Control;\n",
    )
    .unwrap();
    for name in ["left", "right"] {
        std::fs::write(
            root.join(format!("src/control/object/{name}/{name}.rs")),
            "#[xirang_toolchain::soil]\n\
             pub const SOIL: xirang_toolchain::Soil = xirang_toolchain::Soil::of::<Button>()\n\
             \x20\x20\x20\x20.under(crate::control::NODE_ID);\n\n\
             pub struct Button;\n",
        )
        .unwrap();
    }
    let output = std::process::Command::new("cargo")
        .args(["generate-lockfile", "--offline"])
        .current_dir(&root)
        .output()
        .unwrap();
    assert!(output.status.success());
    root
}

/// The host entry, with the declarations a test wants to plant in it.
/// 宿主入口，带上测试想植进去的那些声明。
///
/// `declarations` is spliced in verbatim, so a test can add the same cut twice — which is the shape a
/// conflict has.
/// `declarations` 被逐字插入，因此一个测试可以把同一条切口写两遍——那就是冲突的形状。
fn entry(declarations: &str) -> String {
    format!(
        "//! A host whose graft declarations the parser reads and rustc ignores.\n\
         //! 一个解析器会读、rustc 会忽略的 graft 声明宿主。\n\
         #[macro_export]\n\
         macro_rules! static_graft_plan {{ ($($rest:tt)*) => {{}}; }}\n\
         \n\
         {declarations}\
         pub fn answer() -> i64 {{\n    41\n}}\n\n\
         #[test]\nfn the_answer_is_stable() {{\n    assert_eq!(answer(), 41);\n}}\n"
    )
}

/// One declaration naming the `left` face.
/// 一条点名 `left` 面的声明。
const LEFT: &str =
    "crate::static_graft_plan!(FRAMEWORK, cut \"root/control/left\" graft \"left_fast\");\n";

/// A second declaration naming the **same** face, with a different replacement.
/// 第二条点名**同一个**面的声明，替换端不同。
const LEFT_AGAIN: &str =
    "crate::static_graft_plan!(FRAMEWORK, cut \"root/control/left\" graft \"left_other\");\n";

/// Open a candidate over `src`, and hand back its id, its tree, and its record's path.
/// 打开一棵范围覆盖 `src` 的候选，交回它的 id、它的树，以及它记录的路径。
fn opened(root: &Path) -> (String, PathBuf, PathBuf) {
    let reply = candidate(
        root,
        &json!({"action":"open","scope":["src"],"authority":"control.object.button"}),
    )
    .unwrap();
    let field = |name: &str| {
        reply
            .lines()
            .find_map(|line| line.strip_prefix(name))
            .unwrap()
            .to_owned()
    };
    let id = reply
        .split_whitespace()
        .nth(1)
        .expect("the first line names the candidate")
        .to_owned();
    let id_tree_record = (
        id,
        PathBuf::from(field("tree ")),
        PathBuf::from(field("record ")),
    );
    approve(root, &id_tree_record.0);
    id_tree_record
}

/// The approval a landing needs, written through the real controller store this fixture stands in for.
/// 落地所需的那份批准，经这条夹具代表的真实控制器存储写下。
fn approve(root: &Path, id: &str) {
    let fields = vec![
        ("budget".to_owned(), "executions=8".to_owned()),
        ("candidate".to_owned(), id.to_owned()),
        ("operation".to_owned(), "land".to_owned()),
        (
            "policy".to_owned(),
            crate::mcp::candidate::POLICY.to_owned(),
        ),
        (
            "project".to_owned(),
            crate::mcp::registry::namespace(root).unwrap(),
        ),
        ("proposal".to_owned(), "1".to_owned()),
        ("revision".to_owned(), "1".to_owned()),
        ("scope".to_owned(), "src".to_owned()),
        ("task".to_owned(), "candidate".to_owned()),
    ];
    crate::mcp::controller::issue(root, fields, "maintainer", "interactive-cli")
        .expect("the controller issues an approval for the fixture's candidate");
}

/// Run the verification the product path runs, and insist it passed.
/// 按产品路径跑一次验证，并要求它通过。
fn verified(root: &Path, id: &str) -> String {
    let reply = candidate(root, &json!({"action":"verify","id":id,"mode":"test"})).unwrap();
    assert!(
        reply.starts_with(&format!("candidate {id} verification passed")),
        "the fixture has to verify for a landing to be reachable: {reply}"
    );
    reply
}

/// The refusal of `candidate`, which must name the rule rather than the symptom.
/// `candidate` 的拒绝，必须点名规则而不是症状。
fn refused(root: &Path, arguments: Value) -> String {
    candidate(root, &arguments).expect_err("this call must be refused")
}

/// A landing that would leave two declarations on one slot is refused, both parties are named at
/// their own lines, nothing is written, and the record says so — then the fixed copy lands, so the
/// same record reads back "recovery succeeded" rather than "something was blocked".
/// 一次会留下两条声明争同一槽位的落地被拒，两方各自带着自己的行号被点名，什么都没写，记录里也这么写——随后修好的
/// 副本落地成功，因此同一条记录读回的是"恢复成功"，而不是"有东西被挡住了"。
#[test]
fn a_graft_conflict_names_both_parties_writes_nothing_and_then_recovers() {
    let root = fixture("conflict");
    let (id, tree, record) = opened(&root);
    let base_before = std::fs::read_to_string(root.join("src/lib.rs")).unwrap();
    // The candidate plants **both** declarations: the conflict is one this landing would introduce.
    // 候选把**两条**声明都植进去：这个冲突是这次落地会引入的那一个。
    std::fs::write(
        tree.join("src/lib.rs"),
        entry(&format!("{LEFT}{LEFT_AGAIN}")),
    )
    .unwrap();
    let verified_bytes = verified(&root, &id);
    assert!(
        verified_bytes.contains("nothing was written to"),
        "a verification runs inside the copy: {verified_bytes}"
    );
    let refusal = refused(&root, json!({"action":"land","id":id}));
    // Both parties, each with its own replacement and the slot they compete for. A count would have
    // left the reader to find which two.
    // 两方都在，各自带着自己的替换端与它们争的槽位。一个计数会把"哪两条"留给读者自己找。
    for expected in [
        "root/control/left",
        "graft `left_fast`",
        "graft `left_other`",
        "Nothing was written",
    ] {
        assert!(
            refusal.contains(expected),
            "the refusal must name `{expected}`: {refusal}"
        );
    }
    assert_eq!(
        std::fs::read_to_string(root.join("src/lib.rs")).unwrap(),
        base_before,
        "a graft conflict is refused before the first write"
    );
    // The refusal is on the **record**, because a refusal that only lives in a reply is
    // indistinguishable from a landing that never happened.
    // 这次拒绝**在记录里**，因为只活在回复里的拒绝，与一次从未发生的落地分不出来。
    let status = candidate(&root, &json!({"action":"status","id":id})).unwrap();
    for expected in [
        "graft 1 landing attempt(s) on record",
        "graft attempt 1: refused (graft conflict); wrote nothing",
        "conflict introduced: `root/control/left` is claimed by",
        "graft `left_fast`",
        "graft `left_other`",
    ] {
        assert!(
            status.contains(expected),
            "`status` reads the refusal back, and is missing `{expected}`: {status}"
        );
    }
    assert!(
        status.contains("state open"),
        "a refused landing leaves the candidate open: {status}"
    );
    // Recovery: one party removed, verified again (the bytes moved, so the old verdict does not
    // cover them), landed.
    // 恢复：去掉一方，重新验证（字节动了，旧判定不覆盖它们），然后落地。
    std::fs::write(tree.join("src/lib.rs"), entry(LEFT)).unwrap();
    verified(&root, &id);
    let landed = candidate(&root, &json!({"action":"land","id":id})).unwrap();
    assert!(
        landed.starts_with(&format!("candidate {id} landed")),
        "{landed}"
    );
    let base_after = std::fs::read_to_string(root.join("src/lib.rs")).unwrap();
    assert!(base_after.contains("left_fast") && !base_after.contains("left_other"));
    let status = candidate(&root, &json!({"action":"status","id":id})).unwrap();
    for expected in [
        "state landed",
        "graft 2 landing attempt(s) on record",
        "graft attempt 1: refused (graft conflict); wrote nothing",
        "graft attempt 2: accepted",
        "wrote src/lib.rs",
    ] {
        assert!(
            status.contains(expected),
            "the same record now reads back that the recovery landed, missing `{expected}`: {status}"
        );
    }
    // The refused attempt is still on the record after the landing: "there was a conflict and then it
    // was fixed" must stay different from "there never was one".
    // 被拒的那次尝试在落地之后仍在记录里："曾经有冲突而后被修好"必须仍与"从来没有过"不同。
    assert!(
        std::fs::read_to_string(&record)
            .unwrap()
            .contains("left_other"),
        "the landing does not erase the attempt that was refused"
    );
    std::fs::remove_dir_all(root).unwrap();
}

/// A projection that moved is printed as an old and a new object set, because a graft that still
/// parses is not a graft that still means the same thing.
/// 移动过的投影会印成旧/新两个对象集合，因为还能解析的嫁接不是含义不变的嫁接。
#[test]
fn a_landing_prints_the_object_set_it_changed() {
    let root = fixture("projection");
    let (id, tree, _record) = opened(&root);
    // Before the edit the base declares no cut at all, so this is the empty→one case: the reader
    // must be able to see that the landing **added** an object, not only that it added a line.
    // 编辑之前基树一条切口都没声明，因此这是"空→一"的情形：读者必须能看出这次落地**新增了一个对象**，
    // 而不只是新增了一行。
    std::fs::write(tree.join("src/lib.rs"), entry(LEFT)).unwrap();
    verified(&root, &id);
    let landed = candidate(&root, &json!({"action":"land","id":id})).unwrap();
    for expected in [
        "+ root/control/left graft `left_fast` now covers [root/control/left]",
        "effective objects [nothing] -> [root/control/left]",
    ] {
        assert!(
            landed.contains(expected),
            "the landing prints the semantic change (`{expected}`): {landed}"
        );
    }
    std::fs::remove_dir_all(root).unwrap();
}

/// A binding is only comparable under the rule set that produced it, so a record from another
/// version is refused instead of being reinterpreted as if it were the same contract.
/// 一份绑定只在产出它的规则集下可比，因此另一个版本写下的记录会被拒绝，而不是被当作同一份合同重新解释。
#[test]
fn a_binding_from_another_rule_set_is_not_reused() {
    let root = fixture("policy");
    let (id, tree, record) = opened(&root);
    std::fs::write(tree.join("src/lib.rs"), entry(LEFT)).unwrap();
    verified(&root, &id);
    // Simulate a record written by another version of this rule: the facts are the same tree's, and
    // the only thing that changed is the contract they were read under.
    // 模拟一条由另一版规则写下的记录：事实还是同一棵树的事实，改变的只有解读它们的合同。
    let text = std::fs::read_to_string(&record).unwrap();
    assert!(
        text.contains("xirang.graft.binding/v1"),
        "the fixture writes the policy down"
    );
    std::fs::write(
        &record,
        text.replace("xirang.graft.binding/v1", "xirang.graft.binding/v0"),
    )
    .unwrap();
    let refusal = refused(&root, json!({"action":"land","id":id}));
    for expected in [
        "xirang.graft.binding/v0",
        "xirang.graft.binding/v1",
        "a binding is only comparable under the rule set that produced it",
    ] {
        assert!(
            refusal.contains(expected),
            "the refusal names both rule sets (`{expected}`): {refusal}"
        );
    }
    let status = candidate(&root, &json!({"action":"status","id":id})).unwrap();
    assert!(
        status.contains(
            "graft attempt 1: refused (the binding was read under a different rule set or \
             configuration); wrote nothing"
        ),
        "a refused landing writes nothing and says so: {status}"
    );
    std::fs::remove_dir_all(root).unwrap();
}

/// A tree with no host entry has no graft surface, and saying `none` is different from saying
/// `unknown`: the first is a clean state, the second is a read that failed.
/// 没有宿主入口的树没有 graft 面，而说 `none` 与说 `unknown` 不同：前者是干净状态，后者是一次失败的读取。
#[test]
fn a_tree_without_a_host_entry_says_none_rather_than_unknown() {
    let root = std::env::temp_dir().join(format!("xirang-graft-none-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::write(
        root.join("Cargo.toml"),
        "[package]\nname='graft-none'\nversion='0.0.0'\nedition='2024'\n[workspace]\n",
    )
    .unwrap();
    let binding = super::binding(&root, "a stated configuration");
    assert_eq!(binding["reading"], "none", "{binding}");
    assert!(
        super::line(&binding, "at open").contains("none"),
        "the line reads `none` rather than an empty list: {}",
        super::line(&binding, "at open")
    );
    // `unknown` is reserved for a surface that exists and could not be read.
    // `unknown` 只留给"面存在但读不出来"。
    std::fs::write(root.join("src/lib.rs"), "pub fn answer() -> i64 { 41 }\n").unwrap();
    let binding = super::binding(&root, "a stated configuration");
    assert_eq!(binding["reading"], "read", "{binding}");
    assert_eq!(binding["declarations"], json!([]), "{binding}");
    std::fs::remove_dir_all(root).unwrap();
}
