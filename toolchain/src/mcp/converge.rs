//! Everything an agent needs to start on one face, in one answer.
//! 代理着手处理一个面所需的全部，集中在一个答案里。
//!
//! The other tools each answer one question, which means an agent that wants to
//! touch a face has to ask several and join the answers itself. This is that join,
//! and it adds the one verdict no single tool can give: a requirement is written
//! `capability=>ProviderKind`, and whether an ancestor actually answers
//! it is a question about the whole tree. Everything here is composed from what the
//! other tools report — the build's scope and pruning, the tree's edges, the
//! declared fields — so a disagreement between this and them would be a bug in the
//! composition rather than a second derivation.
//! online: joins use current declarations and a later runtime artifact, neither guaranteed to match the last build.
//! 别的工具各回答一个问题，因此想动一个面的代理得问好几次、再自己把答案拼起来。这里就是那次拼接，
//! 并补上任何一个工具都给不出的那个判断：需求写成 `capability=>ProviderKind`，而"祖先链上有东西真的满足
//! 它吗"是关于整棵树的问题。这里的一切都从别的工具所报告的东西组合而来——构建的作用域与剪枝、树的边、
//! 声明的字段——因此它若与它们不一致，那是组合的缺陷，而不是第二份推导。

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use crate::build_method::face_views;
use serde_json::Value;

use crate::mcp::build_evidence::build_evidence;
use crate::mcp::protocol::DEFAULT_LIMIT;
use crate::mcp::registry::namespace;
use crate::mcp::resolve::resolve_in;

#[path = "converge_context.rs"]
mod context;

#[path = "converge_chain.rs"]
mod chain;

/// Join declarations, build selection, observations and verification in one request.
/// 在一次请求内拼接声明、构建选择、运行观察与验证义务。
pub(crate) fn converge(root: &Path, arguments: &Value) -> Result<String, String> {
    let namespace = namespace(root)?;
    let faces = crate::source_cache::phase("face_views", || face_views(root, &namespace))?;
    let known: BTreeMap<_, _> = faces.iter().map(|face| (face.id, face)).collect();
    let limit = arguments
        .get("limit")
        .and_then(Value::as_u64)
        .map_or(DEFAULT_LIMIT, |value| value.clamp(1, 200) as usize);
    let target = arguments
        .get("node")
        .and_then(Value::as_str)
        .map(|target| resolve_in(&namespace, target, &faces))
        .transpose()?;
    if target.is_none() && arguments.get("trace").and_then(Value::as_bool) != Some(true) {
        return Err(
            "xirang.converge requires node (one face) or trace: true (the recorded run)".to_owned(),
        );
    }
    if target.is_some_and(|id| !known.contains_key(&id)) {
        return Err("no face in the derived tree has that identity".to_owned());
    }
    let targets = target.map(|id| [id].into_iter().collect());
    let packet = packet(root, &namespace, &faces, targets.as_ref(), target, limit);
    let selected = &packet.selected;
    let mut output = packet.overview + &packet.responsibilities + &packet.runtime;
    output.push_str(&format!(
        "chain read plan ({} files)\n",
        selected
            .iter()
            .map(|id| &known[id].source)
            .collect::<BTreeSet<_>>()
            .len()
    ));
    for id in selected {
        output.push_str(&format!(
            "  explore {{query: \"src/{}\"}}\n",
            known[id].source
        ));
    }
    output.push_str("next verification: status {} lists test faces; check {face: <test face from status>}; a registry node path is not a test-face name\n");
    Ok(bounded(&output, 48_000))
}

/// Shared evidence sections let callers budget source and responsibility independently.
/// 共用证据段落使调用方能分别给源码与责任信息分配预算。
pub(super) struct Packet {
    /// Connected identities within the limit / 上限内相连的身份。
    pub selected: BTreeSet<xirang_kernel::NodeId>,
    /// Focus and explicit traversal bounds / 焦点与明确的遍历边界。
    pub overview: String,
    /// Declarations, build/graft facts and obligations / 声明、构建与嫁接事实、验证义务。
    pub responsibilities: String,
    /// Observations and their provenance, or absence/refusal / 观察及来源，或缺失与拒绝。
    pub runtime: String,
}

/// Compose one evidence packet for all targets using the caller's current tree.
/// 使用调用方的当前树，为全部目标组合一份证据包。
pub(super) fn packet(
    root: &Path,
    namespace: &str,
    faces: &[crate::build_method::FaceView],
    targets: Option<&BTreeSet<xirang_kernel::NodeId>>,
    target: Option<xirang_kernel::NodeId>,
    limit: usize,
) -> Packet {
    let known: BTreeMap<_, _> = faces.iter().map(|face| (face.id, face)).collect();
    let trace = crate::source_cache::phase("trace", || {
        crate::mcp::trace::read_verified_in(root, namespace, faces)
    });
    let runtime = match &trace {
        Ok(crate::mcp::trace::RecordedTrace::Verified { artifact, .. }) => {
            Some(crate::mcp::converge_trace::Runtime::new(artifact))
        }
        _ => None,
    };
    let seeds: BTreeSet<_> = match targets {
        Some(ids) => ids.clone(),
        None => runtime
            .as_ref()
            .map(|run| {
                run.observed
                    .iter()
                    .filter(|id| known.contains_key(id))
                    .copied()
                    .collect()
            })
            .unwrap_or_default(),
    };
    let declarations = crate::source_cache::phase("declarations", || {
        chain::Declarations::read_current(root, namespace, faces)
    });
    let evidence = crate::source_cache::phase("build_evidence", || build_evidence(root));
    let (selected, pending) = chain::neighbourhood(
        &seeds,
        &declarations,
        runtime
            .as_ref()
            .map_or(&[], |run| run.neighbours.as_slice()),
        &known,
        limit,
    );
    let mut output = format!("namespace {namespace}\n");
    if let Some(id) = target {
        let face = known[&id];
        output.push_str(&format!("focus {}\n", face.path));
        output.push_str(&node_report(face, faces, &declarations, limit));
    }
    output.push_str(&format!("responsibility neighbourhood: {} objects; {pending} queued objects withheld; deeper closure beyond the limit is not computed\n", selected.len()));
    let responsibilities = chain::report(
        root,
        faces,
        &selected,
        &declarations,
        &evidence,
        runtime.as_ref().map(|run| &run.observed),
        limit,
    );
    let mut runtime_output = String::new();
    match &trace {
        Ok(crate::mcp::trace::RecordedTrace::Verified { header, .. }) => {
            runtime_output.push_str(header);
            runtime_output.push_str(
                &runtime
                    .as_ref()
                    .expect("verified runtime")
                    .report(&known, &selected, limit),
            );
        }
        Ok(
            crate::mcp::trace::RecordedTrace::Absent(answer)
            | crate::mcp::trace::RecordedTrace::Refused(answer),
        ) => runtime_output.push_str(answer),
        Err(error) => runtime_output.push_str(&format!(
            "trace unreadable: {error}; runtime evidence unavailable\n"
        )),
    }
    Packet {
        selected,
        overview: output,
        responsibilities,
        runtime: runtime_output,
    }
}

fn node_report(
    face: &crate::build_method::FaceView,
    faces: &[crate::build_method::FaceView],
    declarations: &chain::Declarations,
    limit: usize,
) -> String {
    let id = face.id;
    let authored = declarations.fields.get(&id);
    let rejected = &declarations.rejected;
    let unreadable = declarations.unreadable;
    let change_context = context::change_context(faces, face, declarations, limit);
    let mut output = format!(
        "node {}\n  path {}\n  kind {}\n",
        face.id, face.path, face.kind
    );
    let children = faces
        .iter()
        .filter(|candidate| candidate.parent == id)
        .collect::<Vec<_>>();
    output.push_str(&format!("children {}\n", children.len()));
    match (rejected, authored) {
        (Some(_), _) => {
            output.push_str("requires unavailable: registry rejection is reported below\n")
        }
        (None, Some(authored)) => {
            output.push_str(&format!("requires {}\n", blank(&authored.requires)));
            for verdict in declarations.edges.iter().filter(|edge| edge.consumer == id) {
                match verdict
                    .provider
                    .and_then(|id| faces.iter().find(|face| face.id == id))
                    .map(|face| &face.path)
                {
                    Some(who) => output.push_str(&format!(
                        "  {} => {}  answered by {who}\n",
                        verdict.capability, verdict.kind
                    )),
                    None => output.push_str(&format!(
                        "  {} => {}  UNANSWERED (no ancestor of kind `{}` offers `{}`)\n",
                        verdict.capability, verdict.kind, verdict.kind, verdict.capability
                    )),
                }
            }
        }
        (None, None) => output.push_str("requires unreadable (inspect declaration)\n"),
    }
    output.push_str(&change_context);
    let mut plan: Vec<(String, &'static str)> = vec![(face.source.clone(), "this face")];
    if let Some(parent) = faces.iter().find(|candidate| candidate.id == face.parent) {
        plan.push((parent.source.clone(), "parent"));
    }
    for child in &children {
        plan.push((child.source.clone(), "child"));
    }
    plan.dedup_by(|left, right| left.0 == right.0);
    output.push_str(&format!("read plan ({} files)\n", plan.len()));
    for (source, why) in plan.iter().take(limit) {
        output.push_str(&format!("  {source:<44} ({why})\n"));
    }
    if plan.len() > limit {
        output.push_str(&format!(
            "  {}\n",
            crate::mcp::truncation::withheld(
                plan.len() - limit,
                plan.len(),
                limit,
                "files",
                "raise `limit`"
            )
        ));
    }
    if unreadable > 0 {
        output.push_str(&format!(
            "unreadable faces {unreadable} (hand-written modules declare no readable fields)\n"
        ));
    }
    output.push_str(
        "detail: xirang.explain (build evidence) · xirang.usages (fields and capability refs) · \
         xirang.trace (what ran) · xirang.diff (what changed since the build)\n",
    );
    // The layered answer ends by naming the next call, because that is the one place guidance was
    // measured to work (the `literal` pointer in `search`): prose at `initialize` was ignored 4/4.
    // 分层答案以"下一个调用"收尾，因为那是实测唯一有效的指引位置（`search` 里指向 `literal` 的那条）：
    // `initialize` 里的散文 0/4 被无视。
    output.push_str(&format!(
        "next   explore {{query: \"src/{}\"}} for current implementation and test callers; check {{face}} after edits\n",
        face.source.trim_start_matches("./")
    ));
    output
}

/// Cap the complete reply on a UTF-8 boundary, with an explicit continuation instruction.
/// 在 UTF-8 边界限制完整回复，并明确给出继续查询的指令。
fn bounded(output: &str, limit: usize) -> String {
    if output.len() <= limit {
        return output.to_owned();
    }
    let mut end = limit;
    while !output.is_char_boundary(end) {
        end -= 1;
    }
    format!(
        "{}\n… truncated at {limit} bytes; focus a node or lower limit; remaining evidence and verification targets omitted\n",
        &output[..end]
    )
}

/// An empty field reads as `-` rather than as nothing at all.
/// 空字段读作 `-`，而不是什么都没有。
fn blank(value: &str) -> &str {
    if value.trim().is_empty() { "-" } else { value }
}

#[cfg(test)]
#[path = "converge_tests.rs"]
mod converge_tests;
