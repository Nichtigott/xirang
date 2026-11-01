//! Small pure helpers for the build pipeline.
//! 构建管线的小型纯助手。

use std::time::Duration;

use xirang_kernel::lexicon;

/// Whether `entry` is one a later entry superseded (its subtree is in `dropped`).
/// `entry` 是否被后一条盖住（它的子树在 `dropped` 里）。
/// Keep only the entries whose **position** is not in `superseded`.
/// 只保留**位置**不在 `superseded` 里的条目。
///
/// Positions refer to the list the overlap decision was made on, which is the entry list itself (entries whose cut
/// names no subtree are dropped from `pairs`, but they are also never superseded, so the two lists agree entry for
/// entry on everything this retain can see).
/// 位置指的是做重叠判定时的那份清单，也就是条目清单本身（切口点名不到子树的条目不会进入 `pairs`，但它们也从不会被
/// 判定为被取代，因此在这次保留能看到的范围内两份清单逐条对应）。
pub(super) fn retain_unsuperseded(
    entries: &mut Vec<xirang_kernel::syntax::GraftSyntax>,
    superseded: &[usize],
) {
    let mut position = 0usize;
    entries.retain(|_| {
        let keep = !superseded.contains(&position);
        position += 1;
        keep
    });
}

pub(super) fn build_output_is_verbose() -> bool {
    std::env::var_os(lexicon::BUILD_VERBOSE_ENV).is_some_and(|value| !value.is_empty())
}

pub(super) fn cache_status_line(
    cache_state: &str,
    fingerprint: &str,
    verbose: bool,
) -> Option<String> {
    verbose.then(|| format!("xirang discovery cache {cache_state} ({fingerprint})"))
}

/// One line: every phase in order, then the whole run's wall time.
/// 一行：按顺序列出每个相位，最后是整次运行的墙钟时间。
pub(super) fn phase_line(marks: &[(&'static str, Duration)], total: Duration) -> String {
    let mut line = String::from("xirang build phases:");
    for (name, elapsed) in marks {
        line.push_str(&format!(" {name}={:.3}s", elapsed.as_secs_f64()));
    }
    // The mechanism's own dependent variable rides along (`stem 2`): a duration cannot resolve the
    // consumer switch on a 1,000-face tree, a count can — and a count does not move with machine load.
    // 机制自己的因变量跟着一起报（`stem 2`）：在 1,000 面的树上时长分辨不出消费者切换，计数可以——而且计数不随
    // 机器负载漂移。
    line.push_str(&format!(
        " parses={} facts={}",
        crate::build_method::face_syntax_check::face_parses(),
        crate::build_method::discovery_cache::facts_answered()
    ));
    line.push_str(&format!(" total={:.3}s", total.as_secs_f64()));
    line
}
