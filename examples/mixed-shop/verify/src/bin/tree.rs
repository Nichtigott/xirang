//! 断言：宿主留下的面（含 C++ 声明的 Area）与拆出去那个 crate 的面，能在同一棵树里接上。
//! Assert: the faces the host kept and the faces the split-off crate carries join in one tree.

use xirang_toolchain::{FrameworkId, Registry};

fn main() {
    let mut registry =
        Registry::root_for_namespace(FrameworkId::new("xirang.example.mixed-shop"), "shop");
    registry
        .register_all(&shop::registrations())
        .expect("host faces");
    registry
        .register_all(&shop_panel::registrations())
        .expect("panel faces");

    let mut rows = registry
        .depth_first()
        .iter()
        .map(|info| {
            format!(
                "{}  kind={}",
                registry.path_for(info.id).unwrap_or_default(),
                info.kind
            )
        })
        .collect::<Vec<_>>();
    rows.sort();
    for row in &rows {
        println!("{row}");
    }
}
