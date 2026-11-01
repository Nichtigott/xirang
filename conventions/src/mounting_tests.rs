//! Tests for the module-mounting gate.
//! 模块挂载门禁的测试。

use super::*;
use crate::shims::{SHIMS, missing_shims, xirang_reexports};
use crate::workspace_root;

/// The only `include!` is the generated plan, and no file is named `mod.rs`.
/// `include!` is recognised by the macro, not by one exact spelling: a
/// delimiter change (`include! { … }`) splices a module just as well, and the
/// workspace gate must see it.
/// `include!` 靠宏本身识别，而不是靠一种拼法：换个定界符（`include! { … }`）照样拼接模块，
/// 工作区门禁必须看见它。
#[test]
fn an_include_with_another_delimiter_is_still_a_splice() {
    let root = synthetic(&[(
        "toolchain/src/cli/zz_audit_probe.rs",
        "pub mod spliced {\n    include! {\"zz_body.rs\"}\n}\n",
    )]);
    let found = findings(&root);
    assert_eq!(
        found.includes.len(),
        1,
        "a brace-delimited include! is a splice too: {:#?}",
        found.includes
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// The `!` may sit on the next line: `include` and `!` are two tokens to rustc, and only
/// spaces and tabs were trimmed between them, so a line break hid the splice entirely.
/// `!` 可以在下一行：`include` 与 `!` 对 rustc 是两个 token，而两者之间过去只裁空格与制表符，
/// 于是换行把这次拼接完全藏了起来。
#[test]
fn an_include_whose_bang_is_on_the_next_line_is_still_a_splice() {
    let root = synthetic(&[(
        "toolchain/src/cli/zz_audit_probe.rs",
        "pub mod spliced {\n    include\n        !(\"zz_body.rs\");\n}\n",
    )]);
    let found = findings(&root);
    assert_eq!(
        found.includes.len(),
        1,
        "a `!` on the next line is the same macro: {:#?}",
        found.includes
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// A throwaway checkout with the given files under it.
/// 一个只含给定文件的一次性检出。
fn synthetic(files: &[(&str, &str)]) -> std::path::PathBuf {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "xirang-mounting-{}-{}-{sequence}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    for (relative, contents) in files {
        let path = root.join(relative);
        std::fs::create_dir_all(path.parent().expect("parent")).expect("fixture dir");
        std::fs::write(&path, contents).expect("fixture file");
    }
    crate::fixture_manifest(&root);
    root
}

/// 唯一的 `include!` 是生成计划，且没有文件叫 `mod.rs`。
#[test]
fn modules_are_mounted_without_splicing_identity() {
    let root = workspace_root();
    let found = findings(&root);
    assert!(
        found.mod_rs.is_empty(),
        "the workspace mounts modules with #[path], never mod.rs: {:#?}",
        found.mod_rs
    );
    assert_eq!(
        found.includes.len(),
        1,
        "exactly one include! mounts the generated plan; a second one would \
         splice a file and silently change the NodeId of every face it declares: {:#?}",
        found.includes
    );
    assert!(
        found.includes[0].contains("generated_lib.rs"),
        "the single include! must be host!()'s generated plan, found: {:#?}",
        found.includes
    );
}

/// The three spellings the line-based literal search missed.
/// 逐行字面搜索漏掉的三种写法。
#[test]
fn a_splice_is_seen_however_it_is_spelled() {
    let cases: &[(&str, &str)] = &[
        ("a space before the bang", "include ! (\"body.rs\");"),
        (
            "a delimiter on the next line",
            "include!\n        (\"body.rs\");",
        ),
        (
            "a comment between the name and the bang",
            "include/* spliced */!(\"body.rs\");",
        ),
        ("a brace delimiter", "include! { \"body.rs\" }"),
    ];
    for (shape, splice) in cases {
        let root = synthetic(&[(
            "zzprobe/src/lib.rs",
            &format!("pub mod spliced {{\n    {splice}\n}}\n"),
        )]);
        let found = findings(&root);
        assert_eq!(
            found.includes.len(),
            1,
            "{shape} must be seen as a splice: {:#?}",
            found.includes
        );
        let _ = std::fs::remove_dir_all(&root);
    }
}

/// The shipped kernel mounts every module file it owns: a file nothing declares
/// is a file nothing compiles.
/// 出厂内核挂载它拥有的每个模块文件：没有声明指名的文件就是不会被编译的文件。
#[test]
fn the_shipped_kernel_mounts_every_module_file() {
    let found = mounts(&workspace_root());
    assert!(
        found.unmounted.is_empty(),
        "every kernel module file must be named by a declaration: {:#?}",
        found.unmounted
    );
    assert!(
        found.dangling.is_empty(),
        "every declaration must resolve to a file: {:#?}",
        found.dangling
    );
    assert!(
        found.duplicated.is_empty(),
        "no file may be mounted twice: {:#?}",
        found.duplicated
    );
}

/// A kernel module file no declaration names is reported, which is the whole
/// point: `cargo package --list` would still ship it.
/// 没有任何声明指名的内核模块文件会被报出，而这正是重点：`cargo package --list` 照样会把它
/// 打进包里。
#[test]
fn a_kernel_module_that_nothing_declares_is_reported() {
    let root = synthetic(&[
        ("kernel/src/lib.rs", "pub mod registry_core;\n"),
        (
            "kernel/src/registry_core.rs",
            "#[path = \"registry_core/tree/tree.rs\"]\npub mod tree;\n",
        ),
        (
            "kernel/src/registry_core/tree/tree.rs",
            "pub struct Tree;\n",
        ),
        (
            "kernel/src/registry_core/tree/forgotten/forgotten.rs",
            "pub struct Forgotten;\n",
        ),
    ]);
    let found = mounts(&root);
    assert_eq!(
        found.unmounted,
        vec!["kernel/src/registry_core/tree/forgotten/forgotten.rs".to_owned()],
        "an unregistered kernel module must be named: {found:#?}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// A `#[path]` that names no file is reported, so a rename cannot leave the
/// declaration pointing at a module that no longer exists.
/// 指不到文件的 `#[path]` 会被报出，因此重命名不会留下一条指向已不存在模块的声明。
#[test]
fn a_path_attribute_that_names_no_file_is_reported() {
    let root = synthetic(&[
        ("kernel/src/lib.rs", "pub mod registry_core;\n"),
        (
            "kernel/src/registry_core.rs",
            "#[path = \"registry_core/tree/missing.rs\"]\npub mod tree;\n",
        ),
        ("kernel/src/registry_core/tree.rs", "pub struct Tree;\n"),
    ]);
    let found = mounts(&root);
    assert_eq!(found.dangling.len(), 1, "{found:#?}");
    assert!(
        found.dangling[0].contains("registry_core/tree/missing.rs"),
        "{found:#?}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// A declaration may carry other attributes around its `#[path]`, and a bare
/// `mod x;` inside a module directory resolves to `x/x.rs`.
/// 声明可以在 `#[path]` 前后带其他属性；模块目录里的裸 `mod x;` 解析到 `x/x.rs`。
#[test]
fn a_path_attribute_is_found_among_other_attributes() {
    let root = synthetic(&[
        ("kernel/src/lib.rs", "pub mod registry_core;\n"),
        (
            "kernel/src/registry_core.rs",
            "#[path = \"registry_core/tree/tree.rs\"]\npub mod tree;\n",
        ),
        (
            "kernel/src/registry_core/tree/tree.rs",
            "#[cfg(test)]\n#[path = \"leaf/leaf.rs\"]\nmod leaf;\nmod nested;\n",
        ),
        ("kernel/src/registry_core/tree/leaf/leaf.rs", "fn x() {}\n"),
        (
            "kernel/src/registry_core/tree/nested/nested.rs",
            "fn y() {}\n",
        ),
    ]);
    let found = mounts(&root);
    assert!(found.unmounted.is_empty(), "{found:#?}");
    assert!(found.dangling.is_empty(), "{found:#?}");
    let _ = std::fs::remove_dir_all(&root);
}

/// A file named `lib.rs` is an entry point only where cargo looks for one: a nested
/// `…/zz/lib.rs` is a module file like any other, and skipping every file by that *name*
/// left it invisible to this gate.
/// 名叫 `lib.rs` 的文件只在 cargo 会去找它的那个位置才是入口：嵌套的 `…/zz/lib.rs` 与别的模块
/// 文件没有区别，而按**名字**跳过每个这样的文件让它对本门禁隐形。
#[test]
fn a_nested_lib_rs_is_a_module_like_any_other() {
    let root = synthetic(&[
        ("kernel/src/lib.rs", "pub mod registry_core;\n"),
        ("kernel/src/registry_core.rs", "pub mod zz;\n"),
        ("kernel/src/registry_core/zz/lib.rs", "fn hidden() {}\n"),
    ]);
    let found = mounts(&root);
    assert!(
        found
            .unmounted
            .iter()
            .any(|path| path.contains("registry_core/zz/lib.rs")),
        "an undeclared nested `lib.rs` is unmounted: {found:#?}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// A pinned re-export behind `#[cfg(any())]` (never compiled) or `#[cfg(test)]` (not part
/// of the public surface) is in the file but not in the crate a host compiles against, so
/// both spellings count as missing.
/// 藏在 `#[cfg(any())]`（永不编译）或 `#[cfg(test)]`（不属于公开面）后面的钉住重导出，存在于文件
/// 里却不在宿主编译所对的那个 crate 里，因此两种拼法都算缺失。
#[test]
fn a_pinned_reexport_behind_a_cfg_is_missing() {
    let (file, statement) = SHIMS[0];
    for attribute in ["#[cfg(any())]", "#[cfg(test)]"] {
        let (root, _) = fixture_with_every_pin();
        let text = std::fs::read_to_string(root.join(file)).expect("fixture file");
        let hidden = text.replacen(statement, &format!("{attribute}\n{statement}"), 1);
        assert_ne!(text, hidden, "the statement is in the fixture file");
        std::fs::write(root.join(file), hidden).expect("rewrite the fixture file");
        let missing = missing_shims(&root);
        assert!(
            missing
                .iter()
                .any(|entry| entry == &format!("{file}: {statement}")),
            "`{attribute}` hides the re-export from the built crate: {missing:#?}"
        );
        let _ = std::fs::remove_dir_all(&root);
    }
}

/// The execution surfaces still carry every pinned shim: deleting one compiles
/// and silently removes a path downstream hosts write.
/// 执行面仍然带着每一条钉住的 shim：删掉一条能编译，却会静默移除下游宿主书写的路径。
#[test]
fn the_shipped_shims_are_still_there() {
    let missing = missing_shims(&workspace_root());
    assert!(
        missing.is_empty(),
        "a pinned re-export disappeared; add it back or update SHIMS deliberately: {missing:#?}"
    );
}

/// A fixture checkout carrying every pinned statement, so a test can then damage one.
/// 一个带着全部钉住语句的夹具检出，供测试随后破坏其中一条。
fn fixture_with_every_pin() -> (std::path::PathBuf, Vec<(String, String)>) {
    let mut files: Vec<(String, String)> = Vec::new();
    for (file, statement) in SHIMS {
        match files.iter_mut().find(|(name, _)| name == file) {
            Some((_, contents)) => {
                contents.push('\n');
                contents.push_str(statement);
                contents.push('\n');
            }
            None => files.push(((*file).to_owned(), format!("{statement}\n"))),
        }
    }
    let borrowed: Vec<(&str, &str)> = files
        .iter()
        .map(|(name, contents)| (name.as_str(), contents.as_str()))
        .collect();
    (synthetic(&borrowed), files)
}

/// Removing one pinned statement from its file is reported.
/// 从文件里删掉一条钉住的语句会被报出。
#[test]
fn a_deleted_shim_is_reported() {
    let (root, files) = fixture_with_every_pin();
    assert!(
        missing_shims(&root).is_empty(),
        "the fixture carries every pinned statement"
    );

    // Drop the last statement of one file and re-check.
    let (file, contents) = files.last().expect("at least one pinned shim");
    let shortened = contents
        .rsplit_once("pub use xirang_kernel::")
        .map(|(head, _)| head.to_owned())
        .expect("a pinned statement to remove");
    let path = root.join(file);
    std::fs::write(&path, shortened).expect("rewrite the fixture file");
    let missing = missing_shims(&root);
    assert_eq!(missing.len(), 1, "{missing:#?}");
    assert!(missing[0].starts_with(file), "{missing:#?}");
    let _ = std::fs::remove_dir_all(&root);
}

/// The extraction normalises whitespace, so a `rustfmt` re-wrap of a braced
/// list does not read as a deleted shim.
/// 提取会规范化空白，因此花括号列表被 `rustfmt` 重新折行不会被读成删掉的 shim。
#[test]
fn a_reexport_is_recognised_across_line_breaks() {
    let found = xirang_reexports("pub use xirang_kernel::{\n    A,\n    B,\n};\n");
    assert_eq!(found, vec!["pub use xirang_kernel::{ A, B, };".to_owned()]);
}
