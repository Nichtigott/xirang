//! `xirang languages`: the backends this checkout registers and what each declares.
//! `xirang languages`：本检出注册的后端，以及每个后端声明了什么。

use std::io::Write;

/// Print the registered backends and the languages the task names but this checkout does not.
/// 打印已注册的后端，以及任务点名、但本检出尚未覆盖的语言。
pub(crate) fn languages(out: &mut dyn Write) -> Result<(), String> {
    writeln!(out, "{}", crate::multilang::languages()?)
        .map_err(|error| format!("cannot write: {error}"))
}
