//! Pins for the language scaffold: the id rules, the template's anchors, and the write-refusal.
//! 语言脚手架的钉子：id 规则、模板锚点与写入拒绝。

use std::fs;

use super::{MARKER, plan, register, registration, write_files};

fn checkout(label: &str) -> std::path::PathBuf {
    let root = std::env::temp_dir().join(format!("xirang-lang-new-{label}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("multilang/src/languages")).expect("fixture languages");
    fs::create_dir_all(root.join("multilang/sdk")).expect("fixture sdk");
    root
}

/// A scaffolded module carries the marker, the kit delegation, and the language's own name.
/// 脚手架模块带上标记、kit 委托与语言自己的名字。
#[test]
fn the_template_anchors_hold() {
    let planned = plan("toy", "toy").expect("a valid id");
    assert_eq!(planned.len(), 2, "one module and one SDK file");
    let module = &planned[0].text;
    assert!(module.starts_with(&format!("//! {MARKER}")), "{module}");
    assert!(module.contains("pub struct Toy;"), "{module}");
    assert!(module.contains("impl LanguageAdapter for Toy"), "{module}");
    assert!(
        module.contains("crate::languages::kit::adapter::plan(&SPEC, request, create, modify)"),
        "the plan is the kit's: {module}"
    );
    assert!(module.contains("language: \"toy\""), "{module}");
    let lines = registration("toy");
    assert!(
        lines
            .iter()
            .any(|line| line.contains("lang-toy = [\"semantics\"]"))
    );
    assert!(
        lines
            .iter()
            .any(|line| line.contains("adapters.push(&crate::languages::toy::Toy)"))
    );
}

/// A bad id or a wrong extension is refused by name.
/// 坏 id 或错误扩展名按名拒绝。
#[test]
fn a_bad_id_is_refused() {
    for id in ["Toy", "toy-box", "1toy", ""] {
        assert!(plan(id, "toy").is_err(), "`{id}` must be refused");
    }
    assert!(plan("kit", "kit").is_err(), "the kit is not a language");
    assert!(plan("toy", "").is_err(), "an empty extension is refused");
    assert!(
        plan("toy", "t/oy").is_err(),
        "a path separator is not an extension"
    );
}

/// Writing twice refuses the second time and leaves the first files untouched.
/// 第二次写入被拒绝，第一次的文件保持原样。
#[test]
fn writing_twice_refuses_and_keeps_the_first_write() {
    let root = checkout("write");
    let written = write_files(&root, "toy", "toy").expect("the first write");
    assert_eq!(written.len(), 2);
    let module = root.join("multilang/src/languages/toy/toy.rs");
    let text = fs::read_to_string(&module).expect("the module");
    let refused = write_files(&root, "toy", "toy").expect_err("the second write is refused");
    assert!(refused.contains("already exists"), "{refused}");
    assert_eq!(
        fs::read_to_string(&module).expect("the module"),
        text,
        "a refused write does not touch the existing files"
    );
    let _ = fs::remove_dir_all(root);
}

/// A synthetic checkout with the five anchor files, so `register` edits real text.
/// 一个带五个锚点文件的合成检出，让 `register` 编辑真实文本。
fn anchored_checkout(label: &str) -> std::path::PathBuf {
    let root = checkout(label);
    fs::create_dir_all(root.join("multilang/src")).expect("fixture src");
    fs::create_dir_all(root.join("toolchain")).expect("fixture toolchain");
    fs::write(root.join("multilang/src/languages.rs"), "pub mod rust;\n").expect("languages");
    fs::write(root.join("multilang/src/lib.rs"), "pub mod languages;\n").expect("lib");
    fs::write(
        root.join("multilang/Cargo.toml"),
        "[features]\nlang-python = [\"semantics\"]\n",
    )
    .expect("multilang manifest");
    fs::write(
        root.join("toolchain/Cargo.toml"),
        "[features]\nlang-python = [\"xirang-multilang/lang-python\"]\n",
    )
    .expect("toolchain manifest");
    fs::write(
        root.join("multilang/src/registration.rs"),
        "        #[cfg(feature = \"lang-python\")]\n        {\n            adapters.push(&PythonLanguage);\n        }\n",
    )
    .expect("registration");
    root
}

/// `register` writes every registration line, and a second run refuses.
/// `register` 写入全部注册行，第二次运行被拒绝。
#[test]
fn registration_is_written_once() {
    let root = anchored_checkout("register");
    let edited = register(&root, "toy").expect("the first registration");
    assert_eq!(edited.len(), 5, "five files are edited");
    let languages = fs::read_to_string(root.join("multilang/src/languages.rs")).expect("languages");
    assert!(
        languages.contains("#[cfg(feature = \"lang-toy\")]") && languages.contains("pub mod toy;"),
        "{languages}"
    );
    let lib = fs::read_to_string(root.join("multilang/src/lib.rs")).expect("lib");
    assert!(lib.contains("pub use languages::toy::Toy;"), "{lib}");
    let manifest = fs::read_to_string(root.join("multilang/Cargo.toml")).expect("manifest");
    assert!(
        manifest.contains("lang-toy = [\"semantics\"]"),
        "{manifest}"
    );
    let forward = fs::read_to_string(root.join("toolchain/Cargo.toml")).expect("forward");
    assert!(
        forward.contains("lang-toy = [\"xirang-multilang/lang-toy\"]"),
        "{forward}"
    );
    let registry =
        fs::read_to_string(root.join("multilang/src/registration.rs")).expect("registry");
    assert!(
        registry.contains("adapters.push(&crate::languages::toy::Toy);"),
        "{registry}"
    );
    let refused = register(&root, "toy").expect_err("the second run is refused");
    assert!(refused.contains("already mentions"), "{refused}");
    let _ = fs::remove_dir_all(root);
}

/// A checkout missing an anchor is refused by file, before anything is written.
/// 缺少锚点的检出按文件拒绝，且在写入任何东西之前。
#[test]
fn a_missing_anchor_is_refused_by_file() {
    let root = anchored_checkout("no-anchor");
    fs::write(root.join("multilang/src/languages.rs"), "pub mod other;\n").expect("languages");
    let refused = register(&root, "toy").expect_err("the anchor is gone");
    assert!(
        refused.contains("multilang/src/languages.rs") && refused.contains("pub mod rust;"),
        "{refused}"
    );
    let lib = fs::read_to_string(root.join("multilang/src/lib.rs")).expect("lib");
    assert_eq!(lib, "pub mod languages;\n", "nothing was written");
    let _ = fs::remove_dir_all(root);
}
