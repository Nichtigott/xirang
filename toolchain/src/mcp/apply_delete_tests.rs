//! A delete's failure path: the trash directory cannot be created, and the tree it would have
//! changed must come back byte for byte.
//! 删除的失败路径：回收目录建不出来，而它本该改动的那棵树必须逐字节复原。

use super::*;

/// Delete used to be `create_dir_all` followed by `rename`, so a failure left an empty `.xirang/trash`
/// and reported nothing it had not done. It is now the shared transaction: the directory creation and
/// the move are one unit, and a failure names what it could not undo instead of claiming success.
/// 删除过去是 `create_dir_all` 加 `rename`，失败会留下一个空的 `.xirang/trash`，且不报告它没做的事。
/// 现在它走共享事务：目录创建与搬动是一个单位，失败会点名它没能撤销的东西，而不是声称成功。
#[test]
fn a_delete_that_cannot_create_trash_leaves_the_tree_untouched() {
    let (root, _name) = package("delete-trash");
    apply(
        &root,
        &json!({
            "action": "add",
            "parent": "root",
            "apply": true,
            "fields": {"module": "button", "kind": "Button", "name_en": "Button"},
        }),
    )
    .expect("the face is created");
    // `.xirang` itself is a directory (the bridge reads its bindings), and a **file** named `trash`
    // sits where the trash directory would be created: that is the failure the transaction absorbs.
    // `.xirang` 本身是目录（桥要读它的绑定），而一个名为 `trash` 的**文件**挡在回收目录该创建的位置：
    // 那正是事务要吸收的失败。
    std::fs::create_dir_all(root.join(".xirang")).expect("the bridge directory");
    std::fs::write(root.join(".xirang/trash"), "// not a directory").expect("the blocker");
    let face = root.join("src/button/button.rs");
    let before = std::fs::read_to_string(&face).expect("the face");

    let error = apply(
        &root,
        &json!({"action": "delete", "node": "root/button", "confirm": true, "apply": true}),
    )
    .expect_err("the trash cannot be created");
    assert!(error.contains("cannot move module to trash"), "{error}");
    assert!(
        error.contains("were restored") || error.contains("could not"),
        "the failure carries the transaction's own report: {error}"
    );
    assert!(face.is_file(), "the module is still there");
    assert_eq!(
        std::fs::read_to_string(&face).expect("the face"),
        before,
        "its bytes are untouched"
    );
    assert_eq!(
        std::fs::read_to_string(root.join(".xirang/trash")).expect("the blocker"),
        "// not a directory",
        "the blocker is untouched too"
    );
    let _ = std::fs::remove_dir_all(&root);
}
