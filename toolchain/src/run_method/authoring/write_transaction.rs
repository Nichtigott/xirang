//! One recoverable write set: every write of the move's own artifacts is saved **before** it is
//! written, and a later failure puts all of it back. Eviction of old migration records happens after
//! the transaction is dropped (it is a delete of a file the move did not create) and is reported, not
//! rolled back.
//! 一个可恢复的写入集合：这次搬动**自己那些产物**的每一处写入都在写之前保存，之后任何一步失败都把全部
//! 放回去。旧迁移记录的淘汰发生在事务丢弃之后（它删的是搬动没有创建的文件），被报告出来，而不回滚。
//! 一个可恢复的写入集合：每一份字节都在**写入之前**保存，之后任何一步失败都把全部放回去。
//!
//! Why this exists: the two `move` branches used to keep their own backup lists, and both had the same
//! hole — a list that is appended to *after* a write succeeds cannot restore the write that failed
//! half-way through, and record/graft publication happened outside the list entirely. This module owns
//! the one rule: **capture, then write**. A caller that writes without capturing has not used it.
//! 之所以有它：`move` 的两个分支过去各自维护备份表，而两者都有同一个洞——只在写入**成功之后**才追加的
//! 表无法恢复"写了一半失败"的那次写入，而记录/嫁接的发布干脆在表之外。本模块拥有那条唯一规则：
//! **先保存，再写入**。没有保存就写的调用方等于没用它。
//!
//! What this does **not** claim, stated once so no reader over-reads it:
//! 本模块**不**声称的东西，写在这里，免得读者读过头：
//!
//! * **Process interruption is not covered.** There is no durable journal: a `SIGKILL` between two
//!   writes leaves the first one on disk. `apply move` reports that boundary in its own reply.
//! * **Media durability is not claimed.** No `fsync` is issued; "written" means the process handed the
//!   bytes to the kernel, not that the platter has them.
//! * **Ordinary I/O failure is covered**, including a write that fails after truncating the file: that
//!   is what `WriteTransaction::write`'s capture-before-write ordering buys, and it is the only one
//!   of the three a test can inject deterministically.
//! * **进程中断不在覆盖内**：没有持久日志，两次写入之间被 `SIGKILL` 会留下第一次写入。`apply move`
//!   在自己的回复里写出这条边界。
//! * **不声称介质耐久**：没有发 `fsync`；"写过"指进程把字节交给了内核，不是盘片上已经有了。
//! * **普通 I/O 失败在覆盖内**，包括"截断之后才失败"的那次写入：这正是
//!   `WriteTransaction::write` 先保存再写的次序买来的，也是三者中唯一能被测试确定注入的。

use std::path::{Path, PathBuf};

/// A set of writes that can be undone as one unit.
/// 可作为一体撤销的一组写入。
///
/// Construct one per operation, call [`Self::capture`]/[`Self::write`]/[`Self::remove`] in the order
/// the operation runs, and on any failure call [`Self::rollback`]. On success the transaction is
/// dropped: a transaction never "commits" a second time, so there is no half-committed state to
/// describe.
/// 每次操作构造一个，按操作实际执行的顺序调用 [`Self::capture`]/[`Self::write`]/[`Self::remove`]，
/// 任何一步失败就调用 [`Self::rollback`]。成功时事务被丢弃：事务从不"第二次提交"，因此没有"提交了
/// 一半"的状态要描述。
/// One undoable step, in the order the operation performed it.
/// 一个可撤销的步骤，按操作执行它的顺序排列。
///
/// **The order is the contract.** Undo replays this log strictly in reverse, because the steps
/// depend on one another: a write that lands under a directory the operation renamed has to be
/// restored *after* that rename is rewound, or the rewind moves the edited bytes back over the
/// original ones. Two phase-separated loops (all bytes, then all renames) cannot express that, and
/// the Rust move — which writes a declaration under its old file name and then renames that file —
/// is exactly the shape that breaks.
/// **顺序就是契约。** 撤销严格按这份日志逆序重放，因为各步互相依赖：写进"操作改过名的目录"里的文件，
/// 必须在那次改名被撤回**之后**再恢复，否则撤回会把改过的字节搬回原始字节之上。分成"先全部字节、再全部
/// 改名"两段循环表达不了这件事，而 Rust 搬动——先按旧文件名写入声明、再把该文件改名——正是会坏掉的那个形状。
enum Undo {
    /// Put `original` back at `path`; `None` means the path did not exist and is removed.
    /// 把 `original` 放回 `path`；`None` 表示该路径本来不存在，撤销时删除它。
    Restore {
        path: PathBuf,
        original: Option<Vec<u8>>,
    },
    /// Remove a directory this transaction created.
    /// 删除本事务创建的一个目录。
    CreatedDir(PathBuf),
    /// Rename `to` back to `from`.
    /// 把 `to` 改回 `from`。
    ///
    Rename { from: PathBuf, to: PathBuf },
}

/// A set of writes that can be undone as one unit, in the order they happened.
/// 一组可按发生顺序整体撤销的写入。
#[derive(Default)]
pub(crate) struct WriteTransaction {
    /// The undo log, in operation order. Rollback replays it in reverse.
    /// 撤销日志，按操作顺序。回滚按逆序重放。
    undo: Vec<Undo>,
    /// Paths whose original state is already on the log: the first capture is the one to restore.
    /// 原始状态已经进日志的路径：第一次保存的那份才是要恢复的。
    captured: std::collections::BTreeSet<PathBuf>,
}

impl WriteTransaction {
    /// An empty transaction.
    /// 一个空事务。
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Save the bytes at `path` before anything changes it. Calling it twice for one path keeps the
    /// **first** read: that is the state the operation started from, and it is the one to restore.
    /// 在任何东西改动之前保存 `path` 的字节。对同一路径调用两次保留**第一次**读到的内容：那才是操作
    /// 开始时的状态，也正是要恢复的那一份。
    pub(crate) fn capture(&mut self, path: &Path) -> Result<(), String> {
        if !self.captured.insert(path.to_path_buf()) {
            return Ok(());
        }
        let bytes = match std::fs::read(path) {
            Ok(bytes) => Some(bytes),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => {
                self.captured.remove(path);
                return Err(format!(
                    "cannot read {} before changing it: {error}",
                    path.display()
                ));
            }
        };
        self.undo.push(Undo::Restore {
            path: path.to_path_buf(),
            original: bytes,
        });
        Ok(())
    }

    /// Save, then write. The saving happens first on purpose: a write that fails after truncating the
    /// file still has its original bytes recorded, which is the case a backup list appended to after
    /// success can never restore.
    /// 先保存，再写。保存有意放在前面：截断文件之后才失败的写入仍有原始字节被记录——正是"成功后才追加"
    /// 的备份表永远恢复不了的那一种。
    #[cfg(feature = "mcp")]
    pub(crate) fn write(&mut self, path: &Path, bytes: &[u8]) -> Result<(), String> {
        // The parent directories are created **before** the file is captured, so the undo log reads
        // `CreatedDir(…)` then `Restore(…)` and its reverse replay removes the file before it tries
        // to remove the directory holding it. Capturing first would log the restore before the
        // directory, and the reverse replay would then try to remove a non-empty directory, leave it
        // behind, and report a restore that did not happen.
        // 父目录在文件被保存**之前**创建，因此撤销日志是 `CreatedDir(…)` 然后 `Restore(…)`，逆序重放时
        // 先删文件、再删装着它的目录。反过来记会让逆序重放去删一个非空目录，留下它，并报出一次其实没发生的
        // 恢复。
        if let Some(directory) = path.parent() {
            self.create_dir(directory)?;
        }
        self.capture(path)?;
        if let Some(failure) = fault::take(path) {
            // Test-only: a write that lands `failure` bytes and then reports an error.
            // 仅测试用：写入 `failure` 个字节之后报错。
            use std::io::Write as _;
            let mut file = std::fs::File::create(path)
                .map_err(|error| format!("cannot write {}: {error}", path.display()))?;
            let head = &bytes[..failure.min(bytes.len())];
            file.write_all(head)
                .map_err(|error| format!("cannot write {}: {error}", path.display()))?;
            return Err(format!(
                "cannot write {}: injected failure after {failure} byte(s)",
                path.display()
            ));
        }
        std::fs::write(path, bytes)
            .map_err(|error| format!("cannot write {}: {error}", path.display()))
    }

    /// Save, then create a file that must **not** exist yet.
    /// 先保存，再创建一个**必须尚不存在**的文件。
    ///
    /// The capture is what makes a half-written new file removable: the recorded original is
    /// `None`, so a rollback deletes it. A target that already exists fails before a byte is
    /// written, and its capture is dropped again — restoring a file this operation never changed
    /// would touch somebody else's bytes, which is exactly the mistake "register the target as
    /// deletable up front" makes.
    /// 保存正是让"写了一半的新文件"可被删除的东西：记下的原始状态是 `None`，因此回滚会删掉它。已存在
    /// 的目标在一个字节写出之前就失败，并且那份保存会被撤掉——恢复一个本操作从未改过的文件，会去动别人的
    /// 字节，而那正是"提前把目标登记成待删除"所犯的错误。
    pub(crate) fn create_new(&mut self, path: &Path, bytes: &[u8]) -> Result<(), String> {
        if let Some(directory) = path.parent() {
            self.create_dir(directory)?;
        }
        self.capture(path)?;
        let opened = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path);
        let mut file = match opened {
            Ok(file) => file,
            Err(error) => {
                // Nothing changed: the path was already there, or its parent is not writable.
                // Forgetting the capture keeps rollback from rewriting somebody else's file.
                // 什么都没变：路径本来就在，或者父目录写不了。忘掉这次保存，回滚才不会去重写别人的文件。
                self.forget(path);
                return Err(format!("cannot create {}: {error}", path.display()));
            }
        };
        if let Some(failure) = fault::take(path) {
            use std::io::Write as _;
            let head = &bytes[..failure.min(bytes.len())];
            file.write_all(head)
                .map_err(|error| format!("cannot write {}: {error}", path.display()))?;
            return Err(format!(
                "cannot write {}: injected failure after {failure} byte(s)",
                path.display()
            ));
        }
        use std::io::Write as _;
        file.write_all(bytes)
            .map_err(|error| format!("cannot write {}: {error}", path.display()))
    }

    /// Drop the capture for one path: what it held is not this operation's to restore.
    /// 撤掉一个路径的保存：它当时的内容不归本操作恢复。
    fn forget(&mut self, path: &Path) {
        self.captured.remove(path);
        if let Some(index) = self.undo.iter().rposition(
            |step| matches!(step, Undo::Restore { path: captured, .. } if captured == path),
        ) {
            self.undo.remove(index);
        }
    }

    /// Save, then replace `path` through a unique sibling temporary.
    /// 先保存，再经一个唯一的同级临时文件替换 `path`。
    ///
    /// Why not capture-then-`write`: `std::fs::write` truncates the target, so a crash in the middle
    /// leaves a short file. The rename this uses publishes either the old bytes or the new ones, and
    /// the capture is what lets an ordinary I/O failure put the old ones back.
    /// 为什么不是"保存后直接 `write`"：`std::fs::write` 会截断目标，中途崩溃会留下一个短文件。这里用的
    /// 改名要么发布旧字节、要么发布新字节，而那份保存让普通 I/O 失败能把旧字节放回去。
    pub(crate) fn write_atomic(&mut self, path: &Path, bytes: &[u8]) -> Result<(), String> {
        if let Some(directory) = path.parent() {
            self.create_dir(directory)?;
        }
        self.capture(path)?;
        atomic_replace(path, bytes)?;
        if let Some(after) = fault::take(path) {
            // The seam models "the replace landed and a **later** step failed": the bytes are on disk,
            // and putting the captured ones back is this transaction's job. A seam that failed before
            // the rename would prove only that the target was never touched.
            // 这个接缝模拟的是"替换已落地、**之后**某一步失败"：字节在盘上，把保存的那一份放回去是本事务的
            // 职责。一个在改名之前失败的接缝只能证明目标从没被碰过。
            return Err(format!(
                "cannot write {}: injected failure after {after} byte(s)",
                path.display()
            ));
        }
        Ok(())
    }

    /// Save, then remove. A path that is already absent is not an error and is not recorded: rollback
    /// must not invent a file the operation never had.
    /// 先保存，再删除。本就不存在不是错误、也不记录：回滚不得造出一个操作从未有过的文件。
    pub(crate) fn remove(&mut self, path: &Path) -> Result<(), String> {
        if !path.exists() {
            return Ok(());
        }
        self.capture(path)?;
        std::fs::remove_file(path)
            .map_err(|error| format!("cannot remove {}: {error}", path.display()))
    }

    /// Create `path` and every missing parent, remembering which ones this transaction created so a
    /// rollback can take them away again. An existing directory is left alone and not recorded.
    /// 创建 `path` 与每个缺失的父级，并记住哪些是本事务创建的，使回滚能把它们拿掉。已存在的目录不动、
    /// 也不记录。
    pub(crate) fn create_dir(&mut self, path: &Path) -> Result<(), String> {
        let mut missing = Vec::new();
        let mut current = path;
        while !current.exists() {
            missing.push(current.to_path_buf());
            match current.parent() {
                Some(parent) if !parent.as_os_str().is_empty() => current = parent,
                _ => break,
            }
        }
        for directory in missing.into_iter().rev() {
            std::fs::create_dir(&directory)
                .map_err(|error| format!("cannot create {}: {error}", directory.display()))?;
            self.undo.push(Undo::CreatedDir(directory));
        }
        Ok(())
    }

    /// Rename `from` to `to` as part of this transaction, so a later failure renames it back.
    /// 把 `from` 改名为 `to` 作为本事务的一部分，因此后续失败会把它改回去。
    ///
    /// **A move never replaces what is already at the destination.** For a file that is an atomic
    /// guarantee rather than a check: `hard_link` fails when `to` exists, so two concurrent moves
    /// cannot both win. For a directory the standard library exposes no no-replace primitive, so the
    /// destination is refused when it exists and the claim is stated as exactly that — the check and
    /// the rename are one method inside the transaction, and if a destination appeared in the gap the
    /// rollback restores the source and names what it could not undo. A silent overwrite is the one
    /// outcome this method will not produce.
    /// **搬动绝不替换目的地已有的东西**。对文件这是原子保证而不是检查：`hard_link` 在 `to` 已存在时
    /// 失败，因此两个并发搬动不可能都成功。目录没有标准库的 no-replace 原语，因此目的地存在时直接拒绝，
    /// 而这句话就是它的全部承诺——检查与改名是事务里的同一个方法，若真有目的地在这一瞬出现，回滚会恢复
    /// 源并点名它没能撤销的东西。静默覆盖是这个方法唯一不会产生的结果。
    pub(crate) fn rename(&mut self, from: &Path, to: &Path) -> Result<(), String> {
        // `symlink_metadata` sees a dangling symlink too, which `Path::exists` follows away.
        if std::fs::symlink_metadata(to).is_ok() {
            return Err(format!(
                "cannot move {} to {}: the destination already exists, and a move does not overwrite \
                 what is there",
                from.display(),
                to.display()
            ));
        }
        let is_file = std::fs::symlink_metadata(from)
            .map(|metadata| metadata.file_type().is_file())
            .unwrap_or(false);
        if is_file {
            // The atomic half: `hard_link` refuses an existing destination, so nothing is replaced
            // even if another writer creates it between this line and the next.
            std::fs::hard_link(from, to).map_err(|error| {
                format!(
                    "cannot move {} to {}: {error}",
                    from.display(),
                    to.display()
                )
            })?;
            std::fs::remove_file(from).map_err(|error| {
                // The link is already at `to`; report where the bytes are so a reader is not left
                // with a moved-in name and no way to find out.
                format!(
                    "cannot remove {} after linking it to {}: {error}",
                    from.display(),
                    to.display()
                )
            })?;
        } else {
            std::fs::rename(from, to).map_err(|error| {
                format!(
                    "cannot move {} to {}: {error}",
                    from.display(),
                    to.display()
                )
            })?;
        }
        self.undo.push(Undo::Rename {
            from: from.to_path_buf(),
            to: to.to_path_buf(),
        });
        Ok(())
    }

    /// Put every captured byte back, newest capture first, then rename directories back, then
    /// remove the directories this
    /// transaction created (deepest first, and only when they are empty again).
    /// 按保存的逆序放回每一份字节，再删除本事务创建的目录（最深的先删，且仅当它们又空了）。
    ///
    /// The report names every path it could not restore and every directory it left behind, because a
    /// failure here is the one case where the reader has to go and look: "restored" is the ordinary
    /// answer, and a partial restore must not be summarised into it.
    /// 报告点名每个没恢复成功的路径与每个留下的目录——因为这里的失败正是读者必须亲自去看的那一种：
    /// "已恢复"是通常的答案，而部分恢复绝不能被概括进它。
    pub(crate) fn rollback(&self) -> Recovery {
        let mut recovery = Recovery::default();
        // Strictly reverse: a rename is rewound before the bytes written under its destination are
        // restored, and a created directory is removed only after everything inside it has left.
        // 严格逆序：改名在"写进它目的地的字节被恢复"之前撤回，而创建的目录只在里面的一切都离开之后才删除。
        for step in self.undo.iter().rev() {
            // Every failure names the **path** it is about. A rollback that says only "Is a directory"
            // leaves the reader with no way to find what is still on disk, and F2's rule is that the
            // un-restored paths themselves are the report.
            // 每个失败都点名它关于的**路径**。一次只报"是一个目录"的回滚，让读者无从找到磁盘上还留着什么，
            // 而 F2 的规则是：没恢复的路径本身就是报告。
            let result = match step {
                Undo::Rename { from, to } => std::fs::rename(to, from)
                    .map_err(|error| format!("{} -> {}: {error}", to.display(), from.display())),
                Undo::CreatedDir(directory) => match std::fs::remove_dir(directory) {
                    Ok(()) => Ok(()),
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
                    Err(error) => Err(format!("{}: {error}", directory.display())),
                },
                Undo::Restore { path, original } => match original {
                    Some(bytes) => std::fs::write(path, bytes)
                        .map_err(|error| format!("{}: {error}", path.display())),
                    None => match std::fs::remove_file(path) {
                        Ok(()) => Ok(()),
                        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
                        Err(error) => Err(format!("{}: {error}", path.display())),
                    },
                },
            };
            match result {
                Ok(()) => recovery.restored += 1,
                Err(error) => recovery.changed.push(error),
            }
        }
        recovery
    }
}

/// What a rollback actually achieved.
/// 一次回滚实际做到了什么。
#[derive(Default)]
pub(crate) struct Recovery {
    /// How many captured paths were put back.
    /// 恢复了多少个已保存的路径。
    restored: usize,
    /// Paths whose bytes could **not** be restored (or directories it could not remove), each with
    /// the error and the path.
    /// 字节**没能**恢复的路径（或没能删除的目录），各带错误与路径。
    changed: Vec<String>,
}

impl Recovery {
    /// The one line a caller appends to its own refusal.
    /// 调用方拼到自己那条拒绝后面的那一行。
    pub(crate) fn describe(&self) -> String {
        if self.changed.is_empty() {
            return format!(
                "the {} file(s) this operation changed were restored",
                self.restored
            );
        }
        let mut message = format!(
            "restoring did not fully succeed: {} file(s) were restored, and these still differ from \
             what they held before this operation — {}",
            self.restored,
            self.changed.join("; ")
        );
        message.push_str(
            "; inspect each path listed here before retrying — the operation is not atomic across a \
             crash (no journal), and this message is the record of what it changed",
        );
        message
    }
}

/// The unique-sibling replace every writer in this crate uses.
/// 本 crate 每个写入方都用的"唯一同级文件 + 改名"替换。
///
/// A fixed temporary name has to be deleted before it can be reused, and that deletion is how a
/// writer once destroyed a sibling it had not created. The process id and a counter keep two
/// writers — and the leftovers of a crashed one — apart, so the temporary is always ours to remove.
/// 固定临时名必须先删掉才能复用，而那次删除正是某个写入方过去会销毁一个并非自己创建的同级文件的途径。
/// 进程 id 与一个计数器让两个写者（以及崩溃者的残留）互不相撞，因此临时文件始终是我们自己的、可以放心删。
pub(crate) fn atomic_replace(path: &Path, bytes: &[u8]) -> Result<(), String> {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("file");
    let temporary = path.with_file_name(format!(
        ".{name}.xirang-{}-{sequence}.tmp",
        std::process::id()
    ));
    let outcome = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)
        .and_then(|mut file| {
            use std::io::Write as _;
            file.write_all(bytes)
        })
        .map_err(|error| format!("cannot write {}: {error}", temporary.display()))
        .and_then(|()| {
            std::fs::rename(&temporary, path)
                .map_err(|error| format!("cannot replace {}: {error}", path.display()))
        });
    if outcome.is_err() {
        let _ = std::fs::remove_file(&temporary);
    }
    outcome
}

/// Deterministic write failures for the fault pins.
/// 给故障钉子用的确定性写入失败。
///
/// A real partial write (disk full, quota, a device that stops answering) cannot be provoked on
/// purpose, and a test that only occupies a path with a directory proves the *pre-write* refusal, not
/// the restore of a file that was already truncated. This seam makes the second case reproducible:
/// arm a path, and the next write to it lands `after` bytes and then reports an error.
/// 真的写了一半（磁盘满、配额、设备不再响应）没法故意制造，而"用目录占住路径"只证明**写入之前**的拒绝，
/// 不证明"文件已经被截断之后"的恢复。这个接缝让第二种可复现：指定一个路径，下一次写它时先落 `after`
/// 个字节、再报错。
#[cfg(not(test))]
mod fault {
    use std::path::Path;

    /// No seam exists in a non-test build: every write goes to the disk.
    /// 非测试构建里没有接缝：每次写入都落到磁盘。
    pub(crate) fn take(_path: &Path) -> Option<usize> {
        None
    }
}

#[cfg(test)]
pub(crate) mod fault {
    use std::cell::RefCell;
    use std::path::{Path, PathBuf};

    thread_local! {
        static ARMED: RefCell<Option<(PathBuf, usize)>> = const { RefCell::new(None) };
    }

    /// Arm one partial write. Any previously armed path is replaced, so a test arms exactly once.
    /// 武装一次部分写入。之前武装的路径被替换，因此一个测试只武装一次。
    pub(crate) fn arm(path: &Path, after: usize) {
        let canonical = path.to_path_buf();
        ARMED.with(|armed| *armed.borrow_mut() = Some((canonical, after)));
    }

    /// Whether this write is the armed one, and how many bytes to land first.
    /// 这次写入是否是武装的那一次，以及先落多少个字节。
    pub(crate) fn take(path: &Path) -> Option<usize> {
        ARMED.with(|armed| {
            let mut armed = armed.borrow_mut();
            match armed.as_ref() {
                Some((armed_path, after)) if armed_path == path => {
                    let after = *after;
                    *armed = None;
                    Some(after)
                }
                _ => None,
            }
        })
    }
}

/// Write one file through the shared transaction, and report what a failed rollback could not undo.
/// 经共享事务写一个文件，并报告失败回滚没能撤销的东西。
///
/// A single-file rewrite still belongs here rather than in a bare `std::fs::write`: the guarantee a
/// reader needs is the same one the multi-file operations give — a failing write restores the bytes
/// it replaced, and a failing restore names the path instead of claiming success. Four callers used
/// to write directly (`deepen`, `cut`, `promote` and the adopted ledger), which is four chances for
/// the same rule to drift.
/// 单文件改写也属于这里，而不是一句裸 `std::fs::write`：读者要的保证与多文件操作给出的是同一个——失败的写
/// 会恢复它替换掉的字节，而恢复失败会点名路径而不是声称成功。过去有四个调用方直接写（`deepen`、`cut`、
/// `promote` 与 adopted 台账），那就是同一条规则漂移的四次机会。
///
/// Gated with its callers: all four are the bridge's, and an authoring-only build has none of them —
/// compiling it there made `write` (also bridge-only) look missing and took the whole face down.
/// 与它的调用方同一门控：四处都在桥里，而仅 authoring 的构建一处都没有——在那里编译它会让（同样只在
/// 桥里用到的）`write` 看起来不存在，并让整个面编译不过。
#[cfg(feature = "mcp")]
pub(crate) fn write_one(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let mut transaction = WriteTransaction::new();
    match transaction.write(path, bytes) {
        Ok(()) => Ok(()),
        Err(error) => {
            let recovery = transaction.rollback();
            Err(format!("{error}\n{}", recovery.describe()))
        }
    }
}

#[cfg(test)]
#[path = "write_transaction_tests.rs"]
mod write_transaction_tests;
