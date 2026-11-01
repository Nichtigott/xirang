//! Tests for the disk-backed ordinary-source proof: what it writes, what it accepts, and what it
//! refuses to call a proof.
//! 磁盘持有的普通源码证明的测试：它写什么、接受什么、以及拒绝把什么当成证明。

use std::path::PathBuf;

use super::{MIN_BYTES, Scope, proved, remember};

/// A source big enough to be worth a record, and one that is not.
/// 一份大到值得留记录的源码，以及一份不值得的。
fn big(label: &str) -> String {
    let mut source = format!("//! {label}\n//! {label}\n\n");
    while source.len() < MIN_BYTES + 64 {
        source.push_str("pub fn helper(value: u32) -> u32 { value.wrapping_add(1) }\n");
    }
    source
}

fn scratch(label: &str) -> PathBuf {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "xirang-ordinary-{label}-{}-{sequence}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    root
}

/// The point of the module: a proof written by one pass is read by the **next** one, i.e. by the next
/// process, without the source being parsed again.
/// 本模块存在的理由：一趟写下的证明会被**下一趟**（也就是下一个进程）读到，而源码不必再被解析一次。
#[test]
fn a_proof_survives_the_pass_that_wrote_it() {
    let directory = scratch("survives");
    let source = big("first");
    assert!(!proved(&source), "nothing is proved before anything runs");
    {
        let _scope = Scope::open(&directory);
        assert!(
            !proved(&source),
            "and not while the pass that will prove it is open"
        );
        remember(&source);
        assert!(proved(&source), "the pass knows what it just proved");
    }
    // A **second pass**, i.e. the next process: its session is empty, so the only thing that can
    // answer is the record on disk. (The cross-process half itself is measured by A33's restart leg,
    // which re-runs the pass in a child process — report §7i.3.)
    // **第二趟**，也就是下一个进程：它的会话是空的，因此能作答的只有盘上那条记录。（跨进程那一半本身由 A33 的
    // 重启腿量——它在子进程里重跑这一趟；报告 §7i.3。）
    let _second = Scope::open(&directory);
    let records: Vec<String> = std::fs::read_dir(&directory)
        .expect("the store directory exists")
        .flatten()
        .filter_map(|entry| std::fs::read_to_string(entry.path()).ok())
        .collect();
    assert_eq!(records.len(), 1, "one proof, one record");
    assert_eq!(
        records[0],
        super::schema_line(),
        "and the record is the schema line this module accepts"
    );
    assert!(
        proved(&source),
        "the second pass reads the record instead of parsing again"
    );

    // With no store open nothing is proved, whatever is on disk: a one-off caller keeps today's
    // behaviour exactly, which is the property that makes this safe to add under a hot path.
    // 没有打开存储时，盘上有什么都不算证明：一次性调用方的行为与从前完全一致，这正是它能安全地加在热路径下的性质。
    let unopened = {
        let _scope = Scope::open(&scratch("elsewhere"));
        proved(&source)
    };
    assert!(
        !unopened,
        "a store pointed somewhere else does not see this record"
    );
    let _ = std::fs::remove_dir_all(&directory);
}

/// An edit is a different key, so it cannot be served by the record the old bytes wrote.
/// 一次编辑就是一个不同的键，因此它不可能被旧字节写下的记录供上。
#[test]
fn an_edited_source_is_not_the_proved_one() {
    let directory = scratch("edited");
    let source = big("before");
    let edited = format!("{source}\npub fn extra() -> u32 {{ 7 }}\n");
    {
        let _scope = Scope::open(&directory);
        remember(&source);
    }
    let _scope = Scope::open(&directory);
    assert!(proved(&source), "the unchanged bytes are still proved");
    assert!(!proved(&edited), "and the edited bytes are not");
    let _ = std::fs::remove_dir_all(&directory);
}

/// Small sources are not stored: the record costs more to find than the parse costs to redo.
/// 小源码不存：找一条记录比重新解析一次更贵。
#[test]
fn a_small_source_is_not_worth_a_record() {
    let directory = scratch("small");
    let small = "pub fn tiny() -> u32 { 1 }\n";
    assert!(
        small.len() < MIN_BYTES,
        "the fixture is below the threshold"
    );
    {
        let _scope = Scope::open(&directory);
        remember(small);
    }
    let _scope = Scope::open(&directory);
    assert!(!proved(small), "below the threshold there is no record");
    assert!(
        !directory.join(super::fingerprint(small)).is_file(),
        "and nothing was written for it"
    );
    let _ = std::fs::remove_dir_all(&directory);
}

/// A record that is not this module's — another schema, another shape, a truncated file — is not a
/// proof, and refusing it only costs a parse.
/// 不是本模块的记录——别的 schema、别的形状、被截断的文件——都不是证明，而拒绝它只损失一次解析。
#[test]
fn a_foreign_record_is_not_a_proof() {
    let directory = scratch("foreign");
    std::fs::create_dir_all(&directory).expect("the store directory");
    let source = big("foreign");
    let record = directory.join(super::fingerprint(&source));
    for body in [
        "# schema\txirang-ordinary-0\n",
        "# schema\txirang-unit-1\n",
        "",
        "not a record at all\n",
    ] {
        std::fs::write(&record, body).expect("the record writes");
        let _scope = Scope::open(&directory);
        assert!(
            !proved(&source),
            "a record saying {body:?} is not this module's proof"
        );
    }
    std::fs::write(&record, super::schema_line()).expect("the record writes");
    let _scope = Scope::open(&directory);
    assert!(proved(&source), "and the right schema is accepted");
    let _ = std::fs::remove_dir_all(&directory);
}

/// An inner pass does not steal the outer pass's store, and what the inner one learned is written when
/// the inner one ends.
/// 内层那一趟不会抢走外层那一趟的存储，而内层学到的东西在内层结束时就被写下。
///
/// The stack is what makes this true; a single slot would have the inner scope's `Drop` write the inner
/// records into — or read them from — the outer scope's directory.
/// 让这件事成立的是那个栈；换成单个格子，内层作用域的 `Drop` 就会把内层的记录写进（或从其中读取）外层的目录。
#[test]
fn an_inner_pass_keeps_its_own_store() {
    let outer = scratch("outer");
    let inner = scratch("inner");
    let first = big("outer-source");
    let second = big("inner-source");
    let _outer = Scope::open(&outer);
    remember(&first);
    {
        let _inner = Scope::open(&inner);
        assert!(
            !proved(&first),
            "the inner store does not see the outer proof"
        );
        remember(&second);
        assert!(proved(&second), "but it sees its own");
    }
    assert!(
        inner.join(super::fingerprint(&second)).is_file(),
        "the inner pass wrote its record when it ended"
    );
    assert!(
        proved(&first),
        "the outer store is back in force, with its own proof"
    );
    let _ = std::fs::remove_dir_all(&outer);
    let _ = std::fs::remove_dir_all(&inner);
}
