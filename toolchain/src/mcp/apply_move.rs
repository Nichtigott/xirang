//! The `move` action: relocate a face — and the module subtree under it — to a new parent.
//! `move` 动作：把一个面（连同它下面的模块子树）搬到新的父级之下。
//!
//! Why this is its own module: `apply.rs` is a ratcheted file (600 code lines) and every action that
//! grows lives beside it, the same reason `apply_cut.rs` and `apply_promote.rs` do.
//! 为什么单独一个模块：`apply.rs` 是受棘轮约束的文件（600 代码行），而每个会长大的动作都住在它旁边
//! ——与 `apply_cut.rs`、`apply_promote.rs` 同一个理由。
//!
//! **A move is an identity change.** `NodeId = hash(namespace, source path, name)` takes the source
//! path as an input, so the face comes back under a new id and everything booked against the old one
//! stops resolving. The record under `.xirang/moves/` is a compatibility note for the next reader,
//! not an audit ledger, and only the most recent five are kept.
//! **一次搬动就是一次身份变化。** `NodeId = hash(命名空间, 源码路径, 名字)` 把源码路径当输入，因此面会以
//! 新 id 回来，而按旧 id 记账的一切都不再解析。`.xirang/moves/` 下的记录是留给下一个读者的兼容提示，
//! 不是审计账本，只保留最近五条。
//!
//! **What it writes, and why it is not the executor** (the maintainer's three premises, decided
//! 2026-10-09): it **may** rewrite hand-written face files (P1) but **previews unless `apply` is true**;
//! a move across a crate boundary reports a **notice** (P2 revised 2026-10-10); and the alias
//! macro **is** rewritten automatically (P3). The executor cannot be reused: it only rewrites files it
//! generated, and `AuthoredFace` has no `parent` field at all — so `move` is a new write path that has
//! to keep its own atomicity and rollback.
//! **它写什么，以及为什么不用执行器**（维护者 2026-10-09 拍板的三个前提）：它**可以**改写手写的面文件
//! （P1），但**除非 `apply` 为真否则只预览**；跨 crate 边界的搬动给出 **notice**（2026-10-10 修订 P2）；别名宏
//! **自动替换**（P3）。这里用不了执行器：它只改写它生成过的文件，而 `AuthoredFace` 根本没有 `parent`
//! 字段——因此 `move` 是一块新的写入路径，必须自己保证原子性与回滚。
//!
//! Parent edits are planned explicitly; the kernel applies them only at parsed declaration spans.
//! 父表达式先显式规划，再由内核仅在已解析的声明区间内改写。

use std::path::Path;

use serde_json::Value;

use self::external::{check_plan, plan_digest, run_move_external};
use crate::build_method::FaceView;
use crate::mcp::apply::Outcome;
use xirang_kernel::identity::NodeId;

#[path = "apply_move_grafts.rs"]
mod grafts;

#[path = "apply_move_rebind.rs"]
mod rebind;

#[path = "apply_move_endpoint.rs"]
mod endpoint;

#[path = "apply_move_references.rs"]
mod references;

#[path = "apply_move_write.rs"]
mod write;

#[path = "apply_move_external.rs"]
mod external;

#[path = "apply_move_record.rs"]
mod record;

#[path = "apply_move_publication.rs"]
mod publication;

#[path = "apply_move_destination.rs"]
mod destination;

#[path = "apply_move_children.rs"]
mod children;

use children::{ChildEdit, plan_child_edits};

pub(super) use self::destination::{Moved, directory_of, moved_source};

pub(super) use self::record::{
    KEEP_RECORDS, ROOT_PARENT_SPELLING, plan_record, prune, publish_record,
};

#[cfg(test)]
pub(super) use self::record::next_number;

/// Relocate one face under a new parent, previewing unless the caller is applying.
/// 把一个面搬到新的父级之下；除非调用方在落盘，否则只预览。
///
/// It takes **both** roots for the reason the caller cannot explain in one line: the claim ledger
/// (`add_crates.rs`) and the host entry are read from the **project** root, because a preview's copy skips
/// `.xirang/` by design, while every write lands in the work directory — which is also why it takes the
/// `applying` flag rather than inferring it.
/// 它要**两个**根，理由没法在调用处一行说清：认领台账（`add_crates.rs`）与宿主入口都从**项目**根读，因为
/// 预览副本按设计跳过 `.xirang/`；而每一处写入都落在工作目录里——这也是它接 `applying` 标志而不是自己
/// 推断的原因。
pub(crate) fn run_move(
    root: &Path,
    work: &Path,
    applying: bool,
    arguments: &Value,
) -> Result<Outcome, String> {
    // A generated partition package is a **build product**: its files are rewritten from the host by
    // `xirang crates --write`, so a successful edit here would be overwritten by the next regeneration.
    // Refuse before planning and name the authoritative entry, instead of returning a success whose
    // bytes do not survive.
    // 生成分区包是**构建产物**：它的文件由 `xirang crates --write` 从宿主重写，因此在这里成功编辑，下一次
    // 重生成就会覆盖。必须在规划前拒绝并点名权威入口，而不是返回一个字节活不过下次生成的“成功”。
    if generated_partition_package(root) {
        return Err(format!(
            "`{}` is a generated partition package; its sources are regenerated from the host by \
             `xirang crates --write`, so a move here would be lost. Move the authoritative source under \
             the host (the entry named in the partition declaration), or change that declaration first. \
             Nothing changed.",
            root.display()
        ));
    }
    let node =
        text(arguments, "node").map_err(|error| format!("{error}\n{}", move_example(root)))?;
    let to = text(arguments, "to").map_err(|error| format!("{error}\n{}", move_example(root)))?;
    let namespace = crate::mcp::registry::namespace(root)?;
    let expected_plan = arguments.get("plan").and_then(Value::as_str);
    let rebind_grafts = arguments
        .get("rebind_grafts")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let views = crate::build_method::face_views(work, &namespace)?;
    // A `FaceView::source` is relative to the **source root** the build scans (`src/`, or whatever the
    // manifest renames it to), not to the package root — measured while writing this action: reading
    // `work.join(&face.source)` asked for `<copy>/panel/object/tile/tile.rs`, and the file is under
    // `<copy>/src/`. Every filesystem step below goes through this root, while the **identity** is
    // computed from the same relative path the build hashes (audit `M7`, §M7.69).
    // `FaceView::source` 是相对构建扫描的**源码根**（`src/`，或清单把它改名成的地方），而不是包根——
    // 写这个动作时实测：`work.join(&face.source)` 去读 `<副本>/panel/object/tile/tile.rs`，而文件在
    // `<副本>/src/` 下。下面每一步文件操作都走这个根，而**身份**用构建所哈希的同一条相对路径算
    // （审计 `M7`，§M7.69）。
    let layout = crate::build_method::source_layout(work)?;
    let source_root = layout.scan_root.clone();
    let identity_base = layout.identity_base.clone();
    let (face, instance_note) = named_face_or_instance(work, &views, &node)?.ok_or_else(|| {
        format!(
            "no face in this tree has the path or identity `{node}`; `xirang.faces` lists them, and \
             `xirang.explain <path>` answers for one"
        )
    })?;
    // The destination is a face that owns a registry, or the package root itself — which is not a face,
    // because the root mounts the faces rather than being one of them.
    // 目的地是一个拥有注册机的面，或者包根本身——包根不是面，因为它挂载面而不是成为其中一个。
    let parent = if to == "root" {
        None
    } else {
        Some(named_face(&views, &to).ok_or_else(|| {
            format!(
                "no face in this tree has the path or identity `{to}`; `xirang.faces` lists them, and \
                 the package root is spelled `root`"
            )
        })?)
    };
    if let Some(parent) = parent
        && !parent.owns_registry
    {
        return Err(format!(
            "`{}` does not own a registry, so nothing may be mounted under it; pick a face whose \
             declaration carries `needs_registry: true` (the ones this tool would accept are in \
             `xirang.faces`)",
            parent.path
        ));
    }
    let name = match arguments.get("name").and_then(Value::as_str) {
        Some(name) => name.to_owned(),
        None => face.module.rsplit("::").next().unwrap().to_owned(),
    };
    if face.source.ends_with(".rs") {
        xirang_kernel::authoring::validation::validate_name(&name)?;
    } else {
        return run_move_external(
            root,
            work,
            applying,
            expected_plan,
            rebind_grafts,
            instance_note.as_deref(),
            &namespace,
            &views,
            face,
            parent,
            &name,
            &layout,
        );
    }
    let source = face.locate_source(&layout)?;
    // A Rust face whose declaration resolves through a declared source is readable, not writable.
    // 一条 Rust 声明如果经已声明来源解析，就是可读、不可写的。
    if external::outside_package(&layout.package_root, &source)? {
        return Err(format!(
            "`{}` lives in a declared source (`{}`), which this project may read but not write; \
             moving it would change a file outside this package. Nothing changed.",
            face.path, face.source
        ));
    }
    let text = std::fs::read_to_string(&source)
        .map_err(|error| format!("{} is not readable: {error}", source.display()))?;
    let moved = moved_source(face, parent, &name, &layout);
    if external::outside_package_or_ancestor(
        &layout.package_root,
        &layout.package_root.join(&moved.to_physical),
    )? {
        return Err(format!(
            "`{}` would be written outside this package (its directory resolves through a declared \
             source, which is readable but not writable). Nothing changed.",
            moved.to_physical
        ));
    }
    let new_path = match parent {
        Some(parent) => format!("{}/{name}", parent.path),
        None => format!("root/{name}"),
    };
    if new_path == face.path && moved.source == face.source {
        return Err(format!(
            "`{}` is already there: the same parent and the same name would change nothing",
            face.path
        ));
    }
    if let Some(existing) = views
        .iter()
        .find(|view| view.path == new_path && view.id != face.id)
    {
        return Err(format!(
            "`{new_path}` is already `{}`; moving onto an occupied position is a different operation — \
             pick another name (`name`) or another parent",
            existing.kind
        ));
    }
    let ownership_notice = plan_ownership_change(work, face, &moved.module)?;
    // The two spellings the tree writes: the root's own, and a typed module path for every face.
    // 这棵树写的两种拼法：根自己的那条，以及每个面用的类型化模块路径。
    let spelling = parent.map_or_else(
        || ROOT_PARENT_SPELLING.to_owned(),
        |parent| format!("crate::{}::NODE_ID", parent.module),
    );
    let parent_edit = plan_parent_edit(&text, &spelling)?;
    let alias_edit = rewrite_alias(&text, &views, face, parent)?;
    // The identity map is pure, so it is computed before the graft check: the check needs it to
    // tell a determined record from an ambiguous one (W5).
    // 身份映射是纯的，因此在嫁接检查之前就算好：检查需要它来区分"确定的记录"与"有歧义的记录"（W5）。
    let identities = references::moved_faces(&namespace, &layout, &views, face, &moved, &new_path);
    let rebind = arguments
        .get("rebind_grafts")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let follows =
        grafts::refuse_affected_grafts(root, work, &views, face, &moved, &new_path, rebind)?;
    // Planned before the move, written after it: the text comes from the same map the refusal
    // prints, so the plan here and the plan in the refusal can never disagree (W5).
    // 搬迁之前规划、搬迁之后写入：文字来自拒绝里印出的同一张映射，因此两处的计划不可能不一致（W5）。
    let declaration_edits =
        grafts::plan_declaration_edits(work, &views, face, &moved, &new_path, rebind)?;
    // A consumer package's plan is a separate entry file, so it is planned separately (W5).
    // 消费方包的计划是另一个入口文件，因此单独规划（W5）。
    let sibling_edits =
        grafts::plan_sibling_edits(root, work, &views, face, &moved, &new_path, rebind)?;
    // All descendants whose parents move need new absolute parent paths, at every depth.
    // 每一层后代只要父对象也被搬走，都需要新的绝对父路径。
    let children = plan_child_edits(&layout, &views, face, &moved)?;
    references::ensure_structural_support(&views, &moved.from_dir)?;
    let moves = references::module_moves(&views, face, &moved);
    let crate_name = references::manifest_crate_name(&layout.package_root);
    let reference_plan = references::plan(
        &references::Scope {
            package_root: &layout.package_root,
            scan_root: &source_root,
            identity_base: &identity_base,
            crate_name: crate_name.as_deref(),
        },
        &moves,
        &moved.from_package,
        &moved.to_package,
    )?;
    let old_id = face.id;
    let new_id = NodeId::from_namespaced_path(&namespace, &moved.source, &face.kind);

    let mut message = String::new();
    if let Some(note) = &instance_note {
        message.push_str(&format!("       {note}\n"));
    }
    message.push_str(&format!("move   {} → {new_path}\n", face.path));
    if let Some(notice) = ownership_notice {
        message.push_str(&notice);
    }
    message.push_str(&format!(
        "       parent: {} → {}\n",
        value_of(&parent_edit.before),
        value_of(&parent_edit.after)
    ));
    if let Some(alias) = &alias_edit {
        message.push_str(&format!("       alias:  {alias}\n"));
    }
    for child in &children {
        message.push_str(&format!(
            "       child {}: {} → {}\n",
            child.path, child.before_value, child.after_value
        ));
        if let Some(alias) = &child.alias {
            message.push_str(&format!("       child alias: {alias}\n"));
        }
    }
    message.push_str(&format!(
        "       identity {old_id} → {new_id}\n       `NodeId` hashes the source path, so every record \
         that names the old identity stops resolving; the note under `.xirang/moves/` is what the \
         next reader gets instead\n"
    ));
    for line in &follows {
        message.push_str(&format!("       rebind {line}\n"));
    }
    // Every moved face, not only the root: a record keyed by any of these stops
    // resolving, and W5's lineage needs the whole map before it can rebind one. The
    // renderer lives with the reference planner so this file keeps its size budget.
    // 每个被搬的面，而不只是根：以其中任何一个为键的记录都不再解析，而 W5 的血缘在重绑其一之前需要整张
    // 映射。渲染器与引用规划器同住，使本文件守住尺寸预算。
    message.push_str(&references::describe(&identities, &reference_plan));

    // What a preview commits to: the object, its package-relative source, the bytes it read, and
    // where those bytes are going. `apply` recomputes this from the tree it is about to write, so a
    // plan from a tree that has moved on is refused instead of landing against different bytes.
    // 预览所承诺的东西：对象、它的包内相对源码、它读到的字节，以及这些字节要去哪里。`apply` 从它即将
    // 写入的那棵树重算这个摘要，因此来自已经变过的树的计划会被拒绝，而不是落到不同的字节上。
    let mut bound_files = reference_plan
        .files
        .iter()
        .map(|plan| {
            let relative = plan
                .file
                .strip_prefix(&layout.package_root)
                .unwrap_or(&plan.file)
                .to_string_lossy()
                .replace('\\', "/");
            (
                relative,
                xirang_kernel::identity::sha256_hex(plan.before.as_bytes()),
            )
        })
        .collect::<Vec<_>>();
    // A destination the effective discovery already covers needs no mapping: either it is under the
    // walk root, or another object's mapping already describes that directory (a face moved **into**
    // an island).
    // 有效发现已经覆盖的目的地不需要映射：要么在遍历根之下，要么另一个对象的映射已经描述了那个目录
    // （被搬**进**孤岛的面）。
    let covered = moved.covered_by_default_walk
        || crate::object_ledger::covering_directory(&layout.package_root, &moved.to_physical)?
            .is_some();
    bound_files.sort();
    let plan = plan_digest(
        face,
        &face.source_ref(&layout).source,
        text.as_bytes(),
        &moved.source,
        &new_path,
        if covered { "identity" } else { "package" },
        &bound_files,
    );
    check_plan(expected_plan, &plan)?;
    message.push_str(&format!("       plan {plan}\n"));

    // The same write plan runs in the preview copy; only publication of the record differs.
    // 预览副本执行同一写入计划；只有发布迁移记录这一步不同。
    //
    // Publication joins the transaction: the record, the graft records and the host-entry
    // declarations are part of the same recoverable set as the moved bytes, so a failure in any of
    // them restores all of them.
    // 发布进入事务：记录、嫁接记录与宿主入口声明与被搬的字节属于同一个可恢复集合，因此其中任何一步失败都会
    // 把全部恢复回去。
    let mut published: Option<std::path::PathBuf> = None;
    // The ledger fact is computed before the rename, from paths that are already decided; only the
    // write happens inside the transaction.
    // 台账事实在改名之前就算好——它依据的路径已经定了；事务里发生的只是那次写入。
    let old_observed = face.observed(&layout, &namespace);
    let new_observed = xirang_multilang::object_map::Observed::new(
        face.object_id(&namespace),
        new_path.clone(),
        moved.to_physical.clone(),
    );
    // **Publication is planned before the move, not inside it.** Its paths belong to the write set
    // the move authorizes, and a set assembled while writing is a set whose second half is never
    // authorized. Planning earlier also means a publication that cannot even be planned refuses the
    // move before anything is renamed.
    // **发布在搬动之前规划，而不是在它里面。** 它的路径属于搬动所授权的写入集合，而"边写边装配"的集合，
    // 后一半永远不会被授权。提前规划还意味着：连规划都做不到的发布，会在任何改名之前拒绝这次搬动。
    let publication = publication::plan(
        work,
        root,
        applying,
        rebind,
        &face.path,
        &new_path,
        old_id,
        new_id,
        &old_observed,
        &new_observed,
        !covered,
        &namespace,
        &identities,
        &declaration_edits,
        &sibling_edits,
    )?;
    // A rule the move cannot carry is a finding about the knowledge, printed in both modes: the
    // preview and the apply are where a reader learns that this commitment needs a person.
    // 搬动带不走的规则是关于知识的发现，两种模式都印：预览与落盘都是读者得知"这份承诺需要人处理"的地方。
    for line in &publication.stranded_rules {
        message.push_str(&format!("       {line}\n"));
    }
    if !applying && !publication.rule_writes.is_empty() {
        message.push_str(&format!(
            "       would migrate {} rule(s) with the object (this preview does not write the rule book)\n",
            publication.rule_writes.len()
        ));
    }
    // The same question about the template lineage: which followers this move relocates, and which
    // template anchors it leaves naming nothing. Both modes say it; only a write performs it.
    // 关于模板谱系的同一个问题：这次搬动改写了哪些跟随者的路径，又让哪些模板锚点不再点名任何对象。两种
    // 模式都说，只有落盘才执行。
    if let Some(lineage) = &publication.lineage {
        for line in &lineage.stranded {
            message.push_str(&format!("       {line}\n"));
        }
        if !applying && !lineage.moved.is_empty() {
            let rows = lineage.moved.len();
            message.push_str(&format!(
                "       would rewrite {rows} template lineage row(s) with the object (this preview \
                 does not write the lineage)\n"
            ));
        }
    }
    write::apply_move(
        &layout.package_root,
        &identity_base,
        &moved,
        &parent_edit,
        alias_edit.as_deref(),
        &children,
        &reference_plan.files,
        &publication.paths,
        |transaction| {
            // The ledger is written in both modes, so a preview shows it in the diff and a failure
            // in any later step takes it back. It keeps the object across the move and keeps both
            // source instances resolvable.
            // 台账两种模式都写，因此预览能在 diff 里看到它，而后面任何一步失败都会把它收回去。它让对象
            // 跨这次搬动保持身份，并让两个源码实例都仍可解析。
            // A Rust face's destination **is** the layout its parent derives (its directory is its
            // module path), so a move never keeps a deviation: a mapping it carried is cleared.
            // Rust 面的目的地**就是**父级推导出的布局（它的目录就是模块路径），因此搬动从不保留偏离：
            // 它携带的映射会被清除。
            for ledger in &publication.ledgers {
                transaction.write(ledger.path(), ledger.bytes())?;
                message.push_str(&format!("       {}\n", ledger.summary()));
            }
            if applying {
                // A rule bound to the moved object is rewritten in the same transaction as the
                // move: a failure anywhere restores both, so the rule book never names a location
                // the tree does not have.
                // 绑定到被搬对象的规则与搬动在同一个事务里改写：任何一步失败都会把两者一起恢复，因此规则簿
                // 绝不会点名一个树并不存在的位置。
                for planned in &publication.rule_writes {
                    transaction.write(&planned.path, planned.updated.as_bytes())?;
                    transaction.write(&planned.history_path, planned.history_updated.as_bytes())?;
                    message.push_str(&format!("       {}\n", planned.description));
                }
                // A follower whose path this move changes is rewritten in the same transaction: a
                // failure anywhere leaves neither the move nor a row naming a path it never had.
                // 路径被这次搬动改掉的跟随者在同一个事务里改写：任何一步失败，既不会留下这次搬动，也不会留下
                // 一行点名一个它从未有过的路径。
                if let Some(lineage) = &publication.lineage
                    && !lineage.moved.is_empty()
                {
                    super::apply_template::publish(transaction, lineage)?;
                    for (before, after) in &lineage.moved {
                        message.push_str(&format!("       lineage {before} -> {after}\n"));
                    }
                }
                if let Some(record) = &publication.record {
                    publish_record(transaction, record)?;
                    message.push_str(&format!("record {}\n", record.path.display()));
                    published = Some(record.directory.clone());
                }
                // The move has landed, so the records that follow their objects can be rewritten
                // against a tree that already agrees with the new identities (W5).
                // 搬迁已经落地，因此"跟着对象走"的那些记录可以对着已与新身份一致的树改写（W5）。
                if rebind {
                    for planned in &publication.graft_records {
                        transaction.write(&planned.path, planned.text.as_bytes())?;
                        message.push_str(&format!("       {}\n", planned.description));
                    }
                    for line in grafts::apply_declaration_edits_in(&declaration_edits, transaction)?
                    {
                        message.push_str(&format!("       {line}\n"));
                    }
                    for line in grafts::apply_declaration_edits_in(&sibling_edits, transaction)? {
                        message.push_str(&format!("       {line}\n"));
                    }
                }
            }
            Ok(())
        },
    )?;
    // Eviction is not part of the transaction: it runs only once the move has landed, and a failure
    // to drop an old record is reported rather than undoing a move that succeeded.
    // 淘汰不属于事务：它只在搬动落地之后运行，而"没能丢掉旧记录"是被报告出来的，而不是把一次已经成功的搬动
    // 撤回去。
    if let Some(directory) = published {
        let dropped = prune(&directory, KEEP_RECORDS);
        if !dropped.is_empty() {
            message.push_str(&format!(
                "       pruned {} (the newest {} are kept): the mapping it carried is no longer on \
                 disk\n",
                dropped.join(", "),
                KEEP_RECORDS
            ));
        }
    }
    Ok(Outcome {
        message,
        declaration: None,
        source: layout.package_root.join(&moved.to_physical),
        moved: true,
        alternative: false,
        inherited: Vec::new(),
        consumers: Vec::new(),
    })
}

/// The face a path or an identity names.
/// 某个路径或身份指名的面。
///
/// Named for its object rather than `find`: a bare verb is only allowed at an entry position in this
/// workspace, and this is not one (the verb-table gate refused it by name).
/// 名字带着它的宾语，而不是叫 `find`：本工作区只允许裸动词出现在入口位置，而这里不是（动词表门禁点名拒了它）。
fn named_face<'a>(views: &'a [FaceView], target: &str) -> Option<&'a FaceView> {
    views
        .iter()
        .find(|view| view.path == target)
        .or_else(|| views.iter().find(|view| view.id.to_string() == target))
}

/// Resolve a node name or an **old source instance** to the face it is now.
/// 把节点名或一个**旧源码实例**解析成它现在的面。
///
/// A physical move changes the source-instance identity, so a reference written before the move
/// names an id no face in the tree carries any more. The object ledger is the thing that connects
/// the two (`entry_for_instance`), and without a consumer here the old id was only resolvable by
/// library callers — the MCP answer was "no face", which made "old references still resolve" a
/// claim about a method rather than about the bridge. The note is returned so the reply says which
/// instance it followed instead of silently pretending the id was current.
/// 物理搬动会改变源码实例身份，因此搬动之前写下的引用点名的 id 已不再被树里任何面持有。对象台账正是把两者
/// 连起来的东西（`entry_for_instance`），而这里没有消费者时，旧 id 只有库调用方能解析——MCP 的回答是
/// "没有这个面"，这让"旧引用仍可解析"变成一句关于方法、而不是关于桥的话。返回那条注记，是为了让回复说出
/// 它跟到了哪个实例，而不是静默假装那个 id 是当前的。
fn named_face_or_instance<'a>(
    root: &Path,
    views: &'a [FaceView],
    target: &str,
) -> Result<Option<(&'a FaceView, Option<String>)>, String> {
    if let Some(face) = named_face(views, target) {
        return Ok(Some((face, None)));
    }
    let Ok(id) = target.parse::<NodeId>() else {
        return Ok(None);
    };
    let map = xirang_multilang::object_map::ObjectMap::read_from(root)?;
    let Some(entry) = map.entry_for_instance(id) else {
        return Ok(None);
    };
    let current = views
        .iter()
        .find(|view| view.path == entry.path)
        .or_else(|| {
            entry
                .current_id()
                .and_then(|current| views.iter().find(|view| view.id == current))
        });
    Ok(current.map(|face| {
        (
            face,
            Some(format!(
                "`{target}` is an old source instance of `{}`; this request uses its current \
                 instance `{}`",
                entry.path, face.id
            )),
        )
    }))
}

/// The alias macro a face uses, from the face its `parent:` points at.
/// 一个面所用的别名宏，由它的 `parent:` 指向的那个面决定。
///
/// `renderer/aliases.rs` emits one `<name>_object!` per registry-owning face and `root_object!` for the
/// root; a face uses the one belonging to the nearest registry owner **at or above its parent**, because
/// `parent:` is what decides the answer (audit `M7`, §1.3).
/// `renderer/aliases.rs` 为每个拥有注册机的面发一条 `<name>_object!`，为根发 `root_object!`；一个面用的是
/// **它父级自身或之上**最近的那个注册机拥有者的那条——因为决定这个答案的正是 `parent:`（审计 `M7`，§1.3）。
pub(super) fn alias_for(views: &[FaceView], parent: Option<&FaceView>) -> String {
    registry_owner(views, parent).map_or_else(
        || "root_object!".to_owned(),
        |view| format!("{}_object!", view.module.rsplit("::").next().unwrap()),
    )
}

/// The nearest registry owner, retaining its identity for owner renames.
/// 最近的注册机拥有者，保留身份以处理拥有者改名。
pub(super) fn registry_owner<'a>(
    views: &'a [FaceView],
    parent: Option<&'a FaceView>,
) -> Option<&'a FaceView> {
    let mut current = parent;
    while let Some(view) = current {
        if view.owns_registry {
            return Some(view);
        }
        current = views.iter().find(|candidate| candidate.id == view.parent);
    }
    None
}

/// Parsed parent values rendered as fields for the move preview.
/// 已解析父值按字段形式渲染，供搬迁预览使用。
#[derive(Debug)]
pub(super) struct ParentEdit {
    /// Original field value; not a source line or a replacement span.
    /// 原字段值；不是源码行，也不是替换区间。
    pub(super) before: String,
    /// Planned field value, applied by the native syntax editor.
    /// 计划中的字段值，由原生语法编辑器应用。
    pub(super) after: String,
}

/// Plan from the native declaration, independent of layout, comments and string contents.
/// 从原生声明规划，不依赖排版、注释或字符串内容。
pub(super) fn plan_parent_edit(text: &str, spelling: &str) -> Result<ParentEdit, String> {
    use xirang_kernel::syntax::{ParentSyntax, parse_face};
    let face = parse_face(text)
        .map_err(|error| error.to_string())?
        .ok_or("source has no registration face")?;
    let value = face
        .field("parent")
        .ok_or("registration face has no explicit parent")?;
    let known = match face.parent() {
        Some(ParentSyntax::Root) => true,
        Some(ParentSyntax::NodePath(module)) => module.starts_with("crate::"),
        _ => false,
    };
    if !known {
        return Err(format!(
            "this face's `parent:` is spelled `{value}`, which is neither a typed `crate::…::NODE_ID` \
             nor the root's `crate::root_node_id(…)`; relocate that reference explicitly"
        ));
    }
    Ok(ParentEdit {
        before: format!("parent: {value},"),
        after: format!(
            "parent: {},",
            if face.is_native()
                && spelling.starts_with("crate::")
                && spelling.ends_with("::NODE_ID")
            {
                spelling.replacen("crate::", "crate::catalog::", 1)
            } else {
                spelling.to_owned()
            }
        ),
    })
}

pub(super) fn value_of(line: &str) -> String {
    line.trim()
        .split_once(':')
        .map(|(_, value)| value.trim().trim_end_matches(',').to_owned())
        .unwrap_or_else(|| line.trim().to_owned())
}

/// Rewrite the alias macro when the new parent changes whose registry it is.
/// 当新的父级换了注册机归属时，改写别名宏。
///
/// P3: automatic. It is the one edit that is not required for the tree to **compile** — the face would
/// build either way — and it is required for the file to keep telling the truth about where it hangs.
/// P3：自动。这是唯一一处"不改也能**编译**过"的改写——两种写法都能构建——而不改的话，这个文件就会继续
/// 说错自己挂在哪里。
fn rewrite_alias(
    text: &str,
    views: &[FaceView],
    face: &FaceView,
    parent: Option<&FaceView>,
) -> Result<Option<String>, String> {
    if xirang_kernel::syntax::parse_face(text)
        .map_err(|error| error.to_string())?
        .is_some_and(|face| face.is_native())
    {
        return Ok(None);
    }
    let current_parent = views.iter().find(|view| view.id == face.parent);
    let before = alias_for(views, current_parent);
    let after = alias_for(views, parent);
    if before == after {
        return Ok(None);
    }
    if !text.contains(&before) {
        return Err(format!(
            "this face hangs under `{before}` by its `parent:` line, but its file never spells that \
             alias; this action rewrites the alias the tree derives, and refuses to invent one in a file \
             that does not carry it"
        ));
    }
    Ok(Some(format!("{before} → {after}")))
}

/// Ownership follows existing claims; crossing owners alone is informational.
/// 编译归属跟随已有认领；仅跨越归属只是通知。
/// Claim roots still require declaration rebinding, independently of graft impact checks.
/// 认领根仍需重绑声明，与 graft 影响检查相互独立。
fn plan_ownership_change(
    work: &Path,
    face: &FaceView,
    new_module: &str,
) -> Result<Option<String>, String> {
    let declaration = crate::build_method::read_shape_declaration(work)?;
    // No declaration is the ordinary case: nothing claims anything, so nothing can be crossed.
    // 没有声明是普通情形：没有东西被认领，也就无从跨越。
    let Some(declaration) = declaration else {
        return Ok(None);
    };
    let claims = declaration.cut_subtrees();
    for claim in &claims {
        if under(claim, &face.module) {
            return Err(format!(
                "`{}` contains `{claim}`, which `add_crates.rs` claims for another crate; \
                 moving that claim or its ancestor would invalidate it — change the declaration first, then move",
                face.path
            ));
        }
    }
    let owner = |module: &str| {
        declaration.crates.iter().find_map(|(name, claims)| {
            claims
                .iter()
                .any(|claim| under(module, claim))
                .then_some(name.as_str())
        })
    };
    let before = owner(&face.module);
    let after = owner(new_module);
    if before != after {
        return Ok(Some(format!(
            "notice: compilation owner {} -> {}; existing `add_crates.rs` claims follow the destination. \
             Regenerate packages with `xirang crates --write`, update affected Rust imports and \
             dependencies, then run the affected consumers' tests. This move does not rewrite business references.\n",
            before.unwrap_or("host"),
            after.unwrap_or("host")
        )));
    }
    Ok(None)
}

/// Whether `module` is `claim` or sits under it.
/// `module` 是不是 `claim`、或位于其下。
fn under(module: &str, claim: &str) -> bool {
    module == claim || module.starts_with(&format!("{claim}::"))
}

/// One string argument, or the shape this action accepts.
/// 取一个字符串参数，或给出本动作接受的形状。
fn text(arguments: &Value, key: &str) -> Result<String, String> {
    arguments
        .get(key)
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| format!("this action needs `{key}`"))
}

/// A complete move request, built from the tree this call resolved.
/// 一个完整的搬动请求，由本次调用解析出的那棵树搭出来。
///
/// Same rule as `add`'s and `promote`'s examples: a refusal that offers placeholders makes the reader go
/// and list the tree before the request becomes a call.
/// 与 `add`、`promote` 的示例同一条规则：给出占位符的拒绝，会让读者在请求变成调用之前先去列一遍树。
fn move_example(root: &Path) -> String {
    let namespace = crate::mcp::registry::namespace(root).unwrap_or_default();
    let views = crate::build_method::face_views(root, &namespace).unwrap_or_default();
    let face = views
        .iter()
        .find(|view| !view.owns_registry)
        .or(views.first());
    let parent = views
        .iter()
        .find(|view| view.owns_registry)
        .map(|view| view.path.clone())
        .unwrap_or_else(|| "root".to_owned());
    let node = face
        .map(|view| view.path.clone())
        .unwrap_or_else(|| "root".to_owned());
    format!(
        "accepted shape  {{\"action\":\"move\",\"node\":\"{node}\",\"to\":\"{parent}\",\"apply\":true}}"
    )
}

#[cfg(test)]
#[path = "apply_move_tests.rs"]
mod apply_move_tests;

#[cfg(test)]
#[path = "apply_move_foreign_tests.rs"]
mod apply_move_foreign_tests;

#[cfg(test)]
#[path = "apply_move_graft_fixtures.rs"]
mod graft_fixtures;

#[cfg(test)]
#[path = "apply_move_graft_tests.rs"]
mod graft_tests;

#[cfg(test)]
#[path = "apply_move_fault_tests.rs"]
mod fault_tests;

#[cfg(test)]
#[path = "apply_move_rebind_tests.rs"]
mod rebind_tests;

#[cfg(test)]
#[path = "apply_move_transaction_tests.rs"]
mod transaction_tests;

/// Whether this package is one `xirang crates --write` generated.
/// 这个包是否由 `xirang crates --write` 生成。
fn generated_partition_package(root: &Path) -> bool {
    std::fs::read_to_string(root.join("Cargo.toml")).is_ok_and(|text| {
        text.lines()
            .any(|line| line.trim_start().starts_with("# xirang-generation:"))
    })
}
