//! `xirang sign <directory> --key <private-key>`: sign a packaged directory's manifest with an SSH
//! key, writing `manifest.json.sig` beside it. The manifest binds every packaged file by sha256,
//! so the signature covers the files transitively; verification is `xirang verify --signers …`.
//! `xirang sign <directory> --key <private-key>`：用 SSH 密钥给打包目录的清单签名，写在旁边的
//! `manifest.json.sig`。清单用 sha256 绑定每个打包文件，因此签名间接覆盖全部文件；核验用
//! `xirang verify --signers …`。

use std::io::Write;

/// `xirang sign <directory> --key <private-key>`.
/// `xirang sign <directory> --key <private-key>`。
pub(crate) fn sign(
    arguments: &mut dyn Iterator<Item = String>,
    out: &mut dyn Write,
) -> Result<(), String> {
    let mut directory = None;
    let mut key = None;
    let mut principal = None;
    let mut pending = None::<&str>;
    for argument in arguments {
        if let Some(flag) = pending.take() {
            match flag {
                "--key" => key = Some(argument),
                "--principal" => principal = Some(argument),
                _ => return Err(format!("unknown argument `{flag}`")),
            }
            continue;
        }
        match argument.as_str() {
            "--key" => pending = Some("--key"),
            "--principal" => pending = Some("--principal"),
            other if other.starts_with('-') => {
                return Err(format!(
                    "unknown argument `{other}`; `xirang sign` takes a directory, `--key <file>` and optionally `--principal <name>`"
                ));
            }
            other => {
                if directory.replace(other.to_owned()).is_some() {
                    return Err("`xirang sign` takes one packaged directory".to_owned());
                }
            }
        }
    }
    if pending.is_some() {
        return Err("`--key` and `--principal` need values".to_owned());
    }
    let directory = directory.ok_or_else(|| {
        "`xirang sign` needs the packaged directory printed by `xirang package`".to_owned()
    })?;
    let key = key.ok_or_else(|| {
        "`xirang sign` needs `--key <file>`: this tool carries no key of its own".to_owned()
    })?;
    writeln!(
        out,
        "{}",
        crate::multilang::sign_package(
            std::path::Path::new(&directory),
            std::path::Path::new(&key),
            principal.as_deref()
        )?
    )
    .map_err(|error| format!("cannot write: {error}"))
}
