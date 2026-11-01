//! Pins for the syntax-scoping facts the call graph consumes.
//! 调用图消费的语法作用域事实的钉子。

use super::*;

const METHODS: &str = r#"
struct Entry;
struct Store;
impl Entry {
    fn new() -> Self { Self { } }
    fn helper(&self) { Self::new(); }
}
impl Store {
    fn new() -> Self { Self { } }
}
fn caller() {
    Entry::new();
    Store::new();
}
fn dynamic(value: &dyn Trait) {
    value.run();
}
"#;

#[test]
fn explicit_paths_keep_their_owner() {
    let functions = semantic_functions(METHODS).expect("the fixture parses");
    let caller = functions
        .iter()
        .find(|f| f.name == "caller")
        .expect("caller");
    assert!(
        caller.calls.contains(&"Entry::new".to_owned()),
        "{caller:?}"
    );
    assert!(
        caller.calls.contains(&"Store::new".to_owned()),
        "{caller:?}"
    );
    let entry = functions
        .iter()
        .find(|f| f.qualified == "Entry::new")
        .expect("Entry::new");
    assert_eq!(entry.owner.as_deref(), Some("Entry"));
    let helper = functions
        .iter()
        .find(|f| f.qualified == "Entry::helper")
        .expect("Entry::helper");
    assert!(
        helper.calls.contains(&"Entry::new".to_owned()),
        "{helper:?}"
    );
}

#[test]
fn a_method_call_on_an_unwritten_value_is_unresolved() {
    let functions = semantic_functions(METHODS).expect("the fixture parses");
    let dynamic = functions
        .iter()
        .find(|f| f.name == "dynamic")
        .expect("dynamic");
    assert!(dynamic.calls.is_empty(), "{dynamic:?}");
    assert!(
        dynamic
            .unresolved
            .iter()
            .any(|reason| reason.starts_with("run (")),
        "{dynamic:?}"
    );
}

#[test]
fn a_file_that_does_not_parse_is_not_a_semantic_claim() {
    assert!(semantic_functions("fn broken( {").is_none());
}

#[test]
fn a_dynamic_method_call_lists_the_candidates_it_cannot_choose_between() {
    let source = r#"
trait Runner { fn run(&self); }
struct Fast;
impl Runner for Fast { fn run(&self) {} }
fn caller(value: &dyn Runner) { value.run(); }
"#;
    let functions = semantic_functions(source).expect("fixture parses");
    let caller = functions
        .iter()
        .find(|f| f.name == "caller")
        .expect("caller");
    assert!(caller.calls.is_empty(), "{caller:?}");
    assert!(
        caller
            .unresolved
            .iter()
            .any(|reason| reason.contains("candidates: Fast::run")),
        "{caller:?}"
    );
}

#[test]
fn a_function_pointer_call_is_unknown_not_a_bare_function_edge() {
    let functions =
        semantic_functions("fn caller(action: fn()) { action(); }\n").expect("fixture parses");
    let caller = functions
        .iter()
        .find(|f| f.name == "caller")
        .expect("caller");
    assert!(caller.calls.is_empty(), "{caller:?}");
    assert!(
        caller
            .unresolved
            .iter()
            .any(|reason| reason.contains("local binding/function pointer")),
        "{caller:?}"
    );
}
