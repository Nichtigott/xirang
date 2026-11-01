//! Overlay keyboard state machine.
//! 浮层键盘状态机。
//!
//! This parent owns the shared Escape handling and dispatches one function per
//! `Overlay` variant; each variant's keys live in its own file under `overlay/`.
//! 本父模块负责共用的 Escape 处理，并为每个 `Overlay` 变体分派一个函数；
//! 各变体的按键逻辑位于 `overlay/` 下各自的文件中。

#[path = "overlay/add.rs"]
mod add;
#[path = "overlay/delete.rs"]
mod delete;
#[path = "overlay/edit.rs"]
mod edit;
// The one transition table the add and edit overlays both run.
// 添加与编辑两个浮层共用的唯一一份转移表。
#[path = "overlay/face_form.rs"]
mod face_form;
#[path = "overlay/graft.rs"]
mod graft;
#[path = "overlay/new_project.rs"]
mod new_project;
#[path = "overlay/partition.rs"]
mod partition;
#[path = "overlay/plugin.rs"]
mod plugin;
#[path = "overlay/search.rs"]
mod search;

use super::*;

impl App {
    pub(super) fn handle_overlay_key(&mut self, key: KeyEvent) {
        if key.code == KeyCode::Esc {
            if let Some(Overlay::Search(search)) = self.overlay.as_mut()
                && search.graph_mode
            {
                search.graph_mode = false;
                search.center = None;
                search.center_line = None;
                search.outline_selected = 0;
                self.page = StudioPage::Search;
                return;
            }
            self.overlay = None;
            self.page = StudioPage::Inspect;
            return;
        }
        let overlay = self.overlay.take().expect("overlay exists");
        match overlay {
            Overlay::Search(search) => self.handle_search_overlay_key(key, search),
            Overlay::NewProject(project) => self.handle_new_project_overlay_key(key, project),
            Overlay::Add(add) => self.handle_add_overlay_key(key, add),
            Overlay::Edit(id, edit) => self.handle_edit_overlay_key(key, id, edit),
            Overlay::Plugin(plugin) => self.handle_plugin_overlay_key(key, plugin),
            Overlay::Graft(graft) => self.handle_graft_overlay_key(key, graft),
            Overlay::Delete(id) => self.handle_delete_overlay_key(key, id),
            Overlay::Partition(state) => self.handle_partition_overlay_key(key, state),
        }
    }
}
