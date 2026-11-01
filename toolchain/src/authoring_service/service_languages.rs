//! The language table the shared authoring service publishes to every surface.
//! 共享创作服务向每个执行面发布的语言表。

use xirang_kernel::authoring::{AuthoringOperation, LanguageCapabilities, RegistrationField};
use xirang_kernel::interchange::LanguageId;

use xirang_multilang::registration;

/// The languages this build can author, without binding a package root.
/// 本构建可以创作的语言，不需要绑定包根。
pub fn available_languages() -> Vec<LanguageId> {
    let mut languages = vec![rust_language()];
    for language in registration::registry().languages() {
        if language.as_str() != "rust" {
            languages.push(language.clone());
        }
    }
    languages
}

/// The capability table for one language, when this build carries it.
/// 一门语言的能力表（当本构建携带它时）。
pub fn language_capabilities(language: &LanguageId) -> Option<LanguageCapabilities> {
    if language.as_str() == "rust" {
        return Some(rust_capabilities());
    }
    registration::registry()
        .adapter(language)
        .map(|adapter| adapter.capabilities().clone())
}

pub(super) fn rust_language() -> LanguageId {
    LanguageId::parse_language("rust").expect("`rust` is a language id")
}

pub(super) fn rust_capabilities() -> LanguageCapabilities {
    LanguageCapabilities {
        language: rust_language(),
        file_extensions: &["rs"],
        operations: &[AuthoringOperation::Create, AuthoringOperation::Modify],
        fields: RegistrationField::ALL,
    }
}

/// Refuse a language this build cannot author, naming what it can.
/// 拒绝本构建无法创作的语言，并点名它能创作的。
pub(super) fn ensure_language(enabled: &[LanguageId], language: &LanguageId) -> Result<(), String> {
    if language.as_str() == "rust" {
        return Ok(());
    }
    if registration::registry().adapter(language).is_some() {
        return Ok(());
    }
    Err(format!(
        "`{}` support is not compiled into this build; enabled languages are {}",
        language.as_str(),
        enabled
            .iter()
            .map(|language| format!("`{}`", language.as_str()))
            .collect::<Vec<_>>()
            .join(", ")
    ))
}

/// Choose the language for a request, or explain the ambiguity.
/// 为请求选择语言，或解释这处歧义。
///
/// The order is the contract: an explicit choice wins, an existing file's extension is a reliable
/// hint, a project default is a decision the project made, and anything else is an ambiguity the
/// caller must settle. Deriving the language from the parent is deliberately not one of the steps.
/// 次序就是契约：显式选择优先，已有文件的扩展名是可靠线索，项目默认值是项目做过的决定，其余属于调用方
/// 必须解决的歧义。从父级推导语言有意不在这些步骤里。
pub(super) fn select_language(
    root: &std::path::Path,
    enabled: &[LanguageId],
    explicit: Option<&LanguageId>,
    destination: Option<&str>,
) -> Result<LanguageId, String> {
    if let Some(language) = explicit {
        ensure_language(enabled, language)?;
        return Ok(language.clone());
    }
    if let Some(destination) = destination {
        if destination.ends_with(".rs") {
            return Ok(rust_language());
        }
        if let Some(language) = registration::registry().language_of_path(destination) {
            return Ok(language.clone());
        }
    }
    if let Some(default) = project_default_language(root)? {
        ensure_language(enabled, &default)?;
        return Ok(default);
    }
    Err(format!(
        "this request names no language; pass one explicitly (enabled: {}), name a destination \
         file whose extension selects one, or set `[project] default_language` in xirang.toml",
        enabled
            .iter()
            .map(|language| format!("`{}`", language.as_str()))
            .collect::<Vec<_>>()
            .join(", ")
    ))
}

/// The project's default language, when `xirang.toml` declares one.
/// 项目默认语言（当 `xirang.toml` 声明了它时）。
pub(super) fn project_default_language(
    root: &std::path::Path,
) -> Result<Option<LanguageId>, String> {
    let manifest = root.join("xirang.toml");
    let Ok(text) = std::fs::read_to_string(&manifest) else {
        return Ok(None);
    };
    let mut in_project = false;
    for line in text.lines() {
        let line = line.split('#').next().unwrap_or("").trim();
        if line.starts_with('[') {
            in_project = line == "[project]";
            continue;
        }
        if !in_project {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        if key.trim() != "default_language" {
            continue;
        }
        let value = value.trim().trim_matches(['"', '\'']);
        return match LanguageId::parse_language(value) {
            Some(language) => Ok(Some(language)),
            None => Err(format!(
                "{}: `{value}` is not a language id",
                manifest.display()
            )),
        };
    }
    Ok(None)
}
