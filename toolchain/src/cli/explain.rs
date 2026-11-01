//! `xirang explain`: why a face is present, absent, or replaced.
//! `xirang explain`：一个注册面为何存在、缺席或被替换。
//!
//! This command answers operator questions about a host the CLI cannot link:
//! what a node is, why the build (not) ships it, and which declared graft cuts
//! name it. Every rule it applies is read from the build's own surface —
//! `face_views` for the tree and identities, `declared_grafts` for the cuts,
//! `read_build_scope`/`read_pruning_manifest` for the published build output —
//! so the command cannot disagree with the build about what it is describing.
//! 本命令回答 CLI 无法链接的宿主的运维问题：节点是什么、构建为何（不）发布它、
//! 哪些已声明的 graft 切口命名了它。它应用的每条规则都读自构建自身的表面——
//! 树与身份取 `face_views`，切口取 `declared_grafts`，已发布的构建产物取
//! `read_build_scope`/`read_pruning_manifest`——因此命令不可能与构建对它描述的对象
//! 产生分歧。

use std::io::Write;

use crate::build_method::{FaceView, face_views};
use serde_json::{Value, json};
use xirang_kernel::identity::NodeId;

use super::{build_out_dir, resolve_package};

// Split decision: `explain` has three output shapes that change independently —
// the per-node report (`report`), the whole-tree `--overlay` projection
// (`overlay`), and the JSON serializer both share (`json`). Each is mounted here
// with `#[path]` per the workspace rule, and this page keeps only argv parsing,
// the routing decision, and the query resolver they all depend on.
// 拆分决定：`explain` 有三种独立变化输出形状——逐节点报告（`report`）、整树
// `--overlay` 投影（`overlay`），以及两者共用的 JSON 序列化器（`json`）。各自按
// 工作区规则用 `#[path]` 在此挂载；本页只保留 argv 解析、路由决定，以及三者都依赖的
// 查询解析器。
#[path = "explain_json.rs"]
pub(crate) mod json;
#[path = "explain_overlay.rs"]
mod overlay;
#[path = "explain_report.rs"]
mod report;

/// One operator question: resolve a node, then report everything the build
/// knows about it.
/// 一次运维提问：解析一个节点，然后报告构建关于它知道的一切。
pub(crate) fn explain(
    args: &mut impl Iterator<Item = String>,
    out: &mut dyn Write,
) -> Result<(), String> {
    let mut json_output = false;
    let mut overlay = false;
    let mut directory: Option<String> = None;
    let mut target: Option<String> = None;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            // Help is not an error, and every subcommand answers it the same way:
            // the usage banner and success. Four of them used to refuse the flag with
            // "unexpected argument" (audit `LGC-LG-44`).
            // 帮助不是错误，而且每个子命令都以同一种方式回答：用法横幅 + 成功。其中四个过去
            // 用 "unexpected argument" 拒绝这个旗标（审计 `LGC-LG-44`）。
            "-h" | "--help" => return crate::cli::usage(out),
            "--json" => json_output = true,
            "--overlay" => overlay = true,
            "--path" => {
                directory = Some(args.next().ok_or("--path requires a directory")?);
            }
            _ if arg.starts_with('-') => return Err(format!("unexpected argument '{arg}'")),
            _ if target.is_none() => target = Some(arg),
            _ => return Err("explain accepts at most one node id or logical path".to_owned()),
        }
    }
    let directory = directory.unwrap_or_else(|| ".".to_owned());
    let (manifest, package) = match resolve_package(&directory) {
        Ok(resolved) => resolved,
        Err(error) => {
            if json_output {
                write_unresolved(&target, overlay, &error, out)?;
            }
            return Err(error);
        }
    };
    let mut faces = match face_views(&manifest, &package) {
        Ok(faces) => faces,
        Err(error) => {
            // The `--json` contract this command shares with `grafts` is "exactly one document on
            // stdout, with the failure inside it". This path returned before writing anything, so a
            // machine reader got a bare parse error and could not tell "no document" from "the tool
            // crashed" (audit `S10`). The package resolved and the source tree did not: that is an
            // answer, and it goes into the document.
            // 本命令与 `grafts` 共享的 `--json` 契约是"stdout 恰好一份文档、失败写在里面"。这条路径
            // 过去在写出任何东西之前就返回，于是机器读者只拿到一个裸解析错误，分不清"没有文档"与
            // "工具崩了"（审计 `S10`）。包解析成功而源码树没有：那也是一个答案，写进文档里。
            if json_output {
                write_unresolved(&target, overlay, &error, out)?;
            }
            return Err(error);
        }
    };
    // A record-file carrier (Java/Go/C#) has no source file for the walk to read: its faces live in
    // the file its build writes. Listing them here is what lets `explain <logical path>` answer for
    // that carrier too. The read is read-only (the producer does not run); a missing file is simply
    // no extra rows.
    // 记录文件载体（Java/Go/C#）没有可遍历的源码文件：面住在它的构建写出的文件里。在这里列出它们，正是
    // 让 `explain <逻辑路径>` 对这种载体也能作答。这次读取是只读的（不运行生产者）；缺文件就是没有额外行。
    if let Ok(Some((record_faces, _))) =
        crate::build_method::face_view::record_face_views(&manifest, &package, &faces)
    {
        faces.extend(record_faces);
    }
    if overlay {
        if target.is_some() {
            return Err(
                "--overlay renders the whole effective tree; drop the node id or path".to_owned(),
            );
        }
        return overlay::overlay_report(&manifest, &faces, json_output, out);
    }
    let query = target.ok_or("explain requires a node id or a logical path")?;
    let face = match resolve_face(&faces, &query) {
        Ok(face) => face,
        Err(reason) => {
            if json_output {
                let report = json!({
                    "schema": "xirang.explain/1",
                    "query": query,
                    "resolved": false,
                    "reason": reason,
                    "candidate_count": faces.len(),
                });
                writeln!(out, "{}", json::render_json(&report)).map_err(json::write_error)?;
            } else {
                writeln!(out, "unresolved: {reason}").map_err(json::write_error)?;
            }
            return Err(reason);
        }
    };
    let out_dir = build_out_dir(&manifest);
    // Published output is trusted only while it still describes these sources; see
    // `build_output_is_current` for why the fingerprint is the token.
    // 已发布的产物只在仍然描述这批源码时才被信任；为什么指纹是那枚凭据见
    // `build_output_is_current`。
    let current = crate::build_method::build_output_is_current(&manifest, &out_dir);
    let (scope_json, scope_note) = report::scope_report(&out_dir, face, current);
    let (pruning_json, pruning_note) = report::pruning_report(&out_dir, face, current);
    let (grafts_json, graft_note) = report::declared_report(&manifest, face);
    let kept = scope_json["kept"].as_bool();

    if json_output {
        let report = json!({
            "schema": "xirang.explain/1",
            "query": query,
            "resolved": true,
            "kept": kept,
            "node": {
                "id": face.id.to_string(),
                "path": face.path,
                "kind": face.kind,
                "registry_name": face.registry_name,
                "module": face.module,
                "source": face.source,
                "owns_registry": face.owns_registry,
                "parent": {
                    "id": face.parent.to_string(),
                    "path": report::parent_path(&faces, face.parent),
                    "resolved": face.parent_resolved,
                },
                "identity_inputs": {
                    "namespace": package,
                    "relative_source": face.source,
                    "kind": face.kind,
                },
            },
            "scope": scope_json,
            "pruning": pruning_json,
            "declared_grafts": grafts_json,
        });
        writeln!(out, "{}", json::render_json(&report)).map_err(json::write_error)?;
    } else {
        writeln!(out, "node {}", face.id).map_err(json::write_error)?;
        writeln!(out, "  path: {}", face.path).map_err(json::write_error)?;
        writeln!(out, "  kind: {}", face.kind).map_err(json::write_error)?;
        writeln!(out, "  source: {}", face.source).map_err(json::write_error)?;
        writeln!(out, "  module: {}", face.module).map_err(json::write_error)?;
        writeln!(
            out,
            "  identity: namespace={package} relative-source={} kind={}",
            face.source, face.kind
        )
        .map_err(json::write_error)?;
        writeln!(
            out,
            "  parent: {} ({}){}",
            report::parent_path(&faces, face.parent),
            face.parent,
            if face.parent_resolved {
                ""
            } else {
                "  [unresolved: the declaration names a parent this tree does not have]"
            }
        )
        .map_err(json::write_error)?;
        writeln!(
            out,
            "  registry: owns_registry={} slot={}",
            face.owns_registry, face.registry_name
        )
        .map_err(json::write_error)?;
        for line in scope_note
            .iter()
            .chain(pruning_note.iter())
            .chain(graft_note.iter())
        {
            writeln!(out, "{line}").map_err(json::write_error)?;
        }
    }
    Ok(())
}

/// Write the one JSON document a machine reader gets when the package cannot be
/// resolved, in the shape the requested output mode publishes.
/// 当包解析不出来时，为机器读者写出一个 JSON 文档，形状与所请求的输出模式一致。
///
/// `check` and `grafts` already emit a document on this path; `explain` used to
/// return before writing anything, so a consumer that asked for JSON got an
/// empty stdout. The per-node document is the same `resolved: false` shape the
/// command already uses for an unresolvable query; the overlay document keeps
/// the projection's schema and carries the failure in `error`.
/// `check` 与 `grafts` 在这条路径上已经会输出文档；`explain` 此前在写出任何东西之前就
/// 返回，因此要 JSON 的消费者拿到空 stdout。逐节点文档沿用本命令对无法解析查询所用的
/// `resolved: false` 形状；覆盖文档保持投影的 schema，并把失败放在 `error` 里。
pub(super) fn write_unresolved(
    target: &Option<String>,
    overlay: bool,
    error: &str,
    out: &mut dyn Write,
) -> Result<(), String> {
    let report = if overlay {
        json!({
            "schema": "xirang.explain-overlay/1",
            "kind": "static-projection",
            "entry": Value::Null,
            "entry_error": Value::Null,
            "scope": {"known": false},
            "slots": [],
            "pruned": [],
            "plans": [],
            "note": overlay::OVERLAY_NOTE,
            "error": error,
        })
    } else {
        json!({
            "schema": "xirang.explain/1",
            "query": target,
            "resolved": false,
            "reason": error,
            "candidate_count": 0,
        })
    };
    writeln!(out, "{}", json::render_json(&report)).map_err(json::write_error)
}

/// Resolve the argument the operator wrote: a 32-hex node identity, or the
/// logical path a runtime tree would print.
/// 解析操作者写下的参数：32 位十六进制节点身份，或运行期树会打印的逻辑路径。
///
/// The failure message lists the paths the tree does have, because "not found"
/// without the alternatives makes an operator guess at spelling.
/// 失败消息列出树确实拥有的路径，因为只报"找不到"会让操作者去猜拼写。
fn resolve_face<'a>(faces: &'a [FaceView], query: &str) -> Result<&'a FaceView, String> {
    if let Ok(id) = query.parse::<NodeId>() {
        return faces
            .iter()
            .find(|face| face.id == id)
            .ok_or_else(|| format!("no registration face has node id `{query}`"));
    }
    if let Some(face) = faces.iter().find(|face| face.path == query) {
        return Ok(face);
    }
    let mut paths = faces
        .iter()
        .map(|face| face.path.as_str())
        .collect::<Vec<_>>();
    paths.sort_unstable();
    let shown = paths.iter().take(8).copied().collect::<Vec<_>>().join(", ");
    Err(format!(
        "no registration face has logical path `{query}`; the tree has {} face(s): {shown}",
        paths.len()
    ))
}
