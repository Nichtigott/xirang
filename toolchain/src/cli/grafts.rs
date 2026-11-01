//! `xirang grafts`: inspect the external graft plans on disk.
//! `xirang grafts`：检视磁盘上的外部 graft 计划。
//!
//! A plan under `.xirang/external-grafts/` is an authoring record the build
//! never opens; the build only warns when the host entry's `static_graft_plan!`
//! does not declare the slot (see `build_method/src/graft_plan_check.rs`). That
//! warning is easy to miss in a long `cargo` log, so this command makes the same
//! decision inspectable on demand, using the kernel's `GraftPlanDocument` parser
//! and the build's `declared_grafts`/`names_face` rather than a second copy of
//! either rule. Read-only: it opens files and writes nothing.
//! `.xirang/external-grafts/` 下的计划是构建从不打开的创作记录；只有当宿主入口的
//! `static_graft_plan!` 没有声明该槽位时构建才警告（见
//! `build_method/src/graft_plan_check.rs`）。在冗长的 `cargo` 日志里这条警告容易被
//! 漏掉，因此本命令用内核的 `GraftPlanDocument` 解析器和构建的
//! `declared_grafts`/`names_face` 让同一判断可按需检视，而不是各写第二份规则。
//! 只读：只打开文件，不写任何东西。

use std::io::Write;
use std::path::Path;

use crate::build_method::{DeclaredGrafts, FaceView, declared_grafts, face_views};
use serde_json::{Value, json};

// The JSON error document and the command error text are one rule each; the
// `explain` page owns both, so this command cannot drift from it.
// JSON 错误文档与命令错误文本各是一条规则；它们归 `explain` 页面所有，因此本命令不会
// 与它漂移。
use crate::cli::explain::json::{render_json, write_error};

use super::resolve_package;

/// List every plan under `.xirang/external-grafts/*/graft.plan`.
/// 列出 `.xirang/external-grafts/*/graft.plan` 下的每个计划。
pub(crate) fn grafts(
    args: &mut impl Iterator<Item = String>,
    out: &mut dyn Write,
) -> Result<(), String> {
    let mut json_output = false;
    let mut directory: Option<String> = None;
    for arg in args.by_ref() {
        match arg.as_str() {
            // Help is not an error, and every subcommand answers it the same way:
            // the usage banner and success. Four of them used to refuse the flag with
            // "unexpected argument" (audit `LGC-LG-44`).
            // 帮助不是错误，而且每个子命令都以同一种方式回答：用法横幅 + 成功。其中四个过去
            // 用 "unexpected argument" 拒绝这个旗标（审计 `LGC-LG-44`）。
            "-h" | "--help" => return crate::cli::usage(out),
            "--json" => json_output = true,
            _ if arg.starts_with('-') => return Err(format!("unexpected argument '{arg}'")),
            _ if directory.is_none() => directory = Some(arg),
            _ => return Err("grafts accepts at most one path".to_owned()),
        }
    }
    let directory = directory.unwrap_or_else(|| ".".to_owned());
    let (manifest, package) = match resolve_package(&directory) {
        Ok(resolved) => resolved,
        Err(error) => {
            // The `--json` contract is "stdout is one JSON document"; a
            // resolution failure used to return before writing anything, so a
            // machine reader got an empty stream instead of a document that
            // names the failure. The success shape is kept, with the reason.
            // `--json` 契约是"stdout 是一个 JSON 文档"；解析失败此前在写出任何东西之前
            // 就返回，机器读者拿到的是空流，而不是点名失败的文档。这里保持成功时的形状，
            // 并带上原因。
            if json_output {
                let report = json!({
                    "schema": "xirang.grafts/1",
                    "entry": Value::Null,
                    "entry_error": Value::Null,
                    "plans": [],
                    "error": error.clone(),
                });
                writeln!(out, "{}", render_json(&report)).map_err(write_error)?;
            }
            return Err(error);
        }
    };
    // Every read failure here is collected and reported at the end. This command
    // answers one question — "does the host entry declare the slot each plan
    // addresses" — and a source tree, an entry or a plans directory it cannot
    // read means it cannot answer it. Exiting zero on a partial answer would be
    // worse than the failure: the declaration column would look computed when it
    // was guessed.
    // 这里的每次读取失败都会被收集并在最后报告。本命令回答一个问题——"宿主入口是否声明了
    // 每条计划所针对的槽位"——而读不了的源码树、入口或计划目录都意味着它答不出来。带着部分
    // 答案以 0 退出比失败更糟：声明那一列看上去像是算出来的，其实是猜的。
    let mut problems = Vec::new();
    let faces = match face_views(&manifest, &package) {
        Ok(faces) => faces,
        Err(error) => {
            problems.push(format!("host source tree: {error}"));
            Vec::new()
        }
    };
    let declared = declared_grafts(&manifest);
    let rows = match plan_rows(&manifest, &faces, declared.as_ref().ok()) {
        Ok(rows) => rows,
        Err(error) => {
            problems.push(error);
            Vec::new()
        }
    };
    let entry = declared
        .as_ref()
        .map(|declared| declared.entry.display().to_string())
        .ok();
    let entry_error = declared.as_ref().err().cloned();
    if let Some(error) = &entry_error {
        problems.push(format!("host entry: {error}"));
    }

    if json_output {
        let report = json!({
            "schema": "xirang.grafts/1",
            "entry": entry,
            "entry_error": entry_error,
            "plans": rows,
        });
        writeln!(out, "{}", render_json(&report)).map_err(write_error)?;
        return report_problems(problems);
    }

    match (&entry, &entry_error) {
        (Some(entry), _) => writeln!(out, "host entry: {entry}").map_err(write_error)?,
        (None, Some(error)) => {
            writeln!(out, "host entry: unreadable ({error})").map_err(write_error)?
        }
        (None, None) => {}
    }
    if rows.is_empty() {
        writeln!(
            out,
            "no external graft plans under .xirang/external-grafts/"
        )
        .map_err(write_error)?;
        return report_problems(problems);
    }
    for row in &rows {
        match row.get("error").and_then(Value::as_str) {
            Some(error) => writeln!(
                out,
                "{}: unreadable ({error})",
                row["selector"].as_str().unwrap_or("<unknown>")
            )
            .map_err(write_error)?,
            None => {
                // "the slot is declared" and "this record is what gets applied" are two answers, and only
                // the first one used to be printed (audit `M7`, §M7.63).
                // "槽位有声明"与"会被应用的是这条记录"是两个答案，而过去只印了第一个（审计 `M7`，§M7.63）。
                let declared = match (row["declared"].as_bool(), row["selector_matches"].as_bool())
                {
                    (Some(true), Some(false)) => {
                        "declared for this slot, but handed to another implementation"
                    }
                    (Some(true), _) => "declared",
                    (Some(false), _) => "NOT declared by the host entry",
                    (None, _) => "declaration unknown",
                };
                writeln!(
                    out,
                    "{}: target={} graft={} full={} [{declared}]",
                    row["selector"].as_str().unwrap_or("<unknown>"),
                    row["target_path"].as_str().unwrap_or("<unknown>"),
                    row["graft"].as_str().unwrap_or("<unknown>"),
                    row["full"]
                )
                .map_err(write_error)?;
                if let Some(cut) = row["declared_by"].as_object() {
                    // Render the string values, not the `serde_json::Value`s:
                    // `Value`'s `Display` is JSON, so a quoted `"button_fast"`
                    // would reach a terminal that is not reading JSON.
                    // 渲染字符串值而不是 `serde_json::Value`：`Value` 的 `Display`
                    // 是 JSON，会把带引号的 `"button_fast"` 送到并不读 JSON 的终端。
                    writeln!(
                        out,
                        "    declared at line {} as cut `{}` graft `{}`",
                        cut["line"],
                        cut["cut"].as_str().unwrap_or("<unknown>"),
                        cut["graft"].as_str().unwrap_or("<unknown>")
                    )
                    .map_err(write_error)?;
                }
            }
        }
    }
    report_problems(problems)
}

/// Succeed only when every part of the answer could be read.
/// 只有答案的每个部分都读到了才成功。
///
/// The report is written first, so a reader still gets the rows that were
/// readable; the error then makes the exit code say the answer is incomplete.
/// 报告先写出，因此读者仍拿到可读的那些条目；随后的错误让退出码说明这份答案不完整。
fn report_problems(problems: Vec<String>) -> Result<(), String> {
    if problems.is_empty() {
        return Ok(());
    }
    Err(format!(
        "incomplete: could not read {}",
        problems.join("; ")
    ))
}

/// Build one report row per plan directory, sorted by selector.
/// 为每个计划目录生成一行报告，按 selector 排序。
///
/// The rule — which plans exist, what each targets, and whether the host entry declares
/// that slot — belongs to `crate::build_method::graft_plan_rows`, because the MCP
/// bridge's `xirang.grafts` asks the same question and two copies of a rule like this
/// drift. This only renders the rows into the `xirang.grafts/1` JSON shape.
/// 规则——有哪些计划、每条针对什么、以及宿主入口是否声明了那个槽位——属于
/// `crate::build_method::graft_plan_rows`，因为 MCP 桥的 `xirang.grafts` 问的是同一个
/// 问题，而这类规则的两份副本会漂移。这里只把那些行渲染成 `xirang.grafts/1` 的 JSON 形状。
pub(crate) fn plan_rows(
    manifest: &Path,
    faces: &[FaceView],
    declared: Option<&DeclaredGrafts>,
) -> Result<Vec<Value>, String> {
    Ok(
        crate::build_method::graft_plan_rows(manifest, faces, declared)?
            .iter()
            .map(row_json)
            .collect(),
    )
}

/// Render one plan row as the `xirang.grafts/1` JSON shape.
/// 把一条计划行渲染成 `xirang.grafts/1` 的 JSON 形状。
///
/// Shared with the overlay projection, which carries the same rows: two renderers
/// for one row would let the `grafts` verb and `explain --overlay` describe the
/// same plan differently.
/// 与携带同一批行的覆盖投影共用：一条行有两个渲染器，会让 `grafts` 与 `explain --overlay`
/// 对同一条计划给出不同描述。
pub(crate) fn row_json(row: &crate::build_method::GraftPlanRow) -> Value {
    let declared_by = row.declared_by.as_ref().map(|cut| {
        json!({
            "cut": match &cut.cut_end {
                Some(end) => format!("{} to {end}", cut.cut),
                None => cut.cut.clone(),
            },
            "graft": cut.graft,
            "full": cut.full,
            "line": cut.line,
        })
    });
    json!({
        "selector": row.selector,
        "error": row.error,
        "target": row.target.map(|target| target.to_string()),
        "target_path": row.target_path,
        "graft": row.graft,
        "full": row.full,
        "declared": row.declared,
        "declared_by": declared_by,
        "selector_matches": row.selector_matches,
    })
}
