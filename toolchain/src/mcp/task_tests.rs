//! P4 pins: a maintenance task closes only when all five obligations hold — and never on a test pass
//! alone.
//! P4 钉子：维护任务只有在五条义务全部成立时才闭合——绝不只凭一次测试通过。

use serde_json::{Value, json};
use std::path::{Path, PathBuf};

/// A package that compiles with one passing test, plus a policy file.
/// 一个能编译、带一条通过测试的包，外加一个政策文件。
fn package(label: &str) -> PathBuf {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let name = format!("mcp-task-{label}");
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
    // The face lives outside the module tree on purpose: the package compiles without a dependency on
    // the toolchain, while the source scan still derives the face this task is about.
    // 面有意住在模块树之外：包因此不必依赖工具链就能编译，而源码扫描仍能推导出这条任务关于的面。
    std::fs::write(
        root.join("src/lib.rs"),
        "pub fn fixture() -> i32 { 3 }\n#[test]\nfn contract() { assert_eq!(fixture(), 3); }\n",
    )
    .expect("library target");
    std::fs::write(root.join("policy.txt"), "fixture policy version 1\n").expect("policy");
    let output = std::process::Command::new("cargo")
        .args(["generate-lockfile", "--offline"])
        .current_dir(&root)
        .output()
        .expect("cargo runs");
    assert!(output.status.success(), "the fixture lockfile generates");
    root
}

/// A package with one face, a policy lease in force and one accepted rule about that face.
/// 一个带一个面、一份生效政策租约、以及一条关于那个面的已接受规则的包。
fn fixture(label: &str) -> PathBuf {
    let root = package(label);
    let request = json!({"anchor":"policy","certifies":"fixture policy","evidence":"desk check",
        "verifier":"task test","reason":"the fixture policy is the authority",
        "files":["policy.txt"],"apply":true,"confirm":true});
    crate::mcp::adopted::adopted(&root, &request).expect("the lease is in force");
    crate::mcp::apply::apply(
        &root,
        &json!({"action":"add","fields":{"module":"provider","kind":"Provider","provides":"cap.render"},
                "apply":true}),
    )
    .expect("the face is created");
    crate::mcp::rules::rules(
        &root,
        &json!({"action":"propose","rule_id":"keep-render","category":"protocol",
                "selector":"root/provider","requirement":"retain cap.render",
                "origin":"accepted_policy","sources":["policy.txt"],
                "expects":{"provides":"cap.render"}}),
    )
    .expect("the rule is proposed");
    crate::mcp::rules::rules(
        &root,
        &json!({"action":"accept","rule_id":"keep-render","authority":"policy","confirm":true}),
    )
    .expect("the rule is accepted");
    root
}

/// Run a real check over the object's declaration file and return the plan id it printed.
/// 对对象的声明文件跑一次真实检查，并返回它打印的计划 id。
fn plan(root: &Path) -> String {
    let reply = crate::mcp::tools::run_tool(
        root,
        "xirang.check",
        &json!({"files":["src/provider/provider.rs"],"census":false}),
    )
    .expect("the check runs");
    reply
        .lines()
        .find_map(|line| line.strip_prefix("plan "))
        .unwrap_or_else(|| panic!("the check printed no plan id: {reply}"))
        .split_whitespace()
        .next()
        .expect("a plan id follows the word")
        .to_owned()
}

fn task(root: &Path, arguments: Value) -> Result<String, String> {
    super::task(root, &arguments)
}

fn open_task(root: &Path, id: &str, plan_id: &str) -> String {
    task(
        root,
        json!({"action":"open","task":id,"object":"root/provider","verification":plan_id,"apply":true}),
    )
    .expect("the task opens")
}

fn task_status(root: &Path, id: &str) -> String {
    task(root, json!({"action":"status","task":id})).expect("the task answers")
}

fn close_task(root: &Path, id: &str) -> Result<String, String> {
    task(
        root,
        json!({"action":"close","task":id,"confirm":true,"apply":true}),
    )
}

fn request(root: &Path, request: Value) -> String {
    crate::mcp::apply::apply(root, &request).expect("the edit applies")
}

/// **A task with all five obligations satisfied opens, statuses and closes; a preview writes nothing.**
/// **五条义务全部满足的任务能开、能查、能闭合；预览不写任何东西。**
#[test]
fn a_task_closes_when_all_five_obligations_hold() {
    let root = fixture("complete");
    let plan_id = plan(&root);
    let preview = task(
        &root,
        json!({"action":"open","task":"prove","object":"root/provider","verification":plan_id}),
    )
    .expect("the open previews");
    assert!(
        preview.contains("nothing was written") && preview.contains("verification plan"),
        "a preview names what it would record: {preview}"
    );
    assert!(
        !root.join(".xirang/tasks/prove.json").exists(),
        "the preview wrote no record"
    );
    let opened = open_task(&root, "prove", &plan_id);
    assert!(
        opened.contains("task `prove`") && opened.contains("record "),
        "the task records what it rests on: {opened}"
    );
    let status = task_status(&root, "prove");
    assert!(
        status.contains("verdict complete")
            && status.contains("applied      satisfied")
            && status.contains("verified     satisfied")
            && status.contains("rules        satisfied")
            && status.contains("knowledge    satisfied")
            && status.contains("acceptance   satisfied"),
        "all five obligations are satisfied and printed: {status}"
    );
    let closed = close_task(&root, "prove").expect("the task closes");
    assert!(
        closed.contains("task `prove` closed: verdict complete"),
        "the closure names its verdict: {closed}"
    );
    let record: Value = serde_json::from_str(
        &std::fs::read_to_string(root.join(".xirang/tasks/prove.json")).unwrap(),
    )
    .expect("the record is JSON");
    assert_eq!(record["state"], json!("closed"));
    assert_eq!(record["closed"]["verdict"], json!("complete"));
    assert_eq!(
        record["closed"]["obligations"].as_array().map(Vec::len),
        Some(5),
        "the record keeps the five obligations it closed on: {record}"
    );
    let again = close_task(&root, "prove").expect("a closed task answers");
    assert!(
        again.contains("already closed"),
        "a closure is not rewritten: {again}"
    );
    let _ = std::fs::remove_dir_all(root);
}

/// **A task does not close on a test pass alone: a plan that never ran is named.**
/// **任务不会只凭一次测试通过就闭合：从未跑过的计划被点名。**
#[test]
fn a_plan_that_never_ran_keeps_the_task_open() {
    let root = fixture("planned");
    let plan_id = plan(&root);
    // A plan-only call seals the obligations and starts nothing, so its state is not `passed`.
    // 只做计划的那次调用封存义务、不启动任何东西，因此它的状态不是 `passed`。
    let planned = crate::mcp::tools::run_tool(
        &root,
        "xirang.check",
        &json!({"files":["src/lib.rs"],"census":false,"action":"plan"}),
    )
    .expect("the plan seals");
    let planned_id = planned
        .lines()
        .find_map(|line| line.strip_prefix("plan "))
        .unwrap_or_else(|| panic!("no plan id: {planned}"))
        .split_whitespace()
        .next()
        .expect("a plan id follows the word")
        .to_owned();
    let opened = task(
        &root,
        json!({"action":"open","task":"unrun","object":"root/provider","verification":planned_id,"apply":true}),
    )
    .expect("the task opens against a sealed plan");
    assert!(opened.contains("verification plan"), "{opened}");
    let status = task_status(&root, "unrun");
    assert!(
        status.contains("verdict incomplete") && status.contains("verified     outstanding"),
        "a plan with no verdict does not satisfy the verification: {status}"
    );
    let refused = close_task(&root, "unrun").expect_err("the task does not close");
    assert!(
        refused.contains("does not close") && refused.contains("verified"),
        "the refusal names the obligation that failed: {refused}"
    );
    assert!(
        root.join(".xirang/tasks/unrun.json").is_file(),
        "the record is untouched by a refused closure"
    );
    // The unrelated plausible id must not be reused for the second task: keep it addressable.
    let _ = plan_id;
    let _ = std::fs::remove_dir_all(root);
}

/// **A change to any bound input after the plan ran makes the verification undecidable, by path.**
/// **计划跑完之后任何一个绑定输入的改变都会让验证判不了，并且点名路径。**
#[test]
fn a_changed_input_keeps_the_verification_unknown_and_names_it() {
    let root = fixture("changed");
    let plan_id = plan(&root);
    open_task(&root, "changed", &plan_id);
    std::fs::write(
        root.join("src/lib.rs"),
        "pub fn fixture() -> i32 { 4 }\n#[test]\nfn contract() { assert_eq!(fixture(), 4); }\n",
    )
    .expect("an input changes");
    let status = task_status(&root, "changed");
    assert!(
        status.contains("verified     unknown")
            && status.contains("inputs changed after it ran")
            && status.contains("src/lib.rs"),
        "the closure change is undecidable and names the file: {status}"
    );
    assert!(
        status.contains("verdict unknown"),
        "an undecidable obligation is never complete: {status}"
    );
    let refused = close_task(&root, "changed").expect_err("the task does not close");
    assert!(refused.contains("verdict unknown"), "{refused}");
    let _ = std::fs::remove_dir_all(root);
}

/// **Reverting the object after the task opened makes `applied` outstanding.**
/// **任务开启之后把对象改回去，会让 `applied` 变成未满足。**
#[test]
fn a_changed_object_keeps_the_task_open() {
    let root = fixture("objchanged");
    let plan_id = plan(&root);
    open_task(&root, "objchanged", &plan_id);
    // The declaration changes through the real edit path, so the bytes are not the recorded ones.
    // 声明经真实编辑路径改变，因此字节不再是记录下的那些。
    let reply = request(
        &root,
        json!({"action":"edit","node":"root/provider","fields":{"summary_en":"moved on"},"apply":true}),
    );
    assert!(reply.contains("summary_en") || !reply.is_empty(), "{reply}");
    let status = task_status(&root, "objchanged");
    assert!(
        status.contains("verdict incomplete") && status.contains("applied      outstanding"),
        "the bytes no longer match what the task recorded: {status}"
    );
    let _ = std::fs::remove_dir_all(root);
}

/// **A revised rule book after the task opened is undecidable, not silently re-read.**
/// **任务开启之后规则簿被修订，是判不了，而不是被静默重读。**
#[test]
fn a_revised_rule_book_keeps_the_task_open() {
    let root = fixture("rulechanged");
    let plan_id = plan(&root);
    open_task(&root, "rulechanged", &plan_id);
    // A second rule about the same object: the task recorded one, the book now has two.
    // 同对象的第二条规则：任务记录了一条，而规则簿现在有两条。
    crate::mcp::rules::rules(
        &root,
        &json!({"action":"propose","rule_id":"keep-shape","category":"shape",
                "selector":"root/provider","requirement":"retain the kind","origin":"accepted_policy",
                "sources":["policy.txt"]}),
    )
    .expect("the second rule is proposed");
    crate::mcp::rules::rules(
        &root,
        &json!({"action":"accept","rule_id":"keep-shape","authority":"policy","confirm":true}),
    )
    .expect("the second rule is accepted");
    let status = task_status(&root, "rulechanged");
    assert!(
        status.contains("rules        unknown")
            && status.contains("was accepted after this task recorded its rules"),
        "the book moved under the task and it says which rule: {status}"
    );
    let _ = std::fs::remove_dir_all(root);
}

/// **A lapsed lease is outstanding, and `knowledge` says why.**
/// **租约失效是未满足，而 `knowledge` 说明原因。**
#[test]
fn a_lapsed_lease_keeps_the_task_open() {
    let root = fixture("leaselapsed");
    let plan_id = plan(&root);
    open_task(&root, "leaselapsed", &plan_id);
    std::fs::write(root.join("policy.txt"), "fixture policy version 2\n").expect("policy moves");
    let status = task_status(&root, "leaselapsed");
    assert!(
        status.contains("knowledge    outstanding") && status.contains("no longer in force"),
        "the lapsed lease is named as the reason: {status}"
    );
    let _ = std::fs::remove_dir_all(root);
}

/// **Acceptance is required only when the task says so — and then a new confirmation line is the proof.**
/// **只有在任务自己声明时才要求接受——而那时"新增一条确认行"就是证据。**
#[test]
fn acceptance_is_conditional_and_needs_a_confirmation_after_open() {
    let root = fixture("accept");
    let plan_id = plan(&root);
    let opened = task(
        &root,
        json!({"action":"open","task":"accepted","object":"root/provider","verification":plan_id,
                "acceptance":{"required":true,"anchor":"policy"},"apply":true}),
    )
    .expect("the task opens with an acceptance policy");
    assert!(opened.contains("acceptance required"), "{opened}");
    let status = task_status(&root, "accepted");
    assert!(
        status.contains("acceptance   outstanding") && status.contains("has not been confirmed"),
        "a required acceptance is not satisfied by the words of the request: {status}"
    );
    // A fresh confirmation line on new bytes is the lease's own channel for a decision.
    // 新字节上的一条新确认行，就是这份租约自己的决定通道。
    std::fs::write(root.join("policy.txt"), "fixture policy version 2\n").expect("policy moves");
    crate::mcp::adopted::adopted(
        &root,
        &json!({"anchor":"policy","certifies":"fixture policy","evidence":"re-read",
                "verifier":"task test","reason":"the policy was reviewed again",
                "files":["policy.txt"],"apply":true,"confirm":true}),
    )
    .expect("the lease is confirmed again");
    let after = task_status(&root, "accepted");
    assert!(
        after.contains("acceptance   satisfied")
            && after.contains("confirmed again after this task opened"),
        "a confirmation made after the task opened satisfies it: {after}"
    );
    // …but the lease renewal is a knowledge change the task must not silently absorb.
    // ……但租约续签是知识变化，任务不得静默吸收。
    assert!(
        after.contains("knowledge    unknown"),
        "the renewed lease is reported, not absorbed: {after}"
    );
    let _ = std::fs::remove_dir_all(root);
}

/// **The advertised tool answers a real call, and its record is addressable.**
/// **被广告的工具对真实调用作答，而它的记录可寻址。**
#[test]
fn the_advertised_tool_answers_a_real_call() {
    let root = fixture("advertised");
    let reply = crate::mcp::tools::tool_call(
        &root,
        json!(1),
        &json!({"name":"xirang.task","arguments":{"action":"list"}}),
    );
    let text = reply["result"]["content"][0]["text"]
        .as_str()
        .unwrap_or_else(|| panic!("the task tool returned no text: {reply}"));
    let failed = reply["result"]["isError"].as_bool().unwrap_or(false);
    assert!(
        !failed && text.contains("no task record"),
        "the advertised tool answers before any task exists: {text}"
    );
    let _ = std::fs::remove_dir_all(root);
}

/// A plan sealed but never run: the interrupted state `resume` exists to continue.
/// 一份只封存、从未跑过的计划：正是 resume 要延续的中断状态。
fn sealed_plan(root: &Path) -> String {
    let reply = crate::mcp::tools::run_tool(
        root,
        "xirang.check",
        &json!({"files":["src/provider/provider.rs"],"census":false,"action":"plan"}),
    )
    .expect("the plan seals");
    reply
        .lines()
        .find_map(|line| line.strip_prefix("plan "))
        .unwrap_or_else(|| panic!("no plan id: {reply}"))
        .split_whitespace()
        .next()
        .expect("a plan id follows the word")
        .to_owned()
}

/// The plan ledger as it is on disk, for the "nothing ran" assertions.
/// 磁盘上的计划台账，供"什么都没跑"的断言使用。
fn plan_ledger(root: &Path, id: &str) -> Value {
    let path = root
        .join("target/xirang/verification")
        .join(format!("{id}.json"));
    serde_json::from_str(&std::fs::read_to_string(path).expect("the plan record")).expect("JSON")
}

fn retire_rule(root: &Path) {
    crate::mcp::rules::rules(
        root,
        &json!({"action":"retire","rule_id":"keep-render","reason":"the fixture retires it",
                "confirm":true}),
    )
    .expect("the rule retires");
}

/// **An interrupted plan is continued by `resume`, and a resume never closes the task.**
/// **中断的计划由 `resume` 继续，而 resume 从不闭合任务。**
#[test]
fn an_interrupted_plan_is_resumed_and_the_task_can_then_close() {
    let root = fixture("resumable");
    let plan_id = sealed_plan(&root);
    open_task(&root, "recovered", &plan_id);
    let before = task_status(&root, "recovered");
    assert!(
        before.contains("verdict incomplete") && before.contains("verified     outstanding"),
        "a sealed plan is not evidence yet: {before}"
    );
    let journal = root.join(".xirang/tasks/recovered.resumes.jsonl");
    let plan_before = plan_ledger(&root, &plan_id);

    // The dry resume says what it would do and writes nothing at all.
    // 干跑说它会做什么，并且什么都不写。
    let dry = task(&root, json!({"action":"resume","task":"recovered"})).expect("the dry resume");
    assert!(
        dry.contains("would resume the check plan") && dry.contains("nothing was run"),
        "the dry resume names the call it would make: {dry}"
    );
    assert!(!journal.exists(), "a dry resume writes no journal");
    assert_eq!(
        plan_ledger(&root, &plan_id),
        plan_before,
        "a dry resume leaves the plan byte for byte unchanged"
    );

    let resumed = task(
        &root,
        json!({"action":"resume","task":"recovered","apply":true}),
    )
    .expect("the resume runs");
    assert!(
        resumed.contains("resumed the check plan")
            && resumed.contains("verdict incomplete -> complete"),
        "the resume re-ran the plan and the verdict moved: {resumed}"
    );
    assert!(
        resumed.contains("all five are satisfied") && resumed.contains("a resume never closes"),
        "the reply says closure is now available and that resume did not take it: {resumed}"
    );
    let record: Value = serde_json::from_str(
        &std::fs::read_to_string(root.join(".xirang/tasks/recovered.json")).expect("the record"),
    )
    .expect("JSON");
    assert_eq!(
        record["state"],
        json!("open"),
        "a resume never closes: {record}"
    );
    let line = std::fs::read_to_string(&journal).expect("the journal recorded the attempt");
    assert!(
        line.contains("\"resumed\":true") && line.contains("\"verdict_after\":\"complete\""),
        "the attempt is journaled with its outcome: {line}"
    );
    let closed =
        close_task(&root, "recovered").expect("the task closes after its evidence is back");
    assert!(closed.contains("verdict complete"), "{closed}");
    let _ = std::fs::remove_dir_all(root);
}

/// **A changed input is repaired by resuming the plan, not by pretending the old pass still holds.**
/// **变了的输入靠继续计划修好，而不是假装旧通过仍然成立。**
#[test]
fn a_changed_input_is_repaired_by_resuming_the_plan() {
    let root = fixture("resumechanged");
    let plan_id = plan(&root);
    open_task(&root, "changed2", &plan_id);
    std::fs::write(
        root.join("src/lib.rs"),
        "pub fn fixture() -> i32 { 4 }\n#[test]\nfn contract() { assert_eq!(fixture(), 4); }\n",
    )
    .expect("an input changes");
    let status = task_status(&root, "changed2");
    assert!(
        status.contains("verdict unknown") && status.contains("verified     unknown"),
        "the changed closure is undecidable before the resume: {status}"
    );
    let resumed = task(
        &root,
        json!({"action":"resume","task":"changed2","apply":true}),
    )
    .expect("the resume runs");
    assert!(
        resumed.contains("resumed the check plan")
            && resumed.contains("verdict unknown -> complete")
            && resumed.contains("re-run"),
        "the plan was re-run over the changed closure and the verdict moved: {resumed}"
    );
    let closed = close_task(&root, "changed2").expect("the task closes on the re-run evidence");
    assert!(closed.contains("verdict complete"), "{closed}");
    let _ = std::fs::remove_dir_all(root);
}

/// **A closed task is never resumed.**
/// **闭合的任务永远不会被 resume。**
#[test]
fn a_closed_task_is_never_resumed() {
    let root = fixture("resumeclosed");
    let plan_id = plan(&root);
    open_task(&root, "done", &plan_id);
    close_task(&root, "done").expect("the task closes");
    let answer = task(&root, json!({"action":"resume","task":"done","apply":true}))
        .expect("the resume answers");
    assert!(
        answer.contains("never resumed") && answer.contains("Open a new task"),
        "a closed conclusion is answered, not continued: {answer}"
    );
    assert!(
        !root.join(".xirang/tasks/done.resumes.jsonl").exists(),
        "no resume journal is written for a closed task"
    );
    let record: Value = serde_json::from_str(
        &std::fs::read_to_string(root.join(".xirang/tasks/done.json")).expect("the record"),
    )
    .expect("JSON");
    assert_eq!(record["state"], json!("closed"), "{record}");
    let _ = std::fs::remove_dir_all(root);
}

/// **A resume re-runs the check plan; it never decides a rule, a lease or an acceptance.**
/// **resume 只重跑检查计划；它从不替规则、租约或接受做决定。**
#[test]
fn a_resume_does_not_decide_a_rule_change() {
    let root = fixture("resumerule");
    let plan_id = plan(&root);
    open_task(&root, "rulechanged2", &plan_id);
    retire_rule(&root);
    let status = task_status(&root, "rulechanged2");
    assert!(
        status.contains("rules        unknown") && status.contains("is now revision"),
        "the retired rule is the open fact: {status}"
    );
    let plan_before = plan_ledger(&root, &plan_id);
    let resumed = task(
        &root,
        json!({"action":"resume","task":"rulechanged2","apply":true}),
    )
    .expect("the resume answers");
    assert!(
        resumed.contains("the verification is satisfied; there is nothing for a resume to re-run"),
        "nothing about the verification is missing, so nothing is re-run: {resumed}"
    );
    assert!(
        resumed.contains("still open:") && resumed.contains("rules        unknown"),
        "the rule change is named as a decision, not a re-run: {resumed}"
    );
    assert_eq!(
        plan_ledger(&root, &plan_id),
        plan_before,
        "the check plan was not touched"
    );
    let line = std::fs::read_to_string(root.join(".xirang/tasks/rulechanged2.resumes.jsonl"))
        .expect("the attempt is journaled");
    assert!(
        line.contains("\"resumed\":false"),
        "the journal records that nothing was run: {line}"
    );
    let _ = std::fs::remove_dir_all(root);
}

/// **A plan whose headline and rows disagree is not evidence.**
/// **标题与行互相矛盾的计划不是证据。**
#[test]
fn a_plan_that_contradicts_itself_is_not_evidence() {
    let root = fixture("contradiction");
    let plan_id = plan(&root);
    open_task(&root, "contradiction", &plan_id);
    let path = root
        .join("target/xirang/verification")
        .join(format!("{plan_id}.json"));
    let text = std::fs::read_to_string(&path).expect("the plan record");
    let mut plan: Value = serde_json::from_str(&text).expect("JSON");
    assert_eq!(
        plan["state"],
        json!("passed"),
        "the fixture plan passed: {plan}"
    );
    plan["obligations"][0]["state"] = json!("failed");
    std::fs::write(&path, serde_json::to_vec_pretty(&plan).expect("JSON"))
        .expect("the plan is edited");
    let status = task_status(&root, "contradiction");
    assert!(
        status.contains("verified     unknown") && status.contains("disagrees with itself"),
        "the contradiction is named instead of being read as a pass: {status}"
    );
    let _ = std::fs::remove_dir_all(root);
}

/// Open a closed task and inherit from it in one more task, without naming a plan.
/// 开一条已闭合的任务，再开第二条从它继承、不点名计划的任务。
fn closed_source(root: &Path, source: &str) -> String {
    let plan_id = plan(root);
    open_task(root, source, &plan_id);
    close_task(root, source).expect("the source task closes");
    plan_id
}

/// **A second task inherits a closed task's evidence, and the plan is not re-run.**
/// **第二条任务继承已闭合任务的证据，且计划不被重跑。**
#[test]
fn the_second_task_inherits_the_closed_tasks_evidence() {
    let root = fixture("inherit");
    let plan_id = closed_source(&root, "first");
    let plan_before = plan_ledger(&root, &plan_id);
    let opened = task(
        &root,
        json!({"action":"open","task":"second","object":"root/provider","from":"first","apply":true}),
    )
    .expect("the second task opens from the closed one");
    assert!(
        opened.contains("from       task `first`")
            && opened.contains("verification inherited from task `first`")
            && opened.contains("rules inherited (same revisions as the closed task)")
            && opened.contains("leases inherited (same fingerprints as the closed task)"),
        "the reply names what it inherited and why: {opened}"
    );
    let record: Value = serde_json::from_str(
        &std::fs::read_to_string(root.join(".xirang/tasks/second.json")).expect("the record"),
    )
    .expect("JSON");
    assert_eq!(record["inherited_from"], json!("first"), "{record}");
    assert_eq!(record["inherited"]["verification"], json!(true), "{record}");
    assert_eq!(record["inherited"]["rules"], json!(true), "{record}");
    assert_eq!(record["inherited"]["leases"], json!(true), "{record}");
    assert_eq!(
        record["verification"]["plan"],
        json!(plan_id),
        "the inherited plan is the one that ran: {record}"
    );
    assert_eq!(
        plan_ledger(&root, &plan_id),
        plan_before,
        "inheriting evidence does not run the plan again"
    );
    let status = task_status(&root, "second");
    assert!(
        status.contains("inherited from task `first`") && status.contains("verdict complete"),
        "the inherited evidence satisfies the five obligations: {status}"
    );
    close_task(&root, "second").expect("the second task closes on inherited evidence");
    let _ = std::fs::remove_dir_all(root);
}

/// **A task that has not closed is not a source of established knowledge.**
/// **未闭合的任务不是既有知识的来源。**
#[test]
fn an_open_task_is_not_a_source() {
    let root = fixture("inheritopen");
    let plan_id = plan(&root);
    open_task(&root, "unfinished", &plan_id);
    let refused = task(
        &root,
        json!({"action":"open","task":"after","object":"root/provider","from":"unfinished"}),
    )
    .expect_err("the source has not closed");
    assert!(
        refused.contains("has not closed") && refused.contains("established nothing"),
        "the refusal names why an open task cannot be inherited from: {refused}"
    );
    assert!(
        !root.join(".xirang/tasks/after.json").exists(),
        "the refused open wrote nothing"
    );
    let _ = std::fs::remove_dir_all(root);
}

/// **A moved rule book is re-read and named, never inherited.**
/// **移动过的规则簿被重读并点名，绝不被继承。**
#[test]
fn a_moved_rule_book_is_reread_not_inherited() {
    let root = fixture("inheritrule");
    closed_source(&root, "before");
    crate::mcp::rules::rules(
        &root,
        &json!({"action":"propose","rule_id":"keep-shape","category":"shape",
                "selector":"root/provider","requirement":"retain the kind",
                "origin":"accepted_policy","sources":["policy.txt"]}),
    )
    .expect("the second rule is proposed");
    crate::mcp::rules::rules(
        &root,
        &json!({"action":"accept","rule_id":"keep-shape","authority":"policy","confirm":true}),
    )
    .expect("the second rule is accepted");
    let opened = task(
        &root,
        json!({"action":"open","task":"after","object":"root/provider","from":"before","apply":true}),
    )
    .expect("the second task opens against the moved book");
    assert!(
        opened.contains("rules re-read (the rule book moved since the closed task)"),
        "the reply names the move instead of copying the old revisions: {opened}"
    );
    let record: Value = serde_json::from_str(
        &std::fs::read_to_string(root.join(".xirang/tasks/after.json")).expect("the record"),
    )
    .expect("JSON");
    assert_eq!(
        record["rules"].as_object().map(serde_json::Map::len),
        Some(2),
        "the record carries the book as it is now: {record}"
    );
    assert_eq!(record["inherited"]["rules"], json!(false), "{record}");
    let _ = std::fs::remove_dir_all(root);
}

/// **A plan that stopped holding is not inherited.**
/// **不再成立的计划不会被继承。**
#[test]
fn a_stale_plan_is_not_inherited() {
    let root = fixture("inheritstale");
    closed_source(&root, "before");
    std::fs::write(
        root.join("src/lib.rs"),
        "pub fn fixture() -> i32 { 4 }\n#[test]\nfn contract() { assert_eq!(fixture(), 4); }\n",
    )
    .expect("an input changes");
    let refused = task(
        &root,
        json!({"action":"open","task":"after","object":"root/provider","from":"before"}),
    )
    .expect_err("the inherited plan no longer holds");
    assert!(
        refused.contains("no longer holds") && refused.contains("input(s) changed since it ran"),
        "the refusal names the changed closure and asks for a new plan: {refused}"
    );
    assert!(
        !root.join(".xirang/tasks/after.json").exists(),
        "the refused open wrote nothing"
    );
    let _ = std::fs::remove_dir_all(root);
}

/// **A different object cannot inherit another object's rule set.**
/// **另一个对象不能继承别的对象的规则集。**
#[test]
fn another_object_rereads_its_own_rules() {
    let root = fixture("inheritanother");
    closed_source(&root, "before");
    crate::mcp::apply::apply(
        &root,
        &json!({"action":"add","fields":{"module":"provider2","kind":"Provider",
                "provides":"cap.other"},"apply":true}),
    )
    .expect("the second face is created");
    let fresh = plan(&root);
    let opened = task(
        &root,
        json!({"action":"open","task":"other","object":"root/provider2",
                "verification":fresh,"from":"before","apply":true}),
    )
    .expect("the second task opens about another object");
    assert!(
        opened.contains("rules re-read (the closed task is about another object)")
            && opened.contains("leases re-read (the closed task is about another object)")
            && opened.contains("verification named separately"),
        "another object re-reads its own knowledge and does not borrow the plan: {opened}"
    );
    let record: Value = serde_json::from_str(
        &std::fs::read_to_string(root.join(".xirang/tasks/other.json")).expect("the record"),
    )
    .expect("JSON");
    assert_eq!(record["inherited"]["same_object"], json!(false), "{record}");
    assert_eq!(record["inherited"]["rules"], json!(false), "{record}");
    let source: Value = serde_json::from_str(
        &std::fs::read_to_string(root.join(".xirang/tasks/before.json"))
            .expect("the source record"),
    )
    .expect("JSON");
    assert_ne!(
        record["verification"]["plan"], source["verification"]["plan"],
        "the plan is the fresh one, not the one inherited from another object's task: {record}"
    );
    let _ = std::fs::remove_dir_all(root);
}

/// Write the object ledger so the fixture's face carries two source instances, and return the
/// primary source spelling the ledger uses.
/// 写对象台账，使夹具的面携带两个源码实例，并返回台账所用的主源码拼写。
fn ledger_with_instance(root: &Path, second: Option<(&str, &str)>) -> String {
    let namespace = crate::mcp::registry::namespace(root).expect("namespace");
    let views = crate::build_method::face_views(root, &namespace).expect("views");
    let view = views
        .iter()
        .find(|view| view.path == "root/provider")
        .expect("the fixture face");
    let layout = crate::build_method::source_layout(root).expect("layout");
    let observed = view.observed(&layout, &namespace);
    let mut instances = format!("{}@{}", view.id, observed.source);
    if let Some((source, id)) = second {
        instances.push_str(&format!(",{id}@{source}"));
    }
    let line = format!(
        "{}\troot/provider\t{}\t-\t{instances}",
        view.object_id(&namespace),
        observed.source
    );
    std::fs::write(
        root.join("xirang.objects.tsv"),
        format!("# xirang objects v1\n{line}\n"),
    )
    .expect("the object ledger");
    observed.source
}

/// The instance id a second source would carry, derived the way the ledger derives one.
/// 第二个源码会携带的实例 id，按台账的推导方式算出。
fn instance_id(root: &Path, source: &str) -> String {
    let namespace = crate::mcp::registry::namespace(root).expect("namespace");
    xirang_kernel::identity::NodeId::from_namespaced_path(&namespace, source, "Provider")
        .to_string()
}

/// **A second live source is the object too: it is bound at open and a change in it keeps the task open.**
/// **第二个存活源码也是这个对象：开启时被绑定，它变了任务就开不下去。**
#[test]
fn a_second_live_source_is_bound_and_checked() {
    let root = fixture("instances");
    std::fs::write(
        root.join("src/provider/provider_alt.rs"),
        "// the second source\n",
    )
    .expect("the second source");
    let alt = "src/provider/provider_alt.rs";
    let alt_id = instance_id(&root, alt);
    ledger_with_instance(&root, Some((alt, &alt_id)));
    let plan_id = plan(&root);
    let opened = open_task(&root, "instances", &plan_id);
    assert!(
        opened.contains("instances  2 (2 on disk, 0 historical)"),
        "the open names every instance it bound: {opened}"
    );
    let record: Value = serde_json::from_str(
        &std::fs::read_to_string(root.join(".xirang/tasks/instances.json")).expect("the record"),
    )
    .expect("JSON");
    assert_eq!(
        record["object"]["instances"].as_array().map(Vec::len),
        Some(2),
        "the record carries both instances: {record}"
    );
    assert!(
        record["object"]["instances"]
            .as_array()
            .into_iter()
            .flatten()
            .all(|row| row["digest"].is_string()),
        "both files were on disk at open, so both carry a digest: {record}"
    );
    assert!(
        task_status(&root, "instances").contains("verdict complete"),
        "two live instances and a passing plan are complete"
    );
    std::fs::write(root.join(alt), "// the second source, edited\n").expect("the second changes");
    let changed = task_status(&root, "instances");
    assert!(
        changed.contains("applied      outstanding")
            && changed.contains(alt)
            && changed.contains("changed after this task recorded it"),
        "the change in the second source is the object's change and it is named: {changed}"
    );
    std::fs::write(root.join(alt), "// the second source\n").expect("restore");
    assert!(
        task_status(&root, "instances").contains("verdict complete"),
        "restoring the bytes restores the obligation"
    );
    let _ = std::fs::remove_dir_all(root);
}

/// **A historical instance whose file is gone is history, not a reason to keep the task open.**
/// **文件已消失的历史实例是历史，不是让任务开着的原因。**
#[test]
fn a_historical_source_instance_does_not_keep_the_task_open() {
    let root = fixture("historical");
    let gone = "src/provider/provider_gone.rs";
    ledger_with_instance(&root, Some((gone, "00000000000000000000000000000009")));
    let plan_id = plan(&root);
    let opened = open_task(&root, "historical", &plan_id);
    assert!(
        opened.contains("instances  2 (1 on disk, 1 historical)"),
        "the open says which instances are history: {opened}"
    );
    let status = task_status(&root, "historical");
    assert!(
        status.contains("verdict complete"),
        "a historical instance does not block closure: {status}"
    );
    let _ = std::fs::remove_dir_all(root);
}
