//! Pins for the partition view both authoring surfaces read (audit `M7`, P3.6).
//! 两个创作面都读的那份分区视图的钉子（审计 `M7`，P3.6）。

use std::fs;
use std::path::{Path, PathBuf};

use super::{OnDisk, view};

/// A throwaway host package with a declaration and the records a plan reads.
/// 一个一次性宿主包：带声明，以及规划要读的那些记录。
///
/// The records are the ones `xirang check` publishes (`pruning_manifest.tsv`), written here in the
/// shape the reader wants, so the fixture never needs a build.
/// 记录就是 `xirang check` 发布的那一份（`pruning_manifest.tsv`），按读取器要的形状写下，因此夹具不必构建。
fn host(label: &str, declaration: Option<&str>) -> PathBuf {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let area = std::env::temp_dir()
        .join("xirang-scratch")
        .join(module_path!().replace("::", "-"))
        .join(format!(
            "xirang-partition-view-{label}-{}-{sequence}",
            std::process::id()
        ));
    let _ = std::fs::remove_dir_all(&area);
    // The host is one level down: a ghost is the host's **sibling**, so a host sitting directly in
    // the temp directory would plan its packages into `/tmp` — where a leftover from any earlier run
    // could make this fixture read somebody else's directory (measured: it did).
    // 宿主在下一层：幽灵是宿主的**同级**，因此直接坐在临时目录里的宿主的包会被规划进 `/tmp`——那里任何一次
    // 早先运行留下的东西都可能让本夹具读到别人的目录（实测：确实读到了）。
    let root = area.join("host");
    fs::create_dir_all(root.join("src/panel/frame/widget")).expect("the host's sources");
    // A lib target, because the package's name *is* the identity namespace and cargo has to be able
    // to read it (a manifest with no target is a `cargo metadata` failure, not a package).
    // 一个 lib target，因为包名**就是**身份命名空间、而 cargo 得读得到它（没有 target 的清单是
    // `cargo metadata` 失败，而不是一个包）。
    fs::write(
        root.join("src/lib.rs"),
        "xirang_toolchain::run_method::host!();\n",
    )
    .expect("the entry");
    for (file, kind, parent) in [
        ("src/panel/panel.rs", "Panel", "crate::ROOT_NODE_ID"),
        ("src/panel/frame/frame.rs", "Frame", "crate::panel::NODE_ID"),
        (
            "src/panel/frame/widget/widget.rs",
            "Widget",
            "crate::panel::frame::NODE_ID",
        ),
    ] {
        fs::write(
            root.join(file),
            format!("crate::root_object! {{ kind: {kind}, parent: {parent}, }}\n"),
        )
        .expect("a face file");
    }
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"host\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n\
         [dependencies]\nxirang-toolchain = { path = \"../toolchain\" }\n",
    )
    .expect("the host manifest");
    if let Some(declaration) = declaration {
        fs::write(root.join("add_crates.rs"), declaration).expect("the declaration");
    }
    let out = root.join("target/xirang/out");
    fs::create_dir_all(&out).expect("the records directory");
    let id = |relative: &str, kind: &str| {
        crate::build_method::registry_identity::package_node_id(relative, kind).to_string()
    };
    let rows = [
        (id("panel/panel.rs", "Panel"), "panel/panel.rs"),
        (id("panel/frame/frame.rs", "Frame"), "panel/frame/frame.rs"),
        (
            id("panel/frame/widget/widget.rs", "Widget"),
            "panel/frame/widget/widget.rs",
        ),
    ];
    let mut manifest = String::from(
        "# node\tsource\tsymbol\tpath\tkind\tregistry_name\tparent\tsource_hash\tfields\tcalls\t\
         parent_node\towns_registry\tlogical_path\n",
    );
    for (node, source) in rows {
        manifest.push_str(&format!(
            "{node}\t{source}\tX\t-\tX\tX\t-\t-\t-\t-\t-\tfalse\t-\n"
        ));
    }
    fs::write(out.join("pruning_manifest.tsv"), manifest).expect("the pruning manifest");
    root
}

/// The declaration the fixtures use: `panel::frame` becomes a crate of its own.
/// 夹具使用的声明：`panel::frame` 自己成为一个 crate。
const DECLARATION: &str = r#"use xirang_toolchain::run_method::{Crate, Shape};

pub const SHAPE: Shape = Shape {
    package_prefix: "host",
    crates: &[Crate::named("widgets").at(&[crate::panel::frame::SUBTREE])],
};
"#;

/// A host with no declaration is one crate, and the view says so by saying nothing.
/// 没有声明的宿主就是一个 crate，而视图以"什么都不说"来表达这一点。
#[test]
fn a_host_without_a_declaration_has_no_partition() {
    let root = host("absent", None);
    assert!(view(&root).expect("reads").is_none());
    let _ = std::fs::remove_dir_all(root);
}

/// The view describes what the declaration plans, in both shapes, and what is on disk.
/// 视图描述声明规划出来的东西（两种形状），以及磁盘上已经有什么。
#[test]
fn the_view_describes_both_shapes_and_the_disk() {
    let root = host("view", Some(DECLARATION));
    let view = view(&root).expect("reads").expect("declared");

    assert_eq!(view.package_prefix, "host");
    assert_eq!(view.host_faces, 3, "the records name the whole tree");
    assert_eq!(view.packages.len(), 2, "one ghost and one facade");
    let ghost = &view.packages[0];
    assert_eq!(ghost.package, "host-widgets");
    assert_eq!(ghost.crate_name.as_deref(), Some("widgets"));
    assert_eq!(ghost.subtrees, vec!["panel::frame".to_owned()]);
    assert_eq!(ghost.compiles, 2, "frame and widget, not panel");
    assert_eq!(
        ghost.mounts, 2,
        "the development shape mounts those two files"
    );
    assert!(
        ghost.copies > ghost.mounts,
        "the release shape carries the ancestor faces and the rule files too: {} copies",
        ghost.copies
    );
    assert_eq!(ghost.on_disk, OnDisk::Absent);
    assert_eq!(ghost.publish, vec!["not written yet".to_owned()]);
    let facade = &view.packages[1];
    assert_eq!(facade.package, "host-facade");
    assert!(
        facade.crate_name.is_none(),
        "the facade serves no single crate"
    );
    assert!(
        view.notes
            .iter()
            .all(|note| note.contains("workspace does not list")),
        "nothing is on disk, so the only note is the member list a write would materialize: {:?}",
        view.notes
    );
    let _ = std::fs::remove_dir_all(root);
}

/// What is on disk is read from the package's own manifest, and a foreign directory is named.
/// 磁盘上是什么从包自己的清单读出，而别人的目录会被点名。
#[test]
fn the_disk_state_names_the_shape_and_refuses_to_claim_a_foreign_package() {
    let root = host("disk", Some(DECLARATION));
    let ghost = &view(&root).expect("reads").expect("declared").packages[0];
    let directory = ghost.directory.clone();

    // Somebody else's package: the directory is there, and it is not this action's.
    fs::create_dir_all(directory.join("src")).expect("the directory");
    fs::write(
        directory.join("Cargo.toml"),
        "[package]\nname = \"host-widgets\"\n",
    )
    .expect("a hand-written manifest");
    fs::write(directory.join("src/lib.rs"), "// mine\n").expect("a source file");
    let ghost = &view(&root).expect("reads").expect("declared").packages[0];
    assert_eq!(ghost.on_disk, OnDisk::Foreign);
    assert_eq!(ghost.publish, vec!["not this action's package".to_owned()]);
    assert!(
        view(&root)
            .expect("reads")
            .expect("declared")
            .notes
            .iter()
            .any(|note| note.contains("is not a package this action created")),
        "a foreign directory is named, not silently overwritten"
    );

    // The development shape: generated, but not publishable.
    fs::write(
        directory.join("Cargo.toml"),
        "// Generated by XiRang\n[package]\nname = \"host-widgets\"\n",
    )
    .expect("a generated manifest");
    fs::write(directory.join("src/lib.rs"), "// Generated by XiRang\n").expect("a generated entry");
    let ghost = &view(&root).expect("reads").expect("declared").packages[0];
    assert_eq!(ghost.on_disk, OnDisk::Development);
    assert!(
        ghost.publish[0].contains("cannot be published")
            || ghost.publish[0].contains("crates.io would reject"),
        "{:?}",
        ghost.publish
    );
    assert!(
        view(&root)
            .expect("reads")
            .expect("declared")
            .notes
            .iter()
            .any(|note| note.contains("development shape is on disk")),
        "the screen says which shape is on disk"
    );

    // The release shape: generated, fingerprinted, and the notes are about publishing it.
    fs::write(
        directory.join("Cargo.toml"),
        "// Generated by XiRang\n[package]\nname = \"host-widgets\"\n\n\
         [package.metadata.xirang]\nshape = \"release\"\n\n\
         [dependencies]\nfast-widget = { path = \"../fast-widget\" }\n",
    )
    .expect("a release-shaped manifest");
    let ghost = &view(&root).expect("reads").expect("declared").packages[0];
    assert_eq!(ghost.on_disk, OnDisk::Release);
    assert!(
        ghost
            .publish
            .iter()
            .any(|note| note.contains("no `description`")),
        "a missing description is named: {:?}",
        ghost.publish
    );
    assert!(
        ghost
            .publish
            .iter()
            .any(|note| note.contains("path-only dependencies")),
        "a path-only dependency is named: {:?}",
        ghost.publish
    );
    assert!(ghost.files > 0 && ghost.bytes > 0, "the size is measured");
    let _ = std::fs::remove_dir_all(root);
}

/// The member list is part of the picture: a generated package the workspace does not list is named.
/// 成员清单也是这幅画的一部分：工作区没列出的生成包会被点名。
#[test]
fn an_unlisted_generated_package_is_named() {
    let parent = std::env::temp_dir()
        .join("xirang-scratch")
        .join(module_path!().replace("::", "-"))
        .join(format!(
            "xirang-partition-ws-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
    let _ = std::fs::remove_dir_all(&parent);
    let root = parent.join("host");
    fs::create_dir_all(&root).expect("the host");
    let fixture = host("ws-source", Some(DECLARATION));
    copy_tree(&fixture, &root);
    // The dependency has to resolve for cargo to name the package, and the fixture's own `../toolchain`
    // does not exist beside it: point it at this checkout's toolchain.
    // 依赖必须解析得到，cargo 才能说出包名；而夹具旁边的 `../toolchain` 并不存在：把它指向本检出的 toolchain。
    let manifest = fs::read_to_string(root.join("Cargo.toml")).expect("the host manifest");
    fs::write(
        root.join("Cargo.toml"),
        manifest.replace("../toolchain", env!("CARGO_MANIFEST_DIR")),
    )
    .expect("the repointed manifest");
    fs::write(
        parent.join("Cargo.toml"),
        "[workspace]\nresolver = \"3\"\nmembers = [\"host\"]\n",
    )
    .expect("the workspace manifest");

    let view = view(&root).expect("reads").expect("declared");
    assert_eq!(
        view.members_missing,
        vec![
            "crates/host-facade".to_owned(),
            "crates/host-widgets".to_owned()
        ],
        "both generated packages are missing from the list"
    );
    assert!(
        view.notes.iter().any(|note| note.contains("does not list")),
        "{:?}",
        view.notes
    );
    assert_eq!(
        view.workspace.as_deref(),
        Some(parent.as_path()),
        "the workspace root is the directory the member list belongs to"
    );
    let _ = std::fs::remove_dir_all(&parent);
    let _ = std::fs::remove_dir_all(&fixture);
}

/// A relative copy of one directory tree, for the workspace fixture.
/// 一棵目录树的相对复制，供工作区夹具使用。
fn copy_tree(from: &Path, to: &Path) {
    fs::create_dir_all(to).expect("the destination");
    for entry in fs::read_dir(from).expect("the source").flatten() {
        let path = entry.path();
        let target = to.join(entry.file_name());
        if path.is_dir() {
            copy_tree(&path, &target);
        } else {
            fs::copy(&path, &target).expect("a copied file");
        }
    }
}
