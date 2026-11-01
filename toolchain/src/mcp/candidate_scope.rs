//! The scope side of a candidate: the smallest write set a change needs, and the ledger of widening.
//! 候选的"范围"那一半：一次改动所需的最小写集合，以及范围扩大的台账。
//!
//! Split out of `candidate.rs` for the size ratchet, along the seam the two halves already have: that
//! module owns **a candidate** (open, verify, land, status, discard) and this one owns the **task's**
//! view of scope — what a change would have to be allowed to touch, who else consumes it, and the
//! history that keeps a widening visible. §5 makes the second a property of the task and not of one
//! candidate: a grant that was widened and then narrowed again is still a widening.
//! 按尺寸棘轮从 `candidate.rs` 拆出，沿着这两半本来就有的那条缝：那个模块拥有**一棵候选**（open/verify/
//! land/status/discard），这一个拥有**任务**视角的范围——一次改动要获准触碰什么、还有谁在消费它、以及让
//! 扩大一直可见的历史。§5 让后者成为**任务**的性质而不是某棵候选的：一次被扩大、随后又收回的授权，仍然是
//! 一次扩大。

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use serde_json::{Value, json};

use super::{Change, Record, delta, in_scope, tree_root_of};

/// The smallest set of scope prefixes that covers every changed path.
/// 覆盖每一条改动路径的**最小**范围前缀集合。
///
/// One prefix per changed path — its directory, or the file itself when it sits at the root — with
/// any prefix that another already covers dropped. "Smallest" is the point: a proposal that answered
/// `["."]` every time would be an authorization for the whole tree wearing the word "minimal".
/// 每条改动路径一个前缀——它所在的目录，或在根上时就是文件本身——并丢掉已被另一个前缀覆盖的那些。"最小"是
/// 要点：一个每次都答 `["."]` 的提案，是一件写着"最小"二字的全树授权。
fn minimal_scope(paths: &BTreeSet<String>) -> Vec<String> {
    let mut prefixes: BTreeSet<String> = BTreeSet::new();
    for path in paths {
        match path.rsplit_once('/') {
            Some((directory, _)) if !directory.is_empty() => {
                prefixes.insert(directory.to_owned());
            }
            _ => {
                prefixes.insert(path.clone());
            }
        }
    }
    let all: Vec<String> = prefixes.iter().cloned().collect();
    all.iter()
        .filter(|prefix| {
            !all.iter()
                .any(|other| other != *prefix && prefix.starts_with(&format!("{other}/")))
        })
        .cloned()
        .collect()
}

/// Append one line to the scope ledger: what an attempt asked for, or what an opening authorised.
/// 往范围台账追加一行：一次尝试要了什么，或一次打开授权了什么。
///
/// Append-only and separate from the candidate records on purpose. A record describes one candidate;
/// the ledger describes the **task**, and §5's semantic account is a property of the task — a scope
/// that was widened and then narrowed again is still a widening.
/// 有意做成只追加、且与候选记录分开。记录描述的是一棵候选；台账描述的是**任务**，而 §5 的语义账是任务的性质
/// ——一次被扩大、随后又收回的范围，仍然是一次扩大。
pub(super) fn note_history(directory: &Path, line: &Value) -> Result<(), String> {
    std::fs::create_dir_all(directory).map_err(|error| error.to_string())?;
    crate::mcp::check_task::note(&directory.join("scope-history.jsonl"), line.clone())
}

/// Read the scope ledger back.
/// 读回范围台账。
fn history(directory: &Path) -> Vec<Value> {
    let path = directory.join("scope-history.jsonl");
    let Ok(text) = std::fs::read_to_string(&path) else {
        return Vec::new();
    };
    text.lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .collect()
}

/// The identity of one widening request: the task, the boundary it grew from, the increment and the
/// paths that made it necessary.
/// 一份扩大请求的身份：任务、它从中长出的边界、增量，以及逼出它的那些路径。
///
/// A request that differs in any of the four is a **different request**, and that is what makes an
/// old confirmation stop vouching for a newer change: the version it names no longer matches the one
/// the copy needs, so the decision has to be made again. The "second time outside the boundary" arm
/// is therefore decided by construction rather than by a flag somebody has to remember to clear.
/// 四者任一不同就是**另一份请求**，这正是让旧确认不再为更新的改动背书的东西：它点名的版本已经对不上副本需要的那一份，
/// 于是决定必须重做。因此"第二次越界"那一臂由构造决定，而不是靠谁记得去清一个标志。
pub(super) fn version_of(
    task: &str,
    current: &[String],
    increment: &[String],
    paths: &[String],
) -> String {
    let mut paths = paths.to_vec();
    paths.sort();
    xirang_kernel::sha256_hex(
        format!(
            "{task}|{}|{}|{}",
            current.join(","),
            increment.join(","),
            paths.join(",")
        )
        .as_bytes(),
    )[..16]
        .to_owned()
}

/// File the widening request this candidate's change needs, and say why it needs it.
/// 为这棵候选的改动立下它需要的扩大请求，并说出为什么需要。
///
/// A request is a **fact about the change**, not a permission. Filing it moves no boundary: the
/// declared scope still refuses the landing, and the only thing that moves it is a decision the
/// host's confirmation entry records against **this exact version**. That split is the maintainer's
/// correction of 2026-10-18: explain the overrun, and let the user decide whether the boundary grows.
/// 请求是**关于改动的事实**，不是许可。立下它不移动任何边界：声明的范围仍然拒绝落地，唯一能移动它的是宿主确认入口
/// 针对**这个确切版本**记下的决定。这个分工就是维护者 2026-10-18 的纠正：越权时说明原因，由使用方决定边界是否扩大。
pub(super) fn file_request(
    record: &mut Record,
    tree: &Path,
    changed: &BTreeMap<String, Change>,
) -> Result<Value, String> {
    let task = record.body["task"]
        .as_str()
        .unwrap_or("candidate")
        .to_owned();
    let id = record.body["id"].as_str().unwrap_or("?").to_owned();
    let current = record.scope();
    let paths: BTreeSet<String> = changed.keys().cloned().collect();
    let increment = minimal_scope(&paths);
    let outside: Vec<String> = paths
        .iter()
        .filter(|path| !in_scope(path, &current))
        .cloned()
        .collect();
    let version = version_of(&task, &current, &increment, &outside);
    // The same four facts are the same request: filing twice must not mint a second version, because
    // the version is how a decision stays attached to the text somebody read.
    // 同样的事实就是同一份请求：重复立不会铸出第二个版本，因为版本正是决定与被读过的文字绑在一起的方式。
    if record.body["broaden"]["version"].as_str() == Some(version.as_str()) {
        // The same facts are the same request. A request already awaiting a decision is returned as it
        // stands; one that was **decided** becomes pending again, because a refusal binds the boundary,
        // not the future — the work is still here, and the person who said no may say yes later. The
        // decision that was made stays in the history rather than being overwritten.
        // 同样的事实就是同一份请求。已经在等决定的那份原样返回；已经被**决定**过的那份重新变成待确认，因为拒绝绑住的是
        // 边界、不是将来——活儿还在那儿，先前说"不"的人以后可以说"是"。做过的决定留在历史里，而不是被覆盖。
        if record.body["broaden"]["state"].as_str() == Some("pending") {
            return Ok(record.body["broaden"].clone());
        }
        let mut request = record.body["broaden"].clone();
        request["state"] = json!("pending");
        request["refiled_at"] = json!(super::now());
        let mut history = record.body["broaden_history"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        history.push(record.body["broaden"].clone());
        record.body["broaden_history"] = Value::Array(history);
        record.body["broaden"] = request.clone();
        record.publish()?;
        return Ok(request);
    }
    let files = serde_json::to_value(&paths).unwrap_or(Value::Null);
    let shared: Option<Vec<String>> = match crate::mcp::check_protection::census(tree, &files) {
        Ok(Some(census)) => census.summary["shared_providers"].as_array().map(|rows| {
            rows.iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect::<Vec<_>>()
        }),
        _ => None,
    };
    let request = json!({
        "version": version,
        "at": super::now(),
        "state": "pending",
        "task": task,
        "candidate": id,
        "current": current,
        "increment": increment,
        "outside": outside,
        "reason": format!(
            "{} changed path(s) are outside the declared scope ({}): {}. The work is in the copy; landing it would write paths the boundary never named",
            outside.len(),
            if current.is_empty() { "none".to_owned() } else { current.join(", ") },
            outside.join(", "),
        ),
        "impact": {
            "change": changed.iter().map(|(path, change)| match change {
                Change::Written => format!("{path} (written)"),
                Change::Gone => format!("{path} (removed)"),
            }).collect::<Vec<_>>(),
            "shared_consumers": shared,
            "base_tree": "every path above is written into the base tree once the landing succeeds",
        },
        "alternatives": [
            {"kind": "open a candidate for it", "what": format!("open a candidate whose scope names {} and redo the change there; nothing from this copy lands", increment.join(", "))},
            {"kind": "narrow the change", "what": format!("take {} back out of the copy so the change stays inside the declared scope", outside.join(", "))},
            {"kind": "confirm the increment", "what": format!("the host's confirmation entry decides this request: xirang-mcp --confirm --root <path> --id {id} --accept --version {version} (or --reject)")},
        ],
    });
    // The request this one replaces stays readable: "the boundary was once asked to grow by X and that
    // request is gone" is a fact a reader of a refusal needs, and overwriting it would leave only the
    // last version on disk.
    // 被它取代的那份请求仍然可读："边界曾被要求增长 X、而那份请求已经不在了"是读拒绝的人需要的事实；覆盖它会让磁盘上
    // 只剩最后一个版本。
    if record.body["broaden"].is_object() {
        let mut history = record.body["broaden_history"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        history.push(record.body["broaden"].clone());
        record.body["broaden_history"] = Value::Array(history);
    }
    record.body["broaden"] = request.clone();
    record.publish()?;
    Ok(request)
}

/// The request as a person reads it: what is being asked, why, what it touches, and how to answer.
/// 一个人读到的请求：在申请什么、为什么、牵动什么、以及怎么回答。
pub(crate) fn describe(request: &Value, root: &Path, id: &str) -> String {
    let list = |value: &Value| {
        value
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .collect::<Vec<_>>()
            .join(", ")
    };
    let version = request["version"].as_str().unwrap_or("?");
    let alternatives = request["alternatives"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|row| {
            format!(
                "  - {}: {}",
                row["kind"].as_str().unwrap_or("?"),
                row["what"].as_str().unwrap_or("?")
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "widening request {version} ({state})\n\
         current scope       {current}\n\
         requested increment {increment}\n\
         reason              {reason}\n\
         change              {changed}\n\
         shared consumers    {shared}\n\
         alternatives\n{alternatives}\n\
         decide with        xirang-mcp --confirm --root {root} --id {id} --accept --version {version}\n\
         or                 xirang-mcp --confirm --root {root} --id {id} --reject --version {version}\n\
         the boundary is unchanged while this is `pending`: nothing lands, and no tool in this bridge can record a decision\n",
        state = request["state"].as_str().unwrap_or("unknown"),
        current = list(&request["current"]),
        increment = list(&request["increment"]),
        reason = request["reason"].as_str().unwrap_or("?"),
        changed = list(&request["impact"]["change"]),
        shared = match request["impact"]["shared_consumers"].as_array() {
            Some(rows) => {
                let names = rows
                    .iter()
                    .filter_map(Value::as_str)
                    .collect::<Vec<_>>()
                    .join(", ");
                if names.is_empty() {
                    "none this rule can see".to_owned()
                } else {
                    names
                }
            }
            None => "unknown (no census was taken)".to_owned(),
        },
        root = root.display(),
    )
}

/// Which way a person answered a widening request.
/// 一个人对扩大请求的回答是哪一种。
pub(crate) enum Decision {
    /// The boundary grows by the requested increment; the verification is cleared because the
    /// obligations grew, so the next step is `verify` and then `land`.
    /// 边界按请求的增量增长；因为义务变多，验证被清空，下一步是 `verify` 再 `land`。
    Accept,
    /// The boundary does not move, and the refusal is on record.
    /// 边界不动，拒绝入账。
    Reject,
}

/// Record the host's decision on one request, and move the boundary only when it was an acceptance.
/// 记下宿主对一份请求的决定；只有接受时才移动边界。
///
/// Three refusals, all before anything is written: no pending request, a version that no longer
/// describes what the copy needs, and — from the caller — a grant that could not be issued for the
/// widened scope. The boundary moves only after the grant exists, so there is no state in which the
/// scope names something no approval covers.
/// 三条拒绝，都在写任何东西之前：没有待确认的请求、版本已不再描述副本的所需、以及调用方签不出覆盖扩大后范围的授权。
/// 边界只在授权存在之后移动，因此不存在"范围点名的东西没有批准覆盖"的中间态。
pub(crate) fn decide<F>(
    directory: &Path,
    id: &str,
    record: &mut Record,
    version: &str,
    decision: Decision,
    note: Option<&str>,
    issue: F,
) -> Result<String, String>
where
    F: FnOnce(&[String], &str) -> Result<xirang_kernel::maintenance::Grant, String>,
{
    let pending = record.body["broaden"].clone();
    if pending["state"].as_str() != Some("pending") {
        return Err(format!(
            "candidate {id} has no pending widening request (the last one is `{}`); there is nothing to decide",
            pending["state"].as_str().unwrap_or("absent")
        ));
    }
    if pending["version"].as_str() != Some(version) {
        return Err(format!(
            "the request on record is {}, not {version}: the change moved after it was read, so this decision would answer a request nobody saw. Run `land` or `widen` again and decide the version printed then",
            pending["version"].as_str().unwrap_or("absent")
        ));
    }
    let strings = |value: &Value| -> Vec<String> {
        value
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .map(str::to_owned)
            .collect()
    };
    let increment = strings(&pending["increment"]);
    match decision {
        Decision::Reject => {
            record.body["broaden"]["state"] = json!("refused");
            record.body["broaden"]["decided_at"] = json!(super::now());
            record.body["broaden"]["decided_note"] = json!(note);
            record.publish()?;
            note_history(
                directory,
                &json!({"kind": "refused", "candidate": id, "refused": increment, "version": version}),
            )?;
            Ok(format!(
                "widening request {version} refused\ncurrent scope {}\nthe boundary is unchanged: nothing was added, and the landing still refuses every path outside it\n",
                record.scope().join(", ")
            ))
        }
        Decision::Accept => {
            let mut widened = record.scope();
            for prefix in &increment {
                if !widened.contains(prefix) {
                    widened.push(prefix.clone());
                }
            }
            let proposal = record.body["proposal"]
                .as_str()
                .unwrap_or("1")
                .parse::<u64>()
                .unwrap_or(1)
                + 1;
            let proposal = proposal.to_string();
            // The grant is issued **before** the boundary moves: a boundary that grew without an
            // approval behind it would be exactly the authorization-by-paperwork this surface exists
            // to refuse.
            // 授权在边界移动**之前**签发：一次没有批准在背后的边界增长，正是这个面存在的理由所要拒绝的纸面授权。
            let grant = issue(&widened, &proposal)?;
            record.body["scope"] = json!(widened);
            record.body["proposal"] = json!(proposal);
            record.body["grant_revision"] = json!(grant.field("revision").unwrap_or("0"));
            record.body["verification"] = Value::Null;
            record.body["broaden"]["state"] = json!("confirmed");
            record.body["broaden"]["decided_at"] = json!(super::now());
            record.body["broaden"]["decided_note"] = json!(note);
            let mut confirmations = record.body["scope_confirmations"]
                .as_array()
                .cloned()
                .unwrap_or_default();
            confirmations.push(json!({
                "version": version,
                "at": super::now(),
                "increment": increment,
                "proposal": proposal,
                "grant": grant.digest(),
            }));
            record.body["scope_confirmations"] = Value::Array(confirmations);
            record.publish()?;
            note_history(
                directory,
                &json!({"kind": "widened", "candidate": id, "scope": increment, "version": version, "by": "host-confirmation", "proposal": proposal}),
            )?;
            Ok(format!(
                "widening request {version} confirmed\nold scope {}\nadded {}\nnew scope {}\nproposal {proposal}\n{}\nthe verification was cleared because the obligations grew: run `verify` again, then `land` under the widened scope\n",
                strings(&pending["current"]).join(", "),
                increment.join(", "),
                record.scope().join(", "),
                crate::mcp::controller::line(&grant),
            ))
        }
    }
}

/// `candidate {action:"widen"}`: file the widening request a change needs, without moving anything.
/// `candidate {action:"widen"}`：为这次改动立下它需要的扩大请求，不移动任何东西。
///
/// The old shape took `confirm: true` and applied the widening itself — a value the caller wrote. The
/// maintainer's correction of 2026-10-18 removed that: an agent tool call cannot mark its own request
/// as confirmed by the user. What this action can do is describe the request properly, and that is
/// all it does; the decision belongs to the host's confirmation entry.
/// 旧形状收一个 `confirm: true` 并自己把扩大落下去——那是调用方自己写的值。维护者 2026-10-18 的纠正把它去掉了：
/// agent 的工具调用不能把自己的请求标成"用户已确认"。这个动作能做的是把请求描述清楚，而它只做这件事；决定属于宿主
/// 确认入口。
pub(super) fn widen(root: &Path, arguments: &Value, directory: &Path) -> Result<String, String> {
    let id = arguments
        .get("id")
        .and_then(Value::as_str)
        .ok_or("candidate widen needs `id`")?;
    let mut record = Record::read_at(directory, id)?;
    let tree = record.tree()?;
    if !tree.exists() {
        return Err(format!(
            "candidate {id} has no copy left (its state is {}); there is no change left to request a wider boundary for",
            record.body["state"].as_str().unwrap_or("unknown")
        ));
    }
    let changed = delta(&tree, &tree_root_of(&record)?)?;
    let outside: Vec<&String> = changed
        .keys()
        .filter(|path| !in_scope(path, &record.scope()))
        .collect();
    if outside.is_empty() {
        return Err(format!(
            "candidate {id} changed nothing outside its declared scope; a widening request that asks for nothing is a request nobody needs to answer"
        ));
    }
    let request = file_request(&mut record, &tree, &changed)?;
    Ok(format!("{}\n", describe(&request, root, id)))
}

/// The smallest scope that would cover what this candidate changed, and who consumes it.
/// 覆盖这棵候选实际改动的那个最小范围，以及谁在消费它。
///
/// §7 asks an upstream proposal to carry its evidence, and the two pieces of evidence that are
/// decidable here are exactly these: the minimal write set, and the **shared consumers** of what
/// changed — because "this is the upstream root cause" is a claim about a fan-in, and a proposal that
/// did not name the fan-in would be asking to be believed.
/// §7 要求上游提案带上它的证据，而这里可判定的证据恰好是这两样：最小写集合，与改动之物的**共同消费者**
/// ——因为"这是上游根因"是一句关于扇入的主张，而不点名扇入的提案是在要求被相信。
pub(super) fn propose(arguments: &Value, directory: &Path) -> Result<String, String> {
    let id = arguments
        .get("id")
        .and_then(Value::as_str)
        .ok_or("candidate propose needs `id`")?;
    let record = Record::read_at(directory, id)?;
    let tree = record.tree()?;
    if !tree.exists() {
        return Err(format!(
            "candidate {id} has no copy left (its state is {}); a proposal is about what a candidate changed, and there is nothing to read",
            record.body["state"].as_str().unwrap_or("unknown")
        ));
    }
    let scope = record.scope();
    let changed: BTreeMap<String, Change> = delta(&tree, &tree_root_of(&record)?)?;
    let paths: BTreeSet<String> = changed.keys().cloned().collect();
    let outside: Vec<&String> = paths
        .iter()
        .filter(|path| !in_scope(path, &scope))
        .collect();
    let minimal = minimal_scope(&paths);

    let mut output = format!("candidate {id} proposal\n");
    output.push_str(&format!(
        "changed {} path(s): {}\n",
        changed.len(),
        if changed.is_empty() {
            "none — the copy is byte-identical to the base, so there is nothing to authorise"
                .to_owned()
        } else {
            changed
                .iter()
                .map(|(path, change)| match change {
                    Change::Written => format!("{path} (written)"),
                    Change::Gone => format!("{path} (removed)"),
                })
                .collect::<Vec<_>>()
                .join(", ")
        }
    ));
    output.push_str(&format!(
        "declared scope {}; outside it: {}\n",
        if scope.is_empty() {
            "none".to_owned()
        } else {
            scope.join(", ")
        },
        if outside.is_empty() {
            "nothing — this candidate only needs `land`".to_owned()
        } else {
            outside
                .iter()
                .map(|path| path.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        }
    ));
    output.push_str(&format!(
        "smallest scope covering every changed path: {}\n",
        if minimal.is_empty() {
            "nothing to authorise".to_owned()
        } else {
            minimal.join(", ")
        }
    ));
    if !minimal.is_empty() {
        output.push_str(&format!(
            "ready to paste: xirang.candidate {{\"action\":\"open\",\"scope\":{}}}\n",
            serde_json::to_string(&minimal).unwrap_or_else(|_| "[]".to_owned())
        ));
    }

    // The fan-in evidence: who else calls what this candidate changed.
    // 扇入证据：还有谁在调用这棵候选改过的东西。
    let files = serde_json::to_value(&paths).unwrap_or(Value::Null);
    match crate::mcp::check_protection::census(&tree_root_of(&record)?, &files) {
        Ok(Some(census)) => {
            let shared = census.summary["shared_providers"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
                .collect::<Vec<_>>();
            output.push_str(&format!(
                "shared consumers of what changed: {}\n",
                if shared.is_empty() {
                    "none this rule can see: no changed file defines a function another file calls"
                        .to_owned()
                } else {
                    shared.join(", ")
                }
            ));
            if let Some(uncovered) = census.summary["uncovered"].as_array()
                && !uncovered.is_empty()
            {
                output.push_str(&format!(
                    "  without a test calling them by name: {}\n",
                    uncovered
                        .iter()
                        .filter_map(Value::as_str)
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
            }
        }
        Ok(None) => {
            output.push_str("shared consumers of what changed: not computed (nothing was named)\n")
        }
        Err(error) => output.push_str(&format!(
            "shared consumers of what changed: unknown ({error})\n"
        )),
    }

    let ledger = history(directory);
    let mut authorised: BTreeSet<String> = BTreeSet::new();
    let mut attempted: BTreeSet<String> = BTreeSet::new();
    for line in &ledger {
        let target = match line["kind"].as_str() {
            Some("authorised") | Some("widened") => &mut authorised,
            Some("attempted") => &mut attempted,
            _ => continue,
        };
        for key in ["scope", "refused"] {
            for entry in line[key].as_array().into_iter().flatten() {
                if let Some(text) = entry.as_str() {
                    target.insert(text.to_owned());
                }
            }
        }
    }
    output.push_str(&format!(
        "scope ledger ({} line(s)): authorised {}; attempted beyond it {}\n",
        ledger.len(),
        if authorised.is_empty() {
            "nothing yet".to_owned()
        } else {
            authorised.iter().cloned().collect::<Vec<_>>().join(", ")
        },
        if attempted.is_empty() {
            "nothing".to_owned()
        } else {
            attempted.iter().cloned().collect::<Vec<_>>().join(", ")
        },
    ));
    output.push_str(
        "this is a **scope**, not an authorisation: it says which paths would have to be writable, not that they may be. Opening a candidate with it is a new decision, and the ledger above keeps the widening visible\n",
    );
    Ok(output)
}
