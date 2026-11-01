//! The Swift consumer backend: `swiftc -parse` each source, then compile every test into a program
//! that links the producers' shared libraries and run it.
//! Swift 消费者后端：先对每个源文件跑 `swiftc -parse`，再把每个测试编成链接生产者动态库的程序并运行。

use std::path::{Path, PathBuf};

use super::{Action, ActionKind, Project, Unit};

/// `swiftc -parse` every source file of a Swift unit.
/// 对 Swift 单元的每个源文件跑 `swiftc -parse`。
pub(super) fn swift_build_actions(sources: &[PathBuf], program: &str, root: &Path) -> Vec<Action> {
    sources
        .iter()
        .map(|source| Action {
            kind: ActionKind::Compile,
            command: vec![
                program.to_owned(),
                "-parse".to_owned(),
                source.display().to_string(),
            ],
            cwd: root.to_path_buf(),
            outputs: Vec::new(),
            test: None,
            environment: Vec::new(),
        })
        .collect()
}

/// The directory holding the Swift runtime libraries, found next to the compiler that will run.
/// 运行中的编译器旁边存放 Swift 运行库的目录。
fn swift_runtime(program: &str) -> Option<String> {
    let binary = if program.contains('/') {
        Path::new(program).to_path_buf()
    } else {
        let output = std::process::Command::new("sh")
            .arg("-c")
            .arg(format!("command -v {program}"))
            .output()
            .ok()?;
        let found = String::from_utf8_lossy(&output.stdout).trim().to_owned();
        if found.is_empty() {
            return None;
        }
        Path::new(&found).to_path_buf()
    };
    let prefix = binary.parent()?.parent()?;
    let libraries = prefix.join("lib");
    libraries
        .join("libswiftCore.so")
        .is_file()
        .then(|| libraries.display().to_string())
}

/// The environment a Swift compile or run needs: a writable module cache and the producers'
/// export directories plus the Swift runtime on the dynamic loader's path.
/// Swift 编译或运行所需的环境：可写的模块缓存，以及生产者导出目录与 Swift 运行库（放进动态加载器路径）。
fn swift_environment(
    directory: &Path,
    libraries: &[(String, String)],
    runtime: Option<&str>,
) -> Vec<(String, String)> {
    let mut environment = vec![
        (
            "HOME".to_owned(),
            directory.join("swift-home").display().to_string(),
        ),
        (
            "XDG_CACHE_HOME".to_owned(),
            directory.join("cache").display().to_string(),
        ),
        (
            "CLANG_MODULE_CACHE_PATH".to_owned(),
            directory.join("cache/clang").display().to_string(),
        ),
    ];
    let mut directories: Vec<String> = libraries.iter().map(|(_, dir)| dir.clone()).collect();
    if let Some(runtime) = runtime {
        directories.push(runtime.to_owned());
    }
    if !directories.is_empty() {
        environment.push(("LD_LIBRARY_PATH".to_owned(), directories.join(":")));
    }
    environment
}

/// The build and test actions of a Swift project.
/// Swift 项目的构建与测试动作。
pub(super) fn swift_test_plan(
    root: &Unit,
    project: &Project,
    program: &str,
    export_headers: &Path,
    directory: &Path,
    binaries: &Path,
) -> Result<Vec<Action>, String> {
    if program.is_empty() {
        return Err(
            "no Swift compiler was found; install swift or name one with XIRANG_SWIFT".to_owned(),
        );
    }
    if root.tests.is_empty() {
        return Err(
            "the Swift project has no `tests/*_test.swift`; `xirang test` would run nothing"
                .to_owned(),
        );
    }
    let runtime = swift_runtime(program);
    let libraries: Vec<(String, String)> = project
        .units
        .iter()
        .filter(|unit| !unit.manifest.exports.is_empty())
        .map(|unit| {
            (
                unit.key.clone(),
                export_headers.join(&unit.key).display().to_string(),
            )
        })
        .collect();
    let mut actions = swift_build_actions(&root.sources, program, &project.root);
    for test in &root.tests {
        let stem = test
            .file_stem()
            .and_then(|stem| stem.to_str())
            .ok_or_else(|| format!("{} has no file stem", test.display()))?;
        let binary = binaries.join(stem);
        let mut command = vec![
            program.to_owned(),
            test.display().to_string(),
            "-o".to_owned(),
            binary.display().to_string(),
        ];
        for (name, dir) in &libraries {
            command.push("-L".to_owned());
            command.push(dir.clone());
            command.push(format!("-l{name}"));
            command.push("-Xlinker".to_owned());
            command.push("-rpath".to_owned());
            command.push("-Xlinker".to_owned());
            command.push(dir.clone());
        }
        if let Some(runtime) = &runtime {
            command.push("-L".to_owned());
            command.push(runtime.clone());
            command.push("-Xlinker".to_owned());
            command.push("-rpath".to_owned());
            command.push("-Xlinker".to_owned());
            command.push(runtime.clone());
        }
        actions.push(Action {
            kind: ActionKind::Compile,
            command,
            cwd: project.root.clone(),
            outputs: vec![binary.clone()],
            test: None,
            environment: swift_environment(directory, &libraries, runtime.as_deref()),
        });
        actions.push(Action {
            kind: ActionKind::Test,
            command: vec![binary.display().to_string()],
            cwd: project.root.clone(),
            outputs: Vec::new(),
            test: Some(stem.to_owned()),
            environment: swift_environment(directory, &libraries, runtime.as_deref()),
        });
    }
    Ok(actions)
}
