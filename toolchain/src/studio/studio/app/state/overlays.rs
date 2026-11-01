//! Modals the workspace can open, one variant per screen.
//! 工作区可以打开的模态，一个界面一个变体。

use super::forms::{AddState, NewProjectState, PluginState};
use super::graft::GraftState;
use super::partition::PartitionState;
use super::search::SearchState;
use crate::run_method::NodeId;

/// Modal screen currently covering the base workspace.
/// 当前覆盖基础工作区的模态界面。
#[derive(Clone, Debug)]
pub enum Overlay {
    /// Search results, optionally shown as a provenance graph.
    /// 搜索结果，可选以溯源图显示。
    Search(SearchState),
    /// New-project wizard.
    /// 新建项目向导。
    NewProject(NewProjectState),
    /// Add a registration face under the current parent.
    /// 在当前父注册面下添加注册面。
    Add(AddState),
    /// Edit the fields of one existing face.
    /// 编辑某个已有注册面的字段。
    Edit(NodeId, AddState),
    /// Plugin selection form.
    /// 插件选择表单。
    Plugin(PluginState),
    /// External graft authoring screen.
    /// 外部 graft 创作界面。
    Graft(GraftState),
    /// Delete confirmation for one node.
    /// 针对某个节点的删除确认。
    Delete(NodeId),
    /// The crate partition this host declares, and the three actions that write it.
    /// 本宿主声明的 crate 分区，以及写下它的三个动作。
    Partition(PartitionState),
}
