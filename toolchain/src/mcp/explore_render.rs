//! Budget allocation and line-preserving rendering for explore.
//! explore 的预算分配与保留原行的渲染。

use std::collections::BTreeMap;
use std::fmt::Write;

use super::{Selection, SourceFile};

pub(super) fn answer(
    sources: &[SourceFile],
    selections: BTreeMap<usize, Selection>,
    max_files: usize,
    budget: usize,
    context: &super::context::Context,
    structure: &str,
    repair: &str,
) -> String {
    let mut files: Vec<_> = selections.into_iter().collect();
    files.sort_by_key(|(id, selection)| (selection.rank, *id));
    let total = files.len();
    let mut output = format!(
        "explore: {total} matching/context files; {} Rust files scanned\n\
        scope: current verbatim disk source, including uncompiled files; shown lines are already read\n\
        relations: direct lexical candidates and transitive caller tests, NOT resolved dispatch or coverage\n",
        sources.len()
    );
    output.push_str(&context.scope_notice());
    output.push_str(structure);
    output.push_str(repair);
    if files.is_empty() {
        output.push_str(
            "no matches; use function names, Rust file paths or distinctive symptom words\n",
        );
        output.push_str("next   search {literal: query} for literal text, or locate {symptom} for prose, via xirang_tools\n");
        return output;
    }
    // Reserve the footer plus the dispatcher's evidence (up to 120 Unicode characters).
    // 为末尾和分派器证据行（最多 120 个 Unicode 字符）预留预算。
    let usable = budget.saturating_sub(output.len() + 1100);
    // Avoid dividing a small budget into fragments too small to carry useful source.
    // 小预算下先少选文件，避免每份配额小到只能容纳标题。
    let shown = total.min(max_files).min((usable / 700).max(1));
    let mut rendered = Vec::new();
    for (offset, (id, selection)) in files.iter().take(shown).enumerate() {
        let allocation = usable / shown + usize::from(offset < usable % shown);
        let file = &sources[*id];
        // Receipt overhead is reserved before choosing source lines, so opting in preserves bounds.
        // 先给回执留空间再选源码行，启用复用也不能突破字节预算。
        let block = file_block(
            file,
            selection,
            allocation.saturating_sub(context.receipt_overhead()),
        );
        if !block.is_empty() {
            output.push_str(&context.source_block(file, block));
            rendered.push(*id);
        }
    }
    let _ = writeln!(
        output,
        "files omitted: {} of {total}; narrow query or raise max_files/max_bytes",
        total - rendered.len()
    );
    let mut pointers = 0;
    for (id, _) in &files {
        if rendered.contains(id) {
            continue;
        }
        let row = format!("withheld file {}; query this path\n", sources[*id].relative);
        if pointers + row.len() > 300 {
            continue;
        }
        output.push_str(&row);
        pointers += row.len();
    }
    output.push_str("next   reuse shown source while retained and unchanged; read only missing context needed for the repair. After edits, test behavior via xirang_tools {\"tool\":\"check\",\"arguments\":{\"workspace\":true,\"census\":false}}\n");
    output
}

fn file_block(file: &SourceFile, selection: &Selection, budget: usize) -> String {
    let mut output = format!("\nfile {}\n", file.relative);
    // Keep complete source lines, never a byte slice masquerading as verbatim source.
    // 只输出完整源码行，不把截断的字节片段伪装成原文。
    let footer_reserve = 280;
    if output.len() + footer_reserve >= budget {
        return String::new();
    }
    let notes_budget = (budget - output.len() - footer_reserve) / 4;
    let mut notes_used = 0;
    let mut notes_shown = 0;
    for note in &selection.notes {
        if notes_used + note.len() + 3 > notes_budget {
            continue;
        }
        let _ = writeln!(output, "  {note}");
        notes_used += note.len() + 3;
        notes_shown += 1;
    }
    let lines: Vec<_> = file.source.lines().collect();
    let mut ranges = selection.ranges.clone();
    for (start, _, _) in &mut ranges {
        while *start > 1 {
            let line = lines.get(*start - 2).copied().unwrap_or("").trim_start();
            if line.starts_with("///") || line.starts_with("#[") {
                *start -= 1;
            } else {
                break;
            }
        }
    }
    let mut priorities: BTreeMap<usize, u8> = BTreeMap::new();
    for (start, end, rank) in ranges {
        for number in start..=end.min(lines.len()) {
            priorities
                .entry(number)
                .and_modify(|r| *r = (*r).min(rank))
                .or_insert(rank);
        }
    }
    let mut ordered: Vec<_> = priorities.into_iter().collect();
    ordered.sort_by_key(|&(number, rank)| (rank, number));
    let total = ordered.len();
    let mut first_omitted = 0;
    let mut reserved = output.len();
    let mut kept = Vec::new();
    for (number, _) in ordered {
        // Reserve a gap marker for each line; actual rendering can use less space.
        // 每行预留间隔标记，实际渲染可少用；点名函数不会被前面的邻居挤掉。
        let size = number.to_string().len() + 7 + lines[number - 1].len();
        if reserved + size + footer_reserve > budget {
            if first_omitted == 0 || number < first_omitted {
                first_omitted = number;
            }
        } else {
            reserved += size;
            kept.push(number);
        }
    }
    kept.sort_unstable();
    let omitted = total - kept.len();
    let coverage = if kept.len() == lines.len() {
        "whole file"
    } else {
        "selected spans"
    };
    let _ = writeln!(
        output,
        "source: {coverage}; {} of {} file lines shown",
        kept.len(),
        lines.len()
    );
    let mut previous = 0;
    for number in kept {
        if previous > 0 && number > previous + 1 {
            output.push_str("...\n");
        }
        let _ = writeln!(output, "{number} | {}", lines[number - 1]);
        previous = number;
    }
    if omitted > 0 {
        let _ = writeln!(
            output,
            "source omitted: {omitted} selected lines; first omitted line: {first_omitted}"
        );
    }
    if selection.notes.len() > notes_shown {
        let _ = writeln!(
            output,
            "relation/target notes omitted: {}",
            selection.notes.len() - notes_shown
        );
    }
    output
}
