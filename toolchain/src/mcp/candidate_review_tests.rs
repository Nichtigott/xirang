//! Pins for the review surface: what a review is bound to, who may record one, and what a required review discharges.
//! 审查面的钉子：一次审查绑定在什么上、谁能记一次审查、以及一个必需审查要结清什么。
//!
//! A failed review must not be landed, a pass recorded against another packet must not count, and a
//! candidate that requires a review must not land without one — and the tool surface must not be able
//! to write a verdict at all, because the developer can call it. Each pin below names the refusal; the
//! happy path is the landing tests' business.
//! 失败的审查不许落地，对着另一份材料包记下的通过不算数，声明了需要审查的候选没有审查就不许落地——而且工具面
//! 根本不许写判定，因为开发者调得到它。下面每条钉子都点名一次拒绝；顺利路径归落地测试管。
//!
//! Helpers come from the sibling test module, which owns the fixture and the candidate driver.
//! 辅助函数来自兄弟测试模块——夹具与候选驱动归它管。

use serde_json::json;
use std::path::PathBuf;

use crate::mcp::candidate::candidate;
use crate::mcp::candidate::candidate_tests::*;

#[test]
fn a_review_is_bound_to_the_packet_it_read() {
    let root = fixture();
    let (id, tree) = opened(&root, json!(["src"]));
    std::fs::write(tree.join("src/lib.rs"), PASSING).unwrap();
    candidate(&root, &json!({"action":"verify","id":id,"face":"default"})).unwrap();
    let before = packet_digest(&root, &id);

    // The copy moves after the packet was printed ⇒ the verdict is refused and names both digests.
    // 材料包打印之后副本动过 ⇒ 判定被拒，并点名两个摘要。
    std::fs::write(tree.join("src/lib.rs"), format!("{PASSING}\n// moved\n")).unwrap();
    let refused = record_against(&root, &id, "pass", "a second session", &before).unwrap_err();
    assert!(
        refused.contains("does not describe the candidate as it is now"),
        "{refused}"
    );
    assert!(
        refused.contains("print the packet again and review that one"),
        "the refusal names the way forward: {refused}"
    );
    std::fs::remove_dir_all(root).unwrap();
}

/// The tool surface has **no** review action: a tool that writes a verdict can write its own pass.
/// 工具面**没有** review 动作：能写判定的工具就能写自己的"通过"。
#[test]
fn recording_a_review_is_not_a_tool_action() {
    let root = fixture();
    let (id, tree) = opened(&root, json!(["src"]));
    std::fs::write(tree.join("src/lib.rs"), PASSING).unwrap();
    candidate(&root, &json!({"action":"verify","id":id,"face":"default"})).unwrap();
    let refused = refused(
        &root,
        json!({"action":"review","id":id,"verdict":"pass","reviewer":"me, honest"}),
    );
    assert!(
        refused.contains("recording a review is not a tool action"),
        "{refused}"
    );
    assert!(
        refused.contains("xirang-mcp --review"),
        "and it names the entry that does record one: {refused}"
    );
    assert!(
        refused.contains("can write its own pass"),
        "and says why the action is gone: {refused}"
    );
    std::fs::remove_dir_all(root).unwrap();
}

/// A failed review is a decision, not a note: if the last step could ignore it, a failed judgement
/// would be delivered by rerunning the last step. It refuses only while it still covers the packet,
/// so the legal continuation — change the copy, verify again, review again — stays open.
/// 失败的评审是决定而不是笔记：如果最后一步能忽略它，重跑最后一步就能把一个失败判定交付出去。它只在仍覆盖
/// 那份材料包时拒绝，因此合法出路——改动副本、重新验证、重新评审——仍然开着。
#[test]
fn a_failed_review_is_not_landed_by_rerunning_the_last_step() {
    let root = fixture();
    let (id, tree) = opened(&root, json!(["src"]));
    std::fs::write(tree.join("src/lib.rs"), PASSING).unwrap();
    candidate(&root, &json!({"action":"verify","id":id,"face":"default"})).unwrap();
    host_review(&root, &id, "fail", "the reviewer").unwrap();
    let refused = refused(&root, json!({"action":"land","id":id}));
    assert!(
        refused.contains("the last review of this candidate was `fail`"),
        "{refused}"
    );
    assert!(
        refused.contains("have it reviewed again"),
        "the refusal names the way forward: {refused}"
    );
    // The legal continuation: the copy moves, the new bytes verify, and the old failed review no
    // longer covers them — a review of moved work is a review of nothing.
    // 合法出路：副本动过、新字节重新验证，旧的失败评审不再覆盖它们——对动过的工作的评审等于没评。
    std::fs::write(tree.join("src/lib.rs"), format!("{PASSING}\n// fixed\n")).unwrap();
    candidate(&root, &json!({"action":"verify","id":id,"face":"default"})).unwrap();
    let landed = candidate(&root, &json!({"action":"land","id":id})).unwrap();
    assert!(landed.contains(" landed"), "{landed}");
    std::fs::remove_dir_all(root).unwrap();
}

/// A verdict needs the packet it read; a note with neither is not a review.
/// 判定需要它所读的那份材料包；两样都没有的笔记不是评审。
#[test]
fn a_verdict_needs_the_packet_it_read() {
    let root = fixture();
    let (id, tree) = opened(&root, json!(["src"]));
    std::fs::write(tree.join("src/lib.rs"), PASSING).unwrap();
    candidate(&root, &json!({"action":"verify","id":id,"face":"default"})).unwrap();
    let bad_verdict = record_against(&root, &id, "looks fine", "someone", "whatever").unwrap_err();
    assert!(bad_verdict.contains("`pass` or `fail`"), "{bad_verdict}");
    let bad_packet = record_against(&root, &id, "pass", "someone", "not a digest").unwrap_err();
    assert!(
        bad_packet.contains("does not describe the candidate as it is now"),
        "{bad_packet}"
    );
    // A name is optional — the maintainer's correction: an independent **context** is the requirement,
    // not a different identity — and the reply says what the record is.
    // 姓名是可选的——维护者纠正：要的是独立**上下文**而不是另一个身份——而回复说清这份记录是什么。
    let unnamed = super::record(
        &records(&root),
        &id,
        "pass",
        None,
        &[],
        &packet_digest(&root, &id),
    )
    .unwrap();
    assert!(
        unnamed.contains("review pass by an unnamed reviewer"),
        "{unnamed}"
    );
    assert!(unnamed.contains("declaration"), "{unnamed}");
    std::fs::remove_dir_all(root).unwrap();
}

/// A review from the host entry is recorded, and every reply says what it is.
/// 宿主入口记下的评审会被记下，而每条回复都说明它是什么。
#[test]
fn the_host_entry_records_a_declaration_against_the_packet() {
    let root = fixture();
    let (id, tree) = opened(&root, json!(["src"]));
    std::fs::write(tree.join("src/lib.rs"), PASSING).unwrap();
    candidate(&root, &json!({"action":"verify","id":id,"face":"default"})).unwrap();
    let reviewed = host_review(&root, &id, "pass", "the reviewer").unwrap();
    assert!(
        reviewed.contains("review pass by the reviewer"),
        "{reviewed}"
    );
    assert!(reviewed.contains("packet "), "{reviewed}");
    assert!(
        reviewed.contains("declaration") && reviewed.contains("belongs to the channel"),
        "a review that read as proof would be the dishonest half: {reviewed}"
    );
    let landed = candidate(&root, &json!({"action":"land","id":id})).unwrap();
    assert!(
        landed.contains(
            "review pass by the reviewer (a declaration, not proof of a separate session)"
        ),
        "the landing reports the review it landed under: {landed}"
    );
    std::fs::remove_dir_all(root).unwrap();
}

/// The review belongs on the **read** surface too: silence must not be readable as approval.
/// 评审也属于**读**面：沉默不许被读成批准。
#[test]
fn status_states_that_no_review_is_on_record_rather_than_saying_nothing() {
    let root = fixture();
    let (id, tree) = opened(&root, json!(["src"]));
    std::fs::write(tree.join("src/lib.rs"), PASSING).unwrap();
    candidate(&root, &json!({"action":"verify","id":id,"face":"default"})).unwrap();
    let before = candidate(&root, &json!({"action":"status","id":id})).unwrap();
    assert!(
        before.contains("review none on record; nothing here says anyone reviewed it"),
        "a reader of `status` must be told nobody reviewed it, not left to infer it: {before}"
    );
    host_review(&root, &id, "pass", "reviewer").unwrap();
    let after = candidate(&root, &json!({"action":"status","id":id})).unwrap();
    assert!(
        after.contains("review pass by reviewer on packet"),
        "the read surface names the review as the declaration it is: {after}"
    );
    assert!(
        !after.contains("none on record"),
        "and stops saying nobody reviewed it: {after}"
    );
    std::fs::remove_dir_all(root).unwrap();
}

/// The overrun is explained, the user decides on **one exact request**, and the work continues — or
/// stays inside the old boundary when the answer is no.
/// 越权被说明，使用方针对**一份确切的请求**作决定，工作得以继续——或者在被拒绝时留在旧边界里。
///
/// Paired acceptance, all in one flow: nothing moves while the request is pending; a decision aimed at
/// another version is refused (an old confirmation never vouches for a newer change); `--reject` keeps
/// the boundary and the landing still refuses; the same facts can be asked again and become pending
/// again; `--accept` applies the increment, clears the verification because the obligations grew, and
/// the landing that follows lands.
/// 成对验收，全在一个流程里：请求待确认时什么都不动；针对别的版本的决定被拒（旧确认从不为更新的改动背书）；
/// `--reject` 保住边界、落地仍被拒；同样的请求可以再问一次并重新变成待确认；`--accept` 应用增量、因为义务变多而
/// 清掉验证，随后的落地真的落得下来。
///
/// The rule changed twice and this pin changed with it both times. Round 42 replaced an adoption-ledger
/// `verifier` (which this same bridge can write) with a controller-issued grant. 2026-10-18 removed the
/// terminal-and-name channel: what the entry records is a decision on a request, not a claim about who
/// is at the keyboard. Then 2026-10-18 (second correction) moved the review itself off the tool surface:
/// the packet keeps the original requirement, and the verdict comes from the host entry against the
/// packet digest. All halves have to be in the same flow — an implementation that refuses everything
/// would pass a test that only checked the refusal.
/// 规则变过两次，这道钉子两次都跟着改。round 42 把采信台账里的 `verifier`（本桥能写）换成控制器签发的授权；
/// 2026-10-18 去掉了终端与姓名通道：入口记下的是关于一份请求的决定，不是关于谁在键盘前的声明。随后 2026-10-18
/// （第二次纠正）把评审本身移出工具面：材料包保留原始需求，判定由宿主入口对照材料包摘要记录。所有半边必须在同一个
/// 流程里——只测拒绝的测试，一个"什么都拒"的实现也能过。
#[test]
fn a_required_review_blocks_the_landing_until_a_pass_covers_these_bytes() {
    let root = fixture();
    let reply = candidate(
        &root,
        &json!({"action":"open","scope":["src"],"authority":"control.object.button","require_review":true,
                "requirement":"make answer() return the value the acceptance test expects",
                "acceptance":["the crate's own test passes"]}),
    )
    .unwrap();
    let id = reply
        .lines()
        .next()
        .and_then(|line| line.strip_prefix("candidate "))
        .and_then(|rest| rest.strip_suffix(" open"))
        .and_then(|rest| rest.split_whitespace().next())
        .unwrap()
        .to_owned();
    let tree = PathBuf::from(
        reply
            .lines()
            .find_map(|line| line.strip_prefix("tree "))
            .unwrap(),
    );
    approve(&root, &id, &["src".to_owned()]);
    std::fs::write(tree.join("src/lib.rs"), PASSING).unwrap();
    candidate(&root, &json!({"action":"verify","id":id,"face":"default"})).unwrap();

    let refused = refused(&root, json!({"action":"land","id":id}));
    assert!(
        refused.contains("require_review: true") && refused.contains("no review has been recorded"),
        "an unmet required review is a refusal that names the un-met obligation: {refused}"
    );

    let status = candidate(&root, &json!({"action":"status","id":id})).unwrap();
    assert!(status.contains("review packet "), "{status}");
    assert!(
        status.contains("requirement (as the person stated it):")
            && status.contains("make answer() return the value the acceptance test expects"),
        "the packet keeps the request as the person stated it, not the author's account: {status}"
    );
    assert!(
        status.contains("acceptance criteria:") && status.contains("the crate's own test passes"),
        "and the criteria the review is judged against: {status}"
    );
    assert!(
        status.contains("changed src/lib.rs") && status.contains("test(s) executed"),
        "and the delta and the evidence it binds to: {status}"
    );
    assert!(
        status.contains("--review --root") && status.contains("--verdict pass|fail"),
        "and the host entry that records the verdict, so the reviewer is not asked to invent it: {status}"
    );

    host_review(&root, &id, "pass", "the reviewer").unwrap();
    let landed = candidate(&root, &json!({"action":"land","id":id})).unwrap();
    assert!(landed.contains(" landed"), "{landed}");
    assert!(
        landed.contains("review pass by the reviewer"),
        "and the landing says who reviewed it: {landed}"
    );
    std::fs::remove_dir_all(root).unwrap();
}

/// A required review without the request it is judged against is refused at **open**, before any work
/// exists: a reviewer given only the author's account reviews the author.
/// 必需评审若没有它所评判的需求，就在**开候选**时被拒——在任何工作存在之前：只拿到作者说法的评审者，评的是作者。
#[test]
fn a_required_review_needs_the_request_it_is_judged_against() {
    let root = fixture();
    let refused = refused(
        &root,
        json!({"action":"open","scope":["src"],"require_review":true}),
    );
    assert!(refused.contains("needs `requirement`"), "{refused}");
    assert!(
        refused.contains("author's account"),
        "and says why the request is not optional: {refused}"
    );
    std::fs::remove_dir_all(root).unwrap();
}

/// The packet keeps the request even when the work passes its own tests: a change can be tested and
/// still be the wrong change, and the packet is the material that lets a reviewer say so.
/// 哪怕工作通过了自己的测试，材料包也保留原始需求：一次改动可以测试全绿却改错了，而材料包正是让评审者说出这一点的
/// 材料。
#[test]
fn the_packet_carries_the_request_even_when_the_work_passes_its_own_tests() {
    let root = fixture();
    let reply = candidate(
        &root,
        &json!({"action":"open","scope":["src"],"authority":"control.object.button","require_review":true,
                "requirement":"answer() must return 42 for the caller in src/consumer.rs",
                "acceptance":["the existing test suite passes","the caller gets 42"]}),
    )
    .unwrap();
    let id = reply
        .lines()
        .next()
        .and_then(|line| line.strip_prefix("candidate "))
        .and_then(|rest| rest.strip_suffix(" open"))
        .and_then(|rest| rest.split_whitespace().next())
        .unwrap()
        .to_owned();
    let tree = PathBuf::from(
        reply
            .lines()
            .find_map(|line| line.strip_prefix("tree "))
            .unwrap(),
    );
    // The author's implementation: it changes the test to match the code it wrote, so everything is
    // green — and the work is still the wrong work. The packet has to keep that visible.
    // 作者的做法：把测试改成符合自己写的代码，于是一切全绿——而这次工作仍然是错的工作。材料包必须让它可见。
    std::fs::write(tree.join("src/lib.rs"), PASSING).unwrap();
    candidate(&root, &json!({"action":"verify","id":id,"face":"default"})).unwrap();
    let status = candidate(&root, &json!({"action":"status","id":id})).unwrap();
    assert!(status.contains("verification passed"), "{status}");
    assert!(
        status.contains("answer() must return 42 for the caller in src/consumer.rs"),
        "a passing build must not hide the request the work was asked to serve: {status}"
    );
    assert!(
        status.contains("the caller gets 42") && status.contains("the existing test suite passes"),
        "and the criteria stay next to it: {status}"
    );
    std::fs::remove_dir_all(root).unwrap();
}

/// The obligation follows the **packet**: a pass recorded against an earlier version does not discharge
/// a review of the version that would land. This is the "modify after review, review again" half of §7.
/// 义务跟着**材料包**走：对着早先版本记下的通过，不能履行对将要落地那个版本的评审。这就是 §7 里"评审后改动、
/// 改动后重新评审"的那一半。
#[test]
fn a_required_review_of_other_bytes_does_not_discharge_the_obligation() {
    let root = fixture();
    let reply = candidate(
        &root,
        &json!({"action":"open","scope":["src"],"authority":"control.object.button","require_review":true,
                "requirement":"make the change the request describes"}),
    )
    .unwrap();
    let id = reply
        .lines()
        .next()
        .and_then(|line| line.strip_prefix("candidate "))
        .and_then(|rest| rest.strip_suffix(" open"))
        .and_then(|rest| rest.split_whitespace().next())
        .unwrap()
        .to_owned();
    let tree = PathBuf::from(
        reply
            .lines()
            .find_map(|line| line.strip_prefix("tree "))
            .unwrap(),
    );
    approve(&root, &id, &["src".to_owned()]);
    std::fs::write(tree.join("src/lib.rs"), PASSING).unwrap();
    candidate(&root, &json!({"action":"verify","id":id,"face":"default"})).unwrap();
    host_review(&root, &id, "pass", "the reviewer").unwrap();
    // The copy moves and is verified again: the pass above is now about work nobody would land.
    // 副本动过并重新验证：上面那次通过对现在来说是关于没人会落地的工作。
    std::fs::write(tree.join("src/lib.rs"), format!("{PASSING}\n// moved\n")).unwrap();
    candidate(&root, &json!({"action":"verify","id":id,"face":"default"})).unwrap();
    let refused = refused(&root, json!({"action":"land","id":id}));
    assert!(
        refused.contains("require_review: true")
            && refused.contains("on another packet, so it no longer covers these bytes"),
        "a stale pass is not a pass: {refused}"
    );
    std::fs::remove_dir_all(root).unwrap();
}

/// The advisory case stays advisory: without the flag a candidate lands with `none on record` printed. That is the
/// other half of §7 — the requirement is recorded at open, so a missing flag is a decision the opener made, not an
/// omission this surface fills in later.
/// 咨询情形照旧是咨询：没有那个标志时候选照常落地，并印出 `none on record`。这是 §7 的另一半——要求在开候选时记下，
/// 因此缺少标志是开启者做过的决定，而不是本执行面以后会补上的遗漏。
#[test]
fn a_candidate_without_a_required_review_lands_and_says_no_review_is_on_record() {
    let root = fixture();
    let (id, tree) = opened(&root, json!(["src"]));
    std::fs::write(tree.join("src/lib.rs"), PASSING).unwrap();
    candidate(&root, &json!({"action":"verify","id":id,"face":"default"})).unwrap();
    let landed = candidate(&root, &json!({"action":"land","id":id})).unwrap();
    assert!(
        landed
            .contains("none on record; this landing says nothing about whether anyone reviewed it"),
        "the landing states the absence instead of implying a review: {landed}"
    );
    std::fs::remove_dir_all(root).unwrap();
}
