#![cfg(feature = "cli")]
//! End-to-end: a dependency that names a `[sources]` entry resolves inside that source, the lock
//! records where it came from, and a lookup that lands on a differently named package is refused
//! by name. Two sources hold a package with the same name on purpose: selecting by source is what
//! keeps them apart.
//! 端到端：点名 `[sources]` 的依赖在该源里解析，锁记录它来自哪里，落到不同名字的包上时按名拒绝。
//! 两个源故意各放一个同名包：按源选择才是把它们分开的那件事。

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
        "xirang-sources-{label}-{}-{:?}",
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

/// Write one library package at `directory`: a manifest and one C++ source.
/// 在 `directory` 写一个库包：一份清单与一个 C++ 源文件。
fn library(directory: &Path, name: &str, source: &str) {
    std::fs::create_dir_all(directory.join("src")).unwrap();
    std::fs::write(
        directory.join("xirang.toml"),
        format!(
            "schema = 1\n\n[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nkind = \"library\"\n\n\
             [language.cpp]\nstandard = \"17\"\n\n[dependencies]\n"
        ),
    )
    .unwrap();
    std::fs::write(directory.join("src").join(format!("{name}.cpp")), source).unwrap();
}

/// A source lookup picks the named source, and a sibling source with the same package name is not
/// what gets built.
/// 按源查找会选中点名的源；另一个同名包的兄弟源不会被构建。
#[test]
fn a_source_qualified_dependency_resolves_to_the_source_it_names() {
    let scratch = scratch("resolve");
    library(
        &scratch.join("sources/team/greeter"),
        "greeter",
        "int greeter_value() { return 7; }\n",
    );
    library(
        &scratch.join("sources/vendor/greeter"),
        "greeter",
        "int greeter_value() { return 9; }\n",
    );
    let consumer = scratch.join("consumer");
    library(
        &consumer,
        "consumer",
        "int consumer_marker() { return 1; }\n",
    );
    std::fs::create_dir_all(consumer.join("tests")).unwrap();
    std::fs::write(
        consumer.join("xirang.toml"),
        "schema = 1\n\n[package]\nname = \"consumer\"\nversion = \"0.1.0\"\nkind = \"library\"\n\n\
         [language.cpp]\nstandard = \"17\"\n\n[sources]\n\
         team = { path = \"../sources/team\" }\nvendor = { path = \"../sources/vendor\" }\n\n\
         [dependencies]\ngreeter = { source = \"team\", package = \"greeter\" }\n",
    )
    .unwrap();
    std::fs::write(
        consumer.join("tests/value_test.cpp"),
        "int greeter_value();\n\nint main() { return greeter_value() == 7 ? 0 : 1; }\n",
    )
    .unwrap();
    let built = invoke(&consumer, &["build"]);
    assert!(built.status.success(), "{}", both(&built));
    let tested = invoke(&consumer, &["test"]);
    assert!(tested.status.success(), "{}", both(&tested));

    // The sibling source keeps its own value: editing it changes nothing here.
    // 兄弟源保留自己的值：改它对这里没有影响。
    library(
        &scratch.join("sources/vendor/greeter"),
        "greeter",
        "int greeter_value() { return 42; }\n",
    );
    let tested = invoke(&consumer, &["test"]);
    assert!(tested.status.success(), "{}", both(&tested));

    // The selected source is the one that is compiled.
    // 被编译的是选中的那个源。
    library(
        &scratch.join("sources/team/greeter"),
        "greeter",
        "int greeter_value() { return 8; }\n",
    );
    let tested = invoke(&consumer, &["test"]);
    assert!(!tested.status.success(), "{}", both(&tested));
}

/// A source lookup that lands on a differently named package is refused, and the refusal names both.
/// 落到不同名字的包上的源查找会被拒绝，拒绝里同时点名两边。
#[test]
fn a_source_package_name_mismatch_is_refused_by_name() {
    let scratch = scratch("mismatch");
    library(
        &scratch.join("sources/team/other"),
        "different",
        "int other_value() { return 1; }\n",
    );
    let consumer = scratch.join("consumer");
    library(
        &consumer,
        "consumer",
        "int consumer_marker() { return 1; }\n",
    );
    std::fs::write(
        consumer.join("xirang.toml"),
        "schema = 1\n\n[package]\nname = \"consumer\"\nversion = \"0.1.0\"\nkind = \"library\"\n\n\
         [language.cpp]\nstandard = \"17\"\n\n[sources]\nteam = { path = \"../sources/team\" }\n\n\
         [dependencies]\nother = { source = \"team\", package = \"other\" }\n",
    )
    .unwrap();
    let built = invoke(&consumer, &["build"]);
    assert!(!built.status.success(), "{}", both(&built));
    let text = both(&built);
    assert!(
        text.contains("asks source `team` for package `other`"),
        "{text}"
    );
    assert!(text.contains("named `different`"), "{text}");
}

/// An undeclared source is refused with the names the root manifest does declare.
/// 未声明的源被拒绝，并列出根清单实际声明的名字。
#[test]
fn an_undeclared_source_is_refused_with_the_declared_names() {
    let scratch = scratch("undeclared");
    library(
        &scratch.join("sources/team/greeter"),
        "greeter",
        "int greeter_value() { return 7; }\n",
    );
    let consumer = scratch.join("consumer");
    library(
        &consumer,
        "consumer",
        "int consumer_marker() { return 1; }\n",
    );
    std::fs::write(
        consumer.join("xirang.toml"),
        "schema = 1\n\n[package]\nname = \"consumer\"\nversion = \"0.1.0\"\nkind = \"library\"\n\n\
         [language.cpp]\nstandard = \"17\"\n\n[sources]\nteam = { path = \"../sources/team\" }\n\n\
         [dependencies]\ngreeter = { source = \"vendor\", package = \"greeter\" }\n",
    )
    .unwrap();
    let built = invoke(&consumer, &["build"]);
    assert!(!built.status.success(), "{}", both(&built));
    let text = both(&built);
    assert!(text.contains("names source `vendor`"), "{text}");
    assert!(text.contains("it declares: team"), "{text}");
}

/// The lock records the source, and `--locked` refuses a change of source.
/// 锁记录源；`--locked` 在源改变时拒绝。
#[test]
fn the_lock_records_the_source_and_refuses_a_change_under_locked() {
    let scratch = scratch("lock");
    library(
        &scratch.join("sources/team/greeter"),
        "greeter",
        "int greeter_value() { return 7; }\n",
    );
    library(
        &scratch.join("sources/vendor/greeter"),
        "greeter",
        "int greeter_value() { return 9; }\n",
    );
    let consumer = scratch.join("consumer");
    library(
        &consumer,
        "consumer",
        "int consumer_marker() { return 1; }\n",
    );
    let manifest = |source: &str| {
        format!(
            "schema = 1\n\n[package]\nname = \"consumer\"\nversion = \"0.1.0\"\nkind = \"library\"\n\n\
             [language.cpp]\nstandard = \"17\"\n\n[sources]\n\
             team = {{ path = \"../sources/team\" }}\nvendor = {{ path = \"../sources/vendor\" }}\n\n\
             [dependencies]\ngreeter = {{ source = \"{source}\", package = \"greeter\" }}\n"
        )
    };
    std::fs::write(consumer.join("xirang.toml"), manifest("team")).unwrap();
    let built = invoke(&consumer, &["build"]);
    assert!(built.status.success(), "{}", both(&built));
    let lock = std::fs::read_to_string(consumer.join("xirang.lock")).unwrap();
    assert!(lock.contains("source = \"team\""), "{lock}");

    std::fs::write(consumer.join("xirang.toml"), manifest("vendor")).unwrap();
    let locked = invoke(&consumer, &["build", "--locked"]);
    assert!(!locked.status.success(), "{}", both(&locked));
    let text = both(&locked);
    // A unit's identity is (name, version, source, path): the source changed *and* the path did.
    // 单元身份是（名字、版本、来源、路径）：这里来源与路径都变了。
    assert!(text.contains("(source team, path"), "{text}");
    assert!(text.contains("(source vendor, path"), "{text}");
}

/// An override redirects one source package to a local directory, and the lock records which
/// override produced the unit.
/// 覆盖把具名源里的一个包改指到本地目录，锁记录是哪个覆盖产生了这个单元。
#[test]
fn an_override_redirects_one_package_and_the_lock_records_it() {
    let scratch = scratch("override");
    library(
        &scratch.join("sources/team/greeter"),
        "greeter",
        "int greeter_value() { return 7; }\n",
    );
    library(
        &scratch.join("pinned/greeter"),
        "greeter",
        "int greeter_value() { return 5; }\n",
    );
    let consumer = scratch.join("consumer");
    library(
        &consumer,
        "consumer",
        "int consumer_marker() { return 1; }\n",
    );
    std::fs::create_dir_all(consumer.join("tests")).unwrap();
    std::fs::write(
        consumer.join("xirang.toml"),
        "schema = 1\n\n[package]\nname = \"consumer\"\nversion = \"0.1.0\"\nkind = \"library\"\n\n\
         [language.cpp]\nstandard = \"17\"\n\n[sources]\nteam = { path = \"../sources/team\" }\n\n\
         [overrides]\nteam/greeter = { path = \"../pinned/greeter\" }\n\n\
         [dependencies]\ngreeter = { source = \"team\", package = \"greeter\" }\n",
    )
    .unwrap();
    std::fs::write(
        consumer.join("tests/value_test.cpp"),
        "int greeter_value();\n\nint main() { return greeter_value() == 5 ? 0 : 1; }\n",
    )
    .unwrap();
    let tested = invoke(&consumer, &["test"]);
    assert!(tested.status.success(), "{}", both(&tested));
    let lock = std::fs::read_to_string(consumer.join("xirang.lock")).unwrap();
    assert!(
        lock.contains("source = \"override:team/greeter\""),
        "{lock}"
    );

    // The overridden source no longer feeds the build; the override does.
    // 被覆盖的源不再喂给构建；覆盖目标才喂。
    library(
        &scratch.join("sources/team/greeter"),
        "greeter",
        "int greeter_value() { return 9; }\n",
    );
    let tested = invoke(&consumer, &["test"]);
    assert!(tested.status.success(), "{}", both(&tested));
    library(
        &scratch.join("pinned/greeter"),
        "greeter",
        "int greeter_value() { return 6; }\n",
    );
    let failed = invoke(&consumer, &["test"]);
    assert!(!failed.status.success(), "{}", both(&failed));
}

/// An override that no dependency reaches is refused by name: a silent no-op is a typo that keeps
/// looking like a pin.
/// 没有任何依赖走到的覆盖按名拒绝：静默无效的覆盖就是一条看起来像钉子的拼写错误。
#[test]
fn an_override_that_names_no_dependency_is_refused() {
    let scratch = scratch("override-unused");
    library(
        &scratch.join("sources/team/greeter"),
        "greeter",
        "int greeter_value() { return 7; }\n",
    );
    library(
        &scratch.join("pinned/other"),
        "other",
        "int other_value() { return 1; }\n",
    );
    let consumer = scratch.join("consumer");
    library(
        &consumer,
        "consumer",
        "int consumer_marker() { return 1; }\n",
    );
    std::fs::write(
        consumer.join("xirang.toml"),
        "schema = 1\n\n[package]\nname = \"consumer\"\nversion = \"0.1.0\"\nkind = \"library\"\n\n\
         [language.cpp]\nstandard = \"17\"\n\n[sources]\nteam = { path = \"../sources/team\" }\n\n\
         [overrides]\nteam/other = { path = \"../pinned/other\" }\n\n\
         [dependencies]\ngreeter = { source = \"team\", package = \"greeter\" }\n",
    )
    .unwrap();
    let built = invoke(&consumer, &["build"]);
    assert!(!built.status.success(), "{}", both(&built));
    let text = both(&built);
    assert!(
        text.contains("override `team/other` matches no dependency"),
        "{text}"
    );
}

/// An override that lands on a differently named package is refused by name.
/// 落到不同名字的包上的覆盖按名拒绝。
#[test]
fn an_override_that_lands_on_a_different_package_is_refused() {
    let scratch = scratch("override-mismatch");
    library(
        &scratch.join("sources/team/greeter"),
        "greeter",
        "int greeter_value() { return 7; }\n",
    );
    library(
        &scratch.join("pinned/other"),
        "different",
        "int other_value() { return 1; }\n",
    );
    let consumer = scratch.join("consumer");
    library(
        &consumer,
        "consumer",
        "int consumer_marker() { return 1; }\n",
    );
    std::fs::write(
        consumer.join("xirang.toml"),
        "schema = 1\n\n[package]\nname = \"consumer\"\nversion = \"0.1.0\"\nkind = \"library\"\n\n\
         [language.cpp]\nstandard = \"17\"\n\n[sources]\nteam = { path = \"../sources/team\" }\n\n\
         [overrides]\nteam/greeter = { path = \"../pinned/other\" }\n\n\
         [dependencies]\ngreeter = { source = \"team\", package = \"greeter\" }\n",
    )
    .unwrap();
    let built = invoke(&consumer, &["build"]);
    assert!(!built.status.success(), "{}", both(&built));
    let text = both(&built);
    assert!(
        text.contains("overrides `team/greeter` for package `greeter`"),
        "{text}"
    );
    assert!(text.contains("named `different`"), "{text}");
}

/// The host project used by the registry tests: a C++ library that calls `greeter_value` in its
/// test, depending on a package looked up in the `team` registry.
/// 注册表测试用的宿主项目：一个 C++ 库，在测试里调用 `greeter_value`，依赖从 `team` 注册表查找的包。
fn registry_host(scratch: &Path) {
    std::fs::create_dir_all(scratch.join("host/src")).unwrap();
    std::fs::create_dir_all(scratch.join("host/tests")).unwrap();
    std::fs::write(
        scratch.join("host/xirang.toml"),
        "schema = 1\n\n[package]\nname = \"host\"\nversion = \"0.1.0\"\nkind = \"library\"\n\n\
         [language.cpp]\nstandard = \"17\"\n\n[sources]\nteam = { registry = \"../registry\" }\n\n\
         [dependencies]\ngreeter = { source = \"team\", package = \"greeter\" }\n",
    )
    .unwrap();
    std::fs::write(
        scratch.join("host/src/host.cpp"),
        "int host_marker() { return 0; }\n",
    )
    .unwrap();
    std::fs::write(
        scratch.join("host/tests/uses_test.cpp"),
        "#include <cassert>\nint greeter_value();\nint main() { assert(greeter_value() == 10); return 0; }\n",
    )
    .unwrap();
}

#[test]
fn a_registry_source_picks_the_highest_version_and_records_it_in_the_lock() {
    let scratch = scratch("registry-highest");
    library(
        &scratch.join("registry/greeter/0.1.0"),
        "greeter",
        "int greeter_value() { return 1; }\n",
    );
    library(
        &scratch.join("registry/greeter/0.10.0"),
        "greeter",
        "int greeter_value() { return 10; }\n",
    );
    registry_host(&scratch);
    let host = scratch.join("host");
    let tested = invoke(&host, &["test"]);
    assert!(tested.status.success(), "{}", both(&tested));
    let updated = invoke(&host, &["update"]);
    assert!(updated.status.success(), "{}", both(&updated));
    let lock = std::fs::read_to_string(host.join("xirang.lock")).unwrap();
    assert!(lock.contains("team@0.10.0"), "{lock}");
}

#[test]
fn a_registry_that_lacks_the_package_is_refused_by_name() {
    let scratch = scratch("registry-missing");
    library(
        &scratch.join("registry/greeter/0.1.0"),
        "greeter",
        "int greeter_value() { return 1; }\n",
    );
    registry_host(&scratch);
    let manifest = std::fs::read_to_string(scratch.join("host/xirang.toml")).unwrap();
    std::fs::write(
        scratch.join("host/xirang.toml"),
        manifest.replace("package = \"greeter\"", "package = \"other\""),
    )
    .unwrap();
    let built = invoke(&scratch.join("host"), &["build"]);
    assert!(!built.status.success(), "{}", both(&built));
    let text = both(&built);
    assert!(text.contains("package `other`"), "{text}");
    assert!(text.contains("it holds: greeter"), "{text}");
}

#[test]
fn the_lock_pins_the_registry_version_until_update_moves_it() {
    let scratch = scratch("registry-pin");
    library(
        &scratch.join("registry/greeter/0.1.0"),
        "greeter",
        "int greeter_value() { return 1; }\n",
    );
    registry_host(&scratch);
    let host = scratch.join("host");
    let updated = invoke(&host, &["update"]);
    assert!(updated.status.success(), "{}", both(&updated));
    let lock = std::fs::read_to_string(host.join("xirang.lock")).unwrap();
    assert!(lock.contains("team@0.1.0"), "{lock}");
    library(
        &scratch.join("registry/greeter/0.2.0"),
        "greeter",
        "int greeter_value() { return 2; }\n",
    );
    let locked = invoke(&host, &["build", "--locked"]);
    assert!(!locked.status.success(), "{}", both(&locked));
    assert!(both(&locked).contains("0.2.0"), "{}", both(&locked));
    let updated = invoke(&host, &["update"]);
    assert!(updated.status.success(), "{}", both(&updated));
    let lock = std::fs::read_to_string(host.join("xirang.lock")).unwrap();
    assert!(lock.contains("team@0.2.0"), "{lock}");
    let locked = invoke(&host, &["build", "--locked"]);
    assert!(locked.status.success(), "{}", both(&locked));
}
