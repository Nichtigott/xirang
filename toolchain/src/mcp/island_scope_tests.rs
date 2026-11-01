//! The write-scope boundary tests: what a declared (read-only) source refuses, for every writer.
//! 写入作用域边界测试：已声明（只读）来源对每个写入方拒绝什么。
//!
//! Split out of `island_tests.rs` for the size ratchet. What they pin is not layout but **permission**:
//! a child in a declared source is kept rather than rewritten, a structural rename may not move the
//! third party's tree, a move may not publish into it, and an unwritable reference is refused by name
//! with executable options.
//! 从 `island_tests.rs` 拆出，为尺寸棘轮。它们钉的不是布局而是**许可**：已声明来源里的子面被保留而不是
//! 改写、结构改名不得搬动第三方的树、搬动不得往它里面发布、写不了的引用被按名拒绝并给出可执行出路。

#![cfg(feature = "lang-cpp")]

use std::path::Path;

use serde_json::json;

use super::island_tests::{apply, island, package};

/// A child that lives in a declared (read-only) source is **kept**, not rewritten: the move changes
/// the parent's module path, and the generated header's compatibility namespace makes the child's
/// existing `.under(...)` resolve to the moved object.
/// 住在已声明（只读）来源里的子面被**保留**，而不是改写：搬动改变了父级的模块路径，而生成的头部兼容
/// 命名空间让子面既有的 `.under(...)` 解析到被搬的对象。
///
/// This is the half the review asked to prefer over "edit the third party": the external file keeps
/// its bytes, the tree keeps working, and the reply says why. The other half — a reference that
/// cannot be kept stable — is pinned by
/// [`a_declared_source_is_a_read_permission_not_a_write_permission`].
/// 这是复核要求优先于"去改第三方"的那一半：外部文件保留原字节、树继续工作、回复说出原因。另一半——
/// 无法保持稳定的引用——由 [`a_declared_source_is_a_read_permission_not_a_write_permission`] 钉住。
#[test]
fn a_child_in_a_declared_source_is_kept_stable_instead_of_rewritten() {
    use xirang_multilang::source_map::{
        BINDINGS_FILE, BINDINGS_HEADER, SOURCES_FILE, SOURCES_HEADER,
    };

    let root = package("external-child");
    apply(
        &root,
        json!({"action":"add","fields":{"module":"panel","kind":"Panel","needs_registry":true},"apply":true}),
    )
    .expect("Panel is created");
    apply(
        &root,
        json!({"action":"add","language":"cpp","parent":"root/panel",
               "file":"components/area/area.cpp",
               "fields":{"module":"area","kind":"Area","needs_registry":true},"apply":true}),
    )
    .expect("Area is created");
    apply(
        &root,
        json!({"action":"add","language":"cpp","parent":"root/panel/Area",
               "file":"vendor/ui/gauge.cpp",
               "fields":{"module":"gauge","kind":"Gauge"},"apply":true}),
    )
    .expect("Gauge is created in a directory that becomes a declared source");

    // Move the child's directory out of the package and leave a declared source in its place: the
    // child is now **readable** through the declaration and **not writable**.
    // 把子面的目录搬出包外，在原处留一个已声明来源：子面此刻**可读**、**不可写**。
    let outside = std::env::temp_dir().join(format!(
        "xirang-external-child-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    std::fs::create_dir_all(&outside).expect("outside dir");
    let gauge = "vendor/ui/gauge.cpp";
    std::fs::copy(root.join(gauge), outside.join("gauge.cpp")).expect("copy out");
    std::fs::remove_dir_all(root.join("vendor/ui")).expect("remove the materialization");
    std::os::unix::fs::symlink(&outside, root.join("vendor/ui")).expect("symlink");
    std::fs::write(
        root.join(SOURCES_FILE),
        format!("{SOURCES_HEADER}\nvendor-ui\tlocal\tvendor/ui\n"),
    )
    .expect("declaration");
    std::fs::create_dir_all(root.join(".xirang")).expect("machine dir");
    std::fs::write(
        root.join(BINDINGS_FILE),
        format!("{BINDINGS_HEADER}\nvendor-ui\t{}\n", outside.display()),
    )
    .expect("binding");
    let before = std::fs::read_to_string(outside.join("gauge.cpp")).expect("outside file");

    let reply = apply(
        &root,
        json!({"action":"move","node":"root/panel/Area","to":"root/panel",
               "name":"area2","apply":true}),
    )
    .expect("a child in a declared source does not block the move");
    assert!(
        reply.contains("kept `vendor/ui/gauge.cpp`"),
        "the reply says the child was kept: {reply}"
    );
    assert!(
        reply.contains("compatibility catalog alias") || reply.contains("compatibility namespace"),
        "and why that is enough: {reply}"
    );
    assert_eq!(
        std::fs::read_to_string(outside.join("gauge.cpp")).expect("outside file"),
        before,
        "the declared source was not written through"
    );
    assert!(
        reply.contains("components/area/area2.cpp"),
        "the moved declaration itself still landed: {reply}"
    );
    let _ = std::fs::remove_dir_all(root);
    let _ = std::fs::remove_dir_all(outside);
}

/// A declared source is a **read** permission, not a write permission: a consumer inside a declared
/// external source that names the moved alias makes the move refuse and name the file, instead of
/// reaching through the link and editing a file outside this package.
/// 已声明来源是**读**权限，不是写权限：已声明外部来源里点名被搬别名的消费者会让搬动拒绝并点名那个文件，
/// 而不是穿过链接去编辑本包之外的文件。
#[test]
fn a_declared_source_is_a_read_permission_not_a_write_permission() {
    use xirang_multilang::source_map::{
        BINDINGS_FILE, BINDINGS_HEADER, SOURCES_FILE, SOURCES_HEADER,
    };

    let root = island();
    let outside = std::env::temp_dir().join(format!(
        "xirang-island-declared-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    let _ = std::fs::remove_dir_all(&outside);
    std::fs::create_dir_all(&outside).expect("outside dir");
    let ident = crate::build_method::declared_native::ident("components/area/area.cpp", "Area");
    let consumer = format!("use crate::xirang_external::{ident}::NODE_ID;\n");
    std::fs::write(outside.join("consumer.rs"), &consumer).expect("outside file");
    std::fs::create_dir_all(root.join("vendor")).expect("vendor dir");
    #[cfg(unix)]
    std::os::unix::fs::symlink(&outside, root.join("vendor/ui")).expect("symlink");
    std::fs::write(
        root.join(SOURCES_FILE),
        format!("{SOURCES_HEADER}\nvendor-ui\tlocal\tvendor/ui\n"),
    )
    .expect("declaration");
    std::fs::create_dir_all(root.join(".xirang")).expect("machine dir");
    std::fs::write(
        root.join(BINDINGS_FILE),
        format!("{BINDINGS_HEADER}\nvendor-ui\t{}\n", outside.display()),
    )
    .expect("binding");

    let error = apply(
        &root,
        json!({"action":"move","node":"root/panel/Area","to":"root/panel",
               "name":"area_moved","apply":true}),
    )
    .expect_err("a declaration is not a write permission");
    // The refusal names **every** member and the file, says nothing was written, and offers a way
    // forward — a bare "not allowed" would leave the caller to guess which file and what to do.
    // 拒绝点名**每一个**成员与那个文件，说明什么都没写，并给出一条出路——干巴巴一句"不允许"会让调用方
    // 去猜是哪个文件、该怎么办。
    assert!(error.contains("live outside the package"), "{error}");
    assert!(
        error.contains("consumer.rs"),
        "the refusal names the file: {error}"
    );
    assert!(error.contains("nothing was written"), "{error}");
    assert!(error.contains("way forward"), "{error}");
    assert!(
        root.join("components/area/area.cpp").is_file(),
        "the declaration never moved"
    );
    assert!(
        !root.join("components/area/area_moved.cpp").exists(),
        "and nothing was written at the destination"
    );
    assert_eq!(
        std::fs::read_to_string(outside.join("consumer.rs")).expect("outside file"),
        consumer,
        "the declared source was not written through"
    );
    let _ = std::fs::remove_dir_all(root);
    let _ = std::fs::remove_dir_all(outside);
}

/// A structural rename (or a kind edit) of a face whose module directory is a declared-source link is
/// bounded by the same rule as every other write: it must not move the third party's tree.
/// 一个模块目录是已声明来源链接的面，其结构改名（或改 kind）受与其它写入相同的规则约束：它不得搬动
/// 第三方的树。
///
/// The independent review reproduced this as a blocker: `apply rename` and `apply edit` with a module
/// change took the structural-migration path, which renamed and rewrote files with no write scope and
/// no transaction — returning **success** while editing outside the package.
/// 独立复核把这个复现为 blocker：`apply rename` 与带模块变更的 `apply edit` 走结构迁移路径，那条路径在没有
/// 写入作用域、也没有事务的情况下改名并改写文件——在包外被编辑的同时返回**成功**。
#[test]
fn a_structural_rename_cannot_move_a_declared_source() {
    use xirang_multilang::source_map::{
        BINDINGS_FILE, BINDINGS_HEADER, SOURCES_FILE, SOURCES_HEADER,
    };

    let root = package("rename-external");
    apply(
        &root,
        json!({"action":"add","fields":{"module":"panel","kind":"Panel","needs_registry":true},"apply":true}),
    )
    .expect("Panel is created");
    // Move the module directory out and leave a **declared** link in its place: readable, and a
    // rename would move the third party's directory.
    // 把模块目录搬出去，在原处留一个**已声明**链接：可读，而改名会搬动第三方的目录。
    let outside = std::env::temp_dir().join(format!(
        "xirang-rename-external-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    std::fs::create_dir_all(&outside).expect("outside dir");
    std::fs::rename(root.join("src/panel"), outside.join("panel")).expect("move the module out");
    std::fs::write(
        outside.join("panel/panel.rs"),
        std::fs::read_to_string(outside.join("panel/panel.rs")).expect("the face"),
    )
    .expect("rewrite");
    std::os::unix::fs::symlink(outside.join("panel"), root.join("src/panel")).expect("link");
    std::fs::write(
        root.join(SOURCES_FILE),
        format!("{SOURCES_HEADER}\npanel-ext\tlocal\tsrc/panel\n"),
    )
    .expect("declaration");
    std::fs::create_dir_all(root.join(".xirang")).expect("machine dir");
    std::fs::write(
        root.join(BINDINGS_FILE),
        format!(
            "{BINDINGS_HEADER}\npanel-ext\t{}\n",
            outside.join("panel").display()
        ),
    )
    .expect("binding");
    let before = std::fs::read_to_string(outside.join("panel/panel.rs")).expect("outside face");

    let error = apply(
        &root,
        json!({"action":"rename","node":"root/panel","fields":{"module":"panel2"},"apply":true}),
    )
    .expect_err("a structural rename may not move a declared source");
    assert!(
        error.contains("would write outside the package"),
        "the refusal says what it refused: {error}"
    );
    assert!(error.contains("panel"), "and names the path: {error}");
    assert_eq!(
        std::fs::read_to_string(outside.join("panel/panel.rs")).expect("outside face"),
        before,
        "the outside source is untouched"
    );
    assert!(
        outside.join("panel").is_dir(),
        "the third party's directory was not moved"
    );
    let _ = std::fs::remove_dir_all(root);
    let _ = std::fs::remove_dir_all(outside);
}

/// A Rust move authorizes its **publication** writes too: the migration record, the ledger and the
/// graft rebinds are in the same transaction, so they are in the same write set.
/// Rust 搬动也授权它的**发布**写入：迁移记录、台账与嫁接重绑在同一个事务里，因此也在同一个写入集合里。
///
/// The independent review reproduced this as a high: the Rust branch authorized only the source, the
/// destination and the rewrites, then wrote the record outside the package when `.xirang` was a link.
/// 独立复核把这个复现为 high：Rust 分支只授权了源码、目的地与改写，随后在 `.xirang` 是链接时把记录写到了
/// 包外。
#[test]
fn a_rust_move_cannot_publish_outside_the_package() {
    let root = package("rust-move-record");
    apply(
        &root,
        json!({"action":"add","fields":{"module":"panel","kind":"Panel","needs_registry":true},"apply":true}),
    )
    .expect("Panel is created");
    let outside = std::env::temp_dir().join(format!(
        "xirang-move-record-out-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    std::fs::create_dir_all(&outside).expect("outside dir");
    std::os::unix::fs::symlink(&outside, root.join(".xirang")).expect("the .xirang link");
    // Declared, so the readers follow it: the shape the write scope has to stop is "readable, not
    // writable", and an undeclared link is refused by the scanner before this code is reached.
    // 已声明，因此读取方会跟随它：写入作用域必须挡住的形状是"可读、不可写"；未声明的链接在到达这段代码
    // 之前就被扫描器拒绝了。
    std::fs::write(
        root.join("xirang.sources.tsv"),
        "# xirang sources v1\nxirang-dir\tlocal\t.xirang\n",
    )
    .expect("the declaration");
    // The binding file lives **inside** the materialization (the link points at it), and it names the
    // machine root this materialization stands for.
    // 绑定文件住在**物化位置里面**（链接指向它），并点名这次物化代表的本机根。
    std::fs::write(
        outside.join("source-bindings.tsv"),
        format!(
            "# xirang source bindings v1\nxirang-dir\t{}\n",
            outside.display()
        ),
    )
    .expect("the binding");
    let before = is_files(&root);
    let error = apply(
        &root,
        json!({"action":"move","node":"root/panel","to":"root","name":"panel2","apply":true}),
    )
    .expect_err("publication outside the package is refused");
    assert!(
        error.contains("would have to write outside the package")
            || error.contains("would write outside the package"),
        "the refusal says what it refused: {error}"
    );
    assert!(
        error.contains("move.plan") || error.contains("migration record"),
        "and names the publication path: {error}"
    );
    assert_eq!(is_files(&root), before, "nothing was written");
    assert!(
        root.join("src/panel/panel.rs").is_file(),
        "the declaration never moved"
    );
    assert!(
        !outside.join("moves").exists(),
        "no migration record was published into the third party's directory"
    );
    let _ = std::fs::remove_dir_all(root);
    let _ = std::fs::remove_dir_all(outside);
}

/// The project's own files, without the build's products.
/// 项目自己的文件，不含构建产物。
fn is_files(root: &Path) -> Vec<String> {
    let mut files = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(directory) = stack.pop() {
        for entry in std::fs::read_dir(&directory).expect("readable").flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().into_owned();
            if name == "target" {
                continue;
            }
            if path.is_dir() {
                stack.push(path);
            } else {
                files.push(
                    path.strip_prefix(root)
                        .unwrap_or(&path)
                        .display()
                        .to_string(),
                );
            }
        }
    }
    files.sort();
    files
}

/// A delete may not move a **declared** source into the trash.
/// 删除不得把**已声明**来源搬进回收目录。
///
/// Delete is the second writer that moves a whole directory, so it needs the same boundary the
/// structural rename has: the directory's **real** location is outside the package, and the trash
/// move would take the third party's tree with it. The rule is the same `WriteScope`; this is the
/// pin that says the delete path asks it.
/// 删除是第二个搬动整个目录的写入方，因此需要与结构改名相同的边界：目录的**真实**位置在包外，而搬进回收
/// 目录会把第三方的树一起带走。规则是同一个 `WriteScope`；这条钉子说的就是删除路径确实问了它。
#[test]
fn a_delete_cannot_move_a_declared_source_into_trash() {
    use xirang_multilang::source_map::{
        BINDINGS_FILE, BINDINGS_HEADER, SOURCES_FILE, SOURCES_HEADER,
    };

    let root = package("delete-external");
    apply(
        &root,
        json!({"action":"add","fields":{"module":"panel","kind":"Panel","needs_registry":true},"apply":true}),
    )
    .expect("Panel is created");
    let outside = std::env::temp_dir().join(format!(
        "xirang-delete-external-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    std::fs::create_dir_all(&outside).expect("outside dir");
    std::fs::rename(root.join("src/panel"), outside.join("panel")).expect("move the module out");
    std::os::unix::fs::symlink(outside.join("panel"), root.join("src/panel")).expect("link");
    std::fs::write(
        root.join(SOURCES_FILE),
        format!("{SOURCES_HEADER}\npanel-ext\tlocal\tsrc/panel\n"),
    )
    .expect("declaration");
    std::fs::create_dir_all(root.join(".xirang")).expect("machine dir");
    std::fs::write(
        root.join(BINDINGS_FILE),
        format!(
            "{BINDINGS_HEADER}\npanel-ext\t{}\n",
            outside.join("panel").display()
        ),
    )
    .expect("binding");
    let before = std::fs::read_to_string(outside.join("panel/panel.rs")).expect("outside face");

    let error = apply(
        &root,
        json!({"action":"delete","node":"root/panel","confirm":true,"apply":true}),
    )
    .expect_err("a delete may not move a declared source");
    assert!(
        error.contains("outside the package"),
        "the refusal says what it refused: {error}"
    );
    assert!(error.contains("panel"), "and names the path: {error}");
    assert_eq!(
        std::fs::read_to_string(outside.join("panel/panel.rs")).expect("outside face"),
        before,
        "the outside source is untouched"
    );
    assert!(
        outside.join("panel").is_dir(),
        "the third party's directory is not in the trash"
    );
    let _ = std::fs::remove_dir_all(root);
    let _ = std::fs::remove_dir_all(outside);
}
