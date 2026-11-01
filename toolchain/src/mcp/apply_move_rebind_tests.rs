//! The pins for what a move **carries**: records, declarations, membership and order.
//! 搬迁**带走什么**的钉子：记录、声明、成员集合与顺序。
//!
//! Split from `apply_move_graft_tests.rs` at the size ratchet, along the seam that was already
//! there: that file pins what the *detector* says before writing, this one pins what the *rebinder*
//! writes after. Both drive the same fixture (`graft_fixtures::Tree`).
//! 按尺寸棘轮从 `apply_move_graft_tests.rs` 拆出，沿着本来就有的那条缝：那个文件钉**检测器**在写入前说了
//! 什么，这个文件钉**重绑器**在写入后写了什么。两者驱动同一个夹具（`graft_fixtures::Tree`）。

use super::graft_fixtures::Tree;

/// W5/A20: a record whose target and target_path both name **one** moved object is carried by the
/// move when the caller asks for it — `target` and `target_path` rewritten together against a tree
/// that has already moved — while the same move without that request is still refused.
/// W5/A20：当调用方明确要求时，`target` 与 `target_path` **同时**命中同一个被搬对象的记录会跟着搬迁一起
/// 被改写（对着已经搬完的树改写）；而同样一次搬迁在不带该请求时照旧被拒绝。
#[test]
fn a_move_carries_the_record_whose_target_it_determines() {
    let tree = Tree::fixture();
    let faces = crate::build_method::face_views(&tree.0, "move-graft-test").unwrap();
    let face = faces
        .iter()
        .find(|face| face.path == "root/panel/b")
        .expect("the fixture has the face")
        .clone();
    let record = tree.0.join(".xirang/external-grafts/impl_b/graft.plan");
    std::fs::create_dir_all(record.parent().unwrap()).unwrap();
    std::fs::write(
        &record,
        format!(
            "version=1\ntarget={}\ntarget_path=root/panel/b\ngraft=impl_b\nfull=false\n",
            face.id
        ),
    )
    .unwrap();

    // Without the request the move is refused, and nothing is written.
    // 不带该请求时搬迁被拒绝，且什么都不写。
    let refused = tree.refused("root/panel/b", "root/shelf", None);
    assert!(refused.contains("impl_b"), "{refused}");
    assert!(
        std::fs::read_to_string(&record)
            .unwrap()
            .contains(&face.id.to_string()),
        "a refusal leaves the record alone"
    );

    let reply = tree.applied("root/panel/b", "root/shelf", None, true);
    assert!(
        reply.contains("rebound impl_b") && reply.contains("root/panel/b -> root/shelf/b"),
        "the move says what it carried: {reply}"
    );
    let after = std::fs::read_to_string(&record).unwrap();
    assert!(
        after.contains("target_path=root/shelf/b"),
        "target_path follows the object: {after}"
    );
    assert!(
        !after.contains(&face.id.to_string()),
        "and target moved with it, not stayed behind: {after}"
    );
    assert!(
        after.contains("graft=impl_b") && after.contains("full=false"),
        "the rest of the record is preserved: {after}"
    );
}
/// W5/A20: the **declaration** a move determines is rewritten too, in both spellings, and the
/// rewrite follows the same map the refusal prints.
/// W5/A20：这次搬迁所确定的**声明**同样被改写，两种拼法都覆盖，而且改写依据的正是拒绝里印出的同一张映射。
///
/// Measured before this was written: a rebinding move left the plan source **byte-identical** for
/// both `cut "root/panel/b"` and `cut(crate::panel::b::NODE_ID)`, because the reference planner
/// rewrites module paths in ordinary source and never looks inside the plan's macro arguments.
/// 写下这段之前实测过：无论 `cut "root/panel/b"` 还是 `cut(crate::panel::b::NODE_ID)`，一次带重绑的
/// 搬迁都让计划源码**逐字节不变**——引用规划器改写的是普通源码里的模块路径，从不看计划宏的参数里面。
#[test]
fn a_move_rewrites_the_declaration_it_determines_in_both_spellings() {
    for (cut, expected) in [
        (
            "cut \"root/panel/b\" graft \"replacement\",",
            "cut \"root/shelf/b\" graft \"replacement\",",
        ),
        (
            "cut(crate::panel::b::NODE_ID) graft(external::replacement::NODE_ID),",
            "cut(crate::shelf::b::NODE_ID) graft(external::replacement::NODE_ID),",
        ),
    ] {
        let tree = Tree::fixture();
        tree.plan(cut);
        let before = std::fs::read_to_string(tree.0.join("src/lib.rs")).unwrap();
        assert!(before.contains(cut), "the fixture wrote the cut: {before}");

        // Without the request the move is refused and the plan is untouched.
        // 不带请求时搬迁被拒绝，计划源码不动。
        let refused = tree.refused("root/panel/b", "root/shelf", None);
        assert!(refused.contains("determined"), "{refused}");
        assert_eq!(
            std::fs::read_to_string(tree.0.join("src/lib.rs")).unwrap(),
            before,
            "a refusal rewrites nothing"
        );

        let reply = tree.applied("root/panel/b", "root/shelf", None, true);
        let after = std::fs::read_to_string(tree.0.join("src/lib.rs")).unwrap();
        assert!(
            reply.contains("rebound") && reply.contains("src/lib.rs:2"),
            "the move says which line it rewrote: {reply}"
        );
        assert!(
            after.contains(expected),
            "the new spelling is written: {after}"
        );
        assert!(
            !after.contains(cut),
            "and the old one is gone, so the plan cannot name both: {after}"
        );
    }
}
/// A22: "the member set changes" is a conclusion; the report hands the reader the **two states** it
/// was drawn from, so a person deciding whether the graft should follow does not have to re-derive
/// them. Both spellings of a coverage change are covered, and a move that leaves the object where it
/// was reports both states as `true` rather than staying silent.
/// A22："成员集合变了"是一个结论；报告把它所依据的**两个状态**交给读者，于是要决定这个嫁接该不该跟着走的人
/// 不必自己再推一遍。覆盖变化的两种拼法都覆盖，而"对象没动窝"的搬迁会把两个状态都报成 `true`，而不是不说。
#[test]
fn a_coverage_change_shows_the_membership_before_and_after() {
    // A full graft over the parent: moving the child out of it changes the set.
    // 父级上的全覆盖嫁接：把子级搬出去就改变了集合。
    let tree = Tree::fixture();
    tree.plan("cut(crate::panel::NODE_ID) full graft(external::replacement::NODE_ID),");
    let out = tree.refused("root/panel/b", "root/shelf", None);
    assert!(
        out.contains("full graft on `") && out.contains("inside before = true, after = false"),
        "both states are named for the object that left: {out}"
    );

    // The same graft, moving an object that stays inside: both states are true, and the report says
    // so instead of claiming a change.
    // 同一个嫁接，搬一个仍在里面的对象：两个状态都是 true，报告这么说，而不是声称发生了变化。
    let tree2 = Tree::fixture();
    tree2.plan("cut(crate::panel::NODE_ID) full graft(external::replacement::NODE_ID),");
    let inside = tree2.refused("root/panel/b", "root/panel", Some("bb"));
    assert!(
        inside.contains("inside before = true, after = true"),
        "an object that stayed inside is shown staying inside: {inside}"
    );

    // A sibling range whose interior the move crosses.
    // 一次跨过兄弟区间内部的搬迁。
    let tree3 = Tree::fixture();
    tree3.plan(
        "cut(crate::panel::a::NODE_ID to crate::panel::c::NODE_ID) graft(external::replacement::NODE_ID),",
    );
    let range = tree3.refused("root/panel/b", "root/shelf", None);
    assert!(
        range.contains("range graft on `") && range.contains("inside before = true, after = false"),
        "a range reports the same two states: {range}"
    );
}
/// A23: the same three moves land in the same **declared** plan whatever order they are applied in.
/// A23：同样三次搬迁，无论按哪种顺序执行，最终**声明出来的**计划都一样。
///
/// The comparison is on the parsed declarations, not on the file text: rewriting three lines in a
/// different order legitimately reorders the lines, and a rule that insisted on byte equality would
/// be pinning the order rather than the outcome. What must not depend on the order is the **set** of
/// `(cut, graft, full)` triples the host declares, and that is what each permutation is compared on.
/// 比的是解析出来的声明而不是文件文本：三行按不同顺序改写，行序本来就该不同，坚持逐字节相等等于钉住顺序
/// 而不是结果。绝不能随顺序变的，是宿主声明的 `(cut, graft, full)` **集合**——每个排列就是按它来比的。
#[test]
fn three_moves_land_in_the_same_plan_in_every_order() {
    use crate::build_method::declared_grafts;
    let orders: [[&str; 3]; 6] = [
        ["a", "b", "c"],
        ["a", "c", "b"],
        ["b", "a", "c"],
        ["b", "c", "a"],
        ["c", "a", "b"],
        ["c", "b", "a"],
    ];
    let cut_set = |root: &std::path::Path| {
        let mut declared: Vec<(String, String, bool)> = declared_grafts(root)
            .expect("the plan parses")
            .cuts
            .into_iter()
            .map(|cut| (cut.cut, cut.graft, cut.full))
            .collect();
        declared.sort();
        declared
    };
    let mut expected: Option<Vec<(String, String, bool)>> = None;
    for order in orders {
        let tree = Tree::fixture();
        // One string cut per leaf, so each move determines exactly one declaration.
        // 每个叶子一条字符串切口，因此每次搬迁恰好确定一条声明。
        tree.plan(
            "cut \"root/panel/a\" graft \"replacement\",\n\
             cut \"root/panel/b\" graft \"replacement\",\n\
             cut \"root/panel/c\" graft \"replacement\",",
        );
        for leaf in order {
            let reply = tree.applied(&format!("root/panel/{leaf}"), "root/shelf", None, true);
            assert!(
                reply.contains("rebound")
                    && reply.contains(&format!("\"root/panel/{leaf}\" -> \"root/shelf/{leaf}\"")),
                "every move in this order rebinds its own declaration: {reply}"
            );
        }
        let landed = cut_set(&tree.0);
        assert_eq!(
            landed.len(),
            3,
            "all three declarations survive: {landed:?}"
        );
        assert!(
            landed
                .iter()
                .all(|(cut, _, _)| cut.starts_with("root/shelf/")),
            "and every one of them followed its object: {landed:?}"
        );
        match &expected {
            None => expected = Some(landed),
            Some(first) => assert_eq!(
                &landed, first,
                "order {order:?} reached a different declared plan"
            ),
        }
    }
}
/// W5/A20: **several** declarations naming the same moved object are each carried, in their own
/// spelling, by one move. A single-target rebinder would rewrite the first and leave the second
/// pointing at a path that no longer exists.
/// W5/A20：**多条**声明同时指向同一个被搬对象时，一次搬迁把它们**各自按自己的拼法**一起带走。只会重绑单个
/// 目标的写入器会改掉第一条、把第二条留在一条已经不存在的路径上。
#[test]
fn one_move_carries_every_declaration_that_names_the_object() {
    let tree = Tree::fixture();
    tree.plan(
        "cut \"root/panel/b\" graft \"replacement\",\n\
         cut(crate::panel::b::NODE_ID) graft(external::replacement::NODE_ID),",
    );
    let reply = tree.applied("root/panel/b", "root/shelf", None, true);
    let plan = std::fs::read_to_string(tree.0.join("src/lib.rs")).unwrap();
    assert!(
        plan.contains("cut \"root/shelf/b\" graft \"replacement\","),
        "the string spelling followed the object: {plan}"
    );
    assert!(
        plan.contains("cut(crate::shelf::b::NODE_ID) graft(external::replacement::NODE_ID),"),
        "and so did the typed one: {plan}"
    );
    assert_eq!(
        reply.matches("rebound").count(),
        2,
        "the reply accounts for both rewrites rather than one: {reply}"
    );
    assert!(
        !plan.contains("root/panel/b") && !plan.contains("crate::panel::b::NODE_ID"),
        "nothing is left naming the old position: {plan}"
    );
}
/// W5/A20: a **consumer package** in the same workspace follows the move too. Its plan is a
/// separate entry file, so nothing about the moved package's own entry would ever reach it — this is
/// the joint rebinding the acceptance asks for, across crates.
/// W5/A20：同一个工作区里的**消费方包**也随着这次搬迁走。它的计划是另一个入口文件，因此被搬那个包自己的
/// 入口永远够不到它——这就是验收要的、**跨 crate** 的联合重绑。
#[test]
fn a_consumer_packages_plan_follows_the_move_as_well() {
    let tree = Tree::fixture();
    // The root manifest lists the consumer as a member; without that it is a directory, not a crate
    // the workspace would build.
    // 根清单把消费方列为成员；不列它，它就只是一个目录，而不是工作区会构建的 crate。
    let manifest = tree.0.join("Cargo.toml");
    let listed = std::fs::read_to_string(&manifest)
        .unwrap()
        .replace("[workspace]\n", "[workspace]\nmembers = [\"consumer\"]\n");
    std::fs::write(&manifest, listed).unwrap();
    let consumer = tree.0.join("consumer");
    std::fs::create_dir_all(consumer.join("src")).unwrap();
    std::fs::write(
        consumer.join("Cargo.toml"),
        "[package]\nname=\"consumer\"\nversion=\"0.1.0\"\nedition=\"2024\"\n",
    )
    .unwrap();
    let plan = consumer.join("src/lib.rs");
    std::fs::write(
        &plan,
        "crate::static_graft_plan!(FRAMEWORK,\ncut \"root/panel/b\" graft \"replacement\",\n);\n",
    )
    .unwrap();
    tree.plan("cut \"root/panel/b\" graft \"replacement\",");

    // Without the request the move is refused and neither entry is touched.
    // 不带请求时搬迁被拒绝，两个入口都不动。
    let refused = tree.refused("root/panel/b", "root/shelf", None);
    assert!(refused.contains("determined"), "{refused}");
    assert!(
        std::fs::read_to_string(&plan)
            .unwrap()
            .contains("root/panel/b"),
        "a refusal leaves the consumer alone"
    );

    let reply = tree.applied("root/panel/b", "root/shelf", None, true);
    let consumer_plan = std::fs::read_to_string(&plan).unwrap();
    assert!(
        consumer_plan.contains("cut \"root/shelf/b\""),
        "the consumer's plan follows the object: {consumer_plan}\nREPLY {reply}"
    );
    assert!(
        !consumer_plan.contains("root/panel/b"),
        "and nothing there still names the old position: {consumer_plan}"
    );
    assert!(
        reply.contains("consumer/src/lib.rs"),
        "the reply says which entry in another package it rewrote: {reply}"
    );
}
