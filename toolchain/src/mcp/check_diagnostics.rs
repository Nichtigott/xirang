//! Bounded compiler evidence from the existing Cargo log; never rerun to read diagnostics.
//! 从已有 Cargo 日志取有界编译器证据；读取诊断绝不重跑。

/// Preserve compiler error blocks within a byte budget, disclosing omissions.
/// 在字节预算内保留编译器错误块，并披露省略。
pub(super) fn compiler_lines(text: &str, verbose: bool) -> Vec<String> {
    let budget: usize = if verbose { 12_000 } else { 3_000 };
    let mut used = 0;
    let mut omitted = 0;
    let mut output = Vec::new();
    let mut active = false;
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("error:") || trimmed.starts_with("error[") {
            active = true;
        } else if trimmed.is_empty() || trimmed.starts_with("warning") {
            active = false;
        }
        if !active {
            continue;
        }
        let available = budget.saturating_sub(used);
        if available < 32 {
            omitted += 1;
            continue;
        }
        let mut end = line.len().min(available.saturating_sub(16));
        while !line.is_char_boundary(end) {
            end -= 1;
        }
        let suffix = if end < line.len() { " …" } else { "" };
        let rendered = format!("compiler {slice}{suffix}", slice = &line[..end]);
        used += rendered.len() + 1;
        if end < line.len() {
            omitted += 1;
        }
        output.push(rendered);
    }
    if omitted > 0 {
        output.push(format!("compiler: {omitted} line(s) partially/fully withheld; read the log for complete diagnostics"));
    }
    output
}

#[cfg(test)]
#[path = "check_diagnostics_tests.rs"]
mod check_diagnostics_tests;
