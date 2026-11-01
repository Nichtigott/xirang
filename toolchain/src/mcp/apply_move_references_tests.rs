//! What the move rewrites, and what it refuses to touch, across a real package layout.
//! 一次搬动会改写什么、拒绝碰什么，横跨一个真实的包布局。
//!
//! The walk's boundary is the package, and the spelling rules are per file: `crate::…`
//! names the moved tree only inside the crate that owns it, an integration test reaches it
//! through the crate's own name, and a nested package or a link out of the tree is
//! somebody else's bytes. Each of those is a case here because getting one wrong either
//! leaves a consumer stale or edits a tree the move never owned.
//! 遍历的边界是包，而拼法规则逐文件成立：只有属主 crate 之内 `crate::…` 才命名被搬的树，集成测试经
//! crate 自己的名字到达它，而嵌套的包或指向树外的链接都是别人的字节。每一条都在这里有用例，因为弄错
//! 任何一条，要么让消费者留在过期状态，要么去改搬动从未拥有的树。

use std::path::{Path, PathBuf};

use xirang_kernel::syntax::ModuleMove;

use super::{Scope, manifest_crate_name, plan};

/// A throwaway package root with the given files, each written verbatim.
/// 一个一次性包根，按给定内容逐字写下每个文件。
fn package(label: &str, manifest: &str, files: &[(&str, &str)]) -> PathBuf {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "xirang-references-{label}-{}-{sequence}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(root.join("Cargo.toml"), manifest).unwrap();
    for (relative, text) in files {
        let path = root.join(relative);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }
    root
}

fn moved(from: &str, to: &str) -> ModuleMove {
    ModuleMove {
        from: from.to_owned(),
        to: to.to_owned(),
    }
}

/// Run the walk with the historical layout: `src/` is both the scan root and the
/// identity base, which is the layout every one of these fixtures uses.
/// 用历史布局跑这趟遍历：`src/` 既是扫描根又是身份基准，这些夹具用的都是这套布局。
fn scan(root: &Path, crate_name: Option<&str>) -> super::Plan {
    let src = root.join("src");
    plan(
        &Scope {
            package_root: root,
            scan_root: &src,
            identity_base: &src,
            crate_name,
        },
        &[moved("control::button", "control::panel::button")],
        "control/button",
        "control/panel/button",
    )
    .expect("the package is scanned")
}

/// The bytes the plan would write for one relative file.
/// 计划会为一个相对文件写下的字节。
fn rewritten(root: &Path, plan: &super::Plan, relative: &str) -> Option<String> {
    let file = root.join(relative);
    let entry = plan.files.iter().find(|entry| entry.file == file)?;
    Some(super::bound_text(entry, &entry.before).expect("the plan fits its own bytes"))
}

/// A test file's `crate::…` path names the **test** crate, so the move must not rewrite it;
/// the same file reaches the package through its own name, and that spelling must follow.
/// 测试文件里的 `crate::…` 命名的是**测试** crate，因此搬动不得改写它；同一个文件经包自己的名字到达
/// 那个模块，而那种拼法必须跟随。
#[test]
fn an_integration_test_uses_the_crate_name_and_not_crate() {
    let root = package(
        "consumer",
        "[package]\nname = \"demo-host\"\nversion = \"0.0.0\"\nedition = \"2024\"\n",
        &[
            ("src/lib.rs", "pub mod control { pub mod button {} }\n"),
            (
                "tests/behavior.rs",
                "use demo_host::control::button::Knob;\n\
                 fn local() -> crate::control::button::Other { todo!() }\n\
                 fn local_bare() -> control::button::Other { todo!() }\n",
            ),
        ],
    );
    let plan = scan(&root, Some("demo_host"));
    let text = rewritten(&root, &plan, "tests/behavior.rs").expect("the test file is rewritten");
    assert!(
        text.contains("use demo_host::control::panel::button::Knob;"),
        "{text}"
    );
    assert!(
        text.contains("crate::control::button::Other"),
        "`crate::` in a test file is the test crate: {text}"
    );
    assert!(
        text.contains("fn local_bare() -> control::button::Other"),
        "a bare path in a test file is not the package's module either: {text}"
    );
    let _ = std::fs::remove_dir_all(root);
}

/// `build.rs` is a crate of its own, and the library target beside it is the one the move
/// owns — so the same spelling rule separates them.
/// `build.rs` 自成一个 crate，而它旁边的库目标才是搬动拥有的那个——于是同一条拼法规则把它们分开。
#[test]
fn build_rs_is_a_consumer_and_a_second_binary_is_not_the_library() {
    let root = package(
        "second-binary",
        "[package]\nname = \"demo-host\"\nversion = \"0.0.0\"\nedition = \"2024\"\n",
        &[
            ("src/lib.rs", "pub mod control { pub mod button {} }\n"),
            (
                "src/main.rs",
                "use crate::control::button::A;\nuse demo_host::control::button::B;\n",
            ),
            (
                "build.rs",
                "use crate::control::button::A;\nuse demo_host::control::button::B;\n",
            ),
        ],
    );
    let plan = scan(&root, Some("demo_host"));
    for file in ["src/main.rs", "build.rs"] {
        let text = rewritten(&root, &plan, file).expect("the consumer file is rewritten");
        assert!(
            text.contains("crate::control::button::A"),
            "{file}: `crate::` here is this file's own crate: {text}"
        );
        assert!(
            text.contains("demo_host::control::panel::button::B"),
            "{file}: the crate-name spelling follows the move: {text}"
        );
    }
    let _ = std::fs::remove_dir_all(root);
}

/// A module file inside the owning crate keeps `crate::…` but has no bare spelling, and the
/// crate root has both.
/// 属主 crate 里的模块文件保留 `crate::…` 而没有裸拼法，crate 根两者都有。
#[test]
fn the_crate_root_has_both_spellings_and_a_module_file_only_the_absolute_one() {
    let root = package(
        "inside",
        "[package]\nname = \"demo-host\"\nversion = \"0.0.0\"\nedition = \"2024\"\n",
        &[
            (
                "src/lib.rs",
                "pub mod control;\nuse crate::control::button::A;\nuse control::button::B;\n",
            ),
            (
                "src/control.rs",
                "use crate::control::button::A;\nuse control::button::B;\n",
            ),
        ],
    );
    let plan = scan(&root, Some("demo_host"));
    let root_text = rewritten(&root, &plan, "src/lib.rs").expect("the root is rewritten");
    assert!(
        root_text.contains("crate::control::panel::button::A")
            && root_text.contains("use control::panel::button::B"),
        "{root_text}"
    );
    let module_text = rewritten(&root, &plan, "src/control.rs").expect("the module is rewritten");
    assert!(
        module_text.contains("crate::control::panel::button::A"),
        "{module_text}"
    );
    assert!(
        module_text.contains("use control::button::B"),
        "a module file's bare path is an external crate: {module_text}"
    );
    let _ = std::fs::remove_dir_all(root);
}

/// A directory carrying its own manifest is a different package: its sources are not this
/// move's to rewrite, and the reply says it was not scanned rather than staying silent.
/// 自带清单的目录是另一个包：它的源码不归这次搬动改写，而回复说它没被扫描，而不是一声不吭。
#[test]
fn a_nested_package_is_reported_as_not_scanned() {
    let root = package(
        "nested",
        "[package]\nname = \"demo-host\"\nversion = \"0.0.0\"\nedition = \"2024\"\n",
        &[
            ("src/lib.rs", "pub mod control { pub mod button {} }\n"),
            (
                "graft/Cargo.toml",
                "[package]\nname = \"graft\"\nversion = \"0.0.0\"\nedition = \"2024\"\n",
            ),
            (
                "graft/src/lib.rs",
                "pub use crate::control::button::A;\nuse demo_host::control::button::B;\n",
            ),
        ],
    );
    let plan = scan(&root, Some("demo_host"));
    assert!(
        rewritten(&root, &plan, "graft/src/lib.rs").is_none(),
        "third-party sources are not rewritten"
    );
    assert!(
        plan.skipped.iter().any(|entry| entry.starts_with("graft")),
        "and the reply names what it did not read: {:?}",
        plan.skipped
    );
    let _ = std::fs::remove_dir_all(root);
}

/// A link that leaves the package is followed by neither the walk nor the writer.
/// 指向包外的链接，遍历与写入方都不跟随。
#[test]
fn a_link_out_of_the_package_is_refused_by_name() {
    let outside = package(
        "outside-target",
        "[package]\nname = \"elsewhere\"\nversion = \"0.0.0\"\nedition = \"2024\"\n",
        &[("src/lib.rs", "pub use crate::control::button::A;\n")],
    );
    let root = package(
        "linked",
        "[package]\nname = \"demo-host\"\nversion = \"0.0.0\"\nedition = \"2024\"\n",
        &[("src/lib.rs", "pub mod control { pub mod button {} }\n")],
    );
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(outside.join("src/lib.rs"), root.join("src/borrowed.rs"))
            .expect("a symlink fixture");
        let plan = scan(&root, Some("demo_host"));
        assert!(rewritten(&root, &plan, "src/borrowed.rs").is_none());
        assert!(
            plan.skipped
                .iter()
                .any(|entry| entry.contains("leaves the package")),
            "{:?}",
            plan.skipped
        );
    }
    let _ = std::fs::remove_dir_all(root);
    let _ = std::fs::remove_dir_all(outside);
}

/// A link that stays inside the package is read once, so a cycle cannot make the walk
/// unbounded.
/// 留在包内的链接只被读一次，因此环状链接不会让遍历无界。
#[test]
fn a_link_inside_the_package_is_read_once_and_cannot_loop() {
    let root = package(
        "cycle",
        "[package]\nname = \"demo-host\"\nversion = \"0.0.0\"\nedition = \"2024\"\n",
        &[
            ("src/lib.rs", "pub mod control { pub mod button {} }\n"),
            ("src/consumer.rs", "use demo_host::control::button::A;\n"),
        ],
    );
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(&root, root.join("src/loop")).expect("a symlink fixture");
        let plan = scan(&root, Some("demo_host"));
        assert!(
            rewritten(&root, &plan, "src/consumer.rs").is_some(),
            "the real file is still planned"
        );
    }
    let _ = std::fs::remove_dir_all(root);
}

/// The crate name comes from the manifest, hyphen folded, so a consumer path is spelled the
/// way Rust spells it. A manifest this reader cannot follow answers nothing rather than a
/// guess, which leaves the unknown-head obligation to report the gap.
/// crate 名来自清单并折叠连字符，因此消费者路径按 Rust 的拼法写出。读不出的清单回答"没有"而不是猜，
/// 于是那条"未知首段"义务负责报出这个缺口。
#[test]
fn the_crate_name_is_read_from_the_manifest() {
    let root = package(
        "name",
        "[package]\nname = \"demo-host\"\nversion = \"0.0.0\"\nedition = \"2024\"\n",
        &[("src/lib.rs", "pub fn f() {}\n")],
    );
    assert_eq!(manifest_crate_name(&root).as_deref(), Some("demo_host"));
    std::fs::write(root.join("Cargo.toml"), "[workspace]\nmembers = []\n").unwrap();
    assert_eq!(manifest_crate_name(&root), None);
    let _ = std::fs::remove_dir_all(root);
}

/// With no crate name available, a consumer path that clearly tails into the moved module is
/// an outstanding obligation rather than a silent miss.
/// 拿不到 crate 名时，明显以被搬模块为尾的消费者路径是一条未完成义务，而不是静默漏掉。
#[test]
fn an_unnameable_consumer_head_is_reported() {
    let root = package(
        "unknown-head",
        "[package]\nname = \"demo-host\"\nversion = \"0.0.0\"\nedition = \"2024\"\n",
        &[
            ("src/lib.rs", "pub mod control { pub mod button {} }\n"),
            (
                "tests/behavior.rs",
                "use somewhere_else::control::button::Knob;\n",
            ),
        ],
    );
    let plan = scan(&root, None);
    assert!(
        plan.unresolved
            .iter()
            .any(|reason| reason.contains("somewhere_else")),
        "{:?}",
        plan.unresolved
    );
    let _ = std::fs::remove_dir_all(root);
}
