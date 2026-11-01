//! The editor handoff: turning a registry node or a runtime location into the request the
//! event loop opens in the configured editor.
//! 编辑器交接：把一个注册节点或一个运行期位置，变成事件循环要在配置好的编辑器里打开的那次请求。

use std::path::PathBuf;

use super::*;

impl App {
    pub(super) fn open_editor_at(&mut self, node: NodeId, line: Option<u32>) {
        let Some(info) = self.registry.find_registry(node) else {
            return;
        };
        let path = source_path_for(&info.source.file);
        if !path.is_file() {
            self.alert(format!(
                "Editor failed: source file not found at {}",
                path.display()
            ));
            return;
        }
        self.editor_request = Some((path, line.unwrap_or(info.source.line)));
    }

    /// Open an arbitrary runtime source location, including a local value.
    /// 打开任意运行时源码位置，包括局部变量位置。
    ///
    /// The failure is returned as well as written to `event`: `event` is the only
    /// feedback channel this screen has, so a caller that shows a success banner
    /// right afterwards has to be able to keep the failure instead of assigning
    /// over it (audit `LGC-LG-50`).
    /// 失败既写进 `event` 也作为返回值交回：`event` 是本界面唯一的反馈通道，因此紧接着要显示
    /// 成功横幅的调用方必须能保住这条失败，而不是把它覆盖掉（审计 `LGC-LG-50`）。
    pub(super) fn open_editor_file(&mut self, path: PathBuf, line: u32) -> Result<(), String> {
        if !path.is_file() {
            let failure = format!("Editor failed: source file not found at {}", path.display());
            self.alert(failure.clone());
            return Err(failure);
        }
        self.editor_request = Some((path, line.max(1)));
        Ok(())
    }

    /// Take the pending editor handoff, leaving none queued behind it.
    /// 取走待处理的编辑器交接请求，取走后不再有排队项。
    ///
    /// Returns the file and the 1-based line to open; `None` when nothing is pending.
    /// 返回要打开的文件与从 1 开始的行号；没有待处理请求时为 `None`。
    pub fn take_editor_request(&mut self) -> Option<(PathBuf, u32)> {
        self.editor_request.take()
    }
}
