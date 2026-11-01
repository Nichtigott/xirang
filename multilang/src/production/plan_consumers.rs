//! The consumer test plans that are more than "compile one file and run it": Python imports the
//! producers' modules, Ruby loads the producers' shared libraries with Fiddle.
//! 那些不只是"编译一个文件再运行"的消费者测试计划：Python 导入生产者的模块，Ruby 用 Fiddle 加载
//! 生产者的共享库。

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use super::{Action, ActionKind, Unit, generate, go_environment};

/// The export directory of every producer in the graph, in unit order: the place its generated
/// header, `ctypes` module and shared library live.
/// 图中每个生产者的导出目录，按单元顺序：它生成的头部、`ctypes` 模块与共享库所在之处。
pub(super) fn export_directories(units: &[Unit], export_headers: &Path) -> Vec<String> {
    let mut directories = Vec::new();
    for unit in units {
        if !unit.manifest.exports.is_empty() {
            directories.push(export_headers.join(&unit.key).display().to_string());
        }
    }
    directories
}

/// One `Test` action per `tests/*_test.rb`. The producers' export directories go on
/// `LD_LIBRARY_PATH` so `Fiddle.dlopen("lib<unit>.so")` finds the C ABI the C++ unit published.
/// 每个 `tests/*_test.rb` 一条 `Test` 动作。生产者的导出目录放进 `LD_LIBRARY_PATH`，让
/// `Fiddle.dlopen("lib<unit>.so")` 找到 C++ 单元发布的 C ABI。
pub(super) fn ruby_actions(
    directory: &Path,
    tests: &[PathBuf],
    ruby_program: &str,
    library_directories: &[String],
) -> Result<Vec<Action>, String> {
    let mut actions = Vec::new();
    for source in tests {
        let stem = source
            .file_stem()
            .and_then(|stem| stem.to_str())
            .ok_or_else(|| format!("{} has no file stem", source.display()))?;
        actions.push(Action {
            kind: ActionKind::Test,
            command: vec![ruby_program.to_owned(), source.display().to_string()],
            cwd: directory.to_path_buf(),
            outputs: Vec::new(),
            test: Some(stem.to_owned()),
            environment: vec![("LD_LIBRARY_PATH".to_owned(), library_directories.join(":"))],
        });
    }
    Ok(actions)
}

/// One `Test` action per `tests/*_test.py`, with `PYTHONPATH` naming the producers' modules.
/// 每个 `tests/*_test.py` 一条 `Test` 动作，`PYTHONPATH` 给出生产者的模块路径。
pub(super) fn python_actions(
    directory: &Path,
    tests: &[PathBuf],
    python_program: &str,
    python_paths: &[String],
) -> Result<Vec<Action>, String> {
    let mut actions = Vec::new();
    for source in tests {
        let stem = source
            .file_stem()
            .and_then(|stem| stem.to_str())
            .ok_or_else(|| format!("{} has no file stem", source.display()))?;
        actions.push(Action {
            kind: ActionKind::Test,
            command: vec![python_program.to_owned(), source.display().to_string()],
            cwd: directory.to_path_buf(),
            outputs: Vec::new(),
            test: Some(stem.to_owned()),
            environment: vec![("PYTHONPATH".to_owned(), python_paths.join(":"))],
        });
    }
    Ok(actions)
}

/// The Go test action: one `go test -v ./...` for the unit, with the producers' export
/// directories on `CGO_CFLAGS`/`CGO_LDFLAGS` (see `go_environment`) so cgo compiles against the
/// generated headers and links the static archives.
/// Go 测试动作：单元一条 `go test -v ./...`，生产者的导出目录放进 `CGO_CFLAGS`/`CGO_LDFLAGS`
/// （见 `go_environment`），让 cgo 按生成的头部编译并链接静态归档。
pub(super) fn go_actions(
    root: &Unit,
    go_program: &str,
    library_order: &[String],
    library_archives: &BTreeMap<String, PathBuf>,
    export_headers: &Path,
    objects: &Path,
) -> Result<Vec<Action>, String> {
    if go_program.is_empty() {
        return Err("no Go toolchain was found; install go or name one with XIRANG_GO".to_owned());
    }
    if !root.directory.join("go.mod").is_file() {
        return Err(
            "a Go project needs a `go.mod`: the plan does not invent a module path".to_owned(),
        );
    }
    if root.tests.is_empty() {
        return Err(
            "the Go project has no `tests/*_test.go`; `xirang test` would run nothing".to_owned(),
        );
    }
    Ok(vec![Action {
        kind: ActionKind::Test,
        command: vec![
            go_program.to_owned(),
            "test".to_owned(),
            "-v".to_owned(),
            "./...".to_owned(),
        ],
        cwd: root.directory.clone(),
        outputs: Vec::new(),
        test: Some(root.manifest.package.name.clone()),
        environment: go_environment(library_order, library_archives, export_headers, objects),
    }])
}

/// The Rust test actions: `rustc --test` compiles a libtest binary, and a second action runs it.
/// The archives are linked explicitly because rustc does not know what `-L`/`-l` the plan's
/// libraries need.
/// Rust 测试动作：`rustc --test` 编出 libtest 二进制，第二条动作运行它。归档要显式链接，因为 rustc
/// 不知道计划里的库需要哪些 `-L`/`-l`。
pub(super) fn rust_test_actions(
    root: &Unit,
    rust_program: &str,
    programs: &Path,
    library_order: &[String],
    library_archives: &BTreeMap<String, PathBuf>,
    units: &[Unit],
) -> Result<Vec<Action>, String> {
    let links_cpp = units
        .iter()
        .any(|unit| unit.manifest.language == crate::production::manifest::Language::Cpp);
    let mut actions = Vec::new();
    for source in &root.tests {
        let stem = source
            .file_stem()
            .and_then(|stem| stem.to_str())
            .ok_or_else(|| format!("{} has no file stem", source.display()))?;
        let program = programs.join(format!("{stem}_test"));
        let mut command = vec![
            rust_program.to_owned(),
            "--edition".to_owned(),
            root.manifest.rust.standard.clone(),
            "--test".to_owned(),
            source.display().to_string(),
            "-o".to_owned(),
            program.display().to_string(),
        ];
        for name in library_order {
            if let Some(archive) = library_archives.get(name) {
                command.push(format!("-Clink-arg={}", archive.display()));
            }
        }
        if links_cpp {
            command.push("-Clink-arg=-lstdc++".to_owned());
        }
        actions.push(Action {
            kind: ActionKind::Compile,
            command,
            cwd: root.directory.clone(),
            outputs: vec![program.clone()],
            test: None,
            environment: Vec::new(),
        });
        actions.push(Action {
            kind: ActionKind::Test,
            command: vec![program.display().to_string()],
            cwd: root.directory.clone(),
            outputs: vec![program],
            test: Some(stem.to_owned()),
            environment: Vec::new(),
        });
    }
    Ok(actions)
}

/// A generated C# project: the unit's own sources, or one test program that references the
/// producers through `DllImport`. Default compile items are off so the project compiles exactly
/// the files the plan names.
/// 生成的 C# 项目：单元自己的源码，或一个通过 `DllImport` 引用生产者的测试程序。默认编译项关闭，
/// 项目只编译计划点名的文件。
pub(super) fn csharp_project(
    name: &str,
    standard: &str,
    output_type: &str,
    sources: &[PathBuf],
) -> String {
    let mut text = format!(
        "<Project Sdk=\"Microsoft.NET.Sdk\">\n  <PropertyGroup>\n    <TargetFramework>{standard}</TargetFramework>\n    <OutputType>{output_type}</OutputType>\n    <AssemblyName>{name}</AssemblyName>\n    <EnableDefaultCompileItems>false</EnableDefaultCompileItems>\n    <ImplicitUsings>disable</ImplicitUsings>\n    <Nullable>disable</Nullable>\n  </PropertyGroup>\n  <ItemGroup>\n"
    );
    for source in sources {
        text.push_str(&format!(
            "    <Compile Include=\"{}\" />\n",
            source.display()
        ));
    }
    text.push_str("  </ItemGroup>\n</Project>\n");
    text
}

/// The environment the .NET SDK needs offline: a writable CLI home (the SDK writes a sentinel on
/// first use and refuses when it cannot), no telemetry, no logo. The producers' export
/// directories go on `LD_LIBRARY_PATH`, because `DllImport("lib<unit>.so")` is a `dlopen` by
/// soname.
/// .NET SDK 离线所需的环境：可写的 CLI 主目录（首次使用会写哨兵文件，写不进就拒绝）、关闭遥测与
/// 徽标。生产者的导出目录放进 `LD_LIBRARY_PATH`，因为 `DllImport("lib<unit>.so")` 是按 soname 的
/// `dlopen`。
pub(super) fn csharp_environment(
    directory: &Path,
    library_directories: &[String],
) -> Vec<(String, String)> {
    let mut environment = vec![
        (
            "DOTNET_CLI_HOME".to_owned(),
            directory.join("dotnet-home").display().to_string(),
        ),
        ("DOTNET_CLI_TELEMETRY_OPTOUT".to_owned(), "1".to_owned()),
        ("DOTNET_NOLOGO".to_owned(), "1".to_owned()),
    ];
    if !library_directories.is_empty() {
        environment.push(("LD_LIBRARY_PATH".to_owned(), library_directories.join(":")));
    }
    environment
}

/// Everything one C# test plan needs: the build directory, the unit's own sources, its tests,
/// the SDK, the target framework, the programs directory and the producers' export directories.
/// 一份 C# 测试计划所需的全部：构建目录、单元自己的源码、测试、SDK、目标框架、程序目录与生产者的
/// 导出目录。
pub(super) struct CsharpTest<'a> {
    pub(super) directory: &'a Path,
    pub(super) sources: &'a [PathBuf],
    pub(super) tests: &'a [PathBuf],
    pub(super) program: &'a str,
    pub(super) standard: &'a str,
    pub(super) programs: &'a Path,
    pub(super) library_directories: &'a [String],
}

/// One `Compile` action to build a test project, and the `Test` action that runs its program.
/// 一条构建测试项目的 `Compile` 动作，以及运行该程序的 `Test` 动作。
pub(super) fn csharp_test_actions(
    request: CsharpTest<'_>,
    generated: &mut Vec<(PathBuf, String)>,
) -> Result<Vec<Action>, String> {
    let mut actions = Vec::new();
    for source in request.tests {
        let stem = source
            .file_stem()
            .and_then(|stem| stem.to_str())
            .ok_or_else(|| format!("{} has no file stem", source.display()))?;
        let project = request
            .directory
            .join("csharp")
            .join(format!("{stem}.csproj"));
        let mut compiled: Vec<PathBuf> = request.sources.to_vec();
        compiled.push(source.clone());
        generated.push((
            project.clone(),
            csharp_project(stem, request.standard, "Exe", &compiled),
        ));
        let binaries = request.programs.join(stem);
        actions.push(Action {
            kind: ActionKind::Compile,
            command: vec![
                request.program.to_owned(),
                "build".to_owned(),
                project.display().to_string(),
                "--nologo".to_owned(),
                "-v".to_owned(),
                "q".to_owned(),
                "-c".to_owned(),
                "Release".to_owned(),
                "-o".to_owned(),
                binaries.display().to_string(),
            ],
            cwd: request.directory.to_path_buf(),
            outputs: Vec::new(),
            test: None,
            environment: csharp_environment(request.directory, &[]),
        });
        actions.push(Action {
            kind: ActionKind::Test,
            command: vec![
                request.program.to_owned(),
                binaries.join(format!("{stem}.dll")).display().to_string(),
            ],
            cwd: request.directory.to_path_buf(),
            outputs: Vec::new(),
            test: Some(stem.to_owned()),
            environment: csharp_environment(request.directory, request.library_directories),
        });
    }
    Ok(actions)
}

/// One `Compile` action that builds a C# library unit: a generated project whose default compile
/// items are off, so it compiles exactly the files the plan names.
/// 一条构建 C# 库单元的 `Compile` 动作：生成的工程关闭默认编译项，只编译计划点名的文件。
pub(super) fn csharp_build_actions(
    unit: &Unit,
    directory: &Path,
    objects: &Path,
    program: &str,
    generated: &mut Vec<(PathBuf, String)>,
) -> Vec<Action> {
    let name = &unit.manifest.package.name;
    let key = &unit.key;
    let project = directory.join("csharp").join(format!("{key}.csproj"));
    generated.push((
        project.clone(),
        csharp_project(
            name,
            &unit.manifest.csharp.standard,
            "Library",
            &unit.sources,
        ),
    ));
    vec![Action {
        kind: ActionKind::Compile,
        command: vec![
            program.to_owned(),
            "build".to_owned(),
            project.display().to_string(),
            "--nologo".to_owned(),
            "-v".to_owned(),
            "q".to_owned(),
            "-c".to_owned(),
            "Release".to_owned(),
            "-o".to_owned(),
            objects.join(name).display().to_string(),
        ],
        cwd: directory.to_path_buf(),
        outputs: Vec::new(),
        test: None,
        environment: csharp_environment(directory, &[]),
    }]
}

/// Build one Go unit: `go build ./...`, with the toolchain pinned to `local` and the cache inside
/// the plan (see `go_environment`).
/// 构建一个 Go 单元：`go build ./...`，工具链固定为 `local`，缓存放在计划目录（见 `go_environment`）。
pub(super) fn go_build_actions(
    unit: &Unit,
    program: &str,
    environment: Vec<(String, String)>,
) -> Result<Vec<Action>, String> {
    let name = &unit.manifest.package.name;
    if program.is_empty() {
        return Err(format!(
            "unit `{name}` is written in Go, and no Go toolchain was found; install go or name \
             one with XIRANG_GO"
        ));
    }
    if !unit.directory.join("go.mod").is_file() {
        return Err(format!(
            "unit `{name}` is written in Go and has no `go.mod`; a Go unit is a module the plan \
             does not invent"
        ));
    }
    if !unit.manifest.exports.is_empty() {
        return Err(format!(
            "unit `{name}` is written in Go and declares `[[export]]`; this slice exports from C++ \
             and Rust libraries only"
        ));
    }
    Ok(vec![Action {
        kind: ActionKind::Compile,
        command: vec![program.to_owned(), "build".to_owned(), "./...".to_owned()],
        cwd: unit.directory.clone(),
        outputs: Vec::new(),
        test: None,
        environment,
    }])
}

/// A Ruby library unit's build is its syntax check: `ruby -c` per source file.
/// Ruby 库单元的构建就是语法检查：逐个源文件跑 `ruby -c`。
pub(super) fn ruby_build_actions(sources: &[PathBuf], program: &str, root: &Path) -> Vec<Action> {
    sources
        .iter()
        .map(|source| Action {
            kind: ActionKind::Compile,
            command: vec![
                program.to_owned(),
                "-c".to_owned(),
                source.display().to_string(),
            ],
            cwd: root.to_path_buf(),
            outputs: Vec::new(),
            test: None,
            environment: Vec::new(),
        })
        .collect()
}

/// Everything one Rust library build needs: the unit, its plan directories, the compiler, and the
/// two maps the plan keeps so a dependent can link the library.
/// 一次 Rust 库构建所需的全部：单元、计划目录、编译器，以及计划用来让依赖方链接该库的两张表。
pub(super) struct RustBuild<'a> {
    pub(super) unit: &'a Unit,
    pub(super) root: &'a Path,
    pub(super) program: &'a str,
    pub(super) objects: &'a Path,
    pub(super) archives: &'a Path,
    pub(super) export_headers: &'a Path,
    pub(super) generated: &'a mut Vec<(PathBuf, String)>,
    pub(super) library_archives: &'a mut BTreeMap<String, PathBuf>,
}

/// Build one Rust library: an rlib, plus a staticlib wrapper when the unit declares exports. The
/// wrapper carries the C ABI; the rlib stays the crate's own artifact.
/// 构建一个 Rust 库：一个 rlib；声明了导出时再加一个 staticlib 包装器。包装器承载 C ABI，rlib 仍是
/// crate 自己的产物。
pub(super) fn rust_build_actions(request: RustBuild<'_>) -> Result<Vec<Action>, String> {
    let unit = request.unit;
    let name = &unit.manifest.package.name;
    let key = &unit.key;
    if request.program.is_empty() {
        return Err(format!(
            "unit `{name}` is written in Rust, and no Rust compiler was found; install rustc or \
             name one with XIRANG_RUSTC"
        ));
    }
    let source = unit.directory.join("src/lib.rs");
    if !source.is_file() {
        return Err(format!(
            "unit `{name}` is written in Rust and has no `src/lib.rs`; a Rust unit is a library \
             the plan does not invent"
        ));
    }
    let crate_name = key.replace('-', "_");
    let rlib = request.objects.join(format!("lib{crate_name}.rlib"));
    let mut actions = vec![Action {
        kind: ActionKind::Compile,
        command: vec![
            request.program.to_owned(),
            "--edition".to_owned(),
            unit.manifest.rust.standard.clone(),
            "--crate-type".to_owned(),
            "rlib".to_owned(),
            source.display().to_string(),
            "-o".to_owned(),
            rlib.display().to_string(),
        ],
        cwd: request.root.to_path_buf(),
        outputs: vec![rlib.clone()],
        test: None,
        environment: Vec::new(),
    }];
    if !unit.manifest.exports.is_empty() {
        let (header, wrapper) = generate::generated_rust_exports(key, &unit.manifest.exports)?;
        let header_path = request.export_headers.join(key).join("exports.h");
        let wrapper_path = request.objects.join(format!("{crate_name}__exports.rs"));
        request.generated.push((header_path, header));
        request.generated.push((wrapper_path.clone(), wrapper));
        let archive = request.archives.join(format!("lib{crate_name}.a"));
        actions.push(Action {
            kind: ActionKind::Compile,
            command: vec![
                request.program.to_owned(),
                "--edition".to_owned(),
                unit.manifest.rust.standard.clone(),
                "--crate-type".to_owned(),
                "staticlib".to_owned(),
                wrapper_path.display().to_string(),
                "-L".to_owned(),
                format!("dependency={}", request.objects.display()),
                "--extern".to_owned(),
                format!("{crate_name}={}", rlib.display()),
                "-o".to_owned(),
                archive.display().to_string(),
            ],
            cwd: request.root.to_path_buf(),
            outputs: vec![archive.clone()],
            test: None,
            environment: Vec::new(),
        });
        request.library_archives.insert(key.clone(), archive);
    }
    Ok(actions)
}

/// The test half of a Python unit: the interpreter needs the unit's own sources and the generated
/// export headers on its path.
/// Python 单元的测试半边：解释器需要把单元自己的源码与生成的导出头放进搜索路径。
pub(super) fn python_test_plan(
    root: &Path,
    tests: &[PathBuf],
    python_sources: &[PathBuf],
    program: &str,
    export_headers: &Path,
) -> Result<Vec<Action>, String> {
    let mut paths: Vec<String> = python_sources
        .iter()
        .map(|path| path.display().to_string())
        .collect();
    paths.push(export_headers.display().to_string());
    python_actions(root, tests, program, &paths)
}
