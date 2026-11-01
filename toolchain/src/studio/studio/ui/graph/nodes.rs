//! Call-tree panel for the call-graph overlay.
//! 调用图浮层的调用树面板。
//!
//! With the `node-graph` feature (on by default) the tree is drawn by the
//! `rataflow` widget and this page owns the chrome around it: the border, the
//! heading that counts what the model holds, the legend that explains the
//! markers, and the status row that states the cursor's signature. The widget is
//! told what the kernel knows — the cut counts, the evidence kind of each edge,
//! whether the compiler was the only witness, and which node the cursor is on.
//! 在 `node-graph` 特性（默认开启）下，树由 `rataflow` 控件绘制，本页负责它周围的说明：
//! 边框、数出模型内容的小标题、解释标记的图例，以及写出游标签名的状态行。控件不知道的内核
//! 事实由本页告诉它——裁剪计数、每条边的证据种类、是否只有编译器作证，以及游标在哪个节点。
//!
//! Without the feature there is no drawer at all, so the panel keeps its border
//! and says so in one line rather than drawing a blank pane or panicking.
//! 没有该特性时完全没有绘制方，因此面板保留边框并用一行说明，而不是画一块空白面板或 panic。

use super::*;

/// The legend shown along the panel's bottom border: the markers the drawing uses
/// and cannot explain in place.
/// 沿面板底边显示的图例：图使用、且无法就地解释的标记。
const LEGEND: &str = " + live · ? candidate · ~ source · x external · ! unknown · ←n/→n not drawn ";

/// The one-line note the panel shows when Studio was built without the
/// `node-graph` feature.
/// Studio 未启用 `node-graph` 特性构建时，面板显示的一行说明。
#[cfg(not(feature = "node-graph"))]
const NEEDS_NODE_GRAPH: &str = "The call tree needs the `node-graph` feature.";

/// What the caller knows about this panel: which node the cursor is on, whether
/// the panel holds the focus, and what to call it.
/// 调用方对本面板知道的事：游标在哪个节点上、面板是否持有焦点，以及它叫什么。
pub(super) struct TreePanel {
    /// Index of the node under the cursor.
    /// 游标所在节点的下标。
    pub(super) cursor: usize,
    /// Whether this panel holds the keyboard focus.
    /// 本面板是否持有键盘焦点。
    pub(super) focused: bool,
    /// The panel's title.
    /// 面板标题。
    pub(super) title: &'static str,
}

// The render cache is consumed only by the `node-graph` backend; the surface must still compile
// without it, and a renamed parameter would be an unused binding there rather than a warning about
// the missing feature.
// 渲染缓存只被 `node-graph` 后端消费；没有那个特性时本面仍须编译，而改名参数会让那里多一个未使用的
// 绑定，而不是关于缺失特性的警告。
#[cfg_attr(not(feature = "node-graph"), allow(unused_variables))]
pub(super) fn draw_call_tree(
    frame: &mut Frame<'_>,
    area: Rect,
    app: &App,
    cache: &mut RenderCache,
    selected: Option<&CallRef>,
    tree_panel: TreePanel,
) {
    let TreePanel {
        cursor,
        focused,
        title,
    } = tree_panel;
    let Some(item) = selected else {
        frame.render_widget(
            Paragraph::new("Select a call node.").block(panel(title, MUTED)),
            area,
        );
        return;
    };
    let view = app.call_tree_view(item);
    if view.is_empty() {
        frame.render_widget(
            Paragraph::new("Select a call node.").block(panel(title, MUTED)),
            area,
        );
        return;
    }
    // The budget belongs in the panel title: the border row is otherwise empty,
    // and a count that shares a row with the ruler gets clipped by it.
    // 预算放在面板标题里：边框行本来是空的，而和标尺共享一行的计数会被标尺裁掉。
    let depth = view
        .tree
        .nodes
        .iter()
        .map(|node| node.level.unsigned_abs())
        .max()
        .unwrap_or(0);
    let cursor = cursor.min(view.len().saturating_sub(1));
    let inner = area.inner(ratatui::layout::Margin {
        vertical: 1,
        horizontal: 1,
    });
    let mut heading = format!("{title} · depth {depth} · {} nodes", view.len());
    if view.tree.truncated {
        heading.push_str(" · truncated");
    }
    frame.render_widget(
        panel(heading, if focused { GREEN } else { MAGENTA }).title_bottom(
            Line::from(Span::styled(LEGEND, Style::default().fg(MUTED)))
                .alignment(Alignment::Center),
        ),
        area,
    );
    #[cfg(not(feature = "node-graph"))]
    {
        let _ = cursor;
        // The border and the legend above are already drawn; this sentence is the
        // honest answer for the pane between them.
        // 上面的边框与图例已经画好；这句话就是两者之间那块内容的诚实答案。
        frame.render_widget(
            Paragraph::new(NEEDS_NODE_GRAPH).style(Style::default().fg(MUTED)),
            inner,
        );
    }
    #[cfg(feature = "node-graph")]
    {
        // MIR candidates are the closest thing Studio has to the prototype's
        // "undecided": the compiler said these functions call each other, and no
        // live observation confirmed it. They are marked on the node, never merged
        // into the edges the widget routes.
        // MIR 候选是 Studio 里最接近原型"未决"的东西：编译器说这些函数互相调用，而没有实时
        // 观测确认。它们标在节点上，绝不并入控件画出的那些边。
        let uncertain = view
            .refs
            .iter()
            .map(|item| {
                item.as_ref()
                    .is_some_and(|item| !app.mir_candidates_for(&item.function).is_empty())
            })
            .collect::<Vec<bool>>();
        // The widget keeps its own viewport, so it is rebuilt only when the tree
        // it draws changes — a moved cursor is a selection, not a new graph.
        // 控件保留自己的视口，因此只在它绘制的树变化时重建——游标移动是选择，不是新图。
        let key = flow_key(&view, app.source_stamp());
        match cache.graph_flow.as_ref() {
            // A different tree: build it and fit it into the panel.
            // 另一棵树：构建它并让它铺满面板。
            None => {
                cache.graph_flow =
                    node_graph::build(&view, cursor, &uncertain)
                        .ok()
                        .map(|mut flow| {
                            flow.request_fit_view();
                            (key, cursor, flow)
                        });
            }
            Some((cached, _, _)) if *cached != key => {
                cache.graph_flow =
                    node_graph::build(&view, cursor, &uncertain)
                        .ok()
                        .map(|mut flow| {
                            flow.request_fit_view();
                            (key, cursor, flow)
                        });
            }
            // The same tree with the cursor somewhere else: the mark lives in the
            // node's text, so it is re-stamped — and the viewport travels with it,
            // because pan and zoom are the reader's, not the mark's.
            // 同一棵树但游标移了位：标记住在节点文本里，因此重盖一次——而视口随行，因为平移与
            // 缩放属于读者，不属于标记。
            Some((_, marked, flow)) if *marked != cursor => {
                let viewport = flow.viewport;
                if let Ok(mut rebuilt) = node_graph::build(&view, cursor, &uncertain) {
                    rebuilt.viewport = viewport;
                    cache.graph_flow = Some((key, cursor, rebuilt));
                }
            }
            Some(_) => {}
        }
        if let Some((_, _, flow)) = cache.graph_flow.as_mut() {
            // The cursor is the widget's selection, and it is written into the
            // node's own title as well, so it is visible without colour.
            // 游标就是控件的选中项，同时也写进节点自己的标题，因此没有颜色也看得见。
            flow.select_node(&cursor.to_string());
            frame.render_widget(&mut *flow, inner);
            // The panel keeps the chrome that explains the drawing, which the
            // widget knows nothing about.
            // 面板保留解释这张图的说明行，那是控件不知道的东西。
            draw_status(frame, inner, app, &view, cursor);
        }
    }
}

/// What the cached widget was built from: the focus's symbol, the shape of the tree
/// around it, and the source stamp that tree came from. A re-centred focus changes it,
/// a moved cursor does not, and a rebuild of the same shape with different node text
/// or lanes no longer reuses the old drawing (audit `STU-S-10`).
/// 缓存的控件是从什么构建的：焦点的符号、它周围那棵树的形状，以及那棵树来自哪个源码戳。
/// 重新居中会改变它，游标移动不会；而形状相同、节点文本或车道不同的重建不再复用旧绘制
/// （审计 `STU-S-10`）。
#[cfg(feature = "node-graph")]
fn flow_key(view: &CallTreeView, source_stamp: u128) -> String {
    let focus = view
        .tree
        .nodes
        .first()
        .map(|node| node.symbol.as_str())
        .unwrap_or("");
    format!(
        "{focus}:{}:{}:{source_stamp}",
        view.tree.nodes.len(),
        view.tree.edges.len()
    )
}

/// Draw the status row: the cursor's signature and its first transform, plus the
/// gaps that belong to this node alone.
/// 绘制状态行：游标的签名与它的第一条 transform，以及只属于这个节点的缺口。
#[cfg(feature = "node-graph")]
fn draw_status(frame: &mut Frame<'_>, body: Rect, app: &App, view: &CallTreeView, cursor: usize) {
    let Some(item) = view.item(cursor) else {
        return;
    };
    let mut status = format!("ƒ {}", signature_of(&item));
    if let Some(transform) = app.call_tree_transforms(&item).first() {
        status.push_str(&format!("   ↳ {transform}"));
    }
    let node = &view.tree.nodes[cursor];
    if node.cut_callers > 0 {
        status.push_str(&format!("   ←{} caller(s) not drawn", node.cut_callers));
    }
    if node.cut_callees > 0 {
        status.push_str(&format!("   →{} callee(s) not drawn", node.cut_callees));
    }
    if incoming_off_axis(view, cursor) {
        status.push_str("   ◀ off-axis edge");
    }
    put_line(
        frame,
        body,
        body.x,
        body.bottom().saturating_sub(1),
        &status,
        Style::default().fg(MUTED),
    );
}

/// Whether a node has an incoming edge the drawn tree cannot show as a hop, which
/// is what the status row reports for it.
/// 某个节点是否有画出的树无法表示成一次跳转的入边，也就是状态行为它报告的情况。
#[cfg(feature = "node-graph")]
fn incoming_off_axis(view: &CallTreeView, index: usize) -> bool {
    view.tree
        .edges
        .iter()
        .any(|edge| edge.callee == index && !edge.forward)
}

/// Write one line of text into the frame, clipped to the panel's body.
/// 把一行文本写入帧缓冲，并裁剪到面板内部。
#[cfg(feature = "node-graph")]
fn put_line(frame: &mut Frame<'_>, area: Rect, x: u16, y: u16, text: &str, style: Style) {
    if y < area.y || y >= area.bottom() {
        return;
    }
    for (offset, character) in text.chars().enumerate() {
        let x = x.saturating_add(offset as u16);
        if x < area.x {
            continue;
        }
        if x >= area.right() {
            break;
        }
        if let Some(cell) = frame.buffer_mut().cell_mut((x, y)) {
            cell.set_char(character);
            cell.set_style(style);
        }
    }
}

/// The declared signature of one function, read from its source.
/// 某个函数的声明签名，从其源码读出。
#[cfg(feature = "node-graph")]
fn signature_of(item: &CallRef) -> String {
    let Ok(text) = std::fs::read_to_string(source_path_for(&item.file)) else {
        return item.function.clone();
    };
    text.lines()
        .find(|line| line.contains(&format!("{}(", item.function)))
        .map(|line| line.trim().trim_end_matches('{').trim().to_owned())
        .unwrap_or_else(|| item.function.clone())
}
