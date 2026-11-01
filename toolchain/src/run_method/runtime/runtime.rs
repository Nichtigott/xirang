//! Observed call edges and evidence shared by tracing and tooling.
//! 追踪与工具共用的已观测调用边与证据。

#[path = "evidence.rs"]
mod evidence;
#[path = "graft_record.rs"]
mod graft_record;
#[path = "trace/trace.rs"]
pub mod trace;

pub use xirang_kernel::{
    COORDINATES_IN_VIEWPORT, Coordinates, FINITE_NUMBER, NON_EMPTY_TEXT, Provenance,
    ProvenanceStep, RuntimeCheckFailure, RuntimeCheckSpec, RuntimeValue,
};

pub use self::evidence::*;
pub use self::graft_record::*;
