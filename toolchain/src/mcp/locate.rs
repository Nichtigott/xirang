//! `xirang.locate`: rank the places a symptom's own words point at, in one call.
//! `xirang.locate`：用症状自己的词，一次把"该去看的地方"排好序。
//!
//! The measured failure this answers: an agent holding a symptom (a failing assertion's message, a
//! runtime error, a description) had no entry that took **words** and returned **places**. It had to
//! guess a symbol name and ask `callgraph`, or guess a phrase and ask `search {literal}` — and the
//! seventh round measured that guessing costs one or two extra round trips per question, which is
//! the axis this bridge is behind on (113 instrument calls against 84, of which the control spends
//! 23 on indexing: answering calls were 113 against 61).
//! 这条回答的实测失败：手上有症状（失败的断言消息、运行期报错、一句描述）的代理，没有任何入口能拿
//! **词**换**地方**。它只能猜一个符号名去问 `callgraph`，或猜一句话去问 `search {literal}` —— 而第七
//! 轮量到"猜"每题要多花一到两个往返，正是本桥落后那一条轴（仪器调用 113 对 84，其中对照有 23 次花在
//! 建索引上：答题调用是 113 对 61）。
//!
//! Ranking uses each function's own name, docs and body (including error messages). Query-local
//! parsing and ranking are shared with explore; neither text overlap nor a test hit proves a defect.
//! 排序读取每个函数自己的名字、文档和函数体（包括报错文字）。请求内解析与 explore 共用；文字重叠或测试
//! 命中都不证明缺陷。
//!
//! online: ranking the places a symptom's words point at reads the sources as they are now, including files the build never saw.

use std::path::Path;

use serde_json::Value;

use crate::mcp::source_query::{self, QuerySources, words};
use crate::mcp::truncation::withheld;

#[cfg(test)]
#[path = "locate_tests.rs"]
mod locate_tests;

/// How many candidates the default answer lists.
/// 默认答案列出多少个候选。
const CANDIDATES: usize = 5;

/// Answer "where should I look for this symptom?" from the symptom's own words.
/// 用症状自己的词回答"这个症状该去哪里看"。
pub(crate) fn locate(root: &Path, arguments: &Value) -> Result<String, String> {
    let symptom = arguments
        .get("symptom")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|symptom| !symptom.is_empty())
        .ok_or_else(|| {
            "locate needs `symptom`: the words of the symptom (a failing assertion's message, an \
             error, a description) — accepted shape: {\"symptom\":\"<words>\",\"root\":\"<path>\"}"
                .to_owned()
        })?;
    let wanted = words(symptom);
    if wanted.is_empty() {
        return Ok(format!(
            "no matches in {}: every word of the symptom is a stopword or shorter than three \
             characters, so there is nothing to rank — pass the symptom's own distinctive words, or \
             `search {{literal}}` for an exact phrase\n",
            root.display()
        ));
    }
    let mut context = QuerySources::current(root)?;
    context.parse_matching(|file| source_query::mentions(&file.source, &wanted));
    let rows = source_query::rank(&context.files, symptom, &wanted);
    let total = rows.len();
    if total == 0 {
        // Measured (W8, h4): the arm spent a call here on a symptom that was about **a whole file**
        // ("one algorithm in one file does the opposite of its documentation"), and the reply named
        // neither that this is a file-level question nor the tools that answer one. Zero matches is
        // the decidable signal, so the pointer costs nothing on a query that did match.
        // 量到的（W8 的 h4）：那一臂在这里花掉一次调用，而它的症状说的是**一整个文件**（"一个文件里有一套
        // 算法与它自己的文档相反"），回复既没说这是文件级问题，也没说哪些工具答得了。零命中就是可判定的
        // 信号，因此在真正命中的查询上，这条指引不花任何代价。
        return Ok(format!(
            "no matches in {}: none of the symptom's words is in a function name, its docs or body in \
             this tree — pass fewer or different words, or `search {{literal}}` for a phrase. **If the \
             symptom is a failing test, `check`'s `failed` line is the symptom's own words** — this \
             tool ranks *source* words, so paste that line's phrase rather than the framework's summary. \
             **If the symptom is about a whole file** rather than a symbol (a file whose algorithms \
             contradict its own docs, a file nothing calls), that is a file-level question, and it is \
             answered by dumping the file rather than by ranking words: `--call read --path <file> \
             --whole` prints it with its symbols, `--call digest --file <file>` summarises it, and \
             `--call check --face <face>` runs a face\n",
            root.display()
        ));
    }
    let mut lines = vec![format!(
        "symptom \"{symptom}\" — {total} candidate(s), ranked by word overlap with the names and the \
         docs and bodies of this tree's functions"
    )];
    for row in rows.iter().take(CANDIDATES) {
        let reason = if row.reasons.is_empty() {
            String::new()
        } else {
            format!(" ({})", row.reasons.join("; "))
        };
        lines.push(format!(
            "  {}:{} `{}`{}",
            context.files[row.file_id].relative,
            context.files[row.file_id].functions[row.function_id].line,
            context.files[row.file_id].functions[row.function_id].name,
            reason
        ));
    }
    if total > CANDIDATES {
        lines.push(withheld(
            total - CANDIDATES,
            total,
            CANDIDATES,
            "candidate locations",
            "pass a narrower word from the symptom, or `read {path, line}` the top few",
        ));
    }
    lines.push(
        "not covered by the ranking: it reads text only, so a paraphrase that shares no word with \
         the name, docs or body finds nothing; dynamic dispatch, macro expansion and generated code are \
         invisible; and a hit is a place to look, not the defect"
            .to_owned(),
    );
    // W4-7: the routing is stated on the answer, with the file the ranking just put first. A
    // file-level symptom ("one algorithm in one file contradicts its docs") is answered by the whole
    // file, not by a window around one line, so the call that dumps it rides here rather than being
    // left to the reader to work out from the shape of the question.
    // W4-7：路由写在答案上，并用**排名第一的那个文件**实例化。文件级症状（"一个文件里有一套算法与它自己的
    // 文档相反"）由整份文件来回答，而不是由某一行周围的窗口，因此把文件倒出来的那次调用搭在这里，而不是
    // 留给读者自己从问题的形状里推。
    let top = rows
        .first()
        .map(|row| context.files[row.file_id].relative.as_str())
        .unwrap_or_default();
    lines.push(format!(
        "next   `--call read --path {} --line {}` for the body (the reply prints the contract lines \
         too), `--call callgraph --function {}` for who calls it",
        top,
        rows.first().map_or(1, |row| context.files[row.file_id].functions[row.function_id].line),
        rows.first().map(|row| context.files[row.file_id].functions[row.function_id].name.as_str()).unwrap_or_default()
    ));
    lines.push(format!(
        "shape  understanding question (what does this file do / why is it like this) -> dump the \
         whole file: `--call read --path {top} --whole`; structural question (what is in it, who \
         calls it) -> the bounded rows above"
    ));
    Ok(format!("{}\n", lines.join("\n")))
}
