//! Execution-owned parser reuse and optional MCP phase timings.
//! 执行层持有的解析复用与可选 MCP 阶段计时。
//! Filesystem reads stay fresh; only exact-content validation results are reused.
//! 文件系统每次照常读取；仅复用完整内容相同的校验结果。

#[path = "source_cache/facts.rs"]
mod facts;
#[cfg(feature = "mcp")]
pub(crate) use facts::Unit;
pub(crate) use facts::facts;
#[cfg(feature = "build")]
#[path = "source_cache/snapshot.rs"]
mod snapshot;
#[cfg(feature = "build")]
pub(crate) use snapshot::scope;

#[cfg(feature = "mcp")]
#[path = "source_cache/disk.rs"]
mod disk;

#[path = "source_cache/ordinary.rs"]
mod ordinary;
#[path = "source_cache/ordinary_store.rs"]
pub(crate) mod ordinary_store;

/// Optional phase timings stay on stderr and never alter a tool reply.
/// 可选阶段耗时只写 stderr，不改变工具回答。
#[cfg(feature = "mcp")]
pub(crate) fn phase<T>(name: &str, read: impl FnOnce() -> T) -> T {
    let started = std::time::Instant::now();
    let result = read();
    if std::env::var_os("XIRANG_MCP_PROFILE").is_some() {
        eprintln!(
            "xirang phase: {name} ms={:.3}",
            started.elapsed().as_secs_f64() * 1000.0
        );
    }
    result
}

/// Execution adapter retaining the native parser's results and diagnostics.
/// 保留原生解析结果与诊断的执行适配器。
pub(crate) fn parse_face(
    source: &str,
) -> Result<Option<xirang_kernel::syntax::FaceSyntax>, xirang_kernel::syntax::FaceSyntaxError> {
    match ordinary::validate(source) {
        Some(result) => result.map(|()| None),
        None => xirang_kernel::syntax::parse_face(source),
    }
}

/// Entry discovery shares ordinary-source validation with face discovery.
/// 入口发现与注册面发现共用普通源码校验。
#[cfg(feature = "build")]
pub(crate) fn application_entries(
    source: &str,
) -> Result<
    Vec<(String, xirang_kernel::syntax::SyntaxLocation)>,
    xirang_kernel::syntax::FaceSyntaxError,
> {
    match ordinary::validate(source) {
        Some(result) => result.map(|()| Vec::new()),
        None => xirang_kernel::syntax::application_entries(source),
    }
}
