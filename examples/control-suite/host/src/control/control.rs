//! Control 文件夹面：它拥有一个 Registry，所有直接子对象都要满足它旁边的规则。
//! Control folder face: it owns a Registry, and every direct child must satisfy
//! the rule kept beside it.

use xirang_toolchain::run_method::{ContractId, FlowContract};

#[xirang_toolchain::soil]
pub const SOIL: xirang_toolchain::Soil = xirang_toolchain::Soil::of::<Control>()
    .needs_registry(true)
    .under(crate::root_node_id(crate::XIRANG_NAMESPACE))
    .registry_rule_path("src/control/registry_rule/registry_rule.rs")
    .flow(FlowContract::new(ContractId::new("control.frame.v1"), 1 , "ControlInput" , "ControlFrame" ,));

/// 父注册面交给子对象的绘制结果。
/// The frame a parent face hands to its children for painting.
pub struct ControlFrame;

/// 每个直接子对象必须实现的接口。
/// The interface every direct child must implement.
pub trait ControlHandle {
    fn paint(&self) -> ControlFrame;
}

pub struct Control;
