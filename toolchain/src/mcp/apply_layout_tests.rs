//! Add and move under a manifest library target outside `src/`.
//! `src/` 之外的清单库目标下的 add 与 move。
//!
//! These pins exist because the writer used to assume `package/src`: a face landed
//! where the build never read it, and the next call could not find its parent. The
//! layout resolver is the build's own [`xirang_toolchain::build_method::source_layout`].
//! 这些钉子存在，是因为写入方过去写死 `package/src`：面落在构建永不读取的地方，下一次调用便找不到它的
//! 父级。布局解析器就是构建自己的 [`xirang_toolchain::build_method::source_layout`]。

use std::path::PathBuf;

use crate::build_method::face_views;
use serde_json::json;

use crate::mcp::apply::apply;

/// A package whose library target lives outside `src/`, as `[lib] path = "source/lib.rs"`.
/// 一个库目标住在 `src/` 之外的包，即 `[lib] path = "source/lib.rs"`。
///
/// The target file exists, because `source_layout` refuses a named target that names
/// nothing before it walks anything.
/// 目标文件存在，因为 `source_layout` 在遍历任何东西之前就拒绝指向空处的具名目标。
fn outside_src_package(label: &str) -> (PathBuf, String) {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let name = format!("mcp-add-outside-{label}");
    let root = std::env::temp_dir().join(format!("{name}-{}-{sequence}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("source")).expect("source directory");
    std::fs::write(
        root.join("Cargo.toml"),
        format!(
            "[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n\
             [lib]\npath = \"source/lib.rs\"\n"
        ),
    )
    .expect("manifest");
    std::fs::write(root.join("source/lib.rs"), "// host entry\n").expect("library target");
    (root, name)
}

/// `apply add` writes where the build reads: under the manifest's library target,
/// not the historical `src/`, and a child add finds the parent it just wrote.
/// `apply add` 写到构建读取的地方：清单库目标之下，而不是历史性的 `src/`；子级 add 也能找到它刚写下的父级。
///
/// Red before the fix: the parent landed in `src/provider/provider.rs`, the read-only
/// view (which walks `source/`) never found it, and the child add failed with "parent
/// does not own a Registry".
/// 修前为红：父级落在 `src/provider/provider.rs`，只读视图（遍历 `source/`）永远找不到它，子级 add
/// 便以"父级不拥有注册机"失败。
#[test]
fn add_lands_under_the_manifest_library_target() {
    let (root, _) = outside_src_package("layout");
    apply(
        &root,
        &json!({"action": "add",
                "fields": {"module": "provider", "kind": "Provider", "needs_registry": true},
                "apply": true}),
    )
    .expect("the root add applies");
    assert!(
        root.join("source/provider/provider.rs").is_file(),
        "the face lands under the library target"
    );
    assert!(
        !root.join("src/provider/provider.rs").exists(),
        "and not under the historical src"
    );

    apply(
        &root,
        &json!({"action": "add", "parent": "root/provider",
                "fields": {"module": "consumer", "kind": "Consumer"},
                "apply": true}),
    )
    .expect("the child add finds the parent the build reads");
    assert!(
        root.join("source/provider/object/consumer/consumer.rs")
            .is_file(),
        "the child is placed under the target-relative parent directory"
    );
    let views = face_views(&root, "mcp-add-outside-layout").expect("the tree reads");
    assert!(
        views
            .iter()
            .any(|view| view.path == "root/provider/consumer"),
        "the derived tree names both faces: {views:?}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// A move in the same outside-`src` layout renames under the library target and
/// rewrites the re-export that spells the target-relative module path.
/// 同一套 `src/` 之外布局中的搬动在库目标下改名，并改写拼出目标相对模块路径的重导出。
#[test]
fn move_in_the_manifest_library_target_rewrites_the_reexport() {
    let (root, name) = outside_src_package("move");
    for request in [
        json!({"action": "add",
                "fields": {"module": "provider", "kind": "Provider", "needs_registry": true},
                "apply": true}),
        json!({"action": "add", "parent": "root/provider",
                "fields": {"module": "button", "kind": "Button", "needs_registry": true},
                "apply": true}),
        json!({"action": "add", "parent": "root/provider",
                "fields": {"module": "panel", "kind": "Panel", "needs_registry": true},
                "apply": true}),
    ] {
        apply(&root, &request).unwrap_or_else(|error| panic!("{request}: {error}"));
    }
    let button = face_views(&root, &name)
        .expect("the tree reads")
        .into_iter()
        .find(|view| view.path == "root/provider/button")
        .expect("the button face")
        .module;
    let library = root.join("source/lib.rs");
    let mut text = std::fs::read_to_string(&library).expect("library");
    text.push_str(&format!("pub use crate::{button} as button_api;\n"));
    std::fs::write(&library, text).expect("alias");

    apply(
        &root,
        &json!({"action": "move", "node": "root/provider/button",
                "to": "root/provider/panel", "apply": true}),
    )
    .expect("the move applies in the outside-src layout");
    assert!(
        root.join("source/provider/object/panel/button/button.rs")
            .is_file(),
        "the moved face lands under the library target"
    );
    let rewritten = std::fs::read_to_string(&library).expect("library");
    let prefix = button
        .rsplit_once("::")
        .map(|(prefix, _)| prefix)
        .expect("a module path");
    assert!(
        rewritten.contains(&format!(
            "pub use crate::{prefix}::panel::button as button_api;"
        )),
        "the re-export follows the target-relative move: {rewritten}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// A Rust face may be placed in a package directory **outside** the library target's source root:
/// the placement decides where the file lives, the generated module chain mounts it with `#[path]`,
/// and the registered parent does not change.
/// Rust 面可以放在库目标源码根**之外**的包内目录：落点决定文件住哪，生成的模块链用 `#[path]` 挂载它，
/// 而注册父级不变。
///
/// This is the failing case the F3/R9 migration is driven by: a custom `[lib] path` already moved the
/// discovery root once; a second, per-face placement must not be rejected as "outside the source
/// tree" any more.
/// 这是驱动 F3/R9 迁移的失败用例：自定义 `[lib] path` 已经把发现根搬过一次；第二个按面的落点不能再以
/// "在源码树之外"为由拒绝。
#[test]
fn a_rust_face_takes_an_explicit_directory_outside_the_library_target() {
    let (root, name) = outside_src_package("island");
    apply(
        &root,
        &json!({"action": "add", "fields": {"module": "panel", "kind": "Panel", "needs_registry": true},
                "apply": true}),
    )
    .expect("the parent face is created under the library target");
    apply(
        &root,
        &json!({"action": "add", "parent": "root/panel",
                "file": "components/reading/reading.rs",
                "fields": {"module": "reading", "kind": "Reading"}, "apply": true}),
    )
    .expect("a child may live outside the discovery root, inside the package");
    assert!(
        root.join("components/reading/reading.rs").is_file(),
        "the child landed where the request named it"
    );
    let views = face_views(&root, &name).expect("the tree reads");
    let panel_view = views
        .iter()
        .find(|view| view.path == "root/panel")
        .expect("the parent face");
    assert_eq!(
        panel_view.module, "panel",
        "module chain under the custom lib target"
    );
    assert_eq!(
        panel_view.source, "source/panel/panel.rs",
        "record spelling under the custom lib target"
    );
    let reading = views
        .iter()
        .find(|view| view.path == "root/panel/reading")
        .expect("the derived tree names the island child");
    assert_eq!(
        reading.source, "components/reading/reading.rs",
        "the source fact is package-relative, not absolute"
    );
    let island = std::fs::read_to_string(root.join("components/reading/reading.rs"))
        .expect("the island source");
    assert!(
        island.contains(".under(crate::panel::NODE_ID)"),
        "the island names its parent by the generated module chain, not by the record spelling \
         below it:\n{island}"
    );

    // An edit rewrites the declaration from the values the reader parsed, and the reader must carry
    // the **declared** module address: under a custom `[lib] path` the parent's record spelling
    // (`source/panel/panel.rs`) is not its module (`panel`), so re-deriving it emitted
    // `crate::source::panel::NODE_ID` and the generated host stopped compiling.
    // 一次编辑会按读取器解析出的取值重写声明，而读取器必须带着**声明过的**模块地址：自定义
    // `[lib] path` 下父级的记录拼写（`source/panel/panel.rs`）不是它的模块（`panel`），重新推导会发射
    // `crate::source::panel::NODE_ID`，生成的宿主于是编译不过。
    apply(
        &root,
        &json!({"action": "edit", "node": "root/panel/reading",
                "fields": {"summary_en": "an edited island"}, "apply": true}),
    )
    .expect("the island is editable");
    let edited = std::fs::read_to_string(root.join("components/reading/reading.rs"))
        .expect("the island source");
    assert!(
        edited.contains(".under(crate::panel::NODE_ID)"),
        "the edit wrote the parent's module chain back verbatim:\n{edited}"
    );
    assert!(
        edited.contains("an edited island"),
        "and it did write the field the edit named"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// A default-layout face and an island face in one package read back **every** parent after three
/// creates, and an edit of the island still finds it.
/// 一个包里的默认布局面与孤岛面在连续三次新建之后仍读回**每一个**父引用，而对孤岛面的一次编辑仍能
/// 找到它。
///
/// The reader used to resolve its own layout from the scan root, so a custom `[lib] path` made the
/// parent spelling come back as an absolute path or as the wrong module; three creates then failed on
/// the face the previous call had written. This is the regression that keeps them together.
/// 读取方过去从扫描根重新解析布局，于是自定义 `[lib] path` 会让父拼写回成绝对路径或错误模块；三次新建
/// 之后就因上一次写下的面失败。这条回归把它们钉在一起。
#[test]
fn a_default_face_and_an_island_read_back_every_parent_after_three_creates() {
    let (root, name) = outside_src_package("mixed");
    apply(
        &root,
        &json!({"action": "add", "fields": {"module": "provider", "kind": "Provider",
                "needs_registry": true}, "apply": true}),
    )
    .expect("the default parent");
    apply(
        &root,
        &json!({"action": "add", "parent": "root/provider",
                "file": "components/reading/reading.rs",
                "fields": {"module": "reading", "kind": "Reading"}, "apply": true}),
    )
    .expect("the island child");
    apply(
        &root,
        &json!({"action": "add", "parent": "root/provider",
                "fields": {"module": "button", "kind": "Button"}, "apply": true}),
    )
    .expect("a default child after the island; this read-back is the regression");

    let views = face_views(&root, &name).expect("the tree reads");
    let mut paths = views
        .iter()
        .map(|view| view.path.clone())
        .collect::<Vec<_>>();
    paths.sort();
    assert_eq!(
        paths,
        vec![
            "root/provider",
            "root/provider/button",
            "root/provider/reading"
        ],
        "every face is under the parent the requests named"
    );
    let reading = views
        .iter()
        .find(|view| view.path == "root/provider/reading")
        .expect("the island face");
    assert_eq!(reading.module, "reading");
    assert_eq!(reading.source, "components/reading/reading.rs");

    // An edit goes through the same read-back: it must still find the island's file and module.
    apply(
        &root,
        &json!({"action": "edit", "node": "root/provider/reading",
                "fields": {"summary_en": "the island reading"}, "apply": true}),
    )
    .expect("the island face is editable through the same reader");
    assert!(
        std::fs::read_to_string(root.join("components/reading/reading.rs"))
            .expect("the island file")
            .contains("the island reading"),
        "the edit reached the island file"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// A package with the historical `src/` layout: no `[lib]` table, so the scan root and the identity
/// base are both `src/`.
/// 一个采用历史 `src/` 布局的包：没有 `[lib]` 表，因此扫描根与身份基准都是 `src/`。
fn src_layout_package(label: &str) -> (PathBuf, String) {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let name = format!("mcp-add-src-{label}");
    let root = std::env::temp_dir().join(format!("{name}-{}-{sequence}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("src")).expect("source directory");
    std::fs::write(
        root.join("Cargo.toml"),
        format!(
            "[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[workspace]\n"
        ),
    )
    .expect("manifest");
    std::fs::write(root.join("src/lib.rs"), "// host entry\n").expect("library target");
    (root, name)
}

/// In the historical `src/` layout, an island outside `src/` is read back by its **package-relative**
/// spelling — the identity the discovery walk derives — and an `edit` on it reaches the file.
/// 在历史 `src/` 布局下，`src/` 之外的孤岛按其**包内相对**拼写读回——也就是发现遍历推导出的身份——
/// 而对它的 `edit` 能落到文件上。
///
/// Red before the fix: the read-back derived the spelling from the identity base alone, so a file
/// outside it came back as its absolute path. The loaded registry then held the island under a
/// different identity than the tree reported, and the edit failed with `node ... is not registered`.
/// 修前为红：读取方只从身份基准推导拼写，因此基准之外的文件回成了它的绝对路径。已加载注册机随后把孤岛
/// 放在与树报告的身份不同的那个身份下，编辑便以 `node ... is not registered` 失败。
#[test]
fn an_island_outside_src_reads_back_by_its_package_relative_spelling() {
    let (root, name) = src_layout_package("island");
    apply(
        &root,
        &json!({"action": "add", "fields": {"module": "panel", "kind": "Panel",
                "needs_registry": true}, "apply": true}),
    )
    .expect("the default parent");
    apply(
        &root,
        &json!({"action": "add", "parent": "root/panel",
                "file": "components/reading/reading.rs",
                "fields": {"module": "reading", "kind": "Reading"}, "apply": true}),
    )
    .expect("a child may live outside the src walk root, inside the package");

    let views = face_views(&root, &name).expect("the tree reads");
    let reading = views
        .iter()
        .find(|view| view.path == "root/panel/reading")
        .expect("the derived tree names the island child");
    assert_eq!(
        reading.source, "components/reading/reading.rs",
        "the identity spelling is package-relative, not absolute"
    );

    // The registry the editor validates against must hold the same identity the tree reports.
    // 编辑器据以校验的注册机必须持有树所报告的那个身份。
    let registry =
        crate::mcp::apply::load_registry(&root, &name).expect("the package's faces load");
    let face = registry
        .find_registry(reading.id)
        .expect("the loaded registry holds the identity the tree reports");
    assert_eq!(face.source.file, "components/reading/reading.rs");

    apply(
        &root,
        &json!({"action": "edit", "node": "root/panel/reading",
                "fields": {"summary_en": "the island reading"}, "apply": true}),
    )
    .expect("the island face is editable through the same reader");
    assert!(
        std::fs::read_to_string(root.join("components/reading/reading.rs"))
            .expect("the island file")
            .contains("the island reading"),
        "the edit reached the island file"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// An island's directory is a **discovery root**: descendants that inherit it are walked, a nested
/// override keeps a directory of its own, and the build plan mounts all of them.
/// 孤岛的目录就是**发现根**：继承它的后代被遍历，嵌套覆盖保留自己的目录，而生成计划把全部挂上。
///
/// The ledger used to contribute only the mapped **file**, so an inherited child (`object/gauge`) was
/// invisible to the build even though authoring placed it there — the tree the author edited and the
/// tree the compiler saw were different trees.
/// 台账过去只贡献被映射的**那个文件**，因此继承的子面（`object/gauge`）尽管被创作放在那里，构建却看不见
/// ——作者编辑的树与编译器看见的树不是同一棵。
#[test]
fn an_island_is_a_discovery_root_and_its_inherited_descendants_are_planned() {
    let (root, name) = src_layout_package("island-tree");
    for request in [
        json!({"action": "add", "fields": {"module": "panel", "kind": "Panel",
                "needs_registry": true}, "apply": true}),
        json!({"action": "add", "parent": "root/panel",
                "file": "components/reading/reading.rs",
                "fields": {"module": "reading", "kind": "Reading",
                           "needs_registry": true}, "apply": true}),
        // Inherits the island's directory: no `file` names one.
        json!({"action": "add", "parent": "root/panel/reading",
                "fields": {"module": "gauge", "kind": "Gauge",
                           "needs_registry": true}, "apply": true}),
        // A nested override: its own mapped directory, outside the island tree.
        json!({"action": "add", "parent": "root/panel/reading/gauge",
                "file": "components/needle/needle.rs",
                "fields": {"module": "needle", "kind": "Needle"}, "apply": true}),
    ] {
        apply(&root, &request).unwrap_or_else(|error| panic!("{request}: {error}"));
    }
    for path in [
        "components/reading/reading.rs",
        "components/reading/object/gauge/gauge.rs",
        "components/needle/needle.rs",
    ] {
        assert!(root.join(path).is_file(), "the product wrote `{path}`");
    }

    let views = face_views(&root, &name).expect("the tree reads");
    let mut rows = views
        .iter()
        .map(|view| format!("{}|{}|{}", view.path, view.module, view.source))
        .collect::<Vec<_>>();
    rows.sort();
    let mut expected = vec![
        "root/panel|panel|panel/panel.rs",
        "root/panel/reading|reading|components/reading/reading.rs",
        "root/panel/reading/gauge|reading::object::gauge|components/reading/object/gauge/gauge.rs",
        "root/panel/reading/gauge/needle|needle|components/needle/needle.rs",
    ];
    expected.sort();
    assert_eq!(
        rows, expected,
        "the inherited child is mounted under the island's chain and the override at its own stem"
    );

    let out = root.join("out");
    crate::build_method::run_for(&root, &out, &name).expect("the host build succeeds");
    let generated = std::fs::read_to_string(out.join("generated_lib.rs")).expect("the plan");
    assert_eq!(
        generated.matches("StaticFace::new(").count(),
        4,
        "the plan carries the parent, the island, the inherited child and the override:\n{generated}"
    );
    for spelling in [
        "reading::REGISTRATION",
        "reading::object::gauge::REGISTRATION",
        "needle::REGISTRATION",
    ] {
        assert!(
            generated.contains(spelling),
            "the plan addresses `{spelling}`:\n{generated}"
        );
    }
    assert!(
        !generated.contains("components::reading"),
        "no module path is derived from the island's physical directories:\n{generated}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// Moving an island to the logical root lands it **under the walk root**, so the reply, the ledger and
/// the next discovery all agree, and the mapping is cleared only because default discovery now covers it.
/// 把孤岛搬到逻辑根会落在**遍历根之下**，因此回复、台账与下一次发现三者一致，而映射被清除的唯一理由是
/// 默认发现如今覆盖了它。
#[test]
fn an_island_move_to_the_root_lands_in_the_default_walk_and_clears_the_mapping() {
    use xirang_multilang::object_map::ObjectMap;
    let (root, name) = src_layout_package("island-move-root");
    apply(
        &root,
        &json!({"action": "add", "fields": {"module": "panel", "kind": "Panel",
                "needs_registry": true}, "apply": true}),
    )
    .expect("panel");
    apply(
        &root,
        &json!({"action": "add", "parent": "root/panel",
                "file": "components/reading/reading.rs",
                "fields": {"module": "reading", "kind": "Reading"}, "apply": true}),
    )
    .expect("the island");
    let before = face_views(&root, &name)
        .expect("the tree reads")
        .into_iter()
        .find(|view| view.path == "root/panel/reading")
        .expect("the island face");
    let object = ObjectMap::read_from(&root)
        .expect("the ledger")
        .entry_for_source("components/reading/reading.rs")
        .expect("the island's object")
        .stable;

    apply(
        &root,
        &json!({"action": "move", "node": "root/panel/reading", "to": "root", "apply": true}),
    )
    .expect("an island can move to the root");

    assert!(
        root.join("src/reading/reading.rs").is_file(),
        "the file landed under the walk root, not at the package root"
    );
    assert!(
        !root.join("components/reading/reading.rs").exists(),
        "and it is not still in the island directory"
    );
    let after = face_views(&root, &name).expect("the tree reads");
    let moved = after
        .iter()
        .find(|view| view.path == "root/reading")
        .expect("the moved face");
    assert_eq!(
        moved.source, "reading/reading.rs",
        "record spelling below `src/`"
    );
    assert_eq!(moved.module, "reading");
    assert_eq!(
        moved.id,
        crate::mcp::resolve::resolve_node(&root, &name, &before.id.to_string())
            .expect("the old instance resolves"),
        "the ledger connects the two instances"
    );
    assert!(
        ObjectMap::read_from(&root)
            .expect("the ledger")
            .entry_for_source("src/reading/reading.rs")
            .expect("the moved object")
            .directory
            .is_none(),
        "the mapping is cleared because the destination is under the default walk"
    );
    let ledger = ObjectMap::read_from(&root).expect("the ledger");
    let entry = ledger.entry(object).expect("one object");
    assert_eq!(entry.path, "root/reading");
    assert_eq!(
        entry.instances.len(),
        2,
        "both instances stay on one object"
    );
    let out = root.join("out");
    crate::build_method::run_for(&root, &out, &name).expect("the host build succeeds");
    let generated = std::fs::read_to_string(out.join("generated_lib.rs")).expect("the plan");
    assert_eq!(generated.matches("StaticFace::new(").count(), 2);
    let _ = std::fs::remove_dir_all(&root);
}

/// Moving a default face **into** an island records no per-face deviation: the island's own mapping
/// already covers that directory, and the destination is where the last step of discovery looks.
/// 把默认面搬**进**孤岛不会记下逐面偏离：孤岛自己的映射已经覆盖那个目录，而目的地正是发现的最后一步
/// 会看的地方。
#[test]
fn a_face_moved_into_an_island_reuses_the_islands_mapping() {
    use xirang_multilang::object_map::ObjectMap;
    let (root, name) = src_layout_package("island-move-in");
    for request in [
        json!({"action": "add", "fields": {"module": "panel", "kind": "Panel",
                "needs_registry": true}, "apply": true}),
        json!({"action": "add", "parent": "root/panel",
                "file": "components/reading/reading.rs",
                "fields": {"module": "reading", "kind": "Reading",
                           "needs_registry": true}, "apply": true}),
        json!({"action": "add", "parent": "root", "fields": {"module": "button", "kind": "Button"},
                "apply": true}),
    ] {
        apply(&root, &request).unwrap_or_else(|error| panic!("{request}: {error}"));
    }
    apply(
        &root,
        &json!({"action": "move", "node": "root/button", "to": "root/panel/reading", "apply": true}),
    )
    .expect("a default face can move into an island");

    assert!(
        root.join("components/reading/button/button.rs").is_file(),
        "the destination is the parent's directory plus the leaf"
    );
    let views = face_views(&root, &name).expect("the tree reads");
    let button = views
        .iter()
        .find(|view| view.path == "root/panel/reading/button")
        .expect("the moved face");
    assert_eq!(
        button.source, "components/reading/button/button.rs",
        "its spelling is package-relative"
    );
    assert_eq!(
        button.module, "reading::button",
        "the island's chain grows one segment"
    );
    let ledger = ObjectMap::read_from(&root).expect("the ledger");
    let button_entry = ledger
        .entry_for_source("components/reading/button/button.rs")
        .expect("the moved object is in the ledger with both instances");
    assert!(
        button_entry.directory.is_none(),
        "no per-face deviation is needed inside a directory the island already maps"
    );
    let out = root.join("out");
    crate::build_method::run_for(&root, &out, &name).expect("the host build succeeds");
    let generated = std::fs::read_to_string(out.join("generated_lib.rs")).expect("the plan");
    assert_eq!(generated.matches("StaticFace::new(").count(), 3);
    assert!(
        generated.contains("reading::button::REGISTRATION"),
        "the moved face is mounted inside the island:\n{generated}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// The combined regression the maintainer asked for: an island, an inherited child, a nested override,
/// then the island moves — every object survives, parents follow the requests, and the ledger's
/// logical paths and sources describe what is on disk.
/// 维护者要求的组合回归：孤岛、继承子面、嵌套覆盖，然后搬动孤岛——每个对象都存活，父级符合请求，台账的
/// 逻辑路径与源码描述磁盘上的事实。
#[test]
fn moving_an_island_carries_inherited_children_and_nested_overrides() {
    use xirang_multilang::object_map::ObjectMap;
    let (root, name) = src_layout_package("island-subtree-move");
    for request in [
        json!({"action": "add", "fields": {"module": "panel", "kind": "Panel",
                "needs_registry": true}, "apply": true}),
        json!({"action": "add", "parent": "root/panel",
                "file": "components/reading/reading.rs",
                "fields": {"module": "reading", "kind": "Reading",
                           "needs_registry": true}, "apply": true}),
        json!({"action": "add", "parent": "root/panel/reading",
                "fields": {"module": "gauge", "kind": "Gauge",
                           "needs_registry": true}, "apply": true}),
        json!({"action": "add", "parent": "root/panel/reading/gauge",
                "file": "components/needle/needle.rs",
                "fields": {"module": "needle", "kind": "Needle"}, "apply": true}),
    ] {
        apply(&root, &request).unwrap_or_else(|error| panic!("{request}: {error}"));
    }
    apply(
        &root,
        &json!({"action": "move", "node": "root/panel/reading", "to": "root", "apply": true}),
    )
    .expect("the island moves to the root with its subtree");

    for path in [
        "src/reading/reading.rs",
        "src/reading/object/gauge/gauge.rs",
        "components/needle/needle.rs",
    ] {
        assert!(
            root.join(path).is_file(),
            "`{path}` is on disk after the move"
        );
    }
    let views = face_views(&root, &name).expect("the tree reads");
    let mut rows = views
        .iter()
        .map(|view| format!("{}|{}|{}", view.path, view.module, view.source))
        .collect::<Vec<_>>();
    rows.sort();
    let mut expected = vec![
        "root/panel|panel|panel/panel.rs",
        "root/reading|reading|reading/reading.rs",
        "root/reading/gauge|reading::object::gauge|reading/object/gauge/gauge.rs",
        "root/reading/gauge/needle|needle|components/needle/needle.rs",
    ];
    expected.sort();
    assert_eq!(rows, expected, "every face followed its parent's new path");

    let ledger = ObjectMap::read_from(&root).expect("the ledger");
    let island = ledger
        .entry_for_source("src/reading/reading.rs")
        .expect("the island object moved its source");
    assert_eq!(island.path, "root/reading");
    assert!(
        island.directory.is_none(),
        "the destination is under the default walk"
    );
    assert_eq!(
        island.instances.len(),
        2,
        "both instances stay on the object"
    );
    let needle = ledger
        .entry_for_source("components/needle/needle.rs")
        .expect("the nested override keeps its object");
    assert_eq!(
        needle.path, "root/reading/gauge/needle",
        "a logical descendant's ledger path follows the parent chain although its file did not move"
    );
    assert_eq!(needle.directory.as_deref(), Some("components/needle"));
    assert_eq!(
        needle.current_id(),
        Some(
            views
                .iter()
                .find(|view| view.path == "root/reading/gauge/needle")
                .expect("the override face")
                .id
        ),
        "its identity is unchanged and still the tree's"
    );

    let out = root.join("out");
    crate::build_method::run_for(&root, &out, &name).expect("the host build succeeds");
    let generated = std::fs::read_to_string(out.join("generated_lib.rs")).expect("the plan");
    assert_eq!(generated.matches("StaticFace::new(").count(), 4);
    for spelling in [
        "reading::REGISTRATION",
        "reading::object::gauge::REGISTRATION",
        "needle::REGISTRATION",
    ] {
        assert!(generated.contains(spelling), "the plan mounts `{spelling}`");
    }
    let _ = std::fs::remove_dir_all(&root);
}
