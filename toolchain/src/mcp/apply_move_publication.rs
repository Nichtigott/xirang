//! What a Rust move publishes, planned **before** the move so its paths can be authorized with the
//! rest of the write set.
//! Rust 搬动发布什么——在搬动**之前**规划，使它的路径能与其余写入集合一起被授权。
//!
//! Publication used to be planned inside the write closure, which made it the one part of the write
//! set nobody authorized: with `.xirang` materialized as a declared-source link, the migration record
//! landed outside the package while the reply said the move succeeded (independent review, high).
//! 发布过去在写入闭包里规划，于是它成了写入集合里唯一没人授权的那部分：把 `.xirang` 物化成已声明来源
//! 链接时，迁移记录落到包外，而回复说搬动成功（独立复核，high）。

use std::path::{Path, PathBuf};

use xirang_kernel::identity::NodeId;
use xirang_multilang::object_map::Observed;

use crate::mcp::apply::apply_template;
use crate::mcp::rules;
use crate::run_method::authoring::write_scope::ChangeKind;

use super::grafts::{self, DeclarationEdit};
use super::record::{RecordPlan, plan_record};
use super::references::FaceIdentity;

/// Everything a Rust move publishes, plus the paths the authorization pass needs.
/// Rust 搬动发布的全部内容，以及授权那一趟需要的路径。
pub(super) struct PublicationPlan {
    /// The object ledger entries, in both modes (a preview shows them in the diff). The root comes
    /// first; descendants that already had an entry follow their object too.
    /// 台账条目，两种模式都写（预览要在 diff 里看到它们）。根在前；已经有条目的后代也跟随它们的对象。
    pub(super) ledgers: Vec<crate::object_ledger::LedgerWrite>,
    /// The migration record, when this is an apply.
    /// 迁移记录（当这是一次落盘时）。
    pub(super) record: Option<RecordPlan>,
    /// The graft records that follow the moved object.
    /// 跟随被搬对象的嫁接记录。
    pub(super) graft_records: Vec<grafts::RecordRebind>,
    /// The rules that follow the moved object: rewritten in the same transaction, because a rule
    /// bound to the object must keep naming it after the move.
    /// 跟随被搬对象的规则：在同一个事务里改写，因为绑定到该对象的规则必须在搬动后仍点名它。
    pub(super) rule_writes: Vec<rules::RuleWrite>,
    /// Rules that could not follow because the move re-derived their object's identity. They are
    /// reported, not rewritten: silently rebinding would transfer the commitment to another object.
    /// 因搬动重新推导了对象身份而跟不过去的规则。它们被**报告**而不是改写：静默换绑会把承诺转给另一个对象。
    pub(super) stranded_rules: Vec<String>,
    /// The template lineage this move carries, plus the template anchors it strands. Planned for both
    /// modes so the preview names the same carry the write performs.
    /// 这次搬动带走的模板谱系，以及它留下的模板锚点。两种模式都规划，因此预览点名的正是落盘会做的搬运。
    pub(super) lineage: Option<apply_template::LineageRewrite>,
    /// The publication half of the write set: `(path, kind, subject)`.
    /// 写入集合的发布那一半：`(路径, 种类, 拒绝时的称呼)`。
    pub(super) paths: Vec<(PathBuf, ChangeKind, String)>,
}

/// Plan a Rust move's publication.
/// 规划一次 Rust 搬动的发布。
#[allow(clippy::too_many_arguments)]
pub(super) fn plan(
    work: &Path,
    root: &Path,
    applying: bool,
    rebind: bool,
    face_path: &str,
    new_path: &str,
    old_id: NodeId,
    new_id: NodeId,
    old_observed: &Observed,
    new_observed: &Observed,
    keeps_mapping: bool,
    namespace: &str,
    identities: &[FaceIdentity],
    declaration_edits: &[DeclarationEdit],
    sibling_edits: &[DeclarationEdit],
) -> Result<PublicationPlan, String> {
    use xirang_kernel::identity::StableFaceId;
    // Every object that follows this move: the root, plus descendants that already have a ledger
    // entry. They are planned **together** — one read, one rendered file — because each plan carries
    // the whole ledger and publishing them one by one let the last writer roll the others back.
    // 跟随这次搬动的一切对象：根，加上已经有台账条目的后代。它们**一起**规划——一次读、一份渲染——
    // 因为每份计划都带整份台账，逐个发布会让最后一份把其余的回滚掉。
    let mut batch: Vec<(Observed, NodeId, Observed, NodeId, bool)> = vec![(
        old_observed.clone(),
        old_id,
        new_observed.clone(),
        new_id,
        keeps_mapping,
    )];
    let map = xirang_multilang::object_map::ObjectMap::read_from(work)?;
    for identity in identities {
        if identity.old_id == old_id {
            continue;
        }
        let seed = StableFaceId::from_namespaced_name(namespace, &identity.old_path);
        let before = Observed::new(seed, identity.old_path.clone(), identity.old_source.clone());
        let Some(stable) = map.own(&before)?.map(|entry| entry.stable) else {
            continue;
        };
        let after = Observed::new(
            stable,
            identity.new_path.clone(),
            identity.new_source.clone(),
        );
        // A destination under the walk root is covered by default discovery even when no mapping
        // names it; only the mapped-directory test alone would keep a now-unnecessary deviation.
        // 落在遍历根之下的目的地由默认发现覆盖，即使没有映射点名它；只看映射会把一条不再需要的偏离留下。
        let layout = crate::build_method::source_layout(work)?;
        let prefix = layout
            .scan_root
            .strip_prefix(&layout.package_root)
            .map(|prefix| xirang_kernel::declaration::portable_path(&prefix.to_string_lossy()))
            .unwrap_or_default();
        let under_walk = prefix.is_empty()
            || identity.new_source == prefix
            || identity.new_source.starts_with(&format!("{prefix}/"));
        let covered = under_walk
            || crate::object_ledger::covering_directory(work, &identity.new_source)?.is_some();
        batch.push((before, identity.old_id, after, identity.new_id, !covered));
    }
    let records: Vec<crate::object_ledger::MoveRecord<'_>> = batch
        .iter()
        .map(
            |(before, before_id, after, after_id, keeps)| crate::object_ledger::MoveRecord {
                before,
                before_id: *before_id,
                after,
                after_id: *after_id,
                keeps_mapping: *keeps,
            },
        )
        .collect();
    let ledgers: Vec<crate::object_ledger::LedgerWrite> =
        crate::object_ledger::plan_moves(work, &records)?
            .into_iter()
            .collect();
    // The rules a move carries: matched by **object identity** (the ledger's stable id wins over
    // the derived seed), so a rule about this object keeps applying after the move, and a rule that
    // merely spells the old path is bound and rewritten only because the move itself determines the
    // mapping.
    // 搬动带走的规则：按**对象身份**匹配（台账的稳定 id 胜过推导种子），因此关于这个对象的规则在搬动后继续
    // 适用；而仅仅写着旧路径的规则只有在搬动本身就确定了映射时才被绑定并改写。
    // Rules are matched over **every** face this move relocates, not only the ledger-recorded ones:
    // a descendant whose identity the move re-derives still has a rule that must be reported as left
    // behind. The ledger batch above is the set this tool can *keep* identities for; this set is the
    // one knowledge may have commitments about.
    // 规则在**每一个**被搬动的面上匹配，而不只是台账记录的那些：一个身份被搬动重新推导的后代，仍然有
    // 一条必须被报成"留在原处"的规则。上面的台账批次是本工具**能保持**身份的那一组；这一组才是知识可能
    // 对它作出承诺的那一组。
    struct Relocation {
        before: xirang_multilang::object_map::Observed,
        after: xirang_multilang::object_map::Observed,
        old_stable: StableFaceId,
        stable: StableFaceId,
    }
    let views = crate::build_method::face_views(work, namespace).unwrap_or_default();
    let view_for = |path: &str| views.iter().find(|view| view.path == path);
    let mut relocations = vec![Relocation {
        before: old_observed.clone(),
        after: new_observed.clone(),
        old_stable: old_observed.stable,
        stable: new_observed.stable,
    }];
    for identity in identities {
        if identity.old_id == old_id {
            continue;
        }
        let old_view = view_for(&identity.old_path);
        let seed = old_view
            .map(|view| view.object_id(namespace))
            .unwrap_or_else(|| StableFaceId::from_namespaced_name(namespace, &identity.old_path));
        let before = Observed::new(seed, identity.old_path.clone(), identity.old_source.clone());
        let preserved = map.own(&before)?.map(|entry| entry.stable);
        let old_stable = preserved.unwrap_or(seed);
        // The object keeps its identity when the ledger recorded it, or when its declaration opted
        // into an author-owned name (`stable_name`) — a path-derived identity is re-derived by the
        // move, and that is the difference a rule must not cross silently.
        // 对象在台账记录过它、或它的声明选择了作者拥有的名字（`stable_name`）时保持身份——按路径推导的身份
        // 会被搬动重新推导，而那条边界正是规则不能静默跨过的。
        let stable = match preserved {
            Some(stable) => stable,
            None if old_view.is_some_and(|view| view.stable_name.is_some()) => old_stable,
            None => StableFaceId::from_namespaced_name(namespace, &identity.new_path),
        };
        relocations.push(Relocation {
            before,
            after: Observed::new(
                stable,
                identity.new_path.clone(),
                identity.new_source.clone(),
            ),
            old_stable,
            stable,
        });
    }
    let moved_objects = relocations
        .iter()
        .map(|relocation| rules::MovedObject {
            old_path: &relocation.before.path,
            new_path: &relocation.after.path,
            new_source: &relocation.after.source,
            stable: relocation.stable,
            old_stable: relocation.old_stable,
        })
        .collect::<Vec<_>>();
    let move_plan = rules::plan_move(root, &moved_objects)?;
    let record = if applying {
        Some(plan_record(work, face_path, new_path, old_id, new_id)?)
    } else {
        None
    };
    let graft_records = if applying && rebind {
        grafts::plan_rebind_records(root, identities)?
    } else {
        Vec::new()
    };
    // The template lineage joins the same transaction as the moved bytes: a follower whose path this
    // move changes must not keep naming a path the tree no longer has. Planned in **both** modes so
    // the preview says exactly what the write would carry.
    // 模板谱系与被搬字节进同一个事务：路径被这次搬动改掉的跟随者，不得继续点名一个树里已不存在的路径。
    // 两种模式都规划，因此预览说的正是落盘会搬运的东西。
    let lineage = apply_template::plan_move(root, face_path, new_path)?;
    let mut paths: Vec<(PathBuf, ChangeKind, String)> = Vec::new();
    if !ledgers.is_empty() {
        paths.push((
            work.join(xirang_multilang::object_map::LEDGER_FILE),
            ChangeKind::Write,
            "the object ledger".to_owned(),
        ));
    }
    if let Some(record) = &record {
        paths.push((
            record.path.clone(),
            ChangeKind::Write,
            "the migration record".to_owned(),
        ));
    }
    for planned in &graft_records {
        paths.push((
            planned.path.clone(),
            ChangeKind::Write,
            format!("the graft record `{}`", planned.description),
        ));
    }
    // Only when the write lands: a preview copy has no rule book (`.xirang` is not copied), and
    // authorizing project paths from a preview would be a write set the preview never performs.
    // 只在真正落盘时授权：预览副本没有规则簿（`.xirang` 不被复制），而从预览里授权项目路径会变成一份
    // 预览从不执行的写入集合。
    if applying {
        for planned in &move_plan.writes {
            paths.push((
                planned.path.clone(),
                ChangeKind::Write,
                format!("the rule `{}`", planned.description),
            ));
            paths.push((
                planned.history_path.clone(),
                ChangeKind::Write,
                format!("the history of `{}`", planned.description),
            ));
        }
        // The lineage file is authorized only when this move actually changes it: a move of a face
        // no row names has nothing to write there, and authorizing an unwritten path is a write set
        // the move does not perform.
        // 谱系文件只在这次搬动真的要改它时才被授权：没有任何行点名的面，搬动在那里没有东西可写，而授权
        // 一条不会被写的路径，就是一份搬动并不执行的写入集合。
        if let Some(lineage) = &lineage
            && !lineage.moved.is_empty()
        {
            paths.push((
                lineage.path.clone(),
                ChangeKind::Write,
                "the template lineage".to_owned(),
            ));
        }
    }
    if applying && rebind {
        for edit in declaration_edits.iter().chain(sibling_edits.iter()) {
            paths.push((
                edit.file.clone(),
                ChangeKind::Write,
                format!("the graft declaration in `{}`", edit.file.display()),
            ));
        }
    }
    Ok(PublicationPlan {
        ledgers,
        record,
        graft_records,
        rule_writes: move_plan.writes,
        stranded_rules: move_plan.stranded,
        lineage,
        paths,
    })
}
