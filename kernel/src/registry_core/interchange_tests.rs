//! Tests for the interchange vocabulary: identities, the capability handshake, and the export
//! subset's C ABI spelling. They live beside the module so the module itself stays under the size
//! budget.
//! 交换词汇的测试：身份、能力握手与导出子集的 C ABI 拼写。它们与模块同级存放，使模块本身不超过
//! 尺寸预算。

use super::{Capability, Export, Handshake, LanguageId, ParameterType, ReturnType, Scalar};

fn language(text: &str) -> LanguageId {
    LanguageId::parse_language(text).expect("a valid language identity")
}

#[test]
fn a_language_identity_is_mechanical_and_refuses_two_spellings_of_one_name() {
    for valid in ["cpp", "c", "c-sharp", "python3", "objective_c"] {
        assert!(LanguageId::parse_language(valid).is_some(), "{valid}");
    }
    for invalid in ["", "Cpp", "9lives", "c++", "a".repeat(33).as_str()] {
        assert!(LanguageId::parse_language(invalid).is_none(), "{invalid}");
    }
}

#[test]
fn a_missing_ability_is_refused_by_name_not_discovered_halfway() {
    let handshake = Handshake::new(
        language("cpp"),
        "g++",
        [
            Capability::Discovery,
            Capability::Resolution,
            Capability::BuildTest,
        ],
    );
    assert!(
        handshake
            .require(&[Capability::Discovery, Capability::BuildTest])
            .is_ok()
    );
    let refused = handshake.require(&[Capability::Export]).unwrap_err();
    assert!(refused.contains("export"), "{refused}");
    assert!(refused.contains("cpp"), "{refused}");
    assert!(refused.contains("g++"), "{refused}");
    assert_eq!(
        handshake.declared(),
        ["discovery", "resolution", "build-test"]
    );
}

#[test]
fn the_scalar_subset_parses_and_everything_else_is_refused_by_name() {
    assert_eq!(Scalar::parse_scalar("i64").unwrap(), Scalar::I64);
    assert_eq!(Scalar::parse_scalar("utf8").unwrap(), Scalar::Utf8);
    for (text, reason) in [
        ("optional<i64>", "does not lay it out"),
        ("record ControlHandle", "not in the export subset"),
        ("result<i64, string>", "not in the export subset"),
        ("u128", "not an export scalar"),
    ] {
        let refused = Scalar::parse_scalar(text).unwrap_err();
        assert!(refused.contains(reason), "{text}: {refused}");
    }
}

#[test]
fn a_scalar_export_renders_its_c_prototype() {
    let export = Export {
        name: "add".to_owned(),
        parameters: vec![
            ("a", ParameterType::Scalar(Scalar::I64)),
            ("b", ParameterType::Scalar(Scalar::I64)),
        ],
        returns: Some(ReturnType::Scalar(Scalar::I64)),
        errors: Vec::new(),
    };
    assert_eq!(
        export.c_prototype().unwrap(),
        "int64_t add(int64_t a, int64_t b)"
    );
}

#[test]
fn text_is_a_parameter_with_a_length_and_never_a_return_in_this_slice() {
    let export = Export {
        name: "greet".to_owned(),
        parameters: vec![("name", ParameterType::Scalar(Scalar::Utf8))],
        returns: None,
        errors: Vec::new(),
    };
    assert_eq!(
        export.c_prototype().unwrap(),
        "void greet(const char *name, size_t name_len)"
    );
    let returning = Export {
        name: "name_of".to_owned(),
        parameters: Vec::new(),
        returns: Some(ReturnType::Scalar(Scalar::Utf8)),
        errors: Vec::new(),
    };
    let refused = returning.c_prototype().unwrap_err();
    assert!(refused.contains("ownership"), "{refused}");
}

#[test]
fn bad_names_and_duplicate_parameters_are_refused() {
    let bad_name = Export {
        name: "add two".to_owned(),
        parameters: Vec::new(),
        returns: None,
        errors: Vec::new(),
    };
    assert!(bad_name.c_prototype().is_err());
    let duplicate = Export {
        name: "add".to_owned(),
        parameters: vec![
            ("a", ParameterType::Scalar(Scalar::I64)),
            ("a", ParameterType::Scalar(Scalar::I32)),
        ],
        returns: None,
        errors: Vec::new(),
    };
    let refused = duplicate.c_prototype().unwrap_err();
    assert!(refused.contains("twice"), "{refused}");
}

#[test]
fn a_text_return_is_a_caller_owned_buffer_whose_length_the_call_reports() {
    let export = Export {
        name: "greet".to_owned(),
        parameters: vec![("name", ParameterType::Scalar(Scalar::Utf8))],
        returns: Some(ReturnType::Text),
        errors: Vec::new(),
    };
    assert_eq!(
        export.c_prototype().unwrap(),
        "bool greet(const char *name, size_t name_len, char *out, size_t cap, size_t *out_len)"
    );
    let bytes = Export {
        name: "snapshot".to_owned(),
        parameters: Vec::new(),
        returns: Some(ReturnType::Bytes),
        errors: Vec::new(),
    };
    assert_eq!(
        bytes.c_prototype().unwrap(),
        "bool snapshot(uint8_t *out, size_t cap, size_t *out_len)"
    );
    assert!(matches!(
        ReturnType::parse_return("text"),
        Ok(ReturnType::Text)
    ));
}

#[test]
fn an_optional_return_is_a_struct_and_optional_text_is_refused_by_name() {
    let export = Export {
        name: "maybe".to_owned(),
        parameters: vec![("value", ParameterType::Scalar(Scalar::I64))],
        returns: Some(ReturnType::parse_return("optional<i64>").unwrap()),
        errors: Vec::new(),
    };
    assert_eq!(
        export.c_prototype().unwrap(),
        "xirang_optional_i64 maybe(int64_t value)"
    );
    assert_eq!(
        export
            .returns
            .expect("a return")
            .c_typedef()
            .unwrap()
            .expect("a typedef"),
        "typedef struct { bool present; int64_t value; } xirang_optional_i64;"
    );
    assert!(ReturnType::parse_return("optional<utf8>").is_err());
    assert!(ReturnType::parse_return("optional<nope>").is_err());
    assert!(matches!(
        ReturnType::parse_return("optional<bool>"),
        Ok(ReturnType::Optional(Scalar::Bool))
    ));
}

#[test]
fn an_optional_parameter_arrives_as_the_same_struct_and_text_is_refused() {
    let export = Export {
        name: "pick".to_owned(),
        parameters: vec![
            ("value", ParameterType::Optional(Scalar::I64)),
            ("fallback", ParameterType::Scalar(Scalar::I64)),
        ],
        returns: Some(ReturnType::Scalar(Scalar::I64)),
        errors: Vec::new(),
    };
    assert_eq!(
        export.c_prototype().unwrap(),
        "int64_t pick(xirang_optional_i64 value, int64_t fallback)"
    );
    assert_eq!(
        ParameterType::Optional(Scalar::I64)
            .c_typedef()
            .expect("a typedef"),
        "typedef struct { bool present; int64_t value; } xirang_optional_i64;"
    );
    assert!(ParameterType::parse_parameter("optional<utf8>").is_err());
    assert!(ParameterType::parse_parameter("text").is_err());
    assert!(matches!(
        ParameterType::parse_parameter("optional<u32>"),
        Ok(ParameterType::Optional(Scalar::U32))
    ));
}
#[test]
fn typed_errors_become_a_status_an_out_parameter_and_a_named_enum() {
    let export = Export {
        name: "checked_div".to_owned(),
        parameters: vec![
            ("a", ParameterType::Scalar(Scalar::I64)),
            ("b", ParameterType::Scalar(Scalar::I64)),
        ],
        returns: Some(ReturnType::Scalar(Scalar::I64)),
        errors: vec!["divide_by_zero".to_owned(), "overflow".to_owned()],
    };
    assert_eq!(
        export.c_prototype().unwrap(),
        "int32_t checked_div(int64_t a, int64_t b, int64_t *out, \
             xirang_error_checked_div *error)"
    );
    let typedef = export.c_error_typedef().expect("an enum typedef");
    assert!(typedef.contains("XIRANG_ERROR_CHECKED_DIV_DIVIDE_BY_ZERO = 1"));
    assert!(typedef.contains("XIRANG_ERROR_CHECKED_DIV_OVERFLOW = 2"));
    let mut duplicate = export.clone();
    duplicate.errors = vec!["twice".to_owned(), "twice".to_owned()];
    assert!(duplicate.validate_export().is_err());
    let mut text = export.clone();
    text.returns = Some(ReturnType::Text);
    assert!(text.validate_export().is_err());
    let mut unnamed = export.clone();
    unnamed.errors = vec!["9lives".to_owned()];
    assert!(unnamed.validate_export().is_err());
}

#[test]
fn every_capability_word_parses_back_and_unknown_words_name_nothing() {
    for capability in Capability::ALL {
        assert_eq!(
            Capability::parse_ability(capability.name()),
            Some(capability)
        );
    }
    assert_eq!(Capability::parse_ability("teleport"), None);
    assert_eq!(Capability::parse_ability(""), None);
}
