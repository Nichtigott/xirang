//! Search result list rendering.
//! 搜索结果列表渲染。

use super::super::*;

use crate::studio::studio::app::SearchRow;

pub(super) struct SearchListOptions<'a> {
    pub(super) query: &'a str,
    pub(super) selected: usize,
    pub(super) offset: usize,
    pub(super) active: bool,
    pub(super) title: &'static str,
}

pub(super) fn draw_search_list(
    frame: &mut Frame<'_>,
    area: Rect,
    app: &App,
    options: SearchListOptions<'_>,
) -> usize {
    let SearchListOptions {
        query,
        selected,
        offset,
        active,
        title,
    } = options;
    let rows = app.search_rows(query);
    let items = rows
        .iter()
        .enumerate()
        .map(|(index, row)| {
            // Only file and function rows exist: `search_rows` returns a flat
            // list, so there is no fold state and no expand/collapse marker.
            // 只有文件和函数两类行：`search_rows` 返回扁平列表，因此没有折叠
            // 状态，也没有展开/折叠标记。
            let marker = if row.function.is_empty() {
                "󰈙"
            } else {
                "ƒ"
            };
            let style = if index == selected && active {
                Style::default().fg(Color::Black).bg(CYAN)
            } else {
                Style::default().fg(INK)
            };
            let line = search_row_line(row, query, marker);
            ListItem::new(line).style(style)
        })
        .collect::<Vec<_>>();
    let selected = selected.min(rows.len().saturating_sub(1));
    let mut state = ListState::default()
        .with_selected((!rows.is_empty()).then_some(selected))
        .with_offset(offset);
    frame.render_stateful_widget(
        List::new(items).block(panel(title, if active { CYAN } else { MAGENTA })),
        area,
        &mut state,
    );
    state.offset()
}

fn search_row_line(row: &SearchRow, query: &str, marker: &str) -> Line<'static> {
    let marker = Span::styled(format!("{marker} "), Style::default().fg(CYAN));
    if row.function.is_empty() {
        let path = row.path.as_str();
        let split = path.rfind('/').map(|index| index + 1).unwrap_or(0);
        return Line::from(vec![
            marker,
            Span::styled(path[..split].to_owned(), Style::default().fg(MUTED)),
            Span::styled(
                path[split..].to_owned(),
                search_hit_style(query, &path[split..]),
            ),
        ]);
    }
    let function_style = search_hit_style(query, &row.function);
    Line::from(vec![
        marker,
        Span::styled(row.path.clone(), Style::default().fg(MUTED)),
        Span::styled(" -> fn ", Style::default().fg(MUTED)),
        Span::styled(row.function.clone(), function_style),
    ])
}

fn search_hit_style(query: &str, value: &str) -> Style {
    if !query.trim().is_empty()
        && value
            .to_ascii_lowercase()
            .contains(&query.trim().to_ascii_lowercase())
    {
        Style::default().fg(INK).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(INK)
    }
}
