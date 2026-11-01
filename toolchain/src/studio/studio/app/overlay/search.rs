//! Search overlay keyboard handling.
//! 搜索浮层键盘处理。

use super::super::*;

impl App {
    pub(super) fn handle_search_overlay_key(&mut self, key: KeyEvent, mut search: SearchState) {
        if search.graph_mode {
            let Some(_center) = search.center else {
                if matches!(key.code, KeyCode::Tab | KeyCode::BackTab) {
                    if key.code == KeyCode::Tab {
                        advance_graph_focus(&mut search);
                    } else {
                        retreat_graph_focus(&mut search);
                    }
                }
                self.overlay = Some(Overlay::Search(search));
                return;
            };
            let tree_len = self
                .graph_item(&search)
                .map(|item| self.call_tree_targets(&item).len())
                .unwrap_or_default();
            let tree_cursor = search.outline_selected;
            let tree_item = self.graph_tree_item(&search, tree_cursor);
            let data_len = tree_item
                .as_ref()
                .map(|item| self.graph_locals(item).len())
                .unwrap_or_default();
            match key.code {
                KeyCode::Char('/') => {
                    self.page = StudioPage::Search;
                    search.graph_mode = false;
                    search.center = None;
                    search.center_function = None;
                    search.center_line = None;
                    search.outline_selected = 0;
                    search.data_selected = 0;
                }
                KeyCode::Char('m') => self.load_mir_snapshot_report(),
                KeyCode::Tab => advance_graph_focus(&mut search),
                KeyCode::BackTab => retreat_graph_focus(&mut search),
                // The one drawer runs the calls down the screen, so the arrows
                // mean what the drawing says: ↓ walks downstream (what this
                // calls), ↑ walks upstream (who calls this), and ←/→ move between
                // the lanes of one band.
                // 唯一的绘制方把调用沿屏幕向下排，因此方向键表示画面上写着的东西：↓ 走下游
                // （它调用了谁），↑ 走上游（谁在调它），←/→ 在同一条带的车道间移动。
                // `[`/`]` move the split.
                // `[`/`]` 移动分栏。
                KeyCode::Char('[') | KeyCode::Char(']') if search.graph_focus == 0 => {
                    self.shift_graph_split(key.code == KeyCode::Char('['))
                }
                // The arrows follow the picture: one step along the drawn grid, in
                // the tree or in the value pane, whichever holds the focus.
                // 方向键跟着图走：沿画出的网格走一步，在树里或在取值面板里，取决于谁持有焦点。
                KeyCode::Up | KeyCode::Down | KeyCode::Left | KeyCode::Right => {
                    self.step_graph_cursor(&mut search, key.code, tree_len, data_len)
                }
                KeyCode::Enter if search.graph_focus == 0 => {
                    if let Some(target) = self.graph_tree_item(&search, search.outline_selected) {
                        // Enter on a tree node means "now show me the tree around
                        // this one"; at the focus itself it opens the editor.
                        // 在树节点上按 Enter 表示"现在给我看围绕它的那棵树"；在焦点本身
                        // 上则打开编辑器。
                        if !self.recentre_call_tree(&mut search, target.clone()) {
                            let line = self.source_function_line(target.node, &target.function);
                            self.selected = target.node;
                            self.open_editor_at(target.node, line);
                            self.overlay = None;
                            return;
                        }
                    }
                }
                KeyCode::Enter if search.graph_focus == 1 => {
                    if let Some(item) = tree_item.as_ref() {
                        let locals = self.graph_locals(item);
                        if let Some(local) = locals.get(search.data_selected) {
                            // The refusal is already on the event line; there is
                            // nothing else for this arm to report.
                            // 拒绝本身已在事件行；这一支没有别的要报告。
                            let _ = self.open_editor_file(
                                source_path_for(local.source.file),
                                local.source.line,
                            );
                            self.overlay = None;
                            return;
                        }
                    }
                }
                KeyCode::Enter => {}
                KeyCode::Char('e') if search.graph_focus == 1 => {
                    if let Some(item) = tree_item.as_ref()
                        && let Some(local) = self.graph_locals(item).get(search.data_selected)
                    {
                        // The refusal is already on the event line; there is nothing
                        // else for this arm to report.
                        // 拒绝本身已在事件行；这一支没有别的要报告。
                        let _ = self.open_editor_file(
                            source_path_for(local.source.file),
                            local.source.line,
                        );
                        self.overlay = None;
                        return;
                    }
                }
                KeyCode::Char('e') if search.graph_focus == 0 => {
                    if let Some(item) = self.graph_tree_item(&search, search.outline_selected)
                        && let Some(line) = self.source_function_line(item.node, &item.function)
                    {
                        self.selected = item.node;
                        self.open_editor_at(item.node, Some(line));
                        return;
                    }
                }
                // No typing arm here on purpose. In this page the letters are
                // commands (`m`, `e`), and a catch-all typing arm would
                // shadow them — which is exactly what it did: `e` was listed after
                // it and could never fire. The query is edited in the list page,
                // which `/` returns to, so one mode owns the letters and the other
                // owns the text.
                // 这里有意不设输入分支。本页的字母是命令（`m`、`e`），而一个兜底的输入
                // 分支会遮蔽它们——事实正是如此：`e` 排在它后面，永远轮不到。查询在列表页编辑，
                // `/` 回到那里，因此一种模式拥有字母、另一种拥有文本。
                _ => {}
            }
            self.overlay = Some(Overlay::Search(search));
            return;
        }
        let active_query = &search.query;
        let active_selected = search.selected;
        match key.code {
            // No Left/Right/Space arm here: the old code toggled a fold set, but
            // `search_rows` returns a flat list where every row has no children,
            // so the arm could never match and the footer no longer advertises
            // it. Removed instead of implemented.
            // 这里没有 Left/Right/Space 分支：旧代码切换折叠集合，但
            // `search_rows` 返回的扁平列表里没有任何行有子节点，该分支永远匹配
            // 不上，页脚也不再展示它。选择删除而不是实现。
            KeyCode::Char(character) => {
                search.query.push(character);
                search.selected = 0;
                search.offset = 0;
            }
            KeyCode::Backspace => {
                search.query.pop();
                search.selected = 0;
                search.offset = 0;
            }
            KeyCode::Up => search.selected = search.selected.saturating_sub(1),
            KeyCode::Down => {
                let last = self.search_rows(active_query).len().saturating_sub(1);
                search.selected = (search.selected + 1).min(last);
            }
            KeyCode::Enter => {
                let row = self
                    .search_rows(active_query)
                    .get(active_selected)
                    .cloned()
                    .unwrap_or(SearchRow {
                        depth: 0,
                        node: None,
                        path: String::new(),
                        function: String::new(),
                        line: None,
                        signature: String::new(),
                        text: "No matching call row".to_owned(),
                    });
                if let Some(node) = row.node {
                    let function = (!row.function.is_empty()).then(|| row.function.clone());
                    search.center = Some(node);
                    search.center_function = function;
                    search.center_line = row.line;
                    search.graph_mode = true;
                    self.page = StudioPage::Data;
                    search.graph_focus = 0;
                    // The opened node is the tree's focus, i.e. its row zero, and
                    // the value pane starts at its first row.
                    // 打开的那个节点就是树的焦点，也就是第零行；取值面板从第一行开始。
                    search.outline_selected = 0;
                    search.data_selected = 0;
                    self.overlay = Some(Overlay::Search(search));
                } else {
                    self.note(format!("Selected {}", row.text));
                    self.overlay = Some(Overlay::Search(search));
                }
                return;
            }
            _ => {}
        }
        self.overlay = Some(Overlay::Search(search));
    }
}
