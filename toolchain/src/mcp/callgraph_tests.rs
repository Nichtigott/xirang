//! Tests for the two bounds `xirang.callgraph` needs: definitions and callers.
//! `xirang.callgraph` 需要的两道上限的测试：定义数与调用者数。
//!
//! The measured failure these exist against: a common name (`new`) has 142
//! definitions in the NichUI corpus, and every call site of that name was listed for
//! each of them, which arrived as a 4.5 MB reply. An answer that an agent cannot read
//! is not an answer, so both bounds are pinned here.
//! 这些测试所针对的实测失败：常见名（`new`）在 NichUI 语料里有 142 个定义，而每个定义都列出该名字
//! 在树里的每一个调用点，最终以 4.5 MB 的回复抵达。代理读不下的答案不算答案，因此两道上限都钉在这里。

use std::path::{Path, PathBuf};

use serde_json::json;

use super::{callgraph, is_call_to};

#[test]
fn same_named_definitions_keep_their_own_neighbours() {
    let root = tree("neighbours", 2, 0);
    for index in 0..2 {
        std::fs::write(
            root.join(format!("defs{index}/defs{index}.rs")),
            format!("pub fn new() {{}}\npub fn neighbour{index}() {{}}\n"),
        )
        .unwrap();
    }
    let reply = callgraph(&root, &json!({"function":"new", "source":false})).unwrap();
    let second = reply.split("defs1/defs1.rs:1 fn new").nth(1).unwrap();
    assert!(second.contains("neighbour1 -> defs1/defs1.rs:2"), "{reply}");
    assert!(!second.contains("neighbour0"), "{reply}");
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn batch_calls_preserve_single_answers_and_reject_invalid_members() {
    let root = tree("batch-contract", 1, 1);
    let names = ["new", "caller0", "missing"];
    let expected = names
        .iter()
        .map(|name| callgraph(&root, &json!({"function":name, "source":false})).unwrap())
        .collect::<Vec<_>>()
        .join("\n");
    assert_eq!(
        callgraph(&root, &json!({"function":names, "source":false})).unwrap(),
        expected
    );
    for names in [json!([]), json!(["new", 42]), json!(["new", " "])] {
        assert!(callgraph(&root, &json!({"function":names})).is_err());
    }
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn ambiguous_callees_are_candidates_not_a_claimed_resolution() {
    let root = tree("ambiguous-callee", 4, 1);
    let answer = callgraph(&root, &json!({"function":"caller0", "source":false})).unwrap();
    assert!(
        answer.contains("new -> ambiguous (4 definitions; showing 3)"),
        "{answer}"
    );
    assert!(!answer.contains("new -> defs0/defs0.rs"), "{answer}");
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn withheld_definitions_do_not_expand_into_an_unbounded_path_list() {
    let root = tree("withheld-budget", 100, 0);
    let answer = callgraph(&root, &json!({"function":"new", "limit":1, "source":false})).unwrap();
    assert!(
        answer.contains("99 of 100 definitions withheld"),
        "{answer}"
    );
    assert!(answer.contains("+79 more"), "{answer}");
    assert!(answer.len() < 3000, "{} bytes", answer.len());
    let _ = std::fs::remove_dir_all(root);
}

/// A throwaway source root: `definitions` files each declaring `fn new`, and one
/// file holding `callers` functions that all call it.
/// 一个一次性源码根：`definitions` 个文件各声明一个 `fn new`，外加一个文件，里面有 `callers`
/// 个函数都调用它。
fn tree(label: &str, definitions: usize, callers: usize) -> PathBuf {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "mcp-callgraph-{label}-{}-{sequence}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    for index in 0..definitions {
        let directory = root.join(format!("defs{index}"));
        std::fs::create_dir_all(&directory).expect("definition directory");
        std::fs::write(
            directory.join(format!("defs{index}.rs")),
            "pub fn new() {}\n",
        )
        .expect("definition");
    }
    let callers_directory = root.join("callers");
    std::fs::create_dir_all(&callers_directory).expect("caller directory");
    let mut body = String::new();
    for index in 0..callers {
        body.push_str(&format!("pub fn caller{index}() {{ new(); }}\n"));
    }
    std::fs::write(callers_directory.join("callers.rs"), body).expect("callers");
    root
}

/// A name with several definitions is named as ambiguous, the extra definitions
/// are withheld with their count, and the caller list is capped rather than
/// dumped.
/// 有多个定义的名字会被说成歧义，多出来的定义连同数量一起被扣下，调用者清单被截断而不是倾倒。
#[test]
fn a_common_name_is_bounded_and_its_ambiguity_is_named() {
    let root = tree("ambiguous", 3, 25);
    let reply =
        callgraph(&root, &json!({"function": "new", "limit": 1})).expect("the answer renders");
    assert!(reply.contains("matches 3"), "{reply}");
    assert!(reply.contains("note: 3 definitions match"), "{reply}");
    assert!(reply.contains("pass `path` to select one"), "{reply}");
    // One definition shown, two withheld, both stated.
    // 显示一个定义、扣下两个，两件事都写出来。
    assert!(
        reply.contains("… truncated: 2 of 3 definitions withheld at the limit of 1"),
        "{reply}"
    );
    // The caller list is capped: 25 callers, 20 shown.
    // 调用者清单有上限：25 个调用者，显示 20 个。
    assert!(reply.contains("callers (25):"), "{reply}");
    assert!(
        reply.contains("… truncated: 5 of 25 callers withheld at the limit of 20"),
        "{reply}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// `path` removes the ambiguity, so the note must disappear — it is advice, not
/// boilerplate.
/// `path` 消除了歧义，因此那句提示必须消失——它是建议，不是套话。
#[test]
fn a_path_filter_removes_the_ambiguity_note() {
    let root = tree("filtered", 3, 2);
    let reply = callgraph(&root, &json!({"function": "new", "path": "defs1/defs1.rs"}))
        .expect("the answer renders");
    assert!(reply.contains("matches 1"), "{reply}");
    assert!(!reply.contains("definitions match"), "{reply}");
    assert!(reply.contains("defs1/defs1.rs:1 fn new"), "{reply}");
    let _ = std::fs::remove_dir_all(&root);
}

/// The catalog's contract for `limit` is the implementation's behaviour, pinned here rather than
/// only declared there: absent `limit` shows 5 definitions, a `limit` past the declared maximum of
/// 50 is *clamped* (the description says "at most 50", not "refused"), and a `limit` below the
/// declared minimum of 1 behaves as 1. The contract used to be undeclared while the
/// implementation read the key, which made `raise \`limit\`` advice a caller could not act on
/// (audit `LGC-LG-43`).
/// catalog 对 `limit` 的契约就是实现的行为，并且钉在这里、而不只是声明在那里：不传 `limit` 显示
/// 5 个定义；超过声明上限 50 的 `limit` 被**夹到 50**（描述说的是"至多 50"，不是"拒绝"）；低于声明
/// 下限 1 的 `limit` 按 1 处理。过去这个键未声明而实现却在读它，于是 `raise \`limit\`` 那条建议是
/// 调用方无法执行的（审计 `LGC-LG-43`）。
#[test]
fn the_limit_contract_is_the_one_the_catalog_declares() {
    let root = tree("limits", 60, 1);
    let default = callgraph(&root, &json!({"function": "new"})).expect("the answer renders");
    assert!(default.contains("matches 60"), "{default}");
    assert!(
        default.contains("… truncated: 55 of 60 definitions withheld at the limit of 5"),
        "the default shows 5 of 60 definitions: {default}"
    );
    let clamped =
        callgraph(&root, &json!({"function": "new", "limit": 999})).expect("the answer renders");
    assert!(
        clamped.contains("… truncated: 10 of 60 definitions withheld at the limit of 50"),
        "a limit past the declared maximum is clamped to 50, not refused: {clamped}"
    );
    let zero =
        callgraph(&root, &json!({"function": "new", "limit": 0})).expect("the answer renders");
    assert!(
        zero.contains("… truncated: 59 of 60 definitions withheld at the limit of 1"),
        "a limit below the declared minimum behaves as 1: {zero}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// The bodies are in the answer unless the caller asks for the short one.
/// 函数体默认就在答案里，除非调用方点名要短的那份。
///
/// Measured: every evaluation round spent one extra call after this tool had shown only names, so
/// the default is now the answer that needs no follow-up, and `source: false` is the opt-out that
/// says so in the reply.
/// 实测：评测每一轮都在这个工具只给了名字之后多花一次调用，因此默认改成"不需要补问"的那份答案，而
/// `source: false` 是那个会在回复里说明自己的退出选项。
/// The source block starts at the doc comment's first line, because that pairing is what lets a
/// reader catch a promise the implementation contradicts — the round-4 trap was decided by it.
/// 源码块从文档注释首行起打，因为正是这个并排让读者抓住"承诺与实现相矛盾"——第 4 题的陷阱就由它裁决。
///
/// The line now carries the `contract` marker (W1.6): the pairing is the strongest signal this bridge
/// prints, so the reader should not have to work out which half each printed line is. The intent of
/// this pin is unchanged — **the doc still rides along** — so the expectation moves with the shape
/// rather than the implementation being narrowed to keep the old string.
/// 这一行现在带 `contract` 标记（W1.6）：这个并排是本桥打印的最强信号，因此读者不必自己分辨每一行是
/// 哪一半。本钉子的意图不变——**文档仍然随行打出**——因此预期随形状一起改，而不是把实现收窄去迁就旧
/// 字符串。
#[test]
fn the_source_block_starts_at_the_docs_first_line() {
    let root = crate::mcp::tools::tools_tests::scratch_package("docline");
    let answer = callgraph(&root, &json!({"function": "used"})).expect("an answer");
    let source = answer
        .split("  source:\n")
        .nth(1)
        .expect("the bodies are included by default");
    let first = source.lines().next().unwrap_or_default();
    assert!(
        first.trim_start().starts_with("contract ///"),
        "the doc rides along, marked as the contract: {first:?}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn the_bodies_are_included_unless_asked_otherwise() {
    let root = tree("bodies", 1, 1);
    let full = callgraph(&root, &json!({"function": "new"})).expect("an answer");
    assert!(
        full.contains("source:"),
        "the bodies come by default: {full}"
    );
    let short = callgraph(&root, &json!({"function": "new", "source": false})).expect("an answer");
    assert!(!short.contains("  source:"), "{short}");
    assert!(short.contains("bodies omitted on request"), "{short}");
    let _ = std::fs::remove_dir_all(&root);
}

/// The orphan view names what nothing here calls, and says what its count can and cannot see.
/// 孤儿视图点名这里没人调用的东西，并说清它的计数看得见什么、看不见什么。
#[test]
fn the_orphan_view_names_what_nothing_calls() {
    let root = crate::mcp::tools::tools_tests::scratch_package("orphans");
    // One test function, which no other function calls: the harness is its caller, so it belongs in
    // the count the view reports rather than in the list it prints.
    // 一个测试函数，没有任何函数调用它：测试框架才是它的调用者，因此它属于视图报出的那个**计数**，
    // 而不属于它打印的那张**清单**。
    std::fs::create_dir_all(root.join("tests")).expect("fixture test dir");
    std::fs::write(
        root.join("tests/orphans.rs"),
        "#[test]\nfn the_only_test() { assert!(fixture_orphans::used() == 1); }\n",
    )
    .expect("fixture test file");
    let answer = callgraph(&root, &json!({"orphans": true})).expect("an answer");
    assert!(answer.contains("fn    orphan ->"), "{answer}");
    assert!(
        !answer.contains("fn    used ->"),
        "`used` is called by `call_used`: {answer}"
    );
    assert!(
        answer.contains("static"),
        "the count says it is static: {answer}"
    );
    assert!(
        answer.contains("invisible to it"),
        "and says what it cannot see: {answer}"
    );
    assert!(
        !answer.contains("fn    the_only_test ->"),
        "a test function is not listed as an orphan: {answer}"
    );
    assert!(
        answer.contains("1 function(s) in test files have no static caller"),
        "but the test half is counted and named: {answer}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// A printed source line says which half it is: the doc comment is the contract, the body is the
/// implementation. The pair disagreeing is the strongest signal the bridge prints, so the reader
/// should not have to work out which line is which.
/// 打印出来的源码行要说明自己是哪一半：文档注释是契约，函数体是实现。两者不一致是本桥打印的最强信号，
/// 因此读者不必自己分辨哪一行是哪种。
#[test]
fn a_printed_source_line_says_contract_or_implementation() {
    assert!(
        super::source_line("/// Whether this entry may be posted at all.")
            .starts_with("    contract"),
        "a doc line is the contract"
    );
    assert!(
        super::source_line("    self.has_receipt && self.amount != 0").starts_with("    impl"),
        "a body line is the implementation"
    );
    assert!(
        super::source_line("//! Module doc.").starts_with("    contract"),
        "an inner doc line is the contract too"
    );
    // The line itself is printed unchanged after the marker.
    // 标记之后原样打印那一行。
    assert!(
        super::source_line("/// c").ends_with(" /// c\n"),
        "the line survives the marker"
    );
}

/// A caller's note says only what the two labels prove — that it is a test file, and that it sits
/// outside the definition's own directory. It never names a package: a path cannot prove one.
/// 调用者的注只说两个标签能证明的事 —— 它是测试文件、以及它在该定义自己的目录之外。它从不点名包名：
/// 路径证明不了包名。
#[test]
fn a_caller_note_says_only_what_the_labels_prove() {
    let definition = "crates/core/src/store.rs";

    assert_eq!(
        super::caller_note(definition, "crates/core/src/other.rs"),
        None,
        "same directory, not a test file: nothing worth saying"
    );
    assert_eq!(
        super::caller_note(definition, "crates/report/tests/report.rs").as_deref(),
        Some("a test file; outside this file's directory (crates/report/tests)")
    );
    assert_eq!(
        super::caller_note(definition, "crates/report/src/query.rs").as_deref(),
        Some("outside this file's directory (crates/report/src)")
    );
    assert_eq!(
        super::caller_note(definition, "crates/core/tests/audit.rs").as_deref(),
        Some("a test file; outside this file's directory (crates/core/tests)")
    );
}

/// The rule "a call names a definition" is one rule: the bare name and the qualified path both
/// count, and a name the call does not end on at a `::` boundary does not.
/// "一次调用点名某个定义"是一条规则：裸名与限定路径都算，而调用并不在 `::` 边界上以其结尾的名字不算。
#[test]
fn a_call_names_a_definition_by_the_one_rule() {
    assert!(is_call_to("paint", "paint"));
    assert!(is_call_to("::paint", "paint"));
    assert!(is_call_to("control::paint", "paint"));
    assert!(is_call_to("crate::control::paint", "paint"));
    assert!(!is_call_to("repaint", "paint"));
    assert!(!is_call_to("paint_brush", "paint"));
    assert!(!is_call_to("paint", "pain"));
}

/// The definition half keeps its own direction, and that is pinned as behaviour rather than left as
/// a comment: a query spelled as a path does **not** name the bare definition, so
/// `--function inner::helper` answers exactly the no-match sentence while `--function helper`
/// finds it.
/// 定义名那一半保持自己的方向，而且这一点作为**行为**被钉住、不是只留在注释里：以路径拼写的查询**不**
/// 点名裸定义，因此 `--function inner::helper` 恰好答那句"没有匹配"，而 `--function helper` 找得到。
///
/// The reverse spelling (`is_call_to(query, &function.name)`) is the one that would change this, and
/// it is a query-semantics choice rather than a copy of the caller rule: the verification round
/// measured it, and this pin is what keeps the choice visible if someone takes it by accident.
/// 反方向的拼法（`is_call_to(query, &function.name)`）才是会改变它的那个，而那是**查询语义**的取舍、
/// 不是调用者规则的副本：复核那一轮量到了它，而这条钉子就是"有人误踩时它会现形"的保证。
#[test]
fn the_definition_half_keeps_its_own_direction() {
    let root = std::env::temp_dir().join(format!("mcp-callgraph-query-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("src")).expect("fixture dirs");
    std::fs::write(
        root.join("src/lib.rs"),
        "//! A query fixture.\n\
         pub mod inner {\n\
         \x20   pub fn helper() -> i64 { 1 }\n\
         }\n",
    )
    .expect("fixture source");
    let bare = callgraph(&root, &json!({"function": "helper"})).expect("an answer");
    assert!(bare.contains("matches 1"), "{bare}");
    assert!(bare.contains("src/lib.rs:3 fn helper"), "{bare}");
    let qualified = callgraph(&root, &json!({"function": "inner::helper"})).expect("an answer");
    assert!(
        qualified.contains("src/lib.rs:3 fn helper"),
        "a module-qualified query now names the free function it spells: {qualified}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// One implementation, four consumers: the name view, `orphans`, the census's test-reachability
/// column and `affected` all reach a definition through `is_call_to`, and none of them keeps a
/// clause of its own.
/// 一份实现、四个消费方：名字视图、`orphans`、总账的测试可达性栏与 `affected` 都经 `is_call_to`
/// 到达定义，谁也不留自己的子句。
///
/// This is the pin behaviour tests cannot give. Two byte-identical clauses behave identically until
/// one of them is edited, so a behaviour test stays green through exactly the change this refinement
/// exists to prevent; only reading the sources sees the copy. The counts are taken over every
/// shipping source file in this checkout and the body assertions are anchored to each consumer body
/// (four of them — `callgraph.rs` contributes the name view and the orphan view), so a further
/// spelling — in any file, or text elsewhere in these files — fails here.
/// 这是行为测试给不出的钉子。两段逐字相同的子句在被改动之前行为一致，因此正是这次收敛要防的那种改动
/// 会让行为测试保持绿色；只有读源码才看得见那份副本。计数取自本检出**每一个出厂源码文件**，函数体断言
/// 锚定在每一个消费方函数体上（共四个——`callgraph.rs` 贡献名字视图与孤儿视图），因此再多一份拼写——
/// 不论出现在哪个文件、还是这些文件里别处的文本——都会在这里失败。
#[test]
fn every_view_reaches_a_definition_through_the_one_predicate() {
    let sources = shipped_rust_sources();
    let joined = sources
        .iter()
        .map(|(_, text)| text.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert_eq!(
        joined.matches("fn call_matches(").count(),
        1,
        "the semantic call predicate is defined once, in the module that owns the call graph"
    );
    assert_eq!(
        joined.matches("strip_suffix(name)").count(),
        1,
        "the rule's body is spelled exactly once in this checkout's shipping sources"
    );
    for (file, consumer) in [
        ("toolchain/src/mcp/callgraph.rs", "fn function_answer"),
        ("toolchain/src/mcp/claims.rs", "fn reachability_column"),
        ("toolchain/src/mcp/relations.rs", "fn reaching"),
    ] {
        let source = sources
            .iter()
            .find(|(path, _)| path == file)
            .map(|(_, text)| text.as_str())
            .unwrap_or_else(|| panic!("{file} is part of this checkout"));
        let body = if consumer == "fn reachability_column" {
            source
        } else {
            body_of(source, consumer)
        };
        assert!(
            body.contains("call_matches(") || body.contains("is_call_to("),
            "{file} does not reach a definition through a shared predicate: {body}"
        );
        assert!(
            !body.contains("ends_with("),
            "{file} must not keep its own matching clause: {body}"
        );
    }
}

/// Same-named constructors on different types are not interchangeable: a qualified query names the
/// owner, a bare query still lists both definitions and prints their owners.
/// 不同类型上的同名构造器不可互换：限定查询点名所属者，裸查询仍列出两个定义并打印所属者。
#[test]
fn same_named_methods_are_separated_by_their_owner() {
    let root = std::env::temp_dir().join(format!("mcp-callgraph-owner-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("src")).expect("fixture dirs");
    std::fs::write(
        root.join("src/lib.rs"),
        "pub struct Entry;\n\
         pub struct Store;\n\
         impl Entry { pub fn new() -> Self { Self } }\n\
         impl Store { pub fn new() -> Self { Self } }\n\
         pub fn caller() { Entry::new(); Store::new(); }\n",
    )
    .expect("fixture source");
    let entry = callgraph(&root, &json!({"function": "Entry::new"})).expect("Entry answer");
    assert!(entry.contains("matches 1"), "{entry}");
    assert!(entry.contains("fn Entry::new"), "{entry}");
    assert!(entry.contains("callers (1): src/lib.rs::caller"), "{entry}");
    let store = callgraph(&root, &json!({"function": "Store::new"})).expect("Store answer");
    assert!(store.contains("fn Store::new"), "{store}");
    assert!(store.contains("callers (1): src/lib.rs::caller"), "{store}");
    assert!(
        !entry.contains("fn Store::new"),
        "Entry answer names Store as the definition: {entry}"
    );
    let bare = callgraph(&root, &json!({"function": "new"})).expect("bare answer");
    assert!(bare.contains("matches 2"), "{bare}");
    assert!(bare.contains("Entry::new"), "{bare}");
    assert!(bare.contains("Store::new"), "{bare}");
    let _ = std::fs::remove_dir_all(&root);
}

/// The old bare-suffix rule would have matched `Entry::new` to `Store::new`; the semantic matcher
/// requires the owner when the definition has one.
/// 旧的裸后缀规则会把 `Entry::new` 匹配到 `Store::new`；语义匹配器在定义有所属者时要求所属者。
#[test]
fn the_old_suffix_rule_would_have_false_positived() {
    let root = std::env::temp_dir().join(format!("mcp-callgraph-old-rule-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("src")).expect("fixture dirs");
    std::fs::write(
        root.join("src/lib.rs"),
        "pub struct Entry; pub struct Store;\n\
         impl Entry { pub fn new() -> Self { Self } }\n\
         impl Store { pub fn new() -> Self { Self } }\n\
         pub fn caller() { Entry::new(); }\n",
    )
    .expect("fixture source");
    let sources = crate::mcp::source_index::load_sources(&root).expect("sources");
    let store = sources[0]
        .functions
        .iter()
        .find(|function| function.qualified == "Store::new")
        .expect("Store::new");
    assert!(
        crate::mcp::callgraph::is_call_to("Entry::new", &store.name),
        "the old rule cannot tell the two owners apart"
    );
    assert!(
        !crate::mcp::callgraph::call_matches(store, "Entry::new"),
        "the semantic rule rejects the wrong owner"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// A Python AST provider is consumed by the same callgraph, not a separate mock path.
/// Python AST 提供者由同一张 callgraph 消费，而不是单独的 mock 路径。
#[cfg(feature = "lang-python")]
#[test]
fn python_class_methods_are_separated_by_owner() {
    let root = std::env::temp_dir().join(format!("mcp-callgraph-python-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("fixture dirs");
    std::fs::write(
        root.join("fair.py"),
        "class Entry:\n    def new():\n        return Entry()\nclass Store:\n    def new():\n        return Store()\ndef caller():\n    Entry.new()\n    Store.new()\n",
    )
    .expect("python source");
    let entry = callgraph(&root, &json!({"function": "Entry::new"})).expect("Entry answer");
    assert!(entry.contains("matches 1"), "{entry}");
    assert!(entry.contains("fn Entry::new"), "{entry}");
    assert!(entry.contains("callers (1): fair.py::caller"), "{entry}");
    assert!(!entry.contains("fn Store::new"), "{entry}");
    let store = callgraph(&root, &json!({"function": "Store::new"})).expect("Store answer");
    assert!(
        store.contains("matches 1") && store.contains("fn Store::new"),
        "{store}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// A clang-backed C++ provider is consumed by the same callgraph, not a parallel mock.
/// clang C++ 提供者由同一张 callgraph 消费，而不是并行的 mock。
#[cfg(feature = "lang-cpp")]
#[test]
fn cpp_class_methods_are_separated_by_owner() {
    let root = std::env::temp_dir().join(format!("mcp-callgraph-cpp-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("fixture dirs");
    std::fs::write(
        root.join("fair.cpp"),
        "struct Entry { static Entry make(); };\n\
         struct Store { static Store make(); };\n\
         Entry Entry::make() { return Entry{}; }\n\
         Store Store::make() { return Store{}; }\n\
         void caller() { Entry::make(); Store::make(); }\n",
    )
    .expect("cpp source");
    let entry = callgraph(&root, &json!({"function": "Entry::make"})).expect("Entry answer");
    assert!(entry.contains("matches 1"), "{entry}");
    assert!(entry.contains("fn Entry::make"), "{entry}");
    assert!(entry.contains("callers (1): fair.cpp::caller"), "{entry}");
    assert!(!entry.contains("fn Store::make"), "{entry}");
    let store = callgraph(&root, &json!({"function": "Store::make"})).expect("Store answer");
    assert!(
        store.contains("matches 1") && store.contains("fn Store::make"),
        "{store}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// A C++ namespace is part of the identity the answer prints, and a call resolves to its namespace.
/// C++ 命名空间是答案打印的身份的一部分，而调用解析到它所在的命名空间。
#[cfg(feature = "lang-cpp")]
#[test]
fn cpp_namespaces_keep_two_same_named_classes_apart() {
    let root = std::env::temp_dir().join(format!(
        "mcp-callgraph-cpp-namespace-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("fixture dirs");
    std::fs::write(
        root.join("ns.cpp"),
        "namespace shop { struct Entry { static Entry make(); }; Entry Entry::make() { return Entry{}; } }\n\
         namespace admin { struct Entry { static Entry make(); }; Entry Entry::make() { return Entry{}; } }\n\
         void caller() { shop::Entry::make(); }\n",
    )
    .expect("cpp source");
    let answer = callgraph(&root, &json!({"function": "Entry::make"})).expect("answer");
    assert!(answer.contains("matches 2"), "{answer}");
    assert!(answer.contains("fn shop::Entry::make"), "{answer}");
    assert!(answer.contains("fn admin::Entry::make"), "{answer}");
    // The call names `shop`, so the caller belongs to that definition only.
    // 调用写的是 `shop`，因此调用者只属于那个定义。
    assert!(answer.contains("callers (1): ns.cpp::caller"), "{answer}");
    let _ = std::fs::remove_dir_all(&root);
}

/// A template instantiation is its own target in the answer, not the primary template.
/// 模板实例化在答案里是它自己的目标，而不是主模板。
#[cfg(feature = "lang-cpp")]
#[test]
fn cpp_template_calls_name_the_instantiation_not_the_pattern() {
    let root =
        std::env::temp_dir().join(format!("mcp-callgraph-cpp-template-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("fixture dirs");
    std::fs::write(
        root.join("t.cpp"),
        "template<typename T> T pick(T x) { return x; }\n\
         void caller() { pick(1); }\n",
    )
    .expect("cpp source");
    let caller = callgraph(&root, &json!({"function": "caller"})).expect("caller answer");
    assert!(
        caller.contains("callees: pick<int> -> t.cpp:1"),
        "the callee is the instantiation: {caller}"
    );
    let pick = callgraph(&root, &json!({"function": "pick"})).expect("pick answer");
    assert!(pick.contains("matches 2"), "{pick}");
    assert!(pick.contains("fn pick<int>"), "{pick}");
    let _ = std::fs::remove_dir_all(&root);
}

/// An unparseable C++ translation unit is named as unknown, not silently treated as empty.
/// 无法解析的 C++ 翻译单元会被点名为未知，而不是静默当成空文件。
#[cfg(feature = "lang-cpp")]
#[test]
fn an_unparseable_cpp_file_is_reported_as_unknown() {
    let root =
        std::env::temp_dir().join(format!("mcp-callgraph-cpp-broken-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("fixture dirs");
    std::fs::write(root.join("broken.cpp"), "struct Broken {").expect("broken source");
    let reply = callgraph(&root, &json!({"function": "Broken"})).expect("an answer");
    assert!(reply.starts_with("language facts unknown:"), "{reply}");
    assert!(reply.contains("broken.cpp"), "{reply}");
    let _ = std::fs::remove_dir_all(&root);
}

/// A broken Python file is named as unknown, not silently treated as empty.
/// 损坏的 Python 文件会被点名为未知，而不是静默当成空文件。
#[cfg(feature = "lang-python")]
#[test]
fn an_unparseable_python_file_is_reported_as_unknown() {
    let root = std::env::temp_dir().join(format!("mcp-callgraph-py-broken-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("fixture dirs");
    std::fs::write(
        root.join("broken.py"),
        "def Broken(:
",
    )
    .expect("broken source");
    let reply = callgraph(&root, &json!({"function": "Broken"})).expect("an answer");
    assert!(reply.starts_with("language facts unknown:"), "{reply}");
    assert!(reply.contains("broken.py"), "{reply}");
    let _ = std::fs::remove_dir_all(&root);
}

/// An unresolved call is reported as unknown, never silently dropped or promoted to an edge.
/// 未解析调用会作为未知报告，绝不静默丢弃或提升成边。
#[test]
fn unresolved_calls_are_counted_in_the_answer() {
    let root = std::env::temp_dir().join(format!("mcp-callgraph-unknown-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("src")).expect("fixture dirs");
    std::fs::write(
        root.join("src/lib.rs"),
        "pub fn caller(value: &dyn crate::Trait) { value.run(); }\n",
    )
    .expect("fixture source");
    let reply = callgraph(&root, &json!({"function": "caller"})).expect("an answer");
    assert!(reply.contains("note unresolved call sites: 1"), "{reply}");
    assert!(
        reply.contains("run (receiver type is not written)"),
        "{reply}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// Declared binding and a verified trace observation are reported on the same query, without one
/// filling the other. The trace is written to a real host tree and removed afterwards.
/// 声明绑定与已核验 trace 观察在同一次查询里分别报告，互不填充；trace 写进真实宿主后删除。
#[test]
fn declared_binding_and_observed_are_separate_lines_on_the_same_query() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../examples/control-button");
    let namespace = crate::mcp::registry::namespace(&root).expect("namespace");
    let views = crate::build_method::face_views(&root, &namespace).expect("faces");
    let button = views
        .iter()
        .find(|view| view.path == "root/control/button")
        .expect("button face");
    let mut trace = crate::run_method::CallTrace::full();
    trace.with(xirang_kernel::root_node_id(&namespace), "caller", |trace| {
        trace.with(button.id, "paint", |_| {});
    });
    let path = crate::run_method::trace_artifact_path(&root);
    crate::run_method::write_trace_artifact(&trace, &path, &namespace).expect("trace writes");
    let reply = callgraph(&root, &json!({"function": "Button::paint"})).expect("answer");
    let _ = std::fs::remove_file(&path);
    assert!(
        reply.contains("binding: declared graft control_button_graft::button_fast::NODE_ID"),
        "{reply}"
    );
    assert!(
        reply.contains("observed (one recorded run") && reply.contains("callers [caller]"),
        "{reply}"
    );
}

/// Every shipping Rust file in this checkout, as `(path relative to the checkout root, text)`.
/// 本检出里每一个出厂 Rust 文件，形如 `(相对检出根的路径, 文本)`。
///
/// Test files are left out on purpose: this rule is about the code that ships, and a test is
/// allowed to quote it. Collecting the paths (rather than `include_str!`-ing a hand-kept list) is
/// what makes the counts above cover the whole checkout instead of the files that happened to
/// consume the predicate the day this pin was written.
/// 有意排除测试文件：这条规则针对的是会出厂的代码，而测试允许引用它。收集路径（而不是把一份手工清单
/// `include_str!` 进来）正是让上面的计数覆盖**整个检出**、而不是"写下这条钉子那天恰好用到该判据的那几个
/// 文件"的原因。
fn shipped_rust_sources() -> Vec<(String, String)> {
    // The verb table spells a private recursive helper as `visit_` (D-5): `walk` is a bare verb,
    // and `collect_` on a private function is the table's `visit_`.
    // 动词表把私有的递归辅助函数拼作 `visit_`（D-5）：`walk` 是裸动词，而私有函数上的 `collect_` 正是
    // 表里叫作 `visit_` 的那种东西。
    fn visit_sources(root: &Path, directory: &Path, found: &mut Vec<(String, String)>) {
        let Ok(entries) = std::fs::read_dir(directory) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                visit_sources(root, &path, found);
                continue;
            }
            let spelling = path.to_string_lossy();
            let is_rust = path.extension().and_then(|ext| ext.to_str()) == Some("rs");
            if !is_rust || spelling.ends_with("_tests.rs") || spelling.contains("/tests/") {
                continue;
            }
            let relative = path
                .strip_prefix(root)
                .map(|relative| relative.to_string_lossy().into_owned())
                .unwrap_or_default();
            found.push((relative, std::fs::read_to_string(&path).unwrap_or_default()));
        }
    }
    let checkout = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("this crate lives inside the checkout")
        .to_path_buf();
    let mut found = Vec::new();
    for member in ["kernel", "toolchain", "macro", "conventions", "examples"] {
        visit_sources(&checkout, &checkout.join(member).join("src"), &mut found);
    }
    found
}

/// The source of one top-level function: from the line that declares it to the closing brace at
/// column zero, so a body assertion cannot be satisfied by text elsewhere in the file.
/// 一个顶层函数的源码：从声明它的那一行到列零处的收尾花括号——这样函数体断言不会被文件里别处的文本
/// 满足。
fn body_of<'a>(source: &'a str, declaration: &str) -> &'a str {
    let start = source
        .find(declaration)
        .expect("the declaration is in the file");
    let rest = &source[start..];
    let Some(open) = rest.find('{') else {
        return rest;
    };
    let masked = xirang_kernel::source::mask_non_code(rest);
    let mut depth = 0usize;
    for (offset, character) in masked[open..].char_indices() {
        match character {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return &rest[..open + offset + 1];
                }
            }
            _ => {}
        }
    }
    rest
}

/// The definitions the cap drops are **named**, by the file each lives in.
/// 被上限丢掉的定义**被点名**，点的是每个所在的文件。
///
/// Measured (W8, h1): nine definitions of `offset`, a default limit of five, and the one the question
/// was about (`toggle`) among the four cut — the reader only got there because a *callers* sentence
/// happened to name it. Before this pin the cap sentence gave a count and advice ("raise `limit` or
/// pass `path`") but never said which four, so an agent had to spend a call to find out.
/// 量到的（W8 的 h1）：`offset` 九个定义、默认上限五个，而问题所关心的那一个（`toggle`）在被切掉的四个
/// 里——读者之所以还能找到，只是因为另一条**调用者**的句子碰巧点了它的名。在这条钉子之前，上限那句只给了
/// 数字与建议（"raise `limit` or pass `path`"），从不说哪四个，于是代理只能再花一次调用去弄清。
#[test]
fn the_definition_cap_names_the_files_it_dropped() {
    let root = crate::mcp::tools::tools_tests::scratch_package("callgraph-cap");
    for name in ["alpha", "beta", "gamma", "delta", "epsilon", "zeta"] {
        let directory = root.join("src").join(name);
        std::fs::create_dir_all(&directory).expect("fixture dir");
        std::fs::write(
            directory.join(format!("{name}.rs")),
            format!("pub fn twin() -> i32 {{ 1 }}\npub fn {name}_only() -> i32 {{ 2 }}\n"),
        )
        .expect("fixture source");
    }
    let answer = super::callgraph(&root, &json!({"function": "twin"})).expect("an answer");
    assert!(answer.contains("matches 6"), "{answer}");
    assert!(
        answer.contains("definitions withheld at the limit of"),
        "the cap is announced: {answer}"
    );
    assert!(
        answer.contains("src/zeta/zeta.rs") || answer.contains("src/epsilon/epsilon.rs"),
        "and it names the files it dropped, not just how many: {answer}"
    );
    assert!(
        !answer.contains("`twin`, `twin`"),
        "naming the function would identify nothing — every one is called `twin`: {answer}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// **A caveat on the callee is printed where the caller's question is asked.**
/// **被调用者的 caveat 印在"问调用者"的那个答案里。**
///
/// A decorator on a Python method may replace its binding, and the fact belongs to the definite edge
/// from `caller`: the definition's own block is not where a reader asking about the caller is
/// looking, so the same caveat travels to the caller's answer.
/// Python 方法上的装饰器可能替换它的绑定，而这个事实属于从 `caller` 出发的那条确定边：问调用者的读者
/// 并不在看定义自己的块，因此同一条 caveat 跟到调用者的答案里。
#[cfg(feature = "lang-python")]
#[test]
fn a_callee_caveat_travels_to_the_callers_answer() {
    let root = std::env::temp_dir().join(format!("mcp-callgraph-caveat-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("fixture dirs");
    std::fs::write(
        root.join("fair.py"),
        "def deco(fn):\n    return fn\nclass Entry:\n    @deco\n    def make(self):\n        return 1\ndef caller():\n    return Entry.make()\n",
    )
    .expect("python source");
    let caller = callgraph(&root, &json!({"function": "caller"})).expect("caller answer");
    assert!(
        caller.contains("caveat (callee): Entry::make")
            && caller.contains("@deco (decorator may replace this binding)"),
        "the caller is told what stands behind the edge: {caller}"
    );
    let definition = callgraph(&root, &json!({"function": "Entry::make"})).expect("definition");
    assert!(
        definition.contains("caveat: @deco (decorator may replace this binding)"),
        "and the definition says it on its own block: {definition}"
    );
    let _ = std::fs::remove_dir_all(&root);
}
