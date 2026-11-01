//! Ratatui rendering for Studio.
//! Studio 的 Ratatui 渲染。
//!
//! This root owns the top-level [`draw`] layout, the shared chrome helpers, and
//! the palette. The persistent panels live in `panels`, the event log and
//! footer in `status`, and the modal router in `overlay`; the search and
//! form widgets keep their own mounted pages.
//! 本模块根承载顶层 [`draw`] 布局、共享外观辅助函数与调色板。常驻面板位于
//! `panels`，事件日志与页脚位于 `status`，模态路由位于 `overlay`；
//! 搜索与表单控件保留各自挂载的页面。

use ratatui::Frame;
use ratatui::backend::Backend;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, BorderType, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap,
};

mod graph;
use graph::draw_search_graph;
#[path = "render_cache.rs"]
mod render_cache;
pub use render_cache::RenderCache;
#[path = "mark.rs"]
mod mark;
use mark::XIRANG_MARK;
#[path = "overlay.rs"]
mod overlay;
use overlay::draw_overlay;
#[path = "panels.rs"]
pub(crate) mod panels;
use panels::{draw_brand, draw_workspace};
mod forms;
use forms::{
    draw_add, draw_delete, draw_edit, draw_graft, draw_new_project, draw_partition, draw_plugin,
};
mod search;
use search::draw_search;
#[path = "multilang.rs"]
mod multilang;
#[path = "status.rs"]
mod status;
use status::{draw_event, draw_keys};

use super::app::{
    AddState, App, CallRef, Focus, Overlay, PartitionState, SearchState, app_function_source_range,
    face_field_indices, new_project_field, plugin_field, source_path_for,
};
// The call-tree model types are named only by the widget drawing path, so they are
// imported with it: without the `node-graph` feature the import would be unused.
// 调用树模型类型只被控件绘制路径命名，因此与它一同导入：没有 `node-graph` 特性时该导入会
// 未使用。
#[cfg(feature = "node-graph")]
use super::app::{CallTreeNode, CallTreeView};
// The authoring layout's slot names: the form, the appliers and the tests index
// one array, so they all read the same constants.
// 创作布局的槽位名：表单、写入方与测试索引同一个数组，因此都读同一批常量。
use super::app::StudioPage;
use crate::run_method::face_field;

const INK: Color = Color::Rgb(214, 225, 231);
const MUTED: Color = Color::Rgb(112, 132, 143);
const CYAN: Color = Color::Rgb(75, 201, 220);
const GREEN: Color = Color::Rgb(97, 210, 151);
const MAGENTA: Color = Color::Rgb(223, 116, 186);
const PANEL: Color = Color::Rgb(16, 23, 28);

/// Draw one complete Studio frame: brand, workspace, event log, key hints, overlay.
/// 绘制一帧完整的 Studio 界面：品牌、工作区、事件日志、按键提示与浮层。
///
/// The caller owns redraw cadence; Studio draws only after input or a terminal event.
/// 重绘节奏由调用方掌握；Studio 只在有输入或终端事件后绘制。
pub fn draw(frame: &mut Frame<'_>, app: &App, cache: &mut RenderCache) {
    let area = frame.area();
    frame.render_widget(
        Block::default().style(Style::default().bg(Color::Rgb(8, 12, 15))),
        area,
    );
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            // Seven logo rows plus status and borders keep the complete mark visible.
            // 七行标题、状态行和边框一起保留完整字标，避免裁掉底行。
            Constraint::Length(10),
            Constraint::Min(9),
            Constraint::Length(5),
            Constraint::Length(2),
        ])
        .split(area);
    draw_brand(frame, rows[0], app);
    if app.page == StudioPage::Multilang {
        multilang::draw_multilang(frame, rows[1], app);
    } else {
        draw_workspace(frame, rows[1], app, cache);
    }
    draw_event(frame, rows[2], app);
    draw_keys(frame, rows[3]);
    draw_overlay(frame, app, cache);
}

/// Draw one complete frame the way the event loop does: take the render cache out of the
/// session, draw, and apply it back before the next input event.
/// 按事件循环的方式绘制完整一帧：把渲染缓存从会话中取出、绘制，并在下一个输入事件之前应用回去。
///
/// Both the loop and the render tests go through here, so "one frame" means one thing
/// (audit `STU-S-04`).
/// 事件循环与渲染测试都走这里，因此“一帧”只有一种含义（审计 `STU-S-04`）。
pub fn draw_once<B: Backend>(
    terminal: &mut ratatui::Terminal<B>,
    app: &mut App,
) -> Result<(), B::Error> {
    let mut cache = RenderCache::take(app);
    terminal.draw(|frame| draw(frame, app, &mut cache))?;
    cache.apply(app);
    Ok(())
}

fn panel(title: impl Into<String>, color: Color) -> Block<'static> {
    Block::default()
        .title(title.into())
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(color))
        .style(Style::default().bg(PANEL))
}

fn field(name: &str, value: &str) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("{name:<15}"), Style::default().fg(MUTED)),
        Span::styled(value.to_owned(), Style::default().fg(INK)),
    ])
}

fn detail_field(name: &str, value: String, width: usize) -> Vec<Line<'static>> {
    if 15 + value.chars().count() <= width {
        return vec![field(name, &value)];
    }
    vec![
        Line::from(Span::styled(name.to_owned(), Style::default().fg(MUTED))),
        Line::from(Span::styled(format!("  {value}"), Style::default().fg(INK))),
    ]
}

fn centered(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(vertical[1])[1]
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{Terminal, backend::TestBackend};

    fn rendered_text(width: u16, height: u16) -> String {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        let mut app = App::load_app();
        draw_once(&mut terminal, &mut app).unwrap();
        terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect()
    }

    /// The session fields a frame must never touch: the ones a draw could plausibly reach,
    /// plus the ones that decide what it draws. The cache fields (`hot`, the tree offset, the
    /// search viewport, the graph widget) are the frame's *output*, applied by `draw_once`;
    /// everything here has to come out identical (audit `STU-S-04`).
    /// 一帧绝不得触碰的会话字段：绘制可能触及的那些，加上决定它画什么的那些。缓存字段
    /// （`hot`、树偏移、搜索视口、调用图控件）是帧的**产出**，由 `draw_once` 应用；这里的一切必须
    /// 原样返回（审计 `STU-S-04`）。
    fn session_snapshot(app: &App) -> String {
        format!(
            "event={:?} selected={:?} page={:?} focus={:?} details={} split={} graph_split={} stamp={} quit={} registry={:?}",
            app.event,
            app.selected,
            app.page,
            app.focus,
            app.details_selected,
            app.split_percent,
            app.graph_split_percent,
            app.source_stamp(),
            app.should_quit,
            app.registry.id(),
        )
    }

    /// The screen as flat text, for a terminal that already exists.
    /// 已存在终端上的屏幕平铺文本。
    fn screen(terminal: &Terminal<TestBackend>) -> String {
        terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect()
    }

    /// A frame is an observation: it does not change the session, and the same session draws
    /// the same picture twice. This is the pin for audit `STU-S-04`.
    /// 一帧是一次观察：它不改变会话，而同一个会话两次画出同一幅画。这是审计 `STU-S-04` 的钉子。
    #[test]
    fn a_frame_observes_the_session_without_changing_it() {
        let mut terminal = Terminal::new(TestBackend::new(140, 48)).unwrap();
        let mut app = App::load_app();
        app.overlay = Some(Overlay::Search(SearchState {
            query: "canvas".to_owned(),
            ..SearchState::default()
        }));
        let before = session_snapshot(&app);

        draw_once(&mut terminal, &mut app).unwrap();
        let first = screen(&terminal);
        assert_eq!(
            session_snapshot(&app),
            before,
            "drawing a frame changed the session (audit `STU-S-04`)"
        );

        draw_once(&mut terminal, &mut app).unwrap();
        assert_eq!(
            screen(&terminal),
            first,
            "the same session must draw the same frame"
        );
        assert_eq!(
            session_snapshot(&app),
            before,
            "and drawing it twice must still not change it"
        );
    }

    /// The render path itself never writes the session: the only writes under `ui/` are the
    /// applier's three lines in `render_cache.rs`, which is what `draw_once` is for
    /// (audit `STU-S-04`).
    /// 渲染路径本身从不写会话：`ui/` 下唯一的写入是 `render_cache.rs` 里应用器的那三行，而那正是
    /// `draw_once` 的用途（审计 `STU-S-04`）。
    #[test]
    fn the_render_path_never_writes_the_session() {
        for (name, source) in [
            ("ui.rs", include_str!("ui.rs")),
            ("panels.rs", include_str!("panels.rs")),
            ("graph.rs", include_str!("graph.rs")),
            ("graph/nodes.rs", include_str!("graph/nodes.rs")),
            ("overlay.rs", include_str!("overlay.rs")),
            ("search.rs", include_str!("search.rs")),
            ("status.rs", include_str!("status.rs")),
        ] {
            let body = source.split("\n#[cfg(test)]").next().unwrap_or(source);
            for needle in [
                "app.hot.",
                "app.tree_offset",
                "app.graph_flow",
                "app.overlay =",
            ] {
                assert!(
                    !body.contains(needle),
                    "the render path in {name} still writes `{needle}`"
                );
            }
        }
    }

    #[test]
    fn renders_brand_and_primary_panels_at_wide_size() {
        let output = rendered_text(140, 48);
        assert!(output.contains("NICH LINK"));
        for row in XIRANG_MARK {
            assert!(output.contains(row), "missing or distorted logo row: {row}");
        }
        assert!(output.contains("REGISTRATION TREE"));
        assert!(output.contains("FACE INSPECTOR"));
    }

    #[test]
    fn narrow_terminal_stays_renderable_without_panicking() {
        let output = rendered_text(64, 20);
        assert!(!output.is_empty());
    }

    #[test]
    fn add_form_explains_required_and_derived_values() {
        let mut terminal = Terminal::new(TestBackend::new(140, 48)).unwrap();
        let mut app = App::load_app();
        app.overlay = Some(Overlay::Add(AddState::new(app.registry.id())));
        draw_once(&mut terminal, &mut app).unwrap();
        let output = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();

        assert!(output.contains("module name"));
        assert!(output.contains("* required"));
        assert!(output.contains("FIELD GUIDE"));
        assert!(output.contains("derived, or fixed after creation"));
        assert!(output.contains("<default: NoParts>"));

        // Moving the guide to a machine row reports it read-only, which is the
        // half of the form the author only reviews.
        // 把指南移到机器取值行会报告它为只读，这正是作者只查看的那一半表单。
        if let Some(Overlay::Add(ref mut add)) = app.overlay {
            add.field = face_field::TREE_SLOT;
        }
        draw_once(&mut terminal, &mut app).unwrap();
        let output = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert!(output.contains("DERIVED / READ ONLY"));
        assert!(output.contains("tree slot"));
    }

    #[test]
    fn graft_screen_shows_the_entry_line_the_slot_and_the_plans() {
        let mut terminal = Terminal::new(TestBackend::new(160, 48)).unwrap();
        let mut app = App::load_app();
        app.overlay = Some(Overlay::Graft(super::super::app::GraftState {
            target: crate::run_method::NodeId::from_path(
                "control/object/button/button.rs",
                "Button",
            ),
            target_path: "root/control/button".to_owned(),
            selector: "button_fast".to_owned(),
            full: false,
            field: 0,
            pane: 0,
            editing: false,
            plans: vec![super::super::app::GraftPlanRow {
                selector: "button_fast".to_owned(),
                target_path: "root/control/button".to_owned(),
                full: false,
                error: None,
            }],
            plan_selected: 0,
            pending_delete: None,
            declaration: super::super::app::GraftDeclaration::Declared {
                expression: "cut \"root/control/button\" graft \"button_fast\"".to_owned(),
                line: 48,
                cfg: None,
            },
            flow_declared: true,
            inherited_children: 2,
        }));
        draw_once(&mut terminal, &mut app).unwrap();
        let output = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();

        assert!(output.contains("COMPOSE EXTERNAL GRAFT"));
        assert!(output.contains("DECLARED SLOT"));
        assert!(output.contains("declared at line 48"));
        assert!(output.contains("ENTRY DECLARATION"));
        assert!(output.contains("cut \"root/control/button\" graft \"button_fast\","));
        assert!(output.contains("EXISTING PLANS"));
        assert!(output.contains("2 child face(s) are inherited by the overlay"));
    }

    /// The trace legend must never claim recorded data that is not loaded. This
    /// session has no artifact, so it reads `TRACE: none`; a bare `LIVE` would
    /// advertise a recording that does not exist. The `LIVE` half of the mapping
    /// is pinned where an artifact can be written — `studio::app::tests::trace_ingest`.
    /// 追踪图例绝不能宣称尚未装入的记录数据。本会话没有 artifact，因此读作 `TRACE: none`；
    /// 裸的 `LIVE` 会宣称一份并不存在的记录。映射中 `LIVE` 的那一半钉在能写出 artifact 的地方
    /// ——`studio::app::tests::trace_ingest`。
    #[test]
    fn trace_legend_never_claims_live_data_when_nothing_is_loaded() {
        let output = rendered_text(140, 48);
        assert!(
            output.contains("TRACE: none"),
            "a session with no artifact must say so: {output}"
        );
        assert!(
            !output.contains("LIVE"),
            "a bare `LIVE` claims a real trace that was never loaded: {output}"
        );
    }
}
