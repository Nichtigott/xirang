//! Object declaration macros: generated aliases and their hidden implementation.
//! 对象声明宏：生成的别名及其隐藏实现。

/// Internal declaration primitive used by generated parent-specific macros.
/// 供生成的父级专属宏使用的内部声明原语。
#[doc(hidden)]
#[macro_export]
macro_rules! __xirang_object {
    (collector: $collector:ident, $($tokens:tt)*) => {
        $crate::__control_object! { collector: $collector, $($tokens)* }
    };
    ($($tokens:tt)*) => {
        $crate::__control_object! { collector: development, $($tokens)* }
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __control_object {
    // One arm, one story: after `collector` and `kind` every field may be
    // omitted, and the fields an omission would otherwise leave dangling are
    // derived rather than defaulted to something the author has to know about.
    // `handle`, `params`, `registry_name` and `registry_rule_path` all follow
    // from `kind` or from the face's own file, so a face that says only what it
    // is (`kind`) and what implements it (the same type) declares the whole
    // protocol. The strict arm and the handle-carrying arm that used to sit
    // above were removed here: they could express nothing this arm cannot, and
    // two arms meant two answers to "what must I write".
    // 一条 arm、一个故事：`collector` 与 `kind` 之后每个字段都可省，而省略后会悬空的字段
    // 一律**推导**，而不是留一个作者必须知道的默认值。`handle`、`params`、
    // `registry_name`、`registry_rule_path` 都来自 `kind` 或注册面自己的文件，因此一条只
    // 说明自己是什么（`kind`）、由谁实现（同一个类型）的注册面就声明了整套协议。此前位于
    // 上方的严格 arm 与带 handle 的 arm 在此删除：它们表达不出本 arm 表达不了的东西，而两条
    // arm 就是两个"我必须写什么"的答案。
    {
        collector: $collector:ident,
        kind: $kind:ident,
        $(preset: $preset:ty,)?
        $(parts: $parts:ty,)?
        $(name: { zh: $name_zh:expr, en: $name_en:expr },)?
        $(summary: { zh: $summary_zh:expr, en: $summary_en:expr },)?
        $(exports: [$($export:expr),* $(,)?],)?
        $(stable_name: $stable_name:literal,)?
        $(needs_registry: $needs_registry:expr,)?
        $(parent: $parent:expr,)?
        $(getting_from_other_registry: $getting:expr,)?
        $(registry_rule_path: $rule_path:expr,)?
        $(registry_rule: $rule:expr,)?
        $(admission: $admission:expr,)?
        $(handle_traits: [$($handle_trait:literal),* $(,)?],)?
        $(handle_contracts: [$($handle_contract:path),* $(,)?],)?
        $(part_traits: [$($part_trait:literal),* $(,)?],)?
        $(part_contracts: [$($part_contract:path),* $(,)?],)?
        $(requires: [$($require:expr => $provider:expr),* $(,)?],)?
        $(provides: [$($provide:expr),* $(,)?],)?
        $(flow: $flow:expr,)?
        $(flow_provider: $flow_provider:path,)?
        $(plugin: $plugin:expr,)?
        $(runtime_checks: [$($runtime_check:expr),* $(,)?],)?
        $(,)?
    } => {
        // This is the arm for a face with no `handle:` field, so it has to forward
        // whatever `preset:`/`parts:` the author wrote instead of substituting
        // defaults: the matcher already binds them, and a hardcoded
        // `NoPreset`/`NoParts` here would silently rewrite the author's types and
        // their names in `REGISTRATION` while the compiler saw nothing wrong.
        // Presence is decided per binding — the matcher makes `preset` and `parts`
        // independently optional, and both resolve an omitted binding through the
        // same `__face_ty_or!`/`__face_ty_name_or!` pair, so an omitted one keeps
        // `NoPreset`/`NoParts` and a written one is forwarded verbatim.
        // Pinned by `run_method/tests/face_preset_parts.rs` and
        // `run_method/tests/face_arm_defaults.rs`.
        // 这一支面向没有 `handle:` 字段的注册面，因此必须原样转发作者写下的
        // `preset:`/`parts:`，而不是替换成默认值：匹配器已经绑定了它们，在这里写死
        // `NoPreset`/`NoParts` 会静默改写作者的类型及其在 `REGISTRATION` 里的名字，而编译器看不出
        // 任何问题。有无按每个绑定单独判断——匹配器让 `preset` 与 `parts` 各自可省，两者经同一对
        // `__face_ty_or!`/`__face_ty_name_or!` 解析省略绑定，因此省略者保持
        // `NoPreset`/`NoParts`，写下的原样转发。
        // 由 `run_method/tests/face_preset_parts.rs` 与
        // `run_method/tests/face_arm_defaults.rs` 钉住。
        $crate::__registration_face! {
            source: $crate::run_method::registry_core::manifest_relative_source(env!("CARGO_MANIFEST_DIR"), file!()),
            collector: $collector,
            kind: $kind,
            preset: $crate::__face_ty_or!($crate::run_method::NoPreset; $($preset)?),
            preset_name: $crate::__face_ty_name_or!("NoPreset"; $($preset)?),
            parts: $crate::__face_ty_or!($crate::run_method::NoParts; $($parts)?),
            parts_name: $crate::__face_ty_name_or!("NoParts"; $($parts)?),
            name: {
                zh: $crate::__face_value_or!(stringify!($kind); [$($name_zh)?]),
                en: $crate::__face_value_or!(stringify!($kind); [$($name_en)?]),
            },
            summary: {
                zh: $crate::__face_value_or!(""; [$($summary_zh)?]),
                en: $crate::__face_value_or!(""; [$($summary_en)?]),
            },
            params: stringify!($kind),
            exports: [$($($export),*)?],
            handle: $kind,
            handle_name: stringify!($kind),
            $(stable_name: $stable_name,)?
            needs_registry: $crate::__face_expr_or!(false; $($needs_registry)?),
            registry_name: $crate::run_method::registry_core::last_path_segment(module_path!()),
            parent: $crate::__face_expr_or!($crate::run_method::root_node_id(crate::XIRANG_NAMESPACE); $($parent)?),
            getting_from_other_registry: $crate::__face_expr_or!(None; $($getting)?),
            registry_rule_path: $crate::__face_expr_or!($crate::run_method::registry_core::manifest_relative_source(env!("CARGO_MANIFEST_DIR"), file!()); $($rule_path)?),
            // The author's expression wins; an omitted rule on a
            // registry-owning face resolves to the canonical sibling rule
            // module, and every other face keeps `ANY`. The resolver is a proc
            // macro because the canonical spelling is a *relative* path — and it
            // is the resolver, not this matcher, that answers the IDE too: the
            // IDE's view of a nested face is a crate-root shadow where that
            // relative path does not resolve.
            // 作者写下的表达式优先；拥有注册机的面省略规则时解析到同目录的规范规则模块，
            // 其余面保留 `ANY`。解析器是过程宏，因为规范写法是**相对**路径——而回答 IDE 的
            // 也是解析器而不是本匹配器：嵌套面在 IDE 眼里的视图是 crate 根影子，那条相对路径
            // 在那里解析不了。
            registry_rule: $crate::run_method::__face_rule_or!(
                $crate::run_method::RegistrationRule::ANY;
                $($needs_registry)?;
                $($rule)?
            ),
            $(admission: $admission,)?
            $(handle_traits: [$($handle_trait),*],)?
            $(handle_contracts: [$($handle_contract),*],)?
            $(part_traits: [$($part_trait),*],)?
            $(part_contracts: [$($part_contract),*],)?
            requires: [$($($require => $provider),*)?],
            provides: [$($($provide),*)?],
            $(flow: $flow,)?
            $(flow_provider: $flow_provider,)?
            $(plugin: $plugin,)?
            runtime_checks: [$($($runtime_check),*)?],
        }
    };

    // A kind-only face needs no arm of its own: the arm above makes every field
    // after `kind` optional, so `{ collector, kind }` already matches there, and
    // the front end has nothing left to reorder.
    // 只写 `kind` 的注册面不需要单独的 arm：上面的 arm 让 `kind` 之后的每个字段都可省，
    // 因此 `{ collector, kind }` 已经在那里匹配，前端也没有需要重排的东西。

    // Anything the arm above declines — fields in another order, `;` separators,
    // a forgotten separator, a misspelled field — goes to the front end, which
    // reorders tolerantly and reports the exact token it rejects. A well-formed
    // face never reaches this arm, so its expansion is unchanged. The front end is
    // named through `$crate` like every other expansion point: an absolute
    // `::xirang_run_method` resolves only while the host keeps that dependency
    // name, so a renamed dependency (`run = { package = "xirang-toolchain" }`)
    // compiled in field order and failed here with "use of undeclared crate".
    // 上面那条 arm 不接受的声明——字段顺序不同、用 `;` 分隔、漏写分隔符、字段名拼错——
    // 交给前端：它宽容重排，并把被拒绝的那个 token 精确报出来。合法注册面永远不会走到
    // 这里，展开因此保持不变。前端与其它展开点一样经 `$crate` 命名：绝对路径
    // `::xirang_run_method` 只在宿主保留了这个依赖名时能解析，因此改了依赖名
    // （`run = { package = "xirang-toolchain" }`）的声明会按字段顺序编译通过、却在这里报
    // "use of undeclared crate"。
    {
        collector: $collector:ident,
        $($tokens:tt)*
    } => {
        $crate::run_method::face_fields! { @control collector: $collector, $($tokens)* }
    };

}
