//! Non-Rust declarations are refused by the generic move until their adapter can plan the rewrite.
//! 非 Rust 声明由通用 move 拒绝，直到其适配器能规划改写。

use super::*;
use crate::build_method::FaceView;
use xirang_kernel::identity::NodeId;

fn view(source: &str) -> FaceView {
    FaceView {
        id: NodeId::from_raw([1; 16]),
        stable_name: None,
        parent: NodeId::from_raw([2; 16]),
        kind: "Area".to_owned(),
        registry_name: "Area".to_owned(),
        module: "control::area".to_owned(),
        source: source.to_owned(),
        source_base: crate::build_method::source_ref::SourceBase::Package,
        owns_registry: false,
        path: "root::control::area".to_owned(),
        parent_resolved: true,
    }
}

/// A C++ face under the moved directory has no adapter move plan, so the operation refuses before
/// anything is planned or written.
/// 被搬目录下的 C++ 面没有适配器的搬动计划，因此操作在任何规划/写入之前拒绝。
#[test]
fn a_cpp_face_under_the_moved_directory_is_refused() {
    let error = ensure_structural_support(&[view("control/object/area/area.cpp")], "control")
        .expect_err("a C++ face cannot be moved silently");
    assert!(error.contains("structural move"), "{error}");
    assert!(error.contains("area.cpp"), "{error}");
}

/// A Rust face is the build's own language and is not refused by this guard.
/// Rust 面是构建自己的语言，不被这道守卫拒绝。
#[test]
fn a_rust_face_is_left_to_the_generic_planner() {
    ensure_structural_support(&[view("control/object/area/area.rs")], "control")
        .expect("a Rust face is planned by the generic walk");
}
