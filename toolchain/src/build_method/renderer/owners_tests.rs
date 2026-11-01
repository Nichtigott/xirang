//! Pins for the facade's expression rewriting.
//! facade 表达式改写的钉子。

use std::path::Path;

use super::{emitted_grafts, graft_constructor, names_a_compiled_module, owned};
use crate::build_method::diagnostics::BuildDiagnostics;
use crate::build_method::registry_syntax::GraftSyntax;
use crate::build_method::renderer::ShapeRender;
use crate::build_method::static_plan::StaticFaceRecord;
use crate::build_method::test_support::temporary_directory;
use crate::build_method::{SourceScope, render_lib};
use xirang_kernel::registry_core::identity::NodeId;
use xirang_kernel::registry_core::syntax::{GraftExpressions, SyntaxLocation};

/// Facade mode names the crate that compiles each module; every other mode leaves expressions exactly
/// as the author wrote them (audit `M7`, §M7.33).
/// facade 模式点出编译每个模块的那个 crate；其余模式让表达式保持作者写下的样子（审计 `M7`，§M7.33）。
#[test]
fn facade_mode_names_the_crate_that_compiles_each_module() {
    let owners = [
        ("panel::frame".to_owned(), "fix_widgets".to_owned()),
        ("control".to_owned(), "fix".to_owned()),
    ];
    let facade = ShapeRender {
        cut_out: &[],
        only: None,
        mounts: &[],
        ancestors: &[],
        facade: true,
        owners: &owners,
    };
    // A typed graft expression loses its `crate::` and gains its owner. The longest matching owner
    // wins, so a face under a claim goes to the ghost that claims it.
    // 类型化切口表达式丢掉 `crate::`、换上属主。最长匹配者胜，因此认领之下的面归认领它的幽灵。
    assert_eq!(
        owned(&facade, "crate::panel::frame::widget::NODE_ID"),
        "fix_widgets::panel::frame::widget::NODE_ID"
    );
    // A bare module path — how the registration-rules block writes them — gains the same prefix.
    // 裸模块路径——注册规则块就是这么写的——加同一个前缀。
    assert_eq!(
        owned(&facade, "panel::frame::REGISTRATION"),
        "fix_widgets::panel::frame::REGISTRATION"
    );
    // A **registration rule** spells `<module>::REGISTRATION.registry_rule`: the whole expression has to
    // be matched against the owner table, because splitting at the last `::` yields
    // `panel::frame::REGISTRATION`, an owner nobody has (measured in the facade, §M7.38).
    // **注册规则**写的是 `<模块>::REGISTRATION.registry_rule`：整条表达式都要拿去匹配属主表，因为按最后一个
    // `::` 切会得到 `panel::frame::REGISTRATION`——一个谁都不是的属主（在 facade 里实测，§M7.38）。
    assert_eq!(
        owned(&facade, "panel::frame::REGISTRATION.registry_rule"),
        "fix_widgets::panel::frame::REGISTRATION.registry_rule"
    );
    assert_eq!(
        owned(&facade, "crate::panel::frame::widget::NODE_ID"),
        "fix_widgets::panel::frame::widget::NODE_ID"
    );

    // An external implementation crate is not part of this tree: it passes through untouched.
    // 项目外的实现 crate 不属于这棵树：原样通过。
    assert_eq!(
        owned(&facade, "control_button_graft::button_fast::NODE_ID"),
        "control_button_graft::button_fast::NODE_ID"
    );
    // A host module is owned by the host, because `crate::` inside the facade means the facade.
    // 宿主模块归宿主，因为 facade 里的 `crate::` 指的是 facade 自己。
    assert_eq!(
        owned(&facade, "crate::control::NODE_ID"),
        "fix::control::NODE_ID"
    );

    // Any other mode is the author's spelling, byte for byte.
    // 其余模式一律是作者写下的拼写，逐字节不变。
    let host = ShapeRender::whole();
    for expression in [
        "crate::panel::frame::widget::NODE_ID",
        "panel::frame::REGISTRATION",
        "control_button_graft::button_fast::NODE_ID",
    ] {
        assert_eq!(owned(&host, expression), expression);
    }
}

/// The face this fixture's graft cuts, as the plan records it.
/// 本夹具的 graft 所切的那个面，按规划记录的样子。
fn widget() -> StaticFaceRecord {
    StaticFaceRecord {
        written_as_literal: false,
        kind: "Face".to_owned(),
        preset: "Preset".to_owned(),
        parts: "Parts".to_owned(),
        id: NodeId::from_bytes(b"widget"),
        parent: NodeId::from_bytes(b"frame"),
        owns_registry: false,
        source: "panel/frame/widget/widget.rs".to_owned(),
        module: "panel::frame::widget".to_owned(),
    }
}

/// A typed graft entry is emitted only by a crate that compiles its cut face (audit `M7`, §M7.39).
/// 类型化 graft 条目只由编译其切口面的 crate 发射（审计 `M7`，§M7.39）。
///
/// The negative half is what a partition needs: the host hands `panel::frame` away, so its own
/// generated tree must not name `panel::frame::widget` any more — not in the modules, and not in the
/// graft table, which is the half that first end-to-end build with a graft inside a claim missed.
/// 否定的那一半正是分区需要的：宿主把 `panel::frame` 交出去，因此它自己的生成树不能再点名
/// `panel::frame::widget`——模块里不能，graft 表里也不能，而后者正是"含切口落在认领之内的首次端到端构建"
/// 漏掉的那一半。
#[test]
fn a_crate_emits_only_the_graft_entries_it_compiles() {
    let root = temporary_directory("graft-visibility");
    let graft = GraftSyntax {
        cfg: None,
        cut: "crate::panel::frame::widget::NODE_ID".to_owned(),
        cut_end: None,
        graft: "fast_widget::fast::NODE_ID".to_owned(),
        full: false,
        location: SyntaxLocation { line: 7, column: 8 },
        expressions: Some(GraftExpressions {
            cut: "crate::panel::frame::widget::NODE_ID".to_owned(),
            cut_end: None,
            graft: "fast_widget::fast::NODE_ID".to_owned(),
        }),
        spans: None,
    };
    let entries = std::slice::from_ref(&graft);
    let render = |faces: &[StaticFaceRecord]| {
        render_lib(
            &root,
            &[],
            &BuildDiagnostics::default(),
            &BuildDiagnostics::default(),
            &SourceScope {
                roots: None,
                reason: "test",
            },
            faces,
            entries,
            ShapeRender::whole(),
            Path::new("xirang_modules"),
            &mut Vec::new(),
        )
    };

    // The crate that compiles the cut face emits the entry the author wrote.
    // 编译切口面的 crate 发射作者写下的那条条目。
    let host = render(&[widget()]);
    assert!(
        host.contains(
            "StaticGraftCut::from_ids(crate::panel::frame::widget::NODE_ID, \
             fast_widget::fast::NODE_ID, false)"
        ),
        "{host}"
    );
    assert!(
        host.contains("assert_contract::<crate::panel::frame::widget::__Preset"),
        "{host}"
    );

    // The crate that handed that face away emits neither the entry nor its contract assertion — the
    // table is left empty rather than left naming a module this crate does not have.
    // 把那个面交出去的 crate 既不发射条目、也不发射契约断言——表留空，而不是留着一个本 crate 没有的模块名。
    let elsewhere = render(&[]);
    assert!(
        elsewhere.contains("BUILTIN_GRAFT_CUTS: &[::xirang_toolchain::run_method::registry_core::StaticGraftCut] = &[\n];"),
        "{elsewhere}"
    );
    assert!(
        !elsewhere.contains("panel::frame::widget"),
        "a cut face this crate does not compile must not be named here: {elsewhere}"
    );

    // A selector the parser did not resolve to a module names nothing to check, so it stays data.
    // 解析器没有解析成模块的选择器没有可查的东西，因此保持为数据。
    assert!(names_a_compiled_module(&[], "root/control/button"));
    assert!(names_a_compiled_module(
        &[widget()],
        "crate::panel::frame::widget"
    ));
    assert!(!names_a_compiled_module(
        &[],
        "crate::panel::frame::widget::NODE_ID"
    ));
    std::fs::remove_dir_all(root).expect("temporary fixture cleanup");
}

/// A second face, under a subtree a **different** guest claims: the two-guest case the partition has
/// never been tested with (audit `M7`, §M7.50).
/// 第二个面，位于**另一个**幽灵认领的子树之下：分区从未被两个幽灵同时在场地测过的情形（审计 `M7`，§M7.50）。
fn dial() -> StaticFaceRecord {
    StaticFaceRecord {
        written_as_literal: false,
        kind: "Face".to_owned(),
        preset: "Preset".to_owned(),
        parts: "Parts".to_owned(),
        id: NodeId::from_bytes(b"dial"),
        parent: NodeId::from_bytes(b"gauge"),
        owns_registry: false,
        source: "panel/gauge/dial/dial.rs".to_owned(),
        module: "panel::gauge::dial".to_owned(),
    }
}

/// One typed graft entry, spelled the way the host writes it.
/// 一条类型化 graft 条目，按宿主书写的样子。
fn typed_graft(cut: &str, replacement: &str) -> GraftSyntax {
    GraftSyntax {
        cfg: None,
        cut: cut.to_owned(),
        cut_end: None,
        graft: replacement.to_owned(),
        full: false,
        location: SyntaxLocation { line: 7, column: 8 },
        expressions: Some(GraftExpressions {
            cut: cut.to_owned(),
            cut_end: None,
            graft: replacement.to_owned(),
        }),
        spans: None,
    }
}

/// A cross-crate range belongs to the facade; no fragment may name a missing endpoint.
/// 跨 crate 区间由 facade 携带；任何碎片都不能点名本包不存在的端点。
#[test]
fn a_range_split_between_crates_is_emitted_by_the_facade() {
    let mut cap = dial();
    cap.id = NodeId::from_bytes(b"cap");
    cap.module = "panel::gauge::cap".into();
    cap.source = "panel/gauge/cap/cap.rs".into();
    let mut range = typed_graft(
        "crate::panel::gauge::cap::NODE_ID",
        "fast_widget::fast::NODE_ID",
    );
    range.cut_end = Some("crate::panel::gauge::dial::NODE_ID".into());
    range.expressions.as_mut().unwrap().cut_end = range.cut_end.clone();
    let entries = [range];
    assert!(emitted_grafts(std::slice::from_ref(&cap), &entries, false).is_empty());
    assert!(emitted_grafts(&[dial()], &entries, false).is_empty());
    assert_eq!(
        emitted_grafts(&[cap.clone(), dial()], &entries, false).len(),
        1
    );
    let emitted = emitted_grafts(&[cap, dial()], &entries, true);
    assert_eq!(emitted.len(), 1);
    let owners = [
        ("panel::gauge::cap".into(), "app_widgets".into()),
        ("panel::gauge::dial".into(), "app_gauges".into()),
    ];
    let shape = ShapeRender {
        facade: true,
        owners: &owners,
        ..ShapeRender::whole()
    };
    let constructor = graft_constructor(&shape, &emitted[0], "registry");
    assert!(constructor.contains("from_id_range(app_widgets::panel::gauge::cap::NODE_ID, app_gauges::panel::gauge::dial::NODE_ID"), "{constructor}");
}

/// Local implementation references also belong to their compiling owner.
/// 本地实现引用同样必须由实际编译归属解析，不能给兄弟 crate 增加循环依赖。
#[test]
fn local_replacements_across_owners_are_emitted_by_the_facade() {
    for prefix in ["crate", "self"] {
        let entries = [typed_graft(
            "crate::panel::frame::widget::NODE_ID",
            &format!("{prefix}::panel::gauge::dial::NODE_ID"),
        )];
        assert!(emitted_grafts(&[widget()], &entries, false).is_empty());
        assert!(emitted_grafts(&[dial()], &entries, false).is_empty());
        assert_eq!(
            emitted_grafts(&[widget(), dial()], &entries, false).len(),
            1
        );
        let emitted = emitted_grafts(&[widget(), dial()], &entries, true);
        assert_eq!(emitted.len(), 1);
        let owners = [
            ("panel::frame".into(), "app_widgets".into()),
            ("panel::gauge".into(), "app_gauges".into()),
        ];
        let shape = ShapeRender {
            facade: true,
            owners: &owners,
            ..ShapeRender::whole()
        };
        let constructor = graft_constructor(&shape, &emitted[0], "registry");
        assert!(
            constructor.contains("app_gauges::panel::gauge::dial::NODE_ID"),
            "{constructor}"
        );
    }
}

/// **Two guests at once**: each ghost emits only its own cut, and the facade carries **both**, each
/// rewritten to the crate that compiles it (audit `M7`, §M7.50).
/// **两个幽灵同时在场**：每个幽灵只发射自己那条切口，而 facade 携带**两条**，各自改写成编译它的那个 crate
/// （审计 `M7`，§M7.50）。
///
/// The one-guest case is pinned above; what this adds is the case the maintainer asked about — a plan
/// whose cuts fall in **different** crates of the same partition, which is the only shape in which the
/// facade's owner table has to hold two answers at once.
/// 单个幽灵的情形上面已经钉过；这一条补的是维护者问的那种情形——一个计划的切口落在同一次拆分里**不同**的
/// crate 上，而那是 facade 的属主表必须同时装下两个答案的唯一形状。
#[test]
fn two_guests_each_emit_their_own_cut_and_the_facade_carries_both() {
    let root = temporary_directory("graft-two-crates");
    let entries = [
        typed_graft(
            "crate::panel::frame::widget::NODE_ID",
            "fast_widget::fast::NODE_ID",
        ),
        typed_graft(
            "crate::panel::gauge::dial::NODE_ID",
            "fast_gauge::fast::NODE_ID",
        ),
    ];
    let render = |faces: &[StaticFaceRecord], shape: ShapeRender<'_>| {
        render_lib(
            &root,
            &[],
            &BuildDiagnostics::default(),
            &BuildDiagnostics::default(),
            &SourceScope {
                roots: None,
                reason: "test",
            },
            faces,
            &entries,
            shape,
            Path::new("xirang_modules"),
            &mut Vec::new(),
        )
    };
    // Owner names are **crate** names: `crate_plan::owners` underscores the package's hyphens, because
    // a Rust path segment cannot contain one (the fixture here spells them the way the pipeline does).
    // 属主名是 **crate** 名：`crate_plan::owners` 把包名的连字符换成下划线，因为 Rust 的路径段不能含它
    // （这里的夹具按管线的写法拼）。
    let owners = [
        ("panel::frame".to_owned(), "app_widgets".to_owned()),
        ("panel::gauge".to_owned(), "app_gauges".to_owned()),
    ];
    let facade_shape = ShapeRender {
        cut_out: &["panel::frame".to_owned(), "panel::gauge".to_owned()],
        only: None,
        mounts: &[],
        ancestors: &[],
        facade: true,
        owners: &owners,
    };

    // The host compiles neither claimed subtree, so it emits neither entry.
    // 宿主不编译任何被认领的子树，因此两条都不发射。
    let host = render(&[], ShapeRender::whole());
    assert!(
        !host.contains("fast_widget") && !host.contains("fast_gauge"),
        "the host handed both subtrees away: {host}"
    );

    // Each ghost emits exactly its own, and not the other's.
    // 每个幽灵只发射自己那条，不发射另一条。
    let ghost_a = render(&[widget()], ShapeRender::whole());
    assert!(
        ghost_a.contains("StaticGraftCut::from_ids(crate::panel::frame::widget::NODE_ID, fast_widget::fast::NODE_ID, false)"),
        "{ghost_a}"
    );
    assert!(
        !ghost_a.contains("fast_gauge"),
        "the other crate's cut is not this crate's: {ghost_a}"
    );
    let ghost_b = render(&[dial()], ShapeRender::whole());
    assert!(
        ghost_b.contains(
            "StaticGraftCut::from_ids(crate::panel::gauge::dial::NODE_ID, fast_gauge::fast::NODE_ID, false)"
        ),
        "{ghost_b}"
    );
    assert!(
        !ghost_b.contains("fast_widget"),
        "the other crate's cut is not this crate's: {ghost_b}"
    );

    // The facade carries the union, and each entry names the crate that compiles its cut — which is
    // the whole question of a plan that spans two crates.
    // facade 携带并集，而每条条目点名编译它那个切口面的 crate——这正是"计划横跨两个 crate"的全部问题。
    let facade = render(&[widget(), dial()], facade_shape);
    assert!(
        facade.contains("StaticGraftCut::from_ids(app_widgets::panel::frame::widget::NODE_ID"),
        "the facade names crate A's owner: {facade}"
    );
    assert!(
        facade.contains("assert_contract::<app_widgets::panel::frame::widget::__Preset, fast_widget::fast::__Parts>()"),
        "and its contract assertion belongs to the same owner: {facade}"
    );
    assert!(
        !facade.contains("app-widgets"),
        "a hyphen would make the generated path invalid Rust: {facade}"
    );
    assert!(
        facade.contains("app_gauges::panel::gauge::dial::NODE_ID"),
        "the facade names crate B's owner: {facade}"
    );
    std::fs::remove_dir_all(root).expect("temporary fixture cleanup");
}

/// A face that owns a registry names its own `registry_rule` module's constant; a face that does not
/// admits any child — the convention the declaration macros used, reproduced for the form whose
/// declaration cannot carry the value.
/// 拥有注册机的面点名叫自己 `registry_rule` 模块里的常量；不拥有的面接纳任意子面——这正是声明宏当年用的
/// 那条约定，为"声明带不动这个值"的那种拼法复述一遍。
#[test]
fn a_face_that_owns_a_registry_names_its_rule_module() {
    let registry = "::xirang_toolchain::run_method::registry_core";
    assert_eq!(
        super::resolve_own_rule("control", true, registry),
        "crate::control::registry_rule::REGISTRATION_RULE",
        "a face with a registry names the sibling rule module's constant"
    );
    assert_eq!(
        super::resolve_own_rule("control::object::button", false, registry),
        format!("{registry}::RegistrationRule::ANY"),
        "a face without one admits any child"
    );
    assert_eq!(
        super::resolve_own_rule("", true, registry),
        format!("{registry}::RegistrationRule::ANY"),
        "a crate-root face has no module to name, so it admits any child"
    );
}
