//! Unit tests for the trace artifact document and its file half.
//! trace artifact 文档及其文件一半的单元测试。

use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use crate::run_method::runtime::trace::{LocalKind, Observation};

use super::*;

fn node(seed: u8) -> NodeId {
    NodeId::from_raw([seed; 16])
}

// A trace with two frames (one nested), three locals, and two edges.
// 一条含两个帧（其中一个嵌套）、三个局部值与两条边的追踪。
fn recorded_trace() -> CallTrace {
    let mut trace = CallTrace::full();
    trace.with(node(1), "root", |trace| {
        let input = trace.local("input", "u32", 1, LocalKind::Input);
        let binding = trace.transform(input, "binding", "u32", 2);
        trace.with(node(2), "child", |trace| {
            trace.local("inner", "&str", "hi", LocalKind::Binding);
        });
        trace.return_value(binding, "result", "u32", 2);
    });
    trace
}

fn frame(frame_id: u64, parent: Option<u64>) -> TraceFrame {
    TraceFrame {
        frame_id,
        parent,
        node: node(1),
        function: "f",
        source: None,
    }
}

fn local(id: u64, frame_id: Option<u64>) -> LocalValue {
    LocalValue {
        id,
        name: "v".to_owned(),
        type_name: "u32".to_owned(),
        value: "1".to_owned(),
        kind: LocalKind::Input,
        source: SourceLocation {
            file: "src/lib.rs",
            line: 1,
            column: 1,
            function: "f",
        },
        frame_id,
        observation: Observation::Observed,
    }
}

fn artifact_with(
    frames: Vec<TraceFrame>,
    locals: Vec<LocalValue>,
    edges: Vec<DataEdge>,
) -> TraceArtifact {
    TraceArtifact {
        version: TRACE_ARTIFACT_VERSION,
        namespace: "test".to_owned(),
        root: node(9),
        mode: TraceMode::Full,
        frames,
        locals,
        edges,
    }
}

fn fixture(label: &str) -> PathBuf {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "xirang-trace-{label}-{}-{sequence}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("fixture directory");
    root
}

#[test]
fn a_recorded_trace_round_trips_through_the_document() {
    let trace = recorded_trace();
    let artifact = TraceArtifact::from_trace(&trace);
    let text = artifact.render_trace_artifact();
    let parsed = TraceArtifact::parse_trace_artifact(&text).expect("the canonical document parses");
    assert_eq!(parsed, artifact);
    assert_eq!(parsed.mode, TraceMode::Full);
    assert_eq!(parsed.frames.len(), 2);
    assert_eq!(parsed.frames[1].parent, Some(0));

    let rebuilt = parsed
        .into_trace()
        .expect("a well-formed document rebuilds");
    assert_eq!(rebuilt.locals(), trace.locals());
    assert_eq!(rebuilt.data_edges(), trace.data_edges());
    assert_eq!(
        rebuilt.frame_ids().collect::<Vec<_>>(),
        trace.frame_ids().collect::<Vec<_>>()
    );
    assert_eq!(rebuilt.render_tree(), trace.render_tree());
}

#[test]
fn every_enum_spelling_and_escaped_value_survives() {
    let mut trace = CallTrace::full();
    trace.with(node(3), "spellings", |trace| {
        let input = trace.local("input", "String", "a\tb\nc\\d|e", LocalKind::Input);
        let binding = trace.transform(input, "binding", "String", "next");
        let consumer = trace.consume(binding, "f", "p");
        trace.return_value(consumer, "result", "String", "done");
        trace.inferred_local("spellings", "inferred", "u8", 7);
    });
    let artifact = TraceArtifact::from_trace(&trace);
    let text = artifact.render_trace_artifact();
    for spelling in ["\tlet\t", "\treturn\t", "\tconsumer\t", "\tunobserved\t"] {
        assert!(text.contains(spelling), "missing `{spelling}` in\n{text}");
    }
    assert!(text.contains("a\\tb\\nc\\\\d|e"), "{text}");

    let parsed = TraceArtifact::parse_trace_artifact(&text).expect("escaped values parse");
    assert_eq!(parsed.frames, artifact.frames);
    assert_eq!(parsed.locals.len(), artifact.locals.len());
    for (before, after) in artifact.locals.iter().zip(&parsed.locals) {
        assert_eq!(before.id, after.id);
        assert_eq!(before.name, after.name);
        assert_eq!(before.type_name, after.type_name);
        assert_eq!(before.value, after.value);
        assert_eq!(before.kind, after.kind);
        assert_eq!(before.observation, after.observation);
        assert_eq!(before.frame_id, after.frame_id);
        assert_eq!(before.source.file, after.source.file);
        assert_eq!(before.source.line, after.source.line);
        assert_eq!(before.source.column, after.source.column);
    }
    assert_eq!(
        parsed.locals[0].value, "a\tb\nc\\d|e",
        "the escaped value must survive byte for byte"
    );
    assert_eq!(parsed.locals[0].kind, LocalKind::Input);
    assert_eq!(parsed.locals[1].kind, LocalKind::Binding);
    assert_eq!(parsed.locals[2].kind, LocalKind::Consumer);
    assert_eq!(parsed.locals[3].kind, LocalKind::Output);
    assert_eq!(parsed.locals[4].observation, Observation::Unobserved);
}

/// The escaping rules are pinned against the values that break naive writers: a
/// trailing backslash (it must not escape the record separator), a backslash
/// followed by `n` (it must come back as two characters, not a newline), a lone
/// backslash, a carriage return, an empty value, quotes, and a multi-byte value.
/// 转义规则对着能击穿朴素写入方的取值钉死：结尾反斜杠（不能让它转义记录分隔符）、反斜杠加
/// `n`（必须回来成两个字符而不是换行）、单独一个反斜杠、回车、空值、引号，以及多字节值。
#[test]
fn the_values_that_break_naive_escaping_round_trip_exactly() {
    let cases = [
        "trailing\\",
        "back\\nslash",
        "\\",
        "carriage\rreturn",
        "",
        "quote\"and'apostrophe",
        "line\rs\nend",
        "中文值",
    ];
    let mut trace = CallTrace::full();
    trace.with(node(5), "escaping", |trace| {
        for (index, value) in cases.iter().enumerate() {
            trace.local(format!("v{index}"), "String", *value, LocalKind::Binding);
        }
    });
    let text = TraceArtifact::from_trace(&trace).render_trace_artifact();
    // No raw control character may reach the file. A carriage return is the one
    // that bites: it survives the round trip *inside* a field (Rust's `lines()`
    // only strips a CR that ends a line), so a round-trip assertion alone does not
    // pin the `\r` escape — but a raw CR in the data is exactly what a tool that
    // normalises line endings (git's `autocrlf`, an editor, a copy through a
    // Windows filesystem) can rewrite, and then the value changes. The record
    // terminator is the only newline-shaped byte this format may contain.
    // 文件里不允许出现裸控制字符。真正会咬人的是回车：字段**内部**的 CR 能活着往返
    // （Rust 的 `lines()` 只剥掉行尾的 CR），因此单靠往返断言钉不住 `\r` 转义——而数据里的
    // 裸 CR 正是那些会规范化行尾的工具（git 的 `autocrlf`、编辑器、经 Windows 文件系统的
    // 拷贝）可能改写的字节，改写之后取值就变了。本格式里唯一允许出现的换行形状字节是记录
    // 终止符。
    assert!(
        !text.contains('\r'),
        "a raw carriage return reached the artifact:\n{text}"
    );
    let parsed = TraceArtifact::parse_trace_artifact(&text).expect("every escaped value parses");
    for (index, value) in cases.iter().enumerate() {
        assert_eq!(
            parsed.locals[index].value, *value,
            "value {index} must survive byte for byte; rendered:\n{text}"
        );
    }
}

#[test]
fn parse_refuses_another_version_and_unknown_keys() {
    let text = TraceArtifact::from_trace(&recorded_trace()).render_trace_artifact();
    let version_two = text.replace("version=1", "version=2");
    assert_eq!(
        TraceArtifact::parse_trace_artifact(&version_two).expect_err("version 2 is refused"),
        TraceArtifactError::UnsupportedVersion(2)
    );
    let unknown = format!("{text}editor=vscode\n");
    assert_eq!(
        TraceArtifact::parse_trace_artifact(&unknown).expect_err("an unknown key is refused"),
        TraceArtifactError::UnknownKey("editor".to_owned())
    );
}

#[test]
fn parse_refuses_missing_and_repeated_scalar_keys() {
    let text = TraceArtifact::from_trace(&recorded_trace()).render_trace_artifact();
    let missing = text.replace("mode=full\n", "");
    assert_eq!(
        TraceArtifact::parse_trace_artifact(&missing).expect_err("a missing key is refused"),
        TraceArtifactError::MissingKey("mode")
    );
    let repeated = text.replace("mode=full\n", "mode=full\nmode=full\n");
    assert!(
        matches!(
            TraceArtifact::parse_trace_artifact(&repeated).expect_err("a repeated key is refused"),
            TraceArtifactError::Malformed { line: 5, .. }
        ),
        "a repeated scalar key is malformed"
    );
}

#[test]
fn parse_refuses_malformed_records() {
    let text = TraceArtifact::from_trace(&recorded_trace()).render_trace_artifact();
    let short_frame = text.replace("frame=0\t-\t", "frame=0\t");
    assert!(
        matches!(
            TraceArtifact::parse_trace_artifact(&short_frame)
                .expect_err("a short record is refused"),
            TraceArtifactError::Malformed { .. }
        ),
        "a record with the wrong field count is malformed"
    );
    let bad_kind = text.replace("\tinput\t", "\tmystery\t");
    assert!(
        matches!(
            TraceArtifact::parse_trace_artifact(&bad_kind).expect_err("an unknown kind is refused"),
            TraceArtifactError::Malformed { .. }
        ),
        "a local kind outside the vocabulary is malformed"
    );
    let bad_escape = artifact_with(vec![frame(0, None)], Vec::new(), Vec::new())
        .render_trace_artifact()
        .replace("namespace=test", "namespace=te\\qst");
    assert!(
        matches!(
            TraceArtifact::parse_trace_artifact(&bad_escape)
                .expect_err("an unknown escape is refused"),
            TraceArtifactError::Malformed { .. }
        ),
        "an escape the format does not define is malformed"
    );
}

#[test]
fn into_trace_refuses_broken_references() {
    let duplicate_frame =
        artifact_with(vec![frame(0, None), frame(0, None)], Vec::new(), Vec::new());
    assert_eq!(
        duplicate_frame
            .into_trace()
            .expect_err("duplicate frame ids are refused"),
        TraceArtifactError::DuplicateFrame(0)
    );

    let duplicate_local =
        artifact_with(Vec::new(), vec![local(0, None), local(0, None)], Vec::new());
    assert_eq!(
        duplicate_local
            .into_trace()
            .expect_err("duplicate local ids are refused"),
        TraceArtifactError::DuplicateLocal(0)
    );

    let missing_parent = artifact_with(vec![frame(0, Some(9))], Vec::new(), Vec::new());
    assert_eq!(
        missing_parent
            .into_trace()
            .expect_err("a missing parent frame is refused"),
        TraceArtifactError::MissingParent(9)
    );

    let dangling = artifact_with(
        vec![frame(0, None)],
        vec![local(0, Some(0))],
        vec![DataEdge {
            from: 0,
            to: 5,
            label: "transform".to_owned(),
            source: None,
        }],
    );
    assert_eq!(
        dangling
            .into_trace()
            .expect_err("an edge endpoint with no local is refused"),
        TraceArtifactError::DanglingEdge { from: 0, to: 5 }
    );
}

#[test]
fn into_trace_rebuilds_the_local_function_index() {
    let artifact = artifact_with(
        vec![frame(0, None)],
        vec![local(0, Some(0)), local(1, Some(0))],
        Vec::new(),
    );
    let trace = artifact
        .into_trace()
        .expect("a well-formed document rebuilds");
    let matched = trace.matching_locals("f");
    assert_eq!(
        matched.len(),
        2,
        "the rebuilt function index resolves locals"
    );
    assert!(trace.matching_locals("f").iter().any(|local| local.id == 0));
}

#[test]
fn from_trace_renders_the_same_document_twice() {
    let trace = recorded_trace();
    let first = TraceArtifact::from_trace(&trace).render_trace_artifact();
    let parsed = TraceArtifact::parse_trace_artifact(&first).expect("parses");
    let rebuilt = parsed.into_trace().expect("rebuilds");
    assert_eq!(
        TraceArtifact::from_trace(&rebuilt).render_trace_artifact(),
        first
    );
}

/// The override wins when it names a path, and the shared lexicon supplies the
/// convention path otherwise. The decision is tested without the process
/// environment, which is why it is a separate function.
/// 覆盖给出路径时以它为准，否则由共享词典给出约定路径。判断不依赖进程环境地测试，这正是它被拆成
/// 单独函数的原因。
#[test]
fn the_artifact_path_prefers_the_override_and_falls_back_to_the_lexicon() {
    let root = Path::new("/pkg");
    let conventional = Path::new("/pkg/.xirang/traces/xirang.trace");
    assert_eq!(resolve_artifact_path(root, None), conventional);
    assert_eq!(
        resolve_artifact_path(root, Some(OsStr::new(""))),
        conventional,
        "an empty override is not an override"
    );
    assert_eq!(
        resolve_artifact_path(root, Some(OsStr::new("elsewhere/run.trace"))),
        Path::new("/pkg/elsewhere/run.trace"),
        "a relative override resolves against the package root"
    );
    assert_eq!(
        resolve_artifact_path(root, Some(OsStr::new("/tmp/run.trace"))),
        Path::new("/tmp/run.trace"),
        "an absolute override is taken as written"
    );
}

#[test]
fn writing_and_reading_an_artifact_round_trips() {
    let root = fixture("round-trip");
    let path = trace_artifact_path(&root);
    std::fs::create_dir_all(path.parent().expect("parent")).expect("artifact directory");
    let trace = recorded_trace();
    write_trace_artifact(&trace, &path, "host-crate").expect("the artifact writes");

    let (artifact, rebuilt) = read_trace_artifact(&path).expect("the artifact reads");
    assert_eq!(artifact.version, TRACE_ARTIFACT_VERSION);
    assert_eq!(artifact.frames.len(), trace.frames.len());
    assert_eq!(rebuilt.locals(), trace.locals());
    assert_eq!(rebuilt.data_edges(), trace.data_edges());
    // The anchors come from the caller, not from the process: the namespace is the
    // compile-time identity the host's faces were stamped with, and the root is
    // derived from it. Reading either back out of the environment is what the
    // reader must not do, and what the writer cannot.
    // 锚点来自调用方而不是进程：命名空间是宿主的注册面被盖下的编译期身份，root 由它推出。
    // 从环境里把它们读回来，正是读取方不该做、写入方也做不到的事。
    assert_eq!(artifact.namespace, "host-crate");
    assert_eq!(
        artifact.root,
        NodeId::from_namespaced_path("host-crate", "<root>", "root")
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// The documented pair works on a fresh project: `trace_artifact_path` names a
/// file whose `.xirang/traces/` directory does not exist yet, and the writer
/// creates it. It used to fail with "No such file or directory", naming its own
/// temporary file instead of the missing directory.
/// 文档化的那一对在全新的项目里就能用：`trace_artifact_path` 指名的文件所在的
/// `.xirang/traces/` 还不存在，由写入方创建它。它过去会以 "No such file or directory" 失败，
/// 并且点名的是自己的临时文件而不是缺失的目录。
#[test]
fn writing_to_the_convention_path_creates_its_directory() {
    let root = fixture("fresh-root");
    let path = trace_artifact_path(&root);
    assert!(
        !path.parent().expect("parent").exists(),
        "the fixture must start without the traces directory"
    );
    let trace = recorded_trace();
    write_trace_artifact(&trace, &path, "host-crate").expect("the writer creates its directory");
    let (artifact, rebuilt) = read_trace_artifact(&path).expect("the artifact it wrote reads back");
    assert_eq!(artifact.namespace, "host-crate");
    assert_eq!(rebuilt.locals(), trace.locals());
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn reading_a_missing_artifact_is_a_named_error() {
    let root = fixture("missing");
    let path = root.join("absent.trace");
    let error = read_trace_artifact(&path).expect_err("a missing file is an error");
    assert!(error.contains("cannot read"), "{error}");
    assert!(error.contains("absent.trace"), "{error}");
    let _ = std::fs::remove_dir_all(&root);
}
