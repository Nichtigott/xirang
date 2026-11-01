//! The refactor plan a template change implies for the objects that follow it, and the one part of it
//! that is mechanical enough to perform.
//! 模板变化对跟随它的对象所意味着的重构计划，以及其中"机械到可以执行"的那一部分。
//!
//! **The old shape is the follower's own file.** The lineage records which template and version an
//! object followed, not the template's bytes at that version, and inventing a snapshot would be
//! inventing the very thing being compared. What the follower's file declares *is* what the old
//! template left there, so the diff is "this file against the template's file now".
//! **旧形状就是跟随者自己的文件。** 谱系记录的是"哪个对象跟随哪个模板、哪个版本"，不是那个版本模板的字节，
//! 而凭空造一份快照，等于凭空造出被比较的东西。跟随者的文件所声明的，**就是**旧模板留在那里的东西，因此差分
//! 就是"这个文件对照模板现在的文件"。
//!
//! **Mechanical means `add` already does exactly this.** A template supplies its shape fields
//! (`preset`, `parts` — see `SHAPE_FIELDS`) and its conventions. `add` copies the shape fields into a
//! new face when the request states none, so copying one into a follower that still leaves it at the
//! default is the same operation, not a new judgment. Everything else is semantic and is reported with
//! a paste-ready request instead of being written: a value the follower chose is not overwritten by a
//! template, and a test body is not generated for a test this tool cannot check.
//! **机械的意思是 `add` 已经正是这么做的。** 模板提供它的形状字段（`preset`、`parts`——见
//! `SHAPE_FIELDS`）与它的约定。请求没有声明时，`add` 会把形状字段复制进新面，因此把形状字段复制进"仍停在
//! 默认值"的跟随者是同一个操作，不是新的判断。其余都是语义，只以可粘贴的请求报告而不写入：跟随者自己选的值
//! 不被模板覆盖，而"本工具无法核验的测试体"不生成。
//!
//! **A template cannot derive a follower's place.** Its directory, file name and module name are the
//! follower's own identity, so this plan never proposes a move; when one is needed, `xirang.apply`
//! (`move`/`rename`) is the path and it already previews and transacts. What a template *does* carry
//! is conventions a file can be checked for: the generated marker and a `#[cfg(test)]` entry.
//! **模板推不出跟随者的位置。** 它的目录、文件名与模块名是跟随者自己的身份，因此本计划从不提议搬动；需要搬动
//! 时，路径是 `xirang.apply`（`move`/`rename`），它本来就预览并走事务。模板**确实**携带的是可以逐文件核对的
//! 约定：生成标记与 `#[cfg(test)]` 入口。

use std::path::{Path, PathBuf};

use serde_json::{Map, Value, json};

use crate::mcp::adopted::AnchorStanding;
use crate::mcp::apply::apply_template;
use crate::mcp::rules::resolve_subject;

/// The whole tool's description, which is also its contract.
/// 整个工具的描述，也是它的契约。
pub(crate) const DESCRIPTION: &str = "Plan, and optionally perform, the refactor a template change \
implies for the objects recorded as following it in `.xirang/templates/lineage.tsv`. **The old shape \
is each follower's own file**: the lineage keeps the template version an object followed, and the \
follower's declaration is the only record of what that version left there, so the diff is that file \
against the template's file now. `preset` and `parts` are the fields a template supplies (the same \
two `add` copies into a new face); when a follower still leaves one at its default, copying the \
template's value is mechanical and `apply: true` performs it through the same `edit` path. Everything \
else is **semantic and never written**: a `preset`/`parts` the follower chose itself, a value the \
template dropped, a missing `#[cfg(test)]` entry, a hand-written file (`edit` refuses a file without \
`// generated-by=XiRang`) — each comes back with a paste-ready request or snippet. A follower's \
directory, file name and module name belong to the follower, so no move is ever proposed here; use \
`xirang.apply` with `move`/`rename` for that. A lease that is not in force plans nothing: an \
unconfirmed template version is not a target. With `apply: true` the lineage version advances only \
for a follower whose shape then matches the template; conflicting followers stay on their version so \
the next plan lists them again.";

/// The tool's input schema.
/// 工具的输入 schema。
pub(crate) fn schema() -> Value {
    json!({"type":"object","properties":{
        "anchor":{"type":"string","description":"One template anchor to plan for; omit to plan for every anchor the lineage records"},
        "apply":{"type":"boolean","description":"Perform the mechanical shape copies and advance the lineage of matching followers. Default false: the plan only, nothing written."},
        "root":{"type":"string"}
    }})
}

/// One difference a template must not overwrite.
/// 一处模板不得覆盖的差异。
struct Conflict {
    /// The sentence a reader gets, already naming both values.
    /// 读者拿到的那句话，已经点名两个值。
    sentence: String,
    /// The template's value for the field, when proposing it is meaningful; `None` when the template
    /// dropped the value and keeping or dropping it is a decision rather than a copy.
    /// 该字段的模板值（提议它有意义时）；当模板删去该值时是 `None`——保留还是去掉是决定，不是复制。
    proposed: Option<(String, String)>,
}

/// One follower's plan.
/// 一个跟随者的计划。
struct Follower {
    /// The logical path the lineage records.
    /// 谱系记录的逻辑路径。
    object: String,
    /// The version the lineage records for it.
    /// 谱系为它记录的版本。
    from: String,
    /// The shape fields still at their default that the template now supplies, each `(field, value)`.
    /// 仍未离开默认值、而模板现在提供其值的形状字段，每个 `(字段, 值)`。
    copies: Vec<(String, String)>,
    /// The differences a template must not overwrite.
    /// 模板不得覆盖的那些差异。
    conflicts: Vec<Conflict>,
    /// Conventions the follower does not carry yet, each already a sentence.
    /// 跟随者尚未携带的约定，每条已经是一句话。
    conventions: Vec<String>,
    /// Whether `edit` may write this file at all (the generated marker is its one text contract).
    /// `edit` 究竟能不能写这个文件（生成标记是它唯一的文本契约）。
    writable: bool,
}

/// What one plan did, counted.
/// 一份计划做了什么，按项计数。
#[derive(Default)]
struct Tally {
    /// Followers whose lineage moved to the current version.
    /// 谱系推进到当前版本的跟随者。
    carried: usize,
    /// Followers the plan found behind the current version.
    /// 计划发现落后于当前版本的跟随者。
    behind: usize,
    /// Followers already on the current version.
    /// 已经在当前版本上的跟随者。
    current: usize,
    /// Followers whose object no longer resolves.
    /// 对象已解析不到的跟随者。
    stranded: usize,
}

/// Plan (and optionally perform) the refactor a template change implies.
/// 规划（并可选执行）模板变化所意味着的重构。
pub(crate) fn refactor(root: &Path, arguments: &Value) -> Result<String, String> {
    let apply = arguments.get("apply").and_then(Value::as_bool) == Some(true);
    let namespace = crate::mcp::registry::namespace(root)?;
    let only = arguments.get("anchor").and_then(Value::as_str);
    let anchors = match only {
        Some(only) => vec![only.to_owned()],
        None => {
            let mut anchors = apply_template::lineage_rows(root)
                .into_iter()
                .map(|(_, anchor, _)| anchor)
                .collect::<Vec<_>>();
            anchors.sort();
            anchors.dedup();
            anchors
        }
    };
    if anchors.is_empty() {
        return Ok("no template lineage is recorded: no object follows an approved template yet (or \
                   this tree keeps no `.xirang` state), so there is nothing a template change could \
                   carry\n"
            .to_owned());
    }
    let mut reply = format!(
        "refactor plan for the template lineage in {}\n",
        root.join(xirang_kernel::lexicon::XIRANG_DIR)
            .join("templates/lineage.tsv")
            .display()
    );
    // The boundaries the plan does not cross, said before any per-object line so a reader does not
    // take "no move proposed" for "nothing to do about placement".
    // 计划不越过的边界：在任何逐对象行之前说，免得读者把"没提议搬动"当成"位置问题没东西可做"。
    reply.push_str(
        "note: a follower's directory, file name and module name are its own identity — a template \
         cannot derive them, so no move is proposed here (`xirang.apply {action: move|rename}` is the \
         transacted path when one is needed)\n",
    );
    let mut tally = Tally::default();
    for anchor in anchors {
        let header = format!("template `{anchor}`");
        // 1. Is the version confirmed? An unconfirmed template is not a target.
        // 1. 版本被确认了吗？未确认的模板不是目标。
        match crate::mcp::adopted::anchor_standing(root, &anchor) {
            Ok(AnchorStanding::InForce) => {}
            Ok(standing) => {
                let why = match standing {
                    AnchorStanding::Missing => "no adoption entry names this anchor".to_owned(),
                    AnchorStanding::Lapsed(reason) => format!("its lease lapsed: {reason}"),
                    AnchorStanding::Unreadable(reason) => {
                        format!("its lease cannot be judged: {reason}")
                    }
                    AnchorStanding::InForce => unreachable!("matched above"),
                };
                reply.push_str(&format!(
                    "{header}: nothing is planned — {why}. A refactor carries a **confirmed** \
                     template version; re-confirm the decision (`xirang.adopted` with apply+confirm) \
                     and plan again\n"
                ));
                continue;
            }
            Err(cause) => {
                reply.push_str(&format!(
                    "{header}: the adoption ledger cannot be read, so no version is confirmed — \
                     {cause}\n"
                ));
                continue;
            }
        }
        let Some(version) = apply_template::template_version(root, &anchor) else {
            reply.push_str(&format!(
                "{header}: the ledger has no version for this anchor\n"
            ));
            continue;
        };
        // 2. Does the anchor still name an object? A moved or deleted template leaves its followers
        // pointing at an anchor that cannot be compared with anything.
        // 2. 锚点仍点名一个对象吗？被搬走或删除的模板会让跟随者点着一个无法与任何东西比较的锚点。
        let Some(subject) = resolve_subject(root, &anchor)? else {
            let (present, unresolved) = apply_template::followers_split(root, &anchor);
            let mut named = present
                .into_iter()
                .map(|(object, _)| object)
                .collect::<Vec<_>>();
            named.extend(unresolved);
            tally.stranded += named.len();
            reply.push_str(&format!(
                "{header}: nothing is planned — the anchor names no object in the tree. The object(s) \
                 recorded as following it need a new pointer, and their shape cannot be compared until \
                 then: {}\n",
                if named.is_empty() {
                    "none recorded".to_owned()
                } else {
                    named.join(", ")
                }
            ));
            continue;
        };
        // 3. The template's own shape, read from its file now.
        // 3. 模板自己的形状，从它现在的文件读出。
        let Some((template_file, template_text)) = face_file(root, &namespace, &subject.path)?
        else {
            reply.push_str(&format!(
                "{header}: nothing is planned — the template's own file cannot be read (`{}`)\n",
                subject.path
            ));
            continue;
        };
        let template_shape = apply_template::shape_of_text(&template_text, &subject.path);
        let shape_line = if template_shape.is_empty() {
            "shape: none supplied (both shape fields are at their defaults)".to_owned()
        } else {
            format!(
                "shape: {}",
                template_shape
                    .iter()
                    .map(|(field, value)| format!("`{field}` `{value}`"))
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        };
        reply.push_str(&format!(
            "{header} at {} (lease in force; {shape_line})\n",
            short(&version)
        ));
        // 4. Every follower: current, behind, or no longer an object.
        // 4. 每个跟随者：当前的、落后的，或已经不是一个对象。
        let (followers, unresolved) = apply_template::followers_split(root, &anchor);
        for object in unresolved {
            tally.stranded += 1;
            reply.push_str(&format!(
                "  {object}  (the object no longer resolves; its lineage needs re-pointing, and no \
                 shape is compared for it)\n"
            ));
        }
        for (object, from) in followers {
            if from == version {
                tally.current += 1;
                reply.push_str(&format!(
                    "  {object}  version {} (matches the current shape; nothing to change)\n",
                    short(&from)
                ));
                continue;
            }
            tally.behind += 1;
            let Some((_file, follower_text)) = face_file(root, &namespace, &object)? else {
                tally.stranded += 1;
                reply.push_str(&format!(
                    "  {object}  (the object is recorded but its file cannot be read; its lineage \
                     needs re-pointing)\n"
                ));
                continue;
            };
            let plan = plan_one(&object, &from, &follower_text, &template_text);
            reply.push_str(&format!(
                "  {object}  version {} -> {}\n",
                short(&plan.from),
                short(&version)
            ));
            for (field, value) in &plan.copies {
                if plan.writable {
                    reply.push_str(&format!(
                        "    mechanical: `{field}` `{value}` (a field `add` copies from a template)\n"
                    ));
                    reply.push_str(&format!(
                        "    request {}\n",
                        edit_request(&plan.object, &[(field.clone(), value.clone())])
                    ));
                } else {
                    reply.push_str(&format!(
                        "    mechanical but not writable here: this file does not carry `{}` and \
                         `edit` refuses a file it did not generate\n      paste into the declaration: \
                         .{field}::<{value}>()\n",
                        xirang_kernel::lexicon::GENERATED_MARKER
                    ));
                }
            }
            for conflict in &plan.conflicts {
                reply.push_str(&format!("    conflict: {}\n", conflict.sentence));
                match &conflict.proposed {
                    Some((field, value)) => reply.push_str(&format!(
                        "    not applied: a template does not overwrite a value an object chose; \
                         resolve it with\n      request {}\n",
                        edit_request(&plan.object, &[(field.clone(), value.clone())])
                    )),
                    None => reply.push_str(
                        "    not applied: the template dropped this value, and whether this object \
                         keeps it is a decision — no request is proposed\n",
                    ),
                }
            }
            for convention in &plan.conventions {
                reply.push_str(&format!("    convention: {convention}\n"));
            }
            if plan.copies.is_empty() && plan.conflicts.is_empty() {
                reply.push_str(
                    "    shape: already the template's; only the version differs, so applying this \
                     plan advances the lineage without touching the file\n",
                );
            }
            if !apply {
                continue;
            }
            // 5. Perform the mechanical half, then advance a version only where the bytes now agree.
            // 5. 执行机械的那一半，然后只在字节已经一致的地方推进版本。
            if plan.copies.is_empty() && plan.conflicts.is_empty() {
                advance(
                    root,
                    &plan.object,
                    &anchor,
                    &version,
                    &mut reply,
                    &mut tally,
                )?;
                continue;
            }
            if plan.copies.is_empty() {
                reply.push_str(&format!(
                    "    its lineage stays at {} until that difference is resolved\n",
                    short(&plan.from)
                ));
                continue;
            }
            if !plan.writable {
                reply.push_str(&format!(
                    "    its lineage stays at {} — nothing here can be written through `edit`\n",
                    short(&plan.from)
                ));
                continue;
            }
            match crate::mcp::apply::apply(root, &edit_request(&plan.object, &plan.copies)) {
                Ok(_) => {
                    let after = face_file(root, &namespace, &plan.object)?
                        .map(|(_, text)| apply_template::shape_of_text(&text, &object))
                        .unwrap_or_default();
                    let remaining = shape_differences(&plan.copies, &after, &template_shape);
                    reply.push_str(&format!(
                        "    applied: one edit landed ({})\n",
                        plan.copies
                            .iter()
                            .map(|(field, value)| format!("`{field}` <- `{value}`"))
                            .collect::<Vec<_>>()
                            .join(", ")
                    ));
                    if plan.conflicts.is_empty() && remaining.is_empty() {
                        advance(
                            root,
                            &plan.object,
                            &anchor,
                            &version,
                            &mut reply,
                            &mut tally,
                        )?;
                    } else {
                        reply.push_str(&format!(
                            "    its lineage stays at {} — the file still differs from the template \
                             ({})\n",
                            short(&plan.from),
                            if remaining.is_empty() {
                                "see the conflicts above".to_owned()
                            } else {
                                format!("`{}`", remaining.join("`, `"))
                            }
                        ));
                    }
                }
                Err(error) => {
                    let first = error.lines().next().unwrap_or("(no message)");
                    reply.push_str(&format!(
                        "    refused: the edit did not land — {first}\n    its lineage stays at {}\n",
                        short(&plan.from)
                    ));
                }
            }
        }
        let _ = &template_file;
    }
    reply.push_str(&format!(
        "\n{}{} behind, {} already current, {} stranded\n",
        if apply {
            format!("{} carried; ", tally.carried)
        } else {
            "nothing was written (this is the plan; `apply: true` performs only the mechanical shape \
             copies and advances the lineage of followers that then match); "
                .to_owned()
        },
        tally.behind,
        tally.current,
        tally.stranded
    ));
    Ok(reply)
}

/// A paste-ready `xirang.apply` request that sets exactly the named fields.
/// 一条可粘贴的 `xirang.apply` 请求，只设点名的那些字段。
fn edit_request(object: &str, fields: &[(String, String)]) -> Value {
    let mut map = Map::new();
    for (field, value) in fields {
        map.insert(field.clone(), Value::String(value.clone()));
    }
    json!({"action":"edit","node":object,"fields":map,"apply":true})
}

/// Plan one follower against the template's text.
/// 按模板的文本规划一个跟随者。
fn plan_one(object: &str, from: &str, follower_text: &str, template_text: &str) -> Follower {
    let follower_shape = apply_template::shape_of_text(follower_text, object);
    let template_shape = apply_template::shape_of_text(template_text, object);
    let mut copies = Vec::new();
    let mut conflicts = Vec::new();
    for (field, _default) in apply_template::SHAPE_FIELDS {
        let wanted = template_shape
            .iter()
            .find(|(name, _)| name == field)
            .map(|(_, value)| value.as_str());
        let have = follower_shape
            .iter()
            .find(|(name, _)| name == field)
            .map(|(_, value)| value.as_str());
        match (wanted, have) {
            (Some(wanted), None) => copies.push((field.to_owned(), wanted.to_owned())),
            (Some(wanted), Some(have)) if have == wanted => {}
            (Some(wanted), Some(have)) => conflicts.push(Conflict {
                sentence: format!(
                    "declares `{field}` `{have}` while the template now declares `{wanted}`"
                ),
                proposed: Some((field.to_owned(), wanted.to_owned())),
            }),
            (None, Some(have)) => conflicts.push(Conflict {
                sentence: format!(
                    "declares `{field}` `{have}` while the template now leaves it at the default"
                ),
                proposed: None,
            }),
            (None, None) => {}
        }
    }
    let mut conventions = Vec::new();
    if template_text.contains("#[cfg(test)]") && !follower_text.contains("#[cfg(test)]") {
        conventions.push(
            "the template's file carries a `#[cfg(test)]` entry and this one does not; copy it from \
             the template and adapt its names — this plan does not write a test body it cannot check"
                .to_owned(),
        );
    }
    let writable = follower_text
        .lines()
        .any(|line| line.trim() == xirang_kernel::lexicon::GENERATED_MARKER);
    Follower {
        object: object.to_owned(),
        from: from.to_owned(),
        copies,
        conflicts,
        conventions,
        writable,
    }
}

/// The shape fields that still differ after one edit, named.
/// 一次编辑之后仍然不同的形状字段，逐个点名。
fn shape_differences(
    copies: &[(String, String)],
    after: &[(String, String)],
    template: &[(String, String)],
) -> Vec<String> {
    let mut remaining = Vec::new();
    for (field, _) in copies {
        let now = after
            .iter()
            .find(|(name, _)| name == field)
            .map(|(_, value)| value.as_str());
        let wanted = template
            .iter()
            .find(|(name, _)| name == field)
            .map(|(_, value)| value.as_str());
        if now != wanted {
            remaining.push(field.clone());
        }
    }
    remaining
}

/// Advance one row and say so, or report why the record did not move.
/// 推进一行并说出来，或报告记录为什么没有前进。
fn advance(
    root: &Path,
    object: &str,
    anchor: &str,
    version: &str,
    reply: &mut String,
    tally: &mut Tally,
) -> Result<(), String> {
    match apply_template::advance_version(root, object, anchor, version) {
        Ok(true) => {
            tally.carried += 1;
            reply.push_str(&format!(
                "    advanced the lineage of `{object}` to {}\n",
                short(version)
            ));
        }
        Ok(false) => reply.push_str(&format!(
            "    the lineage has no row for `{object}` and `{anchor}`, so nothing was advanced\n"
        )),
        Err(error) => reply.push_str(&format!(
            "    the lineage could not be advanced: {error} (the bytes are unaffected; the next plan \
             lists this object again)\n"
        )),
    }
    Ok(())
}

/// One face's file and text, from the tree that is being read.
/// 一个面的文件与文本，读自正在被读取的那棵树。
fn face_file(
    work: &Path,
    namespace: &str,
    path: &str,
) -> Result<Option<(PathBuf, String)>, String> {
    let views = crate::build_method::face_views(work, namespace)?;
    let Some(view) = views.iter().find(|view| view.path == path) else {
        return Ok(None);
    };
    let layout = crate::build_method::source_layout(work)?;
    let file = view.locate_source(&layout)?;
    let text = std::fs::read_to_string(&file)
        .map_err(|error| format!("cannot read `{}`: {error}", file.display()))?;
    Ok(Some((file, text)))
}

/// The first eight characters of a fingerprint, as a reader compares two of them.
/// 指纹的前八个字符，读者正是这样比较两个的。
fn short(value: &str) -> &str {
    value.get(..8).unwrap_or(value)
}

#[cfg(test)]
#[path = "refactor_tests.rs"]
mod refactor_tests;
