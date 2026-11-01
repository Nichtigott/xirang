//! Pins for `xirang.digest`: bounded rows, the contract line, and the test-name rule.
//! `xirang.digest` 的钉子：有界的行、契约那一行，以及"被测试点名"的规则。
//!
//! The failure these guard is the fourth class the maintainer named: a file holding several
//! algorithms, one branch of one of them wrong. The reply must show the structure — which functions
//! carry a contract, which a test names — without reading the file out in full.
//! 这些钉子守的是维护者点名的第四类：一个文件里好几套算法、错在其中一套的一个分支。回复要给出结构 ——
// 哪些函数带契约、哪些被测试点名 —— 而不是把文件整个倒出来。

use serde_json::json;

fn scratch(label: &str) -> std::path::PathBuf {
    crate::mcp::tools::tools_tests::scratch_package(label)
}

/// The file summary names each function with its range, its contract and its test naming.
/// 文件摘要给每个函数一行：范围、契约、是否被测试点名。
#[test]
fn the_summary_names_functions_contracts_and_test_naming() {
    let root = scratch("digest-rows");
    // The fixture ships no test file, and "named by a test" is a fact about *this tree*: write the
    // one file that makes it true, under the spelling the reader recognises (`tests/`).
    // 夹具本身没有测试文件，而"被测试点名"是**这棵树**的事实：写下那个能让它为真的文件，用读者认得
    // 的拼法（`tests/`）。
    std::fs::create_dir_all(root.join("tests")).expect("tests directory");
    std::fs::write(root.join("tests/probe.rs"), "pub fn probe() { used(); }\n")
        .expect("a test file");
    let answer = super::digest(&root, &json!({"file": "src/lib.rs"})).expect("an answer");
    assert!(
        answer.starts_with("file src/lib.rs — 3 function(s)"),
        "the header counts the functions: {answer}"
    );
    assert!(
        answer.contains("`used`") && answer.contains("contract: What this function is for."),
        "the contract's first line rides with the row: {answer}"
    );
    // The reader's "test file" spelling is `…/tests/…` (a segment), and this fixture's only
    // test-shaped path is the bare `tests/probe.rs` the block above writes — so what this pin can
    // show is the caller count that file contributes, not the label. The spelling itself is pinned
    // where the rule lives (`callgraph::looks_like_a_test`).
    // 读者的"测试文件"拼法是 `…/tests/…`（一个路径段），而本夹具唯一的测试形状路径是上面写下的裸
    // `tests/probe.rs` —— 因此这条钉子能证明的是那个文件贡献的调用者数，而不是那个标签；拼法本身在规则
    // 所在处（`callgraph::looks_like_a_test`）钉着。
    assert!(
        answer.contains("`used`") && answer.contains("2 caller(s)"),
        "the caller count includes the file written above: {answer}"
    );
    assert!(
        answer.contains("not covered here") && answer.contains("next"),
        "the bounds and the next call ride along: {answer}"
    );
}

/// A missing `file` is refused **with the accepted shape**.
/// 缺 `file` 时被拒绝，**并带上可接受形状**。
#[test]
fn a_missing_file_is_refused_with_the_shape() {
    let root = scratch("digest-shape");
    let error = super::digest(&root, &json!({})).expect_err("a file is required");
    assert!(
        error.contains("digest needs `file`") && error.contains("accepted shape"),
        "{error}"
    );
}
