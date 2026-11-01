//! covers `app/keyboard.rs`, `app/state/forms.rs`, `app/state/plugin_field.rs`.
//! 覆盖 `app/keyboard.rs`、`app/state/forms.rs`、`app/state/plugin_field.rs`。
//!
//! Add/edit form and registration-face field tests.
//! 新增/编辑表单与注册面字段测试。

use super::super::plugin_field;
use super::*;
use crate::run_method::face_field;

#[test]
fn add_overlay_click_selects_a_field_and_toggles_the_checkbox() {
    let mut app = App::load_app();
    app.overlay = Some(Overlay::Add(AddState::new(app.selected_parent())));
    app.hot.overlay_area = Rect::new(5, 5, 70, 18);
    app.hot.overlay_list_area = Rect::new(7, 7, 66, 10);

    app.handle_overlay_click(12, 16);

    let Some(Overlay::Add(add)) = app.overlay else {
        panic!("add overlay should remain open");
    };
    assert_eq!(add.field, 2);
    assert_eq!(add.values[face_field::NEEDS_REGISTRY], "true");
    assert!(!add.editing);
}

#[test]
fn add_overlay_keyboard_navigation_reaches_kind_field() {
    let mut app = App::load_app();
    app.overlay = Some(Overlay::Add(AddState::new(app.selected_parent())));
    for _ in 0..2 {
        app.handle_overlay_key(crossterm::event::KeyEvent::from(
            crossterm::event::KeyCode::Down,
        ));
    }

    let Some(Overlay::Add(add)) = app.overlay else {
        panic!("add overlay should remain open");
    };
    assert_eq!(add.field, face_field::KIND);
}

#[test]
fn add_navigation_walks_one_fixed_field_order() {
    // The layout used to be rebuilt from the focused field, which is why this
    // test watched for the visible set changing. It is a constant now, so the
    // claim worth pinning is the other direction: every Down press lands on the
    // next entry of that one order, and no slot is skipped or repeated.
    // 布局过去会按聚焦字段重建，这正是本测试当初盯住可见集合变化的原因。现在它是常量，
    // 因此值得钉住的是另一个方向：每次 Down 落在该顺序的下一个条目上，没有槽位被跳过或重复。
    let mut app = App::load_app();
    app.overlay = Some(Overlay::Add(AddState::new(app.selected_parent())));
    let order = super::super::face_field_indices();
    let mut visited = Vec::new();
    for step in 0..9 {
        if step > 0 {
            app.handle_overlay_key(KeyEvent::from(KeyCode::Down));
        }
        let Some(Overlay::Add(current)) = app.overlay.as_ref() else {
            panic!("add overlay should remain open");
        };
        visited.push(current.field);
    }
    assert_eq!(visited, order[..9], "navigation skipped a form row");
    assert_eq!(order.len(), crate::run_method::FACE_FIELD_COUNT);
}

#[test]
fn add_form_starts_with_editable_bilingual_summary() {
    let add = AddState::new(crate::run_method::ROOT_NODE_ID);
    assert!(add.values[face_field::NAME_ZH].is_empty());
    assert!(add.values[face_field::NAME_EN].is_empty());
    assert!(add.values[face_field::SUMMARY_ZH].is_empty());
    assert!(add.values[face_field::SUMMARY_EN].is_empty());
    assert!(add.values[face_field::PART_CONTRACTS].is_empty());
}

#[test]
fn parent_rule_marks_the_fields_a_child_must_supply() {
    let mut add = AddState::new(crate::run_method::ROOT_NODE_ID);
    add.apply_parent_rule(&crate::run_method::OwnedRegistrationRule {
        required_preset: Some("ActionParts".to_owned()),
        required_parts: vec!["paint".to_owned()],
        required_exports: vec!["control.render".to_owned()],
        required_handle_traits: vec!["ControlHandle".to_owned()],
        required_part_traits: vec!["ActionPartsContract".to_owned()],
    });

    assert_eq!(
        add.parent_requirements.keys().copied().collect::<Vec<_>>(),
        [
            face_field::PARTS,
            face_field::EXPORTS,
            face_field::PRESET,
            face_field::HANDLE_CONTRACTS,
            face_field::PART_CONTRACTS,
        ]
    );
    assert_eq!(
        add.parent_requirements
            .get(&face_field::HANDLE_CONTRACTS)
            .map(String::as_str),
        Some("handle implements ControlHandle")
    );
    assert_eq!(add.values[face_field::PRESET], "ActionParts");
    assert_eq!(add.values[face_field::EXPORTS], "control.render");
}

#[test]
fn edit_recovers_compiler_checked_contract_paths_from_source() {
    let source = r#"crate::root_object! {
        kind: Button,
        handle_contracts: [crate::ui::ControlHandle],
        part_contracts: [crate::ui::ActionPartsContract],
    }"#;

    assert_eq!(
        super::super::keyboard::declaration_contract_paths(source),
        (
            "crate::ui::ControlHandle".to_owned(),
            "crate::ui::ActionPartsContract".to_owned(),
        )
    );
}

/// A source file that cannot be read is reported loudly, and an empty result keeps
/// meaning "no contracts": the two used to be the same value (audit `LGC-LG-34`).
/// 读不到的源文件要被响亮地报出，而空结果仍旧只表示“没有契约”：两者曾经是同一个值
/// （审计 `LGC-LG-34`）。
#[test]
fn an_unreadable_source_is_loud_where_an_empty_one_is_quiet() {
    let missing = std::path::Path::new("toolchain/src/studio/studio/app/definitely-not-here.rs");
    let (handle, part, failure) = super::super::keyboard::declaration_contract_fields(missing);
    assert!(
        failure.is_some(),
        "a read failure must be reported, not rendered as `no contracts`"
    );
    assert!(
        handle.contains("unreadable") && part.contains("unreadable"),
        "the columns themselves must say they were never read: {handle:?} / {part:?}"
    );

    let dir = std::env::temp_dir().join("xirang-toolchain-empty-contracts");
    std::fs::create_dir_all(&dir).expect("a temp dir");
    let empty = dir.join("no-contracts.rs");
    std::fs::write(&empty, "// a source file with no face at all\n").expect("write");
    let (handle, part, failure) = super::super::keyboard::declaration_contract_fields(&empty);
    assert!(
        failure.is_none() && handle.is_empty() && part.is_empty(),
        "a readable file with no contracts stays quiet and empty"
    );
}

#[test]
fn stable_identity_is_editable_from_the_form() {
    let mut app = App::load_app();
    let mut add = AddState::new(app.selected_parent());
    add.field = face_field::STABLE_NAME;
    app.overlay = Some(Overlay::Add(add));

    app.handle_overlay_key(KeyEvent::from(KeyCode::Enter));

    assert!(matches!(app.overlay, Some(Overlay::Add(ref add)) if add.editing));
}

#[test]
fn derived_storage_fields_do_not_open_a_fake_editor() {
    let mut app = App::load_app();
    let mut add = AddState::new(app.selected_parent());
    add.field = face_field::REGISTRY_RULE_PATH;
    app.overlay = Some(Overlay::Add(add));

    app.handle_overlay_key(KeyEvent::from(KeyCode::Enter));

    assert!(matches!(app.overlay, Some(Overlay::Add(ref add)) if !add.editing));
}

/// Selecting a plugin writes two files, so it takes two presses, and a failure on
/// the second file puts the first one back: a half-written pair would leave the
/// entry importing a crate the lock does not record.
/// 选择插件会写两个文件，因此需要两次按键；而第二个文件失败时会把第一个恢复：写了一半会让
/// 入口导入一个锁里没有记录的 crate。
#[test]
fn a_plugin_selection_takes_two_presses_and_rolls_back_a_half_write() {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let root = std::env::temp_dir().join(format!("xirang-toolchain-plugin-{suffix}"));
    let plugins = root.join(".xirang/plugins");
    std::fs::create_dir_all(&plugins).expect("plugin directory");
    let entry = plugins.join("user.rs");
    let lock = plugins.join("user.lock");
    select_project(root.clone(), root.join("Cargo.toml"), "plugin-app");

    let mut app = App::load_app();
    app.handle_key(KeyEvent::from(KeyCode::Char('p')));
    let Some(Overlay::Plugin(mut plugin)) = app.overlay.take() else {
        panic!("p should open the Plugin form");
    };
    plugin.values[plugin_field::SOURCE] = "user".to_owned();
    plugin.values[plugin_field::FRAMEWORK] = "xirang.test".to_owned();
    plugin.values[plugin_field::PACKAGE] = "demo-plugin".to_owned();
    plugin.values[plugin_field::VERSION] = "0.1.0".to_owned();
    plugin.values[plugin_field::CRATE] = "demo_plugin".to_owned();
    plugin.values[plugin_field::CHECKSUM] = "sha256:00".to_owned();
    plugin.values[plugin_field::MODE] = "extension".to_owned();
    app.overlay = Some(Overlay::Plugin(plugin));

    // The first press only arms, and an unrelated key clears the arm.
    app.handle_overlay_key(KeyEvent::from(KeyCode::Char('s')));
    assert!(app.event.contains("press s again"), "{}", app.event);
    assert!(
        !entry.exists() && !lock.exists(),
        "one press must not write"
    );
    app.handle_overlay_key(KeyEvent::from(KeyCode::Down));
    app.handle_overlay_key(KeyEvent::from(KeyCode::Char('s')));
    assert!(
        !entry.exists() && !lock.exists(),
        "another key clears the arm, so this press only arms again"
    );

    // The lock is a directory, so the second write fails after the first
    // succeeded, and the entry file must not survive it.
    // 锁的位置是个目录，因此第二个写入在第一个成功之后失败，而入口文件绝不能留下来。
    std::fs::create_dir_all(&lock).expect("a directory where the lock belongs");
    app.handle_overlay_key(KeyEvent::from(KeyCode::Char('s')));
    assert!(
        app.event.contains("cannot update user.lock"),
        "the failure names the file: {}",
        app.event
    );
    assert!(
        app.event.contains("restored"),
        "the report says what happened to the other file: {}",
        app.event
    );
    assert!(
        !entry.exists(),
        "the entry line must not survive a failed lock write: {}",
        app.event
    );

    let _ = std::fs::remove_dir_all(&root);
}

/// A paste is typing into the open form, and nothing at all when no form is open.
/// 粘贴就是往打开的表单里键入；没有表单打开时什么都不做。
#[test]
fn a_paste_types_into_the_open_form_and_is_ignored_without_one() {
    use crossterm::event::Event;
    // No overlay: a pasted `ab` must not be replayed as the base key map (`a` opens the
    // add form, `b` starts a build).
    // 没有浮层：粘进来的 `ab` 不得被当作基础键位回放（`a` 打开添加表单、`b` 触发构建）。
    let mut app = App::load_app();
    app.handle_app(Event::Paste("ab".to_owned()));
    assert!(
        app.overlay.is_none(),
        "a paste with no form open must not open one: {:?}",
        app.event
    );
    assert!(!app.should_quit, "and it must not be read as `q`");

    // With the add form open and a field accepting input, the paste lands in that field.
    // 添加表单打开且某字段正在接受输入时，粘贴落进那个字段。
    let mut add = AddState::new(app.registry.id());
    add.field = face_field::NAME_EN;
    add.editing = true;
    app.overlay = Some(Overlay::Add(add));
    app.handle_app(Event::Paste("Button".to_owned()));
    let Some(Overlay::Add(add)) = app.overlay else {
        panic!("the form stays open across a paste")
    };
    assert_eq!(add.values[face_field::NAME_EN], "Button");
}
