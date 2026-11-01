//! Control 文件夹面：它拥有一个 Registry，所有直接子对象都要满足它旁边的规则。
//! Control folder face: it owns a Registry, and every direct child must satisfy
//! the rule kept beside it.

use xirang_toolchain::run_method::{ContractId, FlowContract};

#[xirang_toolchain::soil]
pub const SOIL: xirang_toolchain::Soil = xirang_toolchain::Soil::of::<Control>()
    .needs_registry(true)
    .under(crate::root_node_id(crate::XIRANG_NAMESPACE))
    .registry_rule_path("src/control/registry_rule/registry_rule.rs")
    .flow(FlowContract::new(
        ContractId::new("control.frame.v1"),
        1,
        "ControlInput",
        "ControlFrame",
    ));

// The contract lives in the replacement crate so both sides can implement it without a dependency
// cycle; these re-exports preserve the host's original public paths.
// 契约住在替换件 crate 里，双方都能实现而不产生依赖环；这些重导出保留宿主原来的公开路径。
pub use control_button_graft::{ControlFrame, ControlHandle};

pub struct Control;
