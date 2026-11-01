//! The tail of a plan reply: what is still open, what is outside the root, and why `reuse` was not taken.
//! 计划回复的尾部：还有什么开着、什么在根之外，以及 `reuse` 为什么没拿到。
//!
//! Split out of `check_plan` when that page reached its code-line budget (the `size` ratchet named it at 601). The seam
//! is not arbitrary: everything here is **rendering** — it reads the ledger and the request and writes text — while
//! `check_plan` decides what the plan is. A page that both decides and renders is the shape that keeps growing.
//! 从 `check_plan` 拆出，是因为那一页到达了代码行预算（`size` 棘轮点名 601）。这条缝不是随手划的：这里的一切都是
//! **渲染**——读台账与请求、写出文本——而 `check_plan` 决定计划是什么。既决定又渲染的一页，正是长个不停的那种形状。

use serde_json::Value;

/// Everything a plan reply says after the executions and the variants.
/// 计划回复在执行与变体之后要说的一切。
pub(super) fn render(output: &mut String, rows: &[Value], ledger: &Value, reuse_note: &str) {
    let open: Vec<&Value> = rows
        .iter()
        .filter(|row| row["state"].as_str() != Some("passed"))
        .collect();
    if open.is_empty() {
        output.push_str("remaining obligations: none of the declared ones; scope above is the whole of what this plan checked\n");
    } else {
        output.push_str("remaining obligations:\n");
        for row in open {
            output.push_str(&format!(
                "  {}: {}\n",
                row["name"].as_str().unwrap_or("?"),
                row["state"].as_str().unwrap_or("unknown")
            ));
        }
    }
    if let Some(notice) = ledger["outside_root_obligations"].as_str() {
        output.push_str(notice);
        output.push('\n');
    }
    // `reuse` asked and not taken: the caller passes the sentence the witness module produced **at the decision
    // point**, before this plan was written — looking it up here would find the plan we just wrote.
    // `reuse` 要求了却没拿到计划：调用方把见证模块在**决定那一刻**产出的句子传进来——在这里查会查到我们刚写下的那份。
    output.push_str(reuse_note);
    output.push_str("repair completeness: unknown outside executed tests; supply replacement/bypass cases for behavioral counterproofs. No code was rewritten by verification.\n");
}
