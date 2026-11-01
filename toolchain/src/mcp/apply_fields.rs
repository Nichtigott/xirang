//! Field handling for `xirang.apply`: the accepted keys, their type checks, and the overlay.
//! `xirang.apply` 的字段处理：接受的键、类型检查与覆盖。

use serde_json::Value;

use super::Action;
use super::apply_refusals::editable_fields_line;

/// The face fields a request may carry, which is the set `overlay` matches by name.
/// 请求可以携带的注册面字段，也就是 `overlay` 按名字匹配的那一组。
pub(crate) const EDITABLE_FIELDS: &[&str] = &[
    "module",
    "kind",
    "preset",
    "parts",
    "name_zh",
    "name_en",
    "summary_zh",
    "summary_en",
    "exports",
    "stable_name",
    "needs_registry",
    "getting_from_other_registry",
    "registration_rule",
    "admission",
    "handle_traits",
    "handle_contracts",
    "part_traits",
    "part_contracts",
    "requires",
    "provides",
    "runtime_checks",
    "flow",
    "flow_provider",
];

/// The first thing wrong with a request's `fields`, if anything is.
/// 请求的 `fields` 里第一处不对的地方（若有）。
///
/// Both write actions share this so they answer alike: a misspelled key is refused by
/// name rather than dropped, and a key whose value cannot be a field is refused instead of
/// being coerced to the empty string or to `false`. `add` used to take only the keys it
/// knew and silently ignore the rest, so an agent that wrote `knd` believed it had set the
/// kind — the exact direction this check exists to close.
/// 两个写入动作共用它，因此它们答得一样：拼错的键被点名拒绝而不是被丢弃；值不可能成为字段的键
/// 也被拒绝，而不是被强转成空串或 `false`。`add` 过去只取它认识的键、静默忽略其余，于是写下
/// `knd` 的代理会以为自己设了 kind——这道检查正是为了关掉这个方向。
pub(super) fn invalid_field(fields: &Value, action: Action) -> Option<String> {
    let object = fields.as_object()?;
    for (key, value) in object {
        // `fields.parent` is the documented alias of the request's sibling `parent`, and
        // it names a place in the tree rather than a field of the face.
        // `fields.parent` 是与请求平级的 `parent` 的文档化别名，它命名的是树里的位置而不是面的字段。
        if key == "parent" {
            if !value.is_string() {
                return Some(
                    "`parent` must be a string (a logical path or an identity)".to_owned(),
                );
            }
            continue;
        }
        if !EDITABLE_FIELDS.contains(&key.as_str()) {
            // The refusal names the shape as well as the mistake: the round measured five refusals
            // in one family, all of them field spelling, and a refusal that only says "no" costs the
            // next call as well.
            // 拒绝同时说出"形状"和"错在哪"：那轮量到一个族里五次被拒、全是字段拼法，而只会说"不行"的
            // 拒绝还要再花掉下一次调用。
            return Some(format!(
                "`{key}` is not an editable registration-face field\n{}",
                editable_fields_line()
            ));
        }
        // The executor's edit field order does not carry the two contract keys: they
        // are rewritten on `add` and *kept* on `edit`, so accepting them here returned
        // success while the value never reached the file — an agent that asked for a
        // checked contract got an unchecked label instead, silently (audit
        // `LGC-LG-21`). Refusing by name is the loud half of the same choice.
        // 执行器的 edit 字段顺序不携带这两个 contract 键：它们在 `add` 时被写、在 `edit` 时被**保留**，
        // 因此在这里接受它们会返回成功而取值从未抵达文件——一个要求"参与编译检查的契约"的代理拿到的
        // 是未经检查的标签，而且无声（审计 `LGC-LG-21`）。按名拒绝是同一个选择的响亮那一半。
        if matches!(action, Action::Edit | Action::Rename)
            && matches!(key.as_str(), "handle_contracts" | "part_contracts")
        {
            return Some(format!(
                "`{key}` cannot be changed by `{}`: the executor's edit field order does not \
                 carry it, so the write would be dropped. Set it in `add`, or leave it as the \
                 file has it",
                if matches!(action, Action::Rename) {
                    "rename"
                } else {
                    "edit"
                }
            ));
        }
        if key == "needs_registry" {
            if !value.is_boolean() {
                return Some(format!(
                    "`needs_registry` must be true or false\n{}",
                    editable_fields_line()
                ));
            }
        } else if !value.is_string() {
            return Some(format!(
                "`{key}` must be a string (this shape takes strings, not arrays or objects)\n{}",
                editable_fields_line()
            ));
        }
    }
    None
}

/// Create a face in one non-Rust language through the shared authoring service.
/// 经共享创作服务用一种非 Rust 语言创建注册面。
pub(super) fn action_name(action: Action) -> &'static str {
    match action {
        Action::Add => "add",
        Action::Edit => "edit",
        Action::Rename => "rename",
        Action::Delete => "delete",
        Action::Deepen => "deepen",
        Action::Cut => "cut",
        Action::Promote => "promote",
        Action::Move => "move",
    }
}

/// Rewrite the fields a request names, keeping everything it does not.
/// 重写请求点名的字段，其余全部保留。
///
/// The executor's own contract is the opposite — it rebuilds a face from the values
/// it is given, blank included — and that contract is right for an editor showing
/// every field. An agent asking for two changes would have had to restate
/// twenty-three fields and would silently blank any it forgot, so this path reads
/// the face back through `authored_face`, overlays the request, and hands the
/// complete set to the executor. The executor still decides everything that
/// matters; only the *input* is completed for the caller.
/// 执行器自己的契约相反——它用拿到的取值整体重建一个面，包括空值——而那对展示所有字段的编辑器是
/// 正确的。一个只要求两处改动的代理否则必须复述二十三个字段，且会静默抹掉它忘掉的任何一个，因此
/// 这条路径经 `authored_face` 读回该面、覆盖请求、再把完整的一组交给执行器。仍然由执行器决定所有
/// 要紧的事；只是**输入**替调用方补全了。
/// Overlay a request's fields onto a face read back from the tree.
/// 把请求的字段覆盖到从树上读回的那个面上。
///
/// Every key is matched by name: a misspelled field is refused instead of being
/// dropped, which is the direction that keeps an agent from believing it changed
/// something it did not.
/// 每个键按名字匹配：拼错的字段被拒绝而不是被丢弃——正是这个方向让代理不会以为自己改了什么而
/// 其实没改。
pub(super) fn overlay(
    authored: &mut crate::run_method::AuthoredFace,
    fields: &Value,
    action: Action,
) -> Result<(), String> {
    // The same predicate `add` uses, so the two actions cannot drift apart on which key
    // they accept or on how a value's type is answered.
    // 与 `add` 用同一个判定，因此两个动作在"接受哪个键"和"值的类型怎么答"上不会漂移。
    if let Some(problem) = invalid_field(fields, action) {
        return Err(problem);
    }
    let Some(object) = fields.as_object() else {
        return Ok(());
    };
    for (key, value) in object {
        let text = || {
            value.as_str().map(str::to_owned).ok_or_else(|| {
                format!(
                    "`{key}` must be a string (this shape takes strings, not arrays or objects)\n{}",
                    editable_fields_line()
                )
            })
        };
        match key.as_str() {
            "module" => authored.module = text()?,
            "kind" => authored.kind = text()?,
            "preset" => authored.preset = text()?,
            "parts" => authored.parts = text()?,
            "name_zh" => authored.name_zh = text()?,
            "name_en" => authored.name_en = text()?,
            "summary_zh" => authored.summary_zh = text()?,
            "summary_en" => authored.summary_en = text()?,
            "exports" => authored.exports = text()?,
            "stable_name" => authored.stable_name = text()?,
            "needs_registry" => {
                authored.needs_registry = value.as_bool().ok_or_else(|| {
                    format!(
                        "`needs_registry` must be true or false\n{}",
                        editable_fields_line()
                    )
                })?;
            }
            "getting_from_other_registry" => authored.getting_from_other_registry = text()?,
            "registration_rule" => authored.registration_rule = text()?,
            "admission" => authored.admission = text()?,
            "handle_traits" => authored.handle_traits = text()?,
            "handle_contracts" => authored.handle_contracts = text()?,
            "part_traits" => authored.part_traits = text()?,
            "part_contracts" => authored.part_contracts = text()?,
            "requires" => authored.requires = text()?,
            "provides" => authored.provides = text()?,
            "runtime_checks" => authored.runtime_checks = text()?,
            "flow" => authored.flow = text()?,
            "flow_provider" => authored.flow_provider = text()?,
            other => {
                return Err(format!(
                    "`{other}` is not an editable registration-face field"
                ));
            }
        }
    }
    Ok(())
}

/// The one string field of a request, or the empty string.
/// 请求里的一个字符串字段；没有时为空串。
pub(super) fn text<'a>(fields: &'a Value, key: &str) -> &'a str {
    fields.get(key).and_then(Value::as_str).unwrap_or("")
}
