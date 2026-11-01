//! Where a red run points next, and the assertion literal the hint is instantiated from.
//! 一次红的运行接下来指向哪里，以及那条提示据以实例化的断言字面量。

use super::Observation;

/// What to do about a run that did not pass, in the answer rather than in a preamble.
/// 对于一次没有通过的运行该怎么办——写在答案里，而不是开场白里。
///
/// A failing assertion's message is a string literal in the tree, so the cheapest next call is the
/// literal search that finds where it is produced; that pointer is the reason `literal` exists and
/// the one shape of guidance this bridge has measured to work.
/// 失败断言的那句话是树里的一个字符串字面量，因此下一个最便宜的调用就是找出它在哪产出的字面检索；这条指引
/// 正是 `literal` 存在的理由，也是本桥实测唯一生效的那种指引形态。
pub(crate) fn next_step(
    face: &str,
    timed_out: bool,
    code: Option<i32>,
    observed: &Observation,
) -> Option<String> {
    if timed_out || code == Some(0) {
        return None;
    }
    let lines = &observed.lines;
    // Audit `W5-5`: route by **what kind of red this is**, because the cheapest next call differs by
    // kind, and a battery that has to rediscover which kind it is pays for that every run. The
    // default (an assertion's words) keeps the literal search below; the other two kinds are the ones
    // this bridge already has a purpose-built tool for.
    // 审计 `W5-5`：按**这是哪一种红**路由，因为最便宜的下一次调用因种类而异，而一套每次都要重新发现
    // "这是哪一种"的电池会为此反复付费。默认（断言自己的话）走下面的字面检索；另外两种正是本桥已经有
    // 专用工具的情形。
    if observed.results == 0 {
        let at = observed
            .first_location
            .clone()
            .unwrap_or_else(|| format!("{face}/<the file the error names>:<line>"));
        return Some(format!(
            "next   **nothing ran**: the log has no `test result:` line, so this red is the build's, \
             not a test's — `why {{at: \"{at}\"}}` gathers the upstream facts at that location, and \
             read the log named above for complete compiler diagnostics without rerunning Cargo"
        ));
    }
    // The signal is the tree's **own** vocabulary for that family, not a guess: the kernel exports
    // the runtime-check nouns, and a red in this family carries the one that names the check. The
    // words are matched case-insensitively because a failure message is prose an author wrote.
    // 信号是这棵树**自己的**族词汇，不是猜的：内核导出了运行期校验的名词，而这个族的红携带点名那条校验
    // 的那一个。匹配不分大小写，因为失败消息是作者写的散文。
    let coordinates = observed
        .failed
        .iter()
        .map(|test| &test.name)
        .chain(observed.lines.iter())
        .any(|line| {
            let lowered = line.to_lowercase();
            // The check's own spelling, taken from the kernel rather than typed here.
            // 这条校验自己的拼写，取自内核而不是在这里手打。
            lowered.contains(xirang_kernel::RuntimeCheckSpec::CoordinatesInViewport.name())
                || lowered.contains("coordinate")
                || lowered.contains("viewport")
        });
    if coordinates {
        let parent = face
            .trim_end_matches('/')
            .rsplit_once('/')
            .map_or("root", |(above, _)| above);
        return Some(format!(
            "next   this red is in the **coordinates family**: `consistency {{parent: \"{parent}\"}}` \
             compares the siblings that must agree on those values and names the one that drifted, \
             and it hands back the `fix` request that pulls it back — one call instead of a hunt \
             through the family by hand"
        ));
    }
    // The hint is **instantiated from this answer's own state**: the literal it tells the caller to
    // search is taken out of the `why` line this reply just printed, so the next call is copyable
    // instead of a template. Measured (W8): `next` lines were followed 28% of the time, and the only
    // one followed reliably named a concrete thing to read — a placeholder leaves the filling-in to
    // the caller, which is the thinking the hint exists to remove.
    // 这条提示**由这次答案自己的状态实例化**：它让调用方去搜的那个字面量，取自这次回复刚打印出来的 `why` 行
    // ——于是下一次调用是**可粘贴的**，而不是一个模板。实测（W8）：`next` 行被采纳 28%，而唯一被稳定采纳的是
    // 那条点名了"具体要读什么"的——占位符把"填空"留给调用方，而填空正是这条提示存在来省掉的思考。
    let search = match literal_to_search(lines) {
        Some(phrase) => format!("`search {{literal: \"{phrase}\"}}`"),
        None => "`search {literal: \"<a short phrase from the assertion>\"}`".to_owned(),
    };
    Some(format!(
        "next   the `why` lines above are the failing assertion's own words: a short, stable \
         phrase from one is a string literal in this tree, so {search} finds the line that produced \
         it. If two red things may be independent, two green runs are not the evidence: fix one and \
         re-run, and say which red survived. And a probe you built yourself that disagrees with the \
         source is a reason to re-read that line (`read`, `search {{literal}}`) before rebuilding — a \
         second look is cheaper than a second build"
    ))
}

/// The literal worth searching for, taken out of the `why` line this reply carries.
/// 值得拿去搜的那个字面量，取自这次回复携带的 `why` 行。
///
/// What has to be dropped, and why each part is there: the harness prefix (`thread '<name>'
/// (<pid>) panicked at <file>:<line>: `) carries a **process id**, which no source contains; the
/// generic `assertion \`left == right\` failed:` clause is shared by many tests and matches the wrong
/// one; and the numbers the framework appends are the values, not the words. What is left is the
/// message the test itself wrote — the phrase that is actually a string literal in this tree.
/// 必须丢掉的、以及每一部分为什么在：harness 前缀（`thread '<名>' (<pid>) panicked at <文件>:<行>: `）
/// 里带着**进程号**，任何源码里都没有它；通用的 `assertion \`left == right\` failed:` 从句被许多测试共用，
/// 会匹到错的那一个；框架追加的数字是取值，不是词。剩下的才是测试自己写下的那句话——也就是这棵树里真正
/// 作为字符串字面量存在的短语。
pub(crate) fn literal_to_search(lines: &[String]) -> Option<String> {
    const WORDS: usize = 6;
    let message = lines
        .iter()
        .find_map(|line| line.strip_prefix("why    "))?
        .split_once(": ")?
        .1;
    let tail = match message.split_once("panicked at ") {
        Some((_, rest)) => rest.split_once(": ").map_or(rest, |(_, tail)| tail),
        None => message,
    };
    let tail = match tail.split_once("failed: ") {
        Some((_, rest)) => rest,
        None => tail,
    };
    let phrase = tail
        .split_whitespace()
        .filter(|word| {
            !word
                .chars()
                .all(|character| character.is_ascii_digit() || "(),.:;`'\"".contains(character))
        })
        .take(WORDS)
        .collect::<Vec<_>>()
        .join(" ");
    (phrase.split_whitespace().count() >= 3).then(|| phrase.replace('"', "'"))
}
