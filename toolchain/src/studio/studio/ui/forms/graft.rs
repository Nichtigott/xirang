//! Graft composition form rendering.
//! graft 撰写表单渲染。

use super::super::*;
use super::draw_form_frame;

use crate::studio::studio::app::{GraftDeclaration, GraftState};

pub(crate) fn draw_graft(
    frame: &mut Frame<'_>,
    area: Rect,
    graft: &GraftState,
) -> (Rect, Rect, Rect, Rect, Rect) {
    let inner = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(9), Constraint::Length(4)])
        .split(area);
    let body = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(58), Constraint::Percentage(42)])
        .split(inner[0]);

    let marker = |selected: bool| {
        if selected && graft.editing {
            "●"
        } else if selected {
            "›"
        } else {
            " "
        }
    };
    let compose = |line: String, selected: bool| {
        let style = if selected {
            Style::default().fg(Color::Black).bg(CYAN)
        } else {
            Style::default().fg(INK)
        };
        Line::from(Span::styled(line, style))
    };
    let scope = if graft.full {
        "full — replaces the subtree"
    } else {
        "cut — keeps the children"
    };
    let mut lines = vec![
        field("target", &graft.target_path),
        compose(
            format!(
                "{} selector     {}",
                marker(graft.field == 0 && graft.pane == 0),
                if graft.selector.is_empty() {
                    "_"
                } else {
                    &graft.selector
                }
            ),
            graft.pane == 0 && graft.field == 0,
        ),
        compose(
            format!(
                "{} scope        {scope}",
                marker(graft.field == 1 && graft.pane == 0)
            ),
            graft.pane == 0 && graft.field == 1,
        ),
        Line::from(""),
        Line::from(Span::styled("DECLARED SLOT", Style::default().fg(MUTED))),
    ];
    match &graft.declaration {
        GraftDeclaration::Declared {
            expression,
            line,
            cfg,
        } => {
            lines.push(Line::from(Span::styled(
                format!("✓ declared at line {line}: {expression}"),
                Style::default().fg(GREEN),
            )));
            if let Some(cfg) = cfg {
                lines.push(Line::from(Span::styled(
                    format!("  gated by cfg({cfg}) — the build follows that feature"),
                    Style::default().fg(MUTED),
                )));
            }
        }
        GraftDeclaration::Absent { entry } => {
            lines.push(Line::from(Span::styled(
                "✗ this slot is not declared by the host entry",
                Style::default().fg(Color::LightRed),
            )));
            lines.push(Line::from(Span::styled(
                format!("  {} will not ship this face", entry.display()),
                Style::default().fg(MUTED),
            )));
        }
        GraftDeclaration::Unknown { reason } => {
            lines.push(Line::from(Span::styled(
                format!("? {reason}"),
                Style::default().fg(MUTED),
            )));
        }
    }
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "PRE-FLIGHT",
        Style::default().fg(MUTED),
    )));
    if graft.flow_declared {
        lines.push(Line::from(Span::styled(
            "✓ flow contract declared",
            Style::default().fg(GREEN),
        )));
    } else {
        lines.push(Line::from(Span::styled(
            "! no flow contract: overlay requires one on both sides",
            Style::default().fg(Color::LightRed),
        )));
    }
    if graft.inherited_children > 0 {
        lines.push(Line::from(Span::styled(
            format!(
                "· {} child face(s) {}",
                graft.inherited_children,
                if graft.full {
                    "are dropped by a full cut"
                } else {
                    "are inherited by the overlay"
                }
            ),
            Style::default().fg(MUTED),
        )));
    }
    if graft
        .plans
        .iter()
        .any(|plan| plan.selector == graft.selector.trim())
    {
        lines.push(Line::from(Span::styled(
            "! that selector already exists; o opens it, d deletes it (press d twice)",
            Style::default().fg(Color::LightRed),
        )));
    }
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "ENTRY DECLARATION (paste into static_graft_plan!)",
        Style::default().fg(MUTED),
    )));
    if let Some(error) = graft_declaration_error(graft) {
        lines.push(Line::from(Span::styled(error, Style::default().fg(MUTED))));
    } else {
        let document = crate::run_method::GraftPlanDocument::new(
            graft.target,
            graft.target_path.clone(),
            graft.selector.trim(),
            graft.full,
        );
        lines.push(Line::from(Span::styled(
            format!("  {}", document.declaration()),
            Style::default().fg(CYAN),
        )));
    }
    frame.render_widget(
        Paragraph::new(lines)
            .wrap(Wrap { trim: false })
            .block(panel(" COMPOSE EXTERNAL GRAFT ", GREEN)),
        body[0],
    );

    let rows = if graft.plans.is_empty() {
        vec![ListItem::new("no plans yet").style(Style::default().fg(MUTED))]
    } else {
        graft
            .plans
            .iter()
            .enumerate()
            .map(|(index, plan)| {
                let selected = graft.pane == 1 && index == graft.plan_selected;
                let scope = if plan.full { "full" } else { "cut" };
                let detail = if let Some(error) = &plan.error {
                    format!("{}  invalid: {error}", plan.selector)
                } else {
                    format!("{}  {}  {scope}", plan.selector, plan.target_path)
                };
                ListItem::new(format!(" {} {detail}", if selected { "›" } else { " " })).style(
                    if selected {
                        Style::default().fg(Color::Black).bg(CYAN)
                    } else {
                        Style::default().fg(INK)
                    },
                )
            })
            .collect()
    };
    // `then_some` evaluates its argument eagerly, so `plans.len() - 1` underflowed
    // on an empty list: in every debug build (the repo's own `target/debug`, any
    // `cargo run`) pressing `g` on a fresh project panicked before a first plan
    // could exist. `checked_sub` asks the question instead of assuming the answer.
    // `then_some` 会立即求值它的参数，因此 `plans.len() - 1` 在空列表上会下溢：在每种 debug
    // 构建（仓库自己的 `target/debug`、任何 `cargo run`）里，在全新工程上按 `g` 会在第一个
    // 计划存在之前就 panic。`checked_sub` 是去问这个问题，而不是假定答案。
    let mut state = ListState::default().with_selected(
        graft
            .plans
            .len()
            .checked_sub(1)
            .map(|last| graft.plan_selected.min(last)),
    );
    let border = if graft.pane == 1 { CYAN } else { MUTED };
    frame.render_stateful_widget(
        List::new(rows).block(panel(" EXISTING PLANS ", border)),
        body[1],
        &mut state,
    );

    let areas = draw_form_frame(
        frame,
        inner[1],
        body[1],
        "Write [s]",
        "Close [Esc]",
        "↑↓ row  Tab pane  Enter edit/toggle  o open  f full  d delete (press twice)",
    );
    // The two clickable compose rows: the panel's inner first line is `target`,
    // so `selector` and `scope` start one row lower.
    // 撰写区可点击的两行：面板内首行是 `target`，因此 `selector` 与 `scope` 各低一行。
    let compose = Rect::new(
        body[0].x.saturating_add(1),
        body[0].y.saturating_add(2),
        body[0].width.saturating_sub(2),
        2,
    );
    (areas.list, areas.cancel, areas.confirm, areas.exit, compose)
}

/// A selector the entry declaration cannot be rendered for yet.
/// 还无法渲染入口声明的选择器。
fn graft_declaration_error(graft: &GraftState) -> Option<String> {
    crate::run_method::validate_graft_selector(graft.selector.trim())
        .err()
        .map(|error| format!("  ({error})"))
}
