//! The versioned `xirang.trace` artifact: a recorded `CallTrace` as text.
//! 带版本的 `xirang.trace` artifact：以文本保存的已记录 `CallTrace`。
//!
//! The document is line-oriented `key=value`, exactly like `graft.plan`: unknown
//! keys and other versions are refused rather than guessed. Record fields are
//! tab-separated and backslash-escaped, because a local value and an edge label
//! are arbitrary text. The module is a child of `trace` so it can read the
//! collector's `pub(in trace)` fields without widening any public field. The
//! parsing and file halves live in sibling files only to keep every file under
//! the repository's size ratchet.
//! 本文档与 `graft.plan` 一样是按行的 `key=value`：未知键与其他版本一律拒绝，而不是猜。
//! 记录字段以制表符分隔并做反斜杠转义，因为局部值与边标签是任意文本。本模块是 `trace` 的
//! 子模块，因此能读取收集器的 `pub(in trace)` 字段而不放宽任何公开字段。解析与文件两半放在
//! 同级文件里，只是为了把每个文件保持在仓库尺寸棘轮之下。

use std::fmt;

use crate::run_method::registry_core::declaration::SourceLocation;
use crate::run_method::registry_core::identity::{NodeId, root_node_id};
use crate::run_method::registry_core::lexicon::DEFAULT_NAMESPACE;

use super::call_trace::FrameRecord;
use super::{CallSite, CallTrace, DataEdge, LocalValue, TraceMode};

#[path = "io.rs"]
mod io;
#[path = "parse.rs"]
mod parse;

pub use self::io::*;

/// The only `xirang.trace` layout this build understands.
/// 本版本唯一能读懂的 `xirang.trace` 版式。
pub const TRACE_ARTIFACT_VERSION: u32 = 1;

/// One recorded frame, flattened for text.
/// 一个已记录调用帧的文本平铺形式。
///
/// `function` and `source` are interned while a document is parsed, so repeated
/// frames share one string per distinct name instead of leaking one per record.
/// `function` 与 `source` 在解析文档时被驻留，因此重复帧对每个不同名字只共享一个字符串，
/// 而不是每条记录泄漏一个。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TraceFrame {
    /// The trace-local frame id.
    /// 追踪内的帧 id。
    pub frame_id: u64,
    /// The enclosing frame id, `None` for a root frame.
    /// 外层调用帧 id；根帧为 `None`。
    pub parent: Option<u64>,
    /// The registry node the frame belongs to.
    /// 该帧所属的注册机节点。
    pub node: NodeId,
    /// The logical function active in the frame.
    /// 该帧中处于活动状态的逻辑函数名。
    pub function: &'static str,
    /// The callsite that entered the frame, when one is known.
    /// 进入该帧的调用点（若可知）。
    pub source: Option<SourceLocation>,
}

/// A recorded `CallTrace` as the versioned `xirang.trace` document.
/// 已记录 `CallTrace` 的带版本 `xirang.trace` 文档。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TraceArtifact {
    /// Layout version; must equal `TRACE_ARTIFACT_VERSION`.
    /// 版式版本；必须等于 `TRACE_ARTIFACT_VERSION`。
    pub version: u32,
    /// Host package namespace the artifact was recorded under.
    /// 记录该 artifact 时宿主包的命名空间。
    pub namespace: String,
    /// Registry root identity the recorded nodes were minted under.
    /// 铸造这些已记录节点时所用的注册机根身份。
    pub root: NodeId,
    /// The collection policy in force while recording.
    /// 记录期间生效的收集策略。
    pub mode: TraceMode,
    /// Recorded frames, in recording order.
    /// 已记录的调用帧，按记录顺序。
    pub frames: Vec<TraceFrame>,
    /// Recorded locals, in recording order.
    /// 已记录的局部值，按记录顺序。
    pub locals: Vec<LocalValue>,
    /// Recorded value edges, in recording order.
    /// 已记录的值边，按记录顺序。
    pub edges: Vec<DataEdge>,
}

/// Why a `xirang.trace` document was refused.
/// `xirang.trace` 文档被拒绝的原因。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TraceArtifactError {
    /// Document declares a layout this build cannot read.
    /// 文档声明的版式是本构建读不懂的。
    UnsupportedVersion(u32),
    /// Document omitted a required scalar key.
    /// 文档缺少一个必需的标量键。
    MissingKey(&'static str),
    /// Document carried a key this format does not define.
    /// 文档带有本格式未定义的键。
    UnknownKey(String),
    /// Document had a line, field count, or escape the format cannot read.
    /// 文档中存在本格式读不懂的行、字段数或转义。
    Malformed {
        /// 1-based line the failure was found on.
        /// 发现失败的行号，从 1 起。
        line: usize,
        /// What was wrong with that line.
        /// 该行错在哪里。
        message: String,
    },
    /// Two frames carry the same id.
    /// 两个调用帧使用了同一个 id。
    DuplicateFrame(u64),
    /// Two locals carry the same id.
    /// 两个局部值使用了同一个 id。
    DuplicateLocal(u64),
    /// A frame names a parent frame the document does not contain.
    /// 某个调用帧指名的父帧不在文档中。
    MissingParent(u64),
    /// An edge endpoint has no local in the document.
    /// 某条边的端点没有对应的局部值。
    DanglingEdge {
        /// The local id the edge leaves.
        /// 该边离开的局部值 id。
        from: u64,
        /// The local id the edge enters.
        /// 该边进入的局部值 id。
        to: u64,
    },
    /// This process has interned its budget of distinct trace strings.
    ///
    /// The table is process-level and never reclaimed — `function` and `file`
    /// are `&'static str`, so those strings cannot be freed while an artifact
    /// holds them — and the budget is what keeps that from being unbounded.
    /// Restarting the process is what clears it.
    /// 本进程已驻留满 trace 字符串预算。
    ///
    /// 这张表是进程级且永不回收的——`function` 与 `file` 都是 `&'static str`，因此只要有
    /// artifact 持有它们，那些字符串就释放不了——而这个预算正是让那件事不再无界的办法。
    /// 清空它的办法是重启进程。
    VocabularyExhausted {
        /// The table's ceiling, which is also its size when this is returned.
        /// 表的天花板；返回它时表的大小也正是这个数。
        limit: usize,
    },
}

impl fmt::Display for TraceArtifactError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedVersion(version) => write!(
                formatter,
                "trace artifact version `{version}` is not supported (expected {TRACE_ARTIFACT_VERSION})"
            ),
            Self::MissingKey(key) => write!(formatter, "trace artifact is missing `{key}`"),
            Self::UnknownKey(key) => write!(formatter, "trace artifact has an unknown key `{key}`"),
            Self::Malformed { line, message } => {
                write!(formatter, "trace artifact line {line}: {message}")
            }
            Self::DuplicateFrame(id) => write!(formatter, "trace artifact repeats frame id `{id}`"),
            Self::DuplicateLocal(id) => write!(formatter, "trace artifact repeats local id `{id}`"),
            Self::MissingParent(id) => {
                write!(
                    formatter,
                    "trace artifact frame parent `{id}` is not recorded"
                )
            }
            Self::DanglingEdge { from, to } => {
                write!(
                    formatter,
                    "trace artifact edge `{from} -> {to}` has no local"
                )
            }
            Self::VocabularyExhausted { limit } => write!(
                formatter,
                "this process has interned its budget of {limit} distinct trace strings, and the \
                 table is never reclaimed; restart the process to read a vocabulary this large"
            ),
        }
    }
}

impl std::error::Error for TraceArtifactError {}

impl TraceArtifact {
    /// Flatten a recorded trace into the artifact document.
    /// 把已记录的追踪平铺成 artifact 文档。
    ///
    /// The namespace and root anchors default to `DEFAULT_NAMESPACE`; the file
    /// writer re-stamps them from the process environment, because only the host
    /// that owns the package knows its name.
    /// namespace 与 root 锚点默认取 `DEFAULT_NAMESPACE`；文件写入方会用进程环境重新盖戳，
    /// 因为只有拥有该包的宿主才知道它的名字。
    pub fn from_trace(trace: &CallTrace) -> Self {
        Self {
            version: TRACE_ARTIFACT_VERSION,
            namespace: DEFAULT_NAMESPACE.to_owned(),
            root: root_node_id(DEFAULT_NAMESPACE),
            mode: trace.mode,
            frames: trace
                .frames
                .iter()
                .map(|frame| TraceFrame {
                    frame_id: frame.call.frame_id,
                    parent: frame.parent,
                    node: frame.call.node,
                    function: frame.call.function,
                    source: frame.call.source,
                })
                .collect(),
            locals: trace.locals.clone(),
            edges: trace.edges.clone(),
        }
    }

    /// Render the canonical document, one `key=value` line per record.
    /// 渲染规范文档，每条记录一行 `key=value`。
    pub fn render_trace_artifact(&self) -> String {
        let mut output = format!(
            "version={}\nnamespace={}\nroot={}\nmode={}\n",
            self.version,
            escape(&self.namespace),
            self.root,
            mode_spelling(self.mode),
        );
        for frame in &self.frames {
            let parent = frame
                .parent
                .map_or_else(|| "-".to_owned(), |id| id.to_string());
            let (file, line, column) = source_fields(frame.source);
            output.push_str(&format!(
                "frame={}\t{}\t{}\t{}\t{}\t{}\t{}\n",
                frame.frame_id,
                parent,
                frame.node,
                escape(frame.function),
                file,
                line,
                column
            ));
        }
        for local in &self.locals {
            let frame = local
                .frame_id
                .map_or_else(|| "-".to_owned(), |id| id.to_string());
            output.push_str(&format!(
                "local={}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
                local.id,
                frame,
                local.kind.label(),
                local.observation.label(),
                escape(&local.name),
                escape(&local.type_name),
                escape(&local.value),
                escape(local.source.file),
                local.source.line,
                local.source.column
            ));
        }
        for edge in &self.edges {
            let (file, line, column) = source_fields(edge.source);
            output.push_str(&format!(
                "edge={}\t{}\t{}\t{}\t{}\t{}\n",
                edge.from,
                edge.to,
                escape(&edge.label),
                file,
                line,
                column
            ));
        }
        output
    }

    /// Rebuild the trace these flat arenas describe, indexes included.
    /// 重建这些平铺 arena 所描述的追踪，含全部索引。
    ///
    /// Every reference is checked first: duplicate ids, a frame whose parent is
    /// absent, and an edge whose endpoint has no local are refused rather than
    /// silently producing a trace that resolves to nothing.
    /// 先检查每一条引用：重复 id、父帧缺失的帧、端点没有局部值的边都会被拒绝，而不是静默产出
    /// 一条什么都解析不到的追踪。
    pub fn into_trace(self) -> Result<CallTrace, TraceArtifactError> {
        let mut frame_ids = std::collections::BTreeSet::new();
        for frame in &self.frames {
            if !frame_ids.insert(frame.frame_id) {
                return Err(TraceArtifactError::DuplicateFrame(frame.frame_id));
            }
        }
        for frame in &self.frames {
            if let Some(parent) = frame.parent
                && !frame_ids.contains(&parent)
            {
                return Err(TraceArtifactError::MissingParent(parent));
            }
        }
        let mut local_ids = std::collections::BTreeSet::new();
        for local in &self.locals {
            if !local_ids.insert(local.id) {
                return Err(TraceArtifactError::DuplicateLocal(local.id));
            }
        }
        for edge in &self.edges {
            if !local_ids.contains(&edge.from) || !local_ids.contains(&edge.to) {
                return Err(TraceArtifactError::DanglingEdge {
                    from: edge.from,
                    to: edge.to,
                });
            }
        }
        let next_frame_id = self
            .frames
            .iter()
            .map(|frame| frame.frame_id)
            .max()
            .map_or(0, |id| id.wrapping_add(1));
        let next_local_id = self
            .locals
            .iter()
            .map(|local| local.id)
            .max()
            .map_or(0, |id| id.wrapping_add(1));
        let frames: Vec<FrameRecord> = self
            .frames
            .iter()
            .map(|frame| FrameRecord {
                call: CallSite {
                    node: frame.node,
                    function: frame.function,
                    frame_id: frame.frame_id,
                    source: frame.source,
                },
                parent: frame.parent,
            })
            .collect();
        let mut trace = CallTrace::with_mode(self.mode);
        trace.frames = frames;
        trace.locals = self.locals;
        trace.edges = self.edges;
        trace.next_frame_id = next_frame_id;
        trace.next_local_id = next_local_id;
        trace.rebuild_indexes();
        Ok(trace)
    }
}

/// The canonical spelling of a collection policy in the document.
/// 收集策略在文档中的规范拼写。
fn mode_spelling(mode: TraceMode) -> &'static str {
    match mode {
        TraceMode::Off => "off",
        TraceMode::ErrorsOnly => "errors-only",
        TraceMode::Full => "full",
    }
}

/// The `(file, line, column)` fields of an optional source location.
/// 可选源码位置的 `(file, line, column)` 字段。
fn source_fields(source: Option<SourceLocation>) -> (String, String, String) {
    match source {
        Some(source) => (
            escape(source.file),
            source.line.to_string(),
            source.column.to_string(),
        ),
        None => ("-".to_owned(), "-".to_owned(), "-".to_owned()),
    }
}

/// Escape a value so it stays on one tab-separated record.
/// 转义一个取值，使它留在同一条制表符分隔的记录里。
fn escape(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '\\' => output.push_str("\\\\"),
            '\t' => output.push_str("\\t"),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            other => output.push(other),
        }
    }
    output
}

#[cfg(test)]
#[path = "snapshot_tests.rs"]
mod snapshot_tests;
