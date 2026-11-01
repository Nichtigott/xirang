//! Registration-face add/edit form rendering.
//! 注册面新增/编辑表单渲染。

use super::super::*;
use super::draw_form_frame;

use super::face_fields::{self, draw_face_field_help, face_field_presentation, face_field_value};

pub(crate) fn draw_add(
    frame: &mut Frame<'_>,
    area: Rect,
    add: &AddState,
) -> (Rect, Rect, Rect, Rect) {
    draw_face_form(frame, area, add, " ADD REGISTRATION FACE ")
}

pub(crate) fn draw_edit(
    frame: &mut Frame<'_>,
    area: Rect,
    edit: &AddState,
) -> (Rect, Rect, Rect, Rect) {
    draw_face_form(frame, area, edit, " EDIT REGISTRATION FACE ")
}

fn draw_face_form(
    frame: &mut Frame<'_>,
    area: Rect,
    add: &AddState,
    title: &'static str,
) -> (Rect, Rect, Rect, Rect) {
    let inner = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(9), Constraint::Length(4)])
        .split(area);
    let body = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(58), Constraint::Percentage(42)])
        .split(inner[0]);
    let fields = face_field_indices();
    let title = format!("{title}  lang: {} (l) ", add.language);
    let name_width = fields
        .iter()
        .map(|index| face_field_presentation(*index).label.len())
        .max()
        .unwrap_or(1);
    let rows = fields
        .iter()
        .map(|index| {
            let selected = *index == add.field;
            let carried = add.is_carried(*index);
            let marker = if selected && add.editing {
                "●"
            } else if selected {
                "›"
            } else {
                " "
            };
            let presentation = face_field_presentation(*index);
            let value = face_field_value(add, *index);
            // The read-only rows close the list and print muted, so the whole
            // block reads as one machine-value strip rather than as rows the
            // author forgot to fill in.
            // 只读行收尾并以暗色打印，使整块读起来像一条机器取值，而不是作者忘了填的几行。
            let style = if selected {
                Style::default().fg(Color::Black).bg(CYAN)
            } else if !carried {
                Style::default().fg(MUTED).add_modifier(Modifier::DIM)
            } else if add.is_editable(*index) {
                Style::default().fg(INK)
            } else {
                Style::default().fg(MUTED)
            };
            let role = if add.locked_fields.contains(index) {
                face_fields::FaceFieldRole::ReadOnly
            } else if add.parent_requirements.contains_key(index) {
                face_fields::FaceFieldRole::Required
            } else {
                presentation.role
            };
            ListItem::new(format!(
                "{marker} {} {:<name_width$} {value}",
                if carried { role.marker() } else { "·" },
                presentation.label,
            ))
            .style(style)
        })
        .collect::<Vec<_>>();
    let selected_row = fields.iter().position(|index| *index == add.field);
    let visible = body[0].height.saturating_sub(2) as usize;
    let offset = selected_row
        .unwrap_or_default()
        .saturating_sub(visible.saturating_sub(1));
    let mut state = ListState::default()
        .with_selected(selected_row)
        .with_offset(offset);
    frame.render_stateful_widget(
        List::new(rows).block(
            panel(&title, GREEN).title_bottom(
                Line::from(Span::styled(
                    " ↳ derived, or fixed after creation: shown, not typed ",
                    Style::default().fg(MUTED),
                ))
                .alignment(Alignment::Center),
            ),
        ),
        body[0],
        &mut state,
    );
    draw_face_field_help(frame, body[1], add);
    let areas = draw_form_frame(
        frame,
        inner[1],
        body[0],
        "Save [s]",
        "Cancel [Esc]",
        "* required  ◇ derived  ↳ read only  · optional  · not carried by this language",
    );
    (areas.list, areas.cancel, areas.confirm, areas.exit)
}
