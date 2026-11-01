//! What a failed creation takes back, and what it refuses to touch.
//! 一次失败的创建收回什么、又不碰什么。

use super::super::write_transaction::WriteTransaction;
use std::fs;
use std::path::PathBuf;

/// A directory no other test shares.
/// 一个没有别的测试共用的目录。
fn unique_directory(label: &str) -> PathBuf {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let directory = std::env::temp_dir().join(format!(
        "xirang-lg48-{label}-{}-{sequence}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&directory);
    directory
}

/// A file this transaction created is removed, and the report says so.
/// 本事务创建的文件会被移除，报告也说出来。
#[test]
fn a_created_file_is_taken_back() {
    let directory = unique_directory("file");
    fs::create_dir_all(&directory).expect("fixture directory");
    let file = directory.join("widget.rs");

    let mut transaction = WriteTransaction::new();
    transaction
        .create_new(&file, b"// fixture")
        .expect("create the face");
    assert!(file.exists());

    let recovery = transaction.rollback();
    let message = recovery.describe();
    assert!(!file.exists(), "the created file is gone");
    assert!(
        message.contains("were restored") || message.contains("restored"),
        "the report says what the rollback did: {message}"
    );
    assert!(!message.contains("still differ"), "{message}");
    let _ = fs::remove_dir_all(&directory);
}

/// A directory this transaction created is removed again when it is empty.
/// 本事务创建、最终为空的目录会被再次移除。
#[test]
fn a_created_empty_directory_is_taken_back() {
    let directory = unique_directory("empty");
    let leaf = directory.join("module");

    let mut transaction = WriteTransaction::new();
    transaction
        .create_dir(&leaf)
        .expect("create the module directory");
    assert!(leaf.exists());

    let message = transaction.rollback().describe();
    assert!(!leaf.exists(), "the empty directory is gone");
    assert!(!message.contains("still differ"), "{message}");
}

/// A directory holding somebody else's content survives, and the report names it — a rollback that
/// could not finish is a fact the caller has to see, not a silent success.
/// 装着别人内容的目录保留下来，而报告点它的名——没能完成的回滚是调用方必须看见的事实，不是静默的成功。
#[test]
fn a_directory_holding_someone_elses_content_survives_and_is_named() {
    let directory = unique_directory("occupied");
    let leaf = directory.join("module");

    let mut transaction = WriteTransaction::new();
    transaction
        .create_dir(&leaf)
        .expect("create the module directory");
    let keep = leaf.join("keep.rs");
    fs::write(&keep, "// not ours to remove").expect("fixture file");

    let message = transaction.rollback().describe();
    assert!(keep.exists(), "the caller's file survives");
    assert!(
        message.contains("still differ"),
        "a directory left behind is reported, not hidden: {message}"
    );
    assert!(
        message.contains(&leaf.display().to_string()),
        "the report names the path: {message}"
    );
    let _ = fs::remove_dir_all(&directory);
}

/// A rollback that really fails is reported with the path, so the caller is never told the tree is
/// clean while something is still on disk.
/// 真正失败的回滚会连同路径被报告出来，因此调用方绝不会在磁盘上还留着东西时被告知树是干净的。
///
/// Red before the fix: `create.rs` discarded these errors (`let _ = fs::remove_file(&rule);`), so the
/// caller saw only the first failure. The fixture makes the restore fail deterministically by turning
/// the captured file into a directory: `std::fs::write` refuses it whatever the permissions are.
/// 修前为红：`create.rs` 丢弃了这些错误，因此调用方只看到最初那条错误。夹具让恢复确定性地失败：把被保存
/// 的文件换成一个目录，`std::fs::write` 无论权限如何都会拒绝它。
#[test]
fn a_failed_rollback_is_named_in_the_message() {
    let directory = unique_directory("failed");
    fs::create_dir_all(&directory).expect("fixture directory");
    let file = directory.join("widget.rs");
    fs::write(&file, "// original").expect("fixture file");

    let mut transaction = WriteTransaction::new();
    transaction
        .write_atomic(&file, b"// edited")
        .expect("write the edit");
    fs::remove_file(&file).expect("clear the file");
    fs::create_dir(&file).expect("a directory where the file was");

    let message = transaction.rollback().describe();
    assert!(
        message.contains("still differ"),
        "a rollback that could not undo its work must say so: {message}"
    );
    assert!(
        message.contains(&file.display().to_string()),
        "the message names what is still on disk: {message}"
    );
    let _ = fs::remove_dir_all(&directory);
}
