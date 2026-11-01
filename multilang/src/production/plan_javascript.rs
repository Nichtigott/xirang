//! The scripted consumers (JavaScript and TypeScript): `node --check` / a strip-only syntax check,
//! one N-API addon per unit, and one `node <test>` action per test file.
//! 脚本消费者（JavaScript 与 TypeScript）：`node --check` / strip-only 语法检查、每单元一个
//! N-API 插件、每测试文件一个 `node <test>` 动作。

use std::path::{Path, PathBuf};

use super::plan_consumers::export_directories;
use super::{Action, ActionKind, Unit};
use crate::production::manifest::Language;

/// The language word this slice's refusals use for a scripted unit.
/// 本切片在拒绝信息里称呼脚本单元的语言名。
fn scripted_language(unit: &Unit) -> &'static str {
    match unit.manifest.language {
        Language::Typescript => "TypeScript",
        _ => "JavaScript",
    }
}

/// The compile action that proves one TypeScript module parses. Node strips the types and a
/// `vm.SourceTextModule` parses the result, so a syntax error is a build failure and the check
/// never executes the module.
/// 证明一个 TypeScript 模块能解析的编译动作：Node 去掉类型、`vm.SourceTextModule` 解析结果，
/// 因此语法错误是构建失败，而且检查过程从不执行该模块。
fn typescript_check(program: &str, source: &Path) -> Vec<String> {
    let script = format!(
        "const {{stripTypeScriptTypes}} = require(\"node:module\"); \
         const {{SourceTextModule}} = require(\"node:vm\"); \
         const fs = require(\"node:fs\"); \
         new SourceTextModule(stripTypeScriptTypes(fs.readFileSync({:?}, \"utf8\")));",
        source.display().to_string()
    );
    vec![
        program.to_owned(),
        "--experimental-vm-modules".to_owned(),
        "--no-warnings".to_owned(),
        "-e".to_owned(),
        script,
    ]
}

/// The `.node` addon a JavaScript unit's C sources are compiled into.
/// JavaScript 单元的 C 源码编译出的 `.node` 插件路径。
pub(super) fn javascript_addon(objects: &Path, name: &str) -> PathBuf {
    objects.join(format!("{name}.node"))
}

/// A JavaScript unit's build: `node --check` for every `.js` module, then one N-API addon built
/// from the unit's `.c` sources when it has any.
/// JavaScript 单元的构建：每个 `.js` 模块跑 `node --check`，有 `.c` 源码时再把它们编成一个
/// N-API 插件。
pub(super) fn javascript_build_actions(
    unit: &Unit,
    root: &Path,
    objects: &Path,
    program: &str,
    c_program: &str,
    include: Option<&str>,
) -> Result<Vec<Action>, String> {
    if program.is_empty() {
        return Err(format!(
            "unit `{}` is written in {}, and no Node.js runtime was found; install node \
             or name one with XIRANG_NODE",
            unit.manifest.package.name,
            scripted_language(unit)
        ));
    }
    if unit.sources.is_empty() {
        return Err(format!(
            "unit `{}` at {} has no source file under src/",
            unit.manifest.package.name,
            unit.directory.display()
        ));
    }
    if !unit.manifest.exports.is_empty() {
        return Err(format!(
            "unit `{}` is written in {} and declares `[[export]]`; this slice exports \
             from C++ and Rust libraries only",
            unit.manifest.package.name,
            scripted_language(unit)
        ));
    }
    let module_extension = match unit.manifest.language {
        Language::Typescript => "ts",
        _ => "js",
    };
    let mut actions: Vec<Action> = unit
        .sources
        .iter()
        .filter(|source| {
            source.extension().and_then(|extension| extension.to_str()) == Some(module_extension)
        })
        .map(|source| Action {
            kind: ActionKind::Compile,
            command: match unit.manifest.language {
                Language::Typescript => typescript_check(program, source),
                _ => vec![
                    program.to_owned(),
                    "--check".to_owned(),
                    source.display().to_string(),
                ],
            },
            cwd: root.to_path_buf(),
            outputs: Vec::new(),
            test: None,
            environment: Vec::new(),
        })
        .collect();
    let addon_sources: Vec<&PathBuf> = unit
        .sources
        .iter()
        .filter(|source| source.extension().and_then(|extension| extension.to_str()) == Some("c"))
        .collect();
    if addon_sources.is_empty() {
        return Ok(actions);
    }
    let include = include.ok_or_else(|| {
        format!(
            "unit `{}` carries a C addon and Node headers were not found; set \
             XIRANG_NODE_INCLUDE to the directory holding node_api.h",
            unit.manifest.package.name
        )
    })?;
    if c_program.is_empty() {
        return Err(format!(
            "unit `{}` carries a C addon and no C compiler was found",
            unit.manifest.package.name
        ));
    }
    let output = javascript_addon(objects, &unit.key);
    let mut command = vec![
        c_program.to_owned(),
        "-shared".to_owned(),
        "-fPIC".to_owned(),
        "-std=c99".to_owned(),
        format!("-I{include}"),
    ];
    command.extend(
        addon_sources
            .iter()
            .map(|source| source.display().to_string()),
    );
    command.push("-o".to_owned());
    command.push(output.display().to_string());
    command.push("-ldl".to_owned());
    actions.push(Action {
        kind: ActionKind::Compile,
        command,
        cwd: root.to_path_buf(),
        outputs: vec![output],
        test: None,
        environment: Vec::new(),
    });
    Ok(actions)
}

/// The test actions of a JavaScript project: `node <test>` per `tests/*_test.js`, with the addon
/// path and the shared libraries' directories in the environment.
/// JavaScript 项目的测试动作：每个 `tests/*_test.js` 一条 `node <test>`，环境里带插件路径与
/// 共享库目录。
pub(super) fn javascript_test_actions(
    unit: &Unit,
    root: &Path,
    program: &str,
    objects: &Path,
    library_directories: &[String],
) -> Vec<Action> {
    let addon = javascript_addon(objects, &unit.manifest.package.name);
    let mut environment = vec![("XIRANG_ADDON".to_owned(), addon.display().to_string())];
    if !library_directories.is_empty() {
        environment.push(("LD_LIBRARY_PATH".to_owned(), library_directories.join(":")));
    }
    unit.tests
        .iter()
        .map(|test| Action {
            kind: ActionKind::Test,
            command: vec![program.to_owned(), test.display().to_string()],
            cwd: root.to_path_buf(),
            outputs: Vec::new(),
            test: None,
            environment: environment.clone(),
        })
        .collect()
}

/// Everything one JavaScript test plan needs. A struct rather than eight arguments keeps clippy's
/// `too_many_arguments` quiet without an `#[allow]`.
/// 一个 JavaScript 测试计划所需的全部输入。用结构体而不是八个参数，既避开 clippy 的
/// `too_many_arguments`，也不加 `#[allow]`。
pub(super) struct JavascriptTest<'a> {
    pub unit: &'a Unit,
    pub root: &'a Path,
    pub objects: &'a Path,
    pub program: &'a str,
    pub c_program: &'a str,
    pub include: Option<&'a str>,
    pub units: &'a [Unit],
    pub export_headers: &'a Path,
}

/// The complete test plan of a JavaScript project: build the modules and the addon, then run each
/// `tests/*_test.js` with the addon's path and the shared libraries' directories.
/// JavaScript 项目的完整测试计划：先构建模块与插件，再逐个运行 `tests/*_test.js`，环境里带插件
/// 路径与共享库目录。
pub(super) fn javascript_test_plan(request: JavascriptTest<'_>) -> Result<Vec<Action>, String> {
    if request.program.is_empty() {
        return Err(
            "no Node.js runtime was found; install node or name one with XIRANG_NODE".to_owned(),
        );
    }
    if request.unit.tests.is_empty() {
        return Err(format!(
            "the {} project has no test matching `tests/*_test.{}`; `xirang test` would run \
             nothing",
            scripted_language(request.unit),
            match request.unit.manifest.language {
                Language::Typescript => "ts",
                _ => "js",
            }
        ));
    }
    let mut actions = javascript_build_actions(
        request.unit,
        request.root,
        request.objects,
        request.program,
        request.c_program,
        request.include,
    )?;
    let library_directories = export_directories(request.units, request.export_headers);
    actions.extend(javascript_test_actions(
        request.unit,
        request.root,
        request.program,
        request.objects,
        &library_directories,
    ));
    Ok(actions)
}
