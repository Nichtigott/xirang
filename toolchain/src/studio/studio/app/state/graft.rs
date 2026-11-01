//! External graft screen state and declaration vocabulary.
//! 外部 graft 界面状态与声明词汇。

use std::path::PathBuf;

use crate::run_method::NodeId;

/// Compose one external graft declaration and manage the plans already on disk.
/// 撰写一条外部 graft 声明，并管理磁盘上已有的计划。
///
/// A plan is an authoring record, not the declaration the compiler reads and
/// not the overlay itself: this screen writes the record, shows the entry line
/// the host still has to declare, and never edits host source.
/// 计划是创作记录，既不是编译器读取的声明，也不是覆盖应用本身：这个界面写记录、
/// 显示宿主仍需声明的那一行，并且永不改动宿主源码。
#[derive(Clone, Debug)]
pub struct GraftState {
    /// The selected face whose slot is handed over.
    /// 交出去的槽位所属的、当前选中的注册面。
    pub target: NodeId,
    /// Path of the selected target face, as shown in the compose pane.
    /// 所选目标注册面的路径，显示在撰写区。
    pub target_path: String,
    /// Editable selector naming the external implementation.
    /// 可编辑的选择器，命名外部实现。
    pub selector: String,
    /// Whether the whole subtree is replaced instead of only the node.
    /// 是否替换整棵子树，而不是只替换节点。
    pub full: bool,
    /// Focused compose row: 0 selector, 1 scope.
    /// 撰写区当前行：0 选择器，1 替换范围。
    pub field: usize,
    /// Focused pane: 0 compose, 1 the plans already on disk.
    /// 当前面板：0 撰写区，1 磁盘上已有的计划。
    pub pane: usize,
    /// Whether the focused compose row is accepting typed input.
    /// 撰写区当前行是否正在接受键入。
    pub editing: bool,
    /// Plans already written under `.xirang/external-grafts/`.
    /// 已写在 `.xirang/external-grafts/` 下的计划。
    pub plans: Vec<GraftPlanRow>,
    /// Index of the highlighted plan row.
    /// 当前高亮计划行的下标。
    pub plan_selected: usize,
    /// Selector a first `d` armed for deletion.
    /// 第一次按 `d` 时进入待删状态的选择器。
    ///
    /// Deleting a record moves a directory into the trash; one keypress is too
    /// little for that, so the first press arms and names the record, and any
    /// other key clears the arm. The field lives in the state rather than in a
    /// local so the prompt can be drawn.
    /// 删除一条记录会把目录移进回收目录；一个按键对这件事太少，因此第一次按只进入待删状态并
    /// 点名该记录，任何其他键都会解除。字段放在状态里而不是局部变量里，是为了能把提示画出来。
    pub pending_delete: Option<String>,
    /// What the host entry declares for this target.
    /// 宿主入口为这个目标声明了什么。
    pub declaration: GraftDeclaration,
    /// Whether the target face declares a flow contract the overlay can check.
    /// 目标注册面是否声明了 overlay 能校验的数据流合同。
    pub flow_declared: bool,
    /// Child faces a non-full cut inherits from the base registry.
    /// 非整树切口从原注册机继承的子注册面数量。
    pub inherited_children: usize,
}

/// One plan already written under `.xirang/external-grafts/`.
/// `.xirang/external-grafts/` 下已写好的一个计划。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GraftPlanRow {
    /// Selector naming the external implementation.
    /// 命名外部实现的选择器。
    pub selector: String,
    /// Face path the plan cuts, as recorded on disk.
    /// 计划切出的注册面路径，按磁盘记录。
    pub target_path: String,
    /// Whether the plan replaces the whole subtree.
    /// 计划是否替换整棵子树。
    pub full: bool,
    /// Why the plan could not be read, when it could not.
    /// 计划读不懂时的原因。
    pub error: Option<String>,
}

impl GraftPlanRow {
    /// Whether the plan was read cleanly and may be applied.
    /// 计划是否读取无误、可以应用。
    pub fn is_valid(&self) -> bool {
        self.error.is_none()
    }
}

/// What the host entry says about the selected slot.
/// 宿主入口对当前槽位的说法。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GraftDeclaration {
    /// A declared cut names this face; the build ships it.
    /// 有一条声明命名了这个面；构建会发布它。
    Declared {
        /// Declaration expression naming this face.
        /// 命名该注册面的声明表达式。
        expression: String,
        /// Entry line the expression was read from.
        /// 读到该表达式的入口行号。
        line: usize,
        /// Cfg condition guarding the declaration, when present.
        /// 门控该声明的 cfg 条件（若有）。
        cfg: Option<String>,
    },
    /// The entry was read and names other slots.
    /// 入口读到了，但命名的是别的槽位。
    Absent {
        /// Entry file that was read but does not name this face.
        /// 已读取但未命名该注册面的入口文件。
        entry: PathBuf,
    },
    /// The entry could not be resolved, read, or parsed.
    /// 入口无法解析、读取或解析失败。
    Unknown {
        /// Why the entry could not be resolved, read, or parsed.
        /// 入口无法解析、读取或解析失败的原因。
        reason: String,
    },
}
