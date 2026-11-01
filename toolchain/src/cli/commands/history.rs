//! `xirang history`: report the generations an install prefix holds, verifying each against its
//! manifest. Read-only; a generation whose bytes drifted is reported and makes the command fail.
//! `xirang history`：报告安装前缀里的世代，并按各自的清单校验。只读；字节漂移的世代会被点名并让命令失败。

use std::io::Write;

/// `xirang history --prefix <dir>`.
/// `xirang history --prefix <dir>`。
pub(crate) fn history(
    arguments: &mut dyn Iterator<Item = String>,
    out: &mut dyn Write,
) -> Result<(), String> {
    let mut prefix = None;
    let mut arguments = arguments.peekable();
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--prefix" => {
                prefix = Some(
                    arguments
                        .next()
                        .ok_or_else(|| "`--prefix` needs a directory".to_owned())?,
                );
            }
            other => {
                return Err(format!(
                    "unknown argument `{other}`; `xirang history` takes `--prefix <dir>`"
                ));
            }
        }
    }
    let prefix = prefix.ok_or_else(|| "`xirang history` needs `--prefix <dir>`".to_owned())?;
    writeln!(
        out,
        "{}",
        crate::multilang::history(std::path::Path::new(&prefix))?
    )
    .map_err(|error| format!("cannot write: {error}"))
}
