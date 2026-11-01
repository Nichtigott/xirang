//! `xirang verify <directory>`: check a packaged directory against the manifest it carries, without
//! installing it. Every file the manifest names must be present with its recorded sha256, and the
//! directory must hold nothing the manifest does not name.
//! `xirang verify <directory>`：按包自带的清单核验交付目录，不安装。清单点名的每个文件必须在且
//! sha256 相符，目录里不得有清单未点名的文件。

use std::io::Write;

/// `xirang verify <directory>`.
/// `xirang verify <directory>`。
pub(crate) fn verify(
    arguments: &mut dyn Iterator<Item = String>,
    out: &mut dyn Write,
) -> Result<(), String> {
    let mut directory = None;
    let mut signers = None;
    let mut principals: Vec<String> = Vec::new();
    let mut revoked = None;
    let mut required = None;
    let mut policy = None;
    let mut required_weight = None;
    let mut pending = None::<&str>;
    for argument in arguments {
        if let Some(flag) = pending.take() {
            match flag {
                "--signers" => signers = Some(argument),
                "--principal" => principals.push(argument),
                "--revoked" => revoked = Some(argument),
                "--require" => {
                    required = Some(
                        argument
                            .parse::<usize>()
                            .map_err(|_| format!("`--require {argument}` is not a count"))?,
                    );
                }
                "--policy" => policy = Some(argument),
                "--require-weight" => {
                    required_weight =
                        Some(argument.parse::<usize>().map_err(|_| {
                            format!("`--require-weight {argument}` is not a weight")
                        })?);
                }
                _ => return Err(format!("unknown argument `{flag}`")),
            }
            continue;
        }
        match argument.as_str() {
            "--signers" => pending = Some("--signers"),
            "--principal" => pending = Some("--principal"),
            "--revoked" => pending = Some("--revoked"),
            "--require" => pending = Some("--require"),
            "--policy" => pending = Some("--policy"),
            "--require-weight" => pending = Some("--require-weight"),
            other if other.starts_with('-') => {
                return Err(format!(
                    "unknown argument `{other}`; `xirang verify` takes one packaged directory, optionally with `--signers <file> --principal <name>… [--require <n> | --policy <file> [--require-weight <n>]] [--revoked <file>]`"
                ));
            }
            other => {
                if directory.replace(other.to_owned()).is_some() {
                    return Err("`xirang verify` takes one packaged directory".to_owned());
                }
            }
        }
    }
    if pending.is_some() {
        return Err(
            "`--signers`, `--principal`, `--revoked`, `--require`, `--policy` and `--require-weight` need values".to_owned(),
        );
    }
    let directory = directory.ok_or_else(|| {
        "`xirang verify` needs the packaged directory printed by `xirang package`".to_owned()
    })?;
    let directory = std::path::Path::new(&directory);
    writeln!(out, "{}", crate::multilang::verify_package(directory)?)
        .map_err(|error| format!("cannot write: {error}"))?;
    match signers {
        Some(signers) => {
            if principals.is_empty() {
                return Err(
                    "`--signers` needs at least one `--principal`: a signature is verified for named principal(s)"
                        .to_owned(),
                );
            }
            let signers = std::path::Path::new(&signers);
            writeln!(
                out,
                "{}",
                crate::multilang::verify_signature(
                    directory,
                    signers,
                    &principals,
                    required,
                    revoked.as_deref().map(std::path::Path::new),
                    policy.as_deref().map(std::path::Path::new),
                    required_weight
                )?
            )
            .map_err(|error| format!("cannot write: {error}"))?;
            Ok(())
        }
        None => {
            if policy.is_some() || required_weight.is_some() {
                return Err(
                    "`--policy` and `--require-weight` need `--signers` and `--principal`"
                        .to_owned(),
                );
            }
            if revoked.is_some() {
                return Err(
                    "`--revoked` needs `--signers` and `--principal`: revocation applies to a named signer"
                        .to_owned(),
                );
            }
            if !principals.is_empty() {
                return Err(
                    "`--principal` without `--signers` has no allowed signers to verify against"
                        .to_owned(),
                );
            }
            Ok(())
        }
    }
}
