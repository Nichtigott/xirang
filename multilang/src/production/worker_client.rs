//! The client side of the worker protocol: send one job over a command transport, wait with an
//! optional timeout, retry transport failures, and run the same job over several projects with
//! bounded concurrency.
//! worker 协议的客户端：把一份任务交给命令传输、带可选超时等待、重传传输失败，并以有界并发把同一任务
//! 发给多个项目。

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use serde_json::{Value, json};

use super::worker_evidence::summarise;

/// Client-side limits: one timeout and one retry budget, carried through the wrappers.
/// 客户端限制：一个超时与一个重试预算，贯穿各层包装。
#[derive(Clone, Copy)]
pub struct Control {
    /// The per-task timeout, when the caller set one.
    /// 每个任务的超时（调用方设置时）。
    pub timeout: Option<std::time::Duration>,
    /// How many times a transport failure is retried.
    /// 传输失败重试几次。
    pub retries: usize,
}

/// What a transfer job points at: the path it names, where a fetched file is written, and the
/// transfer root under `target/xirang`. Bundled because three entries share the same three values
/// and passing them separately pushed each signature past the argument budget.
/// 一次传输任务指向什么：它点名的路径、取回的文件写到哪里、以及 `target/xirang` 下的回传根。把三者
/// 捆在一起，是因为三个入口共用同一组值，分开传会让每个签名越过参数预算。
#[derive(Clone, Copy)]
pub struct Transfer<'a> {
    /// The file or directory the job names, relative to the transfer root.
    /// 任务点名的文件或目录，相对回传根。
    pub path: Option<&'a str>,
    /// The directory a fetched file is written into.
    /// 取回的文件写入的目录。
    pub into: Option<&'a Path>,
    /// The transfer root under `target/xirang`.
    /// `target/xirang` 下的回传根。
    pub root: &'a str,
}

/// Send the same job up to `retries + 1` times. A retry re-sends the job; it does not repair
/// anything, so it is only useful for transports that fail before the task ran (a dropped
/// connection, a worker that did not answer). The attempts are named in the final error.
/// 把同一份任务最多发 `retries + 1` 次。重试只是重发，不会修复任何东西，因此只对“任务还没跑就断”的传输
/// 有用（连接断开、worker 没应答）；最终错误里点名尝试了几次。
pub(super) fn remote_with_retries(
    root: &Path,
    command: &str,
    action: &str,
    locked: bool,
    control: Control,
    transfer: Transfer<'_>,
) -> Result<String, String> {
    let attempts = control.retries.saturating_add(1);
    let mut last = String::new();
    for attempt in 1..=attempts {
        match remote(root, command, action, locked, control, transfer) {
            Ok(summary) => return Ok(summary),
            Err(error) => {
                // A refusal is the task's own answer, not a transport failure: re-sending the same
                // job would only ask the same question again.
                // 拒绝是任务自己的回答，不是传输失败：重发同一份任务只是再问一遍同一个问题。
                if error.starts_with("the worker refused:") {
                    return Err(error);
                }
                last = error;
                if attempt == attempts {
                    break;
                }
            }
        }
    }
    if attempts > 1 {
        Err(format!("{last} (after {attempts} attempt(s))"))
    } else {
        Err(last)
    }
}

/// Wait for a child that already has its output pipes drained by reader threads, killing it when
/// `limit` elapses. Without this the client would wait forever on a worker that hangs.
/// 等待一个输出管道已由读线程抽干的子进程；超过 `limit` 就杀掉它。没有这一步，客户端会永远等一个挂住的
/// worker。
fn wait_with_timeout(
    child: &mut std::process::Child,
    limit: std::time::Duration,
) -> Result<std::process::Output, String> {
    use std::io::Read;
    let mut stdout = child.stdout.take().ok_or("the worker has no stdout")?;
    let mut stderr = child.stderr.take().ok_or("the worker has no stderr")?;
    let stdout_reader = std::thread::spawn(move || {
        let mut buffer = Vec::new();
        let _ = stdout.read_to_end(&mut buffer);
        buffer
    });
    let stderr_reader = std::thread::spawn(move || {
        let mut buffer = Vec::new();
        let _ = stderr.read_to_end(&mut buffer);
        buffer
    });
    let started = std::time::Instant::now();
    let status = loop {
        if let Some(status) = child
            .try_wait()
            .map_err(|error| format!("cannot wait for the worker: {error}"))?
        {
            break status;
        }
        if started.elapsed() >= limit {
            let _ = child.kill();
            let _ = child.wait();
            let _ = stdout_reader.join();
            let _ = stderr_reader.join();
            return Err(format!(
                "the worker did not finish within {}s",
                limit.as_secs()
            ));
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    };
    let stdout = stdout_reader.join().unwrap_or_default();
    let stderr = stderr_reader.join().unwrap_or_default();
    Ok(std::process::Output {
        status,
        stdout,
        stderr,
    })
}

/// Ask one worker command to run the same job for several project roots, at most `jobs` at a
/// time. The worker protocol is unchanged: each task is one job in and one result out; only the
/// client decides how many are in flight. A failing project never hides the others — the report
/// names every task, and the call fails when any task failed.
/// 让一个 worker 命令对多个项目根跑同一份任务，最多同时 `jobs` 个。协议不变：每个任务仍是一进一出；
/// 只是客户端决定同时发几份。失败的项目不会盖住其它项目——报告逐条点名，只要有失败整体就失败。
pub(crate) fn remote_many_batch(
    roots: &[PathBuf],
    command: &str,
    action: &str,
    locked: bool,
    jobs: usize,
    control: Control,
) -> Result<String, String> {
    if roots.is_empty() {
        return Err("a batch needs at least one directory".to_owned());
    }
    if jobs == 0 {
        return Err("`--jobs` must be at least 1".to_owned());
    }
    let next = std::sync::atomic::AtomicUsize::new(0);
    let results = std::sync::Mutex::new(Vec::new());
    std::thread::scope(|scope| {
        for _ in 0..jobs.min(roots.len()) {
            scope.spawn(|| {
                loop {
                    let index = next.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    if index >= roots.len() {
                        break;
                    }
                    let outcome = remote_with_retries(
                        &roots[index],
                        command,
                        action,
                        locked,
                        control,
                        Transfer {
                            path: None,
                            into: None,
                            root: super::worker::DEFAULT_ROOT,
                        },
                    );
                    results
                        .lock()
                        .unwrap_or_else(|poisoned| poisoned.into_inner())
                        .push((index, outcome));
                }
            });
        }
    });
    let mut outcomes = results
        .into_inner()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    outcomes.sort_by_key(|(index, _)| *index);
    let mut lines = Vec::new();
    let mut failed = 0;
    for (index, outcome) in &outcomes {
        match outcome {
            Ok(summary) => lines.push(format!("{}: {summary}", roots[*index].display())),
            Err(error) => {
                failed += 1;
                lines.push(format!("{}: failed: {error}", roots[*index].display()));
            }
        }
    }
    let passed = outcomes.len().saturating_sub(failed);
    lines.push(format!(
        "remote worker: {passed} passed, {failed} failed of {} task(s)",
        outcomes.len()
    ));
    let report = lines.join("\n");
    if failed > 0 {
        return Err(report);
    }
    Ok(report)
}

/// Ask a worker command to run one job and return its summary.
/// 让一个 worker 命令运行一份任务并返回摘要。
/// Send one job over the command transport and return the worker's parsed answer after checking
/// the envelope: one JSON object, protocol 1, and the action the job asked for.
/// 把一份任务交给命令传输，检查信封（一个 JSON 对象、协议 1、动作回显）后返回解析好的应答。
pub(super) fn send(
    command: &str,
    job: &Value,
    action: &str,
    control: Control,
) -> Result<(Value, bool), String> {
    let job = job.to_string();
    let mut child = Command::new("sh")
        .arg("-c")
        .arg(command)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("cannot start the worker `{command}`: {error}"))?;
    {
        use std::io::Write;
        child
            .stdin
            .as_mut()
            .ok_or("the worker has no stdin")?
            .write_all(job.as_bytes())
            .map_err(|error| format!("cannot send the job: {error}"))?;
    }
    let output = match control.timeout {
        Some(limit) => wait_with_timeout(&mut child, limit)?,
        None => child
            .wait_with_output()
            .map_err(|error| format!("cannot wait for the worker: {error}"))?,
    };
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
    let value: Value = serde_json::from_str(&stdout).map_err(|error| {
        format!("the worker did not answer with one JSON document ({error}); stderr: {stderr}")
    })?;
    let object = value
        .as_object()
        .ok_or("the worker's answer is not a JSON object")?;
    match object.get("protocol") {
        Some(Value::Number(number)) if number.as_u64() == Some(1) => {}
        other => {
            return Err(format!(
                "this client speaks worker protocol 1; the worker answered {other:?}"
            ));
        }
    }
    if object.get("action").and_then(Value::as_str) != Some(action) {
        return Err(format!(
            "the worker answered for action {} while the job asked for `{action}`",
            object
                .get("action")
                .map_or_else(|| "none".to_owned(), Value::to_string)
        ));
    }
    Ok((value, output.status.success()))
}

pub(super) fn remote(
    root: &Path,
    command: &str,
    action: &str,
    locked: bool,
    control: Control,
    transfer: Transfer<'_>,
) -> Result<String, String> {
    if action != "build" && action != "test" && action != "evidence" && action != "fetch" {
        return Err(format!(
            "`{action}` is not an action this client asks for; it asks for `build`, `test`, \
             `evidence` and `fetch`"
        ));
    }
    if action != "fetch" && transfer.path.is_some() {
        return Err("`--fetch` is its own action; it cannot be combined with `--test`, `--evidence` or a plain build".to_owned());
    }
    if action == "fetch" && transfer.path.is_none() {
        return Err(format!(
            "`--fetch` names the file to fetch, relative to `<project>/target/xirang/{}`",
            transfer.root
        ));
    }
    if action == "fetch" && transfer.into.is_none() {
        return Err("`--fetch` needs `--into <directory>` to write the file to".to_owned());
    }
    let directory = root
        .canonicalize()
        .map_err(|error| format!("cannot resolve {}: {error}", root.display()))?;
    let mut job = json!({
        "protocol": 1,
        "action": action,
        "directory": directory.display().to_string(),
        "locked": locked,
    });
    if let Some(path) = transfer.path {
        job["path"] = json!(path);
        job["root"] = json!(transfer.root);
    }
    let (object, exit_success) = send(command, &job, action, control)?;
    match object.get("ok").and_then(Value::as_bool) {
        Some(true) => {
            if action == "fetch" {
                super::transfer::answered_root(&object, transfer.root)?;
                return super::transfer::fetch_file(
                    root,
                    command,
                    transfer.path.ok_or("`--fetch` names the file to fetch")?,
                    transfer
                        .into
                        .ok_or("`--fetch` needs `--into <directory>`")?,
                    transfer.root,
                    control,
                );
            }
            if action == "evidence" {
                let evidence = object
                    .get("evidence")
                    .and_then(Value::as_str)
                    .ok_or("the worker answered `evidence` but carries no `evidence` text")?;
                return Ok(format!("remote evidence: {}", summarise(evidence)?));
            }
            let summary = object
                .get("summary")
                .and_then(Value::as_str)
                .ok_or("the worker succeeded but names no `summary`")?;
            Ok(format!(
                "remote worker ({}): {summary}",
                if exit_success {
                    "exit 0"
                } else {
                    "non-zero exit"
                }
            ))
        }
        Some(false) => {
            let error = object
                .get("error")
                .and_then(Value::as_str)
                .ok_or("the worker failed but names no `error`")?;
            Err(format!("the worker refused: {error}"))
        }
        None => Err("the worker's answer carries no boolean `ok`".to_owned()),
    }
}

/// The `multilang` facade: run one build or test job through a worker command.
/// `multilang` 门面：让一个 worker 命令运行一次 build 或 test 任务。
pub fn remote_worker(
    root: &Path,
    command: &str,
    action: &str,
    locked: bool,
    control: Control,
    transfer: Transfer<'_>,
) -> Result<String, String> {
    remote_with_retries(root, command, action, locked, control, transfer)
}

/// The `multilang` facade over the batch entry point.
/// `multilang` 对批量入口的门面。
/// Explain one listing as text.
/// 把一份清单打印成文本。
pub fn remote_listing(
    root: &Path,
    command: &str,
    path: &str,
    transfer: &str,
    control: Control,
) -> Result<String, String> {
    let entries = super::transfer::listing(root, command, path, transfer, control)?;
    let mut text = format!(
        "remote listing {path} in transfer root `{transfer}`: {} file(s)",
        entries.len()
    );
    for (relative, bytes, sha256) in entries {
        text.push_str(&format!("\n  {relative} ({bytes} bytes, sha256 {sha256})"));
    }
    Ok(text)
}

/// Fetch a whole build directory through a worker.
/// 通过 worker 取回整个构建目录。
pub fn remote_fetch_all(
    root: &Path,
    command: &str,
    path: &str,
    into: &Path,
    transfer: &str,
    control: Control,
) -> Result<String, String> {
    super::transfer::fetch_all(root, command, path, into, transfer, control)
}

/// Run one task against several projects, at most `jobs` at a time.
/// 对多个项目并发（最多 `jobs` 个）运行同一个任务。
pub fn remote_many(
    roots: &[PathBuf],
    command: &str,
    action: &str,
    locked: bool,
    jobs: usize,
    control: Control,
) -> Result<String, String> {
    remote_many_batch(roots, command, action, locked, jobs, control)
}
