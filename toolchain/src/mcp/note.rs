//! The one outlet for a derived fact that rides on a row about something else.
//! "附带事实"的唯一出口：一条**派生**事实搭在一条本来在讲别的事的行上。
//!
//! Two readers do this. The census's branch column attaches the contract above a dead arm, and
//! `read --whole` attaches each function's name and contract to the line that starts it. What they
//! share is **not the wording** — each
//! keeps its own. What they share is the two rules an attachment has to obey: it is **bounded**, and
//! **when the bound bites it says where the rest is**. Written twice by hand, one of the two ends up
//! truncating silently, and a silent truncation in an attachment reads as "there was nothing more".
//! 有两个读者这么做：总账的分支栏给一条死臂附上它上方的契约；`read --whole` 给每个函数的起始行附上它的名字
//! 与契约。它们共享的**不是措辞**——各自保留自己的。
//! 共享的是附带事实必须守的两条规则：**有界**，以及**上限一旦生效就要说出剩下的在哪**。这两条手写两遍，
//! 其中一遍迟早会静默截断，而附带内容里的静默截断读起来就是"没有更多了"。

/// Numbered lines, bounded, with the way to the rest when the bound bites.
/// 带行号的若干行，有界；上限生效时给出剩下的在哪。
///
/// The numbering carried through is the **source** line each entry sits on, not its position here:
/// a reader follows the number back into the file.
/// 带上的是每一条所在的**源码**行号，不是它在这里的位置：读者要拿这个号回到文件里。
pub(crate) fn numbered(lines: &[(usize, String)], budget: usize, pointer: &str) -> String {
    let shown = lines
        .iter()
        .take(budget)
        .map(|(line, text)| format!("{line}: {text}"))
        .collect::<Vec<_>>()
        .join(" / ");
    if lines.len() > budget {
        format!("{shown} (first {budget} lines; {pointer})")
    } else {
        shown
    }
}

// The `brief` form (the first twelve characters of a fingerprint) was **removed** with its last
// consumer: printing a print turned out to be the wrong idea, and the ledger now says which file
// moved instead of quoting two hashes at the reader. Keeping a shared outlet member with no
// consumer would be dead code the size gate cannot see.
// `brief`（指纹前十二个字符）随它**最后一个消费者一起移除**：把指纹印出来本身就是错的主意，台账现在说
// "哪个文件动了"，而不是把两串散列摆给读者。留一个没有消费者的共享出口成员就是尺寸门禁看不见的死代码。

#[cfg(test)]
#[path = "note_tests.rs"]
mod note_tests;
