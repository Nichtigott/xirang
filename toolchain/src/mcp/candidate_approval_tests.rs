//! One approval, many revisions: what the ruling means by "范围内的后续编辑不必反复询问用户".
//! 一次批准、多次修订：裁定里"范围内的后续编辑不必反复询问用户"是什么意思。

use serde_json::{Value, json};

use super::candidate;
use super::candidate_tests::{
    PASSING, approve, approve_replacing, confirm, fixture, opened, refused,
};

/// Ruling §1 gives the agent two things: request a scope, and **see the state**. This pin holds the second one — with no
/// approval, with an approval for this candidate, and with one for another candidate — because before it existed the
/// only way to learn "am I allowed to land this?" was to attempt a write.
/// 裁定 §1 给 agent 两样东西：提出范围申请，以及**查看状态**。这道钉子守住第二样——没有批准、有本候选的批准、
/// 有别的候选的批准三种情形——因为在它存在之前，想知道"这棵我能落地吗"只有去试一次写。
#[test]
fn status_reports_the_authorization_the_landing_would_be_decided_on() {
    let root = fixture();
    let (id, tree) = opened(&root, json!(["src"]));
    std::fs::write(tree.join("src/lib.rs"), PASSING).unwrap();
    candidate(&root, &json!({"action":"verify","id":id,"face":"default"})).unwrap();
    // ① Nothing is approved: the reply says a landing would be refused, and says who can change that. The fixture's
    // `opened` issues an approval of its own, so the store is emptied first — otherwise this case would be untested.
    // ① 什么都没批：回复说落地会被拒，并说清谁能改变这一点。夹具的 `opened` 自己会签一份批准，因此先把存储清空——
    // 否则这一情形根本没被测到。
    let _ = std::fs::remove_dir_all(crate::mcp::controller::directory(&root));
    let none = candidate(&root, &json!({"action":"status","id":id})).unwrap();
    assert!(
        none.contains("approval none:") && none.contains("a land would be refused"),
        "status states the absence rather than staying silent: {none}"
    );
    assert!(
        none.contains("--confirm") && none.contains("no tool in this bridge reaches it"),
        "and names the host's confirmation entry, and says a tool cannot reach it: {none}"
    );
    // ② An approval for this candidate: the reply reads it back and says the gate would pass.
    // ② 本候选的批准：回复把它读回来，并说这道门会过。
    approve(&root, &id, &["src".to_owned()]);
    let covered = candidate(&root, &json!({"action":"status","id":id})).unwrap();
    assert!(
        covered.contains("approved by")
            && covered.contains("covers this candidate's declared scope"),
        "status reports the approval and that it covers this candidate: {covered}"
    );
    assert!(
        covered.contains("the other gates are reported separately"),
        "and refuses to be read as 'this will land': {covered}"
    );
    // The split is stated in the reply, and it is the **current** split: `budget` and `policy` moved to the compared
    // half on 2026-10-18 (M3: a limit that is only printed is not a limit — the field was read by nothing while the
    // receipt printed `budget executions=8`). `approver` and `channel` stay provenance, and saying so is what keeps
    // "approved by maintainer" from reading as "someone was verified". The old assertion pinned the old split
    // (`never compared: approver, budget`); it is rewritten rather than dropped, so a future change that moves a
    // dimension back into the uncompared half without updating this line turns it red.
    // 回复说出这条分界，而且是**当前**的分界：2026-10-18 起 `budget` 与 `policy` 移入被比较的那一半（M3：只印出来的
    // 限制不是限制——那个字段此前没有任何东西读，而回执印着 `budget executions=8`）。`approver` 与 `channel` 仍是来源
    // 信息，说出来才能让"approved by maintainer"不被读成"某个人已被核实"。旧断言钉的是旧分界
    // （`never compared: approver, budget`）；这里是改写而不是删除，因此将来把某个维度悄悄挪回"不比较"的那一半会变红。
    assert!(
        covered.contains("compared on every use: budget")
            && covered.contains("compared on every use: budget, candidate")
            && covered.contains("never compared: approver, channel"),
        "the reply separates what is compared from what is only recorded: {covered}"
    );
    // ③ An approval that covers some **other** candidate: the reply says so, with the dimension that refuses.
    // ③ 覆盖**别的**候选的批准：回复说出来，并点名拒绝的那个维度。
    let (other, _) = opened(&root, json!(["src"]));
    // Opening the second candidate issued an approval for **it**; re-issue the first one's so the store's active
    // authorization is the one that does not name `other`.
    // 打开第二棵候选会为**它**签发一份批准；把第一棵的再签一次，让存储里生效的授权是不点名 `other` 的那份。
    approve(&root, &id, &["src".to_owned()]);
    let elsewhere = candidate(&root, &json!({"action":"status","id":other})).unwrap();
    assert!(
        elsewhere.contains("approval does not cover this candidate")
            && elsewhere.contains("an approval for one candidate never vouches for another"),
        "status names the dimension that refuses: {elsewhere}"
    );
    std::fs::remove_dir_all(root).unwrap();
}

/// The positive half: the approval is taken **before** the candidate's content is final, and a later revision inside the
/// declared scope lands under that same approval — no second signature.
/// 正的一半：批准在候选内容定稿**之前**拿到，之后在声明范围内的修订仍然落在同一份批准之下——不需要第二个签名。
///
/// This is the sentence the design relies on: a grant is bound to the candidate, the declared scope and the operation,
/// **not** to the bytes. A person who approved "change `src`, land it" does not have to be asked again because the
/// author improved the change inside that scope.
/// 这正是设计依赖的那句话：批准绑的是候选、声明的范围与操作，**不是**字节。批准了"改 `src` 并落地"的人，不该因为
/// 作者在该范围内把改动改得更好而被再问一次。
#[test]
fn one_approval_covers_a_revision_inside_the_declared_scope() {
    let root = fixture();
    let (id, tree) = opened(&root, json!(["src"]));
    approve(&root, &id, &["src".to_owned()]);
    std::fs::write(tree.join("src/lib.rs"), PASSING).unwrap();
    candidate(&root, &json!({"action":"verify","id":id,"face":"default"})).unwrap();
    // Before landing, the author improves the change: same scope, different bytes, re-verified.
    // 落地之前作者把改动改好：同样的范围、不同的字节、重新验证。
    std::fs::write(
        tree.join("src/extra.rs"),
        "pub fn extra() -> i64 {\n    7\n}\n",
    )
    .unwrap();
    candidate(&root, &json!({"action":"verify","id":id,"face":"default"})).unwrap();
    let record: Value = serde_json::from_str(
        &std::fs::read_to_string(root.join(format!("target/xirang/candidates/{id}.json"))).unwrap(),
    )
    .unwrap();
    assert_eq!(
        record["grant_revision"], "0",
        "the revision the approval covers did not move: no second signature was needed"
    );
    let landed = candidate(&root, &json!({"action":"land","id":id}))
        .expect("the approval taken before the revision still lands");
    assert!(
        landed.contains(" landed") && landed.contains("src/extra.rs"),
        "{landed}"
    );
    std::fs::remove_dir_all(root).unwrap();
}

/// The negative half: the same shape, but the revision leaves the declared scope — refused by the scope check, so "one
/// approval covers revisions" never becomes "one approval covers anything".
/// 负的一半：同样的形状，但修订越出声明范围——由范围检查拒绝，于是"一次批准覆盖修订"不会变成"一次批准覆盖一切"。
#[test]
fn the_same_approval_does_not_cover_a_revision_outside_the_declared_scope() {
    let root = fixture();
    let (id, tree) = opened(&root, json!(["README.md"]));
    approve(&root, &id, &["README.md".to_owned()]);
    std::fs::write(tree.join("README.md"), "a revised readme\n").unwrap();
    // A file the candidate's scope never named.
    // 一个候选范围里从未点名的文件。
    std::fs::write(tree.join("src/lib.rs"), PASSING).unwrap();
    candidate(&root, &json!({"action":"verify","id":id,"face":"default"})).unwrap();
    let refused = refused(&root, json!({"action":"land","id":id}));
    assert!(
        refused.contains("outside the declared scope") && refused.contains("src/lib.rs"),
        "the refusal names the path and the rule: {refused}"
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn an_approval_issued_under_another_policy_version_does_not_cover_this_use() {
    let root = fixture();
    let passing = "pub fn answer() -> i64 {\n    42\n}\n\n#[test]\nfn the_answer_is_stable() {\n    assert_eq!(answer(), 42);\n}\n";
    let (id, tree) = opened(&root, json!(["src"]));
    std::fs::write(tree.join("src/lib.rs"), passing).unwrap();
    candidate(&root, &json!({"action":"verify","id":id,"face":"default"})).unwrap();
    approve_replacing(&root, &id, &[("policy", "xirang.candidate/v0")]);
    let refused_by_policy = refused(&root, json!({"action":"land","id":id}));
    assert!(
        refused_by_policy.contains("policy `xirang.candidate/v1`")
            && refused_by_policy.contains("`xirang.candidate/v0`"),
        "the refusal names both versions rather than the symptom: {refused_by_policy}"
    );
    assert!(
        std::fs::read_to_string(root.join("src/lib.rs"))
            .unwrap()
            .contains("41"),
        "nothing landed under the other policy"
    );
    // The legal success control: the same candidate, the same bytes, an approval under the policy this surface runs
    // under — it lands. Without this half, a gate that refuses everything would look like it worked.
    // 合法成功控制：同一份候选、同一批字节、一份按本执行面所用政策签发的批准——它落地。少了这一半，一道什么都拒绝的门
    // 会看起来像起作用了。
    approve(&root, &id, &["src".to_owned()]);
    let landed = candidate(&root, &json!({"action":"land","id":id})).unwrap();
    assert!(landed.contains(" landed"), "{landed}");
    std::fs::remove_dir_all(root).unwrap();
}

/// M3: the budget the person approved is a **limit on what may be published**. An approval for one file does not
/// authorise two, and the refusal carries both numbers so the caller knows what to ask a person for.
/// M3：这个人批准的预算是**对可发布内容的限制**。批一个文件的授权不授权两个，而拒绝会带上两个数字，好让调用方知道
/// 该找人批什么。
#[test]
fn an_approval_that_covers_one_file_does_not_authorise_a_two_file_landing() {
    let root = fixture();
    let (id, tree) = opened(&root, json!(["src"]));
    std::fs::write(tree.join("src/lib.rs"), "pub fn answer() -> i64 {\n    42\n}\n\n#[test]\nfn the_answer_is_stable() {\n    assert_eq!(answer(), 42);\n}\n").unwrap();
    std::fs::write(tree.join("src/extra.rs"), "pub const EXTRA: i64 = 1;\n").unwrap();
    candidate(&root, &json!({"action":"verify","id":id,"face":"default"})).unwrap();
    // The host's entry, told to authorise exactly one file, records that limit — and the landing then refuses.
    // 宿主的入口被要求只批一个文件，于是记下那条限制——随后落地被拒。
    assert_eq!(
        confirm(&root, &id, &["--land", "--budget", "files=1"]),
        0,
        "the entry records the limit it was asked for"
    );
    let refused_by_budget = refused(&root, json!({"action":"land","id":id}));
    assert!(
        refused_by_budget.contains("would publish 2 file(s)")
            && refused_by_budget.contains("covers 1"),
        "the refusal carries both numbers: {refused_by_budget}"
    );
    // The read surface answers before the write is tried: `status` says the same thing without running anything.
    // 读面在试写之前就回答：`status` 不跑任何东西也说出同一件事。
    let status = candidate(&root, &json!({"action":"status","id":id})).unwrap();
    assert!(
        status.contains("approval does not cover this candidate")
            && status.contains("would publish 2 file(s)"),
        "status reports the budget shortfall, not just the write refusal: {status}"
    );
    // The legal success control: the entry run without a budget covers exactly the delta it printed.
    // 合法成功控制：不带预算运行入口时，它覆盖的恰好是它印出的那份改动。
    assert_eq!(confirm(&root, &id, &["--land"]), 0);
    let landed = candidate(&root, &json!({"action":"land","id":id})).unwrap();
    assert!(landed.contains(" landed"), "{landed}");
    std::fs::remove_dir_all(root).unwrap();
}

/// M3: a budget the checker cannot read is **not** an unlimited budget. This is the failure mode the old shape had
/// in reverse — `--confirm` used to write the note (or `unknown`) into the budget field, and nothing read it, so the
/// loosest-looking grant was indistinguishable from the strictest.
/// M3：校验方读不懂的预算**不是**无限预算。这正是旧形状反着犯的那个错——`--confirm` 曾把备注（或 `unknown`）写进预算
/// 字段，而没有任何东西读它，于是看起来最松的授权与最严的那份无法区分。
#[test]
fn an_unreadable_budget_is_refused_rather_than_treated_as_unlimited() {
    let root = fixture();
    let (id, tree) = opened(&root, json!(["src"]));
    std::fs::write(tree.join("src/lib.rs"), "pub fn answer() -> i64 {\n    42\n}\n\n#[test]\nfn the_answer_is_stable() {\n    assert_eq!(answer(), 42);\n}\n").unwrap();
    candidate(&root, &json!({"action":"verify","id":id,"face":"default"})).unwrap();
    // The controller refuses to **issue** an unreadable budget, so the only way this shape reaches a landing is a store
    // written by something else — exactly the case this bridge cannot authenticate (the digest is a corruption check,
    // not a signature). The landing must still refuse it rather than read it as unlimited.
    // 控制器会拒绝**签发**读不懂的预算，因此这种形状到达落地的唯一路径是一份由别的什么东西写下的存储——正是本桥无法
    // 鉴别的情形（摘要只是损坏检查，不是签名）。落地仍必须拒绝它，而不是把它读成无限。
    let directory = crate::mcp::controller::directory(&root);
    std::fs::create_dir_all(&directory).unwrap();
    let fields = vec![
        vec!["approver", "whoever wrote this file"],
        vec!["budget", "unknown"],
        vec!["candidate", id.as_str()],
        vec!["channel", "interactive-cli"],
        vec!["operation", "land"],
        vec!["policy", super::POLICY],
        vec!["project", "candidate-probe"],
        vec!["proposal", "1"],
        vec!["revision", "1"],
        vec!["scope", "src"],
        vec!["task", "candidate"],
    ];
    std::fs::write(
        directory.join("grant.json"),
        serde_json::to_vec_pretty(&json!({"fields": fields})).unwrap(),
    )
    .unwrap();
    let refused_by_budget = refused(&root, json!({"action":"land","id":id}));
    assert!(
        refused_by_budget.contains("`unknown` is not a budget limit"),
        "an unreadable budget is refused by name, not skipped: {refused_by_budget}"
    );
    std::fs::remove_dir_all(root).unwrap();
}
