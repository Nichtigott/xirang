#![cfg(feature = "syntax")]

use xirang_kernel::syntax::{
    graft_entries, parse_face, relocate_face_declaration, render_native_face,
};

#[test]
fn soil_and_legacy_face_are_discovered_and_relocated_as_native_declarations() {
    for (attribute, ty) in [("soil", "Soil"), ("face", "Face")] {
        let source = format!(
            "#[xirang_toolchain::{attribute}] pub const BUTTON: {ty} = {ty}::of::<Button>().under(crate::a::NODE_ID);"
        );
        assert!(xirang_kernel::syntax::is_face_source(
            &source,
            "// generated"
        ));
        let parsed = parse_face(&source).unwrap().unwrap();
        assert!(parsed.is_native());
        assert_eq!(parsed.macro_name, attribute);
        let moved =
            relocate_face_declaration(&source, "crate::b::NODE_ID", Some(("a_object", "b_object")))
                .unwrap();
        assert_eq!(
            moved,
            source.replace("crate::a::NODE_ID", "crate::b::NODE_ID")
        );
        let edited = xirang_kernel::syntax::replace_face_macro(
            &source,
            "#[xirang_toolchain::soil] pub const SOIL: Soil = Soil::of::<Button>().under(crate::a::NODE_ID).name(\"New\", \"新\");",
        ).unwrap();
        assert!(edited.contains(&format!(
            "#[xirang_toolchain::{attribute}] pub const BUTTON: {ty}"
        )));
        assert_eq!(
            parse_face(&edited)
                .unwrap()
                .unwrap()
                .localized("name", "en")
                .as_deref(),
            Some("New")
        );
    }
}

#[test]
fn native_round_trip_preserves_every_legacy_registration_field() {
    let source = r#"external_object! {
        source: "button.rs", kind: Button, preset: Preset, parts: Parts,
        name: {zh: "按钮", en: "Button"}, summary: {zh: "说明", en: "Summary"},
        exports: ["click"], stable_name: "button", needs_registry: true,
        parent: crate::panel::NODE_ID, getting_from_other_registry: Some("external"),
        registry_rule_path: "rule.rs", registry_rule: rules::RULE,
        admission: Admission::ANY, handle_traits: ["Label"], handle_contracts: [Handle],
        part_traits: ["PartLabel"], part_contracts: [Part],
        requires: ["render" => "Renderer", "layout" => "Layout"], provides: ["click"],
        flow: FlowContract::NONE, flow_provider: Flow,
        plugin: PLUGIN, runtime_checks: [FINITE_NUMBER],
    }"#;
    let before = parse_face(source).unwrap().unwrap();
    let output = render_native_face(source).unwrap();
    let after = parse_face(&output).unwrap().unwrap();
    assert_eq!(after.macro_name, "soil");
    for field in xirang_kernel::declaration::FACE_FIELD_ORDER {
        assert_eq!(before.field(field), after.field(field), "{field}: {output}");
    }
}

#[test]
fn relocating_a_native_face_only_changes_its_parent_argument() {
    let source = "// keep 中文\n#[xirang_toolchain::soil]\npub const BUTTON: Soil = Soil::of::<Button>().name(\"按钮\", \"按钮\").under(crate::a::NODE_ID);\nfn business() {}";
    let moved =
        relocate_face_declaration(source, "crate::b::NODE_ID", Some(("a_object", "b_object")))
            .unwrap();
    assert_eq!(
        moved,
        source.replace("crate::a::NODE_ID", "crate::b::NODE_ID")
    );
}

#[test]
fn native_graft_array_keeps_ranges_descendants_and_cfg() {
    let source = r#"
        #[cfg(feature = "fast")]
        pub const GRAFTS: &[Graft] = &[
            graft(crate::a::NODE_ID).with(fast::NODE_ID),
            graft(range(crate::a::NODE_ID, crate::z::NODE_ID)).with(fast::NODE_ID),
            graft(subtree(crate::a::NODE_ID)).with(fast::NODE_ID),
        ];
        fn runtime() { const LOCAL: &[Graft] = &[graft(crate::x::NODE_ID).with(fast::NODE_ID)]; }
        #[cfg(test)] mod tests { const TEST: &[Graft] = &[graft("a").with("b")]; }
    "#;
    let entries = graft_entries(source).unwrap();
    assert_eq!(entries.len(), 3);
    assert_eq!(entries[1].cut_end.as_deref(), Some("crate::z::NODE_ID"));
    assert!(entries[2].full);
    assert!(
        entries
            .iter()
            .all(|entry| entry.cfg.as_deref() == Some("feature = \"fast\""))
    );
}

#[test]
fn malformed_native_declarations_are_refused() {
    for expression in [
        "Soil::of::<Button>().unknown(true)",
        "Soil::of::<Button>().under(A).under(B)",
        "Soil::of::<Button>().parts()",
        "Soil::of::<Button>().name(\"missing translation\")",
        "Soil::of::<Button>().requires(\"missing provider\")",
    ] {
        assert!(
            parse_face(&format!(
                "#[xirang_toolchain::soil] const F: Soil = {expression};"
            ))
            .is_err()
        );
    }
    assert!(graft_entries("const G: &[Graft] = &[graft(\"a\").with(fast::NODE_ID)];").is_err());
}

#[test]
fn editing_native_faces_keeps_the_public_item_and_attributes() {
    let source = "/// Button docs 中文\n#[cfg(feature = \"button\")]\n#[xirang_toolchain::soil(collector = debug)]\npub(crate) const BUTTON: Soil = Soil::of::<Button>().name(\"Old\", \"旧\");\nfn business() {}";
    let replacement = "#[xirang_toolchain::soil] pub const SOIL: Soil = Soil::of::<Button>().name(\"New\", \"新\");";
    let edited = xirang_kernel::syntax::replace_face_macro(source, replacement).unwrap();
    assert_eq!(
        edited,
        source.replace(".name(\"Old\", \"旧\")", ".name(\"New\", \"新\")")
    );
    let legacy = "/// Keep docs\n#[cfg(feature = \"button\")]\nroot_object! { kind: Button }";
    let converted = render_native_face(legacy).unwrap();
    assert!(
        converted
            .starts_with("/// Keep docs\n#[cfg(feature = \"button\")]\n#[xirang_toolchain::soil]")
    );
}

#[test]
fn native_metadata_does_not_widen_executable_source_scope() {
    let source = "#[xirang_toolchain::soil] const F: Soil = Soil::of::<Button>().under(crate::catalog::PANEL).handle_contracts(contracts!(Button: crate::Contract)); fn run() { crate::business::run(); }";
    let references = xirang_kernel::syntax::source_references(source).unwrap();
    assert_eq!(
        references.paths.into_iter().collect::<Vec<_>>(),
        ["crate::business::run"]
    );
    assert!(!references.conservative);
}

#[test]
fn migrating_a_legacy_collector_keeps_its_submission_policy() {
    let source = "external_object! { collector: debug, kind: Widget }";
    let converted = render_native_face(source).unwrap();
    assert!(
        converted.contains("#[xirang_toolchain::soil(collector = debug)]"),
        "{converted}"
    );
}

#[test]
fn promoting_one_native_graft_preserves_aliases_comments_and_other_entries() {
    let source = "use crate::catalog as ui; // 中文\n#[cfg(feature = \"fast\")]\nconst GRAFTS: &[Graft] = &[graft(ui::BUTTON).with(/* keep 中文 */ fast::BUTTON), graft(ui::SLIDER).with(fast::SLIDER)];";
    let entries = graft_entries(source).unwrap();
    let edited =
        xirang_kernel::syntax::repoint_native_graft(source, &entries[0].cut, &entries[0].graft)
            .unwrap()
            .unwrap();
    assert_eq!(edited, source.replace("fast::BUTTON", "ui::BUTTON"));
    let after = graft_entries(&edited).unwrap();
    assert_eq!(after[0].graft, after[0].cut);
    assert_eq!(entries[1].expressions, after[1].expressions);
    assert_eq!(entries[1].cfg, after[1].cfg);
}
