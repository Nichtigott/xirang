//! Pins for the B6-studio batch's consistency cleanups: the first-screen keys say why
//! they do nothing on the root, the modal hot zones reset in one place, the named row
//! indices replace bare numbers, the call-graph cache keys on the source stamp, both
//! plugin locks count for the hot-reload stamp, the reserved hot-zone rectangles are
//! gone, and the graft screen calls its reader `read_graft_state`.
//! B6-studio 批次一致性清扫的钉子：首屏按键在根上会说明原因、模态热区只有一处复位、具名行下标
//! 取代裸数字、调用图缓存把源码戳纳入键、两个插件锁都参与热重载戳、预留的热区矩形已删除、
//! graft 界面把它的读取器叫 `read_graft_state`。
//!
//! covers `app/keyboard.rs`, `ui/overlay.rs`, `app/overlay/{plugin,new_project}.rs`,
//! `app/pointer.rs`, `ui/graph/nodes.rs`, `app/navigation.rs`, `app/hot_zones.rs` and
//! `app/graft.rs` — one assertion group per finding.
//! 覆盖上列生产模块——每个 finding 一组断言。

use super::*;

/// The first-screen face keys say why they do nothing when the root is selected.
/// 选中根时首屏那几个面级按键会说明它们为什么什么都不做。
#[test]
fn the_root_only_keys_say_why() {
    for key in ['d', 'e', 'g'] {
        let mut app = App::load_app();
        assert_eq!(
            app.selected,
            app.registry.id(),
            "premise: a fresh session selects the root"
        );
        app.handle_key(KeyEvent::from(KeyCode::Char(key)));
        assert!(
            app.event.contains("Select a registration face first"),
            "`{key}` must say why it did nothing: {}",
            app.event
        );
        assert!(
            app.overlay.is_none(),
            "`{key}` must not open a modal on the root"
        );
    }
}

/// The modal hot zones reset in one place, and the reserved rectangles are gone.
/// 模态热区只有一处复位，预留矩形已删除。
#[test]
fn the_modal_hot_zones_reset_in_one_place() {
    // Since audit `STU-S-04` the reset writes the render cache instead of `App`, so the
    // needle names the cache; the claim is unchanged — one reset body, called from both
    // paths.
    // 自审计 `STU-S-04` 起，复位写的是渲染缓存而不是 `App`，因此 needle 指向缓存；命题没变——
    // 一份复位体，两条路径都调它。
    let overlay = include_str!("../../ui/overlay.rs");
    assert_eq!(
        overlay
            .matches("cache.hot.overlay_list_area = Rect::default()")
            .count(),
        1,
        "the finding was two copies of the same reset"
    );
    assert_eq!(
        overlay.matches("reset_hot_zones(cache,").count(),
        2,
        "both paths call the one helper"
    );
    let zones = include_str!("../hot_zones.rs");
    for reserved in ["action_validate_area", "action_edit_area"] {
        assert!(
            !zones.contains(reserved),
            "`{reserved}` has no screen that sets it"
        );
        assert!(
            !overlay.contains(reserved),
            "`{reserved}` must not be reset either"
        );
    }
}

/// Bare row numbers cannot survive a reordering; the named indices can.
/// 裸行号撑不过一次重新排序；具名下标记可以。
#[test]
fn the_form_rows_are_named_indices_not_numbers() {
    for (file, source) in [
        (
            "app/overlay/plugin.rs",
            include_str!("../overlay/plugin.rs"),
        ),
        (
            "app/overlay/new_project.rs",
            include_str!("../overlay/new_project.rs"),
        ),
        ("app/pointer.rs", include_str!("../pointer.rs")),
    ] {
        for bare in ["min(6)", "min(2)", "== 6", "== 2"] {
            assert!(
                !source.contains(bare),
                "{file} still spells the bare row number `{bare}`"
            );
        }
    }
    assert!(include_str!("../overlay/plugin.rs").contains("plugin_field::COUNT - 1"));
    assert!(include_str!("../overlay/new_project.rs").contains("new_project_field::COUNT - 1"));
    assert!(include_str!("../pointer.rs").contains("new_project_field::KIND"));
}

/// The call-graph widget's cache key carries the stamp its tree came from.
/// 调用图控件的缓存键带着它的树来自哪个源码戳。
#[test]
fn the_graph_cache_key_carries_the_source_stamp() {
    let nodes = include_str!("../../ui/graph/nodes.rs");
    assert!(
        nodes.contains("fn flow_key(view: &CallTreeView, source_stamp: u128)"),
        "the key takes the stamp"
    );
    assert!(
        nodes.contains("flow_key(&view, app.source_stamp())"),
        "the call site passes it"
    );
}

/// Both plugin locks mark a source change for the hot-reload stamp.
/// 两个插件锁都会为热重载戳标记一次源码变更。
#[test]
fn both_plugin_locks_mark_a_source_change() {
    let stamp = include_str!("../source_stamp.rs");
    for lock in ["\"official.lock\"", "\"user.lock\""] {
        assert!(stamp.contains(lock), "the stamp must accept {lock}");
    }
}

/// The graft screen reads its state, and its three words stay three words.
/// graft 界面读的是状态，它的三个词仍是三个词。
#[test]
fn the_graft_reader_is_named_for_reading() {
    let graft = include_str!("../graft.rs");
    assert!(
        !graft.contains("graft_facts"),
        "the old noun-phrase name is gone"
    );
    assert!(graft.contains("fn read_graft_state("));
    assert!(
        graft.contains("entry line") && graft.contains("入口行"),
        "the terminology bullet names the three artefacts in both languages"
    );
}

/// The add and edit overlays run the same transitions: equivalent input leaves the two
/// forms in the same state.
/// 添加与编辑浮层跑同一套转移：等价输入让两份表单落在同一状态。
#[test]
fn add_and_edit_forms_reach_the_same_state() {
    let mut add_app = App::load_app();
    let mut edit_app = App::load_app();
    let root = add_app.registry.id();
    add_app.overlay = Some(Overlay::Add(AddState::new(root)));
    edit_app.overlay = Some(Overlay::Edit(root, AddState::new(root)));
    for key in [
        KeyCode::Down,
        KeyCode::Tab,
        KeyCode::Down,
        KeyCode::Up,
        KeyCode::Char('x'),
        KeyCode::Enter,
        KeyCode::Char('z'),
        KeyCode::Enter,
    ] {
        add_app.handle_overlay_key(KeyEvent::from(key));
        edit_app.handle_overlay_key(KeyEvent::from(key));
    }
    let (Some(Overlay::Add(add)), Some(Overlay::Edit(_, edit))) =
        (add_app.overlay, edit_app.overlay)
    else {
        panic!("both forms must stay open")
    };
    assert_eq!(add.field, edit.field, "the same row");
    assert_eq!(add.editing, edit.editing, "the same mode");
    assert_eq!(add.values, edit.values, "the same values");
}

/// The add/edit transition table exists once; the two overlays only decide what a
/// submit writes.
/// 添加/编辑的转移表只有一份；两个浮层只决定提交写什么。
#[test]
fn the_add_and_edit_transitions_live_once() {
    for (name, source) in [
        ("add.rs", include_str!("../overlay/add.rs")),
        ("edit.rs", include_str!("../overlay/edit.rs")),
    ] {
        assert!(
            source.contains("face_form_key(key, &mut "),
            "{name} must run the shared transition table"
        );
        assert!(
            !source.contains("move_face_field"),
            "{name} must not carry its own transitions"
        );
    }
    assert!(include_str!("../overlay/face_form.rs").contains("pub(super) fn face_form_key"));
}

/// All four form pages derive their button row from the one layout.
/// 四个表单页的按钮行都由同一处布局派生。
#[test]
fn the_form_pages_share_one_button_row() {
    for (name, source) in [
        ("face.rs", include_str!("../../ui/forms/face.rs")),
        ("project.rs", include_str!("../../ui/forms/project.rs")),
        ("plugin.rs", include_str!("../../ui/forms/plugin.rs")),
        ("graft.rs", include_str!("../../ui/forms/graft.rs")),
    ] {
        assert!(
            source.contains("draw_form_frame("),
            "{name} must use the shared button row"
        );
        assert!(
            !source.contains("Constraint::Percentage(25)"),
            "{name} must not spell its own four-way split"
        );
    }
    assert_eq!(
        include_str!("../../ui/forms.rs")
            .matches("pub(super) fn draw_form_frame(")
            .count(),
        1,
        "one layout function"
    );
}

/// Severity is set where the line is written, not parsed out of it: a failure whose
/// text never says "fail" is still an alert (audit `STU-S-18`).
/// 严重程度在写入那一行的地方设定，而不是解析出来的：文案里从不含 “fail” 的失败仍是 alert
/// （审计 `STU-S-18`）。
#[test]
fn severity_is_set_where_the_line_is_written() {
    let mut app = App::load_app();
    assert!(!app.event_is_alert, "a fresh session's line is ordinary");
    app.note("Selected nothing in particular");
    assert!(!app.event_is_alert, "`note` is the ordinary channel");
    let failure = "the MIR snapshot could not be produced";
    assert!(
        !failure.to_ascii_lowercase().contains("fail"),
        "premise: the old rule read this line as ordinary"
    );
    app.alert(failure);
    assert!(app.event_is_alert, "`alert` is the severe channel");

    // And nothing draws the colour by reading the text.
    let status = include_str!("../../ui/status.rs");
    assert!(
        !status.contains("contains(\"fail\")") && !status.contains("starts_with(\"Warning:\")"),
        "the event colour still parses the wording"
    );
}

/// The MIR failure is the example that made the old rule worth removing: its text never
/// said "fail", so it was drawn green, and the site now routes it to `alert` (audit
/// `STU-S-18`).
/// MIR 失败正是让旧规则值得被删掉的例子：它的文案从不含 “fail”，因此被画成绿色；现在该站点把它
/// 交给 `alert`（审计 `STU-S-18`）。
#[test]
fn the_mir_failure_that_used_to_be_green_is_an_alert() {
    let text = "MIR unavailable: cargo rustc exited with exit status: 1";
    assert!(
        !text.to_ascii_lowercase().contains("fail") && !text.starts_with("Warning:"),
        "premise: the old rule missed this line on both of its ways in"
    );
    assert!(
        include_str!("../lifecycle.rs")
            .contains("Err(error) => self.alert(format!(\"MIR unavailable: {error}\")),"),
        "the MIR failure must go through `alert`"
    );
}

/// The one invariant that keeps the channel honest: `self.event` is assigned in the two
/// write points and nowhere else (audit `STU-S-18`).
/// 让这条通道保持诚实的不变量：`self.event` 只在那两个写入口被赋值，别处没有（审计
/// `STU-S-18`）。
#[test]
fn every_event_line_goes_through_note_or_alert() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut offenders = Vec::new();
    let mut stack = vec![root];
    while let Some(directory) = stack.pop() {
        for entry in std::fs::read_dir(&directory)
            .expect("src is readable")
            .flatten()
        {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            if path.extension().and_then(|extension| extension.to_str()) != Some("rs") {
                continue;
            }
            if path.ends_with(file!()) {
                // This test spells the needle it searches for.
                continue;
            }
            let source = std::fs::read_to_string(&path).expect("a readable module");
            let count = source.matches("self.event = ").count();
            if count == 0 {
                continue;
            }
            if !path.ends_with("app/lifecycle.rs") || count != 2 {
                offenders.push(format!("{}: {count}", path.display()));
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "raw assignments outside `note`/`alert`: {offenders:?}"
    );
}

/// A project write's outcome is applied in one place, and the session's stamp already
/// accounts for the file it wrote (audit `STU-S-27`).
/// 一次项目写入的结果只有一处应用，而且会话的源码戳已经计入了它写的那个文件（审计 `STU-S-27`）。
#[test]
fn a_project_write_is_applied_in_one_place() {
    // Structural: both write paths call the one entry point, and nothing else applies a
    // snapshot or a reload.
    // 结构上：两条写路径都调用唯一入口，别处不再有应用快照或重载的代码。
    // Four write paths now, not five: this round removed the Rust add's own executor call, so the
    // add form — every language — is the one in-language path. The count is the structural claim,
    // and the claim it encodes changed with the entrance, so the number follows it.
    // 现在有四条写路径而不是五条：本轮移除了 Rust 新增自己的执行器调用，于是新增表单——所有语言——就是
    // 唯一那条"按语言创作"的路径。计数就是那条结构性主张，而主张随入口一起变了，数字也跟着它走。
    let mutations = include_str!("../mutations.rs");
    assert_eq!(
        mutations.matches("self.after_project_write(").count(),
        4,
        "new project, the add form (every language), edit and plugin all share it"
    );
    assert!(
        !mutations.contains("register_snapshot_batch") && !mutations.contains("self.reload()"),
        "applying the snapshot moved into `after_project_write`"
    );
    assert!(include_str!("../lifecycle.rs").contains("pub(super) fn after_project_write("));

    // Behavioural: the file this write produced is already part of the stamp the session
    // holds, so the hot-reload watcher cannot read our own write as someone else's change.
    // 行为上：这次写入产生的文件已经计入会话持有的源码戳，因此热重载监听器不会把我们自己的写入
    // 读成别人的变更。
    let mut app = App::load_app();
    app.last_source_stamp = 0;
    let _ = app.after_project_write(None);
    assert_eq!(
        app.last_source_stamp,
        super::super::source_stamp::source_stamp(),
        "the write is accounted for before the next poll"
    );
}
