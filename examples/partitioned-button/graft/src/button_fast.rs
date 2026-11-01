//! 替换 `Button` 的外部实现。它以 `#[path]` 之外的普通模块方式载入，因此
//! 文件可以照常以 `//!` 开头，编辑器也能原生解析它。
//! The out-of-project replacement for `Button`. It loads as an ordinary module
//! (the author writes `mod button_fast;`), so the file keeps its `//!` header and
//! editor tooling resolves it natively.

use xirang_toolchain::run_method::registry_core::{
    ContractId, FlowContract, NoParts, NoPreset, RegistrationRule, root_node_id,
};

pub struct ButtonFast;

#[xirang_toolchain::soil]
pub const SOIL: xirang_toolchain::Soil = xirang_toolchain::Soil::of::<ButtonFast>()
    .source("button_fast/button_fast.rs")
    .preset::<NoPreset>()
    .parts::<NoParts>()
    .name("Fast button", "快速按钮")
    .summary("Out-of-project implementation", "项目外实现")
    .exports(&["control.render"])
    .needs_registry(false)
    .under(root_node_id(crate::XIRANG_NAMESPACE))
    .getting_from_other_registry(None)
    .registry_rule_path("button_fast/button_fast.rs")
    .registry_rule(RegistrationRule::ANY)
    .handle_traits(&["ControlHandle"])
    .provides(&[])
    .flow(FlowContract::new(ContractId::new("control.render.v1"), 1 , "ControlInput" , "ControlFrame" ,))
    .runtime_checks(&[]);
