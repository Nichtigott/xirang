//! The partition screen's state (audit `M7`, P3.6).
//! 分区屏的状态（审计 `M7`，P3.6）。

use crate::build_method::PartitionView;

/// What the partition screen shows, and what the user has asked it to do.
/// 分区屏显示什么，以及用户让它做什么。
///
/// The view is the shared reader's output (`build_method::partition_view`), not a second summary: the
/// screen and `xirang crates` have to describe the same tree the same way.
/// 这里的视图是共用读取器（`build_method::partition_view`）的产物，不是第二份摘要：本屏与
/// `xirang crates` 必须用同一种说法描述同一棵树。
#[derive(Clone, Debug, Default)]
pub struct PartitionState {
    /// The partition as read from the selected project, when it declares one.
    /// 从选中项目读到的拆分（它声明了拆分时）。
    pub view: Option<PartitionView>,
    /// Why it could not be read. A host that declares nothing is not an error — see `declared`.
    /// 读不出来的原因。没有声明的宿主不是错误——见 `declared`。
    pub error: Option<String>,
    /// Whether the selected host declares any crates at all.
    /// 选中的宿主是否声明了任何 crate。
    pub declared: bool,
    /// Which package row is selected.
    /// 当前选中的包行。
    pub selected: usize,
    /// The action waiting for its confirmation.
    /// 正在等待确认的动作。
    pub pending: Option<PartitionAction>,
    /// The declared crate whose entry is waiting to be removed from `add_crates.rs`.
    /// 正在等待从 `add_crates.rs` 里移除条目的那个已声明 crate。
    ///
    /// It carries the name rather than reusing `pending`: that one names an action on the **generated
    /// packages**, while this one edits the author's hand-written declaration, and the two are
    /// different layers (audit `M7`, §M7.48).
    /// 它带着名字，而不是复用 `pending`：后者点名的是对**生成包**的动作，而这一个编辑的是作者手写的声明，
    /// 两者是两层（审计 `M7`，§M7.48）。
    pub pending_undeclare: Option<String>,
    /// What the last action did, said out loud.
    /// 上一个动作做了什么，明说出来。
    pub outcome: Option<String>,
}

/// What the screen can do to the tree, once the user has said so twice.
/// 本屏能对这棵树做的事，前提是用户说了两遍。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PartitionAction {
    /// Write the development shape (mounts; needs the workspace remap).
    /// 写下开发形状（挂载；依赖工作区 remap）。
    WriteDevelopment,
    /// Write the release shape (self-contained; publishable).
    /// 写下发布形状（自包含；可发布）。
    WriteRelease,
    /// Take the generated packages back.
    /// 把生成的包收回来。
    Revert,
}

impl PartitionAction {
    /// The key that arms this action.
    /// 装备这个动作的按键。
    pub fn key(self) -> char {
        match self {
            PartitionAction::WriteDevelopment => 'w',
            PartitionAction::WriteRelease => 'R',
            PartitionAction::Revert => 'x',
        }
    }

    /// What the confirmation sentence says this action does.
    /// 确认句说这个动作做什么。
    pub fn sentence(self) -> &'static str {
        match self {
            PartitionAction::WriteDevelopment => {
                "write the development shape (the fragments mount the host's files, and the workspace \
                 config gains the remap)"
            }
            PartitionAction::WriteRelease => {
                "write the release shape (each package carries the sources its build reads, so it can \
                 be published)"
            }
            PartitionAction::Revert => {
                "take the generated packages back (this action's directories only, and the member \
                 list entries it wrote)"
            }
        }
    }
}
