//! Validation of test-selection requests before any process or filesystem side effect.
//! 在进程或文件系统副作用之前校验测试选择请求。

use serde_json::Value;

/// Reject unsupported selectors, wrong types and conflicting execution scopes.
/// 拒绝不支持的选择器、错误类型及冲突的执行范围。
/// The one action the `files` branch has, checked where every other request key is checked.
/// `files` 分支唯一的那一个动作，与其它请求键在同一处校验。
///
/// It has to be an action and not a flag: a plan phase that could be asked for with a stray truthy
/// value would be reachable by typos, and a typo that silently runs Cargo is the thing this refuses.
/// 它必须是一个动作而不是一个开关：能用某个随手写的真值请求到的计划阶段会被拼写错误触发，而"静默跑起 Cargo
/// 的拼写错误"正是这里要拒绝的东西。
pub(super) fn validate_action(arguments: &Value) -> Result<(), String> {
    let Some(action) = arguments.get("action") else {
        return Ok(());
    };
    let action = action
        .as_str()
        .ok_or("check action must be the string `plan`")?;
    if action == "plan" {
        return Ok(());
    }
    Err(format!(
        "check action is `plan` or absent, not `{action}`; `status`/`resume` are addressed by `plan_id`"
    ))
}

pub(super) fn validate(arguments: &Value) -> Result<(), String> {
    // Refuse misspelled selectors before running Cargo: silently dropping `features`
    // can report a green default run while the requested feature still fails.
    // 运行 Cargo 前拒绝拼错的选择参数：静默丢弃 `features` 会把默认面的绿误当作目标特性的绿。
    if !arguments.is_null() && !arguments.is_object() {
        return Err("check arguments must be an object".to_owned());
    }
    if let Some(fields) = arguments.as_object() {
        for key in fields.keys() {
            if ![
                "files",
                "cases",
                "face",
                "timeout_ms",
                "target_dir",
                "census",
                "verbose",
                "root",
                "freshness",
                "workspace",
                "package",
                "test",
                "filter",
                "no_default_features",
                "mode",
                "profile",
                "target",
                "all_targets",
                "deny_warnings",
                "plan_id",
                "reuse",
                "action",
            ]
            .contains(&key.as_str())
            {
                return Err(format!(
                    "unsupported check parameter `{key}`; use `face` for Cargo features (e.g. \"dev-supervisor,studio\"); no tests were run"
                ));
            }
        }
    }
    for key in [
        "face",
        "target_dir",
        "root",
        "package",
        "test",
        "filter",
        "mode",
        "target",
    ] {
        if arguments
            .get(key)
            .is_some_and(|value| value.as_str().is_none_or(|text| text.trim().is_empty()))
        {
            return Err(format!(
                "check `{key}` must be a non-empty string; no tests were run"
            ));
        }
    }
    for key in [
        "census",
        "verbose",
        "workspace",
        "no_default_features",
        "all_targets",
        "deny_warnings",
        "reuse",
    ] {
        if arguments.get(key).is_some_and(|value| !value.is_boolean()) {
            return Err(format!(
                "check `{key}` must be a boolean; no tests were run"
            ));
        }
    }
    if arguments
        .get("timeout_ms")
        .is_some_and(|value| value.as_u64().is_none())
    {
        return Err(
            "check `timeout_ms` must be a non-negative integer; no tests were run".to_owned(),
        );
    }
    if arguments.get("workspace").and_then(Value::as_bool) == Some(true)
        && arguments.get("package").is_some()
    {
        return Err("choose workspace or package, not both; no tests were run".into());
    }
    for key in ["package", "test", "filter", "mode", "target"] {
        if arguments
            .get(key)
            .and_then(Value::as_str)
            .is_some_and(|text| text.starts_with('-') || text.chars().any(char::is_control))
        {
            return Err(format!("invalid {key} selector; no tests were run"));
        }
    }
    if arguments.get("cases").is_some() && arguments.get("files").is_none() {
        return Err("cases requires files (a repair verification plan)".into());
    }
    let mode = arguments
        .get("mode")
        .and_then(Value::as_str)
        .unwrap_or("test");
    if !matches!(mode, "test" | "check" | "build" | "clippy") {
        return Err("mode must be test, check, build, or clippy".into());
    }
    if mode != "test" && arguments.get("filter").is_some() {
        return Err("filter applies only to test mode".into());
    }
    if mode != "clippy" && arguments.get("deny_warnings").is_some() {
        return Err("deny_warnings applies only to clippy mode".into());
    }
    // The face is validated here rather than at the command line, because "rejected before any
    // process or filesystem side effect" is this module's whole contract: a face that reaches
    // `cargo` is a face Cargo decides about, and a selector with a space in it would be two argv
    // entries rather than a refusal.
    // 特性面在这里校验而不是在命令行处，因为"在任何进程或文件系统副作用之前被拒绝"正是本模块的全部契约：
    // 到达 `cargo` 的特性面就是由 Cargo 裁定的特性面，而带空格的选择器会变成两个 argv 项，而不是一次拒绝。
    if let Some(face) = arguments.get("face").and_then(Value::as_str)
        && face.split(',').any(|feature| {
            feature.is_empty()
                || feature.starts_with('-')
                || feature
                    .contains(|character: char| character.is_whitespace() || character.is_control())
        })
    {
        return Err(format!(
            "`face` is `default`, `all`, or comma-separated Cargo feature names, not `{face}`"
        ));
    }
    // A profile is a build identity, not a hint: `debug` and `release` compile different code
    // (`debug_assert!`, overflow checks, `cfg(debug_assertions)`) and produce different test
    // binaries, so a verdict from one can never be read as a verdict about the other. The value is
    // validated here, carried on the command line, and bound into the execution identity — an
    // unvalidated spelling would be the "documented but ineffective call" the specification forbids.
    // profile 是构建身份而不是提示：`debug` 与 `release` 编译出不同的代码（`debug_assert!`、溢出检查、
    // `cfg(debug_assertions)`）与不同的测试二进制，因此一个的判定永远不能读成关于另一个的判定。取值在这里
    // 校验、在命令行上携带、并绑进执行身份——未校验的拼法正是规格禁止的"写进文档却无效的调用"。
    if let Some(profile) = arguments.get("profile") {
        match profile.as_str() {
            Some("debug" | "release") => {}
            _ => return Err("profile must be debug or release; no tests were run".into()),
        }
    }
    Ok(())
}
