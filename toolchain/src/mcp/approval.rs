//! The host's confirmation entry: the one place a decision about a boundary is recorded.
//! 宿主确认入口：关于边界的决定被记下来的唯一地方。
//!
//! **Corrected 2026-10-18 by the maintainer.** This used to demand an interactive terminal and a typed
//! name, and treat both as evidence that a person approved. Neither is evidence of a person: a process
//! that allocates a pseudo-terminal can answer a prompt, and a name is a string. What the requirement
//! actually serves was never identity — it was that **an out-of-bounds change is explained, and the
//! user decides whether the modification boundary grows**. So the entry now confirms *a request*: it
//! prints the version, the current scope, the requested increment, the reason, the impact and the
//! alternatives, and it records a decision on that exact version. No name, no terminal check.
//! **2026-10-18 维护者纠正。** 这里过去要求交互终端并手输姓名，并把两者当作"有人批准"的证据。两者都不是人的证据：
//! 能分配伪终端的进程可以回答提示符，而姓名只是一个字符串。这条要求真正服务的从来不是身份，而是**越权改动必须说明
//! 原因、由使用方决定修改边界是否扩大**。因此这个入口现在确认的是**一份请求**：印出版本、当前范围、请求增量、理由、
//! 影响与替代方案，并针对那个确切版本记下决定。没有姓名，也没有终端检查。
//!
//! What it cannot verify is **who** ran it, and it says so, in its output and in the record: the entry
//! is "the host's confirmation entry recorded this decision", not "this person did". Anything stronger
//! needs a principal the agent cannot reach, and that is a property of the host, not something this
//! binary can manufacture. When the host has no such entry, the request simply stays `pending`.
//! 它无法核实的是**是谁**运行的，并且它在输出与记录里都说出这一点：记录是"宿主确认入口记下了这个决定"，不是"这个人做的"。
//! 更强的保证需要一个 agent 够不到的主体，而那是宿主的性质，不是这个二进制能凭空造出来的。宿主没有这样的入口时，
//! 请求就保持 `pending`。
//!
//! **No tool reaches it.** The catalogue has no confirmation entry, and the pin
//! `no_tool_in_this_bridge_can_issue_an_approval` keeps `controller::issue` reachable from this file
//! alone — so an agent's tool call cannot mark its own request as confirmed.
//! **没有任何工具到得了它。** 目录里没有确认条目，而钉子 `no_tool_in_this_bridge_can_issue_an_approval` 让
//! `controller::issue` 只从这个文件可达——因此 agent 的工具调用无法把自己的请求标成已确认。

use std::path::Path;

use serde_json::Value;
use xirang_kernel::maintenance::{Budget, Grant};

use super::candidate::scope::{self, Decision, decide};
use super::candidate::{Record, delta, tree_root_of};
use super::controller;

/// Read the flag `name` and the value after it.
/// 读取参数 `name` 以及它后面的那个值。
fn flag<'a>(arguments: &'a [String], name: &str) -> Option<&'a str> {
    arguments
        .iter()
        .position(|argument| argument == name)
        .and_then(|position| arguments.get(position + 1))
        .map(String::as_str)
}

/// Every value given for a repeatable flag, in order.
/// 一个可重复旗标给出的每一个取值，按顺序。
fn values<'a>(arguments: &'a [String], name: &str) -> Vec<&'a str> {
    arguments
        .iter()
        .enumerate()
        .filter(|(_, argument)| argument.as_str() == name)
        .filter_map(|(position, _)| arguments.get(position + 1).map(String::as_str))
        .collect()
}

/// The confirmation entry. **Called from argv dispatch only** — see the module doc and its pin.
/// 确认入口。**只被参数分派调用**——见模块文档与它的钉子。
pub(super) fn confirm(arguments: &[String]) -> i32 {
    match run_confirmation(arguments) {
        Ok(text) => {
            print!("{text}");
            0
        }
        Err(error) => {
            eprintln!("{error}");
            2
        }
    }
}

/// The host review entry: print the packet, or record a verdict against it.
/// 宿主评审入口：打印材料包，或对照它记录判定。
///
/// It exists here, beside the confirmation entry, for the same reason: a verdict recorded by a tool
/// the developer can call is a verdict the developer can write. The tool surface only prints the
/// packet (`status`), and this entry records **against the packet digest**, so a verdict cannot
/// silently outlive the work it judged.
/// 它在这里、与确认入口并排，理由相同：开发者能调用的工具记下的判定就是开发者能写的判定。工具面只打印材料包
/// （`status`），而本入口**对照材料包摘要**记录，因此判定的寿命不会悄悄长过它所评判的工作。
pub(super) fn review(arguments: &[String]) -> i32 {
    match run_review(arguments) {
        Ok(text) => {
            print!("{text}");
            0
        }
        Err(error) => {
            eprintln!("{error}");
            2
        }
    }
}

/// Print the packet, add a correction, or record a verdict against one packet.
/// 打印材料包、追加一条纠正，或对照某一份材料包记录判定。
fn run_review(arguments: &[String]) -> Result<String, String> {
    let root = flag(arguments, "--root").ok_or(
        "the review entry needs `--root <path>`: a review is about a candidate in one project",
    )?;
    let id = flag(arguments, "--id")
        .ok_or("the review entry needs `--id <candidate>`: the verdict is about one candidate")?;
    let root = Path::new(root).canonicalize().map_err(|error| {
        format!("`--root` must name the project the candidate was opened in: {error}")
    })?;
    let directory = flag(arguments, "--target-dir")
        .map(|value| root.join(value))
        .unwrap_or_else(|| root.join("target"))
        .join("xirang/candidates");
    let mut record = Record::read_at(&directory, id)?;
    // A correction is a review **input**, not a note kept on the side: the packet carries the
    // corrections so the reviewer judges the request as it stands now, not as it was first written.
    // 纠正是评审的**输入**而不是另存一边的注记：材料包带着纠正，评审者判的是现在这份需求，而不是最初写下的那份。
    if let Some(correction) = flag(arguments, "--correction") {
        let text = correction.trim();
        if text.is_empty() {
            return Err("`--correction` needs the correction as it was stated".to_owned());
        }
        match record.body["corrections"].as_array_mut() {
            Some(rows) => rows.push(Value::String(text.to_owned())),
            None => record.body["corrections"] = serde_json::json!([text]),
        }
        record.publish()?;
    }
    let findings: Vec<String> = arguments
        .iter()
        .enumerate()
        .filter(|(_, argument)| *argument == "--finding")
        .filter_map(|(position, _)| arguments.get(position + 1).cloned())
        .collect();
    match (flag(arguments, "--verdict"), flag(arguments, "--packet")) {
        (None, None) => super::candidate::review::packet(&record),
        (Some(verdict), Some(packet)) => super::candidate::review::record(
            &directory,
            id,
            verdict,
            flag(arguments, "--reviewer"),
            &findings,
            packet,
        ),
        _ => Err(
            "recording a review needs both `--verdict pass|fail` and `--packet <digest>`: a verdict that does not say which packet it read is a verdict about nothing".to_owned(),
        ),
    }
}

/// The one path through: read the decision, print what it decides, record it.
/// 唯一那条路：读决定、印出它在决定什么、记下来。
fn run_confirmation(arguments: &[String]) -> Result<String, String> {
    let root = flag(arguments, "--root")
        .ok_or("the confirmation entry needs `--root <path>`: the decision is about one project")?;
    let id = flag(arguments, "--id").ok_or(
        "the confirmation entry needs `--id <candidate>`: the decision is about one request",
    )?;
    let root = Path::new(root).canonicalize().map_err(|error| {
        format!("`--root` must name the project the candidate was opened in: {error}")
    })?;
    let directory = flag(arguments, "--target-dir")
        .map(|value| root.join(value))
        .unwrap_or_else(|| root.join("target"))
        .join("xirang/candidates");
    let note = flag(arguments, "--note");
    let accept = arguments.iter().any(|argument| argument == "--accept");
    let reject = arguments.iter().any(|argument| argument == "--reject");
    let land = arguments.iter().any(|argument| argument == "--land");
    if [accept, reject, land]
        .iter()
        .filter(|chosen| **chosen)
        .count()
        != 1
    {
        return Err(
            "the confirmation entry decides exactly one thing: `--accept` (grow the boundary by the pending request), `--reject` (keep it), or `--land` (authorise landing what is already inside it)"
                .to_owned(),
        );
    }
    let mut record = Record::read_at(&directory, id)?;
    if land {
        return authorise_landing(
            &root,
            id,
            &record,
            note,
            flag(arguments, "--budget"),
            &values(arguments, "--protect"),
            &values(arguments, "--allow-removed-test"),
        );
    }
    let version = flag(arguments, "--version").ok_or(
        "the confirmation entry needs `--version <the version the request printed>`: a decision is about one exact request, and a version the person did not read would confirm a request nobody saw",
    )?;
    let printed = format!(
        "project {}\n{}\n",
        root.display(),
        scope::describe(&record.body["broaden"], &root, id),
    );
    let identity = record.body.clone();
    let planned = match flag(arguments, "--budget") {
        Some(text) => Budget::parse_limits(text)?,
        None => landing_budget(&record),
    };
    let decision = if accept {
        Decision::Accept
    } else {
        Decision::Reject
    };
    let decided = decide(
        &directory,
        id,
        &mut record,
        version,
        decision,
        note,
        |widened, proposal| issue(&root, id, &identity, widened, proposal, planned),
    )?;
    Ok(format!(
        "{printed}{decided}\nrecorded by: the host's confirmation entry (this binary cannot verify who ran it; no tool in this bridge can create one)\n"
    ))
}

/// Authorise landing a candidate that is **already inside its boundary**.
/// 授权落地一棵**已经在边界之内**的候选。
///
/// This is the ordinary case — a small repair that never left its scope — and it is still a decision:
/// the entry prints the project, the candidate, the scope, the changed paths and the verification
/// state, then writes one grant bound to all of them. It does not read a name, because a name would
/// only be a declaration.
/// 这是普通情形——一次从未越界的小修复——而它仍然是一个决定：入口印出项目、候选、范围、改动路径与验证状态，然后写下
/// 一份把它们全绑上的授权。它不读姓名，因为姓名只会是一份声明。
fn authorise_landing(
    root: &Path,
    id: &str,
    record: &Record,
    note: Option<&str>,
    requested_budget: Option<&str>,
    protect: &[&str],
    allow_removed: &[&str],
) -> Result<String, String> {
    if record.body["broaden"]["state"].as_str() == Some("pending") {
        return Err(format!(
            "candidate {id} has a pending widening request; decide it first (`--accept --version {}`) — authorising the landing would leave the request unanswered and the landing would still be refused",
            record.body["broaden"]["version"].as_str().unwrap_or("?")
        ));
    }
    let scope: Vec<String> = record.scope();
    if scope.is_empty() {
        return Err(format!(
            "candidate {id} declares no scope; there is nothing to authorise"
        ));
    }
    let tree = record.tree()?;
    let changed = if tree.exists() {
        delta(&tree, &tree_root_of(record)?)?
            .keys()
            .cloned()
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };
    let proposal = record.body["proposal"].as_str().unwrap_or("1").to_owned();
    // The budget is a limit the person approves with the change they were shown: by default it covers exactly the files
    // in the delta the entry just printed, and one landing. `--budget` may widen or narrow it, and an unreadable budget
    // is refused rather than treated as unlimited.
    // 预算是**随他看到的那份改动一起**被批准的限额：默认恰好覆盖入口刚印出的那些文件与一次落地；`--budget` 可以放宽或
    // 收紧，而读不懂的预算会被拒绝，不会被当作无限。
    // The boundary the person approves is recorded here, by the same entry, not by the candidate:
    // `--protect <path>#<entity>` names content the requirement protects, and `--allow-removed-test
    // <path>::<name>` names a test the person decided may disappear. `land` reads this record and
    // never the candidate's own list, so a candidate cannot loosen its own criteria (2026-10-19
    // review, blocker on R4/EX11).
    // 人批准的那条边界由同一个入口记录，而不是由候选记录：`--protect <path>#<entity>` 点名需求保护的
    // 内容，`--allow-removed-test <path>::<name>` 点名一个人决定可以消失的测试。`land` 读这份记录、
    // 从不读候选自己的清单，因此候选不能放宽自己的判据（2026-10-19 复核关于 R4/EX11 的阻断项）。
    let protect: Vec<String> = protect.iter().map(|entry| (*entry).to_owned()).collect();
    let allow_removed: Vec<String> = allow_removed
        .iter()
        .map(|entry| (*entry).to_owned())
        .collect();
    crate::mcp::controller::note_boundary(root, id, &protect, &allow_removed)?;
    let boundary_line = format!(
        "boundary: protect [{}]; allow_removed_tests [{}] (recorded here, not by the candidate)\n",
        if protect.is_empty() {
            "none".to_owned()
        } else {
            protect.join(", ")
        },
        if allow_removed.is_empty() {
            "none".to_owned()
        } else {
            allow_removed.join(", ")
        },
    );
    let budget = match requested_budget {
        Some(text) => Budget::parse_limits(text)?,
        None => Budget {
            files: Some(changed.len() as u64),
            executions: Some(1),
        },
    };
    let grant = issue(root, id, &record.body, &scope, &proposal, budget)?;
    Ok(format!(
        "landing authorisation for candidate {id}\nproject {}\nscope {}\nchanged {}\nverification {}\nproposal {proposal}\n{}\n{}\nthis grants the landing only: `land` still refuses if the base moved, if the copy changed after its verification, or if a review failed at these bytes\n{}
recorded by: the host's confirmation entry (this binary cannot verify who ran it; no tool in this bridge can create one)\n",
        root.display(),
        scope.join(", "),
        if changed.is_empty() {
            "nothing — the copy is byte-identical to the base".to_owned()
        } else {
            changed.join(", ")
        },
        match record.body["verification"]["verdict"].as_str() {
            Some(verdict) => verdict.to_owned(),
            None => "not verified yet".to_owned(),
        },
        controller::line(&grant),
        boundary_line,
        match note {
            Some(note) => format!("note {note}\n"),
            None => String::new(),
        },
    ))
}

/// Issue the grant, through the controller's store. The only call site in this bridge.
/// 签发授权，走控制器的存储。本桥唯一的调用点。
fn issue(
    root: &Path,
    id: &str,
    record: &Value,
    scope: &[String],
    proposal: &str,
    budget: Budget,
) -> Result<Grant, String> {
    let project = super::registry::namespace(root).unwrap_or_else(|_| "unknown".to_owned());
    // Every dimension the kernel binds, in one place. `budget` and `policy` are **compared** now: a grant issued under
    // another policy version, or one that would be used beyond its limit, is refused by name. `approver` and `channel`
    // remain provenance and the status surface says so.
    // 内核绑定的每一个维度，集中在一处。`budget` 与 `policy` 现在**被比较**：按另一版政策签发的授权、或将被用到超出限额
    // 的那一份，都会被点名拒绝；`approver` 与 `channel` 仍是来源信息，状态面会说出来。
    let fields = vec![
        ("budget".to_owned(), budget.text()),
        (
            "candidate".to_owned(),
            record["id"].as_str().unwrap_or(id).to_owned(),
        ),
        ("operation".to_owned(), "land".to_owned()),
        ("policy".to_owned(), super::candidate::POLICY.to_owned()),
        ("project".to_owned(), project),
        ("proposal".to_owned(), proposal.to_owned()),
        (
            "revision".to_owned(),
            record["grant_revision"].as_str().unwrap_or("0").to_owned(),
        ),
        ("scope".to_owned(), scope.join(",")),
        (
            "task".to_owned(),
            record["task"].as_str().unwrap_or("candidate").to_owned(),
        ),
    ];
    controller::issue(
        root,
        fields,
        "the host's confirmation entry",
        "host-confirmation",
    )
}

/// The limit a widening's grant carries: exactly the files the increment will let land, and one landing.
/// 扩大授权携带的限额：恰好是这次增量会放行落地的那些文件，以及一次落地。
fn landing_budget(record: &Record) -> Budget {
    Budget {
        files: super::candidate::changed_files(record),
        executions: Some(1),
    }
}
