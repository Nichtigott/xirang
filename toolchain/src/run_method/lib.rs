//! XiRang runtime: trace state, macros, and the authoring executor.
//! The registry tree and plugin protocol live in the kernel; this crate is
//! the run_method execution surface.
//! XiRang 运行期：trace 状态、宏与 authoring 执行器。
//! 注册树与插件协议在 kernel；本 crate 是 run_method 执行面。

// The published surface must be readable on docs.rs without leaving the page,
// so the lint is on for the whole crate; `clippy -D warnings` makes a new
// undocumented public item a failure.
// 发布表面必须能在 docs.rs 上不跳页读懂，因此 lint 开在整个 crate 上；
// `clippy -D warnings` 会让新增的、没有文档的公开项变成失败。

#[macro_use]
#[path = "macros/macros.rs"]
pub mod macros;
#[cfg(feature = "authoring")]
#[path = "authoring/authoring.rs"]
pub mod authoring;
#[path = "call_report.rs"]
pub mod call_report;
#[path = "host_registry.rs"]
pub mod host_registry;
#[path = "plugin.rs"]
pub mod plugin;
#[path = "registry.rs"]
pub mod registry;
#[path = "runtime/runtime.rs"]
// The merged crate keeps the surface's own historical module name; the nesting is the merge,
// not a second concept.  合并后保留了该执行面自己的历史模块名；这层嵌套是合并的结果，不是第二个概念。
#[allow(clippy::module_inception)]
pub mod runtime;
#[path = "shape.rs"]
pub mod shape;

pub use xirang_kernel::registry_core;
// This glob names the same kernel items the historical `xirang_kernel::*` glob did,
// and it overlaps this crate's own `authoring` shim (`parse`, `snapshot`,
// `validation`). Both paths are the same surface, so the overlap is allowed
// here rather than resolved.
// 本 glob 命名的内核条目与历史上的 `xirang_kernel::*` 完全相同，并与本 crate 自己的
// `authoring` shim（`parse`、`snapshot`、`validation`）重叠。两条路径同属一个执行面，
// 因此这里允许重叠而不做消解。
#[allow(ambiguous_glob_reexports)]
pub use xirang_kernel::registry_core::*;
/// The face field front end, re-exported so a host does not have to depend on
/// the proc-macro crate itself.
/// 注册面字段前端；再导出后宿主无需自己依赖 proc-macro crate。
pub use xirang_macro::face_fields;
/// The default `registry_rule` resolver used by the declarative face arms.
/// 声明式注册面分支使用的 `registry_rule` 默认值解析器。
#[doc(hidden)]
pub use xirang_macro::face_rule_or as __face_rule_or;
pub use xirang_macro::face_trait_labels_or as __face_trait_labels_or;
/// Register a native Rust declaration.
/// 注册原生 Rust 声明。
pub use xirang_macro::{contracts, face, soil};

// `#[macro_export]` 把宏放在 crate 根（`xirang_toolchain::host!`），而宿主与生成代码写的是
// 模块路径（`xirang_toolchain::run_method::host!`）。根路径那几个带
// `#[rust_analyzer::macro_style]`（属性宏展开过），因此**不能**用 `pub use crate::…`
// 再导出；这里改成薄转发宏：`$crate` 是 toolchain crate 根，转发到真正的实现。
// `#[macro_export]` lands the implementations at the crate root, while hosts and generated
// code write the module path. Those root macros are macro-expanded (they carry
// `#[rust_analyzer::macro_style]`), so `pub use crate::…` is rejected by rustc; these thin
// forwarders are plain `macro_rules!` and can be re-exported under the module path.
/// The runtime surface's `external_object!` alias (see [`crate::run_method::external_object`]).
/// 运行期面的 `external_object!` 别名（见 [`crate::run_method::external_object`]）。
#[macro_export]
macro_rules! __runtime_external_object {
    ($($tokens:tt)*) => { $crate::external_object! { $($tokens)* } };
}
pub use crate::__runtime_external_object as external_object;
/// The runtime surface's `host!` alias (see [`crate::run_method::host`]).
/// 运行期面的 `host!` 别名（见 [`crate::run_method::host`]）。
#[macro_export]
macro_rules! __runtime_host {
    ($($tokens:tt)*) => { $crate::host! { $($tokens)* } };
}
pub use crate::__runtime_host as host;
/// The runtime surface's `static_graft_plan!` alias (see [`crate::run_method::static_graft_plan`]).
/// 运行期面的 `static_graft_plan!` 别名（见 [`crate::run_method::static_graft_plan`]）。
#[macro_export]
macro_rules! __runtime_static_graft_plan {
    ($($tokens:tt)*) => { $crate::static_graft_plan! { $($tokens)* } };
}
pub use crate::__runtime_static_graft_plan as static_graft_plan;

#[cfg(feature = "authoring")]
#[allow(ambiguous_glob_reexports)]
pub use authoring::*;
pub use call_report::*;
pub use runtime::*;
pub use shape::*;

// The trace artifact is the one document a host hands to a separate reader, so
// its entry points are worth a crate-root path: `runtime::trace::*` stays the
// module path, and this is the short one the design document shows.
// 写入方交给独立读取方的唯一文档就是 trace artifact，因此它的入口值得一条 crate 根部路径：
// `runtime::trace::*` 仍是模块路径，这一条是设计文档里写的那条短路径。
// This line deliberately still names `artifact`: the module is `snapshot` now (`NAM-11`),
// and the alias in `runtime/trace/trace.rs` is what keeps the published path
// `crate::run_method::runtime::trace::artifact` alive. Naming it here is the pin the
// shim ratchet cannot express (it only reads `pub use xirang_kernel::…` statements): delete the
// alias and this line stops compiling.
// 这一行有意仍写 `artifact`：模块现在是 `snapshot`（`NAM-11`），保住已发布路径
// `crate::run_method::runtime::trace::artifact` 的是 `runtime/trace/trace.rs` 里的别名。
// 把旧路径写在这里，正是 shim 棘轮表达不了的那根钉子（它只读 `pub use xirang_kernel::…`）：
// 删掉别名，这一行就编译不过。
pub use runtime::trace::artifact::{
    TRACE_ARTIFACT_VERSION, TraceArtifact, TraceArtifactError, TraceFrame, read_trace_artifact,
    trace_artifact_path, write_trace_artifact,
};
