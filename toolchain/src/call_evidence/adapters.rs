//! Adapters for mature tracing and graph backends.
//! 成熟 tracing 与图后端的适配层。

use std::collections::BTreeMap;

use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::visit::EdgeRef;
use tracing::Span;

use crate::run_method::{CallSite, CallTrace, EvidenceKind};

/// Create a structured tracing span for one XiRang call site.
/// 为一个 XiRang 调用点创建结构化 tracing span。
pub fn span_for(call: &CallSite) -> Span {
    tracing::span!(
        tracing::Level::TRACE,
        "xirang.call",
        node = %call.node,
        function = call.function,
        frame_id = call.frame_id,
    )
}

/// A graph view built from logical runtime edges.
/// 从运行时逻辑边构建的图视图。
#[derive(Debug, Default)]
pub struct CallGraph {
    graph: DiGraph<String, EvidenceKind>,
    nodes: BTreeMap<String, NodeIndex>,
}

impl CallGraph {
    /// Build a graph without copying full call paths or local values.
    /// 构建图时不复制完整调用路径或局部值。
    pub fn from_trace(trace: &CallTrace) -> Self {
        let mut graph = Self::default();
        for edge in trace.logical_call_edges() {
            let caller = format!("{}::{}", edge.caller.node, edge.caller.function);
            let callee = format!("{}::{}", edge.callee.node, edge.callee.function);
            let caller_index = graph.node(caller);
            let callee_index = graph.node(callee);
            if let Some(existing) = graph.graph.find_edge(caller_index, callee_index) {
                graph.graph[existing] = edge.evidence;
            } else {
                graph
                    .graph
                    .add_edge(caller_index, callee_index, edge.evidence);
            }
        }
        graph
    }

    /// Build the same graph from merged MIR/live relations.
    /// 从合并后的 MIR/实时关系构建同一图后端。
    pub fn from_relations(relations: &[crate::call_evidence::CallRelation]) -> Self {
        let mut graph = Self::default();
        for relation in relations {
            let caller = graph.node(relation.caller.clone());
            let callee = graph.node(relation.callee.clone());
            let evidence = relation.evidence;
            if let Some(existing) = graph.graph.find_edge(caller, callee) {
                graph.graph[existing] = evidence;
            } else {
                graph.graph.add_edge(caller, callee, evidence);
            }
        }
        graph
    }

    fn node(&mut self, name: String) -> NodeIndex {
        if let Some(index) = self.nodes.get(&name) {
            return *index;
        }
        let index = self.graph.add_node(name.clone());
        self.nodes.insert(name, index);
        index
    }

    /// Number of distinct logical functions currently in the graph; parallel
    /// invocations never add a second node.
    /// 图中当前不同逻辑函数的数量；同一次调用的多次执行不会新增第二个节点。
    pub fn node_count(&self) -> usize {
        self.graph.node_count()
    }

    /// Number of distinct logical edges in the graph; repeated invocations
    /// between the same pair collapse into one edge.
    /// 图中不同逻辑边的数量；同一对函数间的多次调用会合并为一条边。
    pub fn edge_count(&self) -> usize {
        self.graph.edge_count()
    }

    /// Export a compact DOT representation for TUI or external graph tools.
    /// 导出紧凑 DOT 表示，供 TUI 或外部图工具使用。
    pub fn to_dot(&self) -> String {
        let mut output = String::from("digraph xirang {\n");
        for index in self.graph.node_indices() {
            let name = &self.graph[index];
            output.push_str("  ");
            output.push_str(&quote_dot(name));
            output.push_str(";\n");
        }
        for edge in self.graph.edge_references() {
            output.push_str("  ");
            output.push_str(&quote_dot(&self.graph[edge.source()]));
            output.push_str(" -> ");
            output.push_str(&quote_dot(&self.graph[edge.target()]));
            output.push_str(" [label=\"");
            output.push_str(&evidence_label(*edge.weight()));
            output.push_str("\"];\n");
        }
        output.push_str("}\n");
        output
    }
}

fn evidence_label(evidence: EvidenceKind) -> String {
    format!("{} {}", evidence.marker(), evidence.label())
}

fn quote_dot(value: &str) -> String {
    format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
}

#[cfg(test)]
mod tests {
    use super::{CallGraph, span_for};
    use crate::call_evidence::{CallEvidence, CallRelation};
    use crate::call_evidence::{CallTrace, NodeId, SourceLocation};

    #[test]
    fn graph_keeps_logical_edges_without_duplicate_invocations() {
        let mut trace = CallTrace::full();
        let source = SourceLocation {
            file: "test.rs",
            line: 1,
            column: 1,
            function: "root",
        };
        trace.with_at(
            NodeId::from_path("root.rs", "Root"),
            "root",
            source,
            |trace| {
                trace.with_at(
                    NodeId::from_path("child.rs", "Child"),
                    "child",
                    source,
                    |_| {},
                );
                trace.with_at(
                    NodeId::from_path("child.rs", "Child"),
                    "child",
                    source,
                    |_| {},
                );
            },
        );
        let graph = CallGraph::from_trace(&trace);
        assert_eq!(graph.node_count(), 2);
        assert_eq!(graph.edge_count(), 1);
        assert!(graph.to_dot().contains("xirang"));
    }

    #[test]
    fn span_contains_xirang_fields() {
        let call = crate::call_evidence::CallSite {
            node: NodeId::from_path("test.rs", "Test"),
            function: "test",
            frame_id: 7,
            source: None,
        };
        let _span = span_for(&call);
    }

    #[test]
    fn all_evidence_kinds_share_marker_and_label_rendering() {
        let relations = [
            CallRelation {
                caller: "a".to_owned(),
                callee: "b".to_owned(),
                evidence: CallEvidence::Live,
                source: None,
                mir_line: None,
                caller_frame: None,
                callee_frame: None,
            },
            CallRelation {
                caller: "b".to_owned(),
                callee: "c".to_owned(),
                evidence: CallEvidence::Mir,
                source: None,
                mir_line: Some(4),
                caller_frame: None,
                callee_frame: None,
            },
            CallRelation {
                caller: "c".to_owned(),
                callee: "d".to_owned(),
                evidence: CallEvidence::Source,
                source: None,
                mir_line: None,
                caller_frame: None,
                callee_frame: None,
            },
        ];
        let dot = CallGraph::from_relations(&relations).to_dot();
        assert!(dot.contains("+ live"));
        assert!(dot.contains("? mir"));
        assert!(dot.contains("~ source"));
        assert!(CallEvidence::Live.confirmed());
        assert!(!CallEvidence::Mir.confirmed());
    }
}
