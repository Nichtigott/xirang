//! The remote-worker protocol: one job in, one result out over a command transport, exercised
//! through the real binary on both sides (`xirang worker` as the worker, `xirang remote` as the
//! client). The SSH/VM transports are not covered here and are not claimed.
//! 远程 worker 协议：任务进、结果出，经命令传输；两侧都用真二进制演练（worker 侧 `xirang worker`、
//! 客户端 `xirang remote`）。SSH/虚拟机传输不在本测试内，也不被声称。

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

fn scratch(label: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "xirang-remote-{label}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&path).unwrap();
    path
}

fn binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_xirang"))
}

fn xirang(directory: &Path, arguments: &[&str]) -> std::process::Output {
    Command::new(binary())
        .args(arguments)
        .current_dir(directory)
        .output()
        .unwrap()
}

fn xirang_env(
    directory: &Path,
    arguments: &[&str],
    environment: &[(&str, &str)],
) -> std::process::Output {
    let mut command = Command::new(binary());
    command.args(arguments).current_dir(directory);
    for (key, value) in environment {
        command.env(key, value);
    }
    command.output().unwrap()
}

fn set_executable(path: &Path) {
    use std::os::unix::fs::PermissionsExt;
    let mut permissions = std::fs::metadata(path).unwrap().permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(path, permissions).unwrap();
}

fn text(output: &std::process::Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn project(label: &str) -> PathBuf {
    let root = scratch(label);
    let created = xirang(&root, &["new", "--lang", "cpp", "--lib", "widget"]);
    assert!(created.status.success(), "{}", text(&created));
    root.join("widget")
}

#[test]
fn a_worker_runs_a_test_job_through_a_command_transport() {
    let root = project("job");
    let worker = format!("{} worker", binary().display());
    let output = xirang(&root, &["remote", "--worker", &worker, "--test"]);
    assert!(output.status.success(), "{}", text(&output));
    let reported = text(&output);
    assert!(
        reported.contains("remote worker (exit 0): xirang test:"),
        "{reported}"
    );
}

#[test]
fn a_client_refuses_an_answer_that_is_not_one_result_document() {
    let root = project("not-json");
    let empty = root.join("empty.sh");
    std::fs::write(&empty, "#!/bin/sh\nexit 0\n").unwrap();
    set_executable(&empty);
    let refused = xirang(&root, &["remote", "--worker", empty.to_str().unwrap()]);
    assert!(!refused.status.success());
    assert!(
        text(&refused).contains("did not answer with one JSON document"),
        "{}",
        text(&refused)
    );
    let wrong = root.join("wrong.sh");
    std::fs::write(&wrong, "#!/bin/sh\necho '{\"protocol\":2}'\n").unwrap();
    set_executable(&wrong);
    let wrong_protocol = xirang(&root, &["remote", "--worker", wrong.to_str().unwrap()]);
    assert!(!wrong_protocol.status.success());
    assert!(
        text(&wrong_protocol).contains("worker protocol 1"),
        "{}",
        text(&wrong_protocol)
    );
}

#[test]
fn a_worker_refuses_a_job_it_cannot_read() {
    let root = scratch("bad-job");
    let run = |job: &str| {
        let mut child = Command::new(binary())
            .arg("worker")
            .current_dir(&root)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        child
            .stdin
            .as_mut()
            .unwrap()
            .write_all(job.as_bytes())
            .unwrap();
        child.wait_with_output().unwrap()
    };
    let relative = run("{\"protocol\":1,\"action\":\"build\",\"directory\":\"local\"}");
    assert!(!relative.status.success());
    assert!(
        text(&relative).contains("not an absolute directory"),
        "{}",
        text(&relative)
    );
    let unknown = run("{\"protocol\":1,\"action\":\"build\",\"directory\":\"/tmp\",\"extra\":1}");
    assert!(!unknown.status.success());
    assert!(
        text(&unknown).contains("unknown job key"),
        "{}",
        text(&unknown)
    );
    let action = run("{\"protocol\":1,\"action\":\"publish\",\"directory\":\"/tmp\"}");
    assert!(!action.status.success());
    assert!(
        text(&action).contains("not an action this worker runs"),
        "{}",
        text(&action)
    );
}

#[test]
fn a_worker_returns_the_evidence_of_the_build_it_ran() {
    let root = project("evidence");
    let worker = format!("{} worker", binary().display());
    let built = xirang(&root, &["remote", "--worker", &worker]);
    assert!(built.status.success(), "{}", text(&built));
    let evidence = xirang(&root, &["remote", "--worker", &worker, "--evidence"]);
    assert!(evidence.status.success(), "{}", text(&evidence));
    let reported = text(&evidence);
    assert!(reported.contains("remote evidence: plan "), "{reported}");
    assert!(reported.contains("action(s)"), "{reported}");
    assert!(reported.contains("result passed"), "{reported}");
}

#[test]
fn evidence_before_any_build_is_refused() {
    let root = project("no-evidence");
    let worker = format!("{} worker", binary().display());
    let refused = xirang(&root, &["remote", "--worker", &worker, "--evidence"]);
    assert!(!refused.status.success());
    assert!(
        text(&refused).contains("has no build evidence"),
        "{}",
        text(&refused)
    );
}

#[test]
fn two_projects_run_on_one_worker_command_with_bounded_concurrency() {
    let first = project("batch-one");
    let second = project("batch-two");
    let worker = format!("{} worker", binary().display());
    let built = xirang(
        &first,
        &[
            "remote",
            "--worker",
            &worker,
            "--jobs",
            "2",
            first.to_str().unwrap(),
            second.to_str().unwrap(),
        ],
    );
    assert!(built.status.success(), "{}", text(&built));
    let reported = text(&built);
    assert!(reported.contains("batch-one"), "{reported}");
    assert!(reported.contains("batch-two"), "{reported}");
    assert!(
        reported.contains("2 passed, 0 failed of 2 task(s)"),
        "{reported}"
    );
    let zero = xirang(
        &first,
        &[
            "remote",
            "--worker",
            &worker,
            "--jobs",
            "0",
            first.to_str().unwrap(),
            second.to_str().unwrap(),
        ],
    );
    assert!(!zero.status.success());
    assert!(text(&zero).contains("--jobs"), "{}", text(&zero));
}

#[test]
fn a_failing_project_is_named_and_the_batch_fails() {
    let good = project("batch-good");
    let broken = good.parent().unwrap().join("batch-broken");
    std::fs::create_dir_all(&broken).unwrap();
    std::fs::write(
        broken.join("xirang.toml"),
        "schema = 1\n\n[package]\nname = \"broken\"\nversion = \"0.1.0\"\nkind = \"library\"\n\n[language.cpp]\nstandard = \"17\"\n\n[dependencies]\n",
    )
    .unwrap();
    let worker = format!("{} worker", binary().display());
    let built = xirang(
        &good,
        &[
            "remote",
            "--worker",
            &worker,
            "--jobs",
            "2",
            good.to_str().unwrap(),
            broken.to_str().unwrap(),
        ],
    );
    assert!(!built.status.success());
    let reported = text(&built);
    assert!(reported.contains("batch-broken"), "{reported}");
    assert!(
        reported.contains("1 passed, 1 failed of 2 task(s)"),
        "{reported}"
    );
}

#[test]
fn the_ssh_preset_asks_for_batch_mode_and_strict_host_keys() {
    let root = project("ssh");
    let bin = root.join("fake");
    std::fs::create_dir_all(&bin).unwrap();
    let ssh = bin.join("ssh");
    std::fs::write(
        &ssh,
        "#!/bin/sh\nprintf '%s\\n' \"$@\" > \"$SSH_ARGS\"\ncat > /dev/null\nprintf '%s\\n' '{\"protocol\":1,\"action\":\"build\",\"ok\":true,\"summary\":\"fake ssh worker\"}'\n",
    )
    .unwrap();
    set_executable(&ssh);
    let recorded = root.join("ssh-args.txt");
    let pinned = root.join("known_hosts");
    std::fs::write(&pinned, "builder.example ssh-ed25519 AAAA\n").unwrap();
    let path = format!(
        "{}:{}",
        bin.display(),
        std::env::var("PATH").unwrap_or_default()
    );
    let output = xirang_env(
        &root,
        &[
            "remote",
            "--ssh",
            "builder.example",
            "--known-hosts",
            pinned.to_str().unwrap(),
        ],
        &[("PATH", &path), ("SSH_ARGS", recorded.to_str().unwrap())],
    );
    assert!(output.status.success(), "{}", text(&output));
    assert!(
        text(&output).contains("fake ssh worker"),
        "{}",
        text(&output)
    );
    let seen = std::fs::read_to_string(&recorded).unwrap();
    for expected in [
        "-o",
        "BatchMode=yes",
        "StrictHostKeyChecking=yes",
        &format!("UserKnownHostsFile={}", pinned.display()),
        "GlobalKnownHostsFile=/dev/null",
        "builder.example",
        "xirang",
        "worker",
    ] {
        assert!(
            seen.contains(expected),
            "`{expected}` missing from:\n{seen}"
        );
    }
}

#[test]
fn ssh_and_worker_together_or_a_host_with_shell_characters_are_refused() {
    let root = project("ssh-refused");
    let both = xirang(
        &root,
        &["remote", "--worker", "true", "--ssh", "builder.example"],
    );
    assert!(!both.status.success());
    assert!(text(&both).contains("not both"), "{}", text(&both));
    let bad = xirang(&root, &["remote", "--ssh", "-oProxyCommand=evil"]);
    let pinned = xirang(
        &root,
        &["remote", "--worker", "true", "--known-hosts", "/tmp/x"],
    );
    assert!(!pinned.status.success());
    assert!(text(&pinned).contains("means nothing"), "{}", text(&pinned));
    let missing = xirang(
        &root,
        &[
            "remote",
            "--ssh",
            "builder.example",
            "--known-hosts",
            "/tmp/no-such-known-hosts",
        ],
    );
    assert!(!missing.status.success());
    assert!(
        text(&missing).contains("is not a file"),
        "{}",
        text(&missing)
    );
    assert!(!bad.status.success());
    assert!(
        text(&bad).contains("does not pass") || text(&bad).contains("host name"),
        "{}",
        text(&bad)
    );
}

fn newest_build(root: &Path) -> String {
    let mut names: Vec<String> = std::fs::read_dir(root.join("target/xirang/build"))
        .unwrap()
        .filter_map(|entry| {
            let path = entry.ok()?.path();
            if !path.is_dir() {
                return None;
            }
            path.file_name()?.to_str().map(str::to_owned)
        })
        .collect();
    names.sort();
    names.pop().unwrap()
}

#[test]
fn a_hung_worker_is_killed_at_the_timeout() {
    let root = project("timeout");
    let started = std::time::Instant::now();
    let output = xirang(&root, &["remote", "--worker", "sleep 5", "--timeout", "1"]);
    assert!(!output.status.success());
    assert!(
        text(&output).contains("did not finish within 1s"),
        "{}",
        text(&output)
    );
    assert!(
        started.elapsed() < std::time::Duration::from_secs(4),
        "took {:?}",
        started.elapsed()
    );
}

#[test]
fn a_transport_failure_is_retried_and_the_attempt_count_is_named() {
    let root = project("retry");
    let counter = root.join("counter.txt");
    let flaky = root.join("flaky.sh");
    std::fs::write(
        &flaky,
        "#!/bin/sh\ncat > /dev/null\ncount=$(cat \"$COUNTER\" 2>/dev/null || echo 0)\ncount=$((count+1))\necho \"$count\" > \"$COUNTER\"\nif [ \"$count\" -lt 2 ]; then exit 1; fi\nprintf '%s\\n' '{\"protocol\":1,\"action\":\"build\",\"ok\":true,\"summary\":\"recovered on attempt 2\"}'\n",
    )
    .unwrap();
    set_executable(&flaky);
    let path = flaky.to_str().unwrap();
    let counter_value = counter.to_str().unwrap();
    let first = xirang_env(
        &root,
        &["remote", "--worker", path],
        &[("COUNTER", counter_value)],
    );
    assert!(!first.status.success());
    assert!(std::fs::read_to_string(&counter).unwrap().trim() == "1");
    let retried = xirang_env(
        &root,
        &["remote", "--worker", path, "--retries", "1"],
        &[("COUNTER", counter_value)],
    );
    assert!(retried.status.success(), "{}", text(&retried));
    assert!(
        text(&retried).contains("recovered on attempt 2"),
        "{}",
        text(&retried)
    );
    let exhausted = root.join("exhausted.sh");
    std::fs::write(&exhausted, "#!/bin/sh\ncat > /dev/null\nexit 1\n").unwrap();
    set_executable(&exhausted);
    let failed = xirang(
        &root,
        &[
            "remote",
            "--worker",
            exhausted.to_str().unwrap(),
            "--retries",
            "1",
        ],
    );
    assert!(!failed.status.success());
    assert!(
        text(&failed).contains("after 2 attempt(s)"),
        "{}",
        text(&failed)
    );
    let zero_timeout = xirang(&root, &["remote", "--worker", "true", "--timeout", "0"]);
    assert!(!zero_timeout.status.success());
    assert!(
        text(&zero_timeout).contains("--timeout"),
        "{}",
        text(&zero_timeout)
    );
    let too_many = xirang(&root, &["remote", "--worker", "true", "--retries", "9"]);
    assert!(!too_many.status.success());
    assert!(text(&too_many).contains("--retries"), "{}", text(&too_many));
}

#[test]
fn a_worker_returns_one_build_file_with_its_sha256() {
    let root = project("fetch");
    let worker = format!("{} worker", binary().display());
    let built = xirang(&root, &["remote", "--worker", &worker]);
    assert!(built.status.success(), "{}", text(&built));
    let plan = newest_build(&root);
    let relative = format!("{plan}/evidence.json");
    let into = root.join("fetched");
    let fetched = xirang(
        &root,
        &[
            "remote",
            "--worker",
            &worker,
            "--fetch",
            &relative,
            "--into",
            into.to_str().unwrap(),
        ],
    );
    assert!(fetched.status.success(), "{}", text(&fetched));
    let local = std::fs::read(into.join("evidence.json")).unwrap();
    let original = std::fs::read(root.join("target/xirang/build").join(&relative)).unwrap();
    assert_eq!(local, original);
    assert!(String::from_utf8_lossy(&local).contains("\"plan\""));
    assert!(
        text(&fetched).contains(&xirang_kernel::sha256_hex(&local)),
        "{}",
        text(&fetched)
    );
}

#[test]
fn a_fetch_outside_the_build_directory_or_without_into_is_refused() {
    let root = project("fetch-refused");
    let worker = format!("{} worker", binary().display());
    let built = xirang(&root, &["remote", "--worker", &worker]);
    assert!(built.status.success(), "{}", text(&built));
    let plan = newest_build(&root);
    let into = root.join("fetched");
    let escape = xirang(
        &root,
        &[
            "remote",
            "--worker",
            &worker,
            "--fetch",
            "../../etc/passwd",
            "--into",
            into.to_str().unwrap(),
        ],
    );
    assert!(!escape.status.success());
    assert!(
        text(&escape).contains("must not contain `..`"),
        "{}",
        text(&escape)
    );
    let missing = xirang(
        &root,
        &[
            "remote",
            "--worker",
            &worker,
            "--fetch",
            &format!("{plan}/missing.bin"),
            "--into",
            into.to_str().unwrap(),
        ],
    );
    assert!(!missing.status.success());
    assert!(text(&missing).contains("no `"), "{}", text(&missing));
    let no_into = xirang(
        &root,
        &[
            "remote",
            "--worker",
            &worker,
            "--fetch",
            &format!("{plan}/evidence.json"),
        ],
    );
    assert!(!no_into.status.success());
    assert!(text(&no_into).contains("--into"), "{}", text(&no_into));
    let into_only = xirang(
        &root,
        &[
            "remote",
            "--worker",
            &worker,
            "--into",
            into.to_str().unwrap(),
        ],
    );
    assert!(!into_only.status.success());
    assert!(text(&into_only).contains("--into"), "{}", text(&into_only));
}

#[test]
fn a_worker_lists_a_build_root_and_the_client_fetches_it_whole() {
    let root = project("listing");
    let worker = format!("{} worker", binary().display());
    let built = xirang(&root, &["build"]);
    assert!(built.status.success(), "{}", text(&built));
    let builds = root.join("target/xirang/build");
    let listed = xirang(&root, &["remote", "--worker", &worker, "--list", "."]);
    assert!(listed.status.success(), "{}", text(&listed));
    let listing = text(&listed);
    assert!(
        listing.contains("remote listing . in transfer root `build`:"),
        "{listing}"
    );
    assert!(listing.contains("sha256"), "{listing}");
    let first = listing
        .lines()
        .find_map(|line| {
            line.trim()
                .split_once(" (")
                .map(|(path, _)| path.to_owned())
        })
        .expect("one file in the listing");
    assert!(
        builds.join(&first).is_file(),
        "{first} is not under the build root"
    );
    let into = root.join("fetched");
    let fetched = xirang(
        &root,
        &[
            "remote",
            "--worker",
            &worker,
            "--fetch-all",
            ".",
            "--into",
            into.to_str().unwrap(),
        ],
    );
    assert!(fetched.status.success(), "{}", text(&fetched));
    assert!(
        text(&fetched).contains("remote fetch-all:"),
        "{}",
        text(&fetched)
    );
    assert_eq!(
        std::fs::read(into.join(&first)).unwrap(),
        std::fs::read(builds.join(&first)).unwrap(),
        "the fetched bytes differ from the build's own bytes"
    );
}

#[test]
fn fetch_all_checks_the_listed_sha256_against_the_bytes_it_wrote() {
    let root = scratch("listing-tamper");
    let script = root.join("worker.py");
    std::fs::write(
        &script,
        "import json, sys\njob = json.load(sys.stdin)\nif job[\"action\"] == \"list\":\n    print(json.dumps({\"protocol\": 1, \"action\": \"list\", \"ok\": True, \"entries\": [{\"path\": \"one.txt\", \"bytes\": 3, \"sha256\": \"00\" * 32}]}))\nelse:\n    print(json.dumps({\"protocol\": 1, \"action\": \"fetch\", \"ok\": True, \"path\": \"one.txt\", \"offset\": 0, \"length\": 3, \"total\": 3, \"sha256\": \"00\" * 32, \"whole_sha256\": \"00\" * 32, \"content\": \"616263\"}))\n",
    )
    .unwrap();
    let worker = format!("python3 {}", script.display());
    let into = root.join("out");
    let fetched = xirang(
        &root,
        &[
            "remote",
            "--worker",
            &worker,
            "--fetch-all",
            ".",
            "--into",
            into.to_str().unwrap(),
        ],
    );
    assert!(!fetched.status.success());
    assert!(
        text(&fetched).contains("hashes to") || text(&fetched).contains("does not match"),
        "{}",
        text(&fetched)
    );
}

#[test]
fn listing_and_fetch_all_refuse_escapes_and_missing_destinations() {
    let root = project("listing-refusals");
    let worker = format!("{} worker", binary().display());
    let escape = xirang(
        &root,
        &["remote", "--worker", &worker, "--list", "../../etc"],
    );
    assert!(!escape.status.success());
    assert!(
        text(&escape).contains("must not contain `..`"),
        "{}",
        text(&escape)
    );
    let no_into = xirang(&root, &["remote", "--worker", &worker, "--fetch-all", "."]);
    assert!(!no_into.status.success());
    assert!(text(&no_into).contains("--into"), "{}", text(&no_into));
    let into_only = xirang(&root, &["remote", "--worker", &worker, "--into", "out"]);
    assert!(!into_only.status.success());
    assert!(
        text(&into_only).contains("only means something"),
        "{}",
        text(&into_only)
    );
}

#[test]
fn a_file_larger_than_one_result_document_travels_in_pieces() {
    let root = project("chunked");
    let worker = format!("{} worker", binary().display());
    let built = xirang(&root, &["build"]);
    assert!(built.status.success(), "{}", text(&built));
    let builds = root.join("target/xirang/build");
    let payload: Vec<u8> = (0..(5 * 1024 * 1024))
        .map(|index| (index % 251) as u8)
        .collect();
    std::fs::write(builds.join("big.bin"), &payload).unwrap();
    let into = root.join("out");
    let fetched = xirang(
        &root,
        &[
            "remote",
            "--worker",
            &worker,
            "--fetch",
            "big.bin",
            "--into",
            into.to_str().unwrap(),
        ],
    );
    assert!(fetched.status.success(), "{}", text(&fetched));
    assert!(
        text(&fetched).contains("remote fetch: big.bin"),
        "{}",
        text(&fetched)
    );
    assert_eq!(std::fs::read(into.join("big.bin")).unwrap(), payload);
}

/// A file past the 64 MiB limit the transport used to refuse now moves whole. The old rule is
/// gone on purpose — the worker hashes the file in pieces and the client writes each verified
/// piece to a partial file — so this test replaces the refusal it used to pin. It is the one long
/// test in this file: 65 MiB through an unoptimized build costs the better part of a minute.
/// 过去被传输拒绝的、超过 64 MiB 的文件现在整份搬得动。旧规则是有意去掉的——worker 按片哈希、客户端
/// 把每一片校验后写进临时文件——因此这条测试取代了它原先钉住的拒绝。它是本文件里唯一的长测试：65 MiB
/// 走一遍未优化的构建要花掉将近一分钟。
#[test]
fn a_file_past_the_old_sixty_four_mib_limit_moves_whole() {
    let root = project("chunked-limit");
    let worker = format!("{} worker", binary().display());
    let built = xirang(&root, &["build"]);
    assert!(built.status.success(), "{}", text(&built));
    let huge = root.join("target/xirang/build/huge.bin");
    let block = vec![b'h'; 1024 * 1024];
    {
        use std::io::Write;
        let mut file = std::fs::File::create(&huge).unwrap();
        for _ in 0..65 {
            file.write_all(&block).unwrap();
        }
    }
    let expected = xirang_kernel::sha256_hex(&std::fs::read(&huge).unwrap());
    let into = root.join("out");
    let fetched = xirang(
        &root,
        &[
            "remote",
            "--worker",
            &worker,
            "--fetch",
            "huge.bin",
            "--into",
            into.to_str().unwrap(),
        ],
    );
    assert!(fetched.status.success(), "{}", text(&fetched));
    assert!(
        text(&fetched).contains(&expected),
        "the fetched digest is not the file's own digest: {}",
        text(&fetched)
    );
    let size = std::fs::metadata(into.join("huge.bin")).unwrap().len();
    assert_eq!(size, 65 * 1024 * 1024, "the fetched file is {size} bytes");
    let leftovers: Vec<String> = std::fs::read_dir(&into)
        .unwrap()
        .filter_map(|entry| entry.ok()?.file_name().to_str().map(str::to_owned))
        .filter(|name| name.contains("partial"))
        .collect();
    assert!(
        leftovers.is_empty(),
        "a finished fetch left {leftovers:?} behind"
    );
}

#[test]
fn a_transfer_root_selects_another_subtree_of_target_xirang() {
    let root = project("transfer-root");
    let worker = format!("{} worker", binary().display());
    let built = xirang(&root, &["build"]);
    assert!(built.status.success(), "{}", text(&built));
    let note = root.join("target/xirang/cas/objects/note.txt");
    std::fs::create_dir_all(note.parent().unwrap()).unwrap();
    std::fs::write(&note, b"a note in another subtree\n").unwrap();

    let builds = text(&xirang(
        &root,
        &["remote", "--worker", &worker, "--list", "."],
    ));
    assert!(
        builds.contains("in transfer root `build`:") && !builds.contains("objects/note.txt"),
        "the default root is not the build directory: {builds}"
    );
    let cas = xirang(
        &root,
        &[
            "remote", "--worker", &worker, "--list", ".", "--root", "cas",
        ],
    );
    let listing = text(&cas);
    assert!(cas.status.success(), "{listing}");
    assert!(
        listing.contains("in transfer root `cas`:") && listing.contains("objects/note.txt"),
        "{listing}"
    );

    let into = root.join("fetched");
    let fetched = xirang(
        &root,
        &[
            "remote",
            "--worker",
            &worker,
            "--fetch",
            "objects/note.txt",
            "--root",
            "cas",
            "--into",
            into.to_str().unwrap(),
        ],
    );
    assert!(fetched.status.success(), "{}", text(&fetched));
    assert_eq!(
        std::fs::read(into.join("note.txt")).unwrap(),
        std::fs::read(&note).unwrap()
    );

    let escape = xirang(
        &root,
        &[
            "remote", "--worker", &worker, "--list", ".", "--root", "../etc",
        ],
    );
    assert!(!escape.status.success());
    assert!(
        text(&escape).contains("must not contain `..`"),
        "{}",
        text(&escape)
    );
    let absolute = xirang(
        &root,
        &[
            "remote", "--worker", &worker, "--list", ".", "--root", "/etc",
        ],
    );
    assert!(!absolute.status.success());
    assert!(
        text(&absolute).contains("must be a relative path"),
        "{}",
        text(&absolute)
    );
    let missing = xirang(
        &root,
        &[
            "remote", "--worker", &worker, "--list", ".", "--root", "nowhere",
        ],
    );
    assert!(!missing.status.success());
    assert!(
        text(&missing).contains("the transfer root has no `nowhere`"),
        "{}",
        text(&missing)
    );
    let wrong_action = xirang(
        &root,
        &["remote", "--worker", &worker, "--test", "--root", "cas"],
    );
    assert!(!wrong_action.status.success());
    assert!(
        text(&wrong_action).contains("`--root` only means something"),
        "{}",
        text(&wrong_action)
    );
}

/// A worker that ignores `root` must not be believed when the job asked for another subtree. The
/// compatible case stays compatible: no echoed root is exactly what a worker said before the field
/// existed, so the default root accepts it.
/// 忽略 `root` 的 worker，在任务要求另一棵子树时不能信。兼容情形保持兼容：没有回显正是该字段存在
/// 之前 worker 的回答，因此默认根接受它。
#[test]
fn a_worker_that_does_not_echo_the_transfer_root_is_only_trusted_for_build() {
    let root = scratch("transfer-root-echo");
    let silent = root.join("silent.sh");
    std::fs::write(
        &silent,
        "#!/bin/sh\ncat >/dev/null\nprintf '%s' '{\"protocol\":1,\"action\":\"list\",\"ok\":true,\"entries\":[]}'\n",
    )
    .unwrap();
    set_executable(&silent);
    let echoing = root.join("echoing.sh");
    std::fs::write(
        &echoing,
        "#!/bin/sh\ncat >/dev/null\nprintf '%s' '{\"protocol\":1,\"action\":\"list\",\"ok\":true,\"root\":\"build\",\"entries\":[]}'\n",
    )
    .unwrap();
    set_executable(&echoing);
    let silent_worker = silent.display().to_string();
    let echoing_worker = echoing.display().to_string();

    let default_root = xirang(
        &root,
        &["remote", "--worker", &silent_worker, "--list", "."],
    );
    assert!(default_root.status.success(), "{}", text(&default_root));
    assert!(
        text(&default_root).contains("in transfer root `build`: 0 file(s)"),
        "{}",
        text(&default_root)
    );
    let another_root = xirang(
        &root,
        &[
            "remote",
            "--worker",
            &silent_worker,
            "--list",
            ".",
            "--root",
            "cas",
        ],
    );
    assert!(!another_root.status.success());
    assert!(
        text(&another_root).contains("names no transfer root"),
        "{}",
        text(&another_root)
    );
    let mismatched = xirang(
        &root,
        &[
            "remote",
            "--worker",
            &echoing_worker,
            "--list",
            ".",
            "--root",
            "cas",
        ],
    );
    assert!(!mismatched.status.success());
    assert!(
        text(&mismatched)
            .contains("the worker read transfer root `build` while the job asked for `cas`"),
        "{}",
        text(&mismatched)
    );
    let agreeing = xirang(
        &root,
        &[
            "remote",
            "--worker",
            &echoing_worker,
            "--list",
            ".",
            "--root",
            "build",
        ],
    );
    assert!(agreeing.status.success(), "{}", text(&agreeing));
}

#[test]
fn a_file_larger_than_one_piece_is_listed_fetched_and_fetched_whole() {
    let root = project("chunked");
    let worker = format!("{} worker", binary().display());
    let built = xirang(&root, &["build"]);
    assert!(built.status.success(), "{}", text(&built));
    let plan = newest_build(&root);
    let big = root.join("target/xirang/build").join(&plan).join("big.bin");
    let bytes: Vec<u8> = (0..(6 * 1024 * 1024)).map(|index| index as u8).collect();
    std::fs::write(&big, &bytes).unwrap();
    let expected = xirang_kernel::sha256_hex(&bytes);

    let listed = xirang(&root, &["remote", "--worker", &worker, "--list", &plan]);
    let listing = text(&listed);
    assert!(listed.status.success(), "{listing}");
    assert!(
        listing.contains(&format!(
            "{plan}/big.bin (6291456 bytes, sha256 {expected})"
        )),
        "{listing}"
    );

    let into = root.join("fetched");
    let fetched = xirang(
        &root,
        &[
            "remote",
            "--worker",
            &worker,
            "--fetch",
            &format!("{plan}/big.bin"),
            "--into",
            into.to_str().unwrap(),
        ],
    );
    assert!(fetched.status.success(), "{}", text(&fetched));
    assert!(text(&fetched).contains(&expected), "{}", text(&fetched));
    assert_eq!(
        std::fs::metadata(into.join("big.bin")).unwrap().len(),
        6291456
    );

    let whole = root.join("whole");
    let fetched_all = xirang(
        &root,
        &[
            "remote",
            "--worker",
            &worker,
            "--fetch-all",
            &plan,
            "--into",
            whole.to_str().unwrap(),
        ],
    );
    assert!(fetched_all.status.success(), "{}", text(&fetched_all));
    assert_eq!(
        std::fs::read(whole.join(&plan).join("big.bin")).unwrap(),
        bytes,
        "a piece-wise fetch-all did not preserve the bytes"
    );
}
