//! The multilingual workspace: a read-only listing of the `xirang.toml` units.
//! 多语言工作区：只读列出 `xirang.toml` 的单元。

use super::*;

/// Draw the inventory the page loaded when it opened.
/// 绘制页面打开时装入的清单。
pub(super) fn draw_multilang(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let text = if app.multilang.is_empty() {
        "Press 4 to load the `xirang.toml` inventory of this project.".to_owned()
    } else {
        app.multilang.clone()
    };
    let paragraph = Paragraph::new(text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title(" MULTILANG "),
        )
        .wrap(Wrap { trim: false });
    frame.render_widget(paragraph, area);
}
