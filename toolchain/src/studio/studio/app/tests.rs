//! App interaction regression tests.
//! App 交互回归测试。
//!
//! `app` mounts this file as `mod tests`; the root keeps the shared prelude
//! and fans the groups out to sibling files by concern so each stays
//! reviewable on its own.
//! `app` 把本文件挂载为 `mod tests`；根保留共享前导，并按关注点把测试组
//! 分散到同级文件，使每组都能独立审阅。

// Shared prelude re-exported for the mounted test submodules.
// 为挂载的测试子模块重新导出的共享前导。
pub(super) use std::time::{SystemTime, UNIX_EPOCH};

pub(super) use crossterm::event::{KeyCode, KeyEvent, MouseButton, MouseEventKind};
pub(super) use ratatui::layout::Rect;

pub(super) use super::cargo_probe::cargo_rustc_mir;
pub(super) use super::namespace::namespace_for;
pub(super) use super::project_context::{
    clear_project_context, host_manifest, package_namespace, package_root, resolve_project,
    resolve_project_from, select_project, with_authoring_context,
};
pub(super) use super::write_guard::{selected_package_root, with_selected_project};
pub(super) use super::{
    AddState, App, GraftDeclaration, Overlay, PartitionState, StudioPage, TraceStatus,
    app_function_source_range, body_calls, function_bodies, function_symbols,
};
// `SearchState` is named by the fixture-gated call-tree tests only, so it is
// imported with them: the default build refuses an unused import.
// `SearchState` 只被门控在夹具上的调用树测试命名，因此与它们一起导入：默认构建拒绝未使用的
// 导入。
#[cfg(feature = "prototype-fixtures")]
pub(super) use super::SearchState;
// The fixture helper exists only with the feature that names the fixture.
// 该夹具助手只在命名该夹具的特性下存在。
/// The node-editor fixture host, when this checkout has it.
/// node-editor 夹具宿主，当本检出有它时。
///
/// The fixture is a nested package, and cargo does not put a nested package into a
/// published `.crate`: a consumer who runs `cargo test --all-features` on the
/// published `xirang-toolchain` has no fixture to index. The contract
/// `prototype-fixtures` names is a *checkout* contract, so the tests that need it
/// skip when it is absent instead of panicking. It is always present here.
/// 该夹具是嵌套包，而 cargo 不会把嵌套包放进发布的 `.crate`：在已发布 `xirang-toolchain` 上跑
/// `cargo test --all-features` 的消费者没有夹具可索引。`prototype-fixtures` 命名的契约是**检出**
/// 契约，因此需要它的测试在夹具缺席时跳过而不是 panic；本检出里它始终存在。
/// The node-editor fixture loaded as a session: the one body every test that needs a
/// real host shares, so the copies cannot drift apart (audit `STU-S-22`).
/// node-editor 夹具按会话加载：所有需要真实宿主的测试共用这一份，因此各份拷贝不会各自漂移
/// （审计 `STU-S-22`）。
#[cfg(feature = "prototype-fixtures")]
pub(super) fn fixture_project() -> Option<std::path::PathBuf> {
    let fixture = node_editor_fixture()?;
    select_project(
        fixture.clone(),
        fixture.join("Cargo.toml"),
        "xirang.fixture.node-editor",
    );
    Some(fixture)
}

/// That fixture as a loaded session, for tests that need the registration tree.
/// 同一个夹具作为已加载会话，给需要注册树的测试。
#[cfg(feature = "prototype-fixtures")]
pub(super) fn fixture_app() -> Option<App> {
    fixture_project()?;
    Some(App::load_app())
}

#[cfg(feature = "prototype-fixtures")]
pub(super) fn node_editor_fixture() -> Option<std::path::PathBuf> {
    let root =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/node-editor");
    if root.join("Cargo.toml").is_file() {
        Some(root)
    } else {
        eprintln!(
            "skipping: the node-editor fixture is a checkout fixture and is not in this package"
        );
        None
    }
}

// The prototype-fixture tests read call relations and build a compiler snapshot
// directly, so they name these instead of reaching them through `app`'s private
// imports. Gated with the tests that use them: with the feature off they would be
// unused imports, which the `-D warnings` gate refuses. The call-tree tests are
// fixture-gated too — a call graph needs a project with one — so the names they
// bring in stay in this group.
// 原型夹具测试直接读取调用关系并构造编译器快照，因此它们直接命名这些名字，而不是经由
// `app` 的私有导入取得。门控与使用它们的测试相同：特性关闭时它们就是未使用导入，会被
// `-D warnings` 门禁拒绝。调用树测试同样门控在夹具上——调用图需要一个有图的工程——因此
// 它们引入的名字留在这一组。
#[cfg(feature = "prototype-fixtures")]
pub(super) use super::{CallRef, CallTreeView, same_symbol, source_path_for};
#[cfg(feature = "prototype-fixtures")]
pub(super) use crate::call_evidence::{CallEvidence, MirGraph};

// The call-tree tests read a real call graph, so they need the fixture project.
// A "the source must not say this" pin has to survive a rewrite that only changes case or
// spacing, or the absence check can be bypassed (audit `N-6`; `M7` escaped exactly this way).
// The positive half keeps plain `contains`: loosening a presence check would make it weaker,
// not stronger — so the two halves of a table-driven pin are deliberately read differently.
// "源码里不该出现这个"这类钉子必须能扛住"只改大小写或空白"的重写，否则缺席判据会被绕过（审计
// `N-6`；`M7` 就是这样逃过去的）。正例那一半继续用普通 `contains`：对"存在"判据放宽只会让它更弱
// 而不是更强——因此同一张表驱动的钉子，两半故意按不同的方式读。
pub(crate) fn flattened(source: &str) -> String {
    source
        .chars()
        .filter(|character| !character.is_whitespace())
        .flat_map(char::to_lowercase)
        .collect()
}

// 调用树测试要读真实调用图，因此需要夹具项目。
#[cfg(feature = "prototype-fixtures")]
#[path = "tests/call_tree.rs"]
mod call_tree;
#[path = "tests/edit.rs"]
mod edit;
// The evidence observation classifies the edges of a real project tree, so it is
// mounted with the fixture tests that load one.
// 证据观察要对一棵真实工程树的边分级，因此与加载工程树的夹具测试一同挂载。
#[cfg(feature = "prototype-fixtures")]
#[path = "tests/evidence.rs"]
mod evidence;
// The live trace the prototype-fixture tests install: mounted only with them, so
// the default build carries neither the helper nor its imports.
// 原型夹具测试安装的实时追踪：只在启用它们时挂载，因此默认构建既不携带该辅助函数，
// 也不携带它的导入。
#[cfg(feature = "prototype-fixtures")]
#[path = "tests/fixtures.rs"]
mod fixtures;
// The B4-studio batch's own pins: module docs against their implementation, the
// inspector's single row list, the editor failure that survives the graft banner,
// the plugin entry gate, evidence re-judged after a snapshot swap, and the search
// memo. One file because they share one subject — that batch's findings.
// B4-studio 批次自己的钉子：模块文档与实现、检视器唯一一份行清单、活过 graft 横幅的编辑器
// 失败、插件入口闸门、快照替换后重新裁决的证据、搜索备忘。合成一个文件是因为它们共享一个主语
// ——该批次的条目。
#[path = "tests/b4_studio.rs"]
mod b4_studio;
// The B5-studio code half's pins: the graph page's arrow vocabulary and its refusal of
// every other key, the one entry point that opens the editor form, the source stamp
// living outside navigation, the write guard's name, and the stamp/watcher mirror.
// B5-studio 代码半的钉子：调用图页的方向键词汇与其对其它键的拒绝、打开编辑表单的唯一入口、
// 住在导航之外的源码戳、写入守卫的名字，以及戳/watcher 的镜像。
#[path = "tests/b5_studio.rs"]
mod b5_studio;
// The B6-studio batch's pins: the consistency cleanups of that batch — root-only key
// feedback, the single modal hot-zone reset, named row indices, the graph cache's
// source stamp, both plugin locks, the removed reserved rectangles, and the graft
// reader's name.
// B6-studio 批次自己的钉子：该批次的一致性清扫——根上按键的反馈、模态热区唯一一处复位、具名
// 行下标、调用图缓存的源码戳、两个插件锁、删除的预留矩形，以及 graft 读取器的名字。
#[path = "tests/authoring_language.rs"]
mod authoring_language;
#[path = "tests/b6_studio.rs"]
mod b6_studio;
#[path = "tests/forms.rs"]
mod forms;
#[path = "tests/graft.rs"]
mod graft;
// The graft screen's own tests used to sit beside `app/graft.rs` as
// `graft_tests.rs`; they live here now, like every other mounted test file.
// graft 界面自己的测试过去以 `graft_tests.rs` 待在 `app/graft.rs` 旁边；现在它们和其余测试
// 文件一样挂在这里（审计 `STU-S-23`）。
#[path = "tests/graft_records.rs"]
mod graft_records;
#[path = "tests/graph.rs"]
mod graph;
#[path = "tests/mir_target.rs"]
mod mir_target;
#[path = "tests/navigation.rs"]
mod navigation;
// The partition screen's pins: what it opens on, and the three actions it shares with
// `xirang crates` (audit `M7`, P3.6).
// 分区屏的钉子：它打开时看到什么，以及它与 `xirang crates` 共用的三个动作（审计 `M7`，P3.6）。
#[path = "tests/partition.rs"]
mod partition;
// The old `tests/project.rs` held four subjects; it is now three files, each named
// for what it covers: the project root/manifest/namespace, the new-project wizard,
// and the MIR target resolver.
// 旧的 `tests/project.rs` 装四件事；现在是三个文件，各按它测的东西命名：项目根/清单/命名空间、
// 新项目向导、MIR target 解析器。
#[path = "tests/new_project.rs"]
mod new_project;
#[path = "tests/project_root.rs"]
mod project_root;
#[path = "tests/source.rs"]
mod source;
// Trace ingest builds its own temp host project, so it carries no fixture gate:
// the default `cargo test -p xirang-toolchain` exercises the loader too.
// trace ingest 自建临时宿主工程，因此不门控在夹具上：默认的
// `cargo test -p xirang-toolchain` 也会跑这套加载方测试。
#[path = "tests/trace_ingest.rs"]
mod trace_ingest;
