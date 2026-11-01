//! The static overlay projection: which slot a declared cut replaces, and which
//! the build's scope prunes.
//! 静态覆盖投影：哪个槽位被已声明切口替换，哪个被构建的作用域剪掉。
//!
//! An *overlay* is what the host's two live registries produce at run time, and
//! the CLI cannot link either of them — the base and external trees are built
//! inside the host crate by its generated `registrations()` and
//! `external_object!` declarations. What a command *can* read is what the overlay
//! is computed from: the faces the build discovered, the scope it selected, and
//! the cuts its entry declared. This module turns those three into per-slot rows,
//! so the CLI's `explain --overlay` and the MCP bridge's
//! `xirang.explain {"overlay": true}` report one traversal instead of two that
//! drift. The rule moved out of the CLI in this release for the same reason
//! `graft_plan_rows` did: a second copy is how two surfaces start disagreeing
//! about which slot is replaced.
//! *覆盖*是宿主两棵活的注册树在运行期产生的东西，而 CLI 一棵都链接不到——基树与外部树是在
//! 宿主 crate 内由其生成的 `registrations()` 与 `external_object!` 声明构建的。命令**能**读到
//! 的是覆盖由之算出的东西：构建发现的面、它选中的作用域，以及宿主入口声明的切口。本模块把这三样
//! 变成逐槽位的行，因此 CLI 的 `explain --overlay` 与 MCP 桥的 `xirang.explain {"overlay": true}`
//! 报告的是一次遍历，而不是两份会漂移的副本。这条规则与 `graft_plan_rows` 出于同样的理由搬出
//! CLI：第二份副本正是两个执行面开始就"哪个槽位被替换"产生分歧的方式。

use std::path::Path;

use xirang_kernel::identity::NodeId;

use crate::build_method::face_view::{BuildScopeView, FaceView};
use crate::build_method::graft_view::{
    DeclaredGraft, DeclaredGrafts, GraftPlanRow, graft_plan_rows,
};

/// The note every overlay projection carries, whichever surface renders it.
/// 每一次覆盖投影（无论哪个执行面渲染）都携带的说明。
///
/// It says what the projection is *not*: the tree an overlay actually computes
/// needs two live registries, and only a host that links both of them can dump
/// one. One text for both surfaces keeps the CLI's JSON document and the
/// bridge's answer from describing the same projection two different ways.
/// 它说明这份投影**不是**什么：覆盖真正算出的树需要两棵活的注册树，只有同时链接两者的宿主才能
/// 把它 dump 出来。两个执行面共用一段文本，CLI 的 JSON 文档与桥的回答因此不会对同一份投影给出
/// 两种说法。
pub const OVERLAY_NOTE: &str = "static projection of the build's scope and declared cuts; the live effective tree is `Registry::dump_effective` (overlay_static + dump) inside a host that links both registries";

/// One face's row in the static overlay projection.
/// 静态覆盖投影中一个面的行。
///
/// `selected` is "the scope named this face as a root"; `kept` is "the published
/// tree still contains it", which is also true for an ancestor a selected face
/// needs. Both are `None` when the build published no scope: an unknown verdict
/// is not `false`, because calling a surviving face pruned would misreport the
/// tree. `replacement` is the declared cut that hands this slot over, if any.
/// `selected` 表示"作用域把该面点名为根"；`kept` 表示"发布树仍然包含它"，被选中面所需的
/// 祖先同样成立。构建没有发布作用域时两者都是 `None`：未知的结论不是 `false`，因为把一个仍然
/// 存活的面说成被剪掉就是在错误描述这棵树。`replacement` 是交出该槽位的已声明切口（若有）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OverlaySlot {
    /// The face's source-instance identity.
    /// 该面的源码实例身份。
    pub id: NodeId,
    /// The logical registry path a runtime tree would report.
    /// 运行期树会报告的逻辑注册路径。
    pub path: String,
    /// The declared `kind`.
    /// 声明的 `kind`。
    pub kind: String,
    /// Source path relative to the package's `src/`.
    /// 相对包 `src/` 的源码路径。
    pub source: String,
    /// The Rust module path the source declares.
    /// 源码声明的 Rust 模块路径。
    pub module: String,
    /// Whether the scope named this face, when a scope was published.
    /// 作用域是否点名了该面（发布了作用域时）。
    pub selected: Option<bool>,
    /// Whether the published tree contains this face.
    /// 发布树是否包含该面。
    pub kept: Option<bool>,
    /// The declared cut that replaces this slot, when one does.
    /// 替换该槽位的已声明切口（若有）。
    pub replacement: Option<DeclaredGraft>,
}

/// The whole static projection: the slots that ship, the ones pruning drops, and
/// the plan records the entry does or does not keep.
/// 整份静态投影：会发布的槽位、被剪枝丢掉的槽位，以及宿主入口（不）保住计划记录。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OverlayProjection {
    /// Slots the published tree keeps, replaced ones included.
    /// 发布树保住的槽位，包含被替换的那些。
    pub slots: Vec<OverlaySlot>,
    /// Slots the scope does not keep; `kept == Some(false)` is the only thing
    /// that lands here.
    /// 作用域不保住的槽位；只有 `kept == Some(false)` 会落在这里。
    pub pruned: Vec<OverlaySlot>,
    /// Every external graft plan, joined with the declaration that keeps it.
    /// 每条外部 graft 计划，与保住它的声明对照。
    pub plans: Vec<GraftPlanRow>,
}

/// One slot-to-implementation binding from the build's normalized overlay facts.
/// 来自构建规范化 overlay 事实的一条“槽位→实现”绑定。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SlotBinding {
    /// The slot's own identity. A replacement never overwrites it.
    /// 槽位自身的身份。替换件绝不覆盖它。
    pub slot: NodeId,
    /// The slot's logical path.
    /// 槽位的逻辑路径。
    pub path: String,
    /// The replacement expression or selector the plan names (display spelling).
    /// 计划点名的替换表达式或选择器（展示拼写）。
    pub implementation: String,
    /// The replacement's resolved identity, when an existing plan/declaration supplied one.
    /// This is deliberately separate from the display spelling; `None` means the record does not
    /// prove an implementation NodeId.
    /// 替换件已解析身份（仅当现有计划/声明提供了它）。它有意与展示拼写分开；`None` 表示这条记录
    /// 不能证明实现 NodeId。
    pub implementation_id: Option<NodeId>,
}

/// The slot-to-implementation bindings implied by the same projection the CLI/MCP read.
/// CLI/MCP 读取的同一份投影所蕴含的槽位→实现绑定。
pub fn slot_bindings(
    package_root: &Path,
    faces: &[FaceView],
    declared: Option<&DeclaredGrafts>,
) -> Result<Vec<SlotBinding>, String> {
    Ok(overlay_projection(package_root, faces, None, declared)?
        .slots
        .into_iter()
        .filter_map(|slot| {
            slot.replacement.map(|replacement| SlotBinding {
                slot: slot.id,
                path: slot.path,
                implementation: replacement.graft,
                // The authoring overlay carries the declared spelling, not a resolved external
                // NodeId; claiming one here would be a new parsing rule.
                implementation_id: None,
            })
        })
        .collect())
}

/// Project the build's own overlay inputs into per-slot rows.
/// 把构建自己的覆盖输入投影成逐槽位的行。
///
/// `scope` is `None` when no current scope was published, and `declared` is
/// `None` when the host entry could not be read; both are *answers* rather than
/// errors here, and each row says so instead of guessing. A plans directory that
/// exists and cannot be read is the one hard failure, because the projection then
/// cannot say whether a record is kept — that comes from `graft_plan_rows`.
/// 没有发布当前作用域时 `scope` 为 `None`，宿主入口读不了时 `declared` 为 `None`；两者在这里
/// 都是**答案**而不是错误，每一行会如实说明而不是猜。存在却读不了的计划目录是唯一的硬失败，
/// 因为此时投影无法说明某条记录是否被保住——那来自 `graft_plan_rows`。
pub fn overlay_projection(
    package_root: &Path,
    faces: &[FaceView],
    scope: Option<&BuildScopeView>,
    declared: Option<&DeclaredGrafts>,
) -> Result<OverlayProjection, String> {
    // The plans are read here rather than by each caller, so the CLI and the
    // bridge cannot pair the projection with a differently-derived plan list.
    // 计划在这里读取而不是由每个调用方各自读取，因此 CLI 与桥不可能把投影与另一份推导出来的
    // 计划清单配在一起。
    let plans = graft_plan_rows(package_root, faces, declared)?;
    let mut slots = Vec::new();
    let mut pruned = Vec::new();
    for face in faces {
        let (selected, kept) = match scope {
            Some(scope) => {
                let selected = scope.all
                    || scope.selected_sources.contains(&face.source)
                    || scope.selected_ids.contains(&face.id);
                (Some(selected), Some(selected || scope.keeps(&face.module)))
            }
            None => (None, None),
        };
        let replacement = declared
            .and_then(|declared| {
                declared
                    .cuts
                    .iter()
                    .find(|cut| cut.names_face(&face.path, Some(&face.module)))
            })
            .cloned();
        let slot = OverlaySlot {
            id: face.id,
            path: face.path.clone(),
            kind: face.kind.clone(),
            source: face.source.clone(),
            module: face.module.clone(),
            selected,
            kept,
            replacement,
        };
        if kept == Some(false) {
            pruned.push(slot);
        } else {
            slots.push(slot);
        }
    }
    Ok(OverlayProjection {
        slots,
        pruned,
        plans,
    })
}

#[cfg(test)]
#[path = "overlay_rows_tests.rs"]
mod overlay_rows_tests;
