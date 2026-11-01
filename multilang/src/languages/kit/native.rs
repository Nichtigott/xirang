//! Native build execution: the `[native]` recipe, the source discovery a language's extensions
//! drive, and the compiler/archiver commands one crate runs.
//! 原生构建执行：`[native]` 配方、由语言扩展名驱动的源码发现，以及一个 crate 运行的编译器/归档器命令。
//!
//! A native language owns only its extensions, its default recipe and its SDK directory; the driver
//! here is shared, so a second native language does not reimplement recipe reading, discovery or
//! process execution. It stays a plain function over explicit paths (no environment, no cargo
//! directives unless the caller asks) so the toolchain can call it from a build script and the same
//! code remains testable outside cargo.
//! 原生语言只拥有它的扩展名、默认配方与 SDK 目录；这里的驱动是共享的，因此第二种原生语言不必重写
//! 配方读取、发现或进程执行。它是显式路径上的普通函数（不读环境；只有调用方要求时才发 cargo 指令），
//! 因此工具链可以从构建脚本调用它，而同一份代码在 cargo 之外仍可测试。

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::external_faces::Recipe;

/// One source becoming one object, as the language's compiler spells that command.
/// 一份源码变成一份目标文件，按该语言编译器拼出的那条命令。
///
/// The flag shape is language-specific — a cc-like compiler takes `-c/-o/-I` and `zig` takes
/// `build-obj` with `-M` modules — so the spec hands the driver a function instead of the driver
/// growing a table of compilers. Everything the function needs is borrowed here, so the same code
/// stays testable without spawning anything.
/// 参数形状是语言自己的——cc 风格编译器用 `-c/-o/-I`，`zig` 用 `build-obj` 与 `-M` 模块——因此由 spec
/// 交给驱动一个函数，而不是让驱动长出一张编译器表。函数需要的一切都在这里借入，因此同一份代码仍可
/// 在不启动任何进程的情况下测试。
pub struct ObjectCommand<'a> {
    /// The package root the compilation belongs to.
    /// 本次编译所属的包根。
    pub package_root: &'a Path,
    /// The source file, already resolved inside the package.
    /// 已经解析到包内的源码文件。
    pub source: &'a Path,
    /// The object file this compilation must produce.
    /// 本次编译必须产出的目标文件。
    pub object: &'a Path,
    /// The build's own output directory (generated headers live here).
    /// 构建自己的输出目录（生成头住在这里）。
    pub out_dir: &'a Path,
    /// The language's SDK include directories.
    /// 该语言 SDK 的包含目录。
    pub include_dirs: &'a [PathBuf],
    /// The recipe the manifest wrote, or the language's default.
    /// 清单写下的配方，或该语言的默认配方。
    pub recipe: &'a Recipe,
}

/// How a cc-like compiler is invoked for one source: `-c` into the object file, then the SDK's,
/// the build's and the recipe's include directories.
/// 一份源码如何被 cc 风格编译器调用：`-c` 写到目标文件，随后是 SDK、构建与配方的三个包含目录。
///
/// C and C++ wrote this invocation out verbatim; the flags are the compiler's, not the language's,
/// so the shape lives next to the driver that starts the process.
/// C 与 C++ 曾一字不差地写过这次调用；这些开关属于编译器而不属于语言，因此形状住在启动进程的驱动旁边。
pub fn object_command(compile: &ObjectCommand<'_>) -> (String, Vec<String>) {
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

/// What one language tells the shared native driver.
/// 一门语言告诉共享原生驱动的东西。
pub struct NativeSpec {
    /// The file-name suffixes this build compiles.
    /// 本构建会编译的文件名后缀。
    pub extensions: &'static [&'static str],
    /// The compiler and arguments when the manifest writes no `[native]` recipe.
    /// 清单没有写 `[native]` 配方时的编译器与参数。
    pub default_recipe: fn(&[String]) -> (String, Vec<String>),
    /// Include directories the language's SDK adds to every compilation.
    /// 该语言 SDK 给每次编译添加的包含目录。
    pub include_dirs: fn() -> Vec<PathBuf>,
    /// Directories the shared source scope never enters.
    /// 共享源码范围从不进入的目录。
    pub skip_directories: &'static [&'static str],
    /// The language's compiler invocation: program and arguments for one source.
    /// 该语言的编译器调用：一份源码的程序与参数。
    pub object_command: fn(&ObjectCommand<'_>) -> (String, Vec<String>),
}

/// Read a package's native recipe and the sources it names or discovers.
/// 读出一个包的原生配方，以及它点名或发现的源码。
///
/// `[native]` overrides the recipe when it is written, and the sources are discovered otherwise —
/// a file that declares a face is a file this build compiles. Discovery is what keeps the author's
/// side free of a second declaration: the declaration marker is already the statement "this file
/// registers a face", so asking for the file name again would be the maintenance list this design
/// exists to delete. The extension picks the compiler, which is the one fact the source itself
/// cannot carry.
/// 写了 `[native]` 就覆盖配方，否则**发现**源码——声明了面的文件就是本次构建要编译的文件。发现让作者
/// 那一侧不必写第二份声明：声明标记本身已经说了"这个文件登记一个面"，再要一次文件名就是这套设计要删掉
/// 的那份维护清单。语言由扩展名给出——那是源码自己唯一带不了的事实。
pub fn read_recipe(
    package_root: &Path,
    spec: &NativeSpec,
    mentions: fn(&str) -> bool,
) -> Result<Option<(Recipe, Vec<String>)>, String> {
    let manifest = package_root.join("xirang.toml");
    let text = std::fs::read_to_string(&manifest).unwrap_or_default();
    let overrides = section(&text, "native");
    let listed = overrides.and_then(|section| strings(section, "sources"));
    let sources = match listed {
        Some(sources) if !sources.is_empty() => sources,
        _ => discover(package_root, spec, mentions)?,
    };
    if sources.is_empty() {
        return Ok(None);
    }
    let (default_program, default_arguments) = (spec.default_recipe)(&sources);
    let recipe = Recipe {
        program: overrides
            .and_then(|section| string(section, "program"))
            .unwrap_or(default_program),
        arguments: overrides
            .and_then(|section| strings(section, "args"))
            .unwrap_or(default_arguments),
        includes: overrides
            .and_then(|section| strings(section, "include"))
            .unwrap_or_default(),
        archive: overrides
            .and_then(|section| string(section, "archive"))
            .unwrap_or_else(|| "ar".to_owned()),
    };
    Ok(Some((recipe, sources)))
}

/// Compile one crate's share of the declared sources, returning one object per compiled source.
/// 编译本 crate 那一份声明的源码，每个编译过的源文件交回一个目标文件。
///
/// The objects land in this crate's `OUT_DIR` and the archive is linked into this crate, so the crate
/// that registers a face is the crate that links its implementation — one compile, one link, one
/// registrar. The caller's include directories (its SDK, for example) are added here, which is what
/// lets the author write the SDK include without naming the checkout.
/// 目标文件落在本 crate 的 `OUT_DIR`，归档链进本 crate，因此**登记一个面的 crate 就是链接它实现的
/// crate**——一次编译、一次链接、一个登记者。调用方的包含目录（例如它的 SDK）在这里补上，这正是作者
/// 能写 SDK 的 include 而不必点名本检出的原因。
pub fn compile(
    package_root: &Path,
    out_dir: &Path,
    recipe: &Recipe,
    sources: &[&str],
    include_dirs: &[PathBuf],
    spec: &NativeSpec,
    emit_cargo_directives: bool,
) -> Result<Vec<PathBuf>, String> {
    let mut objects = Vec::new();
    for source in sources {
        let stem = Path::new(source)
            .file_stem()
            .and_then(|stem| stem.to_str())
            .ok_or_else(|| {
                format!(
                    "`[native]` names `{source}`, whose file name has no stem this build can name \
                     an object after"
                )
            })?
            .replace(['-', '.'], "_");
        let object = out_dir.join(format!("{stem}.o"));
        // The compiler input goes through the shared source fact, not a second `package_root.join`:
        // the bridge, the adapter and the build must agree on which root applies and on what an
        // escaping path is.
        // 编译器输入走共享源码事实，而不是第二份 `package_root.join`：桥、适配器与构建必须在"哪个根
        // 适用"以及"越界路径是什么"上给出同一个答案。
        let source_path = crate::source_ref::locate_relative(package_root, source)
            .map_err(|error| format!("`[native]`: {error}"))?;
        // The language owns the flag shape; the driver owns discovery, archiving and directives.
        // 语言拥有参数形状；驱动拥有发现、归档与指令。
        let (program, arguments) = (spec.object_command)(&ObjectCommand {
            package_root,
            source: &source_path,
            object: &object,
            out_dir,
            include_dirs,
            recipe,
        });
        let mut command = Command::new(&program);
        command.args(&arguments);
        let output = command.output().map_err(|error| {
            format!(
                "`[native]` declares `{}` as the compiler and this build cannot run it: {error}",
                recipe.program
            )
        })?;
        if !output.status.success() {
            return Err(format!(
                "`[native]` compiling {source} failed ({}):\n{}",
                recipe.program,
                String::from_utf8_lossy(&output.stderr).trim_end()
            ));
        }
        let archive = out_dir.join(format!("lib{stem}.a"));
        let archived = Command::new(&recipe.archive)
            .arg("rcs")
            .arg(&archive)
            .arg(&object)
            .output()
            .map_err(|error| {
                format!(
                    "`[native]` declares `{}` as the archiver and this build cannot run it: {error}",
                    recipe.archive
                )
            })?;
        if !archived.status.success() {
            return Err(format!(
                "`[native]` archiving {source} failed ({}):\n{}",
                recipe.archive,
                String::from_utf8_lossy(&archived.stderr).trim_end()
            ));
        }
        if emit_cargo_directives {
            println!("cargo:rustc-link-search=native={}", out_dir.display());
            println!("cargo:rustc-link-lib=static={stem}");
            println!("cargo:rerun-if-changed={}", source_path.display());
        }
        objects.push(object);
    }
    Ok(objects)
}

/// The package's native sources that declare a face, as package-relative spellings.
/// 包里声明了面的本地源码，按包内相对拼写。
///
/// One walk, and the marker is the only filter: a source that never mentions the language's
/// declaration marker is a helper — included by a declarer, or not part of this build — and
/// compiling it would link an object the tree has no identity for. Directories a build never owns
/// are skipped by name.
/// 一次遍历，唯一的筛子就是那个标记：从不提该语言声明标记的源码是帮手——被声明者 include，或者根本
/// 不属于本次构建——编译它只会链进一个树里没有身份的目标文件。构建从不拥有的目录按名字跳过。
pub fn discover(
    package_root: &Path,
    spec: &NativeSpec,
    mentions: fn(&str) -> bool,
) -> Result<Vec<String>, String> {
    let declaration_spec = crate::languages::kit::declarations::DeclarationSpec {
        extensions: spec.extensions,
        skip_directories: spec.skip_directories,
        marker: mentions,
        // A build compiles what it can read; the walk itself does not decide that a file it cannot
        // open carries no declaration.
        // 构建只编译读得到的源码；遍历本身不判定"打不开的文件就没有声明"。
        unreadable: crate::languages::kit::declarations::Unreadable::Skip,
    };
    let mut sources = crate::languages::kit::declarations::read_package(
        package_root,
        &declaration_spec,
        |source, _text| Ok(vec![source.to_owned()]),
    )?;
    sources.sort();
    Ok(sources)
}

/// The `[name]` table's text, from its header to the next table header.
/// `[name]` 表的文本，从它的表头到下一个表头。
fn section<'a>(text: &'a str, name: &str) -> Option<&'a str> {
    let header = format!("[{name}]");
    let start = text.find(&header)? + header.len();
    let rest = &text[start..];
    let end = rest
        .match_indices('[')
        .find(|(index, _)| {
            let line_start = rest[..*index].rfind('\n').map(|at| at + 1).unwrap_or(0);
            rest[line_start..*index].trim().is_empty()
        })
        .map(|(index, _)| index)
        .unwrap_or(rest.len());
    Some(&rest[..end])
}

/// One `key = "value"` inside a section.
/// 表内的一条 `key = "value"`。
fn string(section: &str, key: &str) -> Option<String> {
    let mut values = strings(section, key)?;
    if values.len() == 1 {
        values.pop()
    } else {
        None
    }
}

/// One `key = ["a", "b"]` (or a single quoted value) inside a section.
/// 表内的一条 `key = ["a", "b"]`（或单个带引号的值）。
fn strings(section: &str, key: &str) -> Option<Vec<String>> {
    let mut values = Vec::new();
    for line in section.lines() {
        let line = line.split('#').next().unwrap_or("").trim();
        let Some(rest) = line.strip_prefix(key) else {
            continue;
        };
        let Some(rest) = rest.trim_start().strip_prefix('=') else {
            continue;
        };
        let rest = rest.trim();
        let body = if let Some(body) = rest.strip_prefix('[') {
            body.split(']').next().unwrap_or("")
        } else {
            rest
        };
        for piece in body.split('"').skip(1).step_by(2) {
            values.push(piece.to_owned());
        }
    }
    if values.is_empty() {
        None
    } else {
        Some(values)
    }
}

#[cfg(test)]
#[path = "native_tests.rs"]
mod native_tests;
