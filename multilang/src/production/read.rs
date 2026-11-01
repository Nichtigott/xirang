//! The read-only views of an `xirang.toml` project: its units (`inventory`) and the actions a
//! build would run (`plan`). They live apart from the writer entry points so the MCP
//! `xirang.multilang` tool can depend on exactly these two and nothing that builds.
//! `xirang.toml` 项目的只读视图：单元清单（`inventory`）与一次构建会跑的动作（`plan`）。它们与写入口
//! 分开，正是为了让 MCP 的 `xirang.multilang` 工具只依赖这两项、不依赖任何会构建的东西。

use std::path::Path;

use super::{exec, manifest, project};

/// Describe the units of an `xirang.toml` project without building anything.
/// 描述一个 `xirang.toml` 项目的单元，不构建任何东西。
pub fn inventory(root: &Path) -> Result<String, String> {
    let project = project::Project::load(root)?;
    let mut text = format!("{} unit(s)\n", project.units.len());
    for unit in &project.units {
        let package = &unit.manifest.package;
        let source = match &unit.source {
            Some(source) => format!("  source {source}"),
            None => String::new(),
        };
        text.push_str(&format!(
            "{}  {}  {}  {}  v{}{}  {} source file(s), {} test(s)\n",
            package.name,
            language_name(&unit.manifest.language),
            kind_name(package.kind),
            unit.directory.display(),
            package.version,
            source,
            unit.sources.len(),
            unit.tests.len()
        ));
    }
    Ok(text)
}

/// The actions `xirang build` would run, without running anything and without writing the fact
/// cache. The plan's identity is printed, so two answers can be compared.
/// 一次 `xirang build` 会跑的动作，不执行任何东西、也不写事实缓存。计划的身份会印出来，因此两次答复可以比较。
pub fn plan(root: &Path) -> Result<String, String> {
    let compiler = exec::detect()?;
    let project = project::Project::load(root)?;
    let plan = project.plan_read_only(&compiler)?;
    let mut text = format!(
        "plan {}  {} action(s)\ndirectory {}\ncompiler {} {}\n",
        plan.id,
        plan.actions.len(),
        plan.directory.display(),
        compiler.program,
        compiler.version
    );
    if let Some(backend) = &plan.backend {
        text.push_str(&format!(
            "backend {} ({}) inputs {} through {}\n",
            backend.name,
            backend.language,
            backend.inputs,
            backend.program.join(" ")
        ));
    }
    for (index, action) in plan.actions.iter().enumerate() {
        text.push_str(&format!(
            "{index:04} {:?} {}\n",
            action.kind,
            action.command.join(" ")
        ));
        for output in &action.outputs {
            text.push_str(&format!("     -> {}\n", output.display()));
        }
        if let Some(test) = &action.test {
            text.push_str(&format!("     test {test}\n"));
        }
    }
    for (path, _) in &plan.generated {
        text.push_str(&format!("generated {}\n", path.display()));
    }
    text.push_str("nothing was executed\n");
    Ok(text)
}

/// The same plan as machine-readable JSON: one object with the identity, the compiler, the
/// actions (kind, argv, outputs, test name, working directory, environment) and the generated
/// files. Nothing is executed and the fact cache is not written, exactly like [`plan`].
/// 与 [`plan`] 同一份计划的机器可读 JSON：身份、编译器、动作（种类、argv、产物、测试名、工作目录、
/// 环境）与生成文件。和 [`plan`] 一样不执行任何东西，也不写事实缓存。
pub fn plan_json(root: &Path) -> Result<String, String> {
    let compiler = exec::detect()?;
    let project = project::Project::load(root)?;
    let plan = project.plan_read_only(&compiler)?;
    let actions: Vec<serde_json::Value> = plan
        .actions
        .iter()
        .enumerate()
        .map(|(index, action)| {
            serde_json::json!({
                "index": index,
                "kind": format!("{:?}", action.kind).to_lowercase(),
                "command": action.command,
                "outputs": action
                    .outputs
                    .iter()
                    .map(|output| output.display().to_string())
                    .collect::<Vec<_>>(),
                "test": action.test,
                "cwd": action.cwd.display().to_string(),
                "environment": action.environment,
            })
        })
        .collect();
    let generated: Vec<String> = plan
        .generated
        .iter()
        .map(|(path, _)| path.display().to_string())
        .collect();
    let document = serde_json::json!({
        "protocol": 1,
        "plan": plan.id,
        "directory": plan.directory.display().to_string(),
        "compiler": {"program": compiler.program, "version": compiler.version},
        "backend": plan.backend.as_ref().map(|backend| serde_json::json!({
            "language": backend.language,
            "name": backend.name,
            "program": backend.program,
            "program_digest": backend.program_digest,
            "inputs": backend.inputs,
        })),
        "actions": actions,
        "generated": generated,
        "executed": false,
    });
    serde_json::to_string_pretty(&document)
        .map_err(|error| format!("cannot render the plan as JSON: {error}"))
}

/// The language's spelling in this workspace (`cpp`, `javascript`, …), for the inventory line.
/// 该语言在本工作区的拼写（`cpp`、`javascript` 等），用于清单行。
pub(crate) fn language_name(language: &manifest::Language) -> &'static str {
    match language {
        manifest::Language::Cpp => "cpp",
        manifest::Language::C => "c",
        manifest::Language::Python => "python",
        manifest::Language::Rust => "rust",
        manifest::Language::Go => "go",
        manifest::Language::Ruby => "ruby",
        manifest::Language::Csharp => "csharp",
        manifest::Language::Java => "java",
        manifest::Language::Php => "php",
        manifest::Language::Kotlin => "kotlin",
        manifest::Language::Swift => "swift",
        manifest::Language::Javascript => "javascript",
        manifest::Language::Typescript => "typescript",
        manifest::Language::External => "external",
    }
}

/// The package kind's spelling (`library` or `binary`), for the inventory line.
/// 包类型的拼写（`library` 或 `binary`），用于清单行。
pub(crate) fn kind_name(kind: manifest::Kind) -> &'static str {
    match kind {
        manifest::Kind::Library => "library",
        manifest::Kind::Binary => "binary",
    }
}
