//! Pins for the repair projection, driven through the dispatch CLI and MCP share.
//! 修复投影的钉子，从 CLI 与 MCP 共用的分派入口驱动。
//!
//! Every test here goes through `xirang.explore` itself rather than calling [`super::project`]
//! directly, because the projection's contract is what an agent *receives*: a section that is
//! correct in isolation but never reaches the reply has not been implemented. The fixtures vary the
//! object names, the delegation depth and the package layout on purpose (BR12): a projection that
//! only works for the three arithmetic tasks would pass a narrower set of pins.
//! The **two** pins about the `references` section are the exception, and they say why: that section
//! is keyed by a seed map, and the case "the faces could not be read" cannot be reached through a
//! query at all. So they call [`super::references`] with a hand-built map — and the one about
//! sibling roles re-checks the same section through a real `explore` query afterwards, so the
//! decision and its arrival are both pinned.
//! 这里的每个测试都走 `xirang.explore` 本身，而不是直接调用 [`super::project`]，因为投影的契约是代理
//! **收到**的东西：一个单独看正确、却到不了回复里的节，等于没有实现。夹具**有意**改变对象名、委派深度
//! 与包布局（BR12）：只对那三道算术题成立的投影会通过一组更窄的钉子。
//! 关于 `references` 那一段的**两道**钉子是例外，理由写在它们自己里面：那一段以种子映射为键，而"读不到注册面"
//! 这一情形根本无法经一次查询到达。因此它们用一份手搭的映射直接调用 [`super::references`]——而其中关于兄弟角色的
//! 那一道，之后又会经一次真的 `explore` 查询把同一段再核一遍，因此判断与它是否到得了回复都有钉子。

use serde_json::{Value, json};
use std::path::PathBuf;

struct Tree(PathBuf);

impl Tree {
    fn new(label: &str) -> Self {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let id = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let path =
            std::env::temp_dir().join(format!("xirang-repair-{label}-{}-{id}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).unwrap();
        Self(path)
    }

    fn write_source(&self, path: &str, source: &str) {
        let path = self.0.join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, source).unwrap();
    }

    fn ask(&self, args: Value) -> String {
        crate::mcp::tools::run_tool(&self.0, "xirang.explore", &args).unwrap()
    }

    fn explore(&self, query: &str) -> String {
        self.ask(json!({"query": query, "max_bytes": 24000}))
    }
}

impl Drop for Tree {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// A workspace whose shared implementation lives in one member and whose consumers live in another,
/// so the projection has to cross a package boundary to name either the public route or the tests.
/// 一个工作区：共享实现住在一个成员里、消费者住在另一个成员里，因此投影必须跨包边界才能点名公开路线或测试。
fn workspace() -> Tree {
    let tree = Tree::new("shared");
    tree.write_source(
        "Cargo.toml",
        "[workspace]\nmembers = [\"crates/math-kit\", \"crates/app-kit\"]\n",
    );
    tree.write_source(
        "crates/math-kit/Cargo.toml",
        "[package]\nname = \"math-kit\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    tree.write_source(
        "crates/math-kit/src/lib.rs",
        "//! Rounding shared by every consumer.\n\
         /// Round a step up to a whole number of steps.\n\
         pub fn settle(amount: i64, step: i64) -> i64 {\n    amount.div_euclid(step) * step\n}\n",
    );
    tree.write_source(
        "crates/app-kit/Cargo.toml",
        "[package]\nname = \"app-kit\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    tree.write_source(
        "crates/app-kit/src/lib.rs",
        "pub mod alpha;\npub mod beta;\n",
    );
    tree.write_source(
        "crates/app-kit/src/alpha.rs",
        "/// The bar chart's rounding entry.\npub fn alpha_round(amount: i64) -> i64 {\n    math_kit::settle(amount, 4)\n}\n",
    );
    tree.write_source(
        "crates/app-kit/src/beta.rs",
        "pub fn beta_round(amount: i64) -> i64 {\n    math_kit::settle(amount, 8)\n}\n",
    );
    tree.write_source(
        "crates/math-kit/tests/settle.rs",
        "#[test]\nfn settles_to_whole_steps() {\n    assert_eq!(math_kit::settle(5, 4), 4);\n}\n",
    );
    tree.write_source(
        "crates/app-kit/tests/rounding.rs",
        "#[test]\nfn both_consumers_round_up() {\n    assert_eq!(app_kit::alpha_round(5), 4);\n}\n",
    );
    tree
}

#[test]
fn one_query_names_the_shared_implementation_its_consumers_their_public_route_and_their_tests() {
    let tree = workspace();
    let reply = tree.explore("settle");
    let focus = line_with(&reply, "focus: crates/math-kit/src/lib.rs:3 settle");
    assert!(
        focus.contains("shared implementation; 2 independent consumer files"),
        "{reply}"
    );
    for consumer in ["crates/app-kit/src/alpha.rs", "crates/app-kit/src/beta.rs"] {
        assert!(focus.contains(consumer), "{consumer} missing from {focus}");
    }
    // The contract travels with the implementation, because "what must stay true" is what a repair
    // breaks when it only reads the body.
    // 合同与实现同行，因为只读函数体的修复破坏的正是"必须仍然成立"的东西。
    assert!(
        reply.contains("contract Round a step up to a whole number of steps."),
        "{reply}"
    );
    // Both consumer entry points, with their own file and line, as static candidates.
    assert!(
        reply.contains("pub fn alpha_round(amount: i64) -> i64 {"),
        "{reply}"
    );
    assert!(
        reply.contains("pub fn beta_round(amount: i64) -> i64 {"),
        "{reply}"
    );
    // The public route: package name, library root, and the declaration that confirms the module.
    assert!(
        reply.contains("public_access: package math-kit; lib src/lib.rs"),
        "{reply}"
    );
    assert!(
        reply.contains("crates/math-kit/src/lib.rs: crate [the library root itself]"),
        "{reply}"
    );
    assert!(
        reply.contains("crates/app-kit/src/alpha.rs: crate::alpha [declared at")
            && reply.contains("`pub mod alpha;`"),
        "{reply}"
    );
    // The test that reaches the implementation, and the consumer it calls by name are different
    // lines: only the second one is the consumer's own delegation being protected.
    // 到达实现的测试与"测试按名字调用的消费者"是两行：只有后者才是该消费者自身的委派得到保护。
    assert!(
        reply.contains("protected (lexically) settle <- crates/math-kit/tests/settle.rs"),
        "{reply}"
    );
    assert!(
        reply.contains("protected (lexically) alpha_round <- crates/app-kit/tests/rounding.rs"),
        "{reply}"
    );
    assert!(
        reply.contains("needs evidence: beta_round calls settle"),
        "{reply}"
    );
    assert!(reply.contains("coverage: "), "{reply}");
    assert!(
        reply.contains("source file(s) parsed in this one query; 1 selected for source"),
        "{reply}"
    );
    assert!(
        reply.contains("unknown: no macro invocation or trait-object line"),
        "{reply}"
    );
    assert!(
        reply.contains("check {files: [\"crates/math-kit/src/lib.rs\"], census: false}"),
        "{reply}"
    );
}

#[test]
fn a_single_consumer_is_reported_as_not_shared_rather_than_as_a_shared_root_cause() {
    let tree = workspace();
    tree.write_source(
        "crates/app-kit/src/beta.rs",
        "pub fn beta_round(amount: i64) -> i64 {\n    amount\n}\n",
    );
    let reply = tree.explore("settle");
    let focus = line_with(&reply, "focus: crates/math-kit/src/lib.rs:3 settle");
    assert!(
        focus.contains("one consumer file only (crates/app-kit/src/alpha.rs), so not shared"),
        "{focus}"
    );
    assert!(!focus.contains("shared implementation"), "{focus}");
}

#[test]
fn a_route_that_no_declaration_confirms_is_labelled_as_derived_instead_of_asserted() {
    let tree = workspace();
    tree.write_source(
        "crates/app-kit/src/internal/gamma.rs",
        "pub fn gamma_round(amount: i64) -> i64 {\n    math_kit::settle(amount, 2)\n}\n",
    );
    let reply = tree.explore("settle");
    assert!(
        reply.contains(
            "crates/app-kit/src/internal/gamma.rs: crate::internal::gamma [layout-derived only:"
        ),
        "{reply}"
    );
    assert!(
        reply.contains("declares no `mod internal` for it; read that file"),
        "{reply}"
    );
    // The derived path is still printed, and it is printed next to the fact that nothing confirmed
    // it — the reader gets a lead and the means to falsify it in the same line.
    // 推导出的路径照样印出来，而且就印在"没有东西确认它"这句话旁边——读者在同一行里既拿到线索，也拿到
    // 证伪它的手段。
    assert!(
        !reply.contains("crates/app-kit/src/alpha.rs: crate::alpha [the library root"),
        "{reply}"
    );
}

#[test]
fn two_identical_queries_return_identical_bytes_and_reparse_nothing_new() {
    let tree = workspace();
    let first = tree.explore("settle");
    let second = tree.explore("settle");
    assert_eq!(first, second, "a cold and a warm query disagreed");
    assert!(first.contains("repair evidence"), "{first}");
}

#[test]
fn a_consumer_that_appeared_after_the_first_query_is_named_with_no_restart_and_no_stale_count() {
    let tree = workspace();
    let before = tree.explore("settle");
    assert!(
        line_with(&before, "focus: crates/math-kit/src/lib.rs:3 settle")
            .contains("2 independent consumer files"),
        "{before}"
    );
    tree.write_source(
        "crates/app-kit/src/gamma.rs",
        "pub fn gamma_round(amount: i64) -> i64 {\n    math_kit::settle(amount, 16)\n}\n",
    );
    let after = tree.explore("settle");
    assert!(
        line_with(&after, "focus: crates/math-kit/src/lib.rs:3 settle")
            .contains("3 independent consumer files"),
        "{after}"
    );
    assert!(after.contains("crates/app-kit/src/gamma.rs"), "{after}");
    assert!(!before.contains("gamma_round"), "{before}");
    // Moving the file is a different fact from editing it, and the route has to follow the new path
    // rather than keep repeating the old one.
    // 搬动文件与编辑它是两件事，而路线必须跟着新路径走，而不是继续复述旧的。
    std::fs::rename(
        tree.0.join("crates/app-kit/src/gamma.rs"),
        tree.0.join("crates/app-kit/src/delta.rs"),
    )
    .unwrap();
    let moved = tree.explore("settle");
    assert!(moved.contains("crates/app-kit/src/delta.rs"), "{moved}");
    assert!(!moved.contains("crates/app-kit/src/gamma.rs"), "{moved}");
}

/// BR12: the same question with a **deeper delegation chain** and a renamed object. A consumer entry
/// is a direct caller, so the chain must not inflate the fan-in; and it must not vanish either,
/// because "how far behind this implementation does the tree reach" is what decides the blast radius.
/// BR12：同一个问题换一条**更深的委派链**与改过的对象名。消费者入口是直接调用者，因此这条链不得夸大
/// 扇入；但它也不得消失，因为"这棵树在这个实现背后伸了多远"正是决定影响半径的东西。
#[test]
fn a_deeper_delegation_chain_is_summarised_without_inflating_the_consumer_count() {
    let tree = workspace();
    tree.write_source(
        "crates/app-kit/src/facade.rs",
        "pub fn facade_round(amount: i64) -> i64 {\n    alpha_round(amount)\n}\n",
    );
    let reply = tree.explore("settle");
    assert!(
        line_with(&reply, "focus: crates/math-kit/src/lib.rs:3 settle")
            .contains("2 independent consumer files"),
        "a two-hop caller is not a third consumer entry: {reply}"
    );
    assert!(reply.contains("crate::alpha"), "{reply}");
    assert!(
        reply.contains(
            "chain    crates/app-kit/src/facade.rs:1 facade_round -> alpha_round (2 lexical hop(s)"
        ),
        "{reply}"
    );
    assert!(
        reply.contains("further delegation is a path, not a consumer entry"),
        "{reply}"
    );
    std::fs::remove_dir_all(&tree.0).unwrap();
}

#[test]
fn a_query_that_selects_nothing_projects_nothing() {
    let tree = workspace();
    let reply = tree.explore("no_such_symbol_anywhere");
    assert!(!reply.contains("repair evidence"), "{reply}");
    assert!(reply.contains("no matches"), "{reply}");
}

#[test]
fn the_projection_never_speaks_for_a_function_it_did_not_select() {
    let tree = workspace();
    let reply = tree.explore("settle");
    let projection = reply
        .split_once("repair evidence")
        .map(|(_, rest)| rest)
        .unwrap_or_default();
    assert!(
        !projection.contains("alpha_round calls settle, and no test"),
        "a protected consumer was reported as unprotected: {projection}"
    );
}

#[test]
fn package_name_reads_the_package_table_and_refuses_to_guess() {
    for (manifest, expected) in [
        ("[package]\nname = \"app-kit\"\n", Some("app-kit")),
        ("[package]\nname = 'app-kit'\n", Some("app-kit")),
        (
            "[workspace]\nmembers = [\"a\"]\n\n[package]\nname = \"app-kit\"\n",
            Some("app-kit"),
        ),
        // A `[lib] name` is a different key in a different table: answering from it would name the
        // package after its library, which is exactly the ambiguity the narrow read refuses.
        // `[lib] name` 是另一张表里的另一个键：照它作答会把包命名为它的库，而这正是这次窄读拒绝的歧义。
        ("[lib]\nname = \"app-kit\"\n", None),
        ("# [package]\n# name = \"app-kit\"\n", None),
        ("[package]\nname = 7\n", None),
        ("[package]\nversion = \"1\"\n", None),
    ] {
        assert_eq!(
            super::package_name(manifest).as_deref(),
            expected,
            "{manifest:?}"
        );
    }
}

/// The one line of a reply that starts with `prefix`, so an assertion about the focus line cannot be
/// satisfied by the same words appearing somewhere else in the projection.
/// 回复中以 `prefix` 开头的那一行；这样关于 focus 行的断言不会被投影里别处出现的同样措辞满足。
fn line_with<'a>(reply: &'a str, prefix: &str) -> &'a str {
    reply
        .lines()
        .find(|line| line.starts_with(prefix))
        .unwrap_or_else(|| panic!("no line starts with {prefix:?} in:\n{reply}"))
}

/// A projection must never turn "I could not read the registration faces" into "there are no
/// references": an empty list reads as a finding about the tree, and that is the wrong finding.
/// 投影绝不能把"读不到注册面"变成"没有参考"：空清单会被读成关于那棵树的结论，而那是错的结论。
#[test]
fn a_section_that_cannot_read_the_faces_says_unknown_instead_of_nothing() {
    let root = std::env::temp_dir().join(format!("xirang-references-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("src")).unwrap();
    // No `Cargo.toml`, so the identity namespace cannot be learned and no face can be listed.
    // 没有 `Cargo.toml`，因此学不到身份命名空间，也列不出任何面。
    let sources = vec![super::SourceFile {
        relative: "src/lib.rs".to_owned(),
        source: String::new(),
        functions: Vec::new(),
        branches: Default::default(),
    }];
    let mut faces = std::collections::BTreeMap::new();
    faces.insert("src/lib.rs".to_owned(), "root/control/button".to_owned());
    let seeds = [(0usize, 0usize)].into_iter().collect();
    let reply = super::references(&root, &sources, &seeds, &faces);
    assert!(
        reply.starts_with("references unknown:"),
        "an unreadable registry is not an empty list: {reply}"
    );
    assert!(
        reply.contains("could not be read"),
        "and it says what it could not read: {reply}"
    );
    std::fs::remove_dir_all(root).unwrap();
}

/// The **old** rule is gone, and this is the note that says where it went rather than a pin that keeps it alive.
/// **旧**规则已删，这里留一句"它去哪了"，而不是一条把它续命的钉子。
///
/// It used to assert "a sibling is a reference only when the parent **and** the kind both match", and an
/// independent review measured that rule wrong (`docs/managed-maintenance/independent-review-2026-10-17.md` §2, P1):
/// the shipped `control-button` example's Button and Slider declare the same handle contract, the same export and the
/// same flow contract, and the kind comparison excluded Slider as a different role. The maintainer's ruling replaced
/// it — 「kind 只作参考信号」 plus "the same type is not a reliable specimen either" — and the replacement's pins live
/// in `siblings::explore_references_tests`, which assert the four sentences of that ruling instead of this one.
/// 它过去断言"兄弟姐妹只有在**同父且同 kind** 时才算参考"，而独立复核量出这条规则是错的
/// （`docs/managed-maintenance/independent-review-2026-10-17.md` §2 P1）：出厂示例 `control-button` 的 Button 与 Slider
/// 声明了同一个 handle 合同、同一个导出与同一个 flow 合同，而 kind 比较把 Slider 当作不同角色排除了。维护者的裁定替换了它
/// ——「kind 只作参考信号」加上"同类型也不是可靠标本"——替换后的钉子住在 `siblings::explore_references_tests`，断言的是
/// 裁定那四句话，而不是这一句。
#[test]
fn the_old_parent_and_kind_rule_is_replaced_by_a_contract_first_judgment() {
    // The positive control the old rule got wrong, in one line: a same-parent sibling with a *different* kind and no
    // contract on either side is no longer excluded — it is a weak candidate with a reason.
    // 旧规则搞错的那个正控制，写成一行：同父、**不同** kind、两边都没有合同的兄弟不再被排除——它是一条带理由的弱候选。
    let root = std::env::temp_dir().join(format!("xirang-old-rule-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("src/control/object/left")).unwrap();
    std::fs::create_dir_all(root.join("src/control/object/note")).unwrap();
    std::fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"old-rule\"\nversion = \"0.1.0\"\nedition = \"2021\"\n[workspace]\n",
    )
    .unwrap();
    std::fs::write(root.join("src/lib.rs"), "//! host\n").unwrap();
    std::fs::write(
        root.join("src/control/control.rs"),
        "#[xirang_toolchain::soil]\npub const SOIL: xirang_toolchain::Soil = xirang_toolchain::Soil::of::<Control>()\n    .needs_registry(true)\n    .under(crate::root_node_id(crate::XIRANG_NAMESPACE));\n\npub struct Control;\n",
    )
    .unwrap();
    for (name, kind) in [("left", "Button"), ("note", "Slider")] {
        std::fs::write(
            root.join(format!("src/control/object/{name}/{name}.rs")),
            format!("#[xirang_toolchain::soil]\npub const SOIL: xirang_toolchain::Soil = xirang_toolchain::Soil::of::<{kind}>()\n    .under(crate::control::NODE_ID);\n\npub struct {kind};\n"),
        )
        .unwrap();
    }
    let namespace = crate::mcp::registry::namespace(&root).unwrap();
    let views = crate::build_method::face_views(&root, &namespace).unwrap();
    let left = views
        .iter()
        .find(|view| view.path.ends_with("/left"))
        .unwrap();
    let mut faces = std::collections::BTreeMap::new();
    faces.insert(
        "src/control/object/left/left.rs".to_owned(),
        left.path.clone(),
    );
    let sources = vec![super::SourceFile {
        relative: "src/control/object/left/left.rs".to_owned(),
        source: String::new(),
        functions: Vec::new(),
        branches: Default::default(),
    }];
    let seeds = [(0usize, 0usize)].into_iter().collect();
    let reply = super::references(&root, &sources, &seeds, &faces);
    assert!(
        reply.contains("candidate (weak)") && reply.contains("/note"),
        "a different kind is no longer an exclusion: {reply}"
    );
    assert!(
        !reply.contains("not references"),
        "and the old wording is gone with the old rule: {reply}"
    );
    let _ = std::fs::remove_dir_all(root);
}

/// A receiver call is a call: when its spelling names a method, the reply must carry the call site
/// and name it as a candidate edge, not deny that a caller exists.
/// receiver 调用也是调用：当拼写点名某个方法时，回复必须携带调用点并把它说成候选边，而不是否认存在调用者。
#[test]
fn a_receiver_call_site_is_carried_and_named_as_a_candidate() {
    let tree = Tree::new("receiver-candidate");
    tree.write_source(
        "Cargo.toml",
        "[package]\nname = \"ledger\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    tree.write_source("src/lib.rs", "pub mod model;\npub mod store;\n");
    tree.write_source(
        "src/model.rs",
        "/// One entry.\npub struct Entry {\n    /// The amount.\n    pub amount: i64,\n}\n\nimpl Entry {\n    /// Whether this entry may be posted.\n    pub fn postable(&self) -> bool {\n        self.amount != 0\n    }\n}\n",
    );
    tree.write_source(
        "src/store.rs",
        "use crate::model::Entry;\n\n/// Entries held in memory.\npub struct Store;\n\nimpl Store {\n    /// Post one entry when it is postable.\n    pub fn post(&mut self, entry: Entry) -> bool {\n        if !entry.postable() {\n            return false;\n        }\n        true\n    }\n}\n",
    );
    let reply = tree.explore("Entry::postable");
    assert!(
        reply.contains("if !entry.postable()"),
        "the call site is source the reply must carry: {reply}"
    );
    assert!(
        reply.contains("unresolved receiver call site"),
        "the site is labelled as a candidate, not a claimed edge: {reply}"
    );
    assert!(
        reply.contains("unresolved receiver call site(s) may reach it"),
        "the focus line must not deny a caller: {reply}"
    );
    assert!(
        !reply.contains("no lexical caller in this tree"),
        "the old sentence is a claim about the tree that the tree contradicts: {reply}"
    );
    assert!(
        reply.contains("may still exercise it"),
        "the protection line must not assert that nothing tests it: {reply}"
    );
}

/// The other half: when nothing names the method, the reply says **what was checked** instead of
/// claiming the tree has no callers, because the two sentences are not the same claim.
/// 另一半：没有任何东西点名这个方法时，回复要说清**查了什么**，而不是断言树里没有调用者——这两句话不是
/// 同一个断言。
#[test]
fn an_uncalled_method_says_what_was_checked_instead_of_claiming_the_tree_has_no_callers() {
    let tree = Tree::new("receiver-none");
    tree.write_source(
        "Cargo.toml",
        "[package]\nname = \"ledger\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    tree.write_source("src/lib.rs", "pub mod model;\n");
    tree.write_source(
        "src/model.rs",
        "/// One entry.\npub struct Entry;\n\nimpl Entry {\n    /// Never called in this tree.\n    pub fn postable(&self) -> bool {\n        true\n    }\n}\n",
    );
    let reply = tree.explore("Entry::postable");
    assert!(
        reply.contains("no caller in this tree: no claimed edge names it"),
        "{reply}"
    );
    assert!(!reply.contains("no lexical caller in this tree"), "{reply}");
}

/// The same rule for the other spelling the reader produces: the receiver's type is not written, so
/// the call is recorded by bare name plus a candidate list. A reader that matched only the qualified
/// spelling would still report "no caller" here.
/// 读取器产出的另一种拼写适用同一条规则：receiver 的类型没有写出来，因此调用按裸名加候选清单记录。
/// 只匹配限定拼写的读取器在这里仍会报"没有调用者"。
#[test]
fn a_receiver_without_a_written_type_is_matched_through_its_candidate_list() {
    let tree = Tree::new("receiver-candidates");
    tree.write_source(
        "Cargo.toml",
        "[package]\nname = \"ledger\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    tree.write_source("src/lib.rs", "pub mod index;\n");
    tree.write_source(
        "src/index.rs",
        "/// One bucket.\npub struct Bucket {\n    /// Inclusive low.\n    pub low: i64,\n    /// Exclusive high.\n    pub high: Option<i64>,\n}\n\nimpl Bucket {\n    /// Whether this bucket holds an amount.\n    pub fn contains(&self, amount: i64) -> bool {\n        amount >= self.low && self.high.is_none_or(|high| amount < high)\n    }\n}\n\n/// Any bucket holds it.\npub fn any_bucket(amount: i64) -> bool {\n    let list = vec![Bucket { low: 0, high: None }];\n    list.into_iter().any(|bucket| bucket.contains(amount))\n}\n",
    );
    let reply = tree.explore("Bucket::contains");
    assert!(
        reply.contains("bucket.contains(amount)"),
        "the body holding the call is source the reply carries: {reply}"
    );
    assert!(
        reply.contains("candidates: Bucket::contains"),
        "the reader's own candidate list is quoted, not paraphrased: {reply}"
    );
    assert!(
        reply.contains("unresolved receiver call site"),
        "the site is labelled as a candidate: {reply}"
    );
    assert!(
        !reply.contains("no lexical caller in this tree"),
        "a bare spelling plus a candidate is a call site, not an empty tree: {reply}"
    );
}

/// One hop past the consumer: the test that calls it by name is evidence a reader deciding what to
/// run needs, and it arrives labelled as a chain rather than as coverage.
/// 消费者再往后一跳：按名字调用它的那个测试是"该跑什么"的读者需要的证据，而它以链的形式到达，
/// 不是作为覆盖到达。
#[test]
fn a_test_that_calls_the_only_consumer_is_carried_as_a_candidate_chain() {
    let tree = Tree::new("candidate-chain");
    tree.write_source(
        "Cargo.toml",
        "[package]\nname = \"ledger\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    tree.write_source("src/lib.rs", "pub mod index;\n");
    tree.write_source(
        "src/index.rs",
        "/// One bucket.\npub struct Bucket {\n    /// Inclusive low.\n    pub low: i64,\n}\n\nimpl Bucket {\n    /// Whether this bucket holds an amount.\n    pub fn contains(&self, amount: i64) -> bool {\n        amount >= self.low\n    }\n}\n\n/// Which bucket holds it.\npub fn bucket_of(list: &[Bucket], amount: i64) -> i64 {\n    for (index, bucket) in list.iter().enumerate() {\n        if bucket.contains(amount) {\n            return index as i64;\n        }\n    }\n    -1\n}\n",
    );
    tree.write_source(
        "tests/ledger.rs",
        "use ledger::index::{Bucket, bucket_of};\n\n#[test]\nfn the_bucket_boundary_is_half_open() {\n    let list = [Bucket { low: 0 }];\n    assert_eq!(bucket_of(&list, 1), 0);\n}\n\n#[test]\nfn a_string_has_its_own_contains() {\n    let text = \"LEDGER\";\n    assert!(text.contains(\"LEDGER\"));\n}\n",
    );
    let reply = tree.explore("Bucket::contains");
    assert!(
        reply.contains("fn the_bucket_boundary_is_half_open"),
        "the test that calls the only consumer is carried: {reply}"
    );
    assert!(
        reply.contains("candidate chain"),
        "the chain is labelled as a candidate, not a route: {reply}"
    );
    // The name collision is the trap: `text.contains(..)` is `String::contains`, and the tree has
    // exactly one `contains`. Name uniqueness alone must not attribute it.
    // 名字撞车才是陷阱：`text.contains(..)` 是 `String::contains`，而树里恰好只有一个 `contains`。
    // 光凭名字唯一不能把它归属过来。
    assert!(
        !reply.contains("fn a_string_has_its_own_contains"),
        "a bare macro call in an unrelated test is not attributed by name alone: {reply}"
    );
}

/// A macro's tokens are not expressions to `syn`, so a test's `store.post(..)` inside `assert_eq!`
/// arrives as a bare spelling. The only definition with that name is the method, and the caller
/// touches that method's file, so the chain is carried — labelled, never claimed as coverage.
/// 宏的 token 对 `syn` 不是表达式，因此测试里 `assert_eq!` 内的 `store.post(..)` 以裸拼写到达。树里
/// 只有那一个同名定义，而调用方又碰到那个定义所在的文件，因此这条链被携带——带标注，永不声称覆盖。
#[test]
fn a_macro_call_in_a_test_reaches_the_only_method_with_that_name() {
    let tree = Tree::new("macro-chain");
    tree.write_source(
        "Cargo.toml",
        "[package]\nname = \"ledger\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    tree.write_source("src/lib.rs", "pub mod model;\npub mod store;\n");
    tree.write_source(
        "src/model.rs",
        "/// One entry.\npub struct Entry {\n    /// The amount.\n    pub amount: i64,\n}\n\nimpl Entry {\n    /// Whether this entry may be posted.\n    pub fn postable(&self) -> bool {\n        self.amount != 0\n    }\n}\n",
    );
    tree.write_source(
        "src/store.rs",
        "use crate::model::Entry;\n\n/// Entries held in memory.\npub struct Store;\n\nimpl Store {\n    /// An empty store.\n    pub fn new() -> Self {\n        Self\n    }\n\n    /// Post one entry when it is postable.\n    pub fn post(&mut self, entry: Entry) -> bool {\n        if !entry.postable() {\n            return false;\n        }\n        true\n    }\n}\n",
    );
    tree.write_source(
        "tests/ledger.rs",
        "use ledger::model::Entry;\nuse ledger::store::Store;\n\n#[test]\nfn a_zero_entry_is_not_posted() {\n    let mut store = Store::new();\n    assert_eq!(store.post(Entry { amount: 0 }), false);\n}\n",
    );
    let reply = tree.explore("Entry::postable");
    assert!(
        reply.contains("fn a_zero_entry_is_not_posted"),
        "the test behind the macro call is carried: {reply}"
    );
    assert!(
        reply.contains("candidate chain"),
        "and the chain says it is a candidate: {reply}"
    );
}
