//! covers `app/project_context.rs`, `app/write_guard.rs`.
//! 覆盖 `app/project_context.rs`、`app/write_guard.rs`。
//!
//! Project-root, manifest, namespace and write-scope tests.
//! 项目根、清单、命名空间与写作用域测试（covers `app/project_context.rs`）。

use super::*;

#[test]
fn standalone_studio_resolves_one_project_root_and_manifest() {
    let root = package_root();
    assert!(root.is_dir());
    assert_eq!(
        host_manifest().file_name().and_then(|name| name.to_str()),
        Some("Cargo.toml")
    );
}

/// A throwaway project directory holding one manifest and a library target, so
/// Cargo can be asked about it: `cargo metadata` refuses a manifest with no
/// target, and the name's authority is Cargo.
/// 一个只含一份清单与一个库目标的一次性项目目录，因此可以向 Cargo 询问它：`cargo metadata`
/// 拒绝没有目标的清单，而包名的权威是 Cargo。
fn temp_project(label: &str, manifest: &str) -> std::path::PathBuf {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let root = std::env::temp_dir().join(format!("xirang-toolchain-namespace-{label}-{suffix}"));
    std::fs::create_dir_all(root.join("src")).expect("project directory");
    std::fs::write(root.join("Cargo.toml"), manifest).expect("manifest");
    std::fs::write(root.join("src/lib.rs"), "// host entry\n").expect("library target");
    root
}

/// A launched session authors under the host crate's own package name, read from
/// the manifest it adopted — the value the host's build script stamps as
/// `CARGO_PKG_NAME`, and therefore the identity domain its recorded node ids live
/// in.
/// 已启动的会话在宿主 crate 自己的包名之下创作，读自它采纳的清单——也就是宿主构建脚本盖成
/// `CARGO_PKG_NAME` 的那个值，因此也是它记录的节点身份所在的域。
#[test]
fn a_launched_session_authors_under_the_host_crates_package_name() {
    let root = temp_project(
        "launched",
        "[package]\nname = \"demo-app\"\nversion = \"0.1.0\"\n",
    );
    let resolved = resolve_project(Some(&root)).expect("the fixture is a project");
    assert_eq!(resolved, root);
    assert_eq!(host_manifest(), root.join("Cargo.toml"));
    // The environment override still wins verbatim, exactly as the build side reads
    // it; without one the manifest names the namespace.
    let expected = std::env::var("XIRANG_NAMESPACE").unwrap_or_else(|_| "demo-app".to_owned());
    assert_eq!(package_namespace(), expected);
    // The adopted context is thread-local and the harness reuses threads, so the
    // fixture directory stays: a later test on this thread that falls back to
    // `package_root()` must still find a directory.
    // 采纳的上下文是线程局部的，而测试框架会复用线程，因此夹具目录留在原处：本线程上随后
    // 回落到 `package_root()` 的测试仍须找到一个存在的目录。
}

/// The namespace comes from Cargo's answer for the adopted manifest, not from a
/// literal line scan. The load-bearing case is TOML's dotted form: it is the same
/// table as `[package]`, a line scan sees no header and answers nothing, and
/// Studio then authored the project under `xirang.default` — an identity domain
/// none of that project's recorded ids live in, so a trace or a graft record on
/// disk could not resolve. The remaining cases are the ones the old scan already
/// had to get right, kept so the authority change cannot quietly lose them.
/// 命名空间来自 Cargo 对已采纳清单的回答，而不是字面逐行扫描。承重的情形是 TOML 的点式写法：
/// 它与 `[package]` 是同一张表，逐行扫描看不到任何表头、什么也答不出，于是 Studio 会在
/// `xirang.default` 之下创作该项目——而该项目记录的任何 id 都不住在那个身份域里，落盘的
/// trace 或 graft 记录因此解析不了。其余情形是旧扫描本来就必须答对的那些，保留它们是为了让这次
/// 权威更替不会悄悄丢掉它们。
#[test]
fn the_namespace_comes_from_cargo_not_from_a_literal_manifest_scan() {
    let default = crate::run_method::lexicon::DEFAULT_NAMESPACE;
    let cases: &[(&str, &str)] = &[
        (
            "[package]\nname = \"demo-app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
            "demo-app",
        ),
        (
            "[package]\nname = 'single-quoted'\nversion = \"0.1.0\"\nedition = \"2021\"\n",
            "single-quoted",
        ),
        (
            "# name = \"commented\"\n[package]\nname = \"real\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
            "real",
        ),
        (
            "[package]\nname = \"inline\" # trailing comment\nversion = \"0.1.0\"\nedition = \"2021\"\n",
            "inline",
        ),
        (
            "package.name = \"dotted-name\"\npackage.version = \"0.1.0\"\npackage.edition = \"2021\"\n",
            "dotted-name",
        ),
        ("[dependencies]\nname = \"another-package\"\n", default),
        ("[package]\nname.workspace = true\n", default),
        ("[workspace]\nmembers = [\"app\"]\n", default),
    ];
    for (index, (manifest, expected)) in cases.iter().enumerate() {
        let root = temp_project(&format!("cargo-{index}"), manifest);
        assert_eq!(
            namespace_for(&root.join("Cargo.toml"), None),
            *expected,
            "{manifest:?}"
        );
        let _ = std::fs::remove_dir_all(&root);
    }
}

/// Every write goes through the selected project. With none selected the wrapper
/// refuses; it never resolves a root from the environment or the working
/// directory, which is how a delete once moved a module out of the wrong project.
/// 每次写入都经过已选中的项目。没有选中时包装函数拒绝执行，绝不从环境或工作目录解析根——正是
/// 那样一次解析让删除把模块从错误的项目里搬走了。
#[test]
fn a_write_without_a_selected_project_is_refused() {
    clear_project_context();
    assert_eq!(
        with_selected_project(|| Ok::<_, String>("wrote")),
        Err("no project is selected; open a project before writing to it".to_owned()),
        "a write must not fall back to a guessed root"
    );
    assert!(selected_package_root().is_err());

    let root = temp_project(
        "writers",
        "[package]\nname = \"writers-app\"\nversion = \"0.1.0\"\n",
    );
    select_project(root.clone(), root.join("Cargo.toml"), "writers-app");
    assert_eq!(
        with_selected_project(|| Ok::<_, String>("wrote")),
        Ok("wrote")
    );
    assert_eq!(selected_package_root(), Ok(root.clone()));
    // The context a write runs in carries the selected project's namespace, so
    // `delete_module` and friends resolve ids in the domain the reader opened.
    // 写入所运行的上下文带着已选中项目的命名空间，因此 `delete_module` 之类的调用在读者打开的
    // 那个域里解析 id。
    assert_eq!(
        with_selected_project(|| Ok::<_, String>(package_namespace())),
        Ok("writers-app".to_owned())
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// A delete with no project selected is refused before it can act on a guessed
/// root. This is the operation the overlay's own comment remembers: it used to
/// resolve its root through the environment or the working directory and move a
/// module out of whichever project that named.
/// 没有选中项目时的删除会在作用于猜来的根之前被拒绝。这正是删除浮层自己的注释记得的那个操作：
/// 它过去会经环境变量或工作目录解析根，并把模块从那个恰好命名的项目里搬走。
#[test]
fn a_delete_without_a_selected_project_is_refused() {
    clear_project_context();
    let mut app = App::load_app();
    let target = app.selected;
    app.overlay = Some(Overlay::Delete(target));
    app.handle_key(KeyEvent::from(KeyCode::Char('y')));
    assert!(
        app.event.contains("no project is selected"),
        "a delete must refuse a guessed root: {}",
        app.event
    );
}

/// The namespace follows the manifest unless the environment names one, and a
/// manifest with no `[package]` falls back to the documented default.
/// 命名空间跟随清单，除非环境给出一个；没有 `[package]` 的清单回落到文档化的默认值。
#[test]
fn the_namespace_follows_the_manifest_unless_the_environment_names_one() {
    let host = temp_project(
        "ns-host",
        "[package]\nname = \"demo-app\"\nversion = \"0.1.0\"\n",
    );
    let manifest = host.join("Cargo.toml");
    assert_eq!(namespace_for(&manifest, None), "demo-app");
    assert_eq!(namespace_for(&manifest, Some("pinned")), "pinned");

    let workspace = temp_project("ns-workspace", "[workspace]\nmembers = [\"app\"]\n");
    assert_eq!(
        namespace_for(&workspace.join("Cargo.toml"), None),
        crate::run_method::lexicon::DEFAULT_NAMESPACE
    );
    let _ = std::fs::remove_dir_all(&host);
    let _ = std::fs::remove_dir_all(&workspace);
}

#[test]
fn installed_studio_never_exports_cargo_git_cache_paths() {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let checkout = std::env::temp_dir().join(format!("cargo-git-checkout-{suffix}"));
    // The two siblings are the ones a checkout has today; the fixture used to lay out
    // `core/` and `build_method/`, so it described a checkout that no longer exists.
    // 这两个同级目录是今天的检出才有的；这个夹具过去摆的是 `core/` 与 `build_method/`，
    // 描述的是一份已经不存在的检出。
    std::fs::create_dir_all(checkout.join("kernel")).expect("kernel sibling");
    std::fs::create_dir_all(checkout.join("toolchain")).expect("toolchain sibling");

    let source = crate::build_method::scaffold::detected_source(
        &checkout.join("toolchain"),
        &checkout.join("outside-bin/xirang-toolchain"),
    );
    let (core, build) = crate::build_method::scaffold::dependency_specs(&source);

    // Audit `F8` changed what an installed copy writes: the fallback used to be a git dependency,
    // which needs a fetch before the project resolves at all. It is now the published release — so
    // the property this pin is about (no path from *this machine* leaks into a generated manifest)
    // is checked the same way, and the stronger half is checked too: nothing to fetch either.
    // 审计 `F8` 改了装出来的副本会写下的东西：过去回落到 git 依赖，而那个项目要能解析就得先拉一次仓库。
    // 现在回落到已发布的那一版——因此这条钉子守的性质（**本机**的路径不会漏进生成的清单）照样检查，而且
    // 更强的另一半也一起检查：也没有要拉的东西。
    assert!(
        core.contains(&format!("version = \"{}\"", env!("CARGO_PKG_VERSION"))),
        "the released requirement is what an installed copy writes: {core}"
    );
    assert!(
        !core.contains("git =") && !build.contains("git ="),
        "nothing in a generated manifest asks for a fetch: {core} / {build}"
    );
    assert!(!core.contains(checkout.to_string_lossy().as_ref()));
    assert!(!build.contains(checkout.to_string_lossy().as_ref()));
    let _ = std::fs::remove_dir_all(checkout);
}
