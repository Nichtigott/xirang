//! The parent fact table: both spellings, the island stem, and the refusal for missing facts.
//! 父级事实表：两种拼写、孤岛 stem，以及缺事实时的拒绝。

use super::*;

/// A layout with the historical `src/` scan root.
/// 历史 `src/` 扫描根的布局。
fn layout() -> crate::source_layout::SourceLayout {
    crate::source_layout::SourceLayout {
        package_root: PathBuf::from("/pkg"),
        scan_root: PathBuf::from("/pkg/src"),
        identity_base: PathBuf::from("/pkg/src"),
        target: None,
    }
}

/// A walk-root face is spelled below `src/`, an island keeps its package-relative spelling and
/// mounts at its stem, and an inherited child grows the island's chain.
/// 遍历根下的面拼在 `src/` 之下，孤岛保留包内相对拼写并挂在 stem 上，而继承的子面在孤岛的链上长一段。
#[test]
fn module_chains_come_from_the_layout_and_the_mappings() {
    let prefix = scan_prefix(&layout());
    let mappings = vec![("components/reading".to_owned(), "reading".to_owned())];
    assert_eq!(
        module_chain("src/panel/panel.rs", &prefix, &mappings).as_deref(),
        Some("panel")
    );
    assert_eq!(
        module_chain("components/reading/reading.rs", &prefix, &mappings).as_deref(),
        Some("reading")
    );
    assert_eq!(
        module_chain(
            "components/reading/object/gauge/gauge.rs",
            &prefix,
            &mappings
        )
        .as_deref(),
        Some("reading::object::gauge")
    );
    // A mapping inside the walk root does not change the walk's own chain.
    // 遍历根之内的映射不改变主遍历自己的链。
    let inside = vec![("src/components/x".to_owned(), "x".to_owned())];
    assert_eq!(
        module_chain("src/components/x/x.rs", &prefix, &inside).as_deref(),
        Some("components::x")
    );
}

/// A custom library target separates the scan root from the identity base; the chain follows the
/// scan root, not the package root.
/// 自定义库目标把扫描根与身份基准分开；链跟随扫描根，而不是包根。
#[test]
fn a_custom_library_target_keeps_the_chain_below_the_scan_root() {
    let layout = crate::source_layout::SourceLayout {
        package_root: PathBuf::from("/pkg"),
        scan_root: PathBuf::from("/pkg/host"),
        identity_base: PathBuf::from("/pkg"),
        target: Some(PathBuf::from("/pkg/host/lib.rs")),
    };
    let prefix = scan_prefix(&layout);
    assert_eq!(prefix, "host");
    assert_eq!(
        module_chain("host/panel/panel.rs", &prefix, &[]).as_deref(),
        Some("panel")
    );
}

/// Two files claiming one module is not "last one wins": it is a refusal naming both.
/// 两个文件声称同一个模块不是"后者胜"：是一条点名两者的拒绝。
#[test]
fn two_files_claiming_one_module_are_refused() {
    let root = std::env::temp_dir().join(format!(
        "xirang-parents-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(root.join("src/a")).expect("tree");
    std::fs::write(
        root.join("src/a/a.rs"),
        "// generated-by=XiRang\ncrate::root_object! { kind: Alpha, parent: crate::ROOT_NODE_ID }\n",
    )
    .expect("a");
    // Two directories that both resolve to the chain `a` under a scan root whose prefix is empty is
    // impossible; the ambiguity this table can actually see is one module claimed by two sources in
    // different discovered roots, which needs a package-relative source outside `src/`.
    // 两个目录在空前缀下同时解析成 `a` 是不可能的；这张表真能看见的歧义是"一个模块被不同发现根下的两份
    // 源码声称"，那需要一个 `src/` 之外的包内相对源码。
    std::fs::create_dir_all(root.join("b")).expect("tree");
    std::fs::write(
        root.join("b/a.rs"),
        "// generated-by=XiRang\ncrate::root_object! { kind: Beta, parent: crate::ROOT_NODE_ID }\n",
    )
    .expect("b");
    std::fs::write(
        root.join("xirang.objects.tsv"),
        "# xirang objects v1\n0123456789abcdef0123456789abcdef\troot/a\tb/a.rs\tb\t-\n",
    )
    .expect("ledger");
    let mut layout = layout();
    layout.package_root = root.clone();
    layout.scan_root = root.join("src");
    layout.identity_base = root.join("src");
    let facts = facts_in(&layout).expect_err("two sources claim the module `a`");
    assert!(facts.contains("one module path is one face"), "{facts}");
    let _ = std::fs::remove_dir_all(&root);
}
