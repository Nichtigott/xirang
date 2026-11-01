//! Documentation must not acquire execution semantics.
//! 文档读取不得获得项目执行语义。

use super::super::{run_tool, tools_tests::scratch_package};
use serde_json::json;

#[test]
fn gateway_manual_is_never_project_execution() {
    let root = scratch_package("manual-only");
    for arguments in [
        json!({}),
        json!({"tool":"verify"}),
        json!({"tool":"new_project"}),
    ] {
        let reply = run_tool(&root, "xirang_tools", &arguments).unwrap();
        assert!(
            reply.starts_with("documentation only; no project operation executed"),
            "{reply}"
        );
        assert!(
            !reply.contains("→ exit 0"),
            "manual must not supply execution evidence: {reply}"
        );
        assert!(
            !root.join(".xirang").exists(),
            "manual must not publish build records"
        );
    }
    let read = run_tool(&root, "xirang_tools", &json!({"tool":"read"})).unwrap();
    assert!(
        read.contains("arguments=<object matching inputSchema>"),
        "{read}"
    );
    let executed = run_tool(
        &root,
        "xirang_tools",
        &json!({"tool":"read", "arguments":{"path":"src/lib.rs"}}),
    )
    .unwrap();
    assert!(executed.contains("evidence xirang.read"), "{executed}");
    assert!(!executed.contains("documentation only"), "{executed}");
    let _ = std::fs::remove_dir_all(root);
}
