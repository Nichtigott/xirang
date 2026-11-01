//! Main workspace panels: brand, registration tree, and face inspector.
//! 主工作区面板：标题、注册树与注册面检视器。
//!
//! The status line and footer live in `super::status`; the overlay router in
//! `super::overlay`. This page only draws the persistent three-column body.
//! 状态行与页脚位于 `super::status`；浮层路由位于 `super::overlay`。
//! 本页只绘制常驻的三栏主体。

use super::*;

pub(super) fn draw_brand(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let mut art = XIRANG_MARK.into_iter().map(Line::from).collect::<Vec<_>>();
    art.push(Line::from(vec![
        // The legend states what was actually loaded. `LIVE` means a trace
        // artifact passed the identity checks in `app::trace`; with no artifact it
        // reads `TRACE: none`, and with a refused one `TRACE mismatch`. There is
        // no sample behind it any more, so a bare `LIVE` can never advertise
        // values this session does not have.
        // 图例陈述实际装入的东西。`LIVE` 表示某份 trace artifact 通过了 `app::trace` 的身份
        // 检查；没有 artifact 时读作 `TRACE: none`，被拒绝时读作 `TRACE mismatch`。它背后
        // 已经没有示例，因此裸的 `LIVE` 绝不会宣称本会话并不持有的数值。
        Span::styled(
            format!("  {}  ", app.trace_legend()),
            Style::default()
                .fg(if app.trace_is_live() { GREEN } else { MUTED })
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!(
                "{}  nodes {}  registries {}  checks {}  studio adapter",
                app.page.label(),
                app.registry.node_count(),
                app.registry.registry_count(),
                app.registry.check_count()
            ),
            Style::default().fg(MUTED),
        ),
    ]));
    frame.render_widget(
        Paragraph::new(art)
            .alignment(Alignment::Center)
            .style(Style::default().fg(INK).bg(Color::Rgb(12, 55, 65)))
            .block(panel(" NICH LINK // REGISTRATION STUDIO ", CYAN)),
        area,
    );
}

pub(super) fn draw_workspace(
    frame: &mut Frame<'_>,
    area: Rect,
    app: &App,
    cache: &mut RenderCache,
) {
    cache.hot.workspace_area = area;
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(app.split_percent),
            Constraint::Percentage(100 - app.split_percent),
        ])
        .split(area);
    cache.hot.tree_area = columns[0];
    cache.hot.details_area = columns[1];
    draw_tree(frame, columns[0], app, cache);
    draw_details(frame, columns[1], app);
}

pub(super) fn draw_tree(frame: &mut Frame<'_>, area: Rect, app: &App, cache: &mut RenderCache) {
    let nodes = app.visible_nodes();
    let selected = nodes
        .iter()
        .position(|(id, _)| *id == app.selected)
        .unwrap_or_default();
    let items = nodes
        .into_iter()
        .map(|(id, depth)| {
            let selected = id == app.selected;
            let branch = if app.owns_registry(id) {
                if app.collapsed.contains(&id) {
                    "▸"
                } else {
                    "▾"
                }
            } else {
                "·"
            };
            let style = if selected {
                Style::default()
                    .fg(Color::Black)
                    .bg(CYAN)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(INK)
            };
            ListItem::new(format!(
                "{}{} {}",
                "  ".repeat(depth),
                branch,
                app.node_label(id)
            ))
            .style(style)
        })
        .collect::<Vec<_>>();
    let border = if app.focus == Focus::Tree {
        CYAN
    } else {
        MUTED
    };
    let mut state = ListState::default()
        .with_selected(Some(selected))
        .with_offset(cache.tree_offset);
    frame.render_stateful_widget(
        List::new(items).block(panel(" REGISTRATION TREE ", border)),
        area,
        &mut state,
    );
    cache.tree_offset = state.offset();
}

pub(crate) fn draw_details(frame: &mut Frame<'_>, area: Rect, app: &App) {
    // One source for the rows: `App::detail_rows` also answers
    // `App::detail_field_count`, so the drawing and the keys that walk it cannot
    // disagree (audit `STU-S-02`).
    // 行只有一份来源：`App::detail_rows` 同时也是 `App::detail_field_count` 的答案，因此绘制与
    // 走它的按键不会各说一套（审计 `STU-S-02`）。
    let values = app.detail_rows();
    let inner_width = area.width.saturating_sub(2) as usize;
    let selected = app.details_selected.min(values.len().saturating_sub(1));
    let lines = values
        .into_iter()
        .enumerate()
        .flat_map(|(index, (name, value))| {
            let mut field_lines = detail_field(name, value, inner_width);
            if index == selected
                && let Some(first) = field_lines.first_mut()
            {
                first.spans.insert(
                    0,
                    Span::styled(
                        "› ",
                        Style::default()
                            .fg(if app.focus == Focus::Details {
                                Color::Black
                            } else {
                                CYAN
                            })
                            .add_modifier(Modifier::BOLD),
                    ),
                );
                if app.focus == Focus::Details {
                    first.style = Style::default().bg(CYAN).fg(Color::Black);
                }
            }
            field_lines
        })
        .collect::<Vec<_>>();
    let border = if app.focus == Focus::Details {
        CYAN
    } else {
        MUTED
    };
    frame.render_widget(
        Paragraph::new(lines)
            .wrap(Wrap { trim: false })
            .block(panel(" FACE INSPECTOR ", border)),
        area,
    );
}
