//! Timing and queue pins for the slot's generation state machine.
//! 槽代际状态机的时序与排队钉子。
//!
//! These pins are the apparatus for audit `LGC-LG-47`: they drive
//! [`SlotState::activate_with`](super::SlotState::activate_with), the injectable load
//! seam, so the claim "the slot lock is not held while a generation compiles" is
//! measured rather than read off the code. `the_slot_lock_is_free_while_a_generation_compiles`
//! holds a load open on another thread and asks the slot — from this thread — whether
//! the queue is still reachable, and whether an `install` lands without waiting.
//! 这些钉子是审计 `LGC-LG-47` 的装置：它们驱动可注入加载的接缝
//! [`SlotState::activate_with`](super::SlotState::activate_with)，因此"编译期间不持槽锁"这条
//! 主张是被**测**出来的，而不是从代码里读出来的。`the_slot_lock_is_free_while_a_generation_compiles`
//! 在另一个线程上把一次加载按住，再从本线程问这个槽：队列是否仍然可达、`install` 是否会等。

use super::*;
use crate::run_method::{
    Admission, FlowContract, FrameworkId, LocalizedText, NodeId, ObjectContract, PluginArtifact,
    PluginManifest, PluginMode, PluginSource, PluginTrustPolicy, RegistrationInfo,
    RegistrationRule, RuntimeCheckSpec, SourceLocation, sha256_hex,
};

/// A verified artifact the queue can hold. It is never loaded here — the probe
/// drives the hand-off and the publish, which is where the audit's race lives.
/// 队列可以持有的一份已验证工件。这里从不加载它——探针驱动的是交接与发布，审计说的竞态就在
/// 那里。
fn artifact(bytes: Vec<u8>) -> VerifiedPluginArtifact {
    let checksum = Box::leak(sha256_hex(&bytes).into_boxed_str());
    let registration = RegistrationInfo {
        namespace: "plugin-test",
        id: NodeId::from_path("plugin.rs", "plugin"),
        parent: crate::run_method::ROOT_NODE_ID,
        kind: "Plugin",
        preset: "",
        parts: "",
        params: "",
        handle: "PluginHandle",
        stable_name: None,
        name: LocalizedText {
            zh: "插件",
            en: "Plugin",
        },
        summary: LocalizedText { zh: "", en: "" },
        exports: &[],
        needs_registry: false,
        registry_name: "plugin",
        getting_from_other_registry: None,
        registry_rule_path: "<test>",
        registry_rule: RegistrationRule::ANY,
        admission: Admission::ANY,
        requires: &[],
        provides: &[],
        contract: ObjectContract {
            required_parts: &[],
            provided_parts: &[],
        },
        flow: FlowContract::NONE,
        flow_provider: None,
        handle_traits: &[],
        part_traits: &[],
        runtime_checks: &[] as &[RuntimeCheckSpec],
        plugin: Some(PluginManifest {
            name: "plugin-test",
            crate_name: "plugin_test",
            version: "1.0.0",
            framework: FrameworkId::new("xirang.test"),
            source: PluginSource::User,
            mode: PluginMode::Extension,
            checksum,
            signature: None,
            public_key_fingerprint: None,
            revocation_list: None,
        }),
        source: SourceLocation {
            file: "plugin.rs",
            line: 1,
            column: 1,
            function: "plugin",
        },
    };
    PluginArtifact {
        registration,
        bytes,
        key_fingerprint: None,
    }
    .verify_artifact(PluginTrustPolicy::open())
    .expect("the probe artifact's digest verifies")
}

fn state() -> SlotState {
    SlotState {
        definition: WasmPluginSlot::new(
            "test",
            FrameworkId::new("xirang.test"),
            PluginMode::Extension,
            FlowContract::NONE,
            &[ValidationChannel::Local],
        ),
        active: ArcSwapOption::empty(),
        pending: Mutex::new(None),
        has_pending: AtomicBool::new(false),
        next_generation: AtomicU64::new(1),
        activation_error: Mutex::new(None),
    }
}

fn queue(state: &SlotState, generation: u64) {
    *state.pending.lock().expect("the queue lock") = Some(PendingPlugin {
        generation,
        artifact: artifact(vec![generation as u8]),
    });
    state.has_pending.store(true, Ordering::Release);
}

/// A generation installed while another one is being compiled stays pending: the
/// flag is published from the queue, not cleared blindly at the end of a load
/// (audit `LGC-LG-47`). Without the publish step this generation would be lost —
/// `call` would see `has_pending == false` and never activate it.
/// 在另一个代际正被编译时安装的代际会保持待定：标志从队列发布，而不是在加载结束时盲目清零
/// （审计 `LGC-LG-47`）。没有这一步，这个代际会丢失——`call` 会看到
/// `has_pending == false`，于是永远不会激活它。
#[test]
fn a_generation_queued_during_a_load_stays_pending() {
    let state = state();
    queue(&state, 1);
    let taken = state
        .take_pending()
        .expect("the queue lock is free")
        .expect("the queued generation is taken");
    assert_eq!(taken.generation, 1);
    assert!(
        state.pending.lock().expect("the queue lock").is_none(),
        "the queue is empty while the load runs, and the lock is not held"
    );

    // An install lands while generation 1 is still compiling.
    // 代际 1 仍在编译时落下一次安装。
    queue(&state, 2);
    // ...and the load finishes, successfully or not: both publish.
    // ……随后加载结束（无论成败都会发布）。
    state.publish_pending();
    assert!(
        state.has_pending.load(Ordering::Acquire),
        "a generation queued during the load is still pending"
    );

    // With nothing queued the flag clears, so `is_loaded` keeps its meaning.
    // 没有排队项时标志清零，`is_loaded` 因此保持它原有的含义。
    let _ = state.take_pending().expect("the queue lock is free");
    state.publish_pending();
    assert!(!state.has_pending.load(Ordering::Acquire));
}

/// `LGC-LG-47`, the lock-availability half: while one generation is being compiled the
/// queue is still reachable, and an `install` issued from another thread lands *before*
/// the load is released — an ordering oracle, not a duration threshold.
/// `LGC-LG-47` 的“锁可用”那一半：一个代际正在编译时队列仍然可达，而另一个线程发出的 `install`
/// 在加载被放行**之前**就落地了——这是顺序判据，不是时长阈值。
///
/// The load is held open through the injectable seam, which is the only way to observe it
/// on this machine. The judgment never compares against a wall-clock limit: the main
/// thread releases the load only after it has the install's answer, and the load reports
/// whether that release (rather than its own fallback) is what let it finish. A blocked
/// install is therefore a **false flag** here, not a hung test — the loader's fallback
/// releases the lock after two seconds, so the pre-fix shape fails this pin and the suite
/// still terminates.
/// 加载经可注入接缝按住，这是在本机上唯一能观察到它的办法。判据**不**与任何时长上限比较：主线程
/// 先拿到 install 的答复、才放行加载，而加载会回报究竟是这次放行（还是它自己的兜底）让它结束的。
/// 因此被阻塞的 install 在这里表现为**标志为假**，而不是把测试吊死——加载方的兜底会在两秒后释放锁，
/// 于修前的形状下这条钉子失败、而整个套件仍会结束。
#[test]
fn the_slot_lock_is_free_while_a_generation_compiles() {
    use std::sync::mpsc;
    use std::time::{Duration, Instant};

    let state = state();
    queue(&state, 1);
    let (entered_tx, entered_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel::<()>();
    let (install_tx, install_rx) = mpsc::channel();
    let (report_tx, report_rx) = mpsc::channel();
    let state = &state;
    let mut lock_was_free = false;
    let mut install_elapsed = Duration::MAX;
    let mut install_landed = false;

    std::thread::scope(|scope| {
        scope.spawn(move || {
            // Locals of this thread: the seam's closure must not need to reach back into
            // the slot (that would be the same-thread re-lock the seam documents).
            // 本线程的局部变量：接缝闭包不得回探本槽（那正是接缝文档里写的同线程重锁）。
            let mut released_by_signal = false;
            let outcome = state.activate_with(|_artifact| {
                entered_tx.send(()).expect("signal that the load started");
                // The fallback is what keeps the pre-fix shape a failure rather than a
                // hang: without it a blocked install would wait for a release only this
                // thread can send.
                // 兜底正是让"修前形状"表现为失败而不是吊死的原因：没有它，被阻塞的 install 会一直
                // 等一个只有本线程能发出的放行。
                released_by_signal = release_rx.recv_timeout(Duration::from_secs(2)).is_ok();
                Err(HostError::State("probe load refuses".to_owned()))
            });
            let _ = report_tx.send((
                released_by_signal,
                outcome.err().map(|error| error.to_string()),
            ));
        });

        // Everything below happens while that load is held open.
        // 下面的一切都发生在那次加载被按住的时候。
        entered_rx.recv().expect("the load started");
        lock_was_free = state.pending.try_lock().is_ok();
        scope.spawn(move || {
            let started = Instant::now();
            let landed = state
                .install(ValidationChannel::Local, artifact(vec![2]))
                .is_ok();
            let _ = install_tx.send((landed, started.elapsed()));
        });
        if let Ok((landed, elapsed)) = install_rx.recv() {
            install_landed = landed;
            install_elapsed = elapsed;
        }
        release_tx.send(()).expect("release the load");
    });
    let (released_by_signal, refused) = report_rx.recv().expect("the load reported");

    assert!(
        lock_was_free,
        "the queue lock is free while a generation compiles, so another thread can queue work"
    );
    assert!(
        install_landed,
        "an install lands during the load instead of waiting for the compile"
    );
    assert!(
        released_by_signal,
        "the install finished while the load was still held, not after its fallback released \
         it: install={install_elapsed:?}"
    );
    println!("PROBE install={install_elapsed:?} (the load was still held)");
    assert!(
        refused
            .as_deref()
            .is_some_and(|text| text.contains("probe load refuses")),
        "the refusal is reported to the caller: {refused:?}"
    );
    assert!(
        state.has_pending.load(Ordering::Acquire),
        "the generation queued during the load is still pending"
    );
}

/// `LGC-LG-47`, the "does not wait for the load" half, judged as a **ratio** rather than
/// against a wall-clock threshold: an install issued while the load compiles must cost a
/// small fraction of the load it overlaps (`install * 4 < load`), which is what "not
/// blocked" means when both numbers come from the same machine and the same run.
/// `LGC-LG-47` 的“不等加载”那一半，用**比例**而非时长阈值判定：在加载进行中发出的 install，
/// 其耗时必须只是它所重叠的那次加载的一小部分（`install * 4 < load`）——当两个数字来自同一台机器、
/// 同一次运行时，这就是"没有被阻塞"的含义。
///
/// The margin is the point: the pre-fix shape makes the install wait out the load, so the
/// ratio inverts (the review that asked for this measured `42.745 µs` against `400 ms`).
/// 余量就是重点：修前的形状会让 install 等完这次加载，比例随之翻转（要求改成比例式的那次复核测到
/// `42.745 µs` 对 `400 ms`）。
#[test]
fn an_install_does_not_wait_for_a_slow_load() {
    use std::sync::mpsc;
    use std::time::{Duration, Instant};

    const LOAD: Duration = Duration::from_millis(400);
    let state = state();
    queue(&state, 1);
    let (entered_tx, entered_rx) = mpsc::channel();
    let mut install_elapsed = Duration::MAX;
    let mut landed = false;
    let mut lock_was_free = false;

    std::thread::scope(|scope| {
        let entered = &entered_tx;
        let loader = scope.spawn(|| {
            let mut held = Duration::MAX;
            let outcome = state.activate_with(|_artifact| {
                entered.send(()).expect("signal that the load started");
                let started = Instant::now();
                std::thread::sleep(LOAD);
                held = started.elapsed();
                Err(HostError::State("slow probe load".to_owned()))
            });
            assert!(outcome.is_err(), "the probe load refuses");
            held
        });
        // The load is inside its sleep: the install below overlaps it for certain.
        // 加载正在它的 sleep 里：下面的 install 必定与它重叠。
        entered_rx.recv().expect("the load started");
        lock_was_free = state.pending.try_lock().is_ok();
        let started = Instant::now();
        landed = state
            .install(ValidationChannel::Local, artifact(vec![3]))
            .is_ok();
        install_elapsed = started.elapsed();
        let load_elapsed = loader.join().expect("the loader thread returns");
        println!("PROBE install={install_elapsed:?} load={load_elapsed:?}");
        assert!(
            install_elapsed * 4 < load_elapsed,
            "the install must cost a fraction of the load it overlaps: install={install_elapsed:?} \
             load={load_elapsed:?}"
        );
    });

    assert!(landed, "the install lands while the slow load runs");
    assert!(
        lock_was_free,
        "the queue lock is free while the slow load runs"
    );
}
