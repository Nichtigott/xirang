//! Parent rewrites for every depth of the moved subtree, in package coordinates.
//! 被搬子树每一层的父路径改写，按包内坐标。

use std::path::Path;

use super::{FaceView, Moved, alias_for, plan_parent_edit, registry_owner, value_of};

pub(super) struct ChildEdit {
    /// Where the file is now, package-relative.
    /// 文件现在的位置，包内相对。
    pub(super) from_physical: String,
    /// Where it goes, package-relative; equal to `from_physical` for a logical-only descendant.
    /// 它要去的位置，包内相对；只发生逻辑搬动的后代两者相同。
    pub(super) to_physical: String,
    /// The child's logical path, for the report.
    /// 该子面的逻辑路径，供报告用。
    pub(super) path: String,
    /// The field before and after, rendered for the preview and syntax editor.
    /// 字段改前与改后的文本，供预览与语法编辑器使用。
    pub(super) before: String,
    pub(super) after: String,
    /// Both values, for the preview.
    /// 两个值，供预览用。
    pub(super) before_value: String,
    pub(super) after_value: String,
    /// Alias rewrite when the moved registry owner is renamed.
    /// 被搬的注册机拥有者改名时，同步改写别名。
    pub(super) alias: Option<String>,
}

/// Plan parent rewrites at every depth of the moved source subtree.
/// 为被搬源码子树的每一层规划父路径改写。
///
/// Planned **before** the rename, so a child whose file does not spell its parent exactly once refuses the
/// whole move before anything on disk changes. Rollback must restore both contents and paths.
/// 在改名**之前**规划，因此某个子面的文件拼不出恰好一次时，整次搬动会在磁盘上任何东西改变之前被拒绝——而
/// 回滚必须同时恢复文件内容和路径。
pub(super) fn plan_child_edits(
    layout: &crate::source_layout::SourceLayout,
    views: &[FaceView],
    face: &FaceView,
    moved: &Moved,
) -> Result<Vec<ChildEdit>, String> {
    let parents: std::collections::BTreeMap<_, _> = views
        .iter()
        .filter(|view| Path::new(&view.source).starts_with(&moved.from_dir))
        .map(|view| (view.id, view))
        .collect();
    let mut edits = Vec::new();
    for child in views.iter().filter(|view| view.id != face.id) {
        let Some(parent) = parents.get(&child.parent) else {
            continue;
        };
        let suffix = parent
            .module
            .strip_prefix(&face.module)
            .filter(|suffix| suffix.is_empty() || suffix.starts_with("::"))
            .ok_or_else(|| format!("cannot map moved parent module `{}`", parent.module))?;
        let spelling = format!("crate::{}{suffix}::NODE_ID", moved.module);
        // Two paths, and mixing them up is what the first version did: the plan runs **before** the
        // rename, so the child is read where it is now and written where it will be (measured:
        // `src/shelf/panel/object/knob/knob.rs is not readable` — the post-move path, asked for too
        // early).
        // 两条路径，而把两者弄混正是第一版干的事：规划跑在改名**之前**，因此子面要按它**现在**的位置读、
        // 按它将来的位置写（实测：`src/shelf/panel/object/knob/knob.rs is not readable`——把搬动之后的
        // 路径问得太早了）。
        // Both ends in **package** coordinates: a child that lives outside the moved directory (a
        // nested override) keeps its file and only its declaration's parent changes; deriving a path
        // from `to_dir` for it asked for `src/components/needle/...`, a file that never existed.
        // 两端都用**包**坐标：住在搬动目录之外的孩子（嵌套覆盖）文件不动，只有声明的父级改变；为它从
        // `to_dir` 推导路径，会去要 `src/components/needle/...`——一个从不存在的文件。
        let old_package = xirang_multilang::source_ref::package_relative(
            child.source_base,
            &layout.package_root,
            &layout.identity_base,
            &child.source,
        );
        let physically_moved = old_package == moved.from_package
            || old_package.starts_with(&format!("{}/", moved.from_package));
        let new_package = if physically_moved {
            let rest = old_package
                .strip_prefix(&format!("{}/", moved.from_package))
                .unwrap_or_default();
            if rest.is_empty() {
                moved.to_physical.clone()
            } else {
                format!("{}/{}", moved.to_package, rest)
            }
        } else {
            old_package.clone()
        };
        let here = layout.package_root.join(&old_package);
        let text = std::fs::read_to_string(&here)
            .map_err(|error| format!("{} is not readable: {error}", here.display()))?;
        let edit = plan_parent_edit(&text, &spelling).map_err(|error| {
            format!(
                "`{}` hangs under the face being moved, and {error}",
                child.path
            )
        })?;
        let before_alias = alias_for(views, Some(parent));
        let after_alias = format!("{}_object!", moved.module.rsplit("::").next().unwrap());
        let native = xirang_kernel::syntax::parse_face(&text)
            .ok()
            .flatten()
            .is_some_and(|face| face.is_native());
        let alias = if !native
            && registry_owner(views, Some(parent)).is_some_and(|owner| owner.id == face.id)
            && before_alias != after_alias
        {
            if !text.contains(&before_alias) {
                return Err(format!(
                    "`{}` does not spell expected alias `{before_alias}`",
                    child.path
                ));
            }
            Some(format!("{before_alias} → {after_alias}"))
        } else {
            None
        };
        edits.push(ChildEdit {
            from_physical: old_package,
            to_physical: new_package,
            path: child.path.clone(),
            before_value: value_of(&edit.before),
            after_value: value_of(&edit.after),
            before: edit.before,
            after: edit.after,
            alias,
        });
    }
    Ok(edits)
}
