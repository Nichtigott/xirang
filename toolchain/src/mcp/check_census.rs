//! The bounded census sample `check` carries when the census is not requested whole.
//! `check` 在未请求完整总账时携带的有界抽样。

use crate::mcp::truncation::withheld;

/// How many census rows the default `check` keeps before it points at `census: true`.
/// 默认的 `check` 在指向 `census: true` 之前保留多少行总账。
pub(crate) const CENSUS_SAMPLE: usize = 5;

/// The column a head opens, as the name before its colon.
/// 一个栏头开启的那一栏，取它冒号前的名字。
fn column_name(line: &str) -> String {
    line.split_whitespace()
        .next()
        .unwrap_or_default()
        .trim_end_matches(':')
        .to_owned()
}

/// Whether one census line is a column head rather than a row under one.
/// 一条总账行是**栏头**，而不是它下面的数据行。
///
/// The rule is the same one the sample has always used — the first word ends with `:` — factored
/// out so the heads pass and the "one row under each head" pass cannot drift apart.
/// 规则就是抽样一直用的那条——首个词以 `:` 结尾——抽成函数，好让"取栏头"与"每栏取一行"两遍不会漂移。
fn is_column_head(line: &str) -> bool {
    line.split_whitespace()
        .next()
        .is_some_and(|word| word.ends_with(':'))
}

/// The census rows the default reply carries: the head of the table, and the lines that say what
/// the table is not.
/// 默认回复携带的总账行：表头一段，以及说出这份表**不是**什么的那几行。
///
/// The boundary lines are kept unconditionally rather than by position, because they are not rows
/// of the inventory — they are the sentence that keeps the inventory from being read as one, and a
/// sample that dropped them would be exactly the false green the census exists against. What is
/// left out is counted through [`withheld`], the one truncation outlet, so the withheld count, the
/// cap and the way to the rest are all named.
/// 边界行是**无条件**保留的，而不是按位置保留：它们不是这份清单的行，而是让这份清单不被读成一次量度
/// 的那句话，丢掉它们的抽样恰恰是这份总账要对付的那种假绿。省下的部分经 [`withheld`]——唯一那个截断
/// 出口——计数，因此被扣下的数量、上限与拿到其余部分的办法都会被点名。
pub(crate) fn census_sample(census: &[String]) -> Vec<String> {
    // The sample keeps the **head of every column**, its first few rows, and every column's
    // "not covered" line. A flat take of the first five lines hid a whole column once: on the `g4`
    // tree the five kept lines were all the numeric-constant column, so the default answer never
    // showed `branch-level:` or the reachability summary at all — and that level's question *is*
    // the branch column. The column heads are the unindented lines (rows are indented by two
    // spaces), and each one carries its own count, which is what a reader needs in order to know
    // whether the part they did not get could matter.
    // 抽样保留**每一栏的开头**、它的头几行、以及每栏的 `not covered` 行。曾经"平取前五行"把整整一栏藏掉：
    // 在 `g4` 那棵树上留下的五行全属数值常量栏，于是默认答案里**根本看不到** `branch-level:` 与可达性汇总
    // ——而那一关的问题**就是**分支栏。栏头就是不缩进的那些行（数据行缩进两个空格），而每个栏头自带计数，正是
    // 读者判断"没拿到的部分会不会要紧"所需要的东西。
    let mut kept: Vec<String> = Vec::new();
    for line in census {
        if is_column_head(line) && !kept.contains(line) {
            kept.push(line.clone());
        }
    }
    // **One row under every head**, before the flat take. A column whose head survives while its
    // decisive row is withheld is a column the reader cannot dispose of without a second call.
    // **每栏保留栏头下面的一行**，先于平取。栏头活下来而它那一行判据被扣下，等于那一栏要第二跳才能处置。
    let mut served: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    let mut current: Option<String> = None;
    for line in census {
        if is_column_head(line) {
            current = Some(column_name(line));
            continue;
        }
        let Some(head) = current.clone() else {
            continue;
        };
        if served.insert(head) && !kept.contains(line) {
            kept.push(line.clone());
        }
    }
    for line in census.iter().take(CENSUS_SAMPLE) {
        if !kept.contains(line) {
            kept.push(line.clone());
        }
    }
    for line in census {
        if line.starts_with("  not covered") && !kept.contains(line) {
            kept.push(line.clone());
        }
    }
    let omitted = census.len().saturating_sub(kept.len());
    if omitted > 0 {
        kept.push(withheld(
            omitted,
            census.len(),
            CENSUS_SAMPLE,
            "census rows",
            "pass `census: true` for the whole table (every column head is already here with its count)",
        ));
    }
    kept
}
