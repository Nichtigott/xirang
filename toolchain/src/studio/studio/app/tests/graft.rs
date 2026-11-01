//! covers `app/graft.rs`, `app/mutations.rs`.
//! 覆盖 `app/graft.rs`、`app/mutations.rs`。
//!
//! External graft composition regression tests.
//! 外部 graft 组合回归测试。

use super::*;
use crate::run_method::face_field;
use crate::run_method::registry_core::declaration::portable_path;

/// The graft screen composes a plan, shows the entry line, and never edits host
/// source. A plan is a record; the overlay itself is applied by the host.
/// graft 界面撰写计划、显示入口那一行，并且永不改动宿主源码。计划是记录，覆盖
/// 本身由宿主应用。
#[test]
fn graft_composes_an_external_overlay_plan_without_touching_source() {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let root = std::env::temp_dir().join(format!("xirang-toolchain-external-graft-{suffix}"));
    std::fs::create_dir_all(root.join("src")).expect("create source root");
    select_project(root.clone(), root.join("Cargo.toml"), "external-graft-test");

    let mut app = App::load_app();
    let mut add = AddState::new(app.registry.id());
    add.values[face_field::MODULE] = "canvas".to_owned();
    add.values[face_field::KIND] = "Canvas".to_owned();
    app.submit_add(&add);
    let target = app
        .registry
        .depth_first()
        .into_iter()
        .find(|face| face.registry_name == "canvas")
        .expect("canvas face")
        .id;
    app.selected = target;
    let source = root.join("src").join(
        app.registry
            .find_registry(target)
            .expect("target metadata")
            .source
            .file
            .as_str(),
    );
    let before = std::fs::read(&source).expect("source exists");

    // `g` opens the screen instead of writing a file nobody reads.
    app.handle_key(KeyEvent::from(KeyCode::Char('g')));
    let Some(Overlay::Graft(state)) = app.overlay.clone() else {
        panic!("g opens the graft screen: {}", app.event);
    };
    assert_eq!(state.target_path, "root/canvas");
    assert_eq!(state.selector, "canvas_graft");
    assert!(!state.full);
    assert!(state.plans.is_empty());
    assert_eq!(state.inherited_children, 0);
    // This fixture has no host entry yet, and the screen says so.
    assert!(
        matches!(state.declaration, GraftDeclaration::Unknown { .. }),
        "{:?}",
        state.declaration
    );

    // Choose the subtree scope, then write the plan.
    app.handle_overlay_key(KeyEvent::from(KeyCode::Down));
    app.handle_overlay_key(KeyEvent::from(KeyCode::Enter));
    app.handle_overlay_key(KeyEvent::from(KeyCode::Char('s')));
    assert!(
        app.event.contains("External graft plan created"),
        "{}",
        app.event
    );
    assert!(
        app.event
            .contains("cut \"root/canvas\" full graft \"canvas_graft\","),
        "the event hands over the entry line: {}",
        app.event
    );
    let (plan_path, line) = app
        .take_editor_request()
        .expect("external plan editor request");
    assert!(plan_path.ends_with("external-grafts/canvas_graft/graft.plan"));
    assert_eq!(line, 1);
    let plan_text = std::fs::read_to_string(&plan_path).expect("plan written");
    assert!(
        plan_text.contains("target_path=root/canvas\n"),
        "{plan_text}"
    );
    assert!(plan_text.contains("full=true\n"), "{plan_text}");
    assert_eq!(std::fs::read(&source).expect("source remains"), before);

    // The screen lists the record it just wrote.
    let Some(Overlay::Graft(state)) = app.overlay.clone() else {
        panic!("the graft screen stays open");
    };
    assert_eq!(state.plans.len(), 1);
    assert_eq!(state.plans[0].selector, "canvas_graft");
    assert!(state.plans[0].full);

    // `f` rewrites the record; `d` arms and only a second `d` moves it to the
    // recoverable trash: moving a directory deserves two deliberate presses, and
    // any other key in between clears the arm.
    // `f` 重写记录；`d` 只进入待删状态，只有第二次 `d` 才把它移进可恢复的回收目录：移动
    // 目录值得两次有意的按键，而中间任何其他键都会解除。
    app.handle_overlay_key(KeyEvent::from(KeyCode::Char('f')));
    assert!(
        !with_authoring_context(|| crate::run_method::read_external_graft("canvas_graft"))
            .expect("plan reads back")
            .full()
    );
    app.handle_overlay_key(KeyEvent::from(KeyCode::Char('d')));
    assert!(app.event.contains("press d again"), "{}", app.event);
    assert!(
        with_authoring_context(|| crate::run_method::read_external_graft("canvas_graft")).is_ok(),
        "one press must not delete the record"
    );
    app.handle_overlay_key(KeyEvent::from(KeyCode::Down));
    app.handle_overlay_key(KeyEvent::from(KeyCode::Char('d')));
    assert!(
        with_authoring_context(|| crate::run_method::read_external_graft("canvas_graft")).is_ok(),
        "another key clears the arm, so this press only arms again"
    );
    app.handle_overlay_key(KeyEvent::from(KeyCode::Char('d')));
    assert!(
        with_authoring_context(|| crate::run_method::read_external_graft("canvas_graft")).is_err()
    );
    assert!(app.event.contains("moved to"), "{}", app.event);
    assert_eq!(std::fs::read(&source).expect("source remains"), before);

    let _ = std::fs::remove_dir_all(&root);
}

/// Writing over an existing selector is refused; the screen points at it.
/// 覆盖已存在的选择器会被拒绝；界面指向那一条计划。
#[test]
fn graft_refuses_a_selector_that_already_exists() {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let root = std::env::temp_dir().join(format!("xirang-toolchain-graft-collision-{suffix}"));
    std::fs::create_dir_all(root.join("src")).expect("create source root");
    select_project(
        root.clone(),
        root.join("Cargo.toml"),
        "external-graft-collision",
    );

    let mut app = App::load_app();
    let mut add = AddState::new(app.registry.id());
    add.values[face_field::MODULE] = "canvas".to_owned();
    add.values[face_field::KIND] = "Canvas".to_owned();
    app.submit_add(&add);
    app.selected = app
        .registry
        .depth_first()
        .into_iter()
        .find(|face| face.registry_name == "canvas")
        .expect("canvas face")
        .id;

    // Write one plan, then ask for a different scope under the same selector.
    app.handle_key(KeyEvent::from(KeyCode::Char('g')));
    app.handle_overlay_key(KeyEvent::from(KeyCode::Char('s')));
    app.handle_overlay_key(KeyEvent::from(KeyCode::Down));
    app.handle_overlay_key(KeyEvent::from(KeyCode::Enter));
    app.handle_overlay_key(KeyEvent::from(KeyCode::Char('s')));
    assert!(
        app.event.contains("already exists"),
        "a collision names the existing plan: {}",
        app.event
    );
    assert!(
        !with_authoring_context(|| crate::run_method::read_external_graft("canvas_graft"))
            .expect("the original plan is untouched")
            .full()
    );

    let _ = std::fs::remove_dir_all(&root);
}

/// An undeclared slot still gets its plan written — "write the record, then
/// declare the slot" is a legitimate order — but the banner is a warning that
/// names the pruned path, the skipped record, and the exact clause to add.
/// 未声明的槽位仍然会被写入计划——“先写记录、再声明槽位”是合法的顺序——但横幅是一条
/// 警告，报出被剪掉的路径、被跳过的记录，以及要补的那条子句。
#[test]
fn graft_warns_about_an_undeclared_slot_after_writing_the_plan() {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let root = std::env::temp_dir().join(format!("xirang-toolchain-graft-undeclared-{suffix}"));
    std::fs::create_dir_all(root.join("src")).expect("create source root");
    select_project(
        root.clone(),
        root.join("Cargo.toml"),
        "external-graft-undeclared",
    );

    let mut app = App::load_app();
    let mut add = AddState::new(app.registry.id());
    add.values[face_field::MODULE] = "canvas".to_owned();
    add.values[face_field::KIND] = "Canvas".to_owned();
    app.submit_add(&add);
    app.selected = app
        .registry
        .depth_first()
        .into_iter()
        .find(|face| face.registry_name == "canvas")
        .expect("canvas face")
        .id;
    // An entry that exists but declares no slot is `Absent`, not `Unknown`: the
    // difference decides whether the warning may say "the release prunes it".
    // 存在但不声明任何槽位的入口是 `Absent` 而不是 `Unknown`：这一区别决定了警告能否
    // 说“发布态会剪掉它”。
    std::fs::write(root.join("src/lib.rs"), "crate::host!();\n").expect("host entry");

    app.handle_key(KeyEvent::from(KeyCode::Char('g')));
    let Some(Overlay::Graft(state)) = app.overlay.clone() else {
        panic!("g opens the graft screen");
    };
    assert!(
        matches!(state.declaration, GraftDeclaration::Absent { .. }),
        "{:?}",
        state.declaration
    );

    app.handle_overlay_key(KeyEvent::from(KeyCode::Char('s')));
    let plan = with_authoring_context(|| crate::run_method::read_external_graft("canvas_graft"))
        .expect("the plan is written even though the slot is undeclared");
    assert!(app.event.starts_with("Warning:"), "{}", app.event);
    // The wording is for the reader; the colour comes from the field the write set
    // (audit `STU-S-18`).
    // 措辞是给读者看的；颜色来自写入时设定的字段（审计 `STU-S-18`）。
    assert!(
        app.event_is_alert,
        "an undeclared slot is an alert, not only a sentence starting with `Warning:`"
    );
    assert!(app.event.contains("root/canvas"), "{}", app.event);
    assert!(app.event.contains("UnkeptSlot"), "{}", app.event);
    // The entry is a host path, so Windows spells it `...\src\lib.rs`; the
    // warning has to name that file however the platform spells it.
    // 入口是宿主路径，因此 Windows 写成 `...\src\lib.rs`；无论平台怎么拼，警告都必须
    // 报出那个文件。
    assert!(
        portable_path(&app.event).contains("src/lib.rs"),
        "the warning names the entry that lacks the declaration: {}",
        app.event
    );
    assert_eq!(
        app.event.lines().last().expect("clause line"),
        plan.document.declaration(),
        "the warning hands over exactly the clause the record produces"
    );

    let _ = std::fs::remove_dir_all(&root);
}

/// The screen reads the slot out of the host entry instead of guessing.
/// 界面从宿主入口读出槽位声明，而不是猜。
#[test]
fn graft_reads_the_declared_slot_from_the_host_entry() {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let root = std::env::temp_dir().join(format!("xirang-toolchain-graft-declared-{suffix}"));
    std::fs::create_dir_all(root.join("src")).expect("create source root");
    select_project(
        root.clone(),
        root.join("Cargo.toml"),
        "external-graft-declared",
    );

    let mut app = App::load_app();
    let mut add = AddState::new(app.registry.id());
    add.values[face_field::MODULE] = "canvas".to_owned();
    add.values[face_field::KIND] = "Canvas".to_owned();
    app.submit_add(&add);
    app.selected = app
        .registry
        .depth_first()
        .into_iter()
        .find(|face| face.registry_name == "canvas")
        .expect("canvas face")
        .id;
    std::fs::write(
        root.join("src/lib.rs"),
        "crate::host!();\n\
         crate::static_graft_plan!(FRAMEWORK, cut \"root/canvas\" graft \"canvas_fast\");\n",
    )
    .expect("host entry");

    app.handle_key(KeyEvent::from(KeyCode::Char('g')));
    let Some(Overlay::Graft(state)) = app.overlay.clone() else {
        panic!("g opens the graft screen");
    };
    assert_eq!(
        state.declaration,
        GraftDeclaration::Declared {
            expression: "cut \"root/canvas\" graft \"canvas_fast\"".to_owned(),
            line: 2,
            cfg: None,
        }
    );

    let _ = std::fs::remove_dir_all(&root);
}

/// A graft read with no selected project is refused, not guessed from the environment or
/// the working directory (audit `STU-S-29`).
/// 没有选中项目时的 graft 读取会被拒绝，而不是从环境或工作目录猜（审计 `STU-S-29`）。
#[test]
#[cfg(feature = "prototype-fixtures")]
fn a_graft_read_without_a_selected_project_is_refused_not_guessed() {
    let Some(mut app) = fixture_app() else {
        return;
    };
    assert_ne!(
        app.selected,
        app.registry.id(),
        "premise: the fixture selects a face, so the graft screen opens"
    );
    clear_project_context();
    app.open_graft();
    let Some(Overlay::Graft(state)) = app.overlay else {
        panic!("g opens the graft screen")
    };
    assert!(
        state.plans.is_empty(),
        "no plan may be listed from a root the reader did not open: {:?}",
        state.plans
    );
    assert!(
        matches!(
            state.declaration,
            GraftDeclaration::Unknown { ref reason } if reason.contains("no project is selected")
        ),
        "the screen must say the selection is missing, not describe a guessed tree: {:?}",
        state.declaration
    );
}
