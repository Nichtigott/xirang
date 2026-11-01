//! Optional runtime evidence model.
//!
//! `EvidenceKind` is re-exported from core and is the single evidence type
//! used by MIR, live traces, source scanners, graph adapters, and Studio.
//! MIR and source candidates are informative only; `Live` is the sole
//! confirmed execution evidence. Locals marked `unobserved` have no runtime
//! value and must not be treated as observed data.
//! 可选的运行期证据模型。
//!
//! `EvidenceKind` 从 core 再导出，是 MIR、实时 trace、源码扫描、图适配器与 Studio
//! 共用的唯一证据类型。MIR 与源码候选只是提示；只有 `Live` 是已确认的执行证据。
//! 标记为 `unobserved` 的局部值没有运行期取值，不得当作已观测数据。

// The published surface must be readable on docs.rs without leaving the page,
// so the lint is on for the whole crate; `clippy -D warnings` makes a new
// undocumented public item a failure.
// 发布表面必须能在 docs.rs 上不跳页读懂，因此 lint 开在整个 crate 上；
// `clippy -D warnings` 会让新增的、没有文档的公开项变成失败。

pub mod adapters;
pub mod collector;
pub mod mir;

#[doc(hidden)]
pub use inventory;

/// Local inventory payload avoids orphan-rule coupling to core's declaration
/// type while keeping the collected value zero-copy.
/// 本地的 inventory 载荷避免与内核声明类型产生孤儿规则耦合，同时让收集到的值保持零拷贝。
#[doc(hidden)]
pub struct CollectedRegistration(pub &'static crate::run_method::RegistrationInfo);

#[cfg(debug_assertions)]
inventory::collect!(CollectedRegistration);

pub use crate::run_method::{
    CallEdge, CallSite, CallTrace, DataEdge, DataHop, EvidenceKind, LocalId, LocalKind, LocalValue,
    NodeId, Observation, SourceLocation,
};
pub use mir::{CallEvidence, CallRelation, MirCall, MirGraph, MirLocal, UnifiedCallGraph};

// The collector probe declares its own face and then collects *this crate's* opted-in
// declarations, so it has to run inside the crate whose registry is being read — as an
// integration test it would collect the test crate's registry instead. Its macro invocation is
// the strict arm (both fields, comma-separated), and the strict arm's path call goes to a
// hand-written macro, so from inside the crate it reaches the chain — the thing the tolerant arm
// cannot do in-crate (see `docs/b3-registration-diagnosis.md`).
// 采集探针先声明自己的面，再采集**本 crate** 中已选择加入的声明，因此必须运行在被读取注册表的那个
// crate 内部——作为集成测试会去读测试 crate 的注册表。它的宏调用是严格那一支（两个字段、全逗号），
// 而严格分支的路径调用是手写宏，因此它在 crate 内部能到达宏链——宽容分支恰恰做不到这一点
// （见 `docs/b3-registration-diagnosis.md`）。
#[cfg(test)]
#[path = "tests/collector_integration.rs"]
mod collector_integration;
