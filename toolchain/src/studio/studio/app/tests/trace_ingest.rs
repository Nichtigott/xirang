//! covers `app/trace.rs`, `app/source_index.rs`.
//! 覆盖 `app/trace.rs`、`app/source_index.rs`。
//!
//! End-to-end trace ingest: a host's artifact reaches the DATA panel or is refused.
//! 端到端 trace ingest：宿主的 artifact 到达 DATA 面板，否则被拒绝。
//!
//! The fixture is a temp host project with one generated control face, so the
//! artifact can be written to the convention path of the project this session
//! selected and every identity check has a real registry to resolve against. The
//! face's id is minted exactly as a host's macro mints it —
//! `NodeId::from_namespaced_path(package name, relative source path, kind)` — and
//! the test asserts that equality rather than hard-coding a hash.
//! 夹具是一个只含单个生成控制面的临时宿主工程，因此 artifact 可以写到本会话所选中项目的约定
//! 路径，而每项身份检查都有一个真实注册表可解析。该面的 id 完全按宿主宏的方式铸造——
//! `NodeId::from_namespaced_path(包名, 相对源码路径, kind)`——测试断言这条等式，而不是硬编码散列。

use std::path::PathBuf;

use crate::call_evidence::{CallTrace, LocalKind, SourceLocation};
use crate::run_method::{NodeId, TraceArtifact, root_node_id, trace_artifact_path};

use super::*;
// `CallRef` and `SearchState` are named by this file in every feature
// configuration, while the shared prelude imports them only for the
// fixture-gated tests, so they are named here directly.
// 本文件在每种特性配置下都会命名 `CallRef` 与 `SearchState`，而共享前导只为夹具门控的测试
// 导入它们，因此这里直接命名。
use super::super::{CallRef, SearchState};

/// The generated face's source, identical in shape to a host's authored module.
/// 生成面的源码，形状与宿主创作的模块完全一致。
const CONTROL_SOURCE: &str = "pub struct ControlRegistry;\n\ncrate::control_object! {\n    kind: ControlRegistry,\n    needs_registry: true,\n    parent: crate::ROOT_NODE_ID,\n    registry_rule_path: \"src/control/registry_rule/registry_rule.rs\",\n    registry_rule: crate::control::registry_rule::REGISTRATION_RULE,\n}\n";

/// The registration rule the generated face names.
/// 生成面所命名的注册规则。
const RULE_SOURCE: &str = "use crate::RegistrationRule;\npub const REGISTRATION_RULE: RegistrationRule = RegistrationRule::ANY;\n";

/// The function the recorded trace ran inside.
/// 已记录追踪运行所在的函数。
const FUNCTION: &str = "control_width";
/// The recorded input value.
/// 已记录的输入值。
const INPUT_VALUE: &str = "9000";
/// The recorded output value, distinct from the input so the panel cannot draw
/// one and pass for the other.
/// 已记录的输出值，与输入不同，因此面板画出其中一个时无法冒充另一个。
const OUTPUT_VALUE: &str = "1280";

/// A temp host project with one registered face.
/// 只含一个已注册面的临时宿主工程。
struct Fixture {
    root: PathBuf,
    name: String,
    node: NodeId,
    file: String,
}

impl Drop for Fixture {
    /// Remove the throwaway project even when a test panics.
    /// 即使测试 panic 也移除一次性工程。
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

impl Fixture {
    /// The path the loader and the host agree on for this project.
    /// 本项目的加载方与宿主共同约定的路径。
    fn artifact_path(&self) -> PathBuf {
        trace_artifact_path(&self.root)
    }

    /// Write an artifact at the convention path, exactly as the reader finds it.
    /// 在约定路径写出 artifact，与读取方找到它的方式完全一致。
    fn write_fixture(&self, artifact: &TraceArtifact) {
        let path = self.artifact_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("create the trace directory");
        }
        std::fs::write(&path, artifact.render_trace_artifact()).expect("write the artifact");
    }
}

/// Build the temp project, load it, and resolve the face the trace must name.
/// 建好临时工程、加载它，并解析出追踪必须指名的那个面。
fn fixture(label: &str) -> Fixture {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let root = std::env::temp_dir().join(format!("xirang-toolchain-trace-{label}-{suffix}"));
    let control = root.join("src/control/control.rs");
    let rule = root.join("src/control/registry_rule/registry_rule.rs");
    std::fs::create_dir_all(rule.parent().expect("rule parent")).expect("create fixture");
    std::fs::write(&control, CONTROL_SOURCE).expect("write control face");
    std::fs::write(&rule, RULE_SOURCE).expect("write rule");
    // A library target, because the fixture is asked about through Cargo now and
    // `cargo metadata` refuses a manifest with no target — which is what a real
    // host has anyway.
    // 一个库目标，因为本夹具现在要经 Cargo 询问，而 `cargo metadata` 拒绝没有目标的清单——真实
    // 宿主本来也有。
    std::fs::write(root.join("src/lib.rs"), "// host entry\n").expect("write library target");
    let manifest = root.join("Cargo.toml");
    std::fs::write(
        &manifest,
        format!("[package]\nname = \"ingest-{label}\"\nversion = \"0.1.0\"\n"),
    )
    .expect("write manifest");
    // The namespace comes from Cargo's answer for the manifest, exactly as a
    // launched session takes it, so this test pins the chain a host relies on.
    // 命名空间来自 Cargo 对该清单的回答，与已启动会话取得它的方式一致，因此本条测试钉住宿主所依赖
    // 的那条链。
    let name = crate::build_method::package_name(&manifest)
        .expect("the fixture manifest names its package");
    select_project(root.clone(), manifest, name.clone());
    let app = App::load_app();
    let face = app
        .registry
        .depth_first()
        .into_iter()
        .find(|info| info.source.file == "control/control.rs")
        .expect("the fixture face is registered");
    assert_eq!(
        NodeId::from_namespaced_path(&name, face.source.file.as_str(), &face.kind),
        face.id,
        "the face's id is the namespace + path + kind hash a host also mints"
    );
    Fixture {
        root,
        name,
        node: face.id,
        file: face.source.file.to_owned(),
    }
}

/// The recorded run: one input local and one returned output inside the face.
/// 已记录的一次运行：该面内一个输入局部值与一个返回的输出值。
fn recorded_trace(fixture: &Fixture) -> CallTrace {
    let source = SourceLocation {
        file: "control/control.rs",
        line: 1,
        column: 1,
        function: FUNCTION,
    };
    let mut trace = CallTrace::full();
    trace.with_at(fixture.node, FUNCTION, source, |trace| {
        let requested = trace.local("requested_width", "u32", INPUT_VALUE, LocalKind::Input);
        trace.return_value(requested, "clamped_width", "u32", OUTPUT_VALUE);
    });
    trace
}

/// A well-formed artifact for this project, before any identity is broken.
/// 为该项目构造的完好 artifact，尚未破坏任何身份。
fn matching_artifact(fixture: &Fixture, trace: &CallTrace) -> TraceArtifact {
    let mut artifact = TraceArtifact::from_trace(trace);
    artifact.namespace = fixture.name.clone();
    artifact.root = root_node_id(&fixture.name);
    artifact
}

/// The call reference `draw_data_flow_panel` resolves for the fixture's face.
/// `draw_data_flow_panel` 为夹具的面解析出的调用引用。
fn center(fixture: &Fixture) -> CallRef {
    CallRef {
        node: fixture.node,
        function: FUNCTION.to_owned(),
        file: fixture.file.clone(),
    }
}

/// Render the current page as the flat text a reader sees.
/// 把当前页面渲染成读者所见的平铺文本。
fn render_fixture(app: &mut App) -> String {
    let mut terminal =
        ratatui::Terminal::new(ratatui::backend::TestBackend::new(160, 48)).expect("terminal");
    crate::studio::studio::ui::draw_once(&mut terminal, app).expect("draw one frame");
    terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|cell| cell.symbol())
        .collect()
}

/// Render the base workspace, where the brand legend is not covered.
/// 渲染基础工作区；在那里标题图例不会被浮层遮住。
fn render_base(app: &mut App) -> String {
    app.overlay = None;
    render_fixture(app)
}

/// Render the provenance graph page for the fixture's face.
/// 为夹具的面渲染溯源图页面。
fn render_graph(app: &mut App, fixture: &Fixture) -> String {
    app.overlay = Some(Overlay::Search(SearchState {
        graph_mode: true,
        center: Some(fixture.node),
        center_function: Some(FUNCTION.to_owned()),
        ..SearchState::default()
    }));
    render_fixture(app)
}

/// Precondition for every refusal test: a matching artifact *would* load.
/// 每条拒绝测试的前提：匹配的 artifact **本会**装入。
fn matching_artifact_loads(fixture: &Fixture) -> App {
    let trace = recorded_trace(fixture);
    fixture.write_fixture(&matching_artifact(fixture, &trace));
    let app = App::load_app();
    assert_eq!(
        app.trace_status,
        TraceStatus::Loaded,
        "the matching control case must load first: {}",
        app.event
    );
    app
}

/// A matching artifact reaches the DATA panel: `graph_locals` — the call the
/// panel makes — returns the recorded values, and the page draws them without the
/// removed `built-in sample` label.
/// 匹配的 artifact 到达 DATA 面板：面板所调用的 `graph_locals` 返回已记录数值，而页面把它们
/// 画出来，且不再带已删除的 `built-in sample` 标注。
#[test]
fn a_matching_artifact_loads_its_values_into_the_data_panel() {
    let fixture = fixture("match");
    let mut app = matching_artifact_loads(&fixture);

    let locals = app.graph_locals(&center(&fixture));
    assert_eq!(
        locals
            .iter()
            .map(|local| local.name.as_str())
            .collect::<Vec<_>>(),
        vec!["requested_width", "clamped_width"],
        "{locals:?}"
    );
    assert_eq!(
        locals
            .iter()
            .map(|local| local.value.as_str())
            .collect::<Vec<_>>(),
        vec![INPUT_VALUE, OUTPUT_VALUE],
        "{locals:?}"
    );

    let rendered = render_graph(&mut app, &fixture);
    assert!(
        rendered.contains(INPUT_VALUE),
        "the input is drawn: {rendered}"
    );
    assert!(
        rendered.contains(OUTPUT_VALUE),
        "the output is drawn: {rendered}"
    );
    assert!(
        !rendered.contains("built-in sample"),
        "the sample label is gone once real values are drawn: {rendered}"
    );
}

/// The three states render their own legend and DATA-panel note, and `LIVE` is
/// never among them unless an artifact was loaded.
/// 三种状态各自渲染自己的图例与 DATA 面板说明，而除非装入了 artifact，`LIVE` 绝不在其中。
#[test]
fn the_three_trace_states_render_their_own_legend_and_panel_title() {
    let fixture = fixture("states");

    // Absent: nothing was found.
    // Absent：什么都没找到。
    let mut absent = App::load_app();
    assert_eq!(absent.trace_status, TraceStatus::Absent);
    let legend = render_base(&mut absent);
    assert!(legend.contains("TRACE: none"), "{legend}");
    assert!(!legend.contains("LIVE"), "{legend}");
    let panel = render_graph(&mut absent, &fixture);
    assert!(panel.contains("no trace attached"), "{panel}");

    // Loaded: the recorded values are drawn under a plain LIVE.
    // Loaded：已记录数值在朴素的 LIVE 之下被画出。
    let trace = recorded_trace(&fixture);
    fixture.write_fixture(&matching_artifact(&fixture, &trace));
    let mut loaded = App::load_app();
    assert_eq!(loaded.trace_status, TraceStatus::Loaded);
    let legend = render_base(&mut loaded);
    assert!(legend.contains("LIVE"), "{legend}");
    assert!(!legend.contains("TRACE: none"), "{legend}");
    let panel = render_graph(&mut loaded, &fixture);
    assert!(!panel.contains("no trace attached"), "{panel}");
    assert!(!panel.contains("built-in sample"), "{panel}");
    assert!(panel.contains(INPUT_VALUE), "{panel}");

    // Mismatch: a foreign artifact is named, and no value from it is drawn.
    // Mismatch：外来 artifact 被点名，且不画出它的任何数值。
    let mut foreign = matching_artifact(&fixture, &trace);
    foreign.namespace = "someone.elses.app".to_owned();
    foreign.root = root_node_id("someone.elses.app");
    fixture.write_fixture(&foreign);
    let mut refused = App::load_app();
    assert!(matches!(refused.trace_status, TraceStatus::Mismatch { .. }));
    let legend = render_base(&mut refused);
    assert!(legend.contains("TRACE mismatch"), "{legend}");
    assert!(!legend.contains("LIVE"), "{legend}");
    let panel = render_graph(&mut refused, &fixture);
    assert!(panel.contains("trace mismatch"), "{panel}");
    assert!(
        !panel.contains(INPUT_VALUE),
        "a refused artifact must not supply values: {panel}"
    );
}

/// A foreign namespace is refused by name, installs nothing, and the registry it
/// would have been drawn against stays visible.
/// 外来命名空间按名字被拒绝，不装入任何东西，而它本会被画在其上的注册表保持可见。
#[test]
fn a_foreign_namespace_is_refused_and_installs_nothing() {
    let fixture = fixture("namespace");
    let trace = recorded_trace(&fixture);
    let mut artifact = matching_artifact(&fixture, &trace);
    artifact.namespace = "someone.elses.app".to_owned();
    artifact.root = root_node_id("someone.elses.app");
    fixture.write_fixture(&artifact);

    let app = App::load_app();
    let TraceStatus::Mismatch { reason } = &app.trace_status else {
        panic!(
            "a foreign namespace must be refused: {:?}",
            app.trace_status
        );
    };
    assert!(reason.contains("someone.elses.app"), "{reason}");
    assert!(reason.contains(&fixture.name), "{reason}");
    assert!(
        !app.runtime_trace.is_collecting(),
        "nothing may be installed"
    );
    assert!(
        app.runtime_trace.locals().is_empty(),
        "no values may be installed"
    );
    assert!(
        app.registry.find_registry(fixture.node).is_some(),
        "the registry stays visible through a trace refusal"
    );
    assert!(app.event.contains("refused"), "{}", app.event);
}

/// A foreign root is refused even when the namespace agrees.
/// 即使命名空间一致，外来根也会被拒绝。
#[test]
fn a_foreign_root_is_refused_and_installs_nothing() {
    let fixture = fixture("root");
    let trace = recorded_trace(&fixture);
    let mut artifact = matching_artifact(&fixture, &trace);
    artifact.root = root_node_id("some-other-package");
    fixture.write_fixture(&artifact);

    let app = App::load_app();
    let TraceStatus::Mismatch { reason } = &app.trace_status else {
        panic!("a foreign root must be refused: {:?}", app.trace_status);
    };
    assert!(reason.contains("registry root"), "{reason}");
    assert!(
        reason.contains(&root_node_id("some-other-package").to_string()),
        "the reason names the foreign root: {reason}"
    );
    assert!(!app.runtime_trace.is_collecting());
    assert!(app.registry.find_registry(fixture.node).is_some());
}

/// A recorded node this snapshot cannot resolve is refused, and the reason names
/// it: this is the only real "different build" detector.
/// 本快照无法解析的已记录节点会被拒绝，且原因点名它：这是唯一真正的“不同构建”检测器。
#[test]
fn an_unresolved_recorded_node_is_refused_and_the_reason_names_it() {
    let fixture = fixture("stale");
    let trace = recorded_trace(&fixture);
    let mut artifact = matching_artifact(&fixture, &trace);
    let ghost = NodeId::from_namespaced_path(&fixture.name, "control/ghost.rs", "Ghost");
    artifact.frames[0].node = ghost;
    fixture.write_fixture(&artifact);

    let app = App::load_app();
    let TraceStatus::Mismatch { reason } = &app.trace_status else {
        panic!(
            "a node this snapshot lacks must be refused: {:?}",
            app.trace_status
        );
    };
    assert!(reason.contains("not in this project"), "{reason}");
    assert!(reason.contains(&ghost.to_string()), "{reason}");
    assert!(!app.runtime_trace.is_collecting());
    assert!(
        app.registry.find_registry(fixture.node).is_some(),
        "the registry stays visible through a stale trace"
    );
}

/// An unsupported document version is refused with the parser's own message, which
/// is the loader's only version check.
/// 不支持的文档版本按解析器自己的消息被拒绝，那也是加载方唯一的版本检查。
#[test]
fn an_unsupported_version_is_refused_with_the_parsers_reason() {
    let fixture = fixture("version");
    let path = fixture.artifact_path();
    std::fs::create_dir_all(path.parent().expect("artifact parent")).expect("trace directory");
    std::fs::write(&path, "version=2\nnamespace=x\nroot=y\nmode=full\n").expect("write artifact");

    let app = App::load_app();
    let TraceStatus::Mismatch { reason } = &app.trace_status else {
        panic!("version 2 must be refused: {:?}", app.trace_status);
    };
    assert!(reason.contains("version `2`"), "{reason}");
    assert!(reason.contains("not supported"), "{reason}");
    assert!(!app.runtime_trace.is_collecting());
    assert!(app.registry.find_registry(fixture.node).is_some());
}

/// The convention path is the one the host writes and Studio reads; the override
/// is `trace_artifact_path`'s rule, already pinned in `run_method`, so this only
/// checks the loader used it.
/// 约定路径就是宿主写出、Studio 读入的那条；覆盖规则属于 `trace_artifact_path`，已在
/// `run_method` 中钉住，因此这里只检查加载方走了它。
#[test]
fn the_loader_reads_the_convention_path() {
    let fixture = fixture("path");
    let trace = recorded_trace(&fixture);
    fixture.write_fixture(&matching_artifact(&fixture, &trace));
    let expected = fixture
        .root
        .join(crate::run_method::lexicon::XIRANG_DIR)
        .join(crate::run_method::lexicon::TRACE_DIR)
        .join(crate::run_method::lexicon::TRACE_FILE);
    assert_eq!(fixture.artifact_path(), expected);
    assert!(fixture.artifact_path().is_file());
    assert_eq!(App::load_app().trace_status, TraceStatus::Loaded);
}
