//! The tree-wide half of a `check`: what the tree declares and how that declaration can drift,
//! in the two forms that are decidable without a model.
//! `check` 的"全树"那一半：这棵树声明了什么、以及这些声明会怎么漂移——只做两种**不需要模型**就能
//! 判定的形态。
//!
//! A symptom narrows the scope, and that is the right shape for a symptom (the caller names the failing
//! assertion, the face and the call sites do the rest). An **open** question — "are there other
//! problems in this tree" — has no such narrowing, and the measured failure was that nothing answered
//! it: one arm hand-swept every public doc to find a re-spelled constant, the other only compared the
//! contracts on the symptom's own path and never saw it.
//! 症状会自己收窄范围，那对症状是对的（调用方给出失败断言，面与调用点做完剩下的事）。**开放式**的问题
//!
//! online: declaration drift is a difference between the record and the sources, so the sources have to be read to state it.
// ——"这棵树里还有别的问题吗"——没有这种收窄，而量到的失败正是没有任何东西回答它：一个臂手工扫遍所有
// 公开文档才找到那处被重拼的常量，另一个只比对了症状路径上的契约、因此从没看见它。
//!
//! Every line here is a **static fact about the tree**, not a verdict about the code, and the section
//! ends by saying what it does not cover — a census that claimed completeness would be the same
//! false green this repository keeps deleting.
//! 这里的每一行都是**关于这棵树的静态事实**，不是对代码的裁定，而且这一节末尾会说明它**不覆盖**什么
// ——一份自称完备的总账，就是这个仓库一直在删的那种假绿。

use std::collections::{HashMap, HashSet};
#[cfg(test)]
use std::path::Path;

const SAMPLE: usize = 5;

/// How many rows one column prints when the whole table was asked for.
/// 要求整表时，一栏最多印多少行。
///
/// Not "all of them": the walk was sized for a tree of a few thousand functions, and a column is a
/// place to aim the next read rather than a dump. It is large enough to hold this checkout's whole
/// reachability column, so on the tree it is used on it is not a cap at all — and when it does cut,
/// the one truncation sentence names the count, the cap and the way to the rest.
/// 不是"全部"：这次遍历是按几千个函数的树定尺寸的，而一栏是用来瞄准下一次阅读的，不是一次倒出。它大到
/// 装得下本检出整条可达性栏，因此在它实际被用的那棵树上它根本不是一道上限——而它真的切下去时，那句唯一的
/// 截断说明会点名数量、上限与拿到其余部分的办法。
const WHOLE_SAMPLE: usize = 40;

/// The most indexed functions the test-reachability walk answers for.
/// 测试可达性遍历最多为多少个已索引函数作答。
///
/// The walk tests each call name it reaches against this tree's functions through the one shared
/// `is_call_to` predicate, so this is not a timer: it is where the census stops claiming to answer
/// at all, because a tree larger than this has never been measured against that column. 10,000 is
/// an order of magnitude above a single-package root and comfortably above this checkout's own tree
/// (a few thousand indexed functions); past it the column prints the skip with its own count
/// instead of walking a tree it was never sized for.
/// 这次遍历把它到达的每个调用名通过那条共享的 `is_call_to` 判据与本树的函数比对，因此这不是计时器：
/// 它是**总账停止自称作答**的那一点，因为比这更大的树从未对着那一栏量过。10,000 比单包根高一个数量级、
/// 也比本检出自己的树（几千个已索引函数）宽裕得多；超过它时那一栏打印带自身计数的跳过说明，而不是去走
/// 一棵从未按它定过尺寸的树。
const REACHABILITY_BUDGET: usize = 10_000;

/// What the test-reachability column does not cover.
/// 测试可达性那一栏**不覆盖**什么。
///
/// One line, emitted with the column, because a static walk that let itself be read as a
/// coverage measurement would be exactly the false green this census exists against. It is an
/// **index** rather than the prose it used to be: every bound it admits is still named here, and
/// the sentences that say *why* each one is invisible live in the tool's own description
/// (`--list check`), which is where a reader goes when one line is not enough. The two phrases
/// that must survive any compression are in it: the column names what it does not cover, and it
/// says out loud that it is `not a coverage measurement`.
/// 一句话，随那一栏输出——一次放任自己被读成"覆盖率"的静态遍历，正是这份总账要对付的那种假绿。它是一条
/// **索引**而不是它过去那种散文：它承认的每一条边界仍然在这里点名，而说清每一条**为什么**不可见的句子住在
/// 工具自己的描述里（`--list check`）——一行不够时读者正是去那里。任何压缩都必须留下的两个短语都在里面：
/// 这一栏点名它不覆盖什么，并且明说它"不是覆盖率量度"。
const REACHABILITY_BOUNDARY: &str = "  not covered by the test-reachability column: dynamic dispatch, function pointers, FFI, macro expansion, and reach only through a trait method or a closure are invisible, so a function reached only that way stays listed; a test-looking file (`tests/`, `_tests.rs`, or `#[test]`) seeds the walk, so a production file with its own `#[test]` is likely not listed; matching is by name, so an unrelated same-named call counts; `main` is never listed. A static walk, not a coverage measurement; `--list check` has the full text.";

/// What the census as a whole does not cover, in the one line every `check` ends with.
/// 整份总账**不覆盖**什么——每次 `check` 结尾那一行。
///
/// An index rather than a second copy of the columns above it: the reader already has the four
/// counts and their own sentences in the same reply, so restating them here bought nothing and
/// cost every `check` the same hundred words. What it must keep is the boundary a count cannot
/// imply — the shapes no column reads — and the phrase that stops a reader taking an inventory
/// for a measurement.
/// 它是上面那几栏的索引，而不是它们的第二份副本：读者在同一份回复里已经有那四个计数与它们自己的句子，
/// 因此在这里复述它们什么都买不到，却让每一次 `check` 都付同样的上百个词。它必须留下的是一个计数推不出
/// 的边界——没有任何一栏去读的那些形状——以及那个阻止读者把一份清单当成一次量度的短语。
const CENSUS_BOUNDARY: &str = "  not covered: this census reads exactly what the columns above name; string constants, structural duplication, runtime behaviour and claims written in prose are outside it, and the static walk is not a coverage measurement. `--list check` has the full text";

/// What the branch-level column does not cover, emitted with the column itself.
/// 分支级那一栏**不覆盖**什么，随那一栏一起输出。
///
/// The column reports arms that are unreachable **by construction**, and this sentence is what
/// keeps that from reading as "no run takes this arm". The first bound is the important one: a
/// condition whose value depends on data is not judged at all, which is why an arm that only a run
/// could rule out stays invisible here. The rest are the shapes a text read cannot see, each named
/// with the consequence — a construction this tree does not spell would **falsify** a row rather
/// than merely be missed by it, which is why the row's own wording says what is spelled and not
/// what is reachable.
/// 这一栏报的是**按构造**不可达的臂，而这句话正是阻止它被读成"没有运行走到这个臂"的东西。第一条边界
/// 最要紧：值依赖数据的条件一律不判，因此只有运行期才能排除的臂在这里仍然不可见。其余是文本读取看不见
/// 的形状，每一条都点名后果——本树没拼出的构造会**否证**某一行，而不只是被它漏掉，因此那一行自己的
/// 措辞说的是"拼出了什么"，而不是"是否可达"。
const BRANCH_BOUNDARY: &str = "  not covered by the branch-level column: a condition whose value depends on data — a field, a parameter, a comparison, a `match` over a value — is not judged at all, so an arm no run has taken yet stays invisible here; `false` is the only guard literal decided, so `1 == 2`, `!true`, a `const` bool and `cfg!(…)` are not read; macro expansion, dynamic dispatch, function pointers and FFI are invisible, while a `macro_rules!` body this tree writes **is** text — an `if false` inside one is listed (and when that body sits outside any function, its row names no function, because there is none to name), and an arm that only exists after expansion is invisible; a construction this tree does not spell (a derive that builds a value, `unsafe`, a consumer outside this root) would falsify a row; a `pub` enum is never judged, an arm reached through a wildcard or a binding is not read, and an enum name this file imports from another crate is conservatively skipped, so a same-named foreign enum's arms are a miss here rather than a false row. A static read of the source text, not a coverage measurement; `--list check` has the full text.";

/// One declared numeric constant, as the tree spells it.
/// 一条被声明的数值常量，按这棵树里的写法。
struct Declared {
    name: String,
    value: String,
    relative: String,
    line: usize,
}

/// Whether `text` uses `value` as a whole number, with no word character or `.` next to it.
/// `text` 是否把 `value` 当作一个完整的数字使用——两侧既不是词字符、也不是 `.`。
fn uses_whole_number(text: &str, value: &str) -> bool {
    let bytes = text.as_bytes();
    let mut from = 0;
    while let Some(at) = text[from..].find(value) {
        let start = from + at;
        let end = start + value.len();
        let before_ok = start == 0 || {
            let before = bytes[start - 1] as char;
            !before.is_alphanumeric() && before != '_' && before != '.'
        };
        let after_ok = end >= bytes.len() || {
            let after = bytes[end] as char;
            !after.is_alphanumeric() && after != '_' && after != '.'
        };
        if before_ok && after_ok {
            return true;
        }
        from = end;
    }
    false
}

/// The census, as answer lines: the header, one line per finding, and the coverage sentence.
/// 总账，按答案行给出：表头、每条发现一行、以及覆盖范围那一句。
///
/// `whole` is the caller saying "I want the table, not a sample": it buys the full boundary prose
/// (the index is what the shorter reply carries), every test-unreachable function instead of five,
/// and the per-directory split of that column. Everything else is the same answer — the two forms
/// differ in how much of it is printed, never in what counts as a finding.
/// `whole` 是调用方在说"我要那张表，不要样本"：它买下完整的边界散文（短回复携带的是索引）、
/// 全部"没有测试能到达"的函数而不是五条，以及那一栏的逐目录拆分。其余都是同一个答案——两种形态的差别
/// 只在印出多少，从不在"什么算一条发现"。
/// Consecutive rows that share a name and a directory prefix, in the order they were sorted.
/// 名字与目录前缀都相同的连续行，按排序后的顺序。
fn same_shape_runs(rows: &[(String, usize, String)]) -> Vec<Vec<(String, usize, String)>> {
    let mut runs: Vec<Vec<(String, usize, String)>> = Vec::new();
    for row in rows {
        let joined = runs
            .last()
            .is_some_and(|run| run[0].2 == row.2 && shared_prefix(run, row).is_some());
        if joined {
            runs.last_mut().expect("a run exists").push(row.clone());
        } else {
            runs.push(vec![row.clone()]);
        }
    }
    runs
}

/// The directory prefix a run of same-named rows shares, when it is worth stating once.
/// 一串同名行共享的目录前缀——值得说一次的时候。
fn shared_prefix(
    run: &[(String, usize, String)],
    next: &(String, usize, String),
) -> Option<String> {
    let mut segments = run[0].0.split('/').collect::<Vec<_>>();
    segments.pop();
    for row in run.iter().chain(std::iter::once(next)) {
        let mut candidate = segments.clone();
        let theirs = row.0.split('/').collect::<Vec<_>>();
        while !candidate.is_empty() && !theirs.starts_with(&candidate) {
            candidate.pop();
        }
        if candidate.is_empty() {
            return None;
        }
        segments = candidate;
    }
    Some(format!("{}/", segments.join("/")))
}

/// One line for a run of unreachable functions that differ only in their file.
/// 只有文件不同的一串够不着的函数，压成一行。
fn collapsed_unreachable(run: &[(String, usize, String)]) -> String {
    let prefix = shared_prefix(&run[..run.len() - 1], run.last().expect("non-empty run"))
        .unwrap_or_default();
    let tails = run
        .iter()
        .map(|(file, line, _)| format!("{}:{line}", file.trim_start_matches(&prefix)))
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "  {FN}     no test reaches `{}` in {} file(s) under {prefix}: {tails}",
        run[0].2,
        run.len()
    )
}

/// The layer vocabulary every per-item row speaks, so a reader can tell what a row is about before
/// reading it.
/// 每条**逐项**行都说的一种层级词表，让读者在读完之前就知道这一行说的是什么层。
///
/// The other answers already speak it — `search` and `digest` rows start with `face`/`file`/`fn`,
/// `read`'s symbol line with the definition's line number — while the census's per-item rows started
/// with the fact itself, so the same tree came back in two vocabularies and only one of them said
/// which layer it was about (audit T-10). Summary rows keep their prose: they name a column, not an
/// item in a layer.
/// 别的答案已经在说它——`search`/`digest` 的行以 `face`/`file`/`fn` 开头，`read` 的符号行带定义行号
/// ——而总账的逐项行以事实本身开头，于是同一棵树用两套词汇回来，而只有一套说了它讲的是哪一层（审计
/// T-10）。汇总行保留它们的散文：它们点名的是一栏，不是某一层里的一项。
const DECL: &str = "decl";

/// A function, in whichever column it appears.
/// 一个函数，无论在那一栏里出现。
const FN: &str = "fn";

/// The three boundary indexes, merged into the one line the default reply can afford.
/// 三条边界索引合并成默认回复负担得起的那一行。
///
/// Measured on a small tree: the three index lines were **1,036 of a 2,201-character reply (47%)**,
/// and each restated two facts the others already carried — that this is a static read rather than a
/// coverage measurement, and that `--list check` holds the prose. `check` runs on every question, so
/// that repetition is a multiplier. The three facts stay (what each column does not read, one clause
/// each); what goes is saying the same two things three times.
/// 在一棵小题上量到：三条索引行占 **2,201 字符回复里的 1,036（47%）**，而每一条都重述了另外两条已经
/// 带着的两件事——这是静态读取而不是覆盖率测量，以及 `--list check` 里有散文。`check` 每个问题都要跑，
/// 因此这种重复是乘数。三件事留下（每一栏不读什么，各一个从句）；去掉的是把同样两句话说三遍。
const SHORT_BOUNDARY: &str = "  not covered: the reachability column (dynamic dispatch, function pointers, FFI, macro expansion, trait methods and closures are invisible; matching is by name; `main` is never listed) · the branch column (no data-dependent condition is judged, `false` is the only guard literal decided, a `pub` enum is never judged, and a construction this tree does not spell would falsify a row) · this census (string constants, structural duplication, runtime behaviour and claims written in prose are outside it). A static read of the source text, not a coverage measurement; `census: true` and `--list check` have the full text";

#[cfg(test)]
pub(crate) fn census(root: &Path, whole: bool) -> Result<Vec<String>, String> {
    Ok(census_from(
        &crate::mcp::source_index::load_sources(root)?,
        whole,
    ))
}

/// Analyse an already-read source inventory; callers own its freshness and lifetime.
/// 分析已读取的源码清单；调用方管理其新鲜度与生命周期。
pub(crate) fn census_from(
    sources: &[crate::mcp::source_index::SourceFile],
    whole: bool,
) -> Vec<String> {
    let mut declared: Vec<Declared> = Vec::new();
    for file in sources {
        for (index, line) in file.source.lines().enumerate() {
            let Some(rest) = line.trim().strip_prefix("pub const ") else {
                continue;
            };
            let (Some(name), Some(value)) = (rest.split(':').next(), rest.split('=').nth(1)) else {
                continue;
            };
            let value = value.trim().trim_end_matches(';').trim().replace('_', "");
            if !value.is_empty() && value.chars().all(|digit| digit.is_ascii_digit()) {
                declared.push(Declared {
                    name: name.trim().to_owned(),
                    value,
                    relative: file.relative.clone(),
                    line: index + 1,
                });
            }
        }
    }
    let mut lines = vec![format!(
        "census: {} named numeric constant(s); a static fact about this tree, not a verdict",
        declared.len()
    )];
    let mut found: Vec<String> = Vec::new();
    for constant in &declared {
        for file in sources {
            for (index, text) in file.source.lines().enumerate() {
                let at = format!("{}:{}", file.relative, index + 1);
                if at == format!("{}:{}", constant.relative, constant.line) {
                    continue;
                }
                if uses_whole_number(text, &constant.value) {
                    found.push(format!(
                        "  respelled {} is declared as `{}` ({}:{}) and written again at {at}",
                        constant.value, constant.name, constant.relative, constant.line
                    ));
                }
            }
        }
        let production = sources
            .iter()
            .filter(|file| !crate::mcp::callgraph::looks_like_a_test(&file.relative, &file.source))
            .filter(|file| file.relative != constant.relative)
            .filter(|file| file.source.contains(&constant.name))
            .count();
        if production == 0 {
            found.push(format!(
                "  unreferenced `{}` ({}:{}) is not read anywhere outside tests",
                constant.name, constant.relative, constant.line
            ));
        }
    }
    let total = found.len();
    for line in found.iter().take(SAMPLE) {
        lines.push(line.clone());
    }
    if total > SAMPLE {
        lines.push(crate::mcp::truncation::withheld(
            total - SAMPLE,
            total,
            SAMPLE,
            "census items",
            "ask per directory to see its own items",
        ));
    }
    // The third column: what the entry plan declares, as a static count. Families a and b both spent
    // real effort working out that "declaring one more cut" is what changes what the application
    // ships — and both arms derived it by experiment. The count itself is a fact this tree already
    // contains, so the census reports it and lets the caller draw the conclusion.
    // 第三栏：入口计划声明了什么，按静态计数给出。a/b 两族都花了真实力气才弄清"多声明一条 cut"才是
    // 改变"这个应用发布什么"的那一步——而两臂都是靠实验推出来的。计数本身是这棵树已有的事实，总账把它
    // 报出来，结论留给调用方。
    // The count is over **code**, not over the file's text: [`xirang_kernel::source::mask_non_code`]
    // blanks comments and literals while preserving offsets, because a comment that merely mentions
    // `cut(…)` is not a declaration. Measured before this line existed (2026-10-10 defect queue, report
    // §7h): adding one two-line prose comment to a tree moved this column from 8 `cut(` / 12 `graft(`
    // to 10 / 14, with no declaration changed.
    // 计数数的是**代码**而不是文件的文本：[`xirang_kernel::source::mask_non_code`] 在保留偏移的前提下把
    // 注释与字面量抹成空白，因为一条只是提到 `cut(…)` 的注释不是声明。在本行出现之前实测（2026-10-10 缺陷
    // 队列，报告 §7h）：给一棵树加一条两行散文注释，这一栏就从 8 个 `cut(` / 12 个 `graft(` 变成 10 / 14，
    // 而声明一个都没变。
    let (mut cuts, mut grafts) = (0_usize, 0_usize);
    for file in sources {
        let code = xirang_kernel::source::mask_non_code(&file.source);
        cuts += code.matches("cut(").count();
        grafts += code.matches("graft(").count();
    }
    lines.push(format!(
        "  entry plan: {cuts} `cut(` site(s) and {grafts} `graft(` site(s) across this tree's sources \
         (a static count; which of them this application ships is the plan's own business)"
    ));
    // The fourth column: declarations no test writes down. The round-4 index asked for a "which
    // promises does this tree make, who verifies them, and which ones no case touches" view, and
    // the decidable first cut is the cheap one: a `pub fn` in a production file that appears in no
    // test file at all. It is a text-level fact and it says so — a test that reaches the function
    // through a face, a macro, or a call it never spells does not count here.
    // 第四栏：没有任何测试写下来的声明。第四轮的索引要的是"这棵树许了哪些承诺、谁在验证、哪些没有任何
    // 用例碰过"的视图，而可判定的第一刀是便宜的那一刀：生产文件里的 `pub fn`，在任何测试文件里一次都
    // 没出现。这是文本级事实，而且它自己会说出来——通过面、宏或一次未拼出名字的调用抵达该函数的测试，
    // 在这里不算数。
    let test_text = sources
        .iter()
        .filter(|file| crate::mcp::callgraph::looks_like_a_test(&file.relative, &file.source))
        .map(|file| file.source.as_str())
        .collect::<Vec<_>>();
    let mut unnamed: Vec<String> = Vec::new();
    for file in sources {
        if crate::mcp::callgraph::looks_like_a_test(&file.relative, &file.source) {
            continue;
        }
        for (index, line) in file.source.lines().enumerate() {
            let Some(rest) = line.trim().strip_prefix("pub fn ") else {
                continue;
            };
            let name = rest
                .split(['(', '<', ' '])
                .next()
                .unwrap_or_default()
                .trim();
            if name.is_empty() || name == "main" {
                continue;
            }
            if !test_text.iter().any(|text| text.contains(name)) {
                unnamed.push(format!(
                    "  {DECL}   no test names `{name}` ({}:{})",
                    file.relative,
                    index + 1
                ));
            }
        }
    }
    let unnamed_total = unnamed.len();
    for line in unnamed.iter().take(SAMPLE) {
        lines.push(line.clone());
    }
    if unnamed_total > SAMPLE {
        lines.push(crate::mcp::truncation::withheld(
            unnamed_total - SAMPLE,
            unnamed_total,
            SAMPLE,
            "unverified declarations",
            "ask per directory to see its own items",
        ));
    }
    lines.push(format!(
        "  declarations: {unnamed_total} production `pub fn` name(s) appear in no test file (a \
         text-level count: a test that reaches one without writing its name does not count here)"
    ));
    // The fifth column: production functions **no test can reach**, by walking the call graph the
    // name view and the orphan view already walk. This is the behavioural half of the round-4
    // question and it is deliberately not the orphan question: "nothing calls it" and "no test can
    // reach it" are two different facts about a function, so this column decides what a call names
    // through the one shared `callgraph::is_call_to` and shares nothing else with `orphans`.
    // 第五栏：**没有任何测试能到达**的生产函数，沿名字视图与孤儿视图本来就在走的那张调用图走。这是第四轮
    // 那个问题的行为级那一半，而且它**有意**不是孤儿问题："没人调用它"与"没有测试能到达它"是关于同一个函数
    // 的两件不同事实，因此这一栏经**唯一**那份 `callgraph::is_call_to` 判定一次调用点名了什么，其余什么都不
    // 与 `orphans` 共享。
    lines.extend(reachability_column(sources, whole));
    // The sixth column: the arms unreachable **by construction**, which is a different question
    // from the fifth one. A function can be test-reachable and still hold an arm no execution can
    // enter; and an arm no run has taken yet can be perfectly reachable, which is why nothing
    // data-dependent is reported here and the boundary says so in the same reply.
    // 第六栏：**按构造**不可达的臂，与第五栏是两个不同的问题。一个函数可以被测试到达、同时含着一个
    // 任何执行都进不去的臂；而一个还没有任何运行走到过的臂完全可以是可达的——这正是这里不报任何数据相关
    // 东西的原因，也是边界句在同一份回复里说出这一点的原因。
    lines.extend(branch_column(sources, whole));
    // The closing line is the same kind of index the reachability column's own boundary is:
    // one line naming what this census does not read, with the prose left to the tool's own
    // description (`--list check`). It used to restate the four columns above it on every
    // `check`, which is the repetition this compression removes; the phrase it must never lose
    // is the one that stops a reader taking an inventory for a measurement.
    // 结尾那一行与可达性栏自己的边界是同一种索引：一行点名这份总账**不读**什么，把散文留给工具自己的
    // 描述（`--list check`）。它过去在每次 `check` 上把上面那四栏复述一遍，那正是这次压缩去掉的重复；它
    // 绝不能丢的短语，是那个阻止读者把一份清单当成一次量度的短语。
    lines.push(if whole {
        CENSUS_BOUNDARY.to_owned()
    } else {
        SHORT_BOUNDARY.to_owned()
    });
    lines
}

/// The production functions a static walk from this tree's test files cannot reach, and the bounds
/// that walk carries.
/// 本树的测试文件出发做一次静态遍历**到不了**的那些生产函数，以及这次遍历自带的上限。
///
/// Behavioural, and static: the walk starts at every function defined in a file the bridge's own
/// convenience rule calls a test file, follows the call lists the source index already extracted,
/// and reports the functions left over. An edge is the same fact `orphans` reads — a name appearing
/// in some function's call list — and it is read through the same function, `callgraph::is_call_to`,
/// so the two views cannot disagree about which call names which definition; dynamic dispatch,
/// function pointers, FFI and macro-expanded calls are invisible here exactly as they are there, and
/// the column says so rather than calling itself coverage.
/// 行为级、且静态：遍历从**桥自己的便利规则**判为测试文件的文件里的每个函数出发，沿源码索引已经抽出的
/// 调用清单走，把剩下的函数报出来。一条边与 `orphans` 读的是同一个事实——一个名字出现在某个函数的调用
/// 清单里——而且经过**同一个函数** `callgraph::is_call_to` 来读，因此两个视图不可能对"哪次调用点名了哪个
/// 定义"产生分歧；动态派发、函数指针、FFI 与宏展开出来的调用在这里与在那里一样不可见，而这一栏会把这点
/// 说出来，而不是自称覆盖率。
fn reachability_column(
    sources: &[crate::mcp::source_index::SourceFile],
    whole: bool,
) -> Vec<String> {
    let indexed = sources
        .iter()
        .map(|file| file.functions.len())
        .sum::<usize>();
    // Only the census states this column's own prose; the default reply carries the merged index
    // once, at its end, instead of three lines that each repeat what the others say.
    // 只有总账陈述这一栏自己的散文；默认回复在末尾携带那一条合并索引，而不是三条各说一遍的行。
    let mut lines = Vec::new();
    if indexed > REACHABILITY_BUDGET {
        lines.push(format!(
            "  test-reachable: skipped ({indexed} function(s) over the limit of \
             {REACHABILITY_BUDGET}); this tree is larger than the walk was sized for, so no \
             reachability rows are computed for it"
        ));
        if whole {
            lines.push(REACHABILITY_BOUNDARY.to_owned());
        }
        return lines;
    }
    // One flat list of every indexed function, with the file it came from and whether that file is
    // a test file — the same classification `looks_like_a_test` gives the other columns.
    // 一份扁平的已索引函数清单，带上它来自哪个文件、以及那个文件是不是测试文件——与其它几栏同一个
    // `looks_like_a_test` 判据。
    let flat = sources
        .iter()
        .flat_map(|file| {
            let is_test = crate::mcp::callgraph::looks_like_a_test(&file.relative, &file.source);
            file.functions
                .iter()
                .map(move |function| (file, function, is_test))
        })
        .collect::<Vec<_>>();
    // The seeds are the names of every function in a test file. Worklist rather than recursion:
    // the queue only grows when a call string is new, and there are finitely many of those, so the
    // walk terminates on a cyclic graph and cannot overflow the stack on a deep one.
    // 种子是测试文件里每个函数的名字。用工作表而不是递归：只有当某个调用字符串是新的时队列才增长，而这样
    // 的字符串是有限的，因此遍历在带环的图上会终止、在很深的图上也不会把栈撑爆。
    let mut reached: HashSet<String> = HashSet::new();
    let mut queue: Vec<String> = Vec::new();
    for (_, function, is_test) in &flat {
        if *is_test && reached.insert(function.name.clone()) {
            queue.push(function.name.clone());
        }
    }
    let mut visited = vec![false; flat.len()];
    while let Some(call) = queue.pop() {
        // Which definitions this call names is `is_call_to`'s business and nobody else's: the rule
        // has one implementation, in the module that owns the call graph, and both this column and
        // the orphan view go through it.
        // 这次调用点名了哪些定义是 `is_call_to` 的事、不是别人的：这条规则只有一份实现，住在拥有调用图的
        // 那个模块里，而本栏与孤儿视图都经过它。
        for (at, (_, function, _)) in flat.iter().enumerate() {
            if visited[at] || !crate::mcp::callgraph::call_matches(function, &call) {
                continue;
            }
            visited[at] = true;
            for callee in &flat[at].1.calls {
                if reached.insert(callee.clone()) {
                    queue.push(callee.clone());
                }
            }
        }
    }
    let mut production = 0usize;
    let mut rows: Vec<(String, usize, String, String)> = Vec::new();
    // The per-directory split is the answer to the question the sampled reply made a reader ask once
    // per directory: "which parts of this tree are the unreachable ones in". Two numbers per
    // directory — unreachable of production — are enough to aim the next read, and the rows below
    // say which functions they are.
    // 逐目录拆分回答的正是抽样回复逼读者**逐个目录**去问的那个问题："这棵树里够不着的是哪几块"。
    // 每个目录两个数字——够不着的 / 生产的——就足以瞄准下一次阅读，而下面的行说清是哪些函数。
    let mut per_directory: std::collections::BTreeMap<String, (usize, usize)> =
        std::collections::BTreeMap::new();
    for (at, (file, function, is_test)) in flat.iter().enumerate() {
        // `main` is left out for the same reason `orphans` leaves it out: it is the process entry
        // point, the OS calls it, and listing it would be a row every reader has to undo.
        // `main` 不列，理由与 `orphans` 一样：它是进程入口、由操作系统调用，列出来是一行每个读者都得自己
        // 撤销的东西。
        if *is_test || function.name == "main" {
            continue;
        }
        production += 1;
        let directory = directory_of(&file.relative);
        let counts = per_directory.entry(directory).or_default();
        counts.1 += 1;
        if visited[at] {
            continue;
        }
        counts.0 += 1;
        rows.push((
            file.relative.clone(),
            function.line,
            function.name.clone(),
            format!(
                "  {FN}     no test reaches `{}` ({}:{})",
                function.name, file.relative, function.line
            ),
        ));
    }
    rows.sort_by(|left, right| (&left.0, left.1).cmp(&(&right.0, right.1)));
    let total = rows.len();
    lines.push(format!(
        "  test-reachable: {total} of {production} production function(s) no test can reach \
         ({indexed} function(s) indexed in this tree; a static walk from the test files along the \
         same name-in-call-list rule the orphan view uses)"
    ));
    if whole && total > 0 {
        lines.push(directory_split(&per_directory));
    }
    let cap = if whole { WHOLE_SAMPLE } else { SAMPLE };
    // Facts of the same shape **and the same layer** share a line: nine `no test reaches \`paint\``
    // rows repeated a full path each, while the one thing that differed was the file. The collapse
    // states the shared prefix once and keeps every path resolvable — it is a compression, not a
    // summary, so nothing a reader could act on is dropped. Groups smaller than three fall through
    // to the plain rows: collapsing two rows buys nothing and makes the answer harder to scan.
    // 形状**与层级**都相同的事实共用一行：九行 `no test reaches \`paint\`` 每一行都重复一整条路径，而
    // 真正不同的只有文件名。这次折叠把共享前缀说一次，并让每条路径仍然可还原——它是压缩而不是概括，因此
    // 读者能据以行动的东西一样没少。小于三的组落回普通的行：折叠两行买不到什么，还让答案更难扫读。
    let bare = rows
        .iter()
        .take(cap)
        .map(|(file, line, name, _)| (file.clone(), *line, name.clone()))
        .collect::<Vec<_>>();
    for group in same_shape_runs(&bare) {
        if group.len() >= 3 {
            lines.push(collapsed_unreachable(&group));
        } else {
            for (file, line, name) in group {
                lines.push(format!(
                    "  {FN}     no test reaches `{name}` ({file}:{line})"
                ));
            }
        }
    }
    if total > cap {
        lines.push(crate::mcp::truncation::withheld(
            total - cap,
            total,
            cap,
            "test-unreachable functions",
            if whole {
                "ask per directory to see its own items"
            } else {
                "pass `census: true` for every row, or ask per directory"
            },
        ));
    }
    if whole {
        lines.push(REACHABILITY_BOUNDARY.to_owned());
    }
    lines
}

/// The two-segment prefix a source path belongs to: `src/mcp/src/tools.rs` is `src/mcp`.
/// 一条源码路径所属的两段前缀：`src/mcp/src/tools.rs` 属于 `src/mcp`。
///
/// Two segments is the granularity the split is for. In a merged crate it is the module that used to
/// be its own crate; in a flat package it is the top module directory; and a file at the root (`src/lib.rs`)
/// is its own bucket rather than being attributed to a directory it does not sit in.
/// 两段正是这次拆分要的颗粒度。在合并后的 crate 里它就是过去各自成 crate 的那个模块；在扁平包里它是顶层
/// 模块目录；而根上的文件（`src/lib.rs`）自成一格，不会被算进它并不在其中的目录。
fn directory_of(relative: &str) -> String {
    let mut parts = relative.split('/');
    match (parts.next(), parts.next(), parts.next()) {
        (Some(first), Some(second), Some(_)) => format!("{first}/{second}"),
        (Some(first), Some(_), None) => first.to_owned(),
        _ => relative.to_owned(),
    }
}

/// The per-directory split, one line, directories with nothing unreachable left out.
/// 逐目录拆分，一行；没有够不着的函数的目录不列。
///
/// A directory with rows is the whole point of the line, so a directory with none is not a zero to
/// print: it is the absence of a finding.
/// 有行的目录才是这一行的意义，因此没有行的目录不是一个要印出来的零：它意味着这里没有发现。
fn directory_split(counts: &std::collections::BTreeMap<String, (usize, usize)>) -> String {
    const DIRECTORIES: usize = 12;
    let mut present = counts
        .iter()
        .filter(|(_, (unreachable, _))| *unreachable > 0)
        .map(|(directory, (unreachable, production))| {
            format!("{directory} {unreachable} of {production}")
        })
        .collect::<Vec<_>>();
    let total = present.len();
    present.truncate(DIRECTORIES);
    let mut line = format!("  by directory: {}", present.join(" · "));
    if total > DIRECTORIES {
        line.push_str(&format!(
            "\n{}",
            crate::mcp::truncation::withheld(
                total - DIRECTORIES,
                total,
                DIRECTORIES,
                "directories",
                "ask per directory to see its own items",
            )
        ));
    }
    line
}

/// The contract lines above the function an arm sits in, as one bounded note.
/// 某个臂所在函数上方的契约行，作为一条有界的注。
///
/// The join this adds was measured, not guessed. W8's h4 put **31,397 characters of reasoning** into
/// one block (87% of that question, 20% of the arm) spent comparing each algorithm against its own
/// documentation — and the two facts it needed sat in two different answers: the census named the
/// dead arm, and `digest` (a call later) printed the contract's first line. Two facts, two answers,
/// one join left to the reader. The join is mechanical here because both halves are already computed.
/// 这条注加的是一次**量出来的**合并，不是猜的。W8 的 h4 把 **31,397 字符的推理**塞进一个块（占该题 87%、
/// 占该臂 20%），内容是把每个算法与它自己的文档逐条比对——而它需要的两块事实分居**两次答案**：总账点名了
/// 死臂，契约首行是隔一次调用才由 `digest` 印出来的。两块事实、两次答案，合并留给了读者。而这里的两半本来
/// 都已经算出来了，因此这个合并是机械的。
fn contract_note(file: &crate::mcp::source_index::SourceFile, function: &str) -> String {
    if function.is_empty() {
        return String::new();
    }
    let Some(definition) = file
        .functions
        .iter()
        .find(|candidate| candidate.name == function)
    else {
        return String::new();
    };
    let lines = crate::mcp::source_index::contract_lines(&file.source, definition.line);
    if lines.is_empty() {
        return String::new();
    }
    // Three, not two: at two the h4 fixture cut off exactly the decisive line ("a zero entry never
    // is") and left only its setup. The cap exists to bound a row, not to hide the sentence the row
    // is about.
    // 取三不取二：h4 夹具在 2 行时恰好切掉了决定性的那一句（"a zero entry never is"），只留下它的铺垫。
    // 上限是为了给一行封顶，不是为了藏掉这一行正在讲的那句话。
    const SHOWN: usize = 3;
    // Blank `///` lines are separators inside a doc block, not contract text: rendering one as
    // `35: ` would put an empty quote in the answer and read as a broken row.
    // 空的 `///` 行是文档块里的分隔，不是契约正文：把它渲染成 `35: ` 会让答案里出现一句空引文、读起来像坏行。
    let shown = lines
        .iter()
        .map(|(line, text)| (*line, crate::mcp::source_index::contract_text(text)))
        .filter(|(_, text)| !text.is_empty())
        .collect::<Vec<_>>();
    if shown.is_empty() {
        return String::new();
    }
    format!(
        "; the contract above it says {}",
        crate::mcp::note::numbered(&shown, SHOWN, "`read {path, line}` has the rest")
    )
}

/// The ` in `fn`` segment of a row, empty when the arm sits outside any function.
/// 一行里的 ` in `fn`` 片段；臂不在任何函数里时为空。
///
/// A `macro_rules!` body this tree writes outside a function produces an arm with no enclosing
/// function, and rendering that as a pair of empty backticks reads as a broken row rather than as
/// the fact it is. The kernel still *reports* the empty name — "no enclosing function" is
/// information a caller may want — so the decision not to print it is made here, in the one place
/// that renders rows, and the boundary sentence says the row "names no function".
/// 写在函数之外的 `macro_rules!` 体会产出没有所属函数的臂，而把它渲染成一对空反引号会被读成一行坏掉的
/// 输出，而不是它所是的那个事实。内核**仍然报告**空名字——"没有所属函数"是调用方可能想要的信息——因此
/// "不打印它"这个决定落在这里、唯一渲染行的地方，而边界句会说那一行"不点名函数"。
fn arm_site(function: &str) -> String {
    if function.is_empty() {
        String::new()
    } else {
        format!(" in `{function}`")
    }
}

/// The arms no execution can enter, decided by construction and joined across the whole tree.
/// 任何执行都进不去的臂——按构造判定，并在全树范围内连接。
///
/// Two rules, and both of them are the kernel's: this function only **joins** the facts
/// `source_index` already carries. **A** is a guard whose condition is the literal `false`.
/// **B** is a `match` arm whose variant the tree never spells a construction of, where the enum
/// declaring that variant is itself declared in this tree without `pub` and without an attribute
/// that could build a value. B's two halves are what make it an answer rather than a guess: a
/// non-`pub` variant cannot have its path spelled outside this tree, so its construction sites are
/// exactly the ones this scan can see.
/// 两条规则，而且两条都是内核的：本函数只**连接** `source_index` 已经携带的事实。**A** 是条件为字面量
/// `false` 的守卫。**B** 是本树从未拼出构造的变体的 `match` 臂，且声明该变体的枚举本身在本树声明、不带
/// `pub`、也不带能造出值的属性。B 的那两半合起来才让它成为答案而不是猜测：非 `pub` 的变体在树外拼不出
/// 路径，因此它的构造点恰好就是这次扫描能看见的那些。
fn branch_column(sources: &[crate::mcp::source_index::SourceFile], whole: bool) -> Vec<String> {
    // Which enums may be judged at all. Two declarations of one name resolve to the unjudged side:
    // a miss is the cheap direction, and a tree that declares the same name twice is exactly the
    // tree where the join cannot be sure which one a pattern meant.
    // 哪些枚举可以被判定。同名声明出现两次时取"不判"那一侧：漏报是便宜的方向，而同名声明两次的树恰好
    // 就是连接无法确定模式指的是哪一个的树。
    let mut judged: HashMap<&str, bool> = HashMap::new();
    for file in sources {
        for declaration in &file.branches.enums {
            judged
                .entry(declaration.name.as_str())
                .and_modify(|current| *current &= declaration.judged)
                .or_insert(declaration.judged);
        }
    }
    // Every path the tree spells counts, test files included: a test that constructs a variant
    // makes it constructed, whatever the column thinks about where tests are.
    // 本树拼出的每条路径都算数，测试文件也算：构造了某个变体的测试就让它成了已构造的，无论这一栏怎么看
    // 测试文件在哪。
    let mut constructed: HashSet<(&str, &str)> = HashSet::new();
    for file in sources {
        for path in &file.branches.variant_paths {
            constructed.insert((path.enum_name.as_str(), path.variant.as_str()));
        }
    }
    let mut rows: Vec<(String, u32, String)> = Vec::new();
    let mut guards = 0usize;
    let mut variants = 0usize;
    for file in sources {
        // Test-looking files are left out for the reason the other columns leave them out: this
        // column is about what this tree ships, and a test's own arms are its business.
        // 测试样子的文件与其它几栏一样被略过：这一栏说的是这棵树发布什么，而测试自己的臂是它自己的事。
        if crate::mcp::callgraph::looks_like_a_test(&file.relative, &file.source) {
            continue;
        }
        for guard in &file.branches.false_guards {
            rows.push((
                file.relative.clone(),
                guard.line,
                format!(
                    "  `if false` guards an arm{} at {}:{} that no run can enter{}",
                    arm_site(&guard.function),
                    file.relative,
                    guard.line,
                    contract_note(file, &guard.function)
                ),
            ));
            guards += 1;
        }
        // One arm is one row. An or-pattern names several variants and they all carry the arm's
        // line, so the first unconstructed one decides the row and the rest are the same arm.
        // 一个臂就是一行。或模式点名多个变体，而它们都带该臂的行号，因此第一个未被构造的变体决定这一行，
        // 其余的都是同一个臂。
        let mut reported: HashSet<u32> = HashSet::new();
        for arm in &file.branches.matched_variants {
            if !judged.get(arm.enum_name.as_str()).copied().unwrap_or(false) {
                continue;
            }
            // A two-segment pattern names whatever that name means in *this file*. When the file
            // imports it from outside its own crate, this tree's declaration is not what the arm
            // named, so no row may claim it — a miss here is the cheap direction.
            // 两段模式点名的是那个名字**在这个文件里**的含义。当该文件从自己 crate 之外引入它时，这个臂
            // 点名的就不是本树的那个声明，因此任何一行都不得声称它——这里漏报是便宜的方向。
            if file
                .branches
                .foreign_imports
                .iter()
                .any(|imported| imported == &arm.enum_name)
            {
                continue;
            }
            if constructed.contains(&(arm.enum_name.as_str(), arm.variant.as_str())) {
                continue;
            }
            if !reported.insert(arm.line) {
                continue;
            }
            rows.push((
                file.relative.clone(),
                arm.line,
                format!(
                    "  no construction of `{}::{}` is spelled in this tree, so the arm matching \
                     it{} at {}:{} can never be entered (the enum is private, so a constructor \
                     outside this tree cannot spell the variant either){}",
                    arm.enum_name,
                    arm.variant,
                    arm_site(&arm.function),
                    file.relative,
                    arm.line,
                    contract_note(file, &arm.function)
                ),
            ));
            variants += 1;
        }
    }
    rows.sort_by(|left, right| (&left.0, left.1).cmp(&(&right.0, right.1)));
    let total = rows.len();
    let mut lines = vec![format!(
        "  branch-level: {total} constructively unreachable arm(s) in this tree ({guards} `false` \
         guard(s), {variants} never-constructed variant(s); a static read of the source text, not a \
         coverage measurement)"
    )];
    let cap = if whole { WHOLE_SAMPLE } else { SAMPLE };
    // Facts of the same shape **and the same layer** share a line: nine `no test reaches \`paint\``
    // rows repeated a full path each, while the one thing that differed was the file. The collapse
    // states the shared prefix once and keeps every path resolvable — it is a compression, not a
    // summary, so nothing a reader could act on is dropped. Groups smaller than three fall through
    // to the plain rows: collapsing two rows buys nothing and makes the answer harder to scan.
    // 形状**与层级**都相同的事实共用一行：九行 `no test reaches \`paint\`` 每一行都重复一整条路径，而
    // 真正不同的只有文件名。这次折叠把共享前缀说一次，并让每条路径仍然可还原——它是压缩而不是概括，因此
    // 读者能据以行动的东西一样没少。小于三的组落回普通的行：折叠两行买不到什么，还让答案更难扫读。
    for (_, _, row) in rows.iter().take(cap) {
        lines.push(row.clone());
    }
    if total > cap {
        lines.push(crate::mcp::truncation::withheld(
            total - cap,
            total,
            cap,
            "unreachable arms",
            "ask per directory to see its own items",
        ));
    }
    if whole {
        lines.push(BRANCH_BOUNDARY.to_owned());
    }
    lines
}

#[cfg(test)]
#[path = "claims_tests.rs"]
mod claims_tests;
