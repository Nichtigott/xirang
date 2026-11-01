//! The one source language a unit is written in, and the per-language table identity.
//! 一个单元所使用的唯一源码语言，以及各语言表的身份。

/// The `[language.cpp]` table.
/// `[language.cpp]` 表。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Cpp {
    pub(crate) standard: String,
}

/// The `[language.c]` table.
/// `[language.c]` 表。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct C {
    pub(crate) standard: String,
}

/// The `[language.python]` table.
/// `[language.python]` 表。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Python {
    pub(crate) standard: String,
}

/// The `[language.rust]` table.
/// `[language.rust]` 表。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Rust {
    pub(crate) standard: String,
}

/// The `[language.go]` table.
/// `[language.go]` 表。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Go {
    pub(crate) standard: String,
}

/// The `[language.ruby]` table.
/// `[language.ruby]` 表。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Ruby {
    pub(crate) standard: String,
}

/// The `[language.csharp]` table.
/// `[language.csharp]` 表。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Csharp {
    pub(crate) standard: String,
}

/// The `[language.java]` table.
/// `[language.java]` 表。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Java {
    pub(crate) standard: String,
}

/// The `[language.php]` table.
/// `[language.php]` 表。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Php {
    pub(crate) standard: String,
}

/// The `[language.swift]` table.
/// `[language.swift]` 表。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Swift {
    pub(crate) standard: String,
}

/// The `[language.kotlin]` table.
/// `[language.kotlin]` 表。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Kotlin {
    pub(crate) standard: String,
}

/// The `[language.javascript]` table.
/// `[language.javascript]` 表。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Javascript {
    pub(crate) standard: String,
}

/// The `[language.typescript]` table.
/// `[language.typescript]` 表。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Typescript {
    pub(crate) standard: String,
}

/// The one source language a unit is written in.
/// 一个单元所使用的唯一源码语言。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Language {
    /// C++.
    /// C++。
    Cpp,
    /// C.
    /// C。
    C,
    /// Python.
    /// Python。
    Python,
    /// Rust.
    /// Rust。
    Rust,
    /// Go.
    /// Go。
    Go,
    /// Ruby.
    /// Ruby。
    Ruby,
    /// C#.
    /// C#。
    Csharp,
    /// Java.
    /// Java。
    Java,
    /// PHP.
    /// PHP。
    Php,
    /// Swift.
    /// Swift。
    Swift,
    /// Kotlin.
    /// Kotlin。
    Kotlin,
    /// JavaScript (Node.js).
    /// JavaScript（Node.js）。
    Javascript,
    /// TypeScript (Node.js strip-only).
    /// TypeScript（Node.js strip-only）。
    Typescript,
    /// A language no built-in backend implements; a recorded backend answers for it, and the
    /// manifest's `external` field names which one.
    /// 没有内置后端实现的语种；由记录里的后端回答，清单的 `external` 字段点名是哪一个。
    External,
}

impl Language {
    /// The spelling this language's `[language.<name>]` table uses. The external arm has no table of
    /// its own, so it answers with the word the manifest's `external` field makes.
    /// 这门语言 `[language.<name>]` 表的拼写。外部那一支没有自己的表，因此回答清单里 `external`
    /// 字段点名的那个词。
    pub(crate) fn name(self) -> &'static str {
        match self {
            Language::Cpp => "cpp",
            Language::C => "c",
            Language::Python => "python",
            Language::Rust => "rust",
            Language::Go => "go",
            Language::Ruby => "ruby",
            Language::Csharp => "csharp",
            Language::Java => "java",
            Language::Php => "php",
            Language::Swift => "swift",
            Language::Kotlin => "kotlin",
            Language::Javascript => "javascript",
            Language::Typescript => "typescript",
            Language::External => "external",
        }
    }
}
