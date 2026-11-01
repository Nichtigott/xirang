//! Form overlay rendering for Studio.
//! Studio 表单浮层渲染。
//!
//! This root mounts one page per form and re-exports its entry point: the
//! add/edit face form in `face`, the new-project form in `project`, the
//! plugin form in `plugin`, the graft composer in `graft`, and the delete
//! confirmation in `delete`. The shared field dictionary and field guide stay
//! in `face_fields`.
//! 本模块根为每个表单挂载一个页面并重导出其入口：新增/编辑注册面表单在 `face`，
//! 新建项目表单在 `project`，插件表单在 `plugin`，graft 撰写器在 `graft`，
//! 删除确认在 `delete`。共享字段词典与字段指南仍在 `face_fields`。

use super::*;

#[path = "forms/delete.rs"]
mod delete;
#[path = "forms/face.rs"]
mod face;
#[path = "forms/face_fields.rs"]
mod face_fields;
#[path = "forms/graft.rs"]
mod graft;
#[path = "forms/partition.rs"]
mod partition;
#[path = "forms/plugin.rs"]
mod plugin;
#[path = "forms/project.rs"]
mod project;

pub(super) use delete::draw_delete;
pub(super) use face::{draw_add, draw_edit};
pub(super) use graft::draw_graft;
pub(super) use partition::draw_partition;
pub(super) use plugin::draw_plugin;
pub(super) use project::draw_new_project;

/// The four areas one form page's bottom row yields to its click handler.
/// 一个表单页底行交给点击处理器的四个区域。
pub(super) struct FormAreas {
    /// The body/list area above the buttons.
    /// 按钮上方的列表/正文区域。
    pub list: Rect,
    /// The cancel (or close) button.
    /// 取消（或关闭）按钮。
    pub cancel: Rect,
    /// The primary action: save, create or write.
    /// 主操作：保存、创建或写入。
    pub confirm: Rect,
    /// The exit button.
    /// 退出按钮。
    pub exit: Rect,
}

/// Draw a form page's bottom row: four equal cells — primary action, cancel, exit, hint —
/// and hand back the areas the pointer handler clicks.
/// 绘制表单页底行：四个等宽格子——主操作、取消、退出、提示——并把点击处理器要用的区域交回去。
///
/// All four pages used to spell this split, the button closure and the returned tuple
/// themselves, so a change to one page's hit boxes could disagree with the other three
/// without anything failing (audit `STU-S-21`).
/// 四个页面过去各自写着这份切分、按钮闭包与返回元组，因此改动某一页的点击区可能与另外三页不一致
/// 而没有任何东西失败（审计 `STU-S-21`）。
pub(super) fn draw_form_frame(
    frame: &mut Frame<'_>,
    row: Rect,
    list: Rect,
    confirm_label: &str,
    cancel_label: &str,
    hint: &str,
) -> FormAreas {
    let buttons = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
        ])
        .split(row);
    /// One button cell: its label, its colour, and the same frame every page used.
    /// 一个按钮格子：它的标签、颜色，以及每个页面过去都用的同一个边框。
    fn button<'a>(label: &'a str, color: Color) -> Paragraph<'a> {
        Paragraph::new(label)
            .alignment(Alignment::Center)
            .style(Style::default().fg(color))
            .block(panel("", color))
    }
    frame.render_widget(button(confirm_label, GREEN), buttons[0]);
    frame.render_widget(button(cancel_label, MUTED), buttons[1]);
    frame.render_widget(button("Exit [q]", Color::LightRed), buttons[2]);
    frame.render_widget(
        Paragraph::new(hint)
            .alignment(Alignment::Center)
            .style(Style::default().fg(MUTED).bg(PANEL)),
        buttons[3],
    );
    FormAreas {
        list,
        cancel: buttons[1],
        confirm: buttons[0],
        exit: buttons[2],
    }
}
