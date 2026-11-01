//! Recoverable folder-backed module migration.
//! 可恢复的目录型模块迁移。

use super::*;

use crate::run_method::authoring::write_scope::ChangeKind;
use crate::run_method::authoring::write_transaction::WriteTransaction;

/// Rename a standard generated module and every authored child below it.
/// 重命名标准生成模块，并同步迁移其下的全部注册子对象。
pub(super) fn migrate_module_subtree(
    registry: &Registry,
    id: NodeId,
    source: PathBuf,
    mut face: FaceManifest,
    requested_module: &str,
) -> Result<AuthoringChange, String> {
    let old_dir = source
        .parent()
        .ok_or_else(|| "generated source has no module directory".to_owned())?
        .to_path_buf();
    let new_dir = old_dir
        .parent()
        .ok_or_else(|| "generated module directory has no parent".to_owned())?
        .join(requested_module);
    let old_file_name = source
        .file_stem()
        .and_then(|name| name.to_str())
        .unwrap_or_default();
    if old_file_name != face.values.get("module").map(String::as_str).unwrap_or("")
        || old_dir.file_name().and_then(|name| name.to_str()) != Some(old_file_name)
    {
        return Err("only standard `<module>/<module>.rs` faces can be renamed".to_owned());
    }
    if new_dir.exists() {
        return Err(format!("module `{requested_module}` already exists"));
    }
    // The destination is a **physical** fact and the spellings are **record** facts. They are the
    // same for a face under the walk root and differ for an explicitly placed one, whose spelling has
    // no path relative to the identity base — stripping that base used to refuse this rename with
    // "outside the package source tree", although the face is inside the package.
    // 目的地是**物理**事实，而拼写是**记录**事实。对遍历根之下的面两者相同；对显式落点的面不同——它的
    // 拼写没有相对身份基准的路径，而 strip 那个基准正是过去把这次改名拒成"在包源码树之外"的原因，
    // 尽管那个面就在包内。
    let layout = super::super::context::layout()?;
    let old_spelling = xirang_multilang::source_ref::record_spelling(
        &layout.package_root,
        &layout.identity_base,
        &source,
    );
    let old_relative = Path::new(&old_spelling)
        .parent()
        .ok_or_else(|| "generated source has no module spelling".to_owned())?
        .to_path_buf();
    let new_relative = old_relative
        .parent()
        .unwrap_or_else(|| Path::new(""))
        .join(requested_module);
    // The module chain is the discovery tree's answer, not the physical directory chain: an explicit
    // placement is a root-level node named by its stem, so `components/reading` is `reading`.
    // 模块链是发现树的答案，不是物理目录链：显式落点是按 stem 命名的根级节点，因此
    // `components/reading` 是 `reading`。
    let old_module_path = layout
        .module_chain(&source)
        .ok_or_else(|| "generated module has no Rust module chain".to_owned())?;
    let new_module_path = match old_module_path.rsplit_once("::") {
        Some((prefix, _)) => format!("{prefix}::{requested_module}"),
        None => requested_module.to_owned(),
    };
    let (old_source_prefix, new_source_prefix) =
        source_prefixes(&layout, &source, &old_relative, &new_relative);

    face.values
        .insert("module".to_owned(), requested_module.to_owned());
    face.values.insert(
        "source".to_owned(),
        normalized_path(&new_relative.join(format!("{requested_module}.rs"))),
    );
    if let Some(rule_path) = face.values.get_mut("registry_rule_path") {
        let old_segment = format!("/{}/", normalized_path(&old_relative));
        let new_segment = format!("/{}/", normalized_path(&new_relative));
        if rule_path.contains(&old_segment) {
            *rule_path = rule_path.replacen(&old_segment, &new_segment, 1);
        }
    }

    // **The complete write set is authorized before the first rename.** A structural rename moves a
    // directory and rewrites every declaration under it; when that directory is a declared-source
    // link, renaming it edits the third party's tree while the reply says "renamed". The set is the
    // directory this call removes, the one it creates, and every file it rewrites.
    // **在第一次改名之前授权完整写入集合。** 结构改名搬动一个目录并改写它下面的每条声明；当那个目录是
    // 已声明来源的链接时，改名就是在编辑第三方的树，而回复会说"已改名"。集合是本次调用移除的目录、创建的
    // 目录，以及它改写的每一份文件。
    let mut subtree = Vec::new();
    visit_rust_files(&old_dir, &mut subtree)?;
    authorize_migration(
        &[
            (
                old_dir.clone(),
                ChangeKind::Remove,
                format!("the module directory `{}`", old_dir.display()),
            ),
            (
                new_dir.clone(),
                ChangeKind::Write,
                format!("its destination `{}`", new_dir.display()),
            ),
        ],
        &subtree,
    )?;

    let moved_source = new_dir.join(format!("{old_file_name}.rs"));
    let renamed_source = new_dir.join(format!("{requested_module}.rs"));
    let new_spelling = xirang_multilang::source_ref::record_spelling(
        &layout.package_root,
        &layout.identity_base,
        &renamed_source,
    );
    let old_path = registry
        .path_for(id)
        .ok_or_else(|| format!("node `{id}` has no logical path in this tree"))?;
    let new_path = match old_path.rsplit_once('/') {
        Some((parent, _)) => format!("{parent}/{requested_module}"),
        None => requested_module.to_owned(),
    };
    let mut transaction = WriteTransaction::new();
    let outcome = (|| -> Result<(), String> {
        transaction.rename(&old_dir, &new_dir)?;
        transaction.rename(&moved_source, &renamed_source)?;
        record_migration_in_the_ledger(
            &layout,
            &MigrationRecord {
                id,
                kind: face.values.get("kind").map(String::as_str).unwrap_or(""),
                stable_name: face.values.get("stable_name").map(String::as_str),
                old_file: &source,
                new_file: &renamed_source,
                old_path: &old_path,
                new_path: &new_path,
                new_record_spelling: &new_spelling,
            },
            &mut transaction,
        )?;
        rewrite_migrated_sources(
            &new_dir,
            &source,
            &face,
            MigrationPaths {
                old_module_path: &old_module_path,
                new_module_path: &new_module_path,
                old_source_prefix: &old_source_prefix,
                new_source_prefix: &new_source_prefix,
            },
            &mut transaction,
        )?;
        let snapshots = generated_snapshots()?
            .into_iter()
            .filter(|snapshot| {
                let prefix = format!("{}/", normalized_path(&new_relative));
                snapshot.source.file
                    == normalized_path(&new_relative.join(format!("{requested_module}.rs")))
                    || snapshot.source.file.starts_with(&prefix)
            })
            .collect::<Vec<_>>();
        if snapshots.is_empty() {
            return Err("renamed module produced no registration face".to_owned());
        }
        registry
            .validate_snapshot_migration(id, snapshots)
            .map_err(|error| format!("registration rejected after rename:\n{error}"))?;
        Ok(())
    })();
    if let Err(error) = outcome {
        let recovery = transaction.rollback();
        return Err(format!("{error}; {}", recovery.describe()));
    }

    Ok(AuthoringChange {
        message: format!("renamed module `{old_file_name}` to `{requested_module}`"),
        source: renamed_source,
    })
}

/// Record a rename in the object ledger, inside the migration's own transaction.
/// 在一次改名的事务里，把这次搬动记进对象台账。
///
/// The ledger is not bookkeeping that can wait: the discovery walk finds an **explicitly placed** face
/// through it, so a transaction that moves the file but leaves the mapping on the old directory makes
/// the read-back find no face at all. Measured symptom: the rename moved two files, then refused with
/// "renamed module produced no registration face" and rolled back — the file was at the new path, the
/// ledger still described the old one.
/// 台账不是可以晚点补的记账：发现遍历正是**通过它**找到显式落点的面的，因此一次搬了文件、却把映射留在旧
/// 目录上的事务，会让读回一个面都找不到。实测症状：改名搬了两个文件，随后以"改名后的模块没有产出注册面"
/// 拒绝并回滚——文件已在新路径，台账还在描述旧的那一个。
///
/// The object keeps its identity (that is `plan_move`'s stronger guarantee), the mapping follows the
/// directory the file actually landed in, and both source instances stay resolvable.
/// 对象保持身份（这是 `plan_move` 更强的保证），映射跟着文件实际落到的目录走，两个源码实例都仍可解析。
struct MigrationRecord<'a> {
    id: NodeId,
    kind: &'a str,
    stable_name: Option<&'a str>,
    old_file: &'a Path,
    new_file: &'a Path,
    old_path: &'a str,
    new_path: &'a str,
    new_record_spelling: &'a str,
}

fn record_migration_in_the_ledger(
    layout: &crate::source_layout::SourceLayout,
    record: &MigrationRecord<'_>,
    transaction: &mut WriteTransaction,
) -> Result<(), String> {
    use xirang_kernel::identity::StableFaceId;
    use xirang_multilang::object_map::{ObjectMap, Observed};
    let namespace = super::super::context::authoring_namespace();
    let map = ObjectMap::read_from(&layout.package_root)?;
    // The seed is what the tree derives for this declaration (the author-declared `stable_name`, else
    // the logical path); `plan_move` replaces it with the ledger's answer when an entry already owns it.
    // 种子是树为这条声明推导出的值（作者声明的 `stable_name`，否则逻辑路径）；已有条目拥有它时，
    // `plan_move` 会用台账的答案替换它。
    let seed = match record.stable_name {
        Some(name) if !name.is_empty() => StableFaceId::from_name(name),
        _ => StableFaceId::from_namespaced_name(&namespace, record.old_path),
    };
    // The ledger's third column is the **package-relative** source (its documented basis), not the
    // record spelling the identity uses. Passing the identity spelling for a `src/` face recorded
    // `panel/panel.rs`, and every reader that trusts the ledger's basis then looked for
    // `<package>/panel/panel.rs` — a file that does not exist (measured: Studio lost the selection
    // after a rename).
    // 台账第三列是**包内相对**源码（它的文档基准），不是身份所用的记录拼写。对一个 `src/` 之下的面传入身份
    // 拼写会记下 `panel/panel.rs`，而每个相信台账基准的读取方随后去找 `<包>/panel/panel.rs`——一个不存在的
    // 文件（实测：Studio 在一次改名后丢失了选择）。
    let old_source =
        xirang_multilang::source_ref::package_source(&layout.package_root, record.old_file);
    let new_source =
        xirang_multilang::source_ref::package_source(&layout.package_root, record.new_file);
    let before = Observed::new(seed, record.old_path, old_source);
    let stable = map
        .own(&before)?
        .map_or(before.stable, |entry| entry.stable);
    let after = Observed::new(stable, record.new_path, new_source);
    let new_id = NodeId::from_namespaced_path(&namespace, record.new_record_spelling, record.kind);
    let Some(write) = crate::object_ledger::plan_move(
        &layout.package_root,
        &before,
        record.id,
        &after,
        new_id,
        // A structural rename stays where its declaration family lives, so an explicit mapping
        // follows the file rather than being cleared.
        // 结构改名后声明仍住在它这一族所在的地方，因此显式映射跟着文件走，而不是被清掉。
        true,
    )?
    else {
        return Ok(());
    };
    transaction.write_atomic(write.path(), write.bytes())
}

/// Authorize a structural migration's write set through the one write-scope rule.
/// 经那一条写入作用域规则授权一次结构迁移的写入集合。
///
/// `members` are the paths with a specific meaning (the directory that moves, its destination);
/// `files` are every file the migration rewrites.
/// `members` 是有特定含义的路径（被搬的目录、它的目的地）；`files` 是迁移改写的每一份文件。
fn authorize_migration(
    members: &[(PathBuf, ChangeKind, String)],
    files: &[PathBuf],
) -> Result<(), String> {
    use crate::run_method::authoring::write_scope::{Change, WriteScope, describe_outside};
    let root = super::super::context::package_root();
    let mut changes = members
        .iter()
        .map(|(path, kind, subject)| Change {
            path: path.clone(),
            kind: *kind,
            subject: subject.clone(),
        })
        .collect::<Vec<_>>();
    changes.extend(files.iter().map(|file| Change {
        path: file.clone(),
        kind: ChangeKind::Write,
        subject: format!("the declaration in `{}`", file.display()),
    }));
    if let Err(outside) = WriteScope::new(&root)?.authorize(&changes) {
        return Err(format!(
            "this structural change would write outside the package `{}`, so nothing was changed.\n{}",
            root.display(),
            describe_outside(&outside, &root)
        ));
    }
    Ok(())
}

struct MigrationPaths<'a> {
    old_module_path: &'a str,
    new_module_path: &'a str,
    old_source_prefix: &'a str,
    new_source_prefix: &'a str,
}

/// The literal source prefixes a migration rewrite looks for, by where the face lives.
/// 迁移改写按面所在位置所寻找的字面源码前缀。
///
/// A face under the walk root is named by the historical `src/…` spelling in the artifacts that carry
/// source paths (a registry rule's path, for one); an explicitly placed face is named by its
/// package-relative spelling, which never starts with `src/`. One rule, decided once here.
/// 遍历根之下的面，在携带源码路径的产物里（例如注册规则路径）用历史上的 `src/…` 拼写命名；显式落点的面
/// 用它的包内相对拼写命名，那个拼写从不以 `src/` 开头。一条规则，在这里决定一次。
fn source_prefixes(
    layout: &crate::source_layout::SourceLayout,
    source: &Path,
    old_relative: &Path,
    new_relative: &Path,
) -> (String, String) {
    let old = normalized_path(old_relative);
    let new = normalized_path(new_relative);
    if source.starts_with(&layout.scan_root) {
        // A walked face has always been named by the historical `src/…` spelling in the artifacts that
        // carry source paths; keeping that spelling is what makes this rename a rename and not a
        // re-spelling of every path in the subtree.
        // 遍历到的面在携带源码路径的产物里一直用历史上的 `src/…` 拼写命名；保留它，才让这次操作是一次
        // 改名，而不是把子树里每条路径都换一种拼法。
        (format!("src/{old}/"), format!("src/{new}/"))
    } else {
        // An explicitly placed face's artifacts carry its package-relative spelling, which has no
        // `src/` in front of it.
        // 显式落点的面的产物携带它的包内相对拼写，前面没有 `src/`。
        (format!("{old}/"), format!("{new}/"))
    }
}

fn rewrite_migrated_sources(
    new_dir: &Path,
    old_source: &Path,
    root_face: &FaceManifest,
    paths: MigrationPaths<'_>,
    transaction: &mut WriteTransaction,
) -> Result<(), String> {
    let old_source_name = old_source
        .file_name()
        .ok_or_else(|| "generated source has no file name".to_owned())?;
    let new_source = new_dir.join(old_source_name).with_file_name(format!(
        "{}.rs",
        root_face
            .values
            .get("module")
            .map(String::as_str)
            .unwrap_or_default()
    ));
    let old_prefix = paths.old_source_prefix;
    let new_prefix = paths.new_source_prefix;
    let rust_old = format!("crate::{}", paths.old_module_path);
    let rust_new = format!("crate::{}", paths.new_module_path);
    let mut files = Vec::new();
    visit_rust_files(new_dir, &mut files)?;
    for path in files {
        let original = fs::read_to_string(&path)
            .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
        let mut updated = replace_source_prefix(
            &replace_module_path(&original, &rust_old, &rust_new),
            old_prefix,
            new_prefix,
        );
        if path == new_source {
            updated = replace_source_prefix(
                &replace_module_path(
                    &xirang_kernel::syntax::replace_face_region(
                        &original,
                        &root_face.render_source()?,
                    )
                    .map_err(|error| error.to_string())?,
                    &rust_old,
                    &rust_new,
                ),
                old_prefix,
                new_prefix,
            );
        }
        if updated != original {
            transaction.write_atomic(&path, updated.as_bytes())?;
        }
    }
    Ok(())
}

/// Replace a Rust module path only where it is a whole sequence of `::` segments.
/// 只在整段 `::` 序列处替换 Rust 模块路径。
///
/// A plain `str::replace` matched text prefixes, so renaming `control` rewrote a
/// sibling's `crate::run_method::control_extra::NODE_ID` into `crate::run_method::widget_extra::NODE_ID` —
/// a module silently pointing at something that does not exist, with the write
/// reported as "changed". The boundary is a `::` segment, which is the same rule
/// `build_method`'s scope view states for subtree selection (`control` keeps
/// `control::object::button` and never `control_extra`).
///
/// One suffix belongs to the renamed module itself rather than to a longer
/// sibling name: the generated alias `{module}_object!` names the very face whose
/// module moved. A `::`-only rule left `crate::run_method::control_object!` behind inside a
/// file whose parent had become `widget`, and the parser then refused that file by
/// name (`registration macro \`control_object!\` does not match parent \`widget\``).
/// The alias is matched together with its `!`, so a sibling's
/// `control_extra_object!` is still left alone.
/// 有一个后缀属于被改名的模块自己，而不属于更长的兄弟名：生成的别名 `{module}_object!` 指的正是
/// 模块搬走了的那个面。只认 `::` 的规则会把 `crate::run_method::control_object!` 留在父级已变成 `widget` 的
/// 文件里，解析器随后按名拒绝该文件（`registration macro \`control_object!\` does not match
/// parent \`widget\``）。别名连同它的 `!` 一起匹配，因此兄弟的 `control_extra_object!` 仍然原样保留。
/// 朴素的 `str::replace` 匹配文本前缀，因此把 `control` 改名会把兄弟模块的
/// `crate::run_method::control_extra::NODE_ID` 改写成 `crate::run_method::widget_extra::NODE_ID`——一个悄悄指向不存在
/// 东西的模块，而写入被报成"已改变"。边界是 `::` 段，这正是 `build_method` 的作用域视图为子树选择
/// 写下的规则（`control` 保留 `control::object::button`，绝不保留 `control_extra`）。
fn replace_module_path(text: &str, old: &str, new: &str) -> String {
    let mut output = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(index) = rest.find(old) {
        let starts_at_a_boundary = index == 0 || !is_identifier_byte(rest.as_bytes()[index - 1]);
        let after = index + old.len();
        let ends_at_a_boundary = after >= rest.len()
            || !is_identifier_byte(rest.as_bytes()[after])
            || rest[after..].starts_with("_object!");
        output.push_str(&rest[..index]);
        output.push_str(if starts_at_a_boundary && ends_at_a_boundary {
            new
        } else {
            old
        });
        rest = &rest[after..];
    }
    output.push_str(rest);
    output
}

/// Whether a byte can be part of an identifier or a module segment.
/// 某个字节能否属于一个标识符或模块段。
fn is_identifier_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

/// Replace a `src/<module>/` source prefix only where it starts a path.
/// 只在路径起始处替换 `src/<module>/` 源前缀。
///
/// The other half of a migration rewrite is bounded by `::` segments; this half is a
/// path, so its bound is the byte in front of it. A path that merely *contains*
/// `src/control/` — a sibling pointing at another tree, e.g. `dep/src/control/x.rs` —
/// keeps its own meaning, and the trailing `/` is the far bound, which is why
/// `src/control_extra/` was never at risk from this one. A plain `str::replace` had
/// no bound on the left at all, so the whole migrated subtree (a sibling included)
/// could be rewritten silently (audit `LGC-LG-10`).
/// 迁移改写的另一半以 `::` 段为界；这一半是路径，它的界是它前面的那个字节。只是**包含**
/// `src/control/` 的路径——例如兄弟模块指向另一棵树的 `dep/src/control/x.rs`——保持自己的含义；
/// 尾部的 `/` 是另一侧的界，因此 `src/control_extra/` 从来不受这一处影响。朴素的 `str::replace`
/// 左侧完全没有界，于是整棵被迁移的子树（包括兄弟模块）都可能被静默改写（审计 `LGC-LG-10`）。
fn replace_source_prefix(text: &str, old: &str, new: &str) -> String {
    let mut output = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(index) = rest.find(old) {
        let starts_a_path = index == 0 || !is_path_byte(rest.as_bytes()[index - 1]);
        output.push_str(&rest[..index]);
        output.push_str(if starts_a_path { new } else { old });
        rest = &rest[index + old.len()..];
    }
    output.push_str(rest);
    output
}

/// Whether a byte can be part of a path.
/// 某个字节能否属于路径。
fn is_path_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'.' | b'/' | b'\\' | b'-')
}

#[cfg(test)]
#[path = "migration_tests.rs"]
mod migration_tests;

fn visit_rust_files(directory: &Path, files: &mut Vec<PathBuf>) -> Result<(), String> {
    for entry in fs::read_dir(directory)
        .map_err(|error| format!("cannot scan {}: {error}", directory.display()))?
    {
        let path = entry
            .map_err(|error| format!("cannot scan {}: {error}", directory.display()))?
            .path();
        if path.is_dir() {
            visit_rust_files(&path, files)?;
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("rs") {
            files.push(path);
        }
    }
    Ok(())
}

/// Change a face kind by migrating its identity in one validated transaction.
/// 修改注册面的 kind，并在一次校验事务中迁移其身份。
///
/// `kind` participates in `NodeId`, so changing it cannot use an in-place
/// replacement. The source file stays where it is; descendants are reparsed
/// so their parent IDs follow the new kind, then the complete subtree is
/// validated before the caller reloads the live registry.
/// `kind` 是 `NodeId` 的组成部分，不能原地替换。源码文件位置保持不变，
/// 重新解析后代以跟随新的父级身份，并在刷新实时注册树前校验整棵子树。
pub(super) fn migrate_kind_subtree(
    registry: &Registry,
    id: NodeId,
    source: PathBuf,
    face: FaceManifest,
) -> Result<AuthoringChange, String> {
    let old_source = fs::read_to_string(&source)
        .map_err(|error| format!("cannot read {}: {error}", source.display()))?;
    let mut face = face;
    face.set_identity(id);
    let rendered = face.render_source()?;
    // A kind change rewrites the face and re-validates the subtree; the write set is the face plus
    // every declaration the validation reads back, so it is authorized the same way.
    // 改 kind 会重写该面并重新校验子树；写入集合是该面加上校验会读回的每条声明，因此用同样的方式授权。
    let directory = source.parent().unwrap_or_else(|| Path::new(""));
    let mut subtree = Vec::new();
    visit_rust_files(directory, &mut subtree)?;
    authorize_migration(
        &[(
            source.clone(),
            ChangeKind::Write,
            format!("the declaration `{}`", source.display()),
        )],
        &subtree,
    )?;
    let layout = super::super::context::layout()?;
    let relative = xirang_multilang::source_ref::record_spelling(
        &layout.package_root,
        &layout.identity_base,
        &source,
    );
    let parent = Path::new(&relative)
        .parent()
        .map(normalized_path)
        .unwrap_or_default();
    let prefix = if parent.is_empty() {
        String::new()
    } else {
        format!("{parent}/")
    };
    let mut transaction = WriteTransaction::new();
    let outcome = (|| -> Result<(), String> {
        transaction.write_atomic(&source, rendered.as_bytes())?;
        let snapshots = generated_snapshots()?
            .into_iter()
            .filter(|snapshot| {
                snapshot.source.file == relative
                    || (!prefix.is_empty() && snapshot.source.file.starts_with(&prefix))
            })
            .collect::<Vec<_>>();
        if snapshots.is_empty() {
            return Err("kind migration produced no registration face".to_owned());
        }
        registry
            .validate_snapshot_migration(id, snapshots)
            .map_err(|error| format!("registration rejected after kind change:\n{error}"))?;
        Ok(())
    })();
    if let Err(error) = outcome {
        let recovery = transaction.rollback();
        return Err(format!("{error}; {}", recovery.describe()));
    }
    let _ = old_source;
    Ok(AuthoringChange {
        message: format!("updated kind in {}", source.display()),
        source,
    })
}
