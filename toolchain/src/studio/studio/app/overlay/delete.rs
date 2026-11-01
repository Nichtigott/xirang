//! Delete overlay keyboard handling.
//! 删除浮层键盘处理。

use super::super::*;

impl App {
    pub(super) fn handle_delete_overlay_key(&mut self, key: KeyEvent, id: NodeId) {
        match key.code {
            KeyCode::Char('y') | KeyCode::Enter => {
                let spec = format!("{id} confirm");
                // Every authoring call establishes the context that names the
                // project Studio has open; this one did not, so `delete_module`
                // resolved its root through the environment, the process CWD or
                // this crate's manifest directory instead — and moved a module out
                // of whichever project that happened to be. Deleting a module is
                // the last operation that may act on a guessed root.
                // 每个创作调用都会建立"Studio 打开的是哪个项目"这一上下文；只有这一处没有，于是
                // `delete_module` 改为经环境变量、进程 CWD 或本 crate 的清单目录解析根路径——并把
                // 模块从那个恰好命中的项目里搬走。删除模块是最不能靠猜根路径的操作。
                let result = super::super::write_guard::with_selected_project(|| {
                    crate::run_method::delete_module(&self.registry, &spec)
                });
                // Per arm: a deleted module is ordinary; a refused delete is the alert
                // (audit `STU-S-18`).
                // 逐臂：模块删掉了是普通的；删除被拒才是 alert（审计 `STU-S-18`）。
                match result {
                    Ok(change) => self.note(format!("{}; press r to reload", change.message)),
                    Err(error) => self.alert(format!("Delete failed: {error}")),
                };
            }
            KeyCode::Char('n') => {}
            _ => self.overlay = Some(Overlay::Delete(id)),
        }
    }
}
