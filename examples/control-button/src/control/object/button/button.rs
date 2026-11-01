//! Button 叶子面：它是 Control 的直接子对象，不再拥有自己的 Registry。
//! Button leaf face: a direct child of Control that owns no Registry of its own.

use crate::control::{ControlFrame, ControlHandle};
use xirang_toolchain::run_method::{ContractId, FlowContract, NON_EMPTY_TEXT};

#[xirang_toolchain::soil]
pub const SOIL: xirang_toolchain::Soil = xirang_toolchain::Soil::of::<Button>()
    .exports(&["control.render"])
    .under(crate::control::NODE_ID)
    .handle_contracts(xirang_toolchain::contracts!(Button: crate::control::ControlHandle))
    .flow(FlowContract::new(
        ContractId::new("control.render.v1"),
        1,
        "ControlInput",
        "ControlFrame",
    ))
    .runtime_checks(&[NON_EMPTY_TEXT]);

pub struct Button;

impl ControlHandle for Button {
    fn paint(&self) -> ControlFrame {
        ControlFrame
    }
}
