//! `xirang bindings`: render a starter for a consumer of this project's `[[export]]`s, in one
//! supported language. The declarations are the contract; the generator writes scalars and optional
//! scalars, and refuses the text/optional-return/error shapes it cannot honestly guess.
//! `xirang bindings`：为消费本项目的 `[[export]]` 的程序渲染绑定起点。声明即契约；生成器写标量与
//! 可选标量，对 text/可选返回/错误形状按名拒绝。

use std::io::Write;

/// `xirang bindings --language <id> [path]`.
/// `xirang bindings --language <id> [path]`。
pub(crate) fn bindings(
    arguments: &mut dyn Iterator<Item = String>,
    out: &mut dyn Write,
) -> Result<(), String> {
    let mut language = None;
    let mut path = None;
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--language" => {
                let value = arguments.next().ok_or(
                    "`--language` needs a value; `xirang bindings --language python|csharp|go`",
                )?;
                language = Some(value);
            }
            other if other.starts_with('-') => {
                return Err(format!(
                    "unknown argument `{other}`; `xirang bindings` takes `--language <id>` and an \
                     optional path"
                ));
            }
            other => path = Some(other.to_owned()),
        }
    }
    let language = language.ok_or(
        "`xirang bindings` needs `--language <id>`; the languages are python, csharp, go, rust and ruby",
    )?;
    let root = std::path::Path::new(path.as_deref().unwrap_or("."));
    writeln!(out, "{}", crate::multilang::bindings(root, &language)?)
        .map_err(|error| format!("cannot write: {error}"))
}
