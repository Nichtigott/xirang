//! New-project form rendering.
//! 新建项目表单渲染。

use super::super::*;
use super::draw_form_frame;

use crate::studio::studio::app::NewProjectState;

pub(crate) fn draw_new_project(
    frame: &mut Frame<'_>,
    area: Rect,
    project: &NewProjectState,
) -> (Rect, Rect, Rect, Rect) {
    let inner = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(7), Constraint::Length(4)])
        .split(area);
    let rows = new_project_field::LABELS
        .iter()
        .enumerate()
        .map(|(index, name)| {
            let selected = index == project.field;
            let marker = if selected && project.editing {
                "●"
            } else if selected {
                "›"
            } else {
                " "
            };
            let value = if project.values[index].is_empty() {
                "_"
            } else {
                &project.values[index]
            };
            ListItem::new(format!("{marker} {name:<12} {value}")).style(if selected {
                Style::default().fg(Color::Black).bg(CYAN)
            } else {
                Style::default().fg(INK)
            })
        })
        .collect::<Vec<_>>();
    let mut state = ListState::default().with_selected(Some(project.field));
    frame.render_stateful_widget(
        List::new(rows).block(panel(" NEW NICH LINK PROJECT ", GREEN)),
        inner[0],
        &mut state,
    );
    let areas = draw_form_frame(
        frame,
        inner[1],
        inner[0],
        "Create [s]",
        "Cancel [Esc]",
        "↑↓ field   Enter edit/toggle",
    );
    (areas.list, areas.cancel, areas.confirm, areas.exit)
}
