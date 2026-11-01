//! Continuation pins for the B4 `core/` findings `t26` left un-repaired.
//! t26 未修的 B4 `core/` 条目的接续钉子。
//!
//! `LGC-LG-06` (the overlay's non-`full` branch did not ask `needs_registry`),
//! `LGC-LG-07` (a sibling `registry_name` collision was nobody's business, and the graft-cut
//! path arm took the first match), and `LGC-LG-29` (a module-level `cfg` skipped the
//! whole module even when its feature was on). Each pin is asserted through the crate's
//! public surface, so it says what a consumer sees rather than what a private helper
//! happens to do — except the `cfg` one, which is a parser fixture by nature.
//! `LGC-LG-06`（overlay 的非 `full` 分支不问 `needs_registry`）、`LGC-LG-07`（兄弟 `registry_name` 碰撞
//! 无人过问，且切口路径那一支取第一个匹配）与 `LGC-LG-29`（模块级 `cfg` 即便特性开着也整棵跳过）。
//! 每条钉子都经 crate 的公开面断言，因此它说的是消费方看到的东西，而不是某个私有助手凑巧做了什么
//! ——只有 `cfg` 那条本质上是解析器夹具。

use xirang_kernel::declaration::{
    OwnedLocalizedText, OwnedObjectContract, OwnedSourceLocation, RegistrationSnapshot,
};
use xirang_kernel::release::StaticGraftCut;
use xirang_kernel::{
    Admission, FrameworkId, NodeId, OwnedFlowContract, RegistrationRule, Registry, root_node_id,
};

/// The framework every fixture tree in this file is built under.
/// 本文件每个夹具树所用的框架。
const FRAMEWORK: FrameworkId = FrameworkId::new("b4-continuation");

/// The one flow contract the fixtures declare, so both sides of a graft agree.
/// 夹具声明的唯一数据流合同，因此嫁接两侧相容。
fn flow() -> OwnedFlowContract {
    OwnedFlowContract {
        id: "render.v1".to_owned(),
        version: 1,
        input: "LocalCoordinates".to_owned(),
        output: "CanvasFrame".to_owned(),
    }
}

/// One minimal face, mounted under its namespace's root unless a caller moves it.
/// 一个最小注册面，默认挂在它命名空间的根下，除非调用方改它的父级。
fn face(namespace: &str, source: &str, kind: &str, slot: &str) -> RegistrationSnapshot {
    RegistrationSnapshot {
        namespace: namespace.to_owned(),
        id: NodeId::from_namespaced_path(namespace, source, kind),
        parent: root_node_id(namespace),
        kind: kind.to_owned(),
        preset: "NoPreset".to_owned(),
        parts: "NoParts".to_owned(),
        params: kind.to_owned(),
        handle: kind.to_owned(),
        stable_name: None,
        name: OwnedLocalizedText {
            zh: kind.to_owned(),
            en: kind.to_owned(),
        },
        summary: OwnedLocalizedText {
            zh: String::new(),
            en: String::new(),
        },
        exports: Vec::new(),
        needs_registry: false,
        registry_name: slot.to_owned(),
        getting_from_other_registry: None,
        registry_rule_path: "<test>".to_owned(),
        registry_rule: RegistrationRule::ANY.into_owned(),
        admission: Admission::ANY.into_owned(),
        requires: Vec::new(),
        provides: Vec::new(),
        contract: OwnedObjectContract {
            required_parts: Vec::new(),
            provided_parts: Vec::new(),
        },
        flow: flow(),
        flow_provider: None,
        handle_traits: Vec::new(),
        part_traits: Vec::new(),
        runtime_checks: Vec::new(),
        plugin: None,
        source: OwnedSourceLocation {
            file: source.to_owned(),
            line: 1,
            column: 1,
            function: kind.to_owned(),
        },
    }
}

/// A base tree with one owner that owns a registry, and one child inside it.
/// 一株基树：一个拥有注册机的 owner，以及它里面的一个子级。
fn base_with_a_live_child(
    namespace: &str,
) -> (Registry, RegistrationSnapshot, RegistrationSnapshot) {
    let mut base = Registry::root_for_namespace(FRAMEWORK, namespace);
    let mut owner = face(namespace, "owner.rs", "Owner", "owner");
    owner.needs_registry = true;
    base.register_snapshot_batch([owner.clone()])
        .expect("the owner registers");
    let mut child = face(namespace, "child.rs", "Child", "child");
    child.parent = owner.id;
    base.register_snapshot_batch([child.clone()])
        .expect("the child registers under the owner's registry");
    (base, owner, child)
}

/// `LGC-LG-06`: a non-`full` overlay may not leave a face that declares
/// `needs_registry: false` owning a registry that still holds faces.
/// `LGC-LG-06`：非 `full` 覆盖不得留下"声明 `needs_registry: false`、却仍拥有有子级的注册机"的面。
///
/// The in-place replacement path refuses exactly that state
/// (`graft_ops::replace_info`'s `(Some(child), false) if !child.entries.is_empty()` arm),
/// so the two paths answering one question have to give one answer.
/// 就地替换路径正是拒绝这一状态的（`graft_ops::replace_info` 的
/// `(Some(child), false) if !child.entries.is_empty()` 分支），因此两条路径回答同一个问题就该给出
/// 同一个答案。
#[test]
fn a_non_full_overlay_refuses_a_leaf_that_would_keep_a_live_child_registry() {
    let namespace = "lg06-live-child";
    let (base, owner, child) = base_with_a_live_child(namespace);
    assert!(
        base.registry(owner.id)
            .is_some_and(|registry| registry.find_registry(child.id).is_some()),
        "the fixture's owner really does own a live child registry"
    );

    let mut external = Registry::root_for_namespace(FRAMEWORK, namespace);
    let mut leaf = face(namespace, "leaf.rs", "Leaf", "owner");
    leaf.needs_registry = false;
    external
        .register_snapshot_batch([leaf])
        .expect("the replacement registers in its own tree");

    let cuts = [StaticGraftCut::new("root/owner", "owner", false)];
    let error = base
        .overlay_static(&cuts, &external)
        .expect_err("a face that declares no registry may not keep a non-empty one");
    let rendered = format!("{error}");
    assert!(
        rendered.contains("child registry"),
        "the refusal names what it would leave behind: {rendered}"
    );
    assert!(
        base.registry(owner.id)
            .is_some_and(|registry| registry.find_registry(child.id).is_some()),
        "the refusal leaves the base tree untouched"
    );
}

/// `LGC-LG-06` (the other half of the same rule): an *empty* child registry is not a
/// reason to refuse — it is dropped, exactly as the in-place path drops it.
/// `LGC-LG-06`（同一条规则的另一半）：**空**子注册机不是拒绝的理由——它被丢弃，与就地路径一致。
#[test]
fn a_non_full_overlay_drops_an_empty_child_registry_instead_of_refusing() {
    let namespace = "lg06-empty-child";
    let mut base = Registry::root_for_namespace(FRAMEWORK, namespace);
    let mut owner = face(namespace, "owner.rs", "Owner", "owner");
    owner.needs_registry = true;
    base.register_snapshot_batch([owner.clone()])
        .expect("the owner registers");
    assert!(
        base.registry(owner.id).is_some(),
        "the fixture's owner owns a registry, it is just empty"
    );

    let mut external = Registry::root_for_namespace(FRAMEWORK, namespace);
    let mut leaf = face(namespace, "leaf.rs", "Leaf", "owner");
    leaf.needs_registry = false;
    external
        .register_snapshot_batch([leaf])
        .expect("the replacement registers in its own tree");

    let overlaid = base
        .overlay_static(
            &[StaticGraftCut::new("root/owner", "owner", false)],
            &external,
        )
        .expect("an empty child registry is dropped, not refused");
    assert!(
        overlaid.registry(owner.id).is_none(),
        "the face now declares it owns no registry, and it owns none"
    );
}

/// `LGC-LG-07` ①: two siblings may not carry one `registry_name`. It is a path
/// segment, and three consumers used to answer "which face does this path name" by silently
/// picking one (the by-path index, `path_for`, and the cut selector).
/// `LGC-LG-07` ①：两个兄弟不得共用一个 `registry_name`。它是路径段，而三个消费方过去各自静默
/// 挑一个来回答"这条路径命名哪个面"（按路径的索引、`path_for`、以及切口选择器）。
#[test]
fn two_siblings_may_not_share_one_registry_name() {
    let namespace = "lg07-siblings";
    let mut root = Registry::root_for_namespace(FRAMEWORK, namespace);
    root.register_snapshot_batch([face(namespace, "first.rs", "First", "slot")])
        .expect("the first face registers");

    let error = root
        .register_snapshot_batch([face(namespace, "second.rs", "Second", "slot")])
        .expect_err("a second face claiming the same slot must be refused");
    let rendered = format!("{error}");
    assert!(
        rendered.contains("duplicate sibling registry name"),
        "the refusal names the collision: {rendered}"
    );
    assert!(
        rendered.contains("slot"),
        "the refusal names the slot: {rendered}"
    );

    let error = root
        .register_snapshot_batch([
            face(namespace, "third.rs", "Third", "other"),
            face(namespace, "fourth.rs", "Fourth", "other"),
        ])
        .expect_err("two faces in one batch may not claim one slot either");
    assert!(
        format!("{error}").contains("duplicate sibling registry name"),
        "the in-batch case is refused by the same rule"
    );

    root.register_snapshot_batch([face(namespace, "fifth.rs", "Fifth", "fifth")])
        .expect("a distinct slot still registers");
}

/// `LGC-LG-07` ②: a cut path two faces share is refused instead of the first match
/// winning, and the remedy the message points at — naming the target by identity — still
/// works.
/// `LGC-LG-07` ②：两个面共用的切口路径被拒绝，而不是让第一个匹配胜出；而消息指向的补救办法
/// ——用身份命名目标——仍然可用。
///
/// A collision stays reachable after registration refuses same-named siblings, because an
/// in-place edit may rename a face: that path checks namespace, identity, parent, rule
/// and contract — not the `registry_name`. That is why the selector arm had to be fixed rather
/// than relying on the registration rule alone.
/// 在注册拒绝兄弟同名之后这种碰撞仍然可达：就地编辑可以给面改名——那条路径检查命名空间、身份、
/// 父级、规则与契约，但不检查 `registry_name`。这正是选择器那一支必须自己修、而不能只靠注册规则的
/// 原因。
#[test]
fn a_cut_path_two_faces_share_is_refused_instead_of_taking_the_first_match() {
    let namespace = "lg07-ambiguous-path";
    let mut base = Registry::root_for_namespace(FRAMEWORK, namespace);
    let first = face(namespace, "first.rs", "First", "slot");
    let second = face(namespace, "second.rs", "Second", "other");
    base.register_snapshot_batch([first.clone(), second.clone()])
        .expect("two distinct slots register");

    let mut renamed = second.clone();
    renamed.registry_name = "slot".to_owned();
    base.apply_snapshot_replacement(second.id, renamed)
        .expect("an in-place rename is what makes two paths collide");

    let mut external = Registry::root_for_namespace(FRAMEWORK, namespace);
    let replacement = face(namespace, "replacement.rs", "Replacement", "replacement");
    external
        .register_snapshot_batch([replacement.clone()])
        .expect("the replacement registers in its own tree");

    let error = base
        .overlay_static(
            &[StaticGraftCut::new("root/slot", "replacement", false)],
            &external,
        )
        .expect_err("an ambiguous cut path must be refused");
    let rendered = format!("{error}");
    assert!(rendered.contains("matches 2"), "{rendered}");
    assert!(rendered.contains("root/slot"), "{rendered}");

    base.overlay_static(
        &[StaticGraftCut::from_ids(first.id, replacement.id, false)],
        &external,
    )
    .expect("naming the target by identity resolves it");
}

/// `LGC-LG-29`: a module-level `cfg` is read, not merely noticed. A gate the build can
/// evaluate travels with the cuts inside the module, so a feature-gated module keeps its
/// cuts instead of losing them silently; only a test-only expression skips the module.
/// `LGC-LG-29`：模块级 `cfg` 要被**读**，不能只被注意到。构建能求值的门控随模块内的切口一起
/// 下传，因此被特性门控的模块保住自己的切口、不再静默丢掉；只有"仅测试"的表达式才跳过整个模块。
#[cfg(feature = "syntax")]
#[test]
fn a_module_gate_is_read_not_merely_noticed() {
    let plan = "xirang_kernel::static_graft_plan!(FRAMEWORK, cut \"root/a\" graft \"fast\");";

    let gated = format!("#[cfg(feature = \"fast\")]\nmod fast {{\n    {plan}\n}}\n");
    let entries = xirang_kernel::syntax::graft_entries(&gated).expect("a gated module parses");
    assert_eq!(
        entries.len(),
        1,
        "a feature-gated module's cut must not be dropped: {entries:#?}"
    );
    assert_eq!(
        entries[0].cfg.as_deref(),
        Some("feature = \"fast\""),
        "the gate travels with the cut, for the build step to evaluate"
    );

    let negated = format!("#[cfg(not(test))]\nmod real {{\n    {plan}\n}}\n");
    assert_eq!(
        xirang_kernel::syntax::graft_entries(&negated)
            .expect("a not(test) module parses")
            .len(),
        1,
        "`not(test)` is not test-only, so the module is visited"
    );

    let tests = format!("#[cfg(test)]\nmod tests {{\n    {plan}\n}}\n");
    assert!(
        xirang_kernel::syntax::graft_entries(&tests)
            .expect("a test module parses")
            .is_empty(),
        "a test-only module still contributes nothing to a build plan"
    );

    let both = format!(
        "#[cfg(feature = \"fast\")]\nmod fast {{\n    #[cfg(feature = \"extra\")]\n    {plan}\n}}\n"
    );
    let entries = xirang_kernel::syntax::graft_entries(&both).expect("both gates parse");
    assert_eq!(
        entries[0].cfg.as_deref(),
        Some("all(feature = \"fast\", feature = \"extra\")"),
        "a declaration inherits the gates around it: {entries:#?}"
    );
}

/// 一个面不能把子面挂到一个不自持注册机的父面下：这样的批次过去会**静默**丢弃子面（提交成功、面数
/// 比交给它的少）。判据是"调用方交来的节点一个都不许无声消失"。
/// A face may not hang its child under a parent that owns no registry: such a batch used to drop the
/// child in silence (it committed, with one face fewer than it was given). The rule is that nothing
/// the caller handed over may disappear without being named.
#[test]
fn a_child_whose_parent_owns_no_registry_is_refused_by_name() {
    let namespace = "b4-no-registry-parent";
    let mut registry = Registry::root_for_namespace(FRAMEWORK, namespace);
    let parent = face(namespace, "panel/panel.rs", "Panel", "panel");
    assert!(
        !parent.needs_registry,
        "the fixture's parent owns no registry"
    );
    let mut child = face(namespace, "panel/area/area.cpp", "Area", "area");
    child.parent = parent.id;
    let before = registry.depth_first().len();
    let refusal = registry
        .register_snapshot_batch([parent, child])
        .expect_err("a child cannot attach to a parent that owns no registry");
    let text = refusal.to_string();
    assert!(text.contains("`Panel` owns no registry"), "{text}");
    assert!(text.contains("`Area`"), "{text}");
    assert!(
        text.contains("needs_registry"),
        "the refusal has to name the declaration that is missing: {text}"
    );
    assert_eq!(
        registry.depth_first().len(),
        before,
        "a refused batch writes nothing"
    );
}
