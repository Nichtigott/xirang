//! covers `app/mutations.rs`, `app/project_context.rs`.
//! 覆盖 `app/mutations.rs`、`app/project_context.rs`。
//!
//! Regression tests for the plugin-lock write path (audit `LGC-LG-03`).
//! 插件锁写入路径的回归测试（审计 `LGC-LG-03`）。

use super::super::project_context::select_project;
use super::*;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

/// A throwaway project whose `.xirang/plugins/` is the write target.
/// 一个一次性工程，其 `.xirang/plugins/` 就是写入目标。
fn temp_plugins(label: &str) -> (std::path::PathBuf, std::path::PathBuf) {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let root = std::env::temp_dir().join(format!("xirang-toolchain-lock-{label}-{suffix}"));
    let plugins = root.join(".xirang/plugins");
    std::fs::create_dir_all(&plugins).expect("plugin directory");
    select_project(root.clone(), root.join("Cargo.toml"), "plugin-app");
    (root, plugins)
}

/// Submit one form and report what the status line said.
/// 提交一份表单，并交回状态行说了什么。
fn submit(source: &str, fields: [&str; 5], mode: &str) -> String {
    let mut plugin = PluginState::new();
    plugin.values[plugin_field::SOURCE] = source.to_owned();
    plugin.values[plugin_field::FRAMEWORK] = fields[0].to_owned();
    plugin.values[plugin_field::PACKAGE] = fields[1].to_owned();
    plugin.values[plugin_field::VERSION] = fields[2].to_owned();
    plugin.values[plugin_field::CRATE] = fields[3].to_owned();
    plugin.values[plugin_field::CHECKSUM] = fields[4].to_owned();
    plugin.values[plugin_field::MODE] = mode.to_owned();
    let mut app = App::load_app();
    app.submit_plugin(&plugin);
    app.event
}

/// Fail on the one forbidden outcome: success reported over a lock the kernel
/// refuses to parse. A refusal is the acceptable other end, if it is visible.
/// 只禁止一个结局：在内核拒绝解析的锁上报成功。另一端只要可见就可接受。
fn assert_never_succeeds_over_an_unparseable_lock(lock: &Path, event: &str) {
    let text = std::fs::read_to_string(lock).unwrap_or_default();
    match (
        event.contains("Plugin selected"),
        PluginCatalog::parse_plugin_catalog(&text),
    ) {
        (true, Err(error)) => {
            panic!("success over a lock the host refuses: {error}\nevent = {event}\n{text:?}")
        }
        (false, _) => assert!(
            event.contains("Plugin failed"),
            "a submission that wrote nothing must say why: {event}"
        ),
        (true, Ok(_)) => {}
    }
}

/// The old append wrote a second record with the same identity five-tuple,
/// making the lock unreadable while the status line said "Plugin selected".
/// 旧追加会写下第二条同身份五元组记录，使锁不可读，而状态行说 "Plugin selected"。
#[test]
fn a_duplicate_user_identity_is_never_written_unreadably() {
    let (root, plugins) = temp_plugins("duplicate-checksum");
    let lock = plugins.join("user.lock");
    std::fs::write(
        &lock,
        "user|xirang.test|demo-plugin|0.1.0|demo_plugin|sha256:00|extension\n",
    )
    .expect("seed lock");

    let event = submit(
        "user",
        [
            "xirang.test",
            "demo-plugin",
            "0.1.0",
            "demo_plugin",
            "sha256:01",
        ],
        "extension",
    );

    assert_never_succeeds_over_an_unparseable_lock(&lock, &event);
    // A refusal names the file and the parser's reason; a bare "Plugin failed"
    // is a message no reader can act on.
    // 拒绝要点名文件与解析器的理由；光说 "Plugin failed" 是读者无法据以行动的消息。
    if !event.contains("Plugin selected") {
        assert!(
            event.contains("user.lock") && event.contains("duplicates package identity"),
            "{event}"
        );
    }
    let _ = std::fs::remove_dir_all(&root);
}

/// Switching the mode rewrites one identity field, so it collides the same way.
/// 切换 mode 改写的是同一个身份字段，因此以同样方式碰撞。
#[test]
fn toggling_the_mode_is_never_written_as_a_duplicate_identity() {
    let (root, plugins) = temp_plugins("duplicate-mode");
    let lock = plugins.join("user.lock");
    std::fs::write(
        &lock,
        "user|xirang.test|demo-plugin|0.1.0|demo_plugin|sha256:00|extension\n",
    )
    .expect("seed lock");

    let event = submit(
        "user",
        [
            "xirang.test",
            "demo-plugin",
            "0.1.0",
            "demo_plugin",
            "sha256:00",
        ],
        "replacement",
    );

    assert_never_succeeds_over_an_unparseable_lock(&lock, &event);
    let _ = std::fs::remove_dir_all(&root);
}

/// A lock whose last line has no terminator must gain one, not swallow the next
/// record: `a` + `b` written as `ab` is neither record.
/// 末行没有终止符的锁要补一个，而不是吞下一条记录：`a` 接 `b` 写成 `ab` 后两者都不是。
#[test]
fn a_lock_without_a_trailing_newline_is_never_glued_to_the_next_record() {
    let (root, plugins) = temp_plugins("no-terminator");
    let lock = plugins.join("user.lock");
    std::fs::write(
        &lock,
        "user|xirang.test|first-plugin|0.1.0|first_plugin|sha256:00|extension",
    )
    .expect("seed lock");

    let event = submit(
        "user",
        [
            "xirang.test",
            "second-plugin",
            "0.2.0",
            "second_plugin",
            "sha256:01",
        ],
        "extension",
    );

    assert_never_succeeds_over_an_unparseable_lock(&lock, &event);
    let text = std::fs::read_to_string(&lock).expect("lock readable");
    assert_eq!(
        PluginCatalog::parse_plugin_catalog(&text)
            .expect("the pair of records parses")
            .records()
            .len(),
        2,
        "{text:?}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// A ten-field official record whose provenance columns are empty parses with
/// `Some("")` in each of them — the ten-field spelling's "declared: no value", which
/// is deliberately not the seven-field form's `None` ("never mentioned") — so the
/// record still accounts for a candidate that names no value either, the trust rule
/// admits the duplicate identity, and the append runs. That append is the one
/// official outcome this gate changes, and it becomes a visible refusal; the two
/// shapes are pinned by `core/tests/plugin_lock_provenance.rs`'s
/// `an_explicitly_empty_provenance_column_pins_absence`.
/// 来源字段为空的十字段官方记录，三个空列各解析成 `Some("")`——十字段拼法的"声明此处没有值"，
/// 刻意不同于七字段形式的 `None`（"没提到"）——因此它仍然覆盖同样没点值的候选，信任规则放行
/// 同身份第二条，追加被执行。这次追加是本闸门唯一改变的官方结局，它变成一次可见的拒绝；两种形态
/// 的区别由 `core/tests/plugin_lock_provenance.rs` 的
/// `an_explicitly_empty_provenance_column_pins_absence` 钉住。
#[test]
fn an_official_append_that_would_duplicate_an_identity_is_refused_not_written() {
    let (root, plugins) = temp_plugins("official-duplicate");
    let lock = plugins.join("official.lock");
    std::fs::write(
        &lock,
        "official|xirang.test|official-plugin|1.0.0|official_plugin|sha256:00|extension|||\n",
    )
    .expect("seed lock");

    let event = submit(
        "official",
        [
            "xirang.test",
            "official-plugin",
            "1.0.0",
            "official_plugin",
            "sha256:00",
        ],
        "extension",
    );

    assert_never_succeeds_over_an_unparseable_lock(&lock, &event);
    let text = std::fs::read_to_string(&lock).expect("lock readable");
    assert_eq!(
        PluginCatalog::parse_plugin_catalog(&text)
            .expect("the seeded lock still parses")
            .records()
            .len(),
        1,
        "{text:?}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// The three provenance rows write the ten-column form, and leaving them empty keeps the
/// seven-column one. Both come from the kernel's single renderer, so this form cannot spell a
/// column layout of its own — and the upgrade replaces the bare record in place rather than
/// leaving two lines with one identity.
/// 三个来源行写出十列形式，留空则保持七列。两者都出自内核唯一的渲染器，因此这份表单拼不出自己的一套
/// 列布局——而这次升级就地把裸记录换掉，而不是留下两行同身份的记录。
#[test]
fn the_provenance_rows_write_the_ten_column_form() {
    let (_root, plugins) = temp_plugins("provenance");
    let lock = plugins.join("official.lock");
    std::fs::write(
        &lock,
        "# source|framework|package|version|crate|checksum|mode\n\
         official|xirang.default|plugin-app|1.0.0|plugin_app|sha256:00|extension\n",
    )
    .expect("seed lock");
    let mut plugin = PluginState::new();
    plugin.values[plugin_field::PACKAGE] = "plugin-app".to_owned();
    plugin.values[plugin_field::VERSION] = "1.0.0".to_owned();
    plugin.values[plugin_field::CRATE] = "plugin_app".to_owned();
    plugin.values[plugin_field::CHECKSUM] = "sha256:00".to_owned();
    plugin.values[plugin_field::SIGNATURE] = "sig-v1".to_owned();
    plugin.values[plugin_field::FINGERPRINT] = "key-v1".to_owned();
    let mut app = App::load_app();
    app.submit_plugin(&plugin);
    let text = std::fs::read_to_string(&lock).expect("lock");
    let records = text
        .lines()
        .filter(|line| !line.starts_with('#') && !line.trim().is_empty())
        .collect::<Vec<_>>();
    assert_eq!(records.len(), 1, "one identity, one record: {text}");
    assert_eq!(records[0].split('|').count(), 10, "{text}");
    assert!(records[0].ends_with("|sig-v1|key-v1|"), "{text}");
    assert!(
        PluginCatalog::parse_plugin_catalog(&text).is_ok(),
        "the lock the form wrote must parse: {text}"
    );
}
