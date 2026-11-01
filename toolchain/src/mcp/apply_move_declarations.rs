//! The host-entry declaration rewrites a move plans, and the transaction that writes them.
//! 搬动规划的宿主入口声明改写，以及写下它们的那个事务。
//!
//! Split out of `apply_move_grafts.rs` to keep that file inside the repository's size ratchet. The
//! grouping is a correctness step, not a convenience: two entries in one host file must compose
//! against one buffer, or the second write starts from the pre-first-write bytes and silently undoes
//! the first.
//! 从 `apply_move_grafts.rs` 拆出，使那个文件留在仓库的尺寸棘轮之内。分组是正确性的一步，不是便利：
//! 同一个宿主文件里的两条条目必须作用在同一份缓冲区上，否则第二次写会从第一次写之前的字节出发，静默把
//! 第一次改回去。

use super::DeclarationEdit;

/// Plan every declaration rewrite, grouped **per file**, so two entries in one host file compose.
/// 按**文件**把每条声明改写规划好，使同一个宿主文件里的两条条目能够叠加。
///
/// This is also a correctness step, not only a convenience: applying two edits to one file by
/// reading it twice would have the second write start from the pre-first-write bytes and silently
/// undo the first. The grouping reads each file once and replays every edit against the same buffer.
/// 这不只是便利，也是正确性的一步：对同一个文件读两次各改一处，第二次写会从第一次写之前的字节出发，
/// 静默把第一次改回去。分组之后每个文件只读一次，所有编辑都作用在同一份缓冲区上。
pub(crate) fn plan_declaration_rewrites(
    edits: &[DeclarationEdit],
) -> Result<Vec<DeclarationRewrite>, String> {
    let mut rewrites: Vec<DeclarationRewrite> = Vec::new();
    for edit in edits {
        let position = match rewrites
            .iter()
            .position(|rewrite| rewrite.file == edit.file)
        {
            Some(position) => position,
            None => {
                let original = std::fs::read_to_string(&edit.file)
                    .map_err(|error| format!("cannot read {}: {error}", edit.file.display()))?;
                rewrites.push(DeclarationRewrite {
                    file: edit.file.clone(),
                    text: original,
                    rewritten: Vec::new(),
                });
                rewrites.len() - 1
            }
        };
        let rewrite = &mut rewrites[position];
        let range = match edit.span {
            Some(range) => range,
            None => locate_on_line(&rewrite.text, edit)?,
        };
        // The parser reported the range, so the writer must still find the same bytes there: a
        // moved line or an earlier edit in the same call would otherwise redirect the edit.
        // 解析器报告了范围，写入方必须在同一位置仍找到同样的字节：行号漂移或同一次调用里更早的编辑，
        // 否则会把编辑指到别处。
        let Some(found) = rewrite.text.get(range.0..range.1) else {
            return Err(format!(
                "cannot rebind {}:{}: the reported source range is outside the file",
                edit.file.display(),
                edit.line
            ));
        };
        if found != edit.before {
            return Err(format!(
                "cannot rebind {}:{}: the source now says `{found}`, not `{}` — the preview and the \
                 file disagree; re-read it and try again",
                edit.file.display(),
                edit.line,
                edit.before
            ));
        }
        rewrite.rewritten.push(Rewrite {
            line: edit.line,
            before: edit.before.clone(),
            after: edit.after.clone(),
            range,
            verify: edit.verify.clone(),
            verify_absent: edit.verify_absent.clone(),
        });
    }
    for rewrite in &mut rewrites {
        apply_ranges(rewrite)?;
        verify_endpoints(rewrite)?;
    }
    Ok(rewrites)
}

/// The byte range of the one occurrence of `edit.before` on its line.
/// `edit.before` 在那一行上唯一一次出现的字节范围。
fn locate_on_line(text: &str, edit: &DeclarationEdit) -> Result<(usize, usize), String> {
    let mut start = 0;
    for (index, line) in text.split('\n').enumerate() {
        if index + 1 == edit.line {
            if line.matches(&edit.before).count() != 1 {
                return Err(format!(
                    "cannot rebind {}:{}: `{}` does not appear exactly once on that line; rebind it \
                     by hand so the declaration means what you intend",
                    edit.file.display(),
                    edit.line,
                    edit.before
                ));
            }
            let offset = line
                .find(&edit.before)
                .expect("a single match has a position");
            return Ok((start + offset, start + offset + edit.before.len()));
        }
        start += line.len() + 1;
    }
    Err(format!(
        "cannot rebind {}:{}: the entry no longer has that line",
        edit.file.display(),
        edit.line
    ))
}

/// Apply this file's rewrites from the end, so earlier ranges stay valid.
/// 从文件末尾向前应用本文件的改写，使更早的范围保持有效。
fn apply_ranges(rewrite: &mut DeclarationRewrite) -> Result<(), String> {
    let mut order = rewrite.rewritten.clone();
    order.sort_by_key(|edit| std::cmp::Reverse(edit.range.0));
    for (index, edit) in order.iter().enumerate() {
        if index > 0 && order[index - 1].range.0 < edit.range.1 {
            return Err(format!(
                "cannot rebind {}:{}: two edits overlap in one declaration",
                rewrite.file.display(),
                edit.line
            ));
        }
        rewrite
            .text
            .replace_range(edit.range.0..edit.range.1, &edit.after);
    }
    Ok(())
}

/// An endpoint without `crate::`/`self::`, matching the parser's comparison.
/// 去掉 `crate::`/`self::` 的端点，与解析器的比较一致。
fn normalized(endpoint: &str) -> String {
    endpoint
        .trim()
        .strip_prefix("crate::")
        .or_else(|| endpoint.trim().strip_prefix("self::"))
        .unwrap_or(endpoint.trim())
        .to_owned()
}

/// Re-parse the rewritten entry and confirm the endpoints resolve to the expected identities.
/// 重新解析改写后的入口，确认端点解析到预期身份。
fn verify_endpoints(rewrite: &mut DeclarationRewrite) -> Result<(), String> {
    let expected: Vec<&String> = rewrite
        .rewritten
        .iter()
        .filter_map(|edit| edit.verify.as_ref())
        .collect();
    let absent: Vec<&String> = rewrite
        .rewritten
        .iter()
        .filter_map(|edit| edit.verify_absent.as_ref())
        .collect();
    if expected.is_empty() {
        return Ok(());
    }
    let entries = xirang_kernel::syntax::graft_entries(&rewrite.text).map_err(|error| {
        format!(
            "cannot rebind {}: the rewritten entry no longer parses ({error}); nothing changed",
            rewrite.file.display()
        )
    })?;
    let mut seen: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    for entry in &entries {
        // A typed entry carries the normalized identities in `expressions`; `cut`/`graft` keep the
        // host's spelling, and the catalog form is exactly the case where those differ.
        // 类型化条目的归一化身份在 `expressions` 里；`cut`/`graft` 保留宿主的拼写，而目录形式正是两者
        // 不同的那种情形。
        let endpoints: Vec<&String> = match &entry.expressions {
            Some(expressions) => [
                Some(&expressions.cut),
                expressions.cut_end.as_ref(),
                Some(&expressions.graft),
            ]
            .into_iter()
            .flatten()
            .collect(),
            None => [Some(&entry.cut), entry.cut_end.as_ref(), Some(&entry.graft)]
                .into_iter()
                .flatten()
                .collect(),
        };
        for endpoint in endpoints {
            seen.insert(normalized(endpoint));
        }
    }
    for wanted in expected {
        if !seen.contains(wanted) {
            return Err(format!(
                "cannot rebind {}: after the edit no endpoint resolves to `{wanted}`; nothing changed",
                rewrite.file.display()
            ));
        }
    }
    for gone in absent {
        if seen.contains(gone) {
            return Err(format!(
                "cannot rebind {}: after the edit an endpoint still names `{gone}`; nothing changed",
                rewrite.file.display()
            ));
        }
    }
    Ok(())
}
/// The bytes one file ends up with after every planned declaration edit.
/// 一个文件在所有规划的声明改写之后得到的那份字节。
/// One applied rewrite, with the range it replaced and what the result must resolve to.
/// 一条已应用的改写，连同它替换的范围与结果必须解析到的对象。
#[derive(Clone)]
pub(crate) struct Rewrite {
    /// 1-based line the parser reported.
    /// 解析器报告的 1 起始行号。
    line: usize,
    /// The exact old text at the range.
    /// 范围内原来的精确文本。
    before: String,
    /// What replaced it.
    /// 替换它的文本。
    after: String,
    /// The byte range replaced.
    /// 被替换的字节范围。
    range: (usize, usize),
    /// The normalized endpoint the file must now contain.
    /// 文件现在必须含有的归一化端点。
    verify: Option<String>,
    /// The normalized endpoint the file must no longer contain.
    /// 文件不得再含有的归一化端点。
    verify_absent: Option<String>,
}

pub(crate) struct DeclarationRewrite {
    /// The host entry file.
    /// 宿主入口文件。
    pub(crate) file: std::path::PathBuf,
    /// Its text, with every planned edit already applied.
    /// 它的文本，所有规划的编辑已经应用。
    text: String,
    /// The reply lines describing what this file's edits did.
    /// 描述这个文件的编辑改了什么，供回复使用。
    pub(crate) rewritten: Vec<Rewrite>,
}

impl Rewrite {
    /// The reply line for this change.
    /// 这次改动的回复行。
    pub(crate) fn describe(&self, file: &std::path::Path) -> String {
        format!(
            "rebound {}:{}: {} -> {}",
            file.display(),
            self.line,
            self.before,
            self.after
        )
    }
}

impl DeclarationRewrite {
    /// The file's new text.
    /// 该文件的新文本。
    pub(crate) fn text(&self) -> String {
        self.text.clone()
    }
}

/// Apply the planned declaration rewrites through one transaction.
/// 经同一个事务应用规划好的声明改写。
///
/// Each file is captured **before** it is written, so a failure part-way through the set — including a
/// write that fails after truncating — is restored by the caller's rollback instead of leaving the
/// host entry half-rewritten.
/// 每个文件都在写入**之前**保存，因此这一组的任何一步失败——包括截断之后才失败的写入——都由调用方的
/// 回滚恢复，而不是把宿主入口留在改了一半的状态。
pub(crate) fn apply_declaration_edits_in(
    edits: &[DeclarationEdit],
    transaction: &mut crate::run_method::authoring::write_transaction::WriteTransaction,
) -> Result<Vec<String>, String> {
    let rewrites = plan_declaration_rewrites(edits)?;
    let mut rewritten = Vec::new();
    for rewrite in rewrites {
        transaction.write(&rewrite.file, rewrite.text().as_bytes())?;
        rewritten.extend(
            rewrite
                .rewritten
                .iter()
                .map(|edit| edit.describe(&rewrite.file)),
        );
    }
    Ok(rewritten)
}
