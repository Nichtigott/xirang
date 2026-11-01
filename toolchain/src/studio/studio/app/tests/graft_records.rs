//! covers `app/graft.rs`.
//! 覆盖 `app/graft.rs`。
//!
//! External-graft record and screen tests, moved out of `app/graft.rs` so the module under test stays inside the size ratchet.
//! 外部 graft 记录与界面测试；从 `app/graft.rs` 移出，使被测模块留在尺寸棘轮之内。

use super::super::graft::{declaration_for, graft_selector_error};
use super::*;
use crate::build_method::{DeclaredGraft, DeclaredGrafts};
use std::path::PathBuf;

fn declared(cut: DeclaredGraft) -> DeclaredGrafts {
    DeclaredGrafts {
        entry: PathBuf::from("src/lib.rs"),
        cuts: vec![cut],
    }
}

fn string_cut(path: &str) -> DeclaredGraft {
    DeclaredGraft {
        cut: path.to_owned(),
        cut_end: None,
        graft: "button_fast".to_owned(),
        full: false,
        cfg: None,
        expressions: None,
        line: 4,
    }
}

#[test]
fn a_string_cut_names_the_logical_path() {
    let declared = declared(string_cut("root/control/button"));
    assert_eq!(
        declaration_for(
            "root/control/button",
            Some("control::object::button"),
            &declared
        ),
        GraftDeclaration::Declared {
            expression: "cut \"root/control/button\" graft \"button_fast\"".to_owned(),
            line: 4,
            cfg: None,
        }
    );
    assert!(matches!(
        declaration_for(
            "root/control/slider",
            Some("control::object::slider"),
            &declared
        ),
        GraftDeclaration::Absent { .. }
    ));
}

#[test]
fn a_typed_cut_names_the_module_from_the_crate_root() {
    let declared = declared(DeclaredGraft {
        cut: "crate::control::object::button::NODE_ID".to_owned(),
        cut_end: None,
        graft: "control_button_graft::button_fast::NODE_ID".to_owned(),
        full: true,
        cfg: None,
        expressions: Some(crate::build_method::DeclaredGraftExpressions {
            cut: "crate::control::object::button::NODE_ID".to_owned(),
            cut_end: None,
            graft: "control_button_graft::button_fast::NODE_ID".to_owned(),
        }),
        line: 7,
    });
    assert_eq!(
            declaration_for(
                "root/control/button",
                Some("control::object::button"),
                &declared
            ),
            GraftDeclaration::Declared {
                expression:
                    "cut(crate::control::object::button::NODE_ID) full graft(control_button_graft::button_fast::NODE_ID)"
                        .to_owned(),
                line: 7,
                cfg: None,
            }
        );
    // The build strips `crate::studio::` before comparing; so does this check.
    assert!(matches!(
        declaration_for("root/control/button", Some("other::module"), &declared),
        GraftDeclaration::Absent { .. }
    ));
}

#[test]
fn a_range_cut_names_both_endpoints() {
    let mut range = string_cut("root/a");
    range.cut_end = Some("root/c".to_owned());
    let declared = declared(range);
    for path in ["root/a", "root/c"] {
        assert!(matches!(
            declaration_for(path, Some("a"), &declared),
            GraftDeclaration::Declared { .. }
        ));
    }
    assert!(matches!(
        declaration_for("root/b", Some("b"), &declared),
        GraftDeclaration::Absent { .. }
    ));
    // The rendered clause keeps the endpoints as two literals, so pasting it
    // back declares a range instead of one path named `root/a to root/c`.
    // 渲染出的子句把端点保持为两个字面量，粘回去声明的是区间，而不是一条名为
    // `root/a to root/c` 的路径。
    assert_eq!(
        declaration_for("root/a", Some("a"), &declared),
        GraftDeclaration::Declared {
            expression: "cut [\"root/a\" to \"root/c\"] graft \"button_fast\"".to_owned(),
            line: 4,
            cfg: None,
        }
    );
}

/// A typed cut cannot match once the target module cannot be resolved.
/// 目标模块解析不出来时，类型化切口不再匹配。
#[test]
fn an_unresolved_module_only_matches_a_string_cut() {
    let typed = declared(DeclaredGraft {
        cut: "crate::control::object::button::NODE_ID".to_owned(),
        cut_end: None,
        graft: "control_button_graft::button_fast::NODE_ID".to_owned(),
        full: false,
        cfg: None,
        expressions: Some(crate::build_method::DeclaredGraftExpressions {
            cut: "crate::control::object::button::NODE_ID".to_owned(),
            cut_end: None,
            graft: "control_button_graft::button_fast::NODE_ID".to_owned(),
        }),
        line: 7,
    });
    assert!(matches!(
        declaration_for("root/control/button", None, &typed),
        GraftDeclaration::Absent { .. }
    ));
    let text = declared(string_cut("root/control/button"));
    assert!(matches!(
        declaration_for("root/control/button", None, &text),
        GraftDeclaration::Declared { .. }
    ));
}

#[test]
fn selectors_that_escape_the_plan_directory_are_refused() {
    assert!(graft_selector_error("button_fast").is_none());
    assert!(graft_selector_error(" ").is_some());
    assert!(graft_selector_error("a/b").is_some());
    assert!(graft_selector_error("a\\b").is_some());
}

/// The graft screen renders before any plan exists. `then_some` evaluated
/// `plans.len() - 1` eagerly, so on a fresh project the `g` key panicked the whole
/// app in every build with overflow checks on — which is every debug build,
/// including this repo's own `target/debug`.
/// 没有任何计划时 graft 界面也必须能渲染。`then_some` 会立即求值 `plans.len() - 1`，因此在
/// 全新工程上按 `g` 会让整个 app 在开启溢出检查的构建里 panic——也就是每种 debug 构建，包括
/// 本仓库自己的 `target/debug`。
///
/// Fixture-gated like the other tests that need a project with faces, and it asserts
/// the premise (a face is selected, the screen opened) rather than returning early:
/// an earlier version of this test returned when nothing was selected and therefore
/// passed with the bug still in place.
/// 与其余需要"有注册面的工程"的测试一样门控在夹具上，并且它断言前提（确实选中了一个面、界面
/// 确实打开了）而不是提前返回：本测试的早先版本在没有选中项时直接返回，因此在缺陷仍在时也通过。
#[test]
#[cfg(feature = "prototype-fixtures")]
fn the_graft_screen_renders_before_any_plan_exists() {
    use crossterm::event::{KeyCode, KeyEvent};
    let Some(mut app) = fixture_app() else {
        return;
    };
    assert_ne!(
        app.selected,
        app.registry.id(),
        "the fixture has faces, so one must be selected"
    );
    app.handle_key(KeyEvent::from(KeyCode::Char('g')));
    assert!(
        matches!(app.overlay, Some(Overlay::Graft(_))),
        "g must open the graft screen"
    );
    let mut terminal = ratatui::Terminal::new(ratatui::backend::TestBackend::new(140, 48)).unwrap();
    crate::studio::studio::ui::draw_once(&mut terminal, &mut app)
        .expect("the graft screen renders with no plans");
}
