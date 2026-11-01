//! Tests for the source index and its path safety.
//! 源码索引及其路径安全的测试。

use super::*;

/// The write path's recoverable trash lives under `.xirang/` and holds
/// `.rs` files, so a deleted fact must not keep answering `status` and
/// `search` from its own backup.
/// 写入路径的可恢复回收目录在 `.xirang/` 下、里面就是 `.rs` 文件，因此被删掉的东西不得继续
/// 从它自己的备份里回答 `status` 与 `search`。
#[test]
fn the_recoverable_trash_is_not_indexed() {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!("mcp-scan-{}-{sequence}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("src")).expect("source directory");
    std::fs::create_dir_all(root.join(".xirang/trash/faces")).expect("trash directory");
    std::fs::write(root.join("src/live.rs"), "pub fn live() {}\n").expect("live file");
    std::fs::write(
        root.join(".xirang/trash/faces/deleted.rs"),
        "pub fn deleted() {}\n",
    )
    .expect("backup file");
    let files = load_sources(&root).expect("the scan reads the tree");
    let names = files
        .iter()
        .map(|file| file.relative.clone())
        .collect::<Vec<_>>();
    assert!(names.iter().any(|name| name == "src/live.rs"), "{names:?}");
    assert!(
        !names.iter().any(|name| name.contains(".xirang")),
        "the trash must not be indexed: {names:?}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn parser_indexes_functions_and_direct_calls() {
    let functions = parse_functions(
        "fn source() { let value = helper(1); sink(value); }\nfn helper(_: i32) {}\nfn sink(_: i32) {}",
    );
    assert_eq!(functions.len(), 3);
    assert_eq!(functions[0].name, "source");
    assert_eq!(functions[0].calls, ["helper", "sink"]);
}

/// The index **carries** the kernel's branch facts instead of deciding them: an indexed file
/// reports the literal-`false` guard with its function and line, the qualified variant pattern, the
/// enum declaration, and the construction the tree spells — all four as `kernel::source` read them.
/// 索引**搬运**内核的分支事实，而不是自己判定它们：一个已索引的文件报出带函数名与行号的字面量
/// `false` 守卫、限定变体模式、枚举声明，以及本树拼出的构造——四者都按 `kernel::source` 读到的样子。
#[test]
fn the_index_carries_the_kernels_branch_facts() {
    let root = temporary_root("branches");
    fs::write(
        root.join("bands.rs"),
        "enum Band { Small, Frozen }\n\
         \n\
         fn band_word(band: Band) -> &'static str {\n\
         \x20   if false { return \"unreachable\"; }\n\
         \x20   match band {\n\
         \x20       Band::Small => \"small\",\n\
         \x20       Band::Frozen => \"frozen\",\n\
         \x20   }\n\
         }\n\
         \n\
         fn first_band() -> Band { Band::Small }\n",
    )
    .expect("write");
    let sources = load_sources(&root).expect("the walk reads the root");
    let branches = &sources[0].branches;
    assert_eq!(
        branches
            .false_guards
            .iter()
            .map(|guard| (guard.function.as_str(), guard.line))
            .collect::<Vec<_>>(),
        [("band_word", 4)],
        "the guard is attributed to its function, at its own line"
    );
    assert_eq!(
        branches
            .matched_variants
            .iter()
            .map(|arm| (arm.enum_name.as_str(), arm.variant.as_str(), arm.line))
            .collect::<Vec<_>>(),
        [("Band", "Small", 6), ("Band", "Frozen", 7)],
        "both arms are read, with their own pattern lines"
    );
    assert_eq!(
        branches
            .enums
            .iter()
            .map(|declaration| (declaration.name.as_str(), declaration.judged))
            .collect::<Vec<_>>(),
        [("Band", true)],
        "the declaration's judgement rides along with the fact"
    );
    assert_eq!(
        branches
            .variant_paths
            .iter()
            .map(|path| (path.enum_name.as_str(), path.variant.as_str(), path.line))
            .collect::<Vec<_>>(),
        [("Band", "Small", 11)],
        "only the construction is a path; the two patterns are not"
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn registration_kinds_are_compact_and_deduplicated() {
    let kinds = xirang_kernel::source::registration_kinds(
        "crate::control_object! { kind: Button, }\ncrate::control_object! { kind: Button, }",
    );
    assert_eq!(kinds, ["Button"]);
}

/// A throwaway source root, unique per call.
/// 每次调用唯一的临时源码根。
fn temporary_root(tag: &str) -> PathBuf {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "xirang-toolchain-{tag}-{}-{sequence}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    fs::create_dir_all(&root).expect("create root");
    root
}

/// A file genuinely inside the root is read, so the guard is about location
/// and not about links as such.
/// 真正位于根内的文件照常读取，因此这道守卫针对的是位置而不是链接本身。
#[test]
fn a_file_inside_the_source_root_is_still_read() {
    let root = temporary_root("inside");
    fs::write(root.join("inside.rs"), "fn inside() {}\n").expect("write");
    let sources = load_sources(&root).expect("the walk reads the root");
    assert_eq!(sources.len(), 1);
    assert!(sources[0].source.contains("fn inside"));
    let _ = std::fs::remove_dir_all(&root);
}

/// A link out of the root is neither walked into nor read. A prefix
/// comparison accepts both, because the link's own path is inside the root.
/// 指向根外的链接既不会被进入，也读不到。前缀比较两者都会接受，因为链接自身的路径就在
/// 根内。
#[cfg(unix)]
#[test]
fn a_link_out_of_the_source_root_is_neither_walked_nor_read() {
    let root = temporary_root("escape");
    let outside = root.with_file_name(format!(
        "{}-outside",
        root.file_name().expect("a name").to_string_lossy()
    ));
    let _ = std::fs::remove_dir_all(&outside);
    fs::create_dir_all(&outside).expect("create outside tree");
    fs::write(outside.join("secret.rs"), "fn secret() {}\n").expect("write outside file");
    std::os::unix::fs::symlink(&outside, root.join("linked")).expect("link the outside tree");
    std::os::unix::fs::symlink(outside.join("secret.rs"), root.join("secret.rs"))
        .expect("link the outside file");

    let sources = load_sources(&root).expect("the walk survives an escaping link");
    assert!(
        sources
            .iter()
            .all(|file| !file.source.contains("fn secret")),
        "a file outside the root must not be indexed: {sources:?}"
    );
    let error = load_one(&root, "secret.rs").expect_err("a link out of the root is refused");
    assert!(error.contains("source root"), "{error}");

    let _ = std::fs::remove_dir_all(&root);
    let _ = std::fs::remove_dir_all(&outside);
}

/// A tree-relative path has one spelling, and it is the shared rule's: a file whose
/// *name* contains a backslash — legal bytes on Unix — is reported with `/`
/// separators here and in `preview`'s declaration anchors alike, because neither
/// decides the spelling for itself. This module used to fold only the platform
/// separator, so on Unix it reported `a\b.rs` where the anchor that names the same
/// file reported `a/b.rs`.
/// 树内相对路径只有一种拼法，而且来自那条共用规则：名字里含反斜杠的文件（Unix 上合法的字节）在这里
/// 与 `preview` 的声明锚点里都用 `/` 分隔符报告，因为两者都不自行决定拼法。本模块过去只折平台分隔符，
/// 因此在 Unix 上它报 `a\b.rs`，而命名同一个文件的锚点报 `a/b.rs`。
#[cfg(unix)]
#[test]
fn a_backslash_in_a_file_name_is_spelled_one_way() {
    let root = temporary_root("portable");
    fs::write(root.join(r"a\b.rs"), "fn inside() {}\n").expect("write");
    let sources = load_sources(&root).expect("the walk reads the root");
    let names = sources
        .iter()
        .map(|file| file.relative.clone())
        .collect::<Vec<_>>();
    assert_eq!(names, ["a/b.rs"], "{names:?}");
    let _ = std::fs::remove_dir_all(&root);
}

/// The containment refusal names the base it checked and the rule that makes the base matter: with
/// `root` set, a path is relative to that root, which is exactly what a caller who pasted a
/// tree-relative path needs to hear.
/// 越界拒绝点名它检查的基准，以及让基准要紧的那条规则：给了 `root` 时路径相对那个根——这正是一个照抄
/// 树相对路径的调用方需要听到的话。
#[test]
fn the_containment_refusal_names_the_base_and_the_rule() {
    let root = std::env::temp_dir().join(format!(
        "xirang-load-one-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    std::fs::create_dir_all(root.join("src")).expect("root");
    std::fs::write(root.join("src/lib.rs"), "// entry\n").expect("file");
    let error = load_one(&root, "../outside.rs").expect_err("a refusal");
    assert!(error.contains("must stay inside"), "{error}");
    assert!(error.contains("relative to that root"), "{error}");
    assert!(error.contains("`src/…`"), "{error}");
    assert!(error.contains(&root.display().to_string()), "{error}");
    let _ = std::fs::remove_dir_all(&root);
}
