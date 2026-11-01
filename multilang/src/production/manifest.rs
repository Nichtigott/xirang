//! The `xirang.toml` reader: one main configuration for the first multilingual slice.
//! `xirang.toml` 读取器：第一块多语言竖切的一份主配置。
//!
//! The reader is deliberately smaller than TOML and *refuses* what it does not understand,
//! naming the line: a manifest that quietly ignored `[language.go]` would report a build of a
//! project it never configured. The supported subset is `schema`, `[package]`,
//! `[language.cpp]`, `[dependencies]` with `path` inline tables and `[build]`` profile`.
//! 本读取器有意小于 TOML，并且**拒绝**它读不懂的东西、点名行号：静默忽略 `[language.go]` 的清单会
//! 报告一个它从未配置过的项目的构建结果。支持的子集是 `schema`、`[package]`、`[language.cpp]`、
//! 带 `path` 内联表的 `[dependencies]`，以及 `[build]` 的 `profile`。
//!
//! A `[language.<x>]` table declares the unit's language just as much as its `standard` line does:
//! a second table is refused even when it carries no keys, and a table that never names a
//! `standard` selects nothing instead of quietly falling back to C++ — that fallback was how a unit
//! used to compile one language while its manifest named two.
//! `[language.<x>]` 表本身就在声明单元的语言，与它的 `standard` 行同理：第二个表会被拒绝——即使它
//! 一个键都没写；而一个从不写 `standard` 的表什么也不选中，不会静默退回 C++——那个退回正是过去
//! "清单点名两门语言、却只编其中一门"的来源。

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[path = "values.rs"]
mod values;

pub(crate) use self::values::string;
use self::values::{array, inline_dependency, inline_path, integer, option_value, strip_comment};

/// The language a `[language.<name>]` table names, or `None` when no built-in backend answers for
/// that spelling (the caller then asks the recorded backends).
/// `[language.<name>]` 表点名的语言；没有内置后端回答这个拼写时返回 `None`（调用方接着去问记录里的
/// 后端）。
fn builtin_language(section: &str) -> Option<Language> {
    Some(match section {
        "language.cpp" => Language::Cpp,
        "language.c" => Language::C,
        "language.python" => Language::Python,
        "language.rust" => Language::Rust,
        "language.go" => Language::Go,
        "language.ruby" => Language::Ruby,
        "language.csharp" => Language::Csharp,
        "language.java" => Language::Java,
        "language.php" => Language::Php,
        "language.swift" => Language::Swift,
        "language.kotlin" => Language::Kotlin,
        "language.javascript" => Language::Javascript,
        "language.typescript" => Language::Typescript,
        _ => return None,
    })
}

use xirang_kernel::interchange::{Export, ParameterType, ReturnType};

pub(crate) use super::language::{
    C, Cpp, Csharp, Go, Java, Javascript, Kotlin, Language, Php, Python, Ruby, Rust, Swift,
    Typescript,
};

/// The version of the manifest schema this reader accepts.
/// 本读取器接受的清单 schema 版本。
pub(crate) const SCHEMA: u32 = 1;

/// One parsed `xirang.toml`.
/// 一份解析后的 `xirang.toml`。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Manifest {
    pub(crate) package: Package,
    pub(crate) language: Language,
    pub(crate) cpp: Cpp,
    pub(crate) c: C,
    pub(crate) python: Python,
    pub(crate) rust: Rust,
    /// The `[language.go]` identity.
    /// `[language.go]` 身份。
    pub(crate) go: Go,
    /// The `[language.ruby]` identity.
    /// `[language.ruby]` 身份。
    pub(crate) ruby: Ruby,
    /// The `[language.csharp]` identity.
    /// `[language.csharp]` 身份。
    pub(crate) csharp: Csharp,
    /// The `[language.java]` identity.
    /// `[language.java]` 身份。
    pub(crate) java: Java,
    /// The `[language.php]` identity.
    /// `[language.php]` 身份。
    pub(crate) php: Php,
    /// The `[language.swift]` identity.
    /// `[language.swift]` 身份。
    pub(crate) swift: Swift,
    /// The `[language.kotlin]` identity.
    /// `[language.kotlin]` 身份。
    pub(crate) kotlin: Kotlin,
    /// The `[language.javascript]` identity.
    /// `[language.javascript]` 身份。
    pub(crate) javascript: Javascript,
    /// The `[language.typescript]` identity.
    /// `[language.typescript]` 身份。
    pub(crate) typescript: Typescript,
    /// The exports this unit promises to another language, in declaration order.
    /// 本单元承诺给另一种语言的导出，按声明顺序。
    pub(crate) exports: Vec<Export>,
    pub(crate) dependencies: BTreeMap<String, Dependency>,
    /// The `[sources]` table: the directories a `source` dependency resolves against.
    /// `[sources]` 表：`source` 依赖解析所对的目录。
    pub(crate) sources: BTreeMap<String, SourceSpec>,
    /// The `[overrides]` table: `<source>/<package>` keys redirected to one local directory.
    /// `[overrides]` 表：把 `<source>/<package>` 键改指到一个本地目录。
    pub(crate) overrides: BTreeMap<String, PathBuf>,
    pub(crate) profile: String,
    /// The id of the recorded backend that answers for an external language, when this unit names
    /// one (`[language.<id>]` with no built-in backend).
    /// 当本单元点名一个外部语言（`[language.<id>]` 且没有内置后端）时，回答它的记录型后端的 id。
    pub(crate) external: Option<String>,
    /// The keys of an external language's table, as the backend receives them.
    /// 外部语言表的键，按后端收到的形式保存。
    pub(crate) options: BTreeMap<String, String>,
}

/// The `[package]` table: what the project is called and what it produces.
/// `[package]` 表：项目叫什么、产出什么。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Package {
    pub(crate) name: String,
    pub(crate) version: String,
    pub(crate) kind: Kind,
}

/// The two package kinds this slice produces.
/// 本切片产生的两种包类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Kind {
    Library,
    Binary,
}

/// Where a named `[sources]` entry lives: a directory, or a git repository at one exact revision.
/// A git source must name `rev`; a delivery that silently follows a branch tip is not reproducible.
/// 具名 `[sources]` 条目住在哪里：一个目录，或一个 git 仓库的某个确切版本。git 源必须点名 `rev`；
/// 静默跟随分支顶端的交付不可复现。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SourceSpec {
    /// A directory, relative to the root manifest's directory or absolute.
    /// 一个目录，相对于根清单所在目录，或绝对的。
    Path(PathBuf),
    /// A git repository at `url`, checked out at `rev`.
    /// `url` 处的 git 仓库，检出到 `rev`。
    Git {
        /// The URL git clones (a local `file://` URL is the offline case).
        /// 让 git 克隆的 URL（本地 `file://` URL 是离线情形）。
        url: String,
        /// The revision the delivery expects.
        /// 交付期望的版本。
        rev: String,
    },
    /// A local package registry: `<directory>/<package>/<version>/xirang.toml`. The highest version
    /// that has a manifest is chosen and recorded in the lock, so a new version changes the lock.
    /// 本地包注册表：`<directory>/<package>/<version>/xirang.toml`。选择带清单的最高版本并记进锁，
    /// 因此新增版本会改变锁。
    Registry {
        /// The registry directory, relative to the root manifest's directory or absolute.
        /// 注册表目录，相对于根清单所在目录，或绝对的。
        directory: PathBuf,
    },
}

/// One `[dependencies]` entry.
/// 一个 `[dependencies]` 条目。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Dependency {
    /// A local path, resolved relative to the manifest that names it.
    /// 本地路径，相对于点名它的清单解析。
    Path(PathBuf),
    /// A package looked up in a named source declared by the root manifest's `[sources]` table.
    /// 在根清单 `[sources]` 表声明的具名源里查找的包。
    Source {
        /// The source name, as `[sources]` spells it.
        /// 源名，与 `[sources]` 的拼写一致。
        source: String,
        /// The package name inside that source.
        /// 该源里的包名。
        package: String,
    },
}

impl Manifest {
    /// Read `<directory>/xirang.toml`.
    /// 读取 `<directory>/xirang.toml`。
    pub(crate) fn load(directory: &Path) -> Result<Self, String> {
        let path = directory.join("xirang.toml");
        let text = std::fs::read_to_string(&path)
            .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
        Self::parse_at(&text, Some(directory))
            .map_err(|error| format!("{}: {error}", path.display()))
    }

    /// Parse the manifest text. Every refusal names the line it came from.
    /// 解析清单文本；每条拒绝都点名它来自哪一行。
    pub(crate) fn parse(text: &str) -> Result<Self, String> {
        Self::parse_at(text, None)
    }

    /// The same reader, with the project directory available when an external language table has to
    /// find the record that answers for it.
    /// 同一个读取器；当外部语言表需要找到回答它的记录时，项目目录是可用的。
    fn parse_at(text: &str, root: Option<&Path>) -> Result<Self, String> {
        let mut schema: Option<u32> = None;
        let mut name = None;
        let mut version = None;
        let mut kind = None;
        let mut standard = None;
        let mut c_standard = None;
        let mut python_standard = None;
        let mut rust_standard = None;
        let mut go_standard = None;
        let mut ruby_standard = None;
        let mut csharp_standard = None;
        let mut java_standard = None;
        let mut php_standard = None;
        let mut swift_standard = None;
        let mut kotlin_standard = None;
        let mut javascript_standard = None;
        let mut typescript_standard = None;
        let mut language: Option<Language> = None;
        let mut exports: Vec<Export> = Vec::new();
        let mut pending: Option<Pending> = None;
        let mut pending_line = 0_usize;
        let mut dependencies = BTreeMap::new();
        let mut sources = BTreeMap::new();
        let mut overrides = BTreeMap::new();
        let mut external: Option<String> = None;
        // The `[language.<x>]` table a manifest wrote, and the line it wrote it on. A table declares
        // the unit's language just as much as its `standard = "…"` line does, so a second one is a
        // second declaration even when it carries no keys at all.
        // 清单写下的 `[language.<x>]` 表与它所在的行。声明单元语言的不只是 `standard = "…"` 那一行，
        // 表本身也在声明，所以第二个表就是第二次声明——即使它一个键也没写。
        let mut declared: Option<(usize, Language)> = None;
        let mut options = BTreeMap::new();
        let mut profile = "dev".to_owned();
        let mut table = String::new();
        for (index, raw) in text.lines().enumerate() {
            let number = index + 1;
            let line = strip_comment(raw);
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            if let Some(inner) = line
                .strip_prefix("[[")
                .and_then(|rest| rest.strip_suffix("]]"))
            {
                if inner.trim() != "export" {
                    return Err(format!(
                        "line {number}: unknown table `[[{}]]`; the only repeated table this \
                         reader accepts is `[[export]]`",
                        inner.trim()
                    ));
                }
                flush(&mut pending, &mut exports, pending_line)?;
                pending = Some(Pending::default());
                pending_line = number;
                table = "export".to_owned();
                continue;
            }
            if let Some(inner) = line
                .strip_prefix('[')
                .and_then(|rest| rest.strip_suffix(']'))
            {
                flush(&mut pending, &mut exports, pending_line)?;
                table = inner.trim().to_owned();
                match table.as_str() {
                    "package" | "dependencies" | "sources" | "overrides" | "build" => {}
                    other if builtin_language(other).is_some() => {
                        let named = builtin_language(other).expect("the guard just matched");
                        match declared {
                            Some((line, first)) if first != named => {
                                return Err(format!(
                                    "line {number}: this unit already declares `[language.{}]` at \
                                     line {line}; one unit has one source language",
                                    first.name()
                                ));
                            }
                            Some(_) => {}
                            None => declared = Some((number, named)),
                        }
                    }
                    other if other.starts_with("language.") => {
                        let id = &other["language.".len()..];
                        let Some(root) = root else {
                            return Err(format!(
                                "line {number}: `[{other}]` names an external language, and only a \
                                 project loaded from a directory can find the record that answers \
                                 for it"
                            ));
                        };
                        super::backends::external(root, id)
                            .map_err(|error| format!("line {number}: {error}"))?;
                        if language.is_some() && external.as_deref() != Some(id) {
                            return Err(format!(
                                "line {number}: this unit already declares a source language; one \
                                 unit has one source language"
                            ));
                        }
                        language = Some(Language::External);
                        external = Some(id.to_owned());
                    }
                    other => {
                        return Err(format!(
                            "line {number}: unknown table `[{other}]`; this reader accepts \
                             package, language.cpp, language.rust, dependencies, sources and                              build"
                        ));
                    }
                }
                continue;
            }
            let (key, value) = line
                .split_once('=')
                .ok_or_else(|| format!("line {number}: expected `key = value`, found `{line}`"))?;
            let key = key.trim();
            let value = value.trim();
            match (table.as_str(), key) {
                ("", "schema") => {
                    let parsed = integer(value, number)?;
                    if parsed != SCHEMA {
                        return Err(format!(
                            "line {number}: schema {parsed} is not this reader's {SCHEMA}"
                        ));
                    }
                    schema = Some(parsed);
                }
                ("package", "name") => name = Some(string(value, number)?),
                ("package", "version") => version = Some(string(value, number)?),
                ("package", "kind") => {
                    kind = Some(match string(value, number)?.as_str() {
                        "library" => Kind::Library,
                        "binary" => Kind::Binary,
                        other => {
                            return Err(format!(
                                "line {number}: package kind `{other}` is not library or binary"
                            ));
                        }
                    });
                }
                ("language.cpp", "standard") => {
                    if language.is_some() {
                        return Err(format!(
                            "line {number}: this unit already declares a source language; one unit \
                             has one source language"
                        ));
                    }
                    language = Some(Language::Cpp);
                    standard = Some(string(value, number)?);
                }
                ("language.c", "standard") => {
                    if language.is_some() {
                        return Err(format!(
                            "line {number}: this unit already declares a source language; one \
                             unit has one source language"
                        ));
                    }
                    language = Some(Language::C);
                    c_standard = Some(string(value, number)?);
                }
                ("language.python", "standard") => {
                    if language.is_some() {
                        return Err(format!(
                            "line {number}: this unit already declares a source language; one \
                             unit has one source language"
                        ));
                    }
                    language = Some(Language::Python);
                    python_standard = Some(string(value, number)?);
                }
                ("language.rust", "standard") => {
                    if language.is_some() {
                        return Err(format!(
                            "line {number}: this unit already declares a source language; one \
                             unit has one source language"
                        ));
                    }
                    language = Some(Language::Rust);
                    rust_standard = Some(string(value, number)?);
                }
                ("language.go", "standard") => {
                    if language.is_some() {
                        return Err(format!(
                            "line {number}: this unit already declares a source language; one \
                             unit has one source language"
                        ));
                    }
                    language = Some(Language::Go);
                    go_standard = Some(string(value, number)?);
                }
                ("language.ruby", "standard") => {
                    if language.is_some() {
                        return Err(format!(
                            "line {number}: this unit already declares a source language; one \
                             unit has one source language"
                        ));
                    }
                    language = Some(Language::Ruby);
                    ruby_standard = Some(string(value, number)?);
                }
                ("language.csharp", "standard") => {
                    if language.is_some() {
                        return Err(format!(
                            "line {number}: this unit already declares a source language; one \
                             unit has one source language"
                        ));
                    }
                    language = Some(Language::Csharp);
                    csharp_standard = Some(string(value, number)?);
                }
                ("language.java", "standard") => {
                    if language.is_some() {
                        return Err(format!(
                            "line {number}: this unit already declares a source language; one \
                             unit has one source language"
                        ));
                    }
                    language = Some(Language::Java);
                    java_standard = Some(string(value, number)?);
                }
                ("language.php", "standard") => {
                    language = Some(Language::Php);
                    php_standard = Some(string(value, number)?);
                }
                ("language.swift", "standard") => {
                    language = Some(Language::Swift);
                    swift_standard = Some(string(value, number)?);
                }
                ("language.kotlin", "standard") => {
                    if language.is_some() {
                        return Err(format!(
                            "line {number}: this unit already declares a source language; one \
                             unit has one source language"
                        ));
                    }
                    language = Some(Language::Kotlin);
                    kotlin_standard = Some(string(value, number)?);
                }
                ("language.javascript", "standard") => {
                    if language.is_some() {
                        return Err(format!(
                            "line {number}: this unit already declares a source language; one \
                             unit has one source language"
                        ));
                    }
                    let chosen = string(value, number)?;
                    if chosen != "commonjs" {
                        return Err(format!(
                            "line {number}: `[language.javascript] standard` is `{chosen}`; this \
                             slice runs CommonJS modules, so the only supported standard is \
                             `commonjs`"
                        ));
                    }
                    language = Some(Language::Javascript);
                    javascript_standard = Some(chosen);
                }
                ("language.typescript", "standard") => {
                    if language.is_some() {
                        return Err(format!(
                            "line {number}: this unit already declares a source language; one \
                             unit has one source language"
                        ));
                    }
                    let chosen = string(value, number)?;
                    if chosen != "strip" {
                        return Err(format!(
                            "line {number}: `[language.typescript] standard` is `{chosen}`; this \
                             slice runs Node's strip-only TypeScript, so the only supported \
                             standard is `strip`"
                        ));
                    }
                    language = Some(Language::Typescript);
                    typescript_standard = Some(chosen);
                }
                ("export", "name") => {
                    let pending = pending.as_mut().ok_or_else(|| {
                        format!("line {number}: `name` outside an `[[export]]` table")
                    })?;
                    pending.name = Some(string(value, number)?);
                }
                ("export", "params") => {
                    let pending = pending.as_mut().ok_or_else(|| {
                        format!("line {number}: `params` outside an `[[export]]` table")
                    })?;
                    pending.parameters = array(value, number)?
                        .into_iter()
                        .map(|text| {
                            ParameterType::parse_parameter(&text)
                                .map_err(|error| format!("line {number}: {error}"))
                        })
                        .collect::<Result<Vec<_>, _>>()?;
                }
                ("export", "errors") => {
                    let pending = pending.as_mut().ok_or_else(|| {
                        format!("line {number}: `errors` outside an `[[export]]` table")
                    })?;
                    pending.errors = array(value, number)?;
                }
                ("export", "returns") => {
                    let pending = pending.as_mut().ok_or_else(|| {
                        format!("line {number}: `returns` outside an `[[export]]` table")
                    })?;
                    let text = string(value, number)?;
                    pending.returns = Some(
                        ReturnType::parse_return(&text)
                            .map_err(|error| format!("line {number}: {error}"))?,
                    );
                }
                ("build", "profile") => profile = string(value, number)?,
                ("dependencies", name) => {
                    dependencies.insert(name.to_owned(), inline_dependency(value, number)?);
                }
                ("sources", source) => {
                    let spec = crate::production::sources::inline_source(source, value, number)?;
                    sources.insert(source.to_owned(), spec);
                }
                ("overrides", key) => {
                    let path = inline_path(value, number)?;
                    let mut parts = key.split('/');
                    let named = (parts.next(), parts.next(), parts.next());
                    let (Some(source), Some(package), None) = named else {
                        return Err(format!(
                            "line {number}: an override is keyed `<source>/<package>`, and `{key}` is not"
                        ));
                    };
                    if source.is_empty() || package.is_empty() {
                        return Err(format!(
                            "line {number}: an override names a source and a package, and `{key}` leaves one empty"
                        ));
                    }
                    overrides.insert(key.to_owned(), path);
                }
                (table, key) if table.starts_with("language.") => {
                    let id = &table["language.".len()..];
                    if super::backends::registered_languages().contains(&id) {
                        return Err(format!(
                            "line {number}: unknown key `{key}` in table `[{table}]`"
                        ));
                    }
                    if external.as_deref() != Some(id) {
                        return Err(format!(
                            "line {number}: this unit already declares a source language; one \
                             unit has one source language"
                        ));
                    }
                    language = Some(Language::External);
                    options.insert(key.to_owned(), option_value(value, number)?);
                }
                (_, other) => {
                    return Err(format!(
                        "line {number}: unknown key `{other}` in table `[{table}]`"
                    ));
                }
            }
        }
        flush(&mut pending, &mut exports, pending_line)?;
        schema.ok_or("the manifest has no `schema = 1` line")?;
        let package = Package {
            name: name.ok_or("the manifest has no `[package] name`")?,
            version: version.ok_or("the manifest has no `[package] version`")?,
            kind: kind.ok_or("the manifest has no `[package] kind`")?,
        };
        let language = match (language, declared) {
            (Some(selected), Some((line, declared))) if selected != declared => {
                return Err(format!(
                    "line {line}: this unit declares `[language.{}]`, but `{}` is the language it \
                     compiles with; one unit has one source language",
                    declared.name(),
                    selected.name()
                ));
            }
            // A table with no `standard` selects nothing, and defaulting to C++ on top of a written
            // `[language.rust]` is how a unit used to compile nothing it named and succeed anyway.
            // 没有 `standard` 的表什么都没选中；在写下 `[language.rust]` 之上再默认成 C++，正是过去
            // 那种"点名的语言一个字都没编、却照样成功"的来源。
            (None, Some((line, declared))) => {
                return Err(format!(
                    "line {line}: `[language.{}]` names no `standard`; a language table has to name \
                     the standard it compiles with",
                    declared.name()
                ));
            }
            (Some(selected), _) => selected,
            (None, None) => Language::Cpp,
        };
        if language == Language::Rust && package.kind != Kind::Library {
            return Err(
                "a Rust unit is a consumer library in this slice; a Rust binary is not implemented"
                    .to_owned(),
            );
        }
        if language == Language::Ruby && package.kind != Kind::Library {
            return Err(
                "a Ruby unit is a consumer library in this slice; a Ruby binary is not implemented"
                    .to_owned(),
            );
        }
        if language == Language::Csharp && package.kind != Kind::Library {
            return Err(
                "a C# unit is a consumer library in this slice; a C# binary is not implemented"
                    .to_owned(),
            );
        }
        if language == Language::Java && package.kind != Kind::Library {
            return Err(
                "a Java unit is a consumer library in this slice; a Java binary is not implemented"
                    .to_owned(),
            );
        }

        if language == Language::Php && package.kind != Kind::Library {
            return Err(
                "a PHP unit is a consumer library in this slice; a PHP binary is not implemented"
                    .to_owned(),
            );
        }

        if language == Language::Swift && package.kind != Kind::Library {
            return Err(
                "a Swift unit is a consumer library in this slice; a Swift binary is not                  implemented"
                    .to_owned(),
            );
        }
        if language == Language::Kotlin && package.kind != Kind::Library {
            return Err(
                "a Kotlin unit is a consumer library in this slice; a Kotlin binary is \
                 not implemented"
                    .to_owned(),
            );
        }
        if language == Language::Javascript && package.kind != Kind::Library {
            return Err(
                "a JavaScript unit is a consumer library in this slice; a JavaScript binary is \
                 not implemented"
                    .to_owned(),
            );
        }
        if language == Language::Typescript && package.kind != Kind::Library {
            return Err(
                "a TypeScript unit is a consumer library in this slice; a TypeScript binary is \
                 not implemented"
                    .to_owned(),
            );
        }
        if language == Language::Go && package.kind != Kind::Library {
            return Err(
                "a Go unit is a consumer library in this slice; a Go binary is not implemented"
                    .to_owned(),
            );
        }
        if language == Language::External {
            if package.kind != Kind::Library {
                return Err(
                    "an external-language unit is a library in this slice; a binary is not \
                     implemented"
                        .to_owned(),
                );
            }
            if !exports.is_empty() {
                return Err(
                    "an `[[export]]` table names a built-in producer; an external-language unit \
                     is consumed through its backend"
                        .to_owned(),
                );
            }
        }
        if !exports.is_empty() {
            if language != Language::Cpp && language != Language::Rust {
                return Err(
                    "an `[[export]]` table needs a C++ or Rust library: the wrapper that \
                            carries the C ABI is generated in that language in this slice"
                        .to_owned(),
                );
            }
            if package.kind != Kind::Library {
                return Err(format!(
                    "`{}` declares exports but is a binary; only a library is called from another \
                     language",
                    package.name
                ));
            }
        }
        Ok(Self {
            package,
            language,
            cpp: Cpp {
                standard: standard.unwrap_or_else(|| "20".to_owned()),
            },
            c: C {
                standard: c_standard.unwrap_or_else(|| "99".to_owned()),
            },
            python: Python {
                standard: python_standard.unwrap_or_else(|| "3".to_owned()),
            },
            rust: Rust {
                standard: rust_standard.unwrap_or_else(|| "2021".to_owned()),
            },
            go: Go {
                standard: go_standard.unwrap_or_else(|| "1.21".to_owned()),
            },
            ruby: Ruby {
                standard: ruby_standard.unwrap_or_else(|| "3".to_owned()),
            },
            csharp: Csharp {
                standard: csharp_standard.unwrap_or_else(|| "net10.0".to_owned()),
            },
            java: Java {
                standard: java_standard.unwrap_or_else(|| "21".to_owned()),
            },
            php: Php {
                standard: php_standard.unwrap_or_else(|| "8.4".to_owned()),
            },
            swift: Swift {
                standard: swift_standard.unwrap_or_else(|| "6.2".to_owned()),
            },
            kotlin: Kotlin {
                standard: kotlin_standard.unwrap_or_else(|| "2.4".to_owned()),
            },
            javascript: Javascript {
                standard: javascript_standard.unwrap_or_else(|| "commonjs".to_owned()),
            },
            typescript: Typescript {
                standard: typescript_standard.unwrap_or_else(|| "strip".to_owned()),
            },
            exports,
            dependencies,
            sources,
            overrides,
            profile,
            external,
            options,
        })
    }
}

/// Drop a `#` comment that starts outside a quoted string.
/// 去掉引号之外开始的 `#` 注释。
/// One `[[export]]` table while it is being read.
/// 正在读取的一张 `[[export]]` 表。
#[derive(Default)]
struct Pending {
    name: Option<String>,
    parameters: Vec<ParameterType>,
    returns: Option<ReturnType>,
    errors: Vec<String>,
}

/// The parameter names the generated prototype and wrapper use, by position.
/// 生成的声明与包装器按位置使用的参数名。
const PARAM_NAMES: [&str; 16] = [
    "p0", "p1", "p2", "p3", "p4", "p5", "p6", "p7", "p8", "p9", "p10", "p11", "p12", "p13", "p14",
    "p15",
];

/// Close one pending export table and validate it with the kernel's own export rules.
/// 关闭一张待完成的导出表，并用内核自己的导出规则校验它。
fn flush(
    pending: &mut Option<Pending>,
    exports: &mut Vec<Export>,
    line: usize,
) -> Result<(), String> {
    let Some(entry) = pending.take() else {
        return Ok(());
    };
    let name = entry
        .name
        .ok_or_else(|| format!("line {line}: this `[[export]]` table has no `name`"))?;
    if entry.parameters.len() > PARAM_NAMES.len() {
        return Err(format!(
            "line {line}: `{name}` declares more parameters than the C ABI slice carries"
        ));
    }
    let parameters = entry
        .parameters
        .iter()
        .enumerate()
        .map(|(index, kind)| (PARAM_NAMES[index], *kind))
        .collect();
    let export = Export {
        name,
        parameters,
        returns: entry.returns,
        errors: entry.errors,
    };
    export
        .validate_export()
        .map_err(|error| format!("line {line}: {error}"))?;
    exports.push(export);
    Ok(())
}
