//! Local-value recording and query methods on `CallTrace`.
//! `CallTrace` 上的局部值记录与查询方法。

use std::collections::BTreeSet;

use crate::run_method::registry_core::declaration::{SourceLocation, source_file_matches};
use crate::run_method::runtime::trace::{CallSite, CallTrace, TraceMode};

use super::{LocalId, LocalKind, LocalValue, Observation};

impl CallTrace {
    /// Record a local, taking file/line/column from the caller's location.
    /// 记录一个局部值，文件、行、列取自调用方位置。
    ///
    /// Returns `LocalId(0)` and stores nothing when the trace is `Off`.
    /// 追踪为 `Off` 时不存储任何内容并返回 `LocalId(0)`。
    #[track_caller]
    pub fn local(
        &mut self,
        name: impl Into<String>,
        type_name: impl Into<String>,
        value: impl std::fmt::Display,
        kind: LocalKind,
    ) -> LocalId {
        let caller = std::panic::Location::caller();
        self.local_at_callsite(
            name,
            type_name,
            value.to_string(),
            kind,
            caller.file(),
            caller.line(),
            caller.column(),
        )
    }

    /// Record an observed local at an explicit source location.
    /// 在显式源码位置记录一个已观测局部值。
    pub fn local_at(
        &mut self,
        name: impl Into<String>,
        type_name: impl Into<String>,
        value: impl Into<String>,
        kind: LocalKind,
        source: SourceLocation,
    ) -> LocalId {
        self.local_with_observation(name, type_name, value, kind, source, Observation::Observed)
    }

    fn local_with_observation(
        &mut self,
        name: impl Into<String>,
        type_name: impl Into<String>,
        value: impl Into<String>,
        kind: LocalKind,
        source: SourceLocation,
        observation: Observation,
    ) -> LocalId {
        if matches!(self.mode, TraceMode::Off) {
            return LocalId(0);
        }
        let id = LocalId(self.next_local_id);
        self.next_local_id = self.next_local_id.wrapping_add(1);
        let local = LocalValue {
            id: id.0,
            name: name.into(),
            type_name: type_name.into(),
            value: value.into(),
            kind,
            source,
            frame_id: self.current.last().copied(),
            observation,
        };
        self.locals.push(local);
        self.local_index.insert(id.0, self.locals.len() - 1);
        // The invariant the `expect` rests on, stated where the distance is zero:
        // this is the value pushed two lines up, nothing can remove it in between
        // (both statements above only insert), and the three indexes below only
        // read it. Audited as the one production panic site in this file (S16);
        // it stays an `expect` because a caller cannot make it fire.
        // `expect` 所依赖的不变量，写在距离为零的地方：这就是上面两行压入的值，中间没有任何东西
        // 能移除它（上面两条语句都只插入），而下面三个索引只读取它。审计把它记为本文件唯一的
        // 生产 panic 点（S16）；它保留为 `expect`，因为调用方无法让它触发。
        let local = self.locals.last().expect("local was just pushed");
        self.local_name_index
            .entry(local.name.clone())
            .or_default()
            .push(id.0);
        if let Some(frame_id) = local.frame_id
            && let Some(function) = self.frame(frame_id).map(|frame| frame.call.function)
        {
            self.local_function_index
                .entry(function)
                .or_default()
                .push(id.0);
        }
        id
    }

    /// Add a MIR candidate whose runtime value was not observed.
    /// 添加运行时未观察到的 MIR 候选局部变量。
    pub fn inferred_local(
        &mut self,
        function: impl Into<String>,
        name: impl Into<String>,
        type_name: impl Into<String>,
        line: u32,
    ) -> LocalId {
        if matches!(self.mode, TraceMode::Off) {
            return LocalId(0);
        }
        let function = function.into();
        let name = name.into();
        self.local_with_observation(
            format!("{function}::{name}"),
            type_name,
            "<not observed>",
            LocalKind::Binding,
            SourceLocation {
                file: "<rustc-mir>",
                line,
                column: 0,
                function: "<rustc-mir>",
            },
            Observation::Unobserved,
        )
    }

    /// Record an observed local with an explicit callsite.
    /// 用显式调用点记录一个已观测局部值。
    ///
    /// This is the `#[track_caller]`-free entry point for adapters, and it is
    /// what `local` delegates to; the function name is read from the active
    /// frame, or `"<local>"` outside any traced call.
    /// 这是供适配器使用的、不依赖 `#[track_caller]` 的入口，也是 `local` 的委托目标；
    /// 函数名取自活动帧，不在任何被追踪调用内时为 `"<local>"`。
    #[allow(clippy::too_many_arguments)]
    pub fn local_at_callsite(
        &mut self,
        name: impl Into<String>,
        type_name: impl Into<String>,
        value: impl Into<String>,
        kind: LocalKind,
        file: &'static str,
        line: u32,
        column: u32,
    ) -> LocalId {
        if matches!(self.mode, TraceMode::Off) {
            return LocalId(0);
        }
        let function = self
            .current
            .last()
            .and_then(|id| self.frame(*id))
            .map(|frame| frame.call.function)
            .unwrap_or("<local>");
        self.local_at(
            name,
            type_name,
            value,
            kind,
            SourceLocation {
                file,
                line,
                column,
                function,
            },
        )
    }

    /// Every local recorded so far, in recording order.
    /// 按记录顺序返回目前记录的全部局部值。
    pub fn locals(&self) -> &[LocalValue] {
        &self.locals
    }

    /// The root-to-leaf call path that encloses `id`, empty when unknown.
    /// 包含 `id` 的根到叶调用路径；未知时为空。
    pub fn path_for_local(&self, id: LocalId) -> Vec<CallSite> {
        self.find_local(id)
            .and_then(|local| local.frame_id)
            .map_or_else(Vec::new, |frame_id| self.path_for(frame_id))
    }

    /// Look up one local by id, `None` when the id describes no live evidence.
    /// 按 id 查找一个局部值；该 id 不对应任何现存证据时为 `None`。
    pub fn find_local(&self, id: LocalId) -> Option<&LocalValue> {
        self.local_index
            .get(&id.0)
            .and_then(|index| self.locals.get(*index))
    }

    /// Locals matching a case-insensitive name, function, type, or value query.
    /// 匹配大小写不敏感的名称、函数、类型或取值查询的局部值。
    ///
    /// An empty query returns every local; an exact name or function match wins
    /// over the substring search, so a caller sees the precise hit first.
    /// 空查询返回全部局部值；名称或函数的精确匹配优先于子串搜索，调用方先看到精确命中。
    pub fn matching_locals(&self, query: &str) -> Vec<&LocalValue> {
        let query = query.trim().to_ascii_lowercase();
        if query.is_empty() {
            return self.locals.iter().collect();
        }
        let exact_ids = self
            .local_name_index
            .iter()
            .filter(|(name, _)| name.to_ascii_lowercase() == query)
            .flat_map(|(_, ids)| ids.iter().copied())
            .chain(
                self.local_function_index
                    .iter()
                    .filter(|(function, _)| function.to_ascii_lowercase() == query)
                    .flat_map(|(_, ids)| ids.iter().copied()),
            )
            .collect::<BTreeSet<_>>();
        if !exact_ids.is_empty() {
            return exact_ids
                .into_iter()
                .filter_map(|id| self.find_local(LocalId(id)))
                .collect();
        }
        self.locals
            .iter()
            .filter(|local| {
                local.name.to_ascii_lowercase().contains(&query)
                    || local.type_name.to_ascii_lowercase().contains(&query)
                    || local.value.to_ascii_lowercase().contains(&query)
                    || source_file_matches(local.source.file, &query)
                    || local.source.function.to_ascii_lowercase().contains(&query)
                    || local
                        .frame_id
                        .is_some_and(|frame_id| self.frame_chain_matches(frame_id, &query))
            })
            .collect()
    }
}
