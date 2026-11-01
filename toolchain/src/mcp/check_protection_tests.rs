//! Pins for the consumer-protection duty, driven through the real plan entry.
//! 消费者保护义务的钉子，从真实的计划入口驱动。
//!
//! The point is not "the census printed something" but that a missing consumer test **keeps a plan
//! open** while the same tree with that test closes it — a note that never changed a verdict would
//! have left the measured gap exactly where it was.
//! 要点不是"普查印了东西"，而是缺一个消费者测试会让计划**保持开启**，而同一棵树上补上那个测试就会结清它
//! ——一条从不改变判定的注记会把实测到的缺口原样留在那里。

use crate::mcp::check_plan::run;
use serde_json::{Value, json};
use std::path::PathBuf;

/// One shared implementation in its own module, two consumers, and (optionally) a test per consumer.
/// 一个自成一模块的共享实现、两个消费者，以及（可选）每个消费者一个测试。
fn fixture() -> PathBuf {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "xirang-protection-{}-{sequence}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::create_dir_all(root.join("tests")).unwrap();
    std::fs::write(
        root.join("Cargo.toml"),
        "[package]\nname='protection-probe'\nversion='0.0.0'\nedition='2024'\n[workspace]\n",
    )
    .unwrap();
    std::fs::write(
        root.join("src/lib.rs"),
        "pub mod arithmetic;\npub mod first;\npub mod second;\n",
    )
    .unwrap();
    std::fs::write(
        root.join("src/arithmetic.rs"),
        "/// Round up to a whole step.\npub fn settle(n: i64) -> i64 {\n    n.max(0)\n}\n",
    )
    .unwrap();
    std::fs::write(
        root.join("src/first.rs"),
        "pub fn first_round(n: i64) -> i64 {\n    crate::arithmetic::settle(n)\n}\n",
    )
    .unwrap();
    std::fs::write(
        root.join("src/second.rs"),
        "pub fn second_round(n: i64) -> i64 {\n    crate::arithmetic::settle(n)\n}\n",
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

/// The ledger one plan wrote, read back from its own reply.
/// 一份计划写下的台账，从它自己的回复读回。
fn ledger_of(reply: &str) -> Value {
    let record = reply
        .lines()
        .find_map(|line| line.strip_prefix("record "))
        .unwrap_or_else(|| panic!("no record line in:\n{reply}"));
    serde_json::from_str(&std::fs::read_to_string(record).unwrap()).unwrap()
}

/// BR07: one uncovered consumer out of two keeps the plan open, and the row is named. This is the
/// measured shape of the basic-repair delivery that shipped an arithmetic unit test and no
/// consumer-level test.
/// BR07：两个消费者里有一个没被覆盖，计划就保持开启，而且那一行被点名。这正是基础修复交付里"只加了
/// arithmetic 单元测试、没有消费者级测试"的实测形状。
#[test]
fn one_uncovered_consumer_of_a_shared_provider_keeps_the_plan_open() {
    let root = fixture();
    std::fs::write(
        root.join("tests/first.rs"),
        "#[test]\nfn first_round_is_protected() {\n    assert_eq!(protection_probe::first::first_round(-4), 0);\n}\n",
    )
    .unwrap();
    let reply = run(
        &root,
        &json!({"files":["src/arithmetic.rs"],"face":"default"}),
    )
    .unwrap();
    assert!(reply.starts_with("obligations open"), "{reply}");
    assert!(
        reply.contains("consumer protection: src/second.rs::second_round: unknown"),
        "{reply}"
    );
    assert!(
        reply.contains("consumer protection: src/first.rs::first_round: passed"),
        "{reply}"
    );
    assert!(reply.contains("remaining obligations:"), "{reply}");
    assert!(reply.contains("second_round: unknown"), "{reply}");

    let ledger = ledger_of(&reply);
    let protection = &ledger["protection"];
    assert_eq!(
        protection["shared_providers"],
        json!(["src/arithmetic.rs::settle"]),
        "{ledger}"
    );
    assert_eq!(
        protection["uncovered"],
        json!(["src/second.rs::second_round"])
    );
    assert!(
        protection["note"]
            .as_str()
            .is_some_and(|note| note.contains("a test that exists but never executes")),
        "the duty must say what does not close it: {protection}"
    );
    std::fs::remove_dir_all(root).unwrap();
}

/// BR07, the other half: a test per consumer closes the duty, and the plan reads `passed` again.
/// BR07 的另一半：每个消费者一个测试就结清义务，计划重新读作 `passed`。
#[test]
fn a_test_per_consumer_closes_the_duty_without_adding_a_cargo_obligation() {
    let root = fixture();
    for (name, module, call, value) in [
        ("first", "first", "first_round", -4),
        ("second", "second", "second_round", -9),
    ] {
        std::fs::write(
            root.join(format!("tests/{name}.rs")),
            format!(
                "#[test]\nfn {module}_round_is_protected() {{\n    assert_eq!(protection_probe::{module}::{call}({value}), 0);\n}}\n"
            ),
        )
        .unwrap();
    }
    let reply = run(
        &root,
        &json!({"files":["src/arithmetic.rs"],"face":"default"}),
    )
    .unwrap();
    assert!(reply.starts_with("obligations passed"), "{reply}");
    assert_eq!(
        reply.matches("consumer protection: ").count(),
        2,
        "both entries are reported, not only the ones that were open: {reply}"
    );
    assert!(!reply.contains("needs_evidence"), "{reply}");
    std::fs::remove_dir_all(root).unwrap();
}

/// BR07: a test that calls the **provider** directly is not protection for a consumer's delegation,
/// and a provider with a single consumer is not a shared implementation at all.
/// BR07：直接调用**提供者**的测试不是对消费者委派的保护；而只有一个消费者的提供者根本不是共享实现。
#[test]
fn a_direct_test_of_the_provider_and_a_single_consumer_are_both_insufficient() {
    let root = fixture();
    std::fs::write(
        root.join("tests/arithmetic.rs"),
        "#[test]\nfn the_provider_is_clamped() {\n    assert_eq!(protection_probe::arithmetic::settle(-4), 0);\n}\n",
    )
    .unwrap();
    let reply = run(
        &root,
        &json!({"files":["src/arithmetic.rs"],"face":"default"}),
    )
    .unwrap();
    assert!(reply.starts_with("obligations open"), "{reply}");
    assert!(
        reply.contains(
            "A test that calls `settle` directly protects the implementation, not this consumer."
        ),
        "{reply}"
    );

    // Removing one consumer leaves a provider called from one file: not shared, so no duty either way.
    // 去掉一个消费者后，提供者只被一个文件调用：不是共享，因此两个方向都没有义务。
    std::fs::write(
        root.join("src/second.rs"),
        "pub fn second_round(n: i64) -> i64 {\n    n\n}\n",
    )
    .unwrap();
    let single = run(
        &root,
        &json!({"files":["src/arithmetic.rs"],"face":"default"}),
    )
    .unwrap();
    assert!(!single.contains("consumer protection: "), "{single}");
    let ledger = ledger_of(&single);
    assert_eq!(
        ledger["protection"]["shared_providers"],
        json!([]),
        "{ledger}"
    );
    assert!(
        ledger["protection"]["note"]
            .as_str()
            .is_some_and(|note| note.contains("not a shared root cause")),
        "{ledger}"
    );
    std::fs::remove_dir_all(root).unwrap();
}

/// EX03: one file holds both a writable and a protected object, and the protection is keyed on the
/// **file**. That is what keeps a protected change from hiding behind a writable one: an edit that
/// touches only the writable half still raises every consumer of the shared provider defined beside
/// it, while a file that defines no shared provider raises no duty — so a legal change still lands
/// without a protection obligation it never earned.
/// The alternative reading (deciding **which object inside the file** changed) is a different feature,
/// not this granularity: `delta` is a file-level record, so it cannot carry that distinction, and a
/// surface that guessed it from a diff would be inventing evidence. This pin states the reading it
/// verifies instead of leaving it implicit.
/// EX03：同一文件里既有可写对象也有受保护对象，而保护按**文件**守恒——这正是保护半边的改动不会躲进
/// 可写半边的原因：只改可写半边的编辑仍会举起同文件里共享提供者的每个消费者义务，而不定义共享提供者的
/// 文件不举义务，因此合法改动不必背上它从未获得的保护义务。另一种读法（判定文件内**哪个对象**变了）是
/// 另一个特性而不是这里的粒度：`delta` 是文件级记录，载不动那个区分，而凭空从差异里猜的读面只是在造证据。
/// 这条钉子把它验证的读法写出来，而不是留着让人默认。
#[test]
fn a_change_beside_a_shared_provider_keeps_that_providers_duty_and_a_file_without_one_raises_none()
{
    let root = fixture();
    // The writable half: a helper in the provider's own file that no other file calls. `settle` is
    // byte-identical to what it was, so a reader that compared provider bodies would say "untouched".
    // 可写半边：提供者自己文件里一个别的文件都不调用的辅助函数。`settle` 逐字节未变，因此一个比对提供者
    // 函数体的读法会说"没动过"。
    std::fs::write(
        root.join("src/arithmetic.rs"),
        "/// Round up to a whole step.\npub fn settle(n: i64) -> i64 {\n    n.max(0)\n}\n\n/// Only this file uses it.\npub fn label() -> &'static str {\n    \"the writable half\"\n}\n",
    )
    .unwrap();
    let reply = run(
        &root,
        &json!({"files":["src/arithmetic.rs"],"face":"default"}),
    )
    .unwrap();
    let ledger = ledger_of(&reply);
    assert_eq!(
        ledger["protection"]["shared_providers"],
        json!(["src/arithmetic.rs::settle"]),
        "{ledger}"
    );
    assert_eq!(
        ledger["protection"]["uncovered"],
        json!(["src/first.rs::first_round", "src/second.rs::second_round"]),
        "the duty follows the file, not the function body: {ledger}"
    );

    // The other half: a change in a consumer file defines no provider, so there is nothing to protect
    // and no duty is fabricated for it.
    // 另一半：消费者文件里的改动不定义提供者，因此没有东西要保护，也不会为它造出义务。
    std::fs::write(
        root.join("src/first.rs"),
        "pub fn first_round(n: i64) -> i64 {\n    crate::arithmetic::settle(n) + 1\n}\n",
    )
    .unwrap();
    let reply = run(&root, &json!({"files":["src/first.rs"],"face":"default"})).unwrap();
    let ledger = ledger_of(&reply);
    assert_eq!(
        ledger["protection"]["shared_providers"],
        json!([]),
        "a consumer file defines no shared provider: {ledger}"
    );
    assert!(!reply.contains("consumer protection: "), "{reply}");
    std::fs::remove_dir_all(root).unwrap();
}
