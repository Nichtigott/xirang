//! Cursor pages remain bounded, complete and confined to the requested root.
//! 游标分页有界、完整且限制在请求根内。
use super::*;
use serde_json::json;
#[test]
fn directories_and_unicode_byte_pages_can_be_consumed_without_loss() {
    let root =
        std::env::temp_dir().join(format!("xirang-nav-{}", crate::mcp::check_jobs::new_id()));
    std::fs::create_dir_all(root.join("sub")).unwrap();
    let source = "中文abc".repeat(1000);
    std::fs::write(root.join("huge.log"), &source).unwrap();
    let first = read(&root, &json!({"path":".","limit":1}))
        .unwrap()
        .unwrap();
    assert!(first.contains("next offset 1"), "{first}");
    let second = read(&root, &json!({"path":".","limit":1,"offset":1}))
        .unwrap()
        .unwrap();
    assert!(second.contains("sub/"));
    let mut collected = String::new();
    let mut offset = 0;
    while offset < source.len() {
        let reply = read(
            &root,
            &json!({"path":"huge.log","offset_bytes":offset,"max_bytes":100}),
        )
        .unwrap()
        .unwrap();
        let (header, body) = reply.split_once('\n').unwrap();
        offset = header.rsplit_once(' ').unwrap().1.parse().unwrap();
        assert!(body.len() <= 100);
        collected.push_str(body);
    }
    assert_eq!(source, collected);
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink("/etc", root.join("outside")).unwrap();
        assert!(read(&root, &json!({"path":"outside"})).is_err());
        assert!(
            !read(&root, &json!({"path":"."}))
                .unwrap()
                .unwrap()
                .contains("outside")
        );
    }
    std::fs::remove_dir_all(root).unwrap();
}
