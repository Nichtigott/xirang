//! Loading a host-recorded trace artifact into a Studio session.
//! 把宿主记录的 trace artifact 载入 Studio 会话。
//!
//! Studio reads at most one artifact, once, when the session starts. The path
//! comes from `trace_artifact_path`, the same function the host's writer asked,
//! so an override cannot move the file for one end only. An artifact that names
//! another project is refused with its reason instead of being drawn as if it
//! belonged here — that refusal is the honesty the old sample label stood in for.
//! The legend and the DATA panel read `App::trace_status`; no code installs
//! evidence that has not passed the identity checks below.
//! Studio 最多读一份 artifact，且只在会话启动时读一次。路径来自 `trace_artifact_path`，与宿主
//! 的写入方问的是同一个函数，因此覆盖不会只挪动一端。指名了别的项目的 artifact 会带着原因被拒绝，
//! 而不是被当作本项目的证据画出来——那份拒绝正是旧的示例标注过去代为承担的诚实。图例与 DATA
//! 面板都读 `App::trace_status`；没有任何代码会装入未通过下列身份检查的证据。

use std::collections::{BTreeMap, BTreeSet};

use crate::run_method::{
    CallTrace, NodeId, Registry, TraceArtifact, read_trace_artifact, trace_artifact_path,
};

use super::App;
use super::project_context::{package_namespace, package_root};

/// What became of the trace artifact a session looked for.
/// 会话查找 trace artifact 的结果。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TraceStatus {
    /// No artifact at the convention path, so no trace is installed.
    /// 约定路径上没有 artifact，因此没有装入任何追踪。
    Absent,
    /// A matching artifact was rebuilt into `App::runtime_trace`.
    /// 匹配的 artifact 已重建进 `App::runtime_trace`。
    Loaded,
    /// An artifact was found and refused, with the reason it was refused.
    /// 找到了 artifact 但予以拒绝，并保留拒绝它的原因。
    Mismatch {
        /// Why the artifact does not belong to this session.
        /// 该 artifact 不属于本会话的原因。
        reason: String,
    },
}

impl App {
    /// Look for this project's trace artifact and install it when it matches.
    /// 查找本项目的 trace artifact，并在匹配时装入。
    ///
    /// Called once from `App::load`. A missing file installs a disabled trace and
    /// is not an error; a present-but-refused artifact installs nothing, keeps the
    /// registry visible, and states the reason on the event line. That is the
    /// `poll_hot_reload` habit of keeping the last good snapshot applied to
    /// evidence: the faces are still good, so they stay.
    /// 由 `App::load` 调用一次。文件缺失时装入关闭的追踪，且不算错误；存在却被拒绝的 artifact
    /// 不装入任何东西，保持注册表可见，并把原因写到事件行。这是把 `poll_hot_reload`“保留上一份
    /// 好快照”的习惯用在证据上：注册面仍然完好，因此继续显示。
    pub(super) fn install_trace(&mut self) {
        if let Some(reason) = self.rejudge_evidence() {
            self.alert(format!("Trace artifact refused: {reason}"));
        }
    }

    /// Judge the artifact against the snapshot that is loaded *now*, dropping the
    /// MIR snapshot that was built from the previous one.
    /// 针对**此刻**已装入的快照裁决 artifact，并丢弃按上一份快照构建的 MIR 快照。
    ///
    /// Any path that replaces `registry` has to call this: the trace's only real
    /// "different build" detector is "every recorded node still exists in this
    /// snapshot", so a swap makes the previous judgement stale, and a MIR graph built
    /// from the old tree would keep marking edges for faces that are gone. `App::load`
    /// did this through `install_trace`; `poll_hot_reload` and `reload` replaced the
    /// registry without it, which is how a session could keep saying `LIVE` over
    /// evidence that no longer held (audit `STU-S-03`).
    /// 任何替换 `registry` 的路径都必须调用它：追踪唯一真正的"不同构建"检测器是"每个已记录节点
    /// 仍存在于本快照"，因此一次替换会让上一次裁决过期，而按旧树构建的 MIR 图会继续为已经不存在的
    /// 注册面标边。`App::load` 经 `install_trace` 做了这件事；`poll_hot_reload` 与 `reload`
    /// 替换了注册表却没有做，这正是会话能在不再成立的证据上继续宣称 `LIVE` 的原因（审计
    /// `STU-S-03`）。
    ///
    /// The reason is returned rather than written to `event`, so the caller can keep
    /// its own message and append this one instead of losing either.
    /// 原因作为返回值交回而不是写进 `event`，调用方因此能保住自己的消息并把这条接上去，而不是
    /// 丢掉任何一条。
    pub(super) fn rejudge_evidence(&mut self) -> Option<String> {
        let (status, trace) = self.read_trace();
        let reason = match &status {
            TraceStatus::Mismatch { reason } => Some(reason.clone()),
            TraceStatus::Absent | TraceStatus::Loaded => None,
        };
        self.runtime_trace = trace;
        self.trace_status = status;
        self.mir_graph = None;
        reason
    }

    /// The artifact decision, without mutating the session.
    /// artifact 的判定，不改动会话。
    ///
    /// Absence is decided by the file check, so a missing artifact is `Absent`
    /// rather than the reader's "cannot read" error; a file that is there but
    /// unreadable or malformed is a refusal with that message as its reason.
    /// 缺失由文件检查判定，因此没有 artifact 是 `Absent`，而不是读取方的“无法读取”错误；
    /// 文件存在却读不动或格式错误，则是一次拒绝，原因就是那条消息。
    fn read_trace(&self) -> (TraceStatus, CallTrace) {
        let path = trace_artifact_path(&package_root());
        if !path.is_file() {
            return (TraceStatus::Absent, CallTrace::disabled());
        }
        match read_trace_artifact(&path) {
            Ok((artifact, trace)) => match self.trace_mismatch(&artifact) {
                Some(reason) => (TraceStatus::Mismatch { reason }, CallTrace::disabled()),
                None => (TraceStatus::Loaded, trace),
            },
            Err(reason) => (TraceStatus::Mismatch { reason }, CallTrace::disabled()),
        }
    }

    /// The identity checks an artifact must pass, or the first reason it fails.
    /// artifact 必须通过的身份检查，或它失败的第一个原因。
    ///
    /// The layout version is not checked here because the parser refuses an
    /// unsupported one before this is reached, and that refusal is the reason
    /// `read_trace` reports. The checks left are the ones nothing else can make:
    /// the namespace and root anchors, and whether every recorded node still
    /// exists in this snapshot. The last is the only real "different build"
    /// detector — node ids are source-path plus namespace derived, so neither side
    /// carries a build hash, and a stale artifact of the same crate is caught by
    /// its records pointing at faces this snapshot no longer has.
    /// 这里不检查版式版本，因为解析器会在到此之前拒绝不支持的版本，而那份拒绝就是 `read_trace`
    /// 报告的原因。剩下的检查是别处无法代劳的：命名空间与根锚点，以及每个已记录节点是否仍存在于本
    /// 快照。最后一项是唯一真正的“不同构建”检测器——节点 id 由源码路径加命名空间推出，两侧都不
    /// 携带构建散列；同一 crate 的过期 artifact 会因其记录指向本快照已没有的注册面而被抓住。
    fn trace_mismatch(&self, artifact: &TraceArtifact) -> Option<String> {
        let namespace = package_namespace();
        if artifact.namespace != namespace {
            return Some(format!(
                "namespace `{}` does not match this project's `{namespace}`",
                artifact.namespace
            ));
        }
        if artifact.root != self.registry.id() {
            return Some(format!(
                "root `{}` is not this project's registry root `{}`",
                artifact.root,
                self.registry.id()
            ));
        }
        let unresolved = unresolved_nodes(artifact, &self.registry);
        if !unresolved.is_empty() {
            let named = unresolved
                .iter()
                .take(3)
                .map(NodeId::to_string)
                .collect::<Vec<_>>()
                .join(", ");
            return Some(format!(
                "{} recorded node(s) are not in this project: {named}",
                unresolved.len()
            ));
        }
        None
    }

    /// The brand legend for the current trace state.
    /// 当前追踪状态的标题图例。
    ///
    /// `LIVE` is only ever returned for [`TraceStatus::Loaded`]; the other two
    /// states say what is true instead of claiming a recording that is not there.
    /// 只有 [`TraceStatus::Loaded`] 才会返回 `LIVE`；另外两种状态陈述事实，而不是宣称一份并不
    /// 存在的记录。
    pub fn trace_legend(&self) -> &'static str {
        match self.trace_status {
            TraceStatus::Absent => "TRACE: none",
            TraceStatus::Loaded => "LIVE",
            TraceStatus::Mismatch { .. } => "TRACE mismatch",
        }
    }

    /// Whether an artifact supplied the values the DATA panel may draw.
    /// 是否有 artifact 提供了 DATA 面板可以绘制的数值。
    pub fn trace_is_live(&self) -> bool {
        matches!(self.trace_status, TraceStatus::Loaded)
    }

    /// The DATA panel's note about the trace state, empty when one is loaded.
    /// DATA 面板关于追踪状态的说明；已装入时为空白。
    ///
    /// A mismatch reason can list every unresolved node, so the panel shows a
    /// short form and the event line keeps the full one.
    /// Mismatch 原因可能列出每个未解析节点，因此面板只显示简短形式，完整形式留在事件行。
    pub fn trace_data_note(&self) -> String {
        match &self.trace_status {
            TraceStatus::Absent => "no trace attached".to_owned(),
            TraceStatus::Loaded => String::new(),
            TraceStatus::Mismatch { reason } => format!("trace mismatch: {}", shorten(reason, 48)),
        }
    }
}

/// The distinct recorded nodes this registry cannot resolve.
/// 本注册表无法解析的去重已记录节点。
///
/// Frames carry the ids directly; a local reaches its node through its frame, and
/// an edge through the frame of each endpoint local. A local whose frame is not in
/// the artifact, or an edge endpoint that names no local, contributes nothing,
/// because then the record has no node to resolve at all — `into_trace` already
/// refused the torn references it can see.
/// 调用帧直接携带 id；局部值经其调用帧到达节点，边经两个端点局部值各自的调用帧到达节点。局部值的
/// 调用帧不在 artifact 里、或边的端点没有局部值，都不贡献任何东西，因为此时该记录根本没有节点可
/// 解析——`into_trace` 已经拒绝了它看得见的断裂引用。
fn unresolved_nodes(artifact: &TraceArtifact, registry: &Registry) -> Vec<NodeId> {
    let frame_nodes: BTreeMap<u64, NodeId> = artifact
        .frames
        .iter()
        .map(|frame| (frame.frame_id, frame.node))
        .collect();
    let local_frames: BTreeMap<u64, Option<u64>> = artifact
        .locals
        .iter()
        .map(|local| (local.id, local.frame_id))
        .collect();
    let mut nodes: BTreeSet<NodeId> = artifact.frames.iter().map(|frame| frame.node).collect();
    for local in &artifact.locals {
        if let Some(node) = local.frame_id.and_then(|id| frame_nodes.get(&id)) {
            nodes.insert(*node);
        }
    }
    for edge in &artifact.edges {
        for endpoint in [edge.from, edge.to] {
            if let Some(node) = local_frames
                .get(&endpoint)
                .copied()
                .flatten()
                .and_then(|id| frame_nodes.get(&id))
            {
                nodes.insert(*node);
            }
        }
    }
    nodes
        .into_iter()
        .filter(|node| registry.find_registry(*node).is_none())
        .collect()
}

/// Shorten `text` to at most `limit` characters for a panel title.
/// 把 `text` 截到最多 `limit` 个字符，供面板标题使用。
fn shorten(text: &str, limit: usize) -> String {
    if text.chars().count() <= limit {
        return text.to_owned();
    }
    let mut shortened: String = text.chars().take(limit).collect();
    shortened.push('…');
    shortened
}
