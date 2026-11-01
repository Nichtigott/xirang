#![cfg(feature = "cli")]
//! Cross-unit consumer obligations: a producer's export surface is part of the consumer's lock,
//! so changing it makes a locked build refuse until the lock is updated — and the consumer is
//! really rebuilt, so it has to adapt to the new signature instead of passing on stale bytes.
//! 跨单元消费者义务：生产者的导出面是消费者锁的一部分，改它会让 `--locked` 构建先拒绝、直到更新锁；
//! 而消费者是真的被重建，所以必须适配新签名，不能靠旧字节通过。

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn binary() -> &'static str {
    env!("CARGO_BIN_EXE_xirang")
}

fn scratch(label: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "xirang-obligations-{label}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&path).unwrap();
    path
}

fn invoke(directory: &Path, arguments: &[&str]) -> Output {
    Command::new(binary())
        .args(arguments)
        .current_dir(directory)
        .output()
        .unwrap()
}

fn both(output: &Output) -> String {
    format!(
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

/// The producer: `add` with a two-argument or a one-argument signature.
fn producer(directory: &Path, one_argument: bool) {
    std::fs::create_dir_all(directory.join("src")).unwrap();
    let (params, source) = if one_argument {
        (
            "params = [\"i64\"]",
            "#include <cstdint>\n\nstd::int64_t add(std::int64_t a) {\n    return a + 3;\n}\n",
        )
    } else {
        (
            "params = [\"i64\", \"i64\"]",
            "#include <cstdint>\n\nstd::int64_t add(std::int64_t a, std::int64_t b) {\n    return a + b;\n}\n",
        )
    };
    std::fs::write(
        directory.join("xirang.toml"),
        format!(
            "schema = 1\n\n[package]\nname = \"math\"\nversion = \"0.1.0\"\nkind = \"library\"\n\n\
             [language.cpp]\nstandard = \"17\"\n\n[[export]]\nname = \"add\"\n{params}\nreturns = \"i64\"\n\n[dependencies]\n"
        ),
    )
    .unwrap();
    std::fs::write(directory.join("src/math.cpp"), source).unwrap();
}

fn consumer(directory: &Path, argument_count: usize) {
    std::fs::create_dir_all(directory.join("src")).unwrap();
    std::fs::create_dir_all(directory.join("tests")).unwrap();
    std::fs::write(
        directory.join("xirang.toml"),
        "schema = 1\n\n[package]\nname = \"app\"\nversion = \"0.1.0\"\nkind = \"library\"\n\n\
         [language.cpp]\nstandard = \"17\"\n\n[dependencies]\nmath = { path = \"../math\" }\n",
    )
    .unwrap();
    std::fs::write(
        directory.join("src/lib.cpp"),
        "#include <cstdint>\n\nstd::int64_t marker() {\n    return 1;\n}\n",
    )
    .unwrap();
    let call = if argument_count == 2 {
        "add(2, 3) == 5"
    } else {
        "add(2) == 5"
    };
    std::fs::write(
        directory.join("tests/use_test.cpp"),
        format!(
            "#include <cassert>\n#include \"math/exports.h\"\n\nint main() {{\n    assert({call});\n    return 0;\n}}\n"
        ),
    )
    .unwrap();
}

#[test]
fn a_producers_export_change_forces_an_unlock_and_a_real_adaptation() {
    let root = scratch("export-change");
    producer(&root.join("math"), false);
    consumer(&root.join("app"), 2);
    let app = root.join("app");

    let first = invoke(&app, &["test"]);
    assert!(first.status.success(), "{}", both(&first));
    let updated = invoke(&app, &["update"]);
    assert!(updated.status.success(), "{}", both(&updated));
    let accepted = invoke(&app, &["build", "--locked"]);
    assert!(accepted.status.success(), "{}", both(&accepted));

    producer(&root.join("math"), true);
    let refused = invoke(&app, &["build", "--locked"]);
    assert!(
        !refused.status.success(),
        "the lock accepted a new export surface"
    );
    let refusal = both(&refused);
    assert!(
        refusal.contains("export add("),
        "the refusal did not name the export surface: {refusal}"
    );

    // The consumer is really rebuilt: the old two-argument call cannot pass against the new
    // one-argument export, even though the lock is stale and the plan id changed.
    let unadapted = invoke(&app, &["test"]);
    assert!(
        !unadapted.status.success(),
        "the consumer passed without adapting: {}",
        both(&unadapted)
    );

    consumer(&root.join("app"), 1);
    let adapted = invoke(&app, &["test"]);
    assert!(adapted.status.success(), "{}", both(&adapted));
    let updated = invoke(&app, &["update"]);
    assert!(updated.status.success(), "{}", both(&updated));
    let accepted = invoke(&app, &["build", "--locked"]);
    assert!(accepted.status.success(), "{}", both(&accepted));
}
