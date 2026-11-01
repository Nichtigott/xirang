//! Pins for the const-array graft plan: the two element spellings agree.
//! 常量数组计划的钉子：两种元素拼法必须一致。

use super::super::super::imports::Imports;
use super::entries;

/// Read one `const PLAN: &[Graft] = &[…]` declaration.
/// 读一条 `const PLAN: &[Graft] = &[…]` 声明。
fn parse_plan(source: &str) -> Result<Vec<super::super::graft::GraftSyntax>, String> {
    // Walk the **file**, the way the build's visitor does: more than one declaration is normal here
    // (adding a cut writes a numbered one beside the host's), and every declaration it finds counts.
    // 按**文件**走，与构建的访问器同形：这里出现多条声明是正常的（加一条切口会在宿主那份旁边写一条编号声明），
    // 而它找到的每一条都算数。
    let file: syn::File = syn::parse_str(source).expect("a source file");
    let mut collected = Vec::new();
    for item in &file.items {
        let syn::Item::Const(item) = item else {
            continue;
        };
        match entries(item, &Imports::new()) {
            Ok(Some(entries)) => collected.extend(entries),
            Ok(None) => {}
            Err(error) => return Err(error.to_string()),
        }
    }
    if collected.is_empty() {
        return Err("no declaration was recognised as a plan".to_owned());
    }
    Ok(collected)
}

/// The readable chain states the same three shapes as the `graft(…).with(…)` spelling: a single cut, a
/// sibling range, and a whole subtree. The chain's head is the **innermost** receiver, which is the one
/// thing an implementation is likely to get wrong.
/// 可读链与 `graft(…).with(…)` 拼法陈述同样三种形状：单个切口、兄弟区间、整棵子树。链的头是**最内层**的
/// 接收者——而这正是实现最可能搞错的那一处。
#[test]
fn the_readable_chain_states_the_same_three_shapes() {
    let chain = parse_plan(
        r#"const PLAN: &[Graft] = &[
            cut("root/a").graft("g1"),
            cut("root/x").to("root/z").graft("g2"),
            cut("root/f").graft("g3").full(),
        ];"#,
    )
    .expect("the chain is a plan");

    let older = parse_plan(
        r#"const PLAN: &[Graft] = &[
            graft("root/a").with("g1"),
            graft(range("root/x", "root/z")).with("g2"),
            graft(subtree("root/f")).with("g3"),
        ];"#,
    )
    .expect("the older spelling is a plan");

    assert_eq!(chain.len(), 3);
    for (left, right) in chain.iter().zip(older.iter()) {
        assert_eq!(left.cut, right.cut, "the cut target");
        assert_eq!(left.cut_end, right.cut_end, "the far endpoint");
        assert_eq!(left.graft, right.graft, "the replacement");
        assert_eq!(left.full, right.full, "whole-subtree or not");
    }
    assert_eq!(chain[1].cut_end.as_deref(), Some("root/z"));
    assert!(chain[2].full);
    assert!(!chain[0].full);
}

/// `to(…)` and `full()` name two different shapes, so stating both is refused by name — the same
/// refusal the older spelling makes for `subtree(range(…))`.
/// `to(…)` 与 `full()` 命名两种不同形状，因此同时写出会被**按名拒绝**——与旧拼法对
/// `subtree(range(…))` 的拒绝完全对称。
#[test]
fn a_chain_that_states_a_range_and_a_whole_subtree_is_refused() {
    let error =
        parse_plan(r#"const PLAN: &[Graft] = &[cut("root/x").to("root/z").graft("g").full()];"#)
            .expect_err("two shapes at once");
    assert!(
        error.contains("full()") && error.contains("to("),
        "the refusal names both spellings: {error}"
    );
}

/// A chain with no replacement has no meaning: the replacement is what a cut grafts.
/// 没有替换的链没有意义：替换才是切口所嫁接的东西。
#[test]
fn a_chain_without_a_replacement_is_refused() {
    let error = parse_plan(r#"const PLAN: &[Graft] = &[cut("root/a").to("root/z")];"#)
        .expect_err("no replacement");
    assert!(!error.is_empty(), "the refusal says something: {error}");
}

/// The plan wrapper is `Plan::of([…])`, and it sits at **item** position — a `const`, not a function body.
/// That is not a detail: this reader deliberately ignores function bodies, because a plan inside one is a
/// runtime expression, and capturing those once shipped test cuts into the release table.
/// 计划的外层是 `Plan::of([…])`，而且它写在**条目**位置——是一条 `const`，不是函数体。这不是细节：
/// 本读法**有意**忽略函数体，因为写在里面的计划是运行时表达式，而过去把它们收进来，曾把测试切口带进发布表。
#[test]
fn a_plan_wrapper_is_read_at_item_position() {
    let entries = parse_plan(
        r#"const PLAN: Plan = Plan::of([
            cut("root/a").graft("g1"),
            cut("root/x").to("root/z").graft("g2"),
        ]);"#,
    )
    .expect("the wrapper is a plan");
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].cut, "root/a");
    assert_eq!(entries[1].cut_end.as_deref(), Some("root/z"));
}

/// A plan that names no replacement is refused inside the wrapper too — the wrapper changes the spelling,
/// never the rules.
/// 没有替换的计划在包装里同样被拒——包装只改拼法，从不改规则。
#[test]
fn a_wrapped_plan_without_a_replacement_is_refused() {
    let error =
        parse_plan(r#"const PLAN: Plan = Plan::of([cut("root/a")]);"#).expect_err("no replacement");
    assert!(!error.is_empty(), "the refusal says something: {error}");
}

/// Adding a cut to a host **that already has a plan** adds a numbered declaration beside it — that is
/// the design (`XIRANG_GRAFTS_<n>`), and the reader collects every declaration it finds. So the contract
/// to pin is behavioural, not textual: after the append, the source yields **both** cuts.
/// 给一个**已经有计划**的宿主加切口，会在它旁边加一条**编号**声明——这是设计（`XIRANG_GRAFTS_<n>`），而读法
/// 会收集它找到的每一条声明。因此要钉的契约是**行为**而不是文本：追加之后，这份源码读出**两条**切口。
#[test]
fn adding_a_cut_to_a_wrapped_plan_yields_both_cuts() {
    let wrapped = r#"const PLAN: Plan = Plan::of([
    cut("root/a").graft("g1"),
]);"#;
    let edited = super::append_native_graft(wrapped, r#"cut("root/b").graft("g2")"#)
        .expect("the cut lands beside the existing plan");
    let entries = parse_plan(&edited).expect("the edited source is still a plan");
    let cuts = entries.iter().map(|e| e.cut.as_str()).collect::<Vec<_>>();
    assert_eq!(
        cuts,
        vec!["root/a", "root/b"],
        "both cuts, in order:\n{edited}"
    );
    assert!(
        edited.contains("Plan::of"),
        "the host's own spelling is untouched"
    );
}

/// The same gesture on the older wrapper behaves identically — the two spellings are one plan
/// vocabulary, and adding a cut must not care which one a host uses.
/// 同一次操作在较旧的包装上表现完全一致——两种拼法是同一套计划词汇，加切口不该在意宿主用哪一种。
#[test]
fn adding_a_cut_to_the_array_wrapper_yields_both_cuts() {
    let array =
        "const PLAN: &[xirang_toolchain::Graft] = &[\n    graft(\"root/a\").with(\"g1\"),\n];";
    let edited = super::append_native_graft(array, r#"cut("root/b").graft("g2")"#)
        .expect("the cut lands beside the existing array");
    let entries = parse_plan(&edited).expect("the edited source is still a plan");
    let cuts = entries.iter().map(|e| e.cut.as_str()).collect::<Vec<_>>();
    assert_eq!(cuts, vec!["root/a", "root/b"], "both cuts:\n{edited}");
}

/// The readable chains produce marker values, so the array wrapper has to accept their type: with `&[Graft]`
/// alone, the same text written by an author would be silently read as "not a plan at all".
/// 可读链产出的是标记值，因此数组包装必须接受它们的类型：只认 `&[Graft]` 时，作者写下的同一段文本会被**静默**
/// 读成"根本不是计划"。
#[test]
fn the_array_wrapper_accepts_the_marker_entry_type() {
    let source = "const PLAN: &[PlanEntry] = &[\n    cut(\"root/a\").graft(\"g1\"),\n];";
    let entries = parse_plan(source).expect("the marker-typed array is a plan");
    assert_eq!(entries.len(), 1, "one entry:\n{source}");
    assert_eq!(entries[0].cut, "root/a");
}
