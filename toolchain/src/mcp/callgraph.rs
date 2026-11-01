//! The static call-graph answer, and the bounds it needs.
//! 静态调用图答案，以及它需要的上限。
//!
//! Lifted out of `tools.rs` when a new tool pushed that page into the line
//! ratchet: the catalog and the query implementations belong there, the one answer
//! that grows without limit belongs here, next to its two bounds and their tests.
//! 当一个新工具把 `tools.rs` 推到行数棘轮边上时把它挪出来：目录与查询实现留在那里，而这唯一会
//! 无界增长的答案连同它的两道上限与测试放在这里。
//!
//! The measured failure these bounds exist against: a common name (`new`) had 142
//! definitions in the NichUI corpus, and every call site of that name was listed for
//! each of them, which arrived as a 4.5 MB reply. An answer an agent cannot read is
//! not an answer.
//! 这些上限所针对的实测失败：常见名（`new`）在 NichUI 语料里有 142 个定义，而每个定义都列出该名字
//! 的每一个调用点，最终以 4.5 MB 的回复抵达。代理读不下的答案不算答案。
//!
//! online: the build publishes faces, not call edges; the call graph is a property of the sources as they are now.

use std::path::Path;

use serde_json::Value;

use crate::mcp::source_index::display_list;

#[path = "callgraph_callees.rs"]
mod callees;

#[path = "callgraph_reading.rs"]
mod reading;

#[path = "callgraph_owners.rs"]
mod owners;

#[path = "callgraph_sources.rs"]
mod sources_load;

pub(crate) use owners::{RegistrationOwners, registration_owners};

use sources_load::{labelled_sources, labelled_sources_with};

use crate::mcp::truncation::withheld;
pub(crate) use reading::{caller_note, looks_like_a_test};

/// One printed line of a definition's source, marked as contract or implementation.
/// 定义源码里被打印的一行，标成"契约"或"实现"。
///
/// The doc comment **is** the contract this tree writes down, and the body is what it does; the
/// strongest signal this bridge can print is the pair disagreeing, so the reader should not have to
/// work out which printed line is which. `///` and `//!` are the doc spellings; everything else is
/// code.
/// 文档注释**就是**这棵树写下的契约，而函数体是它实际做的事；本桥能打印的最强信号就是这两者不一致，
/// 因此读者不必自己分辨哪一行是哪种。`///` 与 `//!` 是文档的拼法，其余都是代码。
fn source_line(line: &str) -> String {
    let trimmed = line.trim_start();
    let kind = if trimmed.starts_with("///") || trimmed.starts_with("//!") {
        "contract"
    } else {
        "impl"
    };
    format!("    {kind:<8} {line}\n")
}

pub(crate) fn is_call_to(call: &str, name: &str) -> bool {
    call == name
        || call
            .strip_suffix(name)
            .is_some_and(|prefix| prefix.ends_with("::"))
}

/// Whether a call names this definition. A method with a known owner only matches its qualified
/// target; a bare name remains a suffix match for free functions and for the lexical fallback.
/// 一次调用是否点名这个定义。所属者已知的方法只匹配限定目标；自由函数与词法回退仍按后缀匹配。
pub(crate) fn call_matches(function: &crate::mcp::source_index::Function, call: &str) -> bool {
    if function.owner.is_some() {
        call == function.qualified || call.ends_with(&format!("::{}", function.qualified))
    } else {
        is_call_to(call, &function.qualified)
    }
}

/// Whether a definition answers the query. An owner-qualified query is exact on the owner path; a
/// bare query keeps the old name semantics, because a reader asking for `new` must see both
/// `Entry::new` and `Store::new` and then choose with the printed owner.
/// 定义是否回答查询。带所属者的查询在所属者路径上精确；裸查询保留旧名字语义，因为问 `new` 的读者必须
/// 看到 `Entry::new` 与 `Store::new` 两个定义，再按打印出的所属者选择。
///
/// A definition may carry **more** qualification than the query writes: a C++ definition is
/// `shop::Entry::make` while the call — and therefore often the question — says `Entry::make`. That
/// direction is part of the definition's own spelling (the namespace it was declared in), so it is
/// accepted, and two namespaces then answer together exactly as two owners do. The opposite
/// direction stays refused on purpose: it is the case where the **query** is more qualified than a
/// bare definition, and letting `inner::helper` name the bare `helper` was measured to change
/// answers, so it remains "no static function match".
/// 定义可以带有比查询**更多**的限定：C++ 定义是 `shop::Entry::make`，而调用——因此常常是问题——写的是
/// `Entry::make`。那个方向属于定义自己的拼法（它被声明所在的命名空间），因此接受；于是两个命名空间会像两个
/// 所属者一样一起作答。反方向仍有意拒绝：那是**查询**比裸定义更受限定的情形，让 `inner::helper` 命中裸的
/// `helper` 经实测会改变答案，因此仍答"no static function match"。
fn definition_matches(function: &crate::mcp::source_index::Function, query: &str) -> bool {
    if query.contains("::") {
        // An overloaded definition carries its signature (`Entry::make(int)`); a reader asking for
        // `Entry::make` means the owner and the name, and must see every overload rather than "no
        // static function match". The signature stays on the printed row, so the choice is visible.
        // 重载定义带着签名（`Entry::make(int)`）；而问 `Entry::make` 的读者指的是所属者加名字，必须看到
        // 每一个重载，而不是"no static function match"。签名仍印在那一行上，因此选择是可见的。
        let base = function
            .qualified
            .split('(')
            .next()
            .unwrap_or(&function.qualified);
        base == query
            || base.ends_with(&format!("::{query}"))
            || function.qualified == query
            || query.ends_with(&format!("::{}", function.qualified))
            || function.qualified.ends_with(&format!("::{query}"))
    } else {
        function.name == query || function.qualified.ends_with(&format!("::{query}"))
    }
}

/// Every name a test-looking file in this tree calls, in the order the files were read.
/// 这棵树里每个像测试的文件所调用的名字，按文件被读到的顺序。
///
/// One implementation, two readers: `digest` marks a function "named by a test" with it, and `why`
/// reports whether anything pins the definition a change is about. The rule is the same one the
/// other readers use — a name that appears in a call list — so a second copy would be a second rule.
/// 一份实现、两个读者：`digest` 用它把函数标成"被测试点名"，而 `why` 用它报告"这次改动有没有东西钉着"。
/// 规则与其它读者用的是同一条——出现在调用列表里的名字——因此写第二份就是第二条规则。
pub(crate) fn names_tests_call(sources: &[crate::mcp::source_index::SourceFile]) -> Vec<String> {
    let mut named = Vec::new();
    for source in sources {
        if !looks_like_a_test(&source.relative, &source.source) {
            continue;
        }
        for function in &source.functions {
            named.extend(function.calls.iter().cloned());
        }
    }
    named
}

pub(crate) fn callgraph(root: &Path, arguments: &Value) -> Result<String, String> {
    // One read-only answer: the census header and the call-graph annotations ask for the same
    // members' faces, so they share one derivation instead of walking the tree twice.
    // 一次只读答案：普查抬头与调用图加注问的是同一批成员的面，因此共享一次推导，而不是遍历两遍。
    let _shared = crate::build_method::face_view::ReadOnlyAnswer::begin();
    if arguments.get("orphans").and_then(Value::as_bool) == Some(true) {
        if arguments.get("function").is_some() {
            return Err("pass either `function` or `orphans`, not both".to_owned());
        }
        return orphan_answer(root, arguments);
    }
    let names = match arguments.get("function") {
        Some(Value::String(name)) => vec![name.as_str()],
        Some(Value::Array(items)) if !items.is_empty() && items.len() <= 8 => items
            .iter()
            .map(|item| {
                item.as_str()
                    .ok_or("every `function` must be a non-empty string")
            })
            .collect::<Result<Vec<_>, _>>()?,
        Some(Value::Array(_)) => return Err("`function` takes 1 to 8 names in one call".to_owned()),
        _ => return Err("xirang.callgraph requires function or orphans".to_owned()),
    };
    if names.iter().any(|name| name.trim().is_empty()) {
        return Err("function must not be empty".to_owned());
    }
    // One source snapshot for every name, definition, caller and callee in this request.
    // 本次请求的名字、定义、调用方和被调用方共用一次源码读取。
    let hint = names.first().copied();
    // A name that is not in a file's bytes cannot be a definition or a call site in it, so the
    // index parses only the files whose text mentions one of the queried leaves. The whole tree is
    // still **listed** (the index owes its readers every path), but `syn`-parsing hundreds of
    // unrelated files per question was the dominant cost of a query on this checkout (measured:
    // 42 s for one crate, 71 s at the workspace root, both dominated by parsing rather than I/O).
    // 名字不在文件字节里，就不可能是该文件里的定义或调用点，因此索引只解析文本提到任一被查询叶名的文件。
    // 整棵树仍被**列出**（索引欠读者每一条路径），但每个问题都 `syn` 解析几百个无关文件，是本次检出上查询的
    // 主要成本（实测：单 crate 42 s、工作区根 71 s，两者都以解析而非 I/O 为主）。
    let leaves = names
        .iter()
        .map(|name| name.rsplit("::").next().unwrap_or(name))
        .collect::<Vec<_>>();
    let (labelled, notes) = labelled_sources_with(root, hint, &leaves, &names)?;
    let observed = crate::mcp::trace::recorded_call_edges(root);
    let answers = names
        .iter()
        .map(|name| function_answer(root, arguments, name.trim(), &labelled, &observed))
        .collect::<Result<Vec<_>, _>>()?
        .join("\n");
    if notes.is_empty() {
        Ok(answers)
    } else {
        Ok(format!(
            "language facts unknown:\n{}\n{answers}",
            notes.join("\n")
        ))
    }
}

fn function_answer(
    root: &Path,
    arguments: &Value,
    query: &str,
    labelled: &[(String, crate::mcp::source_index::SourceFile)],
    observed: &[(String, String)],
) -> Result<String, String> {
    const DEFINITIONS: usize = 5;
    const CALLERS: usize = 20;
    let path_filter = arguments.get("path").and_then(Value::as_str);
    let limit = arguments
        .get("limit")
        .and_then(Value::as_u64)
        .map_or(DEFINITIONS, |value| value.clamp(1, 50) as usize);
    let mut found = Vec::new();
    for (label, file) in labelled {
        if path_filter.is_some_and(|path| label != path) {
            continue;
        }
        for function in &file.functions {
            // The definition half asks its own question — *which* definitions match the requested
            // query — and it keeps today's direction on purpose: `is_call_to` receives the
            // definition's spelled name as the spelling and the query as the target, so a definition
            // spelled with a path would be found by a bare query. The reverse direction
            // (`is_call_to(query, &function.name)`), which would let the **query** `inner::helper`
            // name the bare `helper`, is a query-semantics choice rather than a spelling: it was
            // measured to change behaviour (`--function inner::helper` answers "no static function
            // match" today and would hit `helper`), so it is deliberately not taken here.
            // 定义那一半问的是它自己的问题——**哪些**定义匹配这个查询——而它有意保持今天的方向：
            // `is_call_to` 收到的"拼法"是定义的名字、"目标"是查询，因此若有以路径拼写的定义，裸查询也能
            // 找到它。反方向（`is_call_to(query, &function.name)`）会让**查询** `inner::helper` 点名裸的
            // `helper`，那是**查询语义**的取舍而不是拼法：实测它会改变行为（今天 `--function
            // inner::helper` 答"no static function match"，改后命中 `helper`），因此这里有意不取。
            if definition_matches(function, query) {
                found.push((label, function));
            }
        }
    }
    if found.is_empty() {
        return Ok(format!("no static function match for `{query}`"));
    }
    let total = found.len();
    let semantic = found.iter().any(|(_, function)| function.semantic);
    let mut output = if semantic {
        format!(
            "evidence: syntax-resolved (owner-aware where the source writes one; unresolved \n\
             receivers and bare-name candidates are not claimed as edges)\nmatches {total}\n"
        )
    } else {
        format!("evidence: static-heuristic\nmatches {total}\n")
    };
    let unresolved_sites: usize = labelled
        .iter()
        .flat_map(|(_, file)| &file.functions)
        .map(|function| function.unresolved.len())
        .sum();
    if unresolved_sites > 0 {
        output.push_str(&format!(
            "note unresolved call sites: {unresolved_sites} in the files this query read (kept as unknown, not claimed as edges)\n"
        ));
    }
    if total > 1 && path_filter.is_none() {
        // Naming the ambiguity is the difference between a usable answer and a
        // dump: with several definitions the reader has to choose, and `path` is
        // how they choose.
        // 把歧义说出来，是好答案与一坨倾倒之间的区别：有多个定义时读取方必须选一个，而 `path`
        // 就是他们选的工具。
        output.push_str(&format!(
            "note: {total} definitions match `{query}`; pass `path` to select one. Callers are matched \
             by name across the whole tree, so for a common name they include unrelated call sites.\n"
        ));
    }
    // The definitions the cap drops are named, because dropping them **silently** was measured to
    // cost a whole diagnosis: with nine definitions of `offset` and a default limit of five, the one
    // the question was about — `toggle`, the outlier — was exactly the one cut, and the reply said
    // nothing (W8, h1). The reader then recovered only because a *callers* sentence happened to name
    // it. Naming them turns the cap into a choice the reader can act on (`path`), instead of a gap
    // they cannot see.
    // 被上限丢掉的定义要**点名**，因为静默丢掉它们被量到过一次整段诊断的代价：`offset` 有九个定义、
    // 默认上限五个，而问题所关心的那一个——离群者 `toggle`——恰恰是被切掉的那个，回复对此一言不发
    // （W8，h1）。读者之所以还能挽回，只是因为另一条**调用者**的句子碰巧点了它的名。点名它们，把这道上限
    // 变成读者能据以行动的选择（`path`），而不是一个他看不见的缺口。
    let dropped = found
        .iter()
        .skip(limit)
        .take(CALLERS)
        .map(|(label, _)| format!("`{label}`"))
        .collect::<Vec<_>>()
        .join(", ");
    let dropped = if total.saturating_sub(limit) > CALLERS {
        format!(
            "{dropped} (+{} more; narrow with `path`)",
            total - limit - CALLERS
        )
    } else {
        dropped
    };
    for (label, function) in found.into_iter().take(limit) {
        // The matched name is cloned into the inner closure: it runs once per candidate file, and
        // borrowing the definition from the outer scope made the closure outlive it. Whether a call
        // names that definition is `is_call_to`'s business now, as it is for `orphans`, the census's
        // test-reachability column and `affected`.
        // 被匹配的名字被克隆进内层闭包：它每个候选文件跑一次，而从外层作用域借用那个定义会让闭包活得比它
        // 长。这次调用是否点名了那个定义，现在归 `is_call_to` 管——`orphans`、总账的测试可达性栏与
        // `affected` 也一样。
        let target = function;
        let mut callers = labelled
            .iter()
            .flat_map(|(caller_label, candidate)| {
                let prefix = format!("{caller_label}::");
                candidate
                    .functions
                    .iter()
                    .filter(move |caller| {
                        caller.calls.iter().any(|call| call_matches(target, call))
                    })
                    .map(move |caller| format!("{prefix}{}", caller.qualified))
            })
            .collect::<Vec<_>>();
        callers.sort();
        callers.dedup();
        let callers_total = callers.len();
        output.push_str(&format!(
            "{label}:{} fn {}\n",
            function.line, function.qualified
        ));
        if let Some(face) = &function.registration {
            output.push_str(&format!("  registration: {face}\n"));
        }
        if let Some(binding) = &function.binding {
            output.push_str(&format!("  binding: declared graft {binding}\n"));
        }
        if !function.semantic {
            output.push_str("  evidence: lexical fallback (this file did not parse as Rust)\n");
        }
        if function.semantic && !function.unresolved.is_empty() {
            output.push_str(&format!(
                "  unresolved: {}\n",
                display_list(&function.unresolved)
            ));
        }
        // A caveat qualifies a **definite** edge: the reader must not be told the call is unknown
        // (it is not), and must not be left to believe the plain target runs (the decorator may
        // wrap it). It is printed on the definition's own block and, below, on the caller's block.
        // caveat 限定一条**确定**的边：不能让读者以为这次调用是未知（它不是），也不能让读者以为就是
        // 那个目标在跑（装饰器可能包装它）。它印在定义自己的块上，下面也印在调用者的块上。
        for line in callees::definition_caveat_lines(function) {
            output.push_str(&line);
        }
        // Where each caller sits, when there is something to say. Measured in the seventh round:
        // `callers (1): crates/report/tests/report.rs::store` left an agent to send one more `search`
        // to work out that the caller is in another crate's test file.
        // 每个调用者坐在哪里 —— 有话可说时才说。第七轮量到：`callers (1): crates/report/tests/report.rs::store`
        // 让代理又发了一次 `search` 才弄清"调用者在另一个 crate 的测试文件里"。
        let shown: Vec<String> = callers
            .iter()
            .take(CALLERS)
            .map(|caller| {
                let caller_label = caller
                    .rsplit_once("::")
                    .map(|(label, _)| label)
                    .unwrap_or(caller.as_str());
                match caller_note(label, caller_label) {
                    Some(note) => format!("{caller} ({note})"),
                    None => caller.clone(),
                }
            })
            .collect();
        output.push_str(&format!(
            "  callers ({}): {}\n",
            callers_total,
            display_list(&shown)
        ));
        let observed_callers = observed
            .iter()
            .filter(|(_, callee)| callee == &function.name)
            .map(|(caller, _)| caller.clone())
            .collect::<std::collections::BTreeSet<_>>();
        let observed_callees = observed
            .iter()
            .filter(|(caller, _)| caller == &function.name)
            .map(|(_, callee)| callee.clone())
            .collect::<std::collections::BTreeSet<_>>();
        if !observed_callers.is_empty() || !observed_callees.is_empty() {
            output.push_str(&format!(
                "  observed (one recorded run; absence is not evidence): callers [{}] callees [{}]\n",
                display_list(&observed_callers.into_iter().collect::<Vec<_>>()),
                display_list(&observed_callees.into_iter().collect::<Vec<_>>())
            ));
        }
        if callers_total == 0
            && let Some(above) = crate::mcp::workspace::enclosing_workspace(root)
        {
            // Measured: asked at a member root, this census cannot see the other members — and zero
            // callers is exactly the answer a reader takes as "nothing calls it". The note appears
            // only at zero, so it is a warning where it matters rather than noise everywhere.
            // 量到的：在成员根上问时这份普查看不见别的成员——而"0 个调用者"恰恰是读者会当成"没人调用它"的
            // 答案。这句话只在 0 时出现，因此它是**该出现处的警告**，而不是到处都有的噪声。
            output.push_str(&format!(
                "  note  this root is the member `{}` of the workspace at {}; a caller in another \
                 member is invisible here — ask at that root for the whole-workspace answer\n",
                root.file_name().unwrap_or_default().to_string_lossy(),
                above.display()
            ));
        }
        if callers_total > CALLERS {
            // The caller cap is this tool's own constant, not `limit`: no argument
            // raises it, so the sentence has to name the absence rather than advice a
            // caller cannot act on.
            // 调用者上限是本工具自己的常数、不是 `limit`：没有任何参数能提高它，因此那句话必须
            // 点名这个"没有"，而不是给出调用方无法执行的建议。
            output.push_str(&format!(
                "  {}\n",
                withheld(
                    callers_total - CALLERS,
                    callers_total,
                    CALLERS,
                    "callers",
                    "pass `path` to list one definition's callers; no argument raises this cap"
                )
            ));
        }
        // Each callee carries the line this tree defines it at: the measured `callgraph → callgraph`
        // pair (16 forced returns in one arm's two rounds) is a walk where every hop ended with "now
        // where is *that* one defined", which this answer can already know.
        // 每个被调用者在**本树**有定义时带上它的定义行：量到的 `callgraph → callgraph`（一个臂两轮里 16 次
        // 被迫回头）就是这样一次走链——每一跳都以"那它又定义在哪"收尾，而这个答案本来就能知道。
        output.push_str(&format!(
            "  callees: {}\n",
            display_list(&callees::located_callees(function, labelled))
        ));
        // The callee's caveat, at the reader who asked about the **caller**: a decorator on the
        // target is invisible from the call site, and the definition's own block is not where the
        // question was asked.
        // 被调用者的 caveat，放在问**调用者**的读者面前：目标上的装饰器从调用点看不见，而定义自己的
        // 块并不是问题被提出的地方。
        let callee_caveats = callees::callee_caveat_lines(function, labelled);
        if !callee_caveats.is_empty() {
            output.push_str(&format!(
                "  caveat (callee): {}\n",
                display_list(&callee_caveats)
            ));
        }
        // This file's other definitions, with their lines: the measured `callgraph → read` pair (10
        // forced returns in one arm's two rounds) is a caller who got this function and still had to
        // open the file for its neighbours — the "the next hop is in the same file" case.
        // 本文件的其它定义及其行号：量到的 `callgraph → read`（一个臂两轮里 10 次被迫回头）就是"拿到了这个
        // 函数、却还得为它的邻居打开文件"——也就是"下一跳就在同一个文件里"那一族。
        if let Some((label, file)) = labelled.iter().find(|(file_label, _)| file_label == label) {
            let others: Vec<&crate::mcp::source_index::Function> = file
                .functions
                .iter()
                .filter(|seen| seen.qualified != function.qualified)
                .collect();
            if !others.is_empty() {
                const SHOWN: usize = 6;
                let listed = others
                    .iter()
                    .take(SHOWN)
                    .map(|seen| format!("{} -> {label}:{}", seen.qualified, seen.line))
                    .collect::<Vec<_>>()
                    .join(", ");
                let more = if others.len() > SHOWN {
                    format!(" (+{} more in this file)", others.len() - SHOWN)
                } else {
                    String::new()
                };
                output.push_str(&format!("  also here: {listed}{more}\n"));
            }
        }
        // Which tests reference this definition: a name that a test file calls is covered by
        // that file, and a reader deciding what to run needs the file rather than the caller
        // list it already has. The rule for "a test file" is this reader's convenience — a
        // `tests/` directory, a `_tests.rs` sibling, or a file that declares `#[test]` itself —
        // and not the conventions gate's, which the bridge cannot use and which judges placement
        // rather than what a file does.
        // 哪些测试引用了这个定义：某个测试文件调用的名字就由那个文件覆盖，而"该跑什么"的读者要的是文件名，
        // 不是他已经拿到的调用者清单。"测试文件"的判据是这个读取方的便利规则——位于 `tests/` 目录、同级的
        // `_tests.rs`、或文件里带 `#[test]`——不是 conventions 门禁的那条：桥用不了它，而且它判的是位置
        // 而不是文件在做什么。
        let mut tests = labelled
            .iter()
            .filter(|(test_label, candidate)| {
                looks_like_a_test(test_label, &candidate.source)
                    && candidate
                        .functions
                        .iter()
                        .any(|caller| caller.calls.iter().any(|call| call_matches(function, call)))
            })
            .map(|(test_label, _)| test_label.clone())
            .collect::<Vec<_>>();
        tests.sort();
        tests.dedup();
        if !tests.is_empty() {
            output.push_str(&format!(
                "  tests: {}\n",
                display_list(&tests[..tests.len().min(CALLERS)])
            ));
        }
        if arguments
            .get("source")
            .and_then(Value::as_bool)
            .unwrap_or(true)
            && let Some((_, file)) = labelled.iter().find(|(file_label, _)| file_label == label)
        {
            // The definition's own span, capped by this tool's own constant and declared
            // through the one truncation outlet: a reader who asked for source gets source, and
            // one who asked for a summary keeps the summary.
            // 定义自己的那一段，由本工具自己的常数设上限、并走那个唯一的截断出口：要源码的读者拿到源码，
            // 要摘要的读者仍然拿到摘要。
            const SOURCE_LINES: usize = 60;
            let lines = file.source.lines().collect::<Vec<_>>();
            let definition = function.line.saturating_sub(1);
            // Start at the doc comment's first line when there is one: "the doc says A and the code
            // writes not-A" is the strongest signal this bridge can print, and the evaluation's arm
            // hit that pairing in every round — twice over two requests each time, because the doc
            // and the body arrived from different calls.
            // 有文档注释时从它的首行开始："文档说要 A、代码写着 ¬A"是本桥能打印的最强信号，而评测里那一组
            // 每轮都撞到这个配对——每次都要两次请求，因为文档与函数体来自不同的调用。
            let mut first = definition;
            while first > 0 {
                let above = lines[first - 1].trim_start();
                if above.starts_with("///") || above.starts_with("#[") {
                    first -= 1;
                } else {
                    break;
                }
            }
            let last = function.end_line.min(lines.len());
            let shown = last.saturating_sub(first).min(SOURCE_LINES);
            output.push_str("  source:\n");
            for line in &lines[first..first + shown] {
                output.push_str(&source_line(line));
            }
            if last.saturating_sub(first) > shown {
                output.push_str(&format!(
                    "  {}\n",
                    withheld(
                        last - first - shown,
                        last - first,
                        SOURCE_LINES,
                        "lines",
                        "pass `path` and use `xirang.read` for the whole file"
                    )
                ));
            }
        }
    }
    if total > limit {
        // The cap was already announced — what it did not do was name **which** definitions it
        // dropped. Measured (W8, h1): nine definitions of `offset`, a default limit of five, and the
        // one the question was about (`toggle`) among the four cut; the reader only got there because
        // a *callers* sentence happened to name it. Naming the files turns the cap into a choice it
        // can act on: each withheld definition is what `path` takes.
        // 这道上限本来就**有**提示——它没做的是点名它丢掉了**哪些**定义。实测（W8，h1）：`offset` 九个
        // 定义、默认上限五个，而问题所关心的那一个（`toggle`）在被切掉的四个里；读者之所以还能找到，只是
        // 因为另一条**调用者**的句子碰巧点了它的名。点名文件把这道上限变成能据以行动的选择：每条被扣下的
        // 定义正是 `path` 收的东西。
        output.push_str(&format!(
            "{}\n",
            withheld(
                total - limit,
                total,
                limit,
                "definitions",
                &format!("pass `path` for one of: {dropped}; `limit` raises this cap")
            )
        ));
    }
    output.push_str("dynamic dispatch, function pointers, FFI, and runtime branches require live CallTrace evidence.\n");
    // Guidance in the answer, and the measured default: the evaluation's rounds each spent one
    // extra call to see the body after this tool had shown only names. Bodies are therefore
    // included **by default**; a caller who wants the short answer says `source: false` and the
    // answer says how to get them back.
    // 指引放进答案，默认值来自实测：评测每一轮都在这个工具只给了名字之后，多花一次调用去看函数体。
    // 因此函数体**默认包含**；想要短答案的调用方说 `source: false`，而答案会说怎么把它们要回来。
    if arguments.get("source").and_then(Value::as_bool) == Some(false) {
        output.push_str(
            "note   bodies omitted on request: drop `source: false` (or pass source: true) for \
             each definition's own lines\n",
        );
    }
    Ok(output)
}

/// C++ files whose text can contain the queried name; clang is started only for candidates.
/// 文本可能包含被查询名字的 C++ 文件；只为候选启动 clang。
pub(super) fn cpp_sources(
    root: &Path,
    hint: Option<&str>,
) -> (Vec<crate::mcp::source_index::SourceFile>, Vec<String>) {
    let Some(hint) = hint else {
        return (Vec::new(), Vec::new());
    };
    let leaf = hint.rsplit("::").next().unwrap_or(hint);
    if leaf.is_empty() {
        return (Vec::new(), Vec::new());
    }
    crate::mcp::semantic_sources::cpp(root, leaf)
}

/// Python files whose text can contain the queried name; the provider starts one interpreter per
/// candidate, so a query must not pay for every Python file in the checkout.
/// 文本可能包含被查询名字的 Python 文件；提供者每个候选启动一次解释器，因此查询不能为检出里的每个
/// Python 文件付费。
pub(super) fn python_sources(
    root: &Path,
    hint: Option<&str>,
) -> (Vec<crate::mcp::source_index::SourceFile>, Vec<String>) {
    let Some(hint) = hint else {
        return (Vec::new(), Vec::new());
    };
    let leaf = hint.rsplit("::").next().unwrap_or(hint);
    if leaf.is_empty() {
        return (Vec::new(), Vec::new());
    }
    crate::mcp::semantic_sources::python(root, leaf)
}

/// What this package defines and never calls.
/// 本包定义了、却从不调用的那些东西。
///
/// The round asked for this view by name: the trap round's `write_totals` was a textbook dead-code
/// candidate, the table advertised `orphans: true`, and the tool refused it twice — two of that
/// round's ten calls. The count is static and says so: a name counts as called when it appears in
/// some function's call list *in this tree*, so dynamic dispatch, function pointers and
/// macro-expanded calls are invisible to it, exactly as they are to the name view.
/// 这一视图是那轮点名要的：陷阱题里的 `write_totals` 是教科书级死代码候选，而流程表推荐了 `orphans: true`
/// 却被工具拒了两次——那轮 10 次调用里的 2 次。计数是静态的并且说出来：只要一个名字出现在**这棵树**某个函数
/// 的调用清单里就算被调用，因此动态派发、函数指针与宏展开出来的调用对它不可见——与名字视图一样。
fn orphan_answer(root: &Path, arguments: &Value) -> Result<String, String> {
    const ORPHANS: usize = 20;
    let limit = arguments
        .get("limit")
        .and_then(Value::as_u64)
        .map_or(ORPHANS, |value| value.clamp(1, 200) as usize);
    let (labelled, _) = labelled_sources(root, None)?;
    let mut called: Vec<&str> = Vec::new();
    for (_, file) in &labelled {
        for function in &file.functions {
            for call in &function.calls {
                called.push(call.as_str());
            }
        }
    }
    let mut found: Vec<String> = Vec::new();
    // The test half is **counted**, not listed: the harness calls those functions, so listing them
    // as orphans would be a false positive a reader has to undo — but leaving them out silently is
    // what capped both arms of the round-5 evaluation at "partly right" on the orphan question.
    // 测试那一半只**计数**、不列出：测试框架会调用它们，把它们列成孤儿是读者还得自己撤销的假阳性；
    // 而默默略过它们，正是第五轮评测两臂在孤儿题上一同止步于"部分对"的原因。
    let mut test_only: usize = 0;
    for (label, file) in &labelled {
        let is_test = looks_like_a_test(label, &file.source);
        for function in &file.functions {
            if function.name == "main" {
                continue;
            }
            let reached = called
                .iter()
                .any(|call| call == &function.qualified || call_matches(function, call));
            match (reached, is_test) {
                (true, _) => {}
                (false, true) => test_only += 1,
                (false, false) => found.push(format!(
                    "  fn    {} -> {label}:{}",
                    function.name, function.line
                )),
            }
        }
    }
    let total = found.len();
    let mut output = String::from("evidence: static-heuristic\n");
    if let Some(above) = crate::mcp::workspace::enclosing_workspace(root) {
        // Measured: asked at a member root this view calls cross-member callers invisible, so it says
        // who is invisible and where to ask instead rather than presenting them as orphans.
        // 量到的：在成员根上问时，这个视图看不见跨成员的调用者，因此它说清"谁不可见、该去哪儿问"，
        // 而不是把它们当成孤儿报出来。
        output.push_str(&format!(
            "note  this root is the member `{}` of the workspace at {}; callers in other members \
             are invisible here — ask at that root for the whole-workspace answer\n",
            root.file_name().unwrap_or_default().to_string_lossy(),
            above.display()
        ));
    }
    output.push_str(&format!(
        "orphans {total} (defined here, no static caller in this tree)\n"
    ));
    if test_only > 0 {
        // The boundary is stated with its number instead of being silent: "one orphan" and "one
        // orphan plus five functions only the harness calls" are different facts about a tree, and
        // the round measured an answer that could not tell them apart.
        // 边界连同它的数字一起说出来，而不是沉默：对一棵树来说，"一个孤儿"与"一个孤儿外加五个只有
        // 测试框架调用的函数"是两件不同的事实，而那轮量到的正是一个分不清它们的答案。
        output.push_str(&format!(
            "note   {test_only} function(s) in test files have no static caller either; the test \
             harness calls them, so they are counted here rather than listed above\n"
        ));
    }
    for line in found.iter().take(limit) {
        output.push_str(line);
        output.push('\n');
    }
    if total > limit {
        output.push_str(&format!(
            "{}\n",
            withheld(
                total - limit,
                total,
                limit,
                "orphans",
                "raise `limit`, or read the file the row names"
            )
        ));
    }
    output.push_str(
        "note   a name counts as called when it appears in some function's call list in this \
         tree: dynamic dispatch, function pointers and macro-expanded calls are invisible to it\n",
    );
    Ok(output)
}

#[cfg(test)]
#[path = "callgraph_tests.rs"]
mod callgraph_tests;
