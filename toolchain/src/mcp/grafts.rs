//! The external graft plans on disk, and whether the host entry keeps them.
//! 磁盘上的外部 graft 计划，以及宿主入口是否保住它们。
//!
//! A plan under `.xirang/external-grafts/<selector>/graft.plan` is an authoring record
//! the build never opens. When the host entry's `static_graft_plan!` does not name the
//! slot a plan targets, the release prunes that slot and the record can never take
//! effect — and the build **fails** rather than warning, naming the plan file and the exact
//! `cut … graft …` line to declare (measured 2026-10-06: `cargo build` exits 101). A warning a
//! `cargo` log swallows would be the wrong shape for a record that can never take effect. The CLI
//! has answered this since `xirang grafts`; this is the same answer for an agent, and
//! the rule behind both is `crate::build_method::graft_plan_rows`, so they cannot
//! disagree. A declaration is about the **slot** a plan targets, not the implementation
//! the plan selects — that is the build's own question — so each row shows the plan's
//! target and graft *and* the declaration's own cut and graft, and a difference between
//! them is visible rather than silently accepted or silently rejected. Read-only: it opens
//! files and writes nothing.
//! `.xirang/external-grafts/<selector>/graft.plan` 下的计划是构建从不打开的创作记录。宿主入口的
//! `static_graft_plan!` 没有点名计划所针对的槽位时，发布态会剪掉那个槽位，这条记录便永远无法生效
//! ——而构建是**失败**而不是警告，并点名是哪个计划文件、以及该补上的那行 `cut … graft …`（2026-10-06
//! 实测：`cargo build` 退出码 101）。对一条永远无法生效的记录来说，一条会被 `cargo` 日志吞掉的警告是
//! 错的形状。CLI 从 `xirang grafts` 起就在回答这个问题；
//! 这里是给代理的同一个答案，而两者背后的规则是 `crate::build_method::graft_plan_rows`，因此它们
//! 不可能给出不同答案。声明针对的是计划所瞄准的**槽位**，而不是计划选择的那个实现——那正是构建自己的
//! 问题——因此每行既给出计划的目标与 graft，也给出声明自己的切口与 graft，两者不同时是**看得见**的，
//! 而不是被默默接受或默默拒绝。只读：只打开文件，不写任何东西。
//!
//! online: graft plans live on disk rather than in the pruning record, and a plan may have been written since the build.

use std::path::Path;

use crate::build_method::{declared_grafts, graft_plan_rows};
use serde_json::Value;

use crate::mcp::protocol::DEFAULT_LIMIT;
use crate::mcp::workspace::{self, Member, Scope};

/// The most plan rows one reply carries before it says it truncated.
/// 一条回复在声明被截断之前最多携带的计划条目数。
const MAX_ROWS: usize = 400;

/// The most of the entry's **own** declarations one reply lists before it says it truncated.
/// 一条回复在声明被截断之前最多列出多少条入口**自己**的声明。
const MAX_ENTRY_ROWS: usize = 5;

/// Report every external graft plan and whether the host entry declares its slot.
/// 报告每条外部 graft 计划，以及宿主入口是否声明了它的槽位。
///
/// A **virtual manifest** is answered as the workspace it is: the plans are records
/// under a package's own `.xirang/`, so they are read per member and grouped, with
/// every member's status in the census above them.
/// **虚拟清单**按它实际的样子——工作区——作答：计划是各包自己 `.xirang/` 之下的记录，因此逐成员
/// 读取并分组，而它们上方是每个成员的状态普查。
pub(crate) fn grafts(root: &Path, arguments: &Value) -> Result<String, String> {
    match workspace::scope(root)? {
        Scope::Package(namespace) => {
            let member = Member::package(root, namespace);
            grafts_body(&member, arguments)
        }
        Scope::Workspace(members) => workspace::merge(root, &members, arguments, grafts_body),
        Scope::Unresolvable(reason) => Ok(workspace::unresolvable(root, &reason)),
    }
}

/// Why this answer derives instead of reading a member's published records.
/// 这份答案为什么推导，而不是读成员的已发布记录。
///
/// A declaration is judged against the face a plan's target names, and a target is
/// a *logical path* — the one fact the published record does not carry, because a
/// path is assembled from the parent chain and the declared registry name rather
/// than written down. The records are still read first; this is the reason they
/// could not answer, and every reply says it took this path.
/// 声明的判定依据是计划目标点名的那个面，而目标是**逻辑路径**——正是已发布记录不携带的那一个事实，
/// 因为路径是由父链与声明的注册面名拼出来的，而不是写下来的。记录仍然先被读取；这是它们答不了的
/// 原因，而每份回复都会说自己走了这条路。
const DERIVES_BECAUSE: &str =
    "graft targets are logical paths, which the published record does not carry";

/// One package's graft report, as the merged view and the single-package view both call it.
/// 一个包的 graft 报告；合并视图与单包视图都调用它。
pub(crate) fn grafts_body(member: &Member, arguments: &Value) -> Result<String, String> {
    let root = member.dir.as_path();
    let namespace = member.name.as_str();
    let (faces, unparsable) = member.derived_tree()?;
    let faces = faces.as_slice();
    let unparsable = unparsable.as_str();
    let limit = arguments
        .get("limit")
        .and_then(Value::as_u64)
        .map_or(DEFAULT_LIMIT, |value| {
            value.clamp(1, MAX_ROWS as u64) as usize
        });
    // The entry and the plans are read independently: an unreadable entry leaves the
    // declaration state unknown rather than turning every plan into "not declared",
    // which is a different — and wrong — answer.
    // 入口与计划各自独立读取：入口读不了只会让声明状态未知，而不会把每条计划都变成"未被声明"——
    // 那是另一个（而且错误的）答案。
    let declared = declared_grafts(root);
    let entry_line = match &declared {
        Ok(declared) => format!("host entry {}\n", declared.entry.display()),
        Err(error) => format!("host entry unreadable ({error})\n"),
    };
    // The `plans` count is over **external records**; a tree whose whole plan lives in its own entry
    // — the typed `graft(…).with(…)` const array, or a `static_graft_plan!` — has a plan the build
    // applies and **zero** external records. The 2026-10-10 defect queue (item ②) measured exactly
    // that on the shipped example: this reply said `plans 0` while the entry declared two grafts, and
    // a reader asking "what does this tree graft" could take that zero for the answer. Saying how many
    // the entry declares is what keeps "no records" from reading as "no grafts" (report §7h).
    // `plans` 数的是**外部记录**；整份计划都写在自己入口里的树——类型化的 `graft(…).with(…)` 常量数组，
    // 或 `static_graft_plan!`——有一份构建会应用的计划、而外部记录是**零**。2026-10-10 缺陷队列第 ② 条
    // 在出厂示例上量到的正是这个：这条回复说 `plans 0`，而入口声明了两条 graft，问"这棵树 graft 了什么"
    // 的读者可能就把那个零当成了答案。把入口自己声明了几条说出来，才不会让"没有记录"被读成"没有 graft"
    // （报告 §7h）。
    let entry_grafts = match &declared {
        Ok(declared) => {
            let mut block = format!("entry grafts {}\n", declared.cuts.len());
            for cut in declared.cuts.iter().take(MAX_ENTRY_ROWS) {
                block.push_str(&format!(
                    "  line {} cut `{}` graft `{}` — the build applies this one itself\n",
                    cut.line,
                    cut.cut_label(),
                    cut.graft
                ));
            }
            if declared.cuts.len() > MAX_ENTRY_ROWS {
                block.push_str(&format!(
                    "  {}\n",
                    crate::mcp::truncation::withheld(
                        declared.cuts.len() - MAX_ENTRY_ROWS,
                        declared.cuts.len(),
                        MAX_ENTRY_ROWS,
                        "entry declarations",
                        "read the host entry named above"
                    )
                ));
            }
            block
        }
        Err(_) => "entry grafts unknown\n".to_owned(),
    };
    let rows = graft_plan_rows(root, faces, declared.as_ref().ok())?;
    // The count of unparsable registration files belongs to the tree this judgement was
    // made over: a face the derivation could not read is judged like an absent one by
    // `graft_plan_rows`, so the reader has to be told the tree was short.
    // 解析不了的注册面文件数属于做出这个判断所依据的那棵树：推导读不了的面在 `graft_plan_rows` 看来
    // 与不存在的面一样，因此必须告诉读者这棵树是短的。
    let mut output = format!(
        "namespace {namespace}\n{}{unparsable}{entry_line}",
        member.evidence_line(DERIVES_BECAUSE),
    );

    if let Err(error) = &declared {
        let _ = error;
        output.push_str(
            "declaration state unknown: no plan below can be told whether the release keeps it\n",
        );
    }
    output.push_str(&entry_grafts);
    output.push_str(&format!("plans {}\n", rows.len()));
    if rows.is_empty() {
        output.push_str(
            "no external graft plans under .xirang/external-grafts/; the entry's own declaration(s) \
             above are the plan the build applies\n",
        );
        return Ok(output);
    }
    // The headline count is over *every* row, not over the rows the bound happened to
    // print: this number is what a maintainer reads to see how many records are waiting for a
    // declaration that could name their slot — the build refuses such a plan by name rather than
    // pruning it at release time — and counting inside the truncating loop silently rewrote it to
    // whatever `limit` allowed.
    // 头条计数是**全部**行，而不是上限恰好打印出来的那些：这个数字是维护者用来查看"有多少条记录在等一条
    // 可能命名其槽位的声明"的——构建对这样的计划**按名拒绝**，而不是在发布期剪掉它——而在截断循环里计数会
    // 把它静默改写成 `limit` 允许的那部分。
    let unkept = rows
        .iter()
        .filter(|row| row.error.is_none() && row.declared == Some(false))
        .count();
    for (index, row) in rows.iter().enumerate() {
        if index >= limit {
            output.push_str(&format!(
                "  {}\n",
                crate::mcp::truncation::withheld(
                    rows.len() - limit,
                    rows.len(),
                    limit,
                    "plan rows",
                    "raise `limit`"
                )
            ));
            break;
        }
        match &row.error {
            Some(error) => {
                output.push_str(&format!("  {}: unreadable ({error})\n", row.selector));
            }
            None => {
                let verdict = match row.declared {
                    Some(true) => match &row.declared_by {
                        Some(cut) => format!(
                            "declared at entry line {} as cut `{}` graft `{}`{}",
                            cut.line,
                            cut.cut_label(),
                            cut.graft,
                            // A record on a slot the entry hands to *another* implementation will never be
                            // applied, and saying only "declared" made that look like a working plan
                            // (audit `M7`, §M7.63).
                            // 一份坐在"入口交给**别的**实现"的槽位上的记录永远不会被应用，而只说一句
                            // "declared" 会让它看起来是一份能工作的计划（审计 `M7`，§M7.63）。
                            if row.selector_matches == Some(false) {
                                format!(
                                    " — but this record selects `{}`, so no build applies it",
                                    row.selector
                                )
                            } else {
                                String::new()
                            }
                        ),
                        None => "declared".to_owned(),
                    },
                    Some(false) => "NOT declared by the host entry".to_owned(),
                    None => "declaration unknown".to_owned(),
                };
                output.push_str(&format!(
                    "  {}: target={} graft={} full={} [{verdict}]\n",
                    row.selector,
                    row.target_path.as_deref().unwrap_or("<unknown>"),
                    row.graft.as_deref().unwrap_or("<unknown>"),
                    row.full
                        .map(|full| full.to_string())
                        .unwrap_or_else(|| "<unknown>".to_owned()),
                ));
            }
        }
    }
    if unkept > 0 {
        output.push_str(&format!(
            "unkept plans {unkept}: the release prunes these slots, so the records can never take \
             effect. Add a `static_graft_plan!` declaration naming each one to the host entry.\n"
        ));
    }
    Ok(output)
}

#[cfg(test)]
#[path = "grafts_tests.rs"]
mod grafts_tests;
