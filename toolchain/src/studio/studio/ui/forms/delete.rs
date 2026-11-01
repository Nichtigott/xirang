//! Delete-confirmation form rendering.
//! 删除确认表单渲染。

use super::super::*;

pub(crate) fn draw_delete(
    frame: &mut Frame<'_>,
    area: Rect,
    app: &App,
    id: crate::run_method::NodeId,
) -> (Rect, Rect) {
    let path = app
        .registry
        .path_for(id)
        .unwrap_or_else(|| "<unknown>".to_owned());
    let text = vec![
        Line::from(Span::styled(
            "Move this generated module to recoverable trash?",
            Style::default().fg(INK),
        )),
        Line::from(""),
        field("path", &path),
        field("node", &id.to_string()),
        Line::from(""),
    ];
    frame.render_widget(
        Paragraph::new(text)
            .alignment(Alignment::Center)
            .block(panel(" DELETE REGISTRATION FACE ", Color::LightRed)),
        area,
    );
    let buttons = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(30),
            Constraint::Length(16),
            Constraint::Length(3),
            Constraint::Length(16),
            Constraint::Percentage(30),
        ])
        .split(Rect::new(
            area.x,
            area.bottom().saturating_sub(4),
            area.width,
            3,
        ));
    frame.render_widget(
        Paragraph::new("Cancel")
            .alignment(Alignment::Center)
            .block(panel(" n / Esc ", MUTED)),
        buttons[1],
    );
    frame.render_widget(
        Paragraph::new("Delete")
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::LightRed))
            .block(panel(" Enter / y ", Color::LightRed)),
        buttons[3],
    );
    (buttons[1], buttons[3])
}
