//! The island fixture: default layout, an explicit directory, inheritance, nested override, an
//! inferred parent, a shared directory, a physical move, another absolute path, and the status line.
//! 孤岛夹具：默认布局、显式目录、继承、嵌套覆盖、推断父级、共享目录、物理搬动、另一个绝对路径，以及
//! 状态行。
//!
//! Every test goes through the product entry (`crate::mcp::apply::apply`), not through an internal
//! helper, so what is pinned is what an agent gets. The tree being built is the one the foundation
//! review names:
//! 每个测试都走产品入口（`crate::mcp::apply::apply`），而不是内部辅助函数，因此钉住的就是代理拿到的。
//! 被搭出来的树正是基础复核点名的那一棵：
//!
//! ```text
//! Panel
//! └── Area            (explicit: components/area)
//!     ├── Gauge       (inherits: components/area/object/gauge)
//!     ├── Gauge2      (nested override: components/gauge)
//!     └── Reading     (inherits: components/area/object/reading)
//! ```
#![cfg(feature = "lang-cpp")]

use std::path::{Path, PathBuf};

use serde_json::json;
use xirang_kernel::identity::StableFaceId;
use xirang_multilang::object_map::{LEDGER_FILE, ObjectMap};

/// A bare package `cargo metadata` can name, so the namespace is real rather than configured.
/// 一个 `cargo metadata` 能命名的裸包，因此命名空间是真的而不是配置出来的。
pub(super) fn package(label: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "xirang-island-{label}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("src")).expect("package dir");
    std::fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"island\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[workspace]\n",
    )
    .expect("manifest");
    std::fs::write(root.join("src/lib.rs"), "// host entry\n").expect("entry");
    root
}

/// `xirang.apply`, the product entry, which previews unless `apply` is true.
/// 产品入口 `xirang.apply`；除非 `apply` 为真，否则只预览。
pub(super) fn apply(root: &Path, request: serde_json::Value) -> Result<String, String> {
    crate::mcp::apply::apply(root, &request)
}

/// A Rust face under a C++ parent must not poison the package for later authoring.
/// 挂在 C++ 父级下的 Rust 面不得让这个包之后的创作全部失败。
///
/// The generated declaration names its parent through `xirang_external::<flat>`, and two things used to
/// break on the **next** call: the authoring parser read that alias as a module path, and the staged
/// registry validated the Rust child before the native placeholder existed. Either alone makes the
/// package unauthorable, so both are reproduced here as one sequence.
/// 生成的声明经 `xirang_external::<flat>` 点名父级，而**下一次**调用过去有两处会坏：创作解析器把别名
/// 当模块路径读，以及暂存注册树在本地占位存在之前就校验了 Rust 子面。任一个单独就足以让这个包无法再被创作，
/// 因此两个都在这条序列里复现。
#[test]
fn a_rust_face_under_a_cpp_parent_does_not_block_later_authoring() {
    let root = package("rust-under-cpp-sequence");
    apply(
        &root,
        json!({"action":"add","fields":{"module":"panel","kind":"Panel","needs_registry":true},"apply":true}),
    )
    .expect("Panel");
    apply(
        &root,
        json!({"action":"add","language":"cpp","parent":"root/panel",
               "file":"components/area/area.cpp",
               "fields":{"module":"area","kind":"Area","needs_registry":true},"apply":true}),
    )
    .expect("the C++ Area");
    apply(
        &root,
        json!({"action":"add","language":"rust","parent":"root/panel/Area",
               "fields":{"module":"sample","kind":"Sample"},"apply":true}),
    )
    .expect("a Rust face under the C++ parent");

    // The next calls are the regression: both must see the package's own faces, including the one
    // that names its parent through the generated alias.
    // 接下来的两次调用就是回归：两者都必须看见这个包自己的面，包括那个经生成别名点名父级的面。
    apply(
        &root,
        json!({"action":"add","language":"rust","parent":"root/panel",
               "fields":{"module":"later","kind":"Later"},"apply":true}),
    )
    .expect("a later Rust add still works");
    apply(
        &root,
        json!({"action":"add","language":"cpp","parent":"root/panel/Area",
               "fields":{"module":"other","kind":"Other"},"apply":true}),
    )
    .expect("a later own-language add still works");
    let faces = faces(&root);
    for path in [
        "root/panel",
        "root/panel/Area",
        "root/panel/Area/sample",
        "root/panel/later",
        "root/panel/Area/Other",
    ] {
        assert!(
            faces.iter().any(|face| face.path == path),
            "the tree lost `{path}`: {:?}",
            faces.iter().map(|face| &face.path).collect::<Vec<_>>()
        );
    }
    let _ = std::fs::remove_dir_all(root);
}

/// The tree the product reports, derived the way every consumer derives it.
/// 产品报告的树，按每个消费者都用的方式推导。
fn faces(root: &Path) -> Vec<crate::build_method::FaceView> {
    crate::build_method::face_views(root, "island").expect("the tree reads")
}

fn view<'a>(
    faces: &'a [crate::build_method::FaceView],
    path: &str,
) -> &'a crate::build_method::FaceView {
    faces
        .iter()
        .find(|face| face.path == path)
        .unwrap_or_else(|| panic!("no face at `{path}`: {faces:?}"))
}

fn ledger(root: &Path) -> ObjectMap {
    ObjectMap::read_from(root).expect("the ledger reads")
}

/// Panel, then the C++ `Area` at an explicit directory, then its two children.
/// Panel，然后显式放在独立目录里的 C++ `Area`，然后是它的两个子面。
pub(super) fn island() -> PathBuf {
    let root = package("tree");
    apply(
        &root,
        json!({"action":"add","fields":{"module":"panel","kind":"Panel","needs_registry":true},"apply":true}),
    )
    .expect("Panel is created");
    apply(
        &root,
        json!({"action":"add","language":"cpp","parent":"root/panel",
               "file":"components/area/area.cpp",
               "fields":{"module":"area","kind":"Area","needs_registry":true},"apply":true}),
    )
    .expect("Area is created in its own directory");
    apply(
        &root,
        json!({"action":"add","language":"cpp","parent":"root/panel/Area",
               "fields":{"module":"gauge","kind":"Gauge"},"apply":true}),
    )
    .expect("Gauge inherits Area's directory");
    root
}

/// A default child of a regular parent follows `<parent directory>/object/<module>/<module>.<ext>`.
/// 普通父级下的默认子面遵循 `<父目录>/object/<模块>/<模块>.<扩展名>`。
#[test]
fn a_default_child_follows_the_standard_layout() {
    let root = package("default");
    apply(
        &root,
        json!({"action":"add","fields":{"module":"panel","kind":"Panel","needs_registry":true},"apply":true}),
    )
    .expect("Panel is created");
    apply(
        &root,
        json!({"action":"add","language":"cpp","parent":"root/panel",
               "fields":{"module":"gauge","kind":"Gauge"},"apply":true}),
    )
    .expect("Gauge is created");
    let faces = faces(&root);
    let panel = view(&faces, "root/panel");
    // The standard layout is `<parent directory>/object/<module>/<module>.<ext>`; the parent's own
    // directory is the one the tree reports, so the expectation is derived rather than restated.
    // 标准布局是 `<父目录>/object/<模块>/<模块>.<扩展名>`；父级自己的目录就是树报告的那个，因此预期值
    // 是推导出来的，而不是再抄一遍。
    let layout = crate::build_method::source_layout(&root).expect("the layout reads");
    let parent_source = crate::build_method::source_ref::package_relative(
        panel.source_base,
        &layout.package_root,
        &layout.identity_base,
        &panel.source,
    );
    let parent_directory = Path::new(&parent_source)
        .parent()
        .expect("the parent has a directory")
        .to_string_lossy()
        .replace('\\', "/");
    let expected = root.join(&parent_directory).join("object/gauge/gauge.cpp");
    assert!(
        expected.is_file(),
        "a default C++ child of a Rust parent lands under the parent's directory + object/gauge: {}",
        expected.display()
    );
    let gauge = view(&faces, "root/panel/Gauge");
    assert_eq!(gauge.parent, panel.id, "the parent is the face named");
    assert!(
        !root.join(LEDGER_FILE).exists(),
        "a default placement records no mapping: the ledger is the list of deviations"
    );
    let _ = std::fs::remove_dir_all(root);
}

/// An explicit directory keeps the declared parent, and the mapping is recorded as a deviation.
/// 显式目录保持声明的父级，而映射作为一次偏离被记录下来。
#[test]
fn an_explicit_directory_keeps_the_parent_and_records_the_mapping() {
    let root = package("explicit");
    apply(
        &root,
        json!({"action":"add","fields":{"module":"panel","kind":"Panel","needs_registry":true},"apply":true}),
    )
    .expect("Panel is created");
    apply(
        &root,
        json!({"action":"add","language":"cpp","parent":"root/panel",
               "file":"components/area/area.cpp",
               "fields":{"module":"area","kind":"Area","needs_registry":true},"apply":true}),
    )
    .expect("Area is created");
    assert!(
        root.join("components/area/area.cpp").is_file(),
        "the declaration is where the request put it"
    );
    let faces = faces(&root);
    let panel = view(&faces, "root/panel");
    let area = view(&faces, "root/panel/Area");
    assert_eq!(
        area.parent, panel.id,
        "the physical location did not change the registration parent"
    );
    let map = ledger(&root);
    let entry = map
        .entry_for_source("components/area/area.cpp")
        .expect("the deviation is recorded");
    assert_eq!(entry.path, "root/panel/Area");
    assert_eq!(entry.directory.as_deref(), Some("components/area"));
    assert_eq!(
        area.object_id("island"),
        entry.stable,
        "the recorded object is the one the tree derives"
    );
    let _ = std::fs::remove_dir_all(root);
}

/// A child of the island inherits the island's directory with no per-file registration, and a nested
/// override forms its own mapping that covers only its own subtree.
/// 孤岛的子面继承孤岛目录，无需逐文件登记；而嵌套覆盖形成自己的映射，只覆盖自己的子树。
#[test]
fn an_island_is_inherited_and_a_nested_override_forms_its_own_mapping() {
    let root = island();
    assert!(
        root.join("components/area/object/gauge/gauge.cpp")
            .is_file(),
        "Gauge was placed under Area's effective directory"
    );
    let faces = faces(&root);
    let area = view(&faces, "root/panel/Area");
    assert_eq!(
        view(&faces, "root/panel/Area/Gauge").parent,
        area.id,
        "the inherited child hangs under Area"
    );

    apply(
        &root,
        json!({"action":"add","language":"cpp","parent":"root/panel/Area",
               "file":"components/gauge2/gauge2.cpp",
               "fields":{"module":"gauge2","kind":"Gauge2"},"apply":true}),
    )
    .expect("the nested override is created");
    let map = ledger(&root);
    assert_eq!(
        map.owner_of_directory("components/gauge2")
            .expect("one owner")
            .expect("the nested override")
            .path,
        "root/panel/Area/Gauge2"
    );
    assert_eq!(
        map.owner_of_directory("components/area/object/reading")
            .expect("one owner")
            .expect("the island still covers what it holds")
            .path,
        "root/panel/Area"
    );
    let _ = std::fs::remove_dir_all(root);
}

/// An `add` that names a file inside an island but no parent takes the island's object as its parent.
/// 只点名孤岛目录里的文件、不点名父级的 `add` 会把孤岛的对象当作父级。
#[test]
fn an_add_in_an_island_directory_infers_the_parent() {
    let root = island();
    apply(
        &root,
        json!({"action":"add","language":"cpp",
               "file":"components/area/object/reading/reading.cpp",
               "fields":{"module":"reading","kind":"Reading"},"apply":true}),
    )
    .expect("the island decides the parent");
    let faces = faces(&root);
    let area = view(&faces, "root/panel/Area");
    let reading = view(&faces, "root/panel/Area/Reading");
    assert_eq!(
        reading.parent, area.id,
        "the parent came from the mapped directory, not from the root"
    );
    let _ = std::fs::remove_dir_all(root);
}

/// A physical move keeps the persistent object identity, both source instances stay resolvable, and
/// an explicit mapping follows the file instead of describing a place it left.
/// 物理搬动保持持续对象身份，两个源码实例都仍可解析，而显式映射跟随文件，而不是描述它离开的地方。
#[test]
fn a_physical_move_keeps_the_object_and_the_mapping_follows() {
    let root = island();
    let before = faces(&root);
    let area = view(&before, "root/panel/Area").clone();
    let stable = area.object_id("island");
    let old_source = "components/area/area.cpp";

    apply(
        &root,
        json!({"action":"move","node":"root/panel/Area","to":"root/panel",
               "name":"area_moved","apply":true}),
    )
    .expect("the move applies");

    let after = faces(&root);
    let moved = view(&after, "root/panel/Area");
    assert_eq!(
        moved.path, area.path,
        "a physical move does not change the logical path"
    );
    assert_ne!(
        moved.id, area.id,
        "the source-instance identity follows the source path, and that is expected"
    );
    assert_eq!(
        moved.object_id("island"),
        stable,
        "the persistent object identity survives the move"
    );
    let map = ledger(&root);
    let entry = map.entry(stable).expect("the object is one entry, not two");
    assert_eq!(
        entry.source, "components/area/area_moved.cpp",
        "a rename under the same parent stays in the island"
    );
    assert_eq!(
        entry.directory.as_deref(),
        Some("components/area"),
        "the explicit mapping still names the directory the file is in"
    );
    assert!(
        map.entry_for_source(old_source).is_some(),
        "the old instance still resolves to this object"
    );
    assert_eq!(
        map.entries().len(),
        1,
        "only Area deviated from the default layout, so only Area has a mapping: {map:?}"
    );
    let _ = std::fs::remove_dir_all(root);
}

/// A link inside the package that points outside it may not widen the **write** set: the reference
/// rewrite enumerates sources through the shared scope, so the move refuses instead of editing a file
/// nobody declared.
/// 包内指向包外的链接不得扩大**写入**范围：引用改写经共同来源范围枚举，因此搬动会拒绝，而不是去改一个
/// 没人声明过的文件。
#[test]
fn an_undeclared_link_cannot_widen_the_write_set() {
    let root = island();
    let outside = std::env::temp_dir().join(format!(
        "xirang-island-outside-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    let _ = std::fs::remove_dir_all(&outside);
    std::fs::create_dir_all(&outside).expect("outside dir");
    std::fs::write(
        outside.join("consumer.rs"),
        "use crate::xirang_external::native__area_Area;\n",
    )
    .expect("outside file");
    #[cfg(unix)]
    std::os::unix::fs::symlink(&outside, root.join("linked")).expect("symlink");
    let error = apply(
        &root,
        json!({"action":"move","node":"root/panel/Area","to":"root/panel",
               "name":"area_moved","apply":true}),
    )
    .expect_err("an undeclared link is not a source this move may rewrite");
    assert!(error.contains("links to"), "{error}");
    assert!(error.contains("declare that root"), "{error}");
    assert!(
        root.join("components/area/area.cpp").is_file(),
        "the move wrote nothing"
    );
    let _ = std::fs::remove_dir_all(root);
    let _ = std::fs::remove_dir_all(outside);
}

/// The same project at another absolute path keeps every identity and every mapping: the ledger is
/// spelled package-relative, so where the checkout sits is not part of the project.
/// 同一个项目在另一个绝对路径下保持每个身份与每个映射：台账是包内相对拼写，因此检出放在哪儿不属于项目。
#[test]
fn the_same_project_elsewhere_keeps_every_identity_and_mapping() {
    let root = island();
    let elsewhere = std::env::temp_dir().join(format!(
        "xirang-island-relocated-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    let _ = std::fs::remove_dir_all(&elsewhere);
    // The write path starts a background index refresh; copying while it rewrites a file can meet a
    // path that vanished between `read_dir` and `copy`. Wait for the refresh, then copy sources only.
    // 写入路径会启动后台索引刷新；边刷新边复制会在 `read_dir` 与 `copy` 之间撞到一个刚消失的路径。
    // 先等刷新结束，再只复制源码。
    assert!(
        crate::mcp::index::wait_until_idle(&root, std::time::Duration::from_secs(30)),
        "the index refresh a write starts must finish"
    );
    copy_tree(&root, &elsewhere);

    let here = faces(&root);
    let there = faces(&elsewhere);
    assert_eq!(here.len(), there.len());
    for (left, right) in here.iter().zip(there.iter()) {
        assert_eq!(left.path, right.path, "the logical tree is the same");
        assert_eq!(left.id, right.id, "and so is every identity");
    }
    assert_eq!(
        ledger(&root),
        ledger(&elsewhere),
        "the mapping is package-relative, so relocating changes nothing"
    );
    let _ = std::fs::remove_dir_all(root);
    let _ = std::fs::remove_dir_all(elsewhere);
}

/// The status line reports the mappings the ledger holds, and names a mapping the tree no longer
/// follows instead of staying quiet about it.
/// 状态行报告台账持有的映射，并点名一个树已不再遵守的映射，而不是对它保持沉默。
#[test]
fn the_status_line_reports_mappings_and_names_drift() {
    let root = island();
    // A mapped **leaf**: a hand move of a face with children would also leave a stale `.under(...)`,
    // and then the tree is not readable at all — a different fact from drift.
    // 一个被映射的**叶子**：手动搬动一个有子面的面还会留下过期的 `.under(...)`，那时树根本不可读——那是
    // 与漂移不同的事实。
    apply(
        &root,
        json!({"action":"add","language":"cpp","parent":"root/panel",
               "file":"components/zone/zone.cpp",
               "fields":{"module":"zone","kind":"Zone"},"apply":true}),
    )
    .expect("Zone is created in its own directory");
    let (line, count) = crate::mcp::faces::registered_faces_line(&root);
    assert_eq!(count, Some(4), "{line}");
    assert!(
        line.contains("2 object(s) have an explicit directory mapping"),
        "{line}"
    );
    assert!(!line.contains("not followed"), "{line}");

    // Move the declaration by hand, the way a person would with `mv`.
    // 像人用 `mv` 那样手动搬动那条声明。
    std::fs::create_dir_all(root.join("elsewhere")).expect("dir");
    std::fs::rename(
        root.join("components/zone/zone.cpp"),
        root.join("elsewhere/zone.cpp"),
    )
    .expect("hand move");
    // The tree has to be readable before a drift row can be reported; when it is not, say why instead
    // of calling the line "unknown" and losing the reason.
    // 树必须可读才谈得上报告漂移；不可读时要说清原因，而不是把这一行叫"unknown"、把理由丢掉。
    if let Err(error) = crate::build_method::face_views(&root, "island") {
        panic!("the hand-moved tree must still read: {error}");
    }
    let (line, _) = crate::mcp::faces::registered_faces_line(&root);
    assert!(
        line.contains("mapping(s) are not followed"),
        "a mapping nobody follows is reported: {line}"
    );
    assert!(line.contains("elsewhere/zone.cpp"), "{line}");
    let _ = std::fs::remove_dir_all(root);
}

/// Copy every file under `from` into `to`.
/// 把 `from` 下的每个文件复制进 `to`。
fn copy_tree(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).expect("copy destination");
    for entry in std::fs::read_dir(from).expect("fixture tree") {
        let entry = entry.expect("fixture entry");
        let name = entry.file_name();
        // Build outputs are not part of the project's identity or its ledger.
        // 构建产物不属于项目身份或台账。
        if name == "target" || name == ".git" {
            continue;
        }
        let target = to.join(&name);
        if entry.file_type().expect("fixture type").is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), target).expect("fixture file");
        }
    }
}

/// The stable id the ledger records for one source, or a failure naming the ledger.
/// 台账为某份源码记录的稳定 id；没有时给出点名台账的失败。
#[allow(dead_code)]
fn stable_of_source(root: &Path, source: &str) -> StableFaceId {
    ledger(root)
        .entry_for_source(source)
        .unwrap_or_else(|| panic!("no ledger entry for `{source}`"))
        .stable
}

/// A Rust face **can** be placed in an explicit island directory; what it cannot be is placed where
/// its module path would disagree with the file. Both halves are pinned here, because the old
/// behaviour (refusing every `file` for Rust) is exactly what this round replaced.
/// Rust 面**可以**放进显式孤岛目录；它不能的是"放成模块路径与文件不一致"。两半都在这里钉住，因为旧行为
/// （对 Rust 一律拒绝 `file`）正是本轮替换掉的东西。
#[test]
fn a_rust_face_takes_an_explicit_directory_and_refuses_one_it_cannot_honour() {
    let root = package("rust-destination");
    apply(
        &root,
        json!({"action":"add","fields":{"module":"panel","kind":"Panel","needs_registry":true},"apply":true}),
    )
    .expect("Panel is created");

    // The half that must now **work**: an explicit island directory under the source tree, with the
    // module named by the file's directory — the placement is recorded as a deviation.
    apply(
        &root,
        json!({"action":"add","parent":"root/panel","file":"src/components/reading/reading.rs",
               "fields":{"module":"reading","kind":"Reading"},"apply":true}),
    )
    .expect("a Rust face takes an explicit directory");
    assert!(
        root.join("src/components/reading/reading.rs").is_file(),
        "the file is where the request named it"
    );
    assert!(
        ledger(&root)
            .entries()
            .iter()
            .any(|entry| entry.source == "src/components/reading/reading.rs"
                && entry.directory.as_deref() == Some("src/components/reading")),
        "the placement is a recorded deviation, so a descendant inherits it"
    );

    // The second half that must now **work**: a destination outside the source tree but inside the
    // package. The placement is package-relative, the build mounts it with `#[path]`, and the
    // registered parent does not change — the walk root is not the package boundary.
    // 第二个现在**必须可用**的一半：在源码树之外、但在包之内的目的地。落点是包内相对，构建用 `#[path]`
    // 挂载它，而注册父级不变——遍历根不是包的边界。
    apply(
        &root,
        json!({"action":"add","parent":"root/panel","file":"components/extra/extra.rs",
               "fields":{"module":"extra","kind":"Extra"},"apply":true}),
    )
    .expect("a Rust face takes an explicit directory outside the walk root");
    assert!(
        root.join("components/extra/extra.rs").is_file(),
        "the island lives where the request named it"
    );
    let views = faces(&root);
    let extra = views
        .iter()
        .find(|view| view.path == "root/panel/extra")
        .expect("the derived tree names the outside island");
    assert_eq!(
        extra.source, "components/extra/extra.rs",
        "the outside island's source is package-relative, not its absolute path"
    );
    assert_eq!(
        views
            .iter()
            .find(|view| view.path == "root/panel/reading")
            .expect("the in-tree island")
            .parent,
        extra.parent,
        "both islands hang under the parent the requests named"
    );

    // The half that must still be refused: a destination whose stem is not the module its
    // directory names. The refusal names the spelling that works instead of saying the feature
    // does not exist.
    let error = apply(
        &root,
        json!({"action":"add","parent":"root/panel","file":"components/other/extra.rs",
               "fields":{"module":"other","kind":"Other"},"apply":true}),
    )
    .expect_err("a destination whose stem is not its module cannot be honoured");
    assert!(error.contains("cannot be the module"), "{error}");
    assert!(
        !root.join("components/other/extra.rs").exists(),
        "nothing was written"
    );
    assert!(
        root.join("src/panel/panel.rs").is_file(),
        "and the tree that existed is untouched"
    );
    let _ = std::fs::remove_dir_all(root);
}

/// A **logical** move returns a mapped object to the layout its new parent derives, and the ledger
/// stops carrying the deviation it no longer needs. A physical move under the same parent keeps it
/// (pinned above), so the two cases are distinguished rather than both "the mapping follows".
/// **逻辑**搬动把被映射的对象送回新父级推导出的布局，台账不再携带那个不再需要的偏离。同父级下的物理搬动
/// 保留它（上面已钉），因此两种情形被区分开，而不是都写成"映射跟着走"。
#[test]
fn a_logical_move_returns_a_mapped_object_to_the_default_layout() {
    let root = island();
    apply(
        &root,
        json!({"action":"add","language":"cpp","parent":"root/panel",
               "file":"components/zone/zone.cpp",
               "fields":{"module":"zone","kind":"Zone","needs_registry":true},"apply":true}),
    )
    .expect("Zone is created");
    apply(
        &root,
        json!({"action":"add","language":"cpp","parent":"root/panel/Area",
               "file":"components/gauge2/gauge2.cpp",
               "fields":{"module":"gauge2","kind":"Gauge2"},"apply":true}),
    )
    .expect("Gauge2 is created in its own directory");
    let before = ledger(&root);
    assert_eq!(
        before
            .entry_for_path("root/panel/Area/Gauge2")
            .and_then(|entry| entry.directory.as_deref()),
        Some("components/gauge2"),
        "the nested override is recorded"
    );

    let message = apply(
        &root,
        json!({"action":"move","node":"root/panel/Area/Gauge2","to":"root/panel/Zone",
               "name":"gauge2","apply":true}),
    )
    .expect("the logical move applies");
    assert!(
        message.contains("mapping cleared"),
        "the reply says the deviation is gone: {message}"
    );
    let after = ledger(&root);
    let entry = after
        .entry_for_path("root/panel/Zone/Gauge2")
        .expect("the object followed its new parent");
    assert_eq!(entry.directory, None, "the mapping is gone, not moved");
    assert!(
        root.join("components/zone/object/gauge2/gauge2.cpp")
            .is_file(),
        "the object landed at the layout its new parent derives"
    );
    let faces = faces(&root);
    let zone = view(&faces, "root/panel/Zone");
    assert_eq!(
        view(&faces, "root/panel/Zone/Gauge2").parent,
        zone.id,
        "and its parent is the face the move named"
    );
    let _ = std::fs::remove_dir_all(root);
}

/// A plan binds the **relation** it produces, not only the destination file. Two parents can share
/// one directory, so a move to either lands at the same path while the logical path (the relation)
/// differs; a plan made for one must not land the other.
/// 计划绑定它产出的**关系**，而不只是目的地文件。两个父级可以共用一个目录，因此搬到任一个都落在同一路径，
/// 而逻辑路径（关系）不同；为其中一个做的计划不得落下另一个。
#[test]
fn a_plan_is_refused_when_only_the_relation_changed() {
    let root = package("relation");
    apply(
        &root,
        json!({"action":"add","fields":{"module":"panel","kind":"Panel","needs_registry":true},"apply":true}),
    )
    .expect("Panel is created");
    for (module, kind) in [("area", "Area"), ("zone", "Zone"), ("zone2", "Zone2")] {
        apply(
            &root,
            json!({"action":"add","language":"cpp","parent":"root/panel",
                   "file":format!("components/shared/{module}.cpp"),
                   "fields":{"module":module,"kind":kind,"needs_registry":true},"apply":true}),
        )
        .expect("the parent is created");
    }
    apply(
        &root,
        json!({"action":"add","language":"cpp","parent":"root/panel/Area",
               "fields":{"module":"gauge","kind":"Gauge"},"apply":true}),
    )
    .expect("Gauge is created under Area");

    let preview = apply(
        &root,
        json!({"action":"move","node":"root/panel/Area/Gauge","to":"root/panel/Zone",
               "name":"gauge","apply":false}),
    )
    .expect("the preview runs");
    let plan = preview
        .lines()
        .find_map(|line| line.trim().strip_prefix("plan "))
        .expect("the preview prints a plan")
        .trim()
        .to_owned();
    // The same file would be written for `Zone2` — the destination is identical — but the relation
    // is not. The plan must refuse, and the tree must not move.
    // 对 `Zone2` 而言要写的文件相同——目的地一致——但关系不同。计划必须拒绝，树不得移动。
    let error = apply(
        &root,
        json!({"action":"move","node":"root/panel/Area/Gauge","to":"root/panel/Zone2",
               "name":"gauge","apply":true,"plan":plan}),
    )
    .expect_err("a plan from another relation is not this move");
    assert!(error.contains("no longer describes this move"), "{error}");
    assert!(error.contains("Nothing changed"), "{error}");
    let faces = faces(&root);
    assert_eq!(
        view(&faces, "root/panel/Area/Gauge").parent,
        view(&faces, "root/panel/Area").id,
        "the object stayed under the parent it had"
    );
    let _ = std::fs::remove_dir_all(root);
}

/// The other half of the same boundary: a declared source is readable, and it is still **not
/// writable by `add`**. A link the project declared may be read through, but a plan may only write
/// inside the package; the refusal happens before the first byte, so no file is left outside.
/// 同一条边界的另一半：已声明来源可读，但它仍然**不能被 `add` 写**。项目声明的链接可以读，但计划只能写在
/// 包内；拒绝发生在第一个字节之前，因此包外不会留下文件。
#[test]
fn a_declared_source_is_readable_but_add_may_not_write_through_it() {
    use xirang_multilang::source_map::{
        BINDINGS_FILE, BINDINGS_HEADER, SOURCES_FILE, SOURCES_HEADER,
    };

    let root = package("declared-write");
    apply(
        &root,
        json!({"action":"add","fields":{"module":"panel","kind":"Panel","needs_registry":true},"apply":true}),
    )
    .expect("Panel is created");
    let outside = std::env::temp_dir().join(format!(
        "xirang-island-declared-write-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    let _ = std::fs::remove_dir_all(&outside);
    std::fs::create_dir_all(&outside).expect("outside dir");
    std::fs::create_dir_all(root.join("vendor")).expect("vendor dir");
    #[cfg(unix)]
    std::os::unix::fs::symlink(&outside, root.join("vendor/ui")).expect("symlink");
    std::fs::write(
        root.join(SOURCES_FILE),
        format!("{SOURCES_HEADER}\nvendor-ui\tlocal\tvendor/ui\n"),
    )
    .expect("declaration");
    std::fs::create_dir_all(root.join(".xirang")).expect("machine dir");
    std::fs::write(
        root.join(BINDINGS_FILE),
        format!("{BINDINGS_HEADER}\nvendor-ui\t{}\n", outside.display()),
    )
    .expect("binding");

    let error = apply(
        &root,
        json!({"action":"add","language":"cpp","parent":"root/panel",
               "file":"vendor/ui/evil.cpp",
               "fields":{"module":"evil","kind":"Evil"},"apply":true}),
    )
    .expect_err("a declaration is a read permission, not a write permission");
    assert!(
        error.contains("not a place a plan may write"),
        "the refusal names the rule: {error}"
    );
    assert!(
        !outside.join("evil.cpp").exists(),
        "nothing was written outside the package"
    );
    let _ = std::fs::remove_dir_all(root);
    let _ = std::fs::remove_dir_all(outside);
}

/// An **old source instance** is resolvable through the bridge, not only through a library call: a
/// physical move changes the instance identity, and a reference written before it must follow the
/// ledger to the face that exists now. Both the move request and the node resolver are checked.
/// **旧源码实例**能经桥解析，而不只是经库调用：物理搬动会改变实例身份，搬动之前写下的引用必须跟随台账走到
/// 现在存在的那个面。搬动请求与节点解析器都检查。
#[test]
fn an_old_source_instance_resolves_to_the_face_that_exists_now() {
    let root = island();
    let before = faces(&root);
    let area = view(&before, "root/panel/Area").clone();
    let old_id = area.id;
    apply(
        &root,
        json!({"action":"move","node":"root/panel/Area","to":"root/panel",
               "name":"area_moved","apply":true}),
    )
    .expect("the physical move applies");
    let moved = view(&faces(&root), "root/panel/Area").clone();
    assert_ne!(
        moved.id, old_id,
        "the instance identity changed, as expected"
    );

    let resolved = crate::mcp::resolve::resolve_node(&root, "island", &old_id.to_string())
        .expect("the old instance resolves");
    assert_eq!(
        resolved, moved.id,
        "the resolver followed the ledger to the current instance"
    );

    let message = apply(
        &root,
        json!({"action":"move","node":old_id.to_string(),"to":"root/panel",
               "name":"area_back","apply":true}),
    )
    .expect("a move may name the old instance");
    assert!(
        message.contains("is an old source instance of `root/panel/Area`"),
        "the reply says which instance it followed: {message}"
    );
    assert!(
        root.join("components/area/area_back.cpp").is_file(),
        "and the move happened on the current face"
    );
    let _ = std::fs::remove_dir_all(root);
}

/// A structural rename of an explicitly placed island moves the file, keeps the object, and leaves
/// both the tree and the ledger readable under the new spelling.
/// 显式落点孤岛的一次结构改名：搬文件、保对象，并让树与台账在新拼写下都仍可读。
///
/// The rename used to be refused with "generated module is outside the package source tree" (the
/// spelling was stripped against the identity base, which a package-relative spelling has no path
/// under). After that was fixed the transaction moved the file and then refused with "renamed module
/// produced no registration face": the walk finds an explicitly placed face **through the ledger**,
/// and the ledger still named the old directory. Both halves are pinned here.
/// 这次改名过去被拒成"generated module is outside the package source tree"（拼写被拿去对身份基准做
/// strip，而包内相对拼写下面没有那条路径）。修掉之后事务搬了文件、随后又以"renamed module produced no
/// registration face"拒绝：遍历是**通过台账**找到显式落点的面的，而台账还在点名旧目录。两半都钉在这里。
#[test]
fn an_island_rename_moves_the_file_and_keeps_the_object() {
    let root = package("island-rename");
    apply(
        &root,
        json!({"action":"add","fields":{"module":"panel","kind":"Panel","needs_registry":true},"apply":true}),
    )
    .expect("Panel");
    apply(
        &root,
        json!({"action":"add","parent":"root/panel",
               "file":"components/reading/reading.rs",
               "fields":{"module":"reading","kind":"Reading"},"apply":true}),
    )
    .expect("the island");

    let before = faces(&root);
    let reading = before
        .iter()
        .find(|view| view.path == "root/panel/reading")
        .expect("the island face")
        .clone();
    let panel = before
        .iter()
        .find(|view| view.path == "root/panel")
        .expect("the parent")
        .id;
    let object = stable_of_source(&root, "components/reading/reading.rs");

    apply(
        &root,
        json!({"action":"rename","node":"root/panel/reading","fields":{"module":"sensor"},"apply":true}),
    )
    .expect("an island can be renamed");

    assert!(
        root.join("components/sensor/sensor.rs").is_file(),
        "the file landed under the new module name"
    );
    assert!(
        !root.join("components/reading/reading.rs").exists(),
        "and the old file is gone"
    );

    let after = faces(&root);
    let paths = after
        .iter()
        .map(|view| view.path.as_str())
        .collect::<Vec<_>>();
    assert!(
        paths.contains(&"root/panel/sensor"),
        "the derived tree names the renamed face: {paths:?}"
    );
    let sensor = after
        .iter()
        .find(|view| view.path == "root/panel/sensor")
        .expect("the renamed face");
    assert_eq!(
        sensor.source, "components/sensor/sensor.rs",
        "the source fact is the new package-relative spelling"
    );
    assert_eq!(sensor.parent, panel, "the registered parent did not move");
    assert_ne!(
        sensor.id, reading.id,
        "a source-path hash does change when the file moves"
    );
    assert_eq!(
        sensor.module, "sensor",
        "the module chain follows the stem the walk mounts"
    );

    let map = ledger(&root);
    let entry = map
        .entry_for_path("root/panel/sensor")
        .or_else(|| map.entry(object))
        .expect("the ledger still knows the object");
    assert_eq!(entry.stable, object, "the object kept its identity");
    assert_eq!(
        entry.directory.as_deref(),
        Some("components/sensor"),
        "the explicit mapping follows the file"
    );
    assert!(
        entry.has_source("components/reading/reading.rs")
            && entry.has_source("components/sensor/sensor.rs"),
        "both source instances stay resolvable: {:?}",
        entry.instances
    );
    assert_eq!(
        entry.current_id(),
        Some(sensor.id),
        "the current instance is the one the tree reports"
    );
    let _ = std::fs::remove_dir_all(root);
}
