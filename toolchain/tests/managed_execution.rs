//! End-to-end pins for managed execution, run against the real binary and the real kernel.
//! 托管执行的端到端钉子，跑在真二进制与真内核上。

//! The ruling (§2) allows "unauthorized writes are prevented" only where the isolation actually holds, and it forbids
//! the plain bridge from saying it. These pins hold the shape that makes the sentence true — a write domain the kernel
//! enforces for a child and every process it starts — and they hold the two halves that a "policy" implementation
//! would quietly drop: a refused write is a kernel error rather than a sentence, and the publishing process is never
//! the confined one.
//! 裁定（§2）只允许在隔离真的成立的地方说出"越权写入已被阻止"，并禁止普通桥这么说。这些钉子守住让那句话为真的形状——由内核
//! 为一个子进程及它启动的每个进程执行的写域——也守住一个"政策式"实现会悄悄丢掉的两半：被拒的写入是内核错误而不是一句话，以及
//! 发布进程永远不是被关住的那一个。

#![cfg(feature = "mcp")]

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

/// The binary cargo built for this test.
/// 本测试对应的、由 cargo 构建出来的二进制。
fn binary() -> &'static str {
    env!("CARGO_BIN_EXE_xirang-mcp")
}

/// A fresh directory per test: the pins write real files, and sharing one would make them order-dependent.
/// 每条测试一个全新目录：钉子会写真实文件，共用一个目录会让它们依赖执行顺序。
fn fixture(label: &str) -> PathBuf {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "xirang-managed-{label}-{}-{unique}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::write(
        root.join("src/lib.rs"),
        "pub fn answer() -> i64 {\n    41\n}\n",
    )
    .unwrap();
    std::fs::create_dir_all(root.join("target/xirang")).unwrap();
    root
}

/// Run the bridge's managed entry the way a host would: the tree it must not write, the domain it may, then the
/// command. No `--managed-exec`: that is the confined half, and a host never starts it directly.
/// 像宿主那样运行桥的托管入口：它不许写的树、它可以写的域，然后是命令。不传 `--managed-exec`：那是被关住的那一半，
/// 宿主从不直接启动它。
fn managed(root: &Path, writable: &[&Path], script: &str) -> std::process::Output {
    let mut arguments: Vec<String> = vec!["--managed".to_owned(), "--root".to_owned()];
    arguments.push(root.display().to_string());
    for path in writable {
        arguments.push("--writable".to_owned());
        arguments.push(path.display().to_string());
    }
    arguments.push("--".to_owned());
    arguments.push("sh".to_owned());
    arguments.push("-c".to_owned());
    arguments.push(script.to_owned());
    Command::new(binary()).args(&arguments).output().unwrap()
}

/// The whole claim: a command started under `--managed` cannot write the tree it was told to leave alone — and the
/// refusal is the kernel's, so a `sh`, a `python3` or a build script inside it hits the same wall.
/// 整个主张：在 `--managed` 下启动的命令，写不了它被告知不许动的树——而拒绝来自内核，因此它里面跑的 `sh`、`python3`
/// 或构建脚本撞的是同一堵墙。
#[test]
fn a_confined_command_cannot_write_the_base_tree() {
    let root = fixture("denied");
    let victim = root.join("src/lib.rs");
    let before = std::fs::read_to_string(&victim).unwrap();
    let output = managed(
        &root,
        &[&root.join("target/xirang")],
        &format!("printf 'rewritten\\n' > {}", victim.display()),
    );
    assert!(
        !output.status.success(),
        "the shell reports the refused redirection as a failure: {output:?}"
    );
    assert_eq!(
        std::fs::read_to_string(&victim).unwrap(),
        before,
        "the file the command was told not to touch is byte-identical"
    );
    // The line says which guarantee is in force, so a reader does not have to infer it from the exit code.
    // 那一行说出哪一条保证在生效，读者不必从退出码去推断。
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("capability managed-landlock: ABI"),
        "the run states the guarantee it is under: {stdout}"
    );
    assert!(
        stdout.contains("refused by the kernel (EPERM)"),
        "and says the boundary is the kernel's, not a policy: {stdout}"
    );
    assert!(
        stdout.contains("not an identity check")
            && stdout.contains("publishing runs outside the domain"),
        "and states what it does not do: {stdout}"
    );
    let _ = std::fs::remove_dir_all(root);
}

/// The coverage the ruling asks the report to state rather than imply: the domain follows the **launch chain**, so a
/// process this wrapper did not start keeps full rights over the base tree — including one that is already running
/// while the managed command runs. A version that claimed "the system is confined" would fail here.
/// 裁定要求报告"说清楚而不是暗示"的覆盖范围：写域跟着**启动链**走，因此不是这个包装启动的进程对基树保有全部权限——包括在托管命令
/// 运行期间同时在跑的那一个。一个声称"整个系统被关住了"的版本会在这里失败。
#[test]
fn processes_the_launch_chain_did_not_start_keep_their_rights() {
    let root = fixture("sibling");
    let victim = root.join("src/lib.rs");
    let sibling = Command::new("sh")
        .arg("-c")
        .arg(format!(
            "sleep 0.4; printf 'sibling\\n' > {}",
            victim.display()
        ))
        .spawn()
        .unwrap();
    let output = managed(
        &root,
        &[&root.join("target/xirang")],
        &format!(
            "sleep 1; printf 'confined\\n' > {}",
            root.join("target/xirang/inside.txt").display()
        ),
    );
    let sibling = sibling.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "the managed command ran: {output:?}"
    );
    assert!(
        sibling.status.success(),
        "a process started outside the wrapper was not confined: {sibling:?}"
    );
    assert_eq!(std::fs::read_to_string(&victim).unwrap(), "sibling\n");
    let _ = std::fs::remove_dir_all(root);
}

/// The other half, in the same test as the refusal: a write **inside** the named domain succeeds. A mode that refused
/// everything would pass the pin above and be useless.
/// 另一半，与拒绝写在同一条测试里：**写域之内**的写入成功。一个什么都拒的模式能通过上面那条钉子，但毫无用处。
#[test]
fn a_confined_command_can_write_the_domain_it_was_given() {
    let root = fixture("allowed");
    let domain = root.join("target/xirang");
    let written = domain.join("allowed.txt");
    let output = managed(
        &root,
        &[&domain],
        &format!("printf 'ok\\n' > {}", written.display()),
    );
    assert!(output.status.success(), "{output:?}");
    assert_eq!(std::fs::read_to_string(&written).unwrap(), "ok\n");
    let _ = std::fs::remove_dir_all(root);
}

/// The restriction is inherited: a process the command starts is confined by the same ruleset, which is what stops the
/// obvious escape of "run the write in a child of the shell you gave me".
/// 限制会被继承：命令启动的进程受同一份规则集约束，这正是堵住"你给我的 shell 再开一个子进程去写"这个显然逃逸的原因。
#[test]
fn descendants_inherit_the_restriction() {
    let root = fixture("descendant");
    let victim = root.join("src/lib.rs");
    let before = std::fs::read_to_string(&victim).unwrap();
    let output = managed(
        &root,
        &[&root.join("target/xirang")],
        &format!("sh -c \"printf 'nested\\n' > {}\"", victim.display()),
    );
    assert!(!output.status.success(), "{output:?}");
    assert_eq!(std::fs::read_to_string(&victim).unwrap(), before);
    let _ = std::fs::remove_dir_all(root);
}

/// A host that cannot provide the isolation must be **refused**, not handed a weaker mode that runs the command
/// unconfined. The refusal is exercised through the one path a test can make deterministic on a capable kernel: a
/// domain that cannot be opened. What is asserted is the property, not the cause: no configuration failure ever ends in
/// "the command ran anyway".
/// 无法提供隔离的宿主必须被**拒绝**，而不是被交给一个把命令无隔离地跑起来的更弱模式。拒绝走的是在能用的内核上测试能确定触发的那条路：
/// 打不开的写域。断言的是性质而不是原因：任何配置失败都不会以"命令照样跑了"结束。
#[test]
fn a_domain_that_cannot_be_established_runs_nothing() {
    let root = fixture("no-domain");
    let proof = root.join("src/ran.txt");
    let missing = root.join("does-not-exist");
    let output = managed(
        &root,
        &[&missing],
        &format!("printf 'ran\\n' > {}", proof.display()),
    );
    assert_eq!(
        output.status.code(),
        Some(3),
        "a host without a domain is told so, with the distinct exit code: {output:?}"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("managed mode is not available"),
        "the refusal says what is missing: {stderr}"
    );
    assert!(
        !proof.exists(),
        "and the command did not run: a refused boundary must not become an unrestricted one"
    );
    let _ = std::fs::remove_dir_all(root);
}

/// Publishing is not part of the confined half: after a managed run, the **launcher's** process can still write the
/// base tree. That is the separation the ruling asks for — the thing that publishes is not the thing that was confined
/// — and a version that confined the bridge itself would fail here.
/// 发布不属于被关住的那一半：托管运行之后，**启动器**所在进程仍然能写基树。这就是裁定要求的分工——发布的那一方不是被关住的那一方
/// ——一个把桥自己也关住的版本会在这里失败。
#[test]
fn the_launcher_is_not_confined_by_the_run() {
    let root = fixture("launcher");
    let output = managed(&root, &[&root.join("target/xirang")], "true");
    assert!(output.status.success(), "{output:?}");
    let written = root.join("src/published.txt");
    std::fs::write(&written, "published\n")
        .expect("the launcher still owns the base tree's writes");
    assert_eq!(std::fs::read_to_string(&written).unwrap(), "published\n");
    let _ = std::fs::remove_dir_all(root);
}

/// The second half of the ruling's separation: the approval store lives **beside** the base tree
/// (`<root>/../.xirang-controller/…`), so a confined process cannot write it either. Without this pin the first half
/// would still be satisfied while a confined command forged its own approval — the 2026-10-17 review's exact failure,
/// only now behind a kernel wall instead of behind a promise.
/// 裁定要求的分离的另一半：批准存储住在基树**旁边**（`<root>/../.xirang-controller/…`），因此被关住的进程也写不了它。没有这条
/// 钉子，前一半仍会满足，而一个被关住的命令可以伪造自己的批准——正是 2026-10-17 复核的失败，只不过这回挡在前面的是内核的墙而不是
/// 一句承诺。
#[test]
fn a_confined_process_cannot_write_the_approval_store() {
    let root = fixture("approval-store");
    let store = root.parent().unwrap().join(".xirang-controller");
    let _ = std::fs::remove_dir_all(&store);
    let forged = store.join("forged/grant.json");
    let script = format!(
        "mkdir -p '{}' && printf 'forged\n' > '{}'; printf 'exit=%s\\n' \"$?\"",
        store.join("forged").display(),
        forged.display()
    );
    let output = managed(&root, &[&root.join("target/xirang")], &script);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !forged.exists(),
        "a confined process cannot create an approval record: {stdout}"
    );
    assert!(
        stdout.contains("refused by the kernel (EPERM)") || stdout.contains("exit=1"),
        "the refusal is the kernel's, not a message: {stdout}"
    );
    // The store is still writable by the launcher — the thing that publishes is the thing that may record approvals.
    // 启动器仍然写得了这个存储——发布的那一方才是可以记批准的那一方。
    std::fs::create_dir_all(&store).unwrap();
    std::fs::write(store.join("granted.json"), "{}\n").unwrap();
    assert!(store.join("granted.json").exists());
    let _ = std::fs::remove_dir_all(&store);
    let _ = std::fs::remove_dir_all(root);
}
