//! Pins for the shape a host declares: it validates, and a bad one is named.
//! 宿主声明的形状的钉子：它会被校验，而坏的会被点名。

use super::{Crate, Plan, Shape, Subtree, add_crates, cut};

/// A shape that holds together is accepted and handed back unchanged.
/// 一份成立的形状会被接受，并原样交回。
#[test]
fn a_declared_shape_is_accepted_and_handed_back() {
    static WIDGETS: Subtree = Subtree::new("host::control::object");
    static SHAPE: Shape = Shape {
        package_prefix: "myapp",
        crates: &[Crate::named("widgets").at(&[WIDGETS])],
    };
    let shape = add_crates(&SHAPE);
    assert_eq!(shape.package_prefix, "myapp");
    assert_eq!(shape.crates[0].name(), "widgets");
    assert_eq!(
        shape.crates[0].subtrees()[0].module(),
        "host::control::object"
    );
}

/// A shape whose crates overlap fails where it can be seen, and the refusal names both crates.
/// 两个 crate 重叠的形状会在看得见的地方失败，而拒绝把两个 crate 都点名。
#[test]
fn an_overlapping_shape_is_refused_by_name() {
    static OUTER: Subtree = Subtree::new("host::control::object");
    static INNER: Subtree = Subtree::new("host::control::object::button");
    static SHAPE: Shape = Shape {
        package_prefix: "myapp",
        crates: &[
            Crate::named("widgets").at(&[OUTER]),
            Crate::named("tiny").at(&[INNER]),
        ],
    };
    let refusal = std::panic::catch_unwind(|| add_crates(&SHAPE))
        .expect_err("an overlapping declaration is refused");
    let message = refusal
        .downcast_ref::<String>()
        .cloned()
        .unwrap_or_else(|| {
            refusal
                .downcast_ref::<&str>()
                .map(|s| (*s).to_owned())
                .unwrap_or_default()
        });
    assert!(message.contains("`widgets`"), "{message}");
    assert!(message.contains("`tiny`"), "{message}");
    assert!(message.contains("exactly one crate"), "{message}");
}

/// A plan is written as ordinary Rust, and this declaration is the pin: it only compiles if `cut`, the
/// chain and `Plan::of` are all usable in a **`const`** — which is what makes the spelling completable in
/// an editor without any LSP configuration.
/// 计划写成普通 Rust，而这条声明就是钉子：只有当 `cut`、链式方法与 `Plan::of` 在 **`const`** 里都可用时它
/// 才编译得过——而"可用"正是这种拼法在编辑器里不必任何 LSP 配置就能补全的原因。
const _PLAN: Plan = Plan::of([
    cut("crate::control::object::button").graft("crate::fast::button_fast"),
    cut("crate::panel::axis")
        .to("crate::panel::grid")
        .graft("crate::fast::axis_fast"),
    cut("crate::panel::legend")
        .graft("crate::fast::legend_fast")
        .full(),
]);

#[test]
fn a_plan_compiles_as_ordinary_rust() {
    let entries = [
        cut("a").graft("b"),
        cut("c").to("d").graft("e"),
        cut("f").graft("g").full(),
    ];
    assert_eq!(entries[0].cut, "a");
    assert_eq!(entries[0].graft, "b");
    assert_eq!(entries[0].end, None);
    assert!(!entries[0].whole);
    assert_eq!(entries[1].end, Some("d"));
    assert!(entries[2].whole);
    let _ = Plan::of(entries);
}
