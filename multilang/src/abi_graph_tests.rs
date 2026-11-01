//! Pins for the language-neutral boundary graph.
//! 语言无关边界图的钉子。

use super::*;

const CPP_AREA: &str = include_str!("../../examples/mixed-shop/shop/native/area/area.cpp");
const RUST_CALL: &str = include_str!("../../examples/mixed-shop/verify/tests/cpp_call.rs");
const CPP_GAUGE: &str = include_str!("../../examples/mixed-shop/shop/native/area/gauge.cpp");

#[test]
fn real_cpp_and_rust_sources_form_one_boundary_edge() {
    let graph = build_graph(&[
        AbiSource {
            language: AbiLanguage::Cpp,
            label: "shop/native/area/area.cpp",
            source: CPP_AREA,
            owner: Some("root/shop/panel/Area"),
        },
        AbiSource {
            language: AbiLanguage::Cpp,
            label: "shop/native/area/gauge.cpp",
            source: CPP_GAUGE,
            owner: Some("root/shop/panel/Area/Gauge"),
        },
        AbiSource {
            language: AbiLanguage::Rust,
            label: "verify/tests/cpp_call.rs",
            source: RUST_CALL,
            owner: None,
        },
    ]);
    assert_eq!(graph.diagnostics, Vec::new(), "{graph:#?}");
    let edge = graph
        .edges
        .iter()
        .find(|edge| {
            graph.sites[edge.declaration].symbol.name == "area"
                && graph.sites[edge.definition].language == AbiLanguage::Cpp
        })
        .expect("the Rust extern declaration finds the C++ definition");
    let definition = &graph.sites[edge.definition];
    assert_eq!(definition.owner.as_deref(), Some("root/shop/panel/Area"));
    assert_eq!(
        definition.signature.params,
        vec![AbiType::I64, AbiType::I64]
    );
    assert_eq!(definition.signature.ret, Some(AbiType::I64));
}

#[test]
fn a_signature_mismatch_is_a_boundary_diagnostic_not_an_edge() {
    let graph = build_graph(&[
        AbiSource {
            language: AbiLanguage::Cpp,
            label: "area.cpp",
            source: "extern \"C\" std::int64_t area(std::int64_t width) { return width; }",
            owner: None,
        },
        AbiSource {
            language: AbiLanguage::Rust,
            label: "call.rs",
            source: "unsafe extern \"C\" {\n    fn area(width: i32) -> i64;\n}",
            owner: None,
        },
    ]);
    assert!(graph.edges.is_empty(), "{graph:#?}");
    assert!(matches!(
        graph.diagnostics.as_slice(),
        [BoundaryDiagnostic::SignatureMismatch { symbol, .. }] if symbol == "area"
    ));
}

#[test]
fn a_declaration_without_a_definition_is_reported() {
    let graph = build_graph(&[AbiSource {
        language: AbiLanguage::Rust,
        label: "call.rs",
        source: "unsafe extern \"C\" {\n    fn missing(width: i64) -> i64;\n}",
        owner: None,
    }]);
    assert!(graph.edges.is_empty());
    assert!(matches!(
        graph.diagnostics.as_slice(),
        [BoundaryDiagnostic::MissingDefinition { symbol, .. }] if symbol == "missing"
    ));
}

#[test]
fn a_python_ctypes_load_joins_the_same_boundary() {
    let python = r#"
import ctypes
lib = ctypes.CDLL("libshop.so")
area = lib.area
area.argtypes = [ctypes.c_int64, ctypes.c_int64]
area.restype = ctypes.c_int64
"#;
    let graph = build_graph(&[
        AbiSource {
            language: AbiLanguage::Cpp,
            label: "area.cpp",
            source: "extern \"C\" std::int64_t area(std::int64_t width, std::int64_t height) { return width * height; }",
            owner: None,
        },
        AbiSource {
            language: AbiLanguage::Python,
            label: "load_area.py",
            source: python,
            owner: None,
        },
    ]);
    assert_eq!(graph.diagnostics, Vec::new(), "{graph:#?}");
    assert_eq!(graph.edges.len(), 1);
    assert_eq!(
        graph.sites[graph.edges[0].declaration].language,
        AbiLanguage::Python
    );
    assert_eq!(graph.sites[graph.edges[0].declaration].role, AbiRole::Load);
}

#[test]
fn normalization_does_not_make_two_unknowns_equal() {
    assert_eq!(normalize_type("std::int64_t"), AbiType::I64);
    assert_eq!(normalize_type("i64"), AbiType::I64);
    assert_eq!(normalize_type("uint32_t"), AbiType::U32);
    assert_eq!(normalize_type("f64"), AbiType::F64);
    assert_eq!(
        normalize_type("not_a_type"),
        AbiType::Unknown("not_a_type".to_owned())
    );
    assert_ne!(normalize_type("not_a_type"), normalize_type("another_type"));
}
