//! The ledger preserves native contracts and all declared consumers alongside executed rows.
//! 账本在执行行之外保留原生合同与全部已声明消费者。
use super::*;
#[test]
fn native_responsibility_evidence_joins_the_regression_ledger() {
    let root = std::env::temp_dir().join(format!(
        "xirang-native-plan-{}",
        crate::mcp::check_jobs::new_id()
    ));
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::write(root.join("Cargo.toml"), format!("[package]\nname='native-plan'\nversion='0.0.0'\nedition='2024'\n[workspace]\n[build-dependencies]\nxirang-toolchain={{path={:?}}}\n", env!("CARGO_MANIFEST_DIR"))).unwrap();
    std::fs::write(root.join("src/lib.rs"), "// host\n").unwrap();
    std::fs::write(root.join("build.rs"), "fn main() {}\n").unwrap();
    crate::mcp::apply::apply(&root, &json!({"action":"add","apply":true,"fields":{"module":"provider","kind":"Provider","needs_registry":true,"provides":"cap.render"}})).unwrap();
    for module in ["consumer", "sibling"] {
        crate::mcp::apply::apply(&root, &json!({"action":"add","apply":true,"parent":"root/provider","fields":{"module":module,"kind":if module == "consumer" {"Consumer"} else {"Sibling"},"requires":"cap.render=>Provider"}})).unwrap();
    }
    let (rows, evidence) = obligations(&root, &json!(["src/provider/provider.rs"])).unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["state"], "passed", "{rows:?}");
    let evidence = evidence[0].to_string();
    for expected in [
        "cap.render",
        "root/provider/consumer",
        "root/provider/sibling",
        "runtime",
    ] {
        assert!(evidence.contains(expected), "{evidence}");
    }
    assert!(crate::mcp::index::wait_until_idle(
        &root,
        std::time::Duration::from_secs(30)
    ));
    std::fs::remove_dir_all(root).unwrap();
}
