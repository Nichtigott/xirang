//! 夹具的 Canvas 叶子面：NodeEditor 的直接子对象，定义画布宽度的钳制与接收入口。
//! Fixture Canvas leaf face: a direct child of NodeEditor that owns the
//! canvas-width clamp and the accept entry point.

use crate::control::object::node_editor::{NodeEditorFrame, NodeEditorHandle};
use xirang_run_method::{ContractId, FlowContract};

/// 画布允许的最小宽度。
/// The canvas's minimum allowed width.
pub const MIN_CANVAS_WIDTH: u32 = 120;

/// 画布允许的最大宽度。
/// The canvas's maximum allowed width.
pub const MAX_CANVAS_WIDTH: u32 = 4096;

pub struct Canvas;

impl NodeEditorHandle for Canvas {
    fn resize(&self) -> NodeEditorFrame {
        NodeEditorFrame
    }
}

/// 把请求宽度钳制到画布允许的区间内。
/// Clamp a requested width into the canvas's allowed range.
pub fn clamp_canvas_width(requested: u32) -> u32 {
    let clamped = requested.clamp(MIN_CANVAS_WIDTH, MAX_CANVAS_WIDTH);
    clamped
}

/// 接收一个具名画布，并返回它钳制后的宽度。
/// Accept a named canvas and return its clamped width.
pub fn accept_canvas(canvas_name: &str, requested: u32) -> u32 {
    let _ = canvas_name;
    clamp_canvas_width(requested)
}

#[xirang_toolchain::soil]
pub const SOIL: xirang_toolchain::Soil = xirang_toolchain::Soil::of::<Canvas>()
    .exports(&["node_editor.render"])
    .under(crate::control::object::node_editor::NODE_ID)
    .handle_contracts(xirang_toolchain::contracts!(Canvas: crate::control::object::node_editor::NodeEditorHandle))
    .flow(FlowContract::new(ContractId::new("node_editor.render.v1"), 1 , "NodeEditorInput" , "NodeEditorFrame" ,));
