//! Authoring view of the graft declarations a host entry carries.
//! 宿主入口携带的 graft 声明的创作视图。
//!
//! The build captures these declarations into the static plan, while an
//! authoring surface reads them to tell an author whether a slot is shipped.
//! Both read one conversion and one module-matching rule from here.
//! 构建把这些声明捕获进静态计划，创作界面则读取它们以告诉作者某个槽位是否被发布。
//! 两者都从这里读取同一份转换与同一条模块匹配规则。
//!
//! This file only mounts the view types (`declared`), the entry reader
//! (`query`) and the matching helpers (`matching`), then re-exports the surface
//! callers already use, so every path stays where it was.
//! 本文件只挂载视图类型（`declared`）、入口读取器（`query`）与匹配辅助
//! （`matching`），并重新导出调用方已在使用的表面，使每个路径保持在原处。

#[path = "graft_view/declared.rs"]
mod declared;
#[path = "graft_view/matching.rs"]
mod matching;
#[path = "graft_view/overlay_rows.rs"]
mod overlay_rows;
#[path = "graft_view/plan_rows.rs"]
mod plan_rows;
#[path = "graft_view/query.rs"]
mod query;

pub(crate) use declared::graft_cut_label;
pub use declared::{DeclaredGraft, DeclaredGraftExpressions, DeclaredGrafts};
pub(crate) use matching::{face_declares_plugin, graft_expression_module, string_cut_modules};
pub use overlay_rows::{
    OVERLAY_NOTE, OverlayProjection, OverlaySlot, SlotBinding, overlay_projection, slot_bindings,
};
pub use plan_rows::{GraftPlanRow, graft_plan_rows};
pub use query::declared_grafts;
pub(crate) use query::{declared_graft_view, enable_graft_entries, host_graft_entries};
