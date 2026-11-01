//! `xirang.why`: one call that gathers the upstream facts a symptom at a line depends on.
//! `xirang.why`：一次调用，把一个位置的症状所依赖的上游事实收齐。
//!
//! The measured failure: a symptom in file Z sent agents hopping — `callgraph` for the callers, then
//! a `read` for the contract, then the registry or the plan for whether the face ships. The seventh
//! round measured that shape directly: the four injected-defect questions took 5, 6, 5 and 8
//! instrument calls against the control's 5, 2, 2 and 2, and the extra hops were exactly the ones
//! this tool answers in one reply.
//! 量到的失败：一个在 Z 文件里的症状逼代理逐跳——`callgraph` 查调用者、`read` 看契约、再问注册面或计划
//! 看这个面到底发不发布。第七轮直接量到这个形状：四道注入缺陷分别花了 5、6、5、8 次仪器调用，而对照是
//! 5、2、2、2，多出来的那几跳正是这个工具一次回答的东西。
//!
//! What it does **not** answer, it names: runtime evidence, and how a grafted subtree's *runtime*
//! state looks. Those are `trace`'s and `grafts`' questions, and the reply says which one to ask.
//! 它**不**回答的，它会点名：运行期证据，以及一条被嫁接子树的**运行期**状态。那些分别归 `trace` 与
//! `grafts`，答案里会写明该问哪一个。
//!
//! The plan half is here too, and it is the half the third hard-bug class turns on: whether the file is
//! one of the sources the build **selected** (a file on disk can be absent from the scope the entry
//! declared, which is exactly what makes a new face vanish), whether a `#[cfg]` gates the definition,
//! and whether a declared graft cut names the face that owns the file. Each of those reads the one
//! implementation that owns it — `build_evidence`'s scope/pruning lines and the graft view's
//! `names_face` — so this cannot disagree with `explain` or `grafts` about the same fact.
//! 计划那一半也在这里，而它正是第三类复杂 bug 所依赖的那一半：这个文件是不是构建**选中**的那些源之一
//! （盘上有的文件可以不在入口声明的作用域里，而那正是新面消失的来路）、定义是否被 `#[cfg]` 门控、以及
//! 有没有一条已声明的 graft 切口点名拥有该文件的面。三条各自读拥有它的那份唯一实现——`build_evidence`
//! 的作用域/剪枝行与 graft 视图的 `names_face`——因此它不会与 `explain` 或 `grafts` 对同一个事实给出
//! 不同答案。
//!
//! online: the upstream facts at a line come from the sources' own text and call graph, neither of which the build publishes.

use std::path::Path;

use serde_json::Value;

use crate::build_method::{DeclaredGraft, declared_grafts};
use crate::mcp::callgraph::{call_matches, caller_note};
use crate::mcp::source_index::{load_one, load_sources};

#[cfg(test)]
#[path = "why_tests.rs"]
mod why_tests;

/// How many doc lines of the contract the reply prints.
/// 答案打印多少行契约文档。
const CONTRACT_LINES: usize = 6;

/// The `#[cfg(…)]` attribute written directly above a definition, with its line.
/// 直接写在定义上方的 `#[cfg(…)]` 属性，及其行号。
///
/// Only the definition's own attributes are read, and the walk stops at the first line that is not a
/// doc comment or an attribute — so a gate sitting on the enclosing **module** is deliberately not
/// reported rather than guessed at, which the emitted line says.
/// 只读定义自己的属性，且走到第一行既不是文档注释也不是属性的地方就停——因此挂在**外层模块**上的门控
/// 有意不报、绝不靠猜，而输出的那一行会说清这一点。
fn gate(source: &str, definition: usize) -> Option<(usize, String)> {
    let lines = source.lines().collect::<Vec<_>>();
    let mut index = definition.saturating_sub(1);
    while index > 0 {
        let above = lines[index - 1].trim();
        if above.starts_with("#[cfg") {
            return Some((index, above.to_owned()));
        }
        if above.starts_with("///") || above.starts_with("#[") {
            index -= 1;
            continue;
        }
        break;
    }
    None
}

/// The registration declaration a line falls inside, by macro name and line span.
/// 某一行落在里面的那个注册面声明（宏名与行区间）。
///
/// Measured need (W8 round): `why --at <face file>:<line>` with a line inside the `control_object!`
/// block answered "no function covers this line" and stopped, and the agent had to ask again with a
/// different line. Twice in one round, on two questions. The fact that the line is inside a
/// declaration is something this tree already knows — the kernel's parser carries the macro's whole
/// span — so the answer was withholding information it had, which is exactly the shape of waste the
/// maintainer named.
/// 量出来的需求（W8 那轮）：`why --at <面文件>:<行>` 给的行号落在 `control_object!` 块里时，答案只说
/// "no function covers this line" 就停了，agent 只能换个行号再问一次。一轮里两次、两道题。而"这一行在
/// 一个声明里"是这棵树**已经知道**的事实——内核解析器带着宏的整个区间——所以那次答案是扣下了自己手里的
/// 信息，这正是维护者点名的那种浪费。
fn declaration_at(source: &str, line: usize) -> Option<(String, usize, usize)> {
    let faces = xirang_kernel::syntax::parse_faces(source).ok()?;
    faces
        .into_iter()
        .find(|face| face.location.line <= line && line <= face.end.line)
        .map(|face| (face.macro_name, face.location.line, face.end.line))
}

/// The plan facts a symptom at one definition depends on: scope, gate, and the declared cut.
/// 一个定义处的症状所依赖的计划事实：作用域、门控、以及已声明的切口。
///
/// Every branch answers rather than staying silent. A missing `source_scope.tsv` is "this tree was
/// never built", a file that no face owns is "the scope names faces, so this file's membership is not
/// decidable here", and an entry with no cut is "nothing replaces this face's slot" — each of which is
/// a fact about this tree, not an absence of an answer.
/// 每一条分支都作答而不是沉默。缺 `source_scope.tsv` 是"这棵树从未构建过"，没有面拥有的文件是"作用域
/// 点名的是面，因此这个文件在不在其中在这里不可判定"，入口没有任何切口是"没有东西替换这个面的槽位"
/// ——每一条都是关于这棵树的事实，而不是没有答案。
fn plan_facts(
    root: &Path,
    relative: &str,
    source: &str,
    definition: usize,
    remaining: &mut Vec<String>,
) -> (Vec<String>, Option<String>) {
    let mut lines = Vec::new();
    lines.push(match gate(source, definition) {
        Some((line, attribute)) => format!(
            "  gate       {attribute} at {relative}:{line} gates this definition (attributes on the \
             enclosing module are not read here)"
        ),
        None => format!(
            "  gate       no `#[cfg]` attribute sits directly above this definition at \
             {relative}:{definition} (a gate on the enclosing module is not read here)"
        ),
    });
    // The owning face is what makes the scope question decidable at all: the build's scope names
    // faces, so a file no face owns has no membership to report.
    // 拥有它的那个面才是让作用域问题可判定的东西：构建的作用域点名的是面，因此没有面拥有的文件没有
    // 可报的成员资格。
    let scope_relative = relative.strip_prefix("src/").unwrap_or(relative);
    let owning = crate::mcp::registry::namespace(root)
        .ok()
        .and_then(|namespace| crate::mcp::resolve::derived_faces(root, &namespace).ok())
        .and_then(|(faces, _)| {
            faces
                .into_iter()
                .find(|face| face.source.trim_start_matches("./") == scope_relative)
        });
    let evidence = crate::mcp::build_evidence::build_evidence(root);
    let Some(face) = owning else {
        lines.push(format!(
            "  scope      `{relative}` is not any registration face's own source, and the build's \
             scope names faces — so whether it ships is `check {{face}}`'s question rather than one \
             this line can answer"
        ));
        // Nothing here can say which face owns this file, so the answer is open and the call that
        // answers it is the one that lists them.
        // 这里没有东西能说出这个文件归哪个面，因此答案是开的，而回答它的那次调用就是列出面清单的那次。
        remaining.push("--call registry".to_owned());
        return (lines, None);
    };
    if evidence.scope.is_none() {
        // A tree that was never built has no published scope; that is a fact, and the call that
        // produces one is `check`.
        // 一棵从未构建过的树没有已发布的作用域；那是一个事实，而产生它的调用是 `check`。
        remaining.push(format!("--call check --face {}", face.path));
    }
    let owning = face.path.clone();
    lines.push(format!(
        "  scope      {}",
        crate::mcp::build_evidence::scope_line(evidence.scope.as_ref(), &face).trim_end()
    ));
    lines.push(format!(
        "  pruning    {}",
        crate::mcp::build_evidence::pruning_line(evidence.pruning.as_deref(), &face).trim_end()
    ));
    // Wiring: the host entry's own declarations, matched to this face by the graft view's rule
    // rather than by a second path comparison written here.
    // 接线：宿主入口自己的声明，按 graft 视图的规则匹配到这个面，而不是在这里另写一次路径比较。
    match declared_grafts(root) {
        Err(reason) => {
            lines.push(format!(
                "  wiring     the host entry's declarations could not be read ({reason})"
            ));
            remaining.push("--call grafts".to_owned());
        }
        Ok(declared) if declared.cuts.is_empty() => lines.push(format!(
            "  wiring     the host entry {} declares no graft cut, so nothing replaces this face's \
             slot (`grafts` reads the same declarations)",
            declared.entry.display()
        )),
        Ok(declared) => {
            let named = declared
                .cuts
                .iter()
                .filter(|cut: &&DeclaredGraft| cut.names_face(&face.path, Some(&face.module)))
                .collect::<Vec<_>>();
            if named.is_empty() {
                lines.push(format!(
                    "  wiring     no declared cut in {} names `{}` — the entry declares {} cut(s), \
                     and a face no cut names is one the build replaces with nothing (`grafts` lists \
                     them)",
                    declared.entry.display(),
                    face.path,
                    declared.cuts.len()
                ));
            } else {
                for cut in named {
                    lines.push(format!(
                        "  wiring     declared cut `{}` → `{}` (full={}) at {}:{}",
                        cut.cut,
                        cut.graft,
                        cut.full,
                        declared.entry.display(),
                        cut.line
                    ));
                }
            }
        }
    }
    (lines, Some(owning))
}

/// Answer "what does this line depend on?" for a `path:line`.
/// 对 `路径:行号` 回答"这一行依赖什么"。
pub(crate) fn why(root: &Path, arguments: &Value) -> Result<String, String> {
    let at = arguments
        .get("at")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|at| !at.is_empty())
        .ok_or_else(|| {
            "why needs `at`: a `path:line` inside this tree — accepted shape: \
             {\"at\":\"<relative path>:<line>\",\"root\":\"<path>\"}; a registration node is \
             `explain {node}`'s question"
                .to_owned()
        })?;
    let Some((relative, line)) = at.rsplit_once(':') else {
        return Err(format!(
            "why needs `at` as `path:line`, got `{at}` — accepted shape: \
             {{\"at\":\"<relative path>:<line>\",\"root\":\"<path>\"}}"
        ));
    };
    let Ok(line) = line.trim().parse::<usize>() else {
        return Err(format!(
            "why needs a line number after the last colon, got `{at}` — accepted shape: \
             {{\"at\":\"<relative path>:<line>\",\"root\":\"<path>\"}}"
        ));
    };
    let file = load_one(root, relative.trim())?;
    let Some(function) = file
        .functions
        .iter()
        .find(|function| function.line <= line && line <= function.end_line)
    else {
        // A line inside a registration declaration is a **different answer**, not a miss: the
        // declaration is the thing that registers this face, and saying so costs one line while
        // leaving it out cost the round two extra calls.
        // 落在注册面声明里的行是**另一个答案**，不是没命中：那条声明正是注册这个面的东西，说出来只花一行，
        // 而不说让那一轮多花了两次调用。
        let inside = declaration_at(&file.source, line).map_or(String::new(), |(macro_name, start, end)| {
            format!(
                "this line is inside the `{macro_name}!` declaration spanning lines {start}-{end}, \
                 not inside a function — that declaration is what registers this face in the tree \
                 (its `kind` is a token in it), and `explain {{node}}` reports what it declares\n"
            )
        });
        let mut lines = Vec::new();
        if let Some(census) = crate::mcp::consistency::tree_census(root) {
            lines.push(census);
        }
        lines.push(format!(
            "no function covers {}:{line} in {}\n{inside}the functions in this file are:\n{}",
            file.relative,
            root.display(),
            file.functions
                .iter()
                .map(|function| format!(
                    "  {} lines {}-{}",
                    function.name, function.line, function.end_line
                ))
                .collect::<Vec<_>>()
                .join("\n")
        ));
        // The answer names the functions, so it is open in a way the reader can close in one call:
        // the first one's own line is a real coordinate, not a template.
        // 这个答案点名了这些函数，因此它是"一次调用就能合上"的开：第一个函数自己的行号是一个真实坐标，
        // 而不是模板。
        let remaining = match file.functions.first() {
            Some(first) => vec![format!(
                "--call read --path {} --line {}",
                file.relative, first.line
            )],
            None => vec![format!("--call inspect --path {}", file.relative)],
        };
        lines.push(
            crate::mcp::tools::closure(false, &remaining)
                .trim_end()
                .to_owned(),
        );
        return Ok(format!("{}\n", lines.join("\n")));
    };

    let mut lines = Vec::new();
    // W1-2: the tree's size rides on the first answer instead of costing an opening `status` call.
    // W1-2：树的大小随第一个答案一起到达，而不是先花一次 `status` 调用。
    if let Some(census) = crate::mcp::consistency::tree_census(root) {
        lines.push(census);
    }
    lines.push(format!(
        "at {}:{} — the definition `{}` (lines {}-{})",
        file.relative, line, function.name, function.line, function.end_line
    ));
    let contract: Vec<(usize, String)> =
        crate::mcp::source_index::contract_lines(&file.source, function.line)
            .into_iter()
            .take(CONTRACT_LINES)
            .collect();
    if contract.is_empty() {
        lines.push("  contract   none: no `///` line sits above this definition".to_owned());
    } else {
        lines.push(
            "  contract   (from the doc above it, which is the promise this tree writes)"
                .to_owned(),
        );
        for (number, text) in &contract {
            lines.push(format!("             {}:{number} {text}", file.relative));
        }
    }

    // Who calls it — the same rule every other reader uses, so this cannot disagree with `callgraph`.
    // 谁调用它 —— 与其它读者同一条规则，因此它不会与 `callgraph` 给出不同答案。
    let sources = load_sources(root)?;
    let mut callers: Vec<String> = Vec::new();
    for source in &sources {
        for caller in &source.functions {
            if caller.calls.iter().any(|call| call_matches(function, call)) {
                let note = caller_note(&file.relative, &source.relative);
                let entry = format!("{}::{}", source.relative, caller.name);
                callers.push(match note {
                    Some(note) => format!("{entry} ({note})"),
                    None => entry,
                });
            }
        }
    }
    callers.sort();
    callers.dedup();
    let unresolved: usize = sources
        .iter()
        .flat_map(|source| &source.functions)
        .map(|caller| caller.unresolved.len())
        .sum();
    if unresolved > 0 {
        lines.push(format!(
            "  unresolved {unresolved} call site(s) kept as unknown; not claimed as callers"
        ));
    }
    if callers.is_empty() {
        // T-13's second layer: the siblings that must follow the same shape. A change to one member of a
        // family is a change to the family, and the fact that decides whether the others still match is
        // not on this line — it is the set of files that define the same name under the same parent.
        // T-13 的第二层：必须跟着同一形状的那些兄弟。改动一个族里的成员就是改动整个族，而"其余成员是否还对得上"
        // 这个事实不在这条定义上——它是同一父目录下定义同一个名字的那组文件。
        // The family convention is `<prefix>/<name>/<name>.rs`, so a face's siblings live under the
        // **grandparent**: taking the immediate parent found none of the nine `paint` definitions this
        // tree actually has — measured by running the line against the fixture, which is why the first
        // version answered "none" on a family of nine.
        // 族约定是 `<前缀>/<名>/<名>.rs`，因此一个面的兄弟住在**祖父**目录下：取直接父目录时，这棵树实际有的那九个
        // `paint` 定义一个都没找到——这是拿真夹具跑这一行量出来的，也是第一版在九个成员的族上答 "none" 的原因。
        let parent = {
            let (directory, file) = relative.rsplit_once('/').unwrap_or(("", relative));
            let stem = file.strip_suffix(".rs").unwrap_or(file);
            match directory.rsplit_once('/') {
                Some((grandparent, last)) if last == stem => grandparent.to_owned(),
                _ => directory.to_owned(),
            }
        };
        let siblings = sources
            .iter()
            .filter(|source| {
                source.relative != relative
                    && !parent.is_empty()
                    && source.relative.starts_with(&format!("{parent}/"))
                    && source
                        .functions
                        .iter()
                        .any(|candidate| candidate.name == function.name)
            })
            .map(|source| source.relative.clone())
            .collect::<Vec<_>>();
        lines.push(if siblings.is_empty() {
        "  siblings   none: no other file under this parent defines this name".to_owned()
    } else {
        format!(
            "  siblings   {} file(s) under {parent} define `{}` too: {} — a change here is a change \
             to the family (`consistency --parent` compares their declared shapes)",
            siblings.len(),
            function.name,
            siblings.join(", ")
        )
    });
        lines.push("  callers    0 in this root".to_owned());
    } else {
        lines.push(format!("  callers    {}", callers.join(", ")));
    }

    // T-13's sixth layer: what pins it. "Nothing names it" is the answer that matters most — it is
    // the difference between a change that a run will catch and one nothing will.
    // T-13 的第六层：是什么钉着它。"没有任何东西点名它"是最要紧的那个答案——它是"这次改动会有一次运行抓住"
    // 与"没有任何东西会抓住"之间的差别。
    let named = crate::mcp::callgraph::names_tests_call(&sources);
    lines.push(if named.iter().any(|call| call == &function.name) {
        format!(
            "  pins       a test names `{}`; `check {{face}}` is the run that shows it still holds",
            function.name
        )
    } else {
        format!(
            "  pins       no test names `{}` — nothing pins this, so a change here is unverified \
             until something does",
            function.name
        )
    });

    // The adoption ledger, when this tree carries one: entries that name this file. The verdict is
    // `adopted`'s to compute; this says only which entries name the file, and says so.
    // 采信台账（这棵树带的话）：点名了这个文件的条目。判定归 `adopted` 算；这里只说哪些条目点了名。
    match crate::mcp::adopted::entries(root) {
        Err(error) => lines.push(format!("  adoption   ledger unreadable ({error})")),
        Ok(entries) => {
            // The parser is the kernel's (`parse_adoption`), reached through `adopted::entries`, so
            // this reader cannot drift from the verdict `adopted` prints. It says only which entries
            // name the file; the verdict is `adopted`'s to compute.
            // 解析器是内核那个（`parse_adoption`），经 `adopted::entries` 到达，因此这个读者不会与
            // `adopted` 打印的判定漂移。它只说哪些条目点名了这个文件；判定归 `adopted` 算。
            let naming: Vec<String> = entries
                .iter()
                .filter(|entry| {
                    entry
                        .files
                        .iter()
                        .any(|named| named.trim() == file.relative)
                })
                .map(|entry| entry.anchor.clone())
                .collect();
            if naming.is_empty() {
                lines.push(if entries.is_empty() {
                    "  adoption   no ledger at .xirang/adopted/entries in this root".to_owned()
                } else {
                    format!(
                        "  adoption   {} entry(ies) in the ledger, none naming this file",
                        entries.len()
                    )
                });
            } else {
                lines.push(format!(
                    "  adoption   {} entry(ies) name this file — anchors: {} (read the verdict with \
                     `adopted`)",
                    naming.len(),
                    naming.join(", ")
                ));
            }
        }
    }

    // The plan half: the facts a symptom's *cause* often sits in, one or three hops away from the
    // code that shows it. Kept after the source facts and before the boundary, so the shape of the
    // reply stays "what this line is, who calls it, what the build did with it, what is not here".
    // 计划那一半：症状的**原因**常常住在这里，离显出症状的代码一到三跳。放在源码事实之后、边界之前，
    // 因此回复的形状仍是"这一行是什么、谁调用它、构建对它做了什么、这里没有什么"。
    let mut remaining: Vec<String> = Vec::new();
    let (plan, owning_face) = plan_facts(root, &file.relative, &file.source, line, &mut remaining);
    lines.extend(plan);
    // W1-2: the `next` line names calls this answer can already fill in — the face that compiles
    // this file, this function's own name, this definition's own first line. A placeholder sends the
    // reader back to the tree to look up what the line was about to hand them.
    // W1-2：`next` 行点名的是这个答案**已经能填好**的调用——编译这个文件的那个面、这个函数自己的名字、
    // 这个定义自己的首行。占位符等于把读者打发回树上，去查这一行本来正要交给他的东西。
    lines.push(
        "not covered here: how a grafted subtree looks at runtime (ask `trace`), and whether the \
         declarations above describe the tree **as it is now** — the scope and pruning lines are the \
         build's own output, and a tree built before the last edit says so on the `scope` line"
            .to_owned(),
    );
    lines.push(match &owning_face {
        Some(face) => format!(
            "next   `--call check --face {face}` runs the face that compiles this file, \
             `--call callgraph --function {}` gives the whole chain, `--call read --path {} --line \
             {}` the body",
            function.name, file.relative, function.line
        ),
        None => format!(
            "next   `--call registry` lists this tree's faces (one of them owns this file), \
             `--call callgraph --function {}` gives the whole chain, `--call read --path {} --line \
             {}` the body",
            function.name, file.relative, function.line
        ),
    });
    lines.push(
        crate::mcp::tools::closure(remaining.is_empty(), &remaining)
            .trim_end()
            .to_owned(),
    );
    Ok(format!("{}\n", lines.join("\n")))
}
