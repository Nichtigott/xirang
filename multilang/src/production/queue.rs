//! A persistent queue of remote jobs, kept on disk so it survives the process that submitted it.
//! A job is a project directory, an action and a `--locked` flag; the queue only records what has
//! been submitted, what is running, and what a run produced. A claim is one atomic rename, so two
//! runners can share a state directory without executing the same job twice.
//! 磁盘上的持久远程任务队列，因此它在提交进程退出后仍然存在。任务是一个项目目录、一个动作和一个
//! `--locked` 标志；队列只记录提交了什么、什么在运行、一次运行产出了什么。领取是一次原子改名，
//! 因此两个 runner 共用一个状态目录也不会把同一个任务执行两遍。

use std::path::{Path, PathBuf};

/// The state of one submitted job.
/// 一个已提交任务的状态。
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Status {
    Queued,
    Running,
    Done,
    Failed,
    Cancelled,
}

impl Status {
    pub(crate) fn name(self) -> &'static str {
        match self {
            Status::Queued => "queued",
            Status::Running => "running",
            Status::Done => "done",
            Status::Failed => "failed",
            Status::Cancelled => "cancelled",
        }
    }
}

/// One job as the queue knows it.
/// 队列知道的一个任务。
pub(crate) struct Entry {
    pub(crate) id: String,
    pub(crate) directory: String,
    pub(crate) action: String,
    pub(crate) status: Status,
    pub(crate) result: Option<String>,
}

fn jobs(state: &Path) -> PathBuf {
    state.join("jobs")
}

fn job_path(state: &Path, id: &str) -> PathBuf {
    jobs(state).join(format!("{id}.json"))
}

fn running_path(state: &Path, id: &str) -> PathBuf {
    jobs(state).join(format!("{id}.running"))
}

fn result_path(state: &Path, id: &str) -> PathBuf {
    jobs(state).join(format!("{id}.result.json"))
}

fn cancelled_path(state: &Path, id: &str) -> PathBuf {
    jobs(state).join(format!("{id}.cancelled"))
}

/// A job id: twelve hex digits derived from the project, the action and the submission moment.
/// 任务 id：十二个十六进制数字，由项目、动作与提交时刻派生。
fn identifier(directory: &Path, action: &str, locked: bool) -> String {
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    let material = format!(
        "{}\0{action}\0{locked}\0{stamp}\0{}",
        directory.display(),
        std::process::id()
    );
    xirang_kernel::sha256_hex(material.as_bytes())[..12].to_owned()
}

fn valid_id(id: &str) -> bool {
    id.len() == 12 && id.chars().all(|character| character.is_ascii_hexdigit())
}

/// Submit one project to the queue and return its id.
/// 把一个项目提交进队列，返回它的 id。
pub fn enqueue(
    state: &Path,
    directory: &Path,
    action: &str,
    locked: bool,
) -> Result<String, String> {
    let directory = directory
        .canonicalize()
        .map_err(|error| format!("cannot resolve {}: {error}", directory.display()))?;
    if !directory.join("xirang.toml").is_file() {
        return Err(format!(
            "{} is not a project: it has no `xirang.toml`",
            directory.display()
        ));
    }
    if !matches!(action, "build" | "test") {
        return Err(format!(
            "`{action}` is not a queued action; the queue holds `build` and `test`"
        ));
    }
    std::fs::create_dir_all(jobs(state))
        .map_err(|error| format!("cannot create {}: {error}", jobs(state).display()))?;
    let id = identifier(&directory, action, locked);
    let job = serde_json::json!({
        "id": id,
        "directory": directory.display().to_string(),
        "action": action,
        "locked": locked,
        "queued_at": std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0)
            .to_string(),
    });
    let temporary = jobs(state).join(format!("{id}.json.tmp"));
    let destination = job_path(state, &id);
    std::fs::write(&temporary, serde_json::to_string_pretty(&job).unwrap())
        .map_err(|error| format!("cannot write {}: {error}", temporary.display()))?;
    std::fs::rename(&temporary, &destination)
        .map_err(|error| format!("cannot publish {}: {error}", destination.display()))?;
    Ok(id)
}

fn read_job(path: &Path) -> Option<(String, String, String)> {
    let text = std::fs::read_to_string(path).ok()?;
    let value: serde_json::Value = serde_json::from_str(&text).ok()?;
    Some((
        value.get("id")?.as_str()?.to_owned(),
        value.get("directory")?.as_str()?.to_owned(),
        value.get("action")?.as_str()?.to_owned(),
    ))
}

/// Every job the state directory knows, queued first in submission order.
/// 状态目录知道的全部任务，排队中的按提交顺序排在前面。
pub(crate) fn entries(state: &Path) -> Result<Vec<Entry>, String> {
    let Ok(read) = std::fs::read_dir(jobs(state)) else {
        return Ok(Vec::new());
    };
    // A job owns several files over its life: `.json` while queued, `.running` once claimed,
    // `.result.json` when a run finished, `.cancelled` when it was withdrawn before starting.
    // One pass collects the ids; the job's own bytes live in whichever file still carries them.
    // 一个任务在一生中拥有多个文件：排队时 `.json`、被领取后 `.running`、跑完有 `.result.json`、
    // 开始前被撤走则 `.cancelled`。一趟扫描收集 id；任务本身的字节留在仍携带它的那个文件里。
    let mut ids: Vec<String> = Vec::new();
    for entry in read {
        let entry = entry.map_err(|error| format!("cannot read the queue: {error}"))?;
        let name = entry.file_name().to_string_lossy().into_owned();
        let id = name
            .strip_suffix(".result.json")
            .or_else(|| name.strip_suffix(".cancelled"))
            .or_else(|| name.strip_suffix(".running"))
            .or_else(|| name.strip_suffix(".json"));
        let Some(id) = id else { continue };
        if valid_id(id) && !ids.iter().any(|seen| seen == id) {
            ids.push(id.to_owned());
        }
    }
    let mut entries = Vec::new();
    for id in ids {
        let job = std::fs::read_to_string(job_path(state, &id))
            .or_else(|_| std::fs::read_to_string(running_path(state, &id)))
            .or_else(|_| std::fs::read_to_string(cancelled_path(state, &id)));
        let Ok(job) = job else { continue };
        let Ok(value) = serde_json::from_str::<serde_json::Value>(&job) else {
            continue;
        };
        let result = std::fs::read_to_string(result_path(state, &id)).ok();
        let status = match &result {
            Some(text) => match serde_json::from_str::<serde_json::Value>(text)
                .ok()
                .and_then(|value| value.get("ok").and_then(serde_json::Value::as_bool))
            {
                Some(true) => Status::Done,
                _ => Status::Failed,
            },
            None if running_path(state, &id).is_file() => Status::Running,
            None if cancelled_path(state, &id).is_file() => Status::Cancelled,
            None => Status::Queued,
        };
        entries.push(Entry {
            id,
            directory: value
                .get("directory")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("")
                .to_owned(),
            action: value
                .get("action")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("")
                .to_owned(),
            status,
            result,
        });
    }
    entries.sort_by(|left, right| {
        let rank = |status: Status| match status {
            Status::Queued => 0,
            Status::Running => 1,
            _ => 2,
        };
        rank(left.status)
            .cmp(&rank(right.status))
            .then_with(|| left.id.cmp(&right.id))
    });
    Ok(entries)
}

/// Print one line per job, with a one-line result where a run produced one.
/// 每个任务打印一行，跑过的话带一行结果。
pub fn list(state: &Path) -> Result<String, String> {
    let entries = entries(state)?;
    if entries.is_empty() {
        return Ok(format!("queue {}: no job", state.display()));
    }
    let mut out = String::new();
    for entry in &entries {
        out.push_str(&format!(
            "{}\t{}\t{}\t{}",
            entry.id,
            entry.status.name(),
            entry.action,
            entry.directory
        ));
        if let Some(result) = &entry.result
            && let Ok(value) = serde_json::from_str::<serde_json::Value>(result)
        {
            let text = value
                .get("summary")
                .or_else(|| value.get("error"))
                .and_then(serde_json::Value::as_str)
                .unwrap_or("")
                .lines()
                .next()
                .unwrap_or("");
            out.push('\t');
            out.push_str(text);
        }
        out.push('\n');
    }
    Ok(out)
}

/// Show the job and the result it produced, if any.
/// 显示任务与它产出的结果（如果有）。
pub fn show(state: &Path, id: &str) -> Result<String, String> {
    if !valid_id(id) {
        return Err(format!("`{id}` is not a job id (twelve hex digits)"));
    }
    let job = std::fs::read_to_string(job_path(state, id))
        .or_else(|_| std::fs::read_to_string(running_path(state, id)))
        .or_else(|_| std::fs::read_to_string(cancelled_path(state, id)))
        .map_err(|_| format!("no job `{id}` in {}", state.display()))?;
    let result = std::fs::read_to_string(result_path(state, id)).unwrap_or_default();
    Ok(if result.is_empty() {
        job
    } else {
        format!("{job}\n{result}")
    })
}

/// Cancel a job that has not started; running and finished jobs are named, not touched.
/// 取消一个尚未开始的任务；运行中与已结束的任务只被点名，不被触碰。
pub fn cancel(state: &Path, id: &str) -> Result<String, String> {
    if !valid_id(id) {
        return Err(format!("`{id}` is not a job id (twelve hex digits)"));
    }
    let queued = job_path(state, id);
    if !queued.is_file() {
        if result_path(state, id).is_file() {
            let text = std::fs::read_to_string(result_path(state, id)).unwrap_or_default();
            let status = match serde_json::from_str::<serde_json::Value>(&text)
                .ok()
                .and_then(|value| value.get("ok").and_then(serde_json::Value::as_bool))
            {
                Some(true) => "done",
                _ => "failed",
            };
            return Err(format!(
                "job `{id}` already finished ({status}); nothing was cancelled"
            ));
        }
        if running_path(state, id).is_file() {
            return Err(format!(
                "job `{id}` is running; a job is cancelled before it starts"
            ));
        }
        if cancelled_path(state, id).is_file() {
            return Err(format!("job `{id}` is already cancelled"));
        }
        return Err(format!("no job `{id}` in {}", state.display()));
    }
    std::fs::rename(&queued, cancelled_path(state, id))
        .map_err(|error| format!("cannot cancel `{id}`: {error}"))?;
    Ok(format!("cancelled job `{id}`; it had not started"))
}

/// Run queued jobs through `command`, up to `jobs` at a time, and record every result.
/// 用 `command` 运行排队中的任务，最多 `jobs` 个并发，并记录每个结果。
pub fn run_queue(state: &Path, command: &str, jobs: usize) -> Result<String, String> {
    if jobs == 0 {
        return Err("`--jobs` needs at least one worker".to_owned());
    }
    if !state.join("jobs").is_dir() {
        return Ok(format!("queue {}: no queued job", state.display()));
    }
    let queued: Vec<String> = entries(state)?
        .into_iter()
        .filter(|entry| entry.status == Status::Queued)
        .map(|entry| entry.id)
        .collect();
    if queued.is_empty() {
        return Ok(format!("queue {}: no queued job", state.display()));
    }
    let next = std::sync::atomic::AtomicUsize::new(0);
    let results = std::sync::Mutex::new(Vec::new());
    std::thread::scope(|scope| {
        for _ in 0..jobs {
            scope.spawn(|| {
                loop {
                    let index = next.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    let Some(id) = queued.get(index).cloned() else {
                        break;
                    };
                    let claimed = std::fs::rename(job_path(state, &id), running_path(state, &id));
                    if claimed.is_err() {
                        // Another runner took this job first; that is not a failure.
                        continue;
                    }
                    let job = read_job(&running_path(state, &id));
                    let Some((_, directory, action)) = job else {
                        continue;
                    };
                    let locked = std::fs::read_to_string(running_path(state, &id))
                        .ok()
                        .and_then(|text| serde_json::from_str::<serde_json::Value>(&text).ok())
                        .and_then(|value| value.get("locked").and_then(serde_json::Value::as_bool))
                        .unwrap_or(false);
                    let control = crate::production::worker_client::Control {
                        timeout: None,
                        retries: 0,
                    };
                    let outcome = crate::production::worker_client::remote_worker(
                        Path::new(&directory),
                        command,
                        &action,
                        locked,
                        control,
                        crate::production::Transfer {
                            path: None,
                            into: None,
                            root: crate::production::worker::DEFAULT_ROOT,
                        },
                    );
                    let record = match &outcome {
                        Ok(summary) => serde_json::json!({
                            "id": id,
                            "ok": true,
                            "summary": summary,
                        }),
                        Err(error) => serde_json::json!({
                            "id": id,
                            "ok": false,
                            "error": error,
                        }),
                    };
                    let _ = std::fs::write(
                        result_path(state, &id),
                        serde_json::to_string_pretty(&record).unwrap(),
                    );
                    results.lock().unwrap().push((id, outcome));
                }
            });
        }
    });
    let results = results.into_inner().unwrap();
    let passed = results
        .iter()
        .filter(|(_, outcome)| outcome.is_ok())
        .count();
    let failed = results.len() - passed;
    let mut out = String::new();
    for (id, outcome) in &results {
        match outcome {
            Ok(summary) => out.push_str(&format!(
                "{}: done: {}\n",
                id,
                summary.lines().next().unwrap_or("")
            )),
            Err(error) => out.push_str(&format!("{id}: failed: {error}\n")),
        }
    }
    out.push_str(&format!(
        "queue {}: ran {} job(s) ({passed} passed, {failed} failed)",
        state.display(),
        results.len()
    ));
    Ok(out)
}
