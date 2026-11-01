//! Join family leases to a specimen comparison without making the caller reconstruct adoption rules.
//! 将同族租约接到标本比对上，调用方不必自行重建采信规则。

use std::collections::BTreeMap;
use std::path::Path;

use xirang_kernel::adoption::{AdoptionEntry, AdoptionVerdict};

/// Latest ledger claims directly under this parent, bounded independently of source excerpts.
/// 此父级直接下属的最新台账声明，预算与源码摘录分别计算。
///
/// Ledger membership is not proof that a face still exists. Missing files are unreadable, never
/// provisional. A lease and a shape deviation remain independent verdicts; reading renews neither.
/// 台账成员不证明注册面仍存在。文件缺失须报不可读，不能报暂时采信。租约与形状偏差各自判定，读取不续期。
pub(crate) fn family_leases(
    root: &Path,
    entries: &[AdoptionEntry],
    parent: &str,
) -> Result<Vec<String>, String> {
    let mut latest = BTreeMap::new();
    for entry in entries {
        if entry
            .anchor
            .trim_end_matches('/')
            .rsplit_once('/')
            .map(|(p, _)| p)
            == Some(parent)
        {
            latest.insert(&entry.anchor, entry);
        }
    }
    let prints = super::prints_by_entry(root)?;
    let mut lines = vec![format!(
        "family leases {parent}: {} ledger anchor(s); newest entry wins; provisional is not proof of correctness",
        latest.len()
    )];
    for entry in latest.values().take(20) {
        match super::read_lease(root, entry) {
            Ok(reading) => {
                let status = match reading.state.verdict {
                    AdoptionVerdict::Provisional => "provisional",
                    AdoptionVerdict::Lapsed { .. } => "lapsed — needs human confirmation",
                };
                let moved = super::moved_files(
                    prints.get(&(entry.anchor.clone(), entry.at.clone())),
                    &reading.current,
                );
                lines.push(format!(
                    "  lease {}: {status}; {}",
                    entry.anchor,
                    super::lease(
                        &entry.fingerprint,
                        &reading.state.current,
                        &entry.files,
                        false,
                        moved.as_deref()
                    )
                ));
            }
            Err(reason) => lines.push(format!("  lease {}: unreadable — {reason}", entry.anchor)),
        }
    }
    if latest.len() > 20 {
        lines.push(format!(
            "  {} lease(s) omitted (not checked); use conformance {{anchor}} for a named anchor",
            latest.len() - 20
        ));
    }
    Ok(lines)
}
