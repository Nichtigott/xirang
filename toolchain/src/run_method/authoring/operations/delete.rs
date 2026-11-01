//! Deleting a generated registration module into the trash.
//! 把生成的注册模块删除进回收目录。

use super::super::*;
use super::generated_paths;
use super::trash::trash_root;

/// Move one generated module subtree into the recoverable XiRang trash.
/// 将一个生成模块子树移动到可恢复的 XiRang 回收目录。
pub fn delete_module(registry: &Registry, spec: &str) -> Result<AuthoringChange, String> {
    let mut fields = spec.split_whitespace();
    let id = fields
        .next()
        .ok_or_else(|| "usage: delete <node> confirm".to_owned())?
        .parse::<NodeId>()
        .map_err(|_| "delete requires a 32-digit node identity".to_owned())?;
    if fields.next() != Some("confirm") || fields.next().is_some() {
        return Err("usage: delete <node> confirm".to_owned());
    }

    let (name, source) = generated_paths(registry, id)?;
    let module_dir = source
        .parent()
        .ok_or_else(|| "generated source has no module directory".to_owned())?;
    // The trash move takes the whole directory, so the directory has to be *this*
    // module's own: `<module>/<module>.rs`. A marker-carrying face that sits directly
    // under `src/` — a hand-made or flattened module — is listed as an ordinary
    // deletable face, and renaming its parent would take all of `src/` with it. The
    // rename path already refuses this shape for the same reason; deleting needs the
    // same guard because it moves a directory, not a file.
    // 回收搬走的是整个目录，因此该目录必须是**这个**模块自己的：`<module>/<module>.rs`。
    // 带标记却直接位于 `src/` 下的面——手写的或已被拍平的模块——会像普通面一样被列为可删除，而
    // 重命名它的父目录会把整个 `src/` 一起带走。改名路径出于同样的理由已经拒绝这种形状；删除同样
    // 需要这道守卫，因为它搬的是目录而不是文件。
    let stem = source
        .file_stem()
        .and_then(|name| name.to_str())
        .unwrap_or_default();
    if module_dir.file_name().and_then(|name| name.to_str()) != Some(stem) {
        return Err("only standard `<module>/<module>.rs` faces can be deleted".to_owned());
    }
    let trash = trash_root().join(format!("{name}-{id}-{}", super::trash::stamp()?));
    // A delete **moves a directory**: it must not move a directory whose real location is outside
    // this package, and the slot it moves into must be inside it too.
    // 删除**搬动一个目录**：它不得搬动真实位置在包外的目录，搬入的位置也必须在包内。
    {
        use crate::run_method::authoring::write_scope::{
            Change, ChangeKind, WriteScope, describe_outside,
        };
        let root = package_root();
        let changes = [
            Change {
                path: module_dir.to_path_buf(),
                kind: ChangeKind::Remove,
                subject: format!("the module directory `{}`", module_dir.display()),
            },
            Change {
                path: trash.clone(),
                kind: ChangeKind::Write,
                subject: format!("its trash entry `{}`", trash.display()),
            },
        ];
        if let Err(outside) = WriteScope::new(&root)?.authorize(&changes) {
            return Err(format!(
                "this delete would move something outside the package `{}`, so nothing was moved.\n{}",
                root.display(),
                describe_outside(&outside, &root)
            ));
        }
    }
    // The directory creation and the move are **one transaction**, the same one every other writer
    // uses: a failure after the trash directory exists puts the directory back or names what it
    // could not undo, and the move itself refuses a destination that already exists. This used to be
    // `create_dir_all` followed by `rename`, which left an empty `.xirang/trash` behind on failure and
    // had no shared rollback to report it.
    // 目录创建与搬动是**同一个事务**，也就是其它每个写入方用的那一个：回收目录已出现之后的失败会把目录
    // 撤掉、或点名它没能撤销的东西，而搬动本身拒绝已存在的目的地。这里过去是 `create_dir_all` 加
    // `rename`，失败会留下一个空的 `.xirang/trash`，且没有共享回滚来报告它。
    let mut transaction = super::super::write_transaction::WriteTransaction::new();
    if let Err(error) = transaction.create_dir(trash.parent().expect("trash has a parent")) {
        let recovery = transaction.rollback();
        return Err(format!(
            "cannot create XiRang trash: {error}\n{}",
            recovery.describe()
        ));
    }
    if let Err(error) = transaction.rename(module_dir, &trash) {
        let recovery = transaction.rollback();
        return Err(format!(
            "cannot move module to trash: {error}\n{}",
            recovery.describe()
        ));
    }

    Ok(AuthoringChange {
        message: format!("moved `{name}` to {}", trash.display()),
        source: trash,
    })
}
