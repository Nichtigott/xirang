//! What the native reader accepts as a declaration, and what it refuses **by name**.
//! 本地读取器接受什么样的声明，以及它按名拒绝什么。
//!
//! A face's display name is one logical path segment: the reader is the single entrance every
//! consumer goes through (the authoring adapter, the build's own reader, and therefore every fact
//! reader), so a name that is actually a path is refused here rather than split into two tree
//! levels by whoever reads it next.
//! 面的显示名是一个逻辑路径段：读取器是每个消费者都经过的唯一入口（创作适配器、构建自己的读取器，以及
//! 由此而来的每个事实读取方），因此"其实是路径"的名字在这里被拒，而不是被下一个读它的人劈成两层树。

use std::sync::atomic::{AtomicU64, Ordering};

/// A throwaway package with one native source that declares one face.
/// 一个含一份本地源码、声明一个面的一次性包。
fn package(label: &str, name: &str) -> std::path::PathBuf {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "xirang-cpp-native-{label}-{}-{sequence}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("native/area")).expect("native directory");
    std::fs::write(
        root.join("native/area/area.cpp"),
        format!(
            "// fixture\n#include \"xirang/soil.hpp\"\n#include \"xirang/faces.hpp\"\n\n\
             struct Area {{}};\n\nXIRANG_SOIL\nconstexpr auto SOIL = ::xirang::soil<Area>()\n    \
             .under(::xirang::faces::panel::NODE_ID)\n    .named(\"{name}\");\n"
        ),
    )
    .expect("native declaration");
    root
}

/// A name that is a path or a navigation segment is refused, and the refusal names the reason.
/// 一个"其实是路径或导航段"的名字会被拒绝，而拒绝会点名原因。
#[test]
fn a_name_that_is_not_one_path_segment_is_refused_by_name() {
    for (name, reason) in [
        ("A/B", "path separator"),
        // The C++ spelling needs the escape, because the literal parser already resolves `\B` to
        // `B`; what the reader sees — and what the test is about — is one backslash in the name.
        // C++ 拼法需要转义，因为字面量解析器已经把 `\B` 解析成 `B`；读取器看到（也正是本测试针对）的
        // 是名字里的一个反斜杠。
        (r"A\\B", "path separator"),
        (".", "navigation segment"),
        ("..", "navigation segment"),
        ("A\u{7}B", "control character"),
    ] {
        let root = package("segment", name);
        let error = crate::languages::cpp::native::read_recipe(&root)
            .expect_err(&format!("`{name}` must be refused"));
        assert!(
            error.contains("one path segment") && error.contains(reason),
            "the refusal names the reason (`{reason}`) for `{name}`: {error}"
        );
        let _ = std::fs::remove_dir_all(&root);
    }
}

/// What the current contract allows stays allowed: Unicode and mixed case are names, not defects.
/// 当前契约允许的写法继续允许：Unicode 与混合大小写是名字，不是缺陷。
#[test]
fn unicode_and_mixed_case_names_are_still_accepted() {
    let root = package("unicode", "面板Panel");
    let (_, faces) = crate::languages::cpp::native::read_recipe(&root)
        .expect("a Unicode name is one segment")
        .expect("the source declares a face");
    assert_eq!(faces[0].name, "面板Panel");
    let _ = std::fs::remove_dir_all(&root);
}
