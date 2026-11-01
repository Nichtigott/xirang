#![cfg(feature = "cli")]
//! End-to-end: a dependency that two consumers both name is one unit (a diamond, not a cycle),
//! and a real dependency cycle is refused by name instead of being silently accepted.
//! 端到端：两个消费者都点名的依赖是同一个单元（菱形，不是环）；真正的依赖环按名拒绝，而不是被
//! 静默接受。

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
        "xirang-graph-{label}-{}-{:?}",
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

/// Write one C++ library that may declare one export and name path dependencies.
/// 写一个 C++ 库：可以声明一个导出、可以点名若干路径依赖。
fn library(
    directory: &Path,
    name: &str,
    source: &str,
    export: Option<&str>,
    dependencies: &[(&str, &str)],
) {
    std::fs::create_dir_all(directory.join("src")).unwrap();
    let export_table = match export {
        Some(export) => {
            format!("[[export]]\nname = \"{export}\"\nparams = []\nreturns = \"i64\"\n\n")
        }
        None => String::new(),
    };
    let mut dependencies_table = String::new();
    for (key, path) in dependencies {
        dependencies_table.push_str(&format!("{key} = {{ path = \"{path}\" }}\n"));
    }
    std::fs::write(
        directory.join("xirang.toml"),
        format!(
            "schema = 1\n\n[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nkind = \"library\"\n\n\
             [language.cpp]\nstandard = \"17\"\n\n{export_table}[dependencies]\n{dependencies_table}"
        ),
    )
    .unwrap();
    std::fs::write(directory.join("src").join(format!("{name}.cpp")), source).unwrap();
}

#[test]
fn a_diamond_lists_its_shared_dependency_once_and_links() {
    let root = scratch("diamond");
    let base = root.join("base");
    let a = root.join("a");
    let b = root.join("b");
    let app = root.join("app");
    library(
        &base,
        "base",
        "#include <cstdint>\n\nstd::int64_t base_value() {\n    return 40;\n}\n",
        Some("base_value"),
        &[],
    );
    library(
        &a,
        "a",
        "#include <cstdint>\n#include \"base/exports.h\"\n\nstd::int64_t a_value() {\n    return base_value() + 1;\n}\n",
        Some("a_value"),
        &[("base", "../base")],
    );
    library(
        &b,
        "b",
        "#include <cstdint>\n#include \"base/exports.h\"\n\nstd::int64_t b_value() {\n    return base_value() + 2;\n}\n",
        Some("b_value"),
        &[("base", "../base")],
    );
    std::fs::create_dir_all(app.join("src")).unwrap();
    std::fs::create_dir_all(app.join("tests")).unwrap();
    std::fs::write(
        app.join("xirang.toml"),
        "schema = 1\n\n[package]\nname = \"app\"\nversion = \"0.1.0\"\nkind = \"library\"\n\n\
         [language.cpp]\nstandard = \"17\"\n\n[dependencies]\na = { path = \"../a\" }\n\
         b = { path = \"../b\" }\n",
    )
    .unwrap();
    std::fs::write(
        app.join("src/app.cpp"),
        "#include <cstdint>\n\nstd::int64_t app_marker() {\n    return 1;\n}\n",
    )
    .unwrap();
    std::fs::write(
        app.join("tests/diamond_test.cpp"),
        "#include <cassert>\n#include \"a/exports.h\"\n#include \"b/exports.h\"\n\nint main() {\n    assert(a_value() == 41);\n    assert(b_value() == 42);\n    return 0;\n}\n",
    )
    .unwrap();
    let tested = invoke(&app, &["test"]);
    assert!(tested.status.success(), "{}", both(&tested));
    // The shared dependency is one unit, not two: the build lists it once.
    let planned = invoke(&app, &["plan"]);
    assert!(planned.status.success(), "{}", both(&planned));
    let text = String::from_utf8_lossy(&planned.stdout);
    assert!(
        text.contains("base.cpp"),
        "the shared source has to be in the plan: {text}"
    );
    assert!(!text.contains("cycle"), "a diamond is not a cycle: {text}");
    // Red control: an expectation that the linked libraries cannot satisfy fails the test.
    std::fs::write(
        app.join("tests/diamond_test.cpp"),
        "#include <cassert>\n#include \"a/exports.h\"\n#include \"b/exports.h\"\n\nint main() {\n    assert(a_value() == 0);\n    assert(b_value() == 42);\n    return 0;\n}\n",
    )
    .unwrap();
    let failed = invoke(&app, &["test"]);
    assert!(!failed.status.success(), "{}", both(&failed));
}

#[test]
fn a_dependency_cycle_is_refused_by_name() {
    let root = scratch("cycle");
    let app = root.join("app");
    let a = root.join("a");
    library(
        &a,
        "a",
        "#include <cstdint>\n\nstd::int64_t a_value() {\n    return 1;\n}\n",
        Some("a_value"),
        &[("app", "../app")],
    );
    library(
        &app,
        "app",
        "#include <cstdint>\n\nstd::int64_t app_marker() {\n    return 1;\n}\n",
        None,
        &[("a", "../a")],
    );
    let refused = invoke(&app, &["build"]);
    assert!(!refused.status.success(), "{}", both(&refused));
    let message = both(&refused);
    assert!(message.contains("form a cycle"), "{message}");
    assert!(message.contains("cycle through"), "{message}");
}

/// Write a library reached through a named `[sources]` entry instead of a path dependency.
/// 写一个通过具名 `[sources]` 条目（而不是路径依赖）到达的库。
fn sourced_library(base: &Path, package: &str, export: &str, source: &str) {
    let directory = base.join(package);
    std::fs::create_dir_all(directory.join("src")).unwrap();
    std::fs::write(
        directory.join("xirang.toml"),
        format!(
            "schema = 1\n\n[package]\nname = \"{package}\"\nversion = \"0.1.0\"\nkind = \"library\"\n\n\
             [language.cpp]\nstandard = \"17\"\n\n[[export]]\nname = \"{export}\"\nparams = []\nreturns = \"i64\"\n\n\
             [dependencies]\n"
        ),
    )
    .unwrap();
    std::fs::write(directory.join("src").join(format!("{package}.cpp")), source).unwrap();
}

#[test]
fn a_dependency_field_may_be_named_differently_from_its_package() {
    let root = scratch("field-name");
    sourced_library(
        &root.join("team"),
        "greeter",
        "answer",
        "#include <cstdint>\nstd::int64_t answer() { return 7; }\n",
    );
    let app = root.join("app");
    std::fs::create_dir_all(app.join("src")).unwrap();
    std::fs::create_dir_all(app.join("tests")).unwrap();
    std::fs::write(
        app.join("xirang.toml"),
        "schema = 1\n\n[package]\nname = \"app\"\nversion = \"0.1.0\"\nkind = \"library\"\n\n\
         [language.cpp]\nstandard = \"17\"\n\n[sources]\nteam = \"../team\"\n\n\
         [dependencies]\nwest = { source = \"team\", package = \"greeter\" }\n",
    )
    .unwrap();
    std::fs::write(app.join("src/lib.cpp"), "int app_marker() { return 1; }\n").unwrap();
    std::fs::write(
        app.join("tests/answer_test.cpp"),
        "#include <cassert>\nextern \"C\" long long answer(void);\n\nint main() {\n    assert(answer() == 7);\n    return 0;\n}\n",
    )
    .unwrap();
    let tested = invoke(&app, &["test"]);
    assert!(tested.status.success(), "{}", both(&tested));
}

#[test]
fn two_sources_may_hold_packages_of_the_same_name() {
    let root = scratch("same-name");
    sourced_library(
        &root.join("west-src"),
        "greeter",
        "west_value",
        "#include <cstdint>\nstd::int64_t west_value() { return 11; }\n",
    );
    sourced_library(
        &root.join("east-src"),
        "greeter",
        "east_value",
        "#include <cstdint>\nstd::int64_t east_value() { return 22; }\n",
    );
    let app = root.join("app");
    std::fs::create_dir_all(app.join("src")).unwrap();
    std::fs::create_dir_all(app.join("tests")).unwrap();
    std::fs::write(
        app.join("xirang.toml"),
        "schema = 1\n\n[package]\nname = \"app\"\nversion = \"0.1.0\"\nkind = \"library\"\n\n\
         [language.cpp]\nstandard = \"17\"\n\n[sources]\nwest = \"../west-src\"\neast = \"../east-src\"\n\n\
         [dependencies]\nwest_pkg = { source = \"west\", package = \"greeter\" }\n\
         east_pkg = { source = \"east\", package = \"greeter\" }\n",
    )
    .unwrap();
    std::fs::write(app.join("src/lib.cpp"), "int app_marker() { return 1; }\n").unwrap();
    std::fs::write(
        app.join("tests/both_test.cpp"),
        "#include <cassert>\nextern \"C\" long long west_value(void);\nextern \"C\" long long east_value(void);\n\n\
         int main() {\n    assert(west_value() == 11);\n    assert(east_value() == 22);\n    return 0;\n}\n",
    )
    .unwrap();
    let tested = invoke(&app, &["test"]);
    assert!(tested.status.success(), "{}", &both(&tested));
    // Two units share the package name, so their archives must be two distinct files: one would
    // silently stand in for the other otherwise.
    // 两个单元共享包名，因此它们的归档必须是两个不同文件：否则其中一个会静默冒充另一个。
    let mut archives = Vec::new();
    let build = app.join("target/xirang/build");
    for entry in std::fs::read_dir(&build).unwrap() {
        let plan = entry.unwrap().path().join("lib");
        if let Ok(libraries) = std::fs::read_dir(&plan) {
            for library in libraries {
                let path = library.unwrap().path();
                if path.extension().is_some_and(|extension| extension == "a") {
                    archives.push(path.file_name().unwrap().to_string_lossy().into_owned());
                }
            }
        }
    }
    archives.sort();
    // The root is a library too, so the plan holds three archives; the two greeters must be
    // distinct files rather than one standing in for the other.
    // 根自己也是库，所以计划里有三个归档；两个 greeter 必须是不同文件，而不是一个冒充另一个。
    assert_eq!(archives.len(), 3, "{archives:?}");
    let greeters: Vec<&String> = archives
        .iter()
        .filter(|name| name.starts_with("libgreeter"))
        .collect();
    assert_eq!(greeters.len(), 2, "{archives:?}");
    assert_ne!(greeters[0], greeters[1], "{archives:?}");
}
