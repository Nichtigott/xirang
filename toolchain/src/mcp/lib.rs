//! MCP stdio bridge for compact XiRang source and registry queries, plus
//! previewed authoring writes.
//! 面向紧凑 XiRang 源码与注册树查询、以及先预览后落盘的创作写入的 MCP stdio 桥。
//!
//! Five tools index Rust source text; the rest answer from evidence that is not
//! source text. `xirang.registry` reports the registration tree the *build*
//! published — `<package>/target/xirang/out`, read through `build_method`'s
//! readers — so an agent can ask what the registry is instead of reconstructing
//! it from macro names, and a workspace-rooted answer costs the records rather
//! than every member's source walk. The derivation (`face_views`, what the CLI's
//! `explain` uses) is the fallback for a member that published nothing, and every
//! reply says which of the two it used.
//! `xirang.explain` reads the build's *published* files (`target/xirang/out`)
//! and answers what actually ships — scope and release pruning — which the source
//! cannot, or the static overlay projection (`overlay: true`) that says which slot
//! each declared cut replaces; `xirang.search` matches registry faces and
//! annotates each with that build's verdict; `xirang.diff` states the face-level
//! delta between those two sides,
//! and `xirang.trace` reads a recorded trace artifact and answers what actually
//! ran, refusing an artifact that describes a different tree. A **workspace root** is a
//! root too: it is a virtual manifest with no identity namespace of its own, so the tree
//! tools enumerate its members through `cargo metadata`, give each one its own context,
//! and answer grouped by package — every member present with its status, and a member
//! Cargo cannot resolve named with its reason rather than dropped (`workspace.rs` is that
//! entrance). Contract, admission,
//! and registration-rule fields are still absent: those need a loaded registry,
//! not a source scan.
//! 五个工具索引 Rust 源码文本；其余工具用非源码文本的证据作答。`xirang.registry` 报告**构建**
//! 推导出的注册树，走的是 CLI 的 `explain` 所用的同一个 `face_views`，因此代理可以直接问注册树是
//! 什么，而不是从宏名重建。`xirang.explain` 读构建**发布**的文件（`target/xirang/out`），回答
//! 真正会发布什么——作用域与发布剪枝——这是源码答不出来的；给出 `overlay: true` 时则是静态覆盖投影，
//! 说出每条已声明切口替换哪个槽位。`xirang.search` 匹配注册面并给每个命中标注那次构建的结论；
//! `xirang.diff` 说出两侧的面级差异；
//! `xirang.trace` 读取已记录的 trace artifact，回答真正跑了什么，并拒绝描述另一棵树的 artifact。
//! contract、admission 与 registration rule 字段仍然没有：那些需要一个已加载的注册机，而不是
//! 源码扫描。
//!
//! JSON-RPC frames arrive on stdin and responses leave on stdout. Reads stay
//! below the configured `XIRANG_PACKAGE_ROOT`. Writes exist — `xirang.apply`
//! — and they go through the **same authoring executor Studio uses**, inside an
//! `AuthoringContext` built from the resolved package root and the namespace
//! Cargo reports, so an agent's edit passes the kernel's admission and topology
//! checks instead of re-implementing them here. A write is previewed first: the
//! tool runs the real operation against a throwaway copy of the package and
//! reports the resulting tree and file diff; only `apply: true` touches the
//! project. Naming a package runs `cargo metadata` (through
//! `crate::build_method::package_name`), because the package name is the
//! `NodeId` namespace and Cargo is its authority.
//! Three writes exist: `xirang.apply` (faces, through the authoring executor), and
//! the two Studio actions the bridge was missing — `xirang.new_project`
//! (`build_method::scaffold::create_project`, the same scaffold `xirang new` and
//! Studio's wizard run) and `xirang.plugin` (the plugin lock record
//! `submit_plugin` writes, through the kernel's `PluginCatalog` gate). All three
//! preview by default, and the two whose effect is not recoverable by reading the
//! source — a plugin lock record and a project written into an existing directory —
//! also require the request to say `confirm: true`.
//! JSON-RPC 帧从 stdin 进入、响应从 stdout 输出。读取仍在配置的
//! `XIRANG_PACKAGE_ROOT` 之下。写入是存在的——`xirang.apply`——而且它走**与 Studio
//! 相同的 authoring 执行器**，运行在由已解析包根与 Cargo 报告的命名空间构成的
//! `AuthoringContext` 里，因此代理的编辑会经过内核的准入与拓扑校验，而不是在这里重新实现一遍。
//! 写入先预览：工具在一份一次性的包副本上运行真实操作，报告将得到的树与文件 diff；只有
//! `apply: true` 才会碰真实项目。为包命名会运行 `cargo metadata`（经
//! `crate::build_method::package_name`），因为包名就是 `NodeId` 命名空间，而 Cargo 是它
//! 的权威。
//! 写入一共有三个：`xirang.apply`（注册面，走 authoring 执行器），以及桥此前缺的那两个 Studio
//! 动作——`xirang.new_project`（`build_method::scaffold::create_project`，即 `xirang new` 与
//! Studio 向导运行的同一份脚手架）与 `xirang.plugin`（`submit_plugin` 写下的插件锁记录，经内核的
//! `PluginCatalog` 闸门）。三者默认都只预览；其中两个的效果无法靠读源码复原——插件锁记录，以及写进
//! 已存在目录的项目——因此它们还要求请求自己说出 `confirm: true`。

// The published surface must be readable on docs.rs without leaving the page,
// so the lint is on for the whole crate; `clippy -D warnings` makes a new
// undocumented public item a failure.
// 发布表面必须能在 docs.rs 上不跳页读懂，因此 lint 开在整个 crate 上；
// `clippy -D warnings` 会让新增的、没有文档的公开项变成失败。

pub mod client;
#[path = "protocol.rs"]
mod protocol;

pub use protocol::run;

#[path = "tools.rs"]
mod tools;

#[path = "registry.rs"]
mod registry;

#[path = "apply.rs"]
mod apply;
#[path = "crates.rs"]
mod crates;
mod multilang_tool;

#[path = "apply_move_set.rs"]
mod apply_move_set;

#[path = "apply_target.rs"]
mod apply_target;

#[path = "new_project.rs"]
mod new_project;

#[path = "plugin.rs"]
mod plugin;

#[path = "resolve.rs"]
mod resolve;

#[path = "workspace.rs"]
mod workspace;

#[path = "published.rs"]
mod published;

#[path = "freshness.rs"]
mod freshness;

#[path = "ownership.rs"]
mod ownership;

#[path = "relations.rs"]
mod relations;

#[path = "affected.rs"]
mod affected;

#[path = "adopted.rs"]
mod adopted;

#[path = "refactor.rs"]
mod refactor;

#[path = "task.rs"]
mod task;

#[path = "preview.rs"]
mod preview;

#[path = "semantic_cache.rs"]
mod semantic_cache;

#[path = "semantic_sources.rs"]
mod semantic_sources;

#[path = "source_index.rs"]
mod source_index;
#[path = "source_request.rs"]
mod source_request;

#[path = "source_query.rs"]
mod source_query;

#[path = "explore.rs"]
mod explore;

#[path = "navigation.rs"]
mod navigation;

#[path = "read.rs"]
mod read;

#[path = "build_evidence.rs"]
mod build_evidence;

#[path = "trace.rs"]
mod trace;

#[path = "faces.rs"]
mod faces;

#[path = "diff.rs"]
mod diff;

#[path = "usages.rs"]
mod usages;

#[path = "check_registry.rs"]
mod check_registry;

#[path = "constraints.rs"]
mod constraints;

#[path = "check_plan.rs"]
mod check_plan;

#[path = "check_task.rs"]
mod check_task;

#[path = "check_cost.rs"]
mod check_cost;

#[path = "check_plan_phase.rs"]
mod check_plan_phase;

#[path = "check_resume.rs"]
mod check_resume;

#[path = "check_witness.rs"]
mod check_witness;

#[path = "check_plan_tail.rs"]
mod check_plan_tail;

#[path = "check_variant.rs"]
mod check_variant;

#[path = "check_execution.rs"]
mod check_execution;

#[path = "check_protection.rs"]
mod check_protection;

#[path = "candidate.rs"]
mod candidate;

#[path = "controller.rs"]
mod controller;

#[path = "approval.rs"]
mod approval;

#[path = "managed.rs"]
mod managed;

#[path = "capability.rs"]
mod capability;

#[path = "rules.rs"]
mod rules;

#[path = "check_log.rs"]
mod check_log;

#[path = "check_jobs.rs"]
mod check_jobs;

#[path = "check.rs"]
mod check;
#[path = "check_diagnostics.rs"]
mod check_diagnostics;
#[path = "check_request.rs"]
mod check_request;
#[path = "note.rs"]
mod note;

#[path = "claims.rs"]
pub mod claims;

#[path = "converge.rs"]
mod converge;

#[path = "converge_trace.rs"]
mod converge_trace;

#[path = "callgraph.rs"]
mod callgraph;

#[path = "verify.rs"]
mod verify;

#[path = "mir_capture.rs"]
mod mir_capture;

#[path = "mir.rs"]
mod mir;

#[path = "impact.rs"]
mod impact;

#[path = "grafts.rs"]
mod grafts;
#[path = "graph.rs"]
mod graph;
#[path = "index.rs"]
pub(crate) mod index;

#[path = "overlay.rs"]
mod overlay;

#[path = "search.rs"]
mod search;
#[path = "search_record.rs"]
mod search_record;

#[path = "locate.rs"]
mod locate;

#[path = "why.rs"]
mod why;

#[path = "consistency.rs"]
mod consistency;

#[path = "digest.rs"]
mod digest;

#[path = "tree_delta.rs"]
mod tree_delta;

#[path = "truncation.rs"]
mod truncation;

// What this server has already told this session (audit `W2-2`/`W2-5`): a fixed block is worth
// reading once, and from the second call on it is repetition paid for in every answer.
// 这个服务器在这个会话里已经说过的话（审计 `W2-2`/`W2-5`）：固定块值得读一次，而从第二次调用起它就是
// 在每条答案里被付费的重复。
#[path = "session.rs"]
mod session;
