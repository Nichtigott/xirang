//! Planning the **facade**: the crate that compiles the cross-crate half (audit `M7`, §M7.33).
//! 规划 **facade**：那个编译跨 crate 那一半的 crate（审计 `M7`，§M7.33）。
//!
//! A partition hands subtrees to ghost crates, and once a subtree leaves, the host's generated tree can
//! no longer name it: `assert_static_registration(control::REGISTRATION.registry_rule,
//! control::object::button::REGISTRATION)` and `StaticGraftCut::from_ids(crate::control::object::
//! button::NODE_ID, …)` both point at a module the host does not compile any more. The facade is the
//! crate that sees **both ends**: it depends on the host, on every ghost, and on every crate the graft
//! declarations name, and it carries the cut table plus the contract assertions with every
//! `crate::<module>` rewritten to the crate that compiles that module.
//! 一次拆分把子树交给幽灵 crate，而子树一旦离开，宿主的生成树就再也点不了它的名：
//! `assert_static_registration(control::REGISTRATION.registry_rule, control::object::button::REGISTRATION)`
//! 与 `StaticGraftCut::from_ids(crate::control::object::button::NODE_ID, …)` 都指向一个宿主不再编译的模块。
//! facade 就是那个**两端都看得见**的 crate：它依赖宿主、每个幽灵、以及 graft 声明点名的每个 crate，并携带
//! 切口表与契约断言——其中每个 `crate::<模块>` 都改写成编译该模块的那个 crate。
//!
//! It is planned here and **written** by the authoring CLI, exactly like a ghost: the planner returns
//! bytes, and nothing here touches the filesystem.
//! 它在这里被规划、由创作 CLI **写下**，与幽灵完全一样：规划器返回字节，这里不碰文件系统。

use std::fs;
use std::path::{Path, PathBuf};

use super::HostCut;
use xirang_kernel::lexicon;

use super::crate_plan::{
    GENERATED_MARKER, PlannedCrate, relative_walk, respell_dependency_paths, toml_section,
    toml_value,
};

/// A facade, planned but not written.
/// 一个已规划但尚未写下的 facade。
#[derive(Debug)]
pub(crate) struct PlannedFacade {
    /// The package name, `<package_prefix>-facade`.
    /// 包名，`<package_prefix>-facade`。
    pub(crate) package: String,
    /// Where it goes: the host's sibling, so the host's relative dependency paths resolve identically.
    /// 落在哪里：宿主的同级目录，这样宿主的相对依赖路径解析结果完全相同。
    pub(crate) directory: PathBuf,
    /// The crates the facade is planned to see: the host and each ghost. The implementation crates the
    /// graft declarations name arrive with the host's copied dependency table, which is why they are not
    /// listed here.
    /// facade 被规划为看得见的 crate：宿主与每个幽灵。graft 声明点名的实现 crate 随照抄来的宿主依赖表一起
    /// 到来，因此不在这里列出。
    pub(crate) dependencies: Vec<String>,
    /// `src/lib.rs`: the host's namespace constant, then the generated plan.
    /// `src/lib.rs`：宿主的命名空间常量，然后是生成的计划。
    /// The crate-local alias module (B2): the union of the claims below, so the facade's own `src/`
    /// names every subtree marker this partition exposes.
    /// crate 内的别名模块（B2）：下面各认领子树的并集，于是 facade 自己的 `src/` 点名这次分区暴露的每一个
    /// 子树标记。
    pub(crate) aliases_rs: String,
    pub(crate) lib_rs: String,
    /// `build.rs`: run the host's manifest through the pipeline in **facade** mode.
    /// `build.rs`：让宿主的清单以 **facade** 模式跑一遍管线。
    pub(crate) build_rs: String,
    /// `Cargo.toml`: the host's dependencies, copied verbatim, plus the sibling path dependencies.
    /// `Cargo.toml`：宿主的依赖逐字照抄，再加上同级目录的 path 依赖。
    pub(crate) cargo_toml: String,
}

/// Plan the facade for one host, or `None` when there is nothing cross-crate to carry.
/// 为一个宿主规划 facade；没有跨 crate 的东西要承载时返回 `None`。
///
/// `None` is the honest answer for a declaration whose crates are empty: a facade with no ghost to see
/// and no cut to assert is a crate that exists to say nothing. `implementation_crates` are the crate
/// names the **typed** graft expressions begin with; the host's own dependency table already carries
/// their paths, and the facade — being the host's sibling — resolves those paths identically.
/// 对"声明的 crate 为空"来说 `None` 是诚实的答案：一个没有幽灵可看、没有切口可断言的 facade，是为说空话而
/// 存在的 crate。`implementation_crates` 是**类型化** graft 表达式开头的那些 crate 名；宿主自己的依赖表已经
/// 带着它们的路径，而 facade 作为宿主的同级目录，解析那些路径的结果完全相同。
pub(crate) fn plan_facade(
    host_root: &Path,
    package_prefix: &str,
    namespace: &str,
    host_package: &str,
    planned: &[PlannedCrate],
    cuts: &[HostCut],
) -> Result<Option<PlannedFacade>, String> {
    if planned.is_empty() {
        return Ok(None);
    }
    let package = format!("{package_prefix}-facade");
    if package == host_package {
        return Err(format!(
            "add_crates: the facade would be called `{package}`, the same as the host package. \
             Rename the host or pick a different `package_prefix`; a crate cannot depend on itself."
        ));
    }
    let directory = super::crate_plan::crates_dir(&super::crate_plan::partition_roots(host_root).0)
        .join(&package);
    let mut dependencies: Vec<String> = vec![host_package.to_owned()];
    dependencies.extend(planned.iter().map(|planned| planned.package.clone()));
    dependencies.sort();
    dependencies.dedup();
    let lib_rs = facade_lib_rs(namespace);
    let build_rs = facade_build_rs(host_root, &directory, namespace, cuts);
    let cargo_toml = facade_cargo_toml(host_root, &package, &directory, host_package, planned)?;
    Ok(Some(PlannedFacade {
        package,
        directory,
        dependencies,
        aliases_rs: super::crate_plan::aliases_rs(
            &planned
                .iter()
                .flat_map(|ghost| ghost.subtrees.iter().cloned())
                .collect::<Vec<_>>(),
        ),
        lib_rs,
        build_rs,
        cargo_toml,
    }))
}

/// The facade's `src/lib.rs`.
/// facade 的 `src/lib.rs`。
fn facade_lib_rs(namespace: &str) -> String {
    format!(
        "//! {marker}: the crate that compiles the cross-crate half of a host's registration tree.\n\
         //! Do not edit — the declaration lives in the host's `{declaration}`.\n\
         //! 由 XiRang 生成：编译宿主注册树跨 crate 那一半的 crate。请勿手工修改——声明住在宿主的\n\
         //! `{declaration}` 里。\n\n\
         pub const XIRANG_NAMESPACE: &str = {namespace:?};\n\n\
         include!(concat!(env!(\"OUT_DIR\"), \"/generated_lib.rs\"));\n",
        marker = GENERATED_MARKER,
        declaration = lexicon::ADD_CRATES_FILE,
    )
}

/// The facade's `build.rs`.
/// facade 的 `build.rs`。
///
/// Same shape as a ghost's, with one difference that is the whole point: the mode is **facade**, so the
/// tree this crate renders carries no modules and no face identities — it carries the cut table and the
/// contract assertions, rewritten to the crates that own each module.
/// 形状与幽灵的相同，只有一处不同而这处正是要点：模式是 **facade**，因此这个 crate 渲染出来的树不发模块、
/// 不发面的身份——它携带切口表与契约断言，并改写成各自模块的属主 crate。
fn facade_build_rs(
    host_root: &Path,
    fragment_dir: &Path,
    namespace: &str,
    cuts: &[HostCut],
) -> String {
    let (function, cuts_argument) = super::crate_plan::partition_call(cuts);
    let host_crate = super::package::declared_library_name(&host_root.join("Cargo.toml"))
        .unwrap_or_else(|| {
            super::package::package_name(&host_root.join("Cargo.toml"))
                .unwrap_or_default()
                .replace('-', "_")
        });
    let declaration = lexicon::ADD_CRATES_FILE;
    // Same walk as a ghost's: the facade is committed too, so it reaches the host from
    // `CARGO_MANIFEST_DIR` instead of naming the machine that generated it.
    // 与幽灵相同的走法：facade 同样要入库，因此它从 `CARGO_MANIFEST_DIR` 到达宿主，而不是点名产生它的机器。
    let host = super::crate_plan::host_binding(fragment_dir, host_root, "    ");
    format!(
        "//! {marker}: build the cross-crate half out of the host's sources.\n\
         //! 由 XiRang 生成：从宿主的源码构建跨 crate 那一半。\n\n\
         #[rustfmt::skip]\n\
         fn main() {{\n\
         {host}\
         {i}println!(\"cargo:rerun-if-changed={{}}\", host.join(\"src\").display());\n\
         {i}println!(\"cargo:rerun-if-changed={{}}\", host.join({declaration:?}).display());\n\
         {i}let out = std::path::PathBuf::from(std::env::var(\"OUT_DIR\").expect(\"OUT_DIR\"));\n\
         {i}// The host's library name is a fact of the committed partition plan; it is passed as an\n\
         {i}// input instead of being re-derived (or set through a process environment) here.\n\
         {i}// 宿主库名是已提交分区计划里的一个事实；它作为输入传入，而不是在这里重推（也不改进程环境）。\n\
         {i}xirang_toolchain::build_method::{function}_named(\n\
         {i}    &host,\n\
         {i}    &out,\n\
         {i}    {namespace:?},\n\
         {i}    None,\n\
         {i}    true,\n\
         {cuts_argument}\
         {i}    Some({host_crate:?}),\n\
         {i})\n\
         {i}.expect(\"xirang\");\n\
         }}\n",
        i = "    ",
        host = host,
        declaration = declaration,
        marker = GENERATED_MARKER,
    )
}

/// The facade's `Cargo.toml`: the host's dependencies, with their relative paths re-spelled, plus the
/// sibling path dependencies.
/// facade 的 `Cargo.toml`：宿主的依赖（相对路径已重拼），加上同级目录的 path 依赖。
///
/// Every path in the copied table is re-spelled **relative to the facade** rather than copied verbatim:
/// a copied table is not a second answer to "what does this source need", but a copied *path* is not an
/// answer at all unless it lands on the same package, and the facade no longer sits where the host does.
/// 照抄来的表里每条路径都**相对 facade 重拼**，而不是逐字照抄：照抄来的表不是"这份源码需要什么"的第二个
/// 答案，但照抄来的**路径**只有落在同一个包上才算答案，而 facade 已经不在宿主所在的位置了。
fn facade_cargo_toml(
    host_root: &Path,
    package: &str,
    directory: &Path,
    host_package: &str,
    planned: &[PlannedCrate],
) -> Result<String, String> {
    let manifest = host_root.join("Cargo.toml");
    let text = fs::read_to_string(&manifest).map_err(|error| {
        format!(
            "add_crates: cannot read {} to copy its dependencies: {error}",
            manifest.display()
        )
    })?;
    let version = toml_value(&text, "version").unwrap_or_else(|| "0.1.0".to_owned());
    let edition = toml_value(&text, "edition").unwrap_or_else(|| "2024".to_owned());
    let mut dependencies = super::crate_plan::strip_generated_dependencies(
        &respell_dependency_paths(
            &toml_section(&text, "dependencies").unwrap_or_default(),
            host_root,
            directory,
        ),
        &{
            let mut names: Vec<String> = planned.iter().map(|p| p.package.clone()).collect();
            names.push(format!("{host_package}-facade"));
            names
        },
    );
    for (name, path) in sibling_dependencies(directory, host_root, host_package, planned) {
        dependencies.push_str(&format!("{name} = {{ path = {path:?} }}\n"));
    }
    let mut output = format!(
        "# {marker}: the dependencies are the host's, with their relative paths re-spelled, plus the\n\
         # crates it hands work to.\n\
         # 由 XiRang 生成：依赖是宿主的（相对路径已重拼），再加上它把工作交出去的那些 crate。\n\
         [package]\nname = {package:?}\nversion = {version:?}\nedition = {edition:?}\n\
         publish = false\n\n[dependencies]\n{dependencies}",
        marker = GENERATED_MARKER,
    );
    if let Some(block) = toml_section(&text, "build-dependencies") {
        let block = respell_dependency_paths(&block, host_root, directory);
        output.push_str(&format!("\n[build-dependencies]\n{block}"));
    }
    Ok(output)
}

/// `(dependency name, relative path)` for the host and for every ghost.
/// 宿主与每个幽灵的 `(依赖名, 相对路径)`。
pub(super) fn sibling_dependencies(
    directory: &Path,
    host_root: &Path,
    host_package: &str,
    planned: &[PlannedCrate],
) -> Vec<(String, String)> {
    let mut siblings = vec![(host_package.to_owned(), host_root.to_path_buf())];
    siblings.extend(
        planned
            .iter()
            .map(|planned| (planned.package.clone(), planned.directory.clone())),
    );
    siblings
        .into_iter()
        .filter_map(|(name, target)| {
            relative_walk(directory, &target.join("Cargo.toml")).map(|walk| {
                (
                    name.clone(),
                    walk.trim_end_matches("/Cargo.toml").to_owned(),
                )
            })
        })
        .collect()
}

#[cfg(test)]
#[path = "crate_facade_tests.rs"]
mod crate_facade_tests;
