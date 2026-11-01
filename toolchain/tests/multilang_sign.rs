#![cfg(feature = "cli")]
//! End-to-end: `xirang sign` signs a packaged manifest with an SSH key and `xirang verify
//! --signers --principal` accepts exactly that signature — not a tampered manifest, not another
//! principal, and not a missing signature. `ssh-keygen` generates the key inside the test, so no
//! key or trust root is committed.
//! 端到端：`xirang sign` 用 SSH 密钥签打包清单，`xirang verify --signers --principal` 只接受那一个
//! 签名——篡改过的清单、别的 principal、缺失的签名都不接受。密钥由测试内的 `ssh-keygen` 生成，
//! 仓库里不存任何密钥或信任根。

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// The binary under test.
/// 被测二进制。
fn binary() -> &'static str {
    env!("CARGO_BIN_EXE_xirang")
}

/// A fresh scratch directory.
/// 一个全新的临时目录。
fn scratch(label: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "xirang-sign-{label}-{}-{:?}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&path).unwrap();
    path
}

/// Run the binary in `directory`.
/// 在 `directory` 里运行该二进制。
fn invoke(directory: &Path, arguments: &[&str]) -> Output {
    Command::new(binary())
        .args(arguments)
        .current_dir(directory)
        .output()
        .unwrap()
}

/// Both streams, for a failure message.
/// 两条输出流，供失败信息使用。
fn both(output: &Output) -> String {
    format!(
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

/// Whether `ssh-keygen` is on this machine.
/// 本机是否有 `ssh-keygen`。
fn ssh_keygen() -> bool {
    Command::new("sh")
        .args(["-c", "command -v ssh-keygen"])
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

/// Sign the package, then return the paths of the package, key and allowed-signers file.
/// 签好包，返回包目录、密钥与允许签名者文件的路径。
fn signed(label: &str) -> (PathBuf, PathBuf, PathBuf) {
    let root = scratch(label);
    let project = root.join("widget");
    std::fs::create_dir_all(project.join("src")).unwrap();
    std::fs::write(
        project.join("xirang.toml"),
        "schema = 1\n\n[package]\nname = \"widget\"\nversion = \"0.1.0\"\nkind = \"library\"\n\n\
         [language.cpp]\nstandard = \"17\"\n\n[dependencies]\n",
    )
    .unwrap();
    std::fs::write(
        project.join("src/widget.cpp"),
        "int widget_value() { return 1; }\n",
    )
    .unwrap();
    let built = invoke(&project, &["build"]);
    assert!(built.status.success(), "{}", both(&built));
    let out = root.join("out");
    let packaged = invoke(&project, &["package", "--output", out.to_str().unwrap()]);
    assert!(packaged.status.success(), "{}", both(&packaged));

    let key = root.join("key");
    let generated = Command::new("ssh-keygen")
        .args(["-t", "ed25519", "-N", "", "-C", "test", "-f"])
        .arg(&key)
        .arg("-q")
        .output()
        .unwrap();
    assert!(generated.status.success(), "{}", both(&generated));
    let public = std::fs::read_to_string(root.join("key.pub")).unwrap();
    let signers = root.join("allowed_signers");
    std::fs::write(&signers, format!("builder {public}")).unwrap();

    let signed = invoke(
        &out,
        &[
            "sign",
            out.to_str().unwrap(),
            "--key",
            key.to_str().unwrap(),
        ],
    );
    assert!(signed.status.success(), "{}", both(&signed));
    (out, key, signers)
}

#[test]
fn a_signature_verifies_for_the_named_principal_and_nothing_else_changes_it() {
    if !ssh_keygen() {
        eprintln!("skipping: no ssh-keygen on this machine");
        return;
    }
    let (out, _key, signers) = signed("ok");
    let verified = invoke(
        &out,
        &[
            "verify",
            out.to_str().unwrap(),
            "--signers",
            signers.to_str().unwrap(),
            "--principal",
            "builder",
        ],
    );
    assert!(verified.status.success(), "{}", both(&verified));
    let report = String::from_utf8_lossy(&verified.stdout);
    assert!(report.contains("signature: verified"), "{report}");
    assert!(report.contains("builder"), "{report}");

    // Another principal is another statement.
    // 另一个 principal 是另一份声明。
    let wrong = invoke(
        &out,
        &[
            "verify",
            out.to_str().unwrap(),
            "--signers",
            signers.to_str().unwrap(),
            "--principal",
            "intruder",
        ],
    );
    assert!(!wrong.status.success(), "{}", both(&wrong));

    // A byte appended to the manifest is a different manifest.
    // 清单末尾多一个字节就是另一份清单。
    let manifest = out.join("manifest.json");
    let mut bytes = std::fs::read(&manifest).unwrap();
    bytes.push(b'\n');
    std::fs::write(&manifest, bytes).unwrap();
    let tampered = invoke(
        &out,
        &[
            "verify",
            out.to_str().unwrap(),
            "--signers",
            signers.to_str().unwrap(),
            "--principal",
            "builder",
        ],
    );
    assert!(!tampered.status.success(), "{}", both(&tampered));
    assert!(
        both(&tampered).contains("does not verify"),
        "{}",
        both(&tampered)
    );
}

#[test]
fn an_unsigned_package_is_refused_instead_of_being_read_as_trusted() {
    if !ssh_keygen() {
        eprintln!("skipping: no ssh-keygen on this machine");
        return;
    }
    let (out, _key, signers) = signed("unsigned");
    let signature = out.join("manifest.json.sig");
    std::fs::remove_file(&signature).unwrap();
    let refused = invoke(
        &out,
        &[
            "verify",
            out.to_str().unwrap(),
            "--signers",
            signers.to_str().unwrap(),
            "--principal",
            "builder",
        ],
    );
    assert!(!refused.status.success(), "{}", both(&refused));
    assert!(
        both(&refused).contains("carries no manifest.json.sig"),
        "{}",
        both(&refused)
    );

    // The plain package check still passes: a missing signature is not a damaged package.
    // 普通包检查仍然通过：缺签名不等于包坏了。
    let plain = invoke(&out, &["verify", out.to_str().unwrap()]);
    assert!(plain.status.success(), "{}", both(&plain));
}

#[test]
fn a_revoked_signing_key_is_refused_by_fingerprint() {
    if !ssh_keygen() {
        eprintln!("skipping: no ssh-keygen on this machine");
        return;
    }
    let (out, key, signers) = signed("revoked");
    let public = std::fs::read_to_string(key.with_file_name("key.pub")).unwrap();
    let revoked = key.with_file_name("revoked");
    std::fs::write(&revoked, &public).unwrap();
    let refused = invoke(
        &out,
        &[
            "verify",
            out.to_str().unwrap(),
            "--signers",
            signers.to_str().unwrap(),
            "--principal",
            "builder",
            "--revoked",
            revoked.to_str().unwrap(),
        ],
    );
    assert!(!refused.status.success(), "{}", both(&refused));
    let report = both(&refused);
    assert!(report.contains("revoked"), "{report}");
    assert!(report.contains("SHA256:"), "{report}");

    // An unrelated key in the list does not revoke this signer.
    // 撤销清单里列一个无关的密钥，不会撤销这个签名者。
    let other = key.with_file_name("other");
    let generated = Command::new("ssh-keygen")
        .args(["-t", "ed25519", "-N", "", "-C", "other", "-f"])
        .arg(&other)
        .arg("-q")
        .output()
        .unwrap();
    assert!(generated.status.success(), "{}", both(&generated));
    let other_public = std::fs::read_to_string(key.with_file_name("other.pub")).unwrap();
    std::fs::write(&revoked, other_public).unwrap();
    let accepted = invoke(
        &out,
        &[
            "verify",
            out.to_str().unwrap(),
            "--signers",
            signers.to_str().unwrap(),
            "--principal",
            "builder",
            "--revoked",
            revoked.to_str().unwrap(),
        ],
    );
    assert!(accepted.status.success(), "{}", both(&accepted));
    assert!(
        String::from_utf8_lossy(&accepted.stdout)
            .contains("revocation: `builder`'s key is not named"),
        "{}",
        both(&accepted)
    );

    // Revocation without a named signer says what it needs.
    // 没点名签名者就给 `--revoked`，要说出缺什么。
    let bare = invoke(
        &out,
        &[
            "verify",
            out.to_str().unwrap(),
            "--revoked",
            revoked.to_str().unwrap(),
        ],
    );
    assert!(!bare.status.success(), "{}", both(&bare));
    assert!(both(&bare).contains("needs `--signers`"), "{}", both(&bare));
}

#[test]
fn two_principals_each_need_their_own_signature() {
    if !ssh_keygen() {
        eprintln!("skipping: no ssh-keygen on this machine");
        return;
    }
    let (out, key, signers) = signed("multi");
    let text = out.to_str().unwrap();
    let builder = invoke(
        &out,
        &[
            "sign",
            text,
            "--key",
            key.to_str().unwrap(),
            "--principal",
            "builder",
        ],
    );
    assert!(builder.status.success(), "{}", both(&builder));
    assert!(out.join("manifest.json.builder.sig").is_file());

    // A second signer needs a second key and a second signature.
    // 第二个签名者要第二把密钥、第二份签名。
    let reviewer_key = key.with_file_name("reviewer");
    let generated = Command::new("ssh-keygen")
        .args(["-t", "ed25519", "-N", "", "-C", "reviewer", "-f"])
        .arg(&reviewer_key)
        .arg("-q")
        .output()
        .unwrap();
    assert!(generated.status.success(), "{}", both(&generated));
    let reviewer_public = std::fs::read_to_string(key.with_file_name("reviewer.pub")).unwrap();
    let mut allowed = std::fs::read_to_string(&signers).unwrap();
    allowed.push_str(&format!("reviewer {reviewer_public}"));
    std::fs::write(&signers, allowed).unwrap();
    let reviewer = invoke(
        &out,
        &[
            "sign",
            text,
            "--key",
            reviewer_key.to_str().unwrap(),
            "--principal",
            "reviewer",
        ],
    );
    assert!(reviewer.status.success(), "{}", both(&reviewer));

    let both_signed = invoke(
        &out,
        &[
            "verify",
            text,
            "--signers",
            signers.to_str().unwrap(),
            "--principal",
            "builder",
            "--principal",
            "reviewer",
        ],
    );
    assert!(both_signed.status.success(), "{}", both(&both_signed));
    let report = String::from_utf8_lossy(&both_signed.stdout);
    assert_eq!(report.matches("signature: verified").count(), 2, "{report}");

    // Dropping one signer's file is a refusal that names the principal.
    // 少一位签名者的文件，要按名拒绝。
    std::fs::remove_file(out.join("manifest.json.reviewer.sig")).unwrap();
    let missing = invoke(
        &out,
        &[
            "verify",
            text,
            "--signers",
            signers.to_str().unwrap(),
            "--principal",
            "builder",
            "--principal",
            "reviewer",
        ],
    );
    assert!(!missing.status.success(), "{}", both(&missing));
    assert!(
        both(&missing).contains("reviewer") && both(&missing).contains("no signature of their own"),
        "{}",
        both(&missing)
    );

    // Signing the same principal twice is refused rather than overwritten.
    // 同一 principal 重复签名要拒绝，不静默覆盖。
    let again = invoke(
        &out,
        &[
            "sign",
            text,
            "--key",
            key.to_str().unwrap(),
            "--principal",
            "builder",
        ],
    );
    assert!(!again.status.success(), "{}", both(&again));
    assert!(both(&again).contains("already signed"), "{}", both(&again));
}

#[test]
fn a_threshold_accepts_some_named_signers_and_refuses_below_it() {
    if !ssh_keygen() {
        eprintln!("skipping: no ssh-keygen on this machine");
        return;
    }
    let (out, key, signers) = signed("threshold");
    let text = out.to_str().unwrap();
    let builder = invoke(
        &out,
        &[
            "sign",
            text,
            "--key",
            key.to_str().unwrap(),
            "--principal",
            "builder",
        ],
    );
    assert!(builder.status.success(), "{}", both(&builder));
    let reviewer_key = key.with_file_name("reviewer");
    Command::new("ssh-keygen")
        .args(["-t", "ed25519", "-N", "", "-C", "reviewer", "-f"])
        .arg(&reviewer_key)
        .arg("-q")
        .output()
        .unwrap();
    let reviewer_public = std::fs::read_to_string(key.with_file_name("reviewer.pub")).unwrap();
    let mut allowed = std::fs::read_to_string(&signers).unwrap();
    allowed.push_str(&format!("reviewer {reviewer_public}"));
    std::fs::write(&signers, allowed).unwrap();
    let reviewer = invoke(
        &out,
        &[
            "sign",
            text,
            "--key",
            reviewer_key.to_str().unwrap(),
            "--principal",
            "reviewer",
        ],
    );
    assert!(reviewer.status.success(), "{}", both(&reviewer));

    // Two of three named signers sign: a threshold of two is met, a threshold of three is not.
    // 三位签名者中两位签了：阈值 2 满足、阈值 3 不满足。
    let met = invoke(
        &out,
        &[
            "verify",
            text,
            "--signers",
            signers.to_str().unwrap(),
            "--principal",
            "builder",
            "--principal",
            "reviewer",
            "--principal",
            "auditor",
            "--require",
            "2",
        ],
    );
    assert!(met.status.success(), "{}", both(&met));
    let report = String::from_utf8_lossy(&met.stdout);
    assert!(
        report.contains("signatures: 2 verified of 3 named"),
        "{report}"
    );
    assert!(report.contains("unsigned: auditor"), "{report}");

    let short = invoke(
        &out,
        &[
            "verify",
            text,
            "--signers",
            signers.to_str().unwrap(),
            "--principal",
            "builder",
            "--principal",
            "reviewer",
            "--principal",
            "auditor",
            "--require",
            "3",
        ],
    );
    assert!(!short.status.success(), "{}", both(&short));
    assert!(both(&short).contains("3 required"), "{}", both(&short));

    // A revoked signature stops counting toward the threshold.
    // 被撤销的签名不再计入阈值。
    let revoked = key.with_file_name("revoked_threshold");
    std::fs::write(&revoked, &reviewer_public).unwrap();
    let one_left = invoke(
        &out,
        &[
            "verify",
            text,
            "--signers",
            signers.to_str().unwrap(),
            "--principal",
            "builder",
            "--principal",
            "reviewer",
            "--require",
            "2",
            "--revoked",
            revoked.to_str().unwrap(),
        ],
    );
    assert!(!one_left.status.success(), "{}", both(&one_left));
    assert!(
        both(&one_left).contains("1 verified"),
        "{}",
        both(&one_left)
    );

    let one_required = invoke(
        &out,
        &[
            "verify",
            text,
            "--signers",
            signers.to_str().unwrap(),
            "--principal",
            "builder",
            "--principal",
            "reviewer",
            "--require",
            "1",
            "--revoked",
            revoked.to_str().unwrap(),
        ],
    );
    assert!(one_required.status.success(), "{}", both(&one_required));
    let report = String::from_utf8_lossy(&one_required.stdout);
    assert!(report.contains("threshold 1"), "{report}");

    // A threshold outside the named principals is refused by name.
    // 阈值超出点名的签名者数量时按名拒绝。
    for bad in ["0", "4"] {
        let refused = invoke(
            &out,
            &[
                "verify",
                text,
                "--signers",
                signers.to_str().unwrap(),
                "--principal",
                "builder",
                "--principal",
                "reviewer",
                "--require",
                bad,
            ],
        );
        assert!(!refused.status.success(), "{}", both(&refused));
        assert!(
            both(&refused).contains("between 1 and 2"),
            "{}",
            both(&refused)
        );
    }
}

/// Generate a key and add it to the allowed-signers file under `principal`.
/// 生成一把密钥，并以 `principal` 加进允许签名者文件。
fn add_signer(key: &Path, principal: &str, signers: &Path) {
    let generated = Command::new("ssh-keygen")
        .args(["-t", "ed25519", "-N", "", "-C", principal, "-f"])
        .arg(key)
        .arg("-q")
        .output()
        .unwrap();
    assert!(generated.status.success(), "{}", both(&generated));
    let public = std::fs::read_to_string(key.with_extension("pub")).unwrap();
    let mut allowed = std::fs::read_to_string(signers).unwrap();
    allowed.push_str(&format!("{principal} {public}"));
    std::fs::write(signers, allowed).unwrap();
}

#[test]
fn a_weighted_policy_counts_weights_instead_of_signatures() {
    if !ssh_keygen() {
        eprintln!("skipping: no ssh-keygen on this machine");
        return;
    }
    let (out, key, signers) = signed("weighted");
    let text = out.to_str().unwrap();
    let builder = invoke(
        &out,
        &[
            "sign",
            text,
            "--key",
            key.to_str().unwrap(),
            "--principal",
            "builder",
        ],
    );
    assert!(builder.status.success(), "{}", both(&builder));
    let reviewer_key = key.with_file_name("reviewer");
    add_signer(&reviewer_key, "reviewer", &signers);
    let auditor_key = key.with_file_name("auditor");
    add_signer(&auditor_key, "auditor", &signers);
    // The auditor is named by the policy and by the verification, but does not sign: weights let
    // a threshold pass without every named principal.
    let signed = invoke(
        &out,
        &[
            "sign",
            text,
            "--key",
            reviewer_key.to_str().unwrap(),
            "--principal",
            "reviewer",
        ],
    );
    assert!(signed.status.success(), "{}", both(&signed));
    let policy = out.with_file_name("policy");
    std::fs::write(&policy, "builder 3\nreviewer 1\nauditor 2\n").unwrap();
    let policy_text = policy.to_str().unwrap();
    let signers_text = signers.to_str().unwrap();

    let passed = invoke(
        &out,
        &[
            "verify",
            text,
            "--signers",
            signers_text,
            "--principal",
            "builder",
            "--principal",
            "reviewer",
            "--principal",
            "auditor",
            "--policy",
            policy_text,
            "--require-weight",
            "4",
        ],
    );
    assert!(passed.status.success(), "{}", both(&passed));
    let ok = String::from_utf8_lossy(&passed.stdout).into_owned();
    assert!(ok.contains("weight 4 verified of 6"), "{ok}");

    // With only the builder's signature, weight 3 is below a threshold of 4.
    let short = invoke(
        &out,
        &[
            "verify",
            text,
            "--signers",
            signers_text,
            "--principal",
            "builder",
            "--principal",
            "reviewer",
            "--principal",
            "auditor",
            "--policy",
            policy_text,
            "--require-weight",
            "5",
        ],
    );
    assert!(!short.status.success(), "{}", both(&short));
    assert!(
        both(&short).contains("weight 4 verified of 6"),
        "{}",
        both(&short)
    );

    // Counting and weighing are two different policies; giving both is refused.
    let mixed = invoke(
        &out,
        &[
            "verify",
            text,
            "--signers",
            signers_text,
            "--principal",
            "builder",
            "--policy",
            policy_text,
            "--require",
            "1",
        ],
    );
    assert!(!mixed.status.success());
    assert!(
        both(&mixed).contains("give one, not both"),
        "{}",
        both(&mixed)
    );

    // A named principal the policy does not weigh is refused by name.
    std::fs::write(&policy, "builder 3\nreviewer 1\n").unwrap();
    let absent = invoke(
        &out,
        &[
            "verify",
            text,
            "--signers",
            signers_text,
            "--principal",
            "auditor",
            "--principal",
            "builder",
            "--policy",
            policy_text,
        ],
    );
    assert!(!absent.status.success());
    assert!(
        both(&absent).contains("names no weight for `auditor`"),
        "{}",
        both(&absent)
    );
}

/// The SHA-256 fingerprint `ssh-keygen` prints for a public key.
/// `ssh-keygen` 为公钥打印的 SHA-256 指纹。
fn fingerprint(public: &Path) -> String {
    let output = Command::new("ssh-keygen")
        .arg("-lf")
        .arg(public)
        .output()
        .unwrap();
    assert!(output.status.success(), "{}", both(&output));
    String::from_utf8_lossy(&output.stdout)
        .split_whitespace()
        .nth(1)
        .unwrap()
        .to_owned()
}

#[test]
fn a_policy_keyed_by_fingerprint_survives_a_principal_rename() {
    if !ssh_keygen() {
        eprintln!("skipping: no ssh-keygen on this machine");
        return;
    }
    let (out, key, signers) = signed("fingerprint-policy");
    let text = out.to_str().unwrap();
    let builder = invoke(
        &out,
        &[
            "sign",
            text,
            "--key",
            key.to_str().unwrap(),
            "--principal",
            "builder",
        ],
    );
    assert!(builder.status.success(), "{}", both(&builder));
    let reviewer_key = key.with_file_name("reviewer");
    add_signer(&reviewer_key, "reviewer", &signers);
    let reviewer = invoke(
        &out,
        &[
            "sign",
            text,
            "--key",
            reviewer_key.to_str().unwrap(),
            "--principal",
            "reviewer",
        ],
    );
    assert!(reviewer.status.success(), "{}", both(&reviewer));
    let builder_fingerprint = fingerprint(&key.with_extension("pub"));
    let reviewer_fingerprint = fingerprint(&reviewer_key.with_extension("pub"));
    let policy = out.with_file_name("policy");
    std::fs::write(
        &policy,
        format!("{builder_fingerprint} 2\n{reviewer_fingerprint} 1\n"),
    )
    .unwrap();
    let policy_text = policy.to_str().unwrap();
    let signers_text = signers.to_str().unwrap();
    let passed = invoke(
        &out,
        &[
            "verify",
            text,
            "--signers",
            signers_text,
            "--principal",
            "builder",
            "--principal",
            "reviewer",
            "--policy",
            policy_text,
            "--require-weight",
            "3",
        ],
    );
    assert!(passed.status.success(), "{}", both(&passed));
    assert!(String::from_utf8_lossy(&passed.stdout).contains("weight 3 verified of 3"));

    // The signing key is what carries the weight: renaming the principal in the allowed signers
    // keeps the same weight.
    let mut allowed = std::fs::read_to_string(&signers).unwrap();
    allowed = allowed.replace("builder ", "carol ");
    std::fs::write(&signers, allowed).unwrap();
    let carol = invoke(
        &out,
        &[
            "sign",
            text,
            "--key",
            key.to_str().unwrap(),
            "--principal",
            "carol",
        ],
    );
    assert!(carol.status.success(), "{}", both(&carol));
    let renamed = invoke(
        &out,
        &[
            "verify",
            text,
            "--signers",
            signers_text,
            "--principal",
            "carol",
            "--policy",
            policy_text,
            "--require-weight",
            "2",
        ],
    );
    assert!(renamed.status.success(), "{}", both(&renamed));
    assert!(String::from_utf8_lossy(&renamed.stdout).contains("weight 2 verified of 2"));

    // A key the policy does not weigh is refused with both its name and its fingerprint.
    let other_key = key.with_file_name("other");
    add_signer(&other_key, "other", &signers);
    let refused = invoke(
        &out,
        &[
            "verify",
            text,
            "--signers",
            signers_text,
            "--principal",
            "other",
            "--policy",
            policy_text,
            "--require-weight",
            "1",
        ],
    );
    assert!(!refused.status.success());
    assert!(
        both(&refused).contains("names no weight for `other` (key SHA256:"),
        "{}",
        both(&refused)
    );
}
