//! Headless call reports shared by the core and the optional Studio.
//! 注册核心与可选 Studio 共用的无终端调用报告。

use std::fmt::Write as _;

use crate::run_method::registry_core::declaration::source_file_matches;
use crate::run_method::{CallTrace, Registry};

/// Render the compact fallback frame used by integration diagnostics.
/// 渲染集成诊断使用的紧凑备用画面。
pub fn render_frame(registry: &Registry, tick: u64, event: &str) -> String {
    format!(
        "REGISTRATION DEBUG  tick={tick}\nTREE\n{}\nEVENT\n{event}\n",
        registry.dump()
    )
}

/// Render the registration-face report used when no call event has been recorded.
/// 渲染尚无调用事件时使用的注册面报告。
///
/// With no trace to read, this entry can only list the faces a query matches
/// (`… no call event recorded yet`); a call tree needs a trace the caller
/// recorded, which is [`render_call_report_for_trace`]. The first line used to
/// promise a complete logical call tree, which this function cannot produce: it
/// builds a disabled trace on purpose rather than turning the registration tree
/// into a fake call graph, and it reads no process policy either — a report has
/// no reason to depend on `XIRANG_TRACE`.
/// 没有 trace 可读时，这一入口只能列出与查询匹配的注册面（`… no call event recorded yet`）；
/// 调用树需要调用方自己记录的 trace，即 [`render_call_report_for_trace`]。首句过去承诺渲染完整
/// 逻辑调用树，而本函数做不到：它有意构造一条关闭的 trace，而不是把注册树伪装成调用图；它也不读
/// 进程策略——报告没有理由依赖 `XIRANG_TRACE`。
pub fn render_call_report(registry: &Registry, query: Option<&str>) -> String {
    // A headless report has no execution event by definition. Do not turn the
    // registration tree into a fake call graph; callers must provide a trace.
    // 无终端报告默认没有执行事件，不能把注册树伪装成调用图；调用方必须提供 trace。
    let trace = CallTrace::disabled();
    render_call_report_for_trace(registry, &trace, query)
}

/// Render a report from a trace collected by a real runtime operation.
/// 用真实运行时操作收集的调用记录渲染报告。
pub fn render_call_report_for_trace(
    registry: &Registry,
    trace: &CallTrace,
    query: Option<&str>,
) -> String {
    let paths = matching_call_paths(registry, trace, query);
    let heading = query.map_or_else(
        || "call tree:".to_owned(),
        |query| format!("call search `{query}`: {} matching path(s)", paths.len()),
    );
    if paths.is_empty() {
        let registrations = query
            .map(|query| matching_registrations(registry, query))
            .unwrap_or_default();
        if registrations.is_empty() {
            return format!("{heading}\nno matching function, method, step, or declaration file");
        }
        let mut output = format!("{heading}\nregistered faces (no call event recorded yet):");
        for info in registrations {
            let path = registry
                .path_for(info.id)
                .unwrap_or_else(|| "<unknown>".to_owned());
            let function = query
                .filter(|query| {
                    info.runtime_checks.iter().any(|check| {
                        check
                            .name()
                            .to_ascii_lowercase()
                            .contains(&query.to_ascii_lowercase())
                    })
                })
                .and_then(|query| {
                    info.runtime_checks
                        .iter()
                        .find(|check| {
                            check
                                .name()
                                .to_ascii_lowercase()
                                .contains(&query.to_ascii_lowercase())
                        })
                        .map(|check| check.name())
                })
                .unwrap_or(&info.source.function);
            // `kind` is printed once: the handle is the same string by rule
            // (`__registration_face!` expands both from `stringify!($kind)`), and
            // a report that states one fact twice reads as two facts.
            // `kind` 只打印一次：handle 按规则就是同一个字符串
            // （`__registration_face!` 用 `stringify!($kind)` 展开两者），而把同一个事实
            // 写两遍的报告读起来像两个事实。
            writeln!(
                output,
                "  `-- {}::{} kind={} declared-at={}",
                path,
                function,
                info.kind,
                info.source.describe(),
            )
            .unwrap();
        }
        return output;
    }
    let mut output = heading;
    for path in paths {
        output.push('\n');
        for (depth, call) in path.iter().enumerate() {
            let prefix = if depth + 1 == path.len() {
                "`--"
            } else {
                "|--"
            };
            let details = registry
                .find_registry(call.node)
                .map(|info| {
                    let path = registry
                        .path_for(call.node)
                        .unwrap_or_else(|| "<unknown>".to_owned());
                    let call_source = call
                        .source
                        .map(|source| format!(" call-at={source}"))
                        .unwrap_or_default();
                    format!(
                        "{} {}::{}#{} declared-at={}{call_source}",
                        call.node,
                        path,
                        call.function,
                        call.frame_id,
                        info.source.describe(),
                    )
                })
                .unwrap_or_else(|| format!("{} {}", call.node, call.function));
            writeln!(output, "  {}{} {}", "  ".repeat(depth), prefix, details).unwrap();
        }
    }
    output
}

fn matching_registrations<'a>(
    registry: &'a Registry,
    query: &str,
) -> Vec<&'a crate::run_method::RegistrationSnapshot> {
    let query = query.to_ascii_lowercase();
    registry.find_where(|info| registration_matches(registry, info, &query))
}

/// Match every searchable registration-face field.
/// 匹配所有可搜索的注册面字段。
///
/// The list holds one entry per distinct field: the handle and the parameter
/// metadata are the kind by rule, so searching the kind already searches them.
/// 列表里每个不同的字段各占一项：handle 与参数元数据按规则就是 kind，因此搜索 kind 就等于
/// 搜索它们。
fn registration_matches(
    registry: &Registry,
    info: &crate::run_method::RegistrationSnapshot,
    query: &str,
) -> bool {
    let text_matches = [
        info.kind.as_str(),
        info.preset.as_str(),
        info.parts.as_str(),
        info.source.file.as_str(),
        info.source.function.as_str(),
        info.name.zh.as_str(),
        info.name.en.as_str(),
        info.summary.zh.as_str(),
        info.summary.en.as_str(),
        info.registry_name.as_str(),
    ]
    .into_iter()
    .any(|value| value.to_ascii_lowercase().contains(query));
    text_matches
        || info.id.to_string().contains(query)
        || registry
            .path_for(info.id)
            .is_some_and(|path| path.to_ascii_lowercase().contains(query))
        || info
            .exports
            .iter()
            .any(|export| export.to_ascii_lowercase().contains(query))
        || info.requires.iter().any(|requirement| {
            requirement.capability.to_ascii_lowercase().contains(query)
                || requirement.provider.to_ascii_lowercase().contains(query)
        })
        || info
            .provides
            .iter()
            .any(|provide| provide.to_ascii_lowercase().contains(query))
        || info
            .runtime_checks
            .iter()
            .any(|check| check.name().to_ascii_lowercase().contains(query))
}

fn matching_call_paths(
    registry: &Registry,
    trace: &CallTrace,
    query: Option<&str>,
) -> Vec<Vec<crate::run_method::CallSite>> {
    let query = query.map(str::trim).filter(|query| !query.is_empty());
    let matches = trace
        .frame_ids()
        .filter_map(|frame_id| trace.path_for_frame(frame_id))
        .filter(|path| {
            query.is_none_or(|query| {
                let query = query.to_ascii_lowercase();
                path.iter().any(|call| {
                    call.node.to_string().contains(&query)
                        || call.function.to_ascii_lowercase().contains(&query)
                        || call
                            .source
                            .is_some_and(|source| source_file_matches(source.file, &query))
                        || registry
                            .find_registry(call.node)
                            .is_some_and(|info| registration_matches(registry, info, &query))
                })
            })
        })
        .collect::<Vec<_>>();

    // Keep only maximal paths so a query returns one complete chain.
    // 只保留最长路径，使查询返回完整调用链。
    matches
        .iter()
        .filter(|path| {
            !matches
                .iter()
                .any(|other| other.len() > path.len() && other.starts_with(path.as_slice()))
        })
        .cloned()
        .collect()
}
