//! covers `app/cargo_probe.rs`, `app/project_context.rs`.
//! 覆盖 `app/cargo_probe.rs`、`app/project_context.rs`。
//!
//! Project-resolution fallbacks and MIR target selection tests.
//! 项目解析回落与 MIR target 选择测试（covers `app/project_context.rs`、`app/cargo_probe.rs`）。

use super::*;

#[test]
fn an_unresolvable_project_is_refused_instead_of_falling_back() {
    let missing = std::env::temp_dir().join("xirang-toolchain-missing-project");

    let nothing = resolve_project_from(None, None, None, None, false)
        .expect_err("no project at all must be an error");
    assert!(nothing.contains("no project to open"), "{nothing}");
    assert!(
        nothing.contains("XIRANG_PACKAGE_ROOT"),
        "the message must say how to point Studio at a project: {nothing}"
    );

    let explicit = resolve_project_from(None, Some(&missing), None, None, false)
        .expect_err("a path argument that is not a directory must be refused");
    assert!(explicit.contains("path argument"), "{explicit}");
    assert!(
        explicit.contains("xirang-toolchain-missing-project"),
        "{explicit}"
    );

    let configured = resolve_project_from(None, None, Some(&missing), None, false)
        .expect_err("a configured root that is not a directory must be refused");
    assert!(configured.contains("XIRANG_PACKAGE_ROOT"), "{configured}");

    let selected = resolve_project_from(Some(&missing), None, None, None, false)
        .expect_err("a selected project that vanished must be refused");
    assert!(selected.contains("select_project"), "{selected}");
}

/// The working directory is the only implicit project, and only when it holds a
/// package: a directory without a manifest is not guessed at.
/// 当前目录是唯一的隐式项目，而且只在它持有包时成立；没有清单的目录不会被猜。
#[test]
fn only_a_working_directory_that_holds_a_package_is_opened() {
    let current = std::env::temp_dir();
    assert_eq!(
        resolve_project_from(None, None, None, Some(&current), true),
        Ok(current.clone())
    );
    assert!(
        resolve_project_from(None, None, None, Some(&current), false).is_err(),
        "a directory without a Cargo.toml is not a project"
    );
}

/// A relative candidate resolves against the working directory, so a relative
/// argument or environment value names the directory the shell would name.
/// 相对候选值相对当前目录解析，因此相对参数或相对环境变量指向 shell 会指向的目录。
#[test]
fn a_relative_candidate_resolves_against_the_working_directory() {
    let root = std::env::temp_dir();
    let child = root.join("xirang-toolchain-relative");
    std::fs::create_dir_all(&child).expect("fixture directory");
    assert_eq!(
        resolve_project_from(
            None,
            Some(std::path::Path::new("xirang-toolchain-relative")),
            None,
            Some(&root),
            false,
        ),
        Ok(child.clone())
    );
    let _ = std::fs::remove_dir_all(&child);
}

/// Build a throwaway package with the requested targets and run the MIR target
/// resolver against it, returning whether the selected target compiled.
/// 构建一个带有所请求 target 的一次性包，对其运行 MIR target 解析器，返回所选 target
/// 是否编译通过。
fn mir_target_resolves(label: &str, library: bool, binary: bool, second_binary: bool) -> bool {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let root = std::env::temp_dir().join(format!("xirang-toolchain-mir-{label}-{suffix}"));
    std::fs::create_dir_all(root.join("src")).expect("fixture src");
    std::fs::write(
        root.join("Cargo.toml"),
        format!(
            "[package]\nname = \"mir-{label}-host\"\nversion = \"0.1.0\"\nedition = \"2021\"\n"
        ),
    )
    .expect("manifest");
    if library {
        std::fs::write(root.join("src/lib.rs"), "pub fn placeholder() {}\n")
            .expect("library entry");
    }
    if binary {
        std::fs::write(root.join("src/main.rs"), "fn main() {}\n").expect("binary entry");
    }
    if second_binary {
        std::fs::create_dir_all(root.join("src/bin")).expect("bin directory");
        std::fs::write(root.join("src/bin/tool.rs"), "fn main() {}\n").expect("second binary");
    }
    // `--emit=metadata` is stable, so the pin is the target *selection*; the
    // production call site requests stable `--emit=mir`.
    // `--emit=metadata` 在 stable 上可用，因此这条钉子钉的是 target **选择**；生产调用点
    // 请求稳定版 `--emit=mir`。
    let output =
        cargo_rustc_mir(&root.join("Cargo.toml"), &["--emit=metadata"]).expect("cargo runs");
    let compiled = output.status.success();
    if !compiled {
        eprintln!("{label}: {}", String::from_utf8_lossy(&output.stderr));
    }
    let _ = std::fs::remove_dir_all(&root);
    compiled
}

/// A binary-only package — the default output of `xirang new` — resolves a
/// target for MIR inspection instead of failing on the hardcoded `--lib`.
/// 仅含二进制的包——`xirang new` 的默认产物——会为 MIR 检视解析出一个 target，而不是在
/// 硬编码的 `--lib` 上失败。
#[test]
fn mir_inspection_resolves_the_target_a_package_actually_has() {
    assert!(
        mir_target_resolves("bin", false, true, false),
        "a binary-only package must resolve its binary target"
    );
    assert!(
        mir_target_resolves("lib", true, false, false),
        "a library-only package must still resolve its library target"
    );
    assert!(
        mir_target_resolves("both", true, true, false),
        "a package with both targets must resolve one of them"
    );
}

/// A host with two binary targets is inspected too. `cargo rustc --bins` refuses to hand the extra
/// `rustc` arguments to several targets at once, so the resolver could not serve such a host at all
/// (audit `S14`).
/// 含两个二进制 target 的宿主也能被检视。`cargo rustc --bins` 拒绝把额外的 `rustc` 参数同时交给多个
/// target，因此解析器过去完全服务不了这种宿主（审计 `S14`）。
#[test]
fn mir_inspection_resolves_a_host_with_two_binary_targets() {
    assert!(
        mir_target_resolves("two-bins", false, true, true),
        "a two-bin package must resolve one of its binary targets"
    );
}

/// `cargo metadata` decides whether this package has a library, and a sibling workspace
/// member's binary is not this package's binary (audit `STU-S-25`).
/// 这个包有没有库由 `cargo metadata` 决定；同一工作区里兄弟成员的二进制不是这个包的二进制
/// （审计 `STU-S-25`）。
#[test]
fn metadata_decides_the_library_and_keeps_only_this_packages_bins() {
    let root =
        std::env::temp_dir().join(format!("xirang-toolchain-targets-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("app/src")).expect("app src");
    std::fs::create_dir_all(root.join("other/src")).expect("other src");
    std::fs::write(
        root.join("Cargo.toml"),
        "[workspace]\nmembers = [\"app\", \"other\"]\nresolver = \"2\"\n",
    )
    .expect("workspace manifest");
    std::fs::write(
        root.join("app/Cargo.toml"),
        "[package]\nname = \"app-host\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[lib]\npath = \"src/lib.rs\"\n\n[[bin]]\nname = \"app_tool\"\npath = \"src/main.rs\"\n",
    )
    .expect("app manifest");
    std::fs::write(root.join("app/src/lib.rs"), "pub fn placeholder() {}\n").expect("app lib");
    std::fs::write(root.join("app/src/main.rs"), "fn main() {}\n").expect("app bin");
    std::fs::write(
        root.join("other/Cargo.toml"),
        "[package]\nname = \"other-host\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[[bin]]\nname = \"other_tool\"\npath = \"src/main.rs\"\n",
    )
    .expect("other manifest");
    std::fs::write(root.join("other/src/main.rs"), "fn main() {}\n").expect("other bin");

    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let app = super::super::cargo_probe::package_targets(&cargo, &root.join("app/Cargo.toml"))
        .expect("cargo metadata answers for the workspace member");
    assert!(app.has_library, "the app package declares a library");
    assert_eq!(
        app.bins,
        vec!["app_tool".to_owned()],
        "a sibling member's binary must not be handed to `cargo rustc --bin`"
    );

    let other = super::super::cargo_probe::package_targets(&cargo, &root.join("other/Cargo.toml"))
        .expect("cargo metadata answers for the other member");
    assert!(!other.has_library, "the other package is binary-only");
    assert_eq!(other.bins, vec!["other_tool".to_owned()]);

    let _ = std::fs::remove_dir_all(&root);
}
