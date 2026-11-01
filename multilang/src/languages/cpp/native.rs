//! The C++ native half: the declaration reader and the recipe this language hands the shared
//! native driver.
//! C++ 原生那一半：声明读取器，以及这门语言交给共享原生驱动的配方。
//!
//! Recipe reading, discovery and process execution live in [`crate::languages::kit::native`]; this
//! module owns what is C++ about them — the extensions, the default compiler, the SDK include
//! directory, and the reading of `XIRANG_SOIL`/`XIRANG_FACE` declarations into [`NativeFace`]s.
//! 配方读取、发现与进程执行住在 [`crate::languages::kit::native`]；本模块拥有其中属于 C++ 的部分——
//! 扩展名、默认编译器、SDK 包含目录，以及把 `XIRANG_SOIL`/`XIRANG_FACE` 声明读成 [`NativeFace`]。

use std::path::{Path, PathBuf};

use crate::external_faces::Recipe;
use crate::languages::kit::native::NativeSpec;

/// One declaration found in a native source, before the build assigns it an identity.
/// 本地源码里发现的一条声明，尚未由构建分配身份。
#[derive(Clone, Debug)]
pub struct NativeFace {
    /// The package-relative spelling of the source.
    /// 源码在包内的相对拼写。
    pub source: String,
    /// The parent as the declaration spells it, without the generated-header prefix.
    /// 父面按声明的拼法，去掉生成头前缀。
    pub parent: String,
    /// The face name from `.named("…")`.
    /// `.named("…")` 里的面名。
    pub name: String,
    /// Whether the declaration says it owns a registry.
    /// 声明是否说自己自持注册机。
    pub owns_registry: bool,
}

/// What the C++ half tells the shared native driver.
/// C++ 那一半告诉共享原生驱动的东西。
pub const SPEC: NativeSpec = NativeSpec {
    extensions: &[".cpp", ".cc", ".cxx", ".c++", ".c"],
    default_recipe,
    include_dirs,
    skip_directories: &["target", ".git", "crates", "node_modules", ".cargo"],
    object_command,
};

/// How a cc-like compiler is invoked for one source: the shape this language has always used.
/// 一份源码如何被 cc 风格编译器调用：这门语言一直以来的形状。
fn object_command(
    compile: &crate::languages::kit::native::ObjectCommand<'_>,
) -> (String, Vec<String>) {
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

/// Compiler and arguments when the manifest writes no `[native]` recipe.
/// 清单没有写 `[native]` 配方时的编译器与参数。
///
/// The recipe follows the sources: a `.c` file is compiled by the C compiler, everything else this
/// build knows is C++.
/// 配方跟着源码走：`.c` 由 C 编译器编，本构建认识的其它扩展名都是 C++。
fn default_recipe(sources: &[String]) -> (String, Vec<String>) {
    let c = sources.iter().all(|source| source.ends_with(".c"));
    if c {
        ("cc".to_owned(), vec!["-O0".to_owned(), "-fPIC".to_owned()])
    } else {
        (
            "g++".to_owned(),
            vec![
                "-std=c++20".to_owned(),
                "-O0".to_owned(),
                "-fPIC".to_owned(),
            ],
        )
    }
}

/// The C++ SDK include directory every compilation adds.
/// 每次编译都会添加的 C++ SDK 包含目录。
fn include_dirs() -> Vec<PathBuf> {
    vec![crate::sdk::cpp_include_dir()]
}

/// The module a declaration's source lives in; one implementation lives in [`crate::languages::cpp::names`].
/// 声明所在源码的模块；唯一实现住在 [`crate::languages::cpp::names`]。
pub fn module_of(source: &str) -> String {
    crate::languages::cpp::names::module_of(source)
}

/// Compatibility alias for the published `cpp_native::read` spelling.
/// 已发布拼法 `cpp_native::read` 的兼容别名。
#[deprecated(note = "use cpp_native::read_recipe; this alias keeps 0.2 callers compiling")]
pub fn read(package_root: &Path) -> Result<Option<(Recipe, Vec<NativeFace>)>, String> {
    read_recipe(package_root)
}

/// Reads a package native recipe and its declared faces.
/// 读取一个包的原生 recipe 与它声明的面。
pub fn read_recipe(package_root: &Path) -> Result<Option<(Recipe, Vec<NativeFace>)>, String> {
    let manifest = package_root.join("xirang.toml");
    let Some((recipe, sources)) = crate::languages::kit::native::read_recipe(
        package_root,
        &SPEC,
        crate::languages::cpp::syntax::mentions_declaration,
    )?
    else {
        return Ok(None);
    };
    let mut declared = Vec::new();
    for source in &sources {
        let path = crate::source_ref::locate_relative(package_root, source)
            .map_err(|error| format!("{}: {error}", manifest.display()))?;
        let text = std::fs::read_to_string(&path).map_err(|error| {
            format!(
                "{}: this build names `{source}` as a native source and cannot read it: {error}",
                manifest.display()
            )
        })?;
        let declared_faces = crate::languages::cpp::declarations::faces(&text)
            .map_err(|error| format!("{}: `{source}`: {error}", manifest.display()))?;
        if declared_faces.len() > 1 {
            return Err(format!(
                "{}: `{source}` declares {} faces; one translation unit declares one face, so the \
                 build cannot tell which one owns this file's module path — split them into one \
                 file per face",
                manifest.display(),
                declared_faces.len()
            ));
        }
        for face in declared_faces {
            if face.parent.is_empty() {
                return Err(format!(
                    "{}: a declaration in `{source}` never calls `.under(…)`; the parent is part of \
                     the face's position and cannot be omitted",
                    manifest.display()
                ));
            }
            if let Some(problem) = crate::languages::cpp::names::path_segment_problem(&face.name) {
                // The name is one logical path segment, and this is the one place every reader goes
                // through: the authoring adapter, the build's own reader (`declared_native::read`
                // calls this function), and therefore every consumer of the fact. A name that is a
                // path would be accepted by the template and then split the tree when it is read back.
                // 名字是一个逻辑路径段，而这里是每个读者都经过的唯一位置：创作适配器、构建自己的读取器
                // （`declared_native::read` 调用本函数），以及由此而来的每个事实消费者。一个"其实是路径"的
                // 名字会被模板接受，然后在读回时把树劈成两半。
                return Err(format!(
                    "{}: a declaration in `{source}` names its face `{}`, but {problem}; a face's name \
                     is one path segment, so it cannot be a path or a navigation segment",
                    manifest.display(),
                    face.name
                ));
            }
            declared.push(NativeFace {
                source: source.clone(),
                parent: face.parent,
                name: face.name,
                owns_registry: face.owns_registry,
            });
        }
    }
    if declared.is_empty() {
        return Err(format!(
            "{}: {} native source(s) were named and none of them declares a face; a source without \
             `XIRANG_SOIL`/`XIRANG_FACE` has nothing to register, so the recipe would compile a file \
             the tree never \
             hears about",
            manifest.display(),
            sources.len()
        ));
    }
    Ok(Some((recipe, declared)))
}

/// Compile one crate's share of the declared sources through the shared driver.
/// 经共享驱动编译本 crate 那一份声明的源码。
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

#[cfg(test)]
#[path = "native_tests.rs"]
mod cpp_native_tests;
