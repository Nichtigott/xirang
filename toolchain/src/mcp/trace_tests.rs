//! Tests for the trace report: absence is actionable, a foreign artifact is
//! refused, and a matching one renders.
//! trace 报告的测试：缺失要可行动，异树 artifact 要被拒绝，匹配的要能渲染。

use std::path::{Path, PathBuf};

use crate::build_method::face_views;
use crate::run_method::{CallTrace, LocalKind, trace_artifact_path, write_trace_artifact};
use serde_json::json;

use super::trace;

/// A throwaway package with one hand-written root face, plus that face's identity.
/// 一个含一个手写根面的一次性包，外加该面的身份。
fn package(label: &str) -> (PathBuf, String, xirang_kernel::identity::NodeId) {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let name = format!("mcp-trace-{label}");
    let root = std::env::temp_dir().join(format!("{name}-{}-{sequence}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("src/button")).expect("package directory");
    std::fs::write(
        root.join("Cargo.toml"),
        format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n"),
    )
    .expect("manifest");
    std::fs::write(root.join("src/lib.rs"), "// host entry\n").expect("library target");
    std::fs::write(
        root.join("src/button/button.rs"),
        "pub struct Button;\n\ncrate::root_object! {\n    kind: Button,\n    parent: crate::root_node_id(env!(\"CARGO_PKG_NAME\")),\n}\n",
    )
    .expect("face");
    let id = face_views(&root, &name).expect("faces derive")[0].id;
    (root, name, id)
}

/// Record one frame into an artifact, the way a host would.
/// 像宿主那样把一个帧记进 artifact。
fn record(root: &Path, namespace: &str, node: xirang_kernel::identity::NodeId) {
    let mut recorded = CallTrace::full();
    recorded.with(node, "button", |recorded| {
        recorded.local("input", "u32", 1, LocalKind::Input);
    });
    write_trace_artifact(&recorded, &trace_artifact_path(root), namespace)
        .expect("the artifact writes");
}

/// Record a run that captured values inside its frame, connected two of them, and
/// captured one before any traced call.
/// 记录一次运行：在帧内捕获了值、把其中两个连了起来，并在任何被追踪调用之前捕获了一个。
fn record_values(root: &Path, namespace: &str, node: xirang_kernel::identity::NodeId) {
    let mut recorded = CallTrace::full();
    recorded.local("seed", "u8", 7, LocalKind::Binding);
    recorded.with(node, "button", |recorded| {
        let input = recorded.local("input", "u32", 1, LocalKind::Input);
        let doubled = recorded.transform(input, "doubled", "u32", 2);
        recorded.consume(doubled, "render", "count");
    });
    write_trace_artifact(&recorded, &trace_artifact_path(root), namespace)
        .expect("the artifact writes");
}

/// The call report says what ran; `values: true` says what it saw, and it keeps the
/// two kinds of local apart — the one inside the frame and the one captured before
/// any traced call.
/// 调用报告说跑了什么；`values: true` 说它看见了什么，并把两类局部值分开——帧内的那个、以及在任何
/// 被追踪调用之前捕获的那个。
#[test]
fn recorded_values_are_reported_under_the_frame_that_captured_them() {
    let (root, name, id) = package("values");
    record_values(&root, &name, id);
    let reply = trace(&root, &json!({"values": true})).expect("the report renders");
    // Two counts, two labels, and the *values section's own* header — an exact line, because
    // the artifact header above it (`frames 1 locals 4 edges 2`) contains the same words and
    // would make a substring check pass on the unfixed build.
    // 两个计数、两个标签，而且必须是**值这一节自己的**表头——用整行精确匹配，因为它上面的
    // artifact 表头（`frames 1 locals 4 edges 2`）含同样的词，子串检查在未修复时会假通过。
    assert!(
        reply.lines().any(|line| line == "locals 4 edges 2"),
        "the values header is one label per number: {reply}"
    );
    assert!(
        !reply.contains("values 4 locals"),
        "two counts under three labels is gone: {reply}"
    );
    assert!(
        reply.contains("frame 0 button") && reply.contains("3 local(s)"),
        "the frame's own values are grouped under it: {reply}"
    );
    assert!(
        reply.contains("input: u32 = 1  [input, observed]"),
        "{reply}"
    );
    assert!(
        reply.contains("doubled: u32 = 2  [let, observed]"),
        "{reply}"
    );
    assert!(
        reply.contains("outside any traced frame"),
        "a value captured before any traced call is not folded into one: {reply}"
    );
    assert!(reply.contains("seed: u8 = 7  [let, observed]"), "{reply}");
    assert!(reply.contains("data edges 2"), "{reply}");
    assert!(reply.contains("input -> doubled  (transform)"), "{reply}");
    assert!(
        reply.contains("trace_tests.rs"),
        "every value carries the callsite that captured it: {reply}"
    );
    assert!(
        !reply.contains("<not recorded>"),
        "every edge end here was recorded: {reply}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// `query` narrows the values report the same way it narrows the call report, and
/// says so when nothing matched rather than showing an empty list.
/// `query` 与缩小调用报告时一样缩小值的报告，并且在没有命中时说出来，而不是给一份空清单。
#[test]
fn the_values_report_is_narrowed_by_the_query() {
    let (root, name, id) = package("values-query");
    record_values(&root, &name, id);
    let reply = trace(&root, &json!({"values": true, "query": "doubled"})).expect("the render");
    assert!(reply.contains("doubled: u32 = 2"), "{reply}");
    assert!(
        !reply.contains("  input: u32 = 1"),
        "the un-matched local is not shown: {reply}"
    );
    assert!(
        !reply.contains("  seed: u8 = 7"),
        "the un-matched value outside the frame is not shown either: {reply}"
    );
    let none = trace(
        &root,
        &json!({"values": true, "query": "zzz-no-such-value"}),
    )
    .expect("the render");
    assert!(
        none.contains("no recorded value matches the query"),
        "{none}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// No artifact is the normal state, and the answer must say how one appears.
/// 没有 artifact 是常态，答案必须说出它从哪来。
#[test]
fn an_absent_trace_names_the_way_to_produce_one() {
    let (root, _, _) = package("absent");
    let reply = trace(&root, &json!({})).expect("the report renders");
    assert!(reply.contains("trace absent"), "{reply}");
    assert!(reply.contains("trace_call!"), "{reply}");
    assert!(reply.contains("XIRANG_TRACE"), "{reply}");
    assert!(
        reply.contains("xirang new"),
        "the answer must point at a host that really records one: {reply}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// A trace recorded under this namespace, naming this face, renders a report.
/// 在本命名空间下记录、且点名本面的 trace，会渲染出一份报告。
#[test]
fn a_matching_artifact_renders_the_call_report() {
    let (root, name, id) = package("matching");
    record(&root, &name, id);
    let reply = trace(&root, &json!({})).expect("the report renders");
    assert!(reply.contains("frames 1"), "{reply}");
    assert!(!reply.contains("REFUSED"), "{reply}");
    let _ = std::fs::remove_dir_all(&root);
}

/// An artifact from another package is refused rather than rendered: its frames
/// are identities from a domain this tree does not share.
/// 来自另一个包的 artifact 被拒绝而不是被渲染：它的帧来自这棵树不共享的身份域。
#[test]
fn a_foreign_artifact_is_refused_by_name() {
    let (root, _, id) = package("foreign");
    record(&root, "some-other-package", id);
    let reply = trace(&root, &json!({})).expect("the refusal renders");
    assert!(reply.contains("REFUSED"), "{reply}");
    assert!(reply.contains("namespace"), "{reply}");
    let _ = std::fs::remove_dir_all(&root);
}

/// A frame naming a node this tree does not have is refused too, and the count is
/// named so the agent knows how much of the trace is unusable.
/// 帧点名的节点不在这棵树里时同样拒绝，并说出数量，让代理知道这份 trace 有多少不可用。
#[test]
fn a_frame_from_another_tree_is_refused_with_its_count() {
    let (root, name, _) = package("ghost-frame");
    let ghost =
        xirang_kernel::identity::NodeId::from_namespaced_path(&name, "ghost/ghost.rs", "Ghost");
    record(&root, &name, ghost);
    let reply = trace(&root, &json!({})).expect("the refusal renders");
    assert!(reply.contains("REFUSED"), "{reply}");
    assert!(reply.contains("frame(s) name nodes"), "{reply}");
    let _ = std::fs::remove_dir_all(&root);
}

/// A13: one artifact is **one recorded run**, and the report says so — a call the run never reached
/// is absent from it, and that absence is not evidence the call cannot happen. Without this line the
/// report reads as the complete binding, which is the claim the lexical side refuses to make.
/// A13：一份 artifact 就是**一次被记录的运行**，而报告会说出这一点——这次运行没走到的调用不在其中，而
/// 这种缺席**不是**"该调用不可能发生"的证据。少了这一行，报告就会被读成完整绑定，而那正是词法那一侧拒绝
/// 做出的声明。
#[test]
fn a_report_states_that_it_is_one_recorded_runs_coverage() {
    let (root, name, id) = package("coverage");
    record(&root, &name, id);
    let reply = trace(&root, &json!({})).expect("the report renders");
    assert!(reply.contains("frames 1"), "{reply}");
    assert!(
        reply.contains("coverage one recorded run"),
        "the report names what it covers: {reply}"
    );
    assert!(
        reply.contains("not evidence that a call cannot happen"),
        "and it says what absence here does not mean: {reply}"
    );
    assert!(
        reply.contains("xirang.callgraph"),
        "pointing at the lexical reading as its complement: {reply}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// A13's decision half: **nothing in this workspace turns one run into a verdict.** The trace
/// reports what one recorded run covered, and the surface that could be read as deciding behaviour —
/// `constraints` — answers `unknown` for a behavioural claim under either control direction. Pinning
/// them **together** is the point: either one alone can be read as "the other one decides it".
/// A13 的决策半边：**这个工作区里没有任何东西会把一次运行变成判决。** trace 报告一次被记录的运行覆盖了
/// 什么，而那个可能被读成"判定行为"的面——`constraints`——对行为性主张在两个控制方向下都答 `unknown`。
/// 把两者**一起**钉住才是要点：单独看任何一个，都可能被读成"另一个会替它判"。
#[test]
fn one_run_never_becomes_a_verdict() {
    let (root, name, id) = package("never-a-verdict");
    record(&root, &name, id);
    let report = trace(&root, &json!({})).expect("the report renders");
    assert!(
        report.contains("coverage one recorded run"),
        "the trace says what it covers: {report}"
    );

    // The same object, asked the behavioural question a trace would be misread as answering.
    // 同一个对象，被问到那个"trace 可能被误读成已回答"的行为问题。
    for control in ["positive", "negative"] {
        let answer = crate::mcp::constraints::constraints(
            &root,
            &json!({
                "node": id.to_string(),
                "expect": {"must_call": "root/provider"},
                "control": control
            }),
        )
        .expect("the tool answers");
        assert!(
            answer.starts_with("constraint unknown"),
            "{control}: a run's coverage is not a statement about what must happen: {answer}"
        );
        assert!(
            answer.contains("not a control"),
            "{control}: and it is not a passing control either: {answer}"
        );
    }
    let _ = std::fs::remove_dir_all(&root);
}

/// A trace from the same node set is still stale when the source it describes changed after it was
/// written. Node identity alone is not freshness.
/// 同一节点集合上的 trace 在源码于其后变化时仍然过期。只有节点身份不构成新鲜度。
#[test]
fn a_trace_older_than_its_source_is_refused_as_stale() {
    let (root, name, id) = package("stale-source");
    record(&root, &name, id);
    std::thread::sleep(std::time::Duration::from_millis(1100));
    std::fs::write(
        root.join("src/button/button.rs"),
        "pub struct Button;\n\ncrate::root_object! {\n    kind: Button,\n    parent: crate::root_node_id(env!(\"CARGO_PKG_NAME\")),\n}\n",
    )
    .expect("source changes");
    let reply = trace(&root, &json!({})).expect("the refusal renders");
    assert!(
        reply.contains("REFUSED") && reply.contains("stale:"),
        "{reply}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// The same applies when the host entry (the binding plan's source) changes after the trace.
/// 宿主入口（绑定计划的来源）在 trace 之后变化时同理。
#[test]
fn a_trace_older_than_its_binding_plan_is_refused_as_stale() {
    let (root, name, id) = package("stale-plan");
    record(&root, &name, id);
    std::thread::sleep(std::time::Duration::from_millis(1100));
    std::fs::write(root.join("src/lib.rs"), "// host entry changed\n").expect("entry changes");
    let reply = trace(&root, &json!({})).expect("the refusal renders");
    assert!(
        reply.contains("REFUSED") && reply.contains("stale:"),
        "{reply}"
    );
    let _ = std::fs::remove_dir_all(&root);
}
