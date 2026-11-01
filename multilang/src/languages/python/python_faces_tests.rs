//! Pins for the build-time Python extraction: facts, parent spelling and refusals.
//! 构建期 Python 提取的钉子：事实、父拼写与拒绝。

use std::path::PathBuf;

use super::*;

fn root(label: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "xirang-python-faces-{label}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("fixture root");
    root
}

fn write_file(root: &std::path::Path, relative: &str, text: &str) {
    let path = root.join(relative);
    std::fs::create_dir_all(path.parent().expect("parent")).expect("fixture dir");
    std::fs::write(path, text).expect("fixture file");
}

/// Both declarations are read with the fields a tree needs, and a plain module is ignored.
/// 两条声明都被读出树需要的字段，普通模块被忽略。
#[test]
fn every_declaration_is_read_with_its_fields() {
    let root = root("facts");
    write_file(
        &root,
        "widgets/gauge.py",
        "from xirang import soil, faces\n\n@soil.face(under=faces.shop.panel.NODE_ID, \
         named=\"Gauge\", exports=(\"gauge\", \"reset\"), needs_registry=False)\nclass Gauge:\n    pass\n",
    );
    write_file(
        &root,
        "widgets/panel/needle.py",
        "from xirang import soil, faces\n\n@soil.face(under=faces.shop.panel.area.NODE_ID, \
         named=\"Needle\", needs_registry=True)\nclass Needle:\n    pass\n",
    );
    write_file(&root, "widgets/plain.py", "VALUE = 1\n");

    let faces = read_declarations(&root).expect("the package extracts");
    assert_eq!(faces.len(), 2, "{faces:?}");
    let gauge = faces
        .iter()
        .find(|face| face.name == "Gauge")
        .expect("gauge");
    assert_eq!(gauge.source, "widgets/gauge.py");
    assert_eq!(gauge.class, "Gauge");
    assert_eq!(gauge.parent, "shop::panel");
    assert!(!gauge.owns_registry);
    assert_eq!(gauge.exports, ["gauge", "reset"]);
    let needle = faces
        .iter()
        .find(|face| face.name == "Needle")
        .expect("needle");
    assert_eq!(needle.parent, "shop::panel::area");
    assert!(needle.owns_registry);
    assert!(needle.exports.is_empty());
}

/// A parent must be one of the generated `faces.…` references, not an arbitrary attribute chain.
/// 父必须是生成的 `faces.…` 引用之一，而不是任意属性链。
#[test]
fn a_parent_outside_the_generated_object_is_refused() {
    let root = root("parent");
    write_file(
        &root,
        "gauge.py",
        "@soil.face(under=somewhere.else.NODE_ID, named=\"Gauge\")\nclass Gauge:\n    pass\n",
    );
    let error = read_declarations(&root).expect_err("an arbitrary parent is refused");
    assert!(error.contains("generated `faces.…`"), "{error}");
    assert!(error.contains("gauge.py"), "{error}");
}

/// A declaration without a display name cannot describe a tree row.
/// 没有显示名的声明无法描述一行树。
#[test]
fn a_declaration_without_named_is_refused() {
    let root = root("named");
    write_file(
        &root,
        "gauge.py",
        "@soil.face(under=faces.shop.panel.NODE_ID)\nclass Gauge:\n    pass\n",
    );
    let error = read_declarations(&root).expect_err("no display name");
    assert!(error.contains("`named=`"), "{error}");
}

/// `needs_registry` is a Python boolean, not any text.
/// `needs_registry` 是 Python 布尔值，不是任意文本。
#[test]
fn a_non_boolean_needs_registry_is_refused() {
    let root = root("boolean");
    write_file(
        &root,
        "gauge.py",
        "@soil.face(under=faces.shop.panel.NODE_ID, named=\"Gauge\", needs_registry=\"yes\")\n\
         class Gauge:\n    pass\n",
    );
    let error = read_declarations(&root).expect_err("a text boolean is refused");
    assert!(error.contains("not a boolean"), "{error}");
}

/// The walk never leaves the package root and never enters caches or other packages.
/// 遍历不离开包根，也不进入缓存或其它包。
#[test]
fn the_walk_stays_inside_the_package() {
    let root = root("walk");
    write_file(
        &root,
        "inner/gauge.py",
        "@soil.face(under=faces.shop.NODE_ID, named=\"Gauge\")\nclass Gauge:\n    pass\n",
    );
    for skipped in ["target", "__pycache__", "crates", ".xirang"] {
        write_file(
            &root,
            &format!("{skipped}/ghost.py"),
            "@soil.face(under=faces.shop.NODE_ID, named=\"Ghost\")\nclass Ghost:\n    pass\n",
        );
    }
    let faces = read_declarations(&root).expect("the package extracts");
    assert_eq!(faces.len(), 1, "{faces:?}");
    assert_eq!(faces[0].name, "Gauge");
}

/// The reproduced defect, as a pin: a link inside the package that points outside it is refused by
/// the extractor, not followed. The reviewer's probe returned `Ok([PythonFace { source:
/// "linked/outside.py", … }])` before this rule moved into the shared source scope.
/// 被复现的缺陷，作为钉子：包内指向包外的链接会被提取器拒绝，而不是被跟随。在这条规则移进共同来源范围
/// 之前，复核探针返回的是 `Ok([PythonFace { source: "linked/outside.py", … }])`。
#[test]
fn a_link_that_leaves_the_package_is_refused_by_the_extractor() {
    let package = root("link");
    let outside = root("link-outside");
    write_file(
        &outside,
        "outside.py",
        "@soil.face(under=faces.shop.NODE_ID, named=\"Outside\")\nclass Outside:\n    pass\n",
    );
    std::fs::create_dir_all(package.join("pkg")).expect("package dir");
    #[cfg(unix)]
    std::os::unix::fs::symlink(&outside, package.join("pkg/linked")).expect("symlink");
    #[cfg(not(unix))]
    let _ = &outside;
    let error = read_declarations(&package).expect_err("an undeclared link is not a source");
    assert!(error.contains("links to"), "{error}");
    assert!(error.contains("declare that root"), "{error}");
    let _ = std::fs::remove_dir_all(&package);
    let _ = std::fs::remove_dir_all(&outside);
}

/// A declaration inside a **declared** external source is read, with the package-relative spelling
/// the tree records. This is the second-language proof that the shared source enumeration — not the
/// adapter — decides what a read may enter.
/// **已声明**外部来源里的声明会被读出，并带上树记录的包内相对拼写。这是第二种语言对"由共同来源枚举、
/// 而不是适配器决定一次读取能进入什么"的证明。
#[test]
fn a_declaration_in_a_declared_source_is_read() {
    use crate::source_map::{BINDINGS_FILE, BINDINGS_HEADER, SOURCES_FILE, SOURCES_HEADER};
    let package = root("declared-source");
    let outside = root("declared-source-outside");
    write_file(
        &outside,
        "outside.py",
        "@soil.face(under=faces.shop.NODE_ID, named=\"Outside\")\nclass Outside:\n    pass\n",
    );
    std::fs::create_dir_all(package.join("vendor")).expect("package dir");
    #[cfg(unix)]
    std::os::unix::fs::symlink(&outside, package.join("vendor/ui")).expect("symlink");
    std::fs::write(
        package.join(SOURCES_FILE),
        format!("{SOURCES_HEADER}\nvendor-ui\tlocal\tvendor/ui\n"),
    )
    .expect("declaration");
    std::fs::create_dir_all(package.join(".xirang")).expect("machine dir");
    std::fs::write(
        package.join(BINDINGS_FILE),
        format!("{BINDINGS_HEADER}\nvendor-ui\t{}\n", outside.display()),
    )
    .expect("binding");
    let faces = read_declarations(&package).expect("the declared source is read");
    assert_eq!(faces.len(), 1, "{faces:?}");
    assert_eq!(faces[0].source, "vendor/ui/outside.py");
    assert_eq!(faces[0].name, "Outside");
    let _ = std::fs::remove_dir_all(&package);
    let _ = std::fs::remove_dir_all(&outside);
}
