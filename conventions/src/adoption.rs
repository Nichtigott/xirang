//! The adoption ledger is a lease, and a lease that nobody re-confirmed is a finding.
//! 采信台账是租约，而没有人重新确认过的租约就是一条发现。
//!
//! The maintainer's rule: 「采信是可以改的松动的，但是要强制人去确认可修改，因为肯定是稳步发展的，只是
//! 暂时采信」. This gate is the *forcing* half. It does not forbid a change and it does not revert
//! one: an entry whose files have moved becomes a finding, and the action the finding asks for is a
//! person confirming the new state, which the adoption surface records as one more ledger line.
//! 维护者的规矩：「采信是可以改的松动的，但是要强制人去确认可修改，因为肯定是稳步发展的，只是暂时采信」。
//! 本门禁就是**强制**那一半。它不禁止改动，也不回退改动：文件动过的条目变成一条发现，而这条发现要求的动作
//! 是**由人确认新状态**——采信面把它记成台账里又多了一行。
//!
//! Two things are checked, because they fail differently: a ledger line that does not parse is a
//! file nobody can act on, and an entry whose bytes moved is a decision that is no longer true. The
//! second one is the one that matters — it is the whole reason the ledger carries a fingerprint.
//! 检查两件事，因为它们的失效方式不同：解析不了的一行是没人能据以行动的文件，而字节动过的条目是一个不再
// 成立的决定。要紧的是第二件——台账携带指纹的全部理由就在这里。

use std::path::Path;

use xirang_kernel::adoption::{AdoptionVerdict, parse_adoption, verdict_of};
use xirang_kernel::lexicon::{ADOPTION_DIR, ADOPTION_FILE, XIRANG_DIR};

/// Every ledger entry that needs a person, as one line each.
/// 每一条需要人来处理的台账条目，各一行。
pub fn findings(root: &Path) -> Vec<String> {
    let mut found = Vec::new();
    for ledger in ledgers(root) {
        let relative = crate::relative(root, &ledger);
        if crate::archived_evaluation(&relative) {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(&ledger) else {
            continue;
        };
        let entries = match parse_adoption(&text) {
            Ok(entries) => entries,
            Err(error) => {
                found.push(format!("{}:{} {}", relative, error.line, error.message));
                continue;
            }
        };
        for entry in entries {
            let mut held = Vec::new();
            for file in &entry.files {
                let path = root.join(file);
                let contents = std::fs::read_to_string(&path).unwrap_or_default();
                held.push((file.clone(), contents));
            }
            if let AdoptionVerdict::Lapsed { file } = verdict_of(&entry, &held) {
                found.push(format!(
                    "{}: adoption lapsed at {file}; needs confirmation — {} (adopted at {} by {})",
                    relative, entry.anchor, entry.at, entry.verifier
                ));
            }
        }
    }
    found.sort();
    found
}

/// Every adoption ledger in the checkout, sorted.
/// 检出里的每一份采信台账，已排序。
fn ledgers(root: &Path) -> Vec<std::path::PathBuf> {
    let mut found = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(directory) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&directory) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if path.is_dir() {
                // The same three directories every walk in this repository skips.
                // 与本仓每一趟遍历都跳过的同样三个目录。
                if name != "target" && name != ".git" && name != "node_modules" {
                    stack.push(path);
                }
                continue;
            }
            if path
                .parent()
                .and_then(|parent| parent.file_name())
                .is_some_and(|parent| parent == ADOPTION_DIR)
                && name == ADOPTION_FILE
                && path
                    .parent()
                    .and_then(|parent| parent.parent())
                    .and_then(|parent| parent.file_name())
                    .is_some_and(|parent| parent == XIRANG_DIR)
            {
                found.push(path);
            }
        }
    }
    found.sort();
    found
}

#[cfg(test)]
#[path = "adoption_tests.rs"]
mod adoption_tests;
