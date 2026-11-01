//! What a write reply says, in the shape an agent reads next.
//! 一次写入的回复说什么，按代理接下来要读的形状。
//!
//! Split out of `apply.rs` so the operation file stays inside the repository's size ratchet, and
//! because "what happened" is a subject of its own: the same `Outcome` is rendered for a preview and
//! an apply, and the one thing a preview must never say is that the project changed.
//! 从 `apply.rs` 拆出来，使操作文件留在仓库的尺寸棘轮之内，也因为"发生了什么"是它自己的题目：同一个
//! `Outcome` 要为预览与应用各渲染一次，而预览绝不能说的那件事是"本项目已改变"。

use std::path::Path;

use crate::build_method::face_views;
use crate::mcp::apply_target::Target;

use super::Outcome;
use super::apply_family;
use super::apply_notes;
use super::apply_refusals::consequences;

/// What happened, in the shape the agent reads next.
/// 发生了什么，以代理接下来要读的形状给出。
pub(super) fn report(
    root: &Path,
    target: &Target,
    namespace: &str,
    outcome: &Outcome,
    full: bool,
) -> Result<String, String> {
    let applied = target.applied();
    let verb = match (outcome.moved, applied) {
        (false, true) => "applied",
        (false, false) => "would write",
        (true, true) => "moved",
        (true, false) => "would move",
    };
    // The tree the change produces is the strongest part of the preview: it is the
    // same derivation the registry query reports, run on the tree the executor just
    // changed, so a validation failure shows up here rather than after a write.
    // 变更产生的树是预览最强的部分：它与注册树查询报告的是同一份推导，只是跑在执行器刚改过的
    // 那棵树上，因此校验失败会在这里出现，而不是在写入之后。
    let faces = face_views(target.work_dir(root), namespace)?;
    // Audit `W2-4`: the reply used to print **every** face of the tree on every write. On the
    // nine-face fixtures that is a few lines; on the 50,000-face tree the same sentence is the
    // whole payload — and the reader of a write is looking at *what changed*, not at an
    // inventory. What arrives unasked is the census, the face that changed, and the rest is
    // withheld with the call that buys it (`full: true`).
    // 审计 `W2-4`：回复过去在每次写入时印出**整棵树的每个面**。在九面的夹具上那是几行；在五万面的树上
    // 同一句话就是整份载荷——而写入的读者看的是**改了什么**，不是一份清单。不请自来的是普查与发生变动的
    // 那个面，其余被扣下，并附上买下它的那次调用（`full: true`）。
    let changed = outcome
        .source
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    let changed_face = faces
        .iter()
        .find(|face| !changed.is_empty() && face.source.ends_with(&changed));
    let list = if full {
        faces
            .iter()
            .map(|face| format!("  {}  {}  {}", face.path, face.kind, face.source))
            .collect::<Vec<_>>()
            .join("\n")
    } else {
        let shown = changed_face
            .map(|face| format!("changed  {}  {}  {}", face.path, face.kind, face.source))
            .unwrap_or_else(|| {
                "changed  (this action moves the file rather than rewriting a face)".to_owned()
            });
        let hidden = faces
            .len()
            .saturating_sub(usize::from(changed_face.is_some()));
        if hidden == 0 {
            shown
        } else {
            let withheld = crate::mcp::truncation::withheld(
                hidden,
                faces.len(),
                1,
                "face(s)",
                "pass `full: true` for the whole tree; `registry` prints it with the members grouped",
            );
            format!("{shown}\n{withheld}")
        }
    };
    let declaration = outcome
        .declaration
        .as_ref()
        .map(|anchor| format!("declaration {anchor}\n"))
        .unwrap_or_default();
    // The executor reports what it did in the past tense, and in a preview it did
    // that on the copy: printing the sentence bare reads as a claim that this
    // project changed, which is the one thing a preview must never say. The label
    // scopes it to the preview instead of hiding it, because the sentence carries
    // the parent identity and the old-to-new module names the diff does not.
    // 执行器用过去时报告它做了什么，而在预览里它是在副本上做的：把这句原样打印，读起来就是
    // "本项目已改变"，而这正是预览绝对不能说的话。给它一个标签、把它限定在预览里，而不是藏
    // 起来——那句话里带着 diff 没有的父级身份与改名前后。
    let reported = if applied {
        outcome.message.clone()
    } else {
        format!("preview effect: {}", outcome.message)
    };
    let work = target.work_dir(root);
    let consequences = consequences(work, root, namespace, outcome)?;
    // Audit `W2-5` (D2): the editable-field list is a **refusal** aid — it exists so a caller who
    // guessed a key learns the shape instead of guessing again — and it was being reprinted in every
    // successful reply, where the round measured it as a constant block nobody needed twice. The
    // refusal paths still carry it (`invalid_field`, `run_add`, `run_edit`), so the shape is one
    // mistake away rather than one reply away.
    // 审计 `W2-5`（D2）：可编辑字段清单是**拒绝**的辅助——它的存在是为了让猜错键的调用方学会形状、而不是
    // 再猜一次——而它过去在每条成功回复里重印，那一轮量到它是一个"没人需要第二次"的常量块。拒绝路径仍然带着
    // 它（`invalid_field`、`run_add`、`run_edit`），因此那形状离一次犯错，而不是离一条回复。
    let mut reply = format!(
        "action {}\nnamespace {namespace}\n{verb} {}\n{declaration}{reported}\nfaces {}\n{list}\n{consequences}",
        if applied { "apply" } else { "preview" },
        outcome.source.display(),
        faces.len()
    );
    // Audit `W5-6`: the write reply carries the family verdict (see `apply_family`).
    // 审计 `W5-6`：写入回复携带家族判定（见 `apply_family`）。
    if let Some(face) = changed_face {
        reply.push_str(&apply_family::verdict_lines(work, namespace, &face.path));
    }
    // Audit `W5-3`/`W5-6`/`W5-7`: what the caller needs to know about the change beyond the tree.
    // 审计 `W5-3`/`W5-6`/`W5-7`：除树之外，调用方还需要知道的事。
    reply.push_str(&apply_notes::notes(outcome, applied));
    Ok(reply)
}
