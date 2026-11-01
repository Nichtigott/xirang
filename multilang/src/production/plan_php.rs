//! The PHP consumer backend: `php -l` every source, then run each test with the producers'
//! export directories on the dynamic loader's path so `FFI::cdef` can find their shared libraries.
//! PHP 消费者后端：先对每个源码跑 `php -l`，再运行每个测试；生产者的导出目录放进动态加载器路径，
//! 让 `FFI::cdef` 能找它们的共享库。

use std::path::{Path, PathBuf};

use super::{Action, ActionKind, Project, Unit};

/// `php -l` every source file of a PHP unit.
/// 对一个 PHP 单元的每个源码跑 `php -l`。
pub(super) fn php_build_actions(
    sources: &[PathBuf],
    program: &str,
    root: &Path,
) -> Result<Vec<Action>, String> {
    if program.is_empty() {
        return Err(
            "no PHP interpreter was found; install php or name one with XIRANG_PHP".to_owned(),
        );
    }
    Ok(sources
        .iter()
        .map(|source| Action {
            kind: ActionKind::Compile,
            command: vec![
                program.to_owned(),
                "-l".to_owned(),
                source.display().to_string(),
            ],
            cwd: root.to_path_buf(),
            outputs: Vec::new(),
            test: None,
            environment: Vec::new(),
        })
        .collect())
}

/// The build and test actions of a PHP project.
/// 一个 PHP 项目的构建与测试动作。
pub(super) fn php_test_plan(
    root: &Unit,
    project: &Project,
    program: &str,
    export_headers: &Path,
) -> Result<Vec<Action>, String> {
    if program.is_empty() {
        return Err(
            "no PHP interpreter was found; install php or name one with XIRANG_PHP".to_owned(),
        );
    }
    if root.tests.is_empty() {
        return Err(
            "the PHP project has no `tests/*_test.php`; `xirang test` would run nothing".to_owned(),
        );
    }
    let mut actions = php_build_actions(&root.sources, program, &project.root)?;
    // A PHP consumer loads the shared C ABI with `FFI::cdef`, so the dynamic loader has to find
    // the producer's `.so`. The plan names the directories that hold them, not today's file names.
    // PHP 消费者用 `FFI::cdef` 加载 C ABI 动态库，因此动态加载器要能找到生产者的 `.so`。计划给出
    // 存放它们的目录，而不是今天的文件名清单。
    let library_directories =
        super::plan_consumers::export_directories(&project.units, export_headers);
    let joined = library_directories.join(":");
    for test in &root.tests {
        actions.push(Action {
            kind: ActionKind::Test,
            command: vec![program.to_owned(), test.display().to_string()],
            cwd: project.root.clone(),
            outputs: Vec::new(),
            test: test
                .file_stem()
                .and_then(|stem| stem.to_str())
                .map(str::to_owned),
            environment: vec![("LD_LIBRARY_PATH".to_owned(), joined.clone())],
        });
    }
    Ok(actions)
}
