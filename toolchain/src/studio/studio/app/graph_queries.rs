//! Call graph and evidence queries owned by App.
//! App 所有的调用图与证据查询。
//!
//! The tree and the value pane are built from the same `call_relations`, so they
//! cannot disagree about who calls whom; the tree itself lives in
//! `call_tree_queries`.
//! 树与取值面板由同一个 `call_relations` 构建，因此它们在"谁调用谁"上不可能不一致；调用树
//! 本身位于 `call_tree_queries`。

use super::*;

impl App {
    /// Extract a few value-changing statements for the compact call tree.
    /// 提取紧凑调用树中少量改变值的语句。
    ///
    /// Only the widget drawer's status row reads this, so it is compiled with the
    /// `node-graph` feature; without the feature the tree has no cursor row.
    /// 只有控件绘制方的状态行读它，因此与 `node-graph` 特性一同编译；没有该特性时调用树没有
    /// 游标行。
    #[cfg(feature = "node-graph")]
    pub(crate) fn call_tree_transforms(&self, item: &CallRef) -> Vec<String> {
        let Some(info) = self.registry.find_registry(item.node) else {
            return Vec::new();
        };
        let source = source_path_for(&info.source.file);
        let Ok(text) = std::fs::read_to_string(source) else {
            return Vec::new();
        };
        let lines = text.lines().collect::<Vec<_>>();
        function_source_range(&lines, &item.function)
            .map(|(start, end)| {
                lines[start..=end]
                    .iter()
                    .filter_map(|line| {
                        let trimmed = line.trim();
                        (trimmed.starts_with("let ") || trimmed.starts_with("return "))
                            .then_some(trimmed.to_owned())
                    })
                    .take(4)
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Find real function-to-function edges from source bodies. Registration
    /// parents and `use` statements are intentionally ignored.
    /// 从源码函数体提取真实函数调用边，明确忽略注册父子关系和 `use`。
    pub fn call_relations(&self, center: NodeId, function: &str) -> (Vec<CallRef>, Vec<CallRef>) {
        let mut definitions = Vec::new();
        let mut bodies = Vec::new();
        for info in self.registry.depth_first() {
            let source = source_path_for(&info.source.file);
            let Ok(text) = std::fs::read_to_string(source) else {
                continue;
            };
            for function in function_symbols(&text) {
                definitions.push((function.name.clone(), info.id));
                bodies.push((function.name, info.id, function.body));
            }
        }
        let target_name = if function.is_empty() {
            self.registry
                .find_registry(center)
                .map(|info| info.source.function.as_str())
                .unwrap_or("")
        } else {
            function
        };
        let mut callers = Vec::new();
        let mut callees = Vec::new();
        for (name, node, body) in bodies {
            let calls_target =
                body_calls(&body, target_name) && !(node == center && name == target_name);
            if calls_target {
                let file = self
                    .registry
                    .find_registry(node)
                    .map(|info| info.source.file.as_str())
                    .unwrap_or("");
                let reference = CallRef {
                    node,
                    function: name.clone(),
                    file: file.to_owned(),
                };
                if !callers
                    .iter()
                    .any(|item: &CallRef| item.node == node && item.function == reference.function)
                {
                    callers.push(reference);
                }
            }
            if node == center && name == target_name {
                for (callee, callee_node) in &definitions {
                    if callee != target_name
                        && body_calls(&body, callee)
                        && !callees.iter().any(|item: &CallRef| {
                            item.node == *callee_node && item.function == *callee
                        })
                    {
                        let file = self
                            .registry
                            .find_registry(*callee_node)
                            .map(|info| info.source.file.as_str())
                            .unwrap_or("");
                        callees.push(CallRef {
                            node: *callee_node,
                            function: callee.clone(),
                            file: file.to_owned(),
                        });
                    }
                }
            }
        }
        for edge in self.runtime_trace.call_edges() {
            if edge.callee.node == center && edge.callee.function == target_name {
                push_call_ref(
                    &self.registry,
                    &mut callers,
                    edge.caller.node,
                    edge.caller.function,
                );
            }
            if edge.caller.node == center && edge.caller.function == target_name {
                push_call_ref(
                    &self.registry,
                    &mut callees,
                    edge.callee.node,
                    edge.callee.function,
                );
            }
        }
        if let Some(graph) = &self.mir_graph {
            for call in &graph.calls {
                if same_symbol(&call.callee, target_name) {
                    self.push_resolved_mir_refs(&mut callers, &definitions, &call.caller);
                }
                if same_symbol(&call.caller, target_name) {
                    self.push_resolved_mir_refs(&mut callees, &definitions, &call.callee);
                }
            }
        }
        (callers, callees)
    }

    fn push_resolved_mir_refs(
        &self,
        output: &mut Vec<CallRef>,
        definitions: &[(String, NodeId)],
        symbol: &str,
    ) {
        for (name, node) in definitions {
            if same_symbol(symbol, name) {
                push_call_ref(&self.registry, output, *node, name);
            }
        }
    }

    /// Classify the strongest evidence behind a caller-to-callee edge.
    /// 判定一条调用者到被调用者边的最强证据。
    ///
    /// Prefers a live trace edge, then a MIR call, and falls back to source.
    /// 优先实时追踪边，其次 MIR 调用，最后退回源码。
    pub fn call_evidence(&self, caller: &CallRef, callee: &CallRef) -> CallEvidence {
        if self.runtime_trace.call_edges().iter().any(|edge| {
            edge.caller.node == caller.node
                && edge.caller.function == caller.function
                && edge.callee.node == callee.node
                && edge.callee.function == callee.function
        }) {
            return CallEvidence::Live;
        }
        if self.mir_graph.as_ref().is_some_and(|graph| {
            graph.calls.iter().any(|edge| {
                same_symbol(&edge.caller, &caller.function)
                    && same_symbol(&edge.callee, &callee.function)
            })
        }) {
            return CallEvidence::Mir;
        }
        CallEvidence::Source
    }

    /// Whether the node declares a nested registry owned by that face.
    /// 该节点是否声明了一个由该注册面拥有的嵌套注册表。
    pub fn owns_registry(&self, id: NodeId) -> bool {
        self.registry.registry(id).is_some()
    }
}

impl App {
    pub(crate) fn graph_locals(&self, item: &CallRef) -> Vec<crate::run_method::LocalValue> {
        self.runtime_trace
            .locals()
            .iter()
            .filter(|local| {
                self.runtime_trace
                    .path_for_local(crate::run_method::LocalId(local.id))
                    .iter()
                    .any(|call| call.function == item.function)
            })
            .cloned()
            .collect()
    }
}
