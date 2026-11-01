//! Produce compiler evidence through the same command shape used by Studio.
//! 经 Studio 共用的命令形状产出编译器证据。
use serde_json::{Value, json};
use std::path::Path;
use std::time::Duration;

pub(super) fn capture(root: &Path, args: &Value) -> Result<String, String> {
    for key in args
        .as_object()
        .into_iter()
        .flat_map(|object| object.keys())
    {
        if ![
            "capture",
            "root",
            "package",
            "bin",
            "face",
            "no_default_features",
            "target",
            "timeout_ms",
        ]
        .contains(&key.as_str())
        {
            return Err(format!("unsupported MIR capture parameter {key}"));
        }
    }
    for key in ["package", "bin", "face", "target"] {
        if args.get(key).is_some_and(|v| {
            v.as_str().is_none_or(|s| {
                s.is_empty() || s.starts_with('-') || s.chars().any(char::is_control)
            })
        }) {
            return Err(format!("invalid MIR {key}"));
        }
    }
    if args
        .get("no_default_features")
        .is_some_and(|v| !v.is_boolean())
    {
        return Err("no_default_features must be boolean".into());
    }
    let timeout = match args.get("timeout_ms") {
        None => 900_000,
        Some(value) => value
            .as_u64()
            .filter(|n| (1000..=3_600_000).contains(n))
            .ok_or("timeout_ms must be 1000..3600000")?,
    };
    let root = root.canonicalize().map_err(|e| e.to_string())?;
    let relative = format!("target/xirang/mir/{}.mir", super::check_jobs::new_id());
    let path = root.join(&relative);
    // Resolve a first-run lockfile before recording source/configuration identity.
    // 首次运行先解析锁文件，再记录源码和配置身份。
    if !root.join("Cargo.lock").exists() {
        let log = path.with_extension("lock.log");
        let mut command = std::process::Command::new("cargo");
        command.arg("generate-lockfile").current_dir(&root);
        let result = super::check::run_command(&mut command, Duration::from_millis(timeout), &log)?;
        if result.code != Some(0) || result.timed_out {
            let text = super::check_log::read(&log)?.text;
            return Ok(format!(
                "verdict unknown\nMIR dependency resolution did not complete; no artifact certified\nlog {}\n{}",
                log.display(),
                super::check_diagnostics::compiler_lines(&text, false).join("\n")
            ));
        }
    }
    let before = super::check_plan::fingerprint(&root, None)?;
    let mut selectors = vec!["--lib"];
    if let Some(bin) = args.get("bin").and_then(Value::as_str) {
        selectors = vec!["--bin", bin];
    }
    for (key, flag) in [("package", "--package"), ("target", "--target")] {
        if let Some(value) = args.get(key).and_then(Value::as_str) {
            selectors.extend([flag, value]);
        }
    }
    match args
        .get("face")
        .and_then(Value::as_str)
        .unwrap_or("default")
    {
        "default" => {}
        "all" => selectors.push("--all-features"),
        face => selectors.extend(["--features", face]),
    }
    if args.get("no_default_features").and_then(Value::as_bool) == Some(true) {
        selectors.push("--no-default-features");
    }
    let emission = format!("--emit=mir={}", path.display());
    let mut command = crate::cargo_mir::command(&root.join("Cargo.toml"), &selectors, &[&emission]);
    command.current_dir(&root).env("CARGO_TERM_COLOR", "never");
    let invocation: Vec<_> = command
        .get_args()
        .map(|arg| arg.to_string_lossy().into_owned())
        .collect();
    let log_path = path.with_extension("log");
    let outcome =
        super::check::run_command(&mut command, Duration::from_millis(timeout), &log_path)?;
    if outcome.code != Some(0) || outcome.timed_out {
        let log = super::check_log::read(&log_path)?.text;
        return Ok(format!(
            "verdict {}\nMIR capture failed; no artifact certified\nlog {}\n{}",
            if outcome.timed_out {
                "unknown"
            } else {
                "failed"
            },
            log_path.display(),
            super::check_diagnostics::compiler_lines(&log, false).join("\n")
        ));
    }
    let after = super::check_plan::fingerprint(&root, None)?;
    let graph = xirang_kernel::mir::MirGraph::from_mir_text(
        &std::fs::read_to_string(&path).map_err(|e| e.to_string())?,
    );
    let identity = json!({"source_configuration":after,"stable":before == after,"argv":invocation,
        "artifact_digest":xirang_kernel::sha256_hex(&std::fs::read(&path).map_err(|e| e.to_string())?)});
    std::fs::write(path.with_extension("identity.json"), identity.to_string())
        .map_err(|e| e.to_string())?;
    Ok(format!(
        "verdict {}\npath {relative}\nfunctions {} calls {}\nsource_configuration {after}\ncompiler candidates only; in-root source/configuration checked; external dependencies and runtime input/dispatch remain unverified",
        if before == after && !graph.functions.is_empty() {
            "passed"
        } else {
            "unknown (inputs changed or empty MIR)"
        },
        graph.functions.len(),
        graph.calls.len()
    ))
}

pub(super) fn validate(root: &Path, path: &Path) -> Result<(), String> {
    let sidecar = path.with_extension("identity.json");
    if !sidecar.exists() {
        return Ok(());
    }
    let identity: Value =
        serde_json::from_slice(&std::fs::read(sidecar).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
    let digest = xirang_kernel::sha256_hex(&std::fs::read(path).map_err(|e| e.to_string())?);
    if identity["stable"] != true
        || identity["source_configuration"] != super::check_plan::fingerprint(root, None)?
        || identity["artifact_digest"] != digest
    {
        return Err("captured MIR is stale or changed; repeat mir {capture:true} for current compiler evidence".into());
    }
    Ok(())
}

#[cfg(test)]
#[path = "mir_capture_tests.rs"]
mod mir_capture_tests;
