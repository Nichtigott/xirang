//! Build-time declaration validation.
//! 构建期注册声明校验。

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use super::Node;
use super::diagnostics::{BuildDiagnostic, BuildDiagnostics};
use super::registration_phase;
use super::registry_syntax::{FaceSyntax, ParentSyntax, parse_face};
use super::source_walk::UnplacedFace;
use super::{SourceScope, collect_active_ids, face_source_is_active, node_id, relative_display};

pub(crate) fn aggregate_requirements(
    src: &Path,
    nodes: &[Node],
    include_demo: bool,
    scope: &SourceScope,
    cache_units: Option<&Path>,
) -> BuildDiagnostics {
    let active = scope.roots.as_ref().map(|_| {
        let mut active = std::collections::BTreeSet::new();
        collect_active_ids(src, nodes, scope, false, &mut active);
        active
    });
    // One memo for this pass: the registration phase resolves a parent per face, and a thousand faces
    // naming one folder face should read and parse it once.
    // 本趟共用一份备忘：注册相位每个面解析一次父级，而一千个面点名同一个文件夹面时，它只该被读解析一次。
    let _parents = crate::build_method::discovery_cache::ParentLookupScope::open();
    registration_phase::aggregate(src, include_demo, active.as_ref(), cache_units)
}

pub(crate) fn aggregate_stable_name_errors(src: &Path, nodes: &[Node]) -> BuildDiagnostics {
    let mut names = BTreeMap::<String, (String, usize)>::new();
    let mut errors = BuildDiagnostics::default();
    visit_stable_names(src, nodes, &mut names, &mut errors);
    errors
}

/// Check that a parent-specific macro agrees with the declared parent.
/// 校验父级专属宏名与声明的 parent 一致。
pub(crate) fn aggregate_parent_macro_errors(src: &Path, nodes: &[Node]) -> BuildDiagnostics {
    let mut errors = BuildDiagnostics::default();
    visit_parent_macro_errors(src, nodes, &mut errors);
    errors
}

fn visit_parent_macro_errors(src: &Path, nodes: &[Node], errors: &mut BuildDiagnostics) {
    for node in nodes {
        if let Some(file) = &node.file {
            let relative = relative_display(src, file);
            if let Ok(source) = fs::read_to_string(file)
                && let Some(face) = parsed_face(&source, &relative)
            {
                let declared = face
                    .macro_name
                    .strip_suffix("_object")
                    .filter(|name| *name != "external")
                    .filter(|name| *name != "control" || source.contains("generated-by=XiRang"));
                if let Some(declared) = declared {
                    let Some(parent) = face.parent() else {
                        errors.push(
                            BuildDiagnostic::new(
                                "parent-macro",
                                "parent-specific registration macro requires an explicit parent",
                            )
                            .at(relative.clone(), face.location.line)
                            .field("parent")
                            .expected(if declared == "root" {
                                "parent: crate::root_node_id(env!(\"CARGO_PKG_NAME\"))".to_owned()
                            } else {
                                format!("parent: crate::{declared}::NODE_ID")
                            })
                            .actual("missing"),
                        );
                        visit_parent_macro_errors(src, &node.children, errors);
                        continue;
                    };
                    // `NodeId::from_path` is a plain identity: it is not
                    // namespaced, while every face's own `NODE_ID` is. A parent
                    // written that way resolves at build time but not at run
                    // time, so the registry reports `<missing-parent>` after a
                    // successful build. Refuse it with the spelling that works.
                    // `NodeId::from_path` 是普通身份函数：它不带命名空间，而每个面自己的
                    // `NODE_ID` 带。这样写的父级在构建期能解析、运行期不能，于是构建成功
                    // 之后注册机会报 `<missing-parent>`。这里直接拒绝，并给出可用的写法。
                    if let ParentSyntax::FromPath { .. } = parent {
                        errors.push(
                            BuildDiagnostic::new(
                                "parent-macro",
                                "`NodeId::from_path` carries no namespace, so the runtime cannot resolve this parent",
                            )
                            .at(relative.clone(), face.location.line)
                            .field("parent")
                            .expected(format!(
                                "parent: crate::root_node_id(env!(\"CARGO_PKG_NAME\")) or crate::{declared}::NODE_ID"
                            ))
                            .actual("parent: NodeId::from_path(..)".to_owned()),
                        );
                        continue;
                    }
                    let expected = match parent {
                        ParentSyntax::Root => "root".to_owned(),
                        ParentSyntax::FromPath { source, .. } => Path::new(&source)
                            .file_stem()
                            .and_then(|stem| stem.to_str())
                            .unwrap_or("root")
                            .to_owned(),
                        ParentSyntax::NodePath(module) => module
                            .rsplit("::")
                            .find(|segment| !segment.is_empty())
                            .unwrap_or("root")
                            .to_owned(),
                    };
                    if declared != expected {
                        errors.push(
                            BuildDiagnostic::new(
                                "parent-macro",
                                "registration macro does not match its parent registry",
                            )
                            .at(relative.clone(), face.location.line)
                            .field("parent")
                            .expected(format!("crate::{expected}_object!"))
                            .actual(format!("crate::{}_object!", declared)),
                        );
                    }
                }
            }
        }
        visit_parent_macro_errors(src, &node.children, errors);
    }
}

fn visit_stable_names(
    src: &Path,
    nodes: &[Node],
    names: &mut BTreeMap<String, (String, usize)>,
    errors: &mut BuildDiagnostics,
) {
    for node in nodes {
        if let Some(file) = &node.file {
            let relative = relative_display(src, file);
            // The decoded name and the line it was written on come from the facts this pass already
            // read; the source is not opened at all when a unit answered for this face (`stem 2`).
            // 解码后的名字与它写在哪一行，来自本趟已读过的事实；当某个 unit 已经替这个面作答时，源码根本不会被打开
            // （`stem 2`）。
            if !xirang_kernel::lexicon::is_registration_path(&relative)
                && let Some((stable_name, line)) =
                    super::discovery_cache::face_string_field(file, &relative, "stable_name")
            {
                if let Some((previous_file, previous_line)) = names.get(&stable_name) {
                    errors.push(
                        BuildDiagnostic::new(
                            "stable-identity",
                            format!("duplicate stable_name `{stable_name}`"),
                        )
                        .at(relative, line)
                        .field("stable_name")
                        .expected(format!(
                            "unique; already declared at {previous_file}:{previous_line}"
                        ))
                        .actual(stable_name),
                    );
                } else {
                    names.insert(stable_name, (relative, line));
                }
            }
        }
        visit_stable_names(src, &node.children, names, errors);
    }
}

/// The module name a registry-owning face reads its rule from when it names none.
/// 拥有注册机的注册面在未点名规则时所读的模块名。
///
/// The name is the macro's, not this file's: `face_rule_or` (`macro/src/lib.rs`) expands an
/// unnamed rule into the **relative** path `super::registry_rule::REGISTRATION_RULE`, so the rule
/// module has to be the sibling of the face's own module. Spelling it here as a constant is what
/// keeps the check and the expansion from drifting apart.
/// 这个名字是宏的，不是本文件的：`face_rule_or`（`macro/src/lib.rs`）把未点名的规则展开成**相对**路径
/// `super::registry_rule::REGISTRATION_RULE`，因此规则模块必须是注册面自己模块的兄弟。在这里写成常量，
/// 正是让本检查与那次展开不会各自漂移的原因。
pub(crate) const REGISTRY_RULE_MODULE: &str = "registry_rule";

/// Refuse a face whose module reads a rule module the generated tree never mounts.
/// 拒绝这样的注册面：它的模块会去读一个生成树从未挂载的规则模块。
///
/// A face that owns a registry and names no rule at all reads the **canonical sibling** rule: the
/// macro expands the omitted field to `super::registry_rule::REGISTRATION_RULE`. The tree mounts a
/// module of that name only for a child folder of the face's own directory that holds
/// `<face>/registry_rule/registry_rule.rs`, so a face whose rule file was deleted, renamed, or moved
/// out of that folder cannot be compiled by any crate — and before this check existed `check`
/// reported `ok` while `cargo build` failed with `error[E0433]: cannot find registry_rule in super`.
/// The criterion is the emitted reference, not the file: an explicit `registry_rule:` expression
/// takes the author's path and needs no sibling (the external implementation that declares
/// `registry_rule: RegistrationRule::ANY` is the shipped example), and a face that owns no registry
/// keeps the permissive fallback, so neither is refused.
/// 拥有注册机、又没有点名任何规则的注册面读的是**同目录规范规则**：宏把省略的字段展开成
/// `super::registry_rule::REGISTRATION_RULE`。树只为注册面自己目录下的子文件夹（里面放着
/// `<面>/registry_rule/registry_rule.rs`）挂载同名的模块，因此规则文件被删除、改名或搬出那个文件夹的面
/// 任何 crate 都编译不过——而在本检查存在之前，`check` 报 `ok` 而 `cargo build` 报
/// `error[E0433]: cannot find registry_rule in super`。判据是**被发射的那条引用**，不是文件：显式写了
/// `registry_rule:` 的面走作者给的路径、不需要兄弟模块（声明 `registry_rule: RegistrationRule::ANY`
/// 的项目外实现就是出厂例子），不拥有注册机的面保留宽松兜底，两者都不会被拒绝。
pub(crate) fn aggregate_registry_rule_errors(
    src: &Path,
    nodes: &[Node],
    include_demo: bool,
    scope: &SourceScope,
) -> BuildDiagnostics {
    let mut errors = BuildDiagnostics::default();
    visit_registry_rule_errors(src, nodes, include_demo, scope, false, &mut errors);
    // A `source:` that names no file fails for the same reason as a missing rule: the build and the
    // declaration disagree before any crate compiles. The two travel together so the pipeline keeps
    // one call site for the declaration-level checks.
    // 点名不到文件的 `source:` 与缺失规则同因：在任何 crate 编译之前，构建与声明就已有分歧。两者一起走，
    // 让管线对声明层检查只保留一个调用点。
    visit_face_source_errors(src, nodes, include_demo, scope, false, &mut errors);
    errors
}

fn visit_registry_rule_errors(
    src: &Path,
    nodes: &[Node],
    include_demo: bool,
    scope: &SourceScope,
    selected_ancestor: bool,
    errors: &mut BuildDiagnostics,
) {
    for node in nodes {
        if node.name == crate::build_method::DEMO_ONLY_DIRECTORY && !include_demo
            || !scope.includes(src, node, selected_ancestor)
        {
            continue;
        }
        let selected_here = selected_ancestor
            || node_id(src, node).is_some_and(|id| {
                scope
                    .roots
                    .as_ref()
                    .is_some_and(|roots| roots.contains(&id))
            });
        // Only a face this run actually mounts can emit the reference: a node outside the build
        // scope is not rendered, so its rule is nobody's problem in this build.
        // 只有本次运行**确实挂载**的面才会发射那条引用：构建范围之外的节点不会被渲染，它的规则在本次构建里
        // 不归任何人管。
        // The facts answer both questions, and the cheap `contains("needs_registry")` gate this used to
        // carry before parsing is gone with the parse itself: a face that does not own a registry has no
        // row for the field, so there is nothing to avoid. (When the gate was added, a missing one cost
        // 1,000 parses; now it costs nothing, and the gate's own failure mode — a comment that mentions
        // the field still had to be parsed — disappears with it.)
        // 事实同时回答两个问题；这里过去在解析前挂的 `contains("needs_registry")` 便宜闸门随解析一起消失：
        // 不拥有注册机的面对该字段根本没有行，因此没有东西需要回避。（当年加上那道闸门时，缺它要付 1,000 次解析；
        // 现在它什么也不花，而它自己的失败模式——注释里提到该字段仍然必须被解析——也随之消失。）
        if let Some(file) = &node.file
            && let relative = relative_display(src, file)
            && face_source_is_active(src, node, scope, selected_ancestor)
            && let Some((_, line)) =
                super::discovery_cache::face_raw_field(file, &relative, "needs_registry")
            && super::discovery_cache::face_bool_field(file, &relative, "needs_registry")
                == Some(true)
            && super::discovery_cache::face_raw_field(file, &relative, "registry_rule").is_none()
            && !mounts_registry_rule(node)
        {
            errors.push(missing_registry_rule(src, file, line));
        }
        visit_registry_rule_errors(
            src,
            &node.children,
            include_demo,
            scope,
            selected_here,
            errors,
        );
    }
}

/// Whether the tree mounts the canonical sibling rule module for this face.
/// 树是否为这个注册面挂载了同目录的规范规则模块。
///
/// The mount is decided by the tree rather than by path text: the renderer emits `pub mod
/// registry_rule;` exactly for a child node of that name whose own file exists, so that is what
/// this asks. A `registry_rule/` folder whose file is gone keeps a node with no file, and the
/// renderer emits an empty container module for it — a module that resolves and has no
/// `REGISTRATION_RULE` in it, which still cannot compile.
/// 挂载由树决定，而不是由路径文本决定：渲染器只为**名字是 `registry_rule` 且自带文件**的子节点发射
/// `pub mod registry_rule;`，因此这里问的就是那件事。文件已经不在的 `registry_rule/` 文件夹留下一个没有
/// 文件的节点，渲染器为它发射一个空容器模块——一个能解析、里面却没有 `REGISTRATION_RULE` 的模块，照样
/// 编译不过。
fn mounts_registry_rule(node: &Node) -> bool {
    node.children
        .iter()
        .any(|child| child.name == REGISTRY_RULE_MODULE && child.file.is_some())
}

fn missing_registry_rule(src: &Path, file: &Path, line: usize) -> BuildDiagnostic {
    let relative = relative_display(src, file);
    let directory = Path::new(&relative)
        .parent()
        .map(|parent| xirang_kernel::declaration::portable_path(&parent.to_string_lossy()))
        .unwrap_or_default();
    let canonical = if directory.is_empty() {
        format!("{REGISTRY_RULE_MODULE}/registry_rule.rs")
    } else {
        format!("{directory}/{REGISTRY_RULE_MODULE}/registry_rule.rs")
    };
    BuildDiagnostic::new(
        "face-rule",
        format!(
            "this face owns a registry and names no rule, so its generated module reads \
             `super::{REGISTRY_RULE_MODULE}::REGISTRATION_RULE`, and this tree mounts no such \
             module beside it: no crate can compile it\n\
             way forward: put the rule back at `{canonical}`, or write \
             `registry_rule: <path>::REGISTRATION_RULE` in this declaration, or drop \
             `needs_registry: true` if this face owns no registry"
        ),
    )
    .at(relative, line)
    .field("needs_registry")
    .expected(canonical)
    .actual("no sibling registry_rule module")
}

/// Refuse a face whose `source:` names no file in this package.
/// 拒绝 `source:` 点名了本包中不存在的文件的注册面。
///
/// The spelling is an identity input: the compiled declaration derives its id from it, while the
/// plan bakes the id it read during discovery, so a spelling that names nothing is a face the build
/// stamps one id for and compiles another. `check` said ok and `cargo build` died with `E0080`
/// ("this face compiles an id the build did not bake") — measured by `tools/xirang-daily-behaviors`
/// case 9 after the example moved to the native builder form. Look beside the source root and
/// beside the package root, which are the two bases the shipped external declarations use.
/// 这条拼写是身份输入：编译出的声明由它导出 id，而计划烘焙的是发现在读取时的那一个，因此点名不到东西的
/// 拼写就是"构建盖一个 id、编译出另一个 id"的面。`check` 说 ok 而 `cargo build` 以 `E0080` 死掉
/// （"this face compiles an id the build did not bake"）——示例迁到原生构建器形之后由
/// `tools/xirang-daily-behaviors` 第 9 格实测。查找在源根与包根两处进行，出厂的外部声明用的正是这两个
/// 基准。
fn visit_face_source_errors(
    src: &Path,
    nodes: &[Node],
    include_demo: bool,
    scope: &SourceScope,
    selected_ancestor: bool,
    errors: &mut BuildDiagnostics,
) {
    for node in nodes {
        if node.name == crate::build_method::DEMO_ONLY_DIRECTORY && !include_demo
            || !scope.includes(src, node, selected_ancestor)
        {
            continue;
        }
        let selected_here = selected_ancestor
            || node_id(src, node).is_some_and(|id| {
                scope
                    .roots
                    .as_ref()
                    .is_some_and(|roots| roots.contains(&id))
            });
        if let Some(file) = &node.file
            && let relative = relative_display(src, file)
            && face_source_is_active(src, node, scope, selected_ancestor)
            && let Some((value, line)) =
                super::discovery_cache::face_string_field(file, &relative, "source")
        {
            let spelled = Path::new(&value);
            let exists = if spelled.is_absolute() {
                spelled.is_file()
            } else {
                src.join(spelled).is_file()
                    || src
                        .parent()
                        .is_some_and(|package| package.join(spelled).is_file())
            };
            if !exists {
                errors.push(missing_source(src, file, line, &value));
            }
        }
        visit_face_source_errors(
            src,
            &node.children,
            include_demo,
            scope,
            selected_here,
            errors,
        );
    }
}

fn missing_source(src: &Path, file: &Path, line: usize, value: &str) -> BuildDiagnostic {
    BuildDiagnostic::new(
        "face-source",
        format!(
            "this face names the source `{value}`, which is not a file in this package: the \
             compiled declaration derives its identity from that spelling, so the build bakes one \
             id and the declaration compiles another (`E0080`)\n\
             way forward: write the path of the file that really holds the implementation, or drop \
             `source(...)` if this face is compiled from the file it is written in"
        ),
    )
    .at(relative_display(src, file), line)
    .field("source")
    .actual(value.to_owned())
}

/// Decode the face in one discovered file, or `None` when it has none.
/// 解码一个已发现文件中的注册面；文件没有面时返回 `None`。
///
/// A malformed face yields `None` here on purpose. [`face_syntax_errors`] reports
/// it once, with a position, before any stage decodes fields, so every later
/// stage skips the file instead of aborting the process — that ordering is what
/// keeps `check --json` producing a document for a host whose face is broken.
/// 畸形面在这里有意返回 `None`。[`face_syntax_errors`] 会在任何阶段解码字段之前带着位置
/// 报告它一次，因此后续每个阶段都跳过该文件而不是打死进程——正是这个顺序让"宿主的面坏了"
/// 时 `check --json` 仍能产出文档。
pub(crate) fn parsed_face(source: &str, _display_path: &str) -> Option<FaceSyntax> {
    // Counted because this is the mechanism's own dependent variable (`stem 2`): the consumer switch
    // removes *parses per face*, and on a 1,000-face tree that is ~0.1 s inside a `declare` phase of
    // ~3 s — too small for wall clock to resolve, and not affected by machine drift the way a duration
    // is. The verbose phase line reports it beside the facts count.
    // 计数，是因为这正是这条机制自己的因变量（`stem 2`）：消费者切换消掉的是**每面的解析次数**，而在 1,000 面的树上
    // 那约值 0.1 s、却埋在一个约 3 s 的 `declare` 相位里——墙钟分辨不出来，而且它不像时长那样受机器漂移影响。
    // verbose 相位行会把它与"由事实作答"的次数一起报出来。
    FACE_PARSES.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    parse_face(source).ok().flatten()
}

/// How many face parses this process has run.
/// 本进程跑过多少次面解析。
pub(crate) fn face_parses() -> u64 {
    FACE_PARSES.load(std::sync::atomic::Ordering::Relaxed)
}

static FACE_PARSES: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// Report every discovered file whose registration face does not parse.
/// 报告每个注册面解析不了的已发现文件。
///
/// This runs before the stages that decode fields, because those stages cannot
/// describe a file they cannot parse and would otherwise abort.
/// 本函数在解码字段的各阶段之前运行，因为那些阶段描述不了自己解析不了的文件，否则只会
/// 打死进程。
pub(crate) fn face_syntax_errors(src: &Path, nodes: &[Node]) -> BuildDiagnostics {
    let mut errors = BuildDiagnostics::default();
    visit_face_syntax_errors(src, nodes, &mut errors);
    errors
}

fn visit_face_syntax_errors(src: &Path, nodes: &[Node], errors: &mut BuildDiagnostics) {
    for node in nodes {
        if let Some(file) = &node.file
            && let Ok(source) = fs::read_to_string(file)
            && let Err(error) = parse_face(&source)
        {
            errors.push(BuildDiagnostic::new("face-syntax", error.message).at(
                relative_display(src, file),
                error.location.as_ref().map_or(0, |location| location.line),
            ));
        }
        visit_face_syntax_errors(src, &node.children, errors);
    }
}

/// Turn the faces discovery could not place into diagnostics.
/// 把发现过程安放不了的面变成诊断。
pub(crate) fn unplaced_face_errors(unplaced: &[UnplacedFace]) -> BuildDiagnostics {
    let mut errors = BuildDiagnostics::default();
    for face in unplaced {
        errors.push(
            BuildDiagnostic::new(face.phase, face.message.clone())
                .at(face.relative.clone(), face.line),
        );
    }
    errors
}

#[cfg(test)]
mod tests {
    use super::{aggregate_parent_macro_errors, aggregate_registry_rule_errors};
    use crate::build_method::{Node, SourceScope};
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    /// A face that owns a registry and names no rule reads the rule module the generated tree mounts
    /// beside it (`super::registry_rule`). When that folder is gone the face cannot be compiled by
    /// any crate, so this check has to name it: `check` reporting `ok` while `cargo build` fails with
    /// `error[E0433]: cannot find registry_rule in super` is the defect this pins (measured on
    /// `examples/control-suite` before the check existed).
    /// 拥有注册机、又没有点名规则的注册面读的是生成树在它旁边挂载的规则模块（`super::registry_rule`）。
    /// 那个文件夹不在时任何 crate 都编译不了它，因此本检查必须点名它：`check` 报 `ok` 而 `cargo build`
    /// 报 `error[E0433]: cannot find registry_rule in super`，正是本条钉子所钉的缺陷（在本检查存在
    /// 之前于 `examples/control-suite` 上实测）。
    #[test]
    fn refuses_a_registry_owning_face_without_its_sibling_rule() {
        let root = temporary_directory("missing-rule");
        let panel = root.join("panel/panel.rs");
        write_face(
            &panel,
            "crate::root_object! {\n    kind: Panel,\n    needs_registry: true,\n    parent: crate::ROOT_NODE_ID,\n}",
        );
        let nodes = vec![Node {
            name: "panel".to_owned(),
            file: Some(panel),
            children: Vec::new(),
        }];
        let rendered = aggregate_registry_rule_errors(&root, &nodes, false, &whole_tree_scope())
            .render_build_diagnostics();
        assert!(rendered.contains("phase=face-rule"), "{rendered}");
        assert!(rendered.contains("source=panel/panel.rs:3"), "{rendered}");
        assert!(
            rendered.contains("expected=panel/registry_rule/registry_rule.rs"),
            "{rendered}"
        );
        assert!(
            rendered.contains("super::registry_rule::REGISTRATION_RULE"),
            "{rendered}"
        );
        // All three ways out are in the refusal: put the file back, name the rule in the
        // declaration, or stop owning a registry.
        // 三条出路都在拒绝文案里：把文件放回去、在声明里点名规则、或不再拥有注册机。
        for way_out in [
            "put the rule back at `panel/registry_rule/registry_rule.rs`",
            "`registry_rule: <path>::REGISTRATION_RULE`",
            "drop `needs_registry: true`",
        ] {
            assert!(rendered.contains(way_out), "{rendered}");
        }
        fs::remove_dir_all(root).expect("temporary fixture cleanup");
    }

    /// The other half, and the one that must not regress: a face that owns no registry needs no rule
    /// file (the maintainer's criterion — some faces really do not need one), a face that names its
    /// rule needs no sibling module, and a face that owns a registry **and** has its canonical rule
    /// passes. Refusing any of the three would be a false refusal.
    /// 另一半，也是绝不能回退的一半：不拥有注册机的面不需要规则文件（维护者的判据——有些面确实不需要），
    /// 点名了规则的面不需要兄弟模块，而拥有注册机**并且**带着规范规则的面通过。拒绝这三者中任何一个都是
    /// 假拒绝。
    #[test]
    fn accepts_faces_that_owe_no_unnamed_rule() {
        let root = temporary_directory("named-rule");
        let panel = root.join("panel/panel.rs");
        let rule = root.join("panel/registry_rule/registry_rule.rs");
        let leafy = root.join("panel/leafy/leafy.rs");
        let named = root.join("panel/named/named.rs");
        write_face(
            &panel,
            "crate::root_object! {\n    kind: Panel,\n    needs_registry: true,\n    parent: crate::ROOT_NODE_ID,\n}",
        );
        write_face(
            &rule,
            "use crate::RegistrationRule;\n\npub const REGISTRATION_RULE: RegistrationRule = RegistrationRule::new();\n",
        );
        write_face(
            &leafy,
            "crate::panel_object! {\n    kind: Leafy,\n    parent: crate::panel::NODE_ID,\n}",
        );
        write_face(
            &named,
            "crate::panel_object! {\n    kind: Named,\n    needs_registry: true,\n    parent: crate::panel::NODE_ID,\n    registry_rule: crate::RegistrationRule::ANY,\n}",
        );
        let nodes = vec![Node {
            name: "panel".to_owned(),
            file: Some(panel),
            children: vec![
                Node {
                    name: "registry_rule".to_owned(),
                    file: Some(rule),
                    children: Vec::new(),
                },
                Node {
                    name: "leafy".to_owned(),
                    file: Some(leafy),
                    children: Vec::new(),
                },
                Node {
                    name: "named".to_owned(),
                    file: Some(named),
                    children: Vec::new(),
                },
            ],
        }];
        let rendered = aggregate_registry_rule_errors(&root, &nodes, false, &whole_tree_scope())
            .render_build_diagnostics();
        assert!(rendered.is_empty(), "{rendered}");
        fs::remove_dir_all(root).expect("temporary fixture cleanup");
    }

    /// A face this build does not mount emits no reference, so its rule is nobody's problem here —
    /// the check judges what the render compiles, not every face on disk.
    /// 本次构建不挂载的面不会发射那条引用，因此它的规则在这里不归任何人管——本检查判的是渲染所编译的东西，
    /// 不是盘上的每一个面。
    #[test]
    fn says_nothing_about_a_face_the_build_does_not_mount() {
        let root = temporary_directory("unmounted-rule");
        let panel = root.join("panel/panel.rs");
        write_face(
            &panel,
            "crate::root_object! {\n    kind: Panel,\n    needs_registry: true,\n    parent: crate::ROOT_NODE_ID,\n}",
        );
        let nodes = vec![Node {
            name: "panel".to_owned(),
            file: Some(panel),
            children: Vec::new(),
        }];
        let selected_nothing = SourceScope {
            roots: Some(std::collections::BTreeSet::new()),
            reason: "test",
        };
        let rendered = aggregate_registry_rule_errors(&root, &nodes, false, &selected_nothing)
            .render_build_diagnostics();
        assert!(rendered.is_empty(), "{rendered}");
        fs::remove_dir_all(root).expect("temporary fixture cleanup");
    }

    /// A scope that selects nothing, so the build renders the whole tree.
    /// 什么都不筛选的范围，因此构建渲染整棵树。
    fn whole_tree_scope() -> SourceScope {
        SourceScope {
            roots: None,
            reason: "test",
        }
    }

    #[test]
    fn validates_all_three_parent_specific_declaration_levels() {
        let root = temporary_directory("parent-macros");
        let workspace = root.join("workspace/workspace.rs");
        let panel = root.join("workspace/object/panel/panel.rs");
        let child = root.join("workspace/object/panel/object/child/child.rs");
        write_face(
            &workspace,
            "crate::root_object! { kind: Workspace, parent: crate::ROOT_NODE_ID, }",
        );
        write_face(
            &panel,
            "crate::workspace_object! { kind: Panel, parent: crate::workspace::NODE_ID, }",
        );
        write_face(
            &child,
            "crate::panel_object! { kind: Child, parent: crate::workspace::object::panel::NODE_ID, }",
        );
        let nodes = vec![Node {
            name: "workspace".to_owned(),
            file: Some(workspace),
            children: vec![Node {
                name: "panel".to_owned(),
                file: Some(panel),
                children: vec![Node {
                    name: "child".to_owned(),
                    file: Some(child),
                    children: Vec::new(),
                }],
            }],
        }];
        assert!(aggregate_parent_macro_errors(&root, &nodes).is_empty());
        fs::remove_dir_all(root).expect("temporary fixture cleanup");
    }

    #[test]
    fn reports_the_expected_macro_for_a_wrong_parent_spelling() {
        let root = temporary_directory("wrong-parent-macro");
        let child = root.join("panel/object/child/child.rs");
        write_face(
            &child,
            "crate::wrong_object! { kind: Child, parent: crate::panel::NODE_ID, }",
        );
        let nodes = vec![Node {
            name: "child".to_owned(),
            file: Some(child),
            children: Vec::new(),
        }];
        let rendered = aggregate_parent_macro_errors(&root, &nodes).render_build_diagnostics();
        assert!(rendered.contains("phase=parent-macro"));
        assert!(rendered.contains("expected=crate::panel_object!"));
        assert!(rendered.contains("actual=crate::wrong_object!"));
        fs::remove_dir_all(root).expect("temporary fixture cleanup");
    }

    /// A parent spelled `NodeId::from_path(..)` resolves for the build but not
    /// for the runtime, so it must be refused rather than accepted and then
    /// reported as `<missing-parent>` after a successful build.
    /// 写成 `NodeId::from_path(..)` 的父级对构建期可解析、对运行期不可，因此必须拒绝，
    /// 而不是接受之后在构建成功时由注册机报 `<missing-parent>`。
    #[test]
    fn refuses_a_parent_that_the_runtime_cannot_resolve() {
        let root = temporary_directory("unnamespaced-parent");
        let child = root.join("panel/object/child/child.rs");
        write_face(
            &child,
            "crate::panel_object! { kind: Child, parent: crate::NodeId::from_path(\"panel/panel.rs\", \"Panel\"), }",
        );
        let nodes = vec![Node {
            name: "child".to_owned(),
            file: Some(child),
            children: Vec::new(),
        }];
        let rendered = aggregate_parent_macro_errors(&root, &nodes).render_build_diagnostics();
        assert!(
            rendered.contains("carries no namespace"),
            "unexpected diagnostics: {rendered}"
        );
        assert!(rendered.contains("crate::panel::NODE_ID"), "{rendered}");
        fs::remove_dir_all(root).expect("temporary fixture cleanup");
    }

    #[test]
    fn reports_a_missing_explicit_parent() {
        let root = temporary_directory("missing-parent");
        let child = root.join("panel/object/child/child.rs");
        write_face(&child, "crate::panel_object! { kind: Child, }");
        let nodes = vec![Node {
            name: "child".to_owned(),
            file: Some(child),
            children: Vec::new(),
        }];
        let rendered = aggregate_parent_macro_errors(&root, &nodes).render_build_diagnostics();
        assert!(
            rendered.contains("parent-specific registration macro requires an explicit parent")
        );
        assert!(rendered.contains("expected=parent: crate::panel::NODE_ID"));
        fs::remove_dir_all(root).expect("temporary fixture cleanup");
    }

    fn write_face(path: &std::path::Path, source: &str) {
        fs::create_dir_all(path.parent().expect("fixture parent"))
            .expect("temporary fixture directory");
        fs::write(path, source).expect("temporary registration face");
    }

    fn temporary_directory(label: &str) -> PathBuf {
        crate::build_method::registry_identity::freeze_test_namespace();
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock after Unix epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "xirang-build-{label}-{}-{stamp}",
            std::process::id()
        ));
        fs::create_dir_all(&path).expect("temporary fixture root");
        path
    }
}
