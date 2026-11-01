//! Graft records on disk: loading `.xirang/external-grafts/` and applying it.
//! 磁盘上的嫁接记录：读取 `.xirang/external-grafts/` 并应用。
//!
//! The kernel owns reconciliation
//! ([`Registry::overlay_recorded`](xirang_kernel::Registry::overlay_recorded)); this
//! page owns the filesystem boundary. It is deliberately **not**
//! feature-gated: a runtime host must not need the `authoring` feature (and its
//! `syn` dependency) merely to read a plan file, so the loader takes an explicit
//! `package_root` instead of the authoring thread-local context.
//! 内核拥有对账（[`Registry::overlay_recorded`](xirang_kernel::Registry::overlay_recorded)）；
//! 本页拥有文件系统边界。它刻意**不**受特性门控：运行期宿主读取计划文件不该需要
//! `authoring` 特性（及其 `syn` 依赖），因此加载器接收显式的 `package_root`，而不是
//! 创作期的线程局部上下文。
//!
//! One parser and one layout serve Studio and hosts: this module reuses
//! [`GraftPlanDocument::parse`] and the [`lexicon`] constants, and
//! `authoring::external_graft` delegates here.
//! 一个解析器、一套版式同时服务 Studio 与宿主：本模块复用
//! [`GraftPlanDocument::parse`] 与 [`lexicon`] 常量，`authoring::external_graft`
//! 委派到这里。

use std::fs;
use std::path::{Path, PathBuf};

use xirang_kernel::lexicon;

use crate::run_method::{GraftPlanDocument, Registry, StaticGraftCut};

pub use crate::run_method::registry_core::tree::graft_ops::{
    RecordReport, RecordedGraft, ResolvedRecord,
};

/// The directory that owns every external graft plan under `package_root`.
/// `package_root` 下拥有全部外部 graft 计划的目录。
///
/// `package_root` is a parameter, not a thread-local or an environment
/// variable, so a test and a host can point at a throwaway tree without
/// mutating process-global state.
/// `package_root` 是参数，而不是线程局部变量或环境变量，因此测试与宿主都能指向一棵
/// 一次性目录树，而不必改动进程级状态。
pub fn graft_record_root(package_root: &Path) -> PathBuf {
    package_root
        .join(lexicon::XIRANG_DIR)
        .join(lexicon::EXTERNAL_GRAFT_DIR)
}

/// One directory under `.xirang/external-grafts/`, read or broken.
/// `.xirang/external-grafts/` 下的一个目录，读得懂或坏了。
///
/// A broken plan is reported rather than hidden: the author has to see it, and
/// the other records still apply. The build treats the same file as advisory
/// (`build_method::graft_plan_check`), so the runtime does not let it abort.
/// 坏计划会被报告而不是藏起来：作者必须看见它，其余记录照常应用。构建把同一文件当作
/// 提示（`build_method::graft_plan_check`），因此运行期也不会因它中止。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LoadedGraft {
    /// A plan directory that parsed into a usable recorded graft.
    /// 解析成功、可用的已记录 graft 计划目录。
    Record(RecordedGraft),
    /// A plan directory that could not be read; the reason is reported.
    /// 无法读取的计划目录；原因随条目一同上报。
    Unreadable {
        /// The directory name this entry came from.
        /// 本条目来自的目录名。
        selector: String,
        /// Why the plan could not be read or parsed.
        /// 计划无法读取或解析的原因。
        reason: String,
    },
}

impl LoadedGraft {
    /// The directory name this entry came from.
    /// 本条目来自的目录名。
    pub fn selector(&self) -> &str {
        match self {
            Self::Record(record) => &record.selector,
            Self::Unreadable { selector, .. } => selector,
        }
    }
}

/// Every plan directory under `package_root`, readable or not, sorted by
/// selector.
/// `package_root` 下的每个计划目录，无论是否可读，按选择器排序。
///
/// A missing `.xirang/external-grafts/` is an empty list, not an error: most
/// packages have no records at all.
/// `.xirang/external-grafts/` 不存在时返回空列表而不是错误：大多数包根本没有记录。
pub fn load_graft_records(package_root: &Path) -> Result<Vec<LoadedGraft>, String> {
    let root = graft_record_root(package_root);
    let entries = match fs::read_dir(&root) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(format!("cannot scan {}: {error}", root.display())),
    };
    let mut loaded = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|error| format!("cannot scan {}: {error}", root.display()))?;
        if !entry.path().is_dir() {
            continue;
        }
        let selector = entry.file_name().to_string_lossy().into_owned();
        let path = entry.path().join(lexicon::GRAFT_PLAN_FILE);
        let document = fs::read_to_string(&path)
            .map_err(|error| format!("cannot read {}: {error}", path.display()))
            .and_then(|text| {
                GraftPlanDocument::parse_graft_plan_document(&text)
                    .map_err(|error| format!("{error}"))
            });
        match document {
            Ok(document) => {
                loaded.push(LoadedGraft::Record(RecordedGraft::new(selector, document)))
            }
            Err(reason) => loaded.push(LoadedGraft::Unreadable { selector, reason }),
        }
    }
    loaded.sort_by(|left, right| left.selector().cmp(right.selector()));
    Ok(loaded)
}

/// Read one plan back by its directory selector.
/// 按目录选择器读回一条计划。
///
/// The selector is validated before it is joined, so a record cannot address a
/// path outside `.xirang/external-grafts/`.
/// 选择器在拼接之前先校验，因此记录无法寻址 `.xirang/external-grafts/` 之外的路径。
pub fn load_graft_record(package_root: &Path, selector: &str) -> Result<GraftPlanDocument, String> {
    let selector = selector.trim();
    crate::run_method::registry_core::validate_graft_selector(selector)?;
    let path = graft_record_root(package_root)
        .join(selector)
        .join(lexicon::GRAFT_PLAN_FILE);
    let text = fs::read_to_string(&path)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    GraftPlanDocument::parse_graft_plan_document(&text)
        .map_err(|error| format!("{}: {error}", path.display()))
}

/// The effective tree record files produced, with their evidence.
/// 记录文件产生的有效树及其证据。
///
/// `reports` is the kernel's reconciliation evidence for the records that were
/// carried through. It is also printed by [`apply_recorded_grafts`], so a host
/// that never looks at this struct still sees every problem. Records the kernel
/// refuses, and plans this loader cannot parse, never reach here: they are an
/// `Err` instead of a field.
/// `reports` 是被带过来的那些记录在内核里的对账证据。它也会由
/// [`apply_recorded_grafts`] 打印，因此从不查看本结构体的宿主依然能看到每一个问题。
/// 内核拒绝的记录与本加载器解析不了的计划到不了这里：它们是 `Err`，而不是某个字段。
#[derive(Debug)]
pub struct GraftOverlay {
    /// The base tree with every applied record overlaid.
    /// 叠加全部已应用记录后的有效树。
    pub effective: Registry,
    /// The kernel's per-record reconciliation evidence, problems included.
    /// 内核逐条记录的对账证据，含问题项。
    pub reports: Vec<RecordReport>,
}

/// Whether one report describes a problem rather than a precedence decision.
/// 一条报告描述的是问题，还是一次优先级裁决。
///
/// The split is what makes the printed lines scannable without reading the
/// message text: these two mean a graft did **not** take effect, while the other
/// four are the documented precedence order working as intended. The defects
/// that have no legitimate reading at all — an unparseable plan, a directory
/// selector that disagrees with the plan's `graft` — never get this far; they
/// refuse the overlay outright.
/// 这条分界让打印出来的行不必读消息文本就能扫出问题：这两个意味着某次嫁接**没有**生效，
/// 另外四个则是记录在案的优先级顺序按设计生效。完全没有合法解读的缺陷——解析不了的
/// 计划、目录选择器与计划里的 `graft` 不一致——根本到不了这里：它们直接拒绝整次 overlay。
fn report_is_problem(report: &RecordReport) -> bool {
    matches!(
        report,
        RecordReport::UnkeptSlot { .. } | RecordReport::RecordSelectorUnresolved { .. }
    )
}

/// Render every report as one level-prefixed line.
/// 把每条报告渲染为一行，并带严重程度前缀。
///
/// Why a host cannot be left to print these itself: the obvious design returns
/// `reports` as data and documents "print them", but a host that only takes
/// `.effective` then applies **no** graft at all and sees nothing — and a graft
/// that silently never happens is worse than one that fails, because the source
/// tree says one thing and the running binary does another. So the two
/// "did not take effect" reports are `warning:`; the four precedence decisions
/// are `note:`.
/// 为什么不能把打印交给宿主自己：显而易见的做法是把 `reports` 当数据返回并注明
/// "打印它们"，但只取 `.effective` 的宿主会**完全没有**应用嫁接却什么都看不到——一次
/// 悄无声息从未发生的嫁接比一次失败更糟，因为源码说的是一回事、运行中的二进制是另一
/// 回事。因此两条"没有生效"的报告是 `warning:`，四条优先级裁决是 `note:`。
///
/// Boundary: this only renders; it does not decide whether to fail. A skipped
/// record stays non-fatal on purpose (`Registry::resolve_record` — one stale
/// record must not stop every other record), but it can no longer be invisible.
/// 边界：这里只渲染，不决定是否失败。被跳过的记录刻意保持非致命
/// （`Registry::resolve_record`——一条陈旧记录不该让其余记录全部失效），但它再也无法
/// 不可见。
///
/// Pinned by `graft_report_lines_mark_problems_as_warnings`.
/// 由 `graft_report_lines_mark_problems_as_warnings` 钉住。
pub(crate) fn graft_report_lines(reports: &[RecordReport]) -> Vec<String> {
    reports
        .iter()
        .map(|report| {
            let level = if report_is_problem(report) {
                "warning"
            } else {
                "note"
            };
            format!("{level}: {report}")
        })
        .collect()
}

/// Load `.xirang/external-grafts/` under `package_root` and overlay it.
/// 读取 `package_root` 下的 `.xirang/external-grafts/` 并覆盖。
///
/// `declared` is the build-captured static plan (`builtin_static_plan().grafts()`);
/// it is the arbiter of which slots stay alive. A record cannot resurrect a slot
/// no declaration hands over.
/// `declared` 是构建捕获的静态计划（`builtin_static_plan().grafts()`）；它才是哪些
/// 槽位存活的仲裁者。记录无法复活没有声明交出的槽位。
///
/// Three things fail this call instead of being carried as evidence, because none
/// of them has a legitimate reading: a plan file that does not parse, a record
/// directory that disagrees with the `graft` its plan names, and a record whose
/// identity and path name different faces. Everything else is **printed to
/// stderr** — one level-prefixed line per report — before this returns, and the
/// same items stay in the returned value for a host that routes evidence
/// elsewhere. The build refuses the unambiguous half of the same class even
/// earlier (`build_method::graft_plan_check`: a plan no declaration could ever
/// name); this covers a record added after the build, or one whose slot the base
/// tree no longer has.
/// 有三种情况会让本次调用失败而不是作为证据带出来，因为它们都没有合法解读：解析不了的
/// 计划文件、目录与该计划里的 `graft` 不一致的记录，以及身份与路径指向不同面的记录。
/// 其余情况都会在返回前**打印到 stderr**（每条报告一行，带严重程度前缀），同样的条目
/// 仍留在返回值里，供把证据转往别处的宿主使用。同一类问题里无歧义的那一半，构建拒绝得
/// 更早（`build_method::graft_plan_check`：没有任何声明可能命名的计划）；这里覆盖构建
/// 之后新增的记录，或槽位已不在原树里的记录。
///
/// `.xirang/external-grafts/` is **runtime input**, not generated state. A
/// record may re-route a string-form declaration, so a package that does not
/// review this directory can have shipped behavior changed by a machine-local
/// file; review it the way source is reviewed and keep it out of untrusted
/// checkouts. Because of that same power, `apply_recorded_grafts` and
/// `overlay_static` can produce different trees for identical inputs; the
/// example test `a_record_moves_the_effective_tree_but_not_the_static_plan`
/// pins that intended divergence rather than treating it as a bug.
/// `.xirang/external-grafts/` 是**运行期输入**而不是生成物。记录可以重新路由字符串
/// 形式声明，因此不审查该目录的包可能被机器本地文件改变已发布行为；请像审查源码一样
/// 审查它，并把它挡在不可信检出之外。也正因如此，`apply_recorded_grafts` 与
/// `overlay_static` 对相同输入可能产出不同的树；示例测试
/// `a_record_moves_the_effective_tree_but_not_the_static_plan` 把这处有意为之的偏离
/// 钉住，而不是当作缺陷。
pub fn apply_recorded_grafts(
    base: &Registry,
    external: &Registry,
    declared: &[StaticGraftCut],
    package_root: &Path,
) -> Result<GraftOverlay, String> {
    let loaded = load_graft_records(package_root)?;
    let mut records = Vec::new();
    let mut unreadable = Vec::new();
    for entry in loaded {
        match entry {
            LoadedGraft::Record(record) => records.push(record),
            LoadedGraft::Unreadable { selector, reason } => unreadable.push((selector, reason)),
        }
    }
    // All the unreadable plans at once, not the first: an author fixing a hand-
    // edited directory should see every broken file in one run, and a host that
    // wants to tolerate them calls `load_graft_records` (which still reports them
    // one by one) instead of this convenience entry point.
    // 一次报出全部不可读计划，而不是第一条：手工整理目录的作者应当一次看到所有坏文件，
    // 而想要容忍它们的宿主应改用 `load_graft_records`（它仍然逐条上报），而不是这个便利
    // 入口。
    if !unreadable.is_empty() {
        let details = unreadable
            .iter()
            .map(|(selector, reason)| format!("`{selector}`: {reason}"))
            .collect::<Vec<_>>()
            .join("; ");
        return Err(format!(
            "{} external graft plan(s) could not be read, so no record was applied: {details}",
            unreadable.len()
        ));
    }
    let outcome = base
        .overlay_recorded(&records, declared, external)
        .map_err(|error| error.to_string())?;
    // Printed here rather than left to the caller. The returned `reports` are
    // data a host can ignore, and ignoring `UnkeptSlot` means a graft that
    // silently never happens — the one failure mode the source tree cannot show.
    // 在这里打印，而不是留给调用方。返回的 `reports` 是宿主可以忽略的数据，而忽略
    // `UnkeptSlot` 意味着一次悄无声息从未发生的嫁接——这是源码树唯一无法体现的失败模式。
    for line in graft_report_lines(&outcome.reports) {
        eprintln!("{line}");
    }
    Ok(GraftOverlay {
        effective: outcome.effective,
        reports: outcome.reports,
    })
}

#[cfg(test)]
mod tests {
    use super::{RecordReport, graft_report_lines};
    use crate::run_method::registry_core::root_node_id;

    /// Problems and precedence decisions must be distinguishable before the
    /// message text is read, and a skipped record must say it was skipped.
    /// 问题与优先级裁决必须在读消息文本之前就能区分，且被跳过的记录必须说明自己被跳过。
    #[test]
    fn graft_report_lines_mark_problems_as_warnings() {
        let reports = vec![
            RecordReport::UnkeptSlot {
                selector: "canvas_graft".to_owned(),
                target_path: "root/canvas".to_owned(),
            },
            RecordReport::DeclarationOverridden {
                slot: root_node_id("report-lines"),
                declared: "canvas_fast".to_owned(),
                recorded: "canvas_graft".to_owned(),
            },
        ];
        let lines = graft_report_lines(&reports);

        assert_eq!(lines.len(), 2, "{lines:?}");
        // A skipped record says so in words a reader cannot miss.
        // 被跳过的记录用读者不可能漏掉的措辞说明这一点。
        assert!(lines[0].starts_with("warning: "), "{}", lines[0]);
        assert!(lines[0].contains("canvas_graft"), "{}", lines[0]);
        assert!(lines[0].contains("record skipped"), "{}", lines[0]);
        assert!(lines[1].starts_with("note: "), "{}", lines[1]);
    }
}
