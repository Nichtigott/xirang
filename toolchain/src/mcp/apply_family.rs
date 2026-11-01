//! The family verdict a write reports: what the tree says about the face it just wrote, in the
//! **same** terms the comparison uses (audit `W5-6`).
//! 一次写入报告的家族判定：树对"它刚写下的那个面"说了什么，用的是**比对**同一套说法（审计 `W5-6`）。
//!
//! The acceptance this exists for is "零写后回查": a caller that has just created a face has to know
//! whether it landed **like its siblings**, and the cheapest place to say that is the reply to the
//! write itself. Asking `consistency` afterwards costs a second instrument call for a fact the write
//! path already had in hand.
//! 它为之存在的验收是"零写后回查"：刚创建一个面的调用方需要知道它是否**长得像它的兄弟**，而说这件事最便宜
//! 的地方就是写入自己的回复。事后去问 `consistency`，等于为一个写入路径本来就有的事实付第二次仪器调用。
//!
//! One rule, not two: the majority arithmetic is `consistency`'s own (`deviations`), and the shape
//! vocabulary is `consistency_support`'s. A second copy here is how "outlier" would come to mean two
//! different things in two answers about one tree.
//! 只有一条规则：多数派算术用的是 `consistency` 自己的（`deviations`），形状词汇用的是
//! `consistency_support` 的。在这里抄第二份，就是"outlier"在关于同一棵树的两份答案里开始有两个意思的来路。
//!
//! online: the verdict is about the tree **as the write just left it** — the face that was just
//! created is in no record at all, and its siblings may have changed since the build. A published
//! record could only answer about faces it has already seen.

use std::path::Path;

use crate::mcp::consistency::deviations;
use crate::mcp::consistency::{Wording, declared_shape, one_face};

/// The family verdict for one face's parent, as reply lines.
/// 某个面的父级那一家族的判定，按回复行给出。
///
/// Returns an empty vector when the face has no parent to compare under, when its parent has no other
/// child, or when the family cannot be derived — a verdict about a family of one is not a verdict.
/// 当这个面没有可比的父级、父级没有别的子面、或家族推导不出来时回空向量——对"只有自己一家"的判定不是判定。
/// The verdict as reply-ready text, so the report stays a report (and `apply.rs` stays under the
/// 600-code-line ratchet).
/// 判定按可直接写进回复的文本给出，好让报告保持是报告（也让 `apply.rs` 留在 600 行代码行的棘轮之内）。
pub(super) fn verdict_lines(work: &Path, namespace: &str, face_path: &str) -> String {
    let lines = family_verdict(work, namespace, face_path);
    if lines.is_empty() {
        return String::new();
    }
    format!("{}\n", lines.join("\n"))
}

pub(super) fn family_verdict(work: &Path, namespace: &str, face_path: &str) -> Vec<String> {
    let Some(parent) = face_path
        .trim_end_matches('/')
        .rsplit_once('/')
        .map(|(above, _)| above)
    else {
        return Vec::new();
    };
    let Ok(faces) = crate::mcp::resolve::derived_faces(work, namespace) else {
        return Vec::new();
    };
    let siblings: Vec<_> = faces
        .0
        .iter()
        .filter(|face| {
            face.path
                .trim_end_matches('/')
                .rsplit_once('/')
                .is_some_and(|(above, _)| above == parent)
        })
        .collect();
    if siblings.len() < 2 {
        return Vec::new();
    }
    // The shape signal, whose vocabulary a family contract is stated in: the field names each sibling
    // declares. The `api` signal compares call names and belongs to the comparison, not to a write.
    // 形状信号——家族契约就是用它的词汇陈述的：每个兄弟声明了哪些字段。`api` 信号比的是调用名，那属于比对，
    // 不属于一次写入。
    let mut sets = Vec::new();
    for face in &siblings {
        let text = std::fs::read_to_string(work.join("src").join(&face.source));
        let Ok(text) = text else {
            continue;
        };
        let Ok(parsed) = one_face(&text) else {
            continue;
        };
        let names = declared_shape(&parsed)
            .into_iter()
            .map(|field| field.spelling.to_owned())
            .collect::<std::collections::BTreeSet<_>>();
        let label = face
            .path
            .rsplit('/')
            .next()
            .unwrap_or(&face.path)
            .to_owned();
        sets.push((label, names));
    }
    if sets.len() < 2 {
        return Vec::new();
    }
    let found = deviations(&sets, Wording::Declares);
    let mut lines = vec![format!(
        "family     {} sibling(s) under {parent}; outliers: {} of {}",
        sets.len(),
        found.len(),
        sets.len()
    )];
    for (sibling, notes) in found.iter().take(3) {
        lines.push(format!(
            "  outlier     {sibling}: {} — `consistency {{parent: \"{parent}\"}}` prints the fix",
            notes.join("; ")
        ));
    }
    lines
}

/// Complete an `add` request's fields from the family the new face is joining.
/// 用新面正在加入的那个家族补全 `add` 请求的字段。
///
/// The parent is read from the request in both spellings it accepts (`parent` at the top level, or
/// inside `fields`), because a caller that wrote the second one would otherwise silently get no
/// inheritance at all — a difference no reply would show.
/// 父级按请求接受的两种拼法读（顶层的 `parent`，或 `fields` 里的），因为写了第二种的调用方否则会静默地
/// 完全得不到继承——而那个差别没有任何回复会显示。
pub(super) fn complete_add_fields(
    work: &Path,
    namespace: &str,
    arguments: &serde_json::Value,
    fields: &serde_json::Value,
) -> (serde_json::Value, Vec<String>) {
    match arguments
        .get("parent")
        .or_else(|| fields.get("parent"))
        .and_then(serde_json::Value::as_str)
    {
        Some(parent) => inherited_fields(work, namespace, parent, fields),
        None => (fields.clone(), Vec::new()),
    }
}

/// The family's own declarations, filled into a new face's fields where the caller left them out.
/// 当调用方没写时，把家族自己的声明填进新面的字段里。
///
/// Audit `W5-6`'s first half: "derive the family contract and template **from the tree**". A caller
/// that has to spell `exports` and `handle_traits` from memory is doing work the parent's other
/// children have already done — and getting one of them wrong is how a family drifts. What is
/// inherited is the **majority** label set per field, and only where the caller said nothing: an
/// explicit value always wins, and the reply says which fields it filled in, because a value that
/// arrived without being asked for must never look like one the caller sent.
/// 审计 `W5-6` 的前半："**从树上**推导家族契约与模板"。一个必须凭记忆拼 `exports` 与 `handle_traits`
/// 的调用方，做的是父级其它孩子已经做过的工作——而拼错其中之一正是家族开始漂移的方式。继承的是每个字段的
/// **多数派**标签集，而且只在调用方什么都没说的地方：显式给的值永远赢，而回复会说出它替调用方补了哪些字段，
/// 因为一个不请自来的值绝不能看起来像调用方送的那个。
///
/// `parts` is deliberately **not** inherited: it names the object's own parts type, so a value copied
/// from a sibling would be wrong by construction (the same rule the shape comparison follows).
/// `parts` **有意不继承**：它点名对象自己的零件类型，因此抄兄弟的值按构造就是错的（与形状比对同一条规则）。
///
/// Returns the completed fields and one note per inherited field.
/// 返回补全后的字段，以及每个被继承字段的一条说明。
pub(super) fn inherited_fields(
    work: &Path,
    namespace: &str,
    parent_path: &str,
    fields: &serde_json::Value,
) -> (serde_json::Value, Vec<String>) {
    let inherited_keys = ["exports", "handle_traits", "part_traits"];
    let mut completed = fields.clone();
    let mut notes = Vec::new();
    let Ok(faces) = crate::mcp::resolve::derived_faces(work, namespace) else {
        return (completed, notes);
    };
    let siblings: Vec<_> = faces
        .0
        .iter()
        .filter(|face| {
            face.path
                .trim_end_matches('/')
                .rsplit_once('/')
                .is_some_and(|(above, _)| above == parent_path.trim_end_matches('/'))
        })
        .collect();
    if siblings.is_empty() {
        return (completed, notes);
    }
    // Each sibling's declared labels per inheritable field, read with the comparison's own parser.
    // 每个兄弟在每个可继承字段上声明的标签，用比对自己的解析器读。
    let mut declared: Vec<(String, std::collections::BTreeMap<&str, Vec<String>>)> = Vec::new();
    for face in &siblings {
        let Ok(text) = std::fs::read_to_string(work.join("src").join(&face.source)) else {
            continue;
        };
        let Ok(parsed) = one_face(&text) else {
            continue;
        };
        let mut per_field = std::collections::BTreeMap::new();
        for field in declared_shape(&parsed) {
            if inherited_keys.contains(&field.spelling) {
                per_field.insert(
                    field.spelling,
                    field
                        .value
                        .split(", ")
                        .map(|label| label.trim().to_owned())
                        .filter(|label| !label.is_empty())
                        .collect(),
                );
            }
        }
        declared.push((face.path.clone(), per_field));
    }
    if declared.is_empty() {
        return (completed, notes);
    }
    let Some(object) = completed.as_object_mut() else {
        return (completed, notes);
    };
    for key in inherited_keys {
        let stated = fields
            .get(key)
            .and_then(serde_json::Value::as_str)
            .is_some_and(|value| !value.trim().is_empty());
        if stated {
            continue;
        }
        let mut counts: std::collections::BTreeMap<&str, usize> = std::collections::BTreeMap::new();
        for (_, per_field) in &declared {
            for label in per_field.get(key).into_iter().flatten() {
                *counts.entry(label.as_str()).or_default() += 1;
            }
        }
        let majority: Vec<&str> = counts
            .iter()
            .filter(|(_, count)| **count * 2 > declared.len())
            .map(|(label, _)| *label)
            .collect();
        if majority.is_empty() {
            continue;
        }
        let value = majority.join(", ");
        object.insert(key.to_owned(), serde_json::Value::String(value.clone()));
        notes.push(format!(
            "inherited  {key} = {value} from the family under {parent_path}; pass it explicitly to \
             override"
        ));
    }
    (completed, notes)
}
