//! Tests for `xirang.search`'s tree half: a face is found by logical path,
//! `kind`, module or `registry_name`, and each hit says what the build thinks of it.
//! `xirang.search` 树那一半的测试：面可以按逻辑路径、`kind`、模块或 `registry_name` 找到，而每个命中都说
//! 出构建对它的看法。
//!
//! The classification rule itself is pinned through `xirang.diff`'s tests as
//! well; what this file pins is that search reads that one rule, so a face cannot
//! be `ok` in one tool and `added` in the other.
//! 分类规则本身也由 `xirang.diff` 的测试钉住；本文件钉的是 search 读的正是那一条规则，因此一个面
//! 不可能在一个工具里是 `ok`、在另一个里是 `added`。

use std::path::{Path, PathBuf};

use serde_json::json;

use super::search;

/// A throwaway package with two hand-written root faces.
/// 一个含两个手写根面的一次性包。
fn package(label: &str) -> (PathBuf, String) {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let name = format!("mcp-search-{label}");
    let root = std::env::temp_dir().join(format!("{name}-{}-{sequence}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("src")).expect("package directory");
    std::fs::write(
        root.join("Cargo.toml"),
        format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n"),
    )
    .expect("manifest");
    std::fs::write(root.join("src/lib.rs"), "// host entry\n").expect("library target");
    face(
        &root,
        "gauge/gauge.rs",
        "Gauge",
        "pub fn gauge_value() -> u8 { 1 }\n\ncrate::root_object! {\n    kind: Gauge,\n    parent: crate::root_node_id(env!(\"CARGO_PKG_NAME\")),\n}\n",
    );
    face(
        &root,
        "button/button.rs",
        "Button",
        "crate::root_object! {\n    kind: Button,\n    parent: crate::root_node_id(env!(\"CARGO_PKG_NAME\")),\n}\n",
    );
    (root, name)
}

/// Write one face's module.
/// 写入一个面的模块。
fn face(root: &Path, relative: &str, _kind: &str, source: &str) {
    let path = root.join("src").join(relative);
    std::fs::create_dir_all(path.parent().expect("face parent")).expect("face directory");
    std::fs::write(path, source).expect("face source");
}

/// Publish the build evidence for this package.
/// 发布本包的构建证据。
fn publish(root: &Path, name: &str) {
    crate::build_method::check_for(root, &root.join("target/xirang/out"), name)
        .expect("a healthy tree checks clean");
}

/// A face is found by its logical path, kind, module and `registry_name`, and with no
/// build published the verdict is `build unknown` rather than a guess.
/// 面可以按其逻辑路径、kind、模块与 `registry_name` 找到；没有发布构建时结论是 `build unknown` 而不是猜。
#[test]
fn a_face_is_found_and_an_unbuilt_tree_says_the_verdict_is_unknown() {
    let (root, _) = package("unbuilt");
    let reply = search(&root, &json!({"query": "gauge"})).expect("the search answers");
    assert!(reply.contains("face  root/gauge"), "{reply}");
    assert!(reply.contains("kind=Gauge"), "{reply}");
    assert!(reply.contains("module=gauge"), "{reply}");
    assert!(reply.contains("source=gauge/gauge.rs"), "{reply}");
    assert!(
        reply.contains("[build unknown (run `xirang check`)]"),
        "an unbuilt tree must not invent a verdict: {reply}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// Once the build published, a face it saw is `ok` and one it never saw is
/// `added since build`.
/// 构建发布之后，它见过的面是 `ok`，它从未见过的是 `added since build`。
#[test]
fn a_published_face_is_ok_and_a_new_one_is_added_since_build() {
    let (root, name) = package("published");
    publish(&root, &name);
    let known = search(&root, &json!({"query": "root/button"})).expect("the search answers");
    assert!(known.contains("[ok]"), "{known}");

    face(
        &root,
        "dial/dial.rs",
        "Dial",
        "crate::root_object! {\n    kind: Dial,\n    parent: crate::root_node_id(env!(\"CARGO_PKG_NAME\")),\n}\n",
    );
    let added = search(&root, &json!({"query": "dial"})).expect("the search answers");
    assert!(added.contains("face  root/dial"), "{added}");
    assert!(added.contains("[added since build]"), "{added}");
    let _ = std::fs::remove_dir_all(&root);
}

/// A `kind` change under an unmoved file is `re-identified` with the old and new
/// identities — the one change no text search can see.
/// 文件没动而 `kind` 变了会报成 `re-identified` 并带出旧、新身份——这是文本搜索看不见的那种变化。
#[test]
fn a_kind_change_under_an_unmoved_file_is_re_identified() {
    let (root, name) = package("re-identified");
    publish(&root, &name);
    face(
        &root,
        "button/button.rs",
        "Button",
        "crate::root_object! {\n    kind: RenamedButton,\n    parent: crate::root_node_id(env!(\"CARGO_PKG_NAME\")),\n}\n",
    );
    let reply = search(&root, &json!({"query": "root/button"})).expect("the search answers");
    assert!(reply.contains("[re-identified ("), "{reply}");
    assert!(reply.contains(" -> "), "{reply}");
    assert!(
        reply.contains("build stale (run `xirang check`)"),
        "the verdicts are about the build that was published, and the reply says so: {reply}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// The source half is unchanged: a file path and a function declaration by name
/// still answer exactly as before, below the face hits.
/// 源码那一半不变：按名字匹配的文件路径与函数声明仍和以前一样作答，排在面命中之后。
#[test]
fn the_source_half_still_answers_file_and_function_names() {
    let (root, _) = package("source-half");
    let by_file = search(&root, &json!({"query": "gauge"})).expect("the search answers");
    assert!(by_file.contains("face  root/gauge"), "{by_file}");
    assert!(by_file.contains("file  src/gauge/gauge.rs"), "{by_file}");

    let by_function = search(&root, &json!({"query": "gauge_value"})).expect("the search answers");
    assert!(
        by_function.contains("fn    gauge_value -> src/gauge/gauge.rs:"),
        "{by_function}"
    );

    let none =
        search(&root, &json!({"query": "nothing-matches-this"})).expect("the search answers");
    // The intent is unchanged — a query that matches nothing says so — and the answer now also
    // names the question that would have matched it, because names and text are different searches.
    // 意图没变——什么都没匹配上的查询会说出来——而答案现在也点名那个本可以匹配的问题，因为名字与文本
    // 是两种检索。
    assert!(none.starts_with("no matches"), "{none}");
    assert!(none.contains("search {literal:"), "{none}");
    let _ = std::fs::remove_dir_all(&root);
}

/// The result bound is a bound **and says so**: the scan used to stop at `limit` with
/// nothing in the reply saying rows were left out, so a caller could not tell a complete
/// answer from a full one. The count is taken over the whole scan, so the sentence can say
/// `of N` rather than a lower bound.
/// 结果上限是上限**而且自己说出来**：扫描过去在 `limit` 处停下，而回复里没有任何东西说还有行被
/// 落下，因此调用方分不出完整答案与塞满的答案。计数取自整次扫描，因此那句话说的是「N 中的 M」，
/// 而不是一个下界。
#[test]
fn a_result_list_cut_by_the_limit_says_how_many_it_withheld() {
    let (root, _) = package("bounded");
    let reply = search(&root, &json!({"query": "gauge", "limit": 1})).expect("the search answers");
    // `gauge` matches the face, the file and the function: three rows, one shown.
    // `gauge` 匹配面、文件与函数：三行，显示一行。
    assert!(
        reply.contains("… truncated: 2 of 3 results withheld at the limit of 1"),
        "{reply}"
    );
    assert!(reply.contains("face  root/gauge"), "{reply}");
    let _ = std::fs::remove_dir_all(&root);
}

/// A root whose identity namespace cannot be learned still answers the source
/// half, and says the tree half is unavailable instead of pretending it is empty.
/// 身份命名空间无从得知的根仍然回答源码那一半，并说明树那一半不可用，而不是假装它是空的。
#[test]
fn a_root_without_a_package_still_answers_the_source_half() {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let root =
        std::env::temp_dir().join(format!("mcp-search-bare-{}-{sequence}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("src")).expect("source directory");
    std::fs::write(root.join("src/widget.rs"), "pub fn widget_spin() {}\n").expect("source file");

    let reply = search(&root, &json!({"query": "widget"})).expect("the search answers");
    assert!(reply.contains("tree  unavailable ("), "{reply}");
    assert!(reply.contains("file  src/widget.rs"), "{reply}");
    assert!(
        reply.contains("fn    widget_spin -> src/widget.rs:1"),
        "{reply}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// A registration file the derivation cannot parse is counted in the reply instead of vanishing:
/// a read-only tree query used to report a smaller tree as if it were the whole one, which is what
/// `LGC-LG-11` recorded on the producer side and what the consumer half now says out loud.
/// 推导解析不了的注册面文件被计入回复而不是消失：只读的树查询过去把一棵更小的树当成完整的树报出去
/// ——这正是 `LGC-LG-11` 在生产端记录的事，而消费端现在把它说出来。
fn broken_face(root: &std::path::Path, label: &str) {
    let directory = root.join("src").join(label);
    std::fs::create_dir_all(&directory).expect("module directory");
    std::fs::write(
        directory.join(format!("{label}.rs")),
        "crate::root_object! {\n    kind: Broken,\n",
    )
    .expect("truncated face");
}

#[test]
fn the_tree_half_counts_unparsable_registration_files() {
    let (root, _) = package("unparsable");
    broken_face(&root, "broken");
    let reply = search(&root, &json!({"query": "gauge"})).expect("the search answers");
    assert!(reply.contains("unparsable faces 1"), "{reply}");
    let _ = std::fs::remove_dir_all(&root);
}

/// The count of registration files the derivation could not parse is a fact about the tree that
/// was read, so it is reported even when the query matches no face: a query naming a face that
/// lives in a broken file used to come back as nothing special, with the broken file unmentioned
/// (`LGC-LG-11`). The half fixed here is that the line no longer depends on a face having matched.
/// 推导解析不了的注册面文件数量是**被读到的那棵树**的事实，因此即使查询没有命中任何面也要报出来：
/// 一个点名了住在坏文件里的面的查询，过去会回一条看不出异常的结果，而那个坏文件一字未提
/// （`LGC-LG-11`）。这里修的一半是：这一行不再取决于"有没有面命中"。
#[test]
fn an_unparsable_file_is_reported_even_when_no_face_matches() {
    let (root, _) = package("unparsable-no-match");
    broken_face(&root, "broken");
    let reply =
        search(&root, &json!({"query": "nothing-matches-this"})).expect("the search answers");
    assert!(
        reply.contains("unparsable faces 1"),
        "the tree read must say what it could not parse: {reply}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// The convergence entry answers in layers and says what it did not look at: the tree's own
/// verdicts, the chain the name leads into (composed from the call-graph tool rather than
/// re-derived), the next step, and the bounds. One call localizes where three used to.
/// 收敛入口分层作答，并说出它**没有**看什么：树自己的裁决、这个名字引向的链（由调用图工具**组合**而来，
/// 不是重新推导）、下一步，以及边界。过去要三次调用才能定位的事，现在一次就够。
#[test]
fn the_convergence_layer_labels_each_layer_and_its_bounds() {
    let (root, _name) = package("converge");
    let text = search(&root, &json!({"query": "Button", "converge": true})).expect("an answer");
    assert!(text.contains("chain:"), "{text}");
    assert!(text.contains("next: pass `path`"), "{text}");
    assert!(text.contains("bounds: static only"), "{text}");
    assert!(
        text.contains("no static function match") || text.contains("callers ("),
        "the chain layer is the call graph's own answer: {text}"
    );
    // Without the flag the answer is exactly what it was: layers are opt-in, so no existing
    // caller's shape moves.
    // 不带这个开关时答案与从前完全一样：分层是显式请求的，因此既有调用方的形状不变。
    let plain = search(&root, &json!({"query": "Button"})).expect("an answer");
    assert!(!plain.contains("chain:"), "{plain}");
    let _ = std::fs::remove_dir_all(&root);
}

/// A literal search finds raw text — what a failing assertion's message sends you looking for — and
/// it finds it where a masked view would not: inside a string literal and inside a comment. Asking
/// for a name and a literal at once is refused by name, because the two answers are not the same
/// shape.
/// 字面检索找的是原始文本——失败断言的文案正是让人去找的那个东西——而且它能在屏蔽过的视图找不到的
/// 地方找到：字符串字面量里与注释里。同时要名字与字面会被按名拒绝，因为两种答案不是同一种形状。
#[test]
fn a_literal_search_finds_text_in_strings_and_comments() {
    let (root, _name) = package("literal");
    std::fs::write(
        root.join("src/literal_probe.rs"),
        "// the message: plugin does not target framework\n\
         pub const MESSAGE: &str = \"plugin does not target framework `graft-test`\";\n",
    )
    .expect("probe source");
    let text = search(&root, &json!({"literal": "does not target framework"})).expect("an answer");
    assert!(text.contains("raw bytes, case-sensitive"), "{text}");
    assert!(text.contains("src/literal_probe.rs:1:"), "{text}");
    assert!(text.contains("src/literal_probe.rs:2:"), "{text}");
    let error = search(&root, &json!({"query": "Button", "literal": "x"})).expect_err("a refusal");
    assert!(error.contains("pass one of them"), "{error}");
    assert!(
        error.contains("keep `query`"),
        "the refusal names the key the bare name belongs to: {error}"
    );
    let none = search(&root, &json!({"literal": "nowhere-at-all"})).expect("an answer");
    assert!(none.contains("no matches"), "{none}");
    let _ = std::fs::remove_dir_all(&root);
}

/// The round's white call, pinned: a literal that is a **spelling** and matched nothing says which
/// question answers it, and a literal that is plain text does not get that second suggestion.
/// 那一轮的白跑，钉住：一段**拼法**式的字面量什么都没匹配上时，说出该由哪个问题来答；而一段纯文本
/// 的字面量不会得到那第二条建议。
///
/// The line is what turns a white call into a next step: `.post(`, `Store::post` and `entry.postable(`
/// are code pasted where the tool asks for text, and the name inside them is what `query` answers.
/// 这一行正是把一次白跑变成下一步的东西：`.post(`、`Store::post` 与 `entry.postable(` 是被贴到了工具
/// 要文本的地方的代码，而它们里面的那个名字才是 `query` 答的东西。
#[test]
fn a_spelling_that_matches_nothing_names_the_name_search() {
    let (root, _name) = package("spelling");
    std::fs::write(
        root.join("src/spelling_probe.rs"),
        "pub fn post() {}\n\
         pub fn caller() { post(); }\n",
    )
    .expect("probe source");
    // `post(` is text and it hits here — the call site is written that way — so the alternative is
    // not offered: a literal with hits has answered the question it was asked.
    let hit = search(&root, &json!({"literal": "post("})).expect("an answer");
    assert!(hit.contains("src/spelling_probe.rs:2:"), "{hit}");
    assert!(
        !hit.contains("next"),
        "a literal that matched needs no alternative: {hit}"
    );
    // `Store::post` is a spelling of a name no byte of this tree writes, and that empty answer is
    // the one that has to point somewhere: the root, then the key and the bare name.
    let none = search(&root, &json!({"literal": "Store::post"})).expect("an answer");
    assert!(
        none.contains("no matches in "),
        "the empty answer still carries the root: {none}"
    );
    assert!(
        none.contains("next   ") && none.contains("writes `post`") && none.contains("`query`"),
        "the spelling points at the name search, with the name in it: {none}"
    );
    // Prose is text: the same miss gets the root and nothing else.
    let prose = search(&root, &json!({"literal": "nowhere at all"})).expect("an answer");
    assert!(prose.contains("no matches in "), "{prose}");
    assert!(
        !prose.contains("next"),
        "prose is text; `query` is not its question: {prose}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// Neither argument at all: the tool names both, so the caller is not left guessing which one it
/// wanted.
/// 两个参数都不给：工具把两个都点名，调用方不必猜它要的是哪一个。
#[test]
fn a_search_with_neither_a_name_nor_a_literal_names_both() {
    let (root, _name) = package("neither");
    let error = search(&root, &json!({})).expect_err("a refusal");
    assert!(
        error.contains("`query`") && error.contains("`literal`"),
        "{error}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// A name that matches nothing points at the other question instead of stopping at "no matches":
/// the spellings that fail as names are the ones the literal mode answers.
/// 按名字查不到时，答案指向另一个问题，而不是停在"no matches"：按名字失败的拼法正是字面模式能答的。
#[test]
fn a_name_that_matches_nothing_points_at_the_literal_mode() {
    let (root, _name) = package("hint");
    let text = search(&root, &json!({"query": "FrameworkMismatch"})).expect("an answer");
    assert!(text.contains("no matches"), "{text}");
    // The pointer is a pastable call now, not the phrase "pass `literal`": the round measured that
    // phrase being read as a description, and a `const` name is exactly what fails as a *name*.
    // 指引现在是一条可粘贴的调用，而不是 "pass `literal`" 这句话：那一轮量到那句被读成描述，而
    // `const` 名恰恰就是"按名字失败"的那种拼法。
    assert!(text.contains("search {literal:"), "{text}");
    assert!(
        !text.contains("pass `literal`"),
        "the bare pointer is gone: {text}"
    );
    assert!(text.contains("Type::method"), "{text}");
    let _ = std::fs::remove_dir_all(&root);
}

/// A name that is in the build's published record is answered **without deriving the sources** —
/// that is what the four published face facts are for — and a query that only the module can
/// satisfy still falls back to the derivation, because a record does not carry a module.
/// 名字出现在构建已发布的记录里时，答案是**不推导源码**就给出的——那四项已发布的面事实正是为此——而只有
/// 模块能满足的查询仍然回落到推导，因为记录不携带模块。
#[test]
fn a_published_record_answers_a_name_without_deriving_the_sources() {
    use crate::mcp::published::derivations;

    let (root, _name) = package("record-first");
    let out = root.join("target/xirang/out");
    std::fs::create_dir_all(&out).expect("output dir");
    std::fs::write(
        out.join("pruning_manifest.tsv"),
        "# node\tsource\tsymbol\tpath\tkind\tregistry_name\tparent\n\
         bdb4427ce81c9bc51e56bee7667fd2be\tbutton/button.rs\t-\talpha::beta::gamma\tGamma\t\
         gamma\t-\n",
    )
    .expect("published record");

    let before = derivations();
    let answered = search(&root, &json!({"query": "gamma"})).expect("an answer");
    assert!(
        answered.contains("answered from the build's published records"),
        "{answered}"
    );
    assert!(answered.contains("kind=Gamma"), "{answered}");
    assert!(answered.contains("registry=gamma"), "{answered}");
    assert_eq!(
        derivations(),
        before,
        "the record answered it, so no source should have been walked: {answered}"
    );

    // `button` is the fixture face's **module**, which a record does not carry: the same query on
    // the same tree has to derive.
    let before = derivations();
    let derived = search(&root, &json!({"query": "button"})).expect("an answer");
    assert!(
        derived.contains("module=button"),
        "a module-only match comes from the sources: {derived}"
    );
    assert!(
        derivations() > before,
        "the module half can only be answered by deriving: {derived}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// Several names is one call, one group per name — and the shape is **stated**, never guessed.
/// 几个名字是一次调用、每个名字一组——而形状是**声明**出来的，从不靠猜。
///
/// Measured (audit T-06): the control arm's `explore "signed postable normalized_account zero_arm
/// algorithms"` answered with each symbol's source in one call; ours asked one name at a time. The
/// pins are four, and the fourth is the one that caught the design: a **sentence** whose every word
/// looks like a bare name (`"the gauge value"`) must not be read as a bag — a sentence and a bag of
/// symbols are the same string at this layer, so the caller states the shape (`names`) and a string
/// of names gets a pointer to it instead of a guess.
/// 量到的（审计 T-06）：对照臂的 `explore "signed postable normalized_account zero_arm algorithms"`
/// 一次调用给出每个符号的源码；我们一次问一个名字。钉子四条，而第四条正是抓出设计问题的那条：一句
/// **每个词都像裸名**的话（`"the gauge value"`）**不许**被读成一串——在这一层，一句话与一串符号是同一个
/// 字符串，因此形状由调用方声明（`names`），而"一串名字"的字符串得到的是一条指向它的指引，而不是一次猜。
#[test]
fn several_names_is_one_call_with_a_group_per_name() {
    let (root, _) = package("bag");
    // An array (what `--json` carries) and a string (what the one-shot client produces).
    // 数组（`--json` 携带的）与字符串（一次性客户端产出的）。
    for request in [
        json!({"names": ["gauge", "button"]}),
        json!({"names": "gauge,button"}),
    ] {
        let bag = search(&root, &request).expect("several names answer");
        assert!(bag.contains("name gauge\n"), "{request} ⇒ {bag}");
        assert!(
            bag.contains("\nname button\n") || bag.starts_with("name button\n"),
            "a group header sits on its own line, not glued to the row above it: {bag}"
        );
    }
    // One name keeps its old shape: no group header, nothing wrapped.
    // 一个名字保持旧形状：没有组表头，也没被包起来。
    let one = search(&root, &json!({"query": "gauge"})).expect("one name answers");
    assert!(
        !one.starts_with("name ") && !one.contains("\nname "),
        "a single name is not wrapped in a group header: {one}"
    );
    // The sentence is not a bag, and it says how to state one.
    // 那句话不是一串，而它说出了该怎么声明一串。
    let sentence = search(&root, &json!({"query": "the gauge value"})).expect("one query answers");
    assert!(
        !sentence.contains("\nname gauge\n") && !sentence.starts_with("name gauge"),
        "a sentence of bare-looking words is not read as several names: {sentence}"
    );
    assert!(
        sentence
            .contains("hint   a string is one name; for several names state the shape: `names: [")
            && sentence.contains("\"gauge\""),
        "and it points at the shape that can say it: {sentence}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// A `literal` search on a tree that cannot be read **refuses** — it must not report "no matches".
/// 读不了的树上的 `literal` 检索**拒绝**——它不可以报"没有匹配"。
///
/// Measured (round 8, A5): a tree whose manifest could not be loaded answered
/// `no matches in <root>` for a string the tree contains, with exit 0, while `grep` found it. The
/// scan's scope comes from the package, so with no package there was nothing to scan — and a false
/// negative a reader cannot tell from a conclusion is worse than a refusal that says so.
/// 量到的（第八轮 A5）：清单加载不了的树，对树里确实存在的字符串回了 `no matches in <root>`、退出码 0，
/// 而 `grep` 找得到。扫描的范围来自那个包，因此没有包时根本没有东西可扫——而一个读者分不出与结论的假阴性，
/// 比一次把话说清的拒绝更糟。
#[test]
fn a_literal_search_on_a_tree_that_cannot_be_read_refuses_instead_of_reporting_no_matches() {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "mcp-search-broken-{}-{sequence}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("src")).expect("source directory");
    // A manifest that names a dependency which is not there: the tree exists, the package does not.
    // 一份点名了不存在的依赖的清单：树在，包不在。
    std::fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"broken\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\nnope = { path = \"../nowhere\" }\n\n[workspace]\n",
    )
    .expect("manifest");
    std::fs::write(root.join("src/widget.rs"), "pub fn widget_spin() {}\n").expect("source file");

    let refused = search(&root, &json!({"literal": "widget_spin"}))
        .expect_err("a scan that cannot run is a refusal, not an empty result");
    assert!(
        refused.contains("nothing was searched") && refused.contains("not** `no matches`"),
        "the refusal says which of the two it is: {refused}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// The counter-proof: a tree that **can** be read still answers `no matches` when it has none.
/// 反证：读得了的树在没有匹配时**仍然**回 `no matches`。
#[test]
fn a_literal_search_on_a_readable_tree_still_reports_no_matches() {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "mcp-search-empty-{}-{sequence}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("src")).expect("source directory");
    std::fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"readable\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[workspace]\n",
    )
    .expect("manifest");
    std::fs::write(root.join("src/lib.rs"), "pub fn present() {}\n").expect("source file");

    let reply = search(&root, &json!({"literal": "absent_thing"})).expect("the search answers");
    assert!(reply.contains("no matches in"), "{reply}");
    assert!(!reply.contains("nothing was searched"), "{reply}");
    let _ = std::fs::remove_dir_all(&root);
}

/// The pre-filter decides what to **lex**, never what to **list** (audit `W6-2`, step two).
/// 预筛决定**词法什么**，绝不决定**列出什么**（审计 `W6-2` 第②步）。
///
/// The saving is real only if it is safe: `search --query` now lexes a file only when its text
/// mentions the query, and the property that must survive is that **every file is still listed** by
/// path. The fixture is built to make that sharp — `widget/widget.rs` spells `widget` in its **path**
/// and never in its **text**, so the filter skips lexing it while the path half must still find it.
/// A pre-filter that quietly narrowed the file list would pass a naive "the function was found" pin
/// and fail this one.
/// 只有安全时这份节省才算数：`search --query` 现在只在文件**文本**提到查询时才词法它，而必须活下来的
/// 性质是**每个文件仍然按路径列出**。夹具刻意把这一点做尖——`widget/widget.rs` 把 `widget` 拼在**路径**里、
/// 从不拼在**文本**里，于是筛选跳过它的词法，而路径那一半仍必须找到它。一个悄悄缩小了文件清单的预筛，
/// 能骗过一条天真的"函数找到了"的钉子，但骗不过这一条。
#[test]
fn a_file_is_listed_by_path_even_when_its_text_never_mentions_the_query() {
    let (root, _) = package("prefilter");
    face(
        &root,
        "widget/widget.rs",
        "Widget",
        "pub fn held() -> u8 { 7 }\n\ncrate::root_object! {\n    kind: Widget,\n    parent: crate::root_node_id(env!(\"CARGO_PKG_NAME\")),\n}\n",
    );
    let text = std::fs::read_to_string(root.join("src/widget/widget.rs")).expect("the file");
    assert!(
        !text.to_ascii_lowercase().contains("widget") || text.contains("Widget"),
        "the fixture's own text names the kind, so the check below is about the path half"
    );

    let by_path = search(&root, &json!({"query": "widget"})).expect("an answer");
    assert!(
        by_path.contains("src/widget/widget.rs"),
        "the file is listed by its path although the lexer skipped it: {by_path}"
    );

    // And the function half still works for a name that **is** in a file's text.
    // 而"名字确实在文件文本里"时，函数那一半照旧工作。
    let by_name = search(&root, &json!({"query": "held"})).expect("an answer");
    assert!(
        by_name.contains("held") && by_name.contains("src/widget/widget.rs"),
        "a function name in the text is still found: {by_name}"
    );
}

/// A fresh record is not a complete one: a package that declares a face its own record cannot carry
/// must not let "no matches" be the final answer.
/// 新鲜的记录不是完整的记录：一个声明了自有记录无法携带的面的包，不能把"没有命中"当成最终答案。
///
/// This is the shared fact `registry`, `search` and `status` all ask
/// (`workspace::record_covers_producers`), pinned from the outside: no producer → the record answers;
/// a native declaration → the record cannot, because that face is registered by the compiled crate
/// and never appears in this package's static rows.
/// 这就是 `registry`、`search` 与 `status` 都去问的那个共享事实（`workspace::record_covers_producers`），
/// 从外部钉住：没有生产者 → 记录可以作答；有一条本地声明 → 记录不能，因为那个面由被编译的 crate 登记、
/// 从不出现在本包的静态行里。
#[test]
fn a_record_that_cannot_carry_every_declared_face_does_not_answer_no() {
    let (root, name) = package("native-completeness");
    publish(&root, &name);
    assert!(
        crate::mcp::workspace::record_covers_producers(&root, &name)
            .expect("the completeness question answers"),
        "with no producer besides its own sources, the record is the whole tree"
    );

    // One native declaration: the build registers that face from the compiled crate, so the package's
    // static record cannot list it. "No matches" from the record is now a claim it cannot make.
    // 一条本地声明：构建从被编译的 crate 登记那个面，因此本包的静态记录列不出它。记录现在作不出"没有命中"
    // 这个断言。
    std::fs::create_dir_all(root.join("native/area")).expect("native directory");
    std::fs::write(
        root.join("native/area/area.cpp"),
        "// fixture\n#include \"xirang/soil.hpp\"\n#include \"xirang/faces.hpp\"\n\nstruct Area {};\n\nXIRANG_SOIL\nconstexpr auto SOIL = ::xirang::soil<Area>()\n    .under(::xirang::faces::gauge::NODE_ID)\n    .named(\"Area\");\n",
    )
    .expect("native declaration");
    assert!(
        !crate::mcp::workspace::record_covers_producers(&root, &name)
            .expect("the completeness question answers"),
        "a native face the record does not carry makes the record incomplete"
    );
    let _ = std::fs::remove_dir_all(root);
}

/// A **non-empty** record answer is not final when the record cannot carry every producer: a broad
/// query has to count the faces the record does not hold.
/// 记录**非空**时也不一定是终局：当记录不携带每个生产者时，宽查询必须把记录装不下的那些面也数进来。
///
/// The first version believed the record whenever it had at least one hit — "this row exists" was
/// read as "no other producer matches" — and a five-face island answered a broad query with one
/// face. This pins both halves: with no producer the record is still the fast path (no walk), and
/// one native declaration makes the same shape of query include the native face.
/// 第一版在记录至少有一个命中时就采信它——把"这一行存在"读成"没有别的生产者命中"——于是一个五面孤岛
/// 对一次宽查询只答了一个面。本测试钉住两半：没有生产者时记录仍是快路径（不遍历），一条本地声明让同形
/// 查询把本地声明的面也算进来。
#[cfg(feature = "lang-cpp")]
#[test]
fn a_non_empty_record_that_cannot_carry_every_producer_does_not_answer_alone() {
    use crate::mcp::published::derivations;

    let (root, name) = package("partial-hit");
    publish(&root, &name);

    let before = derivations();
    let record_only = search(&root, &json!({"query": "gauge"})).expect("an answer");
    assert!(
        record_only.contains("answered from the build's published records"),
        "{record_only}"
    );
    assert_eq!(
        derivations(),
        before,
        "no producer is declared, so the record is the whole tree and no source is walked: {record_only}"
    );

    // One native declaration makes the record incomplete. `a` matches the record's own `Gauge` and
    // the native `Area`; an answer holding only the former is a subset presented as complete.
    // 一条本地声明让记录变得不完整。`a` 同时匹配记录里的 `Gauge` 与本地声明的 `Area`；只含前者的答案
    // 就是把子集当成完整答案。
    std::fs::create_dir_all(root.join("native/area")).expect("native directory");
    std::fs::write(
        root.join("native/area/area.cpp"),
        "// fixture\n#include \"xirang/soil.hpp\"\n#include \"xirang/faces.hpp\"\n\nstruct Area {};\n\nXIRANG_SOIL\nconstexpr auto SOIL = ::xirang::soil<Area>()\n    .under(::xirang::faces::gauge::NODE_ID)\n    .named(\"Area\");\n",
    )
    .expect("native declaration");
    let before = derivations();
    let broad = search(&root, &json!({"query": "a", "limit": 100})).expect("an answer");
    assert!(
        broad.contains("kind=Gauge"),
        "the record's own hit stays: {broad}"
    );
    assert!(
        broad.contains("kind=Area"),
        "the producer the record cannot carry is not dropped: {broad}"
    );
    assert!(
        derivations() > before,
        "an incomplete record forces a derivation instead of answering alone: {broad}"
    );
    let _ = std::fs::remove_dir_all(&root);
}
