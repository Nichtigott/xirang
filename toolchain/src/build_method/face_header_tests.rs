//! Tests for the header that names every face to C++.
//! 面向 C++ 命名每个面的头文件的测试。

use super::{NamedFace, compatibility_namespaces, render};
use crate::build_method::registry_identity::NodeId;
use crate::build_method::static_plan::StaticFaceRecord;

fn face(module: &str, kind: &str, id: [u8; 16]) -> StaticFaceRecord {
    StaticFaceRecord {
        id: NodeId::from_raw(id),
        written_as_literal: false,
        kind: kind.to_owned(),
        preset: String::new(),
        parts: String::new(),
        parent: NodeId::from_raw([0; 16]),
        owns_registry: false,
        source: format!("{}.rs", module.replace("::", "/")),
        module: module.to_owned(),
    }
}

fn header(faces: &[StaticFaceRecord], native: &[NamedFace]) -> String {
    render(faces, native, &[], "test-namespace").expect("the fixture modules are distinct")
}

fn header_with(
    faces: &[StaticFaceRecord],
    native: &[NamedFace],
    aliases: &[(String, [u8; 16])],
) -> String {
    render(faces, native, aliases, "test-namespace").expect("the fixture modules are distinct")
}

/// The point of the file: a C++ declaration names the parent by constant rather than by
/// hand-copied bytes, so a wrong name is a compile error and a moved parent is a rebuild.
/// 本文件的要点：C++ 声明用常量而不是手抄字节点名父面，于是写错名字是编译错误、父面挪动是
/// 一次重建。
#[test]
fn a_rust_face_becomes_a_named_identity_in_the_faces_namespace() {
    let header = header(&[face("control::object::button", "Button", [1; 16])], &[]);
    assert!(header.contains("#pragma once"));
    assert!(header.contains(r#"#include "xirang/soil.hpp""#));
    assert!(header.contains("namespace xirang::faces {"));
    // The author-facing spelling mirrors the logical tree.
    // 面向作者的拼法镜像逻辑树。
    assert!(
        header.contains("namespace control::object::button {"),
        "{header}"
    );
    assert!(
        header.contains(
            "inline constexpr ::xirang::Id NODE_ID = \
             ::xirang::Id{{1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1}};"
        ),
        "{header}"
    );
    // The version-1 flat identifier stays as an alias.
    // 版本 1 的扁平标识符保留为别名。
    assert!(
        header.contains(
            "inline constexpr ::xirang::Id control__object__button_Button = \
             ::xirang::Id{{1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1}};"
        ),
        "{header}"
    );
    assert!(header.contains("}  // namespace xirang::faces"));
}

/// Discovery order is not part of the file: the same faces in another order are the same
/// header, which is what lets a build skip rewriting it and lets a committed copy diff.
/// 发现顺序不是文件的一部分：同一批面换个顺序还是同一份头，这正是构建可以跳过重写、入库副本
/// 还能逐行比对的原因。
#[test]
fn the_header_is_stable_under_discovery_order() {
    let button = face("control::object::button", "Button", [1; 16]);
    let slider = face("control::object::slider", "Slider", [2; 16]);
    assert_eq!(
        render(
            &[button.clone(), slider.clone()],
            &[],
            &[],
            "test-namespace"
        )
        .expect("distinct modules"),
        render(&[slider, button], &[], &[], "test-namespace").expect("distinct modules")
    );
}

/// Two faces may share a kind; their modules are what keeps the identifiers apart.
/// 两个面可以共用一个种类名；把它们区分开的是模块路径。
#[test]
fn faces_that_share_a_kind_keep_distinct_identifiers() {
    let header = header(
        &[
            face("panel::object::button", "Button", [3; 16]),
            face("panel::object::slider", "Button", [4; 16]),
        ],
        &[],
    );
    assert!(header.contains("panel__object__button_Button"));
    assert!(header.contains("panel__object__slider_Button"));
    assert!(header.contains("namespace panel::object::button {"));
    assert!(header.contains("namespace panel::object::slider {"));
}

/// Two faces in one module have no distinct nested path, so the header refuses them instead of
/// emitting a duplicate `NODE_ID`.
/// 同一模块里的两个面没有彼此不同的嵌套路径，因此头部拒绝它们，而不是发射重复的 `NODE_ID`。
#[test]
fn two_faces_in_one_module_are_refused_by_name() {
    let error = render(
        &[
            face("shop::panel", "Panel", [5; 16]),
            face("shop::panel", "Other", [6; 16]),
        ],
        &[],
        &[],
        "test-namespace",
    )
    .unwrap_err();
    assert!(error.contains("share the module"), "{error}");
    assert!(error.contains("shop::panel"), "{error}");
    assert!(error.contains("NODE_ID"), "{error}");
}

/// A declared native face is named the same way: its module comes from the source spelling, so a
/// child can name it before the compiler runs.
/// 声明式本地面用同一种方式命名：它的模块来自源码拼写，因此子面能在编译器运行之前点名它。
#[test]
fn a_native_face_is_named_by_its_source_module() {
    let header = header(
        &[],
        &[NamedFace {
            module: "native::area".to_owned(),
            kind: "Area".to_owned(),
            id: [7; 16],
        }],
    );
    assert!(header.contains("namespace native::area {"), "{header}");
    assert!(header.contains("native__area_Area"), "{header}");
}

/// A retired module path becomes a namespace for the object's current identity: that is what keeps
/// a third party's `.under(...)` valid when a move changes the parent's module.
/// 退役的模块路径变成该对象当前身份的命名空间：这正是搬动改变父模块路径时，第三方的 `.under(...)`
/// 仍然有效的原因。
#[test]
fn a_retired_module_path_is_kept_as_a_compatibility_namespace() {
    let live = face("components::area::area2", "Area", [7; 16]);
    let ledger = ledger_with(&[("components/area/area.cpp", "components/area/area2.cpp")]);
    let aliases = compatibility_namespaces(
        &[(
            "components/area/area2.cpp".to_owned(),
            live.module.clone(),
            live.id.into_bytes(),
        )],
        &ledger,
    );
    assert_eq!(aliases, vec![("components::area".to_owned(), [7; 16])]);
    let header = header_with(std::slice::from_ref(&live), &[], &aliases);
    assert!(
        header.contains("namespace components::area {"),
        "the retired path stays nameable: {header}"
    );
    assert!(
        header.contains("// Compatibility spellings"),
        "the section says what it is: {header}"
    );
    // The current path is still the primary spelling.
    assert!(
        header.contains("namespace components::area::area2 {"),
        "{header}"
    );
}

/// A retired path that a **live** face now occupies is skipped: the live face is the answer there,
/// and two `NODE_ID`s in one namespace would be ambiguous.
/// 被活面占据的退役路径被跳过：那里答案是活面，而一个命名空间里两个 `NODE_ID` 会有歧义。
#[test]
fn a_retired_path_another_face_now_owns_is_skipped() {
    let live = face("components::area::area2", "Area", [8; 16]);
    let occupant = face("components::area", "Other", [9; 16]);
    let ledger = ledger_with(&[("components/area/area.cpp", "components/area/area2.cpp")]);
    let aliases = compatibility_namespaces(
        &[(
            "components/area/area2.cpp".to_owned(),
            live.module.clone(),
            live.id.into_bytes(),
        )],
        &ledger,
    );
    let header = header_with(&[live, occupant], &[], &aliases);
    let retired = header
        .split("// Compatibility spellings")
        .nth(1)
        .unwrap_or_default();
    assert!(
        !retired.contains("namespace components::area {"),
        "the live face keeps its namespace: {header}"
    );
}

/// An alias is a fact about this tree, not a guess: a ledger entry whose current source is not one
/// of this build's faces contributes nothing.
/// 别名是关于这棵树的事实而不是猜测：当前源码不是本次构建任何面的台账条目什么都不贡献。
#[test]
fn a_ledger_entry_this_build_does_not_carry_contributes_nothing() {
    let ledger = ledger_with(&[("components/area/area.cpp", "components/area/area2.cpp")]);
    assert!(compatibility_namespaces(&[], &ledger).is_empty());
}

/// A ledger with one object whose source moved.
/// 一份台账：一个对象换过源码。
fn ledger_with(history: &[(&str, &str)]) -> xirang_multilang::object_map::ObjectMap {
    let mut map = xirang_multilang::object_map::ObjectMap::empty();
    for (old, new) in history {
        let stable =
            xirang_kernel::identity::StableFaceId::from_namespaced_name("probe", "root/Area");
        let observed = xirang_multilang::object_map::Observed::new(stable, "root/Area", *old);
        let instance = xirang_kernel::identity::NodeId::from_namespaced_path("probe", old, "Area");
        map.observe(&observed, Some(instance)).expect("observe");
        let observed = xirang_multilang::object_map::Observed::new(stable, "root/Area", *new);
        let instance = xirang_kernel::identity::NodeId::from_namespaced_path("probe", new, "Area");
        map.observe(&observed, Some(instance)).expect("observe");
    }
    map
}
