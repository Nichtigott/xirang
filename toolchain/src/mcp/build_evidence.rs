//! The build's own evidence for one face, or for the tree it scoped.
//! 构建对某个面、或对它划定作用域的整棵树给出的证据。
//!
//! The read tools answer from source text; the build answers from
//! `target/xirang/out`. The two can disagree — a file written a moment ago is
//! not yet in the tree the host compiled — and until this module existed only the
//! CLI's `explain` could tell them apart. It reads the build's own files through
//! `build_method`'s readers, so an agent can ask "is this face in the shipped
//! scope, and does pruning strip it" without a second derivation of its own.
//! 读工具用源码文本作答，构建用 `target/xirang/out` 作答。两者可以不一致——刚写下的文件还不在
//! 宿主编译出的树里——而在这个模块出现之前，只有 CLI 的 `explain` 能把它们区分开。它经
//! `build_method` 的读取器读构建自己的文件，因此代理能直接问"这个面在发布作用域里吗、剪枝会不会
//! 剥掉它"，而不需要自己再做一份推导。
//!
//! What it deliberately does **not** report: declared graft state (`static_graft_plan!`
//! cuts stay in the `grafts` verb and the host entry), and contract/admission
//! fields (those live in built `RegistrationSnapshot`s and need a loaded registry).
//! 它有意**不**报告：声明的 graft 状态（`static_graft_plan!` 的切口仍归 `grafts` verb 与宿主
//! 入口），以及 contract/admission 字段（那些住在已构建的 `RegistrationSnapshot` 里，需要一个
//! 已加载的注册机）。

use std::cell::RefCell;
use std::path::{Path, PathBuf};

use crate::build_method::{
    BuildScopeView, FaceView, PruningRow, build_output_is_current, face_views, read_build_scope,
    read_pruning_manifest,
};
use serde_json::Value;

use crate::mcp::protocol::DEFAULT_LIMIT;
use crate::mcp::registry::namespace;
use crate::mcp::resolve::resolve_node;

thread_local! {
    /// The target directory this thread's current tool call named, if it named one.
    /// 本线程当前工具调用点名的 target 目录（若点名了）。
    ///
    /// Request-level, like [`crate::mcp::freshness`]'s policy, and set in the same single place: the
    /// alternative is every reader remembering to thread the argument down, which is how the two
    /// sides of this convention drift apart.
    /// 与 [`crate::mcp::freshness`] 的策略一样是**请求级**的，而且在同一个唯一位置设置：否则就得每个
    /// 读取方自己记得把这个参数传下去，而这条约定的两侧正是这样走散的。
    static TARGET_DIR: RefCell<Option<PathBuf>> = const { RefCell::new(None) };
}

/// Set (or clear) the target directory for this thread's current tool call.
/// 设置（或清除）本线程当前工具调用的 target 目录。
///
/// A **relative** value is kept as it came: it is resolved per package, exactly as `check` resolves
/// its own `target_dir` against the root it was called with. Resolving it once, here, would make
/// every member of a workspace share one evidence directory — and two packages' `source_scope.tsv`
/// would then overwrite each other.
/// **相对**值原样保留：它按包解析，与 `check` 把它自己的 `target_dir` 按调用时的根解析完全一致。在这里
/// 就解析掉，会让一个工作区的每个成员共用同一个证据目录——两个包的 `source_scope.tsv` 会互相覆盖。
pub(crate) fn set_target_dir(target_dir: Option<&str>) {
    let value = target_dir
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from);
    TARGET_DIR.with(|slot| *slot.borrow_mut() = value);
}

/// The directory the build publishes its evidence into.
/// 构建发布其证据的目录。
///
/// `<package>/target/xirang/out` by default — the same place the build script's evidence is copied to
/// and the bridge reads back. A caller who built with a `target_dir` names it here (and `check` names
/// it in the same request), so the reader follows the writer instead of reading an empty default and
/// silently answering `derived now` forever: that was the 2026-10-10 defect queue item ⑤, reproduced
/// on 2026-10-13 with the evidence sitting at `target/out-check-default` while `registry` — told
/// exactly where it was — still read `<tree>/target/xirang/out` (report §7h).
/// 默认是 `<包>/target/xirang/out`——构建脚本的证据被复制到那里、桥也从那里读回。用 `target_dir` 构建的
/// 调用方在这里点名它（`check` 在同一次请求里也点名它），于是读取方跟着写入方走，而不是读一个空的默认目录、
/// 永远静默地回答 `derived now`：这正是 2026-10-10 缺陷队列第 ⑤ 条，2026-10-13 复现时证据就躺在
/// `target/out-check-default`，而被明确告知位置的 `registry` 仍然去读 `<tree>/target/xirang/out`（报告 §7h）。
pub(crate) fn out_dir(root: &Path) -> PathBuf {
    TARGET_DIR.with(|slot| slot.borrow().clone()).map_or_else(
        || root.join("target/xirang/out"),
        |target_dir| root.join(target_dir).join("xirang/out"),
    )
}

/// The build's evidence for this package, read once for every caller that needs it.
/// 本包构建给出的证据，为每个需要它们的调用方读一次。
///
/// These readings used to be a tuple whose meaning lived in this file's prose, so
/// every caller had to remember their order — and `overlay` simply dropped the
/// third one. Named fields say what each reading answers instead.
/// 这些读数过去是一个元组，含义只住在本文件的散文里，因此每个调用方都得记住它们的顺序——而
/// `overlay` 干脆丢掉了第三个。现在由具名字段说出每个读数回答的是什么。
///
/// A missing or stale build is not an error: it is the answer to "why does this
/// face not know whether it ships".
/// 缺失或过期的构建不是错误：它正是"这个面为什么不知道自己发不发布"的答案。
pub(crate) struct BuildEvidence {
    /// Whether the published output still describes these sources.
    /// 已发布的产物是否仍在描述这批源码。
    pub(crate) current: bool,
    /// The scope the build selected, when `source_scope.tsv` was readable.
    /// 构建选中的作用域；`source_scope.tsv` 可读时才有。
    pub(crate) scope: Option<BuildScopeView>,
    /// The pruning manifest rows, when `pruning_manifest.tsv` was readable.
    /// 修剪清单行；`pruning_manifest.tsv` 可读时才有。
    pub(crate) pruning: Option<Vec<PruningRow>>,
}

impl BuildEvidence {
    /// The word for how fresh the published output is.
    /// 已发布产物新鲜度的那个词。
    ///
    /// `current` answers "does this output still describe these sources" and not
    /// "is the build healthy", and both reports below used to spell that answer
    /// themselves. The spelling lives here, with the field that answers it.
    /// `current` 回答的是"这份产物是否仍在描述这批源码"，而不是"构建健不健康"，而下面两份报告过去
    /// 各拼一次这个答案。拼法住在这里，与回答它的那个字段放在一起。
    pub(crate) fn freshness(&self) -> &'static str {
        if self.current {
            "current"
        } else {
            "stale (run `xirang check`)"
        }
    }
}

pub(crate) fn build_evidence(root: &Path) -> BuildEvidence {
    let out = out_dir(root);
    BuildEvidence {
        current: build_output_is_current(root, &out),
        scope: read_build_scope(&out).ok(),
        pruning: read_pruning_manifest(&out).ok(),
    }
}

/// Report the build's evidence for one face, or for the whole scoped tree.
/// 报告构建对某个面的证据，或对整棵被划定作用域的树给出的证据。
///
/// `node` names one face by logical path or identity; omitting it reports the tree
/// projection. `limit` bounds the projection's rows, because a tree report is the
/// one answer here that grows with the project.
/// `node` 用逻辑路径或身份点名一个面；省略它则报告树的投影。`limit` 限制投影的行数，因为树报告是
/// 这里唯一随项目变大的答案。
pub(crate) fn explain(root: &Path, arguments: &Value) -> Result<String, String> {
    let namespace = namespace(root)?;
    let faces = face_views(root, &namespace)?;
    // A missing or stale build is not an error: it is the answer to "why does
    // this face not know whether it ships".
    // 缺失或过期的构建不是错误：它正是"这个面为什么不知道自己发不发布"的答案。
    let evidence = build_evidence(root);
    let limit = arguments
        .get("limit")
        .and_then(Value::as_u64)
        .map_or(DEFAULT_LIMIT, |value| value.clamp(1, 200) as usize);
    match arguments.get("node").and_then(Value::as_str) {
        Some(target) => {
            let id = resolve_node(root, &namespace, target)?;
            let face = faces
                .iter()
                .find(|face| face.id == id)
                .ok_or_else(|| format!("no face in the derived tree has identity {id}"))?;
            Ok(node_report(
                &namespace,
                face,
                &evidence,
                &graft_line(root, face),
            ))
        }
        None => Ok(tree_report(&namespace, &faces, &evidence, limit)),
    }
}

/// What the host entry hands this slot to — and it says so even when the answer is "nothing".
/// 宿主入口把这个槽位交给了谁——即使答案是"没有"，它也会说出来。
///
/// The bridge's `explain` did not mention this at all, while the CLI's `explain --overlay` printed
/// `<- graft=… full=false (line N)` for the same slot: the agent-facing half of the same question was
/// missing, and an agent asking "who implements this face now" got a report that never said (audit
/// `M7`, §M7.62). The "none" line is deliberate and not noise: a view that stays silent when there is
/// nothing to hand over makes "nothing" indistinguishable from "this tool did not look", which is the
/// shape of the defect this line closes.
/// 桥的 `explain` 完全不提这件事，而 CLI 的 `explain --overlay` 为同一个槽位打印
/// `<- graft=… full=false (line N)`：同一个问题的**代理面**那一半是缺的，问"这个面现在由谁实现"的代理
/// 拿到一份通篇不说的报告（审计 `M7`，§M7.62）。那句"没有"是有意的、不是噪音：一个在没有东西可交时保持
/// 沉默的视图，会让"没有"与"这件工具根本没看"变得无法区分——而那正是本行要闭合的缺陷形状。
fn graft_line(root: &std::path::Path, face: &FaceView) -> String {
    match crate::build_method::declared_grafts(root) {
        Ok(declared) => match declared
            .cuts
            .iter()
            .find(|cut| cut.names_face(&face.path, Some(&face.module)))
        {
            Some(cut) => format!(
                "graft  <- {} full={} form={} (entry line {})\n",
                cut.graft,
                cut.full,
                cut.form(),
                cut.line
            ),
            None => {
                "graft  none — no declaration in the host entry hands this slot over\n".to_owned()
            }
        },
        Err(error) => format!("graft  unknown — the host entry could not be read: {error}\n"),
    }
}

/// One face, plus what the build says about its scope and pruning.
/// 一个面，外加构建对其作用域与剪枝的说法。
fn node_report(namespace: &str, face: &FaceView, evidence: &BuildEvidence, graft: &str) -> String {
    let mut output = format!(
        "namespace {namespace}\nnode {}\n  path {}\n  kind {}\n  source {}\n  module {}\n  parent {}{}\n  registry_name {}\n",
        face.id,
        face.path,
        face.kind,
        face.source,
        face.module,
        face.parent,
        if face.parent_resolved {
            ""
        } else {
            " (unresolved)"
        },
        face.registry_name,
    );
    output.push_str(graft);
    output.push_str(&format!("build {}\n", evidence.freshness()));
    output.push_str(&scope_line(evidence.scope.as_ref(), face));
    output.push_str(&pruning_line(evidence.pruning.as_deref(), face));
    output
}

/// Whether the scope selected this face, and why.
/// 作用域是否选中了这个面，以及原因。
pub(crate) fn scope_line(scope: Option<&BuildScopeView>, face: &FaceView) -> String {
    let Some(scope) = scope else {
        return "scope unknown — and for most trees that is the whole truth, not a missing file: **a tree that does not host xirang** (no `build.rs` calling `build_method::run()`, no `host!()` in its crate root) has no build-time scope to read, because there is no xirang build here at all. For a tree that **does** host it, the file is `<package>/target/xirang/out/source_scope.tsv` and the **CLI** `xirang check --root <tree>` writes it (`apply` starts one in the background and the one-shot client waits for it; `verify` builds on demand). Either way `registry` / `why --at` / `read {path, line}` answer without it\n".to_owned();
    };
    // T-28: the scope is built from the paths the **code** spells out (`cut(<expr>)` and friends),
    // never from what a file `use`s. The round measured an arm working this out by itself and
    // writing it into its answer — that is the signal a pointer belongs here, where the question
    // about scope is actually asked, rather than in a manual nobody opens mid-task.
    // T-28：作用域是**代码里拼出来的路径**（`cut(<表达式>)` 之类）构成的，从不来自某个文件 `use` 了什么。
    // 那一轮量到一臂自己摸索出这条规则并写进答案——这正是"该在这里给指路"的信号：问题在**这里**被问，
    // 而不是在那本人到中途不会翻的手册里。
    if scope.all {
        return format!(
            "scope selected (all=true, mode={}) — the scope is the set of paths the code spells out \
             (`cut(<expr>)` and friends), not what a file `use`s: a `use` alone never widens it\n",
            scope.mode
        );
    }
    if scope.selected_ids.contains(&face.id) {
        return format!("scope selected (by id, mode={})\n", scope.mode);
    }
    if scope.selected_sources.contains(&face.source) {
        return format!("scope selected (by source, mode={})\n", scope.mode);
    }
    // Not selected is the interesting answer: this face is discovered but the
    // build's scope does not include it, so it is a pruning candidate rather than
    // a missing face.
    // "未选中"才是有意思的答案：这个面被发现了，但构建的作用域不含它，因此它是剪枝候选而不是
    // 一个缺失的面。
    format!("scope not-selected (mode={})\n", scope.mode)
}

/// What release pruning does to this face's symbols.
/// 发布剪枝对这个面的符号做了什么。
///
/// A manifest row whose symbol is `-` records a face with nothing tracked, so
/// "a row exists" is not "something is stripped" — the symbol has to be named.
/// 符号为 `-` 的清单行记录的是一个没有可跟踪符号的面，因此"行存在"不等于"有东西被剥掉"——必须点名
/// 那个符号。
pub(crate) fn pruning_line(pruning: Option<&[PruningRow]>, face: &FaceView) -> String {
    let Some(rows) = pruning else {
        return "pruning unknown (no pruning_manifest.tsv; run `xirang check`)\n".to_owned();
    };
    let tracked: Vec<&str> = rows
        .iter()
        .filter(|row| row.id == face.id)
        .map(|row| row.symbol.as_str())
        .filter(|symbol| *symbol != "-")
        .collect();
    if !tracked.is_empty() {
        return format!("pruning strips {}\n", tracked.join(", "));
    }
    if rows.iter().any(|row| row.id == face.id) {
        return "pruning nothing to strip (the face has no tracked symbol)\n".to_owned();
    }
    "pruning nothing to strip\n".to_owned()
}

/// The whole tree the build saw, bounded.
/// 构建看到的那整棵树，有上限。
fn tree_report(
    namespace: &str,
    faces: &[FaceView],
    evidence: &BuildEvidence,
    limit: usize,
) -> String {
    let mut output = format!("namespace {namespace}\nfaces {}\n", faces.len());
    output.push_str(&format!("build {}\n", evidence.freshness()));
    match evidence.scope.as_ref() {
        Some(scope) => output.push_str(&format!(
            "scope mode={} all={} reason={} selected_ids={} selected_sources={}\n",
            scope.mode,
            scope.all,
            scope.reason.as_deref().unwrap_or("-"),
            scope.selected_ids.len(),
            scope.selected_sources.len(),
        )),
        None => output.push_str("scope unknown — and for most trees that is the whole truth, not a missing file: **a tree that does not host xirang** (no `build.rs` calling `build_method::run()`, no `host!()` in its crate root) has no build-time scope to read, because there is no xirang build here at all. For a tree that **does** host it, the file is `<package>/target/xirang/out/source_scope.tsv` and the **CLI** `xirang check --root <tree>` writes it (`apply` starts one in the background and the one-shot client waits for it; `verify` builds on demand). Either way `registry` / `why --at` / `read {path, line}` answer without it\n"),
    }
    // Rows carry the scope verdict each, so the projection answers "what ships"
    // per face rather than only in aggregate. The header names what a row is — a
    // face, by logical path, `kind`, scope verdict and source — because `slot`
    // belongs to the overlay's graft cuts and to the plugin host's plugin slots,
    // and a reader searching for `slot` must not land here.
    // 每一行都带自己的作用域结论，因此这个投影逐个面地回答"什么会发布"，而不只是给个总数。表头
    // 说出每一行是什么——一个面，按逻辑路径、`kind`、作用域结论与源码——因为 `slot` 属于叠加层的
    // graft 切口与插件宿主的插件槽，按 `slot` 检索的读者不该落到这里。
    output.push_str("faces:\n");
    for face in faces.iter().take(limit) {
        let selected = evidence.scope.as_ref().map_or("unknown", |scope| {
            if scope.all
                || scope.selected_ids.contains(&face.id)
                || scope.selected_sources.contains(&face.source)
            {
                "selected"
            } else {
                "not-selected"
            }
        });
        output.push_str(&format!(
            "  {:<40} {:<14} {:<10} {}\n",
            face.path, face.kind, selected, face.source
        ));
    }
    if faces.len() > limit {
        output.push_str(&format!(
            "  {}\n",
            crate::mcp::truncation::withheld(
                faces.len() - limit,
                faces.len(),
                limit,
                "faces",
                "raise `limit`"
            )
        ));
    }
    match evidence.pruning.as_deref() {
        Some(rows) => {
            output.push_str(&format!("pruned {}\n", rows.len()));
            for row in rows.iter().take(limit) {
                output.push_str(&format!("  {} {} {}\n", row.id, row.source, row.symbol));
            }
            if rows.len() > limit {
                output.push_str(&format!(
                    "  {}\n",
                    crate::mcp::truncation::withheld(
                        rows.len() - limit,
                        rows.len(),
                        limit,
                        "pruned rows",
                        "raise `limit`"
                    )
                ));
            }
        }
        None => output.push_str("pruned unknown (no pruning_manifest.tsv; run `xirang check`)\n"),
    }
    output
}

#[cfg(test)]
#[path = "build_evidence_tests.rs"]
mod build_evidence_tests;
