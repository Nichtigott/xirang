//! Search query methods owned by App.
//! App 所有的搜索查询方法。

use super::*;

use crate::run_method::registry_core::declaration::source_file_matches;
use crate::run_method::source::item_symbols;

impl App {
    /// Flat rows for one search query, with **adjacent** duplicates collapsed.
    /// 一次搜索查询的扁平结果行，**相邻**重复项被合并。
    ///
    /// `dedup_by` only folds neighbours, so this is not a promise of global
    /// uniqueness; the wording says so, because a later reader who needs every row
    /// distinct has to sort or key the list instead of assuming this call did it.
    /// `dedup_by` 只合并相邻项，因此这里承诺的不是全局唯一；措辞如实如此——后来需要"每行
    /// 都不同"的读者必须自行排序或建键，而不是假设这一次调用已经做了。
    ///
    /// The list is intentionally flat: there is no hierarchy to fold, so the
    /// caller has no fold state and `Enter` promotes a row straight into the
    /// call graph. The fold keys and `▸/▾` markers that used to be advertised
    /// were removed rather than implemented.
    /// 结果是刻意扁平的：没有可折叠的层级，因此调用方不持有折叠状态，`Enter`
    /// 直接把选中行推进调用图。此前展示的折叠键与 `▸/▾` 标记已删除，而非实现。
    pub fn search_rows(&self, query: &str) -> Vec<SearchRow> {
        // One scan per (query, snapshot), not one per panel: the result list, the
        // RELATION column, the SELECTED SYMBOL preview and the graph page all ask
        // `search_rows` in the same frame, and every call used to read and lex each
        // face's source file again (audit `STU-S-05`). The stamp in the key is what
        // makes an external edit invalidate the memo; the shape is `CallTreeMemo`'s.
        // 每个（查询, 快照）只扫一次，而不是每个面板一次：结果列表、RELATION 栏、SELECTED
        // SYMBOL 预览与调用图页在同一帧都要问 `search_rows`，而过去每次调用都会重新读取并对每个
        // 注册面的源文件做一遍词法扫描（审计 `STU-S-05`）。键里的戳正是让外部编辑使其失效的东西；
        // 形状沿用 `CallTreeMemo`。
        if let Some((memo_query, stamp, rows)) = self.search_memo.borrow().as_ref()
            && memo_query == query
            && *stamp == self.last_source_stamp
        {
            return rows.as_ref().clone();
        }
        let mut rows = self.source_symbol_rows(query);
        rows.dedup_by(|left, right| {
            left.node == right.node && left.function == right.function && left.line == right.line
        });
        *self.search_memo.borrow_mut() = Some((
            query.to_owned(),
            self.last_source_stamp,
            std::rc::Rc::new(rows.clone()),
        ));
        rows
    }

    /// Add compact source symbols when a file or face is searched.
    /// 搜索文件或注册面时，补充紧凑的源代码符号项。
    fn source_symbol_rows(&self, query: &str) -> Vec<SearchRow> {
        let needle = query.trim().to_ascii_lowercase();
        if needle.is_empty() {
            return Vec::new();
        }
        let mut rows = Vec::new();
        for info in self.registry.depth_first() {
            let path = self.registry.path_for(info.id).unwrap_or_default();
            let source = source_path_for(&info.source.file);
            let file_match = source_file_matches(&info.source.file, &needle)
                || path.to_ascii_lowercase().contains(&needle)
                || info.registry_name.to_ascii_lowercase().contains(&needle)
                || info.kind.to_ascii_lowercase().contains(&needle);
            let Ok(text) = std::fs::read_to_string(source) else {
                continue;
            };
            if file_match {
                rows.push(SearchRow {
                    depth: 0,
                    node: Some(info.id),
                    path: info.source.file.to_owned(),
                    function: String::new(),
                    line: Some(1),
                    signature: String::new(),
                    text: info.source.file.to_owned(),
                });
            }
            rows.extend(source_rows_for_text(
                &text,
                &info.source.file,
                info.id,
                &info.source.function,
                file_match,
                &needle,
            ));
        }
        rows
    }

    pub(super) fn source_function_line(&self, node: NodeId, function: &str) -> Option<u32> {
        let info = self.registry.find_registry(node)?;
        function_line(&info.source.file, function)
    }
}

/// The rows one source file contributes to a search, rendered from the kernel's item
/// list.
/// 一份源文件为搜索贡献的行，渲染自内核的条目清单。
///
/// The declaration vocabulary is the kernel's (`item_symbols`), not a prefix table
/// spelled here: Studio carried such a table, and it missed qualified declarations
/// (`pub(crate) struct`, a bodyless `pub(crate) fn`) while promoting `let` bindings to
/// symbols that `call_relations` can never find (audit `STU-S-06`). This function only
/// renders what the kernel reports and applies the query filter.
/// 声明词表归内核（`item_symbols`），不是在这里拼的前缀表：Studio 曾有这样一张表，它漏掉带
/// 可见性的声明（`pub(crate) struct`、没有函数体的 `pub(crate) fn`），又把 `let` 绑定提升成
/// `call_relations` 永远找不到的符号（审计 `STU-S-06`）。本函数只渲染内核报告的条目并施加查询
/// 过滤。
fn source_rows_for_text(
    text: &str,
    file: &str,
    node: NodeId,
    declared_function: &str,
    file_match: bool,
    needle: &str,
) -> Vec<SearchRow> {
    let mut rows = Vec::new();
    for item in item_symbols(text) {
        if !file_match
            && !declared_function.to_ascii_lowercase().contains(needle)
            && !item.name.to_ascii_lowercase().contains(needle)
            && !item.signature.to_ascii_lowercase().contains(needle)
        {
            continue;
        }
        rows.push(SearchRow {
            depth: 1,
            node: Some(node),
            path: file.to_owned(),
            function: item.name.clone(),
            line: Some(item.line),
            signature: item.signature.clone(),
            text: if item.is_function {
                format!("{file} -> fn {}", item.name)
            } else {
                format!("{file} -> {}", item.name)
            },
        });
    }
    rows
}

#[cfg(test)]
mod source_rows_tests {
    //! The declaration vocabulary this file renders is the kernel's (audit `STU-S-06`).
    //! 本文件渲染的声明词表归内核所有（审计 `STU-S-06`）。

    use super::*;

    /// The shapes the old word table got wrong: a qualified bodyless `fn` and an
    /// `unsafe` bodyless `fn` (neither is a function with a body, and neither is a
    /// prefix the table spelled), plus a `let` binding (which the table did spell).
    /// 旧词表弄错的那些形状：没有函数体的 `pub(crate) fn` 与 `unsafe fn`（既不是有函数体的函数，
    /// 也不是词表拼出的前缀），外加 `let` 绑定（词表恰恰拼了它）。
    const SOURCE: &str = "\
pub(crate) async fn load(value: usize) {}\n\
impl Widget {\n\
    pub(crate) fn measure(&self) -> usize;\n\
    unsafe fn render(&self) -> u32;\n\
}\n\
pub(crate) struct Panel;\n\
fn outer() {\n\
    let text = \"load\";\n\
}\n";

    /// The names the rows report for one query.
    /// 某次查询下各行报告的名字。
    fn names(needle: &str) -> Vec<String> {
        source_rows_for_text(
            SOURCE,
            "src/panel.rs",
            crate::run_method::ROOT_NODE_ID,
            "outer",
            false,
            needle,
        )
        .into_iter()
        .map(|row| row.function)
        .collect()
    }

    /// A qualified `fn` without a body is a symbol: the kernel lists it.
    /// 没有函数体的带可见性 `fn` 是符号：内核会列出它。
    #[test]
    fn a_bodyless_qualified_fn_is_a_symbol() {
        assert_eq!(
            names("pub(crate) fn measure"),
            ["measure"],
            "the kernel's item list must report the bodyless qualified function"
        );
    }

    /// An `unsafe fn` without a body is a symbol too.
    /// 没有函数体的 `unsafe fn` 同样是符号。
    #[test]
    fn a_bodyless_unsafe_fn_is_a_symbol() {
        assert_eq!(
            names("unsafe fn render"),
            ["render"],
            "the kernel's item list must report the bodyless unsafe function"
        );
    }

    /// A `let` binding is not a symbol, however its text is written.
    /// `let` 绑定不是符号，无论它的文本怎么写。
    #[test]
    fn a_let_binding_is_not_a_symbol() {
        assert!(
            names("text").is_empty(),
            "a `let` binding must not become a symbol row"
        );
    }

    /// The `struct` shape the old table could not spell with a qualifier.
    /// 旧词表拼不出带可见性的 `struct` 形状。
    #[test]
    fn a_qualified_struct_is_a_symbol() {
        assert_eq!(names("pub(crate) struct"), ["Panel"]);
    }
}
