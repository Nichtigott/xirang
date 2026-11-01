//! Native const graft selectors; all forms lower to the existing static plan.
//! 原生 const graft 选择器；所有形式都落到既有静态计划。

use super::{NodeId, StaticGraftCut};

/// The same plan entry as `StaticGraftCut`, with the declaration-facing name.
/// 与 `StaticGraftCut` 相同的计划条目，使用面向声明的名称。
pub type Graft = StaticGraftCut;

/// An inclusive sibling range, evaluated against the current tree.
/// 根据当前树求值的、包含两端的兄弟区间。
#[derive(Clone, Copy, Debug)]
pub struct GraftRange<T> {
    start: T,
    end: T,
}

/// A selection that includes each selected node's descendants.
/// 包含每个选中节点的后代的选择器。
#[derive(Clone, Copy, Debug)]
pub struct GraftSubtree<T>(T);

/// A target awaiting its replacement.
/// 等待指定替换件的目标。
#[derive(Clone, Copy, Debug)]
pub struct GraftSelection<T>(T);

/// Select a node, range, or subtree for replacement.
/// 选择需要替换的节点、区间或子树。
pub const fn graft<T: Copy>(target: T) -> GraftSelection<T> {
    GraftSelection(target)
}

/// Select an inclusive sibling range.
/// 选择包含两端的兄弟区间。
pub const fn range<T: Copy>(start: T, end: T) -> GraftRange<T> {
    GraftRange { start, end }
}

/// Include descendants of a node.
/// 包含节点的后代。
pub const fn subtree<T: Copy>(target: T) -> GraftSubtree<T> {
    GraftSubtree(target)
}

macro_rules! replacements {
    ($ty:ty, $single:ident, $range:ident) => {
        impl GraftSelection<$ty> {
            /// Replace the selected node through the graft plan.
            /// 通过 graft 计划替换选中节点。
            pub const fn with(self, replacement: $ty) -> Graft {
                Graft::$single(self.0, replacement, false)
            }
        }
        impl GraftSelection<GraftRange<$ty>> {
            /// Replace the selected sibling range through the graft plan.
            /// 通过 graft 计划替换选中兄弟区间。
            pub const fn with(self, replacement: $ty) -> Graft {
                Graft::$range(self.0.start, self.0.end, replacement, false)
            }
        }
        impl GraftSelection<GraftSubtree<$ty>> {
            /// Replace the selected subtree through the graft plan.
            /// 通过 graft 计划替换选中子树。
            pub const fn with(self, replacement: $ty) -> Graft {
                Graft::$single(self.0.0, replacement, true)
            }
        }
    };
}

replacements!(NodeId, from_ids, from_id_range);
replacements!(&'static str, new, new_range);
