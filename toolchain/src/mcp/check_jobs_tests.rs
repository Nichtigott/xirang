//! Exercise actual Cargo modes, asynchronous cancellation and immutable logs.
//! 验证真实 Cargo 模式、异步取消与不可变日志。
use super::*;
use serde_json::json;
use std::time::{Duration, Instant};
fn fixture(label: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!("xirang-jobs-{label}-{}", new_id()));
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::write(
        root.join("Cargo.toml"),
        "[package]\nname='jobs-probe'\nversion='0.0.0'\nedition='2024'\n[workspace]\n",
    )
    .unwrap();
    std::fs::write(root.join("src/lib.rs"), "#[test]\nfn passes() {}\n").unwrap();
    root
}
#[test]
fn actual_compilation_modes_and_logs_are_distinct_from_test_evidence() {
    let root = fixture("modes");
    let mut logs = Vec::new();
    for mode in ["check", "build", "clippy", "test"] {
        let reply = check(&root, &json!({"mode":mode,"census":false})).unwrap();
        assert!(reply.starts_with("verdict  passed"), "{reply}");
        assert!(
            reply.contains(&format!("check  cargo \"{mode}\"")),
            "{reply}"
        );
        assert_eq!(
            reply.contains("no behavioral tests executed"),
            mode != "test"
        );
        let log = reply
            .lines()
            .find_map(|line| line.strip_prefix("log    "))
            .unwrap()
            .to_owned();
        assert!(!logs.contains(&log));
        logs.push(log);
    }
    assert!(logs.iter().all(|log| Path::new(log).is_file()));
    for args in [
        json!({"mode":"check","filter":"passes"}),
        json!({"mode":"oops"}),
        json!({"mode":"build","deny_warnings":true}),
        json!({"target":false}),
    ] {
        assert!(check(&root, &args).is_err());
    }
    std::fs::remove_dir_all(root).unwrap();
}
#[test]
fn cancellation_reaches_cargo_test_descendants_and_returns_promptly() {
    let _session = session();
    let root = fixture("cancel");
    std::fs::write(root.join("src/lib.rs"), "#[test]\nfn slow() { std::fs::write(\"started\", \"yes\").unwrap(); std::thread::sleep(std::time::Duration::from_secs(30)); std::fs::write(\"escaped\", \"bad\").unwrap(); }\n").unwrap();
    let started = Instant::now();
    let reply = check(&root, &json!({"action":"start","census":false})).unwrap();
    assert!(started.elapsed() < Duration::from_secs(1));
    let id = reply.lines().next().unwrap().strip_prefix("job ").unwrap();
    while !root.join("started").exists() {
        assert!(started.elapsed() < Duration::from_secs(30));
        std::thread::sleep(Duration::from_millis(50));
    }
    assert!(
        check(&root, &json!({"action":"cancel","job":id}))
            .unwrap()
            .contains("cancelling")
    );
    loop {
        let status = check(&root, &json!({"action":"status","job":id})).unwrap();
        if status.contains("finished") {
            break;
        }
        assert!(started.elapsed() < Duration::from_secs(35));
        std::thread::sleep(Duration::from_millis(50));
    }
    let result = check(&root, &json!({"action":"result","job":id})).unwrap();
    assert!(result.starts_with("verdict  unknown"), "{result}");
    assert!(!root.join("escaped").exists());
    std::fs::remove_dir_all(root).unwrap();
}
#[test]
#[cfg(unix)]
fn timeout_kills_the_process_group_not_only_the_parent() {
    let root = fixture("group");
    let pidfile = root.join("child.pid");
    let mut command = std::process::Command::new("sh");
    command
        .arg("-c")
        .arg("sleep 30 & echo $! > child.pid; wait")
        .current_dir(&root);
    let outcome = crate::mcp::check::run_command(
        &mut command,
        Duration::from_millis(300),
        &root.join("output.log"),
    )
    .unwrap();
    assert!(outcome.timed_out);
    let pid = std::fs::read_to_string(pidfile).unwrap();
    // SIGKILL delivery is asynchronous; reaping the parent does not schedule its child.
    // SIGKILL 异步送达；回收父进程不保证子进程已获得调度并退出。
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        let status =
            std::fs::read_to_string(format!("/proc/{}/stat", pid.trim())).unwrap_or_default();
        if status.is_empty() || status.split_whitespace().nth(2) == Some("Z") {
            break;
        }
        assert!(Instant::now() < deadline, "descendant survived: {status}");
        std::thread::sleep(Duration::from_millis(10));
    }
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn one_shot_cannot_orphan_a_job() {
    let root = fixture("one-shot");
    let error = check(&root, &json!({"action":"start"})).unwrap_err();
    assert!(error.contains("persistent MCP"), "{error}");
    assert!(!root.join("target").exists());
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn closing_a_session_cancels_its_worker_and_keeps_other_sessions_separate() {
    let root = fixture("eof");
    std::fs::write(root.join("src/lib.rs"), "#[test]\nfn slow() { std::fs::write(\"started\", \"yes\").unwrap(); std::thread::sleep(std::time::Duration::from_secs(30)); }\n").unwrap();
    let owner;
    let id;
    {
        let connection = session();
        owner = connection.owner;
        let reply = check(&root, &json!({"action":"start","census":false})).unwrap();
        id = reply
            .lines()
            .next()
            .unwrap()
            .strip_prefix("job ")
            .unwrap()
            .to_owned();
        let start = Instant::now();
        while !root.join("started").exists() {
            assert!(start.elapsed() < Duration::from_secs(30));
            std::thread::sleep(Duration::from_millis(50));
        }
        assert!(
            check(&root, &json!({"action":"list"}))
                .unwrap()
                .contains(&id)
        );
    }
    let jobs = JOBS
        .get()
        .unwrap()
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let job = &jobs[&id];
    assert_eq!(job.owner, owner);
    assert!(job.cancel.load(Ordering::Acquire));
    assert!(
        job.result
            .as_ref()
            .unwrap()
            .as_ref()
            .unwrap()
            .starts_with("verdict  unknown")
    );
    drop(jobs);
    let _other = session();
    assert!(check(&root, &json!({"action":"status","job":id})).is_err());
    std::fs::remove_dir_all(root).unwrap();
}
