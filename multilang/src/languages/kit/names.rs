//! Shared spelling rules for a face reference: the source-path rule and the one-segment rule.
//! 面引用共享的拼写规则：源码路径规则与"名字必须是一段"的规则。
//!
//! Both are language-neutral: which directories a source's module path carries, and what a display
//! name may not contain, do not depend on the language's syntax. A language keeps only the spellings
//! its own surface needs (nested namespace paths, flat aliases, header constants).
//! 两者都是语言中立的：源码的模块路径带哪些目录、显示名不许包含什么，都不依赖语言的语法。语言只保留
//! 它自己表面需要的拼法（嵌套命名空间路径、扁平别名、头文件常量）。

pub use crate::languages::naming::{fold, segment};

/// The module a declaration's source lives in: the directories under the package root, then the file
/// stem (which does not repeat its directory).
/// 声明所在源码的模块：包根下的目录段，然后文件 stem（与目录同名时不重复自己）。
///
/// This is a pure path rule, and the build needs it without any adapter: the authoring parser reads
/// the alias a Rust child uses for a native parent from it.
/// 这是一条纯路径规则，没有适配器的构建也需要它：创作解析器靠它读 Rust 子面为本地父级使用的别名。
pub fn module_of(source: &str) -> String {
    let path = std::path::Path::new(source);
    let mut segments: Vec<String> = path
        .parent()
        .map(|parent| {
            parent
                .components()
                .filter_map(|component| match component {
                    std::path::Component::Normal(part) => Some(part.to_string_lossy().into_owned()),
                    _ => None,
                })
                .collect()
        })
        .unwrap_or_default();
    let stem = path
        .file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_default();
    if segments.last().map(String::as_str) != Some(stem.as_str()) {
        segments.push(stem);
    }
    segments.join("::")
}

/// Why a declared name cannot be **one logical path segment**, or `None` when it can.
/// 一个声明名为什么不能是**一个逻辑路径段**；能时返回 `None`。
///
/// A face's display name is the last segment of its path, so it must be exactly that: not empty, not
/// a path (no separator, so it cannot smuggle in several segments), and not the two navigation
/// segments. Everything else the current contract allows is kept — including Unicode and the
/// existing mixed-case spellings — because tightening a name that already compiles and is already in
/// someone's `xirang.objects.tsv` would re-identify a face that does not need to change.
/// 面的显示名是它路径的末段，因此必须恰好是一段：不为空、不是路径（不含分隔符，否则会夹带多段）、也不是
/// 两个导航段。当前契约允许的其它写法一律保留——包括 Unicode 与既有的混合大小写拼法——因为收紧一个已经在
/// 编译、且已经写进别人的 `xirang.objects.tsv` 的名字，会给一个不需要变的面重新生成身份。
pub fn path_segment_problem(name: &str) -> Option<String> {
    if name.is_empty() {
        return Some(
            "the name is empty, and a face's name is the last segment of its path".to_owned(),
        );
    }
    if name == "." || name == ".." {
        return Some(format!("`{name}` is a path navigation segment, not a name"));
    }
    if let Some(separator) = name
        .chars()
        .find(|character| matches!(character, '/' | '\\'))
    {
        return Some(format!(
            "it contains the path separator `{separator}`, so it reads as several segments"
        ));
    }
    if name.chars().any(char::is_control) {
        return Some(
            "it contains a control character, which cannot appear in a path segment".to_owned(),
        );
    }
    None
}

#[cfg(test)]
#[path = "names_tests.rs"]
mod names_tests;
