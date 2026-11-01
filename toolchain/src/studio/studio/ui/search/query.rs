//! Search query bar rendering.
//! 搜索查询栏渲染。

use super::super::*;

/// One query field, always visible: the page searched for here is the page the
/// tree and the values are drawn around.
/// 一个搜索框，始终可见：这里搜索到的页面就是树与取值围绕的那个页面。
pub(super) fn draw_query_bar(frame: &mut Frame<'_>, area: Rect, search: &SearchState) {
    frame.render_widget(
        Paragraph::new(format!("/ {}", search.query))
            .style(Style::default().fg(INK).bg(Color::Rgb(25, 34, 40)))
            .block(panel(" SEARCH ", CYAN)),
        area,
    );
}
