//! Partition-screen rendering (audit `M7`, P3.6).
//! 分区屏渲染（审计 `M7`，P3.6）。
//!
//! Two questions this screen exists to answer without leaving the TUI: **what would this write**, and
//! **could it be published**. The first is the plan (packages, the subtrees each one serves, how many
//! files it carries), the second is the per-package publishing note the shared reader produces.
//! 本屏存在的意义是不离开 TUI 就能回答两个问题：**这次写入会写下什么**，以及**它能不能发布**。前者是计划
//! （有哪些包、各自服务哪棵子树、携带多少文件），后者是共用读取器为每个包给出的发布说明。

use super::super::*;

/// One package's block of lines.
/// 一个包的那一块行。
fn package_lines(package: &crate::build_method::PackageView) -> Vec<Line<'static>> {
    let role = match &package.crate_name {
        Some(name) => format!("crate {name} ({} )", package.subtrees.join(", ")),
        None => "facade (carries the cross-crate half)".to_owned(),
    };
    let on_disk = match package.on_disk {
        crate::build_method::OnDisk::Absent => "absent",
        crate::build_method::OnDisk::Development => "development",
        crate::build_method::OnDisk::Release => "release",
        crate::build_method::OnDisk::Foreign => "NOT this action's package",
    };
    let kilobytes = package.bytes as f64 / 1024.0;
    let mut lines = vec![
        Line::from(Span::styled(
            package.package.clone(),
            Style::default().fg(INK).add_modifier(Modifier::BOLD),
        )),
        field("role", &role),
        field("on disk", on_disk),
        field(
            "carries",
            &format!(
                "{} face(s) compiled · {} mounted (development) · {} copied (release) · {} file(s), {:.1} KiB",
                package.compiles, package.mounts, package.copies, package.files, kilobytes
            ),
        ),
        field("at", &package.directory.display().to_string()),
    ];
    if !package.depends_on.is_empty() {
        lines.push(field("depends on", &package.depends_on.join(", ")));
    }
    for note in &package.publish {
        lines.push(field("publish", note));
    }
    lines.push(Line::from(""));
    lines
}

pub(crate) fn draw_partition(
    frame: &mut Frame<'_>,
    area: Rect,
    state: &PartitionState,
) -> (Rect, Rect) {
    let mut text: Vec<Line<'static>> = Vec::new();
    if let Some(error) = &state.error {
        text.push(Line::from(Span::styled(
            error.clone(),
            Style::default().fg(Color::LightRed),
        )));
        text.push(Line::from(""));
    } else if !state.declared {
        text.push(Line::from(Span::styled(
            "This host declares no crates: it is one crate, and `add_crates.rs` is where a split is \
             declared."
                .to_owned(),
            Style::default().fg(INK),
        )));
        text.push(Line::from(""));
    }
    if let Some(view) = &state.view {
        text.push(field(
            "declared",
            &format!(
                "package_prefix `{}` · {} face(s) in the host's own tree · {} generated package(s)",
                view.package_prefix,
                view.host_faces,
                view.packages.len()
            ),
        ));
        text.push(field(
            "belongs to",
            &match &view.workspace {
                Some(workspace) => format!(
                    "workspace {} (config and member list live there)",
                    workspace.display()
                ),
                None => format!(
                    "no enclosing workspace; the config goes to {}",
                    view.config_root.display()
                ),
            },
        ));
        if !view.members_missing.is_empty() {
            text.push(field(
                "members",
                &format!(
                    "{} generated package(s) are not in the workspace member list: {}",
                    view.members_missing.len(),
                    view.members_missing.join(", ")
                ),
            ));
        }
        for note in &view.notes {
            text.push(field("note", note));
        }
        text.push(Line::from(""));
        for (index, package) in view.packages.iter().enumerate() {
            // The selected row keeps a mark so the arrow keys have something to move.
            // 选中的那一行带一个标记，好让方向键有东西可移。
            let mark = if index == state.selected { ">" } else { " " };
            text.push(Line::from(Span::styled(
                format!("{mark} {}", package.package),
                Style::default().fg(INK).add_modifier(Modifier::BOLD),
            )));
            text.extend(package_lines(package).into_iter().skip(1));
        }
    }
    if let Some(pending) = state.pending {
        text.push(Line::from(Span::styled(
            format!("{} to {}", pending.key(), pending.sentence()),
            Style::default().fg(Color::LightYellow),
        )));
    }
    if let Some(name) = &state.pending_undeclare {
        text.push(Line::from(Span::styled(
            format!(
                "D to remove `{name}` from {} — press y to confirm (the generated package is not \
                 touched; revert it with x first)",
                xirang_kernel::lexicon::ADD_CRATES_FILE
            ),
            Style::default().fg(Color::LightYellow),
        )));
    }
    if let Some(outcome) = &state.outcome {
        text.push(Line::from(Span::styled(
            outcome.clone(),
            Style::default().fg(MUTED),
        )));
    }
    text.push(Line::from(""));
    text.push(Line::from(Span::styled(
        "w development · R release · x revert · D remove the selected crate from the declaration \
         (each asks once more) · r reread · q/Esc close"
            .to_owned(),
        Style::default().fg(MUTED),
    )));
    frame.render_widget(
        Paragraph::new(text)
            .wrap(Wrap { trim: false })
            .block(panel(" CRATE PARTITION ", Color::LightCyan)),
        area,
    );
    let buttons = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(30),
            Constraint::Length(14),
            Constraint::Length(3),
            Constraint::Length(20),
            Constraint::Percentage(30),
        ])
        .split(Rect::new(
            area.x,
            area.bottom().saturating_sub(4),
            area.width,
            3,
        ));
    frame.render_widget(
        Paragraph::new("Close")
            .alignment(Alignment::Center)
            .block(panel(" q / Esc ", MUTED)),
        buttons[1],
    );
    frame.render_widget(
        Paragraph::new("Confirm")
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::LightYellow))
            .block(panel(" y ", Color::LightYellow)),
        buttons[3],
    );
    (buttons[1], buttons[3])
}
