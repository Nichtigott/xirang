//! Foreign-source moves: preview, cross-base destinations, children, consumers and records.
//! 外语源码面的搬动：预览、跨基准目的地、子面、消费者与记录。
//!
//! These pins need a real C++ declaration, so they copy the mixed-language example outside the
//! checkout; they are separate from `apply_move_tests` so each file keeps its size budget.
//! 这些钉子需要真实的 C++ 声明，因此把混合语言示例复制到检出之外；与 `apply_move_tests` 分开，
//! 让每个文件都守住自己的尺寸预算。

use serde_json::json;

/// Copy a fixture tree so an external move can be exercised outside the checkout.
/// 复制夹具树，使外部面的搬动能在检出之外演练。
#[cfg(feature = "lang-cpp")]
fn copy_tree(from: &std::path::Path, to: &std::path::Path) {
    std::fs::create_dir_all(to).expect("copy destination");
    for entry in std::fs::read_dir(from).expect("fixture tree") {
        let entry = entry.expect("fixture entry");
        let target = to.join(entry.file_name());
        if entry.file_type().expect("fixture type").is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), target).expect("fixture file");
        }
    }
}

/// A foreign preview is the real move on the throwaway copy: the diff shows it, the project is untouched.
/// 外语面的预览是在一次性副本上真正执行搬动：diff 展示它，而项目本身未被触碰。
#[cfg(feature = "lang-cpp")]
#[test]
fn a_foreign_preview_moves_the_copy_and_leaves_the_project_alone() {
    let example =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../examples/mixed-shop/shop");
    let root = std::env::temp_dir().join(format!(
        "xirang-foreign-preview-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    let _ = std::fs::remove_dir_all(&root);
    copy_tree(&example, &root);

    let preview = crate::mcp::apply::apply(
        &root,
        &json!({
            "action": "move",
            "node": "root/shop/panel/Area/Gauge",
            "to": "root/shop/panel/Area",
            "name": "gauge2",
            "apply": false
        }),
    )
    .expect("the foreign preview runs");
    assert!(
        preview.contains("diff:"),
        "a preview must show the file it would move: {preview}"
    );
    assert!(preview.contains("gauge2.cpp"), "{preview}");
    assert!(
        root.join("native/area/gauge.cpp").is_file(),
        "the project keeps its declaration"
    );
    assert!(
        !root.join("native/area/gauge2.cpp").exists(),
        "the project gains nothing from a preview"
    );
    assert!(
        !root.join(".xirang/moves").exists(),
        "a preview must not leave a compatibility note behind"
    );
    let _ = std::fs::remove_dir_all(root);
}

/// A foreign face cannot be moved to the package root: the generated header has no root constant,
/// and the old `.under(...)` would otherwise survive the rename.
/// 外语面不能搬到包根：生成头没有根的常量，否则旧 `.under(...)` 会活过这次改名。
#[cfg(feature = "lang-cpp")]
#[test]
fn a_foreign_move_to_the_package_root_is_refused() {
    let example =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../examples/mixed-shop/shop");
    let root = std::env::temp_dir().join(format!(
        "xirang-foreign-root-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    let _ = std::fs::remove_dir_all(&root);
    copy_tree(&example, &root);
    let before = std::fs::read_to_string(root.join("native/area/gauge.cpp")).expect("declaration");

    let error = crate::mcp::apply::apply(
        &root,
        &json!({
            "action": "move",
            "node": "root/shop/panel/Area/Gauge",
            "to": "root",
            "name": "gauge2",
            "apply": false
        }),
    )
    .expect_err("a root destination has no C++ spelling");
    assert!(error.contains("package root"), "{error}");
    assert!(error.contains("Nothing changed"), "{error}");
    assert_eq!(
        std::fs::read_to_string(root.join("native/area/gauge.cpp")).expect("declaration"),
        before,
        "a refused move writes nothing"
    );
    assert!(!root.join("gauge2.cpp").exists());
    let _ = std::fs::remove_dir_all(root);
}

/// A C++ leaf can move under a Rust parent even though the two live under different source bases:
/// the destination is computed package-relative, the module follows the new path, and the parent is
/// still named by the generated header's constant.
/// C++ 叶子可以搬到 Rust 父面下——即使两者处在不同源码基准：目的地按包内相对计算，模块跟随新路径，
/// 父级仍由生成头的常量点名。
#[cfg(feature = "lang-cpp")]
#[test]
fn a_foreign_leaf_moves_between_source_bases() {
    let example =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../examples/mixed-shop/shop");
    let root = std::env::temp_dir().join(format!(
        "xirang-foreign-cross-base-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    let _ = std::fs::remove_dir_all(&root);
    copy_tree(&example, &root);
    // Put the child on the **old** `::xirang::faces::…` spelling before the move: reading it is
    // compatibility, and the rewritten declaration must come out in the shared catalog spelling.
    // 搬动前把子面改成旧的 `::xirang::faces::…` 拼法：读取是兼容，改写后的声明必须是共享目录拼法。
    let old_gauge = root.join("native/area/gauge.cpp");
    let old_text = std::fs::read_to_string(&old_gauge).expect("gauge source");
    std::fs::write(
        &old_gauge,
        old_text.replace(
            "crate::catalog::shop::panel::Area::NODE_ID",
            "::xirang::faces::native::area::NODE_ID",
        ),
    )
    .expect("old spelling on disk");

    crate::mcp::apply::apply(
        &root,
        &json!({
            "action": "move",
            "node": "root/shop/panel/Area/Gauge",
            "to": "root/shop/panel",
            "name": "gauge",
            "apply": true
        }),
    )
    .expect("the cross-base move applies");

    let moved = root.join("src/shop/panel/object/gauge/gauge.cpp");
    assert!(moved.is_file(), "the declaration moved under the Rust tree");
    assert!(
        !root.join("native/area/gauge.cpp").exists(),
        "the old location is gone"
    );
    let text = std::fs::read_to_string(&moved).expect("moved declaration");
    assert!(
        text.contains(".under(crate::catalog::shop::panel::NODE_ID)"),
        "the parent is the generated header's constant: {text}"
    );

    let views = crate::build_method::face_views(&root, "shop").expect("the moved tree reads");
    let gauge = views
        .iter()
        .find(|view| view.source == "src/shop/panel/object/gauge/gauge.cpp")
        .unwrap_or_else(|| panic!("the moved face is missing: {views:?}"));
    assert_eq!(gauge.path, "root/shop/panel/Gauge");
    let panel = views
        .iter()
        .find(|view| view.module == "shop::panel")
        .expect("panel face");
    assert_eq!(
        gauge.parent, panel.id,
        "the tree relation followed the move"
    );
    let _ = std::fs::remove_dir_all(root);
}

/// Moving a C++ face with a C++ child and a Rust consumer rewrites both reference spellings in one
/// transaction: the child's `.under(...)` and the consumer's `xirang_external` alias.
/// 搬一个带 C++ 子面与 Rust 消费者的 C++ 面，会在同一事务里改写两种引用拼写：子面的 `.under(...)`
/// 与消费者的 `xirang_external` 别名。
#[cfg(feature = "lang-cpp")]
#[test]
fn a_foreign_move_rewrites_children_and_rust_consumers() {
    let example =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../examples/mixed-shop/shop");
    let root = std::env::temp_dir().join(format!(
        "xirang-foreign-children-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    let _ = std::fs::remove_dir_all(&root);
    copy_tree(&example, &root);

    let applied = crate::mcp::apply::apply(
        &root,
        &json!({
            "action": "move",
            "node": "root/shop/panel/Area",
            "to": "root/shop/panel",
            "name": "area_moved",
            "apply": true
        }),
    )
    .expect("the move with children and consumers applies");
    let moved = root.join("src/shop/panel/object/area_moved/area_moved.cpp");
    assert!(moved.is_file(), "the declaration moved: {applied}");
    assert!(!root.join("native/area/area.cpp").exists());

    let gauge = std::fs::read_to_string(root.join("native/area/gauge.cpp")).expect("child");
    assert!(
        gauge.contains(".under(crate::catalog::shop::panel::Area::NODE_ID)"),
        "the child follows the moved parent: {gauge}"
    );
    let reading = std::fs::read_to_string(root.join("src/shop/panel/area/reading/reading.rs"))
        .expect("consumer");
    assert!(
        reading
            .contains("crate::xirang_external::src__shop__panel__object__area_moved_Area::NODE_ID"),
        "the Rust consumer follows the new alias: {reading}"
    );
    assert!(
        !reading.contains("native__area_Area"),
        "the old alias is gone from the consumer: {reading}"
    );

    let views = crate::build_method::face_views(&root, "shop").expect("the moved tree reads");
    assert_eq!(views.len(), 6, "no face is lost: {views:?}");
    let area = views
        .iter()
        .find(|view| view.source == "src/shop/panel/object/area_moved/area_moved.cpp")
        .unwrap_or_else(|| panic!("the moved face is missing: {views:?}"));
    assert_eq!(area.path, "root/shop/panel/Area");
    let panel = views
        .iter()
        .find(|view| view.module == "shop::panel")
        .expect("panel face");
    assert_eq!(area.parent, panel.id);
    let gauge = views
        .iter()
        .find(|view| view.source == "native/area/gauge.cpp")
        .expect("gauge face");
    assert_eq!(
        gauge.parent, area.id,
        "the child's parent identity follows the move"
    );

    // A foreign move is an identity change too: the Rust branch's compatibility note is written
    // here, with the old and new identity of the face that changed.
    // 外语面的搬动同样是身份变化：这里也要写 Rust 分支那条兼容记录，带上变化了的那张面的新旧身份。
    let record = std::fs::read_to_string(root.join(".xirang/moves/1/move.plan"))
        .expect("the move record is written");
    assert!(record.contains("from=root/shop/panel/Area"), "{record}");
    assert!(record.contains("to=root/shop/panel/Area"), "{record}");
    let ids: Vec<&str> = record
        .lines()
        .filter(|line| line.starts_with("from_id=") || line.starts_with("to_id="))
        .collect();
    assert_eq!(ids.len(), 2, "{record}");
    assert_ne!(ids[0], ids[1], "the identity really changed: {record}");
    let _ = std::fs::remove_dir_all(root);
}

/// A graft record that names the moved C++ face blocks the move until the caller says it may be
/// carried; with that permission the record's target and path are rewritten to the new identity.
/// 点名被搬 C++ 面的嫁接记录会阻塞搬动，直到调用方说可以带走；给了许可后，记录的目标与路径被改写到新身份。
#[cfg(feature = "lang-cpp")]
#[test]
fn a_foreign_move_refuses_a_graft_record_until_it_may_rebind() {
    let example =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../examples/mixed-shop/shop");
    let root = std::env::temp_dir().join(format!(
        "xirang-foreign-graft-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    let _ = std::fs::remove_dir_all(&root);
    copy_tree(&example, &root);

    let views = crate::build_method::face_views(&root, "shop").expect("faces");
    let gauge = views
        .iter()
        .find(|view| view.source == "native/area/gauge.cpp")
        .expect("gauge face");
    let record_dir = root.join(".xirang/external-grafts/impl_gauge");
    std::fs::create_dir_all(&record_dir).expect("record dir");
    std::fs::write(
        record_dir.join("graft.plan"),
        format!(
            "version=1\ntarget={}\ntarget_path={}\ngraft=impl_gauge\nfull=false\n",
            gauge.id, gauge.path
        ),
    )
    .expect("record");

    let request = json!({
        "action": "move",
        "node": "root/shop/panel/Area/Gauge",
        "to": "root/shop/panel",
        "name": "gauge",
        "apply": true
    });
    let error = crate::mcp::apply::apply(&root, &request).expect_err("the record blocks the move");
    assert!(error.contains("impl_gauge"), "{error}");
    assert!(error.contains("rebind_grafts"), "{error}");
    assert!(
        root.join("native/area/gauge.cpp").is_file(),
        "a refused move writes nothing"
    );

    let mut allowed = request;
    allowed["rebind_grafts"] = json!(true);
    crate::mcp::apply::apply(&root, &allowed).expect("the record may be carried");

    let rewritten =
        std::fs::read_to_string(root.join(".xirang/external-grafts/impl_gauge/graft.plan"))
            .expect("rewritten record");
    let views = crate::build_method::face_views(&root, "shop").expect("moved tree");
    let moved = views
        .iter()
        .find(|view| view.source == "src/shop/panel/object/gauge/gauge.cpp")
        .expect("moved face");
    assert!(
        rewritten.contains(&format!("target={}", moved.id)),
        "the record follows the new identity: {rewritten}"
    );
    assert!(
        rewritten.contains(&format!("target_path={}", moved.path)),
        "the record's path follows too: {rewritten}"
    );
    let _ = std::fs::remove_dir_all(root);
}

/// A typed cut in the host's graft plan that names the moved foreign face blocks the move: that
/// spelling is the generated `xirang_external` reference, and rewriting it is not wired yet.
/// 宿主嫁接计划里点名被搬外语面的类型化切口会阻塞搬动：那条拼写是生成的 `xirang_external` 引用，
/// 而改写它尚未接通。
#[cfg(feature = "lang-cpp")]
#[test]
fn a_typed_cut_naming_a_foreign_face_blocks_the_move() {
    let example =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../examples/mixed-shop/shop");
    let root = std::env::temp_dir().join(format!(
        "xirang-foreign-cut-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    let _ = std::fs::remove_dir_all(&root);
    copy_tree(&example, &root);
    let library = root.join("src/lib.rs");
    let mut text = std::fs::read_to_string(&library).expect("library");
    text.push_str(
        "\nxirang_toolchain::run_method::static_graft_plan!(FRAMEWORK,\n    \
         cut(crate::xirang_external::native__area_Area::NODE_ID) \
         graft(external::impl_area::NODE_ID),\n);\n",
    );
    std::fs::write(&library, text).expect("graft plan");

    for (cut, expected) in [
        (
            "cut(crate::xirang_external::native__area_Area::NODE_ID) \
             graft(external::impl_area::NODE_ID),",
            "native__area_Area",
        ),
        (
            "cut \"root/shop/panel/Area\" graft \"impl_area\",",
            "root/shop/panel/Area",
        ),
    ] {
        let mut text = std::fs::read_to_string(&library).expect("library");
        let head = text
            .split("xirang_toolchain::run_method::static_graft_plan!")
            .next()
            .expect("head")
            .to_owned();
        text = format!(
            "{head}xirang_toolchain::run_method::static_graft_plan!(FRAMEWORK,\n    {cut}\n);\n"
        );
        std::fs::write(&library, text).expect("graft plan");

        let error = crate::mcp::apply::apply(
            &root,
            &json!({
                "action": "move",
                "node": "root/shop/panel/Area",
                "to": "root/shop/panel",
                "name": "area_moved",
                "apply": true,
                "rebind_grafts": false
            }),
        )
        .expect_err("the graft entry blocks the move");
        assert!(error.contains("graft entry"), "{error}");
        assert!(error.contains(expected), "{error}");
        assert!(
            root.join("native/area/area.cpp").is_file(),
            "a refused move writes nothing"
        );
    }

    // With permission, the typed entry is carried: the move lands and the cut points at the new
    // generated reference.
    // 给了许可后类型化条目被带走：搬动落地，切口指向新的生成引用。
    let head = std::fs::read_to_string(&library)
        .expect("library")
        .split("xirang_toolchain::run_method::static_graft_plan!")
        .next()
        .expect("head")
        .to_owned();
    std::fs::write(
        &library,
        format!(
            "{head}xirang_toolchain::run_method::static_graft_plan!(FRAMEWORK,\n    \
             cut(crate::xirang_external::native__area_Area::NODE_ID) \
             graft(external::impl_area::NODE_ID),\n);\n"
        ),
    )
    .expect("typed plan");
    crate::mcp::apply::apply(
        &root,
        &json!({
            "action": "move",
            "node": "root/shop/panel/Area",
            "to": "root/shop/panel",
            "name": "area_moved",
            "apply": true,
            "rebind_grafts": true
        }),
    )
    .expect("the typed entry is carried");
    let text = std::fs::read_to_string(&library).expect("library");
    assert!(
        text.contains("crate::xirang_external::src__shop__panel__object__area_moved_Area::NODE_ID"),
        "the cut follows the new reference: {text}"
    );
    assert!(
        !text.contains("crate::xirang_external::native__area_Area::NODE_ID"),
        "the old reference is gone: {text}"
    );
    let _ = std::fs::remove_dir_all(root);
}

/// The file/module name and the display name are separate: renaming the file must not silently
/// rename the object, and a name that is not a Rust module identifier is legal for a C++ face.
/// 文件/模块名与显示名是分开的：改文件名不得静默改掉对象名，而“不是 Rust 模块标识符”的名字对 C++ 面合法。
#[cfg(feature = "lang-cpp")]
#[test]
fn a_foreign_move_keeps_the_display_name_when_the_file_is_renamed() {
    let example =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../examples/mixed-shop/shop");
    let root = std::env::temp_dir().join(format!(
        "xirang-foreign-display-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    let _ = std::fs::remove_dir_all(&root);
    copy_tree(&example, &root);

    let applied = crate::mcp::apply::apply(
        &root,
        &json!({
            "action": "move",
            "node": "root/shop/panel/Area/Gauge",
            "to": "root/shop/panel",
            "name": "Gauge2",
            "apply": true
        }),
    )
    .expect("a C++ name need not be a Rust module identifier");
    assert!(
        root.join("src/shop/panel/object/Gauge2/Gauge2.cpp")
            .is_file(),
        "the file follows the requested name: {applied}"
    );
    assert!(applied.contains("display name: Gauge"), "{applied}");

    let views = crate::build_method::face_views(&root, "shop").expect("the moved tree reads");
    let moved = views
        .iter()
        .find(|view| view.source == "src/shop/panel/object/Gauge2/Gauge2.cpp")
        .expect("moved face");
    assert_eq!(moved.registry_name, "Gauge");
    assert_eq!(
        moved.path, "root/shop/panel/Gauge",
        "the display name and logical path did not follow the file name"
    );
    let _ = std::fs::remove_dir_all(root);
}
