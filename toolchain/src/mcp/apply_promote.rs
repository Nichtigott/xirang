//! The `promote` action: land a confirmed external graft into the base tree.
//! `promote` 动作：把一条已确认的外部 graft 落地进基树。
//!
//! Why this is its own module: `apply.rs` is a ratcheted file (600 code lines) and every action
//! that grows lives beside it, the same reason `apply_cut.rs` does.
//! 为什么单独一个模块：`apply.rs` 是受棘轮约束的文件（600 代码行），而每个会长大的动作都住在它旁边
//! ——与 `apply_cut.rs` 同一个理由。
//!
//! **What landing means here.** A graft record is a *runtime* overlay: the base tree keeps its
//! declaration and the record re-points the slot at an external implementation while the host runs.
//! Promoting makes that permanent in the source: the target face's declaration is rewritten to
//! carry the external implementation's fields, the declaration entry that handed the slot over is
//! retired, and the record moves to the trash. The rewrite goes through **the same executor
//! `apply edit` uses** (`runtime::edit_module_face`), so the host-form declaration, the field order
//! and the kind migration are the ones this workspace already ships rather than a second renderer
//! written here.
//! **落地在这里的含义。** graft 记录是一层*运行期*覆盖：基树保留自己的声明，而记录在宿主运行时把槽位
//! 指向外部实现。落地让它固化进源码：目标面的声明被改写成携带外部实现的字段，交出该槽位的那条声明条目
//! 被退役，记录移入回收目录。这次改写走**`apply edit` 用的同一个执行器**（`runtime::edit_module_face`），
//! 因此宿主形式的声明、字段顺序与 kind 迁移都是本工作区已经发货的那一套，而不是在这里另写一个渲染器。
//!
//! **Refusals, and why they are refusals.** The executor only rewrites files **it generated** — a
//! hand-written face file is refused by name rather than rewritten into a shape its author never
//! wrote. Fields that name a path *inside the declaring tree* (`admission`, a non-default
//! registration rule) cannot move between trees at all: the external spelling means nothing here.
//! And the record is read from the **project** root, never from a preview's copy, because
//! `copy_package` skips `.xirang/` — a preview that read the copy would report a record it never
//! saw.
//! **拒绝，以及为什么拒绝。** 执行器只改写**它生成过的**文件——手写的面文件会被点名拒绝，而不是被改写成
//! 作者从未写过的形状。命名*声明树内部*路径的字段（`admission`、非默认的注册规则）根本无法跨树搬移：
//! 外部的那套拼法在这里不指任何东西。而记录永远从**项目**根读取，而不是预览的副本，因为 `copy_package`
//! 会跳过 `.xirang/`——读副本的预览会报告一条它从未见过的记录。

use std::path::{Path, PathBuf};

use serde_json::Value;
use xirang_kernel::tree::graft_ops::{RecordReport, RecordedGraft, ResolvedRecord};

use crate::mcp::apply::{Outcome, load_registry};
use crate::run_method::AuthoringContext;

#[path = "apply_promote/source.rs"]
mod source;

use source::{External, crate_and_module, external_file, external_registry, external_root};

/// Land one external graft record, previewing unless the caller is applying.
/// 把一条外部 graft 记录落地；除非调用方在落盘，否则只预览。
pub(crate) fn run_promote(
    root: &Path,
    work: &Path,
    applying: bool,
    arguments: &Value,
) -> Result<Outcome, String> {
    let selector = text(arguments, "selector")
        .map_err(|error| format!("{error}\n{}", promote_example(root)))?;
    // The record is a source rewrite plus a record retirement, so the request says `confirm`
    // itself — the same shape `delete` uses, for the same reason: this is the operation whose
    // preview a caller can step past by accident.
    // 这是一次源码改写加一次记录退役，因此请求自己说出 `confirm`——与 `delete` 同一种形状、同一个理由：
    // 这是调用方可能不小心跨过其预览的那个操作。
    if arguments.get("confirm").and_then(Value::as_bool) != Some(true) {
        return Err(format!(
            "promote requires `confirm: true`: it rewrites the host's source and retires the \
             record, and a preview is the only place to read what it would do\n{}",
            promote_example(root)
        ));
    }
    // Read from the project root: a preview's copy has no `.xirang/`, so reading `work` would
    // answer as if this package had no records at all.
    // 从项目根读取：预览的副本里没有 `.xirang/`，读 `work` 会答成"这个包根本没有记录"。
    let document = crate::run_method::load_graft_record(root, &selector).map_err(|error| {
        format!(
            "{error}; the record is `.xirang/external-grafts/{selector}/graft.plan` under the \
             project root this call resolved"
        )
    })?;
    let recorded = RecordedGraft::new(selector.clone(), document.clone());
    // The namespace comes from the **project**, not from the work directory: in a preview the work
    // directory is a throwaway copy in the temp directory, so any relative `path` dependency in the
    // host's manifest points at something that does not exist beside that copy and `cargo metadata`
    // cannot name the package at all. Measured by the partition rehearsal: a host that depends on its
    // implementation crate by path could not be previewed at all ("cannot learn the identity
    // namespace"), while the apply half — which runs in the project itself — worked, so the defect only
    // ever showed up in previews. Every other action already asks `root`; this one asked the copy.
    // 命名空间取自**项目**，而不是工作目录：预览时工作目录是临时目录里的一次性副本，因此宿主清单里任何相对的
    // `path` 依赖都指向那个副本旁边并不存在的东西，`cargo metadata` 根本说不出包名。由分区演练实测：一个按
    // path 依赖其实现 crate 的宿主**完全无法预览**（"cannot learn the identity namespace"），而落盘那一半
    // ——它就在项目里跑——是好的，因此这个缺陷只在预览里现形。其它动作早就是问 `root`，只有它问的是副本。
    let namespace = crate::mcp::registry::namespace(root)?;
    let registry = load_registry(work, &namespace)?;
    let resolved = registry
        .resolve_record(&recorded)
        .map_err(|error| format!("this record does not resolve against the tree: {error}"))?;
    let (slot, path) = match resolved {
        ResolvedRecord::Slot {
            slot,
            path,
            reports,
            ..
        } => {
            if let Some(drifted) = reports
                .iter()
                .find(|report| matches!(report, RecordReport::IdentityDrifted { .. }))
            {
                return Err(format!(
                    "this record's target identity no longer resolves ({drifted}); \
                     `xirang.diff {{\"records\": true}}` lists what moved, and re-identifying the \
                     record is a decision rather than something this action may guess"
                ));
            }
            (slot, path)
        }
        ResolvedRecord::Unkept { reports } => {
            return Err(format!(
                "no declaration keeps this record's slot alive ({reports:?}), so there is nothing \
                 to land: the slot is what a `cut(…)` in the host entry hands over"
            ));
        }
    };
    let faces = crate::build_method::face_views(work, &namespace)?;
    let face = faces
        .iter()
        .find(|face| face.id == slot)
        .ok_or_else(|| format!("`{path}` is not a face this tree derives"))?;
    let declared = crate::build_method::declared_grafts(work).map_err(|error| {
        format!("this tree's graft plan is unreadable, so there is nothing to retire ({error})")
    })?;
    let cut = declared
        .cuts
        .iter()
        .find(|cut| cut.names_face(&path, Some(&face.module)))
        .ok_or_else(|| {
            format!(
                "no declaration hands over `{path}`, so this record has nothing to land into — \
                 `.xirang/external-grafts/` is not the host entry"
            )
        })?;
    let (crate_path, module) = crate_and_module(cut.expressions.as_ref().ok_or_else(|| {
        format!(
            "the declaration for `{path}` names its implementation by string (`cut \"…\" graft \
             \"{}\"`), and a string names nothing about *where* that implementation lives, so \
             there is no source to land. This action cannot take `implementation` for it either: \
             that argument locates the crate a **Rust path** already named, and there is no path \
             here to locate. Way forward: redeclare the slot as a Rust path — \
             `cut(<face>::NODE_ID) graft(<crate>::<module>::NODE_ID)` — and run this again",
            cut.graft
        )
    })?)?;
    let external_root = external_root(root, arguments, &crate_path)?;
    let external_file = external_file(&external_root, &module)?;
    let external_source = std::fs::read_to_string(&external_file).map_err(|error| {
        format!(
            "the external implementation {} is unreadable: {error}",
            external_file.display()
        )
    })?;
    let external_face = xirang_kernel::syntax::parse_face(&external_source)
        .map_err(|error| {
            format!(
                "the external implementation {} does not parse: {}",
                external_file.display(),
                error.message
            )
        })?
        .ok_or_else(|| {
            format!(
                "{} declares no registration face; this action lands one declaration into one \
                 face, so a file with no face (or several) is not a shape it rewrites",
                external_file.display()
            )
        })?;
    // The selector has to name *this* face: it is the record's own word for the implementation, and
    // a file whose face is called something else would silently land the wrong one.
    // 选择器必须点名**这个**面：它是记录自己对实现的称呼，而一个面叫别的名字的文件会静默落地错的那个。
    let named = [
        module.rsplit("::").next().unwrap_or(&module).to_owned(),
        external_face.field("kind").unwrap_or_default(),
        external_face.field("registry_name").unwrap_or_default(),
    ];
    if !named.iter().any(|name| name == &document.graft) {
        return Err(format!(
            "the record selects `{}`, and {} declares a face named {}; this action will not land \
             a face the record did not select",
            document.graft,
            external_file.display(),
            named
                .iter()
                .filter(|name| !name.is_empty())
                .cloned()
                .collect::<Vec<_>>()
                .join(" / ")
        ));
    }
    // `flow` comes from the typed snapshot (see `external_registry`); everything else comes from
    // the declaration's own spellings.
    // `flow` 来自类型化快照（见 `external_registry`），其余字段来自声明自己的拼法。
    let kind = external_face.field("kind").unwrap_or_default();
    let external = external_registry(&external_root)?;
    let snapshot = external
        .depth_first()
        .into_iter()
        .find(|snapshot| snapshot.kind == kind)
        .ok_or_else(|| {
            format!(
                "{} declares `kind: {kind}`, which the external crate's own registration does not \
                 carry; this action reads the declaration and the registration, and they disagree",
                external_file.display()
            )
        })?;
    let flow = if snapshot.flow.id.is_empty() {
        String::new()
    } else {
        format!(
            "{}|{}|{}|{}",
            snapshot.flow.id, snapshot.flow.version, snapshot.flow.input, snapshot.flow.output
        )
    };
    let replacement = External::from_syntax(&external_face, &external_file, flow)?;
    let context = AuthoringContext::new(work.to_path_buf(), namespace.clone());
    // The executor writes the face, and the entry is retired after it — so a refusal in the second
    // step would leave a tree that is half-landed: a face whose identity already moved, and a plan
    // still handing its old slot over. The landing is therefore rolled back by hand if anything
    // after it fails, and the reply says nothing was written.
    // 执行器先写面，条目在其后被退役——因此第二步被拒会留下一棵**半落地**的树：身份已经变了的签名面，
    // 而计划仍在交出自己的旧槽位。因此落地之后任何一步失败都要把它手工撤回，并在回复里说明什么都没写。
    let face_before = std::fs::read_to_string(
        faces
            .iter()
            .find(|face| face.id == slot)
            .map(|face| work.join("src").join(&face.source))
            .ok_or_else(|| format!("`{path}` has no source file"))?,
    )
    .map_err(|error| format!("the face's own file is unreadable: {error}"))?;
    let (change, previous_kind) = context
        .scope(|| -> Result<_, String> {
            let mut authored = crate::run_method::authored_face(&registry, slot)?;
            let previous = authored.kind.clone();
            overlay(&mut authored, &replacement);
            let change =
                crate::run_method::edit_module_face(&registry, slot, &authored.as_patch())?;
            Ok((change, previous))
        })
        .map_err(|error| {
            format!(
                "{error}\nway forward: this action rewrites the declaration through the authoring \
                 executor, which only rewrites files it generated. A hand-written face file has to \
                 be landed by hand (the external declaration's fields are reported below), or \
                 scaffolded through `apply add` / `new_project` in the first place"
            )
        })?;
    let retired = match repoint_entry(work, &declared.entry, cut, applying) {
        Ok(retired) => retired,
        Err(refused) => {
            // Nothing about the retirement is on disk (its write happens after its own read-back),
            // so undoing the face rewrite is enough to return this tree to what it was.
            // 退役本身什么都没落到盘上（它的写入发生在自己的读回之后），因此撤掉面的那次改写就足以让这棵树
            // 回到原样。
            crate::run_method::authoring::write_transaction::write_one(
                &change.source,
                face_before.as_bytes(),
            )
            .map_err(|error| format!("{refused}; and undoing the face rewrite failed: {error}"))?;
            return Err(format!(
                "{refused}\nnothing was written: the face rewrite was undone, and the record is \
                 still in place"
            ));
        }
    };
    let mut message = format!(
        "landed `{}` into `{path}`: the declaration in {} now carries the external \
         implementation's fields, the entry in {} now points that slot at **this face itself** \
         (so the plan still names it — that naming is what keeps a face in the generated tree), \
         and the record {}\n{retired}",
        document.graft,
        change.source.display(),
        declared.entry.display(),
        if applying {
            "moved to `.xirang/trash/external-grafts/`"
        } else {
            "would move to `.xirang/trash/external-grafts/`"
        }
    );
    if applying {
        let trash = retire_record(root, &selector)?;
        message.push_str(&format!("\nrecord moved to {}\n", trash.display()));
    }
    if previous_kind != replacement.kind {
        // `kind` is an identity input, so this is the one field whose move has fallout the caller
        // has to handle: the marker type is renamed by the executor, and every mention of the old
        // name elsewhere in the package stops compiling.
        // `kind` 是身份输入，因此这是唯一一个搬移后有调用方必须处理的后果的字段：标记类型会被执行器改名，
        // 而这个包里别处每一处旧名字都会编译不过。
        message.push_str(&format!(
            "note   the kind moved: `{previous_kind}` → `{}`. `kind` is an identity input \
             (`NodeId = hash(namespace, source, name)`), so this face is no longer `{slot}` — \
             references to the old type have to be updated by hand, and \
             `xirang.diff {{\"records\": true}}` lists records keyed by the old identity\n",
            replacement.kind
        ));
    }
    // The one thing this landing deliberately does **not** take from the implementation, said out loud.
    // Silently keeping the host's answer would be a self-description that does not match what happened;
    // refusing instead would reject a landing that works, because a face's children belong to the tree it
    // sits in, not to the crate its fields came from.
    // 这次落地**有意不**从实现那里拿走的那一件事，明说。静默保留宿主的答案会是一句与事实不符的自我描述；
    // 而改成拒绝，会拒掉一次本来能工作的落地——因为一个面的子面属于它所在的那棵树，不属于它字段的来源 crate。
    if replacement.needs_registry && !slot_carries_a_registry_rule(work, &face.source) {
        message.push_str(
            "note   the implementation owns a registry (`needs_registry: true`) and this slot does \
             not: its rule would come from a `registry_rule` module beside the face, this tree has \
             none there, and a land does not move the implementation's children — so the landed face \
             keeps the slot's own answer. A face that needs a registry here has to be landed into a \
             slot that carries one\n",
        );
    }
    Ok(Outcome {
        message,
        declaration: None,
        source: change.source,
        moved: true,
        alternative: false,
        inherited: Vec::new(),
        consumers: Vec::new(),
    })
}

/// A complete promote request, with one of this package's real record directories in it.
/// 一个完整的 promote 请求，里面填的是这个包里**真实存在**的一个记录目录。
///
/// A refusal that offers `<selector>` makes the reader go and list the directory before the request
/// becomes a call; this reads the directory it is standing in. `add`'s example has the same rule —
/// see `apply::write_example` — and here the values are the only ones the tool needs.
/// 一个只给出 `<selector>` 的拒绝，会让读者在请求变成调用之前先去列一遍目录；这里读的就是它所在的那个目录。
/// `add` 的示例遵循同一条规则——见 `apply::write_example`——而这里工具需要的值就这一个。
fn promote_example(root: &Path) -> String {
    let selector = std::fs::read_dir(
        root.join(xirang_kernel::lexicon::XIRANG_DIR)
            .join(xirang_kernel::lexicon::EXTERNAL_GRAFT_DIR),
    )
    .ok()
    .and_then(|entries| {
        entries
            .flatten()
            .filter(|entry| entry.path().is_dir())
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .min()
    })
    .unwrap_or_else(|| "<selector>".to_owned());
    format!(
        "accepted shape  {{\"action\":\"promote\",\"selector\":\"{selector}\",\"apply\":true,\
         \"confirm\":true}}"
    )
}

/// The one required string argument named `key`.
/// 名为 `key` 的那一个必填字符串参数。
fn text(arguments: &Value, key: &str) -> Result<String, String> {
    match arguments.get(key).and_then(Value::as_str) {
        Some(value) if !value.trim().is_empty() => Ok(value.trim().to_owned()),
        Some(_) => Err(format!("`{key}` must not be empty")),
        None => Err(format!(
            "promote requires `{key}`: the `.xirang/external-grafts/<selector>/` directory to \
             land"
        )),
    }
}

/// Overlay the external face's fields onto the host face, keeping what belongs to the host.
/// 把外部的面的字段覆盖到宿主的面上，同时保留属于宿主的东西。
///
/// `module` and `parent` stay the host's: the landed face occupies the slot the base tree already
/// has, which is what the runtime overlay does with the same fields. `kind` **is** copied, because
/// the runtime overlay replaces it too — and the executor migrates a kind change properly (marker
/// type, declaration, and the identity it feeds), which is why the reply warns about identity.
/// `module` 与 `parent` 保留宿主的：落地的面占据基树本来就有的那个槽位，而运行期覆盖对同样的字段也是
/// 这么做的。`kind` **会被复制**，因为运行期覆盖也替换它——而执行器会正确地迁移 kind 变化（标记类型、
/// 声明，以及它喂给的身份），这也是回复要警告身份变化的原因。
fn overlay(authored: &mut crate::run_method::AuthoredFace, replacement: &External) {
    authored.kind = replacement.kind.clone();
    authored.preset = replacement.preset.clone();
    authored.parts = replacement.parts.clone();
    authored.name_zh = replacement.name_zh.clone();
    authored.name_en = replacement.name_en.clone();
    authored.summary_zh = replacement.summary_zh.clone();
    authored.summary_en = replacement.summary_en.clone();
    authored.exports = replacement.exports.clone();
    authored.stable_name = replacement.stable_name.clone();
    // `needs_registry` and `registration_rule` stay the host's, and they are the two fields whose move
    // would break the tree rather than the face. A face that owns a registry reads its rule from the
    // `registry_rule` module **beside it**, and this action lands one declaration into one slot: it does
    // not move the implementation's children, so whether this position has a registry is a fact about the
    // **host's tree** and not about the crate the implementation came from. Measured on a hand-built
    // project: copying them made the host stop compiling — `phase=face-rule … expected=board/object/
    // tile2/registry_rule/registry_rule.rs, actual=no sibling registry_rule module`, reported through the
    // generated tree's `compile_error!` — while the landed face itself was correct (audit `M7`, §M7.57).
    // `needs_registry` 与 `registration_rule` 保留宿主的，而它们正是"搬过去会弄坏**树**而不只是面"的两个字段。
    // 拥有注册机的面从**它旁边**的 `registry_rule` 模块读规则，而本动作只把一条声明落进一个槽位：它不搬实现
    // 的子面，因此"这个位置有没有注册机"是关于**宿主那棵树**的事实，而不是关于实现来自哪个 crate 的事实。
    // 在一个手工项目上实测：复制它们会让宿主停止编译——`phase=face-rule … expected=board/object/tile2/
    // registry_rule/registry_rule.rs, actual=no sibling registry_rule module`，经生成树的 `compile_error!`
    // 报出——而落地的面本身是对的（审计 `M7`，§M7.57）。
    authored.handle_traits = replacement.handle_traits.clone();
    authored.handle_contracts = replacement.handle_contracts.clone();
    authored.part_traits = replacement.part_traits.clone();
    authored.part_contracts = replacement.part_contracts.clone();
    authored.requires = replacement.requires.clone();
    authored.provides = replacement.provides.clone();
    authored.runtime_checks = replacement.runtime_checks.clone();
    authored.flow = replacement.flow.clone();
    authored.flow_provider = replacement.flow_provider.clone();
}

/// Whether the slot a landing targets carries the `registry_rule` module beside it.
/// 落地的槽位旁边是否带着那个 `registry_rule` 模块。
///
/// The same layout the build's `face-rule` phase judges, asked here because this is the last moment a
/// reader can be told: after the rewrite, the generated tree reports it as a `compile_error!` inside a
/// file the author cannot edit (measured, audit `M7`, §M7.57).
/// 这是构建的 `face-rule` 阶段所判的同一套布局，在这里问是因为这是读者还能被告知的最后时刻：改写之后，生成树
/// 会把它报成一份作者改不了的 `compile_error!`（实测，审计 `M7`，§M7.57）。
fn slot_carries_a_registry_rule(work: &Path, source: &str) -> bool {
    Path::new(source).parent().is_some_and(|directory| {
        work.join("src")
            .join(directory)
            .join(crate::build_method::REGISTRY_RULE_MODULE)
            .join(format!("{}.rs", crate::build_method::REGISTRY_RULE_MODULE))
            .is_file()
    })
}

/// Point the declaration entry at the face itself, and read the result back.
/// 把那条声明条目改指向这个面自己，并把结果读回来。
///
/// **Why the entry is rewritten rather than removed.** In a host that declares a plan, being
/// named by the plan is the only way a face enters the generated tree — so deleting the entry
/// does not "clean up the graft", it **drops the face out of the build** (measured: the generated
/// plan mounts the neighbours and not this face, and `source_scope` counts one fewer). The
/// external implementation is what has to go, and the expressible form that keeps the naming is
/// the self-graft: `cut(<expr>) graft(<expr>)`. A bare `cut(…)` is not legal syntax, so this is
/// not a stylistic choice.
/// **为什么改写条目而不是删掉它。** 在声明了计划的宿主里，"被计划点名"是面进入生成树的唯一方式——
/// 因此删条目不是"清理掉 graft"，而是**把那个面从构建里丢掉**（实测：生成计划挂载邻居而不挂载它，
/// `source_scope` 少一个）。要走的是那份外部实现，而保住点名的可表达形式就是自嫁接
/// `cut(<expr>) graft(<expr>)`。裸 `cut(…)` 不是合法语法，所以这不是风格选择。
///
/// The rewrite is decided by the kernel's own parser: the source before and after must parse, the
/// entry count must be **unchanged**, the promoted entry must name the cut expression, and every
/// other entry must be byte-identical in what the parser reports. A rewrite that reformatted,
/// reordered or dropped a neighbour is refused before it reaches the author's file.
/// 这次改写由内核自己的解析器裁决：改前与改后的源码都必须解析，条目数必须**不变**，被改的那条必须点名
/// 切口表达式，而其余条目在解析器报出的内容上必须逐字未变。重排、改动格式或丢掉邻居的改写会在抵达作者
/// 文件之前被拒绝。
fn repoint_entry(
    work: &Path,
    entry: &Path,
    cut: &crate::build_method::DeclaredGraft,
    applying: bool,
) -> Result<String, String> {
    let source = std::fs::read_to_string(entry)
        .map_err(|error| format!("{} is not readable: {error}", entry.display()))?;
    let before = xirang_kernel::syntax::entries::graft_entries(&source).map_err(|error| {
        format!(
            "the entry does not parse before the write: {}",
            error.message
        )
    })?;
    let index = before
        .iter()
        .position(|item| item.cut == cut.cut && item.graft == cut.graft)
        .ok_or_else(|| {
            format!(
                "the declaration at {}:{} (`cut({}) graft({})`) is not in the parsed entry list, so \
                 this action will not guess which text to rewrite",
                entry.display(),
                cut.line,
                cut.cut,
                cut.graft
            )
        })?;
    let edited = repoint_graft(&source, &cut.cut, &cut.graft)?;
    let after = xirang_kernel::syntax::entries::graft_entries(&edited)
        .map_err(|error| format!("the rewritten entry does not parse: {}", error.message))?;
    if after.len() != before.len() {
        return Err(format!(
            "the rewrite left {} entries where it should have left {}: the plan's naming is what \
             keeps a face in the generated tree, so this action never removes one",
            after.len(),
            before.len()
        ));
    }
    for (position, (kept, want)) in after.iter().zip(before.iter()).enumerate() {
        if position == index {
            if kept.graft != want.cut {
                return Err(format!(
                    "the rewritten entry names `{}` as the implementation where it should name \
                     its own cut expression `{}`",
                    kept.graft, want.cut
                ));
            }
            continue;
        }
        if kept.cut != want.cut || kept.graft != want.graft || kept.full != want.full {
            return Err(format!(
                "the rewrite changed an entry it was not asked to touch (`{}` graft `{}`)",
                want.cut, want.graft
            ));
        }
    }
    if applying {
        // The backup is written before the new text, and the read-back above ran on the text we
        // are about to write — so a refusal cannot leave half an entry behind.
        // 备份先于新文本写下，而上面的读回是在"即将写入的文本"上跑的——因此拒绝不会留下半条条目。
        backup(work, entry)?;
    }
    crate::run_method::authoring::write_transaction::write_one(entry, edited.as_bytes())?;
    Ok(format!(
        "the entry at {}:{} now reads `cut({}) graft({})`: the plan still names this face, and the \
         declaration no longer names the external crate ({} → {} entries, unchanged)",
        entry.display(),
        cut.line,
        cut.cut,
        cut.cut,
        before.len(),
        after.len()
    ))
}

/// Rewrite one entry's implementation expression to the cut expression it is paired with.
/// 把某一条条目的实现表达式改写成与它配对的切口表达式。
///
/// Located in the bytes, not by line: `GraftSyntax::location` reports line 1 for an entry inside
/// `static_graft_plan!`, so line arithmetic would rewrite the wrong text. What was found is checked
/// against the expressions the parser reported, and the result is re-parsed by the caller.
/// 在字节里定位，不按行：`static_graft_plan!` 里条目的 `GraftSyntax::location` 报的是第 1 行，按行算会改
/// 错文本。找到的东西要与解析器报出的表达式核对，而结果由调用方重新解析。
fn repoint_graft(source: &str, cut: &str, graft: &str) -> Result<String, String> {
    if let Some(edited) = xirang_kernel::syntax::repoint_native_graft(source, cut, graft)
        .map_err(|error| error.to_string())?
    {
        return Ok(edited);
    }
    let (open, close) = graft_group(source, cut, graft)?;
    let mut edited = String::with_capacity(source.len());
    edited.push_str(&source[..open]);
    edited.push_str(cut);
    edited.push_str(&source[close..]);
    Ok(edited)
}

/// The parentheses of the `graft(…)` group that belongs to the entry with these expressions.
/// 属于这两个表达式那条条目的 `graft(…)` 组的括号位置。
fn graft_group(source: &str, cut: &str, graft: &str) -> Result<(usize, usize), String> {
    let bytes = source.as_bytes();
    let wanted_cut = squeeze(cut);
    let wanted_graft = squeeze(graft);
    let mut index = 0;
    while let Some(found) = source[index..].find("cut") {
        let start = index + found;
        index = start + 3;
        if start > 0 && is_word_byte(bytes[start - 1]) {
            continue;
        }
        let Some(open) = source[start + 3..].find('(').map(|at| start + 3 + at) else {
            continue;
        };
        if !source[start + 3..open].trim().is_empty() {
            continue;
        }
        let Some(cut_end) = matching(source, open) else {
            continue;
        };
        if squeeze(&source[open + 1..cut_end]) != wanted_cut {
            continue;
        }
        let Some(graft_at) = source[cut_end + 1..]
            .find("graft")
            .map(|at| cut_end + 1 + at)
        else {
            continue;
        };
        let Some(gopen) = source[graft_at + 5..].find('(').map(|at| graft_at + 5 + at) else {
            continue;
        };
        if !source[graft_at + 5..gopen].trim().is_empty() {
            continue;
        }
        let Some(gclose) = matching(source, gopen) else {
            continue;
        };
        if squeeze(&source[gopen + 1..gclose]) != wanted_graft {
            continue;
        }
        return Ok((gopen + 1, gclose));
    }
    Err(format!(
        "the parsed declaration `cut({cut}) graft({graft})` is not in the entry file; this action \
         will not rewrite a declaration it cannot point at"
    ))
}

/// The byte after the group that `open` opens, or `None` when it never closes.
/// `open` 打开的组之后的那个字节；永不闭合时为 `None`。
fn matching(source: &str, open: usize) -> Option<usize> {
    let mut depth = 0usize;
    for (offset, character) in source[open..].char_indices() {
        match character {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return Some(open + offset);
                }
            }
            _ => {}
        }
    }
    None
}

/// Source text with every whitespace run collapsed, for comparing two spellings of one expression.
/// 把每一段空白折叠后的源码文本，用于比较同一个表达式的两种拼法。
fn squeeze(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Whether a byte can be part of a Rust word.
/// 一个字节是否可以属于一个 Rust 词。
fn is_word_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

/// Copy the file being rewritten into the project's trash before it changes.
/// 在改写之前，把即将被改写的文件复制进项目的回收目录。
fn backup(work: &Path, file: &Path) -> Result<(), String> {
    let relative = file.strip_prefix(work).unwrap_or(file);
    let trash = work
        .join(xirang_kernel::lexicon::XIRANG_DIR)
        .join("trash")
        .join("promoted")
        .join(stamp().to_string())
        .join(relative);
    if let Some(parent) = trash.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("cannot create {}: {error}", parent.display()))?;
    }
    std::fs::copy(file, &trash)
        .map_err(|error| format!("cannot back {} up: {error}", file.display()))?;
    Ok(())
}

/// Move the record directory into the trash, the same discipline `remove_external_graft` uses.
/// 把记录目录移进回收目录——与 `remove_external_graft` 相同的回收纪律。
fn retire_record(root: &Path, selector: &str) -> Result<PathBuf, String> {
    let from = root
        .join(xirang_kernel::lexicon::XIRANG_DIR)
        .join(xirang_kernel::lexicon::EXTERNAL_GRAFT_DIR)
        .join(selector);
    let to = root
        .join(xirang_kernel::lexicon::XIRANG_DIR)
        .join("trash")
        .join(xirang_kernel::lexicon::EXTERNAL_GRAFT_DIR)
        .join(format!("{selector}-{}", stamp()));
    if let Some(parent) = to.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("cannot create XiRang trash: {error}"))?;
    }
    std::fs::rename(&from, &to).map_err(|error| {
        format!(
            "cannot move {} to {}: {error}",
            from.display(),
            to.display()
        )
    })?;
    Ok(to)
}

/// A nanosecond stamp, for a trash path that does not collide.
/// 一个纳秒时间戳，用于不会撞车的回收路径。
fn stamp() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| elapsed.as_nanos())
        .unwrap_or(0)
}

#[cfg(test)]
#[path = "apply_promote_tests.rs"]
mod apply_promote_tests;
