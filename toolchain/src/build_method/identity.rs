//! Build-time identity helpers layered on the kernel identities.
//! 基于 kernel 身份的构建期身份辅助函数。

use std::cell::RefCell;
use std::sync::OnceLock;

pub use xirang_kernel::registry_core::identity::NodeId;

/// Version of the identity input and persisted catalog formats.
/// 身份输入与持久化目录格式的版本。
///
/// The kernel owns the value: a build that carried its own copy could accept a
/// `XIRANG_SCOPE` or a plugin lock the kernel rejects, or the other way
/// round, with no compiler noticing.
/// 该值由内核拥有：构建若自带一份副本，就可能接受内核拒绝的 `XIRANG_SCOPE`
/// 或插件锁（反之亦然），而编译器不会察觉。
pub use xirang_kernel::registry_core::identity::IDENTITY_SCHEMA;

/// Explicit namespace for standalone runs (CLI, tests). Cargo build scripts
/// read `CARGO_PKG_NAME` from the environment instead.
/// 独立运行（CLI、测试）使用的显式命名空间；Cargo build script 改从环境变量读取。
static PACKAGE_NAMESPACE_OVERRIDE: OnceLock<String> = OnceLock::new();

thread_local! {
    /// The namespace one in-process pipeline run has in force **on this thread**.
    /// 一次进程内管线运行**在本线程上**生效的命名空间。
    ///
    /// [`package_namespace`] reads this first. It is what makes a run's identities
    /// belong to the package being run rather than to whichever package happened
    /// to pin [`PACKAGE_NAMESPACE_OVERRIDE`] first in this process: a bridge or a
    /// Studio session serves many packages from one process, and an identity
    /// stamped with another package's namespace is a wrong answer that looks like
    /// a right one — it is the input to every `NodeId` this build publishes.
    /// [`package_namespace`] 最先读它。正因如此，一次运行的身份才属于正在运行的那个包，而不是本进程里
    /// 最先固定 [`PACKAGE_NAMESPACE_OVERRIDE`] 的那个包：桥或 Studio 会话会在一个进程里服务多个包，而
    /// 用别的包命名空间盖下的身份是一个"看起来正确"的错误答案——它是这次构建发布的每一个 `NodeId`
    /// 的输入。
    ///
    /// It is thread-local, and not a process-wide cell, because the pipeline is
    /// single-threaded: a run's namespace is nobody else's business, while a
    /// process-wide one would flip the identities a *concurrent* reader computes
    /// — which is exactly what the first-write-wins pin below exists to prevent.
    /// 它是线程局部的而不是进程级单元，因为管线是单线程的：一次运行的命名空间与别人无关，而进程级的
    /// 那个会翻转**并发**读取方算出的身份——那正是下面那个"先到先得"固定值要防止的事。
    static RUN_NAMESPACE: RefCell<Option<String>> = const { RefCell::new(None) };
}

/// Pin the package namespace for the rest of the process. Used by
/// `run_for`, which executes outside Cargo and therefore has no
/// `CARGO_PKG_NAME` in its environment.
/// 为当前进程固定包命名空间。由 `run_for` 使用——它在 Cargo 之外执行，
/// 环境里没有 `CARGO_PKG_NAME`。
pub fn set_package_namespace(namespace: String) {
    let _ = PACKAGE_NAMESPACE_OVERRIDE.set(namespace);
}

/// Run `body` as the pipeline for `namespace`, with that namespace in force on
/// this thread for the whole call.
/// 以 `namespace` 的管线身份运行 `body`：该命名空间在整个调用期间于本线程生效。
///
/// The previous value is restored even when `body` panics, so one run cannot
/// leak its namespace into whatever runs next on the same thread.
/// 即使 `body` panic 也会恢复之前的值，因此一次运行不会把它的命名空间泄漏给同线程接下来的运行。
pub(crate) fn run_as_package<R>(namespace: &str, body: impl FnOnce() -> R) -> R {
    let previous = RUN_NAMESPACE.with(|slot| slot.replace(Some(namespace.to_owned())));
    let restore = RestoreNamespace(previous);
    let result = body();
    drop(restore);
    result
}

/// Puts the run's namespace back when it is dropped.
/// 被丢弃时把这次运行的命名空间放回去。
struct RestoreNamespace(Option<String>);

impl Drop for RestoreNamespace {
    fn drop(&mut self) {
        RUN_NAMESPACE.with(|slot| *slot.borrow_mut() = self.0.take());
    }
}

/// Pin a deterministic namespace for the whole test process.
/// 为整个测试进程固定一个确定的命名空间。
///
/// The override is first-write-wins. Freezing it at fixture creation means a
/// concurrent `run_for` test can no longer flip the namespace between another
/// test's rule-collection and rule-lookup scans, which previously made the
/// parent-rule lookup miss and rendered empty diagnostics.
/// 该覆盖是先到先得。在夹具创建时冻结后，并发的 `run_for` 测试就无法在
/// 另一个测试收集规则与查找规则之间翻转命名空间——那会导致父规则查找
/// 失配、诊断渲染为空。
#[cfg(test)]
pub(crate) fn freeze_test_namespace() {
    set_package_namespace("xirang-toolchain-tests".to_owned());
}

/// Return the namespace of the package whose build script is currently running.
/// 返回当前正在运行的构建脚本所属包的命名空间。
///
/// Cargo exposes the consuming package name to a build-script process. Using
/// that value keeps build-time identities byte-for-byte compatible with the
/// `crate::XIRANG_NAMESPACE` value the declaration macros read, which `host!()` defines from `env!("CARGO_PKG_NAME")` (audit `M7`, P3.3).
/// Cargo 把消费方包名暴露给构建脚本进程。使用该值让构建期身份与声明宏捕获的
/// `crate::XIRANG_NAMESPACE` 逐字节一致——那个常量由 `host!()` 用 `env!("CARGO_PKG_NAME")` 定义（审计 `M7`，P3.3）。
///
/// An in-process run's namespace wins over both the pinned override and
/// `CARGO_PKG_NAME`; see [`IN_PROCESS_NAMESPACE`].
/// 进程内运行的命名空间优先于固定覆盖与 `CARGO_PKG_NAME`；见 [`IN_PROCESS_NAMESPACE`]。
pub fn package_namespace() -> String {
    if let Some(active) = active_namespace() {
        return active;
    }
    PACKAGE_NAMESPACE_OVERRIDE
        .get()
        .cloned()
        .unwrap_or_else(|| {
            std::env::var("CARGO_PKG_NAME").unwrap_or_else(|_| "xirang.default".to_owned())
        })
}

/// The namespace an in-process run has in force on this thread, if any.
/// 本线程上进程内运行当前生效的命名空间（若有）。
///
/// Read on its own rather than folded into the answer above so a test can ask
/// "is a run active?" without also asking what the process pinned.
/// 单独读取，而不是折进上面的答案里，这样测试可以只问"有运行在生效吗"，而不必同时问进程固定了什么。
fn active_namespace() -> Option<String> {
    RUN_NAMESPACE.with(|slot| slot.borrow().clone())
}

/// Compute the identity used by generated plans and caches.
/// 计算生成计划与缓存所使用的身份。
pub fn package_node_id(relative_path: &str, declared_name: &str) -> NodeId {
    let namespace = package_namespace();
    NodeId::from_namespaced_path(&namespace, relative_path, declared_name)
}

/// Compute the root identity used by generated plans and caches.
/// 计算生成计划与缓存所使用的根身份。
pub fn package_root_node_id() -> NodeId {
    let namespace = package_namespace();
    NodeId::from_namespaced_path(&namespace, "<root>", "root")
}

#[cfg(test)]
mod tests {
    use super::{NodeId, freeze_test_namespace, package_namespace, package_node_id};

    #[test]
    fn package_identity_matches_the_macro_namespace_algorithm() {
        // Pin the namespace before reading it: the override is first-write-wins,
        // so a concurrent test pinning the test namespace between the two reads
        // below would make the comparison disagree with itself.
        // 先固定命名空间再读取：该覆盖是先到先得，若并发测试在这两次读取之间固定
        // 测试命名空间，比较就会与自身不一致。
        freeze_test_namespace();
        let namespace = package_namespace();
        let path = "control/object/button/button.rs";
        let kind = "Button";
        assert_eq!(
            package_node_id(path, kind),
            NodeId::from_namespaced_path(&namespace, path, kind)
        );
        assert_ne!(
            NodeId::from_namespaced_path("library-a", path, kind),
            NodeId::from_namespaced_path("library-b", path, kind)
        );
    }

    /// A run's own namespace wins over whatever this process pinned, and it is
    /// gone once the run ends.
    /// 一次运行自己的命名空间优先于本进程已固定的值，而运行结束后它就不再生效。
    ///
    /// Measured failure this exists against: a second `check_for` in one process
    /// was ignored by the first-write-wins pin, so its pruning manifest was
    /// stamped with the *first* package's namespace and `xirang.diff` reported
    /// every face as re-identified (`ba9a8808…` for the run, `77fc3680…` for the
    /// sources, measured on this fixture).
    /// 这条测试针对的实测失败：进程内第二次 `check_for` 被"先到先得"的固定值忽略，于是它的剪枝清单
    /// 盖上了**第一个**包的命名空间，`xirang.diff` 把每个面都报成身份变了（本夹具上实测为运行侧
    /// `ba9a8808…`、源码侧 `77fc3680…`）。
    ///
    /// The assertions read the active namespace rather than the process answer,
    /// because `freeze_test_namespace` may pin the process between two reads of
    /// the latter; the active slot is this thread's alone.
    /// 断言读的是"生效中的命名空间"而不是进程级答案，因为 `freeze_test_namespace` 可能在两次读取
    /// 后者之间固定进程的值；而生效槽位只属于本线程。
    #[test]
    fn a_run_namespace_wins_over_the_pin_and_puts_it_back() {
        assert_eq!(super::active_namespace(), None);
        assert_eq!(
            super::run_as_package("scoped-package", package_namespace),
            "scoped-package"
        );
        assert_eq!(super::active_namespace(), None);
    }
}
