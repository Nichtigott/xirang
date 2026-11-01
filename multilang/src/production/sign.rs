//! Package signatures: sign `manifest.json` with an SSH key, verify it against a list of allowed
//! signers. The manifest binds every packaged file by sha256, so a verified signature covers the
//! files transitively. This tool carries no trust root: the caller names the allowed-signers file
//! and the principal, and `ssh-keygen` does the cryptography.
//! 包签名：用 SSH 密钥签 `manifest.json`，再按允许签名者文件核验。清单用 sha256 绑定每个打包文件，
//! 因此通过核验的签名间接覆盖全部文件。本工具不内置信任根：调用方给出允许签名者文件与 principal，
//! 密码学由 `ssh-keygen` 完成。

use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// A signing policy: how much each principal's signature weighs.
/// 签名策略：每一位 principal 的签名值多少权重。
pub(super) struct Policy {
    pub(super) path: PathBuf,
    pub(super) weights: BTreeMap<String, usize>,
}

impl Policy {
    /// Read `principal weight` lines; `#` starts a comment.
    /// 读 `principal weight` 行；`#` 起注释。
    pub(super) fn load(path: &Path) -> Result<Self, String> {
        let text = std::fs::read_to_string(path)
            .map_err(|error| format!("cannot read the policy {}: {error}", path.display()))?;
        let mut weights = BTreeMap::new();
        for (index, line) in text.lines().enumerate() {
            let line = line.split('#').next().unwrap_or("").trim();
            if line.is_empty() {
                continue;
            }
            let mut parts = line.split_whitespace();
            let principal = parts.next().unwrap_or_default();
            let Some(weight) = parts.next() else {
                return Err(format!(
                    "{}:{}: a policy line is `principal weight` or `SHA256:… weight`",
                    path.display(),
                    index + 1
                ));
            };
            if parts.next().is_some() {
                return Err(format!(
                    "{}:{}: a policy line is `principal weight` or `SHA256:… weight`",
                    path.display(),
                    index + 1
                ));
            }
            if let Some(rest) = principal.strip_prefix("SHA256:")
                && (rest.len() != 43
                    || !rest.chars().all(|character| {
                        character.is_ascii_alphanumeric() || matches!(character, '+' | '/')
                    }))
            {
                return Err(format!(
                    "{}:{}: `{principal}` is not a SHA256 fingerprint (`SHA256:` and 43 base64 characters)",
                    path.display(),
                    index + 1
                ));
            }
            let weight: usize = weight.parse().map_err(|_| {
                format!(
                    "{}:{}: `{weight}` is not a weight (a positive whole number)",
                    path.display(),
                    index + 1
                )
            })?;
            if weight == 0 {
                return Err(format!(
                    "{}:{}: a weight must be at least 1",
                    path.display(),
                    index + 1
                ));
            }
            if weights.insert(principal.to_owned(), weight).is_some() {
                return Err(format!(
                    "{}:{}: `{principal}` is named twice",
                    path.display(),
                    index + 1
                ));
            }
        }
        if weights.is_empty() {
            return Err(format!("{} names no principal", path.display()));
        }
        Ok(Self {
            path: path.to_path_buf(),
            weights,
        })
    }

    /// The weight of one named principal, or of the key that signed for a renamed one.
    /// 一位被点名 principal 的权重；principal 改过名时按它的密钥指纹取。
    pub(super) fn weight_for(
        &self,
        principal: &str,
        fingerprint: Option<&str>,
    ) -> Result<usize, String> {
        if let Some(weight) = self.weights.get(principal) {
            return Ok(*weight);
        }
        if let Some(fingerprint) = fingerprint
            && let Some(weight) = self.weights.get(fingerprint)
        {
            return Ok(*weight);
        }
        let key = fingerprint
            .map(|fingerprint| format!(" (key {fingerprint})"))
            .unwrap_or_default();
        Err(format!(
            "the policy {} names no weight for `{principal}`{key}; it names {}",
            self.path.display(),
            self.weights
                .keys()
                .cloned()
                .collect::<Vec<String>>()
                .join(", ")
        ))
    }
}

/// Sign `<directory>/manifest.json`, writing `<directory>/manifest.json.sig`.
/// 签 `<directory>/manifest.json`，写出 `<directory>/manifest.json.sig`。
pub(super) fn sign(
    directory: &Path,
    key: &Path,
    principal: Option<&str>,
) -> Result<String, String> {
    let manifest = directory.join("manifest.json");
    if !manifest.is_file() {
        return Err(format!(
            "{} is not a package: it has no manifest.json; `xirang package` writes one",
            directory.display()
        ));
    }
    if !key.is_file() {
        return Err(format!("`{}` is not a key file", key.display()));
    }
    let output = Command::new("ssh-keygen")
        .args(["-Y", "sign", "-f"])
        .arg(key)
        .args(["-n", "xirang"])
        .arg(&manifest)
        .output()
        .map_err(|error| format!("cannot run ssh-keygen: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "ssh-keygen refused to sign: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let written = directory.join("manifest.json.sig");
    if !written.is_file() {
        return Err(format!(
            "ssh-keygen reported success but wrote no {}",
            written.display()
        ));
    }
    let signature = match principal {
        Some(principal) => {
            if principal.is_empty()
                || !principal.chars().all(|character| {
                    character.is_ascii_alphanumeric() || matches!(character, '_' | '.' | '-')
                })
            {
                return Err(format!(
                    "`{principal}` is not a principal name; use letters, digits, `_`, `.` and `-`"
                ));
            }
            let named = directory.join(format!("manifest.json.{principal}.sig"));
            if named.is_file() {
                return Err(format!(
                    "`{principal}` already signed {}; remove {} to sign again",
                    manifest.display(),
                    named.display()
                ));
            }
            std::fs::rename(&written, &named).map_err(|error| {
                format!("cannot name the signature after `{principal}`: {error}")
            })?;
            named
        }
        None => written,
    };
    Ok(format!(
        "signed {} with `{}`\n  the signature is {}; it covers the manifest, which binds every file it names",
        manifest.display(),
        key.display(),
        signature.file_name().map_or_else(
            || signature.display().to_string(),
            |name| name.to_string_lossy().into_owned()
        )
    ))
}

/// Verify `<directory>/manifest.json.sig` for `principal` against `signers`.
/// 按 `signers` 为 `principal` 核验 `<directory>/manifest.json.sig`。
pub(super) fn verify(
    directory: &Path,
    signers: &Path,
    principals: &[String],
    required: Option<usize>,
    revoked: Option<&Path>,
    policy: Option<&Policy>,
    required_weight: Option<usize>,
) -> Result<String, String> {
    let manifest = directory.join("manifest.json");
    if !manifest.is_file() {
        return Err(format!(
            "{} is not a package: it has no manifest.json",
            directory.display()
        ));
    }
    if principals.is_empty() {
        return Err("verification needs at least one principal".to_owned());
    }
    if !signers.is_file() {
        return Err(format!(
            "`{}` is not an allowed-signers file",
            signers.display()
        ));
    }
    if policy.is_some() && required.is_some() {
        return Err(
            "`--require` counts signatures and `--policy` weighs them; give one, not both"
                .to_owned(),
        );
    }
    if required_weight.is_some() && policy.is_none() {
        return Err("`--require-weight` needs `--policy`".to_owned());
    }
    let signers_text = std::fs::read_to_string(signers)
        .map_err(|error| format!("cannot read {}: {error}", signers.display()))?;
    let fingerprints: Vec<Option<String>> = match policy {
        Some(_) => principals
            .iter()
            .map(|principal| {
                allowed_key(&signers_text, principal)
                    .and_then(|(kind, base64)| fingerprint(&format!("{kind} {base64}\n")).ok())
            })
            .collect(),
        None => Vec::new(),
    };
    let weights: Vec<usize> = match policy {
        Some(policy) => principals
            .iter()
            .zip(&fingerprints)
            .map(|(principal, fingerprint)| policy.weight_for(principal, fingerprint.as_deref()))
            .collect::<Result<Vec<_>, _>>()?,
        None => Vec::new(),
    };
    let total_weight: usize = weights.iter().sum();
    let threshold_weight = match (policy, required_weight) {
        (Some(policy), Some(weight)) if weight == 0 || weight > total_weight => {
            return Err(format!(
                "`--require-weight {weight}` needs a weight between 1 and {total_weight} (the named principals in {})",
                policy.path.display()
            ));
        }
        (Some(_), Some(weight)) => Some(weight),
        (Some(_), None) => Some(total_weight),
        (None, _) => None,
    };
    let threshold = required.unwrap_or(principals.len());
    if policy.is_none() && (threshold == 0 || threshold > principals.len()) {
        return Err(format!(
            "`--require {threshold}` needs a count between 1 and {} (the named principals)",
            principals.len()
        ));
    }
    let revoked_text = match revoked {
        Some(path) => {
            if !path.is_file() {
                return Err(format!("`{}` is not a revocation list", path.display()));
            }
            Some(
                std::fs::read_to_string(path)
                    .map_err(|error| format!("cannot read {}: {error}", path.display()))?,
            )
        }
        None => None,
    };
    let single = principals.len() == 1;
    let mut lines = Vec::new();
    let mut verified = 0usize;
    let mut verified_weight = 0usize;
    let mut revoked_count = 0usize;
    let mut missing = Vec::new();
    let mut failed: Vec<(String, String)> = Vec::new();
    let mut revocation_lines = Vec::new();
    for (index, principal) in principals.iter().enumerate() {
        let weight = policy.map(|_| weights[index]).unwrap_or(0);
        let named = directory.join(format!("manifest.json.{principal}.sig"));
        let signature = if named.is_file() {
            named
        } else if single {
            directory.join("manifest.json.sig")
        } else {
            missing.push(principal.clone());
            continue;
        };
        if !signature.is_file() {
            missing.push(principal.clone());
            continue;
        }
        match verify_one(&manifest, &signature, signers, principal) {
            Ok(line) => {
                let fingerprint = match (&revoked_text, revoked) {
                    (Some(text), Some(_)) => revoked_fingerprint(&signers_text, text, principal)?,
                    _ => None,
                };
                match fingerprint {
                    Some(fingerprint) => {
                        revoked_count += 1;
                        lines.push(format!(
                            "signature: revoked — `{principal}` ({fingerprint})"
                        ));
                        if let Some(path) = revoked {
                            revocation_lines.push(format!(
                                "revocation: `{principal}`'s key is named in {}",
                                path.display()
                            ));
                        }
                    }
                    None => {
                        verified += 1;
                        verified_weight += weight;
                        lines.push(line);
                        if let Some(path) = revoked {
                            revocation_lines.push(format!(
                                "revocation: `{principal}`'s key is not named in {}",
                                path.display()
                            ));
                        }
                    }
                }
            }
            Err(error) => failed.push((principal.clone(), error)),
        }
    }
    let strict = required.is_none() && required_weight.is_none();
    if strict {
        if let Some((_, error)) = failed.first() {
            return Err(error.clone());
        }
        if !missing.is_empty() {
            if single {
                return Err(format!(
                    "{} carries no manifest.json.sig; `xirang sign` is what writes one",
                    directory.display()
                ));
            }
            return Err(format!(
                "these principals have no signature of their own: {}; with more than one principal each needs `manifest.json.<principal>.sig`, because a single `manifest.json.sig` is one signer's statement",
                missing.join(", ")
            ));
        }
        if revoked_count > 0 {
            let principal = principals
                .iter()
                .find(|principal| {
                    let named = directory.join(format!("manifest.json.{principal}.sig"));
                    revoked_text.as_ref().is_some_and(|text| {
                        revoked_fingerprint(&signers_text, text, principal)
                            .ok()
                            .flatten()
                            .is_some()
                            && (named.is_file() || single)
                    })
                })
                .cloned()
                .unwrap_or_default();
            let fingerprint = revoked_text
                .as_ref()
                .and_then(|text| {
                    revoked_fingerprint(&signers_text, text, &principal)
                        .ok()
                        .flatten()
                })
                .unwrap_or_default();
            return Err(format!(
                "the key `{principal}` signs with is revoked ({fingerprint}); {} names it",
                revoked
                    .map(|path| path.display().to_string())
                    .unwrap_or_default()
            ));
        }
    }
    let shortfall = match threshold_weight {
        Some(threshold) => verified_weight < threshold,
        None => verified < threshold,
    };
    if shortfall {
        let mut parts = match threshold_weight {
            Some(threshold) => vec![format!(
                "signatures: weight {verified_weight} verified of {total_weight} named; {threshold} required (policy {})",
                policy
                    .map(|policy| policy.path.display().to_string())
                    .unwrap_or_default()
            )],
            None => vec![format!(
                "signatures: {verified} verified of {} named; {threshold} required",
                principals.len()
            )],
        };
        if !missing.is_empty() {
            parts.push(format!("unsigned: {}", missing.join(", ")));
        }
        if !failed.is_empty() {
            parts.push(format!(
                "did not verify: {}",
                failed
                    .iter()
                    .map(|(principal, _)| principal.as_str())
                    .collect::<Vec<&str>>()
                    .join(", ")
            ));
        }
        if revoked_count > 0 {
            parts.push(format!("revoked: {revoked_count}"));
        }
        return Err(parts.join("; "));
    }
    if !lines.is_empty() {
        lines.push(String::new());
    }
    if !missing.is_empty() {
        lines.push(format!("unsigned: {}", missing.join(", ")));
    }
    lines.extend(revocation_lines);
    lines.push(match threshold_weight {
        Some(threshold) => format!(
            "signatures: weight {verified_weight} verified of {total_weight} (threshold {threshold}; policy {})",
            policy
                .map(|policy| policy.path.display().to_string())
                .unwrap_or_default()
        ),
        None => format!(
            "signatures: {verified} verified of {} named (threshold {threshold})",
            principals.len()
        ),
    });
    Ok(lines.join("\n"))
}

/// The SHA256 fingerprint of the key `principal` signs with, if the revocation list names it.
/// 若撤销清单点名了 `principal` 签名用的密钥，返回其 SHA256 指纹。
fn revoked_fingerprint(
    signers: &str,
    revoked: &str,
    principal: &str,
) -> Result<Option<String>, String> {
    let Some((kind, blob)) = allowed_key(signers, principal) else {
        return Ok(None);
    };
    if !revoked_keys(revoked)
        .into_iter()
        .any(|(other_kind, other_blob)| other_kind == kind && other_blob == blob)
    {
        return Ok(None);
    }
    fingerprint(&format!("{kind} {blob}")).map(Some)
}

/// Verify one signature for one principal.
/// 为一个 principal 核验一份签名。
fn verify_one(
    manifest: &Path,
    signature: &Path,
    signers: &Path,
    principal: &str,
) -> Result<String, String> {
    let bytes = std::fs::read(manifest)
        .map_err(|error| format!("cannot read {}: {error}", manifest.display()))?;
    let mut child = Command::new("ssh-keygen")
        .args(["-Y", "verify", "-f"])
        .arg(signers)
        .args(["-I", principal, "-n", "xirang", "-s"])
        .arg(signature)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("cannot run ssh-keygen: {error}"))?;
    child
        .stdin
        .as_mut()
        .ok_or_else(|| "cannot write to ssh-keygen".to_owned())?
        .write_all(&bytes)
        .map_err(|error| format!("cannot write to ssh-keygen: {error}"))?;
    let output = child
        .wait_with_output()
        .map_err(|error| format!("cannot wait for ssh-keygen: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "the manifest signature does not verify for `{principal}`: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(format!(
        "signature: verified — `{principal}` signed this manifest\n  {}",
        String::from_utf8_lossy(&output.stdout).trim()
    ))
}

/// Whether a token is an OpenSSH key type.
/// 某个词是否是 OpenSSH 密钥类型。
fn is_key_type(token: &str) -> bool {
    token.starts_with("ssh-") || token.starts_with("ecdsa-") || token.starts_with("sk-")
}

/// The key an allowed-signers file lets `principal` sign with, as `(type, base64)`.
/// 允许签名者文件给 `principal` 的密钥，写作 `(类型, base64)`。
fn allowed_key(signers: &str, principal: &str) -> Option<(String, String)> {
    for line in signers.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let tokens: Vec<&str> = line.split_whitespace().collect();
        let Some(at) = tokens.iter().position(|token| is_key_type(token)) else {
            continue;
        };
        if at + 1 >= tokens.len() {
            continue;
        }
        if tokens[0].split(',').any(|name| name == principal) {
            return Some((tokens[at].to_owned(), tokens[at + 1].to_owned()));
        }
    }
    None
}

/// Every public key a revocation list names, as `(type, base64)`.
/// 撤销清单点名的每个公钥，写作 `(类型, base64)`。
fn revoked_keys(text: &str) -> Vec<(String, String)> {
    text.lines()
        .filter_map(|line| {
            let tokens: Vec<&str> = line.split_whitespace().collect();
            let at = tokens.iter().position(|token| is_key_type(token))?;
            (at + 1 < tokens.len()).then(|| (tokens[at].to_owned(), tokens[at + 1].to_owned()))
        })
        .collect()
}

/// The SHA-256 fingerprint of one public key, asked of `ssh-keygen`.
/// 一个公钥的 SHA-256 指纹，向 `ssh-keygen` 要。
fn fingerprint(key: &str) -> Result<String, String> {
    let mut child = Command::new("ssh-keygen")
        .args(["-l", "-f", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("cannot run ssh-keygen: {error}"))?;
    child
        .stdin
        .as_mut()
        .ok_or_else(|| "cannot write to ssh-keygen".to_owned())?
        .write_all(key.as_bytes())
        .map_err(|error| format!("cannot write to ssh-keygen: {error}"))?;
    let output = child
        .wait_with_output()
        .map_err(|error| format!("cannot wait for ssh-keygen: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "ssh-keygen cannot fingerprint that key: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let output = String::from_utf8_lossy(&output.stdout);
    output
        .split_whitespace()
        .nth(1)
        .map(str::to_owned)
        .ok_or_else(|| format!("ssh-keygen printed no fingerprint: {}", output.trim()))
}

/// Sign a packaged directory's manifest with an SSH key.
/// 用 SSH 密钥给打包目录的清单签名。
pub fn sign_package(
    directory: &Path,
    key: &Path,
    principal: Option<&str>,
) -> Result<String, String> {
    sign(directory, key, principal)
}

/// Verify a packaged directory's manifest signatures for the named principals.
/// 按点名的 principals 核验打包目录的清单签名。
pub fn verify_signature(
    directory: &Path,
    signers: &Path,
    principals: &[String],
    required: Option<usize>,
    revoked: Option<&Path>,
    policy: Option<&Path>,
    required_weight: Option<usize>,
) -> Result<String, String> {
    let policy = match policy {
        Some(path) => Some(Policy::load(path)?),
        None => None,
    };
    verify(
        directory,
        signers,
        principals,
        required,
        revoked,
        policy.as_ref(),
        required_weight,
    )
}
