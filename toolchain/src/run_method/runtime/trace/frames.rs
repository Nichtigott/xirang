//! Persistent call-frame operations.
//! 持久化调用帧操作。

use std::fmt::Write as _;

use super::*;

/// Borrowed root-to-leaf path over the frame arena.
/// 从帧 arena 借用的根到叶调用路径。
///
/// The iterator trades repeated parent-link walks for zero heap allocation and
/// zero `CallSite` cloning. It is intended for high-frequency debug views.
/// 迭代器用重复的父链接访问换取零堆分配和零 `CallSite` 复制，供高频调试视图使用。
pub struct FramePath<'a> {
    trace: &'a CallTrace,
    target: u64,
    next_depth: usize,
    depth: usize,
}

impl<'a> Iterator for FramePath<'a> {
    type Item = &'a CallSite;

    fn next(&mut self) -> Option<Self::Item> {
        if self.next_depth > self.depth {
            return None;
        }
        let mut frame_id = self.target;
        for _ in 0..(self.depth - self.next_depth) {
            frame_id = self.trace.frame(frame_id)?.parent?;
        }
        self.next_depth += 1;
        self.trace.frame(frame_id).map(|frame| &frame.call)
    }
}

impl CallTrace {
    /// Run `operation` inside a new frame for `node`/`function`.
    /// 在 `node`/`function` 的新调用帧内执行 `operation`。
    ///
    /// The frame is pushed before the callback and popped afterwards, including
    /// when it panics, so a stale frame never captures later values. `Off` mode
    /// runs the callback without recording a frame.
    /// 回调前压入该帧、之后弹出，panic 时同样弹出，陈旧帧不会捕获后续值。`Off` 模式只
    /// 执行回调而不记录帧。
    #[track_caller]
    pub fn with<R>(
        &mut self,
        node: NodeId,
        function: &'static str,
        operation: impl FnOnce(&mut Self) -> R,
    ) -> R {
        let caller = std::panic::Location::caller();
        self.with_at(
            node,
            function,
            SourceLocation {
                file: caller.file(),
                line: caller.line(),
                column: caller.column(),
                function,
            },
            operation,
        )
    }

    /// Record a call with the source line that entered it.
    /// 记录调用及其进入位置的源码行。
    pub fn with_at<R>(
        &mut self,
        node: NodeId,
        function: &'static str,
        source: SourceLocation,
        operation: impl FnOnce(&mut Self) -> R,
    ) -> R {
        self.with_source(node, function, Some(source), operation)
    }

    fn with_source<R>(
        &mut self,
        node: NodeId,
        function: &'static str,
        source: Option<SourceLocation>,
        operation: impl FnOnce(&mut Self) -> R,
    ) -> R {
        if matches!(self.mode, TraceMode::Off) {
            return operation(self);
        }
        let frame_id = self.next_frame_id;
        self.next_frame_id = self.next_frame_id.wrapping_add(1);
        let call = CallSite {
            node,
            function,
            frame_id,
            source,
        };
        let parent = self.current.last().copied();
        self.frames.push(FrameRecord { call, parent });
        self.frame_index.insert(frame_id, self.frames.len() - 1);
        self.current.push(frame_id);
        // Pop the active frame even when the traced operation panics. A stale
        // frame would attach later values to a call that no longer exists.
        // 即使被追踪操作 panic 也要弹出活动帧，否则后续值会错误挂到已结束调用。
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| operation(self)));
        self.current.pop();
        match result {
            Ok(value) => value,
            Err(payload) => std::panic::resume_unwind(payload),
        }
    }

    pub(super) fn frame(&self, frame_id: u64) -> Option<&FrameRecord> {
        self.frame_index
            .get(&frame_id)
            .and_then(|index| self.frames.get(*index))
    }

    /// Return the number of caller frames above one invocation.
    /// 返回某个调用实例上方的调用帧数量。
    pub fn frame_depth(&self, frame_id: u64) -> usize {
        let mut depth = 0;
        let mut next = self.frame(frame_id).and_then(|frame| frame.parent);
        while let Some(id) = next {
            depth += 1;
            next = self.frame(id).and_then(|frame| frame.parent);
        }
        depth
    }

    pub(super) fn frame_chain_matches(&self, frame_id: u64, query: &str) -> bool {
        let mut next = Some(frame_id);
        while let Some(id) = next {
            let Some(frame) = self.frame(id) else { break };
            if frame.call.function.to_ascii_lowercase().contains(query)
                || frame
                    .call
                    .source
                    .is_some_and(|source| source_file_matches(source.file, query))
            {
                return true;
            }
            next = frame.parent;
        }
        false
    }

    pub(super) fn path_for(&self, frame_id: u64) -> Vec<CallSite> {
        let mut path = Vec::new();
        let mut next = Some(frame_id);
        while let Some(id) = next {
            let Some(frame) = self.frame(id) else { break };
            path.push(frame.call.clone());
            next = frame.parent;
        }
        path.reverse();
        path
    }

    /// Borrow one frame and its parent link without allocating.
    /// 无分配地借用一个调用帧及其父链接。
    pub fn frame_view(&self, frame_id: u64) -> Option<FrameView<'_>> {
        self.frame(frame_id).map(|frame| FrameView {
            call: &frame.call,
            parent: frame.parent,
        })
    }

    /// Iterate frame IDs in recording order for indexed debug views.
    /// 按记录顺序遍历帧 ID，供索引型调试视图使用。
    pub fn frame_ids(&self) -> impl Iterator<Item = u64> + '_ {
        self.frames.iter().map(|frame| frame.call.frame_id)
    }

    /// Materialize only one requested invocation path.
    /// 只生成指定调用实例的路径。
    pub fn path_for_frame(&self, frame_id: u64) -> Option<Vec<CallSite>> {
        self.frame(frame_id).map(|_| self.path_for(frame_id))
    }

    /// Borrow one invocation path without allocating a snapshot.
    /// 借用一条调用路径，不分配快照。
    pub fn path_iter(&self, frame_id: u64) -> Option<FramePath<'_>> {
        self.frame(frame_id).map(|_| FramePath {
            trace: self,
            target: frame_id,
            next_depth: 0,
            depth: self.frame_depth(frame_id),
        })
    }

    /// Borrow the currently active path without allocating.
    /// 借用当前活动调用路径，不分配临时数组。
    pub fn current_path_iter(&self) -> Option<FramePath<'_>> {
        self.current
            .last()
            .and_then(|frame_id| self.path_iter(*frame_id))
    }

    /// Visit one invocation path from its root to the selected frame without
    /// cloning `CallSite` values or allocating a temporary path vector.
    /// 从根到目标帧访问一条调用路径，不复制 `CallSite`，也不分配临时路径数组。
    ///
    /// The callback runs once per frame. Returning `false` stops the walk and
    /// makes the method return `false`; this is useful for bounded TUI renders.
    /// 回调每帧执行一次；返回 `false` 会停止遍历并让方法返回 `false`，适合限制 TUI 输出。
    pub fn visit_path<F>(&self, frame_id: u64, mut visit: F) -> bool
    where
        F: FnMut(&CallSite) -> bool,
    {
        if self.frame(frame_id).is_none() {
            return false;
        }
        self.visit_path_inner(frame_id, &mut visit)
    }

    fn visit_path_inner<F>(&self, frame_id: u64, visit: &mut F) -> bool
    where
        F: FnMut(&CallSite) -> bool,
    {
        let Some(frame) = self.frame(frame_id) else {
            return true;
        };
        if let Some(parent) = frame.parent
            && !self.visit_path_inner(parent, visit)
        {
            return false;
        }
        visit(&frame.call)
    }

    /// Clone the active root-to-leaf call path, outermost frame first.
    /// 克隆当前活动的根到叶调用路径，最外层帧在前。
    pub fn current_path(&self) -> Vec<CallSite> {
        self.current
            .iter()
            .filter_map(|id| self.frame(*id).map(|frame| frame.call.clone()))
            .collect()
    }

    /// Rebuild paths on demand; the frame arena is the persistent form.
    /// 按需重建路径；持久存储形式是帧表。
    pub fn paths(&self) -> Vec<Vec<CallSite>> {
        self.frames
            .iter()
            .map(|frame| self.path_for(frame.call.frame_id))
            .collect()
    }

    /// Lazily yield invocation paths; no path is built before it is requested.
    /// 惰性产生调用路径；只有请求某一项时才构造该路径。
    pub fn paths_iter(&self) -> impl Iterator<Item = Vec<CallSite>> + '_ {
        self.frames
            .iter()
            .map(|frame| self.path_for(frame.call.frame_id))
    }

    /// Lazily borrow every recorded path without cloning call sites.
    /// 惰性借用所有已记录路径，不复制调用点。
    pub fn borrowed_paths_iter(&self) -> impl Iterator<Item = FramePath<'_>> + '_ {
        self.frames
            .iter()
            .filter_map(|frame| self.path_iter(frame.call.frame_id))
    }

    /// Number of persistent call frames, useful for memory/debug audits.
    /// 持久化调用帧数量，便于内存和调试审计。
    pub fn frame_count(&self) -> usize {
        self.frames.len()
    }

    /// Match a function or step name without losing its complete call chain.
    /// 按函数或步骤名匹配，同时保留完整调用链。
    pub fn matching_paths(&self, query: &str) -> Vec<Vec<CallSite>> {
        self.matching_paths_iter(query).collect()
    }

    /// Lazily match call paths without materializing unrelated paths.
    /// 惰性匹配调用路径，不为无关调用生成路径。
    pub fn matching_paths_iter(&self, query: &str) -> impl Iterator<Item = Vec<CallSite>> + '_ {
        let query = query.trim().to_ascii_lowercase();
        self.frames
            .iter()
            .filter(move |frame| {
                query.is_empty()
                    || frame.call.function.to_ascii_lowercase().contains(&query)
                    || frame
                        .call
                        .source
                        .is_some_and(|source| source_file_matches(source.file, &query))
            })
            .map(|frame| self.path_for(frame.call.frame_id))
    }

    /// Render every frame as one indented line, in recording order.
    /// 按记录顺序把每个调用帧渲染成一行缩进文本。
    pub fn render_tree(&self) -> String {
        let mut output = String::new();
        for frame in &self.frames {
            let call = &frame.call;
            writeln!(
                output,
                "{}- {} {}#{}{}",
                "  ".repeat(self.frame_depth(call.frame_id)),
                call.node,
                call.function,
                call.frame_id,
                call.source
                    .map_or_else(String::new, |source| format!(" @ {source}")),
            )
            .unwrap();
        }
        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn visit_path_reads_the_frame_arena_without_materializing_a_path() {
        let mut trace = CallTrace::full();
        let root = NodeId::from_raw([1; 16]);
        let child = NodeId::from_raw([2; 16]);
        trace.with(root, "root", |trace| {
            trace.with(child, "child", |trace| trace.with(child, "leaf", |_| {}))
        });
        let target = trace.frame_ids().last().expect("leaf frame");
        let mut names = Vec::new();
        assert!(trace.visit_path(target, |call| {
            names.push(call.function);
            true
        }));
        assert_eq!(names, ["root", "child", "leaf"]);
        let borrowed = trace
            .path_iter(target)
            .expect("leaf path")
            .map(|call| call.function)
            .collect::<Vec<_>>();
        assert_eq!(borrowed, ["root", "child", "leaf"]);
    }

    #[test]
    fn disabled_mode_does_not_retain_frames_locals_or_edges() {
        let mut trace = CallTrace::disabled();
        let node = NodeId::from_raw([7; 16]);
        let result = trace.with(node, "work", |trace| {
            let input = trace.local("input", "u32", 7, LocalKind::Input);
            trace.transform(input, "output", "u32", 8)
        });
        assert_eq!(result, LocalId(0));
        assert_eq!(trace.frame_count(), 0);
        assert!(trace.locals().is_empty());
        assert!(trace.data_edges().is_empty());
    }

    #[test]
    fn errors_only_discards_success_and_keeps_failure_evidence() {
        let node = NodeId::from_raw([8; 16]);
        let mut trace = CallTrace::errors_only();
        let success: Result<(), &str> = trace.with_result(|trace| {
            trace.with(node, "successful", |trace| {
                trace.local("value", "u32", 1, LocalKind::Binding);
            });
            Ok(())
        });
        assert!(success.is_ok());
        assert_eq!(trace.frame_count(), 0);
        assert!(trace.locals().is_empty());

        let failure: Result<(), &str> = trace.with_result(|trace| {
            trace.with(node, "failed", |trace| {
                trace.local("value", "u32", 2, LocalKind::Binding);
            });
            Err("broken")
        });
        assert_eq!(failure, Err("broken"));
        assert_eq!(trace.frame_count(), 1);
        assert_eq!(trace.locals().len(), 1);
        assert_eq!(trace.locals()[0].value, "2");
    }

    /// An id that escaped a discarded scope must not describe later evidence.
    /// 逃出被丢弃作用域的 id 不得描述后来的证据。
    #[test]
    fn an_id_from_a_discarded_scope_is_never_reused() {
        let mut trace = CallTrace::errors_only();
        let escaped: Result<LocalId, &str> =
            trace.with_result(|trace| Ok(trace.local("discarded", "u32", 1, LocalKind::Binding)));
        let escaped = escaped.expect("a successful scope returns its local id");

        let mut kept = None;
        let failed: Result<(), &str> = trace.with_result(|trace| {
            kept = Some(trace.local("kept", "u32", 2, LocalKind::Binding));
            Err("broken")
        });
        assert_eq!(failed, Err("broken"));
        let kept = kept.expect("the failing scope keeps its local");
        assert_ne!(escaped, kept, "the discarded scope's id came back");
        assert!(
            trace.find_local(escaped).is_none(),
            "an id from discarded evidence must resolve to nothing"
        );
        assert_eq!(
            trace.find_local(kept).map(|local| local.value.as_str()),
            Some("2")
        );
    }

    #[test]
    fn resetting_the_mode_inside_a_scope_does_not_break_the_pairing() {
        let node = NodeId::from_raw([9; 16]);
        let mut trace = CallTrace::errors_only();
        let cleared: Result<(), &str> = trace.with_result(|trace| {
            trace.clear();
            Ok(())
        });
        assert_eq!(cleared, Ok(()));
        let changed: Result<(), &str> = trace.with_result(|trace| {
            trace.set_mode(TraceMode::Full);
            Ok(())
        });
        assert_eq!(changed, Ok(()));
        // The trace still works, and a later failure is still kept.
        let failure: Result<(), &str> = trace.with_result(|trace| {
            trace.with(node, "after-reset", |trace| {
                trace.local("value", "u32", 3, LocalKind::Binding);
            });
            Err("broken")
        });
        assert_eq!(failure, Err("broken"));
        assert_eq!(trace.locals().len(), 1);
    }

    #[test]
    fn runtime_default_matches_the_build_profile() {
        let trace = CallTrace::runtime();
        if cfg!(debug_assertions) {
            assert_eq!(trace.mode(), TraceMode::ErrorsOnly);
        } else {
            assert_eq!(trace.mode(), TraceMode::Off);
        }
    }

    #[test]
    fn trace_mode_accepts_human_facing_names() {
        assert_eq!(TraceMode::parse_trace_mode("off"), Some(TraceMode::Off));
        assert_eq!(
            TraceMode::parse_trace_mode("errors_only"),
            Some(TraceMode::ErrorsOnly)
        );
        assert_eq!(TraceMode::parse_trace_mode("FULL"), Some(TraceMode::Full));
        assert_eq!(TraceMode::parse_trace_mode("verbose"), None);
    }
}
