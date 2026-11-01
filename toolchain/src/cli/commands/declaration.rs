//! `xirang declaration`: the declaration interchange a non-Rust program can emit. `check`
//! normalises one document; `new` writes a runnable starter for one face.
//! `xirang declaration`：非 Rust 程序可以发出的声明交换。`check` 规范化一份文档；`new` 为一个面写出
//! 可直接运行的起点程序。

use std::io::Write;
use std::path::Path;

/// `xirang declaration check <file>` or `xirang declaration new --language <id> --name <face>
/// --directory <dir> [--kind <kind>] [--contract <path>]…`.
/// `xirang declaration check <file>` 或 `xirang declaration new …`。
pub(crate) fn declaration(
    arguments: &mut dyn Iterator<Item = String>,
    out: &mut dyn Write,
) -> Result<(), String> {
    let Some(action) = arguments.next() else {
        return Err(
            "`xirang declaration` takes `check <file>`, `tree [dir] [--json]` or `new --language \
             <id> --name <face> --directory <dir> [--kind <kind>] [--contract <path>]…"
                .to_owned(),
        );
    };
    match action.as_str() {
        "check" => {
            let file = arguments
                .next()
                .ok_or_else(|| "`xirang declaration check` needs a file".to_owned())?;
            let canonical = crate::multilang::declaration_check(Path::new(&file))?;
            writeln!(out, "{canonical}").map_err(|error| format!("cannot write: {error}"))
        }
        "tree" => {
            let mut root = None;
            let mut as_json = false;
            for argument in arguments {
                match argument.as_str() {
                    "--json" => as_json = true,
                    other if other.starts_with("--") => {
                        return Err(format!(
                            "`xirang declaration tree` takes an optional directory and `--json`; \
                             it does not take `{other}`"
                        ));
                    }
                    other => {
                        if root.is_some() {
                            return Err("`xirang declaration tree` names one directory".to_owned());
                        }
                        root = Some(other.to_owned());
                    }
                }
            }
            let root = root.unwrap_or_else(|| ".".to_owned());
            let tree = crate::multilang::declared_tree(Path::new(&root), as_json)?;
            write!(out, "{tree}").map_err(|error| format!("cannot write: {error}"))
        }
        "new" => {
            let mut language = None;
            let mut name = None;
            let mut kind = None;
            let mut directory = None;
            let mut contracts = Vec::new();
            let mut preset = None;
            let mut parts = Vec::new();
            let mut requires = Vec::new();
            let mut provides = Vec::new();
            let mut exports = Vec::new();
            let mut stable_name = None;
            while let Some(argument) = arguments.next() {
                match argument.as_str() {
                    "--language" => language = arguments.next(),
                    "--name" => name = arguments.next(),
                    "--kind" => kind = arguments.next(),
                    "--directory" => directory = arguments.next(),
                    "--stable-name" => stable_name = arguments.next(),
                    "--preset" => preset = arguments.next(),
                    "--contract" => match arguments.next() {
                        Some(contract) => contracts.push(contract),
                        None => return Err("`--contract` needs a path".to_owned()),
                    },
                    "--part" => match arguments.next() {
                        Some(part) => parts.push(part),
                        None => return Err("`--part` needs a type path".to_owned()),
                    },
                    "--provides" => match arguments.next() {
                        Some(provided) => provides.push(provided),
                        None => return Err("`--provides` needs a name".to_owned()),
                    },
                    "--exports" => match arguments.next() {
                        Some(exported) => exports.push(exported),
                        None => return Err("`--exports` needs a name".to_owned()),
                    },
                    "--requires" => match arguments.next() {
                        Some(pair) => {
                            let Some((capability, provider)) = pair.split_once('=') else {
                                return Err(format!(
                                    "`--requires` takes `<capability>=<provider>`; `{pair}` has no `=`"
                                ));
                            };
                            requires.push((capability.to_owned(), provider.to_owned()));
                        }
                        None => {
                            return Err("`--requires` needs `<capability>=<provider>`".to_owned());
                        }
                    },
                    other => {
                        return Err(format!(
                            "unknown argument `{other}`; `xirang declaration new` takes \
                             `--language`, `--name`, `--kind`, `--directory`, `--contract`, \
                             `--preset`, `--part`, `--requires`, `--provides`, `--exports` and \
                             `--stable-name`"
                        ));
                    }
                }
            }
            let language = language.ok_or_else(|| "`--language <id>` is required".to_owned())?;
            let name = name.ok_or_else(|| "`--name <face>` is required".to_owned())?;
            let kind = kind.unwrap_or_else(|| name.clone());
            let directory =
                directory.ok_or_else(|| "`--directory <dir>` is required".to_owned())?;
            if contracts.is_empty() {
                contracts.push("control::ControlHandle".to_owned());
            }
            let spec = crate::multilang::DeclarationSpec {
                language,
                name,
                kind,
                contracts,
                preset,
                parts,
                requires,
                provides,
                exports,
                stable_name,
            };
            let written = crate::multilang::declaration_new(&spec, Path::new(&directory))?;
            writeln!(out, "{written}").map_err(|error| format!("cannot write: {error}"))
        }
        other => Err(format!(
            "unknown declaration action `{other}`; `xirang declaration` takes `check`, `tree` \
             and `new`"
        )),
    }
}
