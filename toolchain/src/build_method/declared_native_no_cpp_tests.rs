//! Language selection without the C++ adapter: unrelated files name no language, markers do.
//! 没有 C++ 适配器时的语言选择：无关文件不点名语言，标记会。

use super::*;

fn scratch(name: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "xirang-no-cpp-{name}-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("the scratch directory");
    root
}

/// A `.cpp` with no declaration marker names no language: the package takes the default.
/// 没有声明标记的 `.cpp` 不点名语言：包取默认。
#[test]
fn an_unrelated_cpp_file_names_no_language() {
    let root = scratch("unrelated");
    std::fs::write(root.join("helper.cpp"), "int helper() { return 1; }\n").expect("the file");
    assert_eq!(
        declared_language(&root).expect("selection"),
        None,
        "a file's extension alone is not a declaration"
    );
}

/// A `.cpp` that mentions a marker selects C++, so the read refuses with the enable method.
/// 提到标记的 `.cpp` 选择 C++，因此读取按名拒绝并给启用方法。
#[test]
fn a_marked_cpp_file_selects_cpp_and_refuses_without_the_adapter() {
    let root = scratch("marked");
    std::fs::write(
        root.join("face.cpp"),
        "#include \"xirang/soil.hpp\"\nXIRANG_FACE(r, (x));\n",
    )
    .expect("the file");
    assert_eq!(
        declared_language(&root).expect("selection"),
        Some(NativeLanguage::Cpp)
    );
    let error = read(&root)
        .err()
        .expect("the C++ adapter is not compiled in");
    assert!(error.contains("lang-cpp"), "{error}");
}

/// `[native] language` is explicit and wins over the markers.
/// `[native] language` 是显式声明，优先于标记。
#[test]
fn a_language_table_selects_the_language() {
    let root = scratch("language-table");
    std::fs::write(
        root.join("xirang.toml"),
        "[native]\nlanguage = \"zig\"\nsources = [\"a.zig\"]\n",
    )
    .expect("the manifest");
    assert_eq!(
        declared_language(&root).expect("selection"),
        Some(NativeLanguage::Zig)
    );
    std::fs::write(
        root.join("xirang.toml"),
        "[native]\nlanguage = \"c++\"\nsources = [\"a.cpp\"]\n",
    )
    .expect("the manifest");
    assert_eq!(
        declared_language(&root).expect("selection"),
        Some(NativeLanguage::Cpp)
    );
    std::fs::write(root.join("xirang.toml"), "[native]\nlanguage = \"rust\"\n")
        .expect("the manifest");
    let error = declared_language(&root).expect_err("an unknown language is refused");
    assert!(error.contains("rust"), "{error}");
}
