#![cfg(feature = "cli")]
//! End-to-end: `xirang move` moves one unit directory and rewrites exactly the path dependencies
//! that point at it, in one transaction; units that do not depend on it are not rewritten, and a
//! move that cannot be spelled (or a destination that exists) changes nothing.
//! 端到端：`xirang move` 一次事务搬动一个单元目录，并**恰好**重写指向它的路径依赖；不依赖它的
//! 单元不被改写；写不出来的搬迁（或已存在的目标）什么都不改。

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// The binary under test.
/// 被测二进制。
fn binary() -> &'static str {
    env!("CARGO_BIN_EXE_xirang")
}

/// A fresh scratch directory.
/// 一个全新的临时目录。
fn scratch(label: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "xirang-move-{label}-{}-{:?}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&path).unwrap();
    path
}

/// Run the binary in `directory`.
/// 在 `directory` 里运行该二进制。
fn invoke(directory: &Path, arguments: &[&str]) -> Output {
    Command::new(binary())
        .args(arguments)
        .current_dir(directory)
        .output()
        .unwrap()
}

/// Both streams, for a failure message.
/// 两条输出流，供失败信息使用。
fn both(output: &Output) -> String {
    format!(
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

/// Write a C++ library with an optional export and path dependencies.
/// 写一个 C++ 库：可选导出、可点名路径依赖。
fn library(
    directory: &Path,
    name: &str,
    source: &str,
    export: &str,
    dependencies: &[(&str, &str)],
) {
    std::fs::create_dir_all(directory.join("src")).unwrap();
    let mut table = String::new();
    for (key, path) in dependencies {
        table.push_str(&format!("{key} = {{ path = \"{path}\" }}\n"));
    }
    std::fs::write(
        directory.join("xirang.toml"),
        format!(
            "schema = 1\n\n[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nkind = \"library\"\n\n\
             [language.cpp]\nstandard = \"17\"\n\n[[export]]\nname = \"{export}\"\nparams = [\"i64\", \"i64\"]\nreturns = \"i64\"\n\n\
             [dependencies]\n{table}"
        ),
    )
    .unwrap();
    std::fs::write(directory.join("src").join(format!("{name}.cpp")), source).unwrap();
}

#[test]
fn a_move_rewrites_its_dependents_and_leaves_the_rest_alone() {
    let root = scratch("dependents");
    let math = root.join("math");
    library(
        &math,
        "math",
        "#include <cstdint>\n\nstd::int64_t add(std::int64_t a, std::int64_t b) {\n    return a + b;\n}\n",
        "add",
        &[],
    );
    let twice = root.join("twice");
    library(
        &twice,
        "twice",
        "#include <cstdint>\n#include \"math/exports.h\"\n\nstd::int64_t twice(std::int64_t a, std::int64_t b) {\n    return add(a, b) * 2;\n}\n",
        "twice",
        &[("math", "../math")],
    );
    let solo = root.join("solo");
    library(
        &solo,
        "solo",
        "#include <cstdint>\n\nstd::int64_t solo_value(std::int64_t a, std::int64_t b) {\n    return a + b;\n}\n",
        "solo_value",
        &[],
    );
    let app = root.join("app");
    std::fs::create_dir_all(app.join("src")).unwrap();
    std::fs::create_dir_all(app.join("tests")).unwrap();
    std::fs::write(
        app.join("xirang.toml"),
        "schema = 1\n\n[package]\nname = \"app\"\nversion = \"0.1.0\"\nkind = \"library\"\n\n\
         [language.cpp]\nstandard = \"17\"\n\n[dependencies]\nmath = { path = \"../math\" }\n\
         twice = { path = \"../twice\" }\nsolo = { path = \"../solo\" }\n",
    )
    .unwrap();
    std::fs::write(
        app.join("src/app.cpp"),
        "#include <cstdint>\n\nstd::int64_t app_marker() {\n    return 1;\n}\n",
    )
    .unwrap();
    std::fs::write(
        app.join("tests/add_test.cpp"),
        "#include <cassert>\n#include \"math/exports.h\"\n#include \"twice/exports.h\"\n#include \"solo/exports.h\"\n\nint main() {\n    assert(add(2, 3) == 5);\n    assert(twice(2, 2) == 8);\n    assert(solo_value(1, 1) == 2);\n    return 0;\n}\n",
    )
    .unwrap();
    std::fs::create_dir_all(root.join("vendor")).unwrap();
    let solo_manifest = std::fs::read(solo.join("xirang.toml")).unwrap();
    let math_source = std::fs::read(math.join("src/math.cpp")).unwrap();
    let moved = invoke(&app, &["move", "math", "../vendor/math"]);
    assert!(moved.status.success(), "{}", both(&moved));
    let text = both(&moved);
    assert!(text.contains("rewrote 2 manifest(s)"), "{text}");
    let math_manifest = std::fs::read_to_string(app.join("xirang.toml")).unwrap();
    assert!(math_manifest.contains("../vendor/math"), "{math_manifest}");
    let twice_manifest = std::fs::read_to_string(twice.join("xirang.toml")).unwrap();
    assert!(
        twice_manifest.contains("../vendor/math"),
        "{twice_manifest}"
    );
    assert_eq!(
        std::fs::read(solo.join("xirang.toml")).unwrap(),
        solo_manifest,
        "a unit that does not depend on the moved one is not rewritten"
    );
    assert_eq!(
        std::fs::read(root.join("vendor/math/src/math.cpp")).unwrap(),
        math_source
    );
    let tested = invoke(&app, &["test"]);
    assert!(tested.status.success(), "{}", both(&tested));
    let updated = invoke(&app, &["update"]);
    assert!(updated.status.success(), "{}", both(&updated));
    let locked = invoke(&app, &["build", "--locked"]);
    assert!(locked.status.success(), "{}", both(&locked));
}

#[test]
fn a_move_into_an_existing_directory_changes_nothing() {
    let root = scratch("existing");
    let math = root.join("math");
    library(
        &math,
        "math",
        "#include <cstdint>\n\nstd::int64_t add(std::int64_t a, std::int64_t b) {\n    return a + b;\n}\n",
        "add",
        &[],
    );
    let app = root.join("app");
    std::fs::create_dir_all(app.join("src")).unwrap();
    std::fs::create_dir_all(app.join("tests")).unwrap();
    std::fs::write(
        app.join("xirang.toml"),
        "schema = 1\n\n[package]\nname = \"app\"\nversion = \"0.1.0\"\nkind = \"library\"\n\n\
         [language.cpp]\nstandard = \"17\"\n\n[dependencies]\nmath = { path = \"../math\" }\n",
    )
    .unwrap();
    std::fs::write(app.join("src/app.cpp"), "int app_marker() { return 1; }\n").unwrap();
    std::fs::write(
        app.join("tests/add_test.cpp"),
        "#include <cassert>\n#include \"math/exports.h\"\n\nint main() {\n    assert(add(2, 3) == 5);\n    return 0;\n}\n",
    )
    .unwrap();
    std::fs::create_dir_all(root.join("vendor/math")).unwrap();
    let before = std::fs::read(app.join("xirang.toml")).unwrap();
    let refused = invoke(&app, &["move", "math", "../vendor/math"]);
    assert!(!refused.status.success(), "{}", both(&refused));
    assert!(math.join("xirang.toml").is_file());
    assert_eq!(std::fs::read(app.join("xirang.toml")).unwrap(), before);
}

#[test]
fn a_unit_reached_through_a_source_is_refused_by_name() {
    let root = scratch("source");
    let team = root.join("team");
    let math = team.join("math");
    library(
        &math,
        "math",
        "#include <cstdint>\n\nstd::int64_t add(std::int64_t a, std::int64_t b) {\n    return a + b;\n}\n",
        "add",
        &[],
    );
    let app = root.join("app");
    std::fs::create_dir_all(app.join("src")).unwrap();
    std::fs::create_dir_all(app.join("tests")).unwrap();
    std::fs::write(
        app.join("xirang.toml"),
        "schema = 1\n\n[package]\nname = \"app\"\nversion = \"0.1.0\"\nkind = \"library\"\n\n\
         [language.cpp]\nstandard = \"17\"\n\n[sources]\nteam = { path = \"../team\" }\n\n\
         [dependencies]\nmath = { source = \"team\", package = \"math\" }\n",
    )
    .unwrap();
    std::fs::write(app.join("src/app.cpp"), "int app_marker() { return 1; }\n").unwrap();
    std::fs::write(
        app.join("tests/add_test.cpp"),
        "#include <cassert>\n#include \"math/exports.h\"\n\nint main() {\n    assert(add(2, 3) == 5);\n    return 0;\n}\n",
    )
    .unwrap();
    let refused = invoke(&app, &["move", "math", "../vendor/math"]);
    assert!(!refused.status.success(), "{}", both(&refused));
    assert!(both(&refused).contains("source"), "{}", both(&refused));
    assert!(math.join("xirang.toml").is_file());
}

#[test]
fn a_move_inside_the_unit_itself_is_refused() {
    let root = scratch("inside");
    let math = root.join("math");
    library(
        &math,
        "math",
        "int add(int a, int b) { return a + b; }\n",
        "add",
        &[],
    );
    let app = root.join("app");
    std::fs::create_dir_all(app.join("src")).unwrap();
    std::fs::write(
        app.join("xirang.toml"),
        "schema = 1\n\n[package]\nname = \"app\"\nversion = \"0.1.0\"\nkind = \"library\"\n\n\
         [language.cpp]\nstandard = \"17\"\n\n[dependencies]\nmath = { path = \"../math\" }\n",
    )
    .unwrap();
    std::fs::write(app.join("src/app.cpp"), "int app_marker() { return 1; }\n").unwrap();
    let refused = invoke(&app, &["move", "math", "../math/inside"]);
    assert!(!refused.status.success(), "{}", both(&refused));
    assert!(math.join("xirang.toml").is_file());
}

/// Write a C++ library whose manifest declares one export and no dependencies.
/// 写一个 C++ 库清单：一条导出、无依赖。
fn exporting(directory: &Path, name: &str, export: &str, body: &str) {
    std::fs::create_dir_all(directory.join("src")).unwrap();
    std::fs::write(
        directory.join("xirang.toml"),
        format!(
            "schema = 1\n\n[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nkind = \"library\"\n\n\
             [language.cpp]\nstandard = \"17\"\n\n[[export]]\nname = \"{export}\"\nparams = []\nreturns = \"i64\"\n\n\
             [dependencies]\n"
        ),
    )
    .unwrap();
    std::fs::write(directory.join("src").join(format!("{name}.cpp")), body).unwrap();
}

/// Every file under one directory, keyed by relative path.
/// 一个目录下的每个文件，按相对路径索引。
fn snapshot(directory: &Path) -> Vec<(PathBuf, Vec<u8>)> {
    fn walk_files(directory: &Path, base: &Path, files: &mut Vec<(PathBuf, Vec<u8>)>) {
        for entry in std::fs::read_dir(directory).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                walk_files(&path, base, files);
            } else {
                files.push((
                    path.strip_prefix(base).unwrap().to_path_buf(),
                    std::fs::read(&path).unwrap(),
                ));
            }
        }
    }
    let mut files = Vec::new();
    walk_files(directory, directory, &mut files);
    files.sort();
    files
}

#[test]
fn a_source_move_rewrites_the_root_manifest_and_its_overrides() {
    let root = scratch("source");
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::create_dir_all(root.join("tests")).unwrap();
    std::fs::write(
        root.join("xirang.toml"),
        "schema = 1\n\n[package]\nname = \"app\"\nversion = \"0.1.0\"\nkind = \"library\"\n\n\
         [language.cpp]\nstandard = \"17\"\n\n\
         [sources]\nteam = { path = \"team\" }\n\n\
         [overrides]\nteam/greeter = { path = \"team/pinned/greeter\" }\n\n\
         [dependencies]\ngreeter = { source = \"team\", package = \"greeter\" }\n",
    )
    .unwrap();
    std::fs::write(
        root.join("src/app.cpp"),
        "#include <cstdint>\n\nstd::int64_t marker() { return 1; }\n",
    )
    .unwrap();
    std::fs::write(
        root.join("tests/greet_test.cpp"),
        "#include <cassert>\n#include \"greeter/exports.h\"\n\nint main() {\n    assert(greet() == 7);\n    return 0;\n}\n",
    )
    .unwrap();
    exporting(
        &root.join("team/greeter"),
        "greeter",
        "greet",
        "#include <cstdint>\n\nstd::int64_t greet() { return 7; }\n",
    );
    exporting(
        &root.join("team/pinned/greeter"),
        "greeter",
        "greet",
        "#include <cstdint>\n\nstd::int64_t greet() { return 7; }\n",
    );
    let before = invoke(&root, &["test"]);
    assert!(before.status.success(), "{}", both(&before));
    let team = snapshot(&root.join("team"));
    std::fs::create_dir_all(root.join("vendor")).unwrap();
    let moved = invoke(&root, &["move", "--source", "team", "vendor/team"]);
    assert!(moved.status.success(), "{}", both(&moved));
    let manifest = std::fs::read_to_string(root.join("xirang.toml")).unwrap();
    assert!(
        manifest.contains("team = { path = \"vendor/team\" }"),
        "the source declaration was not rewritten:\n{manifest}"
    );
    assert!(
        manifest.contains("team/greeter = { path = \"vendor/team/pinned/greeter\" }"),
        "the override that pointed inside the source was not rewritten:\n{manifest}"
    );
    assert_eq!(
        snapshot(&root.join("vendor/team")),
        team,
        "the moved source's own bytes changed"
    );
    // The lock still records the old location, so `--locked` refuses until `update` records the move.
    // 锁里仍记着旧位置，因此在 `update` 记录这次搬动之前 `--locked` 会拒绝。
    let locked = invoke(&root, &["build", "--locked"]);
    assert!(!locked.status.success(), "{}", both(&locked));
    let updated = invoke(&root, &["update"]);
    assert!(updated.status.success(), "{}", both(&updated));
    let rebuilt = invoke(&root, &["build", "--locked"]);
    assert!(rebuilt.status.success(), "{}", both(&rebuilt));
    let after = invoke(&root, &["test"]);
    assert!(after.status.success(), "{}", both(&after));
}

#[test]
fn a_source_that_is_not_a_local_path_is_refused_by_name() {
    let root = scratch("source-git");
    std::fs::create_dir_all(root.join("vendor")).unwrap();
    std::fs::write(
        root.join("xirang.toml"),
        "schema = 1\n\n[package]\nname = \"app\"\nversion = \"0.1.0\"\nkind = \"library\"\n\n\
         [language.cpp]\nstandard = \"17\"\n\n\
         [sources]\nteam = { git = \"file:///nonexistent\", rev = \"abc\" }\n\n\
         [dependencies]\ngreeter = { source = \"team\", package = \"greeter\" }\n",
    )
    .unwrap();
    let refused = invoke(&root, &["move", "--source", "team", "vendor/team"]);
    assert!(!refused.status.success());
    let stderr = String::from_utf8_lossy(&refused.stderr);
    assert!(
        stderr.contains("git source"),
        "the refusal does not say why:\n{stderr}"
    );
    assert!(!root.join("vendor/team").exists());
}

#[test]
fn a_source_move_into_an_existing_directory_changes_nothing() {
    let root = scratch("source-exists");
    std::fs::create_dir_all(root.join("team/greeter/src")).unwrap();
    std::fs::create_dir_all(root.join("vendor/team")).unwrap();
    std::fs::write(
        root.join("xirang.toml"),
        "schema = 1\n\n[package]\nname = \"app\"\nversion = \"0.1.0\"\nkind = \"library\"\n\n\
         [language.cpp]\nstandard = \"17\"\n\n\
         [sources]\nteam = { path = \"team\" }\n\n\
         [dependencies]\ngreeter = { source = \"team\", package = \"greeter\" }\n",
    )
    .unwrap();
    let before = std::fs::read_to_string(root.join("xirang.toml")).unwrap();
    let refused = invoke(&root, &["move", "--source", "team", "vendor/team"]);
    assert!(!refused.status.success());
    assert!(
        root.join("team/greeter/src").is_dir(),
        "the source moved anyway"
    );
    assert_eq!(
        std::fs::read_to_string(root.join("xirang.toml")).unwrap(),
        before,
        "the manifest changed although the move was refused"
    );
}
