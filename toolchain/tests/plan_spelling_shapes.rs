//! The readable plan chains are ordinary Rust: every shape the parser accepts must also be writable, and the
//! written value must record what the source says.
//! 可读计划链是普通 Rust：解析器接受的每一种形状都必须写得出来，且写出来的值要记下源码所说的内容。

use xirang_toolchain::run_method::shape::{Plan, PlanEntry, cut};

/// The readable wrapper with all three chains §7k.49 fixed: a single cut, a sibling range, a whole subtree.
/// 可读包装与 §7k.49 定下的三种链：单个切口、兄弟区间、整棵子树。
pub const PLAN_OF: Plan = Plan::of([
    cut("root/a").graft("g1"),
    cut("root/x").to("root/z").graft("g2"),
    cut("root/f").graft("g3").full(),
]);

/// The array wrapper with the same chains, written with the entry type the parser accepts.
/// 数组包装与同一批链，写成解析器接受的条目类型。
pub const PLAN_ARRAY: &[PlanEntry] = &[cut("root/a").graft("g1"), cut("root/f").graft("g3").full()];

/// The declarations above are the pin; this body reads them back, so a chain that silently drops a field is
/// caught here rather than by a build nobody runs in a test.
/// 上面两条声明就是钉子；这个测试体把值读回来，因此"某条链静默丢了一个字段"会在这里被抓到，而不是留给一次
/// 测试里不会发生的构建。
#[test]
fn the_readable_plan_shapes_record_what_the_source_says() {
    assert_eq!(PLAN_ARRAY.len(), 2, "the array wrapper keeps both chains");
    assert_eq!(PLAN_ARRAY[0].cut, "root/a");
    assert_eq!(PLAN_ARRAY[0].graft, "g1");
    assert_eq!(PLAN_ARRAY[0].end, None, "a single cut has no far endpoint");
    assert!(!PLAN_ARRAY[0].whole, "a single cut is not whole-subtree");
    assert_eq!(PLAN_ARRAY[1].end, None);
    assert!(PLAN_ARRAY[1].whole, ".full() is recorded");
    assert_eq!(
        std::mem::size_of::<Plan>(),
        0,
        "the plan itself carries nothing"
    );
    let _ = &PLAN_OF;
}
