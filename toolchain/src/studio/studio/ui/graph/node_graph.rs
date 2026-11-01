//! The call tree drawn by the `rataflow` node-editor widget.
//! 由 `rataflow` 节点编辑器控件绘制的调用树。
//!
//! This is the only drawer. What the widget does not know, this page tells it:
//! the kernel's own facts go into the node text and the edge labels — the cut
//! counts (`←n`/`→n`, what the budgets left out), the evidence kind of each edge,
//! whether the compiler was the only witness (`?`), and which node the cursor is
//! on, which is written into the title as well as the border style so it survives
//! a terminal with no colour.
//! 这是唯一的绘制方。控件不知道的事情由本页告诉它：内核自身的事实进入节点文本与边标签——
//! 裁剪计数（`←n`/`→n`，预算漏掉了谁）、每条边的证据种类、是否只有编译器作证（`?`），以及
//! 游标在哪个节点上（除了边框样式，还写进标题，因此在没有颜色的终端里也读得出来）。

use super::*;

use std::collections::BTreeMap;

use rataflow::{Edge, Flow, HandlePosition, Node, StepEdge, TextContent};
use ratatui::text::Text;

/// World cells left between two lanes' widest boxes, so a long name in one lane
/// cannot touch its neighbour.
/// 两条车道最宽盒子之间留下的世界格数，使一条车道里的长名字碰不到邻列。
const LANE_GAP: f64 = 5.0;
/// World rows left between two bands, so the edges that leave one band and enter
/// the next have room to route.
/// 两条带之间留下的世界行数，使离开一条带、进入下一条带的边有走线余地。
const BAND_GAP: f64 = 4.0;
/// World rows the smallest band is padded to, so a band of one-line boxes still
/// leaves the routing gap.
/// 最小带被补齐到的世界行数，使只放一行盒子的带也留出走线空隙。
const MIN_BAND: f64 = 3.0;

/// Build the widget's graph from one view of the kernel's tree.
/// 由内核树的某个视图构建控件的图。
pub(super) fn build(
    tree: &CallTreeView,
    cursor: usize,
    uncertain: &[bool],
) -> Result<Flow, rataflow::Error> {
    // One piece per node, measured before anything is placed: a lane is as wide
    // as its widest box and a band as tall as its tallest one, which is what keeps
    // boxes apart. Fixed pitches do not: the widths come from the names, so a
    // 27-character symbol overlapped its neighbour at every pitch small enough to
    // fit the short ones.
    // 每个节点先量后放：车道取其中最宽盒子的宽度，带取其中最高盒子的高度，这正是盒子互不
    // 相碰的原因。固定间距做不到：宽度来自名字，因此任何"刚好塞得下短名字"的间距都会让一个
    // 27 字符的符号压到邻列上。
    let mut pieces = tree
        .tree
        .nodes
        .iter()
        .enumerate()
        .map(|(index, node)| {
            let (lines, title) = node_text(node, index, cursor, uncertain);
            let width = lines
                .iter()
                .map(|line| line.width())
                .max()
                .unwrap_or(1)
                .saturating_add(2) as f64;
            let height = (lines.len() + 2) as f64;
            let mut content = TextContent::new(Text::from(lines));
            content.title = Some(title);
            content.border_style = Some(Style::default().fg(if index == cursor {
                CYAN
            } else if node.level == 0 {
                GREEN
            } else {
                MUTED
            }));
            content.selected_border_style =
                Some(Style::default().fg(CYAN).add_modifier(Modifier::BOLD));
            content.selected_text_style =
                Some(Style::default().fg(INK).add_modifier(Modifier::BOLD));
            (index, node.level, node.lane, content, width, height)
        })
        .collect::<Vec<_>>();
    // Lane widths and band heights, then the offsets that place them.
    // 车道宽度与带高，然后是安放它们的偏移。
    let mut lane_width: BTreeMap<usize, f64> = BTreeMap::new();
    let mut band_height: BTreeMap<i32, f64> = BTreeMap::new();
    for (_, level, lane, _, width, height) in &pieces {
        let lane = lane_width.entry(*lane).or_insert(0.0);
        *lane = lane.max(*width);
        let band = band_height.entry(*level).or_insert(MIN_BAND);
        *band = band.max(*height);
    }
    let lane_x = offsets(lane_width.values().copied(), LANE_GAP);
    let band_y = offsets(band_height.values().copied(), BAND_GAP);
    let lane_keys = lane_width.keys().copied().collect::<Vec<_>>();
    let band_keys = band_height.keys().copied().collect::<Vec<_>>();
    let nodes = pieces
        .drain(..)
        .map(|(index, level, lane, content, width, height)| {
            let column = lane_keys.iter().position(|key| *key == lane).unwrap_or(0);
            let band = band_keys.iter().position(|key| *key == level).unwrap_or(0);
            // Centred in its own cell of the grid, so a row of boxes reads as a row.
            // 在网格自己的格子里居中，使一排盒子读起来就是一排。
            let position = (
                lane_x[column] + (lane_width[&lane] - width) / 2.0,
                band_y[band] + (band_height[&level] - height) / 2.0,
            );
            Node::new(index.to_string(), position, (width, height), content)
                // The tree is a view of the kernel's model: a node the reader
                // drags must not drift away from the level and lane it stands
                // for, so dragging is off and the viewport is what moves.
                // 这棵树是内核模型的视图：读者拖动的节点不能偏离它所代表的层与车道，因此关掉
                // 节点拖动，移动的是视口。
                .with_draggable(false)
                // The call direction is downwards, so an edge leaves the bottom
                // of its caller and enters the top of its callee. Side ports made
                // every hop look like a sideways shuffle; top and bottom make the
                // picture the tree it is.
                // 调用方向向下，因此边从调用者的底边离开、进入被调用者的顶边。侧边端口让每一跳
                // 都像横着挪一步；上下端口才让这张图成为它本来的样子——一棵树。
                .with_source_position(HandlePosition::Bottom)
                .with_target_position(HandlePosition::Top)
        })
        .collect::<Vec<_>>();
    let edges = tree
        .tree
        .edges
        .iter()
        .enumerate()
        .filter(|(_, edge)| edge.forward)
        .map(|(index, edge)| {
            let mut built = Edge::<StepEdge>::new(
                format!("e{index}"),
                edge.caller.to_string(),
                edge.callee.to_string(),
            );
            // The evidence kind is a fact about the call, not about the drawing,
            // so it travels as the edge's label.
            // 证据种类是关于这次调用的事实，而不是关于绘制的事实，因此作为边的标签随行。
            built.label = Some(edge.evidence.marker().to_owned());
            built
        })
        .collect::<Vec<_>>();
    Flow::with_graph(nodes, edges)
}

/// The lines and the border title one node shows: the marks the reader navigates
/// by, the symbol, and the cuts the budgets made.
/// 一个节点展示的行与边框标题：读者据以导航的标记、符号，以及预算造成的裁剪。
fn node_text(
    node: &CallTreeNode,
    index: usize,
    cursor: usize,
    uncertain: &[bool],
) -> (Vec<Line<'static>>, String) {
    // The cursor and the focus are facts worth a mark, and a mark is what
    // survives a terminal with no colour.
    // 游标与焦点值得一个标记，而标记正是"没有颜色的终端"也吃不掉的东西。
    let mark = if index == cursor {
        "▶ "
    } else if node.level == 0 {
        "◆ "
    } else {
        ""
    };
    let badges = badge_text(node, uncertain.get(index).copied().unwrap_or(false));
    let mut lines = vec![Line::from(format!("{mark}{}", node.symbol))];
    if !badges.is_empty() {
        lines.push(Line::from(badges));
    }
    // The title repeats the level the border colour already says, so the same
    // fact reaches a monochrome terminal.
    // 标题重复了边框颜色已经说过的层号，使同一事实也能到达单色终端。
    let title = if node.level == 0 {
        "focus".to_owned()
    } else {
        format!("{:+}", node.level)
    };
    (lines, title)
}

/// The start of each cell of a grid whose cells are as big as their content plus
/// `gap`.
/// 一个网格每格的起点，格子大小等于内容加 `gap`。
fn offsets(sizes: impl Iterator<Item = f64>, gap: f64) -> Vec<f64> {
    let mut offsets = Vec::new();
    let mut cursor = 0.0;
    for size in sizes {
        offsets.push(cursor);
        cursor += size + gap;
    }
    offsets
}

/// The cut counts and the compiler-only mark one node shows.
/// 一个节点展示的裁剪计数与"仅编译器作证"标记。
fn badge_text(node: &CallTreeNode, uncertain: bool) -> String {
    let mut badges = String::new();
    if node.cut_callers > 0 {
        badges.push_str(&format!("←{} ", node.cut_callers));
    }
    if node.cut_callees > 0 {
        badges.push_str(&format!("→{} ", node.cut_callees));
    }
    if uncertain {
        badges.push('?');
    }
    badges.trim_end().to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A tree with a long name beside a short one: the shape that used to overlap.
    /// 一棵长名字挨着短名字的树：过去会重叠的形状。
    fn tree() -> CallTreeView {
        let mut view = CallTreeView::default();
        view.tree.nodes = vec![
            CallTreeNode {
                symbol: "paint_node_editor".to_owned(),
                level: 0,
                lane: 0,
                parent: None,
                cut_callers: 0,
                cut_callees: 0,
            },
            CallTreeNode {
                symbol: "preview_canvas_width_traced".to_owned(),
                level: 1,
                lane: 1,
                parent: Some(0),
                cut_callers: 0,
                cut_callees: 0,
            },
            CallTreeNode {
                symbol: "place_panel".to_owned(),
                level: 1,
                lane: 2,
                parent: Some(0),
                cut_callers: 2,
                cut_callees: 0,
            },
        ];
        view.refs = vec![None; view.tree.nodes.len()];
        view
    }

    /// Why the pitch is measured rather than fixed is `build`'s comment above; the
    /// assertion below is what holds it.
    /// 为什么间距要实测而不是写死见上方 `build` 的注释；下面那条断言是它的守卫。
    #[test]
    fn every_box_gets_its_own_room() {
        let view = tree();
        let flow = build(&view, 0, &[false, false, false]).expect("the graph is valid");
        let bounds = (0..view.tree.nodes.len())
            .map(|index| {
                flow.node_bounds(&index.to_string())
                    .unwrap_or_else(|| panic!("node {index} has bounds"))
            })
            .collect::<Vec<_>>();
        for (index, one) in bounds.iter().enumerate() {
            for (other, two) in bounds.iter().enumerate().skip(index + 1) {
                let overlaps = one.position.x < two.position.x + two.dimensions.width
                    && two.position.x < one.position.x + one.dimensions.width
                    && one.position.y < two.position.y + two.dimensions.height
                    && two.position.y < one.position.y + one.dimensions.height;
                assert!(
                    !overlaps,
                    "nodes {index} ({:?}) and {other} ({:?}) share a cell",
                    one, two
                );
            }
        }
        // And the gap the layout promises is really there, not just non-overlap.
        // 而且排版承诺的空隙确实在，而不只是"没重叠"。
        assert!(
            bounds[1].position.x - (bounds[0].position.x + bounds[0].dimensions.width) >= 1.0,
            "{bounds:?}"
        );
    }

    /// Every node carries the cuts the budgets made, and a mark for the cursor.
    /// 每个节点都带上预算造成的裁剪，以及游标的标记。
    #[test]
    fn node_text_carries_the_kernels_facts() {
        let view = tree();
        let uncertain = [false, false, true];
        let lines = |index: usize, cursor: usize| {
            node_text(&view.tree.nodes[index], index, cursor, &uncertain)
                .0
                .iter()
                .map(|line| line.to_string())
                .collect::<Vec<_>>()
                .join("\n")
        };
        assert!(
            lines(0, 0).starts_with("▶ paint_node_editor"),
            "{}",
            lines(0, 0)
        );
        assert!(
            lines(2, 0).contains("←2"),
            "the cut count travels: {}",
            lines(2, 0)
        );
        assert!(lines(2, 0).contains('?'), "the compiler-only mark travels");
        assert!(
            lines(0, 2).starts_with("◆ paint_node_editor"),
            "the focus keeps a mark of its own: {}",
            lines(0, 2)
        );
    }
}
