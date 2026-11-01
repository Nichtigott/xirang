//! Registration-face declaration and submission macros.
//! 注册面声明与提交宏。
//!
//! The `collector` field is a development control plane, and only one of its
//! three values submits anything: `debug` (and only in a debug build) hands the
//! declaration to `xirang-toolchain`'s inventory section; `development` and
//! `linked` expand to nothing, which is what every shipped application uses. The
//! declaration is compiled into the crate under all three; the generated
//! `StaticPlan` is what a release reads.
//! `collector` 字段是开发控制面，三个取值里只有一个会提交任何东西：`debug`（且只在 debug
//! 构建里）把声明交给 `xirang-toolchain` 的 inventory 段；`development` 与 `linked`
//! 展开为空，而每个出厂应用用的都是这两个。三个取值下声明都被编译进本 crate；正式应用读的是生成的
//! `StaticPlan`。
//!
//! A value outside those three is refused by name rather than falling through to
//! a bare macro error inside a macro the author never wrote:
//! 三个取值之外的写法会按名被拒，而不是掉进一个作者从未写过的宏内部的裸宏错误：
//!
//! ```rust,compile_fail
//! xirang_toolchain::run_method::external_object! {
//!     collector: nonsense,
//!     kind: ProbeCollector,
//! }
//! ```
//!
//! This fence is compiled by CI's `cargo test --workspace --all-features --doc`
//! job, which is the only command that runs doctests.
//! 这道围栏由 CI 的 `cargo test --workspace --all-features --doc` 任务编译，那是唯一会跑
//! doctest 的命令。
//!
//! An external face that names no collector still declares itself, because
//! `external_object!` injects `linked` — the one spelling whose whole meaning is
//! "this declaration comes from a linked crate, so nothing is submitted here":
//! 没写 collector 的外部面仍然完成声明，因为 `external_object!` 会注入 `linked`——这个拼法的
//! 全部含义就是"这条声明来自被链接的 crate，因此这里什么都不提交"：
//!
//! ```rust
//! pub struct LinkedProbe;
//!
//! // A crate that declares faces without `host!()` owns the identity namespace itself: the
//! // declaration macros read this constant, not `env!("CARGO_PKG_NAME")` at each site, because a
//! // partitioned crate mounts the same file under another `#[path]` and would read *its own*
//! // package name there (audit `M7`, P3.3).
//! // 声明注册面却不调用 `host!()` 的 crate 自己拥有身份命名空间：声明宏读这个常量，而不是在每个声明处
//! // 读 `env!("CARGO_PKG_NAME")`——因为分区后的 crate 会用另一个 `#[path]` 挂载同一个文件，在那里会读到
//! // **它自己的**包名（审计 `M7`，P3.3）。
//! pub const XIRANG_NAMESPACE: &str = env!("CARGO_PKG_NAME");
//!
//! xirang_toolchain::run_method::external_object! {
//!     kind: LinkedProbe,
//! }
//!
//! // An explicit `main` keeps this fence from being wrapped in one: the constant above has to sit
//! // at the crate root, because that is where the macros look for it.
//! // 显式写一个 `main`，这道围栏就不会被包进一个函数里：上面的常量必须位于 crate 根，因为宏就是在
//! // 那里找它。
//! fn main() {
//!     // The declaration is compiled in and reachable through its consts; nothing was
//!     // submitted, which is what `linked` and `development` share one arm for.
//!     // 声明被编译进来、经它的常量可达；没有任何提交发生，这正是 `linked` 与 `development`
//!     // 共用一条臂的原因。
//!     assert_eq!(REGISTRATION.kind, "LinkedProbe");
//! }
//! ```
//!
//! This fence is the pin for audit `LGC-LG-14`: refusing `linked` (or moving that
//! arm to `$crate::submit!`) turns it — and every
//! `external_object!` in the tree, 36 references including
//! `run_method/tests/external_*.rs` — into a compile failure, because `linked` is
//! the default this macro injects rather than a value nobody writes.
//! 这道围栏是审计 `LGC-LG-14` 的钉子：拒绝 `linked`（或把那条臂改成
//! `$crate::submit!`）会让它——以及树里每一处 `external_object!`，共 36 处引用，
//! 含 `run_method/tests/external_*.rs`——变成编译失败，因为 `linked` 是这个宏注入的**默认值**，
//! 而不是没人手写的取值。

/// Expand one fully normalized registration face into consts and submission.
/// 把一个完整规范化的注册面展开为常量与提交调用。
///
/// Internal: `object!` and `external_object!` normalize their shorthand into
/// this arm, so host code should call those instead. Every field is assumed
/// present.
///
/// The collector ident decides whether anything is *submitted*, not which linker
/// section receives it: only `debug` submits, and only in a debug build;
/// `development` and `linked` expand to nothing (see `__submit_registration`).
/// A declaration is compiled into the crate either way — the generated
/// `StaticPlan` is what a release application reads — so `collector` is a
/// development control plane, and a misspelled value is refused by name.
/// 内部宏：`object!` 与 `external_object!` 把简写规范化为这一分支，宿主应改用那些宏。
/// 它假定每个字段都已给出。
///
/// 收集器标识决定的是**是否提交**任何东西，而不是注册信息进入哪个链接器段：只有 `debug` 会提交，
/// 且只在 debug 构建里；`development` 与 `linked` 展开为空（见 `__submit_registration`）。
/// 无论哪个取值，声明都被编译进本 crate——正式应用读的是生成的 `StaticPlan`——因此 `collector`
/// 属于开发控制面，而拼错的取值会按名被拒。
#[macro_export]
macro_rules! __registration_face {
    {
        source: $source:expr,
        collector: $collector:ident,
        kind: $kind:ident,
        preset: $preset:ty,
        preset_name: $preset_name:expr,
        parts: $parts:ty,
        parts_name: $parts_name:expr,
        name: { zh: $name_zh:expr, en: $name_en:expr $(,)? },
        summary: { zh: $summary_zh:expr, en: $summary_en:expr $(,)? },
        params: $params:expr,
        exports: [$($export:expr),* $(,)?],
        handle: $handle:ident,
        handle_name: $handle_name:expr,
        $(stable_name: $stable_name:literal,)?
        needs_registry: $needs_registry:expr,
        registry_name: $registry_name:expr,
        parent: $parent:expr,
        getting_from_other_registry: $getting:expr,
        registry_rule_path: $rule_path:expr,
        registry_rule: $rule:expr,
        $(admission: $admission:expr,)?
        $(handle_traits: [$($handle_trait:literal),* $(,)?],)?
        $(handle_contracts: [$($handle_contract:path),* $(,)?],)?
        $(part_traits: [$($part_trait:literal),* $(,)?],)?
        $(part_contracts: [$($part_contract:path),* $(,)?],)?
        requires: [$($require:expr => $provider:expr),* $(,)?],
        provides: [$($provide:expr),* $(,)?],
        $(flow: $flow:expr,)?
        $(flow_provider: $flow_provider:path,)?
        $(plugin: $plugin:expr,)?
        runtime_checks: [$($runtime_check:expr),* $(,)?] $(,)?
    } => {
        const _: () = $crate::run_method::assert_contract::<$preset, $parts>();
        $crate::__assert_impls!($handle; [$($($handle_contract),*)?]);
        $crate::__assert_impls!($parts; [$($($part_contract),*)?]);

        // These two constants are part of the author's public API — a typed graft
        // cut names `crate::run_method::…::NODE_ID` — and an out-of-project face mounts its own
        // file, so the declaring crate's `#![warn(missing_docs)]` sees them. They
        // are documented here rather than hidden because an author is supposed to
        // use them; `#[doc(hidden)]` would only silence the warning by lying about
        // the surface.
        // 这两个常量属于作者的公开 API——类型化 graft 切口就是命名
        // `crate::run_method::…::NODE_ID`——而项目外的面由作者自己挂载文件，因此声明方 crate 的
        // `#![warn(missing_docs)]` 会看到它们。在这里补文档而不是隐藏，是因为作者本就
        // 应当使用它们；`#[doc(hidden)]` 只会靠谎报表面来消除警告。

        /// The face's compile-time identity, for typed graft cuts and parent links.
        /// 该注册面的编译期身份，供类型化 graft 切口与父级链接使用。
        ///
        /// It hashes `crate::XIRANG_NAMESPACE` (the crate root constant, audit `M7`, P3.3),
        /// the declaration's relative source path
        /// and its kind — never the registry name, so renaming the registry name
        /// does not move an identity.
        /// 它哈希 `crate::XIRANG_NAMESPACE`（crate 根常量，审计 `M7`，P3.3）、声明的相对源码路径
        /// 与 kind——绝不含注册面名，因此改注册面名
        /// 不会移动身份。
        pub const NODE_ID: $crate::run_method::NodeId = $crate::run_method::NodeId::from_namespaced_path(
            crate::XIRANG_NAMESPACE,
            $source,
            stringify!($kind),
        );

        /// The preset type this face was declared with, for a graft cut's
        /// compile-time output check. Hidden because the author names the preset
        /// once, in the declaration, and never needs this alias by name.
        /// 本注册面声明时使用的 preset 类型，供 graft 切口的编译期输出检查使用。隐藏，
        /// 因为作者只在声明里写一次 preset，从不需要按名字引用这个别名。
        #[doc(hidden)]
        pub type __Preset = $preset;

        /// The parts type this face was declared with, for a graft cut's
        /// compile-time output check.
        /// 本注册面声明时使用的 parts 类型，供 graft 切口的编译期输出检查使用。
        #[doc(hidden)]
        pub type __Parts = $parts;

        /// The same declaration in the owned form registration consumes.
        /// 同一份声明的拥有型快照，注册过程消费它。
        pub const REGISTRATION: $crate::run_method::RegistrationInfo = $crate::run_method::RegistrationInfo {
            namespace: crate::XIRANG_NAMESPACE,
            id: NODE_ID,
            parent: $parent,
            kind: stringify!($kind),
            preset: $preset_name,
            parts: $parts_name,
            params: $params,
            handle: $handle_name,
            stable_name: $crate::__stable_name!($($stable_name)?),
            name: $crate::run_method::LocalizedText { zh: $name_zh, en: $name_en },
            summary: $crate::run_method::LocalizedText { zh: $summary_zh, en: $summary_en },
            exports: &[$($export),*],
            needs_registry: $needs_registry,
            registry_name: $registry_name,
            getting_from_other_registry: $getting,
            registry_rule_path: $rule_path,
            registry_rule: $rule,
            admission: $crate::__admission!($($admission)?),
            requires: &[$($crate::run_method::RequirementSpec {
                capability: $require,
                provider: $provider,
            }),*],
            provides: &[$($provide),*],
            // The construction contract keeps what the types say: the parts the
            // preset requires and the parts the parts type supplies, both read
            // straight off the traits. The output names the author used to write
            // beside them were compared only with each other, so they are gone;
            // `assert_contract` proves that fact with types, per face, and a graft
            // cut proves it across two faces.
            // 构造合同只保留类型说了算的东西：preset 要求的 parts 与 parts 类型提供的
            // parts，两者都直接从 trait 读出。作者过去写在旁边的那对输出名只互相比较过，
            // 因此删除；同一件事由 `assert_contract` 按面用类型证明，嫁接切口则跨两个面证明。
            contract: $crate::run_method::ObjectContract {
                required_parts: <$preset as $crate::run_method::PresetContract>::REQUIRED_PARTS,
                provided_parts: <$parts as $crate::run_method::PartsContract>::PROVIDED_PARTS,
            },
            flow: $crate::__flow_select!($($flow)?; $($flow_provider)?),
            flow_provider: $crate::__flow_provider!($($flow_provider)?),
            // The label follows the path: a face that states a compiler-checked
            // contract does not state its name a second time, and a face that
            // states only a label keeps it as the unchecked claim it is.
            // 标签跟随路径：写了参与编译检查的契约的注册面不再写第二遍名字；只写了标签的
            // 注册面则保留它——那本来就是一条未经检查的声明。
            handle_traits: $crate::run_method::__face_trait_labels_or!(
                [$($($handle_contract),*)?]; [$($($handle_trait),*)?]
            ),
            part_traits: $crate::run_method::__face_trait_labels_or!(
                [$($($part_contract),*)?]; [$($($part_trait),*)?]
            ),
            runtime_checks: &[$($runtime_check),*],
            plugin: $crate::__plugin!($($plugin)?),
            source: $crate::run_method::SourceLocation {
                file: $source,
                line: line!(),
                column: column!(),
                function: $handle_name,
            },
        };

        // The collector is a development control plane. Release applications
        // use the generated StaticPlan and do not retain inventory sections.
        // 收集器属于开发控制面；正式应用使用生成的 StaticPlan，不保留 inventory 段。
        $crate::__submit_registration!($collector; REGISTRATION);
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __submit_registration {
    // `development` and `linked` are one effect written as two arms, because
    // `macro_rules` has no token alternation: `(development | linked; $x:ident)`
    // matches the literal tokens `|` and never fires, which is how the first draft
    // of this change sent the implicit `linked` into the refusal arm below (the
    // doctest at the top of this file caught it). They say different things about
    // the face — `development` means the author is developing it in this crate,
    // `linked` means the declaration comes from a crate that is linked into this
    // one — and the same thing about submission: nothing here, because neither has
    // an inventory section to go into. The audit entry this answers (`LGC-LG-14`)
    // names the shape of two empty arms for one effect, and the honest encoding of
    // "one effect" in this macro system is one comment for the pair plus a pin that
    // refuses to let the two spellings drift apart.
    // `linked` in particular is not a value nobody uses: it is the default
    // `external_object!` injects when the author writes no collector
    // (`macros/face_external.rs`).
    // `development` 与 `linked` 是同一个效果写成的两条臂，因为 `macro_rules` 没有 token 级
    // 交替：`(development | linked; $x:ident)` 匹配的是字面 token `|`、永不命中——本次改动的第一
    // 稿正是因此把隐式 `linked` 送进了下面的拒绝臂（本文件顶部的 doctest 抓住了它）。它们对**面的
    // 来源**说法不同——`development` 指作者正在本 crate 里开发它，`linked` 指声明来自被链接进本
    // crate 的那个 crate——而对**提交**说的一样：这里什么都不提交，因为两者都没有可用的 inventory 段。
    // 审计条目 `LGC-LG-14` 点名的正是"一个效果配两条空臂"这个形状，而在这套宏系统里"一个效果"的
    // 诚实写法是：为这一对写一段注释，再加一条不让两个拼法漂移的钉子。`linked` 尤其不是没人用的取值：
    // 它就是 `external_object!` 在作者不写 collector 时注入的默认值（见 `macros/face_external.rs`）。
    (development; $registration:ident) => {};
    (linked; $registration:ident) => {};
    // `debug`: the only value that submits anything, and only in a debug build.
    // The switch is deliberately opt-in and undocumented elsewhere: a release
    // build of the same source compiles the arm away.
    // `debug`：唯一真正提交的取值，且只在 debug 构建里。这个开关有意是显式选择且别无文档：
    // 同一份源码的 release 构建会把这个 arm 编译掉。
    (debug; $registration:ident) => {
        #[cfg(debug_assertions)]
        $crate::submit! { $registration }
    };
    // Any other ident is a misspelling, and naming the three valid values here
    // beats the bare "no rules expected" error a missing arm produces inside a
    // macro the author never wrote.
    // 任何其它 ident 都是拼错；在这里点出三个合法取值，好过一个作者从未写过的宏内部缺 arm 带来的
    // 裸 "no rules expected" 错误。
    ($other:ident; $registration:ident) => {
        ::core::compile_error!(::core::concat!(
            "`collector` must be one of `development`, `debug`, `linked`; got `",
            ::core::stringify!($other),
            "` (only `debug` submits a registration, and only in a debug build)"
        ));
    };
}
