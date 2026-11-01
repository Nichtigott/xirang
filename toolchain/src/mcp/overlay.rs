//! The static overlay projection for an agent: what ships after the declared cuts
//! replace their slots.
//! 面向代理的静态覆盖投影：已声明切口替换各自槽位之后，什么会发布。
//!
//! `xirang.explain` answers one face at a time, or projects the tree the build
//! scoped; neither says which slot a graft hands over. The CLI has answered that
//! since `explain --overlay`, and the traversal behind it now lives in
//! `crate::build_method::overlay_projection` — this module renders that one
//! projection for an agent instead of re-walking the tree, which is the same
//! reason `xirang.grafts` shares `graft_plan_rows` with the CLI's `grafts`.
//! What it deliberately is **not**: the live effective tree. An overlay needs two
//! live registries, and only a host that links both of them can dump one; the
//! reply carries the same note the CLI's document does, so a reader cannot mistake
//! this projection for a `Registry::dump`.
//! `xirang.explain` 一次回答一个面，或投影构建划定作用域的那棵树；两者都不说哪个槽位被 graft
//! 交出。CLI 从 `explain --overlay` 起就在回答这个问题，而它背后的遍历现在住在
//! `crate::build_method::overlay_projection`——本模块为代理渲染那一份投影，而不是重新走一遍树，
//! 这与 `xirang.grafts` 与 CLI 的 `grafts` 共用 `graft_plan_rows` 是同一个理由。它有意**不是**：
//! 活的生效树。覆盖需要两棵活的注册树，只有同时链接两者的宿主才能 dump 出它；回复携带与 CLI 文档
//! 相同的说明，因此读者不可能把这份投影误当成一次 `Registry::dump`。

use std::path::Path;

use crate::build_method::{
    BuildScopeView, DeclaredGrafts, OVERLAY_NOTE, OverlayProjection, OverlaySlot, declared_grafts,
    face_views, overlay_projection,
};
use serde_json::Value;

use crate::mcp::build_evidence::build_evidence;
use crate::mcp::protocol::DEFAULT_LIMIT;
use crate::mcp::registry::namespace;

/// The most rows of each list one reply carries before it says it truncated.
/// 每条列表在声明被截断之前最多携带的行数。
const MAX_ROWS: usize = 200;

/// Report the static overlay projection for this package.
/// 报告本包的静态覆盖投影。
///
/// `node` is refused rather than ignored: the projection walks every slot at
/// once, so naming one face has no meaning here, and silently dropping the
/// argument would answer a question the caller did not ask.
/// `node` 会被拒绝而不是忽略：投影一次遍历每个槽位，因此点名一个面在这里没有意义，而默默丢掉
/// 该参数等于回答了调用方没问的问题。
pub(crate) fn overlay(root: &Path, arguments: &Value) -> Result<String, String> {
    if arguments.get("node").and_then(Value::as_str).is_some() {
        return Err(
            "overlay renders the whole effective tree; drop `node` (use it without `overlay` for \
             one face)"
                .to_owned(),
        );
    }
    let namespace = namespace(root)?;
    let faces = face_views(root, &namespace)?;
    let evidence = build_evidence(root);
    let declared = declared_grafts(root);
    let projection = overlay_projection(
        root,
        &faces,
        evidence.scope.as_ref(),
        declared.as_ref().ok(),
    )?;
    let limit = arguments
        .get("limit")
        .and_then(Value::as_u64)
        .map_or(DEFAULT_LIMIT, |value| {
            value.clamp(1, MAX_ROWS as u64) as usize
        });
    Ok(render_overlay(
        &namespace,
        evidence.freshness(),
        evidence.scope.as_ref(),
        &declared,
        &projection,
        limit,
    ))
}

/// Render one projection as the lines an agent reads.
/// 把一份投影渲染成代理阅读的行。
fn render_overlay(
    namespace: &str,
    freshness: &str,
    scope: Option<&BuildScopeView>,
    declared: &Result<DeclaredGrafts, String>,
    projection: &OverlayProjection,
    limit: usize,
) -> String {
    let mut output = format!("namespace {namespace}\n");
    output.push_str("overlay (static projection of the build's scope and declared cuts)\n");
    match declared {
        Ok(declared) => output.push_str(&format!("entry {}\n", declared.entry.display())),
        Err(error) => output.push_str(&format!("entry unreadable ({error})\n")),
    }
    // The state word comes from the one place that spells it, `BuildEvidence::freshness()`,
    // and is passed in: this report used to spell the freshness word itself, which left two
    // more copies to drift from the one in `build_evidence.rs` — nothing coupled them, as an
    // independent check found with mutation `E`.
    // 状态词来自唯一拼它的地方 `BuildEvidence::freshness()`，由调用方传进来：本报告过去自己拼这个
    // 新鲜度词，于是相对 `build_evidence.rs` 里的那一份又多了两处可能漂移的副本——没有任何钉子把三者耦合
    // 起来，这正是独立复核用变异 `E` 发现的。
    output.push_str(&format!("build {freshness}\n"));
    match scope {
        Some(scope) => output.push_str(&format!(
            "scope mode={} all={} reason={}\n",
            scope.mode,
            scope.all,
            scope.reason.as_deref().unwrap_or("-")
        )),
        None => output.push_str("scope unknown (no source_scope.tsv; run `xirang check`)\n"),
    }
    let replaced = projection
        .slots
        .iter()
        .filter(|slot| slot.replacement.is_some())
        .count();
    output.push_str(&format!(
        "slots {} (replaced {replaced}):\n",
        projection.slots.len()
    ));
    for slot in projection.slots.iter().take(limit) {
        output.push_str(&format!(
            "  {:<40} kind={}{}\n",
            slot.path,
            slot.kind,
            replacement(slot)
        ));
    }
    if projection.slots.len() > limit {
        output.push_str(&format!(
            "  {}\n",
            crate::mcp::truncation::withheld(
                projection.slots.len() - limit,
                projection.slots.len(),
                limit,
                "slots",
                "raise `limit`"
            )
        ));
    }
    output.push_str(&format!("pruned {}:\n", projection.pruned.len()));
    for slot in projection.pruned.iter().take(limit) {
        output.push_str(&format!("  {:<40} kind={}\n", slot.path, slot.kind));
    }
    if projection.pruned.len() > limit {
        output.push_str(&format!(
            "  {}\n",
            crate::mcp::truncation::withheld(
                projection.pruned.len() - limit,
                projection.pruned.len(),
                limit,
                "pruned slots",
                "raise `limit`"
            )
        ));
    }
    output.push_str(&format!("plan records {}:\n", projection.plans.len()));
    for row in projection.plans.iter().take(limit) {
        match &row.error {
            Some(error) => output.push_str(&format!("  {}: unreadable ({error})\n", row.selector)),
            None => {
                let kept = match (row.declared, &row.declared_by) {
                    (Some(true), Some(cut)) => format!(
                        "kept by cut `{}` graft `{}` at entry line {}",
                        cut.cut_label(),
                        cut.graft,
                        cut.line
                    ),
                    (Some(true), None) => "declared".to_owned(),
                    (Some(false), _) => "NOT declared by the host entry".to_owned(),
                    (None, _) => "declaration unknown".to_owned(),
                };
                output.push_str(&format!(
                    "  {} target={} graft={} full={} [{kept}]\n",
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
    if projection.plans.len() > limit {
        output.push_str(&format!(
            "  {}\n",
            crate::mcp::truncation::withheld(
                projection.plans.len() - limit,
                projection.plans.len(),
                limit,
                "plan records",
                "raise `limit`"
            )
        ));
    }
    output.push_str(&format!("note: {OVERLAY_NOTE}\n"));
    output
}

/// How one kept slot is annotated when a declared cut replaces it.
/// 一个被保住的槽位在被已声明切口替换时如何标注。
fn replacement(slot: &OverlaySlot) -> String {
    match &slot.replacement {
        Some(cut) => format!(
            "  <- graft={} full={} form={} (entry line {})",
            cut.graft,
            cut.full,
            cut.form(),
            cut.line
        ),
        None => String::new(),
    }
}

#[cfg(test)]
#[path = "overlay_tests.rs"]
mod overlay_tests;
