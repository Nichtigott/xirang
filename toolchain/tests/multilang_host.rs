//! A Rust host crate that consumes a multilingual C++ producer through its `build.rs`, across a
//! `xirang move` of a dependency. This is the joint case: one transaction moves a producer unit,
//! and the Rust consumer keeps building, linking and running the delivered shared library.
//! 一个 Rust 宿主 crate 通过 `build.rs` 消费多语言 C++ 生产者，并在依赖被 `xirang move` 搬动之后
//! 继续构建、链接、运行交付的动态库。这是联合场景：一次事务搬动生产者单元，Rust 消费者仍然工作。

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// The xirang binary this test package built.
/// 本测试包构建出的 xirang 二进制。
fn binary() -> &'static str {
    env!("CARGO_BIN_EXE_xirang")
}

/// A fresh scratch directory.
/// 一个新的临时目录。
fn scratch(label: &str) -> PathBuf {
    let base = std::env::temp_dir().join(format!("xirang-host-{label}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    std::fs::create_dir_all(&base).expect("scratch");
    base
}

/// Write one file, creating its parent directory.
/// 写一个文件，必要时建父目录。
fn write_file(path: &Path, text: &str) {
    std::fs::create_dir_all(path.parent().expect("parent")).expect("directory");
    std::fs::write(path, text).expect("write");
}

/// The producer's root manifest: an export that calls into the `math` dependency.
/// 生产者的根清单：一个会调用 `math` 依赖的导出。
const APP_MANIFEST: &str = r#"schema = 1

[package]
name = "app"
version = "0.1.0"
kind = "library"

[language.cpp]
standard = "17"

[[export]]
name = "add"
params = ["i64", "i64"]
returns = "i64"

[dependencies]
math = { path = "math" }
"#;

/// The `math` dependency's manifest.
/// `math` 依赖的清单。
const MATH_MANIFEST: &str = r#"schema = 1

[package]
name = "math"
version = "0.1.0"
kind = "library"

[language.cpp]
standard = "17"

[dependencies]
"#;

/// The host's build script: ask the toolchain to build and package the producer, then link the
/// delivered shared library. It re-runs when the producer tree changes.
/// 宿主的构建脚本：让工具链构建并打包生产者，然后链接交付的动态库；生产者树变化时重跑。
const HOST_BUILD: &str = r#"use std::path::Path;
use std::process::Command;

fn find(directory: &Path, extension: &str) -> Option<std::path::PathBuf> {
    for entry in std::fs::read_dir(directory).expect("read package") {
        let entry = entry.expect("entry");
        let path = entry.path();
        if path.is_dir() {
            if let Some(found) = find(&path, extension) {
                return Some(found);
            }
        } else if path.extension().and_then(|value| value.to_str()) == Some(extension) {
            return Some(path);
        }
    }
    None
}

fn main() {
    let xirang = std::env::var("XIRANG_BIN").expect("XIRANG_BIN must name the xirang binary");
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let producer = root.join("../producer");
    let packaged = root.join("target/xirang-package");
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed={}", producer.display());
    println!("cargo:rerun-if-env-changed=XIRANG_BIN");
    let _ = std::fs::remove_dir_all(&packaged);
    for action in ["build", "package"] {
        let mut command = Command::new(&xirang);
        command.arg(action).current_dir(&producer);
        if action == "package" {
            command.arg("--output").arg(&packaged);
        }
        let status = command.status().expect("run xirang");
        assert!(status.success(), "xirang {action} failed");
    }
    let library = find(&packaged, "so").expect("the package holds no shared library");
    let directory = library.parent().expect("parent");
    let name = library.file_name().unwrap().to_str().unwrap();
    let name = name.strip_prefix("lib").unwrap_or(name);
    let name = name.strip_suffix(".so").unwrap_or(name);
    println!("cargo:rustc-link-search=native={}", directory.display());
    println!("cargo:rustc-link-lib=dylib={name}");
    println!("cargo:rustc-link-arg=-Wl,-rpath,{}", directory.display());
}
"#;

/// The host library: a C ABI declaration and one test that calls it.
/// 宿主库：一条 C ABI 声明和一条调用它的测试。
const HOST_LIB: &str = concat!(
    "unsafe extern \"C\" {\n",
    "    fn add(a: i64, b: i64) -> i64;\n",
    "}\n",
    "\n",
    "#[te",
    "st]\n",
    "fn calls_the_cpp_export() {\n",
    "    assert_eq!(unsafe { add(2, 3) }, 5);\n",
    "}\n",
);

/// The host's crate manifest (no dependencies: it compiles offline).
/// 宿主的 crate 清单（无依赖，可离线编译）。
const HOST_MANIFEST: &str = r#"[package]
name = "host"
version = "0.1.0"
edition = "2021"

[lib]
path = "src/lib.rs"
"#;

/// Run the host's tests with the toolchain binary on its path.
/// 带上工具链二进制路径跑宿主测试。
fn cargo_test(host: &Path) -> Output {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned());
    Command::new(cargo)
        .args(["test", "--offline", "--quiet"])
        .current_dir(host)
        .env("XIRANG_BIN", binary())
        .env("CARGO_TARGET_DIR", host.join("target"))
        .output()
        .expect("cargo test")
}

/// The bytes of every file under one directory, keyed by relative path.
/// 一个目录下每个文件的字节，按相对路径索引。
fn snapshot(directory: &Path) -> Vec<(PathBuf, Vec<u8>)> {
    fn walk_files(directory: &Path, base: &Path, files: &mut Vec<(PathBuf, Vec<u8>)>) {
        for entry in std::fs::read_dir(directory).expect("read") {
            let entry = entry.expect("entry");
            let path = entry.path();
            if path.is_dir() {
                walk_files(&path, base, files);
            } else {
                files.push((
                    path.strip_prefix(base).unwrap().to_path_buf(),
                    std::fs::read(&path).expect("bytes"),
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
fn a_rust_host_links_a_multilingual_producer_across_a_move() {
    let root = scratch("joint");
    let producer = root.join("producer");
    write_file(&producer.join("xirang.toml"), APP_MANIFEST);
    write_file(
        &producer.join("src/app.cpp"),
        "#include <cstdint>\n\nstd::int64_t base();\n\nstd::int64_t add(std::int64_t a, std::int64_t b) {\n    return a + b + base() - 2;\n}\n",
    );
    write_file(&producer.join("math/xirang.toml"), MATH_MANIFEST);
    write_file(
        &producer.join("math/src/math.cpp"),
        "#include <cstdint>\n\nstd::int64_t base() {\n    return 2;\n}\n",
    );
    let host = root.join("host");
    write_file(&host.join("Cargo.toml"), HOST_MANIFEST);
    write_file(&host.join("build.rs"), HOST_BUILD);
    write_file(&host.join("src/lib.rs"), HOST_LIB);
    std::fs::create_dir_all(host.join("target")).expect("target");

    let first = cargo_test(&host);
    assert!(
        first.status.success(),
        "the host did not pass before the move:\n{}\n{}",
        String::from_utf8_lossy(&first.stdout),
        String::from_utf8_lossy(&first.stderr)
    );

    let host_bytes = snapshot(&host.join("src"));
    let math_bytes = snapshot(&producer.join("math/src"));
    std::fs::create_dir_all(producer.join("vendor")).expect("vendor");
    let moved = Command::new(binary())
        .args(["move", "math", "vendor/math"])
        .current_dir(&producer)
        .output()
        .expect("move");
    assert!(
        moved.status.success(),
        "the move failed:\n{}",
        String::from_utf8_lossy(&moved.stderr)
    );
    let manifest = std::fs::read_to_string(producer.join("xirang.toml")).expect("manifest");
    assert!(
        manifest.contains("path = \"vendor/math\""),
        "the move did not rewrite the dependency line:\n{manifest}"
    );
    assert_eq!(
        snapshot(&producer.join("vendor/math/src")),
        math_bytes,
        "the move changed the moved unit's own sources"
    );
    assert_eq!(
        snapshot(&host.join("src")),
        host_bytes,
        "the move reached into the third-party Rust host"
    );

    let second = cargo_test(&host);
    assert!(
        second.status.success(),
        "the host stopped passing after the move:\n{}\n{}",
        String::from_utf8_lossy(&second.stdout),
        String::from_utf8_lossy(&second.stderr)
    );

    // Red control: change the moved dependency so the export's answer changes. The consumer must
    // now fail, which proves the host really links what the producer delivers and that the delivered
    // shared library carries the dependency's code rather than a dangling reference.
    // 红控制：改掉被搬动的依赖使导出结果变化。消费者必须失败——证明宿主确实链接生产者交付的东西，
    // 且交付的动态库带着依赖的代码而不是一个悬空引用。
    write_file(
        &producer.join("vendor/math/src/math.cpp"),
        "#include <cstdint>\n\nstd::int64_t base() {\n    return 3;\n}\n",
    );
    let third = cargo_test(&host);
    assert!(
        !third.status.success(),
        "the host still passed after the dependency changed; the chain is not live"
    );
}
