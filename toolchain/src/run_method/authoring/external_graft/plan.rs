//! Persistent declarations for immutable external grafts.
//! 不可变外部 graft 的持久化声明。
//!
//! A plan file is an authoring record, not a declaration the compiler sees and
//! not an overlay application. It names the logical slot (`target_path`), the
//! base face's identity (`target`), the external implementation's selector, and
//! whether the whole subtree or only the node is replaced. `GraftPlanDocument`
//! in the kernel owns the text format; this module owns the filesystem.
//! 计划文件是创作记录：它不是编译器看到的声明，也不是覆盖应用。它记录逻辑槽位
//! （`target_path`）、原注册面的身份（`target`）、外部实现的选择器，以及替换整棵
//! 子树还是只替换节点。文本格式由 kernel 的 `GraftPlanDocument` 拥有，本模块只
//! 负责文件系统。

use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::run_method::runtime::{
    LoadedGraft, graft_record_root, load_graft_record, load_graft_records,
};
use crate::run_method::{GraftPlanDocument, NodeId, Registry};
use xirang_kernel::lexicon;

use super::super::context::package_root;
use super::super::filesystem::atomic_write;

/// The directory that owns every external graft plan.
/// 拥有全部外部 graft 计划的目录。
///
/// The layout lives in the non-gated runtime loader now, so Studio and a
/// runtime host resolve the same path from the same constant list.
/// 版式现在住在不受门控的运行期加载器里，因此 Studio 与运行期宿主按同一份常量表解析
/// 同一条路径。
pub fn external_graft_root() -> PathBuf {
    graft_record_root(&package_root())
}

/// An external overlay declaration owned by the host package.
/// 宿主包拥有的外部覆盖声明。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExternalGraftPlanFile {
    /// The plan's directory name under `.xirang/external-grafts/`.
    /// 计划在 `.xirang/external-grafts/` 下的目录名。
    pub selector: String,
    /// The parsed plan body.
    /// 解析后的计划主体。
    pub document: GraftPlanDocument,
    /// The plan directory itself.
    /// 该计划所在目录。
    pub root: PathBuf,
}

impl ExternalGraftPlanFile {
    /// The path of the plan file inside `root`.
    /// `root` 内计划文件的路径。
    pub fn plan_path(&self) -> PathBuf {
        self.root.join(lexicon::GRAFT_PLAN_FILE)
    }

    /// The identity of the base face this plan replaces.
    /// 该计划所替换原注册面的身份。
    pub fn target(&self) -> NodeId {
        self.document.target
    }

    /// The logical slot path this plan replaces.
    /// 该计划替换的逻辑槽位路径。
    pub fn target_path(&self) -> &str {
        &self.document.target_path
    }

    /// The selector naming the external implementation.
    /// 命名外部实现的选择器。
    pub fn graft(&self) -> &str {
        &self.document.graft
    }

    /// Whether the whole subtree is replaced rather than only the node.
    /// 替换整棵子树还是仅替换该节点。
    pub fn full(&self) -> bool {
        self.document.full
    }
}

/// One directory under `.xirang/external-grafts/`.
/// `.xirang/external-grafts/` 下的一个目录。
///
/// A plan the tooling can read and a plan it cannot are both listed: a broken
/// file is something the author has to see, not something to hide.
/// 读得懂与读不懂的计划都会被列出：坏文件是作者必须看见的东西，不该被藏起来。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExternalGraftPlanEntry {
    /// The plan's directory name under `.xirang/external-grafts/`.
    /// 计划在 `.xirang/external-grafts/` 下的目录名。
    pub selector: String,
    /// The plan directory itself.
    /// 该计划所在目录。
    pub root: PathBuf,
    /// The parsed plan, or the reason it could not be read.
    /// 解析后的计划，或无法读取的原因。
    pub document: Result<GraftPlanDocument, String>,
}

impl ExternalGraftPlanEntry {
    /// The path of the plan file inside `root`.
    /// `root` 内计划文件的路径。
    pub fn plan_path(&self) -> PathBuf {
        self.root.join(lexicon::GRAFT_PLAN_FILE)
    }

    /// Borrow the parsed plan, or the stored reason it is unusable.
    /// 借出解析后的计划，或它不可用的已存原因。
    pub fn document(&self) -> Result<&GraftPlanDocument, &str> {
        self.document.as_ref().map_err(String::as_str)
    }
}

/// Reject a selector the plan format could not read back.
/// 拒绝计划格式读不回来的选择器。
fn checked_selector(selector: &str) -> Result<&str, String> {
    let selector = selector.trim();
    crate::run_method::validate_graft_selector(selector)?;
    Ok(selector)
}

/// The directory one selector's plan lives in, without reading the plan.
/// 某个选择器的计划所在目录，不读取计划本身。
///
/// Deleting must not require a parse. An unreadable record is exactly the one a
/// reader most needs to be able to remove, and while the delete path read first
/// it refused every broken record instead. Reading stays the job of
/// [`read_external_graft`]; this only resolves and checks the directory.
/// 删除不得以解析为前提。读不懂的记录恰恰是读者最需要能删掉的，而删除路径此前先读，于是
/// 拒绝了每一条坏记录。读仍是 [`read_external_graft`] 的职责；这里只解析并检查目录。
pub fn external_graft_directory(selector: &str) -> Result<PathBuf, String> {
    let selector = checked_selector(selector)?;
    let root = external_graft_root().join(selector);
    if !root.is_dir() {
        return Err(format!(
            "external graft `{selector}` does not exist at {}",
            root.display()
        ));
    }
    Ok(root)
}

/// Create an external graft plan without touching the base source tree.
/// 创建外部 graft 计划，不接触原树源码。
pub fn create_external_graft(
    registry: &Registry,
    target: NodeId,
    graft: impl Into<String>,
    full: bool,
) -> Result<ExternalGraftPlanFile, String> {
    let target_path = registry
        .path_for(target)
        .ok_or_else(|| format!("graft target `{target}` is not registered"))?;
    let selector = checked_selector(&graft.into())?.to_owned();

    let root = external_graft_root().join(&selector);
    if root.exists() {
        return Err(format!(
            "external graft `{selector}` already exists at {}",
            root.join(lexicon::GRAFT_PLAN_FILE).display()
        ));
    }
    let document = GraftPlanDocument::new(target, target_path, selector.clone(), full);
    fs::create_dir_all(&root)
        .map_err(|error| format!("cannot create external graft directory: {error}"))?;
    if let Err(error) = atomic_write(
        &root.join(lexicon::GRAFT_PLAN_FILE),
        &document.render_graft_plan_document(),
    ) {
        let _ = fs::remove_dir_all(&root);
        return Err(format!("cannot write external graft plan: {error}"));
    }

    Ok(ExternalGraftPlanFile {
        selector,
        document,
        root,
    })
}

/// Read one plan back, so an authoring surface can show and edit it.
/// 读回一条计划，供创作界面显示与编辑。
///
/// The read and parse go through the runtime loader, so Studio and a host refuse
/// exactly the same documents.
/// 读取与解析都走运行期加载器，因此 Studio 与宿主拒绝的文档完全相同。
pub fn read_external_graft(selector: &str) -> Result<ExternalGraftPlanFile, String> {
    let selector = checked_selector(selector)?;
    let document = load_graft_record(&package_root(), selector)?;
    Ok(ExternalGraftPlanFile {
        selector: selector.to_owned(),
        document,
        root: external_graft_root().join(selector),
    })
}

/// Every plan directory, including the ones that do not parse.
/// 列出全部计划目录，包括解析失败的。
pub fn list_external_grafts() -> Result<Vec<ExternalGraftPlanEntry>, String> {
    let root = external_graft_root();
    let loaded = load_graft_records(&package_root())?;
    Ok(loaded
        .into_iter()
        .map(|entry| match entry {
            LoadedGraft::Record(record) => ExternalGraftPlanEntry {
                root: root.join(&record.selector),
                selector: record.selector.clone(),
                document: Ok(record.document),
            },
            LoadedGraft::Unreadable { selector, reason } => ExternalGraftPlanEntry {
                root: root.join(&selector),
                selector,
                document: Err(reason),
            },
        })
        .collect())
}

/// Change whether a plan replaces the whole subtree.
/// 改变一条计划是否替换整棵子树。
pub fn rewrite_external_graft(selector: &str, full: bool) -> Result<ExternalGraftPlanFile, String> {
    let mut plan = read_external_graft(selector)?;
    if plan.document.full == full {
        return Ok(plan);
    }
    plan.document.full = full;
    let path = plan.plan_path();
    atomic_write(&path, &plan.document.render_graft_plan_document())?;
    Ok(plan)
}

/// Move one plan directory to the recoverable XiRang trash, returning its path.
/// 把一个计划目录移到可恢复的 XiRang 回收目录，并返回其路径。
///
/// The plan is not parsed: a record whose text is broken still has a directory,
/// and removing it is the repair. The trash keeps the bytes, so a removal that
/// turns out to be a mistake remains reversible by hand.
/// 计划不被解析：文本损坏的记录仍有目录，删掉它就是修复。回收目录保留字节，因此删错时仍可
/// 手工恢复。
pub fn remove_external_graft(selector: &str) -> Result<PathBuf, String> {
    let selector = checked_selector(selector)?;
    let root = external_graft_directory(selector)?;
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| format!("clock error: {error}"))?
        .as_nanos();
    let trash = package_root()
        .join(lexicon::XIRANG_DIR)
        .join("trash")
        .join(lexicon::EXTERNAL_GRAFT_DIR)
        .join(format!("{selector}-{stamp}"));
    fs::create_dir_all(trash.parent().expect("trash has a parent"))
        .map_err(|error| format!("cannot create XiRang trash: {error}"))?;
    fs::rename(&root, &trash).map_err(|error| {
        format!(
            "cannot move {} to {}: {error}",
            root.display(),
            trash.display()
        )
    })?;
    Ok(trash)
}

#[cfg(test)]
#[path = "plan_tests.rs"]
mod plan_tests;
