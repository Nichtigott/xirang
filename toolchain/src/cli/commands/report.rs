//! `xirang report`: summarize the economics of one build from the evidence it wrote.
//! `xirang report`：从构建写下的证据汇总一次构建的经济性。

use std::io::Write;

/// `xirang report [--plan <id>] [path]`.
/// `xirang report [--plan <id>] [path]`。
pub(crate) fn report(
    arguments: &mut dyn Iterator<Item = String>,
    out: &mut dyn Write,
) -> Result<(), String> {
    let mut plan = None;
    let mut path = None;
    let mut arguments = arguments.peekable();
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--plan" => {
                let id = arguments
                    .next()
                    .ok_or_else(|| "`--plan` needs a plan id".to_owned())?;
                plan = Some(id);
            }
            other if other.starts_with('-') => {
                return Err(format!(
                    "unknown argument `{other}`; `xirang report` takes `--plan <id>` and an optional path"
                ));
            }
            other => path = Some(other.to_owned()),
        }
    }
    let root = std::path::Path::new(path.as_deref().unwrap_or("."));
    writeln!(out, "{}", crate::multilang::report(root, plan.as_deref())?)
        .map_err(|error| format!("cannot write: {error}"))
}
