//! The migration record a move publishes, planned before it is written.
//! 搬动发布的迁移记录，在写入之前先规划。
//!
//! Split out of `apply_move.rs` to keep that file inside the repository's size ratchet. Plan and
//! publish are separate on purpose: the record joins the same transaction as the moved bytes, so a
//! record written outside it would survive a later failure and name a move the tree never made.
//! 从 `apply_move.rs` 拆出，使那个文件留在仓库的尺寸棘轮之内。规划与发布有意分开：记录与被搬的字节属于
//! 同一个事务，因此写在事务之外的记录会活过后来的一次失败，并陈述一次树从未做过的搬动。

use std::path::{Path, PathBuf};

use xirang_kernel::identity::NodeId;
use xirang_kernel::lexicon;

/// A move record, planned but **not yet published**.
/// 一条搬动记录，已规划但**尚未发布**。
///
/// Plan and publish are separate so the record joins the same transaction as the moved bytes: a
/// record written outside it survives a later failure and names a move the tree never made.
/// 规划与发布分开，是为了让记录与被搬的字节属于同一个事务：写在事务之外的记录会活过后来的一次失败，并
/// 陈述一次树从未做过的搬动。
pub(crate) struct RecordPlan {
    /// The `.xirang/moves` directory the record directory lives in.
    /// 记录目录所在的 `.xirang/moves` 目录。
    pub(crate) directory: PathBuf,
    /// The record file itself.
    /// 记录文件本身。
    pub(crate) path: PathBuf,
    /// The bytes to publish there.
    /// 要发布到那里的字节。
    pub(crate) body: String,
}

/// Plan the next move record, keeping the most recent five once it is published.
/// 规划下一条搬动记录；发布之后只保留最近五条。
pub(crate) fn plan_record(
    work: &Path,
    from: &str,
    to: &str,
    old_id: NodeId,
    new_id: NodeId,
) -> Result<RecordPlan, String> {
    let directory = work.join(lexicon::XIRANG_DIR).join(lexicon::MOVES_DIR);
    let next = next_number(&directory);
    let record = directory.join(next.to_string());
    let when = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs())
        .unwrap_or_default();
    let body =
        format!("version=1\nfrom={from}\nto={to}\nfrom_id={old_id}\nto_id={new_id}\nwhen={when}\n");
    Ok(RecordPlan {
        directory,
        path: record.join(lexicon::MOVE_PLAN_FILE),
        body,
    })
}

/// Publish a planned record through the move's transaction.
/// 经搬动的事务发布一条已规划好的记录。
///
/// The numbered directory is created with `create_dir`, never reused: two writers must not publish
/// into one entry, and an existing number is a collision rather than something to overwrite.
/// 编号目录用 `create_dir` 创建，绝不复用：两个写入方不得发布进同一条目，已存在的编号是碰撞，而不是可以
/// 覆盖的东西。
pub(crate) fn publish_record(
    transaction: &mut crate::run_method::authoring::write_transaction::WriteTransaction,
    plan: &RecordPlan,
) -> Result<(), String> {
    let Some(record_directory) = plan.path.parent() else {
        return Err(format!("{} has no parent directory", plan.path.display()));
    };
    transaction.create_dir(record_directory)?;
    transaction.write(&plan.path, plan.body.as_bytes())
}

/// How the root is spelled in a `parent:` line.
/// 根在 `parent:` 行里的拼法。
pub(crate) const ROOT_PARENT_SPELLING: &str = "crate::root_node_id(crate::XIRANG_NAMESPACE)";

/// How many move records are kept (the maintainer's decision, 2026-10-09).
/// 保留多少条搬动记录（维护者 2026-10-09 的决定）。
pub(crate) const KEEP_RECORDS: u32 = 5;

/// The numbers of the record directories, ascending.
/// 记录目录的编号，升序。
pub(crate) fn numbers(directory: &Path) -> Vec<u32> {
    let Ok(entries) = std::fs::read_dir(directory) else {
        return Vec::new();
    };
    let mut numbers: Vec<u32> = entries
        .flatten()
        .filter_map(|entry| entry.file_name().to_string_lossy().parse::<u32>().ok())
        .collect();
    numbers.sort_unstable();
    numbers
}

/// The number the next record takes.
/// 下一条记录取的编号。
pub(crate) fn next_number(directory: &Path) -> u32 {
    numbers(directory).last().map_or(1, |last| last + 1)
}

/// Drop the oldest records so that at most `keep` remain.
/// 丢掉最旧的记录，使至多留下 `keep` 条。
pub(crate) fn prune(directory: &Path, keep: u32) -> Vec<String> {
    let numbers = numbers(directory);
    let excess = numbers.len().saturating_sub(keep as usize);
    let mut dropped = Vec::new();
    for number in numbers.into_iter().take(excess) {
        let _ = std::fs::remove_dir_all(directory.join(number.to_string()));
        dropped.push(format!("moves/{number}"));
    }
    dropped
}
