//! Pins for the shared AST walker: owner from the class chain, calls grouped per function, and the
//! pre-order stream rule.
//! 共享 AST 遍历器的钉子：从类链取所属者、按函数分组调用，以及先序流规则。

use super::{KindMap, qualified, walk_nodes};

const KINDS: KindMap = KindMap {
    function: &["FunctionDef"],
    class: &["ClassDef"],
    call: &["Call"],
};

fn stream(nodes: serde_json::Value) -> Vec<u8> {
    nodes.to_string().into_bytes()
}

/// A method's owner is its class; its calls and unresolved reasons are the ones inside it; a call in
/// another function is not attributed to it.
/// 方法的所属者是它的类；它的调用与未解析原因只包含它内部那些；另一个函数里的调用不会算到它头上。
#[test]
fn the_walker_assembles_one_function_from_its_nodes() {
    let bytes = stream(serde_json::json!([
        {"kind": "ClassDef", "name": "Entry"},
        {"kind": "FunctionDef", "name": "make", "line": 2, "end_line": 3, "parent": 0,
         "caveats": ["@deco may replace this binding"]},
        {"kind": "Call", "parent": 1, "target": "Store::open"},
        {"kind": "Call", "parent": 1, "target": "Store::open"},
        {"kind": "Call", "parent": 1, "unresolved": "handler(...) (dynamic binding)"},
        {"kind": "FunctionDef", "name": "free", "line": 6, "end_line": 7},
        {"kind": "Call", "parent": 5, "target": "Other::run"}
    ]));
    let functions = walk_nodes(&bytes, &KINDS, "the fixture").expect("the stream walks");
    assert_eq!(functions.len(), 2);
    assert_eq!(functions[0].qualified, "Entry::make");
    assert_eq!(functions[0].owner.as_deref(), Some("Entry"));
    assert_eq!(
        functions[0].calls,
        vec!["Store::open".to_owned()],
        "deduplicated"
    );
    assert_eq!(
        functions[0].unresolved,
        vec!["handler(...) (dynamic binding)".to_owned()]
    );
    assert_eq!(
        functions[0].caveats,
        vec!["@deco may replace this binding".to_owned()]
    );
    assert_eq!(functions[1].qualified, "free");
    assert_eq!(functions[1].calls, vec!["Other::run".to_owned()]);
}

/// A parent that is not earlier in the stream is refused rather than guessed.
/// 父节点不在流中更早位置时被拒绝，而不是猜。
#[test]
fn a_broken_stream_is_refused() {
    let bytes = stream(serde_json::json!([
        {"kind": "FunctionDef", "name": "a", "parent": 1},
        {"kind": "ClassDef", "name": "Entry"}
    ]));
    let error = walk_nodes(&bytes, &KINDS, "the fixture").expect_err("the stream is not pre-order");
    assert!(error.contains("not earlier in the stream"), "{error}");
}

/// A method keys as `Owner::name`; a free function is its bare name — one rule for every provider.
/// 方法以 `Owner::name` 为键；自由函数是裸名——每个提供者同一条规则。
#[test]
fn the_qualified_spelling_has_one_rule() {
    assert_eq!(qualified(Some("Entry"), "make"), "Entry::make");
    assert_eq!(qualified(None, "main"), "main");
}
