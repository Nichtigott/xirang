//! Pins for the shared spelling rules: the source-path rule and the one-segment rule.
//! 共享拼写规则的钉子：源码路径规则与"名字必须是一段"的规则。

use super::{module_of, path_segment_problem};

/// The module path is the directories under the package root plus the stem, and the stem is not
/// repeated when it already is the last directory.
/// 模块路径是包根下的目录段加 stem；当 stem 已经与最后一个目录同名时不再重复。
#[test]
fn a_source_becomes_its_module_path() {
    assert_eq!(module_of("native/area/area.cpp"), "native::area");
    assert_eq!(module_of("src/knob/knob.rs"), "src::knob");
    assert_eq!(module_of("area.cpp"), "area");
    assert_eq!(module_of("a/b/other.cpp"), "a::b::other");
}

/// A display name is exactly one path segment: the refusals name the reason, and the spellings the
/// contract already allows stay allowed.
/// 显示名恰好是一段：拒绝点名原因，而契约已经允许的写法继续允许。
#[test]
fn a_display_name_is_one_path_segment() {
    for (name, reason) in [
        ("", "empty"),
        (".", "navigation"),
        ("..", "navigation"),
        ("A/B", "path separator"),
        (r"A\B", "path separator"),
        ("A\u{7}B", "control character"),
    ] {
        let problem = path_segment_problem(name).expect("the name is not one segment");
        assert!(
            problem.contains(reason),
            "the refusal names `{reason}` for `{name}`: {problem}"
        );
    }
    assert!(
        path_segment_problem("Area").is_none(),
        "an ordinary name is a segment"
    );
    assert!(
        path_segment_problem("面板Panel").is_none(),
        "Unicode is a segment, not a defect"
    );
}
