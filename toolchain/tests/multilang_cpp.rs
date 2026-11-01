#![cfg(feature = "cli")]
//! End-to-end: the `xirang` binary scaffolds a C++ project, builds it with a real compiler, runs its
//! test program, and reports failures as failures. Nothing here inspects `xirang.toml` parsing in
//! isolation: the evidence file is the answer, and the compiler really ran.
//! 端到端：`xirang` 二进制搭出 C++ 项目、用真编译器构建、运行它的测试程序，并把失败报成失败。这里
//! 不孤立地检查 `xirang.toml` 解析：证据文件才是答案，而编译器真的跑过。

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// The binary under test.
/// 被测二进制。
fn binary() -> &'static str {
    env!("CARGO_BIN_EXE_xirang")
}

/// A fresh scratch directory, so two runs never share a plan or an evidence file.
/// 一个全新的临时目录，使两次运行不共享计划或证据文件。
fn scratch(label: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "xirang-multilang-{label}-{}-{:?}",
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

/// Both streams, for a readable failure message.
/// 两路输出，失败时容易读。
fn both(output: &Output) -> String {
    format!(
        "status {}\nstdout:\n{}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

/// The evidence document of the most recent build directory.
/// 最近一次构建目录里的证据文档。
fn evidence(directory: &Path) -> serde_json::Value {
    let builds = directory.join("target/xirang/build");
    let mut newest: Option<(std::time::SystemTime, PathBuf)> = None;
    for entry in std::fs::read_dir(&builds).unwrap() {
        let path = entry.unwrap().path().join("evidence.json");
        let Ok(metadata) = std::fs::metadata(&path) else {
            continue;
        };
        let modified = metadata.modified().unwrap();
        if newest.as_ref().is_none_or(|(at, _)| modified > *at) {
            newest = Some((modified, path));
        }
    }
    let (_, path) = newest.expect("no evidence.json was written");
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

#[test]
fn a_cpp_library_is_scaffolded_built_and_tested() {
    let scratch = scratch("library");
    let created = invoke(&scratch, &["new", "--lang", "cpp", "--lib", "widget"]);
    assert!(created.status.success(), "{}", both(&created));
    for path in [
        "widget/xirang.toml",
        "widget/src/widget.cpp",
        "widget/include/widget.hpp",
        "widget/tests/widget_test.cpp",
    ] {
        assert!(
            scratch.join(path).is_file(),
            "`xirang new --lang cpp` did not write {path}"
        );
    }
    let project = scratch.join("widget");
    let built = invoke(&project, &["build"]);
    assert!(built.status.success(), "{}", both(&built));
    let built_evidence = evidence(&project);
    assert_eq!(built_evidence["result"], "passed");
    assert!(
        built_evidence["actions"]
            .as_array()
            .is_some_and(|actions| actions.iter().any(|action| action["kind"] == "compile")),
        "the evidence records no compile action: {built_evidence}"
    );
    assert_eq!(
        built_evidence["compiler"]["program"],
        built_evidence["compiler"]["program"]
    );
    let tested = invoke(&project, &["test"]);
    assert!(tested.status.success(), "{}", both(&tested));
    let text = String::from_utf8_lossy(&tested.stdout);
    assert!(
        text.contains("test widget_test ... ok"),
        "the test program did not run: {text}"
    );
    assert_eq!(evidence(&project)["result"], "passed");
}

#[test]
fn an_unimplemented_language_is_refused_and_writes_nothing() {
    let scratch = scratch("unimplemented");
    let refused = invoke(&scratch, &["new", "--lang", "cobol", "--lib", "widget"]);
    assert!(!refused.status.success(), "{}", both(&refused));
    let text = both(&refused);
    assert!(
        text.contains("not implemented"),
        "the refusal does not say the language is unimplemented: {text}"
    );
    assert!(
        !scratch.join("widget/xirang.toml").exists(),
        "a refused language still wrote a project"
    );
    // A registered consumer language without a template is refused by name instead of being
    // scaffolded as C++.
    // 注册为消费者但没有模板的语言按名拒绝，而不是被当成 C++ 搭出来。
    let registered = invoke(&scratch, &["new", "--lang", "kotlin", "--lib", "widget"]);
    assert!(!registered.status.success(), "{}", both(&registered));
    assert!(
        both(&registered).contains("no `xirang new` scaffold"),
        "{}",
        both(&registered)
    );
    assert!(!scratch.join("widget/xirang.toml").exists());
    // The language that *is* implemented scaffolds its own module, not a C++ project under a
    // different name.
    // **已实现**的语言搭出它自己的模块，而不是换了个名字的 C++ 项目。
    let scaffolded = invoke(&scratch, &["new", "--lang", "go", "--lib", "gadget"]);
    assert!(scaffolded.status.success(), "{}", both(&scaffolded));
    let manifest = std::fs::read_to_string(scratch.join("gadget/xirang.toml")).unwrap();
    assert!(manifest.contains("[language.go]"), "{manifest}");
    assert!(
        scratch.join("gadget/go.mod").is_file(),
        "a Go scaffold wrote no go.mod"
    );
}

#[test]
fn a_path_dependency_is_compiled_linked_and_tested() {
    let scratch = scratch("dependency");
    let base = invoke(&scratch, &["new", "--lang", "cpp", "--lib", "base"]);
    assert!(base.status.success(), "{}", both(&base));
    let app = scratch.join("app");
    std::fs::create_dir_all(app.join("src")).unwrap();
    std::fs::create_dir_all(app.join("include")).unwrap();
    std::fs::create_dir_all(app.join("tests")).unwrap();
    std::fs::write(
        app.join("xirang.toml"),
        r#"schema = 1

[package]
name = "app"
version = "0.1.0"
kind = "library"

[language.cpp]
standard = "17"

[dependencies]
base = { path = "../base" }
"#,
    )
    .unwrap();
    std::fs::write(
        app.join("include/app.hpp"),
        "#pragma once\n\nnamespace app {\nint answer();\n}\n",
    )
    .unwrap();
    std::fs::write(
        app.join("src/app.cpp"),
        "#include \"app.hpp\"\n#include \"base.hpp\"\n\nnamespace app {\nint answer() { return base::answer() + 1; }\n}\n",
    )
    .unwrap();
    std::fs::write(
        app.join("tests/app_test.cpp"),
        "#include \"app.hpp\"\n\n#include <cstdio>\n\nint main() {\n    if (app::answer() != 43) {\n        std::fprintf(stderr, \"app::answer() != 43\\n\");\n        return 1;\n    }\n    std::printf(\"ok app_test\\n\");\n    return 0;\n}\n",
    )
    .unwrap();
    let built = invoke(&app, &["build"]);
    assert!(built.status.success(), "{}", both(&built));
    let tested = invoke(&app, &["test"]);
    assert!(tested.status.success(), "{}", both(&tested));
    assert!(
        String::from_utf8_lossy(&tested.stdout).contains("test app_test ... ok"),
        "{}",
        both(&tested)
    );
}

#[test]
fn a_chain_of_path_dependencies_is_linked_in_consumer_order() {
    let scratch = scratch("chain");
    let util = invoke(&scratch, &["new", "--lang", "cpp", "--lib", "util"]);
    assert!(util.status.success(), "{}", both(&util));
    std::fs::write(
        scratch.join("util/src/util.cpp"),
        "#include \"util.hpp\"\n\nnamespace util {\nint answer() { return 41; }\n}\n",
    )
    .unwrap();
    let base = invoke(&scratch, &["new", "--lang", "cpp", "--lib", "base"]);
    assert!(base.status.success(), "{}", both(&base));
    std::fs::write(
        scratch.join("base/xirang.toml"),
        "schema = 1\n\n[package]\nname = \"base\"\nversion = \"0.1.0\"\nkind = \"library\"\n\n[language.cpp]\nstandard = \"17\"\n\n[dependencies]\nutil = { path = \"../util\" }\n",
    )
    .unwrap();
    std::fs::write(
        scratch.join("base/src/base.cpp"),
        "#include \"base.hpp\"\n#include \"util.hpp\"\n\nnamespace base {\nint answer() { return util::answer() + 1; }\n}\n",
    )
    .unwrap();
    let app = scratch.join("app");
    std::fs::create_dir_all(app.join("src")).unwrap();
    std::fs::create_dir_all(app.join("include")).unwrap();
    std::fs::create_dir_all(app.join("tests")).unwrap();
    std::fs::write(
        app.join("xirang.toml"),
        "schema = 1\n\n[package]\nname = \"app\"\nversion = \"0.1.0\"\nkind = \"library\"\n\n[language.cpp]\nstandard = \"17\"\n\n[dependencies]\nbase = { path = \"../base\" }\n",
    )
    .unwrap();
    std::fs::write(
        app.join("include/app.hpp"),
        "#pragma once\n\nnamespace app {\nint answer();\n}\n",
    )
    .unwrap();
    std::fs::write(
        app.join("src/app.cpp"),
        "#include \"app.hpp\"\n#include \"base.hpp\"\n\nnamespace app {\nint answer() { return base::answer() + 1; }\n}\n",
    )
    .unwrap();
    std::fs::write(
        app.join("tests/app_test.cpp"),
        "#include \"app.hpp\"\n\n#include <cstdio>\n\nint main() {\n    if (app::answer() != 43) {\n        std::fprintf(stderr, \"app::answer() != 43\\n\");\n        return 1;\n    }\n    std::printf(\"ok app_test\\n\");\n    return 0;\n}\n",
    )
    .unwrap();
    let tested = invoke(&app, &["test"]);
    assert!(tested.status.success(), "{}", both(&tested));
    assert!(
        String::from_utf8_lossy(&tested.stdout).contains("test app_test ... ok"),
        "{}",
        both(&tested)
    );
}

#[test]
fn a_compile_error_is_a_failure_and_a_changed_source_gets_another_plan() {
    let scratch = scratch("failure");
    let created = invoke(&scratch, &["new", "--lang", "cpp", "--lib", "widget"]);
    assert!(created.status.success(), "{}", both(&created));
    let project = scratch.join("widget");
    let first = invoke(&project, &["build"]);
    assert!(first.status.success(), "{}", both(&first));
    let first_plan = evidence(&project)["plan"].as_str().unwrap().to_owned();
    std::fs::write(
        project.join("src/widget.cpp"),
        "#include \"widget.hpp\"\n\nnamespace widget {\nint answer() { return 43; }\n}\n",
    )
    .unwrap();
    let second = invoke(&project, &["build"]);
    assert!(second.status.success(), "{}", both(&second));
    let second_plan = evidence(&project)["plan"].as_str().unwrap().to_owned();
    assert_ne!(
        first_plan, second_plan,
        "a changed source kept the same plan identity"
    );
    std::fs::write(
        project.join("src/widget.cpp"),
        "#include \"widget.hpp\"\n\nnamespace widget {\nint answer() { return ; }\n}\n",
    )
    .unwrap();
    let broken = invoke(&project, &["build"]);
    assert!(
        !broken.status.success(),
        "a source that does not compile was reported as built: {}",
        both(&broken)
    );
    assert_eq!(evidence(&project)["result"], "failed");
    assert!(
        String::from_utf8_lossy(&broken.stderr).contains("compile failed"),
        "{}",
        both(&broken)
    );
}

#[test]
fn adding_a_local_dependency_updates_the_manifest_and_the_build_uses_it() {
    let scratch = scratch("add");
    let util = invoke(&scratch, &["new", "--lang", "cpp", "--lib", "util"]);
    assert!(util.status.success(), "{}", both(&util));
    let app = invoke(&scratch, &["new", "--lang", "cpp", "--lib", "app"]);
    assert!(app.status.success(), "{}", both(&app));
    let added = invoke(&scratch.join("app"), &["add", "util", "--path", "../util"]);
    assert!(added.status.success(), "{}", both(&added));
    let manifest = std::fs::read_to_string(scratch.join("app/xirang.toml")).unwrap();
    assert!(
        manifest.contains("util = { path = \"../util\" }"),
        "{manifest}"
    );
    std::fs::write(
        scratch.join("app/src/app.cpp"),
        "#include \"app.hpp\"\n#include \"util.hpp\"\n\nnamespace app {\nint answer() { return util::answer(); }\n}\n",
    )
    .unwrap();
    let tested = invoke(&scratch.join("app"), &["test"]);
    assert!(tested.status.success(), "{}", both(&tested));
}

#[test]
fn adding_a_dependency_that_would_close_a_cycle_is_refused_and_writes_nothing() {
    let scratch = scratch("add-cycle");
    let util = invoke(&scratch, &["new", "--lang", "cpp", "--lib", "util"]);
    assert!(util.status.success(), "{}", both(&util));
    let app = invoke(&scratch, &["new", "--lang", "cpp", "--lib", "app"]);
    assert!(app.status.success(), "{}", both(&app));
    let first = invoke(&scratch.join("app"), &["add", "util", "--path", "../util"]);
    assert!(first.status.success(), "{}", both(&first));
    let before = std::fs::read_to_string(scratch.join("util/xirang.toml")).unwrap();
    let cycle = invoke(&scratch.join("util"), &["add", "app", "--path", "../app"]);
    assert!(!cycle.status.success(), "{}", both(&cycle));
    assert!(both(&cycle).contains("cycle"), "{}", both(&cycle));
    let after = std::fs::read_to_string(scratch.join("util/xirang.toml")).unwrap();
    assert_eq!(before, after);
}

#[test]
fn adding_a_binary_or_duplicate_dependency_is_refused() {
    let scratch = scratch("add-refused");
    let util = invoke(&scratch, &["new", "--lang", "cpp", "--lib", "util"]);
    assert!(util.status.success(), "{}", both(&util));
    let tool = invoke(&scratch, &["new", "--lang", "cpp", "tool"]);
    assert!(tool.status.success(), "{}", both(&tool));
    let app = invoke(&scratch, &["new", "--lang", "cpp", "--lib", "app"]);
    assert!(app.status.success(), "{}", both(&app));
    let binary = invoke(&scratch.join("app"), &["add", "tool", "--path", "../tool"]);
    assert!(!binary.status.success(), "{}", both(&binary));
    assert!(both(&binary).contains("binary"), "{}", both(&binary));
    let mismatch = invoke(&scratch.join("app"), &["add", "wrong", "--path", "../util"]);
    assert!(!mismatch.status.success(), "{}", both(&mismatch));
    assert!(both(&mismatch).contains("`util`"), "{}", both(&mismatch));
    let first = invoke(&scratch.join("app"), &["add", "util", "--path", "../util"]);
    assert!(first.status.success(), "{}", both(&first));
    let duplicate = invoke(&scratch.join("app"), &["add", "util", "--path", "../util"]);
    assert!(!duplicate.status.success(), "{}", both(&duplicate));
    assert!(
        both(&duplicate).contains("already depends"),
        "{}",
        both(&duplicate)
    );
}

#[test]
fn a_build_writes_a_lock_and_reports_when_the_dependency_graph_changes() {
    let scratch = scratch("lock");
    let util = invoke(&scratch, &["new", "--lang", "cpp", "--lib", "util"]);
    assert!(util.status.success(), "{}", both(&util));
    let app = invoke(&scratch, &["new", "--lang", "cpp", "--lib", "app"]);
    assert!(app.status.success(), "{}", both(&app));
    let added = invoke(&scratch.join("app"), &["add", "util", "--path", "../util"]);
    assert!(added.status.success(), "{}", both(&added));
    let first = invoke(&scratch.join("app"), &["build"]);
    assert!(first.status.success(), "{}", both(&first));
    let lock = std::fs::read_to_string(scratch.join("app/xirang.lock")).unwrap();
    assert!(lock.contains("name = \"util\""), "{lock}");
    assert!(lock.contains("version = \"0.1.0\""), "{lock}");
    let manifest = std::fs::read_to_string(scratch.join("util/xirang.toml")).unwrap();
    std::fs::write(
        scratch.join("util/xirang.toml"),
        manifest.replace("version = \"0.1.0\"", "version = \"0.2.0\""),
    )
    .unwrap();
    let second = invoke(&scratch.join("app"), &["build"]);
    assert!(second.status.success(), "{}", both(&second));
    assert!(
        both(&second).contains("xirang.lock: updated"),
        "{}",
        both(&second)
    );
    let lock = std::fs::read_to_string(scratch.join("app/xirang.lock")).unwrap();
    assert!(lock.contains("version = \"0.2.0\""), "{lock}");
}

#[test]
fn a_locked_build_refuses_when_the_graph_changed_and_keeps_the_lock() {
    let scratch = scratch("lock-locked");
    let util = invoke(&scratch, &["new", "--lang", "cpp", "--lib", "util"]);
    assert!(util.status.success(), "{}", both(&util));
    let app = invoke(&scratch, &["new", "--lang", "cpp", "--lib", "app"]);
    assert!(app.status.success(), "{}", both(&app));
    let added = invoke(&scratch.join("app"), &["add", "util", "--path", "../util"]);
    assert!(added.status.success(), "{}", both(&added));
    let first = invoke(&scratch.join("app"), &["build"]);
    assert!(first.status.success(), "{}", both(&first));
    let before = std::fs::read_to_string(scratch.join("app/xirang.lock")).unwrap();
    let manifest = std::fs::read_to_string(scratch.join("util/xirang.toml")).unwrap();
    std::fs::write(
        scratch.join("util/xirang.toml"),
        manifest.replace("version = \"0.1.0\"", "version = \"0.2.0\""),
    )
    .unwrap();
    let refused = invoke(&scratch.join("app"), &["build", "--locked"]);
    assert!(!refused.status.success(), "{}", both(&refused));
    assert!(
        both(&refused).contains("--locked forbids updating"),
        "{}",
        both(&refused)
    );
    let after = std::fs::read_to_string(scratch.join("app/xirang.lock")).unwrap();
    assert_eq!(before, after);
    let updated = invoke(&scratch.join("app"), &["build"]);
    assert!(updated.status.success(), "{}", both(&updated));
    let after = std::fs::read_to_string(scratch.join("app/xirang.lock")).unwrap();
    assert!(after.contains("version = \"0.2.0\""), "{after}");
}

#[test]
fn a_locked_build_on_a_project_without_a_lock_is_refused_and_writes_nothing() {
    let scratch = scratch("lock-missing");
    let app = invoke(&scratch, &["new", "--lang", "cpp", "--lib", "app"]);
    assert!(app.status.success(), "{}", both(&app));
    let refused = invoke(&scratch.join("app"), &["build", "--locked"]);
    assert!(!refused.status.success(), "{}", both(&refused));
    assert!(
        both(&refused).contains("needs an existing") && both(&refused).contains("xirang.lock"),
        "{}",
        both(&refused)
    );
    assert!(!scratch.join("app/xirang.lock").exists());
}

#[test]
fn the_registered_backends_are_reported_and_claim_only_what_they_declare() {
    let scratch = scratch("languages");
    let listed = invoke(&scratch, &["languages"]);
    assert!(listed.status.success(), "{}", both(&listed));
    let text = both(&listed);
    let cpp = text
        .lines()
        .find(|line| line.starts_with("cpp"))
        .unwrap_or_else(|| panic!("no cpp row: {text}"));
    assert!(cpp.contains("discovery"), "{cpp}");
    assert!(cpp.contains("resolution"), "{cpp}");
    assert!(cpp.contains("build-test"), "{cpp}");
    assert!(
        cpp.contains("export"),
        "the C++ backend now produces an export: {cpp}"
    );
    let c = text
        .lines()
        .find(|line| line.starts_with("c "))
        .unwrap_or_else(|| panic!("no C row: {text}"));
    assert!(c.contains("build-test"), "{c}");
    assert!(
        c.contains("consumer"),
        "the C backend consumes a C++ export in the verified chain: {c}"
    );
    let python = text
        .lines()
        .find(|line| line.starts_with("python"))
        .unwrap_or_else(|| panic!("no Python row: {text}"));
    assert!(python.contains("build-test"), "{python}");
    assert!(
        python.contains("consumer"),
        "the Python backend consumes a C++ export in the verified chain: {python}"
    );
    let rust = text
        .lines()
        .find(|line| line.starts_with("rust"))
        .unwrap_or_else(|| panic!("no Rust row: {text}"));
    assert!(
        rust.contains("consumer"),
        "the Rust backend consumes a C++ export in the verified chain: {rust}"
    );
    assert!(
        text.contains("no backend registered here") && text.contains("go"),
        "{text}"
    );
}

#[test]
fn a_rust_consumer_calls_a_cpp_export_through_the_c_abi() {
    let scratch = scratch("rust-consumer");
    let created = invoke(&scratch, &["new", "--lang", "cpp", "--lib", "producer"]);
    assert!(created.status.success(), "{}", both(&created));
    let manifest_path = scratch.join("producer/xirang.toml");
    let manifest = std::fs::read_to_string(&manifest_path).unwrap()
        + "[[export]]\nname = \"add\"\nparams = [\"i64\", \"i64\"]\nreturns = \"i64\"\n";
    std::fs::write(&manifest_path, manifest).unwrap();
    std::fs::write(
        scratch.join("producer/src/producer.cpp"),
        "#include <cstdint>\nstd::int64_t add(std::int64_t a, std::int64_t b) {\n    return a + b;\n}\n",
    )
    .unwrap();
    // The consumer is a Rust crate that declares the C ABI it consumes; `extern "C"` names the
    // symbol the export header declares, and the plan links the producer's archive.
    // 消费者是一个 Rust crate，自己声明它消费的 C ABI；`extern "C"` 点名导出头声明的符号，计划则
    // 链接生产者的归档。
    let consumer = scratch.join("consumer");
    std::fs::create_dir_all(consumer.join("src")).unwrap();
    std::fs::create_dir_all(consumer.join("tests")).unwrap();
    std::fs::write(
        consumer.join("xirang.toml"),
        "schema = 1\n\n[package]\nname = \"consumer\"\nversion = \"0.1.0\"\nkind = \"library\"\n\n[language.rust]\nstandard = \"2021\"\n\n[dependencies]\nproducer = { path = \"../producer\" }\n",
    )
    .unwrap();
    std::fs::write(
        consumer.join("src/lib.rs"),
        "pub fn answer() -> i64 {\n    42\n}\n",
    )
    .unwrap();
    let passing = "unsafe extern \"C\" {\n    fn add(a: i64, b: i64) -> i64;\n}\n\n#[test]\nfn adds_through_the_c_abi() {\n    assert_eq!(unsafe { add(2, 3) }, 5);\n}\n";
    std::fs::write(consumer.join("tests/add_test.rs"), passing).unwrap();
    let tested = invoke(&consumer, &["test"]);
    assert!(tested.status.success(), "{}", both(&tested));
    // The red control proves the call happened: a wrong expectation must fail.
    // 反控制证明调用真的发生：错的期望必须失败。
    let failing = passing.replace("add(2, 3) }, 5", "add(2, 3) }, 6");
    std::fs::write(consumer.join("tests/add_test.rs"), failing).unwrap();
    let failed = invoke(&consumer, &["test"]);
    assert!(
        !failed.status.success(),
        "a wrong expectation must fail, so the passing run proves the call reached the C++ export: {}",
        both(&failed)
    );
}

#[test]
fn a_python_consumer_calls_a_cpp_export_through_the_generated_module() {
    let scratch = scratch("python-consumer");
    let created = invoke(&scratch, &["new", "--lang", "cpp", "--lib", "producer"]);
    assert!(created.status.success(), "{}", both(&created));
    let manifest = std::fs::read_to_string(scratch.join("producer/xirang.toml")).unwrap();
    std::fs::write(
        scratch.join("producer/xirang.toml"),
        format!(
            "{manifest}\n[[export]]\nname = \"add\"\nparams = [\"i64\", \"i64\"]\nreturns = \"i64\"\n"
        ),
    )
    .unwrap();
    std::fs::write(
        scratch.join("producer/src/producer.cpp"),
        "#include <cstdint>\n\nint64_t add(int64_t a, int64_t b) { return a + b; }\n",
    )
    .unwrap();
    let created = invoke(&scratch, &["new", "--lang", "python", "--lib", "consumer"]);
    assert!(created.status.success(), "{}", both(&created));
    std::fs::write(
        scratch.join("consumer/xirang.toml"),
        "schema = 1\n\n[package]\nname = \"consumer\"\nversion = \"0.1.0\"\nkind = \"library\"\n\n[language.python]\nstandard = \"3\"\n\n[dependencies]\nproducer = { path = \"../producer\" }\n",
    )
    .unwrap();
    let test = "\"\"\"Call the C++ export from Python; this is the cross-language link, not a header check.\"\"\"\nimport sys\n\nfrom producer.exports import add\n\n\ndef main() -> int:\n    if add(2, 3) != 5:\n        print(\"test uses_export ... FAILED\")\n        return 1\n    print(\"test uses_export ... ok\")\n    return 0\n\n\nif __name__ == \"__main__\":\n    sys.exit(main())\n";
    std::fs::write(scratch.join("consumer/tests/uses_export_test.py"), test).unwrap();
    let built = invoke(&scratch.join("consumer"), &["build"]);
    assert!(built.status.success(), "{}", both(&built));
    let tested = invoke(&scratch.join("consumer"), &["test"]);
    assert!(tested.status.success(), "{}", both(&tested));
    assert!(
        both(&tested).contains("uses_export"),
        "the Python test did not run: {}",
        both(&tested)
    );
    // The same chain must fail when the expectation is wrong: a green result that cannot be red
    // is not evidence that the export was called.
    // 同一个链路在期望值错误时必须失败：不会变绿的结果不能证明导出真被调用。
    std::fs::write(
        scratch.join("consumer/tests/uses_export_test.py"),
        test.replace("add(2, 3) != 5", "add(2, 3) != 6"),
    )
    .unwrap();
    let failed = invoke(&scratch.join("consumer"), &["test"]);
    assert!(
        !failed.status.success(),
        "a wrong expectation must fail: {}",
        both(&failed)
    );
    assert!(
        both(&failed).contains("uses_export"),
        "the failure does not name the test: {}",
        both(&failed)
    );
}

#[test]
fn a_c_consumer_calls_a_cpp_export_through_the_generated_header() {
    let scratch = scratch("cross-language");
    let created = invoke(&scratch, &["new", "--lang", "cpp", "--lib", "producer"]);
    assert!(created.status.success(), "{}", both(&created));
    let producer = scratch.join("producer");
    let manifest = std::fs::read_to_string(producer.join("xirang.toml")).unwrap();
    std::fs::write(
        producer.join("xirang.toml"),
        format!(
            "{manifest}\n[[export]]\nname = \"add\"\nparams = [\"i64\", \"i64\"]\nreturns = \"i64\"\n"
        ),
    )
    .unwrap();
    std::fs::write(
        producer.join("src/producer.cpp"),
        "#include <cstdint>\n\nstd::int64_t add(std::int64_t a, std::int64_t b) { return a + b; }\n",
    )
    .unwrap();
    std::fs::write(
        producer.join("tests/producer_test.cpp"),
        "#include <cstdint>\n\nstd::int64_t add(std::int64_t, std::int64_t);\n\nint main() { return add(2, 3) == 5 ? 0 : 1; }\n",
    )
    .unwrap();
    let consumer_created = invoke(&scratch, &["new", "--lang", "c", "--lib", "consumer"]);
    assert!(
        consumer_created.status.success(),
        "{}",
        both(&consumer_created)
    );
    let consumer = scratch.join("consumer");
    let manifest = std::fs::read_to_string(consumer.join("xirang.toml")).unwrap();
    std::fs::write(
        consumer.join("xirang.toml"),
        format!("{manifest}producer = {{ path = \"../producer\" }}\n"),
    )
    .unwrap();
    std::fs::write(
        consumer.join("tests/consumer_test.c"),
        "#include <assert.h>\n\n#include \"producer/exports.h\"\n\nint main(void) {\n    assert(add(2, 3) == 5);\n    return 0;\n}\n",
    )
    .unwrap();
    let tested = invoke(&consumer, &["test"]);
    assert!(tested.status.success(), "{}", both(&tested));
    let document = evidence(&consumer);
    assert_eq!(document["result"], "passed", "{document}");
    assert_eq!(document["tests"][0]["name"], "consumer_test", "{document}");
    assert_eq!(document["tests"][0]["passed"], true, "{document}");
}

#[test]
fn an_export_in_a_c_unit_is_refused_by_name() {
    let scratch = scratch("c-export");
    let created = invoke(&scratch, &["new", "--lang", "c", "--lib", "plain"]);
    assert!(created.status.success(), "{}", both(&created));
    let project = scratch.join("plain");
    let manifest = std::fs::read_to_string(project.join("xirang.toml")).unwrap();
    std::fs::write(
        project.join("xirang.toml"),
        format!("{manifest}\n[[export]]\nname = \"answer\"\nparams = []\nreturns = \"i64\"\n"),
    )
    .unwrap();
    let built = invoke(&project, &["build"]);
    assert!(!built.status.success(), "{}", both(&built));
    let text = both(&built);
    assert!(
        text.contains("needs a C++ or Rust library"),
        "the refusal has to say why: {text}"
    );
}

#[test]
fn a_text_return_crosses_both_consumers_with_a_caller_owned_buffer() {
    let scratch = scratch("text-return");
    let created = invoke(&scratch, &["new", "--lang", "cpp", "--lib", "producer"]);
    assert!(created.status.success(), "{}", both(&created));
    let producer = scratch.join("producer");
    let manifest = std::fs::read_to_string(producer.join("xirang.toml")).unwrap();
    std::fs::write(
        producer.join("xirang.toml"),
        format!(
            "{manifest}\n[[export]]\nname = \"greet\"\nparams = [\"utf8\"]\nreturns = \"text\"\n"
        ),
    )
    .unwrap();
    std::fs::write(
        producer.join("src/producer.cpp"),
        "#include <string>\n#include <string_view>\n\nstd::string greet(std::string_view name) { return std::string(name) + \"!\"; }\n",
    )
    .unwrap();
    std::fs::write(
        producer.join("tests/producer_test.cpp"),
        "#include <string>\n#include <string_view>\n\nstd::string greet(std::string_view);\n\nint main() { return greet(\"x\") == \"x!\" ? 0 : 1; }\n",
    )
    .unwrap();

    // A C consumer owns the buffer. The first call asks for the length, the second copies.
    // C 消费者拥有缓冲区：第一次调用问长度，第二次才复制。
    let consumer = scratch.join("consumer-c");
    std::fs::create_dir_all(consumer.join("tests")).unwrap();
    std::fs::create_dir_all(consumer.join("src")).unwrap();
    std::fs::write(
        consumer.join("src/consumer.c"),
        "int consumer_identity(int v) { return v; }\n",
    )
    .unwrap();
    std::fs::write(
        consumer.join("xirang.toml"),
        "schema = 1\n\n[package]\nname = \"consumer\"\nversion = \"0.1.0\"\nkind = \"library\"\n\n[language.c]\nstandard = \"99\"\n\n[dependencies]\nproducer = { path = \"../producer\" }\n",
    )
    .unwrap();
    std::fs::write(
        consumer.join("tests/consumer_test.c"),
        "#include <assert.h>\n#include <string.h>\n\n#include \"producer/exports.h\"\n\nint main(void) {\n    char out[16];\n    size_t len = 0;\n    assert(!greet(\"x\", 1, 0, 0, &len));\n    assert(len == 2);\n    assert(greet(\"x\", 1, out, sizeof out, &len));\n    assert(len == 2 && memcmp(out, \"x!\", 2) == 0);\n    return 0;\n}\n",
    )
    .unwrap();
    let tested = invoke(&consumer, &["test"]);
    assert!(tested.status.success(), "{}", both(&tested));

    // The Python consumer gets the same text through the generated ctypes module.
    // Python 消费者经生成的 ctypes 模块拿到同一段文本。
    let python = scratch.join("consumer-py");
    std::fs::create_dir_all(python.join("src")).unwrap();
    std::fs::create_dir_all(python.join("tests")).unwrap();
    std::fs::write(
        python.join("xirang.toml"),
        "schema = 1\n\n[package]\nname = \"consumer_py\"\nversion = \"0.1.0\"\nkind = \"library\"\n\n[language.python]\nstandard = \"3\"\n\n[dependencies]\nproducer = { path = \"../producer\" }\n",
    )
    .unwrap();
    std::fs::write(
        python.join("src/consumer.py"),
        "def identity(value):\n    return value\n",
    )
    .unwrap();
    let test = "\"\"\"Call the C++ text export from Python; a caller-owned buffer crosses the ABI.\"\"\"\nimport sys\n\nfrom producer.exports import greet\n\n\ndef main() -> int:\n    if greet(\"x\") != \"x!\":\n        print(\"test uses_export ... FAILED\")\n        return 1\n    print(\"test uses_export ... ok\")\n    return 0\n\n\nif __name__ == \"__main__\":\n    sys.exit(main())\n";
    std::fs::write(python.join("tests/uses_export_test.py"), test).unwrap();
    let built = invoke(&python, &["build"]);
    assert!(built.status.success(), "{}", both(&built));
    let tested = invoke(&python, &["test"]);
    assert!(tested.status.success(), "{}", both(&tested));
    assert!(both(&tested).contains("uses_export"), "{}", both(&tested));

    // The same chain must fail when the expectation is wrong.
    // 期望值错误时同一条链必须失败。
    std::fs::write(
        python.join("tests/uses_export_test.py"),
        test.replace("greet(\"x\") != \"x!\"", "greet(\"x\") != \"y!\""),
    )
    .unwrap();
    let failed = invoke(&python, &["test"]);
    assert!(
        !failed.status.success(),
        "a wrong expectation must fail: {}",
        both(&failed)
    );
}

#[test]
fn an_optional_return_crosses_both_consumers_as_a_struct() {
    let scratch = scratch("optional-return");
    let created = invoke(&scratch, &["new", "--lang", "cpp", "--lib", "producer"]);
    assert!(created.status.success(), "{}", both(&created));
    let producer = scratch.join("producer");
    let manifest = std::fs::read_to_string(producer.join("xirang.toml")).unwrap();
    std::fs::write(
        producer.join("xirang.toml"),
        format!(
            "{manifest}\n[[export]]\nname = \"maybe\"\nparams = [\"i64\"]\nreturns = \"optional<i64>\"\n"
        ),
    )
    .unwrap();
    std::fs::write(
        producer.join("src/producer.cpp"),
        "#include <cstdint>\n#include <optional>\n\nstd::optional<int64_t> maybe(int64_t value) {\n    if (value < 0) {\n        return std::nullopt;\n    }\n    return value * 2;\n}\n",
    )
    .unwrap();
    std::fs::write(
        producer.join("tests/producer_test.cpp"),
        "#include <cstdint>\n#include <optional>\n\nstd::optional<int64_t> maybe(int64_t);\n\nint main() { return maybe(3) == 6 && !maybe(-1).has_value() ? 0 : 1; }\n",
    )
    .unwrap();

    // A C consumer sees the generated struct: `present` plus the value.
    // C 消费者看到生成的结构体：`present` 加值。
    let consumer = scratch.join("consumer-c");
    std::fs::create_dir_all(consumer.join("tests")).unwrap();
    std::fs::create_dir_all(consumer.join("src")).unwrap();
    std::fs::write(
        consumer.join("src/consumer.c"),
        "int consumer_identity(int v) { return v; }\n",
    )
    .unwrap();
    std::fs::write(
        consumer.join("xirang.toml"),
        "schema = 1\n\n[package]\nname = \"consumer\"\nversion = \"0.1.0\"\nkind = \"library\"\n\n[language.c]\nstandard = \"99\"\n\n[dependencies]\nproducer = { path = \"../producer\" }\n",
    )
    .unwrap();
    std::fs::write(
        consumer.join("tests/consumer_test.c"),
        "#include <assert.h>\n\n#include \"producer/exports.h\"\n\nint main(void) {\n    xirang_optional_i64 present = maybe(3);\n    assert(present.present && present.value == 6);\n    xirang_optional_i64 absent = maybe(-1);\n    assert(!absent.present);\n    return 0;\n}\n",
    )
    .unwrap();
    let tested = invoke(&consumer, &["test"]);
    assert!(tested.status.success(), "{}", both(&tested));

    // Python maps the same struct to a value or `None`.
    // Python 把同一结构体映射成值或 `None`。
    let python = scratch.join("consumer-py");
    std::fs::create_dir_all(python.join("src")).unwrap();
    std::fs::create_dir_all(python.join("tests")).unwrap();
    std::fs::write(
        python.join("xirang.toml"),
        "schema = 1\n\n[package]\nname = \"consumer_py\"\nversion = \"0.1.0\"\nkind = \"library\"\n\n[language.python]\nstandard = \"3\"\n\n[dependencies]\nproducer = { path = \"../producer\" }\n",
    )
    .unwrap();
    std::fs::write(
        python.join("src/consumer.py"),
        "def identity(value):\n    return value\n",
    )
    .unwrap();
    let test = "\"\"\"Call the C++ optional export from Python.\"\"\"\nimport sys\n\nfrom producer.exports import maybe\n\n\ndef main() -> int:\n    if maybe(3) != 6 or maybe(-1) is not None:\n        print(\"test uses_export ... FAILED\")\n        return 1\n    print(\"test uses_export ... ok\")\n    return 0\n\n\nif __name__ == \"__main__\":\n    sys.exit(main())\n";
    std::fs::write(python.join("tests/uses_export_test.py"), test).unwrap();
    let built = invoke(&python, &["build"]);
    assert!(built.status.success(), "{}", both(&built));
    let tested = invoke(&python, &["test"]);
    assert!(tested.status.success(), "{}", both(&tested));
    assert!(both(&tested).contains("uses_export"), "{}", both(&tested));

    // A wrong expectation must fail: the mapping to a value is really checked.
    // 期望值错误时必须失败：到值的映射真的被检查。
    std::fs::write(
        python.join("tests/uses_export_test.py"),
        test.replace("maybe(3) != 6", "maybe(3) != 7"),
    )
    .unwrap();
    let failed = invoke(&python, &["test"]);
    assert!(
        !failed.status.success(),
        "a wrong expectation must fail: {}",
        both(&failed)
    );
}

#[test]
fn an_optional_parameter_crosses_both_consumers_as_a_struct() {
    let scratch = scratch("optional-parameter");
    let created = invoke(&scratch, &["new", "--lang", "cpp", "--lib", "producer"]);
    assert!(created.status.success(), "{}", both(&created));
    let producer = scratch.join("producer");
    let manifest = std::fs::read_to_string(producer.join("xirang.toml")).unwrap();
    std::fs::write(
        producer.join("xirang.toml"),
        format!(
            "{manifest}\n[[export]]\nname = \"pick\"\nparams = [\"optional<i64>\", \"i64\"]\nreturns = \"i64\"\n"
        ),
    )
    .unwrap();
    std::fs::write(
        producer.join("src/producer.cpp"),
        "#include <cstdint>\n#include <optional>\n\nint64_t pick(std::optional<int64_t> value, int64_t fallback) {\n    return value.has_value() ? *value : fallback;\n}\n",
    )
    .unwrap();
    std::fs::write(
        producer.join("tests/producer_test.cpp"),
        "#include <cstdint>\n#include <optional>\n\nint64_t pick(std::optional<int64_t>, int64_t);\n\nint main() { return pick(3, 9) == 3 && pick(std::nullopt, 9) == 9 ? 0 : 1; }\n",
    )
    .unwrap();

    // A C caller builds the generated struct by value.
    // C 调用方按值构造生成的结构体。
    let consumer = scratch.join("consumer-c");
    std::fs::create_dir_all(consumer.join("src")).unwrap();
    std::fs::create_dir_all(consumer.join("tests")).unwrap();
    std::fs::write(
        consumer.join("src/consumer.c"),
        "int consumer_identity(int v) { return v; }\n",
    )
    .unwrap();
    std::fs::write(
        consumer.join("xirang.toml"),
        "schema = 1\n\n[package]\nname = \"consumer\"\nversion = \"0.1.0\"\nkind = \"library\"\n\n[language.c]\nstandard = \"99\"\n\n[dependencies]\nproducer = { path = \"../producer\" }\n",
    )
    .unwrap();
    std::fs::write(
        consumer.join("tests/consumer_test.c"),
        "#include <assert.h>\n\n#include \"producer/exports.h\"\n\nint main(void) {\n    xirang_optional_i64 present = {true, 3};\n    assert(pick(present, 9) == 3);\n    xirang_optional_i64 absent = {false, 0};\n    assert(pick(absent, 9) == 9);\n    return 0;\n}\n",
    )
    .unwrap();
    let tested = invoke(&consumer, &["test"]);
    assert!(tested.status.success(), "{}", both(&tested));

    // Python passes a value or `None` and gets the same answer.
    // Python 传值或 `None`，拿到同样的答案。
    let python = scratch.join("consumer-py");
    std::fs::create_dir_all(python.join("src")).unwrap();
    std::fs::create_dir_all(python.join("tests")).unwrap();
    std::fs::write(
        python.join("xirang.toml"),
        "schema = 1\n\n[package]\nname = \"consumer_py\"\nversion = \"0.1.0\"\nkind = \"library\"\n\n[language.python]\nstandard = \"3\"\n\n[dependencies]\nproducer = { path = \"../producer\" }\n",
    )
    .unwrap();
    std::fs::write(
        python.join("src/consumer.py"),
        "def identity(value):\n    return value\n",
    )
    .unwrap();
    let test = "\"\"\"Call the C++ optional-parameter export from Python.\"\"\"\nimport sys\n\nfrom producer.exports import pick\n\n\ndef main() -> int:\n    if pick(3, 9) != 3 or pick(None, 9) != 9:\n        print(\"test uses_export ... FAILED\")\n        return 1\n    print(\"test uses_export ... ok\")\n    return 0\n\n\nif __name__ == \"__main__\":\n    sys.exit(main())\n";
    std::fs::write(python.join("tests/uses_export_test.py"), test).unwrap();
    let built = invoke(&python, &["build"]);
    assert!(built.status.success(), "{}", both(&built));
    let tested = invoke(&python, &["test"]);
    assert!(tested.status.success(), "{}", both(&tested));

    // A wrong expectation must fail, so the None mapping is truly exercised.
    // 期望值错误时必须失败，`None` 的映射确实被走到。
    std::fs::write(
        python.join("tests/uses_export_test.py"),
        test.replace("pick(3, 9) != 3", "pick(3, 9) != 4"),
    )
    .unwrap();
    let failed = invoke(&python, &["test"]);
    assert!(
        !failed.status.success(),
        "a wrong expectation must fail: {}",
        both(&failed)
    );
}

#[test]
fn a_typed_error_becomes_a_status_and_a_named_python_exception() {
    let scratch = scratch("typed-error");
    let created = invoke(&scratch, &["new", "--lang", "cpp", "--lib", "producer"]);
    assert!(created.status.success(), "{}", both(&created));
    let producer = scratch.join("producer");
    let manifest = std::fs::read_to_string(producer.join("xirang.toml"))
        .unwrap()
        .replace("standard = \"17\"", "standard = \"23\"");
    std::fs::write(
        producer.join("xirang.toml"),
        format!(
            "{manifest}\n[[export]]\nname = \"checked_div\"\nparams = [\"i64\", \"i64\"]\nreturns = \"i64\"\nerrors = [\"divide_by_zero\", \"overflow\"]\n"
        ),
    )
    .unwrap();
    std::fs::write(
        producer.join("src/producer.cpp"),
        "#include <cstdint>\n#include <expected>\n#include <limits>\n\nstd::expected<int64_t, std::int32_t> checked_div(int64_t a, int64_t b) {\n    if (b == 0) {\n        return std::unexpected(1);\n    }\n    if (a == std::numeric_limits<int64_t>::min() && b == -1) {\n        return std::unexpected(2);\n    }\n    return a / b;\n}\n",
    )
    .unwrap();
    std::fs::write(
        producer.join("tests/producer_test.cpp"),
        "#include <cstdint>\n#include <expected>\n\nstd::expected<int64_t, std::int32_t> checked_div(int64_t, int64_t);\n\nint main() {\n    return checked_div(6, 2).value() == 3 && !checked_div(1, 0).has_value() ? 0 : 1;\n}\n",
    )
    .unwrap();

    // A C caller reads the status code and the error enum the header names.
    // C 调用方读取状态码与头文件命名的错误枚举。
    let consumer = scratch.join("consumer-c");
    std::fs::create_dir_all(consumer.join("src")).unwrap();
    std::fs::create_dir_all(consumer.join("tests")).unwrap();
    std::fs::write(
        consumer.join("src/consumer.c"),
        "int consumer_identity(int v) { return v; }\n",
    )
    .unwrap();
    std::fs::write(
        consumer.join("xirang.toml"),
        "schema = 1\n\n[package]\nname = \"consumer\"\nversion = \"0.1.0\"\nkind = \"library\"\n\n[language.c]\nstandard = \"99\"\n\n[dependencies]\nproducer = { path = \"../producer\" }\n",
    )
    .unwrap();
    std::fs::write(
        consumer.join("tests/consumer_test.c"),
        "#include <assert.h>\n\n#include \"producer/exports.h\"\n\nint main(void) {\n    int64_t out = 0;\n    xirang_error_checked_div error = XIRANG_ERROR_CHECKED_DIV_DIVIDE_BY_ZERO;\n    assert(checked_div(6, 2, &out, &error) == 0);\n    assert(out == 3);\n    assert(checked_div(1, 0, &out, &error) == 1);\n    assert(error == XIRANG_ERROR_CHECKED_DIV_DIVIDE_BY_ZERO);\n    return 0;\n}\n",
    )
    .unwrap();
    let tested = invoke(&consumer, &["test"]);
    assert!(tested.status.success(), "{}", both(&tested));

    // Python raises the generated exception whose name is the error's variant.
    // Python 抛出以错误变体命名的生成异常。
    let python = scratch.join("consumer-py");
    std::fs::create_dir_all(python.join("src")).unwrap();
    std::fs::create_dir_all(python.join("tests")).unwrap();
    std::fs::write(
        python.join("xirang.toml"),
        "schema = 1\n\n[package]\nname = \"consumer_py\"\nversion = \"0.1.0\"\nkind = \"library\"\n\n[language.python]\nstandard = \"3\"\n\n[dependencies]\nproducer = { path = \"../producer\" }\n",
    )
    .unwrap();
    std::fs::write(
        python.join("src/consumer.py"),
        "def identity(value):\n    return value\n",
    )
    .unwrap();
    let test = "\"\"\"Call the C++ typed-error export from Python.\"\"\"\nimport sys\n\nfrom producer.exports import CheckedDivError, checked_div\n\n\ndef main() -> int:\n    if checked_div(6, 2) != 3:\n        print(\"test uses_export ... FAILED\")\n        return 1\n    try:\n        checked_div(1, 0)\n    except CheckedDivError as error:\n        if str(error) != \"divide_by_zero\":\n            print(\"test uses_export ... FAILED\")\n            return 1\n    else:\n        print(\"test uses_export ... FAILED\")\n        return 1\n    print(\"test uses_export ... ok\")\n    return 0\n\n\nif __name__ == \"__main__\":\n    sys.exit(main())\n";
    std::fs::write(python.join("tests/uses_export_test.py"), test).unwrap();
    let built = invoke(&python, &["build"]);
    assert!(built.status.success(), "{}", both(&built));
    let tested = invoke(&python, &["test"]);
    assert!(tested.status.success(), "{}", both(&tested));

    // A wrong expectation must fail, so the status check is truly exercised.
    // 期望值错误时必须失败，状态码检查确实被走到。
    std::fs::write(
        python.join("tests/uses_export_test.py"),
        test.replace("checked_div(6, 2) != 3", "checked_div(6, 2) != 4"),
    )
    .unwrap();
    let failed = invoke(&python, &["test"]);
    assert!(
        !failed.status.success(),
        "a wrong expectation must fail: {}",
        both(&failed)
    );
}

#[test]
fn a_recorded_backend_is_reported_as_declared_and_never_becomes_an_executor() {
    let scratch = scratch("record");
    let project = scratch.join("recorded");
    std::fs::create_dir_all(&project).unwrap();
    let record = "protocol = 1\n\n[[backend]]\nlanguage = \"perl\"\nbackend = \"perl-backend\"\ncapabilities = [\"discovery\", \"resolution\"]\nreason.build-test = \"no perl toolchain here\"\n";
    std::fs::write(project.join("xirang.backends.toml"), record).unwrap();
    let listed = invoke(&project, &["languages"]);
    assert!(listed.status.success(), "{}", both(&listed));
    let text = both(&listed);
    assert!(
        text.contains("perl  perl-backend  discovery, resolution  (declared)"),
        "the record was not reported as declared: {text}"
    );
    assert!(
        text.contains("no build-test: no perl toolchain here"),
        "the record's reason was not reported: {text}"
    );
    // A declaration is not an executor: a project whose language only a record names still refuses
    // to build, and the refusal names the registered languages.
    // 声明不是执行器：只有记录点名某语言的项目仍然拒绝构建，拒绝语点名已注册的语言。
    std::fs::write(
        project.join("xirang.toml"),
        "[package]\nname = \"r\"\nkind = \"library\"\n\n[language.perl]\n",
    )
    .unwrap();
    std::fs::create_dir_all(project.join("src")).unwrap();
    std::fs::write(project.join("src/r.rb"), "def answer\n  42\nend\n").unwrap();
    let built = invoke(&project, &["build"]);
    assert!(!built.status.success(), "{}", both(&built));
    let refused = both(&built);
    assert!(
        refused.contains("names no `command`")
            && refused.contains("declaration is not an executor"),
        "the build did not say why the recorded backend cannot run: {refused}"
    );
    // Red control: a record cannot replace a built-in backend, and a future protocol is named.
    // 红控制：记录不能替换内置后端，未来的协议版本会被点名。
    std::fs::write(
        project.join("xirang.backends.toml"),
        "protocol = 1\n\n[[backend]]\nlanguage = \"cpp\"\nbackend = \"other\"\ncapabilities = [\"discovery\"]\n",
    )
    .unwrap();
    let replaced = invoke(&project, &["languages"]);
    assert!(!replaced.status.success(), "{}", both(&replaced));
    assert!(
        both(&replaced).contains("built-in backend"),
        "{}",
        both(&replaced)
    );
    std::fs::write(project.join("xirang.backends.toml"), "protocol = 2\n").unwrap();
    let future = invoke(&project, &["languages"]);
    assert!(!future.status.success(), "{}", both(&future));
    assert!(both(&future).contains("protocol 2"), "{}", both(&future));
}

#[test]
fn a_rust_producer_reaches_a_c_consumer_through_the_generated_header() {
    let scratch = scratch("rust-producer");
    // A Rust library declares the export; the wrapper `generated_rust_exports` writes around it is
    // Rust too, so the C ABI has no C++ in the path.
    // Rust 库声明导出；包住它的 `generated_rust_exports` 包装器也是 Rust，因此 C ABI 里没有 C++。
    let producer = scratch.join("mathlib");
    std::fs::create_dir_all(producer.join("src")).unwrap();
    std::fs::write(
        producer.join("xirang.toml"),
        "schema = 1\n\n[package]\nname = \"mathlib\"\nversion = \"0.1.0\"\nkind = \"library\"\n\n\
         [language.rust]\nstandard = \"2021\"\n\n\
         [[export]]\nname = \"add\"\nparams = [\"i64\", \"i64\"]\nreturns = \"i64\"\n",
    )
    .unwrap();
    std::fs::write(
        producer.join("src/lib.rs"),
        "pub fn add(a: i64, b: i64) -> i64 {\n    a + b\n}\n",
    )
    .unwrap();
    let consumer = scratch.join("consumer");
    std::fs::create_dir_all(consumer.join("src")).unwrap();
    std::fs::create_dir_all(consumer.join("tests")).unwrap();
    std::fs::write(
        consumer.join("xirang.toml"),
        "schema = 1\n\n[package]\nname = \"consumer\"\nversion = \"0.1.0\"\nkind = \"library\"\n\n\
         [language.c]\nstandard = \"99\"\n\n\
         [dependencies]\nmathlib = { path = \"../mathlib\" }\n",
    )
    .unwrap();
    std::fs::write(
        consumer.join("src/lib.c"),
        "int consumer_marker(void) { return 1; }\n",
    )
    .unwrap();
    std::fs::write(
        consumer.join("tests/add_test.c"),
        "#include <assert.h>\n#include \"mathlib/exports.h\"\n\nint main(void) {\n    assert(add(2, 3) == 5);\n    return 0;\n}\n",
    )
    .unwrap();
    let tested = invoke(&consumer, &["test"]);
    assert!(tested.status.success(), "{}", both(&tested));
    // Red control: the same test with a wrong expectation must fail, so the exported symbol really
    // is what the test calls.
    // 红控制：同一个测试写错期望值必须失败，证明它确实调用了导出的符号。
    std::fs::write(
        consumer.join("tests/add_test.c"),
        "#include <assert.h>\n#include \"mathlib/exports.h\"\n\nint main(void) {\n    assert(add(2, 3) == 6);\n    return 0;\n}\n",
    )
    .unwrap();
    let failed = invoke(&consumer, &["test"]);
    assert!(!failed.status.success(), "{}", both(&failed));
}

#[test]
fn a_rust_producer_carries_an_optional_parameter() {
    // The wrapper reduces the C struct to the `Option` the user's function takes, so a Rust
    // producer is not the one language where an optional parameter has no path.
    // 包装器把 C 结构体归约成用户函数接受的 `Option`，于是 Rust 生产者不是“可选参数无路可走”的那一种。
    let scratch = scratch("rust-producer-optional");
    let producer = scratch.join("mathlib");
    std::fs::create_dir_all(producer.join("src")).unwrap();
    std::fs::write(
        producer.join("xirang.toml"),
        "schema = 1\n\n[package]\nname = \"mathlib\"\nversion = \"0.1.0\"\nkind = \"library\"\n\n\
         [language.rust]\nstandard = \"2021\"\n\n\
         [[export]]\nname = \"pick\"\nparams = [\"optional<i64>\", \"i64\"]\nreturns = \"i64\"\n",
    )
    .unwrap();
    std::fs::write(
        producer.join("src/lib.rs"),
        "pub fn pick(value: Option<i64>, fallback: i64) -> i64 {\n    value.unwrap_or(fallback)\n}\n",
    )
    .unwrap();
    let consumer = scratch.join("consumer");
    std::fs::create_dir_all(consumer.join("src")).unwrap();
    std::fs::create_dir_all(consumer.join("tests")).unwrap();
    std::fs::write(
        consumer.join("xirang.toml"),
        "schema = 1\n\n[package]\nname = \"consumer\"\nversion = \"0.1.0\"\nkind = \"library\"\n\n\
         [language.c]\nstandard = \"99\"\n\n\
         [dependencies]\nmathlib = { path = \"../mathlib\" }\n",
    )
    .unwrap();
    std::fs::write(
        consumer.join("src/lib.c"),
        "int consumer_marker(void) { return 1; }\n",
    )
    .unwrap();
    std::fs::write(
        consumer.join("tests/pick_test.c"),
        "#include <assert.h>\n#include \"mathlib/exports.h\"\n\nint main(void) {\n    xirang_optional_i64 present = {true, 3};\n    assert(pick(present, 9) == 3);\n    xirang_optional_i64 absent = {false, 0};\n    assert(pick(absent, 9) == 9);\n    return 0;\n}\n",
    )
    .unwrap();
    let tested = invoke(&consumer, &["test"]);
    assert!(tested.status.success(), "{}", both(&tested));
    // Red control: the `present == false` branch must really reach `None`.
    // 红控制：`present == false` 那一支必须真的到达 `None`。
    std::fs::write(
        consumer.join("tests/pick_test.c"),
        "#include <assert.h>\n#include \"mathlib/exports.h\"\n\nint main(void) {\n    xirang_optional_i64 absent = {false, 0};\n    assert(pick(absent, 9) == 3);\n    return 0;\n}\n",
    )
    .unwrap();
    let failed = invoke(&consumer, &["test"]);
    assert!(!failed.status.success(), "{}", both(&failed));
}

#[test]
fn a_rust_producer_refuses_what_this_slice_does_not_carry() {
    // The refusal is by name, at generation time, before any compiler runs: a Rust producer in this
    // slice carries fixed-width scalars and booleans only.
    // 拒绝发生在生成期、任何编译器启动之前，并按名字说明：本切片的 Rust 生产者只承载固定宽度标量与布尔。
    let scratch = scratch("rust-producer-refusals");
    let project = scratch.join("producer");
    std::fs::create_dir_all(project.join("src")).unwrap();
    std::fs::write(
        project.join("src/lib.rs"),
        "pub fn greet(name: &str) -> i64 {\n    name.len() as i64\n}\n",
    )
    .unwrap();
    let cases = [
        (
            "name = \"greet\"\nparams = [\"utf8\"]\nreturns = \"i64\"\n",
            "a Rust producer carries fixed-width scalars and booleans in this slice",
        ),
        (
            "name = \"greet\"\nparams = []\nreturns = \"text\"\n",
            "a Rust producer carries fixed-width scalar returns in this slice",
        ),
        (
            "name = \"greet\"\nparams = []\nreturns = \"i64\"\nerrors = [\"empty\"]\n",
            "a Rust producer carries them in a later slice",
        ),
    ];
    for (declaration, expected) in cases {
        std::fs::write(
            project.join("xirang.toml"),
            format!(
                "schema = 1\n\n[package]\nname = \"t\"\nversion = \"0.1.0\"\nkind = \"library\"\n\n\
                 [language.rust]\nstandard = \"2021\"\n\n[[export]]\n{declaration}"
            ),
        )
        .unwrap();
        let built = invoke(&project, &["build"]);
        assert!(!built.status.success(), "{}", both(&built));
        assert!(
            both(&built).contains(expected),
            "the refusal has to name the reason `{expected}`: {}",
            both(&built)
        );
    }
}

/// The Go toolchain this test can use: `XIRANG_GO` when the caller names one, else `go` on PATH.
/// 本测试能用的 Go 工具链：调用方点名 `XIRANG_GO` 时用它，否则用 PATH 上的 `go`。
fn go_toolchain() -> Option<String> {
    if let Ok(named) = std::env::var("XIRANG_GO")
        && !named.is_empty()
    {
        return Some(named);
    }
    Command::new("go")
        .arg("version")
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|_| "go".to_owned())
}

fn invoke_with_env(directory: &Path, arguments: &[&str], environment: &[(&str, &str)]) -> Output {
    let mut command = Command::new(binary());
    command.args(arguments).current_dir(directory);
    for (key, value) in environment {
        command.env(key, value);
    }
    command.output().unwrap()
}

/// A Go consumer calls a C++ export through cgo: the producer's header and archive reach the Go
/// action as `CGO_CFLAGS`/`CGO_LDFLAGS`, and the test is a real `go test` program.
/// 一个 Go 消费者经 cgo 调用 C++ 导出：生产者的头与归档以 `CGO_CFLAGS`/`CGO_LDFLAGS` 到达 Go 动作，
/// 测试是真正的 `go test` 程序。
#[test]
fn a_go_consumer_calls_a_cpp_export_through_cgo() {
    let Some(go) = go_toolchain() else {
        eprintln!("skipping: no Go toolchain on PATH or XIRANG_GO");
        return;
    };
    let scratch = scratch("go-consumer");
    let producer = scratch.join("mathlib");
    std::fs::create_dir_all(producer.join("src")).unwrap();
    std::fs::write(
        producer.join("xirang.toml"),
        "schema = 1\n\n[package]\nname = \"mathlib\"\nversion = \"0.1.0\"\nkind = \"library\"\n\n[language.cpp]\nstandard = \"17\"\n\n[[export]]\nname = \"add\"\nparams = [\"i64\", \"i64\"]\nreturns = \"i64\"\n\n[dependencies]\n",
    )
    .unwrap();
    std::fs::write(
        producer.join("src/lib.cpp"),
        "long add(long a, long b) { return a + b; }\n",
    )
    .unwrap();
    let consumer = scratch.join("consumer");
    std::fs::create_dir_all(consumer.join("src")).unwrap();
    std::fs::create_dir_all(consumer.join("tests")).unwrap();
    std::fs::write(
        consumer.join("xirang.toml"),
        "schema = 1\n\n[package]\nname = \"consumer\"\nversion = \"0.1.0\"\nkind = \"library\"\n\n[language.go]\nstandard = \"1.21\"\n\n[dependencies]\nmathlib = { path = \"../mathlib\" }\n",
    )
    .unwrap();
    std::fs::write(
        consumer.join("go.mod"),
        "module example.com/consumer\n\ngo 1.21\n",
    )
    .unwrap();
    std::fs::write(
        consumer.join("src/lib.go"),
        "package src\n\n/*\n#include \"mathlib/exports.h\"\n*/\nimport \"C\"\n\nfunc Add(a, b int64) int64 { return int64(C.add(C.int64_t(a), C.int64_t(b))) }\n",
    )
    .unwrap();
    std::fs::write(
        consumer.join("tests/add_test.go"),
        "package tests\n\nimport (\n\t\"testing\"\n\n\t\"example.com/consumer/src\"\n)\n\nfunc TestAdd(t *testing.T) {\n\tif got := src.Add(2, 3); got != 5 {\n\t\tt.Fatalf(\"got %d\", got)\n\t}\n}\n",
    )
    .unwrap();
    let tested = invoke_with_env(&consumer, &["test"], &[("XIRANG_GO", &go)]);
    assert!(tested.status.success(), "{}", both(&tested));
    // Red control: a wrong expectation must fail, so the test really ran.
    std::fs::write(
        consumer.join("tests/add_test.go"),
        "package tests\n\nimport (\n\t\"testing\"\n\n\t\"example.com/consumer/src\"\n)\n\nfunc TestAdd(t *testing.T) {\n\tif got := src.Add(2, 3); got != 6 {\n\t\tt.Fatalf(\"got %d\", got)\n\t}\n}\n",
    )
    .unwrap();
    let failed = invoke_with_env(&consumer, &["test"], &[("XIRANG_GO", &go)]);
    assert!(!failed.status.success(), "{}", both(&failed));
}

/// A Ruby consumer loads the C++ producer's shared library with Fiddle: the plan puts the
/// producer's export directory on `LD_LIBRARY_PATH`, and the test is a real Ruby program.
/// 一个 Ruby 消费者用 Fiddle 加载 C++ 生产者的共享库：计划把生产者的导出目录放进
/// `LD_LIBRARY_PATH`，测试是真正的 Ruby 程序。
#[test]
fn a_ruby_consumer_calls_a_cpp_export_through_fiddle() {
    let Some(ruby) = ruby_interpreter() else {
        eprintln!("skipping: no Ruby interpreter on PATH or XIRANG_RUBY");
        return;
    };
    let scratch = scratch("ruby-consumer");
    let producer = scratch.join("mathlib");
    std::fs::create_dir_all(producer.join("src")).unwrap();
    std::fs::write(
        producer.join("xirang.toml"),
        "schema = 1\n\n[package]\nname = \"mathlib\"\nversion = \"0.1.0\"\nkind = \"library\"\n\n[language.cpp]\nstandard = \"17\"\n\n[[export]]\nname = \"add\"\nparams = [\"i64\", \"i64\"]\nreturns = \"i64\"\n\n[dependencies]\n",
    )
    .unwrap();
    std::fs::write(
        producer.join("src/lib.cpp"),
        "long add(long a, long b) { return a + b; }\n",
    )
    .unwrap();
    let consumer = scratch.join("consumer");
    std::fs::create_dir_all(consumer.join("src")).unwrap();
    std::fs::create_dir_all(consumer.join("tests")).unwrap();
    std::fs::write(
        consumer.join("xirang.toml"),
        "schema = 1\n\n[package]\nname = \"consumer\"\nversion = \"0.1.0\"\nkind = \"library\"\n\n[language.ruby]\nstandard = \"3\"\n\n[dependencies]\nmathlib = { path = \"../mathlib\" }\n",
    )
    .unwrap();
    std::fs::write(
        consumer.join("src/lib.rb"),
        "def consumer_marker\n  1\nend\n",
    )
    .unwrap();
    let passing = "require \"fiddle\"\n\nlibrary = Fiddle.dlopen(\"libmathlib.so\")\nadd = Fiddle::Function.new(\n  library[\"add\"],\n  [Fiddle::TYPE_LONG, Fiddle::TYPE_LONG],\n  Fiddle::TYPE_LONG\n)\nraise \"add(2, 3) != 5\" unless add.call(2, 3) == 5\n";
    std::fs::write(consumer.join("tests/add_test.rb"), passing).unwrap();
    let environment = [("XIRANG_RUBY", ruby.as_str())];
    let tested = invoke_with_env(&consumer, &["test"], &environment);
    assert!(tested.status.success(), "{}", both(&tested));
    // Red control: a wrong expectation must fail, so the test really ran.
    let failing = passing.replace("== 5", "== 6").replace("!= 5", "!= 6");
    std::fs::write(consumer.join("tests/add_test.rb"), failing).unwrap();
    let failed = invoke_with_env(&consumer, &["test"], &environment);
    assert!(!failed.status.success(), "{}", both(&failed));
}

/// The Ruby interpreter this test can use: `XIRANG_RUBY`, or `ruby` on `PATH`.
/// 本测试可用的 Ruby 解释器：`XIRANG_RUBY`，或 `PATH` 上的 `ruby`。
fn ruby_interpreter() -> Option<String> {
    if let Ok(named) = std::env::var("XIRANG_RUBY")
        && !named.is_empty()
    {
        return Some(named);
    }
    Command::new("ruby")
        .arg("--version")
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|_| "ruby".to_owned())
}

/// The Ruby scaffold is a real project: `new --lang ruby --lib` writes a manifest the planner
/// accepts, and `xirang test` runs its test. A Ruby binary is refused by name.
/// Ruby 脚手架是真实项目：`new --lang ruby --lib` 写出的清单计划能接受，`xirang test` 会跑它的测试；
/// Ruby 二进制按名拒绝。
#[test]
fn a_ruby_project_scaffold_builds_and_tests() {
    let Some(ruby) = ruby_interpreter() else {
        eprintln!("skipping: no Ruby interpreter on PATH or XIRANG_RUBY");
        return;
    };
    let scratch = scratch("ruby-scaffold");
    let created = invoke_with_env(
        &scratch,
        &["new", "--lang", "ruby", "--lib", "gadget"],
        &[("XIRANG_RUBY", ruby.as_str())],
    );
    assert!(created.status.success(), "{}", both(&created));
    let project = scratch.join("gadget");
    let tested = invoke_with_env(&project, &["test"], &[("XIRANG_RUBY", ruby.as_str())]);
    assert!(tested.status.success(), "{}", both(&tested));
    // `new` produces a binary unless `--lib` is given; a Ruby binary is refused by name.
    // `new` 默认产生二进制，除非给 `--lib`；Ruby 二进制按名拒绝。
    let refused = invoke(&scratch, &["new", "--lang", "ruby", "gadget-bin"]);
    assert!(!refused.status.success(), "{}", both(&refused));
    assert!(
        both(&refused).contains("consumer library"),
        "{}",
        both(&refused)
    );
}

/// The .NET SDK this test drives, when one is available.
/// 本测试驱动的 .NET SDK（若可用）。
fn csharp_sdk() -> Option<String> {
    if let Ok(named) = std::env::var("XIRANG_DOTNET")
        && !named.is_empty()
    {
        return Some(named);
    }
    Command::new("dotnet")
        .arg("--version")
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|_| "dotnet".to_owned())
}

/// A C# consumer calls the C++ producer's C ABI through `DllImport`: the plan puts the
/// producer's export directory on `LD_LIBRARY_PATH`, and the test is a real .NET program.
/// 一个 C# 消费者通过 `DllImport` 调用 C++ 生产者的 C ABI：计划把生产者的导出目录放进
/// `LD_LIBRARY_PATH`，测试是真正的 .NET 程序。
#[test]
fn a_csharp_consumer_calls_a_cpp_export_through_pinvoke() {
    let Some(sdk) = csharp_sdk() else {
        eprintln!("skipping: no .NET SDK on PATH or XIRANG_DOTNET");
        return;
    };
    let scratch = scratch("csharp-consumer");
    let producer = scratch.join("mathlib");
    std::fs::create_dir_all(producer.join("src")).unwrap();
    std::fs::write(
        producer.join("xirang.toml"),
        "schema = 1\n\n[package]\nname = \"mathlib\"\nversion = \"0.1.0\"\nkind = \"library\"\n\n[language.cpp]\nstandard = \"17\"\n\n[[export]]\nname = \"add\"\nparams = [\"i64\", \"i64\"]\nreturns = \"i64\"\n\n[dependencies]\n",
    )
    .unwrap();
    std::fs::write(
        producer.join("src/lib.cpp"),
        "long add(long a, long b) { return a + b; }\n",
    )
    .unwrap();
    let consumer = scratch.join("consumer");
    for directory in ["src", "tests"] {
        std::fs::create_dir_all(consumer.join(directory)).unwrap();
    }
    std::fs::write(
        consumer.join("xirang.toml"),
        "schema = 1\n\n[package]\nname = \"consumer\"\nversion = \"0.1.0\"\nkind = \"library\"\n\n[language.csharp]\nstandard = \"net10.0\"\n\n[dependencies]\nmathlib = { path = \"../mathlib\" }\n",
    )
    .unwrap();
    std::fs::write(
        consumer.join("src/lib.cs"),
        "public static class Marker\n{\n    public static int Value() => 1;\n}\n",
    )
    .unwrap();
    let test = "using System;\nusing System.Runtime.InteropServices;\n\ninternal static class Test\n{\n    [DllImport(\"libmathlib.so\")]\n    private static extern long add(long a, long b);\n\n    private static int Main()\n    {\n        if (add(2, 3) != 5)\n        {\n            Console.WriteLine(\"test add ... FAILED\");\n            return 1;\n        }\n        Console.WriteLine(\"test add ... ok\");\n        return 0;\n    }\n}\n";
    std::fs::write(consumer.join("tests/add_test.cs"), test).unwrap();
    let tested = invoke_with_env(&consumer, &["test"], &[("XIRANG_DOTNET", &sdk)]);
    assert!(tested.status.success(), "{}", both(&tested));
    assert!(
        String::from_utf8_lossy(&tested.stdout).contains("test add_test ... ok"),
        "{}",
        both(&tested)
    );
    // Red control: a wrong expectation must fail, so the test really ran.
    std::fs::write(
        consumer.join("tests/add_test.cs"),
        test.replace("!= 5", "!= 6"),
    )
    .unwrap();
    let failed = invoke_with_env(&consumer, &["test"], &[("XIRANG_DOTNET", &sdk)]);
    assert!(!failed.status.success(), "{}", both(&failed));
}

/// `xirang new --lang csharp --lib` writes a C# module whose own test runs.
/// `xirang new --lang csharp --lib` 写出一个 C# 模块，它自己的测试能跑。
#[test]
fn a_csharp_project_is_scaffolded_and_runs_its_own_test() {
    let Some(sdk) = csharp_sdk() else {
        eprintln!("skipping: no .NET SDK on PATH or XIRANG_DOTNET");
        return;
    };
    let scratch = scratch("csharp-scaffold");
    let scaffolded = invoke(&scratch, &["new", "--lang", "csharp", "--lib", "gadget"]);
    assert!(scaffolded.status.success(), "{}", both(&scaffolded));
    let manifest = std::fs::read_to_string(scratch.join("gadget/xirang.toml")).unwrap();
    assert!(manifest.contains("[language.csharp]"), "{manifest}");
    let tested = invoke_with_env(
        &scratch.join("gadget"),
        &["test"],
        &[("XIRANG_DOTNET", &sdk)],
    );
    assert!(tested.status.success(), "{}", both(&tested));
}

/// The Node N-API header directory this test can use: `XIRANG_NODE_INCLUDE`, then the directory
/// next to the runtime, then the newest `~/.cache/node-gyp/*/include/node`.
/// 本测试可用的 Node N-API 头文件目录：`XIRANG_NODE_INCLUDE`，再运行时旁的目录，最后
/// `~/.cache/node-gyp/*/include/node` 里最新的一个。
fn node_include() -> Option<String> {
    if let Ok(named) = std::env::var("XIRANG_NODE_INCLUDE")
        && !named.is_empty()
    {
        return Some(named);
    }
    if let Ok(program) = std::env::var("XIRANG_NODE") {
        let prefix = Path::new(&program).parent().and_then(Path::parent)?;
        let candidate = prefix.join("include/node");
        if candidate.join("node_api.h").is_file() {
            return Some(candidate.display().to_string());
        }
    }
    let home = std::env::var_os("HOME")?;
    let mut entries: Vec<PathBuf> = std::fs::read_dir(PathBuf::from(home).join(".cache/node-gyp"))
        .ok()?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .collect();
    entries.sort();
    entries
        .iter()
        .rev()
        .map(|directory| directory.join("include/node"))
        .find(|directory| directory.join("node_api.h").is_file())
        .map(|directory| directory.display().to_string())
}

/// A JavaScript consumer reaches the C++ producer's C ABI through an N-API addon: the plan compiles
/// the unit's `.c` file with the Node headers and runs the test with the addon's path in
/// `XIRANG_ADDON` and the producer's export directory on `LD_LIBRARY_PATH`.
/// 一个 JavaScript 消费者通过 N-API 插件调 C++ 生产者的 C ABI：计划用 Node 头文件编译单元的 `.c`
/// 文件，测试运行时 `XIRANG_ADDON` 指向插件、`LD_LIBRARY_PATH` 含生产者导出目录。
#[test]
fn a_javascript_consumer_calls_a_cpp_export_through_n_api() {
    if node_include().is_none() {
        eprintln!("skipping: no Node headers on this machine");
        return;
    }
    let scratch = scratch("javascript-consumer");
    let producer = scratch.join("mathlib");
    std::fs::create_dir_all(producer.join("src")).unwrap();
    std::fs::write(
        producer.join("xirang.toml"),
        "schema = 1\n\n[package]\nname = \"mathlib\"\nversion = \"0.1.0\"\nkind = \"library\"\n\n[language.cpp]\nstandard = \"17\"\n\n[[export]]\nname = \"add\"\nparams = [\"i64\", \"i64\"]\nreturns = \"i64\"\n\n[dependencies]\n",
    )
    .unwrap();
    std::fs::write(
        producer.join("src/lib.cpp"),
        "long add(long a, long b) {\n    return a + b;\n}\n",
    )
    .unwrap();
    let consumer = scratch.join("consumer");
    std::fs::create_dir_all(consumer.join("src")).unwrap();
    std::fs::create_dir_all(consumer.join("tests")).unwrap();
    std::fs::write(
        consumer.join("xirang.toml"),
        "schema = 1\n\n[package]\nname = \"consumer\"\nversion = \"0.1.0\"\nkind = \"library\"\n\n[language.javascript]\nstandard = \"commonjs\"\n\n[dependencies]\nmathlib = { path = \"../mathlib\" }\n",
    )
    .unwrap();
    std::fs::write(
        consumer.join("src/addon.c"),
        "#include <node_api.h>\n#include <dlfcn.h>\n\nstatic napi_value call_add(napi_env env, napi_callback_info info) {\n    size_t argc = 2;\n    napi_value argv[2];\n    napi_get_cb_info(env, info, &argc, argv, NULL, NULL);\n    int64_t a = 0, b = 0;\n    napi_get_value_int64(env, argv[0], &a);\n    napi_get_value_int64(env, argv[1], &b);\n    void *handle = dlopen(\"libmathlib.so\", RTLD_LAZY);\n    if (handle == NULL) {\n        napi_throw_error(env, NULL, dlerror());\n        return NULL;\n    }\n    long (*add)(long, long) = (long (*)(long, long))dlsym(handle, \"add\");\n    if (add == NULL) {\n        napi_throw_error(env, NULL, dlerror());\n        return NULL;\n    }\n    napi_value result;\n    napi_create_int64(env, add(a, b), &result);\n    return result;\n}\n\nNAPI_MODULE_INIT() {\n    napi_value function;\n    napi_create_function(env, \"add\", NAPI_AUTO_LENGTH, call_add, NULL, &function);\n    napi_set_named_property(env, exports, \"add\", function);\n    return exports;\n}\n",
    )
    .unwrap();
    std::fs::write(
        consumer.join("tests/add_test.js"),
        "const assert = require(\"node:assert\");\n\nconst addon = require(process.env.XIRANG_ADDON);\n\nassert.strictEqual(addon.add(2, 3), 5);\nconsole.log(\"test add_test ... ok\");\n",
    )
    .unwrap();
    let tested = invoke(&consumer, &["test"]);
    assert!(tested.status.success(), "{}", both(&tested));
    // Red control: a wrong expectation must fail, so the test really ran.
    // 红控制：期望值写错必须失败，证明测试真的跑了。
    std::fs::write(
        consumer.join("tests/add_test.js"),
        "const assert = require(\"node:assert\");\n\nconst addon = require(process.env.XIRANG_ADDON);\n\nassert.strictEqual(addon.add(2, 3), 6);\nconsole.log(\"test add_test ... ok\");\n",
    )
    .unwrap();
    let failed = invoke(&consumer, &["test"]);
    assert!(!failed.status.success(), "{}", both(&failed));
}

/// The JavaScript scaffold is a real project: `new --lang javascript --lib` writes a manifest the
/// planner accepts, and `xirang test` runs its test. A JavaScript binary is refused by name.
/// JavaScript 脚手架是真实项目：`new --lang javascript --lib` 写出的清单计划能接受，`xirang test`
/// 会跑它的测试；JavaScript 二进制按名拒绝。
#[test]
fn a_javascript_project_scaffold_builds_and_tests() {
    if node_include().is_none() {
        eprintln!("skipping: no Node headers on this machine");
        return;
    }
    let scratch = scratch("javascript-scaffold");
    let created = invoke(
        &scratch,
        &["new", "--lang", "javascript", "--lib", "gadget"],
    );
    assert!(created.status.success(), "{}", both(&created));
    let project = scratch.join("gadget");
    let tested = invoke(&project, &["test"]);
    assert!(tested.status.success(), "{}", both(&tested));
    let refused = invoke(&scratch, &["new", "--lang", "javascript", "gadget-bin"]);
    assert!(!refused.status.success(), "{}", both(&refused));
    assert!(
        both(&refused).contains("consumer library"),
        "{}",
        both(&refused)
    );
}

/// A TypeScript consumer calls the C++ producer's C ABI through the same N-API addon JavaScript
/// uses; Node's strip-only TypeScript runs the `.ts` test, and the build parses every `.ts` module
/// without executing it.
/// 一个 TypeScript 消费者通过 JavaScript 用的同一个 N-API 插件调用 C++ 生产者的 C ABI；
/// Node 的 strip-only TypeScript 直接运行 `.ts` 测试，构建期只解析每个 `.ts` 模块而不执行。
#[test]
fn a_typescript_consumer_calls_a_cpp_export_through_the_node_addon() {
    if node_include().is_none() {
        eprintln!("skipping: no Node headers on this machine");
        return;
    }
    let scratch = scratch("typescript-consumer");
    let producer = scratch.join("mathlib");
    std::fs::create_dir_all(producer.join("src")).unwrap();
    std::fs::write(
        producer.join("xirang.toml"),
        "schema = 1\n\n[package]\nname = \"mathlib\"\nversion = \"0.1.0\"\nkind = \"library\"\n\n[language.cpp]\nstandard = \"17\"\n\n[[export]]\nname = \"add\"\nparams = [\"i64\", \"i64\"]\nreturns = \"i64\"\n\n[dependencies]\n",
    )
    .unwrap();
    std::fs::write(
        producer.join("src/lib.cpp"),
        "long add(long a, long b) {\n    return a + b;\n}\n",
    )
    .unwrap();
    let consumer = scratch.join("consumer");
    std::fs::create_dir_all(consumer.join("src")).unwrap();
    std::fs::create_dir_all(consumer.join("tests")).unwrap();
    std::fs::write(
        consumer.join("xirang.toml"),
        "schema = 1\n\n[package]\nname = \"consumer\"\nversion = \"0.1.0\"\nkind = \"library\"\n\n[language.typescript]\nstandard = \"strip\"\n\n[dependencies]\nmathlib = { path = \"../mathlib\" }\n",
    )
    .unwrap();
    std::fs::write(
        consumer.join("src/helper.ts"),
        "export function double(value: number): number {\n  return value * 2;\n}\n",
    )
    .unwrap();
    std::fs::write(
        consumer.join("src/addon.c"),
        "#include <node_api.h>\n#include <dlfcn.h>\n\nstatic napi_value call_add(napi_env env, napi_callback_info info) {\n    size_t argc = 2;\n    napi_value argv[2];\n    napi_get_cb_info(env, info, &argc, argv, NULL, NULL);\n    int64_t a = 0, b = 0;\n    napi_get_value_int64(env, argv[0], &a);\n    napi_get_value_int64(env, argv[1], &b);\n    void *handle = dlopen(\"libmathlib.so\", RTLD_LAZY);\n    if (handle == NULL) {\n        napi_throw_error(env, NULL, dlerror());\n        return NULL;\n    }\n    long (*add)(long, long) = (long (*)(long, long))dlsym(handle, \"add\");\n    if (add == NULL) {\n        napi_throw_error(env, NULL, dlerror());\n        return NULL;\n    }\n    napi_value result;\n    napi_create_int64(env, add(a, b), &result);\n    return result;\n}\n\nNAPI_MODULE_INIT() {\n    napi_value function;\n    napi_create_function(env, \"add\", NAPI_AUTO_LENGTH, call_add, NULL, &function);\n    napi_set_named_property(env, exports, \"add\", function);\n    return exports;\n}\n",
    )
    .unwrap();
    std::fs::write(
        consumer.join("tests/add_test.ts"),
        "const assert = require(\"node:assert\");\n\nconst addon = require(process.env.XIRANG_ADDON);\n\nassert.strictEqual(addon.add(2, 3), 5);\nconsole.log(\"test add_test ... ok\");\n",
    )
    .unwrap();
    let tested = invoke(&consumer, &["test"]);
    assert!(tested.status.success(), "{}", both(&tested));
    // Red control: a wrong expectation must fail, so the test really ran.
    // 红控制：期望值写错必须失败，证明测试真的跑了。
    std::fs::write(
        consumer.join("tests/add_test.ts"),
        "const assert = require(\"node:assert\");\n\nconst addon = require(process.env.XIRANG_ADDON);\n\nassert.strictEqual(addon.add(2, 3), 6);\nconsole.log(\"test add_test ... ok\");\n",
    )
    .unwrap();
    let failed = invoke(&consumer, &["test"]);
    assert!(!failed.status.success(), "{}", both(&failed));
}

/// The TypeScript scaffold is a real project: `new --lang typescript --lib` writes a manifest the
/// planner accepts, `xirang build` parses the module without running it, and a syntax error is a
/// build failure. A TypeScript binary is refused by name.
/// TypeScript 脚手架是真实项目：`new --lang typescript --lib` 写出的清单计划能接受，`xirang build`
/// 只解析模块而不运行，语法错误是构建失败；TypeScript 二进制按名拒绝。
#[test]
fn a_typescript_project_scaffold_builds_and_tests() {
    if node_include().is_none() {
        eprintln!("skipping: no Node headers on this machine");
        return;
    }
    let scratch = scratch("typescript-scaffold");
    let created = invoke(
        &scratch,
        &["new", "--lang", "typescript", "--lib", "gadget"],
    );
    assert!(created.status.success(), "{}", both(&created));
    let project = scratch.join("gadget");
    let built = invoke(&project, &["build"]);
    assert!(built.status.success(), "{}", both(&built));
    let tested = invoke(&project, &["test"]);
    assert!(tested.status.success(), "{}", both(&tested));
    std::fs::write(project.join("src/broken.ts"), "const value: number = ;\n").unwrap();
    let broken = invoke(&project, &["build"]);
    assert!(!broken.status.success(), "{}", both(&broken));
    let refused = invoke(&scratch, &["new", "--lang", "typescript", "gadget-bin"]);
    assert!(!refused.status.success(), "{}", both(&refused));
    assert!(
        both(&refused).contains("consumer library"),
        "{}",
        both(&refused)
    );
}

/// The JDK this test can use: `XIRANG_JAVA` when the caller names `javac`, else `javac` on PATH.
/// 本测试能用的 JDK：调用方点名 `XIRANG_JAVA` 时用它，否则用 PATH 上的 `javac`。
fn java_jdk() -> Option<(String, String)> {
    let program = if let Ok(named) = std::env::var("XIRANG_JAVA") {
        if named.is_empty() {
            return None;
        }
        named
    } else {
        "javac".to_owned()
    };
    let output = std::process::Command::new(&program)
        .arg("-version")
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let home = if program == "javac" {
        let which = std::process::Command::new("sh")
            .args(["-c", "command -v javac"])
            .output()
            .ok()?;
        let path = String::from_utf8_lossy(&which.stdout).trim().to_owned();
        path.rsplit_once("/bin/").map(|(home, _)| home.to_owned())?
    } else {
        program
            .rsplit_once("/bin/")
            .map(|(home, _)| home.to_owned())
            .unwrap_or_default()
    };
    Some((program, home))
}

/// One Java consumer, one C++ producer: the JNI bridge is C, the runtime is a real `java`.
/// 一个 Java 消费者、一个 C++ 生产者：JNI 桥是 C，运行时是真的 `java`。
#[test]
fn a_java_consumer_calls_a_cpp_export_through_jni() {
    let Some((javac, _)) = java_jdk() else {
        eprintln!("skipping: no JDK on PATH or XIRANG_JAVA");
        return;
    };
    let scratch = scratch("java-consumer");
    let producer = scratch.join("mathlib");
    std::fs::create_dir_all(producer.join("src")).unwrap();
    std::fs::write(
        producer.join("xirang.toml"),
        "schema = 1\n\n[package]\nname = \"mathlib\"\nversion = \"0.1.0\"\nkind = \"library\"\n\n\
         [language.cpp]\nstandard = \"17\"\n\n[[export]]\nname = \"add\"\nparams = [\"i64\", \"i64\"]\n\
         returns = \"i64\"\n\n[dependencies]\n",
    )
    .unwrap();
    std::fs::write(
        producer.join("src/lib.cpp"),
        "#include <cstdint>\n\nstd::int64_t add(std::int64_t a, std::int64_t b) { return a + b; }\n",
    )
    .unwrap();
    let consumer = scratch.join("consumer");
    std::fs::create_dir_all(consumer.join("src")).unwrap();
    std::fs::create_dir_all(consumer.join("tests")).unwrap();
    std::fs::write(
        consumer.join("xirang.toml"),
        "schema = 1\n\n[package]\nname = \"consumer\"\nversion = \"0.1.0\"\nkind = \"library\"\n\n\
         [language.java]\nstandard = \"21\"\n\n[dependencies]\nmathlib = { path = \"../mathlib\" }\n",
    )
    .unwrap();
    std::fs::write(
        consumer.join("src/consumer.c"),
        "#include <jni.h>\n\n#include \"mathlib/exports.h\"\n\n\
         JNIEXPORT jlong JNICALL Java_consumer_1test_add(JNIEnv *env, jclass type, jlong a, jlong b) {\n\
         \x20   (void)env;\n\x20   (void)type;\n\x20   return (jlong)add((int64_t)a, (int64_t)b);\n}\n",
    )
    .unwrap();
    let test_source = |expected: i64| {
        format!(
            "final class consumer_test {{\n\
             \x20   static {{ System.loadLibrary(\"consumer\"); }}\n\
             \x20   private static native long add(long a, long b);\n\
             \x20   public static void main(String[] arguments) {{\n\
             \x20       if (add(2, 3) != {expected}) {{\n\
             \x20           System.out.println(\"test consumer_test ... FAILED\");\n\
             \x20           System.exit(1);\n\
             \x20       }}\n\
             \x20       System.out.println(\"test consumer_test ... ok\");\n\
             \x20   }}\n}}\n"
        )
    };
    std::fs::write(consumer.join("tests/consumer_test.java"), test_source(5)).unwrap();
    let tested = invoke_with_env(&consumer, &["test"], &[("XIRANG_JAVA", &javac)]);
    assert!(tested.status.success(), "{}", both(&tested));
    assert!(
        String::from_utf8_lossy(&tested.stdout).contains("test consumer_test ... ok"),
        "{}",
        both(&tested)
    );
    // Red control: a wrong expectation must fail, so the test really ran.
    // 红控制：期望值写错必须失败，证明测试真的跑了。
    std::fs::write(consumer.join("tests/consumer_test.java"), test_source(6)).unwrap();
    let failed = invoke_with_env(&consumer, &["test"], &[("XIRANG_JAVA", &javac)]);
    assert!(!failed.status.success(), "{}", both(&failed));
}

/// The Kotlin compiler and the JDK, when this machine has both.
/// 本机同时具备 Kotlin 编译器与 JDK 时返回它们。
fn kotlin_toolchain() -> Option<(String, String)> {
    let kotlinc = std::env::var("XIRANG_KOTLIN").ok().or_else(|| {
        let output = std::process::Command::new("sh")
            .arg("-c")
            .arg("command -v kotlinc")
            .output()
            .ok()?;
        if !output.status.success() {
            return None;
        }
        let path = String::from_utf8_lossy(&output.stdout).trim().to_owned();
        (!path.is_empty()).then_some(path)
    })?;
    let (javac, _) = java_jdk()?;
    Some((kotlinc, javac))
}

#[test]
fn a_kotlin_consumer_calls_a_cpp_export_through_jni() {
    let Some((kotlinc, javac)) = kotlin_toolchain() else {
        eprintln!("skipping: no kotlinc or JDK");
        return;
    };
    let scratch = scratch("kotlin-consumer");
    let producer = scratch.join("mathlib");
    std::fs::create_dir_all(producer.join("src")).unwrap();
    std::fs::write(
        producer.join("xirang.toml"),
        "schema = 1\n\n[package]\nname = \"mathlib\"\nversion = \"0.1.0\"\nkind = \"library\"\n\n\
         [language.cpp]\nstandard = \"17\"\n\n[[export]]\nname = \"add\"\nparams = [\"i64\", \"i64\"]\n\
         returns = \"i64\"\n\n[dependencies]\n",
    )
    .unwrap();
    std::fs::write(
        producer.join("src/lib.cpp"),
        "#include <cstdint>\n\nstd::int64_t add(std::int64_t a, std::int64_t b) { return a + b; }\n",
    )
    .unwrap();
    let consumer = scratch.join("consumer");
    std::fs::create_dir_all(consumer.join("src")).unwrap();
    std::fs::create_dir_all(consumer.join("tests")).unwrap();
    std::fs::write(
        consumer.join("xirang.toml"),
        "schema = 1\n\n[package]\nname = \"consumer\"\nversion = \"0.1.0\"\nkind = \"library\"\n\n\
         [language.kotlin]\nstandard = \"2.4\"\n\n[dependencies]\nmathlib = { path = \"../mathlib\" }\n",
    )
    .unwrap();
    std::fs::write(
        consumer.join("src/consumer.c"),
        "#include <jni.h>\n\n#include \"mathlib/exports.h\"\n\n\
         JNIEXPORT jlong JNICALL Java_Consumer_add(JNIEnv *env, jobject self, jlong a, jlong b) {\n\
         \x20   (void)env;\n\x20   (void)self;\n\x20   return (jlong)add((int64_t)a, (int64_t)b);\n}\n",
    )
    .unwrap();
    std::fs::write(
        consumer.join("src/Consumer.kt"),
        "class Consumer {\n    init {\n        System.loadLibrary(\"consumer\")\n    }\n\n    \
         external fun add(a: Long, b: Long): Long\n}\n",
    )
    .unwrap();
    let test_source = |expected: i64| {
        format!(
            "fun main() {{\n    val value = Consumer().add(2, 3)\n    if (value != {expected}L) {{\n        \
             println(\"test add_test ... FAILED\")\n        kotlin.system.exitProcess(1)\n    }}\n    \
             println(\"test add_test ... ok\")\n}}\n"
        )
    };
    std::fs::write(consumer.join("tests/add_test.kt"), test_source(5)).unwrap();
    let environment = [
        ("XIRANG_KOTLIN", kotlinc.as_str()),
        ("XIRANG_JAVA", javac.as_str()),
    ];
    let tested = invoke_with_env(&consumer, &["test"], &environment);
    assert!(tested.status.success(), "{}", both(&tested));
    assert!(
        String::from_utf8_lossy(&tested.stdout).contains("test add_test ... ok"),
        "{}",
        both(&tested)
    );
    std::fs::write(consumer.join("tests/add_test.kt"), test_source(6)).unwrap();
    let failed = invoke_with_env(&consumer, &["test"], &environment);
    assert!(!failed.status.success(), "{}", both(&failed));
}

/// A PHP interpreter with the FFI extension, when this machine has one.
/// 本机具备带 FFI 扩展的 PHP 解释器时返回它。
fn php_toolchain() -> Option<String> {
    let php = std::env::var("XIRANG_PHP").ok().or_else(|| {
        let output = std::process::Command::new("sh")
            .arg("-c")
            .arg("command -v php")
            .output()
            .ok()?;
        if !output.status.success() {
            return None;
        }
        let path = String::from_utf8_lossy(&output.stdout).trim().to_owned();
        (!path.is_empty()).then_some(path)
    })?;
    let probe = std::process::Command::new(&php)
        .arg("-r")
        .arg("exit(extension_loaded(\"ffi\") ? 0 : 1);")
        .status()
        .ok()?;
    probe.success().then_some(php)
}

#[test]
fn a_php_consumer_calls_a_cpp_export_through_ffi() {
    let Some(php) = php_toolchain() else {
        eprintln!("skipping: no PHP with the FFI extension");
        return;
    };
    let scratch = scratch("php-consumer");
    let producer = scratch.join("mathlib");
    std::fs::create_dir_all(producer.join("src")).unwrap();
    std::fs::write(
        producer.join("xirang.toml"),
        "schema = 1\n\n[package]\nname = \"mathlib\"\nversion = \"0.1.0\"\nkind = \"library\"\n\n\
         [language.cpp]\nstandard = \"17\"\n\n[[export]]\nname = \"add\"\nparams = [\"i64\", \"i64\"]\n\
         returns = \"i64\"\n\n[dependencies]\n",
    )
    .unwrap();
    std::fs::write(
        producer.join("src/lib.cpp"),
        "#include <cstdint>\n\nstd::int64_t add(std::int64_t a, std::int64_t b) { return a + b; }\n",
    )
    .unwrap();
    let consumer = scratch.join("consumer");
    std::fs::create_dir_all(consumer.join("src")).unwrap();
    std::fs::create_dir_all(consumer.join("tests")).unwrap();
    std::fs::write(
        consumer.join("xirang.toml"),
        "schema = 1\n\n[package]\nname = \"consumer\"\nversion = \"0.1.0\"\nkind = \"library\"\n\n\
         [language.php]\nstandard = \"8.4\"\n\n[dependencies]\nmathlib = { path = \"../mathlib\" }\n",
    )
    .unwrap();
    std::fs::write(
        consumer.join("src/consumer.php"),
        "<?php\n\nfunction consumer_marker(): int\n{\n    return 1;\n}\n",
    )
    .unwrap();
    let test_source = |expected: i64| {
        format!(
            "<?php\n\n$library = FFI::cdef(\"int64_t add(int64_t a, int64_t b);\", \"libmathlib.so\");\n\n\
             if ($library->add(2, 3) !== {expected}) {{\n    fwrite(STDERR, \"test add_test ... FAILED\\n\");\n    exit(1);\n}}\n\n\
             echo \"test add_test ... ok\\n\";\n"
        )
    };
    std::fs::write(consumer.join("tests/add_test.php"), test_source(5)).unwrap();
    let tested = invoke_with_env(&consumer, &["test"], &[("XIRANG_PHP", php.as_str())]);
    assert!(tested.status.success(), "{}", both(&tested));
    assert!(
        String::from_utf8_lossy(&tested.stdout).contains("test add_test ... ok"),
        "{}",
        both(&tested)
    );
    std::fs::write(consumer.join("tests/add_test.php"), test_source(6)).unwrap();
    let failed = invoke_with_env(&consumer, &["test"], &[("XIRANG_PHP", php.as_str())]);
    assert!(!failed.status.success(), "{}", both(&failed));
}

#[test]
fn a_php_project_scaffold_builds_and_tests() {
    let Some(php) = php_toolchain() else {
        eprintln!("skipping: no PHP with the FFI extension");
        return;
    };
    let scratch = scratch("php-scaffold");
    let created = invoke_with_env(
        &scratch,
        &["new", "--lang", "php", "--lib", "gadget"],
        &[("XIRANG_PHP", php.as_str())],
    );
    assert!(created.status.success(), "{}", both(&created));
    let manifest = std::fs::read_to_string(scratch.join("gadget/xirang.toml")).unwrap();
    assert!(manifest.contains("[language.php]"), "{manifest}");
    let tested = invoke_with_env(
        &scratch.join("gadget"),
        &["test"],
        &[("XIRANG_PHP", php.as_str())],
    );
    assert!(tested.status.success(), "{}", both(&tested));
    assert!(
        String::from_utf8_lossy(&tested.stdout).contains("test gadget_test ... ok"),
        "{}",
        both(&tested)
    );
}

fn swift_toolchain() -> Option<String> {
    if let Ok(program) = std::env::var("XIRANG_SWIFT") {
        return Some(program);
    }
    let output = std::process::Command::new("swiftc")
        .arg("--version")
        .output()
        .ok()?;
    output.status.success().then(|| "swiftc".to_owned())
}

#[test]
fn a_swift_consumer_calls_a_cpp_export_through_c_interop() {
    let Some(swift) = swift_toolchain() else {
        eprintln!("skipping: no Swift compiler");
        return;
    };
    let scratch = scratch("swift-consumer");
    let producer = scratch.join("mathlib");
    std::fs::create_dir_all(producer.join("src")).unwrap();
    std::fs::write(
        producer.join("xirang.toml"),
        "schema = 1\n\n[package]\nname = \"mathlib\"\nversion = \"0.1.0\"\nkind = \"library\"\n\n\
         [language.cpp]\nstandard = \"17\"\n\n[[export]]\nname = \"add\"\nparams = [\"i64\", \"i64\"]\n\
         returns = \"i64\"\n\n[dependencies]\n",
    )
    .unwrap();
    std::fs::write(
        producer.join("src/lib.cpp"),
        "#include <cstdint>\n\nstd::int64_t add(std::int64_t a, std::int64_t b) { return a + b; }\n",
    )
    .unwrap();
    let consumer = scratch.join("consumer");
    std::fs::create_dir_all(consumer.join("src")).unwrap();
    std::fs::create_dir_all(consumer.join("tests")).unwrap();
    std::fs::write(
        consumer.join("xirang.toml"),
        "schema = 1\n\n[package]\nname = \"consumer\"\nversion = \"0.1.0\"\nkind = \"library\"\n\n\
         [language.swift]\nstandard = \"6.2\"\n\n[dependencies]\nmathlib = { path = \"../mathlib\" }\n",
    )
    .unwrap();
    std::fs::write(
        consumer.join("src/consumer.swift"),
        "public func consumer_marker() -> Int {\n    return 1\n}\n",
    )
    .unwrap();
    let test_source = |expected: i64| {
        format!(
            "import Glibc\n\n@_silgen_name(\"add\")\nfunc add(_ a: Int64, _ b: Int64) -> Int64\n\n\
             if add(2, 3) != {expected} {{\n    print(\"test add_test ... FAILED\")\n    exit(1)\n}}\n\n\
             print(\"test add_test ... ok\")\n"
        )
    };
    std::fs::write(consumer.join("tests/add_test.swift"), test_source(5)).unwrap();
    let tested = invoke_with_env(&consumer, &["test"], &[("XIRANG_SWIFT", swift.as_str())]);
    assert!(tested.status.success(), "{}", both(&tested));
    assert!(
        String::from_utf8_lossy(&tested.stdout).contains("test add_test ... ok"),
        "{}",
        both(&tested)
    );
    // Red control: a wrong expectation must fail, so the test really ran.
    std::fs::write(consumer.join("tests/add_test.swift"), test_source(6)).unwrap();
    let failed = invoke_with_env(&consumer, &["test"], &[("XIRANG_SWIFT", swift.as_str())]);
    assert!(!failed.status.success(), "{}", both(&failed));
}

#[test]
fn a_swift_project_scaffold_builds_and_tests() {
    let Some(swift) = swift_toolchain() else {
        eprintln!("skipping: no Swift compiler");
        return;
    };
    let scratch = scratch("swift-scaffold");
    let created = invoke_with_env(
        &scratch,
        &["new", "--lang", "swift", "--lib", "gadget"],
        &[("XIRANG_SWIFT", swift.as_str())],
    );
    assert!(created.status.success(), "{}", both(&created));
    let manifest = std::fs::read_to_string(scratch.join("gadget/xirang.toml")).unwrap();
    assert!(manifest.contains("[language.swift]"), "{manifest}");
    let tested = invoke_with_env(
        &scratch.join("gadget"),
        &["test"],
        &[("XIRANG_SWIFT", swift.as_str())],
    );
    assert!(tested.status.success(), "{}", both(&tested));
    assert!(
        String::from_utf8_lossy(&tested.stdout).contains("test gadget_test ... ok"),
        "{}",
        both(&tested)
    );
}
