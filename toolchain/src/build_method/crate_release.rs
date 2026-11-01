//! The **release** shape: the same partition, materialized as packages that can be published.
//! 发布形状：同一次拆分，物化成可以发布的包。
//!
//! The development shape mounts a fragment's files from the host package, which is what makes an edit
//! visible in both crates at once — and it is also why that shape cannot be published: crates.io
//! rejects a package whose `#[path]` reaches outside it (measured), and the `--remap-path-prefix` that
//! keeps the identities identical in a development tree lives in a workspace config, which no consumer
//! inherits. This module plans the other half of that trade: **every generated package carries the
//! sources its own build reads**, and nothing about the shape depends on a file outside the package.
//! 开发形状把碎片的文件从宿主包里挂载出来，这正是"一次编辑在两个 crate 里同时可见"的原因——也正是那个
//! 形状发布不了的原因：crates.io 拒绝 `#[path]` 伸到包外的包（已实测），而让身份在开发树里保持一致的
//! `--remap-path-prefix` 住在工作区配置里，没有任何依赖方会继承它。本模块规划的是那笔交易的另一半：
//! **每个生成的包都携带自己构建要读的源码**，而形状里没有任何东西依赖包外的文件。
//!
//! Two measured facts make that cheap rather than clever:
//! 两个实测事实让它变得便宜，而不是取巧：
//!
//! * a face file copied **under the package's own `src/`** keeps its identity, because the declaration
//!   macro derives the identity input from `file!()` by dropping `CARGO_MANIFEST_DIR` and one leading
//!   `src/` ([`xirang_kernel::identity::manifest_relative_source`]) — so the copied file yields the
//!   same `<module>/<file>.rs` the host baked, with **no remap at all**. Measured: the release ghost's
//!   `StaticFace` rows and the cut face's own `assert_static_identity` are byte-identical to the
//!   unpartitioned tree's;
//! * a facade can find its host **at build time** through `cargo metadata`, which is the only spelling
//!   that survives publishing: an inline `path` is rewritten to a registry requirement in the packaged
//!   manifest. Measured: a facade that resolves its host that way builds green.
//! * 一个被复制到**包自己的 `src/` 之下**的注册面文件仍保住身份，因为声明宏从 `file!()` 推导身份输入的方式
//!   是去掉 `CARGO_MANIFEST_DIR` 与其后的一个 `src/`（[`xirang_kernel::identity::manifest_relative_source`]）
//!   ——因此被复制的文件产出的正是宿主烤进去的那个 `<模块>/<文件>.rs`，**完全不需要 remap**。实测：发布幽灵的
//!   `StaticFace` 行与切口面自己的 `assert_static_identity` 都与不分区那棵树逐字节相同；
//! * facade 可以在**构建期**经 `cargo metadata` 找到它的宿主，而那是唯一能在发布后成立的拼写：行内 `path`
//!   在打包后的清单里会被改写成注册局要求。实测：这样解析宿主的 facade 构建全绿。

use std::fs;
use std::path::{Path, PathBuf};

use super::HostCut;
use xirang_kernel::identity::NodeId;
use xirang_kernel::lexicon;

use super::crate_facade::sibling_dependencies;
use super::crate_plan::respell_dependency_paths;
use super::crate_plan::{PlannedCrate, crates_dir};
use super::crate_plan::{toml_section, toml_value};

/// One package of the release shape, planned but not written.
/// 发布形状里的一个包，已规划但尚未写下。
#[derive(Debug)]
pub(crate) struct ReleasePackage {
    /// The package name.
    /// 包名。
    pub(crate) package: String,
    /// Where it goes: the host's sibling, exactly as in the development shape, so reverting one shape
    /// and writing the other lands in the same directories.
    /// 落在哪里：宿主的同级，与开发形状完全相同，因此撤回一个形状、写下另一个形状落在同一批目录里。
    pub(crate) directory: PathBuf,
    /// The claimed subtrees this package compiles, used to decide whether the host needs its edge.
    /// 这个包编译的认领子树，用来判断宿主是否需要它的依赖边。
    pub(crate) subtrees: Vec<String>,
    /// `Cargo.toml`: the host's dependency tables, copied verbatim, plus the shape fingerprint.
    /// `Cargo.toml`：宿主的依赖表逐字照抄，外加形状指纹。
    pub(crate) cargo_toml: String,
    /// `src/lib.rs`: the host's namespace constant, then the generated plan.
    /// `src/lib.rs`：宿主的命名空间常量，然后是生成的计划。
    /// The crate-local alias module (B2), regenerated for the released shape: a released package's own
    /// `src/` names the subtree markers its declaration talks about, exactly as the development shape does.
    /// crate 内的别名模块（B2），为发布形状重新生成：发布出去的包自己的 `src/` 里也点名它声明谈到的那些
    /// 子树标记，与开发形状一致。
    pub(crate) aliases_rs: String,
    pub(crate) lib_rs: String,
    /// `build.rs`: the same pipeline, run over **this package's own** sources.
    /// `build.rs`：同一条管线，跑在**本包自己**的源码上。
    pub(crate) build_rs: String,
    /// Files copied in: `(source under the host, destination relative to this package)`.
    /// 复制进来的文件：`(宿主下的来源, 相对本包的目标)`。
    pub(crate) copies: Vec<(PathBuf, String)>,
}

/// Plan one ghost: the fragment's files copied inside it, and a build that reads them there.
/// 规划一个幽灵：把碎片的文件复制进它里面，并让构建在那里读它们。
///
/// The declaration is copied too, because the pipeline needs it to plan the same mounts and the same
/// ancestor shells the development shape planned: a fragment's `parent: crate::<ancestor>::NODE_ID`
/// has no ancestor module in this package, so the shell is still what makes it resolve.
/// 声明也被复制，因为管线需要它来规划与开发形状相同的挂载和相同的祖先壳：碎片的
/// `parent: crate::<祖先>::NODE_ID` 在本包里没有祖先模块，因此仍然要靠壳才解析得到。
pub(crate) fn plan_ghost(
    host_root: &Path,
    planned: &PlannedCrate,
    faces: &[(String, String, NodeId)],
    cuts: &[HostCut],
) -> Result<ReleasePackage, String> {
    let text = host_manifest(host_root)?;
    let version = toml_value(&text, "version").unwrap_or_else(|| "0.1.0".to_owned());
    let edition = toml_value(&text, "edition").unwrap_or_else(|| "2024".to_owned());
    // Re-spelled for this package's directory: the host's paths are relative to the host, and this
    // ghost is one level down in `crates/` (the same walk the development shapes do).
    // 按本包所在目录重拼：宿主的路径是相对宿主写的，而这个幽灵在 `crates/` 下低一层（开发形状走的是同一个走法）。
    let dependencies = respell_dependency_paths(
        &toml_section(&text, "dependencies").unwrap_or_default(),
        host_root,
        &planned.directory,
    );
    let mut cargo_toml = format!(
        "# {marker}: the dependencies are the host's, with their relative paths re-spelled.\n\
         # 由 XiRang 生成：依赖是宿主的，相对路径已按本包所在目录重拼。\n\
         [package]\nname = {package:?}\nversion = {version:?}\nedition = {edition:?}\n\n\
         [package.metadata.xirang]\nshape = \"release\"\n\
         # The host this fragment was split out of, for a reader of the published package.\n\
         # 这个碎片是从哪个宿主拆出来的，供发布包的读者查看。\n\
         host = {host:?}\n\n[dependencies]\n{dependencies}",
        marker = super::crate_plan::GENERATED_MARKER,
        package = planned.package,
        host = planned.namespace,
    );
    if let Some(block) = toml_section(&text, "build-dependencies") {
        let block = respell_dependency_paths(&block, host_root, &planned.directory);
        cargo_toml.push_str(&format!("\n[build-dependencies]\n{block}"));
    }
    // What the fragment's own build **derives** from is more than what it compiles: the ancestor faces
    // have to be there for a fragment's `parent:` to resolve during derivation (the first release build
    // failed with `parent declaration cannot be resolved` without them), and every face's registration
    // rule file has to be there because the generated tree mounts it. The claim's whole directory comes
    // across — faces, rules, and any file a hand-written host keeps beside them — and each ancestor
    // brings its own file and the rule directory that sits beside it.
    // 碎片自己的构建**推导**所需的东西比它编译的更多：推导时碎片的 `parent:` 要解析得到，祖先面就必须在场
    // （第一次发布构建没有它们，报 `parent declaration cannot be resolved`）；而每个注册面的注册规范文件也
    // 必须在场，因为生成树会挂载它。认领的整个目录一起过来——注册面、规范文件，以及手写宿主放在它们旁边的任何
    // 文件——而每个祖先带来它自己的文件与旁边的规范目录。
    let mut copies: Vec<(PathBuf, String)> = Vec::new();
    for subtree in &planned.subtrees {
        let relative = subtree.replace("::", "/");
        let source = host_root.join("src").join(&relative);
        if !source.is_dir() {
            return Err(format!(
                "add_crates: `{subtree}` is not a directory under {}, so there is nothing to copy \
                 into {}",
                host_root.join("src").display(),
                planned.package
            ));
        }
        copy_tree(&source, Path::new("src").join(&relative), &mut copies)?;
    }
    for (module, _) in &planned.ancestors {
        let Some((source, _, _)) = faces.iter().find(|(_, face, _)| face == module) else {
            return Err(format!(
                "add_crates: the ancestor `{module}` is not one of the faces this build published, so \
                 {} cannot carry the declaration a fragment's `parent:` resolves against",
                planned.package
            ));
        };
        let path = host_root.join("src").join(source);
        let text = std::fs::read_to_string(&path)
            .map_err(|error| format!("add_crates: cannot read {}: {error}", path.display()))?;
        // A custom rule path points outside the ancestor's own directory, and this shape copies
        // directories rather than following declarations — so it is refused by name instead of shipped
        // with a tree whose registration rules are not the host's.
        // 自定义的规范路径指向祖先自己目录之外，而本形状复制的是目录、不追声明——因此它被点名拒绝，而不是带着
        // 一棵注册规范并非宿主的树发出去。
        if text.contains("registry_rule_path") {
            return Err(format!(
                "add_crates: the ancestor `{module}` names a custom `registry_rule_path`, which the \
                 release shape does not follow.\nway forward: move that rule to \
                 `{}`'s `registry_rule/registry_rule.rs`, or partition a subtree that contains it",
                path.parent().unwrap_or(&path).display()
            ));
        }
        let rule = path
            .parent()
            .map(|directory| directory.join("registry_rule"))
            .filter(|directory| directory.is_dir());
        copies.push((path, format!("src/{source}")));
        if let Some(rule) = rule {
            let relative = rule.strip_prefix(host_root.join("src")).map_err(|_| {
                "add_crates: an ancestor's rule directory is outside the host's src".to_owned()
            })?;
            copy_tree(&rule, Path::new("src").join(relative), &mut copies)?;
        }
    }
    copies.push((
        host_root.join(lexicon::ADD_CRATES_FILE),
        lexicon::ADD_CRATES_FILE.to_owned(),
    ));
    Ok(ReleasePackage {
        package: planned.package.clone(),
        directory: planned.directory.clone(),
        subtrees: planned.subtrees.clone(),
        cargo_toml,
        aliases_rs: super::crate_plan::aliases_rs(&planned.subtrees),
        lib_rs: lib_rs(&planned.namespace),
        build_rs: ghost_build_rs(&planned.namespace, &planned.subtrees, cuts),
        copies,
    })
}

/// Plan the facade: it carries no sources, because its build resolves the host as a dependency.
/// 规划 facade：它不携带源码，因为它的构建把宿主当依赖来解析。
pub(crate) fn plan_facade(
    host_root: &Path,
    host_package: &str,
    package_prefix: &str,
    namespace: &str,
    planned: &[PlannedCrate],
    cuts: &[HostCut],
) -> Result<ReleasePackage, String> {
    if planned.is_empty() {
        return Err(
            "add_crates: a release facade needs at least one generated package to see".to_owned(),
        );
    }
    let package = format!("{package_prefix}-facade");
    if package == host_package {
        return Err(format!(
            "add_crates: the facade would be called `{package}`, the same as the host package. \
             Rename the host or pick a different `package_prefix`; a crate cannot depend on itself."
        ));
    }
    // The same one implementation the ghost and the development facade use: a release shape that put
    // its packages somewhere else would be a fourth answer to "where does a partition write".
    // 与幽灵、开发形状的 facade 共用同一份实现：发布形状若把包写到别处，就是"划分写到哪里"的第四个答案。
    let directory = crates_dir(&super::crate_plan::partition_roots(host_root).0).join(&package);
    let cargo_toml = facade_cargo_toml(
        host_root,
        &package,
        &directory,
        host_package,
        namespace,
        planned,
    )?;
    Ok(ReleasePackage {
        package,
        directory,
        subtrees: Vec::new(),
        cargo_toml,
        aliases_rs: super::crate_plan::aliases_rs(
            &planned
                .iter()
                .flat_map(|ghost| ghost.subtrees.iter().cloned())
                .collect::<Vec<_>>(),
        ),
        lib_rs: lib_rs(namespace),
        build_rs: facade_build_rs(host_package, namespace, cuts),
        copies: Vec::new(),
    })
}

/// Every file under `source`, as `(host path, package-relative destination)` copies.
/// `source` 之下的每个文件，写成 `(宿主路径, 包内相对目标)` 的复制项。
fn copy_tree(
    source: &Path,
    destination: PathBuf,
    copies: &mut Vec<(PathBuf, String)>,
) -> Result<(), String> {
    let entries = std::fs::read_dir(source)
        .map_err(|error| format!("add_crates: cannot read {}: {error}", source.display()))?;
    for entry in entries {
        let entry = entry
            .map_err(|error| format!("add_crates: cannot read {}: {error}", source.display()))?;
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        let target = destination.join(&name);
        if path.is_dir() {
            copy_tree(&path, target, copies)?;
        } else {
            copies.push((path, target.to_string_lossy().replace('\\', "/")));
        }
    }
    Ok(())
}

/// The host's manifest text.
/// 宿主的清单文本。
fn host_manifest(host_root: &Path) -> Result<String, String> {
    let manifest = host_root.join("Cargo.toml");
    std::fs::read_to_string(&manifest)
        .map_err(|error| format!("add_crates: cannot read {}: {error}", manifest.display()))
}

/// `src/lib.rs`, shared by both packages: the namespace, then this package's own generated plan.
/// `src/lib.rs`，两个包共用：命名空间，然后是本包自己的生成计划。
fn lib_rs(namespace: &str) -> String {
    format!(
        "//! {marker}: compile one package of a host's registration tree as published sources.\n\
         //! Do not edit — the declaration lives in this package's `{declaration}`.\n\
         //! 由 XiRang 生成：把宿主注册树的一个包当作已发布的源码来编译。请勿手工修改——声明住在本包的\n\
         //! `{declaration}` 里。\n\n\
         pub const XIRANG_NAMESPACE: &str = {namespace:?};\n\n\
         include!(concat!(env!(\"OUT_DIR\"), \"/generated_lib.rs\"));\n",
        marker = super::crate_plan::GENERATED_MARKER,
        declaration = lexicon::ADD_CRATES_FILE,
        namespace = namespace,
    )
}

/// The release ghost's `build.rs`: the pipeline over **this package's** root.
/// 发布幽灵的 `build.rs`：管线跑在**本包自己**的根上。
///
/// `env!("CARGO_MANIFEST_DIR")` is expanded when the consumer compiles this script, so the spelling is
/// theirs and there is nothing to remap: the source is under the manifest directory, which is exactly
/// the case the identity derivation handles by itself.
/// `env!("CARGO_MANIFEST_DIR")` 在依赖方编译本脚本时展开，因此拼写是他们的、没有任何东西需要 remap：源码就在
/// 清单目录之下，而那正是身份推导自己处理得了的情形。
fn ghost_build_rs(namespace: &str, subtrees: &[String], cuts: &[HostCut]) -> String {
    let (function, cuts_argument) = super::crate_plan::partition_call(cuts);
    format!(
        "//! {marker}: build this fragment out of its own sources.\n\
         //! 由 XiRang 生成：从它自己的源码构建这个碎片。\n\n\
         fn main() {{\n\
         {i}println!(\"cargo:rerun-if-changed=src\");\n\
         {i}println!(\"cargo:rerun-if-changed={declaration}\");\n\
         {i}let root = std::path::Path::new(env!(\"CARGO_MANIFEST_DIR\"));\n\
         {i}let out = std::path::PathBuf::from(std::env::var(\"OUT_DIR\").expect(\"OUT_DIR\"));\n\
         {i}xirang_toolchain::build_method::{function}(\n\
         {i}    root,\n\
         {i}    &out,\n\
         {i}    {namespace:?},\n\
         {i}    Some({claims:?}),\n\
         {i}    false,\n\
         {cuts_argument}\
         {i})\n\
         {i}.expect(\"xirang\");\n\
         }}\n",
        i = "    ",
        marker = super::crate_plan::GENERATED_MARKER,
        declaration = lexicon::ADD_CRATES_FILE,
        claims = subtrees.join(","),
        namespace = namespace,
    )
}

/// Whether another package can depend on this host: does it have a **library** target?
/// 别的包能不能依赖这个宿主：它有**库**目标吗？
///
/// Read from the filesystem rather than asked of cargo, and the reason is the caller: the partition
/// writer runs before the generated packages exist, offline, on a manifest whose dependency paths may not
/// resolve yet — `cargo metadata` would refuse it for reasons that have nothing to do with this question.
/// What cargo itself keys on is `src/lib.rs`, or a `[lib] path` that names one; that is what this asks.
/// 从文件系统读，而不是问 cargo，理由在调用方：分区写入方在生成包存在之前、离线、在一份依赖路径可能还解析不
/// 了的清单上运行——`cargo metadata` 会因为与这个问题毫无关系的原因拒绝它。cargo 自己认的是 `src/lib.rs`，
/// 或者一个指名了某个文件的 `[lib] path`；这里问的就是这个。
///
/// The answer is load-bearing: a facade lists the host as a dependency, cargo **ignores** a dependency
/// with no library target (warns, builds on), and measured on a binary host the facade then compiled zero
/// faces and emitted zero cuts while reporting success (audit `M7`, §M7.55).
/// 这个答案是要紧的：facade 把宿主列为依赖，而 cargo 会**忽略**没有库目标的依赖（警告一句、继续构建），
/// 在二进制宿主上实测，facade 随后编译了零个面、发射了零条切口，却报告成功（审计 `M7`，§M7.55）。
pub(crate) fn has_library_target(host_root: &Path) -> bool {
    if host_root.join("src/lib.rs").is_file() {
        return true;
    }
    let Ok(manifest) = fs::read_to_string(host_root.join("Cargo.toml")) else {
        return false;
    };
    let Some(lib) = super::crate_plan::toml_section(&manifest, "lib") else {
        return false;
    };
    // A `[lib]` table with no `path` is `src/lib.rs`, which the check above already answered.
    // 没有 `path` 的 `[lib]` 表就是 `src/lib.rs`，上面的检查已经答过了。
    match super::crate_plan::toml_value(&lib, "path") {
        Some(path) => host_root.join(path).is_file(),
        None => false,
    }
}

/// Refuse a release partition whose host cargo will not hand to a facade.
/// 拒绝一次"宿主不会被 cargo 交给 facade"的发布分区。
///
/// The check lives at the **writers**, not in the planner: planning is also what a read-only view does, and
/// a declaration edit that merely *looks* at the plan must not fail because the host is a binary — measured,
/// putting it in `plan_facade` made `crates --declare … --write` refuse an edit that has nothing to do with
/// release shapes (audit `M7`, §M7.55).
/// 这项检查住在**写入方**，不在规划器里：只读视图做的也是规划，而一次只是**看一眼**计划的声明编辑，不该因为
/// 宿主是二进制而失败——实测，把它放进 `plan_facade` 让 `crates --declare … --write` 拒绝了一次与发布形状毫无
/// 关系的编辑（审计 `M7`，§M7.55）。
///
/// What it refuses: cargo **ignores** a dependency with no library target (it warns and builds on), and the
/// facade's build script then took its own directory for the host's — measured, the published facade carried
/// neither the faces nor the cuts and the build still reported success.
/// 它拒绝什么：cargo 会**忽略**没有库目标的依赖（警告一句然后继续构建），而 facade 的构建脚本随后把自己的目录
/// 当成了宿主的——实测，发布出去的 facade 既不带面也不带切口，而构建仍报告成功。
pub(crate) fn refuse_binary_host(host_root: &Path) -> Result<(), String> {
    if has_library_target(host_root) {
        return Ok(());
    }
    let host_package = super::package::package_name(&host_root.join("Cargo.toml"))
        .unwrap_or_else(|_| host_root.display().to_string());
    Err(format!(
        "add_crates: the host package `{host_package}` has no library target, so a release facade \
         cannot reach its tree: cargo ignores a dependency that is only a binary, and the facade would \
         compile no faces and carry no cuts while still reporting success.\n\
         way forward: give the host a `src/lib.rs` (a library host with a thin binary, the shape the \
         examples use), or partition in the development shape"
    ))
}

/// The release facade's `build.rs`: find the host package through cargo, then run the facade pipeline.
/// 发布 facade 的 `build.rs`：经 cargo 找到宿主包，再跑 facade 管线。
///
/// The host is a **dependency**, and a published facade's manifest no longer carries the inline `path`
/// cargo rewrites to a registry requirement at package time — so the root is resolved at build time
/// from `cargo metadata`, which lists every dependency with the directory it was unpacked into. The
/// JSON is scanned rather than parsed because this script has no dependency of its own, and the scan
/// keys on the package name plus the very next `manifest_path`, which the metadata format keeps
/// adjacent inside one package object (measured: builds green).
/// 宿主是一个**依赖**，而发布后的 facade 清单里不再带行内 `path`——打包时 cargo 会把它改写成注册局要求——
/// 因此根在构建期由 `cargo metadata` 解析，它会列出每个依赖及其被解包到的目录。这里的 JSON 是扫描而不是解析，
/// 因为这个脚本没有自己的依赖；扫描以包名为键、取紧接其后的 `manifest_path`，而元数据格式把两者放在同一个包
/// 对象里相邻的位置（实测：构建全绿）。
fn facade_build_rs(host_package: &str, namespace: &str, cuts: &[HostCut]) -> String {
    let (function, cuts_argument) = super::crate_plan::partition_call(cuts);
    format!(
        "//! {marker}: build the cross-crate half out of the host package it depends on.\n\
         //! 由 XiRang 生成：从它依赖的那个宿主包里构建跨 crate 那一半。\n\n\
         /// The host package's root, as cargo placed it.\n\
         fn host_root() -> std::path::PathBuf {{\n\
         {i}let manifest = std::path::Path::new(env!(\"CARGO_MANIFEST_DIR\")).join(\"Cargo.toml\");\n\
         {i}let output = std::process::Command::new(\"cargo\")\n\
         {i}    .args([\"metadata\", \"--format-version\", \"1\", \"--manifest-path\"])\n\
         {i}    .arg(&manifest)\n\
         {i}    .output()\n\
         {i}    .expect(\"cargo metadata\");\n\
         {i}if !output.status.success() {{\n\
         {i}    panic!(\"cargo metadata failed: {{}}\", String::from_utf8_lossy(&output.stderr));\n\
         {i}}}\n\
         {i}let text = String::from_utf8(output.stdout).expect(\"utf-8 metadata\");\n\
         {i}let name = format!(\"\\\"name\\\":{{:?}},\\\"version\\\":\", {host:?});\n\
         {i}let at = text.find(&name).unwrap_or_else(|| panic!(\"`{host}` is not a dependency of this facade\"));\n\
         {i}let rest = &text[at..];\n\
         {i}let key = \"\\\"manifest_path\\\":\\\"\";\n\
         {i}let start = rest.find(key).expect(\"the host's manifest path\") + key.len();\n\
         {i}let end = start + rest[start..].find('\"').expect(\"the closing quote\");\n\
         {i}std::path::PathBuf::from(&rest[start..end])\n\
         {i}    .parent()\n\
         {i}    .expect(\"the host's package root\")\n\
         {i}    .to_path_buf()\n\
         }}\n\n\
         fn main() {{\n\
         {i}println!(\"cargo:rerun-if-changed=src\");\n\
         {i}let host = host_root();\n\
         {i}println!(\"cargo:rerun-if-changed={{}}\", host.join(\"src\").display());\n\
         {i}let out = std::path::PathBuf::from(std::env::var(\"OUT_DIR\").expect(\"OUT_DIR\"));\n\
         {i}xirang_toolchain::build_method::{function}(\n\
         {i}    &host,\n\
         {i}    &out,\n\
         {i}    {namespace:?},\n\
         {i}    None,\n\
         {i}    true,\n\
         {cuts_argument}\
         {i})\n\
         {i}.expect(\"xirang\");\n\
         }}\n",
        i = "    ",
        marker = super::crate_plan::GENERATED_MARKER,
        host = host_package,
        namespace = namespace,
    )
}

/// The release facade's `Cargo.toml`: the host's dependencies, plus it and every ghost as siblings.
/// 发布 facade 的 `Cargo.toml`：宿主的依赖，加上宿主与每个幽灵作为同级依赖。
///
/// Unlike the development shape's facade this one is **publishable**: no `publish = false`, and the
/// sibling dependencies keep the inline `path` (what cargo turns into a registry requirement when the
/// package is published) — which is also why the generated build script resolves the host through
/// cargo rather than from that path.
/// 与开发形状的 facade 不同，这一个**可发布**：没有 `publish = false`，而同级依赖保留行内 `path`（cargo 在
/// 打包时会把它变成注册局要求）——这也正是生成的构建脚本经 cargo 而不是从这个路径解析宿主的原因。
fn facade_cargo_toml(
    host_root: &Path,
    package: &str,
    directory: &Path,
    host_package: &str,
    namespace: &str,
    planned: &[PlannedCrate],
) -> Result<String, String> {
    let text = host_manifest(host_root)?;
    let version = toml_value(&text, "version").unwrap_or_else(|| "0.1.0".to_owned());
    let edition = toml_value(&text, "edition").unwrap_or_else(|| "2024".to_owned());
    let mut dependencies = respell_dependency_paths(
        &toml_section(&text, "dependencies").unwrap_or_default(),
        host_root,
        directory,
    );
    for (name, path) in sibling_dependencies(directory, host_root, host_package, planned) {
        dependencies.push_str(&format!("{name} = {{ path = {path:?} }}\n"));
    }
    let mut output = format!(
        "# {marker}: the dependencies are the host's, with their relative paths re-spelled, plus the\n\
         # crates it hands work to.\n\
         # 由 XiRang 生成：依赖是宿主的（相对路径已重拼），再加上它把工作交出去的那些 crate。\n\
         [package]\nname = {package:?}\nversion = {version:?}\nedition = {edition:?}\n\n\
         [package.metadata.xirang]\nshape = \"release\"\nhost = {namespace:?}\n\n[dependencies]\n{dependencies}",
        marker = super::crate_plan::GENERATED_MARKER,
    );
    if let Some(block) = toml_section(&text, "build-dependencies") {
        // Re-spelled exactly like `[dependencies]`: cargo refuses a crate whose path differs between
        // build targets ("different source paths depending on the build target"), and the host spells
        // its toolchain path relative to the host, not to this package.
        // 与 `[dependencies]` 一样重拼：cargo 会拒绝"同一个 crate 在不同 target 下路径不同"的清单，而宿主
        // 那条 toolchain 路径是相对**宿主**写的，不是相对本包。
        let block = respell_dependency_paths(&block, host_root, directory);
        output.push_str(&format!("\n[build-dependencies]\n{block}"));
    }
    Ok(output)
}

#[cfg(test)]
#[path = "crate_release_tests.rs"]
mod crate_release_tests;
