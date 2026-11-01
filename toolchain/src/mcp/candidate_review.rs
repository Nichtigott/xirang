//! The review packet, and who may record a verdict against it.
//! 审查材料包，以及谁可以对照它记录判定。
//!
//! A review is independent **context**, not a different name. This module therefore does two things
//! and refuses a third: it assembles the facts a reviewer needs — the original request, the
//! corrections made since, the acceptance criteria, the authorised boundary, the actual delta, the
//! verification the review binds to — into one packet with a digest; it records a verdict only
//! against that exact digest; and it has **no tool entry** that writes a verdict, because a surface
//! the developer can call is a surface that can write its own pass.
//! 审查靠的是独立的**上下文**，不是另一个名字。因此本模块做两件事、拒绝第三件：把评审者需要的事实
//! ——原始需求、此后发生的纠正、验收标准、授权边界、实际差异、评审所绑定的验证——拼成一份带摘要的
//! 材料包；只对照那个确切摘要记录判定；并且**没有**任何写判定的工具入口，因为开发者能调用的面就是
//! 能写自己那次"通过"的面。
//!
//! What it cannot decide is whether the host really started an independent reviewer. It does not
//! claim to: every reply says the record is the host entry's declaration, and the independence
//! belongs to the channel. What it *can* decide is whether the verdict still covers the packet: the
//! copy, the request, the corrections and the evidence all feed the digest, so any of them moving
//! makes an old pass stale by construction.
//! 它判定不了的是宿主是否真的启动了独立评审者，而它不声称能：每条回复都说这份记录是宿主入口的声明，
//! 独立性归通道所有。它**能**判定的是判定是否仍然覆盖那份材料包：副本、请求、纠正与证据都进入摘要，
//! 其中任何一个移动，旧"通过"就由构造决定地过期。

use std::path::Path;

use serde_json::{Value, json};

use super::Record;

/// The facts a review is about, as one canonical value.
/// 一次评审所针对的事实，作为一个规范值。
fn facts(record: &Record) -> Result<Value, String> {
    let tree = record.tree()?;
    let base = record
        .body
        .get("base_root")
        .and_then(Value::as_str)
        .map(Path::new)
        .ok_or("this candidate record has no base root; it cannot be reviewed")?;
    // The delta is part of the identity: a review of "the change" is a review of a diff, so the
    // changed **contents** feed the digest, not just the file names. A copy edited after the packet
    // was printed otherwise keeps the same key set and the same recorded verification identity, and
    // the verdict would silently cover work it never read.
    // 差异是身份的一部分：评审评的是"这次改动"，因此进入摘要的是**改动后的内容**，不只是文件名。否则材料包
    // 打印之后又被编辑的副本，键集合与记录的验证身份都不变，判定就会悄悄覆盖它从未读过的工作。
    let mut delta: Vec<Value> = Vec::new();
    if tree.exists() && base.exists() {
        for (path, change) in super::delta(&tree, base)? {
            match change {
                super::Change::Gone => delta.push(json!({"path": path, "state": "gone"})),
                super::Change::Written => {
                    let bytes = std::fs::read(tree.join(&path)).map_err(|error| {
                        format!("the changed file {path} cannot be read for the packet: {error}")
                    })?;
                    delta.push(json!({
                        "path": path,
                        "state": "written",
                        "digest": xirang_kernel::sha256_hex(&bytes),
                    }));
                }
            }
        }
    }
    let verification = &record.body["verification"];
    Ok(json!({
        "id": record.body["id"],
        "requirement": record.body["requirement"],
        "acceptance": record.body["acceptance"],
        "corrections": record.body["corrections"],
        "scope": record.body["scope"],
        "base_at_open": record.body["base_at_open"],
        "delta": delta,
        "verification": {
            "verdict": verification["verdict"],
            "executed": verification["executed"],
            "identity": verification["candidate"],
        },
        "require_review": record.body["require_review"],
    }))
}

/// The digest a verdict is bound to.
/// 判定所绑定的摘要。
pub(crate) fn digest(record: &Record) -> Result<String, String> {
    let text = serde_json::to_string(&facts(record)?).map_err(|error| error.to_string())?;
    Ok(xirang_kernel::sha256_hex(text.as_bytes()))
}

/// The packet the host hands to an independent reviewer: facts first, the author's story never.
/// 宿主交给独立评审者的材料包：先事实，从不带作者的说法。
pub(crate) fn packet(record: &Record) -> Result<String, String> {
    let digest = digest(record)?;
    let text = |value: &Value, fallback: &str| {
        value
            .as_str()
            .map(str::to_owned)
            .filter(|text| !text.trim().is_empty())
            .unwrap_or_else(|| fallback.to_owned())
    };
    let list = |value: &Value, fallback: &str| -> String {
        match value.as_array() {
            Some(rows) if !rows.is_empty() => rows
                .iter()
                .filter_map(Value::as_str)
                .map(|row| format!("  - {row}\n"))
                .collect(),
            _ => format!("  {fallback}\n"),
        }
    };
    let facts = facts(record)?;
    let delta = facts["delta"]
        .as_array()
        .map(|rows| {
            rows.iter()
                .filter_map(|row| row["path"].as_str())
                .collect::<Vec<_>>()
                .join(", ")
        })
        .filter(|text| !text.is_empty())
        .unwrap_or_else(|| "nothing (the copy is byte-identical to the base)".to_owned());
    let scope = record.scope().join(", ");
    let verification = &record.body["verification"];
    let verdict = text(&verification["verdict"], "none");
    let executed = verification["executed"]
        .as_u64()
        .map(|count| count.to_string())
        .unwrap_or_else(|| "unknown".to_owned());
    let identity = text(&verification["candidate"], "not verified yet");
    let base_root = text(&record.body["base_root"], "unknown");
    let id = text(&record.body["id"], "unknown");
    Ok(format!(
        "review packet {digest}\n\
         candidate {id}\n\
         base {base_root}\n\
         scope {scope}\n\
         changed {delta}\n\
         verification {verdict} ({executed} test(s) executed) at {identity}\n\
         \n\
         requirement (as the person stated it):\n  {requirement}\n\
         \n\
         acceptance criteria:\n{acceptance}\
         corrections recorded since the request:\n{corrections}\
         \n\
         how to review:\n\
         \x20 1. Judge from the requirement, the criteria and the evidence above **before** reading the\n\
         \x20    author's explanation of the implementation. The author's reasoning is not part of this\n\
         \x20    packet; reading it first is what turns a review into a proof of the author's understanding.\n\
         \x20 2. Look for counterexamples, at least in these categories: requirements that were missed;\n\
         \x20    upstream edits that are not the root cause; indirect consumers left untested; call-chain\n\
         \x20    detours around the fix; patches that cancel each other; tests that flatter the\n\
         \x20    implementation; and constraints that should have been corrected instead of kept as\n\
         \x20    project memory.\n\
         \x20 3. Add context through the tree's own tools and say what you used; do not ask the author to\n\
         \x20    cut the evidence down for you.\n\
         \x20 4. Record the verdict from the host review entry, against this packet:\n\
         \x20    xirang-mcp --review --root {base_root} --id {id} --packet {digest} --verdict pass|fail [--finding \"…\"]\n\
         \n\
         a verdict covers exactly this packet: if the copy, the request, the corrections or the evidence\n\
         move, the old verdict no longer covers it and a new packet is printed. This bridge's tool\n\
         surface cannot record a verdict; only the host review entry can\n",
        requirement = text(
            &record.body["requirement"],
            "not recorded — a review without it can only judge the author's account"
        ),
        acceptance = list(&record.body["acceptance"], "none recorded"),
        corrections = list(&record.body["corrections"], "none recorded"),
    ))
}

/// Record a verdict against the packet it was produced from.
/// 对照判定所依据的那份材料包记录判定。
pub(crate) fn record(
    directory: &Path,
    id: &str,
    verdict: &str,
    reviewer: Option<&str>,
    findings: &[String],
    packet: &str,
) -> Result<String, String> {
    if !matches!(verdict, "pass" | "fail") {
        return Err(format!(
            "review verdict is `pass` or `fail`, not `{verdict}`; an unstated verdict is not a review"
        ));
    }
    let mut record = Record::read_at(directory, id)?;
    let tree = record.tree()?;
    if !tree.exists() {
        return Err(format!(
            "candidate {id} has no copy left (its state is {}); a review is about a version, and there is no version to review",
            record.body["state"].as_str().unwrap_or("unknown")
        ));
    }
    if record.body["verification"]["candidate"].as_str().is_none() {
        return Err(format!(
            "candidate {id} has not been verified, so there is no version to review; run `verify` first, then print the packet again"
        ));
    }
    let now = digest(&record)?;
    if now != packet {
        return Err(format!(
            "this packet does not describe the candidate as it is now (packet {packet}, now {now}): the copy, the requirement, the corrections or the evidence moved since it was printed. A verdict about another version is a verdict about nothing; print the packet again and review that one"
        ));
    }
    let reviewer = reviewer
        .map(str::trim)
        .filter(|name| !name.is_empty() && name.len() <= 200);
    let candidate = record.body["verification"]["candidate"].clone();
    let review = json!({
        "verdict": verdict,
        "reviewer": reviewer,
        "findings": findings,
        "packet": now,
        "candidate": candidate,
        "verification_at": record.body["verification"]["at"],
        "at": super::now(),
        "channel": "host-review",
    });
    record.body["review"] = review.clone();
    if let Some(history) = record.body["review_history"].as_array_mut() {
        history.push(review);
    } else {
        record.body["review_history"] = json!([review]);
    }
    record.publish()?;
    let reviewer = reviewer.unwrap_or("an unnamed reviewer");
    let findings_line = if findings.is_empty() {
        "no findings were recorded".to_owned()
    } else {
        format!("{} finding(s) recorded", findings.len())
    };
    Ok(format!(
        "candidate {id} review {verdict} by {reviewer}\npacket {now}\n{findings_line}\nthis record is the host review entry's declaration: nothing here proves the reviewer was an independent context — that belongs to the channel that launched it — but it does bind the verdict to exactly this packet, so work that moves after it is unreviewed again\n"
    ))
}

/// What `land` checks: a failed review blocks, and a required one must be a pass on this packet.
/// `land` 检查的东西：失败的评审阻断，必需的评审必须是这份材料包上的通过。
pub(crate) fn gate(record: &Record) -> Result<(), String> {
    let current = digest(record)?;
    let review = &record.body["review"];
    let here = review["packet"].as_str() == Some(current.as_str());
    if review["verdict"].as_str() == Some("fail") && here {
        return Err(format!(
            "the last review of this candidate was `fail` by {} on this packet; a failed review is not delivered. Address the findings, verify the copy again, print the packet again and have it reviewed again",
            review["reviewer"].as_str().unwrap_or("an unnamed reviewer"),
        ));
    }
    if !record.body["require_review"].as_bool().unwrap_or(false) {
        return Ok(());
    }
    if review["verdict"].as_str() == Some("pass") && here {
        return Ok(());
    }
    let state = match review["verdict"].as_str() {
        None => "no review has been recorded".to_owned(),
        Some("fail") if here => "the review on record is `fail` on this packet".to_owned(),
        Some(verdict) => format!(
            "the review on record is `{verdict}` on another packet, so it no longer covers these bytes"
        ),
    };
    Err(format!(
        "this candidate was opened with `require_review: true`, and {state}: a required review is an obligation, not a note. The host starts an independent reviewer on the packet (`xirang-mcp --review --root <base> --id <candidate>` prints it) and records the verdict against the packet digest; `land` is available after a `pass` on the packet as it stands"
    ))
}

/// The status line: the packet, and where the verdict stands.
/// 状态行：材料包，以及判定停在哪里。
pub(crate) fn status(record: &Record) -> Result<String, String> {
    let review = &record.body["review"];
    let line = match review["verdict"].as_str() {
        Some(verdict) => {
            let reviewer = review["reviewer"].as_str().unwrap_or("an unnamed reviewer");
            let findings = review["findings"]
                .as_array()
                .map(|rows| rows.len())
                .unwrap_or(0);
            format!(
                "review {verdict} by {reviewer} on packet {} ({findings} finding(s); the record is the host entry's declaration, and independence belongs to the channel that launched the reviewer)",
                review["packet"].as_str().unwrap_or("nothing")
            )
        }
        None => "none on record; nothing here says anyone reviewed it".to_owned(),
    };
    Ok(format!("review {line}\n{}", packet(record)?))
}

#[cfg(test)]
#[path = "candidate_review_tests.rs"]
mod candidate_review_tests;
