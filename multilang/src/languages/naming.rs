//! Shared identifier-spelling rules for language adapters.
//! 语言适配器共享的标识符拼写规则。
//!
//! A language adapter may need to turn a logical module segment into an identifier in its own
//! language. The folding rule is shared; the keyword table is the language's own. Keeping both here
//! means Python does not have to borrow the C++ keyword list (or vice versa).
//! 语言适配器可能需要把逻辑模块段变成本语言的标识符。折字规则共享；关键字表属于语言自己。两者放在
//! 这里，意味着 Python 不必借 C++ 的关键字表（反之亦然）。

/// Fold every character an identifier cannot carry to an underscore.
/// 把标识符不能携带的每个字符折成下划线。
pub fn fold(text: &str) -> String {
    let mut name = String::with_capacity(text.len() + 1);
    for (index, character) in text.chars().enumerate() {
        if character.is_ascii_alphanumeric() || character == '_' {
            name.push(character);
        } else {
            name.push('_');
        }
        if index == 0 && character.is_ascii_digit() {
            name.insert(0, '_');
        }
    }
    name
}

/// One identifier segment; a keyword or an empty segment gets a trailing underscore.
/// 一个标识符段；关键字或空段在末尾加下划线。
pub fn segment(text: &str, keywords: &[&str]) -> String {
    let folded = fold(text);
    if keywords.contains(&folded.as_str()) || folded.is_empty() {
        format!("{folded}_")
    } else {
        folded
    }
}

/// The Python keywords a module segment may collide with.
/// 模块段可能撞上的 Python 关键字。
pub const PYTHON_KEYWORDS: &[&str] = &[
    "False", "None", "True", "and", "as", "assert", "async", "await", "break", "class", "continue",
    "def", "del", "elif", "else", "except", "finally", "for", "from", "global", "if", "import",
    "in", "is", "lambda", "nonlocal", "not", "or", "pass", "raise", "return", "try", "while",
    "with", "yield",
];

/// One Python identifier segment.
/// 一个 Python 标识符段。
pub fn python_segment(text: &str) -> String {
    segment(text, PYTHON_KEYWORDS)
}

/// The JavaScript reserved words a module segment may collide with.
/// 模块段可能撞上的 JavaScript 保留字。
///
/// The list is the ECMAScript reserved-word set plus the strict-mode-only words (`implements`,
/// `interface`, `let`, `package`, `private`, `protected`, `public`, `static`, `yield`), because a
/// module is strict by default: a property named after any of them is legal, a **binding** is not,
/// and the generated `faces` directory is read through property access, so a collision only becomes
/// a syntax error when the segment is written as a bare identifier.
/// 这份清单是 ECMAScript 保留字集合加上仅严格模式保留的词（`implements`、`interface`、`let`、`package`、
/// `private`、`protected`、`public`、`static`、`yield`），因为模块默认严格：以它们命名的**属性**合法，
/// **绑定**不合法，而生成的 `faces` 目录经属性访问读取，所以只有段被写成裸标识符时才会变成语法错误。
pub const JAVASCRIPT_KEYWORDS: &[&str] = &[
    "await",
    "break",
    "case",
    "catch",
    "class",
    "const",
    "continue",
    "debugger",
    "default",
    "delete",
    "do",
    "else",
    "enum",
    "export",
    "extends",
    "false",
    "finally",
    "for",
    "function",
    "if",
    "implements",
    "import",
    "in",
    "instanceof",
    "interface",
    "let",
    "new",
    "null",
    "package",
    "private",
    "protected",
    "public",
    "return",
    "static",
    "super",
    "switch",
    "this",
    "throw",
    "true",
    "try",
    "typeof",
    "var",
    "void",
    "while",
    "with",
    "yield",
];

/// One JavaScript identifier segment.
/// 一个 JavaScript 标识符段。
pub fn javascript_segment(text: &str) -> String {
    segment(text, JAVASCRIPT_KEYWORDS)
}

/// The Go keywords a package or identifier segment may collide with.
/// 包名或标识符段可能撞上的 Go 关键字。
///
/// The list is the language's 25 keywords plus the predeclared names a package clause must not
/// shadow carelessly (`true`, `false`, `nil`, `iota`); a package named after a keyword does not
/// compile, and a package named `true` is a different kind of confusion.
/// 清单是语言的 25 个关键字加上包子句不应草率遮蔽的预声明名（`true`、`false`、`nil`、`iota`）；
/// 以关键字命名的包编译不过，而以 `true` 命名的包是另一种混乱。
pub const GO_KEYWORDS: &[&str] = &[
    "break",
    "case",
    "chan",
    "const",
    "continue",
    "default",
    "defer",
    "else",
    "fallthrough",
    "for",
    "func",
    "go",
    "goto",
    "if",
    "import",
    "interface",
    "map",
    "package",
    "range",
    "return",
    "select",
    "struct",
    "switch",
    "type",
    "var",
    "true",
    "false",
    "nil",
    "iota",
];

/// One Go package or identifier segment.
/// 一个 Go 包或标识符段。
pub fn go_segment(text: &str) -> String {
    segment(text, GO_KEYWORDS)
}

/// The Java keywords and reserved literals a package or identifier segment may collide with.
/// 包名或标识符段可能撞上的 Java 关键字与保留字面量。
///
/// `true`, `false` and `null` are literals rather than keywords, but a package named after one is
/// just as uncompileable.
/// `true`、`false`、`null` 是字面量而不是关键字，但以它们命名的包同样编译不过。
pub const JAVA_KEYWORDS: &[&str] = &[
    "abstract",
    "assert",
    "boolean",
    "break",
    "byte",
    "case",
    "catch",
    "char",
    "class",
    "const",
    "continue",
    "default",
    "do",
    "double",
    "else",
    "enum",
    "extends",
    "final",
    "finally",
    "float",
    "for",
    "goto",
    "if",
    "implements",
    "import",
    "instanceof",
    "int",
    "interface",
    "long",
    "native",
    "new",
    "package",
    "private",
    "protected",
    "public",
    "return",
    "short",
    "static",
    "strictfp",
    "super",
    "switch",
    "synchronized",
    "this",
    "throw",
    "throws",
    "transient",
    "try",
    "void",
    "volatile",
    "while",
    "true",
    "false",
    "null",
    "var",
    "record",
    "yield",
    "sealed",
    "permits",
];

/// One Java package or identifier segment.
/// 一个 Java 包或标识符段。
pub fn java_segment(text: &str) -> String {
    segment(text, JAVA_KEYWORDS)
}

/// The C# keywords a namespace or identifier segment may collide with.
/// 命名空间或标识符段可能撞上的 C# 关键字。
pub const CSHARP_KEYWORDS: &[&str] = &[
    "abstract",
    "as",
    "base",
    "bool",
    "break",
    "byte",
    "case",
    "catch",
    "char",
    "checked",
    "class",
    "const",
    "continue",
    "decimal",
    "default",
    "delegate",
    "do",
    "double",
    "else",
    "enum",
    "event",
    "explicit",
    "extern",
    "false",
    "finally",
    "fixed",
    "float",
    "for",
    "foreach",
    "goto",
    "if",
    "implicit",
    "in",
    "int",
    "interface",
    "internal",
    "is",
    "lock",
    "long",
    "namespace",
    "new",
    "null",
    "object",
    "operator",
    "out",
    "override",
    "params",
    "private",
    "protected",
    "public",
    "readonly",
    "ref",
    "return",
    "sbyte",
    "sealed",
    "short",
    "sizeof",
    "stackalloc",
    "static",
    "string",
    "struct",
    "switch",
    "this",
    "throw",
    "true",
    "try",
    "typeof",
    "uint",
    "ulong",
    "unchecked",
    "unsafe",
    "ushort",
    "using",
    "virtual",
    "void",
    "volatile",
    "while",
];

/// One C# namespace or identifier segment.
/// 一个 C# 命名空间或标识符段。
pub fn csharp_segment(text: &str) -> String {
    segment(text, CSHARP_KEYWORDS)
}

/// The C keywords a variable or identifier segment may collide with.
/// 变量或标识符段可能撞上的 C 关键字。
pub const C_KEYWORDS: &[&str] = &[
    "auto",
    "break",
    "case",
    "char",
    "const",
    "continue",
    "default",
    "do",
    "double",
    "else",
    "enum",
    "extern",
    "float",
    "for",
    "goto",
    "if",
    "inline",
    "int",
    "long",
    "register",
    "restrict",
    "return",
    "short",
    "signed",
    "sizeof",
    "static",
    "struct",
    "switch",
    "typedef",
    "union",
    "unsigned",
    "void",
    "volatile",
    "while",
    "_Bool",
    "_Complex",
    "_Imaginary",
];

/// One C variable or identifier segment.
/// 一个 C 变量或标识符段。
pub fn c_segment(text: &str) -> String {
    segment(text, C_KEYWORDS)
}

/// The C spelling of a face module's generated parent constant (`shop::panel` → `SHOP__PANEL`).
/// 一个面模块的生成父常量的 C 拼法（`shop::panel` → `SHOP__PANEL`）。
///
/// The build writes `#define XIRANG_FACES_<flat>` for every face module; the authoring adapter
/// renders the same suffix for a parent it is handed. This is the one implementation both sides
/// call, so a declaration cannot name a constant the header spells differently.
/// 构建为每个面模块写 `#define XIRANG_FACES_<flat>`；创作适配器为收到的父级渲染同一个后缀。这是构建与
/// 适配器共用的唯一实现，因此声明不可能点名一个头文件拼法不同的常量。
pub fn c_flat_module(module: &str) -> String {
    let mut flat = String::new();
    for character in module.replace("::", "__").chars() {
        if character.is_ascii_alphanumeric() {
            flat.push(character.to_ascii_uppercase());
        } else {
            flat.push('_');
        }
    }
    if flat.starts_with(|character: char| character.is_ascii_digit()) {
        flat.insert(0, '_');
    }
    flat
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_fold_is_shared_but_the_keyword_tables_are_not() {
        assert_eq!(fold("area-1"), "area_1");
        assert_eq!(segment("new", &["new"]), "new_");
        assert_eq!(python_segment("class"), "class_");
        assert_eq!(python_segment("new"), "new");
        // The two tables disagree on purpose: `default` is a JavaScript word and a Python name.
        assert_eq!(javascript_segment("default"), "default_");
        assert_eq!(javascript_segment("class"), "class_");
        assert_eq!(javascript_segment("gauge"), "gauge");
        assert_eq!(python_segment("default"), "default");
        // Go's table is its own: `type` is reserved and `class` is an ordinary name.
        assert_eq!(go_segment("type"), "type_");
        assert_eq!(go_segment("class"), "class");
        assert_eq!(go_segment("gauge"), "gauge");
        // Java's table is its own: `class` is reserved and `type` is an ordinary name.
        assert_eq!(java_segment("class"), "class_");
        assert_eq!(java_segment("type"), "type");
        assert_eq!(java_segment("gauge"), "gauge");
        // C#'s table is its own: `class` is reserved and `type` is an ordinary name.
        assert_eq!(csharp_segment("class"), "class_");
        assert_eq!(csharp_segment("type"), "type");
        assert_eq!(csharp_segment("gauge"), "gauge");
        // C's table is its own, and the flat module constant folds `::` to `__`.
        assert_eq!(c_segment("static"), "static_");
        assert_eq!(c_segment("gauge"), "gauge");
        assert_eq!(c_flat_module("panel"), "PANEL");
        assert_eq!(c_flat_module("shop::panel"), "SHOP__PANEL");
        assert_eq!(c_flat_module("1panel"), "_1PANEL");
    }
}
