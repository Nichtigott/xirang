//! Writing a planned partition: the ghost's three files, and the workspace config they need.
//! 写下一次已规划的拆分：幽灵的三份文件，以及它们所需的那份工作区配置。
//!
//! The planner decides; this module is the mechanical half. Two rules shape it:
//! 规划器做决定；本模块是机械的那一半。两条规则塑造了它：
//!
//! 1. **Every write is idempotent, compares content, and carries the tree's generation** (`write_if_changed_at`): running the action
//!    twice changes nothing the second time, because a partition is a thing you commit, and a
//!    commit-shaped action that rewrites identical bytes is a diff nobody asked for.
//!    **每次写入都幂等、先比内容、并带着这棵树的代次**（`write_if_changed_at`）：这个动作跑第二遍不会改任何东西，因为一次拆分是
//!    要提交的东西，而一个把相同字节重写一遍的提交形状动作，是一份没人要的 diff。
//! 2. **The workspace config is merged, never written**: `rustflags` are per **invocation** and cargo
//!    finds config from the current directory, so the remap has to live at the workspace root — a file
//!    the user owns. Keys this action does not understand are left exactly as they are, and a shape it
//!    cannot merge safely is **refused by name** with the lines to add by hand.
//!    **工作区配置是合并的、从不整体写下**：`rustflags` 是每次调用生效的，而 cargo 从当前目录找 config，
//!    因此 remap 必须住在工作区根——那是用户拥有的文件。本动作看不懂的键**原样保留**，而它无法安全合并的
//!    形状会被**点名拒绝**，并给出该手写上去的那几行。

use std::fs;
use std::path::{Path, PathBuf};

use super::crate_facade::PlannedFacade;
use super::crate_members;
use super::crate_plan::{GENERATED_MARKER, PlannedCrate};
use super::crate_release::ReleasePackage;
use super::crate_write_backup::Backup;
use super::crate_write_host_edges::{relative, relative_from_path};
use super::discovery_cache::write_if_changed_at;

/// What one write pass changed, for the reply the caller renders.
/// 一次写入改了哪些东西，供调用方渲染回复。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Written {
    /// Files this pass created or rewrote, relative to the workspace root.
    /// 本次创建或重写的文件，相对工作区根。
    pub(crate) files: Vec<String>,
    /// Whether the workspace config was touched (false when it already carried every entry).
    /// 工作区配置是否被改动（它已经带着每一条时是 false）。
    pub(crate) config_changed: bool,
    /// Whether the host manifest gained dependency edges (false when it already had them).
    /// 宿主清单是否新增了依赖边（已有全部边时为 false）。
    pub(crate) host_dependencies_changed: bool,
    /// Whether the workspace manifest's `members` was touched (false when there is no enclosing
    /// workspace, or when it already listed every generated package).
    /// 工作区清单的 `members` 是否被改动（没有外层工作区、或它已经列出每一个生成包时是 false）。
    pub(crate) members_changed: bool,
}

/// Write every planned crate, then make sure the workspace config carries the remap and the
/// workspace manifest lists the new packages.
/// 写下每个已规划的 crate，然后确保工作区配置带着那些 remap、且工作区清单列出了这些新包。
///
/// `root` is the directory cargo reads config from when it builds the generated packages, and
/// `workspace` is the workspace manifest they have to be members of — `None` when the host is not in
/// one. Two callers pass both — the write half and the revert half — so the decision lives in one
/// place: a package generated beside a host that belongs to a workspace has to be a member of it, and
/// without a workspace the generated packages are standalone siblings with nothing to materialize
/// them into.
/// `root` 是 cargo 构建这些生成包时读取配置的目录，`workspace` 是它们必须成为成员的那份工作区清单——宿主
/// 不在任何工作区里时是 `None`。两个调用方都传这两样——写入那一半与撤回那一半——因此判断只有一处：生成在
/// 某个属于工作区的宿主旁边的包必须是它的成员，而没有工作区时，生成的包就是彼此独立的同级包，没有任何东西可以
/// 把它们物化进去。
pub(crate) fn write_partition(
    root: &Path,
    workspace: Option<&Path>,
    planned: &[PlannedCrate],
    facade: Option<&PlannedFacade>,
) -> Result<Written, String> {
    write_partition_with_host(root, workspace, planned, facade, None, &[])
}

/// The same write, with the host manifest dependency edges a partition needs.
/// 同一个写入，另外写下分区所需的宿主清单依赖边。
pub(crate) fn write_partition_with_host(
    root: &Path,
    workspace: Option<&Path>,
    planned: &[PlannedCrate],
    facade: Option<&PlannedFacade>,
    host_manifest: Option<&Path>,
    host_dependencies: &[(String, String)],
) -> Result<Written, String> {
    let mut written = Written::default();
    let generation = super::discovery_cache::generation(root);
    let mut remap: Vec<(String, String)> = Vec::new();
    let mut directories: Vec<(&Path, &str)> = planned
        .iter()
        .map(|planned| (planned.directory.as_path(), planned.package.as_str()))
        .collect();
    if let Some(facade) = facade {
        directories.push((facade.directory.as_path(), facade.package.as_str()));
    }
    for (directory, _) in &directories {
        if !directory.starts_with(root) {
            return Err(format!(
                "add_crates: `{}` is outside the partition root `{}`; nothing was written",
                directory.display(),
                root.display()
            ));
        }
    }
    if std::fs::metadata(root)
        .map(|meta| meta.permissions().readonly())
        .unwrap_or(true)
    {
        return Err(format!(
            "add_crates: `{}` is not writable; nothing was written",
            root.display()
        ));
    }
    for planned in planned {
        remap.extend(planned.remap.iter().cloned());
    }
    remap.sort();
    remap.dedup();
    // Plan every mutation **before** the first write: the config merge and the member list are the
    // two places that can refuse a shape, and clear_generated is destructive. Computing both here is
    // what makes a refusal leave the tree untouched.
    // 第一次写入**之前**把每一处变更都规划好：配置合并与成员清单是两处会拒绝某种形状的地方，而
    // clear_generated 是破坏性的。把它们都算在这里，拒绝才让树保持原样。
    let config = planned_workspace_config(root, &remap, generation)?;
    let mut members = members_edit(
        workspace,
        &directories
            .iter()
            .map(|(directory, _)| *directory)
            .collect::<Vec<_>>(),
        false,
    )?;
    // The host's dependency edges are planned with everything else, before the first write. A host
    // face outside a claimed subtree names that subtree's types through `crate::…`; the host therefore
    // has to depend on the generated crate that now owns them. The reverse edge does not exist
    // (objects/facade depend on the host, never the host on the facade), so no cycle is introduced.
    // 宿主依赖边与其它变更一起、在第一次写入之前规划。被认领子树之外的宿主面经 `crate::…` 点名那些类型，
    // 因此宿主必须依赖现在拥有它们的生成 crate。反向边不存在（objects/facade 依赖宿主，宿主不依赖
    // facade），因此不会成环。
    // The host manifest may be the same file the member list goes into (a standalone host is its own
    // workspace root). Compose the dependency lines **onto the member-list text**, never onto the
    // original bytes: writing the second plan over the first would silently drop `[workspace]`.
    // 宿主清单可能就是成员清单所在的那份文件（独立宿主自己是工作区根）。依赖行必须叠加在**成员清单文本**
    // 上，而不是原始字节上：把第二份计划覆盖写上去，会静默丢掉 `[workspace]`。
    let member_base = members
        .as_ref()
        .filter(|(path, _)| Some(path.as_path()) == host_manifest)
        .map(|(_, text)| text.as_str());
    let host_edit = match host_manifest {
        Some(manifest) => planned_host_dependencies(manifest, host_dependencies, member_base)?,
        None => None,
    };
    let mut members_combined = false;
    if let (Some((member_path, _)), Some((host_path, _))) = (&members, &host_edit)
        && member_path == host_path
    {
        // One file, one write: the host edit already carries the member list.
        members = None;
        members_combined = true;
    }
    let mut packages: Vec<(&Path, &str, &str, &str, &str)> = planned
        .iter()
        .map(|planned| {
            (
                planned.directory.as_path(),
                planned.cargo_toml.as_str(),
                planned.lib_rs.as_str(),
                planned.aliases_rs.as_str(),
                planned.build_rs.as_str(),
            )
        })
        .collect();
    if let Some(facade) = facade {
        packages.push((
            facade.directory.as_path(),
            facade.cargo_toml.as_str(),
            facade.lib_rs.as_str(),
            facade.aliases_rs.as_str(),
            facade.build_rs.as_str(),
        ));
    }
    // Back up everything this pass may replace, so a failed write restores exactly what was there.
    // 备份本次可能替换的一切，使写入失败时精确恢复原状。
    let backup = Backup::take(
        root,
        &directories,
        config.as_ref(),
        members.as_ref(),
        host_edit.as_ref(),
    )?;
    let result = (|| -> Result<(), String> {
        clear_generated(root, &directories, &mut written)?;
        for (directory, cargo_toml, lib_rs, aliases_rs, build_rs) in &packages {
            for (path, content) in [
                (directory.join("Cargo.toml"), *cargo_toml),
                (directory.join("src/lib.rs"), *lib_rs),
                (directory.join("src/xirang_aliases.rs"), *aliases_rs),
                (directory.join("build.rs"), *build_rs),
            ] {
                write_ghost_file(&path, content, generation)?;
                written.files.push(relative(root, &path));
            }
        }
        if let Some((path, text)) = &config {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)
                    .map_err(|error| format!("cannot create {}: {error}", parent.display()))?;
            }
            write_if_changed_at(path, text, generation)?;
            written.config_changed = true;
        }
        if let Some((path, text)) = &members {
            crate::build_method::discovery_cache::write_verbatim_at(path, text)?;
            written.members_changed = true;
        }
        if let Some((path, text)) = &host_edit {
            crate::build_method::discovery_cache::write_verbatim_at(path, text)?;
            written.host_dependencies_changed = true;
            if members_combined {
                written.members_changed = true;
            }
        }
        Ok(())
    })();
    if let Err(error) = result {
        let restored = backup.restore();
        return Err(format!("{error}; {}", restored));
    }
    written.files.sort();
    written.files.dedup();
    Ok(written)
}

/// The workspace config merge, computed but not yet written.
/// 工作区配置合并，只计算、尚未写入。
fn planned_workspace_config(
    workspace_root: &Path,
    remap: &[(String, String)],
    _generation: u64,
) -> Result<Option<(PathBuf, String)>, String> {
    let path = workspace_root.join(".cargo/config.toml");
    let entries: Vec<String> = remap
        .iter()
        .map(|(from, to)| format!("\"--remap-path-prefix={from}={to}\"",))
        .collect();
    let text = match fs::read_to_string(&path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            let content = format!(
                "# Added by XiRang's partition action: the remap keeps every mounted face's `file!()`\n\
                 # spelling identical to the host's, which is what keeps its identity (audit `M7`, P3.3).\n\
                 # 由 XiRang 的拆分动作添加：remap 让每个被挂载面的 `file!()` 拼写与宿主一致，\n\
                 # 而这正是保住它身份的东西（审计 `M7`，P3.3）。\n\
                 [build]\nrustflags = [{}]\n",
                entries.join(", ")
            );
            return Ok(Some((path, content)));
        }
        Err(error) => {
            return Err(format!(
                "add_crates: cannot read {}: {error}",
                path.display()
            ));
        }
    };
    let missing: Vec<&String> = entries
        .iter()
        .filter(|entry| !text.contains(entry.as_str()))
        .collect();
    if missing.is_empty() {
        return Ok(None);
    }
    let merged = match merge_rustflags(&text, &missing) {
        Some(merged) => merged,
        None => {
            return Err(format!(
                "add_crates: {} already has a `rustflags` this action will not merge into (it is not a \
                 single-line array). Add these entries to that array by hand, then run this again:\n{}",
                path.display(),
                missing
                    .iter()
                    .map(|entry| format!("    {entry}"))
                    .collect::<Vec<_>>()
                    .join("\n")
            ));
        }
    };
    Ok(Some((path, merged)))
}

pub(crate) fn guard_shape(directories: &[&Path], release: bool) -> Result<(), String> {
    for directory in directories {
        let directory = *directory;
        let manifest = directory.join("Cargo.toml");
        let Ok(text) = std::fs::read_to_string(&manifest) else {
            continue;
        };
        let written_as_release = text.contains("shape = \"release\"");
        if written_as_release != release {
            return Err(format!(
                "add_crates: {} was written as the {} shape, and this run asked for the {} shape. \
                 The two share their directories, so one is taken back before the other is written.\
                 \nway forward: run `xirang crates --revert`, then write the shape you want",
                directory.display(),
                if written_as_release {
                    "release"
                } else {
                    "development"
                },
                if release { "release" } else { "development" },
            ));
        }
    }
    Ok(())
}

/// The directory cargo reads config from when it builds the generated packages, and the workspace
/// manifest they are materialized into (`None` when the host belongs to no workspace).
/// cargo 构建这些生成包时读取配置的目录，以及把它们物化进去的那份工作区清单（宿主不属于任何工作区时是
/// `None`）。
///
/// The config location is **not** the host package when there is no enclosing workspace: the generated
/// packages are the host's siblings, cargo finds config from the directory it is invoked in and that
/// directory's ancestors, and a `.cargo/config.toml` inside the host is never read while cargo builds
/// a sibling. Measured: with the config written inside the host, the remap never reached the ghost and
/// every mounted face failed `assert_static_identity` (audit `M7`, §M7.40). The directory both the host
/// and its siblings share is the parent.
/// 没有外层工作区时，配置的位置**不是**宿主包本身：生成的包是宿主的同级包，cargo 从它被调用的目录及其祖先
/// 目录找配置，而宿主的 `.cargo/config.toml` 在 cargo 构建一个同级包时永远不会被读到。实测：配置写在宿主
/// 里面时，remap 从未到达幽灵，于是每个被挂载的面都让 `assert_static_identity` 失败（审计 `M7`，§M7.40）。
/// 宿主与它的同级包共同拥有的那个目录，就是父目录。
///
/// Write a release-shaped partition: the same packages, each carrying the sources its build reads.
/// 写下发布形状的拆分：同一批包，每个都携带自己构建要读的源码。
///
/// No workspace config and no remap: a release package reads its own `src/`, so there is nothing to
/// remap — that is the whole point of the shape (audit `M7`, §M7.42). The member list is still
/// materialized when the host lives in a workspace, because cargo refuses a package that sits under a
/// workspace it is not listed in, exactly as in the development shape.
/// 没有工作区配置、也没有 remap：发布包读的是它自己的 `src/`，因此没有任何东西需要 remap——那正是这个形状的
/// 要点（审计 `M7`，§M7.42）。宿主位于工作区里时成员清单照样物化，因为 cargo 会拒绝一个位于它未被列入的工作区
/// 之下的包，与开发形状完全相同。
pub(crate) fn write_release(
    root: &Path,
    workspace: Option<&Path>,
    packages: &[ReleasePackage],
    host_root: &Path,
) -> Result<Written, String> {
    // Before a single file lands: a release facade reaches the host's tree through cargo, and a host cargo
    // will not hand over is a refusal rather than a package that compiles zero faces (see
    // [`super::crate_release::refuse_binary_host`]). Checked here because this is the one function all three
    // writers call — the CLI, the bridge and Studio — while `plan_facade` is also what a read-only view runs.
    // 在任何文件落地之前：发布 facade 经 cargo 到达宿主的树，而 cargo 不肯交出的宿主是一句拒绝，而不是一个编译
    // 零个面的包（见 [`super::crate_release::refuse_binary_host`]）。在这里查，因为这是三个写入方——CLI、桥与
    // Studio——都会调用的唯一函数，而 `plan_facade` 也是只读视图会跑的东西。
    super::crate_release::refuse_binary_host(host_root)?;
    use super::crate_write_host_edges::host_needs_fragment;
    // A host that carries `[workspace]` in its own manifest **is** the workspace root, even when no
    // directory above it is one; its release members have to be listed there.
    // 清单里带 `[workspace]` 的宿主**就是**工作区根，即使它上面没有任何工作区目录；它的发布成员必须列在那里。
    let host_is_workspace = std::fs::read_to_string(host_root.join("Cargo.toml"))
        .is_ok_and(|text| text.lines().any(|line| line.trim() == "[workspace]"));
    let workspace = if host_is_workspace {
        Some(host_root)
    } else {
        workspace
    };
    let generation = super::discovery_cache::generation(root);
    let mut written = Written::default();
    // The release host may own a face whose type imports now live in a generated crate (a move after
    // the partition). Those host→crate edges are part of the same write plan.
    // 发布形状的宿主可能拥有某个面，而它 import 的类型现在住在生成 crate 里（分区之后又 move）。这些
    // 宿主→生成 crate 的依赖边属于同一份写入计划。
    let host_dependencies: Vec<(String, String)> = packages
        .iter()
        .filter(|package| {
            !package.package.ends_with("-facade")
                && host_needs_fragment(host_root, &package.subtrees)
        })
        .map(|package| {
            (
                package.package.clone(),
                relative_from_path(host_root, &package.directory),
            )
        })
        .collect();
    let directories: Vec<(&Path, &str)> = packages
        .iter()
        .map(|package| (package.directory.as_path(), package.package.as_str()))
        .collect();
    clear_generated(root, &directories, &mut written)?;
    let mut members = members_edit(
        workspace,
        &directories
            .iter()
            .map(|(directory, _)| *directory)
            .collect::<Vec<_>>(),
        false,
    )?;
    let member_base = members
        .as_ref()
        .filter(|(path, _)| path == &host_root.join("Cargo.toml"))
        .map(|(_, text)| text.as_str());
    let host_edit = if host_root.join("Cargo.toml").is_file() {
        planned_host_dependencies(
            &host_root.join("Cargo.toml"),
            &host_dependencies,
            member_base,
        )?
    } else {
        None
    };
    if member_base.is_some() && host_edit.is_some() {
        members = None;
    }
    for package in packages {
        for (path, content) in [
            (package.directory.join("Cargo.toml"), &package.cargo_toml),
            (package.directory.join("src/lib.rs"), &package.lib_rs),
            (
                package.directory.join("src/xirang_aliases.rs"),
                &package.aliases_rs,
            ),
            (package.directory.join("build.rs"), &package.build_rs),
        ] {
            write_ghost_file(&path, content, generation)?;
            written.files.push(relative(root, &path));
        }
        for (source, destination) in &package.copies {
            let text = fs::read_to_string(source).map_err(|error| {
                format!(
                    "add_crates: cannot read {} to copy it into {}: {error}",
                    source.display(),
                    package.package
                )
            })?;
            let path = package.directory.join(destination);
            // A copy is the host's bytes: it carries no generation of ours (see `write_verbatim_at`).
            // 副本是宿主的字节：它不带我们的代次（见 `write_verbatim_at`）。
            crate::build_method::discovery_cache::write_verbatim_at(&path, &text)?;
            written.files.push(relative(root, &path));
        }
    }
    if let Some((path, text)) = members {
        // The workspace manifest is the host's file that this action only edits, so it is not decorated either.
        // 工作区清单是宿主自己的文件、本动作只做编辑，因此它同样不被装饰。
        crate::build_method::discovery_cache::write_verbatim_at(&path, &text)?;
        written.members_changed = true;
    }
    if let Some((path, text)) = &host_edit {
        crate::build_method::discovery_cache::write_verbatim_at(path, text)?;
        written.host_dependencies_changed = true;
    }
    written.files.sort();
    written.files.dedup();
    Ok(written)
}

/// Remove the generated packages that are already there, after checking they are this action's.
/// 删掉已经在那里的生成包，先确认它们是本动作的产物。
fn clear_generated(
    root: &Path,
    directories: &[(&Path, &str)],
    written: &mut Written,
) -> Result<(), String> {
    for (directory, package) in directories {
        let directory = *directory;
        if !directory.exists() {
            continue;
        }
        let marker = [directory.join("Cargo.toml"), directory.join("src/lib.rs")]
            .iter()
            .any(|path| {
                fs::read_to_string(path)
                    .map(|text| text.contains(GENERATED_MARKER))
                    .unwrap_or(false)
            });
        if !marker {
            return Err(format!(
                "add_crates: {package} at {} does not look like a package this action created (no \
                 `{GENERATED_MARKER}` line in its Cargo.toml or src/lib.rs), so it is left alone.\
                 \nway forward: remove it by hand if it really is a leftover",
                directory.display()
            ));
        }
        fs::remove_dir_all(directory).map_err(|error| {
            format!("add_crates: cannot remove {}: {error}", directory.display())
        })?;
        written.files.push(relative(root, directory));
    }
    Ok(())
}

/// The `members` edit this pass has to make, computed **before** anything is written.
/// 本次写入要做的 `members` 编辑，在任何东西被写下**之前**算出来。
///
/// `None` when there is no enclosing workspace, or when the list already says what this pass needs.
/// The caller computes it first for the same reason the config is merged first: a refusal must leave
/// the tree as it found it, and a member list naming a package that does not exist is exactly the
/// state cargo refuses to load (audit `M7`, P3.5).
/// 没有外层工作区、或清单已经说出本次需要的内容时是 `None`。调用方先算它的理由与先合并配置相同：拒绝必须让
/// 树保持原样，而一份点名了不存在的包的成员清单正是 cargo 拒绝加载的状态（审计 `M7`，P3.5）。
fn members_edit(
    workspace: Option<&Path>,
    directories: &[&Path],
    removing: bool,
) -> Result<Option<(PathBuf, String)>, String> {
    let Some(workspace_root) = workspace else {
        return Ok(None);
    };
    let entries = crate_members::entries(workspace_root, directories);
    let path = workspace_root.join("Cargo.toml");
    let text = fs::read_to_string(&path)
        .map_err(|error| format!("add_crates: cannot read {}: {error}", path.display()))?;
    let merged = if removing {
        crate_members::stripped(&text, &entries)?
    } else {
        crate_members::merged(&text, &entries)?
    };
    Ok(merged.map(|text| (path, text)))
}

#[cfg(test)]
#[path = "crate_write_tests.rs"]
mod crate_write_tests;

/// The host manifest with one dependency line per generated crate that owns a claimed subtree.
/// 宿主清单，为每个拥有被认领子树的生成 crate 加一条依赖。
fn planned_host_dependencies(
    manifest: &Path,
    dependencies: &[(String, String)],
    base: Option<&str>,
) -> Result<Option<(PathBuf, String)>, String> {
    let text = match base {
        Some(text) => text.to_owned(),
        None => fs::read_to_string(manifest)
            .map_err(|error| format!("add_crates: cannot read {}: {error}", manifest.display()))?,
    };
    let mut missing = Vec::new();
    for (package, path) in dependencies {
        if text.lines().any(|line| {
            let trimmed = line.trim_start();
            trimmed.starts_with(&format!("{package} ="))
                || trimmed.starts_with(&format!("{package} = {{"))
        }) {
            continue;
        }
        missing.push(format!("{package} = {{ path = \"{path}\" }}"));
    }
    if missing.is_empty() {
        return Ok(None);
    }
    let mut lines: Vec<String> = text.lines().map(str::to_owned).collect();
    match lines
        .iter()
        .position(|line| line.trim() == "[dependencies]")
    {
        Some(section) => {
            for (offset, line) in missing.iter().enumerate() {
                lines.insert(section + 1 + offset, line.clone());
            }
        }
        None => {
            lines.push(String::new());
            lines.push("[dependencies]".to_owned());
            lines.extend(missing);
        }
    }
    Ok(Some((manifest.to_path_buf(), lines.join("\n") + "\n")))
}

/// Take a partition back: remove the ghost packages this action created, and its config entries.
/// 把一次拆分撤回：删掉本动作创建的幽灵包，以及它加进配置里的那些条目。
///
/// Two lines are held here, and both matter more on the way out than on the way in:
/// 这里守两条线，而两条在**撤出**时比进入时更要紧：
///
/// - **A directory this action did not create is never removed.** The generated `Cargo.toml`/`lib.rs`
///   carry a `Generated by XiRang` line; a directory without one is refused by name, because the
///   alternative is a tool deleting somebody's package that happens to sit where a ghost would.
/// - **配置文件只减去它自己加的那些条目**, and a file that is left with nothing but headers this action
///   added is removed rather than left as an empty shell.
/// - **本动作没有创建的目录永不删除**。生成的 `Cargo.toml`/`lib.rs` 带着 `Generated by XiRang` 一行；
///   没有这一行的目录会被点名拒绝——另一种做法是工具删掉别人的包，只因为它恰好躺在幽灵会在的位置。
/// - **配置文件只减去它自己加的那些条目**，而只剩本动作加进去的表头的文件会被删掉，而不是留一个空壳。
pub(crate) fn revert_partition(
    root: &Path,
    workspace: Option<&Path>,
    planned: &[PlannedCrate],
    facade: Option<&PlannedFacade>,
) -> Result<Written, String> {
    let mut written = Written::default();
    let generation = super::discovery_cache::generation(root);
    let mut remap: Vec<(String, String)> = Vec::new();
    // One list of packages to take back: every ghost this action planned, plus the facade, which is
    // planned as the host's sibling and carries the cross-crate half (audit `M7`, §M7.33).
    // 一份要撤回的包清单：本动作规划的每个幽灵，加上 facade——它被规划成宿主的同级目录、承载跨 crate 那一半
    // （审计 `M7`，§M7.33）。
    let mut directories: Vec<(&Path, &str)> = planned
        .iter()
        .map(|planned| (planned.directory.as_path(), planned.package.as_str()))
        .collect();
    if let Some(facade) = facade {
        directories.push((facade.directory.as_path(), facade.package.as_str()));
    }
    // The member list is stripped **first**, and only in memory: taking the packages out and then
    // refusing the manifest edit would leave a list naming packages that no longer exist, which is
    // exactly the state cargo refuses to load (audit `M7`, P3.5).
    // 成员清单**先**剥、而且只在内存里剥：把包拿掉之后再拒绝清单编辑，会留下一份点名了已不存在的包的清单，
    // 而那正是 cargo 拒绝加载的状态（审计 `M7`，P3.5）。
    let members = members_edit(
        workspace,
        &directories
            .iter()
            .map(|(directory, _)| *directory)
            .collect::<Vec<_>>(),
        true,
    )?;
    clear_generated(root, &directories, &mut written)?;
    for planned in planned {
        remap.extend(planned.remap.iter().cloned());
    }
    remap.sort();
    remap.dedup();
    written.config_changed = strip_workspace_config(root, &remap, generation)?;
    if let Some((path, text)) = members {
        // The workspace manifest is the host's file that this action only edits, so it is not decorated either.
        // 工作区清单是宿主自己的文件、本动作只做编辑，因此它同样不被装饰。
        crate::build_method::discovery_cache::write_verbatim_at(&path, &text)?;
        written.members_changed = true;
    }
    written.files.sort();
    written.files.dedup();
    Ok(written)
}

/// Remove this action's `--remap-path-prefix` entries from the workspace config.
/// 从工作区配置里移掉本动作加的 `--remap-path-prefix` 条目。
///
/// Returns whether the file changed. The same shapes the forward merge handles are handled here, and
/// the same unrecognizable shape is refused — with the entries to delete by hand.
/// 返回文件是否被改动。正向合并处理的那几种形状在这里同样处理，同样无法辨认的形状同样被拒绝——并给出该手动
/// 删掉的那些条目。
fn strip_workspace_config(
    workspace_root: &Path,
    remap: &[(String, String)],
    generation: u64,
) -> Result<bool, String> {
    let path = workspace_root.join(".cargo/config.toml");
    let Ok(text) = fs::read_to_string(&path) else {
        return Ok(false);
    };
    let prefixes: Vec<String> = remap
        .iter()
        .map(|(from, to)| format!("--remap-path-prefix={from}={to}"))
        .collect();
    if !prefixes.iter().any(|prefix| text.contains(prefix.as_str())) {
        return Ok(false);
    }
    let stripped = match strip_rustflags(&text, &prefixes) {
        Some(stripped) => stripped,
        None => {
            return Err(format!(
                "add_crates: {} has a `rustflags` this action will not rewrite (it is not a single-line \
                 array). Delete these entries by hand:\n{}",
                path.display(),
                prefixes
                    .iter()
                    .map(|prefix| format!("    \"{prefix}\"", prefix = prefix))
                    .collect::<Vec<_>>()
                    .join("\n")
            ));
        }
    };
    if nothing_but_our_shell(&stripped) {
        return fs::remove_file(&path)
            .map(|()| true)
            .map_err(|error| format!("cannot remove {}: {error}", path.display()));
    }
    write_if_changed_at(&path, &stripped, generation).map(|()| true)
}

/// Take our entries out of a single-line `rustflags` array, or `None` for a shape this action refuses.
/// 把我们的条目从单行 `rustflags` 数组里拿掉；形状无法处理时返回 `None`。
fn strip_rustflags(text: &str, prefixes: &[String]) -> Option<String> {
    let mut lines: Vec<String> = text.lines().map(str::to_owned).collect();
    let mut in_build = false;
    for line in &mut lines {
        let trimmed = line.trim_start().to_owned();
        if trimmed.starts_with('[') {
            in_build = trimmed == "[build]";
            continue;
        }
        if !in_build || !trimmed.starts_with("rustflags") {
            continue;
        }
        let (_, value) = trimmed.split_once('=')?;
        let value = value.trim();
        if !value.starts_with('[') || !value.ends_with(']') {
            return None;
        }
        let kept: Vec<String> = value[1..value.len() - 1]
            .split(',')
            .map(str::trim)
            .filter(|item| !item.is_empty())
            .filter(|item| {
                let bare = item.trim_matches('"');
                !prefixes.iter().any(|prefix| bare == prefix)
            })
            .map(str::to_owned)
            .collect();
        let indent = line.len() - line.trim_start().len();
        if kept.is_empty() {
            *line = String::new();
        } else {
            *line = format!("{}rustflags = [{}]", " ".repeat(indent), kept.join(", "));
        }
        return Some(lines.join("\n") + "\n");
    }
    Some(text.to_owned())
}

/// Whether a config is now only the shell this action would have added, and can therefore go.
/// 配置现在是否只剩本动作会加的那层壳，因而可以被删掉。
fn nothing_but_our_shell(text: &str) -> bool {
    text.lines().all(|line| {
        let trimmed = line.trim();
        trimmed.is_empty() || trimmed.starts_with('#') || trimmed == "[build]"
    })
}

/// Create the file's directory and write it if the bytes differ.
/// 创建文件的目录，并在字节不同时写入。
fn write_ghost_file(path: &Path, content: &str, generation: u64) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("cannot create {}: {error}", parent.display()))?;
    }
    write_if_changed_at(path, content, generation)
}
/// Insert entries into a single-line `rustflags` array, or `None` for a shape this action refuses.
/// 把条目插进单行的 `rustflags` 数组；形状无法处理时返回 `None`。
fn merge_rustflags(text: &str, missing: &[&String]) -> Option<String> {
    let mut lines: Vec<String> = text.lines().map(str::to_owned).collect();
    let mut in_build = false;
    for line in &mut lines {
        let trimmed = line.trim_start().to_owned();
        if trimmed.starts_with('[') {
            in_build = trimmed == "[build]";
            if in_build {
                continue;
            }
        }
        if !in_build || !trimmed.starts_with("rustflags") {
            continue;
        }
        let (_, value) = trimmed.split_once('=')?;
        let value = value.trim();
        if !value.starts_with('[') || !value.ends_with(']') {
            return None;
        }
        let inner = &value[1..value.len() - 1];
        let existing: Vec<&str> = inner
            .split(',')
            .map(str::trim)
            .filter(|item| !item.is_empty())
            .collect();
        let mut combined: Vec<String> = existing.iter().map(|item| (*item).to_owned()).collect();
        combined.extend(missing.iter().map(|entry| (*entry).clone()));
        let indent = line.len() - line.trim_start().len();
        *line = format!(
            "{}rustflags = [{}]",
            " ".repeat(indent),
            combined.join(", ")
        );
        return Some(lines.join("\n") + "\n");
    }
    // No `[build]` section at all: append one.
    let mut merged = text.trim_end().to_owned();
    merged.push_str("\n\n[build]\nrustflags = [");
    merged.push_str(
        &missing
            .iter()
            .map(|entry| (*entry).clone())
            .collect::<Vec<_>>()
            .join(", "),
    );
    merged.push_str("]\n");
    Some(merged)
}
