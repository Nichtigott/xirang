//! `xirang crates`: plan the crate split a host declared, and optionally write it.
//! `xirang crates`：规划宿主声明的 crate 拆分，并可选地把它写下来。
//!
//! The declaration (`add_crates.rs`) is an authoring input, so this is the authoring action's CLI
//! face: `--check` (the default) prints what would be created and refuses a shape that cannot work,
//! `--write` creates the ghost packages and merges the workspace-root config they need. Splitting is a
//! thing you commit, so the write half is **idempotent**: a second run rewrites nothing.
//! 声明（`add_crates.rs`）是创作输入，因此这是创作动作的 CLI 面：`--check`（默认）打印会创建什么、并对
//! 无法成立的形状点名拒绝；`--write` 创建幽灵包并合并它们所需的工作区根配置。拆分是要提交的东西，因此写入
//! 那一半是**幂等**的：第二遍什么都不重写。

use std::io::Write;
use std::path::{Path, PathBuf};

use super::resolve_package;

/// Write one line, turning an I/O failure into the string this command returns.
/// 写一行，把 I/O 失败转成本命令返回的字符串。
fn line(out: &mut dyn Write, text: String) -> Result<(), String> {
    writeln!(out, "{text}").map_err(|error| error.to_string())
}

/// Plan (and with `--write`, create) the crates the host's declaration names.
/// 规划（并在 `--write` 时创建）宿主声明点名的那些 crate。
pub(crate) fn crates(
    args: &mut impl Iterator<Item = String>,
    out: &mut dyn Write,
) -> Result<(), String> {
    let mut directory: Option<PathBuf> = None;
    let mut write = false;
    let mut revert = false;
    let mut release = false;
    // The declaration layer, which is handled before the declaration is read: creating the first one
    // is exactly the case where reading it would refuse.
    // 声明层，在读取声明之前处理：创建第一份声明正是"读取它会拒绝"的那种情形。
    let mut declaration: Option<DeclarationAction> = None;
    let mut declared_subtrees: Vec<String> = Vec::new();
    // `while let` rather than `for … in args.by_ref()`: the declaration flags take a value from the
    // same iterator, which a `for` loop has already borrowed.
    // 用 `while let` 而不是 `for … in args.by_ref()`：声明类旗标要从同一个迭代器取一个值，而 `for` 循环已经
    // 借走了它。
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-h" | "--help" => return crate::cli::usage(out),
            "--check" => {
                write = false;
                revert = false;
            }
            "--write" => write = true,
            "--revert" => revert = true,
            // The development shape mounts a fragment's files out of the host package; the release
            // shape copies them in, because crates.io rejects a package whose `#[path]` reaches
            // outside it (audit `M7`, §M7.42). The two shapes land in the same directories, so one is
            // reverted before the other is written.
            // 开发形状把碎片的文件从宿主包里挂载出来；发布形状把它们复制进去，因为 crates.io 拒绝 `#[path]`
            // 伸到包外的包（审计 `M7`，§M7.42）。两个形状落在同一批目录里，因此写一个之前要先撤回另一个。
            "--release" => release = true,
            "--declare" => {
                let name = args
                    .next()
                    .ok_or_else(|| "--declare needs the crate's name".to_owned())?;
                declaration = Some(DeclarationAction::Declare {
                    name,
                    subtrees: Vec::new(),
                });
            }
            // One subtree per flag, and a `declare` claims all of them: a crate is a subtree, and a
            // crate that claims two is two entries in one `Crate`.
            // 每个旗标一棵子树，而一次 `declare` 认领它们全部：crate 是一棵子树，认领两棵的 crate 就是同一个
            // `Crate` 里的两条。
            "--subtree" => {
                let subtree = args.next().ok_or_else(|| {
                    "--subtree needs the host's path to a SUBTREE constant".to_owned()
                })?;
                declared_subtrees.push(subtree);
            }
            "--undeclare" => {
                let name = args
                    .next()
                    .ok_or_else(|| "--undeclare needs the crate's name".to_owned())?;
                declaration = Some(DeclarationAction::Undeclare { name });
            }
            _ if arg.starts_with('-') => return Err(format!("unexpected argument '{arg}'")),
            _ if directory.is_none() => directory = Some(PathBuf::from(arg)),
            _ => return Err("crates accepts at most one project path".to_owned()),
        }
    }
    let directory = match directory {
        Some(directory) => directory,
        None => std::env::current_dir()
            .map_err(|error| format!("cannot read the current directory: {error}"))?,
    };
    let (package_root, package) = resolve_package(&directory.to_string_lossy())?;
    if let Some(action) = declaration {
        return declaration_edit(&package_root, action, declared_subtrees, write, out);
    }
    let declaration = crate::build_method::read_shape_declaration(&package_root)?.ok_or_else(|| {
        format!(
            "{} has no {}: a host without a declaration is one crate, so there is nothing to plan",
            package_root.display(),
            xirang_kernel::lexicon::ADD_CRATES_FILE
        )
    })
            .map_err(|error| error.to_string())?;
    let out_dir = super::build_out_dir(&package_root);
    let rows = crate::build_method::read_pruning_manifest(&out_dir).map_err(|error| {
        format!("{error}\nway forward: run `xirang check` first — the plan reads the faces the build published")
    })
            .map_err(|error| error.to_string())?;
    let faces: Vec<(String, String, crate::build_method::NodeId)> = rows
        .iter()
        .map(|row| {
            (
                row.source.clone(),
                crate::build_method::source_module_path(&row.source),
                row.id,
            )
        })
        .collect();
    // The preview plans the same bytes the writer writes, so it needs the same input the writer uses:
    // the host entry's declared cuts, which travel with every generated package (audit `M7`, §M7.55).
    // 预览规划的是写入方写下的同一份字节，因此需要写入方所用的同一个输入：宿主入口声明的切口，它随每个
    // 生成包一起走（审计 `M7`，§M7.55）。
    let cuts = crate::build_method::declared_host_cuts(&package_root)?;
    let planned =
        crate::build_method::plan_crates(&package_root, &package, &declaration, &faces, &cuts)?;
    let facade = crate::build_method::plan_facade(
        &package_root,
        &declaration.package_prefix,
        &package,
        &package,
        &planned,
        &cuts,
    )?;
    if planned.is_empty() {
        return line(out, format!("nothing declared: {package} names no crate"));
    }
    // The release packages are planned **before** they are described. A refusal belongs to the plan, and
    // it used to arrive after the preview had already listed a facade the writer would never create:
    // measured on a binary host, the CLI printed all three release packages and then exited non-zero.
    // The `--check` path needs this too — it prints the plan a `--write` would follow, so it has to
    // refuse what that write would refuse.
    // 发布包**先规划、后描述**。拒绝属于规划，而它过去在预览已经列出了写入方永远不会创建的 facade 之后才
    // 到来：在一个二进制宿主上实测，CLI 打印了全部三个发布包、然后以非零退出。`--check` 路径同样需要它
    // ——它打印的是 `--write` 将要遵循的计划，因此必须拒绝那次写入会拒绝的东西。
    let release_packages = if release {
        let mut packages = Vec::new();
        for planned in &planned {
            packages.push(crate::build_method::plan_release_ghost(
                &package_root,
                planned,
                &faces,
                &cuts,
            )?);
        }
        packages.push(crate::build_method::plan_release_facade(
            &package_root,
            &package,
            &declaration.package_prefix,
            &package,
            &planned,
            &cuts,
        )?);
        Some(packages)
    } else {
        None
    };
    if release {
        for planned in &planned {
            line(
                out,
                format!(
                    "crate {} at {} (release shape, {} copied file(s))",
                    planned.package,
                    planned.directory.display(),
                    planned.mounts.len() + 1
                ),
            )?;
        }
        line(
            out,
            format!(
                "facade {}-facade at {} (release shape, resolves the host through cargo at build time)",
                declaration.package_prefix,
                // The same rule the packager uses, asked rather than re-derived: this line used to
                // spell the path itself and printed the facade one level above the directory it was
                // actually written to (`…/partitioned-button-facade` instead of `…/crates/…`), which
                // is exactly the kind of self-description that sends a reader hunting a phantom bug.
                // 与打包方同一条规则，**问**它而不是自己再拼一遍：这一行原先自己拼路径，于是打印的 facade
                // 比实际写下处高了一层（`…/partitioned-button-facade` 而不是 `…/crates/…`），正是那种会把
                // 读者送去追一个并不存在的缺陷的自我描述。
                crate::build_method::crate_plan::crates_dir(
                    package_root.parent().unwrap_or(&package_root)
                )
                .join(format!("{}-facade", declaration.package_prefix))
                .display(),
            ),
        )?;
    } else {
        for planned in &planned {
            line(
                out,
                format!(
                    "crate {} at {} ({} mounted file(s), {} remap entr(ies))",
                    planned.package,
                    planned.directory.display(),
                    planned.mounts.len(),
                    planned.remap.len()
                ),
            )?;
        }
        if let Some(facade) = &facade {
            line(
                out,
                format!(
                    "facade {} at {} (dependencies: {})",
                    facade.package,
                    facade.directory.display(),
                    facade.dependencies.join(", ")
                ),
            )?;
        }
    }
    let (root, workspace) = crate::build_method::partition_roots(&package_root);
    if revert {
        let reverted = crate::build_method::revert_partition(
            &root,
            workspace.as_deref(),
            &planned,
            facade.as_ref(),
        )?;
        return line(
            out,
            format!(
                "removed {} generated package(s) under {}; workspace config {}{}",
                reverted.files.len(),
                root.display(),
                if reverted.config_changed {
                    "updated"
                } else {
                    "carried none of this action's entries"
                },
                members_reply(reverted.members_changed, workspace.is_some(), &root)
            ),
        );
    }
    // `--check` prints the plan a `--write` would follow, so it has to refuse what that write would
    // refuse. The shape already on disk is the one thing a reader of the plan cannot see, and printing
    // a development plan over a committed release shape (or the reverse) is how a hand-run write lands
    // on the wrong shape — the two shapes share their directories, so the difference is invisible in
    // the plan itself. The guarded set is exactly the one the matching `--write` guards: `planned` in
    // both shapes, plus the facade package in the release shape (the release write guards it; the
    // development write does not).
    // `--check` 打印的是 `--write` 将要遵循的那份计划，因此它必须拒绝那次写入会拒绝的东西。磁盘上已有的
    // 形状正是计划读者看不见的那一件事，而在入库的发布形状上打印一份开发计划（或反过来）正是手跑写入落错
    // 形状的成因——两个形状共用同一批目录，因此差别在计划本身里看不出来。受检集合与对应 `--write` 所检的
    // 完全一致：两个形状都是 `planned`，发布形状另加 facade 包（发布写入会检它，开发写入不检）。
    let mut guarded: Vec<&Path> = planned
        .iter()
        .map(|planned| planned.directory.as_path())
        .collect();
    if release && let Some(facade) = &facade {
        guarded.push(facade.directory.as_path());
    }
    crate::build_method::guard_shape(&guarded, release)?;
    if !write {
        line(
            out,
            if release {
                "preview only: pass --write to create these packages in the release shape"
                    .to_owned()
            } else {
                "preview only: pass --write to create these packages and merge the workspace config"
                    .to_owned()
            },
        )?;
        return Ok(());
    }
    if release {
        let packages = release_packages.expect("a release run planned its packages above");
        crate::build_method::guard_shape(
            &packages
                .iter()
                .map(|package| package.directory.as_path())
                .collect::<Vec<_>>(),
            true,
        )?;
        let written = crate::build_method::write_release(
            &root,
            workspace.as_deref(),
            &packages,
            &package_root,
        )?;
        return line(
            out,
            format!(
                "wrote {} file(s) under {}; the release shape carries its own sources, so there is \
                 nothing to remap{}",
                written.files.len(),
                root.display(),
                members_reply(written.members_changed, workspace.is_some(), &root)
            ),
        );
    }
    crate::build_method::guard_shape(
        &planned
            .iter()
            .map(|planned| planned.directory.as_path())
            .collect::<Vec<_>>(),
        false,
    )?;
    // A host dependency is only an **edge the host's code needs**: a host face outside the claimed
    // subtree that still spells `crate::<subtree>::…`. A partition that no host face refers to must
    // not touch the host manifest at all (the rehearsal gate hashes it byte for byte).
    // 宿主依赖只是**宿主代码需要的边**：被认领子树之外、仍写 `crate::<子树>::…` 的宿主面。没有任何宿主面
    // 引用的分区不许改动宿主清单（rehearsal 门禁会逐字节比较它）。
    let host_dependencies: Vec<(String, String)> = planned
        .iter()
        .filter(|planned| host_needs_fragment(&package_root, planned))
        .map(|planned| {
            (
                planned.package.clone(),
                relative_from(&package_root, &planned.directory),
            )
        })
        .collect();
    let written = crate::build_method::write_partition_with_host(
        &root,
        workspace.as_deref(),
        &planned,
        facade.as_ref(),
        Some(&package_root.join("Cargo.toml")),
        &host_dependencies,
    )?;
    line(
        out,
        format!(
            "wrote {} file(s) under {}; workspace config {}{}",
            written.files.len(),
            root.display(),
            if written.config_changed {
                "updated"
            } else {
                "already carried the remap"
            },
            members_reply(written.members_changed, workspace.is_some(), &root)
        ),
    )?;
    Ok(())
}

/// The member-list half of a write or revert reply.
/// 写入或撤回回复里属于成员清单的那一半。
pub(crate) fn members_reply(changed: bool, in_a_workspace: bool, root: &std::path::Path) -> String {
    if !in_a_workspace {
        // The split wrote packages that **belong to no workspace**, and the reason is structural rather than
        // a missing feature: the host declares a `[workspace]` of its own, the packages land beside it, and
        // cargo refuses a member that is not hierarchically below its root — measured,
        // `error: workspace member `…/crates/dash-dash-board/Cargo.toml` is not hierarchically below the
        // workspace root `…/dash/Cargo.toml``. Saying only "no member list" left the author with a split
        // whose `cargo build --workspace` succeeds **without touching any generated package** (audit `M7`,
        // §M7.64), so the sentence now carries both ways out.
        // 这次拆分写下了**不属于任何工作区**的包，而原因是结构性的、不是缺功能：宿主自己声明了
        // `[workspace]`，包落在它旁边，而 cargo 拒绝一个不在其根之下的成员——实测，`error: workspace member
        // `…/crates/dash-dash-board/Cargo.toml` is not hierarchically below the workspace root
        // `…/dash/Cargo.toml``。只说一句"没有成员清单"，会让作者拿到一次**成功却不碰任何生成包**的
        // `cargo build --workspace`（审计 `M7`，§M7.64），因此这句话现在把两条出路都带上。
        return format!(
            "; no workspace can take these packages: this host declares a `[workspace]` of its own, and \
             cargo refuses a member that is not hierarchically below its root — a root above it would \
             have to be a workspace too, and cargo refuses that as well (`multiple workspace roots found \
             in the same workspace`, measured when a workspace beside the host listed both). Build each \
             of them with `cargo build --manifest-path <package>/Cargo.toml` under {}",
            root.display()
        );
    }
    if changed {
        "; workspace members updated".to_owned()
    } else {
        "; workspace members already listed".to_owned()
    }
}

/// Which declaration change the command line asked for.
/// 命令行要求的是哪一种声明改动。
enum DeclarationAction {
    /// Add a crate, claiming every `--subtree` given beside it.
    /// 加一个 crate，认领它旁边给出的每一个 `--subtree`。
    Declare { name: String, subtrees: Vec<String> },
    /// Remove a crate's entry.
    /// 移除某个 crate 的条目。
    Undeclare { name: String },
}

/// Edit the host's declaration, previewing unless `--write` was given.
/// 编辑宿主的声明；除非给了 `--write`，否则只预览。
///
/// The same discipline as the write path: a declaration change is text the author also reads, so the
/// preview is the exact diff, and the change is rolled back when the resulting declaration turns out
/// not to plan.
/// 与写入路径同一套纪律：声明改动是作者也会读的文本，因此预览就是那段确切的差异，而改动在"改完之后声明规划
/// 不成立"时会被回滚。
fn declaration_edit(
    package_root: &Path,
    action: DeclarationAction,
    mut subtrees: Vec<String>,
    write: bool,
    out: &mut dyn Write,
) -> Result<(), String> {
    if let DeclarationAction::Declare {
        subtrees: inline, ..
    } = &action
    {
        subtrees.extend(inline.iter().cloned());
    }
    let edit = match action {
        DeclarationAction::Declare { name, .. } => {
            if subtrees.is_empty() {
                return Err(format!(
                    "`{name}` claims no subtree: pass `--subtree crate::…::SUBTREE` — a crate is a \
                     subtree, not a name on its own"
                ));
            }
            crate::build_method::declare(package_root, &name, &subtrees)?
        }
        DeclarationAction::Undeclare { name } => {
            if let Some(directory) = crate::build_method::package_directory_of(package_root, &name)?
            {
                return Err(format!(
                    "`{name}` still has its package at {}: `--revert` first (it takes the generated \
                     packages back while the declaration still names them), then undeclare",
                    directory.display()
                ));
            }
            crate::build_method::undeclare(package_root, &name)?
        }
    };
    let diff = edit.diff();
    if !write {
        line(out, diff.clone())?;
        return line(
            out,
            format!(
                "preview: {} is untouched; pass --write to change it",
                edit.path.display()
            ),
        );
    }
    let after = crate::build_method::apply_declaration_edit(package_root, edit)?;
    line(out, diff)?;
    match after {
        Some(view) => line(
            out,
            format!(
                "wrote {}: {} crate(s) declared",
                package_root
                    .join(xirang_kernel::lexicon::ADD_CRATES_FILE)
                    .display(),
                view.packages
                    .iter()
                    .filter(|package| package.crate_name.is_some())
                    .count()
            ),
        ),
        None => line(
            out,
            "wrote the removal: this host declares no crates again".to_owned(),
        ),
    }
}

/// Whether a host face outside `planned`'s subtrees still spells `crate::<subtree>::…`.
/// 宿主在 `planned` 的子树之外，是否仍写 `crate::<子树>::…`。
///
/// The read is textual and conservative: any `.rs` the host compiles counts (including tests), while
/// files under the claimed subtree are skipped because the fragment compiles those. A false positive
/// merely adds an unused dependency edge; a false negative would leave host code unable to resolve.
/// 读取是文本的、偏保守的：宿主编译的每个 `.rs` 都算（含测试），而被认领子树下的文件跳过，因为那些由
/// 碎片编译。误报只多一条未用依赖边；漏报会让宿主代码解析不到。
fn host_needs_fragment(
    package_root: &Path,
    planned: &super::super::build_method::crate_plan::PlannedCrate,
) -> bool {
    let mut files = Vec::new();
    visit_rust_files(&package_root.join("src"), &mut files);
    visit_rust_files(&package_root.join("tests"), &mut files);
    for subtree in &planned.subtrees {
        let needle = format!("crate::{subtree}");
        let claimed = format!("/{}/", subtree.replace("::", "/"));
        for file in &files {
            let relative = file
                .strip_prefix(package_root)
                .unwrap_or(file)
                .to_string_lossy()
                .replace('\\', "/");
            if relative.contains(&claimed) {
                continue;
            }
            // The host **entry** keeps the author's `static_graft_plan!` text, including cuts for
            // faces it just handed away; that text is data for the generator, not a type the host
            // compiles. Only faces outside the entry create a dependency need.
            // 宿主**入口**保留作者的 `static_graft_plan!` 文本，其中包括刚交出去的面的切口；那是给生成器的
            // 数据，不是宿主编译的类型。只有入口之外的面才会产生依赖需求。
            if file
                .file_name()
                .is_some_and(|name| name == "lib.rs" || name == "main.rs")
            {
                continue;
            }
            if std::fs::read_to_string(file).is_ok_and(|text| text.contains(&needle)) {
                return true;
            }
        }
    }
    false
}

/// Every `.rs` below `directory`, skipping build outputs and generated packages.
/// `directory` 下每个 `.rs`，跳过构建产物与生成包。
fn visit_rust_files(directory: &Path, files: &mut Vec<std::path::PathBuf>) {
    let Ok(entries) = std::fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name();
        if name == "target" || name == "crates" || name == ".xirang" {
            continue;
        }
        if path.is_dir() {
            visit_rust_files(&path, files);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            files.push(path);
        }
    }
}

/// `to` relative to `from`, with `/` separators, for a manifest `path = "…"`.
/// `to` 相对 `from` 的路径，使用 `/` 分隔符，供清单的 `path = "…"` 使用。
fn relative_from(from: &Path, to: &Path) -> String {
    let from: Vec<_> = from.components().collect();
    let to: Vec<_> = to.components().collect();
    let common = from
        .iter()
        .zip(&to)
        .take_while(|(left, right)| left == right)
        .count();
    let mut parts: Vec<String> =
        std::iter::repeat_n("..".to_owned(), from.len() - common).collect();
    parts.extend(
        to[common..]
            .iter()
            .map(|component| component.as_os_str().to_string_lossy().into_owned()),
    );
    parts.join("/")
}
