//! Managed execution: the one place this workspace restricts **the process it starts**, instead of describing what
//! it hopes the caller will not do.
//! 托管执行：本工作区唯一一处**限制它启动的进程**，而不是描述"希望调用方不要做什么"的地方。
//!
//! The ruling (§2) allows the sentence "unauthorized writes are prevented" only where the isolation actually holds,
//! and it forbids the plain bridge from claiming it. This module is the other shape: it names the write domain the
//! child may use, has the **kernel** enforce the boundary, and says which restriction is in force rather than which
//! policy was agreed. The enforcement is Landlock, so the guarantee is not a convention: a write outside the domain
//! returns `EPERM` from the kernel, in the child and in every process it starts.
//! 裁定（§2）只允许在隔离真的成立的地方说出"越权写入已被阻止"，并禁止普通桥声称它。本模块是另一种形状：它点名子进程可用的写域，
//! 由**内核**执行边界，并说出实际生效的是哪一条限制，而不是约定了哪一份政策。执行者是 Landlock，因此这个保证不是约定：写域之外的
//! 写入由内核返回 `EPERM`，在子进程里如此，在它启动的每一个进程里也如此。
//!
//! What this module does **not** do, said in the same breath because it is the half a reader over-reads: it does not
//! identify the caller, it does not stop a process that starts **unrestricted** (the agent's shell, if the host offers
//! one, can run this binary without `--managed`), and it does not publish. The host's confirmation entry runs outside
//! the restriction, which is the point — the thing that publishes is not the thing that was confined.
//! 本模块**不**做的事，与上面同一句里说完，因为那是读者最容易读过头的一半：它不识别调用方，它拦不住一个**不受限**启动的进程
//! （如果宿主提供了 shell，agent 可以不带 `--managed` 直接跑这个二进制），它也不发布。宿主的确认入口在限制之外运行，这正是要点
//! ——发布的那一方，不是被关住的那一方。

use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};

/// Exit code for "this host cannot provide managed mode". Distinct from a refusal by policy (2) and from the
/// child's own failure (which is propagated unchanged), because "I cannot isolate" and "I refuse you" are
/// different answers and a caller that cannot tell them apart will read the first as the second.
/// "本宿主无法提供托管模式"的退出码。与按政策拒绝（2）以及子进程自己的失败（原样传播）区分开，因为"我隔离不了"与"我拒绝你"
/// 是两个不同的回答，分不清它们的调用方会把第一个读成第二个。
const NO_ISOLATION: i32 = 3;

use crate::isolation::restrict;

/// What the two halves agree on: the command to run, and the domain it may write.
/// 两半共同约定的东西：要跑的命令，以及它可写的域。
fn spec(root: &Path, writable: &[PathBuf], command: &[String]) -> Value {
    json!({
        "root": root.display().to_string(),
        "writable": writable.iter().map(|path| path.display().to_string()).collect::<Vec<_>>(),
        "command": command,
    })
}

/// The line a managed run prints before it starts: which guarantee is in force, and which one is not.
/// 托管运行在开始前印的那一行：哪一条保证在生效，哪一条不在。
fn capability_line(abi: i32, writable: &[PathBuf], root: &Path) -> String {
    let paths = writable
        .iter()
        .map(|path| path.display().to_string())
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "capability managed-landlock: ABI {abi}; the command started under `--managed` and every process it starts \
         may write {paths}, and every other path — including {} and the approval store beside it — is refused by the \
         kernel (EPERM), not by a policy this reply asks you to trust. The boundary follows the launch chain: this \
         bridge, the host and any process started outside the wrapper keep their own rights, which is why publishing \
         runs outside the domain. It is a write-domain restriction, not an identity check",
        root.display()
    )
}

/// Run a command inside a write domain the kernel enforces.
/// 在内核执行的写域里运行一条命令。
///
/// The restriction is applied by a **child** of this process, not by this one: Landlock is one-way and applies to the
/// caller for the rest of its life, so a bridge that confined itself would confine the bridge. The parent is only a
/// launcher — it parses, spawns, and propagates the status — which is also why a test can call this function without
/// ending up confined.
/// 限制由本进程的**子进程**施加，而不是本进程：Landlock 是单向的，并且对调用方余下的整个生命周期生效，因此一个把自己关起来的
/// 桥会关住这个桥。父进程只是启动器——解析、派生、传播状态——这也是测试能调用它而不会把自己关住的原因。
pub(super) fn main(arguments: &[String]) -> i32 {
    let arguments = match arguments.first().map(String::as_str) {
        Some("--managed") => &arguments[1..],
        _ => arguments,
    };
    if arguments.first().map(String::as_str) == Some("--managed-exec") {
        return match arguments
            .get(1)
            .map(|text| serde_json::from_str::<Value>(text))
        {
            Some(Ok(spec)) => exec_mode(&spec),
            _ => {
                eprintln!(
                    "--managed-exec takes one JSON object; it is not an interface for callers"
                );
                2
            }
        };
    }
    let Some((root, writable, command)) = parse_arguments(arguments) else {
        return 2;
    };
    let spec = spec(&root, &writable, &command).to_string();
    let Ok(executable) = std::env::current_exe() else {
        eprintln!("cannot find this binary to start the confined half");
        return 127;
    };
    match std::process::Command::new(executable)
        .args(["--managed-exec", &spec])
        .status()
    {
        Ok(status) => status
            .code()
            .unwrap_or_else(|| 128 + status.signal().unwrap_or(0)),
        Err(error) => {
            eprintln!("cannot start the confined half: {error}");
            127
        }
    }
}

/// The confined half: restrict this process, say which guarantee is in force, then become the command.
/// 被关住的那一半：限制本进程、说出哪条保证在生效，然后变成那条命令。
fn exec_mode(spec: &Value) -> i32 {
    let Some(root) = spec["root"].as_str().map(PathBuf::from) else {
        eprintln!("the specification names no root");
        return 2;
    };
    let writable: Vec<PathBuf> = spec["writable"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(PathBuf::from)
        .collect();
    let command: Vec<String> = spec["command"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect();
    if command.is_empty() || writable.is_empty() {
        eprintln!("the specification names no command or no writable path");
        return 2;
    }
    match restrict(&writable) {
        Ok(abi) => {
            println!("{}", capability_line(abi, &writable, &root));
            run_command(&command)
        }
        Err(why) => {
            eprintln!("managed mode is not available: {why}");
            NO_ISOLATION
        }
    }
}

/// Run the child and propagate its status; a child killed by a signal is reported as `128 + n`, the shell's
/// convention, so a caller reading exit codes is not told "0" about a process that never finished.
/// 运行子进程并传播它的状态；被信号杀死的子进程按 shell 的惯例报 `128 + n`，这样读退出码的调用方不会被告知一个从未结束的进程
/// "成功"。
fn run_command(command: &[String]) -> i32 {
    let (program, rest) = command
        .split_first()
        .expect("the caller refuses an empty command");
    match std::process::Command::new(program).args(rest).status() {
        Ok(status) => status
            .code()
            .unwrap_or_else(|| 128 + status.signal().unwrap_or(0)),
        Err(error) => {
            eprintln!("cannot start {program}: {error}");
            127
        }
    }
}

/// Parse `--root <path> [--writable <path>]… -- <command>…`.
/// 解析 `--root <path> [--writable <path>]… -- <command>…`。
fn parse_arguments(arguments: &[String]) -> Option<(PathBuf, Vec<PathBuf>, Vec<String>)> {
    let mut root = None;
    let mut writable: Vec<PathBuf> = Vec::new();
    let mut command = Vec::new();
    let mut index = 0;
    while index < arguments.len() {
        match arguments[index].as_str() {
            "--root" => {
                index += 1;
                root = arguments.get(index).map(PathBuf::from);
            }
            "--writable" => {
                index += 1;
                match arguments.get(index) {
                    Some(path) => writable.push(PathBuf::from(path)),
                    None => {
                        eprintln!("--writable needs a path");
                        return None;
                    }
                }
            }
            "--" => {
                command.extend(arguments[index + 1..].iter().cloned());
                break;
            }
            other => {
                eprintln!(
                    "managed mode takes `--root <path>`, `--writable <path>` (repeatable), then `-- <command>`; it does not take `{other}`"
                );
                return None;
            }
        }
        index += 1;
    }
    let Some(root) = root else {
        eprintln!("managed mode needs `--root <path>`: the tree it must not be able to write");
        return None;
    };
    if command.is_empty() {
        eprintln!("managed mode needs a command after `--`: there is nothing to confine otherwise");
        return None;
    }
    // The candidate copy and the build directory live under `<root>/target/xirang`; naming it by default is what makes
    // the wrapper useful without letting the caller name anything wider than what it asked for.
    // 候选副本与构建目录住在 `<root>/target/xirang` 之下；默认点名它，使包装器无需调用方点名就可用，同时不让调用方点到
    // 比它要求的东西更宽的范围。
    if writable.is_empty() {
        writable.push(root.join("target/xirang"));
    }
    Some((root, writable, command))
}
