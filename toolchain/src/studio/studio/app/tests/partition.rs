//! The partition screen's own pins: what it opens on, and what its three actions do (audit `M7`,
//! P3.6).
//! 分区屏自己的钉子：它打开时看到什么，以及它的三个动作做什么（审计 `M7`，P3.6）。

use super::*;
use crate::build_method::OnDisk;
use crate::build_method::crate_plan::crates_dir;
use std::fs;
use std::path::PathBuf;

/// A host package with a declaration, the records a plan reads, and two faces.
/// 一个宿主包：带声明、规划要读的记录，以及两个注册面。
fn partition_host(label: &str) -> PathBuf {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let area = std::env::temp_dir().join(format!(
        "xirang-studio-partition-{label}-{}-{sequence}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&area);
    // One level down, because a ghost is the host's sibling: a host directly in the temp directory
    // would plan its packages into `/tmp` (see the view pins' note).
    // 在下一层，因为幽灵是宿主的同级：直接坐在临时目录里的宿主会把它的包规划进 `/tmp`（见视图钉子的说明）。
    let root = area.join("host");
    fs::create_dir_all(root.join("src/panel/frame")).expect("the host's sources");
    fs::write(
        root.join("src/lib.rs"),
        "xirang_toolchain::run_method::host!();\n",
    )
    .expect("the entry");
    fs::write(
        root.join("src/panel/panel.rs"),
        "crate::root_object! { kind: Panel, parent: crate::ROOT_NODE_ID, }\n",
    )
    .expect("the root face");
    fs::write(
        root.join("src/panel/frame/frame.rs"),
        "crate::panel_object! { kind: Frame, parent: crate::panel::NODE_ID, }\n",
    )
    .expect("the fragment's face");
    fs::write(
        root.join("Cargo.toml"),
        format!(
            "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n\
             [dependencies]\nxirang-toolchain = {{ path = {:?} }}\n",
            env!("CARGO_MANIFEST_DIR")
        ),
    )
    .expect("the host manifest");
    fs::write(
        root.join("add_crates.rs"),
        "use xirang_toolchain::run_method::{Crate, Shape};\n\n\
         pub const SHAPE: Shape = Shape {\n    package_prefix: \"app\",\n\
         crates: &[Crate::named(\"widgets\").at(&[crate::panel::frame::SUBTREE])],\n};\n",
    )
    .expect("the declaration");
    let out = root.join("target/xirang/out");
    fs::create_dir_all(&out).expect("the records directory");
    // Literal ids: the planner reads them as data (ancestor shells and the records it publishes),
    // and this fixture is about what the screen does with a plan, not about the hash.
    // 字面 id：规划器把它们当数据读（祖先壳与它发布的记录），而本夹具关心的是本屏拿一份计划做什么，
    // 不是那个散列。
    let id = |relative: &str, _kind: &str| match relative {
        "panel/panel.rs" => "00000000000000000000000000000001",
        _ => "00000000000000000000000000000002",
    };
    let mut manifest = String::from(
        "# node\tsource\tsymbol\tpath\tkind\tregistry_name\tparent\tsource_hash\tfields\tcalls\t\
         parent_node\towns_registry\tlogical_path\n",
    );
    for (source, kind) in [
        ("panel/panel.rs", "Panel"),
        ("panel/frame/frame.rs", "Frame"),
    ] {
        manifest.push_str(&format!(
            "{}\t{source}\t{kind}\t-\t{kind}\t{kind}\t-\t-\t-\t-\t-\tfalse\t-\n",
            id(source, kind)
        ));
    }
    fs::write(out.join("pruning_manifest.tsv"), manifest).expect("the pruning manifest");
    root
}

/// Open the screen on that host, and read what it shows.
/// 在那个宿主上打开本屏，并读出它显示的东西。
fn open_screen(root: &std::path::Path) -> App {
    select_project(root.to_path_buf(), root.join("Cargo.toml"), "app");
    let mut app = App::load_app();
    app.open_partition();
    app
}

/// The partition the fixture declares.
/// 夹具声明的那份拆分。
fn state(app: &App) -> PartitionState {
    match app.overlay.as_ref() {
        Some(Overlay::Partition(state)) => state.clone(),
        other => panic!("the partition overlay should be open, found {other:?}"),
    }
}

/// Opening the screen shows the declaration's packages, and says nothing is written yet.
/// 打开本屏会显示声明出来的包，并说明还什么都没写。
#[test]
fn the_screen_shows_what_the_declaration_plans() {
    let root = partition_host("shows");
    let app = open_screen(&root);
    let state = state(&app);
    assert!(state.error.is_none(), "{:?}", state.error);
    assert!(state.declared);
    let view = state.view.as_ref().expect("a view");
    assert_eq!(view.package_prefix, "app");
    assert_eq!(view.packages.len(), 2, "one ghost and one facade");
    assert_eq!(view.packages[0].package, "app-widgets");
    assert_eq!(view.packages[0].compiles, 1, "the fragment's one face");
    assert_eq!(view.packages[0].on_disk, OnDisk::Absent);
    assert_eq!(view.packages[1].package, "app-facade");
    assert_eq!(
        view.packages[1].depends_on,
        vec!["app".to_owned(), "app-widgets".to_owned()],
        "the facade depends on the host and the ghost, which is the publish order"
    );
    let _ = fs::remove_dir_all(root);
    clear_project_context();
}

/// A host with no declaration opens on a screen that says so rather than on an empty list.
/// 没有声明的宿主打开后，本屏说出来，而不是给一张空清单。
#[test]
fn a_host_that_declares_nothing_says_so() {
    let root = partition_host("none");
    fs::remove_file(root.join("add_crates.rs")).expect("remove the declaration");
    let app = open_screen(&root);
    let state = state(&app);
    assert!(!state.declared);
    assert!(state.view.is_none(), "there is no partition to show");
    assert!(state.error.is_none(), "and no declaration is not an error");
    let _ = fs::remove_dir_all(root);
    clear_project_context();
}

/// The three actions write and take back the same packages the CLI verb does.
/// 三个动作写下、收回的包，与 CLI 动词是同一批。
#[test]
fn the_actions_write_and_take_back_the_partition() {
    let root = partition_host("actions");
    let mut app = open_screen(&root);
    let ghost = crates_dir(&root).join("app-widgets");
    let facade = crates_dir(&root).join("app-facade");

    // Arm and confirm: one key arms, `y` runs, and the outcome is said out loud.
    app.handle_key(KeyEvent::from(KeyCode::Char('R')));
    assert!(
        state(&app).pending.is_some(),
        "the release write waits for its confirmation"
    );
    assert!(
        !ghost.exists(),
        "arming an action writes nothing until it is confirmed"
    );
    app.handle_key(KeyEvent::from(KeyCode::Char('y')));
    let state = state(&app);
    assert!(state.pending.is_none(), "the confirmation consumed the arm");
    assert!(
        state.outcome.as_deref().unwrap_or("").contains("release"),
        "{:?}",
        state.outcome
    );
    let manifest = fs::read_to_string(ghost.join("Cargo.toml")).expect("the release manifest");
    assert!(
        manifest.contains("shape = \"release\""),
        "the release shape is fingerprinted: {manifest}"
    );
    assert!(
        ghost.join("src/panel/frame/frame.rs").is_file(),
        "the release shape carries the fragment's own sources"
    );
    assert!(facade.join("build.rs").is_file(), "and the facade exists");
    assert_eq!(
        state
            .view
            .as_ref()
            .expect("a view")
            .packages
            .iter()
            .map(|package| package.on_disk)
            .collect::<Vec<_>>(),
        vec![OnDisk::Release, OnDisk::Release],
        "the screen now reads the release shape off the disk"
    );

    // Reverting is the same two presses, and it takes exactly those packages back.
    app.handle_key(KeyEvent::from(KeyCode::Char('x')));
    app.handle_key(KeyEvent::from(KeyCode::Char('y')));
    assert!(!ghost.exists(), "the revert removed the ghost");
    assert!(!facade.exists(), "the revert removed the facade");
    let _ = fs::remove_dir_all(root);
    clear_project_context();
}

/// `D` removes the selected crate from the declaration, and `y` confirms it — the same two-press
/// discipline as the three package actions, because this edits the author's hand-written source.
/// `D` 把选中的 crate 从声明里移除，`y` 确认——与那三个针对生成包的动作同一套"两次按键"纪律，因为它编辑的是
/// 作者手写的源码。
#[test]
fn the_delete_key_removes_the_selected_crate_from_the_declaration() {
    let root = partition_host("undeclare");
    let mut app = open_screen(&root);
    let file = root.join("add_crates.rs");
    let before = fs::read_to_string(&file).expect("the declaration");
    let app_widgets = crates_dir(&root).join("app-widgets");
    assert!(before.contains(r#"Crate::named("widgets")"#), "{before}");

    // Arming writes nothing.
    app.handle_key(KeyEvent::from(KeyCode::Char('D')));
    assert_eq!(
        state(&app).pending_undeclare.as_deref(),
        Some("widgets"),
        "the selected crate is the one armed"
    );
    assert!(
        !app_widgets.exists(),
        "and arming touches neither the file nor a package"
    );

    // `y` removes exactly that entry, and the screen reads the new declaration.
    app.handle_key(KeyEvent::from(KeyCode::Char('y')));
    let after_confirm = state(&app);
    assert!(
        after_confirm.pending_undeclare.is_none(),
        "the arm was consumed"
    );
    assert!(
        after_confirm
            .outcome
            .as_deref()
            .unwrap_or("")
            .contains("widgets"),
        "{:?}",
        after_confirm.outcome
    );
    // This fixture declares a single crate, so removing it takes the declaration with it — the host
    // is one crate again, which is what the screen must then say.
    // 这个夹具只声明了一个 crate，因此移除它会连声明一起带走——宿主回到"就是一个 crate"，而那正是本屏接下来
    // 必须说的话。
    assert!(
        !file.exists(),
        "removing the only declared crate takes the file"
    );
    assert!(
        !state(&app).declared,
        "and the screen reads the host as one crate again"
    );
    let _ = fs::remove_dir_all(root);
    clear_project_context();
}

/// `D` on the facade says why it cannot be removed, rather than doing nothing.
/// 在 facade 上按 `D` 会说出它为什么不能被移除，而不是什么都不做。
#[test]
fn the_delete_key_names_the_facade_as_not_a_crate() {
    let root = partition_host("facade-key");
    let mut app = open_screen(&root);
    // The facade is the second row, and it serves no single declared crate.
    app.handle_key(KeyEvent::from(KeyCode::Char('j')));
    app.handle_key(KeyEvent::from(KeyCode::Char('D')));
    let state = state(&app);
    assert!(
        state.pending_undeclare.is_none(),
        "the facade cannot be undeclared"
    );
    assert!(
        app.event.contains("facade") || app.event.contains("declared crate"),
        "the key says why: {:?}",
        app.event
    );
    let _ = fs::remove_dir_all(root);
    clear_project_context();
}

/// A crate whose package is still on disk is refused with the key that fixes it, because after the
/// entry is gone nothing knows that directory any more.
/// 包还在磁盘上的 crate 会被拒绝，并给出能修好的那个键——因为条目一旦消失，就没有东西还知道那个目录了。
#[test]
fn the_delete_key_refuses_while_the_package_is_on_disk() {
    let root = partition_host("guarded");
    let mut app = open_screen(&root);
    let area = root.clone();
    fs::create_dir_all(crates_dir(&area).join("app-widgets")).expect("a package directory");
    app.handle_key(KeyEvent::from(KeyCode::Char('D')));
    app.handle_key(KeyEvent::from(KeyCode::Char('y')));
    let state = state(&app);
    assert!(
        state.outcome.as_deref().unwrap_or("").contains("revert"),
        "{:?}",
        state.outcome
    );
    assert!(
        fs::read_to_string(root.join("add_crates.rs"))
            .expect("the declaration")
            .contains(r#"Crate::named("widgets")"#),
        "the entry is still there"
    );
    let _ = fs::remove_dir_all(area);
    clear_project_context();
}
