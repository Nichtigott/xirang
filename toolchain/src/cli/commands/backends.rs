//! `xirang backends install`: record one backend in the user-level record.
//! `xirang backends install`：把一个后端记录进用户级记录。

use std::io::Write;

/// Dispatch `xirang backends <subcommand>`.
/// 分发 `xirang backends <子命令>`。
pub(crate) fn backends(
    arguments: &mut dyn Iterator<Item = String>,
    out: &mut dyn Write,
) -> Result<(), String> {
    match arguments.next().as_deref() {
        Some("install") => install(arguments, out),
        Some("new") => new_backend(arguments, out),
        Some("remove") => remove(arguments, out),
        Some("check") => check_backends(arguments, out),
        Some(other) => Err(format!(
            "unknown `xirang backends` subcommand `{other}`; the subcommands are `install`, `new`, `remove` and `check`"
        )),
        None => {
            Err("`xirang backends` needs a subcommand: install, new, remove or check".to_owned())
        }
    }
}

/// One value after a named flag.
/// 点名旗标后的一个值。
fn value(arguments: &mut dyn Iterator<Item = String>, flag: &str) -> Result<String, String> {
    arguments
        .next()
        .ok_or_else(|| format!("`{flag}` needs a value"))
}

/// `xirang backends remove`: drop one language's entry from the user record.
/// `xirang backends remove`：从用户记录里删掉一门语言的条目。
fn remove(arguments: &mut dyn Iterator<Item = String>, out: &mut dyn Write) -> Result<(), String> {
    let mut language = None;
    while let Some(flag) = arguments.next() {
        match flag.as_str() {
            "--language" => language = Some(value(arguments, "--language")?),
            other => {
                return Err(format!(
                    "unknown argument `{other}`; `xirang backends remove` takes `--language`"
                ));
            }
        }
    }
    let Some(language) = language else {
        return Err("`xirang backends remove` needs `--language`".to_owned());
    };
    writeln!(out, "{}", crate::multilang::remove_backend(&language)?)
        .map_err(|error| format!("cannot write: {error}"))
}

/// `xirang backends check`: run the conformance kit against one recorded backend.
/// `xirang backends check`：对某个记录后端跑一致性检查。
fn check_backends(
    arguments: &mut dyn Iterator<Item = String>,
    out: &mut dyn Write,
) -> Result<(), String> {
    let mut language = None;
    let mut path = None;
    while let Some(flag) = arguments.next() {
        match flag.as_str() {
            "--language" => language = Some(value(arguments, "--language")?),
            other if other.starts_with('-') => {
                return Err(format!(
                    "unknown argument `{other}`; `xirang backends check` takes `--language` and \
                     an optional path"
                ));
            }
            other => path = Some(other.to_owned()),
        }
    }
    let Some(language) = language else {
        return Err("`xirang backends check` needs `--language`".to_owned());
    };
    let root = std::path::Path::new(path.as_deref().unwrap_or("."));
    writeln!(out, "{}", crate::multilang::check_backend(root, &language)?)
        .map_err(|error| format!("cannot write: {error}"))
}

/// `xirang backends new`: write a runnable backend template and print the install command.
/// `xirang backends new`：写出一个可运行的后端模板并打印安装命令。
fn new_backend(
    arguments: &mut dyn Iterator<Item = String>,
    out: &mut dyn Write,
) -> Result<(), String> {
    let mut language = None;
    let mut backend = None;
    let mut directory = None;
    while let Some(flag) = arguments.next() {
        match flag.as_str() {
            "--language" => language = Some(value(arguments, "--language")?),
            "--backend" => backend = Some(value(arguments, "--backend")?),
            "--directory" => directory = Some(value(arguments, "--directory")?),
            other => {
                return Err(format!(
                    "unknown argument `{other}`; `xirang backends new` takes `--language`, \
                     `--backend` and `--directory`"
                ));
            }
        }
    }
    let Some(language) = language else {
        return Err("`xirang backends new` needs `--language`".to_owned());
    };
    let Some(backend) = backend else {
        return Err("`xirang backends new` needs `--backend`".to_owned());
    };
    let Some(directory) = directory else {
        return Err("`xirang backends new` needs `--directory`".to_owned());
    };
    writeln!(
        out,
        "{}",
        crate::multilang::scaffold_backend(&language, &backend, std::path::Path::new(&directory))?
    )
    .map_err(|error| format!("cannot write: {error}"))
}

/// `xirang backends install`: parse the flags and record what the process answers for.
/// `xirang backends install`：解析旗标并记录该进程作答的后端。
fn install(arguments: &mut dyn Iterator<Item = String>, out: &mut dyn Write) -> Result<(), String> {
    let mut language = None;
    let mut backend = None;
    let mut command = Vec::new();
    let mut capabilities = Vec::new();
    let mut reasons = Vec::new();
    while let Some(flag) = arguments.next() {
        match flag.as_str() {
            "--language" => language = Some(value(arguments, "--language")?),
            "--backend" => backend = Some(value(arguments, "--backend")?),
            "--command" => command.push(value(arguments, "--command")?),
            "--capability" => capabilities.push(value(arguments, "--capability")?),
            "--reason" => {
                let text = value(arguments, "--reason")?;
                let (word, reason) = text.split_once('=').ok_or_else(|| {
                    "`--reason` takes `<capability>=<text>`; the `=` is what separates them"
                        .to_owned()
                })?;
                reasons.push((word.to_owned(), reason.to_owned()));
            }
            other => {
                return Err(format!(
                    "unknown argument `{other}`; `xirang backends install` takes `--language`, \
                     `--backend`, `--command`, `--capability` and `--reason`"
                ));
            }
        }
    }
    let Some(language) = language else {
        return Err("`xirang backends install` needs `--language`".to_owned());
    };
    let Some(backend) = backend else {
        return Err("`xirang backends install` needs `--backend`".to_owned());
    };
    let installed = crate::multilang::install_backend(
        std::path::Path::new("."),
        crate::multilang::BackendInstall {
            language,
            backend,
            command,
            capabilities,
            reasons,
        },
    )?;
    writeln!(out, "{installed}").map_err(|error| format!("cannot write: {error}"))
}
