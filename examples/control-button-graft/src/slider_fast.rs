//! 替换 `Slider` 的项目外实现，用于兄弟区间切口。
//! An out-of-project replacement for `Slider`, used by a sibling-range cut.

use xirang_toolchain::run_method::registry_core::{
    ContractId, FlowContract, NoParts, NoPreset, RegistrationRule, root_node_id,
};

pub struct SliderFast;

#[xirang_toolchain::soil]
pub const SOIL: xirang_toolchain::Soil = xirang_toolchain::Soil::of::<SliderFast>()
    .source("slider_fast/slider_fast.rs")
    .preset::<NoPreset>()
    .parts::<NoParts>()
    .name("Fast slider", "快速滑块")
    .summary("Out-of-project implementation", "项目外实现")
    .exports(&["control.render"])
    .needs_registry(false)
    .under(root_node_id(crate::XIRANG_NAMESPACE))
    .getting_from_other_registry(None)
    .registry_rule_path("slider_fast/slider_fast.rs")
    .registry_rule(RegistrationRule::ANY)
    .handle_traits(&["ControlHandle"])
    .provides(&[])
    .flow(FlowContract::new(
        ContractId::new("control.render.v1"),
        1,
        "ControlInput",
        "ControlFrame",
    ))
    .runtime_checks(&[]);
