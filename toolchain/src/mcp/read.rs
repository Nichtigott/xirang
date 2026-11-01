//! `xirang.read`: one bounded look at a source file, and the file's total.
//! `xirang.read`：对一份源码文件的一次有界读取，以及该文件的总行数。
//!
//! The window was the only shape this tool had, and it was a shape the caller could
//! not see: `{path, line}` answered with a half-width of 40 — 81 lines at most — and
//! said nothing about how much file was left. Reading a 473-line file therefore took
//! six calls and no call could tell the reader it was done — the reply carried no
//! total. The bounded window stays the default, because the maintainer's rule is that
//! a bridge does not feed the whole tree by default, and what a caller that names only
//! a line is asking for is a **peek**: its half-width is [`DEFAULT_CONTEXT`] (8, so 17
//! lines), because the 40 it used to be was measured as the largest single class of
//! the characters this bridge sent back. The two explicit shapes are how a caller asks
//! for more, and the **total line count is on every header**, so even a windowed read
//! says how much of the file is still unread.
//! 窗口是此前唯一的形状，而这是调用方看不见的形状：`{path, line}` 以该行为中心返回半宽 40 行、上限
//! 81 行，且对"还剩多少文件"一言不发。于是读一个 473 行的文件要六次调用，而没有一次调用能告诉读者它读完了
//! ——回复里没有总数。有界窗口仍是默认，因为维护者的原则是本桥默认不投喂整棵树，而只点名一行的调用方要的
//! 是一次**窥视**：它的半宽是 [`DEFAULT_CONTEXT`]（8，也就是 17 行），因为它曾经的 40 被量到是本桥
//! 送回的字符里最大的一类。两种显式形状才是调用方要更多东西的方式，而**每个标头都带总行数**，因此即使是
//! 一次窗口读取，也说得出这个文件还有多少没读。
//!
//! Three shapes, one header. `whole: true` prints the file; `lines: "120-260"` prints
//! that forward range; neither means the window around `line`. The window and the
//! explicit shapes have different caps on purpose: a window is a peek and stays at
//! [`MAX_READ_LINES`], while an explicit whole/range read is the caller saying "I want
//! this much" and is bounded by [`MAX_FULL_READ_LINES`]. Past that bound the reply is
//! cut by `crate::mcp::truncation`, the one outlet, so the withheld count, the cap and
//! the way to the rest are all named rather than silently dropped.
//! 三种形状，一个标头。`whole: true` 打印整个文件；`lines: "120-260"` 打印那段朝前的区间；两者都没有时
//! 是围绕 `line` 的窗口。窗口与显式形状的上限有意不同：窗口是一次窥视，保持在 [`MAX_READ_LINES`]；而显式
//! 的整文件/区间读取是调用方在说"我要这么多"，由 [`MAX_FULL_READ_LINES`] 约束。越过那道界限时，回复由
//! `crate::mcp::truncation` 这唯一出口切分，因此被扣下的数量、上限与拿到其余部分的办法都会被点名，
//! 而不是被静默丢弃。

use std::path::Path;

use serde_json::Value;

use crate::mcp::note;
use crate::mcp::protocol::MAX_READ_LINES;
use crate::mcp::source_index::{contract_lines, contract_text, load_one, load_text, required_path};
use crate::mcp::truncation::withheld;

/// The most lines an explicit whole-file or span read prints before it says it truncated.
/// 一次显式的整文件或区间读取在声明被截断之前最多打印的行数。
///
/// It is separate from [`MAX_READ_LINES`] because the two bounds answer two different
/// requests. A window is the *default* — a caller that named only a line asked for a
/// peek, and 240 lines of peek is already generous — while `whole`/`lines` is a caller
/// stating how much it wants, and the answer has to be able to be a whole file: the
/// largest file this repository ships is under this bound, which is why one explicit
/// call replaces the six windowed calls a 473-line file used to take. Past it the reply
/// is not silently short: the shared truncation sentence names the withheld count, this
/// cap, and the narrower request that reaches the rest.
/// 它与 [`MAX_READ_LINES`] 分开，因为两道上限回答的是两种不同的请求。窗口是**默认**——只点名了一行的
/// 调用方要的是一次窥视，而 240 行的窥视已经很宽——而 `whole`/`lines` 是调用方在声明它要多少，其答案必须
/// 能够是一整个文件：本仓库出厂的最大文件在此界限之下，因此一次显式调用就取代了过去读一个 473 行文件所需
/// 的六次窗口调用。越过它时回复不会静默地变短：共享的截断说明会点名被扣下的数量、这道理上限，以及能拿到
/// 其余部分的那个更窄的请求。
pub(crate) const MAX_FULL_READ_LINES: usize = 1200;

/// The half-width of the window a request that names no span gets.
/// 没有点名区间的请求所得窗口的半宽。
///
/// Eight lines either side, so a bare `{path, line}` answers with 17 lines. It was forty, and that
/// default was the largest single class of the characters this bridge sent back — a peek was being
/// paid for at the price of a chunk of the file. The rule it keeps is the maintainer's: a bridge
/// does not feed the whole tree by default, which is why this number stays small and why the two
/// explicit shapes exist. `context` raises the half-width up to [`MAX_CONTEXT`], `whole`/`lines`
/// leave the window entirely, and [`MAX_READ_LINES`] still bounds what any window prints.
/// 每侧八行，因此一个只给 `{path, line}` 的请求回 17 行。它曾经是四十，而那个默认是本桥送回的字符里最大
/// 的一类——一次窥视被按整块文件的价钱买下。它守的原则是维护者的：本桥默认不投喂整棵树，这既是这个数字保持
/// 小的原因，也是两种显式形状存在的原因。`context` 把半宽抬到最多 [`MAX_CONTEXT`]，`whole`/`lines` 干脆
/// 离开窗口，而 [`MAX_READ_LINES`] 仍然约束任何窗口能打印多少。
const DEFAULT_CONTEXT: usize = 8;

/// The widest window half-width a caller can ask for.
/// 调用方能要求的最大窗口半宽。
const MAX_CONTEXT: usize = 120;

/// How many function rows the `symbols` line carries before the shared outlet names the rest.
/// `symbols` 行在共享出口点名"剩下的在哪"之前最多携带多少个函数。
const SYMBOLS: usize = 12;

/// What a read request asked for, before the file is consulted.
/// 一次读取请求要什么，在查阅文件之前。
enum Span {
    /// The whole file.
    /// 整个文件。
    Whole,
    /// A forward, 1-based line range.
    /// 一段朝前的、以 1 起始的行区间。
    Range(usize, usize),
    /// A window around one line.
    /// 围绕某一行的窗口。
    Window { center: usize, context: usize },
}

/// Read a file the tree carries — source or text: a window by default, whole or a range on request.
/// 读取这棵树携带的一份文件——源码或文本：默认是窗口，按要求给整份或一个显式区间。
///
/// The header is always `path:start-end (N lines)`, where `N` is the **file's** total
/// rather than the printed range: that number is what lets a caller that got a window
/// know how much is left, and it is what a whole-file read is checked against.
/// 标头始终是 `path:start-end (N lines)`，其中 `N` 是**文件**的总行数而不是打印区间的行数：正是这个数字
/// 让只拿到窗口的调用方知道还剩多少，也是整文件读取据以自证的依据。
pub(crate) fn read_source(root: &Path, arguments: &Value) -> Result<String, String> {
    if let Some(answer) = super::navigation::read(root, arguments)? {
        return Ok(answer);
    }
    let relative = required_path(arguments)?;
    // A `.rs` file goes through the source index so its symbols are available to the readers that
    // annotate lines with them; anything else the tree carries is read as text. The ledger is the
    // reason: it is not Rust, and refusing it sent the round's arm to `cat` — outside this tool's
    // log, so that work left the account (audit T-02, measured in W8's h2).
    // `.rs` 走源码索引，好让那些用符号标注行的读者拿得到符号；树里携带的其它东西按文本读。台账就是理由：
    // 它不是 Rust，而拒绝它把那一轮的臂送去了 `cat`——在那个工具的日志之外，于是那段工作离开了账本
    // （审计 T-02，在 W8 的 h2 量到）。
    let (relative, source, functions) = if relative.ends_with(".rs") {
        let file = load_one(root, &relative)?;
        (file.relative, file.source, file.functions)
    } else {
        let (relative, source) = load_text(root, &relative)?;
        (relative, source, Vec::new())
    };
    let lines = source.lines().collect::<Vec<_>>();
    // The file's own line count, and the number the header always reports. An empty
    // file has zero lines and still has a header: "nothing to read here" is an answer.
    // 文件自己的行数，也就是标头始终报告的那个数字。空文件有零行，也仍然有标头："这里没有可读的东西"
    // 本身就是一个答案。
    let total = lines.len();
    let span = requested_span(arguments)?;
    let annotated = !matches!(span, Span::Window { .. });
    let (start, end, cap) = bounds(span, total);
    let mut output = format!("{relative}:{start}-{end} ({total} lines)\n");
    // The symbols the printed span covers, in the same reply as their source. The control arm's
    // `node <file>` answers "what is in this file" with source **and** its symbol count in one call,
    // and the measured cost of our splitting that in two was a second call per file (audit T-04).
    // A **window stays a peek** — the default is deliberately small — so the annotation rides only on
    // the shapes where the caller said how much it wants.
    // 打印区间覆盖到的符号，与其源码在同一次回复里。对照臂的 `node <file>` 用一次调用同时回答"这个文件里
    // 有什么"的**源码**与**符号数**，而我们把它拆成两次的代价是每个文件多一次调用（审计 T-04）。**窗口仍然是
    // 一次窥视**——默认有意做小——因此这条注只搭在"调用方说了它要多少"的形状上。
    if annotated {
        let pairs = functions
            .iter()
            .filter(|function| function.line >= start && function.line <= end)
            .map(|function| {
                let contract = contract_lines(&source, function.line)
                    .iter()
                    .map(|(_, text)| contract_text(text))
                    .find(|text| !text.is_empty())
                    .unwrap_or_default();
                let tail = if contract.is_empty() {
                    String::new()
                } else {
                    format!(" — {contract}")
                };
                (
                    function.line,
                    format!(
                        "`{}` lines {}-{}{tail}",
                        function.name, function.line, function.end_line
                    ),
                )
            })
            .collect::<Vec<_>>();
        if !pairs.is_empty() {
            output.push_str(&format!(
                "symbols {}\n",
                note::numbered(
                    &pairs,
                    SYMBOLS,
                    "`digest {file}` lists every function; `read {path, line}` opens one"
                )
            ));
        }
    }
    let span_lines = if total > 0 && end >= start {
        end - start + 1
    } else {
        0
    };
    let mut shown = 0usize;
    for (index, line) in lines.iter().enumerate() {
        let number = index + 1;
        if number < start {
            continue;
        }
        if number > end || shown >= cap {
            break;
        }
        if output.len() + line.len() + 16 > 24_000 {
            output.push_str(&format!("withheld remaining source at line {number}; use offset_bytes:0 with max_bytes to page giant lines\n"));
            break;
        }
        output.push_str(&format!("{number:>5} | {line}\n"));
        shown += 1;
    }
    if span_lines > shown {
        output.push_str(&format!(
            "{}\n",
            withheld(
                span_lines - shown,
                span_lines,
                shown,
                "lines",
                "narrow the range with `lines`, or read a window around one `line`"
            )
        ));
    }
    Ok(output)
}

/// What a request named, or why its arguments cannot be honoured together.
/// 请求点名了什么，或者它的参数为何无法同时被满足。
///
/// The three shapes are mutually exclusive rather than layered: `whole` plus `lines` is
/// two answers to one question, and `lines` plus `line`/`context` mixes a range with a
/// window. Silently letting one win is how a caller comes to believe it read a range it
/// did not, and the reply is the only place it would ever find out.
/// 三种形状是互斥的而不是叠加的：`whole` 加 `lines` 是对同一个问题的两个答案，而 `lines` 加
/// `line`/`context` 是把区间与窗口混在一起。让其中一个静默胜出，正是调用方以为自己读了某个区间、实际没有的
/// 原因，而回复是它唯一会发现这件事的地方。
fn requested_span(arguments: &Value) -> Result<Span, String> {
    let whole = arguments.get("whole").and_then(Value::as_bool) == Some(true);
    let range = arguments.get("lines").and_then(Value::as_str);
    if whole && range.is_some() {
        return Err(
            "`whole` reads the file and `lines` reads a range; ask for one of them".to_owned(),
        );
    }
    if let Some(range) = range {
        if arguments.get("line").is_some() || arguments.get("context").is_some() {
            return Err(
                "`lines` names an explicit range while `line`/`context` describe a window around \
                 one line; ask for one of them"
                    .to_owned(),
            );
        }
        let (start, end) = parse_range(range)?;
        return Ok(Span::Range(start, end));
    }
    if whole {
        return Ok(Span::Whole);
    }
    // A caller-supplied line number is unbounded, and `center + context` used to
    // overflow: a panic in a debug build, and in release a wrapped range whose start is
    // past its end while the reply still says `isError: false` — a wrong answer an agent
    // would trust. The centre is clamped to the file here, and every step below
    // saturates.
    // 调用方给的行号没有上界，而 `center + context` 过去会溢出：debug 构建里 panic，release 里回绕成
    // 一个起点超过终点的区间、回复却仍写着 `isError: false`——这是 agent 会相信的错误答案。这里先把中心
    // 夹到文件内，下面每一步都用饱和运算。
    let center = arguments
        .get("line")
        .and_then(Value::as_u64)
        .map_or(1, |line| line.max(1) as usize);
    let context = arguments
        .get("context")
        .and_then(Value::as_u64)
        .map_or(DEFAULT_CONTEXT, |value| {
            value.min(MAX_CONTEXT as u64) as usize
        });
    Ok(Span::Window { center, context })
}

/// Parse the `START-END` spelling of `lines`.
/// 解析 `lines` 的 `START-END` 拼法。
///
/// The range must run forward and be 1-based: `0-10` and `260-120` are refusals with the
/// spelling in the message, because a reversed range silently answered an empty body
/// once and that reads exactly like a range past the end of the file.
/// 区间必须朝前且以 1 起始：`0-10` 与 `260-120` 都是拒绝，消息里带着正确拼法，因为反向区间曾经静默地
/// 回答出空正文，而那读起来和"区间在文件末尾之外"一模一样。
fn parse_range(text: &str) -> Result<(usize, usize), String> {
    let text = text.trim();
    // `START:END` is how a range is spelled in most tools, so it is accepted alongside `START-END`;
    // the round's arm lost one call to the `-` spelling.
    // `START:END` 是多数工具里区间的写法，因此与 `START-END` 一并接受；那轮的成员为 `-` 的拼法白花
    // 了一次调用。
    let Some((start, end)) = text.split_once('-').or_else(|| text.split_once(':')) else {
        return Err(format!(
            "`lines` must be spelled `START-END` (for example `120-260`); got `{text}`"
        ));
    };
    let number = |part: &str| {
        part.trim().parse::<usize>().map_err(|_| {
            format!("`lines` must be spelled `START-END` with whole numbers; got `{text}`")
        })
    };
    let (start, end) = (number(start)?, number(end)?);
    if start == 0 || end == 0 {
        return Err(format!(
            "`lines` is 1-based: line numbers start at 1; got `{text}`"
        ));
    }
    if start > end {
        return Err(format!(
            "`lines` must run forward (`START-END` with START <= END); got `{text}`"
        ));
    }
    Ok((start, end))
}

/// The printed range and its cap, clamped into a file of `total` lines.
/// 打印区间与它的上限，夹进一个有 `total` 行的文件里。
fn bounds(span: Span, total: usize) -> (usize, usize, usize) {
    // An empty file has no line to print, and a window still has to be a forward range:
    // `at_least_one` is the file as far as the window arithmetic is concerned, while
    // `total` (zero) is what the header keeps reporting.
    // 空文件没有可打印的行，而窗口仍必须是一段朝前的区间：就窗口运算而言，`at_least_one` 就是这个文件，
    // 而标头继续报告的是 `total`（零）。
    let at_least_one = total.max(1);
    match span {
        Span::Whole => (1, at_least_one, MAX_FULL_READ_LINES),
        Span::Range(start, end) => (
            start.min(at_least_one),
            end.min(at_least_one).max(start.min(at_least_one)),
            MAX_FULL_READ_LINES,
        ),
        Span::Window { center, context } => {
            let center = center.min(at_least_one);
            let start = center.saturating_sub(context).max(1);
            let end = center
                .saturating_add(context)
                .min(at_least_one)
                .min(start.saturating_add(MAX_READ_LINES - 1));
            (start, end, MAX_READ_LINES)
        }
    }
}

#[cfg(test)]
#[path = "read_tests.rs"]
mod read_tests;
