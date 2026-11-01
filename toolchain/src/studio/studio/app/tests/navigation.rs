//! covers `app/navigation.rs`, `app/pointer.rs`.
//! 覆盖 `app/navigation.rs`、`app/pointer.rs`。
//!
//! Page routing and divider-drag tests.
//! 页面路由与分隔条拖拽测试。

use super::*;

#[test]
fn workspace_page_shortcuts_route_to_the_expected_mode() {
    let mut app = App::load_app();
    app.handle_key(KeyEvent::from(KeyCode::Char('1')));
    assert_eq!(app.page, StudioPage::Search);
    assert!(matches!(app.overlay, Some(Overlay::Search(_))));

    app.handle_key(KeyEvent::from(KeyCode::Esc));
    assert_eq!(app.page, StudioPage::Inspect);
    assert!(app.overlay.is_none());

    app.handle_key(KeyEvent::from(KeyCode::Char('3')));
    assert_eq!(app.page, StudioPage::Data);
    assert!(matches!(app.overlay, Some(Overlay::Search(ref search)) if search.graph_mode));

    app.handle_key(KeyEvent::from(KeyCode::Esc));
    app.handle_key(KeyEvent::from(KeyCode::Esc));
    // `4` opens the multilingual workspace: a read-only view of this project's
    // `xirang.toml`. Outside such a project the page still opens and says why it is empty.
    // `4` 打开多语言工作区：本项目 `xirang.toml` 的只读视图。不在这样的项目里时页面照样
    // 打开，并说明为什么是空的。
    app.handle_key(KeyEvent::from(KeyCode::Char('4')));
    assert_eq!(app.page, StudioPage::Multilang);
    assert!(app.overlay.is_none());
    assert!(app.multilang.contains("xirang.toml"), "{}", app.multilang);
}

#[test]
fn the_multilang_page_draws_its_label_and_the_inventory() {
    let mut app = App::load_app();
    app.handle_key(KeyEvent::from(KeyCode::Char('4')));
    app.multilang = "consumer  library  cpp  src (2)  tests (1)".to_owned();
    let mut terminal = ratatui::Terminal::new(ratatui::backend::TestBackend::new(120, 40))
        .expect("a test terminal");
    crate::studio::studio::ui::draw_once(&mut terminal, &mut app).expect("the page draws");
    let text: String = terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|cell| cell.symbol())
        .collect();
    assert!(text.contains("MULTILANG"), "{text}");
    assert!(text.contains("consumer  library  cpp"), "{text}");
}

#[test]
fn divider_drag_is_clamped_and_stops_on_release() {
    let mut app = App::load_app();
    app.hot.workspace_area = Rect::new(10, 5, 100, 20);
    app.hot.tree_area = Rect::new(10, 5, 45, 20);

    app.handle_mouse(MouseEventKind::Down(MouseButton::Left), 55, 10);
    app.handle_mouse(MouseEventKind::Drag(MouseButton::Left), 105, 10);
    assert_eq!(app.split_percent, 70);

    app.handle_mouse(MouseEventKind::Up(MouseButton::Left), 105, 10);
    app.handle_mouse(MouseEventKind::Drag(MouseButton::Left), 35, 10);
    assert_eq!(app.split_percent, 70);
}
