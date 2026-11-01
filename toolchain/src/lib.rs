//! The publishing surface: the seven thin execution surfaces of XiRang in one crate.
//! 发布面：XiRang 的七个薄执行面合成一个 crate。
//!
//! Each module was its own crate before the merge; the module name is the merge plan's
//! name for that surface (`build_method` -> `build_method`, `run_method` -> `runtime`, ...).
//! 每个模块在合并前都是独立 crate；模块名取合并方案里那一面的名字
//! （`build_method` -> `build_method`、`run_method` -> `runtime`，等）。
//!
//! The root globs every module so the historical crate-root paths (`crate::host!()` and
//! friends) still resolve; `run`-shaped names live in their modules, and the build entry
//! is written `crate::build_method::run()`.
//! 根部把每个模块 glob 出来，使历史上的 crate 根路径仍然可解析；`run` 形状的名字留在各自
//! 模块里，构建入口写成 `crate::build_method::run()`。
#![warn(missing_docs)]

#[cfg(any(feature = "build", feature = "authoring"))]
#[path = "source_cache.rs"]
mod source_cache;

#[cfg(any(feature = "studio", feature = "mcp"))]
#[path = "cargo_mir.rs"]
mod cargo_mir;

#[cfg(any(feature = "cli", feature = "mcp"))]
mod isolation;

#[cfg(feature = "build")]
#[path = "cargo_metadata.rs"]
mod cargo_metadata;

// Where a package's registration faces live is one rule with one implementation, and
// two execution surfaces need it: the build (which walks it) and the authoring writer
// (which writes into it). It lives at the crate root so the authoring surface reaches
// the same answer **without** the `build` feature — a feature-dependent copy is how a
// custom `[lib] path` layout silently got its faces written into `src/`.
// 一个包的注册面住在哪里是一条规则、一份实现，而两个执行面都需要它：构建（遍历它）与创作写入方
// （写进它）。它住在 crate 根，使创作执行面**无需** `build` 特性就能拿到同一个答案——依赖特性的
// 副本正是自定义 `[lib] path` 布局把面静默写进 `src/` 的原因。
#[cfg(any(feature = "build", feature = "authoring"))]
#[path = "source_layout.rs"]
pub(crate) mod source_layout;

// The bridge from a face view to the project's object ledger. It reads and renders the project file
// and nothing else, so `authoring` alone is the gate: the structural migration that renames a module
// must update the mapping **inside the same transaction**, and that path must also compile for a
// consumer that authors without the build surface.
// 从注册面视图到项目对象台账的桥。它只读写这个项目文件、不碰别的，因此门控只到 `authoring`：改名模块的
// 结构迁移必须在**同一个事务**里更新映射，而这条路径也要能为不带构建面的创作调用方编译。
#[cfg(feature = "authoring")]
#[path = "object_ledger.rs"]
pub(crate) mod object_ledger;

// The merged crate has to be able to name itself. Macros that the old per-crate layout expanded
// into carry `::xirang_toolchain::…` paths — they were written when every surface was its own
// crate and the host was an external caller — and the in-crate test modules that arrived with
// batch 2 were written the same way. Without this alias those expansions fail with "cannot find
// `xirang_toolchain` in the crate root", which is what parked the `(b)` residue.
// 合并后的 crate 必须能叫出自己的名字。旧的分 crate 布局下，那些宏展开出来的路径是
// `::xirang_toolchain::…`——写它们的时候每个执行面都是独立 crate、宿主是外部调用者——而随批 2
// 一起进来的 crate 内测试模块也按同样方式书写。没有这个别名，那些展开会以
// "cannot find `xirang_toolchain` in the crate root" 失败，这正是 `(b)` 遗留当初被搁置的原因。
extern crate self as xirang_toolchain;

/// The identity namespace the in-crate face probes declare under.
/// crate 内的面探针声明注册面时所用的身份命名空间。
///
/// The probes (the `call_evidence` collector probe and its siblings) declare faces with the
/// declaration macros, which read `crate::XIRANG_NAMESPACE` — **this** crate's root, because a
/// probe is mounted inside the crate (audit `M7`, P3.3). The value is the one those probes used to
/// bake in by hand, so nothing about them moves. It exists only for test builds: a published
/// toolchain crate is not a host and has no identity domain of its own.
/// 探针（`call_evidence` 的采集探针及其同类）用声明宏声明注册面，而声明宏读的是
/// `crate::XIRANG_NAMESPACE`——**本** crate 的根，因为探针挂载在 crate 之内（审计 `M7`，P3.3）。
/// 这个值就是那些探针过去手工烤进去的那个，因此它们什么都不移动。它只存在于测试构建里：发布出去的工具链
/// crate 不是宿主、没有自己的身份域。
#[cfg(test)]
pub const XIRANG_NAMESPACE: &str = env!("CARGO_PKG_NAME");

// The shared authoring service: one request vocabulary, one plan, one transaction, for every
// surface that creates or edits a registration face. It needs `authoring` for the Rust generator
// and `build` for the source layout / declared-native readers.
// 共享创作服务：一套请求词汇、一份计划、一个事务，服务每个创建或编辑注册面的执行面。它需要
// `authoring` 提供 Rust 生成器，需要 `build` 提供源码布局与声明式本地面读取器。
#[cfg(all(feature = "authoring", feature = "build"))]
#[path = "authoring_service/service.rs"]
pub mod authoring_service;

// t115-mount: begin
#[cfg(feature = "build")]
#[path = "build_method/lib.rs"]
pub mod build_method;
#[cfg(feature = "evidence")]
#[path = "call_evidence/lib.rs"]
pub mod call_evidence;
#[cfg(feature = "cli")]
#[path = "cli/lib.rs"]
pub mod cli;
#[cfg(feature = "mcp")]
#[path = "mcp/lib.rs"]
pub mod mcp;
/// The multilingual production surface, re-exported from `xirang-multilang`.
/// 多语言生产面，从 `xirang-multilang` 重导出。
///
/// The implementations live in the crate that owns language adapters; this module is the historical
/// path every execution surface calls, and it holds no second implementation.
/// 实现住在拥有语言适配器的那个 crate 里；本模块是每个执行面调用的历史路径，它不持有第二份实现。
#[cfg(any(feature = "cli", feature = "mcp", feature = "studio"))]
pub mod multilang {
    pub use xirang_multilang::production::*;
}
#[cfg(feature = "plugins")]
#[path = "plugin_host/lib.rs"]
pub mod plugin_host;
#[cfg(feature = "run")]
#[path = "run_method/lib.rs"]
pub mod run_method;
#[cfg(feature = "studio")]
#[path = "studio/lib.rs"]
pub mod studio;
// t115-mount: end

// t115-root-glob: begin
// The historical crate root of each surface, lifted so `crate::X` (and therefore
// `xirang_toolchain::X`) resolves for every module item.  Explicit re-exports below
// win over these globs where a name would be ambiguous.
// 把每个执行面的历史 crate 根提升到根上，使 `crate::X`（也就是 `xirang_toolchain::X`）
// 对每个模块条目都可解析；下面显式再导出的名字在冲突处优先于这些 glob。
#[cfg(feature = "build")]
pub use self::build_method::*;
#[cfg(feature = "evidence")]
pub use self::call_evidence::*;
#[cfg(feature = "cli")]
pub use self::cli::*;
// `mcp` contributes exactly one root-level name (`run`, its stdio entry) and that name is
// claimed explicitly above, so its glob would re-export nothing — which `unused_imports`
// reports under `-D warnings`. The module keeps its path: `xirang_toolchain::mcp::run`
// is how the bridge entry is addressed, and the module itself is still mounted below.
// `mcp` 只贡献一个根级名字（`run`，它的 stdio 入口），而那个名字已被上面的显式再导出占用，
// 于是它的 glob 什么也带不上来——这在 `-D warnings` 下会被 `unused_imports` 报出来。模块路径
// 保留：桥的入口按 `xirang_toolchain::mcp::run` 寻址，模块本身仍在上面挂载。
#[cfg(feature = "plugins")]
pub use self::plugin_host::*;
#[cfg(feature = "run")]
pub use self::run_method::*;
#[cfg(feature = "studio")]
pub use self::studio::*;

// Two names are carried by more than one of the globs above, so the paragraph at the top of
// this block has to be made true rather than merely asserted: `run` arrives from `build_method`,
// `mcp` and `cli`, and `mir` arrives both from `call_evidence` and from `runtime` (which
// re-exports the kernel's `mir`). The explicit re-exports below win over the globs and name the
// host-facing one of each pair — the build-script entry `build_method::run()` and the kernel's
// `mir` module — while every other name stays reachable through the globs. Nothing else is
// promised at this level: the module path is still the official address
// (`xirang_toolchain::mcp::run`, `xirang_toolchain::cli::run`, …). Measured by
// `cargo clippy --all-targets --all-features -- -D warnings`, which reported
// `ambiguous_glob_reexports` for both names before this pair existed.
// 上面这些 glob 里有两个名字被多个模块同时带上，因此这一段开头那句话得**做到**而不只是写着：
// `run` 会从 `build_method`、`mcp`、`cli` 三处来，`mir` 会同时从 `call_evidence` 与 `runtime`
// （后者重导出了内核的 `mir`）来。下面这两行显式再导出优先于 glob，并各自点名宿主面向的那一个
// ——构建脚本入口 `build_method::run()` 与内核的 `mir` 模块；其余名字仍可经 glob 取得。
// 这一层不再承诺别的：官方地址依旧是模块路径（`xirang_toolchain::mcp::run`、
// `xirang_toolchain::cli::run` 等）。判据是
// `cargo clippy --all-targets --all-features -- -D warnings`：在这一对出现之前，这两个名字都会
// 报 `ambiguous_glob_reexports`。
#[cfg(feature = "build")]
pub use self::build_method::run;
#[cfg(feature = "run")]
pub use self::run_method::mir;
// t115-root-glob: end
