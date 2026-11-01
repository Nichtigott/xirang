//! The runtime evidence a host recorded, read back and checked against this tree.
//! 宿主记录下的运行期证据，读回来并与这棵树对照校验。
//!
//! A trace says what actually ran, which no static read can: `xirang.callgraph`
//! is a lexical candidate list and says so. The artifact format, its reader, and
//! the headless call report all already existed — the four renderers had no
//! caller anywhere in the workspace — so this module is the outlet, not a second
//! implementation. It keeps Studio's rule: a report is refused when the artifact
//! describes a different tree, because frames are node identities and identities
//! from another tree render a plausible, wrong call tree.
//! trace 说明真正跑了什么，这是任何静态读取都做不到的：`xirang.callgraph` 是词法候选清单，它
//! 自己也这么声明。artifact 格式、它的读取器、无终端调用报告本来都存在——那四个渲染器在整个工作区
//! 里没有一个调用者——因此本模块是出口而不是第二份实现。它保留 Studio 的规矩：当 artifact 描述的
//! 是另一棵树时拒绝出报告，因为帧是节点身份，而另一棵树的身份会渲染出一棵看似合理但错误的调用树。

use std::collections::HashSet;
use std::path::Path;

use crate::build_method::face_views;
use crate::run_method::runtime::trace::locals::LocalValue;
use crate::run_method::{
    CallTrace, TraceArtifact, read_trace_artifact, render_call_report_for_trace,
    trace_artifact_path,
};
use serde_json::Value;

use crate::mcp::apply::load_registry;
use crate::mcp::registry::namespace;

/// The most report lines one reply carries before it says it truncated.
/// 一条回复在声明被截断之前最多携带的报告行数。
const MAX_REPORT_LINES: usize = 400;

/// What reading this package's recorded trace produced.
/// 读取本包已记录的 trace 得到了什么。
///
/// Absence and refusal are *answers*, not errors: "no host recorded one" and "this
/// artifact describes another tree" are both things an agent can act on, and a
/// second tool needing the same trace (the trace-driven `xirang.converge`) must
/// inherit both verbatim rather than inventing its own wording.
/// 缺失与拒绝都是**答案**而不是错误：「没有宿主记录过」与「这份 artifact 描述的是另一棵树」都是代理
/// 能据以行动的东西；而需要同一份 trace 的第二个工具（由 trace 驱动的 `xirang.converge`）必须原样
/// 继承两者，而不是另造一套说法。
pub(crate) enum RecordedTrace {
    /// The ready-to-print answer for "no artifact here", with the way to produce one.
    /// 「这里没有 artifact」的现成答案，并带上产出它的办法。
    Absent(String),
    /// The header plus the refusal, when the artifact describes a different tree.
    /// 当 artifact 描述另一棵树时：表头加上拒绝理由。
    Refused(String),
    /// The artifact, its call trace, and the header that describes it.
    /// artifact、它的调用 trace，以及描述它的表头。
    Verified {
        artifact: TraceArtifact,
        call_trace: Box<CallTrace>,
        header: String,
    },
}

/// Read this package's trace artifact and check that it describes this tree.
/// 读取本包的 trace artifact，并核验它描述的是这棵树。
pub(crate) fn read_verified(root: &Path) -> Result<RecordedTrace, String> {
    // An absent recording needs no tree derivation, including for a broken package.
    // 缺失记录无需推导树，即使包当前损坏也应报告缺失。
    let path = trace_artifact_path(root);
    if !path.is_file() {
        return Ok(absent_recording(&path));
    }
    let namespace = namespace(root)?;
    let faces = face_views(root, &namespace)?;
    read_verified_in(root, &namespace, &faces)
}

/// Reuse the caller's tree so one query verifies and joins the same identities.
/// 复用调用方的树，使一次查询核验并拼接同一组身份。
pub(crate) fn read_verified_in(
    root: &Path,
    namespace: &str,
    faces: &[crate::build_method::FaceView],
) -> Result<RecordedTrace, String> {
    let path = trace_artifact_path(root);
    if !path.is_file() {
        return Ok(absent_recording(&path));
    }
    let (artifact, call_trace) = read_trace_artifact(&path)?;
    let expected_root = xirang_kernel::root_node_id(namespace);
    let header = format!(
        "artifact {}\nnamespace {} (recorded {})\nroot {} (recorded {})\nmode {:?}\nframes {} locals {} edges {}\n",
        path.display(),
        namespace,
        artifact.namespace,
        expected_root,
        artifact.root,
        artifact.mode,
        artifact.frames.len(),
        artifact.locals.len(),
        artifact.edges.len(),
    );
    // A13: one artifact is **one recorded run**, and a run reaches only the paths it took. Saying so
    // is what keeps this report from being read as the complete binding — which is exactly the claim
    // `xirang.callgraph` refuses to make from the other side (it is lexical, and says that too).
    // A13：一份 artifact 就是**一次被记录的运行**，而一次运行只走到它走到的那些路径。把这一点说出来，才能
    // 让这份报告不被读成完整绑定——这正是 `xirang.callgraph` 从另一侧拒绝做出的声明（它是词法的，也这么说）。
    let coverage = format!(
        "{}coverage one recorded run: a call this run never reached is absent from the report below, \
         and absence here is not evidence that a call cannot happen. Widen the picture with more runs, \
         and read `xirang.callgraph` beside it for the calls the source spells out.\n",
        header
    );
    let header = coverage;
    let mut mismatch = Vec::new();
    if artifact.namespace != namespace {
        mismatch.push(format!(
            "namespace: the artifact was recorded under `{}`, this package is `{namespace}`",
            artifact.namespace
        ));
    }
    if artifact.root != expected_root {
        mismatch.push(format!(
            "root: the artifact was minted under `{}`, this package's root is `{expected_root}`",
            artifact.root
        ));
    }
    // The root is a node the tree has even though no face declares it, so a host
    // that records a root frame is not reporting a foreign tree.
    // 根是没有面声明、但树确实拥有的节点，因此记录了根帧的宿主并不是在报告一棵异树。
    let mut known: HashSet<_> = faces.iter().map(|face| face.id).collect();
    known.insert(expected_root);
    let unresolved: Vec<_> = artifact
        .frames
        .iter()
        .filter(|frame| !known.contains(&frame.node))
        .collect();
    if let Some(first) = unresolved.first() {
        mismatch.push(format!(
            "{} of {} frame(s) name nodes this tree does not have, e.g. {} in `{}`",
            unresolved.len(),
            artifact.frames.len(),
            first.node,
            first.function
        ));
    }
    // Same-tree stale evidence: the artifact's node set can still match while the source or the
    // graft plan that produced it has moved. A trace older than either is a previous build's
    // observation, not current evidence.
    // 同树但过期：artifact 的节点集合可能仍然匹配，而产生它的源码或 graft 计划已经变了。比二者都旧的
    // trace 是上一次构建的观察，不是当前证据。
    if let Ok(trace_time) = std::fs::metadata(&path).and_then(|metadata| metadata.modified()) {
        let layout = crate::source_layout::source_layout(root).ok();
        if let Some(layout) = layout.as_ref() {
            for face in faces {
                let Ok(source) = face.locate_source(layout) else {
                    continue;
                };
                if let Ok(source_time) =
                    std::fs::metadata(&source).and_then(|metadata| metadata.modified())
                    && source_time > trace_time
                {
                    mismatch.push(format!(
                        "stale: {} was modified after the trace; rerun the host and record a new trace",
                        source.display()
                    ));
                }
            }
        }
        if let Ok(entry) = crate::build_method::host_entry_source(root)
            && let Ok(entry_time) =
                std::fs::metadata(&entry).and_then(|metadata| metadata.modified())
            && entry_time > trace_time
        {
            mismatch.push(format!(
                "stale: host entry {} was modified after the trace; rerun the host and record a new trace",
                entry.display()
            ));
        }
        let plans = root.join(".xirang/external-grafts");
        if let Ok(entries) = std::fs::read_dir(&plans) {
            for plan in entries
                .flatten()
                .map(|entry| entry.path().join("graft.plan"))
            {
                if let Ok(plan_time) =
                    std::fs::metadata(&plan).and_then(|metadata| metadata.modified())
                    && plan_time > trace_time
                {
                    mismatch.push(format!(
                        "stale: graft plan {} was modified after the trace; rerun the host and record a new trace",
                        plan.display()
                    ));
                }
            }
        }
    }
    if !mismatch.is_empty() {
        return Ok(RecordedTrace::Refused(format!(
            "{header}REFUSED: the artifact does not describe this tree\n  {}\nRerun the host after \
             `xirang check` so the recorded identities match the compiled ones.\n",
            mismatch.join("\n  ")
        )));
    }
    Ok(RecordedTrace::Verified {
        artifact,
        call_trace: Box::new(call_trace),
        header,
    })
}

/// The shared absence answer needs no source tree or namespace.
/// 共用的缺失回答不需要源码树或命名空间。
fn absent_recording(path: &Path) -> RecordedTrace {
    // Absence is the common case today, so the answer carries the way to
    // produce one instead of a bare "not found".
    // 缺失是今天的常态，因此答案要带上产出它的办法，而不是一句光秃秃的 "not found"。
    RecordedTrace::Absent(format!(
        "trace absent: {}\nA host writes one by recording with the `trace_call!` family and running \
             with `XIRANG_TRACE` (the mode) or `XIRANG_TRACE_FILE` (the path) set; a project \
             scaffolded by `xirang new` demonstrates that whole chain in its `src/main.rs`. \
             `xirang check` reports the static side only.\n",
        path.display()
    ))
}

/// The observed caller/callee function pairs from this package's verified trace, if one exists.
/// 本包已核验追踪里的调用者/被调用者函数对（若存在）。
///
/// The caller (`callgraph`) reports these separately from static edges: one recorded run proves an
/// edge happened, never that a missing edge cannot happen.
/// 调用图把这些与静态边分开报告：一次被记录的运行证明某条边发生过，绝不证明缺失的边不可能发生。
pub(crate) fn recorded_call_edges(root: &Path) -> Vec<(String, String)> {
    match read_verified(root) {
        Ok(RecordedTrace::Verified { call_trace, .. }) => call_trace
            .call_edges()
            .into_iter()
            .map(|edge| {
                (
                    edge.caller.function.to_owned(),
                    edge.callee.function.to_owned(),
                )
            })
            .collect(),
        Ok(RecordedTrace::Absent(_) | RecordedTrace::Refused(_)) | Err(_) => Vec::new(),
    }
}

/// Read this package's trace artifact and answer with the call report it implies.
/// 读取本包的 trace artifact，并用它蕴含的调用报告作答。
pub(crate) fn trace(root: &Path, arguments: &Value) -> Result<String, String> {
    match read_verified(root)? {
        RecordedTrace::Absent(answer) | RecordedTrace::Refused(answer) => Ok(answer),
        RecordedTrace::Verified {
            call_trace,
            header,
            artifact,
            ..
        } => {
            let query = arguments.get("query").and_then(Value::as_str);
            if arguments.get("values").and_then(Value::as_bool) == Some(true) {
                return Ok(bounded(header, &render_values(&artifact, query)));
            }
            let namespace = namespace(root)?;
            let registry = load_registry(root, &namespace)?;
            let report = render_call_report_for_trace(&registry, &call_trace, query);
            Ok(bounded(header, &report))
        }
    }
}

/// One recorded value as a line, in the one spelling both tools use.
/// 一条记录值渲染成一行，两个工具共用同一种拼法。
pub(crate) fn local_line(local: &LocalValue) -> String {
    format!(
        "  {}: {} = {}  [{}, {}]  @ {}:{}\n",
        local.name,
        local.type_name,
        local.value,
        local.kind.label(),
        local.observation.label(),
        local.source.file,
        local.source.line
    )
}

/// Render the values the run recorded, grouped by the frame they were recorded in,
/// plus the observed data edges between them.
/// 渲染这次运行记录下的值——按记录它们时所在的帧分组——以及它们之间被观察到的数据边。
///
/// The call report says what ran; this says what it saw. A local belongs to a frame
/// by `frame_id`, and one recorded outside every traced call is reported as such
/// rather than folded into a frame it never had; a local or edge naming something
/// this artifact does not contain is *counted* instead of being silently dropped, so
/// an incomplete artifact cannot read as a complete one.
/// 调用报告说跑了什么，这里说它看见了什么。局部值靠 `frame_id` 属于某个帧，而在任何被追踪调用之外记录
/// 的那个会被如实报成如此，而不是被塞进它从未属于的帧；点名的东西不在本 artifact 里的局部值或边会被
/// **计数**而不是悄悄丢掉，因此一份不完整的 artifact 不会读起来像完整的。
fn render_values(artifact: &TraceArtifact, query: Option<&str>) -> String {
    let keeps = |name: &str, type_name: &str, value: &str| match query {
        Some(query) => name.contains(query) || type_name.contains(query) || value.contains(query),
        None => true,
    };
    let line = local_line;
    let mut lines = 0usize;
    let mut shown = 0usize;
    // One label per number, in the order this function writes them everywhere else (label,
    // then count). The header used to read `values {locals} locals {edges} edges` — two
    // counts under three labels, so a reader could not tell which number belonged to what.
    // 每个数字一个标签，顺序与本函数别处一致（标签在前、计数在后）。表头过去是
    // `values {locals} locals {edges} edges`——两个计数配三个标签，读者分不清哪个数字属于谁。
    let mut output = format!(
        "locals {} edges {}\n",
        artifact.locals.len(),
        artifact.edges.len()
    );
    for frame in &artifact.frames {
        if lines >= MAX_REPORT_LINES {
            break;
        }
        let inside: Vec<_> = artifact
            .locals
            .iter()
            .filter(|local| local.frame_id == Some(frame.frame_id))
            .filter(|local| keeps(&local.name, &local.type_name, &local.value))
            .collect();
        if inside.is_empty() {
            continue;
        }
        let at = frame
            .source
            .as_ref()
            .map(|source| format!("{}:{}", source.file, source.line))
            .unwrap_or_else(|| "-".to_owned());
        output.push_str(&format!(
            "frame {} {}  ({at})  {} local(s)\n",
            frame.frame_id,
            frame.function,
            inside.len()
        ));
        lines += 1;
        for local in inside {
            output.push_str(&line(local));
            lines += 1;
            shown += 1;
        }
    }
    let outside: Vec<_> = artifact
        .locals
        .iter()
        .filter(|local| local.frame_id.is_none())
        .filter(|local| keeps(&local.name, &local.type_name, &local.value))
        .collect();
    if !outside.is_empty() && lines < MAX_REPORT_LINES {
        output.push_str("outside any traced frame\n");
        lines += 1;
        for local in &outside {
            output.push_str(&line(local));
            lines += 1;
            shown += 1;
        }
    }
    let dangling = artifact
        .locals
        .iter()
        .filter(|local| {
            local
                .frame_id
                .is_some_and(|id| !artifact.frames.iter().any(|frame| frame.frame_id == id))
        })
        .count();
    if dangling > 0 {
        output.push_str(&format!(
            "locals naming a frame this artifact does not have {dangling}\n"
        ));
    }
    let name_of = |id: u64| {
        artifact
            .locals
            .iter()
            .find(|local| local.id == id)
            .map(|local| local.name.as_str())
            .unwrap_or("<not recorded>")
    };
    let edges: Vec<_> = artifact
        .edges
        .iter()
        .filter(|edge| match query {
            Some(query) => {
                name_of(edge.from).contains(query)
                    || name_of(edge.to).contains(query)
                    || edge.label.contains(query)
            }
            None => true,
        })
        .collect();
    output.push_str(&format!("data edges {}\n", artifact.edges.len()));
    lines += 1;
    for edge in &edges {
        if lines >= MAX_REPORT_LINES {
            break;
        }
        let at = edge
            .source
            .as_ref()
            .map(|source| format!("{}:{}", source.file, source.line))
            .unwrap_or_else(|| "-".to_owned());
        output.push_str(&format!(
            "  {} -> {}  ({})  @ {at}\n",
            name_of(edge.from),
            name_of(edge.to),
            edge.label
        ));
        lines += 1;
    }
    let unrecorded = artifact
        .edges
        .iter()
        .filter(|edge| {
            !artifact.locals.iter().any(|local| local.id == edge.from)
                || !artifact.locals.iter().any(|local| local.id == edge.to)
        })
        .count();
    if unrecorded > 0 {
        output.push_str(&format!(
            "edges naming a local this artifact does not have {unrecorded}\n"
        ));
    }
    if shown == 0 && query.is_some() {
        output.push_str("no recorded value matches the query\n");
    }
    output
}

/// Keep one reply inside a size an agent can read, and say when it did not.
/// 把一条回复限制在代理读得下的规模里，并在截断时说出来。
///
/// The header this module prepends is not part of the bound — it says which artifact
/// the report came from, and a reader needs it whether or not the body was cut — so
/// the cut itself is the shared outlet's, `crate::mcp::truncation::bounded`.
/// 本模块前置的表头不计入上限——它说明这份报告来自哪份 artifact，而无论正文是否被切，读取方都
/// 需要它——因此切分本身由共享出口 `crate::mcp::truncation::bounded` 完成。
fn bounded(header: String, report: &str) -> String {
    format!(
        "{header}{}",
        crate::mcp::truncation::bounded(report, MAX_REPORT_LINES, "lines", "narrow with `query`")
    )
}

#[cfg(test)]
#[path = "trace_tests.rs"]
mod trace_tests;
