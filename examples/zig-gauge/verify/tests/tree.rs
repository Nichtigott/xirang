//! 分区之后两个 crate 的面必须接成同一棵树：Rust 父面、Zig 子面、再挂在其下的 Rust 面。
//! After the partition the two crates' faces must join into one tree: the Rust parent, the Zig child,
//! and the Rust face under it.

use xirang_toolchain::{FrameworkId, Registry};

fn assembled() -> Registry {
    let mut registry =
        Registry::root_for_namespace(FrameworkId::new("xirang.example.zig-gauge"), "gauge");
    registry
        .register_all(&gauge::registrations())
        .expect("host faces");
    registry
        .register_all(&gauge_panel::registrations())
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
            "root/shop/panel/Gauge kind=Gauge source=native/gauge/gauge.zig",
            "root/shop/panel/Gauge/dial kind=Dial source=shop/panel/dial/dial.rs",
        ]
    );
}

/// 碎片真的编译了那个 Zig 面：宿主这一侧不再编译它，只有碎片编译时它才在树里。
/// The fragment really compiles the Zig face: the host no longer compiles it, so it is in the tree
/// only because the fragment did.
#[test]
fn the_fragment_compiles_the_zig_face() {
    let registry = assembled();
    let tree = registry.depth_first();
    let gauge = tree
        .iter()
        .find(|info| info.kind == "Gauge")
        .expect("the Zig face is in the assembled tree");
    assert_eq!(gauge.source.file, "native/gauge/gauge.zig");
}
