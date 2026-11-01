//! covers `app/trace.rs`, `app/graph_queries.rs`.
//! 覆盖 `app/trace.rs`、`app/graph_queries.rs`。
//!
//! What a Studio session's trace can confirm, observed instead of argued.
//! Studio 会话的追踪实际能确认什么：观察，而不是论证。

use super::*;

/// Every caller-to-callee edge the graph page could draw for the loaded project.
/// 已加载工程里调用图页面可能画出的每一条调用者→被调用者边。
///
/// The enumeration follows the page's own definition of an edge: for each
/// function the source scan sees, ask the app for its callers and callees. That
/// is the same `call_relations` the tree and the detail pane read, so a
/// classification over this list covers every edge they could render.
/// 枚举沿用页面自己对边的定义：对源码扫描看到的每个函数，向 app 问它的调用者与被调用者。
/// 这与树和详情面板读的是同一个 `call_relations`，因此对这份列表分级就覆盖了它们可能渲染的
/// 每一条边。
fn drawn_edges(app: &App) -> Vec<(CallRef, CallRef)> {
    let mut edges = Vec::new();
    for info in app.registry.depth_first() {
        let Ok(text) = std::fs::read_to_string(source_path_for(&info.source.file)) else {
            continue;
        };
        for symbol in function_symbols(&text) {
            let center = CallRef {
                node: info.id,
                function: symbol.name.clone(),
                file: info.source.file.clone(),
            };
            let (callers, callees) = app.call_relations(info.id, &symbol.name);
            for caller in callers {
                edges.push((caller, center.clone()));
            }
            for callee in callees {
                edges.push((center.clone(), callee));
            }
        }
    }
    edges
}

/// The drawn edges that a live observation confirmed, as function-name pairs.
/// 被实时观察确认的已画出边，以函数名对表示。
///
/// `drawn_edges` reaches every edge from both endpoints — once as the caller's
/// callee and once as the callee's caller — so the set below is what collapses
/// those two sightings into the one relationship they describe.
/// `drawn_edges` 会从两个端点各到达每条边一次——一次作为调用方的被调用者，一次作为被调用方
/// 的调用者——下面这个集合正是把这两次目击收敛为它们所描述的那一条关系。
fn confirmed_edges(app: &App) -> Vec<(String, String)> {
    let mut confirmed = std::collections::BTreeSet::new();
    for (caller, callee) in drawn_edges(app) {
        if app.call_evidence(&caller, &callee) == CallEvidence::Live {
            confirmed.insert((caller.function, callee.function));
        }
    }
    confirmed.into_iter().collect()
}

/// `U5` in `docs/audit-production-readiness.md` recorded an argument — the
/// shipped sample's nodes cannot come from the same provenance as a real
/// project's, so `CallEvidence::Live` was said to be unreachable — and never an
/// observation. The sample is gone now, and the same question is asked of the
/// state every session starts in: no artifact, no trace, no confirmation. The
/// positive half — that `Live` *is* reachable once a trace over the loaded faces
/// is installed — is the next test, so this one cannot pass by `Live` being
/// impossible.
/// `docs/audit-production-readiness.md` 里的 `U5` 记下的是一条论证——出厂样本的节点不可能
/// 与真实工程的节点同源，因此称 `CallEvidence::Live` 不可达——而从来不是一次观察。样本现在
/// 已删除，同一个问题改为问每个会话启动时的状态：没有 artifact、没有追踪、没有确认。正面的一半
/// ——一旦装上针对已加载注册面的追踪，`Live` **确实**可达——是下一条测试，因此本条不会因
/// `Live` 根本不可能而通过。
#[test]
fn a_session_without_an_artifact_installs_no_trace_and_confirms_nothing() {
    let Some(app) = fixture_app() else {
        return;
    };

    // The premise, asserted rather than assumed: the session holds no trace
    // because it found no artifact, not because a trace was hidden.
    // 前提是被断言的而不是被假设的：会话不持有追踪，是因为它没找到 artifact，而不是因为追踪被藏起来。
    assert_eq!(app.trace_status, TraceStatus::Absent, "{}", app.event);
    assert!(
        app.runtime_trace.call_edges().is_empty(),
        "a session with no artifact must not carry call edges: {:?}",
        app.runtime_trace.call_edges()
    );

    let drawn = drawn_edges(&app);
    assert!(
        !drawn.is_empty(),
        "the fixture project has a call graph, so this cannot pass vacuously"
    );
    assert_eq!(
        confirmed_edges(&app),
        Vec::<(String, String)>::new(),
        "a session with no trace confirmed an edge anyway"
    );
}

/// The other half of the observation: the same enumeration does report `Live`
/// once a trace over the loaded faces is installed. Without this, the test above
/// would also pass if `call_evidence` could never answer `Live` at all — which is
/// exactly the confusion `U5` was about.
/// 观察的另一半：一旦装上针对已加载注册面的追踪，同一份枚举就会报出 `Live`。没有这一半，
/// 上面那条测试在 `call_evidence` 根本答不出 `Live` 时也会通过——而 `U5` 问的正是这个区别。
#[test]
fn a_trace_over_the_loaded_faces_confirms_the_edge_it_observed() {
    let Some(mut app) = fixture_app() else {
        return;
    };
    let node_editor = app
        .registry
        .depth_first()
        .into_iter()
        .find(|info| {
            info.registry_name == "node_editor" && info.source.file.ends_with("node_editor.rs")
        })
        .expect("the fixture declares the NodeEditor face");
    let (_, callees) = app.call_relations(node_editor.id, "preview_canvas_width");
    let canvas = callees
        .iter()
        .find(|item| item.function == "clamp_canvas_width")
        .expect("the fixture's source scan finds this call")
        .clone();
    let center = CallRef {
        node: node_editor.id,
        function: "preview_canvas_width".to_owned(),
        file: node_editor.source.file.clone(),
    };
    app.runtime_trace = super::fixtures::fixture_live_trace(&center, &canvas);
    assert_eq!(
        confirmed_edges(&app),
        vec![(
            "preview_canvas_width".to_owned(),
            "clamp_canvas_width".to_owned()
        )],
        "a trace over the loaded faces must confirm exactly the edge it recorded"
    );
}
