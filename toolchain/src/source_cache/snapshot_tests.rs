//! Snapshots round-trip parser facts and ignore incompatible or corrupt payloads.
//! 快照往返保留解析事实，忽略不兼容或损坏的载荷。
use super::*;
#[test]
fn cheap_leaves_do_not_pay_persistence_cost() {
    let root = std::env::temp_dir().join(format!("xirang-cheap-snapshot-{}", std::process::id()));
    {
        let _scope = scope(&root);
        super::super::facts("fn cheap_leaf_snapshot_probe() -> u32 { 7 }");
    }
    assert!(!root.join("target/xirang/cache/syntax-facts.json").exists());
}
#[test]
fn shared_snapshot_roundtrips_and_rejects_corruption() {
    let root = std::env::temp_dir().join(format!("xirang-snapshot-{}", std::process::id()));
    let source = "fn snapshot_unique() { other(); }";
    {
        let _scope = scope(&root);
        super::super::facts(source);
    }
    let path = root.join("target/xirang/cache/syntax-facts.json");
    let value: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    assert_eq!(value["schema"], schema());
    let rows: Vec<Value> = serde_json::from_str(value["payload"].as_str().unwrap()).unwrap();
    let row = rows.iter().find(|row| row[0] == source).unwrap();
    let unit = decode(row).unwrap();
    assert_eq!(
        unit.functions,
        xirang_kernel::source::function_symbols(source)
    );
    assert_eq!(unit.calls[0], vec!["other"]);
    std::fs::write(&path, "{corrupt").unwrap();
    restore(&path);
    assert_eq!(super::super::facts(source).calls[0], vec!["other"]);
    std::fs::remove_dir_all(root).unwrap();
}
