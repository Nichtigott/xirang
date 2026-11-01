//! Observed value edges and provenance queries.
//! 已观测值边与来源查询。

use std::collections::{BTreeSet, VecDeque};
use std::fmt::Write as _;

use super::*;

/// One value transformation observed by the trace.
/// 追踪中观察到的一次值变换。
///
/// `source` is optional only for adapters that cannot provide a callsite;
/// built-in transformation APIs always fill it.
/// `source` 仅在外部适配器无法提供调用点时为空；内置变换 API 总会填充它。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DataEdge {
    /// The source local id this edge leaves.
    /// 该边离开的源局部值 id。
    pub from: u64,
    /// The destination local id this edge enters.
    /// 该边进入的目标局部值 id。
    pub to: u64,
    /// The transformation kind, for example `transform` or `used by f::p`.
    /// 变换种类，例如 `transform` 或 `used by f::p`。
    pub label: String,
    /// The callsite that produced the edge, when one is known.
    /// 产生该边的调用点（若可知）。
    pub source: Option<SourceLocation>,
}

/// One end of a data edge together with the local reached through it.
/// 数据边的一端，以及经该边到达的局部值。
#[derive(Clone, Copy, Debug)]
pub struct DataHop<'a> {
    /// The edge being traversed.
    /// 正在遍历的边。
    pub edge: &'a DataEdge,
    /// The local at this end, `None` when it was not recorded.
    /// 该端的局部值；未记录时为 `None`。
    pub value: Option<&'a LocalValue>,
}

impl CallTrace {
    /// Record `input` transformed into a new binding; returns the new local.
    /// 记录 `input` 变换出的新绑定，返回新局部值。
    ///
    /// The callsite is taken from the caller; in `Off` mode nothing is recorded
    /// and `LocalId(0)` is returned.
    /// 调用点取自调用方；`Off` 模式下不记录任何内容并返回 `LocalId(0)`。
    #[track_caller]
    pub fn transform(
        &mut self,
        input: LocalId,
        name: impl Into<String>,
        type_name: impl Into<String>,
        value: impl std::fmt::Display,
    ) -> LocalId {
        let caller = std::panic::Location::caller();
        self.transform_at_callsite(
            input,
            name,
            type_name,
            value.to_string(),
            caller.file(),
            caller.line(),
            caller.column(),
        )
    }

    /// `transform` with an explicit callsite for adapters.
    /// 带显式调用点的 `transform`，供适配器使用。
    #[allow(clippy::too_many_arguments)]
    pub fn transform_at_callsite(
        &mut self,
        input: LocalId,
        name: impl Into<String>,
        type_name: impl Into<String>,
        value: impl Into<String>,
        file: &'static str,
        line: u32,
        column: u32,
    ) -> LocalId {
        if matches!(self.mode, TraceMode::Off) {
            return LocalId(0);
        }
        let source = self.callsite_source(file, line, column);
        let output = self.local_at(name, type_name, value, LocalKind::Binding, source);
        self.push_edge(DataEdge {
            from: input.0,
            to: output.0,
            label: "transform".to_owned(),
            source: Some(source),
        });
        output
    }

    /// Record `input` as returned by the current call; returns the returned local.
    /// 记录当前调用返回 `input`，返回代表返回值的局部值。
    #[track_caller]
    pub fn return_value(
        &mut self,
        input: LocalId,
        name: impl Into<String>,
        type_name: impl Into<String>,
        value: impl std::fmt::Display,
    ) -> LocalId {
        let caller = std::panic::Location::caller();
        self.return_at_callsite(
            input,
            name,
            type_name,
            value.to_string(),
            caller.file(),
            caller.line(),
            caller.column(),
        )
    }

    /// `return_value` with an explicit callsite for adapters.
    /// 带显式调用点的 `return_value`，供适配器使用。
    #[allow(clippy::too_many_arguments)]
    pub fn return_at_callsite(
        &mut self,
        input: LocalId,
        name: impl Into<String>,
        type_name: impl Into<String>,
        value: impl Into<String>,
        file: &'static str,
        line: u32,
        column: u32,
    ) -> LocalId {
        if matches!(self.mode, TraceMode::Off) {
            return LocalId(0);
        }
        let source = self.callsite_source(file, line, column);
        let output = self.local_at(name, type_name, value, LocalKind::Output, source);
        self.push_edge(DataEdge {
            from: input.0,
            to: output.0,
            label: "return".to_owned(),
            source: Some(source),
        });
        output
    }

    /// Record `input` consumed as `parameter` of `function`.
    /// 记录 `input` 作为 `function` 的 `parameter` 被消费。
    ///
    /// Returns the consumer local, which is what later queries see as the
    /// downstream end of this edge.
    /// 返回消费者局部值，后续查询会把它当作该边的下游端点。
    #[track_caller]
    pub fn consume(
        &mut self,
        input: LocalId,
        function: impl Into<String>,
        parameter: impl Into<String>,
    ) -> LocalId {
        let caller = std::panic::Location::caller();
        self.consume_at_callsite(
            function,
            parameter,
            input,
            caller.file(),
            caller.line(),
            caller.column(),
        )
    }

    /// `consume` with an explicit callsite for adapters.
    /// 带显式调用点的 `consume`，供适配器使用。
    pub fn consume_at_callsite(
        &mut self,
        function: impl Into<String>,
        parameter: impl Into<String>,
        input: LocalId,
        file: &'static str,
        line: u32,
        column: u32,
    ) -> LocalId {
        if matches!(self.mode, TraceMode::Off) {
            return LocalId(0);
        }
        let source = self.callsite_source(file, line, column);
        let function = function.into();
        let parameter = parameter.into();
        let output = self.local_at(
            parameter.clone(),
            "consumer",
            format!("{function}::{parameter}"),
            LocalKind::Consumer,
            source,
        );
        self.push_edge(DataEdge {
            from: input.0,
            to: output.0,
            label: format!("used by {function}::{parameter}"),
            source: Some(source),
        });
        output
    }

    /// All recorded value edges, in insertion order.
    /// 按写入顺序返回全部已记录的值边。
    pub fn data_edges(&self) -> &[DataEdge] {
        &self.edges
    }

    fn push_edge(&mut self, edge: DataEdge) {
        let index = self.edges.len();
        self.outgoing_index
            .entry(edge.from)
            .or_default()
            .push(index);
        self.incoming_index.entry(edge.to).or_default().push(index);
        self.edges.push(edge);
    }

    /// Every local `id` transitively depends on, including itself, producers first.
    /// `id` 传递依赖的全部局部值（含自身），生产者在前。
    pub fn provenance(&self, id: LocalId) -> Vec<&LocalValue> {
        let mut result = Vec::new();
        self.visit_upstream(id.0, &mut BTreeSet::new(), &mut result);
        result
    }

    /// Edges that consume `id` directly, without following them onward.
    /// 直接消费 `id` 的边，不继续向后追踪。
    pub fn consumers(&self, id: LocalId) -> Vec<&DataEdge> {
        self.outgoing_index
            .get(&id.0)
            .into_iter()
            .flatten()
            .filter_map(|index| self.edges.get(*index))
            .collect()
    }

    /// Direct consumers of `id`, each paired with the local it produced.
    /// `id` 的直接消费者，各自与它产出的局部值配对。
    pub fn outgoing(&self, id: LocalId) -> Vec<DataHop<'_>> {
        self.consumers(id)
            .into_iter()
            .map(|edge| DataHop {
                edge,
                value: self.find_local(LocalId(edge.to)),
            })
            .collect()
    }

    /// Direct producers of `id`, each paired with the local it came from.
    /// `id` 的直接生产者，各自与它来自的局部值配对。
    pub fn incoming(&self, id: LocalId) -> Vec<DataHop<'_>> {
        self.incoming_index
            .get(&id.0)
            .into_iter()
            .flatten()
            .filter_map(|index| self.edges.get(*index))
            .map(|edge| DataHop {
                edge,
                value: self.find_local(LocalId(edge.from)),
            })
            .collect()
    }

    /// Every local reachable from `id`, nearest consumers first, de-duplicated.
    /// 从 `id` 可到达的全部局部值，最近的消费者在前，且不重复。
    pub fn downstream(&self, id: LocalId) -> Vec<&LocalValue> {
        let mut result = Vec::new();
        let mut queue = VecDeque::from([id.0]);
        let mut visited = BTreeSet::from([id.0]);
        while let Some(current) = queue.pop_front() {
            for index in self.outgoing_index.get(&current).into_iter().flatten() {
                let Some(edge) = self.edges.get(*index) else {
                    continue;
                };
                if !visited.insert(edge.to) {
                    continue;
                }
                if let Some(local) = self.find_local(LocalId(edge.to)) {
                    result.push(local);
                }
                queue.push_back(edge.to);
            }
        }
        result
    }

    /// Render `id` and its upstream/downstream neighborhood as text.
    /// 以文本渲染 `id` 及其上下游邻域。
    ///
    /// An unknown id renders a single `local <id> not found` line instead of
    /// panicking.
    /// 未知 id 只渲染一行 `local <id> not found`，不会 panic。
    pub fn render_provenance(&self, id: LocalId) -> String {
        let mut output = String::new();
        let Some(selected) = self.find_local(id) else {
            return format!("local {} not found", id.0);
        };
        writeln!(
            output,
            "value {} {} = {} ({}) [{}] @ {}",
            selected.id,
            selected.name,
            selected.value,
            selected.type_name,
            selected.observation.label(),
            selected.source
        )
        .unwrap();
        output.push_str("upstream:\n");
        self.render_upstream(id.0, 1, &mut BTreeSet::new(), &mut output);
        output.push_str("downstream:\n");
        self.render_downstream(id.0, 1, &mut BTreeSet::new(), &mut output);
        output
    }

    fn render_upstream(
        &self,
        id: u64,
        depth: usize,
        visited: &mut BTreeSet<u64>,
        output: &mut String,
    ) {
        if !visited.insert(id) {
            return;
        }
        for hop in self.incoming(LocalId(id)) {
            let source = hop
                .value
                .map(|local| format!("{} {} = {}", local.id, local.name, local.value))
                .unwrap_or_else(|| format!("local {}", hop.edge.from));
            writeln!(
                output,
                "{} `-- {source} [{}]",
                "  ".repeat(depth),
                hop.edge.label
            )
            .unwrap();
            self.render_upstream(hop.edge.from, depth + 1, visited, output);
        }
    }

    fn render_downstream(
        &self,
        id: u64,
        depth: usize,
        visited: &mut BTreeSet<u64>,
        output: &mut String,
    ) {
        if !visited.insert(id) {
            return;
        }
        for hop in self.outgoing(LocalId(id)) {
            let target = hop
                .value
                .map(|local| format!("{} {} = {}", local.id, local.name, local.value))
                .unwrap_or_else(|| format!("local {}", hop.edge.to));
            writeln!(
                output,
                "{} `-- {target} [{}]",
                "  ".repeat(depth),
                hop.edge.label
            )
            .unwrap();
            self.render_downstream(hop.edge.to, depth + 1, visited, output);
        }
    }

    fn visit_upstream<'a>(
        &'a self,
        id: u64,
        visited: &mut BTreeSet<u64>,
        result: &mut Vec<&'a LocalValue>,
    ) {
        if !visited.insert(id) {
            return;
        }
        for index in self.incoming_index.get(&id).into_iter().flatten() {
            if let Some(edge) = self.edges.get(*index) {
                self.visit_upstream(edge.from, visited, result);
            }
        }
        if let Some(local) = self.find_local(LocalId(id)) {
            result.push(local);
        }
    }

    /// Render every recorded local and edge as one text block.
    /// 把全部已记录局部值与边渲染成一段文本。
    pub fn render_data_flow(&self) -> String {
        let mut output = String::new();
        for local in &self.locals {
            let frame = local
                .frame_id
                .and_then(|id| self.frame(id))
                .map(|frame| frame.call.function)
                .unwrap_or("<outside-call>");
            writeln!(
                output,
                "{} [{}|{}] {}#{} depth={}: {} = {} @ {} ({})",
                local.id,
                local.kind.label(),
                local.observation.label(),
                frame,
                local.frame_id.unwrap_or(0),
                local.frame_id.map_or(0, |id| self.frame_depth(id)),
                local.name,
                local.value,
                local.source,
                local.type_name
            )
            .unwrap();
        }
        if !self.edges.is_empty() {
            output.push_str("edges:\n");
            for edge in &self.edges {
                let source = edge
                    .source
                    .map_or_else(String::new, |location| format!(" @ {location}"));
                writeln!(
                    output,
                    "  {} -> {} ({}){}",
                    edge.from, edge.to, edge.label, source
                )
                .unwrap();
            }
        }
        output
    }
}
