//! Plugin selection form rendering.
//! 插件选择表单渲染。

use super::super::*;
use super::draw_form_frame;

use crate::studio::studio::app::PluginState;

pub(crate) fn draw_plugin(
    frame: &mut Frame<'_>,
    area: Rect,
    plugin: &PluginState,
) -> (Rect, Rect, Rect, Rect) {
    let inner = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(12), Constraint::Length(4)])
        .split(area);
    let rows = plugin_field::LABELS
        .iter()
        .enumerate()
        .map(|(index, name)| {
            let selected = index == plugin.field;
            let marker = if selected && plugin.editing {
                "●"
            } else if selected {
                "›"
            } else {
                " "
            };
            let value = if plugin.values[index].is_empty() {
                "_"
            } else {
                &plugin.values[index]
            };
            let style = if selected {
                Style::default().fg(Color::Black).bg(CYAN)
            } else {
                Style::default().fg(INK)
            };
            ListItem::new(format!("{marker} {name:<12} {value}")).style(style)
        })
        .collect::<Vec<_>>();
    frame.render_widget(
        List::new(rows).block(panel(" SELECT PLUGIN ", GREEN)),
        inner[0],
    );
    let areas = draw_form_frame(
        frame,
        inner[1],
        inner[0],
        "Save [s]",
        "Cancel [Esc]",
        "↑↓ field   Enter edit/toggle",
    );
    (areas.list, areas.cancel, areas.confirm, areas.exit)
}
