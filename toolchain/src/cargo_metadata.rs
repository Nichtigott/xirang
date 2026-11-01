//! Cargo remains the metadata authority; requests share a bounded execution budget and results.
//! Cargo 保持元数据权威；请求共用有界执行预算与结果。
use serde_json::Value;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
#[cfg(feature = "mcp")]
use std::sync::{Arc, atomic::AtomicBool};
use std::time::{Duration, Instant};

struct Request {
    started: Instant,
    results: BTreeMap<PathBuf, Result<Value, String>>,
}
thread_local! { static REQUEST: RefCell<Vec<Request>> = const { RefCell::new(Vec::new()) }; }
#[cfg(feature = "mcp")]
thread_local! { static CANCEL: RefCell<Option<Arc<AtomicBool>>> = const { RefCell::new(None) }; }
#[cfg(feature = "mcp")]
pub(crate) struct Cancellation(Option<Arc<AtomicBool>>);
#[cfg(feature = "mcp")]
pub(crate) fn cancellation(flag: Arc<AtomicBool>) -> Cancellation {
    Cancellation(CANCEL.with(|slot| slot.replace(Some(flag))))
}
#[cfg(feature = "mcp")]
impl Drop for Cancellation {
    fn drop(&mut self) {
        CANCEL.with(|slot| *slot.borrow_mut() = self.0.take());
    }
}
fn cancelled() -> bool {
    #[cfg(feature = "mcp")]
    {
        CANCEL.with(|slot| {
            slot.borrow()
                .as_ref()
                .is_some_and(|flag| flag.load(Ordering::Acquire))
        })
    }
    #[cfg(not(feature = "mcp"))]
    {
        false
    }
}
pub(crate) struct Scope;
pub(crate) fn scope() -> Scope {
    REQUEST.with(|request| {
        request.borrow_mut().push(Request {
            started: Instant::now(),
            results: BTreeMap::new(),
        })
    });
    Scope
}
impl Drop for Scope {
    fn drop(&mut self) {
        REQUEST.with(|request| {
            request.borrow_mut().pop();
        });
    }
}

pub(crate) fn read(manifest: &Path) -> Result<Value, String> {
    if cancelled() {
        return Err("metadata request cancelled; identity unknown".into());
    }
    let manifest = manifest
        .canonicalize()
        .map_err(|e| format!("cargo manifest {}: {e}", manifest.display()))?;
    let cached = REQUEST.with(|request| {
        request
            .borrow()
            .last()
            .and_then(|request| request.results.get(&manifest).cloned())
    });
    if let Some(result) = cached {
        return result;
    }
    let timeout = REQUEST.with(|request| {
        request
            .borrow()
            .last()
            .map_or(Duration::from_secs(120), |request| {
                Duration::from_secs(120).saturating_sub(request.started.elapsed())
            })
    });
    if timeout.is_zero() {
        return Err(
            "Cargo metadata request budget exhausted (120 seconds); identity unknown".into(),
        );
    }
    let result = execute(&manifest, timeout);
    REQUEST.with(|request| {
        if let Some(request) = request.borrow_mut().last_mut() {
            if let Ok(metadata) = &result {
                for package in metadata["packages"].as_array().into_iter().flatten() {
                    if let Some(path) = package["manifest_path"].as_str() {
                        request.results.insert(PathBuf::from(path), result.clone());
                    }
                }
                if let Some(root) = metadata["workspace_root"].as_str() {
                    request
                        .results
                        .insert(Path::new(root).join("Cargo.toml"), result.clone());
                }
            }
            request.results.insert(manifest, result.clone());
        }
    });
    result
}

fn execute(manifest: &Path, timeout: Duration) -> Result<Value, String> {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let directory = std::env::temp_dir().join(format!(
        "xirang-metadata-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir(&directory).map_err(|e| e.to_string())?;
    struct Cleanup(PathBuf);
    impl Drop for Cleanup {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    let _cleanup = Cleanup(directory.clone());
    let stdout = directory.join("out");
    let stderr = directory.join("err");
    let mut command = Command::new("cargo");
    command
        .args([
            "metadata",
            "--format-version",
            "1",
            "--no-deps",
            "--manifest-path",
        ])
        .arg(manifest)
        .stdin(Stdio::null())
        .stdout(std::fs::File::create(&stdout).map_err(|e| e.to_string())?)
        .stderr(std::fs::File::create(&stderr).map_err(|e| e.to_string())?);
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    let mut child = command
        .spawn()
        .map_err(|e| format!("cannot run cargo metadata: {e}"))?;
    let started = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if started.elapsed() < timeout && !cancelled() => {
                std::thread::sleep(Duration::from_millis(10))
            }
            status => {
                #[cfg(unix)]
                {
                    let _ = Command::new("kill")
                        .args(["-KILL", "--", &format!("-{}", child.id())])
                        .status();
                }
                #[cfg(windows)]
                {
                    let _ = Command::new("taskkill")
                        .args(["/F", "/T", "/PID", &child.id().to_string()])
                        .status();
                }
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!(
                    "cargo metadata timed out, cancelled, or wait failed for {}: {status:?}; identity unknown",
                    manifest.display()
                ));
            }
        }
    };
    if !status.success() {
        use std::io::Read;
        let mut detail = String::new();
        let _ = std::fs::File::open(stderr)
            .and_then(|file| file.take(4000).read_to_string(&mut detail));
        return Err(format!(
            "cargo metadata failed for {}: {detail}",
            manifest.display()
        ));
    }
    if std::fs::metadata(&stdout).map_err(|e| e.to_string())?.len() > 64 * 1024 * 1024 {
        return Err("Cargo metadata exceeds 64 MiB; identity unknown".into());
    }
    serde_json::from_reader(std::fs::File::open(stdout).map_err(|e| e.to_string())?)
        .map_err(|e| format!("cannot read cargo metadata output: {e}"))
}
