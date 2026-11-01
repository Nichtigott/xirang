//! The C++ spelling of one XiRang face reference.
//! XiRang 面引用的 C++ 拼法。
//!
//! The generated header names every face twice: once as a nested namespace path that reads like the
//! logical tree (`xirang::faces::shop::panel::NODE_ID`), and once as the flat identifier version 1
//! wrote (`shop__panel_Panel`). Both spellings are produced here, so the build and the C++ authoring
//! adapter cannot disagree about a name — an adapter that rendered one spelling while the header
//! wrote another would generate a reference that does not compile.
//! 生成的头部为每个面写两种拼法：一种是读起来像逻辑树的嵌套命名空间路径
//! （`xirang::faces::shop::panel::NODE_ID`），另一种是版本 1 写出的扁平标识符
//! （`shop__panel_Panel`）。两种拼法都在这里产生，因此构建与 C++ 创作适配器不会对同一个名字产生分歧——
//! 适配器渲染一种、头部写另一种，生成出来的引用就编译不过。
//!
//! A module segment can be a C++ keyword (`type`, `new`, `template`), and `namespace new {}` does not
//! compile. A keyword segment is therefore spelled with a trailing underscore, and that rule is part
//! of this module rather than of either consumer.
//! 模块段可以是 C++ 关键字（`type`、`new`、`template`），而 `namespace new {}` 编译不过。因此关键字段
//! 在末尾加一个下划线，这条规则住在本模块里，而不是住在两个消费者各自那里。

/// The C++ keywords a module segment may collide with.
/// 模块段可能撞上的 C++ 关键字。
///
/// Contextual identifiers (`final`, `override`, `import`, `module`) are absent on purpose: C++ allows
/// them as ordinary identifiers, so escaping them would rename a face that compiles fine today.
/// 上下文标识符（`final`、`override`、`import`、`module`）有意不在表里：C++ 允许它们作普通标识符，
/// 转义它们会给一个今天本来能编译的面改名。
pub const KEYWORDS: &[&str] = &[
    "alignas",
    "alignof",
    "and",
    "and_eq",
    "asm",
    "auto",
    "bitand",
    "bitor",
    "bool",
    "break",
    "case",
    "catch",
    "char",
    "char8_t",
    "char16_t",
    "char32_t",
    "class",
    "compl",
    "concept",
    "const",
    "consteval",
    "constexpr",
    "constinit",
    "const_cast",
    "continue",
    "co_await",
    "co_return",
    "co_yield",
    "decltype",
    "default",
    "delete",
    "do",
    "double",
    "dynamic_cast",
    "else",
    "enum",
    "explicit",
    "export",
    "extern",
    "false",
    "float",
    "for",
    "friend",
    "goto",
    "if",
    "inline",
    "int",
    "long",
    "mutable",
    "namespace",
    "new",
    "noexcept",
    "not",
    "not_eq",
    "nullptr",
    "operator",
    "or",
    "or_eq",
    "private",
    "protected",
    "public",
    "register",
    "reinterpret_cast",
    "requires",
    "return",
    "short",
    "signed",
    "sizeof",
    "static",
    "static_assert",
    "static_cast",
    "struct",
    "switch",
    "template",
    "this",
    "thread_local",
    "throw",
    "true",
    "try",
    "typedef",
    "typeid",
    "typename",
    "union",
    "unsigned",
    "using",
    "virtual",
    "void",
    "volatile",
    "wchar_t",
    "while",
    "xor",
    "xor_eq",
];

pub use crate::languages::kit::names::{fold, module_of, path_segment_problem};

/// One namespace segment a module path contributes.
/// 模块路径贡献的一个命名空间段。
///
/// A keyword gets a trailing underscore so the segment stays an identifier; everything else keeps
/// its own spelling, so a normal module path reads exactly like the logical tree.
/// 关键字段在末尾加一个下划线以保持是标识符；其余段保持自己的拼写，因此普通模块路径读起来就是逻辑树。
pub fn segment(text: &str) -> String {
    crate::languages::naming::segment(text, KEYWORDS)
}

/// The nested namespace path for a module, without the `xirang::faces::` prefix.
/// 一个模块的嵌套命名空间路径，不含 `xirang::faces::` 前缀。
pub fn nested(module: &str) -> String {
    module
        .split("::")
        .filter(|segment| !segment.is_empty())
        .map(segment)
        .collect::<Vec<_>>()
        .join("::")
}

/// The version-1 flat identifier for one face: the module folded to `__`, then `_<kind>`.
/// 一个面的版本 1 扁平标识符：模块折成 `__`，再加 `_<kind>`。
pub fn flat(module: &str, kind: &str) -> String {
    let mut name = fold(module.replace("::", "__").as_str());
    name.push('_');
    name.push_str(&fold(kind));
    name
}

/// How a declaration in `module` names the face at `module` itself.
/// `module` 里的声明如何点名该模块自己的面。
pub fn node_id(module: &str) -> String {
    format!("::xirang::faces::{}::NODE_ID", nested(module))
}

/// The C++ spelling of one logical catalog path (`shop/panel/Area` → `shop::panel::Area`).
/// 一条逻辑目录路径的 C++ 拼法（`shop/panel/Area` → `shop::panel::Area`）。
pub fn catalog_path(path: &str) -> String {
    path.trim_start_matches("root/")
        .split('/')
        .filter(|segment| !segment.is_empty())
        .map(|segment| {
            let mut out: String = segment
                .chars()
                .map(|character| {
                    if character.is_ascii_alphanumeric() || character == '_' {
                        character
                    } else {
                        '_'
                    }
                })
                .collect();
            if out.is_empty()
                || out
                    .chars()
                    .next()
                    .is_some_and(|character| character.is_ascii_digit())
            {
                out.insert(0, '_');
            }
            if matches!(
                out.as_str(),
                "namespace"
                    | "class"
                    | "struct"
                    | "enum"
                    | "union"
                    | "template"
                    | "operator"
                    | "new"
                    | "delete"
                    | "this"
                    | "crate"
            ) {
                out.insert(0, '_');
            }
            out
        })
        .collect::<Vec<_>>()
        .join("::")
}

/// The generated constant that names this package's registry root.
/// 本包注册树根的生成常量。
pub fn root_node_id() -> &'static str {
    crate::languages::cpp::header::ROOT_NODE_ID
}

#[cfg(test)]
#[path = "names_tests.rs"]
mod cpp_names_tests;
