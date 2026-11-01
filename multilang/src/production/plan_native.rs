//! The plans of a C, C++ or other natively-linked unit: the binary link and the per-test
//! compile → link → run pipeline. They live here so `plan.rs` stays inside the size budget while
//! the shape of a native test stays in one place.
//! 原生链接单元（C/C++ 等）的计划：二进制链接，以及逐条测试的“编译 → 链接 → 运行”流水线。
//! 放在这里既让 `plan.rs` 保持尺寸预算内，也让原生测试的形状只有一处。

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use super::{Action, ActionKind, Unit};

/// The final link of a binary root: its own objects first, then every archive in link order.
/// 二进制根的最终链接：先它自己的目标文件，再按链接序接上所有归档。
pub(super) fn binary_link(
    root: &Unit,
    root_objects: &[PathBuf],
    library_order: &[String],
    library_archives: &BTreeMap<String, PathBuf>,
    link_driver: &str,
    programs: &Path,
    project_root: &Path,
) -> Action {
    let program = programs.join(&root.key);
    let mut command = vec![link_driver.to_owned()];
    command.extend(
        root_objects
            .iter()
            .map(|object| object.display().to_string()),
    );
    for name in library_order {
        if let Some(archive) = library_archives.get(name) {
            command.push(archive.display().to_string());
        }
    }
    command.push("-o".to_owned());
    command.push(program.display().to_string());
    Action {
        kind: ActionKind::Link,
        command,
        cwd: project_root.to_path_buf(),
        outputs: vec![program],
        test: None,
        environment: Vec::new(),
    }
}

/// What one native test plan needs from the project it belongs to.
/// 一份原生测试计划从所属项目需要的东西。
pub(super) struct NativeTest<'a> {
    pub(super) root: &'a Unit,
    pub(super) compiler: &'a super::Compiler,
    pub(super) project_root: &'a Path,
    pub(super) tests: &'a [PathBuf],
    pub(super) programs: &'a Path,
    pub(super) objects: &'a Path,
    pub(super) includes: &'a [PathBuf],
    pub(super) link_driver: &'a str,
    pub(super) root_archive: &'a Path,
    pub(super) library_order: &'a [String],
    pub(super) library_archives: &'a BTreeMap<String, PathBuf>,
}

/// Compile, link and run each native test file: one program per test, named after its stem.
/// 逐条编译、链接并运行原生测试文件：每条测试一个程序，以文件名词干命名。
pub(super) fn native_test_actions(request: NativeTest<'_>) -> Result<Vec<Action>, String> {
    let NativeTest {
        root,
        compiler,
        project_root,
        tests,
        programs,
        objects,
        includes,
        link_driver,
        root_archive,
        library_order,
        library_archives,
    } = request;
    let root_program = super::plan_language::root_program(root.manifest.language, compiler);
    let root_standard = super::plan_language::root_standard(root.manifest.language, &root.manifest);
    let mut actions = Vec::new();
    for source in tests {
        let stem = source
            .file_stem()
            .and_then(|stem| stem.to_str())
            .ok_or_else(|| format!("{} has no file stem", source.display()))?;
        let object = objects.join(format!("test__{stem}.o"));
        let mut compile = vec![
            root_program.clone(),
            root_standard.clone(),
            "-O0".to_owned(),
            "-g".to_owned(),
        ];
        for include in includes {
            compile.push(format!("-I{}", include.display()));
        }
        compile.push("-c".to_owned());
        compile.push(source.display().to_string());
        compile.push("-o".to_owned());
        compile.push(object.display().to_string());
        actions.push(Action {
            kind: ActionKind::Compile,
            command: compile,
            cwd: project_root.to_path_buf(),
            outputs: vec![object.clone()],
            test: None,
            environment: Vec::new(),
        });
        let program = programs.join(format!("{stem}_test"));
        let mut link = vec![link_driver.to_owned(), object.display().to_string()];
        link.push(root_archive.display().to_string());
        for name in library_order {
            if name == &root.key {
                continue;
            }
            if let Some(archive) = library_archives.get(name) {
                link.push(archive.display().to_string());
            }
        }
        link.push("-o".to_owned());
        link.push(program.display().to_string());
        actions.push(Action {
            kind: ActionKind::Link,
            command: link,
            cwd: project_root.to_path_buf(),
            outputs: vec![program.clone()],
            test: None,
            environment: Vec::new(),
        });
        let mut command = vec![program.display().to_string()];
        command.push("--xirang-test".to_owned());
        actions.push(Action {
            kind: ActionKind::Test,
            command,
            cwd: project_root.to_path_buf(),
            outputs: vec![program],
            test: Some(stem.to_owned()),
            environment: Vec::new(),
        });
    }
    Ok(actions)
}
