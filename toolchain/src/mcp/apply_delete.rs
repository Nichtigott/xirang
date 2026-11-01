//! The `delete` verb: one face's module subtree moved into the trash.
//! `delete` 动词：把一个面的模块子树移入回收目录。
//!
//! Split from `apply` because the verbs and the rule-plan premise are separate subjects, and because
//! both files have a size budget. Behavior is unchanged: the same registry, the same executor, the
//! same refusal that a delete needs `confirm: true`.
//! 从 `apply` 拆出，因为"动词"与"规则计划前提"是两件事，而且两个文件都有尺寸预算。行为不变：同一棵
//! 注册树、同一个执行器、同一条"删除需要 `confirm: true`"的拒绝。

use std::path::Path;

use serde_json::Value;

use super::apply_refusals::write_example;
use super::{Action, Outcome, load_registry};
use crate::mcp::resolve::resolve_node;
use crate::run_method::AuthoringContext;

/// Move a face's module subtree into the trash.
/// 把一个面的模块子树移入回收目录。
///
/// Deletion is recoverable by design — the executor moves the directory rather than
/// unlinking it — and the request has to say `confirm` for the same reason the CLI
/// does: a delete is the one operation whose preview a caller can skip past by
/// accident.
/// 删除按设计是可恢复的——执行器搬走目录而不是删掉它——而请求必须像 CLI 一样说出 `confirm`：
/// 删除是唯一一种调用方可能不小心跳过其预览的操作。
pub(super) fn run_delete(
    root: &Path,
    namespace: &str,
    arguments: &Value,
) -> Result<Outcome, String> {
    let target = arguments
        .get("node")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            format!(
                "delete requires `node`: a logical path or a 32-digit identity (a delete previews \
                 first; the confirmation is what writes)\n{}",
                write_example(root, namespace, Action::Delete)
            )
        })?;
    // The request says `confirm` itself. This used to be appended right here, which made the
    // sentence above — and the declared schema, which had no such key at all — describe something
    // the bridge did not do: a caller could leave it out and the delete still went through
    // (audit `m5`).
    // 请求自己说出 `confirm`。过去是在这里就地拼上去的，于是上面那句话——以及根本没有这个键的声明
    // schema——描述的是桥并不做的事：调用方可以省掉它，删除照样执行（审计 `m5`）。
    if arguments.get("confirm").and_then(Value::as_bool) != Some(true) {
        return Err(format!(
            "delete requires `confirm: true`: a delete is the one operation whose preview a caller \
             can step past by accident\n{}",
            write_example(root, namespace, Action::Delete)
        ));
    }
    let registry = load_registry(root, namespace)?;
    let id = resolve_node(root, namespace, target)?;
    let change = AuthoringContext::new(root.to_path_buf(), namespace.to_owned())
        .scope(|| crate::run_method::delete_module(&registry, &format!("{id} confirm")))?;
    Ok(Outcome {
        message: change.message,
        source: change.source,
        declaration: None,
        moved: true,
        alternative: false,
        inherited: Vec::new(),
        consumers: Vec::new(),
    })
}

/// The delete verb plus the knowledge it strands, so the caller's dispatch arm stays one call.
/// delete 动词外加它留下的知识，使调用方的分派臂保持一次调用。
pub(super) fn run_with_stranded(
    root: &Path,
    work: &Path,
    namespace: &str,
    arguments: &Value,
) -> Result<(super::Outcome, Vec<String>), String> {
    // The lineage is read **before** the object is gone: its rows live in the project's own state
    // (`.xirang` is never copied into a preview), while "is this anchor still a face" has to be asked
    // of a tree that still holds the template. Reading it after the delete would make the write tell
    // a different story than the preview.
    // 谱系在对象消失**之前**读出：它的行住在项目自己的状态里（`.xirang` 从不被复制进预览），而"这个锚点
    // 还是不是面"必须问一棵仍持有该模板的树。在删除之后读，会让落盘讲一个与预览不同的故事。
    let lineage = match arguments.get("node").and_then(Value::as_str) {
        Some(node) => match removed_path(work, namespace, node) {
            Some(path) => crate::mcp::apply::apply_template::plan_removal(work, root, &path)?,
            None => Vec::new(),
        },
        None => Vec::new(),
    };
    let outcome = run_delete(work, namespace, arguments)?;
    let mut stranded = match arguments.get("node").and_then(Value::as_str) {
        Some(node) => crate::mcp::rules::plan_removal(root, work, node)?,
        None => Vec::new(),
    };
    stranded.extend(lineage);
    Ok((outcome, stranded))
}

/// The logical path of the face a delete names, resolved **before** it is gone.
/// 删除点名的面在它消失**之前**的逻辑路径。
///
/// A delete accepts a path, a module spelling or a 32-digit identity; a lineage row only ever spells
/// the logical path, so the one spelling both halves share has to come from the tree while the object
/// is still in it.
/// 删除接受路径、模块拼法或 32 位身份；谱系行只会写逻辑路径，因此两半共用的那个拼法必须在对象仍在树里时
/// 从树里取。
fn removed_path(work: &Path, namespace: &str, node: &str) -> Option<String> {
    crate::build_method::face_views(work, namespace)
        .ok()?
        .into_iter()
        .find(|view| view.path == node || view.module == node || view.id.to_string() == node)
        .map(|view| view.path)
}
