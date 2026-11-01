//! Edit overlay keyboard handling: what a submit rewrites, and for which node.
//! 编辑浮层键盘处理：提交重写什么、针对哪个节点。

use super::super::*;
use super::face_form::{FormAction, face_form_key};

impl App {
    pub(super) fn handle_edit_overlay_key(
        &mut self,
        key: KeyEvent,
        id: NodeId,
        mut edit: AddState,
    ) {
        match face_form_key(key, &mut edit) {
            FormAction::Close => {
                self.overlay = None;
                return;
            }
            FormAction::Submit => {
                self.submit_edit(id, &edit);
                return;
            }
            FormAction::Stay => {}
        }
        self.overlay = Some(Overlay::Edit(id, edit));
    }
}
