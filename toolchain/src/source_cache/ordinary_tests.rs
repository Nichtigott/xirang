//! Exact-content reuse must preserve parser refusals and declaration visibility.
//! 完整内容复用必须保留解析拒绝及声明可见性。

use super::{Validated, validate};
use crate::source_cache::{application_entries, parse_face};

#[test]
fn concurrent_execution_readers_preserve_native_results() {
    std::thread::scope(|scope| {
        for _ in 0..8 {
            scope.spawn(|| {
                for _ in 0..32 {
                    for source in ["pub fn shared() {}", "pub fn shared() {@}"] {
                        assert_eq!(
                            parse_face(source).map(|face| face.is_none()),
                            xirang_kernel::syntax::parse_face(source).map(|face| face.is_none())
                        );
                        assert_eq!(
                            application_entries(source),
                            xirang_kernel::syntax::application_entries(source)
                        );
                    }
                }
            });
        }
    });
}

#[test]
fn warm_valid_source_does_not_mask_same_length_invalid_edit() {
    let valid = "pub fn cached_ordinary() { let n = 1; }";
    let invalid = "pub fn cached_ordinary() { let n = @; }";
    assert_eq!(valid.len(), invalid.len());
    for _ in 0..2 {
        assert!(parse_face(valid).unwrap().is_none());
        assert!(application_entries(valid).unwrap().is_empty());
        let expected = xirang_kernel::syntax::parse_face(invalid).unwrap_err();
        assert_eq!(parse_face(invalid).unwrap_err(), expected);
        assert_eq!(application_entries(invalid).unwrap_err(), expected);
    }
}

#[test]
fn ordinary_validation_matches_native_errors_including_nesting_guard() {
    let nested = format!("const N: () = {}{};", "(".repeat(200), ")".repeat(200));
    for source in ["", "// empty", "fn broken(", "const X: u8 = ;", &nested] {
        let expected = xirang_kernel::syntax::parse_face(source).map(|_| ());
        assert_eq!(validate(source).unwrap(), expected);
        assert_eq!(validate(source).unwrap(), expected);
    }
}

#[test]
fn declaration_candidates_always_reach_native_parser() {
    for source in [
        "application!(entry = crate::main);",
        "r#application!(entry = crate::main);",
        "crate::root_object! { kind: Root }",
        "crate::alias_object! { kind: Child }",
        "// application!(entry = crate::fake);",
        "const S: &str = \"fake_object! {}\";",
        "application!(malformed);",
        "root_object! {",
    ] {
        assert!(validate(source).is_none(), "{source}");
    }
    assert_eq!(
        application_entries("application!(entry = crate::main);")
            .unwrap()
            .len(),
        1
    );
    assert!(
        parse_face("crate::root_object! { kind: Root }")
            .unwrap()
            .is_some()
    );
    assert!(application_entries("application!(malformed);").is_err());
    assert!(parse_face("root_object! {").is_err());
}

#[test]
fn cache_bounds_include_empty_sources_and_oversized_inputs() {
    let mut cache = Validated::default();
    cache.insert("aa", 4, 2);
    cache.insert("bb", 4, 2);
    cache.insert("bb", 4, 2);
    assert_eq!(cache.bytes, 4);
    assert_eq!(cache.order.len(), 2);
    cache.insert("ccc", 4, 2);
    assert_eq!(cache.sources.len(), 1);
    assert!(cache.sources.contains("ccc"));
    cache.insert("oversized", 4, 2);
    assert_eq!(cache.bytes, 3);
    cache.insert("", 4, 2);
    cache.insert("d", 4, 2);
    assert_eq!(cache.sources.len(), 2);
    assert_eq!(cache.bytes, 1);
    assert!(!cache.sources.contains("ccc"));
}

#[test]
fn native_attributes_bypass_ordinary_reuse_without_penalizing_surface_names() {
    assert!(
        super::validate("fn surface_name() {} struct Interface; fn topsoil() {} struct SoilType;")
            .unwrap()
            .is_ok()
    );
    for source in [
        "#[soil] const F: Soil = Soil::of::<Button>();",
        "#[xirang_toolchain :: soil] const F: Soil = Soil::of::<Button>();",
        "#[face] const F: Face = Face::of::<Button>();",
        "#[xirang_toolchain :: face] const F: Face = Face::of::<Button>();",
    ] {
        assert!(super::validate(source).is_none());
    }
}
