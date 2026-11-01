//! Pins for catalog aliases built from the object ledger.
//! 从对象台账构建目录别名的钉子。

use super::build;
use crate::build_method::static_plan::StaticFaceRecord;
use xirang_kernel::identity::StableFaceId;
use xirang_multilang::object_map::{Instance, ObjectEntry, ObjectMap};

#[test]
fn a_retired_logical_path_becomes_a_catalog_alias() {
    let root = std::env::temp_dir().join(format!("xirang-catalog-alias-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("package root");
    let mut map = ObjectMap::empty();
    map.insert(ObjectEntry {
        stable: StableFaceId::from_name("area"),
        path: "root/panel/area2".to_owned(),
        retired_paths: vec!["root/panel/Area".to_owned()],
        source: "components/area/area2.cpp".to_owned(),
        directory: None,
        instances: vec![Instance {
            id: xirang_kernel::identity::NodeId::from_raw([7; 16]),
            source: "components/area/area2.cpp".to_owned(),
        }],
    })
    .expect("one object");
    std::fs::write(root.join("xirang.objects.tsv"), map.render_text()).expect("ledger");
    let entries = build(&root, &[], &std::collections::BTreeMap::new(), &[]).expect("catalog");
    assert!(
        entries.contains(&("panel/Area".to_owned(), [7; 16])),
        "the retired path names the current object: {entries:?}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn an_old_alias_whose_path_became_live_is_refused_not_rebound() {
    let root =
        std::env::temp_dir().join(format!("xirang-catalog-alias-live-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("package root");
    let mut map = ObjectMap::empty();
    map.insert(ObjectEntry {
        stable: StableFaceId::from_name("moved"),
        path: "root/other".to_owned(),
        retired_paths: vec!["root/panel".to_owned()],
        source: "src/other.rs".to_owned(),
        directory: None,
        instances: vec![Instance {
            id: xirang_kernel::identity::NodeId::from_raw([2; 16]),
            source: "src/other.rs".to_owned(),
        }],
    })
    .expect("one object");
    std::fs::write(root.join("xirang.objects.tsv"), map.render_text()).expect("ledger");
    let live = StaticFaceRecord {
        id: xirang_kernel::identity::NodeId::from_raw([1; 16]),
        written_as_literal: false,
        kind: "Panel".to_owned(),
        preset: String::new(),
        parts: String::new(),
        parent: xirang_kernel::identity::root_node_id("shop"),
        owns_registry: false,
        source: "src/panel.rs".to_owned(),
        module: "panel".to_owned(),
    };
    let error = build(&root, &[], &std::collections::BTreeMap::new(), &[live])
        .expect_err("an old reference must not silently bind to the new object");
    assert!(
        error.contains("silently bind to the wrong object"),
        "{error}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_historical_instance_of_the_same_object_is_not_a_conflict() {
    let root = std::env::temp_dir().join(format!(
        "xirang-catalog-alias-same-object-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("package root");
    let mut map = ObjectMap::empty();
    map.insert(ObjectEntry {
        stable: StableFaceId::from_name("same"),
        path: "root/panel/new".to_owned(),
        retired_paths: vec!["root/panel/old".to_owned()],
        source: "src/new.rs".to_owned(),
        directory: None,
        instances: vec![
            Instance {
                id: xirang_kernel::identity::NodeId::from_raw([1; 16]),
                source: "src/old.rs".to_owned(),
            },
            Instance {
                id: xirang_kernel::identity::NodeId::from_raw([2; 16]),
                source: "src/new.rs".to_owned(),
            },
        ],
    })
    .expect("one stable object");
    std::fs::write(root.join("xirang.objects.tsv"), map.render_text()).expect("ledger");
    let live = StaticFaceRecord {
        id: xirang_kernel::identity::NodeId::from_raw([1; 16]),
        written_as_literal: false,
        kind: "Panel".to_owned(),
        preset: String::new(),
        parts: String::new(),
        parent: xirang_kernel::identity::root_node_id("shop"),
        owns_registry: false,
        source: "src/old.rs".to_owned(),
        module: "panel".to_owned(),
    };
    let entries = build(&root, &[], &std::collections::BTreeMap::new(), &[live])
        .expect("the historical instance belongs to the same stable object");
    assert!(
        entries.contains(&("panel".to_owned(), [1; 16])),
        "the live instance is still the answer: {entries:?}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn two_retired_aliases_with_one_path_are_refused() {
    let root = std::env::temp_dir().join(format!(
        "xirang-catalog-alias-conflict-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("package root");
    let mut map = ObjectMap::empty();
    for (name, current, id) in [
        ("one", "root/panel/one", [1; 16]),
        ("two", "root/panel/two", [2; 16]),
    ] {
        map.insert(ObjectEntry {
            stable: StableFaceId::from_name(name),
            path: current.to_owned(),
            retired_paths: vec!["root/panel/old".to_owned()],
            source: format!("src/{name}.rs"),
            directory: None,
            instances: vec![Instance {
                id: xirang_kernel::identity::NodeId::from_raw(id),
                source: format!("src/{name}.rs"),
            }],
        })
        .expect("one object");
    }
    std::fs::write(root.join("xirang.objects.tsv"), map.render_text()).expect("ledger");
    let error = build(&root, &[], &std::collections::BTreeMap::new(), &[])
        .expect_err("one retired path cannot name two objects");
    assert!(error.contains("ambiguous"), "{error}");
    let _ = std::fs::remove_dir_all(&root);
}
