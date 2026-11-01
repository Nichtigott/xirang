//! Tests for the graft-plan rows: an entry that cannot be read is counted, not dropped.
//! graft 计划记录的测试：读不了的目录项要被计数，而不是被丢掉。

use std::io;

use super::{UNREADABLE_ENTRY, entry_rows, names_the_record};

/// A directory entry that could not be read becomes a counted row instead of vanishing.
/// 读不了的目录项会变成一条被计数的记录，而不是消失。
///
/// The loop over `read_dir` used `flatten()`, which drops the `Err` arm: a plan *file* that could
/// not be read was counted while an unreadable *entry* disappeared, so `records N` could
/// under-count without saying anything — and that number is what a maintainer reads before
/// pruning a release (audit `L1`).
/// 遍历 `read_dir` 的循环过去用 `flatten()`，它会丢掉 `Err` 那一半：读不了的**计划文件**会被计数，
/// 而读不了的**目录项**就此消失，于是 `records N` 可以在一个字都不说的情况下少数——而这个数字正是
/// 维护者在发布剪枝前读的东西（审计 `L1`）。
#[test]
fn an_unreadable_entry_is_a_counted_row() {
    let error = io::Error::new(io::ErrorKind::PermissionDenied, "denied");
    let rows = entry_rows(Err(error), &[], None);
    assert_eq!(rows.len(), 1, "the error becomes one row, not nothing");
    let row = &rows[0];
    assert_eq!(row.selector, UNREADABLE_ENTRY);
    assert!(
        row.error
            .as_deref()
            .is_some_and(|error| error.contains("cannot read a directory entry")),
        "the row carries why it exists: {:?}",
        row.error
    );
    assert!(
        row.target.is_none() && row.target_path.is_none() && row.graft.is_none(),
        "there is nothing else to say about a directory entry that was never read"
    );
}

/// **"The slot is declared" and "this record is what gets applied" are two answers.**
/// **"槽位有声明"与"会被应用的是这条记录"是两个答案。**
///
/// Measured: an external plan named `other_fast` sitting on a slot the entry hands to `board_fast` was reported
/// as `[declared]`, so a record that **no build will ever apply** looked like a working one (audit `M7`,
/// §7.63). Both spellings have to answer the same question, which is why the comparison is segment-wise.
/// 实测：一份叫 `other_fast` 的外部计划坐在入口交给 `board_fast` 的槽位上时被报成 `[declared]`，于是一份
/// **没有任何构建会应用**的记录看起来是能用的（审计 `M7`，§M7.63）。两种拼写必须回答同一个问题，这也是比较
/// 按分段做的原因。
#[test]
fn a_declaration_names_the_record_that_will_be_applied() {
    let typed = |path: &str| super::DeclaredGraft {
        cut: "crate::board::NODE_ID".to_owned(),
        cut_end: None,
        graft: path.to_owned(),
        full: true,
        cfg: None,
        expressions: None,
        line: 12,
    };
    assert!(
        names_the_record(&typed("dash_graft::board_fast::NODE_ID"), "board_fast"),
        "a typed cut is a Rust path, and one of its segments is the selector"
    );
    assert!(
        names_the_record(&typed("board_fast"), "board_fast"),
        "a string cut writes the selector itself"
    );
    assert!(
        !names_the_record(&typed("dash_graft::board_fast::NODE_ID"), "other_fast"),
        "a different implementation is a different answer"
    );
    assert!(
        !names_the_record(&typed("dash_graft::board_faster::NODE_ID"), "board_fast"),
        "segments compare whole, not by prefix"
    );
}
