#![cfg(feature = "cli")]
//! `xirang queue`: a persistent queue of build/test jobs. The state lives on disk, a claim is one
//! atomic rename, and a cancelled job is a job that had not started — a running or finished job is
//! named and left alone.
//! `xirang queue`：持久化的 build/test 任务队列。状态在磁盘上，领取是一次原子改名，被取消的任务
//! 是尚未开始的任务——运行中或已结束的任务只被点名，不被触碰。

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn binary() -> &'static str {
    env!("CARGO_BIN_EXE_xirang")
}

fn scratch(label: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "xirang-queue-{label}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&path).unwrap();
    path
}

fn invoke(directory: &Path, arguments: &[&str]) -> Output {
    Command::new(binary())
        .args(arguments)
        .current_dir(directory)
        .output()
        .unwrap()
}

fn both(output: &Output) -> String {
    format!(
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn unit(directory: &Path) {
    std::fs::create_dir_all(directory.join("src")).unwrap();
    std::fs::write(
        directory.join("xirang.toml"),
        "schema = 1\n\n[package]\nname = \"lib\"\nversion = \"0.1.0\"\nkind = \"library\"\n\n\
         [language.cpp]\nstandard = \"17\"\n\n[dependencies]\n",
    )
    .unwrap();
    std::fs::write(
        directory.join("src/lib.cpp"),
        "#include <cstdint>\nstd::int64_t value() { return 1; }\n",
    )
    .unwrap();
}

fn broken_unit(directory: &Path) {
    std::fs::create_dir_all(directory.join("src")).unwrap();
    std::fs::write(
        directory.join("xirang.toml"),
        "schema = 1\n\n[package]\nname = \"bad\"\nversion = \"0.1.0\"\nkind = \"library\"\n\n\
         [language.cpp]\nstandard = \"17\"\n\n[nonsense]\nx = \"y\"\n",
    )
    .unwrap();
    std::fs::write(
        directory.join("src/lib.cpp"),
        "int broken(void) { return 0; }\n",
    )
    .unwrap();
}

fn id_of(output: &str) -> String {
    let start = output.find('`').expect("a printed id") + 1;
    let rest = &output[start..];
    let end = rest.find('`').expect("a closing backtick") + start;
    output[start..end].to_owned()
}

fn worker() -> String {
    format!("{} worker", binary())
}

#[test]
fn a_queued_job_survives_the_process_and_runs_once() {
    let root = scratch("once");
    let project = root.join("lib");
    unit(&project);
    let state = root.join("queue");
    let added = invoke(
        &root,
        &["queue", "add", "lib", "--state", state.to_str().unwrap()],
    );
    assert!(added.status.success(), "{}", both(&added));
    let id = id_of(&String::from_utf8_lossy(&added.stdout));
    let listed = invoke(
        &root,
        &["queue", "list", "--state", state.to_str().unwrap()],
    );
    assert!(both(&listed).contains("queued"), "{}", both(&listed));
    let run = invoke(
        &root,
        &[
            "queue",
            "run",
            "--state",
            state.to_str().unwrap(),
            "--worker",
            &worker(),
        ],
    );
    assert!(run.status.success(), "{}", both(&run));
    assert!(both(&run).contains("1 passed"), "{}", both(&run));
    assert!(project.join("target/xirang/build").is_dir());
    let after = invoke(
        &root,
        &["queue", "list", "--state", state.to_str().unwrap()],
    );
    assert!(both(&after).contains("done"), "{}", both(&after));
    // A second run finds nothing queued: the result is the evidence the job ran once.
    let again = invoke(
        &root,
        &[
            "queue",
            "run",
            "--state",
            state.to_str().unwrap(),
            "--worker",
            &worker(),
        ],
    );
    assert!(both(&again).contains("no queued job"), "{}", both(&again));
    let shown = invoke(
        &root,
        &["queue", "show", &id, "--state", state.to_str().unwrap()],
    );
    assert!(both(&shown).contains("plan"), "{}", both(&shown));
}

#[test]
fn a_failing_job_is_recorded_and_the_good_job_still_runs() {
    let root = scratch("failed");
    unit(&root.join("good"));
    broken_unit(&root.join("bad"));
    let state = root.join("queue");
    for name in ["good", "bad"] {
        let added = invoke(
            &root,
            &["queue", "add", name, "--state", state.to_str().unwrap()],
        );
        assert!(added.status.success(), "{}", both(&added));
    }
    let run = invoke(
        &root,
        &[
            "queue",
            "run",
            "--state",
            state.to_str().unwrap(),
            "--worker",
            &worker(),
        ],
    );
    assert!(run.status.success(), "{}", both(&run));
    assert!(both(&run).contains("1 passed, 1 failed"), "{}", both(&run));
    let listed = invoke(
        &root,
        &["queue", "list", "--state", state.to_str().unwrap()],
    );
    let text = both(&listed);
    assert!(text.contains("\tdone\t"), "{text}");
    assert!(text.contains("\tfailed\t"), "{text}");
}

#[test]
fn cancel_only_works_before_a_job_starts() {
    let root = scratch("cancel");
    unit(&root.join("lib"));
    let state = root.join("queue");
    let added = invoke(
        &root,
        &["queue", "add", "lib", "--state", state.to_str().unwrap()],
    );
    let id = id_of(&String::from_utf8_lossy(&added.stdout));
    let cancelled = invoke(
        &root,
        &["queue", "cancel", &id, "--state", state.to_str().unwrap()],
    );
    assert!(cancelled.status.success(), "{}", both(&cancelled));
    assert!(
        both(&cancelled).contains("had not started"),
        "{}",
        both(&cancelled)
    );
    let again = invoke(
        &root,
        &["queue", "cancel", &id, "--state", state.to_str().unwrap()],
    );
    assert!(!again.status.success());
    assert!(
        both(&again).contains("already cancelled"),
        "{}",
        both(&again)
    );
    let run = invoke(
        &root,
        &[
            "queue",
            "run",
            "--state",
            state.to_str().unwrap(),
            "--worker",
            &worker(),
        ],
    );
    assert!(both(&run).contains("no queued job"), "{}", both(&run));

    // A finished job is named, not cancelled.
    let added = invoke(
        &root,
        &["queue", "add", "lib", "--state", state.to_str().unwrap()],
    );
    let second = id_of(&String::from_utf8_lossy(&added.stdout));
    let run = invoke(
        &root,
        &[
            "queue",
            "run",
            "--state",
            state.to_str().unwrap(),
            "--worker",
            &worker(),
        ],
    );
    assert!(run.status.success(), "{}", both(&run));
    let refused = invoke(
        &root,
        &[
            "queue",
            "cancel",
            &second,
            "--state",
            state.to_str().unwrap(),
        ],
    );
    assert!(!refused.status.success());
    assert!(
        both(&refused).contains("already finished (done)"),
        "{}",
        both(&refused)
    );
    let unknown = invoke(
        &root,
        &[
            "queue",
            "cancel",
            "000000000000",
            "--state",
            state.to_str().unwrap(),
        ],
    );
    assert!(!unknown.status.success());
    assert!(both(&unknown).contains("no job"), "{}", both(&unknown));
}

#[test]
fn two_jobs_run_with_two_workers() {
    let root = scratch("concurrent");
    unit(&root.join("one"));
    unit(&root.join("two"));
    let state = root.join("queue");
    for name in ["one", "two"] {
        let added = invoke(
            &root,
            &["queue", "add", name, "--state", state.to_str().unwrap()],
        );
        assert!(added.status.success(), "{}", both(&added));
    }
    let run = invoke(
        &root,
        &[
            "queue",
            "run",
            "--state",
            state.to_str().unwrap(),
            "--worker",
            &worker(),
            "--jobs",
            "2",
        ],
    );
    assert!(run.status.success(), "{}", both(&run));
    assert!(both(&run).contains("2 passed"), "{}", both(&run));
}
