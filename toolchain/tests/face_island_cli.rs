//! The CLI must edit an explicitly placed Rust face by **path** and by **NodeId**, not only MCP.
//! CLI 必须能按**路径**和**NodeId**编辑显式落点的 Rust 面，而不只是 MCP 能。
//!
//! The island is created through the same CLI (`face add --file`), then `check` publishes the tree.
//! 孤岛经同一个 CLI（`face add --file`）创建，随后 `check` 发布这棵树。

use std::path::{Path, PathBuf};
use std::process::Command;

fn checkout() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("the toolchain directory sits under the checkout")
        .to_path_buf()
}

fn work() -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "xirang-face-island-cli-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("work root");
    root
}

fn xirang(cwd: &Path, arguments: &[&str]) -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_xirang"))
        .args(arguments)
        .current_dir(cwd)
        .output()
        .unwrap_or_else(|error| panic!("run xirang {arguments:?}: {error}"));
    assert!(
        output.status.success(),
        "xirang {arguments:?} failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn node_id(explain_json: &str) -> String {
    let marker = "\"id\":\"";
    let start = explain_json
        .find(marker)
        .unwrap_or_else(|| panic!("no node id in explain output: {explain_json}"))
        + marker.len();
    let id = &explain_json[start..];
    assert!(id.len() >= 32, "short node id: {id}");
    id[..32].to_owned()
}

#[test]
fn cli_edits_an_explicit_island_by_path_and_by_node_id() {
    let work = work();
    let host = work.join("host");
    xirang(
        &work,
        &[
            "new",
            "host",
            "--lib",
            "--path",
            &checkout().to_string_lossy(),
        ],
    );
    xirang(
        &work,
        &[
            "face",
            "add",
            "panel",
            "--lang",
            "rust",
            "--project",
            "host",
            "--fields",
            r#"{"needs_registry":true}"#,
            "--write",
        ],
    );
    xirang(&host, &["check"]);
    let panel = node_id(&xirang(&host, &["explain", "root/panel", "--json"]));
    xirang(
        &work,
        &[
            "face",
            "add",
            "reading",
            "--lang",
            "rust",
            "--parent",
            &panel,
            "--file",
            "components/reading/reading.rs",
            "--project",
            "host",
            "--fields",
            r#"{"needs_registry":true}"#,
            "--write",
        ],
    );
    xirang(&host, &["check"]);
    let reading = node_id(&xirang(&host, &["explain", "root/panel/reading", "--json"]));
    let source = host.join("components/reading/reading.rs");
    assert!(
        source.is_file(),
        "the island exists at {}",
        source.display()
    );

    xirang(
        &work,
        &[
            "face",
            "edit",
            "components/reading/reading.rs",
            "--lang",
            "rust",
            "--project",
            "host",
            "--fields",
            r#"{"summary_en":"Edited by path","summary_zh":"按路径编辑"}"#,
            "--write",
        ],
    );
    let text = std::fs::read_to_string(&source).expect("the island source");
    assert!(
        text.contains("Edited by path"),
        "path edit did not land: {text}"
    );
    assert!(
        text.contains("按路径编辑"),
        "path edit did not land: {text}"
    );

    xirang(
        &work,
        &[
            "face",
            "edit",
            &reading,
            "--lang",
            "rust",
            "--project",
            "host",
            "--fields",
            r#"{"summary_en":"Edited by node","summary_zh":"按节点编辑"}"#,
            "--write",
        ],
    );
    let text = std::fs::read_to_string(&source).expect("the island source");
    assert!(
        text.contains("Edited by node"),
        "node-id edit did not land: {text}"
    );
    assert!(
        text.contains("按节点编辑"),
        "node-id edit did not land: {text}"
    );

    xirang(&host, &["check"]);
    let _ = std::fs::remove_dir_all(&work);
}
