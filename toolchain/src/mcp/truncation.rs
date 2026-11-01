//! The one sentence a bounded answer ends with, and the one place that spells it.
//! 一条有上限的答案结尾会带着的那一句话，以及拼出它的唯一地方。
//!
//! An answer that withholds rows has to say three things at once — how many it
//! withheld, which cap did it, and how to reach the rest — and every tool here used
//! to say them its own way: `… +2 more definitions`, `… +5 more`, `… truncated: 501
//! lines total, 400 shown.` Three shapes for one fact is how a reader learns to
//! skip them, and a skipped truncation notice makes a partial answer read as a whole
//! one. The sentence lives here so the wording cannot drift between tools, and the
//! phrase's uniqueness is pinned by `truncation_tests`.
//! 一条扣下了行的答案必须一次说清三件事——扣下多少、哪道上限扣的、怎么拿到其余部分——而这里每个
//! 工具过去都用自己的说法：`… +2 more definitions`、`… +5 more`、`… truncated: 501 lines
//! total, 400 shown.` 同一个事实有三种词形，正是读者学会跳过它们的方式，而被跳过的截断说明会让
//! 一个残缺的答案读起来像完整的。这句话住在这里，好让措辞不会在工具之间漂移；而它的唯一性由
//! `truncation_tests` 钉住。

/// The phrase the sentence starts with, spelled once and pinned as unique.
/// 这句话开头的短语，只拼一次，并被钉住为唯一。
pub(crate) const PHRASE: &str = "… truncated:";

/// The one truncation sentence, with the count that was withheld.
/// 唯一的那句截断说明，带被扣下的数量。
///
/// `omitted` and `total` are the row counts — `omitted of total` is the shape, so a
/// reader can see both how much is missing and how much there was — `limit` is the
/// cap that cut it, `unit` names what was counted, and `hint` is how to reach the
/// rest. Every site that knows its own counts uses this; a site that stopped
/// computing before it could count uses [`withheld_uncounted`].
/// `omitted` 与 `total` 是行数——形状是 `omitted of total`，读者因此既看得见缺了多少、也看得见
/// 本来有多少——`limit` 是切下它的那道上限，`unit` 点名被计数的东西，`hint` 是怎么拿到其余部分。
/// 每个知道自己计数的站点都用这个；在能计数之前就停止计算的站点用 [`withheld_uncounted`]。
pub(crate) fn withheld(
    omitted: usize,
    total: usize,
    limit: usize,
    unit: &str,
    hint: &str,
) -> String {
    format!("{PHRASE} {omitted} of {total} {unit} withheld at the limit of {limit}; {hint}")
}

/// The same sentence when the withheld count was never computed.
/// 同一句话，用于被扣下的数量从未被算出来的情形。
///
/// The preview is the site that needs it: it stops *while* walking a diff, so the
/// number of lines past the cap is not known and computing it would walk the whole
/// change the cap exists to avoid. Saying "the count is not computed" is the honest
/// answer there — the alternative was a sentence that named no number at all.
/// 预览正是需要它的站点：它在遍历 diff 的**途中**停下，因此上限之外还有多少行无从得知，而要算出来
/// 就得走完整份改动——那正是上限要避免的。在那里说"数量未计算"才是诚实的答案——另一个选择是一句
/// 根本不带数字的说明。
pub(crate) fn withheld_uncounted(limit: usize, unit: &str, hint: &str) -> String {
    format!(
        "{PHRASE} further {unit} withheld at the limit of {limit} (the exact count is not \
         computed); {hint}"
    )
}

/// Keep one block of text inside `limit` lines, and say when it did not fit.
/// 把一段文本限制在 `limit` 行之内，并在装不下时说出来。
///
/// The block form of [`withheld`]: `text`'s first `limit` lines are kept and the
/// sentence replaces the rest, so a caller that renders a whole report never has to
/// spell the cap itself.
/// [`withheld`] 的整块形式：保留 `text` 的前 `limit` 行，其余由那句话替代，因此渲染整份报告的
/// 调用方不必自己拼出这个上限。
pub(crate) fn bounded(text: &str, limit: usize, unit: &str, hint: &str) -> String {
    let lines = text.lines().count();
    if lines <= limit {
        return text.to_owned();
    }
    let head = text.lines().take(limit).collect::<Vec<_>>().join("\n");
    format!(
        "{head}\n{}\n",
        withheld(lines - limit, lines, limit, unit, hint)
    )
}

#[cfg(test)]
#[path = "truncation_tests.rs"]
mod truncation_tests;

/// Bound merged text by UTF-8 bytes as well as per-section row limits.
/// 除分段行数限制外，还按 UTF-8 字节限制合并文本。
pub(crate) fn bounded_bytes(text: &str, limit: usize, hint: &str) -> String {
    if text.len() <= limit {
        return text.to_owned();
    }
    let mut end = limit.saturating_sub(320);
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    format!(
        "{}\n{}\n",
        &text[..end],
        withheld(text.len() - end, text.len(), limit, "UTF-8 bytes", hint)
    )
}
