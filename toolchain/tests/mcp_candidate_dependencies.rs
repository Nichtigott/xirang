#![cfg(feature = "mcp")]
//! R5 and R2: a candidate must stay buildable when the base depends on a crate by a **relative**
//! path, and a change to an out-of-root dependency between `open` and `land` must invalidate the
//! verification instead of being certified by it.
//! R5 与 R2：当基树用**相对**路径依赖一个 crate 时，候选必须仍然可构建；而在 `open` 与 `land`
//! 之间改动根外依赖，必须让那次验证失效，而不是被它认证。
//!
//! Both are driven through the real binary and real `cargo`: the 2026-10-19 review reproduced them
//! with `--call` on a fixture that is *not* shaped like anything in this file, and a unit test over
//! the manifest text would have reproduced neither.
//! 两者都走真二进制与真 `cargo`：2026-10-19 复核用 `--call` 在**不是**照这份文件捏出来的夹具上
//! 复现了它们，而一个对着清单文本的单元测试两件都复现不了。

use std::path::{Path, PathBuf};
use std::process::Command;

/// A host that depends on one external crate by `path`, plus that crate.
/// 一个用 `path` 依赖一个外部 crate 的宿主，以及那个 crate。
fn fixture(label: &str, absolute: bool) -> (PathBuf, PathBuf) {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let base = std::env::temp_dir().join(format!(
        "xirang-dependency-{label}-{}-{sequence}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&base);
    let dependency = base.join("external-dep");
    let host = base.join("host");
    std::fs::create_dir_all(dependency.join("src")).unwrap();
    std::fs::create_dir_all(host.join("src")).unwrap();
    std::fs::write(
        dependency.join("Cargo.toml"),
        "[package]\nname='external-dep'\nversion='0.0.0'\nedition='2024'\n",
    )
    .unwrap();
    std::fs::write(
        dependency.join("src/lib.rs"),
        "pub fn value() -> i64 {\n    1\n}\n",
    )
    .unwrap();
    let spelled = if absolute {
        dependency.display().to_string()
    } else {
        "../external-dep".to_owned()
    };
    std::fs::write(
        host.join("Cargo.toml"),
        format!(
            "[package]\nname='dependency-host'\nversion='0.0.0'\nedition='2024'\n[workspace]\n[dependencies]\nexternal-dep = {{ path = \"{spelled}\" }}\n"
        ),
    )
    .unwrap();
    std::fs::write(
        host.join("src/lib.rs"),
        "pub fn answer() -> i64 {\n    external_dep::value() + 40\n}\n\n#[test]\nfn the_dependency_is_used() {\n    assert_eq!(answer(), 41);\n}\n",
    )
    .unwrap();
    let output = Command::new("cargo")
        .args(["generate-lockfile", "--offline"])
        .current_dir(&host)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "the fixture's lockfile: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    (host, dependency)
}

/// One `--call`, returning its exit code and its two streams joined.
/// 一次 `--call`，交回退出码与合并后的两路输出。
fn call(root: &Path, tool: &str, request: &str) -> (i32, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_xirang-mcp"))
        .args(["--call", tool, "--root"])
        .arg(root)
        .args(["--json", request])
        .output()
        .unwrap();
    let mut text = String::from_utf8_lossy(&output.stdout).into_owned();
    text.push_str(&String::from_utf8_lossy(&output.stderr));
    (output.status.code().unwrap_or(-1), text)
}

/// The id the opening line names.
/// 开候选那一行点名的 id。
fn opened(root: &Path, scope: &str) -> String {
    let (code, text) = call(
        root,
        "xirang.candidate",
        &format!("{{\"action\":\"open\",\"scope\":[{scope}]}}"),
    );
    assert_eq!(code, 0, "opening a candidate: {text}");
    text.lines()
        .next()
        .and_then(|line| line.strip_prefix("candidate "))
        .and_then(|rest| rest.strip_suffix(" open"))
        .and_then(|rest| rest.split_whitespace().next())
        .unwrap_or_else(|| panic!("no opening line in:\n{text}"))
        .to_owned()
}

/// The person's confirmation of a landing, through the host entry.
/// 通过宿主入口对一次落地的使用方确认。
fn confirm_landing(root: &Path, id: &str) -> i32 {
    Command::new(env!("CARGO_BIN_EXE_xirang-mcp"))
        .args(["--confirm", "--root"])
        .arg(root)
        .args(["--id", id, "--land"])
        .output()
        .unwrap()
        .status
        .code()
        .unwrap_or(-1)
}

/// R5: the copy's own `path = "../external-dep"` still resolves and the whole chain really runs.
/// R5：副本自己的 `path = "../external-dep"` 仍然解析得到，并且整条链真的跑起来。
#[test]
fn a_relative_path_dependency_still_builds_inside_the_candidate() {
    let (host, dependency) = fixture("relative", false);
    let id = opened(&host, "\"src\"");
    let (code, text) = call(
        &host,
        "xirang.candidate",
        &format!("{{\"action\":\"verify\",\"id\":\"{id}\",\"face\":\"default\"}}"),
    );
    assert_eq!(
        code, 0,
        "a candidate whose base resolves a relative dependency must verify: {text}"
    );
    assert!(text.contains("verification passed"), "{text}");
    assert_eq!(confirm_landing(&host, &id), 0);
    let (code, text) = call(
        &host,
        "xirang.candidate",
        &format!("{{\"action\":\"land\",\"id\":\"{id}\"}}"),
    );
    assert_eq!(code, 0, "the legal landing: {text}");
    // The base still builds and still uses the dependency.
    // 基树仍然构建，并且仍然用着那个依赖。
    let output = Command::new("cargo")
        .args(["test", "--offline"])
        .current_dir(&host)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "the landed base builds: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(dependency.join("src/lib.rs").exists());
}

/// R2: an out-of-root dependency is an input. If it changes after the verification, the landing is
/// refused — the review's failure was that it was accepted and the base's own test then failed.
/// R2：根外依赖也是输入。它在验证之后变了，落地就必须被拒——复核里的失败正是它被接受、而基树自己的测试随后失败。
#[test]
fn a_dependency_that_changed_after_the_verification_cannot_land() {
    let (host, dependency) = fixture("absolute", true);
    let id = opened(&host, "\"src\"");
    let (code, text) = call(
        &host,
        "xirang.candidate",
        &format!("{{\"action\":\"verify\",\"id\":\"{id}\",\"face\":\"default\"}}"),
    );
    assert_eq!(code, 0, "{text}");
    assert!(text.contains("verification passed"), "{text}");
    // The dependency's implementation changes; nothing in the candidate moves.
    // 依赖的实现变了；候选里一个字节都没动。
    std::fs::write(
        dependency.join("src/lib.rs"),
        "pub fn value() -> i64 {\n    2\n}\n",
    )
    .unwrap();
    assert_eq!(confirm_landing(&host, &id), 0);
    let (code, text) = call(
        &host,
        "xirang.candidate",
        &format!("{{\"action\":\"land\",\"id\":\"{id}\"}}"),
    );
    assert_ne!(code, 0, "a stale verification must not land: {text}");
    assert!(
        text.contains("moved since this candidate was opened") || text.contains("is not verified"),
        "the refusal says which baseline moved: {text}"
    );
    assert!(
        std::fs::read_to_string(host.join("src/lib.rs"))
            .unwrap()
            .contains("external_dep::value()")
    );
}
