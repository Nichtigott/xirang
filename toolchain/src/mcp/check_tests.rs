//! Pins for the observed run: a failed run reports what it said, a timeout is `unknown` rather than
//! a pass, and a log with no result line says nothing ran.
//! 被观测运行的钉子：失败的运行如实报出它说的，超时是 `unknown` 而不是通过，没有结果行的日志说"什么都没跑"。

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use super::{
    CheckOutcome, FailedTest, Observation, RunOutcome, head, next_step, observation, run_command,
};

/// A throwaway directory for the logs.
/// 一个一次性的日志目录。
fn scratch(label: &str) -> std::path::PathBuf {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "xirang-mcp-check-{label}-{}-{sequence}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("scratch dir");
    root
}

#[test]
fn check_refuses_ignored_selectors_and_wrong_types_before_running_cargo() {
    let root = scratch("request-contract");
    for request in [
        serde_json::json!({"features": ["dev-supervisor"]}),
        serde_json::json!({"package": "consumer", "workspace": true}),
        serde_json::json!({"face": true}),
        serde_json::json!({"face": ""}),
        serde_json::json!({"face": "--all-features"}),
        serde_json::json!({"face": "a,,b"}),
        serde_json::json!({"target_dir": false}),
        serde_json::json!({"root": false}),
        serde_json::json!({"census": "false"}),
        serde_json::json!({"verbose": 1}),
        serde_json::json!({"timeout_ms": "1000"}),
        serde_json::json!([]),
    ] {
        assert!(super::check(&root, &request).is_err(), "{request}");
        assert_eq!(std::fs::read_dir(&root).unwrap().count(), 0, "{request}");
    }
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn check_runs_hyphenated_combined_and_package_qualified_features() {
    let root = scratch("feature-selection");
    std::fs::create_dir(root.join("src")).unwrap();
    std::fs::write(root.join("Cargo.toml"),
        "[package]\nname='check-selection-probe'\nversion='0.0.0'\nedition='2024'\n[workspace]\n[features]\ndefault=[]\ndev-supervisor=[]\nstudio=[]\n").unwrap();
    std::fs::write(root.join("src/lib.rs"),
        "#[test]\nfn selected_feature_fails() { assert!(!cfg!(feature=\"dev-supervisor\"), \"selected feature must fail\"); }\n").unwrap();
    let default = super::check(&root, &serde_json::json!({})).unwrap();
    assert!(default.starts_with("verdict  passed"), "{default}");
    for face in [
        "dev-supervisor",
        "dev-supervisor,studio",
        "check-selection-probe/dev-supervisor",
    ] {
        let answer = super::check(&root, &serde_json::json!({"face": face})).unwrap();
        assert!(answer.starts_with("verdict  failed"), "{answer}");
        assert!(answer.contains("selected feature must fail"), "{answer}");
        assert!(
            answer.contains(&format!("check  cargo test --features {face}")),
            "{answer}"
        );
        let log = answer
            .lines()
            .find_map(|line| line.strip_prefix("log    "))
            .unwrap();
        assert_eq!(
            Path::new(log).parent().unwrap(),
            root.join("target/xirang/out")
        );
        assert!(Path::new(log).is_file());
    }
    std::fs::remove_dir_all(root).unwrap();
}

/// A failing run comes back with its exit code and its own result line, and nothing invented.
/// 失败的运行带着自己的退出码与结果行回来，没有编造的东西。
#[test]
fn a_failing_run_reports_what_it_said() {
    let root = scratch("failed");
    let log = root.join("check-all.log");
    let outcome = run_command(
        Command::new("sh").arg("-c").arg(
            "printf 'test result: FAILED. 1 passed; 1 failed; 0 ignored\\ntest alpha ... FAILED\\n'; \
             exit 101",
        ),
        Duration::from_secs(30),
        &log,
    )
    .expect("the command runs");
    assert_eq!(outcome.code, Some(101), "{:?}", outcome.code);
    assert!(!outcome.timed_out);
    let observed = observation(&log, true).expect("the log reads");
    assert_eq!(observed.results, 1, "one `test result:` line was read");
    let joined = observed.lines.join("\n");
    assert!(joined.contains("1 passed; 1 failed"), "{joined}");
    assert!(joined.contains("failed alpha"), "{joined}");
    let _ = std::fs::remove_dir_all(&root);
}

/// A timeout is `unknown`: it says what was killed and that the run is not evidence of a pass.
/// 超时是 `unknown`：它说出杀的是什么，并说明这次运行不是通过的证据。
#[test]
fn a_timeout_is_unknown_and_never_a_pass() {
    let root = scratch("timeout");
    let log = root.join("check-default.log");
    let outcome = run_command(
        Command::new("sh").arg("-c").arg("sleep 30"),
        Duration::from_millis(1_200),
        &log,
    )
    .expect("the command runs");
    assert!(outcome.timed_out, "{:?}", outcome.code);
    assert!(
        outcome.elapsed < Duration::from_secs(10),
        "it was killed early"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// A log with no result line reports that nothing ran, instead of reading as an empty success.
/// 没有结果行的日志报出"什么都没跑"，而不是被读成一次空的成功。
#[test]
fn a_log_without_a_result_line_says_nothing_ran() {
    let root = scratch("nothing");
    let log = root.join("check-extra.log");
    std::fs::write(
        &log,
        "   Compiling something v0.1.0\nerror: could not compile\n",
    )
    .expect("log");
    let joined = observation(&log, true)
        .expect("the log reads")
        .lines
        .join("\n");
    assert!(joined.contains("nothing ran"), "{joined}");
    assert!(joined.contains("not a pass"), "{joined}");
    let _ = std::fs::remove_dir_all(&root);
}

/// A finished command, as the verdict sees it.
/// 一条结束了的命令，按判定看到的样子。
fn outcome(code: Option<i32>, timed_out: bool) -> RunOutcome {
    RunOutcome {
        code,
        timed_out,
        cancelled: false,
        elapsed: Duration::from_millis(200),
        log: PathBuf::from("/tree/target/xirang/out/check-default.log"),
    }
}

/// One observed log, with the number of `test result:` lines the verdict reads.
/// 一份被观测的日志，带上判定要读的 `test result:` 行数。
fn observed(lines: &[&str], results: usize) -> Observation {
    Observation {
        skipped: 0,
        lines: lines.iter().map(|line| (*line).to_owned()).collect(),
        results,
        executed: results,
        failed: Vec::new(),
        targets: std::collections::BTreeMap::new(),
        target_labels: std::collections::BTreeMap::new(),
        incomplete: false,
        first_location: None,
    }
}

/// One observed log with the routing facts filled in (audit `W5-5`).
/// 一份把路由事实填好的被观测日志（审计 `W5-5`）。
fn routed(results: usize, failed: &[&str], first_location: Option<&str>) -> Observation {
    routed_with(results, failed, first_location, &[])
}

/// The same, with the reply's own lines, for a route that reads them (audit `W5-5`).
/// 同上，外加回复自己的那些行，供读它们的那种路由（审计 `W5-5`）使用。
fn routed_with(
    results: usize,
    failed: &[&str],
    first_location: Option<&str>,
    lines: &[&str],
) -> Observation {
    Observation {
        skipped: 0,
        lines: lines.iter().map(|line| (*line).to_owned()).collect(),
        results,
        executed: results,
        failed: failed
            .iter()
            .map(|name| FailedTest {
                name: (*name).to_owned(),
                binary: None,
                message: None,
            })
            .collect(),
        targets: std::collections::BTreeMap::new(),
        target_labels: std::collections::BTreeMap::new(),
        incomplete: false,
        first_location: first_location.map(str::to_owned),
    }
}

/// The typed outcome `head` renders from, with the verdict classified from the same facts.
/// `head` 渲染所依据的类型化结果，判定由同一批事实分类得出。
fn outcome_of(code: Option<i32>, timed_out: bool, observed: Observation) -> CheckOutcome {
    let verdict = xirang_kernel::maintenance::classify(
        xirang_kernel::maintenance::VerificationKind::Tests,
        timed_out,
        false,
        code,
        observed.results,
        observed.executed,
    );
    CheckOutcome {
        root: PathBuf::from("/tree"),
        face: "default".to_owned(),
        timeout: Duration::from_secs(900),
        mode: "test".to_owned(),
        kind: xirang_kernel::maintenance::VerificationKind::Tests,
        verdict,
        run: outcome(code, timed_out),
        observation: observed,
        command_line: None,
        run_id: "test".to_owned(),
    }
}

/// The verdict is in the first two lines of the reply, so a reader that stops at the top cannot read
/// a failing face as green.
/// 判定在回复头两行里，因此只在开头停下的读者不会把失败的面读成绿。
///
/// This is the pin for the defect round 7 measured on the frozen fixture `target/round7/s5`: the
/// failure was on line six (`exit   101`) while the one-shot client exited `0`. Both halves are
/// pinned here — the head says `failed`, and the `exit` line is still there for the code itself.
/// 这条钉子钉住第七轮在冻结夹具 `target/round7/s5` 上量到的缺陷：失败在第六行（`exit   101`），而一次性
/// 客户端以 `0` 退出。两半都在这里钉住——头部说 `failed`，而 `exit` 行仍保留着那个码本身。
#[test]
fn the_first_two_lines_are_the_verdict() {
    let failing = head(&outcome_of(
        Some(101),
        false,
        observed(
            &["result test result: FAILED. 0 passed; 3 failed; 0 ignored"],
            1,
        ),
    ));
    let top = failing
        .iter()
        .take(2)
        .cloned()
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        top.contains("verdict  failed (cargo exit 101)"),
        "the verdict is on the reply's first line: {top}"
    );
    assert!(
        !top.contains("passed"),
        "a failing run cannot be read as green from the head: {top}"
    );
    assert!(
        failing.iter().any(|line| line.starts_with("exit   101")),
        "the `exit` line stays, with the run's own code: {failing:?}"
    );

    let passing = head(&outcome_of(
        Some(0),
        false,
        observed(&["result test result: ok. 1 passed; 0 failed"], 1),
    ));
    assert!(
        passing[0].starts_with("verdict  passed (cargo exit 0)"),
        "{passing:?}"
    );

    // A log with no `test result:` line is not a pass even when cargo exited 0, and a killed run is
    // not a pass either: both are `unknown`, and the first line says so.
    // 没有 `test result:` 行的日志即使 cargo 以 0 退出也不是通过，被杀死的运行同样不是：两者都是
    // `unknown`，而第一行就这么说。
    let nothing = head(&outcome_of(
        Some(0),
        false,
        observed(&["result none (nothing ran, and that is not a pass)"], 0),
    ));
    assert!(
        nothing[0].starts_with("verdict  unknown (cargo exit 0"),
        "{nothing:?}"
    );
    let timed = head(&outcome_of(None, true, observed(&[], 0)));
    assert!(timed[0].starts_with("verdict  unknown"), "{timed:?}");
}

/// A run that did not pass says where to look next, and one that passed says nothing extra.
/// 没有通过的运行会说下一步去哪找，而通过的运行不多说一句。
#[test]
fn a_failing_run_says_where_to_look_next() {
    // The third argument is this reply's own lines: since T-22 the hint is instantiated from them,
    // so an empty slice is what a reply with no `why` line carries — and that is the shape that must
    // still say where to look.
    // 第三个参数是这次回复自己的那些行：自 T-22 起这条提示由它们实例化，因此空切片就是"没有 `why` 行"的
    // 回复所携带的形状——而正是它仍须说出下一步去哪找。
    let empty = routed(1, &[], None);
    let failing =
        next_step("default", false, Some(101), &empty).expect("a failing run has a next step");
    assert!(failing.contains("search {literal"), "{failing}");
    assert!(
        next_step("default", false, Some(0), &empty).is_none(),
        "a passing run needs no pointer"
    );
    assert!(
        next_step("default", true, None, &empty).is_none(),
        "a timeout is unknown, not a failure to chase"
    );
}

/// The failing assertion's own words ride with the failing test's name.
/// 失败断言自己的话与失败测试的名字同行给出。
///
/// Measured in W8's h1 (step 5): the reply carried `verdict failed` and `failed <name>`, and the arm
/// then read the raw log itself ("Let's read the check log raw") to get the assertion — one call and
/// one `sed` spent on a line we had already read. The log below is that one, in cargo's own shape;
/// it is written rather than shelled out because the panic line contains a quote that no `sh -c`
/// quoting survives intact.
/// W8 的 h1 第 5 步量到：回复带了 `verdict failed` 与 `failed <name>`，那一臂随后自己去读原始日志
/// （「Let's read the check log raw」）拿断言——一次调用加一次 `sed`，花在我们早已读过的那一行上。下面这份
/// 日志就是那一份，按 cargo 自己的形状；直接写盘而不是走 shell，因为那行 panic 里的引号在任何 `sh -c`
/// 的引法下都活不下来。
#[test]
fn a_failing_assertion_rides_with_its_test_name() {
    let root = scratch("why");
    let log = root.join("check-default.log");
    std::fs::write(
        &log,
        "test result: FAILED. 0 passed; 1 failed; 0 ignored\n\
         test the_rendered_offsets_add_up ... FAILED\n\
         \n\
         failures:\n\
         \n\
         ---- the_rendered_offsets_add_up stdout ----\n\
         thread 'main' panicked at tests/offsets.rs:19:5:\n\
         assertion `left == right` failed\n\
         \x20 left: 24\n\
         \x20 right: 31\n\
         note: run with `RUST_BACKTRACE=1` for a backtrace\n",
    )
    .expect("the fixture log");
    let observed = observation(&log, true).expect("the log reads");
    let joined = observed.lines.join("\n");
    assert!(
        joined.contains("failed the_rendered_offsets_add_up"),
        "the name is still reported: {joined}"
    );
    assert!(
        joined.contains(
            "why    the_rendered_offsets_add_up: thread 'main' panicked at tests/offsets.rs:19:5: \
             assertion `left == right` failed"
        ),
        "and the assertion's own words ride with it, capped at two lines: {joined}"
    );
    assert!(
        !joined.contains("RUST_BACKTRACE") && !joined.contains("left: 24"),
        "the log's advice and the diff below the message are not part of it: {joined}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// A run with nothing failing gets no `why` lines: the field is the assertion's, not a template.
/// 没有失败项的运行不产生 `why` 行：那一栏属于断言，不是模板。
#[test]
fn a_green_run_carries_no_assertion_words() {
    let root = scratch("why-green");
    let log = root.join("check-default.log");
    std::fs::write(&log, "test result: ok. 3 passed; 0 failed; 0 ignored\n").expect("log");
    let observed = observation(&log, true).expect("the log reads");
    assert!(
        !observed.lines.iter().any(|line| line.starts_with("why")),
        "{:?}",
        observed.lines
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// The next hint carries a literal taken out of **this reply's own** `why` line, and what is
/// stripped from it is exactly the three parts that would match nothing.
/// 这条提示携带的字面量取自**这次回复自己的** `why` 行，而从它身上剥掉的正是那三种"匹配不到任何东西"的部分。
///
/// Measured (W8): `next` lines were followed 28% of the time, and the only one followed reliably
/// named a concrete thing to read. A placeholder (`<that phrase>`) leaves the filling-in to the
/// caller — which is the thinking the hint exists to remove.
/// 实测（W8）：`next` 行被采纳 28%，而唯一被稳定采纳的是点名了"具体要读什么"的那条。占位符
/// （`<that phrase>`）把填空留给调用方——而填空正是这条提示存在来省掉的思考。
#[test]
fn the_next_hint_carries_a_searchable_literal_from_this_reply() {
    // The harness prefix carries a process id; the generic clause is shared by many tests; the
    // numbers are values. None of the three is in any source file.
    // harness 前缀带进程号；通用从句被许多测试共用；数字是取值。三者都不在任何源码里。
    let lines = vec![
        "why    the_rendered_offsets_add_up: thread 'the_rendered_offsets_add_up' (100729) \
         panicked at tests/offsets.rs:18:5: assertion `left == right` failed: the rendered offsets \
         add up to 160, not 136"
            .to_owned(),
    ];
    let phrase = super::routing::literal_to_search(&lines).expect("the message yields a phrase");
    assert_eq!(
        phrase, "the rendered offsets add up to",
        "the process id, the generic clause and the numbers are all gone"
    );
    let observed = Observation {
        skipped: 0,
        lines: lines.clone(),
        results: 1,
        executed: 1,
        failed: Vec::new(),
        targets: std::collections::BTreeMap::new(),
        target_labels: std::collections::BTreeMap::new(),
        incomplete: false,
        first_location: None,
    };
    let next = super::next_step("default", false, Some(101), &observed)
        .expect("a failing run has a next step");
    assert!(
        next.contains("`search {literal: \"the rendered offsets add up to\"}`"),
        "and the hint carries it as a copyable call: {next}"
    );
    assert!(
        !next.contains("<that phrase>"),
        "no placeholder is left when the reply had the words: {next}"
    );
}

/// With nothing to take a phrase from, the hint falls back to the placeholder rather than inventing.
/// 无从取词时，提示落回占位符，而不是编一句。
#[test]
fn the_next_hint_falls_back_to_a_placeholder_rather_than_inventing_one() {
    let lines = vec!["result test result: FAILED. 1 passed; 1 failed".to_owned()];
    assert!(super::routing::literal_to_search(&lines).is_none());
    let observed = Observation {
        skipped: 0,
        lines: lines.clone(),
        results: 1,
        executed: 1,
        failed: Vec::new(),
        targets: std::collections::BTreeMap::new(),
        target_labels: std::collections::BTreeMap::new(),
        incomplete: false,
        first_location: None,
    };
    let next = super::next_step("default", false, Some(101), &observed).expect("a next step");
    assert!(
        next.contains("<a short phrase from the assertion>"),
        "{next}"
    );
}

/// Every column head survives the sample, with its count.
/// 每一栏的栏头都活过抽样，连同它的计数。
///
/// Measured (round 8, g4): the default sample was "the first five lines", and on that tree the five
/// were all the numeric-constant column — so the answer never showed `branch-level:` at all, while
/// that level's whole question is the branch column. A column head carries its own count, which is
/// what tells a reader whether the part they did not get could matter.
/// 量到的（第八轮 g4）：默认抽样是"最前面五行"，而在那棵树上那五行全属数值常量栏 ⇒ 答案里**根本看不到**
/// `branch-level:`，而那一关的全部问题就是分支栏。栏头自带计数，正是它告诉读者"没拿到的部分会不会要紧"。
#[test]
fn every_column_head_survives_the_census_sample() {
    let census: Vec<String> = vec![
        "  constants: 1 named numeric constant(s)".to_owned(),
        "  respelled 1000 is written again at src/a.rs:3".to_owned(),
        "  respelled 1000 is written again at src/b.rs:4".to_owned(),
        "  respelled 1000 is written again at src/c.rs:4".to_owned(),
        "  respelled 1000 is written again at src/d.rs:4".to_owned(),
        "  respelled 1000 is written again at src/e.rs:4".to_owned(),
        "  respelled 1000 is written again at src/f.rs:4".to_owned(),
        "  respelled 1000 is written again at src/g.rs:4".to_owned(),
        "  respelled 1000 is written again at src/h.rs:4".to_owned(),
        "  unreferenced `LIMIT` is not read anywhere outside tests".to_owned(),
        "  entry plan: 0 `cut(` site(s)".to_owned(),
        "  declarations: 4 production `pub fn` name(s) appear in no test file".to_owned(),
        "  decl   no test names `a` (src/a.rs:1)".to_owned(),
        "  test-reachable: 1 of 15 production function(s) no test can reach".to_owned(),
        "  branch-level: 2 constructively unreachable arm(s) in this tree".to_owned(),
        "  `if false` guards an arm in `word` at src/a.rs:9".to_owned(),
        "  not covered by the branch-level column: a data-dependent condition".to_owned(),
        "  not covered: runtime behaviour and performance".to_owned(),
    ];
    let sample = super::census_sample(&census);
    for line in &census {
        let head = line
            .split_whitespace()
            .next()
            .is_some_and(|word| word.ends_with(':'));
        if head {
            assert!(
                sample.contains(line),
                "每一栏的头都在：{line}\n--- sample ---\n{}",
                sample.join("\n")
            );
        }
    }
    assert!(
        sample.iter().any(|line| line.contains("withheld")),
        "扣掉的行要说出来：{}",
        sample.join("\n")
    );
    assert!(
        sample.len() <= super::census::CENSUS_SAMPLE + census.len(),
        "样本仍然有界：{}",
        sample.len()
    );
}

/// The counter-proof, on the same shape: nothing withheld, no truncation line.
/// 反证（同一形状）：没有扣掉任何行时，不出现截断行。
#[test]
fn a_census_that_fits_says_nothing_about_withholding() {
    let census: Vec<String> = vec![
        "  branch-level: 1 constructively unreachable arm(s) in this tree".to_owned(),
        "  `if false` guards an arm in `word` at src/a.rs:9".to_owned(),
        "  not covered: runtime behaviour".to_owned(),
    ];
    let sample = super::census_sample(&census);
    assert_eq!(sample, census, "装得下就原样给出");
    assert!(
        !sample.iter().any(|line| line.contains("withheld")),
        "没有截断就不许提截断：{}",
        sample.join("\n")
    );
}

/// Known defect (audit h4), remainder pinned as it behaves today on a real census: the g4 fix
/// keeps every column **head** in the default sample (pinned above), but a column's judgement
/// **rows** still lose to the global first-five take — so when the constants column fills the head
/// of the census, the one branch row that names the dead arm is withheld unnamed, and only the
/// head's count survives.
/// The h4 shape itself: the flat take's first five lines all belong to one column, and a column
/// far below still has to arrive with **its own** row — not with the row of whichever column
/// happens to sit under its head.
/// h4 那个形状本身：平取的前五行全属同一栏，而远在下面的一栏仍必须带着**它自己的**那一行到达——而不是
/// 带着恰好排在它栏头下面的那一栏的行。
#[test]
fn a_column_far_down_the_census_still_arrives_with_a_row_of_its_own() {
    let mut census = vec!["  constants: 9 named numeric constant(s)".to_owned()];
    for index in 0..9 {
        census.push(format!(
            "  respelled 1000 is written again at src/k{index}.rs:1"
        ));
    }
    census.push("  branch-level: 1 constructively unreachable arm(s) in this tree".to_owned());
    census.push("  `if false` guards an arm in `clamp` at src/button/button.rs:21".to_owned());
    let sample = super::census_sample(&census).join("\n");
    assert!(
        sample.contains("branch-level: 1 constructively unreachable arm(s)"),
        "栏头到达：{sample}"
    );
    assert!(
        sample.contains("guards an arm in `clamp`"),
        "每一栏都要带**它自己**的一行：{sample}"
    );
    assert!(
        !sample.contains("respelled 1000 is written again at src/k8.rs:1")
            || sample.contains("census rows withheld"),
        "扣下的行要说出来：{sample}"
    );
}

/// 每栏的栏头**和它下面的一行**都要活过默认抽样：曾经那条"取前五行"会把整整一栏的判据行匿名扣下——当
/// 数值常量栏占满总账前部时，唯一那行点名死臂的分支行输给了它，只剩栏头的计数 ⇒ "逐栏处置"这份回复里
/// 根本做不到（审计 h4 登记的就是这一条，上一版钉子钉的是坏形状）。
#[test]
fn the_sample_keeps_the_branch_row_that_names_the_dead_arm() {
    let root = scratch("census-branch-row");
    std::fs::create_dir_all(root.join("src")).expect("fixture dirs");
    std::fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"fixture-census-sample\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .expect("fixture manifest");
    std::fs::write(
        root.join("src/lib.rs"),
        "pub const LIMIT: i64 = 1000;\n\
         pub fn word(amount: i64) -> i64 {\n\
         \x20   if false {\n\
         \x20       return 0;\n\
         \x20   }\n\
         \x20   amount\n\
         }\n",
    )
    .expect("fixture source");
    for name in ["a", "b", "c", "d", "e"] {
        std::fs::write(
            root.join(format!("src/{name}.rs")),
            format!("pub fn {name}() -> i64 {{ 1000 }}\n"),
        )
        .expect("fixture source");
    }
    let census = crate::mcp::claims::census(&root, false).expect("the census answers");
    let arm = census
        .iter()
        .find(|line| line.contains("`if false` guards an arm"))
        .unwrap_or_else(|| {
            panic!(
                "the fixture must produce a branch row: {}",
                census.join("\n")
            )
        })
        .clone();
    let sample = super::census_sample(&census).join("\n");
    assert!(
        sample.contains("branch-level: 1 constructively unreachable arm(s)"),
        "the head survives, with its count: {sample}"
    );
    assert!(
        sample.contains(&arm),
        "the row that names the arm arrives with its head: {sample}"
    );
    // And the truncation outlet, when it is used at all, still names how much was cut.
    // 而截断出口在被用到时，照样点名扣掉了多少。
    assert!(
        sample.contains("census rows withheld") || !sample.contains("withheld"),
        "the sample never cuts without saying so: {sample}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// The groups that passed are a count and the first line; `verbose` prints them all.
/// 通过的那些组折成一个计数加第一行；`verbose` 把它们全印出来。
///
/// Audit `W1-3`: a run has one `test result:` line per test binary, and on a wide tree those lines
/// are most of the reply's tail. The failing ones are the answer and stay line by line; the passing
/// ones fold, and the fold names the switch that undoes it — so the reader is never left guessing
/// whether a group is missing or merely folded.
/// 审计 `W1-3`：一次运行每个测试二进制品一行 `test result:`，而在宽树上这些行就是回复尾部的大半。带失败的
/// 那些是答案、逐行留着；通过的那些折叠，而折叠点名撤销它的开关——因此读者从不必猜是少了一组还是只是被折了。
#[test]
fn the_passing_groups_fold_and_verbose_undoes_it() {
    let root = std::env::temp_dir().join(format!("xirang-check-fold-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("scratch");
    let log = root.join("check-default.log");
    std::fs::write(
        &log,
        "running 3 tests\n\
         test result: ok. 3 passed; 0 failed; 0 ignored\n\
         running 2 tests\n\
         test result: ok. 2 passed; 0 failed; 0 ignored\n\
         running 1 test\n\
         test the_defect ... FAILED\n\
         test result: FAILED. 0 passed; 1 failed; 0 ignored\n",
    )
    .expect("the log");

    let folded = observation(&log, false).expect("the log reads");
    let text = folded.lines.join("\n");
    assert_eq!(folded.results, 3, "three groups ran: {text}");
    assert!(
        text.contains("test result: FAILED"),
        "the failing group is printed line by line: {text}"
    );
    assert!(
        text.contains("2 group(s) passed")
            && text.contains("pass `verbose: true`")
            && text.contains("first: test result: ok. 3 passed"),
        "the passing groups fold into a count and the first line, with the way back: {text}"
    );

    let whole = observation(&log, true).expect("the log reads");
    let text = whole.lines.join("\n");
    assert!(
        text.contains("test result: ok. 3 passed") && text.contains("test result: ok. 2 passed"),
        "`verbose` prints every group: {text}"
    );
    assert!(
        !text.contains("group(s) passed —"),
        "and nothing is folded when it does: {text}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// A build red routes to the build's own answer, and a family red routes to the comparison (audit `W5-5`).
/// 构建的红路由到构建自己的答案，同族的红路由到比对（审计 `W5-5`）。
///
/// The acceptance the checklist asks for is a battery whose step count stops varying, and that only
/// happens if the first call after a red is the right **kind** of call. Two kinds are pinned here
/// because the bridge already has a purpose-built tool for each: a red with no `test result:` line is
/// the build's (route to `why` at the location the compiler named), and a red whose failing test
/// mentions coordinates is the family's (route to `consistency` for that face's parent).
/// 清单要的验收是一套步数不再抖动的电池，而那只在"红之后第一次调用是对的**那种**调用"时才发生。
/// 这里钉两种，因为本桥对每一种都已经有专用工具：没有 `test result:` 行的红是构建的（路由到编译器点名那个
/// 位置的 `why`），失败测试提到坐标的红是同族的（路由到那个面的父级的 `consistency`）。
#[test]
fn a_red_routes_by_its_kind_rather_than_to_the_default_hint() {
    // The build never produced a test binary: zero result lines, and rustc named a location.
    // 构建从未产出测试二进制：零条结果行，而 rustc 点名了一个位置。
    let build = routed(0, &[], Some("src/control/object/slider/slider.rs:18:5"));
    let next = super::next_step("default", false, Some(101), &build).expect("a next step");
    assert!(
        next.contains("**nothing ran**")
            && next.contains("why {at: \"src/control/object/slider/slider.rs:18:5\"}"),
        "the build red routes to the location the compiler named: {next}"
    );
    assert!(
        !next.contains("search {literal"),
        "and not to the assertion hunt, which has no assertion to hunt for: {next}"
    );

    // A family red: the failing test is about the coordinates the siblings must agree on.
    // 同族的红：失败的测试是关于兄弟之间必须一致的坐标。
    // The family's own noun, in the reply's `why` line, is what carries the signal — a failing test
    // whose *name* happens to mention offsets is not a coordinates red, and must not be routed as one.
    // 携带信号的是这个族自己的名词、出现在回复的 `why` 行里——一个名字里凑巧提到 offsets 的失败测试不是
    // 坐标的红，也不该按它路由。
    let family = routed_with(
        1,
        &["the_rendered_offsets_add_up"],
        Some("tests/offsets.rs:18:5"),
        &[&format!(
            "why    the_rendered_offsets_add_up: assertion failed: {} (160, not 136)",
            xirang_kernel::RuntimeCheckSpec::CoordinatesInViewport.name()
        )],
    );
    let next = super::next_step("root/control/object/slider", false, Some(101), &family)
        .expect("a next step");
    assert!(
        next.contains("coordinates family")
            && next.contains("consistency {parent: \"root/control/object\"}"),
        "the family red routes to the comparison of the face's own parent: {next}"
    );

    // And the default kind keeps the hint that was measured to be followed.
    // 而默认那一种保留那条实测会被采纳的提示。
    let assertion = routed(1, &["some_other_test"], Some("tests/other.rs:3:5"));
    let next = super::next_step("default", false, Some(101), &assertion).expect("a next step");
    assert!(next.contains("search {literal"), "{next}");
}

/// Selection must change actual Cargo execution; unmatched filters are never a green repair.
/// 选择参数必须改变真实 Cargo 执行；无匹配过滤器绝不是修复成功。
#[test]
fn package_target_filter_and_disabled_defaults_control_the_observed_run() {
    let root = scratch("target-selection");
    std::fs::write(
        root.join("Cargo.toml"),
        "[workspace]\nmembers=['a','b']\ndefault-members=['a']\nresolver='2'\n",
    )
    .unwrap();
    for name in ["a", "b"] {
        std::fs::create_dir_all(root.join(name).join("src")).unwrap();
        std::fs::create_dir_all(root.join(name).join("tests")).unwrap();
        std::fs::write(root.join(name).join("Cargo.toml"), format!("[package]\nname='{name}'\nversion='0.0.0'\nedition='2024'\n[features]\ndefault=['broken']\nbroken=[]\n")).unwrap();
        std::fs::write(root.join(name).join("src/lib.rs"), "// fixture\n").unwrap();
        let body = if name == "a" {
            "#[test]\nfn passed() {}\n"
        } else {
            "#[test]\nfn feature_failure() { assert!(!cfg!(feature=\"broken\")); }\n"
        };
        std::fs::write(root.join(name).join("tests/selected.rs"), body).unwrap();
    }
    for (request, verdict) in [
        (
            serde_json::json!({"census":false,"target_dir":"build-output"}),
            "passed",
        ),
        (
            serde_json::json!({"workspace":true,"census":false}),
            "failed",
        ),
        (
            serde_json::json!({"package":"b","test":"selected","census":false}),
            "failed",
        ),
        (
            serde_json::json!({"package":"b","test":"selected","no_default_features":true,"census":false}),
            "passed",
        ),
        (
            serde_json::json!({"package":"b","filter":"no_matching_test","census":false}),
            "unknown",
        ),
    ] {
        let reply = super::check(&root, &request).unwrap();
        assert!(
            reply.starts_with(&format!("verdict  {verdict}")),
            "{request}: {reply}"
        );
        if request.get("target_dir").is_some() {
            assert!(
                std::fs::read_dir(root.join("build-output"))
                    .unwrap()
                    .any(|entry| entry
                        .unwrap()
                        .file_name()
                        .to_string_lossy()
                        .starts_with("check-default-")),
                "{reply}"
            );
            // The other half of the same promise: a caller who named a target directory gets no
            // `target/` inside the tree. The 2026-10-10 queue item ④ recorded the opposite for the
            // `all` face (+88 MB in-tree); re-measured on 2026-10-13 through the bridge, both faces
            // put every byte in the named directory — including on the partitioned example, whose
            // generated crates could have built themselves (report §7h). A build that ignores the
            // caller's target dir turns this red.
            // 同一条承诺的另一半：指定了 target 目录的调用方，树里不会长出 `target/`。2026-10-10 队列
            // 第 ④ 条记的正是相反的情形（`all` 面在树内 +88 MB）；2026-10-13 用真桥重测，两个面都把每
            // 一个字节放进了指定的目录——包括那个带生成包的分区示例（报告 §7h）。忽略调用方 target 目录的
            // 构建会让这一条变红。
            assert!(
                !root.join("target").exists(),
                "a check given a target_dir leaves no in-tree target/: {reply}"
            );
        }
        assert!(reply.contains("census skipped"), "{reply}");
        assert!(
            !reply.contains("rust file(s)"),
            "disabled analysis: {reply}"
        );
        assert!(!reply.contains("named numeric constant"), "{reply}");
    }
    std::fs::write(
        root.join("a/src/lib.rs"),
        "pub fn broken() -> u32 { \"wrong\" }\n",
    )
    .unwrap();
    let reply = super::check(&root, &serde_json::json!({"package":"a","census":false})).unwrap();
    assert!(
        reply.contains("E0308") && reply.contains("mismatched types"),
        "{reply}"
    );
    std::fs::remove_dir_all(root).unwrap();
}

/// A09/A27 end to end: a log whose `Running` headers were dropped by the presentation
/// budget still attributes each `shared` row to its own binary, so a scoped counterproof
/// that would have been certified by the wrong package's failure stays unknown.
/// A09/A27 端到端：一份 `Running` 头被展示预算丢掉的日志，仍把每条 `shared` 行归属到各自的二进制，因此
/// 一个本会被另一个包的失败背书的限定反证保持 unknown。
#[test]
fn a_truncated_log_still_refuses_to_certify_a_duplicated_name() {
    let root = scratch("truncated-attribution");
    let log = root.join("check-truncated.log");
    let mut text = String::new();
    for index in 0..5_000 {
        text.push_str(&format!("filler line {index}\n"));
    }
    text.push_str(
        "     Running unittests src/lib.rs (target/debug/deps/a_proof-1111111111111111)\n\
         running 1 test\n\
         test contract ... ok\n\
         test result: ok. 1 passed; 0 failed\n\
         \x20    Running unittests src/lib.rs (target/debug/deps/b_proof-2222222222222222)\n\
         running 1 test\n\
         test contract ... FAILED\n\
         ---- contract stdout ----\n\
         thread 'contract' panicked at b/src/lib.rs:2:17: EXPECTED_ROUTE_VIOLATION\n\
         test result: FAILED. 0 passed; 1 failed\n",
    );
    std::fs::write(&log, &text).expect("log");
    let observed = observation(&log, false).expect("the log reads");
    assert!(
        !observed.lines.join("\n").contains("Running unittests"),
        "the budget really dropped the headers, so this pin is about the abandoned shape"
    );
    let failing = outcome_of(Some(101), false, observed);
    assert!(
        failing.name_is_ambiguous("contract", Some("src/lib.rs")),
        "the shared source label matches both packages, so the selector resolves to two instances"
    );
    assert!(
        !failing.failed_on("contract", Some("src/lib.rs"), "EXPECTED_ROUTE_VIOLATION"),
        "a failure in one package cannot certify a selector that also names the other"
    );
    let _ = std::fs::remove_dir_all(root);
}

/// The same judgement on a stream whose attribution is unsupported: one row ran without a
/// `Running` header, so the evidence cannot claim the name is unique at all.
/// 同一条判定用在归属不受支持的流上：有一条测试行前面没有 `Running` 头，因此证据根本无法声称名字唯一。
#[test]
fn a_headerless_row_leaves_the_target_evidence_incomplete() {
    let root = scratch("headerless");
    let log = root.join("check-headerless.log");
    std::fs::write(
        &log,
        "test contract ... FAILED\n\
         ---- contract stdout ----\n\
         thread 'contract' panicked at b/src/lib.rs:2:17: EXPECTED_ROUTE_VIOLATION\n\
         test result: FAILED. 0 passed; 1 failed\n",
    )
    .expect("log");
    let observed = observation(&log, false).expect("the log reads");
    let failing = outcome_of(Some(101), false, observed);
    assert!(
        failing.target_evidence_incomplete(),
        "the stream said so, and the judge reads that field"
    );
    assert!(
        !failing.failed_on("contract", None, "EXPECTED_ROUTE_VIOLATION"),
        "an unattributed row is not evidence that this name failed"
    );
    let _ = std::fs::remove_dir_all(root);
}
