//! covers `app/trace.rs` — the fixture traces the other pages install.
//! 覆盖 `app/trace.rs`——其他页面装入的夹具追踪。
//!
//! Live-trace fixtures for the prototype-fixture tests.
//! 原型夹具测试使用的实时追踪夹具。
//!
//! A test asserting about *live* evidence has to build a trace over the faces it
//! loaded. Studio no longer ships a demo trace — the sample was removed when the
//! artifact loader landed — so these tests install the trace they mean by hand.
//! The alternative this replaced pinned a sample that recorded no locals for any
//! host project: `graph_locals` always returned nothing and `call_evidence` could
//! never answer `Live`.
//! 断言**实测**证据的测试必须为自己加载的注册面构造追踪。Studio 不再自带演示追踪——artifact
//! 加载方落地时示例已被删除——因此这些测试手工装入它们所指的那条追踪。它所取代的做法钉住的是一条
//! 对任何宿主项目都不记录局部值的样本：`graph_locals` 永远返回空，`call_evidence` 也永远无法
//! 回答 `Live`。

use super::CallRef;
use crate::call_evidence::{CallTrace, LocalKind, SourceLocation};

/// A live trace over the fixture project: the caller is entered, two locals are
/// recorded inside its frame, and it calls the callee.
/// 一条针对夹具项目的实时追踪：进入调用方，在其帧内记录两个局部值，然后它调用被调方。
///
/// Two locals, not one: the DATA panel's cursor stops at `data_len - 1`, so a
/// single local could not show that the cursor moved. The function names are the
/// fixture's own, because `with_at` wants `&'static str`; the assertions that
/// follow are what keep this in step with the fixture.
/// 两个而不是一个局部值：DATA 面板的游标停在 `data_len - 1`，只有一个局部值就演示不出
/// 游标移动。函数名取自夹具本身，因为 `with_at` 需要 `&'static str`；紧随其后的断言
/// 保证这里与夹具保持一致。
pub(super) fn fixture_live_trace(caller: &CallRef, callee: &CallRef) -> CallTrace {
    debug_assert_eq!(caller.function, "preview_canvas_width");
    debug_assert_eq!(callee.function, "clamp_canvas_width");
    let caller_source = SourceLocation {
        file: "<fixture-trace>",
        line: 1,
        column: 1,
        function: "preview_canvas_width",
    };
    let callee_source = SourceLocation {
        file: "<fixture-trace>",
        line: 2,
        column: 1,
        function: "clamp_canvas_width",
    };
    let mut trace = CallTrace::full();
    trace.with_at(
        caller.node,
        "preview_canvas_width",
        caller_source,
        |trace| {
            trace.local_at(
                "canvas_name",
                "&str",
                "node_editor",
                LocalKind::Input,
                caller_source,
            );
            trace.local_at("width", "u32", "320", LocalKind::Binding, caller_source);
            trace.with_at(callee.node, "clamp_canvas_width", callee_source, |_| {});
        },
    );
    trace
}
