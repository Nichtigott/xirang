//! App keyboard, pointer, overlay, and selection interaction.
//! App 键盘、指针、浮层与选择交互。

use super::*;

impl App {
    /// Dispatch one terminal event to the active overlay or the base key and pointer maps.
    /// 把一个终端事件分派给当前浮层，或基础键盘与指针映射。
    ///
    /// The caller reads events; quit intent is recorded in `should_quit` for the loop.
    /// 调用方负责读取事件；退出意图记入 `should_quit`，由事件循环检查。
    pub fn handle_app(&mut self, event: Event) {
        match event {
            Event::Key(key) if key.kind == crossterm::event::KeyEventKind::Press => {
                self.handle_key(key)
            }
            Event::Mouse(mouse) => self.handle_mouse(mouse.kind, mouse.column, mouse.row),
            Event::Paste(text) => self.handle_paste(text),
            Event::Resize(_, _) | Event::FocusGained | Event::FocusLost => {}
            Event::Key(_) => {}
        }
    }

    /// Type a paste into the open form.
    /// 把一次粘贴键入到打开的表单里。
    ///
    /// A paste is typing: its characters go through the same key dispatcher a key press
    /// does, so a field accepts them exactly the way it accepts typing (audit `STU-S-28`).
    /// With no overlay open there is no field to paste into, and the text is dropped
    /// instead of being replayed as keys — a pasted `a`, `b` or `q` would otherwise open
    /// a form, start a build, or ask the session to quit.
    /// 粘贴就是键入：它的字符走按键用的同一个分派器，因此字段接受它的方式与接受键入完全一致（审计
    /// `STU-S-28`）。没有浮层打开时没有字段可粘贴，文本被丢弃而不是当作按键回放——否则粘进来的
    /// `a`、`b` 或 `q` 会打开表单、触发构建或让会话退出。
    pub(super) fn handle_paste(&mut self, text: String) {
        if self.overlay.is_none() {
            return;
        }
        for character in text.chars() {
            self.handle_key(KeyEvent::from(KeyCode::Char(character)));
        }
    }
}
