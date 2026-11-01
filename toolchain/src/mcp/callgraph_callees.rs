//! The callee half of a `callgraph` answer: where each callee is defined, and what qualifies the
//! edge to it.
//! `callgraph` 答案的"被调用者"那一半：每个被调用者定义在哪，以及那条边被什么限定着。
//!
//! Split from `callgraph.rs` so both files stay inside the repository's size budget. The rules are
//! unchanged: a callee carries the line this tree defines it at when there is exactly one
//! definition, an ambiguous name keeps the competition visible, and a caveat (a Python decorator
//! that may wrap the binding) travels to the caller's answer because that is where the edge is read.
//! 从 `callgraph.rs` 拆出，使两个文件都留在仓库的尺寸预算内。规则不变：唯一定义时带本树的定义行，
//! 多义时把竞争保持可见，而 caveat（可能包装绑定的 Python 装饰器）跟到调用者的答案里，因为那条边是在
//! 那里被读的。

use crate::mcp::source_index::{Function, SourceFile};

use super::call_matches;

/// The caveat lines a definition carries on its own block.
/// 定义自己在块上携带的 caveat 行。
pub(super) fn definition_caveat_lines(function: &Function) -> Vec<String> {
    function
        .caveats
        .iter()
        .map(|caveat| format!("  caveat: {caveat}\n"))
        .collect()
}

/// Each definite callee with the line this tree defines it at, or the competition that makes it
/// ambiguous.
/// 每个确定被调用者，带上本树的定义行，或使其中多义的竞争集合。
pub(super) fn located_callees(
    function: &Function,
    labelled: &[(String, SourceFile)],
) -> Vec<String> {
    function
        .calls
        .iter()
        .map(|call| {
            let mut targets = Vec::new();
            let mut count = 0;
            for (label, file) in labelled {
                for candidate in &file.functions {
                    if call_matches(candidate, call) {
                        count += 1;
                        if targets.len() < 3 {
                            targets.push(format!("{label}:{}", candidate.line));
                        }
                    }
                }
            }
            match count {
                0 => call.clone(),
                1 => format!("{call} -> {}", targets[0]),
                _ => format!(
                    "{call} -> ambiguous ({count} definitions; showing {}): {}",
                    targets.len(),
                    targets.join(", ")
                ),
            }
        })
        .collect()
}

/// The caveats of the callees this function calls, when each call has exactly one target.
/// 这个函数调用的被调用者的 caveat（当每次调用只有一个目标时）。
pub(super) fn callee_caveat_lines(
    function: &Function,
    labelled: &[(String, SourceFile)],
) -> Vec<String> {
    let mut caveats = Vec::new();
    for call in &function.calls {
        let mut targets = labelled
            .iter()
            .flat_map(|(label, file)| {
                file.functions
                    .iter()
                    .map(move |candidate| (label, candidate))
            })
            .filter(|(_, candidate)| call_matches(candidate, call));
        let first = targets.next();
        let second = targets.next();
        if let (Some((_, target)), None) = (first, second) {
            for caveat in &target.caveats {
                caveats.push(format!("{} — {}", target.qualified, caveat));
            }
        }
    }
    caveats
}
