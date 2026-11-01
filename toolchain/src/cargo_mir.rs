//! Shared Cargo MIR invocation shape; execution lifetime belongs to each caller.
//! 共用 Cargo MIR 调用形状；执行生命周期仍由各调用方持有。
use std::path::Path;

pub(crate) fn command(
    manifest: &Path,
    selector: &[&str],
    rustc_args: &[&str],
) -> std::process::Command {
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let mut command = std::process::Command::new(cargo);
    command
        .args(["rustc", "--manifest-path"])
        .arg(manifest)
        .args(selector)
        .arg("--quiet")
        .arg("--")
        .args(rustc_args);
    command
}
