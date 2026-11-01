//! Native registration and graft declarations must be real compiled values.
//! 原生注册与 graft 声明必须是真实参与编译的值。

use xirang_toolchain::{Graft, Soil, graft, range, subtree};

pub const XIRANG_NAMESPACE: &str = "native-declarations";

mod first {
    use super::*;
    pub struct Button;
    #[xirang_toolchain::soil]
    pub const BUTTON: Soil = Soil::of::<Button>()
        .under(xirang_toolchain::root_node_id(crate::XIRANG_NAMESPACE))
        .name("Button", "按钮")
        .requires("render", "Renderer")
        .summary("A button", "一个按钮")
        .requires("layout", "Layout")
        .stable_name("ui.button")
        .provides(&["click"]);
}

mod second {
    use super::*;
    pub struct Button;
    #[xirang_toolchain::soil]
    pub const BUTTON: Soil = Soil::of::<Button>().name("Replacement", "替换件");
}

mod compatibility {
    use xirang_toolchain::Face;
    pub struct Button;
    #[xirang_toolchain::face]
    pub const BUTTON: Face = Face::of::<Button>().name("Replacement", "替换件");
}

#[test]
fn old_face_spelling_keeps_the_same_compiled_registration_and_type() {
    let _: Soil = compatibility::BUTTON;
    let _: xirang_toolchain::Face = second::BUTTON;
    let mut old = compatibility::REGISTRATION.into_snapshot();
    let new = second::REGISTRATION.into_snapshot();
    old.registry_name.clone_from(&new.registry_name);
    old.source.line = new.source.line;
    old.source.column = new.source.column;
    assert_eq!(old, new);
}

const GRAFTS: &[Graft] = &[
    graft(first::NODE_ID).with(second::NODE_ID),
    graft(range(first::NODE_ID, second::NODE_ID)).with(second::NODE_ID),
    graft(subtree(first::NODE_ID)).with(second::NODE_ID),
    graft("root/button").with("fast/button"),
];

#[test]
fn native_values_feed_real_registration_and_graft_metadata() {
    let info = first::REGISTRATION;
    assert_eq!(info.name.zh, "按钮");
    assert_eq!(info.summary.en, "A button");
    assert_eq!(info.kind, "Button");
    assert_eq!(info.preset, "NoPreset");
    assert_eq!(
        info.requires
            .iter()
            .map(|r| r.capability)
            .collect::<Vec<_>>(),
        ["render", "layout"]
    );
    assert_eq!(info.provides, ["click"]);
    assert_eq!(info.stable_name, Some("ui.button"));
    assert_eq!(GRAFTS[0].cut().id(), Some(info.id));
    assert!(!GRAFTS[1].full());
    assert!(GRAFTS[2].full());
    assert_eq!(GRAFTS[1].cut_end().unwrap().id(), Some(second::NODE_ID));
    assert_eq!(second::REGISTRATION.name.en, "Replacement");
}

// A non-object-safe contract must work without a `dyn Trait` workaround.
// 非对象安全合同必须正常工作，无需 `dyn Trait` 变通。
trait GenericContract {
    fn echo<T>(value: T) -> T {
        value
    }
}
pub struct Widget;
impl GenericContract for Widget {}
pub struct Preset;
impl xirang_toolchain::PresetContract for Preset {
    type Output = Widget;
    const REQUIRED_PARTS: &'static [&'static str] = &["body"];
}
pub struct Parts;
impl GenericContract for Parts {}
impl xirang_toolchain::PartsContract for Parts {
    type Output = Widget;
    const PROVIDED_PARTS: &'static [&'static str] = &["body"];
}
struct Flow;
impl xirang_toolchain::FlowContractProvider for Flow {
    const FLOW_CONTRACT: xirang_toolchain::FlowContract = xirang_toolchain::FlowContract::new(
        xirang_toolchain::ContractId::new("provider"),
        1,
        "Input",
        "Output",
    );
}
const PLUGIN: xirang_toolchain::PluginManifest = xirang_toolchain::PluginManifest {
    name: "plugin",
    crate_name: "plugin",
    version: "1",
    framework: xirang_toolchain::FrameworkId::new("test"),
    source: xirang_toolchain::PluginSource::User,
    mode: xirang_toolchain::PluginMode::Replacement,
    checksum: "hash",
    signature: None,
    public_key_fingerprint: None,
    revocation_list: None,
};
mod legacy_complete {
    use super::*;
    use xirang_toolchain::*;
    xirang_toolchain::external_object! {
        source: "widget.rs", kind: Widget, preset: Preset, parts: Parts,
        name: {zh: "组件", en: "Widget"}, summary: {zh: "说明", en: "Summary"},
        exports: ["widget"], stable_name: "widget", needs_registry: true,
        parent: xirang_toolchain::root_node_id(crate::XIRANG_NAMESPACE),
        getting_from_other_registry: Some("external"),
        registry_rule_path: "rule.rs", registry_rule: RegistrationRule::ANY,
        admission: Admission::ANY, handle_traits: ["ignored"], part_traits: ["ignored"],
        handle_contracts: [GenericContract], part_contracts: [GenericContract],
        requires: ["render" => "Renderer", "layout" => "Layout"], provides: ["widget"],
        flow: FlowContract::NONE, flow_provider: Flow,
        plugin: PLUGIN, runtime_checks: [FINITE_NUMBER],
    }
}
mod native_complete {
    use super::*;
    use xirang_toolchain::*;
    #[xirang_toolchain::soil(collector = linked)]
    pub const WIDGET: Soil = Soil::of::<Widget>()
        .source("widget.rs")
        .preset::<Preset>()
        .parts::<Parts>()
        .name("Widget", "组件")
        .summary("Summary", "说明")
        .exports(&["widget"])
        .stable_name("widget")
        .needs_registry(true)
        .under(root_node_id(crate::XIRANG_NAMESPACE))
        .getting_from_other_registry(Some("external"))
        .registry_rule_path("rule.rs")
        .registry_rule(RegistrationRule::ANY)
        .admission(Admission::ANY)
        .handle_traits(&["ignored"])
        .part_traits(&["ignored"])
        .handle_contracts(contracts!(Widget: GenericContract))
        .part_contracts(contracts!(Parts: GenericContract))
        .requires("render", "Renderer")
        .requires("layout", "Layout")
        .provides(&["widget"])
        .flow(FlowContract::NONE)
        .flow_provider::<Flow>()
        .plugin(PLUGIN)
        .runtime_checks(&[FINITE_NUMBER]);
}

#[xirang_toolchain::soil]
#[cfg(any())]
const DISABLED: Soil = Soil::of::<Missing>()
    .preset::<Missing>()
    .parts::<Missing>()
    .handle_contracts(xirang_toolchain::contracts!(Missing: MissingTrait));

#[test]
fn all_compiled_registration_fields_match_the_legacy_protocol() {
    let mut expected = legacy_complete::REGISTRATION.into_snapshot();
    let actual = native_complete::REGISTRATION.into_snapshot();
    expected.registry_name.clone_from(&actual.registry_name);
    expected.source.line = actual.source.line;
    expected.source.column = actual.source.column;
    assert_eq!(actual, expected);
    assert_eq!(Widget::echo(7), 7);
}
