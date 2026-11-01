//! Parsing half of the trace artifact document.
//! trace artifact 文档的解析一半。
//!
//! Parsing is separate from the document type only to keep each file under the
//! repository's size ratchet. It interns every distinct function and file, so a
//! repeated name is leaked once rather than once per record — against a
//! process-level ceiling, because that table is never reclaimed (`intern` below).
//! 解析与文档类型分开，只是为了把每个文件保持在仓库尺寸棘轮之下。它对每个不同的函数名与文件名
//! 各驻留一次，因此重复的名字只泄漏一次，而不是每条记录一次。

use std::collections::BTreeMap;

use crate::run_method::registry_core::declaration::SourceLocation;
use crate::run_method::registry_core::identity::NodeId;
use crate::run_method::runtime::trace::{DataEdge, LocalKind, LocalValue, Observation, TraceMode};

use super::{TRACE_ARTIFACT_VERSION, TraceArtifact, TraceArtifactError, TraceFrame};

impl TraceArtifact {
    /// Parse a document, interning every distinct function and file once.
    /// 解析文档，对每个不同的函数名与文件名各驻留一次。
    ///
    /// Records may appear in any order, and the two references that look like they
    /// need an order — a local's source function comes from its frame, an edge's
    /// from its destination local — are resolved after the whole document has been
    /// read, so a forward reference resolves instead of silently becoming
    /// `<local>` / `<runtime>`. The canonical render writes frames, then locals,
    /// then edges.
    /// 记录可以任意顺序出现；两处看起来需要顺序的引用——局部值的来源函数取自它的帧，边的取自它的
    /// 目标局部值——在整份文档读完后补全，因此前向引用会解析出来，而不是静默变成
    /// `<local>` / `<runtime>`。规范渲染按帧、局部值、边的顺序写出。
    pub fn parse_trace_artifact(source: &str) -> Result<Self, TraceArtifactError> {
        let mut version = None;
        let mut namespace = None;
        let mut root = None;
        let mut mode = None;
        let mut frames: Vec<TraceFrame> = Vec::new();
        let mut locals: Vec<LocalValue> = Vec::new();
        let mut edges: Vec<DataEdge> = Vec::new();
        let mut frame_functions: BTreeMap<u64, &'static str> = BTreeMap::new();
        let mut local_functions: BTreeMap<u64, &'static str> = BTreeMap::new();
        for (index, raw) in source.lines().enumerate() {
            let line = raw.strip_suffix('\r').unwrap_or(raw);
            let at = index + 1;
            if line.trim().is_empty() || line.trim_start().starts_with('#') {
                continue;
            }
            let Some((key, value)) = line.split_once('=') else {
                return Err(malformed(at, "expected `key=value`"));
            };
            match key.trim() {
                "version" => {
                    if version.is_some() {
                        return Err(malformed(at, "duplicate key `version`"));
                    }
                    let parsed = value.trim().parse::<u32>().map_err(|_| {
                        malformed(at, format!("version `{}` is not a number", value.trim()))
                    })?;
                    if parsed != TRACE_ARTIFACT_VERSION {
                        return Err(TraceArtifactError::UnsupportedVersion(parsed));
                    }
                    version = Some(parsed);
                }
                "namespace" => {
                    if namespace.is_some() {
                        return Err(malformed(at, "duplicate key `namespace`"));
                    }
                    namespace = Some(unescape(value.trim(), at)?);
                }
                "root" => {
                    if root.is_some() {
                        return Err(malformed(at, "duplicate key `root`"));
                    }
                    root = Some(value.trim().parse::<NodeId>().map_err(|_| {
                        malformed(
                            at,
                            format!("root `{}` is not a node identity", value.trim()),
                        )
                    })?);
                }
                "mode" => {
                    if mode.is_some() {
                        return Err(malformed(at, "duplicate key `mode`"));
                    }
                    mode = Some(TraceMode::parse_trace_mode(value.trim()).ok_or_else(|| {
                        malformed(at, format!("mode `{}` is not a trace mode", value.trim()))
                    })?);
                }
                "frame" => {
                    let fields = split_fields(value, 7, at, "frame")?;
                    let frame_id = parse_u64(fields[0], at, "frame id")?;
                    let parent = parse_optional_id(fields[1], at, "frame parent")?;
                    let node = fields[2].parse::<NodeId>().map_err(|_| {
                        malformed(
                            at,
                            format!("frame node `{}` is not a node identity", fields[2]),
                        )
                    })?;
                    let function = intern(&unescape(fields[3], at)?)?;
                    let source =
                        parse_optional_source(fields[4], fields[5], fields[6], function, at)?;
                    frame_functions.insert(frame_id, function);
                    frames.push(TraceFrame {
                        frame_id,
                        parent,
                        node,
                        function,
                        source,
                    });
                }
                "local" => {
                    let fields = split_fields(value, 10, at, "local")?;
                    let id = parse_u64(fields[0], at, "local id")?;
                    let frame_id = parse_optional_id(fields[1], at, "local frame")?;
                    let kind = parse_local_kind(fields[2]).ok_or_else(|| {
                        malformed(
                            at,
                            format!("local kind `{}` is not a local kind", fields[2]),
                        )
                    })?;
                    let observation = parse_observation(fields[3]).ok_or_else(|| {
                        malformed(
                            at,
                            format!("observation `{}` is not an observation", fields[3]),
                        )
                    })?;
                    let function = frame_id
                        .and_then(|frame_id| frame_functions.get(&frame_id).copied())
                        .unwrap_or("<local>");
                    let source = SourceLocation {
                        file: intern(&unescape(fields[7], at)?)?,
                        line: parse_u32(fields[8], at, "local source line")?,
                        column: parse_u32(fields[9], at, "local source column")?,
                        function,
                    };
                    local_functions.insert(id, function);
                    locals.push(LocalValue {
                        id,
                        name: unescape(fields[4], at)?,
                        type_name: unescape(fields[5], at)?,
                        value: unescape(fields[6], at)?,
                        kind,
                        source,
                        frame_id,
                        observation,
                    });
                }
                "edge" => {
                    let fields = split_fields(value, 6, at, "edge")?;
                    let from = parse_u64(fields[0], at, "edge source")?;
                    let to = parse_u64(fields[1], at, "edge destination")?;
                    let function = local_functions.get(&to).copied().unwrap_or("<runtime>");
                    let source =
                        parse_optional_source(fields[3], fields[4], fields[5], function, at)?;
                    edges.push(DataEdge {
                        from,
                        to,
                        label: unescape(fields[2], at)?,
                        source,
                    });
                }
                other => return Err(TraceArtifactError::UnknownKey(other.to_owned())),
            }
        }
        // Resolution pass. Both references above are provisional: a local whose
        // frame comes later carries `<local>`, and an edge whose local comes later
        // carries `<runtime>`. Correcting them here — rather than re-scanning the
        // document — keeps the line number of the first malformed record exactly
        // where a single pass put it, while making the declared "any order" true
        // (audit `LGC-LG-49`). The `into_trace` integrity check does not look at
        // `function`, so a wrong name here would otherwise stay in the evidence.
        // 补全阶段。上面两处引用都是临时的：帧在后面的局部值带着 `<local>`，局部值在后面的边带着
        // `<runtime>`。在这里改正——而不是重扫文档——既不移动单遍解析给第一条畸形记录的行号，又让
        // 声明的"任意顺序"成立（审计 `LGC-LG-49`）。`into_trace` 的引用完整性检查不看
        // `function`，因此错误的函数名否则会一直留在证据里。
        for local in &mut locals {
            if let Some(frame_id) = local.frame_id
                && let Some(function) = frame_functions.get(&frame_id).copied()
            {
                local.source.function = function;
            }
        }
        local_functions.clear();
        for local in &locals {
            local_functions.insert(local.id, local.source.function);
        }
        for edge in &mut edges {
            if let Some(function) = local_functions.get(&edge.to).copied()
                && let Some(source) = edge.source.as_mut()
            {
                source.function = function;
            }
        }
        Ok(Self {
            version: version.ok_or(TraceArtifactError::MissingKey("version"))?,
            namespace: namespace.ok_or(TraceArtifactError::MissingKey("namespace"))?,
            root: root.ok_or(TraceArtifactError::MissingKey("root"))?,
            mode: mode.ok_or(TraceArtifactError::MissingKey("mode"))?,
            frames,
            locals,
            edges,
        })
    }
}

/// Split one record value into exactly `count` tab-separated fields.
/// 把一个记录取值切成恰好 `count` 个制表符分隔的字段。
fn split_fields<'a>(
    value: &'a str,
    count: usize,
    line: usize,
    record: &str,
) -> Result<Vec<&'a str>, TraceArtifactError> {
    let fields: Vec<&str> = value.split('\t').collect();
    if fields.len() != count {
        return Err(malformed(
            line,
            format!(
                "{record} expects {count} tab-separated fields, found {}",
                fields.len()
            ),
        ));
    }
    Ok(fields)
}

fn parse_u64(value: &str, line: usize, field: &str) -> Result<u64, TraceArtifactError> {
    value
        .parse()
        .map_err(|_| malformed(line, format!("{field} `{value}` is not a number")))
}

fn parse_u32(value: &str, line: usize, field: &str) -> Result<u32, TraceArtifactError> {
    value
        .parse()
        .map_err(|_| malformed(line, format!("{field} `{value}` is not a number")))
}

fn parse_optional_id(
    value: &str,
    line: usize,
    field: &str,
) -> Result<Option<u64>, TraceArtifactError> {
    if value == "-" {
        return Ok(None);
    }
    parse_u64(value, line, field).map(Some)
}

fn parse_optional_source(
    file: &str,
    line: &str,
    column: &str,
    function: &'static str,
    at: usize,
) -> Result<Option<SourceLocation>, TraceArtifactError> {
    if file == "-" {
        if line != "-" || column != "-" {
            return Err(malformed(
                at,
                "an absent source spells file, line, and column as `-`",
            ));
        }
        return Ok(None);
    }
    Ok(Some(SourceLocation {
        file: intern(&unescape(file, at)?)?,
        line: parse_u32(line, at, "source line")?,
        column: parse_u32(column, at, "source column")?,
        function,
    }))
}

fn parse_local_kind(token: &str) -> Option<LocalKind> {
    [
        LocalKind::Input,
        LocalKind::Binding,
        LocalKind::Output,
        LocalKind::Consumer,
    ]
    .into_iter()
    .find(|kind| kind.label() == token)
}

fn parse_observation(token: &str) -> Option<Observation> {
    [Observation::Observed, Observation::Unobserved]
        .into_iter()
        .find(|observation| observation.label() == token)
}

fn malformed(line: usize, message: impl Into<String>) -> TraceArtifactError {
    TraceArtifactError::Malformed {
        line,
        message: message.into(),
    }
}

/// Reverse the document's escaping, refusing an escape this format does not define.
/// 逆向执行文档的转义，拒绝本格式未定义的转义。
fn unescape(value: &str, line: usize) -> Result<String, TraceArtifactError> {
    let mut output = String::with_capacity(value.len());
    let mut characters = value.chars();
    while let Some(character) = characters.next() {
        if character != '\\' {
            output.push(character);
            continue;
        }
        match characters.next() {
            Some('\\') => output.push('\\'),
            Some('t') => output.push('\t'),
            Some('n') => output.push('\n'),
            Some('r') => output.push('\r'),
            Some(other) => return Err(malformed(line, format!("unknown escape `\\{other}`"))),
            None => return Err(malformed(line, "trailing backslash in an escaped field")),
        }
    }
    Ok(output)
}

/// The process-level table that gives equal text the same `&'static str`.
/// 给相同文本同一个 `&'static str` 的进程级表。
type InternTable = std::sync::Mutex<std::collections::BTreeSet<&'static str>>;

/// Every artifact this process parses shares this one table.
/// 本进程解析的每份 artifact 共用这一张表。
static INTERNER: InternTable = InternTable::new(std::collections::BTreeSet::new());

/// How many distinct strings this process will intern before it refuses more.
/// 本进程在拒绝更多之前会驻留的不同字符串上限。
///
/// The measured cost is about 83 bytes per distinct string (`VmRSS` growth of a
/// release probe: +8.3 MB over 100 000 fresh names), so this ceiling is a few
/// megabytes of process-lifetime memory. A single artifact's vocabulary — its
/// function and file names — is four to five orders of magnitude smaller; the
/// ceiling exists for the process that has been fed *many* vocabularies, which is
/// the long-lived bridge, and only restarting clears it.
/// 实测代价约为每个不同字符串 83 字节（release 探针的 `VmRSS` 增长：10 万个新名字 +8.3 MB），
/// 因此这个天花板是几兆字节的进程生命期内存。单份 artifact 的词表——它的函数名与文件名——比这个
/// 上限小四到五个数量级；这个上限是为"被喂过**许多**词表"的进程准备的，也就是长寿命的桥，而只有
/// 重启才能清空它。
const INTERN_LIMIT: usize = 1 << 17;

/// Intern one distinct string, reusing the same `&'static str` for equal text.
/// 驻留一个不同的字符串，相同文本复用同一个 `&'static str`。
///
/// `function` and `file` are `&'static str` (the kernel's `SourceLocation` and
/// this crate's public `TraceFrame` both say so), so a decoder that allocated per
/// record would leak once per record; this leaks once per distinct string
/// instead. **The bound is the process, not the artifact**: every artifact parsed
/// here shares one table, nothing is evicted, and no artifact's drop releases its
/// strings — the ceiling is `INTERN_LIMIT` distinct strings *ever*, after which a
/// parse is refused with `TraceArtifactError::VocabularyExhausted` rather than
/// growing further.
/// `function` 与 `file` 都是 `&'static str`（内核的 `SourceLocation` 与本 crate 公开的
/// `TraceFrame` 都这么写），因此按记录分配的解码器会按记录泄漏；这里改为每个不同字符串泄漏一次。
/// **上界是进程而不是 artifact**：这里解析过的每份 artifact 共用一张表，没有任何淘汰，也没有任何
/// artifact 的 drop 会释放它的字符串——天花板是 `INTERN_LIMIT` 个不同字符串，越过之后解析以
/// `TraceArtifactError::VocabularyExhausted` 被拒，而不是继续变大。
fn intern(value: &str) -> Result<&'static str, TraceArtifactError> {
    intern_into(&INTERNER, value, INTERN_LIMIT)
}

/// `intern` against an explicit table and ceiling, so the policy is testable
/// without allocating the production ceiling.
/// `intern` 对一张显式的表与天花板执行，因此这条策略不必分配出生产上限就能测试。
fn intern_into(
    table: &InternTable,
    value: &str,
    limit: usize,
) -> Result<&'static str, TraceArtifactError> {
    let mut interned = table.lock().unwrap_or_else(|poison| poison.into_inner());
    if let Some(existing) = interned.get(value).copied() {
        return Ok(existing);
    }
    if interned.len() >= limit {
        return Err(TraceArtifactError::VocabularyExhausted { limit });
    }
    let leaked: &'static str = Box::leak(value.to_owned().into_boxed_str());
    interned.insert(leaked);
    Ok(leaked)
}

/// The number of distinct strings this process has interned so far.
/// 本进程目前已驻留的不同字符串数量。
#[cfg(test)]
pub(crate) fn interned_len() -> usize {
    INTERNER
        .lock()
        .unwrap_or_else(|poison| poison.into_inner())
        .len()
}

#[cfg(test)]
#[path = "parse_tests.rs"]
mod parse_tests;
