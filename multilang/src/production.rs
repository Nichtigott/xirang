//! The multilingual production surface: `xirang new --lang`, `xirang build`, `xirang test`.
//! 多语言生产面：`xirang new --lang`、`xirang build`、`xirang test`。
//!
//! This module is the first vertical slice of the open multilingual design: a project described by
//! `xirang.toml`, sources discovered from the filesystem, one plan, one local executor and one
//! evidence document. It refuses every language it has not implemented instead of pretending a
//! partial matrix is a finished one.
//! 本模块是开放多语言设计的第一块竖切：项目由 `xirang.toml` 描述，源码从文件系统清点，一份计划、
//! 一个本地执行器、一份证据文档。它拒绝每一个尚未实现的语言，而不是把不完整的矩阵说成完成的。

use std::path::Path;

#[path = "production/add.rs"]
mod add;
#[path = "production/backend_admin.rs"]
mod backend_admin;
#[path = "production/backends.rs"]
mod backends;
#[path = "production/bindings.rs"]
mod bindings;
#[path = "production/cache.rs"]
mod cache;
#[path = "production/declaration.rs"]
mod declaration;
#[path = "production/delegate.rs"]
mod delegate;
#[path = "production/delivery.rs"]
mod delivery;
#[path = "production/exec.rs"]
mod exec;
#[path = "production/gc.rs"]
mod gc;
#[path = "production/install.rs"]
mod install;
#[path = "production/language.rs"]
mod language;
#[path = "production/lock.rs"]
mod lock;
#[path = "production/log.rs"]
mod log;
#[path = "production/manifest.rs"]
mod manifest;
#[path = "production/move_source.rs"]
mod move_source;
#[path = "production/move_unit.rs"]
mod move_unit;
#[path = "production/package.rs"]
mod package;
#[path = "production/project.rs"]
mod project;
#[path = "production/queue.rs"]
mod queue;
#[path = "production/read.rs"]
mod read;
#[path = "production/record.rs"]
mod record;
#[path = "production/registry.rs"]
mod registry;
#[path = "production/report.rs"]
mod report;
#[path = "production/reuse.rs"]
mod reuse;
#[path = "production/scaffold.rs"]
mod scaffold;
#[path = "production/transfer.rs"]
mod transfer;
#[path = "production/worker.rs"]
mod worker;
#[path = "production/worker_client.rs"]
mod worker_client;
#[path = "production/worker_evidence.rs"]
mod worker_evidence;

#[path = "production/schema.rs"]
mod schema;
#[path = "production/sign.rs"]
mod sign;
#[path = "production/sources.rs"]
mod sources;
#[path = "production/tree.rs"]
mod tree;

pub use add::{SourceDeclaration, add_dependency, add_from_source};
pub use declaration::{check_document as declaration_check, starter as declaration_starter};
/// `xirang declaration tree`: the build-time registry tree this project's declaration documents
/// describe, printed without planning or building.
/// `xirang declaration tree`：本项目的声明文档所描述的构建期注册树，不计划也不构建，只打印。
pub fn declared_tree(root: &Path, json: bool) -> Result<String, String> {
    let project = project::Project::load(root)?;
    let tree = project.declared_tree()?;
    Ok(if json { tree.json() } else { tree.text() })
}

/// Validate a registry mirror (`xirang registry check`).
/// 校验一个注册表镜像（`xirang registry check`）。
pub fn registry_check(directory: &Path) -> Result<String, String> {
    registry::check(directory)
}

/// Fetch a registry mirror from an index whose sha256 the caller pins.
/// 按调用方钉住的 sha256，从索引取回一个注册表镜像。
pub fn registry_fetch(
    index: &str,
    digest: &str,
    into: &Path,
    auth: Option<&str>,
) -> Result<String, String> {
    registry::fetch(index, digest, into, auth)
}

/// Write a starter that prints one face declaration (`xirang declaration new`).
/// 写出一个会打印某个注册面声明的起点程序（`xirang declaration new`）。
/// Everything one `xirang declaration new` invocation declares.
/// 一次 `xirang declaration new` 调用声明的全部内容。
pub struct DeclarationSpec {
    /// The language whose starter prints the declaration.
    /// 由哪个语言的起点程序打印这条声明。
    pub language: String,
    /// The face name.
    /// 面的名字。
    pub name: String,
    /// The face kind.
    /// 面种类名。
    pub kind: String,
    /// Contract paths the face declares.
    /// 面声明的合同路径。
    pub contracts: Vec<String>,
    /// The preset type path, when one is declared.
    /// 声明的 preset 类型路径（若声明了）。
    pub preset: Option<String>,
    /// The parts type paths.
    /// parts 类型路径。
    pub parts: Vec<String>,
    /// Capability requirements, as `(capability, provider)`.
    /// 能力需求，按 `(capability, provider)` 给出。
    pub requires: Vec<(String, String)>,
    /// Capability names the face provides.
    /// 面提供的能力名。
    pub provides: Vec<String>,
    /// Export names.
    /// 导出名。
    pub exports: Vec<String>,
    /// The author-owned stable name, when one is declared.
    /// 作者拥有的稳定名（若声明了）。
    pub stable_name: Option<String>,
}

/// Write a starter that prints one face declaration (`xirang declaration new`).
/// 写出一个会打印某个注册面声明的起点程序（`xirang declaration new`）。
pub fn declaration_new(spec: &DeclarationSpec, directory: &Path) -> Result<String, String> {
    let face = declaration::Face {
        name: spec.name.clone(),
        kind: spec.kind.clone(),
        contracts: spec.contracts.clone(),
        graft: None,
        flow: None,
        preset: spec.preset.clone(),
        parts: spec.parts.clone(),
        requires: spec.requires.iter().cloned().collect(),
        provides: spec.provides.clone(),
        exports: spec.exports.clone(),
        stable_name: spec.stable_name.clone(),
    };
    let (extension, contents) = declaration_starter(&spec.language, &face)?;
    std::fs::create_dir_all(directory)
        .map_err(|error| format!("cannot create {}: {error}", directory.display()))?;
    let path = directory.join(format!("{}.{}", spec.name.to_lowercase(), extension));
    if path.is_file() {
        return Err(format!(
            "{} already exists; refusing to overwrite it",
            path.display()
        ));
    }
    std::fs::write(&path, contents)
        .map_err(|error| format!("cannot write {}: {error}", path.display()))?;
    Ok(format!(
        "wrote {}\n  run it, then feed its output to `xirang declaration check`",
        path.display()
    ))
}
pub use backend_admin::{BackendInstall, install_backend, languages, remove_backend};
pub use delivery::{
    cache_export, cache_import, history, install, log as log_verify, package, report, rollback,
    verify_package,
};
pub use read::{inventory, plan, plan_json};
pub use scaffold::scaffold as scaffold_backend;

/// Run the conformance kit against one recorded backend for `language`.
/// 对某语言的一个记录后端跑一致性检查。
pub fn check_backend(root: &Path, language: &str) -> Result<String, String> {
    backends::check(root, language)
}

/// The `xirang.toml` JSON Schema, for editors.
/// 供编辑器用的 `xirang.toml` JSON Schema。
pub fn manifest_schema() -> Result<String, String> {
    schema::render()
}

/// Render consumer bindings for the root project's `[[export]]` declarations.
/// 为根项目的 `[[export]]` 声明渲染消费端绑定。
pub fn bindings(root: &Path, language: &str) -> Result<String, String> {
    let manifest = manifest::Manifest::load(root)?;
    bindings::render(&manifest.package.name, &manifest.exports, language)
}

/// Collect a project's content store, reporting every entry it removes.
/// 回收一个项目的内容库，并报告它删掉的每个条目。
pub fn collect_garbage(root: &Path, dry_run: bool) -> Result<String, String> {
    gc::collect(&root.join("target/xirang"), dry_run)
}

/// Everything else the task names; used only to write an honest refusal.
/// 任务点名的其余语言；只用来写一句诚实的拒绝。
const NAMED: [&str; 0] = [];

pub use move_source::move_source;
/// Re-resolve the project and record the graph in `xirang.lock` as one transaction.
/// 重新解析项目，并把依赖图作为一次事务记录进 `xirang.lock`。
/// `xirang move <unit> <destination>`: one transaction over a directory and its dependents.
/// `xirang move <unit> <destination>`：一次同时覆盖目录与其依赖者的事务。
pub use move_unit::move_unit;
pub use sign::{sign_package, verify_signature};

/// Re-resolve the project and record the graph in `xirang.lock` as one transaction.
/// 重新解析项目，并把依赖图作为一次事务记录进 `xirang.lock`。
pub fn update(root: &Path, check: bool) -> Result<String, String> {
    let manifest = root.join("xirang.toml");
    if !manifest.is_file() {
        return Err(format!(
            "`xirang update` needs a project described by {}; this directory has none",
            manifest.display()
        ));
    }
    let project = project::load(root)?;
    lock::update(&project, check)
}

pub use worker::worker_job;
pub use worker_client::{
    Control, Transfer, remote_fetch_all, remote_listing, remote_many, remote_worker,
};

/// The persistent job queue's façade.
/// 持久任务队列的门面。
pub use queue::{
    cancel as queue_cancel, enqueue as queue_add, list as queue_list, run_queue as queue_run,
    show as queue_show,
};

/// Build the project at `root` and report the evidence path.
/// 构建 `root` 处的项目并报告证据路径。
/// Compatibility alias for the published `production::build` spelling.
/// 已发布拼法 `production::build` 的兼容别名。
#[deprecated(note = "use production::build_project; this alias keeps 0.2 callers compiling")]
pub fn build(root: &Path, locked: bool) -> Result<String, String> {
    build_project(root, locked)
}

/// Builds the project at `root` and reports the evidence path.
/// 构建 `root` 处的项目并报告证据路径。
pub fn build_project(root: &Path, locked: bool) -> Result<String, String> {
    let (_, plan, compiler, outcome, lock) = exec::run(root, false, locked)?;
    let document = exec::evidence(&plan, &compiler, &outcome);
    let evidence = exec::write_evidence(&plan, "build", &document)?;
    let mut summary = format!(
        "xirang build: plan {} ({} action(s), {}), evidence {}",
        plan.id,
        plan.actions.len(),
        compiler.version,
        evidence.display()
    );
    if let Some(backend) = &plan.backend {
        summary.push_str(&format!(
            "\nthrough the recorded backend `{}` ({}); the build ran its plan",
            backend.name,
            backend.program.join(" ")
        ));
    }
    let summary = match lock {
        Some(line) => format!("{summary}\n{line}"),
        None => summary,
    };
    match outcome.failure {
        Some(failure) => Err(format!("{summary}\n{failure}")),
        None => Ok(format!("{summary}\nbuilt: {}", plan.directory.display())),
    }
}

/// Run the project's test programs and report every verdict.
/// 运行项目的测试程序并报告每个判定。
pub fn test(root: &Path, locked: bool) -> Result<String, String> {
    let (_, plan, compiler, outcome, lock) = exec::run(root, true, locked)?;
    let document = exec::evidence(&plan, &compiler, &outcome);
    let evidence = exec::write_evidence(&plan, "test", &document)?;
    let passed = outcome.tests.iter().filter(|(_, passed)| *passed).count();
    let failed = outcome.tests.len().saturating_sub(passed);
    let mut summary = format!(
        "xirang test: plan {} ({} passed, {} failed), evidence {}",
        plan.id,
        passed,
        failed,
        evidence.display()
    );
    if let Some(backend) = &plan.backend {
        summary.push_str(&format!(
            "\nthrough the recorded backend `{}` ({}); the build ran its plan",
            backend.name,
            backend.program.join(" ")
        ));
    }
    for (name, verdict) in &outcome.tests {
        summary.push_str(&format!(
            "\ntest {name} ... {}",
            if *verdict { "ok" } else { "FAILED" }
        ));
    }
    if let Some(line) = lock {
        summary.push_str(&format!("\n{line}"));
    }
    if let Some(failure) = &outcome.failure {
        return Err(format!("{summary}\n{failure}"));
    }
    if outcome.tests.is_empty() {
        return Err(format!(
            "{summary}\nthe plan contains no test: a project with no `tests/*_test.cpp` has not \
             passed anything"
        ));
    }
    Ok(summary)
}

/// Scaffold a project in the language the caller named. Only a language a registered backend
/// answers for is written; every other language is refused by name.
/// 按调用方点名的语言搭建项目。只写出有已注册后端负责的语言；其它语言按名字拒绝。
pub fn new_project(
    directory: &Path,
    name: &str,
    language: &str,
    library: bool,
) -> Result<String, String> {
    let language = language.to_ascii_lowercase();
    // A registered backend is not a scaffold. `--lang java` used to fall through to the C++
    // files below and write a project in the wrong language; the languages this build
    // scaffolds are listed here, and a registered consumer language without a template is
    // refused by name instead.
    // 注册了后端不等于有脚手架：`--lang java` 过去会掉进下面的 C++ 文件表，写出一个语言不对的项目。
    // 这里列出本构建真正能搭的语言，注册为消费者但没有模板的语言按名拒绝。
    let scaffolded = matches!(
        language.as_str(),
        "cpp"
            | "c"
            | "python"
            | "go"
            | "ruby"
            | "csharp"
            | "javascript"
            | "typescript"
            | "php"
            | "swift"
    );
    if !scaffolded && backends::handshake(&language).is_ok() {
        return Err(format!(
            "`{language}` is registered as a consumer backend in this build, and this build \
                 has no `xirang new` scaffold for it; write `xirang.toml` and its sources by hand, \
                 the way the Java and Kotlin consumer tests do. A registered backend is not a \
                 scaffold, and writing a C++ project under this name would be a lie"
        ));
    }
    if let Err(registered) = backends::handshake(&language) {
        let named = if NAMED.contains(&language.as_str()) {
            format!("`{language}` is named by the task but is not implemented by this build")
        } else {
            format!("`{language}` is not a language this build knows")
        };
        return Err(format!(
            "{named}; {registered}. The support matrix is recorded in \
             `docs/multilang-support-matrix.md`, and an unimplemented language is not a partial \
             success."
        ));
    }
    if !valid_name(name) {
        return Err(format!(
            "`{name}` is not a package name: use lowercase letters, digits, `-` or `_`, starting \
             with a letter"
        ));
    }
    if !matches!(
        language.as_str(),
        "python"
            | "c"
            | "cpp"
            | "go"
            | "ruby"
            | "csharp"
            | "javascript"
            | "typescript"
            | "php"
            | "swift"
    ) {
        return Err(format!(
            "`{language}` has a backend but this build has no `xirang new` scaffold for it; \
             write `xirang.toml` and the sources by hand (the Java, Kotlin and Rust consumers are \
             brought in that way)"
        ));
    }
    if directory.exists() {
        let empty = std::fs::read_dir(directory)
            .map_err(|error| format!("cannot read {}: {error}", directory.display()))?
            .next()
            .is_none();
        if !empty {
            return Err(format!(
                "{} already exists and is not empty",
                directory.display()
            ));
        }
    }
    let module = name.replace('-', "_");
    let kind = if library { "library" } else { "binary" };
    let guard = module.to_uppercase();
    let files: Vec<(String, String)> = if language == "python" {
        if library {
            vec![
                (
                    "xirang.toml".to_owned(),
                    format!(
                        "schema = 1\n\n[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nkind = \"{kind}\"\n\n[language.python]\nstandard = \"3\"\n\n[dependencies]\n"
                    ),
                ),
                (
                    format!("src/{module}.py"),
                    "def answer() -> int:\n    return 42\n".to_owned(),
                ),
                (
                    format!("tests/{module}_test.py"),
                    format!(
                        "\"\"\"One test program for {name}; `xirang test` runs it.\"\"\"\nimport sys\n\nfrom {module} import answer\n\n\ndef main() -> int:\n    if answer() != 42:\n        print(\"test {module}_test ... FAILED\")\n        return 1\n    print(\"test {module}_test ... ok\")\n    return 0\n\n\nif __name__ == \"__main__\":\n    sys.exit(main())\n"
                    ),
                ),
            ]
        } else {
            vec![
                (
                    "xirang.toml".to_owned(),
                    format!(
                        "schema = 1\n\n[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nkind = \"{kind}\"\n\n[language.python]\nstandard = \"3\"\n\n[dependencies]\n"
                    ),
                ),
                (
                    format!("src/{module}.py"),
                    "\"\"\"The entry module.\"\"\"\n\n\ndef main() -> int:\n    print(\"hello from xirang\")\n    return 0\n".to_owned(),
                ),
            ]
        }
    } else if language == "typescript" {
        if !library {
            return Err(
                "a TypeScript unit is a consumer library in this slice; a TypeScript binary is \
                 not implemented"
                    .to_owned(),
            );
        }
        vec![
            (
                "xirang.toml".to_owned(),
                format!(
                    "schema = 1\n\n[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nkind = \"{kind}\"\n\n[language.typescript]\nstandard = \"strip\"\n\n[dependencies]\n"
                ),
            ),
            (
                format!("src/{module}.ts"),
                "export function answer(): number {\n  return 42;\n}\n".to_owned(),
            ),
            (
                format!("tests/{module}_test.ts"),
                "const assert = require(\"node:assert\");\n\nassert.strictEqual(1 + 1, 2);\nconsole.log(\"test lib_test ... ok\");\n".to_owned(),
            ),
        ]
    } else if language == "javascript" {
        if !library {
            return Err(
                "a JavaScript unit is a consumer library in this slice; a JavaScript binary is \
                 not implemented"
                    .to_owned(),
            );
        }
        vec![
            (
                "xirang.toml".to_owned(),
                format!(
                    "schema = 1\n\n[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nkind = \"{kind}\"\n\n[language.javascript]\nstandard = \"commonjs\"\n\n[dependencies]\n"
                ),
            ),
            (
                format!("src/{module}.js"),
                "function answer() {\n  return 42;\n}\n\nmodule.exports = { answer };\n".to_owned(),
            ),
            (
                format!("tests/{module}_test.js"),
                format!(
                    "const {{ answer }} = require(\"../src/{module}.js\");\n\nif (answer() !== 42) {{\n  console.log(\"test {module}_test ... FAILED\");\n  process.exit(1);\n}}\nconsole.log(\"test {module}_test ... ok\");\n"
                ),
            ),
        ]
    } else if language == "c" {
        if library {
            vec![
                (
                    "xirang.toml".to_owned(),
                    format!(
                        "schema = 1\n\n[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nkind = \"{kind}\"\n\n[language.c]\nstandard = \"99\"\n\n[dependencies]\n"
                    ),
                ),
                (
                    format!("include/{module}.h"),
                    format!(
                        "#ifndef {guard}_H\n#define {guard}_H\n\nint answer(void);\n\n#endif\n"
                    ),
                ),
                (
                    format!("src/{module}.c"),
                    format!("#include \"{module}.h\"\n\nint answer(void) {{\n    return 42;\n}}\n"),
                ),
                (
                    format!("tests/{module}_test.c"),
                    format!(
                        "#include \"{module}.h\"\n\nint main(void) {{\n    if (answer() != 42) {{\n        return 1;\n    }}\n    return 0;\n}}\n"
                    ),
                ),
            ]
        } else {
            vec![
                (
                    "xirang.toml".to_owned(),
                    format!(
                        "schema = 1\n\n[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nkind = \"{kind}\"\n\n[language.c]\nstandard = \"99\"\n\n[dependencies]\n"
                    ),
                ),
                (
                    format!("src/{module}.c"),
                    "#include <stdio.h>\n\nint main(void) {\n    printf(\"hello from xirang\\n\");\n    return 0;\n}\n".to_owned(),
                ),
            ]
        }
    } else if language == "go" {
        if !library {
            return Err(
                "a Go unit is a consumer library in this slice; a Go binary is not implemented"
                    .to_owned(),
            );
        }
        vec![
            (
                "xirang.toml".to_owned(),
                format!(
                    "schema = 1\n\n[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nkind = \"{kind}\"\n\n[language.go]\nstandard = \"1.21\"\n\n[dependencies]\n"
                ),
            ),
            (
                "go.mod".to_owned(),
                format!("module example.com/{module}\n\ngo 1.21\n"),
            ),
            (
                "src/lib.go".to_owned(),
                "package src\n\nfunc Answer() int { return 42 }\n".to_owned(),
            ),
            (
                "tests/lib_test.go".to_owned(),
                format!(
                    "package tests\n\nimport (\n\t\"testing\"\n\n\t\"example.com/{module}/src\"\n)\n\nfunc TestAnswer(t *testing.T) {{\n\tif got := src.Answer(); got != 42 {{\n\t\tt.Fatalf(\"got %d\", got)\n\t}}\n}}\n"
                ),
            ),
        ]
    } else if language == "ruby" {
        if !library {
            return Err(
                "a Ruby unit is a consumer library in this slice; a Ruby binary is not implemented"
                    .to_owned(),
            );
        }
        vec![
            (
                "xirang.toml".to_owned(),
                format!(
                    "schema = 1\n\n[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nkind = \"{kind}\"\n\n[language.ruby]\nstandard = \"3\"\n\n[dependencies]\n"
                ),
            ),
            (
                format!("src/{module}.rb"),
                "def answer\n  42\nend\n".to_owned(),
            ),
            (
                format!("tests/{module}_test.rb"),
                format!(
                    "require_relative \"../src/{module}\"\n\nraise \"answer() != 42\" unless answer == 42\nputs \"ok {module}_test\"\n"
                ),
            ),
        ]
    } else if language == "csharp" {
        if !library {
            return Err(
                "a C# unit is a consumer library in this slice; a C# binary is not implemented"
                    .to_owned(),
            );
        }
        vec![
            (
                "xirang.toml".to_owned(),
                format!(
                    "schema = 1\n\n[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nkind = \"{kind}\"\n\n[language.csharp]\nstandard = \"net10.0\"\n\n[dependencies]\n"
                ),
            ),
            (
                format!("src/{module}.cs"),
                "public static class Answer\n{\n    public static int Value() => 42;\n}\n"
                    .to_owned(),
            ),
            (
                format!("tests/{module}_test.cs"),
                "using System;\n\ninternal static class Test\n{\n    private static int Main()\n    {\n        if (Answer.Value() != 42)\n        {\n            Console.WriteLine(\"test answers ... FAILED\");\n            return 1;\n        }\n        Console.WriteLine(\"test answers ... ok\");\n        return 0;\n    }\n}\n"
                    .to_owned(),
            ),
        ]
    } else if language == "swift" {
        if !library {
            return Err(
                "a Swift unit is a consumer library in this slice; a Swift binary is not implemented"
                    .to_owned(),
            );
        }
        vec![
            (
                "xirang.toml".to_owned(),
                format!(
                    "schema = 1\n\n[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nkind = \"{kind}\"\n\n[language.swift]\nstandard = \"6.2\"\n\n[dependencies]\n"
                ),
            ),
            (
                format!("src/{module}.swift"),
                format!("public func {module}_answer() -> Int {{\n    return 42\n}}\n"),
            ),
            (
                format!("tests/{module}_test.swift"),
                format!(
                    "import Glibc\n\nif 1 + 1 != 2 {{\n    print(\"test {module}_test ... FAILED\")\n    exit(1)\n}}\nprint(\"test {module}_test ... ok\")\n"
                ),
            ),
        ]
    } else if language == "php" {
        vec![
            (
                "xirang.toml".to_owned(),
                format!(
                    "schema = 1\n\n[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nkind = \"{kind}\"\n\n[language.php]\nstandard = \"8.4\"\n\n[dependencies]\n"
                ),
            ),
            (
                format!("src/{module}.php"),
                format!("<?php\n\nfunction {module}_answer(): int\n{{\n    return 42;\n}}\n"),
            ),
            (
                format!("tests/{module}_test.php"),
                format!(
                    "<?php\nrequire __DIR__ . \"/../src/{module}.php\";\n\nif ({module}_answer() !== 42) {{\n    fwrite(STDERR, \"answer() != 42\\n\");\n    exit(1);\n}}\necho \"ok {module}_test\\n\";\n"
                ),
            ),
        ]
    } else if library {
        vec![
            (
                "xirang.toml".to_owned(),
                format!(
                    "schema = 1\n\n[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nkind = \"{kind}\"\n\n[language.cpp]\nstandard = \"17\"\n\n[dependencies]\n"
                ),
            ),
            (
                format!("include/{module}.hpp"),
                format!("#pragma once\n\nnamespace {module} {{\nint answer();\n}}\n"),
            ),
            (
                format!("src/{module}.cpp"),
                format!(
                    "#include \"{module}.hpp\"\n\nnamespace {module} {{\nint answer() {{ return 42; }}\n}}\n"
                ),
            ),
            (
                format!("tests/{module}_test.cpp"),
                format!(
                    "#include \"{module}.hpp\"\n\n#include <cstdio>\n\nint main() {{\n    if ({module}::answer() != 42) {{\n        std::fprintf(stderr, \"answer() != 42\\n\");\n        return 1;\n    }}\n    std::printf(\"ok {module}_test\\n\");\n    return 0;\n}}\n"
                ),
            ),
        ]
    } else {
        vec![
            (
                "xirang.toml".to_owned(),
                format!(
                    "schema = 1\n\n[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nkind = \"{kind}\"\n\n[language.cpp]\nstandard = \"17\"\n\n[dependencies]\n"
                ),
            ),
            (
                format!("src/{module}.cpp"),
                "#include <cstdio>\n\nint main() {\n    std::printf(\"hello from xirang\\n\");\n    return 0;\n}\n".to_owned(),
            ),
        ]
    };
    for (path, text) in &files {
        let target = directory.join(path);
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|error| format!("cannot create {}: {error}", parent.display()))?;
        }
        std::fs::write(&target, text)
            .map_err(|error| format!("cannot write {}: {error}", target.display()))?;
    }
    std::fs::write(
        directory.join("README.md"),
        format!(
            "# {name}\n\nA {language} {kind} with one `xirang.toml`, compiled by the local `xirang build`.\n\n```sh\nxirang build\n{}```\n",
            if library { "xirang test\n" } else { "" }
        ),
    )
    .map_err(|error| format!("cannot write the README: {error}"))?;
    std::fs::write(directory.join(".gitignore"), "/target\n")
        .map_err(|error| format!("cannot write .gitignore: {error}"))?;
    Ok(format!(
        "created {lang} {kind} `{name}` in {} ({} file(s)); the plan records its evidence under \
         target/xirang/build/<plan>/evidence.json{}",
        directory.display(),
        files.len() + 2,
        if library {
            "\nbuild it with `xirang build`, then run its tests with `xirang test`"
        } else {
            "\nbuild it with `xirang build`"
        },
        lang = language,
        kind = kind
    ))
}

/// A package name that cannot escape the directory it names or collide with an option.
/// 一个不能逃出它点名的目录、也不会与选项撞车的包名。
fn valid_name(name: &str) -> bool {
    let mut characters = name.chars();
    match characters.next() {
        Some(first) if first.is_ascii_alphabetic() => {}
        _ => return false,
    }
    characters
        .all(|character| character.is_ascii_alphanumeric() || character == '-' || character == '_')
}
