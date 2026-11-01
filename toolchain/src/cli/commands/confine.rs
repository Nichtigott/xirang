//! `xirang __confine`: run one command inside a kernel-enforced write domain.
//! `xirang __confine`：在内核强制执行的写域里运行一条命令。
//!
//! This is not a user-facing command; it is the child end of `Plan::sandbox`. The build starts it,
//! and it confines **itself** before exec'ing the real command, so the restriction covers that
//! command and every process it starts while the build process keeps its own rights (Landlock is
//! one-way).
//! 这不是面向用户的命令，而是 `Plan::sandbox` 的子进程一端：构建启动它，它在 `exec` 真正的命令**之前**
//! 把自己关进写域，于是限制覆盖那条命令及其启动的每个进程，而构建进程保留自己的权限（Landlock 是单向的）。

use std::io::Write;
use std::os::unix::process::CommandExt;

/// `xirang __confine <domain> -- <program> [args…]`.
/// `xirang __confine <domain> -- <program> [args…]`。
pub(crate) fn confine(
    arguments: &mut dyn Iterator<Item = String>,
    _out: &mut dyn Write,
) -> Result<(), String> {
    let arguments: Vec<String> = arguments.collect();
    let usage = || "usage: xirang __confine <domain> -- <program> [args…]".to_owned();
    let Some(separator) = arguments.iter().position(|argument| argument == "--") else {
        return Err(usage());
    };
    let domain = arguments
        .get(..separator)
        .and_then(|head| head.first())
        .ok_or_else(usage)?;
    let command = &arguments[separator + 1..];
    let Some(program) = command.first() else {
        return Err(usage());
    };
    crate::isolation::restrict(&[std::path::PathBuf::from(domain)])?;
    let error = std::process::Command::new(program)
        .args(&command[1..])
        .exec();
    Err(format!(
        "cannot run `{program}` inside the write domain: {error}"
    ))
}
