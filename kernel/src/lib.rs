//! XiRang's registry protocol and runtime core.
//! XiRang 的注册协议与运行期内核。
//!
//! The official address of a protocol noun is its module path
//! (`xirang_kernel::identity::NodeId`). The crate root re-exports the module hierarchy
//! plus a curated vocabulary that host code, the build step and the runtime-check
//! surfaces write as a bare name. The kernel's modules also glob into the root, so
//! every other noun is *reachable* there as well; that is convenience, not a second
//! contract, and only the curated names are promised as bare ones.
//! 协议名词的官方地址就是它的模块路径（`xirang_kernel::identity::NodeId`）。crate 根部重导出模块
//! 层级，外加一份精选词汇——宿主代码、构建步骤与运行期校验面会以裸名书写它们。内核各模块也会
//! 平铺 glob 到根部，因此其余名词同样**可以**在那里取得；那是便利，不是第二份契约，只有精选
//! 清单上的名字被承诺为裸名可用。

// The kernel is the crate every host links, so its public surface is the one that
// has to be readable on docs.rs without leaving the page. The lint is on for the
// whole crate rather than the roadmap's original per-module whitelist: the debt
// was paid down in one pass, and a crate-wide lint is what keeps it from coming
// back. `clippy -D warnings` turns every new undocumented public item into a
// failure.
// 内核是每个宿主都会链接的 crate，因此它的公开面正是必须在 docs.rs 上不必跳页就能读懂的那
// 一份。lint 开在整个 crate 上，而不是路线图最初计划的逐模块白名单：这笔文档债已一次还清，
// 而只有 crate 级别的 lint 才能防止它卷土重来。`clippy -D warnings` 会把每一个新增的、没有
// 文档的公开项变成失败。
#![warn(missing_docs)]

#[path = "registry_core.rs"]
pub mod registry_core;

// The module hierarchy is the official surface.
// 模块层级就是官方表面。
#[cfg(feature = "syntax")]
pub use registry_core::syntax;
pub use registry_core::{
    adoption, authoring, declaration, diagnostic, identity, interchange, json, lexicon,
    maintenance, mir, plugin, release, requirements, source, tree,
};

// Identity, registration and the contract/plugin records declarations carry.
// 身份、注册，以及注册声明携带的合同与插件记录。
pub use registry_core::{
    Admission, ContractId, FlowContract, FrameworkId, NodeId, OwnedFlowContract, PartsContract,
    PluginManifest, PluginMode, PluginSource, PresetContract, ROOT_NODE_ID, RegistrationInfo,
    RegistrationRule, Registry, Sha256, assert_contract, root_node_id, sha256_hex,
};

// Runtime-check and call-evidence nouns the tracing surfaces name directly.
// 追踪面直接命名的运行期校验与调用证据名词。
pub use registry_core::{
    COORDINATES_IN_VIEWPORT, CallEdge, CallSite, Coordinates, EvidenceKind, FINITE_NUMBER,
    LogicalCallEdge, NON_EMPTY_TEXT, Provenance, ProvenanceStep, RuntimeCheckFailure,
    RuntimeCheckSpec, RuntimeValue, TraceMode,
};

// Build-side diagnostics and capability checks.
// 构建侧诊断与能力检查。
pub use registry_core::{
    BuildDiagnostic, BuildDiagnostics, CapabilityDeclaration, CapabilityRequirement, StaticFace,
    TopologyRecord, missing_capabilities, validate_face_topology,
};
