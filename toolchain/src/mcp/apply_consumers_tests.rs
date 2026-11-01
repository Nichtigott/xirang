//! The consumer half of a write: what a change does to the tree's readers outside its own file.
//! 一次写入的消费方那一半：这次改动对"这个文件之外读它的东西"做了什么。
//!
//! Split out of `apply_tests.rs` when that file crossed the 800-code-line test ceiling — the same
//! split, for the same reason, that `apply_refusals_tests.rs` records.
//! 在那个文件越过 800 行代码行的测试上限时拆出来——与 `apply_refusals_tests.rs` 记录的同一种拆分、同一个理由。

use serde_json::json;

use super::apply;
use super::apply_tests::{package, written};

/// The PascalCase kind a module's name implies, as the shared entrance requires.
/// 模块名所暗示的 PascalCase 种类名，共享入口要求如此。
fn pascal(module: &str) -> &'static str {
    match module {
        "button" => "Button",
        "slider" => "Slider",
        "alpha" => "Alpha",
        "beta" => "Beta",
        other => panic!("no kind for `{other}`"),
    }
}

/// The consumer story a deepen tells is the **same** in the preview and after the write (audit `W5-3`).
/// deepen 讲的消费方说法，在预览里与写入之后是**同一件**（审计 `W5-3`）。
///
/// This is the acceptance's real content: a preview that told a different consumer story from the one
/// that lands would be an advertisement. Both replies are produced by the **same executor** running on
/// whichever tree it was handed, so this pin is checking that the report does not filter them — and
/// that a tree with plan entries gets them named, with the reason they keep pointing where they point.
/// 这正是验收的实质：一个与落盘说法不同的消费方故事，就是广告。两条回复由**同一个执行器**在它拿到的那棵树
/// 上产出，因此这条钉子查的是报告没有把它们过滤掉——以及有计划条目的树会看到它们被点名，并看到"它们为何仍然
/// 指向原处"。
#[test]
fn the_consumer_story_is_the_same_in_the_preview_and_after_the_write() {
    let (root, _) = package("deepen-consumers");
    apply(
        &root,
        &json!({"action": "add", "parent": "root", "apply": true,
                "fields": {"module": "control", "kind": "Control", "needs_registry": true}}),
    )
    .expect("the parent face");
    apply(
        &root,
        &json!({"action": "add", "parent": "root/control", "apply": true,
                "fields": {"module": "button", "kind": "Button"}}),
    )
    .expect("the leaf face");
    // One plan that targets the face being deepened: this is the consumer the diff exists for.
    // 一条针对"被做深的那个面"的计划：这就是 diff 为之存在的消费方。
    let names = crate::mcp::registry::namespace(&root).expect("the fixture's namespace");
    let target = xirang_kernel::identity::NodeId::from_namespaced_path(
        &names,
        "control/object/button/button.rs",
        "Button",
    );
    let plan = root.join(".xirang/external-grafts/swap/graft.plan");
    std::fs::create_dir_all(plan.parent().expect("plan directory")).expect("plan dir");
    std::fs::write(
        &plan,
        format!(
            "version = 1\ntarget = {target}\ntarget_path = root/control/button\ngraft = swap\nfull = false\n"
        ),
    )
    .expect("the plan");
    let preview = apply(
        &root,
        &json!({"action": "deepen", "node": "root/control/button",
                "inside": {"parts": {"label": "String"}}}),
    )
    .expect("the deepen preview runs");
    let landed = apply(
        &root,
        &json!({"action": "deepen", "node": "root/control/button", "apply": true,
                "inside": {"parts": {"label": "String"}}}),
    )
    .expect("the deepen applies");
    let consumers = |reply: &str| {
        reply
            .lines()
            .filter(|line| line.starts_with("consumers  ") || line.starts_with("slots      "))
            .map(str::to_owned)
            .collect::<Vec<_>>()
    };
    assert_eq!(
        consumers(&preview),
        consumers(&landed),
        "the preview is not an advertisement:\n{preview}\n---\n{landed}"
    );
    assert!(
        preview.contains("the layer adds 1 filling option(s) inside `Button`"),
        "the layer's own options are named: {preview}"
    );
    assert!(
        preview.contains(
            "1 of 1 graft plan entry(ies) target this face or its subtree (swap targets \
             root/control/button)"
        ) && preview.contains("leaves every")
            && preview.contains("target and path alone"),
        "the consumer line names the plan and the reason it keeps pointing there: {preview}"
    );
}

/// A write says whether the face it wrote landed like its siblings (audit `W5-6`).
/// 一次写入会说出它写下的那个面是否长得像它的兄弟（审计 `W5-6`）。
///
/// This is the "zero write-then-recheck" half of the item: the caller learns the family verdict from
/// the write's own reply instead of spending a second instrument call on `consistency`. The verdict
/// uses `consistency`'s own majority arithmetic, so the two answers cannot come to disagree about what
/// an outlier is.
/// 这是该条目的"零写后回查"那一半：调用方从写入自己的回复里拿到家族判定，而不必再花一次仪器调用去问
/// `consistency`。判定用的是 `consistency` 自己的多数派算术，因此两份答案不可能对"什么算离群"产生分歧。
#[test]
fn a_write_reports_whether_the_new_face_landed_like_its_siblings() {
    let (root, _) = package("family-verdict");
    apply(
        &root,
        &json!({"action": "add", "parent": "root", "apply": true,
                "fields": {"module": "control", "kind": "Control", "needs_registry": true}}),
    )
    .expect("the parent face");
    for module in ["button", "slider"] {
        apply(
            &root,
            &json!({"action": "add", "parent": "root/control", "apply": true,
                    "fields": {"module": module, "kind": pascal(module)}}),
        )
        .expect("an agreeing leaf");
    }
    let agreeing = apply(
        &root,
        &json!({"action": "add", "parent": "root/control", "apply": true,
                "fields": {"module": "knob", "kind": "Knob"}}),
    )
    .expect("a third agreeing leaf");
    assert!(
        agreeing.contains("family     3 sibling(s) under root/control; outliers: 0 of 3"),
        "an agreeing face reports a clean family: {agreeing}"
    );

    // A face that declares what no sibling declares is the outlier, and the reply says so.
    // 一个声明了没有任何兄弟声明的字段的面就是离群者，而回复会说出来。
    let deviating = apply(
        &root,
        &json!({"action": "add", "parent": "root/control", "apply": true,
                "fields": {"module": "dial", "kind": "Dial", "exports": "[control.render]"}}),
    )
    .expect("a deviating leaf");
    assert!(
        deviating.contains("family     4 sibling(s) under root/control; outliers: 1 of 4"),
        "the deviating face is counted: {deviating}"
    );
    assert!(
        deviating.contains("outlier     dial: declares `exports`")
            && deviating.contains("`consistency {parent: \"root/control\"}` prints the fix"),
        "and it is named, with the call that hands back the fix: {deviating}"
    );

    // The verdict follows the **parent** the face landed under, not a fixed path: a second root child
    // is judged against the root's own children.
    // 判定跟着"这个面落在哪个父级"走，而不是一条固定路径：第二个根子面按根自己的孩子们来判。
    let rooted = apply(
        &root,
        &json!({"action": "add", "parent": "root", "apply": true,
                "fields": {"module": "solo", "kind": "Solo", "needs_registry": true}}),
    )
    .expect("a root child");
    assert!(
        rooted.contains("family     2 sibling(s) under root; outliers:"),
        "the verdict names the family the face landed in: {rooted}"
    );
}

/// A field the caller left out is filled from the family it is joining, and the reply says so (audit `W5-6`).
/// 调用方没写的字段，从它正在加入的那个家族里补上，而回复会说出来（审计 `W5-6`）。
///
/// This is the "derive the family contract from the tree" half: the parent's other children have
/// already answered "what does this family declare", so a caller that has to spell it from memory is
/// doing work the tree can do — and getting it wrong is how a family drifts. Both directions are
/// pinned: an omitted field is inherited **and reported**, an explicit one is left alone.
/// 这是"从树上推导家族契约"那一半：父级其它孩子已经回答过"这一家声明了什么"，因此必须凭记忆拼它的调用方
/// 是在做树能做的事——而拼错正是家族开始漂移的方式。两个方向都钉：省略的字段会被继承**并报告**，显式给的
/// 字段不动。
#[test]
fn an_omitted_family_field_is_inherited_and_reported() {
    let (root, _) = package("family-inherit");
    apply(
        &root,
        &json!({"action": "add", "parent": "root", "apply": true,
                "fields": {"module": "control", "kind": "Control", "needs_registry": true}}),
    )
    .expect("the parent face");
    for module in ["alpha", "beta"] {
        apply(
            &root,
            &json!({"action": "add", "parent": "root/control", "apply": true,
                    "fields": {"module": module, "kind": pascal(module), "exports": "control.render",
                               "handle_traits": "ControlHandle"}}),
        )
        .expect("a sibling stating the family contract");
    }
    let inherited = apply(
        &root,
        &json!({"action": "add", "parent": "root/control", "apply": true,
                "fields": {"module": "gamma", "kind": "Gamma"}}),
    )
    .expect("the new face");
    assert!(
        inherited
            .contains("inherited  exports = control.render from the family under root/control")
            && inherited.contains("inherited  handle_traits = ControlHandle from the family"),
        "the reply names what it filled in: {inherited}"
    );
    let text = std::fs::read_to_string(written(&inherited)).expect("the new face file");
    assert!(
        text.contains(".exports(&[\"control.render\"])")
            && text.contains(".handle_traits(&[\"ControlHandle\"])"),
        "and the file carries them in the default declaration's own spelling: {text}"
    );

    // An explicit value wins, and generates no inheritance note for that field.
    // 显式给的值赢，并且那个字段不会产生继承说明。
    let explicit = apply(
        &root,
        &json!({"action": "add", "parent": "root/control", "apply": true,
                "fields": {"module": "delta", "kind": "Delta", "exports": "control.other"}}),
    )
    .expect("a face that overrides one field");
    assert!(
        !explicit.contains("inherited  exports")
            && explicit.contains("inherited  handle_traits = ControlHandle"),
        "the caller's value is not overwritten, and the fields it did not state still are: {explicit}"
    );
    let text = std::fs::read_to_string(written(&explicit)).expect("the overriding face file");
    assert!(
        text.contains(".exports(&[\"control.other\"])"),
        "the caller's spelling is the one that lands: {text}"
    );
}
