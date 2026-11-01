//! An object's source instances, in the ledger's coordinate, and whether they still hold.
//! 一个对象的源码实例（按台账的坐标），以及它们是否仍然成立。
//!
//! One object can have several source instances — the ledger keeps every source it was stamped from,
//! oldest first, current last — and a maintenance task must bind **all the live ones**, not whichever
//! one spelling a single view happened to return. That is the "one object, several language sources
//! do not split" requirement: a change in a second source is a change to the object.
//! 一个对象可以有多个源码实例——台账保留它被盖过的每个源码，最旧的在前、当前的在后——而维护任务必须绑定
//! **所有存活的实例**，而不是某个视图碰巧返回的那一个拼写。这就是"同一对象的多语言来源不分裂"：第二个
//! 源码变了，就是对象变了。
//!
//! The coordinate is the part that is easy to get wrong. `Instance.source` is **package-relative**,
//! the same coordinate the ledger's third column uses; a face view carries a *record spelling* that is
//! relative to its own source base. This module reads only the ledger's coordinate and joins it to the
//! package root; it never treats one coordinate as the other.
//! 容易弄错的是坐标。`Instance.source` 是**包内相对**，与台账第三列同一个坐标；面视图携带的是相对于它自己
//! source base 的**记录拼写**。本模块只读台账的坐标并把它接到包根上；绝不把一个坐标当另一个用。

use std::path::Path;

use serde_json::Value;
use xirang_kernel::maintenance::{TaskObligation, TaskObligationKind};

use super::task_evidence::short;

/// Every source instance the object ledger records for one object, with the digest of each file that
/// is on disk now.
/// 对象台账为一个对象记录的每个源码实例，并给出此刻仍在磁盘上的那个文件的摘要。
///
/// A file that is gone keeps its place with no digest: it is history, and history is not a reason to
/// keep a task open. A file that exists but cannot be read is *not* the same thing and is reported by
/// the caller as undecidable.
/// 已消失的文件保留位置但没有摘要：它是历史，而历史不是让任务一直开着的理由。存在却读不出的文件**不是**
/// 同一件事，由调用方报成判不了。
pub(super) fn object_instances(
    root: &Path,
    stable: &str,
) -> Result<Vec<(String, Option<String>)>, String> {
    let map = xirang_multilang::object_map::ObjectMap::read_from(root)?;
    let Some(entry) = map
        .entries()
        .iter()
        .find(|entry| entry.stable.to_string() == stable)
    else {
        return Ok(Vec::new());
    };
    Ok(entry
        .instances
        .iter()
        .map(|instance| {
            let digest = std::fs::read(root.join(&instance.source))
                .ok()
                .map(|bytes| xirang_kernel::sha256_hex(&bytes));
            (instance.source.clone(), digest)
        })
        .collect())
}

/// The obligation a changed source set implies, or `None` when every live instance still holds.
/// 源码集合变化所意味着的义务；每个存活实例仍成立时为 `None`。
///
/// A record written before instances were bound has no `object.instances` array; those records keep
/// the single-source meaning their `source`/`digest` pair already carries.
/// 在实例被绑定之前写下的记录没有 `object.instances` 数组；那些记录保留它们的 `source`/`digest` 对已经
/// 携带的单源码含义。
pub(super) fn applied_instances(
    root: &Path,
    record: &Value,
    stable: &str,
) -> Result<Option<TaskObligation>, String> {
    let kind = TaskObligationKind::Applied;
    let Some(instances) = record["object"]["instances"].as_array() else {
        return Ok(None);
    };
    let current = object_instances(root, stable)?;
    // No ledger entry means the tree never spelled this object's instances: the record's single
    // source/digest pair — checked against the face's own file by the caller — is the whole answer.
    // 没有台账条目意味着这棵树从未拼写过这个对象的实例：记录里那一对 source/digest（由调用方对着面自己的
    // 文件检查）就是全部答案。
    if current.is_empty() {
        return Ok(None);
    }
    for row in instances {
        let source = row["source"].as_str().unwrap_or_default();
        let Some(digest) = row["digest"].as_str() else {
            continue;
        };
        match current.iter().find(|(listed, _)| listed == source) {
            None => {
                return Ok(Some(TaskObligation::outstanding(
                    kind,
                    format!("the source instance `{source}` is gone since this task recorded it"),
                )));
            }
            Some((_, None)) if !root.join(source).exists() => {
                return Ok(Some(TaskObligation::outstanding(
                    kind,
                    format!(
                        "the source instance `{source}` is gone from disk since this task recorded it"
                    ),
                )));
            }
            Some((_, None)) => {
                return Ok(Some(TaskObligation::unknown(
                    kind,
                    format!("the source instance `{source}` cannot be read now"),
                )));
            }
            Some((_, Some(now))) if now != digest => {
                return Ok(Some(TaskObligation::outstanding(
                    kind,
                    format!(
                        "the source instance `{source}` changed after this task recorded it (was {}, \
                         now {})",
                        short(digest),
                        short(now)
                    ),
                )));
            }
            Some(_) => {}
        }
    }
    for (source, digest) in &current {
        if digest.is_some()
            && !instances
                .iter()
                .any(|row| row["source"].as_str() == Some(source.as_str()))
        {
            return Ok(Some(TaskObligation::outstanding(
                kind,
                format!("a new source instance appeared: `{source}`"),
            )));
        }
    }
    Ok(None)
}

#[cfg(test)]
#[path = "task_sources_tests.rs"]
mod task_sources_tests;
