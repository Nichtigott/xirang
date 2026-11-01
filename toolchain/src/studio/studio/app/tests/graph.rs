//! covers `app/graph_queries.rs`, `app/call_tree_queries.rs`, `app/pointer.rs`.
//! 覆盖 `app/graph_queries.rs`、`app/call_tree_queries.rs`、`app/pointer.rs`。
//!
//! Call-graph navigation and panel focus tests.
//! 调用图导航与面板焦点测试。

// Every test here needs the fixture project, so the prelude is imported with the
// feature that mounts them: without it this file has no tests and the import
// would be unused.
// 这里的每个测试都需要夹具项目，因此前导与挂载它们的特性一起导入：没有该特性时本文件没有
// 测试，那个导入就成了未使用。
#[cfg(feature = "prototype-fixtures")]
use super::*;

#[test]
#[cfg(feature = "prototype-fixtures")]
fn graph_navigation_moves_across_real_call_edges() {
    let Some(mut app) = fixture_app() else {
        return;
    };
    app.overlay = Some(Overlay::Search(SearchState {
        query: "preview_canvas_width".to_owned(),
        ..SearchState::default()
    }));
    app.handle_overlay_key(crossterm::event::KeyEvent::from(
        crossterm::event::KeyCode::Enter,
    ));
    let (before, view) = {
        let Some(Overlay::Search(search)) = app.overlay.as_ref() else {
            panic!("search graph should open");
        };
        assert!(search.graph_mode);
        assert_eq!(search.graph_focus, 0, "the tree holds the focus on entry");
        let center = app
            .graph_item(search)
            .expect("the cursor stands on a function");
        (search.outline_selected, app.call_tree_view(&center))
    };
    // The one drawer runs the calls down the screen, so ↓ is the key that means
    // "toward the callees"; it must reach a callee.
    // 唯一的绘制方把调用沿屏幕向下排，因此表示"走向被调用者"的键是 ↓；它必须到达被调用者。
    app.handle_overlay_key(crossterm::event::KeyEvent::from(
        crossterm::event::KeyCode::Down,
    ));
    let Some(Overlay::Search(search)) = app.overlay.as_ref() else {
        panic!("search graph should remain open");
    };
    let landed = search.outline_selected;
    assert_ne!(landed, before, "the arrow moves the cursor");
    assert!(
        view.tree.nodes[landed].level > 0,
        "down reaches a callee: {}",
        view.tree.nodes[landed].symbol
    );
}

#[test]
#[cfg(feature = "prototype-fixtures")]
fn graph_tab_reaches_both_tree_and_data_panels() {
    let Some(mut app) = fixture_app() else {
        return;
    };
    // The DATA panel lists the trace's locals for the selected tree row, and tree
    // cursor 0 is the center function itself, so this test installs a trace with
    // locals inside `preview_canvas_width`. The standalone demo sample has no
    // locals for any host project, which is why this panel used to be unreachable.
    // DATA 面板列出选中树行对应的追踪局部值，而树游标 0 就是中心函数本身，因此本条测试
    // 安装一条在 `preview_canvas_width` 帧内带局部值的追踪。独立演示样本对任何宿主项目
    // 都没有局部值，这正是该面板此前不可达的原因。
    let node_editor = app
        .registry
        .depth_first()
        .into_iter()
        .find(|info| {
            info.registry_name == "node_editor" && info.source.file.ends_with("node_editor.rs")
        })
        .expect("NodeEditor face");
    let center = CallRef {
        node: node_editor.id,
        function: "preview_canvas_width".to_owned(),
        file: node_editor.source.file.to_owned(),
    };
    let callee = app
        .call_relations(center.node, &center.function)
        .1
        .into_iter()
        .find(|item| item.function == "clamp_canvas_width")
        .expect("the fixture's preview_canvas_width calls clamp_canvas_width");
    app.runtime_trace = super::fixtures::fixture_live_trace(&center, &callee);
    app.overlay = Some(Overlay::Search(SearchState {
        query: "preview_canvas_width".to_owned(),
        ..SearchState::default()
    }));
    app.handle_overlay_key(crossterm::event::KeyEvent::from(KeyCode::Enter));
    // Two panes: Tab moves the tree → the values, and wraps.
    for expected in [1, 0] {
        app.handle_overlay_key(crossterm::event::KeyEvent::from(KeyCode::Tab));
        let Some(Overlay::Search(search)) = app.overlay.as_ref() else {
            panic!("graph overlay should stay open");
        };
        assert_eq!(search.graph_focus, expected);
        assert_eq!(search.outline_focus, expected == 0);
    }

    // The value pane is where the data cursor moves; the tree's cursor is
    // untouched while it has the focus.
    app.handle_overlay_key(crossterm::event::KeyEvent::from(KeyCode::Tab));
    app.handle_overlay_key(crossterm::event::KeyEvent::from(KeyCode::Down));
    let Some(Overlay::Search(search)) = app.overlay.as_ref() else {
        panic!("graph overlay should stay open");
    };
    assert_eq!(search.graph_focus, 1);
    assert_eq!(search.data_selected, 1);
    assert_eq!(search.outline_selected, 0);
}

#[test]
#[cfg(feature = "prototype-fixtures")]
fn graph_enter_promotes_callers_and_opens_center_source() {
    let Some(mut app) = fixture_app() else {
        return;
    };
    app.overlay = Some(Overlay::Search(SearchState {
        query: "preview_canvas_width".to_owned(),
        ..SearchState::default()
    }));
    app.handle_overlay_key(crossterm::event::KeyEvent::from(
        crossterm::event::KeyCode::Enter,
    ));

    // ↓ hops downstream to a callee — the one drawer runs the calls down the
    // screen — and Enter makes that callee the centre.
    // ↓ 跳向下游的被调用者——唯一的绘制方把调用沿屏幕向下排——Enter 让那个被调用者成为新的圆心。
    app.handle_overlay_key(crossterm::event::KeyEvent::from(
        crossterm::event::KeyCode::Down,
    ));
    app.handle_overlay_key(crossterm::event::KeyEvent::from(
        crossterm::event::KeyCode::Enter,
    ));
    let Some(Overlay::Search(search)) = app.overlay.as_ref() else {
        panic!("graph should stay open after promoting a callee");
    };
    assert_eq!(
        search.center_function.as_deref(),
        Some("clamp_canvas_width")
    );

    app.handle_overlay_key(crossterm::event::KeyEvent::from(
        crossterm::event::KeyCode::Enter,
    ));
    assert!(app.overlay.is_none());
    let Some((path, line)) = app.take_editor_request() else {
        panic!("center Enter should request an editor");
    };
    assert!(path.is_file());
    assert!(path.ends_with("control/object/node_editor/object/object.rs"));
    assert!(line > 1);
}

#[test]
#[cfg(feature = "prototype-fixtures")]
fn call_tree_targets_keep_rows_navigable() {
    let Some(app) = fixture_app() else {
        return;
    };
    let node_editor = app
        .registry
        .depth_first()
        .into_iter()
        .find(|info| {
            info.registry_name == "node_editor" && info.source.file.ends_with("node_editor.rs")
        })
        .expect("NodeEditor face");
    let center = CallRef {
        node: node_editor.id,
        function: "preview_canvas_width".to_owned(),
        file: node_editor.source.file.to_owned(),
    };
    let targets = app.call_tree_targets(&center);
    assert_eq!(
        targets
            .first()
            .and_then(Option::as_ref)
            .map(|item| item.function.as_str()),
        Some("preview_canvas_width")
    );
    assert!(
        targets
            .iter()
            .flatten()
            .any(|item| item.function == "clamp_canvas_width")
    );
}
