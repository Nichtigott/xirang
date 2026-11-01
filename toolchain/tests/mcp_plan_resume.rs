//! ME06: a plan is a durable record, not a process's memory.
//!
//! `/tmp`-style fixture, real binary, two processes: the first seals a plan, a second one
//! (a different process, therefore a different session) resumes it. When the bytes changed in
//! between, the earlier pass is stale and the obligation runs again; when they did not, the
//! pass is kept. Nothing here reads the first process's memory: every value the second process
//! uses is on disk under the fixture's `target/`.
//!
//! ME06：计划是持久记录，不是一个进程的记忆。真二进制、两个进程：第一个封存计划，第二个
//! （另一个进程，也就是另一个会话）恢复它。中间改过字节，旧通过就是过期的、义务要重跑；
//! 没改过就保留。这里没有一处读第一个进程的记忆：第二个进程用到的每个值都在夹具的
//! `target/` 下。

#![cfg(feature = "mcp")]

use std::path::{Path, PathBuf};
use std::process::Command;

fn binary() -> &'static str {
    env!("CARGO_BIN_EXE_xirang-mcp")
}

fn fixture(label: &str) -> PathBuf {
    let root =
        std::env::temp_dir().join(format!("xirang-plan-resume-{label}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"plan-resume\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[workspace]\n",
    )
    .unwrap();
    std::fs::write(root.join("README.md"), "fixture\n").unwrap();
    std::fs::write(root.join("src/lib.rs"), SOURCE).unwrap();
    // A plan phase starts no Cargo, and it needs a resolved graph to name the obligations; the
    // fixture resolves it the way a maintainer would before asking for a plan.
    // 计划阶段不启动 Cargo，而它需要一个已解析的依赖图来点名义务；夹具像维护者那样先自己解析。
    let status = Command::new("cargo")
        .arg("generate-lockfile")
        .arg("--offline")
        .current_dir(&root)
        .status()
        .unwrap();
    assert!(status.success(), "the fixture could not resolve its graph");
    root
}

const SOURCE: &str = "pub fn add(a: i64, b: i64) -> i64 { a + b }\n\n#[cfg(test)]\nmod tests {\n    #[test]\n    fn adds() {\n        assert_eq!(super::add(1, 2), 3);\n    }\n}\n";

/// One summon of the binary: a separate process, exactly like a session that starts later.
/// 二进制的每一次召唤：独立进程，与一个之后才开始的会话完全一样。
fn call(root: &Path, request: &str) -> (i32, String) {
    let output = Command::new(binary())
        .arg("--call")
        .arg("xirang.check")
        .arg("--root")
        .arg(root)
        .arg("--json")
        .arg(request)
        .output()
        .unwrap();
    // A refusal goes to stderr; the assertion below must be able to read whichever stream the
    // binary chose, because "it said nothing on stdout" is exactly how a silence gets mistaken for
    // a pass.
    // 拒绝写在 stderr；下面的断言必须能读到二进制选的那条流——"stdout 什么都没说"正是沉默被当成
    // 通过的方式。
    let mut reply = String::from_utf8_lossy(&output.stdout).into_owned();
    reply.push_str(&String::from_utf8_lossy(&output.stderr));
    (output.status.code().unwrap_or(-1), reply)
}

fn plan_id(reply: &str) -> String {
    let first = reply.lines().next().unwrap_or_default();
    first
        .strip_prefix("plan ")
        .and_then(|rest| rest.split_whitespace().next())
        .unwrap_or_else(|| panic!("no plan id in {reply}"))
        .to_owned()
}

#[test]
fn a_second_process_resumes_the_plan_and_refuses_a_pass_from_other_bytes() {
    let root = fixture("refuses");
    let (code, sealed) = call(
        &root,
        r#"{"files":["src/lib.rs"],"face":"default","action":"plan"}"#,
    );
    assert_eq!(code, 0, "{sealed}");
    let id = plan_id(&sealed);
    assert!(
        sealed.contains("no cargo test was started"),
        "a plan must not run anything: {sealed}"
    );
    // The record is a file, so a process that never saw the first one can read it.
    // 记录是一个文件，因此从没见过第一个进程的进程也能读它。
    let ledger = root
        .join("target/xirang/verification")
        .join(format!("{id}.json"));
    assert!(ledger.exists(), "the plan was not written to disk");

    std::fs::write(root.join("src/lib.rs"), SOURCE.replace("a + b", "a - b")).unwrap();
    let (code, resumed) = call(
        &root,
        &format!(r#"{{"plan_id":"{id}","action":"resume","face":"default"}}"#),
    );
    assert_eq!(code, 0, "{resumed}");
    assert!(
        resumed.contains("1 obligation(s) re-run, 0 kept") && resumed.contains("inputs changed"),
        "a different process must not reuse the sealed plan's bytes: {resumed}"
    );
    assert!(
        resumed.contains("modified src/lib.rs"),
        "the refusal names what moved: {resumed}"
    );
    assert!(
        resumed.contains("failed (expected passed)"),
        "the changed bytes really ran and really failed: {resumed}"
    );

    std::fs::write(root.join("src/lib.rs"), SOURCE).unwrap();
    let (code, restored) = call(
        &root,
        &format!(r#"{{"plan_id":"{id}","action":"resume","face":"default"}}"#),
    );
    assert_eq!(code, 0, "{restored}");
    assert!(
        restored.contains("passed (expected passed)"),
        "the restored bytes pass again: {restored}"
    );

    // Same bytes, another process: the pass is kept instead of being thrown away, which is the
    // other half of ME06 — a later session does not have to re-do work whose inputs are intact.
    // 同一批字节、另一个进程：通过被保留而不是丢掉，这是 ME06 的另一半——后来的会话不必重做
    // 输入未变的工作。
    let (code, kept) = call(
        &root,
        &format!(r#"{{"plan_id":"{id}","action":"resume","face":"default"}}"#),
    );
    assert_eq!(code, 0, "{kept}");
    assert!(
        kept.contains("0 obligation(s) re-run, 1 kept"),
        "an unchanged closure must be reusable across sessions: {kept}"
    );

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_plan_that_vanished_from_disk_is_an_error_rather_than_a_quiet_pass() {
    // A session that inherits a plan id but not the file must not report a green it cannot see.
    // 继承了计划 id、却没有那份文件的会话，不许报一个它看不见的绿。
    let root = fixture("missing");
    let (code, sealed) = call(
        &root,
        r#"{"files":["src/lib.rs"],"face":"default","action":"plan"}"#,
    );
    assert_eq!(code, 0, "{sealed}");
    let id = plan_id(&sealed);
    std::fs::remove_file(
        root.join("target/xirang/verification")
            .join(format!("{id}.json")),
    )
    .unwrap();
    let (code, reply) = call(&root, &format!(r#"{{"plan_id":"{id}","action":"resume"}}"#));
    assert_ne!(code, 0, "a missing plan is not a pass: {reply}");
    assert!(
        reply.to_lowercase().contains("no such plan")
            || reply.to_lowercase().contains("not found")
            || reply.to_lowercase().contains("no plan"),
        "the refusal says the plan is gone: {reply}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// **EX04/EX10: a consumer that did not exist when the pass was recorded still joins the closure.**
/// **EX04/EX10：记录那次通过时还不存在的消费者，一样要进输入闭包。**
///
/// A credential is only worth what its input closure covers. A **new file that consumes the changed
/// function** is the case a file list captured at plan time would miss if the closure were "the files
/// the caller named": here it is the tree that is walked, so `src/extra.rs` appears as `added`, the
/// sealed pass is not reused, and the new consumer's test really runs (and is caught failing before
/// it is repaired). The repaired bytes then pass — so the failure was about the code, not the record.
/// 一份凭据只值它输入闭包覆盖的那些东西。**消费被改函数的全新文件**正是"计划时抓一份文件清单"会漏掉
/// 的情形——如果闭包等于"调用方点名的文件"的话。这里走的是整棵树：`src/extra.rs` 以 `added` 出现、
/// 封存的通过不被复用、新消费者的测试真的跑了（并在修复前被抓住失败）。修复后同一条计划转为通过——
/// 说明刚才的失败是关于代码的，不是关于记录的。
#[test]
fn a_file_added_after_the_plan_joins_the_closure_and_the_old_pass_is_not_reused() {
    let root = fixture("consumer");
    let (code, sealed) = call(
        &root,
        r#"{"files":["src/lib.rs"],"face":"default","action":"plan"}"#,
    );
    assert_eq!(code, 0, "{sealed}");
    let id = plan_id(&sealed);

    std::fs::write(
        root.join("src/extra.rs"),
        "pub fn twice(a: i64) -> i64 { crate::add(a, a) }\n\n#[cfg(test)]\nmod tests {\n    #[test]\n    fn consumes() {\n        assert_eq!(crate::add(1, 2), 4);\n    }\n}\n",
    )
    .unwrap();
    let mut source = SOURCE.to_owned();
    source.push_str("\nmod extra;\n");
    std::fs::write(root.join("src/lib.rs"), source).unwrap();

    let (code, resumed) = call(
        &root,
        &format!(r#"{{"plan_id":"{id}","action":"resume","face":"default"}}"#),
    );
    assert_eq!(code, 0, "{resumed}");
    assert!(
        resumed.contains("inputs changed") && resumed.contains("1 obligation(s) re-run, 0 kept"),
        "a new consumer makes the sealed pass stale: {resumed}"
    );
    assert!(
        resumed.contains("added") && resumed.contains("src/extra.rs"),
        "the new consumer is named in the change list instead of being silently outside the closure: {resumed}"
    );
    assert!(
        resumed.contains("modified src/lib.rs"),
        "the edit that wires the new file in is named too: {resumed}"
    );
    assert!(
        resumed.contains("failed (expected passed)"),
        "the new consumer's failing test really ran: {resumed}"
    );

    let fixed = std::fs::read_to_string(root.join("src/extra.rs"))
        .unwrap()
        .replace("crate::add(1, 2), 4", "crate::add(1, 2), 3");
    std::fs::write(root.join("src/extra.rs"), fixed).unwrap();
    let (code, again) = call(
        &root,
        &format!(r#"{{"plan_id":"{id}","action":"resume","face":"default"}}"#),
    );
    assert_eq!(code, 0, "{again}");
    assert!(
        again.contains("passed (expected passed)"),
        "the repaired consumer passes, so the failure was about the code: {again}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// **BR03: a credential is bound to bytes, not to a file's size or timestamp.**
/// **BR03：凭据绑定的是字节，不是文件的大小或时间戳。**
///
/// The cheapest way to make a stale pass look fresh is to change the content while restoring what a
/// metadata-based cache compares. This fixture does exactly that — an edit of the same length, with the
/// original modification time put back — and the sealed plan still refuses to reuse its pass, because the
/// closure hashes the bytes. (The other half of BR03, "resuming from a compressed session", is the same
/// property this file's first test shows: another process reads the record from disk.)
/// 让旧通过显得新鲜的最便宜办法，是在改内容的同时把"按元数据比较"的那几项恢复原样。这个夹具正是这么做的：
/// 等长改动，再把修改时间放回去——封存的计划仍然拒绝复用那次通过，因为闭包哈希的是字节。（BR03 的另一半
/// "压缩后的会话恢复"，就是本文件第一条钉子展示的同一个性质：另一个进程从磁盘读回记录。）
#[test]
fn a_same_size_edit_with_the_old_mtime_restored_still_invalidates_the_pass() {
    let root = fixture("same-size");
    let (code, sealed) = call(
        &root,
        r#"{"files":["src/lib.rs"],"face":"default","action":"plan"}"#,
    );
    assert_eq!(code, 0, "{sealed}");
    let id = plan_id(&sealed);

    let path = root.join("src/lib.rs");
    let before = std::fs::metadata(&path).unwrap().modified().unwrap();
    let text = std::fs::read_to_string(&path).unwrap();
    let edited = text.replace("a + b", "a - b");
    assert_eq!(
        edited.len(),
        text.len(),
        "the edit must not change the size"
    );
    std::fs::write(&path, &edited).unwrap();
    std::fs::File::options()
        .write(true)
        .open(&path)
        .unwrap()
        .set_modified(before)
        .unwrap();
    assert_eq!(
        std::fs::metadata(&path).unwrap().modified().unwrap(),
        before,
        "the fixture restored the modification time it is trying to fool"
    );

    let (code, resumed) = call(
        &root,
        &format!(r#"{{"plan_id":"{id}","action":"resume","face":"default"}}"#),
    );
    assert_eq!(code, 0, "{resumed}");
    assert!(
        resumed.contains("inputs changed") && resumed.contains("1 obligation(s) re-run, 0 kept"),
        "same size and same mtime is not the same bytes: {resumed}"
    );
    assert!(
        resumed.contains("modified src/lib.rs"),
        "the changed file is named even though its metadata says it did not move: {resumed}"
    );
    assert!(
        resumed.contains("failed (expected passed)"),
        "the edited bytes really ran and really failed: {resumed}"
    );
    let _ = std::fs::remove_dir_all(&root);
}
