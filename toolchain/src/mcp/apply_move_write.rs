//! The move write transaction: stage every byte, rename, then write, with rollback.
//! 搬动写入事务：先备好每一份字节，改名，再写入，失败则回滚。
//!
//! Split out of `apply_move.rs` to keep that module inside the repository's size
//! ratchet. The sibling `references` planner supplies the rewritten bytes; this file
//! binds them to disk and never claims crash atomicity — the durable journal is W4's.
//! 从 `apply_move.rs` 拆出，使那个模块留在仓库的尺寸棘轮之内。同级的 `references` 规划器提供改写后的
//! 字节；本文件把它们绑定到磁盘，且绝不声称崩溃原子性——持久 journal 是 W4 的工作。

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use super::{ChildEdit, Moved, ParentEdit, references, value_of};
use crate::run_method::authoring::write_transaction::WriteTransaction;

/// Rename the subtree and edit its declarations, rolling back if the edits fail.
/// 给子树改名并编辑声明；编辑失败时回滚。
///
/// The contract extends to the rewritten references **and to everything the publish step writes**
/// (record, graft records, host-entry declarations): those go through the same [`WriteTransaction`],
/// so a later failure restores them rather than leaving a tree that keeps a record for a move it did
/// not make. Every plan is re-bound to the bytes on disk before anything changes, and a changed file
/// refuses the whole move with `needs re-plan`. That is a minimum precondition, not crash atomicity —
/// the durable journal is W4's work.
/// 该契约延伸到被改写的引用，**也延伸到发布步骤所写的一切**（记录、嫁接记录、宿主入口声明）：它们走
/// 同一个 [`WriteTransaction`]，因此后来的失败会把它们恢复回去，而不是留下一棵"为没发生的搬动留了记录"
/// 的树。任何改动发生前，每份计划都重新绑定到磁盘上的字节，变化的文件以 `needs re-plan` 拒绝整次搬动。
/// 这是最小前提，不是崩溃原子性——持久 journal 是 W4 的工作。
#[allow(clippy::too_many_arguments)]
pub(super) fn apply_move(
    package_root: &Path,
    identity_base: &Path,
    moved: &Moved,
    parent_edit: &ParentEdit,
    alias: Option<&str>,
    children: &[ChildEdit],
    reference_plans: &[references::FilePlan],
    publish_paths: &[(
        PathBuf,
        crate::run_method::authoring::write_scope::ChangeKind,
        String,
    )],
    publish: impl FnOnce(&mut WriteTransaction) -> Result<(), String>,
) -> Result<(), String> {
    // Prepare every byte before renaming. A failed write restores earlier edits as well
    // as the directory name; renaming a directory alone does not undo changed contents.
    // 改名之前准备全部字节。写入失败时同时恢复内容与目录名，不能只改回目录名。
    // **Filesystem facts are package-relative, both ends.** The record spellings (`from_dir`,
    // `to_dir`, `source`) are identity inputs; joining those to one base was how a cross-base move
    // put the file at the package root while reporting the identity-base join.
    // **文件系统事实两端都是包内相对。** 记录拼写（`from_dir`、`to_dir`、`source`）是身份输入；把它们
    // 拼到单一基准上，正是跨基准搬动把文件放到包根、却按身份基准拼接来报告的原因。
    let _ = identity_base;
    let base = package_root;
    let from = base.join(&moved.from_package);
    let to = base.join(&moved.to_package);
    let old_file = Path::new(&moved.original_source).file_name().unwrap();
    let relocated_file = to.join(old_file);
    let final_file = base.join(&moved.to_physical);
    // The destination must not exist at all: `rename(2)` replaces an **empty** target directory,
    // which would silently merge the move into a directory that was already there and would leave
    // nothing for the rollback to recreate. The old check looked inside the *source* directory, so it
    // never saw this.
    // 目的地必须完全不存在：`rename(2)` 会替换一个**空**目标目录，从而把这次搬动静默并进一个本已存在的
    // 目录，而回滚没有任何东西可以重建。旧检查看的是**源**目录里面，因此从来没看见这一点。
    if from != to && to.exists() {
        return Err(format!(
            "{} already exists; a move does not merge into a directory that is already there. \
             Nothing changed.",
            to.display()
        ));
    }
    if relocated_file != final_file && from.join(final_file.file_name().unwrap()).exists() {
        return Err(format!(
            "{} already exists in the moved subtree",
            final_file.display()
        ));
    }
    // Bind every reference plan to the bytes it was computed from. The pair is
    // `(original-for-rollback, edited)`.
    let mut planned: BTreeMap<PathBuf, (String, String)> = BTreeMap::new();
    for plan in reference_plans {
        let current = std::fs::read_to_string(&plan.file)
            .map_err(|error| format!("{} is not readable: {error}", plan.file.display()))?;
        let edited = references::bound_text(plan, &current)?;
        planned.insert(plan.file.clone(), (current, edited));
    }
    let face_original = from.join(old_file);
    let raw = std::fs::read_to_string(&face_original)
        .map_err(|error| format!("{} is not readable: {error}", face_original.display()))?;
    let bound = planned
        .remove(&face_original)
        .map_or_else(|| raw.clone(), |(_, edited)| edited);
    let edited = relocated_declaration(&bound, &parent_edit.before, &parent_edit.after, alias)?;
    let mut edits = vec![(relocated_file.clone(), raw, edited)];
    for child in children {
        // Both ends are package-relative facts from the plan; a logical-only descendant keeps its
        // file and only its declaration changes.
        // 两端都是计划给出的包内相对事实；只发生逻辑搬动的后代文件不动，只有声明改变。
        let original = base.join(&child.from_physical);
        let target = base.join(&child.to_physical);
        let raw = std::fs::read_to_string(&original)
            .map_err(|error| format!("{} is not readable: {error}", original.display()))?;
        let bound = planned
            .remove(&original)
            .map_or_else(|| raw.clone(), |(_, edited)| edited);
        let edited =
            relocated_declaration(&bound, &child.before, &child.after, child.alias.as_deref())?;
        edits.push((target, raw, edited));
    }
    // Reference files outside the moved subtree, and subtree files that carry no
    // declaration edit, are written where the rename leaves them.
    for (file, (raw, edited)) in planned {
        let target = file
            .strip_prefix(&from)
            .map_or_else(|_| file.clone(), |rest| to.join(rest));
        edits.push((target, raw, edited));
    }
    // **The complete set is authorized before anything is created.** A Rust move derives its
    // destination from a module path, and a directory on that path may be a link out of the package:
    // without this pass the rename would be the first thing that noticed. The set is the source it
    // removes, the destination it creates, and every file the move rewrites.
    // **在创建任何东西之前授权完整集合。** Rust 搬动从模块路径推导目的地，而那条路径上的某个目录可能是
    // 通向包外的链接：没有这一趟，改名会是第一个发现它的动作。集合就是它移除的源、它创建的目的地，以及这次
    // 搬动改写的每一份文件。
    {
        use crate::run_method::authoring::write_scope::{Change, ChangeKind, WriteScope};
        let scope = WriteScope::new(package_root)?;
        let mut changes = vec![
            Change {
                path: from.clone(),
                kind: ChangeKind::Remove,
                subject: format!("the subtree `{}`", moved.from_dir),
            },
            Change {
                path: to.clone(),
                kind: ChangeKind::Write,
                subject: format!("the subtree's destination `{}`", moved.to_dir),
            },
        ];
        for (file, _, _) in &edits {
            changes.push(Change {
                path: file.clone(),
                kind: ChangeKind::Write,
                subject: format!("the reference in `{}`", file.display()),
            });
        }
        // Publication is part of the same write set: the migration record, the graft records, the
        // host-entry declarations and the ledger are written inside this transaction, so they are
        // authorized here — before the rename, not after it.
        // 发布属于同一个写入集合：迁移记录、嫁接记录、宿主入口声明与台账都写在这个事务里，因此在这里——
        // 在改名之前，而不是之后——被授权。
        for (path, kind, subject) in publish_paths {
            changes.push(Change {
                path: path.clone(),
                kind: *kind,
                subject: subject.clone(),
            });
        }
        if let Err(outside) = scope.authorize(&changes) {
            return Err(format!(
                "move `{}` would have to write outside the package `{}`, so nothing was written.\n{}",
                moved.source,
                package_root.display(),
                crate::run_method::authoring::write_scope::describe_outside(&outside, package_root)
            ));
        }
    }
    // One transaction for this move's whole write set — including the directories it creates and
    // the rename that **is** the move. The subtree edits are captured here; the publish step adds
    // the record, graft records and host-entry declarations to the same set.
    // 这次搬动整个写入集合用同一个事务——包括它创建的目录与**就是这次搬动**的那次改名。子树编辑在这里
    // 保存；发布步骤把记录、嫁接记录与宿主入口声明加进同一个集合。
    let mut transaction = WriteTransaction::new();
    let result = (|| -> Result<(), String> {
        if let Some(parent) = to.parent() {
            transaction.create_dir(parent)?;
        }
        transaction.rename(&from, &to)?;
        for (file, original, edited) in &edits {
            // Re-check the bytes this edit was planned from: the rename moved the files, so the
            // original bytes must still be the ones the plan was bound to. A file that changed in
            // between is refused with `needs re-plan` instead of overwritten.
            // 复核这次编辑所依据的字节：改名搬动了文件，因此原始字节必须仍是计划所绑定的那一份。中途变过的
            // 文件以 `needs re-plan` 拒绝，而不是被覆盖。
            let current = std::fs::read_to_string(file)
                .map_err(|error| format!("{} is not readable: {error}", file.display()))?;
            if &current != original {
                return Err(format!(
                    "{} changed between planning and writing; the move needs re-plan",
                    file.display()
                ));
            }
            transaction.write(file, edited.as_bytes())?;
        }
        if relocated_file != final_file {
            transaction.rename(&relocated_file, &final_file)?;
        }
        publish(&mut transaction)
    })();
    if let Err(mut error) = result {
        // The rollback restores the edited bytes, renames the directories back and removes the
        // directories this transaction created; whatever it could not restore is named in the
        // error instead of being left for the next reader to discover.
        // 回滚恢复被编辑的字节、把目录改回名字、删掉本事务创建的目录；没能恢复的部分在错误里点名，
        // 而不是留给下一个读者去发现。
        let recovery = transaction.rollback();
        error.push_str(&format!("; {}", recovery.describe()));
        return Err(error);
    }
    Ok(())
}

/// Edit native declaration spans, checking that the planned parent still matches.
/// 编辑原生声明的语法区间，并核对规划时的父表达式仍然一致。
fn relocated_declaration(
    text: &str,
    before: &str,
    after: &str,
    alias: Option<&str>,
) -> Result<String, String> {
    let face = xirang_kernel::syntax::parse_face(text)
        .map_err(|error| error.to_string())?
        .ok_or("source has no registration face")?;
    let planned_parent = value_of(before)
        .parse::<proc_macro2::TokenStream>()
        .map(xirang_kernel::syntax::compact_tokens)
        .map_err(|error| format!("invalid planned parent: {error}"))?;
    let planned_after = value_of(after)
        .parse::<proc_macro2::TokenStream>()
        .map(xirang_kernel::syntax::compact_tokens)
        .map_err(|error| format!("invalid planned parent: {error}"))?;
    // The reference rewriter may have already followed the same module path in
    // this file (a moved face's `parent:` is an expression path too). Seeing the
    // planned destination — in the declaration's catalog spelling or the direct
    // one the reference rewriter emits — is that pass having done this edit, not a
    // stale plan.
    let direct_after = planned_after.replacen("crate::catalog::", "crate::", 1);
    match face.field("parent").as_deref() {
        Some(current)
            if current == planned_parent || current == planned_after || current == direct_after => {
        }
        other => {
            return Err(format!(
                "the planned parent changed before the move: found {other:?}, planned \
                 {planned_parent:?} -> {planned_after:?}"
            ));
        }
    }
    let alias = alias
        .and_then(|alias| alias.split_once(" → "))
        .map(|(before, after)| (before.trim_end_matches('!'), after.trim_end_matches('!')));
    xirang_kernel::syntax::relocate_face_declaration(text, &value_of(after), alias)
        .map_err(|error| error.to_string())
}
