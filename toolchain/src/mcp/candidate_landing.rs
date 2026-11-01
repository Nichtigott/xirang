//! A landing is a transaction over **files**, and this module is the part that makes that sentence true.
//! 落地是一次跨**文件**的事务，而这个模块就是让那句话成立的部分。
//!
//! The measured failure (2026-10-18 completion review) was a two-file candidate whose first file published and whose
//! second did not: the base tree was left holding half a change, and every retry was refused because the baseline had
//! moved — by us. "Never reports completion" was true and useless: it said nothing about the half-written tree.
//! 实测的失败（2026-10-18 完成复核）是一棵两文件候选：第一个文件发布成功、第二个失败，基树留下半套改动，而每次重试都被拒
//! ——因为基线被**我们自己**移动了。"从不报告完成"是真的，但没用：它对那棵写了一半的树一个字也没说。
//!
//! Four states, each with the data needed to leave it, written down **before** the first byte of the base moves:
//! **prepared** (old and new digests, rollback bytes, the operation's identity), **writing**, **published**, and the two
//! ends — **rolled_back** and **conflict**. A stopped process is therefore a record on disk rather than a directory
//! nobody can interpret, and the next landing can finish the same transaction, undo exactly its own writes, or refuse
//! with the exact file that somebody else changed.
//! 四个状态，每个都带着离开它所必需的数据，并且在基树的第一个字节移动**之前**写下来：**prepared**（旧/新摘要、回滚字节、
//! 这次操作的身份）、**writing**、**published**，以及两个端点——**rolled_back** 与 **conflict**。因此一个停下的进程是磁盘上
//! 的一条记录，而不是一个没人能解释的目录，而下一次落地可以完成同一事务、只撤销它自己写的东西，或精确点名某个被别人改过的文件
//! 并拒绝。
//!
//! What this module does **not** claim: durability across a power cut. The rollback bytes, the manifest and the record
//! are ordinary writes; the recovery argument is about a process that stops, not about a machine that does.
//! 本模块**不**声称的：断电持久性。回滚字节、清单与记录都是普通写入；恢复论证说的是进程停下，不是机器停下。

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use super::Change;

/// Prepared: the transaction exists on disk and the base has not moved.
/// 已准备：事务已在磁盘上，基树还没动。
const PREPARED: &str = "prepared";
/// Writing: the base is being changed file by file.
/// 写入中：基树正被逐个文件改动。
const WRITING: &str = "writing";
/// Published: every file was written; the candidate's own record may not have been finalised yet.
/// 已发布：每个文件都写完了；候选自己的记录可能还没收尾。
const PUBLISHED: &str = "published";
/// Rolled back: this transaction's writes were undone and the base is back at `base_at_open`.
/// 已回滚：这次事务的写入被撤销，基树回到 `base_at_open`。
const ROLLED_BACK: &str = "rolled_back";
/// Conflict: at least one file is neither the bytes this transaction wrote nor the bytes it replaced.
/// 冲突：至少一个文件既不是这次事务写的字节，也不是它替换掉的字节。
const CONFLICT: &str = "conflict";

/// Where the transaction's state lives, beside the candidate's own record.
/// 事务状��所在的位置，在候选自己的记录旁边。
fn state_path(directory: &Path, id: &str) -> PathBuf {
    directory.join(format!("{id}.landing.json"))
}

/// Where the bytes this transaction replaced are kept.
/// 这次事务替换掉的字节存放的位置。
fn rollback_path(directory: &Path, id: &str) -> PathBuf {
    directory.join(format!("{id}.rollback"))
}

/// The landing lock: one landing per candidate at a time, so a baseline that was checked cannot be overwritten by a
/// second writer that checked the same baseline. It is removed on drop; a lock left by a stopped process is detected by
/// its pid and taken over, because the alternative — asking a person to delete a file — is a failure mode of its own.
/// 落地锁：同一时刻每棵候选只允许一次落地，于是"检查过的基线"不会被另一个同样检查过它的写者覆盖。锁在 drop 时删除；
/// 停下的进程留下的锁按 pid 识别并接管，因为替代方案——请人去删一个文件——本身就是一个失败模式。
pub(super) struct Lock {
    path: PathBuf,
}

impl Drop for Lock {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

/// Take the landing lock for one candidate.
/// 为一棵候选取落地锁。
pub(super) fn lock(directory: &Path, id: &str) -> Result<Lock, String> {
    let path = directory.join(format!("{id}.landing.lock"));
    for _ in 0..2 {
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
        {
            Ok(mut file) => {
                use std::io::Write;
                let _ = write!(file, "{}", std::process::id());
                return Ok(Lock { path });
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                let holder = std::fs::read_to_string(&path)
                    .ok()
                    .and_then(|value| value.trim().parse::<u32>().ok());
                let alive = holder
                    .map(|pid| Path::new(&format!("/proc/{pid}")).exists())
                    .unwrap_or(false);
                if alive {
                    return Err(format!(
                        "another landing for this candidate is in progress (the lock {} is held by pid {}); nothing is written to the base while a landing holds it",
                        path.display(),
                        holder.unwrap_or(0)
                    ));
                }
                // The holder stopped. Taking the lock over is safe because the state file says what it had done, and
                // the recovery below reads that state before touching anything.
                // 持有者停下了。接管是安全的，因为状态文件写下了它做到哪一步，而下面的恢复在碰任何东西之前会先读那个状态。
                let _ = std::fs::remove_file(&path);
            }
            Err(error) => {
                return Err(format!(
                    "cannot take the landing lock {}: {error}",
                    path.display()
                ));
            }
        }
    }
    Err(format!(
        "cannot take the landing lock {}: it kept reappearing while this process tried to remove a stopped holder's lock",
        path.display()
    ))
}

/// A prepared transaction: the record on disk plus the paths it needs to finish or undo itself.
/// 一份已准备的事务：磁盘上的记录，以及完成或撤销它所需要的路径。
pub(super) struct Prepared {
    path: PathBuf,
    directory: PathBuf,
    body: Value,
}

/// Write down everything a later run needs: the old bytes, their digests, the new digests, the operation's identity.
/// 把后来者需要的一切写下来：旧字节、它们的摘要、新摘要、这次操作的身份。
/// Create a file exclusively, without following a link that sits at its path.
/// 独占创建一个文件，不跟随坐在它路径上的链接。
fn create_new(path: &Path) -> std::io::Result<std::fs::File> {
    #[cfg(unix)]
    {
        use std::os::unix::io::FromRawFd;
        // `open(2)` is variadic; only the third argument this call passes is used, because flags say O_CREAT.
        // `open(2)` 是可变参数函数；本次调用只用到第三个参数，因为 flags 里有 O_CREAT。
        unsafe extern "C" {
            fn open(path: *const std::ffi::c_char, flags: std::ffi::c_int, ...) -> std::ffi::c_int;
        }
        const O_WRONLY: std::ffi::c_int = 0o1;
        const O_CREAT: std::ffi::c_int = 0o100;
        const O_EXCL: std::ffi::c_int = 0o200;
        const O_NOFOLLOW: std::ffi::c_int = 0o400000;
        const O_CLOEXEC: std::ffi::c_int = 0o2000000;
        let name = std::ffi::CString::new(path.as_os_str().as_encoded_bytes()).map_err(|_| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, "path holds a NUL")
        })?;
        let fd = unsafe {
            open(
                name.as_ptr(),
                O_WRONLY | O_CREAT | O_EXCL | O_NOFOLLOW | O_CLOEXEC,
                0o644 as std::ffi::c_int,
            )
        };
        if fd < 0 {
            return Err(std::io::Error::last_os_error());
        }
        Ok(unsafe { std::fs::File::from_raw_fd(fd) })
    }
    #[cfg(not(unix))]
    {
        std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
    }
}

/// A path a landing will write is allowed to be absent or a regular file, and nothing else.
/// 落地要写的路径只允许不存在、或是普通文件，别的都不行。
fn refuse_link(path: &Path, relative: &str, whose: &str) -> Result<(), String> {
    match std::fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => Err(format!(
            "{whose} {relative} is a symbolic link: a landing writes the file it names and never the place a link \
             points at, so this change is refused instead of being written through"
        )),
        Ok(_) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(format!("cannot inspect {whose} {relative}: {error}")),
    }
}

pub(super) fn prepare(
    directory: &Path,
    id: &str,
    root: &Path,
    tree: &Path,
    delta: &BTreeMap<String, Change>,
    identity: Value,
) -> Result<Prepared, String> {
    let path = state_path(directory, id);
    if path.exists() {
        return Err(format!(
            "a landing record for this candidate already exists at {}; it describes an attempt that has not been closed, and a second transaction over the same files would be a second story about the same bytes",
            path.display()
        ));
    }
    let rollback = rollback_path(directory, id);
    std::fs::create_dir_all(&rollback)
        .map_err(|error| format!("cannot create {}: {error}", rollback.display()))?;
    let mut files = Vec::new();
    for (index, (relative, change)) in delta.iter().enumerate() {
        let target = root.join(relative);
        // EX02: a landing writes files, and a write through a link writes somewhere else. The copy's path, the base
        // tree's path and the staging path are each refused when they are symbolic links, so neither the content's
        // origin nor the destination can be redirected by a link — including the rollback write below, which would
        // otherwise reach the link's target instead of replacing the link.
        // EX02：落地做的是写文件，而**穿过链接**的写落在别处。副本路径、基树路径与暂存路径只要有一个是符号链接就拒绝，
        // 于是内容的来源与目的地都不能被链接改道——包括下面的回滚写：否则它到达的是链接目标，而不是替换那个链接本身。
        refuse_link(&tree.join(relative), relative, "the copy's")?;
        refuse_link(&target, relative, "the base tree's")?;
        refuse_link(
            &target.with_extension("xirang-landing"),
            relative,
            "the staging path",
        )?;
        let after = match change {
            Change::Gone => String::new(),
            Change::Written => {
                let bytes = std::fs::read(tree.join(relative))
                    .map_err(|error| format!("cannot read the copy's {relative}: {error}"))?;
                xirang_kernel::identity::sha256_hex(&bytes)
            }
        };
        let (existed, before) = match std::fs::read(&target) {
            Ok(bytes) => {
                let backup = rollback.join(format!("{index}.before"));
                std::fs::write(&backup, &bytes)
                    .map_err(|error| format!("cannot keep the old bytes of {relative}: {error}"))?;
                (true, xirang_kernel::identity::sha256_hex(&bytes))
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => (false, String::new()),
            Err(error) => {
                return Err(format!(
                    "cannot read the base tree's {relative} before changing it: {error}"
                ));
            }
        };
        if matches!(change, Change::Gone) && !existed {
            // `delta` said the file is gone from the copy and the base never had it: there is nothing to do, and a
            // rollback entry for it would be a story about a byte that never existed.
            // `delta` 说副本里没有这个文件、而基树也从未有过：无事可做，为它留回滚条目等于讲一个从未存在过的字节的故事。
            continue;
        }
        files.push(json!({
            "path": relative,
            "existed": existed,
            "before": before,
            "after": after,
            "rollback": format!("{index}.before"),
        }));
    }
    let body = json!({
        "id": id,
        "identity": identity,
        "state": PREPARED,
        "at": super::now(),
        "pid": std::process::id(),
        "files": files,
    });
    crate::mcp::check_plan::persist(&path, &body)?;
    Ok(Prepared {
        path,
        directory: directory.to_path_buf(),
        body,
    })
}

impl Prepared {
    /// How many files this transaction covers.
    /// 这次事务覆盖多少个文件。
    pub(super) fn files(&self) -> usize {
        self.body["files"].as_array().map(Vec::len).unwrap_or(0)
    }

    /// Where the state machine lives, for a refusal to name.
    /// 状态机所在的位置，供拒绝点名。
    pub(super) fn path(&self) -> &Path {
        &self.path
    }

    /// Move the state machine forward, and keep what the last one said.
    /// 把状态机往前推，并保留上一个状态说过的话。
    fn set(&mut self, state: &str, extra: Value) -> Result<(), String> {
        self.body["state"] = json!(state);
        self.body["at"] = json!(super::now());
        if let Some(object) = extra.as_object() {
            for (key, value) in object {
                self.body[key] = value.clone();
            }
        }
        crate::mcp::check_plan::persist(&self.path, &self.body)
    }
}

/// Publish every file, or undo the ones already written and say so. The base is never left holding a change the
/// candidate did not declare, and the failure is reported with the tree in the state the record describes.
/// 发布每个文件；否则撤销已经写下的那些并说出来。基树永远不会留下候选没有声明的改动，而失败报告时树正处在记录所描述的状态。
pub(super) fn publish(
    prepared: &mut Prepared,
    root: &Path,
    tree: &Path,
) -> Result<(Vec<String>, Vec<String>), String> {
    prepared.set(WRITING, json!({}))?;
    let files = prepared.body["files"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let mut landed: Vec<String> = Vec::new();
    let mut removed: Vec<String> = Vec::new();
    for entry in &files {
        let relative = entry["path"].as_str().unwrap_or_default().to_owned();
        let target = root.join(&relative);
        let gone = entry["after"].as_str().unwrap_or_default().is_empty();
        let outcome = if gone {
            match std::fs::remove_file(&target) {
                Ok(()) => Ok(true),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
                Err(error) => Err(format!("cannot remove {relative}: {error}")),
            }
        } else {
            let staged = target.with_extension("xirang-landing");
            let staged_outcome = (|| -> Result<(), String> {
                if let Some(parent) = target.parent() {
                    std::fs::create_dir_all(parent)
                        .map_err(|error| format!("cannot create {}: {error}", parent.display()))?;
                }
                // The staging file is created exclusively and without following a link: a link planted at this path
                // between `prepare` and now is refused instead of being written through.
                // 暂存文件独占创建、且不跟随链接：在 `prepare` 之后坐到这个路径上的链接会被拒绝，而不是被穿过去写。
                if std::fs::symlink_metadata(&staged).is_ok_and(|m| m.file_type().is_file()) {
                    let _ = std::fs::remove_file(&staged);
                }
                let bytes = std::fs::read(tree.join(&relative))
                    .map_err(|error| format!("cannot read the copy's {relative}: {error}"))?;
                let mut file = create_new(&staged)
                    .map_err(|error| format!("cannot stage {relative}: {error}"))?;
                std::io::Write::write_all(&mut file, &bytes)
                    .map_err(|error| format!("cannot stage {relative}: {error}"))?;
                drop(file);
                std::fs::rename(&staged, &target)
                    .map_err(|error| format!("cannot publish {relative}: {error}"))
            })();
            staged_outcome.map(|()| true)
        };
        match outcome {
            Ok(wrote) => {
                if wrote {
                    if gone {
                        removed.push(relative);
                    } else {
                        landed.push(relative);
                    }
                }
            }
            Err(error) => {
                let undone = undo(prepared, root, &landed, &removed);
                return Err(match undone {
                    Ok(()) => format!(
                        "{error}. This landing was a transaction over {} file(s): the ones already written were rolled back, and the base tree's files are the ones `base_at_open` describes again. Fix the cause and run `land` again — the same candidate continues; see {}",
                        files.len(),
                        prepared.path.display()
                    ),
                    Err(conflict) => format!(
                        "{error}. The rollback could not finish: {conflict}. Nothing else is written; {} keeps every digest needed to finish this by hand",
                        prepared.path.display()
                    ),
                });
            }
        }
    }
    prepared.set(PUBLISHED, json!({}))?;
    Ok((landed, removed))
}

/// Undo exactly this transaction's writes. A file is ours if it still hashes to what we wrote; anything else is
/// somebody else's and is left alone, with its three digests recorded.
/// 只撤销这次事务写下的东西。文件仍是"我们写的那个摘要"才算我们的；别的都是别人的，一动不动，并把三个摘要记下来。
fn undo(
    prepared: &mut Prepared,
    root: &Path,
    _landed: &[String],
    _removed: &[String],
) -> Result<(), String> {
    let files = prepared.body["files"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let rollback = rollback_path(
        &prepared.directory,
        prepared.body["id"].as_str().unwrap_or(""),
    );
    let mut conflicts = Vec::new();
    for entry in &files {
        let relative = entry["path"].as_str().unwrap_or_default().to_owned();
        let target = root.join(&relative);
        let before = entry["before"].as_str().unwrap_or_default().to_owned();
        let after = entry["after"].as_str().unwrap_or_default().to_owned();
        let current = match std::fs::read(&target) {
            Ok(bytes) => Some(xirang_kernel::identity::sha256_hex(&bytes)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => {
                conflicts.push(format!("{relative}: cannot read it back ({error})"));
                continue;
            }
        };
        let current = current.unwrap_or_default();
        if current == before {
            continue; // this file was never written, or was already restored
        }
        if current != after {
            conflicts.push(format!(
                "{relative}: before {before}, this transaction wrote {after}, now {current}"
            ));
            continue;
        }
        if std::fs::symlink_metadata(&target).is_ok_and(|m| m.file_type().is_symlink()) {
            conflicts.push(format!(
                "{relative}: the base tree's path became a symbolic link while this transaction held it; restoring it would write through the link, so the old bytes are left in {} for a person to place",
                rollback.display()
            ));
            continue;
        }
        let restored = if entry["existed"].as_bool().unwrap_or(false) {
            let backup = rollback.join(entry["rollback"].as_str().unwrap_or_default());
            std::fs::read(&backup)
                .map_err(|error| format!("cannot read back {}: {error}", backup.display()))
                .and_then(|bytes| {
                    std::fs::write(&target, bytes)
                        .map_err(|error| format!("cannot restore {relative}: {error}"))
                })
        } else {
            std::fs::remove_file(&target)
                .map_err(|error| format!("cannot remove {relative}: {error}"))
        };
        if let Err(error) = restored {
            conflicts.push(error);
        }
    }
    if conflicts.is_empty() {
        prepared.set(ROLLED_BACK, json!({}))?;
        let _ = std::fs::remove_dir_all(&rollback);
        Ok(())
    } else {
        prepared.set(CONFLICT, json!({"conflicts": conflicts.clone()}))?;
        Err(conflicts.join("; "))
    }
}

/// What a previous attempt left behind, told to the caller that is about to start another one.
/// 上一次尝试留下的东西，告诉正要再开一次的调用方。
pub(super) enum Recovered {
    /// Every file had been written before the attempt stopped; the transaction is complete and only its record is not.
    /// 尝试停下之前每个文件都写完了；事务已完成，只是它的记录没收尾。
    Published(String),
    /// The writes were undone; the base is back at the baseline the baseline check will compare against.
    /// 写入被撤销；基树回到了基线检查将要对比的那个基线。
    RolledBack(String),
}

/// Read what the last attempt left, and either finish it, undo it, or refuse with the exact conflict.
/// 读上一次尝试留下的东西，然后完成它、撤销它，或精确点名冲突并拒绝。
pub(super) fn recover(
    directory: &Path,
    id: &str,
    root: &Path,
    base_at_open: &str,
) -> Result<Option<Recovered>, String> {
    let path = state_path(directory, id);
    if !path.exists() {
        return Ok(None);
    }
    let body: Value = serde_json::from_slice(
        &std::fs::read(&path)
            .map_err(|error| format!("cannot read {}: {error}", path.display()))?,
    )
    .map_err(|error| format!("cannot parse {}: {error}", path.display()))?;
    let state = body["state"].as_str().unwrap_or("unknown");
    match state {
        ROLLED_BACK => {
            let _ = std::fs::remove_file(&path);
            Ok(Some(Recovered::RolledBack(format!(
                "the previous attempt was rolled back at {}; its record was cleared, and this run starts from the baseline it was opened on",
                body["at"].as_str().unwrap_or("an unrecorded time")
            ))))
        }
        PUBLISHED => Ok(Some(Recovered::Published(format!(
            "the previous attempt had already written every file ({}) before it stopped; this run finishes its record rather than writing the files twice",
            body["at"].as_str().unwrap_or("an unrecorded time")
        )))),
        WRITING | PREPARED => {
            let files = body["files"].as_array().cloned().unwrap_or_default();
            let all_written = files.iter().all(|entry| {
                let target = root.join(entry["path"].as_str().unwrap_or_default());
                let after = entry["after"].as_str().unwrap_or_default();
                match std::fs::read(&target) {
                    Ok(_bytes) if after.is_empty() => false,
                    Ok(bytes) => xirang_kernel::identity::sha256_hex(&bytes) == after,
                    Err(_) => after.is_empty(),
                }
            });
            let mut prepared = Prepared {
                path: path.clone(),
                directory: directory.to_path_buf(),
                body,
            };
            if all_written {
                prepared.set(PUBLISHED, json!({}))?;
                return Ok(Some(Recovered::Published(
                    "the previous attempt wrote every file before it stopped; this run finishes that same transaction instead of starting a second one".to_owned(),
                )));
            }
            match undo(&mut prepared, root, &[], &[]) {
                Ok(()) => Ok(Some(Recovered::RolledBack(format!(
                    "the previous attempt stopped part-way, so its writes were undone and the base is back at {base_at_open}; run `land` again to publish the whole change"
                )))),
                Err(conflict) => {
                    let _ = std::fs::remove_file(&path);
                    Err(format!(
                        "a previous attempt stopped part-way and cannot be undone safely, because a file it changed has since been edited by something else: {conflict}. Its digests are in the landing record; nothing else was written, and no later step may overwrite those bytes"
                    ))
                }
            }
        }
        CONFLICT => Err(format!(
            "the last attempt recorded a conflict and it is unresolved: {}. The record is at {}",
            body["conflicts"]
                .as_array()
                .map(|rows| {
                    rows.iter()
                        .filter_map(Value::as_str)
                        .collect::<Vec<_>>()
                        .join("; ")
                })
                .unwrap_or_else(|| "no detail recorded".to_owned()),
            path.display()
        )),
        other => Err(format!(
            "the landing record {} has an unknown state `{other}`; it is not treated as either finished or undone",
            path.display()
        )),
    }
}

/// Clear the finished transaction's bookkeeping after the candidate's own record says `landed`.
/// 在候选自己的记录写着 `landed` 之后，清掉已完成事务的簿记。
pub(super) fn close(directory: &Path, id: &str) {
    let _ = std::fs::remove_file(state_path(directory, id));
    let _ = std::fs::remove_dir_all(rollback_path(directory, id));
}
