//! Cross-action regressions: moves must account for every affected graft slot.
//! 动作交叉回归：搬迁必须核对每个受影响的嫁接槽位。

use super::graft_fixtures::Tree;
use super::*;
use serde_json::json;

#[test]
fn an_ancestor_move_checks_typed_and_string_grafts_on_descendants() {
    for cut in [
        "cut(crate::panel::b::NODE_ID) graft(external::replacement::NODE_ID),",
        "cut \"root/panel/b\" graft \"replacement\",",
    ] {
        let tree = Tree::fixture();
        tree.plan(cut);
        let error = tree.refused("root/panel", "root/shelf", None);
        assert!(
            error.contains("root/panel/b") && error.contains("graft"),
            "{error}"
        );
    }
}

#[test]
fn a_move_checks_range_interior_and_incoming_membership() {
    let tree = Tree::fixture();
    tree.plan("cut(crate::panel::a::NODE_ID to crate::panel::c::NODE_ID) graft(external::replacement::NODE_ID),");
    let error = tree.refused("root/panel/b", "root/shelf", None);
    assert!(error.contains("range"), "{error}");
    // Renaming an outside object into the same interval changes the effective graft too.
    // 把区间外对象改名搬入同一区间，也会改变有效嫁接。
    let error = tree.refused("root/loose", "root/panel", Some("bb"));
    assert!(error.contains("range"), "{error}");
}

#[test]
fn moving_out_of_a_full_graft_cannot_silently_restore_the_base_object() {
    let tree = Tree::fixture();
    tree.plan("cut(crate::panel::NODE_ID) full graft(external::replacement::NODE_ID),");
    let error = tree.refused("root/panel/b", "root/shelf", None);
    assert!(error.contains("full"), "{error}");
}

#[test]
fn recorded_grafts_on_only_some_moved_objects_are_named_before_writing() {
    let tree = Tree::fixture();
    let faces = crate::build_method::face_views(&tree.0, "move-graft-test").unwrap();
    for name in ["b", "c"] {
        let face = faces
            .iter()
            .find(|face| face.path == format!("root/panel/{name}"))
            .unwrap();
        let dir = tree.0.join(format!(".xirang/external-grafts/impl_{name}"));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("graft.plan"),
            format!(
                "version=1\ntarget={}\ntarget_path={}\ngraft=impl_{name}\nfull=false\n",
                face.id, face.path
            ),
        )
        .unwrap();
    }
    let untouched =
        std::fs::read(tree.0.join(".xirang/external-grafts/impl_c/graft.plan")).unwrap();
    let error = tree.refused("root/panel/b", "root/shelf", None);
    assert!(
        error.contains("impl_b") && error.contains("root/shelf/b"),
        "{error}"
    );
    assert!(
        !error.contains("impl_c"),
        "unaffected graft must not be in the repair set: {error}"
    );
    assert_eq!(
        std::fs::read(tree.0.join(".xirang/external-grafts/impl_c/graft.plan")).unwrap(),
        untouched
    );
}

#[test]
fn unrelated_grafts_do_not_block_an_unrelated_move() {
    let tree = Tree::fixture();
    tree.plan("cut(crate::panel::b::NODE_ID) graft(external::replacement::NODE_ID),");
    let before = std::fs::read(tree.0.join("src/lib.rs")).unwrap();
    run_move(
        &tree.0,
        &tree.0,
        true,
        &json!({"node":"root/loose","to":"root/shelf"}),
    )
    .unwrap();
    assert_eq!(std::fs::read(tree.0.join("src/lib.rs")).unwrap(), before);
    assert!(tree.0.join("src/shelf/loose/loose.rs").exists());
}

#[test]
fn preview_checks_records_in_the_original_project_and_rejects_broken_records() {
    let tree = Tree::fixture();
    let preview = Tree::fixture();
    let face = crate::build_method::face_views(&tree.0, "move-graft-test")
        .unwrap()
        .into_iter()
        .find(|face| face.path == "root/panel/b")
        .unwrap();
    let dir = tree.0.join(".xirang/external-grafts/impl_b");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("graft.plan");
    std::fs::write(
        &path,
        crate::run_method::GraftPlanDocument::new(face.id, &face.path, "impl_b", false)
            .render_graft_plan_document(),
    )
    .unwrap();
    let args = json!({"node":"root/panel/b", "to":"root/shelf"});
    let error = run_move(&tree.0, &preview.0, false, &args).err().unwrap();
    assert!(error.contains("impl_b"), "{error}");
    assert!(preview.0.join("src/panel/b/b.rs").exists());
    assert!(!preview.0.join(".xirang").exists());
    std::fs::write(path, "version=99\n").unwrap();
    let error = tree.refused("root/loose", "root/shelf", None);
    assert!(
        error.contains("unreadable") && error.contains("impl_b"),
        "{error}"
    );
}

#[test]
fn rebinding_only_the_moved_record_restores_both_external_implementations() {
    use crate::run_method::registry_core::{FrameworkId, Registry, StaticGraftCut, root_node_id};
    use crate::run_method::{GraftPlanDocument, apply_recorded_grafts};
    let tree = Tree::fixture();
    tree.plan("cut \"root/panel/b\" graft \"impl_b\",\ncut \"root/panel/c\" graft \"impl_c\",");
    let base = crate::mcp::apply::load_registry(&tree.0, "move-graft-test").unwrap();
    let mut external = Registry::root_for_namespace(FrameworkId::new("xirang.mcp"), "external");
    for name in ["b", "c"] {
        let mut face = (*base
            .depth_first()
            .into_iter()
            .find(|face| base.path_for(face.id).as_deref() == Some(&format!("root/panel/{name}")))
            .unwrap())
        .clone();
        let dir = tree.0.join(format!(".xirang/external-grafts/impl_{name}"));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("graft.plan"),
            GraftPlanDocument::new(
                face.id,
                format!("root/panel/{name}"),
                format!("impl_{name}"),
                false,
            )
            .render_graft_plan_document(),
        )
        .unwrap();
        face.namespace = "external".into();
        face.id = NodeId::from_namespaced_path("external", name, &face.kind);
        face.parent = root_node_id("external");
        face.registry_name = format!("impl_{name}");
        face.kind = format!("External_{name}");
        external.register_snapshot_batch([face]).unwrap();
    }
    let cuts = |b: &'static str| {
        [
            StaticGraftCut::new(b, "impl_b", false),
            StaticGraftCut::new("root/panel/c", "impl_c", false),
        ]
    };
    let before = apply_recorded_grafts(&base, &external, &cuts("root/panel/b"), &tree.0).unwrap();
    let kind = |registry: &Registry, path: &str| {
        registry
            .depth_first()
            .into_iter()
            .find(|face| registry.path_for(face.id).as_deref() == Some(path))
            .unwrap()
            .kind
            .clone()
    };
    assert_eq!(kind(&before.effective, "root/panel/b"), "External_b");
    assert_eq!(kind(&before.effective, "root/panel/c"), "External_c");
    let untouched_path = tree.0.join(".xirang/external-grafts/impl_c/graft.plan");
    let untouched = std::fs::read(&untouched_path).unwrap();
    tree.refused("root/panel/b", "root/shelf", None);

    // Exercise the documented manual repair, preserving the third-party provider.
    // 走通已说明的手工修复路径，第三方提供者始终保持原样。
    let record = tree.0.join(".xirang/external-grafts/impl_b/graft.plan");
    std::fs::rename(record.parent().unwrap(), tree.0.join("saved-graft")).unwrap();
    tree.plan("cut \"root/panel/c\" graft \"impl_c\",");
    run_move(
        &tree.0,
        &tree.0,
        true,
        &json!({"node":"root/panel/b", "to":"root/shelf"}),
    )
    .unwrap();
    let after = crate::mcp::apply::load_registry(&tree.0, "move-graft-test").unwrap();
    let moved = after
        .depth_first()
        .into_iter()
        .find(|face| after.path_for(face.id).as_deref() == Some("root/shelf/b"))
        .unwrap()
        .id;
    tree.plan("cut \"root/shelf/b\" graft \"impl_b\",\ncut \"root/panel/c\" graft \"impl_c\",");
    std::fs::rename(tree.0.join("saved-graft"), record.parent().unwrap()).unwrap();
    std::fs::write(
        record,
        GraftPlanDocument::new(moved, "root/shelf/b", "impl_b", false).render_graft_plan_document(),
    )
    .unwrap();
    let replay = apply_recorded_grafts(&after, &external, &cuts("root/shelf/b"), &tree.0).unwrap();
    assert_eq!(kind(&replay.effective, "root/shelf/b"), "External_b");
    assert_eq!(kind(&replay.effective, "root/panel/c"), "External_c");
    assert_eq!(std::fs::read(untouched_path).unwrap(), untouched);
    let old_c = before
        .effective
        .depth_first()
        .into_iter()
        .find(|face| face.kind == "External_c")
        .unwrap();
    let new_c = replay
        .effective
        .depth_first()
        .into_iter()
        .find(|face| face.kind == "External_c")
        .unwrap();
    assert_eq!(old_c, new_c);
}

#[test]
fn moving_an_ancestor_rewrites_deep_parent_paths() {
    for name in ["panel", "dashboard"] {
        let tree = Tree::fixture();
        tree.face("panel/group", "panel", true);
        tree.face("panel/group/leaf", "panel/group", false);
        run_move(
            &tree.0,
            &tree.0,
            true,
            &json!({"node":"root/panel", "to":"root/shelf", "name":name}),
        )
        .unwrap();
        let registry = crate::mcp::apply::load_registry(&tree.0, "move-graft-test").unwrap();
        assert!(
            registry
                .depth_first()
                .into_iter()
                .any(|face| registry.path_for(face.id).as_deref()
                    == Some(&format!("root/shelf/{name}/group/leaf")))
        );
        let group =
            std::fs::read_to_string(tree.0.join(format!("src/shelf/{name}/group/group.rs")))
                .unwrap();
        assert!(group.contains(&format!("crate::{name}_object!")), "{group}");
        let leaf =
            std::fs::read_to_string(tree.0.join(format!("src/shelf/{name}/group/leaf/leaf.rs")))
                .unwrap();
        assert!(leaf.contains("crate::group_object!"), "{leaf}");
    }
}

#[test]
fn unknown_graft_targets_refuse_moves_instead_of_assuming_no_impact() {
    let tree = Tree::fixture();
    tree.plan("cut(crate::missing::NODE_ID) graft(external::replacement::NODE_ID),");
    let error = tree.refused("root/loose", "root/shelf", None);
    assert!(error.contains("unresolved endpoint"), "{error}");
    tree.plan("cut(crate::panel::NODE_ID) full graft(external::replacement::NODE_ID),");
    let error = tree.refused("root/loose", "root/panel", None);
    assert!(error.contains("full"), "{error}");
}

#[test]
fn full_record_coverage_uses_the_resolved_identity_when_its_path_has_drifted() {
    let tree = Tree::fixture();
    let views = crate::build_method::face_views(&tree.0, "move-graft-test").unwrap();
    let panel = views.iter().find(|face| face.path == "root/panel").unwrap();
    let dir = tree.0.join(".xirang/external-grafts/impl_panel");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("graft.plan"),
        crate::run_method::GraftPlanDocument::new(panel.id, "root/old_panel", "impl_panel", true)
            .render_graft_plan_document(),
    )
    .unwrap();
    let error = tree.refused("root/panel/b", "root/shelf", None);
    assert!(
        error.contains("impl_panel") && error.contains("full"),
        "{error}"
    );
    std::fs::write(
        dir.join("graft.plan"),
        crate::run_method::GraftPlanDocument::new(panel.id, "root/shelf", "impl_panel", true)
            .render_graft_plan_document(),
    )
    .unwrap();
    let error = tree.refused("root/loose", "root/shelf", None);
    assert!(error.contains("different objects"), "{error}");
}

#[test]
fn undefined_full_range_declarations_refuse_the_move_before_writing() {
    let tree = Tree::fixture();
    tree.face("panel/b", "panel", true);
    tree.face("panel/b/inner", "panel/b", false);
    tree.plan("cut(crate::panel::a::NODE_ID to crate::panel::c::NODE_ID) full graft(external::replacement::NODE_ID),");
    let error = tree.refused("root/panel/b/inner", "root/shelf", None);
    assert!(
        error.contains("`full` on a range cut has no defined meaning"),
        "{error}"
    );
    let error = tree.refused("root/loose", "root/panel/b", None);
    assert!(
        error.contains("`full` on a range cut has no defined meaning"),
        "{error}"
    );
}

#[test]
fn moving_a_claim_root_still_requires_rebinding_the_declaration() {
    let tree = Tree::fixture();
    std::fs::write(tree.0.join("add_crates.rs"),
        "pub fn add_crates() -> Shape { Shape::of(\"app\", &[Crate::named(\"widgets\").at(&[crate::panel::SUBTREE]), Crate::named(\"gauges\").at(&[crate::shelf::SUBTREE])]) }").unwrap();
    for (node, to) in [("root/panel", "root/shelf"), ("root/panel", "root")] {
        let error = tree.refused(node, to, Some("renamed"));
        assert!(error.contains("add_crates.rs"), "{error}");
    }
}

#[test]
fn move_planning_accepts_inline_and_multiline_fields_without_reading_literal_parents() {
    for parent in [
        "crate::panel::NODE_ID",
        "/* 中文前注 */ crate::panel::\n NODE_ID /* 后注 */",
    ] {
        for node in ["root/panel/b", "root/panel"] {
            let tree = Tree::fixture();
            let suffix = "\npub const NOTE: &str = r#\"\nparent: crate::fake::NODE_ID,\n\"#;\n";
            std::fs::write(
                tree.0.join("src/panel/b/b.rs"),
                format!("crate::panel_object! {{ kind: Kind_b, parent: {parent}, }}{suffix}"),
            )
            .unwrap();
            run_move(
                &tree.0,
                &tree.0,
                true,
                &json!({"node":node,"to":"root/shelf"}),
            )
            .unwrap();
            let directory = if node.ends_with("/b") {
                "shelf/b"
            } else {
                "shelf/panel/b"
            };
            let source =
                std::fs::read_to_string(tree.0.join(format!("src/{directory}/b.rs"))).unwrap();
            assert!(source.ends_with(suffix), "{source}");
            if parent.contains("中文") {
                assert!(source.contains("/* 中文前注 */") && source.contains("/* 后注 */"));
            }
            let views = crate::build_method::face_views(&tree.0, "move-graft-test").unwrap();
            assert!(
                views
                    .iter()
                    .any(|face| face.path == format!("root/{directory}"))
            );
        }
    }
}

#[test]
fn cross_crate_moves_report_ownership_without_changing_claims_or_unrelated_grafts() {
    for (node, to, owner) in [
        ("root/panel/b", "root/shelf", "widgets -> gauges"),
        ("root/panel/b", "root", "widgets -> host"),
        ("root/loose", "root/panel", "host -> widgets"),
        ("root/panel/group", "root/shelf", "widgets -> gauges"),
    ] {
        for applying in [false, true] {
            let tree = Tree::fixture();
            tree.face("panel/group", "panel", true);
            tree.face("panel/group/inner", "panel/group", false);
            let claims = "pub fn add_crates() -> Shape { Shape::of(\"app\", &[Crate::named(\"widgets\").at(&[crate::panel::SUBTREE]), Crate::named(\"gauges\").at(&[crate::shelf::SUBTREE])]) }";
            std::fs::write(tree.0.join("add_crates.rs"), claims).unwrap();
            tree.plan("cut(crate::panel::c::NODE_ID) graft(external::replacement::NODE_ID),");
            let entry = std::fs::read(tree.0.join("src/lib.rs")).unwrap();
            let outcome =
                run_move(&tree.0, &tree.0, applying, &json!({"node":node,"to":to})).unwrap();
            assert!(
                outcome
                    .message
                    .contains(&format!("notice: compilation owner {owner}")),
                "{}",
                outcome.message
            );
            assert!(!outcome.message.to_lowercase().contains("warning"));
            assert_eq!(
                std::fs::read_to_string(tree.0.join("add_crates.rs")).unwrap(),
                claims
            );
            assert_eq!(std::fs::read(tree.0.join("src/lib.rs")).unwrap(), entry);
            let views = crate::build_method::face_views(&tree.0, "move-graft-test").unwrap();
            let moved = format!("{to}/{}", node.rsplit('/').next().unwrap());
            assert!(views.iter().any(|view| view.path == moved));
            assert!(!views.iter().any(|view| view.path == node));
            if node.ends_with("group") {
                assert!(
                    views
                        .iter()
                        .any(|view| view.path == format!("{moved}/inner"))
                );
            }
            assert_eq!(tree.0.join(".xirang/moves").exists(), applying);
        }
    }
}

#[test]
fn same_crate_moves_do_not_require_detaching_the_partition() {
    for claims in [
        "Crate::named(\"widgets\").at(&[crate::panel::SUBTREE])",
        "Crate::named(\"widgets\").at(&[crate::panel::SUBTREE, crate::shelf::SUBTREE])",
    ] {
        let tree = Tree::fixture();
        tree.face("panel/group", "panel", true);
        let declaration =
            format!("pub fn add_crates() -> Shape {{ Shape::of(\"app\", &[{claims}]) }}");
        std::fs::write(tree.0.join("add_crates.rs"), &declaration).unwrap();
        tree.plan("cut(crate::panel::c::NODE_ID) graft(external::replacement::NODE_ID),");
        let to = if claims.contains("shelf") {
            "root/shelf"
        } else {
            "root/panel/group"
        };
        run_move(
            &tree.0,
            &tree.0,
            true,
            &json!({"node":"root/panel/b","to":to}),
        )
        .unwrap();
        assert_eq!(
            std::fs::read_to_string(tree.0.join("add_crates.rs")).unwrap(),
            declaration
        );
        let views = crate::build_method::face_views(&tree.0, "move-graft-test").unwrap();
        assert!(views.iter().any(|view| view.path == format!("{to}/b")));
    }
}

#[test]
fn closeout_move_accepts_generated_root_parent_tokens() {
    for applying in [false, true] {
        let tree = Tree::fixture();
        let original = Tree::fixture();
        let source = tree.0.join("src/loose/loose.rs");
        let text = std::fs::read_to_string(&source).unwrap();
        std::fs::write(
            &source,
            text.replace(
                ROOT_PARENT_SPELLING,
                "crate::root_node_id(env!(\"CARGO_PKG_NAME\"))",
            ),
        )
        .unwrap();
        run_move(
            &original.0,
            &tree.0,
            applying,
            &json!({"node":"root/loose","to":"root/shelf"}),
        )
        .unwrap();
        let faces = crate::build_method::face_views(&tree.0, "move-graft-test").unwrap();
        assert!(faces.iter().any(|face| face.path == "root/shelf/loose"));
        assert!(!source.exists());
        assert!(original.0.join("src/loose/loose.rs").exists());
    }
}

#[test]
fn closeout_move_preserves_literals_comments_and_other_macro_names() {
    for (node, name, file, destination) in [
        ("root/panel/b", "b", "panel/b/b.rs", "shelf/b/b.rs"),
        (
            "root/panel",
            "dashboard",
            "panel/b/b.rs",
            "shelf/dashboard/b/b.rs",
        ),
    ] {
        let tree = Tree::fixture();
        let path = tree.0.join("src").join(file);
        let behavior = "\n// panel_object! must stay literal\npub fn message() -> &'static str { r#\"panel_object! parent: crate::panel::NODE_ID,\"# }\nmacro_rules! other_panel_object { () => {} }\npub fn invoke_other() { other_panel_object!(); }\n";
        std::fs::write(&path, std::fs::read_to_string(&path).unwrap() + behavior).unwrap();
        run_move(
            &tree.0,
            &tree.0,
            true,
            &json!({"node":node,"to":"root/shelf","name":name}),
        )
        .unwrap();
        let after = std::fs::read_to_string(tree.0.join("src").join(destination)).unwrap();
        assert!(
            after.ends_with(behavior),
            "business source changed: {after}"
        );
    }
}

#[test]
fn closeout_record_failure_restores_the_original_source() {
    for node in ["root/panel/b", "root/panel"] {
        let tree = Tree::fixture();
        std::fs::create_dir_all(tree.0.join(".xirang")).unwrap();
        std::fs::write(tree.0.join(".xirang/moves"), "occupied").unwrap();
        let before: Vec<_> = [
            "panel/panel.rs",
            "panel/a/a.rs",
            "panel/b/b.rs",
            "panel/c/c.rs",
            "shelf/shelf.rs",
        ]
        .into_iter()
        .map(|file| {
            let path = tree.0.join("src").join(file);
            let bytes = std::fs::read(&path).unwrap();
            (path, bytes)
        })
        .collect();
        let result = run_move(
            &tree.0,
            &tree.0,
            true,
            &json!({"node":node,"to":"root/shelf","name":"renamed"}),
        );
        assert!(result.is_err());
        for (path, bytes) in before {
            assert!(
                path.exists(),
                "a failed move must restore {}",
                path.display()
            );
            assert_eq!(std::fs::read(path).unwrap(), bytes);
        }
        assert!(!tree.0.join("src/shelf/renamed").exists());
        assert_eq!(
            std::fs::read_to_string(tree.0.join(".xirang/moves")).unwrap(),
            "occupied"
        );
    }
}

#[test]
fn closeout_preview_checks_collisions_and_derives_the_moved_copy() {
    let tree = Tree::fixture();
    let preview = Tree::fixture();
    std::fs::write(
        preview.0.join("src/loose/renamed.rs"),
        "pub fn unrelated() {}\n",
    )
    .unwrap();
    let args = json!({"node":"root/loose","to":"root","name":"renamed"});
    assert!(run_move(&tree.0, &preview.0, false, &args).is_err());
    std::fs::remove_file(preview.0.join("src/loose/renamed.rs")).unwrap();
    run_move(&tree.0, &preview.0, false, &args).unwrap();
    let faces = crate::build_method::face_views(&preview.0, "move-graft-test").unwrap();
    assert!(faces.iter().any(|face| face.path == "root/renamed"));
    assert!(!preview.0.join("src/loose").exists());
    assert!(tree.0.join("src/loose/loose.rs").exists());
    assert!(!tree.0.join(".xirang/moves").exists());
    assert!(!preview.0.join(".xirang/moves").exists());
}

#[test]
fn closeout_moves_check_the_graft_implementation_and_its_ancestors() {
    for (cut, node) in [
        (
            "cut(crate::panel::b::NODE_ID) graft(crate::loose::NODE_ID),",
            "root/loose",
        ),
        (
            "cut(crate::loose::NODE_ID) graft(crate::panel::b::NODE_ID),",
            "root/panel",
        ),
        (
            "cut(crate::panel::b::NODE_ID) graft(self::loose::NODE_ID),",
            "root/loose",
        ),
    ] {
        let tree = Tree::fixture();
        tree.plan(cut);
        let error = tree.refused(node, "root/shelf", None);
        assert!(error.contains("implementation"), "{error}");
    }
}

#[test]
fn native_catalog_grafts_keep_the_move_impact_and_repair_channel() {
    let tree = Tree::fixture();
    for module in ["panel", "shelf", "panel/a", "panel/b", "panel/c", "loose"] {
        let path = tree.0.join(format!(
            "src/{module}/{}.rs",
            module.rsplit('/').next().unwrap()
        ));
        let source = std::fs::read_to_string(&path).unwrap();
        std::fs::write(
            path,
            xirang_kernel::syntax::render_native_face(&source)
                .unwrap()
                .replace("const SOIL:", "const DECLARATION:"),
        )
        .unwrap();
    }
    let entry = tree.0.join("src/lib.rs");
    std::fs::write(&entry, "use crate::catalog as ui; const GRAFTS: &[Graft] = &[graft(range(ui::panel::A, ui::panel::C)).with(external::NODE_ID)];").unwrap();
    let error = tree.refused("root/panel/b", "root/shelf", None);
    assert!(error.contains("range"), "{error}");
    // Rebind only the affected declaration, leaving unrelated replacements untouched.
    // 仅重新绑定受影响声明，不触碰无关替换件。
    std::fs::write(
        &entry,
        "const GRAFTS: &[Graft] = &[graft(crate::catalog::panel::C).with(external::NODE_ID)];",
    )
    .unwrap();
    run_move(
        &tree.0,
        &tree.0,
        true,
        &json!({"node":"root/panel/b", "to":"root/shelf"}),
    )
    .unwrap();
    let moved = std::fs::read_to_string(tree.0.join("src/shelf/b/b.rs")).unwrap();
    assert!(
        moved.contains("const DECLARATION:") && moved.contains("pub fn behavior"),
        "{moved}"
    );
    assert_eq!(
        xirang_kernel::syntax::parse_face(&moved)
            .unwrap()
            .unwrap()
            .parent(),
        Some(xirang_kernel::syntax::ParentSyntax::NodePath(
            "crate::shelf".into()
        ))
    );
    let plan = crate::build_method::declared_grafts(&tree.0).unwrap();
    assert_eq!(plan.cuts.len(), 1);
    assert_eq!(
        plan.cuts[0].expressions.as_ref().unwrap().cut,
        "crate::panel::c::NODE_ID"
    );
}

/// A21: the refusal answers the **decidable** question per reference — is the new reference
/// computable from one named moved object — and never pretends that answers whether the graft was
/// meant to follow that object. The tag legend rides with the report so a reader is not left
/// inferring it.
/// A21：拒绝按**每条引用**回答那个**可判定**的问题——新的引用能不能由唯一被点名的被搬对象算出来——并且
/// 绝不假装这回答了"这个嫁接本来就该跟着那个对象吗"。标签图例随报告一起给，读者不必自行推断。
#[test]
fn one_named_object_is_determined_and_a_range_is_ambiguous() {
    let tree = Tree::fixture();
    // One string cut names a single face; the range spans three, so which of them either end
    // follows is not something the mapping can say.
    // 字符串切口只点名一个面；区间跨三个面，因此两端各自该跟谁走不是映射能回答的。
    tree.plan(
        "cut \"root/panel/b\" graft \"replacement\",\ncut(crate::panel::a::NODE_ID to crate::panel::c::NODE_ID) graft(external::replacement::NODE_ID),",
    );
    let error = tree.refused("root/panel", "root/shelf", None);
    assert!(
        error.contains("impact tags:")
            && error.contains("determined = exactly one moved object is named")
            && error.contains("ambiguous = several moved objects are named"),
        "the legend comes with the report: {error}"
    );
    assert!(
        error.contains("[determined]") && error.contains("root/panel/b"),
        "the single named face is computable: {error}"
    );
    assert!(
        error.contains("[ambiguous]") && error.contains("root/panel/a, root/panel/c"),
        "the range names both of its moved endpoints, and that is why it is not computable: {error}"
    );
    assert!(
        error.contains("your decision"),
        "and the report says which part is the person's: {error}"
    );
    assert!(
        error.contains("No automatic repair has been applied"),
        "a plan is not a repair: {error}"
    );
}

/// A full graft changes membership without naming a moved object, so it is `coverage` rather than
/// either of the two reference tags — the three cases have to stay distinguishable.
/// 全覆盖嫁接改变成员集合却不点名任何被搬对象，因此它是 `coverage` 而不是那两种引用标签——三种情形必须
/// 分得开。
#[test]
fn a_membership_change_is_tagged_coverage() {
    let tree = Tree::fixture();
    tree.plan("cut(crate::panel::NODE_ID) full graft(external::replacement::NODE_ID),");
    let error = tree.refused("root/panel/b", "root/shelf", None);
    assert!(
        error.contains("[coverage]") && error.contains("full"),
        "a membership change is neither determined nor ambiguous: {error}"
    );
}

/// The declaration spelling is rewritten in place, and only that endpoint changes.
/// 声明拼写就地在原处改写，且只改那一个端点。
#[test]
fn a_typed_declaration_endpoint_is_rewritten_in_place() {
    let tree = Tree::fixture();
    tree.plan("cut(crate::panel::b::NODE_ID) graft(external::replacement::NODE_ID),");
    tree.applied("root/panel/b", "root/shelf", None, true);
    let text = std::fs::read_to_string(tree.0.join("src/lib.rs")).unwrap();
    assert!(text.contains("crate::shelf::b::NODE_ID"), "{text}");
    assert!(!text.contains("crate::panel::b::NODE_ID"), "{text}");
    assert!(text.contains("external::replacement::NODE_ID"), "{text}");
}

/// The catalog spelling is rewritten in its own form, not normalized to a declaration first.
/// 目录拼写按它自己的形式改写，而不是先归一成声明形式。
#[test]
fn a_catalog_endpoint_keeps_its_catalog_form() {
    let tree = Tree::fixture();
    tree.plan("cut(crate::catalog::panel::B) graft(external::replacement::NODE_ID),");
    tree.applied("root/panel/b", "root/shelf", None, true);
    let text = std::fs::read_to_string(tree.0.join("src/lib.rs")).unwrap();
    assert!(text.contains("crate::catalog::shelf::B"), "{text}");
    assert!(!text.contains("crate::panel::b::NODE_ID"), "{text}");
}

/// A comment and a line break inside the declaration survive; only the endpoint bytes change.
/// 声明里的注释与换行保留；只有端点字节改变。
#[test]
fn a_comment_and_a_line_break_inside_the_declaration_survive() {
    let tree = Tree::fixture();
    tree.plan(
        "cut(\n    // the target this slot replaces\n    crate::panel::b::NODE_ID\n) \
         graft(external::replacement::NODE_ID),",
    );
    tree.applied("root/panel/b", "root/shelf", None, true);
    let text = std::fs::read_to_string(tree.0.join("src/lib.rs")).unwrap();
    assert!(text.contains("crate::shelf::b::NODE_ID"), "{text}");
    assert!(
        text.contains("// the target this slot replaces"),
        "the comment is not part of the endpoint: {text}"
    );
}

/// Two different references on one line are addressed individually.
/// 同一行的两处不同引用被分别定位。
#[test]
fn two_references_on_one_line_are_addressed_individually() {
    let tree = Tree::fixture();
    tree.plan(
        "cut(crate::panel::b::NODE_ID) graft(external::one::NODE_ID), \
         cut(crate::panel::a::NODE_ID) graft(external::two::NODE_ID),",
    );
    tree.applied("root/panel/b", "root/shelf", None, true);
    let text = std::fs::read_to_string(tree.0.join("src/lib.rs")).unwrap();
    assert!(text.contains("crate::shelf::b::NODE_ID"), "{text}");
    assert!(
        text.contains("crate::panel::a::NODE_ID"),
        "the unmoved sibling's reference is untouched: {text}"
    );
}

/// Unrelated text that happens to spell the old reference is not a reference and is not edited.
/// 恰好拼出旧引用的无关文本不是引用，不会被编辑。
#[test]
fn unrelated_text_spelling_the_old_reference_is_not_edited() {
    let tree = Tree::fixture();
    tree.plan(
        "// note: crate::panel::b::NODE_ID used to be the target\n\
         cut(crate::panel::b::NODE_ID) graft(external::replacement::NODE_ID),",
    );
    tree.applied("root/panel/b", "root/shelf", None, true);
    let text = std::fs::read_to_string(tree.0.join("src/lib.rs")).unwrap();
    assert!(
        text.contains("// note: crate::panel::b::NODE_ID used to be the target"),
        "the comment is not a declaration endpoint: {text}"
    );
    assert!(text.contains("crate::shelf::b::NODE_ID"), "{text}");
}

/// A spelling this rebind does not know is refused before anything is written.
/// 这次重绑不认识的拼写在写入任何东西之前拒绝。
#[test]
fn an_unknown_typed_spelling_is_refused_before_the_write() {
    let tree = Tree::fixture();
    tree.plan("cut(self::panel::b::NODE_ID) graft(external::replacement::NODE_ID),");
    let before = std::fs::read_to_string(tree.0.join("src/lib.rs")).unwrap();
    let error = tree.refused_with_rebind("root/panel/b", "root/shelf");
    assert!(error.contains("does not know how to rewrite"), "{error}");
    assert_eq!(
        std::fs::read_to_string(tree.0.join("src/lib.rs")).unwrap(),
        before,
        "a refused rebind leaves the entry byte-identical"
    );
}

/// A range that names the moving face is refused before the write; picking one end would be a guess.
/// 点名被搬面的区间在写入前拒绝；挑一端等于替作者猜。
#[test]
fn a_range_that_names_the_moving_face_is_refused_before_the_write() {
    let tree = Tree::fixture();
    tree.plan(
        "cut(crate::panel::b::NODE_ID to crate::panel::c::NODE_ID) \
         graft(external::replacement::NODE_ID),",
    );
    let before = std::fs::read_to_string(tree.0.join("src/lib.rs")).unwrap();
    let error = tree.refused_with_rebind("root/panel/b", "root/shelf");
    assert!(error.contains("range"), "{error}");
    assert_eq!(
        std::fs::read_to_string(tree.0.join("src/lib.rs")).unwrap(),
        before
    );
}
