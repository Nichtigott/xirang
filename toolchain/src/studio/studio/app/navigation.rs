//! Graph focus navigation: which pane holds the focus, and how the cursor moves
//! through the drawn tree.
//! 调用图焦点导航：哪块面板持有焦点，以及游标如何穿过画出的树。
//!
//! The source stamp that decides *when* the app rebuilds lives next door in
//! `super::source_stamp`; sharing one file is what made the stamp look like part of
//! navigation (audit `STU-S-26`).
//! 决定应用**何时**重建的源码戳住在隔壁的 `super::source_stamp`；两者共用一个文件，正是让戳
//! 看起来属于导航的原因（审计 `STU-S-26`）。

use super::*;
use crate::studio::studio::app::call_tree_queries::TreeStep;
/// Widen or narrow the call tree's share of the page, one step at a time and
/// within the same range the divider drag uses, so the key and the mouse cannot
/// disagree about how wide the tree may get.
/// 加宽或收窄调用树在页面上所占的份额：每次一步，且与拖动分隔线使用同一范围，因此按键与鼠标
/// 不会对"树最多能多宽"各说一套。
impl App {
    pub(super) fn shift_graph_split(&mut self, narrower: bool) {
        let step: i16 = if narrower { -4 } else { 4 };
        self.graph_split_percent = (self.graph_split_percent as i16 + step)
            .clamp(35, 65)
            .unsigned_abs();
        self.note(format!(
            "tree share {}% — [ narrows it, ] widens it",
            self.graph_split_percent
        ));
    }
}

/// One arrow press in the graph page: the tree's cursor when the tree holds the
/// focus, the value pane's own cursor when it does not.
/// 调用图页的一次方向键：树持有焦点时移动树的游标，否则移动取值面板自己的游标。
impl App {
    pub(super) fn step_graph_cursor(
        &mut self,
        search: &mut SearchState,
        key: crossterm::event::KeyCode,
        tree_len: usize,
        data_len: usize,
    ) {
        use crossterm::event::KeyCode;
        if search.graph_focus != 0 {
            // The value pane is a list, so only its two ends are directions.
            // 取值面板是一个列表，因此只有上下两个方向。
            match key {
                KeyCode::Up => search.data_selected = search.data_selected.saturating_sub(1),
                KeyCode::Down => {
                    search.data_selected =
                        (search.data_selected + 1).min(data_len.saturating_sub(1));
                }
                _ => {}
            }
            return;
        }
        let Some(step) = TreeStep::from_key(key) else {
            // The four arrows are this page's whole cursor vocabulary; any other key is
            // refused here rather than silently read as "right" (audit `STU-S-17`).
            // 四个方向键就是本页游标的全部词汇；其他键在这里被拒绝，而不是被静默读成“右”
            // （审计 `STU-S-17`）。
            return;
        };
        self.hop_call_tree(search, step);
        // A tree that does not answer the step reports that and keeps its cursor;
        // the clamp is for a cursor left past the end by a re-centred tree.
        // 树若回答不了这一步会说明并保持游标；这里的钳制是给"重新居中后游标越界"用的。
        search.outline_selected = search.outline_selected.min(tree_len.saturating_sub(1));
    }
}

pub(super) fn advance_graph_focus(search: &mut SearchState) {
    // Two panes: the tree, then the values its cursor's function ran with.
    // 两块面板：调用树，然后是它游标所在函数运行时的取值。
    search.graph_focus = match search.graph_focus {
        0 => 1,
        _ => 0,
    };
    search.outline_focus = search.graph_focus == 0;
}

pub(super) fn retreat_graph_focus(search: &mut SearchState) {
    // BackTab lands on the same toggle: with two panes, "the other one" is the same
    // destination whichever key asked for it, so both share one body and the second
    // name exists only to keep the keyboard mapping readable.
    // BackTab 落回同一个开关：只有两块面板时，"另一块"无论哪个键来问都是同一个目的地，因此两者
    // 共用一个函数体，第二个名字只为让键盘映射读起来清楚。
    advance_graph_focus(search);
}
