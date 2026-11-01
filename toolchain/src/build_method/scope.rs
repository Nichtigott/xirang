//! First-pass source scope: which registration faces survive before expansion.
//! 首轮源码作用域：rustc 展开之前哪些注册面得以存活。
//!
//! The scope decides which registration faces survive before rustc expands the
//! generated module tree. It lives in the build step so discovery and identity
//! share one parser and one node-identity implementation; entry resolution and
//! face discovery live in sibling modules and are imported here. The active-face
//! predicates (`source_is_active`, `face_source_is_active`, `module_feature`,
//! `collect_active_ids`) moved here from the discovery cache module: they answer
//! the scope's own question, not the cache's (`NAM-04`).
//! 作用域决定 rustc 展开生成模块树之前哪些注册面得以存活。它位于构建步骤中，
//! 使发现与身份计算共用同一套解析与 node identity 实现；入口解析与注册面发现位于
//! 同级模块，在此处导入使用。活跃注册面判定（`source_is_active`、`face_source_is_active`、
//! `module_feature`、`collect_active_ids`）从发现缓存模块搬到这里：它们回答的是作用域
//! 自己的问题，而不是缓存的（`NAM-04`）。

use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::path::Path;

use xirang_kernel::lexicon;

use super::diagnostics::{BuildDiagnostic, BuildDiagnostics};
use super::discovery_node::{Node, relative_display};
use super::entry::{HostEntry, path_mentions_module};
use super::graft_view::{face_declares_plugin, graft_expression_module, string_cut_modules};
use super::node_identity::{collect_node_ids, node_id, select_module_subtree};
use super::registry_identity::{IDENTITY_SCHEMA, NodeId};
use super::registry_syntax::{graft_entries, source_references};
use super::scope_faces::{collect_faces, has_selected_face};

/// First-pass source scope. It gates whole registration faces before rustc
/// expands them; functions inside an enabled face are intentionally untouched.
/// 第一次源码范围修剪。在 rustc 展开前按注册面整体门控；启用注册面内部的函数
/// 不在这里处理。
#[derive(Clone, Debug)]
pub(crate) struct SourceScope {
    pub(crate) roots: Option<BTreeSet<NodeId>>,
    pub(crate) reason: &'static str,
}

impl SourceScope {
    /// Read the scope from the environment, reporting unusable values.
    /// 从环境读取范围，并报告不可用的取值。
    ///
    /// A refused value is a diagnostic and a conservative fallback rather than a
    /// panic: the build fails either way, and a diagnostic reaches `check --json`
    /// while a panic leaves its stdout empty. The fallback is the full tree, so a
    /// reader who ignores the diagnostic still prunes nothing.
    /// 被拒绝的取值是诊断加保守回退，而不是 panic：两种方式构建都失败，但诊断能到达
    /// `check --json`，而 panic 会让它的 stdout 一片空白。回退是全树，因此忽略诊断的读者
    /// 也仍然什么都不剪。
    pub(crate) fn from_environment(
        src: &Path,
        nodes: &[Node],
        entry: &HostEntry,
        claims: &[String],
        errors: &mut BuildDiagnostics,
    ) -> Self {
        let Some(raw) = env::var_os(lexicon::SCOPE_ENV) else {
            return Self::auto_reporting(src, nodes, entry, claims, errors);
        };
        Self::from_raw(&raw.to_string_lossy(), src, nodes, entry, claims, errors)
    }

    /// Read the scope from one explicit `XIRANG_SCOPE` value.
    /// 从明确的 `XIRANG_SCOPE` 取值读取范围。
    ///
    /// The value arrives as a parameter rather than from the environment so the
    /// refusals below can be pinned by tests: `#[test]` threads share the process
    /// environment, so a test that set it would race every other test.
    /// 取值作为参数传入而不是就地读环境，使下面这些拒绝能被测试钉住：`#[test]` 线程共享
    /// 进程环境，设置它的测试会与所有其他测试赛跑。
    pub(crate) fn from_raw(
        raw: &str,
        src: &Path,
        nodes: &[Node],
        entry: &HostEntry,
        claims: &[String],
        errors: &mut BuildDiagnostics,
    ) -> Self {
        let raw = if let Some((version, values)) = raw.split_once(':') {
            if version.strip_prefix('v') != Some(IDENTITY_SCHEMA) {
                errors.push(BuildDiagnostic::new(
                    "scope",
                    format!(
                        "{} uses identity schema `{version}`, expected `v{IDENTITY_SCHEMA}`",
                        lexicon::SCOPE_ENV
                    ),
                ));
                return Self {
                    roots: None,
                    reason: "scope-invalid",
                };
            }
            values
        } else {
            raw
        };
        if raw.trim().is_empty() || raw.trim().eq_ignore_ascii_case("all") {
            return Self {
                roots: None,
                reason: "scope-all",
            };
        }
        if raw.trim().eq_ignore_ascii_case("auto") {
            return Self::auto_reporting(src, nodes, entry, claims, errors);
        }
        let mut refused = false;
        let ids = raw
            .split(|ch: char| ch == ',' || ch.is_ascii_whitespace())
            .filter(|value| !value.is_empty())
            .filter_map(|value| match value.parse::<NodeId>() {
                Ok(id) => Some(id),
                Err(_) => {
                    refused = true;
                    errors.push(BuildDiagnostic::new(
                        "scope",
                        format!(
                            "{} entry `{value}` is not a 32-digit node identity",
                            lexicon::SCOPE_ENV
                        ),
                    ));
                    None
                }
            })
            .collect::<BTreeSet<_>>();
        if refused && ids.is_empty() {
            // Nothing parsed, so an explicit scope would prune the whole tree.
            // The diagnostic above already fails the build; keeping everything is
            // the conservative reading of a value the build refused.
            // 一个都没解析出来，因此"显式范围"会把整棵树剪光。上面的诊断已经让构建失败；
            // 对一个被构建拒绝的取值，保留一切是保守的读法。
            return Self {
                roots: None,
                reason: "conservative-fallback",
            };
        }
        let mut known = BTreeSet::new();
        collect_node_ids(src, nodes, &mut known);
        let unknown = ids
            .difference(&known)
            .map(ToString::to_string)
            .collect::<Vec<_>>();
        if !unknown.is_empty() {
            errors.push(BuildDiagnostic::new(
                "scope",
                format!(
                    "{} contains unknown node identity(s): {}",
                    lexicon::SCOPE_ENV,
                    unknown.join(", ")
                ),
            ));
            // An identity no node owns narrows to nothing, so honouring the value
            // would prune the whole tree. The diagnostic already fails the build;
            // keeping everything is the reading that cannot silently lose faces.
            // 没有任何节点拥有的身份会把范围收窄到空，因此照办会剪光整棵树。诊断已经让构建
            // 失败；保留一切是不会静默丢掉注册面的读法。
            return Self {
                roots: None,
                reason: "conservative-fallback",
            };
        }
        Self {
            roots: Some(ids),
            reason: "explicit",
        }
    }

    /// Derive a conservative face scope from the package entry source.
    /// 从包入口源码保守推导注册面作用域。
    ///
    /// This lives in the build step so the same parser and the same node identity
    /// implementation are used for discovery and identity. A source scan is
    /// not a complete rustc call graph; when no face can be proven, or when
    /// dynamic/generated code is present, the safe result is the full tree.
    /// 逻辑放在构建步骤中，发现和身份计算共用同一套解析与 node identity 实现。源码扫描
    /// 不是完整 rustc 调用图；无法证明注册面或发现动态/生成代码时，安全结果是全树。
    fn auto_reporting(
        src: &Path,
        nodes: &[Node],
        entry: &HostEntry,
        claims: &[String],
        errors: &mut BuildDiagnostics,
    ) -> Self {
        Self::auto_from_entry_reporting(src, nodes, entry.path(), claims, errors)
    }

    /// Derive the scope from one explicit entry source, without reading the
    /// environment, so the derivation can be pinned by a test.
    /// 从一个明确的入口源码推导作用域，不读环境变量，因此推导过程可以被测试钉住。
    ///
    /// The entry is passed in rather than resolved here: pruning and the
    /// generated cut table have to read the same file, and two independent
    /// resolutions are exactly how they stopped agreeing. A test pins that by
    /// handing both readers one explicit entry
    /// (`a_configured_entry_drives_the_scope_and_the_cut_table`).
    /// 入口由外部传入而不是就地解析：剪枝与生成的切口表必须读同一个文件，而两次各自
    /// 独立的解析正是它们开始不一致的原因。测试通过把同一个明确入口交给两个读取者来
    /// 钉住这一点（`a_configured_entry_drives_the_scope_and_the_cut_table`）。
    /// The entry-derived scope with the findings thrown away (test-only).
    /// 丢弃发现结果的入口推导作用域（仅测试）。
    #[cfg(test)]
    fn auto_from_entry(src: &Path, nodes: &[Node], entry: &Path) -> Self {
        Self::auto_from_entry_reporting(src, nodes, entry, &[], &mut BuildDiagnostics::default())
    }

    /// The same derivation with crate claims, so the forced-liveness rule can be pinned.
    /// 同一次推导，带 crate 认领，好让那条强制存活的规则能被钉住。
    #[cfg(test)]
    fn auto_from_entry_with_claims(
        src: &Path,
        nodes: &[Node],
        entry: &Path,
        claims: &[String],
    ) -> Self {
        Self::auto_from_entry_reporting(src, nodes, entry, claims, &mut BuildDiagnostics::default())
    }

    fn auto_from_entry_reporting(
        src: &Path,
        nodes: &[Node],
        entry: &Path,
        claims: &[String],
        errors: &mut BuildDiagnostics,
    ) -> Self {
        let Some(entry_source) = fs::read_to_string(entry).ok() else {
            return Self {
                roots: None,
                reason: "missing-entry",
            };
        };
        // Graft declarations belong to the host entry. Parse them here so a
        // malformed overlay fails at build time; the plan itself is applied by
        // the host against an external Registry at runtime or release setup.
        let cuts = match graft_entries(&entry_source) {
            Ok(cuts) => cuts,
            Err(error) => {
                errors.push(BuildDiagnostic::new(
                    "graft-entry",
                    format!(
                        "invalid graft declaration in `{}`: {error}",
                        entry.display()
                    ),
                ));
                Vec::new()
            }
        };
        // Two cuts that overlap have no defined meaning: a cut replaces a whole subtree, so an entry
        // whose target sits inside another entry's subtree is a question with no answer. Refuse by
        // name — the same rule the crate declaration already applies to overlapping claims
        // (`shape::subtree_overlaps`, one implementation, `::`-segment boundaries).
        // 两个重叠的切口没有确定的含义：切口替换的是整棵子树，因此目标落在另一个切口子树里的条目是一个没有答案
        // 的问题。按名拒绝——crate 声明侧对重叠的认领早就是这么做的（`shape::subtree_overlaps`，一份实现，
        // 以 `::` 段为边界）。
        // Overlap is answered by **declaration order**: the later entry wins and the earlier one is
        // dropped (the maintainer's ruling). The rule lives in the kernel, and the hint is spoken once
        // by the build entry point; this reader only applies it.
        // 重叠由**声明顺序**作答：后一条赢、前一条被丢掉（维护者的裁定）。规则住在内核，提示由构建入口说一次；
        // 这个读取者只应用它。
        let pairs: Vec<(String, String)> = cuts
            .iter()
            .filter_map(|cut| cut_subtree(&cut.cut).map(|subtree| (subtree, cut.cut.clone())))
            .collect();
        let (superseded, _) = xirang_kernel::registry_core::shape::superseded_by_later(&pairs);
        let cuts: Vec<_> = cuts
            .into_iter()
            .enumerate()
            .filter(|(position, _)| !superseded.contains(position))
            .map(|(_, cut)| cut)
            .collect();
        let faces = collect_faces(src, nodes);
        if faces.is_empty() {
            return Self {
                roots: None,
                reason: "no-registration-face",
            };
        }
        let mut selected = BTreeSet::new();
        let mut queue = vec![entry_source];
        // Forced liveness roots. Graft slots and plugin-declaring faces are
        // replaceable community surface: a minimal tree must never prune
        // them just because no reachable source mentions them, or the graft
        // plan loses its cut target at release setup.
        // 强制存活根：嫁接槽位与插件声明面是可替换的社区面，
        // minimal 树绝不能因可达源码未提及就把它们剪掉。
        for cut in &cuts {
            // A declared slot is a face the host lets someone else replace, and
            // the scope keeps exactly the subtrees the cuts name plus the plugin
            // faces below. A typed cut names its target with a Rust path to a
            // real face, so its module is matched exactly instead of guessed from
            // a registry path; a string cut names the same face by registry path.
            // Both spellings narrow, and they must narrow identically: the
            // spelling is not allowed to decide the policy. What a cut may never
            // do is narrow by accident, so an endpoint the build cannot place
            // keeps the whole tree and says why.
            // 声明出来的槽位就是宿主允许别人替换的注册面，作用域保留的正是切口命名的子树
            // （外加下面的插件面）。类型化切口用指向真实注册面的 Rust 路径命名目标，因此
            // 精确匹配模块、不必从注册路径猜测；字符串切口用注册路径命名同一个面。
            // 两种写法都收窄，而且必须收窄到同一结果：写法不允许决定策略。切口绝不允许
            // 碰巧收窄，因此构建定位不到的端点保留整棵树并说明原因。
            //
            // A range cut names two targets, and both have to stay live: the
            // overlay replaces everything between them. The far endpoint is now
            // separate data (`cut_end`) rather than part of the path text, so a
            // path that literally contains `" to "` is one target and is not
            // split here. The string form used to hand the whole `"start to end"`
            // text to the module mapper, so the module name contained a space,
            // matched no face, and the cut pinned nothing — leaving its targets
            // to be pruned and the overlay to fail with `UnknownTarget` later.
            // 区间切口命名两个目标，两者都必须保持存活：覆盖层替换它们之间的全部内容。
            // 远端现在是独立数据（`cut_end`）而不是路径文本的一部分，因此字面含有
            // `" to "` 的路径是单个目标，不会在这里被拆分。字符串形式过去把整段
            // `"start to end"` 交给模块映射，于是模块名里带空格、匹配不到任何面、
            // 切口什么也没钉住——目标随后被剪掉，overlay 再以 `UnknownTarget` 失败。
            let resolved = match &cut.expressions {
                Some(expressions) => {
                    let cut_end = expressions.cut_end.as_deref();
                    let module = graft_expression_module(&expressions.cut, &faces);
                    let end_module = cut_end.and_then(|end| graft_expression_module(end, &faces));
                    match (module, cut_end.is_some() && end_module.is_none()) {
                        (Some(module), false) => Ok((module, end_module)),
                        // A typed cut names a real face, so a path the build
                        // cannot place is an unreadable declaration, not a
                        // smaller tree.
                        // 类型化切口命名真实注册面，因此构建定位不到的路径是读不懂的
                        // 声明，而不是一棵更小的树。
                        _ => Err("graft-typed-cut-unrecognized"),
                    }
                }
                None => match string_cut_modules(&cut.cut, cut.cut_end.as_deref()) {
                    // A range whose far endpoint mapped to the whole tree is not
                    // understood, so the whole tree is kept rather than pruned
                    // wrongly.
                    // 区间切口的远端映射到整棵树，说明这个区间没被读懂，因此保留整棵树
                    // 而不是错剪。
                    (Some(module), end) if !(cut.cut_end.is_some() && end.is_none()) => {
                        Ok((module, end))
                    }
                    _ => Err("graft-root-cut"),
                },
            };
            let (module, end_module) = match resolved {
                Ok(pair) => pair,
                Err(reason) => {
                    return Self {
                        roots: None,
                        reason,
                    };
                }
            };
            queue.extend(select_module_subtree(&faces, &module, &mut selected));
            if let Some(end_module) = end_module {
                queue.extend(select_module_subtree(&faces, &end_module, &mut selected));
            }
        }
        // A **crate claim** is a forced-liveness root for the same reason a graft slot is: the crate
        // exists to compile that subtree. Without this, the narrowing above prunes a claimed subtree
        // that holds no declared slot — and then **nobody** compiles it, silently: the host handed it
        // away, so the host does not compile it either. Measured on the two-claim fixture: the ghost
        // mounted `panel/frame/widget/{widget,cap}.rs` and neither `panel/gauge/gauge.rs` nor
        // `panel/gauge/needle/needle.rs`, which no crate in that shape compiled.
        // **crate 认领**与嫁接槽位同理，是强制存活根：那个 crate 存在的意义就是编译这棵子树。没有这一条，
        // 上面的收窄会剪掉一棵"里面没有声明槽位"的被认领子树——而且**没有谁**编译它，悄无声息：宿主把它交出去了，
        // 因此宿主也不编译它。在双认领夹具上实测：幽灵挂载了 `panel/frame/widget/{widget,cap}.rs`，而
        // `panel/gauge/gauge.rs` 与 `panel/gauge/needle/needle.rs` 一个都没挂——那个形状里没有任何 crate
        // 编译它们。
        for claim in claims {
            queue.extend(select_module_subtree(&faces, claim, &mut selected));
        }
        for face in &faces {
            if face_declares_plugin(src, face)
                && selected.insert(face.id)
                && let Ok(next) = fs::read_to_string(&face.source)
            {
                queue.push(next);
            }
        }
        let mut scanned = BTreeSet::new();
        let mut uncertain = false;
        while let Some(source) = queue.pop() {
            if !scanned.insert(source.clone()) {
                continue;
            }
            let Ok(references) = source_references(&source) else {
                return Self {
                    roots: None,
                    reason: "unparseable-or-dynamic-source",
                };
            };
            uncertain |= references.conservative;
            for path in &references.paths {
                let deepest = faces
                    .iter()
                    .filter(|face| path_mentions_module(path, &face.module))
                    .map(|face| face.module.len())
                    .max();
                for face in faces.iter().filter(|face| {
                    deepest == Some(face.module.len()) && path_mentions_module(path, &face.module)
                }) {
                    if selected.insert(face.id)
                        && let Ok(next) = fs::read_to_string(&face.source)
                    {
                        queue.push(next);
                    }
                }
            }
        }
        if selected.is_empty() || uncertain {
            return Self {
                roots: None,
                reason: "conservative-fallback",
            };
        }
        Self {
            roots: Some(selected),
            reason: "auto",
        }
    }

    pub(crate) fn includes(&self, src: &Path, node: &Node, selected_ancestor: bool) -> bool {
        if lexicon::SCOPE_ALWAYS_INCLUDED.contains(&node.name.as_str())
            || node
                .file
                .as_ref()
                .is_some_and(|file| lexicon::is_registration_path(&relative_display(src, file)))
        {
            return true;
        }
        let Some(roots) = &self.roots else {
            return true;
        };
        let selected_here = node_id(src, node).is_some_and(|id| roots.contains(&id));
        selected_ancestor
            || selected_here
            || node
                .children
                .iter()
                .any(|child| self.includes(src, child, selected_ancestor || selected_here))
    }
}

// The active-face predicates live here rather than beside the cache: they answer the
// scope's own question (which faces survive), and the cache module only writes the
// observation record. Moved out of the cache module by `NAM-04`.
// 活跃注册面判定住在这里而不是缓存旁边：它们回答的是作用域自己的问题（哪些面得以存活），
// 而缓存模块只负责写观察记录。`NAM-04` 把这几条判定从缓存模块搬到了这里。

pub(crate) fn collect_active_ids(
    src: &Path,
    nodes: &[Node],
    scope: &SourceScope,
    selected_ancestor: bool,
    active: &mut BTreeSet<NodeId>,
) {
    for node in nodes {
        if !scope.includes(src, node, selected_ancestor) {
            continue;
        }
        let selected_here = selected_ancestor
            || node_id(src, node).is_some_and(|id| {
                scope
                    .roots
                    .as_ref()
                    .is_some_and(|roots| roots.contains(&id))
            });
        if face_source_is_active(src, node, scope, selected_ancestor)
            && let Some(id) = node_id(src, node)
        {
            active.insert(id);
        }
        collect_active_ids(src, &node.children, scope, selected_here, active);
    }
}

pub(crate) fn source_is_active(
    src: &Path,
    node: &Node,
    scope: &SourceScope,
    selected_ancestor: bool,
) -> bool {
    scope.roots.is_none()
        || selected_ancestor
        || (node_id(src, node).is_some() && has_selected_face(src, node, false, scope))
        || node
            .file
            .as_ref()
            .is_some_and(|file| lexicon::is_registration_path(&relative_display(src, file)))
        || matches!(
            node.name.as_str(),
            "registry" | "rules" | "registry_rule" | "root_registry"
        )
}

pub(crate) fn face_source_is_active(
    src: &Path,
    node: &Node,
    scope: &SourceScope,
    selected_ancestor: bool,
) -> bool {
    node_id(src, node).is_some() && source_is_active(src, node, scope, selected_ancestor)
}

/// Keep development tooling outside the default core dependency graph.
/// 将开发工具隔离在默认核心依赖图之外。
pub(crate) fn module_feature(src: &Path, node: &Node) -> Option<&'static str> {
    let relative = node.file.as_ref().map(|file| relative_display(src, file))?;
    match relative.as_str() {
        "registry_core/authoring/authoring.rs" | "registry_core/syntax/syntax.rs" => {
            Some("authoring")
        }
        _ => None,
    }
}

/// The subtree a cut names, as a `::` module path, so the overlap rule can compare cuts written in
/// either spelling.
/// 切口点名的子树，写成 `::` 模块路径，好让重叠规则能比较两种拼写的切口。
///
/// A typed cut is `crate::a::b::NODE_ID` (or `crate::a::b::Type::NODE_ID`); a logical cut is
/// `root/a/b`. Both name the same module, and `root` is the crate itself, so dropping it is what
/// makes the two spellings comparable. `None` means "not a subtree this rule can judge" — a cut
/// this function cannot read is left to the other checks rather than guessed at.
/// 类型化切口是 `crate::a::b::NODE_ID`（或 `crate::a::b::Type::NODE_ID`）；逻辑切口是 `root/a/b`。
/// 两者点名同一个模块，而 `root` 就是 crate 本身，因此丢掉它才让两种拼写可比。`None` 表示"这条规则判不了
/// 这个切口"——读不懂的切口交给别的检查，而不是猜。
pub(crate) fn cut_subtree(cut: &str) -> Option<String> {
    let cut = cut.trim();
    if cut.is_empty() {
        return None;
    }
    if !cut.contains("::") {
        // A logical path: `root/a/b`, or `a/b` when the root is implied.
        let path = cut.strip_prefix("root/").unwrap_or(cut);
        let path = path.trim_matches('/');
        return (!path.is_empty()).then(|| path.replace('/', "::"));
    }
    let rest = cut.strip_prefix("crate::").unwrap_or(cut);
    let mut segments: Vec<&str> = rest.split("::").collect();
    if segments.last()? != &"NODE_ID" {
        return None;
    }
    segments.pop();
    // A marker type sits between the module and `NODE_ID`; modules are lower-case by convention,
    // so an upper-case last segment is that type rather than a module.
    if segments
        .last()
        .is_some_and(|segment| segment.chars().next().is_some_and(char::is_uppercase))
    {
        segments.pop();
    }
    let module = segments.join("::");
    (!module.is_empty()).then_some(module)
}

#[cfg(test)]
#[path = "scope_tests.rs"]
mod scope_tests;

/// What to say when a plan's cuts leave faces outside the build, or `None` when none are.
/// 计划的切口把一些面留在构建之外时该说的话；一个都没有时返回 `None`。
///
/// The narrowing is the feature; saying nothing about it was not. Neither `check` nor the build mentioned
/// it, so the only place the answer existed was `explain --overlay` — measured on a hand-built project: a
/// plan with one cut kept 3 of 6 faces and the build reported success (audit `M7`, §M7.58).
/// 收窄是功能；对它一言不发不是。`check` 与构建都不提它，因此答案只存在于 `explain --overlay` 里——
/// 在一个手工项目上实测：一条切口让 6 个面只剩 3 个，而构建报告成功（审计 `M7`，§M7.58）。
///
/// A pure function of the two lists rather than a print, so the sentence can be pinned: the counts, the
/// names, and the **paste-ready** clause are the parts a reader acts on.
/// 它是两份清单的纯函数而不是一次打印，因为这句话要能被钉住：计数、名字，以及那条**可粘贴**的子句，正是
/// 读者会照做的部分。
pub(crate) fn outside_slots_note(
    entry: &Path,
    all: &[super::scope_faces::FaceSource],
    roots: &BTreeSet<NodeId>,
    declared_slots: usize,
) -> Option<Vec<String>> {
    // What the plan publishes is not the roots alone: `includes` keeps a root, everything **under** it,
    // and every **ancestor** of it (the module chain has to be mounted). Comparing against the roots
    // alone therefore named faces that the same build did publish — measured on `examples/control-button`
    // with one extra face: the notice said `2 of 4 face(s) … control · panel`, while `control` is in
    // `BUILTIN_STATIC_FACES` of that very build (its cuts name `control::object::button` and
    // `control::object::slider`, and a container is kept for them).
    // 计划发布的不是"根"本身：`includes` 会留下根、根**之下**的一切，以及根的每一个**祖先**（模块链必须被挂载）。
    // 因此只与根比较会把同一次构建确实发布过的面点出来——在 `examples/control-button` 上加一个面实测：通知说
    // `2 of 4 face(s) … control · panel`，而 `control` 就在那次构建的 `BUILTIN_STATIC_FACES` 里（它的切口点名
    // `control::object::button` 与 `control::object::slider`，容器是为它们留下的）。
    let root_modules: Vec<&str> = all
        .iter()
        .filter(|face| roots.contains(&face.id))
        .map(|face| face.module.as_str())
        .collect();
    let outside: Vec<&super::scope_faces::FaceSource> = all
        .iter()
        .filter(|face| {
            !(roots.contains(&face.id)
                || root_modules.iter().any(|root| {
                    root.starts_with(&format!("{}::", face.module))
                        || face.module.starts_with(&format!("{root}::"))
                }))
        })
        .collect();
    if outside.is_empty() {
        return None;
    }
    let named = outside
        .iter()
        .take(5)
        .map(|face| face.module.as_str())
        .collect::<Vec<_>>()
        .join(" · ");
    let more = outside.len().saturating_sub(5);
    // The reason clause has to be the reason that actually holds. Measured (2026-10-10, `S8-ours`, kept
    // in `target/bench-runs/S8-ours/answer.md`): on a tree whose entry is three lines with **no**
    // `cut(`/`graft(` in it — the same session's `check` said `entry plan: 0 cut( site(s)` and the file's
    // hash never moved — this line said "the cuts in <entry> name the subtrees this build publishes" and
    // attributed a declaration that did not exist to that file ✗. What narrowed the build there was not a
    // cut but the active subtree, so a build with no declared slot says that instead (report §7h).
    // 原因子句必须是真正成立的那个原因。实测（2026-10-10，`S8-ours`，留档在
    // `target/bench-runs/S8-ours/answer.md`）：在一棵入口只有三行、里面**没有**任何 `cut(`/`graft(` 的树上
    // ——同一次会话的 `check` 报 `entry plan: 0 cut( site(s)`，而该文件哈希前后未变——这一行说"<入口> 里的
    // 切口命名了本次构建发布的子树"，把一个不存在的声明归因给了那个文件 ✗。那里收窄构建的不是切口而是活跃
    // 子树，因此没有声明任何槽位的构建改说后者（报告 §7h）。
    let reason = if declared_slots == 0 {
        format!(
            "{} declares no `cut`/`graft` slot, so what this build publishes is the active subtree \
             alone, and {} of {} face(s) are outside it and are not shipped: {named}{}",
            entry.display(),
            outside.len(),
            all.len(),
            if more > 0 {
                format!(" · and {more} more")
            } else {
                String::new()
            }
        )
    } else {
        format!(
            "the cuts in {} name the subtrees this build publishes, so {} of {} face(s) are outside \
             them and are not shipped: {named}{}",
            entry.display(),
            outside.len(),
            all.len(),
            if more > 0 {
                format!(" · and {more} more")
            } else {
                String::new()
            }
        )
    };
    let mut lines = vec![format!("xirang: {reason}")];
    for face in outside.iter().take(3) {
        lines.push(format!(
            "xirang:   to keep `{}`, declare a slot for it: \
             cut(crate::{}::NODE_ID) graft(crate::{}::NODE_ID),",
            face.module, face.module, face.module
        ));
    }
    lines.push(format!(
        "xirang: `{}=all` keeps the whole tree instead",
        lexicon::SCOPE_ENV
    ));
    Some(lines)
}
