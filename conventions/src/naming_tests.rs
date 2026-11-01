//! Tests for the crate, directory, and library naming gate.
//! crate、目录与 library 命名门禁的测试。

use super::*;

/// A throwaway checkout with the given members.
/// 一个只含给定成员的一次性检出。
fn synthetic(members: &[(&str, &str)]) -> std::path::PathBuf {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let root =
        std::env::temp_dir().join(format!("xirang-naming-{}-{sequence}", std::process::id()));
    for (directory, manifest) in members {
        let base = root.join(directory);
        fs::create_dir_all(base.join("src")).expect("fixture src");
        fs::write(base.join("Cargo.toml"), manifest).expect("fixture manifest");
        fs::write(base.join("src/lib.rs"), "\n").expect("fixture lib");
    }
    crate::fixture_manifest(&root);
    root
}

/// A member whose package name does not name its directory is reported.
/// 包名不指涉其目录的成员会被报出。
#[test]
fn a_package_name_that_disagrees_with_the_directory_is_reported() {
    let root = synthetic(&[(
        "thing",
        "[package]\nname = \"xirang-other\"\n\n[lib]\nname = \"xirang_thing\"\n",
    )]);
    let found = findings(&root);
    assert_eq!(found.len(), 1, "{found:#?}");
    assert!(found[0].reason.contains("xirang-other"), "{found:#?}");
    let _ = fs::remove_dir_all(&root);
}

/// A lib name that is neither the rule nor the documented exception is
/// reported, because it is the name hosts write.
/// 既不符合规则、也不是文档化例外的 lib 名会被报出，因为它正是宿主书写的名字。
#[test]
fn a_lib_name_that_disagrees_is_reported() {
    let root = synthetic(&[(
        "thing",
        "[package]\nname = \"xirang-thing\"\n\n[lib]\nname = \"thing\"\n",
    )]);
    let found = findings(&root);
    assert_eq!(found.len(), 1, "{found:#?}");
    assert!(found[0].reason.contains("lib name"), "{found:#?}");
    let _ = fs::remove_dir_all(&root);
}

/// A hyphenated directory maps to an underscored lib name, because a Rust
/// library name cannot carry a hyphen. This gate reported the workspace's own
/// `plugin-host` before the fold existed.
/// 带连字符的目录映射为下划线的 library 名，因为 Rust 的 library 名不能带连字符。在这次折叠
/// 出现之前，本门禁曾把工作区自己的 `plugin-host` 报出来。
#[test]
fn a_hyphenated_directory_maps_to_an_underscored_lib_name() {
    let root = synthetic(&[(
        "plugin-host",
        "[package]\nname = \"xirang-plugin-host\"\n\n[lib]\nname = \"xirang_plugin_host\"\n",
    )]);
    assert_eq!(findings(&root), Vec::new());
    let _ = fs::remove_dir_all(&root);
}

/// The kernel needs no lib-name carve-out: `kernel/` + `xirang-kernel` +
/// `xirang_kernel` is the rule itself, so the exception table is empty and the
/// kernel is checked like every other crate (batch 1 of the publish-surface merge).
/// 内核不需要 lib 名特例：`kernel/` + `xirang-kernel` + `xirang_kernel` 就是规则本身，
/// 因此例外表为空，内核与其它 crate 一样受检（发布面合并的批次 1）。
#[test]
fn the_kernel_needs_no_lib_name_carve_out() {
    let root = synthetic(&[(
        "kernel",
        "[package]\nname = \"xirang-kernel\"\n\n[lib]\nname = \"xirang_kernel\"\n",
    )]);
    assert_eq!(findings(&root), Vec::new());
    let _ = fs::remove_dir_all(&root);
}

/// A member with no `[lib] name` is fine: cargo derives it from the package
/// name, which is already tied to the directory.
/// 没有 `[lib] name` 的成员没问题：cargo 从包名推导它，而包名已经被绑到目录名。
#[test]
fn a_derived_lib_name_is_accepted() {
    let root = synthetic(&[("thing", "[package]\nname = \"xirang-thing\"\n")]);
    assert_eq!(findings(&root), Vec::new());
    let _ = fs::remove_dir_all(&root);
}

/// The example hosts are a naming family of their own and are left alone.
/// 示例宿主自成一个命名家族，不被过问。
#[test]
fn an_example_host_is_outside_the_rule() {
    let root = synthetic(&[(
        "examples/control-button",
        "[package]\nname = \"xirang-example-control-button\"\n\n[lib]\nname = \"control_button\"\n",
    )]);
    assert_eq!(findings(&root), Vec::new());
    let _ = fs::remove_dir_all(&root);
}

/// A scaffold template or a CI step that names a package the manifests do not
/// define is reported: a rename that misses one of them fails somewhere else.
/// 脚手架模板或 CI 步骤指名清单里不存在的包会被报出：漏改其中一处的重命名会在别处失败。
#[test]
fn a_reference_to_a_package_that_does_not_exist_is_reported() {
    let root = synthetic(&[(
        "thing",
        "[package]\nname = \"xirang-thing\"\n\n[lib]\nname = \"xirang_thing\"\n",
    )]);
    fs::create_dir_all(root.join("toolchain/src/build_method/scaffold")).expect("scaffold dir");
    fs::write(
            root.join("toolchain/src/build_method/scaffold/probe.rs"),
            "let requirement = \"xirang-missing = { path = \\\"../missing\\\", version = \\\"0.1.0\\\" }\";\n",
        )
        .expect("template file");
    fs::create_dir_all(root.join(".github/workflows")).expect("workflow dir");
    fs::write(
        root.join(".github/workflows/ci.yml"),
        "run: cargo test -p xirang-gone --offline\n# prose about `-p xirang-thing`\n",
    )
    .expect("workflow file");

    let found = findings(&root);
    assert_eq!(found.len(), 2, "{found:#?}");
    assert!(
        found
            .iter()
            .any(|finding| finding.reason.contains("xirang-missing")),
        "{found:#?}"
    );
    assert!(
        found
            .iter()
            .any(|finding| finding.reason.contains("xirang-gone")),
        "{found:#?}"
    );
    let _ = fs::remove_dir_all(&root);
}

/// Two more spellings of the same argument, and one more workflow extension.
/// `--package=xirang-typo` and `-p a,xirang-typo` name packages exactly as
/// `-p xirang-typo` does, and a `.yaml` workflow is a workflow to GitHub.
/// 同一个实参的另外两种拼写，外加一个工作流扩展名。`--package=xirang-typo` 与
/// `-p a,xirang-typo` 命名包的方式与 `-p xirang-typo` 完全相同，而 `.yaml` 工作流对 GitHub
/// 也是工作流。
#[test]
fn the_other_package_spellings_and_yaml_workflows_are_read() {
    let root = synthetic(&[(
        "thing",
        "[package]\nname = \"xirang-thing\"\n\n[lib]\nname = \"xirang_thing\"\n",
    )]);
    fs::create_dir_all(root.join(".github/workflows")).expect("workflow dir");
    fs::write(
        root.join(".github/workflows/release.yaml"),
        "run: cargo publish --package=xirang-typo\n",
    )
    .expect("yaml workflow");
    fs::write(
        root.join(".github/workflows/ci.yml"),
        "run: cargo test -p xirang-thing,xirang-second\n",
    )
    .expect("yml workflow");

    let found = findings(&root);
    assert!(
        found
            .iter()
            .any(|finding| finding.reason.contains("xirang-typo")),
        "`--package=` names a package too: {found:#?}"
    );
    assert!(
        found
            .iter()
            .any(|finding| finding.reason.contains("xirang-second")),
        "a comma-separated list names several: {found:#?}"
    );
    let _ = fs::remove_dir_all(&root);
}

/// A dependency key is recognised however it is spaced, and a name that is
/// not a requirement is left alone.
/// 依赖键无论怎么加空格都能被认出，而不是要求位置的名称不受过问。
#[test]
fn only_requirement_positions_are_read() {
    let names = requirement_names(
        "xirang-a = { version = \"0.1\" }\nxirang-b={ version = \"0.1\" }\n\
             let id = format!(\"xirang-{label}-1\");\nprose about xirang-c in a sentence\n\
             [dependencies]\nxirang-d = { package = \"xirang-e\", path = \"../d\" }\n",
    );
    assert_eq!(
        names,
        vec![
            "xirang-a".to_owned(),
            "xirang-b".to_owned(),
            "xirang-d".to_owned(),
            "xirang-e".to_owned(),
        ],
        "{names:#?}"
    );
    assert_eq!(
        package_arguments("cargo test -p xirang-x --offline\nrun-a-pipeline\n"),
        vec!["xirang-x".to_owned()]
    );
}

/// A comment is not a requirement position. The boundary this gate states is "only positions
/// where a name is a *requirement*", and a `# … xirang-old` note in a workflow or a
/// `// … xirang-old = { … }` note in a template is a record of what was once run, not a
/// requirement — reporting it asks the author to rewrite history to satisfy a gate. The
/// string literal beside it is still read: templates write the requirement *inside* one.
/// 注释不是要求位置。本门禁声明的边界是"只读名字处于**要求**位置的地方"，而工作流里一句
/// `# … xirang-old` 或模板里一句 `// … xirang-old = { … }` 是"曾经这么跑过"的记录而不是要求，
/// 报出来等于要求作者为了满足门禁去改写记录。它旁边那句字符串字面量仍会被读：模板正是把要求写在
/// 字面量**里面**的。
///
/// Audit `G-07`: both halves were read through a raw scan, so both comments were reported.
/// 审计 `G-07`：两半边都在原文上扫描，因此两处注释都被报了出来。
#[test]
fn a_name_in_a_comment_is_not_a_requirement() {
    let root = synthetic(&[(
        "thing",
        "[package]\nname = \"xirang-thing\"\n\n[lib]\nname = \"xirang_thing\"\n",
    )]);
    fs::create_dir_all(root.join("toolchain/src/build_method/scaffold")).expect("scaffold dir");
    fs::write(
        root.join("toolchain/src/build_method/scaffold/probe.rs"),
        "// old invocation kept for reference: xirang-old = { path = \"..\" }\n\
         let requirement = \"xirang-gone = { path = \\\"../gone\\\" }\";\n",
    )
    .expect("template file");
    fs::create_dir_all(root.join(".github/workflows")).expect("workflow dir");
    fs::write(
        root.join(".github/workflows/ci.yml"),
        "run: cargo test -p xirang-thing --offline\n# old: cargo test -p xirang-old\n",
    )
    .expect("workflow file");

    let found = findings(&root);
    assert_eq!(
        found.len(),
        1,
        "only the live requirement is a requirement position: {found:#?}"
    );
    assert!(
        found[0].reason.contains("xirang-gone"),
        "the requirement inside the template's string is still read: {found:#?}"
    );
    let _ = fs::remove_dir_all(&root);
}

/// The workspace's own names agree.
/// 工作区自己的名字是一致的。
#[test]
fn the_shipped_crates_follow_the_rule() {
    let found = findings(&crate::workspace_root());
    assert!(
        found.is_empty(),
        "crate, directory and lib names have to agree; a host writes all three: {found:#?}"
    );
}

/// The `package = "…"` half has to read the spelling templates actually write.
/// `package = "…"` 那半边必须读得出模板真正书写的拼法。
///
/// A template is Rust source, so its manifest line sits inside a string literal and its quotes
/// are escaped: `package = \"xirang-x\"`. Reading only the unescaped needle kept the loss
/// invisible while a dependency key and its package name agreed — the key half found the name
/// anyway. A *renamed* dependency has only the field, so the template below names a package
/// that does not exist and nothing was reported (audit `G-07`, finding `F-3`). The pin lives here
/// rather than in `naming.rs` because a test-only file is exempt from the ceiling the size gate
/// enforces.
/// 模板是 Rust 源码，因此它的清单行写在字符串字面量里、引号是转义的：`package = \"xirang-x\"`。
/// 只认未转义的针时，只要依赖键与包名一致，这个损失就不可见——键那半边反正能找到那个名字。
/// **被重命名**的依赖只有这个字段，因此下面这条模板点名了一个不存在的包，却什么也没报出来
/// （审计 `G-07`，发现 `F-3`）。钉子放在这里而不是 `naming.rs`：仅测试文件不受尺寸门禁的上限
/// 约束。
#[test]
fn a_renamed_dependency_is_named_through_its_escaped_package_field() {
    let root = synthetic(&[(
        "thing",
        "[package]\nname = \"xirang-thing\"\n\n[lib]\nname = \"xirang_thing\"\n",
    )]);
    fs::create_dir_all(root.join("toolchain/src/build_method/scaffold")).expect("scaffold dir");
    fs::write(
        root.join("toolchain/src/build_method/scaffold/probe.rs"),
        "let r = \"runtime = { package = \\\"xirang-missing\\\", path = \\\"..\\\" }\";\n",
    )
    .expect("template");
    let found = findings(&root);
    assert_eq!(
        found.len(),
        1,
        "only the escaped package field names it: {found:#?}"
    );
    assert!(found[0].reason.contains("xirang-missing"), "{found:#?}");
    let _ = fs::remove_dir_all(&root);
}

/// The plain spelling still reads, and `--package` is not a manifest field.
/// 普通拼法仍然有效，而 `--package` 不是清单字段。
#[test]
fn the_plain_spelling_still_reads_and_a_command_flag_is_not_a_field() {
    assert_eq!(
        requirement_names("kind = { package = \"xirang-missing\", path = \"..\" }\n"),
        vec!["xirang-missing".to_owned()]
    );
    assert_eq!(
        requirement_names("run: cargo build --package=\"xirang-missing\"\n"),
        Vec::<String>::new()
    );
}
