//! Search page entry: `draw_search`, with the pieces in the `search/` submodules.
//! 搜索页入口：`draw_search`，各部分在 `search/` 子模块里。
//!
//! Search result and source preview rendering.
//! 搜索结果与源码预览渲染。
//!
//! This root owns the `draw_search` layout and re-exports the pieces: the query
//! bar in `query`, the result list in `results`, and the selected symbol preview
//! and call lane in `detail`.
//! 本模块根承载 `draw_search` 布局并重导出各部分：查询栏在 `query`，结果列表在
//! `results`，选中符号预览与调用关系栏在 `detail`。

use super::*;

#[path = "search/detail.rs"]
mod detail;
use detail::{draw_search_lane, draw_search_preview};
#[path = "search/query.rs"]
mod query;
use query::draw_query_bar;
#[path = "search/results.rs"]
mod results;
use results::{SearchListOptions, draw_search_list};

pub(super) fn draw_search(
    frame: &mut Frame<'_>,
    area: Rect,
    app: &App,
    cache: &mut RenderCache,
    search: &SearchState,
) -> (Rect, usize) {
    let inner = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(5),
            Constraint::Length(2),
        ])
        .split(area.inner(ratatui::layout::Margin {
            vertical: 1,
            horizontal: 2,
        }));
    draw_query_bar(frame, inner[0], search);
    if search.graph_mode {
        draw_search_graph(frame, inner[1], app, cache, search);
        // Graph-mode footer. Every key below is matched in the `search.graph_mode`
        // branch of `handle_search_overlay_key`: Tab moves focus, ↑↓ move the
        // selection, Enter re-centres or opens, `m` loads the MIR snapshot,
        // `[`/`]` move the split, and Esc goes back.
        // 调用图页脚。下列按键都在 `handle_search_overlay_key` 的
        // `search.graph_mode` 分支匹配：Tab 切换焦点、↑↓ 移动选择、Enter 重新居中或打开、
        // `m` 载入 MIR 快照、`[`/`]` 移动分栏，Esc 返回。
        let footer = "↑↓ select   ←→ hop   Enter re-centre/open   e source   Tab pane   wheel zoom   [ ] split   m MIR   / search field   Esc back";
        frame.render_widget(
            Paragraph::new(footer)
                .alignment(Alignment::Center)
                .style(Style::default().fg(MUTED)),
            inner[2],
        );
        return (inner[1], search.offset);
    }
    let body = inner[1];
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(31),
            Constraint::Percentage(23),
            Constraint::Percentage(46),
        ])
        .split(body);
    draw_search_lane(frame, columns[1], app, search);
    let offset = draw_search_list(
        frame,
        columns[0],
        app,
        SearchListOptions {
            query: &search.query,
            selected: search.selected,
            offset: search.offset,
            active: true,
            title: " SEARCH ",
        },
    );
    draw_search_preview(frame, columns[2], app, &search.query, search.selected);
    // List-mode footer. Enter promotes the selected row into the call graph
    // (`graph_mode = true`), so it is not an edit binding. There is no `←→ fold`:
    // `search_rows` returns a flat list with no fold state, so those keys are
    // intentionally unbound.
    // 列表页脚。Enter 把选中行推进调用图（`graph_mode = true`），因此不是编辑键。这里没有
    // `←→ fold`：`search_rows` 返回无折叠状态的扁平列表，这两个键有意不绑定。
    frame.render_widget(
        Paragraph::new("type to filter   ↑↓ select   Enter call graph   Esc close")
            .alignment(Alignment::Center)
            .style(Style::default().fg(MUTED)),
        inner[2],
    );
    (columns[0], offset)
}
