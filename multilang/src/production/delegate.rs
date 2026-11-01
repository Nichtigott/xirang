//! Execution delegation: a recorded backend owns discovery and compilation for an external
//! language, and this executor owns the process boundary, the plan shape and the identity.
//! 执行委派：外部语言的发现与编译由记录里的后端拥有，本执行器拥有进程边界、计划形状与身份。
//!
//! The exchange is one JSON request on stdin and one JSON answer on stdout. Protocol 2 binds an
//! answer to the inputs it was computed from: the request carries an input digest and the options
//! the project wrote, the answer must echo the digest, and the plan's identity includes the
//! backend program's own bytes — an edited backend or an edited source is a different plan.
//! 交换是一次 stdin JSON 请求与一次 stdout JSON 应答。协议 2 把应答绑定到它据以计算的输入：请求带
//! 输入摘要与项目写下的选项，应答必须回显摘要，而计划身份包含后端程序自己的字节——改了后端或改了
//! 源码就是另一份计划。

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use serde_json::Value;

use xirang_kernel::interchange::Capability;

use super::project::{Action, ActionKind, Backend, Plan, Project};
use super::record;

/// The backend protocol this build speaks.
/// 本构建理解的后端协议。
pub(super) const PROTOCOL: u32 = 2;

/// One backend's answer, after it has been checked against the protocol.
/// 一个后端应答通过协议检查之后的样子。
struct Answer {
    name: String,
    inputs: String,
    actions: Vec<Action>,
}

/// What a handshake declares: the backend's name, its capabilities, and how its language lays out
/// sources and tests.
/// 握手声明的东西：后端名字、能力，以及它的语言如何放置源码与测试。
pub(super) struct Probe {
    pub(super) backend: String,
    pub(super) capabilities: Vec<Capability>,
    /// Source extensions, without dots.
    /// 源码扩展名，不带点。
    pub(super) extensions: Vec<String>,
    /// The suffix test files carry before their extension.
    /// 测试文件在扩展名之前带的词尾。
    pub(super) test_suffix: String,
}

/// The directory names neither the input digest nor an inventory descends into.
/// 输入摘要与清点都不进入的目录名。
const SKIPPED: [&str; 6] = [
    "target",
    ".git",
    ".xirang",
    ".codegraph",
    ".direnv",
    "node_modules",
];

impl Project {
    /// The plan for an external-language project: ask the recorded backend, check the answer, and
    /// give the plan an identity that binds the inputs, the options and the backend program.
    /// 外部语言项目的计划：询问记录里的后端、检查应答，并给计划一个绑定输入、选项与后端程序的身份。
    pub(crate) fn external_plan(
        &self,
        tests: bool,
        declared: &super::tree::DeclaredTree,
    ) -> Result<Plan, String> {
        let root = &self.root;
        let unit = self.root_unit();
        let language = unit.manifest.external.clone().ok_or_else(|| {
            "this project names no external language; a plan cannot ask a backend for it".to_owned()
        })?;
        let builtin = super::backends::registered_languages();
        let sources = record::discover(root, &builtin)?;
        record::conflicts(&sources)?;
        let (argv, sandbox, planner_sandbox) = sources
            .iter()
            .flat_map(|(_, record)| record.entries.iter())
            .find(|entry| entry.language == language)
            .map(|entry| (entry.command.clone(), entry.sandbox, entry.planner_sandbox))
            .ok_or_else(|| {
                format!(
                    "no record (the project's or the user's) declares a backend for `{language}`; \
                     this build cannot ask for a plan"
                )
            })?;
        let argv = argv.ok_or_else(|| {
            format!(
                "the record declares `{language}` but names no `command`; a declaration is not an \
                 executor, so there is nothing to ask for a plan"
            )
        })?;
        let inputs = inputs_digest(root)?;
        let action = if tests { "test" } else { "build" };
        let mut options = serde_json::Map::new();
        for (key, value) in &unit.manifest.options {
            options.insert(key.clone(), Value::String(value.clone()));
        }
        let request = serde_json::json!({
            "protocol": PROTOCOL,
            "language": language,
            "action": action,
            "directory": root.canonicalize().unwrap_or_else(|_| root.clone()).display().to_string(),
            "inputs": inputs,
            "options": options,
        });
        let domain = planner_sandbox.then(|| root.join("target/xirang"));
        let answer = exchange(root, &argv, request, domain.as_deref())?;
        let answer = validate_answer(&answer, &language, &argv, tests, &inputs, root)?;
        let program_digest = program_digest(&argv);
        let material = format!(
            "xirang.multilang.delegate.v2\0{language}\0{}\0{program_digest}\0{inputs}\0{action}\0{}",
            answer.name,
            options_text(&unit.manifest.options)
        );
        let id = xirang_kernel::sha256_hex(material.as_bytes())[..16].to_owned();
        let directory = root.join("target/xirang/build").join(&id);
        let actions = answer
            .actions
            .into_iter()
            .map(|mut action| {
                if action.cwd.as_os_str().is_empty() {
                    action.cwd = root.clone();
                }
                action
            })
            .collect();
        let sandbox = sandbox.then_some(directory.clone());
        let mut generated = Vec::new();
        if !declared.is_empty() {
            // The backend's own input digest already covers `declarations/`, so the plan id changed
            // when a document changed; publishing the tree keeps the two plans' evidence alike.
            // 后端自己的输入摘要已覆盖 `declarations/`，因此文档一变计划身份就变；发布这棵树是为了
            // 让两条路径的证据形状一致。
            generated.push((directory.join("registry.json"), declared.json()));
        }
        Ok(Plan {
            id,
            actions,
            directory,
            generated,
            backend: Some(Backend {
                language,
                name: answer.name,
                program: argv,
                program_digest,
                inputs: answer.inputs,
            }),
            sandbox,
        })
    }
}

/// A digest over every file under `root` that is an input to the project, keyed by the path
/// relative to the root, so two checkouts of the same bytes agree.
/// 对 `root` 下每个项目输入文件的摘要，按相对根的路径取键，因此同一批字节的两份检出结果一致。
fn inputs_digest(root: &Path) -> Result<String, String> {
    let mut files = Vec::new();
    visit_input_files(root, root, &mut files)?;
    files.sort();
    let mut text = String::from("xirang.multilang.inputs.v1\0");
    for file in files {
        let bytes = std::fs::read(&file)
            .map_err(|error| format!("cannot read {}: {error}", file.display()))?;
        let relative = file.strip_prefix(root).unwrap_or(&file);
        text.push_str(&format!(
            "{}\0{}\0",
            relative.display(),
            xirang_kernel::sha256_hex(&bytes)
        ));
    }
    Ok(xirang_kernel::sha256_hex(text.as_bytes())[..16].to_owned())
}

/// Collect the files an input digest covers.
/// 收集输入摘要覆盖的文件。
fn visit_input_files(
    root: &Path,
    directory: &Path,
    files: &mut Vec<PathBuf>,
) -> Result<(), String> {
    let entries = std::fs::read_dir(directory)
        .map_err(|error| format!("cannot read {}: {error}", directory.display()))?;
    for entry in entries {
        let entry =
            entry.map_err(|error| format!("cannot read {}: {error}", directory.display()))?;
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') || SKIPPED.contains(&name.as_str()) {
            continue;
        }
        if path.is_dir() {
            if path.join("xirang.toml").is_file() && path != root {
                continue;
            }
            visit_input_files(root, &path, files)?;
        } else if path.is_file() {
            files.push(path);
        }
    }
    Ok(())
}

/// A digest over every argv element that names an existing file: editing the backend is a
/// different plan, and a backend named by an absolute path is bound to those bytes too.
/// 对 argv 中每个存在文件的摘要：编辑后端就是另一份计划，用绝对路径点名的后端也被绑定到那些字节。
fn program_digest(argv: &[String]) -> String {
    let mut text = String::from("xirang.multilang.backend.v1\0");
    for element in argv {
        let path = Path::new(element);
        if path.is_file() {
            let bytes = std::fs::read(path).unwrap_or_default();
            text.push_str(&format!("{}\0", xirang_kernel::sha256_hex(&bytes)));
        } else {
            text.push_str("-\0");
        }
    }
    xirang_kernel::sha256_hex(text.as_bytes())[..16].to_owned()
}

/// The options of an external language table, joined in key order.
/// 外部语言表的选项，按键顺序拼接。
fn options_text(options: &std::collections::BTreeMap<String, String>) -> String {
    options
        .iter()
        .map(|(key, value)| format!("{key}={value}"))
        .collect::<Vec<_>>()
        .join(";")
}

/// Ask a recorded backend for its handshake, and check that it names what its language needs.
/// 向记录里的后端要一次握手，并检查它点名了那门语言需要的东西。
pub(super) fn handshake(
    root: &Path,
    argv: &[String],
    language: &str,
    planner_sandbox: bool,
) -> Result<Probe, String> {
    let request = serde_json::json!({
        "protocol": PROTOCOL,
        "language": language,
        "request": "handshake",
    });
    let domain = planner_sandbox.then(|| root.join("target/xirang"));
    let answer = exchange(root, argv, request, domain.as_deref())?;
    let object = answer
        .as_object()
        .ok_or_else(|| "the recorded backend's handshake is not a JSON object".to_owned())?;
    let allowed = [
        "protocol",
        "language",
        "backend",
        "capabilities",
        "reasons",
        "source_extensions",
        "test_suffix",
    ];
    for key in object.keys() {
        if !allowed.contains(&key.as_str()) {
            return Err(format!(
                "the backend `{}` answered its handshake with an unknown key `{key}`; this \
                 protocol has {}",
                argv.join(" "),
                allowed.join(", ")
            ));
        }
    }
    let protocol = object
        .get("protocol")
        .and_then(Value::as_u64)
        .ok_or_else(|| "the handshake has no whole-number `protocol`".to_owned())?;
    if protocol != u64::from(PROTOCOL) {
        return Err(format!(
            "the backend `{}` answered protocol {protocol}; this build speaks protocol {PROTOCOL}",
            argv.join(" ")
        ));
    }
    let answered = object
        .get("language")
        .and_then(Value::as_str)
        .ok_or_else(|| "the handshake has no `language`".to_owned())?;
    if answered != language {
        return Err(format!(
            "the backend `{}` was asked for `{language}` and answered for `{answered}`",
            argv.join(" ")
        ));
    }
    let backend = object
        .get("backend")
        .and_then(Value::as_str)
        .filter(|name| !name.trim().is_empty())
        .ok_or_else(|| "the handshake has no `backend` name".to_owned())?
        .to_owned();
    let mut capabilities = Vec::new();
    for capability in object
        .get("capabilities")
        .and_then(Value::as_array)
        .ok_or_else(|| "the handshake has no `capabilities` array".to_owned())?
    {
        let word = capability
            .as_str()
            .ok_or_else(|| "the handshake lists a capability that is not a string".to_owned())?;
        capabilities.push(Capability::parse_ability(word).ok_or_else(|| {
            format!("the handshake names a capability `{word}` this build does not know")
        })?);
    }
    let mut extensions = Vec::new();
    for extension in object
        .get("source_extensions")
        .and_then(Value::as_array)
        .ok_or_else(|| "the handshake has no `source_extensions` array; this build cannot discover the language's sources without it".to_owned())?
    {
        let extension = extension
            .as_str()
            .ok_or_else(|| "`source_extensions` holds a value that is not a string".to_owned())?;
        if extension.is_empty()
            || extension.starts_with('.')
            || !extension
                .chars()
                .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_'))
        {
            return Err(format!(
                "`{extension}` is not a source extension: write it without a dot, using letters, \
                 digits, `-` or `_`"
            ));
        }
        extensions.push(extension.to_owned());
    }
    if extensions.is_empty() {
        return Err(
            "`source_extensions` is empty; a backend that discovers nothing cannot own a language"
                .to_owned(),
        );
    }
    let test_suffix = match object.get("test_suffix") {
        Some(value) => {
            let suffix = value
                .as_str()
                .ok_or_else(|| "`test_suffix` is not a string".to_owned())?;
            if !suffix.starts_with('_') || suffix.contains('/') || suffix.contains('.') {
                return Err(format!(
                    "`{suffix}` is not a test suffix: write it starting with `_` and without `/` or `.`"
                ));
            }
            suffix.to_owned()
        }
        None => "_test".to_owned(),
    };
    Ok(Probe {
        backend,
        capabilities,
        extensions,
        test_suffix,
    })
}

/// One JSON request to a recorded backend, and its one JSON answer.
/// 向一个记录的后端发送一次 JSON 请求，读回一次 JSON 应答。
fn exchange(
    root: &Path,
    argv: &[String],
    request: Value,
    domain: Option<&Path>,
) -> Result<Value, String> {
    let spelled = argv.join(" ");
    let mut command = if let Some(domain) = domain {
        std::fs::create_dir_all(domain).map_err(|error| {
            format!(
                "cannot create the planner domain {}: {error}",
                domain.display()
            )
        })?;
        let current = std::env::current_exe()
            .map_err(|error| format!("cannot find this executable: {error}"))?;
        let mut command = Command::new(current);
        command
            .arg("__confine")
            .arg(domain)
            .arg("--")
            .arg(&argv[0])
            .args(&argv[1..]);
        command
    } else {
        let mut command = Command::new(&argv[0]);
        command.args(&argv[1..]);
        command
    };
    command
        .current_dir(root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command
        .spawn()
        .map_err(|error| format!("cannot start the recorded backend `{spelled}`: {error}"))?;
    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(request.to_string().as_bytes())
            .and_then(|()| stdin.write_all(b"\n"))
            .map_err(|error| {
                format!("cannot send the request to the recorded backend `{spelled}`: {error}")
            })?;
    }
    let output = child
        .wait_with_output()
        .map_err(|error| format!("cannot wait for the recorded backend `{spelled}`: {error}"))?;
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !output.status.success() {
        return Err(format!(
            "the recorded backend `{spelled}` exited {} without an answer; stderr tail: {}",
            output.status,
            stderr.lines().rev().take(3).collect::<Vec<_>>().join(" / ")
        ));
    }
    let text = String::from_utf8_lossy(&output.stdout);
    serde_json::from_str(text.trim()).map_err(|error| {
        format!(
            "the recorded backend `{spelled}` did not answer with JSON: {error}; first line: {}",
            text.lines().next().unwrap_or("")
        )
    })
}

/// Check one plan answer against the protocol this build speaks.
/// 按本构建理解的协议检查一份计划应答。
fn validate_answer(
    answer: &Value,
    language: &str,
    argv: &[String],
    tests: bool,
    inputs: &str,
    root: &Path,
) -> Result<Answer, String> {
    let object = answer
        .as_object()
        .ok_or_else(|| "the recorded backend's answer is not a JSON object".to_owned())?;
    let spelled = argv.join(" ");
    let allowed = [
        "protocol",
        "language",
        "backend",
        "capabilities",
        "inputs",
        "actions",
    ];
    for key in object.keys() {
        if !allowed.contains(&key.as_str()) {
            return Err(format!(
                "the recorded backend `{spelled}` answered with an unknown key `{key}`; this \
                 protocol has {}",
                allowed.join(", ")
            ));
        }
    }
    let protocol = object
        .get("protocol")
        .and_then(Value::as_u64)
        .ok_or_else(|| {
            format!("the recorded backend `{spelled}` answered without a whole-number `protocol`")
        })?;
    if protocol != u64::from(PROTOCOL) {
        return Err(format!(
            "the recorded backend `{spelled}` answered with protocol {protocol}; this build \
             speaks protocol {PROTOCOL}"
        ));
    }
    let answered = object
        .get("language")
        .and_then(Value::as_str)
        .ok_or_else(|| format!("the recorded backend `{spelled}` answered without a `language`"))?;
    if answered != language {
        return Err(format!(
            "the recorded backend `{spelled}` was asked for `{language}` and answered for \
             `{answered}`"
        ));
    }
    let echoed = object
        .get("inputs")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            format!(
                "the recorded backend `{spelled}` answered without `inputs`; protocol 2 requires \
                 the digest the request carried, so an answer cannot be reused across inputs"
            )
        })?;
    if echoed != inputs {
        return Err(format!(
            "the recorded backend `{spelled}` echoed inputs `{echoed}` but was asked about \
             `{inputs}`; it answered a different project"
        ));
    }
    let name = object
        .get("backend")
        .and_then(Value::as_str)
        .filter(|name| !name.trim().is_empty())
        .ok_or_else(|| {
            format!("the recorded backend `{spelled}` answered without a `backend` name")
        })?;
    let mut claimed = Vec::new();
    for capability in object
        .get("capabilities")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            format!("the recorded backend `{spelled}` answered without a `capabilities` array")
        })?
    {
        let Some(word) = capability.as_str() else {
            return Err(format!(
                "the recorded backend `{spelled}` lists a capability that is not a string"
            ));
        };
        claimed.push(word.to_owned());
    }
    if !claimed.iter().any(|word| word == "build-test") {
        return Err(format!(
            "the backend `{name}` answered for `{language}` but does not claim `build-test`; an \
             executor that cannot build or test cannot be asked to"
        ));
    }
    let actions = object
        .get("actions")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            format!("the recorded backend `{spelled}` answered without an `actions` array")
        })?;
    if actions.is_empty() {
        return Err(format!(
            "the backend `{name}` answered for `{language}` with no action; there is nothing to run"
        ));
    }
    let mut parsed = Vec::new();
    for (index, action) in actions.iter().enumerate() {
        parsed.push(parse_action(action, index, name, root)?);
    }
    if tests && !parsed.iter().any(|action| action.kind == ActionKind::Test) {
        return Err(format!(
            "the backend `{name}` answered a test run for `{language}` with no test action; \
             nothing was run"
        ));
    }
    Ok(Answer {
        name: name.to_owned(),
        inputs: echoed.to_owned(),
        actions: parsed,
    })
}

/// Check one action of an answer: an optional output list, environment and working directory, so
/// the same plan can be cached, packaged and confined.
/// 检查应答里的一个动作：可选的产物清单、环境与工作目录，使同一份计划能被缓存、打包与隔离。
fn parse_action(
    action: &Value,
    index: usize,
    backend: &str,
    root: &Path,
) -> Result<Action, String> {
    let object = action
        .as_object()
        .ok_or_else(|| format!("action {index} of the backend `{backend}` is not a JSON object"))?;
    let allowed = ["kind", "program", "args", "name", "outputs", "cwd", "env"];
    for key in object.keys() {
        if !allowed.contains(&key.as_str()) {
            return Err(format!(
                "action {index} of the backend `{backend}` names an unknown key `{key}`; this \
                 protocol has {}",
                allowed.join(", ")
            ));
        }
    }
    let kind = match object.get("kind").and_then(Value::as_str) {
        Some("compile") => ActionKind::Compile,
        Some("archive") => ActionKind::Archive,
        Some("link") => ActionKind::Link,
        Some("test") => ActionKind::Test,
        Some(other) => {
            return Err(format!(
                "action {index} of the backend `{backend}` has kind `{other}`; the kinds are \
                 compile, archive, link, test"
            ));
        }
        None => {
            return Err(format!(
                "action {index} of the backend `{backend}` has no `kind`"
            ));
        }
    };
    let program = object
        .get("program")
        .and_then(Value::as_str)
        .filter(|program| !program.is_empty())
        .ok_or_else(|| format!("action {index} of the backend `{backend}` has no `program`"))?;
    let mut command = vec![program.to_owned()];
    if let Some(args) = object.get("args") {
        let args = args.as_array().ok_or_else(|| {
            format!("action {index} of the backend `{backend}` has `args` that are not an array")
        })?;
        for arg in args {
            let Some(arg) = arg.as_str() else {
                return Err(format!(
                    "action {index} of the backend `{backend}` has an argument that is not a string"
                ));
            };
            command.push(arg.to_owned());
        }
    }
    let mut outputs = Vec::new();
    if let Some(list) = object.get("outputs") {
        for output in list.as_array().ok_or_else(|| {
            format!("action {index} of the backend `{backend}` has `outputs` that are not an array")
        })? {
            let Some(output) = output.as_str() else {
                return Err(format!(
                    "action {index} of the backend `{backend}` names an output that is not a path string"
                ));
            };
            outputs.push(absolute(root, output));
        }
    }
    let cwd = match object.get("cwd") {
        Some(value) => {
            let spelled = value.as_str().ok_or_else(|| {
                format!("action {index} of the backend `{backend}` has `cwd` that is not a string")
            })?;
            absolute(root, spelled)
        }
        None => root.to_path_buf(),
    };
    let mut environment = Vec::new();
    if let Some(map) = object.get("env") {
        let map = map.as_object().ok_or_else(|| {
            format!("action {index} of the backend `{backend}` has `env` that is not an object")
        })?;
        for (name, value) in map {
            let value = value.as_str().ok_or_else(|| {
                format!(
                    "action {index} of the backend `{backend}` has an environment value for `{name}` that is not a string"
                )
            })?;
            environment.push((name.clone(), value.to_owned()));
        }
        environment.sort();
    }
    let name = object
        .get("name")
        .and_then(Value::as_str)
        .map(str::to_owned);
    if name.is_some() && kind != ActionKind::Test {
        return Err(format!(
            "action {index} of the backend `{backend}` names a test (`name`) but its kind is not \
             test"
        ));
    }
    let test = if kind == ActionKind::Test {
        Some(name.unwrap_or_else(|| {
            Path::new(program)
                .file_stem()
                .and_then(|stem| stem.to_str())
                .unwrap_or("backend")
                .to_owned()
        }))
    } else {
        None
    };
    Ok(Action {
        kind,
        command,
        cwd,
        outputs,
        test,
        environment,
    })
}

/// Resolve a path an answer writes: absolute stays, relative is rooted at the project.
/// 解析应答写下的路径：绝对路径保留，相对路径以项目根为基准。
fn absolute(root: &Path, spelled: &str) -> PathBuf {
    let path = Path::new(spelled);
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        root.join(path)
    }
}
