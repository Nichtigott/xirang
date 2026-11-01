//! Stream arbitrary Cargo logs with bounded retained text, preserving execution counts.
//! 流式读取任意 Cargo 日志，有界保留文本并保留执行计数。
//!
//! The typed facts a reader needs for attribution (which target instance a test ran in, which tests
//! failed) are collected **in the stream**, before the retention budget decides what presentation
//! text survives. Presentation truncation therefore cannot make a duplicated test name look unique.
//! 读者做归属所需的类型化事实（一个测试跑在哪个 target 实例、哪些测试失败）在**流中**收集，早于保留预算
//! 决定哪些展示文本留下。因此展示截断不能让重复的测试名看起来唯一。
use std::collections::BTreeSet;
use std::io::{BufRead, BufReader};
use std::path::Path;

pub(super) fn executed(line: &str) -> usize {
    counted(line, &["passed", "failed"])
}

/// How many tests a `test result:` line says were **skipped** rather than run.
/// 一条 `test result:` 行说有多少测试被**跳过**而不是执行。
///
/// `ignored` and `filtered out` are the two spellings: a test the source marks `#[ignore]` is not exercised, and one a
/// filter excluded may not be either. The anomaly dimension needs them because "the run passed" and "the run passed
/// everything there was" are different facts, and only the second one leaves nothing undisposed.
/// `ignored` 与 `filtered out` 是两种拼法：源码里标了 `#[ignore]` 的测试没有被执行，而被过滤掉的那个也可能没有。
/// 异常维度需要它们，因为"这次运行通过了"与"这次运行把它该跑的全都跑通了"是两个事实，只有后者没有留下未处置之物。
pub(super) fn skipped(line: &str) -> usize {
    counted(line, &["ignored", "filtered"])
}

/// The sum of the counts a result line reports for the given trailing words.
/// 结果行里以给定尾词结尾的那些计数之和。
fn counted(line: &str, endings: &[&str]) -> usize {
    line.split(';')
        .filter_map(|part| {
            let words: Vec<_> = part.split_whitespace().collect();
            let count = words.len().checked_sub(2)?;
            matches!(words.last().copied(), Some(word) if endings.contains(&word))
                .then(|| words[count].parse::<usize>().ok())
                .flatten()
        })
        .sum()
}

/// The unique execution target a `Running …` header names.
/// 一条 `Running …` 行点名的唯一执行 target。
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct TargetInstance {
    /// User-facing source label, e.g. `src/lib.rs`.
    /// 对用户可见的源标签，例如 `src/lib.rs`。
    pub(super) label: String,
    /// Unique binary path from the parenthesized part, e.g. `target/debug/deps/a_proof-1a2b`.
    /// 括号部分里的唯一二进制路径，例如 `target/debug/deps/a_proof-1a2b`。
    pub(super) binary: String,
}

/// One `test <name> ... <verdict>` row, attributed to the target instance it ran in.
/// 一条 `test <name> ... <verdict>` 行，归属到它所在的 target 实例。
#[derive(Clone, Debug)]
pub(super) struct TestRow {
    /// The full test name cargo printed.
    /// cargo 打印的完整测试名。
    pub(super) name: String,
    /// The target instance, when a `Running` header preceded the row.
    /// 该行之前有 `Running` 头时，那个 target 实例。
    pub(super) target: Option<TargetInstance>,
    /// Whether the row read `FAILED`.
    /// 该行是否读作 `FAILED`。
    pub(super) failed: bool,
}

/// Everything the stream established, independent of the retained presentation text.
/// 流确立的一切，独立于保留的展示文本。
pub(super) struct LogFacts {
    /// The bounded text a reply renders.
    /// 回复渲染用的有界文本。
    pub(super) text: String,
    /// How many `test result:` lines the stream had.
    /// 流里有几条 `test result:` 行。
    pub(super) results: usize,
    /// Tests actually executed across every result line.
    /// 所有结果行里实际执行的测试数。
    pub(super) executed: usize,
    /// Tests the result lines say were skipped (`ignored` or `filtered out`), summed across them.
    /// 结果行说被跳过（`ignored` 或 `filtered out`）的测试数，逐行求和。
    pub(super) skipped: usize,
    /// Every test row the stream carried.
    /// 流携带的每一条测试行。
    pub(super) rows: Vec<TestRow>,
    /// The unique target instances the stream named.
    /// 流点名过的唯一 target 实例。
    pub(super) targets: BTreeSet<(String, String)>,
    /// True when a test row ran without a preceding `Running` header, so its target is unknown.
    /// 有测试行前面没有 `Running` 头、其 target 未知时为真。
    pub(super) incomplete: bool,
}

/// Consume even huge individual lines without allocating their full length.
/// 即使单行巨大也完整消费，而不分配其全部长度。
pub(super) fn line(
    reader: &mut impl BufRead,
    cap: usize,
) -> std::io::Result<Option<(String, bool)>> {
    let mut bytes = Vec::new();
    let mut omitted = false;
    let mut read = false;
    loop {
        let buffer = reader.fill_buf()?;
        if buffer.is_empty() {
            break;
        }
        read = true;
        let end = buffer.iter().position(|byte| *byte == b'\n');
        let take = end.map_or(buffer.len(), |at| at + 1);
        let keep = take.min(cap.saturating_sub(bytes.len()));
        bytes.extend_from_slice(&buffer[..keep]);
        omitted |= keep < take;
        reader.consume(take);
        if end.is_some() {
            break;
        }
    }
    Ok(read.then(|| {
        (
            String::from_utf8_lossy(&bytes)
                .trim_end_matches(['\r', '\n'])
                .to_owned(),
            omitted,
        )
    }))
}

/// The target instance a `Running …` header names, parsed from its source label and unique binary.
/// 一条 `Running …` 头点名的 target 实例，从它的源标签与唯一二进制解析。
fn target_instance(rest: &str) -> Option<TargetInstance> {
    let (before, after) = rest.split_once(" (")?;
    let binary = after.strip_suffix(')').unwrap_or(after).trim().to_owned();
    if binary.is_empty() {
        return None;
    }
    let mut words = before.split_whitespace();
    let first = words.next()?;
    let remainder = words.collect::<Vec<_>>().join(" ");
    // Cargo spells unit tests as `unittests src/lib.rs` and integration tests as `tests/right.rs`;
    // the label is what a human selects, so the leading kind word is dropped when a path follows.
    // cargo 把单元测试写成 `unittests src/lib.rs`、集成测试写成 `tests/right.rs`；标签是人选择的东西，
    // 因此后面跟着路径时去掉开头那个种类词。
    let label = if remainder.is_empty() {
        first.to_owned()
    } else {
        remainder
    };
    Some(TargetInstance { label, binary })
}

pub(super) fn read(path: &Path) -> Result<LogFacts, String> {
    let file = std::fs::File::open(path).map_err(|e| format!("read {}: {e}", path.display()))?;
    let mut reader = BufReader::new(file);
    let mut text = String::new();
    let mut details = String::new();
    let mut results = 0;
    let mut ran = 0;
    let mut skipped = 0;
    let mut context: usize = 0;
    let mut omitted = 0;
    let mut rows = Vec::new();
    let mut targets = BTreeSet::new();
    let mut current: Option<TargetInstance> = None;
    let mut incomplete = false;
    while let Some((line, cut)) = line(&mut reader, 16_384).map_err(|e| e.to_string())? {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("Running ") {
            match target_instance(rest) {
                Some(target) => {
                    targets.insert((target.binary.clone(), target.label.clone()));
                    current = Some(target);
                }
                // A header that cannot be parsed leaves every following row unattributed.
                // 解析不出的头会让其后每一行都无法归属。
                None => {
                    current = None;
                    incomplete = true;
                }
            }
        } else if let Some(rest) = trimmed.strip_prefix("test ")
            && let Some((name, verdict)) = rest.rsplit_once(" ... ")
        {
            if current.is_none() {
                incomplete = true;
            }
            rows.push(TestRow {
                name: name.to_owned(),
                target: current.clone(),
                failed: verdict == "FAILED",
            });
        }
        if line.contains("test result:") {
            results += 1;
            ran += executed(&line);
            skipped += self::skipped(&line);
        }
        if trimmed.starts_with("error")
            || trimmed.starts_with("---- ")
            || trimmed.contains("panicked at")
        {
            context = 24;
        }
        let important =
            context > 0 || line.contains("test result:") || line.ends_with(" ... FAILED");
        context = context.saturating_sub(1);
        if text.len() + line.len() < 65_536 {
            text.push_str(&line);
            text.push('\n');
        } else if important && details.len() + line.len() < 196_608 {
            details.push_str(&line);
            details.push('\n');
        } else {
            omitted += 1;
        }
        if cut {
            omitted += 1;
        }
    }
    text.push_str(&details);
    if omitted > 0 {
        text.push_str(&format!(
            "\nxirang-log-withheld: {omitted} line(s); full immutable log remains on disk\n"
        ));
    }
    Ok(LogFacts {
        text,
        results,
        executed: ran,
        skipped,
        rows,
        targets,
        incomplete,
    })
}

#[cfg(test)]
#[path = "check_log_tests.rs"]
mod check_log_tests;
