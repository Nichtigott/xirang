//! Naming the crate that compiles each module, for the **facade** (audit `M7`, §M7.33).
//! 为 **facade** 点出编译每个模块的那个 crate（审计 `M7`，§M7.33）。
//!
//! A facade compiles no faces of its own: it carries the cross-crate half — the graft cut table and the
//! contract assertions — and `crate::` inside it means the facade itself. So every `crate::<module>` it
//! emits is rewritten to `<owner>::<module>`, where the owner comes from the plan: a claim, its root
//! face and its ancestor shells all belong to that ghost, and every other module belongs to the host.
//! facade 自己不编译任何面：它携带跨 crate 那一半——graft 切口表与契约断言——而它里面的 `crate::` 指的是
//! facade 自己。因此它发射的每个 `crate::<模块>` 都改写成 `<owner>::<模块>`，属主来自规划：一条认领、它的根面
//! 与它的祖先壳都归那个幽灵，其余每个模块归宿主。
//!
//! It lives beside the pass rather than inside it because the rewrite is one rule with its own tests, and
//! the pass is already at its size budget.
//! 它住在 pass 旁边而不是里面，因为这条改写是一条有自己钉子的规则，而 pass 已经用满了尺寸预算。

use super::tree::ShapeRender;
use crate::build_method::registry_identity;
use crate::build_method::registry_syntax::GraftSyntax;
use crate::build_method::static_plan::StaticFaceRecord;

/// The crate that compiles one module, as an expression the facade can name.
/// 编译某个模块的那个 crate，写成 facade 能点名的表达式。
///
/// In facade mode every `crate::<module>` becomes `<owner>::<module>`, because `crate::` there means
/// the facade itself; a bare module path (the registration-rules block writes them that way) gets the
/// same owner prefix. The lookup takes the **longest** owner whose module is a prefix at a `::`
/// boundary, so a claim's root face and its descendants are all attributed correctly.
/// facade 模式里每个 `crate::<模块>` 都变成 `<owner>::<模块>`，因为那里的 `crate::` 指的是 facade 自己；
/// 裸模块路径（注册规则块就是这么写的）加同一个属主前缀。查表取**最长**的、模块在 `::` 边界上构成前缀的属主，
/// 因此一条认领的根面与它的后代都能归属正确。
pub(super) fn owned(shape: &ShapeRender<'_>, expression: &str) -> String {
    let trimmed = expression.trim();
    if !shape.facade {
        return trimmed.to_owned();
    }
    let local = trimmed
        .strip_prefix("crate::")
        .or_else(|| trimmed.strip_prefix("self::"));
    let had_prefix = local.is_some();
    let rest = local.unwrap_or(trimmed);
    // Match the **whole** expression against the owner table, taking the longest module that is a
    // prefix at a `::` boundary. Extracting "the module" first only works for `<module>::<ITEM>`; a
    // registration rule spells `<module>::REGISTRATION.registry_rule`, and splitting at the last `::`
    // produced `panel::frame::REGISTRATION`, an owner nobody has — which is why the facade emitted that
    // line bare and failed to resolve it (measured, §M7.38).
    // 用**整条**表达式去匹配属主表，取最长的、在 `::` 边界上构成前缀的模块。先抠出"那个模块"只对
    // `<模块>::<ITEM>` 成立；而注册规则写的是 `<模块>::REGISTRATION.registry_rule`，按最后一个 `::`
    // 切会得到 `panel::frame::REGISTRATION`——一个谁都不是的属主，这正是 facade 把那一行原样发出去、
    // 解析不到的原因（实测，§M7.38）。
    let owner = shape
        .owners
        .iter()
        .filter(|(path, _)| rest == path || rest.starts_with(&format!("{path}::")))
        .max_by_key(|(path, _)| path.len())
        .map(|(_, owner)| owner.clone());
    match (owner, had_prefix) {
        (Some(owner), _) => format!("{owner}::{rest}"),
        (None, true) => format!("crate::{rest}"),
        (None, false) => rest.to_owned(),
    }
}

/// The module a typed graft expression names, i.e. everything before the
/// `::NODE_ID` the expression has to end with.
/// 类型化 graft 表达式所命名的模块，即表达式结尾那个 `::NODE_ID` 之前的全部内容。
///
/// `None` means the expression does not end in `NODE_ID`, so there is no module
/// to name and no assertion to emit. That is a boundary, not a fallback: a cut
/// that cannot name both types is a cut whose substitution nothing checks, and
/// the caller is expected to say so rather than guess.
/// 返回 `None` 表示该表达式不以 `NODE_ID` 结尾，因此没有可命名的模块，也没有断言可发射。
/// 这是边界而不是兜底：命名不出两端类型的切口，其替换没有任何东西检查，调用方应当如实
/// 说明，而不是猜。
/// The module part of a path, without the `crate::` a source spelling may carry.
/// 一条路径里的模块部分，去掉源码写法可能带的 `crate::`。
///
/// The two spellings are both deliberate: a graft expression names a module the way the source writes it
/// (`crate::control::object::slider`), while a face record keeps the module's own path
/// (`control::object::slider`). Comparing them means comparing this part.
/// 两种写法都是**有意**的：切口表达式按源码的写法命名模块（`crate::control::object::slider`），而面记录里存的是
/// 模块自己的路径（`control::object::slider`）。比它们，比的就是这一段。
pub(super) fn bare_module(path: &str) -> &str {
    path.trim_start_matches("crate::")
}

/// The name a face owns among its siblings: the module path's last segment, as the declaration macros
/// derived it (`last_path_segment(module_path!())`). A face at the crate root has no module segment, so it
/// gets the package name — unique, even if the exact crate-root name is not knowable from a build script.
/// 一个面在兄弟之间拥有的名字：模块路径的末段，与声明宏推导它的方式相同（`last_path_segment(module_path!())`）。
/// 位于 crate 根的面没有模块段，于是取包名——它仍然唯一，尽管构建脚本无从知道 crate 根的精确名字。
pub(super) fn registry_name_for(
    face: &crate::build_method::static_plan::StaticFaceRecord,
) -> String {
    match face.module.rsplit("::").next() {
        Some(segment) if !segment.is_empty() => segment.to_owned(),
        _ => std::env::var("CARGO_PKG_NAME")
            .unwrap_or_default()
            .replace('-', "_"),
    }
}

pub(super) fn face_module(expression: &str) -> Option<&str> {
    expression.trim().strip_suffix("::NODE_ID")
}

/// The registration rule one face asserts against, spelled for the crate that compiles it.
/// 某个面据以断言的那条注册规则，按编译它的 crate 拼写。
///
/// A parent that is an **ancestor shell** owns no `REGISTRATION` — the shell carries the ancestor's
/// `NODE_ID` and nothing else — so it asserts against `ANY`; so does a face whose parent is not a face at
/// all. The ghost reached that answer by accident (its face list holds no shells); the facade named the
/// shell and could not resolve it until this rule existed (measured, §M7.38).
/// **祖先壳**不拥有 `REGISTRATION`（壳只携带祖先的 `NODE_ID`），因此按 `ANY` 断言；父级根本不是面的面也一样。
/// 幽灵是**碰巧**得到这个答案的（它的面表里没有壳）；facade 点名了那个壳，直到有了这条规则才解析得到
/// （实测，§M7.38）。
pub(super) fn parent_rule(
    shape: &ShapeRender<'_>,
    modules: &std::collections::BTreeMap<registry_identity::NodeId, &str>,
    assembled: &std::collections::BTreeMap<registry_identity::NodeId, String>,
    parent: registry_identity::NodeId,
    registry: &str,
) -> String {
    let parent_bytes = parent.into_bytes();
    let is_shell = shape.ancestors.iter().any(|(_, id)| *id == parent_bytes);
    if is_shell {
        return format!("{registry}::RegistrationRule::ANY");
    }
    // A parent written as a `Registration` literal emits no module-level `REGISTRATION`: its snapshot is
    // the one this tree assembles, and that is what carries the rule.
    // 写成 `Registration` 字面量的父面不发射模块级 `REGISTRATION`：它的快照是**本树拼装**的那一个，规则在它身上。
    match (assembled.get(&parent), modules.get(&parent)) {
        (Some(name), _) => format!("{name}.registry_rule"),
        (None, Some(module)) => format!("{module}::REGISTRATION.registry_rule"),
        (None, None) => format!("{registry}::RegistrationRule::ANY"),
    }
}

/// The rule a face imposes on its children, spelled as the expression the tree feeds `FaceInputs`.
/// 一个面对其子面施加的规则，写成生成树喂给 `FaceInputs` 的那个表达式。
///
/// The declaration macros resolved it the same way — a face that owns a registry names its own
/// `registry_rule` module's constant as a Rust path, and everything else admits any child (`ANY`) — so
/// this reproduces that rule rather than inventing a second one. A face that owns no registry gets
/// `ANY`, which is also what the macro's fallback produced.
/// 声明宏当年也是这么解析的——拥有注册机的面把自己的 `registry_rule` 模块常量写成 Rust 路径，其余一律接纳任意
/// 子面（`ANY`）——因此这里复述那条规则，而不是另造一条。不拥有注册机的面取 `ANY`，也正是宏回退时给出的值。
pub(super) fn resolve_own_rule(module: &str, owns_registry: bool, registry: &str) -> String {
    if owns_registry && !module.is_empty() {
        format!("crate::{module}::registry_rule::REGISTRATION_RULE")
    } else {
        format!("{registry}::RegistrationRule::ANY")
    }
}

/// One `StaticGraftCut` constructor, spelled for the crate that emits it.
/// 一个 `StaticGraftCut` 构造式，按发射它的 crate 拼写。
///
/// A **typed** cut is emitted verbatim, so the compiler resolves the Rust expressions the author
/// wrote instead of a selector string — through `owned`, because `crate::` inside a facade means the
/// facade. A **string** cut keeps the historical form, with a range split into `new_range` from the
/// two endpoint fields — never by re-splitting the path text, which would cut a path that literally
/// contains `" to "` in half.
/// **类型化**切口原样发射，编译器因此解析作者写的 Rust 表达式，而不是选择器字符串——经由 `owned`，因为
/// facade 里的 `crate::` 指的是 facade 自己。**字符串**切口保持原有形式，区间用两个端点字段拆成
/// `new_range`——绝不重新拆分路径文本，否则字面含有 `" to "` 的路径会被拦腰截断。
pub(super) fn graft_constructor(
    shape: &ShapeRender<'_>,
    graft: &GraftSyntax,
    registry: &str,
) -> String {
    match &graft.expressions {
        Some(expressions) => match &expressions.cut_end {
            Some(end) => format!(
                "{registry}::StaticGraftCut::from_id_range({}, {}, {}, {})",
                owned(shape, &expressions.cut),
                owned(shape, end),
                owned(shape, &expressions.graft),
                graft.full
            ),
            None => format!(
                "{registry}::StaticGraftCut::from_ids({}, {}, {})",
                owned(shape, &expressions.cut),
                owned(shape, &expressions.graft),
                graft.full
            ),
        },
        None => match &graft.cut_end {
            Some(end) => format!(
                "{registry}::StaticGraftCut::new_range({:?}, {end:?}, {:?}, {})",
                graft.cut, graft.graft, graft.full
            ),
            None => format!(
                "{registry}::StaticGraftCut::new({:?}, {:?}, {})",
                graft.cut, graft.graft, graft.full
            ),
        },
    }
}

/// Whether a typed graft expression names a module **this** crate compiles.
/// 某个类型化 graft 表达式是否点名了**本** crate 编译的模块。
///
/// A crate may only name modules it has, and a partition is exactly a crate *not* having some: the
/// host hands `panel::frame` away, so a cut written inside it is a module the host no longer
/// compiles. Emitting the entry anyway failed with `error[E0433]: cannot find 'frame' in 'panel'`
/// (measured on the first end-to-end build whose host carried a graft **inside** a claimed subtree —
/// the fixture before it had no graft at all, which is why this hid). The entry belongs to the crates
/// that compile the face: the ghost that owns it, and the facade that carries the union.
/// 一个 crate 只许点名它拥有的模块，而分区正是一个 crate **没有**某些模块：宿主把 `panel::frame` 交出去，
/// 因此写在其**内**的切口是宿主不再编译的模块。照旧发射这条条目会报
/// `error[E0433]: cannot find 'frame' in 'panel'`（首次端到端构建实测：宿主的 graft 落在被认领的子树
/// **之内**——再往前那个夹具的宿主根本没有 graft，所以它一直躲着）。这条条目属于编译该面的 crate：拥有它的
/// 幽灵，以及携带并集的 facade。
///
/// `true` for an expression that names no module this rule can judge: a string selector is data, and a
/// typed cut the parser did not resolve to a `::NODE_ID` path names nothing here to check.
/// 对这条规则判不了的表达式返回 `true`：字符串选择器是数据，而解析器没能解析成 `::NODE_ID` 路径的类型化
/// 切口在这里没有可查的东西。
pub(super) fn names_a_compiled_module(faces: &[StaticFaceRecord], expression: &str) -> bool {
    match face_module(expression) {
        // `crate::NODE_ID` is the **root** of the tree, and no face record stands for it: a cut anchored
        // there covers everything, so this rule cannot say which crate it belongs to and stays conservative.
        // Getting this wrong is not theoretical — the first version of this filter compared `crate` against
        // every face's module, matched nothing, and dropped every root-face cut from the host that had one:
        // measured by `tools/xirang-graft-matrix`, "the plan does not record the root-face cut" (audit
        // `M7`, §M7.56).
        // `crate::NODE_ID` 是这棵树的**根**，而没有任何一条面记录代表它：锚在那里的切口覆盖一切，因此这条规则
        // 说不出它属于哪个 crate，于是保持保守。弄错它不是理论问题——本过滤器的第一版拿 `crate` 去比每个面的
        // module、什么都没匹配到，于是把带根切口的宿主那条切口丢掉了：由 `tools/xirang-graft-matrix` 实测，
        // "the plan does not record the root-face cut"（审计 `M7`，§M7.56）。
        Some("crate" | "self") => true,
        Some(module) => {
            let module = module
                .strip_prefix("crate::")
                .or_else(|| module.strip_prefix("self::"))
                .unwrap_or(module);
            faces.iter().any(|face| face.module == module)
        }
        None => true,
    }
}

/// The graft cuts **this** crate emits, in declaration order.
/// **本** crate 发射的那些 graft 切口，按声明顺序。
///
/// One rule, two readers: `render_lib` fills `BUILTIN_GRAFT_CUTS` from this list, and the build writes
/// the same list into `graft_plan.tsv`. They used to disagree — the renderer filtered and the audit text
/// did not, so every generated package's `graft_plan.tsv` listed the **host's whole plan** while the
/// table it actually compiled carried one entry or none. Measured on a two-ghost host: the board ghost
/// emitted exactly its own cut, and its own audit file claimed the cut belonged to the input ghost too.
/// An audit text that describes a table nobody compiled is the "self-description must match behaviour"
/// failure, and it is what made two independent readers conclude the release shape "carries every cut
/// everywhere" (audit `M7`, §M7.55).
/// 一条规则、两个读者：`render_lib` 由这份清单填充 `BUILTIN_GRAFT_CUTS`，构建把同一份清单写进
/// `graft_plan.tsv`。两者过去并不一致——渲染器过滤、审计文本不过滤，于是每个生成包的 `graft_plan.tsv`
/// 都列着**宿主的整份计划**，而它真正编译的表里只有一条或一条都没有。实测（两幽灵的宿主）：board 幽灵
/// 只发射自己那条切口，而它自己的审计文件声称那条切口也属于 input 幽灵。**描述一张没人编译过的表的
/// 审计文本**正是"自我描述必须与行为一致"那条的反面，也是让两位独立读者得出"发布形状到处都是全量切口"
/// 的原因（审计 `M7`，§M7.55）。
///
/// `facade` short-circuits the filter because the facade carries the union: it has no modules of its own,
/// and every `crate::<module>` in it is rewritten to the crate that compiles that module.
/// `facade` 直接短路过滤，因为 facade 携带并集：它自己没有模块，而它里面每个 `crate::<模块>` 都被改写成
/// 编译该模块的那个 crate。
/// A range needs both endpoints in this fragment; a range spanning crates is emitted by the facade,
/// where both owners are available. Never manufacture a dependency between sibling ghost crates.
/// 区间要求当前碎片能点名两个端点；跨 crate 的区间由可访问双方的 facade 发射，不能给兄弟幽灵强加依赖。
/// A local replacement in another owner follows the same rule; external dependency paths remain intact.
/// 本地替换实现位于另一个归属时也遵循此规则；外部依赖路径保持原样。
pub(crate) fn emitted_grafts(
    faces: &[StaticFaceRecord],
    grafts: &[GraftSyntax],
    facade: bool,
) -> Vec<GraftSyntax> {
    grafts
        .iter()
        .filter(|graft| {
            facade
                || graft.expressions.as_ref().is_none_or(|expressions| {
                    names_a_compiled_module(faces, &expressions.cut)
                        && expressions
                            .cut_end
                            .as_ref()
                            .is_none_or(|end| names_a_compiled_module(faces, end))
                        && (!(expressions.graft.starts_with("crate::")
                            || expressions.graft.starts_with("self::"))
                            || names_a_compiled_module(faces, &expressions.graft))
                })
        })
        .cloned()
        .collect()
}

#[cfg(test)]
#[path = "owners_tests.rs"]
mod owners_tests;

#[cfg(test)]
mod bare_module_tests {
    use super::bare_module;

    /// The two spellings of one module are one module: a graft expression writes it the way the source
    /// does, a face record writes its own path. Comparing them without this rule silently falls back to
    /// the macro-spelling names (`__Preset`, `__Parts`) for a face that has neither.
    /// 同一个模块的两种写法是同一个模块：切口表达式按源码的写法写它，面记录写的是它自己的路径。少了这条规则，
    /// 比较会静默回退到宏拼法的名字（`__Preset`、`__Parts`），而那个面两者都没有。
    #[test]
    fn the_two_spellings_of_one_module_are_one_module() {
        assert_eq!(
            bare_module("crate::control::object::slider"),
            bare_module("control::object::slider"),
            "the source spelling and the record spelling name one module"
        );
        assert_ne!(
            bare_module("crate::control::object::slider"),
            bare_module("control::object::frame"),
            "and different modules stay different"
        );
    }
}
