//! Pins for the shared native driver: the `[native]` recipe it reads and the discovery a language's
//! extensions drive. No compiler runs here — the driver's process half is exercised by each
//! language's own build gate (`tools/xirang-mixed-shop` for C++).
//! 共享原生驱动的钉子：它读的 `[native]` 配方，以及由语言扩展名驱动的发现。这里不跑编译器——驱动的
//! 进程那一半由各语言自己的构建门禁覆盖（C++ 是 `tools/xirang-mixed-shop`）。

use std::path::PathBuf;

use super::{NativeSpec, read_recipe};

/// A throwaway package with the given files.
/// 一个带给定文件的一次性包。
fn package(label: &str, entries: &[(&str, &str)]) -> PathBuf {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "xirang-kit-native-{label}-{}-{sequence}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    for (relative, text) in entries {
        let path = root.join(relative);
        std::fs::create_dir_all(path.parent().unwrap()).expect("fixture parent");
        std::fs::write(path, text).expect("fixture file");
    }
    std::fs::create_dir_all(&root).expect("fixture root");
    root
}

/// A toy language: `.toy` sources, a marker word, a compiler name and one SDK directory.
/// 一门玩具语言：`.toy` 源码、一个标记词、一个编译器名与一个 SDK 目录。
const SPEC: NativeSpec = NativeSpec {
    extensions: &[".toy"],
    default_recipe: toy_recipe,
    include_dirs: toy_includes,
    skip_directories: &["target", ".git"],
    object_command: toy_object_command,
};

/// The toy compiler's flag shape: `toycc <args> -c <source> -o <object> -I<dir>…`.
/// 玩具编译器的参数形状。
fn toy_object_command(
    compile: &crate::languages::kit::native::ObjectCommand<'_>,
) -> (String, Vec<String>) {
    let mut arguments = compile.recipe.arguments.clone();
    arguments.push("-c".to_owned());
    arguments.push(compile.source.display().to_string());
    arguments.push("-o".to_owned());
    arguments.push(compile.object.display().to_string());
    arguments.push(format!("-I{}", compile.out_dir.display()));
    for include in compile.include_dirs {
        arguments.push(format!("-I{}", include.display()));
    }
    (compile.recipe.program.clone(), arguments)
}

fn toy_recipe(_sources: &[String]) -> (String, Vec<String>) {
    ("toycc".to_owned(), vec!["-std=toy1".to_owned()])
}

fn toy_includes() -> Vec<PathBuf> {
    vec![PathBuf::from("/toy/sdk")]
}

fn mentions_toy(text: &str) -> bool {
    text.contains("TOY_SOIL")
}

/// The manifest's `[native]` table wins over the language's defaults, field by field.
/// 清单的 `[native]` 表逐字段压过语言的默认值。
#[test]
fn a_manifest_recipe_overrides_the_languages_defaults() {
    let root = package(
        "recipe",
        &[
            (
                "xirang.toml",
                "[native]\nprogram = \"mytoy\"\nargs = [\"-O2\", \"-fpic\"]\ninclude = [\"inc\"]\n\
                 archive = \"myar\"\nsources = [\"native/a.toy\"]\n",
            ),
            ("native/a.toy", "TOY_SOIL\n"),
        ],
    );
    let (recipe, sources) = read_recipe(&root, &SPEC, mentions_toy)
        .expect("the recipe reads")
        .expect("the manifest names a source");
    assert_eq!(recipe.program, "mytoy");
    assert_eq!(recipe.arguments, vec!["-O2".to_owned(), "-fpic".to_owned()]);
    assert_eq!(recipe.includes, vec!["inc".to_owned()]);
    assert_eq!(recipe.archive, "myar");
    assert_eq!(sources, vec!["native/a.toy".to_owned()]);
    let _ = std::fs::remove_dir_all(root);
}

/// With no `[native]` table, the language's extensions and marker discover the sources, and the
/// language's defaults fill the recipe.
/// 没有 `[native]` 表时，语言的扩展名与标记发现源码，语言的默认值填配方。
#[test]
fn discovery_uses_the_languages_extensions_and_marker() {
    let root = package(
        "discovery",
        &[
            ("native/a.toy", "TOY_SOIL\n"),
            ("native/helper.toy", "// included by a.toy, no marker\n"),
            ("native/notes.txt", "TOY_SOIL\n"),
        ],
    );
    let (recipe, sources) = read_recipe(&root, &SPEC, mentions_toy)
        .expect("the discovery runs")
        .expect("one source carries the marker");
    assert_eq!(sources, vec!["native/a.toy".to_owned()]);
    assert_eq!(recipe.program, "toycc");
    assert_eq!(recipe.arguments, vec!["-std=toy1".to_owned()]);
    assert_eq!(recipe.archive, "ar", "the archiver default is shared");
    assert_eq!(
        (SPEC.include_dirs)(),
        vec![PathBuf::from("/toy/sdk")],
        "the driver asks the language for its SDK directory"
    );
    let _ = std::fs::remove_dir_all(root);
}

/// A package with nothing to compile has no recipe — not an empty one.
/// 没有东西可编译的包没有配方——不是空配方。
#[test]
fn a_package_with_no_native_source_has_no_recipe() {
    let root = package("empty", &[("src/lib.rs", "fn main() {}\n")]);
    assert!(
        read_recipe(&root, &SPEC, mentions_toy)
            .expect("the discovery runs")
            .is_none(),
        "no extension matched, so there is nothing to compile"
    );
    let _ = std::fs::remove_dir_all(root);
}
