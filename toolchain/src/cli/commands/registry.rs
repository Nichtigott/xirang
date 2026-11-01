//! `xirang registry check <directory>` and `xirang registry fetch --index <url> --sha256 <digest>
//! --into <directory>`: validate a registry mirror's shape, or fetch one from a pinned index.
//! `xirang registry check <directory>` 与 `xirang registry fetch --index <url> --sha256 <digest>
//! --into <directory>`：校验注册表镜像的形状，或从钉住摘要的索引取回一份镜像。

use std::io::Write;
use std::path::Path;

/// `xirang registry check <directory>` / `xirang registry fetch …`.
/// `xirang registry check <directory>` / `xirang registry fetch …`。
pub(crate) fn registry(
    arguments: &mut dyn Iterator<Item = String>,
    out: &mut dyn Write,
) -> Result<(), String> {
    let Some(action) = arguments.next() else {
        return Err("`xirang registry` takes `check <directory>` or `fetch --index <url> --sha256 <digest> --into <directory>`".to_owned());
    };
    if action == "check" {
        let directory = arguments
            .next()
            .ok_or_else(|| "`xirang registry check` needs a directory".to_owned())?;
        if let Some(extra) = arguments.next() {
            return Err(format!("unexpected argument `{extra}`"));
        }
        writeln!(
            out,
            "{}",
            crate::multilang::registry_check(Path::new(&directory))?
        )
        .map_err(|error| format!("cannot write: {error}"))
    } else if action == "fetch" {
        let arguments: Vec<String> = arguments.collect();
        let mut index = None;
        let mut digest = None;
        let mut into = None;
        let mut auth_env = None;
        let mut rest = arguments.iter();
        while let Some(argument) = rest.next() {
            match argument.as_str() {
                "--index" => index = rest.next().cloned(),
                "--sha256" => digest = rest.next().cloned(),
                "--into" => into = rest.next().cloned(),
                "--auth-env" => auth_env = rest.next().cloned(),
                other => {
                    return Err(format!(
                        "unknown argument `{other}`; `xirang registry fetch` takes `--index <url> --sha256 <digest> --into <directory> [--auth-env <variable>]"
                    ));
                }
            }
        }
        let index =
            index.ok_or_else(|| "`xirang registry fetch` needs `--index <url>`".to_owned())?;
        let digest =
            digest.ok_or_else(|| "`xirang registry fetch` needs `--sha256 <digest>`".to_owned())?;
        let into =
            into.ok_or_else(|| "`xirang registry fetch` needs `--into <directory>`".to_owned())?;
        let auth = match auth_env {
            Some(name) => {
                let token = std::env::var(&name).map_err(|_| {
                    format!(
                        "`--auth-env {name}` names a variable that is not set; nothing was fetched"
                    )
                })?;
                Some(token)
            }
            None => None,
        };
        writeln!(
            out,
            "{}",
            crate::multilang::registry_fetch(&index, &digest, Path::new(&into), auth.as_deref())?
        )
        .map_err(|error| format!("cannot write: {error}"))
    } else {
        Err(format!(
            "unknown registry action `{action}`; `xirang registry` takes `check <directory>` or `fetch --index <url> --sha256 <digest> --into <directory>`"
        ))
    }
}
