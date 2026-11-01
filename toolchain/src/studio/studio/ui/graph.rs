//! Call tree and data-flow rendering for Studio.
//! Studio 调用树与数据流渲染。
//!
//! The page is two panes: the call tree of the focused function, and the values
//! that the tree cursor's function was observed with. The relation column that
//! listed callers and callees beside the tree is gone — the tree draws those as
//! edges — and so is the side-by-side comparison, whose second side halved every
//! pane the reader actually wanted.
//! 本页是两块面板：焦点函数的调用树，以及树游标所在函数被观测到的取值。曾经与树并排的
//! "调用者/被调用者"关系栏已删除——树把那些画成了边——左右对比也一并删除：它的第二侧让每块
//! 面板都只剩读者真正想要的一半。
//!
//! The module root owns the overlay entry point and the two-pane layout; the tree's
//! panel and the live-value panel live in the mounted submodules.
//! 模块根承载浮层入口与两栏布局；树面板与实时取值面板位于挂载的子模块。

use super::*;

#[path = "graph/data.rs"]
mod data;
#[cfg(feature = "node-graph")]
#[path = "graph/node_graph.rs"]
mod node_graph;
#[path = "graph/nodes.rs"]
mod nodes;

use data::draw_data_flow_panel;
use nodes::{TreePanel, draw_call_tree};

pub(super) fn draw_search_graph(
    frame: &mut Frame<'_>,
    area: Rect,
    app: &App,
    cache: &mut RenderCache,
    search: &SearchState,
) {
    let Some(center) = search_center_ref(app, search) else {
        frame.render_widget(
            Paragraph::new("No centre selected.").block(panel(" PROVENANCE ", MUTED)),
            area,
        );
        return;
    };
    cache.hot.graph_area = area;
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(app.graph_split_percent),
            Constraint::Percentage(100_u16.saturating_sub(app.graph_split_percent)),
        ])
        .split(area);
    cache.hot.graph_tree_area = columns[0];
    cache.hot.graph_data_area = columns[1];
    // The detail pane is the data pane: what the face inspector and the search
    // lane point at is the values the cursor's function ran with.
    // 详情面板就是数据面板：注册面检视器与搜索栏指向的，是游标所在函数运行时的取值。
    cache.hot.graph_detail_area = columns[1];
    cache.hot.graph_provenance_area = columns[1];
    draw_call_tree(
        frame,
        columns[0],
        app,
        cache,
        Some(&center),
        TreePanel {
            cursor: search.outline_selected,
            focused: search.graph_focus == 0,
            title: "CALL TREE",
        },
    );
    let item = app.graph_tree_item(search, search.outline_selected);
    draw_data_flow_panel(
        frame,
        columns[1],
        app,
        item.as_ref(),
        search.data_selected,
        search.graph_focus == 1,
        "DATA FLOW",
    );
}

/// Resolve the call reference the page is focused on.
/// 解析本页当前聚焦的调用引用。
///
/// The centre is either the node the reader opened, or — before a node is opened —
/// the row the search list has selected, so the page works from the moment the
/// reader presses Enter on a search row.
/// 圆心要么是读者打开的节点，要么——在打开节点之前——是搜索列表选中的那一行，因此读者在搜索行上
/// 按下 Enter 的那一刻本页就可用。
fn search_center_ref(app: &App, search: &SearchState) -> Option<CallRef> {
    if let Some(node) = search.center {
        return app.registry.find_registry(node).map(|info| CallRef {
            node,
            function: search
                .center_function
                .as_deref()
                .filter(|name| !name.is_empty())
                .unwrap_or(&info.source.function)
                .to_owned(),
            file: info.source.file.to_owned(),
        });
    }
    let rows = app.search_rows(&search.query);
    let row = rows.get(search.selected)?;
    let node = row.node?;
    let info = app.registry.find_registry(node)?;
    Some(CallRef {
        node,
        function: if row.function.is_empty() {
            info.source.function.to_owned()
        } else {
            row.function.clone()
        },
        file: info.source.file.to_owned(),
    })
}
