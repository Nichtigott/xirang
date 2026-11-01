//! Project inventory and the build plan for the first multilingual slice.
//! 第一块多语言竖切的项目清点与构建计划。
//!
//! The inventory is the part a user must not maintain by hand: `src/` supplies compilation units,
//! `include/` supplies public headers, `tests/*_test.cpp` supplies one program per test, and a
//! `path` dependency contributes its own include directory and archive. The plan is a value: the
//! local executor, a later cache and a later remote executor all consume the same actions.
//! 清点是用户不必手工维护的那一半：`src/` 提供编译单元，`include/` 提供公开头文件，
//! `tests/*_test.cpp` 每个测试一个程序，`path` 依赖贡献自己的 include 目录与归档。计划是一个值：
//! 本地执行器、将来的缓存与远端执行器消费同一组动作。

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

mod discovery;
#[path = "generate.rs"]
mod generate;
mod plan;
mod plan_consumers;
#[path = "plan_java.rs"]
mod plan_java;
#[path = "plan_javascript.rs"]
mod plan_javascript;
#[path = "plan_kotlin.rs"]
mod plan_kotlin;
#[path = "plan_language.rs"]
mod plan_language;
#[path = "plan_native.rs"]
mod plan_native;
#[path = "plan_php.rs"]
mod plan_php;
#[path = "plan_swift.rs"]
mod plan_swift;

use super::manifest::Manifest;

/// A source-language compiler with the identity the plan records.
/// 一个源码语言编译器，带计划要记录的身份。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Compiler {
    pub(crate) program: String,
    pub(crate) version: String,
    /// The C compiler, empty when this machine has none.
    /// C 编译器；本机没有时为空。
    pub(crate) c_program: String,
    pub(crate) c_version: String,
    /// The Python interpreter, empty when this machine has none.
    /// Python 解释器；本机没有时为空。
    pub(crate) python_program: String,
    pub(crate) python_version: String,
    /// The Rust compiler, empty when this machine has none.
    /// Rust 编译器；本机没有时为空。
    pub(crate) rust_program: String,
    pub(crate) rust_version: String,
    /// The Go toolchain, empty when this machine has none.
    /// Go 工具链；本机没有时为空。
    pub(crate) go_program: String,
    pub(crate) go_version: String,
    /// The Ruby interpreter, empty when this machine has none.
    /// Ruby 解释器；本机没有时为空。
    pub(crate) ruby_program: String,
    pub(crate) ruby_version: String,
    /// The .NET SDK, empty when this machine has none.
    /// .NET SDK；本机没有时为空。
    pub(crate) csharp_program: String,
    pub(crate) csharp_version: String,
    /// The JDK's `javac`, empty when this machine has none.
    /// JDK 的 `javac`；本机没有时为空。
    pub(crate) java_program: String,
    pub(crate) java_version: String,
    /// The PHP interpreter, empty when this machine has none.
    /// PHP 解释器；本机没有时为空。
    pub(crate) php_program: String,
    pub(crate) php_version: String,
    /// The JDK home (the parent of `bin/`), empty when there is none.
    /// JDK 根目录（`bin/` 的父目录）；没有时为空。
    pub(crate) java_home: String,
    /// The Kotlin compiler, empty when this machine has none.
    /// Kotlin 编译器；本机没有时为空。
    pub(crate) kotlin_program: String,
    pub(crate) kotlin_version: String,
    pub(crate) swift_program: String,
    pub(crate) swift_version: String,
    /// The Node.js runtime, empty when this machine has none.
    /// Node.js 运行时；本机没有时为空。
    pub(crate) node_program: String,
    pub(crate) node_version: String,
    /// The Node headers used to build a JavaScript unit's C addon, when they were found.
    /// 构建 JavaScript 单元 C 插件所用的 Node 头文件目录（找到时）。
    pub(crate) node_include: Option<String>,
}

/// One packaged unit: the root project or a `path` dependency.
/// 一个参与构建的单元：根项目或一个 `path` 依赖。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Unit {
    pub(crate) manifest: Manifest,
    pub(crate) directory: PathBuf,
    pub(crate) sources: Vec<PathBuf>,
    pub(crate) tests: Vec<PathBuf>,
    pub(crate) include: Option<PathBuf>,
    /// The `[sources]` name a dependency came from, when it did not come from a path.
    /// 依赖来自 `[sources]` 的名字（若它不是来自路径）。
    pub(crate) source: Option<String>,
    /// The name this unit's artefacts are keyed on: its package name when that name is unique in
    /// the project, and a location-suffixed form when two units share the name.
    /// 本单元产物的取键名：包名在项目里唯一时就是包名，两个单元同名时是带位置后缀的形式。
    pub(crate) key: String,
}

/// Load the project at `root`, validating every unit and the whole dependency graph.
/// 加载 `root` 处的项目，校验每个单元与整张依赖图。
pub(crate) fn load(root: &Path) -> Result<Project, String> {
    discovery::load(root)
}

/// A loaded project: the root unit first, then its dependencies in declaration order.
/// 一个已加载项目：根单元在前，随后是按声明顺序的依赖。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Project {
    pub(crate) root: PathBuf,
    pub(crate) units: Vec<Unit>,
    /// For each unit, the units its manifest dependencies resolved to (indexes into `units`).
    /// 每个单元的清单依赖解析到的单元（`units` 的下标）。
    pub(crate) edges: Vec<Vec<usize>>,
}

/// What an action produces, so a later cache can key on the output definition.
/// 动作产出什么，供将来的缓存按输出定义取键。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ActionKind {
    Compile,
    Archive,
    Link,
    Test,
}

impl ActionKind {
    /// The stable spelling recorded in evidence.
    /// 记录进证据的稳定拼写。
    pub(crate) fn label(self) -> &'static str {
        match self {
            ActionKind::Compile => "compile",
            ActionKind::Archive => "archive",
            ActionKind::Link => "link",
            ActionKind::Test => "test",
        }
    }
}

/// One closed action: everything needed to run it, in order, without a shell.
/// 一个封闭动作：按顺序、不经 shell 就能运行它所需的一切。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Action {
    pub(crate) kind: ActionKind,
    pub(crate) command: Vec<String>,
    pub(crate) cwd: PathBuf,
    pub(crate) outputs: Vec<PathBuf>,
    /// The test program's name, when the action is a test.
    /// 动作是测试时，该测试程序的名字。
    pub(crate) test: Option<String>,
    /// Extra environment the action runs under (a Python test needs its import path).
    /// 动作运行时的额外环境（Python 测试需要它的导入路径）。
    pub(crate) environment: Vec<(String, String)>,
}

/// Which recorded backend produced an external plan, and the identity that makes it keyable.
/// 哪份记录里的后端产生了外部计划，以及让计划可被取键的身份。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Backend {
    /// The language identity the backend answered for.
    /// 该后端回答的语言身份。
    pub(crate) language: String,
    /// The backend's own name, as its answer declared it.
    /// 后端自报的名字。
    pub(crate) name: String,
    /// The argv the record names.
    /// 记录点名的 argv。
    pub(crate) program: Vec<String>,
    /// A digest over every argv element that names an existing file, so editing the backend is a
    /// different plan.
    /// 对 argv 中每个存在文件的摘要：编辑后端就是另一份计划。
    pub(crate) program_digest: String,
    /// The input digest the answer echoed.
    /// 应答回显的输入摘要。
    pub(crate) inputs: String,
}

/// The plan a build or a test run executes.
/// 构建或测试执行的那份计划。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Plan {
    pub(crate) id: String,
    pub(crate) actions: Vec<Action>,
    pub(crate) directory: PathBuf,
    /// Files the plan generates before it runs (the C ABI header and its C++ wrapper).
    /// 计划在运行前生成的文件（C ABI 头文件与它的 C++ 包装器）。
    pub(crate) generated: Vec<(PathBuf, String)>,
    /// The recorded backend that produced this plan, when it came from one.
    /// 产生此计划的记录型后端（若来自它）。
    pub(crate) backend: Option<Backend>,
    /// The only directory this plan's actions may write, when the project asked for a write domain.
    /// 本计划的动作唯一可以写入的目录（当项目要求写域时）。
    pub(crate) sandbox: Option<PathBuf>,
}

/// The environment a Go action runs under: the producers' headers and archives reach cgo through
/// `CGO_CFLAGS`/`CGO_LDFLAGS`, the build cache lives inside the plan's own directory (the user's
/// `$HOME` may be read-only), and `GOTOOLCHAIN=local` keeps an offline run from downloading
/// another toolchain.
/// Go 动作的环境：生产者的头文件与归档经 `CGO_CFLAGS`/`CGO_LDFLAGS` 到达 cgo；构建缓存放在计划
/// 自己目录里（用户的 `$HOME` 可能只读）；`GOTOOLCHAIN=local` 阻止离线运行去下载另一套工具链。
fn go_environment(
    libraries: &[String],
    archives: &BTreeMap<String, PathBuf>,
    export_headers: &Path,
    objects: &Path,
) -> Vec<(String, String)> {
    let mut environment = vec![
        (
            "CGO_CFLAGS".to_owned(),
            format!("-I{}", export_headers.display()),
        ),
        ("GOTOOLCHAIN".to_owned(), "local".to_owned()),
        (
            "GOCACHE".to_owned(),
            objects.join("go-cache").display().to_string(),
        ),
    ];
    let mut ldflags = libraries
        .iter()
        .filter_map(|name| archives.get(name))
        .map(|archive| archive.display().to_string())
        .collect::<Vec<_>>();
    if !ldflags.is_empty() {
        ldflags.push("-lstdc++".to_owned());
        ldflags.push("-lm".to_owned());
        ldflags.push("-ldl".to_owned());
        ldflags.push("-lpthread".to_owned());
    }
    environment.push(("CGO_LDFLAGS".to_owned(), ldflags.join(" ")));
    environment
}

/// The Node N-API header directory, if this machine has one: `XIRANG_NODE_INCLUDE` first, then
/// `<prefix>/include/node` next to the runtime, then the newest `~/.cache/node-gyp/*/include/node`.
/// Node N-API 头文件目录（本机有时）：先 `XIRANG_NODE_INCLUDE`，再运行时旁边的
/// `<prefix>/include/node`，最后 `~/.cache/node-gyp/*/include/node` 里最新的一个。
pub(super) fn node_include(program: &str) -> Option<String> {
    let named = std::env::var("XIRANG_NODE_INCLUDE").ok();
    if let Some(directory) = named {
        let candidate = PathBuf::from(&directory);
        if candidate.join("node_api.h").is_file() {
            return Some(candidate.display().to_string());
        }
    }
    if let Some(prefix) = Path::new(program).parent().and_then(Path::parent) {
        let candidate = prefix.join("include/node");
        if candidate.join("node_api.h").is_file() {
            return Some(candidate.display().to_string());
        }
    }
    let cache = PathBuf::from(std::env::var_os("HOME")?).join(".cache/node-gyp");
    let mut entries: Vec<PathBuf> = std::fs::read_dir(cache)
        .ok()?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .collect();
    entries.sort();
    entries
        .iter()
        .rev()
        .map(|directory| directory.join("include/node"))
        .find(|directory| directory.join("node_api.h").is_file())
        .map(|directory| directory.display().to_string())
}
