//! Pins for `move`: the refusals, and the three pieces of logic a move rests on.
//! `move` 的钉子：它拒绝什么，以及一次搬动所依赖的三处逻辑。
//!
//! The **full path** (build a host, move a face, execute its behavior) lives in
//! `tools/xirang-mcp-workflow-eval`, including handwritten behavior and subtree moves. Here we pin the
//! **declaration-level** `parent:` rewrite and its refusals, the alias the tree derives, and the record
//! directory's retention rule.
//! **完整路径**由 `tools/xirang-mcp-workflow-eval` 通过 Cargo 验证，包含手写行为与子树搬迁。
//! 这里持续验证 **声明级**的 `parent:`
//! 改写与它的拒绝、树推导出的别名，以及记录目录的保留规则。

use serde_json::json;

use super::{KEEP_RECORDS, next_number, plan_parent_edit, prune, run_move, under};

/// One face's private registration tree, plus the alias re-export a real host writes.
/// 一个面的私有注册树，以及真实宿主写下的别名重导出。
///
/// Built through the public `apply` path so the fixture is the same native declaration
/// the tool later moves, not a hand-written stand-in.
/// 经公开的 `apply` 路径搭建，因此夹具就是稍后被搬的同一份原生声明，而不是手写的替身。
fn host_with_button(label: &str) -> std::path::PathBuf {
    let (root, _) = crate::mcp::apply::apply_tests::package(label);
    for request in [
        json!({"action": "add", "fields": {"module": "control", "kind": "Control", "needs_registry": true}, "apply": true}),
        json!({"action": "add", "parent": "root/control", "fields": {"module": "button", "kind": "Button", "needs_registry": true}, "apply": true}),
        json!({"action": "add", "parent": "root/control/button", "fields": {"module": "cap", "kind": "Cap"}, "apply": true}),
        json!({"action": "add", "parent": "root/control", "fields": {"module": "panel", "kind": "Panel", "needs_registry": true}, "apply": true}),
    ] {
        crate::mcp::apply::apply(&root, &request)
            .unwrap_or_else(|error| panic!("{request}: {error}"));
    }
    let library = root.join("src/lib.rs");
    let mut text = std::fs::read_to_string(&library).expect("library");
    text.push_str("pub use crate::control::object::button as button_api;\n");
    std::fs::write(&library, text).expect("alias");
    root
}

/// 「第三方身份保持」：一次宿主布局变化（搬一个自家的面）**不得**改写第三方声明的来源或字节。
/// "Third-party identity is preserved": a host layout change (moving one of our own faces) must not rewrite a
/// third-party declaration's origin or bytes.
///
/// The identity rule is what makes this worth pinning rather than assuming: `NodeId = hash(namespace, source path,
/// name)` takes the source path as an input, so a move is a **rename** for every face it touches. The external form
/// carries its own `source:` — the third-party origin — and that field belongs to the external crate's author, not to
/// us. A host that moves its own files and silently re-points somebody else's provenance has changed a signature it
/// never had the right to change.
/// 身份规则正是这件事值得钉而不值得假定的原因：`NodeId = hash(命名空间, 源码路径, 名字)` 把源码路径作为输入，因此一次
/// 搬动对它碰到的每个面都是**改名**。外部形式自带 `source:`——第三方来源——而那个字段属于外部 crate 的作者，不属于我们。
/// 一个搬自己文件、却顺手改了别人来源声明的宿主，改的是它从来没有权利改的签名。
#[test]
fn a_layout_change_never_rewrites_the_third_party_origin() {
    let root = host_with_button("third-party");
    // A face that declares its own registry and deliberately is not part of this package's generated tree.
    // 一个声明了自己注册机、并且有意不属于本包生成树的面。
    // The shipped shape: a flat file under `src/`, written in the **native** spelling with a `.source(...)` field —
    // that field is the external form's marker, and it is what the host must not rewrite.
    // 出厂形状：`src/` 下的一个扁平文件，用**原生**拼法加 `.source(...)` 字段写——那个字段就是外部形式的标记，也正是宿主
    // 不得改写的东西。
    let external = root.join("src/widget_fast.rs");
    let declaration = "#[xirang_toolchain::soil]\npub const SOIL: xirang_toolchain::Soil = xirang_toolchain::Soil::of::<WidgetFast>()\n    .source(\"widget_fast/widget_fast.rs\")\n    .under(crate::root_node_id(crate::XIRANG_NAMESPACE));\n\npub struct WidgetFast;\n";
    std::fs::write(&external, declaration).unwrap();
    let bytes_before = std::fs::read(&external).unwrap();
    // The host must see it **as external**, not as one of its own faces — otherwise the rest of this pin proves
    // nothing about third-party origins.
    // 宿主必须把它**看作外部的**，而不是自家的面——否则这道钉子余下部分证明不了任何关于第三方来源的事。
    let namespace = crate::mcp::registry::namespace(&root).unwrap();
    let (_, _, external_before) =
        crate::build_method::face_views_with_external(&root, &namespace).expect("the host reads");
    assert!(
        external_before
            .iter()
            .any(|line| line.contains("widget_fast.rs")),
        "the fixture has to be an external face the walk reports: {external_before:?}"
    );
    // Move one of our own faces, which is a real layout change for the host.
    // 搬一个自家的面——对宿主来说这是一次真实的布局变化。
    let applied = crate::mcp::apply::apply(
        &root,
        &json!({
            "action": "move",
            "node": "root/control/button",
            "to": "root/control/panel",
            "apply": true
        }),
    )
    .expect("the move applies");
    assert!(applied.contains("file(s) rewritten"), "{applied}");
    // The third-party declaration is byte-identical, `source:` included.
    // 第三方声明逐字节不变，`source:` 也在内。
    assert_eq!(
        std::fs::read(&external).unwrap(),
        bytes_before,
        "the host rewrote a third-party declaration while moving its own files"
    );
    let after = std::fs::read_to_string(&external).unwrap();
    assert!(
        after.contains("widget_fast/widget_fast.rs"),
        "the third-party origin survived the layout change: {after}"
    );
    // And the host still reports it the same way: the external list is the evidence a reader has.
    // 而宿主对它的报告也没变：外部清单就是读者手里那份证据。
    let (_, _, external_after) =
        crate::build_method::face_views_with_external(&root, &namespace).expect("the host reads");
    assert_eq!(
        external_before, external_after,
        "the move changed how the external face is reported"
    );
    // The moved face is a **different identity** now, and that is the documented rename rather than a defect —
    // recorded here so nobody "fixes" the pin by making identities path-independent.
    // 被搬的面现在是一个**不同的身份**，而这是已记录的改名而不是缺陷——记在这里，免得有人把身份改成与路径无关来"修好"这条钉子。
    assert!(
        applied.contains("subtree") || applied.contains("rewritten"),
        "the move reports what it rewrote: {applied}"
    );
    // Cleanup is not an assertion: under `--all-features` something else in this process may still be writing
    // inside the fixture (the jobs machinery), and a pin about identity bytes must not fail on that.
    // 清理不是断言：在 `--all-features` 下，进程里别的东西（jobs 机器）可能还在往夹具里写，而一条关于身份字节的
    // 钉子不该因此失败。
    let _ = std::fs::remove_dir_all(root);
}

/// The real prior failure: a move rewrites the crate-root `pub use … as button_api`
/// and reports the whole moved subtree, not only the root face.
/// 真实的历史失败：搬动改写 crate 根的 `pub use … as button_api`，并报告整个被搬子树，而不只是根面。
#[test]
fn a_move_rewrites_the_reexport_and_maps_the_whole_subtree() {
    let root = host_with_button("reexport");
    let applied = crate::mcp::apply::apply(
        &root,
        &json!({
            "action": "move",
            "node": "root/control/button",
            "to": "root/control/panel",
            "apply": true
        }),
    )
    .expect("the move applies");
    assert!(applied.contains("file(s) rewritten"), "{applied}");
    assert!(applied.contains("subtree (2 moved faces)"), "{applied}");
    let library = std::fs::read_to_string(root.join("src/lib.rs")).expect("library");
    assert!(
        library.contains("pub use crate::control::object::panel::button as button_api;"),
        "the re-export follows the move: {library}"
    );
    assert!(
        !library.contains("pub use crate::control::object::button as button_api;"),
        "the old path is gone: {library}"
    );
    let _ = std::fs::remove_dir_all(root);
}

/// A preview leaves the project bytes untouched, and a user edit to the alias made
/// after the preview is what the apply uses — the old patch is never replayed.
/// 预览不碰项目字节，而预览之后用户对别名的改动正是 apply 采用的内容——旧补丁绝不重放。
#[test]
fn a_preview_leaves_source_untouched_and_a_later_edit_is_not_overwritten() {
    let root = host_with_button("freshness");
    let library = root.join("src/lib.rs");
    let before = std::fs::read_to_string(&library).expect("library");

    let preview = crate::mcp::apply::apply(
        &root,
        &json!({
            "action": "move",
            "node": "root/control/button",
            "to": "root/control/panel",
            "apply": false
        }),
    )
    .expect("the preview runs");
    assert!(preview.contains("would move"), "{preview}");
    assert_eq!(
        std::fs::read_to_string(&library).expect("library"),
        before,
        "a preview must not change the project"
    );

    let edited = before.replace("as button_api", "as renamed_api");
    std::fs::write(&library, &edited).expect("user edit");
    crate::mcp::apply::apply(
        &root,
        &json!({
            "action": "move",
            "node": "root/control/button",
            "to": "root/control/panel",
            "apply": true
        }),
    )
    .expect("the move applies after the edit");
    let after = std::fs::read_to_string(&library).expect("library");
    assert!(
        after.contains("pub use crate::control::object::panel::button as renamed_api;"),
        "{after}"
    );
    assert!(
        !after.contains("as button_api"),
        "the user's alias name is preserved: {after}"
    );
    let _ = std::fs::remove_dir_all(root);
}

/// A reference plan whose file changed since it was computed refuses the move with
/// `needs re-plan` instead of writing stale bytes.
/// 计算后文件已变化的引用计划以 `needs re-plan` 拒绝搬动，而不是写入过期字节。
#[test]
fn a_reference_plan_bound_to_changed_bytes_needs_replan() {
    let plan = super::references::FilePlan {
        file: std::path::PathBuf::from("/tmp/xirang-needs-replan.rs"),
        before: "pub use crate::a as b;\n".to_owned(),
        edits: Vec::new(),
    };
    let refused = super::references::bound_text(&plan, "pub use crate::a as renamed;\n")
        .expect_err("changed bytes are refused");
    assert!(refused.contains("needs re-plan"), "{refused}");
}

/// Moving an ancestor also moves every claimed descendant; the claim cannot follow implicitly.
/// 搬动祖先也会搬走被认领的后代；crate 声明不能被默认为自动跟随。
#[test]
fn moving_an_ancestor_of_a_claim_is_refused() {
    let root = scratch("claimed-descendant");
    std::fs::write(root.join("add_crates.rs"),
        "pub fn add_crates() -> Shape { Shape::of(\"app\", &[Crate::named(\"widgets\").at(&[crate::panel::widget::SUBTREE])]) }")
        .unwrap();
    let face = crate::build_method::FaceView {
        id: xirang_kernel::identity::NodeId::from_bytes(b"panel"),
        stable_name: None,
        parent: xirang_kernel::identity::NodeId::from_bytes(b"root"),
        kind: "Panel".into(),
        registry_name: "panel".into(),
        module: "panel".into(),
        source: "panel/panel.rs".into(),
        source_base: crate::build_method::source_ref::SourceBase::Identity,
        owns_registry: true,
        path: "root/panel".into(),
        parent_resolved: true,
    };
    let error = super::plan_ownership_change(&root, &face, "renamed")
        .expect_err("renaming the ancestor invalidates the descendant's crate claim");
    assert!(
        error.contains("panel::widget") && error.contains("add_crates.rs"),
        "{error}"
    );
    // A broken declaration must not silently disable the boundary check either.
    // 声明损坏也不能静默关闭边界检查。
    std::fs::write(
        root.join("add_crates.rs"),
        "pub fn add_crates() -> Shape { broken }",
    )
    .unwrap();
    assert!(super::plan_ownership_change(&root, &face, "renamed").is_err());
    std::fs::remove_dir_all(root).unwrap();
}

/// A same-parent rename still changes both filesystem names, preserving handwritten bytes.
/// 同父级改名也要同时改目录和面文件名，并保留手写内容。
#[test]
fn a_renamed_move_changes_the_file_stem_without_requiring_a_new_parent() {
    let root = scratch("rename-file");
    std::fs::create_dir_all(root.join("old")).unwrap();
    let source = "crate::root_object! {\nkind: Old,\nparent: crate::root_node_id(crate::XIRANG_NAMESPACE),\n}\npub fn behavior() -> u32 { 73 }\n";
    std::fs::write(root.join("old/old.rs"), source).unwrap();
    let moved = super::Moved {
        from_dir: "old".into(),
        to_dir: "new".into(),
        source: "new/new.rs".into(),
        original_source: "old/old.rs".into(),
        module: "new".into(),
        from_package: "old".into(),
        to_package: "new".into(),
        to_physical: "new/new.rs".into(),
        covered_by_default_walk: true,
    };
    let parent = plan_parent_edit(source, super::ROOT_PARENT_SPELLING).unwrap();
    super::write::apply_move(&root, &root, &moved, &parent, None, &[], &[], &[], |_| {
        Ok(())
    })
    .unwrap();
    assert!(!root.join("old").exists());
    assert!(!root.join("new/old.rs").exists());
    assert_eq!(
        std::fs::read_to_string(root.join("new/new.rs")).unwrap(),
        source
    );
    std::fs::remove_dir_all(root).unwrap();
}

/// A helper source with the requested new stem must never be overwritten by the face.
/// 面的新文件名如果被辅助源码占用，必须在搬动前拒绝。
#[test]
fn a_face_rename_collision_is_refused_before_moving() {
    let root = scratch("rename-collision");
    std::fs::create_dir_all(root.join("old")).unwrap();
    let source = "crate::root_object! { kind: Old, parent: crate::root_node_id(crate::XIRANG_NAMESPACE), }\n";
    std::fs::write(root.join("old/old.rs"), source).unwrap();
    std::fs::write(root.join("old/new.rs"), "pub fn unrelated() {}\n").unwrap();
    let moved = super::Moved {
        from_dir: "old".into(),
        to_dir: "new".into(),
        source: "new/new.rs".into(),
        original_source: "old/old.rs".into(),
        module: "new".into(),
        from_package: "old".into(),
        to_package: "new".into(),
        to_physical: "new/new.rs".into(),
        covered_by_default_walk: true,
    };
    let parent = plan_parent_edit(source, super::ROOT_PARENT_SPELLING).unwrap();
    assert!(
        super::write::apply_move(&root, &root, &moved, &parent, None, &[], &[], &[], |_| Ok(
            ()
        ))
        .is_err()
    );
    assert!(!root.join("new").exists());
    assert_eq!(
        std::fs::read_to_string(root.join("old/old.rs")).unwrap(),
        source
    );
    assert_eq!(
        std::fs::read_to_string(root.join("old/new.rs")).unwrap(),
        "pub fn unrelated() {}\n"
    );
    std::fs::remove_dir_all(root).unwrap();
}

/// A throwaway directory, named for the **module** as well as the label.
/// 一个一次性目录，名字里既有**模块**也有标签。
///
/// The module namespace is not decoration: two test files whose labels repeat used to compute the same
/// scratch path, and each fixture's opening `remove_dir_all` deleted the directory the other test was
/// using — the symptom is a gate that goes red on a different test every run (audit `M7`, §M7.57).
/// 模块命名空间不是装饰：两个标签重复的测试文件过去会算出同一个 scratch 路径，而每个夹具开头那句
/// `remove_dir_all` 会删掉另一个测试正在用的目录——症状是同一道门禁每次红在另一条测试上
/// （审计 `M7`，§M7.57）。
fn scratch(label: &str) -> std::path::PathBuf {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let module = module_path!().replace("::", "-");
    let root = std::env::temp_dir().join(format!("xirang-{module}-{label}-{sequence}"));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("scratch root");
    root
}

pub(super) fn scratch_for_grafts() -> std::path::PathBuf {
    scratch("cross-action-grafts")
}

/// The refusal text of one call, without asking the shared `Outcome` to be `Debug`.
/// 一次调用的拒绝文本；不去要求共享的 `Outcome` 实现 `Debug`。
fn refusal(result: Result<super::Outcome, String>) -> String {
    match result {
        Ok(_) => panic!("this request was expected to be refused"),
        Err(refused) => refused,
    }
}

/// A request without `node` is refused with the shape to paste, built from the tree.
/// 不带 `node` 的请求被拒绝，并给出可粘贴的形状——形状由那棵树搭出来。
#[test]
fn a_request_without_a_node_names_the_shape() {
    let root = scratch("shape");
    let refused = refusal(run_move(&root, &root, false, &json!({"to": "root"})));
    assert!(
        refused.contains("`node`") && refused.contains("\"action\":\"move\""),
        "the refusal names the key and the accepted shape: {refused}"
    );
    // The other direction: `to` is its own key, and the sentence has to name that one when it is the
    // missing half.
    // 反向：`to` 是另一个键，缺的是它时那句话必须点名它。
    let refused = refusal(run_move(&root, &root, false, &json!({"node": "root/a"})));
    assert!(
        refused.contains("`to`"),
        "the refusal names the missing key: {refused}"
    );
}

/// A face declares exactly one parent, so a file that spells `parent:` twice is refused by name.
/// 一个面只声明一个父级，因此把 `parent:` 拼了两次的文件会被点名拒绝。
#[test]
fn the_parent_field_is_parsed_and_never_guessed() {
    let spelling = "crate::input::NODE_ID";
    let one = "crate::input_object! {\n    kind: Tile,\n    parent: crate::board::NODE_ID,\n}\n";
    let edit = plan_parent_edit(one, spelling).expect("one parent line is enough");
    assert!(
        edit.before.contains("crate::board::NODE_ID"),
        "the line as it stands is kept"
    );
    assert!(
        edit.after.contains("crate::input::NODE_ID"),
        "the replacement is the new parent's typed spelling: {}",
        edit.after
    );

    let none = "crate::input_object! {\n    kind: Tile,\n}\n";
    let refused = plan_parent_edit(none, spelling).expect_err("no parent line is refused");
    assert!(
        refused.contains("no explicit parent"),
        "the refusal names the invalid field: {refused}"
    );

    let twice =
        "crate::root_object! { kind: Leaf, parent: crate::a::NODE_ID, parent: crate::b::NODE_ID, }";
    let refused = plan_parent_edit(twice, spelling).expect_err("two are refused");
    assert!(
        refused.contains("parent"),
        "the refusal names the invalid field: {refused}"
    );

    // A spelling the tree never writes is refused rather than rewritten: this action knows the two
    // spellings `renderer` emits, and a third one would be a guess.
    // 这棵树从不写的拼法被拒绝而不是被改写：本动作认识 `renderer` 发出的那两种，第三种就是猜。
    let strange = "crate::root_object! { kind: Tile, parent: some_macro!(Tile), }";
    let refused = plan_parent_edit(strange, spelling).expect_err("an unknown shape is refused");
    assert!(
        refused.contains("neither a typed"),
        "the refusal says which shapes it knows: {refused}"
    );
}

/// The root's parent line is the other spelling the tree writes, and the root is a destination.
/// 根的 parent 行是这棵树写的另一种拼法，而根本身也是一个目的地。
#[test]
fn the_root_is_a_destination_with_its_own_spelling() {
    let typed = "crate::root_object! { kind: Tile, parent: crate::board::NODE_ID, }";
    let edit =
        plan_parent_edit(typed, super::ROOT_PARENT_SPELLING).expect("the root is a destination");
    assert_eq!(
        edit.after.trim(),
        "parent: crate::root_node_id(crate::XIRANG_NAMESPACE),",
        "moving to the root writes the root's own spelling, keeping the comma"
    );
    let already_root =
        "crate::root_object! { kind: Tile, parent: crate::root_node_id(crate::XIRANG_NAMESPACE), }";
    plan_parent_edit(already_root, super::ROOT_PARENT_SPELLING)
        .expect("the root's spelling is accepted as a source too");
}

/// Whole segments only: `board_faster` is not under `board`.
/// 只按整段比较：`board_faster` 不在 `board` 之下。
#[test]
fn a_claim_is_matched_by_segment_not_by_string_prefix() {
    assert!(under("board", "board"));
    assert!(under("board::object::tile2", "board"));
    assert!(
        !under("board_faster", "board"),
        "a longer segment that merely starts the same way is a different subtree"
    );
    assert!(!under("boardroom::tile", "board"));
}

/// The record directory keeps the most recent five, and numbers the next one after the highest.
/// 记录目录只保留最近五条，并把下一条编号取在最大值之后。
#[test]
fn the_records_keep_the_most_recent_five() {
    let root = scratch("records");
    let directory = root.join("moves");
    for number in 1..=7u32 {
        std::fs::create_dir_all(directory.join(number.to_string())).expect("record directory");
    }
    assert_eq!(
        next_number(&directory),
        8,
        "the next number follows the highest"
    );
    prune(&directory, KEEP_RECORDS);
    let mut left: Vec<u32> = std::fs::read_dir(&directory)
        .expect("records")
        .flatten()
        .filter_map(|entry| entry.file_name().to_string_lossy().parse::<u32>().ok())
        .collect();
    left.sort_unstable();
    assert_eq!(
        left,
        vec![3, 4, 5, 6, 7],
        "the oldest were dropped, not the newest"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// A04: a known `include_str!` path into the moved directory follows the move, a
/// matching ordinary string and comment do not, and a relative import that could
/// name the move is reported as unresolved.
/// A04：指向被搬目录的已知 `include_str!` 路径跟随搬动，相同的普通字符串与注释不跟随，而可能
/// 命名该搬动的相对导入被报为未解决。
#[test]
fn a_move_rewrites_include_str_and_reports_relative_ambiguity() {
    let root = host_with_button("include");
    let button_dir = root.join("src/control/object/button");
    std::fs::create_dir_all(&button_dir).expect("data directory");
    std::fs::write(button_dir.join("data.txt"), "stable").expect("data file");
    let library = root.join("src/lib.rs");
    let mut text = std::fs::read_to_string(&library).expect("library");
    text.push_str("// include_str!(\"control/object/button/data.txt\")\n");
    text.push_str("pub const SAME: &str = \"control/object/button/data.txt\";\n");
    text.push_str("pub const DATA: &str = include_str!(\"control/object/button/data.txt\");\n");
    text.push_str("use super::control::object::button::run;\n");
    std::fs::write(&library, &text).expect("include fixture");

    let applied = crate::mcp::apply::apply(
        &root,
        &json!({
            "action": "move",
            "node": "root/control/button",
            "to": "root/control/panel",
            "apply": true
        }),
    )
    .expect("the move applies");
    assert!(applied.contains("unresolved obligations:"), "{applied}");
    assert!(applied.contains("button"), "{applied}");
    let after = std::fs::read_to_string(&library).expect("library");
    assert!(
        after.contains("include_str!(\"control/object/panel/button/data.txt\")"),
        "the file reference follows the move: {after}"
    );
    assert!(
        after.contains("// include_str!(\"control/object/button/data.txt\")"),
        "the comment is untouched: {after}"
    );
    assert!(
        after.contains("pub const SAME: &str = \"control/object/button/data.txt\";"),
        "the ordinary string is untouched: {after}"
    );
    let _ = std::fs::remove_dir_all(root);
}

/// The digest a preview prints is what an apply checks: changed bytes make that plan stale.
/// 预览打印的摘要就是应用校验的东西：字节一变，那份计划就过期。
#[test]
fn a_plan_from_a_preview_is_refused_after_the_bytes_change() {
    let root = host_with_button("stale-plan");
    let request = json!({
        "action": "move",
        "node": "root/control/button",
        "to": "root/control/panel",
        "apply": false
    });
    let preview = crate::mcp::apply::apply(&root, &request).expect("the preview runs");
    let plan = preview
        .lines()
        .find_map(|line| line.trim().strip_prefix("plan "))
        .unwrap_or_else(|| panic!("the preview prints its plan: {preview}"))
        .to_owned();

    let library = root.join("src/lib.rs");
    let before = std::fs::read_to_string(&library).expect("library");
    std::fs::write(
        &library,
        format!("{before}\n// touched between preview and apply\n"),
    )
    .expect("edit");
    let error = crate::mcp::apply::apply(
        &root,
        &json!({
            "action": "move",
            "node": "root/control/button",
            "to": "root/control/panel",
            "apply": true,
            "plan": plan
        }),
    )
    .expect_err("the stale plan is refused");
    assert!(error.contains("no longer describes this move"), "{error}");
    assert_eq!(
        std::fs::read_to_string(&library).expect("library"),
        format!("{before}\n// touched between preview and apply\n"),
        "a refused apply writes nothing"
    );

    // A fresh preview prints a plan that the unchanged tree accepts.
    // 重新预览打印的计划会被未变的树接受。
    let fresh = crate::mcp::apply::apply(&root, &request).expect("fresh preview");
    let fresh_plan = fresh
        .lines()
        .find_map(|line| line.trim().strip_prefix("plan "))
        .expect("fresh plan")
        .to_owned();
    crate::mcp::apply::apply(
        &root,
        &json!({
            "action": "move",
            "node": "root/control/button",
            "to": "root/control/panel",
            "apply": true,
            "plan": fresh_plan
        }),
    )
    .expect("the fresh plan applies");
    let _ = std::fs::remove_dir_all(root);
}

/// A plan also stops describing the move when the destination changes.
/// 目的地变了，计划同样不再描述这次搬动。
#[test]
fn a_plan_is_refused_for_another_destination() {
    let root = host_with_button("stale-destination");
    let preview = crate::mcp::apply::apply(
        &root,
        &json!({
            "action": "move",
            "node": "root/control/button",
            "to": "root/control/panel",
            "apply": false
        }),
    )
    .expect("preview");
    let plan = preview
        .lines()
        .find_map(|line| line.trim().strip_prefix("plan "))
        .expect("plan")
        .to_owned();
    let error = crate::mcp::apply::apply(
        &root,
        &json!({
            "action": "move",
            "node": "root/control/button",
            "to": "root/control",
            "apply": true,
            "plan": plan
        }),
    )
    .expect_err("a plan for another destination is refused");
    assert!(error.contains("no longer describes this move"), "{error}");
    let _ = std::fs::remove_dir_all(root);
}

/// A declaration that disappears is reported as absent; no same-named file elsewhere is adopted.
/// 消失的声明被报成不存在；别处的同名文件不会被顶替进来。
#[test]
fn a_deleted_declaration_is_reported_as_absent() {
    let root = host_with_button("missing-source");
    let declaration = root.join("src/control/object/button/button.rs");
    std::fs::remove_file(&declaration).expect("remove the declaration");
    // A decoy at the package-relative spelling exists and must not stand in for the record.
    // 包内相对拼写处放一个诱饵，它绝不能顶替那条记录。
    let decoy = root.join("control/object/button");
    std::fs::create_dir_all(&decoy).expect("decoy directory");
    std::fs::write(
        decoy.join("button.rs"),
        "crate::root_object! { kind: Decoy }\n",
    )
    .expect("decoy");
    let error = crate::mcp::apply::apply(
        &root,
        &json!({
            "action": "move",
            "node": "root/control/button",
            "to": "root/control/panel",
            "apply": false
        }),
    )
    .expect_err("a missing declaration is refused");
    assert!(
        error.contains("no face in this tree has the path"),
        "{error}"
    );
    assert!(!error.contains("Decoy"), "{error}");
    let _ = std::fs::remove_dir_all(root);
}

/// A write failure inside a **Rust** move restores the tree — including the directory the move
/// renamed and the parent directory it created. This is the half the transaction used to leave to a
/// second block of compensation code in the caller; the pin is here so "one recoverable set" is
/// measured on this branch too, not only on the C++ one.
/// 一次 **Rust** 搬动里的写入失败把树恢复回去——包括这次搬动改名的目录与它创建的父目录。这正是事务过去留给
/// 调用方第二段补偿代码的那一半；钉子放在这里，是为了"同一个可恢复集合"在这一支上也被量到，而不只是在 C++
/// 那一支上。
#[test]
fn a_partial_write_of_a_rust_move_restores_the_tree_and_the_directory() {
    use crate::run_method::authoring::write_transaction::fault;

    let root = host_with_button("rust-partial");
    let before = snapshot(&root);
    let before_dirs = directories(&root);
    // The moved declaration is written under its **old** file name first (the directory was renamed;
    // the file rename follows inside the transaction), so that is the path the fault seam sees.
    // 被搬的声明先以**旧**文件名写入（目录已改名；文件改名在事务内随后发生），因此故障接缝看到的是这条路径。
    let target = root.join("src/control/button_moved/button.rs");
    fault::arm(&target, 5);

    let error = crate::mcp::apply::apply(
        &root,
        &json!({"action":"move","node":"root/control/button","to":"root/control",
                "name":"button_moved","apply":true}),
    )
    .expect_err("the injected write failure refuses the move");
    assert!(error.contains("injected failure"), "{error}");
    assert!(
        error.contains("restored"),
        "the refusal says what happened to the bytes: {error}"
    );
    assert_eq!(snapshot(&root), before, "every byte is where it was");
    assert_eq!(
        directories(&root),
        before_dirs,
        "the renamed and created directories are gone"
    );
    assert!(
        !root.join("src/control/button_moved").exists(),
        "the destination tree did not stay"
    );

    crate::mcp::apply::apply(
        &root,
        &json!({"action":"move","node":"root/control/button","to":"root/control",
                "name":"button_moved","apply":true}),
    )
    .expect("the retry lands once nothing is armed");
    let _ = std::fs::remove_dir_all(root);
}

/// Every file under `root`, keyed by relative path.
/// `root` 下的每个文件，按相对路径为键。
fn snapshot(root: &std::path::Path) -> std::collections::BTreeMap<String, Vec<u8>> {
    let mut files = std::collections::BTreeMap::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(directory) = pending.pop() {
        for entry in std::fs::read_dir(&directory).expect("fixture tree") {
            let entry = entry.expect("fixture entry");
            let path = entry.path();
            // Build output is not part of "the tree": it changes between runs and a rollback does
            // not (and must not) undo it.
            // 构建产物不属于"这棵树"：它每次运行都会变，而回滚不恢复（也不该恢复）它。
            if path.file_name().is_some_and(|name| name == "target") {
                continue;
            }
            if path.is_dir() {
                pending.push(path);
            } else if path.is_file() {
                files.insert(
                    path.strip_prefix(root)
                        .expect("inside root")
                        .to_string_lossy()
                        .replace('\\', "/"),
                    std::fs::read(&path).expect("fixture file"),
                );
            }
        }
    }
    files
}

/// Every directory under `root`, keyed by relative path.
/// `root` 下的每个目录，按相对路径为键。
fn directories(root: &std::path::Path) -> Vec<String> {
    let mut found = Vec::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(directory) = pending.pop() {
        for entry in std::fs::read_dir(&directory).expect("fixture tree") {
            let entry = entry.expect("fixture entry");
            let path = entry.path();
            if path.file_name().is_some_and(|name| name == "target") {
                continue;
            }
            if path.is_dir() {
                found.push(
                    path.strip_prefix(root)
                        .expect("inside root")
                        .to_string_lossy()
                        .replace('\\', "/"),
                );
                pending.push(path);
            }
        }
    }
    found.sort();
    found
}

/// A move inside a generated partition package is refused before planning: its bytes would be
/// overwritten by the next regeneration, so a success would be a lost edit.
/// 生成分区包内的 move 在规划前拒绝：它的字节会被下一次重生成覆盖，成功等于丢掉修改。
#[test]
fn a_move_inside_a_generated_partition_package_is_refused() {
    let root = std::env::temp_dir().join(format!(
        "xirang-generated-package-move-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("dir");
    std::fs::write(
        root.join("Cargo.toml"),
        "# xirang-generation: 1\n[package]\nname = \"generated\"\nversion = \"0.1.0\"\n",
    )
    .expect("manifest");
    let error = match run_move(
        &root,
        &root,
        false,
        &json!({"node": "root/panel/button", "to": "root"}),
    ) {
        Ok(outcome) => panic!("a generated package accepted the move: {}", outcome.message),
        Err(error) => error,
    };
    assert!(error.contains("generated partition package"), "{error}");
    assert!(error.contains("xirang crates --write"), "{error}");
    assert!(error.contains("Nothing changed"), "{error}");
    let _ = std::fs::remove_dir_all(&root);
}
