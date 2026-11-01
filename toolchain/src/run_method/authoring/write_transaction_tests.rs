//! The shared write transaction: capture before write, create without overwriting, and report every
//! path a rollback could not restore.
//! 共享写入事务：写前保存、创建不覆盖，并报出回滚没能恢复的每个路径。

use super::*;

fn unique(label: &str) -> PathBuf {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!(
        "xirang-txn-{label}-{}-{sequence}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).expect("fixture directory");
    path
}

/// A create that finds the path already there changes **nothing** — the file is neither truncated nor
/// deleted, because it belongs to whoever created it first.
/// 创建时发现路径已存在则**什么都不改**——文件既不被截断也不被删除，因为它属于先创建它的那一方。
///
/// This is the half that "register the target as deletable up front" gets wrong: a rollback that
/// treats a pre-existing file as its own removes somebody else's bytes.
/// 这正是"提前把目标登记成待删除"搞错的那一半：把已存在的文件当成自己的回滚，会删掉别人的字节。
#[test]
fn create_new_never_touches_a_path_that_already_exists() {
    let directory = unique("exists");
    let path = directory.join("face.rs");
    std::fs::write(&path, "// somebody else's face").expect("the neighbour");

    let mut transaction = WriteTransaction::new();
    let error = transaction
        .create_new(&path, b"// mine")
        .expect_err("an existing target is refused");
    assert!(error.contains("cannot create"), "{error}");
    let recovery = transaction.rollback();
    assert_eq!(
        std::fs::read_to_string(&path).expect("the neighbour survives"),
        "// somebody else's face",
        "neither the rollback nor the failed create touched it"
    );
    assert!(
        !recovery.describe().contains("still differ"),
        "{}",
        recovery.describe()
    );
    let _ = std::fs::remove_dir_all(&directory);
}

/// A create that fails half-way is taken back: the file exists only because this call made it, so the
/// rollback removes it (and the directory it created).
/// 写了一半失败的创建会被收回：那个文件只因为本次调用才存在，因此回滚会删掉它（以及它创建的目录）。
#[test]
fn a_half_written_create_is_removed_with_its_directory() {
    let directory = unique("partial");
    let path = directory.join("module").join("face.rs");

    let mut transaction = WriteTransaction::new();
    fault::arm(&path, 3);
    let error = transaction
        .create_new(&path, b"// a face that never finished")
        .expect_err("the armed write fails");
    assert!(error.contains("injected failure"), "{error}");
    assert!(path.exists(), "the probe really wrote those bytes first");
    let _ = transaction.rollback();
    assert!(!path.exists(), "the half-written file is gone");
    assert!(
        !directory.join("module").exists(),
        "and so is its directory"
    );
    let _ = std::fs::remove_dir_all(&directory);
}

/// An atomic replace that fails keeps the original bytes: the temporary is removed and the target is
/// restored from the capture, not re-created from a guess.
/// 失败的原子替换保留原始字节：临时文件被删除，目标从保存的那一份恢复，而不是靠猜测重建。
#[test]
fn a_failed_atomic_replace_restores_the_original_bytes() {
    let directory = unique("replace");
    let path = directory.join("face.rs");
    std::fs::write(&path, "// original").expect("fixture");

    let mut transaction = WriteTransaction::new();
    fault::arm(&path, 2);
    let error = transaction
        .write_atomic(&path, b"// the replacement")
        .expect_err("the armed replace fails");
    assert!(error.contains("injected failure"), "{error}");
    let _ = transaction.rollback();
    assert_eq!(
        std::fs::read_to_string(&path).expect("the target"),
        "// original"
    );
    let leftovers = std::fs::read_dir(&directory)
        .expect("the directory")
        .flatten()
        .filter(|entry| entry.file_name().to_string_lossy().contains("xirang-"))
        .count();
    assert_eq!(leftovers, 0, "no temporary of ours is left behind");
    let _ = std::fs::remove_dir_all(&directory);
}

/// A removal joins the same log, so a later failure puts the bytes back.
/// 删除进入同一份日志，因此后续失败会把字节放回去。
#[test]
fn a_removed_file_is_restored_by_the_rollback() {
    let directory = unique("remove");
    let path = directory.join("face.rs");
    std::fs::write(&path, "// to be removed").expect("fixture");

    let mut transaction = WriteTransaction::new();
    transaction.remove(&path).expect("the removal");
    assert!(!path.exists());
    let _ = transaction.rollback();
    assert_eq!(
        std::fs::read_to_string(&path).expect("restored"),
        "// to be removed"
    );
    let _ = std::fs::remove_dir_all(&directory);
}

/// A move refuses a destination that already exists, and neither side changes.
/// 搬动会拒绝已存在的目的地，且两边都不变。
///
/// The rule is stated as an **execution** guarantee, not a pre-flight check elsewhere: the refusal
/// lives in the transaction method every rename goes through, and a file lands through `hard_link`,
/// which the kernel refuses when the destination appears even in the gap.
/// 这条规则是**执行**保证而不是别处的预检：拒绝住在每个改名都经过的事务方法里，而文件经 `hard_link`
/// 落地——即使目的地在那一瞬出现，内核也会拒绝它。
#[test]
fn a_rename_never_replaces_an_existing_destination() {
    let directory = unique("rename-exists");
    let from = directory.join("from.rs");
    let to = directory.join("to.rs");
    std::fs::write(&from, "// the face").expect("fixture");
    std::fs::write(&to, "// somebody else's file").expect("neighbour");

    let mut transaction = WriteTransaction::new();
    let error = transaction
        .rename(&from, &to)
        .expect_err("an existing destination is refused");
    assert!(error.contains("already exists"), "{error}");
    assert_eq!(
        std::fs::read_to_string(&from).expect("the source"),
        "// the face",
        "the source is untouched"
    );
    assert_eq!(
        std::fs::read_to_string(&to).expect("the neighbour"),
        "// somebody else's file",
        "the neighbour keeps its bytes"
    );
    let recovery = transaction.rollback();
    assert!(
        !recovery.describe().contains("still differ"),
        "{}",
        recovery.describe()
    );
    let _ = std::fs::remove_dir_all(&directory);
}

/// A successful file move lands the bytes, and a later failure puts the source back.
/// 成功的文件搬动会把字节搬过去，而后续失败会把源放回来。
#[test]
fn a_file_rename_lands_and_rolls_back() {
    let directory = unique("rename-ok");
    let from = directory.join("from.rs");
    let to = directory.join("to.rs");
    std::fs::write(&from, "// the face").expect("fixture");

    let mut transaction = WriteTransaction::new();
    transaction.rename(&from, &to).expect("the move lands");
    assert!(!from.exists(), "the source left");
    assert_eq!(
        std::fs::read_to_string(&to).expect("the destination"),
        "// the face"
    );
    let _ = transaction.rollback();
    assert!(from.exists(), "the rollback moved it back");
    assert!(!to.exists(), "and the destination is empty again");
    let _ = std::fs::remove_dir_all(&directory);
}
