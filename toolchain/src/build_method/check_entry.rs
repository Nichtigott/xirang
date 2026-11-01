//! The check entry points: the same discovery and validation pipeline as the build entry, but the
//! structured diagnostics back instead of the text a terminal reads.
//! 校验入口：与构建入口同一条发现与校验管线，但交回结构化诊断，而不是终端读的文本。

use std::path::Path;

use crate::build_method::build_input::{BuildInput, HostCut};
use crate::build_method::registry_identity;

use super::{DeclaredFailure, declared_cuts, run_declared};

/// Run the same discovery and validation pipeline as [`crate::build_method::run_for`], but hand the
/// caller the structured [`xirang_kernel::BuildDiagnostics`] instead of the text a
/// terminal or `compile_error!` reads.
/// 运行与 [`crate::build_method::run_for`] 相同的发现与校验管线，但把结构化的
/// [`xirang_kernel::BuildDiagnostics`] 交给调用方，而不是终端或 `compile_error!` 读的文本。
///
/// This is the machine-readable twin of `run_for`: a CI job or `xirang check
/// --json` needs to count and serialize individual failures, and re-parsing the
/// rendered frame would make the layout part of the contract. The rendered text
/// stays byte-identical because `run_for` renders exactly this value with the
/// unchanged `render()`.
/// 这是 `run_for` 的机器可读孪生：CI 或 `xirang check --json` 需要逐条计数并序列化
/// 失败，而重新解析渲染文本会让版式变成契约。渲染文本保持逐字节一致，因为 `run_for`
/// 用未改动的 `render()` 渲染的正是这个值。
pub fn check_for(
    manifest: &Path,
    out_dir: &Path,
    package: &str,
) -> Result<(), xirang_kernel::BuildDiagnostics> {
    check_for_shape(manifest, out_dir, package, None, false, false)
}

/// [`check_for`] for one fragment of a partition: it compiles `only`, or it is the facade.
/// [`check_for`] 用于划分出来的一个碎片：它编译 `only`，或者它就是 facade。
pub fn check_for_shape(
    manifest: &Path,
    out_dir: &Path,
    package: &str,
    only: Option<String>,
    facade: bool,
    emit_cargo_directives: bool,
) -> Result<(), xirang_kernel::BuildDiagnostics> {
    check_for_shape_with_cuts(
        manifest,
        out_dir,
        package,
        only,
        facade,
        emit_cargo_directives,
        &[],
    )
}

/// [`check_for_shape`] with the host's graft cuts handed in as data.
/// [`check_for_shape`]，外加以数据交进来的宿主 graft 切口。
#[allow(clippy::too_many_arguments)]
pub fn check_for_shape_with_cuts(
    manifest: &Path,
    out_dir: &Path,
    package: &str,
    only: Option<String>,
    facade: bool,
    emit_cargo_directives: bool,
    cuts: &[HostCut],
) -> Result<(), xirang_kernel::BuildDiagnostics> {
    check_for_shape_with_cuts_named(
        manifest,
        out_dir,
        package,
        only,
        facade,
        emit_cargo_directives,
        cuts,
        None,
    )
}

/// The structured check with the host library name carried from the partition plan.
/// 结构化校验，同时携带分区计划里的宿主库名。
#[allow(clippy::too_many_arguments)]
pub fn check_for_shape_with_cuts_named(
    manifest: &Path,
    out_dir: &Path,
    package: &str,
    only: Option<String>,
    facade: bool,
    emit_cargo_directives: bool,
    cuts: &[HostCut],
    host_crate: Option<String>,
) -> Result<(), xirang_kernel::BuildDiagnostics> {
    std::fs::create_dir_all(out_dir).map_err(|error| {
        let mut diagnostics = xirang_kernel::BuildDiagnostics::default();
        diagnostics.push(xirang_kernel::BuildDiagnostic::new(
            "out-dir",
            format!("create {}: {error}", out_dir.display()),
        ));
        diagnostics
    })?;
    registry_identity::set_package_namespace(package.to_owned());
    let shape_only = only.clone();
    let input = BuildInput::new(
        manifest.to_path_buf(),
        out_dir.to_path_buf(),
        emit_cargo_directives,
    )
    .with_shape(only, facade)
    .with_host_cuts(cuts.to_vec())
    .with_host_crate(host_crate);
    let claimed = declared_cuts(manifest, &input.host_cuts);
    // `package` is the authority for this run, not the process-wide pin: a
    // bridge or a test binary runs several packages in one process, and the pin
    // is first-write-wins, so a second run would otherwise publish identities
    // stamped with the first package's namespace. The scope also serializes
    // concurrent runs, because that namespace is process-wide.
    // 本次运行的权威是 `package` 而不是进程级固定值：桥或测试二进制会在一个进程里跑多个包，而该
    // 固定值先到先得，因此第二次运行发布的身份会盖着第一个包的命名空间。这个作用域同时串行化并发
    // 运行，因为那个命名空间是进程级的。
    match run_declared(&input, package, shape_only.as_deref(), facade, &claimed) {
        Ok(()) => Ok(()),
        Err(DeclaredFailure::Pipeline(diagnostics) | DeclaredFailure::Native(diagnostics)) => {
            Err(diagnostics)
        }
    }
}
