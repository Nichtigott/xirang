//! Join current declared responsibilities with bounded runtime neighbours.
//! 拼接当前声明责任与有界的运行期邻域。
//! online: build records cannot contain a later recorded run or unbuilt declaration edits; all joins are request-local.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fmt::Write;
use std::path::Path;

use xirang_kernel::NodeId;
use xirang_kernel::requirements::{CapabilityDeclaration, CapabilityIndex};

use crate::build_method::{FaceView, declared_grafts, overlay_projection};
use crate::mcp::apply::load_registry;
use crate::mcp::build_evidence::{BuildEvidence, pruning_line, scope_line};
use crate::run_method::{AuthoredFace, AuthoringContext};

/// A declared obligation resolved by the kernel's ancestor rule.
/// 由内核祖先规则解析的声明义务。
pub(super) struct Responsibility {
    /// Requiring object / 需求对象。
    pub consumer: NodeId,
    /// Required capability / 所需能力。
    pub capability: String,
    /// Expected provider kind / 期望提供者类型。
    pub kind: String,
    /// Matching ancestor, if readable / 匹配祖先，若可读。
    pub provider: Option<NodeId>,
}

/// One authoring read-back shared by all sections of the answer.
/// 全部回答段落共用的一次创作读回。
pub(super) struct Declarations {
    /// Authored fields by identity / 按身份保存创作字段。
    pub fields: BTreeMap<NodeId, AuthoredFace>,
    /// Resolved obligations / 已解析义务。
    pub edges: Vec<Responsibility>,
    /// Faces with unreadable fields / 字段不可读的面数。
    pub unreadable: usize,
    /// Registry rejection, without discarding other evidence / 注册机拒绝，不丢弃其他证据。
    pub rejected: Option<String>,
}

impl Declarations {
    /// Read and resolve the package's current declarations once.
    /// 一次读取并解析包的当前声明。
    pub fn read_current(root: &Path, namespace: &str, faces: &[FaceView]) -> Self {
        let mut result = Self {
            fields: BTreeMap::new(),
            edges: Vec::new(),
            unreadable: 0,
            rejected: None,
        };
        let registry = match load_registry(root, namespace) {
            Ok(registry) => registry,
            Err(error) => {
                result.rejected = Some(error);
                return result;
            }
        };
        let context = AuthoringContext::new(root.to_path_buf(), namespace.to_owned());
        let layout = crate::build_method::source_layout(root).ok();
        // Capability labels are native strings, not Rust identifiers or manifest tokens.
        // 能力标签是原生字符串，不是 Rust 标识符或清单文本记号。
        let snapshots = registry.depth_first();
        let capabilities: Vec<_> = snapshots
            .iter()
            .map(|face| CapabilityDeclaration {
                id: face.id,
                parent: Some(face.parent),
                kind: face.kind.clone(),
                provides: face.provides.clone(),
            })
            .collect();
        for face in faces {
            let fields = context.scope(|| {
                crate::run_method::authored_face(&registry, face.id).or_else(|error| {
                    let path = layout
                        .as_ref()
                        .and_then(|layout| face.locate_source(layout).ok());
                    match path.filter(|path| crate::mcp::source_index::is_safe_child(root, path)) {
                        Some(path) => AuthoredFace::read_source(&path),
                        None => Err(error),
                    }
                })
            });
            match fields {
                Ok(fields) => {
                    result.fields.insert(face.id, fields);
                }
                Err(_) => result.unreadable += 1,
            }
        }
        let index = CapabilityIndex::new(&capabilities);
        for face in snapshots {
            for requirement in &face.requires {
                let capability = &requirement.capability;
                let kind = &requirement.provider;
                result.edges.push(Responsibility {
                    consumer: face.id,
                    capability: capability.clone(),
                    kind: kind.clone(),
                    provider: index
                        .provider(face.parent, capability, kind)
                        .map(|item| item.id),
                });
            }
        }
        result
    }
}

/// Expand both directions, stopping explicitly instead of claiming a full impact closure.
/// 双向扩展，明确报告停止，不能把有界邻域说成完整影响闭包。
pub(super) fn neighbourhood(
    seeds: &BTreeSet<NodeId>,
    declarations: &Declarations,
    observed: &[(NodeId, NodeId)],
    known: &BTreeMap<NodeId, &FaceView>,
    limit: usize,
) -> (BTreeSet<NodeId>, usize) {
    let mut neighbours: BTreeMap<NodeId, BTreeSet<NodeId>> = BTreeMap::new();
    for (from, to) in declarations
        .edges
        .iter()
        .filter_map(|edge| edge.provider.map(|id| (edge.consumer, id)))
        .chain(observed.iter().copied())
    {
        if known.contains_key(&from) && known.contains_key(&to) {
            neighbours.entry(from).or_default().insert(to);
            neighbours.entry(to).or_default().insert(from);
        }
    }
    let mut queued = seeds.clone();
    let mut queue: VecDeque<_> = seeds.iter().copied().collect();
    let mut selected = BTreeSet::new();
    while selected.len() < limit {
        let Some(id) = queue.pop_front() else {
            break;
        };
        if !known.contains_key(&id) {
            continue;
        }
        selected.insert(id);
        if let Some(adjacent) = neighbours.get(&id) {
            for neighbour in adjacent {
                if queued.insert(*neighbour) {
                    queue.push_back(*neighbour);
                }
            }
        }
    }
    (selected, queue.len())
}

/// Per-object build, declaration and verification evidence for the selected neighbourhood.
/// 为选定邻域逐对象展示构建、声明与验证证据。
pub(super) fn report(
    root: &Path,
    faces: &[FaceView],
    selected: &BTreeSet<NodeId>,
    declarations: &Declarations,
    evidence: &BuildEvidence,
    observed: Option<&BTreeSet<NodeId>>,
    limit: usize,
) -> String {
    let known: BTreeMap<_, _> = faces.iter().map(|face| (face.id, face)).collect();
    let name = |id: &NodeId| known.get(id).map_or("<unknown>", |face| face.path.as_str());
    let mut output =
        String::from("responsibility chain: current declarations, not runtime call edges\n");
    if let Some(error) = &declarations.rejected {
        let _ = writeln!(output, "{}\n{error}", crate::mcp::apply::REJECTED_VERDICT);
    }
    let edges: Vec<_> = declarations
        .edges
        .iter()
        .filter(|edge| {
            selected.contains(&edge.consumer)
                || edge.provider.is_some_and(|id| selected.contains(&id))
        })
        .collect();
    for edge in edges.iter().take(limit) {
        let _ = writeln!(
            output,
            "  {} --[{} => {}]--> {}",
            name(&edge.consumer),
            edge.capability,
            edge.kind,
            edge.provider
                .as_ref()
                .map_or("UNANSWERED (no matching ancestor)", name)
        );
    }
    if edges.len() > limit {
        let _ = writeln!(
            output,
            "  {} responsibility edges withheld; raise limit or focus a node",
            edges.len() - limit
        );
    }
    if declarations.unreadable > 0 {
        let _ = writeln!(
            output,
            "unreadable faces {}; responsibility coverage incomplete",
            declarations.unreadable
        );
    }
    let declared = declared_grafts(root);
    if let Err(error) = &declared {
        let _ = writeln!(output, "graft declarations unreadable: {error}");
    }
    let projection =
        overlay_projection(root, faces, evidence.scope.as_ref(), declared.as_ref().ok());
    let slots: BTreeMap<_, _> = projection
        .as_ref()
        .ok()
        .into_iter()
        .flat_map(|projection| projection.slots.iter().chain(&projection.pruned))
        .map(|slot| (slot.id, slot))
        .collect();
    if let Err(error) = &projection {
        let _ = writeln!(output, "overlay unavailable: {error}");
    }
    let mut plans: BTreeMap<NodeId, Vec<_>> = BTreeMap::new();
    if let Ok(projection) = &projection {
        for plan in &projection.plans {
            if let Some(id) = plan.target {
                plans.entry(id).or_default().push(plan);
            } else {
                let _ = writeln!(
                    output,
                    "unattributed graft plan {}: {}; inspect grafts",
                    plan.selector,
                    plan.error.as_deref().unwrap_or("unknown target")
                );
            }
        }
    }
    let _ = writeln!(
        output,
        "build {}\nscope/pruning are from that build; configuration identity is not verified",
        evidence.freshness()
    );
    output.push_str("overlay: static projection; live replacement requires Registry::dump_effective in the linked host\n");
    for id in selected {
        let Some(face) = known.get(id) else {
            continue;
        };
        let status = match observed {
            Some(ids) if ids.contains(id) => "observed in recorded run",
            Some(_) => "not observed; absence is not proof of bypass",
            None => "runtime unknown",
        };
        let _ = writeln!(
            output,
            "object {} kind={} [{status}] source=src/{}",
            face.path, face.kind, face.source
        );
        output.push_str(&scope_line(evidence.scope.as_ref(), face));
        output.push_str(&pruning_line(evidence.pruning.as_deref(), face));
        if let Some(slot) = slots.get(id)
            && let Some(cut) = &slot.replacement
        {
            let _ = writeln!(
                output,
                "  declared graft={} full={} entry line={} (not runtime confirmation)",
                cut.graft, cut.full, cut.line
            );
        }
        if let Some(plans) = plans.get(id) {
            for plan in plans.iter().take(limit) {
                let _ = writeln!(
                    output,
                    "  graft plan {}: declared={:?} selector_matches={:?} target_path={} error={}",
                    plan.selector,
                    plan.declared,
                    plan.selector_matches,
                    plan.target_path.as_deref().unwrap_or("unknown"),
                    plan.error.as_deref().unwrap_or("none")
                );
            }
            if plans.len() > limit {
                let _ = writeln!(
                    output,
                    "  {} graft plans withheld; use grafts for detail",
                    plans.len() - limit
                );
            }
        }
        output.push_str(&super::context::contracts(face, &declarations.fields));
        let _ = writeln!(
            output,
            "verify {}: check contracts and regression tests in src/{}; replay the failing input",
            face.path, face.source
        );
    }
    output.push_str("repair obligations: test the shared provider and other consumers; for grafts compare original/replacement; for move/crate changes replay the same inputs and compare behavior. Revert the proposed cause fix to confirm the regression fails. These are obligations, not executed checks or proof of causality.\n");
    output
}
