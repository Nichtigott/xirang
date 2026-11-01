//! The authoring execution context: `AuthoringContext` and the process-level
//! paths and namespaces one file-bound authoring operation reads.
//! 创作执行上下文：`AuthoringContext`，以及一次绑定文件的创作操作所读取的进程级路径与命名空间。
//!
//! The file was named `validation` because the kernel's validators are re-exported
//! here — the validators themselves live in the kernel `authoring` module. What this
//! file *holds* is the context and its environment fallback chain, so that name led a
//! reader looking for validation rules to the wrong file; audit `NAM-03` asked for the
//! rename to `context`, and this is it. The historical public path
//! `crate::run_method::authoring::validation` is **not** gone: `authoring.rs` keeps it
//! as an alias (`pub use self::context as validation;`), and that alias is also how the
//! re-export pinned in `conventions/src/shims.rs` stays reachable at the old address.
//! 本文件曾名为 `validation`，是因为内核的校验函数在这里重导出——校验函数本体在 kernel 的
//! `authoring` 模块。本文件**装**的是上下文及其环境变量回落链，因此那个名字会把找校验规则的读者
//! 引到错的文件；审计 `NAM-03` 要的正是改成 `context`，这就是那次改名。历史公开路径
//! `crate::run_method::authoring::validation` **并没有消失**：`authoring.rs` 用别名
//! （`pub use self::context as validation;`）保留了它，钉在 `conventions/src/shims.rs` 里的那条
//! 重导出也正是靠它继续在旧地址上可达。

use std::cell::RefCell;
use std::path::{Path, PathBuf};

pub use xirang_kernel::authoring::validation::*;

/// Filesystem and identity scope for one authoring operation.
/// 单次注册面创作操作使用的文件系统与身份上下文。
#[derive(Clone, Debug)]
pub struct AuthoringContext {
    package_root: PathBuf,
    namespace: String,
}

thread_local! {
    static ACTIVE_CONTEXT: RefCell<Option<AuthoringContext>> = const { RefCell::new(None) };
}

struct ContextRestore(Option<AuthoringContext>);

impl Drop for ContextRestore {
    fn drop(&mut self) {
        ACTIVE_CONTEXT.with(|active| {
            *active.borrow_mut() = self.0.take();
        });
    }
}

impl AuthoringContext {
    /// Pair a package root with the namespace its faces are authored under.
    /// 把一个包根与其注册面创作所用的命名空间配对。
    ///
    /// The values are inert until [`AuthoringContext::scope`] installs them, so
    /// constructing one never touches process-global state.
    /// 这些取值在 [`AuthoringContext::scope`] 安装之前不生效，因此构造本身不会触碰
    /// 进程级状态。
    pub fn new(package_root: impl Into<PathBuf>, namespace: impl Into<String>) -> Self {
        Self {
            package_root: package_root.into(),
            namespace: namespace.into(),
        }
    }

    /// Run an operation in this context without changing process environment.
    /// 在此上下文中执行操作，不修改进程环境变量。
    pub fn scope<T>(&self, operation: impl FnOnce() -> T) -> T {
        let previous = ACTIVE_CONTEXT.with(|active| active.replace(Some(self.clone())));
        let _restore = ContextRestore(previous);
        operation()
    }
}

/// The namespace this operation authors under: the active context first, then
/// the environment, then the documented default.
/// 本次操作创作所用的命名空间：先活动上下文，再环境变量，最后文档化的默认值。
///
/// The decision itself is `xirang_kernel::lexicon::resolve_namespace`, shared with
/// Studio and the MCP bridge; this function only supplies what it reads.
/// 决策本身是 `xirang_kernel::lexicon::resolve_namespace`，与 Studio 和 MCP 桥共用；本函数
/// 只负责提供它读取的东西。
pub(super) fn authoring_namespace() -> String {
    ACTIVE_CONTEXT
        .with(|active| {
            active
                .borrow()
                .as_ref()
                .map(|context| context.namespace.clone())
        })
        .unwrap_or_else(|| {
            xirang_kernel::lexicon::resolve_namespace(
                std::env::var(xirang_kernel::lexicon::NAMESPACE_ENV)
                    .ok()
                    .as_deref(),
            )
            .to_owned()
        })
}

/// Legacy layout kept readable so existing projects can migrate gradually.
/// 保留旧布局读取能力，现有项目可以逐步迁移。
pub(super) fn legacy_rule_path_for_source(source: &str) -> String {
    let directory = Path::new(source).parent().unwrap_or_else(|| Path::new(""));
    format!("src/{}/registry/rules/rules.rs", normalized_path(directory))
}

/// The package this operation writes into: the active context first, then the
/// shared `lexicon` rule over the environment and the working directory.
/// 本次操作写入的包：先活动上下文，再对环境和当前目录套用共用的 `lexicon` 规则。
pub(super) fn package_root() -> PathBuf {
    if let Some(root) = ACTIVE_CONTEXT.with(|active| {
        active
            .borrow()
            .as_ref()
            .map(|context| context.package_root.clone())
    }) {
        return root;
    }
    let configured = std::env::var_os(xirang_kernel::lexicon::PACKAGE_ROOT_ENV).map(PathBuf::from);
    let current = std::env::current_dir().ok();
    xirang_kernel::lexicon::resolve_package_root(
        configured.as_deref(),
        current.as_deref(),
        current
            .as_ref()
            .is_some_and(|directory| directory.join("Cargo.toml").is_file()),
        Path::new(env!("CARGO_MANIFEST_DIR")),
    )
}

/// The layout this operation writes into, from the one shared resolver.
/// 本次操作写入的布局，取自那一份共用的解析器。
///
/// This is the build's own answer, not a copy of it: the resolver lives at the crate
/// root ([`crate::source_layout`]) so it is reachable with the `authoring` feature
/// alone. A feature-dependent copy is how a `[lib] path = "host/lib.rs"` package got
/// its faces written into `src/` — the file was created, the build never read it, and
/// nothing said so.
/// 这是构建自己的答案，不是它的副本：解析器住在 crate 根（[`crate::source_layout`]），因此只开
/// `authoring` 特性也到得了。依赖特性的副本正是 `[lib] path = "host/lib.rs"` 的包把面写进 `src/`
/// 的原因——文件创建了，构建从不读它，而没有任何东西说出口。
pub(super) fn layout() -> Result<crate::source_layout::SourceLayout, String> {
    crate::source_layout::source_layout(&package_root())
}

/// The directory the build walks for this package.
/// 构建为本包遍历的目录。
///
/// It is `package_root/src` for every historical layout, and the library target's
/// directory for a `[lib] path = "host/lib.rs"` layout. Writing a face into the
/// historical `src/` there produced a file the build never read, which is why the
/// writer asks the same resolver the build does — and why a layout that fails to
/// resolve is an error rather than a fallback: falling back to `src/` would recreate
/// exactly that silent wrong file.
/// 对一切历史布局它是 `package_root/src`；对 `[lib] path = "host/lib.rs"` 布局它是库目标所在目录。
/// 在那种布局下把面写进历史性的 `src/` 会产出一个构建永远不读的文件——这正是写入方要问构建所用的
/// 同一个解析器的原因，也是解析不出的布局必须报错而不是回落的原因：回落 `src/` 会原样重现那个静默写错
/// 的文件。
pub(super) fn scan_root() -> Result<PathBuf, String> {
    Ok(layout()?.scan_root)
}

/// The prefix that turns a module's record spelling into its package-relative spelling.
/// 把模块的记录拼写变成包内相对拼写的前缀。
///
/// The read-back parser builds a parent's record spelling from a module address; the layout's scan
/// root offset (`source/` for `[lib] path = "source/lib.rs"`, empty for `src/`) is exactly the part
/// the module address cannot carry. It is one fact, computed once here from the resolved layout —
/// never guessed from a path.
/// 读回解析器从模块地址构造父的记录拼写；布局的扫描根偏移（`[lib] path = "source/lib.rs"` 时是
/// `source/`，`src/` 时为空）正是模块地址带不了的那一段。它是一个事实，在这里从已解析布局算一次——
/// 绝不从某个路径去猜。
pub(super) fn identity_prefix() -> Result<String, String> {
    let scan = scan_root()?;
    let identity = identity_base()?;
    Ok(match scan.strip_prefix(&identity) {
        Ok(prefix) if !prefix.as_os_str().is_empty() => {
            format!("{}/", prefix.to_string_lossy().replace('\\', "/"))
        }
        _ => String::new(),
    })
}

/// The base a face's identity path is relative to.
/// 面的身份路径所相对的基准。
///
/// `FaceView::source` and the declaration macros' `manifest_relative_source` are
/// both relative to this base, so a path read back from the registry must be joined
/// here, not to [`scan_root`]. They coincide under `src/` and diverge for a target
/// outside it.
/// `FaceView::source` 与声明宏的 `manifest_relative_source` 都相对这个基准，因此从注册机读回的路径
/// 必须拼到这里，而不是拼到 [`scan_root`]。两者在 `src/` 下重合，对 `src/` 之外的目标则分离。
pub(super) fn identity_base() -> Result<PathBuf, String> {
    Ok(layout()?.identity_base)
}

#[cfg(test)]
mod tests {
    use super::{AuthoringContext, authoring_namespace, identity_base, package_root, scan_root};
    use std::path::Path;

    /// A throwaway package whose manifest declares the given `[lib]` table body.
    /// 一个临时包，其清单声明给定的 `[lib]` 表体。
    fn package(label: &str, lib_table: &str) -> std::path::PathBuf {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "xirang-layout-{label}-{}-{sequence}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(
            root.join("Cargo.toml"),
            format!("[package]\nname=\"layout-{label}\"\nversion=\"0.0.0\"\nedition=\"2024\"\n{lib_table}"),
        )
        .unwrap();
        root
    }

    /// The supervisor's custom-source-root defect, at the resolver the writer uses: with
    /// `[lib] path = "source/lib.rs"` the scan root is `source/` and the identity base is
    /// the package root, and this must be the same answer with and without the `build`
    /// feature. The pin runs under both faces because nothing here is feature-gated.
    /// 监督者的自定义源码根缺陷，落在写入方所用的解析器上：`[lib] path = "source/lib.rs"` 时扫描根是
    /// `source/`、身份基准是包根，而有 `build` 特性与没有它必须得到同一个答案。因为这里没有任何东西
    /// 门控在特性之后，这条钉子在两个面上都跑。
    #[test]
    fn a_library_target_outside_src_keeps_two_bases_apart() {
        let root = package("outside", "[lib]\npath = \"source/lib.rs\"\n");
        std::fs::create_dir_all(root.join("source")).unwrap();
        std::fs::write(root.join("source/lib.rs"), "pub fn f() {}\n").unwrap();
        let context = AuthoringContext::new(&root, "layout");
        context.scope(|| {
            assert_eq!(scan_root().unwrap(), root.join("source"));
            assert_eq!(identity_base().unwrap(), root);
        });
        let _ = std::fs::remove_dir_all(root);
    }

    /// A layout whose named target is not a file is refused **by name** rather than
    /// falling back to `src/`: the fallback is what wrote a face the build never read.
    /// 具名目标不是文件的布局按名字拒绝，而不是回落 `src/`：那次回落正是写出构建永远不读的那个面的原因。
    #[test]
    fn an_unresolvable_layout_is_refused_instead_of_falling_back_to_src() {
        let root = package("missing", "[lib]\npath = \"source/lib.rs\"\n");
        let context = AuthoringContext::new(&root, "layout");
        context.scope(|| {
            let error = scan_root().expect_err("a named target that is not a file is refused");
            assert!(error.contains("source/lib.rs"), "{error}");
            assert!(identity_base().is_err());
        });
        let _ = std::fs::remove_dir_all(root);
    }

    /// The historical layout keeps the historical answer on both bases.
    /// 历史布局在两个基准上都保留历史答案。
    #[test]
    fn the_historical_layout_answers_src_for_both_bases() {
        let root = package("historical", "");
        let context = AuthoringContext::new(&root, "layout");
        context.scope(|| {
            assert_eq!(scan_root().unwrap(), root.join("src"));
            assert_eq!(identity_base().unwrap(), root.join("src"));
        });
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn nested_authoring_contexts_restore_the_previous_project() {
        let outer = AuthoringContext::new("/tmp/xirang-outer", "outer");
        let inner = AuthoringContext::new("/tmp/xirang-inner", "inner");

        outer.scope(|| {
            assert_eq!(package_root(), Path::new("/tmp/xirang-outer"));
            assert_eq!(authoring_namespace(), "outer");
            inner.scope(|| {
                assert_eq!(package_root(), Path::new("/tmp/xirang-inner"));
                assert_eq!(authoring_namespace(), "inner");
            });
            assert_eq!(package_root(), Path::new("/tmp/xirang-outer"));
            assert_eq!(authoring_namespace(), "outer");
        });
    }

    #[test]
    fn panicking_authoring_context_still_restores_its_parent() {
        let outer = AuthoringContext::new("/tmp/xirang-outer", "outer");
        let inner = AuthoringContext::new("/tmp/xirang-inner", "inner");

        outer.scope(|| {
            let result = std::panic::catch_unwind(|| inner.scope(|| panic!("expected panic")));
            assert!(result.is_err());
            assert_eq!(package_root(), Path::new("/tmp/xirang-outer"));
            assert_eq!(authoring_namespace(), "outer");
        });
    }
}
