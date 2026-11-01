//! The reading rules a `callgraph` answer uses to classify a line: is it a test, and where does a
//! caller sit.
//! `callgraph` 答案用来给一行分类的阅读规则：它是不是测试，以及一个调用者坐在哪里。
//!
//! Split from `callgraph.rs` so both files stay inside the repository's size budget; the paths
//! `callgraph::looks_like_a_test` / `callgraph::caller_note` keep working through a re-export,
//! because other surfaces already call them there.
//! 从 `callgraph.rs` 拆出，使两个文件都留在仓库的尺寸预算内；`callgraph::looks_like_a_test` /
//! `callgraph::caller_note` 两条路径经重导出继续可用，因为别的执行面已经在那些路径上调用它们。

/// Whether a labelled source looks like a test file, by this reader's own convenience rule.
/// 一个已标注的源码看起来是不是测试文件，按这个读取方自己的便利规则判定。
///
/// Not the conventions gate's rule and not a judgement about placement: the bridge answers from
/// text and has to name the files a reader would run, so it reads the three spellings a test
/// file has in this repository — a `tests/` directory, a `_tests.rs` sibling, or `#[test]` in
/// the file itself.
/// 不是 conventions 门禁那条规则，也不是对位置的判断：桥从文本作答，必须点名读者会去跑的那些文件，因此它
/// 读本仓里测试文件的三种拼法——`tests/` 目录、同级 `_tests.rs`、或文件里自带 `#[test]`。
pub(crate) fn looks_like_a_test(label: &str, source: &str) -> bool {
    // `#[cfg(test)]` is deliberately not a test: a production file that mounts its own test
    // module carries it, and listing that file would tell a reader to run the code under test.
    // `#[test]` is, because a file declaring tests runs them.
    // 有意不把 `#[cfg(test)]` 当成测试：挂着自家测试模块的生产文件就带着它，而把它列出来等于叫读者去跑
    // 被测代码本身。`#[test]` 算，因为声明了测试的文件会跑那些测试。
    label.starts_with("tests/")
        || label.contains("/tests/")
        || label.ends_with("_tests.rs")
        || source.contains("#[test]")
}

/// Where a caller sits, when saying it saves the reader a call.
/// 调用者坐在哪里 —— 说出来能替读者省一次调用时才说。
///
/// Two decidable things, both read off the two labels: whether the caller is a test file (the same
/// three spellings `looks_like_a_test` reads), and whether it lives outside the definition's own
/// directory. The wording says **directory**, not "member": the package name is not derivable from a
/// path, and a reply that guessed one would be the kind of fact this bridge refuses to invent.
/// 两件可判定的事，都从两个标签读出来：调用者是不是测试文件（与 `looks_like_a_test` 同一套三种拼法），
/// 以及它是否住在这个定义自己的目录之外。措辞说的是**目录**而不是"成员"：包名无法从路径推出来，而
/// 猜一个正是这座桥拒绝编造的那类事实。
pub(crate) fn caller_note(definition_label: &str, caller_label: &str) -> Option<String> {
    let test = looks_like_a_test(caller_label, "");
    let caller_dir = caller_label.rsplit_once('/').map(|(dir, _)| dir);
    let outside =
        caller_dir.is_some() && caller_dir != definition_label.rsplit_once('/').map(|(dir, _)| dir);
    match (test, outside) {
        (false, false) => None,
        (true, false) => Some("a test file".to_owned()),
        (false, true) => Some(format!(
            "outside this file's directory ({})",
            caller_dir.unwrap_or_default()
        )),
        (true, true) => Some(format!(
            "a test file; outside this file's directory ({})",
            caller_dir.unwrap_or_default()
        )),
    }
}
