//! `xirang.search`: what a name is in the sources, and what the tree says it
//! became.
//! `xirang.search`：一个名字在源码里是什么，以及树说它变成了什么。
//!
//! The source index answers "which file or function has this name"; it said
//! nothing about the registry, which is why an agent used to grep macro names and
//! reconstruct the tree — the drift `xirang.registry` exists to remove. A search
//! that stops at source text also cannot say whether the face it found is still
//! the one the build published, so this page answers both halves in one call: the
//! faces whose logical path, `kind`, module or `registry_name` match, each annotated
//! `ok`, `added since build`, `re-identified` or `build unknown`, followed by the
//! file and function hits exactly as before.
//! 源码索引回答"哪个文件或函数叫这个名字"；它对注册树一无所知，这正是代理过去靠 grep 宏名、自己
//! 重建那棵树的原因——而那正是 `xirang.registry` 要消除的漂移。止步于源码文本的搜索也说不出它找到
//! 的面是否仍是构建发布的那一个，因此本页一次回答两半：逻辑路径、`kind`、模块或 `registry_name` 匹配的面，各自
//! 标注 `ok`、`added since build`、`re-identified` 或 `build unknown`；随后是与此前完全相同的文件与
//! 函数命中。
//!
//! The verdicts come from `crate::mcp::tree_delta`, the rule `xirang.diff` states, so
//! the same face cannot be `added` here and something else there.
//!
//! **A root whose identity namespace cannot be learned splits by question.** A **name** query
//! still answers from the sources, because the file walk does not need the package, and it says
//! the tree half is unavailable — a refusal the caller cannot act on would be worse than an answer
//! that names what is missing. A **`literal`** query refuses, because its scope *is* the package:
//! with no tree to scan there are no matches to report, and a `no matches` printed by a scan that
//! never ran cannot be told from a conclusion (round 8, A5: `search --literal "slider"` answered
//! `no matches` for a string the tree contains, with exit 0, while `grep` found it).
//! 结论来自 `crate::mcp::tree_delta`，也就是 `xirang.diff` 说出的那条规则，因此同一个面不可能在这里是
//! `added`、在那里是别的。
//!
//! **命名空间学不到的根按问题分成两半。** **名字**查询仍从源码作答，因为文件遍历不需要那个包，并说明树那一
//! 半不可用——调用方无法据以行动的拒绝，会比一个点名缺失之物的回答更糟。**`literal`** 查询则拒绝，因为它的
//! 范围**就是**那个包：没有树可扫时没有命中可报，而由一次从未跑过的扫描印出的 `no matches` 分不出与结论的
//! 差别（第八轮 A5：`search --literal "slider"` 对树里确实存在的字符串回了 `no matches`、退出码 0，而
//! `grep` 找得到）。
//!
//! A **virtual manifest** is a root, and it is answered as the workspace it is: the census
//! names every member with its status, each member's matching faces are grouped under it,
//! and a member Cargo cannot resolve says `tree unavailable (reason)` instead of
//! disappearing behind a longer list of file hits.
//! **虚拟清单**是一个根，而它按它实际的样子——工作区——作答：普查点名每个成员及其状态，每个成员命中
//! 的面分组在它之下，而 Cargo 解析不了的成员说出 `tree unavailable (原因)`，而不是消失在一份更长的
//! 文件命中之后。
//!
//! online: search is about the sources as they are now, and the build publishes no function or call data.

use std::path::Path;

use crate::build_method::FaceView;
use serde_json::Value;

/// Says which tree the rows came from when they had to be derived.
/// 只能推导时，说明它们读的是哪棵树。
const DERIVED_SOURCE: &str =
    "tree  derived from the sources (this root's published records did not answer the query)";

use crate::mcp::protocol::DEFAULT_LIMIT;
use crate::mcp::search_record::{
    FaceAnswer, RECORD_SOURCE, face_answer, record_face_lines, record_source_lines,
};
use crate::mcp::source_index::load_sources_matching;
use crate::mcp::tree_delta::{FaceStatus, TreeDelta};
use crate::mcp::workspace::{self, Scope};

/// Find registration faces, source files, and Rust function declarations by name.
/// 按名字查找注册面、源码文件与 Rust 函数声明。
/// What to say when a string looks like several names but a string cannot state that shape.
/// 当一个字符串看起来像几个名字、而字符串无法声明这个形状时该说什么。
///
/// Two words or more, each a bare name, is what a caller writes when it means a bag — and it is also
/// what a sentence looks like. The layer cannot tell them apart, so the answer points at the shape
/// that can say it rather than guessing.
/// 两个词以上、每个都是裸名，既是调用方想说"一串"时的写法，也是一句话的样子。这一层分不开它们，因此答案
/// 指向那个能声明的形状，而不是猜。
fn several_names(written: &str) -> Option<String> {
    let words = written.split_whitespace().collect::<Vec<_>>();
    if words.len() < 2 || !words.iter().all(|word| is_bare_name(word)) {
        return None;
    }
    let list = words
        .iter()
        .map(|word| format!("\"{word}\""))
        .collect::<Vec<_>>()
        .join(", ");
    Some(format!(
        "hint   a string is one name; for several names state the shape: `names: [{list}]`"
    ))
}

/// How many names one bag answers before the shared outlet names the rest.
/// 一串名字在共享出口点名"剩下的在哪"之前最多答几个。
const BAG: usize = 8;

/// One group per name, each rendered by the single-name path.
/// 每个名字一组，每组都由单名字那条路径渲染。
///
/// Rendering the groups **by calling the single-name path** is the point: the two spellings cannot
/// drift into two answers for the same name, which is what a second renderer here would eventually
/// become.
/// 各组都**调用单名字那条路径**来渲染，这正是要点：两种拼法不会漂移成"同一个名字两个答案"——而在这
/// 里再写一个渲染器迟早会变成那样。
fn name_bag(root: &Path, names: &[&str], limit: usize) -> Result<String, String> {
    let mut output = String::new();
    for name in names.iter().take(BAG) {
        output.push_str(&format!("name {name}\n"));
        output.push_str(&search(
            root,
            &serde_json::json!({"query": (*name).to_owned(), "limit": limit}),
        )?);
        // The single-name answer does not end with a newline, so without this the next group's
        // header lands on its last row: `…prints it.name postable`. A group boundary has to be a
        // boundary in the text, not only in the code.
        // 单名字的答案不以换行结尾，因此不补这一下，下一组的表头就落在它最后一行上：
        // `…prints it.name postable`。组的边界必须在文本里也是边界，而不只是在代码里。
        if !output.ends_with('\n') {
            output.push('\n');
        }
    }
    if names.len() > BAG {
        output.push_str(&format!(
            "{}\n",
            crate::mcp::truncation::withheld(
                names.len() - BAG,
                names.len(),
                BAG,
                "names",
                "ask again with the rest of the names"
            )
        ));
    }
    Ok(output)
}

pub(crate) fn search(root: &Path, arguments: &Value) -> Result<String, String> {
    // The convergence layer asks the call graph about the name as it was written: a function name
    // is case-sensitive, and the face matcher below is not.
    // 收敛层按写下的原名去问调用图：函数名区分大小写，而下面的面匹配不区分。
    // Two questions live behind one tool, and they are answered differently: `query` matches
    // **names** (faces, files, functions) while `literal` matches **text** anywhere in a source
    // file, comments and string literals included. Asking both at once is refused rather than
    // silently preferring one, because the two answers are not the same shape.
    // 一个工具后面住着两个问题，而它们答法不同：`query` 匹配**名字**（面、文件、函数），`literal` 匹配
    // 源码文件里任何位置的**文本**，注释与字符串字面量都算。同时问两个会被拒绝，而不是静默偏向其中
    // 一个，因为两个答案不是同一种形状。
    let limit = arguments
        .get("limit")
        .and_then(Value::as_u64)
        .map_or(DEFAULT_LIMIT, |value| value.clamp(1, 200) as usize);
    if arguments.get("query").is_some() && arguments.get("literal").is_some() {
        // The refusal names the key the caller should have kept, and the shape of what it passed is
        // what decides it: a bare name is `query`'s question, and anything else — call punctuation,
        // a sentence, a message — is text, which is `literal`'s. Naming the key is the point: the
        // sixth round's arm lost this call to the refusal and would have spent the next one working
        // out which key it meant.
        // 拒绝点名调用方本该留下哪个键，而它传入东西的形状就是判据：裸名是 `query` 的问题，别的一切
        // ——调用的标点、一句话、一条消息——都是文本，也就是 `literal` 的问题。点名那个键正是要点：
        // 第六轮的成员为这条拒绝白花了一次调用，而它接下来还得再花一次才弄清自己指的是哪个键。
        let query = arguments.get("query").and_then(Value::as_str).unwrap_or("");
        let keep = if is_bare_name(query) {
            format!("`{query}` is a bare name, so keep `query` and drop `literal`")
        } else {
            "what you passed to `query` is not a bare name, so keep `literal` and drop `query`"
                .to_owned()
        };
        return Err(format!(
            "`query` matches names (faces, files, functions) and `literal` matches text (raw \
             bytes, comments included); pass one of them, not both — {keep}"
        ));
    }
    if let Some(literal) = arguments.get("literal").and_then(Value::as_str) {
        // A hit with no context is a line number the caller then has to `read` around — the measured
        // extra call. `context` is off by default so the cheap answer stays cheap, and the trailer
        // below says the one word that turns it on.
        // 没有上下文的命中只是一个行号，调用方随后还得在它周围 `read` 一次——那正是量出来的多出来的
        // 一次调用。`context` 默认关闭，让便宜的答案保持便宜，而下面的尾注说出打开它的那一个词。
        let context = arguments
            .get("context")
            .and_then(Value::as_u64)
            .unwrap_or(0)
            .min(10) as usize;
        return literal_search(root, literal, limit, context);
    }
    // Both spellings of a list are accepted, the way `affected`'s `files` accepts them: an **array**
    // (what `--json` carries) and a **string** (what the one-shot client produces for `--names a,b`).
    // Splitting a string is safe *here* because the key itself declares the list shape — which is
    // exactly what `query` cannot do, and why the first version of this wrongly read a sentence of
    // bare-looking words as a bag.
    // 清单的两种写法都收，与 `affected` 的 `files` 一样：**数组**（`--json` 携带的）与**字符串**
    // （一次性客户端对 `--names a,b` 产出的）。在这里按字符串切是安全的，因为**键本身声明了清单形状**
    // ——而 `query` 恰恰做不到这一点，这正是这条的第一版把一句"看起来都是裸名"的话错读成一串的原因。
    if let Some(value) = arguments.get("names") {
        let names = match value {
            Value::Array(items) => items
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect::<Vec<_>>(),
            Value::String(text) => text
                .split([',', ' ', '\t', '\n'])
                .map(str::to_owned)
                .collect::<Vec<_>>(),
            _ => Vec::new(),
        };
        let names = names
            .into_iter()
            .map(|name| name.trim().to_owned())
            .filter(|name| !name.is_empty())
            .collect::<Vec<_>>();
        if names.is_empty() {
            return Err(
                "`names` takes several bare names: `--names signed,postable` on the command line, or \
                 \"names\": [\"signed\", \"postable\"] in `--json`"
                    .to_owned(),
            );
        }
        let borrowed = names.iter().map(String::as_str).collect::<Vec<_>>();
        return name_bag(root, &borrowed, limit);
    }
    let written = arguments
        .get("query")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            "xirang.search requires `query` (one name), `names` (several names), or `literal` \
             (text)"
                .to_owned()
        })?
        .trim()
        .to_owned();
    let query = written.to_ascii_lowercase();
    if query.is_empty() {
        return Err("query must not be empty".to_owned());
    }
    let several = several_names(&written);
    // Several names is one question with several subjects: the control arm's `explore` takes a string
    // of symbols and answers with each one's source, and the measured shape of asking one at a time is
    // one call per name (audit T-06). The shape is an **array**, not a whitespace-separated string,
    // and the first version of this got that wrong: it read a string whose every word was a bare name
    // as a bag, so `"the gauge value"` — a sentence — came back as three groups, two of them empty.
    // A sentence and a bag of symbols are **the same string** as far as this layer can tell, so
    // guessing is not available: the caller states the shape. A string that looks like several names
    // gets a pointer to the array instead (see `several_names`).
    // 几个名字是"一个问题、几个主语"：对照臂的 `explore` 接一串符号并逐个给出源码，而一次问一个的实测形状
    // 是每个名字一次调用（审计 T-06）。形状是**数组**而不是空格分隔的字符串，而这条的第一版正是错在这里：
    // 它把"每个词都是裸名"的字符串读成串，于是 `"the gauge value"`——一句话——回成了三组、其中两组是空的。
    // 在这一层看来，一句话与一串符号**是同一个字符串**，因此猜不可用：形状由调用方声明。看起来像几个名字的
    // 字符串会得到一条指向数组的指引（见 `several_names`）。
    if let Some(pointer) = several_names(&written) {
        // A string of names is a shape we cannot tell from a sentence, so it is answered as one name
        // and the pointer says how to state the shape. Answering it as a bag is what the first version
        // did, and it turned a sentence into three groups.
        // 一串名字是这一层无法与句子区分开的形状，因此按一个名字作答，而指引说出该怎么声明形状。第一版把它
        // 当串作答，于是把一句话变成了三组。
        let _ = pointer;
    }
    let mut results = Vec::new();
    // The limit used to cut the scan short with nothing said, so a caller that got
    // `limit` rows could not tell a complete answer from a full one. The rows the limit
    // governs are counted over the **whole** scan now — the text is already loaded, so
    // counting costs no I/O — because a count taken while breaking early is a lower
    // bound, and the sentence below says `of N`.
    // 上限过去把扫描提前切断而一言不发，因此拿到 `limit` 行的调用方分不出这是完整答案还是一个被塞满
    // 的答案。现在受上限约束的那些行在**整次**扫描里计数——文本本来就已经载入，计数不花 I/O——因为
    // 在提前 break 处取到的计数只是一个下界，而下面那句话说的是「N 中的 M」。
    let mut hits = 0usize;
    let mut withheld_hits = 0usize;
    // The tree comes first: the face is the unit every other tool names, and a
    // query that names a face must not be consumed by the files that mention it.
    // 树排在前面：面是其它每个工具命名的单位，而点名了某个面的查询不能被提到它的那些文件吃掉。
    //
    // A registration file the derivation could not parse is a fact about the *tree
    // that was read*, not about what this query matched, so it is reported whether
    // or not any face matched: a query naming a face that lives in a broken file
    // used to come back "no matches" with nothing saying the file was broken
    // (`LGC-LG-11`). The stale-manifest note stays inside the guard, because
    // it is about the verdicts of the rows that follow it.
    // 推导解析不了的注册面文件是**被读到的那棵树**的事实，而不是这次查询匹配到了什么的
    // 事实，因此无论有没有面命中它都要报出来：一个点名了某个面的查询，若那个面住在坏文件
    // 里，过去会回一条"no matches"而完全不提那个文件是坏的（`LGC-LG-11`）。下面那条过期
    // 清单的备注仍留在守卫里，因为它说的是随后的行给出的结论。
    //
    // It is also not subject to `limit`: `limit` bounds the *result rows* — the face,
    // file, and function hits below — while this line is a fact about the tree that was
    // read, so it keeps appearing even when the limit has already cut the face list
    // short. Reading it as one more result row is how a caller would conclude that a
    // complete answer came back when a file was in fact dropped.
    // 它同样不受 `limit` 约束：`limit` 限的是**结果行**——下面那些面、文件与函数命中——而
    // 这一行是"读到的这棵树"的事实，因此即使上限已经截短了面的清单，这一行仍会出现。把它当成
    // 又一条结果行，会让调用方在一个文件其实被丢掉时以为拿到的是一份完整的答案。
    //
    // The workspace half follows the same rule at the top level: a virtual root's census is
    // a fact about the tree that was read, so it is stated in full — every member with its
    // status — and a member whose tree cannot be derived says so instead of contributing
    // nothing to a longer list of file hits.
    // 工作区那一半在顶层遵循同一条规则：虚拟根的普查是关于"被读到的那棵树"的事实，因此完整写出
    // ——每个成员带它的状态——而推导不出树的成员明说，而不是在一份更长的文件命中里什么都不贡献。
    // One freshness verdict per member, taken **once** and shared by both halves below (audit `T1`):
    // it used to be asked twice per member, which at 50,000 files was the whole remaining cost. Passing
    // it down is the fix; a cache is not — a remembered verdict is the shape that made an earlier
    // attempt wrong.
    // 每个成员只取**一次**新鲜度判定，由下面两半共用（审计 `T1`）：过去每个成员问两次，而在 50,000 文件上
    // 那就是剩下的全部成本。把判定往下传就是修法；缓存不是——"记住一个判定"正是让先前一次尝试出错的那种形状。
    let members_with_freshness = crate::mcp::consistency::roots_with_freshness(root);
    match workspace::scope(root) {
        Ok(Scope::Package(namespace)) => {
            let current = members_with_freshness
                .iter()
                .find(|(member, _)| member == root)
                .is_some_and(|(_, current)| *current);
            // An **empty** record answer is the one that can be wrong: "no matches" is a claim
            // about the whole tree, and a record that does not carry every face the sources declare
            // cannot make it. A hit is different — a current record's row is about a face that
            // exists — so the (source-walking) completeness check is paid only here, and only for a
            // package that declares a producer whose rows land elsewhere.
            // **空的**记录答案才可能错："没有命中"是关于整棵树的断言，而不携带源码声明每个面的记录作不出这个
            // 断言。命中不同——新鲜记录的某一行说的就是存在的面——因此（要遍历源码的）完整性检查只在这里付，
            // 而且只对"声明了行落在别处的生产者"的包付。
            match face_answer(
                root,
                &namespace,
                current,
                &query,
                limit,
                &mut hits,
                &mut withheld_hits,
            )? {
                FaceAnswer::Record(lines) => results.extend(lines),
                FaceAnswer::EmptyRecord => results.push(RECORD_SOURCE.to_owned()),
                FaceAnswer::Derived(lines) => {
                    if !lines.is_empty() {
                        results.push(DERIVED_SOURCE.to_owned());
                    }
                    results.extend(lines);
                }
            }
        }
        Ok(Scope::Workspace(members)) => {
            results.push(workspace::roster(root, &members));
            for member in &members {
                // Coverage before belief, exactly as the package half does: a fresh record that does
                // not carry every producer may not answer a broad query with its own subset.
                // 与单包那一半完全相同：先问覆盖再采信——一份不携带每个生产者的新鲜记录，不能拿它的子集
                // 回答一次宽查询。
                let coverage = workspace::producer_coverage(&member.dir, &member.name)?;
                let before_hits = hits;
                let before_withheld = withheld_hits;
                let answer = record_face_lines(
                    &member.dir,
                    members_with_freshness
                        .iter()
                        .find(|(root, _)| root == &member.dir)
                        .is_some_and(|(_, current)| *current),
                    &query,
                    limit,
                    &mut hits,
                    &mut withheld_hits,
                )?;
                if let Some(lines) = &answer {
                    if !lines.is_empty() && coverage.complete {
                        results.push(format!("member {} ({})", member.name, member.status()));
                        results.extend(lines.iter().cloned());
                        continue;
                    }
                    if !lines.is_empty() {
                        // The record's hits are a subset; discard them with their counters.
                        // 记录的那些命中只是子集，连同计数一起作废。
                        hits = before_hits;
                        withheld_hits = before_withheld;
                    } else if coverage.complete {
                        // An empty record answer is final only where it covers every producer.
                        // 空记录答案只有在覆盖每个生产者时才是终局。
                        continue;
                    }
                }
                let derived = match coverage.derived {
                    Some(derived) => Ok(derived),
                    None => member.derived_tree(),
                };
                match derived {
                    Ok((faces, unparsable)) => {
                        let lines = package_tree_lines(
                            &member.dir,
                            &faces,
                            &unparsable,
                            &query,
                            limit,
                            &mut hits,
                            &mut withheld_hits,
                        );
                        if !lines.is_empty() {
                            results.push(format!("member {} ({})", member.name, member.status()));
                            results
                                .push(member.evidence_line(DERIVES_BECAUSE).trim_end().to_owned());
                            results.extend(lines);
                        }
                    }
                    Err(reason) => results.push(format!(
                        "member {} ({})  tree unavailable ({reason})",
                        member.name,
                        member.status()
                    )),
                }
            }
        }
        Ok(Scope::Unresolvable(reason)) => results.push(format!("tree  unavailable ({reason})")),
        Err(error) => results.push(format!("tree  unavailable ({error})")),
    }
    // The source half is unchanged: file paths and function declarations by name.
    // 源码那一半不变：按名字匹配的文件路径与函数声明。
    //
    // Audit `W6-2` step two: a **name** that is not in a file's bytes cannot be a declaration in
    // them, so only the files whose text mentions the query are lexed. Every file is still listed —
    // the path half below sees all of them — which is the property the pre-filter must not touch.
    // 审计 `W6-2` 第②步：不在一个文件的字节里的**名字**，不可能是那个文件里的声明，因此只对文本提到该
    // 查询的文件做词法。每个文件仍然被列出——下面的路径那一半看得到全部——这是预筛绝不能碰的性质。
    let needle = query.to_ascii_lowercase();
    // Audit `T1`: both halves of this question — "is it a **path**" and "is it a **function name**" —
    // are facts the build already published (`file_manifest.tsv`), so a current record answers them
    // without reading a single file. Reading them instead was measured at **96 s** on a 50,000-file
    // workspace for a query that matched nothing; a matching one looked 8× cheaper only because it
    // exits early on `limit`, which is why the older figures for this tool were partial runs. The
    // scan below is unchanged for a record that cannot answer, and the pre-filter rule stands: it
    // decides what is **lexed**, never what is **listed**.
    // 审计 `T1`：这个问题的两半——"它是不是一个**路径**"、"它是不是一个**函数名**"——都是构建已经发布过的
    // 事实（`file_manifest.tsv`），因此新鲜的记录不读任何文件就能回答它们。改为现读在 50,000 文件的工作区上
    // 实测 **96 s**（什么也没命中的查询）；命中的那个看着便宜 8 倍，只因它按 `limit` 提前退出——这也是这个工具
    // 早先那些数是**部分**运行的原因。记录答不了时下面的扫描保持原样，而预筛的规则照旧：它决定**词法什么**，
    // 绝不决定**列出什么**。
    if let Some(lines) = record_source_lines(
        &members_with_freshness,
        &query,
        limit,
        &mut hits,
        &mut withheld_hits,
    )? {
        results.extend(lines);
    } else {
        let files =
            load_sources_matching(root, |_, text| text.to_ascii_lowercase().contains(&needle))?;
        for file in &files {
            if file.relative.to_ascii_lowercase().contains(&query) {
                if hits < limit {
                    results.push(format!("file  {}", file.relative));
                    hits += 1;
                } else {
                    withheld_hits += 1;
                }
            }
            for function in &file.functions {
                if !function.name.to_ascii_lowercase().contains(&query) {
                    continue;
                }
                if hits < limit {
                    // The doc's first line rides along: "the doc says A and the code writes not-A" is
                    // the strongest signal this bridge prints, and the evaluation's arm needed two
                    // requests to put the two halves side by side in every round.
                    // 文档首行随行给出："文档说要 A、代码写着 ¬A"是本桥能打印的最强信号，而评测里那一组每轮都
                    // 要两次请求才把两半并排。
                    let doc = match doc_first_line(&file.source, function.line) {
                        Some(doc) => format!("  /// {doc}"),
                        None => String::new(),
                    };
                    results.push(format!(
                        "fn    {} -> {}:{}{doc}",
                        function.name, file.relative, function.line
                    ));
                    hits += 1;
                } else {
                    withheld_hits += 1;
                }
            }
        }
    }
    // Nothing matched: a face, a file, or a function. At a package root that is the whole
    // answer, exactly as before; at a workspace root it follows the census, and saying it is
    // what keeps a census — a fact about the tree that was read rather than a result — from
    // reading as a result list that was cut short.
    // 什么都没有命中：面、文件、函数都没有。在包根上这就是整个答案，与从前完全一样；在工作区根上它
    // 跟在普查之后，而说出来正是为了让普查——关于"被读到的那棵树"的事实而不是结果——不会被读成一份被
    // 截短的结果清单。
    if hits == 0 {
        // A dead end is where a caller most needs the next step: names and text are different
        // questions, and the spellings that fail *as names* (`Type::method`, an enum variant, the
        // message a failing assertion printed) are exactly the ones `literal` answers. The
        // evaluation measured this cost: three rounds in a row the narrowing step was "find where
        // this message is produced", and the answer had to come from a grep outside the bridge.
        // 死路正是调用方最需要下一步的地方：名字与文本是两个问题，而**按名字**失败的拼法
        // （`Type::method`、枚举变体、失败断言印出的那句话）恰恰是 `literal` 能答的那些。评测量出了
        // 这个代价：连着三轮，"范围收窄"那一步都是"去找这句话在哪里产出"，而答案只能来自桥外的 grep。
        // The dead end names the exact call to make. The round measured "pass `literal`" being read
        // as a description rather than as an instruction — and a `const` name is exactly the spelling
        // that fails as a name while succeeding as text.
        // 这条死路点名要发的**那一次**调用。那一轮量到 "pass `literal`" 被读成描述而非指令——而 `const`
        // 名恰恰是"按名字失败、按文本成功"的那种拼法。
        results.push(format!(
            "no matches in {} — names only. Names and text are different questions: for text (a \
             message, an enum variant, `Type::method`, a `const` name) run `search {{literal: \
             \"<your phrase>\"}}`",
            root.display()
        ));
        // A string that looks like several names is the one shape this layer cannot tell from a
        // sentence, so the pointer to the array form rides on the answer rather than replacing it.
        // 看起来像几个名字的字符串是这一层唯一分不开句子的形状，因此指向数组形式的指引搭在答案上，而不是替掉它。
        if let Some(pointer) = several {
            results.push(pointer);
        }
    }
    if withheld_hits > 0 {
        results.push(crate::mcp::truncation::withheld(
            withheld_hits,
            hits + withheld_hits,
            limit,
            "results",
            "raise `limit` or narrow the query",
        ));
    }
    // One entry that converges: the tree layer above, then the chain this name leads into, then
    // what to do next and what was **not** looked at. The chain half is the call-graph tool's own
    // answer, composed rather than re-derived — a layer that repeated the matching rule would be
    // the second implementation this repository keeps deleting.
    // 一个收敛的入口：上面那层是树，接着是这个名字引向的链，然后是下一步该做什么、以及**没有**看什么。
    // 链那一半是调用图工具自己的答案，是**组合**而非重新推导——重复匹配规则的一层，正是本仓一直在删的
    // 第二份实现。
    if arguments.get("converge").and_then(Value::as_bool) == Some(true) {
        results.push("chain:".to_owned());
        let chain = crate::mcp::callgraph::callgraph(
            root,
            &serde_json::json!({"function": written, "limit": limit}),
        )?;
        for line in chain.lines() {
            results.push(format!("  {line}"));
        }
        results.push(
            "next: pass `path` to select one definition when several match, and `xirang.affected` \
             with the files you change to see which tests to run"
                .to_owned(),
        );
        results.push(
            "bounds: static only — dynamic dispatch, function pointers, FFI and runtime branches \
             need a recorded trace (`xirang.trace`), and the index covers the files under this \
             root"
                .to_owned(),
        );
    }
    Ok(results.join("\n"))
}

/// Every line of every source file that contains this text, verbatim.
/// 每个源码文件里含有这段文本的那些行，原样给出。
///
/// This is the one question the bridge could not answer before, and the evaluation measured what
/// that cost: three times in three rounds the narrowing step was "the failing assertion's message
/// names a constant, find where that message is produced" — a string in the sources — and every
/// time the agent had to leave the bridge and grep. The search is deliberately literal: raw bytes,
/// case-sensitive, comments and string literals included. A masked view would be the wrong answer
/// for exactly this question, because the thing being looked for usually *is* a string literal.
/// 这是桥此前唯一答不了的问题，而评测量出了它的代价：三轮里三次，"范围收窄"的那一步都是"失败的断言
/// 文案里点名了一个常量，去找这句话是在哪里产出的"——源码里的一处字符串——而每一次代理都只能离开桥
/// 去 grep。检索刻意做成字面的：原始字节、区分大小写、注释与字符串字面量都算。对这种问题，屏蔽过的
/// 视图恰恰是错答案，因为要找的东西通常**就是**一个字符串字面量。
fn literal_search(
    root: &Path,
    literal: &str,
    limit: usize,
    context: usize,
) -> Result<String, String> {
    if literal.is_empty() {
        return Err("`literal` must not be empty".to_owned());
    }
    let mut results = vec![format!(
        "literal {literal:?} (raw bytes, case-sensitive; comments and string literals included)"
    )];
    let mut hits = 0usize;
    let mut withheld = 0usize;
    // The closure reports whether the hit was recorded: a hit past the limit must not drag its
    // context lines into an answer that already said how much it withheld.
    // 闭包回报这次命中是否被记下：超出上限的命中不得把它周围的上下文拖进一份已经声明扣下多少的答案里。
    let mut record = |file: &str, line: usize, text: &str, results: &mut Vec<String>| -> bool {
        if hits < limit {
            results.push(format!("{file}:{line}: {}", clipped(text)));
            hits += 1;
            true
        } else {
            withheld += 1;
            false
        }
    };
    match workspace::scope(root)? {
        Scope::Package(_) => {
            literal_lines(root, literal, context, &mut results, &mut record)?;
        }
        Scope::Workspace(members) => {
            for member in &members {
                let found =
                    literal_lines(&member.dir, literal, context, &mut results, &mut record)?;
                if found > 0 {
                    results.push(format!("member {} ({found} lines)", member.name));
                }
            }
        }
        // The source scan needs to know which tree it is scanning — its scope comes from the
        // package. When that cannot be learned, **nothing was searched**, so this is a refusal and
        // not an empty result: a `no matches` printed by a scan that never ran is a false negative,
        // and a reader cannot tell it from a conclusion. Measured in round 8 (A5): on a tree whose
        // manifest could not be loaded, `search --literal "slider"` answered `no matches` for a
        // string the tree contains, with exit 0, while `grep` found it.
        // 源码扫描必须先知道自己在扫哪棵树——它的范围来自那个包。学不到时，**什么都没扫**，因此这是
        // 一次拒绝而不是一个空结果：由一次从未跑过的扫描印出的 `no matches` 是假阴性，而读者分不出它与
        // 结论。第八轮（A5）量到：清单加载不了的树上，`search --literal "slider"` 对树里确实存在的字
        // 符串回了 `no matches`、退出码 0，而 `grep` 找得到。
        Scope::Unresolvable(reason) => {
            return Err(format!(
                "nothing was searched: the tree could not be read, so this is **not** `no matches` \
                 — {reason}"
            ));
        }
    }
    if hits == 0 {
        // The moment a caller most needs to know **which tree** was searched is the moment nothing
        // matched: "no matches" alone cannot be told from "no matches in the wrong root".
        // 调用方最需要知道**搜的是哪棵树**的时刻，恰恰是什么都没匹配上的时刻：光一句 "no matches"
        // 分不出"这棵树里没有"与"在错的根上搜了"。
        results.push(format!("no matches in {}", root.display()));
        // A literal that reads like a **spelling** and matched nothing is the round's white call:
        // `.post(`, `Store::post`, `entry.postable(` are code, not text, and the text question is
        // the wrong question for them — what answers them is a name, which is what `query` takes.
        // The line names the key, the bare name the spelling writes, and the call to make, because
        // that is what a caller needs in order not to spend a second call discovering it.
        // 一个读起来像**拼法**、却什么都没匹配上的字面量正是那一轮的白跑：`.post(`、
        // `Store::post`、`entry.postable(` 是代码而不是文本，而"文本"这个问题对它们是错的问题——
        // 能回答它们的是名字，也就是 `query` 收的东西。这一行点名那个键、这段拼法写出的裸名、以及该发的
        // 调用，因为调用方需要的正是这些，好让它不必再花一次调用才发现这件事。
        if let Some(name) = spelled_name(literal) {
            results.push(format!(
                "next   that is a spelling, not text: for a name (a face, a file, a function) pass \
                 `query` — this spelling writes `{name}`, so `search {{query: \"{name}\"}}`"
            ));
        }
    } else if context == 0 {
        // Guidance in the answer rather than in a preamble: prose instructions measured 0/4
        // compliance, while the one pointer embedded in a response was followed immediately.
        // 指引放进答案而不是开场白：散文指令实测 0/4 被遵守，而嵌在响应里的那一条当场就被照做。
        results.push("note   pass context: 2 to print the lines around each hit here".to_owned());
    }
    if withheld > 0 {
        results.push(crate::mcp::truncation::withheld(
            withheld,
            hits + withheld,
            limit,
            "lines",
            "raise `limit` or make the literal longer",
        ));
    }
    Ok(results.join("\n"))
}

/// One root's literal hits, in file order then line order.
/// 一个根上的字面命中，按文件顺序、再按行顺序。
fn literal_lines(
    root: &Path,
    literal: &str,
    context: usize,
    results: &mut Vec<String>,
    record: &mut impl FnMut(&str, usize, &str, &mut Vec<String>) -> bool,
) -> Result<usize, String> {
    let mut found = 0usize;
    // Audit `W6-2` step two: `literal` matches **text**, so every file's text is read and no file is
    // lexed — the symbol scan this mode never looks at is the whole saving.
    // 审计 `W6-2` 第②步：`literal` 匹配的是**文本**，因此每个文件的文本照样读，而一个文件都不做词法
    // ——这一模式从不查看的符号扫描正是省下来的全部。
    for file in load_sources_matching(root, |_, _| false)? {
        let lines = file.source.lines().collect::<Vec<_>>();
        for (index, line) in lines.iter().enumerate() {
            if !line.contains(literal) {
                continue;
            }
            found += 1;
            if !record(&file.relative, index + 1, line, results) || context == 0 {
                continue;
            }
            let start = index.saturating_sub(context);
            let end = (index + context + 1).min(lines.len());
            for (other, text) in lines.iter().enumerate().take(end).skip(start) {
                if other != index {
                    results.push(format!("  {}: {}", other + 1, clipped(text)));
                }
            }
        }
    }
    Ok(found)
}

/// The first line of the doc comment attached above a definition, when there is one.
/// 定义正上方所附文档注释的第一行（若有）。
///
/// "Above" means the contiguous `///` block immediately preceding the definition, and "first" means
/// the top of that block: that is the line a reader has to see next to the code to catch the case
/// where the promise and the implementation disagree.
/// "上方"指紧邻定义之前那段连续的 `///` 块，"第一行"指该块的顶端：要抓住"承诺与实现不一致"这种情况，
/// 读者必须把那一行与代码并排看到。
fn doc_first_line(source: &str, line: usize) -> Option<String> {
    let lines = source.lines().collect::<Vec<_>>();
    let mut at = line.saturating_sub(2);
    let mut top: Option<&str> = None;
    while let Some(current) = lines.get(at) {
        if !current.trim_start().starts_with("///") {
            break;
        }
        top = Some(current);
        if at == 0 {
            break;
        }
        at -= 1;
    }
    let text = top?.trim_start().trim_start_matches("///").trim();
    (!text.is_empty()).then(|| text.to_owned())
}

/// One source line, capped, saying so when it was.
/// 一行源码，设上限，被截时说出来。
fn clipped(line: &str) -> String {
    const COLUMNS: usize = 200;
    if line.chars().count() <= COLUMNS {
        return line.trim().to_owned();
    }
    let head = line.chars().take(COLUMNS).collect::<String>();
    format!("{} … (line cut at {COLUMNS} columns)", head.trim_end())
}

/// Why this answer derives instead of reading a member's published records.
/// 这份答案为什么推导，而不是读成员的已发布记录。
///
/// A face is matched on its logical path, `kind`, module and `registry_name`, and
/// the published record carries none of the first two and only the source for the
/// rest — so the tree half of a search reads the sources. The records are still
/// read first: they are what tells this answer whether a member was built at all,
/// and every group says which tree its rows came from.
/// 面是按逻辑路径、`kind`、模块与 `registry_name` 匹配的；已发布记录从 2026-09-29 起携带声明拼出的
/// 这四项，但仍不携带模块，也不携带"解析不了的面"与"外部面"这两张清单——因此搜索的树那一半仍推导，
/// 理由是它要报告的比一张表多。记录仍然先被读取：正是它告诉这份答案一个成员到底有没有被构建过，而每个
/// 分组都会说出它的行来自哪棵树。
const DERIVES_BECAUSE: &str = "the published record now carries a face's declared path, kind, registry_name and parent, but this answer also counts the faces the derivation cannot parse and the external ones it excludes, which only the derivation sees";

/// The tree half one package contributes: the file that could not be parsed, the
/// stale-manifest note, and the face rows the query matched.
/// 一个包贡献的树那一半：解析不了的文件、过期清单备注，以及查询命中的面行。
///
/// The face rows are the rows `limit` bounds, so they are counted here across every
/// package a workspace query reads; the two note lines are facts about the tree that was
/// read and are never counted (`LGC-LG-11`). The stale note is emitted only in front of
/// rows, because it is about the verdicts that follow it.
/// 面行就是 `limit` 约束的那些行，因此它们在这里跨工作区查询读到的每个包计数；那两行备注是关于
/// "被读到的那棵树"的事实，从不计数（`LGC-LG-11`）。过期备注只在行之前发出，因为它说的是随后那些
/// 行的结论。
pub(super) fn package_tree_lines(
    root: &Path,
    faces: &[FaceView],
    unparsable: &str,
    query: &str,
    limit: usize,
    hits: &mut usize,
    withheld_hits: &mut usize,
) -> Vec<String> {
    let built = TreeDelta::read(root);
    let mut rows = Vec::new();
    for face in faces {
        if matches_face(face, query) {
            if *hits < limit {
                rows.push(face_line(face, &built));
                *hits += 1;
            } else {
                *withheld_hits += 1;
            }
        }
    }
    let mut lines = Vec::new();
    if !unparsable.is_empty() {
        lines.push(unparsable.trim_end().to_owned());
    }
    if !rows.is_empty() {
        // A stale manifest still answers, but every verdict below it is
        // about the tree that build saw rather than this one. The state word
        // is the one every other report prints; the clause after it belongs
        // to this note rather than to the word.
        // 过期的清单仍能作答，但它下面每个结论针对的是那次构建看到的树，而不是眼前这棵。状态词
        // 与其余每份报告打印的那一种相同；后面的从句属于这条备注，而不是那个词。
        if built.known && !built.current {
            lines.push(
                "tree  build stale (run `xirang check`); the statuses below \
                 compare against that build"
                    .to_owned(),
            );
        }
        lines.extend(rows);
    }
    lines
}

/// Whether a face is named by the query: its logical path, kind, module, or
/// `registry_name`.
/// 查询是否点名了某个面：它的逻辑路径、kind、模块或 `registry_name`。
///
/// The source path is deliberately not one of the four: a file hit already answers
/// "which file", and letting a face match on its source would make every file line
/// appear twice — once as a face and once as a file.
/// 源码路径有意不在那四项之内：文件命中的行已经回答了"哪个文件"，而让面按源码匹配会让每个文件都出现
/// 两次——一次作为面，一次作为文件。
fn matches_face(face: &FaceView, query: &str) -> bool {
    [&face.path, &face.kind, &face.module, &face.registry_name]
        .iter()
        .any(|field| field.to_ascii_lowercase().contains(query))
}

/// One face hit, with the build's verdict attached.
/// 一条面命中，附上构建的结论。
fn face_line(face: &FaceView, built: &TreeDelta) -> String {
    let status = if !built.known {
        "build unknown (run `xirang check`)".to_owned()
    } else {
        let verdict = built.status(face);
        if let FaceStatus::Reidentified(previous) = verdict {
            format!("{} ({previous} -> {})", verdict.label(), face.id)
        } else {
            verdict.label().to_owned()
        }
    };
    format!(
        "face  {:<40} kind={:<14} module={:<28} source={}  [{status}]",
        face.path, face.kind, face.module, face.source
    )
}

/// Whether the text is a **bare name** — the one shape `query` answers.
/// 该文本是否是一个**裸名**——`query` 唯一能回答的那种形状。
///
/// A name is an identifier and nothing else: letters, digits and `_`, starting with a letter or
/// `_`. `Button`, `gauge_value` and `postable` are names; `.post(`, `does not target framework` and
/// `a::b` are not, and the refusal above reads this to say which key to keep.
/// 名字就是标识符本身：字母、数字与 `_`，以字母或 `_` 开头。`Button`、`gauge_value` 与 `postable`
/// 是名字；`.post(`、`does not target framework` 与 `a::b` 不是，上面的拒绝读它来说出该留下哪个键。
fn is_bare_name(text: &str) -> bool {
    let text = text.trim();
    !text.is_empty()
        && text
            .chars()
            .next()
            .is_some_and(|first| first.is_alphabetic() || first == '_')
        && text
            .chars()
            .all(|letter| letter.is_alphanumeric() || letter == '_')
}

/// The bare name a call-shaped literal **writes**, or `None` when the text is not a spelling.
/// 一段形似调用的字面量所**写出的**裸名；当该文本不是拼法时为 `None`。
///
/// The question this serves is "did the caller paste code where the tool asks for text": a literal
/// that is one code token — it carries `.`, `::` or a trailing `(`, and no whitespace anywhere — is a
/// spelling, and what answers it is the name inside it. `.post(` writes `post`, `Store::post` writes
/// `post`, `entry.postable(` writes `postable`. Prose is excluded on purpose: a sentence that merely
/// ends in a period is text, and pointing it at `query` would buy a second white call rather than
/// save one.
/// 这一问所服务的问题是"调用方是不是把代码贴到了工具要文本的地方"：一个只由一段代码组成的字面量
/// ——带 `.`、带 `::`、或以 `(` 结尾，且任何位置都没有空白——就是一个拼法，而能回答它的是它里面的那个
/// 名字。`.post(` 写出 `post`，`Store::post` 写出 `post`，`entry.postable(` 写出 `postable`。散文被
/// 有意排除：仅仅以句号结尾的一句话是文本，把它指向 `query` 会买来第二次白跑，而不是省下一次。
fn spelled_name(literal: &str) -> Option<String> {
    let text = literal.trim();
    let spelling = !text.is_empty()
        && !text.chars().any(char::is_whitespace)
        && (text.contains('.') || text.contains("::") || text.ends_with('('));
    if !spelling {
        return None;
    }
    let head = text.trim_end_matches('(');
    let last = head.rsplit("::").next().unwrap_or(head);
    let name = last
        .rsplit(|letter: char| !(letter.is_alphanumeric() || letter == '_'))
        .find(|part| !part.is_empty())?;
    is_bare_name(name).then(|| name.to_owned())
}

#[cfg(test)]
#[path = "search_tests.rs"]
mod search_tests;
