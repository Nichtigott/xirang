//! Asking `cargo` about one host package: a MIR snapshot, or the binary targets
//! it actually has.
//! 向 `cargo` 询问一个宿主包：一份 MIR 快照，或它实际拥有的二进制 target。
//!
//! Every entry point takes the manifest; nothing here resolves the project.
//! 每个入口都收下清单；本模块不解析项目。

use std::path::Path;

/// What `cargo metadata` says about one package.
/// `cargo metadata` 对一个包的说明。
pub(super) struct PackageTargets {
    /// Whether the package carries a library target at all.
    /// 该包是否带库 target。
    pub(super) has_library: bool,
    /// The names of the package's own binary targets, in cargo's order.
    /// 该包自己的二进制 target 名字，按 cargo 的顺序。
    pub(super) bins: Vec<String>,
}

/// Run `cargo rustc` against the host target that actually exists, passing
/// `rustc_args` through to that target.
/// 针对宿主实际存在的 target 运行 `cargo rustc`，并把 `rustc_args` 透传给该 target。
///
/// MIR inspection passed `--lib` unconditionally, so a binary-only host —
/// including the default output of `xirang new` — could not be inspected at
/// all. A library target is preferred; when cargo reports that the package has
/// none, its binary targets are tried **one at a time**, named from cargo's own
/// metadata. Cargo decides which targets exist, so a custom `[lib]`/`[[bin]]` path
/// cannot make the answer wrong.
/// MIR 检视此前无条件传 `--lib`，因此仅含二进制的宿主——包括 `xirang new` 的默认
/// 产物——完全无法被检视。优先选择库 target；当 cargo 报告该包没有库 target 时，改**逐个**
/// 尝试它的二进制 target，名字取自 cargo 自己的 metadata。哪些 target 存在由 cargo 决定，因此
/// 自定义 `[lib]`/`[[bin]]` 路径不会让答案出错。
pub(super) fn cargo_rustc_mir(
    manifest: &Path,
    rustc_args: &[&str],
) -> Result<std::process::Output, String> {
    // Stable rustc can write MIR directly; return its bytes through the existing Output contract.
    // 稳定版 rustc 可直接写 MIR；沿既有 Output 契约返回产物字节。
    let invoke = |selector: &[&str]| -> Result<std::process::Output, std::io::Error> {
        if rustc_args != ["--emit=mir"] {
            return crate::cargo_mir::command(manifest, selector, rustc_args).output();
        }
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "xirang-studio-mir-{}-{}.mir",
            std::process::id(),
            NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        let flag = format!("--emit=mir={}", path.display());
        let mut output = crate::cargo_mir::command(manifest, selector, &[&flag]).output()?;
        let bytes = if output.status.success() {
            std::fs::read(&path)
        } else {
            Ok(Vec::new())
        };
        let _ = std::fs::remove_file(path);
        if output.status.success() {
            output.stdout = bytes?;
        }
        Ok(output)
    };
    let missing_target = |error: std::io::Error| {
        format!("cannot run cargo rustc for {}: {error}", manifest.display())
    };
    let output = invoke(&["--lib"]).map_err(missing_target)?;
    if output.status.success() {
        return Ok(output);
    }
    // Whether this package has a library target is cargo's answer to give, not its English
    // wording: metadata says it outright, and only when metadata cannot run does the old
    // stderr phrase decide (audit `STU-S-25`).
    // 这个包有没有库 target 该由 cargo 给出答案，而不是由它的英文措辞决定：metadata 直接说明，
    // 只有在 metadata 跑不起来时，旧的 stderr 措辞才作数（审计 `STU-S-25`）。
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let targets = package_targets(&cargo, manifest);
    match &targets {
        // The library exists and failed to compile: that failure *is* the answer, and
        // falling through to the binaries would hide it.
        // 库存在但编译失败：那个失败本身就是答案，转而试二进制会把它藏起来。
        Ok(targets) if targets.has_library => return Ok(output),
        Ok(_) => {}
        Err(_) if !String::from_utf8_lossy(&output.stderr).contains("no library targets") => {
            return Ok(output);
        }
        Err(_) => {}
    }
    // `--bins` is not enough for a host with more than one binary target: cargo refuses to hand the
    // extra `rustc` arguments to several targets at once ("extra arguments to `rustc` can only be
    // passed to one target"), so a two-bin host could not be inspected at all (audit `S14`). The
    // names come from `cargo metadata`, and each target is tried in cargo's order; the first that
    // compiles is the answer. The metadata call is a fallback of a fallback: if it fails, the old
    // `--bins` path still runs rather than turning a case that used to work into an error.
    // 对"多于一个二进制 target"的宿主，`--bins` 不够：cargo 拒绝把额外的 `rustc` 参数同时交给多个
    // target（"extra arguments to `rustc` can only be passed to one target"），因此"两个 bin"的宿主
    // 完全无法被检视（审计 `S14`）。名字来自 `cargo metadata`，按 cargo 的顺序逐个尝试，第一个编译
    // 通过的就是答案。这次 metadata 调用是"回退的回退"：它失败时仍走原来的 `--bins` 路径，而不是把
    // 本来能用的情形变成错误。
    let Ok(targets) = targets else {
        return invoke(&["--bins"]).map_err(missing_target);
    };
    let mut last = None;
    for name in targets.bins {
        let output = invoke(&["--bin", name.as_str()]).map_err(missing_target)?;
        if output.status.success() {
            return Ok(output);
        }
        last = Some(output);
    }
    Ok(last.unwrap_or(output))
}

/// Read one package's own targets from `cargo metadata`.
/// 从 `cargo metadata` 读取一个包自己的 target。
///
/// `--no-deps` drops dependencies but not the other members of the same workspace, so the
/// package is picked out by its manifest path: a sibling member's binary is not this
/// package's binary, and handing its name to `cargo rustc --bin` names a target this
/// manifest does not have (audit `STU-S-25`).
/// `--no-deps` 会去掉依赖，但不会去掉同一工作区的其它成员，因此这里按清单路径把这个包挑出来：
/// 兄弟成员的二进制不是这个包的二进制，把它的名字交给 `cargo rustc --bin` 等于指了一个本清单
/// 根本没有的 target（审计 `STU-S-25`）。
pub(super) fn package_targets(
    cargo: &std::ffi::OsStr,
    manifest: &Path,
) -> Result<PackageTargets, String> {
    let output = std::process::Command::new(cargo)
        .args([
            "metadata",
            "--no-deps",
            "--format-version",
            "1",
            "--manifest-path",
        ])
        .arg(manifest)
        .output()
        .map_err(|error| {
            format!(
                "cannot run cargo metadata for {}: {error}",
                manifest.display()
            )
        })?;
    if !output.status.success() {
        return Err(format!(
            "cargo metadata failed for {}: {}",
            manifest.display(),
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let metadata: serde_json::Value = serde_json::from_slice(&output.stdout).map_err(|error| {
        format!(
            "cargo metadata for {} is not JSON: {error}",
            manifest.display()
        )
    })?;
    let wanted = manifest
        .canonicalize()
        .unwrap_or_else(|_| manifest.to_path_buf());
    let packages = metadata["packages"].as_array();
    let package = packages
        .into_iter()
        .flatten()
        .find(|package| {
            package["manifest_path"]
                .as_str()
                .map(Path::new)
                .is_some_and(|path| {
                    path.canonicalize()
                        .map_or_else(|_| path == wanted.as_path(), |path| path == wanted)
                })
        })
        .ok_or_else(|| {
            format!(
                "cargo metadata does not describe the package at {}",
                manifest.display()
            )
        })?;
    let targets = package["targets"]
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or_default();
    let kinds = |target: &serde_json::Value| {
        target["kind"]
            .as_array()
            .map(Vec::as_slice)
            .unwrap_or_default()
            .iter()
            .filter_map(|kind| kind.as_str())
            .map(str::to_owned)
            .collect::<Vec<_>>()
    };
    let has_library = targets.iter().any(|target| {
        kinds(target)
            .iter()
            .any(|kind| kind.contains("lib") || kind == "proc-macro")
    });
    let bins = targets
        .iter()
        .filter(|target| kinds(target).iter().any(|kind| kind == "bin"))
        .filter_map(|target| target["name"].as_str().map(str::to_owned))
        .collect();
    Ok(PackageTargets { has_library, bins })
}
