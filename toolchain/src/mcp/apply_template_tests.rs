//! P3's first slice: a new object's **approved template** is unique, conflicting, or absent.
//! P3 第一片：新对象的**已批准模板**是唯一的、冲突的，或不存在。

use serde_json::json;
use std::path::PathBuf;

/// A package with a policy file the sample adoptions rest on.
/// 一个带政策文件的包，sample 采信依托它。
fn package(label: &str) -> PathBuf {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let name = format!("mcp-apply-template-{label}");
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

fn add_face(root: &std::path::Path, request: serde_json::Value) -> Result<String, String> {
    crate::mcp::apply::apply(root, &request)
}

fn adopt(root: &std::path::Path, anchor: &str) -> String {
    crate::mcp::adopted::adopted(
        root,
        &json!({"anchor":anchor,"certifies":"the specimen shape","evidence":"desk check",
                "verifier":"template test","reason":"this face is the specimen for its family",
                "files":["policy.txt"],"apply":true,"confirm":true}),
    )
    .expect("the adoption applies")
}

/// **A unique approved specimen is selected; a category with none produces a candidate.**
/// **唯一的已批准样板被选中；没有样板的类别产生候选。**
#[test]
fn a_unique_specimen_is_the_template_and_a_new_category_is_a_candidate() {
    let root = package("unique");
    let first = add_face(
        &root,
        json!({"action":"add","fields":{"module":"provider","kind":"Provider","provides":"cap.render",
                "preset":"FastPreset","parts":"FastParts"},"apply":true}),
    )
    .expect("the first face is created");
    assert!(
        first.contains("no approved specimen for this category yet") && first.contains("candidate"),
        "creating the first face does not crown it as a template: {first}"
    );
    adopt(&root, "root/provider");

    let second = add_face(
        &root,
        json!({"action":"add","fields":{"module":"provider2","kind":"Provider","provides":"cap.other"},"apply":true}),
    )
    .expect("the second face is created");
    assert!(
        second.contains("template: follows the approved specimen `root/provider`"),
        "the unique in-force specimen of the family is selected: {second}"
    );
    let generated = std::fs::read_to_string(root.join("src/provider2/provider2.rs"))
        .expect("the second face is on disk");
    assert!(
        generated.contains("FastPreset") && generated.contains("FastParts"),
        "the template's shape fields reach a request that stated none: {generated}"
    );

    // A value the author wrote wins over the template's.
    // 作者写下的值优先于模板的。
    let explicit = add_face(
        &root,
        json!({"action":"add","fields":{"module":"provider3","kind":"Provider","provides":"cap.third","preset":"SlowPreset"},"apply":true}),
    )
    .expect("the third face is created");
    assert!(
        explicit.contains("follows the approved specimen `root/provider`"),
        "the template is still selected: {explicit}"
    );
    let generated = std::fs::read_to_string(root.join("src/provider3/provider3.rs"))
        .expect("the third face is on disk");
    assert!(
        generated.contains("SlowPreset") && !generated.contains("FastPreset"),
        "an explicit preset is not overwritten by the template: {generated}"
    );
    assert!(
        generated.contains("FastParts"),
        "a field the request omitted still comes from the template: {generated}"
    );

    let other = add_face(
        &root,
        json!({"action":"add","fields":{"module":"widget","kind":"Widget","provides":"cap.widget"},"apply":true}),
    )
    .expect("a face of another kind is created");
    assert!(
        other.contains("no approved specimen for this category yet"),
        "an unrelated specimen is not read: {other}"
    );
    let _ = std::fs::remove_dir_all(root);
}

/// **Two in-force specimens of one family are a conflict; nothing is written.**
/// **同一家族的两个生效样板是冲突；什么都不写。**
#[test]
fn two_approved_specimens_refuse_the_add_and_write_nothing() {
    let root = package("conflict");
    for module in ["provider", "provider2"] {
        add_face(
            &root,
            json!({"action":"add","fields":{"module":module,"kind":"Provider","provides":"cap.render"},"apply":true}),
        )
        .expect("the specimen face is created");
    }
    adopt(&root, "root/provider");
    adopt(&root, "root/provider2");

    let refused = add_face(
        &root,
        json!({"action":"add","fields":{"module":"provider3","kind":"Provider","provides":"cap.third"},"apply":true}),
    )
    .expect_err("two approved specimens are a conflict");
    assert!(
        refused.contains("2 approved specimens")
            && refused.contains("root/provider")
            && refused.contains("root/provider2"),
        "the conflict names both specimens: {refused}"
    );
    assert!(
        !root.join("src/provider3/provider3.rs").exists(),
        "a conflict writes nothing"
    );
    let _ = std::fs::remove_dir_all(root);
}

/// **A template that changed between preview and apply refuses the write.**
/// **预览与落盘之间模板变了，写入被拒绝。**
///
/// The preview promised "this face follows that approved specimen"; once its lease lapses the promise
/// cannot be kept by writing anyway, because the delivered plan would be a different one.
/// 预览承诺"这个面跟随那个已批准样板"；租约失效后不能靠"照样写"兑现它——交付的会是另一份计划。
#[test]
fn a_template_that_changes_between_preview_and_apply_is_refused() {
    let root = package("stale");
    add_face(
        &root,
        json!({"action":"add","fields":{"module":"provider","kind":"Provider","provides":"cap.render"},"apply":true}),
    )
    .expect("the specimen is created");
    adopt(&root, "root/provider");
    let preview = add_face(
        &root,
        json!({"action":"add","fields":{"module":"provider2","kind":"Provider","provides":"cap.other"},"apply":false}),
    )
    .expect("the add previews");
    assert!(
        preview.contains("template: follows the approved specimen `root/provider`"),
        "the preview names the template it promises: {preview}"
    );
    // The covered bytes move, so the lease lapses before the write.
    // 被覆盖的字节变了，因此租约在写入前失效。
    std::fs::write(root.join("policy.txt"), "fixture policy version 2\n").expect("policy moved");
    let refused = add_face(
        &root,
        json!({"action":"add","fields":{"module":"provider2","kind":"Provider","provides":"cap.other"},"apply":true}),
    )
    .expect_err("the previewed template is no longer the template");
    assert!(
        refused.contains("approved template this preview followed changed")
            && refused.contains("was `root/provider`")
            && refused.contains("now none"),
        "the refusal names both the promise and the change: {refused}"
    );
    assert!(
        !root.join("src/provider2/provider2.rs").exists(),
        "nothing is written when the promised template moved"
    );
    let _ = std::fs::remove_dir_all(root);
}

/// **A candidate is recorded, and a later adoption names it as following the new specimen.**
/// **候选被记录；之后的采信点名它跟随新样板。**
///
/// This is the loop the review asked for: a new category produces a candidate, and a person's
/// adoption — not the creation — is what turns the candidate into a template-follower.
/// 这是复核要求的闭环：新类别产生候选，而把它变成"跟随模板"的是**人的采信**，不是创建本身。
#[test]
fn a_candidate_is_recorded_and_a_later_specimen_names_it() {
    let root = package("candidate");
    // A project that keeps state: the candidate record lives beside the rules/adoption ledger.
    // 一个保存状态的项目：候选记录与规则/采信台账住在一起。
    adopt(&root, "policy");
    let first = add_face(
        &root,
        json!({"action":"add","fields":{"module":"widget","kind":"Widget"},"apply":true}),
    )
    .expect("the first widget is created");
    assert!(
        first.contains("this face is a candidate"),
        "an unknown family is a candidate: {first}"
    );
    let recorded = std::fs::read_to_string(root.join(".xirang/templates/candidates.tsv"))
        .expect("the candidate is recorded");
    assert!(recorded.contains("root/widget\troot\tWidget"), "{recorded}");

    add_face(
        &root,
        json!({"action":"add","fields":{"module":"widget2","kind":"Widget"},"apply":true}),
    )
    .expect("the second widget is created");
    let adoption = adopt(&root, "root/widget");
    assert!(
        adoption.contains("candidate(s) in this family now follow this specimen")
            && adoption.contains("root/widget2"),
        "the adoption names the candidate that now has a template: {adoption}"
    );
    let third = add_face(
        &root,
        json!({"action":"add","fields":{"module":"widget3","kind":"Widget"},"apply":true}),
    )
    .expect("the third widget follows the adopted specimen");
    assert!(
        third.contains("follows the approved specimen `root/widget`"),
        "after the adoption the family has a template: {third}"
    );
    let lineage = std::fs::read_to_string(root.join(".xirang/templates/lineage.tsv"))
        .expect("the follower is recorded");
    assert!(
        lineage.contains("root/widget3\troot/widget\t"),
        "the object and the template version are on disk: {lineage}"
    );
    // The same record is readable from the adoption side: renewing the lease names the followers,
    // which is the set a template change would have to carry.
    // 同一份记录也能从采信侧读到：续签租约时点名跟随者，那正是模板变化要带走的一组。
    let renewal = adopt(&root, "root/widget");
    assert!(
        renewal.contains("template followers: root/widget3"),
        "the renewal names the objects following this specimen: {renewal}"
    );
    // Move the bytes the lease rests on, then re-adopt: the follower is now on an earlier version,
    // which is exactly the set a template change would have to carry.
    // 改动租约依托的字节后重新采信：跟随者现在停在旧版本上，那正是模板变化要带走的一组。
    std::fs::write(root.join("policy.txt"), "fixture policy version 2\n").expect("policy moved");
    let newer = adopt(&root, "root/widget");
    assert!(
        newer.contains("followers on an earlier template version: root/widget3"),
        "a follower recorded on the old version is named as needing the new template: {newer}"
    );
}

/// The lineage file, read the way a person would.
/// 谱系文件，按人会用的方式读出。
fn lineage(root: &std::path::Path) -> String {
    std::fs::read_to_string(root.join(".xirang/templates/lineage.tsv")).unwrap_or_default()
}

/// **A move carries every lineage row it relocates, and strands the template anchors it moves.**
/// **搬动会带走它改写的每一行谱系，并点出被它挪走的模板锚点。**
///
/// The object column is a path this move changes, so it is rewritten mechanically — the follower at
/// the moved face, the follower **under** it (a prefix, not an exact path), and nothing else. The
/// anchor column names a template through the adoption ledger, which still spells the old name:
/// rewriting it would invent a decision nobody made, so it is reported instead.
/// 对象列是这次搬动会改掉的路径，因此机械改写——被搬面上的跟随者、它**下面**的跟随者（前缀而非精确路径），
/// 其余不动。锚点列经采信台账点名模板，而台账仍写旧名字：改写它等于发明一个没人做过的决定，因此改为报告。
#[test]
fn a_move_carries_every_lineage_row_it_relocates() {
    let root = package("carry");
    adopt(&root, "policy");
    add_face(
        &root,
        json!({"action":"add","fields":{"module":"widget","kind":"Widget"},"apply":true}),
    )
    .expect("the specimen is created");
    adopt(&root, "root/widget");
    add_face(
        &root,
        json!({"action":"add","fields":{"module":"widget2","kind":"Widget","needs_registry":true},
                "apply":true}),
    )
    .expect("the follower that owns a registry is created");
    add_face(
        &root,
        json!({"action":"add","parent":"root/widget2",
                "fields":{"module":"child","kind":"Child"},"apply":true}),
    )
    .expect("a nested specimen is created");
    adopt(&root, "root/widget2/child");
    add_face(
        &root,
        json!({"action":"add","parent":"root/widget2",
                "fields":{"module":"child2","kind":"Child"},"apply":true}),
    )
    .expect("the nested follower is created");
    add_face(
        &root,
        json!({"action":"add","fields":{"module":"other","kind":"Widget"},"apply":true}),
    )
    .expect("a bystander follower is created");
    add_face(
        &root,
        json!({"action":"add","fields":{"module":"panel","kind":"Panel","needs_registry":true},
                "apply":true}),
    )
    .expect("the destination parent is created");
    let before = lineage(&root);
    assert!(
        before.contains("root/widget2\troot/widget\t")
            && before.contains("root/widget2/child2\troot/widget2/child\t")
            && before.contains("root/other\troot/widget\t"),
        "the fixture has the exact row, the prefixed row and the bystander: {before}"
    );

    // A preview promises the carry without writing it.
    // 预览承诺这次搬运，但不写它。
    let preview = add_face(
        &root,
        json!({"action":"move","node":"root/widget2","to":"root/panel","apply":false}),
    )
    .expect("the move previews");
    assert!(
        preview.contains("would rewrite 2 template lineage row(s)"),
        "the preview counts the row at the face and the row under it: {preview}"
    );
    assert!(
        preview.contains(
            "lineage: the template anchor `root/widget2/child` named an object this move relocated"
        ),
        "the preview names the template anchor the move strands: {preview}"
    );
    assert_eq!(lineage(&root), before, "a preview writes no lineage");

    let moved = add_face(
        &root,
        json!({"action":"move","node":"root/widget2","to":"root/panel","apply":true}),
    )
    .expect("the move applies");
    assert!(
        moved.contains("lineage root/widget2 -> root/panel/widget2")
            && moved.contains("lineage root/widget2/child2 -> root/panel/widget2/child2"),
        "the write names both rows it carried: {moved}"
    );
    let after = lineage(&root);
    assert!(
        after.contains("root/panel/widget2\troot/widget\t")
            && after.contains("root/panel/widget2/child2\troot/widget2/child\t"),
        "both followers name their new paths, at the versions they followed: {after}"
    );
    assert!(
        !after.contains("\nroot/widget2\t") && !after.contains("\nroot/widget2/child2\t"),
        "no row keeps a path the tree no longer has: {after}"
    );
    assert!(
        after.contains("root/other\troot/widget\t"),
        "a follower outside the move is byte for byte untouched: {after}"
    );
    assert!(
        after.contains("\troot/widget2/child\t"),
        "the anchor column is not rewritten: the adoption ledger still spells that name: {after}"
    );

    // The consumer reads the carried row: the follower is named at its new path.
    // 消费方读到被搬走的那一行：跟随者以新路径被点名。
    let renewal = adopt(&root, "root/widget");
    assert!(
        renewal.contains("root/panel/widget2") && renewal.contains("root/other"),
        "the adoption names the followers at the paths they now have: {renewal}"
    );

    // A move no row names touches no byte and says nothing about the lineage.
    // 没有任何行点名的搬动，一个字节也不碰，也不提谱系。
    add_face(
        &root,
        json!({"action":"add","fields":{"module":"gauge","kind":"Gauge"},"apply":true}),
    )
    .expect("a face no lineage row names is created");
    let carried = lineage(&root);
    let unrelated = add_face(
        &root,
        json!({"action":"move","node":"root/gauge","to":"root/panel","apply":true}),
    )
    .expect("the unrelated move applies");
    assert!(
        !unrelated.contains("lineage"),
        "a move that carries no follower stays out of the lineage story: {unrelated}"
    );
    assert_eq!(lineage(&root), carried, "and writes no lineage byte");
    let _ = std::fs::remove_dir_all(root);
}

/// **A delete names the lineage it strands, and its consumer never promises a follower that is gone.**
/// **删除点名它留下的谱系，而它的消费方永不承诺一个已经不在的跟随者。**
///
/// A delete is recoverable, so the rows are kept rather than erased: erasing them would make the
/// record's survival depend on an operation that can be undone. What changes is the reading — the
/// follower that no longer resolves is named as needing re-pointing, not offered as something a
/// template change would carry.
/// 删除是可恢复的，因此行被保留而不是擦掉：擦掉会让记录的存亡取决于一件可被撤销的操作。变的是**读法**——
/// 解析不到的跟随者被点名"需要重新指向"，而不是被说成"模板变化要带走"的对象。
#[test]
fn a_delete_names_the_lineage_it_strands_in_both_modes() {
    let root = package("strand");
    adopt(&root, "policy");
    add_face(
        &root,
        json!({"action":"add","fields":{"module":"widget","kind":"Widget"},"apply":true}),
    )
    .expect("the specimen is created");
    adopt(&root, "root/widget");
    for module in ["widget2", "widget3"] {
        add_face(
            &root,
            json!({"action":"add","fields":{"module":module,"kind":"Widget"},"apply":true}),
        )
        .expect("the follower is created");
    }
    let before = lineage(&root);
    assert!(
        before.contains("root/widget2\troot/widget\t")
            && before.contains("root/widget3\troot/widget\t"),
        "both followers are recorded: {before}"
    );

    let preview = add_face(
        &root,
        json!({"action":"delete","node":"root/widget2","confirm":true,"apply":false}),
    )
    .expect("the delete previews");
    assert!(
        preview.contains(
            "lineage: this delete removed the follower(s) root/widget2; their rows are kept"
        ),
        "the preview names the row it strands and says the row survives: {preview}"
    );
    assert_eq!(lineage(&root), before, "a preview writes no lineage");

    let applied = add_face(
        &root,
        json!({"action":"delete","node":"root/widget2","confirm":true,"apply":true}),
    )
    .expect("the delete applies");
    assert!(
        applied.contains("lineage: this delete removed the follower(s) root/widget2"),
        "the write says the same thing as the preview: {applied}"
    );
    assert!(
        lineage(&root).contains("root/widget2\troot/widget\t"),
        "the row is kept, because a delete is recoverable: {}",
        lineage(&root)
    );

    // The consumer splits the record by what is still there.
    // 消费方按"还在不在"切分这份记录。
    let renewal = adopt(&root, "root/widget");
    assert!(
        renewal.contains("template followers: root/widget3"),
        "the surviving follower is still the set a template change would carry: {renewal}"
    );
    assert!(
        renewal.contains(
            "followers whose lineage needs re-pointing (the object no longer resolves): root/widget2"
        ),
        "the removed follower is named as needing re-pointing: {renewal}"
    );
    assert!(
        !renewal.contains("template followers: root/widget2"),
        "and it is never listed as a follower this adoption could carry: {renewal}"
    );

    // Deleting the **template** names the objects that still need one, and does not claim anything
    // about a follower whose own row is already stale.
    // 删除**模板**点名仍然需要新模板的对象，而不对"行本身已经过期"的跟随者作任何声称。
    let orphaned = add_face(
        &root,
        json!({"action":"delete","node":"root/widget","confirm":true,"apply":true}),
    )
    .expect("the template is deleted");
    assert!(
        orphaned.contains(
            "lineage: the template this delete removed is followed by root/widget3; those objects \
             need a new template"
        ),
        "the write names the surviving follower that needs a new template: {orphaned}"
    );
    assert!(
        !orphaned.contains("followed by root/widget2"),
        "a follower whose object is already gone is not claimed as needing a new template: {orphaned}"
    );
    let _ = std::fs::remove_dir_all(root);
}
