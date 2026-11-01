//! covers `app/call_tree_queries.rs`, `app/graph_queries.rs`.
//! 覆盖 `app/call_tree_queries.rs`、`app/graph_queries.rs`。
//!
//! Spatial call-tree regression tests.
//! 空间调用树回归测试。
//!
//! These run against the node-editor fixture for the same reason the call-graph
//! tests do: the rules under test are about a *shape* — the focus owns level zero,
//! callers and callees fill the levels around it, the cursor moves along the drawn
//! axis — and a shape needs a call graph whose edges are already pinned.
//! 这些测试与调用图测试一样跑在 node-editor 夹具上：被测规则关于一种*形状*——焦点独占第零层、
//! 调用者与被调用者填满它周围的层、游标沿画出的轴移动——而形状需要一个调用边已被钉住的调用图。
//!
//! Mounted only with the `prototype-fixtures` feature, so the default build
//! carries neither these tests nor their fixture selection.
//! 仅在启用 `prototype-fixtures` 特性时挂载，因此默认构建既不携带这些测试，也不携带它们的
//! 夹具选择。

use super::*;

#[cfg(feature = "node-graph")]
use ratatui::Terminal;
#[cfg(feature = "node-graph")]
use ratatui::backend::TestBackend;

/// The registered functions of the loaded project, one per name.
/// 当前载入项目中已注册的函数，每个名字取一个。
fn registered_functions(app: &App, limit: usize) -> Vec<CallRef> {
    let mut functions: Vec<CallRef> = Vec::new();
    for info in app.registry.depth_first() {
        if info.source.function.is_empty()
            || functions
                .iter()
                .any(|existing: &CallRef| same_symbol(&existing.function, &info.source.function))
        {
            continue;
        }
        functions.push(CallRef {
            node: info.id,
            function: info.source.function.clone(),
            file: info.source.file.clone(),
        });
        if functions.len() >= limit {
            break;
        }
    }
    functions
}

/// Open the provenance graph on one registered function and hand back its state.
/// 在某个已注册函数上打开溯源图，并交回它的状态。
fn open_graph(app: &mut App, function: &str) -> SearchState {
    let rows = app.search_rows(function);
    let selected = rows
        .iter()
        .position(|row| row.function == function && row.node.is_some())
        .unwrap_or_else(|| panic!("{function} is not a registered function"));
    app.overlay = Some(Overlay::Search(SearchState {
        query: function.to_owned(),
        selected,
        ..SearchState::default()
    }));
    app.handle_overlay_key(KeyEvent::from(KeyCode::Enter));
    let Some(Overlay::Search(opened)) = app.overlay.as_ref() else {
        panic!("Enter on a call row opens the graph");
    };
    opened.clone()
}

/// The tree around one function of the loaded project.
/// 当前载入项目中某个函数周围的调用树。
fn view_around(app: &App, function: &str) -> CallTreeView {
    let rows = app.search_rows(function);
    let row = rows
        .iter()
        .find(|row| row.function == function && row.node.is_some())
        .unwrap_or_else(|| panic!("{function} is not a registered function"));
    app.call_tree_view(&CallRef {
        node: row.node.expect("the row carries a node"),
        function: row.function.clone(),
        file: row.path.clone(),
    })
}

/// The rendered page as rows of characters.
/// 把渲染出的页面取成一行行字符。
#[cfg(feature = "node-graph")]
fn rendered_rows(terminal: &Terminal<TestBackend>) -> Vec<Vec<char>> {
    let buffer = terminal.backend().buffer();
    let width = buffer.area.width as usize;
    buffer
        .content
        .chunks(width)
        .map(|cells| {
            cells
                .iter()
                .map(|cell| cell.symbol().chars().next().unwrap_or(' '))
                .collect()
        })
        .collect()
}

/// The first column where `key` appears on one line.
/// `key` 在该行上首次出现的列。
#[cfg(feature = "node-graph")]
fn find_key(line: &[char], key: &str) -> Option<usize> {
    let key = key.chars().collect::<Vec<_>>();
    if key.is_empty() || key.len() > line.len() {
        return None;
    }
    (0..=line.len() - key.len()).find(|start| line[*start..*start + key.len()] == key[..])
}

/// The call-tree panel's own lines, cut out by the corners it drew.
/// 调用树面板自身的各行，按它画出的四角裁出。
///
/// Reading the panel out of the page by its borders keeps this test independent
/// of the layout percentages: whatever rectangle the panel gets, the picture is
/// the rectangle it drew.
/// 按边框从页面里裁出面板，使本测试与布局百分比无关：面板拿到哪个矩形，图就是它画出的那个
/// 矩形。
#[cfg(feature = "node-graph")]
fn tree_panel(rows: &[Vec<char>]) -> Option<Vec<Vec<char>>> {
    const TITLE: &str = "CALL TREE · depth";
    let top = rows
        .iter()
        .position(|row| row.iter().collect::<String>().contains(TITLE))?;
    // The page holds two panels side by side, so the corners that belong to this
    // one are the nearest ones around its own title, not the page's edges.
    // 页面上并排两块面板，因此属于本面板的四角是它自己标题两侧最近的那两个，而不是页面边缘。
    let title_at = find_key(&rows[top], TITLE)?;
    let left = rows[top][..title_at]
        .iter()
        .rposition(|cell| *cell == '╭')?;
    let right = title_at + rows[top][title_at..].iter().position(|cell| *cell == '╮')?;
    let mut panel = Vec::new();
    for row in &rows[top..] {
        if row.len() <= right || row.len() <= left {
            return None;
        }
        let line = row[left..=right].to_vec();
        let closed = line.first() == Some(&'╰');
        panel.push(line);
        if closed {
            return Some(panel);
        }
    }
    None
}

#[test]
fn every_shape_the_tree_promises_holds_around_a_real_focus() {
    let Some(app) = fixture_app() else {
        return;
    };
    let functions = registered_functions(&app, 3);
    assert!(
        functions.len() >= 2,
        "the fixture registers several functions to focus on"
    );
    for focus in &functions {
        let view = app.call_tree_view(focus);
        assert_eq!(view.len(), view.refs.len(), "refs and nodes stay aligned");
        assert!(
            same_symbol(&view.tree.nodes[0].symbol, &focus.function),
            "{} owns row zero",
            focus.function
        );
        let mut seen: Vec<(i32, usize)> = Vec::new();
        for (index, node) in view.tree.nodes.iter().enumerate() {
            if index == 0 {
                assert_eq!(node.parent, None, "the focus is placed by nobody");
            } else {
                let parent = &view.tree.nodes[node.parent.expect("a placed node has a parent")];
                assert_eq!(
                    node.level.unsigned_abs(),
                    parent.level.unsigned_abs() + 1,
                    "{} hangs exactly one hop off {}",
                    node.symbol,
                    parent.symbol
                );
                assert!(
                    parent.level == 0 || (parent.level > 0) == (node.level > 0),
                    "{} stays on {}'s side of the focus",
                    node.symbol,
                    parent.symbol
                );
            }
            let cell = (node.level, node.lane);
            assert!(
                !seen.contains(&cell),
                "column {cell:?} holds two nodes, so one would be drawn on the other"
            );
            seen.push(cell);
        }
        for edge in &view.tree.edges {
            let caller = view.tree.nodes[edge.caller].level;
            let callee = view.tree.nodes[edge.callee].level;
            assert_eq!(
                edge.forward,
                callee > caller,
                "forward must mean left to right: {caller} → {callee}"
            );
        }
    }
}

#[test]
fn the_memo_answers_the_same_focus_and_a_different_one() {
    let Some(app) = fixture_app() else {
        return;
    };
    let functions = registered_functions(&app, 2);
    let focus = functions.first().expect("a focus to build around").clone();
    let first = app.call_tree_view(&focus);
    let second = app.call_tree_view(&focus);
    assert_eq!(first.tree, second.tree, "the memo returns the same tree");
    let other = app.call_tree_view(functions.get(1).expect("a second focus"));
    assert!(
        !same_symbol(&other.tree.nodes[0].symbol, &first.tree.nodes[0].symbol),
        "a different focus must not be served from the first focus's entry"
    );
}

/// The arrow keys follow the drawn picture: the one drawer runs the calls down the
/// screen, so ↓ follows a drawn edge and ←/→ move between the lanes of one band.
/// 方向键跟随画出来的图：唯一的绘制方把调用沿屏幕向下排，因此 ↓ 沿一条画出的边走一跳，
/// ←/→ 在一条带的车道间移动。
#[test]
fn the_arrows_follow_the_drawn_grid() {
    let Some(mut app) = fixture_app() else {
        return;
    };
    let mut search = open_graph(&mut app, "paint_node_editor");
    search.graph_focus = 0;
    search.outline_selected = 0;
    app.overlay = Some(Overlay::Search(search));
    let view = view_around(&app, "paint_node_editor");
    let selected = |app: &App| match app.overlay.as_ref() {
        Some(Overlay::Search(state)) => state.outline_selected,
        _ => panic!("the graph stays open while hopping"),
    };
    let node = |index: usize| view.tree.nodes[index].clone();

    app.handle_overlay_key(KeyEvent::from(KeyCode::Down));
    let down = selected(&app);
    assert_eq!(
        node(down).level,
        1,
        "↓ leaves the focus: {}",
        node(down).symbol
    );
    assert_eq!(node(down).parent, Some(0), "↓ follows an edge that exists");
    app.handle_overlay_key(KeyEvent::from(KeyCode::Right));
    let right = selected(&app);
    assert_eq!(
        node(right).level,
        1,
        "→ stays in the band: {}",
        node(right).symbol
    );
    assert!(node(right).lane > node(down).lane, "→ takes the next lane");
    app.handle_overlay_key(KeyEvent::from(KeyCode::Left));
    assert_eq!(selected(&app), down, "← returns to the lane it came from");
    app.handle_overlay_key(KeyEvent::from(KeyCode::Up));
    assert_eq!(selected(&app), 0, "↑ returns to the focus");
}

#[test]
fn enter_re_centres_on_a_tree_node_and_opens_the_editor_only_at_the_focus() {
    let Some(mut app) = fixture_app() else {
        return;
    };
    let mut search = open_graph(&mut app, "preview_canvas_width");
    search.graph_focus = 0;
    search.outline_selected = 1;
    app.overlay = Some(Overlay::Search(search));

    app.handle_overlay_key(KeyEvent::from(KeyCode::Enter));
    let Some(Overlay::Search(recentred)) = app.overlay.as_ref() else {
        panic!("Enter on a tree node re-centres instead of closing the graph");
    };
    assert_eq!(recentred.outline_selected, 0, "the new focus is row zero");
    assert!(
        recentred.center_function.as_deref() != Some("preview_canvas_width"),
        "the graph moved to another function"
    );
    assert!(app.event.contains("re-centred"), "{}", app.event);

    app.handle_overlay_key(KeyEvent::from(KeyCode::Enter));
    assert!(
        app.overlay.is_none(),
        "Enter at the focus itself opens the editor"
    );
}

/// The panel's shared chrome: the heading counts what the model holds, the legend
/// explains the markers, and the status row states the cursor's signature. The
/// widget draws the tree between them; these lines are this page's own and must
/// survive the drawer that replaced the hand-drawn canvas.
/// 面板共享的说明：小标题数出模型内容，图例解释标记，状态行写出游标签名。树由控件画在它们
/// 之间；这几行属于本页，必须在取代手绘画布的绘制方下继续存在。
#[cfg(feature = "node-graph")]
#[test]
fn the_panel_chrome_states_the_models_budget() {
    let Some(mut app) = fixture_app() else {
        return;
    };
    app.graph_split_percent = 80;
    let mut search = open_graph(&mut app, "preview_canvas_width");
    search.graph_focus = 0;
    search.outline_selected = 0;
    app.overlay = Some(Overlay::Search(search));

    let Some(Overlay::Search(search)) = app.overlay.as_ref() else {
        panic!("the graph stays open while the tree is drawn");
    };
    let center = app.graph_item(search).expect("the graph has a centre");
    let view = app.call_tree_view(&center);
    assert!(view.len() > 1, "the fixture's focus has neighbours to draw");

    let mut terminal = Terminal::new(TestBackend::new(200, 50)).expect("a test terminal");
    crate::studio::studio::ui::draw_once(&mut terminal, &mut app).expect("the graph page draws");
    let page = rendered_rows(&terminal);
    let panel = tree_panel(&page).expect("the call tree panel is on screen");
    let panel_text = panel
        .iter()
        .map(|line| line.iter().collect::<String>())
        .collect::<Vec<_>>()
        .join("\n");

    // The title counts the model: a header that lies about its node count is
    // worse than no header.
    // 标题数出模型：一个把节点数说错的表头比没有表头更糟。
    let depth = view
        .tree
        .nodes
        .iter()
        .map(|node| node.level.unsigned_abs())
        .max()
        .unwrap_or(0);
    let title = panel
        .first()
        .expect("the panel has a title")
        .iter()
        .collect::<String>();
    assert!(title.contains(&format!("depth {depth}")), "{title}");
    assert!(title.contains(&format!("{} nodes", view.len())), "{title}");
    assert_eq!(
        title.contains("truncated"),
        view.tree.truncated,
        "the header must say so exactly when the model cut something: {title}"
    );

    // The legend lives on the bottom border and explains the markers the widget
    // writes into the nodes and edges.
    // 图例位于底边，解释控件写进节点与边的那些标记。
    let legend = panel
        .last()
        .expect("the panel has a bottom border")
        .iter()
        .collect::<String>();
    assert!(
        legend.contains("+ live"),
        "the legend explains the markers: {legend}"
    );

    // The status row is the cursor's signature; it is the panel's last inner row,
    // directly above the bottom border.
    // 状态行是游标的签名；它是面板最后一行内部行，紧贴底边之上。
    let status = panel[panel.len() - 2].iter().collect::<String>();
    assert!(
        status.contains("ƒ "),
        "the status row is drawn:\n{panel_text}"
    );
    assert!(
        status.contains(&center.function),
        "the status row states the cursor's signature: {status}"
    );
}

/// Clicking a node selects it: the widget reports which node the pointer landed
/// on and the page turns that into the cursor. The hand-drawn canvas used to
/// publish its rectangles for this; the widget owns the hit test now.
/// 点击节点即选中它：控件报告指针落在哪个节点上，本页把它变成游标。过去由手绘画布公布矩形来
/// 做命中测试，现在由控件负责。
#[cfg(feature = "node-graph")]
#[test]
fn a_click_lands_on_the_node_under_the_pointer() {
    let Some(mut app) = fixture_app() else {
        return;
    };
    app.graph_split_percent = 80;
    let mut search = open_graph(&mut app, "preview_canvas_width");
    search.graph_focus = 0;
    search.outline_selected = 0;
    app.overlay = Some(Overlay::Search(search));
    let mut terminal = Terminal::new(TestBackend::new(200, 50)).expect("a test terminal");
    crate::studio::studio::ui::draw_once(&mut terminal, &mut app).expect("the graph page draws");
    // A node besides the focus, and where the widget drew it on screen: the click
    // must land inside the tree pane, because that is where the handler forwards
    // pointer events to the widget.
    // 焦点之外的一个节点，以及控件把它画在屏幕上的位置：点击必须落在树面板内，因为处理函数正是
    // 在那里把指针事件转给控件。
    let (index, x, y) = {
        let Some((_, _, flow)) = app.graph_flow.as_ref() else {
            panic!("the widget drew the tree");
        };
        let Some(Overlay::Search(search)) = app.overlay.as_ref() else {
            panic!("the graph stays open while the tree is drawn");
        };
        let center = app.graph_item(search).expect("the graph has a centre");
        let len = app.call_tree_view(&center).len();
        (1..len)
            .find_map(|index| {
                let (left, top, right, bottom) = flow.node_terminal_rect(&index.to_string())?;
                let (x, y) = (((left + right) / 2) as u16, ((top + bottom) / 2) as u16);
                app.hot
                    .graph_tree_area
                    .contains((x, y).into())
                    .then_some((index, x, y))
            })
            .expect("the fixture draws a node besides the focus inside the tree pane")
    };
    // The widget reports a click on release, so both halves of the gesture have to
    // reach it.
    // 控件在松开时报告点击，因此手势的两半都必须到达它。
    app.handle_mouse(MouseEventKind::Down(MouseButton::Left), x, y);
    app.handle_mouse(MouseEventKind::Up(MouseButton::Left), x, y);
    let Some(Overlay::Search(search)) = app.overlay.as_ref() else {
        panic!("the graph stays open after a click inside it");
    };
    assert_eq!(
        search.outline_selected, index,
        "the click should select the node it landed on"
    );
    assert_eq!(search.graph_focus, 0, "clicking the tree focuses it");
}

/// A panel too narrow for a neighbour still draws, and the split keys it names
/// widen the tree, clamped like the divider drag.
/// 窄到放不下邻列的面板仍然会绘制，而它点名的分栏按键确实能把树加宽，且与分隔线拖动使用同一
/// 范围。
#[test]
fn a_narrow_panel_names_the_keys_that_widen_it() {
    let Some(mut app) = fixture_app() else {
        return;
    };
    let mut search = open_graph(&mut app, "preview_canvas_width");
    search.graph_focus = 0;
    app.overlay = Some(Overlay::Search(search));
    // The page gives the tree a share of its width, so the narrow case is asked
    // for explicitly.
    // 页面把宽度分一份给树，因此窄的情况要显式要求。
    app.graph_split_percent = 35;
    let mut terminal = Terminal::new(TestBackend::new(44, 24)).expect("a test terminal");
    crate::studio::studio::ui::draw_once(&mut terminal, &mut app).expect("the graph page draws");
    let page = rendered_rows(&terminal)
        .iter()
        .map(|row| row.iter().collect::<String>())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        page.contains("CALL TREE"),
        "the panel is drawn even when the pane is narrow:\n{page}"
    );

    // The keys are named in the page footer, which needs a page wide enough to
    // hold the sentence: assert that where it is readable.
    // 这些按键写在页面页脚里，而页脚需要足够宽的页面才放得下整句：因此在那读得清的地方断言。
    let Some(mut wide) = fixture_app() else {
        return;
    };
    let mut search = open_graph(&mut wide, "preview_canvas_width");
    search.graph_focus = 0;
    wide.overlay = Some(Overlay::Search(search));
    let mut terminal = Terminal::new(TestBackend::new(200, 40)).expect("a test terminal");
    crate::studio::studio::ui::draw_once(&mut terminal, &mut wide).expect("the graph page draws");
    let footer = rendered_rows(&terminal)
        .iter()
        .map(|row| row.iter().collect::<String>())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        footer.contains("[ ] split"),
        "the footer must name the split keys:\n{footer}"
    );
    assert!(
        !footer.contains("drawer"),
        "the footer must not name the removed drawer key:\n{footer}"
    );

    assert_eq!(
        app.graph_split_percent, 35,
        "the test asked for a narrow tree"
    );
    app.handle_overlay_key(KeyEvent::from(KeyCode::Char(']')));
    assert!(app.graph_split_percent > 35, "] gives the tree more room");
    app.handle_overlay_key(KeyEvent::from(KeyCode::Char('[')));
    assert_eq!(app.graph_split_percent, 35, "[ takes it back");
    for _ in 0..40 {
        app.handle_overlay_key(KeyEvent::from(KeyCode::Char(']')));
    }
    assert_eq!(
        app.graph_split_percent, 65,
        "the key stops where the divider drag stops"
    );
}

/// The one drawer shows the fixture's tree: the same three nodes and the same two
/// hops must be on screen. This is what fails if `draw_call_tree` stops rendering
/// the widget's graph.
/// 唯一的绘制方展示夹具的树：同样的三个节点与两跳必须出现在屏幕上。若 `draw_call_tree` 不再
/// 渲染控件的图，本测试就会失败。
#[cfg(feature = "node-graph")]
#[test]
fn the_widget_draws_the_same_tree() {
    let Some(mut app) = fixture_app() else {
        return;
    };
    let mut search = open_graph(&mut app, "preview_canvas_width");
    search.graph_focus = 0;
    app.overlay = Some(Overlay::Search(search));
    let mut terminal = Terminal::new(TestBackend::new(200, 40)).expect("a test terminal");
    crate::studio::studio::ui::draw_once(&mut terminal, &mut app).expect("the graph page draws");
    let page = rendered_rows(&terminal);
    let panel = tree_panel(&page).expect("the call tree panel is on screen");
    let text = panel
        .iter()
        .map(|line| line.iter().collect::<String>())
        .collect::<Vec<_>>()
        .join("\n");
    for name in [
        "preview_canvas_width",
        "preview_canvas_width_traced",
        "clamp_canvas_width",
    ] {
        assert!(text.contains(name), "missing {name}:\n{text}");
    }
    assert!(
        text.contains('▶') || text.contains('╰') || text.contains('─'),
        "the widget must route the edges:\n{text}"
    );
}

/// Moving the tree cursor is *visible* in the widget: the mark travels with it,
/// which is what tells the reader that ↑↓ did something. This is the regression
/// the reader reported — a selection that only changed a colour was no feedback at
/// all in a terminal without one.
/// 控件里移动树游标是**看得见**的：标记跟着它走，这正是告诉读者 ↑↓ 起了作用的东西。这正是
/// 读者报告的回归——只改颜色的选中项，在没有颜色的终端里等于没有反馈。
#[cfg(feature = "node-graph")]
#[test]
fn the_widget_marks_the_cursor_where_it_is() {
    let Some(mut app) = fixture_app() else {
        return;
    };
    app.graph_split_percent = 80;
    let mut search = open_graph(&mut app, "preview_canvas_width");
    search.graph_focus = 0;
    app.overlay = Some(Overlay::Search(search));
    let mut terminal = Terminal::new(TestBackend::new(170, 40)).expect("a test terminal");

    let marked = |terminal: &Terminal<TestBackend>| -> String {
        rendered_rows(terminal)
            .iter()
            .map(|row| row.iter().collect::<String>())
            .find(|row| row.contains("▶ "))
            .unwrap_or_default()
    };

    crate::studio::studio::ui::draw_once(&mut terminal, &mut app).expect("the graph page draws");
    let first = marked(&terminal);
    assert!(
        first.contains("preview_canvas_width"),
        "the focus starts under the cursor:\n{first}"
    );
    assert!(
        first.contains("▶"),
        "the cursor carries a mark that survives a colourless terminal"
    );

    app.handle_overlay_key(KeyEvent::from(KeyCode::Down));
    crate::studio::studio::ui::draw_once(&mut terminal, &mut app).expect("the graph page draws");
    let second = marked(&terminal);
    assert_ne!(first, second, "the mark must move with the cursor");
    assert!(
        second.contains('▶'),
        "and it must still be a mark:\n{second}"
    );
}

/// The keys that used to switch or rotate the drawer are gone: `g` and `v` on the
/// graph page redraw the exact same panel, so the one drawer cannot be swapped or
/// rotated by muscle memory.
/// 过去用来切换或旋转绘制方的按键已删除：调用图页上的 `g` 与 `v` 会重绘出完全相同的面板，
/// 因此唯一的绘制方不会被肌肉记忆换掉或转掉。
#[cfg(feature = "node-graph")]
#[test]
fn the_removed_drawer_keys_change_nothing() {
    let Some(mut app) = fixture_app() else {
        return;
    };
    app.graph_split_percent = 80;
    let mut search = open_graph(&mut app, "preview_canvas_width");
    search.graph_focus = 0;
    app.overlay = Some(Overlay::Search(search));
    let mut terminal = Terminal::new(TestBackend::new(200, 50)).expect("a test terminal");
    let page = |terminal: &mut Terminal<TestBackend>, app: &mut App| {
        crate::studio::studio::ui::draw_once(terminal, app).expect("the graph page draws");
        rendered_rows(terminal)
            .iter()
            .map(|row| row.iter().collect::<String>())
            .collect::<Vec<_>>()
            .join("\n")
    };
    let before = page(&mut terminal, &mut app);
    let event_before = app.event.clone();

    for key in ['g', 'v'] {
        app.handle_overlay_key(KeyEvent::from(KeyCode::Char(key)));
    }

    let after = page(&mut terminal, &mut app);
    assert_eq!(
        before, after,
        "g and v must not swap or rotate the drawer:\n{after}"
    );
    assert_eq!(
        app.event, event_before,
        "no key claims to have done something"
    );
    let Some(Overlay::Search(search)) = app.overlay.as_ref() else {
        panic!("the graph stays open");
    };
    assert_eq!(
        search.query, "preview_canvas_width",
        "the letters must not edit the query either"
    );
}

/// The graph page's letters are commands, and typing is not one of them: the query
/// is edited in the list page, which `/` returns to. The catch-all typing arm used
/// to sit in front of `e`, so that key never fired — a conflict rustc cannot see,
/// because the arm it shadowed carries a guard.
/// 调用图页的字母是命令，"输入"不是其中之一：查询在列表页编辑，`/` 回到那里。兜底的输入分支
/// 曾经排在 `e` 前面，于是那个键永远不会触发——这是 rustc 看不见的冲突，因为它遮蔽的那条 arm
/// 带着守卫。
#[test]
fn the_graph_pages_letters_are_commands() {
    let Some(mut app) = fixture_app() else {
        return;
    };
    let mut search = open_graph(&mut app, "preview_canvas_width");
    search.graph_focus = 0;
    app.overlay = Some(Overlay::Search(search));

    app.handle_overlay_key(KeyEvent::from(KeyCode::Char('z')));
    let Some(Overlay::Search(search)) = app.overlay.as_ref() else {
        panic!("the graph stays open");
    };
    assert_eq!(
        search.query, "preview_canvas_width",
        "a letter must not edit the query on this page"
    );

    app.handle_overlay_key(KeyEvent::from(KeyCode::Char('e')));
    assert!(
        app.overlay.is_none(),
        "`e` opens the cursor's source instead of being swallowed"
    );
    let Some((path, line)) = app.take_editor_request() else {
        panic!("`e` must request an editor");
    };
    assert!(path.is_file(), "{}", path.display());
    assert!(line > 1, "the request names a line");
}
