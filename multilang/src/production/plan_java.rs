//! The Java consumer backend: `javac` the unit's sources, compile its C bridge into a JNI library,
//! and run each test with the JDK's own `java`.
//! Java 消费者后端：`javac` 单元的源码，把它的 C 桥编成 JNI 库，再用 JDK 自己的 `java` 跑每个测试。

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use super::{Action, ActionKind, Unit};

/// One Java unit's build: `javac` the sources, then — when the unit carries C sources — compile
/// them into a JNI library that links the producers' archives.
/// 一个 Java 单元的构建：先 `javac` 源码，再（当单元带 C 源码时）把它们编成链接生产者归档的 JNI 库。
pub(super) struct JavaBuild<'a> {
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

/// The compile and link actions of one Java unit.
/// 一个 Java 单元的编译与链接动作。
pub(super) fn java_build_actions(request: JavaBuild<'_>) -> Result<Vec<Action>, String> {
    let JavaBuild {
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
            "unit `{}` is written in Java, and no JDK was found; install one or name `javac` with \
             XIRANG_JAVA",
            unit.manifest.package.name
        ));
    }
    let java_sources: Vec<&PathBuf> = unit
        .sources
        .iter()
        .filter(|path| path.extension().and_then(|extension| extension.to_str()) == Some("java"))
        .collect();
    let classes = objects.join(&unit.key);
    let mut actions = Vec::new();
    if !java_sources.is_empty() {
        let mut command = vec![
            program.to_owned(),
            "-d".to_owned(),
            classes.display().to_string(),
        ];
        command.extend(java_sources.iter().map(|path| path.display().to_string()));
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
                "unit `{}` at {} has neither a `.java` source nor a C bridge under src/",
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

/// One compile action and one run action per Java test file.
/// 每个 Java 测试文件一个编译动作、一个运行动作。
pub(super) fn java_test_actions(
    unit: &Unit,
    programs: &Path,
    library_directory: &Path,
    program: &str,
    home: &str,
) -> Result<Vec<Action>, String> {
    if program.is_empty() {
        return Err("no JDK was found; install one or name `javac` with XIRANG_JAVA".to_owned());
    }
    let runtime = if home.is_empty() {
        "java".to_owned()
    } else {
        format!("{home}/bin/java")
    };
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
                format!("-Djava.library.path={}", library_directory.display()),
                "-cp".to_owned(),
                format!("{}:{}", classes.display(), library_directory.display()),
                stem.to_owned(),
            ],
            cwd: unit.directory.clone(),
            outputs: Vec::new(),
            test: Some(stem.to_owned()),
            environment: Vec::new(),
        });
    }
    Ok(actions)
}

/// Build the Java unit and, when `programs` is given, schedule one compile and one run action per
/// test file. A test action's command is the program that decides the test, so the compile is a
/// separate `Compile` action.
/// 构建 Java 单元；给了 `programs` 时，为每个测试文件排一个编译动作和一个运行动作。测试动作用
/// 命令的退出码判定，因此编译必须是单独的 `Compile` 动作。
pub(super) fn java_test_plan(
    request: JavaBuild<'_>,
    programs: &Path,
) -> Result<Vec<Action>, String> {
    let unit = request.unit;
    let program = request.program.to_owned();
    let home = request.home.to_owned();
    if program.is_empty() {
        return Err("no JDK was found; install one or name `javac` with XIRANG_JAVA".to_owned());
    }
    if unit.tests.is_empty() {
        return Err(
            "the Java project has no `tests/*.java`; `xirang test` would run nothing".to_owned(),
        );
    }
    let objects = request.objects;
    let libraries = objects.join(&unit.key);
    let mut actions = java_build_actions(request)?;
    actions.extend(java_test_actions(
        unit, programs, &libraries, &program, &home,
    )?);
    Ok(actions)
}

/// The build half of one Java unit, for a plan that also builds dependencies.
/// 一个 Java 单元的构建半边，供同时构建依赖的计划使用。
pub(super) fn java_build_plan(
    unit: &Unit,
    root: &Path,
    objects: &Path,
    compiler: &super::Compiler,
    export_headers: &Path,
    archives: &BTreeMap<String, PathBuf>,
    order: &[String],
) -> Result<Vec<Action>, String> {
    java_build_actions(JavaBuild {
        unit,
        root,
        objects,
        program: &compiler.java_program,
        home: &compiler.java_home,
        c_program: &compiler.c_program,
        export_headers,
        archives,
        order,
    })
}

/// The test half of the root Java unit.
/// 根 Java 单元的测试半边。
pub(super) fn java_test_plan_from(
    project: &super::Project,
    objects: &Path,
    programs: &Path,
    compiler: &super::Compiler,
    export_headers: &Path,
    archives: &BTreeMap<String, PathBuf>,
    order: &[String],
) -> Result<Vec<Action>, String> {
    let unit = &project.units[0];
    java_test_plan(
        JavaBuild {
            unit,
            root: &project.root,
            objects,
            program: &compiler.java_program,
            home: &compiler.java_home,
            c_program: &compiler.c_program,
            export_headers,
            archives,
            order,
        },
        programs,
    )
}
