//! External graft-plan authoring.
//! 外部 graft 计划创作。

#[path = "plan.rs"]
mod plan;

pub use plan::{
    ExternalGraftPlanEntry, ExternalGraftPlanFile, create_external_graft, external_graft_root,
    list_external_grafts, read_external_graft, remove_external_graft, rewrite_external_graft,
};
