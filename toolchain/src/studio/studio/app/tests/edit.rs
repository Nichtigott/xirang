//! covers `app/mutations.rs`, `app/keyboard.rs`, `app/state/forms.rs`.
//! 覆盖 `app/mutations.rs`、`app/keyboard.rs`、`app/state/forms.rs`。
//!
//! Editing round-trip regression tests.
//! 编辑往返回归测试。

use super::*;
use crate::run_method::face_field;
use crate::run_method::registry_core::declaration::portable_path;

#[test]
fn edit_save_button_writes_changes_and_adopts_the_old_control_scaffold() {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let root = std::env::temp_dir().join(format!("xirang-toolchain-legacy-{suffix}"));
    let control = root.join("src/control/control.rs");
    let rule = root.join("src/control/registry_rule/registry_rule.rs");
    std::fs::create_dir_all(rule.parent().expect("rule parent")).expect("create fixture");
    std::fs::write(
        &control,
        "pub struct ControlRegistry;\n\ncrate::control_object! {\n    kind: ControlRegistry,\n    needs_registry: true,\n    parent: crate::ROOT_NODE_ID,\n    registry_rule_path: \"src/control/registry_rule/registry_rule.rs\",\n    registry_rule: crate::control::registry_rule::REGISTRATION_RULE,\n}\n",
    )
    .expect("write legacy control");
    std::fs::write(
        &rule,
        "use crate::RegistrationRule;\npub const REGISTRATION_RULE: RegistrationRule = RegistrationRule::ANY;\n",
    )
    .expect("write legacy rule");
    select_project(root.clone(), root.join("Cargo.toml"), "legacy-app");

    let mut app = App::load_app();
    app.selected = app.registry.depth_first()[0].id;
    app.handle_key(KeyEvent::from(KeyCode::Char('e')));
    let Some(Overlay::Edit(id, mut edit)) = app.overlay.take() else {
        panic!("e should open the Edit form");
    };
    edit.field = 11;
    edit.values[face_field::SUMMARY_ZH] = "已保存的摘要。".to_owned();
    edit.editing = true;
    app.overlay = Some(Overlay::Edit(id, edit));
    app.hot.overlay_area = Rect::new(5, 5, 70, 18);
    app.hot.action_confirm_area = Rect::new(10, 18, 14, 3);
    app.handle_overlay_click(12, 19);

    assert!(!app.event.starts_with("Edit failed"), "{}", app.event);
    let source = std::fs::read_to_string(&control).expect("edited control");
    assert!(source.starts_with("// generated-by=XiRang"));
    assert!(source.contains("已保存的摘要。"));
    assert_eq!(
        app.selected_info().map(|info| info.summary.zh.as_str()),
        Some("已保存的摘要。")
    );

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn startup_accepts_a_pre_hierarchy_generated_face_without_parent() {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let root = std::env::temp_dir().join(format!("xirang-toolchain-old-face-{suffix}"));
    let source = root.join("src/test/test.rs");
    std::fs::create_dir_all(source.parent().expect("face parent")).expect("create fixture");
    std::fs::write(
        &source,
        "// generated-by=XiRang\npub struct Test;\ncrate::control_object! { kind: Test, }\n",
    )
    .expect("write old generated face");
    select_project(root.clone(), root.join("Cargo.toml"), "old-face-test");

    let app = App::load_app();

    assert!(app.reload_error.is_none(), "{}", app.event);
    assert_eq!(app.registry.depth_first().len(), 1);
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn editing_module_name_moves_the_face_and_keeps_generated_source_compact() {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let root = std::env::temp_dir().join(format!("xirang-toolchain-rename-{suffix}"));
    std::fs::create_dir_all(root.join("src")).expect("create source root");
    select_project(root.clone(), root.join("Cargo.toml"), "rename-test");

    let mut app = App::load_app();
    let root_id = app.registry.id();
    let mut add = AddState::new(root_id);
    add.values[face_field::MODULE] = "test".to_owned();
    add.values[face_field::NEEDS_REGISTRY] = "true".to_owned();
    add.values[face_field::KIND] = "Test".to_owned();
    app.submit_add(&add);
    assert!(!app.event.starts_with("Add failed"), "{}", app.event);
    let old_source = root.join("src/test/test.rs");
    let initial = std::fs::read_to_string(&old_source).expect("new face source");
    let parsed = xirang_kernel::syntax::parse_face(&initial)
        .unwrap()
        .unwrap();
    // The default author interface is `Soil` + `#[soil]` (maintainer ruling 2026-10-16), and the form reports
    // the spelling it read: a `#[soil]` declaration reports `soil`; a `Registration` literal reports
    // `registration` (kept compatible, but no longer what this path writes).
    // 默认作者接口是 `Soil` ＋ `#[soil]`（维护者 2026-10-16 裁定），而这一形报出它读到的拼法：`#[soil]` 声明报
    // `soil`；`Registration` 字面量报 `registration`（保留兼容，但不再是这条路径写出的东西）。
    assert_eq!(parsed.macro_name, "soil");
    assert_eq!(parsed.path("kind").as_deref(), Some("Test"));
    // `preset` / `parts` are the **type class** half of the literal spelling and are always stated there, so
    // what "compact" means for them is that they carry the defaults; everything else the caller did not ask
    // for stays absent.
    // `preset` / `parts` 是字面量拼法里**类型类**那一半，总会被写明，因此对它们而言"紧凑"意味着带着默认值；
    // 调用方没有点名的其余字段一律不出现。
    // `name` / `summary` are likewise always written (as empty localized text) in this spelling, so what the
    // caller did not ask for shows up as "no text" rather than "no field".
    // `name` / `summary` 在这一形里同样总会被写下（空的本地化文本），因此调用方没点名的东西表现为"没有文本"
    // 而不是"没有字段"。
    for omitted in ["exports", "registry_name"] {
        assert!(parsed.field(omitted).is_none(), "{omitted}: {initial}");
    }
    // The contract here is compactness, not the text's exact shape: the fields the caller did not ask
    // for are present but carry nothing. How an empty localized text is spelled is the renderer's business.
    // 这里的合同是"紧凑"而不是文本的确切形状：调用方没点名的字段在，但什么也没带；空的本地化文本怎么写是
    // 渲染器的事。
    // The default spelling (`Soil` + `#[soil]`, maintainer ruling 2026-10-16) is a builder chain: a field the
    // caller did not set is simply absent, so compactness is asserted on the declaration's spelling rather
    // than on literal-shaped field rows.
    // 默认拼法（`Soil` ＋ `#[soil]`，维护者 2026-10-16 裁定）是一条 builder 链：调用方没设的字段就是不存在，
    // 因此"紧凑"断言在声明的拼法上，而不是字面量形状的字段行上。
    assert!(initial.contains("#[xirang_toolchain::soil]"), "{initial}");
    assert!(!initial.contains("Registration {"), "{initial}");
    assert_eq!(
        parsed.parent(),
        Some(xirang_kernel::syntax::ParentSyntax::Root)
    );

    let id = app
        .registry
        .depth_first()
        .into_iter()
        .find(|info| info.registry_name == "test")
        .expect("new face in registry")
        .id;
    let mut child = AddState::new(id);
    child.values[face_field::MODULE] = "child".to_owned();
    child.values[face_field::NEEDS_REGISTRY] = "true".to_owned();
    child.values[face_field::KIND] = "Child".to_owned();
    app.submit_add(&child);
    assert!(!app.event.starts_with("Add failed"), "{}", app.event);
    let child_id = app
        .registry
        .depth_first()
        .into_iter()
        .find(|info| info.registry_name == "child")
        .expect("child registry in tree")
        .id;
    let mut leaf = AddState::new(child_id);
    leaf.values[face_field::MODULE] = "leaf".to_owned();
    leaf.values[face_field::KIND] = "Leaf".to_owned();
    app.submit_add(&leaf);
    assert!(!app.event.starts_with("Add failed"), "{}", app.event);
    assert!(
        std::fs::read_to_string(root.join("src/test/object/child/object/leaf/leaf.rs"))
            .expect("three-level child source")
            .contains("#[xirang_toolchain::soil]")
    );

    let mut edit = AddState::new(root_id);
    edit.values[face_field::MODULE] = "panel".to_owned();
    edit.values[face_field::NEEDS_REGISTRY] = "true".to_owned();
    edit.values[face_field::TREE_SLOT] = "test".to_owned();
    edit.values[face_field::KIND] = "Test".to_owned();
    app.submit_edit(id, &edit);

    assert!(!app.event.starts_with("Edit failed"), "{}", app.event);
    assert!(!old_source.exists());
    let new_source = root.join("src/panel/panel.rs");
    assert!(new_source.is_file());
    let new_child = root.join("src/panel/object/child/child.rs");
    assert!(new_child.is_file());
    assert!(
        root.join("src/panel/object/child/object/leaf/leaf.rs")
            .is_file()
    );
    assert!(
        std::fs::read_to_string(&new_child)
            .expect("renamed child source")
            .contains("crate::panel::NODE_ID")
    );
    assert!(
        std::fs::read_to_string(&new_child)
            .expect("renamed child source")
            .contains("#[xirang_toolchain::soil]")
    );
    assert_eq!(
        app.selected_info().map(|info| info.source.file.as_str()),
        Some("panel/panel.rs")
    );
    // Re-opening Edit after reload must derive module from the source path.
    // Since the slot now follows the module, the form cannot show a stale slot
    // either: there is no second name left to go stale.
    // reload 后重新打开编辑表单时，module 必须来自源码路径。由于槽位现在跟随模块，
    // 表单也不可能显示过期的槽位：已经没有第二个名字可以过期了。
    app.handle_key(KeyEvent::from(KeyCode::Char('e')));
    let Some(Overlay::Edit(_, reopened)) = app.overlay.take() else {
        panic!("e should reopen the Edit form after module migration");
    };
    assert_eq!(reopened.values[face_field::MODULE], "panel");
    assert_eq!(
        reopened.values[face_field::TREE_SLOT],
        "panel",
        "the tree slot follows the module instead of keeping the old name"
    );
    // Kind is an identity field too, but it is migrated atomically instead of
    // being silently ignored by the form.
    // kind 同样属于身份字段；它应通过原子迁移生效，而不是被表单静默忽略。
    let migrated_id = app
        .selected_info()
        .expect("renamed face remains selected")
        .id;
    let mut kind_edit = reopened;
    kind_edit.values[face_field::KIND] = "Panel".to_owned();
    app.submit_edit(migrated_id, &kind_edit);
    assert!(!app.event.starts_with("Edit failed"), "{}", app.event);
    assert_eq!(
        app.selected_info().map(|info| info.kind.as_str()),
        Some("Panel")
    );
    let panel_id = app
        .selected_info()
        .expect("migrated face remains selected")
        .id;
    let child_parent = app
        .registry
        .depth_first()
        .into_iter()
        .find(|info| info.registry_name == "child")
        .map(|info| info.parent);
    assert_eq!(child_parent, Some(panel_id));
    assert!(
        std::fs::read_to_string(&new_source)
            .expect("renamed face source")
            // The renamed face is the root's own child, and the literal spelling states that link in the
            // declaration: `parent: crate::root_node_id(crate::XIRANG_NAMESPACE)`.
            // 重命名后的面是根自己的孩子，而字面量拼法把这条链接写在声明里：
            // `parent: crate::root_node_id(crate::XIRANG_NAMESPACE)`。
            .contains(".under(crate::root_node_id(crate::XIRANG_NAMESPACE))")
    );

    let _ = std::fs::remove_dir_all(&root);
}

/// A rewrite preserves surrounding source and keeps the previous declaration recoverable:
/// the previous text is kept under the trash the delete path already uses, and
/// the event line says where it went.
/// 重写保留周围手写源码，并保持上一份声明可恢复。它不再**不可恢复**
/// 地丢掉：先前的文本留在删除路径本就使用的回收目录下，而事件行说明它去了哪里。
#[test]
fn a_rewritten_face_keeps_its_previous_text_in_the_trash() {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let root = std::env::temp_dir().join(format!("xirang-toolchain-trash-{suffix}"));
    let control = root.join("src/control/control.rs");
    let rule = root.join("src/control/registry_rule/registry_rule.rs");
    std::fs::create_dir_all(rule.parent().expect("rule parent")).expect("create fixture");
    // One line no form field models: the rewrite cannot keep it, and the reader
    // must still be able to get it back.
    // 一行没有任何表单字段建模的内容：重写留不住它，而读者仍然必须能把它拿回来。
    let hand_written = "// hand-written note that no form field models\n";
    std::fs::write(
        &control,
        format!(
            "{hand_written}pub struct ControlRegistry;\n\ncrate::control_object! {{\n    kind: ControlRegistry,\n    needs_registry: true,\n    parent: crate::ROOT_NODE_ID,\n    registry_rule_path: \"src/control/registry_rule/registry_rule.rs\",\n    registry_rule: crate::control::registry_rule::REGISTRATION_RULE,\n}}\n"
        ),
    )
    .expect("write control");
    std::fs::write(
        &rule,
        "use crate::RegistrationRule;\npub const REGISTRATION_RULE: RegistrationRule = RegistrationRule::ANY;\n",
    )
    .expect("write legacy rule");
    select_project(root.clone(), root.join("Cargo.toml"), "trash-app");

    let mut app = App::load_app();
    app.selected = app.registry.depth_first()[0].id;
    app.handle_key(KeyEvent::from(KeyCode::Char('e')));
    let Some(Overlay::Edit(id, mut edit)) = app.overlay.take() else {
        panic!("e should open the Edit form");
    };
    edit.values[face_field::SUMMARY_ZH] = "重写后的摘要。".to_owned();
    app.overlay = Some(Overlay::Edit(id, edit));
    app.handle_key(KeyEvent::from(KeyCode::Char('s')));

    assert!(!app.event.starts_with("Edit failed"), "{}", app.event);
    let rewritten = std::fs::read_to_string(&control).expect("rewritten control");
    assert!(
        rewritten.contains("hand-written note"),
        "the rewrite preserves what the form does not model: {rewritten}"
    );
    assert!(
        app.event.contains("previous text kept at"),
        "the reader has to be told where the old text went: {}",
        app.event
    );

    let faces = root.join(".xirang/trash/faces");
    let entries = std::fs::read_dir(&faces)
        .expect("the trash directory exists")
        .map(|entry| entry.expect("trash entry").path())
        .collect::<Vec<_>>();
    assert_eq!(entries.len(), 1, "one rewrite, one backup: {entries:?}");
    let backup = std::fs::read_to_string(&entries[0]).expect("backup text");
    assert!(
        backup.contains("hand-written note"),
        "the backup holds the text that was overwritten: {backup}"
    );
    // Both sides name the same file, but they are built differently: the test
    // joins the trash directory from one `/`-carrying string, while the product
    // joins its components. Windows therefore spells the same directory
    // `...\.xirang\trash\faces\x.rs` on one side and
    // `...\.xirang/trash/faces\x.rs` on the other, and a raw `contains`
    // compares two spellings of one path. Folding both to the portable form is
    // what the message actually promises: that it names the backup.
    // 两侧命名的是同一个文件，但拼法不同：测试用一条含 `/` 的字符串拼出垃圾目录，
    // 产品却逐分量拼。于是 Windows 上一侧写成 `...\.xirang\trash\faces\x.rs`，另一侧
    // 写成 `...\.xirang/trash/faces\x.rs`，而裸 `contains` 比较的是同一条路径的两种
    // 拼法。把两边都折叠为可移植形式，才是这条消息真正的承诺：它报出了那份备份。
    assert!(
        portable_path(&app.event).contains(&portable_path(&entries[0].display().to_string())),
        "the message names the backup path: {}",
        app.event
    );

    let _ = std::fs::remove_dir_all(&root);
}

/// Delete moves the module of the project Studio has open, not of whatever
/// directory the process happens to sit in.
/// 删除搬走的是 Studio **打开**的项目的模块，而不是进程恰好所在目录的那个。
///
/// `delete_module` resolves its root through the authoring context, and the delete
/// overlay was the one authoring call that did not establish it — so it acted on the
/// process CWD project while the screen showed another one. A test cannot observe
/// the CWD project without risking the checkout, so this pins the positive half: the
/// open project's file is the one that moves, and the status line names it.
/// `delete_module` 经创作上下文解析根路径，而删除浮层是唯一没有建立它的创作调用——于是它作用
/// 在进程 CWD 的项目上，而屏幕上显示的是另一个。测试无法在不冒犯检出的前提下观察 CWD 项目，
/// 因此这里钉住正面的一半：搬走的是打开项目的文件，而且状态行点名了它。
#[test]
fn delete_moves_the_module_of_the_open_project() {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let root = std::env::temp_dir().join(format!("xirang-toolchain-delete-{suffix}"));
    let control = root.join("src/control/control.rs");
    let rule = root.join("src/control/registry_rule/registry_rule.rs");
    std::fs::create_dir_all(rule.parent().expect("rule parent")).expect("create fixture");
    std::fs::write(
        &control,
        "pub struct ControlRegistry;\n\ncrate::control_object! {\n    kind: ControlRegistry,\n    needs_registry: true,\n    parent: crate::ROOT_NODE_ID,\n    registry_rule_path: \"src/control/registry_rule/registry_rule.rs\",\n    registry_rule: crate::control::registry_rule::REGISTRATION_RULE,\n}\n",
    )
    .expect("write control");
    std::fs::write(
        &rule,
        "use crate::RegistrationRule;\npub const REGISTRATION_RULE: RegistrationRule = RegistrationRule::ANY;\n",
    )
    .expect("write legacy rule");
    select_project(root.clone(), root.join("Cargo.toml"), "delete-app");

    let mut app = App::load_app();
    app.selected = app.registry.depth_first()[0].id;
    app.handle_key(KeyEvent::from(KeyCode::Char('d')));
    assert!(
        matches!(app.overlay, Some(Overlay::Delete(_))),
        "d opens the delete overlay"
    );
    app.handle_key(KeyEvent::from(KeyCode::Char('y')));

    assert!(
        !app.event.starts_with("Delete failed"),
        "delete must act on the open project: {}",
        app.event
    );
    assert!(
        !control.exists(),
        "the open project's module file must be the one that moved"
    );
    assert!(
        app.event.contains(&root.display().to_string()),
        "the status line must name the open project: {}",
        app.event
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// A legacy edit (the one Studio's form still calls) is bounded by the same write scope: a face whose
/// real source is outside the package is refused and nothing is written.
/// 旧编辑路径（Studio 表单仍在调用的那一条）受同一条写入作用域约束：真实源码在包外的面被拒绝，什么都不写。
///
/// Why this is not covered by the shared service's own check: the service is one entrance, and this
/// executor is another (Studio's edit form, the CLI). "Every writer obeys one rule" is only true if
/// the writers that are *not* the service are checked too.
/// 为什么共享服务自己的检查覆盖不到这里：服务是一个入口，这个执行器是另一个（Studio 编辑表单、CLI）。
/// "每个写入方遵守同一条规则"只有在**不是**服务的那些写入方也被检查时成立。
#[test]
fn an_edit_through_the_legacy_executor_cannot_write_outside_the_package() {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let root = std::env::temp_dir().join(format!("xirang-toolchain-legacy-scope-{suffix}"));
    let outside = std::env::temp_dir().join(format!("xirang-toolchain-legacy-out-{suffix}"));
    std::fs::create_dir_all(outside.join("panel")).expect("outside module dir");
    let source_text = "// generated-by=XiRang\npub struct Panel;\n\ncrate::root_object! {\n    kind: Panel,\n    parent: crate::ROOT_NODE_ID,\n}\n";
    std::fs::create_dir_all(root.join("src/panel")).expect("src");
    std::fs::write(root.join("src/panel/panel.rs"), source_text).expect("the face");
    select_project(root.clone(), root.join("Cargo.toml"), "legacy-scope");
    let mut app = App::load_app();
    let Some(face) = app.registry.depth_first().into_iter().next() else {
        panic!("the fixture's face is registered");
    };
    // The tree was read from a real directory; now that directory is a link to somewhere else, so the
    // path the registry recorded resolves outside the package — readable, not writable.
    // 树是从一个真实目录读出来的；现在那个目录是指向别处的链接，因此注册树记录的那条路径解析到包外——
    // 可读、不可写。
    std::fs::rename(root.join("src/panel"), outside.join("panel-moved"))
        .expect("move the face out");
    std::fs::write(outside.join("panel-moved/panel.rs"), source_text).expect("outside face");
    #[cfg(unix)]
    std::os::unix::fs::symlink(outside.join("panel-moved"), root.join("src/panel"))
        .expect("the link back");
    let mut edit = AddState::new(face.parent);
    edit.values[face_field::MODULE] = "panel".to_owned();
    edit.values[face_field::KIND] = "Panel".to_owned();
    edit.values[face_field::SUMMARY_ZH] = "rewritten through the legacy executor".to_owned();
    app.submit_edit(face.id, &edit);
    assert!(
        app.event.starts_with("Edit failed") || app.event.starts_with("Add failed"),
        "an edit that would write through the link must be refused: {}",
        app.event
    );
    assert_eq!(
        std::fs::read_to_string(outside.join("panel-moved/panel.rs")).expect("outside face"),
        source_text,
        "the outside source is untouched"
    );
    let _ = std::fs::remove_dir_all(&root);
    let _ = std::fs::remove_dir_all(&outside);
}

/// Delete refuses a face that is not inside its own same-named directory.
/// 删除拒绝不在自己同名目录里的面。
///
/// The trash move renames the *parent* directory, so a generated face that sits
/// directly under `src/` — the shape a flattened or hand-written module has — would
/// take all of `src/` with it. Such a file carries the generated marker, so the
/// scanner lists it as an ordinary, deletable face; nothing else marks it as
/// unusual. The rename path refuses this shape already, and deleting needs the same
/// rule.
/// 回收搬走的是**父**目录，因此直接位于 `src/` 下的生成面——已被拍平或手写的模块就是这种
/// 形状——会把整个 `src/` 一起带走。这样的文件带生成标记，因此扫描器把它列为普通可删除面；
/// 没有别的东西把它标记为异常。改名路径已经拒绝这种形状，删除需要同一条规则。
#[test]
fn delete_refuses_a_face_outside_its_own_directory() {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let root = std::env::temp_dir().join(format!("xirang-toolchain-flat-delete-{suffix}"));
    std::fs::create_dir_all(root.join("src")).expect("create fixture");
    select_project(root.clone(), root.join("Cargo.toml"), "flat-delete-app");

    // Create a standard module through the authoring path, then flatten it: the file
    // keeps its generated marker and moves out of its own directory.
    // 经创作路径建一个标准模块，然后把它拍平：文件保留生成标记，同时离开它自己的目录。
    let mut app = App::load_app();
    let mut add = AddState::new(app.registry.id());
    add.values[face_field::MODULE] = "widget".to_owned();
    add.values[face_field::KIND] = "Widget".to_owned();
    app.submit_add(&add);
    let generated = root.join("src/widget/widget.rs");
    assert!(
        generated.is_file(),
        "the add form wrote the standard module"
    );
    std::fs::rename(&generated, root.join("src/widget.rs")).expect("flatten the module");
    std::fs::remove_dir_all(root.join("src/widget")).expect("remove the emptied directory");

    let mut app = App::load_app();
    let flattened = app
        .registry
        .depth_first()
        .into_iter()
        .find(|face| face.source.file.ends_with("widget.rs"))
        .expect("the flattened face is registered")
        .id;
    app.selected = flattened;
    app.handle_key(KeyEvent::from(KeyCode::Char('d')));
    assert!(
        matches!(app.overlay, Some(Overlay::Delete(_))),
        "d opens the delete overlay"
    );
    app.handle_key(KeyEvent::from(KeyCode::Char('y')));

    assert!(
        app.event.starts_with("Delete failed"),
        "the delete must be refused rather than moving the whole source tree: {}",
        app.event
    );
    assert!(
        app.event.contains("only standard"),
        "the refusal names the layout it accepts: {}",
        app.event
    );
    assert!(
        root.join("src/widget.rs").is_file(),
        "the face file survives"
    );
    assert!(root.join("src").is_dir(), "src/ survives");
    let _ = std::fs::remove_dir_all(&root);
}
