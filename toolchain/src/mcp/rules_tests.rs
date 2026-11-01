//! Pins for the rule book, driven through its real entry.
//! 规则簿的钉子，从它的真实入口驱动。
//!
//! Every pin here is about what the surface **refuses**: an unconfirmed acceptance, an authority that
//! does not exist, a proposal that would constrain, a source that moved, and a newer rule that would
//! like to withdraw an older commitment. A rule book that only ever says "yes" is a list nobody can
//! rely on.
//! 这里的每条钉子都是关于这个执行面**拒绝**什么的：没确认的接受、不存在的授权、会约束的提议、动过的源，
//! 以及一条想把旧承诺撤掉的更新规则。一本只会说"是"的规则簿，是没人能依赖的清单。

use super::rules;
use serde_json::{Value, json};
use std::path::PathBuf;

/// A package with a file a rule can rest on, and one adoption entry it can be accepted against.
/// 一个有文件可供规则依托、并有一条采信条目可供接受的包。
fn fixture() -> PathBuf {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!("xirang-rules-{}-{sequence}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::create_dir_all(root.join(".xirang/adopted")).unwrap();
    let contents = "pub fn layout() -> i64 {\n    7\n}\n";
    std::fs::write(root.join("src/lib.rs"), contents).unwrap();
    // The lease's fingerprint is the **real** one now, because accepting a rule requires the
    // authority to be in force: a literal `deadbeef` would make every acceptance in these tests a
    // decision against a lapsed lease, which is the refusal the probe proved the surface needed.
    // 租约的指纹现在是**真的**，因为接受一条规则要求授权仍然生效：字面量 `deadbeef` 会让这些测试里的每一次
    // 接受都变成"对照失效租约做出的决定"，而那正是探针证明这个执行面需要的拒绝。
    let fingerprint = xirang_kernel::adoption::adoption_fingerprint(&[(
        "src/lib.rs".to_owned(),
        contents.to_owned(),
    )]);
    std::fs::write(
        root.join(".xirang/adopted/entries"),
        format!("# one line per decision; the fields are separator-tight, so a space around `|` becomes part of the field\ncontrol.object.button|consumers call the layout provider|converge answer|maintainer|2026-10-16|src/lib.rs|{fingerprint}|the provider is the shared route\n"),
    )
    .unwrap();
    root
}

/// Propose one rule against `src/lib.rs` and hand back the reply.
/// 依托 `src/lib.rs` 提议一条规则并交回回复。
fn propose(root: &std::path::Path, id: &str, extra: Value) -> String {
    let mut arguments = json!({
        "action":"propose","rule_id":id,"category":"route","selector":"control.object.button",
        "requirement":"calls the layout provider","sources":["src/lib.rs"],
        // A rule that is going to be accepted has to come from something already reviewed; the
        // `derived_candidate` default is pinned separately, because that refusal is the spec's
        // "one repair's experience is not a project rule".
        // 一条将被接受的规则必须来自已经复核过的东西；`derived_candidate` 默认值另有钉子，因为那条拒绝
        // 正是规格说的"一次修复的经验不是全项目规范"。
        "origin":"adopted_specimen"
    });
    for (key, value) in extra.as_object().cloned().unwrap_or_default() {
        arguments[key] = value;
    }
    rules(root, &arguments).expect("a proposal is written")
}

/// The refusal message of one call, which must name the rule rather than the symptom.
/// 一次调用的拒绝信息，必须点名规则而不是症状。
fn refused(root: &std::path::Path, arguments: Value) -> String {
    rules(root, &arguments).expect_err("this call must be refused")
}

/// **Nothing is accepted by silence, and a model's word is not an authority.**
/// **没有东西靠沉默被接受，模型的话不是授权。**
#[test]
fn acceptance_needs_confirm_and_an_authority_that_already_exists() {
    let root = fixture();
    propose(&root, "r-route", json!({}));
    let unconfirmed = refused(
        &root,
        json!({"action":"accept","rule_id":"r-route","authority":"control.object.button"}),
    );
    assert!(
        unconfirmed.contains("needs `confirm: true`"),
        "{unconfirmed}"
    );
    assert!(
        unconfirmed.contains("nobody agreed to"),
        "the refusal says what a silent acceptance would mean: {unconfirmed}"
    );
    let invented = refused(
        &root,
        json!({"action":"accept","rule_id":"r-route","authority":"policy:whatever","confirm":true}),
    );
    assert!(
        invented.contains("no adoption in") && invented.contains("policy:whatever"),
        "{invented}"
    );
    assert!(
        invented.contains("not against a model's own word"),
        "the refusal names the alternative the caller has: {invented}"
    );
    // And the experience one repair produced is not a project rule on its own.
    // 而一次修复产出的经验，本身不是全项目规范。
    propose(
        &root,
        "r-still-a-candidate",
        json!({"origin":"derived_candidate"}),
    );
    let too_early = refused(
        &root,
        json!({"action":"accept","rule_id":"r-still-a-candidate","authority":"control.object.button","confirm":true}),
    );
    assert!(
        too_early.contains("is not a project rule")
            && too_early.contains("adopted specimen or an accepted policy"),
        "{too_early}"
    );

    let accepted = rules(
        &root,
        &json!({"action":"accept","rule_id":"r-route","authority":"control.object.button","confirm":true}),
    )
    .unwrap();
    assert!(
        accepted.contains("accepted at revision 2")
            && accepted.contains("(an existing adoption entry)"),
        "{accepted}"
    );
    // Cleanup is not the assertion: a background index refresh may still be writing into the
    // fixture when the test ends, and `remove_dir_all` can lose that race.
    // 清理不是断言：后台索引刷新可能还在写夹具，`remove_dir_all` 会输掉这个竞态。
    let _ = std::fs::remove_dir_all(root);
}

/// A proposal constrains nothing, and the rule book says so on every read.
/// 提议什么都不约束，而规则簿每次读出来都会说明这一点。
#[test]
fn a_proposal_is_never_rendered_as_something_that_constrains() {
    let root = fixture();
    propose(&root, "r-route", json!({}));
    let listed = rules(&root, &json!({"action":"list"})).unwrap();
    assert!(listed.contains("r-route [proposed]"), "{listed}");
    assert!(
        listed.contains("does not constrain: a proposal is not a constraint"),
        "{listed}"
    );
    assert!(!listed.contains("unknown: this decider"), "{listed}");
    // Cleanup is not the assertion: a background index refresh may still be writing into the
    // fixture when the test ends, and `remove_dir_all` can lose that race.
    // 清理不是断言：后台索引刷新可能还在写夹具，`remove_dir_all` 会输掉这个竞态。
    let _ = std::fs::remove_dir_all(root);
}

/// An accepted rule whose source moves stops constraining, and `check` **records** that.
/// 源动过的已接受规则不再约束，而 `check` 把这件事**记下来**。
#[test]
fn a_moved_source_is_recorded_as_stale_and_a_vanished_one_also_stops_it() {
    let root = fixture();
    propose(&root, "r-route", json!({}));
    rules(
        &root,
        &json!({"action":"accept","rule_id":"r-route","authority":"control.object.button","confirm":true}),
    )
    .unwrap();
    let stable = rules(&root, &json!({"action":"check"})).unwrap();
    assert!(stable.contains("r-route [accepted]"), "{stable}");
    assert!(
        stable.contains("unknown: this decider does not evaluate `calls the layout provider`"),
        "a constraining rule is reported unknown with what it still needs, never met: {stable}"
    );

    std::fs::write(
        root.join("src/lib.rs"),
        "pub fn layout() -> i64 {\n    8\n}\n",
    )
    .unwrap();
    let moved = rules(&root, &json!({"action":"check"})).unwrap();
    assert!(moved.contains("r-route [stale]"), "{moved}");
    assert!(
        moved.contains("does not constrain: a source this rule was accepted against changed"),
        "{moved}"
    );
    // The decay is on the record now, not recomputed per reader.
    // 衰减现在落在记录上，而不是每个读者重算一遍。
    let listed = rules(&root, &json!({"action":"list"})).unwrap();
    assert!(listed.contains("r-route [stale]"), "{listed}");

    // A source that is gone is not a source that matches.
    // 不在的源不是匹配的源。
    std::fs::remove_file(root.join("src/lib.rs")).unwrap();
    let gone = rules(&root, &json!({"action":"list"})).unwrap();
    assert!(
        gone.contains("does not constrain: a source this rule was accepted against changed")
            || gone.contains("is no longer here"),
        "a vanished source must not read as unchanged: {gone}"
    );
    // Cleanup is not the assertion: a background index refresh may still be writing into the
    // fixture when the test ends, and `remove_dir_all` can lose that race.
    // 清理不是断言：后台索引刷新可能还在写夹具，`remove_dir_all` 会输掉这个竞态。
    let _ = std::fs::remove_dir_all(root);
}

/// §2.2 step 6 as a pin: two accepted opposites over one scope are both `conflicted`, and the newer
/// one does not withdraw the older.
/// §2.2 第 6 条作为钉子：同一作用域上两条相反的已接受规则都被记为 `conflicted`，而更新的那条不撤回旧的。
#[test]
fn a_newer_opposite_never_withdraws_the_older_commitment() {
    let root = fixture();
    for (id, polarity) in [("r-old", "require"), ("r-new", "forbid")] {
        propose(&root, id, json!({"polarity": polarity}));
        rules(
            &root,
            &json!({"action":"accept","rule_id":id,"authority":"control.object.button","confirm":true}),
        )
        .unwrap();
    }
    let checked = rules(&root, &json!({"action":"check"})).unwrap();
    assert!(checked.contains("r-old [conflicted]"), "{checked}");
    assert!(checked.contains("r-new [conflicted]"), "{checked}");
    assert!(
        checked.matches("a person resolves this").count() >= 2,
        "both sides are told a person resolves it: {checked}"
    );
    assert!(
        !checked.contains("r-old [retired]") && !checked.contains("r-new [retired]"),
        "neither is withdrawn: {checked}"
    );
    // ME05: a reader resolving the conflict sees **both** scopes and **both** sources, not just two
    // ids — "there is a conflict somewhere" is not enough to decide which commitment to revisit.
    // ME05：解决冲突的读者看到**两边**的作用域与**两边**的源，而不只是两个 id——"某处有冲突"不足以
    // 判断该重新审视哪一份承诺。
    assert!(
        checked.matches("control.object.button").count() >= 2,
        "both sides print the scope they claim: {checked}"
    );
    assert!(
        checked.matches("sources src/lib.rs").count() >= 2,
        "both sides print the source they rest on: {checked}"
    );
    // Cleanup is not the assertion: a background index refresh may still be writing into the
    // fixture when the test ends, and `remove_dir_all` can lose that race.
    // 清理不是断言：后台索引刷新可能还在写夹具，`remove_dir_all` 会输掉这个竞态。
    let _ = std::fs::remove_dir_all(root);
}

/// A retirement needs a reason, and a rule that was written down is revised rather than overwritten.
/// 撤回需要理由，而写下过的规则是被修订而不是被覆盖。
#[test]
fn retirement_needs_a_reason_and_a_rule_is_not_overwritten() {
    let root = fixture();
    propose(&root, "r-route", json!({}));
    let again = refused(
        &root,
        json!({"action":"propose","rule_id":"r-route","category":"route","selector":"control.object.button","requirement":"calls the layout provider"}),
    );
    assert!(
        again.contains("is already recorded")
            && again.contains("a rule is revised, not overwritten"),
        "{again}"
    );
    let no_reason = refused(
        &root,
        json!({"action":"retire","rule_id":"r-route","confirm":true}),
    );
    assert!(no_reason.contains("`reason` is required"), "{no_reason}");
    let retired = rules(
        &root,
        &json!({"action":"retire","rule_id":"r-route","reason":"superseded by the v3 contract","confirm":true}),
    )
    .unwrap();
    assert!(retired.contains("state retired"), "{retired}");
    assert!(retired.contains("never silent"), "{retired}");
    let listed = rules(&root, &json!({"action":"list"})).unwrap();
    assert!(
        listed.contains("does not constrain: this rule was withdrawn on purpose"),
        "{listed}"
    );
    // Cleanup is not the assertion: a background index refresh may still be writing into the
    // fixture when the test ends, and `remove_dir_all` can lose that race.
    // 清理不是断言：后台索引刷新可能还在写夹具，`remove_dir_all` 会输掉这个竞态。
    let _ = std::fs::remove_dir_all(root);
}

/// The entry refuses what it would have to guess: an action it does not have, an id that would become
/// a path, and a source that leaves the root.
/// 入口拒绝它必须靠猜的东西：没有的动作、会变成路径的 id、以及离开根的源。
#[test]
fn the_entry_refuses_an_unknown_action_a_path_shaped_id_and_an_escaping_source() {
    let root = fixture();
    let action = refused(&root, json!({"action":"accept-all"}));
    assert!(
        action.contains("not `accept-all`") && action.contains("nothing was done"),
        "{action}"
    );
    let id = refused(
        &root,
        json!({"action":"propose","rule_id":"../../etc/passwd","category":"route","selector":"s","requirement":"r"}),
    );
    assert!(id.contains("it is also the file name"), "{id}");
    let escaping = refused(
        &root,
        json!({"action":"propose","rule_id":"r-x","category":"route","selector":"s","requirement":"r","sources":["../outside.rs"]}),
    );
    assert!(escaping.contains("package-relative"), "{escaping}");
    assert!(
        !root.join(".xirang/rules").exists()
            || std::fs::read_dir(root.join(".xirang/rules"))
                .unwrap()
                .next()
                .is_none(),
        "and nothing was written"
    );
    // Cleanup is not the assertion: a background index refresh may still be writing into the
    // fixture when the test ends, and `remove_dir_all` can lose that race.
    // 清理不是断言：后台索引刷新可能还在写夹具，`remove_dir_all` 会输掉这个竞态。
    let _ = std::fs::remove_dir_all(root);
}

/// An exception is a decision, so it is confirmed, attributed, and confined to one scope.
/// 例外是一个决定，因此它要被确认、被归属，并且被限制在一个作用域里。
#[test]
fn an_exception_is_confirmed_attributed_and_confined_to_its_own_scope() {
    let root = fixture();
    propose(&root, "r-route", json!({}));
    rules(
        &root,
        &json!({"action":"accept","rule_id":"r-route","authority":"control.object.button","confirm":true}),
    )
    .unwrap();
    let unconfirmed = refused(
        &root,
        json!({"action":"exception","rule_id":"r-route","scope":"control.object.button.legacy","reason":"predates the provider","authority":"control.object.button"}),
    );
    assert!(
        unconfirmed.contains("needs `confirm: true`"),
        "{unconfirmed}"
    );
    let invented = refused(
        &root,
        json!({"action":"exception","rule_id":"r-route","scope":"control.object.button.legacy","reason":"predates the provider","authority":"policy:made-up","confirm":true}),
    );
    assert!(
        invented.contains("not against a model's own word"),
        "an exception without an authority is a rule that quietly stopped applying: {invented}"
    );
    let granted = rules(
        &root,
        &json!({"action":"exception","rule_id":"r-route","scope":"control.object.button.legacy","reason":"predates the provider","authority":"control.object.button","confirm":true}),
    )
    .unwrap();
    assert!(granted.contains("exception at revision 3"), "{granted}");
    assert!(
        granted.contains("covers `control.object.button.legacy` and nothing else"),
        "the reply must say how narrow it is: {granted}"
    );
    let again = refused(
        &root,
        json!({"action":"exception","rule_id":"r-route","scope":"control.object.button.legacy","reason":"predates the provider","authority":"control.object.button","confirm":true}),
    );
    assert!(again.contains("already has an exception"), "{again}");
    // Cleanup is not the assertion: a background index refresh may still be writing into the
    // fixture when the test ends, and `remove_dir_all` can lose that race.
    // 清理不是断言：后台索引刷新可能还在写夹具，`remove_dir_all` 会输掉这个竞态。
    let _ = std::fs::remove_dir_all(root);
}

/// A rule that names an expectation is **decided**; a rule that names none stays `unknown` and says
/// what it would need. The first is what turns the rule book from "may it constrain" into "does it
/// hold", and the second is what keeps an undecidable rule from reading as a pass.
/// 点名了期望的规则会被**判定**；没有点名的照旧 `unknown` 并说清它需要什么。前者把规则簿从"能不能约束"
/// 变成"成不成立"，后者让判不了的规则不被读成通过。
#[test]
fn a_rule_that_names_an_expectation_is_decided_and_one_that_does_not_stays_unknown() {
    let root = fixture();
    propose(
        &root,
        "r-decided",
        json!({"expects": {"provides": "cap.render"}}),
    );
    propose(&root, "r-blind", json!({}));
    for id in ["r-decided", "r-blind"] {
        rules(
            &root,
            &json!({"action":"accept","rule_id":id,"authority":"control.object.button","confirm":true}),
        )
        .unwrap();
    }
    let checked = rules(&root, &json!({"action":"check"})).unwrap();
    // This fixture has no `Cargo.toml`, so the decision cannot be taken — but it **is attempted**,
    // and the refusal to decide comes from the decider rather than from the rule book's old blanket
    // sentence. That is the difference this pin exists to hold.
    // 这个夹具没有 `Cargo.toml`，因此判定做不成——但它**被尝试过**了，而且拒绝判定来自判定器，而不是规则簿
    // 原来那句一刀切的话。这正是这条钉子要钉住的区别。
    assert!(
        checked.contains("cannot learn the identity namespace"),
        "the rule that names an expectation reaches the decider: {checked}"
    );
    assert!(
        checked.contains("unknown: this decider does not evaluate `calls the layout provider`"),
        "the rule that names none keeps the old wording: {checked}"
    );
    // A second pair would be dropped by the decider, so it is refused where it is written.
    // 第二对会被判定器丢掉，因此在写入处就拒绝。
    let two = refused(
        &root,
        json!({"action":"propose","rule_id":"r-two","category":"route","selector":"s","requirement":"r","expects":{"provides":"a","declares":"b"}}),
    );
    assert!(two.contains("a second pair would be dropped"), "{two}");
    // Cleanup is not the assertion: a background index refresh may still be writing into the
    // fixture when the test ends, and `remove_dir_all` can lose that race.
    // 清理不是断言：后台索引刷新可能还在写夹具，`remove_dir_all` 会输掉这个竞态。
    let _ = std::fs::remove_dir_all(root);
}

/// A package with a **real registration face** and a lease that is actually in force.
/// 一个有**真实注册面**、且租约确实生效的包。
///
/// The rule book's predicate goes through the same decider `constraints` uses, so a fixture that
/// merely writes JSON proves nothing about polarity or exemptions: the face has to exist in the
/// tree, and the adoption fingerprint has to be the one its bytes produce.
/// 规则簿的谓词经过与 `constraints` 相同的判定器，因此一个只写 JSON 的夹具证明不了极性或豁免：面必须真实
/// 存在于树里，采信指纹必须是它的字节产生的那一个。
fn face_fixture() -> PathBuf {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "xirang-rules-face-{}-{sequence}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::write(
        root.join("Cargo.toml"),
        "[package]\nname=\"rules-face-probe\"\nversion=\"0.1.0\"\nedition=\"2024\"\n[workspace]\n",
    )
    .unwrap();
    std::fs::write(root.join("src/lib.rs"), "// inspection fixture\n").unwrap();
    std::fs::write(root.join("policy.txt"), "fixture policy version 1\n").unwrap();
    crate::mcp::apply::apply(
        &root,
        &json!({"action":"add","fields":{"module":"provider","kind":"Provider","provides":"cap.render"},"apply":true}),
    )
    .expect("the probe face is created");
    crate::mcp::adopted::adopted(
        &root,
        &json!({"anchor":"policy","certifies":"fixture policy","evidence":"desk check","verifier":"rules test","reason":"the fixture policy is the authority","files":["policy.txt"],"apply":true,"confirm":true}),
    )
    .expect("the lease is written");
    root
}

/// Accept one rule against the fixture's policy lease.
/// 对照夹具的 policy 租约接受一条规则。
fn accept(root: &std::path::Path, id: &str) {
    rules(
        root,
        &json!({"action":"accept","rule_id":id,"authority":"policy","confirm":true}),
    )
    .unwrap();
}

/// **A forbidden expectation is met exactly when the object stops providing it.**
/// **禁止性期望恰在对象不再提供它时被满足。**
///
/// The probe's F1: the old surface mapped the underlying predicate onto the verdict word without
/// interpreting polarity, so a `forbid` over a capability the object *did* provide answered `met`,
/// and the same rule over a tree that stopped providing it answered `violated`.
/// 探针的 F1：旧执行面把底层谓词直接映射成判决词，没有解释极性，于是"禁止对象提供某能力"在对象**确实**
/// 提供它时报 `met`，而在树不再提供它时报 `violated`。
#[test]
fn a_forbidden_expectation_is_met_only_when_the_capability_is_absent() {
    let root = face_fixture();
    propose(
        &root,
        "no-render",
        json!({
            "category":"protocol","selector":"root/provider","polarity":"forbid",
            "requirement":"must not provide cap.render",
            "expects":{"provides":"cap.render"},
            "sources":[],
            "origin":"accepted_policy"
        }),
    );
    accept(&root, "no-render");
    let providing = rules(&root, &json!({"action":"list"})).unwrap();
    assert!(
        providing.contains("violated (structural)"),
        "an object that provides the forbidden capability violates the rule: {providing}"
    );
    assert!(
        !providing.contains("met (structural)"),
        "the old inversion must be gone: {providing}"
    );
    // The positive side: the same rule over a tree that stopped providing it.
    crate::mcp::apply::apply(
        &root,
        &json!({"action":"edit","node":"root/provider","fields":{"provides":"cap.other"},"apply":true}),
    )
    .expect("the face is edited");
    let absent = rules(&root, &json!({"action":"list"})).unwrap();
    assert!(
        absent.contains("met (structural)"),
        "a tree that no longer provides it satisfies the prohibition: {absent}"
    );
    // Cleanup is not the assertion: a background index refresh may still be writing into the
    // fixture when the test ends, and `remove_dir_all` can lose that race.
    // 清理不是断言：后台索引刷新可能还在写夹具，`remove_dir_all` 会输掉这个竞态。
    let _ = std::fs::remove_dir_all(root);
}

/// **An exception changes the verdict, and the reply names the scope and the authority.**
/// **例外改变判定，且回复点明作用域与授权。**
///
/// The probe's F4: the exception was written and the tool said it covered the object, but the list
/// kept answering `violated`. The kernel had `exception_for`; nothing consumed it.
/// 探针的 F4：例外写下了、工具也说它覆盖该对象，但列举继续报 `violated`。内核有 `exception_for`，
/// 却没有任何东西消费它。
#[test]
fn an_exception_changes_the_verdict_and_names_its_authority() {
    let root = face_fixture();
    propose(
        &root,
        "keep-render",
        json!({
            "category":"protocol","selector":"root/provider","polarity":"require",
            "requirement":"retain cap.render",
            "expects":{"provides":"cap.render"},
            "sources":[],
            "origin":"accepted_policy"
        }),
    );
    accept(&root, "keep-render");
    assert!(
        rules(&root, &json!({"action":"list"}))
            .unwrap()
            .contains("met (structural)"),
        "the fixture provides cap.render"
    );
    // The violation is made **by hand**, not through `apply`: an apply that would violate an
    // accepted rule is refused by the plan gate (`P1`), and this pin is about what the rule book
    // says about a tree that is already in that state.
    // 违规是**手工**造出来的，不经过 `apply`：会违反已接受规则的 apply 会被计划门禁拒绝（`P1`），而这条
    // 钉子说的是规则簿对一个已经处于该状态的树怎么说。
    let source = root.join("src/provider/provider.rs");
    let text = std::fs::read_to_string(&source).expect("the face exists");
    std::fs::write(&source, text.replace("cap.render", "cap.other"))
        .expect("the fixture turns non-compliant");
    assert!(
        rules(&root, &json!({"action":"list"}))
            .unwrap()
            .contains("violated (structural)"),
        "the requirement is violated before the exception"
    );
    rules(
        &root,
        &json!({"action":"exception","rule_id":"keep-render","scope":"root/provider","reason":"predates the provider","authority":"policy","confirm":true}),
    )
    .unwrap();
    let exempt = rules(&root, &json!({"action":"list"})).unwrap();
    assert!(
        exempt.contains("exempt: predates the provider (scope root/provider, authority policy)"),
        "the exception must be the verdict, with its scope and authority: {exempt}"
    );
    assert!(
        !exempt.contains("violated (structural)"),
        "an exempt subject is not also reported as a violation: {exempt}"
    );
    // Cleanup is not the assertion: a background index refresh may still be writing into the
    // fixture when the test ends, and `remove_dir_all` can lose that race.
    // 清理不是断言：后台索引刷新可能还在写夹具，`remove_dir_all` 会输掉这个竞态。
    let _ = std::fs::remove_dir_all(root);
}

/// **A lapsed lease is not an authority for a new rule, and re-confirming restores it.**
/// **失效租约不是新规则的授权；重新确认后恢复。**
///
/// The probe's F3: after `conformance` reported `lapsed`, `rules accept` still succeeded because the
/// check was "an anchor with this name appears in the ledger", not "its newest record is in force".
/// 探针的 F3：`conformance` 报 `lapsed` 之后 `rules accept` 仍然成功，因为检查是"台账里出现过这个
/// anchor"，而不是"它最新的记录仍然生效"。
#[test]
fn a_lapsed_lease_cannot_be_the_authority_of_a_new_rule() {
    let root = face_fixture();
    std::fs::write(root.join("policy.txt"), "fixture policy version 2\n").unwrap();
    let conformance = crate::mcp::adopted::conformance(&root, &json!({"anchor":"policy"})).unwrap();
    assert!(conformance.contains("lapsed"), "{conformance}");
    propose(
        &root,
        "after-lapse",
        json!({"category":"protocol","selector":"root/provider","requirement":"fixture","sources":["policy.txt"],"origin":"accepted_policy"}),
    );
    let refused_after_lapse = refused(
        &root,
        json!({"action":"accept","rule_id":"after-lapse","authority":"policy","confirm":true}),
    );
    assert!(
        refused_after_lapse.contains("no longer in force"),
        "a lapsed lease must be named as such: {refused_after_lapse}"
    );
    assert!(
        refused_after_lapse.contains("needs a person") || refused_after_lapse.contains("renew"),
        "the refusal says the lease is exactly what a person has to renew: {refused_after_lapse}"
    );
    let refused_exception = refused(
        &root,
        json!({"action":"exception","rule_id":"after-lapse","scope":"root/provider","reason":"x","authority":"policy","confirm":true}),
    );
    assert!(
        refused_exception.contains("no longer in force"),
        "an exception is a decision too, and it cannot be granted on a lapsed lease: {refused_exception}"
    );
    // Re-confirming the policy through the real entry makes it current again.
    crate::mcp::adopted::adopted(
        &root,
        &json!({"anchor":"policy","certifies":"fixture policy","evidence":"desk check","verifier":"rules test","reason":"re-confirmed after the policy edit","files":["policy.txt"],"apply":true,"confirm":true}),
    )
    .expect("the lease is re-confirmed");
    accept(&root, "after-lapse");
    // Cleanup is not the assertion: a background index refresh may still be writing into the
    // fixture when the test ends, and `remove_dir_all` can lose that race.
    // 清理不是断言：后台索引刷新可能还在写夹具，`remove_dir_all` 会输掉这个竞态。
    let _ = std::fs::remove_dir_all(root);
}

/// **A listing that was not given the subject's context does not claim a verdict.**
/// **没有被提供主体上下文的列举不声称判决。**
#[test]
fn a_listing_without_the_subject_context_reports_unknown() {
    let root = face_fixture();
    propose(
        &root,
        "release-only",
        json!({
            "category":"shape","selector":"root/provider",
            "requirement":"provides cap.render","expects":{"provides":"cap.render"},
            "roles":["LocalPosition"],"configurations":["release"],
            "sources":[],"origin":"accepted_policy"
        }),
    );
    accept(&root, "release-only");
    let no_context = rules(&root, &json!({"action":"list"})).unwrap();
    assert!(
        no_context.contains("unknown:") && no_context.contains("given no roles"),
        "a rule that speaks about roles cannot be decided without them: {no_context}"
    );
    assert!(
        !no_context.contains("met (structural)"),
        "a context-free listing must not claim the rule holds: {no_context}"
    );
    let release = rules(
        &root,
        &json!({"action":"list","subject":{"roles":["LocalPosition"],"configuration":"release"}}),
    )
    .unwrap();
    assert!(
        release.contains("met (structural)"),
        "with the subject's context the rule is decided: {release}"
    );
    let debug = rules(
        &root,
        &json!({"action":"list","subject":{"roles":["LocalPosition"],"configuration":"debug"}}),
    )
    .unwrap();
    assert!(
        debug.contains("not applicable: the subject is not under a configuration"),
        "a rule accepted for release does not apply to debug: {debug}"
    );
    // Cleanup is not the assertion: a background index refresh may still be writing into the
    // fixture when the test ends, and `remove_dir_all` can lose that race.
    // 清理不是断言：后台索引刷新可能还在写夹具，`remove_dir_all` 会输掉这个竞态。
    let _ = std::fs::remove_dir_all(root);
}

/// **An undecidable expectation is `unknown`, never `met`.**
/// **判不了的期望是 `unknown`，绝不是 `met`。**
#[test]
fn an_undecidable_expectation_is_never_rendered_as_met() {
    let root = face_fixture();
    propose(
        &root,
        "missing-object",
        json!({
            "category":"route","selector":"root/nowhere",
            "requirement":"provides cap.render","expects":{"provides":"cap.render"},
            "sources":[],"origin":"accepted_policy"
        }),
    );
    accept(&root, "missing-object");
    let listed = rules(&root, &json!({"action":"list"})).unwrap();
    assert!(
        listed.contains("unknown:") && listed.contains("no face in this tree"),
        "the decider's own reason is what a reader needs: {listed}"
    );
    assert!(!listed.contains("met (structural)"), "{listed}");
    // Cleanup is not the assertion: a background index refresh may still be writing into the
    // fixture when the test ends, and `remove_dir_all` can lose that race.
    // 清理不是断言：后台索引刷新可能还在写夹具，`remove_dir_all` 会输掉这个竞态。
    let _ = std::fs::remove_dir_all(root);
}

/// **Two rules that name the same typed expectation conflict even when they word it differently.**
/// **点名同一类型化期望的两条规则，即使措辞不同也冲突。**
#[test]
fn two_rules_over_the_same_typed_expectation_conflict_despite_different_wording() {
    let root = face_fixture();
    for (id, polarity, requirement) in [
        ("keep-render", "require", "retain the render capability"),
        ("no-render", "forbid", "must not provide cap.render"),
    ] {
        propose(
            &root,
            id,
            json!({
                "category":"protocol","selector":"root/provider","polarity":polarity,
                "requirement":requirement,"expects":{"provides":"cap.render"},
                "sources":[],"origin":"accepted_policy"
            }),
        );
        accept(&root, id);
    }
    let checked = rules(&root, &json!({"action":"check"})).unwrap();
    assert!(
        checked.contains("keep-render [conflicted]") && checked.contains("no-render [conflicted]"),
        "a typed contradiction is not resolved by wording: {checked}"
    );
    assert!(
        checked.matches("a person resolves this").count() >= 2,
        "both sides are told a person resolves it: {checked}"
    );
    // Cleanup is not the assertion: a background index refresh may still be writing into the
    // fixture when the test ends, and `remove_dir_all` can lose that race.
    // 清理不是断言：后台索引刷新可能还在写夹具，`remove_dir_all` 会输掉这个竞态。
    let _ = std::fs::remove_dir_all(root);
}
