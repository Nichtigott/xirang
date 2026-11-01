//! The C language implementation: the declaration SDK and the native build half.
//! C 语言实现：声明 SDK 与原生构建那一半。
//!
//! Uses the language kit: the native driver, the record decoder and the declaration walk are shared;
//! this module owns only C's spelling — the packed record struct, the `XIRANG_SOIL` marker and the
//! compiler shape (`cc`).
//! 使用语言 kit：原生驱动、记录解码器与声明遍历都是共享的；本模块只拥有 C 的拼法——packed 记录结构体、
//! `XIRANG_SOIL` 标记与编译器形状（`cc`）。

use std::path::{Path, PathBuf};

use crate::external_faces::Recipe;
use crate::languages::kit::native::{NativeSpec, ObjectCommand};

pub mod names;

#[path = "adapter.rs"]
mod adapter;

/// The C language implementation (its authoring half lives in `adapter`).
/// C 语言实现（它的创作那一半住在 `adapter`）。
#[derive(Clone, Copy, Debug, Default)]
pub struct CLanguage;

/// One declaration found in a C source, before the build assigns it an identity.
/// C 源码里发现的一条声明，尚未由构建分配身份。
#[derive(Clone, Debug)]
pub struct NativeFace {
    /// The package-relative spelling of the source.
    /// 源码在包内的相对拼写。
    pub source: String,
    /// The parent as the declaration spells it. A C declaration names the parent by the generated
    /// header's constant; this slice supports the root, spelled `ROOT_NODE_ID`.
    /// 父级按声明的拼法。C 声明经生成头的常量点名父级；本切片支持根，拼作 `ROOT_NODE_ID`。
    pub parent: String,
    /// The face name from `.kind`.
    /// `.kind` 里的面名。
    pub name: String,
    /// Whether the declaration says it owns a registry.
    /// 声明是否说自己自持注册机。
    pub owns_registry: bool,
}

/// What the C half tells the shared native driver.
/// C 那一半告诉共享原生驱动的东西。
pub const SPEC: NativeSpec = NativeSpec {
    extensions: &[".c"],
    default_recipe,
    include_dirs,
    skip_directories: &["target", ".git", "crates", "node_modules", ".cargo"],
    object_command,
};

fn default_recipe(_sources: &[String]) -> (String, Vec<String>) {
    ("cc".to_owned(), vec!["-O0".to_owned(), "-fPIC".to_owned()])
}

fn include_dirs() -> Vec<PathBuf> {
    vec![crate::sdk::c_include_dir()]
}

fn object_command(compile: &ObjectCommand<'_>) -> (String, Vec<String>) {
    let mut arguments = compile.recipe.arguments.clone();
    arguments.push("-c".to_owned());
    arguments.push(compile.source.display().to_string());
    arguments.push("-o".to_owned());
    arguments.push(compile.object.display().to_string());
    arguments.push(format!("-I{}", compile.out_dir.display()));
    for include in compile.include_dirs {
        arguments.push(format!("-I{}", include.display()));
    }
    for include in &compile.recipe.includes {
        arguments.push(format!(
            "-I{}",
            compile.package_root.join(include).display()
        ));
    }
    (compile.recipe.program.clone(), arguments)
}

/// Does this source declare a face?
/// 这份源码有没有声明一个面？
fn mentions(text: &str) -> bool {
    text.contains("XIRANG_SOIL")
}

/// Read the package's native recipe and declarations.
/// 读出包的原生配方与声明。
pub fn read_recipe(package_root: &Path) -> Result<Option<(Recipe, Vec<NativeFace>)>, String> {
    let Some((recipe, sources)) =
        crate::languages::kit::native::read_recipe(package_root, &SPEC, mentions)?
    else {
        return Ok(None);
    };
    let mut faces = Vec::new();
    for source in sources {
        let path = package_root.join(&source);
        let text = std::fs::read_to_string(&path)
            .map_err(|error| format!("read {}: {error}", path.display()))?;
        for face in scan(&source, &text)? {
            faces.push(face);
        }
    }
    Ok(Some((recipe, faces)))
}

/// Every declaration one source carries.
/// 一份源码携带的每条声明。
pub fn scan(source: &str, text: &str) -> Result<Vec<NativeFace>, String> {
    let mut faces = Vec::new();
    let mut at = 0usize;
    while let Some(found) = text[at..].find("XIRANG_SOIL") {
        let start = at + found + "XIRANG_SOIL".len();
        let Some(open) = text[start..].find('{').map(|offset| start + offset) else {
            break;
        };
        let close = matching_brace(text, open)?;
        let block = &text[open + 1..close];
        let mut kind = None;
        let mut parent = None;
        let mut flags = 0u32;
        for line in block.lines() {
            let line = line.split("//").next().unwrap_or("").trim();
            if line.is_empty() || !line.starts_with('.') {
                continue;
            }
            let Some((name, value)) = line[1..].split_once('=') else {
                continue;
            };
            let name = name.trim();
            let value = value.trim().trim_end_matches(',').trim();
            match name {
                "kind" => kind = Some(unquote(value)?),
                "parent" => {
                    parent = Some(parse_parent(value)?);
                }
                "flags" => {
                    flags = value.parse::<u32>().map_err(|_| {
                        format!("`{value}` is not a numeric `.flags` in a C declaration")
                    })?
                }
                _ => {}
            }
        }
        if let Some(name) = kind {
            faces.push(NativeFace {
                source: source.to_owned(),
                parent: parent.unwrap_or_else(|| "ROOT_NODE_ID".to_owned()),
                name,
                owns_registry: flags & 1 != 0,
            });
        }
        at = close + 1;
    }
    Ok(faces)
}

fn matching_brace(text: &str, open: usize) -> Result<usize, String> {
    let mut depth = 0usize;
    for (index, byte) in text.bytes().enumerate().skip(open) {
        match byte {
            b'{' => depth += 1,
            b'}' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return Ok(index);
                }
            }
            _ => {}
        }
    }
    Err("a C declaration's initializer is not closed".to_owned())
}

/// The parent constant a C declaration names: `XIRANG_ROOT_NODE_ID` is the root, and any other
/// `XIRANG_FACES_<module>` names that module. The build resolves the spelling to an identity before
/// compiling; a byte literal is refused because the build cannot resolve it back to a face.
/// C 声明点名的父常量：`XIRANG_ROOT_NODE_ID` 是根，其余 `XIRANG_FACES_<module>` 点名那个模块。构建在编译前
/// 把拼法解析成身份；字节字面量按名拒绝，因为构建无法把它解析回一个面。
fn parse_parent(value: &str) -> Result<String, String> {
    if value == "XIRANG_ROOT_NODE_ID" {
        return Ok("ROOT_NODE_ID".to_owned());
    }
    if let Some(module) = value.strip_prefix("XIRANG_FACES_")
        && !module.is_empty()
        && module
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '_')
    {
        return Ok(module.to_owned());
    }
    Err(format!(
        "`{value}` is not a generated parent constant; use `XIRANG_ROOT_NODE_ID` or \
         `XIRANG_FACES_<module>`"
    ))
}

fn unquote(value: &str) -> Result<String, String> {
    let inner = value
        .strip_prefix('"')
        .and_then(|rest| rest.strip_suffix('"'))
        .ok_or_else(|| format!("`{value}` is not a string literal in a C declaration"))?;
    Ok(inner.replace("\\\"", "\"").replace("\\\\", "\\"))
}

/// Compile this package's C sources through the shared native driver.
/// 经共享原生驱动编译本包的 C 源码。
pub fn compile(
    package_root: &Path,
    out_dir: &Path,
    recipe: &Recipe,
    sources: &[&str],
    emit_cargo_directives: bool,
) -> Result<Vec<PathBuf>, String> {
    crate::languages::kit::native::compile(
        package_root,
        out_dir,
        recipe,
        sources,
        &include_dirs(),
        &SPEC,
        emit_cargo_directives,
    )
}
