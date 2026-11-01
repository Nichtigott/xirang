//! Real stable-compiler MIR must round-trip and reject edits after capture.
//! 真实稳定版编译器 MIR 必须可往返，且拒绝采集后的修改。
use super::*;
#[test]
fn stable_mir_carries_identity_and_never_reuses_edited_sources() {
    let root = std::env::temp_dir().join(format!(
        "xirang-capture-{}",
        crate::mcp::check_jobs::new_id()
    ));
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::write(
        root.join("Cargo.toml"),
        "[package]\nname='capture-probe'\nversion='0.0.0'\nedition='2024'\n[workspace]\n",
    )
    .unwrap();
    std::fs::write(
        root.join("src/lib.rs"),
        "pub fn provider() -> u32 { 7 }\npub fn consumer() -> u32 { provider() }\n",
    )
    .unwrap();
    assert!(!root.join("Cargo.lock").exists());
    let reply = capture(&root, &json!({"capture":true})).unwrap();
    assert!(reply.starts_with("verdict passed"), "{reply}");
    assert!(root.join("Cargo.lock").is_file());
    let path = reply
        .lines()
        .find_map(|line| line.strip_prefix("path "))
        .unwrap();
    let readback = crate::mcp::mir::mir(&root, &json!({"path":path})).unwrap();
    assert!(readback.contains("consumer -> provider"), "{readback}");
    std::fs::write(root.join("src/lib.rs"), "pub fn provider() -> u32 { 9 }\n").unwrap();
    assert!(
        crate::mcp::mir::mir(&root, &json!({"path":path}))
            .unwrap_err()
            .contains("stale or changed")
    );
    std::fs::remove_dir_all(root).unwrap();
}
