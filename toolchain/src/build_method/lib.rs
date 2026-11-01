//! Generate the passive crate entry from the folder layout.
//! 根据文件夹布局生成被动 crate 入口。
//!
//! The build script mirrors the local folder layout and derives a conservative
//! registration-face scope before rustc sees the generated module tree. Distributed
//! registration is routed through the collector adapter; this file does not
//! maintain a registration roster.
//! build.rs 镜像本地文件夹布局，并在 rustc 看到生成模块树之前保守推导注册面范围。
//! 分布式注册经由 collector 适配层完成，本文件不维护注册清单。
//!
//! The implementation lives in sibling modules; this file only mounts them,
//! re-exports the surface, and forwards the two entry points.
//! 实现位于同级模块；本文件只负责挂载它们、重新导出对外表面，并转发两个入口。

// The published surface must be readable on docs.rs without leaving the page,
// so the lint is on for the whole crate; `clippy -D warnings` makes a new
// undocumented public item a failure.
// 发布表面必须能在 docs.rs 上不跳页读懂，因此 lint 开在整个 crate 上；
// `clippy -D warnings` 会让新增的、没有文档的公开项变成失败。

use std::path::Path;

#[path = "build_input.rs"]
mod build_input;
#[path = "contracts.rs"]
mod contracts;

#[cfg(test)]
#[path = "test_support.rs"]
mod test_support;

#[path = "partition_roots.rs"]
pub(crate) mod partition_roots;

#[path = "partition_compat.rs"]
pub(crate) mod partition_compat;

#[path = "crate_plan.rs"]
pub(crate) mod crate_plan;

// Its only caller is the authoring CLI face (`xirang crates`), so the module rides that feature:
// a default build has no writer, and dead code is a warning this workspace refuses.
// 它唯一的调用方是创作面的 CLI（`xirang crates`），因此该模块随那个特性走：默认构建没有写入方，而死代码是
// 本工作区拒绝的告警。
#[cfg(any(feature = "cli", feature = "studio", feature = "mcp"))]
#[path = "crate_facade.rs"]
pub(crate) mod crate_facade;

#[cfg(any(feature = "cli", feature = "studio", feature = "mcp"))]
#[path = "crate_members.rs"]
pub(crate) mod crate_members;
#[cfg(any(feature = "cli", feature = "studio", feature = "mcp"))]
#[path = "crate_release.rs"]
pub(crate) mod crate_release;
#[cfg(any(feature = "cli", feature = "studio", feature = "mcp"))]
#[path = "crate_write.rs"]
pub(crate) mod crate_write;
#[cfg(any(feature = "cli", feature = "studio", feature = "mcp"))]
#[path = "crate_write_backup.rs"]
mod crate_write_backup;
#[cfg(any(feature = "cli", feature = "studio", feature = "mcp"))]
#[path = "crate_write_host_edges.rs"]
mod crate_write_host_edges;
#[path = "diagnostics.rs"]
mod diagnostics;
#[path = "discovery_cache.rs"]
mod discovery_cache;
#[path = "discovery_node.rs"]
mod discovery_node;
#[path = "discovery_record.rs"]
mod discovery_record;
#[path = "entry.rs"]
mod entry;
#[path = "entry_default.rs"]
mod entry_default;
// The reader's caller is `run_with_external`: a build script compiles its C++
// sources, hands the objects here, and the pipeline asks each one what its
// `xirang.faces` section declares. The module's own tests ride along below.
// 读取器的调用方是 `run_with_external`：构建脚本先编译自己的 C++ 源码，再把目标文件交给
// 这里，由管线逐个询问 `xirang.faces` 段声明了什么。模块自己的测试随同挂载。
#[path = "external_face.rs"]
mod external_face;
// The second carrier of the same records: a language that cannot place a section writes the
// records to a file, and this module reads that file into the same records the section path builds.
// 同一种记录的第二种载体：放不下段的语言把记录写进文件，本模块把那个文件读成段路径所构建的同一批记录。
#[path = "face_header.rs"]
pub(crate) mod face_header;
#[path = "record_file.rs"]
mod record_file;
// The constants file a record-file language's declaration names parents through: the build writes
// the identities it knows in that language's syntax before the producer runs.
// 记录文件语言的声明点名父级所用的常量文件：构建在生产运行之前，按该语言的语法写下它知道的身份。
#[path = "record_constants.rs"]
mod record_constants;
// The native sources a project declares once: which crate compiles each one, and the two passes that
// order it. Mounted here because both shapes call it — the host entry and the generated fragment.
// 项目声明一次的本地语言源码：哪个 crate 编译哪一份，以及决定这件事的两遍顺序。挂在这里是两个形状
// 都要调用它——宿主入口与生成的碎片。
#[path = "compatibility.rs"]
pub(crate) mod compatibility;
#[path = "declared_native.rs"]
pub(crate) mod declared_native;
pub use external_face::ExternalSource;
#[path = "face_cache.rs"]
mod face_cache;
#[path = "face_syntax_check.rs"]
pub(crate) mod face_syntax_check;

#[path = "face_view.rs"]
pub mod face_view;
#[path = "graft_plan_check.rs"]
mod graft_plan_check;
#[path = "graft_view.rs"]
mod graft_view;
#[path = "graph.rs"]
mod graph;
#[path = "identity_cache.rs"]
mod identity_cache;
#[path = "manifests.rs"]
mod manifests;
#[path = "node_identity.rs"]
mod node_identity;
#[path = "package.rs"]
mod package;
#[cfg(any(feature = "cli", feature = "studio", feature = "mcp"))]
#[path = "partition_view.rs"]
pub(crate) mod partition_view;
#[path = "pipeline.rs"]
mod pipeline;

#[path = "publish_lock.rs"]
pub(crate) mod publish_lock;
#[path = "registration_phase.rs"]
mod registration_phase;
#[path = "identity.rs"]
mod registry_identity;
#[path = "syntax.rs"]
mod registry_syntax;
#[path = "renderer.rs"]
mod renderer;
#[path = "scaffold.rs"]
pub mod scaffold;
#[path = "scope.rs"]
mod scope;
#[path = "scope_faces.rs"]
mod scope_faces;
#[path = "shape_decl.rs"]
mod shape_decl;
// The source fact is shared with the language adapters, which sit below this crate, so it lives in
// `xirang-multilang` and is re-exported here: every historical `build_method::source_ref::…` path
// still resolves, and `cpp_native` reads the same `locate` rule the bridge does.
// 源码事实与语言适配器共享，而适配器在本 crate 之下，因此它住在 `xirang-multilang` 并在本处重导出：
// 历史 `build_method::source_ref::…` 路径全部照旧，而 `cpp_native` 读的是与桥同一条 `locate` 规则。
pub use xirang_multilang::object_map;
pub use xirang_multilang::source_ref;
#[path = "source_walk.rs"]
mod source_walk;

#[path = "parallel.rs"]
mod parallel;
#[path = "static_plan.rs"]
mod static_plan;

/// The one directory name the build treats as the in-repo compile-failure demo.
/// 构建视为本仓库编译失败演示目录的、唯一那个目录名。
///
/// The demo host in this repository declares it and turns on the feature below to
/// see the diagnostics; every build rule that skips, gates, or plan-excludes it
/// keys on this name. It is an explicit list of one, not an implicit rule, because
/// the name belongs to the *demo* and not to the protocol: a third-party host that
/// happens to have a top-level directory of this name gets its faces gated behind a
/// feature it never declared, silently — audit `LGC-LG-16`, marked latent because it
/// needs the directory name to match by chance. Turning it off outright would change
/// the shipped demo host's build, so the fix is to make the name one documented
/// thing rather than a string repeated five times.
/// 本仓库里的演示宿主声明它并打开下面那个特性，以看到那些诊断；每一条跳过它、门控它或在静态计划里
/// 排除它的构建规则都以这个名字为键。它是一份显式的、只有一项的清单，而不是一条隐式规则，因为这个名字
/// 属于**演示**而不是协议：恰好有同名顶层目录的第三方宿主，会让自己的面被静默门控在一个它从未声明的
/// 特性之后——审计 `LGC-LG-16`，标为潜伏是因为需要目录名恰好命中才会生效。直接取消它则会改变出厂的演示
/// 宿主的构建，因此修法是把这个名字变成"一处有文档的东西"，而不是重复五次的字符串。
///
/// It never enters identity: `file!()` and the `NodeId` hash are computed from the
/// face file's own path, and nothing here rewrites a path.
/// 它从不参与身份：`file!()` 与 `NodeId` 散列都由注册面文件自己的路径算出，这里不改写任何路径。
pub(crate) const DEMO_ONLY_DIRECTORY: &str = "compile_error_demo";

/// The feature that turns the demo directory's diagnostics into build errors.
/// 把演示目录的诊断变成构建错误的那个特性。
///
/// The generated tree carries its `compile_error!` behind this `cfg`, and
/// `pipeline` declares the cfg name to `rustc`, so the name has to be spelled the
/// same way in both places — one constant instead of three literals.
/// 生成树把它的 `compile_error!` 挂在这个 `cfg` 之后，而 `pipeline` 会向 `rustc` 声明该 cfg 名，
/// 因此两处必须拼写一致——用一处常量代替三个字面量。
pub(crate) const DEMO_ONLY_FEATURE: &str = "compile_error_demo";

// Public surface, reachable at exactly the paths callers already use.
// 对外表面，保持在调用方已经在用的路径上。
pub use entry::host_entry_source;
pub use face_view::{
    BuildScopeView, FaceView, FileRow, PruningRow, build_output_is_current, face_views,
    face_views_and_unreadable, face_views_from_pruning, face_views_with_external, read_build_scope,
    read_file_manifest, read_pruning_manifest, read_shape_manifest,
};
pub use graft_view::{
    DeclaredGraft, DeclaredGraftExpressions, DeclaredGrafts, GraftPlanRow, OVERLAY_NOTE,
    OverlayProjection, OverlaySlot, SlotBinding, declared_grafts, graft_plan_rows,
    overlay_projection, slot_bindings,
};
pub use package::duplicated_own_crates;
/// The package name Cargo reports for a package root, which is the identity
/// namespace of every face that package compiles.
/// Cargo 为某个包根报告的包名，也就是该包编译的每个面的身份命名空间。
pub use package::identity_namespace;
pub use package::package_name;
/// The module path a registration source declares, for authoring surfaces.
/// 注册面源码声明的模块路径，供创作界面使用。
pub use static_plan::source_module_path;

// Crate-internal helpers, re-exported so sibling modules keep their `super::…`
// paths after the split instead of reaching into each other's new homes.
// crate 内部辅助项，重新导出以便同级模块在拆分后保留 `super::…` 路径，
// 不必互相伸进对方的新家。
pub use crate::source_layout::{SourceLayout, source_layout};
pub(crate) use build_input::BuildInput;
pub use build_input::HostCut;
pub(crate) use contracts::aggregate_contract_errors;
pub(crate) use discovery_cache::{
    CACHE_SCHEMA, cached_parent_id, update_discovery_cache, write_if_changed,
};
// `write_if_changed_at` is deliberately not re-exported here: its one caller (`crate_write`) imports
// it directly, and a re-export nobody uses is an unused import on the all-features face.
// `write_if_changed_at` **有意**不在这里重导出：它唯一的调用方（`crate_write`）直接导入它，而一个没人用的
// 重导出在全特性面上就是一条 unused import。
// The authoring read-back is the only caller: a build that is not authoring anything has no use
// for the ledger's materialized files, and an ungated re-export would be an unused import on the
// build-only face. The definition is gated with the same feature for the same reason.
// 只有创作读取方调用它：不创作任何东西的构建用不到台账物化的文件，而不设门控的重导出在仅 build
// 的面上会是一条未使用导入。定义按同一个特性门控，理由相同。
#[cfg(feature = "authoring")]
pub(crate) use discovery_node::explicit_placement_files;
pub(crate) use source_walk::{discovery_fingerprint, emit_rerun_paths, source_stamp};
// The read path of the bridge is the only caller of these two: the member map exists to verify the
// members of a workspace beside each other, and the budget is the rule it runs under. They are
// re-exported **under that feature** because a build without the bridge would otherwise carry an
// unused import — and the definitions themselves stay ungated, since the walk uses both.
// 只有桥的读路径调用这两个：成员映射存在的意义是把工作区的成员并排核验，而预算是它运行所依的规则。
// 它们**在那个特性下**才被重导出，因为不带桥的构建否则会背上一个未使用导入——而定义本身不设门控，
// 因为遍历两处都在用。
#[cfg(feature = "mcp")]
pub(crate) use parallel::{parallel_map_with_threshold, worker_budget};
// `resolve_host_entry` is deliberately absent: production code reaches it only
// through `host_entry_from_environment`, and re-exporting it for tests alone
// would be an unused import in a non-test build.
// 这里刻意不导出 `resolve_host_entry`：生产代码只经 `host_entry_from_environment`
// 到达它，仅为测试而导出会在非测试构建里成为未使用导入。
// The authoring surface's CLI face (`xirang crates`) is the only caller of these, so they ride the
// same feature: a default build has no use for them, and an unused re-export is a warning this
// workspace refuses.
// 这些名字唯一的调用方是创作面的 CLI（`xirang crates`），因此它们随着同一个特性走：默认构建用不到它们，
// 而未使用的再导出是本工作区拒绝的告警。
#[cfg(feature = "cli")]
pub(crate) use crate_facade::plan_facade;
#[cfg(feature = "cli")]
pub(crate) use crate_plan::partition_roots;
#[cfg(feature = "cli")]
pub(crate) use crate_plan::plan as plan_crates;
#[cfg(feature = "cli")]
pub(crate) use crate_release::plan_facade as plan_release_facade;
#[cfg(feature = "cli")]
pub(crate) use crate_release::plan_ghost as plan_release_ghost;
#[cfg(any(feature = "cli", feature = "studio", feature = "mcp"))]
pub(crate) use crate_write::{
    guard_shape, revert_partition, write_partition, write_partition_with_host, write_release,
};
#[cfg(feature = "cli")]
pub(crate) use crate_write_host_edges::host_needs_fragment;
pub(crate) use discovery_node::{Node, relative_display};
pub(crate) use entry::{HostEntry, host_entry_from_environment};
pub(crate) use entry_default::default_entry_source;
// The sibling-module name a registry-owning face reads its rule from is re-exported only where a
// second reader exists: the bridge's `promote` asks the same question the `face-rule` phase asks, and
// a build without the bridge would otherwise carry it as an unused import.
// 拥有注册机的面据以读取规则的兄弟模块名，只在存在第二个读者处重导出：桥的 `promote` 问的是与
// `face-rule` 阶段同一个问题，而没有桥的构建否则会把它当成未使用导入背着。
#[cfg(feature = "mcp")]
pub(crate) use face_syntax_check::REGISTRY_RULE_MODULE;
pub(crate) use face_syntax_check::{
    aggregate_parent_macro_errors, aggregate_registry_rule_errors, aggregate_requirements,
    aggregate_stable_name_errors, face_syntax_errors, parsed_face, unplaced_face_errors,
};
pub(crate) use graft_view::{declared_graft_view, enable_graft_entries, host_graft_entries};
pub(crate) use graph::{write_generation, write_graph_manifest};
pub(crate) use identity_cache::{cache_directory, prime_node_id_cache};
pub(crate) use manifests::{
    write_file_manifest, write_function_manifest, write_graft_manifest, write_pruning_manifest,
    write_shape_manifest, write_source_scope_manifest,
};
pub(crate) use node_identity::{CACHED_NODE_IDS, node_id};
#[cfg(feature = "cli")]
pub(crate) use partition_view::declared_host_cuts;
#[cfg(any(feature = "cli", feature = "studio", feature = "mcp"))]
pub use partition_view::{OnDisk, PackageView, PartitionView};
#[cfg(any(feature = "cli", feature = "studio", feature = "mcp"))]
pub(crate) use partition_view::{apply_declaration_edit, package_directory_of};
#[cfg(feature = "cli")]
pub(crate) use registry_identity::NodeId;
#[cfg(test)]
pub(crate) use renderer::render_lib;
pub(crate) use renderer::render_lib_with_external;
pub(crate) use scope::{
    SourceScope, collect_active_ids, face_source_is_active, module_feature, source_is_active,
};
pub(crate) use scope_faces::{FaceSource, collect_faces};
// The reader rides the surfaces that have to answer "is this subtree claimed?": the CLI and Studio, and
// the bridge's `move`, which refuses a move across a claim (P2, 2026-10-09).
// 读取方随那些必须回答"这棵子树被认领了吗"的执行面走：CLI 与 Studio，以及桥的 `move`——它拒绝跨认领的搬动
// （P2，2026-10-09）。
#[cfg(any(feature = "cli", feature = "mcp", feature = "studio"))]
pub(crate) use shape_decl::read_shape_declaration;
// The declaration's two writers ride the authoring surfaces: the CLI's `crates` verb, the bridge's
// `declare`/`undeclare`, and Studio's delete key all edit the same hand-written file through them.
// 声明的两个写入方随创作面走：CLI 的 `crates` 动词、桥的 `declare`/`undeclare`，以及 Studio 的删除键，
// 都经它们编辑同一个手写文件。
#[cfg(any(feature = "cli", feature = "mcp", feature = "studio"))]
pub(crate) use shape_decl::{DeclarationEdit, declare, undeclare};
pub(crate) use static_plan::static_plan;

/// Run the build-time discovery and validation pipeline from a Cargo build
/// script.
/// 从 Cargo build script 运行构建期发现与校验管线。
pub fn run() {
    let input = BuildInput::from_environment();
    let (only, facade) = legacy_shape_from_environment();
    let cuts = declared_cuts(&input.manifest, &input.host_cuts);
    match run_declared(
        &input,
        &registry_identity::package_namespace(),
        only.as_deref(),
        facade,
        &cuts,
    ) {
        Ok(()) => {}
        // A declared source could not be read, compiled or understood: that is a build failure, and the
        // message names the file. A tree that declares no `[native]` table never reaches this arm.
        // 声明的源码读不到、编不过或读不懂：那是构建失败，消息里点名文件。没有 `[native]` 表的树不会走到
        // 这一支。
        Err(DeclaredFailure::Native(diagnostics)) => {
            panic!("{}", diagnostics.render_build_diagnostics())
        }
        // The tree itself carries the pipeline's failure as a `compile_error!`, which is the shape a
        // build-script caller has always read; the warning is for the person watching the build.
        // 管线自己的失败由生成树以 `compile_error!` 携带——构建脚本调用方一直是这么读它的；这条警告是给
        // 盯着构建输出的人看的。
        Err(DeclaredFailure::Pipeline(diagnostics)) => {
            for line in diagnostics.render_build_diagnostics().lines() {
                println!("cargo:warning={line}");
            }
        }
    }
}

/// Where a run failed: the pipeline the tree carries, or the declared sources this build compiles.
/// 一次运行失败在哪里：树自己携带的管线失败，还是本次构建编译的声明源码。
enum DeclaredFailure {
    /// The pipeline refused; the rendered text is what a terminal or `compile_error!` reads.
    /// 管线拒绝了；渲染文本就是终端或 `compile_error!` 读到的内容。
    Pipeline(xirang_kernel::BuildDiagnostics),
    /// The declared sources could not be read, compiled or decoded.
    /// 声明的源码读不到、编不过或解不出来。
    Native(xirang_kernel::BuildDiagnostics),
}

/// The subtrees a host handed to generated crates, as the declaration and the host's cuts spell them.
/// 宿主交给生成 crate 的那些子树，按声明与宿主切口的拼写。
fn declared_cuts(manifest: &Path, host_cuts: &[HostCut]) -> Vec<String> {
    let mut cuts = shape_decl::read_shape_declaration(manifest)
        .ok()
        .flatten()
        .map(|declaration| declaration.cut_subtrees())
        .unwrap_or_default();
    for cut in host_cuts {
        if !cuts.contains(&cut.cut) {
            cuts.push(cut.cut.clone());
        }
    }
    cuts
}

/// One declared-source failure as the diagnostics a caller renders.
/// 一条声明源码失败，作为调用方渲染的诊断。
fn native_failure(message: String) -> DeclaredFailure {
    let mut diagnostics = xirang_kernel::BuildDiagnostics::default();
    diagnostics.push(xirang_kernel::BuildDiagnostic::new(
        "declared-native",
        message,
    ));
    DeclaredFailure::Native(diagnostics)
}

/// Run the pipeline for one shape, compiling the native sources **this crate owns**.
/// 为一个形状运行管线，并编译**本 crate 拥有**的本地源码。
///
/// Two passes, and the order is forced: a declared source includes the generated `xirang_faces.hpp`, so
/// the header has to exist before the source compiles — and the record the source carries has to exist
/// before the tree that registers it renders. A tree with nothing declared pays nothing: it runs the
/// pipeline once, exactly as before.
/// 两遍，顺序是被迫的：声明的源码包含生成的 `xirang_faces.hpp`，因此表要在源码编译前存在；而源码携带的
/// 记录要在登记它的树渲染前存在。什么都没声明的树不付这份代价：它只跑一遍管线，与从前一样。
fn run_declared(
    input: &BuildInput,
    package: &str,
    only: Option<&str>,
    facade: bool,
    cuts: &[String],
) -> Result<(), DeclaredFailure> {
    // The `[record]` table is read **without side effects** first: a package with no table pays
    // nothing, and a package that declares a generated constants file decides below whether the
    // producer needs a pass before it runs (the constants are the build's, and the producer runs
    // inside the build).
    // 先**无副作用地**读 `[record]` 表：没有表的包不付代价；声明了生成常量文件的包在下面决定生产者是否
    // 需要先跑一遍（常量是构建的，而生产者在构建里运行）。
    let record = record_file::context(&input.manifest).map_err(native_failure)?;
    let declaration = record.as_ref().map(|(declaration, _)| declaration);
    let constants = record
        .as_ref()
        .and_then(|(_, constants)| constants.as_ref());
    let declared = declared_native::read(&input.manifest).map_err(native_failure)?;
    let Some((recipe, faces)) = declared else {
        // No declared source. A record-file package that asks for generated constants needs one
        // pass **before** the producer, because the producer names parent identities the build
        // computes; then the second pass registers what the producer wrote. Without a constants
        // request the historical shape is unchanged: produce and read first, one pass.
        // 没有声明的源码。要求生成常量的记录文件包在生产之前需要一遍：生产者点名的是构建算出的父身份；
        // 然后第二遍登记生产者写出的东西。没有常量请求时历史形状不变：先生产再读取，只跑一遍。
        if constants.is_some()
            && let Some(diagnostics) = registry_identity::run_as_package(package, || {
                super::build_method::discovery_cache::with_native_parents(
                    &std::collections::BTreeMap::new(),
                    || pipeline::run(input, &[], &[], &[], &[], constants),
                )
            })
        {
            return Err(DeclaredFailure::Pipeline(diagnostics));
        }
        let file_records =
            record_file::records_after_producing(&input.manifest, declaration, package)
                .map_err(native_failure)?;
        return registry_identity::run_as_package(package, || {
            super::build_method::discovery_cache::with_native_parents(
                &std::collections::BTreeMap::new(),
                || pipeline::run(input, &file_records, &[], &[], &[], None),
            )
        })
        .map_or(Ok(()), |diagnostics| {
            Err(DeclaredFailure::Pipeline(diagnostics))
        });
    };
    // The language implementation is opt-in. A package that declares C++ sources in a build that
    // did not enable `lang-cpp` gets the capability gap and the way to close it, not a silent
    // fall-through to a compiler this binary does not carry.
    // 语言实现是可选的。声明了 C++ 源码、而本次构建没有启用 `lang-cpp` 时，要给出能力缺口与启用方法，
    // 而不是静默落到一个本二进制并未携带的编译器上。
    if let Some((language, feature)) =
        declared_native::adapter_gap(&input.manifest).map_err(native_failure)?
    {
        return Err(native_failure(format!(
            "this package declares {} {language} native source(s), but this build does not carry \
             the {language} adapter; enable it with `xirang-toolchain` features = [\"{feature}\"] \
             (or build the CLI with `--features cli,{feature}`)",
            faces.len()
        )));
    }

    // 名字与身份的对应表按**全部**声明算，不按本包拥有的那些：宿主把子树交出去之后仍要能解析
    // "我的 Rust 面的父是一个声明面"——它只是不编译那个面。
    // The name-to-identity table covers **every** declaration, not just the ones this package owns: a
    // host that handed a subtree away still has to resolve "my Rust face's parent is a declared
    // face" — it simply does not compile that face.
    let parents: Vec<(String, registry_identity::NodeId, bool)> = faces
        .iter()
        .map(|face| {
            (
                declared_native::ident(&face.source, &face.name),
                registry_identity::NodeId::from_namespaced_path(
                    &registry_identity::package_namespace(),
                    &face.source,
                    &face.name,
                ),
                face.owns_registry,
            )
        })
        .collect();
    let native_map: std::collections::BTreeMap<String, registry_identity::NodeId> = parents
        .iter()
        .map(|(ident, id, _)| (ident.clone(), *id))
        .collect();
    // The map is installed around **every** pipeline run, not just the plan: the discovery cache is
    // written from inside the same run, and a row written without a parent cannot be read back — the
    // second pass would then plan one face fewer and render a smaller tree (measured 2026-10-22: the
    // fragment's first pass planned `shop::panel::area::reading`, its second pass dropped it).
    // 这张表装在**每一次**管线运行之外，而不只是计划之外：发现缓存是在同一次运行里写的，一条没有父的
    // 行读不回来——第二遍就会少计划一个面、渲染出更小的树（2026-10-22 实测：碎片第一遍计划了
    // `shop::panel::area::reading`，第二遍把它丢了）。
    let owned = declared_native::owned(faces.clone(), only, facade, cuts);
    // A host that hands a subtree away does not compile the native faces under it — the crate that
    // owns their parent does. Saying so is the difference between a view that is partial and a view
    // that claims to be whole: `xirang check` used to print `3 face(s)` and leave the fourth one to be
    // discovered at run time.
    // 交出子树的宿主不编译它下面的本地面——拥有它们父的那个 crate 编译。把这件事说出来，正是"部分视图"
    // 与"自称完整的视图"之间的差别：`xirang check` 过去打印 `3 face(s)`，第四个面要等到运行期才被发现。
    if only.is_none() && !facade {
        if !owned.is_empty() {
            let here = owned
                .iter()
                .map(|face| face.source.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            let line = format!(
                "note: {} native face(s) are declared in this package and registered by its build, so \
                 the count above does not include them: {here}",
                owned.len()
            );
            if input.emit_cargo_directives {
                println!("cargo:warning={line}");
            } else {
                eprintln!("{line}");
            }
        }
        if owned.len() < faces.len() {
            let elsewhere = declared_native::elsewhere(&input.manifest, &faces, &owned);
            let line = format!(
                "note: {} native face(s) in this project are compiled by the crate that owns their \
                 parent, not by this one: {}",
                faces.len() - owned.len(),
                elsewhere.join(", ")
            );
            if input.emit_cargo_directives {
                println!("cargo:warning={line}");
            } else {
                eprintln!("{line}");
            }
        }
    }
    // One place for the wrapped pipeline run: every pass installs the native parent map, because the
    // discovery cache is written inside the run.
    // 一个包好的管线运行入口：每一遍都装上本地父表，因为发现缓存是在运行里写的。
    let run_pass = |external: &[external_face::ExternalFaceRecord],
                    declared_faces: &[declared_native::DeclaredFace],
                    parents: &[(String, registry_identity::NodeId, bool)],
                    catalog: &[declared_native::DeclaredFace],
                    constants: Option<&record_constants::Request>|
     -> Option<xirang_kernel::BuildDiagnostics> {
        registry_identity::run_as_package(package, || {
            super::build_method::discovery_cache::with_native_parents(&native_map, || {
                pipeline::run(input, external, declared_faces, parents, catalog, constants)
            })
        })
    };
    // The record file's producer runs here. When it needs the generated constants file, one pass has
    // already written it; otherwise this is the historical "produce and read before the pipeline".
    // 记录文件的生产者在这里运行。需要生成常量文件时，已经有一遍把它写好了；否则这就是历史上的
    // "管线之前先生产再读取"。
    if constants.is_some()
        && let Some(diagnostics) = run_pass(&[], &owned, &parents, &faces, constants)
    {
        return Err(DeclaredFailure::Pipeline(diagnostics));
    }
    let file_records = record_file::records_after_producing(&input.manifest, declaration, package)
        .map_err(native_failure)?;
    if owned.is_empty() {
        // Nothing here is this crate's to compile — a facade, or a fragment whose subtree holds no
        // declared source. The faces it registers still travel; only the compile step is skipped.
        // 这里没有本 crate 要编译的东西——facade，或者认领的子树里没有声明源码的碎片。它登记的面照旧
        // 传递，只是跳过编译。
        return run_pass(&file_records, &[], &parents, &faces, None)
            .map_or(Ok(()), |diagnostics| {
                Err(DeclaredFailure::Pipeline(diagnostics))
            });
    }
    // Pass one: the header the declared sources include. When the constants request already forced a
    // pass, that pass also wrote the header, so this one is skipped.
    // 第一遍：声明的源码要包含的那张表。常量请求已经逼出过一遍时，那一遍也写了表，因此这一遍跳过。
    if constants.is_none()
        && let Some(diagnostics) = run_pass(&[], &owned, &parents, &faces, None)
    {
        return Err(DeclaredFailure::Pipeline(diagnostics));
    }
    let objects = declared_native::compile(
        &input.manifest,
        &input.out_dir,
        &recipe,
        &owned,
        input.emit_cargo_directives,
    )
    .map_err(native_failure)?;
    let sources = declared_native::sources(&objects, &owned);
    let mut records = external_face::records(&sources, package).map_err(native_failure)?;
    // Both carriers produce the same record type; a package may declare native sources and a
    // record file in the same build, and the tree registers the union.
    // 两种载体产出同一种记录类型；一个包可以在同一次构建里既声明本地源码又声明记录文件，树登记它们的并集。
    records.extend(file_records);
    // Pass two: the same tree, now registering the faces those sources declared.
    // 第二遍：同一棵树，这一次登记那些源码声明的面。
    run_pass(&records, &owned, &parents, &faces, None).map_or(Ok(()), |diagnostics| {
        Err(DeclaredFailure::Pipeline(diagnostics))
    })
}

/// Run the build-time pipeline with the faces a C++ build just compiled.
/// 带着刚编译出的 C++ 面运行构建期管线。
///
/// Each object file is paired with the **package-relative spelling** of the source
/// that produced it (`native/area/area.cpp`, say). That spelling is part of the
/// face's identity, while an object's path on disk is machine specific, so the
/// caller states the spelling instead of letting the build guess one from a path.
/// 每个目标文件都要配上产生它的源码在**包内的相对拼写**（例如 `native/area/area.cpp`）。
/// 这份拼写是面身份的一部分，而目标文件在磁盘上的路径与机器相关，因此由调用方写明，
/// 而不是让构建从路径里猜。
pub fn run_with_external(objects: &[external_face::ExternalSource<'_>]) -> Result<(), String> {
    let namespace = registry_identity::package_namespace();
    let records = external_face::records(objects, &namespace)?;
    let input = BuildInput::from_environment();
    match pipeline::run(&input, &records, &[], &[], &[], None) {
        Some(diagnostics) => Err(diagnostics.render_build_diagnostics()),
        None => Ok(()),
    }
}

/// Run the discovery and validation pipeline for an explicit host project,
/// outside a Cargo build script. `manifest` is the package **root directory** —
/// the one holding `Cargo.toml`, not the file — because every path this builds on
/// (`src/`, the cache, the generated output) is resolved against it. `package`
/// pins the identity namespace that Cargo would otherwise inject through
/// `CARGO_PKG_NAME`. `out_dir` receives the generated plan and manifests; when
/// validation fails the rendered diagnostics are returned.
/// 在 Cargo build script 之外为显式指定的宿主项目运行发现与校验管线。`manifest` 是包的
/// **根目录**——装着 `Cargo.toml` 的那个目录，而不是文件本身——因为本管线搭出的每条路径
/// （`src/`、缓存、生成产物）都以它为基准。`package` 固定身份命名空间（Cargo 本来会通过
/// `CARGO_PKG_NAME` 注入）。`out_dir` 接收生成的计划与清单；校验失败时返回渲染后的诊断。
pub fn run_for(manifest: &Path, out_dir: &Path, package: &str) -> Result<(), String> {
    // Keep the historical plain-text IO error: `check_for` reports an unwritable
    // `out_dir` as a diagnostic so its structured caller sees it too, but a
    // build-script-era caller must still get `create <dir>: <reason>`.
    // 保留历史上的纯文本 IO 错误：`check_for` 把不可写的 `out_dir` 也报成一条诊断，
    // 让结构化调用方同样看得见；而构建脚本时代的调用方仍应拿到
    // `create <dir>: <reason>`。
    std::fs::create_dir_all(out_dir)
        .map_err(|error| format!("create {}: {error}", out_dir.display()))?;
    let (only, facade) = legacy_shape_from_environment();
    // `false`: this entry is the **structured** caller — it returns diagnostics instead of panicking,
    // and a caller that wants cargo directives is a build script, which is `run_for_partition`'s job
    // (two tests pin that an unwritable tree is an `Err` here, not a panic).
    // `false`：这个入口是**结构化**调用方——它返回诊断而不 panic；想要 cargo 指令的调用方是构建脚本，那是
    // `run_for_partition` 的职责（两条测试钉住：这里写不了生成树时是 `Err`，不是 panic）。
    run_shape(manifest, out_dir, package, only, facade, false)
}

/// Run the pipeline for **one fragment of a partition**: this package compiles `only` (a comma-joined
/// list of claimed subtrees) instead of the whole host, or — with `None` and `facade` — it is the
/// cross-crate half.
/// 为**划分出来的一个碎片**运行管线：这个包编译 `only`（逗号分隔的认领子树表）而不是整个宿主；或者以
/// `None` + `facade` 表示它就是跨 crate 那一半。
///
/// The claim travels as an **argument** rather than as an environment variable, because a generated
/// build script could only set one with `unsafe { std::env::set_var(…) }` under Rust 2024, and a claim
/// is a literal the generator already knows.
/// 认领以**参数**传递而不是环境变量：Rust 2024 下生成的构建脚本只能靠 `unsafe { std::env::set_var(…) }`
/// 设置环境变量，而认领本来就是生成器已知的字面量。
pub fn run_for_partition(
    manifest: &Path,
    out_dir: &Path,
    package: &str,
    only: Option<&str>,
    facade: bool,
) -> Result<(), String> {
    run_for_partition_named(manifest, out_dir, package, only, facade, None)
}

/// The no-cuts partition entry with the host library name carried from the plan.
/// 不携带切口的分区入口，同时携带计划里的宿主库名。
pub fn run_for_partition_named(
    manifest: &Path,
    out_dir: &Path,
    package: &str,
    only: Option<&str>,
    facade: bool,
    host_crate: Option<&str>,
) -> Result<(), String> {
    run_shape_with_cuts_named(
        manifest,
        out_dir,
        package,
        only.map(str::to_owned),
        facade,
        true,
        &[],
        host_crate.map(str::to_owned),
    )
}

/// [`run_for_partition`] for a package that carries its host's graft cuts as **data**.
/// [`run_for_partition`]，用于把宿主的 graft 切口当作**数据**携带的包。
///
/// A generated package has no entry of its own, so without this argument the release shape's ghosts
/// compiled the copied faces and emitted an empty cut table — the graft the plan promised never left the
/// build (see [`HostCut`]; audit `M7`, §M7.55). The cuts are attributed to the packages that compile
/// them by the same rule a hand-written entry is, so a ghost emits its own and the facade emits the
/// union.
/// 生成包没有自己的入口，因此没有这个参数时，发布形状的幽灵编译了复制过来的面、却发射空切口表——计划
/// 承诺的嫁接从未离开构建（见 [`HostCut`]；审计 `M7`，§M7.55）。这些切口按与手写入口**同一条**归属规则
/// 归到编译它们的包上，因此幽灵发射自己那条、facade 发射并集。
pub fn run_for_partition_with_cuts(
    manifest: &Path,
    out_dir: &Path,
    package: &str,
    only: Option<&str>,
    facade: bool,
    cuts: &[HostCut],
) -> Result<(), String> {
    run_for_partition_with_cuts_named(manifest, out_dir, package, only, facade, cuts, None)
}

/// The same entry with the host's **library name** carried as an input from the committed plan.
/// 同一个入口，另外携带已提交计划里宿主**库名**作为输入。
pub fn run_for_partition_with_cuts_named(
    manifest: &Path,
    out_dir: &Path,
    package: &str,
    only: Option<&str>,
    facade: bool,
    cuts: &[HostCut],
    host_crate: Option<&str>,
) -> Result<(), String> {
    run_shape_with_cuts_named(
        manifest,
        out_dir,
        package,
        only.map(str::to_owned),
        facade,
        true,
        cuts,
        host_crate.map(str::to_owned),
    )
}

/// The shared body of [`run_for`] and [`run_for_partition`].
/// [`run_for`] 与 [`run_for_partition`] 共用的主体。
fn run_shape(
    manifest: &Path,
    out_dir: &Path,
    package: &str,
    only: Option<String>,
    facade: bool,
    emit_cargo_directives: bool,
) -> Result<(), String> {
    run_shape_with_cuts(
        manifest,
        out_dir,
        package,
        only,
        facade,
        emit_cargo_directives,
        &[],
    )
}

/// [`run_shape`] with the host's cuts handed in as data.
/// [`run_shape`]，外加以数据交进来的宿主切口。
#[allow(clippy::too_many_arguments)]
fn run_shape_with_cuts(
    manifest: &Path,
    out_dir: &Path,
    package: &str,
    only: Option<String>,
    facade: bool,
    emit_cargo_directives: bool,
    cuts: &[HostCut],
) -> Result<(), String> {
    run_shape_with_cuts_named(
        manifest,
        out_dir,
        package,
        only,
        facade,
        emit_cargo_directives,
        cuts,
        None,
    )
}

#[allow(clippy::too_many_arguments)]
fn run_shape_with_cuts_named(
    manifest: &Path,
    out_dir: &Path,
    package: &str,
    only: Option<String>,
    facade: bool,
    emit_cargo_directives: bool,
    cuts: &[HostCut],
    host_crate: Option<String>,
) -> Result<(), String> {
    std::fs::create_dir_all(out_dir)
        .map_err(|error| format!("create {}: {error}", out_dir.display()))?;
    // A **build script** has to emit cargo's directives, and one of them matters here: the
    // `rustc-check-cfg=cfg(rust_analyzer)` line that keeps the generated tree's rust-analyzer mirror
    // from warning as an unexpected cfg. Passing `false` (the CLI's setting) left every generated
    // package warning on its own build — measured: `unexpected_cfgs` on `partitioned-button-facade`.
    // **构建脚本**必须发 cargo 指令，其中一条在这里很要紧：`rustc-check-cfg=cfg(rust_analyzer)`，它让生成树里
    // 给 rust-analyzer 的镜像不再被当成未知 cfg 警告。传 `false`（CLI 的设置）会让每个生成包在自己的构建里报警
    // ——实测：`partitioned-button-facade` 上的 `unexpected_cfgs`。
    check_for_shape_with_cuts_named(
        manifest,
        out_dir,
        package,
        only,
        facade,
        emit_cargo_directives,
        cuts,
        host_crate,
    )
    .map_err(|diagnostics| diagnostics.render_build_diagnostics())
}

/// The shape the **earlier** generator declared, read from the two variables it set.
/// **早先**的生成器声明的形状，从它设置的两个变量读回。
///
/// Kept so a tree someone already partitioned keeps rendering exactly the same plan without
/// re-running the writer; the current generator writes neither variable, and its build scripts set
/// nothing at all.
/// 保留它是为了让已经划分过的树无需重跑写入方就继续渲染出同一份计划；现在的生成器两个变量都不写，
/// 它生成的构建脚本什么都不设置。
fn legacy_shape_from_environment() -> (Option<String>, bool) {
    (
        std::env::var(xirang_kernel::lexicon::SHAPE_ONLY_ENV).ok(),
        std::env::var(xirang_kernel::lexicon::SHAPE_FACADE_ENV).is_ok(),
    )
}

#[path = "check_entry.rs"]
mod check_entry;
pub use check_entry::{
    check_for, check_for_shape, check_for_shape_with_cuts, check_for_shape_with_cuts_named,
};
