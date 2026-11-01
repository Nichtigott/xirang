//! Host entry, host onboarding, and graft-plan macros.
//! 宿主入口、宿主接入与 graft 计划宏。

/// Declare the single host entry used by source-scope discovery.
/// 声明源码作用域发现使用的唯一宿主入口。
#[macro_export]
macro_rules! application {
    (entry = $entry:path $(,)?) => {
        #[doc(hidden)]
        pub const XIRANG_APPLICATION_ENTRY: &str = stringify!($entry);
    };
}

/// Declare this crate as a XiRang host and pull in the registration plan
/// captured at build time.
/// 声明当前 crate 为 XiRang 宿主，并引入构建时捕获的注册计划。
///
/// `xirang-toolchain` renders the discovered registration tree to
/// `OUT_DIR/generated_lib.rs`; this macro includes it at the crate root so
/// `builtin_static_plan()` and the per-level `{name}_object!` aliases are
/// available crate-wide. It expands to
/// `include!(concat!(env!("OUT_DIR"), "/generated_lib.rs"))` — writing that
/// line directly is an equivalent, advanced alternative.
/// `xirang-toolchain` 把发现的注册树渲染到 `OUT_DIR/generated_lib.rs`；
/// 此宏将其包含到 crate 根，使 `builtin_static_plan()` 与各层级的
/// `{name}_object!` 别名在整个 crate 内可用。它展开为
/// `include!(concat!(env!("OUT_DIR"), "/generated_lib.rs"))`，
/// 直接书写该行是等价的高级写法。
///
/// `include!` only accepts a literal path, so the generated entry's file name
/// cannot be read from the kernel lexicon here. Pinning the two texts together
/// makes a drift a compile error instead of a host that includes a file the
/// build step no longer writes.
/// `include!` 只接受字面量路径，因此这里无法从内核 lexicon 读取生成入口的文件名。
/// 把两份文本钉在一起，漂移会变成编译错误，而不是去 include 一个构建步骤已不再写的
/// 文件。
const _: () = assert!(::xirang_kernel::lexicon::same_text(
    ::xirang_kernel::lexicon::GENERATED_LIB_FILE,
    "generated_lib.rs"
));

/// Pull the build-time registration plan into the crate root.
/// 把构建期捕获的注册计划引入 crate 根。
///
/// Call it once per host crate whose build script has run the XiRang build
/// step; the included file supplies `builtin_static_plan()` and the per-level
/// object aliases. The `include!` target lives under `OUT_DIR`, so a crate that
/// never runs that step fails to compile rather than silently missing faces.
/// 在已由 build script 运行 XiRang 构建步骤的宿主 crate 中调用一次；被包含的文件
/// 提供 `builtin_static_plan()` 与各层级 object 别名。`include!` 目标位于 `OUT_DIR`，
/// 未运行该步骤的 crate 会编译失败，而不是静默缺少注册面。
#[macro_export]
macro_rules! host {
    () => {
        /// The identity namespace every face in this crate is compiled under.
        /// 本 crate 里每个注册面编译时所用的身份命名空间。
        ///
        /// It is a **crate-root constant**, not an `env!` read at each declaration site, because a
        /// partitioned crate mounts the same face file through a different `#[path]` spelling: the
        /// package name it would read there is the *ghost crate's*, and identity is
        /// `hash(namespace, source path, name)` — so reading it locally would silently rename every
        /// face (audit `M7`, P3.3). One constant at the root is the one place a partition can point
        /// at the host's value.
        /// 它是**crate 根的常量**，而不是在每个声明处读一次 `env!`：分区后的 crate 会用不同的 `#[path]`
        /// 拼写挂载同一个面文件，在那里读到的包名是**幽灵 crate 的**，而身份是
        /// `hash(命名空间, 源码路径, 名字)`——就地读会静默重命名每一个面（审计 `M7`，P3.3）。根上这样一个
        /// 常量，正是分区时能指向宿主那个值的唯一位置。
        pub const XIRANG_NAMESPACE: &str = env!("CARGO_PKG_NAME");
        include!(concat!(env!("OUT_DIR"), "/generated_lib.rs"));
    };
}

/// Reach this crate's own registry at run time.
/// 在运行期拿到本 crate 自己的注册机。
///
/// ```ignore
/// xirang_toolchain::run_method::host!();
///
/// pub const FRAMEWORK: FrameworkId = FrameworkId::new("my.app");
///
/// pub fn registry() -> Registry {
///     xirang_toolchain::host_registry!().expect("this crate's faces register")
/// }
/// ```
///
/// It reads three things from the **crate root** the generated entry already populates — `FRAMEWORK`
/// (which the host declares), `XIRANG_NAMESPACE` (emitted by `host!()`) and `registrations()`
/// (emitted into `generated_lib.rs`) — and hands them to
/// [`run_method::host_registry`](crate::run_method::host_registry::host_registry), which keeps the
/// pairing rule and the atomicity in one place. A macro rather than a function for exactly that
/// reason: those three symbols live in the *host's* crate, not in this one.
/// 它从生成入口已经填好的 **crate 根**读三样东西——`FRAMEWORK`（宿主声明）、`XIRANG_NAMESPACE`
/// （`host!()` 发出）与 `registrations()`（发进 `generated_lib.rs`）——并交给
/// [`run_method::host_registry`](crate::run_method::host_registry::host_registry)，由它把配对规则与
/// 原子性收在一处。做成宏而不是函数正是因为这个：那三个符号住在**宿主**的 crate 里，不在这个 crate 里。
///
/// Returns `RegistryResult<Registry>`: a host decides whether a failed self-registration is fatal.
/// 返回 `RegistryResult<Registry>`：自我注册失败是否致命，由宿主决定。
///
/// **The half of audit `M7` §M7.62 this closes.** The bridge's `explain` half (it now names the
/// implementation that replaced a slot) landed first; this is the other half — before it, every host
/// wanting a registry re-derived the assembly from an example.
/// **它闭合的是审计 `M7` §M7.62 的另一半。** 桥的 `explain` 那一半（现在会点名替换某个槽位的实现）
/// 先落地；这是另一半——在它之前，每个想要注册机的宿主都得从某个示例反推这段装配。
#[macro_export]
macro_rules! host_registry {
    () => {
        $crate::run_method::host_registry::host_registry(
            FRAMEWORK,
            XIRANG_NAMESPACE,
            &registrations(),
        )
    };
}

/// Declare graft selectors for build-time capture without constructing a
/// runtime `GraftPlan`.
/// 声明供构建阶段捕获的 graft selector，不构造运行时 `GraftPlan`。
///
/// Put this at the host crate entry. `xirang-toolchain` validates the grammar and
/// stores the cuts in the generated `StaticPlan`. Use the dynamic
/// [`runtime_graft_plan!`](crate::runtime_graft_plan) expression only when code needs to build
/// or edit a plan at runtime.
/// 将它放在宿主 crate 入口。`xirang-toolchain` 校验语法并把切口写入生成的
/// `StaticPlan`；只有运行时代码确实要构造或编辑计划时才使用动态
/// [`runtime_graft_plan!`](crate::runtime_graft_plan) 表达式（旧名 `graft_plan!` 仍接受）。
#[macro_export]
macro_rules! static_graft_plan {
    ($framework:expr, $($cuts:tt)+) => {
        const _: $crate::run_method::FrameworkId = $framework;
        const _: &str = stringify!($($cuts)+);
    };
}

/// Build a persistent external graft overlay without touching source files.
/// 构造持久化外部 graft 覆盖计划，不移动或修改任何源码文件。
///
/// ```
/// # use xirang_toolchain::run_method::FrameworkId;
/// # use xirang_toolchain::runtime_graft_plan;
/// # use xirang_toolchain::graft_plan;
/// # let framework = FrameworkId::new("example");
/// let plan = runtime_graft_plan!(framework,
///     cut ["root/a1/b2"] graft "canvas_fast",
///     cut ["root/a"] full graft "a_fast",
/// );
/// assert_eq!(plan.cuts.len(), 2);
/// // The old name still works: it forwards here.
/// // 旧名仍然可用：它转发到这里。
/// # let same = graft_plan!(framework, cut ["root/a1/b2"] graft "canvas_fast");
/// # assert_eq!(same.cuts.len(), 1);
/// ```
///
/// Formerly named `graft_plan!`. That name still compiles (it forwards here), but it sat one word
/// away from `static_graft_plan!` and said nothing about which of the two entries it was — the
/// build-time one that captures the source text, or this one that builds the value at run time.
/// 原名 `graft_plan!`。那个名字仍然能编译（它转发到这里），但它与 `static_graft_plan!` 只差一个词，
/// 也没有说明自己是两个入口中的哪一个——构建期捕获源码文本的那个，还是运行期构造值的这个。
#[macro_export]
macro_rules! runtime_graft_plan {
    ($framework:expr, $($cuts:tt)+) => {{
        let mut plan = $crate::run_method::GraftPlan::new($framework);
        $crate::__graft_plan_cuts!(plan; $($cuts)+);
        plan
    }};
}

/// The **old name** of [`runtime_graft_plan!`], kept so existing hosts compile unchanged.
/// [`runtime_graft_plan!`] 的**旧名**，保留它以便既有宿主编译不改。
#[macro_export]
macro_rules! graft_plan {
    ($($tokens:tt)*) => { $crate::runtime_graft_plan! { $($tokens)* } };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __graft_plan_cuts {
    ($plan:ident;) => {};
    ($plan:ident; cut [$start:literal to $end:literal] full graft $graft:literal $(, $($rest:tt)*)?) => {{
        let mut cut = $crate::run_method::GraftCut::range($start, $end, $graft);
        cut.subtree = true;
        $plan.cuts.push(cut);
        $crate::__graft_plan_cuts!($plan; $($($rest)*)?);
    }};
    ($plan:ident; cut [$start:literal to $end:literal] graft $graft:literal $(, $($rest:tt)*)?) => {{
        $plan.cuts
            .push($crate::run_method::GraftCut::range($start, $end, $graft));
        $crate::__graft_plan_cuts!($plan; $($($rest)*)?);
    }};
    ($plan:ident; cut [$path:literal] full graft $graft:literal $(, $($rest:tt)*)?) => {{
        $plan.cuts.push($crate::run_method::GraftCut::subtree($path, $graft));
        $crate::__graft_plan_cuts!($plan; $($($rest)*)?);
    }};
    ($plan:ident; cut [$path:literal] graft $graft:literal $(, $($rest:tt)*)?) => {{
        $plan.cuts.push($crate::run_method::GraftCut::new($path, $graft));
        $crate::__graft_plan_cuts!($plan; $($($rest)*)?);
    }};
    ($plan:ident; cut $path:literal full graft $graft:literal $(, $($rest:tt)*)?) => {{
        $plan.cuts.push($crate::run_method::GraftCut::subtree($path, $graft));
        $crate::__graft_plan_cuts!($plan; $($($rest)*)?);
    }};
    ($plan:ident; cut $path:literal graft $graft:literal $(, $($rest:tt)*)?) => {{
        $plan.cuts.push($crate::run_method::GraftCut::new($path, $graft));
        $crate::__graft_plan_cuts!($plan; $($($rest)*)?);
    }};
}
