//! Add overlay keyboard handling: what a submit writes.
//! 添加浮层键盘处理：提交写到哪里。

use super::super::*;
use super::face_form::{FormAction, face_form_key};

impl App {
    pub(super) fn handle_add_overlay_key(&mut self, key: KeyEvent, mut add: AddState) {
        match face_form_key(key, &mut add) {
            FormAction::Close => {
                self.overlay = None;
                return;
            }
            FormAction::Submit => {
                self.submit_add(&add);
                return;
            }
            FormAction::Stay => {}
        }
        self.overlay = Some(Overlay::Add(add));
    }
}
