//! Slider 叶子面：Control 的第二个直接子对象，用来演示同一父级下的兄弟区间替换。
//! Slider leaf face: Control's second direct child, used to demonstrate a graft
//! over a contiguous range of siblings under one parent.

use crate::control::{ControlFrame, ControlHandle};
use xirang_toolchain::run_method::{ContractId, FlowContract};

#[xirang_toolchain::soil]
pub const SOIL: xirang_toolchain::Soil = xirang_toolchain::Soil::of::<Slider>()
    .exports(&["control.render"])
    .under(crate::control::NODE_ID)
    .handle_contracts(xirang_toolchain::contracts!(Slider: crate::control::ControlHandle))
    .flow(FlowContract::new(ContractId::new("control.render.v1"), 1 , "ControlInput" , "ControlFrame" ,));

pub struct Slider;

impl ControlHandle for Slider {
    fn paint(&self) -> ControlFrame {
        ControlFrame
    }
}
