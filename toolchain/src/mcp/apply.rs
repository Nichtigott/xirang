//! The bridge's write path: agent-initiated edits, previewed before they land.
//! 桥的写入路径：代理发起的编辑，落盘前先预览。
//!
//! The bridge does not re-implement editing. It builds the same `Registry` Studio
//! builds from the package's own source, installs the same `AuthoringContext`, and
//! calls the same executor, so an agent's edit passes the kernel's admission,
//! parent-rule, and topology checks — and is refused for the same reasons a
//! human's edit in Studio is.
//! 桥不重新实现编辑。它从包自己的源码构建出与 Studio 相同的 `Registry`，装上相同的
//! `AuthoringContext`，调用同一个执行器，因此代理的编辑会经过内核的准入、父规则与拓扑校验——
//! 也会因为与人在 Studio 里编辑相同的原因被拒绝。
//!
//! **Preview is the default, and it is the real operation.** A preview runs the
//! executor against a throwaway copy of the package (`crate::mcp::preview`), then reports
//! two things: the diff between the copy and the project (which files would change,
//! and how), and the registration faces the copy derives afterwards. That is why the
//! preview cannot drift from the apply: both call `run`, and only the directory
//! differs.
//! **预览是默认，而且它就是真实操作。** 预览在一份一次性的包副本上运行执行器
//! （`crate::mcp::preview`），然后报告两件事：副本与项目之间的 diff（哪些文件会变、怎么变），以及
//! 副本随后推导出的注册面。这正是预览不可能与落盘漂移的原因：两者都调用 `run`，差的只是目录。

use std::path::{Path, PathBuf};

#[path = "apply_cut.rs"]
mod apply_cut;

#[path = "apply_move.rs"]
mod apply_move;

#[path = "apply_promote.rs"]
mod apply_promote;

#[path = "apply_fields.rs"]
mod apply_fields;

#[path = "apply_plan.rs"]
mod apply_plan;

#[path = "apply_delete.rs"]
mod apply_delete;

#[cfg(test)]
#[path = "apply_template_tests.rs"]
mod apply_template_tests;

#[path = "apply_template.rs"]
pub(crate) mod apply_template;

#[cfg(test)]
#[path = "apply_rules_tests.rs"]
mod apply_rules_tests;

pub(super) use self::apply_fields::EDITABLE_FIELDS;
use self::apply_fields::{action_name, invalid_field, overlay, text};

use crate::authoring_service::{AuthoringRequest, AuthoringService, update_from_json};
use crate::build_method::{face_views, source_layout};
use crate::run_method::AuthoringContext;
use serde_json::Value;
use xirang_kernel::Registry;
use xirang_kernel::authoring::AuthoringOperation;
use xirang_kernel::interchange::LanguageId;

use self::apply_plan::{
    forget_plan, precheck_rules, read_plan_record, refuse_if_plan_is_stale, remember_plan,
    render_rules, request_key, touched_subjects,
};
use crate::mcp::apply_target::Target;
use crate::mcp::preview::{copy_package, declaration_line, diff_package};
use crate::mcp::registry::namespace;
use crate::mcp::resolve::{parent_id, resolve_node};

/// One `xirang.apply` request.
/// 一次 `xirang.apply` 请求。
#[derive(Clone, Copy)]
enum Action {
    /// Create a module registration face.
    /// 创建一个模块注册面。
    Add,
    /// Rewrite the fields a request names, keeping the rest of the face.
    /// 重写请求点名的字段，面的其余部分保持不动。
    Edit,
    /// Change a face's module name, keeping every other field.
    /// 改一个面的模块名，其余字段全部保留。
    Rename,
    /// Move one face's module subtree into the recoverable trash.
    /// 把一个面的模块子树移入可恢复的回收目录。
    Delete,
    /// Add a layer **inside** one face, leaving its declaration, path and tree row alone.
    /// 在一个面**内部**加一层，不动它的声明、路径与树行。
    Deepen,
    /// Hand one face's subtree over to another implementation, by writing the declaration the
    /// build-time plan reads.
    /// 把一个面的子树交给另一份实现：写进构建期计划读的那条声明。
    Cut,
    /// Land a confirmed external graft record: rewrite the target face's declaration with the
    /// external implementation's fields, retire the declaration entry, and move the record to the
    /// trash.
    /// 把一条已确认的外部 graft 记录落地：用外部实现的字段改写目标面的声明、退役那条声明条目，并把
    /// 记录移进回收目录。
    Promote,
    /// Relocate a face — and the module subtree under it — to a new parent.
    /// 把一个面（连同它下面的模块子树）搬到新的父级之下。
    Move,
}

/// Run one edit request, previewing unless `apply` is true.
/// 执行一次编辑请求；除非 `apply` 为真，否则只预览。
///
/// `root` is the resolved package root. A root Cargo cannot name is refused here
/// before anything is copied or written — the same refusal the registry query
/// makes, for the same reason: a namespace that is guessed authors faces no host
/// compiles.
/// `root` 是已解析的包根。Cargo 说不出名字的包根在这里、在复制或写入任何东西之前就被拒绝——
/// 与注册树查询相同的拒绝，理由也相同：猜出来的命名空间会创作出没有宿主编译的注册面。
pub(crate) fn apply(root: &Path, arguments: &Value) -> Result<String, String> {
    let action = match arguments.get("action").and_then(Value::as_str) {
        Some("add") => Action::Add,
        Some("edit") => Action::Edit,
        Some("rename") => Action::Rename,
        Some("delete") => Action::Delete,
        Some("deepen") => Action::Deepen,
        Some("cut") => Action::Cut,
        Some("promote") => Action::Promote,
        Some("move") => Action::Move,
        Some(other) => {
            // The example is built from this tree, so the refusal is one paste away from a call
            // rather than one research step away. A root whose namespace cannot be read still gets
            // the sentence, without the example — the namespace error is a different refusal.
            // 示例由**这棵树**搭出来，因此这条拒绝离一次调用只差一次粘贴，而不是差一次调研。命名空间读不出来
            // 的包根照样拿到这句话，只是没有示例——那是另一种拒绝。
            let example = namespace(root)
                .ok()
                .map(|namespace| write_example(root, &namespace, Action::Add));
            return Err(format!(
                "action `{other}` is not implemented; this tool supports `add`, `edit`, \
                 `rename`, `delete`, `deepen`, `cut`, `promote` and `move`{}",
                example.map_or(String::new(), |example| format!("\n{example}"))
            ));
        }
        None => {
            let example = namespace(root)
                .ok()
                .map(|namespace| write_example(root, &namespace, Action::Add));
            return Err(format!(
                "xirang.apply requires `action` (`add`, `edit`, `rename`, `delete`, `deepen`, \
                 `cut`, `promote`, or `move`){}",
                example.map_or(String::new(), |example| format!("\n{example}"))
            ));
        }
    };
    let namespace = namespace(root)?;
    let apply = arguments
        .get("apply")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    // P1: for create/edit/rename, the project's own rules are a **premise of the plan**, not a
    // report a reader might notice afterwards. A direct apply therefore runs the request on a copy
    // first, decides the rules against the copy's post-change state, and only then repeats it on
    // the project. An explicit violation stops here, before any byte of the project changes.
    // P1：对创建/编辑/改名，项目自己的规则是计划的**前提**，而不是读者事后也许注意到的报告。因此直接
    // apply 也先在副本上跑一次、按副本改动后的状态判定规则，然后才在项目上重放。明确违规在这里停下，
    // 项目一个字节都没变。
    let rule_plan = if matches!(action, Action::Add | Action::Edit | Action::Rename) {
        Some(request_key(root, action, arguments))
    } else {
        None
    };
    let (mut has_plan, mut planned_template) = (false, None::<String>);
    if apply
        && let Some(key) = &rule_plan
        && let Some(record) = read_plan_record(key)
    {
        has_plan = true;
        planned_template = apply_plan::plan_template(&record);
        // A preview is a promise about a premise. If the rule book moved since, the promise cannot
        // be kept by applying anyway: the caller previewed one decision and would write under
        // another. Say which facts changed.
        // 预览是关于一个前提的承诺。若规则簿在那之后动过，就不能靠"照样应用"兑现它：调用方预览的是一个
        // 决定，却会在另一个决定下写入。要点名是哪些事实变了。
        refuse_if_plan_is_stale(root, &record)?;
    }
    let prechecked = if apply && matches!(action, Action::Add | Action::Edit | Action::Rename) {
        Some(precheck_rules(root, &namespace, arguments, action)?)
    } else {
        None
    };
    let target = if apply {
        Target::Project
    } else {
        Target::Copy(copy_package(root)?)
    };
    let write = matches!(target, Target::Project);
    let work = target.work_dir(root);
    // P3, first slice: an `add` selects the approved template of its family before anything is
    // written. A conflict is a refusal here, so nothing is written; a unique specimen or a new
    // category becomes a reply line. See `apply_template` for the three-way choice.
    // P3 第一片：`add` 在任何写入前选择家族的已批准模板。冲突在这里成为拒绝，因此什么都不写；唯一样板或
    // 新类别成为一行回复。三态判定见 `apply_template`。
    let mut effective_arguments = arguments.clone();
    let mut template_anchor: Option<String> = None;
    let template_line = if matches!(action, Action::Add) {
        match apply_template::add_choice(root, work, &namespace, arguments, write) {
            Ok(choice) => {
                // The template contributes shape fields only where the request stayed silent; a value
                // the author wrote always wins.
                // 模板只填请求没说的形状字段；作者写下的值永远优先。
                if !choice.defaults.is_empty()
                    && let Some(fields) = effective_arguments
                        .get_mut("fields")
                        .and_then(Value::as_object_mut)
                {
                    for (field, value) in choice.defaults {
                        fields.entry(field).or_insert(Value::String(value));
                    }
                }
                template_anchor = choice.anchor;
                choice.line
            }
            Err(refusal) => {
                target.discard(root);
                return Err(refusal);
            }
        }
    } else {
        String::new()
    };
    // A delete removes the objects rules may be bound to; the commitments that cannot survive are
    // named in the reply rather than left to a later "why is this rule unknown" question.
    // 删除会移除规则可能绑定的对象；活不下来的承诺在回复里点名，而不是留给以后一句"这条规则为什么是未知"。
    let mut removal_stranded: Vec<String> = Vec::new();
    if write
        && has_plan
        && let Err(refusal) = apply_template::ensure_plan_template(
            planned_template.as_deref(),
            template_anchor.as_deref(),
        )
    {
        target.discard(root);
        return Err(refusal);
    }
    let outcome = match action {
        Action::Add => run_add(work, &namespace, &effective_arguments),
        Action::Edit | Action::Rename => run_edit(work, &namespace, arguments, action),
        Action::Delete => {
            let (outcome, stranded) =
                apply_delete::run_with_stranded(root, work, &namespace, arguments)?;
            removal_stranded = stranded;
            Ok(outcome)
        }
        // `deepen`'s consumer story has two halves that live in different trees: the face it
        // deepened is in the **work** directory (a copy, in a preview), while the graft plans it must
        // name are in the **project** — the preview copy skips `.xirang` by design. Passing both is
        // what keeps the preview from telling a different consumer story than the write, which is the
        // one thing a preview must never do (audit `W5-3`).
        // `deepen` 的消费方说法有两半，住在两棵不同的树上：它做深的那个面在**工作**目录里（预览时是副本），
        // 而它必须点名的 graft 计划在**项目**里——预览副本按设计跳过 `.xirang`。两棵都传进去，才让预览
        // 不讲述一个与写入不同的消费方故事，而那正是预览绝不能做的事（审计 `W5-3`）。
        Action::Deepen => run_deepen(work, root, &namespace, arguments),
        Action::Cut => apply_cut::run_cut(work, arguments),
        // `promote` reads the record from the project root and rewrites source in the work
        // directory, so it is the one action that needs both — and the one that has to know
        // whether it is applying, because only then may the record move.
        // `promote` 从项目根读记录、在工作目录里改写源码，因此它是唯一同时需要两者的动作——也是必须知道
        // 自己是否在落盘的那个，因为只有落盘时才可以把记录移走。
        Action::Promote => apply_promote::run_promote(root, work, write, arguments),
        Action::Move => apply_move::run_move(root, work, write, arguments),
    };
    let outcome = match outcome {
        // A preview ran in the copy, so the paths the executor reported belong to
        // the copy. They are rewritten here, before anything reads them, rather
        // than left to a reader to reinterpret: an agent reading `would write
        // /tmp/...` would be told about a directory that is deleted a moment later.
        // 预览在副本里运行，因此执行器报告的路径属于副本。它们在这里、在任何读取方之前被改写，
        // 而不是留给读者自行解释：读到 `would write /tmp/...` 的代理会被告知一个随后就被删掉的目录。
        Ok(mut outcome) => {
            // The declaration anchor is read while the tree it happened in still
            // exists: for a preview that is the copy, for an apply it is the project.
            // 声明锚点是在改动发生的那棵树仍然存在时读取的：预览是副本，落盘是项目。
            let original = outcome.source.clone();
            outcome.declaration =
                declaration_line(&original, &target.report_relative(&original, root));
            outcome.source = target.report_path(&outcome.source, root);
            outcome.message = target.report_message(outcome.message, root);
            outcome
        }
        Err(error) => {
            // A failed preview leaves the copy behind for nothing; a failed apply
            // keeps the project, and the executor has already refused before
            // touching it.
            // 失败的预览不必留下副本；失败的落盘保留项目，而执行器在碰它之前就已经拒绝。
            target.discard(root);
            return Err(refused_with_a_way_forward(error));
        }
    };
    // The preview's rules are decided while the copy still exists; the applied reply uses the
    // pre-check's decisions, which are the same tree bytes the project now holds.
    // 预览的规则在副本仍存在时判定；落盘的回复用预检那一次的决定——它们针对的正是项目现在持有的字节。
    let rules_section = match (&target, action) {
        (Target::Copy(work), Action::Add | Action::Edit | Action::Rename) => {
            let subjects = touched_subjects(work, &namespace, &outcome, arguments);
            match super::rules::check_subjects(root, work, &subjects) {
                Ok(checks) => {
                    if let Some(key) = &rule_plan {
                        // The record lives outside the project on purpose: a preview promises not
                        // to write into it, and that promise is pinned.
                        // 记录有意住在项目之外：预览承诺不写进项目，而这条承诺有钉子。
                        remember_plan(root, key, action, &subjects, template_anchor.as_deref());
                    }
                    if checks.is_empty() {
                        None
                    } else {
                        Some(render_rules(&checks, true))
                    }
                }
                Err(error) => Some(format!(
                    "rules: not decided — {error}\nthe copy is discarded, so this preview cannot say whether the change would satisfy the project's rules\n"
                )),
            }
        }
        (Target::Project, Action::Add | Action::Edit | Action::Rename) => {
            prechecked.map(|prechecked| render_rules(&prechecked.checks, false))
        }
        (_, action) => {
            // These actions do not change the fields this build decides (`declares`, `provides`,
            // `requires`), so a field-level precheck would be vacuous — saying so is more honest
            // than "not checked", which reads as "nobody got around to it". `move`/`delete` still
            // report the knowledge they strand above; that is a different question from field truth.
            // 这些动作不改变本构建可判定的字段（`declares`/`provides`/`requires`），因此字段级预检是空的；
            // 说出来比 "not checked" 诚实——后者读起来像"还没人做"。`move`/`delete` 仍在上方报告它们留下的
            // 知识，那是与字段真值不同的问题。
            let reason = match action {
                Action::Move | Action::Delete => {
                    "it changes where an object exists, not the fields a rule decides (`declares`, `provides`, `requires`); the knowledge it strands is reported above"
                }
                Action::Deepen => {
                    "it adds a parts type inside the face and cannot change `declares`, `provides`, `requires`"
                }
                Action::Cut | Action::Promote => {
                    "it changes what an application publishes or which crate owns a declaration, not the fields a rule decides"
                }
                _ => "rule enforcement covers add/edit/rename",
            };
            Some(format!(
                "rules: no field-level check for `{}` — {reason}; this is not a proof that the rules hold\n",
                action_name(action)
            ))
        }
    };
    let report = match report(
        root,
        &target,
        &namespace,
        &outcome,
        arguments.get("full").and_then(Value::as_bool) == Some(true),
    ) {
        Ok(report) => report,
        // The write already happened, so returning an error here would be an error reply on a
        // changed tree — the exact shape the move transaction exists to prevent. The derived half of
        // the reply is missing; say what was written and why, instead of failing a request that
        // succeeded.
        // 写入已经发生，因此在这里返回错误就是"树已改变却回错误"——正是搬动事务要防止的形状。回复里推导
        // 出来的那一半缺失了；说清写了什么、为什么，而不是让一个已经成功的请求失败。
        Err(error) => format!(
            "action {}\nnamespace {namespace}\n{}\nnote: the tree section of this reply could not be \
             derived: {error}\n",
            if write { "apply" } else { "preview" },
            outcome.message
        ),
    };
    // The copy's diff is computed before it goes away.
    let diff = match &target {
        Target::Project => String::new(),
        Target::Copy(work) => diff_package(root, work)?,
    };
    target.discard(root);
    let mut text = report;
    apply_plan::append_reply_lines(&mut text, &template_line, &removal_stranded);
    if let Some(section) = rules_section {
        text.push('\n');
        text.push_str(&section);
    }
    if !diff.is_empty() {
        text.push_str("\ndiff:\n");
        text.push_str(&diff);
    }
    // A write that landed is the scheduling problem the maintainer named; the whole story — start the
    // refresh, then say what the index is — lives in one place (audit `M7`, P1.2 / P2.2).
    // 落盘的写入正是维护者点名的那个调度问题；整段说法——启动刷新、再说索引如何——只住在一处
    // （审计 `M7`，P1.2 / P2.2）。
    if matches!(target, Target::Project) {
        if let Some(key) = &rule_plan {
            forget_plan(key);
        }
        text.push('\n');
        text.push_str(&crate::mcp::index::after_write(root));
    }
    Ok(text)
}

/// What one successful executor call changed.
/// 一次成功的执行器调用改了什么。
///
/// `pub(crate)` because `new_project` runs the same `add` path for the faces a request
/// asks for at creation time, and that path must stay one implementation rather than
/// two field mappings that drift.
/// `pub(crate)` 是因为 `new_project` 对请求在创建时就要的那些面跑的是同一条 `add` 路径——
/// 这条路径必须只有一份实现，而不是两份会漂移的字段映射。
pub(crate) struct Outcome {
    message: String,
    /// `<path>:<line>` of the face declaration this change produced, when there is
    /// one (a delete moves the file away and has none).
    /// 本次改动产生的面声明所在的 `<path>:<line>`；删除把文件搬走，因此没有。
    declaration: Option<String>,
    /// The file the executor wrote — or, for a delete, the trash path it moved the
    /// module to.
    /// 执行器写入的文件——对删除而言，则是它把模块搬到的回收路径。
    source: PathBuf,
    /// Whether the path is a destination rather than a rewritten source.
    /// 该路径是目的地，而不是被重写的源文件。
    moved: bool,
    /// Whether this change came from `deepen`, whose reply also states the *other* reading of
    /// "make it deeper" and what that one costs.
    /// 这次改动是否来自 `deepen`——它的回复还会说出"把它做深"的**另一种读法**及那种读法的代价。
    alternative: bool,
    /// The family declarations this request left out and the tree filled in (audit `W5-6`).
    /// 这次请求没写、而树替它补上的家族声明（审计 `W5-6`）。
    inherited: Vec<String>,

    /// What this change does to the tree's **consumers**, computed from the tree (audit `W5-3`).
    /// 这次改动对树的**消费方**做了什么，从树上算出（审计 `W5-3`）。
    ///
    /// Empty for an action that has no consumer story to tell. Filled by the executor rather than by
    /// the report because the executor is the one holding the facts — and because the preview runs the
    /// same executor on a copy, which is what makes the preview and the landed reply say the **same**
    /// thing instead of two hand-kept variants of it.
    /// 对"没有消费方故事可说"的动作是空的。由执行器而不是报告来填，因为事实在执行器手里——也因为预览在
    /// 副本上跑的是同一个执行器，这正是预览与落盘回复说**同一件事**（而不是两句各自维护的话）的原因。
    consumers: Vec<String>,
}

/// Create a face through the **shared** authoring service, for every language.
/// 经**共享**创作服务创建一个注册面，对所有语言。
///
/// Why one entrance: this function used to exist only for the non-Rust half, while Rust kept an older
/// executor (`load_registry` + `add_module_from_face`). That executor's staged registry carries Rust
/// faces only, so a Rust child of a C++ parent was refused for "a parent that is not registered" even
/// though the service already knew how to author it — the reproduced defect, and an entrance fork
/// rather than a missing language.
/// 为什么只有一个入口：这个函数过去只服务非 Rust 那一半，而 Rust 保留着一个更老的执行器
/// （`load_registry` + `add_module_from_face`）。那个执行器的暂存注册树只有 Rust 面，因此挂在 C++ 父下的
/// Rust 子面会因为"父未登记"被拒——尽管共享服务已经知道怎么创作它。这正是复现的缺陷，是入口分叉而不是
/// 缺少一种语言。
fn run_add_in_language(
    root: &Path,
    namespace: &str,
    arguments: &Value,
    fields: &Value,
    language: Option<&LanguageId>,
    inherited: Vec<String>,
) -> Result<Outcome, String> {
    let object = fields
        .as_object()
        .ok_or_else(|| "`fields` must be a JSON object".to_owned())?;
    let mut face_fields = object.clone();
    // `parent` names a place in the tree, not a field of the face; it is a request-level sibling.
    // `parent` 命名树里的位置而不是面的字段；它是与请求平级的。
    face_fields.remove("parent");
    let registration = update_from_json(&Value::Object(face_fields))?;
    let module = object
        .get("module")
        .and_then(Value::as_str)
        .map(str::to_owned);
    // A request that names neither `parent` nor a `parent` field leaves the parent **open**, so an
    // island directory can decide it. Naming the root here would make the inference unreachable and
    // would author the child under the root instead of under the object that owns the directory.
    // 既不点名 `parent`、也没有 `parent` 字段的请求把父级**留空**，使孤岛目录可以决定它。在这里写成根会
    // 让推断不可达，并把子面创作到根下，而不是创作到拥有那个目录的对象下。
    let named_parent = arguments.get("parent").is_some() || object.get("parent").is_some();
    let parent = named_parent
        .then(|| parent_id(root, namespace, arguments, fields))
        .transpose()?;
    let mut request = AuthoringRequest::create();
    request.language = language.cloned();
    request.module = module;
    request.parent = parent;
    request.destination = arguments
        .get("file")
        .and_then(Value::as_str)
        .map(str::to_owned);
    request.registration = registration;
    let service = AuthoringService::new(root.to_path_buf(), namespace.to_owned());
    // One plan semantics for preview and apply: `apply` is the only caller of the executor, and a
    // preview runs this same function against a throwaway copy (`mcp::preview`), so the two cannot
    // describe different changes.
    // 预览与落盘共用同一份计划语义：执行器的唯一调用方是 `apply`，而预览是在一次性副本上跑这同一个函数
    // （`mcp::preview`），因此两者不可能描述不同的改动。
    let plan = service.plan(&request)?;
    let target = plan
        .changed_files()
        .map(|file| file.path.clone())
        .find(|path| {
            let language = language.map(LanguageId::as_str);
            xirang_multilang::registration::registry()
                .language_of_path(path)
                .map(|claimed| language.is_none_or(|language| claimed.as_str() == language))
                .unwrap_or(false)
        })
        .or_else(|| plan.changed_files().map(|file| file.path.clone()).next())
        .unwrap_or_default();
    let declaration =
        crate::mcp::preview::declaration_line(&root.join(&target), Path::new(&target));
    let applied = service.apply(&plan)?;
    Ok(Outcome {
        message: applied.message,
        declaration,
        source: root.join(target),
        moved: false,
        alternative: false,
        inherited,
        consumers: Vec::new(),
    })
}

/// Create a face.
/// 创建一个注册面。
pub(crate) fn run_add(root: &Path, namespace: &str, arguments: &Value) -> Result<Outcome, String> {
    // One entrance for every language: Rust included. The field validation, the family completion and
    // the module requirement are the bridge's own work (they read the tree and the request), while
    // resolving the language, the parent and the destination is the shared service's.
    // 所有语言一个入口，包括 Rust。字段校验、家族补全与 module 必需性是本桥自己的工作（它们读树与请求），
    // 而解析语言、父级与目的地是共享服务的工作。
    let language = match arguments.get("language").and_then(Value::as_str) {
        Some(id) => Some(
            LanguageId::parse_language(id).ok_or_else(|| format!("`{id}` is not a language id"))?,
        ),
        // The bridge's long-standing default: an `add` that names neither a language nor a
        // destination is a Rust face. A destination's extension is the other selector, and the
        // service reads it; neither one being present is what makes Rust the answer.
        // 本桥长期以来的默认：既没点名语言、也没点名目的地的 `add` 是一个 Rust 面。另一条选择依据是
        // 目的地的扩展名，由服务读取；两者都没有时，答案就是 Rust。
        None if arguments.get("file").is_none() => {
            Some(LanguageId::parse_language("rust").expect("`rust` is a valid language id"))
        }
        None => None,
    };
    let fields = arguments.get("fields").unwrap_or(&Value::Null);
    if let Some(problem) = invalid_field(fields, Action::Add) {
        return Err(problem);
    }
    // Audit `W5-6`: the fields the caller left out are completed from the family (see `apply_family`).
    // 审计 `W5-6`：调用方没写的字段由家族补全（见 `apply_family`）。
    let (completed, inherited) =
        apply_family::complete_add_fields(root, namespace, arguments, fields);
    let fields = &completed;
    let module = text(fields, "module");
    if module.is_empty() {
        // Two different mistakes used to share one sentence, and that sentence named
        // `fields.module` even when the caller had sent exactly that — as a dotted key,
        // which the tool never sees as a `fields` object at all. The round-13 benchmark
        // measured the cost: three refusals in a row on a fresh agent's first `apply add`.
        // 两种不同的错过去共用一句话，而那句话在调用方**确实**发了 `fields.module` 时（以点号键的
        // 形式，工具根本看不到 `fields` 对象）也照说 `fields.module`。第十三轮量出了代价：一个新代理
        // 的第一次 `apply add` 连吃三次拒绝。
        return Err(if arguments.get("fields").is_none() {
            format!(
                "add requires `fields`: one JSON object holding the new face's fields; on a command \
                 line that object is written `--fields '{{\"module\":\"button\",\"kind\":\
                 \"Object\"}}'`, and one field of it may also be written `--fields.module button`\n{}",
                write_example(root, namespace, Action::Add)
            )
        } else {
            format!(
                "add requires `fields.module`, the new module's name\n{}",
                write_example(root, namespace, Action::Add)
            )
        });
    }
    run_add_in_language(
        root,
        namespace,
        arguments,
        fields,
        language.as_ref(),
        inherited,
    )
}

/// Rewrite one non-Rust face's fields through the shared authoring service.
/// 经共享创作服务重写一个非 Rust 面的字段。
fn run_edit_in_language(
    root: &Path,
    namespace: &str,
    arguments: &Value,
    language: &LanguageId,
    action: Action,
) -> Result<Outcome, String> {
    if !matches!(action, Action::Edit) {
        return Err(format!(
            "`{}` does not implement `{}` for `{}`; only `edit` is wired to the shared authoring \
             service in this build",
            action_name(action),
            action_name(action),
            language.as_str()
        ));
    }
    let file = arguments
        .get("file")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            format!(
                "edit in `{}` requires `file`: the package-relative path of the face to rewrite \
                 (the bridge's `node` resolver only knows Rust faces)",
                language.as_str()
            )
        })?;
    let fields = arguments.get("fields").unwrap_or(&Value::Null);
    let object = fields
        .as_object()
        .ok_or_else(|| "`fields` must be a JSON object".to_owned())?;
    let mut face_fields = object.clone();
    face_fields.remove("parent");
    let registration = update_from_json(&Value::Object(face_fields))?;
    let mut request = AuthoringRequest {
        language: Some(language.clone()),
        operation: AuthoringOperation::Modify,
        node: None,
        module: None,
        parent: None,
        destination: Some(file.to_owned()),
        registration,
    };
    request.language = Some(language.clone());
    let service = AuthoringService::new(root.to_path_buf(), namespace.to_owned());
    let plan = service.plan(&request)?;
    let applied = service.apply(&plan)?;
    Ok(Outcome {
        message: applied.message,
        declaration: None,
        source: root.join(file),
        moved: false,
        alternative: false,
        inherited: Vec::new(),
        consumers: Vec::new(),
    })
}

fn run_edit(
    root: &Path,
    namespace: &str,
    arguments: &Value,
    action: Action,
) -> Result<Outcome, String> {
    // A non-Rust edit names its file: the bridge's node resolver only knows the Rust registry, and
    // a foreign face is not in it until the build reads its record.
    // 非 Rust 编辑点名它的文件：桥的节点解析器只认识 Rust 注册树，而外语面在构建读回记录之前不在其中。
    if let Some(language) = arguments.get("language").and_then(Value::as_str) {
        let language = LanguageId::parse_language(language)
            .ok_or_else(|| format!("`{language}` is not a language id"))?;
        if language.as_str() != "rust" {
            return run_edit_in_language(root, namespace, arguments, &language, action);
        }
    }
    let target = arguments
        .get("node")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            // Every other action names its accepted shape here; `edit` and `delete` did not,
            // and the round measured `apply` being the most-refused tool (16 of 41 refusals).
            // 别的动作都在这里点名可接受的形状，`edit` 与 `delete` 没有——而那一轮量到 `apply` 是
            // 被拒最多的工具（41 次拒绝里占 16 次）。
            format!(
                "{} requires `node`: a logical path or a 32-digit identity (the full key list is \
                 `--list apply`)\n{}",
                if matches!(action, Action::Rename) {
                    "rename"
                } else {
                    "edit"
                },
                write_example(root, namespace, action)
            )
        })?;
    let fields = arguments.get("fields").unwrap_or(&Value::Null);
    let registry = load_registry(root, namespace)?;
    let id = resolve_node(root, namespace, target)?;
    if matches!(action, Action::Rename) {
        let module = fields.get("module").and_then(Value::as_str).unwrap_or("");
        if module.is_empty() {
            return Err(format!(
                "rename requires `fields.module`, the new module name\n{}",
                write_example(root, namespace, Action::Rename)
            ));
        }
    }
    // Both halves run inside one context, and not only the write: reading the face
    // back resolves its path from the context's package root, so a read outside it
    // would look for the face under whatever directory the process happens to be in
    // — a bug this pin caught, because the CLI-shaped manual run had
    // `XIRANG_PACKAGE_ROOT` set and the unit test did not.
    // 两半都在同一个上下文里运行，而且不只是写入那一半：读回该面时它的路径由上下文的包根解析，因此
    // 在上下文之外读会去进程碰巧所在的目录里找那个面——正是这个缺陷被钉子抓住：手工的 CLI 式运行设了
    // `XIRANG_PACKAGE_ROOT`，而单元测试没设。
    let context = AuthoringContext::new(root.to_path_buf(), namespace.to_owned());
    let (change, previous_kind, new_kind) = context.scope(|| -> Result<_, String> {
        let mut authored = crate::run_method::authored_face(&registry, id)?;
        let previous = authored.kind.clone();
        overlay(&mut authored, fields, action)?;
        let kind = authored.kind.clone();
        let change = crate::run_method::edit_module_face(&registry, id, &authored.as_patch())?;
        Ok((change, previous, kind))
    })?;
    // `kind` is an identity input — `NodeId = hash(namespace, source, name)`, and a face's name is its
    // kind — so an edit that changes it rewrites the marker type and the `kind:` field, and the face
    // comes back under a *new* identity while every graft record keyed by the old one stops
    // resolving. The reply used to report the resulting tree without ever saying so (audit `L6`);
    // one sentence closes it, and it points at the tool that lists the fallout.
    // `kind` 是身份输入——`NodeId = hash(namespace, source, name)`，而面的名字就是它的 kind——因此改它的
    // 编辑会重写标记类型与 `kind:` 字段，这个面以**新身份**回来，而以旧身份为键的每条 graft 记录都不再
    // 解析。回复过去只报告结果树、从不说明（审计 `L6`）；一句话即可闭合，并指向列出后果的工具。
    let mut message = change.message;
    if previous_kind != new_kind {
        // The *new* identity is deliberately not printed: computing it here would mean reproducing
        // the kernel's `(namespace, relative path, declared name)` triple from a `PathBuf` that is
        // not spelled the way `face_views` spells it, and the pin below caught exactly that (the
        // number it printed was not the one the tree reported). A wrong number in a diagnostic is
        // worse than no number, so the reply names the old identity — the one graft records are
        // keyed by — and points at the tool that lists the fallout.
        // **新**身份有意不打印：在这里算它意味着用 `face_views` 不采用的那种拼法，从一个 `PathBuf`
        // 复现内核的 `(命名空间, 相对路径, 声明名)` 三元组——而下面的钉子恰好抓到了这一点（它打印的数字
        // 并不是树报告的那个）。诊断里的错数字比没有数字更糟，因此回复只点名**旧**身份——graft 记录正是
        // 以它为键——并指向列出后果的工具。
        message.push_str(&format!(
            "\nidentity changed: `kind` `{previous_kind}` → `{new_kind}` is an identity input \
             (`NodeId = hash(namespace, source, name)`, and a face's name is its kind), so this face \
             is no longer `{id}`; graft records keyed by the old identity no longer resolve — \
             `xirang.diff {{\"records\": true}}` lists them\n"
        ));
    }
    Ok(Outcome {
        message,
        source: change.source,
        declaration: None,
        moved: false,
        alternative: false,
        inherited: Vec::new(),
        consumers: Vec::new(),
    })
}

/// Add a layer **inside** one face: a parts type the face owns, plus an accessor.
/// 在一个面**内部**加一层：它自己拥有的零件类型，以及一个访问器。
///
/// The measured reason this exists: family b of the round-5 evaluation asked for "this object is
/// not deep enough inside", and the only write the bridge had (`add`) works on the registration
/// tree — so "deeper" could only be read as "another face under it". That reading moves four
/// families of factory-shape assertions and adds a segment to the public module path, so the arm
/// that kept the gate green delivered nothing and wrote an essay about why. This action adds the
/// layer the object's own contract already talks about, touches exactly one file, and leaves the
/// tree, the public paths and the pins alone.
/// 这个动作的存在有实测原因：第五轮五族 b 的题面是"这个对象内部还不够"，而桥当时唯一的写
/// （`add`）作用在注册树上——于是"更深"只能被读成"它下面再挂一个面"。那条路会移动四类按出厂形状
/// 钉死的断言、并让公开模块路径多一段，于是守住门的那一臂交不出东西、只写了一篇"为什么不可能"。
/// 本动作加的是对象自己契约已经说到的那一层，只碰一个文件，树、公开路径与钉子都不动。
fn run_deepen(
    root: &Path,
    project: &Path,
    namespace: &str,
    arguments: &Value,
) -> Result<Outcome, String> {
    let target = arguments
        .get("node")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            format!(
                "deepen requires `node`: the face, by logical path or identity\n{}",
                write_example(root, namespace, Action::Deepen)
            )
        })?;
    let parts = arguments
        .get("inside")
        .and_then(|inside| inside.get("parts"))
        .and_then(Value::as_object)
        .filter(|parts| !parts.is_empty())
        .ok_or_else(|| {
            format!(
                "deepen requires `inside.parts`: an object of `field: Type` pairs, at least one\n{}",
                write_example(root, namespace, Action::Deepen)
            )
        })?;
    let mut fields: Vec<(String, String)> = Vec::new();
    for (name, value) in parts {
        let named_well = !name.is_empty()
            && !name.starts_with(|character: char| character.is_ascii_digit())
            && name.chars().all(|character| {
                character == '_' || character.is_ascii_lowercase() || character.is_ascii_digit()
            });
        if !named_well {
            return Err(format!(
                "`{name}` is not a snake_case field name; this action writes Rust, so the names it \
                 writes have to be Rust names"
            ));
        }
        let kind = value
            .as_str()
            .ok_or_else(|| format!("`{name}` must be a type string, like `String`"))?
            .trim();
        if kind.is_empty() || kind.contains([';', '{', '}', '#', '\n']) {
            return Err(format!(
                "`{name}: {kind}` is not a type this action writes; it writes a simple type \
                 (`String`, `&'static str`, an integer, a float, a bool)"
            ));
        }
        fields.push((name.clone(), kind.to_owned()));
    }
    let _registry = load_registry(root, namespace)?;
    let id = resolve_node(root, namespace, target)?;
    let faces = face_views(root, namespace)?;
    let face = faces.iter().find(|face| face.id == id).ok_or_else(|| {
        format!("`{target}` does not name a face in this tree, so there is nothing to deepen")
    })?;
    let kind = face.kind.clone();
    let path = root.join("src").join(&face.source);
    // Every message names the file **relative to the package**, because a preview's absolute path is
    // a throwaway copy deleted a moment later — and the write path's own rule is that a preview must
    // never tell a caller to look somewhere that will not exist. An independent review of the first
    // cut caught this by quoting the `/tmp/xirang-toolchain-preview-…` path out of a refusal.
    // 所有文案都用**相对包**的路径：预览的绝对路径是一份随后就被删掉的一次性副本，而写入路径自己的
    // 规矩就是"预览绝不能叫调用方去看一个随后不存在的目录"。第一版的独立复核正是从一条拒绝文案里
    // 把 `/tmp/xirang-toolchain-preview-…` 抄出来抓到这个的。
    let named = format!("src/{}", face.source);
    let text = std::fs::read_to_string(&path)
        .map_err(|error| format!("{named} is not readable: {error}"))?;
    let marker = format!("pub struct {kind};");
    let markers = text.lines().filter(|line| line.trim() == marker).count();
    if markers != 1 {
        return Err(format!(
            "deepen needs the line `{marker}` in {named} exactly once to hold the layer, and found \
             it {markers} time(s); this action deepens a face whose shape it can read rather than \
             guessing at another shape"
        ));
    }
    let listed = fields
        .iter()
        .map(|(name, _)| format!("\"{name}\""))
        .collect::<Vec<_>>()
        .join(", ");
    let declarations = fields
        .iter()
        .map(|(name, held_kind)| {
            format!(
                "    /// The {name} this face carries.\n    /// 这个面携带的 {name}。\n    \
                 pub {name}: {held_kind},\n"
            )
        })
        .collect::<String>();
    let held = format!("pub struct {kind} {{\n    parts: {kind}Parts,\n}}");
    let mut text = text.replacen(&marker, &held, 1);
    text.push_str(&format!(
        "\n/// The layer inside this face: the parts its own declaration already describes.\n\
         /// 这个面内部的那一层：它自己的声明已经描述过的那些零件。\npub struct {kind}Parts {{\n\
         {declarations}}}\n\
         \nimpl xirang_toolchain::run_method::PartsContract for {kind}Parts {{\n\
         \x20   type Output = {kind}Parts;\n\
         \x20   const PROVIDED_PARTS: &'static [&'static str] = &[{listed}];\n}}\n\
         \nimpl {kind} {{\n\
         \x20   /// The layer this face carries.\n\
         \x20   /// 这个面携带的那一层。\n\
         \x20   pub fn parts(&self) -> &{kind}Parts {{\n\
         \x20       &self.parts\n\
         \x20   }}\n}}\n"
    ));
    crate::run_method::authoring::write_transaction::write_one(&path, text.as_bytes())?;
    Ok(Outcome {
        message: format!(
            "deepened {}: `{kind}` now holds `{kind}Parts` with {} part(s) ({listed}); its \
             declaration, its public path, its tree row and the factory-shape pins were not \
             touched — `check {{face}}` is what shows those pins still hold",
            face.path,
            fields.len()
        ),
        source: path,
        declaration: None,
        moved: false,
        alternative: true,
        inherited: Vec::new(),
        consumers: consumer_diff(root, project, namespace, &face.path, &kind, &fields),
    })
}

pub(crate) const REJECTED_VERDICT: &str = "kernel verdict: this package's own faces are rejected";

/// Build the registry the executor validates against: the package's own faces,
/// under the namespace Cargo reports.
/// 构建执行器据以校验的注册树：该包自己的注册面，位于 Cargo 报告的命名空间之下。
pub(crate) fn load_registry(root: &Path, namespace: &str) -> Result<Registry, String> {
    let mut registry =
        Registry::root_for_namespace(xirang_kernel::FrameworkId::new("xirang.mcp"), namespace);
    let layout = source_layout(root)?;
    let snapshots = AuthoringContext::new(root.to_path_buf(), namespace.to_owned())
        .scope(|| crate::run_method::generated_snapshots_in(&layout))?;
    registry
        .register_snapshot_batch(snapshots)
        .map_err(|error| format!("the package's own faces were rejected: {error}"))?;
    Ok(registry)
}

#[path = "apply_report.rs"]
mod apply_report;

use apply_report::report;

#[path = "apply_family.rs"]
mod apply_family;

#[path = "apply_notes.rs"]
mod apply_notes;

#[path = "apply_refusals.rs"]
mod apply_refusals;

use apply_refusals::{consumer_diff, refused_with_a_way_forward, write_example};

#[cfg(test)]
#[path = "apply_tests.rs"]
pub(crate) mod apply_tests;

#[cfg(test)]
#[path = "apply_consumers_tests.rs"]
mod apply_consumers_tests;

#[cfg(test)]
#[path = "island_tests.rs"]
mod island_tests;

#[cfg(test)]
#[path = "island_scope_tests.rs"]
mod island_scope_tests;
