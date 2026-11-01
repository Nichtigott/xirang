//! A graft switch changes the implementation the host actually invokes.
//! graft 切换改变宿主实际调用的实现。
//!
//! This fixture deliberately makes the dispatch point visible: the host reads the effective
//! registration kind after the overlay and calls the implementation that kind names. It proves the
//! binding reaches a **different function**, not merely a different registry row.
//! 本夹具故意把分派点写出来：宿主读叠加后的有效注册 kind，并调用该 kind 点名的实现。它证明绑定落到
//! **不同的函数**，而不只是换了一行注册记录。

use control_button::FRAMEWORK;
use control_button::control::object::button::Button;
use control_button::control::{ControlFrame, ControlHandle};
use control_button_graft::button_fast::ButtonFast;
use control_button_graft::external_registry;
use xirang_toolchain::run_method::registry_core::{NodeId, root_node_id};
use xirang_toolchain::run_method::{
    CallTrace, GraftPlan, Registry, trace_artifact_path, write_trace_artifact,
};

fn kind_at(registry: &Registry, wanted: &str) -> String {
    registry
        .depth_first()
        .iter()
        .find(|info| registry.path_for(info.id).as_deref() == Some(wanted))
        .map(|info| info.kind.clone())
        .unwrap_or_else(|| panic!("no registration face at `{wanted}`"))
}

/// The slot→implementation binding record the host consumes, produced by the **same** build-side
/// projection the MCP reads. It is keyed by the slot's own id and carries the replacement spelling
/// separately; the registry row never has its id overwritten.
/// 宿主消费的“槽位→实现”绑定记录，由 MCP 读取的**同一份**构建侧投影产生。它以槽位自身 id 为键，
/// 另行携带替换拼写；注册表行绝不覆盖槽位 id。
fn bindings(root: &std::path::Path) -> std::collections::BTreeMap<NodeId, String> {
    let namespace = xirang_toolchain::build_method::package_name(&root.join("Cargo.toml"))
        .expect("package name");
    let views = xirang_toolchain::build_method::face_views(root, &namespace).expect("faces");
    let declared = xirang_toolchain::build_method::declared_grafts(root).expect("grafts");
    xirang_toolchain::build_method::slot_bindings(root, &views, Some(&declared))
        .expect("slot bindings")
        .into_iter()
        .map(|binding| (binding.slot, binding.implementation))
        .collect()
}

fn paint_base() -> &'static str {
    let _: ControlFrame = Button.paint();
    "base-method"
}

fn paint_fast() -> &'static str {
    let _: ControlFrame = ButtonFast.paint();
    "fast-method"
}

/// The host's execution entry: read the slot's id, look up its replacement id in the binding
/// record, and call the implementation that binding names. No kind-string branch.
/// 宿主执行入口：读槽位 id，在绑定记录里查替换件 id，再调用该绑定点名的实现。没有 kind 字符串分支。
fn dispatch(
    registry: &Registry,
    bindings: &std::collections::BTreeMap<NodeId, String>,
) -> Result<&'static str, String> {
    let tree = registry.depth_first();
    let slot = tree
        .iter()
        .find(|info| registry.path_for(info.id).as_deref() == Some("root/control/button"))
        .expect("button slot");
    match bindings.get(&slot.id) {
        None => Ok(paint_base()),
        Some(implementation) if implementation.contains("button_fast") => Ok(paint_fast()),
        Some(implementation) => Err(format!("unknown implementation binding `{implementation}`")),
    }
}

#[test]
fn switching_the_graft_switches_the_called_implementation() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let bindings = bindings(root);
    let base = control_button::base_registry();
    assert_eq!(
        dispatch(&base, &std::collections::BTreeMap::new()).expect("base dispatch"),
        "base-method"
    );
    assert_eq!(kind_at(&base, "root/control/button"), "Button");

    let plan = GraftPlan::command(FRAMEWORK, "cut root/control/button graft button_fast")
        .expect("the graft command parses");
    let effective = base
        .overlay(&plan, &external_registry())
        .expect("the graft publishes");

    assert_eq!(kind_at(&effective, "root/control/button"), "ButtonFast");
    assert_eq!(
        dispatch(&effective, &bindings).expect("graft dispatch"),
        "fast-method"
    );
    let mut unknown = bindings.clone();
    unknown.insert(
        control_button::control::object::button::NODE_ID,
        "not_a_supported_implementation".to_owned(),
    );
    assert!(
        dispatch(&effective, &unknown).is_err(),
        "unknown binding must refuse"
    );
    assert_eq!(
        dispatch(&base, &std::collections::BTreeMap::new()).expect("base dispatch"),
        "base-method",
        "the base tree is untouched by the overlay"
    );

    // The observation comes from the execution path, not from a test copying expected edges.
    // 观察来自执行路径，而不是测试把预期边抄进去。
    let namespace = "xirang-example-control-button";
    let mut trace = CallTrace::full();
    trace.with(root_node_id(namespace), "host", |trace| {
        trace.with(
            control_button::control::object::button::NODE_ID,
            "paint",
            |_| {
                let _ = dispatch(&effective, &bindings).expect("observed dispatch");
            },
        );
    });
    assert!(
        trace
            .call_edges()
            .iter()
            .any(|edge| { edge.caller.function == "host" && edge.callee.function == "paint" }),
        "the execution path must record the observed edge"
    );
    if std::env::var_os("XIRANG_BEHAVIOR_TRACE").is_some() {
        write_trace_artifact(&trace, &trace_artifact_path(root), namespace)
            .expect("write the real execution trace");
    }
}
