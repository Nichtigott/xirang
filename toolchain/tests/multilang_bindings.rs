//! `xirang bindings`: the starter a consumer pastes. These tests build a real producer, render the
//! Python starter, and call the export through it — a generator that only emits text would pass a
//! string test and still hand the user a broken call.
//! `xirang bindings`：消费端可粘贴的起点。这里构建真实生产者、渲染 Python 起点、并通过它调用导出
//! —— 只断言字符串会漏掉“生成的调用根本跑不起来”。

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn scratch(label: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "xirang-bindings-{label}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&path).unwrap();
    path
}

fn xirang(directory: &Path, arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_xirang"))
        .args(arguments)
        .current_dir(directory)
        .output()
        .unwrap()
}

fn text(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn producer(label: &str, returns: &str, parameters: &str) -> PathBuf {
    let root = scratch(label);
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::write(
        root.join("xirang.toml"),
        format!(
            "schema = 1\n\n[package]\nname = \"mathlib\"\nversion = \"0.1.0\"\nkind = \"library\"\n\n[language.cpp]\nstandard = \"17\"\n\n[[export]]\nname = \"add\"\nparams = [{parameters}]\nreturns = \"{returns}\"\n\n[dependencies]\n"
        ),
    )
    .unwrap();
    std::fs::write(
        root.join("src/mathlib.cpp"),
        "#include <cstdint>\n#include <string>\n\nstd::int64_t add(std::int64_t a, std::int64_t b) { return a + b; }\nstd::string greet() { return \"hi\"; }\n",
    )
    .unwrap();
    root
}

/// The directory holding the built shared library of `unit`.
fn export_directory(root: &Path, unit: &str) -> PathBuf {
    let builds = root.join("target/xirang/build");
    let mut newest: Option<(std::time::SystemTime, PathBuf)> = None;
    for entry in std::fs::read_dir(&builds).unwrap() {
        let entry = entry.unwrap();
        let directory = entry.path().join("export").join(unit);
        if !directory.is_dir() {
            continue;
        }
        let time = std::fs::metadata(&directory).unwrap().modified().unwrap();
        if newest.as_ref().is_none_or(|(seen, _)| time > *seen) {
            newest = Some((time, directory));
        }
    }
    newest.expect("the build wrote an export directory").1
}

#[test]
fn a_generated_python_binding_calls_the_export_it_names() {
    let root = producer("python", "i64", "\"i64\", \"i64\"");
    let build = xirang(&root, &["build"]);
    assert!(build.status.success(), "{}", text(&build));
    let rendered = xirang(&root, &["bindings", "--language", "python"]);
    assert!(rendered.status.success(), "{}", text(&rendered));
    let starter = String::from_utf8_lossy(&rendered.stdout).into_owned();
    assert!(
        starter.contains("ctypes.CDLL(\"libmathlib.so\")"),
        "{starter}"
    );
    let driver = root.join("driver.py");
    std::fs::write(&driver, format!("{starter}\nprint(add(2, 3))\n")).unwrap();
    let export = export_directory(&root, "mathlib");
    let run = Command::new("python3")
        .arg(&driver)
        .env("LD_LIBRARY_PATH", &export)
        .output()
        .unwrap();
    assert!(run.status.success(), "{}", text(&run));
    assert_eq!(String::from_utf8_lossy(&run.stdout).trim(), "5");
}

#[test]
fn the_csharp_and_go_starters_name_the_c_signature() {
    let root = producer("other", "i64", "\"i64\", \"i64\"");
    let csharp = xirang(&root, &["bindings", "--language", "csharp"]);
    assert!(csharp.status.success(), "{}", text(&csharp));
    let csharp = String::from_utf8_lossy(&csharp.stdout).into_owned();
    assert!(
        csharp.contains("[DllImport(\"libmathlib.so\")]"),
        "{csharp}"
    );
    assert!(
        csharp.contains("internal static extern long add(long p0, long p1);"),
        "{csharp}"
    );
    let go = xirang(&root, &["bindings", "--language", "go"]);
    assert!(go.status.success(), "{}", text(&go));
    let go = String::from_utf8_lossy(&go.stdout).into_owned();
    assert!(go.contains("#cgo LDFLAGS: -lmathlib"), "{go}");
    assert!(go.contains("int64_t add(int64_t p0, int64_t p1);"), "{go}");
    assert!(go.contains("func Add(p0 int64, p1 int64) int64 {"), "{go}");
    assert!(go.contains("CGO_LDFLAGS=-L"), "{go}");
}

#[test]
fn the_generator_refuses_what_it_cannot_emit_honestly() {
    let root = producer("refuse", "i64", "\"i64\", \"i64\"");
    let unknown = xirang(&root, &["bindings", "--language", "fortran"]);
    assert!(!unknown.status.success());
    let message = text(&unknown);
    assert!(
        message.contains("python")
            && message.contains("csharp")
            && message.contains("go")
            && message.contains("rust")
            && message.contains("ruby"),
        "{message}"
    );

    let text_return = producer("text", "text", "\"utf8\"");
    let refused = xirang(&text_return, &["bindings", "--language", "python"]);
    assert!(!refused.status.success());
    assert!(
        text(&refused).contains("text or bytes"),
        "{}",
        text(&refused)
    );

    let plain = scratch("plain");
    std::fs::create_dir_all(plain.join("src")).unwrap();
    std::fs::write(
        plain.join("xirang.toml"),
        "schema = 1\n\n[package]\nname = \"plain\"\nversion = \"0.1.0\"\nkind = \"library\"\n\n[language.cpp]\nstandard = \"17\"\n\n[dependencies]\n",
    )
    .unwrap();
    std::fs::write(plain.join("src/plain.cpp"), "int plain() { return 1; }\n").unwrap();
    let none = xirang(&plain, &["bindings", "--language", "python"]);
    assert!(!none.status.success());
    assert!(
        text(&none).contains("declares no `[[export]]`"),
        "{}",
        text(&none)
    );
}

#[test]
fn a_generated_rust_binding_links_and_calls_the_export() {
    let root = producer("rust", "i64", "\"i64\", \"i64\"");
    let build = xirang(&root, &["build"]);
    assert!(build.status.success(), "{}", text(&build));
    let rendered = xirang(&root, &["bindings", "--language", "rust"]);
    assert!(rendered.status.success(), "{}", text(&rendered));
    let starter = String::from_utf8_lossy(&rendered.stdout).into_owned();
    assert!(
        starter.contains("pub fn add(p0: i64, p1: i64) -> i64;"),
        "{starter}"
    );
    std::fs::write(root.join("bindings.rs"), &starter).unwrap();
    std::fs::write(
        root.join("consumer.rs"),
        "include!(\"bindings.rs\");\n\n#[test]\nfn calls_the_export() {\n    assert_eq!(unsafe { add(2, 3) }, 5);\n}\n",
    )
    .unwrap();
    let export = export_directory(&root, "mathlib");
    let compiled = Command::new(std::env::var("RUSTC").unwrap_or_else(|_| "rustc".to_owned()))
        .args([
            "--edition",
            "2021",
            "--test",
            "consumer.rs",
            "-o",
            "consumer",
        ])
        .arg(format!("-L{}", export.display()))
        .args(["-l", "mathlib"])
        .arg(format!("-Clink-arg=-Wl,-rpath,{}", export.display()))
        .current_dir(&root)
        .output()
        .unwrap();
    assert!(compiled.status.success(), "{}", text(&compiled));
    let run = Command::new(root.join("consumer"))
        .current_dir(&root)
        .output()
        .unwrap();
    assert!(run.status.success(), "{}", text(&run));
}

#[test]
fn a_generated_ruby_binding_calls_the_export_through_fiddle() {
    let root = producer("ruby", "i64", "\"i64\", \"i64\"");
    let build = xirang(&root, &["build"]);
    assert!(build.status.success(), "{}", text(&build));
    let rendered = xirang(&root, &["bindings", "--language", "ruby"]);
    assert!(rendered.status.success(), "{}", text(&rendered));
    let starter = String::from_utf8_lossy(&rendered.stdout).into_owned();
    assert!(starter.contains("module Mathlib"), "{starter}");
    assert!(starter.contains("Fiddle::TYPE_INT64_T"), "{starter}");
    std::fs::write(root.join("mathlib.rb"), &starter).unwrap();
    let export = export_directory(&root, "mathlib");
    let ruby = std::env::var("XIRANG_RUBY").unwrap_or_else(|_| "ruby".to_owned());
    std::fs::write(
        root.join("test.rb"),
        "require_relative \"mathlib\"\n\nraise \"add(2, 3) != 5\" unless Mathlib.add(2, 3) == 5\nputs \"ok ruby\"\n",
    )
    .unwrap();
    let Ok(run) = Command::new(&ruby)
        .arg("test.rb")
        .current_dir(&root)
        .env("LD_LIBRARY_PATH", &export)
        .output()
    else {
        eprintln!("skipping: no ruby on this machine");
        return;
    };
    assert!(run.status.success(), "{}", text(&run));
    assert!(String::from_utf8_lossy(&run.stdout).contains("ok ruby"));
    std::fs::write(
        root.join("wrong.rb"),
        "require_relative \"mathlib\"\n\nraise \"wrong expectation passed\" unless Mathlib.add(2, 3) == 6\n",
    )
    .unwrap();
    let wrong = Command::new(&ruby)
        .arg("wrong.rb")
        .current_dir(&root)
        .env("LD_LIBRARY_PATH", &export)
        .output()
        .unwrap();
    assert!(!wrong.status.success(), "a wrong expectation must fail");
}

/// A producer whose only export takes an optional parameter, so every starter below has to carry
/// the `present`-flag struct rather than a bare scalar.
/// 只有一个导出的生产者，该导出取一个可选参数，因此下面每个起点都必须承载 `present` 结构体而不是裸标量。
fn optional_producer(label: &str) -> PathBuf {
    let root = scratch(label);
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::write(
        root.join("xirang.toml"),
        "schema = 1\n\n[package]\nname = \"mathlib\"\nversion = \"0.1.0\"\nkind = \"library\"\n\n\
         [language.cpp]\nstandard = \"17\"\n\n\
         [[export]]\nname = \"pick\"\nparams = [\"optional<i64>\", \"i64\"]\nreturns = \"i64\"\n\n\
         [dependencies]\n",
    )
    .unwrap();
    std::fs::write(
        root.join("src/mathlib.cpp"),
        "#include <cstdint>\n#include <optional>\n\nstd::int64_t pick(std::optional<std::int64_t> value, std::int64_t fallback) {\n    return value.has_value() ? *value : fallback;\n}\n",
    )
    .unwrap();
    root
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

#[test]
fn an_optional_parameter_crosses_the_python_and_rust_starters() {
    let root = optional_producer("optional");
    let build = xirang(&root, &["build"]);
    assert!(build.status.success(), "{}", text(&build));
    let export = export_directory(&root, "mathlib");

    // Python passes a value or `None`, and both reach the C struct by value.
    // Python 传值或 `None`，两者都按值到达 C 结构体。
    let rendered = xirang(&root, &["bindings", "--language", "python"]);
    assert!(rendered.status.success(), "{}", text(&rendered));
    let starter = String::from_utf8_lossy(&rendered.stdout).into_owned();
    assert!(
        starter.contains("class xirang_optional_i64(ctypes.Structure):"),
        "{starter}"
    );
    assert!(starter.contains("def pick(p0, p1):"), "{starter}");
    let driver = root.join("driver.py");
    std::fs::write(
        &driver,
        format!("{starter}\nprint(pick(3, 9), pick(None, 9))\n"),
    )
    .unwrap();
    let run = Command::new("python3")
        .arg(&driver)
        .env("LD_LIBRARY_PATH", &export)
        .output()
        .unwrap();
    assert!(run.status.success(), "{}", text(&run));
    assert_eq!(String::from_utf8_lossy(&run.stdout).trim(), "3 9");
    // Red control: the `None` branch is really taken, not silently defaulted.
    // 红控制：`None` 那一支真的被走到，而不是被静默地取默认值。
    std::fs::write(
        &driver,
        format!("{starter}\nassert pick(None, 9) == 0\nprint(pick(3, 9), pick(None, 9))\n"),
    )
    .unwrap();
    let wrong = Command::new("python3")
        .arg(&driver)
        .env("LD_LIBRARY_PATH", &export)
        .output()
        .unwrap();
    assert!(!wrong.status.success(), "a wrong expectation must fail");

    // Rust passes the same struct by value; the starter declares it, so a caller can build one.
    // Rust 按值传同一个结构体；起点声明了它，因此调用方能构造出来。
    let rendered = xirang(&root, &["bindings", "--language", "rust"]);
    assert!(rendered.status.success(), "{}", text(&rendered));
    let starter = String::from_utf8_lossy(&rendered.stdout).into_owned();
    assert!(
        starter.contains("pub struct xirang_optional_i64 { pub present: bool, pub value: i64 }"),
        "{starter}"
    );
    std::fs::write(root.join("bindings.rs"), &starter).unwrap();
    std::fs::write(
        root.join("consumer.rs"),
        "include!(\"bindings.rs\");\n\n#[test]\nfn calls_the_export() {\n    let present = xirang_optional_i64 { present: true, value: 3 };\n    let absent = xirang_optional_i64 { present: false, value: 0 };\n    assert_eq!(unsafe { pick(present, 9) }, 3);\n    assert_eq!(unsafe { pick(absent, 9) }, 9);\n}\n",
    )
    .unwrap();
    let compiled = Command::new(std::env::var("RUSTC").unwrap_or_else(|_| "rustc".to_owned()))
        .args([
            "--edition",
            "2021",
            "--test",
            "consumer.rs",
            "-o",
            "consumer",
        ])
        .arg(format!("-L{}", export.display()))
        .args(["-l", "mathlib"])
        .arg(format!("-Clink-arg=-Wl,-rpath,{}", export.display()))
        .current_dir(&root)
        .output()
        .unwrap();
    assert!(compiled.status.success(), "{}", text(&compiled));
    let run = Command::new(root.join("consumer"))
        .current_dir(&root)
        .output()
        .unwrap();
    assert!(run.status.success(), "{}", text(&run));

    // The C# starter declares the struct with the one-byte flag the C header lays out.
    // C# 起点按 C 头文件的布局声明该结构体，布尔标志占一字节。
    let csharp = xirang(&root, &["bindings", "--language", "csharp"]);
    assert!(csharp.status.success(), "{}", text(&csharp));
    let csharp = String::from_utf8_lossy(&csharp.stdout).into_owned();
    assert!(
        csharp
            .contains("[StructLayout(LayoutKind.Sequential)]\ninternal struct xirang_optional_i64"),
        "{csharp}"
    );
    assert!(
        csharp.contains("internal static extern long pick(xirang_optional_i64 p0, long p1);"),
        "{csharp}"
    );

    // The Go starter keeps the struct by value and builds it through a C helper, because a cgo
    // struct literal cannot spell the `_Bool` field portably.
    // Go 起点按值保留结构体，并通过一个 C 助手构造它，因为 cgo 的结构体字面量无法可移植地写出
    // `_Bool` 字段。
    let go = xirang(&root, &["bindings", "--language", "go"]);
    assert!(go.status.success(), "{}", text(&go));
    let go = String::from_utf8_lossy(&go.stdout).into_owned();
    assert!(
        go.contains("typedef struct { bool present; int64_t value; } xirang_optional_i64;"),
        "{go}"
    );
    assert!(
        go.contains("func Pick(p0 XirangOptionalI64, p1 int64) int64 {"),
        "{go}"
    );
    // Generated text and cgo disagree exactly at a struct passed by value, so compile and run it
    // when a Go toolchain is available.
    // 生成文本与 cgo 恰好在“按值传结构体”处分歧，因此有 Go 工具链时真的编译并运行它。
    if let Some(tool) = go_toolchain() {
        let package = scratch("optional-go");
        std::fs::create_dir_all(package.join("mathlib")).unwrap();
        std::fs::create_dir_all(package.join("check")).unwrap();
        std::fs::write(package.join("go.mod"), "module mathlibcheck\n\ngo 1.21\n").unwrap();
        std::fs::write(package.join("mathlib/bindings.go"), &go).unwrap();
        std::fs::write(
            package.join("check/main.go"),
            "package main\n\nimport (\n    \"fmt\"\n\n    \"mathlibcheck/mathlib\"\n)\n\nfunc main() {\n    fmt.Println(mathlib.Pick(mathlib.SomeI64(3), 9), mathlib.Pick(mathlib.NoneI64(), 9))\n}\n",
        )
        .unwrap();
        let built = Command::new(&tool)
            .args(["build", "-o", "check-bin", "./check"])
            .current_dir(&package)
            .env("CGO_ENABLED", "1")
            .env("CGO_LDFLAGS", format!("-L{}", export.display()))
            .env("GOCACHE", package.join("gocache"))
            .output()
            .unwrap();
        assert!(built.status.success(), "{}", text(&built));
        let run = Command::new(package.join("check-bin"))
            .env("LD_LIBRARY_PATH", &export)
            .output()
            .unwrap();
        assert!(run.status.success(), "{}", text(&run));
        assert_eq!(String::from_utf8_lossy(&run.stdout).trim(), "3 9");
    } else {
        eprintln!("skipping the Go compile: no go toolchain on this machine");
    }
}

#[test]
fn the_ruby_starter_refuses_an_optional_parameter_by_name() {
    let root = optional_producer("optional-ruby");
    let refused = xirang(&root, &["bindings", "--language", "ruby"]);
    assert!(!refused.status.success(), "{}", text(&refused));
    let message = text(&refused);
    assert!(message.contains("takes an optional parameter"), "{message}");
    assert!(
        message.contains("Fiddle passes scalars and pointers only"),
        "{message}"
    );
    // A refusal must not leave half a starter on stdout for somebody to paste.
    // 拒绝不许在 stdout 上留半份可粘贴的起点。
    assert!(
        refused.stdout.is_empty(),
        "a refusal wrote text: {}",
        String::from_utf8_lossy(&refused.stdout)
    );
}
