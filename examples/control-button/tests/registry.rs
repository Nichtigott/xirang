//! 集成测试：把 README 那棵 Control / Button 树当成真实宿主，逐项验证构建期
//! 生成、注册、父级规则、graft、项目外实现和发布态静态计划。
//! Integration tests: treat the README Control/Button tree as a real host and
//! verify the generated plan, registration, the parent rule, grafting, the
//! out-of-project implementation, and the release-time static plan.

use control_button::{FRAMEWORK, base_registry, builtin_static_plan};
use control_button_graft::FRAMEWORK as GRAFT_FRAMEWORK;
use xirang_toolchain::run_method::registry_core::lexicon;
use xirang_toolchain::run_method::registry_core::{
    FrameworkId, GraftPlan, NodeId, OwnedFlowContract, PluginManifest, PluginMode, PluginSource,
    PluginTrustError, PluginTrustPolicy, Registry, StaticGraftCut,
};
use xirang_toolchain::run_method::{
    GraftPlanDocument, RecordReport, apply_recorded_grafts, graft_record_root,
};

/// 读取某一逻辑路径上的 kind；找不到就失败。
/// Read the kind at a logical path, failing when the path is absent.
fn kind_at(registry: &Registry, wanted: &str) -> String {
    registry
        .depth_first()
        .iter()
        .find(|info| registry.path_for(info.id).as_deref() == Some(wanted))
        .map(|info| info.kind.clone())
        .unwrap_or_else(|| panic!("no registration face at `{wanted}`"))
}

fn slot_plan() -> GraftPlan {
    GraftPlan::command(FRAMEWORK, "cut root/control/button graft button_fast")
        .expect("the graft command parses")
}

/// `source` 现在由声明点推导，但必须与旧方案注入的值完全相同。
/// `source` is derived at the declaration site now, and must equal what the
/// previous design injected.
#[test]
fn built_in_tree_has_the_expected_paths_and_derived_sources() {
    let registry = base_registry();
    let mut rows = registry
        .depth_first()
        .iter()
        .map(|info| {
            format!(
                "{} kind={} source={}",
                registry.path_for(info.id).unwrap_or_default(),
                info.kind,
                info.source.portable_file()
            )
        })
        .collect::<Vec<_>>();
    rows.sort();

    assert_eq!(
        rows,
        [
            "root/control kind=Control source=control/control.rs",
            "root/control/button kind=Button source=control/object/button/button.rs",
            "root/control/slider kind=Slider source=control/object/slider/slider.rs",
        ]
    );
}

/// 自定义 `handle` 且省略 `preset`/`parts` 的面必须记录朴素的默认名，而不是展开后的
/// `"$crate :: NoPreset"` 字面 token；三个内置面都是这个形态，因此这条断言钉住的是实际
/// 出厂的那条路径，而不只是单元测试里的形状。
/// A face with a custom `handle` and an omitted `preset`/`parts` must record the
/// plain default names, not the expanded `"$crate :: NoPreset"` literal tokens.
/// All three built-in faces take that shape, so this pins the shipped path, not
/// only a shape in a unit test.
#[test]
fn handle_faces_record_plain_default_preset_and_parts_names() {
    // The snapshot the tree assembles is what a consumer reads now: a face written as a `Registration`
    // literal carries its declaration on its own type, and the runtime record is the build's half.
    // 现在消费者读的是**生成树拼装出来的快照**：写成 `Registration` 字面量的面把声明放在自己的类型上，而运行期
    // 记录是构建那一半。
    let recorded = |kind: &str| {
        control_button::registrations()
            .into_iter()
            .find(|row| row.kind == kind)
            .unwrap_or_else(|| panic!("no registration record for `{kind}`"))
    };
    for kind in ["Control", "Button", "Slider"] {
        let row = recorded(kind);
        assert_eq!(row.preset, "NoPreset", "{kind}");
        assert_eq!(row.parts, "NoParts", "{kind}");
    }
}

/// 叶子面以自身名字载入，公开模块路径和类型名都不变。
/// A leaf face loads under its own name, so its public module path and type
/// name are unchanged.
#[test]
fn leaf_face_module_path_stays_clean() {
    assert_eq!(
        std::any::type_name::<control_button::control::object::button::Button>(),
        "control_button::control::object::button::Button"
    );
}

/// 文件夹面的条目仍按原有路径可寻址（同名子模块 + 重导出）。
/// A folder face's items stay addressable at their original path through the
/// same-named child module and its re-export.
#[test]
fn folder_face_items_stay_reachable() {
    let _: NodeId = control_button::control::NODE_ID;
    let _ = std::marker::PhantomData::<control_button::control::Control>;
    let _ = control_button::control::ControlFrame;
}

/// 面文件以 `//!` 开头（见 `src/control/control.rs`）仍然编译，并且源码路径
/// 就是仓库相对路径，而不是任何构建副本。
/// Face files keep their `//!` header (see `src/control/control.rs`), and the
/// recorded source is the repository-relative path rather than a build copy.
#[test]
fn face_sources_point_at_the_real_files() {
    let registry = base_registry();
    let sources = registry
        .depth_first()
        .iter()
        .map(|info| info.source.portable_file())
        .collect::<Vec<String>>();
    assert!(sources.iter().any(|source| source == "control/control.rs"));
    assert!(
        sources
            .iter()
            .any(|source| source == "control/object/button/button.rs")
    );
    assert!(
        !sources
            .iter()
            .any(|source| source.contains("registration_sources")),
        "no materialised copy may appear in an identity: {sources:?}"
    );
}

/// 父级规则是最低结构要求：缺少必需 export 的子对象整批拒绝。
/// The parent rule is a minimum shape: a child missing a required export is
/// rejected as a batch.
#[test]
fn parent_rule_rejects_a_child_that_misses_a_required_export() {
    let mut registry = Registry::root_for_namespace(FRAMEWORK, env!("CARGO_PKG_NAME"));
    registry
        .register_all(&control_button::registrations())
        .expect("built-in faces register");

    let mut broken = control_button::registrations()
        .into_iter()
        .find(|row| row.kind == "Button")
        .expect("the button face records a registration")
        .into_snapshot();
    broken.kind = "BrokenButton".to_owned();
    broken.exports.clear();
    broken.id =
        NodeId::from_namespaced_path(&broken.namespace, &broken.source.file, "BrokenButton");
    // A slot name of its own, so the fixture exercises the *parent rule* rather than the
    // sibling slot-name rule: the broken copy used to keep `button`, which the kernel now
    // refuses before the rule is ever consulted (audit `LGC-LG-07`). The face is broken in
    // exactly the way this test is about — its exports — and nowhere else.
    // 给坏副本一个自己的槽位名，于是这条夹具考的是**父级规则**而不是兄弟槽位名规则：它过去沿用
    // `button`，而内核现在会在规则被咨询之前就拒绝它（审计 `LGC-LG-07`）。这个面坏掉的正是本
    // 测试要考的那一处——exports——别处一律不变。
    broken.registry_name = "broken-button".to_owned();

    let error = registry
        .register_snapshot_batch([broken])
        .expect_err("the parent rule rejects a child without `control.render`");
    let rendered = error.to_string();
    assert!(
        rendered.contains("control.render"),
        "the diagnostic must name the missing requirement: {rendered}"
    );
}

/// 父级规则不筛选 kind：满足结构的其它 kind 可以进入同一个 Registry。
/// The parent rule is not a kind filter: any shape that satisfies it may enter.
#[test]
fn parent_rule_admits_a_new_kind_that_satisfies_it() {
    let mut registry = base_registry();
    let mut sibling = control_button::registrations()
        .into_iter()
        .find(|row| row.kind == "Button")
        .expect("the button face records a registration")
        .into_snapshot();
    sibling.kind = "Toggle".to_owned();
    sibling.registry_name = "toggle".to_owned();
    sibling.id = NodeId::from_namespaced_path(&sibling.namespace, &sibling.source.file, "Toggle");

    registry
        .register_snapshot_batch([sibling])
        .expect("structure, not kind, decides admission");
    assert_eq!(kind_at(&registry, "root/control/toggle"), "Toggle");
}

/// 宿主与项目外 crate 必须共享同一个 framework，overlay 才接受外部树。
/// Host and out-of-project crate must share one framework for `overlay`.
#[test]
fn host_and_external_share_one_framework() {
    assert_eq!(FRAMEWORK, GRAFT_FRAMEWORK);
}

/// 项目外面由 `external_object!` 显式声明来源，不参与宿主的生成树。
/// An out-of-project face declares its origin explicitly and takes no part in
/// the host's generated tree.
#[test]
fn out_of_project_face_declares_its_own_source() {
    let external = control_button_graft::external_registry();
    let faces = external.depth_first();
    let face = faces
        .iter()
        .find(|info| info.kind == "ButtonFast")
        .expect("the external face registers into its own registry");

    assert_eq!(face.registry_name, "button_fast");
    assert_eq!(face.source.file, "button_fast/button_fast.rs");
    assert_eq!(face.namespace, "xirang-example-control-button-graft");
    assert_eq!(
        external.path_for(face.id).as_deref(),
        Some("root/button_fast")
    );
}

/// graft 替换逻辑槽位，原树和外部树都不被修改。
/// Grafting replaces the logical slot; neither tree is modified.
#[test]
fn graft_replaces_the_slot_and_leaves_both_trees_untouched() {
    let base = base_registry();
    let external = control_button_graft::external_registry();

    let effective = base
        .overlay(&slot_plan(), &external)
        .expect("the overlay publishes");

    assert_eq!(kind_at(&effective, "root/control/button"), "ButtonFast");
    assert_eq!(kind_at(&base, "root/control/button"), "Button");
    assert_eq!(kind_at(&external, "root/button_fast"), "ButtonFast");
    // The slot keeps its logical path; only the implementation changed.
    // 槽位保留逻辑路径，换掉的只是实现。
    assert_eq!(
        kind_at(&effective, "root/control"),
        "Control",
        "the parent face survives the replacement"
    );
}

/// 数据流合同不兼容时，整组计划不发布。
/// When the flow contract is incompatible, nothing is published.
#[test]
fn graft_rejects_an_incompatible_flow_contract() {
    let base = base_registry();
    let external = external_registry_with_flow("control.render.v1", 2);

    let error = base
        .overlay(&slot_plan(), &external)
        .expect_err("a different flow version must be refused");
    let rendered = error.to_string();
    assert!(
        rendered.contains("control.render.v1"),
        "the diagnostic must name the contract: {rendered}"
    );
    assert_eq!(kind_at(&base, "root/control/button"), "Button");
}

/// framework 不同时，外部树不会被接受。
/// A foreign framework is never accepted.
#[test]
fn graft_rejects_a_foreign_framework() {
    let base = base_registry();
    let plan = GraftPlan::command(
        FrameworkId::new("other.framework"),
        "cut root/control/button graft button_fast",
    )
    .expect("the graft command parses");

    let external = control_button_graft::external_registry();
    assert!(
        base.overlay(&plan, &external).is_err(),
        "overlay must reject a tree from another framework"
    );
}

/// 入口声明的 graft 计划被固化成发布态静态表。
/// The plan declared at the entry is frozen into the release-time static table.
#[test]
fn static_plan_carries_faces_and_the_declared_graft() {
    let plan = builtin_static_plan();

    assert_eq!(plan.faces().len(), 3, "every declared face is retained");
    assert_eq!(
        plan.grafts().len(),
        2,
        "the entry declares both replaceable slots"
    );
    let cut = plan.grafts()[0];
    assert_eq!(
        cut.cut().id(),
        Some(control_button::control::object::button::NODE_ID),
        "the typed cut names the host face by compile-time identity"
    );
    assert_eq!(
        cut.graft().id(),
        Some(control_button_graft::button_fast::NODE_ID),
        "the typed graft names the external face by compile-time identity"
    );
    assert!(!cut.is_full(), "a plain cut keeps the target's children");
    assert_eq!(
        plan.grafts()[1].cut().id(),
        Some(control_button::control::object::slider::NODE_ID),
        "the sibling slot is declared too, not kept alive by accident"
    );
    assert_eq!(
        plan.grafts()[1].graft().id(),
        Some(control_button_graft::slider_fast::NODE_ID),
        "each declared slot names the implementation that replaces it"
    );
}

/// 创作查询看到宿主声明的槽位，且与构建捕获到的是同一批切口。
/// The authoring query sees the slots this host declares, and they are the same
/// cuts the build captured — typed form included, through the same module
/// mapping the scope uses.
#[test]
fn the_authoring_query_sees_the_declared_typed_slots() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let declared = xirang_toolchain::build_method::declared_grafts(&root).expect("the entry reads");
    assert!(
        declared.entry.ends_with("src/lib.rs"),
        "the file calling host!() is the entry: {}",
        declared.entry.display()
    );
    assert_eq!(declared.cuts.len(), 2);
    let typed = |index: usize| {
        declared.cuts[index]
            .expressions
            .as_ref()
            .unwrap_or_else(|| panic!("cut {index} is written in the typed form"))
    };
    assert_eq!(
        typed(0).cut,
        "crate::control::object::button::NODE_ID",
        "the authoring view keeps the expression that names the face"
    );
    assert_eq!(typed(1).cut, "crate::control::object::slider::NODE_ID");
    assert!(
        declared.cuts.iter().all(|cut| !cut.full),
        "both example slots are plain cuts"
    );
    assert!(
        declared
            .cuts
            .iter()
            .all(|cut| cut.graft.contains("control_button_graft::")),
        "the authoring view keeps the external expression too"
    );
}

/// 构建期作用域收窄到宿主声明的槽位：切口命名的子树活着，没有声明的面不发布。
/// The build-time scope narrows to the slots the host declared: the subtrees the
/// cuts name stay live, and a face nobody declared is not shipped.
#[test]
fn the_declared_slots_define_the_build_method_scope() {
    let scope = std::fs::read_to_string(concat!(env!("OUT_DIR"), "/source_scope.tsv"))
        .expect("the build step publishes its source scope");
    let selected = scope
        .lines()
        .filter(|line| !line.starts_with('#') && !line.is_empty())
        .collect::<Vec<_>>();

    assert!(
        scope.starts_with("# mode\tauto\n"),
        "the scope is derived from the entry, not pinned by the environment: {scope}"
    );
    assert_eq!(
        selected.len(),
        2,
        "the entry declares exactly two slots: {scope}"
    );
    assert!(
        selected
            .iter()
            .any(|row| row.ends_with("control/object/button/button.rs\tcontrol::object::button")),
        "the button slot stays live: {scope}"
    );
    assert!(
        selected
            .iter()
            .any(|row| row.ends_with("control/object/slider/slider.rs\tcontrol::object::slider")),
        "the slider slot stays live: {scope}"
    );
    // The parent `control` face is not a slot, so it is not a scope root; it
    // survives because a selected face needs it and because the entry reaches
    // it. That is the point of the narrow scope: it records what the
    // declarations prove, not a whole tree left intact by a fallback.
    // 父级 `control` 不是槽位，因此不是作用域根；它活着是因为被选中的面需要它、
    // 而且入口能到达它。这正是收窄的意义：作用域记录的是声明证明的东西，而不是
    // 回退保留下来的整棵树。
}

/// 发布路径用静态选择器 overlay，不需要构造动态计划。
/// The release path overlays straight from the static selectors.
#[test]
fn release_path_overlays_the_declared_static_graft() {
    let base = base_registry();
    let external = control_button_graft::external_registry();

    let effective = base
        .overlay_static(builtin_static_plan().grafts(), &external)
        .expect("the static overlay publishes");

    assert_eq!(kind_at(&effective, "root/control/button"), "ButtonFast");
    assert_eq!(kind_at(&base, "root/control/button"), "Button");
}

/// 外部面带上一个不同的合同版本，用来验证 overlay 的合同比较。
/// An external face carrying a different contract version, used to exercise the
/// overlay's contract comparison.
fn external_registry_with_flow(id: &str, version: u32) -> Registry {
    let mut snapshot = control_button_graft::button_fast::REGISTRATION.into_snapshot();
    snapshot.flow = OwnedFlowContract {
        id: id.to_owned(),
        version,
        input: "ControlInput".to_owned(),
        output: "ControlFrame".to_owned(),
    };
    let namespace = snapshot.namespace.clone();
    let mut registry = Registry::root_for_namespace(FRAMEWORK, namespace);
    registry
        .register_snapshot_batch([snapshot])
        .expect("the modified external face registers");
    registry
}

// ---------------------------------------------------------------------------
// Replacement granularity. The registry model addresses faces, and a plan can
// replace one face, a contiguous range of siblings, a whole subtree, or several
// boundaries at once.
// 替换粒度。注册模型以注册面为寻址单位，一份计划可以替换单个面、一段连续兄弟、
// 整棵子树，或一次改动多个边界。
// ---------------------------------------------------------------------------

/// One face: only the slot changes, its siblings and parent survive.
/// 单个面：只换槽位，兄弟与父级保持不变。
#[test]
fn graft_replaces_one_face_only() {
    let base = base_registry();
    let external = control_button_graft::external_registry();

    let effective = base
        .overlay(&slot_plan(), &external)
        .expect("the overlay publishes");

    assert_eq!(kind_at(&effective, "root/control/button"), "ButtonFast");
    assert_eq!(kind_at(&effective, "root/control/slider"), "Slider");
}

/// A contiguous sibling range: both ends of the range take the replacement,
/// and the parent and any sibling outside the range survive.
/// 连续兄弟区间：区间两端都被替换，父级与区间外的兄弟保持不变。
#[test]
fn graft_replaces_a_contiguous_sibling_range() {
    let base = base_registry();
    let external = control_button_graft::external_registry();
    let plan = GraftPlan::command(
        FRAMEWORK,
        "cut [root/control/button to root/control/slider] graft slider_fast",
    )
    .expect("the range command parses");

    let effective = base
        .overlay(&plan, &external)
        .expect("the overlay publishes");

    assert_eq!(kind_at(&effective, "root/control/button"), "SliderFast");
    assert_eq!(kind_at(&effective, "root/control/slider"), "SliderFast");
    assert_eq!(kind_at(&effective, "root/control"), "Control");
    // The base tree is untouched, so the original faces are still there.
    // 原树未被修改，原始注册面仍然存在。
    assert_eq!(kind_at(&base, "root/control/button"), "Button");
}

/// A whole subtree: `full` discards the original children and adopts the
/// replacement's own subtree.
/// 整棵子树：`full` 丢弃原子节点，改用替换实现自带的子树。
#[test]
fn graft_replaces_a_whole_subtree_with_full() {
    let base = base_registry();
    let external = control_button_graft::external_registry();
    let plan = GraftPlan::command(FRAMEWORK, "cut root/control full graft control_fast")
        .expect("the full-cut command parses");

    let effective = base
        .overlay(&plan, &external)
        .expect("the overlay publishes");

    assert_eq!(kind_at(&effective, "root/control"), "ControlFast");
    // The original children belonged to the discarded subtree.
    // 原来的子节点属于被丢弃的子树。
    assert!(
        effective
            .depth_first()
            .iter()
            .all(|info| info.kind != "Button" && info.kind != "Slider"),
        "a full cut must not keep the replaced subtree"
    );
    // Without `full` the same cut would have inherited them.
    // 不加 `full` 时同一条切口会继承它们。
    let inherited = base
        .overlay(
            &GraftPlan::command(FRAMEWORK, "cut root/control graft control_fast")
                .expect("the plain command parses"),
            &external,
        )
        .expect("the overlay publishes");
    assert_eq!(kind_at(&inherited, "root/control/button"), "Button");
}

/// A chain: one plan changes several data boundaries together, and publishes
/// all of them or none.
/// 链条：一份计划同时改动多个数据边界，要么全部发布、要么一个都不发布。
#[test]
fn graft_applies_a_chain_of_cuts_in_one_plan() {
    let base = base_registry();
    let external = control_button_graft::external_registry();
    let mut plan = GraftPlan::new(FRAMEWORK);
    plan.push("root/control/button", "button_fast");
    plan.push("root/control/slider", "slider_fast");

    let effective = base
        .overlay(&plan, &external)
        .expect("the overlay publishes");

    assert_eq!(kind_at(&effective, "root/control/button"), "ButtonFast");
    assert_eq!(kind_at(&effective, "root/control/slider"), "SliderFast");
    assert_eq!(kind_at(&base, "root/control/button"), "Button");
}

/// The typed range form reaches the same result without any selector string.
/// 类型化区间形式在完全不使用选择器字符串的情况下得到同一结果。
#[test]
fn typed_range_cut_replaces_both_siblings() {
    let base = base_registry();
    let external = control_button_graft::external_registry();
    let cuts = [StaticGraftCut::from_id_range(
        control_button::control::object::button::NODE_ID,
        control_button::control::object::slider::NODE_ID,
        control_button_graft::slider_fast::NODE_ID,
        false,
    )];

    let effective = base
        .overlay_static(&cuts, &external)
        .expect("the typed range overlay publishes");

    assert_eq!(kind_at(&effective, "root/control/button"), "SliderFast");
    assert_eq!(kind_at(&effective, "root/control/slider"), "SliderFast");
}

/// A chain fails as a whole: one unresolvable cut rejects the plan and leaves
/// the base tree published unchanged.
/// 链条整体失败：一条无法解析的切口就让整份计划被拒，原树保持不变。
#[test]
fn graft_chain_rejects_atomically() {
    let base = base_registry();
    let external = control_button_graft::external_registry();
    let mut plan = GraftPlan::new(FRAMEWORK);
    plan.push("root/control/button", "button_fast");
    plan.push("root/control/missing", "slider_fast");

    assert!(
        base.overlay(&plan, &external).is_err(),
        "an unresolvable cut must reject the whole plan"
    );
    assert_eq!(kind_at(&base, "root/control/button"), "Button");
}

// ---------------------------------------------------------------------------
// Plugins. A plugin is the dynamically loaded form of the same thing a graft
// names statically, so its bytes must clear the digest and trust policy before
// they can reach the overlay.
// 插件。插件是 graft 静态命名的同一件事的动态加载形式，因此它的字节必须先通过
// 摘要与信任策略，才能走到 overlay。
// ---------------------------------------------------------------------------

/// A plugin whose bytes match its manifest is accepted; tampered bytes are
/// rejected before any registry work happens.
/// 字节与清单一致的插件被接受；被篡改的字节在任何注册工作之前就被拒绝。
#[test]
fn plugin_bytes_must_verify_before_they_can_replace_a_face() {
    let payload = b"button_fast plugin payload";
    // The manifest carries a `&'static str`, so the test leaks its computed
    // digest the same way a build-time constant would already hold one.
    // 清单持有 `&'static str`，因此测试把算出的摘要泄漏成静态串——构建期常量本来
    // 就是静态的。
    let checksum = Box::leak(
        format!(
            "sha256:{}",
            xirang_toolchain::run_method::registry_core::sha256_hex(payload)
        )
        .into_boxed_str(),
    );
    let manifest = PluginManifest {
        name: "button_fast",
        crate_name: "control_button_graft",
        version: "0.1.0",
        framework: FRAMEWORK,
        source: PluginSource::User,
        mode: PluginMode::Replacement,
        checksum,
        signature: None,
        public_key_fingerprint: None,
        revocation_list: None,
    };

    // Provenance is checked separately from structure and flow.
    // 来源校验与结构、数据流校验刻意分开。
    assert!(manifest.targets(FRAMEWORK));
    assert!(!manifest.targets(FrameworkId::new("other.framework")));

    let policy = PluginTrustPolicy::open();
    policy
        .verify(manifest, payload, None)
        .expect("matching bytes pass the open policy");
    assert_eq!(
        policy.verify(manifest, b"tampered", None),
        Err(PluginTrustError::DigestMismatch),
        "tampered bytes must be rejected before they reach the registry"
    );

    // Bytes that do pass still have to satisfy the destination rule and flow
    // contract; the graft tests above exercise that through the same overlay.
    // 通过校验的字节仍须满足目标规则与数据流合同；上面的 graft 测试已用同一套
    // overlay 覆盖了这一步。
}

// ---------------------------------------------------------------------------
// Studio. The TUI cannot run headless here, so drive the entry point its `g`
// key calls and check the artifact it leaves behind.
// Studio。这里无法无头运行 TUI，因此直接驱动 `g` 键调用的入口，并检查它留下的
// 产物。
// ---------------------------------------------------------------------------

/// Studio's graft flow writes an external plan and never edits host source.
/// Studio 的 graft 流程写下外部计划，绝不改动宿主源码。
#[test]
fn studio_graft_flow_writes_a_plan_without_touching_host_source() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    // SAFETY: the authoring root is process-global and only this test reads it.
    // 安全：创作根是进程级的，且只有本测试读取它。
    unsafe { std::env::set_var("XIRANG_PACKAGE_ROOT", &root) };

    let registry = base_registry();
    let target = control_button::control::object::button::NODE_ID;
    let plan = xirang_toolchain::run_method::create_external_graft(
        &registry,
        target,
        "button_graft",
        false,
    )
    .expect("the studio graft flow creates a plan");

    let plan_path = plan.plan_path();
    let text = std::fs::read_to_string(&plan_path).expect("read the created plan");
    assert!(
        text.contains("target_path=root/control/button"),
        "the plan addresses the logical slot: {text}"
    );
    assert!(text.contains("graft=button_graft"), "{text}");
    assert!(text.contains("full=false"), "{text}");

    // Host source is untouched: the selector lives in `.xirang`, not in `src/`.
    // 宿主源码未被改动：选择器只存在于 `.xirang`，不在 `src/`。
    let face = std::fs::read_to_string(root.join("src/control/object/button/button.rs"))
        .expect("read the face");
    assert!(
        !face.contains("button_graft"),
        "studio must not rewrite host source"
    );

    // Only the plan this test created is removed — and the directory that held it only when it is
    // then empty. The whole `.xirang` tree also holds an adoption ledger, which this test never
    // created and must not delete: two independent evaluation arms measured exactly that, and both
    // had to work around it by running the gate first and writing the ledger afterwards.
    // 只删本测试创建的那份计划——而它所在的目录只在因此为空时才收。整棵 `.xirang` 里还住着采信台账，
    // 本测试从未创建它、也绝不该删掉它：两个独立的评测臂都量到这一点，两边都只能靠"先跑门、再落台账"绕开。
    let _ = std::fs::remove_file(&plan_path);
    if let Some(directory) = plan_path.parent() {
        let _ = std::fs::remove_dir(directory);
    }
}

// ---------------------------------------------------------------------------
// Records. A `.xirang` record is runtime input: it can move the effective
// tree away from `overlay_static`, while the build-captured typed plan stays
// exactly what the host declared.
// 记录。`.xirang` 记录是运行期输入：它能让有效树偏离 `overlay_static`，而构建
// 捕获的类型化计划仍与宿主声明的一模一样。
// ---------------------------------------------------------------------------

/// 一条记录覆盖**字符串**声明时会改变有效树，但绝不会改写发布态静态计划；同一条记录
/// 覆盖本示例的**类型化**声明时被忽略，因为类型化声明是最终裁决。
/// A record over a **string** declaration changes the effective tree but never
/// rewrites the release-time static plan; the same record over this example's
/// **typed** declaration is ignored, because a typed declaration is final.
#[test]
fn a_record_moves_the_effective_tree_but_not_the_static_plan() {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "xirang-control-button-record-{}-{sequence}",
        std::process::id()
    ));
    // The directory is the record's selector and must equal the document's
    // `graft`; naming it after the slot instead is now refused.
    // 目录就是记录的选择器，必须等于文档里的 `graft`；按槽位命名会被拒绝。
    let record_directory = graft_record_root(&root).join("slider_fast");
    std::fs::create_dir_all(&record_directory).expect("create the record directory");
    let document = GraftPlanDocument::new(
        control_button::control::object::button::NODE_ID,
        "root/control/button",
        "slider_fast",
        false,
    );
    std::fs::write(
        record_directory.join(lexicon::GRAFT_PLAN_FILE),
        document.render_graft_plan_document(),
    )
    .expect("write the record");

    let base = base_registry();
    let external = control_button_graft::external_registry();

    // The shipped declaration is typed, so the record is reported and ignored.
    // 出厂声明是类型化的，因此记录被报告并忽略。
    let typed = apply_recorded_grafts(&base, &external, builtin_static_plan().grafts(), &root)
        .expect("the typed declaration still publishes");
    assert_eq!(
        kind_at(&typed.effective, "root/control/button"),
        "ButtonFast"
    );
    assert!(typed.reports.iter().any(|report| matches!(
        report,
        RecordReport::TypedDeclarationKept { recorded, .. } if recorded == "slider_fast"
    )));

    // The same record over a string declaration wins, and the effective tree
    // diverges from `overlay_static` on purpose.
    // 同一条记录覆盖字符串声明时获胜，有效树因此有意偏离 `overlay_static`。
    let string_declared = [StaticGraftCut::new(
        "root/control/button",
        "button_fast",
        false,
    )];
    let recorded = apply_recorded_grafts(&base, &external, &string_declared, &root)
        .expect("the string declaration yields");
    assert_eq!(
        kind_at(&recorded.effective, "root/control/button"),
        "SliderFast"
    );

    // Neither the base tree nor the build-captured plan moved.
    // 原树与构建捕获的计划都没有动。
    assert_eq!(kind_at(&base, "root/control/button"), "Button");
    assert_eq!(
        builtin_static_plan().grafts()[0].graft().id(),
        Some(control_button_graft::button_fast::NODE_ID),
        "the static plan keeps the typed graft the host declared"
    );

    let _ = std::fs::remove_dir_all(&root);
}
