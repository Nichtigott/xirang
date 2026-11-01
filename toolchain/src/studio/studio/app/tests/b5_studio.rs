//! Pins for the B5-studio code half: the graph page's arrow vocabulary and its refusal
//! of every other key, the one entry point that opens the editor form, the source stamp
//! living outside navigation, the write guard's honest name, and the stamp/watcher
//! name mirror.
//! B5-studio 代码半的钉子：调用图页的方向键词汇与其对其它键的拒绝、打开编辑表单的唯一入口、
//! 住在导航之外的源码戳、写入守卫的诚实名字，以及戳与 watcher 的名单镜像。
//!
//! covers `app/call_tree_queries.rs`, `app/navigation.rs`, `app/keyboard.rs`,
//! `app/source_stamp.rs`, `app/write_guard.rs` and `bin/xirang-dev.rs`.
//! 覆盖上列文件。

/// Only the four arrows ask for a tree step; every other key is refused, not read as
/// "right".
/// 只有四个方向键请求树步；其他键一律被拒绝，而不会被读成“右”。
#[test]
fn only_the_four_arrows_ask_for_a_tree_step() {
    use super::super::call_tree_queries::TreeStep;
    use crossterm::event::KeyCode;
    assert_eq!(TreeStep::from_key(KeyCode::Up), Some(TreeStep::Up));
    assert_eq!(TreeStep::from_key(KeyCode::Down), Some(TreeStep::Down));
    assert_eq!(TreeStep::from_key(KeyCode::Left), Some(TreeStep::Left));
    assert_eq!(TreeStep::from_key(KeyCode::Right), Some(TreeStep::Right));
    for key in [
        KeyCode::Char('j'),
        KeyCode::Char(' '),
        KeyCode::Enter,
        KeyCode::Tab,
        KeyCode::BackTab,
        KeyCode::Esc,
    ] {
        assert_eq!(
            TreeStep::from_key(key),
            None,
            "{key:?} must be refused instead of read as `Right`"
        );
    }
    // The page's own cursor step uses that mapping and nothing else.
    //
    // **This half is structural, and here is its measured ceiling** (audit `N-6`): a mutation that
    // replaces the guard with `TreeStep::from_key(key).unwrap_or(TreeStep::Right)` **survives every
    // behaviour pin in this file**, because the only thing the tree arm does after the guard is
    // `hop_call_tree`, which returns early when there is no call graph — so with the current
    // fixtures there is no observable difference to assert. A behaviour pin for this arm needs a
    // fixture carrying a real call graph; until then the strongest available check is on the shape,
    // and it is written against **tokens** rather than characters so that a rewrite which only moves
    // whitespace or wraps a call does not silently pass (the audit's own bypass: `M7` escaped once by
    // changing case).
    // 这半是**结构性**的，而它的**实测上限**如下（审计 `N-6`）：把守卫换成
    // `TreeStep::from_key(key).unwrap_or(TreeStep::Right)` 的变异**能存活于本文件里每一条行为钉子**——因为树那一
    // 支在守卫之后只做 `hop_call_tree`，而没有调用图时它立刻返回，因此在现有夹具下**没有被观察的差别**可断言。要
    // 给这一支写行为钉子，需要一个带真实调用图的夹具；在那之前，最强可用的检查就落在形状上，而它按**词法单元**而不是
    // 字符写，好让"只挪空白、只包一层调用"的重写不会静默通过（审计自己举的绕过例子：`M7` 靠改大小写逃过一次）。
    let navigation = super::flattened(include_str!("../navigation.rs"));
    assert!(
        navigation.contains(&super::flattened(
            "let Some(step) = TreeStep::from_key(key) else"
        )),
        "the cursor step must let the mapping decide, with an `else` that refuses"
    );
    for default in [
        "unwrap_or(TreeStep::Right)",
        "unwrap_or_default()",
        "_ => TreeStep::Right",
    ] {
        assert!(
            !navigation.contains(&super::flattened(default)),
            "an unknown key must be refused, not defaulted to `Right` (`{default}`)"
        );
    }
}

/// Every path into the editor form goes through the one function.
/// 进入编辑表单的每条路径都走同一个函数。
#[test]
fn the_editor_form_has_one_entry_point() {
    // The production half only: the file's own tests may press `e`, which is what a
    // test should do — the finding was about the handler synthesising a press.
    // 只看生产半：文件自己的测试可以按 `e`，那正是测试该做的——条目说的是处理器合成按键。
    let keyboard = include_str!("../keyboard.rs");
    let production = keyboard
        .split("\n#[cfg(test)]")
        .next()
        .expect("a production half");
    assert!(
        !super::flattened(production).contains(&super::flattened(
            "handle_key(KeyEvent::from(KeyCode::Char('e')))"
        )),
        "the details-pane Enter must not synthesise a key press"
    );
    assert_eq!(
        production.matches("self.open_edit_form()").count(),
        2,
        "the `e` key and the details-pane Enter share the one call"
    );
    assert_eq!(
        production.matches("fn open_edit_form(&mut self)").count(),
        1
    );
}

/// Navigation no longer touches the stamp: the stamp is its own module, mounted by
/// name.
/// 导航不再触碰戳：戳是自己的模块，按名字挂载。
#[test]
fn the_stamp_lives_outside_navigation() {
    let navigation = include_str!("../navigation.rs");
    for leaked in [
        "fn source_stamp",
        "fn stamp_directory",
        "fn stamp_file",
        "source_stamp()",
        "stamp_directory(",
        "package_root",
    ] {
        assert!(
            !super::flattened(navigation).contains(&super::flattened(leaked)),
            "navigation still reaches for `{leaked}`"
        );
    }
    let stamp = include_str!("../source_stamp.rs");
    assert!(stamp.contains("pub(super) fn source_stamp()"));
    assert!(stamp.contains("pub(super) fn stamp_directory("));
    let app = include_str!("../app.rs");
    assert!(
        app.contains("#[path = \"source_stamp.rs\"]\nmod source_stamp;"),
        "the stamp module is declared by name"
    );
}

/// The write guard is named for what it holds, and the stamp and the dev supervisor
/// watch the same five file names — by mirror, not by import.
/// 写入守卫按它所装的东西命名；戳与开发监督器看的正是同五个文件名——靠镜像而不是 import。
#[test]
fn the_write_guard_is_named_for_what_it_holds_and_the_watch_lists_agree() {
    let guard = include_str!("../write_guard.rs");
    assert!(
        guard.contains("Write guards") && guard.contains("writes themselves are not here"),
        "the renamed page keeps its honest doc"
    );
    let app = include_str!("../app.rs");
    assert!(app.contains("#[path = \"write_guard.rs\"]"));
    assert!(
        !super::flattened(app).contains(&super::flattened("mod writers;")),
        "the old module name is gone"
    );
    let stamp = include_str!("../source_stamp.rs");
    let watcher = include_str!("../../../../bin/xirang-dev.rs");
    for name in [
        "Cargo.toml",
        "Cargo.lock",
        "build.rs",
        "official.lock",
        "user.lock",
    ] {
        let needle = format!("\"{name}\"");
        assert!(
            stamp.contains(&needle),
            "the stamp must watch {name} (audit `STU-S-09`)"
        );
        assert!(
            watcher.contains(&needle),
            "the watcher must watch {name} (audit `STU-S-09`)"
        );
    }
}
