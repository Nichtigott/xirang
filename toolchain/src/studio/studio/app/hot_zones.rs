//! Click hot-zone caches refreshed by every Studio draw, and the pure interaction
//! helpers that read them: the divider maths and the tree selection the pointer and the
//! keys move.
//! 每次 Studio 绘制时刷新的点击热区缓存，以及读取它们的纯交互辅助：分隔条算术，与指针和按键
//! 移动的那份树选中。

use ratatui::layout::Rect;

use super::*;

/// The rectangles Studio hit-tests pointer events against.
/// Studio 用于命中测试指针事件的矩形集合。
///
/// Every draw recomputes these from the current frame, so they are one cache
/// rather than thirty independent fields on `App`. The rectangle names are
/// unchanged, so a read or write site only gains one `hot.` hop.
/// 每次绘制都会根据当前帧重新计算它们，因此它们是同一份缓存，而不是 `App` 上的
/// 三十个独立字段。矩形名称保持不变，读写点只多一跳 `hot.`。
#[derive(Clone, Debug, Default)]
pub struct HotZones {
    /// Click area of the registry-tree pane.
    /// 注册树面板的点击区域。
    pub tree_area: Rect,
    /// Click area of the selected face's detail pane.
    /// 所选注册面详情面板的点击区域。
    pub details_area: Rect,
    /// Click area of the whole workspace body, below the brand.
    /// 品牌之下整个工作区主体的点击区域。
    pub workspace_area: Rect,
    /// Click area of the open overlay's frame.
    /// 当前浮层外框的点击区域。
    pub overlay_area: Rect,
    /// Click rows of the overlay's primary list.
    /// 浮层主列表的点击行区域。
    pub overlay_list_area: Rect,
    /// Click target of the delete screen's Cancel button.
    /// 删除界面 Cancel 按钮的点击目标。
    pub delete_cancel_area: Rect,
    /// Click target of the delete screen's Confirm button.
    /// 删除界面 Confirm 按钮的点击目标。
    pub delete_confirm_area: Rect,
    /// Click target of the focused form's Cancel button.
    /// 当前表单 Cancel 按钮的点击目标。
    pub action_cancel_area: Rect,
    /// Click target of the focused form's Confirm button.
    /// 当前表单 Confirm 按钮的点击目标。
    pub action_confirm_area: Rect,
    /// Click target of the focused form's Exit or Close button.
    /// 当前表单 Exit/Close 按钮的点击目标。
    pub action_exit_area: Rect,
    /// Clickable compose rows of the external-graft screen.
    /// 外部 graft 界面可点击的撰写区行。
    pub graft_compose_area: Rect,
    /// Click area of the whole provenance-graph page.
    /// 整个溯源图页面的点击区域。
    pub graph_area: Rect,
    /// Click area of the graph page's detail pane.
    /// 调用图页详情面板的点击区域。
    pub graph_detail_area: Rect,
    /// Click area of the graph page's provenance pane.
    /// 调用图页溯源面板的点击区域。
    pub graph_provenance_area: Rect,
    /// Click area of the call-tree pane.
    /// 调用树面板的点击区域。
    pub graph_tree_area: Rect,
    /// Click area of the data-flow pane.
    /// 数据流面板的点击区域。
    pub graph_data_area: Rect,
}

impl App {
    pub(super) fn near_divider(&self, column: u16, row: u16) -> bool {
        self.hot.workspace_area.contains((column, row).into())
            && column.abs_diff(self.hot.tree_area.right()) <= 1
    }

    pub(super) fn near_graph_divider(&self, column: u16, row: u16) -> bool {
        self.hot.graph_area.contains((column, row).into())
            && column.abs_diff(self.hot.graph_tree_area.right()) <= 1
    }

    pub(super) fn resize_graph_split(&mut self, column: u16) {
        if self.hot.graph_area.width == 0 {
            return;
        }
        let relative = column.saturating_sub(self.hot.graph_area.x) as u32;
        self.graph_split_percent =
            ((relative * 100) / u32::from(self.hot.graph_area.width)).clamp(35, 65) as u16;
    }

    pub(super) fn resize_split(&mut self, column: u16) {
        if self.hot.workspace_area.width == 0 {
            return;
        }
        let relative = column.saturating_sub(self.hot.workspace_area.x) as u32;
        self.split_percent =
            ((relative * 100) / u32::from(self.hot.workspace_area.width)).clamp(25, 70) as u16;
    }

    pub(super) fn move_selection(&mut self, delta: isize) {
        let nodes = self.visible_nodes();
        let current = nodes
            .iter()
            .position(|(id, _)| *id == self.selected)
            .unwrap_or_default();
        let last = nodes.len().saturating_sub(1) as isize;
        let next = (current as isize + delta).clamp(0, last) as usize;
        if let Some((id, _)) = nodes.get(next)
            && self.selected != *id
        {
            self.selected = *id;
            self.details_selected = 0;
        }
    }

    pub(super) fn toggle_selected(&mut self) {
        if !self.owns_registry(self.selected) {
            return;
        }
        if !self.collapsed.insert(self.selected) {
            self.collapsed.remove(&self.selected);
        }
    }

    pub(super) fn selected_parent(&self) -> NodeId {
        self.selected_info()
            .filter(|info| info.needs_registry)
            .map(|info| info.id)
            .or_else(|| {
                self.selected_info()
                    .and_then(|info| self.registry.find_registry(info.parent))
                    .filter(|info| info.needs_registry)
                    .map(|info| info.id)
            })
            .unwrap_or(self.registry.id())
    }
}
