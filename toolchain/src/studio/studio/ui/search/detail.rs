//! Selected-symbol preview, call lane, and lightweight Rust highlighting.
//! 选中符号预览、调用关系栏与轻量 Rust 高亮。

use super::super::*;

/// Compact first-page preview: the selected symbol and only its direct calls.
/// 第一页紧凑预览：只显示选中符号及其直接调用关系。
pub(super) fn draw_search_preview(
    frame: &mut Frame<'_>,
    area: Rect,
    app: &App,
    query: &str,
    selected: usize,
) {
    let rows = app.search_rows(query);
    let Some(row) = rows.get(selected) else {
        frame.render_widget(
            Paragraph::new("Type a file or function name.").block(panel(" PREVIEW ", MUTED)),
            area,
        );
        return;
    };
    let Some(node) = row.node else {
        frame.render_widget(
            Paragraph::new(row.text.as_str()).block(panel(" PREVIEW ", MUTED)),
            area,
        );
        return;
    };
    let Some(info) = app.registry.find_registry(node) else {
        return;
    };
    let source = source_path_for(&info.source.file);
    let text = std::fs::read_to_string(source).unwrap_or_default();
    let mut lines = vec![Line::from(vec![
        Span::styled(
            if row.function.is_empty() {
                "󰈙 "
            } else {
                "ƒ "
            },
            Style::default().fg(CYAN),
        ),
        Span::styled(
            if row.function.is_empty() {
                info.registry_name.to_owned()
            } else {
                row.function.clone()
            },
            Style::default().fg(GREEN).add_modifier(Modifier::BOLD),
        ),
    ])];
    lines.push(Line::from(Span::styled(
        info.source.file.to_owned(),
        Style::default().fg(MUTED),
    )));
    if row.function.is_empty() {
        lines.push(Line::from(Span::styled(
            "OBJECTS / FUNCTIONS",
            Style::default().fg(MAGENTA).add_modifier(Modifier::BOLD),
        )));
        for line in text
            .lines()
            .filter(|line| {
                let trimmed = line.trim_start();
                trimmed.starts_with("pub fn ")
                    || trimmed.starts_with("fn ")
                    || trimmed.starts_with("impl ")
            })
            .take(12)
        {
            let mut highlighted = highlight_rust_line(line.trim().trim_end_matches('{').trim());
            highlighted
                .spans
                .insert(0, Span::styled("  · ", Style::default().fg(CYAN)));
            lines.push(highlighted);
        }
    } else {
        let source_lines = text.lines().collect::<Vec<_>>();
        let (callers, callees) = app.call_relations(node, &row.function);
        let signature = text
            .lines()
            .find(|line| line.contains(&format!("{}(", row.function)))
            .map(|line| line.trim().trim_end_matches('{').trim().to_owned());
        if let Some(signature) = signature {
            lines.push(highlight_rust_line(&signature));
        }
        if let Some((start, end)) = app_function_source_range(&source_lines, &row.function) {
            lines.push(Line::from(Span::styled(
                "SOURCE PREVIEW",
                Style::default().fg(MAGENTA).add_modifier(Modifier::BOLD),
            )));
            let available = area.height.saturating_sub(lines.len() as u16 + 8) as usize;
            for (offset, source_line) in source_lines[start..=end]
                .iter()
                .enumerate()
                .take(available.max(4))
            {
                let mut highlighted = highlight_rust_line(source_line);
                highlighted.spans.insert(
                    0,
                    Span::styled(
                        format!(
                            "{:>4} ",
                            row.line.unwrap_or(info.source.line) + offset as u32
                        ),
                        Style::default().fg(MUTED),
                    ),
                );
                lines.push(highlighted);
            }
        }
        lines.push(Line::from(Span::styled(
            "← USED BY",
            Style::default().fg(MAGENTA).add_modifier(Modifier::BOLD),
        )));
        if callers.is_empty() {
            lines.push(Line::from(Span::styled(
                "  · no direct caller",
                Style::default().fg(MUTED),
            )));
        } else {
            for caller in callers.iter().take(8) {
                lines.push(Line::from(vec![
                    Span::styled("  ← ", Style::default().fg(CYAN)),
                    Span::styled(caller.function.clone(), Style::default().fg(INK)),
                ]));
            }
        }
        lines.push(Line::from(Span::styled(
            "CALLS →",
            Style::default().fg(MAGENTA).add_modifier(Modifier::BOLD),
        )));
        if callees.is_empty() {
            lines.push(Line::from(Span::styled(
                "  · no direct callee",
                Style::default().fg(MUTED),
            )));
        } else {
            for callee in callees.iter().take(8) {
                lines.push(Line::from(vec![
                    Span::styled("  → ", Style::default().fg(CYAN)),
                    Span::styled(callee.function.clone(), Style::default().fg(INK)),
                ]));
            }
        }
        lines.push(Line::from(Span::styled(
            "Enter opens the call graph",
            Style::default().fg(MUTED),
        )));
    }
    frame.render_widget(
        Paragraph::new(lines)
            .wrap(Wrap { trim: true })
            .block(panel(" SELECTED SYMBOL ", GREEN)),
        area,
    );
}

/// A narrow relationship lane kept between results and the preview.
/// 放在结果和预览之间的窄调用关系栏。
pub(super) fn draw_search_lane(frame: &mut Frame<'_>, area: Rect, app: &App, search: &SearchState) {
    let rows = app.search_rows(&search.query);
    let Some(row) = rows.get(search.selected) else {
        frame.render_widget(
            Paragraph::new("select a symbol").block(panel(" RELATION ", MUTED)),
            area,
        );
        return;
    };
    let Some(node) = row.node else {
        return;
    };
    let function = if row.function.is_empty() {
        app.registry
            .find_registry(node)
            .map(|info| info.source.function.as_str())
            .unwrap_or("")
    } else {
        row.function.as_str()
    };
    let (callers, callees) = app.call_relations(node, function);
    let mut lines = vec![Line::from(Span::styled(
        "INPUT / OUTPUT",
        Style::default().fg(MAGENTA).add_modifier(Modifier::BOLD),
    ))];
    for caller in callers.iter().take(5) {
        lines.push(Line::from(vec![
            Span::styled("← ", Style::default().fg(CYAN)),
            Span::styled(caller.function.clone(), Style::default().fg(INK)),
        ]));
    }
    lines.push(Line::from(Span::styled("────", Style::default().fg(MUTED))));
    lines.push(Line::from(Span::styled(
        if function.is_empty() {
            "file"
        } else {
            function
        },
        Style::default().fg(GREEN).add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(Span::styled("────", Style::default().fg(MUTED))));
    for callee in callees.iter().take(5) {
        lines.push(Line::from(vec![
            Span::styled("→ ", Style::default().fg(CYAN)),
            Span::styled(callee.function.clone(), Style::default().fg(INK)),
        ]));
    }
    frame.render_widget(
        Paragraph::new(lines)
            .wrap(Wrap { trim: true })
            .block(panel(" CALL LANE ", CYAN)),
        area,
    );
}

/// Highlight a compact Rust source line without pulling a parser into Studio.
/// 不引入解析器，只给紧凑 Rust 源码行提供足够清晰的颜色层次。
fn highlight_rust_line(source: &str) -> Line<'static> {
    let mut spans = Vec::new();
    let chars = source.chars().collect::<Vec<_>>();
    let mut index = 0;
    while index < chars.len() {
        if chars[index].is_whitespace() {
            let start = index;
            while index < chars.len() && chars[index].is_whitespace() {
                index += 1;
            }
            spans.push(Span::raw(chars[start..index].iter().collect::<String>()));
            continue;
        }
        if chars[index] == '/' && chars.get(index + 1) == Some(&'/') {
            spans.push(Span::styled(
                chars[index..].iter().collect::<String>(),
                Style::default().fg(MUTED),
            ));
            break;
        }
        if chars[index] == '"' {
            let start = index;
            index += 1;
            while index < chars.len() {
                if chars[index] == '\\' {
                    index = (index + 2).min(chars.len());
                    continue;
                }
                index += 1;
                if chars[index - 1] == '"' {
                    break;
                }
            }
            spans.push(Span::styled(
                chars[start..index].iter().collect::<String>(),
                Style::default().fg(MAGENTA),
            ));
            continue;
        }
        if chars[index].is_ascii_digit() {
            let start = index;
            while index < chars.len()
                && (chars[index].is_ascii_alphanumeric() || matches!(chars[index], '.' | '_'))
            {
                index += 1;
            }
            spans.push(Span::styled(
                chars[start..index].iter().collect::<String>(),
                Style::default().fg(Color::Yellow),
            ));
            continue;
        }
        if chars[index].is_ascii_alphabetic() || chars[index] == '_' {
            let start = index;
            index += 1;
            while index < chars.len()
                && (chars[index].is_ascii_alphanumeric() || chars[index] == '_')
            {
                index += 1;
            }
            let token = chars[start..index].iter().collect::<String>();
            let style = if matches!(
                token.as_str(),
                "fn" | "pub"
                    | "impl"
                    | "let"
                    | "mut"
                    | "if"
                    | "else"
                    | "match"
                    | "return"
                    | "struct"
                    | "enum"
                    | "trait"
                    | "for"
                    | "in"
                    | "use"
                    | "self"
                    | "Self"
            ) {
                Style::default().fg(CYAN).add_modifier(Modifier::BOLD)
            } else if token.chars().next().is_some_and(char::is_uppercase)
                || matches!(token.as_str(), "f32" | "f64" | "u32" | "usize" | "String")
            {
                Style::default().fg(GREEN)
            } else {
                Style::default().fg(INK)
            };
            spans.push(Span::styled(token, style));
            continue;
        }
        spans.push(Span::styled(
            chars[index].to_string(),
            Style::default().fg(MUTED),
        ));
        index += 1;
    }
    Line::from(spans)
}
