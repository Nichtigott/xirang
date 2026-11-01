//! The `explain --overlay` projection shape.
//! `explain --overlay` 投影形状。
//!
//! Split decision: the projection walks every face at once and pairs it with the
//! build's scope and declared cuts, which is a different traversal from the
//! per-node report in `super::report`. It reuses that page's cut rendering and
//! `super::json`'s serializer so the two shapes still speak one vocabulary.
//! 拆分决定：投影一次遍历每个面并把它与构建的作用域、已声明切口配对，这与
//! `super::report` 的逐节点报告是两种不同的遍历。它复用那页的切口渲染与
//! `super::json` 的序列化器，两种形状因此仍说同一套词汇。

use std::io::Write;
use std::path::Path;

use crate::build_method::{FaceView, OverlaySlot, declared_grafts, read_build_scope};
use serde_json::{Value, json};

use super::super::build_out_dir;
use super::json::{render_json, write_error};
use super::report::{cut_endpoint, cut_form};

/// The note every overlay projection carries, success or failure.
/// 每一次覆盖投影（无论成功或失败）都携带的说明。
/// The text belongs to `crate::build_method::OVERLAY_NOTE`, because the MCP
/// bridge renders the same projection; a local copy would let the CLI's JSON
/// document and the bridge's answer describe one projection two ways.
/// 该文本属于 `crate::build_method::OVERLAY_NOTE`，因为 MCP 桥渲染的是同一份投影；本地副本会
/// 让 CLI 的 JSON 文档与桥的回答对同一份投影给出两种说法。
pub(super) const OVERLAY_NOTE: &str = crate::build_method::OVERLAY_NOTE;

/// Render the static overlay projection: which of the build's slots a declared
/// cut replaces, and which faces the scope prunes.
/// 渲染静态覆盖投影：构建的哪些槽位被已声明切口替换，以及作用域剪掉了哪些面。
///
/// Why this is a projection and not `Registry::dump`: an overlay needs two live
/// registries, and the base/external trees are built inside the host crate by
/// its generated `registrations()` and `external_object!` declarations. A CLI
/// process has linked none of them, and an arbitrary in-repo host does not
/// export `base_registry()`/`external_registry()` (the example does, but that is
/// the example's own API). Fabricating a `Registry` from parsed source would
/// have to guess preset/parts/params/contract fields the macro owns, and a dump
/// of a guessed tree is worse than no dump. This command therefore reports the
/// build's own scope and graft declarations, which is what the overlay is
/// computed from; the host-side dump is `Registry::dump_effective`, which calls
/// `overlay_static` and then `dump` on the real pair.
/// 为什么这是投影而不是 `Registry::dump`：覆盖需要两棵活的注册树，而基树/外部树是在
/// 宿主 crate 内由其生成的 `registrations()` 与 `external_object!` 声明构建的。CLI
/// 进程一棵都没有链接，而仓库内任意宿主也不会导出
/// `base_registry()`/`external_registry()`（示例导出了，但那是示例自己的 API）。从
/// 解析出的源码伪造一棵 `Registry`，就必须猜宏拥有的 preset/parts/params/contract
/// 字段，而"猜出来的树"的 dump 比没有 dump 更糟。因此本命令报告构建自己的作用域与
/// graft 声明——覆盖正是由它们算出的；宿主侧的 dump 是 `Registry::dump_effective`，
/// 它对真实两棵树调用 `overlay_static` 再 `dump`。
pub(super) fn overlay_report(
    manifest: &Path,
    faces: &[FaceView],
    json_output: bool,
    out: &mut dyn Write,
) -> Result<(), String> {
    let out_dir = build_out_dir(manifest);
    // Same rule as the per-node report: output that no longer describes these
    // sources is treated as absent, so the projection says "unknown" instead of
    // drawing a tree from a previous build.
    // 与逐节点报告同一条规则：不再描述这批源码的产物按缺失处理，因此投影说的是"未知"，而不是
    // 用上一次构建画出一棵树。
    let scope = crate::build_method::build_output_is_current(manifest, &out_dir)
        .then(|| read_build_scope(&out_dir).ok())
        .flatten();
    let declared = declared_grafts(manifest);
    // The traversal itself belongs to `crate::build_method::overlay_projection`,
    // because the MCP bridge's `xirang.explain {"overlay": true}` asks the same
    // question; this page only renders the rows it returns.
    // 遍历本身属于 `crate::build_method::overlay_projection`，因为 MCP 桥的
    // `xirang.explain {"overlay": true}` 问的是同一个问题；本页只渲染它返回的行。
    let projection = match crate::build_method::overlay_projection(
        manifest,
        faces,
        scope.as_ref(),
        declared.as_ref().ok(),
    ) {
        Ok(projection) => projection,
        Err(error) => {
            // Same contract as the per-node report: `--json` means exactly one document on stdout,
            // and this path used to return before writing one — so a plans directory that is a plain
            // file (or any other refusal) reached a machine reader as a parse error rather than as an
            // answer (audit `S10`). `grafts --json` already collects the same failures this way.
            // 与逐节点报告同一条契约：`--json` 意味着 stdout 恰好一份文档，而这条路径过去在写出它之前
            // 就返回——于是"计划目录其实是个普通文件"（或任何别的拒绝）到达机器读者时是解析错误而不是
            // 答案（审计 `S10`）。`grafts --json` 早已用这种方式收下同样的失败。
            if json_output {
                let target: Option<String> = None;
                super::write_unresolved(&target, true, &error, out)?;
            }
            return Err(error);
        }
    };
    let slot_json = |slot: &OverlaySlot| {
        json!({
            "path": slot.path,
            "node": slot.id.to_string(),
            "kind": slot.kind,
            "source": slot.source,
            "selected": slot.selected,
            "kept": slot.kept,
            "replacement": slot.replacement.as_ref().map(|cut| {
                json!({
                    "cut": cut_endpoint(cut),
                    "graft": cut.graft,
                    "full": cut.full,
                    "line": cut.line,
                    "form": cut_form(cut),
                })
            }),
        })
    };
    let slots: Vec<Value> = projection.slots.iter().map(slot_json).collect();
    let pruned: Vec<Value> = projection.pruned.iter().map(slot_json).collect();
    let plans: Vec<Value> = projection
        .plans
        .iter()
        .map(super::super::grafts::row_json)
        .collect();

    let entry = declared
        .as_ref()
        .ok()
        .map(|declared| declared.entry.display().to_string());
    let entry_error = declared.as_ref().err().cloned();
    let scope_json = match &scope {
        Some(scope) => json!({
            "known": true,
            "mode": scope.mode,
            "all": scope.all,
            "reason": scope.reason,
        }),
        None => json!({"known": false}),
    };

    if json_output {
        let report = json!({
            "schema": "xirang.explain-overlay/1",
            "kind": "static-projection",
            "entry": entry,
            "entry_error": entry_error,
            "scope": scope_json,
            "slots": slots,
            "pruned": pruned,
            "plans": plans,
            "note": OVERLAY_NOTE,
        });
        writeln!(out, "{}", render_json(&report)).map_err(write_error)?;
        return Ok(());
    }

    writeln!(
        out,
        "overlay preview (static projection of the build's scope and declared cuts)"
    )
    .map_err(write_error)?;
    match (&entry, &entry_error) {
        (Some(entry), _) => writeln!(out, "  entry: {entry}").map_err(write_error)?,
        (None, Some(error)) => {
            writeln!(out, "  entry: unreadable ({error})").map_err(write_error)?
        }
        (None, None) => {}
    }
    if let Some(scope) = &scope {
        writeln!(
            out,
            "  scope: mode={} all={} reason={}",
            scope.mode,
            scope.all,
            scope.reason.as_deref().unwrap_or("<none>")
        )
        .map_err(write_error)?;
    } else {
        writeln!(
            out,
            "  scope: unknown; run `xirang check` to publish source_scope.tsv"
        )
        .map_err(write_error)?;
    }
    writeln!(out, "  slots ({}):", slots.len()).map_err(write_error)?;
    for slot in &slots {
        let replacement = slot["replacement"]
            .as_object()
            .map(|_| {
                format!(
                    " <- graft={} full={} (line {})",
                    slot["replacement"]["graft"].as_str().unwrap_or("<unknown>"),
                    slot["replacement"]["full"],
                    slot["replacement"]["line"]
                )
            })
            .unwrap_or_default();
        writeln!(
            out,
            "    {} kind={}{replacement}",
            slot["path"].as_str().unwrap_or("<unknown>"),
            slot["kind"].as_str().unwrap_or("<unknown>")
        )
        .map_err(write_error)?;
    }
    writeln!(out, "  pruned ({}):", pruned.len()).map_err(write_error)?;
    for slot in &pruned {
        writeln!(
            out,
            "    {} kind={}",
            slot["path"].as_str().unwrap_or("<unknown>"),
            slot["kind"].as_str().unwrap_or("<unknown>")
        )
        .map_err(write_error)?;
    }
    writeln!(out, "  plan records ({}):", plans.len()).map_err(write_error)?;
    for plan in &plans {
        match plan.get("error").and_then(Value::as_str) {
            Some(error) => writeln!(
                out,
                "    {}: unreadable ({error})",
                plan["selector"].as_str().unwrap_or("<unknown>")
            )
            .map_err(write_error)?,
            None => writeln!(
                out,
                "    selector={} target={} graft={} full={} declared={}",
                plan["selector"].as_str().unwrap_or("<unknown>"),
                plan["target_path"].as_str().unwrap_or("<unknown>"),
                plan["graft"].as_str().unwrap_or("<unknown>"),
                plan["full"],
                plan["declared"]
            )
            .map_err(write_error)?,
        }
    }
    Ok(())
}
