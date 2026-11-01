//! Runtime evidence joined by object identity; source locations remain callsites.
//! 按对象身份拼接运行证据；源码位置仍表示调用点。
//! online: parent frames and recorded values only exist after a host run, independently of build records.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write;

use xirang_kernel::NodeId;

use crate::build_method::FaceView;
use crate::run_method::{LocalValue, TraceArtifact, TraceFrame};

/// Index a recording once for both neighbourhood expansion and evidence rendering.
/// 对记录一次建表，用于邻域扩展与证据渲染。
pub(super) struct Runtime<'a> {
    artifact: &'a TraceArtifact,
    frames: BTreeMap<u64, &'a TraceFrame>,
    values: BTreeMap<u64, &'a LocalValue>,
    /// Object identities seen in this recording / 本次记录出现的对象身份。
    pub observed: BTreeSet<NodeId>,
    /// Call and value neighbours, distinct from declared obligations / 调用和值邻域，区别于声明义务。
    pub neighbours: Vec<(NodeId, NodeId)>,
}

impl<'a> Runtime<'a> {
    /// Build request-local frame and value lookups.
    /// 建立请求内的帧和值查找表。
    pub fn new(artifact: &'a TraceArtifact) -> Self {
        let frames: BTreeMap<_, _> = artifact
            .frames
            .iter()
            .map(|frame| (frame.frame_id, frame))
            .collect();
        let values: BTreeMap<_, _> = artifact
            .locals
            .iter()
            .map(|value| (value.id, value))
            .collect();
        let observed = artifact.frames.iter().map(|frame| frame.node).collect();
        let mut neighbours: Vec<_> = artifact
            .frames
            .iter()
            .filter_map(|frame| Some((frames.get(&frame.parent?)?.node, frame.node)))
            .collect();
        for edge in &artifact.edges {
            let owner = |id| {
                frames
                    .get(&values.get(&id)?.frame_id?)
                    .map(|frame| frame.node)
            };
            if let (Some(from), Some(to)) = (owner(edge.from), owner(edge.to)) {
                neighbours.push((from, to));
            }
        }
        Self {
            artifact,
            frames,
            values,
            observed,
            neighbours,
        }
    }

    /// Render selected observations with their provenance and omission counts.
    /// 渲染选定观察，附带来源和省略数量。
    pub fn report(
        &self,
        known: &BTreeMap<NodeId, &FaceView>,
        selected: &BTreeSet<NodeId>,
        limit: usize,
    ) -> String {
        let name = |id: NodeId| {
            known.get(&id).map_or_else(
                || format!("<root-or-unmapped:{id}>"),
                |face| face.path.clone(),
            )
        };
        let owner = |value: &LocalValue| {
            value
                .frame_id
                .and_then(|id| self.frames.get(&id))
                .map(|frame| frame.node)
        };
        let value_name = |id| {
            self.values.get(&id).map_or_else(
                || format!("<missing-value:{id}>"),
                |value| {
                    format!(
                        "{}::{}",
                        owner(value).map_or_else(|| "<unattributed>".to_owned(), name),
                        value.name
                    )
                },
            )
        };
        let ran = self
            .observed
            .iter()
            .filter(|id| known.contains_key(id))
            .count();
        let mut output = format!(
            "trace identity verified; source/config/input freshness unknown; historical observations do not prove causality\nfaces that ran ({ran} of {} declared, matched by NodeId)\n",
            known.len()
        );
        let mut by_node: BTreeMap<NodeId, Vec<&TraceFrame>> = BTreeMap::new();
        for frame in &self.artifact.frames {
            if selected.contains(&frame.node) {
                by_node.entry(frame.node).or_default().push(frame);
            }
        }
        for (id, frames) in &by_node {
            let _ = writeln!(
                output,
                "  {} kind={} {} frame(s)",
                name(*id),
                known[id].kind,
                frames.len()
            );
        }
        let frames: Vec<_> = self
            .artifact
            .frames
            .iter()
            .filter(|frame| {
                selected.contains(&frame.node)
                    || frame
                        .parent
                        .and_then(|id| self.frames.get(&id))
                        .is_some_and(|parent| selected.contains(&parent.node))
            })
            .collect();
        let _ = writeln!(output, "observed call paths ({})", frames.len());
        for frame in frames.iter().take(limit) {
            let from = frame.parent.and_then(|id| self.frames.get(&id));
            let prefix = match from {
                Some(parent) => format!("{}::{} -> ", name(parent.node), parent.function),
                None if frame.parent.is_some() => "<missing-parent-frame> -> ".to_owned(),
                None => String::new(),
            };
            let _ = writeln!(
                output,
                "  {prefix}{}::{} [frame {} parent {:?}]",
                name(frame.node),
                frame.function,
                frame.frame_id,
                frame.parent
            );
            if let Some(source) = frame.source {
                let _ = writeln!(
                    output,
                    "    callsite {}:{} (not callee ownership)",
                    source.file, source.line
                );
            }
        }
        withheld(&mut output, frames.len(), limit, "frames");
        let locals: Vec<_> = self
            .artifact
            .locals
            .iter()
            .filter(|value| owner(value).is_some_and(|id| selected.contains(&id)))
            .collect();
        let _ = writeln!(output, "values ({})", locals.len());
        for value in locals.iter().take(limit) {
            let _ = writeln!(
                output,
                "  {} [value {} frame {:?}] {}",
                value_name(value.id),
                value.id,
                value.frame_id,
                crate::mcp::trace::local_line(value).trim()
            );
        }
        withheld(&mut output, locals.len(), limit, "values");
        let edges: Vec<_> = self
            .artifact
            .edges
            .iter()
            .filter(|edge| {
                [edge.from, edge.to].into_iter().any(|id| {
                    self.values
                        .get(&id)
                        .and_then(|value| owner(value))
                        .is_some_and(|id| selected.contains(&id))
                })
            })
            .collect();
        let _ = writeln!(output, "edges ({})", edges.len());
        for edge in edges.iter().take(limit) {
            let _ = writeln!(
                output,
                "  {} -> {}  ({}) [values {} -> {}]",
                value_name(edge.from),
                value_name(edge.to),
                edge.label,
                edge.from,
                edge.to
            );
        }
        withheld(&mut output, edges.len(), limit, "data edges");
        let outside = self
            .artifact
            .frames
            .iter()
            .filter(|frame| !known.contains_key(&frame.node))
            .count();
        let missing_parents = self
            .artifact
            .frames
            .iter()
            .filter(|frame| {
                frame
                    .parent
                    .is_some_and(|id| !self.frames.contains_key(&id))
            })
            .count();
        let unowned = self
            .artifact
            .locals
            .iter()
            .filter(|value| owner(value).is_none())
            .count();
        let dangling = self
            .artifact
            .edges
            .iter()
            .filter(|edge| {
                !self.values.contains_key(&edge.from) || !self.values.contains_key(&edge.to)
            })
            .count();
        let _ = writeln!(
            output,
            "frames in a face {} / outside any declared face {outside}; missing parents {missing_parents}; unattributed values {unowned}; dangling data edges {dangling}",
            self.artifact.frames.len() - outside
        );
        output
    }
}

fn withheld(output: &mut String, total: usize, limit: usize, label: &str) {
    if total > limit {
        let _ = writeln!(
            output,
            "  {} {label} withheld; raise limit or use trace for detail",
            total - limit
        );
    }
}
