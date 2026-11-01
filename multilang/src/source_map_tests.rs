//! Pins for the project's declared sources and this machine's bindings.
//! 项目声明来源与本机绑定的钉子。
//!
//! Every refusal here is a case the review named: an absolute path in the shared file, a source
//! with no binding, a binding that disagrees with the materialization, a binding nobody declared,
//! and a duplicate declaration. Each one names both sides, because a reader that guesses reads a
//! tree nobody described.
//! 这里每条拒绝都是复核点名的情形：共享文件里的绝对路径、没有绑定的来源、与物化不一致的绑定、无人声明的
//! 绑定、以及重复声明。每条都点名两侧，因为猜的读取方读到的是没人描述过的树。

use std::path::{Path, PathBuf};

use super::{
    BINDINGS_FILE, BINDINGS_HEADER, SOURCES_FILE, SOURCES_HEADER, SourceKind, SourceMap,
    WORKSPACE_SOURCE,
};

fn fixture(label: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "xirang-source-map-{label}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("fixture root");
    root
}

fn write_file(root: &Path, relative: &str, text: &str) {
    let path = root.join(relative);
    std::fs::create_dir_all(path.parent().expect("parent")).expect("dir");
    std::fs::write(path, text).expect("file");
}

fn sources(root: &Path, rows: &[(&str, &str, &str)]) {
    let mut text = format!("{SOURCES_HEADER}\n");
    for (id, kind, location) in rows {
        text.push_str(&format!("{id}\t{kind}\t{location}\n"));
    }
    write_file(root, SOURCES_FILE, &text);
}

fn binding(root: &Path, id: &str, path: &Path) {
    let mut text = format!("{BINDINGS_HEADER}\n");
    text.push_str(&format!("{id}\t{}\n", path.display()));
    write_file(root, BINDINGS_FILE, &text);
}

/// A self-contained declared source — a real directory inside the package — needs no binding.
/// 自包含的已声明来源——包内的一个真实目录——不需要绑定。
#[test]
fn a_self_contained_source_needs_no_binding() {
    let root = fixture("self");
    std::fs::create_dir_all(root.join("vendor/ui")).expect("dir");
    sources(&root, &[("vendor-ui", "local", "vendor/ui")]);
    let roots = SourceMap::read_from(&root)
        .expect("the declarations read")
        .roots(&root)
        .expect("a real directory needs no binding");
    assert_eq!(roots.len(), 1);
    assert_eq!(roots[0].id, "vendor-ui");
    assert_eq!(roots[0].location, root.join("vendor/ui"));
    assert_eq!(roots[0].bound, None);
    let _ = std::fs::remove_dir_all(root);
}

/// A source materialized as a link out of the package has no location on this machine until it is
/// bound, and the refusal says which file to write.
/// 以包外链接物化的来源在本机没有位置，直到它被绑定；拒绝会说要写哪个文件。
#[cfg(unix)]
#[test]
fn a_link_without_a_binding_refuses_to_guess() {
    let root = fixture("unbound");
    let outside = fixture("unbound-outside");
    std::fs::create_dir_all(root.join("vendor")).expect("dir");
    #[cfg(unix)]
    std::os::unix::fs::symlink(&outside, root.join("vendor/ui")).expect("symlink");
    sources(&root, &[("vendor-ui", "local", "vendor/ui")]);
    let error = SourceMap::read_from(&root)
        .expect("the declarations read")
        .roots(&root)
        .expect_err("an unbound external source is not guessed");
    assert!(error.contains("no binding for it"), "{error}");
    assert!(error.contains(BINDINGS_FILE), "{error}");
    assert!(error.contains("machine-local"), "{error}");
    let _ = std::fs::remove_dir_all(root);
    let _ = std::fs::remove_dir_all(outside);
}

/// A binding resolves the source, and the root the machine reads is the one the binding names.
/// 绑定解析出来源，而本机读取的根就是绑定点名的那个。
#[cfg(unix)]
#[test]
fn a_binding_resolves_the_source_it_names() {
    let root = fixture("bound");
    let outside = fixture("bound-outside");
    std::fs::create_dir_all(&outside).expect("outside");
    std::fs::create_dir_all(root.join("vendor")).expect("dir");
    #[cfg(unix)]
    std::os::unix::fs::symlink(&outside, root.join("vendor/ui")).expect("symlink");
    sources(&root, &[("vendor-ui", "local", "vendor/ui")]);
    binding(&root, "vendor-ui", &outside);
    let roots = SourceMap::read_from(&root)
        .expect("the declarations read")
        .roots(&root)
        .expect("the binding resolves it");
    assert_eq!(roots[0].bound.as_deref(), Some(outside.as_path()));
    let _ = std::fs::remove_dir_all(root);
    let _ = std::fs::remove_dir_all(outside);
}

/// The shared file never carries an absolute path: where a checkout sits is not a project fact.
/// 共享文件绝不携带绝对路径：检出放在哪儿不是项目事实。
#[test]
fn a_shared_location_cannot_be_absolute() {
    let root = fixture("absolute");
    let error = SourceMap::parse_text(
        &format!("{SOURCES_HEADER}\nvendor-ui\tlocal\t/opt/vendor/ui\n"),
        SOURCES_FILE,
    )
    .expect_err("an absolute location belongs to a machine, not to the project");
    assert!(error.contains("not a package-relative path"), "{error}");
    assert!(error.contains("this machine"), "{error}");
    let _ = std::fs::remove_dir_all(root);
}

/// One id is one source.
/// 一个 id 就是一个来源。
#[test]
fn a_source_id_cannot_be_declared_twice() {
    let error = SourceMap::parse_text(
        &format!("{SOURCES_HEADER}\nvendor-ui\tlocal\tvendor/a\nvendor-ui\tlocal\tvendor/b\n"),
        SOURCES_FILE,
    )
    .expect_err("one id is one source");
    assert!(error.contains("declared twice"), "{error}");
    assert!(error.contains("vendor/a"), "{error}");
}

/// An unknown kind is refused by name instead of defaulting to `local`.
/// 未知类型按名拒绝，而不是默认成 `local`。
#[test]
fn an_unknown_kind_is_refused_by_name() {
    let error = SourceMap::parse_text(
        &format!("{SOURCES_HEADER}\nvendor-ui\tsvn\tvendor/ui\n"),
        SOURCES_FILE,
    )
    .expect_err("an unimplemented kind is not a kind");
    assert!(error.contains("not a source kind"), "{error}");
    assert!(error.contains("`local`"), "{error}");
}

/// The package itself is implicit and cannot be re-declared.
/// 包自身是隐式的，不能被重复声明。
#[test]
fn the_workspace_source_is_implicit() {
    let error = SourceMap::parse_text(
        &format!("{SOURCES_HEADER}\n{WORKSPACE_SOURCE}\tlocal\tvendor/ui\n"),
        SOURCES_FILE,
    )
    .expect_err("the package source cannot be declared");
    assert!(error.contains("implicit"), "{error}");
}

/// A binding that names somewhere other than the materialization is two answers to one question.
/// 与物化位置不同的绑定是同一个问题的两个答案。
#[cfg(unix)]
#[test]
fn a_binding_that_disagrees_with_the_materialization_is_refused() {
    let root = fixture("disagree");
    let first = fixture("disagree-first");
    let second = fixture("disagree-second");
    std::fs::create_dir_all(&first).expect("first");
    std::fs::create_dir_all(&second).expect("second");
    std::fs::create_dir_all(root.join("vendor")).expect("dir");
    #[cfg(unix)]
    std::os::unix::fs::symlink(&first, root.join("vendor/ui")).expect("symlink");
    sources(&root, &[("vendor-ui", "local", "vendor/ui")]);
    binding(&root, "vendor-ui", &second);
    let error = SourceMap::read_from(&root)
        .expect("the declarations read")
        .roots(&root)
        .expect_err("one source has one location");
    assert!(error.contains("one source has one location"), "{error}");
    assert!(error.contains("vendor/ui"), "{error}");
    let _ = std::fs::remove_dir_all(root);
    let _ = std::fs::remove_dir_all(first);
    let _ = std::fs::remove_dir_all(second);
}

/// A binding nobody declared permits nothing, so it is reported instead of ignored.
/// 无人声明的绑定什么也不许可，因此报告出来而不是忽略。
#[test]
fn a_binding_for_an_undeclared_source_is_refused() {
    let root = fixture("undeclared");
    let outside = fixture("undeclared-outside");
    std::fs::create_dir_all(&outside).expect("outside");
    binding(&root, "vendor-ui", &outside);
    let error = SourceMap::read_from(&root)
        .expect("an absent declaration file is an empty map")
        .roots(&root)
        .expect_err("a binding without a declaration permits nothing");
    assert!(error.contains("no source"), "{error}");
    assert!(error.contains("vendor-ui"), "{error}");
    let _ = std::fs::remove_dir_all(root);
    let _ = std::fs::remove_dir_all(outside);
}

/// A binding is machine-local, so it must not be resolved against the process's directory.
/// 绑定是本机的，因此不得按进程当前目录解析。
#[test]
fn a_relative_binding_is_refused() {
    let root = fixture("relative-binding");
    write_file(
        &root,
        BINDINGS_FILE,
        &format!("{BINDINGS_HEADER}\nvendor-ui\tvendor/ui\n"),
    );
    sources(&root, &[("vendor-ui", "local", "vendor/ui")]);
    let error = SourceMap::read_from(&root)
        .expect("the declarations read")
        .roots(&root)
        .expect_err("a relative binding would depend on the working directory");
    assert!(error.contains("not an absolute path"), "{error}");
    assert!(error.contains("working directory"), "{error}");
    let _ = std::fs::remove_dir_all(root);
}

/// A declared source whose materialization is missing names the declaration, not just "missing".
/// 已声明来源的物化位置缺失时，拒绝点名那条声明，而不只是"缺失"。
#[test]
fn a_missing_materialization_names_the_declaration() {
    let root = fixture("missing");
    sources(&root, &[("vendor-ui", "local", "vendor/ui")]);
    let error = SourceMap::read_from(&root)
        .expect("the declarations read")
        .roots(&root)
        .expect_err("the declared place does not exist");
    assert!(error.contains("vendor-ui"), "{error}");
    assert!(error.contains("vendor/ui"), "{error}");
    assert!(error.contains(BINDINGS_FILE), "{error}");
    let _ = std::fs::remove_dir_all(root);
}

/// The kind this build implements is spelled in the file, and parsing it back is the same value.
/// 本构建实现的类型就是文件里的拼写，解析回来是同一个值。
#[test]
fn the_declared_kind_round_trips() {
    let map = SourceMap::parse_text(
        &format!("{SOURCES_HEADER}\nvendor-ui\tlocal\tvendor/ui\n"),
        SOURCES_FILE,
    )
    .expect("a local source parses");
    assert_eq!(map.declarations()[0].kind, SourceKind::Local);
    assert_eq!(map.declarations()[0].kind.as_str(), "local");
}
