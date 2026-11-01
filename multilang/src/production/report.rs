//! `xirang report`: read the evidence one build wrote and summarize its economics — how many
//! actions executed, how many came from an earlier run (reused) or from the content store
//! (restored), how much time the executed ones took, and how the tests ended. It reads only.
//! `xirang report`：读一次构建写下的证据，汇总它的经济性——多少动作真执行、多少复用了上一次运行
//! （reused）或从内容库恢复（restored）、真执行花了多少秒、测试结果。它只读。

use std::path::{Path, PathBuf};

/// Summarize one build's evidence. `plan` picks an explicit plan id; without it the newest build.
/// 汇总一次构建的证据。`plan` 指定计划 id；不给就用最新的一次构建。
pub(crate) fn summarize(root: &Path, plan: Option<&str>) -> Result<String, String> {
    let evidence = match plan {
        Some(id) => {
            let path = root
                .join("target/xirang/build")
                .join(id)
                .join("evidence.json");
            if !path.is_file() {
                return Err(format!(
                    "no build evidence for plan `{id}` at {}; `xirang report --plan <id>` reads a plan that was built",
                    path.display()
                ));
            }
            path
        }
        None => newest(&root.join("target/xirang/build"))?,
    };
    let text = std::fs::read_to_string(&evidence)
        .map_err(|error| format!("cannot read {}: {error}", evidence.display()))?;
    let document: serde_json::Value = serde_json::from_str(&text).map_err(|error| {
        format!(
            "{} is not a build evidence document: {error}",
            evidence.display()
        )
    })?;
    let actions = document["actions"].as_array().cloned().unwrap_or_default();
    let mut reused = 0usize;
    let mut restored = 0usize;
    let mut executed = 0usize;
    let mut seconds = 0.0f64;
    for action in &actions {
        let was_reused = action["reused"].as_bool().unwrap_or(false);
        let was_restored = action["restored"].as_bool().unwrap_or(false);
        if was_restored {
            restored += 1;
        } else if was_reused {
            reused += 1;
        } else {
            executed += 1;
            seconds += action["seconds"].as_f64().unwrap_or(0.0);
        }
    }
    let tests = document["tests"].as_array().cloned().unwrap_or_default();
    let counted = document["counts"]["tests"]["passed"]
        .as_u64()
        .zip(document["counts"]["tests"]["failed"].as_u64());
    let (passed, failed) = match counted {
        Some((passed, failed)) => (passed as usize, failed as usize),
        None => {
            let passed = tests
                .iter()
                .filter(|test| test["passed"].as_bool().unwrap_or(false))
                .count();
            (passed, tests.len().saturating_sub(passed))
        }
    };
    let mut out = String::new();
    out.push_str(&format!(
        "plan {}\n  result {}\n",
        document["plan"].as_str().unwrap_or("?"),
        document["result"].as_str().unwrap_or("?")
    ));
    out.push_str(&format!(
        "  actions {} total: {} executed ({seconds:.3} s), {reused} reused, {restored} restored\n",
        actions.len(),
        executed
    ));
    if let Some(errors) = document["reuse_errors"].as_array()
        && !errors.is_empty()
    {
        out.push_str(&format!(
            "  reuse refused for {n} action(s): {}\n",
            errors
                .iter()
                .filter_map(|error| error.as_str())
                .collect::<Vec<_>>()
                .join("; "),
            n = errors.len()
        ));
    }
    out.push_str(&format!("  tests {passed} passed, {failed} failed\n"));
    let failures = document["failed_actions"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    for failure in &failures {
        let program = failure["argv"]
            .as_array()
            .and_then(|argv| argv.first())
            .and_then(serde_json::Value::as_str)
            .unwrap_or("?");
        out.push_str(&format!(
            "  failed action: {} `{program}` (status {})\n",
            failure["kind"].as_str().unwrap_or("?"),
            failure["status"].as_i64().unwrap_or(-1)
        ));
    }
    if document["result"].as_str() == Some("failed") && failures.is_empty() && failed == 0 {
        out.push_str("  failure: the run failed and named no failing action or test\n");
    }
    Ok(out.trim_end().to_owned())
}

/// The newest evidence document under the build directory (by modification time).
/// 构建目录下最新的证据文档（按修改时间）。
fn newest(build: &Path) -> Result<PathBuf, String> {
    let entries = std::fs::read_dir(build).map_err(|error| {
        format!(
            "no build evidence under {}: {error}; run `xirang build` first",
            build.display()
        )
    })?;
    let mut found: Vec<(std::time::SystemTime, PathBuf)> = Vec::new();
    for entry in entries.flatten() {
        let evidence = entry.path().join("evidence.json");
        if !evidence.is_file() {
            continue;
        }
        let modified = evidence
            .metadata()
            .and_then(|metadata| metadata.modified())
            .unwrap_or(std::time::UNIX_EPOCH);
        found.push((modified, evidence));
    }
    found.sort();
    found.pop().map(|(_, path)| path).ok_or_else(|| {
        format!(
            "no build evidence under {}; run `xirang build` first",
            build.display()
        )
    })
}
