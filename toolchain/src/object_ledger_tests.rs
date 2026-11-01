//! Pins for the toolchain half of the object ledger: what gets recorded, and what deliberately does
//! not.
//! 对象台账在工具链那一半的钉子：什么会被记录，以及什么有意不记。

use std::path::{Path, PathBuf};

use xirang_kernel::identity::{NodeId, StableFaceId};
use xirang_multilang::object_map::{LEDGER_FILE, ObjectMap, Observed};

use crate::object_ledger::{plan_move, plan_observation};

fn fixture(label: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "xirang-object-ledger-{label}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    root
}

fn observed(name: &str, path: &str, source: &str) -> Observed {
    Observed::new(StableFaceId::from_name(name), path, source)
}

fn ledger(root: &Path) -> String {
    std::fs::read_to_string(root.join(LEDGER_FILE)).expect("the ledger was written")
}

/// A default-layout declaration with no history records nothing: the ledger is the list of
/// deviations, not a second copy of the tree.
/// 没有历史的默认布局声明什么都不记：台账是偏离项的清单，不是树的第二份抄本。
#[test]
fn a_default_declaration_records_nothing() {
    let root = fixture("default");
    let plan = plan_observation(
        &root,
        &observed(
            "gauge",
            "root/shop/panel/Area/Gauge",
            "components/area/object/gauge/gauge.cpp",
        ),
        None,
        None,
    )
    .expect("planning a default declaration is not an error");
    assert!(plan.is_none(), "nothing to record, so nothing is planned");
    assert!(!root.join(LEDGER_FILE).exists(), "and no file is created");
    let _ = std::fs::remove_dir_all(root);
}

/// An explicit placement records the directory and the object; a second observation of the same
/// object after a physical move keeps the identity and appends the instance.
/// 显式落点记录目录与对象；物理搬动之后对同一个对象的再次观察保持身份并追加实例。
#[test]
fn an_explicit_placement_is_recorded_and_a_move_keeps_the_object() {
    let root = fixture("placement");
    let first = observed("area", "root/shop/panel/Area", "components/area/area.cpp");
    let write = plan_observation(
        &root,
        &first,
        Some(NodeId::from_namespaced_path(
            "shop",
            "components/area/area.cpp",
            "Area",
        )),
        Some("components/area".to_owned()),
    )
    .expect("planning succeeds")
    .expect("an explicit placement is recorded");
    assert!(
        write.summary().contains("mapped to `components/area`"),
        "{}",
        write.summary()
    );
    std::fs::write(write.path(), write.bytes()).unwrap();
    assert!(ledger(&root).contains("components/area"));

    let before = observed("area", "root/shop/panel/Area", "components/area/area.cpp");
    let after = observed("area", "root/shop/panel/Area", "islands/area/area.cpp");
    let before_id = NodeId::from_namespaced_path("shop", "components/area/area.cpp", "Area");
    let after_id = NodeId::from_namespaced_path("shop", "islands/area/area.cpp", "Area");
    let write = plan_move(&root, &before, before_id, &after, after_id, true)
        .expect("planning the move succeeds")
        .expect("a move of a recorded object writes the ledger");
    std::fs::write(write.path(), write.bytes()).unwrap();
    let parsed = ObjectMap::read_from(&root).expect("the ledger parses");
    let entry = parsed
        .entry(StableFaceId::from_name("area"))
        .expect("the object is one entry");
    assert_eq!(entry.source, "islands/area/area.cpp");
    assert_eq!(entry.instances.len(), 2, "{entry:?}");
    assert_eq!(
        entry.directory.as_deref(),
        Some("islands/area"),
        "the explicit mapping follows the file instead of describing a place it left"
    );
    assert_eq!(
        parsed
            .entry_for_instance(before_id)
            .map(|entry| entry.stable),
        Some(StableFaceId::from_name("area")),
        "the old instance still resolves"
    );
    let _ = std::fs::remove_dir_all(root);
}

/// A logical move keeps the object too: the `after` observation is re-keyed to the object the
/// ledger already knows, so one object does not become two.
/// 逻辑搬动同样保持对象：`after` 观察被重新挂到台账已知的那个对象上，因此一个对象不会变成两个。
#[test]
fn a_logical_move_does_not_split_one_object_into_two() {
    let root = fixture("logical");
    let before = observed(
        "gauge",
        "root/shop/panel/Area/Gauge",
        "components/area/gauge.cpp",
    );
    let after = observed(
        "gauge",
        "root/shop/panel/Gauge",
        "components/area/gauge.cpp",
    );
    let before_id = NodeId::from_namespaced_path("shop", "components/area/gauge.cpp", "Gauge");
    let write = plan_move(&root, &before, before_id, &after, before_id, false)
        .expect("planning succeeds")
        .expect("a logical move is recorded");
    std::fs::write(write.path(), write.bytes()).unwrap();
    let parsed = ObjectMap::read_from(&root).unwrap();
    assert_eq!(parsed.entries().len(), 1, "{:?}", parsed.entries());
    assert_eq!(
        parsed.entries()[0].path,
        "root/shop/panel/Gauge",
        "the object followed the parent"
    );
    let _ = std::fs::remove_dir_all(root);
}

/// A move that puts the object back at the layout its parent derives **clears** the deviation: a
/// mapping is the list of objects that need one, and one that is no longer needed would describe a
/// place the object left.
/// 把对象放回父级推导布局的搬动会**清除**偏离：映射是"需要它的对象"的清单，而一个不再需要的映射会描述
/// 对象已经离开的地方。
#[test]
fn a_move_back_to_the_default_layout_clears_the_mapping() {
    let root = fixture("cleared");
    let observed_area = observed("area", "root/shop/panel/Area", "components/area/area.cpp");
    let id = NodeId::from_namespaced_path("shop", "components/area/area.cpp", "Area");
    let write = plan_observation(
        &root,
        &observed_area,
        Some(id),
        Some("components/area".to_owned()),
    )
    .expect("planning succeeds")
    .expect("an explicit placement is recorded");
    std::fs::write(write.path(), write.bytes()).unwrap();
    assert!(ledger(&root).contains("components/area"));

    // The same object, now under a parent whose default layout is where it landed.
    let after = observed(
        "area",
        "root/shop/panel/Area",
        "components/area/object/area/area.cpp",
    );
    let after_id =
        NodeId::from_namespaced_path("shop", "components/area/object/area/area.cpp", "Area");
    let write = plan_move(&root, &observed_area, id, &after, after_id, false)
        .expect("planning the move succeeds")
        .expect("clearing a mapping is a write");
    assert!(
        write.summary().contains("mapping cleared"),
        "{}",
        write.summary()
    );
    std::fs::write(write.path(), write.bytes()).unwrap();
    let parsed = ObjectMap::read_from(&root).expect("the ledger parses");
    let entry = parsed
        .entry(StableFaceId::from_name("area"))
        .expect("the object is one entry");
    assert_eq!(entry.directory, None, "the deviation is gone");
    assert_eq!(entry.instances.len(), 2, "both instances still resolve");
    let _ = std::fs::remove_dir_all(root);
}
