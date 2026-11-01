//! What a write request's refusals say, and how they are instantiated.
//! 写入请求的拒绝说什么，以及它们是怎么被实例化的。
//!
//! Split out of `apply.rs` for the 600-code-line ratchet, and it is the half that is **about the
//! reader**: the editable-field list a refusal prints, the complete request built from this tree's
//! own names (audit `W4-4`), and the append/rewrite boundary the executor's wall repeats. The
//! actions themselves stay in the parent — this module never writes anything.
//! 从 `apply.rs` 拆出来是为了 600 代码行棘轮，而它正好是**关于读者**的那一半：拒绝里印的可编辑字段清单、
//! 用这棵树自己的名字搭出来的完整请求（审计 `W4-4`）、以及执行器那堵墙会重复一遍的追加/改写分界。动作本身
//! 留在父模块——这个模块什么都不写。
//!
//! online: a refusal has to be built from the names in this tree as it stands now, and a tree that changed since the build has faces the record has never seen.

use std::path::Path;

use serde_json::json;

use crate::build_method::{declared_grafts, face_views, graft_plan_rows};

use super::apply_cut;
use super::{Action, EDITABLE_FIELDS, Outcome};

/// The editable fields, as a request must spell them: the shape the refusals point at and every
/// successful reply repeats. `--list <tool>` carries the same list, but the round measured that
/// nothing calls it (zero uses in seventeen subjects), so the shape travels with the answer instead.
/// 可编辑字段，按请求必须写出的形状：拒绝文案指向它，每次成功的回复也重复它。`--list <tool>` 载着同一
/// 份清单，但那轮量到**没有任何一次调用**去读它（17 个样本里 0 次），因此形状随答案一起走。
pub(super) fn editable_fields_line() -> String {
    format!(
        "fields: {} — every value is a string; `needs_registry` is the one boolean, and \
         `exports`/`handle_traits`/`requires` are spelled as strings rather than arrays",
        EDITABLE_FIELDS.join(" ")
    )
}

/// What this change will make the rest of the tree say, as static facts with their boundary.
/// 这次改动会让这棵树的其他部分说什么——按静态事实给出，并带上它们的边界。
///
/// Both families of the round-5 evaluation spent their most expensive iterations discovering this by
/// experiment: a new face moves the factory enumerations, and "the application ships this face" is a
/// different question from "the build discovered it". Both facts are already in the tree, so the
/// write path states them before the write instead of leaving them to be rediscovered by a red run.
/// 第五轮评测的两个族都把最贵的迭代花在"靠实验发现这件事"上：新面会移动出厂形状的枚举，而"这个应用
/// 是否发布这个面"与"构建发现了它"是两个问题。两件事本来就写在这棵树里，因此写入路径在写之前把它们
/// 说出来，而不是留给一次红运行去重新发现。
pub(super) fn consequences(
    work: &Path,
    project: &Path,
    namespace: &str,
    outcome: &Outcome,
) -> Result<String, String> {
    let changed = outcome
        .source
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    if changed.is_empty() {
        return Ok(String::new());
    }
    let faces = face_views(work, namespace)?;
    let Some(face) = faces.iter().find(|face| face.source.ends_with(&changed)) else {
        return Ok(String::new());
    };
    let leaf = face.path.rsplit('/').next().unwrap_or_default().to_owned();
    let tokens = [face.registry_name.clone(), face.kind.clone(), leaf];
    let sources = crate::mcp::source_index::load_sources(project)?;
    const PINS: usize = 5;
    let mut pins: Vec<String> = Vec::new();
    let mut shipped_by_a_cut = false;
    // "A cut ships this face" and the two site counts are read from **code**: a comment that merely
    // mentions `cut(…)` is not a declaration (2026-10-10 defect queue, report §7h — the same prose
    // comment that moved the census column by two moved these counts as well). The pin line still
    // shows the reader the file's own text, so the mask is used for the decision and not for display.
    // "有切口在发布这个面"与两个站点计数读的都是**代码**：一条只是提到 `cut(…)` 的注释不是声明
    // （2026-10-10 缺陷队列，报告 §7h——同一条散文注释把总账那一栏挪了两格，也把这两个计数挪了）。
    // 钉子行仍然给读者看文件自己的文本，因此掩码只用于判定，不用于展示。
    for file in &sources {
        let code = xirang_kernel::source::mask_non_code(&file.source);
        let test = crate::mcp::callgraph::looks_like_a_test(&file.relative, &file.source);
        for (index, (line, code_line)) in file.source.lines().zip(code.lines()).enumerate() {
            let names_it = tokens
                .iter()
                .any(|token| !token.is_empty() && line.contains(token.as_str()));
            if !names_it {
                continue;
            }
            if test {
                let text = line.trim();
                let kept: String = text.chars().take(96).collect();
                pins.push(format!("  {}:{}  {kept}", file.relative, index + 1));
            }
            if code_line.contains("cut(") || code_line.contains("graft(") {
                shipped_by_a_cut = true;
            }
        }
    }
    let cuts = sources
        .iter()
        .map(|file| {
            xirang_kernel::source::mask_non_code(&file.source)
                .matches("cut(")
                .count()
        })
        .sum::<usize>();
    let grafts = sources
        .iter()
        .map(|file| {
            xirang_kernel::source::mask_non_code(&file.source)
                .matches("graft(")
                .count()
        })
        .sum::<usize>();
    let mut report = format!(
        "consequences (static, text-level): {} in-tree test line(s) name this face\n",
        pins.len()
    );
    for line in pins.iter().take(PINS) {
        report.push_str(line);
        report.push('\n');
    }
    if pins.len() > PINS {
        report.push_str(&format!(
            "{}\n",
            crate::mcp::truncation::withheld(
                pins.len() - PINS,
                pins.len(),
                PINS,
                "pin lines",
                "grep the face's name in test files"
            )
        ));
    }
    report.push_str(&format!(
        "  entry plan: {cuts} `cut(` and {grafts} `graft(` site(s); this face's name appears at {} \
         of them — whether the application ships it is the plan's own business\n",
        if shipped_by_a_cut { "one" } else { "none" }
    ));
    // Audit `W2-5` (D3): the two lines above are a **constant** block — worth reading once. From the
    // second write in the same session the disclaimer is one line pointing at the first, the same
    // rule `consistency`'s bounds follow (`session::first_time`, keyed by root).
    // 审计 `W2-5`（D3）：上面两行是**常量**块——值得读一次。同一会话里的第二次写入起，免责声明就是一行
    // 指向第一次的回指，与 `consistency` 的边界同一条规则（`session::first_time`，键带根）。
    let first =
        crate::mcp::session::first_time(&format!("apply-consequences:{}", project.display()));
    report.push_str(if first {
        "  not covered: this lists test lines that spell the face's name; a test that counts faces \
         without naming it, or reaches it through another spelling, does not appear here — run the \
         suite before believing either list\n"
    } else {
        "  not covered: unchanged from this session's earlier write (`--list apply` has the full \
         text)\n"
    });
    Ok(report)
}

/// A complete, executable request for one write action, built from **this tree's own** names.
/// 某个写入动作的完整、可执行请求，用的是**这棵树自己的**名字。
///
/// Audit `W4-4` (measured across S1/S6/S7): every refusal already named the shape, but in
/// placeholders — `<node>`, `<snake_case>`, `<Kind>` — so a reader still had to find a legal parent,
/// invent a module name that is free, and work out the typed spelling of a cut before the refusal
/// became a call. This builds the example out of what the tree already says: a real `parent` (the
/// first face that owns a registry), a `module` name nothing in the tree uses, and for a cut the
/// `crate::…::NODE_ID` spelling the build itself derives (`FaceView::module`) plus the `graft`
/// spelling the plan already carries when it carries one.
/// 审计 `W4-4`（在 S1/S6/S7 上量到）：过去的每条拒绝都点了形状，但用的是占位符——`<node>`、
/// `<snake_case>`、`<Kind>`——于是读者还得自己找一个合法父面、想一个不撞的模块名、把切口的类型化拼写
/// 推出来，这条拒绝才变成一次调用。这里用树本来就说出来的东西搭出示例：真实的 `parent`（第一个拥有注册机
/// 的面）、一个树里没人用的 `module` 名，切口则用构建自己推导的 `crate::…::NODE_ID`（`FaceView::module`）
/// 加上计划本来就带着的那个 `graft` 拼写（有的话）。
///
/// It is a **hint, not a promise**: the executor still decides, and the example is only as good as
/// the tree it was read from — a tree with no faces gets the placeholders back.
/// 它是**提示，不是承诺**：仍然由执行器决定，而这个示例的好坏取决于它读的那棵树——没有面的树拿回占位符。
pub(super) fn write_example(root: &Path, namespace: &str, action: Action) -> String {
    let views = face_views(root, namespace).unwrap_or_default();
    let leaf = views
        .iter()
        .find(|view| !view.owns_registry)
        .or_else(|| views.first());
    let parent = views
        .iter()
        .find(|view| view.owns_registry)
        .or(leaf)
        .map(|view| view.path.clone())
        .unwrap_or_else(|| "root".to_owned());
    let node = leaf
        .map(|view| view.path.clone())
        .unwrap_or_else(|| "root".to_owned());
    let module = free_module(&views);
    let kind = {
        let mut chars = module.chars();
        match chars.next() {
            Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
            None => String::new(),
        }
    };
    let cut = leaf
        .map(|view| format!("crate::{}::NODE_ID", view.module))
        .unwrap_or_else(|| "crate::<the face's module>::NODE_ID".to_owned());
    let graft = apply_cut::plan_graft_example(root)
        .unwrap_or_else(|| "<the replacing crate>::<face>_fast::NODE_ID".to_owned());
    let request = match action {
        Action::Add => json!({
            "action": "add",
            "parent": parent,
            "fields": {"module": module, "kind": kind},
            "apply": true
        }),
        Action::Edit => json!({
            "action": "edit",
            "node": node,
            "fields": {"stable_name": "probe"},
            "apply": true
        }),
        Action::Rename => json!({
            "action": "rename",
            "node": node,
            "fields": {"module": free_module(&views)},
            "apply": true
        }),
        Action::Delete => json!({
            "action": "delete",
            "node": node,
            "apply": true,
            "confirm": true
        }),
        Action::Deepen => json!({
            "action": "deepen",
            "node": node,
            "inside": {"parts": {"size": "u32", "label": "String"}},
            "apply": true
        }),
        Action::Cut => json!({"action": "cut", "cut": cut, "graft": graft, "apply": true}),
        Action::Promote => json!({
            "action": "promote",
            "selector": "<.xirang/external-grafts/<selector>>",
            "apply": true,
            "confirm": true
        }),
        // `move` names a face and the parent it should hang under; the face it offers is a leaf that
        // does not own a registry, and the parent is a real registry owner from this tree.
        // `move` 点一个面以及它该挂到哪个父级之下；它给出的面是一个不拥有注册机的叶子，父级则是这棵树里
        // 真实的注册机拥有者。
        Action::Move => json!({
            "action": "move",
            "node": leaf.map(|view| view.path.clone()).unwrap_or_else(|| "root".to_owned()),
            "to": parent,
            "apply": true
        }),
    };
    format!("accepted shape  {request}")
}

/// A bare snake_case module name nothing in this tree uses yet.
/// 一个这棵树还没用过的裸 snake_case 模块名。
pub(super) fn free_module(views: &[crate::build_method::FaceView]) -> String {
    for candidate in ["widget", "widget2", "widget3"] {
        let taken = views.iter().any(|view| {
            view.module.rsplit("::").next() == Some(candidate)
                || view.path.rsplit('/').next() == Some(candidate)
                || view.registry_name == candidate
        });
        if !taken {
            return candidate.to_owned();
        }
    }
    "widget".to_owned()
}

/// An executor refusal, plus the way forward when the wall is one the round measured.
/// 执行器的拒绝；当这堵墙是那轮量到的那一堵时，附上继续走的两条路。
///
/// Two walls cost the evaluation real time, and both are decidable from the message itself: a face
/// cannot take children until it declares a registry of its own, and a face that owns children
/// cannot stay behind a plain cut. Naming the two ways forward is not a licence to skip the rule —
/// the executor still refuses — it is the difference between "no" and "no, and here is what yes
/// needs".
/// 有两堵墙花了评测的真实时间，而两堵都能从消息本身判定：一个面在声明自己的注册机之前不能接收子级；
/// 而一个面一旦拥有子级，它的槽位就不能再是普通切口。点出两条路不是绕过规则的许可证——执行器照样
/// 拒绝——它是"不行"与"不行，而'行'需要什么"之间的差别。
pub(super) fn refused_with_a_way_forward(error: String) -> String {
    // The kernel spells it `does not own a Registry`; the match is case-folded so a wording change in
    // its capitalisation cannot silently drop the way forward.
    // 内核把它写成 `does not own a Registry`；这里按大小写折叠来匹配，免得它改了首字母就把"继续走的
    // 路"悄悄丢掉。
    let folded = error.to_lowercase();
    if folded.contains("does not own a registry") {
        return format!(
            "{error}\nway forward: either declare `needs_registry: true` on that parent together \
             with its own `registration_rule` (then a child can hang under it), or attach this face \
             to a parent that already owns a registry — both are changes to declarations, and the \
             kernel's rule stays as it is"
        );
    }
    if folded.contains("non-empty child registry") {
        return format!(
            "{error}\nway forward: the slot whose face now owns children has to be declared as a \
             full replacement (`cut(…) full graft(…)`) rather than a plain cut, or those children \
             have to move out from under it — the factory assertions move with either decision, and \
             that is the decision, not a workaround"
        );
    }
    // The append/rewrite boundary, said where it is hit. `add`, `deepen`, `cut` and `promote` are
    // additive: they hand a face over or hang something under it, and each is its own declaration
    // change, so a hand-written face is a legal subject. `edit`, `rename` and `delete` rewrite a
    // file, and the executor only rewrites what it generated — a hand-written face is authorship,
    // and taking one over is a different decision rather than a spelling fix.
    // 追加/改写的分界，就在被撞到的地方说出来。`add`、`deepen`、`cut`、`promote` 是**追加**类：它们把
    // 一个面交出去或在它下面挂东西，各自都是一次声明改动，因此手写面是合法主体。`edit`、`rename`、
    // `delete` 改写文件，而执行器只改写它生成过的东西——手写面是作者身份，收编它是另一个决定，不是
    // 拼写修正。
    if folded.contains("was not generated by xirang") {
        // Audit `S7`: the refusal has to hand back a **route**, not just a boundary. The chain that
        // measured this one hit the refusal for a hand-written face's `rename` and left without a
        // next move — the same shape as the tree-root refusal that got fixed by naming the root.
        // 审计 `S7`：拒绝必须交回一条**路**，而不只是划一条界。量到这一处的那个链，在一个手写面的
        // `rename` 上撞到拒绝之后没有下一步——与"点名树根"那次修好的拒绝是同一种形状。
        return format!(
            "{error}\nboundary: this tool rewrites only faces it generated. `add`/`deepen`/`cut`/\
             `promote` are additive — they change declarations and reach an existing hand-written \
             face fine — while `edit`/`rename`/`delete` rewrite the file, so a hand-written face is \
             refused here; taking one over is a separate, explicit adoption, not a spelling these \
             actions accept.\nway forward: if you only need a **declaration** to change (a layer \
             inside it, an entry under it, its parent), use the additive action and this file is a \
             legal subject. If you need the **file** renamed or rewritten, do it in the editor: the \
             module name is the directory and file name, so the rename is that pair plus every plan \
             entry that names the old path — `affected {{files: [\"<the face's source>\"]}}` lists \
             them before you start"
        );
    }
    error
}

#[cfg(test)]
#[path = "apply_refusals_tests.rs"]
pub(crate) mod apply_refusals_tests;

/// What a change does to the tree's **consumers**, computed from the tree (audit `W5-3`).
/// 一次改动对树的**消费方**做了什么，从树上算出（审计 `W5-3`）。
///
/// "Consumers" here are the two things that read this face from outside its own file: the **graft plan
/// entries** that target it or its subtree, and whatever fills the slots its parts layer provides. A
/// preview that showed the new file but not the consumers would be showing half the change — and the
/// half it hid is the one a caller cannot see by reading the file it is about to write.
/// 这里的"消费方"是从这个面自己的文件之外读它的两样东西：针对它或它子树的 **graft 计划条目**，以及填充
/// 它零件层所提供的槽位的那些东西。一个只显示新文件、不显示消费方的预览，只显示了这次改动的一半——而它藏
/// 起来的那一半，正是调用方靠读它即将写入的那个文件看不见的那一半。
///
/// Only **plans that exist** are reported: a tree with no plan directory pays nothing (the same
/// ordering `affected`'s plan layer uses), and a face nothing targets says so rather than staying
/// silent, because silence there would read as "the consumers were not checked".
/// 只报**确实存在**的计划：没有计划目录的树不付任何代价（与 `affected` 的计划层同一个次序），而没有任何
/// 条目针对的面会把这件事说出来，而不是保持沉默——那里的沉默会被读成"消费方没查过"。
pub(super) fn consumer_diff(
    work: &Path,
    project: &Path,
    namespace: &str,
    node: &str,
    kind: &str,
    parts: &[(String, String)],
) -> Vec<String> {
    let mut lines = vec![format!(
        "slots      the layer adds {} filling option(s) inside `{kind}`: {}",
        parts.len(),
        parts
            .iter()
            .map(|(name, _)| name.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    )];
    // The plans are read from the **project**, never from the work directory: a preview copy skips
    // `.xirang` (`preview::skipped_directory`), so reading them there would make the preview say
    // "no plan targets this face" and the write say the opposite.
    // 计划读的是**项目**，绝不是工作目录：预览副本跳过 `.xirang`（`preview::skipped_directory`），
    // 在那里读会让预览说"没有计划针对这个面"、而写入说相反的话。
    let plans = project
        .join(xirang_kernel::lexicon::XIRANG_DIR)
        .join(xirang_kernel::lexicon::EXTERNAL_GRAFT_DIR);
    if !plans.is_dir() {
        return lines;
    }
    // The faces come from the work tree (that is where the deepened face is) and the declaration from
    // the project (that is where the entry is).
    // 面取自工作树（被做深的那个面在那里），而声明取自项目（入口在那里）。
    let Ok(faces) = face_views(work, namespace) else {
        return lines;
    };
    let declared = declared_grafts(project);
    let Ok(rows) = graft_plan_rows(project, &faces, declared.as_ref().ok()) else {
        return lines;
    };
    let prefix = format!("{}/", node.trim_end_matches('/'));
    let mut targeting: Vec<String> = rows
        .iter()
        .filter(|row| {
            row.target_path
                .as_deref()
                .is_some_and(|path| path == node || path.starts_with(&prefix))
        })
        .map(|row| {
            format!(
                "{} targets {}",
                row.selector,
                row.target_path.as_deref().unwrap_or("-")
            )
        })
        .collect();
    targeting.sort();
    targeting.dedup();
    if targeting.is_empty() {
        lines.push(format!(
            "consumers  none of the {} graft plan entry(ies) here targets this face or its subtree, \
             so this change moves no plan",
            rows.len()
        ));
    } else {
        lines.push(format!(
            "consumers  {} of {} graft plan entry(ies) target this face or its subtree ({}) — this \
             layer leaves every target and path alone, so those plans keep pointing where they point \
             and gain the options above",
            targeting.len(),
            rows.len(),
            targeting.join(", ")
        ));
    }
    lines
}
