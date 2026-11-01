//! `xirang test`: run the test programs of an `xirang.toml` project.
//! `xirang test`：运行 `xirang.toml` 项目的测试程序。
//!
//! The command owns argv only; the plan and the verdicts come from the multilingual surface.
//! 本命令只拥有 argv；计划与判定来自多语言面。

use std::io::Write;
use std::path::PathBuf;

/// Run the project's test programs, or say why the argument list is not one.
/// 运行项目的测试程序，或说明这串参数为何不成其为一次调用。
pub(crate) fn test(
    args: &mut impl Iterator<Item = String>,
    out: &mut dyn Write,
) -> Result<(), String> {
    let mut directory: Option<PathBuf> = None;
    let mut locked = false;
    for argument in args {
        if argument == "-h" || argument == "--help" {
            return crate::cli::usage(out);
        }
        if argument == "--locked" {
            locked = true;
            continue;
        }
        if argument.starts_with('-') {
            return Err(format!(
                "unexpected option '{argument}'; usage: xirang test [path] [--locked]"
            ));
        }
        if directory.is_some() {
            return Err(format!(
                "unexpected argument '{argument}'; usage: xirang test [path] [--locked]"
            ));
        }
        directory = Some(PathBuf::from(argument));
    }
    let root = directory.unwrap_or_else(|| PathBuf::from("."));
    let summary = crate::multilang::test(&root, locked)?;
    writeln!(out, "{summary}").map_err(|error| format!("cannot write the test summary: {error}"))
}
