//! `xirang.digest`: one bounded summary of one file, so a multi-algorithm file can be read at all.
//! `xirang.digest`：一个文件的**有界**摘要，让"一个文件里好几套算法"也能先看清结构。
//!
//! The failure this answers is the fourth class the maintainer named: a file holding several
//! algorithms where one branch of one of them is wrong, and every part locally plausible. Reading
//! the whole file to find out what is in it is one call, but it is an unbounded one; this reply is
//! bounded, says which functions carry a written contract, which of them a test names, and who calls
//! them — and it says what it does not read.
//! 这条回答的失败是维护者点名的第四类：一个文件里好几套算法、错在其中一套的一个分支上，而每一部分单看
//! 都自洽。读整个文件能知道里面有什么，但那是一次**无界**的调用；这条回复有界，说清哪些函数带着写下来
//! 的契约、哪些被测试点名、谁调用它们 —— 以及它**不**读什么。
//!
//! online: one file's summary is about that file's text right now, which no build-time record carries.

use std::path::Path;

use serde_json::Value;

use crate::mcp::source_index::load_one;

#[cfg(test)]
#[path = "digest_tests.rs"]
mod digest_tests;

/// How many function rows the default answer prints.
/// 默认答案最多打印多少行函数。
const ROWS: usize = 12;

/// The doc lines directly above a definition.
/// 定义正上方的文档行。
/// Answer "what is in this file, in one bounded reply?"
/// 用一次有界的回复回答"这个文件里有什么"。
pub(crate) fn digest(root: &Path, arguments: &Value) -> Result<String, String> {
    let relative = arguments
        .get("file")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|relative| !relative.is_empty())
        .ok_or_else(|| {
            "digest needs `file`: a path as this root sees it — accepted shape: \
             {\"file\":\"<relative path>\",\"root\":\"<path>\"}"
                .to_owned()
        })?;
    let file = load_one(root, relative)?;
    let sources = crate::mcp::source_index::load_sources(root)?;
    // Which functions a test file names: the same name-in-a-call-list rule the other readers use.
    // 哪些函数被测试文件点名：与其它读者同一条"调用列表里有这个名字"的规则。
    let named_by_test: Vec<String> = crate::mcp::callgraph::names_tests_call(&sources);
    let mut lines = vec![format!(
        "file {} — {} function(s), {} line(s)",
        file.relative,
        file.functions.len(),
        file.source.lines().count()
    )];
    for function in file.functions.iter().take(ROWS) {
        let contract = crate::mcp::source_index::contract_lines(&file.source, function.line);
        let tested = named_by_test
            .iter()
            .any(|call| crate::mcp::callgraph::call_matches(function, call));
        let mut callers: Vec<String> = Vec::new();
        for source in &sources {
            for candidate in &source.functions {
                if candidate
                    .calls
                    .iter()
                    .any(|call| crate::mcp::callgraph::call_matches(function, call))
                {
                    callers.push(format!("{}::{}", source.relative, candidate.name));
                }
            }
        }
        callers.sort();
        callers.dedup();
        // A blank `///` is a separator inside a doc block, not contract text: taking it as the first
        // line printed `contract: ` with nothing after it.
        // 空的 `///` 是文档块里的分隔、不是契约正文：把它当首行会印出后面什么都没有的 `contract: `。
        let contract = contract
            .into_iter()
            .filter_map(|(line, text)| {
                let text = crate::mcp::source_index::contract_text(&text);
                (!text.is_empty()).then_some((line, text))
            })
            .collect::<Vec<_>>();
        lines.push(format!(
            "  {}:{}-{} `{}` — {} call(s) out, {} caller(s){}{}",
            file.relative,
            function.line,
            function.end_line,
            function.name,
            function.calls.len(),
            callers.len(),
            if tested { "; named by a test" } else { "" },
            if contract.is_empty() {
                String::new()
            } else {
                format!("; contract: {}", contract[0].1)
            }
        ));
        if !function.unresolved.is_empty() {
            lines.push(format!(
                "    unresolved: {} call site(s) kept as unknown",
                function.unresolved.len()
            ));
        }
    }
    if file.functions.len() > ROWS {
        lines.push(crate::mcp::truncation::withheld(
            file.functions.len() - ROWS,
            file.functions.len(),
            ROWS,
            "function rows",
            "read the file, or ask `callgraph {function}` for one of them",
        ));
    }
    lines.push(
        // The contract half used to be listed here as not covered, while this very reply prints a
        // `contract:` line per function above it — a sentence contradicting its own output, which a
        // reader then learned as boilerplate. What is genuinely not covered is named instead.
        // 契约那半曾经被列在这里当"不覆盖"，而这条回复自己在上面逐函数印了 `contract:` 行——一句与
        // 自己输出矛盾的话，读者随后就把它学成了样板文。这里改为只点名真正不覆盖的。
        "not covered here: which branches are dead and which are covered (ask `check {face}` for \
         the whole-tree census), and whether the functions are reachable from tests by the static \
         walk (that column is in the same census)"
            .to_owned(),
    );
    lines.push(
        // `callgraph` used to be named here, but this reply already prints each function's caller
        // count; what it does not print is the body, so the pointer says only that.
        // 这里曾经点名 `callgraph`，但这条回复已经逐函数印了调用者数量；它没印的是函数体，所以指引
        // 只留那一件事。
        "next   `read {path, line}` for a body, `why --at` for what a line's own definition \
         depends on"
            .to_owned(),
    );
    Ok(format!("{}\n", lines.join("\n")))
}
