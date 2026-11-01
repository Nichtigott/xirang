//! The recoverable trash authoring writes into.
//! 创作写入的可恢复回收目录。
//!
//! Deleting a face already moved its module here, which made deletion the only
//! reversible destructive operation. A rewrite is destructive in the same way and
//! now leaves its previous text here too: the editor rebuilds a face from the
//! fields it models, so anything hand-added to the file is not in the result.
//! 删除注册面本就把模块移到这里，这使删除成为唯一可逆的破坏性操作。重写以同样的方式具有
//! 破坏性，现在也把先前的文本留在这里：编辑器会用它建模的字段重建注册面，因此手工加进
//! 文件的内容不在结果里。

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use super::super::*;

/// The directory every recoverable copy lands under.
/// 每个可恢复副本所落的目录。
pub(super) fn trash_root() -> PathBuf {
    package_root()
        .join(xirang_kernel::lexicon::XIRANG_DIR)
        .join("trash")
}

/// A unique-enough suffix for one trash entry.
/// 一个回收条目的足够唯一的后缀。
///
/// Nanoseconds rather than seconds: one editing session can save the same face
/// twice inside a second, and the second save must not overwrite the first
/// backup.
/// 用纳秒而不是秒：一次编辑会话可能在同一秒内保存同一个注册面两次，而第二次保存绝不能
/// 覆盖第一次的备份。
pub(super) fn stamp() -> Result<u128, String> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_nanos())
        .map_err(|error| format!("clock error: {error}"))
}

/// Keep one face file's previous text before it is rewritten.
/// 在一个注册面文件被重写之前，留下它先前的文本。
///
/// The name comes from the file rather than from a modeled field, because a
/// legacy or minimal face carries no `module` value to borrow. The path is
/// returned so the caller can tell the reader where the old text went; a backup
/// nobody can find is not a way back.
/// 名字取自文件而不是某个建模字段，因为旧式或最小注册面没有 `module` 取值可借。返回路径，
/// 使调用方能告诉读者旧文本去了哪里；找不到的备份不算回退手段。
pub(super) fn stash_face_source(
    source: &Path,
    id: NodeId,
    previous: &str,
) -> Result<PathBuf, String> {
    let name = source
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("face");
    let directory = trash_root().join("faces");
    {
        // The stash is a write like any other: it must land inside the package.
        // 这份留存与任何写入一样：必须落在包内。
        use crate::run_method::authoring::write_scope::{Change, ChangeKind, WriteScope};
        let root = package_root();
        let changes = [Change {
            path: directory.clone(),
            kind: ChangeKind::Write,
            subject: format!("the previous text at `{}`", directory.display()),
        }];
        if let Err(outside) = WriteScope::new(&root)?.authorize(&changes) {
            return Err(format!(
                "keeping the previous bytes would write outside the package `{}`: {}",
                root.display(),
                crate::run_method::authoring::write_scope::describe_outside(&outside, &root)
            ));
        }
    }
    fs::create_dir_all(&directory)
        .map_err(|error| format!("cannot create XiRang trash: {error}"))?;
    let path = directory.join(format!("{name}-{id}-{}.rs", stamp()?));
    fs::write(&path, previous).map_err(|error| {
        format!(
            "cannot keep the previous text at {} before rewriting: {error}",
            path.display()
        )
    })?;
    Ok(path)
}
