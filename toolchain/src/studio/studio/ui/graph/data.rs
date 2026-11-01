//! Live-value data-flow panel for the call-graph overlay.
//! 调用图浮层的实时值数据流面板。
//!
//! The rows come from `App::runtime_trace`, and the title states what supplied it:
//! `App::trace_data_note` is empty only when a trace artifact was loaded, reads
//! `no trace attached` when none was found, and names a short form of the reason
//! when one was refused. A refused artifact installs no values, so the panel
//! draws its empty state under a title that says why instead of staying silent.
//! 各行来自 `App::runtime_trace`，标题说明它由什么提供：`App::trace_data_note` 只有在装入
//! 了 trace artifact 时才为空，没找到时读作 `no trace attached`，被拒绝时给出原因的简短形式。
//! 被拒绝的 artifact 不会装入任何数值，因此面板在说明缘由的标题下绘制空状态，而不是沉默。

use super::*;

/// Selectable runtime values for the active function.
/// 当前函数的可选择运行时值。
pub(super) fn draw_data_flow_panel(
    frame: &mut Frame<'_>,
    area: Rect,
    app: &App,
    selected: Option<&CallRef>,
    cursor: usize,
    focused: bool,
    title: &str,
) {
    let Some(item) = selected else {
        frame.render_widget(
            Paragraph::new("Select a call node.").block(panel(title, MUTED)),
            area,
        );
        return;
    };
    let locals = app.graph_locals(item);
    let mut rows = Vec::new();
    if locals.is_empty() {
        rows.push(Line::from(Span::styled(
            "· no live locals captured",
            Style::default().fg(MUTED),
        )));
    } else {
        for local in locals {
            let id = crate::run_method::LocalId(local.id);
            let upstream = app
                .runtime_trace
                .incoming(id)
                .into_iter()
                .filter_map(|hop| {
                    hop.value.map(|candidate| {
                        format!(
                            "{}={} [{}]",
                            candidate.name, candidate.value, hop.edge.label
                        )
                    })
                })
                .take(2)
                .collect::<Vec<_>>();
            let downstream = app
                .runtime_trace
                .outgoing(id)
                .into_iter()
                .filter_map(|hop| {
                    hop.value.map(|candidate| {
                        format!(
                            "{}={} [{}]",
                            candidate.name, candidate.value, hop.edge.label
                        )
                    })
                })
                .take(2)
                .collect::<Vec<_>>();
            let frame_id = local.frame_id.unwrap_or(0);
            let mut line = vec![
                Span::styled(
                    format!("#{} {} ", frame_id, local.kind.label()),
                    Style::default().fg(CYAN),
                ),
                Span::styled(
                    format!("{} ", local.name),
                    Style::default().fg(INK).add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!("= {} [{}]", local.value, local.observation.label()),
                    Style::default().fg(GREEN),
                ),
            ];
            if !upstream.is_empty() {
                line.push(Span::styled(
                    format!("  ← {}", upstream.join(", ")),
                    Style::default().fg(MAGENTA),
                ));
            }
            if !downstream.is_empty() {
                line.push(Span::styled(
                    format!("  → {}", downstream.join(", ")),
                    Style::default().fg(CYAN),
                ));
            }
            line.push(Span::styled(
                format!("  {}", local.source),
                Style::default().fg(MUTED),
            ));
            rows.push(Line::from(line));
        }
    }
    let selected_index = (!rows.is_empty()).then_some(cursor.min(rows.len().saturating_sub(1)));
    let mut state = ListState::default().with_selected(selected_index);
    // The note names the trace's state, so a reader never has to guess whether
    // the values below came from this project or from nothing at all.
    // 说明文字点名追踪的状态，因此读者无需猜测下方数值来自本项目还是一无所有。
    let note = app.trace_data_note();
    let panel_title = if note.is_empty() {
        format!("{title} · {}", item.function)
    } else {
        format!("{title} · {note} · {}", item.function)
    };
    frame.render_stateful_widget(
        List::new(rows)
            .highlight_style(
                Style::default()
                    .fg(Color::Black)
                    .bg(if focused { GREEN } else { CYAN })
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("▸ ")
            .block(panel(panel_title, if focused { GREEN } else { MUTED })),
        area,
        &mut state,
    );
}
