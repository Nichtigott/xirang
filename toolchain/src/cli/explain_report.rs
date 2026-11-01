//! The per-node `explain` report shape: identity, scope, pruning, grafts.
//! 逐节点 `explain` 报告形状：身份、作用域、修剪、嫁接。
//!
//! Split decision: this is the answer to "what is this one node and why is it
//! (not) shipped". It is separate from the `--overlay` projection, which walks
//! every slot at once, so a change to one shape cannot silently rewrite the
//! other; the two meet only in `super::json`.
//! 拆分决定：本页回答“这一个节点是什么、为何（不）发布”。它与 `--overlay` 投影分开，
//! 后者一次遍历所有槽位，因此改一种形状不会悄悄改写另一种；两者只在 `super::json`
//! 相遇。

use std::path::Path;

use crate::build_method::{
    DeclaredGraft, FaceView, declared_grafts, read_build_scope, read_pruning_manifest,
};
use serde_json::{Value, json};
use xirang_kernel::identity::NodeId;

/// Report whether the build's published scope keeps the face, and why.
/// 报告构建发布的作用域是否保留该面，以及原因。
///
/// `selected` is "this face is a scope root" (a declared slot); `kept` is "this
/// face is in the published tree", which also holds for an ancestor a selected
/// face needs. Reporting only `selected` would call a surviving parent face
/// pruned.
/// `selected` 表示"该面是作用域根"（已声明槽位）；`kept` 表示"该面在发布树中"，
/// 被选中面所需的祖先同样成立。只报 `selected` 会把仍然存活的父面说成被剪掉。
pub(super) fn scope_report(out_dir: &Path, face: &FaceView, current: bool) -> (Value, Vec<String>) {
    // Output that does not describe the current sources is reported exactly like
    // output that is missing: `known: false` and the same "run `check`" note. The
    // command used to serve the previous build's scope as `known: true`, so the
    // same tree gave two different answers depending on whether a `check` had
    // happened to run in between — and output left behind by a *failed* check was
    // served the same way.
    // 不描述当前源码的产物与被当作缺失的产物一样上报：`known: false` 与同一条"跑 `check`"提示。
    // 本命令过去会把上一次构建的作用域当作 `known: true` 提供，于是同一棵树会因期间是否恰好跑过
    // `check` 而给出两个不同答案——而**失败**的 check 留下的产物也是这样被提供的。
    if !current {
        return (
            json!({
                "known": false,
                "mode": Value::Null,
                "all": Value::Null,
                "reason": Value::Null,
                "selected": Value::Null,
                "kept": Value::Null,
                "error": "build output is out of date with the sources",
            }),
            vec![
                "  scope: unknown (build output is out of date with the sources); \
                 run `xirang check` to publish source_scope.tsv"
                    .to_owned(),
            ],
        );
    }
    match read_build_scope(out_dir) {
        Ok(scope) => {
            let selected = scope.all
                || scope.selected_sources.contains(&face.source)
                || scope.selected_ids.contains(&face.id);
            let kept = selected || scope.keeps(&face.module);
            let reason = scope.reason.clone();
            let report = json!({
                "known": true,
                "mode": scope.mode,
                "all": scope.all,
                "reason": reason,
                "selected": selected,
                "kept": kept,
            });
            let why = if selected {
                "declared slot"
            } else {
                "kept because a selected face needs it"
            };
            let note = if kept {
                format!(
                    "  scope: KEPT ({why}; mode={}, reason={})",
                    scope.mode,
                    scope.reason.as_deref().unwrap_or("<none>")
                )
            } else {
                format!(
                    "  scope: PRUNED (mode={}, reason={}); the build-time scope does not include this face",
                    scope.mode,
                    scope.reason.as_deref().unwrap_or("<none>")
                )
            };
            (report, vec![note])
        }
        Err(error) => (
            json!({
                "known": false,
                "mode": Value::Null,
                "all": Value::Null,
                "reason": Value::Null,
                "selected": Value::Null,
                "kept": Value::Null,
                "error": error,
            }),
            vec![format!(
                "  scope: unknown ({error}); run `xirang check` to publish source_scope.tsv"
            )],
        ),
    }
}

/// Report the pruning symbols the build published for the face.
/// 报告构建为该面发布的修剪符号。
pub(super) fn pruning_report(
    out_dir: &Path,
    face: &FaceView,
    current: bool,
) -> (Value, Vec<String>) {
    if !current {
        return (
            json!({
                "known": false,
                "pruned": Value::Null,
                "symbols": Value::Null,
                "error": "build output is out of date with the sources",
            }),
            vec![
                "  pruning: unknown (build output is out of date with the sources); \
                 run `xirang check` to publish pruning_manifest.tsv"
                    .to_owned(),
            ],
        );
    }
    match read_pruning_manifest(out_dir) {
        Ok(rows) => {
            let mut symbols = rows
                .into_iter()
                .filter(|row| row.source == face.source || row.id == face.id)
                .map(|row| row.symbol)
                .collect::<Vec<_>>();
            symbols.sort();
            symbols.dedup();
            let pruned = symbols.iter().any(|symbol| symbol != "-");
            let report = json!({
                "known": true,
                "pruned": pruned,
                "symbols": symbols,
            });
            let note = if symbols.is_empty() {
                "  pruning: no symbols published for this face".to_owned()
            } else {
                format!(
                    "  pruning: {} [{}]",
                    if pruned {
                        "release-time pruning strips symbols"
                    } else {
                        "nothing to strip"
                    },
                    symbols.join(", ")
                )
            };
            (report, vec![note])
        }
        Err(error) => (
            json!({
                "known": false,
                "pruned": Value::Null,
                "symbols": Value::Null,
                "error": error,
            }),
            vec![format!(
                "  pruning: unknown ({error}); run `xirang check` to publish pruning_manifest.tsv"
            )],
        ),
    }
}

/// Report the declared graft cuts that name the face.
/// 报告命名该面的已声明 graft 切口。
pub(super) fn declared_report(root: &Path, face: &FaceView) -> (Value, Vec<String>) {
    match declared_grafts(root) {
        Ok(declared) => {
            let naming = declared
                .cuts
                .iter()
                .filter(|cut| cut.names_face(&face.path, Some(&face.module)))
                .collect::<Vec<_>>();
            let cuts = naming.iter().map(|cut| graft_json(cut)).collect::<Vec<_>>();
            let mut note = vec![if naming.is_empty() {
                "  declared grafts: none (no static_graft_plan! cut names this face)".to_owned()
            } else {
                format!(
                    "  declared grafts: {} (entry {})",
                    naming.len(),
                    declared.entry.display()
                )
            }];
            for cut in &naming {
                note.push(format!(
                    "    - cut={} graft={} full={} line={} form={}",
                    cut_endpoint(cut),
                    cut.graft,
                    cut.full,
                    cut.line,
                    cut_form(cut)
                ));
            }
            (
                json!({
                    "entry": declared.entry.display().to_string(),
                    "count": naming.len(),
                    "cuts": cuts,
                }),
                note,
            )
        }
        Err(error) => (
            json!({
                "entry": Value::Null,
                "count": 0,
                "cuts": [],
                "error": error,
            }),
            vec![format!(
                "  declared grafts: cannot read the host entry ({error})"
            )],
        ),
    }
}

/// The logical path of a node the current tree knows, if any.
/// 当前树认识的某个节点的逻辑路径（若有）。
pub(super) fn parent_path(faces: &[FaceView], id: NodeId) -> String {
    faces
        .iter()
        .find(|face| face.id == id)
        .map(|face| face.path.clone())
        .unwrap_or_else(|| "root".to_owned())
}

/// One declared cut as JSON, for the per-node report.
/// 单条已声明切口的 JSON，用于逐节点报告。
fn graft_json(cut: &DeclaredGraft) -> Value {
    json!({
        "cut": cut_endpoint(cut),
        "cut_end": cut.cut_end,
        "graft": cut.graft,
        "full": cut.full,
        "cfg": cut.cfg,
        "line": cut.line,
        "form": cut_form(cut),
    })
}

/// A cut's endpoints rendered as one string, the way the operator wrote it.
/// 把切口的端点渲染成一个字符串，与操作者写下的形式一致。
///
/// The rule lives on `DeclaredGraft` itself, because the build writes the same
/// text into `graft_plan.tsv`; this command only prints what that rule returns.
/// 规则住在 `DeclaredGraft` 上，因为构建把同一段文本写进 `graft_plan.tsv`；本命令只打印
/// 那条规则返回的内容。
pub(super) fn cut_endpoint(cut: &DeclaredGraft) -> String {
    cut.cut_label()
}

/// Whether a declaration is the string form, a range, or the typed Rust form.
/// 该声明是字符串形式、区间形式，还是类型化 Rust 形式。
///
/// Delegates to `DeclaredGraft::form`, which owns the rule: the MCP bridge's
/// overlay projection labels the same cuts, so a second copy here would let the
/// two surfaces spell one declaration differently.
/// 委托给拥有该规则的 `DeclaredGraft::form`：MCP 桥的覆盖投影给同一批切口贴标签，因此这里有
/// 第二份副本就会让两个执行面对同一条声明给出不同拼法。
pub(super) fn cut_form(cut: &DeclaredGraft) -> &'static str {
    cut.form()
}
