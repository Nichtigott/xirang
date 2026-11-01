#![cfg(feature = "cli")]
//! End-to-end: the lock records a unit's export surface and its test list, so a change that
//! removes both an export and the test that used it still shows up under `--locked` instead of
//! cancelling out; and a legal move (a dependency directory moved, one manifest path rewritten)
//! is executable without touching the moved unit's bytes.
//! 端到端：锁记录一个单元的导出面与测试清单，因此“删掉导出、再删掉用它的测试”这种两错相抵的
//! 改动在 `--locked` 下仍会显形；合法的搬迁（依赖目录移动、只改根清单里的一条路径）可以执行，
//! 且被搬单元的字节不被改动。

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
        "xirang-lock-{label}-{}-{:?}",
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

/// Write the `math` library: one C++ source and (optionally) one export.
/// 写 `math` 库：一个 C++ 源文件与（可选）一个导出。
fn math_library(directory: &Path, export: bool, dependency: &str) {
    std::fs::create_dir_all(directory.join("src")).unwrap();
    let table = if export {
        "[[export]]\nname = \"add\"\nparams = [\"i64\", \"i64\"]\nreturns = \"i64\"\n\n"
    } else {
        ""
    };
    std::fs::write(
        directory.join("xirang.toml"),
        format!(
            "schema = 1\n\n[package]\nname = \"math\"\nversion = \"0.1.0\"\nkind = \"library\"\n\n\
             [language.cpp]\nstandard = \"17\"\n\n{table}[dependencies]\n{dependency}"
        ),
    )
    .unwrap();
    std::fs::write(
        directory.join("src/math.cpp"),
        "#include <cstdint>\n\nstd::int64_t add(std::int64_t a, std::int64_t b) {\n    return a + b;\n}\n",
    )
    .unwrap();
}

/// Bring the lock up to date after a legal change, the way a person would: `update` first, then a
/// locked build that has to accept what was just recorded.
/// 合法改动之后把锁更新到最新，像人那样：先 `update`，再让 `--locked` 接受刚记录的东西。
fn settled(app: &Path) {
    let updated = invoke(app, &["update"]);
    assert!(updated.status.success(), "{}", both(&updated));
    let accepted = invoke(app, &["build", "--locked"]);
    assert!(accepted.status.success(), "{}", both(&accepted));
}

/// Write the consumer project: one source, one test that may call the export.
/// 写消费者项目：一个源文件与一个测试（可选择是否调用导出）。
fn consumer(directory: &Path, path: &str, calls_export: bool) {
    std::fs::create_dir_all(directory.join("src")).unwrap();
    std::fs::create_dir_all(directory.join("tests")).unwrap();
    std::fs::write(
        directory.join("xirang.toml"),
        format!(
            "schema = 1\n\n[package]\nname = \"app\"\nversion = \"0.1.0\"\nkind = \"library\"\n\n\
             [language.cpp]\nstandard = \"17\"\n\n[dependencies]\nmath = {{ path = \"{path}\" }}\n"
        ),
    )
    .unwrap();
    std::fs::write(
        directory.join("src/app.cpp"),
        "#include <cstdint>\n\nstd::int64_t app_marker() {\n    return 1;\n}\n",
    )
    .unwrap();
    std::fs::write(
        directory.join("tests/add_test.cpp"),
        test_source(calls_export),
    )
    .unwrap();
}

/// The consumer's test body.
/// 消费者测试的正文。
fn test_source(calls_export: bool) -> &'static str {
    if calls_export {
        "#include <cassert>\n#include \"math/exports.h\"\n\nint main() {\n    assert(add(2, 3) == 5);\n    return 0;\n}\n"
    } else {
        "#include <cassert>\n\nint main() {\n    assert(1 + 1 == 2);\n    return 0;\n}\n"
    }
}

#[test]
fn removing_an_export_and_its_test_still_shows_up_under_locked() {
    let root = scratch("export");
    let math = root.join("math");
    math_library(&math, true, "");
    let app = root.join("app");
    consumer(&app, "../math", true);
    let updated = invoke(&app, &["update"]);
    assert!(updated.status.success(), "{}", both(&updated));
    // Two errors that cancel: the export is removed, and the test that called it is replaced by
    // one that cannot notice. The remaining test passes, so only the lock can tell.
    // 两错相抵：导出被删，调用它的测试也被换成一个察觉不到的实现。剩下的测试照样通过，只有锁能说出。
    math_library(&math, false, "");
    std::fs::write(app.join("tests/add_test.cpp"), test_source(false)).unwrap();
    // The guard asks the lock before anything rewrites it.
    // 守卫在别的东西改写锁之前先问锁。
    let refused = invoke(&app, &["build", "--locked"]);
    assert!(!refused.status.success(), "{}", both(&refused));
    let text = both(&refused);
    assert!(
        text.contains("export add("),
        "the lock has to show the export it recorded: {text}"
    );
    let passed = invoke(&app, &["test"]);
    assert!(passed.status.success(), "{}", both(&passed));
    settled(&app);
}

#[test]
fn deleting_a_test_refuses_under_locked_until_the_lock_updates() {
    let root = scratch("test");
    let math = root.join("math");
    math_library(&math, true, "");
    let app = root.join("app");
    consumer(&app, "../math", true);
    std::fs::write(
        app.join("tests/other_test.cpp"),
        "#include <cassert>\n\nint main() {\n    assert(2 + 2 == 4);\n    return 0;\n}\n",
    )
    .unwrap();
    let updated = invoke(&app, &["update"]);
    assert!(updated.status.success(), "{}", both(&updated));
    std::fs::remove_file(app.join("tests/other_test.cpp")).unwrap();
    let refused = invoke(&app, &["build", "--locked"]);
    assert!(!refused.status.success(), "{}", both(&refused));
    let text = both(&refused);
    assert!(text.contains("test tests/other_test.cpp"), "{text}");
    let passed = invoke(&app, &["test"]);
    assert!(passed.status.success(), "{}", both(&passed));
    settled(&app);
}

#[test]
fn moving_a_dependency_is_a_legal_change_that_leaves_its_bytes_alone() {
    let root = scratch("move");
    let math = root.join("math");
    math_library(&math, true, "");
    let app = root.join("app");
    consumer(&app, "../math", true);
    let before = std::fs::read(math.join("src/math.cpp")).unwrap();
    let manifest = std::fs::read(math.join("xirang.toml")).unwrap();
    let updated = invoke(&app, &["update"]);
    assert!(updated.status.success(), "{}", both(&updated));
    // The move: the dependency directory changes place and the root manifest learns the new path.
    // 搬迁：依赖目录换位置，根清单只改路径。
    let vendor = root.join("vendor");
    std::fs::create_dir_all(&vendor).unwrap();
    let moved = vendor.join("math");
    std::fs::rename(&math, &moved).unwrap();
    consumer(&app, "../vendor/math", true);
    // The lock still records the old path, so the guard refuses before anything rewrites it.
    // 锁仍记着旧路径，因此守卫在别的东西改写它之前先拒绝。
    let refused = invoke(&app, &["build", "--locked"]);
    assert!(!refused.status.success(), "{}", both(&refused));
    let text = both(&refused);
    assert!(text.contains("vendor/math"), "{text}");
    assert!(text.contains("different dependency graph"), "{text}");
    let passed = invoke(&app, &["test"]);
    assert!(passed.status.success(), "{}", both(&passed));
    assert_eq!(std::fs::read(moved.join("src/math.cpp")).unwrap(), before);
    assert_eq!(std::fs::read(moved.join("xirang.toml")).unwrap(), manifest);
    settled(&app);
}
