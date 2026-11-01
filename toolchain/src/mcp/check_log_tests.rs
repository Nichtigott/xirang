//! Large logs preserve late failures and full execution counts within bounded memory.
//! 大型日志在有界内存中保留靠后的失败与完整执行计数。
use super::*;
#[test]
fn huge_lines_and_late_diagnostics_do_not_hide_failure_or_counts() {
    let root =
        std::env::temp_dir().join(format!("xirang-log-{}", crate::mcp::check_jobs::new_id()));
    let mut text = "x".repeat(3_000_000);
    text.push_str("\nerror[E0308]: late failure\n --> src/lib.rs:1:2\n\ntest late ... FAILED\n---- late stdout ----\nthread 'late' panicked at src/lib.rs:1:2: broken contract\n\ntest result: FAILED. 400 passed; 1 failed; 8 ignored\n");
    std::fs::write(&root, text).unwrap();
    let facts = read(&root).unwrap();
    assert_eq!((facts.results, facts.executed), (1, 401));
    assert!(facts.text.contains("error[E0308]: late failure"));
    assert!(facts.text.contains("broken contract"));
    assert!(facts.text.contains("withheld"));
    assert!(facts.text.len() < 270_000);
    std::fs::remove_file(root).unwrap();
}

/// A27/A09: target attribution survives the presentation budget. The `Running` headers are dropped
/// from the retained text by the 65 KiB budget, yet the stream facts still bind each `shared` row to
/// its own binary, so two identical names never collapse into one identity.
/// A27/A09：target 归属熬过展示预算。`Running` 头被 65 KiB 预算从保留文本里丢掉，但流事实仍把每条
/// `shared` 行绑到各自的二进制，因此两个同名测试绝不会塌缩成一个身份。
#[test]
fn target_attribution_survives_presentation_truncation() {
    let root = std::env::temp_dir().join(format!(
        "xirang-log-facts-{}",
        crate::mcp::check_jobs::new_id()
    ));
    let mut text = String::new();
    for index in 0..5_000 {
        text.push_str(&format!("filler line {index}\n"));
    }
    text.push_str(
        "     Running unittests src/lib.rs (target/debug/deps/a-1111111111111111)\n\
         running 1 test\n\
         test shared ... ok\n\
         test result: ok. 1 passed; 0 failed\n\
         \x20    Running unittests src/lib.rs (target/debug/deps/b-2222222222222222)\n\
         running 1 test\n\
         test shared ... FAILED\n\
         test result: FAILED. 0 passed; 1 failed\n",
    );
    std::fs::write(&root, &text).unwrap();
    let facts = read(&root).unwrap();
    assert!(
        !facts.text.contains("Running unittests"),
        "the budget dropped the headers from the presentation text"
    );
    assert!(facts.targets.contains(&(
        "target/debug/deps/a-1111111111111111".to_owned(),
        "src/lib.rs".to_owned()
    )));
    assert!(facts.targets.contains(&(
        "target/debug/deps/b-2222222222222222".to_owned(),
        "src/lib.rs".to_owned()
    )));
    let shared: Vec<_> = facts
        .rows
        .iter()
        .filter(|row| row.name == "shared")
        .collect();
    assert_eq!(shared.len(), 2);
    assert_ne!(
        shared[0].target.as_ref().map(|t| &t.binary),
        shared[1].target.as_ref().map(|t| &t.binary),
        "the two rows keep distinct binary identities"
    );
    assert!(!facts.incomplete);
    std::fs::remove_file(root).unwrap();
}

/// A row with no preceding header is an unsupported evidence shape: it is marked incomplete so a
/// reader cannot claim its name is unique.
/// 前面没有头的行是不受支持的证据形状：标记 incomplete，读者不能据此声称名字唯一。
#[test]
fn a_headerless_test_row_marks_the_facts_incomplete() {
    let root = std::env::temp_dir().join(format!(
        "xirang-log-incomplete-{}",
        crate::mcp::check_jobs::new_id()
    ));
    std::fs::write(
        &root,
        "test orphan ... FAILED\ntest result: FAILED. 0 passed; 1 failed\n",
    )
    .unwrap();
    let facts = read(&root).unwrap();
    assert!(facts.incomplete);
    assert_eq!(facts.rows.len(), 1);
    assert!(facts.rows[0].target.is_none());
    std::fs::remove_file(root).unwrap();
}
