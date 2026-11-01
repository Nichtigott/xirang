//! `xirang build`: validate the registration tree, then run `cargo build`.
//! `xirang build`：先校验注册树，再运行 `cargo build`。
//!
//! Split out of `lib.rs`: the command is a thin process wrapper around
//! `registration_check`, and the argv split (leading path versus passed-through
//! cargo options) is the only rule it owns.
//! 从 `lib.rs` 拆出：本命令是 `registration_check` 之上的薄进程包装，唯一拥有的规则
//! 是 argv 拆分（首个路径与透传的 cargo 选项）。

use std::io::Write;

use super::{build_target, registration_check, split_build_args};

/// Validate the registration tree first; only then invoke `cargo build`
/// with the remaining arguments passed through verbatim. The path, when
/// given, must come before any cargo option.
/// 先校验注册树；通过后再调用 `cargo build`，其余参数原样透传。
/// 路径如给出，必须位于任何 cargo 选项之前。
pub(crate) fn build(
    args: &mut impl Iterator<Item = String>,
    out: &mut dyn Write,
) -> Result<(), String> {
    let all: Vec<String> = args.by_ref().collect();
    // `build --help` used to reach cargo with the flag mirrored in the registration
    // check; the flag now answers the same way it does everywhere else (audit
    // `LGC-LG-44`).
    // `build --help` 过去会把该旗标透传给 cargo、同时让它出现在注册校验里；现在这个旗标与其它
    // 子命令回答一致（审计 `LGC-LG-44`）。
    if all
        .iter()
        .any(|argument| argument == "-h" || argument == "--help")
    {
        return crate::cli::usage(out);
    }
    let (directory, cargo_args) = split_build_args(&all);
    // The validated project is the one cargo will build: a `--manifest-path` among the
    // passed-through arguments names it, and the check used to miss that and validate the current
    // directory instead — so the banner described a project that was not the one being built (audit
    // `S11`).
    // 被校验的项目就是 cargo 会构建的那个：透传参数里的 `--manifest-path` 点名了它，而校验过去漏掉
    // 这一点、改为校验当前目录——于是那句横幅描述的是一个并不是正在被构建的项目（审计 `S11`）。
    let directory = build_target(directory, &cargo_args)?;
    // A project described by `xirang.toml` is not a Cargo host: its plan comes from the
    // multilingual inventory, and `cargo build` has nothing to build there.
    // 由 `xirang.toml` 描述的项目不是 Cargo 宿主：它的计划来自多语言清点，`cargo build` 在那里没有可构建之物。
    let xirang_project = std::path::Path::new(&directory).join("xirang.toml");
    if xirang_project.is_file() {
        let locked = all.iter().any(|argument| argument == "--locked");
        let summary = crate::multilang::build_project(std::path::Path::new(&directory), locked)?;
        writeln!(out, "{summary}")
            .map_err(|error| format!("cannot write the build summary: {error}"))?;
        return Ok(());
    }
    let package = registration_check(&directory)?;
    println!("xirang build: registration ok ({package})");
    let manifest = std::fs::canonicalize(&directory)
        .map_err(|error| format!("cannot resolve {directory}: {error}"))?;
    let status = std::process::Command::new("cargo")
        .arg("build")
        .args(&cargo_args)
        .current_dir(&manifest)
        .status()
        .map_err(|error| format!("cannot start cargo build: {error}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("cargo build failed ({status})"))
    }
}
