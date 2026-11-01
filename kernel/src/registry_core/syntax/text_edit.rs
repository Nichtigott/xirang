//! The byte ranges a rewrite is made of, and the checked way to apply them.
//! 改写由哪些字节区间组成，以及应用它们的带校验方式。
//!
//! A plan is bound to the exact text it was planned against, and a set that does not fit
//! that text — an inverted or out-of-range span, a span inside a UTF-8 character, an
//! overlap — is refused **whole**. Half a rewrite is a corrupted file, so the only two
//! outcomes are the full rewrite and the original bytes.
//! 计划绑定在它据以规划的那份原文上，而与那份文本不匹配的一组替换——反向或越界的区间、劈开 UTF-8
//! 字符的区间、重叠——被**整组**拒绝。改写一半就是损坏的文件，因此只有两种结果：完整的改写，或原文。

/// A byte-range replacement inside the source text it was planned against.
/// 计划所依据的那份源码文本中的一处字节区间替换。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextEdit {
    /// Inclusive byte offset where the replaced text starts.
    /// 被替换文本的起始字节偏移（含）。
    pub start: usize,
    /// Exclusive byte offset where the replaced text ends.
    /// 被替换文本的结束字节偏移（不含）。
    pub end: usize,
    /// The text that replaces `start..end`.
    /// 替换 `start..end` 的文本。
    pub replacement: String,
}

/// What one file's reference rewrite would change, and what it could not decide.
/// 一个文件的引用改写会改什么，以及它判不了什么。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ReferencePlan {
    /// Non-overlapping replacements, in ascending byte order.
    /// 互不重叠的替换，按字节升序。
    pub edits: Vec<TextEdit>,
    /// References that look like they name a moved module but cannot be rewritten
    /// safely; each entry is a human-readable reason.
    /// 看起来命名了被搬模块、但无法安全改写的引用；每项是一条人类可读的理由。
    pub unresolved: Vec<String>,
}

impl ReferencePlan {
    /// Whether the plan would change nothing and left nothing unresolved.
    /// 计划是否什么都不改、也没有未解决项。
    pub fn is_empty(&self) -> bool {
        self.edits.is_empty() && self.unresolved.is_empty()
    }
}

/// Whether every edit fits `source`, on a character boundary, without overlapping.
/// 每一处替换是否都在 `source` 之内、落在字符边界上、且互不重叠。
///
/// This is the precondition [`apply_text_edits`] used to leave to its callers'
/// memory. It is a public function so a caller that wants to report the refusal can;
/// the applying function enforces the same rule by refusing the whole set.
/// 这就是 [`apply_text_edits`] 过去留给调用方记忆的前提。它是公开函数，供想报告这次拒绝的调用方
/// 使用；而应用函数以"整组拒绝"的方式强制同一条规则。
pub fn check_text_edits(source: &str, edits: &[TextEdit]) -> Result<(), String> {
    let mut ordered: Vec<&TextEdit> = edits.iter().collect();
    ordered.sort_by_key(|edit| (edit.start, edit.end));
    let mut previous_end: Option<usize> = None;
    for edit in ordered {
        if edit.start > edit.end {
            return Err(format!(
                "edit range {}..{} is inverted",
                edit.start, edit.end
            ));
        }
        if edit.end > source.len() {
            return Err(format!(
                "edit range {}..{} runs past the {} bytes it was planned against",
                edit.start,
                edit.end,
                source.len()
            ));
        }
        if !source.is_char_boundary(edit.start) || !source.is_char_boundary(edit.end) {
            return Err(format!(
                "edit range {}..{} splits a UTF-8 character",
                edit.start, edit.end
            ));
        }
        if previous_end.is_some_and(|end| edit.start < end) {
            return Err(format!(
                "edit range {}..{} overlaps the previous edit",
                edit.start, edit.end
            ));
        }
        previous_end = Some(edit.end);
    }
    Ok(())
}

/// Apply planned edits to the text they were planned against.
/// 把已规划的替换应用到其所依据的文本。
///
/// Edits are applied from the highest offset down, so an earlier replacement never
/// invalidates a later range. A set that does not fit its own text — an inverted or
/// out-of-range span, a span inside a UTF-8 character, an overlap — is refused
/// **whole**: the source comes back untouched rather than half rewritten. Use
/// [`apply_text_edits_checked`] when the refusal has to be reported.
/// 替换从最高偏移向低应用，因此靠前的替换不会让靠后的区间失效。与自身文本不匹配的一组替换——
/// 反向或越界的区间、劈开 UTF-8 字符的区间、重叠——被**整组**拒绝：原文原样返回，而不是改写一半。
/// 需要报告这次拒绝时用 [`apply_text_edits_checked`]。
pub fn apply_text_edits(source: &str, edits: &[TextEdit]) -> String {
    match apply_text_edits_checked(source, edits) {
        Ok(rewritten) => rewritten,
        Err(_) => source.to_owned(),
    }
}

/// [`apply_text_edits`] with the refusal reported instead of swallowed.
/// [`apply_text_edits`] 的带报告形式：拒绝会被报出而不是吞掉。
pub fn apply_text_edits_checked(source: &str, edits: &[TextEdit]) -> Result<String, String> {
    check_text_edits(source, edits)?;
    let mut ordered: Vec<&TextEdit> = edits.iter().collect();
    ordered.sort_by_key(|edit| std::cmp::Reverse(edit.start));
    let mut output = source.to_owned();
    for edit in ordered {
        output.replace_range(edit.start..edit.end, &edit.replacement);
    }
    Ok(output)
}

#[cfg(test)]
#[path = "text_edit_tests.rs"]
mod text_edit_tests;
