//! Call references, the assembled call-tree view, and its memo.
//! 调用引用、装配好的调用树视图，以及它的备忘。

use crate::run_method::mir::CallTree;
use crate::run_method::{NodeId, Registry};

/// One reference to a function at a node, used for call navigation.
/// 指向某个节点上函数的引用，用于调用跳转。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CallRef {
    /// Node that declares the function.
    /// 声明该函数的节点。
    pub node: NodeId,
    /// Function name as resolved for this reference.
    /// 该引用解析出的函数名。
    pub function: String,
    /// Source file the function lives in, empty when unknown.
    /// 函数所在的源文件；未知时为空。
    pub file: String,
}

/// The spatial call tree around one focus, with a reference per node.
/// 某个焦点周围的空间调用树，以及每个节点对应的引用。
///
/// `tree` is the kernel's model (levels, lanes, edges, cuts); `refs` is Studio's
/// side of the same list, aligned index for index, so the cursor a caller holds
/// means the same node to both halves.
/// `tree` 是内核模型（层、车道、边、裁剪计数）；`refs` 是同一列表在 Studio 一侧的对应物，
/// 按下标一一对齐，因此调用方持有的游标对两半指向同一个节点。
#[derive(Clone, Debug, Default)]
pub struct CallTreeView {
    /// Kernel model of the focus's tree.
    /// 焦点调用树的内核模型。
    pub tree: CallTree,
    /// Reference for each node, aligned with `tree.nodes`.
    /// 与 `tree.nodes` 对齐的每个节点的引用。
    pub refs: Vec<Option<CallRef>>,
}

impl CallTreeView {
    /// Number of nodes, focus included.
    /// 节点数（含焦点）。
    pub fn len(&self) -> usize {
        self.tree.nodes.len()
    }

    /// Whether there are no nodes at all.
    /// 是否一个节点都没有。
    pub fn is_empty(&self) -> bool {
        self.tree.nodes.is_empty()
    }

    /// Reference at one node index.
    /// 某个节点下标对应的引用。
    ///
    /// Only the widget drawer's status row reads this, so it is compiled with the
    /// `node-graph` feature.
    /// 只有控件绘制方的状态行读它，因此与 `node-graph` 特性一同编译。
    #[cfg(feature = "node-graph")]
    pub fn item(&self, index: usize) -> Option<CallRef> {
        self.refs.get(index).and_then(Option::clone)
    }
}

/// One memoised call tree, keyed by the focus that produced it and the source
/// stamp it was built from. Building a tree reads every source file once per
/// node, and one frame asks for it several times, so the memo is what keeps the
/// spatial view affordable in a terminal.
/// 一条被备忘的调用树，以产生它的焦点与构建时的源码戳为键。构建一棵树要对每个节点读一遍全部
/// 源文件，而一帧会问它好几次，因此备忘是让空间视图在终端里负担得起的东西。
#[derive(Clone, Debug)]
pub(crate) struct CallTreeMemo {
    /// Source stamp the view was built from.
    /// 构建该视图时的源码戳。
    pub(crate) stamp: u128,
    /// Focus the tree was built around.
    /// 构建该树所用的焦点。
    pub(crate) focus: CallRef,
    /// The built view.
    /// 构建出的视图。
    pub(crate) view: CallTreeView,
}

pub(crate) fn push_call_ref(
    registry: &Registry,
    output: &mut Vec<CallRef>,
    node: NodeId,
    function: impl Into<String>,
) {
    let function = function.into();
    if output
        .iter()
        .any(|item| item.node == node && item.function == function)
    {
        return;
    }
    output.push(CallRef {
        node,
        function,
        file: registry
            .find_registry(node)
            .map(|info| info.source.file.as_str())
            .unwrap_or("")
            .to_owned(),
    });
}
