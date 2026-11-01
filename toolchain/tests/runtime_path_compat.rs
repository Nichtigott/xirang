//! Public path compatibility, pinned in one place.
//! 公开路径兼容性，集中钉在这里。
//!
//! P2 made the module hierarchy the kernel's official surface and narrowed the
//! crate root to a whitelist. A host, an editor and every execution surface had
//! four ways to name one identity before that change, and all four have to keep
//! resolving: the official module path, the kernel's own module page, the
//! surface's flat re-export, and the surface's kernel module path. This file
//! fails to compile if any of them disappears.
//! P2 让模块层级成为内核的官方表面，并把 crate 根部收窄为白名单。改动之前，宿主、编辑器
//! 与每个执行面有四种方式命名同一个身份，四种都必须继续可解析：官方模块路径、内核自己的
//! 模块页、执行面的平铺重导出、执行面的内核模块路径。任何一种消失，本文件就编译不过。

/// The identity namespace this test crate's faces are compiled under.
/// 本测试 crate 的注册面编译时所用的身份命名空间。
///
/// A crate that declares faces without `host!()` owns this constant itself — the declaration macros
/// read it (audit `M7`, P3.3) — so a test crate provides the same one-line value a host gets from
/// `host!()`. It lives at this file's top level because the invocations sit inside `mod` blocks and
/// `crate::` in their expansion resolves to the crate root.
/// 一个声明注册面却不调用 `host!()` 的 crate 自己拥有这个常量——声明宏读它（审计 `M7`，P3.3）——因此测试
/// crate 要提供宿主从 `host!()` 得到的那一行同样的值。它放在本文件的顶层，因为那些调用位于 `mod` 块里，
/// 而展开里的 `crate::` 解析到 crate 根。
pub const XIRANG_NAMESPACE: &str = env!("CARGO_PKG_NAME");

use std::marker::PhantomData;

use xirang_kernel::identity::NodeId as OfficialNodeId;
use xirang_kernel::registry_core::identity::NodeId as KernelNodeId;
use xirang_toolchain::run_method::NodeId as SurfaceNodeId;
use xirang_toolchain::run_method::registry_core::identity::NodeId as SurfaceKernelNodeId;

/// The same identity, reached four ways: module path, kernel module page, the
/// surface's flat re-export, and the surface's kernel module path.
/// 同一个身份的四条路径：模块路径、内核模块页、执行面平铺重导出、执行面的内核模块路径。
#[test]
fn one_identity_resolves_through_every_historical_path() {
    let official = OfficialNodeId::from_namespaced_path("ns", "a/a.rs", "A");
    assert_eq!(
        official,
        KernelNodeId::from_namespaced_path("ns", "a/a.rs", "A")
    );
    assert_eq!(
        official,
        SurfaceNodeId::from_namespaced_path("ns", "a/a.rs", "A")
    );
    assert_eq!(
        official,
        SurfaceKernelNodeId::from_namespaced_path("ns", "a/a.rs", "A")
    );
}

/// The root whitelist keeps the nouns host code writes as bare names, while the
/// module page stays the place a type is owned.
/// 根部白名单保留宿主代码以裸名书写的名词；类型仍由所属模块页拥有。
#[test]
fn the_whitelist_and_the_module_pages_both_hold() {
    let _: PhantomData<xirang_kernel::NodeId> = PhantomData;
    let _: PhantomData<xirang_kernel::Registry> = PhantomData;
    let _: PhantomData<xirang_kernel::identity::NodeId> = PhantomData;
    let _: PhantomData<xirang_kernel::declaration::RegistrationInfo> = PhantomData;
    let _: PhantomData<xirang_kernel::plugin::catalog::PluginCatalog> = PhantomData;
    let _: PhantomData<xirang_kernel::plugin::graft_document::GraftPlanDocument> = PhantomData;
    let _: PhantomData<xirang_kernel::plugin::graft::GraftCut> = PhantomData;
    assert_eq!(
        xirang_kernel::lexicon::GENERATED_LIB_FILE,
        "generated_lib.rs"
    );
}

/// The surfaces keep every kernel name they had: a host that writes
/// `xirang_toolchain::run_method::PluginManifest` or `xirang_toolchain::run_method::sha256_hex`
/// must not notice the kernel root getting smaller.
/// 执行面保留它们原有的全部内核名字：写 `xirang_toolchain::run_method::PluginManifest` 或
/// `xirang_toolchain::run_method::sha256_hex` 的宿主不该察觉内核根部变小了。
#[test]
fn the_surface_keeps_its_flat_kernel_names() {
    let _: PhantomData<xirang_toolchain::run_method::PluginManifest> = PhantomData;
    let _: PhantomData<xirang_toolchain::run_method::Registry> = PhantomData;
    let _: PhantomData<xirang_toolchain::run_method::plugin::catalog::PluginCatalog> = PhantomData;
    assert_eq!(xirang_toolchain::run_method::sha256_hex(b"").len(), 64);
}

/// Every kernel module page is part of the official surface, so a host that
/// reaches a noun through its module must keep being able to. One
/// representative item per module pins that the page itself still exists.
/// 每个内核模块页都是官方表面的一部分，因此通过模块取得名词的宿主必须保持可用。
/// 每个模块取一个有代表性的 item，钉住该页本身仍然存在。
#[test]
fn every_kernel_module_page_still_resolves() {
    let _: PhantomData<xirang_kernel::identity::NodeId> = PhantomData;
    let _: PhantomData<xirang_kernel::declaration::RegistrationInfo> = PhantomData;
    let _: PhantomData<xirang_kernel::diagnostic::RegistryError> = PhantomData;
    let _: PhantomData<xirang_kernel::mir::MirGraph> = PhantomData;
    let _: PhantomData<xirang_kernel::release::StaticPlan> = PhantomData;
    let _: PhantomData<xirang_kernel::requirements::CapabilityDeclaration> = PhantomData;
    let _: PhantomData<xirang_kernel::source::SourceFunction> = PhantomData;
    let _: PhantomData<xirang_kernel::tree::Registry> = PhantomData;
    let _: PhantomData<xirang_kernel::plugin::catalog::PluginCatalog> = PhantomData;
    let _: PhantomData<xirang_kernel::registry_core::identity::NodeId> = PhantomData;
    let _: PhantomData<xirang_kernel::registry_core::tree::Registry> = PhantomData;
    assert_eq!(
        xirang_kernel::lexicon::GENERATED_LIB_FILE,
        "generated_lib.rs"
    );
    assert_eq!(
        xirang_kernel::authoring::FACE_FIELD_COUNT,
        xirang_kernel::authoring::face_field::FACE_FIELD_COUNT
    );
}

/// The plugin protocol keeps its module pages on both the kernel and the
/// surface shim; a host that wrote `xirang_toolchain::run_method::plugin::trust::…`
/// must not notice the shim narrowing.
/// 插件协议在内核与执行面 shim 两侧都保留模块页；写过
/// `xirang_toolchain::run_method::plugin::trust::…` 的宿主不该察觉 shim 变窄。
#[test]
fn the_plugin_module_pages_still_resolve() {
    let _: PhantomData<xirang_kernel::plugin::artifact::PluginArtifact> = PhantomData;
    let _: PhantomData<xirang_kernel::plugin::contracts::FlowContract> = PhantomData;
    let _: PhantomData<xirang_kernel::plugin::graft::GraftCut> = PhantomData;
    let _: PhantomData<xirang_kernel::plugin::plugin_policy::PluginPolicy> = PhantomData;
    let _: PhantomData<xirang_kernel::plugin::slot::PluginChannel> = PhantomData;
    let _: PhantomData<xirang_kernel::plugin::trust::PluginTrustPolicy> = PhantomData;
    let _: PhantomData<xirang_toolchain::run_method::plugin::catalog::PluginCatalog> = PhantomData;
    let _: PhantomData<xirang_toolchain::run_method::plugin::graft_document::GraftPlanDocument> =
        PhantomData;
    let _: PhantomData<xirang_toolchain::run_method::plugin::trust::PluginTrustError> = PhantomData;
}

/// The runtime evidence surface stays reachable both through the
/// `runtime::*` page and through the historical `runtime::trace::locals::*`
/// page.
/// 运行期证据表面既可经 `runtime::*` 页抵达，也可经历史的
/// `runtime::trace::locals::*` 页抵达。
#[test]
fn the_runtime_and_locals_pages_still_resolve() {
    let _: PhantomData<xirang_toolchain::run_method::runtime::CallTrace> = PhantomData;
    let _: PhantomData<xirang_toolchain::run_method::runtime::Coordinates> = PhantomData;
    let _: PhantomData<xirang_toolchain::run_method::runtime::Provenance> = PhantomData;
    let _: PhantomData<xirang_toolchain::run_method::runtime::RuntimeValue> = PhantomData;
    let _: PhantomData<xirang_toolchain::run_method::runtime::trace::CallTrace> = PhantomData;
    let _: PhantomData<xirang_toolchain::run_method::runtime::trace::locals::LocalId> = PhantomData;
    let _: PhantomData<xirang_toolchain::run_method::runtime::trace::locals::LocalKind> =
        PhantomData;
    let _: PhantomData<xirang_toolchain::run_method::runtime::trace::locals::LocalValue> =
        PhantomData;
    let _: PhantomData<xirang_toolchain::run_method::runtime::trace::locals::Observation> =
        PhantomData;
}

/// The exported macro vocabulary keeps resolving under its public names, and
/// under the hidden helpers the generated aliases call. An unresolved macro
/// import is a hard compile error, which is the whole point of this file.
/// 导出的宏词表在其公开名称下保持可解析，隐藏辅助宏（生成的别名会调用它们）同样如此。
/// 宏导入无法解析就是硬编译错误——这正是本文件存在的意义。
#[test]
fn the_public_macro_names_still_resolve() {
    #[allow(unused_imports)]
    use xirang_toolchain::run_method::{external_object, face_fields, host, static_graft_plan};
    #[allow(unused_imports)]
    use xirang_toolchain::{
        __admission, __assert_impls, __control_object, __external_object, __face_string_or,
        __face_ty_name_or, __face_ty_or, __face_value_or, __flow, __flow_provider, __flow_select,
        __plugin, __registration_face, __stable_name, __string_list, __submit_registration,
        __xirang_object,
    };
    // The plan and tracing macros are `#[macro_export]`, so an external caller names them at the
    // crate root — the same hoisting that makes `__control_object!` a root path.
    // 计划与追踪宏是 `#[macro_export]`，因此外部调用者按 crate 根寻址——与 `__control_object!`
    // 被提升到根是同一件事。
    // The pin is that these names *resolve*; whether this test calls each one is not the point,
    // which is what the sibling compatibility blocks already say with the same attribute.
    // 这条钉子钉的是这些名字**能解析**；本测试是否逐个调用它们不是重点——旁边那几块兼容钉子
    // 用同一个属性表达的就是这件事。
    #[allow(unused_imports)]
    use xirang_toolchain::{
        application, graft_plan, trace_call, trace_call_result, trace_consume, trace_transform,
        trace_value,
    };
    assert_eq!(
        xirang_toolchain::run_method::lexicon::GENERATED_LIB_FILE,
        "generated_lib.rs"
    );
}

/// The authoring surface is feature-gated, so it is pinned only when that
/// feature is on: the executor's public entry points and submodules must keep
/// their names. `edit_module` is deliberately absent — B3a deleted it.
/// authoring 表面受特性门控，因此只在打开该特性时钉住：执行器的公开入口与子模块必须
/// 保持名字。`edit_module` 刻意不在其中——B3a 已删除它。
#[cfg(feature = "authoring")]
#[test]
fn the_authoring_surface_still_resolves() {
    #[allow(unused_imports)]
    use xirang_toolchain::run_method::authoring::{
        AuthoringChange, AuthoringContext, FACE_FIELD_COUNT, ModuleFacePatch, NewModuleFace,
        add_module, add_module_from_face, add_module_with_registration, delete_module,
        edit_module_face, generated_snapshots, generated_snapshots_from,
    };
    #[allow(unused_imports)]
    use xirang_toolchain::run_method::authoring::{
        external_graft, filesystem, manifest, operations, parse, snapshot, validation,
    };
    assert_eq!(
        xirang_toolchain::run_method::authoring::FACE_FIELD_COUNT,
        xirang_toolchain::run_method::authoring::face_field::FACE_FIELD_COUNT
    );
}
