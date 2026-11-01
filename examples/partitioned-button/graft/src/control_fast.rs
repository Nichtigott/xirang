//! 替换整个 `Control` 子树的项目外实现。`full` 切口会丢弃原子树，改用这里携带的
//! 子树；合同必须与 `Control` 自己发布的 `control.frame.v1` 兼容。
//! An out-of-project replacement for the whole `Control` subtree. A `full` cut
//! discards the original subtree and uses the one carried here, so the contract
//! must stay compatible with the `control.frame.v1` that `Control` publishes.

use xirang_toolchain::run_method::registry_core::{
    ContractId, FlowContract, NoParts, NoPreset, RegistrationRule, root_node_id,
};

pub struct ControlFast;

#[xirang_toolchain::soil]
pub const SOIL: xirang_toolchain::Soil = xirang_toolchain::Soil::of::<ControlFast>()
    .source("control_fast/control_fast.rs")
    .preset::<NoPreset>()
    .parts::<NoParts>()
    .name("Fast control", "快速控件")
    .summary("Whole-subtree replacement", "整体替换")
    .exports(&["control.render"])
    .needs_registry(true)
    .under(root_node_id(crate::XIRANG_NAMESPACE))
    .getting_from_other_registry(None)
    .registry_rule_path("control_fast/control_fast.rs")
    .registry_rule(RegistrationRule::ANY)
    .handle_traits(&["ControlHandle"])
    .provides(&[])
    .flow(FlowContract::new(ContractId::new("control.frame.v1"), 1 , "ControlInput" , "ControlFrame" ,))
    .runtime_checks(&[]);
