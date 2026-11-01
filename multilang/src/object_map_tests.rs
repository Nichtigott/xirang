//! Pins for the object ledger: the format, the three questions it answers, and its refusals.
//! 对象台账的钉子：格式、它回答的三个问题，以及它的拒绝。

use xirang_kernel::identity::{NodeId, StableFaceId};

use super::{LEDGER_FILE, LEDGER_HEADER, ObjectEntry, ObjectMap, Observed, relative_path};

fn stable(name: &str) -> StableFaceId {
    StableFaceId::from_name(name)
}

fn node(source: &str, name: &str) -> NodeId {
    NodeId::from_namespaced_path("shop", source, name)
}

fn observed(name: &str, path: &str, source: &str) -> Observed {
    Observed::new(stable(name), path, source)
}

/// A package that never deviated has no file, and a header of another version is refused rather
/// than read with this version's columns.
/// 从未偏离的包没有这个文件，而另一个版本的表头会被拒绝，而不是按本版本的列去读。
#[test]
fn a_missing_ledger_is_empty_and_a_wrong_header_is_refused() {
    let root = std::env::temp_dir().join(format!("xirang-ledger-empty-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    assert_eq!(ObjectMap::read_from(&root).unwrap(), ObjectMap::empty());
    std::fs::write(root.join(LEDGER_FILE), "# xirang objects v2\n").unwrap();
    let error = ObjectMap::read_from(&root).expect_err("another version is not this version");
    assert!(error.contains("xirang objects v2"), "{error}");
    assert!(error.contains(LEDGER_HEADER), "{error}");
    let _ = std::fs::remove_dir_all(root);
}

/// Everything an entry carries survives a write and a read: identity, path, source, the explicit
/// directory and every source instance.
/// 一个条目携带的一切都能穿过一次写入与一次读取：身份、路径、源码、显式目录与每个源码实例。
#[test]
fn round_trip_keeps_every_column() {
    let mut map = ObjectMap::empty();
    map.observe(
        &observed("area", "root/shop/panel/Area", "components/area/area.cpp"),
        Some(node("components/area/area.cpp", "Area")),
    )
    .unwrap();
    map.set_directory(stable("area"), Some("components/area".to_owned()))
        .unwrap();
    let text = map.render_text();
    let parsed = ObjectMap::parse_text(&text, "test").expect("the render parses");
    assert_eq!(parsed, map);
    assert!(text.contains("components/area"), "{text}");
    assert!(text.starts_with(LEDGER_HEADER), "{text}");
    let entry = parsed.entry(stable("area")).expect("entry");
    assert_eq!(entry.directory.as_deref(), Some("components/area"));
    assert_eq!(entry.instances.len(), 1);
}

/// A logical rename keeps the old logical path in the entry, so the generated catalog can keep a
/// compatibility alias for a third-party declaration.
/// 逻辑改名保留旧逻辑路径，使生成目录能为第三方声明保留兼容别名。
#[test]
fn a_logical_rename_keeps_the_old_path_as_a_retired_alias() {
    let mut map = ObjectMap::empty();
    map.observe(
        &observed("area", "root/shop/panel/Area", "components/area/area.cpp"),
        Some(node("components/area/area.cpp", "Area")),
    )
    .unwrap();
    map.observe(
        &observed("area", "root/shop/panel/area2", "components/area/area2.cpp"),
        Some(node("components/area/area2.cpp", "Area")),
    )
    .unwrap();
    let entry = map.entry(stable("area")).expect("entry");
    assert_eq!(entry.path, "root/shop/panel/area2");
    assert_eq!(entry.retired_paths, vec!["root/shop/panel/Area"]);
    let text = map.render_text();
    let parsed = ObjectMap::parse_text(&text, "test").expect("the render parses");
    assert_eq!(parsed, map);
    assert!(text.contains("root/shop/panel/Area"), "{text}");
}

/// A logical rename history is oldest-first, and returning to an earlier path keeps the history
/// (the live path is the object's current one; the old spelling remains a retired alias).
/// 逻辑改名历史按最旧在前；回到更早路径时保留历史（活路径是对象当前路径，旧拼写仍是退役别名）。
#[test]
fn a_rename_history_keeps_order_and_survives_returning_to_an_old_path() {
    let mut map = ObjectMap::empty();
    for (path, source) in [
        ("root/panel/Area", "src/area.rs"),
        ("root/panel/area2", "src/area2.rs"),
        ("root/panel/Area", "src/area.rs"),
    ] {
        map.observe(&observed("area", path, source), Some(node(source, "Area")))
            .unwrap();
    }
    let entry = map.entry(stable("area")).expect("entry");
    assert_eq!(entry.path, "root/panel/Area");
    assert_eq!(
        entry.retired_paths,
        vec!["root/panel/Area", "root/panel/area2"]
    );
    let text = map.render_text();
    let parsed = ObjectMap::parse_text(&text, "test").expect("the render parses");
    assert_eq!(parsed, map);
}

/// A physical move keeps the object and appends the new instance; the old instance id still
/// resolves to the same object. This is the long-term mechanism the recent-move notes are not.
/// 物理搬动保持对象并追加新实例；旧实例身份仍解析到同一个对象。这就是"最近搬动记录"不能替代的
/// 长期机制。
#[test]
fn a_physical_move_keeps_the_object_and_every_instance_resolves() {
    let mut map = ObjectMap::empty();
    let before = node("native/area/gauge.cpp", "Gauge");
    map.observe(
        &observed(
            "gauge",
            "root/shop/panel/Area/Gauge",
            "native/area/gauge.cpp",
        ),
        Some(before),
    )
    .unwrap();
    // The same logical path, a new source: a physical move.
    // 同一逻辑路径、新源码：一次物理搬动。
    let after = node("components/gauge/gauge.cpp", "Gauge");
    let kept = map
        .observe(
            &observed(
                "gauge",
                "root/shop/panel/Area/Gauge",
                "components/gauge/gauge.cpp",
            ),
            Some(after),
        )
        .unwrap();
    assert_eq!(kept, stable("gauge"), "the object is the same object");
    let entry = map.entry(stable("gauge")).expect("entry");
    assert_eq!(entry.source, "components/gauge/gauge.cpp");
    assert_eq!(
        entry.instances.len(),
        2,
        "both instances are kept: {entry:?}"
    );
    assert_eq!(entry.current_id(), Some(after));
    assert_eq!(
        map.entry_for_instance(before).map(|entry| entry.stable),
        Some(stable("gauge")),
        "an old reference resolves through the ledger, not through a five-entry note"
    );
    assert_eq!(
        map.entry_for_instance(after).map(|e| e.stable),
        Some(stable("gauge"))
    );
}

/// The three ambiguous shapes are refused by name, not silently merged.
/// 三种有歧义的形状都按名拒绝，而不是静默合并。
#[test]
fn a_duplicate_id_path_or_source_is_refused() {
    let first = format!(
        "{}\troot/shop/panel/Area\tcomponents/area/area.cpp\t-\t-",
        stable("area")
    );
    let cases: &[(&str, &str, &str)] = &[
        (
            "declared twice",
            &format!("{}\troot/other\tother.cpp\t-\t-", stable("area")),
            &first,
        ),
        (
            "logical path",
            &format!("{}\troot/shop/panel/Area\tother.cpp\t-\t-", stable("other")),
            &first,
        ),
        (
            "one object",
            &format!(
                "{}\troot/other\tcomponents/area/area.cpp\t-\t-",
                stable("other")
            ),
            &first,
        ),
    ];
    for (expected, second, first_line) in cases {
        let error = ObjectMap::parse_text(
            &format!("{LEDGER_HEADER}\n{first_line}\n{second}\n"),
            "test",
        )
        .expect_err("an ambiguous ledger is refused");
        assert!(error.contains(expected), "{expected}: {error}");
    }
}

/// One observation that answers to two entries is a conflict, and the error names both.
/// 一条同时答得上两个条目的观察是冲突，而错误点名两者。
#[test]
fn an_observation_that_answers_to_two_entries_is_refused() {
    let mut map = ObjectMap::empty();
    for (name, path, source) in [
        ("one", "root/shop/panel/Area", "components/area/area.cpp"),
        ("two", "root/shop/panel/Other", "components/other/other.cpp"),
    ] {
        map.insert(ObjectEntry {
            stable: stable(name),
            path: path.to_owned(),
            retired_paths: Vec::new(),
            source: source.to_owned(),
            directory: None,
            instances: Vec::new(),
        })
        .unwrap();
    }
    // The derived id says `one`, the source says `two`: the ledger cannot honestly pick one.
    // 推导出的 id 指向 `one`，源码指向 `two`：台账无法诚实地挑一个。
    let conflict = map
        .own(&observed(
            "one",
            "root/shop/panel/Area",
            "components/other/other.cpp",
        ))
        .expect_err("two entries for one declaration is a conflict");
    assert!(conflict.contains("different ledger entries"), "{conflict}");
    assert!(conflict.contains("root/shop/panel/Other"), "{conflict}");
}

/// The nearest mapping decides ownership; two objects claiming one directory is refused rather
/// than guessed.
/// 最近的映射决定归属；两个对象声称同一个目录时拒绝，而不是猜。
#[test]
fn the_deepest_mapping_owns_a_directory_and_a_shared_one_is_refused() {
    let mut map = ObjectMap::empty();
    for (name, path, source, directory) in [
        (
            "area",
            "root/shop/panel/Area",
            "components/area/area.cpp",
            "components/area",
        ),
        (
            "gauge",
            "root/shop/panel/Area/Gauge",
            "components/area/objects/gauge/gauge.cpp",
            "components/area/objects/gauge",
        ),
    ] {
        map.insert(ObjectEntry {
            stable: stable(name),
            path: path.to_owned(),
            retired_paths: Vec::new(),
            source: source.to_owned(),
            directory: Some(directory.to_owned()),
            instances: Vec::new(),
        })
        .unwrap();
    }
    let owner = map
        .owner_of_directory("components/area/objects/gauge")
        .unwrap()
        .expect("the nested mapping covers its own directory");
    assert_eq!(owner.stable, stable("gauge"));
    let owner = map
        .owner_of_directory("components/area/object/reading")
        .unwrap()
        .expect("the area mapping covers its children");
    assert_eq!(owner.stable, stable("area"));

    // Two objects whose mappings are the same depth answer for one directory.
    // 两条深度相同的映射同时为一个目录作答。
    map.insert(ObjectEntry {
        stable: stable("twin"),
        path: "root/shop/panel/Twin".to_owned(),
        retired_paths: Vec::new(),
        source: "components/twin/twin.cpp".to_owned(),
        directory: Some("components/area".to_owned()),
        instances: Vec::new(),
    })
    .unwrap();
    let shared = map
        .owner_of_directory("components/area/object/x")
        .expect_err("two owners cannot be guessed between");
    assert!(shared.contains("cannot decide"), "{shared}");
    assert!(shared.contains("Area"), "{shared}");
    assert!(shared.contains("Twin"), "{shared}");
}

/// A mapping that is not followed is reported, and so is an entry whose declaration is gone.
/// 没有被遵守的映射会被报告，声明已经不在的条目也一样。
#[test]
fn drift_names_every_mapping_that_is_not_followed() {
    let mut map = ObjectMap::empty();
    map.insert(ObjectEntry {
        stable: stable("area"),
        path: "root/shop/panel/Area".to_owned(),
        retired_paths: Vec::new(),
        source: "components/area/area.cpp".to_owned(),
        directory: Some("components/area".to_owned()),
        instances: Vec::new(),
    })
    .unwrap();
    map.insert(ObjectEntry {
        stable: stable("gone"),
        path: "root/shop/panel/Gone".to_owned(),
        retired_paths: Vec::new(),
        source: "components/gone/gone.cpp".to_owned(),
        directory: Some("components/gone".to_owned()),
        instances: Vec::new(),
    })
    .unwrap();
    let drift = map.drift(&[observed(
        "area",
        "root/shop/panel/Area",
        "elsewhere/area.cpp",
    )]);
    assert_eq!(drift.len(), 2, "{drift:?}");
    assert!(
        drift.iter().any(|row| row.contains("elsewhere/area.cpp")),
        "{drift:?}"
    );
    assert!(
        drift.iter().any(|row| row.contains("no declaration")),
        "{drift:?}"
    );
    assert!(
        drift.iter().any(|row| row.contains("root/shop/panel/Gone")),
        "{drift:?}"
    );
}

/// Paths that belong to this machine are refused in every column.
/// 属于本机的路径在每一列都被拒绝。
#[test]
fn an_absolute_or_climbing_path_is_refused() {
    for value in ["/etc/passwd", "../outside/area.cpp", ".", ""] {
        assert!(
            relative_path(value).is_err(),
            "`{value}` is not package-relative"
        );
    }
    assert_eq!(
        relative_path("./components/area/area.cpp").unwrap(),
        "components/area/area.cpp"
    );
}
