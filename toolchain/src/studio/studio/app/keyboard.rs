//! Keyboard and overlay-key interaction.
//! 键盘与浮层键盘交互。

use super::*;

/// What the first-screen face keys say when the root — which has no source file —
/// is selected: the footer advertises `d`/`e`/`g` on the first screen, so silence
/// would read as a broken key (audit `STU-V-02`).
/// 选中根（它没有源文件）时首屏那几个面级按键要说的话：页脚在首屏就宣传了 `d`/`e`/`g`，
/// 静默会被读成坏键（审计 `STU-V-02`）。
const ROOT_HAS_NO_SOURCE: &str =
    "Select a registration face first; the root node has no source file.";

impl App {
    pub(super) fn handle_key(&mut self, key: KeyEvent) {
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            self.should_quit = true;
            return;
        }
        if self.overlay.is_some() {
            self.handle_overlay_key(key);
            return;
        }
        match key.code {
            KeyCode::Char('q') => self.should_quit = true,
            KeyCode::Char('1') => self.open_page(StudioPage::Search),
            KeyCode::Char('2') => self.open_page(StudioPage::Inspect),
            KeyCode::Char('3') => self.open_page(StudioPage::Data),
            KeyCode::Char('4') => self.open_page(StudioPage::Multilang),
            KeyCode::Char('/') => self.overlay = Some(Overlay::Search(SearchState::default())),
            KeyCode::Char('n') => self.overlay = Some(Overlay::NewProject(NewProjectState::new())),
            KeyCode::Char('a') => {
                let parent = self.selected_parent();
                let mut add = AddState::new(parent);
                // Keep the default parent readable; the resolver still accepts node identity.
                // 默认父节点显示可读名称；解析器仍接受 node identity。
                add.values[face_field::PARENT] = self
                    .registry
                    .path_for(parent)
                    .unwrap_or_else(|| "root".to_owned());
                if let Some(parent_face) = self.registry.find_registry(parent) {
                    add.apply_parent_rule(&parent_face.registry_rule);
                }
                self.overlay = Some(Overlay::Add(add));
            }
            KeyCode::Char('p') => self.overlay = Some(Overlay::Plugin(PluginState::new())),
            // `c` for the crate partition this host declares: one screen that says what a split
            // would write, what is on disk, and whether it could be published (audit `M7`, P3.6).
            // `c` 打开本宿主声明的 crate 分区：一屏说清一次拆分要写什么、磁盘上已有什么、以及能不能发布
            // （审计 `M7`，P3.6）。
            KeyCode::Char('c') => self.open_partition(),
            KeyCode::Char('d') => {
                if self.selected == self.registry.id() {
                    self.note(ROOT_HAS_NO_SOURCE.to_owned());
                } else {
                    self.overlay = Some(Overlay::Delete(self.selected));
                }
            }
            KeyCode::Char('e') if self.selected != self.registry.id() => self.open_edit_form(),
            KeyCode::Char('g') if self.selected != self.registry.id() => self.open_graft(),
            KeyCode::Char('r') | KeyCode::F(5) => self.reload(),
            KeyCode::Char('b') | KeyCode::F(9) => self.build_all(),
            KeyCode::Tab => {
                self.focus = if self.focus == Focus::Tree {
                    Focus::Details
                } else {
                    Focus::Tree
                };
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if self.focus == Focus::Tree {
                    self.move_selection(-1);
                } else {
                    self.details_selected = self.details_selected.saturating_sub(1);
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.focus == Focus::Tree {
                    self.move_selection(1);
                } else {
                    self.details_selected = (self.details_selected + 1)
                        .min(self.detail_field_count().saturating_sub(1));
                }
            }
            KeyCode::Left | KeyCode::Char('h') => {
                self.split_percent = self.split_percent.saturating_sub(2).max(25)
            }
            KeyCode::Right | KeyCode::Char('l') => {
                self.split_percent = (self.split_percent + 2).min(70)
            }
            KeyCode::Enter
                if self.focus == Focus::Details && self.selected != self.registry.id() =>
            {
                self.open_edit_form();
            }
            // `e` and `g` carry a "a face is selected" guard; say that instead of
            // doing nothing silently (audit `STU-V-02`).
            // `e` 与 `g` 带“已选中一个面”的守卫；要把这一点说出来，而不是静默空操作（审计
            // `STU-V-02`）。
            KeyCode::Char('e' | 'g') if self.selected == self.registry.id() => {
                self.note(ROOT_HAS_NO_SOURCE.to_owned());
            }
            KeyCode::Enter | KeyCode::Char(' ') => self.toggle_selected(),
            _ => {}
        }
    }
    /// The one entry point that opens the editor form for the selected face: the `e`
    /// key and Enter in the details pane both call it, so the two paths cannot drift
    /// apart (audit `STU-S-19`).
    /// 为选中的面打开编辑表单的唯一入口：`e` 键与详情面板的 Enter 都调它，因此两条路径不会
    /// 各自漂移（审计 `STU-S-19`）。
    fn open_edit_form(&mut self) {
        let Some(info) = self.selected_info() else {
            return;
        };
        // The form shows the paths, not the labels: the labels follow
        // them, in the file and in the compiled registration alike.
        // `params` and `handle` are not shown at all — both are the
        // kind by rule, so a row for either would only repeat it.
        // 表单展示路径而不是标签：标签跟随路径——文件里与编译后的注册信息里
        // 都是如此。`params` 与 `handle` 完全不展示：按规则两者都等于 kind，
        // 任何一行的存在都只是重复它。
        let (handle_contracts, part_contracts, contract_failure) =
            declaration_contract_fields(&source_path_for(&info.source.file));
        let mut edit = AddState::new(info.parent);
        // Editing rewrites a face the registry already carries, and this build's registry only
        // carries Rust faces; the language choice belongs to the add form.
        // 编辑重写注册树已经携带的面，而本构建的注册树只携带 Rust 面；语言选择属于新增表单。
        edit.allow_language = false;
        edit.locked_fields.insert(face_field::PARENT);
        edit.values[face_field::PARENT] = self
            .registry
            .path_for(info.parent)
            .unwrap_or_else(|| "root".to_owned());
        // `module` describes the source directory/file, while
        // `registry_name` is the name in the registration tree.
        // They often start equal, but a module rename must not
        // make the editor appear to revert after reload.
        // `module` 表示源码目录/文件名，`registry_name` 表示注册树
        // 中的槽位名。两者初始值可能相同，但重命名后必须分别读取。
        edit.values[face_field::MODULE] = std::path::Path::new(&info.source.file)
            .file_stem()
            .and_then(|stem| stem.to_str())
            .filter(|stem| !stem.is_empty())
            .unwrap_or(info.registry_name.as_str())
            .to_owned();
        edit.values[face_field::NEEDS_REGISTRY] = info.needs_registry.to_string();
        edit.values[face_field::TREE_SLOT] = info.registry_name.to_owned();
        edit.values[face_field::REGISTRY_RULE] = registration_rule_text(&info.registry_rule);
        edit.values[face_field::ADMISSION] = admission_text(&info.admission);
        edit.values[face_field::PARTS] = info.parts.to_owned();
        edit.values[face_field::EXPORTS] = info.exports.join(",");
        edit.values[face_field::KIND] = info.kind.to_owned();
        edit.values[face_field::NAME_ZH] = info.name.zh.to_owned();
        edit.values[face_field::NAME_EN] = info.name.en.to_owned();
        edit.values[face_field::SUMMARY_ZH] = info.summary.zh.to_owned();
        edit.values[face_field::SUMMARY_EN] = info.summary.en.to_owned();
        edit.values[face_field::PRESET] = info.preset.to_owned();
        edit.values[face_field::STABLE_NAME] = info.stable_name.clone().unwrap_or_default();
        edit.values[face_field::GETTING_FROM_OTHER_REGISTRY] =
            info.getting_from_other_registry.clone().unwrap_or_default();
        edit.values[face_field::REGISTRY_RULE_PATH] = info.registry_rule_path.to_owned();
        edit.values[face_field::HANDLE_TRAITS] = info.handle_traits.join(",");
        edit.values[face_field::HANDLE_CONTRACTS] = handle_contracts;
        edit.values[face_field::PART_TRAITS] = info.part_traits.join(",");
        edit.values[face_field::REQUIRES] = info
            .requires
            .iter()
            .map(|requirement| format!("{}=>{}", requirement.capability, requirement.provider))
            .collect::<Vec<_>>()
            .join(",");
        edit.values[face_field::PROVIDES] = info.provides.join(",");
        edit.values[face_field::RUNTIME_CHECKS] = info
            .runtime_checks
            .iter()
            .map(|check| check.name())
            .collect::<Vec<_>>()
            .join(",");
        edit.values[face_field::FLOW] = if info.flow.is_declared() {
            format!(
                "{}|{}|{}|{}",
                info.flow.id, info.flow.version, info.flow.input, info.flow.output
            )
        } else {
            String::new()
        };
        edit.values[face_field::FLOW_PROVIDER] =
            info.flow_provider.as_deref().unwrap_or_default().to_owned();
        edit.values[face_field::PART_CONTRACTS] = part_contracts;
        if let Some(parent_face) = self.registry.find_registry(info.parent) {
            edit.apply_parent_rule(&parent_face.registry_rule);
        }
        self.overlay = Some(Overlay::Edit(info.id, edit));
        if let Some(failure) = contract_failure {
            // The form still opens — a reader may be editing precisely to fix that file —
            // but the event line says the contracts could not be read instead of letting
            // two empty columns read as "no contracts" (audit `LGC-LG-34`). Written after
            // the overlay because `info` borrows the session until its last use.
            // 表单照旧打开——读者可能正是为修这个文件而来——但事件行要说明契约读不到，而不是让两列
            // 空白被读成“没有契约”（审计 `LGC-LG-34`）。写在浮层之后，因为 `info` 借用会话直到它
            // 最后一次被使用。
            self.alert(failure);
        }
    }
}

pub(super) fn declaration_contract_paths(source: &str) -> (String, String) {
    let Ok(Some(face)) = crate::run_method::parse_face(source) else {
        return (String::new(), String::new());
    };
    let paths = |field| face.path_list(field).unwrap_or_default().join(",");
    (paths("handle_contracts"), paths("part_contracts"))
}

/// The two contract columns for one source file, and the sentence to show when it could
/// not be read.
/// 一个源文件的两列契约，以及读不到时要显示的那句话。
///
/// An unreadable file used to look exactly like a file with no contracts: both came back
/// empty, so the editor showed blank contract columns for a file it had never read
/// (audit `LGC-LG-34`). A failure now says so in the columns themselves *and* hands the
/// caller the line to show; an empty result still means what it says.
/// 读不到的文件过去与“没有契约”的文件一模一样：两者都回空，于是编辑器会为一个它从未读过的文件
/// 显示空白契约列（审计 `LGC-LG-34`）。现在失败既在列里自己说出来，也把要展示的那句话交给调用
/// 方；空结果仍然只表示它字面的意思。
pub(super) fn declaration_contract_fields(
    path: &std::path::Path,
) -> (String, String, Option<String>) {
    match std::fs::read_to_string(path) {
        Ok(source) => {
            let (handle, part) = declaration_contract_paths(&source);
            (handle, part, None)
        }
        Err(error) => {
            let unreadable = format!("<unreadable: {error}>");
            (
                unreadable.clone(),
                unreadable,
                Some(format!(
                    "Could not read {} for its contract columns: {error}",
                    path.display()
                )),
            )
        }
    }
}

#[cfg(test)]
mod edit_form_tests {
    //! The Edit form's admission prefill and its write-back (audit `LGC-LG-02`).
    //! Edit 表单的 admission 预填与写回（审计 `LGC-LG-02`）。

    use super::super::project_context::select_project;
    use super::*;
    use crate::run_method::authoring::parse::parse_admission_owned;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    /// A generated face whose admission names both an allow and a deny list.
    /// 一个生成面，其 admission 同时点名 allow 与 deny 两张列表。
    const FACE_SOURCE: &str = "pub struct ControlRegistry;\n\ncrate::control_object! {\n    kind: ControlRegistry,\n    needs_registry: true,\n    parent: crate::ROOT_NODE_ID,\n    registry_rule_path: \"src/control/registry_rule/registry_rule.rs\",\n    registry_rule: crate::control::registry_rule::REGISTRATION_RULE,\n    admission: crate::Admission::new(&[\"ui\"], &[\"ui/experimental\"]),\n}\n";

    /// The registration rule the fixture face names.
    /// 夹具注册面所点名的注册规则。
    const RULE_SOURCE: &str = "use crate::RegistrationRule;\npub const REGISTRATION_RULE: RegistrationRule = RegistrationRule::ANY;\n";

    /// A temp host project with one such face; returns its root and face file.
    /// 一个只含该注册面的临时宿主工程；返回其根与注册面文件。
    fn temp_project(label: &str) -> (PathBuf, PathBuf) {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("xirang-toolchain-edit-{label}-{suffix}"));
        let rule = root.join("src/control/registry_rule/registry_rule.rs");
        std::fs::create_dir_all(rule.parent().expect("rule parent")).expect("fixture dir");
        std::fs::write(&rule, RULE_SOURCE).expect("rule source");
        let face = root.join("src/control/control.rs");
        std::fs::write(&face, FACE_SOURCE).expect("face source");
        std::fs::write(root.join("src/lib.rs"), "// host entry\n").expect("library target");
        std::fs::write(
            root.join("Cargo.toml"),
            "[package]\nname = \"edit-fixture\"\nversion = \"0.1.0\"\n",
        )
        .expect("manifest");
        select_project(root.clone(), root.join("Cargo.toml"), "edit-fixture");
        (root, face)
    }

    /// The prefill the Edit form shows must keep both lists: it is the value the
    /// next save hands back to the kernel, so a prefill that dropped the deny list
    /// is how an untouched form rewrote the gate wider.
    /// Edit 表单显示的预填值必须保留两张列表：它是下次保存交回内核的值，因此丢掉 deny 列表的
    /// 预填正是“没改过任何字段的表单把门禁改写得更宽”的路径。
    #[test]
    fn the_edit_form_prefill_keeps_both_admission_lists() {
        let (root, _face) = temp_project("prefill");
        let mut app = App::load_app();
        let admission = app
            .registry
            .depth_first()
            .into_iter()
            .next()
            .map(|info| info.admission.clone())
            .expect("the fixture face is registered");
        assert_eq!(
            admission.denied_paths,
            ["ui/experimental"],
            "premise: the fixture declares both lists"
        );

        app.handle_key(KeyEvent::from(KeyCode::Char('e')));
        let Some(Overlay::Edit(_, form)) = app.overlay.as_ref() else {
            panic!("e must open the Edit form: {}", app.event);
        };
        let prefill = form.values[face_field::ADMISSION].clone();
        let read = parse_admission_owned(&prefill);
        assert!(
            read.as_ref()
                .is_ok_and(|read| read.denied_paths == ["ui/experimental"]),
            "the Edit form prefill dropped the deny list: prefill={prefill:?} read={read:?}"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    /// Saving an untouched form must not change the source's gate: the deny list
    /// the author wrote has to still be there after the round trip.
    /// 保存一份没改过的表单不得改变源码里的门禁：作者写下的 deny 列表在往返之后必须还在。
    #[test]
    fn saving_an_untouched_edit_form_keeps_the_source_deny_list() {
        let (root, face) = temp_project("write-back");
        let mut app = App::load_app();
        app.handle_key(KeyEvent::from(KeyCode::Char('e')));
        let Some(Overlay::Edit(id, form)) = app.overlay.as_ref() else {
            panic!("e must open the Edit form: {}", app.event);
        };
        let (id, form) = (*id, form.clone());

        app.submit_edit(id, &form);

        let text = std::fs::read_to_string(&face).expect("face readable");
        assert!(
            text.contains("ui/experimental"),
            "saving the untouched form erased the deny list: event={}\n{text}",
            app.event
        );
        let _ = std::fs::remove_dir_all(&root);
    }
}
