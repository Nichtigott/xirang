//! Pins for the shared source enumeration: what a read may enter, and what it must refuse.
//! 共同源码枚举的钉子：一次读取可以进入什么，以及它必须拒绝什么。

use std::path::{Path, PathBuf};

use super::{SourceScope, relative_spelling};
#[cfg(unix)]
use crate::source_map::{BINDINGS_FILE, BINDINGS_HEADER, SOURCES_FILE, SOURCES_HEADER};

fn fixture(label: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "xirang-source-scope-{label}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    root
}

#[cfg(unix)]
fn link(target: &Path, at: &Path) {
    std::os::unix::fs::symlink(target, at).expect("symlink");
}

fn wanted() -> impl Fn(&Path) -> bool {
    |path: &Path| path.extension().is_some_and(|extension| extension == "py")
}

/// Files under the package are found, with the package-relative spelling the ledger records.
/// 包内的文件被找到，并带上台账会记录的包内相对拼写。
#[test]
fn a_package_file_is_found_with_its_relative_spelling() {
    let root = fixture("plain");
    std::fs::create_dir_all(root.join("pkg")).unwrap();
    std::fs::write(root.join("pkg/face.py"), "class Face: pass\n").unwrap();
    std::fs::write(root.join("pkg/notes.txt"), "not source\n").unwrap();
    let scope = SourceScope::package(&root).unwrap();
    let found = scope.files(wanted(), &["target", ".git"]).unwrap();
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].source, "pkg/face.py");
    assert_eq!(
        relative_spelling(&root, &found[0].path).unwrap(),
        "pkg/face.py"
    );
    let _ = std::fs::remove_dir_all(root);
}

/// The reproduced defect: a link inside the package that points outside it is refused **by name**,
/// not followed and not silently skipped.
/// 被复现的缺陷：包内指向包外的链接按名拒绝，既不跟随也不静默跳过。
#[cfg(unix)]
#[test]
fn an_undeclared_link_out_of_the_package_is_refused() {
    let root = fixture("link");
    let outside = fixture("link-outside");
    std::fs::write(outside.join("outside.py"), "class Outside: pass\n").unwrap();
    std::fs::create_dir_all(root.join("pkg")).unwrap();
    link(&outside, &root.join("pkg/linked"));
    let scope = SourceScope::package(&root).unwrap();
    let error = scope
        .files(wanted(), &["target", ".git"])
        .expect_err("an undeclared link is not a way to widen the sources");
    assert!(error.contains("links to"), "{error}");
    assert!(error.contains("outside"), "{error}");
    assert!(error.contains("declare that root"), "{error}");
    let _ = std::fs::remove_dir_all(root);
    let _ = std::fs::remove_dir_all(outside);
}

/// A root the project declared explicitly is followed — a legitimate materialization may be a
/// link, so the rule is about declaration, not about links.
/// 项目显式声明的根会被跟随——合法物化可以是链接，因此规则在于声明，而不在于链接。
#[cfg(unix)]
#[test]
fn a_declared_root_is_followed() {
    let root = fixture("declared");
    let outside = fixture("declared-outside");
    std::fs::write(outside.join("outside.py"), "class Outside: pass\n").unwrap();
    std::fs::create_dir_all(root.join("pkg")).unwrap();
    link(&outside, &root.join("pkg/vendor"));
    let mut scope = SourceScope::package(&root).unwrap();
    scope.declare(&outside).unwrap();
    let found = scope.files(wanted(), &["target", ".git"]).unwrap();
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].source, "pkg/vendor/outside.py");
    let _ = std::fs::remove_dir_all(root);
    let _ = std::fs::remove_dir_all(outside);
}

/// A directory linked into its own ancestor terminates and reports each file once.
/// 链接到自身祖先的目录会终止，且每个文件只报一次。
#[cfg(unix)]
#[test]
fn a_cycle_terminates_and_a_file_reachable_twice_is_reported_once() {
    let root = fixture("cycle");
    std::fs::create_dir_all(root.join("pkg")).unwrap();
    std::fs::write(root.join("pkg/face.py"), "class Face: pass\n").unwrap();
    link(&root.join("pkg"), &root.join("pkg/again"));
    let scope = SourceScope::package(&root).unwrap();
    let found = scope.files(wanted(), &["target", ".git"]).unwrap();
    // `pkg/again/face.py` and `pkg/face.py` are the same file; the walk reports one of them.
    // `pkg/again/face.py` 与 `pkg/face.py` 是同一个文件；遍历只报其中一个。
    assert_eq!(found.len(), 1, "{found:?}");
    let _ = std::fs::remove_dir_all(root);
}

/// A source the project declares **and** this machine binds is entered through its materialization;
/// the file keeps the package-relative spelling, so the declaration widens what may be read without
/// widening what an identity is derived from.
/// 项目声明、本机绑定的来源经它的物化位置被进入；文件保留包内相对拼写，因此声明扩大的是"可以读什么"，
/// 而不是"身份由什么推导"。
#[cfg(unix)]
#[test]
fn a_declared_and_bound_source_is_entered_through_its_materialization() {
    let root = fixture("declared-source");
    let outside = fixture("declared-source-outside");
    std::fs::write(outside.join("outside.py"), "class Outside: pass\n").unwrap();
    std::fs::create_dir_all(root.join("vendor")).unwrap();
    link(&outside, &root.join("vendor/ui"));
    std::fs::write(
        root.join(SOURCES_FILE),
        format!("{SOURCES_HEADER}\nvendor-ui\tlocal\tvendor/ui\n"),
    )
    .unwrap();
    std::fs::create_dir_all(root.join(".xirang")).unwrap();
    std::fs::write(
        root.join(BINDINGS_FILE),
        format!("{BINDINGS_HEADER}\nvendor-ui\t{}\n", outside.display()),
    )
    .unwrap();
    let scope = SourceScope::with_sources(&root).unwrap();
    let found = scope.files(wanted(), &["target", ".git"]).unwrap();
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].source, "vendor/ui/outside.py");
    let _ = std::fs::remove_dir_all(root);
    let _ = std::fs::remove_dir_all(outside);
}

/// The same link with no declaration is still refused: the declaration is the permission, and the
/// two answers must not collapse into "links are fine now".
/// 同一条链接没有声明时仍被拒绝：声明才是许可，而两个答案不得塌成"现在链接都行了"。
#[cfg(unix)]
#[test]
fn a_link_without_a_declaration_stays_refused_even_though_one_with_it_is_followed() {
    let root = fixture("no-declaration");
    let outside = fixture("no-declaration-outside");
    std::fs::write(outside.join("outside.py"), "class Outside: pass\n").unwrap();
    std::fs::create_dir_all(root.join("vendor")).unwrap();
    link(&outside, &root.join("vendor/ui"));
    let scope = SourceScope::with_sources(&root).unwrap();
    let error = scope
        .files(wanted(), &["target", ".git"])
        .expect_err("no declaration, no permission");
    assert!(error.contains("not a way to widen the sources"), "{error}");
    let _ = std::fs::remove_dir_all(root);
    let _ = std::fs::remove_dir_all(outside);
}
