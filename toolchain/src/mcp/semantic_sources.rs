//! The MCP execution surface for semantic providers.
//! MCP 的语义提供者执行面。
//!
//! Language facts and AST parsing live in `xirang_multilang`; this module only reads candidate
//! files, runs the language tool, and maps the shared facts onto the query layer's `Function`.
//! 语言事实与 AST 解析住在 `xirang_multilang`；本模块只读候选文件、运行语言工具，并把共享事实映射到
//! 查询层的 `Function`。

use std::path::Path;
#[cfg(any(feature = "lang-cpp", feature = "lang-python"))]
use std::process::Command;

use xirang_multilang::semantics::SemanticFunction;

use crate::mcp::source_index::{Function, SourceFile};

fn mapped(functions: Vec<SemanticFunction>) -> Vec<Function> {
    functions
        .into_iter()
        .map(|function| Function {
            name: function.name,
            owner: function.owner,
            qualified: function.qualified,
            line: function.line,
            end_line: function.end_line,
            calls: function.calls,
            unresolved: function.unresolved,
            caveats: function.caveats,
            semantic: true,
            registration: None,
            binding: None,
        })
        .collect()
}

/// Rust facts from the multilang provider. `None` means the file did not parse; the caller falls
/// back to lexical indexing and says so.
/// 来自多语言提供者的 Rust 事实。`None` 表示文件没解析成功；调用方退回词法索引并说明。
pub(crate) fn rust(source: &str) -> Option<Vec<Function>> {
    xirang_multilang::languages::rust::semantics::semantic_functions(source).map(mapped)
}

#[cfg(feature = "lang-cpp")]
pub(crate) fn cpp(root: &Path, hint: &str) -> (Vec<SourceFile>, Vec<String>) {
    let mut files = Vec::new();
    let mut errors = Vec::new();
    visit_cpp(root, root, hint, &mut files, &mut errors);
    link_cpp_across_translation_units(&mut files);
    (files, errors)
}

/// Resolve C++ calls whose definition lives in another translation unit of this package.
/// 解析定义在本包另一个翻译单元里的 C++ 调用。
///
/// Only a unique definition is promoted. Two definitions with one qualified name stay unknown:
/// choosing one without the linker's resolution would be a guess.
/// 只有唯一定义才会被提升。同名多定义保持未知：在不知道链接器选择的情况下挑一个就是猜。
#[cfg(feature = "lang-cpp")]
fn link_cpp_across_translation_units(files: &mut [SourceFile]) {
    let mut definitions: std::collections::BTreeMap<String, usize> =
        std::collections::BTreeMap::new();
    for file in files.iter() {
        for function in &file.functions {
            *definitions.entry(function.qualified.clone()).or_insert(0) += 1;
        }
    }
    for file in files.iter_mut() {
        for function in &mut file.functions {
            let mut promoted = Vec::new();
            function.unresolved.retain(|reason| {
                let Some(target) = reason.strip_suffix(" (no definition in this translation unit)")
                else {
                    return true;
                };
                match definitions.get(target).copied() {
                    Some(1) => {
                        promoted.push(target.to_owned());
                        false
                    }
                    Some(count) => {
                        // Keep the unknown, but say how many definitions compete.
                        let _ = count;
                        true
                    }
                    None => true,
                }
            });
            for target in promoted {
                if !function.calls.contains(&target) {
                    function.calls.push(target);
                }
            }
        }
    }
}

#[cfg(not(feature = "lang-cpp"))]
pub(crate) fn cpp(_root: &Path, _hint: &str) -> (Vec<SourceFile>, Vec<String>) {
    (Vec::new(), Vec::new())
}

#[cfg(feature = "lang-cpp")]
fn visit_cpp(
    root: &Path,
    directory: &Path,
    hint: &str,
    out: &mut Vec<SourceFile>,
    errors: &mut Vec<String>,
) {
    let Ok(entries) = std::fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name == "target" || name == ".git" || name == ".codegraph" || name == ".xirang" {
            continue;
        }
        if path.is_dir() {
            visit_cpp(root, &path, hint, out, errors);
        } else if path.extension().is_some_and(|extension| {
            matches!(
                extension.to_str(),
                Some("cpp" | "cc" | "cxx" | "hpp" | "hh" | "hxx" | "h")
            )
        }) {
            let Ok(source) = std::fs::read_to_string(&path) else {
                continue;
            };
            if !source.contains(hint) {
                continue;
            }
            let Ok(relative) = path.strip_prefix(root) else {
                continue;
            };
            let functions = match parse_cpp(&path, root, &source) {
                Ok(functions) => functions,
                Err(error) => {
                    errors.push(error);
                    continue;
                }
            };
            out.push(SourceFile {
                relative: relative.to_string_lossy().replace('\\', "/"),
                source,
                functions,
                branches: xirang_kernel::source::BranchFacts::default(),
            });
        }
    }
}

#[cfg(feature = "lang-cpp")]
fn parse_cpp(path: &Path, root: &Path, source: &str) -> Result<Vec<Function>, String> {
    let output = Command::new("clang++")
        .args([
            "-std=c++20",
            "-x",
            "c++",
            &format!("-I{}", root.display()),
            &format!("-I{}", root.join("src").display()),
            &format!("-I{}", root.join("include").display()),
            "-Xclang",
            "-ast-dump=json",
            "-fsyntax-only",
        ])
        .arg(path)
        .output()
        .map_err(|error| format!("clang++ is unavailable: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "clang++ could not parse {}: {}",
            path.display(),
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let ast: serde_json::Value = serde_json::from_slice(&output.stdout)
        .map_err(|error| format!("clang emitted non-JSON: {error}"))?;
    Ok(mapped(
        xirang_multilang::languages::cpp::semantics::parse_ast_with_source(&ast, Some(source)),
    ))
}

#[cfg(feature = "lang-python")]
pub(crate) fn python(root: &Path, hint: &str) -> (Vec<SourceFile>, Vec<String>) {
    let mut files = Vec::new();
    let mut errors = Vec::new();
    visit_python(root, root, hint, &mut files, &mut errors);
    (files, errors)
}

#[cfg(not(feature = "lang-python"))]
pub(crate) fn python(_root: &Path, _hint: &str) -> (Vec<SourceFile>, Vec<String>) {
    (Vec::new(), Vec::new())
}

#[cfg(feature = "lang-python")]
fn visit_python(
    root: &Path,
    directory: &Path,
    hint: &str,
    out: &mut Vec<SourceFile>,
    errors: &mut Vec<String>,
) {
    let Ok(entries) = std::fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name == "target" || name == ".git" || name == ".codegraph" || name == ".xirang" {
            continue;
        }
        if path.is_dir() {
            visit_python(root, &path, hint, out, errors);
        } else if path.extension().is_some_and(|extension| extension == "py") {
            let Ok(source) = std::fs::read_to_string(&path) else {
                continue;
            };
            if !source.contains(hint) {
                continue;
            }
            let Ok(relative) = path.strip_prefix(root) else {
                continue;
            };
            let functions = match parse_python(&path) {
                Ok(functions) => functions,
                Err(error) => {
                    errors.push(error);
                    continue;
                }
            };
            out.push(SourceFile {
                relative: relative.to_string_lossy().replace('\\', "/"),
                source,
                functions,
                branches: xirang_kernel::source::BranchFacts::default(),
            });
        }
    }
}

#[cfg(feature = "lang-python")]
fn parse_python(path: &Path) -> Result<Vec<Function>, String> {
    let output = Command::new("python3")
        .arg("-c")
        .arg(xirang_multilang::languages::python::semantics::SCRIPT)
        .arg(path)
        .output()
        .map_err(|error| format!("python3 is unavailable: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "python3 could not parse {}: {}",
            path.display(),
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(mapped(
        xirang_multilang::languages::python::semantics::parse_json(&output.stdout)?,
    ))
}

#[cfg(test)]
#[path = "semantic_sources_tests.rs"]
mod semantic_sources_tests;
