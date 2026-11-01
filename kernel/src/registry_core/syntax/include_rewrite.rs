//! Pure planner for the file-relative `include_*!` paths a move relocates.
//! 搬动所迁移的、相对文件的 `include_*!` 路径的纯规划器。
//!
//! Two coordinate systems meet here and used to be confused: the literal in the file
//! is resolved against the directory the **including file has now**, while the
//! replacement has to be resolved against the directory that file **will have** after
//! the move. A path is rewritten only when the pair of them actually changes meaning,
//! so a literal that already names the right file keeps its bytes.
//! 这里有两个坐标系，过去被混为一谈：文件里的字面量要对着**包含文件现在所在**的目录解析，而替换文本
//! 要对着搬动之后该文件**将要在**的目录解析。只有当两者合起来真的改变了含义时才改写，因此本来就指向
//! 正确文件的字面量保持原字节不动。
//!
//! The planner only rewrites file-relative literals. An absolute path (`/…`) names a
//! location outside the package's identity base, and a computed `include!(concat!(…))`
//! cannot be checked at all: both are reported rather than guessed.
//! 规划器只改写相对文件的字面量。绝对路径（`/…`）命名的是包身份基准之外的位置，而计算式的
//! `include!(concat!(…))` 根本无从检查：两者都报告，而不是猜测。

use std::collections::BTreeMap;

use syn::spanned::Spanned;
use syn::visit::Visit;

use super::FaceSyntaxError;
use super::text_edit::ReferencePlan;

/// Plan the `include_str!`/`include_bytes!`/`include!` path literals one file carries.
/// 规划一个文件携带的 `include_str!`/`include_bytes!`/`include!` 路径字面量。
///
/// `file_dir` is the including file's directory **before** the move, relative to the
/// package identity base (empty at the root); `from_dir`/`to_dir` are the moved
/// directory before and after the move in that same base. The referenced file is
/// relocated iff it resolves inside `from_dir`, and the including file is relocated
/// iff it already lives inside `from_dir` — which is why one call decides both a
/// moved asset and an unmoved one.
/// `file_dir` 是包含文件在搬动**之前**的目录，相对包身份基准（根上为空）；`from_dir`/`to_dir` 是同基准
/// 下搬动前后的目录。被引用文件在解析结果落于 `from_dir` 之内时随之搬动，包含文件在本来就住在
/// `from_dir` 之内时随之搬动——这正是同一次调用能同时裁定"资产一起搬"与"资产不搬"两种情形的原因。
pub fn plan_include_rewrites(
    source: &str,
    file_dir: &str,
    from_dir: &str,
    to_dir: &str,
) -> Result<ReferencePlan, FaceSyntaxError> {
    let file = super::nesting::parse_file(source)?;
    let mut planner = IncludePlanner {
        file_dir: file_dir.to_owned(),
        new_file_dir: relocate(file_dir, from_dir, to_dir),
        from_dir: from_dir.to_owned(),
        to_dir: to_dir.to_owned(),
        edits: BTreeMap::new(),
        unresolved: Vec::new(),
    };
    planner.visit_file(&file);
    Ok(planner.finish())
}

/// Where a directory ends up once the move is applied.
/// 一次搬动落定之后，某个目录到了哪里。
///
/// The move's own directory and everything under it are relocated; everything else
/// keeps its place. An empty `from_dir` would mean "the whole identity base moved",
/// which is not a module move, so it leaves the path alone.
/// 被搬目录本身及其下的一切都被搬到新位置；其余保持原位。空的 `from_dir` 意味着"整个身份基准都搬了"，
/// 那不是模块搬动，因此原样返回。
fn relocate(directory: &str, from_dir: &str, to_dir: &str) -> String {
    if from_dir.is_empty() {
        return directory.to_owned();
    }
    if directory == from_dir {
        return to_dir.to_owned();
    }
    match directory.strip_prefix(&format!("{from_dir}/")) {
        Some(rest) => format!("{to_dir}/{rest}"),
        None => directory.to_owned(),
    }
}

/// One file's include rewrites.
/// 一个文件的 include 改写。
struct IncludePlanner {
    file_dir: String,
    new_file_dir: String,
    from_dir: String,
    to_dir: String,
    edits: BTreeMap<(usize, usize), String>,
    unresolved: Vec<String>,
}

impl IncludePlanner {
    fn finish(self) -> ReferencePlan {
        let mut edits: Vec<super::text_edit::TextEdit> = self
            .edits
            .into_iter()
            .map(|((start, end), replacement)| super::text_edit::TextEdit {
                start,
                end,
                replacement,
            })
            .collect();
        edits.sort_by_key(|edit| (edit.start, edit.end));
        ReferencePlan {
            edits,
            unresolved: self.unresolved,
        }
    }

    fn plan_include_macro(&mut self, mac: &syn::Macro) {
        let Some(name) = mac
            .path
            .segments
            .last()
            .map(|segment| segment.ident.to_string())
        else {
            return;
        };
        if !matches!(name.as_str(), "include_str" | "include_bytes" | "include") {
            return;
        }
        let Ok(literal) = syn::parse2::<syn::LitStr>(mac.tokens.clone()) else {
            self.unresolved.push(format!(
                "`{name}!` at line {} does not take a single string literal; its reference cannot \
                 be checked",
                mac.span().start().line
            ));
            return;
        };
        let value = literal.value();
        // An absolute literal is outside the identity base: no amount of moving this
        // file changes what it names, and re-spelling it relative would change it.
        // 绝对字面量在身份基准之外：搬动这个文件不会改变它命名的东西，而改写成相对路径反而会改变它。
        if value.starts_with('/') {
            return;
        }
        let Some(resolved) = join_relative(&self.file_dir, &value) else {
            return;
        };
        let new_target = relocate(&resolved, &self.from_dir, &self.to_dir);
        if new_target == resolved && self.new_file_dir == self.file_dir {
            // The literal and its owner both stay put, so the bytes already say the
            // right thing — including a spelled-out path such as `../slider/data.txt`
            // that a normalising rewrite would "fix" without changing its meaning.
            // 字面量与它的属主都不动，因此这些字节本来就说对了——包括 `../slider/data.txt` 这种拼写
            // 啰嗦的路径：规范化改写会"修"它，却不改变它的含义。
            return;
        }
        let replacement = relative_path(&self.new_file_dir, &new_target);
        if replacement == value {
            return;
        }
        let range = literal.span().byte_range();
        let literal_text = proc_macro2::Literal::string(&replacement).to_string();
        self.edits.insert((range.start, range.end), literal_text);
    }
}

impl<'ast> Visit<'ast> for IncludePlanner {
    fn visit_macro(&mut self, item: &'ast syn::Macro) {
        self.plan_include_macro(item);
    }
}

/// Join a file-relative include path onto a base directory, normalized.
/// 把相对文件的 include 路径拼到基准目录上并规范化。
fn join_relative(base: &str, value: &str) -> Option<String> {
    let mut parts: Vec<&str> = if base.is_empty() {
        Vec::new()
    } else {
        base.split('/').collect()
    };
    for component in value.split('/') {
        match component {
            "" | "." => {}
            ".." => {
                parts.pop()?;
            }
            other => parts.push(other),
        }
    }
    Some(parts.join("/"))
}

/// Render `target` as a path relative to `from_dir`, both package-relative.
/// 把 `target` 渲染成相对 `from_dir` 的路径，两者都相对包。
fn relative_path(from_dir: &str, target: &str) -> String {
    let from: Vec<&str> = if from_dir.is_empty() {
        Vec::new()
    } else {
        from_dir.split('/').collect()
    };
    let to: Vec<&str> = target.split('/').collect();
    let common = from
        .iter()
        .zip(to.iter())
        .take_while(|(left, right)| left == right)
        .count();
    let mut parts = vec![".."; from.len().saturating_sub(common)];
    parts.extend(to[common..].iter().copied());
    if parts.is_empty() {
        ".".to_owned()
    } else {
        parts.join("/")
    }
}

#[cfg(test)]
#[path = "include_rewrite_tests.rs"]
mod include_rewrite_tests;
