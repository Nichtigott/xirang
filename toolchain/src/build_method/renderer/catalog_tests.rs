//! Pins for the shared logical catalog: object relations, not source paths.
//! 共享逻辑目录的钉子：对象关系，而不是源码路径。

use super::render_catalog;
use crate::build_method::registry_identity::NodeId;
use crate::build_method::static_plan::StaticFaceRecord;

fn face(module: &str, id: [u8; 16]) -> StaticFaceRecord {
    StaticFaceRecord {
        id: NodeId::from_raw(id),
        written_as_literal: false,
        kind: "Node".to_owned(),
        preset: String::new(),
        parts: String::new(),
        parent: NodeId::from_raw([0; 16]),
        owns_registry: false,
        source: format!("{}.rs", module.replace("::", "/")),
        module: module.to_owned(),
    }
}

#[test]
fn a_logical_path_does_not_use_the_face_module_or_source_path() {
    // The legacy module path says `native::area`; the logical relation says `shop/panel/Area`.
    // 旧模块路径说 `native::area`；逻辑关系说 `shop/panel/Area`。
    let text = render_catalog(
        &[face("native::area", [1; 16])],
        &[("shop/panel/Area".to_owned(), [2; 16])],
    )
    .expect("the catalog renders");
    assert!(text.contains("pub mod shop"), "{text}");
    assert!(text.contains("pub mod panel"), "{text}");
    assert!(text.contains("pub mod Area"), "{text}");
    assert!(
        text.contains("pub mod native"),
        "the legacy path stays nameable: {text}"
    );
    let area = text
        .split("pub mod Area")
        .nth(1)
        .expect("the logical Area branch");
    assert!(area.contains("from_raw([2, 2, 2"), "{area}");
}

#[test]
fn two_objects_on_one_logical_path_are_refused() {
    let error = render_catalog(
        &[],
        &[
            ("shop/panel".to_owned(), [1; 16]),
            ("shop/panel".to_owned(), [2; 16]),
        ],
    )
    .expect_err("one path cannot name two objects");
    assert!(error.contains("one catalog path"), "{error}");
}
