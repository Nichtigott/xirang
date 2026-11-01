//! Tests for the static overlay projection's keep/prune and replacement rules.
//! 静态覆盖投影的保留/剪枝与替换规则的测试。
//!
//! The projection decides which slots ship and which a declared cut replaces, so
//! a wrong verdict here is a wrong answer about the published tree — the one
//! thing `explain --overlay` and `xirang.explain {"overlay": true}` exist to
//! give. Each test below is a claim those surfaces make.
//! 投影决定哪些槽位会发布、哪些被已声明切口替换，因此这里的一个错误结论就是对发布树的错误回答
//! ——而这正是 `explain --overlay` 与 `xirang.explain {"overlay": true}` 存在的意义。下面每条
//! 测试都是这两个执行面做出的一个主张。

use std::path::PathBuf;

use xirang_kernel::identity::NodeId;

use super::*;

/// A face the build's discovery would have produced, with only the fields the
/// projection reads filled in from the caller.
/// 构建的发现过程会产出的一个面，只有投影读取的字段由调用方填入。
fn view(
    package: &str,
    source: &str,
    kind: &str,
    module: &str,
    registry_name: &str,
    parent: NodeId,
    path: &str,
) -> FaceView {
    FaceView {
        id: NodeId::from_namespaced_path(package, source, kind),
        stable_name: None,
        parent,
        kind: kind.to_owned(),
        registry_name: registry_name.to_owned(),
        module: module.to_owned(),
        source: source.to_owned(),
        source_base: crate::build_method::source_ref::SourceBase::Identity,
        owns_registry: true,
        path: path.to_owned(),
        parent_resolved: true,
    }
}

/// A scope that names exactly the given sources, modules and ids.
/// 正好点名给定源码、模块与身份的作用域。
fn scope(sources: &[&str], modules: &[&str], ids: &[NodeId], reason: &str) -> BuildScopeView {
    BuildScopeView {
        mode: "auto".to_owned(),
        all: false,
        reason: Some(reason.to_owned()),
        selected_sources: sources.iter().map(|value| (*value).to_owned()).collect(),
        selected_ids: ids.iter().copied().collect(),
        selected_modules: modules.iter().map(|value| (*value).to_owned()).collect(),
    }
}

/// A declaration naming one logical path.
/// 一条点名某个逻辑路径的声明。
fn cut(path: &str, graft: &str, line: usize) -> DeclaredGraft {
    DeclaredGraft {
        cut: path.to_owned(),
        cut_end: None,
        graft: graft.to_owned(),
        full: false,
        cfg: None,
        expressions: None,
        line,
    }
}

/// A package root with no plans directory: the ordinary "no plans" answer.
/// 一个没有计划目录的包根：普通的"没有计划"答案。
fn root(label: &str) -> PathBuf {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "xirang-overlay-rows-{label}-{}-{sequence}",
        std::process::id()
    ))
}

/// Two faces of one tree: a parent registry and the slot a cut replaces.
/// 一棵树的两个面：父注册机，以及被切口替换的槽位。
fn tree(package: &str) -> (NodeId, FaceView, FaceView) {
    let root_id = NodeId::from_namespaced_path(package, "<root>", "root");
    let parent = view(
        package,
        "control/control.rs",
        "Control",
        "control",
        "control",
        root_id,
        "root/control",
    );
    let child = view(
        package,
        "control/object/button/button.rs",
        "Button",
        "control::object::button",
        "button",
        parent.id,
        "root/control/object/button",
    );
    (root_id, parent, child)
}

/// A declared cut is the slot's replacement, and a parent the selected child
/// needs stays in the kept list — reporting it as pruned would misdescribe the
/// published tree.
/// 已声明切口就是该槽位的替换件，而被选中子级所需的父面留在保留清单里——把它报成被剪掉就是
/// 错误描述发布树。
#[test]
fn a_declared_cut_replaces_its_slot_and_a_needed_parent_is_kept() {
    let (_, parent, child) = tree("overlay");
    let scope = scope(
        &["control/object/button/button.rs"],
        &["control::object::button"],
        &[child.id],
        "declared-slot",
    );
    let declared = DeclaredGrafts {
        entry: PathBuf::from("src/lib.rs"),
        cuts: vec![cut("root/control/object/button", "button_fast", 6)],
    };

    let projection = overlay_projection(
        &root("kept"),
        &[parent.clone(), child.clone()],
        Some(&scope),
        Some(&declared),
    )
    .expect("the projection reads a missing plans directory as no plans");

    assert!(projection.pruned.is_empty(), "{projection:#?}");
    assert_eq!(projection.slots.len(), 2);
    let replaced = projection
        .slots
        .iter()
        .find(|slot| slot.id == child.id)
        .expect("the declared slot is a kept slot");
    assert_eq!(replaced.selected, Some(true));
    assert_eq!(replaced.kept, Some(true));
    assert_eq!(
        replaced.replacement.as_ref().map(|cut| cut.graft.as_str()),
        Some("button_fast")
    );
    let surviving = projection
        .slots
        .iter()
        .find(|slot| slot.id == parent.id)
        .expect("the needed parent is in the kept list");
    assert_eq!(surviving.selected, Some(false));
    assert_eq!(surviving.kept, Some(true));
    assert!(surviving.replacement.is_none());
}

/// No published scope means every verdict is `None`: "unknown" is not "pruned",
/// and a projection that guessed would tell an operator a live face is gone.
/// 没有发布作用域意味着每个结论都是 `None`：未知不是"被剪掉"，而靠猜的投影会告诉操作者一个
/// 仍然存活的面消失了。
#[test]
fn an_unknown_scope_keeps_every_slot_and_prunes_nothing() {
    let (_, parent, child) = tree("overlay-unknown");
    let declared = DeclaredGrafts {
        entry: PathBuf::from("src/lib.rs"),
        cuts: vec![cut("root/control/object/button", "button_fast", 6)],
    };

    let projection = overlay_projection(&root("unknown"), &[parent, child], None, Some(&declared))
        .expect("no plans is not a failure");

    assert!(projection.pruned.is_empty(), "{projection:#?}");
    assert_eq!(projection.slots.len(), 2);
    assert!(
        projection
            .slots
            .iter()
            .all(|slot| slot.selected.is_none() && slot.kept.is_none()),
        "an unknown scope must not invent a verdict: {projection:#?}"
    );
    assert_eq!(
        projection
            .slots
            .iter()
            .filter(|slot| slot.replacement.is_some())
            .count(),
        1,
        "a declared cut is still nameable without a published scope"
    );
}

/// The parent-keep boundary is the `::` segment, not a text prefix: selecting
/// `control::object::button` keeps the `control` registry and never
/// `control_extra`.
/// 父级保留的边界是 `::` 段而不是文本前缀：选中 `control::object::button` 会保住
/// `control` 注册机，绝不保住 `control_extra`。
#[test]
fn a_selected_subtree_keeps_its_parent_by_module_segment() {
    let (root_id, parent, child) = tree("overlay-segment");
    let lookalike = view(
        "overlay-segment",
        "control_extra/control_extra.rs",
        "ControlExtra",
        "control_extra",
        "control_extra",
        root_id,
        "root/control_extra",
    );
    let scope = scope(
        &["control/object/button/button.rs"],
        &["control::object::button"],
        &[child.id],
        "declared-slot",
    );

    let projection = overlay_projection(
        &root("segment"),
        &[parent, child, lookalike.clone()],
        Some(&scope),
        None,
    )
    .expect("no plans is not a failure");

    assert!(
        projection.slots.iter().any(|slot| slot.module == "control"),
        "the parent registry stays: {projection:#?}"
    );
    assert!(
        projection.pruned.iter().any(|slot| slot.id == lookalike.id),
        "a module that merely begins with `control` is outside the scope: {projection:#?}"
    );
}
