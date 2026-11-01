//! Pins for the publish lock: one writer per tree, a bounded wait, and a leftover that is taken over.
//! 发布锁的钉子：一棵树一个写者、有界等待、以及被接管的残留。

/// What the child process prints so the parent can tell "it ran" from "nothing matched".
/// 子进程打印的东西，好让父进程把"它跑了"与"什么都没匹配上"区分开。
const CHILD_MARKER: &str = "N49-CHILD";

use std::path::Path;
use std::time::{Duration, Instant};

use super::{DEFAULT_WAIT_MS, acquire_within, path_for};

/// A throwaway output directory, the same shape `check_for` receives.
/// 一个一次性的输出目录，与 `check_for` 收到的形状相同。
fn out_dir(label: &str) -> std::path::PathBuf {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    // Each case gets its **own container**, because the lock lives beside the output directory: a
    // shared parent would make every fixture (and every parallel test) contend for one lock file.
    // 每个用例有自己的**容器**，因为锁住在输出目录旁边：共用一个父目录会让所有夹具（以及并行的每个
    // 测试）争抢同一把锁文件。
    let container = std::env::temp_dir()
        .join("xirang-scratch")
        .join(module_path!().replace("::", "-"))
        .join(format!(
            "xirang-lock-{label}-{}-{sequence}",
            std::process::id()
        ));
    let _ = std::fs::remove_dir_all(&container);
    let dir = container.join("out");
    std::fs::create_dir_all(&dir).expect("out dir");
    dir
}

/// A taken lock exists, names its holder, and is gone once the guard is dropped.
/// 取到的锁存在、点出自己的持有者，而守卫被丢弃后它就没了。
#[test]
fn a_lock_is_taken_and_released() {
    let out = out_dir("take");
    let path = path_for(&out);
    assert!(!path.exists(), "nothing holds it yet");
    {
        let lock = acquire_within(&out, Duration::from_millis(50)).expect("the tree is free");
        assert!(path.exists(), "the lock file is what holds the tree");
        assert_eq!(lock.stolen_from, None, "nothing was stolen");
        let text = std::fs::read_to_string(&path).expect("readable");
        assert!(
            text.contains(&format!("pid\t{}", std::process::id())),
            "the holder names itself: {text}"
        );
    }
    assert!(!path.exists(), "dropping the guard releases the tree");
}

/// A second writer waits for the first instead of publishing beside it.
/// 第二个写者等第一个，而不是在它旁边发布。
#[test]
fn a_second_writer_waits_and_then_gets_the_tree() {
    let out = out_dir("wait");
    let held = acquire_within(&out, Duration::from_millis(50)).expect("the tree is free");
    let holder = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(150));
        drop(held);
    });
    let start = Instant::now();
    let second = acquire_within(&out, Duration::from_secs(5)).expect("the wait pays off");
    let waited = start.elapsed();
    holder.join().expect("the holder thread finishes");
    assert!(
        waited >= Duration::from_millis(100),
        "it really waited ({waited:?})"
    );
    assert_eq!(
        second.stolen_from, None,
        "a live holder is waited for, never taken over"
    );
}

/// A live holder is refused **by name** once the budget runs out — and the refusal is actionable.
/// 活着的持有者在预算耗尽后被**点名**拒绝——而且那句拒绝可照做。
#[test]
fn a_live_holder_is_refused_by_name() {
    let out = out_dir("refuse");
    let path = path_for(&out);
    // Our own pid is alive by construction, so this is a holder the platform can see.
    // 我们自己的 pid 按构造是活着的，因此这是一个平台看得见的持有者。
    std::fs::write(&path, format!("pid\t{}\nstarted\t0\n", std::process::id())).expect("a holder");
    let refused = acquire_within(&out, Duration::from_millis(60)).expect_err("refused");
    assert!(
        refused.contains(&path.display().to_string()),
        "the refusal names the file to clear: {refused}"
    );
    assert!(
        refused.contains(&format!("pid {}", std::process::id())),
        "the refusal names the holder: {refused}"
    );
    assert!(
        refused.contains("way forward") && refused.contains(xirang_kernel::lexicon::LOCK_WAIT_ENV),
        "the refusal is actionable and names the bound: {refused}"
    );
    assert!(
        path.exists(),
        "a refusal does not clear somebody else's lock"
    );
}

/// A lock whose holder is gone is taken over, and the takeover is reported.
/// 持有者已消失的锁会被接管，而且接管这件事被报出来。
#[test]
fn a_gone_holders_lock_is_taken_over() {
    let out = out_dir("stale");
    let path = path_for(&out);
    // A pid no process can have (Linux caps at 2^22; this is far above any cap and below u32::MAX so
    // it parses), so "the holder is alive" is false on every platform that can answer at all.
    // 一个任何进程都不可能的 pid（Linux 上限 2^22；这个远高于任何上限、又低于 u32::MAX 因而能解析），
    // 因此在任何能回答的平台上"持有者活着"都是假。
    std::fs::write(&path, "pid\t4000000000\nstarted\t0\n").expect("a leftover");
    // Whether a pid is alive is answered through `/proc`, which only Linux has; where it does not
    // exist the rule is to **assume alive** rather than steal a lock from a live writer
    // (`process_is_alive`), so the same leftover is a refusal there — and the refusal is what this
    // pin checks on those platforms, instead of asserting a takeover the platform cannot perform.
    // pid 是否活着经 `/proc` 回答，而只有 Linux 有它；它不存在时规则是**假定活着**，而不是从活写者手里夺锁
    // （`process_is_alive`），因此同样的残留在那里是一句拒绝——在这些平台上本钉子检查的就是那句拒绝，而不是
    // 断言一次该平台做不到的接管。
    if Path::new("/proc").is_dir() {
        let lock = acquire_within(&out, Duration::from_millis(200)).expect("taken over");
        assert_eq!(
            lock.stolen_from,
            Some(4_000_000_000),
            "the takeover is reported"
        );
        assert!(
            std::fs::read_to_string(&path)
                .expect("readable")
                .contains(&format!("pid\t{}", std::process::id())),
            "the file now names this run"
        );
    } else {
        let refusal = acquire_within(&out, Duration::from_millis(50)).expect_err("refused");
        assert!(
            refusal.contains("4000000000") && refusal.contains(".publishing.lock"),
            "the refusal names the file and the pid it could not clear: {refusal}"
        );
        assert!(
            path.exists(),
            "and the lock it refused to clear is still there"
        );
    }
}

/// A lock that names no holder is honoured while it is young: with no pid to check, age is the only
/// evidence, and stealing a fresh one would take the tree from a run merely older than this code.
/// 未点出持有者的锁在年轻时被尊重：没有 pid 可查，年龄是唯一证据，而夺走一把新鲜的锁等于从只是比本代码
/// 更早的一次运行手里拿走这棵树。
#[test]
fn an_unreadable_fresh_lock_is_honoured() {
    let out = out_dir("unreadable");
    let path = path_for(&out);
    std::fs::write(&path, "someone else's note\n").expect("a note");
    let refused = acquire_within(&out, Duration::from_millis(60)).expect_err("refused");
    assert!(
        refused.contains("unreadable"),
        "the refusal says what it could see: {refused}"
    );
    assert!(path.exists(), "it is left alone");
}

/// The waiting budget defaults and is read from the environment, because that is the only bound a
/// caller that cannot pass one has (a build script).
/// 等待预算有默认值并且从环境读，因为那是拿不到参数的调用方（构建脚本）唯一能有的界。
#[test]
fn the_wait_budget_comes_from_the_environment() {
    // The variable is process-wide, so this test owns it alone; the default is asserted through the
    // constant rather than by unsetting what another test may have set.
    // 这个变量是进程级的，因此本测试独占它；默认值经常量断言，而不是去取消别的测试可能设过的值。
    // SAFETY: this test is the only one that touches the variable, and it restores nothing because it
    // ends immediately after reading it.
    // 安全：本测试是唯一碰这个变量的，而且读完立刻结束，所以无需还原。
    unsafe { std::env::set_var(xirang_kernel::lexicon::LOCK_WAIT_ENV, "1234") };
    assert_eq!(super::wait_budget(), Duration::from_millis(1234));
    assert_eq!(DEFAULT_WAIT_MS, 30_000, "the documented default");
    unsafe { std::env::remove_var(xirang_kernel::lexicon::LOCK_WAIT_ENV) };
}

/// A fixture host with one registration face, and its output directory.
/// 一个含单个注册面的夹具宿主，以及它的输出目录。
fn fixture_host(label: &str) -> (std::path::PathBuf, std::path::PathBuf) {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let root = std::env::temp_dir()
        .join("xirang-scratch")
        .join(module_path!().replace("::", "-"))
        .join(format!(
            "xirang-lock-host-{label}-{}-{sequence}",
            std::process::id()
        ));
    let _ = std::fs::remove_dir_all(&root);
    let manifest = root.join("host");
    std::fs::create_dir_all(manifest.join("src/button")).expect("src");
    std::fs::write(
        manifest.join("Cargo.toml"),
        "[package]\nname = \"lock-host\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .expect("manifest");
    std::fs::write(manifest.join("src/lib.rs"), "// host entry\n").expect("lib");
    std::fs::write(
        manifest.join("src/button/button.rs"),
        "pub struct Button;\n\ncrate::root_object! {\n    kind: Button,\n    parent: crate::root_node_id(crate::XIRANG_NAMESPACE),\n}\n",
    )
    .expect("face");
    let out = root.join("out");
    (manifest, out)
}

/// A run that cannot take the tree's lock publishes **nothing** — no half generation beside another
/// run's files — and says which lock to clear.
/// 拿不到这棵树锁的运行**什么都不发布**——不会在另一次运行的文件旁边留下半代产物——并说清该清哪把锁。
///
/// The wait budget is read from the environment, and the environment is process-wide, so the run
/// happens in a child process: setting it here would race every other publish in this test binary.
/// 等待预算从环境读，而环境是进程级的，因此这次运行发生在子进程里：在这里设置它会与本测试二进制里
/// 其它每一次发布抢跑。
#[test]
fn a_held_lock_stops_the_publish_and_names_itself() {
    let (manifest, out) = fixture_host("held");
    let lock = path_for(&out);
    std::fs::create_dir_all(out.parent().expect("a parent")).expect("parent");
    std::fs::create_dir_all(&out).expect("out dir");
    // Our own pid holds it: alive by construction.
    // 我们自己的 pid 持有它：按构造是活着的。
    std::fs::write(&lock, format!("pid\t{}\nstarted\t0\n", std::process::id())).expect("a holder");

    let (stdout, stderr) = publish_in_child(&manifest, &out).expect("the child runs");
    let report = format!("{stdout}{stderr}");
    // The child has to **say** it ran: libtest's exit code reflects the test, not the publish, and a
    // child that matched no test at all also exits zero (`0 passed` is nothing ran, audit lesson).
    // 子进程必须**说出**它跑了：libtest 的退出码反映的是测试而不是发布，而一个什么测试都没匹配上的子进程
    // 同样退 0（`0 passed` 就是什么都没跑，见审计教训）。
    assert!(
        report.contains(CHILD_MARKER),
        "the child did not report a verdict: {report}"
    );
    assert!(
        report.contains("refused"),
        "the publish is refused, not attempted: {report}"
    );
    // One separator convention for the comparison: the report prints the path with the platform's
    // own separators, and `lock.display()` does the same, so on Windows both have to be flattened
    // before `contains` can mean anything.
    // 比较时统一一种分隔符：报告用平台自己的分隔符打印路径，`lock.display()` 也一样，因此在 Windows 上
    // 两者都得先拍平，`contains` 才有意义。
    let flatten = |text: &str| text.replace("\\\\", "/").replace('\\', "/");
    let flattened = flatten(&report);
    let lock_text = flatten(&lock.display().to_string());
    assert!(
        report.contains("publish-lock") && flattened.contains(&lock_text),
        "the run names the lock it could not take: {report}"
    );
    assert!(
        !out.join(xirang_kernel::lexicon::GENERATION_FILE).exists(),
        "nothing was published while another run holds the tree"
    );
    assert!(
        !out.join("pruning_manifest.tsv").exists(),
        "not even the first payload"
    );

    // Positive control: with the lock cleared, the same fixture publishes — so the refusal above is
    // about the lock and not about the host.
    // 正对照：清掉锁之后同一个夹具能发布——因此上面那句拒绝是关于锁的，不是关于这个宿主的。
    std::fs::remove_file(&lock).expect("clear the holder");
    let (stdout, stderr) = publish_in_child(&manifest, &out).expect("the child runs");
    let report = format!("{stdout}{stderr}");
    assert!(
        report.contains(CHILD_MARKER) && report.contains("published"),
        "the same host publishes once the tree is free: {report}"
    );
    assert!(out.join("pruning_manifest.tsv").exists(), "payloads landed");
    let _ = std::fs::remove_dir_all(&out);
}

/// Publish `out` from a **child process** with a one-millisecond wait budget, returning whether it
/// succeeded, plus the child's streams.
/// 在**子进程**里以 1 毫秒的等待预算发布 `out`，返回它是否成功，以及子进程的两条流。
fn publish_in_child(manifest: &std::path::Path, out: &std::path::Path) -> Option<(String, String)> {
    let output = std::process::Command::new(std::env::current_exe().expect("the test binary"))
        .args([
            "--ignored",
            "--exact",
            "build_method::publish_lock::publish_lock_tests::lock_child_publishes",
            "--nocapture",
        ])
        .env(xirang_kernel::lexicon::LOCK_WAIT_ENV, "1")
        .env("N49_MANIFEST", manifest)
        .env("N49_OUT", out)
        .output()
        .ok()?;
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    Some((stdout, stderr))
}

/// The child half of the pin above; it prints the diagnostics a refused publish produced.
/// 上面那条钉子的子进程那一半；它打印被拒的发布产生的诊断。
#[test]
#[ignore = "child of a_held_lock_stops_the_publish_and_names_itself"]
fn lock_child_publishes() {
    let manifest = std::path::PathBuf::from(std::env::var("N49_MANIFEST").expect("manifest"));
    let out = std::path::PathBuf::from(std::env::var("N49_OUT").expect("out"));
    let name = "lock-host";
    match crate::build_method::check_for(&manifest, &out, name) {
        Ok(()) => println!("{CHILD_MARKER} published"),
        Err(diagnostics) => println!("{CHILD_MARKER} refused {diagnostics:?}"),
    }
}
