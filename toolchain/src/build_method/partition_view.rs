//! What a host's declared crate partition is, and what is on disk for it (audit `M7`, P3.6).
//! 宿主的 crate 分区是什么样，以及它在磁盘上已经有什么（审计 `M7`，P3.6）。
//!
//! This is the one reader both authoring surfaces use: `xirang crates` prints it, and Studio's
//! partition screen draws it, so the two cannot disagree about what a declaration means, what is
//! already written, or whether a package could be published.
//! 这是两个创作面共用的唯一读取器：`xirang crates` 打印它，Studio 的分区屏绘制它，因此两者不可能对
//! "声明是什么意思、已经写了什么、某个包能不能发布"给出不同答案。
//!
//! It reads the **published** records (`target/xirang/out`), not a fresh derivation, for the same
//! reason the authoring write path does: the plan is what the build published, and a screen that
//! derived its own would describe a tree the writer is not about to write.
//! 它读的是**已发布的记录**（`target/xirang/out`），而不是现推导一遍，理由与创作写入路径相同：计划就是
// 构建发布出来的东西，而一个自己推导的画面会描述一棵写入方并不打算写的树。

use std::fs;
use std::path::{Path, PathBuf};

use xirang_kernel::identity::NodeId;

use super::HostCut;
use super::crate_facade::{PlannedFacade, plan_facade};
use super::crate_plan::partition_roots;
use super::crate_plan::{GENERATED_MARKER, PlannedCrate, plan as plan_crates};
use super::crate_release::plan_ghost as plan_release_ghost;
use super::crate_release::{ReleasePackage, plan_facade as plan_release_facade};
use super::shape_decl::read_shape_declaration;

/// The directory the build publishes its records into, relative to a package root.
/// 构建把记录发布进去的目录，相对包根。
///
/// One implementation for the CLI, the bridge and Studio: three copies of this path is three chances
/// for one of them to read a directory the build never wrote.
/// CLI、桥与 Studio 共用一份实现：这条路径有三份拷贝，就是三次"其中一个读到的目录构建从没写过"的机会。
pub(crate) fn build_out_dir(package_root: &Path) -> PathBuf {
    package_root.join("target/xirang/out")
}

/// The graft cuts the host entry declares, for a generated package to carry.
/// 宿主入口声明的 graft 切口，供生成包携带。
///
/// The **declared** list, not the enabled one: `#[cfg]` is evaluated where the declaration is compiled,
/// and that is the generated package's own build (it copies the host's feature table). Handing over a
/// list already filtered here would bake this machine's feature set into a package built elsewhere.
/// 取的是**声明的**那份清单，而不是已启用的：`#[cfg]` 在声明被编译的地方求值，而那正是生成包自己的构建
/// （它复制了宿主的特性表）。在这里就过滤好，等于把本机的特性组合烤进一个在别处构建的包。
///
/// Reading the entry rather than the published records is deliberate: `graft_plan.tsv` is what one
/// **package** emits, already attributed, so it cannot say what the host declared.
/// 读入口而不是读已发布的记录是有意的：`graft_plan.tsv` 是某个**包**发射的东西、已经过归属，因此它说不出
/// 宿主声明了什么。
pub(crate) fn declared_host_cuts(root: &Path) -> Result<Vec<HostCut>, String> {
    let entry = super::entry::host_entry_source(root).map_err(|error| {
        format!(
            "{error}\nway forward: a partition hands the host entry's graft plan to every package it \
             generates, so the entry has to be where the build looks for it"
        )
    })?;
    let source =
        fs::read_to_string(&entry).map_err(|error| format!("read {}: {error}", entry.display()))?;
    let declarations = super::registry_syntax::graft_entries(&source)
        .map_err(|error| format!("{}: {}", entry.display(), error.message))?;
    Ok(declarations
        .into_iter()
        .map(|declaration| HostCut {
            cut: declaration.cut,
            cut_end: declaration.cut_end,
            graft: declaration.graft,
            full: declaration.full,
            typed: declaration.expressions.is_some(),
            cfg: declaration.cfg,
            line: declaration.location.line,
            column: declaration.location.column,
        })
        .collect())
}

/// 某个宿主所声明各 crate 的计划：写入方要写的东西，也是画面要描述的东西。
pub(crate) struct PartitionPlan {
    /// The declared package prefix.
    /// 声明的包前缀。
    pub(crate) package_prefix: String,
    /// Faces the host's own tree has.
    /// 宿主自己那棵树有多少个注册面。
    pub(crate) host_faces: usize,
    /// The development shape's packages.
    /// 开发形状的包。
    pub(crate) planned: Vec<PlannedCrate>,
    /// The development shape's facade.
    /// 开发形状的 facade。
    pub(crate) facade: Option<PlannedFacade>,
    /// The release shape's packages, ghost(s) first and the facade last.
    /// 发布形状的包，幽灵在前、facade 在后。
    pub(crate) release: Vec<ReleasePackage>,
    /// Where the workspace config and the member list belong (see [`partition_roots`]).
    /// 工作区配置与成员清单该落在哪里（见 [`partition_roots`]）。
    pub(crate) config_root: PathBuf,
    /// The enclosing workspace, when there is one.
    /// 外层工作区（若存在）。
    pub(crate) workspace: Option<PathBuf>,
    /// The host package this partition was planned from.
    /// 本次划分据以规划的宿主包。
    ///
    /// Carried because the writers need it and cannot derive it: a release facade's dependency is spelled
    /// relative to **it**, and the check that refuses a host with no library target is a question about that
    /// package rather than about the partition root (audit `M7`, §M7.55).
    /// 带上它是因为写入方需要它、又推导不出来：发布 facade 的依赖是相对**它**拼写的，而"宿主没有库目标"那句
    /// 拒绝问的是那个包、不是分区根（审计 `M7`，§M7.55）。
    pub(crate) host_root: PathBuf,
}

/// Plan the partition a host declares, or `None` when it declares nothing.
/// 规划宿主声明的拆分；宿主没有声明时返回 `None`。
pub(crate) fn plan(package_root: &Path) -> Result<Option<PartitionPlan>, String> {
    let Some(declaration) = read_shape_declaration(package_root)? else {
        return Ok(None);
    };
    let out_dir = build_out_dir(package_root);
    let rows = super::read_pruning_manifest(&out_dir).map_err(|error| {
        format!(
            "{error}\nway forward: run `xirang check` first — the plan reads the faces the build published"
        )
    })?;
    let faces: Vec<(String, String, NodeId)> = rows
        .iter()
        .map(|row| {
            (
                row.source.clone(),
                super::static_plan::source_module_path(&row.source),
                row.id,
            )
        })
        .collect();
    let host_faces = faces.len();
    // The namespace every generated package must define is the **host package's name**, the same
    // value the CLI passes: it is the identity input, so a view that guessed it would describe
    // packages whose faces carry ids nobody baked.
    // 每个生成包必须定义的命名空间是**宿主包名**，与 CLI 传的是同一个值：它是身份输入，因此视图若自己猜一个，
    // 描述出来的包其面就带着没人烤过的 id。
    // A package cargo cannot name is a refusal, not a fallback: the name **is** the identity
    // namespace, so a guessed one would describe a partition whose faces carry ids nobody baked.
    // cargo 说不出名字的包是一句拒绝，而不是兜底：那个名字**就是**身份命名空间，猜一个会描述出一个其面带着
    // 没人烤过的 id 的拆分。
    let host_package =
        super::package::package_name(&package_root.join("Cargo.toml")).map_err(|error| {
            format!(
                "{error}\nway forward: the partition's namespace is the host package's name, and \
                 cargo has to be able to read it"
            )
        })?;
    // The host's graft plan travels with every generated package: the release shape has no host tree to
    // read, so a package that only re-read its own `src/` emitted an empty cut table and the graft the
    // plan promised never happened in the published crate (audit `M7`, §M7.55).
    // 宿主的 graft 计划随每个生成包一起走：发布形状没有宿主树可读，因此只回读自己 `src/` 的包会发射空切口
    // 表，计划承诺的嫁接在发布出去的 crate 里从未发生（审计 `M7`，§M7.55）。
    let cuts = declared_host_cuts(package_root)?;
    let planned = plan_crates(package_root, &host_package, &declaration, &faces, &cuts)?;
    let facade = plan_facade(
        package_root,
        &declaration.package_prefix,
        &host_package,
        &host_package,
        &planned,
        &cuts,
    )?;
    let mut release = Vec::new();
    for planned in &planned {
        release.push(plan_release_ghost(package_root, planned, &faces, &cuts)?);
    }
    if !planned.is_empty() {
        release.push(plan_release_facade(
            package_root,
            &host_package,
            &declaration.package_prefix,
            &host_package,
            &planned,
            &cuts,
        )?);
    }
    let (config_root, workspace) = partition_roots(package_root);
    Ok(Some(PartitionPlan {
        package_prefix: declaration.package_prefix.clone(),
        host_faces,
        planned,
        facade,
        release,
        config_root,
        workspace,
        host_root: package_root.to_path_buf(),
    }))
}

/// Where a declared crate's generated package is, when that directory exists.
/// 某个已声明 crate 的生成包在哪里——当那个目录确实存在时。
///
/// The path comes from the plan, not from string arithmetic: a generated package sits **beside** the
/// host (the ghost is the host's sibling), and a second implementation of that rule is a second
/// chance to look in the wrong place — measured, it was.
/// 这条路径取自计划，而不是字符串运算：生成的包坐在宿主**旁边**（幽灵是宿主的同级），而写第二份那条规则就是
/// 第二次看错地方的机会——实测确实看错了。
pub(crate) fn package_directory_of(root: &Path, name: &str) -> Result<Option<PathBuf>, String> {
    let view = view(root)?.ok_or_else(|| format!("{} declares no crates yet", root.display()))?;
    Ok(view
        .packages
        .iter()
        .find(|package| package.crate_name.as_deref() == Some(name))
        .map(|package| package.directory.clone())
        .filter(|directory| directory.exists()))
}

/// Apply one declaration edit, and refuse to leave behind a declaration the plan cannot read.
/// 落盘一次声明改动，并拒绝留下一份"计划读不了"的声明。
///
/// The edit is rolled back rather than left behind: the declaration is what every tool here reads, so
/// a version of it they refuse describes a shape nobody can act on, and the author would have to work
/// out which line to undo.
/// 改动会被回滚而不是留下：声明是本处每个工具读取的东西，因此一份它们拒绝的声明描述的是没人能据此行动的形态，
/// 而作者还得自己找出该撤销哪一行。
pub(crate) fn apply_declaration_edit(
    root: &Path,
    edit: super::shape_decl::DeclarationEdit,
) -> Result<Option<PartitionView>, String> {
    edit.apply()?;
    match view(root) {
        Ok(after) => Ok(after),
        Err(refusal) => {
            edit.restore()?;
            Err(format!(
                "{refusal}\n(the declaration was put back as it was)\n（声明已恢复原样）\n"
            ))
        }
    }
}

/// What one generated package looks like on disk right now.
/// 某个生成包此刻在磁盘上的样子。
///
/// The two generated shapes are different code paths with different publishing consequences, so a
/// reader is told which one is there rather than left to infer it from the files.
/// 两种生成形状是两条不同的代码路径、发布后果也不同，因此读者被告知磁盘上是哪一种，而不是自己去推断。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OnDisk {
    /// Nothing is there.
    /// 什么都没有。
    Absent,
    /// The development shape: it mounts the host's files and needs the workspace remap.
    /// 开发形状：挂载宿主的文件，依赖工作区 remap。
    Development,
    /// The release shape: it carries the sources its build reads.
    /// 发布形状：携带自己构建要读的源码。
    Release,
    /// A directory that is there and is not this action's.
    /// 在那里、但不是本动作产物的目录。
    Foreign,
}

/// What one row of the partition screen says.
/// 分区屏一行要说的话。
#[derive(Clone, Debug)]
pub struct PackageView {
    /// The package name.
    /// 包名。
    pub(crate) package: String,
    /// The declared crate it serves, or `None` for the facade.
    /// 它服务哪个声明的 crate；facade 为 `None`。
    pub(crate) crate_name: Option<String>,
    /// The subtrees it claims (empty for the facade).
    /// 它认领的子树（facade 为空）。
    pub(crate) subtrees: Vec<String>,
    /// Where it goes.
    /// 它落在哪里。
    pub(crate) directory: PathBuf,
    /// Faces this crate compiles.
    /// 本 crate 编译的注册面数。
    pub(crate) compiles: usize,
    /// Files the **development** shape mounts for this package.
    /// **开发**形状为这个包挂载的文件数。
    pub(crate) mounts: usize,
    /// Files the **release** shape copies into it.
    /// **发布**形状复制进它的文件数。
    pub(crate) copies: usize,
    /// What is on disk.
    /// 磁盘上的状态。
    pub(crate) on_disk: OnDisk,
    /// Regular files under it, and their total size.
    /// 它之下的常规文件数，以及总字节数。
    pub(crate) files: usize,
    /// Total size in bytes.
    /// 总字节数。
    pub(crate) bytes: u64,
    /// What it depends on inside this partition: the facade depends on the host and every ghost,
    /// which is what decides the order they can be published in.
    /// 它在本次拆分内依赖什么：facade 依赖宿主与每个幽灵，而这决定了它们能被发布的顺序。
    pub(crate) depends_on: Vec<String>,
    /// What stands between this package and `cargo publish`, as sentences.
    /// 这个包与 `cargo publish` 之间还差什么，逐句给出。
    pub(crate) publish: Vec<String>,
}

/// The partition as a screen or a printer can describe it.
/// 拆分，按画面或打印机能描述的样子。
///
/// Public because two surfaces consume it — Studio's partition screen and the MCP bridge's crate
/// tool — and a second description of the same tree is a second answer to "what would this write".
/// 公开是因为有两个执行面消费它——Studio 的分区屏与 MCP 桥的 crate 工具——而对同一棵树写第二份描述，就是
/// 对"这次写入会写下什么"给出第二个答案。
#[derive(Clone, Debug)]
pub struct PartitionView {
    /// The declared package prefix.
    /// 声明的包前缀。
    pub(crate) package_prefix: String,
    /// Faces the host's own tree has.
    /// 宿主自己那棵树有多少个注册面。
    pub(crate) host_faces: usize,
    /// One row per generated package.
    /// 每个生成包一行。
    pub(crate) packages: Vec<PackageView>,
    /// Where the workspace config and member list belong.
    /// 工作区配置与成员清单该落在哪里。
    pub(crate) config_root: PathBuf,
    /// The enclosing workspace, when there is one.
    /// 外层工作区（若存在）。
    pub(crate) workspace: Option<PathBuf>,
    /// Whether the enclosing workspace lists every generated package.
    /// 外层工作区是否列出了每一个生成包。
    pub(crate) members_missing: Vec<String>,
    /// What a reader has to know before acting: no records, a shape mismatch, a foreign directory.
    /// 动手之前必须知道的事：没有记录、形状不符、目录不是本动作的。
    pub(crate) notes: Vec<String>,
}

/// Read the partition, describing the disk state of every package it would write.
/// 读取拆分，并描述它将要写的每个包在磁盘上的状态。
pub(crate) fn view(package_root: &Path) -> Result<Option<PartitionView>, String> {
    let Some(plan) = plan(package_root)? else {
        return Ok(None);
    };
    let mut packages = Vec::new();
    for planned in &plan.planned {
        // One mount per face file below the claims, so the mount count is the number of faces this
        // crate compiles — and, in the release shape, the number of files it copies in.
        // 认领之下每份面文件一项挂载，因此挂载数就是本 crate 编译的注册面数——也是发布形状复制进来的文件数。
        let copies = plan
            .release
            .iter()
            .find(|package| package.package == planned.package)
            .map(|package| package.copies.len())
            .unwrap_or(0);
        packages.push(describe(PlannedRow {
            package: &planned.package,
            crate_name: Some(planned.name.clone()),
            subtrees: planned.subtrees.clone(),
            directory: &planned.directory,
            compiles: planned.mounts.len(),
            mounts: planned.mounts.len(),
            copies,
            depends_on: Vec::new(),
        }));
    }
    if let Some(facade) = &plan.facade {
        packages.push(describe(PlannedRow {
            package: &facade.package,
            crate_name: None,
            subtrees: Vec::new(),
            directory: &facade.directory,
            compiles: 0,
            mounts: 0,
            copies: 0,
            depends_on: facade.dependencies.clone(),
        }));
    }
    let members_missing = missing_members(&plan);
    let mut notes = Vec::new();
    for package in &packages {
        if package.on_disk == OnDisk::Foreign {
            notes.push(format!(
                "{} is at {} and is not a package this action created",
                package.package,
                package.directory.display()
            ));
        }
    }
    if !members_missing.is_empty() && plan.workspace.is_some() {
        notes.push(format!(
            "the workspace does not list {} yet",
            members_missing.join(", ")
        ));
    }
    if packages
        .iter()
        .any(|package| package.on_disk == OnDisk::Development)
    {
        notes.push(
            "the development shape is on disk: it mounts the host's files and needs the workspace \
             remap, so it cannot be published"
                .to_owned(),
        );
    }
    Ok(Some(PartitionView {
        package_prefix: plan.package_prefix.clone(),
        host_faces: plan.host_faces,
        packages,
        config_root: plan.config_root.clone(),
        workspace: plan.workspace.clone(),
        members_missing,
        notes,
    }))
}

/// The generated package names an enclosing workspace's member list does not carry.
/// 外层工作区的成员清单里没有的那些生成包名。
fn missing_members(plan: &PartitionPlan) -> Vec<String> {
    let Some(workspace) = &plan.workspace else {
        return Vec::new();
    };
    let Ok(text) = fs::read_to_string(workspace.join("Cargo.toml")) else {
        return Vec::new();
    };
    let mut missing = Vec::new();
    let mut directories: Vec<&Path> = plan
        .planned
        .iter()
        .map(|planned| planned.directory.as_path())
        .collect();
    if let Some(facade) = &plan.facade {
        directories.push(facade.directory.as_path());
    }
    for entry in super::crate_members::entries(workspace, &directories) {
        if !text.contains(&entry) {
            missing.push(entry.trim_matches('"').to_owned());
        }
    }
    missing
}

/// Describe one package: what it is, what is there, how big it is, and what publishing it needs.
/// 描述一个包：它是什么、那里有什么、多大，以及发布它还需要什么。
/// What a row needs before the disk is looked at: one concept, named once, rather than eight
/// positional arguments a caller can transpose.
/// 一行在查看磁盘之前需要的东西：一个概念、只起一个名字，而不是八个调用方可能写错位置的位置参数。
struct PlannedRow<'a> {
    package: &'a str,
    crate_name: Option<String>,
    subtrees: Vec<String>,
    directory: &'a Path,
    compiles: usize,
    mounts: usize,
    copies: usize,
    depends_on: Vec<String>,
}

fn describe(row: PlannedRow<'_>) -> PackageView {
    let PlannedRow {
        package,
        crate_name,
        subtrees,
        directory,
        compiles,
        mounts,
        copies,
        depends_on,
    } = row;
    let (on_disk, files, bytes) = disk_state(directory);
    let publish = publish_notes(package, directory, on_disk);
    PackageView {
        package: package.to_owned(),
        crate_name,
        subtrees,
        directory: directory.to_path_buf(),
        compiles,
        mounts,
        copies,
        depends_on,
        on_disk,
        files,
        bytes,
        publish,
    }
}

/// What is at that directory: nothing, one of this action's two shapes, or somebody else's package.
/// 那个目录里是什么：什么都没有、本动作两种形状之一、或者别人的包。
fn disk_state(directory: &Path) -> (OnDisk, usize, u64) {
    if !directory.exists() {
        return (OnDisk::Absent, 0, 0);
    }
    let manifest = directory.join("Cargo.toml");
    let text = fs::read_to_string(&manifest).unwrap_or_default();
    let generated = text.contains(GENERATED_MARKER)
        || fs::read_to_string(directory.join("src/lib.rs"))
            .map(|text| text.contains(GENERATED_MARKER))
            .unwrap_or(false);
    let (files, bytes) = size_of(directory);
    if !generated {
        return (OnDisk::Foreign, files, bytes);
    }
    if text.contains("shape = \"release\"") {
        return (OnDisk::Release, files, bytes);
    }
    (OnDisk::Development, files, bytes)
}

/// Regular files under a directory, and their total size.
/// 目录之下的常规文件数，以及总字节数。
///
/// The size is what the maintainer asked the crate tool to judge automatically: a partition is a
/// compile-time decision, and a generated package that copies its fragment's sources is the one whose
/// size can surprise (crates.io refuses a `.crate` over 10 MB).
/// 这个大小正是维护者要 crate 工具自动判断的东西：拆分是编译期决定，而一个把碎片源码复制进来的生成包，
/// 其大小最容易出乎意料（crates.io 拒绝超过 10 MB 的 `.crate`）。
fn size_of(directory: &Path) -> (usize, u64) {
    let mut files = 0;
    let mut bytes = 0;
    let Ok(entries) = fs::read_dir(directory) else {
        return (files, bytes);
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        if path.is_dir() {
            // A build directory is not part of the package: cargo excludes it, and counting it would
            // make every number here meaningless.
            // 构建目录不属于这个包：cargo 会排除它，把它算进来会让这里每个数字都失去意义。
            if name == "target" {
                continue;
            }
            let (nested, nested_bytes) = size_of(&path);
            files += nested;
            bytes += nested_bytes;
        } else if let Ok(metadata) = entry.metadata() {
            files += 1;
            bytes += metadata.len();
        }
    }
    (files, bytes)
}

/// What publishing this package would still need, as sentences a reader can act on.
/// 发布这个包还缺什么，写成读者能据以行动的句子。
fn publish_notes(package: &str, directory: &Path, on_disk: OnDisk) -> Vec<String> {
    let mut notes = Vec::new();
    match on_disk {
        OnDisk::Absent => {
            notes.push("not written yet".to_owned());
            return notes;
        }
        OnDisk::Foreign => {
            notes.push("not this action's package".to_owned());
            return notes;
        }
        OnDisk::Development => {
            notes.push(
                "development shape: `#[path]` leaves the package and the remap lives in the \
                 workspace config, so crates.io would reject it — write `--release` to publish"
                    .to_owned(),
            );
            return notes;
        }
        OnDisk::Release => {}
    }
    let text = fs::read_to_string(directory.join("Cargo.toml")).unwrap_or_default();
    if text.contains("publish = false") {
        notes.push("`publish = false`".to_owned());
    }
    for key in ["description", "license"] {
        if !text.contains(&format!("{key} =")) && !text.contains(&format!("{key}.workspace")) {
            notes.push(format!(
                "no `{key}` (crates.io requires one for a new crate)"
            ));
        }
    }
    let path_only: Vec<String> = text
        .lines()
        .filter(|line| line.contains("path =") && !line.contains("version ="))
        .filter_map(|line| line.split('=').next().map(|name| name.trim().to_owned()))
        .filter(|name| name != "xirang-toolchain" && !name.is_empty() && !name.starts_with('#'))
        .collect();
    if !path_only.is_empty() {
        notes.push(format!(
            "{} are path-only dependencies: add a `version` or `cargo publish` refuses {package}",
            path_only.join(", ")
        ));
    }
    if notes.is_empty() {
        notes.push("ready: self-contained, no path-only sibling of its own".to_owned());
    }
    notes
}

#[cfg(test)]
#[path = "partition_view_tests.rs"]
mod partition_view_tests;
