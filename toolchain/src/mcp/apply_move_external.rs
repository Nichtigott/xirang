//! The foreign-source branch of `move`, plus the plan digest both branches bind.
//! `move` 的外语源码分支，以及两个分支共同绑定的计划摘要。
//!
//! A foreign declaration has no module tree and no generated file: the adapter plans the text edit,
//! and the generic transaction still owns identity, records and refusals. The branch lives here so the
//! Rust half keeps its size budget; it reads the shared source fact the same way the Rust half does.
//! 外语声明没有模块树、也没有生成文件：适配器规划文本改写，事务/身份/记录仍由通用层拥有。放在这里是为了让
//! Rust 那一半守住尺寸预算；它读取共享源码事实的方式与 Rust 那一半相同。

use std::path::{Path, PathBuf};

use crate::build_method::FaceView;
use crate::mcp::apply::Outcome;
use xirang_kernel::identity::NodeId;

use super::directory_of;
use crate::mcp::apply_move_set;

/// Move one declaration in a language whose adapter plans the source rewrite, together with the
/// references that name it: a C++ child's `.under(...)` and any Rust `xirang_external` alias.
/// 搬动一个由适配器规划源码改写的声明，连同点名它的引用：C++ 子面的 `.under(...)` 与 Rust 的
/// `xirang_external` 别名。
///
/// The generic layer still owns identity, records and refusals; this function owns the text, and it
/// plans every rewrite before writing any of them, so a refusal leaves the tree untouched.
/// 通用层仍拥有身份、记录与拒绝；本函数拥有文本，并在写入任何东西之前规划全部改写，因此一次拒绝不会
/// 动到树。
#[allow(clippy::too_many_arguments)]
pub(super) fn run_move_external(
    root: &Path,
    work: &Path,
    applying: bool,
    expected_plan: Option<&str>,
    rebind_grafts: bool,
    instance_note: Option<&str>,
    namespace: &str,
    views: &[FaceView],
    face: &FaceView,
    parent: Option<&FaceView>,
    name: &str,
    layout: &crate::build_method::SourceLayout,
) -> Result<Outcome, String> {
    let language = xirang_multilang::registration::registry()
        .language_of_path(&face.source)
        .ok_or_else(|| format!("`{}` has no registered language adapter", face.source))?;
    let adapter = xirang_multilang::registration::registry()
        .adapter(language)
        .ok_or_else(|| format!("`{}` has no adapter", language.as_str()))?;
    let old_parent = views
        .iter()
        .find(|view| view.id == face.parent)
        .map(|view| view.module.as_str());
    let new_parent = parent.map(|parent| parent.module.as_str());
    // A C++ declaration names its parent with a generated-header constant. The header carries one
    // alias per face; there is no root alias, and `plan_move` would return no edit for `None` and
    // leave the old `.under(...)` in place — a rename that silently keeps the old parent. Refuse
    // before anything is read or written.
    // C++ 声明用生成头常量点名父级。生成头按面给别名，没有根的别名；而 `plan_move` 对 `None` 不产生
    // 编辑，会留下旧的 `.under(...)`——一次静默保留旧父级的改名。因此在读/写任何东西之前拒绝。
    if parent.is_none() {
        return Err(format!(
            "`{}` cannot move to the package root: a C++ declaration names its parent with a \
             constant from the generated header, and that header has no root constant to write into \
             `.under(...)`. Point `to` at a face whose declaration carries `needs_registry: true`. \
             Nothing changed.",
            face.path
        ));
    }
    // Every destination is computed in the **package-relative** space, which is the space a foreign
    // record is written in. The parent's spelling may be identity-relative (a Rust face under
    // `src/`), so it is normalized through the shared fact first; this is what makes a cross-base
    // destination expressible without a second "which root" rule.
    // 每个目的地都在**包内相对**空间里计算，那也是外语记录书写所用的空间。父级的拼写可能是身份相对
    // （`src/` 下的 Rust 面），因此先经共享事实规范化；这正是跨基准目的地无需第二套"哪个根"规则的原因。
    let parent_package_source = parent.map(|parent| {
        crate::build_method::source_ref::package_relative(
            parent.source_base,
            &layout.package_root,
            &layout.identity_base,
            &parent.source,
        )
    });
    // A **pure physical move** — same parent — keeps the object where its mapping says it lives: the
    // destination is the object's mapped directory, not the parent's directory. Without this, renaming
    // an island's declaration would silently pull it out of the island it was placed in. A move that
    // changes the parent is a logical move: it goes under the new parent and the mapping follows (see
    // `plan_move`).
    // **纯物理搬动**——父级不变——把对象留在它映射所说的位置：目的地是对象被映射的目录，而不是父级的目录。
    // 没有这一条，给孤岛里的一条声明改名，就会把它悄悄拉出它被放置的那个孤岛。换父级的搬动是逻辑搬动：它去
    // 新父级之下，映射跟着走（见 `plan_move`）。
    let same_parent = parent.map(|parent| parent.id) == Some(face.parent);
    let mapped = crate::object_ledger::mapped_directory(work, &face.observed(layout, namespace))?;
    let directory = match (&mapped, same_parent) {
        (Some(mapped), true) => mapped.trim_end_matches('/').to_owned(),
        // A **logical** move lands where a *creation* under that parent would land: the one layout
        // rule is `<parent directory>/object/<module>/<module>.<extension>` (see
        // `authoring_service::resolve::destination`). Using `<parent directory>/<name>.<ext>` here
        // would give the same object two different "default" places depending on whether it was
        // created or moved, and the ledger's "is this a deviation?" answer would differ with it.
        // **逻辑**搬动落在"在该父级下新建"会落的地方：唯一的布局规则是
        // `<父目录>/object/<模块>/<模块>.<扩展名>`（见 `authoring_service::resolve::destination`）。在这里用
        // `<父目录>/<名字>.<扩展名>` 会让同一个对象因为"新建"还是"搬动"而有两个不同的"默认"位置，台账
        // "这是不是一次偏离"的答案也会随之不同。
        _ => parent_package_source.map_or_else(String::new, |source| {
            let parent_directory = directory_of(&source);
            if parent_directory.is_empty() {
                format!("object/{name}")
            } else {
                format!("{parent_directory}/object/{name}")
            }
        }),
    };
    let kept_island = same_parent.then_some(mapped.as_deref()).flatten();
    let extension = Path::new(&face.source)
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or("rs");
    let new_source = if directory.is_empty() {
        format!("{name}.{extension}")
    } else {
        format!("{directory}/{name}.{extension}")
    };
    let destination = layout.package_root.join(&new_source);
    // The module a build derives is a function of the **new path**, so the plan must derive it the
    // same way; the old "parent module + name" spelling disagreed with the build whenever the
    // directory and the leaf differed.
    // 构建推导的模块是**新路径**的函数，因此计划必须用同一种推导；旧的"父模块 + 名字"拼写在目录与
    // 末段不同时与构建不一致。
    let new_module = crate::build_method::declared_native::module_of(&new_source);
    // A move is an identity change whenever the source path changes, so the compatibility note the
    // Rust branch writes must be written here too: the next reader needs the old→new mapping even
    // when the logical path happens to stay the same (`Area` keeps its `.named` path but not its id).
    // 只要源码路径变了，搬动就是一次身份变化，因此 Rust 分支写的那条兼容记录这里也要写：即使逻辑路径
    // 恰好不变（`Area` 保留 `.named` 路径却换了 id），下一个读者也需要这条旧→新映射。
    let new_path = format!(
        "{}/{}",
        parent.map_or_else(|| "root".to_owned(), |parent| parent.path.clone()),
        face.registry_name
    );
    let new_id = NodeId::from_namespaced_path(namespace, &new_source, &face.registry_name);
    // A graft record names an identity. When the moved face's identity changes, a record that still
    // names the old one is either carried along (the caller asked) or blocks the move — exactly the
    // policy the Rust branch applies, and for the same reason: following the object is computable,
    // but it is still the caller's decision.
    // 嫁接记录点名的是身份。被搬面的身份变化后，仍点名旧身份的记录要么被一起带走（调用方要求），要么
    // 阻塞这次搬动——与 Rust 分支同一条策略、同一个理由：跟着对象走算得出来，但它仍是调用方的决定。
    let mut rebind = Vec::new();
    if new_id != face.id {
        for loaded in crate::run_method::load_graft_records(root)
            .map_err(|error| format!("cannot inspect graft records before move: {error}"))?
        {
            let crate::run_method::LoadedGraft::Record(record) = loaded else {
                continue;
            };
            let by_id = record.document.target == face.id;
            let by_path = record.document.target_path == face.path;
            if !by_id && !by_path {
                continue;
            }
            if !(by_id && by_path) {
                return Err(format!(
                    "cannot rebind record `{}`: its target and target_path do not name the same \
                     moved object; repair that record instead of letting a move guess. Nothing \
                     changed.",
                    record.selector
                ));
            }
            if !rebind_grafts {
                return Err(format!(
                    "the graft record `{}` names `{}`; the move changes that face's identity, so the \
                     record would stop resolving. Pass `rebind_grafts: true` to carry it, or repair \
                     the record by hand. Nothing changed.",
                    record.selector, face.path
                ));
            }
            rebind.push(record.selector.clone());
        }
    }

    let old_ident = crate::build_method::declared_native::ident(&face.source, &face.registry_name);
    let new_ident = crate::build_method::declared_native::ident(&new_source, &face.registry_name);
    let old_reference = format!("crate::xirang_external::{old_ident}::NODE_ID");
    let new_reference = format!("crate::xirang_external::{new_ident}::NODE_ID");
    // A `static_graft_plan!` cut names a face with a typed Rust path. For a foreign face that path
    // is the generated `xirang_external` reference, not a module path, so the move must see it here
    // or it would leave a cut pointing at an identity that no longer exists.
    // `static_graft_plan!` 的切口用类型化 Rust 路径点名面。对外语面而言那条路径是生成的
    // `xirang_external` 引用，而不是模块路径；搬动必须在这里看见它，否则会留下一条指向已不存在身份的切口。
    let declaration_edits = super::grafts::plan_foreign_declarations(
        work,
        face,
        &old_reference,
        &new_reference,
        &new_path,
        rebind_grafts,
    )?;
    let mut edited_files = std::collections::BTreeSet::new();
    for edit in &declaration_edits {
        edited_files.insert(edit.file.clone());
    }

    let source_path = face.locate_source(layout)?;
    // A declaration that lives in a **declared source** is readable, not writable: moving it would
    // rewrite (and delete) a file outside this package. Refuse before anything is read or planned.
    // 住在**已声明来源**里的声明可读、不可写：搬动它会改写（并删除）本包之外的文件。在读或规划任何东西
    // 之前拒绝。
    if outside_package(&layout.package_root, &source_path)? {
        return Err(format!(
            "`{}` lives in a declared source (`{}`), which this project may read but not write; \
             moving it would change a file outside this package. Nothing changed.",
            face.path, face.source
        ));
    }
    let source = std::fs::read_to_string(&source_path)
        .map_err(|error| format!("{} is not readable: {error}", source_path.display()))?;
    let catalog_parent = parent.map(|parent| xirang_multilang::registration::CatalogParent {
        path: parent.path.trim_start_matches("root/"),
    });
    let edits = adapter.plan_move_with_catalog(
        &xirang_multilang::registration::MoveRequest {
            source: &source,
            old_module: &face.module,
            new_module: &new_module,
            old_parent,
            new_parent,
        },
        catalog_parent.as_ref(),
    )?;
    let updated = xirang_multilang::registration::apply_source_edits(&source, &edits)?;

    // A moved face is named by two kinds of references: a C++ child names it through the generated
    // header's module constant, and Rust code names it through the flat `xirang_external` alias.
    // Both are textual and both are derivable from the old/new source path, so the move rewrites
    // them in the same transaction instead of leaving a tree that compiles into the wrong shape.
    // 被搬的面会被两类引用点名：C++ 子面经生成头的模块常量，Rust 代码经扁平 `xirang_external` 别名。
    // 两者都是文本、都能由新旧源码路径推导，因此搬动在同一事务里改写它们，而不是留下"能编译但树错"的树。
    // The complete set is assembled first and authorized once, below. `kept` records the members
    // that need **no** write — a reference this move does not change, or one the generated header's
    // compatibility namespace keeps resolving — and `conflicts` records the ones that would have to
    // be written outside. Both are reported; neither is silently skipped.
    // 先装配完整集合、再在下面一次性授权。`kept` 记录**不需要**写入的成员——这次搬动不改变的引用，或生成
    // 头部兼容命名空间让它继续解析的那一条；`conflicts` 记录那些会不得不写到包外的成员。两者都会报出来，
    // 谁都不静默跳过。
    let mut rewrites: Vec<Rewrite> = Vec::new();
    let mut kept: Vec<String> = Vec::new();
    let mut conflicts: Vec<apply_move_set::Conflict> = Vec::new();
    for child in views.iter().filter(|view| view.parent == face.id) {
        let child_path = child.locate_source(layout)?;
        let child_source = std::fs::read_to_string(&child_path)
            .map_err(|error| format!("{} is not readable: {error}", child_path.display()))?;
        if outside_package(&layout.package_root, &child_path)? {
            // A child in a declared source: readable, not writable. Two honest outcomes — the
            // reference needs no write at all (`kept`), or the move cannot be done without one
            // (`conflicts`, reported with the whole set so the reader sees every member at once).
            // 住在已声明来源里的子面：可读、不可写。两种诚实的结局——引用根本不需要写入（`kept`），或者
            // 没有一次写入就做不成这次搬动（`conflicts`，连整个集合一起报出，让读者一次看全每个成员）。
            if let Some(note) = apply_move_set::keep_through_compatibility(
                views,
                face,
                child,
                &face.module,
                &new_module,
                &child_source,
            ) {
                kept.push(note);
            } else {
                conflicts.push(apply_move_set::Conflict {
                    subject: format!("the child face `{}`", child.path),
                    path: child_path,
                    because: format!(
                        "it names `{}` as its parent through the module path `{}`, and this move \
                         changes that to `{new_module}`",
                        face.path, face.module
                    ),
                });
            }
            continue;
        }
        let (description, updated) = if child.source.ends_with(".rs") {
            if !child_source.contains(&old_reference) {
                return Err(format!(
                    "`{}` is recorded under `{}` but its declaration does not contain `{old_reference}`, so the move cannot rewrite that parent reference. Nothing changed.",
                    child.path, face.path
                ));
            }
            (
                format!(
                    "{}: {old_ident} → {new_ident}",
                    relative_to(&layout.package_root, &child_path)
                ),
                child_source.replace(&old_reference, &new_reference),
            )
        } else {
            let child_language = xirang_multilang::registration::registry()
                .language_of_path(&child.source)
                .ok_or_else(|| format!("`{}` has no registered language adapter", child.source))?;
            let child_adapter = xirang_multilang::registration::registry()
                .adapter(child_language)
                .ok_or_else(|| format!("`{}` has no adapter", child_language.as_str()))?;
            // A file/module rename does **not** change the object's logical path: the declaration's
            // `.named(…)` stays. Only a move to a different parent changes it; the leaf is the
            // object's existing logical leaf, not the new file name.
            // 文件/模块改名**不改变**对象的逻辑路径：声明的 `.named(…)` 保留。只有换父才改变；叶子是
            // 对象既有逻辑叶子，不是新文件名。
            let old_parent_path = views
                .iter()
                .find(|view| view.id == face.parent)
                .map(|view| view.path.as_str());
            let moved_path = match (parent, old_parent_path) {
                (Some(parent), Some(old)) if parent.path == old => face.path.clone(),
                (Some(parent), _) => {
                    let leaf = face.path.rsplit('/').next().unwrap_or(name);
                    if parent.path == "root" {
                        leaf.to_owned()
                    } else {
                        format!("{}/{}", parent.path.trim_start_matches("root/"), leaf)
                    }
                }
                (None, _) => face.path.clone(),
            };
            let catalog_parent =
                xirang_multilang::registration::CatalogParent { path: &moved_path };
            let edits = child_adapter.plan_move_with_catalog(
                &xirang_multilang::registration::MoveRequest {
                    source: &child_source,
                    old_module: &child.module,
                    new_module: &child.module,
                    old_parent: Some(&face.module),
                    new_parent: Some(&new_module),
                },
                Some(&catalog_parent),
            )?;
            (
                format!(
                    "{}: .under → crate::catalog::{moved_path}::NODE_ID",
                    relative_to(&layout.package_root, &child_path)
                ),
                xirang_multilang::registration::apply_source_edits(&child_source, &edits)?,
            )
        };
        if updated == child_source {
            // The adapter produced the same bytes: the reference does not depend on what changed,
            // so there is nothing to write and nothing to restore.
            // 适配器产出的字节相同：那条引用不依赖这次改变，因此没有要写的、也没有要恢复的。
            kept.push(format!(
                "kept `{}`: its reference did not change",
                child.source
            ));
            continue;
        }
        let relative = relative_to(&layout.package_root, &child_path);
        rewrites.push(Rewrite {
            description,
            path: child_path,
            original: child_source,
            updated,
            relative,
        });
    }
    // Every other Rust file in the package that names the moved face through the same alias follows
    // it too; a consumer left behind would compile against an alias that no longer exists.
    // 包里其它经同一别名点名被搬面的 Rust 文件也跟着走；留在原地的消费者会去编译一个已不存在的别名。
    let mut claimed: std::collections::BTreeSet<std::path::PathBuf> = rewrites
        .iter()
        .map(|rewrite| rewrite.path.clone())
        .collect();
    for path in rust_sources_under(&layout.package_root)? {
        // A file that carries a graft declaration is rewritten by that plan instead; letting the
        // consumer replacement write the same original text afterwards would clobber it.
        // 携带嫁接声明的文件由那份声明改写计划负责；让消费者替换再用原始文本写一遍会覆盖它。
        if edited_files.contains(&path) || !claimed.insert(path.clone()) {
            continue;
        }
        let text = std::fs::read_to_string(&path)
            .map_err(|error| format!("{} is not readable: {error}", path.display()))?;
        if !text.contains(&old_reference) {
            continue;
        }
        // A declared source may be **read** through its materialization, but that declaration is
        // not a write permission: a rewrite that reached through the link would edit a file
        // outside this package. Rust has no compatibility spelling for the retired alias, so this
        // is a conflict — reported with the rest of the set, not as a refusal on whichever member
        // happened to be planned first.
        // 已声明来源可以经它的物化位置**读取**，但那份声明不是写权限：穿过链接的改写会编辑本包之外的文件。
        // Rust 没有为退役别名保留兼容拼法，因此这是一条冲突——与集合其余成员一起报出，而不是在"恰好先被规划"
        // 的那个成员上拒绝。
        if outside_package(&layout.package_root, &path)? {
            conflicts.push(apply_move_set::Conflict {
                subject: format!(
                    "the Rust consumer `{}`",
                    relative_to(&layout.package_root, &path)
                ),
                path,
                because: format!("it names `{old_reference}`, which this move retires"),
            });
            continue;
        }
        let relative = relative_to(&layout.package_root, &path);
        rewrites.push(Rewrite {
            description: format!("{relative}: {old_ident} → {new_ident}"),
            path,
            original: text.clone(),
            updated: text.replace(&old_reference, &new_reference),
            relative,
        });
    }
    if !conflicts.is_empty() {
        return Err(apply_move_set::describe_conflicts(
            &format!("move `{}`", face.path),
            &conflicts,
            &layout.package_root,
        ));
    }

    let mut bound = rewrites
        .iter()
        .map(|rewrite| {
            (
                rewrite.relative.clone(),
                xirang_kernel::identity::sha256_hex(rewrite.original.as_bytes()),
            )
        })
        .collect::<Vec<_>>();
    // The graft-declaration edits are planned from entry-file bytes too, so they join the digest:
    // a preview made against a different `static_graft_plan!` must not be applied.
    // 嫁接声明改写同样依据入口文件的字节规划，因此也进摘要：对着另一份 `static_graft_plan!` 做出的
    // 预览不得被应用。
    for file in &edited_files {
        let bytes = std::fs::read(file).map_err(|error| {
            format!(
                "cannot read {} before planning its edit: {error}",
                file.display()
            )
        })?;
        bound.push((
            relative_to(&layout.package_root, file),
            xirang_kernel::identity::sha256_hex(&bytes),
        ));
    }
    // **Publication is planned here, not inside the transaction.** A record, the graft rebinds and
    // the ledger are facts about identities, so they are computable before anything is written —
    // and planning them now is what lets the **whole** set be authorized in one pass. Planning them
    // inside the write phase is how the old code came to check "the declaration and its destination"
    // and write six other things.
    // **发布在这里规划，而不是在事务里。** 记录、嫁接重绑与台账是关于身份的事实，因此可以在写入任何东西之前
    // 算出来——而此刻规划它们，正是"整个集合一次授权"成立的前提。在写入阶段里规划它们，正是旧代码如何变成
    // "只检查声明与它的目的地，却写了另外六样东西"的原因。
    let identities = [super::references::FaceIdentity {
        old_path: face.path.clone(),
        new_path: new_path.clone(),
        old_id: face.id,
        new_id,
        // The graft rebind reads paths and identities; the foreign move's own publication does not
        // plan ledger entries here, so both spellings carry the face's package-relative source.
        // 嫁接重绑读的是路径与身份；外语搬动的发布不在这里规划台账条目，因此两个拼写都带该面的包内相对源码。
        old_source: face.source.clone(),
        new_source: face.source.clone(),
    }];
    let graft_records = if applying {
        super::grafts::plan_rebind_records(root, &identities)?
    } else {
        Vec::new()
    };
    let record = if applying {
        Some(super::plan_record(
            work, &face.path, &new_path, face.id, new_id,
        )?)
    } else {
        None
    };
    let old_observed = face.observed(layout, namespace);
    let new_observed = xirang_multilang::object_map::Observed::new(
        face.object_id(namespace),
        new_path.clone(),
        new_source.clone(),
    );
    // A pure physical move inside an island keeps the deviation; a logical move lands the object at
    // the layout its new parent derives, so the old mapping is cleared.
    // 孤岛里的纯物理搬动保留偏离；逻辑搬动让对象落在新父级推导出的布局，因此旧映射被清除。
    let ledger = crate::object_ledger::plan_move(
        work,
        &old_observed,
        face.id,
        &new_observed,
        new_id,
        kept_island.is_some(),
    )?;
    // The complete write set — every byte this move will create, replace or delete, including what
    // the adapters produced and what publication writes. One authorization, before the first write.
    // 完整写入集合——这次搬动会创建、替换或删除的每一份字节，包含适配器产出的与发布写入的。写出第一个字节
    // 之前，一次授权。
    let mut changes = vec![apply_move_set::write(
        &destination,
        format!("the moved declaration `{new_source}`"),
    )];
    for rewrite in &rewrites {
        changes.push(apply_move_set::write(
            &rewrite.path,
            format!("the reference in `{}`", rewrite.relative),
        ));
    }
    for edit in &declaration_edits {
        changes.push(apply_move_set::write(
            &edit.file,
            format!(
                "the graft declaration in `{}`",
                relative_to(&layout.package_root, &edit.file)
            ),
        ));
    }
    if source_path != destination {
        changes.push(apply_move_set::remove(
            &source_path,
            format!("the declaration's old file `{}`", face.source),
        ));
    }
    for planned in &graft_records {
        changes.push(apply_move_set::write(
            &planned.path,
            format!("the graft record `{}`", planned.description),
        ));
    }
    if let Some(record) = &record {
        changes.push(apply_move_set::write(
            &record.path,
            "the migration record".to_owned(),
        ));
    }
    if let Some(ledger) = &ledger {
        changes.push(apply_move_set::write(
            ledger.path(),
            "the object ledger".to_owned(),
        ));
    }
    apply_move_set::authorize(
        &layout.package_root,
        &changes,
        &format!("move `{}`", face.path),
    )?;

    let plan = plan_digest(
        face,
        &face.source_ref(layout).source,
        source.as_bytes(),
        &new_source,
        &new_path,
        "package",
        &bound,
    );
    check_plan(expected_plan, &plan)?;
    let mut message = String::new();
    if let Some(note) = instance_note {
        // The request named an id the tree no longer carries; the reply says which instance it
        // followed, because a silent substitution would be a different answer to the same request.
        // 请求点名了一个树已不再持有的 id；回复说出跟到了哪个实例，因为静默替换就是对同一请求的另一个答案。
        message.push_str(&format!(" {note}\n"));
    }
    message.push_str(&format!(
        "move {} → {new_source}\n parent: {} → {}\n",
        face.path, face.module, new_module
    ));
    if let Some(directory) = kept_island {
        message.push_str(&format!(
            "       directory: kept in its mapped island `{directory}` (a rename under the same \
             parent does not leave the island)\n"
        ));
    }
    if name != face.registry_name {
        message.push_str(&format!(
            " display name: {} (the declaration keeps it; the file and its module change)\n",
            face.registry_name
        ));
    }
    for note in &kept {
        message.push_str(&format!(" {note}\n"));
    }
    for rewrite in &rewrites {
        message.push_str(&format!(" rewrite {}\n", rewrite.description));
    }
    message.push_str(&format!(" plan {plan}\n"));
    if destination != source_path && destination.exists() {
        return Err(format!(
            "`{}` already exists and a move does not overwrite one declaration with another. \
             Nothing changed.",
            destination.display()
        ));
    }
    // A preview is the real operation on a throwaway copy: the files are written there so the diff
    // and the reply describe what an apply would do, instead of a prediction. Only the project
    // target survives the call, so writing here is safe in both modes.
    // 预览是在一次性副本上真正执行操作：文件被写到副本里，因此 diff 与回复描述的是"应用会做什么"，
    // 而不是一次预测。只有项目目标会活过这次调用，因此在两种模式下写入都安全。
    //
    // **One transaction owns every byte of it.** The host-entry rewrites, the moved source, the
    // reference updates, the removed original, the graft records and the migration record are one
    // recoverable set: a failure in the last of them restores the first. Before this, publication
    // happened after the write list and its `?` returned directly, which is exactly the reproduced
    // defect — an error reply on a tree that had already moved.
    // **每一个字节都由同一个事务拥有。** 宿主入口改写、被搬源码、引用更新、被删除的原文件、嫁接记录与
    // 迁移记录是一个可恢复集合：最后一步失败会把第一步恢复回去。在此之前，发布发生在写入表之后、错误经 `?`
    // 直接返回——那正是已经复现的缺陷：回复是错误，树却已经搬完了。
    let mut writes: Vec<(std::path::PathBuf, &str)> = vec![(destination.clone(), updated.as_str())];
    for rewrite in &rewrites {
        writes.push((rewrite.path.clone(), rewrite.updated.as_str()));
    }
    let mut transaction = crate::run_method::authoring::write_transaction::WriteTransaction::new();
    let finish = (|| -> Result<(), String> {
        // 1. Host-entry declaration edits. They run in both modes (the preview copy is the throwaway
        //    one) and are captured before they are written, so a partial write of an entry file is
        //    restored like any other file.
        // 1. 宿主入口的声明改写。两种模式都跑（预览用的是副本），并在写入之前保存，因此入口文件"写了一半"
        //    也像其它文件一样被恢复。
        for line in super::grafts::apply_declaration_edits_in(&declaration_edits, &mut transaction)?
        {
            message.push_str(&format!(" {line}\n"));
        }
        // 2. A file cannot be both planned by the declaration editor and by the consumer
        //    replacement; if the old reference survives outside the edited lines, refuse instead of
        //    writing a half-followed tree. The rollback restores the edits step 1 already made.
        // 2. 一个文件不能同时由声明改写与消费者替换负责；若旧引用在改过的行之外仍存在，就拒绝，而不是写下
        //    只跟了一半的树。回滚会把第 1 步已经做的改写恢复回去。
        for file in &edited_files {
            let text = std::fs::read_to_string(file).map_err(|error| {
                format!("cannot read {} after editing: {error}", file.display())
            })?;
            if text.contains(&old_reference) {
                return Err(format!(
                    "{} declares a graft entry for `{}` and also references it elsewhere; rewriting \
                     both in one move is not wired yet — edit that file by hand.",
                    relative_to(&layout.package_root, file),
                    face.path
                ));
            }
        }
        // 3. The moved declaration and every reference that follows it.
        // 3. 被搬的声明，以及跟随它的每一处引用。
        for (path, text) in &writes {
            transaction.write(path, text.as_bytes())?;
        }
        // 4. The original file is removed only after its bytes exist at the destination.
        // 4. 原文件在它的字节已经存在于目的地之后才被删除。
        if source_path != destination {
            transaction.remove(&source_path)?;
        }
        // 5. Publication — the plans were made above, before the set was authorized, so nothing
        //    here decides **what** to write: it only writes what was already judged.
        // 5. 发布——计划在上面、在集合被授权之前就做好了，因此这里不决定**写什么**：它只写已经判定过的东西。
        if applying {
            message.push_str(&format!(" wrote {}\n", destination.display()));
            for planned in &graft_records {
                transaction.write(&planned.path, planned.text.as_bytes())?;
            }
            if let Some(record) = &record {
                super::publish_record(&mut transaction, record)?;
            }
        }
        // 6. The ledger. It is written in **both** modes — a preview shows it in the diff — and it
        //    belongs to the same transaction, so a later failure takes it back with everything else.
        //    It keeps the object's identity across the move, keeps both source instances resolvable,
        //    and moves an explicit directory mapping to where the file went.
        // 6. 台账。**两种模式**都写——预览要在 diff 里看到它——而且它属于同一个事务，因此后来任何失败都会
        //    把它连同其它一切收回去。它让对象跨这次搬动保持身份，让两个源码实例都仍可解析，并把显式目录映射
        //    挪到文件真正去的地方。
        if let Some(ledger) = &ledger {
            transaction.write(ledger.path(), ledger.bytes())?;
            message.push_str(&format!(" {}\n", ledger.summary()));
        }
        Ok(())
    })();
    if let Err(error) = finish {
        let recovery = transaction.rollback();
        return Err(format!("{error}; {}", recovery.describe()));
    }
    for planned in &graft_records {
        message.push_str(&format!(" {}\n", planned.description));
    }
    if let Some(record) = &record {
        message.push_str(&format!("record {}\n", record.path.display()));
        // Eviction runs only after the move landed: a failure to drop an old record is reported, not
        // a reason to undo a move that succeeded.
        // 淘汰只在搬动落地之后运行：没能丢掉旧记录是被报告出来的，而不是把一次成功的搬动撤回的理由。
        let dropped = super::prune(&record.directory, super::KEEP_RECORDS);
        if !dropped.is_empty() {
            message.push_str(&format!(
                "       pruned {} (the newest {} are kept): the mapping it carried is no longer on \
                 disk\n",
                dropped.join(", "),
                super::KEEP_RECORDS
            ));
        }
    } else {
        message.push_str(" (preview; apply to write)\n");
    }
    let _ = (root, work);
    Ok(Outcome {
        message,
        declaration: None,
        source: destination,
        moved: true,
        alternative: false,
        inherited: Vec::new(),
        consumers: Vec::new(),
    })
}

/// One file this move rewrites besides the moved declaration, with the bytes it started from.
/// 这次搬动除被搬声明之外还要改写的一个文件，连同它起始时的字节。
struct Rewrite {
    /// What the reply says changed, e.g. `src/…/reading.rs: old → new`.
    /// 回复里说改了什么，例如 `src/…/reading.rs: old → new`。
    description: String,
    /// The file on disk.
    /// 磁盘上的文件。
    path: PathBuf,
    /// The bytes the rewrite was planned from.
    /// 规划这次改写所依据的字节。
    original: String,
    /// The rewritten text.
    /// 改写后的文本。
    updated: String,
    /// Package-relative spelling, for the plan digest and the reply.
    /// 包内相对拼写，供计划摘要与回复使用。
    relative: String,
}

/// A path as the reply spells it: package-relative, with forward slashes.
/// 回复里的路径拼写：包内相对、正斜杠。
fn relative_to(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

/// Every `.rs` file the package may read, so a consumer of the moved alias follows it.
/// 包可以读取的每个 `.rs` 文件，使被搬别名的消费者跟着走。
///
/// Enumerated by the shared source scope, not by a walk of this module's own: a rewrite that picked
/// up a file through an undeclared link would **write** outside the package, which is the write-side
/// half of the same rule the readers obey. The scope is the one that also carries the project's
/// declared sources, so a declared materialization is **found** here; whether this move may write
/// through it is a separate decision the caller makes, because a declaration is a read permission.
/// 由共同来源范围枚举，而不是由本模块自己遍历：经未声明链接找到的文件会被**改写**到包外，而那正是读取方
/// 遵守的同一条规则的写入侧那一半。这个范围同时携带项目声明的来源，因此已声明的物化位置会在这里被**找到**；
/// 这次搬动是否可以穿过它写，是调用方另行作出的决定，因为声明是读权限。
fn rust_sources_under(root: &Path) -> Result<Vec<PathBuf>, String> {
    let scope = xirang_multilang::source_scope::SourceScope::with_sources(root)?;
    let found = scope.files(
        |path| path.extension().is_some_and(|extension| extension == "rs"),
        &["target", ".xirang", ".git", "node_modules", "crates"],
    )?;
    Ok(found.into_iter().map(|file| file.path).collect())
}

/// Whether one path's **real** location is outside the package root.
/// 一个路径的**真实**位置是否在包根之外。
pub(super) fn outside_package(root: &Path, path: &Path) -> Result<bool, String> {
    let real = std::fs::canonicalize(path)
        .map_err(|error| format!("cannot resolve {}: {error}", path.display()))?;
    let real_root = std::fs::canonicalize(root)
        .map_err(|error| format!("cannot resolve {}: {error}", root.display()))?;
    Ok(!real.starts_with(&real_root))
}

/// The digest a preview prints and an apply checks.
/// 预览打印、应用校验的摘要。
///
/// It binds the four facts a move is planned from — the stable object, its package-relative source,
/// the bytes that source held, and the destination — and nothing that depends on where the
/// workspace happens to sit on this machine, so the same tree previewed and applied under one root
/// agrees, while a changed byte, a moved declaration, or another destination does not.
/// 它绑定规划一次搬动所依据的四件事——稳定对象、它的包内相对源码、那份源码当时的字节、目的地——而不含
/// 任何依赖本机工作区位置的东西，因此同一棵树在同一根目录下预览与应用会一致，而改过的字节、挪过的声明或
/// 另一个目的地则不会。
pub(super) fn plan_digest(
    face: &FaceView,
    source: &str,
    content: &[u8],
    destination: &str,
    logical: &str,
    base: &str,
    files: &[(String, String)],
) -> String {
    // The logical path is the **relation** the move produces. It is bound because a request can
    // change the parent without changing the destination's directory (two parents can share one
    // directory), and a plan that described the old relation must not land the new one.
    // 逻辑路径是这次搬动产出的**关系**。它进摘要，是因为请求可以在不改目的地目录的情况下换父级
    // （两个父级可以共用一个目录），而描述旧关系的那份计划不得落下新关系。
    let mut canonical = format!(
        "move plan v1\nobject {}\nsource {source}\ncontent {}\ndestination {destination}\nlogical {logical}\nbase {base}\n",
        face.id,
        xirang_kernel::identity::sha256_hex(content),
    );
    for (path, digest) in files {
        canonical.push_str(&format!("file {path} {digest}\n"));
    }
    xirang_kernel::identity::sha256_hex(canonical.as_bytes())
}

/// Refuse a plan that no longer describes the move this request would make.
/// 拒绝一个不再描述本次请求将做出的搬动的计划。
pub(super) fn check_plan(expected: Option<&str>, actual: &str) -> Result<(), String> {
    match expected {
        Some(expected) if expected.trim() != actual => Err(format!(
            "the plan `{}` no longer describes this move: the declaration, the bytes it held, or \
             the destination has changed since it was previewed, so applying it would write \
             something the preview never showed. Nothing changed. Preview again and apply the plan \
             that preview prints",
            expected.trim()
        )),
        _ => Ok(()),
    }
}

/// Whether a path that may not exist yet would be written outside the package: its nearest
/// **existing** ancestor decides, because that is the part a link can already redirect.
/// 一个可能还不存在的路径是否会被写到包外：由它最近的**已存在**祖先决定，因为那正是链接此刻已经能改道的
/// 那一段。
pub(super) fn outside_package_or_ancestor(root: &Path, path: &Path) -> Result<bool, String> {
    let mut probe = path;
    while !probe.exists() {
        probe = probe
            .parent()
            .ok_or_else(|| format!("{} has no existing ancestor", path.display()))?;
    }
    outside_package(root, probe)
}
