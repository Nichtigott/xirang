//! The named entry point to a host's own runtime registry.
//! 宿主拿到自己运行期注册机的具名入口。

use crate::run_method::registry_core::{FrameworkId, RegistrationInfo, Registry, RegistryResult};

/// Assemble this crate's runtime registry from the registrations its build produced.
/// 用本次构建产出的注册装配本 crate 的运行期注册机。
///
/// Call it through [`host_registry!`](crate::host_registry!), which supplies the three arguments from
/// the crate root the generated entry already populates.
/// 请经 [`host_registry!`](crate::host_registry!) 调用：它从生成入口已经填好的 crate 根取那三个实参。
///
/// **Why this exists.** Every piece was public and generated — `registrations()` in
/// `OUT_DIR/generated_lib.rs`, [`Registry::root_for_namespace`] and [`Registry::register_all`] in the
/// kernel — and the hand-off list still recorded "a host has **no public way to reach its registry at
/// runtime**" (audit `M7`, §M7.62). That was right in the way that matters: the assembly had no
/// *name*, so every host that wanted one re-derived the incantation from whichever example it found —
/// measured, `examples/control-button` hand-wrote exactly these lines before this entry existed.
/// **为什么有它。** 每一块都是公开且生成的——`registrations()` 在 `OUT_DIR/generated_lib.rs` 里，
/// [`Registry::root_for_namespace`] 与 [`Registry::register_all`] 在内核里——而交接清单仍记着"宿主
/// **没有公开路径**在运行期拿到自己的注册机"（审计 `M7`，§M7.62）。那句话在要紧的意义上是对的：
/// 这次装配**没有名字**，于是每个想要的宿主都从它碰到的某个示例反推那句咒语——实测，
/// `examples/control-button` 在本入口存在之前手写的正是这几行。
///
/// **Two rules it keeps in one place.** The framework and the namespace must be the pair the *build*
/// used: a face is mounted under `hash(namespace, source path, name)`, and a partitioned crate compiles
/// the **host's** namespace, so pairing them wrongly yields a registry that answers for a different
/// tree. And a registration whose parent is absent is **refused**, not dropped —
/// [`Registry::register_all`] is atomic, so a half-populated tree cannot escape from here.
/// **它把两条规则收到一处。** 框架与命名空间必须是**构建**当时那一对：面挂在
/// `hash(命名空间, 源码路径, 名字)` 之下，而分区后的 crate 编译的是**宿主的**命名空间，因此配错会得到
/// 一台为另一棵树作答的注册机。父级缺失的注册会被**拒绝**而不是丢弃——[`Registry::register_all`]
/// 是原子的，因此半填的树不可能从这里逃出去。
pub fn host_registry(
    framework: FrameworkId,
    namespace: &str,
    registrations: &[RegistrationInfo],
) -> RegistryResult<Registry> {
    let mut registry = Registry::root_for_namespace(framework, namespace);
    registry.register_all(registrations)?;
    Ok(registry)
}

#[cfg(test)]
#[path = "host_registry_tests.rs"]
mod host_registry_tests;
