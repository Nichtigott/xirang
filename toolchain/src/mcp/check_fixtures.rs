//! Shared fixtures for the plan pins: nothing here carries a test of its own.
//! 计划钉子共用的夹具：这里没有任何东西自带测试。
//!
//! A test-support module is a shape this repository already ships (`tree/graft_ops/fixtures.rs`), and
//! it is deliberately test-free: `test_shape` judges a test-only file that *carries* a test, and the
//! size ratchet keeps the test budget for the pins rather than spending it on builders.
//! 测试支撑模块是本仓已经出厂的形状（`tree/graft_ops/fixtures.rs`），而且它有意不带测试：
//! `test_shape` 判定的是**自带测试**的仅测试文件，而尺寸棘轮把测试额度留给钉子，不花在构造函数上。

use std::path::PathBuf;

/// entry a caller uses.
/// 持久计划动作住在 `check_resume`；这些钉子经调用方所用的同一个入口驱动它们。
pub(crate) fn plan_action(
    root: &std::path::Path,
    arguments: &serde_json::Value,
) -> Result<String, String> {
    super::super::check_resume::plan_action(root, arguments)
}

pub(crate) fn fixture() -> PathBuf {
    let root =
        std::env::temp_dir().join(format!("xirang-plan-{}", crate::mcp::check_jobs::new_id()));
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::create_dir_all(root.join("tests")).unwrap();
    std::fs::write(root.join("Cargo.toml"), "[package]\nname='plan-probe'\nversion='0.0.0'\nedition='2024'\n[workspace]\n[features]\nbypass=[]\nbad-build=[]\nunrelated-break=[]\nslow=[]\nsibling=[]\nambiguous=[]\n").unwrap();
    std::fs::write(root.join("src/lib.rs"), "pub fn provider() -> i32 { 3 }\n#[test]\nfn contract() { assert_eq!(if cfg!(feature=\"bypass\") { 1 } else { provider() }, 3); }\n#[cfg(feature=\"unrelated-break\")]\n#[test]\nfn unrelated() { assert_eq!(\"unrelated\", \"failure\"); }\n#[cfg(feature=\"slow\")]\n#[test]\nfn slow_target() { std::thread::sleep(std::time::Duration::from_secs(30)); }\n#[cfg(feature=\"sibling\")]\nmod unrelated { #[test] fn contract() { panic!(\"EXPECTED_ROUTE_VIOLATION\"); } }\n#[cfg(feature=\"ambiguous\")]\n#[test]\nfn shared() { assert_eq!(3, 3); }\n#[cfg(feature=\"bad-build\")]\nconst WRONG: u32 = \"wrong\";\n").unwrap();
    // The same full test name in a second Cargo target, so a name-only selector is ambiguous.
    // 第二个 Cargo target 里出现同一个完整测试名，因此只给名字的选择器是有歧义的。
    std::fs::write(root.join("tests/right.rs"), "#[cfg(feature = \"ambiguous\")]\n#[test]\nfn shared() { panic!(\"EXPECTED_ROUTE_VIOLATION\"); }\n").unwrap();
    let output = std::process::Command::new("cargo")
        .args(["generate-lockfile", "--offline"])
        .current_dir(&root)
        .output()
        .unwrap();
    assert!(output.status.success());
    root
}

/// The same fixture with one test marked `#[ignore]`: a run of it executes the others and **skips** that one, which is
/// the observation the anomaly dimension needs (a pass that did not exercise everything there was).
/// 同一个夹具，但有一个测试标了 `#[ignore]`：跑它时其余测试执行、那一个被**跳过**，而那正是异常维度需要的观测
/// （一次没有把它该跑的全都跑通的通过）。
pub(crate) fn skipped_fixture() -> PathBuf {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "xirang-plan-skipped-{}-{sequence}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::write(
        root.join("Cargo.toml"),
        "[package]\nname='skipped-probe'\nversion='0.0.0'\nedition='2024'\n[workspace]\n",
    )
    .unwrap();
    std::fs::write(
        root.join("src/lib.rs"),
        "pub fn provider() -> i32 { 3 }\n#[test]\nfn contract() { assert_eq!(provider(), 3); }\n#[test]\n#[ignore]\nfn never_exercised() { assert_eq!(provider(), 4); }\n",
    )
    .unwrap();
    let output = std::process::Command::new("cargo")
        .args(["generate-lockfile", "--offline"])
        .current_dir(&root)
        .output()
        .unwrap();
    assert!(output.status.success());
    root
}

/// A package whose one test takes long enough that a cancellation can land inside it.
/// 一个测试跑得足够久、足以让一次取消落进去的包。
pub(crate) fn slow_fixture() -> PathBuf {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "xirang-plan-slow-{}-{sequence}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::write(
        root.join("Cargo.toml"),
        "[package]\nname='slow-probe'\nversion='0.0.0'\nedition='2024'\n[workspace]\n",
    )
    .unwrap();
    std::fs::write(
        root.join("src/lib.rs"),
        "pub fn provider() -> i32 { 3 }\n#[test]\nfn contract() { std::thread::sleep(std::time::Duration::from_millis(1500)); assert_eq!(provider(), 3); }\n",
    )
    .unwrap();
    let output = std::process::Command::new("cargo")
        .args(["generate-lockfile", "--offline"])
        .current_dir(&root)
        .output()
        .unwrap();
    assert!(output.status.success());
    root
}

/// a chain of three hops through both families lands on the value of a single hop.
/// 这一对转换会相互抵消（`to_local` 加、`to_world` 减同一个量），因此穿过两族的三层跳转落回"一层"的取值。
/// A host with one shared conversion provider and three consumers that each reach it differently.
/// 一个宿主：一个共享的转换提供者，以及三个各以不同方式到达它的消费者。
///
/// The conversions are a cancelling pair (`to_local` adds, `to_world` subtracts the same amount), so a
/// chain of three hops through both families lands on the value of a single hop.
/// 这一对转换会相互抵消（`to_local` 加、`to_world` 减同一个量），因此穿过两族的三层跳转落回"一层"的取值。
pub(crate) fn conversion_host(label: &str) -> PathBuf {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "xirang-plan-conversion-{label}-{}-{sequence}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::write(
        root.join("Cargo.toml"),
        "[package]\nname='conversion-probe'\nversion='0.0.0'\nedition='2024'\n[workspace]\n",
    )
    .unwrap();
    std::fs::write(
        root.join("src/lib.rs"),
        "pub mod provider;\npub mod button;\npub mod slider;\npub mod timeline;\n",
    )
    .unwrap();
    std::fs::write(
        root.join("src/provider.rs"),
        "pub fn to_local(x: i32) -> i32 { x + 100 }\npub fn to_world(x: i32) -> i32 { x - 100 }\n",
    )
    .unwrap();
    for (module, body) in [
        ("button", "crate::provider::to_local(x)"),
        (
            "slider",
            "crate::provider::to_local(crate::provider::to_world(crate::provider::to_local(x)))",
        ),
        ("timeline", "crate::provider::to_world(x)"),
    ] {
        std::fs::write(
            root.join(format!("src/{module}.rs")),
            format!(
                "/// This consumer's offset, in the family its siblings use.\npub fn offset(x: i32) -> \
                 i32 {{ {body} }}\n\n#[test]\nfn contract_{module}() {{ assert_eq!(offset(0), 100); }}\n"
            ),
        )
        .unwrap();
    }
    let output = std::process::Command::new("cargo")
        .args(["generate-lockfile", "--offline"])
        .current_dir(&root)
        .output()
        .unwrap();
    assert!(output.status.success());
    root
}

/// The task key a `status` reply prints.
/// `status` 回复打印的任务键。
pub(crate) fn key_of(status: &str) -> String {
    status
        .lines()
        .find_map(|line| line.strip_prefix("task "))
        .unwrap_or_else(|| panic!("the status names its task key: {status}"))
        .to_owned()
}

/// Start a verification job, waiting for capacity when the server's job slots are all taken.
/// 启动一个验证任务；服务端的任务槽位都占着时等一等容量。
///
/// The bound is real (four jobs, measured on the server), and **a pin that starts a job therefore
/// depends on a global resource**: under a parallel test run another pin's job can hold the last
/// slot. Waiting is what a real caller does with this refusal, so that is what the pin does — rather
/// than widening the assertion or serialising the whole suite.
/// 这个上限是真的（服务端实测四个任务），因此**启动任务的钉子依赖一个全局资源**：并行跑测试时，另一个
/// 钉子的任务可能占着最后一个槽。真实调用方面对这条拒绝就是等，钉子也照做——而不是把断言放宽或让整个套件
/// 串行。
pub(crate) fn start_when_a_slot_frees(
    root: &std::path::Path,
    request: &serde_json::Value,
) -> String {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
    loop {
        match super::super::check_jobs::check(root, request) {
            Ok(reply) => return reply,
            Err(error)
                if error.contains("already running") && std::time::Instant::now() < deadline =>
            {
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
            Err(error) => panic!("the job could not start: {error}"),
        }
    }
}
