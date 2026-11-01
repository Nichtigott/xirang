//! `source_path_from_file`: a rule for naming a file, never a list of names.
//! `source_path_from_file`：给文件命名的是一条规则，绝不是一份名单。

use super::{after_last_src, rule_syntax_for_source, source_path_from_file};
use std::path::{Path, PathBuf};

/// A file under this package's `src/` is named below it.
/// 位于本包 `src/` 下的文件按它之下的路径命名。
#[test]
fn a_file_under_the_source_root_keeps_its_path_below_it() {
    let path = super::super::context::identity_base()
        .expect("the fixture package resolves a layout")
        .join("control/object/button/button.rs");
    assert_eq!(
        source_path_from_file(&path),
        "control/object/button/button.rs"
    );
}

/// A file outside it is folded by the `src/` rule, not cut at a known directory
/// name: the *last* `src/` component is the boundary, so a host whose project
/// path happens to contain this repository's fixture names — or any other name —
/// no longer loses its prefix to a guess.
/// 它之外的文件按 `src/` 规则折叠，而不是在某个已知目录名处切开：边界是**最后一个** `src/`
/// 段，因此项目路径恰好含有本仓夹具名（或任何别的名字）的宿主不会再被猜掉前缀。
///
/// Red before the fix: this path returned `control/myrepo/src/foo/foo.rs` (measured
/// by restoring the old implementation in a copy), because the name list contained
/// `control` and the walk cut at the first component that matched a fixture name.
/// 修前为红：这条路径会返回 `control/myrepo/src/foo/foo.rs`（在副本里还原旧实现实测），因为名单里
/// 有 `control`，遍历在第一个命中夹具名的路径分量处切断了。
#[test]
fn a_file_outside_the_source_root_is_folded_at_the_last_src() {
    assert_eq!(
        source_path_from_file(Path::new("/data/control/myrepo/src/foo/foo.rs")),
        "foo/foo.rs"
    );
    // The old guess, spelled out: this is what the fixture-name list produced.
    assert_ne!(
        source_path_from_file(Path::new("/data/control/myrepo/src/foo/foo.rs")),
        "control/myrepo/src/foo/foo.rs"
    );
    // A sibling of a migrated module is not a migrated module: the fold is a
    // whole `src/` segment rule, and the name after it is kept verbatim.
    assert_eq!(
        source_path_from_file(Path::new("/anywhere/src/control_extra/extra.rs")),
        "control_extra/extra.rs"
    );
}

/// With no `src/` component there is nothing to fold, so the path stands as
/// written rather than being truncated at whatever directory is in a list.
/// 没有 `src/` 段时没有可折叠的东西，因此路径按原样保留，而不是在名单里的某个目录处被截断。
#[test]
fn a_path_without_src_stands_as_written() {
    // A relative path with no `src/` in it is exactly itself.
    assert_eq!(
        source_path_from_file(Path::new("widget/other.rs")),
        "widget/other.rs"
    );
    // An absolute path with no `src/` is not cut at a directory that happens to
    // be named `control` — that truncation is what the name list did.
    let absolute = Path::new("/data/control/myrepo/other.rs");
    let named = source_path_from_file(absolute);
    assert_ne!(named, "control/myrepo/other.rs");
    assert!(
        named.ends_with("data/control/myrepo/other.rs"),
        "the path is kept whole: {named}"
    );
    assert_eq!(after_last_src(Path::new("/etc/hosts")), None);
    // A trailing `src` names no file below it: there is nothing to return.
    assert_eq!(after_last_src(Path::new("/w/src")), None);
    assert_eq!(
        after_last_src(Path::new("/w/src/a.rs")),
        Some("a.rs".to_owned())
    );
}

/// The rule files this repository ships, read through the production entry.
/// 本仓库出厂的规则文件，经生产入口读取。
///
/// The strict reader must not refuse what the build already consumes: every shipped
/// `registry_rule.rs` still reads, and the bytes agree with the tolerant published entry
/// (which is no longer on this path). The count is asserted so an empty walk cannot pass.
/// 严格读取器不得拒绝构建已在消费的东西：每份出厂的 `registry_rule.rs` 仍能读出，且字节与宽容的
/// 已发布入口一致（该入口已不在本路径上）。断言数量，使空遍历不能通过。
#[test]
fn every_shipped_rule_source_still_reads_through_the_production_entry() {
    let workspace = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("the workspace root")
        .to_path_buf();
    let mut files = Vec::new();
    visit_rule_files(&workspace, &mut files);
    files.sort();
    assert!(!files.is_empty(), "the walk found no shipped rule source");
    for path in files {
        let face = path
            .parent()
            .and_then(|directory| directory.parent())
            .map(|directory| directory.join("probe_face.rs"))
            .expect("a face path");
        let text = std::fs::read_to_string(&path).expect("a shipped rule source");
        let read = rule_syntax_for_source(&face)
            .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
        assert_eq!(
            read,
            xirang_kernel::authoring::parse::rule_syntax_from_text(&text),
            "{}: the production entry and the shipped bytes disagree",
            path.display()
        );
    }
}

/// A malformed rule source is refused with its context, not silently degraded.
/// 畸形的规则源会被拒绝并带上上下文，而不是静默降级。
///
/// Red before the fix, measured on this very entry: the commented pseudo-clause read as
/// `Ok("exports:wrong")` — the real `control.render` requirement dropped, a clause nobody
/// declared adopted — and a list named by a constant read as `Ok("ANY")`, i.e. "no
/// structural requirement". Both are `Err` now, and the message names the face file, the
/// rule file, and what the reader could not read (audit `KRN-K-10`).
/// 修前为红，就在这个入口上实测：注释里的伪子句读成 `Ok("exports:wrong")`——真正的
/// `control.render` 要求被丢掉、没人声明的子句被采纳；而由常量指名列表的规则读成 `Ok("ANY")`，
/// 即"没有结构要求"。现在两者都是 `Err`，消息点名注册面文件、规则文件与读取器读不出的东西
/// （审计 `KRN-K-10`）。
#[test]
fn a_malformed_rule_source_is_refused_with_its_context() {
    let root = unique_root("refused");
    let face_dir = root.join("control");
    std::fs::create_dir_all(face_dir.join("registry_rule")).expect("fixture dir");
    let face = face_dir.join("control.rs");
    let rule = face_dir.join("registry_rule/registry_rule.rs");
    // A clause inside a comment is not part of the rule, so this source *reads* — the
    // production path is supposed to keep working here, and the trap it closes is that the
    // commented clause no longer becomes the rule. The tolerant published entry, for
    // contrast, still reads `exports:wrong` (which is exactly why this path left it).
    // 注释里的子句不属于规则，因此这份源**读得出来**——生产路径在这里本应继续工作，它关掉的陷阱是
    // "注释里的子句不再成为规则"。作为对照，宽容的已发布入口读到的仍是 `exports:wrong`（这正是本路径
    // 弃用它的原因）。
    let commented = "use crate::RegistrationRule;\npub const REGISTRATION_RULE: RegistrationRule = RegistrationRule::new()\n    // .require_exports(&[\"wrong\"])\n    .require_exports(&[\"control.render\"]);\n";
    std::fs::write(&rule, commented).expect("fixture rule file");
    assert_eq!(
        rule_syntax_for_source(&face).as_deref(),
        Ok("exports:control.render"),
        "the const's own initializer decides the rule"
    );
    assert_eq!(
        xirang_kernel::authoring::parse::rule_syntax_from_text(commented),
        "exports:wrong",
        "the tolerant entry still reads the commented clause, which is why this path left it"
    );

    // A list named by a constant cannot be read at all: the production path refuses it with
    // its context instead of silently returning `ANY` ("no structural requirement").
    // 由常量指名的列表根本读不出来：生产路径带上下文拒绝它，而不是静默返回 `ANY`（"没有结构要求"）。
    let through_a_constant = "use crate::RegistrationRule;\nconst EXPORTS: [&str; 1] = [\"control.render\"];\npub const REGISTRATION_RULE: RegistrationRule = RegistrationRule::new()\n    .require_exports(&EXPORTS);\n";
    std::fs::write(&rule, through_a_constant).expect("fixture rule file");
    let error =
        rule_syntax_for_source(&face).expect_err("a rule the strict reader cannot read is refused");
    assert!(
        error.contains("control.rs") && error.contains("registry_rule.rs"),
        "the message names the face and the rule file: {error}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// A face with no rule file still requires nothing.
/// 没有规则文件的注册面仍然什么都不要求。
#[test]
fn a_missing_rule_file_is_still_no_requirement() {
    let root = unique_root("absent");
    let face_dir = root.join("control");
    std::fs::create_dir_all(&face_dir).expect("fixture dir");
    let face = face_dir.join("control.rs");
    assert_eq!(rule_syntax_for_source(&face).as_deref(), Ok("ANY"));
    let _ = std::fs::remove_dir_all(&root);
}

/// A directory no other test shares.
/// 一个没有别的测试共用的目录。
fn unique_root(label: &str) -> PathBuf {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "xirang-t72-{label}-{}-{sequence}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    root
}

fn visit_rule_files(directory: &std::path::Path, files: &mut Vec<std::path::PathBuf>) {
    let Ok(entries) = std::fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            let name = path
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_default();
            if name == "target" || name == ".git" || name == ".dsh-meow" {
                continue;
            }
            visit_rule_files(&path, files);
        } else if path
            .file_name()
            .is_some_and(|name| name == "registry_rule.rs")
        {
            files.push(path);
        }
    }
}

/// The location the field names is the location the production reader reads.
/// 字段名的位置就是生产读取器去读的位置。
///
/// The half the kernel cannot check: the field's value is derived in `core` while the reader
/// that follows it lives here. With the file absent the field still names the canonical
/// location and the reader answers `Ok("ANY")` (absence is not a malformed rule); put the rule
/// exactly where the field says and the reader reads it. Audit `t75` §6.1.
/// 内核查不了的那一半：字段取值在 `core` 派生，而跟着它去读的读取器在这里。文件缺席时字段仍名出
/// 那个规范位置、读取器也仍答 `Ok("ANY")`（缺席不是畸形规则）；把规则放到字段说的那个位置，读取器
/// 就读得到它。审计 `t75` §6.1。
#[test]
fn the_rule_path_field_names_the_location_the_production_reader_reads() {
    let root = unique_root("rule-location");
    let face_dir = root.join("src/control");
    std::fs::create_dir_all(&face_dir).expect("fixture dir");
    let face = face_dir.join("control.rs");

    let mut values = std::collections::BTreeMap::new();
    for (key, value) in [
        ("source", "control/control.rs"),
        ("kind", "ControlRegistry"),
        ("namespace", "probe"),
        ("needs_registry", "false"),
        ("registration_rule", "ANY"),
        ("admission", "ANY"),
    ] {
        values.insert(key.to_owned(), value.to_owned());
    }
    values.insert(
        "parent_node".to_owned(),
        xirang_kernel::identity::NodeId::from_namespaced_path("probe", "root.rs", "root")
            .to_string(),
    );
    let snapshot = xirang_kernel::authoring::snapshot::snapshot_from_values(&values, "probe")
        .expect("a readable face");
    assert_eq!(
        snapshot.registry_rule_path, "src/control/registry_rule/registry_rule.rs",
        "the canonical location beside the face, derived before any file is consulted"
    );

    // Nothing is on disk yet: the field named the location, and the reader reports "no
    // requirement" instead of failing.
    // 磁盘上还什么都没有：字段已经名出位置，而读取器报"没有要求"而不是失败。
    assert_eq!(rule_syntax_for_source(&face).as_deref(), Ok("ANY"));

    let rule = face_dir.join("registry_rule/registry_rule.rs");
    std::fs::create_dir_all(rule.parent().expect("the rule directory")).expect("rule dir");
    std::fs::write(
        &rule,
        "use crate::RegistrationRule;\npub const REGISTRATION_RULE: RegistrationRule = RegistrationRule::new()\n    .require_exports(&[\"control.render\"]);\n",
    )
    .expect("fixture rule file");
    assert_eq!(
        rule_syntax_for_source(&face).as_deref(),
        Ok("exports:control.render"),
        "the reader looks for exactly the location the field named"
    );
    let _ = std::fs::remove_dir_all(&root);
}
