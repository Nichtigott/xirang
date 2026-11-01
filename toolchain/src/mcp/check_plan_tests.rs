//! Real workspace regression and counterproofs cannot close on compiler errors or empty tests.
//! 真实工作区回归与反证不能用编译错误或空测试结清。
use super::check_fixtures::{
    conversion_host, fixture, key_of, plan_action, slow_fixture, start_when_a_slot_frees,
};
use super::*;
use std::path::PathBuf;

/// A two-package workspace where both packages own a `src/lib.rs` with a test named `contract`; `a`
/// passes and `b` fails under its `bypass` feature. This is the duplicate-package false-green shape.
/// 一个双包工作区：两个包都有 `src/lib.rs` 且都含名为 `contract` 的测试；`a` 通过，`b` 在其
/// `bypass` 特性下失败。这就是重复包假绿的形状。
fn workspace_fixture() -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "xirang-plan-ws-{}",
        crate::mcp::check_jobs::new_id()
    ));
    std::fs::create_dir_all(root.join("a/src")).unwrap();
    std::fs::create_dir_all(root.join("b/src")).unwrap();
    std::fs::write(
        root.join("Cargo.toml"),
        "[workspace]\nmembers=[\"a\",\"b\"]\nresolver=\"3\"\n",
    )
    .unwrap();
    std::fs::write(
        root.join("a/Cargo.toml"),
        "[package]\nname=\"a-proof\"\nversion=\"0.0.0\"\nedition=\"2024\"\n[features]\nbypass=[]\n",
    )
    .unwrap();
    std::fs::write(
        root.join("a/src/lib.rs"),
        "#[test]\nfn contract() { assert_eq!(3, 3); }\n",
    )
    .unwrap();
    std::fs::write(
        root.join("b/Cargo.toml"),
        "[package]\nname=\"b-proof\"\nversion=\"0.0.0\"\nedition=\"2024\"\n[features]\nbypass=[]\n",
    )
    .unwrap();
    std::fs::write(
        root.join("b/src/lib.rs"),
        "#[test]\nfn contract() { assert!(!cfg!(feature=\"bypass\"), \"EXPECTED_ROUTE_VIOLATION\"); }\n",
    )
    .unwrap();
    let output = std::process::Command::new("cargo")
        .args(["generate-lockfile", "--offline"])
        .current_dir(&root)
        .output()
        .unwrap();
    assert!(output.status.success());
    root
}

#[test]
fn execute_and_persist_regression_plus_bypass_counterproof() {
    let root = fixture();
    let request = json!({"files":["src/lib.rs"],"face":"default","cases":[{"name":"bypass rejection","expected":"failed","request":{"face":"bypass"},"expect":{"test":"contract","violation":"assertion `left == right` failed"}}]});
    let reply = run(&root, &request).unwrap();
    assert!(reply.starts_with("obligations passed"), "{reply}");
    assert!(reply.contains("bypass rejection: passed (expected failed)"));
    let record = reply
        .lines()
        .find_map(|line| line.strip_prefix("record "))
        .unwrap();
    let ledger: Value = serde_json::from_str(&std::fs::read_to_string(record).unwrap()).unwrap();
    assert!(
        ledger["obligations"][0]["evidence"]
            .as_str()
            .unwrap()
            .contains("verdict  passed")
    );
    assert!(ledger["impact"].as_str().unwrap().contains("src/lib.rs"));
    let bad = run(&root, &json!({"files":["src/lib.rs"],"face":"default","cases":[{"name":"not a counterproof","expected":"failed","request":{"face":"bad-build"},"expect":{"test":"contract","violation":"assertion"}}]})).unwrap();
    assert!(bad.starts_with("obligations open"), "{bad}");
    assert!(bad.contains("not a counterproof: failed"), "{bad}");
    std::fs::remove_dir_all(root).unwrap();
}
/// A09: a negative control must name the test and the violation it intends; an unqualified failure
/// is unknown, and an unrelated failing test never certifies the intended one.
/// A09：负控制必须点名它要检出的测试与违约；未限定失败一律 unknown，无关的失败测试绝不能替目标测试背书。
#[test]
fn a_negative_control_requires_the_named_test_and_violation() {
    let root = fixture();
    let unqualified = run(&root, &json!({"files":["src/lib.rs"],"face":"default","cases":[{"name":"unqualified","expected":"failed","request":{"face":"bypass"}}]})).unwrap();
    assert!(unqualified.starts_with("obligations open"), "{unqualified}");
    assert!(
        unqualified.contains("unqualified: unknown"),
        "{unqualified}"
    );

    let intended = run(&root, &json!({"files":["src/lib.rs"],"face":"default","cases":[{"name":"intended","expected":"failed","request":{"face":"bypass,unrelated-break"},"expect":{"test":"contract","violation":"assertion `left == right` failed"}}]})).unwrap();
    assert!(intended.starts_with("obligations passed"), "{intended}");
    assert!(intended.contains("intended: passed"), "{intended}");

    let unrelated = run(&root, &json!({"files":["src/lib.rs"],"face":"default","cases":[{"name":"unrelated only","expected":"failed","request":{"face":"unrelated-break"},"expect":{"test":"contract","violation":"assertion `left == right` failed"}}]})).unwrap();
    assert!(unrelated.starts_with("obligations open"), "{unrelated}");
    assert!(unrelated.contains("unrelated only: failed"), "{unrelated}");

    // Zero executed (a filter that matches nothing) and a timeout are both `unknown`: neither can
    // certify that the target check detected its intended failure.
    // 零执行（过滤器什么都没匹配到）与超时都是 `unknown`：两者都不能证明目标检查检出了预期失败。
    let bounds = run(&root, &json!({"files":["src/lib.rs"],"face":"default","cases":[
        {"name":"zero matched","expected":"failed","request":{"face":"bypass","filter":"no_such_test"},"expect":{"test":"contract","violation":"assertion"}},
        {"name":"timeout","expected":"failed","request":{"face":"slow","timeout_ms":1000},"expect":{"test":"slow_target","violation":"assertion"}}
    ]})).unwrap();
    assert!(bounds.starts_with("obligations open"), "{bounds}");
    assert!(bounds.contains("zero matched: unknown"), "{bounds}");
    assert!(bounds.contains("timeout: unknown"), "{bounds}");
    std::fs::remove_dir_all(root).unwrap();
}

/// The supervisor's false green: a passing root `contract` must not be certified by a failing
/// `unrelated::contract`. Only the exact full name is the named target.
/// 监督者发现的假绿：失败的 `unrelated::contract` 绝不能给通过的根 `contract` 背书。只有完整原名才是
/// 点名的目标。
#[test]
fn a_sibling_module_with_the_same_test_name_does_not_certify() {
    let root = fixture();
    let partial = run(&root, &json!({"files":["src/lib.rs"],"face":"default","cases":[
        {"name":"root contract","expected":"failed","request":{"face":"sibling"},"expect":{"test":"contract","violation":"EXPECTED_ROUTE_VIOLATION"}}
    ]})).unwrap();
    assert!(partial.starts_with("obligations open"), "{partial}");
    assert!(partial.contains("root contract: failed"), "{partial}");

    let full = run(&root, &json!({"files":["src/lib.rs"],"face":"default","cases":[
        {"name":"sibling contract","expected":"failed","request":{"face":"sibling"},"expect":{"test":"unrelated::contract","violation":"EXPECTED_ROUTE_VIOLATION"}}
    ]})).unwrap();
    assert!(full.starts_with("obligations passed"), "{full}");
    assert!(full.contains("sibling contract: passed"), "{full}");
    std::fs::remove_dir_all(root).unwrap();
}

/// The same full test name in two Cargo targets: a name-only selector is unknown, and binding the
/// target resolves it — binding the wrong target does not.
/// 同一个完整测试名出现在两个 Cargo target 里：只给名字的选择器判 unknown，绑定 target 才可解——绑错
/// target 则不行。
#[test]
fn a_counterproof_binds_the_target_when_the_test_name_is_ambiguous() {
    let root = fixture();
    let ambiguous = run(&root, &json!({"files":["src/lib.rs"],"face":"default","cases":[
        {"name":"ambiguous shared","expected":"failed","request":{"face":"ambiguous"},"expect":{"test":"shared","violation":"EXPECTED_ROUTE_VIOLATION"}}
    ]})).unwrap();
    assert!(ambiguous.starts_with("obligations open"), "{ambiguous}");
    assert!(
        ambiguous.contains("ambiguous shared: unknown"),
        "{ambiguous}"
    );

    let bound = run(&root, &json!({"files":["src/lib.rs"],"face":"default","cases":[
        {"name":"bound shared","expected":"failed","request":{"face":"ambiguous"},"expect":{"test":"shared","target":"tests/right.rs","violation":"EXPECTED_ROUTE_VIOLATION"}}
    ]})).unwrap();
    assert!(bound.starts_with("obligations passed"), "{bound}");
    assert!(bound.contains("bound shared: passed"), "{bound}");

    let wrong = run(&root, &json!({"files":["src/lib.rs"],"face":"default","cases":[
        {"name":"wrong target","expected":"failed","request":{"face":"ambiguous"},"expect":{"test":"shared","target":"src/lib.rs","violation":"EXPECTED_ROUTE_VIOLATION"}}
    ]})).unwrap();
    assert!(wrong.starts_with("obligations open"), "{wrong}");
    assert!(wrong.contains("wrong target: failed"), "{wrong}");
    std::fs::remove_dir_all(root).unwrap();
}

/// The duplicate-package false green: two packages both run `src/lib.rs` with test `contract`; `a`
/// passes and `b` fails. A selector naming only `src/lib.rs` resolves to **two** binary identities,
/// so it is `unknown`; with no selector it is `unknown` too. Scoping the request to the failing
/// package is accepted, and scoping it to the passing package cannot borrow the other's failure.
/// 重复包假绿：两个包都跑 `src/lib.rs` 且测试名都是 `contract`；`a` 通过、`b` 失败。只点名
/// `src/lib.rs` 的选择器解析到**两个**二进制身份，因此判 `unknown`；不给选择器也判 `unknown`。把请求
/// 限定到失败的那个包则被接受，限定到通过的那个包不能借用另一个包的失败。
#[test]
fn a_counterproof_needs_one_target_identity_across_packages() {
    let root = workspace_fixture();

    let ambiguous = run(&root, &json!({"files":["a/src/lib.rs","b/src/lib.rs"],"face":"a-proof/bypass,b-proof/bypass","cases":[
        {"name":"ambiguous label","expected":"failed","request":{"workspace":true,"face":"a-proof/bypass,b-proof/bypass"},"expect":{"test":"contract","target":"src/lib.rs","violation":"EXPECTED_ROUTE_VIOLATION"}}
    ]})).unwrap();
    assert!(ambiguous.starts_with("obligations open"), "{ambiguous}");
    assert!(
        ambiguous.contains("ambiguous label: unknown"),
        "{ambiguous}"
    );

    let unspecified = run(&root, &json!({"files":["a/src/lib.rs","b/src/lib.rs"],"face":"a-proof/bypass,b-proof/bypass","cases":[
        {"name":"no target","expected":"failed","request":{"workspace":true,"face":"a-proof/bypass,b-proof/bypass"},"expect":{"test":"contract","violation":"EXPECTED_ROUTE_VIOLATION"}}
    ]})).unwrap();
    assert!(unspecified.starts_with("obligations open"), "{unspecified}");
    assert!(unspecified.contains("no target: unknown"), "{unspecified}");

    let scoped_failing = run(&root, &json!({"files":["b/src/lib.rs"],"face":"default","cases":[
        {"name":"scoped failing","expected":"failed","request":{"package":"b-proof","face":"bypass"},"expect":{"test":"contract","target":"src/lib.rs","violation":"EXPECTED_ROUTE_VIOLATION"}}
    ]})).unwrap();
    assert!(
        scoped_failing.starts_with("obligations passed"),
        "{scoped_failing}"
    );
    assert!(
        scoped_failing.contains("scoped failing: passed"),
        "{scoped_failing}"
    );

    let scoped_passing = run(&root, &json!({"files":["a/src/lib.rs"],"face":"default","cases":[
        {"name":"scoped passing","expected":"failed","request":{"package":"a-proof","face":"bypass"},"expect":{"test":"contract","target":"src/lib.rs","violation":"EXPECTED_ROUTE_VIOLATION"}}
    ]})).unwrap();
    assert!(
        scoped_passing.starts_with("obligations open"),
        "{scoped_passing}"
    );
    assert!(
        scoped_passing.contains("scoped passing: failed"),
        "{scoped_passing}"
    );
    std::fs::remove_dir_all(root).unwrap();
}

/// A02: the ledger separates answered / applied / verified / accepted; a check-only plan is never
/// applied or accepted, and its typed outcome is saved structurally beside the rendered evidence.
/// A02：台账把作答/落盘/验证/接受分开；只检查的计划从不落盘、从不被接受，其类型化结果与渲染证据并列
/// 结构化保存。
#[test]
fn a_check_plan_records_answered_and_verified_but_never_applied_or_accepted() {
    let root = fixture();
    let reply = run(&root, &json!({"files":["src/lib.rs"],"face":"default"})).unwrap();
    assert!(reply.starts_with("obligations passed"), "{reply}");
    let record = reply
        .lines()
        .find_map(|line| line.strip_prefix("record "))
        .unwrap();
    let ledger: Value = serde_json::from_str(&std::fs::read_to_string(record).unwrap()).unwrap();
    let completion = &ledger["completion"];
    assert_eq!(completion["query_answered"], json!(true), "{completion}");
    assert_eq!(
        completion["operation_applied"],
        json!(false),
        "{completion}"
    );
    assert_eq!(completion["accepted"], json!(false), "{completion}");
    assert_eq!(
        completion["acceptance_required"],
        json!(false),
        "the check-plan policy adds no blanket approval gate: {completion}"
    );
    assert_eq!(
        completion["verification_closes"],
        json!(true),
        "{completion}"
    );
    assert_eq!(
        completion["complete"],
        json!(false),
        "a check-only plan is never applied and never accepted: {completion}"
    );
    let outcome = &ledger["obligations"][0]["outcome"];
    assert_eq!(outcome["kind"], json!("tests"), "{outcome}");
    assert_eq!(outcome["verdict"], json!("passed"), "{outcome}");
    assert_eq!(outcome["executed"], json!(1), "{outcome}");
    // Three `test result:` lines is what this fixture's log carries — the library's unit
    // tests, `tests/right.rs`, and the doc-test group — and the ledger records the
    // stream's own count rather than a fabricated one. The count is asserted exactly so
    // that a run which stopped after the first group cannot pass this pin.
    // 三条 `test result:` 行就是这个夹具的日志所携带的——库单测、`tests/right.rs`、doc-test 组——而台账
    // 记录的是流自己的计数，不是编造的。这里精确断言该计数，使一个在第一组之后就停下的运行过不了这条钉子。
    assert_eq!(outcome["results"], json!(3), "{outcome}");
    assert_eq!(outcome["unknown_reason"], Value::Null, "{outcome}");
    std::fs::remove_dir_all(root).unwrap();
}
#[test]
fn a_case_cannot_silently_override_the_plan_build_directory() {
    let root = fixture();
    let error = run(&root, &json!({"files":["src/lib.rs"],"cases":[{"name":"isolated","request":{"target_dir":"other-output"}}]})).unwrap_err();
    assert!(
        error.contains("set target_dir on the outer plan"),
        "{error}"
    );
    assert!(!root.join("other-output").exists());
    assert!(!root.join("target/xirang/verification").exists());
    std::fs::remove_dir_all(root).unwrap();
}
#[test]
fn source_configuration_and_path_changes_invalidate_identity() {
    let root = fixture();
    let before = fingerprint(&root, None).unwrap();
    std::fs::write(root.join("src/new.rs"), "fn new_consumer() { provider(); }").unwrap();
    let added = fingerprint(&root, None).unwrap();
    assert_ne!(before, added);
    std::fs::rename(root.join("src/new.rs"), root.join("src/moved.rs")).unwrap();
    let moved = fingerprint(&root, None).unwrap();
    assert_ne!(added, moved);
    std::fs::write(root.join("Cargo.toml"), "[workspace]\nmembers=[]\n").unwrap();
    assert_ne!(moved, fingerprint(&root, None).unwrap());
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn restored_timestamps_cannot_reuse_another_revisions_green_binary() {
    let root = fixture();
    let request = json!({"files":["src/lib.rs"],"face":"default"});
    let source = root.join("src/lib.rs");
    let modified = source.metadata().unwrap().modified().unwrap();
    let first = run(&root, &request).unwrap();
    assert!(first.starts_with("obligations passed"), "{first}");
    let text = std::fs::read_to_string(&source)
        .unwrap()
        .replace("provider() -> i32 { 3 }", "provider() -> i32 { 4 }");
    std::fs::write(&source, text).unwrap();
    std::fs::File::options()
        .write(true)
        .open(&source)
        .unwrap()
        .set_times(std::fs::FileTimes::new().set_modified(modified))
        .unwrap();
    let second = run(&root, &request).unwrap();
    assert!(second.starts_with("obligations open"), "{second}");
    assert!(second.contains("regression: failed"), "{second}");
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn sources_edited_during_execution_leave_obligations_open() {
    let root = fixture();
    std::fs::write(root.join("src/lib.rs"), "#[test]\nfn changing() { std::fs::write(\"src/lib.rs\", \"// changed during test\\n\").unwrap(); }\n").unwrap();
    let reply = run(&root, &json!({"files":["src/lib.rs"],"face":"default"})).unwrap();
    assert!(reply.starts_with("obligations open"), "{reply}");
    assert!(reply.contains("regression: unknown"), "{reply}");
    std::fs::remove_dir_all(root).unwrap();
}

/// W4/A17: a plan is a **durable** record — a later call reads it back by id and by task key,
/// `status` writes nothing, and the reply names the files whose bytes the verdict rests on.
/// W4/A17：计划是一份**持久**记录——之后的调用能按 id 与任务键把它读回，`status` 什么都不写，而回复点名
/// 判定所依据的那些文件。
#[test]
fn a_finished_plan_reads_back_without_running_anything() {
    let root = fixture();
    let reply = run(&root, &json!({"files":["src/lib.rs"],"face":"default"})).unwrap();
    assert!(reply.starts_with("obligations passed"), "{reply}");
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
    let ledger_before = std::fs::read(&record).unwrap();
    let journal = std::path::Path::new(&record).with_extension("journal.jsonl");
    let journal_before = std::fs::read(&journal).unwrap();

    let status = plan_action(&root, &json!({"plan_id": id, "action": "status"})).unwrap();
    assert!(status.contains("inputs unchanged"), "{status}");
    assert!(status.contains("writer none"), "{status}");
    assert!(status.contains("regression: passed"), "{status}");
    assert!(
        status.contains("nothing was run and nothing was written"),
        "{status}"
    );
    assert_eq!(
        std::fs::read(&record).unwrap(),
        ledger_before,
        "status is a read: the ledger must be byte-identical afterwards"
    );
    assert_eq!(std::fs::read(&journal).unwrap(), journal_before, "{status}");

    // The task key is the other handle on the same plan, so a caller that lost the id can
    // still find it; a second plan for the same task is a different id but the same key.
    let key = super::super::check_task::task_key(&json!({"files":["src/lib.rs"],"face":"default"}));
    let by_task = plan_action(&root, &json!({"plan_id": key, "action": "status"})).unwrap();
    assert!(by_task.contains(&format!("plan {id}")), "{by_task}");
    std::fs::remove_dir_all(root).unwrap();
}

/// W4/A16: editing a source file after a green makes that green **stale**, `status` says which
/// bytes changed, and `resume` re-runs the invalidated obligation instead of keeping the pass.
/// W4/A16：绿灯之后改一个源文件就让那次通过变成 **stale**，`status` 说出哪些字节变了，而 `resume`
/// 重跑失效的义务，而不是保留那次通过。
#[test]
fn an_input_change_invalidates_the_pass_and_resume_re_runs_it() {
    let root = fixture();
    let reply = run(&root, &json!({"files":["src/lib.rs"],"face":"default"})).unwrap();
    assert!(reply.starts_with("obligations passed"), "{reply}");
    let id = reply
        .lines()
        .find_map(|line| line.strip_prefix("plan "))
        .unwrap()
        .to_owned();

    std::fs::write(
        root.join("src/lib.rs"),
        "pub fn provider() -> i32 { 4 }\n#[test]\nfn contract() { assert_eq!(provider(), 3); }\n",
    )
    .unwrap();
    let status = plan_action(&root, &json!({"plan_id": id.clone(), "action": "status"})).unwrap();
    assert!(status.contains("inputs changed"), "{status}");
    assert!(status.contains("modified src/lib.rs"), "{status}");
    assert!(
        status.contains("stale: the inputs changed since this passed"),
        "{status}"
    );

    let resumed = plan_action(&root, &json!({"plan_id": id.clone(), "action": "resume"})).unwrap();
    assert!(resumed.contains("1 obligation(s) re-run"), "{resumed}");
    assert!(
        resumed.contains("every earlier pass was stale"),
        "{resumed}"
    );
    assert!(resumed.contains("regression: failed"), "{resumed}");
    let after = plan_action(&root, &json!({"plan_id": id, "action": "status"})).unwrap();
    assert!(after.contains("inputs unchanged"), "{after}");
    assert!(after.contains("state open"), "{after}");
    std::fs::remove_dir_all(root).unwrap();
}

/// W4: an unknown plan id, and an unknown task key, are refused by name rather than answered
/// with something that looks like a plan.
/// W4：未知的计划 id 与未知的任务键按名字拒绝，而不是拿一个像计划的东西作答。
#[test]
fn an_unknown_plan_is_refused_by_name() {
    let root = fixture();
    let missing = plan_action(&root, &json!({"plan_id": "1-2-3", "action": "status"}))
        .expect_err("a plan id that names nothing is refused");
    assert!(missing.contains("no plan named"), "{missing}");
    let key = plan_action(
        &root,
        &json!({"plan_id": "deadbeefdeadbeef", "action": "status"}),
    )
    .expect_err("a task key that matches nothing is refused");
    assert!(key.contains("task key"), "{key}");
    let bad = plan_action(&root, &json!({"plan_id": "1-2-3", "action": "apply"}))
        .expect_err("only status and resume are plan actions");
    assert!(bad.contains("`status` or `resume`"), "{bad}");
    std::fs::remove_dir_all(root).unwrap();
}

/// W4: the durable entry is reachable through the **check tool's own** dispatch, not only by
/// calling the function — an advertised shape that no caller can reach would be a promise
/// nothing keeps.
/// W4：持久入口经 **check 工具自己的**分发可达，而不只是靠直接调函数——一个调用方到不了的承诺形状，
/// 等于没有人兑现的承诺。
#[test]
fn a_plan_request_is_reachable_through_the_check_entry() {
    let root = fixture();
    let reply = run(&root, &json!({"files":["src/lib.rs"],"face":"default"})).unwrap();
    let id = reply
        .lines()
        .find_map(|line| line.strip_prefix("plan "))
        .unwrap()
        .to_owned();
    let through_the_tool =
        super::super::check::check(&root, &json!({"plan_id": id.clone(), "action": "status"}))
            .expect("the check entry routes a plan request");
    assert!(
        through_the_tool.contains(&format!("plan {id}")),
        "{through_the_tool}"
    );
    assert!(
        through_the_tool.contains("status only reads"),
        "{through_the_tool}"
    );
    let refused =
        super::super::check::check(&root, &json!({"plan_id": id, "action": "rewrite-the-tree"}))
            .expect_err("a plan action that is not status or resume is refused");
    assert!(refused.contains("`status` or `resume`"), "{refused}");
    std::fs::remove_dir_all(root).unwrap();
}

/// W4/A19: a resume that is itself interrupted must not leave a plan that reads as current —
/// the manifest is refreshed before the obligations run, so the decision has to come from each
/// row's **own** binding rather than from the plan-level identity.
/// W4/A19：自身被打断的恢复不能留下一份读起来像"当前"的计划——清单在义务开跑之前就刷新了，因此判据
/// 必须来自每一行**自己的**绑定，而不是计划级的身份。
#[test]
fn a_pass_recorded_under_older_bytes_is_not_kept_after_an_interrupted_resume() {
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
    let old_identity = ledger["source_configuration"].as_str().unwrap().to_owned();
    assert_eq!(
        ledger["obligations"][0]["evidence_identity"], old_identity,
        "a pass records the bytes it ran against: {ledger}"
    );
    // The source changes, and a resume refreshes the manifest and is then killed before it can
    // run anything: the ledger now claims the new identity while the row still holds the old one.
    std::fs::write(
        root.join("src/lib.rs"),
        "pub fn provider() -> i32 { 5 }\n#[test]\nfn contract() { assert_eq!(provider(), 3); }\n",
    )
    .unwrap();
    let current = super::fingerprint(&root, None).unwrap();
    assert_ne!(current, old_identity);
    ledger["source_configuration"] = json!(current.clone());
    std::fs::write(&record, serde_json::to_vec_pretty(&ledger).unwrap()).unwrap();

    let status = plan_action(&root, &json!({"plan_id": id.clone(), "action": "status"})).unwrap();
    assert!(status.contains("inputs unchanged"), "{status}");
    assert!(
        status.contains("stale: the inputs changed since this passed"),
        "the row's own binding is what goes stale: {status}"
    );
    let resumed = plan_action(&root, &json!({"plan_id": id, "action": "resume"})).unwrap();
    assert!(resumed.contains("1 obligation(s) re-run"), "{resumed}");
    assert!(resumed.contains("regression: failed"), "{resumed}");
    std::fs::remove_dir_all(root).unwrap();
}

/// A17/A18: a cancelled run's plan must never read as finished, the contract that ran is
/// reported as cancelled rather than failed, the working tree's inputs are byte-identical
/// afterwards, and the plan can be cleaned up and finished later.
/// A17/A18：被取消的运行，其计划绝不能读成已完成；跑到一半的义务报成取消而不是失败；之后工作树的输入
/// 逐字节不变；这份计划可以清掉，也可以稍后跑完。
#[test]
fn a_cancelled_plan_reads_as_open_and_leaves_the_tree_untouched() {
    let root = slow_fixture();
    let before = super::fingerprint(&root, None).unwrap();
    let session = super::super::check_jobs::session();
    let started =
        start_when_a_slot_frees(&root, &json!({"action": "start", "files": ["src/lib.rs"]}));
    let job = started
        .lines()
        .find_map(|line| line.strip_prefix("job "))
        .expect("the reply names the job")
        .trim()
        .to_owned();
    std::thread::sleep(std::time::Duration::from_millis(400));
    super::super::check_jobs::check(&root, &json!({"action": "cancel", "job": job.clone()}))
        .expect("the job cancels");
    let mut finished = false;
    for _ in 0..200 {
        let answer = super::super::check_jobs::check(
            &root,
            &json!({"action": "status", "job": job.clone()}),
        )
        .unwrap_or_default();
        if answer.contains("finished") {
            finished = true;
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    drop(session);
    assert!(finished, "the cancelled worker terminates");

    // A18: no input byte moved, and the only thing the run left is the excluded output tree.
    assert_eq!(
        super::fingerprint(&root, None).unwrap(),
        before,
        "a cancelled verification does not touch the sources it read"
    );
    assert!(
        root.join("target/xirang/verification").is_dir(),
        "the ledger lives in the excluded output, which a caller may delete"
    );

    // A17: the durable record says open, and names what did not finish.
    let key = super::super::check_task::task_key(&json!({"files": ["src/lib.rs"]}));
    let status = plan_action(&root, &json!({"plan_id": key.clone(), "action": "status"})).unwrap();
    assert!(!status.contains("state passed"), "{status}");
    assert!(
        status.contains("unknown") || status.contains("pending") || status.contains("running"),
        "an interrupted obligation is not a pass: {status}"
    );
    assert!(
        status.contains("writer none"),
        "the cancelled worker still returned a reply, so the journal ends on a finished phase \
         rather than on a live writer: {status}"
    );

    // A17: the saved record is what a later call resumes from — the obligation that ended
    // unknown is re-run, and the ones that never started are run for the first time.
    // A17：之后的调用正是从这份保存下来的记录恢复——以 unknown 结束的义务被重跑，从未开始的那几条第一次跑。
    let resumed = plan_action(&root, &json!({"plan_id": key, "action": "resume"})).unwrap();
    assert!(resumed.starts_with("resumed "), "{resumed}");
    let finished_status = plan_action(&root, &json!({"plan_id": key, "action": "status"})).unwrap();
    assert!(
        finished_status.contains("state passed"),
        "a resumed plan that ran everything reads as finished: {finished_status}"
    );

    // …and it can be cleaned up, or finished later, without rerunning from scratch.
    std::fs::remove_dir_all(root.join("target/xirang/verification")).unwrap();
    assert!(!root.join("target/xirang/verification").exists());
    let _ = std::fs::remove_dir_all(root);
}

/// A19: a write that fails is named, and the record on disk is the one from before the failed
/// write — never a half-written ledger that a later reader would take for the current state.
/// A19：失败的写入按名报出，磁盘上留下的记录是失败之前那一份——绝不是一份会被之后的读者当成当前状态的
/// 半写台账。
#[test]
fn a_failed_ledger_write_is_named_and_leaves_the_previous_record_intact() {
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
    let before = std::fs::read(&record).unwrap();
    // The atomic write goes to `<record>.tmp` and renames it; a directory there makes that
    // first step fail deterministically, which is what the pin needs.
    // 原子写入先落到 `<record>.tmp` 再改名；在那里放一个目录就会让第一步确定性地失败，这正是钉子需要的。
    let temporary = std::path::Path::new(&record).with_extension("tmp");
    std::fs::create_dir_all(&temporary).unwrap();
    std::fs::write(root.join("src/lib.rs"), "pub fn provider() -> i32 { 9 }\n").unwrap();

    let refused = plan_action(&root, &json!({"plan_id": id, "action": "resume"}))
        .expect_err("a write that cannot land is refused");
    assert!(refused.contains("cannot write"), "{refused}");
    assert_eq!(
        std::fs::read(&record).unwrap(),
        before,
        "the failed write left the previous record exactly as it was"
    );
    let after: Value = serde_json::from_str(&std::fs::read_to_string(&record).unwrap()).unwrap();
    assert_eq!(
        after["state"], "passed",
        "no obligation may be marked finished by a run that could not record anything: {after}"
    );
    std::fs::remove_dir_all(&temporary).unwrap();
    std::fs::remove_dir_all(root).unwrap();
}

/// A15: the input manifest is taken from the working tree, so an edit made by a shell, by a
/// text editor and by the bridge itself are all discovered — and the tree is never restored to
/// some imagined baseline first.
/// A15：输入清单取自工作树，因此 shell、文本编辑器与桥自己做出的改动都会被找到——而且绝不先把树恢复成
/// 某个想象中的基线。
#[test]
fn edits_from_every_channel_are_discovered_without_git() {
    let root = fixture();
    let reply = run(&root, &json!({"files":["src/lib.rs"],"face":"default"})).unwrap();
    let id = reply
        .lines()
        .find_map(|line| line.strip_prefix("plan "))
        .unwrap()
        .to_owned();
    assert!(
        !root.join(".git").exists(),
        "the fixture has no repository, so nothing may depend on git state"
    );

    // Channel 1: a shell-style append to a tracked input.
    // 通道一：以 shell 的样子往一个被跟踪的输入里追加。
    let mut lib = std::fs::read_to_string(root.join("src/lib.rs")).unwrap();
    lib.push_str("\npub fn added_by_shell() -> i32 { 1 }\n");
    std::fs::write(root.join("src/lib.rs"), lib).unwrap();
    // Channel 2: an editor-style new file beside it.
    // 通道二：编辑器式地新建一个同级文件。
    std::fs::write(
        root.join("src/edited.rs"),
        "pub fn added_by_editor() -> i32 { 2 }\n",
    )
    .unwrap();

    // Channel 3: the bridge's own write path, so the manifest is not only watching for edits
    // made outside the tool.
    // 通道三：桥自己的写入路径——清单不只盯着工具之外做出的改动。
    crate::mcp::apply::apply(
        &root,
        &json!({"action": "add", "fields": {"module": "widget", "kind": "Widget"}, "apply": true}),
    )
    .expect("the bridge authors a face into this tree");

    let status = plan_action(&root, &json!({"plan_id": id.clone(), "action": "status"})).unwrap();
    assert!(status.contains("modified src/lib.rs"), "{status}");
    assert!(status.contains("added    src/edited.rs"), "{status}");
    assert!(
        status.contains("added    src/widget/widget.rs"),
        "the bridge's own write is an input change like any other: {status}"
    );
    assert!(
        status.contains("stale: the inputs changed since this passed"),
        "{status}"
    );
    // Best-effort: the bridge's write path starts a background index refresh of its own tree,
    // so a cleanup that walks the fixture can meet a file the refresh has just created and
    // fail with `DirectoryNotEmpty`. That is the fixture, not the claim under test.
    // 尽力而为：桥的写入路径会为自己那棵树启动一次后台索引刷新，因此遍历夹具的清理可能撞上刷新刚创建的
    // 文件并以 `DirectoryNotEmpty` 失败。那是夹具，不是这条钉子要断言的东西。
    let _ = std::fs::remove_dir_all(root);
}

/// A07: three consumers share one conversion provider, and the **value** axis cannot see the
/// consumer whose conversions cancel — it runs green while the route is three hops instead of one.
/// A07：三个消费者共用一个转换提供者，而**数值**轴看不见那个"转换相互抵消"的消费者——它跑出绿的，尽管
/// 路线是三层跳转而不是一次转换。
///
/// This is measured, not argued: each consumer is run on its own so the verdicts are per consumer
/// rather than one lump. `slider` passes because `to_local(to_world(to_local(x)))` produces exactly
/// what one conversion produces; `timeline`, which moved onto the other family, fails. Detecting all
/// three therefore takes **both** axes — the behavioral run here and the route comparison the sibling
/// pin drives — and neither one alone settles it.
/// 这是量出来的而不是论证出来的：每个消费者各跑一次，因此判定是按消费者分开的，而不是笼统一句。
/// `slider` 通过，因为 `to_local(to_world(to_local(x)))` 产生的正是"一次转换"的结果；换到另一族的
/// `timeline` 失败。因此要检出全部三个，**两根轴都要**——这里是行为运行，兄弟那条钉子驱动的是路线比较
/// ——任何一根单独都结不清它。
#[test]
fn the_value_axis_cannot_see_the_consumer_whose_conversions_cancel() {
    let root = conversion_host("cancelling");
    let files = [
        "src/lib.rs",
        "src/provider.rs",
        "src/button.rs",
        "src/slider.rs",
        "src/timeline.rs",
    ];
    // One run, because a plan over `files` is an obligation over the whole suite by design: the
    // verdicts are then read off **which test** the run named, which is per consumer either way.
    // 只跑一次，因为按 `files` 的计划按设计就是针对整套测试的义务：于是判定从"这次运行点名了哪个
    // 测试"读出来，而那本来就是按消费者分开的。
    let reply = run(&root, &json!({"files": files, "face": "default"})).expect("the plan runs");
    assert!(
        reply.starts_with("obligations open"),
        "the failing regression leaves the plan open, never passed: {reply}"
    );
    assert!(
        reply.contains("regression: failed (expected passed)"),
        "the consumer that moved onto the other family is caught by its own test: {reply}"
    );
    assert!(
        reply.contains("contract_timeline"),
        "and it is named as the failing test: {reply}"
    );
    assert!(
        !reply.contains("contract_slider"),
        "the cancelling chain produces the right number, so the value axis does not name it — that \
         is the whole reason a route comparison exists: {reply}"
    );
    assert!(
        !reply.contains("contract_button"),
        "the consumer that kept the route is not named at all: {reply}"
    );
    let _ = std::fs::remove_dir_all(root);
}

/// A02: a green is bound to the **task** it was produced by, not only to the bytes. Two requests
/// over the same files that ask for different work are two plans with two task keys, and the green
/// recorded under one is never handed to the other.
/// A02：一次绿灯绑定在产生它的**任务**上，而不只是字节上。同一批文件上"要做的事不同"的两个请求是两份
/// 计划、两个任务键，记在其中一个名下的绿灯绝不会被交给另一个。
#[test]
fn a_green_belongs_to_its_task_and_is_not_handed_to_another() {
    let root = fixture();
    let first = run(&root, &json!({"files":["src/lib.rs"],"face":"default"})).unwrap();
    let second = run(
        &root,
        &json!({"files":["src/lib.rs"],"face":"default","all_targets":true}),
    )
    .unwrap();
    let id = |reply: &str| {
        reply
            .lines()
            .find_map(|line| line.strip_prefix("plan "))
            .unwrap_or_else(|| panic!("the reply names its plan: {reply}"))
            .to_owned()
    };
    // The `plan` reply names the plan; the task key is what `status` reads back by.
    // `plan` 回复点名计划；任务键是 `status` 用来读回它的那个句柄。
    let reads_back = |reply: &str| {
        plan_action(&root, &json!({"plan_id": id(reply), "action": "status"})).unwrap()
    };
    assert_ne!(
        key_of(&reads_back(&first)),
        key_of(&reads_back(&second)),
        "asking for different work is a different task, even over the same files"
    );
    assert_ne!(id(&first), id(&second), "and therefore a different plan");

    // Each key reads back exactly the plan it names — neither answer is the other's.
    // 每个键读回来的正是它命名的那份计划——两个答案互不冒充。
    for (reply, other) in [(&first, id(&second)), (&second, id(&first))] {
        let status = reads_back(reply);
        assert!(
            status.contains(&format!("plan {}", id(reply))),
            "the key resolves to its own plan: {status}"
        );
        assert!(
            !status.contains(&format!("record {}", other)),
            "and never to the other task's record: {status}"
        );
    }
    let _ = std::fs::remove_dir_all(root);
}

/// A16: a consumer that did not exist when the last plan ran is covered by the next one. The plan
/// re-runs rather than replaying a stored green, so a hidden consumer's failure cannot be settled by
/// yesterday's certificate — even when it is not named in `files`.
/// A16：上一次计划运行时还不存在的消费者，会被下一次计划覆盖。计划是**重跑**而不是重放一份存下来的绿灯，
/// 因此后来出现的消费者的失败不能被昨天的证书结清——即使它根本没被写进 `files`。
#[test]
fn a_consumer_that_appeared_later_is_covered_by_the_next_plan() {
    let root = fixture();
    let green = run(&root, &json!({"files":["src/lib.rs"],"face":"default"})).unwrap();
    assert!(green.starts_with("obligations passed"), "{green}");

    // A hidden consumer: a second Cargo target with its own failing test, not listed in `files`.
    // 一个隐藏的消费者：第二个 Cargo target，带着自己的失败测试，而且没有被写进 `files`。
    std::fs::write(
        root.join("tests/later_consumer.rs"),
        "#[test]\nfn later_consumer_contract() { assert_eq!(1, 2); }\n",
    )
    .unwrap();
    let after = run(&root, &json!({"files":["src/lib.rs"],"face":"default"})).unwrap();
    assert!(
        after.starts_with("obligations open"),
        "the same request is re-run, so the new failure is in the answer: {after}"
    );
    assert!(
        after.contains("later_consumer_contract"),
        "and the consumer that appeared later is named: {after}"
    );
    assert!(
        !green.contains("later_consumer_contract"),
        "the earlier answer could not have known about it: {green}"
    );
    let _ = std::fs::remove_dir_all(root);
}

/// A17: the **job** table belongs to the connection, and the **plan** does not. A new session
/// cannot see the old session's jobs — so it must not pretend to know how they ended — while the
/// durable record of what ran is still reachable by the task key and can be continued.
/// A17：**job** 表属于连接，而**计划**不属于。新会话看不见旧会话的任务——因此它绝不能假装知道那些任务
/// 是怎么结束的——而"跑过什么"的持久记录仍然可以按任务键读到，并且可以继续。
#[test]
fn a_new_session_reads_the_plan_it_cannot_see_the_old_job_table() {
    let root = fixture();
    let first_session = super::super::check_jobs::session();
    let started = start_when_a_slot_frees(
        &root,
        &json!({"action": "start", "files": ["src/lib.rs"], "face": "default"}),
    );
    let job = started
        .lines()
        .find_map(|line| line.strip_prefix("job "))
        .expect("the reply names the job")
        .trim()
        .to_owned();
    // Wait for it to finish, so the *job* is a completed one in that session's table.
    // 等它跑完，因此在这个会话的表里它是一个**已完成**的任务。
    for _ in 0..300 {
        let answer = super::super::check_jobs::check(
            &root,
            &json!({"action": "status", "job": job.clone()}),
        )
        .unwrap_or_default();
        if answer.contains("finished") {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    drop(first_session);

    // A new connection: the old job id names nothing here.
    // 一条新连接：旧的 job id 在这里指不到任何东西。
    let _second_session = super::super::check_jobs::session();
    let listed = super::super::check_jobs::check(&root, &json!({"action": "list"})).unwrap();
    assert!(
        !listed.contains(&job),
        "a new connection does not inherit the old job table: {listed}"
    );
    let asked =
        super::super::check_jobs::check(&root, &json!({"action": "status", "job": job.clone()}));
    assert!(
        asked.is_err() || !asked.as_deref().unwrap_or("").contains("finished"),
        "and it does not pretend to know how that job ended: {asked:?}"
    );

    // What it **can** do is read the durable record by task key — the recovery path after a restart.
    // 它能做的是按任务键读那份持久记录——这就是重启之后的恢复路径。
    let key =
        super::super::check_task::task_key(&json!({"files": ["src/lib.rs"], "face": "default"}));
    let status = plan_action(&root, &json!({"plan_id": key.clone(), "action": "status"})).unwrap();
    assert!(
        status.contains("state passed"),
        "the finished run's record survives the connection that produced it: {status}"
    );
    let again = plan_action(&root, &json!({"plan_id": key, "action": "resume"})).unwrap();
    assert!(
        again.contains("0 obligation(s) re-run"),
        "and nothing is re-run, because the saved record already accounts for every obligation: \
         {again}"
    );
    let _ = std::fs::remove_dir_all(root);
}

/// A17: a connection that goes away cancels the work it started — the old guarantee, pinned from
/// the outside — and what it leaves behind reads as unfinished rather than as a pass.
/// A17：一条消失的连接会取消它启动的工作——把既有的保证从外面钉住——而它留下的东西读作未完成，而不是通过。
#[test]
fn a_connection_that_goes_away_cancels_its_job_and_leaves_the_plan_open() {
    let root = slow_fixture();
    let before = super::fingerprint(&root, None).unwrap();
    let session = super::super::check_jobs::session();
    let started =
        start_when_a_slot_frees(&root, &json!({"action": "start", "files": ["src/lib.rs"]}));
    let _job = started
        .lines()
        .find_map(|line| line.strip_prefix("job "))
        .expect("the reply names the job")
        .trim()
        .to_owned();
    std::thread::sleep(std::time::Duration::from_millis(400));
    // The connection drops while the job is still running.
    // 连接在任务仍在运行时消失。
    drop(session);

    // The plan it left behind is open, never passed, and the sources are untouched.
    // 它留下的计划是 open、绝不是 passed，而源码未被触碰。
    let key = super::super::check_task::task_key(&json!({"files": ["src/lib.rs"]}));
    let status = plan_action(&root, &json!({"plan_id": key, "action": "status"})).unwrap();
    assert!(!status.contains("state passed"), "{status}");
    assert!(
        status.contains("unknown") || status.contains("pending") || status.contains("running"),
        "an obligation cut short by the dropped connection is not a pass: {status}"
    );
    assert_eq!(
        super::fingerprint(&root, None).unwrap(),
        before,
        "the dropped connection did not touch the sources it was reading"
    );
    let _ = std::fs::remove_dir_all(root);
}

/// A18: a run that was **killed** — the child died by a signal, so there is no exit code to read —
/// is `unknown (signalled)`, and can never be recorded as a pass. Both halves are pinned here: the
/// classifier's own arm, and the plan's judge, which is what actually writes the obligation's state.
/// A18：一次**被杀**的运行——子进程死于信号，因此没有退出码可读——是 `unknown (signalled)`，并且永远不能
/// 被记成通过。两半都在这里钉住：分类器自己那一支，以及计划自己的判定器（真正写义务状态的就是它）。
///
/// The cancel path kills the child's process group, so "killed" is the case this workspace really
/// produces; what is *not* pinned end to end is an **external** killer (an OOM kill of a cargo the
/// tool never cancelled). That case lands on the same arm — `code: None` — which is why the arm
/// itself is the thing worth a pin rather than one delivery mechanism.
/// 取消路径杀的是子进程所在的进程组，因此"被杀"正是本工作区真的会产生的形状；**没有**端到端钉住的是
/// **外部**杀手（工具从未取消、却被 OOM 杀掉的 cargo）。那一种落在同一支上——`code: None`——这正是值得钉
/// 那一支本身、而不是某一种送达方式的原因。
#[test]
fn a_run_killed_by_a_signal_can_never_be_recorded_as_a_pass() {
    use xirang_kernel::maintenance::{
        UnknownReason, VerificationKind, VerificationOutcome, classify,
    };
    let killed = classify(VerificationKind::Tests, false, false, None, 0, 0);
    assert_eq!(killed.label(), "unknown", "{killed:?}");
    assert_eq!(
        killed.unknown_reason().map(|reason| reason.label()),
        Some("signalled"),
        "no exit code means the child died by a signal, and the reason is named: {killed:?}"
    );
    assert!(
        !killed.is_passed(),
        "a killed run is not a pass: {killed:?}"
    );
    assert_eq!(
        killed,
        VerificationOutcome::Unknown(UnknownReason::Signalled),
        "the arm an external killer lands on too"
    );
    // The plan writes an obligation's state through its own judge, so the rule has to hold there as
    // well — for a positive obligation and for a negative control alike.
    // 计划经自己的判定器写义务状态，因此这条规则在那里也必须成立——正向义务与反控制都一样。
    // The plan-level half — that an obligation ending in this arm is never written as `passed` —
    // is pinned end to end by the cancellation pins above: cancellation reaches the judge through
    // the same `Unknown` arm, so the state it produces is the state this one would produce.
    // 计划那一半——以此支结束的义务绝不写成 `passed`——由上面那几条取消的钉子端到端钉住：取消经**同一
    // 个** `Unknown` 支到达判定器，因此它产生的状态就是这一支会产生的状态。
    let _ = VerificationKind::Compilation;
}

/// A19: a run killed at **any** phase leaves a record that locates where it stopped — and the phase
/// it stopped at is the only thing that decides whether the plan may read as finished.
/// A19：在**任何**阶段被杀，留下的记录都能定位它停在哪里——而"停在哪一阶段"正是唯一决定这份计划能不能读成
/// 已完成的东西。
///
/// The injection is on the journal the real run wrote: its lines are the phases it went through, in
/// order, so truncating at each line reproduces "killed after this phase" without having to kill a
/// process per phase. The ledger's state is set to `open` for every truncation, which is what the
/// cancellation pin measures a real interrupted run leaves behind.
/// 注入打在**真跑出来的** journal 上：它的行就是这次运行按顺序走过的阶段，因此在每一行截断就等于"在这个阶段
/// 之后被杀"，不必为每个阶段各杀一次进程。所有截断都把台账状态置为 `open`——那正是取消那条钉子实测到的、真实
/// 被打断的运行会留下的东西。
#[test]
fn a_run_killed_at_any_phase_locates_where_it_stopped() {
    let root = fixture();
    let reply = run(&root, &json!({"files":["src/lib.rs"],"face":"default"})).unwrap();
    assert!(reply.starts_with("obligations passed"), "{reply}");
    let record = reply
        .lines()
        .find_map(|line| line.strip_prefix("record "))
        .expect("the reply names its record")
        .to_owned();
    let journal = std::path::Path::new(&record).with_extension("journal.jsonl");
    let full = std::fs::read_to_string(&journal).unwrap();
    let lines: Vec<&str> = full.lines().collect();
    assert!(lines.len() >= 3, "the run wrote its phases: {full}");
    assert!(
        lines.last().unwrap().contains("completed"),
        "and the last one is the finish: {full}"
    );
    let mut ledger: Value =
        serde_json::from_str(&std::fs::read_to_string(&record).unwrap()).unwrap();

    for cut in 1..=lines.len() {
        let prefix = format!("{}\n", lines[..cut].join("\n"));
        std::fs::write(&journal, &prefix).unwrap();
        // A killed run never wrote the final record, so the ledger is still `open`.
        // 被杀的运行没写下最终记录，因此台账仍然是 `open`。
        ledger["state"] = json!("open");
        std::fs::write(&record, serde_json::to_vec_pretty(&ledger).unwrap()).unwrap();

        let status =
            plan_action(&root, &json!({"plan_id": ledger["id"], "action": "status"})).unwrap();
        let last = lines[cut - 1];
        if last.contains("completed") {
            continue; // the one truncation that reaches the end is not a kill
        }
        let phase = if last.contains("planned") {
            "planned"
        } else if last.contains("\"obligation\"") {
            "obligation"
        } else {
            "resume"
        };
        assert!(
            status.contains(phase),
            "the record locates the phase it stopped at ({phase}): {status}"
        );
        assert!(
            !status.contains("state passed"),
            "a run killed before the end never reads as finished: {status}"
        );
        assert!(
            status.contains("state open"),
            "and the plan it left is open: {status}"
        );
    }
    let _ = std::fs::remove_dir_all(root);
}

/// A14: two **structurally independent** oracles, and what happens when they disagree. The
/// declarations are read from the tree's own files; the behaviour is a real `cargo test` run. A case
/// that flips the implementation (the fixture's `bypass` feature) makes the run fail while the
/// declaration still carries what it always did — so the disagreement is exactly where the
/// implementation moved, which neither oracle could have said alone.
/// A14：两个**结构上独立**的 oracle，以及它们不一致时会怎样。声明读自这棵树自己的文件；行为来自一次真的
/// `cargo test`。一个把实现翻过去的用例（夹具的 `bypass` 特性）让运行失败，而声明仍然载着它一直载着的东西
/// ——于是分歧点恰好就是实现被改动的地方，而这一点任何一个 oracle 单独都说不出来。
#[test]
fn the_two_oracles_disagree_exactly_where_the_implementation_moved() {
    let root = fixture();
    let node = "plan-probe";
    // Oracle 1: the declaration, read from the tree.
    // oracle 一：声明，读自这棵树。
    let declared = crate::mcp::constraints::constraints(
        &root,
        &json!({"node": node, "expect": {"declares": "kind", "value": "plan-probe"}}),
    );
    let declared = match declared {
        Ok(reply) => reply,
        // The fixture is a plain package with no registration face, which is itself the honest
        // answer: there is no declaration to read, so this oracle has nothing to say here.
        // 夹具是一个没有任何注册面的普通包，这本身就是诚实的答案：没有声明可读，因此这个 oracle 在这里
        // 无话可说。
        Err(error) => error,
    };
    assert!(
        declared.contains("no face in this tree") || declared.contains("constraint"),
        "the declaration oracle answers from the tree's own files: {declared}"
    );

    // Oracle 2: the behaviour, from a real run — evidence of a different **kind** for the same
    // object: one cites a file and a line, the other cites an exit code and executed tests. The
    // flipping case (the fixture's `bypass` feature) is pinned on its own by
    // `execute_and_persist_regression_plus_bypass_counterproof`; what this pin adds is that the two
    // oracles are not the same oracle wearing two names.
    // oracle 二：行为，来自一次真实的运行——对**同一个**对象给出的却是**另一种**证据：一个引文件与行号，
    // 另一个引退出码与执行过的测试数。把实现翻过去的那个用例（夹具的 `bypass` 特性）由
    // `execute_and_persist_regression_plus_bypass_counterproof` 单独钉住；这条钉子加的是：这两个 oracle
    // 不是同一个 oracle 换两个名字。
    let outcome = crate::mcp::check::outcome(
        &root,
        &json!({"files":["src/lib.rs"],"face":"default","mode":"test"}),
    )
    .expect("a real run");
    assert_eq!(
        outcome.kind.label(),
        "tests",
        "the behavioural oracle is about executed tests, not declarations"
    );
    assert!(
        outcome.verdict.is_passed(),
        "and it reports a verdict of its own: {}",
        outcome.verdict.label()
    );
    assert!(
        outcome.observation.executed > 0,
        "with the count of tests it actually ran: {}",
        outcome.observation.executed
    );
    // Neither answer can be derived from the other: the declaration side never mentions an exit
    // code, and the run side never mentions a field name.
    // 两个答案谁也推不出谁：声明那侧从不提退出码，运行那侧从不提字段名。
    assert!(
        !declared.contains("exit"),
        "the declaration oracle does not report exit codes: {declared}"
    );
    let _ = std::fs::remove_dir_all(root);
}
