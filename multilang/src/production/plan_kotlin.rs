//! The Kotlin consumer backend: `kotlinc` the unit's sources, compile its C bridge into a JNI
//! library, and run each test on the JVM.
//! Kotlin 消费者后端：用 `kotlinc` 编译单元源码，把它的 C 桥编成 JNI 库，再用 JVM 跑每个测试。

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use super::{Action, ActionKind, Unit};

/// One Kotlin unit's build: `kotlinc` the sources, then — when the unit carries C sources —
/// compile them into a JNI library that links the producers' archives.
/// 一个 Kotlin 单元的构建：先 `kotlinc` 源码，再（当单元带 C 源码时）把它们编成链接生产者归档的
/// JNI 库。
pub(super) struct KotlinBuild<'a> {
    pub(super) unit: &'a Unit,
    pub(super) root: &'a Path,
    pub(super) objects: &'a Path,
    pub(super) program: &'a str,
    pub(super) home: &'a str,
    pub(super) c_program: &'a str,
    pub(super) export_headers: &'a Path,
    pub(super) archives: &'a BTreeMap<String, PathBuf>,
    pub(super) order: &'a [String],
}

/// The compile and link actions of one Kotlin unit.
/// 一个 Kotlin 单元的编译与链接动作。
pub(super) fn kotlin_build_actions(request: KotlinBuild<'_>) -> Result<Vec<Action>, String> {
    let KotlinBuild {
        unit,
        root,
        objects,
        program,
        home,
        c_program,
        export_headers,
        archives,
        order,
    } = request;
    if program.is_empty() {
        return Err(format!(
            "unit `{}` is written in Kotlin, and no `kotlinc` was found; install one or name it \
             with XIRANG_KOTLIN",
            unit.manifest.package.name
        ));
    }
    let sources: Vec<&PathBuf> = unit
        .sources
        .iter()
        .filter(|path| path.extension().and_then(|extension| extension.to_str()) == Some("kt"))
        .collect();
    let classes = objects.join(&unit.key);
    let mut actions = Vec::new();
    if !sources.is_empty() {
        let mut command = vec![
            program.to_owned(),
            "-d".to_owned(),
            classes.display().to_string(),
        ];
        command.extend(sources.iter().map(|path| path.display().to_string()));
        actions.push(Action {
            kind: ActionKind::Compile,
            command,
            cwd: root.to_path_buf(),
            outputs: vec![classes.clone()],
            test: None,
            environment: Vec::new(),
        });
    }
    let bridges: Vec<&PathBuf> = unit
        .sources
        .iter()
        .filter(|path| path.extension().and_then(|extension| extension.to_str()) == Some("c"))
        .collect();
    if bridges.is_empty() {
        if actions.is_empty() {
            return Err(format!(
                "unit `{}` at {} has neither a `.kt` source nor a C bridge under src/",
                unit.manifest.package.name,
                unit.directory.display()
            ));
        }
        return Ok(actions);
    }
    if c_program.is_empty() {
        return Err(format!(
            "unit `{}` carries C sources for its JNI bridge, and no C compiler was found; install \
             gcc or name one with XIRANG_CC",
            unit.manifest.package.name
        ));
    }
    let library = classes.join(format!("lib{}.so", unit.manifest.package.name));
    let mut command = vec![
        c_program.to_owned(),
        "-shared".to_owned(),
        "-fPIC".to_owned(),
        "-std=c99".to_owned(),
    ];
    if !home.is_empty() {
        command.push(format!("-I{home}/include"));
        command.push(format!("-I{home}/include/linux"));
    }
    command.push(format!("-I{}", export_headers.display()));
    command.extend(bridges.iter().map(|path| path.display().to_string()));
    for name in order {
        if name == &unit.manifest.package.name {
            continue;
        }
        if let Some(archive) = archives.get(name) {
            command.push(archive.display().to_string());
        }
    }
    command.push("-lstdc++".to_owned());
    command.push("-lm".to_owned());
    command.push("-o".to_owned());
    command.push(library.display().to_string());
    actions.push(Action {
        kind: ActionKind::Link,
        command,
        cwd: root.to_path_buf(),
        outputs: vec![library],
        test: None,
        environment: Vec::new(),
    });
    Ok(actions)
}

/// The Kotlin standard library that ships beside `kotlinc`.
/// `kotlinc` 同级的 Kotlin 标准库。
fn stdlib(program: &str) -> Result<PathBuf, String> {
    let root = Path::new(program)
        .parent()
        .and_then(|bin| bin.parent())
        .ok_or_else(|| {
            format!("`{program}` is not a `…/bin/kotlinc` layout, so its Kotlin stdlib is unknown")
        })?;
    let candidate = root.join("lib").join("kotlin-stdlib.jar");
    if candidate.is_file() {
        Ok(candidate)
    } else {
        Err(format!(
            "`{program}` has no `../lib/kotlin-stdlib.jar` beside it; the tests need the Kotlin \
             runtime on the JVM classpath"
        ))
    }
}

/// The class name `kotlinc` gives a file with top-level declarations: its stem, capitalised,
/// plus `Kt`. `add_test.kt` therefore runs as `Add_testKt`.
/// `kotlinc` 给带顶层声明的文件起的类名：文件名首字母大写再加 `Kt`。`add_test.kt` 因此以
/// `Add_testKt` 运行。
fn class_name(stem: &str) -> String {
    let mut characters = stem.chars();
    let mut name = String::new();
    if let Some(first) = characters.next() {
        name.extend(first.to_uppercase());
        name.push_str(characters.as_str());
    }
    name.push_str("Kt");
    name
}

/// One compile action and one run action per Kotlin test file.
/// 每个 Kotlin 测试文件一个编译动作、一个运行动作。
pub(super) fn kotlin_test_actions(
    unit: &Unit,
    programs: &Path,
    libraries: &Path,
    program: &str,
    home: &str,
) -> Result<Vec<Action>, String> {
    if program.is_empty() {
        return Err("no `kotlinc` was found; install one or name it with XIRANG_KOTLIN".to_owned());
    }
    let runtime = if home.is_empty() {
        "java".to_owned()
    } else {
        format!("{home}/bin/java")
    };
    let standard = stdlib(program)?;
    let mut actions = Vec::new();
    for test in &unit.tests {
        let stem = test
            .file_stem()
            .and_then(|stem| stem.to_str())
            .ok_or_else(|| format!("{} has no file stem", test.display()))?;
        let classes = programs.join(stem);
        actions.push(Action {
            kind: ActionKind::Compile,
            command: vec![
                program.to_owned(),
                "-cp".to_owned(),
                libraries.display().to_string(),
                "-d".to_owned(),
                classes.display().to_string(),
                test.display().to_string(),
            ],
            cwd: unit.directory.clone(),
            outputs: vec![classes.clone()],
            test: None,
            environment: Vec::new(),
        });
        actions.push(Action {
            kind: ActionKind::Test,
            command: vec![
                runtime.clone(),
                format!("-Djava.library.path={}", libraries.display()),
                "-cp".to_owned(),
                format!(
                    "{}:{}:{}",
                    classes.display(),
                    libraries.display(),
                    standard.display()
                ),
                class_name(stem),
            ],
            cwd: unit.directory.clone(),
            outputs: Vec::new(),
            test: Some(stem.to_owned()),
            environment: Vec::new(),
        });
    }
    Ok(actions)
}

/// The build half of one Kotlin unit, for a plan that also builds dependencies.
/// 一个 Kotlin 单元的构建半边，供同时构建依赖的计划使用。
pub(super) fn kotlin_build_plan(
    unit: &Unit,
    root: &Path,
    objects: &Path,
    compiler: &super::Compiler,
    export_headers: &Path,
    archives: &BTreeMap<String, PathBuf>,
    order: &[String],
) -> Result<Vec<Action>, String> {
    kotlin_build_actions(KotlinBuild {
        unit,
        root,
        objects,
        program: &compiler.kotlin_program,
        home: &compiler.java_home,
        c_program: &compiler.c_program,
        export_headers,
        archives,
        order,
    })
}

/// The test half of the root Kotlin unit.
/// 根 Kotlin 单元的测试半边。
pub(super) fn kotlin_test_plan_from(
    project: &super::Project,
    objects: &Path,
    programs: &Path,
    compiler: &super::Compiler,
    export_headers: &Path,
    archives: &BTreeMap<String, PathBuf>,
    order: &[String],
) -> Result<Vec<Action>, String> {
    let unit = &project.units[0];
    if unit.tests.is_empty() {
        return Err(
            "the Kotlin project has no `tests/*_test.kt`; `xirang test` would run nothing"
                .to_owned(),
        );
    }
    let libraries = objects.join(&unit.key);
    let mut actions = kotlin_build_actions(KotlinBuild {
        unit,
        root: &project.root,
        objects,
        program: &compiler.kotlin_program,
        home: &compiler.java_home,
        c_program: &compiler.c_program,
        export_headers,
        archives,
        order,
    })?;
    actions.extend(kotlin_test_actions(
        unit,
        programs,
        &libraries,
        &compiler.kotlin_program,
        &compiler.java_home,
    )?);
    Ok(actions)
}
