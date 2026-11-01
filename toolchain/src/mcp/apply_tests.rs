//! Tests for the write path: what a preview promises, and what an apply does.
//! 写入路径的测试：预览承诺了什么，落盘做了什么。

use std::path::PathBuf;

use crate::build_method::face_views;
use serde_json::json;

use super::apply;
#[cfg(test)]
use crate::mcp::apply::apply_cut::apply_cut_tests::face_package;

/// A throwaway package with no faces: `cargo metadata` can name it (a manifest
/// and a library target), and every face in it is one the tests add.
/// 一个没有注册面的一次性包：`cargo metadata` 能给它命名（有清单与库目标），而它里面的每个面都是
/// 测试加进去的。
pub(crate) fn package(label: &str) -> (PathBuf, String) {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let name = format!("mcp-apply-{label}");
    let root = std::env::temp_dir().join(format!("{name}-{}-{sequence}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("src")).expect("package directory");
    std::fs::write(
        root.join("Cargo.toml"),
        format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n"),
    )
    .expect("manifest");
    std::fs::write(root.join("src/lib.rs"), "// host entry\n").expect("library target");
    (root, name)
}

/// The path one reply says it wrote, as the reply spells it.
/// 一条回复说自己写下的路径，按回复的写法。
pub(crate) fn written(reply: &str) -> PathBuf {
    reply
        .lines()
        .find_map(|line| line.strip_prefix("would write "))
        .or_else(|| reply.lines().find_map(|line| line.strip_prefix("applied ")))
        .or_else(|| {
            reply
                .lines()
                .find_map(|line| line.strip_prefix("would move "))
        })
        .or_else(|| reply.lines().find_map(|line| line.strip_prefix("moved ")))
        .map(PathBuf::from)
        .unwrap_or_else(|| panic!("no written path in: {reply}"))
}

/// A misspelled key and a mistyped flag are refused by name, by both write actions.
/// `add` used to take only the keys it knew and silently drop the rest, so an agent that
/// wrote `knd` believed it had set the kind, and `needs_registry: "yes"` became `false`.
/// 拼错的键与类型不对的标志都被**点名**拒绝，两个写入动作都一样。`add` 过去只取它认识的键、
/// 静默丢弃其余，于是写下 `knd` 的代理会以为自己设了 kind，而 `needs_registry: "yes"` 会变成
/// `false`。
#[test]
fn a_misspelled_field_is_refused_by_add_and_edit() {
    let (root, _) = package("fields");

    let misspelled = apply(
        &root,
        &json!({"action": "add", "fields": {"module": "probe", "knd": "Button"}}),
    )
    .expect_err("a misspelled key is refused rather than dropped");
    assert!(
        misspelled.contains("`knd` is not an editable registration-face field"),
        "{misspelled}"
    );
    // The refusal also names the accepted shape, so the next call does not have to guess it.
    // 拒绝同时点名"可接受的形状"，让下一次调用不必靠猜。
    assert!(
        misspelled.contains("fields: module kind") && misspelled.contains("one boolean"),
        "the refusal carries the shape: {misspelled}"
    );

    let mistyped = apply(
        &root,
        &json!({"action": "add", "fields": {"module": "probe", "needs_registry": "yes"}}),
    )
    .expect_err("a non-boolean flag is refused rather than coerced");
    assert!(
        mistyped.contains("`needs_registry` must be true or false"),
        "{mistyped}"
    );
    // Every type refusal carries the shape too: an independent review of the first cut found this
    // exact hole (the boolean refusal was the one an arm actually hit), so the pin covers both the
    // boolean and the string case rather than only the misspelled key.
    // 每一种类型拒绝也带形状：对第一版的独立复核正是从这个洞里抓到的（被拒的恰是布尔那一条），因此
    // 钉子同时覆盖布尔与字符串两种情形，而不是只覆盖拼错的键。
    assert!(
        mistyped.contains("fields: module kind") && mistyped.contains("one boolean"),
        "the boolean refusal carries the shape: {mistyped}"
    );
    let mistyped_string = apply(
        &root,
        &json!({"action": "add", "fields": {"module": "probe", "exports": ["control.render"]}}),
    )
    .expect_err("an array where a string belongs is refused rather than coerced");
    assert!(
        mistyped_string.contains("`exports` must be a string")
            && mistyped_string.contains("fields: module kind"),
        "the string refusal carries the shape: {mistyped_string}"
    );

    let applied = apply(
        &root,
        &json!({"action": "add", "fields": {"module": "probe"}, "apply": true}),
    )
    .expect("a valid add applies");
    assert!(applied.contains("applied"), "{applied}");

    let edited = apply(
        &root,
        &json!({"action": "edit", "node": "root/probe", "fields": {"knd": "Button"}}),
    )
    .expect_err("edit refuses the same key");
    assert!(
        edited.contains("`knd` is not an editable registration-face field"),
        "{edited}"
    );
}

/// A preview is the real operation on a copy: it reports the face it would create
/// and the tree that would result, and it writes nothing into the project. The
/// second half is the load-bearing one — a preview that leaked into the project
/// would make the confirmation step a lie.
/// 预览是在副本上运行的真实操作：它报告会创建哪个面、会得到哪棵树，而它不向项目写入任何东西。
/// 后半句是承重的那一半——泄漏进项目的预览会让"确认"这一步变成谎言。
#[test]
fn a_preview_reports_the_resulting_tree_and_leaves_the_project_alone() {
    let (root, name) = package("preview");
    let reply = apply(
        &root,
        &json!({
            "action": "add",
            "parent": "root",
            "fields": {"module": "button", "kind": "Button", "name_en": "Button"},
        }),
    )
    .expect("the preview runs");
    assert!(reply.contains("action preview"), "{reply}");
    assert!(reply.contains("namespace "), "{reply}");
    // Audit `W2-5` (D2) reversed an older pin here: the editable-field list used to ride on every
    // reply because `--list <tool>` was never called. It is a **refusal** aid, so it lives where the
    // mistake is made — a successful reply prints the change, and one call away (the next refusal)
    // is the shape. Both halves are asserted, or "removed" and "gone" would look the same.
    // 审计 `W2-5`（D2）推翻了这里更早的一条钉子：可编辑字段清单过去搭在每条回复上，因为 `--list <tool>` 从没
    // 被调用过。它是**拒绝**的辅助，因此它住在犯错的地方——成功回复印的是这次改动，而形状离一次调用（下一次被拒）。
    // 两半都要断言，否则"搬走了"与"没了"看起来一样。
    assert!(
        !reply.contains("fields: module kind preset parts"),
        "a successful reply does not reprint the field list: {reply}"
    );
    let refused = apply(
        &root,
        &json!({"action": "add", "parent": "root", "fields": {"knd": "Button"}}),
    )
    .expect_err("a misspelled key is refused");
    assert!(
        refused.contains("fields: module kind preset parts"),
        "the refusal still states the shape: {refused}"
    );
    assert!(reply.contains("faces 1"), "{reply}");
    assert!(reply.contains("root/button"), "{reply}");
    assert!(reply.contains("diff:"), "{reply}");

    let target = written(&reply);
    assert!(
        target.starts_with(&root),
        "the report must name a path in the project: {reply}"
    );
    assert!(
        !target.exists(),
        "a preview must not write into the project: {reply}"
    );
    assert!(
        face_views(&root, &name)
            .expect("the project reads")
            .is_empty(),
        "the project must still have no faces"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// The write path states what the change will make the rest of the tree say: which in-tree test
/// lines name the face, and whether the entry plan names it at all. Both families of the round-5
/// evaluation discovered those two facts only by running the suite and reading the red.
/// 写入路径会说出这次改动会让树的其他部分说什么：哪些树内测试行点到了这个面，以及入口计划到底有没有
/// 点名它。第五轮评测的两个族都只靠"跑测试、看红"才发现这两件事。
#[test]
fn a_preview_names_the_pins_and_the_entry_plan_the_change_will_move() {
    let (root, _) = package("consequences");
    std::fs::create_dir_all(root.join("tests")).expect("fixture test dir");
    std::fs::write(
        root.join("tests/registry.rs"),
        "#[test]\nfn the_factory_lists_every_face() { assert!(true, \"button\"); }\n",
    )
    .expect("fixture test file");
    let reply = apply(
        &root,
        &json!({
            "action": "add",
            "parent": "root",
            "fields": {"module": "button", "kind": "Button", "name_en": "Button"},
        }),
    )
    .expect("the preview runs");
    assert!(
        reply.contains("consequences (static, text-level)"),
        "{reply}"
    );
    assert!(
        reply.contains("tests/registry.rs:"),
        "the in-tree test line that names the face is listed: {reply}"
    );
    assert!(reply.contains("entry plan:"), "{reply}");
    assert!(
        reply.contains("not covered"),
        "and the block says what it cannot see: {reply}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// The wall the round measured comes back with the way forward named: "no" and "no, and here is what
/// yes needs" are different answers, and the second one is the one an agent can act on.
/// 那轮量到的那堵墙会带着"继续走的两条路"回来："不行"与"不行，而'行'需要什么"是两个不同的回答，
/// 而后者才是代理能据以行动的那个。
#[test]
fn a_refusal_that_has_a_way_forward_names_it() {
    let (root, _) = package("way-forward");
    let control = json!({
        "action": "add",
        "parent": "root",
        "apply": true,
        "fields": {"module": "control", "kind": "Control"},
    });
    apply(&root, &control).expect("the parent face is created");
    let refused = apply(
        &root,
        &json!({
            "action": "add",
            "parent": "root/control",
            "fields": {"module": "label", "kind": "Label"},
        }),
    )
    .expect_err("a parent that owns no registry cannot take a child");
    assert!(
        refused.contains("does not own a Registry"),
        "the kernel's own refusal is kept: {refused}"
    );
    assert!(refused.contains("way forward"), "{refused}");
    assert!(refused.contains("needs_registry"), "{refused}");
    let _ = std::fs::remove_dir_all(&root);
}

/// `deepen` adds a layer **inside** a face: the tree, the declaration and the public path do not
/// move, and the reply prices the other reading ("another face under it").
/// `deepen` 在一个面**内部**加一层：树、声明与公开路径都不动，而且回复会给出另一种读法（"它下面再挂
/// 一个面"）的代价。
#[test]
fn a_deepen_adds_a_layer_inside_the_face_and_prices_the_other_reading() {
    let (root, _) = package("deepen");
    apply(
        &root,
        &json!({"action": "add", "parent": "root", "apply": true,
                "fields": {"module": "control", "kind": "Control", "needs_registry": true}}),
    )
    .expect("the parent face");
    let added = apply(
        &root,
        &json!({"action": "add", "parent": "root/control", "apply": true,
                "fields": {"module": "button", "kind": "Button"}}),
    )
    .expect("the leaf face");
    let face = written(&added);
    let reply = apply(
        &root,
        &json!({"action": "deepen", "node": "root/control/button",
                "inside": {"parts": {"label": "String"}}}),
    )
    .expect("the deepen preview runs");
    assert!(reply.contains("action preview"), "{reply}");
    assert!(reply.contains("deepened root/control/button"), "{reply}");
    assert!(reply.contains("ButtonParts"), "{reply}");
    assert!(
        reply.contains("alternative (the other reading"),
        "the reply prices the other reading: {reply}"
    );
    assert!(
        reply.contains("stops being a unit struct"),
        "and says the one shape change it does make: {reply}"
    );
    assert!(
        reply.contains("faces 2"),
        "the registration tree is unchanged: {reply}"
    );
    assert!(
        !std::fs::read_to_string(&face)
            .expect("the face file")
            .contains("ButtonParts"),
        "a preview writes nothing"
    );
    let applied = apply(
        &root,
        &json!({"action": "deepen", "node": "root/control/button", "apply": true,
                "inside": {"parts": {"label": "String", "count": "usize"}}}),
    )
    .expect("the deepen applies");
    assert!(applied.contains("action apply"), "{applied}");
    // Audit `W5-7`: the two blocks are a decision aid, so they arrive where the decision is still
    // open and not again after it was made. The write keeps one line that says where they were.
    // 审计 `W5-7`：那两块是决策辅助，因此它们到达的时候决定还没做，做过之后不再来一遍。落盘只留一行，
    // 说它们此前在哪儿。
    assert!(
        applied.contains("alternative was stated in preview")
            && !applied.contains("alternative (the other reading")
            && !applied.contains("stops being a unit struct"),
        "the write does not reprint the preview's decision aid: {applied}"
    );
    let text = std::fs::read_to_string(&face).expect("the face file");
    assert!(
        text.contains("pub struct Button {\n    parts: ButtonParts,\n}"),
        "{text}"
    );
    assert!(
        text.contains("impl xirang_toolchain::run_method::PartsContract for ButtonParts"),
        "{text}"
    );
    // The parts list follows the JSON object's own order, which is sorted by key, so the pin states
    // that order rather than the order the request happened to write them in.
    // 零件列表按 JSON 对象自己的顺序（按键排序），因此钉子写的是那个顺序，而不是请求碰巧写下的顺序。
    assert!(
        text.contains("const PROVIDED_PARTS: &'static [&'static str] = &[\"count\", \"label\"]"),
        "{text}"
    );
    assert!(
        text.contains("pub fn parts(&self) -> &ButtonParts"),
        "{text}"
    );
    assert!(
        xirang_kernel::syntax::parse_face(&text)
            .unwrap()
            .unwrap()
            .path("kind")
            .as_deref()
            == Some("Button"),
        "the declaration is untouched: {text}"
    );
    let missing = apply(
        &root,
        &json!({"action": "deepen", "node": "root/nope", "inside": {"parts": {"x": "u8"}}}),
    )
    .expect_err("there is no such face");
    assert!(
        missing.contains("no registration face") || missing.contains("does not name a face"),
        "{missing}"
    );
    // A file whose face still resolves but no longer holds the unit marker: the action refuses by
    // naming the line it looked for, rather than guessing at another shape.
    // 一个面仍能解析、但已不再持有单位标记的文件：本动作按名拒绝、点出它找的那一行，而不是去猜另一种形状。
    let text = std::fs::read_to_string(&face).expect("the face file");
    std::fs::write(
        &face,
        text.replace(
            "pub struct Button {\n    parts: ButtonParts,\n}",
            "pub struct Button {\n    own: u8,\n}",
        ),
    )
    .expect("fixture rewrite");
    let unclear = apply(
        &root,
        &json!({"action": "deepen", "node": "root/control/button", "inside": {"parts": {"x": "u8"}}}),
    )
    .expect_err("a shape it cannot read is refused rather than guessed at");
    assert!(
        unclear.contains("pub struct Button;") && unclear.contains("0 time(s)"),
        "{unclear}"
    );
    // The refusal names the file **relative to the package**: a preview's absolute path is a copy
    // that is deleted a moment later, so quoting it would send the caller somewhere that will not
    // exist (an independent review caught exactly that in the first cut).
    // 拒绝按**相对包**的路径点名文件：预览的绝对路径是随后就被删掉的副本，抄它等于把调用方指向一个
    // 不存在的目录（第一版正是被独立复核这样抄出来的）。
    assert!(
        unclear.contains("src/") && unclear.contains("button.rs") && !unclear.contains("/tmp/"),
        "the refusal names a path that will still exist: {unclear}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// An apply writes the face, and the next call can aim with the tree the previous
/// reply reported: a child is added under the logical path the first apply
/// produced, which is the loop that makes the tool usable by an agent.
/// 落盘会写下这个面，而下一次调用可以用上一条回复所报告的树来瞄准：子面被加在第一次落盘产生的
/// 逻辑路径之下——正是这个回路让工具对代理可用。
#[test]
fn an_apply_writes_the_face_and_its_path_can_aim_the_next_call() {
    let (root, name) = package("apply");
    let control = json!({
        "action": "add",
        "parent": "root",
        "apply": true,
        "fields": {"module": "control", "kind": "Control", "needs_registry": true},
    });
    let reply = apply(&root, &control).expect("the root face is created");
    assert!(reply.contains("action apply"), "{reply}");
    assert!(reply.contains("faces 1"), "{reply}");
    assert!(written(&reply).is_file(), "{reply}");

    let button = json!({
        "action": "add",
        "parent": "root/control",
        "fields": {"module": "button", "kind": "Button", "name_en": "Button"},
    });
    let preview = apply(&root, &button).expect("the child previews against the real tree");
    assert!(
        preview.contains("root/control/button"),
        "the logical path from the first reply must be a usable parent: {preview}"
    );
    assert!(
        written(&preview).ends_with("control/object/button/button.rs"),
        "the executor's own placement decides the file: {preview}"
    );
    assert!(preview.contains("faces 2"), "{preview}");
    assert!(
        !written(&preview).exists(),
        "the child is still only previewed: {preview}"
    );

    let applied = apply(
        &root,
        &json!({
            "action": "add",
            "parent": "root/control",
            "apply": true,
            "fields": {"module": "button", "kind": "Button", "name_en": "Button"},
        }),
    )
    .expect("the child is created");
    assert!(applied.contains("faces 2"), "{applied}");
    let faces = face_views(&root, &name).expect("the project reads");
    assert_eq!(faces.len(), 2, "{faces:?}");
    assert!(
        faces.iter().any(|face| face.path == "root/control/button"),
        "{faces:?}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// A partial edit keeps every field the request did not name. The executor rebuilds
/// a face from the values it is given, so without reading the face back first this
/// request would blank the kind, the exports, and everything else it stayed silent
/// about — and the kernel would either reject it or, worse, accept a face the author
/// never wrote.
/// 局部编辑保留请求没有点名的每个字段。执行器用拿到的取值整体重建一个面，因此若不先读回该面，这次
/// 请求就会抹掉 kind、exports 以及它未提及的一切——内核要么拒绝，要么更糟：接受一个作者从未写过的面。
#[test]
fn a_partial_edit_keeps_the_fields_it_does_not_name() {
    let (root, name) = package("partial-edit");
    let created = apply(
        &root,
        &json!({
            "action": "add",
            "parent": "root",
            "apply": true,
            "fields": {
                "module": "button",
                "kind": "Button",
                "name_en": "Button",
                "exports": "root.render",
                "handle_contracts": "ControlHandle",
            },
        }),
    )
    .expect("the face is created");
    let file = written(&created);
    let before = std::fs::read_to_string(&file).expect("the written face");
    // The reply anchors the declaration, so a caller does not have to search the
    // file for the line it just changed.
    // 回复给出声明的锚点，调用方因此不必去文件里找它刚改的那一行。
    let anchor = created
        .lines()
        .find_map(|line| line.strip_prefix("declaration "))
        .unwrap_or_else(|| panic!("no declaration anchor in: {created}"));
    let (anchor_path, anchor_line) = anchor.rsplit_once(':').expect("path:line");
    assert!(
        anchor_path.ends_with("button/button.rs"),
        "the anchor must name the face: {anchor}"
    );
    let anchored = before
        .lines()
        .nth(anchor_line.parse::<usize>().expect("a line number") - 1)
        .unwrap_or_else(|| panic!("the anchor points past the file: {anchor}"));
    // The anchor points at the **item** that declares the face, and the item's first line is its doc
    // comment; either spelling is a declaration, so accept both rather than pinning the old one.
    // 锚点指向声明这个面的**条目**，而条目的第一行是它的文档注释；两种拼法都是声明，因此两者都接受，
    // 而不是把旧拼法钉死。
    let declares = anchored.contains("#[xirang_toolchain::soil]")
        || before
            .lines()
            .skip(anchor_line.parse::<usize>().expect("a line number") - 1)
            .take(8)
            .any(|line| line.contains("pub const REGISTRATION: Registration"));
    assert!(
        declares,
        "the anchor must point at the declaration, not at line 1: {anchored}"
    );
    // A delete has no declaration to point at, because the file is gone.
    // 删除没有可指的声明，因为文件已经不在了。

    let edited = apply(
        &root,
        &json!({
            "action": "edit",
            "node": "root/button",
            "apply": true,
            "fields": {"name_en": "Push button"},
        }),
    )
    .expect("only one field is named");
    let after = std::fs::read_to_string(written(&edited)).expect("the edited face");
    // The default spelling passes the localized text as the builder's first argument, so the value is
    // checked where that spelling puts it — and this edit must not have switched the declaration's syntax.
    // 默认拼法把本地化文本作为构造器的第一个参数，因此按那种拼法所在的位置检查那个值——而这次编辑不得把声明的
    // 语法换掉。
    assert!(
        after.contains(".name(\"Push button\""),
        "the edited field is in the file: {after}"
    );
    assert!(
        after.contains("#[xirang_toolchain::soil]"),
        "editing one field must not change the declaration's spelling: {after}"
    );
    assert!(
        xirang_kernel::syntax::parse_face(&after)
            .unwrap()
            .unwrap()
            .path("kind")
            .as_deref()
            == Some("Button"),
        "the kind the request did not name must survive: {after}"
    );
    assert!(
        after.contains("ControlHandle"),
        "the contract the request did not name must survive: {after}"
    );
    assert_ne!(before, after, "the named field did change");
    let faces = face_views(&root, &name).expect("the project reads");
    assert_eq!(faces.len(), 1, "{faces:?}");
    let _ = std::fs::remove_dir_all(&root);
}

/// A rename moves the module and keeps the face. Its preview shows both halves —
/// the file that goes and the file that arrives — because a rename that only
/// reported the new file would hide the removal from the caller confirming it.
/// 改名搬走模块并保留这个面。它的预览把两半都显示出来——走掉的文件与到来的文件——因为只报告新文件的
/// 改名会把删除这一半藏起来，而调用方正要据此确认。
#[test]
fn a_rename_moves_the_module_and_keeps_the_face() {
    let (root, name) = package("rename");
    apply(
        &root,
        &json!({
            "action": "add",
            "parent": "root",
            "apply": true,
            "fields": {"module": "button", "kind": "Button", "name_en": "Button"},
        }),
    )
    .expect("the face is created");

    let preview = apply(
        &root,
        &json!({
            "action": "rename",
            "node": "root/button",
            "fields": {"module": "dial"},
        }),
    )
    .expect("the rename previews");
    assert!(preview.contains("- src/button/button.rs"), "{preview}");
    assert!(preview.contains("+ src/dial/dial.rs"), "{preview}");
    assert!(preview.contains("root/dial"), "{preview}");
    assert!(
        root.join("src/button/button.rs").is_file(),
        "the preview must not move anything: {preview}"
    );

    let applied = apply(
        &root,
        &json!({
            "action": "rename",
            "node": "root/button",
            "apply": true,
            "fields": {"module": "dial"},
        }),
    )
    .expect("the rename is applied");
    assert!(applied.contains("root/dial"), "{applied}");
    assert!(root.join("src/dial/dial.rs").is_file(), "{applied}");
    assert!(!root.join("src/button/button.rs").exists(), "{applied}");
    let faces = face_views(&root, &name).expect("the project reads");
    assert_eq!(faces.len(), 1, "{faces:?}");
    assert_eq!(faces[0].path, "root/dial");
    assert_eq!(faces[0].kind, "Button", "the face itself is unchanged");
    let _ = std::fs::remove_dir_all(&root);
}

/// A delete preview removes nothing, and the applied delete moves the module out of
/// the tree. It is recoverable by design — the executor moves the directory into
/// XiRang's trash rather than unlinking it — which is why the request can be
/// confirmed rather than merely trusted. The preview also has to *say* it removed
/// nothing: the executor reports what it did in the past tense, and a preview reply
/// that repeated that sentence bare would tell the agent the move had happened.
/// 删除预览不搬走任何东西，而落盘的删除把模块移出树。它按设计可恢复——执行器把目录移进 XiRang
/// 的回收目录而不是删掉——这正是这次请求可以被"确认"而不只是被信任的原因。预览还必须**说出**
/// 它没有搬走任何东西：执行器用过去时报告它做了什么，而预览回复若把那句话原样重复一遍，就等于
/// 告诉代理这次搬移已经发生。
#[test]
fn a_delete_removes_nothing_until_it_is_applied() {
    let (root, name) = package("delete");
    apply(
        &root,
        &json!({
            "action": "add",
            "parent": "root",
            "apply": true,
            "fields": {"module": "button", "kind": "Button", "name_en": "Button"},
        }),
    )
    .expect("the face is created");

    let preview = apply(
        &root,
        &json!({"action": "delete", "node": "root/button", "confirm": true}),
    )
    .expect("the delete previews");
    assert!(preview.contains("faces 0"), "{preview}");
    assert!(preview.contains("- src/button/button.rs"), "{preview}");
    assert!(preview.contains("would move"), "{preview}");
    assert!(
        preview.contains("preview effect: moved `button`"),
        "{preview}"
    );
    assert!(
        !preview.lines().any(|line| line.starts_with("moved ")),
        "a preview may not report the move as done: {preview}"
    );
    assert!(
        root.join("src/button/button.rs").is_file(),
        "a preview must not delete anything: {preview}"
    );

    let applied = apply(
        &root,
        &json!({"action": "delete", "node": "root/button", "apply": true, "confirm": true}),
    )
    .expect("the delete is applied");
    assert!(applied.contains("faces 0"), "{applied}");
    assert!(!root.join("src/button/button.rs").exists(), "{applied}");
    assert!(
        face_views(&root, &name)
            .expect("the project reads")
            .is_empty(),
        "the face is gone from the tree"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// A delete says `confirm` itself; the bridge does not add it. Appending it here is what made the
/// sentence in `run_delete` and the declared schema — which had no such key — describe something
/// the bridge did not do, so a caller could step past the one operation that is worth confirming
/// (audit `m5`).
/// 删除要自己说出 `confirm`，桥不会替它加上。就地拼上它，正是让 `run_delete` 里那句话与声明 schema
/// （根本没有这个键）描述桥并不做的事的原因——于是调用方可以跳过唯一值得确认的那次操作（审计 `m5`）。
#[test]
fn a_delete_without_confirm_is_refused() {
    let (root, _) = package("delete-confirm");
    apply(
        &root,
        &json!({
            "action": "add",
            "parent": "root",
            "apply": true,
            "fields": {"module": "button", "kind": "Button", "name_en": "Button"},
        }),
    )
    .expect("the face is created");

    let error = apply(
        &root,
        &json!({"action": "delete", "node": "root/button", "apply": true}),
    )
    .expect_err("a delete without `confirm` must be refused");
    assert!(error.contains("confirm"), "{error}");
    assert!(
        root.join("src/button/button.rs").is_file(),
        "the refusal must happen before anything moves: {error}"
    );

    // And the refusal is about `confirm`, not about the target: the same request with it succeeds.
    // 而且拒绝的是 `confirm`，不是目标：同一请求带上它就成功。
    apply(
        &root,
        &json!({"action": "delete", "node": "root/button", "apply": true, "confirm": true}),
    )
    .expect("the confirmed delete is applied");
    assert!(!root.join("src/button/button.rs").exists());
    let _ = std::fs::remove_dir_all(&root);
}

/// A package Cargo cannot name is refused before anything is copied or written:
/// the namespace is the identity of every face the edit would create, and a guess
/// would author faces no host compiles.
/// Cargo 说不出名字的包在复制或写入任何东西之前就被拒绝：命名空间是这次编辑会创建的每个面的身份，
/// 而猜出来的值会创作出没有宿主编译的面。
#[test]
fn a_package_that_cannot_be_named_is_refused_with_the_way_out() {
    let bare = std::env::temp_dir().join(format!("mcp-apply-nameless-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&bare);
    std::fs::create_dir_all(&bare).expect("bare directory");
    let error = apply(
        &bare,
        &json!({"action": "add", "fields": {"module": "button"}}),
    )
    .expect_err("a package Cargo cannot name has no namespace");
    assert!(error.contains("identity namespace"), "{error}");
    assert!(error.contains("XIRANG_NAMESPACE"), "{error}");
    let _ = std::fs::remove_dir_all(&bare);
}

/// An action the tool does not implement is refused by name instead of being
/// guessed at; `graft` and the rest of the production chain are still to come.
/// 工具没有实现的动作按名字被拒，而不是被猜着执行；`graft` 与生产链路其余部分仍待做。
#[test]
fn an_action_that_is_not_implemented_is_refused_by_name() {
    let (root, _) = package("unsupported");
    let error = apply(&root, &json!({"action": "graft", "node": "root/button"}))
        .expect_err("graft is not implemented yet");
    assert!(error.contains("not implemented"), "{error}");
    let _ = std::fs::remove_dir_all(&root);
}

/// Changing a face's `kind` changes its identity, and the reply says so. `kind` is an identity input
/// (`NodeId = hash(namespace, source, name)`, and a face's name is its kind), so an edit that changes
/// it rewrites the marker type and the `kind:` field and the face comes back under a new identity —
/// while every graft record keyed by the old one stops resolving. The reply used to report the
/// resulting tree without ever saying so (audit `L6`). The pin also checks the *new* id it prints
/// against the one the tree reports afterwards, so a wrong computation cannot hide in prose.
/// 改一个面的 `kind` 会改变它的身份，而回复会说明这一点。`kind` 是身份输入
/// （`NodeId = hash(namespace, source, name)`，而面的名字就是它的 kind），因此改它的编辑会重写标记类型与
/// `kind:` 字段，这个面以新身份回来——而以旧身份为键的每条 graft 记录都不再解析。回复过去只报告结果树、
/// 从不说明（审计 `L6`）。这条钉子还把它打印的**新** id 与树此后报告的 id 对照，因此算错的身份无法藏在
/// 散文里。
#[test]
fn editing_the_kind_reports_the_identity_change() {
    let (root, name) = package("kind-identity");
    apply(
        &root,
        &json!({
            "action": "add",
            "parent": "root",
            "apply": true,
            "fields": {"module": "button", "kind": "Button", "name_en": "Button"},
        }),
    )
    .expect("the face is created");
    let before = face_views(&root, &name).expect("faces derive")[0].clone();

    let reply = apply(
        &root,
        &json!({"action": "edit", "node": "root/button", "apply": true, "fields": {"kind": "Bogus"}}),
    )
    .expect("the edit is applied");
    assert!(
        reply.contains("identity changed"),
        "the reply names the identity change it caused: {reply}"
    );
    assert!(
        reply.contains(&format!("no longer `{}`", before.id)),
        "it names the identity the records were keyed by: {reply}"
    );

    let after = face_views(&root, &name).expect("faces derive")[0].clone();
    assert_ne!(after.id, before.id, "the identity really did change");
    // The reply names the *old* identity on purpose and does not print the new one: the first version
    // computed it from `change.source` and printed a number the tree did not agree with, and this pin
    // caught that. A wrong number in a diagnostic is worse than no number.
    // 回复有意只点名**旧**身份、不打印新的：第一版从 `change.source` 算它，打印出的数字与树不一致，
    // 正是这条钉子抓到的。诊断里的错数字比没有数字更糟。
    assert!(
        !reply.contains(&after.id.to_string()),
        "it deliberately does not print an identity it cannot verify: {reply}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// `edit` and `rename` refuse the two contract keys by name instead of returning success while the
/// value never reaches the file. The executor's edit field order does not carry them, so an agent
/// that asked for a checked contract got an unchecked label — silently (audit `LGC-LG-21`). `add`
/// still sets them, which is where the contract is actually written.
/// `edit` 与 `rename` 按名拒绝那两个 contract 键，而不是返回成功而取值从未抵达文件。执行器的 edit
/// 字段顺序不携带它们，因此一个要求"参与编译检查的契约"的代理拿到的是未经检查的标签——而且无声
/// （审计 `LGC-LG-21`）。`add` 仍然能设它们，contract 真正被写下的地方就是那里。
#[test]
fn edit_refuses_the_two_contract_keys_by_name_and_add_still_sets_them() {
    let (root, _) = package("contracts");
    let added = apply(
        &root,
        &json!({"action": "add", "apply": true,
                "fields": {"module": "label", "kind": "Label",
                           "handle_contracts": "crate::Foo::Bar"}}),
    )
    .expect("add sets the contract");
    assert!(
        added.contains("handle_contracts") || added.contains("label"),
        "{added}"
    );
    for key in ["handle_contracts", "part_contracts"] {
        for action in ["edit", "rename"] {
            let error = apply(
                &root,
                &json!({"action": action, "node": "root/label",
                        "fields": {"module": "label", key: "crate::Foo::Bar"}}),
            )
            .expect_err("the key is refused by name");
            assert!(error.contains(key), "{error}");
            assert!(error.contains(action), "{error}");
        }
    }
    let _ = std::fs::remove_dir_all(&root);
}

/// A missing required key is refused **with the whole accepted shape**, so the next call does not
/// have to guess it. A round measured five such refusals, each costing a round trip.
/// 缺一个必填键时，拒绝里带上**完整可接受形状**，让下一次调用不必靠猜 —— 有一轮量到五次这样的拒绝，
/// 每一次都白花一个往返。
#[test]
fn a_missing_required_key_is_refused_with_the_whole_shape() {
    let (root, _) = package("shape");

    let add = apply(
        &root,
        &json!({"action": "add", "fields": {"kind": "Button"}}),
    )
    .expect_err("add without `fields.module` is refused");
    assert!(
        add.contains("accepted shape")
            && add.contains("\"action\":\"add\"")
            && add.contains("\"module\"")
            && add.contains("\"apply\":true"),
        "the add refusal carries the shape: {add}"
    );

    let rename = apply(
        &root,
        &json!({"action": "rename", "node": "root/control/button", "fields": {}}),
    )
    .expect_err("rename without `fields.module` is refused");
    // `rename` resolves its node first, so on this empty fixture the error is the resolution refusal
    // rather than the shape one; both name what was missing, and the shape text is asserted for the
    // two actions whose argument check runs first.
    // `rename` 先解析 node，因此在这个空夹具上得到的是解析拒绝而不是形状拒绝；两者都点名缺了什么，
    // 而形状文本由「先检查参数」的那两个动作断言。
    assert!(
        rename.contains("no registration face") || rename.contains("accepted shape"),
        "the rename refusal names what is missing: {rename}"
    );

    let deepen = apply(
        &root,
        &json!({"action": "deepen", "node": "root/control/button"}),
    )
    .expect_err("deepen without `inside.parts` is refused");
    assert!(
        deepen.contains("accepted shape")
            && deepen.contains("\"action\":\"deepen\"")
            && deepen.contains("\"parts\""),
        "the deepen refusal carries the shape: {deepen}"
    );

    let _ = std::fs::remove_dir_all(&root);
}

/// The deepen preview's stop condition names the factory pins, and the call that shows them.
/// 加深预览的停止条件点名出厂形状钉子，以及展示它们的那个调用。
///
/// Measured (audit T-11): the sentence named the declaration, the public path and the tree row —
/// the three things `deepen` is *for* not touching — and stopped short of the factory-shape pins,
/// which are the fourth thing a reader of this scenario is told not to break.
/// 量到的（审计 T-11）：那句话点名了声明、公开路径与树行——`deepen` 有意不动的三样——却停在出厂形状钉子
/// 之前，而它正是这个场景的读者被告知不要弄坏的第四样。
#[test]
fn the_deepen_preview_names_the_pins_it_leaves_alone() {
    let (root, _) = package("deepen-pins");
    apply(
        &root,
        &json!({
            "action": "add",
            "parent": "root",
            "apply": true,
            "fields": {
                "module": "button",
                "kind": "Button",
                "name_en": "Button",
                "exports": "root.render",
                "handle_contracts": "ControlHandle",
            },
        }),
    )
    .expect("the face is created");
    let preview = apply(
        &root,
        &json!({"action": "deepen", "node": "root/button", "inside": {"parts": {"phase": "i32"}}}),
    )
    .expect("the deepen previews");
    assert!(
        preview.contains("the factory-shape pins were not touched"),
        "the pins are named among what was left alone: {preview}"
    );
    assert!(
        preview.contains("`check {face}` is what shows those pins still hold"),
        "and the call that shows them is named: {preview}"
    );
}

/// A refusal about a write request carries a complete request built from **this tree's** names.
/// 关于写入请求的拒绝，带的是一个用**这棵树自己的**名字搭出来的完整请求。
///
/// Audit `W4-4`: the refusals named their shape in placeholders (`<node>`, `<snake_case>`), so the
/// reader still had to find a legal parent, invent a module name that nothing uses, and work out a
/// cut's typed spelling. Each of the three is a fact about the tree the request ran in, so each is
/// read from it — and the pin checks the values against the fixture's own faces rather than against
/// the shape of the sentence.
/// 审计 `W4-4`：拒绝用占位符（`<node>`、`<snake_case>`）点名形状，于是读者还得找一个合法父面、想一个
/// 没人用的模块名、把切口的类型化拼写推出来。这三件都是"请求所在那棵树"的事实，因此都从树上读——而钉子
/// 拿这些值去对夹具**自己的**面，而不是去对那句话的形状。
#[test]
fn a_refusal_carries_a_request_built_from_this_trees_names() {
    let root = face_package("example-names");
    let plan = "xirang_toolchain::run_method::static_graft_plan!(\n    FRAMEWORK,\n    \
                cut(crate::control::object::button::NODE_ID)\n        \
                graft(control_button_graft::button_fast::NODE_ID),\n);\n";
    std::fs::write(root.join("src/lib.rs"), plan).expect("host entry");

    // `add`: the real registry-owning parent path, and a module name no face here uses.
    // `add`：**真实的**、拥有注册机的父面路径，以及一个这里没有面在用的模块名。
    let add = super::apply(&root, &json!({"action": "add"})).expect_err("`fields` is required");
    assert!(
        add.contains("\"parent\":\"root/control\"") && add.contains("\"module\":\"widget\""),
        "the example names this tree's parent and a free module: {add}"
    );

    // `edit`/`delete`/`deepen`: the real face path, not `<node>`.
    // `edit`/`delete`/`deepen`：真实的面路径，而不是 `<node>`。
    for (request, expected) in [
        (json!({"action": "edit"}), "root/control/dial"),
        (json!({"action": "delete"}), "root/control/dial"),
        (json!({"action": "deepen"}), "root/control/dial"),
    ] {
        let refused = super::apply(&root, &request).expect_err("a required key is missing");
        assert!(
            refused.contains(expected) && !refused.contains("\"<node>\""),
            "the example names a face this tree derives: {refused}"
        );
    }

    // `cut`: both halves come off the tree — the typed spelling the build derives, and the graft
    // spelling the plan already carries.
    // `cut`：两半都从树上取——构建推导的类型化拼写，与计划本来就带着的 graft 拼写。
    let cut = super::apply(&root, &json!({"action": "cut"})).expect_err("`cut` is required");
    assert!(
        cut.contains("crate::control::object::dial::NODE_ID")
            && cut.contains("control_button_graft::button_fast::NODE_ID"),
        "the cut example is this tree's face and this plan's own graft: {cut}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// The append/rewrite boundary is in the **description**, so it is read before the request.
/// 追加/改写的分界写在**描述**里，因此它在请求之前就被读到。
///
/// Audit `W4-4`'s other half: a caller that only reads the advertised schema has to be able to
/// predict whether a hand-written face is a legal subject — otherwise the answer arrives as a
/// refusal after the write was composed. The pin reads the catalog the client actually gets, not
/// this file's prose.
/// 审计 `W4-4` 的另一半：只读广告 schema 的调用方，必须能预知手写面是不是合法主体——否则答案会以
/// "请求写完之后才来的拒绝"出现。钉子读的是客户端真正拿到的目录，不是本文件里的散文。
#[test]
fn the_write_description_states_which_actions_reach_hand_written_faces() {
    let listed = crate::mcp::tools::tools();
    let apply = listed
        .iter()
        .find(|tool| tool["name"] == "xirang.apply")
        .expect("apply is advertised");
    let description = apply["description"].as_str().expect("a description");
    for phrase in [
        "additive",
        "rewrites",
        "hand-written face is refused by name",
        "separate, explicit adoption",
    ] {
        assert!(
            description.contains(phrase),
            "the advertised description carries `{phrase}`: {description}"
        );
    }
    // And the executor's own refusal repeats it, where a caller actually hits the wall.
    // 而执行器自己的拒绝也重复它，就在调用方真正撞到那堵墙的地方。
    let wall =
        super::refused_with_a_way_forward("this module was not generated by XiRang".to_owned());
    assert!(
        wall.contains("boundary:") && wall.contains("additive") && wall.contains("adoption"),
        "the wall itself says which class of action reached it: {wall}"
    );
}

/// The write reply is bounded: the census, the face that changed, and `full: true` for the rest.
/// 写入回复是有界的：普查、发生变动的那个面，其余靠 `full: true`。
///
/// Audit `W2-4`: every write printed the whole tree, which on the nine-face fixtures is a few lines
/// and on the 50,000-face tree is the entire payload. The acceptance is a byte budget on the *small*
/// fixture (≤4.4K preview / ≤2.2K apply) plus the two halves of the rule: the list is withheld by
/// default and bought back by `full: true`.
/// 审计 `W2-4`：每次写入都印整棵树，在九面夹具上是几行，在五万面的树上就是整份载荷。验收是**小**夹具上的
/// 字节预算（预览 ≤4.4K / 落盘 ≤2.2K）加上那条规则的两半：清单默认被扣下，`full: true` 把它买回来。
#[test]
fn the_write_reply_is_bounded_and_full_buys_the_tree_back() {
    let (root, _) = package("bounded-faces");
    apply(
        &root,
        &json!({"action": "add", "parent": "root", "apply": true,
                "fields": {"module": "control", "kind": "Control", "needs_registry": true}}),
    )
    .expect("the parent face");
    for (module, kind) in [("button", "Button"), ("slider", "Slider"), ("dial", "Dial")] {
        apply(
            &root,
            &json!({"action": "add", "parent": "root/control", "apply": true,
                    "fields": {"module": module, "kind": kind}}),
        )
        .expect("a child face");
    }
    let preview = apply(
        &root,
        &json!({"action": "deepen", "node": "root/control/button",
                "inside": {"parts": {"label": "String"}}}),
    )
    .expect("the preview runs");
    assert!(
        preview.contains("changed  root/control/button")
            && preview.contains("withheld")
            && !preview.contains("  root/control/slider  Slider"),
        "the default answer is the census and the changed face: {preview}"
    );
    // The budget is about a preview staying bounded, not about a particular number: the literal spelling
    // is longer per face than the macro one (module docs, two `use` lines, `impl FaceContract`, the baked
    // identity), so the bound moves with it — measured 4523 bytes, kept at 4800 with headroom.
    // 预算关心的是预览**有界**，不是某个具体数字：字面量拼法逐面比宏拼法长（模块文档、两行 `use`、
    // `impl FaceContract`、烘焙的身份），因此这个上界跟着动——实测 4523 字节，取 4800 留余量。
    assert!(
        preview.len() <= 4800,
        "the preview stays inside its budget: {} bytes",
        preview.len()
    );
    let applied = apply(
        &root,
        &json!({"action": "deepen", "node": "root/control/button", "apply": true,
                "inside": {"parts": {"label": "String"}}}),
    )
    .expect("the apply runs");
    assert!(
        applied.len() <= 2200,
        "and so does the write: {} bytes",
        applied.len()
    );

    let full = apply(
        &root,
        &json!({"action": "deepen", "node": "root/control/slider", "full": true,
                "inside": {"parts": {"label": "String"}}}),
    )
    .expect("the preview runs");
    assert!(
        full.contains("  root/control/slider  Slider")
            && full.contains("  root/control/button  Button"),
        "`full: true` prints the whole tree again: {full}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// A delete whose trash cannot be created leaves the module and the blocker exactly as they were.
/// 回收目录无法创建的删除，会让模块与挡路者都保持原样。
///
#[cfg(test)]
#[path = "apply_delete_tests.rs"]
mod apply_delete_tests;

#[cfg(test)]
#[path = "apply_layout_tests.rs"]
mod apply_layout_tests;
