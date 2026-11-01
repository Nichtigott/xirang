//! Overlay router: centered modal sizing and per-overlay dispatch.
//! 浮层路由：居中模态尺寸计算与逐浮层分发。

use super::*;

/// Reset every modal hot zone, with the overlay's own frame.
/// 把所有模态热区复位，并写入浮层外框。
///
/// One implementation for both reset paths: the no-overlay arm and the just-sized
/// arm spelled the same fields twice, in two different orders, so a field added to
/// one of them could leave stale coordinates on the other (audit `STU-S-11`).
/// 两条复位路径共用一份实现：无浮层分支与“刚算好尺寸”分支过去把同样的字段写了两遍、顺序还
/// 不一样，因此只加在其中一个分支上的字段会给另一个分支留下跨帧残留坐标（审计 `STU-S-11`）。
fn reset_hot_zones(cache: &mut RenderCache, overlay_area: Rect) {
    cache.hot.overlay_area = overlay_area;
    cache.hot.overlay_list_area = Rect::default();
    cache.hot.delete_cancel_area = Rect::default();
    cache.hot.delete_confirm_area = Rect::default();
    cache.hot.action_cancel_area = Rect::default();
    cache.hot.action_confirm_area = Rect::default();
    cache.hot.action_exit_area = Rect::default();
    cache.hot.graft_compose_area = Rect::default();
    cache.hot.graph_area = Rect::default();
    cache.hot.graph_tree_area = Rect::default();
    cache.hot.graph_data_area = Rect::default();
    cache.hot.graph_detail_area = Rect::default();
    cache.hot.graph_provenance_area = Rect::default();
}

pub(super) fn draw_overlay(frame: &mut Frame<'_>, app: &App, cache: &mut RenderCache) {
    let Some(overlay) = app.overlay.clone() else {
        reset_hot_zones(cache, Rect::default());
        return;
    };
    let area = if matches!(overlay, Overlay::Search(ref search) if search.graph_mode) {
        centered(98, 92, frame.area())
    } else {
        centered(86, 78, frame.area())
    };
    reset_hot_zones(cache, area);
    frame.render_widget(Clear, area);
    frame.render_widget(Block::default().style(Style::default().bg(PANEL)), area);
    match overlay {
        Overlay::Search(search) => {
            let (list_area, offset) = draw_search(frame, area, app, cache, &search);
            cache.hot.overlay_list_area = list_area;
            // The list widget keeps its own viewport, so the frame has to hand the offset it
            // ended on back to the session — through the cache, because drawing must not
            // write `App` itself (audit `STU-S-04`).
            // 列表控件保留自己的视口，因此帧必须把它停下的偏移交还会话——经缓存交回，因为绘制不得
            // 自己写 `App`（审计 `STU-S-04`）。
            cache.search_offset = Some(offset);
        }
        Overlay::NewProject(project) => {
            let (list, cancel, confirm, exit) = draw_new_project(frame, area, &project);
            cache.hot.overlay_list_area = list;
            cache.hot.action_cancel_area = cancel;
            cache.hot.action_confirm_area = confirm;
            cache.hot.action_exit_area = exit;
        }
        Overlay::Add(add) => {
            let (list, cancel, confirm, exit) = draw_add(frame, area, &add);
            cache.hot.overlay_list_area = list;
            cache.hot.action_cancel_area = cancel;
            cache.hot.action_confirm_area = confirm;
            cache.hot.action_exit_area = exit;
        }
        Overlay::Edit(_, edit) => {
            let (list, cancel, confirm, exit) = draw_edit(frame, area, &edit);
            cache.hot.overlay_list_area = list;
            cache.hot.action_cancel_area = cancel;
            cache.hot.action_confirm_area = confirm;
            cache.hot.action_exit_area = exit;
        }
        Overlay::Plugin(plugin) => {
            let (list, cancel, confirm, exit) = draw_plugin(frame, area, &plugin);
            cache.hot.overlay_list_area = list;
            cache.hot.action_cancel_area = cancel;
            cache.hot.action_confirm_area = confirm;
            cache.hot.action_exit_area = exit;
        }
        Overlay::Graft(graft) => {
            let (list, cancel, confirm, exit, compose) = draw_graft(frame, area, &graft);
            cache.hot.overlay_list_area = list;
            cache.hot.action_cancel_area = cancel;
            cache.hot.action_confirm_area = confirm;
            cache.hot.action_exit_area = exit;
            cache.hot.graft_compose_area = compose;
        }
        Overlay::Partition(state) => {
            let (exit, confirm) = draw_partition(frame, area, &state);
            cache.hot.action_exit_area = exit;
            cache.hot.action_confirm_area = confirm;
        }
        Overlay::Delete(id) => {
            let (cancel, confirm) = draw_delete(frame, area, app, id);
            cache.hot.delete_cancel_area = cancel;
            cache.hot.delete_confirm_area = confirm;
        }
    }
}
