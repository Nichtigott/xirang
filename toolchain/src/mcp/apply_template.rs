//! The approved-template choice a new object follows: unique, conflicting, or none (a candidate).
//! 一个新对象所遵循的"已批准模板"选择：唯一、冲突，或没有（候选）。
//!
//! P3's first slice. An **approved template** is a specimen the adoption ledger trusts: an entry whose
//! anchor resolves to a face and whose lease is in force. "Applies" means the specimen is in the same
//! family as the prospective object — same parent and same kind — so an unrelated template is never
//! read. One such specimen is a template; two are a **conflict** (a person says which one), and none
//! means the new face is a **candidate**: it may become the specimen a later adoption makes a
//! template, but creating it does not crown it.
//! P3 的第一片。**已批准模板**是被采信台账信任的样板：条目锚点解析到一个面，且租约生效。"适用"指样板与将要
//! 创建的对象同族——同父级、同 kind——因此无关模板从不被读取。恰好一个 ⇒ 模板；两个 ⇒ **冲突**（由人指出
//! 哪一个）；没有 ⇒ 新面是**候选**：它可以成为以后某个采信所认证的样板，但创建它本身并不册封它。
//!
//! What this slice does **not** do: the renderer still uses the language adapter's own spelling for the
//! file's contents. The selection is consumed (it decides the reply, and a conflict refuses the write
//! before any byte), while "generate the file *from* the specimen's structure" is the next slice — it
//! must not be claimed as done here.
//! 本片**不做**：渲染器仍用语言适配器自己的拼法写文件内容。选择已被消费（它决定回复，冲突在任何字节改变前
//! 拒绝写入），而"按样板的文件结构生成"是下一片——不能在这里冒充完成。

use std::path::Path;

use crate::mcp::adopted::{AnchorStanding, anchor_standing, entries};
use crate::mcp::rules::resolve_subject;

/// What an add request's family has as its approved template.
/// 一次新增所属家族拥有的已批准模板。
pub(super) enum Choice {
    /// Exactly one in-force specimen is in this family.
    /// 这个家族里恰好有一个生效的样板。
    Unique(String),
    /// More than one; the caller must pick, so nothing is written.
    /// 多于一个；调用方必须选，因此什么都不写。
    Conflict(Vec<String>),
    /// None; the new face is a candidate rather than a template.
    /// 没有；新面是候选而不是模板。
    None,
}

/// Choose the approved template for `(parent, kind)`, excluding `prospective` itself.
/// 为 `(parent, kind)` 选择已批准模板，排除将要创建的 `prospective` 自身。
pub(super) fn select(
    root: &Path,
    work: &Path,
    namespace: &str,
    parent: &str,
    kind: &str,
    prospective: &str,
) -> Result<Choice, String> {
    let mut anchors: Vec<String> = entries(root)?
        .into_iter()
        .map(|entry| entry.anchor)
        .collect();
    anchors.sort();
    anchors.dedup();
    let views = crate::build_method::face_views(work, namespace)?;
    let mut applicable = Vec::new();
    for anchor in anchors {
        if !matches!(anchor_standing(root, &anchor), Ok(AnchorStanding::InForce)) {
            continue;
        }
        let Ok(Some(subject)) = resolve_subject(work, &anchor) else {
            continue;
        };
        if subject.path == prospective {
            continue;
        }
        let Some(view) = views.iter().find(|view| view.path == subject.path) else {
            continue;
        };
        let view_parent = view.path.rsplit_once('/').map_or("", |(above, _)| above);
        if view_parent == parent && view.kind == kind {
            applicable.push(anchor);
        }
    }
    Ok(match applicable.len() {
        0 => Choice::None,
        1 => Choice::Unique(applicable.remove(0)),
        _ => Choice::Conflict(applicable),
    })
}

/// The reply line an `add` carries, plus the defaults its template contributes.
/// 一次 `add` 携带的回复行，以及它的模板贡献的默认值。
pub(super) struct AddChoice {
    pub(super) line: String,
    /// The specimen anchor this add follows, when one applies.
    /// 这次新增所跟随的样板锚点（有适用模板时）。
    pub(super) anchor: Option<String>,
    /// `(field, value)` pairs copied from the template **only for fields the request omitted**.
    /// 从模板复制的 `(字段, 值)`，**只针对请求没有提到的字段**。
    pub(super) defaults: Vec<(String, String)>,
}

/// The reply an `add` carries about its family's approved template.
/// 一次 `add` 就它家族的已批准模板携带的回复。
///
/// Returns the conflict as `Err` — the write path turns that into a refusal **before** any byte is
/// written, which is why the message says so. A request without a module/kind is left to `run_add`,
/// which refuses it with its own, better message for the missing key.
/// 冲突以 `Err` 返回——写入路径据此在**任何字节改变之前**拒绝，消息里也这么说。缺少 module/kind 的请求
/// 留给 `run_add` 用它自己更准确的缺键消息拒绝。
pub(super) fn add_choice(
    root: &Path,
    work: &Path,
    namespace: &str,
    arguments: &serde_json::Value,
    record: bool,
) -> Result<AddChoice, String> {
    let Some((parent, module, kind)) = prospective_add(arguments) else {
        return Ok(AddChoice {
            line: String::new(),
            anchor: None,
            defaults: Vec::new(),
        });
    };
    let prospective = format!("{parent}/{module}");
    match select(root, work, namespace, &parent, &kind, &prospective)? {
        Choice::Unique(anchor) => {
            let mut line = format!(
                "template: follows the approved specimen `{anchor}` (same family: `{parent}`, kind `{kind}`)\n"
            );
            // Lineage is what a later template change enumerates: "which objects follow this
            // specimen, at which version". Recorded only on a write, like the candidate record.
            // 谱系是模板以后变化时枚举"哪些对象跟随这个样板、哪个版本"的输入。与候选一样只在写入时记录。
            if record
                && let Some(version) = template_version(root, &anchor)
                && let Err(error) = record_lineage(root, &prospective, &anchor, &version)
            {
                line.push_str(&format!(
                    "note: the template lineage could not be recorded: {error}\n"
                ));
            }
            Ok(AddChoice {
                line,
                anchor: Some(anchor.clone()),
                defaults: template_defaults(work, namespace, &anchor)?,
            })
        }
        Choice::Conflict(anchors) => Err(format!(
            "this category has {} approved specimens ({}); a template must be unique — retire one \
             of those decisions, or state which one this face follows, and nothing is written \
             meanwhile",
            anchors.len(),
            anchors.join(", ")
        )),
        Choice::None => {
            let mut line = "template: no approved specimen for this category yet; this face is a \
                            candidate — adopting a specimen is what makes it a template\n"
                .to_owned();
            if record && let Err(error) = record_candidate(root, &prospective, &parent, &kind) {
                line.push_str(&format!(
                    "note: the candidate record could not be written: {error}\n"
                ));
            }
            Ok(AddChoice {
                line,
                anchor: None,
                defaults: Vec::new(),
            })
        }
    }
}

/// The shape fields the approved specimen contributes to a new face that did not state them.
/// 已批准样板贡献给"没有自己声明这些"的新面的形状字段。
///
/// Only the two fields a template actually embodies as a **shape**: `preset` and `parts`. Semantics
/// such as `provides`/`requires` are per-object and are not copied. The values are read from the
/// specimen's own declaration, and the language-neutral defaults are skipped (copying `NoPreset`
/// into every new face would turn a default into a template).
/// 只取模板真正作为**形状**体现的两个字段：`preset` 与 `parts`。`provides`/`requires` 这类语义属于各个
/// 对象，不复制。值来自样板自己的声明，语言无关的默认值跳过（把 `NoPreset` 复制进每个新面会把默认值
/// 变成模板）。
fn template_defaults(
    work: &Path,
    namespace: &str,
    anchor: &str,
) -> Result<Vec<(String, String)>, String> {
    let Some(subject) = resolve_subject(work, anchor)? else {
        return Ok(Vec::new());
    };
    let views = crate::build_method::face_views(work, namespace)?;
    let Some(view) = views.iter().find(|view| view.path == subject.path) else {
        return Ok(Vec::new());
    };
    let layout = crate::build_method::source_layout(work)?;
    let path = view.locate_source(&layout)?;
    let text = std::fs::read_to_string(&path)
        .map_err(|error| format!("cannot read the template `{}`: {error}", path.display()))?;
    Ok(shape_of_text(&text, &view.source))
}

/// The fields a template embodies as a **shape**, with the value that means "not supplied".
/// 模板作为**形状**体现的字段，以及表示"没有提供"的那个值。
///
/// One list, used by both the creation path and the refactor plan: a plan that decided what a
/// template supplies differently from `add` would carry a shape this repository never writes.
/// 一份清单，创建路径与重构计划共用：一份对"模板提供什么"作出与 `add` 不同判断的计划，会搬运一个本仓库
/// 从不写出的形状。
pub(crate) const SHAPE_FIELDS: [(&str, &str); 2] = [("preset", "NoPreset"), ("parts", "NoParts")];

/// The non-default shape values one face file declares.
/// 一个面文件声明的非默认形状值。
pub(crate) fn shape_of_text(text: &str, source: &str) -> Vec<(String, String)> {
    let Some(face) = crate::build_method::face_syntax_check::parsed_face(text, source) else {
        return Vec::new();
    };
    let mut shape = Vec::new();
    for (field, default) in SHAPE_FIELDS {
        if let Some(value) = face.path(field)
            && value != default
        {
            shape.push((field.to_owned(), value));
        }
    }
    shape
}

/// The logical path an `add` request is about to create, as `(parent, module, kind)`.
/// 一次 `add` 请求将要创建的逻辑路径，按 `(父级, 模块名, kind)` 给出。
fn prospective_add(arguments: &serde_json::Value) -> Option<(String, String, String)> {
    let fields = arguments.get("fields")?;
    let module = fields
        .get("module")
        .and_then(serde_json::Value::as_str)
        .filter(|module| !module.trim().is_empty())?;
    let kind = fields
        .get("kind")
        .and_then(serde_json::Value::as_str)
        .filter(|kind| !kind.trim().is_empty())?;
    let parent = arguments
        .get("parent")
        .and_then(serde_json::Value::as_str)
        .filter(|parent| !parent.trim().is_empty())
        .unwrap_or("root");
    Some((
        parent.trim_end_matches('/').to_owned(),
        module.to_owned(),
        kind.to_owned(),
    ))
}

/// A write whose preview named a different approved template is refused, before any byte.
/// 预览点名的已批准模板与此刻不同的写入，在任何字节改变之前被拒绝。
pub(super) fn ensure_plan_template(
    planned: Option<&str>,
    current: Option<&str>,
) -> Result<(), String> {
    if planned == current {
        return Ok(());
    }
    let name = |anchor: Option<&str>| {
        anchor
            .map(|anchor| format!("`{anchor}`"))
            .unwrap_or_else(|| "none".to_owned())
    };
    Err(format!(
        "the approved template this preview followed changed: was {}, now {}; nothing was written — \
         re-run the preview so the plan is decided against the current specimens",
        name(planned),
        name(current)
    ))
}

/// The file candidates are recorded in, under the project's `.xirang`.
/// 候选记录所在的文件，位于项目的 `.xirang` 下。
fn candidates_path(root: &Path) -> std::path::PathBuf {
    root.join(xirang_kernel::lexicon::XIRANG_DIR)
        .join("templates")
        .join("candidates.tsv")
}

/// Record a face created with no approved template, so a later adoption can name it.
/// 记录一个在没有已批准模板时创建的面，好让之后的采信能点名它。
pub(super) fn record_candidate(
    root: &Path,
    path: &str,
    parent: &str,
    kind: &str,
) -> Result<(), String> {
    // A project that keeps no `.xirang` state is not given one by creating a face: the candidate
    // is a fact for the adoption loop, and recording it must not turn an add into "this project now
    // has state". A tree that already keeps state there gets the record.
    // 本来没有 `.xirang` 状态的项目，不会因为创建一个面就被塞进一份：候选是给采信闭环用的事实，记录它
    // 不该让一次 add 变成"这个项目从此有状态"。已经在那里保存状态的树才得到这条记录。
    if !root.join(xirang_kernel::lexicon::XIRANG_DIR).is_dir() {
        return Ok(());
    }
    let file = candidates_path(root);
    let mut text = std::fs::read_to_string(&file).unwrap_or_default();
    let already = text.lines().any(|line| {
        let fields = line.split('\t').collect::<Vec<_>>();
        fields.len() >= 3 && fields[0] == path && fields[1] == parent && fields[2] == kind
    });
    if already {
        return Ok(());
    }
    if !text.is_empty() && !text.ends_with('\n') {
        text.push('\n');
    }
    text.push_str(&format!("{path}\t{parent}\t{kind}\n"));
    if let Some(parent) = file.parent() {
        std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    std::fs::write(&file, text).map_err(|error| format!("cannot write {}: {error}", file.display()))
}

/// Candidates recorded for one family, excluding `except`.
/// 某个家族记录在案的候选，排除 `except`。
pub(super) fn candidates(
    root: &Path,
    parent: &str,
    kind: &str,
    except: &str,
) -> Result<Vec<String>, String> {
    let text = std::fs::read_to_string(candidates_path(root)).unwrap_or_default();
    let mut found = Vec::new();
    for line in text.lines() {
        let fields = line.split('\t').collect::<Vec<_>>();
        if fields.len() >= 3 && fields[1] == parent && fields[2] == kind && fields[0] != except {
            found.push(fields[0].to_owned());
        }
    }
    Ok(found)
}

/// The note a new adoption carries when candidates of its family can now follow it.
/// 一次新采信在"同族候选从此可以跟随它"时携带的说明。
pub(crate) fn candidate_note(root: &Path, anchor: &str) -> Option<String> {
    let subject = resolve_subject(root, anchor).ok().flatten()?;
    let namespace = crate::mcp::registry::namespace(root).ok()?;
    let views = crate::build_method::face_views(root, &namespace).ok()?;
    let view = views.iter().find(|view| view.path == subject.path)?;
    let parent = view.path.rsplit_once('/').map_or("", |(above, _)| above);
    let candidates = candidates(root, parent, &view.kind, &view.path).ok()?;
    if candidates.is_empty() {
        return None;
    }
    Some(format!(
        "candidate(s) in this family now follow this specimen as their template: {}\n",
        candidates.join(", ")
    ))
}

/// The file that records which object follows which approved template, and at which version.
/// 记录"哪个对象跟随哪个已批准模板、哪个版本"的文件。
fn lineage_path(root: &Path) -> std::path::PathBuf {
    root.join(xirang_kernel::lexicon::XIRANG_DIR)
        .join("templates")
        .join("lineage.tsv")
}

/// The fingerprint of the lease a template anchor's newest entry was confirmed on.
/// 模板锚点最新条目被确认时那份租约的指纹。
pub(crate) fn template_version(root: &Path, anchor: &str) -> Option<String> {
    crate::mcp::adopted::entries(root)
        .ok()?
        .into_iter()
        .rfind(|entry| entry.anchor == anchor)
        .map(|entry| entry.fingerprint)
}

/// Record that `object` follows `anchor` at `version`; a later template change can enumerate it.
/// 记录 `object` 跟随 `anchor` 的 `version`；模板以后变化时能据此枚举受影响对象。
///
/// Same guard as candidates: a tree that keeps no `.xirang` state is not given one by an add.
/// 与候选同一条门：本来没有 `.xirang` 状态的树，不会因为一次 add 被塞进一份。
pub(super) fn record_lineage(
    root: &Path,
    object: &str,
    anchor: &str,
    version: &str,
) -> Result<(), String> {
    if !root.join(xirang_kernel::lexicon::XIRANG_DIR).is_dir() {
        return Ok(());
    }
    let file = lineage_path(root);
    let mut text = std::fs::read_to_string(&file).unwrap_or_default();
    let already = text.lines().any(|line| {
        let fields = line.split('\t').collect::<Vec<_>>();
        fields.len() >= 3 && fields[0] == object && fields[1] == anchor
    });
    if already {
        // A new version is a new fact: keep one line per object and template, latest version wins.
        // 版本更新是新事实：每个对象与模板只留一行，最新版本胜出。
        text = text
            .lines()
            .filter(|line| {
                let fields = line.split('\t').collect::<Vec<_>>();
                !(fields.len() >= 3 && fields[0] == object && fields[1] == anchor)
            })
            .map(|line| format!("{line}\n"))
            .collect();
    } else if !text.is_empty() && !text.ends_with('\n') {
        text.push('\n');
    }
    text.push_str(&format!("{object}\t{anchor}\t{version}\n"));
    if let Some(parent) = file.parent() {
        std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    std::fs::write(&file, text).map_err(|error| format!("cannot write {}: {error}", file.display()))
}

/// The objects recorded as following one template anchor, each with the version it followed.
/// 记录在案、跟随某个模板锚点的对象，各自带上它跟随的版本。
pub(crate) fn followers(root: &Path, anchor: &str) -> Vec<(String, String)> {
    let text = std::fs::read_to_string(lineage_path(root)).unwrap_or_default();
    text.lines()
        .filter_map(|line| {
            let fields = line.split('\t').collect::<Vec<_>>();
            (fields.len() >= 3 && fields[1] == anchor)
                .then(|| (fields[0].to_owned(), fields[2].to_owned()))
        })
        .collect()
}

/// The followers of one anchor, split by whether the object they name still resolves.
/// 某个锚点的跟随者，按"它点名的对象是否仍可解析"分成两半。
///
/// A path that no longer resolves is not a follower any more: the record outlived a delete (a delete
/// is recoverable) or a change made outside this bridge. Listing it among "the objects a template
/// change would carry" would promise a movement of something that is not there, so the honest act is
/// to name it as needing a new pointer.
/// 解析不到的路径不再是跟随者：记录活过了一次删除（删除是可恢复的）或桥外的一次改动。把它列进"模板变化要
/// 带走的对象"会承诺搬动一个并不存在的东西，因此诚实的动作是点名它需要重新指向。
pub(crate) fn followers_split(root: &Path, anchor: &str) -> (Vec<(String, String)>, Vec<String>) {
    let mut present = Vec::new();
    let mut stranded = Vec::new();
    for (path, version) in followers(root, anchor) {
        if matches!(resolve_subject(root, &path), Ok(Some(_))) {
            present.push((path, version));
        } else {
            stranded.push(path);
        }
    }
    (present, stranded)
}

/// Whether `path` is `prefix` itself or lives under it.
/// `path` 是 `prefix` 本身，还是住在它之下。
fn under(path: &str, prefix: &str) -> bool {
    path == prefix || path.starts_with(&format!("{prefix}/"))
}

/// One lineage rewrite a move owes: the bytes, the bytes they were computed from, and the anchors the
/// move strands.
/// 一次搬动欠下的谱系改写：字节、计算所依据的原文，以及这次搬动留下的锚点。
pub(super) struct LineageRewrite {
    /// The lineage file itself.
    /// 谱系文件本身。
    pub(super) path: std::path::PathBuf,
    /// The bytes the plan was computed from; a change there refuses the write (`needs re-plan`).
    /// 计划所依据的原文；它变了就以 `needs re-plan` 拒绝写入。
    pub(super) original: String,
    /// The body that spells every relocated follower at its new path.
    /// 把每个被搬动的跟随者写到新路径的正文。
    pub(super) body: String,
    /// Every `(before, after)` object path this rewrite carries.
    /// 这次改写带走的每个 `(旧, 新)` 对象路径。
    pub(super) moved: Vec<(String, String)>,
    /// Templates this move relocated: their followers name an anchor that resolves to nothing.
    /// 这次搬动挪走了的模板：它们的跟随者点着解析不到任何对象的锚点。
    pub(super) stranded: Vec<String>,
}

/// Plan the lineage rewrite for a move of `from` to `to`; `None` when the tree keeps no lineage.
/// 为一次 `from` → `to` 的搬动规划谱系改写；树里没有谱系时为 `None`。
///
/// Two facts live in one row, and they are carried differently. The **object** column is a path this
/// move changes, so it is rewritten mechanically — prefix for prefix, descendants included. The
/// **anchor** column names a template through the adoption ledger, which still spells the old name and
/// whose lease was confirmed against the bytes that moved; rewriting it here would invent a decision
/// nobody made, so a relocated template is reported as stranded instead.
/// 一行里有两条事实，搬运方式不同。**对象**列是这次搬动会改掉的路径，因此机械地逐前缀改写——后代也算在内。
/// **锚点**列经采信台账点名一个模板，而台账仍写着旧名字、它的租约也是对照被搬动的字节确认的；在这里改写它等于
/// 发明一个没人做过的决定，因此被挪走的模板被报成 stranded。
pub(super) fn plan_move(
    root: &Path,
    from: &str,
    to: &str,
) -> Result<Option<LineageRewrite>, String> {
    let path = lineage_path(root);
    let Ok(original) = std::fs::read_to_string(&path) else {
        return Ok(None);
    };
    let mut moved = Vec::new();
    let mut body = String::new();
    let mut anchors: Vec<(String, Vec<String>)> = Vec::new();
    for line in original.lines() {
        let fields = line.split('\t').collect::<Vec<_>>();
        if fields.len() < 3 {
            body.push_str(line);
            body.push('\n');
            continue;
        }
        let (object, anchor, version) = (fields[0], fields[1], fields[2]);
        let now = if under(object, from) {
            let now = format!("{to}{}", &object[from.len()..]);
            moved.push((object.to_owned(), now.clone()));
            now
        } else {
            object.to_owned()
        };
        match anchors.iter_mut().find(|(name, _)| name == anchor) {
            Some((_, followers)) => followers.push(now.clone()),
            None => anchors.push((anchor.to_owned(), vec![now.clone()])),
        }
        body.push_str(&format!("{now}\t{anchor}\t{version}\n"));
    }
    let mut stranded = Vec::new();
    for (anchor, followers) in anchors {
        let Ok(Some(subject)) = resolve_subject(root, &anchor) else {
            continue;
        };
        if under(&subject.path, from) {
            stranded.push(format!(
                "lineage: the template anchor `{anchor}` named an object this move relocated \
                 (`{}`), so the follower(s) {} now point at an anchor that names no object; adopt \
                 the template at its new path and re-point the lineage",
                subject.path,
                followers.join(", ")
            ));
        }
    }
    Ok(Some(LineageRewrite {
        path,
        original,
        body,
        moved,
        stranded,
    }))
}

/// Publish a planned lineage rewrite through the move's transaction.
/// 经搬动的事务发布一次已规划好的谱系改写。
///
/// The plan is re-bound to the bytes on disk first: a lineage file that changed between planning and
/// writing refuses the whole move with `needs re-plan`, because writing the old plan over it would
/// erase a row somebody else added. That is the same discipline the reference plans use, and it is a
/// minimum precondition rather than crash atomicity.
/// 计划先重新绑定到磁盘上的字节：规划与写入之间变过的谱系文件以 `needs re-plan` 拒绝整次搬动，因为把旧计划
/// 写上去会擦掉别人新增的一行。这与引用计划的纪律相同，且它是最小前提，不是崩溃原子性。
pub(super) fn publish(
    transaction: &mut crate::run_method::authoring::write_transaction::WriteTransaction,
    plan: &LineageRewrite,
) -> Result<(), String> {
    let current = std::fs::read_to_string(&plan.path).unwrap_or_default();
    if current != plan.original {
        return Err(format!(
            "{} changed while this move was being planned (`needs re-plan`); nothing was written",
            plan.path.display()
        ));
    }
    transaction.write(&plan.path, plan.body.as_bytes())
}

/// The lineage rows a delete strands, read **before** the object is gone.
/// 一次删除留下的谱系行，在对象消失**之前**读出。
///
/// `work` is the tree the faces are read from (the preview copy, or the project), and it still holds
/// the object being removed — that is what lets "this delete removed a template" be told apart from
/// "that anchor never named anything". `root` is where the lineage file lives: `.xirang` is never
/// copied into a preview, and the record is the project's own state either way.
/// `work` 是读取注册面的那棵树（预览副本或项目），它仍持有将被移除的对象——正因如此，才能把"这次删除移除了
/// 一个模板"与"那个锚点从没点名过任何东西"区分开。`root` 是谱系文件所在处：`.xirang` 从不被复制进预览，
/// 而记录在两种模式下都是项目自己的状态。
pub(super) fn plan_removal(work: &Path, root: &Path, removed: &str) -> Result<Vec<String>, String> {
    let text = std::fs::read_to_string(lineage_path(root)).unwrap_or_default();
    let rows = text
        .lines()
        .filter_map(|line| {
            let fields = line.split('\t').collect::<Vec<_>>();
            (fields.len() >= 3).then(|| {
                (
                    fields[0].to_owned(),
                    fields[1].to_owned(),
                    fields[2].to_owned(),
                )
            })
        })
        .collect::<Vec<_>>();
    let mut findings = Vec::new();
    let removed_followers = rows
        .iter()
        .filter(|(object, _, _)| under(object, removed))
        .map(|(object, _, _)| object.clone())
        .collect::<Vec<_>>();
    if !removed_followers.is_empty() {
        findings.push(format!(
            "lineage: this delete removed the follower(s) {}; their rows are kept (a delete is \
             recoverable), and `adopted` reports them as needing re-pointing until the object is back",
            removed_followers.join(", ")
        ));
    }
    // The other direction: the delete removed a **template** some objects follow. Rows are matched by
    // the anchor each one spells, and the anchor is resolved against the pre-delete tree, so the
    // preview and the write tell the same story.
    // 另一个方向：删除移除了某些对象所跟随的**模板**。行按各自写着的锚点归组，而锚点是对着删除前的树解析的，
    // 因此预览与落盘讲同一个故事。
    let mut anchors = rows
        .iter()
        .map(|(_, anchor, _)| anchor.clone())
        .collect::<Vec<_>>();
    anchors.sort();
    anchors.dedup();
    let mut orphaned = Vec::new();
    for anchor in anchors {
        let Ok(Some(subject)) = resolve_subject(work, &anchor) else {
            continue;
        };
        if !under(&subject.path, removed) {
            continue;
        }
        // A row whose object this delete already removed is not an object that needs a new
        // template: the consumer names it as needing re-pointing, and claiming it here would say
        // the tree holds something it does not.
        // 对象已在本轮删除里被移除的行，不是"需要一个新模板"的对象：消费方点名它需要重新指向，而在这里
        // 声称它，就是在说树里有一个它并不持有的东西。
        orphaned.extend(
            rows.iter()
                .filter(|(object, spelled, _)| {
                    *spelled == anchor && matches!(resolve_subject(work, object), Ok(Some(_)))
                })
                .map(|(object, _, _)| object.clone()),
        );
    }
    if !orphaned.is_empty() {
        findings.push(format!(
            "lineage: the template this delete removed is followed by {}; those objects need a new \
             template, because their anchor no longer names an object",
            orphaned.join(", ")
        ));
    }
    Ok(findings)
}

/// Every lineage row, in file order: `(object, anchor, version)`.
/// 每一行谱系，按文件顺序：`(对象, 锚点, 版本)`。
pub(crate) fn lineage_rows(root: &Path) -> Vec<(String, String, String)> {
    std::fs::read_to_string(lineage_path(root))
        .unwrap_or_default()
        .lines()
        .filter_map(|line| {
            let fields = line.split('\t').collect::<Vec<_>>();
            (fields.len() >= 3).then(|| {
                (
                    fields[0].to_owned(),
                    fields[1].to_owned(),
                    fields[2].to_owned(),
                )
            })
        })
        .collect()
}

/// Rewrite one row's version in place; `Ok(false)` when no row names that `(object, anchor)`.
/// 就地改写某一行的版本；没有行点名那个 `(对象, 锚点)` 时返回 `Ok(false)`。
///
/// This records a **conclusion** ("this object now matches that template version"), so it runs after
/// the edit it concludes from and reports failure rather than pretending the record advanced. The
/// safe direction is deliberate: a record left on an older version lists the object again next time,
/// while a record advanced without the bytes would silently stop carrying it.
/// 它记录的是一个**结论**（"这个对象现在符合那个模板版本"），因此它跑在它所依据的编辑之后，失败就如实报告，
/// 而不是假装记录前进了。这个方向是刻意选的：留在旧版本上的记录下次还会列到这个对象，而没有对应字节就前进的
/// 记录会让它被静默地不再搬运。
pub(crate) fn advance_version(
    root: &Path,
    object: &str,
    anchor: &str,
    version: &str,
) -> Result<bool, String> {
    let file = lineage_path(root);
    let Ok(text) = std::fs::read_to_string(&file) else {
        return Ok(false);
    };
    let mut found = false;
    let mut next = String::new();
    for line in text.lines() {
        let fields = line.split('\t').collect::<Vec<_>>();
        if fields.len() >= 3 && fields[0] == object && fields[1] == anchor {
            found = true;
            next.push_str(&format!("{object}\t{anchor}\t{version}\n"));
        } else {
            next.push_str(line);
            next.push('\n');
        }
    }
    if found {
        std::fs::write(&file, next)
            .map_err(|error| format!("cannot write {}: {error}", file.display()))?;
    }
    Ok(found)
}
