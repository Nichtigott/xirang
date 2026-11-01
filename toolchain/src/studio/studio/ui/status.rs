//! Event log panel and the footer key help.
//! 事件日志面板与页脚按键提示。

use super::*;

pub(super) fn draw_event(frame: &mut Frame<'_>, area: Rect, app: &App) {
    // One way an event asks for the warning colour, and it is a property of the line
    // rather than of its wording: `App::note` / `App::alert` decided it where the line
    // was written, and the text is free to say "Warning:" or "failed" for the reader
    // without anything parsing it (audit `STU-S-18`).
    // 事件请求警示色只有一条途径，而且它是**那一行的属性**而不是措辞的属性：`App::note` /
    // `App::alert` 在写入它的地方就定好了；文本可以对读者写 “Warning:” 或 “failed”，但没有
    // 任何东西去解析它（审计 `STU-S-18`）。
    let color = if app.event_is_alert {
        Color::LightRed
    } else {
        GREEN
    };
    let event = app.reload_error.as_ref().map_or_else(
        || app.event.clone(),
        |error| format!("{}\n[phase={}] {}", app.event, error.phase, error.message),
    );
    frame.render_widget(
        Paragraph::new(event)
            .style(Style::default().fg(color))
            .wrap(Wrap { trim: false })
            .block(panel(" EVENT / BUILD ", MAGENTA)),
        area,
    );
}

pub(super) fn draw_keys(frame: &mut Frame<'_>, area: Rect) {
    // This is the persistent workspace footer: the subset of the Inspect/workspace
    // keys a reader needs, not every binding `App::handle_key` accepts. The ones that
    // cannot be guessed are named — `j`/`k` alias `↑`/`↓`, `Tab` moves the focus
    // between panes, and `Enter` is fold in the tree and edit in details (audit
    // `STU-C-06`). `m MIR` is deliberately absent: it is only matched inside the
    // call-graph branch of `handle_search_overlay_key`, and the graph footer in
    // `super::search` advertises it instead. Advertising it here made the footer lie
    // whenever no search overlay was open.
    // 这是常驻工作区页脚：读者需要的那个子集，而不是 `App::handle_key` 接受的全部绑定。猜不到
    // 的那些要写出来——`j`/`k` 是 `↑`/`↓` 的别名，`Tab` 在两块面板间切焦点，`Enter` 在树上折叠、
    // 在详情里编辑（审计 `STU-C-06`）。这里刻意不写 `m MIR`：它只在 `handle_search_overlay_key`
    // 的调用图分支里匹配，改由 `super::search` 的调用图页脚展示。写在这里会在没有搜索浮层时
    // 谎报按键。
    frame.render_widget(
        Paragraph::new(" 1 search   2 inspect   3 data   q quit   / search   n new   a add   g graft   p plugin   c crates   e edit   d delete   Tab focus   j/k move   Enter fold/edit   r/F5 reload   b/F9 build   ←/→ resize ")
            .alignment(Alignment::Center)
            .style(Style::default().fg(MUTED)),
        area,
    );
}
