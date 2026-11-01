//! The `cut` action: hand one face's subtree over by writing the declaration the plan reads.
//! `cut` 动作：把一个面的子树交出去，写进计划读的那条声明。
//!
//! Why this is its own module: `apply.rs` is a ratcheted file (600 code lines), and this action grew
//! past it. Splitting on the action keeps the whole write path in one place — the request shape, the
//! kernel renderer, the read-back, and the impact — rather than scattering it.
//! 为什么单独一个模块：`apply.rs` 是受棘轮约束的文件（600 代码行），而这个动作让它超了。按动作拆分把整条写
//! 路径留在一处——请求形状、内核渲染器、读回、影响面——而不是把它们散开。

use std::path::Path;

use serde_json::Value;

use crate::build_method::face_views;
use crate::mcp::apply::Outcome;

/// Hand one face's subtree over: write the declaration the build-time plan reads.
/// 把一个面的子树交出去：写进构建期计划读的那条声明。
///
/// **Appends one native declaration, including the first plan in a host.** The file is the one `declared_grafts`
/// already reads (`declared.entry`), the spelling comes from the kernel's own renderer
/// (`render_native_graft_expression`, so the two directions of this grammar live in one place), and the
/// result is **parsed back with the kernel's parser** before it is written: a write that does not
/// read back as exactly one more entry is refused, because what this puts in a host is source code.
/// **追加一条原生声明，也支持宿主的第一份计划。** 文件就是 `declared_grafts` 已经在读的那一个（`declared.entry`），
/// 拼写来自内核自己的渲染器（`render_native_graft_expression`——这套语法的两个方向因此住在一处），而写盘**之前**会
/// 用内核的解析器**读回来**：读回来不是"恰好多了条目"就拒绝——因为写进宿主的是源码。
///
/// The impact it reports is the faces the cut's target covers: the build prunes the subtree a cut
/// names, so those are the faces this application stops shipping. The match is on the module path the
/// expression writes (`crate::control::object::button::NODE_ID` ⇒ `control/object/button`), and the
/// build is still the authority — `check` on the preview confirms it.
/// 它报的影响面是切口目标覆盖的那些面：构建会剪掉切口命名的子树，因此那些面就是本应用不再发布的面。匹配依据
/// 是表达式写出的模块路径（`crate::control::object::button::NODE_ID` ⇒ `control/object/button`），而权威
/// 仍是构建本身——在预览上跑 `check` 即可确认。
pub(crate) fn run_cut(root: &Path, arguments: &Value) -> Result<Outcome, String> {
    let text = |key: &str| {
        arguments
            .get(key)
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_owned)
            .ok_or_else(|| format!("`cut` requires `{key}`"))
    };
    let cut = text("cut").map_err(|error| format!("{error}\n{}", cut_example(root)))?;
    let graft = text("graft").map_err(|error| format!("{error}\n{}", cut_example(root)))?;
    let cut_end = arguments
        .get("to")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let full = arguments.get("full").and_then(Value::as_bool) == Some(true);
    let declared = crate::build_method::declared_grafts(root).map_err(|error| {
        format!(
            "this tree's graft plan is unreadable, so there is nothing to add a cut to ({error}); \
             the plan is a native `&[Graft]` declaration or legacy graft macro in the host entry"
        )
    })?;
    let plan = declared.entry.clone();
    let source = std::fs::read_to_string(&plan)
        .map_err(|error| format!("{} is not readable: {error}", plan.display()))?;
    let existing = xirang_kernel::syntax::entries::graft_entries(&source).map_err(|error| {
        format!(
            "the plan does not parse before the write: {}",
            error.message
        )
    })?;
    // F6: an entry has to be written in the class the plan already uses. Checked on the input,
    // before the read-back, because the renderer would happily splice a logical path into
    // `cut(…)` and the parser would read it as an **expression** — a different declaration than
    // the caller meant, in a host's own source.
    // F6：一条条目必须写成这份计划已经在用的那一类。检查落在**输入**上、在读回之前，因为渲染器会照直把
    // 一个逻辑路径拼进 `cut(…)`，而解析器会把它读成**表达式**——一条与调用方本意不同的声明，写进的是
    // 宿主自己的源码。
    if let Some(refusal) = path_class_refusal(root, &cut, &existing) {
        return Err(refusal);
    }
    let logical = !cut.contains("::")
        && (cut.contains('/') || existing.iter().all(|entry| entry.expressions.is_none()));
    let (target, finish, replacement) = if logical {
        (
            format!("{cut:?}"),
            cut_end.map(|end| format!("{end:?}")),
            format!("{graft:?}"),
        )
    } else {
        (cut.clone(), cut_end.map(str::to_owned), graft.clone())
    };
    // A typed cut names a face by its **module** path, and a module path is not something a reader can derive from
    // the logical path: the source layout inserts directories (`<face>/object/<child>`) the logical path never
    // mentions. So a typed endpoint that matches no face in this tree would be written as a declaration naming
    // nothing — it cannot compile, and every later consumer refuses it as an "unresolved endpoint" (`apply move`
    // does, one step later, which is how this was found). Refusing here names the moment the mistake was made.
    // 类型化切口用**模块**路径命名一个面，而模块路径不是读者能从逻辑路径推导出来的：源码布局会插进逻辑路径从不提及的
    // 目录（`<face>/object/<child>`）。因此一个在本树里匹配不到任何面的类型化端点，会被写成一条谁也没命名的声明——
    // 它编译不过，而之后每个消费者都会以 "unresolved endpoint" 拒绝它（`apply move` 就在下一步这么做，这也正是它被
    // 发现的方式）。在这里拒绝，才能点名说错话的那一刻。
    if !logical
        && !matches!(
            cut.as_str(),
            "root" | "crate::ROOT_NODE_ID" | "ROOT_NODE_ID"
        )
        && covered(root, &cut).is_some_and(|covered| covered.is_empty())
    {
        return Err(format!(
            "`{cut}` names no face in this tree, so the declaration it would write names nothing: \
             a typed cut is a **module** path, and the source layout inserts directories the logical \
             path never mentions (`root/control/button` is `crate::control::object::button::NODE_ID`). \
             `registry` (or `faces`) lists the spellings this tree derives; nothing was written"
        ));
    }
    let entry = xirang_kernel::syntax::render_native_graft_expression(
        &target,
        finish.as_deref(),
        full,
        &replacement,
    )
    .map_err(|error| format!("this cut cannot be written: {}", error.message))?;
    let before = existing.len();
    let edited = xirang_kernel::syntax::append_native_graft(&source, &entry)
        .map_err(|error| error.to_string())?;
    let after = xirang_kernel::syntax::entries::graft_entries(&edited)
        .map_err(|error| format!("the written plan does not parse: {}", error.message))?;
    if after.len() != before + 1 {
        return Err(format!(
            "the write did not produce exactly one more declaration ({before} ⇒ {}), so it was \
             refused rather than left in the host",
            after.len()
        ));
    }
    let covered = covered_faces(root, &cut);
    crate::run_method::authoring::write_transaction::write_one(&plan, edited.as_bytes())?;
    Ok(Outcome {
        // The direction matters and I had it backwards once (round-9 review caught it by running the
        // thing): the build's scope **narrows to** the subtrees the cuts name, so a cut is what makes
        // a subtree part of what this application publishes — without it, a declared face is not
        // shipped at all. Saying "stops shipping" describes the opposite of what the write does.
        // 方向要紧，而我说反过一次（第九轮复核靠"跑一遍"抓到）：构建期作用域**收窄到**这些切口命名的子树，
        // 因此切口正是"让这棵子树进入本应用所发布之物"的那一步——没有它，声明了的面根本不会被发布。说成
        // "stops shipping" 描述的是这次写入的反面。
        message: format!(
            "wrote one declaration to {}\n  {entry}\nthe build's scope narrows **to** the subtrees \
             these cuts name, so this cut is what makes {} part of what this application publishes \
             (a declared face no cut names is not shipped); the build is the authority, so run \
             `check` to confirm",
            plan.display(),
            if covered.is_empty() {
                "no face whose module path this expression names (the build decides what the \
                 narrowed scope contains)"
                    .to_owned()
            } else {
                covered.join(", ")
            }
        ),
        declaration: None,
        source: plan,
        moved: false,
        // A cut has one reading: hand this subtree over. `deepen` is the action whose reply has to
        // state the other reading of its request.
        // 切口只有一种读法：把这棵子树交出去。需要说出请求的另一种读法的是 `deepen`。
        alternative: false,
        inherited: Vec::new(),
        consumers: Vec::new(),
    })
}

/// A complete cut request, with this tree's own face path and its plan's own graft spelling.
/// 一个完整的 cut 请求，里面填的是这棵树自己的面路径与它计划自己的 graft 拼写。
///
/// Both halves are read from the tree rather than templated: the `cut` side from the face the build
/// derives (`typed_spelling`), the `graft` side from the plan already on disk — so a reader who
/// pastes it gets a request that names things this host really has. A tree with no faces, or a plan
/// with no entries, falls back to the explicit placeholder, which at least says which value is the
/// reader's to supply.
/// 两半都从树上读、而不是套模板：`cut` 那一半取自构建推导出的面（`typed_spelling`），`graft` 那一半
/// 取自盘上已有的计划——因此照抄的读者得到的是一个点名了本宿主**真有**的东西的请求。没有面的树、或没有
/// 条目的计划，回落到显式占位符，那至少说清了哪个值该由读者提供。
fn cut_example(root: &Path) -> String {
    let cut = example_face(root)
        .and_then(|path| typed_spelling(root, &path))
        .unwrap_or_else(|| "crate::<the face's module>::NODE_ID".to_owned());
    let graft = plan_graft_example(root)
        .unwrap_or_else(|| "<the replacing crate>::<face>_fast::NODE_ID".to_owned());
    format!(
        "accepted shape  {{\"action\":\"cut\",\"cut\":\"{cut}\",\"graft\":\"{graft}\",\"apply\":true}}"
    )
}

/// One face path this tree derives, preferring a leaf: a cut hands a subtree over, and a leaf is the
/// shape an example can show without arguing about a parent's children.
/// 这棵树推导得出的一个面路径，优先叶子：切口交出的是一棵子树，而叶子是示例不用讨论某个父面的子级就能展示的
/// 形状。
fn example_face(root: &Path) -> Option<String> {
    let namespace = crate::mcp::registry::namespace(root).ok()?;
    let views = face_views(root, &namespace).ok()?;
    views
        .iter()
        .find(|view| !view.owns_registry)
        .or_else(|| views.first())
        .map(|view| view.path.clone())
}

/// The `graft` spelling this tree's own plan already uses, when it uses one.
/// 这棵树自己的计划已经在用的那个 `graft` 拼写（在用时）。
///
/// Read back through the kernel's parser rather than by text search, so what an example offers is
/// the spelling the build reads, not a line that looks like one. `None` when the host has no plan or
/// its entries name nothing — an example is a hint, and an empty one is worse than a placeholder.
/// 经内核的解析器读回，而不是按文本搜索，因此示例给出的拼写就是构建读的那个，而不是一行长得像它的东西。
/// 宿主没有计划、或条目一个名字都没点时给 `None`——示例是提示，而空的示例比占位符更糟。
pub(crate) fn plan_graft_example(root: &Path) -> Option<String> {
    let plan = crate::build_method::declared_grafts(root).ok()?.entry;
    let source = std::fs::read_to_string(plan).ok()?;
    let entries = xirang_kernel::syntax::entries::graft_entries(&source).ok()?;
    entries
        .first()
        .map(|entry| entry.graft.clone())
        .filter(|graft| !graft.trim().is_empty())
}

/// F6: the refusal when an entry joins a plan written in another class of spelling.
/// F6：一条条目要加入一份用**另一类拼写**写成的计划时给出的拒绝。
///
/// The gap this closes, measured by audit `F6`: `apply {action: "cut"}` accepted the logical path a
/// reader has in hand (`root/control/dial`) and rendered `cut(root/control/dial)` into a plan whose
/// every other entry is a Rust path (`cut(crate::control::object::button::NODE_ID)`). Both spellings
/// exist in this grammar, but they mean different things: the logical path names a face the
/// **build-time selector** resolves, while the typed form is the expression the compiler resolves.
/// So the entry that comes back is not the entry the caller asked for, and what was written is a
/// host's source code. The refusal names the class the plan uses and, derived from this tree's own
/// face list, the exact spelling to paste instead.
/// 这里补上的缺口由审计 `F6` 量到：`apply {action: "cut"}` 接受读者手上的逻辑路径
/// （`root/control/dial`），并把 `cut(root/control/dial)` 渲染进一份**其余每条**都是 Rust 路径
/// （`cut(crate::control::object::button::NODE_ID)`）的计划。两种拼写都在这套语法里，但含义不同：
/// 逻辑路径点名的是**构建期选择器**解析的面，类型化那一形是**编译器**解析的表达式。于是读回来的条目
/// 并不是调用方要的那条，而写进去的是宿主的源码。拒绝文案点名这份计划用的是哪一类，并从**本棵树自己的
/// 面清单**推导出应当照抄的那个拼写。
///
/// It refuses only when the plan is typed **throughout** (an empty plan has no class to join, and a
/// plan that already mixes the two spellings is not this check's business): the asymmetry is
/// deliberate — the other direction would refuse a request the grammar accepts.
/// 只在计划**通篇**类型化时拒绝（空计划没有类可加入，而一份本来就混用两种拼写的计划不归这条检查管）：
/// 这个不对称是刻意的——反方向会拒绝一个这套语法本来就接受的请求。
fn path_class_refusal(
    root: &Path,
    cut: &str,
    existing: &[xirang_kernel::syntax::entries::GraftSyntax],
) -> Option<String> {
    if cut.starts_with("crate::") || existing.is_empty() {
        return None;
    }
    if !existing
        .iter()
        .all(|entry| entry.cut.starts_with("crate::"))
    {
        return None;
    }
    let head = format!(
        "`{cut}` is a logical path, but every cut in this plan is written as the Rust path of the \
         face it names (`crate::…::NODE_ID`), so this entry belongs to another class and would be \
         written as an expression rather than as the selector you meant"
    );
    Some(match typed_spelling(root, cut) {
        Some(spelling) => format!(
            "{head}; write `{spelling}` — derived from this tree's own faces, where `{cut}` is the \
             face at src/{}. A plan is source code in the host, so this tool refuses a spelling it \
             cannot verify rather than one it can only guess at",
            face_source(root, cut).unwrap_or_else(|| "its declared source".to_owned())
        ),
        None => format!(
            "{head}, and `{cut}` does not name a face in this tree's derived registry, so the typed \
             spelling cannot be derived here — `registry` lists the faces and the source each one \
             lives in"
        ),
    })
}

/// The source file this tree's face at one logical path lives in, when the derivation found it.
/// 本棵树里某个逻辑路径上的面所住的源文件，推导找得到时给出。
fn face_source(root: &Path, logical: &str) -> Option<String> {
    let namespace = crate::mcp::registry::namespace(root).ok()?;
    let views = face_views(root, &namespace).ok()?;
    views
        .iter()
        .find(|view| view.path.trim_end_matches('/') == logical.trim_end_matches('/'))
        .map(|view| view.source.clone())
}

/// The typed `crate::…::NODE_ID` spelling of the face a logical path names, from the face tree.
/// 某个逻辑路径点名的面，按面树推导出的类型化 `crate::…::NODE_ID` 拼写。
///
/// Derived rather than assembled from the logical path: the two are different namespaces. A face's
/// logical path says where it sits in the **tree**, and its module path says where it sits in the
/// **crate** — the example's `root/control/button` is `crate::control::object::button::NODE_ID`
/// because the source layout inserts an `object` directory that the logical path never mentions.
/// Guessing that mapping would be exactly the kind of guess this module refuses to write into a
/// host, so the face's own `source` is the authority and a path this tree does not derive has no
/// spelling to offer.
/// 从面树推导而不是由逻辑路径拼装：两者是不同的命名空间。一个面的**逻辑路径**说的是它在**树**里的位置，
/// 模块路径说的是它在 **crate** 里的位置——示例里 `root/control/button` 是
/// `crate::control::object::button::NODE_ID`，因为源码布局插进了一个逻辑路径从不提及的 `object` 目录。
/// 猜这个映射，正是本模块拒绝写进宿主的猜法，因此以面自己的 `source` 为权威；本棵树推导不出的路径，
/// 就没有拼写可给。
///
/// The mapping itself is the build's own function (`build_method::source_module_path`), not a second
/// spelling of it written here: this file's whole job is writing a spelling the build will read back,
/// so a local copy of that rule is a copy that can disagree with the reader.
/// 映射本身用的是**构建自己的**函数（`build_method::source_module_path`），而不是在这里写第二份：本文件的
/// 全部工作就是写出构建会读回的拼写，因此这条规则的本地副本就是一份可能与读取方不一致的副本。
fn typed_spelling(root: &Path, logical: &str) -> Option<String> {
    let source = face_source(root, logical)?;
    let module = crate::build_method::source_module_path(source.trim_start_matches("./"));
    Some(format!("crate::{module}::NODE_ID"))
}

/// The faces a cut expression's module path covers, named by their node paths.
/// 切口表达式的模块路径覆盖的那些面，按它们的节点路径点名。
fn covered_faces(root: &Path, cut: &str) -> Vec<String> {
    covered(root, cut).unwrap_or_default()
}

/// The same answer, with "this tree cannot answer" kept apart from "nothing matched".
/// 同一个答案，但把"这棵树答不了"与"没有东西匹配"分开。
///
/// `None` means the tree has no faces to compare against (a package whose registration tree lives in another crate),
/// so a typed endpoint cannot be judged either way. An empty `Some` means the tree **can** answer and the answer is
/// "no such face" — the case worth refusing, because the declaration it would write names nothing.
/// `None` 表示这棵树没有可比较的面（注册树住在别的 crate 的包），因此类型化端点两种判断都做不了。空的 `Some` 表示这棵树
/// **能**回答，而答案是"没有这个面"——这才是值得拒绝的情形，因为它写下的声明谁也没命名。
fn covered(root: &Path, cut: &str) -> Option<Vec<String>> {
    let module = cut
        .strip_prefix("crate::")
        .and_then(|rest| rest.strip_suffix("::NODE_ID"))?;
    let wanted = module.replace("::", "/");
    let namespace = crate::mcp::registry::namespace(root).ok()?;
    let views = face_views(root, &namespace).ok()?;
    if views.is_empty() {
        return None;
    }
    let mut covered: Vec<String> = views
        .iter()
        .filter(|view| {
            let dir = view
                .source
                .rsplit_once('/')
                .map_or(view.source.as_str(), |(dir, _)| dir);
            dir == wanted || dir.starts_with(&format!("{wanted}/"))
        })
        .map(|view| view.path.clone())
        .collect();
    covered.sort();
    Some(covered)
}

#[cfg(test)]
#[path = "apply_cut_tests.rs"]
pub(crate) mod apply_cut_tests;
