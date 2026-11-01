//! External graft authoring: compose a plan, read the host entry, manage plans.
//! 外部 graft 创作：撰写计划、读取宿主入口、管理已有计划。
//!
//! Three layers meet here and the screen keeps them apart:
//! 三个层次在此相遇，界面把它们分清楚：
//!
//! * `static_graft_plan!` in the host entry is the **declaration** the build
//!   reads to keep a slot alive and to fill the release-time static plan.
//! * `Registry::overlay` is the **application** that produces the effective
//!   tree at runtime; it never edits either registry.
//! * `graft.plan` under `.xirang/external-grafts/` is the **record** this
//!   screen writes. The screen consumes it here, and the runtime overlay path
//!   (`run_method::apply_recorded_grafts` → `Registry::overlay_recorded`)
//!   consumes it too.
//!
//! * 宿主入口的 `static_graft_plan!` 是构建读取的**声明**，用来保住槽位并填充发布态
//!   静态计划。
//! * `Registry::overlay` 是运行期产生有效树的**应用**；它不改动任何一棵树。
//! * `.xirang/external-grafts/` 下的 `graft.plan` 是本界面写下的**记录**：本界面在
//!   这里消费它，运行期覆盖路径（`run_method::apply_recorded_grafts` →
//!   `Registry::overlay_recorded`）也会消费它。
//!
//! Three words for three artefacts, and this file keeps them apart: the
//! **declaration** is the `static_graft_plan!` slot the host entry names, the
//! **record** is the `graft.plan` file this screen writes, and the **entry line** is
//! the `use <crate> as _;` import a plugin selection appends to the host entry
//! (`super::mutations`). Calling all three "the declaration" is how a reader ends up
//! editing the wrong file.
//! 三个词对应三种工件，本文件把它们分清楚：**声明**是宿主入口 `static_graft_plan!` 点名的槽位，
//! **记录**是本界面写下的 `graft.plan` 文件，**入口行**是插件选择追加到宿主入口的那条
//! `use <crate> as _;` 导入（`super::mutations`）。把三者都叫“声明”，正是读者改错文件的原因。

use super::project_context::package_root;
use super::write_guard::{selected_read_root, with_selected_project, with_selected_project_read};
use super::*;
use crate::build_method::{DeclaredGraft, DeclaredGrafts};

impl App {
    /// Open the graft screen for the selected face.
    /// 为当前选中的注册面打开 graft 界面。
    pub(super) fn open_graft(&mut self) {
        let Some(info) = self.selected_info() else {
            return;
        };
        let target = info.id;
        let target_path = self
            .registry
            .path_for(target)
            .unwrap_or_else(|| "root".to_owned());
        let selector = format!("{}_graft", info.registry_name);
        let flow_declared = info.flow.is_declared();
        let inherited_children = self
            .registry
            .registry(target)
            .map_or(0, |child| child.len());
        let mut state = GraftState {
            target,
            target_path,
            selector,
            full: false,
            field: 0,
            pane: 0,
            editing: false,
            plans: Vec::new(),
            plan_selected: 0,
            pending_delete: None,
            declaration: GraftDeclaration::Unknown {
                reason: "host entry not read yet".to_owned(),
            },
            flow_declared,
            inherited_children,
        };
        read_graft_state(&self.registry, &mut state);
        self.overlay = Some(Overlay::Graft(state));
    }

    /// Re-read the plans on disk and the entry's declarations.
    /// 重新读取磁盘上的计划与入口的声明。
    pub(super) fn refresh_graft(&mut self) {
        let Some(Overlay::Graft(state)) = self.overlay.as_mut() else {
            return;
        };
        read_graft_state(&self.registry, state);
    }

    /// Refresh the graft screen only when it is the open overlay.
    /// 仅当 graft 界面正打开时刷新它。
    pub(super) fn refresh_open_graft(&mut self) {
        if matches!(self.overlay, Some(Overlay::Graft(_))) {
            self.refresh_graft();
        }
    }

    /// Write the composed plan, or open the one that is already there.
    /// 写入撰写的计划，或打开已经存在的那一条。
    pub(super) fn submit_graft(&mut self, state: &GraftState) {
        if let Some(error) = graft_selector_error(&state.selector) {
            self.alert(format!("Graft failed: {error}"));
            return;
        }
        let selector = state.selector.trim();
        if let Some(existing) = state.plans.iter().find(|plan| plan.selector == selector) {
            if existing.is_valid()
                && existing.target_path == state.target_path
                && existing.full == state.full
            {
                self.open_graft_plan(selector);
                self.note(format!(
                    "External graft `{selector}` already declares `{}`; opened {}",
                    state.target_path,
                    plan_path_for(selector).display()
                ));
                return;
            }
            self.alert(format!(
                "Graft failed: `{selector}` already exists at {}; open it (o), delete it (d), or choose another selector",
                plan_path_for(selector).display()
            ));
            return;
        }
        let created = with_selected_project(|| {
            crate::run_method::create_external_graft(
                &self.registry,
                state.target,
                selector,
                state.full,
            )
        });
        match created {
            Ok(plan) => {
                let path = plan.plan_path();
                // The paste-ready line comes from the same kernel constructor and
                // renderer the written record goes through. Hand-formatting a copy
                // here is how the banner and the record start to disagree.
                // 可粘贴的那一行由写出的记录所用的同一个内核构造器与渲染器生成。在这里
                // 手写一份副本，正是横幅与记录开始不一致的原因。
                let declaration = plan.document.declaration();
                // A failed launch must survive the banner below: `event` is this
                // screen's only feedback channel, so assigning the banner straight
                // over it hid both the failure and the empty `editor_request`
                // (audit `LGC-LG-50`).
                // 启动失败必须活过下面的横幅：`event` 是本界面唯一的反馈通道，直接覆盖它会同时
                // 藏起失败与那次空的 `editor_request`（审计 `LGC-LG-50`）。
                let editor_failure = self.open_editor_file(path.clone(), 1).err();
                // The banner carries its own severity, arm by arm: the two arms whose
                // text says `Warning:` are alerts, while `Declared` is an ordinary
                // success — the plan was written and the slot was already declared. The
                // editor failure, when there is one, is an alert on top of either.
                // 横幅的严重度逐臂自带：文本写着 `Warning:` 的两臂是 alert，而 `Declared` 是普通的
                // 成功——计划写好了、槽位本来就已声明。编辑器失败（如果有）叠加在两者之上，是 alert。
                let (banner, banner_is_alert) = match &state.declaration {
                    GraftDeclaration::Absent { entry } => (
                        format!(
                            "Warning: External graft plan created at {}; the host entry {} declares no such slot, so the release prunes `{}` and the runtime skips this record (UnkeptSlot) instead of applying it. Add to static_graft_plan!:\n{declaration}",
                            path.display(),
                            entry.display(),
                            state.target_path,
                        ),
                        true,
                    ),
                    GraftDeclaration::Unknown { reason } => (
                        format!(
                            "Warning: External graft plan created at {}; the host entry could not be read ({reason}), so the release may prune `{}` and the runtime would skip this record (UnkeptSlot). Add to static_graft_plan!:\n{declaration}",
                            path.display(),
                            state.target_path,
                        ),
                        true,
                    ),
                    GraftDeclaration::Declared { line, .. } => (
                        format!(
                            "External graft plan created at {}; the host entry already declares this slot at line {line}:\n{declaration}",
                            path.display(),
                        ),
                        false,
                    ),
                };
                match (editor_failure, banner_is_alert) {
                    (Some(failure), _) => self.alert(format!("{failure}\n{banner}")),
                    (None, true) => self.alert(banner),
                    (None, false) => self.note(banner),
                };
            }
            Err(error) => self.alert(format!("Graft failed: {error}")),
        }
    }

    /// Open one plan file in the configured editor.
    /// 用配置的编辑器打开一个计划文件。
    pub(super) fn open_graft_plan(&mut self, selector: &str) {
        // A read of the authoring records goes through the selected project like a write
        // does: a plan the reader never opened must not be described as this screen's
        // state (audit `STU-S-29`).
        // 对创作记录的读取与写入一样走选中的项目：读者从未打开的计划不得被描述成本界面的状态
        // （审计 `STU-S-29`）。
        match with_selected_project_read(|| crate::run_method::read_external_graft(selector)) {
            Ok(plan) => {
                let path = plan.plan_path();
                // Per arm: opening the plan is ordinary, the editor failure is the alert
                // (it is the text `open_editor_file` produced).
                // 逐臂：打开计划是普通的，编辑器失败才是 alert（文本由 `open_editor_file` 给出）。
                match self.open_editor_file(path.clone(), 1) {
                    Ok(()) => self.note(format!("Opened external graft plan {}", path.display())),
                    // The failure is already the event; "Opened" would be a lie over it.
                    // 失败本身已是事件内容；在它之上再写 "Opened" 就是假话。
                    Err(failure) => self.alert(failure),
                };
            }
            Err(error) => self.alert(format!("Graft failed: {error}")),
        }
    }

    /// Switch one plan between node and subtree replacement.
    /// 在"只替换节点"与"替换整棵子树"之间切换一条计划。
    pub(super) fn toggle_graft_plan(&mut self, selector: &str, full: bool) {
        match with_selected_project(|| crate::run_method::rewrite_external_graft(selector, !full)) {
            Ok(plan) => {
                self.note(format!(
                    "External graft `{selector}` now {} `{}`",
                    if plan.full() {
                        "replaces"
                    } else {
                        "keeps the children of"
                    },
                    plan.target_path()
                ));
            }
            Err(error) => self.alert(format!("Graft failed: {error}")),
        }
    }

    /// Move one plan to the recoverable trash.
    /// 把一条计划移到可恢复的回收目录。
    pub(super) fn delete_graft_plan(&mut self, selector: &str) {
        match with_selected_project(|| crate::run_method::remove_external_graft(selector)) {
            Ok(trash) => {
                self.note(format!(
                    "External graft `{selector}` moved to {}; press r to reload",
                    trash.display()
                ));
            }
            Err(error) => self.alert(format!("Graft failed: {error}")),
        }
    }

    /// The plan an open/delete/toggle key should act on.
    /// 打开/删除/切换键应当作用的那条计划。
    ///
    /// The list pane acts on its highlighted row; the compose pane acts on the
    /// selector being typed, but only when a plan by that name already exists.
    /// 列表面板作用于高亮行；撰写面板作用于正在输入的选择器，但仅当同名计划已存在。
    pub(super) fn selected_graft_selector(&self, state: &GraftState) -> Option<String> {
        if state.pane == 1 {
            return state
                .plans
                .get(state.plan_selected)
                .map(|plan| plan.selector.clone());
        }
        let selector = state.selector.trim();
        state
            .plans
            .iter()
            .any(|plan| plan.selector == selector)
            .then(|| selector.to_owned())
    }
}

/// Read the plans and the entry declaration shown by one graft screen.
/// 读取一个 graft 界面显示的计划与入口声明。
fn read_graft_state(registry: &Registry, state: &mut GraftState) {
    state.plans = with_selected_project_read(crate::run_method::list_external_grafts)
        .unwrap_or_default()
        .into_iter()
        .map(|entry| match entry.document {
            Ok(document) => GraftPlanRow {
                selector: entry.selector,
                target_path: document.target_path,
                full: document.full,
                error: None,
            },
            Err(error) => GraftPlanRow {
                selector: entry.selector,
                target_path: String::new(),
                full: false,
                error: Some(error),
            },
        })
        .collect();
    state.plan_selected = state.plan_selected.min(state.plans.len().saturating_sub(1));
    if state.plans.is_empty() {
        state.pane = 0;
    }

    let module = registry
        .find_registry(state.target)
        .map(|info| crate::build_method::source_module_path(&info.source.file));
    // The declaration is read from the selected project for the same reason: the screen
    // says what the opened project declares, and a refused selection is said out loud in
    // the banner rather than replaced by a guess (audit `STU-S-29`).
    // 声明同样从选中的项目读取：本界面说的是打开的项目声明了什么，而被拒绝的选择要在横幅里说
    // 出来，而不是被一个猜测替换（审计 `STU-S-29`）。
    state.declaration = match selected_read_root() {
        Ok(root) => match crate::build_method::declared_grafts(&root) {
            Ok(declared) => declaration_for(&state.target_path, module.as_deref(), &declared),
            Err(reason) => GraftDeclaration::Unknown { reason },
        },
        Err(refusal) => GraftDeclaration::Unknown { reason: refusal },
    };
}

/// The declaration, if any, that names this face's slot.
/// 命名这个注册面槽位的声明（若有）。
///
/// The one cut/face matching rule lives in `DeclaredGraft::names_face`, the
/// same rule the build uses; its `module` is the face's source module, or
/// `None` when the target cannot be resolved in the current tree.
/// 唯一的切口/注册面匹配规则位于 `DeclaredGraft::names_face`，与构建使用的是同一条
/// 规则；其 `module` 是该注册面的源码模块，目标在当前树里解析不出来时为 `None`。
pub(super) fn declaration_for(
    path: &str,
    module: Option<&str>,
    declared: &DeclaredGrafts,
) -> GraftDeclaration {
    for cut in &declared.cuts {
        if cut.names_face(path, module) {
            return GraftDeclaration::Declared {
                expression: describe_graft(cut),
                line: cut.line,
                cfg: cut.cfg.clone(),
            };
        }
    }
    GraftDeclaration::Absent {
        entry: declared.entry.clone(),
    }
}

/// Render one declaration the way an author reads it.
/// 按作者阅读的方式渲染一条声明。
fn describe_graft(cut: &DeclaredGraft) -> String {
    match &cut.expressions {
        Some(expressions) => {
            let range = expressions
                .cut_end
                .as_deref()
                .map_or(String::new(), |end| format!(" to {end}"));
            format!(
                "cut({}{range}){} graft({})",
                expressions.cut,
                if cut.full { " full" } else { "" },
                expressions.graft
            )
        }
        None => match &cut.cut_end {
            // A range is rendered in the bracketed two-literal form the macro
            // grammar accepts, not as one quoted path: `cut "a to b"` is a
            // single path that literally contains the range word, so echoing the
            // endpoints as text would hand the author a clause that means
            // something else.
            // 区间渲染成宏语法接受的“两个带方括号字面量”形式，而不是一条带引号的路径：
            // `cut "a to b"` 是字面含有区间词的单条路径，把端点当文本回显会给作者一句
            // 含义不同的子句。
            Some(end) => format!(
                "cut [\"{}\" to \"{end}\"]{} graft \"{}\"",
                cut.cut,
                if cut.full { " full" } else { "" },
                cut.graft
            ),
            None => format!(
                "cut \"{}\"{} graft \"{}\"",
                cut.cut,
                if cut.full { " full" } else { "" },
                cut.graft
            ),
        },
    }
}

/// A selector the filesystem and the plan format both accept.
/// 文件系统与计划格式都能接受的选择器。
pub(super) fn graft_selector_error(selector: &str) -> Option<String> {
    crate::run_method::validate_graft_selector(selector.trim())
        .err()
        .map(|error| error.to_string())
}

/// Where a plan by this selector lives, in the same layout the writer uses.
/// 该选择器的计划所在位置，与写入方使用同一套布局。
fn plan_path_for(selector: &str) -> PathBuf {
    crate::run_method::graft_record_root(&package_root())
        .join(selector.trim())
        .join(crate::run_method::lexicon::GRAFT_PLAN_FILE)
}
