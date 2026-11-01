//! 夹具的 NodeEditor 文件夹面：Control 的直接子对象，拥有自己的 Registry。
//! Fixture NodeEditor folder face: a direct child of Control that owns its own
//! Registry.
//!
//! 这里定义画布宽度的预览函数，以及它们对画布对象所做钳制的调用边。
//! This face defines the canvas-width preview functions and the call edges that
//! reach the canvas object's clamping function.

use crate::control::object::node_editor::object::{MIN_CANVAS_WIDTH, clamp_canvas_width};
use crate::control::{ControlFrame, ControlHandle};
use xirang_run_method::{ContractId, FlowContract};

/// NodeEditor 交给子对象的绘制结果。
/// The frame NodeEditor hands to its children for painting.
pub struct NodeEditorFrame;

/// 每个直接子对象必须实现的接口。
/// The interface every direct child must implement.
pub trait NodeEditorHandle {
    fn resize(&self) -> NodeEditorFrame;
}

pub struct NodeEditor;

impl ControlHandle for NodeEditor {
    fn paint(&self) -> ControlFrame {
        ControlFrame
    }
}

/// 预览画布宽度，并把请求交给画布对象做钳制。
/// Preview the canvas width and hand the request to the canvas object to clamp.
pub fn preview_canvas_width(requested: u32) -> u32 {
    let clamped = clamp_canvas_width(requested);
    clamped
}

/// 追踪版预览：记录请求后复用普通预览。
/// Traced preview: record the request, then reuse the plain preview.
pub fn preview_canvas_width_traced(requested: u32) -> u32 {
    let traced = preview_canvas_width(requested);
    traced
}

/// 绘制整个编辑器：布局、钳制宽度，再复用追踪版预览。
/// Paint the whole editor: lay it out, clamp the width, then reuse the traced
/// preview.
///
/// 这个函数存在的理由就是调用图：它是演示里那棵树的根，往下分叉、也在钳制处汇聚，
/// 因此 Studio 的调用树有分叉、有深度、也有扇入可以看。
/// This function exists for the call graph: it is the root of the demo's tree, it
/// fans out and it meets the others again at the clamp, so Studio's call tree has a
/// fork, a depth and a fan-in to look at.
pub fn paint_node_editor(requested: u32) -> u32 {
    let layout = layout_panels(requested);
    let clamped = clamp_canvas_width(layout);
    let preview = preview_canvas_width_traced(clamped);
    preview
}

/// 布局各面板：先测量，再摆放。
/// Lay the panels out: measure first, then place.
pub fn layout_panels(requested: u32) -> u32 {
    let measured = measure_panel(requested);
    let placed = place_panel(measured);
    placed
}

/// 测量一个面板所需的宽度。
/// Measure the width one panel needs.
pub fn measure_panel(requested: u32) -> u32 {
    clamp_canvas_width(requested) / 2
}

/// 摆放一个面板，并把它换算到屏幕坐标。
/// Place one panel and convert it to screen coordinates.
pub fn place_panel(width: u32) -> u32 {
    let screen = to_screen(width);
    screen + MIN_CANVAS_WIDTH
}

/// 把一个宽度换算成屏幕坐标。
/// Convert a width into screen coordinates.
pub fn to_screen(width: u32) -> u32 {
    clamp_canvas_width(width + 1)
}

#[xirang_toolchain::soil]
pub const SOIL: xirang_toolchain::Soil = xirang_toolchain::Soil::of::<NodeEditor>()
    .exports(&["control.render"])
    .needs_registry(true)
    .under(crate::control::NODE_ID)
    .registry_rule_path("src/control/object/node_editor/registry_rule/registry_rule.rs")
    .handle_contracts(xirang_toolchain::contracts!(NodeEditor: crate::control::ControlHandle))
    .flow(FlowContract::new(ContractId::new("control.render.v1"), 1 , "ControlInput" , "ControlFrame" ,));
