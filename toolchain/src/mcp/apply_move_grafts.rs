//! Refuse relocation that would silently change a graft's target or membership.
//! 拒绝会静默改变嫁接目标或覆盖成员的搬迁。
//! online: current declarations and machine-local records must be checked before the filesystem move.

use std::path::{Path, PathBuf};

use crate::build_method::{DeclaredGraft, FaceView};
use crate::run_method::{LoadedGraft, load_graft_records};

use super::{Moved, directory_of};

#[path = "apply_move_declarations.rs"]
mod declarations;

pub(super) use self::declarations::apply_declaration_edits_in;

/// Inspect the whole moved source subtree and both sides of coverage changes before writing.
/// 写入前检查被搬的整个源码子树，以及覆盖范围变化的两端。
pub(super) fn refuse_affected_grafts(
    root: &Path,
    work: &Path,
    views: &[FaceView],
    face: &FaceView,
    moved: &Moved,
    new_path: &str,
    rebind: bool,
) -> Result<Vec<String>, String> {
    // `rebind` is the caller asking this move to carry the **determined** records with it: the
    // mapping is computable, so those are listed for rewriting instead of blocking. Everything else
    // still blocks — `ambiguous` and `coverage` are decisions, not computations (W5/A21).
    // `rebind` 是调用方要求这次搬迁把**确定的**那些记录一起带走：映射算得出来，因此它们只被列出待改写、
    // 不阻塞。其余照旧阻塞——`ambiguous` 与 `coverage` 是决定，不是计算（W5/A21）。
    let mut follows: Vec<String> = Vec::new();
    let mut deferred: Vec<String> = Vec::new();
    let affected: Vec<_> = views
        .iter()
        .filter(|view| Path::new(&view.source).starts_with(&moved.from_dir))
        .collect();
    let declared = crate::build_method::declared_grafts(work).map_err(|error| {
        format!(
            "cannot verify graft impact before move: {error}; repair the host declaration first"
        )
    })?;
    let mut impact = Vec::new();
    for cut in &declared.cuts {
        let targets: Vec<_> = affected
            .iter()
            .filter(|view| cut.names_face(&view.path, Some(&view.module)))
            .map(|view| view.path.as_str())
            .collect();
        let scope_changed = coverage_change(cut, views, face, new_path)?.map(|c| c.describe());
        if !targets.is_empty() || scope_changed.is_some() {
            // A21: the two questions are separate, so the report keeps them separate. **Mapping**
            // determined means one moved object is named and the repair map computes its new
            // reference; **intent** is never determined here — following the object and staying put
            // to pick up whatever now sits there are both legitimate, and only a person picks.
            // A21：两个问题各自独立，报告也把它们分开。**映射**确定＝只点名了一个被搬对象，修复映射能算出
            // 它的新引用；**意图**在这里永远不确定——跟着对象走、与留在原地接住现在那里的东西，两者都合法，
            // 只有人能选。
            let tag = match targets.len() {
                0 => "coverage",
                1 => "determined",
                _ => "ambiguous",
            };
            let line = format!(
                "[{tag}] {}:{} graft `{}`: {}{}",
                declared.entry.display(),
                cut.line,
                cut.cut_label(),
                targets.join(", "),
                scope_changed.unwrap_or_default()
            );
            if rebind && tag == "determined" {
                follows.push(line);
            } else {
                impact.push(line);
            }
        }
        // Both ends are references: a local replacement can move without its cut moving.
        // 两端都是引用：本地替换实现可以被搬走，而切口本身不动。
        if let Some(expressions) = &cut.expressions {
            let mut implementation = cut.clone();
            implementation.cut = expressions.graft.clone();
            implementation.cut_end = None;
            let paths = implementation.expressions.as_mut().unwrap();
            paths.cut = paths.graft.clone();
            paths.cut_end = None;
            for view in &affected {
                if implementation.names_face(&view.path, Some(&view.module)) {
                    impact.push(format!("[determined] {}:{} graft implementation `{}` moves with {}; rebind the implementation reference", declared.entry.display(), cut.line, expressions.graft, view.path));
                }
            }
        }
    }
    // A preview omits .xirang from its copy. Records must come from the real project.
    // 预览副本不带 .xirang；记录必须来自真实项目。
    for loaded in load_graft_records(root)? {
        match loaded {
            LoadedGraft::Record(record) => {
                let by_id = views.iter().find(|view| view.id == record.document.target);
                let by_path = views
                    .iter()
                    .find(|view| view.path == record.document.target_path);
                if let (Some(by_id), Some(by_path)) = (by_id, by_path)
                    && by_id.id != by_path.id
                {
                    return Err(format!(
                        "cannot verify graft impact: record `{}` target and target_path name different objects; repair that record before moving",
                        record.selector
                    ));
                }
                let resolved = by_id.or(by_path);
                if let Some(target) = affected
                    .iter()
                    .find(|view| resolved.is_some_and(|target| view.id == target.id))
                {
                    let path = relocated_path(&target.path, &face.path, new_path);
                    let line = format!(
                        "[determined] .xirang/external-grafts/{}/graft.plan: {} -> {path}; update target AND target_path together after reviewing the move",
                        record.selector, target.path
                    );
                    if rebind {
                        // The writer below moves both fields together, so the caller gets the
                        // rewrite rather than a note (W5).
                        // 下面的写入器把两个字段一起改，因此调用方得到的是改写本身而不是一条提醒。
                        follows.push(line);
                    } else {
                        impact.push(line);
                    }
                } else if record.document.full
                    && resolved.map_or_else(
                        || record.document.target_path == "root",
                        |target| {
                            under_path(&face.path, &target.path)
                                || under_path(new_path, &target.path)
                        },
                    )
                {
                    deferred.push(format!(
                        "[coverage] .xirang/external-grafts/{}/graft.plan: full graft membership \
                         changes",
                        record.selector
                    ));
                }
            }
            LoadedGraft::Unreadable { selector, reason } => {
                return Err(format!(
                    "cannot verify graft impact: record `{selector}` unreadable: {reason}; repair that record before moving"
                ));
            }
        }
    }
    if impact.is_empty() {
        if !follows.is_empty() {
            return Ok(follows);
        }
        if !deferred.is_empty() {
            return Err(format!(
                "move refused before writing: {} membership change(s) are a decision, not a \
                 computation (pass `rebind_grafts` only settles the determined ones)\n{}",
                deferred.len(),
                deferred.join("\n")
            ));
        }
        return Ok(Vec::new());
    }
    let mut report = format!(
        "move refused before writing: graft targets or coverage would change\nimpact tags:          determined = exactly one moved object is named, so the repair map below computes its new          reference; ambiguous = several moved objects are named, so which one a graft follows is          your decision; coverage = the member set changes without naming a moved object\n{}\n         repair map (only moved source objects):\n",
        impact.join("\n")
    );
    for view in affected.iter().take(40) {
        let source = if view.id == face.id {
            moved.source.clone()
        } else {
            let rest = Path::new(&view.source)
                .strip_prefix(&moved.from_dir)
                .unwrap();
            Path::new(&moved.to_dir)
                .join(rest)
                .to_string_lossy()
                .into_owned()
        };
        let module = directory_of(&source).replace('/', "::");
        report.push_str(&format!(
            "  {} -> {}; crate::{}::NODE_ID -> crate::{module}::NODE_ID\n",
            view.path,
            relocated_path(&view.path, &face.path, new_path),
            view.module
        ));
    }
    if affected.len() > 40 {
        report.push_str(&format!(
            "  {} additional moved objects omitted; inspect the subtree before repair\n",
            affected.len() - 40
        ));
    }
    report.push_str("repair sequence: retain the original graft entries/records, detach affected declarations and move their entire record directories outside .xirang/external-grafts (not just graft.plan), move, rebind those same entries using this map and the new registry identities, then check/build and replay the graft. Preserve replacement selectors, full flags and unaffected entries. Ranges/full grafts require reviewing the intended member set; replacing them by guessed targets is not automatic. No automatic repair has been applied.\n");
    Err(report)
}

fn under_path(path: &str, ancestor: &str) -> bool {
    path == ancestor
        || path
            .strip_prefix(ancestor)
            .is_some_and(|rest| rest.starts_with('/'))
}

fn relocated_path(path: &str, from: &str, to: &str) -> String {
    path.strip_prefix(from)
        .filter(|rest| rest.is_empty() || rest.starts_with('/'))
        .map_or_else(|| path.to_owned(), |rest| format!("{to}{rest}"))
}

fn endpoint<'a>(cut: &DeclaredGraft, views: &'a [FaceView], end: bool) -> Option<&'a FaceView> {
    let mut single = cut.clone();
    if end {
        single.cut = single.cut_end.clone()?;
        if let Some(expressions) = &mut single.expressions {
            expressions.cut = expressions.cut_end.clone()?;
        }
    }
    single.cut_end = None;
    if let Some(expressions) = &mut single.expressions {
        expressions.cut_end = None;
    }
    views
        .iter()
        .find(|view| single.names_face(&view.path, Some(&view.module)))
}

/// What a coverage change does to the one object being moved, in the two states a move has.
/// 一次覆盖变化对被搬的那一个对象做了什么，按一次搬迁的两个状态分别给出。
///
/// The two booleans are the point (A22): "the member set changes" is a conclusion, and a reader who
/// has to decide whether the graft should follow needs the **two states** it was drawn from.
/// 这两个布尔值就是要点（A22）："成员集合变了"是一个结论，而必须决定这个嫁接该不该跟着走的读者，需要的是
/// 得出结论所依据的**那两个状态**。
pub(super) struct Coverage {
    /// `full` or `range`.
    /// `full` 或 `range`。
    pub kind: &'static str,
    /// The cut's own spelling, so the reader can find the declaration.
    /// 切口自己的拼法，读者据此找到那条声明。
    pub scope: String,
    /// Whether the moved object is inside the cut **before** the move.
    /// 被搬对象在搬迁**之前**是否在切口之内。
    pub before: bool,
    /// …and after it.
    /// ……以及搬迁之后。
    pub after: bool,
}

impl Coverage {
    /// One clause naming both states, appended to the impact line.
    /// 一句话给出两个状态，接在影响行后面。
    pub(super) fn describe(&self) -> String {
        format!(
            " {} graft on `{}` membership for this object: inside before = {}, after = {}",
            self.kind, self.scope, self.before, self.after
        )
    }
}

fn coverage_change(
    cut: &DeclaredGraft,
    views: &[FaceView],
    face: &FaceView,
    new_path: &str,
) -> Result<Option<Coverage>, String> {
    let start = endpoint(cut, views, false);
    let root_cut = matches!(
        cut.cut.as_str(),
        "root" | "crate::ROOT_NODE_ID" | "ROOT_NODE_ID"
    );
    if start.is_none() && !root_cut {
        return Err(format!(
            "cannot verify graft `{}` before move: unresolved endpoint; repair that declaration first",
            cut.cut_label()
        ));
    }
    if cut.full
        && (root_cut
            || start.is_some_and(|start| {
                under_path(&face.path, &start.path) || under_path(new_path, &start.path)
            }))
    {
        let (before, after) = start.map_or((root_cut, root_cut), |start| {
            (
                under_path(&face.path, &start.path),
                under_path(new_path, &start.path),
            )
        });
        return Ok(Some(Coverage {
            kind: "full",
            scope: cut.cut_label(),
            before,
            after,
        }));
    }
    if cut.cut_end.is_none() {
        return Ok(None);
    }
    let end = endpoint(cut, views, true);
    let (Some(start), Some(end)) = (start, end) else {
        return Err(format!(
            "cannot verify range graft `{}` before move: unresolved endpoint; repair that declaration first",
            cut.cut_label()
        ));
    };
    if start.parent != end.parent || start.registry_name > end.registry_name {
        return Err(format!(
            "cannot verify range graft `{}` before move: invalid sibling range",
            cut.cut_label()
        ));
    }
    let old_member = face.parent == start.parent
        && face.registry_name >= start.registry_name
        && face.registry_name <= end.registry_name;
    let (new_parent, new_name) = new_path.rsplit_once('/').unwrap_or(("root", new_path));
    let range_parent = start
        .path
        .rsplit_once('/')
        .map_or("root", |(parent, _)| parent);
    let new_member = new_parent == range_parent
        && new_name >= start.registry_name.as_str()
        && new_name <= end.registry_name.as_str();
    Ok((old_member || new_member).then_some(Coverage {
        kind: "range",
        scope: cut.cut_label(),
        before: old_member,
        after: new_member,
    }))
}

/// One graft record this move rewrites, planned but **not yet written**.
/// 这次搬迁要改写的一条嫁接记录，已规划但**尚未写入**。
///
/// Planning is separate from writing for one reason: the record belongs to the same transaction as the
/// moved bytes. A record rewritten outside it would survive a later failure and name an identity the
/// tree no longer has.
/// 规划与写入分开只有一个理由：记录与那些被搬的字节属于同一个事务。写在事务之外的记录会活过一次后来的
/// 失败，并点名一个树里已经不存在的身份。
pub(super) struct RecordRebind {
    /// The record file on disk.
    /// 磁盘上的记录文件。
    pub(super) path: std::path::PathBuf,
    /// Its new text.
    /// 它的新文本。
    pub(super) text: String,
    /// The line the reply prints.
    /// 回复打印的那一行。
    pub(super) description: String,
}

/// Plan the records a move determines: `target` and `target_path` move **together**.
/// 规划这次搬迁所确定的那些记录：`target` 与 `target_path` **一起**改。
///
/// Only an identity whose old id **and** old path both name the record's target qualifies, so a
/// record whose two fields disagree is never silently half-rewritten — that shape is refused by the
/// detector before the move, and re-checked here. A record whose target did not move is left alone.
/// 只有"旧身份**与**旧路径同时命中记录的目标"的那条映射才算数，因此两个字段互相矛盾的记录绝不会被静默
/// 改掉一半——那种形状在搬迁之前就被检测拒绝，这里再查一次。目标没动的记录原样不动。
pub(super) fn plan_rebind_records(
    root: &Path,
    identities: &[crate::mcp::apply::apply_move::references::FaceIdentity],
) -> Result<Vec<RecordRebind>, String> {
    let records = load_graft_records(root)?;
    let mut planned = Vec::new();
    for loaded in records {
        let LoadedGraft::Record(record) = loaded else {
            continue;
        };
        let matches: Vec<_> = identities
            .iter()
            .filter(|identity| {
                identity.old_id == record.document.target
                    || identity.old_path == record.document.target_path
            })
            .collect();
        let Some(identity) = matches
            .iter()
            .find(|identity| {
                identity.old_id == record.document.target
                    && identity.old_path == record.document.target_path
            })
            .copied()
        else {
            if matches.is_empty() {
                continue;
            }
            return Err(format!(
                "cannot rebind record `{}`: its target and target_path do not name the same moved \
                 object; repair that record instead of letting a move guess",
                record.selector
            ));
        };
        let path = crate::run_method::graft_record_root(root)
            .join(&record.selector)
            .join("graft.plan");
        let document = crate::run_method::GraftPlanDocument::new(
            identity.new_id,
            identity.new_path.clone(),
            record.document.graft.clone(),
            record.document.full,
        );
        planned.push(RecordRebind {
            path,
            text: document.render_graft_plan_document(),
            description: format!(
                "rebound {}: {} -> {} (target and target_path together)",
                record.selector, identity.old_path, identity.new_path
            ),
        });
    }
    Ok(planned)
}

/// Refuse a move whose face is named by a typed cut in the host's graft plan.
/// 拒绝一次其面被宿主嫁接计划里的类型化切口点名的搬动。
///
/// Rewriting a typed cut that points at a face declared in another language needs that language's
/// Rust reference spelling; until that is planned here, the honest answer is to name the entry and
/// line instead of moving the face and leaving the cut stale.
/// 改写一条指向外语声明面的类型化切口，需要那门语言的 Rust 引用拼写；在把它规划进来之前，诚实做法是
/// 点名入口与行号，而不是搬走面、把切口留在过期状态。
pub(super) fn plan_foreign_declarations(
    work: &Path,
    face: &FaceView,
    old_reference: &str,
    new_reference: &str,
    new_path: &str,
    rebind: bool,
) -> Result<Vec<DeclarationEdit>, String> {
    let declared = crate::build_method::declared_grafts(work)?;
    let mut edits = Vec::new();
    for cut in &declared.cuts {
        let typed = cut.names_reference(old_reference);
        // A string cut names the logical path instead; both spellings are invisible to the merge
        // that plans Rust declarations, so both have to be handled here.
        // 字符串切口点的是逻辑路径；两种拼写对规划 Rust 声明的合并都不可见，因此都要在这里处理。
        let string = !typed && cut.names_face(&face.path, Some(&face.module));
        if !typed && !string {
            continue;
        }
        // A range names two endpoints; replacing one of them would guess which side the author
        // meant, so it stays a hand edit even when the caller allows rebinding.
        // 区间切口点名两个端点；只改其中一个等于替作者猜，因此即使调用方允许重绑也留给手改。
        let range = cut.cut_end.is_some()
            || cut
                .expressions
                .as_ref()
                .is_some_and(|expressions| expressions.cut_end.is_some());
        if range {
            return Err(format!(
                "{}:{} names `{}` as one end of a graft range; rewriting a range that points at \
                 a foreign face is not wired yet — change that entry by hand. Nothing changed.",
                declared.entry.display(),
                cut.line,
                face.path
            ));
        }
        if !rebind {
            let spelling = if typed { old_reference } else { &face.path };
            return Err(format!(
                "{}:{} names `{}` with a graft entry (`{spelling}`); the move changes that face's \
                 identity, so the entry would stop resolving. Pass `rebind_grafts: true` to carry \
                 it, or repair the entry by hand. Nothing changed.",
                declared.entry.display(),
                cut.line,
                face.path
            ));
        }
        let (before, after) = if typed {
            let Some(expressions) = &cut.expressions else {
                continue;
            };
            (expressions.cut.clone(), new_reference.to_owned())
        } else {
            (format!("\"{}\"", face.path), format!("\"{new_path}\""))
        };
        if before == after {
            continue;
        }
        edits.push(DeclarationEdit {
            file: declared.entry.clone(),
            line: cut.line,
            before,
            after,
            span: None,
            verify: None,
            verify_absent: None,
        });
    }
    Ok(edits)
}

/// One authoritative rewrite of a graft declaration that the move determines.
/// 一条由这次搬迁确定的、权威的嫁接声明改写。
///
/// A line plus a substring was not enough: the parser normalizes both accepted spellings to the
/// declaration form, so a catalog reference could not be found on the line it was written, and two
/// references on one line had no address. The edit now carries the **source range** the parser
/// reported and the exact old text in it; applying verifies the range still says that, and the file
/// is re-parsed afterwards to confirm the new endpoints resolve to the expected object.
/// 只有行号加子串不够：解析器把两种可接受拼写都归一成声明形式，因此按行找不到作者写下的目录拼写，
/// 同一行的两处引用也没有地址。现在编辑携带解析器报告的**源码范围**与其中的原文；应用时核对范围仍然
/// 写着那段文字，改完再解析整个文件，确认新端点解析到预期对象。
pub(super) struct DeclarationEdit {
    /// The entry file the declaration lives in.
    /// 声明所在的入口文件。
    pub file: PathBuf,
    /// 1-based line, as the parser reported it.
    /// 1 起始行号，按解析器报告的样子。
    pub line: usize,
    /// The exact text to find on that line (or in `span`).
    /// 要在那一行（或 `span` 中）找到的原文。
    pub before: String,
    /// What it becomes.
    /// 它变成什么。
    pub after: String,
    /// Byte range of the endpoint in the file, when the parser reported one.
    /// 端点在文件中的字节范围（解析器报告时）。
    pub span: Option<(usize, usize)>,
    /// The normalized endpoint the rewritten file must resolve to.
    /// 改写后的文件必须解析到的那条归一化端点。
    pub verify: Option<String>,
    /// A normalized endpoint the rewritten file must no longer contain.
    /// 改写后的文件不得再含有的那条归一化端点。
    pub verify_absent: Option<String>,
}

/// Plan the declaration rewrites a move determines, for **both** spellings of a cut.
/// 规划这次搬迁所确定的声明改写，覆盖切口的**两种**拼法。
///
/// The spelling decides only which text on the line changes: a logical path is a quoted literal, a
/// typed cut is a Rust expression. Either way the new text comes from the same place the repair map
/// already prints — the moved object's new logical path and its new module — so a plan written here
/// and a plan printed in the refusal can never disagree.
/// 拼法只决定那一行里改哪段文字：逻辑路径是带引号的字面量，类型化切口是 Rust 表达式。两者的新文字都来自
/// 修复映射已经在印的同一个地方——被搬对象的新逻辑路径与新模块——因此这里写下的计划与拒绝里印出的计划
/// 不可能互相矛盾。
///
/// A declaration naming **several** moved objects is skipped, not guessed: the detector refuses those,
/// and picking one would be choosing the author's intent for them.
/// 点名了**多个**被搬对象的声明被跳过而不是猜：检测器会拒绝它们，挑一个等于替作者决定他的意图。
pub(super) fn plan_declaration_edits(
    work: &Path,
    views: &[FaceView],
    face: &FaceView,
    moved: &Moved,
    new_path: &str,
    rebind: bool,
) -> Result<Vec<DeclarationEdit>, String> {
    let declared = crate::build_method::declared_grafts(work)?;
    let entry = declared.entry.clone();
    if Path::new(&entry)
        .to_string_lossy()
        .contains(&moved.from_dir)
    {
        return Err(format!(
            "cannot rebind the graft plan at {}: it lives inside the subtree being moved; move the \
             plan out first, or rebind it by hand",
            entry.display()
        ));
    }
    let affected: Vec<_> = views
        .iter()
        .filter(|view| Path::new(&view.source).starts_with(&moved.from_dir))
        .collect();
    let mut edits = Vec::new();
    if !rebind {
        return Ok(edits);
    }
    let layout = crate::build_method::source_layout(work)
        .map_err(|error| format!("cannot read this package's layout: {error}"))?;
    let text = std::fs::read_to_string(&entry)
        .map_err(|error| format!("cannot read {}: {error}", entry.display()))?;
    edits.extend(super::rebind::plan_typed_edits(
        &entry,
        &text,
        &declared.cuts,
        &affected,
        face,
        moved,
        &layout,
    )?);
    for cut in &declared.cuts {
        // Typed endpoints were planned from their source spans; only the string form is line-scoped.
        // 类型化端点已按源码跨度规划；只有字符串形式按行处理。
        if cut.expressions.is_some() {
            continue;
        }
        let targets: Vec<_> = affected
            .iter()
            .filter(|view| cut.names_face(&view.path, Some(&view.module)))
            .collect();
        let [target] = targets[..] else {
            continue;
        };
        let Ok(rest) = Path::new(&target.source).strip_prefix(&moved.from_dir) else {
            continue;
        };
        let new_source = Path::new(&moved.to_dir)
            .join(rest)
            .to_string_lossy()
            .into_owned();
        let new_logical = relocated_path(&target.path, &face.path, new_path);
        let (before, after) = match &cut.expressions {
            None => (format!("\"{}\"", cut.cut), format!("\"{new_logical}\"")),
            Some(expressions) => (
                expressions.cut.clone(),
                format!(
                    "crate::{}::NODE_ID",
                    directory_of(&new_source).replace('/', "::")
                ),
            ),
        };
        edits.push(DeclarationEdit {
            file: entry.clone(),
            line: cut.line,
            before,
            after,
            span: None,
            verify: None,
            verify_absent: None,
        });
    }
    Ok(edits)
}

/// The sibling packages of a workspace, as directories, excluding the one being moved in.
/// 一个工作区的兄弟包（目录形式），不含正在搬迁的那一个。
///
/// Read from the root manifest's `[workspace] members` and expanded one level (`crates/*`), which is
/// the shape this workspace and its fixtures use. A member that does not resolve as a host is the
/// caller's business to tolerate, not this function's: plenty of members declare no grafts at all.
/// 从根清单的 `[workspace] members` 读出并展开一层（`crates/*`），也就是本工作区与它的夹具使用的形状。
/// 解析不成宿主的成员由调用方容错，而不是这里：很多成员根本不声明任何嫁接。
pub(super) fn workspace_siblings(root: &Path, work: &Path) -> Vec<PathBuf> {
    let Ok(manifest) = std::fs::read_to_string(root.join("Cargo.toml")) else {
        return Vec::new();
    };
    let mut siblings = Vec::new();
    let mut in_workspace = false;
    let mut collecting = false;
    let mut listed = String::new();
    for line in manifest.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            in_workspace = trimmed == "[workspace]";
            continue;
        }
        if !in_workspace {
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("members") {
            collecting = true;
            listed.push_str(rest.trim_start_matches([' ', '=']).trim());
        } else if collecting {
            listed.push_str(trimmed);
        }
        if collecting && trimmed.contains(']') {
            collecting = false;
        }
    }
    let listed = listed.trim().trim_start_matches('[').trim_end_matches(']');
    for entry in listed.split(',') {
        let entry = entry.trim().trim_matches(['"', '\'']).trim();
        if entry.is_empty() {
            continue;
        }
        let candidates: Vec<PathBuf> = if let Some(prefix) = entry.strip_suffix("/*") {
            let Ok(entries) = std::fs::read_dir(root.join(prefix)) else {
                continue;
            };
            entries
                .flatten()
                .map(|entry| entry.path())
                .filter(|path| path.join("Cargo.toml").is_file())
                .collect()
        } else {
            vec![root.join(entry)]
        };
        for candidate in candidates {
            if candidate.join("Cargo.toml").is_file() && candidate != work {
                siblings.push(candidate);
            }
        }
    }
    siblings.sort();
    siblings.dedup();
    siblings
}

/// Plan the declaration rewrites of every **other** package in the workspace.
/// 规划工作区里**其它**每个包的声明改写。
///
/// One move changes an identity every package that names it has to follow, and a consumer's plan is
/// a separate entry file (`declared_grafts` answers for one root). A member whose plan cannot be read
/// is skipped rather than failing the move: most members declare no grafts, and refusing on their
/// behalf would make the tool unusable in any real workspace.
/// 一次搬迁改变的是一份身份，凡是点名它的包都要跟着走，而消费方的计划是另一个入口文件
/// （`declared_grafts` 只为某一个根作答）。读不出计划的成员跳过而不是让搬迁失败：绝大多数成员不声明任何
/// 嫁接，替它们拒绝会让这个工具在任何真实工作区里都用不了。
pub(super) fn plan_sibling_edits(
    root: &Path,
    work: &Path,
    views: &[FaceView],
    face: &FaceView,
    moved: &Moved,
    new_path: &str,
    rebind: bool,
) -> Result<Vec<DeclarationEdit>, String> {
    let mut edits = Vec::new();
    if !rebind {
        return Ok(edits);
    }
    for sibling in workspace_siblings(root, work) {
        let Ok(mut planned) =
            plan_declaration_edits(&sibling, views, face, moved, new_path, rebind)
        else {
            continue;
        };
        edits.append(&mut planned);
    }
    Ok(edits)
}
