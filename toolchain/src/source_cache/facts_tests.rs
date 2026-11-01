//! Exact-byte facts must survive path changes and never conceal content edits.
//! 完整字节事实须跨路径复用，且不能掩盖内容改动。
use super::*;
#[test]
fn content_reuse_changes_and_reverts_are_exact() {
    let source = "fn cache_facts_unique() { upstream_one(); }";
    let first = facts(source);
    assert!(Arc::ptr_eq(&first, &facts(source)));
    let edited = facts("fn cache_facts_unique() { upstream_two(); }");
    assert_ne!(first.calls, edited.calls);
    assert!(Arc::ptr_eq(&first, &facts(source)));
    assert_eq!(
        first.functions,
        xirang_kernel::source::function_symbols(source)
    );
    assert_eq!(first.calls[0], vec!["upstream_one"]);
}
#[test]
fn oversize_units_are_not_retained() {
    let mut cache = Cache::default();
    let unit = facts("fn small_cache_probe() {}");
    cache.insert(&" ".repeat(MAX_BYTES + 1), unit);
    assert!(cache.units.is_empty());
}
