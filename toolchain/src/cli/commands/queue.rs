//! `xirang queue …`: a persistent queue of build/test jobs, run through a worker command.
//! `xirang queue …`：持久化的 build/test 任务队列，经 worker 命令运行。

use std::io::Write;
use std::path::Path;

/// `xirang queue add|list|show|cancel|run …`.
/// `xirang queue add|list|show|cancel|run …`。
pub(crate) fn queue(
    arguments: &mut dyn Iterator<Item = String>,
    out: &mut dyn Write,
) -> Result<(), String> {
    let Some(action) = arguments.next() else {
        return Err("`xirang queue` takes `add`, `list`, `show`, `cancel` or `run`".to_owned());
    };
    let mut state = None;
    let mut directory = None;
    let mut id = None;
    let mut worker = None;
    let mut jobs = 1usize;
    let mut locked = false;
    let mut test = false;
    let rest = arguments;
    while let Some(argument) = rest.next() {
        match argument.as_str() {
            "--state" => state = rest.next(),
            "--worker" => worker = rest.next(),
            "--jobs" => {
                let value = rest
                    .next()
                    .ok_or_else(|| "`--jobs` needs a number".to_owned())?;
                jobs = value
                    .parse()
                    .map_err(|_| format!("`{value}` is not a number of workers"))?;
            }
            "--locked" => locked = true,
            "--test" => test = true,
            other if other.starts_with('-') => {
                return Err(format!(
                    "`{other}` is not an argument of `xirang queue {action}`"
                ));
            }
            other => match action.as_str() {
                "add" => directory = Some(other.to_owned()),
                "show" | "cancel" => id = Some(other.to_owned()),
                _ => return Err(format!("unexpected argument `{other}`")),
            },
        }
    }
    let state = Path::new(state.as_deref().unwrap_or("target/xirang/queue"));
    match action.as_str() {
        "add" => {
            let directory = directory
                .ok_or_else(|| "`xirang queue add` needs a project directory".to_owned())?;
            let job_action = if test { "test" } else { "build" };
            let id = crate::multilang::queue_add(state, Path::new(&directory), job_action, locked)?;
            writeln!(
                out,
                "queued job `{id}` ({job_action}{}) in {}",
                if locked { ", --locked" } else { "" },
                state.display()
            )
            .map_err(|error| format!("cannot write: {error}"))
        }
        "list" => writeln!(out, "{}", crate::multilang::queue_list(state)?)
            .map_err(|error| format!("cannot write: {error}")),
        "show" => {
            let id = id.ok_or_else(|| "`xirang queue show` needs a job id".to_owned())?;
            writeln!(out, "{}", crate::multilang::queue_show(state, &id)?)
                .map_err(|error| format!("cannot write: {error}"))
        }
        "cancel" => {
            let id = id.ok_or_else(|| "`xirang queue cancel` needs a job id".to_owned())?;
            writeln!(out, "{}", crate::multilang::queue_cancel(state, &id)?)
                .map_err(|error| format!("cannot write: {error}"))
        }
        "run" => {
            let worker =
                worker.ok_or_else(|| "`xirang queue run` needs `--worker <command>`".to_owned())?;
            writeln!(
                out,
                "{}",
                crate::multilang::queue_run(state, &worker, jobs)?
            )
            .map_err(|error| format!("cannot write: {error}"))
        }
        other => Err(format!(
            "unknown queue action `{other}`; it takes `add`, `list`, `show`, `cancel` or `run`"
        )),
    }
}
