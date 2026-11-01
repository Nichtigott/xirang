//! The MCP online-analysis gate (audit `W3-3`): a bridge file that **computes** an answer instead
//! of reading the build's published record has to say why the build cannot compute that fact.
//! MCP 在线分析门禁（审计 `W3-3`）：一个**计算**答案而不是读构建已发布记录的桥文件，必须说出为什么
//! 构建算不了那件事。
//!
//! **What is mechanical and what is not, stated rather than implied.** The gate cannot judge whether a
//! reason is *true* — no checker can. What it can do is make the question unavoidable at the only
//! place it can be answered, the module that gives the online answer, so that a reviewer has a line to
//! argue with instead of a silence. That is what the checklist asks for: `W3-3` is a **review** rule
//! ("任何 PR 在 MCP 层新增在线分析必须回答…"), and this is its enforceable half.
//! **哪些是机械的、哪些不是，明说而不是暗示。** 门禁判不了某个理由**是否成立**——没有检查器能。它能做的
//! 是让这个问题在唯一能回答它的地方——给出在线答案的那个模块——变得无法回避，于是评审者有一条可以反驳的
//! 句子，而不是一片沉默。这正是清单要的：`W3-3` 是一条**评审**规则（"任何 PR 在 MCP 层新增在线分析必须
//! 回答…"），而这里是它可以机械执行的那一半。
//!
//! **Why the trigger is the derivation entries rather than "analysis".** "Online analysis" is not a
//! property a text scan can see. What it can see is the handful of calls every online answer goes
//! through — the two derivation entries and the source index — so those are the trigger, and a new
//! online answer that reaches the sources some other way is out of scope **by name** rather than by
//! accident. The list lives here so extending it is one edit in one place.
//! **为什么触发条件是那几个推导入口，而不是"分析"这个词。** "在线分析"不是文本扫描看得见的性质。它看得见
//! 的是每个在线答案都会经过的那几个调用——两个推导入口与源码索引——因此以它们为触发条件；而一个从别的路径
//! 摸到源码的新在线答案不在此门禁范围内，这是**点名**的边界而不是碰巧漏掉。这份清单住在这里，因此扩它只需
//! 改一处。
//!
//! **Boundary.** Only the bridge's production half is judged: everything before the first
//! `#[cfg(test)]` in a file, and never a `*_tests.rs` sibling. A test that derives is a test *about*
//! deriving.
//! **边界。** 只判桥的生产那一半：一个文件里第一个 `#[cfg(test)]` 之前的内容，以及从不判 `*_tests.rs`
//! 兄弟文件。推导的测试是**关于**推导的测试。

use std::fs;
use std::path::Path;

use crate::{relative, rust_sources};

/// The call spellings that mean "this file computes rather than reads the record".
/// 意为"这个文件在计算而不是在记录"的那些调用拼写。
pub const DERIVATION_ENTRIES: &[&str] = &[
    "derived_faces(",
    "derived_tree(",
    "load_sources(",
    "face_views_with_external(",
];

/// The module-doc prefix that answers the question. Prose must follow it.
/// 回答这个问题的模块文档前缀。它后面必须跟散文。
pub const MARKER: &str = "//! online:";

/// How much prose the answer must carry.
/// 答案至少要带多少散文。
///
/// A floor, not a judge: it keeps `//! online: no` from passing as an answer, and it says so here
/// rather than pretending to measure whether the reason is good.
/// 这是下限而不是裁判：它拦住 `//! online: no` 这种"答案"，并且在这里明说，而不是假装在衡量理由好不好。
pub const MIN_PROSE: usize = 30;

/// The directory the walk covers.
/// 遍历覆盖的目录。
const BRIDGE: &str = "toolchain/src/mcp";

/// One file that answers online without saying why the build cannot.
/// 一个在线作答、却没说为什么构建算不了的文件。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Finding {
    /// Path relative to the workspace root, with `/` separators.
    /// 以工作区根为基准的路径，使用 `/` 分隔符。
    pub file: String,
    /// Why it was reported.
    /// 它为何被报出。
    pub reason: String,
    /// Up to five `file:line` sites that triggered it, so the author knows where to look.
    /// 触发它的至多五个 `file:line` 位置，好让作者知道该看哪里。
    pub sites: Vec<String>,
}

/// Every bridge file that computes an answer without answering `W3-3`'s question, sorted by path.
/// 每个在计算答案、却没有回答 `W3-3` 那个问题的桥文件，按路径排序。
pub fn findings(root: &Path) -> Vec<Finding> {
    let mut found = Vec::new();
    for path in rust_sources(&root.join(BRIDGE)) {
        if path
            .file_name()
            .is_some_and(|name| name.to_string_lossy().ends_with("_tests.rs"))
        {
            continue;
        }
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };
        let production = match text.find("#[cfg(test)]") {
            Some(at) => &text[..at],
            None => text.as_str(),
        };
        let mut sites = Vec::new();
        for (number, line) in production.lines().enumerate() {
            if DERIVATION_ENTRIES.iter().any(|entry| line.contains(entry)) && sites.len() < 5 {
                sites.push(format!("{}:{}", relative(root, &path), number + 1));
            }
        }
        if sites.is_empty() {
            continue;
        }
        let answered = module_doc(&text).iter().any(|line| {
            line.trim_start_matches("//!")
                .trim_start()
                .starts_with("online:")
                && line
                    .split_once("online:")
                    .is_some_and(|(_, prose)| prose.trim().chars().count() >= MIN_PROSE)
        });
        if answered {
            continue;
        }
        found.push(Finding {
            file: relative(root, &path),
            reason: format!(
                "computes online ({}) and its module doc carries no `{MARKER} <why the build \
                 cannot>` line of at least {MIN_PROSE} characters",
                DERIVATION_ENTRIES
                    .iter()
                    .filter(|entry| production.contains(**entry))
                    .copied()
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            sites,
        });
    }
    found.sort_by(|left, right| left.file.cmp(&right.file));
    found
}

/// The module documentation of a file: the leading `//!` lines, attributes aside.
/// 一个文件的模块文档：开头的 `//!` 行，属性不计。
fn module_doc(text: &str) -> Vec<&str> {
    text.lines()
        .take_while(|line| {
            let trimmed = line.trim_start();
            trimmed.is_empty() || trimmed.starts_with("//!") || trimmed.starts_with("#![")
        })
        .filter(|line| line.trim_start().starts_with("//!"))
        .collect()
}

#[cfg(test)]
#[path = "online_analysis_tests.rs"]
mod online_analysis_tests;
