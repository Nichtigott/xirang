//! Pointer and overlay-click interaction.
//! 指针与浮层点击交互。

use super::*;

impl App {
    pub(super) fn handle_mouse(&mut self, kind: MouseEventKind, column: u16, row: u16) {
        if self.overlay.is_some() {
            match kind {
                MouseEventKind::ScrollUp | MouseEventKind::ScrollDown => {
                    let up = matches!(kind, MouseEventKind::ScrollUp);
                    let graph = matches!(self.overlay, Some(Overlay::Search(ref search)) if search.graph_mode);
                    if graph {
                        // The widget's own gesture first: over its tree the wheel
                        // zooms, which is what a node editor does. Everywhere else
                        // the wheel steps the cursor of whichever pane it is over.
                        // 先给控件自己的手势：在它的树上滚轮是缩放，节点编辑器都这么做。其余位置
                        // 滚轮使它所在那块面板的游标走一步。
                        #[cfg(feature = "node-graph")]
                        if self.forward_mouse_to_flow(kind, column, row) {
                            return;
                        }
                        let point = (column, row).into();
                        let tree = self.hot.graph_tree_area.contains(point);
                        let data = self.hot.graph_data_area.contains(point);
                        if let Some(Overlay::Search(search)) = self.overlay.as_mut() {
                            if tree {
                                search.graph_focus = 0;
                                search.outline_selected = if up {
                                    search.outline_selected.saturating_sub(1)
                                } else {
                                    search.outline_selected.saturating_add(1)
                                };
                            } else if data {
                                search.graph_focus = 1;
                                search.data_selected = if up {
                                    search.data_selected.saturating_sub(1)
                                } else {
                                    search.data_selected.saturating_add(1)
                                };
                            }
                            search.outline_focus = search.graph_focus == 0;
                        }
                    } else {
                        self.handle_overlay_key(KeyEvent::from(if up {
                            KeyCode::Up
                        } else {
                            KeyCode::Down
                        }));
                    }
                }
                MouseEventKind::Down(MouseButton::Left) if self.near_graph_divider(column, row) => {
                    self.graph_dragging_divider = true;
                    self.resize_graph_split(column);
                }
                MouseEventKind::Drag(MouseButton::Left) if self.graph_dragging_divider => {
                    self.resize_graph_split(column)
                }
                MouseEventKind::Up(MouseButton::Left) => {
                    // The widget reports a node click on release, so the release
                    // has to reach it; forwarding only the press left every click
                    // half-finished in the widget's internal drag state.
                    // 控件在松开时报告节点点击，因此松开事件必须到达它；只转发按下会让每次点击都
                    // 停在控件内部的拖拽状态里。
                    #[cfg(feature = "node-graph")]
                    self.forward_mouse_to_flow(kind, column, row);
                    self.graph_dragging_divider = false;
                }
                MouseEventKind::Down(MouseButton::Left) => self.handle_overlay_click(column, row),
                _ => {}
            }
            return;
        }
        match kind {
            MouseEventKind::ScrollUp => {
                if self.hot.details_area.contains((column, row).into()) {
                    self.focus = Focus::Details;
                    self.details_selected = self.details_selected.saturating_sub(1);
                } else {
                    self.focus = Focus::Tree;
                    self.move_selection(-1);
                }
            }
            MouseEventKind::ScrollDown => {
                if self.hot.details_area.contains((column, row).into()) {
                    self.focus = Focus::Details;
                    self.details_selected = (self.details_selected + 1)
                        .min(self.detail_field_count().saturating_sub(1));
                } else {
                    self.focus = Focus::Tree;
                    self.move_selection(1);
                }
            }
            MouseEventKind::Down(MouseButton::Left) if self.near_divider(column, row) => {
                self.dragging_divider = true;
                self.resize_split(column);
            }
            MouseEventKind::Down(MouseButton::Left)
                if self.hot.tree_area.contains((column, row).into()) =>
            {
                let index = self.tree_offset
                    + row.saturating_sub(self.hot.tree_area.y.saturating_add(1)) as usize;
                if let Some((id, _)) = self.visible_nodes().get(index) {
                    let clicked = *id;
                    if clicked == self.selected {
                        self.toggle_selected();
                    } else {
                        self.selected = clicked;
                        self.details_selected = 0;
                    }
                }
            }
            MouseEventKind::Down(MouseButton::Left)
                if self.hot.details_area.contains((column, row).into()) =>
            {
                self.focus = Focus::Details;
                let index = row.saturating_sub(self.hot.details_area.y.saturating_add(1)) as usize;
                self.details_selected = index.min(self.detail_field_count().saturating_sub(1));
            }
            MouseEventKind::Drag(MouseButton::Left) if self.dragging_divider => {
                self.resize_split(column)
            }
            MouseEventKind::Up(MouseButton::Left) => self.dragging_divider = false,
            _ => {}
        }
    }

    /// Hand a mouse event to the call tree's widget, and turn the widget's own
    /// `NodeClicked` into a cursor move.
    /// 把鼠标事件交给调用树的控件，并把控件自己的 `NodeClicked` 变成游标移动。
    ///
    /// The widget owns its viewport, so pan and zoom are its gestures, not ours;
    /// what stays ours is what the cursor *means*, which is why a click comes
    /// back as an event instead of moving the widget's selection directly.
    /// 控件拥有自己的视口，因此平移与缩放是它的手势而不是我们的；仍然属于我们的是"游标意味着
    /// 什么"，这正是点击以事件形式回来、而不是直接移动控件选中项的原因。
    #[cfg(feature = "node-graph")]
    fn forward_mouse_to_flow(&mut self, kind: MouseEventKind, column: u16, row: u16) -> bool {
        let Some(Overlay::Search(search)) = self.overlay.as_ref() else {
            return false;
        };
        if !search.graph_mode || !self.hot.graph_tree_area.contains((column, row).into()) {
            return false;
        }
        let event = crossterm::event::MouseEvent {
            kind,
            column,
            row,
            modifiers: crossterm::event::KeyModifiers::NONE,
        };
        let Some((_, _, flow)) = self.graph_flow.as_mut() else {
            return false;
        };
        let clicked = flow
            .handle_mouse_event(event)
            .into_events()
            .filter_map(|event| match event {
                rataflow::FlowEvent::NodeClicked { node_id } => node_id.parse::<usize>().ok(),
                _ => None,
            })
            .next_back();
        if let Some(index) = clicked
            && let Some(Overlay::Search(search)) = self.overlay.as_mut()
        {
            search.graph_focus = 0;
            search.outline_focus = true;
            search.outline_selected = index;
        }
        true
    }

    pub(super) fn handle_overlay_click(&mut self, column: u16, row: u16) {
        let point = (column, row).into();
        if !self.hot.overlay_area.contains(point) {
            self.overlay = None;
            return;
        }
        if self.hot.delete_cancel_area.contains(point) {
            self.overlay = None;
            return;
        }
        if self.hot.delete_confirm_area.contains(point) {
            self.handle_overlay_key(KeyEvent::from(KeyCode::Enter));
            return;
        }
        // The widget's own gesture first: when it draws the tree, a click on a
        // node is the widget's event to report, and it knows which node.
        // 先给控件自己的手势：当它绘制这棵树时，落在节点上的点击是它要报告的事件，而它知道是哪个
        // 节点。
        #[cfg(feature = "node-graph")]
        if self.forward_mouse_to_flow(MouseEventKind::Down(MouseButton::Left), column, row) {
            return;
        }
        // The graph page has two panes: the call tree, and the values the tree
        // cursor's function ran with. A click lands on the box or the row the
        // reader aimed at, and focuses the pane it landed in.
        // 调用图页有两块面板：调用树，以及树游标所在函数运行时的取值。点击落在读者瞄准的盒子
        // 或那一行上，并把焦点给它落进的那块面板。
        if let Some(Overlay::Search(search)) = self.overlay.as_ref()
            && search.graph_mode
        {
            if self.hot.graph_tree_area.contains(point) {
                // A click that the widget did not claim still gives the tree the
                // focus; which node is under the pointer is the widget's answer,
                // reported above as `NodeClicked`.
                // 控件没有认领的点击仍把焦点交给树；指针下是哪个节点由控件回答，在上面的
                // `NodeClicked` 里报告。
                if let Some(Overlay::Search(search)) = self.overlay.as_mut() {
                    search.graph_focus = 0;
                    search.outline_focus = true;
                }
                return;
            }
            if self.hot.graph_data_area.contains(point) {
                let top = self.hot.graph_data_area.y;
                let raw_selected = row.saturating_sub(top.saturating_add(1)) as usize;
                let selected = self
                    .graph_tree_item(search, search.outline_selected)
                    .map(|item| raw_selected.min(self.graph_locals(&item).len().saturating_sub(1)))
                    .unwrap_or_default();
                if let Some(Overlay::Search(search)) = self.overlay.as_mut() {
                    search.graph_focus = 1;
                    search.outline_focus = false;
                    search.data_selected = selected;
                }
                return;
            }
        }
        // The graft screen has two clickable panes: the compose rows focus a
        // field, the plan list selects a plan.
        // graft 界面有两个可点击面板：撰写区聚焦某一行，计划列表选中一条计划。
        if matches!(self.overlay, Some(Overlay::Graft(_))) {
            if self.hot.graft_compose_area.contains(point) {
                let field = row.saturating_sub(self.hot.graft_compose_area.y) as usize;
                if let Some(Overlay::Graft(graft)) = self.overlay.as_mut() {
                    graft.pane = 0;
                    graft.field = field.min(1);
                }
                return;
            }
            if self.hot.overlay_list_area.contains(point) {
                let index =
                    row.saturating_sub(self.hot.overlay_list_area.y.saturating_add(1)) as usize;
                if let Some(Overlay::Graft(graft)) = self.overlay.as_mut()
                    && index < graft.plans.len()
                {
                    graft.pane = 1;
                    graft.plan_selected = index;
                }
                return;
            }
        }
        if self.hot.action_cancel_area.contains(point) || self.hot.action_exit_area.contains(point)
        {
            self.overlay = None;
            return;
        }
        if self.hot.action_confirm_area.contains(point) {
            // A button click is a submit action even while a text field owns
            // keyboard input. Routing it through `s` used to type into the
            // field instead of saving.
            // 鼠标确认始终表示提交；不能伪装成字符 `s`，否则编辑字段会吞掉保存操作。
            match self.overlay.clone() {
                Some(Overlay::NewProject(project)) => self.submit_new_project(&project),
                Some(Overlay::Add(add)) => self.submit_add(&add),
                Some(Overlay::Edit(id, edit)) => self.submit_edit(id, &edit),
                Some(Overlay::Plugin(plugin)) => self.submit_plugin(&plugin),
                Some(Overlay::Graft(graft)) => {
                    self.submit_graft(&graft);
                    self.refresh_graft();
                }
                _ => {}
            }
            return;
        }
        if !self.hot.overlay_list_area.contains(point) {
            return;
        }
        let visible_row =
            row.saturating_sub(self.hot.overlay_list_area.y.saturating_add(1)) as usize;
        let search_target = match self.overlay.as_ref() {
            Some(Overlay::Search(search)) => {
                let last = self.search_rows(&search.query).len().saturating_sub(1);
                Some((search.offset + visible_row).min(last))
            }
            _ => None,
        };
        match self.overlay.as_mut() {
            Some(Overlay::Search(search)) => {
                search.selected = search_target.unwrap_or_default();
            }
            Some(Overlay::NewProject(project)) if visible_row < project.values.len() => {
                project.field = visible_row;
                if visible_row == new_project_field::KIND {
                    project.values[new_project_field::KIND] =
                        if project.values[new_project_field::KIND] == "binary" {
                            "library".to_owned()
                        } else {
                            "binary".to_owned()
                        };
                } else {
                    project.editing = true;
                }
            }
            Some(Overlay::Add(add)) => {
                let fields = face_field_indices();
                let offset = face_form_offset(add, self.hot.overlay_list_area.height);
                let Some(field) = fields.get(offset + visible_row).copied() else {
                    return;
                };
                add.field = field;
                if field == face_field::NEEDS_REGISTRY {
                    add.values[face_field::NEEDS_REGISTRY] =
                        (add.values[face_field::NEEDS_REGISTRY] != "true").to_string();
                } else if add.is_editable(field) {
                    add.editing = true;
                }
            }
            Some(Overlay::Edit(_, edit)) => {
                let fields = face_field_indices();
                let offset = face_form_offset(edit, self.hot.overlay_list_area.height);
                let Some(field) = fields.get(offset + visible_row).copied() else {
                    return;
                };
                edit.field = field;
                if field == face_field::NEEDS_REGISTRY {
                    edit.values[face_field::NEEDS_REGISTRY] =
                        (edit.values[face_field::NEEDS_REGISTRY] != "true").to_string();
                } else if edit.is_editable(field) {
                    edit.editing = true;
                }
            }
            Some(Overlay::Plugin(plugin)) if visible_row < plugin.values.len() => {
                plugin.field = visible_row;
                if visible_row == plugin_field::SOURCE || visible_row == plugin_field::MODE {
                    plugin.values[visible_row] = match visible_row {
                        plugin_field::SOURCE
                            if plugin.values[plugin_field::SOURCE] == "official" =>
                        {
                            "user".to_owned()
                        }
                        plugin_field::SOURCE => "official".to_owned(),
                        plugin_field::MODE if plugin.values[plugin_field::MODE] == "extension" => {
                            "replacement".to_owned()
                        }
                        _ => "extension".to_owned(),
                    };
                } else {
                    plugin.editing = true;
                }
            }
            _ => {}
        }
    }
}

fn face_form_offset(form: &AddState, height: u16) -> usize {
    let fields = face_field_indices();
    let selected = fields
        .iter()
        .position(|field| *field == form.field)
        .unwrap_or_default();
    let visible = height.saturating_sub(2) as usize;
    selected.saturating_sub(visible.saturating_sub(1))
}
