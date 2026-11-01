//! Pins for the durable half of a plan: the writer verdict and what `resume` will not replay.
//! 计划中"持久"那一半的钉子：写者判决，以及 `resume` 不会重放什么。
//!
//! Split from `check_plan_tests.rs` at the size ratchet, along the seam the modules already have:
//! that file pins plan **creation**, this one pins what a later call does with the record.
//! 按尺寸棘轮从 `check_plan_tests.rs` 拆出，沿着模块本来就有的那条缝：那个文件钉计划的**创建**，这个文件钉
//! 之后的调用拿那份记录做了什么。

use crate::mcp::check_plan::check_fixtures::{fixture, plan_action};
use crate::mcp::check_plan::run;
use serde_json::{Value, json};

/// W4/A19: while another live process owns the plan, a second writer is refused by name —
/// two writers interleaving obligations would produce a ledger neither of them can read.
/// W4/A19：另一个活进程拥有这份计划时，第二个写者被点名拒绝——两个写者交错执行义务会产出一份谁都读不懂
/// 的台账。
#[test]
fn a_second_writer_is_refused_while_a_live_process_owns_the_plan() {
    let root = fixture();
    let reply = run(&root, &json!({"files":["src/lib.rs"],"face":"default"})).unwrap();
    let id = reply
        .lines()
        .find_map(|line| line.strip_prefix("plan "))
        .unwrap()
        .to_owned();
    let record = reply
        .lines()
        .find_map(|line| line.strip_prefix("record "))
        .unwrap();
    let journal = std::path::Path::new(record).with_extension("journal.jsonl");
    // pid 1 is init: on every unix it is alive and is not this test process.
    std::fs::write(
        &journal,
        "{\"at\":1,\"writer\":\"someone-else\",\"pid\":1,\"phase\":\"obligation\",\"obligation\":\"regression\",\"state\":\"running\"}\n",
    )
    .unwrap();

    let status = plan_action(&root, &json!({"plan_id": id.clone(), "action": "status"})).unwrap();
    assert!(status.contains("writer busy"), "{status}");
    let refused = plan_action(&root, &json!({"plan_id": id, "action": "resume"}))
        .expect_err("a second writer must be refused");
    assert!(
        refused.contains("someone-else") && refused.contains("already running"),
        "{refused}"
    );

    // The same tail with a writer that is gone is an interrupted plan, and resuming it is the
    // whole point of a durable task.
    std::fs::write(
        &journal,
        "{\"at\":1,\"writer\":\"gone\",\"pid\":999999,\"phase\":\"obligation\",\"obligation\":\"regression\",\"state\":\"running\"}\n",
    )
    .unwrap();
    let interrupted =
        plan_action(&root, &json!({"plan_id": id.clone(), "action": "status"})).unwrap();
    assert!(interrupted.contains("writer interrupted"), "{interrupted}");
    assert!(
        interrupted.contains("pid 999999") && interrupted.contains("is gone"),
        "{interrupted}"
    );
    let resumed = plan_action(&root, &json!({"plan_id": id, "action": "resume"})).unwrap();
    assert!(resumed.starts_with("resumed "), "{resumed}");
    assert!(resumed.contains("regression: passed"), "{resumed}");
    std::fs::remove_dir_all(root).unwrap();
}
/// W4: an obligation that already landed a side effect is never replayed automatically — the
/// plan cannot tell a retry from a first attempt, so it reports the debt instead of guessing.
/// W4：已经落过副作用的义务绝不自动重放——计划分不出"重试"与"第一次"，因此它报出这笔欠账而不是猜。
#[test]
fn a_side_effecting_obligation_is_never_replayed_automatically() {
    let root = fixture();
    let reply = run(&root, &json!({"files":["src/lib.rs"],"face":"default"})).unwrap();
    let id = reply
        .lines()
        .find_map(|line| line.strip_prefix("plan "))
        .unwrap()
        .to_owned();
    let record = reply
        .lines()
        .find_map(|line| line.strip_prefix("record "))
        .unwrap()
        .to_owned();
    let mut ledger: Value =
        serde_json::from_str(&std::fs::read_to_string(&record).unwrap()).unwrap();
    assert_eq!(
        ledger["obligations"][0]["effect"], "read_only",
        "a planned check declares its effect up front: {ledger}"
    );
    ledger["obligations"][0]["effect"] = json!("side_effecting");
    ledger["obligations"][0]["state"] = json!("running");
    std::fs::write(&record, serde_json::to_vec_pretty(&ledger).unwrap()).unwrap();

    let resumed = plan_action(&root, &json!({"plan_id": id, "action": "resume"})).unwrap();
    assert!(resumed.contains("not re-run automatically"), "{resumed}");
    assert!(resumed.contains("0 obligation(s) re-run"), "{resumed}");
    let after: Value = serde_json::from_str(&std::fs::read_to_string(&record).unwrap()).unwrap();
    assert_eq!(
        after["obligations"][0]["state"], "running",
        "the row keeps the state the interrupted effect left behind: {after}"
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn the_write_and_verification_entries_answer_with_bounded_bytes() {
    let root = fixture();
    let verification =
        crate::mcp::check::check(&root, &json!({"files":["src/lib.rs"],"face":"default"}))
            .expect("the verification entry answers");
    // The write face asked **without** `apply`, which is its preview: a question, not a mutation.
    // 写入面**不带** `apply` 地问，也就是它的预览：这是一次提问，不是一次改动。
    let preview = crate::mcp::apply::apply(
        &root,
        &json!({"action":"add","fields":{"module":"probe","kind":"Probe"},"parent":"root"}),
    )
    .expect("the write face previews");
    println!(
        "W6 entry bytes: check={} apply_preview={}",
        verification.len(),
        preview.len()
    );
    assert!(
        !verification.is_empty() && !preview.is_empty(),
        "both answer"
    );
    assert!(
        verification.contains("obligations"),
        "the verification entry answers about obligations: {verification}"
    );
    assert!(
        preview.contains("probe"),
        "and the write face names what it would write: {preview}"
    );
    // The preview must not have written anything — that is what makes its size a *reading* cost.
    // 预览绝不能写过任何东西——这正是它的字节数算作"阅读成本"的前提。
    assert!(
        !root.join("src/probe").exists(),
        "a preview writes nothing, so the bytes above are a reading cost"
    );
    let _ = std::fs::remove_dir_all(root);
}

/// W6 §7a 的第四行：对话面 `explore` 对同一棵树答多少字节，四个入口的**字节**一栏就此齐全。
/// W6 §7a's fourth row: how many bytes the conversation face `explore` answers about the same tree —
/// which completes the **bytes** column for all four entries.
///
/// Rounds and tokens stay blank on purpose (a single in-process call is not a conversation, and this
/// checkout has no tokenizer). What the four numbers together do say is the thing §7a was written to
/// find out: the entries differ by more than an order of magnitude, and the gateway's first look is
/// the smallest of them by design rather than by truncation.
/// 轮次与 token 有意留白（一次进程内调用不是一场对话，本检出也没有分词器）。四个数合起来能说明的，正是 §7a
/// 想查的事：四个入口的字节差了一个数量级以上，而 gateway 的"第一眼"是**设计上**最小、不是被截断得最小。
#[test]
fn the_conversation_face_answers_with_bounded_bytes() {
    let root = fixture();
    let conversation =
        crate::mcp::explore::explore(&root, &json!({"query": "src/lib.rs"})).expect("it answers");
    println!("W6 entry bytes: explore={}", conversation.len());
    assert!(!conversation.is_empty(), "the conversation face answers");
    assert!(
        conversation.contains("lib.rs"),
        "and it answers about what was asked: {conversation}"
    );
    // The same tree through the verification face, so the two numbers in this pin are comparable
    // within one run rather than quoted from another.
    // 同一棵树经验证面再问一次，因此这条钉子里两个数出自同一次运行，而不是引自别处。
    let verification =
        crate::mcp::check::check(&root, &json!({"files":["src/lib.rs"],"face":"default"}))
            .expect("the verification entry answers");
    println!(
        "W6 entry bytes (same run): explore={} check={}",
        conversation.len(),
        verification.len()
    );
    assert!(
        conversation.len() + verification.len() < 71_084,
        "and neither the conversation nor the verification face is anywhere near the full catalogue \\
         ({} + {} vs 71 084)",
        conversation.len(),
        verification.len()
    );
    let _ = std::fs::remove_dir_all(root);
}

/// BR-A count: a re-recorded plan must move its recorded **input count** along with its digest. Measured on a live
/// host: a plan re-recorded against 10 inputs still reported `"inputs": 6`, which misleads the consumer that reads
/// that field to size the closure. The red-control for this pin is removing the one line that refreshes the field
/// (`ledger["inputs"] = json!(refreshed.count())`): the pin then fails with `left: 4`, `right: 5`.
/// BR-A 计数：重记的计划必须把**记录的输入计数**连同摘要一起更新。真宿主上实测：按 10 项输入重记的计划仍报
/// `"inputs": 6`，会误导那个读该字段判断闭包大小的消费者。这条钉子的红检＝删掉刷新该字段的那一行
/// （`ledger["inputs"] = json!(refreshed.count())`）：钉子随即以 `left: 4`、`right: 5` 失败。
#[test]
fn a_resume_refreshes_the_recorded_input_count() {
    let root = fixture();
    let reply = run(&root, &json!({"files":["src/lib.rs"],"face":"default"})).unwrap();
    let id = reply
        .lines()
        .find_map(|line| line.strip_prefix("plan "))
        .unwrap()
        .to_owned();
    std::fs::create_dir_all(root.join("cases")).unwrap();
    std::fs::write(root.join("cases/case.txt"), "one\n").unwrap();
    let resumed = plan_action(&root, &json!({"plan_id": id.clone(), "action": "resume"})).unwrap();
    assert!(resumed.contains("cases/case.txt"), "{resumed}");
    let status = plan_action(&root, &json!({"plan_id": id, "action": "status"})).unwrap();
    let record = status
        .lines()
        .find_map(|line| line.strip_prefix("record "))
        .unwrap()
        .to_owned();
    let ledger: Value = serde_json::from_str(&std::fs::read_to_string(&record).unwrap()).unwrap();
    let expected = crate::mcp::check_task::Inputs::capture(&root, None)
        .unwrap()
        .count();
    assert_eq!(
        ledger["inputs"],
        json!(expected),
        "the count travels with the digest: {ledger}"
    );
    std::fs::remove_dir_all(root).unwrap();
}

/// BR05: two obligations whose execution identities are equal in **every** dimension are settled by
/// one real run, and each is still judged against its own expectation — the run is shared, the duty
/// is not. Without the identity the second obligation would run Cargo again for an identical answer,
/// which is the repeated-verification cost the benchmark measured; with a weaker rule (say, "both are
/// feature-less") two different builds would be merged instead.
/// BR05：执行身份在**每个**维度上都相等的两条义务由一次真实运行结清，而每条仍按自己的预期判定——共用的是
/// 运行，不是义务。没有身份时第二条会为同一个答案再跑一次 Cargo，正是基准量到的重复验证成本；而规则放宽
/// （比如"两边都没有特性"）就会把两次不同的构建合并。
#[test]
fn one_execution_settles_two_obligations_that_bind_every_dimension_identically() {
    let root = fixture();
    let reply = run(
        &root,
        &json!({"files":["src/lib.rs"],"face":"default","cases":[
            {"name":"same execution","expected":"passed","request":{"workspace":true,"face":"default"}}
        ]}),
    )
    .unwrap();
    assert!(reply.starts_with("obligations passed"), "{reply}");
    assert!(reply.contains("executions 1 run, 1 settled"), "{reply}");
    assert!(
        reply.contains("same execution: passed") && reply.contains("settled by `regression`"),
        "{reply}"
    );
    assert!(
        reply.contains("remaining obligations: none"),
        "an execution being shared says nothing about duties being dropped: {reply}"
    );
    std::fs::remove_dir_all(root).unwrap();
}

/// BR05, the other half: change **one** dimension and the merge must not happen. `profile` is the
/// dimension whose absence would be invisible in a file-only closure — `debug` and `release` compile
/// `debug_assert!` and `cfg(debug_assertions)` differently — so it is the one pinned here.
/// BR05 的另一半：改动**一个**维度就不许合并。`profile` 正是"只绑文件的闭包"看不见的那个维度——
/// `debug` 与 `release` 对 `debug_assert!` 与 `cfg(debug_assertions)` 的编译结果不同——因此这里钉它。
#[test]
fn a_different_profile_is_a_different_execution_even_when_every_byte_is_the_same() {
    let root = fixture();
    let reply = run(
        &root,
        &json!({"files":["src/lib.rs"],"face":"default","cases":[
            {"name":"release copy","expected":"passed","request":{"workspace":true,"face":"default","profile":"release"}}
        ]}),
    )
    .unwrap();
    assert!(reply.contains("executions 2 run, 0 settled"), "{reply}");
    assert!(!reply.contains("settled by `regression`"), "{reply}");
    let record = reply
        .lines()
        .find_map(|line| line.strip_prefix("record "))
        .unwrap()
        .to_owned();
    let ledger: Value = serde_json::from_str(&std::fs::read_to_string(&record).unwrap()).unwrap();
    let rows = ledger["obligations"].as_array().unwrap();
    let profile = |row: &Value| {
        row["execution"]
            .as_str()
            .unwrap()
            .split("; ")
            .find_map(|field| field.strip_prefix("profile="))
            .unwrap()
            .to_owned()
    };
    assert_eq!(profile(&rows[0]), "debug", "{ledger}");
    assert_eq!(profile(&rows[1]), "release", "{ledger}");
    assert_ne!(
        rows[0]["execution_digest"], rows[1]["execution_digest"],
        "the two rows must not share an execution digest: {ledger}"
    );
    std::fs::remove_dir_all(root).unwrap();
}

/// BR05/BR06: `reuse` may only reference a plan that recorded **this** closure and **these**
/// execution identities, and it must stop being possible the moment anything changes. The first call
/// executes; the second, asked to reuse, runs nothing and names the plan; a `.txt` fixture that no
/// extension whitelist would have bound changes the closure, and the third call runs again.
/// BR05/BR06：`reuse` 只能引用一份记录了**这个**闭包与**这些**执行身份的计划，而且任何东西一变它就必须
/// 失效。第一次调用执行；第二次要求复用，什么都不跑并点名那份计划；一个任何扩展名白名单都不会绑的 `.txt`
/// 夹具改变了闭包，于是第三次调用重新执行。
#[test]
fn reuse_references_only_a_plan_with_this_closure_and_these_execution_identities() {
    let root = fixture();
    let first = run(&root, &json!({"files":["src/lib.rs"],"face":"default"})).unwrap();
    assert!(first.starts_with("obligations passed"), "{first}");
    let second = run(
        &root,
        &json!({"files":["src/lib.rs"],"face":"default","reuse":true}),
    )
    .unwrap();
    assert!(second.starts_with("obligations passed (reused"), "{second}");
    assert!(second.contains("nothing was run"), "{second}");
    assert!(
        second.contains("same input closure and identical execution identities"),
        "{second}"
    );

    std::fs::create_dir_all(root.join("cases")).unwrap();
    std::fs::write(root.join("cases/case.txt"), "changed after the green\n").unwrap();
    let third = run(
        &root,
        &json!({"files":["src/lib.rs"],"face":"default","reuse":true}),
    )
    .unwrap();
    assert!(!third.contains("(reused"), "{third}");
    assert!(third.starts_with("obligations passed"), "{third}");

    // And a dimension change disqualifies it even though the bytes are identical again.
    // 而字节重新一致之后，仅仅是维度变了也仍然不合格。
    std::fs::remove_file(root.join("cases/case.txt")).unwrap();
    let fourth = run(
        &root,
        &json!({"files":["src/lib.rs"],"face":"default","reuse":true,"profile":"release"}),
    )
    .unwrap();
    assert!(!fourth.contains("(reused"), "{fourth}");
    std::fs::remove_dir_all(root).unwrap();
}

/// BR05: a pass is stale for two different reasons, and `status` has to say which one. The closure
/// comparison alone could not tell "somebody edited a file" from "the same bytes are now being
/// checked under a configuration whose verdict was never produced" — and reporting the second as the
/// first sends the reader looking for an edit that did not happen.
/// BR05：一次通过会因为两种不同原因失效，而 `status` 必须说出是哪一种。只比闭包分不出"有人改了文件"与
/// "同一批字节现在要在另一种从未产出过判定的配置下检查"——而把后者报成前者，会让读者去找一次没发生过的编辑。
#[test]
fn status_separates_a_stale_closure_from_a_stale_execution_configuration() {
    let root = fixture();
    let reply = run(&root, &json!({"files":["src/lib.rs"],"face":"default"})).unwrap();
    let id = reply
        .lines()
        .find_map(|line| line.strip_prefix("plan "))
        .unwrap()
        .to_owned();
    let record = reply
        .lines()
        .find_map(|line| line.strip_prefix("record "))
        .unwrap()
        .to_owned();
    let clean = plan_action(&root, &json!({"plan_id": id.clone(), "action": "status"})).unwrap();
    assert!(
        clean.contains("regression: passed (expected passed) [execution")
            && !clean.contains("stale:"),
        "a fresh pass is not stale: {clean}"
    );

    // Rewrite the recorded execution digest: every byte of the tree is unchanged, so only the second
    // reason can apply.
    // 改写记录下来的执行摘要：树的每个字节都没变，因此只可能是第二种原因。
    let mut ledger: Value =
        serde_json::from_str(&std::fs::read_to_string(&record).unwrap()).unwrap();
    ledger["obligations"][0]["execution_digest"] =
        json!("0000000000000000000000000000000000000000000000000000000000000000");
    std::fs::write(&record, serde_json::to_vec_pretty(&ledger).unwrap()).unwrap();
    let configuration =
        plan_action(&root, &json!({"plan_id": id.clone(), "action": "status"})).unwrap();
    assert!(
        configuration.contains("stale: the execution configuration changed since this passed"),
        "{configuration}"
    );
    assert!(
        !configuration.contains("stale: the inputs changed"),
        "the bytes did not change, and the reply must not say they did: {configuration}"
    );
    // Resuming must therefore re-run it rather than keep it.
    // 因此恢复必须重跑它，而不是留着它。
    let resumed = plan_action(&root, &json!({"plan_id": id, "action": "resume"})).unwrap();
    assert!(resumed.contains("1 obligation(s) re-run"), "{resumed}");
    std::fs::remove_dir_all(root).unwrap();
}

/// BR06: every new parameter is validated before anything runs, and a rejected call leaves no plan
/// behind — "rejected before the side effect" is only true if the side effect can be shown absent.
/// BR06：每个新参数都在任何东西运行之前被校验，而被拒绝的调用不留下计划——只有能证明副作用不在，
/// "在副作用之前被拒绝"才成立。
#[test]
fn new_parameters_are_validated_before_any_plan_or_process_exists() {
    let root = fixture();
    for (arguments, expected) in [
        (
            json!({"files":["src/lib.rs"],"profile":"fast"}),
            "profile must be debug or release",
        ),
        (
            json!({"files":["src/lib.rs"],"reuse":"yes"}),
            "check `reuse` must be a boolean",
        ),
        (
            json!({"files":["src/lib.rs"],"reuse_typo":true}),
            "unsupported check parameter `reuse_typo`",
        ),
        (
            json!({"files":["src/lib.rs"],"cases":[{"name":"x","request":{"reuse":true}}]}),
            "cases cannot ask for cross-plan reuse",
        ),
    ] {
        let error = match run(&root, &arguments) {
            Err(error) => error,
            Ok(reply) => panic!("this call must be refused, and it answered: {arguments}\n{reply}"),
        };
        assert!(
            error.contains(expected),
            "{expected}: {error} -- for {arguments}"
        );
    }
    let directory = root.join("target/xirang/verification");
    assert!(
        !directory.exists() || std::fs::read_dir(&directory).unwrap().next().is_none(),
        "a refused call writes no plan: {:?}",
        std::fs::read_dir(&directory).map(|rows| rows.count())
    );
    std::fs::remove_dir_all(root).unwrap();
}

/// A run that **skipped** a test leaves the anomaly dimension `Known(true)`, so nothing from it is reused — while the
/// same plan still runs (the ruling: unknown is stated, checks still run, caching is not switched off).
/// 一次**跳过**了测试的运行把异常维度置为 `Known(true)`，因此它身上的东西一律不复用——而同一份计划照旧会跑
/// （裁定：未知要说出来，检查照跑，缓存不整体关掉）。
#[test]
fn a_skipped_test_attaches_an_anomaly_and_nothing_from_that_plan_is_reused() {
    let root = crate::mcp::check_plan::check_fixtures::skipped_fixture();
    let plan = crate::mcp::check_plan::run(
        &root,
        &json!({"files":["src/lib.rs","src"],"face":"default"}),
    )
    .unwrap();
    assert!(plan.starts_with("obligations passed"), "{plan}");
    let record = plan
        .lines()
        .find_map(|line| line.strip_prefix("record "))
        .unwrap()
        .to_owned();
    let ledger: Value = serde_json::from_str(&std::fs::read_to_string(&record).unwrap()).unwrap();
    let outcome = &ledger["obligations"][0]["outcome"];
    assert_eq!(
        outcome["skipped"].as_u64(),
        Some(1),
        "the run has to record what it skipped, or the dimension cannot be observed: {ledger}"
    );
    // Reuse is refused — and says why.
    // 复用被拒——并说出原因。
    let reuse = crate::mcp::check_plan::run(
        &root,
        &json!({"files":["src/lib.rs","src"],"face":"default","reuse":true}),
    )
    .unwrap();
    assert!(
        !reuse.contains("(reused"),
        "a plan with an undisposed anomaly is not reused: {reuse}"
    );
    std::fs::remove_dir_all(root).unwrap();
}

/// The dimension a plan never recorded is reported as **unknown with the reason**, and the reply says so — instead of a
/// silent re-run that reads like "there was nothing to reuse".
/// 计划从未记录过的维度被报成**带原因的未知**，而回复会说出来——而不是一次沉默的重跑，让人读成"没有可复用的东西"。
#[test]
fn an_unrecorded_dimension_is_refused_in_the_reply_rather_than_silently_rerun() {
    let root = crate::mcp::check_plan::check_fixtures::fixture();
    let first = crate::mcp::check_plan::run(
        &root,
        &json!({"files":["src/lib.rs","src"],"face":"default"}),
    )
    .unwrap();
    assert!(first.starts_with("obligations passed"), "{first}");
    let record = first
        .lines()
        .find_map(|line| line.strip_prefix("record "))
        .unwrap()
        .to_owned();
    // Simulate a plan written before the dimension was recorded: the observation is gone, the plan is not.
    // 模拟一份在该维度被记录之前写下的计划：观测没了，计划还在。
    let mut ledger: Value =
        serde_json::from_str(&std::fs::read_to_string(&record).unwrap()).unwrap();
    ledger.as_object_mut().unwrap().remove("coverage_digest");
    std::fs::write(&record, serde_json::to_vec_pretty(&ledger).unwrap()).unwrap();
    let second = crate::mcp::check_plan::run(
        &root,
        &json!({"files":["src/lib.rs","src"],"face":"default","reuse":true}),
    )
    .unwrap();
    assert!(
        !second.contains("(reused"),
        "an unrecorded dimension is not reused: {second}"
    );
    assert!(
        second.contains("reuse not taken:")
            && second.contains("unknown")
            && second.contains("did not record"),
        "and the reply says which dimension is unknown and what would record it: {second}"
    );
    std::fs::remove_dir_all(root).unwrap();
}

/// The other half of the ruling's conservative rule: **refusing reuse must not become refusing to check**. The ruling
/// says the unknown is refused for reuse *and* that real execution is still allowed — so the plan that cannot be reused
/// has to run, and the reply has to show it ran.
/// 裁定那条保守规则的**另一半**：**拒绝复用不得变成拒绝检查**。裁定说未知要拒绝复用，同时**仍允许真正执行检查**——
/// 因此那些不能复用的计划必须真的去跑，而回复里要能看出它跑了。
#[test]
fn a_plan_whose_reuse_is_refused_still_executes_the_checks() {
    let root = crate::mcp::check_plan::check_fixtures::skipped_fixture();
    let arguments = json!({"files":["src/lib.rs","src"],"face":"default"});
    let first = crate::mcp::check_plan::run(&root, &arguments).unwrap();
    assert!(first.starts_with("obligations passed"), "{first}");
    let mut reusing = arguments.clone();
    reusing["reuse"] = json!(true);
    let second = crate::mcp::check_plan::run(&root, &reusing).unwrap();
    assert!(
        !second.contains("(reused"),
        "the run skipped a test, so the plan is not reusable: {second}"
    );
    assert!(
        second.contains("; ran]"),
        "and refusing to reuse did not become refusing to check — the obligation really ran: {second}"
    );
    std::fs::remove_dir_all(root).unwrap();
}

/// **EX14: a failure that also exists somewhere else is still a failure.**
/// **EX14：别处也存在的失败仍然是失败。**
///
/// Two obligations can be settled by **one** execution (`satisfied_by` links the second row to the
/// first). When that shared execution failed, the judgement has to be `Failed` for **every** row it
/// settles — the settling is by execution identity, and nothing in the signature consults a baseline,
/// a previous run, or "it was already red". The row that failed on its own and the row that merely
/// points at the failed execution are therefore read the same way; the positive control below shows
/// the same linkage reaching `Passed`, so the assertion is not satisfied by a constant.
/// 两条义务可以由**同一次**执行结清（`satisfied_by` 把第二行指到第一行）。那次共享执行失败时，它结清的
/// **每一行**都必须是 `Failed`——结清看的是执行身份，而这个判定函数的签名里没有任何 baseline、上一轮结果
/// 或"它本来就红"的输入。因此单独失败的那行与仅仅指向失败执行的那行读法相同；下面的正控制用同一个链接
/// 拿到 `Passed`，说明这条断言不是靠一个常量成立的。
#[test]
fn a_failed_execution_settles_every_obligation_it_covers_as_failed() {
    use xirang_kernel::maintenance::VerificationOutcome;

    let failed = vec![
        json!({"name": "regression", "outcome": {"verdict": "failed", "executed": 3, "results": 1}}),
        json!({"name": "behaviour", "satisfied_by": "regression"}),
        json!({"name": "unrelated-history", "outcome": {"verdict": "failed", "executed": 1, "results": 1}}),
    ];
    assert_eq!(
        super::settled_outcome(&failed, &failed[0]),
        VerificationOutcome::Failed
    );
    assert_eq!(
        super::settled_outcome(&failed, &failed[1]),
        VerificationOutcome::Failed,
        "the row settled by a failed execution is failed, not 'unchanged' or 'excused by another failure'"
    );
    assert_eq!(
        super::settled_outcome(&failed, &failed[2]),
        VerificationOutcome::Failed,
        "an unrelated row keeps its own verdict; mixing failures does not merge them into a pass"
    );

    // Positive control: the same `satisfied_by` linkage over a passing execution.
    // 正控制：同一个 `satisfied_by` 链接配一次通过的执行。
    let passed = vec![
        json!({"name": "regression", "outcome": {"verdict": "passed", "executed": 3, "results": 1}}),
        json!({"name": "behaviour", "satisfied_by": "regression"}),
    ];
    assert_eq!(
        super::settled_outcome(&passed, &passed[0]),
        VerificationOutcome::Passed
    );
    assert_eq!(
        super::settled_outcome(&passed, &passed[1]),
        VerificationOutcome::Passed,
        "the linkage follows the execution's real outcome: a passing run passes"
    );
}
