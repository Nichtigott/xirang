//! Bounded verification jobs with immutable run logs and cooperative process-group cancellation.
//! 有界验证任务、不可变运行日志及进程组取消。

use serde_json::Value;
use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{
    Arc, Mutex, OnceLock,
    atomic::{AtomicBool, AtomicU64, Ordering},
};

struct Job {
    root: PathBuf,
    owner: u64,
    cancel: Arc<AtomicBool>,
    result: Option<Result<String, String>>,
}
static JOBS: OnceLock<Mutex<BTreeMap<String, Job>>> = OnceLock::new();
thread_local! { static CANCEL: RefCell<Option<Arc<AtomicBool>>> = const { RefCell::new(None) }; }

thread_local! { static OWNER: Cell<u64> = const { Cell::new(0) }; }
pub(super) struct Session {
    owner: u64,
    previous: u64,
}
pub(super) fn session() -> Session {
    static NEXT: AtomicU64 = AtomicU64::new(1);
    let owner = NEXT.fetch_add(1, Ordering::Relaxed);
    let previous = OWNER.with(|current| current.replace(owner));
    Session { owner, previous }
}
impl Drop for Session {
    fn drop(&mut self) {
        OWNER.with(|current| current.set(self.previous));
        let Some(jobs) = JOBS.get() else {
            return;
        };
        {
            let jobs = jobs.lock().unwrap_or_else(|e| e.into_inner());
            for job in jobs
                .values()
                .filter(|job| job.owner == self.owner && job.result.is_none())
            {
                job.cancel.store(true, Ordering::Release);
            }
        }
        let started = std::time::Instant::now();
        while started.elapsed() < std::time::Duration::from_secs(5) {
            let pending = jobs
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .values()
                .any(|job| job.owner == self.owner && job.result.is_none());
            if !pending {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
    }
}

pub(super) fn new_id() -> String {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    format!(
        "{}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    )
}

/// The identity of the connection asking. It is what a candidate records as its opener, so a review
/// can refuse the one case that is decidable: a review signed by whoever opened the candidate.
/// 提问的那个连接的身份。候选把它记成"开启者"，因此评审可以拒绝那唯一可判定的一种情形：由开启候选的
/// 那一方签字的评审。
pub(super) fn owner() -> u64 {
    OWNER.with(Cell::get)
}

pub(super) fn cancelled() -> bool {
    CANCEL.with(|slot| {
        slot.borrow()
            .as_ref()
            .is_some_and(|flag| flag.load(Ordering::Acquire))
    })
}

pub(super) fn check(root: &Path, args: &Value) -> Result<String, String> {
    let action = args.get("action").and_then(Value::as_str).unwrap_or("run");
    if args.get("action").is_some_and(|value| !value.is_string()) {
        return Err("action must be plan, run, start, list, status, result, or cancel".into());
    }
    let root = root.canonicalize().map_err(|error| error.to_string())?;
    // A durable plan is addressed by `plan_id`, and its actions (`status`, `resume`) are not
    // session jobs: they read or continue a ledger that outlives this connection (W4).
    // 持久计划按 `plan_id` 寻址，而它的动作（`status`、`resume`）不是会话内任务：它们读或继续一份比
    // 本连接活得更久的台账（W4）。
    if args.get("plan_id").is_some() {
        return super::check_resume::plan_action(&root, args);
    }
    // BR06: a plan phase is not a job. It starts no Cargo, it finishes in the time it takes to read
    // the tree, and its whole point is to be askable **before** anything is spent — so it is answered
    // on the calling thread instead of becoming a background job somebody then polls. It keeps its
    // `action` on the way in, because that key is what tells the plan builder not to execute.
    // BR06：计划阶段不是任务。它不启动 Cargo，读完树就结束，而它的全部意义正是在花掉任何东西**之前**可以
    // 被问到——因此它在调用线程上回答，而不是变成一个还要轮询的后台任务。它把 `action` 带进去，因为正是那个
    // 键在告诉计划构造器不要执行。
    if action == "plan" {
        return super::check_plan::run(&root, args);
    }
    if matches!(action, "list" | "status" | "result" | "cancel") {
        for key in args
            .as_object()
            .into_iter()
            .flat_map(|object| object.keys())
        {
            if !["action", "job", "root"].contains(&key.as_str()) {
                return Err(format!("{action} does not accept {key}"));
            }
        }
        if action == "list" {
            if args.get("job").is_some() {
                return Err("list does not accept job".into());
            }
            let jobs = JOBS
                .get_or_init(Default::default)
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            return Ok(jobs
                .iter()
                .filter(|(_, job)| job.root == root && job.owner == OWNER.with(Cell::get))
                .map(|(id, job)| {
                    format!(
                        "{id}: {}",
                        if job.result.is_some() {
                            "finished"
                        } else {
                            "running"
                        }
                    )
                })
                .collect::<Vec<_>>()
                .join("\n"));
        }
        let id = args
            .get("job")
            .and_then(Value::as_str)
            .ok_or("job is required")?;
        let jobs = JOBS
            .get_or_init(Default::default)
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let job = jobs
            .get(id)
            .filter(|job| job.root == root && job.owner == OWNER.with(Cell::get))
            .ok_or("unknown job for this root (jobs belong to this server session)")?;
        if action == "cancel" && job.result.is_none() {
            job.cancel.store(true, Ordering::Release);
            return Ok(format!(
                "job {id}\nstate cancelling; poll status/result for termination"
            ));
        }
        return match &job.result {
            Some(result) if action == "result" => result.clone(),
            Some(_) => Ok(format!(
                "job {id}\nstate finished; use action:result for verdict"
            )),
            None => Ok(format!(
                "job {id}\nstate {}",
                if job.cancel.load(Ordering::Acquire) {
                    "cancelling"
                } else {
                    "running"
                }
            )),
        };
    }
    if !matches!(action, "run" | "start") {
        return Err("action must be plan, run, start, list, status, result, or cancel".into());
    }
    let mut request = args.clone();
    if let Some(object) = request.as_object_mut() {
        object.remove("action");
    }
    super::check_request::validate(&request)?;
    if action == "run" {
        return super::check::check(&root, &request);
    }
    let owner = OWNER.with(Cell::get);
    if owner == 0 {
        return Err("action:start requires a persistent MCP connection; the one-shot CLI must use action:run".into());
    }
    let id = new_id();
    let flag = Arc::new(AtomicBool::new(false));
    let mut jobs = JOBS
        .get_or_init(Default::default)
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    if jobs.values().filter(|job| job.result.is_none()).count() >= 4 {
        return Err("four verification jobs already running; wait or cancel a job".into());
    }
    if jobs.len() >= 32
        && let Some(old) = jobs
            .iter()
            .find(|(_, job)| job.result.is_some())
            .map(|(id, _)| id.clone())
    {
        jobs.remove(&old);
    }
    jobs.insert(
        id.clone(),
        Job {
            root: root.clone(),
            owner,
            cancel: flag.clone(),
            result: None,
        },
    );
    drop(jobs);
    let worker_id = id.clone();
    let spawn = std::thread::Builder::new()
        .name("xirang-check".into())
        .spawn(move || {
            let _metadata_cancel = crate::cargo_metadata::cancellation(flag.clone());
            CANCEL.with(|slot| *slot.borrow_mut() = Some(flag));
            let result = std::panic::catch_unwind(|| super::check::check(&root, &request))
                .unwrap_or_else(|_| Err("verification worker panicked; outcome unknown".into()));
            if let Some(job) = JOBS
                .get()
                .unwrap()
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .get_mut(&worker_id)
            {
                job.result = Some(result);
            }
            CANCEL.with(|slot| *slot.borrow_mut() = None);
        });
    if let Err(error) = spawn {
        JOBS.get()
            .unwrap()
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(&id);
        return Err(format!("cannot start verification worker: {error}"));
    }
    Ok(format!(
        "job {id}\nstate running; check {{action:status, job:\"{id}\"}}"
    ))
}

/// Kill the entire execution group; children inherit it unless deliberately detached.
/// 杀死整个执行组；除非主动脱离，后代会继承该组。
pub(super) fn terminate(child: &mut std::process::Child) {
    #[cfg(unix)]
    {
        let _ = std::process::Command::new("kill")
            .args(["-KILL", "--", &format!("-{}", child.id())])
            .status();
    }
    #[cfg(windows)]
    {
        let _ = std::process::Command::new("taskkill")
            .args(["/F", "/T", "/PID", &child.id().to_string()])
            .status();
    }
    let _ = child.kill();
    let _ = child.wait();
}

#[cfg(test)]
#[path = "check_jobs_tests.rs"]
mod check_jobs_tests;
