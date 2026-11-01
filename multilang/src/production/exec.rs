//! The local executor for the first multilingual slice: detect a C++ compiler, run the plan,
//! and write the evidence that says what actually ran.
//! 第一块多语言竖切的本地执行器：探测 C++ 编译器、运行计划、写出"实际跑了什么"的证据。
//!
//! The executor owns processes and the filesystem; the plan it consumes is a value built by
//! `project` from what the sources and `xirang.toml` say. Evidence is written next to the build
//! directory so a later cache and a later remote executor can compare the same fields.
//! 执行器拥有进程与文件系统；它消费的计划是 `project` 依据源码与 `xirang.toml` 构造的值。证据写在
//! 构建目录里，供将来的缓存与远端执行器比较同样的字段。

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

use super::project::{ActionKind, Compiler, Plan, Project};
use super::reuse;

/// A compiler this machine actually has, with the version the evidence records.
/// 本机真的有的编译器，带证据要记录的版本。
pub(crate) fn detect() -> Result<Compiler, String> {
    // Prefer the compiler whose standard-library headers are installed here: clang++ on this
    // machine has no libstdc++ include path, so `#include <cstdio>` fails under it. The probe is
    // the real `--version`, and the chosen program is recorded in the evidence.
    // 优先选择本机装有标准库头文件的编译器：本机的 clang++ 没有 libstdc++ 的 include 路径，因此在它
    // 下面 `#include <cstdio>` 会失败。探测用的是真的 `--version`，选中的程序记进证据。
    let (program, version) = pick("XIRANG_CXX", &["g++", "clang++"], "C++")?;
    // The C compiler is optional: a C++-only project must not start failing because `gcc` is absent.
    // A project that has a C unit refuses by name in `Project::plan`.
    // C 编译器是可选的：纯 C++ 项目不能因为本机没有 `gcc` 就开始失败。含 C 单元的项目在
    // `Project::plan` 里按名字拒绝。
    let (c_program, c_version) = pick("XIRANG_CC", &["gcc", "clang"], "C")
        .unwrap_or_else(|_| (String::new(), String::new()));
    // Python is optional too: only a project with a Python unit needs an interpreter.
    // Python 同样是可选的：只有含 Python 单元的项目才需要解释器。
    let (python_program, python_version) = pick("XIRANG_PYTHON", &["python3", "python"], "Python")
        .unwrap_or_else(|_| (String::new(), String::new()));
    // Rust is optional as well: only a project with a Rust unit needs `rustc`.
    // Rust 同样是可选的：只有含 Rust 单元的项目才需要 `rustc`。
    let (rust_program, rust_version) =
        pick("XIRANG_RUSTC", &["rustc"], "Rust").unwrap_or_else(|_| (String::new(), String::new()));
    // Go is optional too: only a project with a Go unit needs the toolchain.
    // Go 同样是可选的：只有含 Go 单元的项目才需要工具链。
    let (go_program, go_version) = pick_with("XIRANG_GO", &["go"], "Go", "version")
        .unwrap_or_else(|_| (String::new(), String::new()));
    // Ruby is optional too: only a project with a Ruby unit needs the interpreter.
    // Ruby 同样是可选的：只有含 Ruby 单元的项目才需要解释器。
    let (ruby_program, ruby_version) =
        pick("XIRANG_RUBY", &["ruby"], "Ruby").unwrap_or_else(|_| (String::new(), String::new()));
    // The .NET SDK is optional too: only a project with a C# unit needs it.
    // .NET SDK 同样是可选的：只有含 C# 单元的项目才需要它。
    let (csharp_program, csharp_version) =
        pick_with("XIRANG_DOTNET", &["dotnet"], "C#", "--version")
            .unwrap_or_else(|_| (String::new(), String::new()));
    // The JDK is optional too: only a project with a Java unit needs it.
    // JDK 同样是可选的：只有含 Java 单元的项目才需要它。
    let (java_program, java_version) =
        pick("XIRANG_JAVA", &["javac"], "Java").unwrap_or_else(|_| (String::new(), String::new()));
    // Kotlin is optional too: only a project with a Kotlin unit needs the compiler.
    // Kotlin 同样是可选的：只有含 Kotlin 单元的项目才需要编译器。
    let (kotlin_program, kotlin_version) =
        pick_with("XIRANG_KOTLIN", &["kotlinc"], "Kotlin", "-version")
            .unwrap_or_else(|_| (String::new(), String::new()));
    let (php_program, php_version) =
        pick("XIRANG_PHP", &["php"], "PHP").unwrap_or_else(|_| (String::new(), String::new()));
    // Swift is optional too: only a project with a Swift unit needs the compiler.
    // Swift 同样是可选的：只有含 Swift 单元的项目才需要编译器。
    let (swift_program, swift_version) = pick("XIRANG_SWIFT", &["swiftc"], "Swift")
        .unwrap_or_else(|_| (String::new(), String::new()));
    let java_home = java_program
        .rsplit_once("/bin/")
        .map(|(home, _)| home.to_owned())
        .unwrap_or_default();
    // Node.js is optional too: only a project with a JavaScript unit needs the runtime.
    // Node.js 同样是可选的：只有含 JavaScript 单元的项目才需要运行时。
    let (node_program, node_version) = pick("XIRANG_NODE", &["node"], "Node.js")
        .unwrap_or_else(|_| (String::new(), String::new()));
    let node_include = if node_program.is_empty() {
        None
    } else {
        super::project::node_include(&node_program)
    };
    Ok(Compiler {
        program,
        version,
        c_program,
        c_version,
        python_program,
        python_version,
        rust_program,
        rust_version,
        go_program,
        go_version,
        ruby_program,
        ruby_version,
        csharp_program,
        csharp_version,
        java_program,
        java_version,
        java_home,
        php_program,
        php_version,
        kotlin_program,
        kotlin_version,
        swift_program,
        swift_version,
        node_program,
        node_version,
        node_include,
    })
}

/// Find the first compiler in `defaults` that answers `--version`, or the one `variable` names.
/// 在 `defaults` 里找到第一个能回答 `--version` 的编译器，或 `variable` 点名的那个。
fn pick(variable: &str, defaults: &[&str], what: &str) -> Result<(String, String), String> {
    pick_with(variable, defaults, what, "--version")
}

/// Pick a toolchain whose version is printed by `flag`: `--version` for most compilers, `version`
/// for `go`, which rejects the double-dash spelling.
/// 挑选一个用 `flag` 打印版本的工具链：多数编译器用 `--version`，而 `go` 用 `version`（它拒绝双短横写法）。
fn pick_with(
    variable: &str,
    defaults: &[&str],
    what: &str,
    flag: &str,
) -> Result<(String, String), String> {
    let candidates: Vec<String> = match std::env::var(variable).ok() {
        Some(program) => vec![program],
        None => defaults
            .iter()
            .map(|program| (*program).to_owned())
            .collect(),
    };
    let mut missing = Vec::new();
    for program in candidates {
        match Command::new(&program).arg(flag).output() {
            Ok(output) if output.status.success() => {
                let text = String::from_utf8_lossy(&output.stdout);
                let version = text.lines().next().unwrap_or("unknown").trim().to_owned();
                if version.is_empty() {
                    missing.push(format!("{program} printed no version"));
                    continue;
                }
                return Ok((program, version));
            }
            Ok(output) => missing.push(format!("{program} exited {}", output.status)),
            Err(error) => missing.push(format!("{program}: {error}")),
        }
    }
    Err(format!(
        "no {what} compiler was found ({}); install one, or name it with {variable}",
        missing.join("; ")
    ))
}

/// One executed action, as the evidence records it.
/// 一条已执行动作，按证据记录的形状。
pub(crate) struct Ran {
    pub(crate) kind: &'static str,
    pub(crate) argv: Vec<String>,
    pub(crate) status: i32,
    pub(crate) seconds: f64,
    pub(crate) stderr_tail: String,
    pub(crate) reused: bool,
    pub(crate) restored: bool,
}

/// What a run produced: the actions that ran, the test verdicts and the failure, if any.
/// 一次运行产出什么：跑过的动作、测试判定，以及失败（若有）。
pub(crate) struct Outcome {
    pub(crate) ran: Vec<Ran>,
    pub(crate) tests: Vec<(String, bool)>,
    pub(crate) failure: Option<String>,
    pub(crate) reuse_errors: Vec<String>,
}

impl Outcome {
    /// Whether every action succeeded.
    /// 每条动作是否都成功。
    pub(crate) fn passed(&self) -> bool {
        self.failure.is_none()
    }
}

/// Run a plan from start to finish. A compile, archive or link failure stops the run; a failing
/// test does not, because the other tests still say something.
/// 从头到尾运行一份计划。编译、归档或链接失败会停止本次运行；测试失败不会，因为其余测试仍然说明问题。
pub(crate) fn execute(plan: &Plan) -> Result<Outcome, String> {
    std::fs::create_dir_all(&plan.directory).map_err(|error| {
        format!(
            "cannot create the build directory {}: {error}",
            plan.directory.display()
        )
    })?;
    for (path, text) in &plan.generated {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|error| format!("cannot create {}: {error}", parent.display()))?;
        }
        std::fs::write(path, text).map_err(|error| {
            format!(
                "cannot write the generated file {}: {error}",
                path.display()
            )
        })?;
    }
    let mut outcome = Outcome {
        ran: Vec::new(),
        tests: Vec::new(),
        failure: None,
        reuse_errors: Vec::new(),
    };
    for (index, action) in plan.actions.iter().enumerate() {
        let decision = if action.kind == ActionKind::Test {
            reuse::Decision::Run
        } else {
            reuse::attempt(plan, index, &action.outputs)
        };
        if decision != reuse::Decision::Run {
            outcome.ran.push(Ran {
                kind: action.kind.label(),
                argv: action.command.clone(),
                status: 0,
                seconds: 0.0,
                stderr_tail: String::new(),
                reused: true,
                restored: decision == reuse::Decision::Restored,
            });
            continue;
        }
        if let Some(parent) = action.outputs.first().and_then(|output| output.parent()) {
            std::fs::create_dir_all(parent)
                .map_err(|error| format!("cannot create {}: {error}", parent.display()))?;
        }
        let started = Instant::now();
        let mut command = match &plan.sandbox {
            // The plan asked for a write domain: the command runs as a child of this build that
            // confines itself first, so the restriction covers the command and everything it
            // starts without confining this process (Landlock is one-way).
            // 计划要求写域：命令作为本构建的子进程运行，子进程先把自己关进写域，于是限制覆盖该命令及其启动的一切，
            // 而不会关住本进程（Landlock 是单向的）。
            Some(domain) => {
                let executable = std::env::current_exe().map_err(|error| {
                    format!(
                        "cannot locate this executable to start `{}` in a write domain: {error}",
                        action.command[0]
                    )
                })?;
                let mut command = Command::new(executable);
                command
                    .arg("__confine")
                    .arg(domain)
                    .arg("--")
                    .args(&action.command);
                command
            }
            None => {
                let mut command = Command::new(&action.command[0]);
                command.args(&action.command[1..]);
                command
            }
        };
        let output = command
            .current_dir(&action.cwd)
            .envs(action.environment.iter().map(|(name, value)| (name, value)))
            .output()
            .map_err(|error| format!("cannot run `{}`: {error}", action.command[0]))?;
        let seconds = started.elapsed().as_secs_f64();
        let stderr = String::from_utf8_lossy(&output.stderr);
        let status = output.status.code().unwrap_or(-1);
        outcome.ran.push(Ran {
            kind: action.kind.label(),
            argv: action.command.clone(),
            status,
            seconds,
            stderr_tail: tail(&stderr, 20),
            reused: false,
            restored: false,
        });
        if action.kind == ActionKind::Test {
            let passed = output.status.success();
            let name = action.test.clone().unwrap_or_else(|| "test".to_owned());
            outcome.tests.push((name, passed));
            if !passed {
                outcome.failure = Some(format!(
                    "test `{}` failed ({})\n{}",
                    action.test.as_deref().unwrap_or("test"),
                    output.status,
                    tail(&stderr, 12)
                ));
            }
            continue;
        }
        if !output.status.success() {
            outcome.failure = Some(format!(
                "{} failed ({}): {}\n{}",
                action.kind.label(),
                output.status,
                action.command.join(" "),
                tail(&stderr, 20)
            ));
            break;
        }
        if !action.outputs.is_empty()
            && let Err(error) = reuse::record(plan, index, &action.outputs)
        {
            outcome.reuse_errors.push(error);
        }
    }
    Ok(outcome)
}

/// The last `lines` lines of a stream, so evidence stays readable.
/// 一段输出的最后 `lines` 行，让证据保持可读。
fn tail(text: &str, lines: usize) -> String {
    let collected: Vec<&str> = text.lines().collect();
    let start = collected.len().saturating_sub(lines);
    collected[start..].join("\n")
}

/// The evidence document: plan identity, compiler identity, every action and every test.
/// 证据文档：计划身份、编译器身份、每条动作与每个测试。
pub(crate) fn evidence(plan: &Plan, compiler: &Compiler, outcome: &Outcome) -> serde_json::Value {
    // Counters are derived from the actions and tests the run actually recorded, so a summary can
    // add up runs without re-deriving "did it fail" from prose. Failed **actions** are counted
    // beside failed tests: a build that fails to compile has no failing test to count, and a
    // summary that only counts tests would report it as "failed, 0 failed test(s)".
    // 计数器从本次运行真正记录的动作与测试导出，汇总可以相加，不必从散文里反推“是不是失败”。
    // 失败**动作**与失败测试并列计数：编译失败的构建没有失败测试，只数测试的汇总会把它报成
    // “failed, 0 failed test(s)”。
    let reused = outcome
        .ran
        .iter()
        .filter(|ran| ran.reused && !ran.restored)
        .count();
    let restored = outcome.ran.iter().filter(|ran| ran.restored).count();
    let executed = outcome.ran.len().saturating_sub(reused + restored);
    let executed_failed = outcome
        .ran
        .iter()
        .filter(|ran| !ran.reused && !ran.restored && ran.status != 0)
        .count();
    let failed_actions: Vec<serde_json::Value> = outcome
        .ran
        .iter()
        .filter(|ran| !ran.reused && !ran.restored && ran.status != 0)
        .map(|ran| {
            serde_json::json!({
                "kind": ran.kind,
                "argv": ran.argv,
                "status": ran.status,
                "stderr_tail": ran.stderr_tail,
            })
        })
        .collect();
    let tests_failed = outcome.tests.iter().filter(|(_, passed)| !passed).count();
    let tests_passed = outcome.tests.len().saturating_sub(tests_failed);
    let mut document = serde_json::json!({
        "plan": plan.id,
        "directory": plan.directory.display().to_string(),
        "actions": outcome.ran.iter().map(|ran| serde_json::json!({
            "kind": ran.kind,
            "argv": ran.argv,
            "status": ran.status,
            "seconds": ran.seconds,
            "reused": ran.reused,
            "restored": ran.restored,
            "stderr_tail": ran.stderr_tail,
        })).collect::<Vec<_>>(),
        "reuse_errors": outcome.reuse_errors,
        "tests": outcome.tests.iter().map(|(name, passed)| serde_json::json!({
            "name": name,
            "passed": passed,
        })).collect::<Vec<_>>(),
        "counts": {
            "actions": {
                "executed": executed,
                "passed": executed.saturating_sub(executed_failed),
                "failed": executed_failed,
                "reused": reused,
                "restored": restored,
            },
            "tests": { "passed": tests_passed, "failed": tests_failed },
        },
        "failed_actions": failed_actions,
        "result": if outcome.passed() { "passed" } else { "failed" },
    });
    if plan.backend.is_none() {
        document["compiler"] = serde_json::json!({
            "program": compiler.program,
            "version": compiler.version,
            "c_program": compiler.c_program,
            "c_version": compiler.c_version,
            "python_program": compiler.python_program,
            "python_version": compiler.python_version,
            "rust_program": compiler.rust_program,
            "rust_version": compiler.rust_version,
            "go_program": compiler.go_program,
            "go_version": compiler.go_version,
            "ruby_program": compiler.ruby_program,
            "ruby_version": compiler.ruby_version,
            "csharp_program": compiler.csharp_program,
            "java_program": compiler.java_program,
            "php_program": compiler.php_program,
            "php_version": compiler.php_version,
            "java_version": compiler.java_version,
            "java_home": compiler.java_home,
            "kotlin_program": compiler.kotlin_program,
            "kotlin_version": compiler.kotlin_version,
            "swift_program": compiler.swift_program,
            "swift_version": compiler.swift_version,
            "csharp_version": compiler.csharp_version,
        });
    }
    if let Some(backend) = &plan.backend {
        document["backend"] = serde_json::json!({
            "language": backend.language,
            "name": backend.name,
            "program": backend.program,
            "program_digest": backend.program_digest,
            "inputs": backend.inputs,
            "protocol": 2,
        });
    }
    document
}

/// The path the evidence document is written to.
/// 证据文档写到哪里。
pub(crate) fn evidence_path(plan: &Plan) -> PathBuf {
    plan.directory.join("evidence.json")
}

/// Write the evidence document, and return its path.
/// 写出证据文档并返回它的路径。
pub(crate) fn write_evidence(
    plan: &Plan,
    name: &str,
    document: &serde_json::Value,
) -> Result<PathBuf, String> {
    let path = evidence_path(plan);
    let text = serde_json::to_string_pretty(document)
        .map_err(|error| format!("cannot render {name} evidence: {error}"))?;
    std::fs::write(&path, format!("{text}\n"))
        .map_err(|error| format!("cannot write {}: {error}", path.display()))?;
    Ok(path)
}

/// Load, plan and execute a project in one step; shared by `build` and `test`.
/// The lock is checked after resolution and before any compile action, so a `--locked` refusal
/// costs nothing but the resolution itself.
/// 一步完成加载、计划与执行；`build` 与 `test` 共用。锁在解析之后、任何编译动作之前检查，因此
/// `--locked` 的一次拒绝只花掉解析本身的代价。
pub(crate) fn run(
    root: &Path,
    tests: bool,
    locked: bool,
) -> Result<(Project, Plan, Compiler, Outcome, Option<String>), String> {
    let compiler = detect()?;
    let project = Project::load(root)?;
    let lock = super::lock::verify(&project, locked)?;
    let plan = project.plan(&compiler, tests)?;
    let outcome = execute(&plan)?;
    Ok((project, plan, compiler, outcome, lock))
}
