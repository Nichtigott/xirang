//! The foreign move's transaction, against the actual file state: source, references, graft records
//! and the migration record are one recoverable set.
//! 外语面搬动的事务，对着真实文件状态：源码、引用、嫁接记录与迁移记录是一个可恢复集合。
//!
//! These pins exist because a real MCP probe reproduced the defect they cover: the record could not be
//! published, the call answered `isError: true`, and the tree had already moved. A pin that only
//! asserts "an error came back" would have passed against that defect, so every pin here compares the
//! **bytes on disk** before and after.
//! 这些钉子存在，是因为一次真实 MCP 探针复现了它们覆盖的缺陷：记录发布不出去、调用回了
//! `isError: true`，而树已经搬完了。只断言"返回了错误"的钉子在那个缺陷上会是绿的，因此这里的每条钉子
//! 都比对操作前后的**磁盘字节**。
#![cfg(feature = "lang-cpp")]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde_json::json;

use crate::run_method::authoring::write_transaction::fault;

/// Copy the mixed-language example outside the checkout, without its build output.
/// 把混合语言示例复制到检出之外，不带它的构建产物。
fn fixture(label: &str) -> PathBuf {
    let example = Path::new(env!("CARGO_MANIFEST_DIR")).join("../examples/mixed-shop/shop");
    let root = std::env::temp_dir().join(format!(
        "xirang-move-transaction-{label}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    let _ = std::fs::remove_dir_all(&root);
    copy_without_output(&example, &root);
    root
}

/// Copy every file except the build output, which is neither an input nor small.
/// 复制除构建产物之外的每个文件——构建产物既不是输入，也不小。
fn copy_without_output(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).expect("copy destination");
    for entry in std::fs::read_dir(from).expect("fixture tree") {
        let entry = entry.expect("fixture entry");
        let name = entry.file_name();
        if name == "target" || name == ".git" {
            continue;
        }
        let target = to.join(&name);
        if entry.file_type().expect("fixture type").is_dir() {
            copy_without_output(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), target).expect("fixture file");
        }
    }
}

/// Every file under `root`, keyed by its relative path.
/// `root` 下的每个文件，按相对路径为键。
///
/// Directories are not part of the answer on purpose: the question these pins ask is "was any byte
/// left where it was not", and an empty directory created and left behind by a rolled-back write is
/// exactly the kind of residue a byte-only comparison would miss — so the directory list is compared
/// separately, by [`directories`].
/// 目录有意不进入这个答案：这些钉子问的是"有没有字节被留在了不该在的地方"，而一次被回滚的写入留下一个
/// 空目录，正是只比字节会漏掉的那种残留——因此目录清单由 [`directories`] 单独比对。
fn snapshot(root: &Path) -> BTreeMap<String, Vec<u8>> {
    let mut files = BTreeMap::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(directory) = pending.pop() {
        let Ok(entries) = std::fs::read_dir(&directory) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let relative = path
                .strip_prefix(root)
                .expect("under the fixture root")
                .to_string_lossy()
                .replace('\\', "/");
            if path.is_dir() {
                pending.push(path);
            } else {
                files.insert(relative, std::fs::read(&path).expect("fixture bytes"));
            }
        }
    }
    files
}

/// Every directory under `root`, relative and sorted.
/// `root` 下的每个目录，相对路径并排序。
fn directories(root: &Path) -> Vec<String> {
    let mut found = Vec::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(directory) = pending.pop() {
        let Ok(entries) = std::fs::read_dir(&directory) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                found.push(
                    path.strip_prefix(root)
                        .expect("under the fixture root")
                        .to_string_lossy()
                        .replace('\\', "/"),
                );
                pending.push(path);
            }
        }
    }
    found.sort();
    found
}

/// Compare the tree before and after, and report **only the paths that differ**.
/// 比对操作前后的树，并且**只报告不同的路径**。
///
/// `assert_eq!` on two whole maps prints every byte of both when one file differs, which buries the
/// one fact a failing run needs. This states the differing paths and nothing else.
/// 对整个映射做 `assert_eq!` 会在一个文件不同时把两边的每个字节都打印出来，把失败运行唯一需要的那件事
/// 埋掉。这里只陈述不同的路径。
fn assert_unchanged(root: &Path, before: &BTreeMap<String, Vec<u8>>, what: &str) {
    let after = snapshot(root);
    let mut differences = Vec::new();
    for (path, bytes) in before {
        match after.get(path) {
            None => differences.push(format!("{path}: missing")),
            Some(current) if current != bytes => differences.push(format!("{path}: bytes changed")),
            Some(_) => {}
        }
    }
    for path in after.keys() {
        if !before.contains_key(path) {
            differences.push(format!("{path}: appeared"));
        }
    }
    assert!(differences.is_empty(), "{what}: {}", differences.join(", "));
}

/// A declaration that fails **after** it truncated its target must still come back: the original
/// bytes were captured before the write, not after it succeeded.
/// 一次在截断目标**之后**才失败的写入仍必须能被恢复：原始字节是在写入之前保存的，不是在写成功之后。
#[test]
fn a_partial_write_of_the_moved_declaration_restores_every_byte() {
    let root = fixture("partial-source");
    let before = snapshot(&root);
    let before_dirs = directories(&root);
    let destination = root.join("native/area/object/gauge2/gauge2.cpp");
    fault::arm(&destination, 3);

    let error = crate::mcp::apply::apply(
        &root,
        &json!({
            "action": "move",
            "node": "root/shop/panel/Area/Gauge",
            "to": "root/shop/panel/Area",
            "name": "gauge2",
            "apply": true
        }),
    )
    .expect_err("the injected write failure refuses the move");
    assert!(error.contains("injected failure"), "{error}");
    assert!(
        error.contains("were restored"),
        "the refusal says what happened to the bytes: {error}"
    );
    assert_unchanged(&root, &before, "every byte is where it was before the move");
    assert_eq!(
        directories(&root),
        before_dirs,
        "and no empty directory was left behind"
    );
    assert!(
        !destination.exists(),
        "the half-written destination is gone"
    );

    // The same request lands once nothing is armed: the failure was the seam, not the move.
    // 没有武装接缝之后同一个请求落地：失败来自接缝，而不是搬动本身。
    crate::mcp::apply::apply(
        &root,
        &json!({
            "action": "move",
            "node": "root/shop/panel/Area/Gauge",
            "to": "root/shop/panel/Area",
            "name": "gauge2",
            "apply": true
        }),
    )
    .expect("the retry lands");
    assert!(root.join("native/area/object/gauge2/gauge2.cpp").is_file());
    let _ = std::fs::remove_dir_all(root);
}

/// A reference rewrite that fails part-way restores the moved declaration **and** the file it was
/// rewriting: the moved face must not end up at a destination whose consumers still name the old one.
/// 写到一半失败的引用改写会把被搬声明**与**它正在改写的文件一起恢复：被搬的面不得停在一个"消费者仍点名
/// 旧身份"的目的地上。
#[test]
fn a_partial_reference_rewrite_restores_the_source_and_the_consumer() {
    let root = fixture("partial-reference");
    let before = snapshot(&root);
    let before_dirs = directories(&root);
    // The C++ child carries the `.under(...)` that follows the moved parent — the first reference
    // rewrite the planner produces for this move.
    // C++ 子面携带跟随被搬父级的 `.under(...)`——这次搬动的规划器产出的第一处引用改写。
    fault::arm(&root.join("native/area/gauge.cpp"), 12);

    let error = crate::mcp::apply::apply(
        &root,
        &json!({
            "action": "move",
            "node": "root/shop/panel/Area",
            "to": "root/shop",
            "name": "area_moved",
            "apply": true
        }),
    )
    .expect_err("the injected reference failure refuses the move");
    assert!(error.contains("injected failure"), "{error}");
    assert_unchanged(&root, &before, "the whole tree is unchanged");
    assert_eq!(directories(&root), before_dirs);
    assert!(
        !root
            .join("src/shop/panel/object/area_moved/area_moved.cpp")
            .exists(),
        "the moved declaration did not stay at its destination"
    );
    let _ = std::fs::remove_dir_all(root);
}

/// A graft record rewrite that fails restores the move: the record and the moved bytes belong to the
/// same transaction, so one cannot survive without the other.
/// 失败的嫁接记录改写会把搬动恢复回去：记录与被搬的字节属于同一个事务，因此不可能只活下一个。
#[test]
fn a_failed_graft_record_rewrite_restores_the_move() {
    let root = fixture("partial-graft");
    let views = crate::build_method::face_views(&root, "shop").expect("faces");
    let gauge = views
        .iter()
        .find(|view| view.source == "native/area/gauge.cpp")
        .expect("gauge face");
    let record = root.join(".xirang/external-grafts/impl_gauge/graft.plan");
    std::fs::create_dir_all(record.parent().expect("record dir")).expect("record dir");
    std::fs::write(
        &record,
        format!(
            "version=1\ntarget={}\ntarget_path={}\ngraft=impl_gauge\nfull=false\n",
            gauge.id, gauge.path
        ),
    )
    .expect("record");
    let before = snapshot(&root);
    let before_dirs = directories(&root);
    fault::arm(&record, 5);

    let error = crate::mcp::apply::apply(
        &root,
        &json!({
            "action": "move",
            "node": "root/shop/panel/Area/Gauge",
            "to": "root/shop/panel",
            "name": "gauge",
            "apply": true,
            "rebind_grafts": true
        }),
    )
    .expect_err("the injected graft-record failure refuses the move");
    assert!(error.contains("injected failure"), "{error}");
    assert_unchanged(
        &root,
        &before,
        "the record, the moved source and every reference are back",
    );
    assert_eq!(directories(&root), before_dirs);
    assert!(
        root.join("native/area/gauge.cpp").is_file(),
        "the declaration is where it started"
    );
    assert!(
        !root.join("src/shop/panel/object/gauge/gauge.cpp").exists(),
        "and it did not stay at the destination"
    );
    let _ = std::fs::remove_dir_all(root);
}

/// The reproduced defect, as a pin: `.xirang/moves` occupied by a plain file makes the record
/// unpublishable, and **nothing else may change** — including the graft record, which is published
/// after the source in the same transaction.
/// 被复现的缺陷，作为钉子：用普通文件占住 `.xirang/moves` 使记录无法发布，而**其它东西一个字节都不许
/// 变**——包括在同一事务里于源码之后发布的嫁接记录。
#[test]
fn a_failed_record_publish_restores_the_source_references_and_the_graft_record() {
    let root = fixture("record-failure");
    let views = crate::build_method::face_views(&root, "shop").expect("faces");
    let gauge = views
        .iter()
        .find(|view| view.source == "native/area/gauge.cpp")
        .expect("gauge face");
    let record = root.join(".xirang/external-grafts/impl_gauge/graft.plan");
    std::fs::create_dir_all(record.parent().expect("record dir")).expect("record dir");
    std::fs::write(
        &record,
        format!(
            "version=1\ntarget={}\ntarget_path={}\ngraft=impl_gauge\nfull=false\n",
            gauge.id, gauge.path
        ),
    )
    .expect("record");
    // The obstacle: a file where the record directory has to go.
    // 障碍：在本该是记录目录的位置放一个文件。
    let obstacle = root.join(".xirang/moves");
    std::fs::write(&obstacle, "not a directory\n").expect("obstacle");
    let before = snapshot(&root);
    let before_dirs = directories(&root);

    let error = crate::mcp::apply::apply(
        &root,
        &json!({
            "action": "move",
            "node": "root/shop/panel/Area/Gauge",
            "to": "root/shop/panel",
            "name": "gauge",
            "apply": true,
            "rebind_grafts": true
        }),
    )
    .expect_err("the record cannot be published into a file");
    assert!(error.contains("moves"), "{error}");
    assert_unchanged(
        &root,
        &before,
        "the moved source, every reference and the graft record are all back",
    );
    assert_eq!(directories(&root), before_dirs);
    assert!(
        obstacle.is_file(),
        "the obstacle is still the obstacle, not a record directory"
    );
    assert!(
        root.join("native/area/gauge.cpp").is_file(),
        "the declaration never moved"
    );

    // Recovery: remove the obstacle, run the same request, and the whole thing lands.
    // 恢复：拿掉障碍，跑同一个请求，整件事落地。
    std::fs::remove_file(&obstacle).expect("remove the obstacle");
    let applied = crate::mcp::apply::apply(
        &root,
        &json!({
            "action": "move",
            "node": "root/shop/panel/Area/Gauge",
            "to": "root/shop/panel",
            "name": "gauge",
            "apply": true,
            "rebind_grafts": true
        }),
    )
    .expect("the same move lands once the record can be published");
    assert!(applied.contains("record "), "{applied}");
    assert!(root.join(".xirang/moves/1/move.plan").is_file());
    let _ = std::fs::remove_dir_all(root);
}
