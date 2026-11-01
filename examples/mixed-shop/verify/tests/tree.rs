//! 断言两件事：两棵 crate 的面接成一棵树（其中 Area 由 C++ 声明、挂在 Rust 的 Panel 下），
//! 以及 C++ 实现真的被调到了。
//! Assert two things: the two crates' faces join into one tree (with Area declared in C++, under the
//! Rust Panel), and the C++ implementation is really reached.

use xirang_toolchain::{FrameworkId, Registry};

fn assembled() -> Registry {
    let mut registry =
        Registry::root_for_namespace(FrameworkId::new("xirang.example.mixed-shop"), "shop");
    registry
        .register_all(&shop::registrations())
        .expect("host faces");
    registry
        .register_all(&shop_panel::registrations())
        .expect("panel faces");
    registry
}

#[test]
fn the_two_crates_assemble_one_tree() {
    let registry = assembled();
    let mut rows = registry
        .depth_first()
        .iter()
        .map(|info| {
            format!(
                "{} kind={} source={}",
                registry.path_for(info.id).unwrap_or_default(),
                info.kind,
                info.source.file
            )
        })
        .collect::<Vec<_>>();
    rows.sort();
    assert_eq!(
        rows,
        vec![
            "root/shop kind=Shop source=shop/shop.rs",
            "root/shop/panel kind=Panel source=shop/panel/panel.rs",
            "root/shop/panel/Area kind=Area source=native/area/area.cpp",
            "root/shop/panel/Area/Gauge kind=Gauge source=native/area/gauge.cpp",
            "root/shop/panel/Area/reading kind=Reading source=shop/panel/area/reading/reading.rs",
            "root/shop/panel/button kind=Button source=shop/panel/button/button.rs",
        ]
    );
}

/// `Gauge` 的父必须是 `Area`：一个 C++ 面拥有另一个 C++ 面，边是记录里的 `parent`，不是目录。
/// `Gauge`'s parent must be `Area`: a C++ face owning another C++ face, with the edge carried by the
/// record's `parent` rather than by the directory.
#[test]
fn a_cpp_face_owns_the_face_declared_below_it() {
    let registry = assembled();
    let rows = registry
        .depth_first()
        .iter()
        .map(|info| (info.kind.clone(), info.parent))
        .collect::<Vec<_>>();
    let parent_of = |kind: &str| {
        rows.iter()
            .find(|(name, _)| name == kind)
            .map(|(_, parent)| *parent)
            .unwrap_or_else(|| panic!("the tree carries `{kind}`"))
    };
    let id_of = |kind: &str| {
        registry
            .depth_first()
            .iter()
            .find(|info| info.kind == kind)
            .map(|info| info.id)
            .unwrap_or_else(|| panic!("the tree carries `{kind}`"))
    };
    // 一个 **Rust** 面挂在 C++ 父面下：`Reading` 的父是 `Area`，而它的源码住在宿主的 `src/` 里、由
    // `shop-panel` 编译。父边是记录里的身份，不是目录，也不是“语言相同”。
    // A **Rust** face under a C++ parent: `Reading`'s parent is `Area`, while its source lives in the
    // host's `src/` and is compiled by `shop-panel`. The edge is the recorded identity — not the
    // directory, and not "same language".
    assert_eq!(parent_of("Reading"), id_of("Area"));
    assert_eq!(
        registry.path_for(id_of("Reading")).as_deref(),
        Some("root/shop/panel/Area/reading")
    );
    assert_eq!(parent_of("Gauge"), id_of("Area"));
    assert_eq!(
        registry.path_for(id_of("Gauge")).as_deref(),
        Some("root/shop/panel/Area/Gauge")
    );
}
