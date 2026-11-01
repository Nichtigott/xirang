//! Opening the partition screen, and doing what it was asked to do (audit `M7`, P3.6).
//! 打开分区屏，并执行它被要求做的事（审计 `M7`，P3.6）。
//!
//! The screen is a viewer of the same reader the CLI uses, and its three actions call the **same**
//! writers `xirang crates --write/--release/--revert` call — so the two surfaces cannot write
//! different trees, and a defect fixed in one is fixed in both.
//! 本屏是 CLI 所用同一个读取器的查看器，而它的三个动作调用**同一批**写入方，也就是
//! `xirang crates --write/--release/--revert` 调用的那些——因此两个执行面不可能写下不同的树，而在一边修好的
//! 缺陷在两边都好。

use std::path::Path;

use super::write_guard::{selected_package_root, with_selected_project_read};
use super::*;
use crate::build_method::partition_view::view;

impl App {
    /// Open the partition screen for the selected project.
    /// 为选中的项目打开分区屏。
    pub(super) fn open_partition(&mut self) {
        let mut state = PartitionState::default();
        read_partition_state(&mut state);
        self.overlay = Some(Overlay::Partition(state));
    }

    /// Read the partition again, into the state the screen is showing.
    /// 重新读取拆分，写进本屏正在显示的状态里。
    pub(super) fn refresh_partition(&mut self) {
        if let Some(Overlay::Partition(state)) = self.overlay.as_mut() {
            read_partition_state(state);
        }
    }

    /// Remove the selected crate from the host's declaration, the way the CLI and the bridge do.
    /// 把选中的 crate 从宿主声明里移除，与 CLI 和桥同一条路。
    ///
    /// The guards are shared rather than repeated: a crate whose package is still on disk is refused
    /// (after the entry is gone, no action knows that directory), and the edit is rolled back when the
    /// resulting declaration does not plan.
    /// 守卫是共用的而不是重写的：包还在磁盘上的 crate 会被拒绝（条目一旦消失就没有动作知道那个目录），而改动在
    /// "改完之后声明规划不成立"时会被回滚。
    pub(super) fn undeclare_selected_crate(&mut self, name: String) {
        let outcome = with_selected_project_read(|| -> Result<String, String> {
            let root = selected_package_root()?;
            if let Some(directory) = crate::build_method::package_directory_of(&root, &name)? {
                return Err(format!(
                    "`{name}` still has its package at {}: press `x` to revert first (it takes the \
                     generated packages back while the declaration still names them)\n\
                     `{name}` 的包还在 {}：先按 `x` 收回（它趁声明还点名它们时把生成的包收回去）\n",
                    directory.display(),
                    directory.display()
                ));
            }
            let edit = crate::build_method::undeclare(&root, &name)?;
            let diff = edit.diff();
            let after = crate::build_method::apply_declaration_edit(&root, edit)?;
            Ok(format!(
                "removed `{name}` from {}:\n{diff}{}",
                root.join(xirang_kernel::lexicon::ADD_CRATES_FILE).display(),
                match after {
                    Some(view) => format!(
                        "the declaration now names {} crate(s)\n",
                        view.packages
                            .iter()
                            .filter(|package| package.crate_name.is_some())
                            .count()
                    ),
                    None => "this host declares no crates again\n".to_owned(),
                }
            ))
        });
        if let Some(Overlay::Partition(state)) = self.overlay.as_mut() {
            state.pending_undeclare = None;
            state.outcome = Some(match outcome {
                Ok(message) => message,
                Err(refusal) => refusal,
            });
        }
        self.refresh_partition();
    }

    /// Do what the screen was asked to do, and say what happened.
    /// 执行本屏被要求做的事，并说明发生了什么。
    pub(super) fn run_partition_action(&mut self, action: PartitionAction) {
        let outcome = with_selected_project_read(|| write_partition_action(action));
        if let Some(Overlay::Partition(state)) = self.overlay.as_mut() {
            state.pending = None;
            state.outcome = Some(match outcome {
                Ok(message) => message,
                Err(refusal) => refusal,
            });
        }
        self.refresh_partition();
    }
}

/// Read the partition of the selected project into `state`.
/// 把选中项目的拆分读进 `state`。
fn read_partition_state(state: &mut PartitionState) {
    state.view = None;
    state.declared = false;
    state.error = None;
    match selected_package_root() {
        // A project whose manifest cargo cannot name is a refusal like any other, and the screen
        // says it instead of drawing an empty tree (audit `STU-S-29`).
        // cargo 说不出名字的项目和别的拒绝一样，本屏把它说出来，而不是画一棵空树（审计 `STU-S-29`）。
        Err(refusal) => state.error = Some(refusal),
        Ok(root) => match with_selected_project_read(|| view(&root)) {
            Ok(None) => {}
            Ok(Some(view)) => {
                state.declared = true;
                state.selected = 0;
                state.view = Some(view);
            }
            Err(refusal) => state.error = Some(refusal),
        },
    }
}

/// The one writer each action calls, shared with the CLI verb of the same name.
/// 每个动作调用的那一个写入方，与同名 CLI 动词共用。
fn write_partition_action(action: PartitionAction) -> Result<String, String> {
    let root = selected_package_root()?;
    let Some(plan) = crate::build_method::partition_view::plan(&root)? else {
        return Err(format!(
            "{} declares no crates: {} has no declaration, so there is nothing to write",
            root.display(),
            root.join(xirang_kernel::lexicon::ADD_CRATES_FILE).display()
        ));
    };
    let directories: Vec<&Path> = plan
        .planned
        .iter()
        .map(|planned| planned.directory.as_path())
        .chain(plan.facade.iter().map(|facade| facade.directory.as_path()))
        .collect();
    match action {
        PartitionAction::WriteDevelopment => {
            crate::build_method::guard_shape(&directories, false)?;
            let written = crate::build_method::write_partition(
                &plan.config_root,
                plan.workspace.as_deref(),
                &plan.planned,
                plan.facade.as_ref(),
            )?;
            Ok(summary(&written, plan.workspace.is_some(), "development"))
        }
        PartitionAction::WriteRelease => {
            crate::build_method::guard_shape(&directories, true)?;
            let written = crate::build_method::write_release(
                &plan.config_root,
                plan.workspace.as_deref(),
                &plan.release,
                &plan.host_root,
            )?;
            Ok(summary(&written, plan.workspace.is_some(), "release"))
        }
        PartitionAction::Revert => {
            let written = crate::build_method::revert_partition(
                &plan.config_root,
                plan.workspace.as_deref(),
                &plan.planned,
                plan.facade.as_ref(),
            )?;
            Ok(format!(
                "removed {} generated package(s); workspace config {}, members {}",
                written.files.len(),
                if written.config_changed {
                    "updated"
                } else {
                    "untouched"
                },
                if written.members_changed {
                    "updated"
                } else {
                    "untouched"
                }
            ))
        }
    }
}

/// One sentence for a write, with the two halves a reader has to know about.
/// 一次写入的一句话，带上读者必须知道的另一半。
fn summary(
    written: &crate::build_method::crate_write::Written,
    in_a_workspace: bool,
    shape: &str,
) -> String {
    format!(
        "wrote {} file(s) in the {shape} shape; workspace config {}, members {}",
        written.files.len(),
        if written.config_changed {
            "updated"
        } else {
            "untouched"
        },
        if !in_a_workspace {
            "not applicable (no enclosing workspace)"
        } else if written.members_changed {
            "updated"
        } else {
            "already listed"
        }
    )
}
