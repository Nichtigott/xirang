//! Runtime call frames, observed data edges, and locals.
//! 运行时调用帧、已观测数据边与局部值。

#[path = "snapshot/snapshot.rs"]
pub mod snapshot;
// The trace document's module was `artifact`; hosts that write
// `crate::run_method::runtime::trace::artifact::…` keep compiling through this alias
// (audit `NAM-11`). It is pinned in `conventions/src/shims.rs`.
// trace 文档的模块曾叫 `artifact`；写着 `crate::run_method::runtime::trace::artifact::…`
// 的宿主通过这条别名继续编译（审计 `NAM-11`）。它钉在 `conventions/src/shims.rs` 里。
pub use self::snapshot as artifact;
#[path = "call_trace.rs"]
mod call_trace;
#[path = "edges.rs"]
pub mod edges;
#[path = "frames.rs"]
pub mod frames;
#[path = "locals/locals.rs"]
pub mod locals;

use crate::run_method::registry_core::declaration::SourceLocation;
use crate::run_method::registry_core::identity::NodeId;

pub use frames::FramePath;
pub use xirang_kernel::CallSite;
pub use xirang_kernel::declaration::source_file_matches;

pub use self::call_trace::*;
pub use self::snapshot::*;

pub use xirang_kernel::TraceMode;
